//! Comprehensive test suite for P0, P1, and P2 roadmap items.
//!
//! P0:
//! - Opcode 0x25: Login Complete / Map Loaded (in_world = true, appear broadcast, entity sync)
//! - Opcode 0x08: Stat Allocation (u16 LE target_val <= current + 1, point decremented)
//! - Opcode 0x05: Move routing (0x05 | 0x06 => handle_move)
//!
//! P1:
//! - Pet width: Mount horse (0x0F sub 4) & Summon (0x13 sub 1) allow 2-byte payload
//! - Battle control 0x0B: Spectate & Jam under sub 2, ground parsing, sub 8 direct PvP
//! - Shop image (0x17 sub 30) and Reborn colors 2x u32 LE (0x17 sub 46)
//!
//! P2:
//! - Chat 0x02: sub 1 (world), sub 4 (gm broadcast), sub 6 (army)
//! - Skills 0x1C: sub 2 multi-entry, sub 5 rb2; Pet reborn 0x2C: sub 2 Addskill4Pet
//! - Inventory 0x17: sub 14 (craft), 17/18 (pet equip), 20 (battle item), 36/37 (aux bag), 45 (warp)

use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Npc;
use ts_dream::protocol::encoder;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::{battle, chat, inventory, pet_actions, shops, skills, stats};
use ts_dream::server::session::{Conn, InventoryItem, PetState};

fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a ts_dream::battle::service::BattleService,
    out: &'a mut HandleOutcome,
    opcode: u8,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

// ==========================================
// P0 Tests
// ==========================================

#[tokio::test]
async fn test_p0_opcode_0x25_login_complete() {
    let mut conn = Conn::default();
    conn.session.id = 12345;
    conn.session.map_id = 10001;
    conn.session.map_x = 500;
    conn.session.map_y = 600;
    conn.session.in_world = false;

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();

    // Frame: [F4 44] [len=2] [0x25] [0x01]
    let decoded = [0xF4, 0x44, 0x02, 0x00, 0x25, 0x01];
    let out = dispatch(&mut conn, &decoded, &data, &service, &env).await;

    assert!(conn.session.in_world, "in_world should be marked true on 0x25 sub 1");
    // Should broadcast player appearance
    assert_eq!(out.map_broadcast.len(), 1);
    assert_eq!(out.map_broadcast[0].subject, 12345);
    // Outgoing should include server name frame
    assert!(out.outgoing.iter().any(|f| f.frame.contains("TS Online") || f.frame.starts_with("F44409002709") || f.frame.contains("2709")));

    // Sub 2 (invalid) should not mark in_world
    let mut conn2 = Conn::default();
    conn2.session.id = 12345;
    conn2.session.in_world = false;
    let decoded_sub2 = [0xF4, 0x44, 0x02, 0x00, 0x25, 0x02];
    dispatch(&mut conn2, &decoded_sub2, &data, &service, &env).await;
    assert!(!conn2.session.in_world);

    // Unauthenticated (id == 0) should not mark in_world
    let mut conn3 = Conn::default();
    conn3.session.id = 0;
    conn3.session.in_world = false;
    let decoded_sub1 = [0xF4, 0x44, 0x02, 0x00, 0x25, 0x01];
    dispatch(&mut conn3, &decoded_sub1, &data, &service, &env).await;
    assert!(!conn3.session.in_world);
}

#[tokio::test]
async fn test_p0_opcode_0x08_stat_allocation() {
    let mut conn = Conn::default();
    conn.session.id = 100;
    conn.session.point = 5;
    conn.session.int1 = 20;

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // Wire payload: [data[2], data[3], data[4] = stat_id, data[5] = target_lo, data[6] = target_hi]
    // payload[2] = 27 (Int), payload[3..5] = 21 (u16 LE)
    let payload = [0x00, 0x00, 27, 21, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x08, 1, &payload);
    stats::handle_stat_allocation(&mut ctx).await;

    assert_eq!(conn.session.int1, 21, "Int should be incremented by 1");
    assert_eq!(conn.session.point, 4, "Point should be decremented by 1");
    assert_eq!(out.outgoing.len(), 2); // point update + int update

    // Try allocating with target_val > current + 1 (should be rejected)
    let mut out2 = HandleOutcome::default();
    let payload_invalid = [0x00, 0x00, 27, 25, 0]; // 25 > 21 + 1
    let mut ctx2 = test_ctx(&mut conn, &data, &service, &mut out2, 0x08, 1, &payload_invalid);
    stats::handle_stat_allocation(&mut ctx2).await;
    assert_eq!(conn.session.int1, 21, "Int should remain unchanged when target_val is too high");
    assert_eq!(conn.session.point, 4, "Point should remain unchanged");

    // Zero points: rejected
    conn.session.point = 0;
    let mut out3 = HandleOutcome::default();
    let payload_valid = [0x00, 0x00, 27, 22, 0];
    let mut ctx3 = test_ctx(&mut conn, &data, &service, &mut out3, 0x08, 1, &payload_valid);
    stats::handle_stat_allocation(&mut ctx3).await;
    assert_eq!(conn.session.int1, 21, "Cannot allocate with 0 points");
}

