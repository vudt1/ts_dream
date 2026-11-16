//! Integration tests for Opcode `0x18` (Quest Ledger `Mark.dat` 200 slots,
//! `quest_dont` bitmask `1..=2400`, actor state `0x18 08`, and `C→S 0x18` rejection).

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::protocol::codecs::quest_sync::QuestSyncCodec;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::quest_sync;
use ts_dream::server::session::{Conn, Session};

/// Frames whose main opcode (byte 4 of `F4 44 lenLE <op>`) is `0x18`.
fn op18_frames(frames: &[String]) -> Vec<&str> {
    frames
        .iter()
        .filter(|frame| {
            hex::decode(frame)
                .map(|bytes| bytes.len() >= 5 && bytes[4] == 0x18)
                .unwrap_or(false)
        })
        .map(String::as_str)
        .collect()
}

// ============================================================================
// Wire codec parity for Opcode 0x18
// ============================================================================

#[test]
fn test_research_builder_hexes_match_wire() {
    assert_eq!(QuestSyncCodec::build_quest_full_hex(), "F44402001803");
    assert_eq!(
        QuestSyncCodec::build_item_clear_hex(10001),
        "F444040018041127"
    );

    // Sub 0x05: quest-dont single (supports bit_id 1..=2400, e.g. 1673 = 0x0689 -> 8906)
    assert_eq!(
        QuestSyncCodec::build_dont_single_hex(101, 1),
        "F44405001805650001"
    );
    assert_eq!(
        QuestSyncCodec::build_dont_single_hex(1673, 1),
        "F44405001805890601"
    );

    // Sub 0x07: quest-dont bulk ([byte_idx: u16 LE][bitmask: u8])
    assert_eq!(
        QuestSyncCodec::build_dont_bulk_hex(&[
            ts_dream::protocol::codecs::quest_sync::QuestDontEntry {
                mark: 13,
                flag: 0x10,
            },
            ts_dream::protocol::codecs::quest_sync::QuestDontEntry {
                mark: 210,
                flag: 0x01,
            },
        ]),
        "F444080018070D0010D20001"
    );

    // Sub 0x06: quest task, single & bulk (4 bytes per entry)
    assert_eq!(
        QuestSyncCodec::build_task_entry_hex(1, 10801, 3),
        "F4440600180601312A03"
    );
    assert_eq!(
        QuestSyncCodec::build_task_bulk_hex(&[
            ts_dream::protocol::codecs::quest_sync::QuestTaskEntry {
                slot: 1,
                quest_id: 10801,
                mark_step: 3,
            },
            ts_dream::protocol::codecs::quest_sync::QuestTaskEntry {
                slot: 2,
                quest_id: 10802,
                mark_step: 1,
            },
        ]),
        "F4440A00180601312A0302322A01"
    );

    // Sub 0x08: actor state flag (Ghost=1, God=2)
    assert_eq!(
        QuestSyncCodec::build_state_flag_hex(1001, 1, 0),
        "F44409001808E9030000010000"
    );
}

// ============================================================================
// Quest-dont flags (Sub 0x05, 1..=2400) and actor state flag (Sub 0x08)
// ============================================================================

#[test]
fn test_quest_dont_marks_1_to_2400_bounds() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // Valid bit_id <= 300
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 101, 1));
    assert!(conn.session.quest_dont.contains(&101));
    assert_eq!(out.outgoing[0].frame, "F44405001805650001");

    // Valid bit_id > 300 (e.g. 1673 = 0x0689 LE -> 8906) and upper bound 2400 (0x0960 -> 6009)
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 1673, 1));
    assert!(conn.session.quest_dont.contains(&1673));
    assert_eq!(out.outgoing[1].frame, "F44405001805890601");

    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 2400, 1));
    assert!(conn.session.quest_dont.contains(&2400));
    assert_eq!(out.outgoing[2].frame, "F44405001805600901");

    // Out-of-range: 0 and 2401 must be rejected before the wire
    assert!(!quest_sync::set_quest_dont(&mut conn.session, &mut out, 0, 1));
    assert!(!quest_sync::set_quest_dont(&mut conn.session, &mut out, 2401, 1));
    assert_eq!(
        out.outgoing.len(),
        3,
        "out-of-range bit_ids must not reach the wire"
    );

    // Clearing emits flag 0 and drops the bit_id
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 1673, 0));
    assert!(!conn.session.quest_dont.contains(&1673));
    assert_eq!(out.outgoing[3].frame, "F44405001805890600");
}

