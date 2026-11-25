//! Integration test for NPC Eve event resolution (Checkpoint 2).

use ts_dream::battle::rng::DotNetRandom;
use ts_dream::server::handlers::npc_event::{
    resolve_npc_event, set_eve_events_enabled, NpcTrigger,
};
use ts_dream::server::session::Session;

#[test]
fn test_inspect_trac_quan_scene_eve() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    assert!(!data.scene_eve_data.is_empty(), "Scenes must be loaded");

    // Map 10817 (Trác Quận tân thủ)
    let scene_10817 = data
        .scene_eve_data
        .get(&10817)
        .expect("Map 10817 must exist in eve.emg");
    assert!(scene_10817.npcs.contains_key(&1), "NPC 1 must exist on map 10817");
    assert_eq!(scene_10817.npcs[&1].npc_id, 33001);
}

#[test]
fn test_resolve_npc_eve_trac_quan_newbie() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());

    // Enable Eve event resolution
    set_eve_events_enabled(true);

    let mut session = Session::new();
    session.id = 1001;
    session.name = b"test_player".to_vec();
    session.map_id = 10817; // Trác Quận tân thủ
    session.map_x = 590;
    session.map_y = 540;
    session.level = 1;

    let mut rng = DotNetRandom::new(12345);

    // Test Click NPC 1 (Tân thủ hướng dẫn, template 33001)
    let event_session = resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(1),
        &mut rng,
    );

    assert!(event_session.is_some(), "Eve event for NPC 1 must be resolved");
    let ev = event_session.unwrap();
    assert_eq!(ev.map_id, 10817);
    assert_eq!(ev.eve_no, 1);
    assert_eq!(ev.npc_click_id, 1);
    assert_eq!(ev.trigger_kind, ts_dream::eve::EventTriggerKind::ClickNpc); // 1 = ClickNpc
    assert_eq!(ev.results.len(), 2, "NPC 1 has 2 results (TTTD)");

    // Check first result: Talk (result_type = 1), mean_no = 52340
    assert_eq!(ev.results[0].result_type, 1);
    assert_eq!(ev.results[0].result_mean_no, 52340);

    // Test Click NPC 4 (NPC chuyển cảnh, template 15009)
    let ev_door_npc = resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(4),
        &mut rng,
    );
    assert!(ev_door_npc.is_some(), "Eve event for NPC 4 must be resolved");
    let ev4 = ev_door_npc.unwrap();
    assert_eq!(ev4.eve_no, 4);
    assert_eq!(ev4.results.len(), 4);
    // Last result is Door (type = 2)
    assert_eq!(ev4.results[3].result_type, 2);

    // Test Click NPC 3 (Yêu cầu có item 32012)
    // Khi chưa có item, resolve phải trả về None
    let ev_item_npc_empty = resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(3),
        &mut rng,
    );
    assert!(
        ev_item_npc_empty.is_none(),
        "NPC 3 requires item 32012, so empty bag must return None"
    );

    // Thêm item 32012 vào túi đồ của session
    session.homdo.push(ts_dream::server::session::InventoryItem {
        slot: 1,
        id: 32012,
        count: 1,
        ..Default::default()
    });

    let ev_item_npc_with_item = resolve_npc_event(
        &session,
        &data,
        NpcTrigger::ClickNpc(3),
        &mut rng,
    );
    assert!(
        ev_item_npc_with_item.is_some(),
        "NPC 3 with item 32012 must resolve event"
    );
    let ev3 = ev_item_npc_with_item.unwrap();
    assert_eq!(ev3.eve_no, 3);
    assert_eq!(ev3.results.len(), 2);
    assert_eq!(ev3.results[0].parameter, 32012);
    assert_eq!(ev3.results[0].result_value, -1); // trừ 1
    assert_eq!(ev3.results[1].parameter, 26012);
    assert_eq!(ev3.results[1].result_value, 1); // thêm 1
}

