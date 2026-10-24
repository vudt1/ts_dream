//! Ticket 07 — `0F 08` pet stat record layout + post-battle pet EXP push.
//!
//! Client reader `007a5010` (HANDOFF §6, G9) parses each `0F 08` record by a
//! fixed offset table and advances by `nameLen + 0x63`, so a record must be
//! exactly `99 + name.len()` bytes. The old Rust builder used `le32(id)` and a
//! 12-byte tail, shifting every field by +2 bytes and every following record by
//! a growing delta. These tests pin the corrected offsets and the `apply_pet_exp`
//! push (`Out::PetExp` → owner receives `0F 08`).

use ts_dream::battle::runner::{BattleAction, BattleCommand};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, Skill};
use ts_dream::server::session::{lock_online_sessions, InventoryItem, PetState, Session};

/// Strip the 4-byte frame header and return the content (payload from the main
/// opcode).
fn content_of(frame: &str) -> Vec<u8> {
    let bytes = hex::decode(frame).expect("frame must be valid hex");
    assert_eq!(&bytes[0..2], &[0xF4, 0x44], "TS magic header");
    let len = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    assert_eq!(len, bytes.len() - 4, "LEN counts everything after the header");
    bytes[4..].to_vec()
}

fn le16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn le32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Two active pets (stt 1 and 2) plus the six gear slots of pet 1 (`stt*10 + i`).
fn session_with_two_pets() -> Session {
    let gear = (1..=6u8)
        .map(|i| InventoryItem {
            slot: 10 + i, // pet 1 weapon..accessory
            id: 5000 + u16::from(i),
            count: 1,
            long_val: 2,
            giatri_long: 3,
            khang: 4,
            texp: 1100 + u32::from(i),
            ..Default::default()
        })
        .collect();

    Session {
        id: 1,
        pets: vec![
            PetState {
                stt: 1,
                id: 0x1234,
                name: b"Pikachu".to_vec(),
                level: 42,
                hp: 111,
                sp: 222,
                int1: 333,
                atk: 444,
                def: 555,
                agi: 666,
                hpx: 777,
                spx: 888,
                fai: 9,
                quest: 7,
                skill_point: 1234,
                texp: 123456,
                skills: [(101, 5), (102, 6), (103, 7), (104, 8)],
                ..Default::default()
            },
            PetState {
                stt: 2,
                id: 0x5678,
                name: b"Raichu".to_vec(),
                level: 9,
                texp: 7,
                ..Default::default()
            },
        ],
        trangbi: gear,
        ..Default::default()
    }
}

/// Every field of the first `0F 08` record must land on the client's offsets and
/// the record must be `99 + name.len()` bytes.
#[test]
fn pet_stat_record_matches_client_offsets() {
    let s = session_with_two_pets();
    let frames = ts_dream::server::spawn::pet_summary(&s);
    let content = content_of(&frames[0]);
    assert_eq!(&content[0..2], &[0x0F, 0x08]);

    let n = b"Pikachu".len();
    let r = &content[2..2 + 99 + n];

    assert_eq!(r.len(), 99 + n, "record length must be 99 + nameLen");
    assert_eq!(r[0x00], 1, "stt at +0x00");
    assert_eq!(le16(r, 0x01), 0x1234, "id is le16 at +0x01 (not le32)");
    assert_eq!(le32(r, 0x03), 123456, "texp is le32 at +0x03");
    assert_eq!(r[0x07], 42, "level at +0x07");
    assert_eq!(le16(r, 0x08), 111, "hp at +0x08");
    assert_eq!(le16(r, 0x0A), 222, "sp at +0x0A");
    assert_eq!(le16(r, 0x0C), 333, "int1 at +0x0C");
    assert_eq!(le16(r, 0x0E), 444, "atk at +0x0E");
    assert_eq!(le16(r, 0x10), 555, "def at +0x10");
    assert_eq!(le16(r, 0x12), 666, "agi at +0x12");
    assert_eq!(le16(r, 0x14), 777, "hpx at +0x14");
    assert_eq!(le16(r, 0x16), 888, "spx at +0x16");
    assert_eq!(r[0x18], 0, "reserved byte at +0x18");
    assert_eq!(r[0x19], 9, "fai at +0x19");
    assert_eq!(r[0x1A], 7, "quest at +0x1A");
    assert_eq!(le16(r, 0x1B), 1234, "skill point at +0x1B");
    assert_eq!(r[0x1D], n as u8, "nameLen at +0x1D");
    assert_eq!(&r[0x1E..0x1E + n], b"Pikachu", "name at +0x1E");
    assert_eq!(
        &r[0x1E + n..0x21 + n],
        &[5, 6, 7],
        "lv_skill[0..2] right after the name"
    );

    // Six 10-byte gear entries starting at +0x21+n.
    for i in 0..6usize {
        let e = &r[0x21 + n + 10 * i..0x2B + n + 10 * i];
        assert_eq!(le16(e, 0), 5001 + i as u16, "gear entry {i} id");
        assert_eq!(le32(e, 6), 1101 + i as u32, "gear entry {i} texp");
    }

    // 6-byte tail: lv_skill[3] | 00 | le16(v) | 2 reserve.
    assert_eq!(r[0x5D + n], 8, "lv_skill[3] opens the tail");
    assert_eq!(r[0x5E + n], 0, "tail byte 2");
    assert_eq!(&r[0x5F + n..0x63 + n], &[0, 0, 0, 0], "tail value + reserve");
}

