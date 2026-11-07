//! Tests for Skill Protocol Parity, Stats Packet, Job System & Respec.
//!
//! Validates:
//! 1. Canonical skill catalog bootstrapping (376 records from Data/skills.json).
//! 2. Byte-exact layout of 05 03 stats packet (StatsPacketBuilder, offsets 96 & 97).
//! 3. Eve script skill awards (EveResultClass::Skill = 4).
//! 4. Job skill learning gates and bulk commits (0x1C 01 & 0x1C 04).
//! 5. Reborn 2 dragon ball bitmask, 8 hero ball skill sockets, and 17 4D / 17 4E frames.
//! 6. Combat job skills (passives in WarInfo, Bá ý crit, defenses).
//! 7. Respec system (opcode 0x2A) for skills and attributes.

mod common;

use std::path::PathBuf;
use ts_dream::data::loader::GameData;
use ts_dream::protocol::codecs::StatsPacketBuilder;
use ts_dream::server::session::Session;
use ts_dream::server::spawn;

#[test]
fn test_skills_data_loader_canonical_376() {
    let data_dir = PathBuf::from("Data");
    let data = GameData::load(&data_dir).expect("GameData::load must succeed");
    assert_eq!(
        data.skills.len(),
        376,
        "Canonical skill catalog must contain exactly 376 records"
    );

    // Verify representative skills
    let skill_10001 = data.skills.get(&10001).expect("Thuật mưa đá must exist");
    assert_eq!(skill_10001.name, "Thuật mưa đá");
    assert_eq!(skill_10001.thuoctinh, 1); // Địa
    assert_eq!(skill_10001.point, 1);
    assert_eq!(skill_10001.lv_max, 10);
    assert_eq!(skill_10001.id_dk[0], 0);

    let skill_10020 = data.skills.get(&10020).expect("Địa Động Quyết must exist");
    assert_eq!(skill_10020.name, "Địa Động Quyết");
    assert_eq!(skill_10020.reborn, 1);

    let skill_14038 = data.skills.get(&14038).expect("Đại chu thiên must exist");
    assert_eq!(skill_14038.name, "Đại chu thiên");
    assert_eq!(skill_14038.thuoctinh, 5); // Mind
    assert_eq!(skill_14038.lv_max, 10);
    assert_eq!(skill_14038.point, 1);

    let skill_14039 = data.skills.get(&14039).expect("Bá khí must exist");
    assert_eq!(skill_14039.name, "Bá khí");
    assert_eq!(skill_14039.id_dk[0], 14038);
    assert_eq!(skill_14039.reborn, 1);

    let skill_10027 = data.skills.get(&10027).expect("Vô Gian Chấn Ba must exist");
    assert_eq!(skill_10027.reborn, 2);
}

#[test]
fn test_stats_packet_builder_byte_exact_offsets() {
    // Test case 1: Unreborn, No Job
    let mut builder = StatsPacketBuilder::new();
    builder.thuoctinh = 2; // Water
    builder.hp = 250;
    builder.sp = 120;
    builder.int1 = 15;
    builder.atk = 20;
    builder.def = 18;
    builder.agi = 22;
    builder.hpx = 10;
    builder.spx = 8;
    builder.lv = 25;
    builder.texp = 12345;
    builder.skill_point = 7;
    builder.point = 12;
    builder.tiengtam = 50;
    builder.hp_max = 300;
    builder.sp_max = 150;
    builder.reborn = 0;
    builder.job = 0;

    let payload = builder.build_payload();
    assert_eq!(payload.len(), 113, "Base header length must be exactly 113 bytes");
    assert_eq!(payload[0], 0x05);
    assert_eq!(payload[1], 0x03);
    assert_eq!(payload[2], 2); // element
    assert_eq!(u16::from_le_bytes([payload[3], payload[4]]), 250); // HP
    assert_eq!(u16::from_le_bytes([payload[5], payload[6]]), 120); // SP at 5..6
    assert_eq!(u16::from_le_bytes([payload[24], payload[25]]), 7); // skill_point at 24..25
    assert_eq!(u16::from_le_bytes([payload[26], payload[27]]), 12); // point at 26..27

    // Literal 500 constants at offset 60..69
    for i in 0..5 {
        let val = u16::from_le_bytes([payload[60 + i * 2], payload[60 + i * 2 + 1]]);
        assert_eq!(val, 500);
    }

    // Byte 96 (offset 0x60): reborn
    assert_eq!(payload[96], 0);
    // Byte 97 (offset 0x61): job
    assert_eq!(payload[97], 0);

    // Test case 2: Reborn 1, Job 1 (Hiệp)
    builder.reborn = 1;
    builder.job = 1;
    let p1 = builder.build_payload();
    assert_eq!(p1[96], 1, "Reborn 1 must be at byte 96");
    assert_eq!(p1[97], 1, "Job 1 (Hiệp) must be at byte 97");

    // Test case 3: Reborn 2, Job 2 (Bá) with skills
    builder.reborn = 2;
    builder.job = 2;
    builder.skills = vec![(14038, 5), (14039, 10)];
    let p2 = builder.build_payload();
    assert_eq!(p2.len(), 113 + 6);
    assert_eq!(p2[96], 2, "Reborn 2 must be at byte 96");
    assert_eq!(p2[97], 2, "Job 2 (Bá) must be at byte 97");

    // Skills tail starting at offset 113
    assert_eq!(u16::from_le_bytes([p2[113], p2[114]]), 14038);
    assert_eq!(p2[115], 5);
    assert_eq!(u16::from_le_bytes([p2[116], p2[117]]), 14039);
    assert_eq!(p2[118], 10);

    // Test case 4: Other Job classes (3=Hiền, 4=Tiên)
    builder.job = 3;
    assert_eq!(builder.build_payload()[97], 3);
    builder.job = 4;
    assert_eq!(builder.build_payload()[97], 4);
}

#[test]
fn test_spawn_initial_frames_reborn_and_job_wire_parity() {
    let mut s = Session::new();
    s.id = 1001;
    s.name = b"HeroJob".to_vec();
    s.thuoctinh = 3; // Hỏa
    s.level = 120;
    s.hp = 1500;
    s.hp_max = 1500;
    s.sp = 800;
    s.sp_max = 800;
    s.skill_point = 45;
    s.point = 10;
    s.reborn = 2;
    s.job = 2; // Bá Vương
    s.skills = vec![(14038, 5), (14040, 3)];

    let frames = spawn::build_logined_sequence_session(&s);
    // Find the stats frame (OPCODE_STATS 05 03)
    let stats_frame = frames
        .iter()
        .find(|f| f.starts_with("F444") && f[8..12] == *"0503")
        .expect("05 03 stats frame must be generated");

    let raw = hex::decode(stats_frame).expect("valid hex");
    // raw[0..4] is F4 44 [len 2B LE]
    // raw[4..6] is 05 03
    // payload starts at raw[4]
    let payload = &raw[4..];
    assert_eq!(payload[96], 2, "Reborn byte at offset 96 must be 2");
    assert_eq!(payload[97], 2, "Job byte at offset 97 must be 2");
}