#[tokio::test]
async fn test_dispatch_click_npc_bridges_to_eve_session() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let service = ts_dream::battle::service::BattleService::default();
    let env = ts_dream::server::dispatcher::ServerEnv::none();

    set_eve_events_enabled(true);

    let mut conn = ts_dream::server::session::Conn::default();
    conn.session.id = 1001;
    conn.session.name = b"hero".to_vec();
    conn.session.map_id = 10817; // Trác Quận tân thủ
    conn.session.map_x = 590;
    conn.session.map_y = 540;
    conn.session.level = 1;
    conn.session.in_world = true;

    assert!(conn.session.current_event_session.is_none());

    // Payload: Opcode 0x14 Sub 0x01, MapObjectID = 1 (LE 0x01, 0x00)
    // Frame: F4 44 04 00 14 01 01 00
    let decoded_click_npc = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x01, 0x00];
    let outcome = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc,
        &data,
        &service,
        &env,
    )
    .await;

    // Verify Eve session is stored on conn.session
    assert!(
        conn.session.current_event_session.is_some(),
        "Eve session must be stored in conn.session.current_event_session"
    );
    let active_ev = conn.session.current_event_session.as_ref().unwrap();
    assert_eq!(active_ev.map_id, 10817);
    assert_eq!(active_ev.eve_no, 1);
    assert_eq!(active_ev.npc_click_id, 1);
    assert_eq!(active_ev.results.len(), 2);

    // Verify dialog open frame was sent: F44402000602
    assert!(
        outcome.outgoing.iter().any(|f| f.frame == "F44402000602"),
        "Must emit F44402000602 dialog trigger"
    );

    // Test end_talk resets current_event_session
    // Frame: Opcode 0x14 Sub 0x04 -> F4 44 02 00 14 04
    let decoded_end_talk = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x04];
    let outcome_end = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_end_talk,
        &data,
        &service,
        &env,
    )
    .await;

    assert!(
        conn.session.current_event_session.is_none(),
        "current_event_session must be cleared on end_talk"
    );
    assert!(
        outcome_end.outgoing.iter().any(|f| f.frame == "F44402001408"),
        "Must emit F44402001408 EndTalk"
    );

    // Edge case 1: Out of range NPC (> 150 px distance)
    // NPC 1 is at (590, 540). Put player at (1000, 1000)
    conn.session.map_x = 1000;
    conn.session.map_y = 1000;
    let outcome_out_of_range = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc,
        &data,
        &service,
        &env,
    )
    .await;
    assert!(
        conn.session.current_event_session.is_none(),
        "Eve session must not be activated when out of range"
    );
    assert!(
        outcome_out_of_range
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402001408"),
        "Out of range NPC click must emit EndTalk (F44402001408)"
    );

    // Edge case 2: Non-existent NPC (MapObjectID 999)
    let decoded_click_non_existent = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0xE7, 0x03]; // 999 = 0x03E7
    let outcome_non_existent = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_non_existent,
        &data,
        &service,
        &env,
    )
    .await;
    assert!(
        conn.session.current_event_session.is_none(),
        "Eve session must not be activated for non-existent NPC"
    );
    assert!(
        outcome_non_existent
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402001408"),
        "Non-existent NPC click must emit EndTalk (F44402001408)"
    );

    // Edge case 3: NPC 3 with unmet condition (requires item 32012)
    // NPC 3 is at (950, 460)
    conn.session.map_x = 950;
    conn.session.map_y = 460;
    // MapObjectID = 3 -> [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x03, 0x00]
    let decoded_click_npc3 = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x03, 0x00];
    let outcome_npc3_unmet = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc3,
        &data,
        &service,
        &env,
    )
    .await;
    assert!(
        conn.session.current_event_session.is_none(),
        "NPC 3 with empty bag must not activate Eve session"
    );
    assert!(
        outcome_npc3_unmet
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402001408"),
        "Unmet NPC without Eve match must emit EndTalk (F44402001408) instead of legacy fake fallback"
    );

    // Now give item 32012 to player
    conn.session.homdo.push(ts_dream::server::session::InventoryItem {
        slot: 1,
        id: 32012,
        count: 1,
        ..Default::default()
    });
    let outcome_npc3_met = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc3,
        &data,
        &service,
        &env,
    )
    .await;
    // Checkpoint 4 superseded the CP2 expectation above: the matched Eve
    // session (eve 3 = two Action/class-1 results: -32012, +26012) now
    // executes inline during the click itself and runs to completion, so the
    // session clears in the same dispatch instead of lingering.
    assert!(
        conn.session.current_event_session.is_none(),
        "eve 3 runs to completion on click (action results) and clears"
    );
    assert!(
        outcome_npc3_met
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402000602"),
        "Must emit F44402000602 dialog trigger"
    );
    assert!(
        outcome_npc3_met
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402001408"),
        "eve 3 must close the dialog after its action results"
    );
    // The bag side effects prove the Eve session activated — a legacy
    // talk/quest fallback would never touch the inventory.
    assert!(
        conn.session
            .homdo
            .iter()
            .all(|i| i.id != 32012 || i.count == 0),
        "eve 3 must consume item 32012"
    );
    assert!(
        conn.session
            .homdo
            .iter()
            .any(|i| i.id == 26012 && i.count == 1),
        "eve 3 must grant item 26012"
    );
}