#[test]
fn test_actor_state_flag_frame() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();
    conn.session.id = 1001; // 0x000003E9 -> LE E9030000

    quest_sync::send_actor_state_flag(&conn.session, &mut out, 1, 0);
    assert_eq!(out.outgoing[0].frame, "F44409001808E9030000010000");
}

// ============================================================================
// Quest Ledger (Sub 0x06): strict 1..=200 slots, 200-slot overflow -> 0x18 03,
// and adjacent slot preservation
// ============================================================================

#[test]
fn test_quest_ledger_slots_1_to_200_and_overflow_refusal() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // Fill slots 1..=200
    for i in 1u16..=200 {
        let qid = 10000 + i;
        assert!(
            quest_sync::set_quest_task(&mut conn.session, &mut out, qid, 1),
            "quest {qid} must fit in slot {i}"
        );
        assert_eq!(conn.session.quest_tasks[&qid], (i as u8, 1));
    }
    assert_eq!(conn.session.quest_tasks.len(), 200);

    // Updating an existing quest keeps its slot and preserves adjacent slots
    let before_len = out.outgoing.len();
    assert!(quest_sync::set_quest_task(&mut conn.session, &mut out, 10002, 5));
    assert_eq!(conn.session.quest_tasks[&10001], (1, 1), "adjacent slot 1 must stay intact");
    assert_eq!(conn.session.quest_tasks[&10002], (2, 5));
    assert_eq!(conn.session.quest_tasks[&10003], (3, 1), "adjacent slot 3 must stay intact");
    assert_eq!(out.outgoing.len(), before_len + 1);

    // Negative test: adding a 201st quest must fail All-or-Nothing and emit S->C 0x18 03
    let before_overflow = out.outgoing.len();
    assert!(!quest_sync::set_quest_task(&mut conn.session, &mut out, 20001, 1));
    assert_eq!(conn.session.quest_tasks.len(), 200, "ledger must not exceed 200");
    assert!(!conn.session.quest_tasks.contains_key(&20001));
    assert_eq!(
        out.outgoing[before_overflow].frame,
        "F44402001803",
        "overflow must emit S->C 0x18 03"
    );
    assert_eq!(out.outgoing.len(), before_overflow + 1);

    // Removing slot 2 frees slot 2 without disturbing slot 1 or slot 3; next quest reuses slot 2
    assert!(quest_sync::remove_quest_task(&mut conn.session, &mut out, 10002));
    assert_eq!(conn.session.quest_tasks[&10001], (1, 1));
    assert_eq!(conn.session.quest_tasks[&10003], (3, 1));
    assert!(quest_sync::set_quest_task(&mut conn.session, &mut out, 20001, 2));
    assert_eq!(conn.session.quest_tasks[&20001], (2, 2));

    // quest_id 0 is rejected
    let before_zero = out.outgoing.len();
    assert!(!quest_sync::set_quest_task(&mut conn.session, &mut out, 0, 1));
    assert_eq!(out.outgoing.len(), before_zero);
}

#[test]
fn test_upsert_quest_task_slot_never_assigns_slot_zero() {
    let mut s = Session::new();
    // Even if a legacy entry had slot 0, upsert_quest_task_slot repairs it to 1..=200
    s.quest_tasks.insert(218, (0, 1));
    let slot = quest_sync::upsert_quest_task_slot(&mut s, 218, 2).expect("must allocate valid slot");
    assert!((1..=200).contains(&slot), "slot must be in 1..=200, got {slot}");
    assert_eq!(s.quest_tasks[&218], (slot, 2));
}

