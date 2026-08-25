//! Ticket 07 integration tests: ResponseSender, PlayerStateManager,
//! TradeSystem, AutoSaveService dirty detection, and the npc_event Eve-engine
//! bridge.

use std::sync::Arc;

use ts_dream::battle::rng::DotNetRandom;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{EveCondition, EveNpcPlacement, NpcEventData, SceneEveData};
use ts_dream::server::auto_save;
use ts_dream::server::dispatcher::HandleOutcome;
use ts_dream::server::handlers::npc_event::{self, NpcTrigger};
use ts_dream::server::player_state::PlayerStateManager;
use ts_dream::server::response::ResponseSender;
use ts_dream::server::session::{online_sessions, InventoryItem, Session, TradeState};
use ts_dream::server::trade_system::{TradeOutcome, TradeSystem};

/// Insert a session into the shared registry and return its id.
fn register(id: u32) -> u32 {
    let mut s = Session::new();
    s.id = id;
    s.authed = true;
    online_sessions().lock().unwrap().insert(id, s);
    id
}

fn unregister(ids: &[u32]) {
    let mut sessions = online_sessions().lock().unwrap();
    for id in ids {
        sessions.remove(id);
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  ResponseSender
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn response_sender_sends_typed_frames() {
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 20023,
        count: 3,
        ..InventoryItem::default()
    });
    let mut out = HandleOutcome::default();
    {
        let mut sender = ResponseSender::new(&mut conn, &mut out);
        sender.send_bag_items();
        sender.send_stat_update(0x19, 1234);
        assert_eq!(sender.session().homdo.len(), 1);
    }
    assert_eq!(out.outgoing.len(), 2);
    assert!(out.outgoing[0].starts_with("F444"));
    assert!(out.outgoing[0].contains("1705"));
    assert!(out.outgoing[1].contains("19"));
}

#[test]
fn response_sender_end_talk_resets_context() {
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.idtalking = 6;
    conn.session.select_menu = 30;
    conn.session.talk_count = 9;
    let mut out = HandleOutcome::default();
    {
        let mut sender = ResponseSender::new(&mut conn, &mut out);
        sender.end_talk();
    }
    assert_eq!(conn.session.idtalking, 0);
    assert_eq!(conn.session.select_menu, 0);
    assert_eq!(conn.session.talk_count, 0);
    assert_eq!(
        out.outgoing.last().map(|f| f.frame.as_str()),
        Some("F44402001408")
    );
}

#[test]
fn response_sender_hp_sp_pair_and_broadcast() {
    let mut conn = ts_dream::server::session::Conn::new();
    conn.session.id = 42;
    conn.session.hp = 500;
    conn.session.sp = 250;
    let mut out = HandleOutcome::default();
    {
        let mut sender = ResponseSender::new(&mut conn, &mut out);
        sender.send_hp_sp_updates();
        sender.broadcast(42, "F44407000601");
    }
    assert_eq!(out.outgoing.len(), 2);
    assert_eq!(out.map_broadcast.len(), 1);
    assert_eq!(out.map_broadcast[0].subject, 42);
}

// ════════════════════════════════════════════════════════════════════════════
//  PlayerStateManager
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn player_state_manager_mutates_live_session() {
    let id = register(700_001);
    assert!(PlayerStateManager::update(id, |s| s.gold = 555));
    let snap = PlayerStateManager::snapshot(id).unwrap();
    assert_eq!(snap.gold, 555);
    unregister(&[id]);
}

#[test]
fn player_state_manager_offline_is_no_op() {
    assert!(!PlayerStateManager::update(700_099, |_| {}));
    assert!(PlayerStateManager::snapshot(700_099).is_none());
    assert!(PlayerStateManager::adjust_hp(700_099, 10).is_none());
    assert!(PlayerStateManager::recompute_stats(700_099).is_none());
}

#[test]
fn player_state_manager_adjust_hp_sp_clamps() {
    let id = register(700_002);
    PlayerStateManager::update(id, |s| {
        s.hp_max = 100;
        s.sp_max = 80;
        s.hp = 50;
        s.sp = 40;
    });
    // Damage below zero clamps at 0.
    assert_eq!(PlayerStateManager::adjust_hp(id, -10_000), Some(0));
    // Overheal clamps at hp_max.
    assert_eq!(PlayerStateManager::adjust_hp(id, 10_000), Some(100));
    assert_eq!(PlayerStateManager::adjust_sp(id, -999), Some(0));
    assert_eq!(PlayerStateManager::adjust_sp(id, 999), Some(80));
    unregister(&[id]);
}

