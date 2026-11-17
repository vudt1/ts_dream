//! Ticket 04 — the single `17 06` add-item builder + post-battle drop delivery.
//!
//! Evidence: client reader `FUN_0076b458` (`HANDOFF-client-reverse.md` §4, G4)
//! reads `17 06 | id:u16 | count:u16 | doben | long | giatri+100 | khang |
//! texp:u32`, so the payload after `17 06` is 12 bytes and `LEN = 0x0E`.
//! The Rust layout was already byte-exact; this file pins the shared builder
//! and proves a defeated NPC's drop frame actually reaches the owner.

use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::runner::{BattleAction, BattleCommand};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, Skill};
use ts_dream::server::inventory::item_added_frame;
use ts_dream::server::session::{lock_online_sessions, InventoryItem, Session};

/// Strip the 4-byte frame header and assert the `LEN` field.
fn content_of(frame: &str) -> Vec<u8> {
    let bytes = hex::decode(frame).expect("frame must be valid hex");
    assert_eq!(&bytes[0..2], &[0xF4, 0x44], "TS magic header");
    let len = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    assert_eq!(len, bytes.len() - 4, "LEN counts everything after the header");
    bytes[4..].to_vec()
}

/// Round-trip the shared builder: rebuild the frame and re-read `id`/`count`/
/// `texp` at the documented offsets with `LEN == 0x0E`.
#[test]
fn shared_builder_round_trips_id_count_texp_at_documented_offsets() {
    let item = InventoryItem {
        slot: 7,
        id: 5001,
        count: 3,
        doben: 100,
        long_val: 25,
        giatri_long: 40,
        khang: 9,
        texp: 123_456,
        ..Default::default()
    };
    let frame = item_added_frame(&item);
    let bytes = hex::decode(&frame).expect("valid hex");

    // 17 06 (2B) + 12B payload = 14B content ⇒ LEN = 0x0E.
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0x0E);

    let content = content_of(&frame);
    assert_eq!(content.len(), 14, "content must be exactly 14 bytes");
    assert_eq!(&content[0..2], &[0x17, 0x06]);

    let id = u16::from_le_bytes([content[2], content[3]]);
    let count = u16::from_le_bytes([content[4], content[5]]);
    let doben = content[6];
    let long_val = content[7];
    let giatri = content[8];
    let khang = content[9];
    let texp = u32::from_le_bytes([content[10], content[11], content[12], content[13]]);

    assert_eq!(id, 5001);
    assert_eq!(count, 3);
    assert_eq!(doben, 100);
    assert_eq!(long_val, 25);
    assert_eq!(giatri, 40 + 100, "giatri_long + 100 bias");
    assert_eq!(khang, 9);
    assert_eq!(texp, 123_456);
}

/// The builder's exact bytes for a bare item — the same frame a battle drop
/// produces (id 5001, count 1, doben 100).
#[test]
fn shared_builder_matches_documented_hex() {
    let item = InventoryItem {
        id: 5001,
        count: 1,
        doben: 100,
        ..Default::default()
    };
    assert_eq!(
        item_added_frame(&item),
        "F4440E001706891301006400640000000000"
    );
}

/// The giatri byte keeps the documented `+100` bias and clamps at the wire
/// maximum (`GIA_TRI_LONG_MAX = 155` ⇒ `255`), never wrapping.
#[test]
fn shared_builder_clamps_giatri_bias() {
    let item = InventoryItem {
        id: 1,
        count: 1,
        giatri_long: 255,
        ..Default::default()
    };
    let content = content_of(&item_added_frame(&item));
    assert_eq!(content.len(), 14);
    assert_eq!(content[8], 255);
}

/// GameData with a 1-HP hostile NPC whose drop table always grants item 5001.
fn data_with_drop_npc() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"DropNPC".to_vec(),
            lv: 1,
            hp: 1,
            sp: 10,
            agi: 1,
            item: [5001; 6],
            ..Default::default()
        },
    );
    data.skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Tan Cong".to_string(),
            skill_type: 1,
            ..Default::default()
        },
    );
    data
}

/// Kill an NPC in a seeded battle and assert the owner receives the shared
/// `17 06` add-item frame carrying the correct item id.
///
/// Seed `s0 = 4` is the deterministic RNG-0 seed for which the drop band roll
/// grants the NPC's item (verified by brute force against the seeded runner).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn killed_npc_drop_reaches_owner_as_17_06() {
    let mut service = BattleService::new(Arc::new(data_with_drop_npc()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: 9003,
        map_id: 1,
        id_leader: 9003,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        ..Default::default()
    }));
    let mut rx = service.register(9003, sess.clone());
    lock_online_sessions().insert(9003, sess.read().await.clone());

    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0, "seeded NPC battle must start");
    }

    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    // Wait for the battle to finish (the 1-HP NPC dies on the first turn).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() <= deadline,
            "battle did not finish in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let mut drop_frame: Option<String> = None;
    while let Ok(frame) = rx.try_recv() {
        if frame.starts_with("F4440E001706") {
            drop_frame = Some(frame);
        }
    }
    lock_online_sessions().remove(&9003);

    let frame = drop_frame.expect("owner must receive the 17 06 add-item frame");
    let content = content_of(&frame);
    assert_eq!(content.len(), 14, "drop frame content must be 14 bytes");
    assert_eq!(&content[0..2], &[0x17, 0x06]);
    assert_eq!(
        u16::from_le_bytes([content[2], content[3]]),
        5001,
        "drop frame must carry the NPC's dropped item id"
    );
    assert_eq!(u16::from_le_bytes([content[4], content[5]]), 1);
}