fn create_test_event_session(results: Vec<ts_dream::data::loaders::EveResult>) -> ts_dream::eve::auto_chain::EventSession {
    ts_dream::eve::auto_chain::EventSession {
        map_id: 1,
        eve_no: 1,
        npc_click_id: 0,
        trigger_kind: 1,
        chain_depth: 0,
        results,
        current_index: 0,
        phase: ts_dream::eve::auto_chain::EventPhase::Executing,
        last_surface_id: 0,
        last_choice_code: 0,
        battle_result: 0,
        matched_condition_no: 0,
    }
}

#[test]
fn test_eve_script_skill_reward() {
    let data = GameData::load(std::path::Path::new("Data")).expect("Failed to load GameData");

    let mut session = Session::new();
    session.id = 5001;
    session.skill_point = 25;

    // 1. Grant new skill 14003 (Thuật mua bán) via Eve Action result
    let result = ts_dream::data::loaders::EveResult {
        result_type: 0,  // Action
        result_class: 4, // Skill
        parameter: 14003,
        result_value: 1, // level 1
        ..Default::default()
    };
    session.current_event_session = Some(create_test_event_session(vec![result]));

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::talk::execute_event_step(&mut session, &data, &mut out);

    // Verify session skills updated
    assert_eq!(session.skills, vec![(14003, 1)]);
    // Verify skill points were NOT deducted
    assert_eq!(session.skill_point, 25);

    // Verify wire frame: 08 01 6E 01 01000000 B3360000 (14003 = 0x36B3)
    let frames: Vec<String> = out.outgoing.iter().map(|f| f.frame.clone()).collect();
    let expected_skill_frame = format!(
        "F4440C0008016E01{}{}",
        ts_dream::protocol::encoder::le32(1),
        ts_dream::protocol::encoder::le32(14003)
    );
    assert!(
        frames.contains(&expected_skill_frame),
        "Expected skill frame {expected_skill_frame} in {frames:?}"
    );

    // Verify system chat toast was emitted
    let toast_frame = frames
        .iter()
        .find(|f| f.starts_with("F444") && f[8..12] == *"0201")
        .expect("Sys msg toast must be emitted");
    assert!(toast_frame.len() > 14);

    // 2. Upgrade to level 3 via Eve Action
    let result_lvl3 = ts_dream::data::loaders::EveResult {
        result_type: 0,
        result_class: 4,
        parameter: 14003,
        result_value: 3,
        ..Default::default()
    };
    session.current_event_session = Some(create_test_event_session(vec![result_lvl3]));
    let mut out2 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::talk::execute_event_step(&mut session, &data, &mut out2);
    assert_eq!(session.skills, vec![(14003, 3)]);
    assert_eq!(session.skill_point, 25);

    // 3. Lower level reward should NOT downgrade
    let result_lvl2 = ts_dream::data::loaders::EveResult {
        result_type: 0,
        result_class: 4,
        parameter: 14003,
        result_value: 2,
        ..Default::default()
    };
    session.current_event_session = Some(create_test_event_session(vec![result_lvl2]));
    let mut out3 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::talk::execute_event_step(&mut session, &data, &mut out3);
    assert_eq!(session.skills, vec![(14003, 3)]);
    let frames3: Vec<String> = out3.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        !frames3.iter().any(|f| f.contains("08016E01")),
        "No skill update frame should be emitted for redundant/downgrade reward"
    );
}

#[tokio::test]
async fn test_job_skill_learning_gates_subcode_01() {
    let data = GameData::load(std::path::Path::new("Data")).expect("Failed to load GameData");
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 6001;
    conn.session.thuoctinh = 3; // Hỏa
    conn.session.skill_point = 50;
    conn.session.reborn = 1;

    // Case 1: Job = 0 (No job) -> Try to learn 14039 (Bá khí) -> Must reject
    conn.session.job = 0;
    let payload_14039 = [0xD7, 0x36, 1]; // 14039 = 0x36D7, lv 1
    let mut out1 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_learn(&mut conn, &payload_14039, &mut out1, &data, None).await;
    assert_eq!(conn.session.skills.len(), 0, "Job skill must be rejected when player has no job");

    // Case 2: Job = 1 (Hiệp Khách) -> Try to learn 14039 (Bá khí, requires Bá=2) -> Must reject
    conn.session.job = 1;
    let mut out2 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_learn(&mut conn, &payload_14039, &mut out2, &data, None).await;
    assert_eq!(conn.session.skills.len(), 0, "Job skill must be rejected on mismatched job");

    // Case 3: Job = 2 (Bá Vương) -> But missing 14038 (Great Heavenly Cycle) -> Must reject
    conn.session.job = 2;
    let mut out3 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_learn(&mut conn, &payload_14039, &mut out3, &data, None).await;
    assert_eq!(conn.session.skills.len(), 0, "Job skill must be rejected without 14038");

    // Case 4: Job = 2 and player has learned 14038 -> Must succeed
    conn.session.skills.push((14038, 1));
    let mut out4 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_learn(&mut conn, &payload_14039, &mut out4, &data, None).await;
    assert!(
        conn.session.skills.iter().any(|(id, lv)| *id == 14039 && *lv == 1),
        "Job skill must succeed when job matches and 14038 is learned"
    );
}

#[tokio::test]
async fn test_bulk_skill_commit_subcode_04_atomic() {
    let data = GameData::load(std::path::Path::new("Data")).expect("Failed to load GameData");
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 6002;
    conn.session.thuoctinh = 3; // Hỏa
    conn.session.reborn = 1;
    conn.session.job = 2; // Bá Vương
    conn.session.skill_point = 10;

    // Batch 1: Learn 14038 Lv 1, then 14039 Lv 1
    // 14038 = 0x36D6 -> D6 36 01
    // 14039 = 0x36D7 -> D7 36 01
    let payload = [0xD6, 0x36, 0x01, 0xD7, 0x36, 0x01];
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_bulk(&mut conn, &payload, &mut out, &data, None).await;

    assert_eq!(conn.session.skills, vec![(14038, 1), (14039, 1)]);
    let frames: Vec<String> = out.outgoing.iter().map(|f| f.frame.clone()).collect();
    // Must contain 6E 01 for 14038
    assert!(frames.iter().any(|f| f.contains("08016E01") && f.contains("D6360000")));
    // Must contain 6E 01 for 14039
    assert!(frames.iter().any(|f| f.contains("08016E01") && f.contains("D7360000")));
    // Must contain 25 01 remaining skill points
    assert!(frames.iter().any(|f| f.contains("08012501")));

    // Batch 2: Atomic rollback test when points are insufficient
    // Current points: < 50. Let's try to upgrade 14038 to Lv 10 and 14039 to Lv 10 with only 1 point.
    conn.session.skill_point = 1; // Insufficient!
    let prev_skills = conn.session.skills.clone();
    let excessive_payload = [0xD6, 0x36, 0x05, 0xD7, 0x36, 0x05]; // requires 4 + 4 = 8 points
    let mut out_fail = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_player_skill_bulk(&mut conn, &excessive_payload, &mut out_fail, &data, None).await;

    assert_eq!(conn.session.skills, prev_skills, "Atomic commit must leave skills unchanged on failure");
    assert_eq!(conn.session.skill_point, 1, "Skill points must remain untouched on rejection");
    assert_eq!(out_fail.outgoing.len(), 0, "No frames emitted on rejected bulk commit");
}

