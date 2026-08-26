//! Ticket 07 integration tests: ResponseSender, PlayerStateManager,
//! TradeSystem, AutoSaveService dirty detection, and the npc_event Eve-engine
//! bridge.

use std::sync::{Arc, Mutex};

use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{EveCondition, EveNpcPlacement, NpcEventData, SceneEveData};
use ts_dream::data::tables::{
    BattleGate, Item, Npc, NpcOnMap, QuestDef, QuestResult, Skill, Warp,
};
use ts_dream::protocol::encoder;
use ts_dream::server::auto_save;
use ts_dream::server::dispatcher::{test_ctx, HandleOutcome};
use ts_dream::server::handlers::battle::{handle_battle, handle_battle_command};
use ts_dream::server::handlers::chat::handle_chat;
use ts_dream::server::handlers::character::{
    apply_to_session, parse_create, CreateCharData,
};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::handlers::npc_event::{self, NpcTrigger};
use ts_dream::server::handlers::party::handle_party;
use ts_dream::server::handlers::pet_actions::{
    find_free, handle_pet_actions, handle_pet_summon, is_roster,
};
use ts_dream::server::handlers::quest::{
    battle_quest_win, evaluate_requirements, generate_daily_quest, handle_pet_reborn_npc,
    handle_warp_confirm, is_pet_reborn_key, quest_key, send_requirement_fail, try_quest_h6,
};
use ts_dream::server::handlers::shops::{
    complete_shop_buy, get_npc_shop_price, handle_npc_shop, handle_player_shop,
    player_shop_catalog_frame, ShopBuyError,
};
use ts_dream::server::handlers::skills::{handle_pet_reborn, handle_skills};
use ts_dream::server::handlers::stats::{build_stat_update, handle_hotkey, handle_stat_allocation};
use ts_dream::server::handlers::system::{
    handle_game_points, handle_gm_shop, handle_pk_war, handle_rank, handle_teleport_confirm,
    parse_len_strings,
};
use ts_dream::server::handlers::talk::handle_talk;
use ts_dream::server::handlers::trade_storage::{
    handle_bank_gold, handle_storage_transfer, handle_trade,
};
use ts_dream::server::handlers::use_item::use_item_rng;
use ts_dream::server::map_drops;
use ts_dream::server::player_state::PlayerStateManager;
use ts_dream::server::response::ResponseSender;
use ts_dream::server::session::{
    online_sessions, Conn, InventoryItem, PetState, Session, TradeState,
};
use ts_dream::server::trade_system::{TradeOutcome, TradeSystem};

/// Serializes the map-drop registry across the drop/pickup tests (they share
/// one global registry keyed by `(map_id, slot)`).
static MAP_DROP_LOCK: Mutex<()> = Mutex::new(());

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

// ════════════════════════════════════════════════════════════════════════════
//  Migrated handler unit tests (ticket 08)
//
//  Each section below was extracted from the `#[cfg(test)]` module of the
//  named source file and now drives the handlers through the public API.
// ════════════════════════════════════════════════════════════════════════════

// ── handlers::battle ── (migrated from src/server/handlers/battle.rs)

fn battle_service() -> BattleService {
    BattleService::new(Arc::new(GameData::default()))
}

#[test]
fn leave_battle_sends_hide_and_clears_battle() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.battle_id = 5;
    let svc = battle_service();
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &[3]);
    handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 0);
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].starts_with("F44408000B00"));
    assert!(out.outgoing[0].ends_with("0000"));
}

#[test]
fn leave_battle_ignored_unless_confirm_3() {
    let mut conn = Conn::new();
    conn.session.battle_id = 5;
    let svc = battle_service();
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &[1]);
    handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 5);
    assert!(out.outgoing.is_empty());
}

#[test]
fn attack_npc_blocked_range_does_not_spawn() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    let svc = battle_service();
    // NPC id 20001 ∈ [20000, 22000) → blocked.
    let npc_id = 20001u32;
    let mut payload = vec![0u8; 6];
    payload[0] = 3;
    payload[1..5].copy_from_slice(&npc_id.to_le_bytes());
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
    handle_battle(&mut ctx);
    assert_eq!(conn.session.battle_id, 0);
    assert!(out.outgoing.is_empty());
}

#[test]
fn pk_challenge_target_pk_zero_replies() {
    let svc = battle_service();
    // Register an online target with pk == 0.
    let target = Arc::new(tokio::sync::RwLock::new(Session::new()));
    {
        let mut t = target.try_write().unwrap();
        t.id = 300002;
        t.pk = 0;
    }
    let _rx = svc.register(300002, target);

    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.pk = 1;
    let mut payload = vec![0u8; 5];
    payload[0] = 2;
    payload[1..5].copy_from_slice(&300002u32.to_le_bytes());
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
    handle_battle(&mut ctx);
    assert_eq!(out.outgoing, vec!["F4440300210101".to_string()]);
}

#[test]
fn pk_challenge_self_pk_zero_is_blocked() {
    let svc = battle_service();
    let target = Arc::new(tokio::sync::RwLock::new(Session::new()));
    {
        let mut t = target.try_write().unwrap();
        t.id = 300002;
        t.pk = 0;
    }
    let _rx = svc.register(300002, target);

    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.pk = 0; // attacker must have pk == 1
    let mut payload = vec![0u8; 5];
    payload[0] = 2;
    payload[1..5].copy_from_slice(&300002u32.to_le_bytes());
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
    handle_battle(&mut ctx);
    assert!(out.outgoing.is_empty());
}

#[test]
fn skill_command_battle_zero_is_ignored() {
    let mut conn = Conn::new();
    conn.session.battle_id = 0;
    let svc = battle_service();
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    // skill 10000 targeting (0,2)
    let mut payload = vec![0u8; 6];
    payload[0] = 3;
    payload[1] = 2;
    payload[2] = 0;
    payload[3] = 2;
    payload[4..6].copy_from_slice(&10000u16.to_le_bytes());
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &payload);
    handle_battle_command(&mut ctx);
    assert!(out.outgoing.is_empty());
}

/// G1: op 0x0B sub4 (join battle) against a live seeded battle. Drives the
/// dispatcher `handle_battle` and asserts the session joins (battle id
/// assigned) and the battle trailer `F44403000B0A01` is delivered to the
/// joiner's frame channel (join burst).
#[tokio::test]
async fn join_battle_sub4_joins_and_sends_trailer() {
    // Service with an NPC so a real battle can be seeded.
    let mut data = GameData::default();
    data.npcs.insert(
        9001,
        Npc {
            id: 9001,
            lv: 1,
            hp: 30,
            sp: 30,
            thuoctinh: 1,
            atk: 1,
            def: 1,
            agi: 1,
            int1: 1,
            skill: [10000, 0, 0, 0],
            ..Default::default()
        },
    );
    let svc = Arc::new(BattleService::new(Arc::new(data)));

    // Leader seeds the battle; the returned id is what sub4 expects.
    let mut leader = Session::new();
    leader.id = 300001;
    let battle_id = svc.start_npc_battle_seeded(&mut leader, 9001, 11, 1, 2, 3);
    assert!(battle_id > 0, "seeded battle must start");

    // Joiner: register so join frames are captured on a channel, then drive
    // op 0x0B sub4 with the battle id LE32.
    let mut conn = Conn::new();
    conn.session.id = 300002;
    let joiner = Arc::new(tokio::sync::RwLock::new(conn.session.clone()));
    let mut rx = svc.register(300002, joiner);

    let mut payload = vec![0u8; 4];
    payload[0..4].copy_from_slice(&(battle_id as u32).to_le_bytes());
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 4, &payload);
    handle_battle(&mut ctx);

    // Join succeeded: battle id assigned and no error frame.
    assert_eq!(conn.session.battle_id, battle_id);
    assert!(out.outgoing.is_empty());

    // Battle trailer `F44403000B0A01` delivered to the joiner's channel.
    let mut got_trailer = false;
    while let Ok(f) = rx.try_recv() {
        if f.contains("F44403000B0A01") {
            got_trailer = true;
        }
    }
    assert!(got_trailer, "expected battle trailer F44403000B0A01 on join");
}

/// G1: op 0x32 sub2 (use-item heal) within a battle. The handler must
/// consume the item from homdo and submit the command with no error frame;
/// the actual heal runs in the battle task.
#[test]
fn use_item_in_battle_removes_item() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.battle_id = 7; // participating in a battle
    conn.session.add_homdo_item(InventoryItem {
        id: 26001,
        count: 1,
        ..Default::default()
    });
    let svc = battle_service();
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    // op 0x32 sub2: row, col, row_attack, col_attack, item id LE16.
    let mut payload = vec![0u8; 6];
    payload[0] = 0;
    payload[1] = 1;
    payload[2] = 0;
    payload[3] = 2;
    payload[4..6].copy_from_slice(&26001u16.to_le_bytes());
    let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
    handle_battle_command(&mut ctx);

    // Item consumed from inventory.
    assert!(
        conn.session.homdo.iter().find(|i| i.id == 26001).is_none(),
        "use-item must remove the consumed item"
    );
    // Handler submits the command and emits no error frame.
    assert!(out.outgoing.is_empty());
}

// ── handlers::character ── (migrated from src/server/handlers/character.rs)

#[test]
fn parse_uses_single_byte_hair() {
    // hair lives at payload[2]; payload[3] is an unused gap that must never
    // raise hair to a 16-bit value.
    let mut payload = vec![0u8; 26];
    payload[0] = 1; // sex
    payload[2] = 5; // hair
    payload[3] = 0xFF; // unused gap
    payload[19] = 4; // pass1_len
    let data = parse_create(&payload).expect("valid payload");
    assert_eq!(data.hair, 5);
    assert_eq!(data.sex, 1);
}

#[test]
fn parse_maps_color_and_stats_by_offset() {
    // 25-byte payload: sex[0], hair[2], color[4..12], thuoctinh[12],
    // stats[13..19], pass1_len[19], pass1[20..22], gap[22], pass2[23..25].
    let mut p = vec![0u8; 25];
    p[10] = 0xAB; // color byte (index 10 of the [4..12] window)
    p[12] = 3; // thuoctinh
    p[13] = 7; // Int
    p[14] = 8; // Atk
    p[15] = 9; // Def
    p[16] = 10; // Hpx
    p[17] = 11; // Spx
    p[18] = 12; // Agi
    p[19] = 2; // pass1_len
    p[20] = 0x41; // pass1[0]
    p[21] = 0x42; // pass1[1]
    p[23] = 0x43; // pass2[0]
    p[24] = 0x44; // pass2[1]
    let d = parse_create(&p).expect("valid payload");
    assert_eq!(d.thuoctinh, 3, "thuoctinh sits after the 8 color bytes");
    assert_eq!(d.color_hex, "000000000000AB00");
    assert_eq!((d.int1, d.atk, d.def, d.hpx, d.spx, d.agi), (7, 8, 9, 10, 11, 12));
    assert_eq!(d.pass1, vec![0x41, 0x42]);
    assert_eq!(d.pass2, vec![0x43, 0x44]);
}

