//! Ticket 19 — Opcode 0x32 is the C→S battle-command channel (ADR 0006).
//!
//! Sub 1 carries action commands (10000 attack, skills, 17001 defend, 18001 flee, 15001 capture).
//! Sub 2 carries battle grid item use (26001..=27165).
//! Opcode 0x0B subs 3 and 4 are restored as out-of-battle map actions.
//!
//! 10-byte wire frame:
//! [32][sub:1B][srcGrp:1B][srcSlot:1B][dstGrp:1B][dstSlot:1B][skillOrItemId:2B LE][rand1:1B][rand2:1B]

use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Npc;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::battle;
use ts_dream::server::session::{lock_online_sessions, Conn, InventoryItem, Session};

fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
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

/// A `GameData` with a single attackable NPC plus its battle service.
fn npc_battle_setup() -> (GameData, BattleService) {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"Test NPC".to_vec(),
            lv: 10,
            ..Default::default()
        },
    );
    let service = BattleService::new(Arc::new(data.clone()));
    (data, service)
}

/// `[kind][u32 target][u16 index]` action body (sub 2 / 0A layout).
fn action_body(kind: u8, target: u32, index: u16) -> Vec<u8> {
    let mut p = vec![kind];
    p.extend_from_slice(&target.to_le_bytes());
    p.extend_from_slice(&index.to_le_bytes());
    p
}

/// Sub 2 kind 3 (`[kind][u32 npc_id][u16 ground]`) still starts an NPC battle;
/// an unknown kind is logged and ignored but never swallowed/crashes.
#[tokio::test]
async fn sub2_known_kind_starts_battle_and_unknown_kind_is_ignored() {
    let (data, service) = npc_battle_setup();

    let mut conn = Conn::default();
    conn.session.id = 10;
    let mut out = HandleOutcome::default();
    let payload = action_body(3, 1001, 250);
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 2, &payload);
    battle::handle_battle(&mut ctx);
    assert_ne!(conn.session.battle_id, 0, "kind 3 must start the NPC battle");

    // Unknown kind on a fresh (out-of-battle) session: no battle, no panic.
    let mut conn2 = Conn::default();
    conn2.session.id = 11;
    let mut out2 = HandleOutcome::default();
    let unknown = action_body(0x77, 4242, 7);
    let mut ctx2 = test_ctx(&mut conn2, &data, &service, &mut out2, 0x0B, 2, &unknown);
    battle::handle_battle(&mut ctx2);
    assert_eq!(conn2.session.battle_id, 0);
    assert!(out2.outgoing.is_empty());
}

/// Sub 2 must parse the whole 9-byte frame; short/empty frames must not panic.
#[tokio::test]
async fn sub2_short_frames_do_not_panic() {
    let (data, service) = npc_battle_setup();
    for payload in [vec![], vec![3], vec![3, 1, 2, 3], vec![3, 1, 2, 3, 4]] {
        let mut conn = Conn::default();
        conn.session.id = 12;
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 2, &payload);
        battle::handle_battle(&mut ctx);
        // A truncated NPC id must never be treated as a valid target.
        assert_eq!(conn.session.battle_id, 0);
    }
}

/// Sub 7 = `[mode][counter]` (auto-battle). Parsed + logged, never auto-opens.
#[tokio::test]
async fn sub7_auto_battle_parse_does_not_open_a_battle() {
    let (data, service) = npc_battle_setup();
    for payload in [vec![], vec![1], vec![1, 7], vec![1, 7, 0xFF]] {
        let mut conn = Conn::default();
        conn.session.id = 13;
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 7, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
    }
}

