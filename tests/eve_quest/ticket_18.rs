//! Ticket 18 tests:
//! - Pre-planned `EventSession::planned_effects` used on open, AutoChain, and post-battle resume
//! - Unified `u16` `eve_no` index lookup between `SceneEveData` and `npc_event` (no `Vec<u8>` / `u8` truncation)

use std::collections::HashMap;
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::runner::Outcome;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveEncounterPlacement, EveNpcPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::eve::auto_chain::{AutoChainResult, EveAutoChainEngine, EventPhase, EventTriggerKind};
use ts_dream::eve::planner::DomainEffect;
use ts_dream::server::dispatcher::HandleOutcome;
use ts_dream::server::handlers::eve_runner::{
    execute_event_step_with_pool, resume_eve_after_battle_full,
};
use ts_dream::server::handlers::npc_event::{resolve_npc_event, NpcTrigger};
use ts_dream::server::session::Session;

fn make_talk_result(mean_no: u16) -> EveResult {
    EveResult {
        result_type: 1,
        result_mean_no: mean_no,
        ..Default::default()
    }
}

fn make_gold_result(amount: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 5,
        parameter: 0,
        parameter_style: 1,
        result_value: amount,
        ..Default::default()
    }
}

#[tokio::test]
async fn ticket18_event_session_preplanned_effects_drive_execution_and_auto_chain() {
    let mut scene = SceneEveData::default();
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                and_num: 1,
                results: vec![
                    make_gold_result(1500),
                    EveResult {
                        result_type: 3,
                        result_mean_no: 7,
                        ..Default::default()
                    },
                    make_talk_result(101),
                ],
                ..Default::default()
            }],
        },
    );
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 2,
                condition_class: 8,
                condition_parameter: 0,
                condition_parameter_style: 1,
                condition_ops: 1,
                condition_value: 1,
                and_num: 1,
                results: vec![make_gold_result(2000)],
                ..Default::default()
            }],
        },
    );
    scene.npcs.insert(
        10,
        EveNpcPlacement {
            events: vec![1, 2],
            ..Default::default()
        },
    );
    scene.npc_events_by_npc.insert(10, vec![1, 2]);
    let mut fight_map = HashMap::new();
    fight_map.insert(7, Default::default());
    scene.fight_datas = fight_map;

    let mut data = GameData::default();
    data.scene_eve_data.insert(12001, scene);

    let mut session = Session::new();
    session.id = 9001;
    session.map_id = 12001;
    session.gold = 0;

    let mut rng = DotNetRandom::new(42);
    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(10), &mut rng)
        .expect("event 1 must resolve");
    assert_eq!(ev.planned_effects.len(), 3);
    assert!(matches!(ev.planned_effects[0], DomainEffect::MutateMoney(_)));
    assert!(matches!(ev.planned_effects[1], DomainEffect::StartBattle { fight_id: 7 }));
    assert!(matches!(ev.planned_effects[2], DomainEffect::TalkStep(_)));

    session.current_event_session = Some(ev);
    let mut out = HandleOutcome::default();
    execute_event_step_with_pool(&mut session, &data, None, &mut out).await;
    assert_eq!(session.gold, 1500);
    assert_eq!(out.eve_battle, Some((7, 112)));
    assert_eq!(
        session.current_event_session.as_ref().map(|e| e.phase),
        Some(EventPhase::AwaitingBattle)
    );

    // Resume after battle: continues from planned_effects[2] (TalkStep 101)
    let resume_out = resume_eve_after_battle_full(&mut session, Outcome::PlayerWin, &data, None).await;
    assert!(!resume_out.outgoing.is_empty());
    assert_eq!(
        session.current_event_session.as_ref().map(|e| e.current_index),
        Some(2)
    );

    // AutoChain also populates planned_effects
    let completed = session.current_event_session.as_ref().unwrap().clone();
    let state = ts_dream::server::handlers::npc_event::snapshot_state_with_data(
        &session,
        Some(&data),
        Some(&completed),
    );
    let scene_ref = data.scene_eve_data.get(&12001).unwrap();
    let chained = match EveAutoChainEngine::try_auto_chain(scene_ref, &completed, &state, &mut rng) {
        AutoChainResult::Chained(c) => c,
        AutoChainResult::NoMatch => panic!("expected AutoChain to match eve_no 2 on win"),
    };
    assert_eq!(chained.eve_no, 2);
    assert_eq!(chained.planned_effects.len(), 1);
    assert!(matches!(
        chained.planned_effects[0],
        DomainEffect::MutateMoney(_)
    ));
}

#[test]
fn ticket18_npc_event_uses_scene_u16_index_without_u8_truncation() {
    let mut scene = SceneEveData::default();
    // eve_no = 300 (> u8::MAX) would be silently dropped by u8::try_from
    scene.npc_events.insert(
        300,
        NpcEventData {
            eve_no: 300,
            when_happen: [true, true, true, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                and_num: 1,
                results: vec![make_talk_result(555)],
                ..Default::default()
            }],
        },
    );
    scene.npc_events_by_npc.insert(1, vec![300]);
    scene.door_events_by_door.insert(2, vec![300]);
    scene.encounters.push(EveEncounterPlacement {
        id: 3,
        ..Default::default()
    });
    scene.encounter_events_by_id.insert(3, vec![300]);

    let mut data = GameData::default();
    data.scene_eve_data.insert(12001, scene);

    let mut session = Session::new();
    session.map_id = 12001;
    let mut rng = DotNetRandom::new(1);

    for trigger in [
        NpcTrigger::ClickNpc(1),
        NpcTrigger::ClickDoor(2),
        NpcTrigger::WalkStepEncounter(3),
    ] {
        let ev = resolve_npc_event(&session, &data, trigger, &mut rng)
            .expect("u16 eve_no > 255 must resolve directly from SceneEveData u16 indices");
        assert_eq!(ev.eve_no, 300);
        assert_eq!(ev.planned_effects.len(), 1);
    }

    // AutoChain must also read &[u16] indices without u8 truncation
    let mut chain_scene = SceneEveData::default();
    chain_scene.npc_events.insert(
        300,
        NpcEventData {
            eve_no: 300,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                and_num: 1,
                results: vec![make_talk_result(555)],
                ..Default::default()
            }],
        },
    );
    chain_scene.npc_events.insert(
        301,
        NpcEventData {
            eve_no: 301,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 2,
                condition_class: 8,
                condition_parameter: 0,
                condition_parameter_style: 1,
                condition_ops: 1,
                condition_value: 1,
                and_num: 1,
                results: vec![make_gold_result(777)],
                ..Default::default()
            }],
        },
    );
    chain_scene.npc_events_by_npc.insert(1, vec![300, 301]);
    let mut completed = resolve_npc_event(
        &session,
        &{
            let mut d = GameData::default();
            d.scene_eve_data.insert(12001, chain_scene.clone());
            d
        },
        NpcTrigger::ClickNpc(1),
        &mut rng,
    )
    .unwrap();
    completed.battle_result = 1;
    let state = ts_dream::server::handlers::npc_event::snapshot_state_with_context(
        &session,
        Some(&completed),
    );
    let chained = match EveAutoChainEngine::try_auto_chain(&chain_scene, &completed, &state, &mut rng)
    {
        AutoChainResult::Chained(c) => c,
        AutoChainResult::NoMatch => panic!("AutoChain must resolve u16 eve_no=301 from npc_events_by_npc"),
    };
    assert_eq!(chained.eve_no, 301);
    assert_eq!(chained.planned_effects.len(), 1);
    let _ = EventTriggerKind::ClickNpc;
}