#[test]
fn test_map_12808_straw_dummy_resolution() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let scene = data.scene_eve_data.get(&12808).expect("Scene 12808 must be loaded from eve.emg");
    assert_eq!(scene.npcs.len(), 12, "Map 12808 should contain 12 NPCs (TTTD)");
    assert!(scene.fight_datas.len() >= 6, "Map 12808 should contain at least 6 FightData entries");

    let mut session = Session::new();
    session.id = 1001;
    session.name = b"player".to_vec();
    session.map_id = 12808;
    session.map_x = 750;
    session.map_y = 1780;
    let mut rng = DotNetRandom::new(12345);

    for npc_id in 1..=6u16 {
        let npc = scene.npcs.get(&npc_id).expect("Dummy NPC 1..6 must exist");
        assert_eq!(npc.npc_id, 22012, "NPC template must be 22012 (Straw Dummy)");

        let res = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(npc_id as i32), &mut rng);
        assert!(res.is_some(), "ClickNpc({}) must resolve to an EventSession", npc_id);
        let session_res = res.unwrap();
        assert_eq!(session_res.results.len(), 1);
        assert_eq!(session_res.results[0].result_type, 3, "Result must be battle trigger (type 3)");
        assert_eq!(session_res.results[0].result_mean_no, npc_id, "Fight ID must match dummy index 1..6");
    }
}

#[test]
fn test_interact_range_boundary_150px() {
    use ts_dream::eve::state::EveStateBuilder;

    assert!(EveStateBuilder::is_within_range(750, 1780, 750, 1780));
    assert!(EveStateBuilder::is_within_range(750, 1780, 900, 1780)); // 150px exact
    assert!(EveStateBuilder::is_within_range(750, 1780, 840, 1900)); // sqrt(90^2 + 120^2) = 150px exact
    assert!(!EveStateBuilder::is_within_range(750, 1780, 901, 1780)); // 151px out of range
    assert!(!EveStateBuilder::is_within_range(750, 1780, 841, 1900)); // >150px out of range
    assert!(EveStateBuilder::is_within_range(0, 0, 9999, 9999)); // origin bypass
}

#[test]
fn test_12136_gian_ung_skips_quiz_after_quest_10049_below_lv15() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"player".to_vec();
    session.map_id = 12136;
    session.level = 5; // < 15
    session.quest_tasks.insert(10049, (1, 1));
    let mut rng = DotNetRandom::new(12345);

    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng)
        .expect("Giản Ung must still resolve to completion greeting");
    let talk_ids: Vec<u16> = ev
        .results
        .iter()
        .filter(|r| r.result_type == 1)
        .map(|r| r.result_mean_no)
        .collect();
    assert!(
        talk_ids.contains(&10592) && !talk_ids.contains(&10593),
        "Level < 15 with Quest 10049 >= 1 must match no=8,9 (10592) instead of re-opening quiz 10593, got: {talk_ids:?}"
    );
    assert!(
        !ev.results.iter().any(|r| r.result_type == 6),
        "Must not re-open Surface 1 quiz"
    );
}