#[tokio::test]
async fn test_p0_opcode_0x05_move() {
    let mut conn = Conn::default();
    conn.session.id = 1;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();

    // Opcode 0x05 sub 1: dir = 3, x = 120, y = 340
    let decoded = [0xF4, 0x44, 0x07, 0x00, 0x05, 0x01, 3, 120, 0, 84, 1];
    let out = dispatch(&mut conn, &decoded, &data, &service, &env).await;

    assert_eq!(conn.session.gocnhin, 3);
    assert_eq!(conn.session.map_x, 120);
    assert_eq!(conn.session.map_y, 340);
    assert_eq!(out.map_broadcast.len(), 1);
}

// ==========================================
// P1 Tests
// ==========================================

#[tokio::test]
async fn test_p1_pet_mount_width_2_bytes() {
    let mut conn = Conn::default();
    conn.session.id = 50;
    let mut pet = PetState::default();
    pet.id = 18005;
    pet.stt = 1;
    conn.session.pets.push(pet);

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    let pet_id: u16 = 18005;
    let payload = [ (pet_id & 0xFF) as u8, (pet_id >> 8) as u8 ];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0F, 4, &payload);
    pet_actions::handle_pet_actions(&mut ctx).await;

    assert_eq!(conn.session.horse_pet_id, 18005, "Horse pet id should be set with 2-byte payload");
    assert!(!out.outgoing.is_empty());
}

#[tokio::test]
async fn test_p1_pet_summon_width_2_bytes() {
    let mut conn = Conn::default();
    conn.session.id = 50;
    let mut pet = PetState::default();
    pet.id = 1200;
    pet.stt = 2;
    conn.session.pets.push(pet);

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // 2-byte payload
    let pet_id: u16 = 1200;
    let payload = [ (pet_id & 0xFF) as u8, (pet_id >> 8) as u8 ];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x13, 1, &payload);
    pet_actions::handle_pet_summon(&mut ctx).await;

    assert_eq!(conn.session.active_pet_stt, 2, "Active pet stt should be updated with 2-byte payload");
}

#[tokio::test]
async fn test_p1_battle_control_inner_subs_and_sub_8() {
    let mut conn = Conn::default();
    conn.session.id = 10;
    conn.session.pk = 1;

    let mut data = GameData::default();
    data.npcs.insert(1001, Npc {
        id: 1001,
        name: b"Test NPC".to_vec(),
        lv: 10,
        ..Default::default()
    });

    let service = ts_dream::battle::service::BattleService::new(std::sync::Arc::new(data.clone()));
    let mut out = HandleOutcome::default();

    // 1. Attack NPC (sub 2, inner 3, npc_id = 1001, ground = 250)
    // payload: [inner=3, npc_id (4B), ground (2B)]
    let mut payload = vec![3];
    payload.extend_from_slice(&1001u32.to_le_bytes());
    payload.extend_from_slice(&250u16.to_le_bytes());
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 2, &payload);
    battle::handle_battle(&mut ctx);
    assert_ne!(conn.session.battle_id, 0, "NPC battle should start");

    // 2. Sub 8 direct PvP
    let mut conn2 = Conn::default();
    conn2.session.id = 20;
    let mut out2 = HandleOutcome::default();
    let mut payload8 = vec![0];
    payload8.extend_from_slice(&99u32.to_le_bytes());
    let mut ctx2 = test_ctx(&mut conn2, &data, &service, &mut out2, 0x0B, 8, &payload8);
    battle::handle_battle(&mut ctx2);
    // target 99 is not online, so start_pk_battle safely returns 0
    assert_eq!(conn2.session.battle_id, 0);
}