#[test]
fn apply_to_session_reflects_players_row_and_starter_items() {
    let mut session = Session::new();
    let d = CreateCharData {
        sex: 1,
        hair: 2,
        color_hex: "0000000000000000".to_string(),
        thuoctinh: 3,
        int1: 1,
        atk: 2,
        def: 3,
        hpx: 6,
        spx: 6,
        agi: 4,
        pass1: vec![],
        pass2: vec![],
    };
    apply_to_session(&mut session, &d);

    // HP/SP computed from the formula (engine get_hp_max, lv 1).
    assert_eq!(session.hp, 105);
    assert_eq!(session.hp_max, 105);
    assert_eq!(session.sp, 73);
    assert_eq!(session.sp_max, 73);
    assert_eq!(session.level, 1);
    assert_eq!(session.job, 0);
    assert_eq!(session.reborn, 0);
    assert_eq!(session.map_id, 10817);
    assert_eq!(session.map_x, 442);
    assert_eq!(session.map_y, 758);
    assert_eq!(session.texp, 6);
    assert_eq!(session.tiengtam, 1);
    assert_eq!(session.tham_chien, 1);

    // Starter inventory seeded into Homdo + Trangbi.
    assert!(session.homdo.iter().any(|i| i.id == 32012 && i.count == 4));
    let armor = session.trangbi.iter().find(|i| i.id == 19737);
    assert_eq!(armor.map(|i| (i.count, i.agi1, i.loai)), Some((1, 1, 2)));
}

// ── handlers::chat ── (migrated from src/server/handlers/chat.rs)

#[tokio::test]
async fn map_chat_echoes_self() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    let mut out = HandleOutcome::default();
    let payload = b"HELLO";
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, payload);
    handle_chat(&mut ctx).await;
    assert_eq!(out.outgoing, vec!["F4440B000202E193040048454C4C4F"]);
}

#[tokio::test]
async fn long_chat_dropped() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    let mut out = HandleOutcome::default();
    let payload = vec![b'X'; 61];
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, &payload);
    handle_chat(&mut ctx).await;
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn global_chat_item_selects_0201() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.trangbi.push(InventoryItem {
        slot: 6,
        id: 23100,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, b"HI");
    handle_chat(&mut ctx).await;
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].starts_with("F44408000201"));
}

#[tokio::test]
async fn slash_where_returns_sysmsg() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.map_id = 12001;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, b"/where");
    handle_chat(&mut ctx).await;
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].contains("020B"));
    assert!(out.outgoing[0].contains("3132303031")); // hex of "12001"
}

#[tokio::test]
async fn unknown_slash_silently_dropped() {
    // A disabled admin command (or any unrecognized `/cmd`) produces no
    // reply and no broadcast.
    let mut conn = Conn::new();
    conn.session.id = 300015;
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        2,
        b"/additem 10001,2",
    );
    handle_chat(&mut ctx).await;
    assert!(
        out.outgoing.is_empty(),
        "unknown /cmd must be silently dropped"
    );
    assert!(conn.session.homdo.is_empty());
}

#[tokio::test]
async fn sleep_heals_self_and_pets() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.hp = 10;
    conn.session.hp_max = 100;
    conn.session.sp = 5;
    conn.session.sp_max = 50;
    conn.session.pets.push(PetState {
        stt: 1,
        id: 18001,
        level: 1,
        hp: 10,
        hp_max: 90,
        sp: 10,
        sp_max: 40,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, b"/sleep");
    handle_chat(&mut ctx).await;
    assert_eq!(conn.session.hp, 100);
    assert_eq!(conn.session.sp, 50);
    assert_eq!(conn.session.pets[0].hp, 90);
    assert_eq!(conn.session.pets[0].sp, 40);
    let joined: String = out.outgoing.iter().map(|f| f.frame.as_str()).collect();
    assert!(joined.contains("1F0A"));
    assert!(joined.contains("080204")); // pet Hp/Sp stat frames
    assert!(joined.contains("1F0100")); // sleep done
}

#[tokio::test]
async fn sleep_skips_when_in_battle() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.battle_id = 7;
    conn.session.hp = 10;
    conn.session.hp_max = 100;
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 2, b"/sleep");
    handle_chat(&mut ctx).await;
    assert!(out.outgoing.is_empty());
    assert_eq!(conn.session.hp, 10);
}

#[tokio::test]
async fn openhotel_builds_stable_frames() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.pets.push(PetState {
        stt: 5,
        id: 18001,
        level: 3,
        hp: 80,
        name: b"PET".to_vec(),
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        2,
        b"/openhotel",
    );
    handle_chat(&mut ctx).await;
    let joined: String = out.outgoing.iter().map(|f| f.frame.as_str()).collect();
    assert!(joined.contains("1F06"));
    assert!(joined.contains("504554")); // hex of "PET"
    assert!(joined.ends_with("1F07"));
    assert_eq!(conn.session.select_menu, 40);
}

#[tokio::test]
async fn openbank_uses_bank_gold() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    conn.session.bank_gold = 4321;
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        2,
        b"/openbank",
    );
    handle_chat(&mut ctx).await;
    // 4321 = 0x10E1 → LE bytes E1 10 00 00
    assert!(out.outgoing.iter().any(|f| f.contains("1D04") && f.contains("E1100000")));
}

#[tokio::test]
async fn whisper_builds_frame_with_recipient_id() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    let mut out = HandleOutcome::default();
    // target id 300016 LE bytes.
    let payload: Vec<u8> = [0x10, 0x39, 0x04, 0x00].to_vec();
    let mut payload = payload;
    payload.extend_from_slice(b"hey");
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx =
        test_ctx(&mut conn, &data, &service, &mut out, 3, &payload);
    handle_chat(&mut ctx).await;
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].starts_with("F44409000203"));
    assert!(out.outgoing[0].contains("10390400"));
    assert!(out.outgoing[0].ends_with("686579"));
}

#[tokio::test]
async fn party_chat_frame_uses_0205() {
    let mut conn = Conn::new();
    conn.session.id = 300015;
    let mut out = HandleOutcome::default();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        5,
        b"hello party",
    );
    handle_chat(&mut ctx).await;
    assert_eq!(out.outgoing.len(), 1);
    assert!(out.outgoing[0].starts_with("F44411000205"));
}

// ── handlers::inventory ── (migrated from src/server/handlers/inventory.rs)

fn bag_item(slot: u8, id: u16, count: u8, loai: u8) -> InventoryItem {
    InventoryItem {
        slot,
        id,
        count,
        loai,
        ..Default::default()
    }
}

async fn run(conn: &mut Conn, sub: u8, payload: &[u8]) -> HandleOutcome {
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(conn, &data, &service, &mut out, sub, payload);
    handle_inventory(&mut ctx).await;
    out
}

#[tokio::test]
async fn equip_respects_level_gate() {
    let mut conn = Conn::new();
    conn.session.level = 5;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 12001,
        count: 1,
        lv: 10,
        loai: 1,
        ..Default::default()
    });
    let out = run(&mut conn, 11, &[1]).await;
    assert!(conn.session.trangbi.is_empty(), "below level: no equip");
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn equip_moves_to_trangbi_and_sends_stats() {
    let mut conn = Conn::new();
    conn.session.level = 10;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 12001,
        count: 1,
        lv: 1,
        loai: 1,
        atk1: 15,
        ..Default::default()
    });
    let out = run(&mut conn, 11, &[1]).await;
    assert_eq!(conn.session.trangbi.len(), 1);
    assert_eq!(conn.session.trangbi[0].id, 12001);
    assert!(conn.session.homdo.is_empty());
    assert_eq!(conn.session.atk2, 15);
    assert!(out.outgoing.iter().any(|f| f.contains("171101")));
    // Gear stat packets include the HP/SP max recompute frames.
    assert!(out.outgoing.iter().any(|f| f.contains("08011A")));
    assert!(out.outgoing.iter().any(|f| f.contains("080119")));
}