#[tokio::test]
async fn test_reborn_2_dragon_balls_and_hero_ball_slots() {
    let data = GameData::load(std::path::Path::new("Data")).expect("Failed to load GameData");
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 7001;
    conn.session.reborn = 2;
    conn.session.ball_point = 5;
    conn.session.skill_point = 10;

    // Construct 0x1C 05 packet payload:
    // Section 1: L1 = 2 dragon balls: ball #1, ball #3 -> L1: 02 00 00 00, bytes: 01 03
    // Section 2: L2 = 1 skill: 14038 (0x36D6) Lv 2 -> L2: 01 00 00 00, bytes: D6 36 02
    // Section 3: L3 = 6 bytes (2 slots):
    //   - slot 6: 10027 (0x272B -> 2B 27)
    //   - slot 7: 10028 (0x272C -> 2C 27)
    //   -> L3: 06 00 00 00, bytes: 06 2B 27 07 2C 27
    let mut payload = Vec::new();
    // L1
    payload.extend_from_slice(&2u32.to_le_bytes());
    payload.push(1); // Ball #1
    payload.push(3); // Ball #3
    // L2
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&14038u16.to_le_bytes());
    payload.push(2); // Lv 2 (costs base 1 + extra 1 = 2 points)
    // L3
    payload.extend_from_slice(&6u32.to_le_bytes());
    payload.push(6); // Slot 6 (maps to index 0)
    payload.extend_from_slice(&10027u16.to_le_bytes());
    payload.push(7); // Slot 7 (maps to index 1)
    payload.extend_from_slice(&10028u16.to_le_bytes());

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_skill_reborn_2(&mut conn, &payload, &mut out, &data, None).await;

    // Verify Section 1: Dragon balls mask: bit 0 and bit 2 set -> (1 | 4) = 5
    assert_eq!(conn.session.dragon_balls, 5);

    // Verify Section 2: Hero skill learned and points deducted (10 - 2 = 8)
    assert!(conn.session.skills.iter().any(|(id, lv)| *id == 14038 && *lv == 2));
    assert_eq!(conn.session.skill_point, 8, "Skill point must be deducted by 2");

    // Verify Section 3: Hero ball sockets
    assert_eq!(conn.session.skill_rb2[0], 10027);
    assert_eq!(conn.session.skill_rb2[1], 10028);

    // Verify wire frames emitted:
    let frames: Vec<String> = out.outgoing.iter().map(|f| f.frame.clone()).collect();
    // Frame 17 4D (dragon balls)
    let f174d = frames.iter().find(|f| f.contains("174D")).expect("17 4D frame must be emitted");
    // Should have ball_point = 05, balls = 01 03
    assert!(f174d.contains("174D050103"));

    // Frame 17 4E (hero ball slots)
    let f174e = frames.iter().find(|f| f.contains("174E")).expect("17 4E frame must be emitted");
    // slot 1 (idx 0 + 1) with 10027 (2B 27), slot 2 with 10028 (2C 27)
    assert!(f174e.contains("174E012B27022C27"));

    // Remaining skill points frame 08 01 25 01
    assert!(frames.iter().any(|f| f.contains("08012501") && f.contains("08000000")));

    // Verify login sequence includes 17 4D and 17 4E for reborn >= 2
    let login_frames = spawn::build_logined_sequence_session(&conn.session);
    assert!(login_frames.iter().any(|f| f.contains("174D050103")), "Login must include 17 4D");
    assert!(login_frames.iter().any(|f| f.contains("174E012B27022C27")), "Login must include 17 4E");

    // Negative test: Section 2 with insufficient points rejects skill learning
    let mut conn_broke = ts_dream::server::session::Conn::new();
    conn_broke.session.id = 7002;
    conn_broke.session.reborn = 2;
    conn_broke.session.skill_point = 0; // 0 points
    let mut out_broke = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::skills::handle_skill_reborn_2(&mut conn_broke, &payload, &mut out_broke, &data, None).await;
    assert_eq!(conn_broke.session.skills.len(), 0, "No skills should be learned when points are insufficient");
}