#[tokio::test]
async fn test_p1_shop_image_and_reborn_colors() {
    // Test Shop Image (0x17 sub 30)
    let mut conn = Conn::default();
    conn.session.id = 1;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 1001,
        count: 1,
        ..Default::default()
    });

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // Payload: name_len(1), name("My Shop"), image(5), slot(1), price(100u32)
    let name = b"MyShop";
    let mut payload = vec![name.len() as u8];
    payload.extend_from_slice(name);
    payload.push(5); // image = 5
    payload.push(1); // slot 1
    payload.extend_from_slice(&100u32.to_le_bytes());

    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x17, 30, &payload);
    shops::handle_player_shop(&mut ctx).await;

    assert!(conn.session.shop.active);
    assert_eq!(conn.session.shop.image, 5, "Shop image should be parsed as 5");
    assert!(out.outgoing.iter().any(|f| f.frame.contains("171E")));

    // Test Reborn 2x u32 LE (0x17 sub 46)
    let mut conn_rb = Conn::default();
    conn_rb.session.id = 2;
    conn_rb.session.level = 120;
    let mut out_rb = HandleOutcome::default();

    // Payload: hair(1B), col1(4B LE), col2(4B LE)
    let col1: u32 = 0x11223344;
    let col2: u32 = 0x55667788;
    let mut payload_rb = vec![3]; // hair = 3
    payload_rb.extend_from_slice(&col1.to_le_bytes());
    payload_rb.extend_from_slice(&col2.to_le_bytes());

    let mut ctx_rb = test_ctx(&mut conn_rb, &data, &service, &mut out_rb, 0x17, 46, &payload_rb);
    inventory::handle_inventory(&mut ctx_rb).await;

    assert_eq!(conn_rb.session.hair, 3);
    assert_eq!(conn_rb.session.color, format!("{}{}", encoder::le32(col1), encoder::le32(col2)));
}

// ==========================================
// P2 Tests
// ==========================================

#[tokio::test]
async fn test_p2_chat_subcodes() {
    let mut conn = Conn::default();
    conn.session.id = 123;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();

    // Sub 1: All chat
    let mut out1 = HandleOutcome::default();
    let mut ctx1 = test_ctx(&mut conn, &data, &service, &mut out1, 0x02, 1, b"Hello World");
    chat::handle_chat(&mut ctx1).await;
    assert_eq!(out1.outgoing.len(), 1);
    assert!(out1.outgoing[0].frame.starts_with("F444"));

    // Sub 4: GM broadcast
    let mut out4 = HandleOutcome::default();
    let mut ctx4 = test_ctx(&mut conn, &data, &service, &mut out4, 0x02, 4, b"GM Announcement");
    chat::handle_chat(&mut ctx4).await;
    assert_eq!(out4.outgoing.len(), 1);

    // Sub 6: Army chat
    let mut out6 = HandleOutcome::default();
    let mut ctx6 = test_ctx(&mut conn, &data, &service, &mut out6, 0x02, 6, b"Army message");
    chat::handle_chat(&mut ctx6).await;
    assert_eq!(out6.outgoing.len(), 1);
}

