//! Integration & unit tests for Tickets 14 and 15:
//! - Ticket 14: `when_happen` separation (`ClickDoor` vs `MeetDoor`, `ClickNpc` `on_ser_stroke` +
//!   all-false fallback, `WalkStep` encounter `on_click_obj`), `GroupData.use_mode == 2` (`KEEP_ALL`
//!   + adjacent `homdo` slot integrity), and `EveResolver::resolve` `chain[1..].results`.
//! - Ticket 15: Removal of `mark_dont_on_step1` (`p_style=1, step=1` advances `Quest Ledger`
//!   without setting `quest_dont` or emitting `0x18 05`; only `MarkDont` `p_style=5` /
//!   `ExplicitMark` `p_style=3` marks `quest_dont`), plus `cc=7, param=2, p_style=4` regression.

use std::collections::HashMap;
use std::sync::Arc;
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveDataLoader, EveDoorPlacement, EveEncounterPlacement, EveGroupData,
    EveNpcPlacement, EveResult, EveSceneInfo, MarkDatLoader, NpcEventData, SceneEveData,
};
use ts_dream::data::tables::Item;
use ts_dream::eve::auto_chain::{
    AutoChainResult, EveAutoChainEngine, EventPhase, EventSession, EventTriggerKind,
};
use ts_dream::eve::evaluator::evaluate;
use ts_dream::eve::group::apply_group_data;
use ts_dream::eve::resolver::resolve;
use ts_dream::eve::state::PlayerEventState;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{Conn, InventoryItem, Session};

fn click_npc_frame(place_id: u16) -> Vec<u8> {
    let [lo, hi] = place_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, lo, hi]
}

fn click_door_frame(door_id: u16) -> Vec<u8> {
    let [lo, hi] = door_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x04, lo, hi]
}

fn move_frame(dir: u8, x: u16, y: u16) -> Vec<u8> {
    let [x0, x1] = x.to_le_bytes();
    let [y0, y1] = y.to_le_bytes();
    vec![0xF4, 0x44, 0x07, 0x00, 0x06, 0x01, dir, x0, x1, y0, y1]
}

fn has_18_05_frame(out: &HandleOutcome) -> bool {
    out.outgoing.iter().any(|f| f.frame.contains("1805"))
}

fn seed_item(data: &mut GameData, item_id: u16) {
    data.items.insert(
        i64::from(item_id),
        Item {
            id: i64::from(item_id),
            ..Default::default()
        },
    );
}

// ===========================================================================
// Ticket 14 — when_happen filtering
// ===========================================================================

