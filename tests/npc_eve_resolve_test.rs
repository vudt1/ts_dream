//! Integration test for NPC Eve event resolution (Checkpoint 2).

use std::path::PathBuf;
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::data::loader::GameData;
use ts_dream::server::handlers::npc_event::{
    resolve_npc_event, set_eve_events_enabled, NpcTrigger,
};
use ts_dream::server::session::Session;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

#[test]
fn test_inspect_trac_quan_scene_eve() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
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
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");

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
    assert_eq!(ev.trigger_kind, 1); // 1 = ClickNpc
    assert_eq!(ev.results.len(), 7, "NPC 1 has 7 results");

    // Check first result: Talk (result_type = 1), mean_no = 10364
    assert_eq!(ev.results[0].result_type, 1);
    assert_eq!(ev.results[0].result_mean_no, 10364);

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
    assert_eq!(ev4.results.len(), 7);
    // Last result is Door (type = 2)
    assert_eq!(ev4.results[6].result_type, 2);

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
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
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
    assert_eq!(active_ev.results.len(), 7);

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
        !outcome_npc3_unmet
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402001408"),
        "Unmet NPC without Eve match falls back to generic/quest talk instead of crash"
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