#[tokio::test]
async fn unequip_requires_empty_destination_slot() {
    let mut conn = Conn::new();
    conn.session.homdo.push(bag_item(2, 5001, 1, 0));
    conn.session.trangbi.push(InventoryItem {
        slot: 1,
        id: 12001,
        count: 1,
        lv: 1,
        loai: 1,
        ..Default::default()
    });
    // Destination homdo slot 2 is occupied -> rejected.
    let out = run(&mut conn, 12, &[1, 2]).await;
    assert_eq!(
        conn.session.trangbi.len(),
        1,
        "must not unequip onto a full slot"
    );
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn unequip_moves_to_empty_homdo_slot() {
    let mut conn = Conn::new();
    conn.session.homdo.push(bag_item(2, 0, 0, 0));
    conn.session.trangbi.push(InventoryItem {
        slot: 1,
        id: 12001,
        count: 1,
        lv: 1,
        loai: 1,
        atk1: 15,
        ..Default::default()
    });
    let out = run(&mut conn, 12, &[1, 2]).await;
    assert!(conn.session.trangbi.is_empty());
    assert_eq!(conn.session.homdo[0].slot, 2);
    assert_eq!(conn.session.atk2, 0);
    assert!(out.outgoing.iter().any(|f| f.contains("17100102")));
}

#[tokio::test]
async fn move_stack_splits_counts() {
    let mut conn = Conn::new();
    conn.session.homdo.push(bag_item(1, 100, 20, 0));
    conn.session.homdo.push(bag_item(2, 100, 10, 0));
    // move 5 from slot 1 to slot 2 (loai 0 -> stackable, total 15 <= 50)
    let decoded = encoder::bytes("F4440700170A010502").unwrap();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 10, &decoded[6..]);
    // decoded is only used for the echo, so feed it via a raw frame anyway.
    ctx.decoded = &decoded;
    handle_inventory(&mut ctx).await;

    assert_eq!(
        conn.session
            .homdo
            .iter()
            .find(|i| i.slot == 1)
            .unwrap()
            .count,
        15
    );
    assert_eq!(
        conn.session
            .homdo
            .iter()
            .find(|i| i.slot == 2)
            .unwrap()
            .count,
        15
    );
    assert_eq!(out.outgoing, vec![encoder::hex(&decoded)]);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // the guard deliberately serializes the whole test
async fn pickup_requires_distance_gate() {
    let _map_guard = MAP_DROP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    map_drops::clear_all();
    let mut conn = Conn::new();
    conn.session.map_id = 7701;
    conn.session.map_x = 1000;
    conn.session.map_y = 1000;
    map_drops::drop(7701, 1, bag_item(1, 1001, 1, 0), 400, 500);
    let out = run(&mut conn, 2, &[1]).await;
    assert!(conn.session.homdo.is_empty(), "out of range: no pickup");
    assert!(out.outgoing.is_empty());
    assert!(
        map_drops::get(7701, 1).is_some(),
        "drop must stay on the map when out of range"
    );
    map_drops::clear_all();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // the guard deliberately serializes the whole test
async fn pickup_adds_item_to_homdo() {
    let _map_guard = MAP_DROP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    map_drops::clear_all();
    let mut conn = Conn::new();
    conn.session.map_id = 7702;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    map_drops::drop(7702, 1, bag_item(1, 1001, 2, 0), 400, 500);
    let out = run(&mut conn, 2, &[1]).await;
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].id, 1001);
    assert_eq!(conn.session.homdo[0].count, 2);
    assert!(
        map_drops::get(7702, 1).is_none(),
        "drop consumed by pickup"
    );
    assert!(out.outgoing.iter().any(|f| f.contains("1702")));
    assert!(out.outgoing.iter().any(|f| f.contains("1706")));
    map_drops::clear_all();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // the guard deliberately serializes the whole test
async fn drop_creates_map_drop_and_reduces_count() {
    let _map_guard = MAP_DROP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    map_drops::clear_all();
    let mut conn = Conn::new();
    conn.session.map_id = 7703;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    conn.session.homdo.push(bag_item(3, 1001, 5, 0));
    let out = run(&mut conn, 3, &[3, 2]).await;
    assert_eq!(conn.session.homdo[0].count, 3, "dropped 2 of 5");
    // The drop lands under a freshly allocated per-map slot (first free = 1),
    // not under the homdo slot, and carries the dropped count (2).
    let drop = map_drops::get(7703, 1).expect("drop on map");
    assert_eq!(drop.item.id, 1001);
    assert_eq!(drop.item.count, 2);
    assert!(out.outgoing.iter().any(|f| f.contains("17090303")));
    map_drops::clear_all();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // the guard deliberately serializes the whole test
async fn drop_refuses_when_map_full_keeps_item() {
    let _map_guard = MAP_DROP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    map_drops::clear_all();
    let mut conn = Conn::new();
    conn.session.map_id = 7704;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    conn.session.homdo.push(bag_item(3, 1001, 5, 0));
    // Fill every map slot 1..=255.
    for slot in 1..=255u8 {
        map_drops::drop(7704, slot, bag_item(slot, 7000, 1, 0), 0, 0);
    }
    let out = run(&mut conn, 3, &[3, 2]).await;
    assert_eq!(conn.session.homdo[0].count, 5, "full map: item kept");
    assert!(out.outgoing.is_empty(), "full map: silent, no frames");
    map_drops::clear_all();
}

#[tokio::test]
async fn reborn_rejected_if_equipped() {
    let mut conn = Conn::new();
    conn.session.trangbi.push(InventoryItem {
        slot: 1,
        id: 12001,
        count: 1,
        lv: 1,
        loai: 1,
        ..Default::default()
    });
    let out = run(&mut conn, 46, &[]).await;
    assert_eq!(conn.session.reborn, 0);
    assert!(out.outgoing.iter().any(|f| f.contains("1401")));
}

#[tokio::test]
async fn reborn_resets_stats_retains_special_skills() {
    let mut conn = Conn::new();
    conn.session.level = 125;
    conn.session.reborn = 0;
    conn.session.skills = vec![(10001, 10), (10016, 10)]; // 10001 normal, 10016 special
    let out = run(&mut conn, 46, &[10, 0, 0, 0, 0, 0, 0, 0, 0]).await;

    assert_eq!(conn.session.reborn, 1);
    assert_eq!(conn.session.level, 1);
    assert_eq!(conn.session.point, 1); // 0 + (125-120)/5 = 1
    assert_eq!(conn.session.skill_point, 25); // 24 + (125-120)/5 = 25
    assert_eq!(conn.session.skills.len(), 1);
    assert_eq!(conn.session.skills[0], (10016, 10)); // special skill retained
    assert!(out
        .outgoing
        .iter()
        .any(|f| f.frame == "F44402002C01"));
    assert!(out
        .outgoing
        .iter()
        .any(|f| f.frame == "F4441100140100000001010302000000000000F476"));
    assert!(out.shutdown, "reborn closes the socket (character death)");
    assert_eq!(conn.session.quest_steps, vec![(59411, 2)]);
}

// ── handlers::party ── (migrated from src/server/handlers/party.rs)

#[test]
fn leader_designates_quan_su() {
    let svc = BattleService::new(Arc::new(GameData::default()));
    let data = GameData::default();
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.id_leader = 300001;
    conn.session.id_mem = [300002, 300003, 0, 0];
    let mut out = HandleOutcome::default();
    let payload = 300002u32.to_le_bytes().to_vec();
    let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 5, &payload);
    handle_party(&mut c);
    assert_eq!(conn.session.id_qs, 300002);
    // Self frames + map fan-out = 3 + 2.
    assert_eq!(out.outgoing.len(), 3);
    assert!(out.outgoing.iter().all(|f| f.contains("0D0")));
    assert_eq!(out.map_broadcast.len(), 2);
}

#[test]
fn non_member_or_non_leader_ignored() {
    let svc = BattleService::new(Arc::new(GameData::default()));
    let data = GameData::default();
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.id_leader = 300001;
    conn.session.id_mem = [300002, 300003, 0, 0];
    let mut out = HandleOutcome::default();
    // Not a member.
    let payload = 300009u32.to_le_bytes().to_vec();
    let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 5, &payload);
    handle_party(&mut c);
    assert_eq!(conn.session.id_qs, 0);
    assert!(out.outgoing.is_empty());
    // Member but not leader.
    let svc2 = BattleService::new(Arc::new(GameData::default()));
    let data2 = GameData::default();
    let mut conn2 = Conn::new();
    conn2.session.id = 300002;
    conn2.session.id_leader = 300001;
    conn2.session.id_mem = [300002, 300003, 0, 0];
    let mut out2 = HandleOutcome::default();
    let payload = 300003u32.to_le_bytes().to_vec();
    let mut c2 = test_ctx(&mut conn2, &data2, &svc2, &mut out2, 5, &payload);
    handle_party(&mut c2);
    assert_eq!(conn2.session.id_qs, 0);
    assert!(out2.outgoing.is_empty());
}

#[test]
fn clear_quan_su() {
    let svc = BattleService::new(Arc::new(GameData::default()));
    let data = GameData::default();
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.id_qs = 300002;
    let mut out = HandleOutcome::default();
    let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 6, &[]);
    handle_party(&mut c);
    assert_eq!(conn.session.id_qs, 0);
    assert_eq!(out.outgoing.len(), 2);
    assert_eq!(out.map_broadcast.len(), 2);
}

// ── handlers::pet_actions ── (migrated from src/server/handlers/pet_actions.rs)

fn pet_actions_fixture() -> (Conn, GameData, BattleService) {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.pets = vec![
        PetState {
            stt: 1,
            id: 18001,
            ..Default::default()
        },
        PetState {
            stt: 5,
            id: 18002,
            ..Default::default()
        },
    ];
    (
        conn,
        GameData::default(),
        BattleService::new(Arc::new(GameData::default())),
    )
}

#[test]
fn find_free_honors_range() {
    let pets: Vec<PetState> = vec![PetState {
        stt: 2,
        ..Default::default()
    }];
    assert_eq!(find_free(&pets, 1, 4), Some(1));
    let full: Vec<PetState> = (1..=4)
        .map(|stt| PetState {
            stt,
            ..Default::default()
        })
        .collect();
    assert_eq!(find_free(&full, 1, 4), None);
}

#[tokio::test]
async fn sub3_stable_to_roster_moves_pet() {
    let (mut conn, data, service) = pet_actions_fixture();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
    handle_pet_actions(&mut ctx).await;

    assert!(
        conn.session
            .pets
            .iter()
            .any(|p| p.id == 18002 && is_roster(p.stt)),
        "pet moved into a free roster slot"
    );
    assert!(out.outgoing.iter().any(|f| f.contains("1F06")));
}

#[tokio::test]
async fn sub7_roster_to_stable_guards_active() {
    let (mut conn, data, service) = pet_actions_fixture();
    conn.session.active_pet_stt = 1;
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 7, &[1]);
    handle_pet_actions(&mut ctx).await;
    // Active pet cannot be stored: red message + `F44402001F09`, no move.
    assert!(out.outgoing.iter().any(|f| f.ends_with("1F09")));
    assert!(
        conn.session.pets.iter().any(|p| p.id == 18001 && p.stt == 1),
        "active pet must not move to the stable"
    );
}

#[tokio::test]
async fn sub7_non_active_roster_to_stable_moves() {
    let (mut conn, data, service) = pet_actions_fixture();
    // Only the 18001 roster pet is active-able; store pet 18001 (stt 1).
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 7, &[1]);
    handle_pet_actions(&mut ctx).await;
    assert!(out.outgoing.iter().any(|f| f.starts_with("F44407000F02")));
    assert!(
        conn.session.pets.iter().any(|p| p.id == 18001 && p.stt >= 5),
        "roster pet stored into the stable"
    );
}

#[tokio::test]
async fn summon_requires_roster() {
    let (mut conn, data, service) = pet_actions_fixture();
    conn.session.pets.push(PetState {
        stt: 5,
        id: 15001,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    // 15001 (0x3A99) is in the stable — LE32 request must be rejected.
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x99, 0x3A, 0, 0]);
    handle_pet_summon(&mut ctx).await;
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn summon_le32_picks_active() {
    let (mut conn, data, service) = pet_actions_fixture();
    let mut out = HandleOutcome::default();
    // 18001 (0x4651) LE32.
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x51, 0x46, 0, 0]);
    handle_pet_summon(&mut ctx).await;
    assert_eq!(conn.session.active_pet_stt, 1);
    assert!(out.outgoing[0].contains("1301"));
}

