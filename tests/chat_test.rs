//! Chat handler tests (Opcode 0x02) — ticket T4.3.
//!
//! Drives `handle_chat` directly via `dispatcher::test_ctx`
//! (`ServerEnv::none()`, no DB, fake sessions only).

use ts_dream::data::loader::GameData;
use ts_dream::protocol::encoder;
use ts_dream::server::dispatcher::{test_ctx, HandleOutcome};
use ts_dream::server::handlers::chat::handle_chat;
use ts_dream::server::session::{lock_online_sessions, Conn};
use ts_dream::server::spawn;

async fn run_chat(
    conn: &mut Conn,
    data: &GameData,
    service: &ts_dream::battle::service::BattleService,
    out: &mut HandleOutcome,
    sub: u8,
    payload: &[u8],
) {
    let mut ctx = test_ctx(conn, data, service, out, sub, payload);
    handle_chat(&mut ctx).await;
}

fn sender_conn(id: u32) -> Conn {
    let mut c = Conn::default();
    c.session.id = id;
    c
}

// --- sub 2 map chat: outgoing holds 0202 + sender id (LE32) ---

#[tokio::test]
async fn test_sub2_map_chat_echo() {
    let mut conn = sender_conn(5001);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let payload = b"hello";
    run_chat(&mut conn, &data, &service, &mut out, 2, payload).await;
    assert_eq!(out.outgoing.len(), 1);
    let expected = spawn::chat_frame(2, 5001, payload);
    assert_eq!(out.outgoing[0].frame, expected);
    assert!(out.outgoing[0].frame.contains("0202"));
    assert!(out.outgoing[0].frame.contains(&encoder::le32(5001)));
}

// --- slash "/where" replies 020B with "MapID" ---

#[tokio::test]
async fn test_sub2_slash_where() {
    let mut conn = sender_conn(5002);
    conn.session.map_id = 10817;
    conn.session.map_x = 100;
    conn.session.map_y = 200;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 2, b"/where").await;
    assert_eq!(out.outgoing.len(), 1);
    let expected = spawn::sys_msg_frame("MapID:10817 X:100 Y:200");
    assert_eq!(out.outgoing[0].frame, expected);
    assert!(out.outgoing[0].frame.contains("020B"));
}

// --- slash "/offq" replies F44402001408 ---

#[tokio::test]
async fn test_sub2_slash_offq() {
    let mut conn = sender_conn(5003);
    conn.session.idtalking = 7;
    conn.session.select_menu = 3;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 2, b"/offq").await;
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(out.outgoing[0].frame, "F44402001408");
    assert_eq!(conn.session.idtalking, 0);
}

// --- unknown slash "/xyz123" produces no outgoing ---

#[tokio::test]
async fn test_sub2_slash_unknown_silent() {
    let mut conn = sender_conn(5004);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 2, b"/xyz123").await;
    assert!(out.outgoing.is_empty());
}

// --- sub 3 whisper carries sender id; offline target gets 020B notice ---

#[tokio::test]
async fn test_sub3_whisper_sender_id() {
    let sender_id = 5011u32;
    let target_id = 5012u32;
    let mut conn = sender_conn(sender_id);
    // Target must be online so the whisper is delivered (sender copy echoed).
    lock_online_sessions().insert(target_id, sender_conn(target_id).session);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let mut payload = target_id.to_le_bytes().to_vec();
    let chat_raw = b"hi";
    payload.extend_from_slice(chat_raw);
    run_chat(&mut conn, &data, &service, &mut out, 3, &payload).await;
    lock_online_sessions().remove(&target_id);
    assert_eq!(out.outgoing.len(), 1);
    let expected = spawn::chat_frame(3, sender_id, chat_raw);
    assert_eq!(out.outgoing[0].frame, expected);
    assert!(out.outgoing[0].frame.contains("0203"));
    assert!(out.outgoing[0].frame.contains(&encoder::le32(sender_id)));
    assert!(!out.outgoing[0].frame.contains(&encoder::le32(target_id)));
}

#[tokio::test]
async fn test_sub3_whisper_offline_notice() {
    let sender_id = 5013u32;
    let target_id = 5014u32; // never inserted -> offline
    lock_online_sessions().remove(&target_id);
    let mut conn = sender_conn(sender_id);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let mut payload = target_id.to_le_bytes().to_vec();
    payload.extend_from_slice(b"hi");
    run_chat(&mut conn, &data, &service, &mut out, 3, &payload).await;
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(
        out.outgoing[0].frame,
        spawn::sys_msg_frame("Nguoi choi khong online.")
    );
    assert!(out.outgoing[0].frame.contains("020B"));
}

// --- sub 4 gate: non-GM silent drop, GM broadcasts 0204 ---

#[tokio::test]
async fn test_sub4_non_gm_dropped() {
    let mut conn = sender_conn(5021);
    conn.session.gm_level = 0;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 4, b"gm hello").await;
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn test_sub4_gm_broadcast() {
    let mut conn = sender_conn(5022);
    conn.session.gm_level = 10;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let payload = b"gm hello";
    run_chat(&mut conn, &data, &service, &mut out, 4, payload).await;
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(out.outgoing[0].frame, spawn::chat_frame(4, 5022, payload));
    assert!(out.outgoing[0].frame.contains("0204"));
}

// --- sub 1 C->S disabled: no outgoing ---