/// When player's bag has all 25 slots filled with non-stackable items,
/// the dropped item is rejected: no 17 06 is sent, and a red system alert is delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn killed_npc_drop_blocked_when_bag_full_receives_sys_msg() {
    let mut service = BattleService::new(Arc::new(data_with_drop_npc()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let mut homdo = Vec::new();
    for slot in 1..=25 {
        homdo.push(InventoryItem {
            slot,
            id: 9000 + slot as u16, // Different IDs so cannot stack
            count: 1,
            ..Default::default()
        });
    }

    let sess = Arc::new(RwLock::new(Session {
        id: 9004,
        map_id: 1,
        id_leader: 9004,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        homdo,
        ..Default::default()
    }));
    let mut rx = service.register(9004, sess.clone());
    lock_online_sessions().insert(9004, sess.read().await.clone());

    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0, "seeded NPC battle must start");
    }

    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() <= deadline,
            "battle did not finish in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let mut frames = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames.push(frame);
    }
    lock_online_sessions().remove(&9004);

    let has_1706 = frames.iter().any(|f| f.starts_with("F4440E001706"));
    assert!(!has_1706, "owner must NOT receive 17 06 when bag is full; got frames: {:?}", frames);

    let has_sys_msg = frames.iter().any(|f| {
        let expected = ts_dream::server::spawn::sys_msg_frame("Hành trang đã đầy, không thể nhận vật phẩm.");
        f == &expected
    });
    assert!(has_sys_msg, "owner must receive full-bag system alert; frames: {:?}", frames);

    // Verify bag is still exactly 25 items
    let final_sess = sess.read().await;
    assert_eq!(final_sess.homdo.len(), 25);
    assert!(final_sess.homdo.iter().all(|i| i.id != 5001), "dropped item 5001 was not added");
}

/// Boundary transition test:
/// Starting with 24 items in homdo:
/// - 1st battle drop fills slot 25 successfully (17 06 received).
/// - 2nd battle drop is blocked because bag is now 25/25 (sys alert received, no 17 06).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn killed_npc_drop_at_boundary_24_to_25_accepted_then_subsequent_blocked() {
    let mut service = BattleService::new(Arc::new(data_with_drop_npc()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let mut homdo = Vec::new();
    for slot in 1..=24 {
        homdo.push(InventoryItem {
            slot,
            id: 8000 + slot as u16,
            count: 1,
            ..Default::default()
        });
    }

    let sess = Arc::new(RwLock::new(Session {
        id: 9005,
        map_id: 1,
        id_leader: 9005,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        homdo,
        ..Default::default()
    }));
    let mut rx = service.register(9005, sess.clone());
    lock_online_sessions().insert(9005, sess.read().await.clone());

    // Battle 1: 24 items -> 1st drop fills slot 25
    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0);
    }
    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(tokio::time::Instant::now() <= deadline);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let mut frames_1 = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames_1.push(frame);
    }
    let has_1706_first = frames_1.iter().any(|f| f.starts_with("F4440E001706"));
    assert!(has_1706_first, "1st drop should fill slot 25; got: {:?}", frames_1);
    assert_eq!(sess.read().await.homdo.len(), 25, "homdo must have 25 items now");

    // Battle 2: 25 items -> 2nd drop must be blocked
    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0);
    }
    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(tokio::time::Instant::now() <= deadline);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let mut frames_2 = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames_2.push(frame);
    }
    lock_online_sessions().remove(&9005);

    let has_1706_second = frames_2.iter().any(|f| f.starts_with("F4440E001706"));
    assert!(!has_1706_second, "2nd drop must NOT be received when bag is full; frames: {:?}", frames_2);
    let has_sys_msg_second = frames_2.iter().any(|f| {
        let expected = ts_dream::server::spawn::sys_msg_frame("Hành trang đã đầy, không thể nhận vật phẩm.");
        f == &expected
    });
    assert!(has_sys_msg_second, "owner must receive full-bag system alert on 2nd drop; frames: {:?}", frames_2);
    assert_eq!(sess.read().await.homdo.len(), 25);
}