#[tokio::test]
async fn test_combat_job_skills_passives_and_mechanics() {
    let mut session = Session::new();
    session.id = 5001;
    session.level = 100;
    session.job = 2; // Bá
    session.reborn = 2;
    session.atk = 300;
    session.def = 100;
    session.agi = 100;
    session.int1 = 100;
    session.hp = 1000;
    session.hp_max = 1000;
    session.sp = 500;
    session.sp_max = 500;

    // Add passive skills:
    // 14039 Bá khí (lv 10: +30 Atk)
    // 14041 Lực bạt sơn hề (lv 10: +20 Atk, +300 HP)
    // 14043 Anh khí hộ thân (lv 10: +40 Def)
    // 14045 Tâm nhãn (lv 10: +30 Agi)
    // 14047 Dụng kế (lv 10: +40 Int)
    // 14051 Tiên khiếu (lv 10: +30 Int, +200 SP)
    // 14052 Hóa Huyết Hoàn Hồn (lv 5)
    session.skills = vec![
        (14039, 10),
        (14041, 10),
        (14043, 10),
        (14045, 10),
        (14047, 10),
        (14051, 10),
        (14052, 5),
    ];

    let mut battle = ts_dream::battle::construction::Battle::new(1, 1);
    battle.add_player(&session, session.id as i64, 3, 2);

    let cell_key = ts_dream::battle::engine::war_key(3, 2);
    let cell = battle.list_war.get(&cell_key).expect("Player cell must exist");
    assert_eq!(cell.lv, 100, "Level must be 100");
    assert_eq!(cell.atk, 350, "Atk must be 300 + 30 + 20 = 350");
    assert_eq!(cell.def, 140, "Def must be 100 + 40 = 140");
    assert_eq!(cell.agi, 130, "Agi must be 100 + 30 = 130");
    assert_eq!(cell.int1, 170, "Int must be 100 + 40 + 30 = 170");
    assert_eq!(cell.hp_max, 1300, "HP max must be 1000 + 300 = 1300");
    assert_eq!(cell.sp_max, 700, "SP max must be 500 + 200 = 700");
    assert_eq!(cell.hoa_huyet_lv, 5, "hoa_huyet_lv must be 5");

    // Test Bá ý (14040) forces 100% crit in combat formula
    let mut rng = ts_dream::battle::rng::DotNetRandom::new(12345);
    let mut attacker = cell.clone();
    attacker.type19_id = 14040; // Bá ý active buff
    let defender = ts_dream::battle::engine::WarInfo {
        typ: 3, // enemy npc
        lv: 50,
        def: 50,
        hp: 1000,
        ..Default::default()
    };
    let res = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng,
        &attacker,
        &defender,
        None,
    );
    assert!(res.hit);
    assert!(res.thunder, "Bá ý must guarantee critical hit (thunder = true)");

    // Test Ngưng khí hộ thân (14046) 30% damage reduction
    let mut defender_ngung_khi = defender.clone();
    defender_ngung_khi.type19_id = 14046;
    let mut rng1 = ts_dream::battle::rng::DotNetRandom::new(42);
    let mut rng2 = ts_dream::battle::rng::DotNetRandom::new(42);
    let base_attacker = cell.clone(); // without Bá ý
    let res_normal = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng1,
        &base_attacker,
        &defender,
        None,
    );
    let res_reduced = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng2,
        &base_attacker,
        &defender_ngung_khi,
        None,
    );
    assert_eq!(
        res_reduced.damage,
        (res_normal.damage * 7) / 10,
        "Ngưng khí hộ thân must reduce damage by 30%"
    );

    // Test Thiên thần thuẫn (14044) 50% damage reduction
    let mut defender_thien_than = defender.clone();
    defender_thien_than.type19_id = 14044;
    let mut rng3 = ts_dream::battle::rng::DotNetRandom::new(42);
    let res_shield = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng3,
        &base_attacker,
        &defender_thien_than,
        None,
    );
    assert_eq!(
        res_shield.damage,
        res_normal.damage / 2,
        "Thiên thần thuẫn must reduce damage by 50%"
    );

    // Test Sóng cuồng (14042) active physical attack formula
    let mut rng4 = ts_dream::battle::rng::DotNetRandom::new(42);
    let res_song_cuong = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng4,
        &base_attacker,
        &defender,
        Some(ts_dream::battle::combat_formula::SkillFormulaInput {
            element: 5,
            numerical: 120,
            attribute: 28,
            round: 0,
            hit_status: 0,
        }),
    );
    assert!(res_song_cuong.hit);
    assert!(res_song_cuong.damage > res_normal.damage, "Sóng cuồng (120% power) must deal higher damage than basic attack");

    // Test Càn Khôn Thiên Lôi (14054) active magical attack formula
    let mut rng5 = ts_dream::battle::rng::DotNetRandom::new(42);
    let res_thien_loi = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng5,
        &base_attacker,
        &defender,
        Some(ts_dream::battle::combat_formula::SkillFormulaInput {
            element: 5,
            numerical: 150,
            attribute: 27, // INT-based
            round: 0,
            hit_status: 0,
        }),
    );
    assert!(res_thien_loi.hit);
    assert!(res_thien_loi.damage > 0);

    // Test Thánh Linh thuật (14053) buff dispel & holy damage via public run_turn
    let mut battle2 = ts_dream::battle::construction::Battle::new(2, 1);
    let mut player_session = session.clone();
    player_session.sp = 500;
    battle2.add_player(&player_session, player_session.id as i64, 3, 2);

    let buffed_enemy = ts_dream::battle::engine::WarInfo {
        typ: 3,
        team: 2,
        row: 0,
        col: 2,
        id: 999,
        lv: 50,
        hp: 1000,
        hp_max: 1000,
        type4_id: 10010, // Kết giới
        type15_id: 12345,
        type19_id: 14040, // Bá ý
        ..Default::default()
    };
    battle2.list_war.insert(ts_dream::battle::engine::war_key(0, 2), buffed_enemy);

    let data_game = ts_dream::data::loader::GameData::load(std::path::Path::new("Data")).expect("load data");
    let (npcs, items, pet_slots, players) = (
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
    );
    let bdata = ts_dream::battle::runner::BattleData::new(
        &npcs,
        &data_game.skills,
        &items,
        &pet_slots,
        &players,
        &data_game.texps,
        None,
        0,
        0,
    );
    let mut commands = ts_dream::battle::runner::BattleCommands::new();
    commands.insert(
        (3, 2),
        ts_dream::battle::runner::BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: ts_dream::battle::runner::BattleAction::CastSkill {
                skill_id: 14053,
                skill_lv: 5,
            },
        },
    );
    let mut out_dispel = Vec::new();
    battle2.run_turn(&bdata, &commands, &mut out_dispel);

    let enemy_after = battle2.list_war.get(&ts_dream::battle::engine::war_key(0, 2)).expect("Enemy must exist");
    assert_eq!(enemy_after.type4_id, 0, "14053 must dispel type4 buffs");
    assert_eq!(enemy_after.type15_id, 0, "14053 must dispel type15 buffs");
    assert_eq!(enemy_after.type19_id, 0, "14053 must dispel type19 buffs");
    assert_eq!(enemy_after.hp, 715, "14053 must deal holy damage scaled with INT ((170/2) + 5 * 40 = 285 dmg, hp 1000 -> 715)");
}

#[tokio::test]
async fn test_respec_memory_manual_and_careful_pouch_opcode_0x2a() {
    let data_dir = PathBuf::from("Data");
    let data = std::sync::Arc::new(GameData::load(&data_dir).expect("GameData::load must succeed"));

    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 6001;
    conn.session.skills = vec![(10001, 5), (14039, 3)];
    conn.session.skill_point = 2;
    conn.session.int1 = 25;
    conn.session.point = 4;
    conn.session.homdo = vec![
        ts_dream::server::session::InventoryItem {
            slot: 1,
            id: 46035, // Memory Manual
            count: 2,
            ..Default::default()
        },
        ts_dream::server::session::InventoryItem {
            slot: 2,
            id: 46203, // Careful Pouch
            count: 1,
            ..Default::default()
        },
    ];

    // 1. Using Memory Manual (slot 1) triggers S→C 2A 01
    let mut out_use1 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::use_item::use_item(
        &mut conn,
        &[1, 1, 0], // slot 1, count 1
        &mut out_use1,
        None,
        &data,
    )
    .await;
    let frames1: Vec<String> = out_use1.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        frames1.iter().any(|f| f.contains("2A01")),
        "Using Memory Manual must open TBK_SkillRegressForm via 2A 01 frame"
    );

    // 2. Using Careful Pouch (slot 2) triggers S→C 2A 02
    let mut out_use2 = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::use_item::use_item(
        &mut conn,
        &[2, 1, 0], // slot 2, count 1
        &mut out_use2,
        None,
        &data,
    )
    .await;
    let frames2: Vec<String> = out_use2.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        frames2.iter().any(|f| f.contains("2A02")),
        "Using Careful Pouch must open TBK_AttrRegressForm via 2A 02 frame"
    );

    // 3. Confirm Skill Downgrade via C→S 0x2A 01: skill 14039
    let skill_payload = 14039u16.to_le_bytes();
    let mut out_respec_sk = ts_dream::server::dispatcher::HandleOutcome::default();
    let battle_service = ts_dream::battle::service::BattleService::new(data.clone());
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out_respec_sk,
            opcode: 0x2A,
            sub: 1,
            payload: &skill_payload,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }

    // Verify in-memory mutations
    let sk_14039 = conn.session.skills.iter().find(|(id, _)| *id == 14039).expect("14039 still exists");
    assert_eq!(sk_14039.1, 2, "Skill 14039 level must be reduced from 3 to 2");
    assert_eq!(conn.session.skill_point, 3, "Skill point refunded from 2 to 3");
    let manual_item = conn.session.homdo.iter().find(|i| i.slot == 1).expect("Manual still in slot 1");
    assert_eq!(manual_item.count, 1, "Memory manual count reduced from 2 to 1");

    // Verify wire frames: 17 09, 08 01 6E 02, 08 01 25 01, 2A 01
    let sk_frames: Vec<String> = out_respec_sk.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        sk_frames.iter().any(|f| f == &ts_dream::server::inventory::item_count_changed_frame(1, 1)),
        "17 09 frame must match canonical item_count_changed_frame(1, 1)"
    );
    assert!(sk_frames.iter().any(|f| f.contains("08016E02")), "08 01 6E 02 must emit delta level reduction");
    assert!(sk_frames.iter().any(|f| f.contains("08012501")), "08 01 25 01 must emit updated skill points");

    // 4. Confirm Attribute Reduction via C→S 0x2A 02: attr 1 (Int)
    let attr_payload = [1u8]; // Int
    let mut out_respec_attr = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out_respec_attr,
            opcode: 0x2A,
            sub: 2,
            payload: &attr_payload,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }

    // Verify in-memory mutations
    assert_eq!(conn.session.int1, 24, "Int reduced from 25 to 24");
    assert_eq!(conn.session.point, 5, "Stat point refunded from 4 to 5");
    assert!(!conn.session.homdo.iter().any(|i| i.slot == 2), "Careful pouch consumed and slot 2 cleared");

    // Verify wire frames: 17 09, 08 01 26 01, 08 01 1B, 2A 02
    let attr_frames: Vec<String> = out_respec_attr.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        attr_frames.iter().any(|f| f == &ts_dream::server::inventory::item_count_changed_frame(2, 0)),
        "17 09 frame must match canonical item_count_changed_frame(2, 0)"
    );
    assert!(attr_frames.iter().any(|f| f.contains("08012601")), "08 01 26 01 must emit updated stat points");
    assert!(attr_frames.iter().any(|f| f.contains("08011B")), "08 01 1B must emit updated Int stat");

    // 5. Negative test: Respec unlearned skill fails cleanly with zero mutations
    let bad_skill = 9999u16.to_le_bytes();
    let mut out_fail = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out_fail,
            opcode: 0x2A,
            sub: 1,
            payload: &bad_skill,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }
    assert_eq!(conn.session.skill_point, 3, "Skill points must not change on failure");
    let fail_frames: Vec<String> = out_fail.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(fail_frames.iter().any(|f| f.contains("020102")), "Must emit system warning frame on failure");
}