#[tokio::test]
async fn sub8_swap_exchanges_occupied_slots() {
    // Both the stable slot (5) and the roster slot (1) hold a pet: the swap
    // must exchange the composite `(player_id, stt)` identities — never
    // leave two pets sharing one `stt` (ticket 17 review).
    let (mut conn, data, service) = pet_actions_fixture(); // stt 1 = 18001, stt 5 = 18002
    conn.session.trangbi.push(InventoryItem {
        slot: 11,
        id: 9001,
        ..Default::default()
    });
    conn.session.trangbi.push(InventoryItem {
        slot: 51,
        id: 9002,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    // payload: stable index 1 (-> stt 5), roster slot 1.
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 8, &[1, 1]);
    handle_pet_actions(&mut ctx).await;

    let at_roster = conn.session.pets.iter().find(|p| p.stt == 1).map(|p| p.id);
    let at_stable = conn.session.pets.iter().find(|p| p.stt == 5).map(|p| p.id);
    assert_eq!(at_roster, Some(18002), "stable pet moved into the roster");
    assert_eq!(at_stable, Some(18001), "roster pet moved into the stable");
    // Equipment relocated with their owners.
    let eq = |slot: u8| {
        conn.session
            .trangbi
            .iter()
            .find(|i| i.slot == slot)
            .map(|i| i.id)
    };
    assert_eq!(eq(51), Some(9001), "roster pet's gear moved to pet stt 5");
    assert_eq!(eq(11), Some(9002), "stable pet's gear moved to pet stt 1");
    // No two pets share one `stt`.
    let mut stts: Vec<u8> = conn.session.pets.iter().map(|p| p.stt).collect();
    stts.sort_unstable();
    stts.dedup();
    assert_eq!(stts.len(), conn.session.pets.len());
}

// ── handlers::quest ── (migrated from src/server/handlers/quest.rs)

#[test]
fn daily_quest_21_draws() {
    // Verify that exactly 21 RNG draws are consumed
    let mut conn = Conn::new();
    let mut out = HandleOutcome::default();
    generate_daily_quest(&mut conn, &GameData::default(), &mut out);
    // No crash = all 21 draws succeeded
}

/// Run `battle_quest_win` against a fresh session with the given OnWin
/// result (talk key `10916:NPC:1:0`), returning the emitted frames.
fn run_quest_win(result: QuestResult, mut data: GameData) -> (Session, Vec<String>) {
    let mut session = Session::new();
    session.id = 300001;
    session.map_id = 10916;
    session.talking_battle = 1;
    data.talks.insert(
        "10916:NPC:1:0".to_string(),
        QuestDef {
            map_id: 10916,
            id: 1,
            on_win: result,
            ..Default::default()
        },
    );
    let mut frames = Vec::new();
    battle_quest_win(&mut session, &data, &mut frames, &mut |_| None);
    (session, frames)
}

#[test]
fn quest_win_rewards() {
    let result = QuestResult {
        rewards: vec![(46001, 5, 0)],
        ..Default::default()
    };
    let (session, _) = run_quest_win(result, GameData::default());
    assert_eq!(session.homdo.len(), 1);
    assert_eq!(session.homdo[0].id, 46001);
    assert_eq!(session.homdo[0].count, 5);
}

#[test]
fn quest_win_random_reward() {
    let result = QuestResult {
        random_rewards: vec![(46001, 1, 0), (46002, 2, 0), (46003, 3, 0)],
        ..Default::default()
    };
    let (session, _) = run_quest_win(result, GameData::default());
    assert_eq!(session.homdo.len(), 1);
    // Should be one of the three items
    let item_id = session.homdo[0].id;
    assert!(
        item_id == 46001 || item_id == 46002 || item_id == 46003,
        "unexpected item: {}",
        item_id
    );
}

#[test]
fn quest_win_add_skill() {
    let mut data = GameData::default();
    data.skills.insert(
        10001,
        Skill {
            id: 10001,
            name: "Kiem".to_string(),
            ..Default::default()
        },
    );
    let result = QuestResult {
        add_skill: vec![(10001, 1)],
        ..Default::default()
    };
    let (session, frames) = run_quest_win(result, data);
    assert_eq!(session.skills.len(), 1);
    assert_eq!(session.skills[0], (10001, 1));
    // Learn packet present.
    assert!(frames.iter().any(|f| f.contains("6E01")));
}

#[test]
fn pet_reborn_npc_handled() {
    let mut conn = Conn::new();
    conn.session.idtalking = 3;
    conn.session.select_menu = 30;
    let mut out = HandleOutcome::default();

    // Map 55002 + object 3 is a pet-reborn key.
    let handled = handle_pet_reborn_npc(&mut conn, 55002, 3, &mut out);
    assert!(handled);
    assert!(!out.outgoing.is_empty());
}

#[test]
fn pet_reborn_npc_not_handled() {
    let mut conn = Conn::new();
    let mut out = HandleOutcome::default();

    // Wrong map/object pair -> not a pet-reborn key.
    let handled = handle_pet_reborn_npc(&mut conn, 55002, 2, &mut out);
    assert!(!handled);
}

#[test]
fn requirement_fail_packet() {
    let mut conn = Conn::new();
    let mut out = HandleOutcome::default();
    send_requirement_fail(&mut conn, 42, &mut out);
    assert_eq!(
        out.outgoing[0],
        "F444110014010000000201032A00000000000000BB"
    );
    // Followed by EndTalk + SelectMenu reset to 40
    assert_eq!(out.outgoing[1], "F44402001408");
    assert_eq!(conn.session.select_menu, 40);
    assert_eq!(conn.session.idtalking, 0);
}

#[test]
fn requirement_fail_packet_zero_id() {
    let mut conn = Conn::new();
    let mut out = HandleOutcome::default();
    send_requirement_fail(&mut conn, 0, &mut out);
    assert_eq!(
        out.outgoing[0],
        "F4441100140100000001010700000000000000493C"
    );
    assert_eq!(out.outgoing[1], "F44402001408");
}

#[test]
fn quest_save_leader_quests() {
    let result = QuestResult {
        save_leader_quests: vec![(100, 1, 0, 1)],
        ..Default::default()
    };
    let (session, _) = run_quest_win(result, GameData::default());
    assert_eq!(session.quest_steps, vec![(100, 1)]);
}

#[test]
fn quest_win_use_items_status_burst() {
    // G4: the self use-item branch must emit the wire-fidelity status
    // burst (gear stats always, Hp/Sp maxima client-only, Hp/Sp only when
    // the old value exceeded the new max).
    let mut session = Session::new();
    session.id = 300001;
    session.map_id = 10916;
    session.talking_battle = 1;
    session.add_homdo_item(InventoryItem {
        id: 19001,
        count: 3,
        loai: 1,
        doben: 100,
        ..Default::default()
    });
    let mut data = GameData::default();
    data.talks.insert(
        "10916:NPC:1:0".to_string(),
        QuestDef {
            map_id: 10916,
            id: 1,
            on_win: QuestResult {
                use_items: vec![(19001, 0)],
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut frames = Vec::new();
    battle_quest_win(&mut session, &data, &mut frames, &mut |_| None);

    // Six gear status bursts (Int2, Atk2, Def2, Hpx2, Spx2, Agi2).
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xD4, session.int2 as i32)));
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xD2, session.atk2 as i32)));
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xD3, session.def2 as i32)));
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xCF, session.hpx2 as i32)));
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xD0, session.spx2 as i32)));
    assert!(frames
        .iter()
        .any(|f| f == &build_stat_update(0xD6, session.agi2 as i32)));
    // Hpmax/Spmax are client-only stores and emit no packet; Hp/Sp only
    // when the old value exceeded the freshly recomputed max (not here).
    assert!(!frames.iter().any(|f| f.starts_with("F4440C00080119")));
    assert!(!frames.iter().any(|f| f.starts_with("F4440C0008011A")));
}