#[tokio::test]
async fn test_p2_skills_and_reborn() {
    let mut conn = Conn::default();
    conn.session.id = 10;
    let mut pet = PetState::default();
    pet.id = 1001;
    pet.stt = 1;
    pet.skill_point = 10;
    pet.skills[0] = (10001, 1);
    pet.skills[1] = (10002, 1);
    conn.session.pets.push(pet);

    let mut data = GameData::default();
    data.skills.insert(10001, ts_dream::data::tables::Skill { id: 10001, lv_max: 5, ..Default::default() });
    data.skills.insert(10002, ts_dream::data::tables::Skill { id: 10002, lv_max: 5, ..Default::default() });
    data.npcs.insert(1001, Npc {
        id: 1001,
        skill: [10001, 10002, 0, 10004],
        ..Default::default()
    });

    let service = ts_dream::battle::service::BattleService::default();

    // 0x1C sub 2: multi-entry pet skill upgrade (stt 1, skill 10001 to lv 2, skill 10002 to lv 3)
    let mut out1 = HandleOutcome::default();
    let mut payload1 = vec![1]; // stt = 1
    payload1.extend_from_slice(&10001u16.to_le_bytes());
    payload1.push(2);
    payload1.extend_from_slice(&10002u16.to_le_bytes());
    payload1.push(3);

    let mut ctx1 = test_ctx(&mut conn, &data, &service, &mut out1, 0x1C, 2, &payload1);
    skills::handle_skills(&mut ctx1).await;

    assert_eq!(conn.session.pets[0].skills[0].1, 2);
    assert_eq!(conn.session.pets[0].skills[1].1, 3);
    assert_eq!(out1.outgoing.len(), 2, "Both skill upgrades should emit response frames");

    // 0x2C sub 2: Addskill4Pet
    let mut out2 = HandleOutcome::default();
    let payload2 = [1]; // stt = 1
    let mut ctx2 = test_ctx(&mut conn, &data, &service, &mut out2, 0x2C, 2, &payload2);
    skills::handle_pet_reborn(&mut ctx2).await;

    assert_eq!(conn.session.pets[0].skills[3].0, 10004, "4th skill should be added to pet");
    assert!(out2.outgoing.iter().any(|f| f.frame == "F44402002C01"));

    // Edge case: 0x1C sub 2 with trailing unaligned bytes (e.g., 2 extra bytes)
    let mut out3 = HandleOutcome::default();
    let mut payload_trailing = vec![1]; // stt = 1
    payload_trailing.extend_from_slice(&10001u16.to_le_bytes());
    payload_trailing.push(3); // target lv 3
    payload_trailing.extend_from_slice(&[0xAA, 0xBB]); // unaligned 2 trailing bytes
    let mut ctx3 = test_ctx(&mut conn, &data, &service, &mut out3, 0x1C, 2, &payload_trailing);
    skills::handle_skills(&mut ctx3).await;
    assert_eq!(conn.session.pets[0].skills[0].1, 3);
    assert_eq!(out3.outgoing.len(), 1);

    // Edge case: 0x2C with invalid subcode (e.g. sub 8 or sub 0) should be ignored
    let mut out4 = HandleOutcome::default();
    let mut ctx4 = test_ctx(&mut conn, &data, &service, &mut out4, 0x2C, 8, &[1]);
    skills::handle_pet_reborn(&mut ctx4).await;
    assert!(out4.outgoing.is_empty());
}

#[tokio::test]
async fn test_p2_inventory_subcodes() {
    let mut conn = Conn::default();
    conn.session.id = 1;
    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();

    // Sub 14: Craft stub
    let mut out14 = HandleOutcome::default();
    let mut ctx14 = test_ctx(&mut conn, &data, &service, &mut out14, 0x17, 14, &[1, 2]);
    inventory::handle_inventory(&mut ctx14).await;
    assert_eq!(out14.outgoing[0].frame, "F4440200170E");

    // Sub 20: Battle item stub
    let mut out20 = HandleOutcome::default();
    let mut ctx20 = test_ctx(&mut conn, &data, &service, &mut out20, 0x17, 20, &[1, 2, 3]);
    inventory::handle_inventory(&mut ctx20).await;
    assert_eq!(out20.outgoing[0].frame, "F44402001714");

    // Sub 37: Auxiliary bag move
    let mut out37 = HandleOutcome::default();
    let mut ctx37 = test_ctx(&mut conn, &data, &service, &mut out37, 0x17, 37, &[1, 2, 5]);
    inventory::handle_inventory(&mut ctx37).await;
    assert!(out37.outgoing[0].frame.starts_with("F44405001725"));

    // Sub 45: Warp item (to 59401, 402, 775)
    let mut out45 = HandleOutcome::default();
    let mut ctx45 = test_ctx(&mut conn, &data, &service, &mut out45, 0x17, 45, &[]);
    inventory::handle_inventory(&mut ctx45).await;
    assert_eq!(conn.session.map_id, 59401);
    assert_eq!(conn.session.map_x, 402);
    assert_eq!(conn.session.map_y, 775);
    assert!(out45.outgoing.iter().any(|f| f.frame.contains("000C")));
}