#[tokio::test]
async fn test_respec_attr_floor_enforcement() {
    let data_dir = PathBuf::from("Data");
    let data = std::sync::Arc::new(GameData::load(&data_dir).expect("GameData::load must succeed"));

    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 6002;
    conn.session.thuoctinh = 3; // Fire: starting base stat for Atk is 2
    conn.session.atk = 2;       // Already at floor
    conn.session.point = 0;
    conn.session.homdo = vec![
        ts_dream::server::session::InventoryItem {
            slot: 1,
            id: 46203, // Careful Pouch
            count: 1,
            ..Default::default()
        },
    ];

    let battle_service = ts_dream::battle::service::BattleService::new(data.clone());
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let payload = [2u8]; // Atk
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out,
            opcode: 0x2A,
            sub: 2,
            payload: &payload,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }

    assert_eq!(conn.session.atk, 2, "Atk must not be reduced below starting base of 2");
    assert_eq!(conn.session.point, 0, "No stat points awarded on floor rejection");
    assert_eq!(conn.session.homdo[0].count, 1, "Careful pouch must not be consumed on floor rejection");

    let frames: Vec<String> = out.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        frames.iter().any(|f| f.contains("020102")),
        "System warning toast must be sent upon floor rejection"
    );
}

#[test]
fn test_instigation_status_bonus_mechanics() {
    let mut session = ts_dream::server::session::Session::new();
    session.id = 7001;
    session.thuoctinh = 5;
    session.level = 100;
    session.skills = vec![(ts_dream::protocol::SKILL_INSTIGATION, 10)]; // Sách động level 10

    let mut battle = ts_dream::battle::construction::Battle::new(1, 1);
    battle.add_player(&session, session.id as i64, 3, 2);

    let cell = battle.cell(3, 2).expect("Player cell must exist");
    assert_eq!(cell.sach_dong_lv, 10, "Sách động level must be populated into WarInfo");

    let mut rng = ts_dream::battle::rng::DotNetRandom::new(42);
    let target = ts_dream::battle::engine::WarInfo {
        typ: 3,
        lv: 100,
        int1: 100,
        agi: 10,
        hp: 1000,
        ..Default::default()
    };
    let mut attacker = cell.clone();
    attacker.agi = 100;
    attacker.int1 = 100;
    attacker.sach_dong_lv = 10; // +20% bonus

    let skill_input = ts_dream::battle::combat_formula::SkillFormulaInput {
        element: 1,
        numerical: 100,
        attribute: 27,
        round: 3,
        hit_status: 1, // Status effect
    };

    let res = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng,
        &attacker,
        &target,
        Some(skill_input),
    );
    assert!(res.hit);
    assert_eq!(res.inflicted_status, 1, "Status must be inflicted with Instigation bonus");
    assert_eq!(res.status_rounds, 3);
}

#[tokio::test]
async fn test_bulk_skill_commit_atomicity_on_insufficient_sp() {
    let data_dir = PathBuf::from("Data");
    let data = std::sync::Arc::new(GameData::load(&data_dir).expect("GameData::load must succeed"));

    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 8001;
    conn.session.reborn = 0;
    conn.session.job = 0;
    conn.session.skill_point = 2; // Only 2 SP available
    conn.session.skills = vec![(10001, 1)];

    let battle_service = ts_dream::battle::service::BattleService::new(data.clone());
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();

    // Payload requesting 10001 (upgrade lv 1 -> 3, cost 2 SP) and 10002 (learn lv 0 -> 1, cost 1 SP)
    // 0x1C 04 format is pure 3-byte chunks (skill_id LE16 + target_lv 1B) with NO prefix length.
    // Total cost = 3 SP > 2 SP available
    let mut payload = Vec::new();
    payload.extend_from_slice(&10001u16.to_le_bytes());
    payload.push(3); // target lv 3
    payload.extend_from_slice(&10002u16.to_le_bytes());
    payload.push(1); // target lv 1

    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out,
            opcode: 0x1C,
            sub: 4,
            payload: &payload,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::skills::handle_skills(&mut ctx).await;
    }

    // Atomic: since 3 SP > 2 SP, NO mutations must occur
    assert_eq!(conn.session.skill_point, 2, "Skill point must remain 2");
    assert_eq!(conn.session.skills.len(), 1, "No new skills learned");
    assert_eq!(conn.session.skills[0].1, 1, "Skill 10001 level must remain 1");
}