#[tokio::test]
async fn ticket14_door_when_happen_separates_click_door_and_meet_door() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // Door 10 at (300, 400):
    // - eve_no 1: ONLY on_click_obj -> warps to scene_info 1 (map 12001)
    // - eve_no 2: ONLY on_stroke_obj -> warps to scene_info 2 (map 12002)
    // - eve_no 3: ONLY on_into_area -> warps to scene_info 3 (map 12003)
    // - eve_no 4: all false -> must NOT match either ClickDoor or MeetDoor
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            dest_map_id: 12001,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    scene.scene_infos.insert(
        2,
        EveSceneInfo {
            eve_no: 2,
            dest_map_id: 12002,
            player_appear_x: 20,
            player_appear_y: 20,
            ..Default::default()
        },
    );
    scene.scene_infos.insert(
        3,
        EveSceneInfo {
            eve_no: 3,
            dest_map_id: 12003,
            player_appear_x: 30,
            player_appear_y: 30,
            ..Default::default()
        },
    );

    let warp_event = |eve_no: u16, target_scene_info: u16, when_happen: [bool; 4]| NpcEventData {
        eve_no,
        when_happen,
        conditions: vec![EveCondition {
            condition_no: 1,
            and_num: 1,
            to_result: 1,
            results: vec![EveResult {
                result_type: 2,
                parameter: target_scene_info,
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    // Put stroke/all-false FIRST in door.events to prove ClickDoor skips them and picks eve 1.
    scene
        .npc_events
        .insert(4, warp_event(4, 1, [false, false, false, false]));
    scene
        .npc_events
        .insert(2, warp_event(2, 2, [false, true, false, false]));
    scene
        .npc_events
        .insert(1, warp_event(1, 1, [true, false, false, false]));
    scene.doors.insert(
        10,
        EveDoorPlacement {
            id: 10,
            events: vec![4, 2, 1],
            x: 300,
            y: 400,
        },
    );

    // Door 11 at (600, 400): has ONLY on_click_obj (eve 1) and all-false (eve 4) -> MeetDoor must NOT trigger!
    scene.doors.insert(
        11,
        EveDoorPlacement {
            id: 11,
            events: vec![4, 1],
            x: 600,
            y: 400,
        },
    );

    // Door 12 at (800, 400): has ONLY on_into_area (eve 3) -> MeetDoor triggers, ClickDoor does NOT.
    scene
        .npc_events
        .insert(3, warp_event(3, 3, [false, false, true, false]));
    scene.doors.insert(
        12,
        EveDoorPlacement {
            id: 12,
            events: vec![3],
            x: 800,
            y: 400,
        },
    );

    data.scene_eve_data.insert(9001, scene);
    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    // 1. ClickDoor on Door 10 skips eve 4 (all-false) and eve 2 (on_stroke_obj), runs eve 1 -> map 12001.
    let mut conn_click = Conn {
        session: Session {
            id: 14001,
            map_id: 9001,
            map_x: 300,
            map_y: 400,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut conn_click, &click_door_frame(10), &data_arc, &service, &env).await;
    assert_eq!(conn_click.session.map_id, 12001);

    // 2. MeetDoor on Door 10 skips eve 4 and eve 1 (on_click_obj), runs eve 2 (on_stroke_obj) -> map 12002.
    let mut conn_meet = Conn {
        session: Session {
            id: 14002,
            map_id: 9001,
            map_x: 290,
            map_y: 390,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut conn_meet, &move_frame(0, 305, 405), &data_arc, &service, &env).await;
    assert_eq!(conn_meet.session.map_id, 12002);

    // 3. Negative: MeetDoor on Door 11 (only on_click_obj + all-false) must NOT warp.
    let mut conn_meet_neg = Conn {
        session: Session {
            id: 14003,
            map_id: 9001,
            map_x: 590,
            map_y: 400,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(
        &mut conn_meet_neg,
        &move_frame(0, 600, 400),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(conn_meet_neg.session.map_id, 9001);

    // 4. Negative: ClickDoor on Door 12 (only on_into_area) must NOT warp; MeetDoor DOES warp to 12003.
    let mut conn_click_neg = Conn {
        session: Session {
            id: 14004,
            map_id: 9001,
            map_x: 790,
            map_y: 400,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(
        &mut conn_click_neg,
        &click_door_frame(12),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(conn_click_neg.session.map_id, 9001);

    let _ = dispatch(
        &mut conn_click_neg,
        &move_frame(0, 800, 400),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(conn_click_neg.session.map_id, 12003);
}

#[tokio::test]
async fn ticket14_click_npc_on_ser_stroke_and_walkstep_encounter_on_click_obj_preserved() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // NPC 1 with when_happen = [false, false, false, true] (on_ser_stroke) -> advances quest 10501
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [false, false, false, true],
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter: 10501,
                    parameter_style: 1,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 11001,
            events: vec![1],
            ..Default::default()
        },
    );

    // WalkStep Encounter with when_happen = [true, false, false, false] (on_click_obj) -> advances quest 10502
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter: 10502,
                    parameter_style: 1,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
        },
    );
    scene.encounters.push(EveEncounterPlacement {
        id: 50,
        events: vec![2],
        start_x: 500,
        start_y: 500,
        end_x: 600,
        end_y: 600,
        full_map: false,
    });

    data.scene_eve_data.insert(9002, scene);
    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 14005,
            map_id: 9002,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(conn.session.quest_tasks.get(&10501).map(|&(_, s)| s), Some(1));

    let _ = dispatch(&mut conn, &move_frame(0, 550, 550), &data_arc, &service, &env).await;
    assert_eq!(conn.session.quest_tasks.get(&10502).map(|&(_, s)| s), Some(1));
}

#[test]
fn ticket14_auto_chain_door_respects_when_happen() {
    let mut scene = SceneEveData::default();
    // Completed ClickDoor event 1; event 2 on the same door is MeetDoor-only (on_stroke_obj).
    scene.doors.insert(
        10,
        EveDoorPlacement {
            id: 10,
            events: vec![1, 2],
            x: 100,
            y: 100,
        },
    );
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            when_happen: [false, true, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 2,
                    ..Default::default()
                }],
                ..Default::default()
            }],
        },
    );
    let completed = EventSession {
        map_id: 9001,
        eve_no: 1,
        npc_click_id: 10,
        trigger_kind: EventTriggerKind::ClickDoor,
        chain_depth: 0,
        results: vec![],
        planned_effects: vec![],
        current_index: 0,
        phase: EventPhase::Completed,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    };
    let mut rng = DotNetRandom::new(42);
    let chained = EveAutoChainEngine::try_auto_chain(&scene, &completed, &PlayerEventState::default(), &mut rng);
    assert_eq!(chained, AutoChainResult::NoMatch);
}

// ===========================================================================
// Ticket 14 — GroupData.use_mode == 2
// ===========================================================================

#[test]
fn ticket14_group_data_use_mode_2_keeps_all_while_1_and_3_pick_one() {
    let make_results = |group_no: u16| -> Vec<EveResult> {
        vec![
            EveResult {
                result_group_no: group_no,
                result_no: 1,
                result_type: 0,
                result_class: 1,
                parameter: 26001,
                result_value: 1,
                ..Default::default()
            },
            EveResult {
                result_group_no: group_no,
                result_no: 2,
                result_type: 0,
                result_class: 1,
                parameter: 26002,
                result_value: 1,
                ..Default::default()
            },
        ]
    };

    let mut groups = HashMap::new();
    // use_mode == 2 with pick_member > 0 AND non-zero weights -> MUST still keep ALL members!
    groups.insert(
        10,
        EveGroupData {
            eve_no: 10,
            use_mode: 2,
            pick_member: 1,
            member_no_ay: vec![1, 2],
            probability_rate_ay: vec![90, 10],
            ..Default::default()
        },
    );
    // use_mode == 1 and use_mode == 3 -> still pick exactly 1 member.
    groups.insert(
        11,
        EveGroupData {
            eve_no: 11,
            use_mode: 1,
            pick_member: 2,
            member_no_ay: vec![1, 2],
            probability_rate_ay: vec![50, 50],
            ..Default::default()
        },
    );
    groups.insert(
        12,
        EveGroupData {
            eve_no: 12,
            use_mode: 3,
            pick_member: 0,
            member_no_ay: vec![1, 2],
            probability_rate_ay: vec![50, 50],
            ..Default::default()
        },
    );

    let mut rng = DotNetRandom::new(123);
    let kept_all = apply_group_data(make_results(10), &groups, &mut rng);
    assert_eq!(kept_all.len(), 2);

    let picked_1 = apply_group_data(make_results(11), &groups, &mut rng);
    assert_eq!(picked_1.len(), 1);
    assert_eq!(picked_1[0].result_no, 2);

    let picked_3 = apply_group_data(make_results(12), &groups, &mut rng);
    assert_eq!(picked_3.len(), 1);
}

#[tokio::test]
async fn ticket14_e2e_group_use_mode_2_grants_all_items_and_preserves_adjacent_homdo_slots() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    seed_item(&mut data, 26001);
    seed_item(&mut data, 26002);
    seed_item(&mut data, 26099);

    let mut scene = SceneEveData::default();
    scene.group_datas.insert(
        5,
        EveGroupData {
            eve_no: 5,
            use_mode: 2,
            pick_member: 1,
            member_no_ay: vec![1, 2],
            probability_rate_ay: vec![80, 20],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![
                    EveResult {
                        result_group_no: 5,
                        result_no: 1,
                        result_type: 0,
                        result_class: 1,
                        parameter_style: 1,
                        parameter: 26001,
                        result_value: 3,
                        ..Default::default()
                    },
                    EveResult {
                        result_group_no: 5,
                        result_no: 2,
                        result_type: 0,
                        result_class: 1,
                        parameter_style: 1,
                        parameter: 26002,
                        result_value: 2,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 11002,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9003, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 14006,
            map_id: 9003,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };
    // Seed adjacent slot 1 with an unrelated item to verify slot integrity.
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26099,
        count: 7,
        ..Default::default()
    });

    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;

    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.slot == 1 && i.id == 26099 && i.count == 7));
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.slot == 2 && i.id == 26001 && i.count == 3));
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.slot == 3 && i.id == 26002 && i.count == 2));
}