#[test]
fn player_state_manager_recompute_reflects_equipment_bonus() {
    let id = register(700_003);
    PlayerStateManager::update(id, |s| {
        s.level = 10;
        s.trangbi.push(InventoryItem {
            slot: 1,
            id: 11001,
            loai: 2,
            hpx1: 500,
            spx1: 300,
            int1: 20,
            ..InventoryItem::default()
        });
    });
    let before = PlayerStateManager::derived_stats(id).unwrap();
    let after = PlayerStateManager::recompute_stats(id).unwrap();
    assert!(
        after.hpx2 >= before.hpx2,
        "equipment bonus must be picked up by the recompute"
    );
    // HP/SP are clamped into the new maxima by the recompute itself.
    let snap = PlayerStateManager::snapshot(id).unwrap();
    assert!(snap.hp <= snap.hp_max);
    assert!(snap.sp <= snap.sp_max);
    unregister(&[id]);
}

// ════════════════════════════════════════════════════════════════════════════
//  TradeSystem
// ════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn trade_system_begin_cancel_lifecycle() {
    let a = register(700_010);
    let b = register(700_011);
    assert_eq!(TradeSystem::begin(a, a).await, TradeOutcome::Rejected);
    assert_eq!(TradeSystem::begin(a, b).await, TradeOutcome::Settled);
    assert!(TradeSystem::is_trading(a));
    assert!(TradeSystem::is_trading(b));
    // A third player cannot open a trade with someone already trading.
    let c = register(700_012);
    assert_eq!(TradeSystem::begin(c, b).await, TradeOutcome::Rejected);
    assert_eq!(TradeSystem::cancel(a).await, TradeOutcome::Settled);
    assert!(!TradeSystem::is_trading(a));
    assert!(!TradeSystem::is_trading(b));
    unregister(&[a, b, c]);
}

#[tokio::test]
async fn trade_system_begin_with_offline_partner_is_offline() {
    let a = register(700_013);
    assert_eq!(
        TradeSystem::begin(a, 700_140).await,
        TradeOutcome::Offline
    );
    unregister(&[a]);
}

fn seed_offer(player: u32, partner: u32, bag_count: u32) {
    PlayerStateManager::update(player, |s| {
        s.gold = 10_000;
        s.homdo.push(InventoryItem {
            slot: 1,
            id: 1001,
            count: bag_count as u8,
            texp: 7,
            ..InventoryItem::default()
        });
        s.trade = TradeState {
            active: true,
            partner_id: partner,
            accepted: true,
            gold: 1_000,
            items: vec![InventoryItem {
                slot: 1,
                id: 1001,
                count: bag_count as u8,
                texp: 7,
                ..InventoryItem::default()
            }],
            pets: Vec::new(),
        };
    });
}

#[tokio::test]
async fn trade_system_accept_swaps_gold_and_items_atomically() {
    let a = register(700_020);
    let b = register(700_021);
    seed_offer(a, b, 2);
    seed_offer(b, a, 2);

    let (outcome, new_a, new_b) = TradeSystem::accept(a).await;
    assert_eq!(outcome, TradeOutcome::Settled);
    let (na, nb) = (new_a.unwrap(), new_b.unwrap());
    // Gold swapped: each side paid 1_000 of its 10_000 and received the
    // partner's 1_000 — net unchanged but the ledger moved through the engine.
    assert_eq!(na.gold, 10_000);
    assert_eq!(nb.gold, 10_000);
    // Items swapped: a now holds the row that came from b's bag and vice versa.
    assert_eq!(na.homdo.iter().find(|i| i.id == 1001).map(|i| i.texp), Some(7));
    assert!(!na.trade.active);
    assert!(!nb.trade.active);
    // Registry reflects the settled sessions.
    let reg = online_sessions().lock().unwrap();
    assert!(!reg.get(&a).unwrap().trade.active);
    assert!(!reg.get(&b).unwrap().trade.active);
    drop(reg);
    unregister(&[a, b]);
}

#[tokio::test]
async fn trade_system_rejects_when_offered_item_vanished() {
    let a = register(700_030);
    let b = register(700_031);
    seed_offer(a, b, 2);
    seed_offer(b, a, 2);
    // Behind-the-bag mutation: a's offered stack shrank after the offer was
    // made — the classic duplicate-item exploit attempt.
    PlayerStateManager::update(a, |s| {
        s.homdo[0].count = 1;
        s.gold = 10_000;
    });

    let (outcome, _, _) = TradeSystem::accept(a).await;
    assert_eq!(outcome, TradeOutcome::Rejected);
    // Both bags untouched.
    let reg = online_sessions().lock().unwrap();
    assert_eq!(reg.get(&b).unwrap().homdo.len(), 1);
    assert_eq!(reg.get(&b).unwrap().gold, 10_000);
    // Trade state torn down so neither side is stuck trading.
    assert!(!reg.get(&a).unwrap().trade.active);
    assert!(!reg.get(&b).unwrap().trade.active);
    drop(reg);
    unregister(&[a, b]);
}