#[test]
fn test_12002_door_warps_to_12304_with_cc12_param1_pstyle1() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let scene = data.scene_eve_data.get(&12002).expect("12002.eve loaded");
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"player".to_vec();
    session.map_id = 12002;
    let mut rng = DotNetRandom::new(12345);

    // Find the door on 12002 that leads to 12304 (via eve_no=3, result_type=2, param=7).
    let door_id = scene
        .door_events_by_door
        .iter()
        .find_map(|(&did, eves)| eves.contains(&3).then_some(did))
        .or_else(|| {
            scene
                .doors
                .iter()
                .find_map(|(&did, d)| d.events.contains(&3).then_some(did))
        })
        .expect("Door triggering 12002 eve_no=3 must exist");

    let ev = resolve_npc_event(&session, &data, NpcTrigger::MeetDoor(i32::from(door_id)), &mut rng)
        .expect("12002 door to 12304 must resolve via cc=12 param=1 p_style=1 (0 != 5)");
    assert_eq!(ev.eve_no, 3);
    assert_eq!(ev.results.len(), 1);
    assert_eq!(ev.results[0].result_type, 2);
    let dest = scene
        .scene_infos
        .get(&ev.results[0].parameter)
        .map(|si| si.dest_map_id);
    assert_eq!(dest, Some(12304), "Must warp to map 12304");
}

#[test]
fn test_12001_tientrang_bank_open_and_store_open() {
    use ts_dream::eve::planner::{plan_results, DomainEffect, NpcServiceOp};

    let data = std::sync::Arc::clone(crate::common::shared_game_data());

    // BankOpen (p_style=2) & BankStoreOpen (p_style=1 on TienTrang 14 09 1F) across loaded eve.emg scenes.
    let cond_bank = data
        .scene_eve_data
        .values()
        .flat_map(|s| s.npc_events.values())
        .flat_map(|ev| ev.conditions.iter())
        .find(|c| c.results.iter().any(|r| r.result_type == 7 && r.result_class == 7 && r.parameter_style == 2))
        .expect("eve.emg must have BankOpen (rt=7, rc=7, p_style=2)");
    let cond_store = data
        .scene_eve_data
        .values()
        .flat_map(|s| s.npc_events.values())
        .flat_map(|ev| ev.conditions.iter())
        .find(|c| c.results.iter().any(|r| r.result_type == 7 && r.result_class == 7 && r.parameter_style == 1))
        .expect("eve.emg must have BankStoreOpen (rt=7, rc=7, p_style=1)");

    let effects_bank = plan_results(&cond_bank.results);
    let effects_store = plan_results(&cond_store.results);
    assert!(
        effects_bank.contains(&DomainEffect::NpcService(NpcServiceOp::BankOpen)),
        "p_style=2 must plan NpcServiceOp::BankOpen, got: {effects_bank:?}"
    );
    assert!(
        effects_store.contains(&DomainEffect::NpcService(NpcServiceOp::BankStoreOpen)),
        "p_style=1 must plan NpcServiceOp::BankStoreOpen, got: {effects_store:?}"
    );
}

#[test]
fn test_12002_doc_du_ac_initial_dialog_before_quest_10004() {
    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let mut rng = DotNetRandom::new(12345);

    // 1. Fresh player: quest 10004 and 10001 untouched -> must match cond 1+2
    //    (10055, 10114, 11372), NOT cond 4 fallback (10157).
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"player".to_vec();
    session.map_id = 12002;
    let ev_fresh = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(3), &mut rng)
        .expect("12002 Doc Du Ac (slot 3) must resolve for fresh player");
    let dialogs_fresh: Vec<u16> = ev_fresh.results.iter().map(|r| r.result_mean_no).collect();
    assert!(
        dialogs_fresh.len() == 1 && [10055, 10114, 11372].contains(&dialogs_fresh[0]),
        "Fresh player must match cond 1+2 (random group 1 of 10055/10114/11372), got: {dialogs_fresh:?}"
    );

    // 2. After completing quest 10004 (flag=1): must match cond 10 (10157).
    session.quest_tasks.insert(10004, (0, 1));
    let ev_after = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(3), &mut rng)
        .expect("12002 Doc Du Ac (slot 3) must resolve after quest 10004");
    let dialogs_after: Vec<u16> = ev_after.results.iter().map(|r| r.result_mean_no).collect();
    assert_eq!(dialogs_after, vec![10157]);

    // 3. During quest 10001 (step=1): must match cond 3 (dialog 10152 + choice 1).
    session.quest_tasks.clear();
    session.quest_tasks.insert(10001, (0, 1));
    let ev_q10001 = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(3), &mut rng)
        .expect("12002 Doc Du Ac (slot 3) must resolve during quest 10001");
    let means_q10001: Vec<u16> = ev_q10001.results.iter().map(|r| r.result_mean_no).collect();
    assert_eq!(means_q10001, vec![10152, 1]);
}