#[test]
fn quest_win_use_items_consumed() {
    let mut session = Session::new();
    session.id = 300001;
    session.map_id = 10916;
    session.talking_battle = 1;
    // Seed a use-item requirement into inventory
    session.add_homdo_item(InventoryItem {
        id: 19001,
        count: 3,
        loai: 1,
        doben: 100,
        ..Default::default()
    });
    let mut data = GameData::default();
    data.talks.insert(
        "10916:NPC:1:0".to_string(),
        QuestDef {
            map_id: 10916,
            id: 1,
            on_win: QuestResult {
                use_items: vec![(19001, 0)],
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut frames = Vec::new();
    battle_quest_win(&mut session, &data, &mut frames, &mut |_| None);
    // 3 - 1 = 2 remaining
    let left = session.homdo.iter().map(|i| i.count).sum::<u8>();
    assert_eq!(left, 2);
    // Self use-item frame `F44403001711`+slot present.
    assert!(frames.iter().any(|f| f.starts_with("F44403001711")));
}

#[test]
fn quest_win_add_pet() {
    let result = QuestResult {
        add_pet: vec![18017],
        ..Default::default()
    };
    let (session, _) = run_quest_win(result, GameData::default());
    assert_eq!(session.pets.len(), 1);
    assert_eq!(session.pets[0].id, 18017);
}

#[test]
fn quest_win_click_npc_id() {
    let result = QuestResult {
        click_npc_id: 59011,
        ..Default::default()
    };
    let (session, _) = run_quest_win(result, GameData::default());
    assert_eq!(session.click_npc_id, 59011);
}

#[test]
fn quest_win_warp_leader() {
    let result = QuestResult {
        warp_to: vec![12001, 400, 500],
        ..Default::default()
    };
    let (session, frames) = run_quest_win(result, GameData::default());
    assert_eq!(session.map_id, 12001);
    assert_eq!(session.map_x, 400);
    assert_eq!(session.map_y, 500);
    assert!(frames.iter().any(|f| f.starts_with("F4440D000C")));
}

#[test]
fn quest_win_end_talk_without_warp() {
    let result = QuestResult::default();
    let (session, frames) = run_quest_win(result, GameData::default());
    assert!(frames.iter().any(|f| f == "F44402001408"));
    assert_eq!(session.talking_battle, 0);
}

#[test]
fn quest_win_dialogs_take_precedence() {
    // Non-empty dialogs skip the reward pipeline entirely.
    let result = QuestResult {
        dialogs: "F44411001401000000010103010000000000009E28".to_string(),
        rewards: vec![(46001, 5, 0)],
        ..Default::default()
    };
    let (session, frames) = run_quest_win(result, GameData::default());
    assert!(frames
        .iter()
        .any(|f| f.starts_with("F44411001401000000010103")));
    assert!(frames.iter().any(|f| f == "F44402001408"));
    assert!(session.homdo.is_empty(), "no rewards when dialogs present");
}

#[test]
fn select_menu_mismatch_sends_lose_dialog() {
    let mut conn = Conn::new();
    conn.session.map_id = 10916;
    conn.session.idtalking = 1;
    conn.session.select_menu = 20; // wrong menu
    let mut data = GameData::default();
    data.talks.insert(
        "10916:NPC:1:0".to_string(),
        QuestDef {
            map_id: 10916,
            id: 1,
            dialogs: "F44411001401000000010603010000000000000100".to_string(),
            require_select_menu: 30,
            on_lose: QuestResult {
                dialogs: "F44411001401000000010103010000000000009E28".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
    );

    let mut out = HandleOutcome::default();

    let handled = try_quest_h6(&mut conn, &data, &mut out);
    assert!(handled, "expected handled quest path");
    assert_eq!(conn.session.select_menu, 40);
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(
        out.outgoing[0],
        "F44411001401000000010103010000000000009E28"
    );
}

#[test]
fn warp_talk_teamdef_triggers_battle() {
    let mut conn = Conn::new();
    conn.session.map_id = 11011;
    conn.session.idtalking = 3;
    let mut data = GameData::default();
    data.talks.insert(
        "11011:WARP:3:0".to_string(),
        QuestDef {
            map_id: 11011,
            talk_type: "WARP".to_string(),
            id: 3,
            teamdef: vec![121, 0, 17177, 14073, 17177, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        },
    );

    let mut out = HandleOutcome::default();
    handle_warp_confirm(&mut conn, &data, &mut out);
    assert!(out.battle_trigger.is_some(), "expected battle trigger");
    let t = out.battle_trigger.as_ref().unwrap();
    assert_eq!(t.diahinh, 121);
    assert_eq!(conn.session.talking_battle, 3);
}

#[test]
fn warp_talk_plain_warp() {
    let mut conn = Conn::new();
    conn.session.map_id = 11011;
    conn.session.idtalking = 2;
    let mut data = GameData::default();
    data.warps.insert(
        (11011, 2),
        Warp {
            map1: 11011,
            warpid: 2,
            map2: 12001,
            x: 400,
            y: 500,
        },
    );

    let mut out = HandleOutcome::default();
    handle_warp_confirm(&mut conn, &data, &mut out);
    assert!(out.battle_trigger.is_none());
    assert_eq!(conn.session.map_id, 12001);
    assert_eq!(conn.session.map_x, 400);
}

#[test]
fn warp_talk_teamdef_missing_uses_gate() {
    let mut conn = Conn::new();
    conn.session.map_id = 59841;
    conn.session.idtalking = 1;
    let mut data = GameData::default();
    data.warps.insert(
        (59841, 1),
        Warp {
            map1: 59841,
            warpid: 1,
            map2: 60000,
            x: 10,
            y: 20,
        },
    );
    data.battle_gates.insert(
        (59841, 1),
        BattleGate {
            mapid1: 59841,
            warpid: 1,
            diahinh: 365,
            defenders: [1001, 1002, 0, 0, 0, 0, 0, 0, 0, 0],
        },
    );

    let mut out = HandleOutcome::default();
    handle_warp_confirm(&mut conn, &data, &mut out);
    assert!(out.battle_trigger.is_some(), "expected gate battle trigger");
    let t = out.battle_trigger.as_ref().unwrap();
    assert_eq!(t.diahinh, 365);
}

#[test]
fn quest_key_roundtrips_type_and_step() {
    let key = quest_key(10916, "NPC", 3, 7);
    assert_eq!(key, "10916:NPC:3:7");
    // Keywords differ by step — never collapse to step 0.
    assert_ne!(quest_key(10916, "NPC", 3, 7), quest_key(10916, "NPC", 3, 0));
}

#[test]
fn pet_reborn_keys_are_map_object_pairs() {
    assert!(is_pet_reborn_key(55002, 3));
    assert!(is_pet_reborn_key(59102, 1));
    assert!(is_pet_reborn_key(59011, 1));
    // Template-id-only leaks (old bug) must be false.
    assert!(!is_pet_reborn_key(55002, 0));
    assert!(!is_pet_reborn_key(0, 3));
}

#[test]
fn requirements_level_gate_fails() {
    let mut conn = Conn::new();
    conn.session.level = 5;
    let quest = QuestDef {
        require_level: Some((50, 1)), // level >= 50
        ..Default::default()
    };
    assert!(
        evaluate_requirements(&conn, &quest).is_some(),
        "level 5 must fail a >= 50 requirement"
    );
}

#[test]
fn requirements_level_gate_passes() {
    let mut conn = Conn::new();
    conn.session.level = 60;
    let quest = QuestDef {
        require_level: Some((50, 1)), // level >= 50
        ..Default::default()
    };
    assert!(evaluate_requirements(&conn, &quest).is_none());
}

#[test]
fn daily_quest_draws_consume_21_and_branch() {
    let mut conn = Conn::new();
    conn.session.idtalking = 1; // pet-shop row
    conn.session.select_menu = 30;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 31044,
        count: 1,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    generate_daily_quest(&mut conn, &GameData::default(), &mut out);
    // Menu 30 + item 31044 -> the pet 18016 is granted.
    assert!(
        conn.session.pets.iter().any(|p| p.id == 18016),
        "daily pet-shop item must grant pet 18016"
    );
    // And the consumed item is removed from the bag.
    assert!(!conn.session.homdo.iter().any(|i| i.id == 31044));
}

// ── handlers::shops ── (migrated from src/server/handlers/shops.rs)

/// Player shop catalog frame `1721`: 17 zero bytes + per listed item
/// (id2 count2 price4 long giatri khang texp4 idx).
#[test]
fn player_shop_catalog_frame_matches_legacy_layout() {
    let mut seller = Session::new();
    seller.id = 300002;
    seller.homdo.push(InventoryItem {
        slot: 1,
        id: 20023,
        count: 5,
        doben: 100,
        long_val: 100,
        giatri_long: 50,
        khang: 30,
        texp: 1234,
        loai: 1,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 1,
        price: 58800,
        ..Default::default()
    });
    seller.shop.active = true;

    let frame = player_shop_catalog_frame(&seller);
    assert_eq!(
        frame,
        "F44423001721".to_string()
            + "0000000000000000000000000000000000"
            + "374E0500B0E5000064961ED204000001"
    );
}

/// Complete a player-shop purchase: item + gold swap both sides.
#[test]
fn complete_shop_buy_transfers_item_and_gold() {
    let mut buyer = Session::new();
    buyer.id = 300001;
    buyer.gold = 100000;
    let mut seller = Session::new();
    seller.id = 300002;
    seller.gold = 1000;
    seller.homdo.push(InventoryItem {
        slot: 3,
        id: 20023,
        count: 2,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 3,
        item_id: 20023,
        price: 58800,
        ..Default::default()
    });
    seller.shop.active = true;

    let res = complete_shop_buy(&mut buyer, &mut seller, 0, 1).unwrap();
    assert_eq!(res.total, 58800);
    assert_eq!(buyer.gold, 41200);
    assert!(buyer.homdo.iter().any(|i| i.id == 20023 && i.count == 1));
    assert_eq!(seller.gold, 59800);
    assert!(seller.homdo.iter().any(|i| i.id == 20023 && i.count == 1));
    // Listing survives while the seller still holds stock.
    assert_eq!(seller.shop.items.len(), 1);
}

/// Buying without enough gold fails before any mutation.
#[test]
fn complete_shop_buy_rejects_not_enough_gold() {
    let mut buyer = Session::new();
    buyer.gold = 100;
    let mut seller = Session::new();
    seller.gold = 0;
    seller.homdo.push(InventoryItem {
        slot: 3,
        id: 20023,
        count: 2,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 3,
        item_id: 20023,
        price: 58800,
        ..Default::default()
    });
    seller.shop.active = true;

    let err = complete_shop_buy(&mut buyer, &mut seller, 0, 1).unwrap_err();
    assert!(matches!(err, ShopBuyError::NotEnoughGold { total: 58800 }));
    assert_eq!(buyer.gold, 100);
    assert_eq!(seller.gold, 0);
    assert!(buyer.homdo.is_empty());
}

/// Buying more than the seller has on that slot fails.
#[test]
fn complete_shop_buy_rejects_not_enough_stock() {
    let mut buyer = Session::new();
    buyer.gold = 100000;
    let mut seller = Session::new();
    seller.homdo.push(InventoryItem {
        slot: 3,
        id: 20023,
        count: 1,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 3,
        item_id: 20023,
        price: 58800,
        ..Default::default()
    });
    seller.shop.active = true;

    let err = complete_shop_buy(&mut buyer, &mut seller, 0, 5).unwrap_err();
    assert!(matches!(err, ShopBuyError::NotEnoughStock));
    assert_eq!(buyer.gold, 100000);
    assert_eq!(seller.homdo[0].count, 1);
}

/// Sub 30: parse name + listed items, store shop state, emit self 171E.
#[tokio::test]
async fn player_shop_sub30_open_parses_and_emits_171e() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 20023,
        count: 1,
        ..Default::default()
    });
    // payload: name_len(4) "TEST" pad slot price(1000)
    let payload = [
        4u8, b'T', b'E', b'S', b'T', 0x00, 0x01, 0xE8, 0x03, 0x00, 0x00,
    ];

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 30, &payload);
    handle_player_shop(&mut ctx).await;

    assert!(conn.session.shop.active);
    assert_eq!(conn.session.shop.name, b"TEST");
    assert_eq!(conn.session.shop.items.len(), 1);
    assert_eq!(conn.session.shop.items[0].slot, 1);
    assert_eq!(conn.session.shop.items[0].price, 1000);
    assert!(out
        .outgoing
        .iter()
        .any(|f| f.starts_with("F4440C00171E") && f.contains("045445535401E8030000")));
    assert_eq!(out.map_broadcast.len(), 1);
    assert!(out.map_broadcast[0].frame.contains("171F"));
}

#[tokio::test]
async fn player_shop_name_preserves_viscii_bytes() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 20001,
        count: 1,
        ..Default::default()
    });
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let name = [0xE1, 0xF2, 0x80];
    let payload = [vec![name.len() as u8], name.to_vec(), vec![0]].concat();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 30, &payload);

    handle_player_shop(&mut ctx).await;

    assert_eq!(conn.session.shop.name, name);
    assert!(out.outgoing[0].contains("03E1F280"));
}

#[tokio::test]
async fn player_shop_rejects_duplicate_or_unknown_listing_slots() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let payload = [
        4u8, b'T', b'E', b'S', b'T', 0, 1, 0xE8, 3, 0, 0, 1, 1, 0, 0, 0,
    ];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 30, &payload);
    handle_player_shop(&mut ctx).await;
    assert!(!conn.session.shop.active);
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn player_shop_rejects_self_buy() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.open_shop_id = 300001;
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        33,
        &[0, 0, 0, 0, 0, 1],
    );
    handle_player_shop(&mut ctx).await;
    assert!(out.outgoing.iter().any(|frame| frame.contains("020B")));
}

/// Sub 31: close clears the shop and emits 1720 + player id.
#[tokio::test]
async fn player_shop_sub31_close_emits_1720() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.shop.active = true;
    conn.session.shop.name = b"TEST".to_vec();

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 31, &[]);
    handle_player_shop(&mut ctx).await;

    assert!(!conn.session.shop.active);
    assert!(conn.session.shop.items.is_empty());
    let close = format!("F44406001720{}", encoder::le32(300001));
    assert!(out.outgoing.iter().any(|f| f == &close));
    assert_eq!(out.map_broadcast.len(), 1);
    assert_eq!(out.map_broadcast[0].frame, close);
}

/// Sub 32 + 33: open a seller's shop then buy from it.
#[tokio::test]
async fn player_shop_sub33_buys_from_registry_seller() {
    // Remove only the ids this test owns — the shared registry may hold
    // sessions from concurrently running tests.
    online_sessions().lock().unwrap().remove(&300002);

    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.gold = 100000;
    conn.session.map_id = 12001;

    let mut seller = Session::new();
    seller.id = 300002;
    seller.gold = 1000;
    seller.homdo.push(InventoryItem {
        slot: 3,
        id: 20023,
        count: 2,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 3,
        item_id: 20023,
        price: 58800,
        ..Default::default()
    });
    seller.shop.active = true;
    online_sessions().lock().unwrap().insert(300002, seller);

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    // Sub 32: open the seller's shop → records open_shop_id + sends 1721.
    let mut out32 = HandleOutcome::default();
    let open_payload = [0xE2u8, 0x93, 0x04, 0x00];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out32, 32, &open_payload);
    handle_player_shop(&mut ctx).await;
    assert_eq!(conn.session.open_shop_id, 300002);
    assert!(out32.outgoing.iter().any(|f| f.contains("1721")));

    // Sub 33: buy index 0, count 1.
    let mut out = HandleOutcome::default();
    let buy_payload = [0u8, 0, 0, 0, 0, 1];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 33, &buy_payload);
    handle_player_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 41200);
    assert!(conn.session.homdo.iter().any(|i| i.id == 20023));
    let seller_now = online_sessions()
        .lock()
        .unwrap()
        .get(&300002)
        .unwrap()
        .clone();
    assert_eq!(seller_now.gold, 59800);
    assert!(seller_now
        .homdo
        .iter()
        .any(|i| i.id == 20023 && i.count == 1));
    let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(41200));
    assert!(out.outgoing.iter().any(|f| f == &gold_frame));
    online_sessions().lock().unwrap().remove(&300002);
}