// ============================================================================
// Login bulk sync (Logined1): 0x18 06 -> 0x18 07 bitmask -> 0x18 08 actor state
// ============================================================================

#[test]
fn test_login_sequence_syncs_quest_ledger_bitmask_and_actor_state() {
    // Fresh character: no quest state -> no 0x18 frames
    let fresh = Session::new();
    let frames = ts_dream::server::spawn::build_logined_sequence_session(&fresh);
    assert!(
        op18_frames(&frames).is_empty(),
        "an empty quest state must add no opcode-0x18 frames"
    );

    // Populated state with bit_id > 300 (1673) + bit_id 101 + two tasks + ghost/god actor flags
    let mut s = Session::new();
    s.id = 1001;
    s.quest_tasks.insert(10801, (1, 3));
    s.quest_tasks.insert(10802, (2, 1));
    // bit_id 101 -> byte_idx = (100 / 8) + 1 = 13 (0x000D -> 0D00), bit = 100 % 8 = 4 -> 0x10
    s.quest_dont.insert(101);
    // bit_id 1673 -> byte_idx = (1672 / 8) + 1 = 210 (0x00D2 -> D200), bit = 1672 % 8 = 0 -> 0x01
    s.quest_dont.insert(1673);
    s.ghost = 1;
    s.god = 5;

    let frames = ts_dream::server::spawn::build_logined_sequence_session(&s);
    assert_eq!(
        op18_frames(&frames),
        vec![
            // 0x18 06 bulk: 2 entries x 4 bytes (slot order 1, 2)
            "F4440A00180601312A0302322A01",
            // 0x18 07 bulk: byte 13 mask 0x10, byte 210 mask 0x01
            "F444080018070D0010D20001",
            // 0x18 08: ghost=1, god=1
            "F44409001808E9030000010001",
            "F44409001808E9030000020001",
        ],
        "login must sync Quest Ledger (0x18 06), quest_dont bitmask (0x18 07), and actor state (0x18 08)"
    );
}

// ============================================================================
// Reject C->S 0x18 01/02 (TMissionForm debug packets) for all accounts incl. GM
// ============================================================================

#[tokio::test]
async fn test_dispatch_rejects_client_to_server_0x18_even_for_gm() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    for gm_level in [0i32, 1i32] {
        let mut conn = Conn::default();
        conn.session.id = 9001;
        conn.session.gm_level = gm_level;
        conn.session.in_world = true;
        conn.session.quest_tasks.insert(10801, (1, 3));
        conn.session.quest_dont.insert(101);

        // C->S 0x18 01 (add) and C->S 0x18 02 (remove)
        let c2s_add = [0xF4, 0x44, 0x05, 0x00, 0x18, 0x01, 0x11, 0x27, 0x02];
        let c2s_rem = [0xF4, 0x44, 0x05, 0x00, 0x18, 0x02, 0x11, 0x27, 0x01];

        let out_add = dispatch(&mut conn, &c2s_add, &data, &service, &env).await;
        assert!(
            out_add.outgoing.is_empty(),
            "C->S 0x18 01 must emit no frames (gm_level={gm_level})"
        );
        let out_rem = dispatch(&mut conn, &c2s_rem, &data, &service, &env).await;
        assert!(
            out_rem.outgoing.is_empty(),
            "C->S 0x18 02 must emit no frames (gm_level={gm_level})"
        );

        // Session quest state must remain untouched
        assert_eq!(conn.session.quest_tasks.len(), 1);
        assert_eq!(conn.session.quest_tasks[&10801], (1, 3));
        assert!(conn.session.quest_dont.contains(&101));
    }
}