#[test]
fn test_12002_doc_du_ac_post_battle_cc8_does_not_loop_back_to_cond3() {
    use ts_dream::eve::auto_chain::{AutoChainResult, EventPhase, EventSession, EventTriggerKind};
    use ts_dream::server::handlers::npc_event::auto_chain_after;

    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let mut rng = DotNetRandom::new(12345);
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"player".to_vec();
    session.map_id = 12002;
    session.quest_tasks.insert(10001, (0, 2));

    // Step 1: cond 5 (StartBattle) finishes with battle_result = 1 (Win) -> must chain to cond 6 (cc=8).
    let completed_battle = EventSession {
        map_id: 12002,
        eve_no: 5,
        npc_click_id: 3,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![],
        planned_effects: vec![],
        current_index: 0,
        phase: EventPhase::Completed,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 1,
        matched_condition_no: 5,
        last_step_sent_ms: 0,
    };

    let cond6 = match auto_chain_after(&completed_battle, &session, &data, &mut rng) {
        AutoChainResult::Chained(ev) => ev,
        AutoChainResult::NoMatch => panic!("Win after cond 5 must chain to cc=8 cond 6"),
    };
    assert_eq!(cond6.matched_condition_no, 6);
    assert_eq!(
        cond6.battle_result, 1,
        "Post-battle cc=8 chain must preserve battle_result so Guard #3 blocks re-opening cond 3"
    );

    // Step 2: cond 6 advances quest 10001 to step 3 and finishes -> must NOT chain back to cond 3 (rt=6).
    session.quest_tasks.insert(10001, (0, 3));
    let next = auto_chain_after(&cond6, &session, &data, &mut rng);
    assert!(
        matches!(next, AutoChainResult::NoMatch),
        "After cc=8 post-battle handler finishes, must NOT loop back to cond 3 (10152 + choice 1); got: {next:?}"
    );
}

#[tokio::test]
async fn test_12002_npc_action_rt9_emits_16_06_pose_frame() {
    use ts_dream::eve::auto_chain::{EventPhase, EventSession, EventTriggerKind};
    use ts_dream::eve::planner::plan_results;
    use ts_dream::server::dispatcher::HandleOutcome;
    use ts_dream::server::handlers::talk::execute_event_step_with_pool;

    let data = std::sync::Arc::clone(crate::common::shared_game_data());
    let scene = data.scene_eve_data.get(&12002).expect("12002 loaded");
    let ev5 = scene.npc_events.get(&5).expect("12002 eve 5");
    let cond6 = ev5.conditions.iter().find(|c| c.condition_no == 6).expect("cond 6");

    let mut session = Session::new();
    session.id = 1001;
    session.map_id = 12002;
    session.current_event_session = Some(EventSession {
        map_id: 12002,
        eve_no: 5,
        npc_click_id: 3,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 1,
        results: cond6.results.clone(),
        planned_effects: plan_results(&cond6.results),
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 1,
        matched_condition_no: 6,
        last_step_sent_ms: 0,
    });

    let mut out = HandleOutcome::default();
    execute_event_step_with_pool(&mut session, &data, None, &mut out).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == "F4440500160603001A"),
        "rt=9 rc=3 param=3 val=26 must emit 16 06 pose frame F4440500160603001A; got: {:?}",
        out.outgoing
    );
}