// ===========================================================================
// Ticket 14 — EveResolver AND chain[1..].results
// ===========================================================================

#[test]
fn ticket14_resolver_collects_chain_1_plus_results_in_order() {
    let conditions = vec![
        EveCondition {
            condition_no: 1,
            condition_class: 0,
            and_num: 2,
            results: vec![EveResult {
                result_no: 1,
                parameter: 100,
                ..Default::default()
            }],
            ..Default::default()
        },
        EveCondition {
            condition_no: 2,
            condition_class: 0,
            and_num: 0,
            results: vec![
                EveResult {
                    result_no: 2,
                    parameter: 200,
                    ..Default::default()
                },
                EveResult {
                    result_no: 3,
                    parameter: 300,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    ];

    let resolved = resolve(&conditions, &|_| true, None).expect("AND chain must resolve");
    assert_eq!(resolved.matched_condition_no, 1);
    let params: Vec<u16> = resolved.results.iter().map(|r| r.parameter).collect();
    assert_eq!(params, vec![100, 200, 300]);

    // Also works when chain[0].results is empty and only chain[1].results is populated.
    let head_empty = vec![
        EveCondition {
            condition_no: 10,
            condition_class: 0,
            and_num: 2,
            results: vec![],
            ..Default::default()
        },
        EveCondition {
            condition_no: 11,
            condition_class: 0,
            and_num: 0,
            results: vec![EveResult {
                result_no: 1,
                parameter: 400,
                ..Default::default()
            }],
            ..Default::default()
        },
    ];
    let resolved2 = resolve(&head_empty, &|_| true, None).expect("tail-only AND chain must resolve");
    assert_eq!(resolved2.results.len(), 1);
    assert_eq!(resolved2.results[0].parameter, 400);
}

#[tokio::test]
async fn ticket14_e2e_and_chain_tail_results_execute() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    seed_item(&mut data, 26010);

    let mut scene = SceneEveData::default();
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![
                EveCondition {
                    condition_no: 1,
                    condition_class: 0,
                    and_num: 2,
                    results: vec![],
                    ..Default::default()
                },
                EveCondition {
                    condition_no: 2,
                    condition_class: 0,
                    and_num: 0,
                    results: vec![
                        EveResult {
                            result_type: 0,
                            result_class: 1,
                            parameter_style: 1,
                            parameter: 26010,
                            result_value: 2,
                            ..Default::default()
                        },
                        EveResult {
                            result_type: 0,
                            result_class: 2,
                            parameter_style: 1,
                            parameter: 10510,
                            result_value: 1,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 11003,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9004, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 14007,
            map_id: 9004,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };

    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert!(conn
        .session
        .homdo
        .iter()
        .any(|i| i.id == 26010 && i.count == 2));
    assert_eq!(conn.session.quest_tasks.get(&10510).map(|&(_, s)| s), Some(1));
}

// ===========================================================================
// Ticket 15 — mark_dont_on_step1 removal & cc=7 param=2 p_style=4 regression
// ===========================================================================

#[tokio::test]
async fn ticket15_e2e_multistep_quest_with_mark_dat_does_not_mark_dont_at_step1() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    // Simulate quest 10012 having Mark.dat position 42 (> 0).
    data.quest_marks.insert(10012, 42);

    let mut scene = SceneEveData::default();
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![
                // Step 0 -> Step 1 via p_style=1, val=1
                EveCondition {
                    condition_no: 1,
                    condition_class: 2,
                    condition_parameter: 10012,
                    condition_parameter_style: 3, // exact == 0
                    condition_ops: 0,
                    condition_value: 0,
                    and_num: 1,
                    to_result: 1,
                    results: vec![EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10012,
                        parameter_style: 1,
                        result_value: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                // Step 1 -> Step 2 via p_style=1, val=1 (requires item 26050 so auto-chain pauses after step 1)
                EveCondition {
                    condition_no: 2,
                    condition_class: 2,
                    condition_parameter: 10012,
                    condition_parameter_style: 3, // exact == 1
                    condition_ops: 0,
                    condition_value: 1,
                    and_num: 2,
                    to_result: 1,
                    results: vec![EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10012,
                        parameter_style: 1,
                        result_value: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                EveCondition {
                    condition_no: 20,
                    condition_class: 1,
                    condition_parameter: 26050,
                    condition_ops: 3, // <= in emg -> qty >= 1
                    condition_value: 1,
                    and_num: 0,
                    ..Default::default()
                },
                // Step 2 -> Final completion via MarkDont (p_style=5) (requires item 26051)
                EveCondition {
                    condition_no: 3,
                    condition_class: 2,
                    condition_parameter: 10012,
                    condition_parameter_style: 3, // exact == 2
                    condition_ops: 0,
                    condition_value: 2,
                    and_num: 2,
                    to_result: 1,
                    results: vec![EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10012,
                        parameter_style: 5, // MarkDont
                        result_value: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                EveCondition {
                    condition_no: 30,
                    condition_class: 1,
                    condition_parameter: 26051,
                    condition_ops: 3, // <= in emg -> qty >= 1
                    condition_value: 1,
                    and_num: 0,
                    ..Default::default()
                },
            ],
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 11012,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9005, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 15001,
            map_id: 9005,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };

    // Step 1: advances to 1, must NOT set quest_dont or emit 0x18 05.
    let out1 = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(conn.session.quest_tasks.get(&10012).map(|&(_, s)| s), Some(1));
    assert!(!conn.session.quest_dont.contains(&42));
    assert!(!has_18_05_frame(&out1));

    // Step 2: provide item 26050, advances to 2, still must NOT set quest_dont or emit 0x18 05.
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26050,
        count: 1,
        ..Default::default()
    });
    let out2 = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(conn.session.quest_tasks.get(&10012).map(|&(_, s)| s), Some(2));
    assert!(!conn.session.quest_dont.contains(&42));
    assert!(!has_18_05_frame(&out2));

    // Step 3: provide item 26051, MarkDont (p_style=5) sets quest_dont 42 and emits 0x18 05.
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 26051,
        count: 1,
        ..Default::default()
    });
    let out3 = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert!(conn.session.quest_dont.contains(&42));
    assert!(has_18_05_frame(&out3));
}

#[test]
fn ticket15_cc7_param2_pstyle4_regression_evaluates_reborn_count() {
    let cond = EveCondition {
        condition_class: 7,
        condition_parameter: 2,
        condition_parameter_style: 4,
        condition_ops: 3, // <= in emg -> inverted to >= for actual >= 1
        condition_value: 1,
        ..Default::default()
    };

    let mut state = PlayerEventState {
        reborn_count: 0,
        ..Default::default()
    };
    assert!(!evaluate(&cond, &state));

    state.reborn_count = 1;
    assert!(evaluate(&cond, &state));
}

#[test]
fn ticket14_real_eve_emg_use_mode_2_and_and_chain_tail_results() {
    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data");
    let eve_bytes = std::fs::read(data_dir.join("eve.emg")).expect("read Data/eve.emg");
    let scenes = EveDataLoader::load(&eve_bytes).expect("parse Data/eve.emg");

    let mut use_mode_2_groups = 0usize;
    let mut and_chains_with_tail_results = 0usize;
    let mut tail_nodes_with_results = 0usize;

    for scene in scenes.values() {
        for group in scene.group_datas.values() {
            if group.use_mode == 2 {
                use_mode_2_groups += 1;
                let mut results = Vec::new();
                for (idx, &m) in group.member_no_ay.iter().enumerate() {
                    results.push(EveResult {
                        result_group_no: group.eve_no,
                        result_no: m,
                        parameter: 26000 + idx as u16,
                        ..Default::default()
                    });
                }
                let mut groups = HashMap::new();
                groups.insert(group.eve_no, group.clone());
                let mut rng = DotNetRandom::new(7);
                let kept = apply_group_data(results.clone(), &groups, &mut rng);
                assert_eq!(kept.len(), results.len(), "use_mode==2 group {} must keep all members", group.eve_no);
            }
        }

        for ev in scene.npc_events.values() {
            for chain in ts_dream::eve::resolver::build_chains(&ev.conditions) {
                let tail_count = chain[1..].iter().filter(|c| !c.results.is_empty()).count();
                if tail_count > 0 {
                    and_chains_with_tail_results += 1;
                    tail_nodes_with_results += tail_count;
                    let total_expected: usize = chain.iter().map(|c| c.results.len()).sum();
                    let resolved = resolve(chain, &|_| true, None).expect("real AND chain must resolve");
                    assert_eq!(resolved.results.len(), total_expected);
                }
            }
        }
    }

    assert!(use_mode_2_groups >= 9, "expected >= 9 use_mode==2 groups, got {use_mode_2_groups}");
    assert_eq!(and_chains_with_tail_results, 15, "expected 15 AND chains with tail results in eve.emg");
    assert_eq!(tail_nodes_with_results, 29, "expected 29 tail nodes with results in eve.emg");
}

#[tokio::test]
async fn ticket15_explicit_mark_pstyle3_only_marks_dont_when_result_value_positive() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    data.quest_marks.insert(10012, 42);

    let make_scene = |result_value: i32| {
        let mut scene = SceneEveData::default();
        scene.npc_events.insert(
            1,
            NpcEventData {
                eve_no: 1,
                when_happen: [true, false, false, false],
                conditions: vec![EveCondition {
                    condition_no: 1,
                    and_num: 1,
                    to_result: 1,
                    results: vec![EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10012,
                        parameter_style: 3,
                        result_value,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
            },
        );
        scene.npcs.insert(
            1,
            EveNpcPlacement {
                id: 1,
                npc_id: 11012,
                events: vec![1],
                ..Default::default()
            },
        );
        scene
    };

    // 1. Negative result_value (-1) must NOT set quest_dont or emit 0x18 05.
    data.scene_eve_data.insert(9010, make_scene(-1));
    // 2. Positive result_value (1) MUST set quest_dont 42 and emit 0x18 05.
    data.scene_eve_data.insert(9011, make_scene(1));

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn_neg = Conn {
        session: Session {
            id: 15010,
            map_id: 9010,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };
    let out_neg = dispatch(&mut conn_neg, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert!(!conn_neg.session.quest_dont.contains(&42));
    assert!(!has_18_05_frame(&out_neg));

    let mut conn_pos = Conn {
        session: Session {
            id: 15011,
            map_id: 9011,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };
    let out_pos = dispatch(&mut conn_pos, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert!(conn_pos.session.quest_dont.contains(&42));
    assert!(has_18_05_frame(&out_pos));
}

#[tokio::test]
async fn ticket15_real_mark_dat_multistep_quests_do_not_mark_dont_at_step1() {
    set_eve_events_enabled(true);
    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data");
    let mark_bytes = std::fs::read(data_dir.join("Mark.dat")).expect("read Data/Mark.dat");
    let quest_marks = MarkDatLoader::load(&mark_bytes).expect("parse Data/Mark.dat");

    let target_quests = [10012u16, 11443, 11863, 12325, 12358, 12377, 12406];
    let mut data = GameData::default();
    data.quest_marks = quest_marks;

    for &qid in &target_quests {
        let mark = *data
            .quest_marks
            .get(&qid)
            .unwrap_or_else(|| panic!("quest {qid} missing in Mark.dat"));
        assert!(mark > 0, "quest {qid} expected Mark.dat position > 0");
    }

    let mut scene = SceneEveData::default();
    for (idx, &qid) in target_quests.iter().enumerate() {
        let place_id = (idx + 1) as u16;
        scene.npc_events.insert(
            place_id,
            NpcEventData {
                eve_no: place_id,
                when_happen: [true, false, false, false],
                conditions: vec![EveCondition {
                    condition_no: 1,
                    and_num: 1,
                    to_result: 1,
                    results: vec![EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: qid,
                        parameter_style: 1,
                        result_value: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
            },
        );
        scene.npcs.insert(
            place_id,
            EveNpcPlacement {
                id: place_id,
                npc_id: 11000 + place_id,
                events: vec![place_id as u8],
                ..Default::default()
            },
        );
    }
    data.scene_eve_data.insert(9012, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 15012,
            map_id: 9012,
            map_x: 0,
            map_y: 0,
            ..Session::new()
        },
        ..Default::default()
    };

    for (idx, &qid) in target_quests.iter().enumerate() {
        let place_id = (idx + 1) as u16;
        let out = dispatch(&mut conn, &click_npc_frame(place_id), &data_arc, &service, &env).await;
        assert_eq!(conn.session.quest_tasks.get(&qid).map(|&(_, s)| s), Some(1));
        assert!(!has_18_05_frame(&out), "quest {qid} step 1 must not emit 0x18 05");
    }
    assert!(conn.session.quest_dont.is_empty());
}