#[tokio::test]
async fn test_reborn_2_rollback_atomicity_on_invalid_hero_skill() {
    let data_dir = PathBuf::from("Data");
    let data = std::sync::Arc::new(GameData::load(&data_dir).expect("GameData::load must succeed"));

    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 8002;
    conn.session.reborn = 2;
    conn.session.skill_point = 1;
    conn.session.dragon_balls = 0;
    conn.session.skill_rb2 = [0; 8];

    let battle_service = ts_dream::battle::service::BattleService::new(data.clone());
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();

    // Section 1: L1 = 3 dragon balls (1, 2, 3) -> mask would be 7
    // Section 2: L2 = 1 hero skill: 10001 target lv 5 (requires 5 SP > 1 SP available)
    // Section 3: L3 = 3 bytes (1 socket): slot 6, skill 10027
    let mut payload = Vec::new();
    payload.extend_from_slice(&3u32.to_le_bytes()); // 3 dragon balls count
    payload.push(1);
    payload.push(2);
    payload.push(3);
    payload.extend_from_slice(&1u32.to_le_bytes()); // 1 hero skill mutation
    payload.extend_from_slice(&10001u16.to_le_bytes());
    payload.push(5); // target lv 5 (cost exceeds 1 SP)
    payload.extend_from_slice(&3u32.to_le_bytes()); // 3 bytes for 1 socket
    payload.push(6); // slot 6
    payload.extend_from_slice(&10027u16.to_le_bytes());

    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out,
            opcode: 0x1C,
            sub: 5,
            payload: &payload,
            decoded: &[],
            env: ts_dream::server::dispatcher::ServerEnv::none(),
        };
        ts_dream::server::handlers::skills::handle_skills(&mut ctx).await;
    }

    // Atomic: since Section 2 fails, Section 1 and Section 3 must NOT be applied to session
    assert_eq!(conn.session.dragon_balls, 0, "Dragon balls must not be mutated");
    assert_eq!(conn.session.skill_point, 1, "Skill points must remain untouched");
    assert_eq!(conn.session.skill_rb2[0], 0, "RB2 sockets must not be mutated");
}

#[tokio::test]
async fn test_respec_db_failure_restores_ram_snapshot_and_preserves_inventory() {
    let pool = common::test_db::setup_test_db().await;
    let repos = ts_dream::db::modern::sqlite::SqliteRepositories::new(pool.clone());
    let data_dir = PathBuf::from("Data");
    let data = std::sync::Arc::new(GameData::load(&data_dir).expect("GameData::load must succeed"));

    // 1. Create character in SQLite test db
    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create test account");
    let pid = acc_id as u32;

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 50,
        sex: 1,
        hair: 1,
        element: 3, // Fire (Atk floor = 2)
        map_id: 10817,
        map_x: 442,
        map_y: 758,
    };
    let mut initial_session = Session::new();
    initial_session.id = pid;
    initial_session.name = format!("RespecTest_{pid}").into_bytes();
    initial_session.thuoctinh = 3;
    initial_session.int1 = 20;
    initial_session.atk = 20;
    initial_session.point = 5;
    initial_session.skill_point = 5;
    initial_session.skills = vec![(10001, 3)];
    initial_session.homdo = vec![
        ts_dream::server::session::InventoryItem {
            slot: 1,
            id: ts_dream::server::handlers::respec::ITEM_MEMORY_MANUAL_PRIMARY, // 46035
            count: 3,
            ..Default::default()
        },
        ts_dream::server::session::InventoryItem {
            slot: 2,
            id: ts_dream::server::handlers::respec::ITEM_CAREFUL_POUCH, // 46203
            count: 2,
            ..Default::default()
        },
    ];

    let char_name = initial_session.name.clone();
    repos
        .sessions()
        .create_and_seed(i64::from(pid), &char_name, &seed, &mut initial_session, b"pass1", b"pass2")
        .await
        .expect("create_and_seed must succeed");

    // Persist skill in DB
    let _ = ts_dream::db::persist::upsert_skill(Some(&pool), pid, 10001, 3, 10, 0).await;

    // Create a SQLite trigger that aborts updates on character_skills to simulate a DB failure
    sqlx::query("CREATE TRIGGER test_fail_skill_update BEFORE UPDATE ON character_skills BEGIN SELECT RAISE(ABORT, 'forced test DB error'); END;")
        .execute(&pool.write)
        .await
        .expect("create trigger");

    let battle_service = ts_dream::battle::service::BattleService::new(data.clone());
    let env = ts_dream::server::dispatcher::ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let mut conn = ts_dream::server::session::Conn::new();
    conn.session = initial_session.clone();

    // Attempt skill respec (should hit DB error trigger and roll back)
    let skill_payload = 10001u16.to_le_bytes();
    let mut out_skill = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out_skill,
            opcode: 0x2A,
            sub: 1,
            payload: &skill_payload,
            decoded: &[],
            env,
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }

    // Verify RAM session snapshot rollback: skills, skill points, and inventory intact!
    assert_eq!(conn.session.skills, initial_session.skills, "Skills must roll back to snapshot on DB error");
    assert_eq!(conn.session.skill_point, initial_session.skill_point, "Skill point must roll back to snapshot on DB error");
    assert_eq!(conn.session.homdo[0].count, 3, "Memory Manual count must NOT be deducted on DB error");

    let skill_frames: Vec<String> = out_skill.outgoing.iter().map(|f| f.frame.clone()).collect();
    // Must emit native error ACK: F4 44 03 00 2A 01 01
    assert!(skill_frames.iter().any(|f| f == "F44403002A0101"), "Server must emit native error ACK frame 2A 01 01");
    // Must NOT emit success ACK: 2A 01 00
    assert!(!skill_frames.iter().any(|f| f.contains("2A0100")), "Server must NOT emit success frame on DB error");

    // Now test attribute respec failure rollback
    // Drop skill trigger and create character update trigger
    sqlx::query("DROP TRIGGER test_fail_skill_update").execute(&pool.write).await.expect("drop trigger");
    sqlx::query("CREATE TRIGGER test_fail_attr_update BEFORE UPDATE ON characters BEGIN SELECT RAISE(ABORT, 'forced test DB error'); END;")
        .execute(&pool.write)
        .await
        .expect("create trigger");

    let attr_payload = [1u8]; // Int
    let mut out_attr = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &battle_service,
            out: &mut out_attr,
            opcode: 0x2A,
            sub: 2,
            payload: &attr_payload,
            decoded: &[],
            env,
        };
        ts_dream::server::handlers::respec::handle_respec(&mut ctx).await;
    }

    // Verify RAM session snapshot rollback: attributes, free points, and inventory intact!
    assert_eq!(conn.session.int1, initial_session.int1, "Int stat must roll back to snapshot on DB error");
    assert_eq!(conn.session.point, initial_session.point, "Free points must roll back to snapshot on DB error");
    assert_eq!(conn.session.homdo[1].count, 2, "Careful Pouch count must NOT be deducted on DB error");

    let attr_frames: Vec<String> = out_attr.outgoing.iter().map(|f| f.frame.clone()).collect();
    // Must emit native error ACK: F4 44 03 00 2A 02 01
    assert!(attr_frames.iter().any(|f| f == "F44403002A0201"), "Server must emit native error ACK frame 2A 02 01");
    // Must NOT emit success ACK: 2A 02 00
    assert!(!attr_frames.iter().any(|f| f.contains("2A0200")), "Server must NOT emit success frame on DB error");

    // Clean up trigger
    sqlx::query("DROP TRIGGER test_fail_attr_update").execute(&pool.write).await.expect("drop trigger");
}