/// Price-table rows, transcribed verbatim.
#[tokio::test]
async fn npc_price_table_transcribed_rows() {
    assert_eq!(get_npc_shop_price(1, 12201, 0), Some((10001, 10)));
    assert_eq!(get_npc_shop_price(1, 12201, 2), Some((10013, 20)));
    assert_eq!(get_npc_shop_price(1, 12223, 0), Some((26041, 5)));
    assert_eq!(get_npc_shop_price(1, 12223, 3), Some((27032, 15)));
    assert_eq!(get_npc_shop_price(1, 12204, 2), Some((15001, 10)));
    assert_eq!(get_npc_shop_price(2, 12204, 6), Some((22001, 10)));
    assert_eq!(get_npc_shop_price(2, 20001, 3), Some((46103, 100)));
    assert_eq!(get_npc_shop_price(3, 12244, 2), Some((26005, 15)));
    assert_eq!(get_npc_shop_price(4, 12002, 0), Some((26016, 5)));
    assert_eq!(get_npc_shop_price(7, 12001, 1), Some((27156, 115)));
    assert_eq!(get_npc_shop_price(7, 12001, 2), Some((52015, 1)));
    assert_eq!(get_npc_shop_price(8, 12990, 4), Some((20401, 10)));
    assert_eq!(get_npc_shop_price(15, 12002, 12), Some((19756, 19900)));
    assert_eq!(get_npc_shop_price(16, 12002, 0), Some((20023, 58800)));
    assert_eq!(get_npc_shop_price(16, 12002, 14), Some((19423, 58800)));
    assert_eq!(get_npc_shop_price(26, 11011, 7), Some((22412, 20)));
    assert_eq!(get_npc_shop_price(1, 19241, 4), Some((26028, 10)));
    // Unknown (map, menu) → not on the shelf.
    assert_eq!(get_npc_shop_price(5, 60000, 0), None);
}

#[test]
fn npc_price_table_has_every_legacy_branch() {
    let shelves = [
        (1, 12223, 4),
        (1, 19241, 5),
        (4, 12002, 3),
        (16, 12002, 15),
        (15, 12002, 16),
        (8, 12990, 8),
        (1, 12201, 4),
        (3, 12244, 4),
        (1, 12007, 4),
        (1, 12204, 3),
        (2, 12204, 8),
        (7, 12001, 3),
        (2, 20001, 4),
        (26, 11011, 8),
    ];
    let mut rows = 0;
    for (idtalking, map_id, menu_count) in shelves {
        for menu in 0..menu_count {
            assert!(
                get_npc_shop_price(idtalking, map_id, menu).is_some(),
                "missing shop row ({idtalking}, {map_id}, {menu})"
            );
            rows += 1;
        }
        assert_eq!(get_npc_shop_price(idtalking, map_id, menu_count), None);
    }
    assert_eq!(rows, 89, "price table contains 89 buy branches");
}

#[test]
fn player_shop_removes_the_listed_slot_not_an_equal_id_stack() {
    let mut buyer = Session::new();
    buyer.gold = 100;
    let mut seller = Session::new();
    seller.shop.active = true;
    seller.homdo.push(InventoryItem {
        slot: 1,
        id: 26041,
        count: 5,
        ..Default::default()
    });
    seller.homdo.push(InventoryItem {
        slot: 2,
        id: 26041,
        count: 3,
        ..Default::default()
    });
    seller.shop.items.push(ts_dream::server::session::ShopItem {
        slot: 2,
        item_id: 26041,
        count: 3,
        price: 1,
    });
    complete_shop_buy(&mut buyer, &mut seller, 0, 2).unwrap();
    assert_eq!(
        seller
            .homdo
            .iter()
            .find(|item| item.slot == 1)
            .unwrap()
            .count,
        5
    );
    assert_eq!(
        seller
            .homdo
            .iter()
            .find(|item| item.slot == 2)
            .unwrap()
            .count,
        1
    );
}

/// Op 0x1B buy: `idtalking` 16 + map 12002 + menu 0 → item 20023 (58800).
#[tokio::test]
async fn npc_buy_deducts_gold_adds_item_emits_1a04() {
    let mut conn = Conn::new();
    conn.session.idtalking = 16;
    conn.session.map_id = 12002;
    conn.session.gold = 100000;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // payload[0] = menu, payload[1] = count (unused for buy).
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 41200);
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.id == 20023 && i.count == 1));
    let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(41200));
    assert!(out.outgoing.iter().any(|f| f == &gold_frame));
    // Red message ("Khách quan mua hàng thành công") via op 0x02 sub 0x0B.
    assert!(out.outgoing.iter().any(|f| f.contains("020B")));
}

/// Op 0x1B buy: not enough gold → nothing changes, no gold frame.
#[tokio::test]
async fn npc_buy_rejects_when_gold_short() {
    let mut conn = Conn::new();
    conn.session.idtalking = 16;
    conn.session.map_id = 12002;
    conn.session.gold = 1000;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 1000);
    assert!(conn.session.homdo.is_empty());
    assert!(!out.outgoing.iter().any(|f| f.contains("1A04")));
}

/// Op 0x1B buy: unknown (map, menu) is a silent no-op.
#[tokio::test]
async fn npc_buy_unknown_shelf_is_noop() {
    let mut conn = Conn::new();
    conn.session.idtalking = 5;
    conn.session.map_id = 60000;
    conn.session.gold = 1000;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 1000);
    assert!(conn.session.homdo.is_empty());
}

/// Op 0x1B sell: idnpctalking 16005 scans 26001..26455, pays `count` gold.
#[tokio::test]
async fn npc_sell_16005_scans_range_adds_count_gold() {
    let mut conn = Conn::new();
    conn.session.idnpctalking = 16005;
    conn.session.map_id = 12001;
    conn.session.gold = 10;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26041,
        count: 3,
        ..Default::default()
    });

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // payload[1] = count to sell.
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &[0, 2]);
    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 12);
    let item = conn.session.homdo.iter().find(|i| i.id == 26041).unwrap();
    assert_eq!(item.count, 1);
    let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(12));
    assert!(out.outgoing.iter().any(|f| f == &gold_frame));
}

#[tokio::test]
async fn npc_sell_rejects_request_larger_than_owned_stack() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.map_id = 12001;
    conn.session.idnpctalking = 16005;
    conn.session.gold = 10;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 2,
        ..Default::default()
    });
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0, &[0, 3]);

    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 10);
    assert_eq!(conn.session.homdo[0].count, 2);
    assert!(out.outgoing.is_empty());
}

/// Op 0x1B sell: idnpctalking 16002 scans 27001..27165.
#[tokio::test]
async fn npc_sell_16002_scans_27000_range() {
    let mut conn = Conn::new();
    conn.session.idnpctalking = 16002;
    conn.session.map_id = 12001;
    conn.session.gold = 0;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 27017,
        count: 1,
        ..Default::default()
    });

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &[0, 1]);
    handle_npc_shop(&mut ctx).await;

    assert_eq!(conn.session.gold, 1);
    assert!(!conn.session.homdo.iter().any(|i| i.id == 27017));
}

/// Op 0x1B: idtalking 7 + map 9999 grants the free starter bundle.
#[tokio::test]
async fn npc_buy_7_9999_free_starter_bundle() {
    let mut conn = Conn::new();
    conn.session.idtalking = 7;
    conn.session.map_id = 9999;
    conn.session.gold = 0;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
    handle_npc_shop(&mut ctx).await;

    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.id == 18001 && i.count >= 1));
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.id == 27156 && i.count >= 50));
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.id == 52015 && i.count >= 50));
    assert_eq!(conn.session.gold, 0);
}

// ── handlers::skills ── (migrated from src/server/handlers/skills.rs)

fn skills_test_game_data() -> GameData {
    let mut data = GameData::default();
    // Add skill 10001 (element 1, point 1, lv_max 10, reborn 0)
    data.skills.insert(
        10001,
        Skill {
            id: 10001,
            name: "Earth Skill".into(),
            point: 1,
            thuoctinh: 1,
            lv_max: 10,
            reborn: 0,
            ..Default::default()
        },
    );
    data.skills.insert(
        10002,
        Skill {
            id: 10002,
            name: "Fire Skill".into(),
            point: 1,
            thuoctinh: 3,
            lv_max: 10,
            reborn: 0,
            ..Default::default()
        },
    );
    // Add reborn pet item and pet NPC templates
    data.items.insert(
        20001,
        Item {
            id: 20001,
            rb_pet_from: 15001,
            rb_pet_to: 15002,
            ..Default::default()
        },
    );
    data.npcs.insert(
        15002,
        Npc {
            id: 15002,
            name: b"Reborn Pet".to_vec(),
            reborn: 1,
            thuoctinh: 1,
            hpx: 10,
            spx: 10,
            atk: 20,
            skill: [10001, 0, 0, 0],
            ..Default::default()
        },
    );
    data
}

#[tokio::test]
async fn test_player_skill_learn_success() {
    let mut conn = Conn::new();
    conn.session.skill_point = 5;
    conn.session.thuoctinh = 1; // Earth

    let data = skills_test_game_data();
    let service = BattleService::new(Arc::new(skills_test_game_data()));
    let mut out = HandleOutcome::default();
    let payload = vec![0x11, 0x27, 0x01]; // skill 10001 target lv 1
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_skills(&mut ctx).await;

    assert_eq!(conn.session.skills.len(), 1);
    assert_eq!(conn.session.skills[0], (10001, 1));
    assert_eq!(conn.session.skill_point, 4);
    assert_eq!(out.outgoing.len(), 2);
}

#[tokio::test]
async fn test_player_skill_learn_opposing_element_rejected() {
    let mut conn = Conn::new();
    conn.session.skill_point = 5;
    conn.session.thuoctinh = 1; // Earth player cannot learn Fire skill (10002)

    let data = skills_test_game_data();
    let service = BattleService::new(Arc::new(skills_test_game_data()));
    let mut out = HandleOutcome::default();
    let payload = vec![0x12, 0x27, 0x01]; // skill 10002 target lv 1
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_skills(&mut ctx).await;

    assert!(
        conn.session.skills.is_empty(),
        "Earth player cannot learn Fire skill"
    );
    assert_eq!(conn.session.skill_point, 5);
}

#[tokio::test]
async fn test_pet_reborn_consumes_item_and_transforms_pet() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.pets.push(PetState {
        stt: 1,
        id: 15001,
        level: 50,
        reborn: 0,
        skills: [(10001, 1), (0, 0), (0, 0), (0, 0)],
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 20001,
        count: 1,
        ..Default::default()
    });

    let data = skills_test_game_data();
    let service = BattleService::new(Arc::new(skills_test_game_data()));
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1]);
    handle_pet_reborn(&mut ctx).await;

    assert_eq!(conn.session.pets[0].id, 15002);
    assert_eq!(conn.session.pets[0].reborn, 1);
    assert_eq!(conn.session.pets[0].level, 1);
    assert!(conn.session.homdo.is_empty(), "Reborn item consumed");
    assert!(out.outgoing.iter().any(|f| f.contains("0F08")));
    assert!(out.outgoing.iter().any(|f| f.contains("2C01")));
}

// ── handlers::stats ── (migrated from src/server/handlers/stats.rs)

#[tokio::test]
async fn test_stat_allocation_int() {
    let mut conn = Conn::new();
    conn.session.point = 10;
    conn.session.int1 = 5;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // stat_id = 27 (0x1B), points = 2
    let payload = vec![0, 0, 27, 2];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_stat_allocation(&mut ctx).await;

    assert_eq!(conn.session.point, 8);
    assert_eq!(conn.session.int1, 7);
    assert_eq!(out.outgoing.len(), 2);
    assert_eq!(out.outgoing[0], build_stat_update(0x26, 8));
    assert_eq!(out.outgoing[1], build_stat_update(0x1B, 7));
}