/// Two records in one frame are separated by exactly `99 + len(name_1)`.
#[test]
fn pet_stat_two_pets_use_name_len_stride() {
    let s = session_with_two_pets();
    let frames = ts_dream::server::spawn::pet_summary(&s);
    let content = content_of(&frames[0]);
    assert_eq!(&content[0..2], &[0x0F, 0x08]);

    let first_len = 99 + b"Pikachu".len();
    let r2 = &content[2 + first_len..];
    assert_eq!(
        r2.len(),
        99 + b"Raichu".len(),
        "the second record starts at 99 + nameLen of the first"
    );
    assert_eq!(r2[0x00], 2, "second record stt");
    assert_eq!(le16(r2, 0x01), 0x5678, "second record id");
    assert_eq!(le32(r2, 0x03), 7, "second record texp");
}

/// Single-pet helper emits one record only (same layout).
#[test]
fn pet_status_single_emits_one_record_frame() {
    let s = session_with_two_pets();
    let frames = ts_dream::server::spawn::pet_status_single(&s, 2);
    assert!(!frames.is_empty());
    let content = content_of(&frames[0]);
    assert_eq!(&content[0..2], &[0x0F, 0x08]);
    assert_eq!(content.len() - 2, 99 + b"Raichu".len());
}

/// GameData with a 1-HP hostile NPC and the basic attack skill.
fn data_with_npc() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"PetExpNPC".to_vec(),
            lv: 1,
            hp: 1,
            sp: 10,
            agi: 1,
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

/// `apply_pet_exp` must push a `0F 08` record to the owner carrying the new EXP
/// (the pet is faster than the player so it lands the kill and earns `per_exp`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_pet_exp_sends_0f08_to_owner() {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let mut service = BattleService::new(Arc::new(data_with_npc()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: 9002,
        map_id: 1,
        id_leader: 9002,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        active_pet_stt: 1,
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"Wolf".to_vec(),
            level: 1,
            hp: 100,
            hp_max: 100,
            sp: 50,
            sp_max: 50,
            atk: 100,  // guarantees the kill on the 1-HP NPC
            agi: 5000, // acts before the player, so the pet lands the kill
            texp: 500,
            ..Default::default()
        }],
        ..Default::default()
    }));
    let mut rx = service.register(9002, sess.clone());
    lock_online_sessions().insert(9002, sess.read().await.clone());

    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0, "seeded NPC battle must start");
    }

    // Only the player cell is commanded; the pet cell auto-attacks on timeout.
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
    let final_texp = sess
        .read()
        .await
        .pets
        .iter()
        .find(|p| p.stt == 1)
        .expect("pet still present")
        .texp;
    lock_online_sessions().remove(&9002);

    let pet_frame = frames
        .iter()
        .find(|f| f.starts_with("F444") && &f[8..12] == "0F08")
        .expect("owner must receive a 0F 08 pet stat frame");
    let content = content_of(pet_frame);
    assert_eq!(&content[0..2], &[0x0F, 0x08]);

    let n = b"Wolf".len();
    assert_eq!(content.len() - 2, 99 + n, "one corrected record");
    let rec = &content[2..];
    assert_eq!(rec[0x00], 1);
    assert_eq!(le16(rec, 0x01), 11001);
    assert_eq!(
        le32(rec, 0x03),
        final_texp,
        "the frame EXP must equal the session's credited EXP"
    );
    assert!(final_texp > 500, "the battle must have granted pet EXP");
}