#[tokio::test]
async fn test_celestial_shield_ally_targeting_and_reduction_via_battle_runner() {
    let data_game = GameData::load(std::path::Path::new("Data")).expect("load data");

    let mut battle = ts_dream::battle::construction::Battle::new(1, 1);
    let mut caster = Session::new();
    caster.id = 9001;
    caster.level = 100;
    caster.hp = 1000;
    caster.hp_max = 1000;
    caster.sp = 500;
    caster.sp_max = 500;
    caster.def = 100;
    caster.agi = 100;
    caster.skills = vec![(ts_dream::protocol::SKILL_CELESTIAL_SHIELD, 5)];
    battle.add_player(&caster, caster.id as i64, 3, 2);

    let ally = ts_dream::battle::engine::WarInfo {
        typ: 2,
        team: 1, // Same team as caster
        row: 3,
        col: 3,
        id: 9002,
        lv: 100,
        hp: 1000,
        hp_max: 1000,
        def: 50,
        ..Default::default()
    };
    battle.list_war.insert(ts_dream::battle::engine::war_key(3, 3), ally);

    let enemy = ts_dream::battle::engine::WarInfo {
        typ: 3,
        team: 2,
        row: 0,
        col: 3,
        id: 9999,
        lv: 100,
        hp: 1000,
        atk: 200,
        ..Default::default()
    };
    battle.list_war.insert(ts_dream::battle::engine::war_key(0, 3), enemy);

    let (npcs, items, pet_slots, players) = (
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
    );
    let bdata = ts_dream::battle::runner::BattleData::new(
        &npcs,
        &data_game.skills,
        &items,
        &pet_slots,
        &players,
        &data_game.texps,
        None,
        0,
        0,
    )
    .with_binary_skills(&data_game.binary_skill_defs);

    // Turn 1: Caster (3, 2) casts Celestial Shield (14044) on ally (3, 3)
    let mut commands = ts_dream::battle::runner::BattleCommands::new();
    commands.insert(
        (3, 2),
        ts_dream::battle::runner::BattleCommand {
            row: 3,
            col: 2,
            target_row: 3,
            target_col: 3,
            action: ts_dream::battle::runner::BattleAction::CastSkill {
                skill_id: ts_dream::protocol::SKILL_CELESTIAL_SHIELD as i64,
                skill_lv: 5,
            },
        },
    );
    let mut out_turn1 = Vec::new();
    battle.run_turn(&bdata, &commands, &mut out_turn1);

    let ally_after_turn1 = battle.list_war.get(&ts_dream::battle::engine::war_key(3, 3)).expect("Ally exists");
    assert_eq!(ally_after_turn1.type19_id, i64::from(ts_dream::protocol::SKILL_CELESTIAL_SHIELD), "Ally must have Celestial Shield buff");
    assert!(ally_after_turn1.type19_turn >= 2, "Celestial Shield buff duration must be 2..4 turns");

    // Turn 2: Verify damage reduction against shielded ally
    // Basic attack from base enemy against unshielded clone
    let mut rng_norm = ts_dream::battle::rng::DotNetRandom::new(42);
    let mut unshielded_ally = ally_after_turn1.clone();
    unshielded_ally.type19_id = 0;
    let res_unshielded = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng_norm,
        &battle.list_war[&ts_dream::battle::engine::war_key(0, 3)],
        &unshielded_ally,
        None,
    );

    let mut rng_shield = ts_dream::battle::rng::DotNetRandom::new(42);
    let res_shielded = ts_dream::battle::combat_formula::calculate_attack(
        &mut rng_shield,
        &battle.list_war[&ts_dream::battle::engine::war_key(0, 3)],
        ally_after_turn1,
        None,
    );

    assert_eq!(res_shielded.damage, res_unshielded.damage / 2, "Celestial Shield must reduce incoming damage by 50%");
}

#[tokio::test]
async fn test_post_battle_eve_skill_reward_persistence() {
    let pool = common::test_db::setup_test_db().await;
    let data_dir = PathBuf::from("Data");
    let data = GameData::load(&data_dir).expect("GameData::load must succeed");

    let acc_id = ts_dream::db::accounts::create(&pool, "postbattle1", "pass")
        .await
        .expect("create test account");
    let pid = acc_id as u32;

    let mut session = Session::new();
    session.id = pid;
    session.name = format!("PostBattle_{pid}").into_bytes();
    session.skill_point = 10;

    // Eve results: fight result (index 0), followed by skill reward action (index 1)
    let fight_step = ts_dream::data::loaders::EveResult {
        result_type: 3, // Battle
        result_mean_no: 1,
        ..Default::default()
    };
    let skill_step = ts_dream::data::loaders::EveResult {
        result_type: 0,  // Action
        result_class: 4, // Skill
        parameter: 14003, // Thuật mua bán
        result_value: 1, // level 1
        ..Default::default()
    };
    let mut ev_session = create_test_event_session(vec![fight_step, skill_step]);
    ev_session.phase = ts_dream::eve::auto_chain::EventPhase::AwaitingBattle;
    ev_session.current_index = 0; // Parked on battle
    session.current_event_session = Some(ev_session);

    let (frames, next_battle) = ts_dream::server::handlers::talk::resume_eve_after_battle(
        &mut session,
        ts_dream::battle::runner::Outcome::PlayerWin,
        &data,
        Some(&pool),
    )
    .await;

    assert!(next_battle.is_none());
    assert_eq!(session.skills, vec![(14003, 1)], "Session skills in RAM must contain awarded skill");

    // Verify frames: must emit 08 01 6E 01
    assert!(
        frames.iter().any(|f| f.contains("08016E01")),
        "08 01 6E 01 frame must be emitted"
    );

    // Verify SQLite persistence: character_skills table must contain the record!
    let row = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT playerid, skillid, level FROM character_skills WHERE playerid = ? AND skillid = ?"
    )
    .bind(i64::from(pid))
    .bind(14003i64)
    .fetch_one(&pool.read)
    .await
    .expect("Skill must be persisted to character_skills");

    assert_eq!(row.0, i64::from(pid));
    assert_eq!(row.1, 14003);
    assert_eq!(row.2, 1);
}

#[tokio::test]
async fn test_eve_skill_reward_db_failure_rolls_back_ram_and_suppresses_frame() {
    let pool = common::test_db::setup_test_db().await;
    let data_dir = PathBuf::from("Data");
    let data = GameData::load(&data_dir).expect("GameData::load must succeed");

    let mut session = Session::new();
    session.id = 9991;
    session.skills = Vec::new();

    let skill_step = ts_dream::data::loaders::EveResult {
        result_type: 0,
        result_class: 4,
        parameter: 14003,
        result_value: 1,
        ..Default::default()
    };
    session.current_event_session = Some(create_test_event_session(vec![skill_step]));

    // Install trigger to abort insertion on character_skills
    sqlx::query("CREATE TRIGGER test_fail_eve_skill BEFORE INSERT ON character_skills BEGIN SELECT RAISE(ABORT, 'forced test DB error'); END;")
        .execute(&pool.write)
        .await
        .expect("create trigger");

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    ts_dream::server::handlers::talk::execute_event_step_with_pool(
        &mut session,
        &data,
        Some(&pool),
        &mut out,
    )
    .await;

    // RAM session must NOT be mutated!
    assert_eq!(session.skills.len(), 0, "Skills must NOT be added to session when DB write fails");

    // Frame 08 01 6E 01 must NOT be emitted!
    let frames: Vec<String> = out.outgoing.iter().map(|f| f.frame.clone()).collect();
    assert!(
        !frames.iter().any(|f| f.contains("08016E01")),
        "08 01 6E 01 frame must NOT be emitted when persistence fails"
    );

    // Clean up trigger
    sqlx::query("DROP TRIGGER test_fail_eve_skill")
        .execute(&pool.write)
        .await
        .expect("drop trigger");
}