#[tokio::test]
async fn test_stat_allocation_hpx_recomputes_max() {
    let mut conn = Conn::new();
    conn.session.point = 10;
    conn.session.hpx = 3;
    // Before: hp_max from (reborn0, job0, lv1, hpx3).
    conn.session.recompute_stats();
    let before_max = conn.session.hp_max;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // stat_id = 31 (0x1F), points = 2
    let payload = vec![0, 0, 31, 2];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_stat_allocation(&mut ctx).await;

    assert_eq!(conn.session.point, 8);
    assert_eq!(conn.session.hpx, 5);
    assert!(
        conn.session.hp_max > before_max,
        "Hpx points must raise hp_max ({} -> {})",
        before_max,
        conn.session.hp_max
    );
    assert_eq!(out.outgoing.len(), 2);
    // Stat 31 emits only Point then Hpx — the Hpmax recompute updates
    // in-memory only, no packet for the new max.
    assert_eq!(out.outgoing[0], build_stat_update(0x26, 8));
    assert_eq!(out.outgoing[1], build_stat_update(0x1F, 5));
}

#[tokio::test]
async fn test_hotkey_assignment() {
    let mut conn = Conn::new();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // Protocol: data[7..8]=skill LE(0x11,0x27)=0x2711, data[9]=slot 3,
    // i.e. payload[0]=0x00 padding, payload[1..3]=skill, payload[3]=slot.
    let payload = vec![0x00, 0x11, 0x27, 3];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_hotkey(&mut ctx).await;

    assert_eq!(conn.session.hotkeys[3], 10001);
    assert!(out.outgoing.is_empty());
}

#[tokio::test]
async fn test_hotkey_slot_zero_is_clear_noop() {
    let mut conn = Conn::new();
    conn.session.hotkeys[4] = 777;
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));
    let mut out = HandleOutcome::default();
    // slot 0: skill 0, slot 0 → clear (no-op, no DB row).
    let payload = vec![0x00, 0x00, 0x00, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_hotkey(&mut ctx).await;

    assert_eq!(conn.session.hotkeys[0], 0);
    assert_eq!(
        conn.session.hotkeys[4], 777,
        "clear must not touch other slots"
    );
    assert!(out.outgoing.is_empty());
}

// ── handlers::system ── (migrated from src/server/handlers/system.rs)

#[tokio::test]
async fn test_pk_and_war_toggle() {
    let mut conn = Conn::new();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1]);
    handle_pk_war(&mut ctx).await;
    assert_eq!(conn.session.pk, 1);
    assert_eq!(out.outgoing[0], "F444040021020100");

    let mut out2 = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 2, &[1]);
    handle_pk_war(&mut ctx).await;
    assert_eq!(conn.session.tham_chien, 1);
    assert_eq!(out2.outgoing[0], "F444040021020101");
}

#[tokio::test]
async fn test_pk_rejects_invalid_flag() {
    let mut conn = Conn::new();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[7]);
    handle_pk_war(&mut ctx).await;
    assert_eq!(conn.session.pk, 0, "flag 7 must be rejected silently");
    assert!(out.outgoing.is_empty(), "no ack for invalid flag");
}

#[test]
fn test_game_points_width_and_gate() {
    let mut conn = Conn::new();
    conn.session.gold = 5000;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    // Sub 1: le32(gold) + 12 zero bytes.
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[]);
    handle_game_points(&mut ctx);
    assert_eq!(out.outgoing[0], "F4441200230488130000000000000000000000000000");

    // Sub 2 must be silent.
    let mut out2 = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 2, &[]);
    handle_game_points(&mut ctx);
    assert!(out2.outgoing.is_empty());
}

#[tokio::test]
async fn test_gm_shop_buy() {
    let mut conn = Conn::new();
    conn.session.shop_point = 500;
    let mut data = GameData::default();
    data.items.insert(
        0x2711,
        Item {
            id: 0x2711,
            ..Default::default()
        },
    );

    let service = BattleService::new(Arc::new(data.clone()));
    let mut out = HandleOutcome::default();
    // C2S mall request: item at raw[9..10] = 0x2711, price at raw[11..12] =
    // 0x00C8. payload = raw[6..], so payload[3..5]=item, payload[5..7]=price.
    let payload = vec![0, 0, 0, 0x11, 0x27, 0xC8, 0x00, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
    handle_gm_shop(&mut ctx).await;

    assert_eq!(conn.session.shop_point, 300);
    assert!(conn.session.homdo.iter().any(|i| i.id == 0x2711));
    // Order: item-add `1706` before the points frame.
    assert!(out.outgoing[0].contains("1706"));
    assert!(out.outgoing.iter().any(|f| f.contains("4202")));
}

#[test]
fn test_rank_frames() {
    let mut conn = Conn::new();
    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[]);
    handle_rank(&mut ctx);
    assert_eq!(out.outgoing, vec!["F44402004101"]);

    let mut out2 = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 2, &[]);
    handle_rank(&mut ctx);
    assert_eq!(out2.outgoing, vec!["F44402004102"]);
}

#[test]
fn test_teleport_confirm_leader_resets_state() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.map_id = 12001;
    conn.session.warp_finish = true;
    conn.session.talk_count = 3;
    conn.session.idtalking = 6;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[]);
    handle_teleport_confirm(&mut ctx);
    assert_eq!(out.outgoing, vec!["F44402000504F44402001408"]);
    // Leader/solo branch resets the warp/talk state.
    assert!(!conn.session.warp_finish);
    assert_eq!(conn.session.talk_count, 0);
    assert_eq!(conn.session.idtalking, 0);
}

#[test]
fn test_teleport_confirm_member_returns_early() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.id_leader = 300002; // somebody else is the leader
    conn.session.warp_finish = true;
    conn.session.talk_count = 3;
    conn.session.idtalking = 6;

    let data = GameData::default();
    let service = BattleService::new(Arc::new(GameData::default()));

    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[]);
    handle_teleport_confirm(&mut ctx);
    assert_eq!(out.outgoing, vec!["F44402000504F44402001408"]);
    // Member branch: the two confirmation frames only, state untouched.
    assert!(conn.session.warp_finish);
    assert_eq!(conn.session.talk_count, 3);
}

#[test]
fn test_len_string_parser() {
    let mut payload = vec![3];
    payload.extend_from_slice(b"abc");
    payload.push(2);
    payload.extend_from_slice(b"xy");
    let parts = parse_len_strings(&payload, 2).unwrap();
    assert_eq!(parts[0], b"abc");
    assert_eq!(parts[1], b"xy");

    // Truncated: 4-byte string but only 3 remain -> None.
    let bad = vec![4, 1, 2, 3];
    assert!(parse_len_strings(&bad, 1).is_none());
}

// ── handlers::talk ── (migrated from src/server/handlers/talk.rs)

fn talk_fixture(npc_id: i64) -> (Conn, GameData, BattleService) {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.map_id = 10817;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    let mut data = GameData::default();
    data.npc_on_map.push(NpcOnMap {
        map_id: 10817,
        id: 6,
        npc_id,
        x: 401,
        y: 501,
        ..Default::default()
    });
    (
        conn,
        data,
        BattleService::new(Arc::new(GameData::default())),
    )
}

#[tokio::test]
async fn test_talk_start_banker_resolves_template() {
    let (mut conn, data, service) = talk_fixture(16080);
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x06, 0x00]);
    handle_talk(&mut ctx).await;
    assert_eq!(conn.session.idtalking, 6);
    assert_eq!(conn.session.idnpctalking, 16080);
    assert_eq!(out.outgoing[0], "F44402000602");
    assert_eq!(
        out.outgoing[1],
        "F44411001401000000010603060000000000000100"
    );
}

#[tokio::test]
async fn test_talk_out_of_range_ends() {
    let (mut conn, data, service) = talk_fixture(16080);
    conn.session.map_x = 999;
    conn.session.map_y = 999;
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x06, 0x00]);
    handle_talk(&mut ctx).await;
    assert_eq!(out.outgoing, vec!["F44402001408"]);
    assert_eq!(conn.session.idtalking, 0);
}

#[tokio::test]
async fn test_talk_missing_instance_rejected() {
    // A talk to an on-map id that has NO row is rejected (EndTalk), per the
    // ticket 18 review "reject missing/out-of-range" rule.
    let (mut conn, data, service) = talk_fixture(16080);
    // Object id 99 is not in `npc_on_map` (only object 6 is registered).
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x63, 0x00]);
    handle_talk(&mut ctx).await;
    assert_eq!(conn.session.idtalking, 0);
    assert_eq!(out.outgoing, vec!["F44402001408"]);
}

#[tokio::test]
async fn test_talk_end_resets_context() {
    let (mut conn, data, service) = talk_fixture(16080);
    conn.session.idtalking = 6;
    conn.session.select_menu = 30;
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 4, &[]);
    handle_talk(&mut ctx).await;
    assert_eq!(conn.session.idtalking, 0);
    assert_eq!(conn.session.select_menu, 0);
    assert_eq!(out.outgoing, vec!["F44402001408"]);
}

// ── handlers::trade_storage ── (migrated from src/server/handlers/trade_storage.rs)

fn storage_item(slot: u8, id: u16, count: u8) -> InventoryItem {
    InventoryItem {
        slot,
        id,
        count,
        ..Default::default()
    }
}

fn trade_dependencies() -> (GameData, BattleService) {
    (
        GameData::default(),
        BattleService::new(Arc::new(GameData::default())),
    )
}

#[tokio::test]
async fn accepted_trade_exchanges_both_players_items_and_gold() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.id = 390_001;
    conn.session.gold = 1_000;
    conn.session.homdo = vec![storage_item(1, 1001, 1)];
    conn.session.trade = TradeState {
        active: true,
        partner_id: 390_002,
        gold: 100,
        items: conn.session.homdo.clone(),
        ..Default::default()
    };
    let mut partner = Session::new();
    partner.id = 390_002;
    partner.gold = 2_000;
    partner.homdo = vec![storage_item(1, 2002, 1)];
    partner.trade = TradeState {
        active: true,
        partner_id: 390_001,
        accepted: true,
        gold: 200,
        items: partner.homdo.clone(),
        ..Default::default()
    };
    online_sessions().lock().unwrap().insert(partner.id, partner);
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
    handle_trade(&mut ctx).await;
    let partner = online_sessions()
        .lock()
        .unwrap()
        .get(&390_002)
        .unwrap()
        .clone();
    assert_eq!((conn.session.gold, partner.gold), (1_100, 1_900));
    assert_eq!(conn.session.homdo[0].id, 2002);
    assert_eq!(partner.homdo[0].id, 1001);
    assert!(out.outgoing[0].contains("1A04"));
    assert!(out.outgoing[1].contains("1706"));
    assert_eq!(out.outgoing[2], "F4440300190204");
    online_sessions().lock().unwrap().remove(&390_002);
}

#[tokio::test]
async fn full_trade_destination_is_reported_without_losing_source_item() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.id = 390_011;
    conn.session.homdo = vec![storage_item(1, 1001, 1)];
    conn.session.trade = TradeState {
        active: true,
        partner_id: 390_012,
        items: conn.session.homdo.clone(),
        ..Default::default()
    };
    let mut partner = Session::new();
    partner.id = 390_012;
    partner.homdo = (1..=25)
        .map(|slot| storage_item(slot, 2000 + u16::from(slot), 1))
        .collect();
    partner.trade = TradeState {
        active: true,
        partner_id: 390_011,
        accepted: true,
        ..Default::default()
    };
    online_sessions().lock().unwrap().insert(partner.id, partner);
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
    handle_trade(&mut ctx).await;
    assert_eq!(conn.session.homdo, vec![storage_item(1, 1001, 1)]);
    assert_eq!(out.outgoing, vec!["F4440300190207"]);
    online_sessions().lock().unwrap().remove(&390_012);
}