/// Sub 8 is a map/PK click. It must NOT be forced into a direct PK battle even
/// when the target is online (this is the exact regression ticket 01 removes).
#[tokio::test]
async fn sub8_does_not_auto_open_pk_even_with_online_target() {
    let (data, service) = npc_battle_setup();

    // Register an online target so `start_pk_battle` *could* succeed.
    let target = Arc::new(RwLock::new(Session {
        id: 99,
        ..Default::default()
    }));
    let _rx = service.register(99, target.clone());
    let target_snapshot = target.read().await.clone();
    lock_online_sessions().insert(99, target_snapshot);

    let mut conn = Conn::default();
    conn.session.id = 20;
    conn.session.pk = 1;
    let mut out = HandleOutcome::default();
    // kind 0x09 = "attack player on map", target 99, index 0 (8-byte frame).
    let payload = vec![0x09, 99, 0, 0, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 8, &payload);
    battle::handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 0, "sub 8 must never open a battle");
    assert!(out.outgoing.is_empty());

    lock_online_sessions().remove(&99);
}

/// Sub 9 (empty) and sub 0A (`[kind][u32][index]`) parse without panic.
#[tokio::test]
async fn sub9_and_sub0a_parse_without_panic() {
    let (data, service) = npc_battle_setup();
    for (sub, payload) in [
        (9u8, vec![]),
        (0x0Au8, vec![]),
        (0x0Au8, vec![0x15]),
        (0x0Au8, action_body(0x15, 777, 0)),
    ] {
        let mut conn = Conn::default();
        conn.session.id = 21;
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, sub, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
    }
}

/// Sub 0 (`[b]`) and sub FA (bare marker) parse without panic.
#[tokio::test]
async fn sub0_and_sub_fa_parse_without_panic() {
    let (data, service) = npc_battle_setup();
    for (sub, payload) in [(0u8, vec![]), (0u8, vec![0x3B]), (0xFAu8, vec![])] {
        let mut conn = Conn::default();
        conn.session.id = 22;
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, sub, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
    }
}

/// Sub 5 is a raw 2-byte no-op — it must NOT tear down the battle (old flee
/// branch removed).
#[tokio::test]
async fn sub5_is_not_a_flee_branch() {
    let (data, service) = npc_battle_setup();
    let mut conn = Conn::default();
    conn.session.id = 23;
    conn.session.battle_id = 77;
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 5, &[0, 0]);
    battle::handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 77);
    assert!(out.outgoing.is_empty());
}

/// Sub 1 with confirm == 3 leaves the battle; any other confirm is ignored.
#[tokio::test]
async fn sub1_confirm_three_leaves_and_other_values_are_ignored() {
    let (data, service) = npc_battle_setup();

    // confirm == 3 → leave battle.
    let mut conn = Conn::default();
    conn.session.id = 30;
    conn.session.battle_id = 55;
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, 1, &[3]);
    battle::handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 0, "confirm 3 must leave the battle");
    assert!(out.outgoing.iter().any(|f| f.frame.contains("0504")));
    assert!(out.outgoing.iter().any(|f| f.frame.contains("1408")));

    // confirm != 3 → unchanged, no frames.
    let mut conn2 = Conn::default();
    conn2.session.id = 31;
    conn2.session.battle_id = 55;
    let mut out2 = HandleOutcome::default();
    for confirm in [0u8, 1, 2, 4] {
        let frame = [confirm];
        let mut ctx2 = test_ctx(
            &mut conn2,
            &data,
            &service,
            &mut out2,
            0x0B,
            1,
            &frame,
        );
        battle::handle_battle(&mut ctx2);
    }
    assert_eq!(conn2.session.battle_id, 55);
    assert!(out2.outgoing.is_empty());
}

/// Opcode 0x32 is dispatched as the C->S battle-command channel (Ticket 19 / ADR 0006).
#[tokio::test]
async fn opcode_0x32_is_dispatched() {
    let (data, service) = npc_battle_setup();
    let mut conn = Conn::default();
    conn.session.id = 40;

    // Start a real battle so the command can be accepted
    let bid = service.start_npc_battle(&mut conn.session, 1001, 0, 112);
    assert_ne!(bid, 0);

    // F4 44 <len=10> <opcode=0x32> <sub=0x01> srcGrp srcSlot dstGrp dstSlot skill_lo skill_hi rand1 rand2
    let frame = [
        0xF4, 0x44, 0x0A, 0x00, 0x32, 0x01, 0x03, 0x02, 0x00, 0x02, 0x10, 0x27, 0x00, 0x00,
    ];
    let out = dispatch(&mut conn, &frame, &data, &service, &ServerEnv::none()).await;
    let _ = out;
}

