use ts_dream::encoding::viscii_encode;
use ts_dream::server::dispatcher::OpcodeCtx;
use ts_dream::server::handlers::character::{handle_character, is_valid_char_name, parse_create};
use ts_dream::server::handlers::login::{handle_enter_game, handle_login};
use ts_dream::server::session::Conn;
use ts_dream::server::spawn;

#[test]
fn test_is_valid_char_name() {
    // Valid names in VISCII
    assert!(is_valid_char_name(&viscii_encode("TrươngPhi")));
    assert!(is_valid_char_name(&viscii_encode("QuanVũ")));
    assert!(is_valid_char_name(&viscii_encode("GiaCátLượng")));
    assert!(is_valid_char_name(&viscii_encode("ỶLan")));
    assert!(is_valid_char_name(&viscii_encode("TàoTháo")));
    assert!(is_valid_char_name(&viscii_encode("NguyễnVănÝ")));

    // Empty name
    assert!(!is_valid_char_name(b""));

    // Exceeds 16 bytes
    let long_name = [b'A'; 17];
    assert!(!is_valid_char_name(&long_name));

    // Valid 16-byte name
    let max_len_name = [b'A'; 16];
    assert!(is_valid_char_name(&max_len_name));

    // Leading or trailing space
    assert!(!is_valid_char_name(b" Hero"));
    assert!(!is_valid_char_name(b"Hero "));

    // Invalid control characters (e.g. 0x00, 0x01, 0x03, 0x1B)
    assert!(!is_valid_char_name(&[b'A', 0x01, b'B']));
    assert!(!is_valid_char_name(&[b'A', 0x1B, b'B']));
    assert!(!is_valid_char_name(&[b'A', 0x7F, b'B'])); // DEL
}

#[test]
fn test_parse_create_character_payload() {
    // Layout:
    // [0] sex
    // [1] style
    // [2] hair
    // [3] face
    // [4..12] color (8 bytes)
    // [12] thuoctinh
    // [13..19] int, atk, def, hpx, spx, agi
    // [19] pass1_len
    // [20..20+len] pass1
    // [20+len] pass2_len
    // [20+len+1..] pass2
    let mut payload = vec![
        1,  // sex: nam
        2,  // style
        5,  // hair
        0,  // face
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // color 8B
        3,  // thuoctinh: hỏa
        10, 15, 12, 8, 7, 14, // int, atk, def, hpx, spx, agi
        4,  // pass1_len = 4
        b'1', b'2', b'3', b'4', // pass1
        4,  // pass2_len = 4
        b'5', b'6', b'7', b'8', // pass2
    ];

    let parsed = parse_create(&payload).expect("Should parse valid payload");
    assert_eq!(parsed.sex, 1);
    assert_eq!(parsed.hair, 5);
    assert_eq!(parsed.thuoctinh, 3);
    assert_eq!(parsed.int1, 10);
    assert_eq!(parsed.atk, 15);
    assert_eq!(parsed.def, 12);
    assert_eq!(parsed.hpx, 8);
    assert_eq!(parsed.spx, 7);
    assert_eq!(parsed.agi, 14);
    assert_eq!(parsed.pass1, b"1234");
    assert_eq!(parsed.pass2, b"5678");

    // Short payload should return None
    payload.truncate(15);
    assert!(parse_create(&payload).is_none());
}