#[tokio::test]
async fn test_sub1_disabled() {
    let mut conn = sender_conn(5031);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 1, b"world hi").await;
    assert!(out.outgoing.is_empty());
}

// --- sub 5 solo (no party) gets 020B error ---

#[tokio::test]
async fn test_sub5_solo_error() {
    let mut conn = sender_conn(5041);
    conn.session.id_leader = 0;
    conn.session.id_mem = [0, 0, 0, 0];
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, 5, b"party hi").await;
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(
        out.outgoing[0].frame,
        spawn::sys_msg_frame("Ban chua tham gia doi.")
    );
    assert!(out.outgoing[0].frame.contains("020B"));
}

// --- spawn builders ---

#[test]
fn test_system_broadcast_frame() {
    let frame = spawn::system_broadcast_frame(5051, b"hi");
    assert!(frame.contains("0200"));
    assert!(frame.contains(&encoder::le32(5051)));
    assert_eq!(frame, spawn::chat_frame(0x00, 5051, b"hi"));
}

#[test]
fn test_input_flag_frame() {
    assert_eq!(spawn::input_flag_frame(), "F44402000208");
}

#[test]
fn test_long_memo_frames_empty() {
    assert!(spawn::long_memo_frames(5061, b"", 0).is_empty());
}

#[test]
fn test_long_memo_frames_chunked() {
    let msg = vec![b'A'; 500];
    let frames = spawn::long_memo_frames(5062, &msg, 0);
    assert!(!frames.is_empty());
    for f in &frames {
        assert!(f.contains("020B"));
    }
    assert!(frames.last().unwrap().ends_with("23656E64"));
}

// --- channel sub-opcode constants match the aLogin client tags (opcode_02.md) ---

#[test]
fn test_chat_sub_constants_match_client_tags() {
    use ts_dream::protocol::*;
    assert_eq!(CHAT_SUB_BROADCAST, 0x00); // (Công bố hệ thống)
    assert_eq!(CHAT_SUB_ANGEL, 0x01); // (Thần)Thiên thần
    assert_eq!(CHAT_SUB_NEAR, 0x02); // (Gần)
    assert_eq!(CHAT_SUB_WHISPER, 0x03); // (Thì Thầm)
    assert_eq!(CHAT_SUB_GM, 0x04); // (GM)
    assert_eq!(CHAT_SUB_LOUDSPEAKER, 0x05); // (Đài)
    assert_eq!(CHAT_SUB_GUILD, 0x06); // (Đoàn)
    assert_eq!(CHAT_SUB_LOCAL, 0x07); // (Minh)
    assert_eq!(CHAT_SUB_INPUT_FLAG, 0x08); // input-bar flag
    assert_eq!(CHAT_SUB_MEMO, 0x0B); // (Tổng Cũ) / 020B banner
    assert_eq!(CHAT_SUB_SYSTEM, 0x0C); // (Công bố hệ thống) / 020C
    assert_eq!(ts_dream::server::handlers::chat::CHAT_COOLDOWN_MS, 5_000);
    // builders render the matching wire sub bytes
    assert!(spawn::system_broadcast_frame(1, b"x").contains("0200"));
    assert!(spawn::self_talk_frame(1, b"x").contains("0207"));
    assert!(spawn::sys_msg_frame("hi").contains("020B"));
    assert!(spawn::announce_frame("hi").contains("020C"));
}

// --- 5s anti-spam cooldown: first message passes, immediate second is held ---

#[tokio::test]
async fn test_sub2_rate_limit_5s() {
    use ts_dream::protocol::CHAT_SUB_NEAR;
    let mut conn = sender_conn(5071);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out1 = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out1, CHAT_SUB_NEAR, b"one").await;
    assert_eq!(out1.outgoing.len(), 1);
    assert!(out1.outgoing[0].frame.contains("0202"));

    let mut out2 = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out2, CHAT_SUB_NEAR, b"two").await;
    assert_eq!(out2.outgoing.len(), 1);
    assert!(out2.outgoing[0].frame.contains("020B")); // wait notice, not chat
    assert!(!out2.outgoing[0].frame.contains("0202"));
}

// --- slash commands are exempt from the cooldown ---

#[tokio::test]
async fn test_slash_exempt_from_rate_limit() {
    use ts_dream::protocol::CHAT_SUB_NEAR;
    let mut conn = sender_conn(5072);
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out1 = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out1, CHAT_SUB_NEAR, b"hi").await;
    assert_eq!(out1.outgoing.len(), 1);

    let mut out2 = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out2, CHAT_SUB_NEAR, b"/where").await;
    assert_eq!(out2.outgoing.len(), 1);
    // answered, not held: default session is map 12001 (400,500)
    let expected = spawn::sys_msg_frame("MapID:12001 X:400 Y:500");
    assert_eq!(out2.outgoing[0].frame, expected);
}

// --- cooldown expiry (stale stamp) lets the next message through ---

#[tokio::test]
async fn test_rate_limit_expired_stamp_passes() {
    use ts_dream::protocol::CHAT_SUB_NEAR;
    let mut conn = sender_conn(5073);
    conn.session.last_chat_ms = 0; // epoch stamp: definitely older than 5s
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    run_chat(&mut conn, &data, &service, &mut out, CHAT_SUB_NEAR, b"again").await;
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].frame.contains("0202"));
    assert!(conn.session.last_chat_ms > 0); // stamp refreshed on accept
}