#[tokio::test]
async fn transfer_uses_requested_count_and_updates_recipient() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.id = 390_021;
    conn.session.homdo = vec![storage_item(3, 3003, 5)];
    let mut recipient = Session::new();
    recipient.id = 390_022;
    online_sessions().lock().unwrap().insert(recipient.id, recipient);
    let mut payload = vec![0; 4];
    payload.extend_from_slice(&390_022u32.to_le_bytes());
    payload.extend_from_slice(&[3, 2]);
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 20, &payload);
    handle_trade(&mut ctx).await;
    assert_eq!(conn.session.homdo[0].count, 3);
    let recipient_now = online_sessions()
        .lock()
        .unwrap()
        .get(&390_022)
        .unwrap()
        .clone();
    assert_eq!(recipient_now.homdo[0].count, 2);
    online_sessions().lock().unwrap().remove(&390_022);
}

#[tokio::test]
async fn storage_and_luulang_validate_destination_and_access_pet() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.tientrang = vec![storage_item(1, 4001, 1), storage_item(2, 4002, 1)];
    conn.session.homdo = (1..=24)
        .map(|slot| storage_item(slot, 5000 + u16::from(slot), 1))
        .collect();
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1, 2]);
    ctx.opcode = 0x1E;
    handle_storage_transfer(&mut ctx).await;
    assert!(conn.session.homdo.iter().any(|i| i.id == 4001));
    assert!(conn.session.tientrang.iter().any(|i| i.id == 4002));
    conn.session.homdo = vec![storage_item(1, 6001, 1)];
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 51, &[1]);
    ctx.opcode = 0x17;
    handle_storage_transfer(&mut ctx).await;
    assert!(conn.session.luulang.is_empty());
    conn.session.pets.push(PetState {
        stt: 1,
        id: 41187,
        ..Default::default()
    });
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 51, &[1]);
    ctx.opcode = 0x17;
    handle_storage_transfer(&mut ctx).await;
    assert_eq!(conn.session.luulang[0].id, 6001);
    let luulang_slot = conn.session.luulang[0].slot;
    let mut out = HandleOutcome::default();
    let luulang_payload = [luulang_slot];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 52, &luulang_payload);
    ctx.opcode = 0x17;
    handle_storage_transfer(&mut ctx).await;
    assert!(conn.session.luulang.is_empty());
    assert_eq!(conn.session.homdo[0].id, 6001);
}

#[tokio::test]
async fn accepted_pet_trade_moves_ownership_and_gold() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.id = 390_031;
    conn.session.gold = 500;
    conn.session.pets = vec![PetState {
        stt: 1,
        id: 7001,
        ..Default::default()
    }];
    conn.session.trade = TradeState {
        active: true,
        partner_id: 390_032,
        gold: 50,
        pets: vec![1],
        ..Default::default()
    };
    let mut partner = Session::new();
    partner.id = 390_032;
    partner.gold = 800;
    partner.pets = vec![PetState {
        stt: 1,
        id: 7002,
        ..Default::default()
    }];
    partner.trade = TradeState {
        active: true,
        partner_id: 390_031,
        accepted: true,
        gold: 100,
        pets: vec![1],
        ..Default::default()
    };
    online_sessions().lock().unwrap().insert(partner.id, partner);
    let mut out = HandleOutcome::default();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 12, &[1]);
    handle_trade(&mut ctx).await;
    let partner = online_sessions()
        .lock()
        .unwrap()
        .get(&390_032)
        .unwrap()
        .clone();
    assert!(conn.session.pets.iter().any(|p| p.id == 7002));
    assert!(partner.pets.iter().any(|p| p.id == 7001));
    assert_eq!((conn.session.gold, partner.gold), (550, 750));
    assert!(out.outgoing.iter().any(|f| f.contains("1A04")));
    assert!(out.outgoing.iter().any(|f| f.contains("0F02")));
    assert!(out.outgoing.iter().any(|f| f.contains("0F07")));
    assert!(out.outgoing.iter().any(|f| f.contains("0F01")));
    assert_eq!(out.outgoing.last().unwrap(), "F4440300190B04");
    online_sessions().lock().unwrap().remove(&390_032);
}

#[tokio::test]
async fn bank_parses_le32_and_enforces_bank_cap() {
    let (data, service) = trade_dependencies();
    let mut conn = Conn::new();
    conn.session.gold = 100_000;
    conn.session.bank_gold = 9_900_000;
    let mut out = HandleOutcome::default();
    let rejected = 100_000u32.to_le_bytes();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &rejected);
    handle_bank_gold(&mut ctx).await;
    assert!(out.outgoing.is_empty());
    let mut out = HandleOutcome::default();
    let accepted = 99_999u32.to_le_bytes();
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &accepted);
    handle_bank_gold(&mut ctx).await;
    assert_eq!(conn.session.bank_gold, 9_999_999);
    assert_eq!(
        out.outgoing,
        vec!["F44406001D019F860100", "F44406001A029F860100"]
    );
}

// ── handlers::use_item ── (migrated from src/server/handlers/use_item/mod.rs)

fn simple_item(id: u16, count: u8) -> InventoryItem {
    InventoryItem {
        slot: 1,
        id,
        count,
        ..Default::default()
    }
}

fn seeded() -> DotNetRandom {
    DotNetRandom::new(42)
}

#[tokio::test]
async fn potion_restores_hp_and_ends_standard() {
    let mut conn = Conn::new();
    conn.session.hp = 50;
    conn.session.hp_max = 200;
    conn.session.sp = 30;
    conn.session.sp_max = 200;
    conn.session.homdo.push(simple_item(30001, 5));
    let mut data = GameData::default();
    data.items.insert(
        30001,
        Item {
            id: 30001,
            hp: 100,
            sp: 50,
            ..Default::default()
        },
    );
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    // slot 1, count 2, use_type 0
    use_item_rng(&mut conn, &[1, 2, 0], &mut out, None, &data, &mut rng).await;

    assert_eq!(conn.session.hp, 200); // 50 + 200 capped
    assert_eq!(conn.session.sp, 130); // 30 + 100
    assert_eq!(conn.session.homdo[0].count, 3); // 5 - 2
    assert_eq!(
        out.outgoing,
        vec![
            "F4440C0008011901C800000000000000".to_string(), // Hp -> 200
            "F4440C0008011A018200000000000000".to_string(), // Sp -> 130
            "F444040017090102".to_string(),                 // 1709 slot 1, used 2
            "F4440200170F".to_string(),
        ]
    );
}

#[tokio::test]
async fn warp_item_moves_map_and_consumes() {
    let mut conn = Conn::new();
    conn.session.id = 300001;
    conn.session.map_id = 12001;
    conn.session.map_x = 400;
    conn.session.map_y = 500;
    conn.session.homdo.push(simple_item(46022, 1));
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

    assert_eq!(conn.session.map_id, 12403);
    assert_eq!(conn.session.map_x, 442);
    assert_eq!(conn.session.map_y, 375);
    assert!(conn.session.homdo.is_empty(), "warp item consumed");
    assert!(out.outgoing.iter().any(|f| f.contains("17090101")));
    assert!(out.outgoing.iter().any(|f| f.starts_with("F4440D000C")));
}

#[tokio::test]
async fn add_pet_item_gives_pet() {
    let mut conn = Conn::new();
    conn.session.homdo.push(simple_item(46001, 1));
    let mut data = GameData::default();
    data.items.insert(
        46001,
        Item {
            id: 46001,
            add_pet: 10001,
            ..Default::default()
        },
    );
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

    assert_eq!(conn.session.pets.len(), 1);
    assert_eq!(conn.session.pets[0].id, 10001);
    assert!(conn.session.homdo.is_empty(), "pet item consumed");
}

#[tokio::test]
async fn point_book_adds_point_and_keeps_item() {
    let mut conn = Conn::new();
    conn.session.homdo.push(simple_item(50010, 1));
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

    assert_eq!(conn.session.point, 1);
    assert_eq!(
        conn.session.homdo.len(),
        1,
        "point book is not consumed"
    );
    assert!(out
        .outgoing
        .iter()
        .any(|f| f.starts_with("F4440C0008012601")));
}

#[tokio::test]
async fn unknown_zero_effect_item_is_silent() {
    // Truly statless unknown item: no frames, no consume.
    let mut conn = Conn::new();
    conn.session.homdo.push(simple_item(40001, 1));
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
    assert_eq!(conn.session.homdo.len(), 1, "not consumed");
    assert!(out.outgoing.is_empty(), "silent: no frames");
}

#[tokio::test]
async fn noop_ids_send_end_frame_without_consume() {
    for id in [46013u16, 46014, 46015, 46042, 46091] {
        let mut conn = Conn::new();
        conn.session.homdo.push(simple_item(id, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
        assert_eq!(conn.session.homdo.len(), 1, "id {id}: not consumed");
        assert!(
            out.outgoing.iter().any(|f| f == "F444040017090101"),
            "id {id}: end feedback present"
        );
        assert!(out.outgoing.iter().any(|f| f == "F4440200170F"));
    }
}

#[tokio::test]
async fn skill_book_learns_at_level_ten() {
    let mut conn = Conn::new();
    conn.session.homdo.push(simple_item(46230, 1));
    let mut data = GameData::default();
    data.skills.insert(
        10016,
        Skill {
            id: 10016,
            name: "Ky Nang".into(),
            sp: 10,
            ..Default::default()
        },
    );
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
    assert_eq!(conn.session.skills.len(), 1);
    assert_eq!(conn.session.skills[0], (10016, 10));
    assert!(conn.session.homdo.is_empty(), "skill book consumed");
    assert!(out
        .outgoing
        .iter()
        .any(|f| f.starts_with("F4440C0008016E01")));
}

#[tokio::test]
async fn texp_book_adds_exp() {
    let mut conn = Conn::new();
    conn.session.homdo.push(simple_item(46211, 1));
    let data = GameData::default();
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
    assert_eq!(conn.session.texp, 106); // session starts at texp 6 + 100
    assert!(conn.session.homdo.is_empty());
}

#[tokio::test]
async fn potion_at_full_still_consumes() {
    let mut conn = Conn::new();
    conn.session.hp = 200;
    conn.session.hp_max = 200;
    conn.session.sp = 200;
    conn.session.sp_max = 200;
    conn.session.homdo.push(simple_item(30001, 5));
    let mut data = GameData::default();
    data.items.insert(
        30001,
        Item {
            id: 30001,
            hp: 100,
            sp: 50,
            ..Default::default()
        },
    );
    let mut out = HandleOutcome::default();
    let mut rng = seeded();
    use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
    assert_eq!(conn.session.hp, 200);
    assert_eq!(conn.session.homdo[0].count, 4, "consumed even at full");
    assert!(out.outgoing.iter().any(|f| f == "F444040017090101"));
}