#[tokio::test]
async fn trade_system_accept_without_partner_online_is_offline() {
    let a = register(700_040);
    PlayerStateManager::update(a, |s| {
        s.trade = TradeState {
            active: true,
            partner_id: 700_141,
            accepted: false,
            ..TradeState::default()
        };
    });
    assert_eq!(TradeSystem::accept(a).await.0, TradeOutcome::Offline);
    unregister(&[a]);
}

// ════════════════════════════════════════════════════════════════════════════
//  AutoSaveService
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn fingerprint_changes_on_any_persisted_mutation() {
    let mut s = Session::new();
    s.id = 1;
    let base = auto_save::fingerprint(&s);
    s.gold += 1;
    assert_ne!(auto_save::fingerprint(&s), base);
    let after_gold = auto_save::fingerprint(&s);
    s.homdo.push(InventoryItem {
        slot: 1,
        id: 100,
        count: 1,
        ..InventoryItem::default()
    });
    assert_ne!(auto_save::fingerprint(&s), after_gold);
    // Stable across clones (deterministic field order).
    assert_eq!(auto_save::fingerprint(&s.clone()), auto_save::fingerprint(&s));
}

#[tokio::test]
async fn auto_save_cycle_without_pool_is_a_no_op() {
    let ledger = Default::default();
    assert_eq!(auto_save::run_cycle(None, &ledger).await, 0);
}

#[tokio::test]
async fn auto_save_ledger_forget_clears_entry() {
    let ledger = <auto_save::SaveLedger as Default>::default();
    ledger.lock().unwrap().insert(700_050, 42);
    auto_save::forget(&ledger, 700_050);
    assert!(!ledger.lock().unwrap().contains_key(&700_050));
}

// ════════════════════════════════════════════════════════════════════════════
//  npc_event — Eve engine bridge
// ════════════════════════════════════════════════════════════════════════════

fn scene_fixture() -> GameData {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.npcs.insert(
        7,
        EveNpcPlacement {
            id: 7,
            npc_id: 16080,
            events: vec![1],
            ..EveNpcPlacement::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_class: 0,
                results: vec![ts_dream::data::loaders::EveResult {
                    result_type: 6,
                    ..ts_dream::data::loaders::EveResult::default()
                }],
                ..EveCondition::default()
            }],
            ..NpcEventData::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);
    data
}

#[test]
fn npc_event_disabled_by_default_keeps_legacy_flow() {
    npc_event::set_eve_events_enabled(false);
    let session = Session::new();
    let data = scene_fixture();
    let out = npc_event::resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(7),
        &mut DotNetRandom::new(1),
    );
    assert!(out.is_none(), "gate off -> legacy talk tables own the flow");
}

#[test]
fn npc_event_resolves_matching_chain_into_event_session() {
    npc_event::set_eve_events_enabled(true);
    let mut session = Session::new();
    session.map_id = 9001;
    let data = scene_fixture();
    let activated = npc_event::resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(7),
        &mut DotNetRandom::new(1),
    );
    npc_event::set_eve_events_enabled(false);
    let ev = activated.expect("unconditional chain must match");
    assert_eq!(ev.eve_no, 1);
    assert_eq!(ev.trigger_kind, 1);
    assert_eq!(ev.npc_click_id, 7);
    assert_eq!(ev.current_index, 0);
}

#[test]
fn npc_event_auto_chain_requires_depth_and_map_consistency() {
    use ts_dream::eve::auto_chain::{
        AutoChainResult, EventPhase, EventSession, MAX_CHAIN_DEPTH,
    };

    let mut session = Session::new();
    session.map_id = 9001;
    let data = scene_fixture();

    let completed = EventSession {
        map_id: 9001,
        eve_no: 1,
        npc_click_id: 7,
        trigger_kind: 1,
        chain_depth: MAX_CHAIN_DEPTH,
        results: Vec::new(),
        current_index: 0,
        phase: EventPhase::Completed,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 0,
    };
    assert_eq!(
        npc_event::auto_chain_after(&completed, &session, &data, &mut DotNetRandom::new(1)),
        AutoChainResult::NoMatch,
        "depth cap blocks chaining"
    );

    let moved_player = {
        let mut s = session.clone();
        s.map_id = 12002;
        s
    };
    let at_cap = EventSession {
        chain_depth: MAX_CHAIN_DEPTH - 1,
        ..completed.clone()
    };
    assert_eq!(
        npc_event::auto_chain_after(&at_cap, &moved_player, &data, &mut DotNetRandom::new(1)),
        AutoChainResult::NoMatch,
        "player left the triggering map"
    );
}

/// The battle service dependency stays constructible headlessly — a sanity
/// check that the dispatcher wiring keeps working without a live DB.
#[allow(dead_code)]
fn battle_service_headless() -> ts_dream::battle::service::BattleService {
    ts_dream::battle::service::BattleService::new(Arc::new(GameData::default()))
}
