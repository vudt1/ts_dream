//! Ticket 05 — post-battle gold award frame (`1A 01`).
//!
//! `apply_gold` used to bump the session balance silently, so the client never
//! learned about battle gold. The client only knows about gold changes through
//! `F44406001A01{le32(delta)}` (handler `FUN_0072b084`,
//! `04_battle_end_rewards.md §2.7`, evidence class A): it adds `delta` to
//! `player+0x12F8` (capped at 9,999,999) and shows the "Nhận được X Tiền"
//! banner. This file pins the shared builder and proves a defeated NPC's gold
//! reward actually reaches the owner with the credited amount.

use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::packets;
use ts_dream::battle::runner::{BattleAction, BattleCommand};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, Skill};
use ts_dream::protocol::GOLD_CAP;
use ts_dream::server::session::{lock_online_sessions, Session};

/// Strip the 4-byte frame header, assert the `LEN` field, return the content
/// (payload starting at the main opcode).
fn content_of(frame: &str) -> Vec<u8> {
    let bytes = hex::decode(frame).expect("frame must be valid hex");
    assert_eq!(&bytes[0..2], &[0xF4, 0x44], "TS magic header");
    let len = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    assert_eq!(len, bytes.len() - 4, "LEN counts everything after the header");
    bytes[4..].to_vec()
}

/// Read the LE32 amount from a `1A 01` gold-gain frame.
fn gold_amount(frame: &str) -> u32 {
    let content = content_of(frame);
    assert_eq!(content.len(), 6, "1A 01 content must be exactly 6 bytes");
    assert_eq!(&content[0..2], &[0x1A, 0x01]);
    u32::from_le_bytes([content[2], content[3], content[4], content[5]])
}

/// The builder emits the exact documented bytes: `F44406001A01` + LE32(amount),
/// `LEN = 0x06`.
#[test]
fn gold_gain_matches_documented_hex() {
    assert_eq!(packets::gold_gain(1234), "F44406001A01D2040000");
    assert_eq!(packets::gold_gain(0), "F44406001A0100000000");
    // Client cap 9,999,999 = 0x98967F ⇒ LE bytes 7F 96 98 00.
    assert_eq!(packets::gold_gain(GOLD_CAP), "F44406001A017F969800");

    let decoded = hex::decode(packets::gold_gain(1234)).expect("valid hex");
    assert_eq!(u16::from_le_bytes([decoded[2], decoded[3]]), 0x06);
}

/// GameData with a 1-HP hostile NPC (level 1) and the basic attack skill.
fn data_with_gold_npc() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"GoldNPC".to_vec(),
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

/// Run one seeded Player-vs-NPC battle to completion, collecting every frame
/// the owner received. Returns `(frames, final_gold)`.
///
/// Gold reward is `npc_lv * rng.random_damage.next_range(2, 6)` with `npc_lv = 1`
/// (`runner.rs` kill loop), so it is a small deterministic amount in `2..=5`.
async fn run_seeded_battle(start_gold: u32) -> (Vec<String>, u32) {
    let mut service = BattleService::new(Arc::new(data_with_gold_npc()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: 9001,
        map_id: 1,
        id_leader: 9001,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        gold: start_gold,
        ..Default::default()
    }));
    let mut rx = service.register(9001, sess.clone());
    lock_online_sessions().insert(9001, sess.read().await.clone());

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

    let mut frames = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames.push(frame);
    }
    let final_gold = sess.read().await.gold;
    lock_online_sessions().remove(&9001);
    (frames, final_gold)
}

/// The gold path after a won battle: the owner receives `1A 01` whose amount
/// equals both the expected reward band and the delta actually credited to the
/// session balance.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn battle_gold_award_sends_1a01_with_credited_amount() {
    let (frames, final_gold) = run_seeded_battle(0).await;

    let frame = frames
        .iter()
        .find(|f| f.starts_with("F44406001A01"))
        .expect("owner must receive a 1A 01 gold frame")
        .clone();
    let amount = gold_amount(&frame);

    assert!(
        (2..=5).contains(&amount),
        "npc lv 1 gold reward must be within rng(2..6), got {amount}"
    );
    assert_eq!(
        amount, final_gold,
        "frame amount must equal the delta credited to the session"
    );
}

/// The frame value respects the client cap of 9,999,999: a session one gold
/// below the cap can only gain 1, and that is exactly what is reported.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn battle_gold_award_respects_client_cap() {
    let (frames, final_gold) = run_seeded_battle(GOLD_CAP - 1).await;

    assert_eq!(
        final_gold, GOLD_CAP,
        "session balance must be clamped to the client cap"
    );
    let frame = frames
        .iter()
        .find(|f| f.starts_with("F44406001A01"))
        .expect("owner must receive a 1A 01 gold frame")
        .clone();
    assert_eq!(
        gold_amount(&frame),
        1,
        "only the credited delta (cap - before) may be reported"
    );
}