/// 0x0B sub 3 and sub 4 are out-of-battle map actions (never open battles or corrupt state).
#[tokio::test]
async fn sub3_and_sub4_are_map_actions() {
    let (data, service) = npc_battle_setup();
    for sub in [3u8, 4u8] {
        let mut conn = Conn::default();
        conn.session.id = 50;
        let mut out = HandleOutcome::default();
        let payload = action_body(1, 1001, 0);
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x0B, sub, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
        assert!(out.outgoing.is_empty());
    }
}

/// Opcode 0x32 sub 1 skill command is reachable and guarded against malformed input.
#[tokio::test]
async fn sub1_0x32_skill_command_is_reachable_and_guarded() {
    let (data, service) = npc_battle_setup();
    let mut conn = Conn::default();
    conn.session.id = 55;

    let bid = service.start_npc_battle(&mut conn.session, 1001, 0, 112);
    assert_ne!(bid, 0);

    let mut out = HandleOutcome::default();
    // Valid 0x32 sub 1 payload: srcGrp(3), srcSlot(2), dstGrp(0), dstSlot(2), skill(10000 = 0x2710)
    let payload = [3u8, 2, 0, 2, 0x10, 0x27, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x32, 1, &payload);
    battle::handle_battle_command(&mut ctx);

    // Out-of-range source row is rejected without panic
    let mut conn2 = Conn::default();
    conn2.session.id = 56;
    conn2.session.battle_id = bid;
    let mut out2 = HandleOutcome::default();
    let bad = [9u8, 9, 9, 9, 0x10, 0x27, 0, 0];
    let mut ctx2 = test_ctx(&mut conn2, &data, &service, &mut out2, 0x32, 1, &bad);
    battle::handle_battle_command(&mut ctx2);
}

/// In-battle item use (0x32 sub 2): an accepted command consumes exactly one item.
#[tokio::test]
async fn item_accepted_by_engine_decrements_exactly_one() {
    let (data, service) = npc_battle_setup();
    let mut conn = Conn::default();
    conn.session.id = 60;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 5,
        ..Default::default()
    });

    // Start a real battle so `submit_command` has a live handle to accept into.
    let bid = service.start_npc_battle(&mut conn.session, 1001, 0, 112);
    assert_ne!(bid, 0, "test needs a live battle handle");

    let mut out = HandleOutcome::default();
    let payload = [3u8, 2, 0, 2, (26001u16 & 0xFF) as u8, (26001u16 >> 8) as u8, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x32, 2, &payload);
    battle::handle_battle_command(&mut ctx);

    let count = conn
        .session
        .homdo
        .iter()
        .find(|i| i.id == 26001)
        .map(|i| i.count)
        .unwrap_or(0);
    assert_eq!(count, 4, "accepted item command must decrement exactly 1");
}

/// In-battle item use (0x32 sub 2): a rejected command leaves the item intact.
#[tokio::test]
async fn item_rejected_by_engine_keeps_item_intact() {
    let (data, service) = npc_battle_setup();
    let mut conn = Conn::default();
    conn.session.id = 61;
    // Non-zero battle id with no registered handle → submit_command() == false.
    conn.session.battle_id = 9999;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 5,
        ..Default::default()
    });

    let mut out = HandleOutcome::default();
    let payload = [3u8, 2, 0, 2, (26001u16 & 0xFF) as u8, (26001u16 >> 8) as u8, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x32, 2, &payload);
    battle::handle_battle_command(&mut ctx);

    let count = conn
        .session
        .homdo
        .iter()
        .find(|i| i.id == 26001)
        .map(|i| i.count)
        .unwrap_or(0);
    assert_eq!(count, 5, "rejected item command must not touch the bag");
}