#[tokio::test]
async fn test_holy_spirit_dispel_integrity_and_damage_scaling() {
    let data_game = GameData::load(std::path::Path::new("Data")).expect("load data");

    // Case 1: Target without Type 19 buff -> Holy Spirit does NOT broadcast purge_status(PURGE_TYPE_AURA)
    let mut battle = ts_dream::battle::construction::Battle::new(1, 1);
    let mut caster = Session::new();
    caster.id = 9010;
    caster.level = 100;
    caster.hp = 1000;
    caster.hp_max = 1000;
    caster.sp = 500;
    caster.sp_max = 500;
    caster.int1 = 100;
    caster.skills = vec![(ts_dream::protocol::SKILL_HOLY_SPIRIT, 5)];
    battle.add_player(&caster, caster.id as i64, 3, 2);

    let enemy = ts_dream::battle::engine::WarInfo {
        typ: 3,
        team: 2,
        row: 0,
        col: 2,
        id: 9991,
        lv: 100,
        hp: 1000,
        hp_max: 1000,
        def: 50,
        ..Default::default()
    };
    battle.list_war.insert(ts_dream::battle::engine::war_key(0, 2), enemy);

    let (npcs, items, pet_slots, players) = (
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
    );
    let bdata = ts_dream::battle::runner::BattleData::new(
        &npcs,
        &data_game.skills,
        &items,
        &pet_slots,
        &players,
        &data_game.texps,
        None,
        0,
        0,
    )
    .with_binary_skills(&data_game.binary_skill_defs);

    let mut commands = ts_dream::battle::runner::BattleCommands::new();
    commands.insert(
        (3, 2),
        ts_dream::battle::runner::BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: ts_dream::battle::runner::BattleAction::CastSkill {
                skill_id: ts_dream::protocol::SKILL_HOLY_SPIRIT as i64,
                skill_lv: 5,
            },
        },
    );

    let mut out = Vec::new();
    battle.run_turn(&bdata, &commands, &mut out);

    // Extract broadcast frames
    let aura_purge_hex = ts_dream::battle::packets::purge_status(0, 2, ts_dream::battle::packets::PURGE_TYPE_AURA);
    let broadcast_frames: Vec<String> = out.iter().filter_map(|o| match o {
        ts_dream::battle::runner::Out::Broadcast(f) => Some(f.clone()),
        _ => None,
    }).collect();

    assert!(
        !broadcast_frames.contains(&aura_purge_hex),
        "Holy Spirit on unbuffed target must NOT broadcast aura purge"
    );

    let enemy_cell = &battle.list_war[&ts_dream::battle::engine::war_key(0, 2)];
    assert_eq!(enemy_cell.type19_id, 0, "Target must NOT be given type19_id from Holy Spirit");
    let dmg_low_int = 1000 - enemy_cell.hp;
    assert!(dmg_low_int > 0, "Holy Spirit must inflict holy damage");

    // Case 2: Target WITH Celestial Shield (14044, Type 19 buff) -> Purges buff and clears aura
    let mut battle2 = ts_dream::battle::construction::Battle::new(1, 1);
    battle2.add_player(&caster, caster.id as i64, 3, 2);

    let enemy_shielded = ts_dream::battle::engine::WarInfo {
        typ: 3,
        team: 2,
        row: 0,
        col: 2,
        id: 9992,
        lv: 100,
        hp: 1000,
        hp_max: 1000,
        def: 50,
        type19_id: i64::from(ts_dream::protocol::SKILL_CELESTIAL_SHIELD),
        type19_lv: 5,
        type19_turn: 3,
        ..Default::default()
    };
    battle2.list_war.insert(ts_dream::battle::engine::war_key(0, 2), enemy_shielded);

    let mut out2 = Vec::new();
    battle2.run_turn(&bdata, &commands, &mut out2);

    let broadcast_frames2: Vec<String> = out2.iter().filter_map(|o| match o {
        ts_dream::battle::runner::Out::Broadcast(f) => Some(f.clone()),
        _ => None,
    }).collect();

    assert!(
        broadcast_frames2.contains(&aura_purge_hex),
        "Holy Spirit on shielded target MUST broadcast aura purge"
    );
    let clear_status_hex = ts_dream::battle::packets::clear_unit_status(0, 2, ts_dream::battle::packets::STATUS_DMG_BONUS_PCT);
    assert!(
        broadcast_frames2.contains(&clear_status_hex),
        "Holy Spirit on shielded target MUST clear unit status"
    );

    let enemy2_cell = &battle2.list_war[&ts_dream::battle::engine::war_key(0, 2)];
    assert_eq!(enemy2_cell.type19_id, 0, "Celestial Shield must be removed by Holy Spirit");
    assert_eq!(enemy2_cell.type19_turn, 0);

    // Case 3: Damage scaling: Attacker with high INT (250) vs low INT (100)
    let mut battle3 = ts_dream::battle::construction::Battle::new(1, 1);
    let mut high_int_caster = caster.clone();
    high_int_caster.id = 9011;
    high_int_caster.int1 = 250;
    battle3.add_player(&high_int_caster, high_int_caster.id as i64, 3, 2);

    let enemy3 = ts_dream::battle::engine::WarInfo {
        typ: 3,
        team: 2,
        row: 0,
        col: 2,
        id: 9993,
        lv: 100,
        hp: 1000,
        hp_max: 1000,
        def: 50,
        ..Default::default()
    };
    battle3.list_war.insert(ts_dream::battle::engine::war_key(0, 2), enemy3);

    let mut out3 = Vec::new();
    battle3.run_turn(&bdata, &commands, &mut out3);

    let enemy3_cell = &battle3.list_war[&ts_dream::battle::engine::war_key(0, 2)];
    let dmg_high_int = 1000 - enemy3_cell.hp;
    assert!(
        dmg_high_int > dmg_low_int,
        "Higher INT ({dmg_high_int}) must yield strictly more Holy Spirit damage than lower INT ({dmg_low_int})"
    );
}

#[test]
fn test_sach_dong_reroll_shared_helper() {
    let mut battle = ts_dream::battle::construction::Battle::new(1, 1);
    let mut attacker = ts_dream::battle::engine::WarInfo {
        sach_dong_lv: 50, // 100% bonus
        ..Default::default()
    };
    let mut hit = 0i64; // miss_status::MISS = 0
    battle.apply_sach_dong_reroll(&attacker, &mut hit);
    assert_eq!(hit, 1, "MISS must be rerolled to ATTACK with 100% Sach Dong bonus");

    // Case with sach_dong_lv = 0 -> must stay MISS
    attacker.sach_dong_lv = 0;
    let mut hit2 = 0i64;
    battle.apply_sach_dong_reroll(&attacker, &mut hit2);
    assert_eq!(hit2, 0, "MISS must not be rerolled when sach_dong_lv is 0");
}