#[tokio::test]
async fn test_character_creation_and_enter_game_flow() {
    let mut conn = Conn::default();
    conn.session.id = 1001;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ts_dream::server::dispatcher::ServerEnv::none();

    // 1. Initial Login with no character in session -> should prompt LOGIN_CREATE_CHAR
    // Frame: [id: 4B LE] [prefix: 2B "VN"] [ver: 2B 0xBC 0x00] [pw]
    let mut login_payload = vec![0xE9, 0x03, 0x00, 0x00, b'V', b'N', 0xBC, 0x00];
    login_payload.extend_from_slice(b"secret123");

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x01,
        sub: 9, // lenPw = 9
        payload: &login_payload,
    };

    handle_login(&mut ctx).await;

    // Must emit LOGIN_CREATE_CHAR
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::LOGIN_CREATE_CHAR),
        "Expected LOGIN_CREATE_CHAR packet"
    );
    // CRITICAL: session.authed MUST be set to true so subsequent enter-game works!
    assert!(
        conn.session.authed,
        "Session must be authed after successful password check"
    );

    // 2. Check duplicate name: candidate = b"EXISTS"
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 2, // Sub 2 = Name check
        payload: b"EXISTS",
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_NAME_DUPLICATE),
        "Expected CHAR_NAME_DUPLICATE (09 03 01)"
    );

    // 3. Check invalid name: candidate with control character
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 2,
        payload: &[b'A', 0x01, b'B'],
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_NAME_INVALID),
        "Expected CHAR_NAME_INVALID (09 03 02)"
    );

    // 4. Check available name: candidate = "TrươngPhi"
    let candidate = viscii_encode("TrươngPhi");
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 2,
        payload: &candidate,
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_NAME_AVAILABLE),
        "Expected CHAR_NAME_AVAILABLE (09 03 00)"
    );
    assert_eq!(conn.session.pending_new_char_name, candidate);

    // 5. Create character: Sub 1
    let create_payload = vec![
        1,  // sex
        1,  // style
        3,  // hair
        0,  // face
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // color
        4,  // element: Phong
        5, 12, 10, 8, 5, 15, // stats
        0,  // pass1_len
        0,  // pass2_len
    ];
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 1, // Sub 1 = Create
        payload: &create_payload,
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_CREATE_SUCCESS),
        "Expected CHAR_CREATE_SUCCESS (09 01)"
    );
    assert_eq!(conn.session.name, candidate);
    assert!(conn.session.authed, "authed must be preserved");

    // 6. Enter game confirmation: Client sends Opcode 0x03 Sub 0x01
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x03,
        sub: 1,
        payload: &[],
    };
    handle_enter_game(&mut ctx).await;

    // Must emit full Logined1 sequence (21 frames for petless session)
    assert!(conn.session.logined, "Session must be marked as logined");
    assert_eq!(
        out.outgoing.len(),
        21,
        "Logined1 sequence without pets must contain exactly 21 frames"
    );

    // First frame must be login_start
    assert_eq!(out.outgoing[0].frame, "F44402001408");
    assert_eq!(out.outgoing[1].frame, "F4440300142100");
    // Third frame must be player_appear (03)
    assert!(out.outgoing[2].frame.starts_with("F444"));
    assert_eq!(&out.outgoing[2].frame[8..10], "03");
}

#[test]
fn test_parse_create_omitted_pass2_and_bounds() {
    // 20-byte payload: pass2 omitted entirely
    let payload_20b = vec![
        1,  // sex: nam
        2,  // style
        5,  // hair
        0,  // face
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // color 8B
        3,  // thuoctinh: hỏa (3)
        10, 15, 12, 8, 7, 14, // int, atk, def, hpx, spx, agi
        0,  // pass1_len = 0 (empty pass1)
    ];
    let parsed = parse_create(&payload_20b).expect("20-byte payload without pass2 must be parsed");
    assert_eq!(parsed.sex, 1);
    assert_eq!(parsed.thuoctinh, 3);
    assert!(parsed.pass1.is_empty());
    assert!(parsed.pass2.is_empty());

    // Out-of-bound element (e.g. 99) should be safely defaulted to 1 (Địa)
    let mut payload_oob = payload_20b.clone();
    payload_oob[12] = 99;
    let parsed_oob = parse_create(&payload_oob).expect("Should parse with defaulted element");
    assert_eq!(parsed_oob.thuoctinh, 1);

    // Out-of-bound sex (e.g. 5) should default to 0
    let mut payload_sex = payload_20b.clone();
    payload_sex[0] = 5;
    let parsed_sex = parse_create(&payload_sex).expect("Should parse with normalized sex");
    assert_eq!(parsed_sex.sex, 0);
}

#[tokio::test]
async fn test_character_creation_edge_cases() {
    let mut conn = Conn::default();
    conn.session.id = 2001;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ts_dream::server::dispatcher::ServerEnv::none();

    // 1. Set a candidate name, then check an invalid name -> must clear pending candidate!
    conn.session.pending_new_char_name = b"ValidCandidate".to_vec();
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 2, // Sub 2 = Name check
        payload: b"Invalid Name \x01", // Contains control char
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_NAME_INVALID),
        "Expected CHAR_NAME_INVALID"
    );
    assert!(
        conn.session.pending_new_char_name.is_empty(),
        "Stale candidate name must be cleared upon invalid name check"
    );

    // 2. Sub 1 create character with no candidate name set -> must return CHAR_NAME_INVALID
    let create_payload = vec![
        1, 1, 3, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        4, 5, 12, 10, 8, 5, 15,
        0, 0,
    ];
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 1,
        payload: &create_payload,
    };
    handle_character(&mut ctx).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::CHAR_NAME_INVALID),
        "Character creation with empty name must be rejected with CHAR_NAME_INVALID"
    );

    // 3. Sub 1 with empty payload (Bear C# 09 01 confirm login) delegates to enter game
    conn.session.authed = true;
    conn.session.name = b"ExistingHero".to_vec();
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x09,
        sub: 1,
        payload: &[],
    };
    handle_character(&mut ctx).await;
    assert!(conn.session.logined, "Empty 09 01 must delegate to enter game");
    assert_eq!(out.outgoing.len(), 21, "Must emit full Logined1 sequence");
}
