//! Integration tests for Ticket 01:
//! - Full `Data/eve.emg` binary loading (3,897 non-empty scenes, 12001/10951/10954 fight counts)
//! - Read-only lookup indices `(scene_id, eve_no)`, NPC, Door, Encounter
//! - `EVE_EVENTS_ENABLED` default `true`, no `.ini` override, no fake fallback
//! - Error propagation on malformed `eve.emg` binary data
//! - `dispatcher::dispatch` on previously dropped scene (10951)

use std::path::PathBuf;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::EveDataLoader;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::npc_event::eve_events_enabled;
use ts_dream::server::session::Conn;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

#[test]
fn test_gamedata_load_full_eve_emg_and_indices() {
    let data = GameData::load(&data_dir()).expect("GameData::load must succeed");

    assert_eq!(
        data.scene_eve_data.len(),
        3897,
        "Expected 3,897 non-empty scenes from Data/eve.emg"
    );
    assert!(
        data.talks.is_empty(),
        "Legacy Quests/*.ini must not be loaded when eve.emg is Single Source of Truth"
    );

    // 12001 must have 9 scripted fights
    let s12001 = data.scene_eve_data.get(&12001).expect("12001 must exist");
    assert_eq!(
        s12001.fight_datas.len(),
        9,
        "Scene 12001 must have 9 fight_datas"
    );

    // Previously dropped scenes 10951 and 10954 must be present with their fights
    let s10951 = data
        .scene_eve_data
        .get(&10951)
        .expect("10951 must be loaded (was previously dropped)");
    assert!(
        !s10951.fight_datas.is_empty(),
        "10951 must have fight_datas (got {})",
        s10951.fight_datas.len()
    );
    assert!(
        !s10951.npcs.is_empty(),
        "10951 must have NPCs"
    );

    let s10954 = data
        .scene_eve_data
        .get(&10954)
        .expect("10954 must be loaded (was previously dropped)");
    assert!(
        !s10954.fight_datas.is_empty(),
        "10954 must have fight_datas"
    );

    // Verify read-only lookup indices
    assert!(data.eve_event(12001, 1).is_some());
    assert!(data.eve_npc_events(12001, 1).is_some());
}

#[test]
fn test_eve_loader_rejects_malformed_binary_instead_of_swallowing_error() {
    // Valid 1-scene directory pointing out-of-bounds
    let mut bad_offset = Vec::new();
    bad_offset.extend_from_slice(&1u16.to_le_bytes()); // scene_count = 1
    bad_offset.push(9); // name_len
    let mut name = [0u8; 23];
    name[..9].copy_from_slice(b"12001.eve");
    bad_offset.extend_from_slice(&name);
    bad_offset.extend_from_slice(&9999i32.to_le_bytes()); // position out of bounds
    bad_offset.extend_from_slice(&100i32.to_le_bytes()); // size

    let err = EveDataLoader::load(&bad_offset).expect_err("must reject out-of-bounds scene");
    assert!(
        err.to_string().contains("out of bounds"),
        "unexpected error message: {err}"
    );

    // Truncated buffer
    let err_short = EveDataLoader::load(&[0x01]).expect_err("must reject truncated header");
    assert!(err_short.to_string().contains("too short"));
}

#[tokio::test]
async fn test_dispatch_npc_on_previously_dropped_scene_10951_with_default_eve_enabled() {
    assert!(
        eve_events_enabled(),
        "EVE_EVENTS_ENABLED must default to true without manual setter"
    );

    let data = GameData::load(&data_dir()).expect("GameData::load must succeed");
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();

    let s10951 = data.scene_eve_data.get(&10951).expect("10951 must exist");
    let (&npc_place_id, npc) = s10951
        .npcs
        .iter()
        .find(|(_, n)| {
            n.events.iter().any(|&ev_no| {
                s10951.npc_events.get(&u16::from(ev_no)).is_some_and(|ev| {
                    ev.when_happen[0]
                        && ev.conditions.iter().any(|c| c.condition_class == 0)
                })
            })
        })
        .expect("10951 must have an NPC with an unconditional ClickNpc Eve event");

    let mut conn = Conn::default();
    conn.session.id = 2001;
    conn.session.name = b"tester".to_vec();
    conn.session.map_id = 10951;
    conn.session.map_x = npc.x as u16;
    conn.session.map_y = npc.y as u16;
    conn.session.level = 50;
    conn.session.in_world = true;

    let id_bytes = npc_place_id.to_le_bytes();
    let click_frame = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, id_bytes[0], id_bytes[1]];
    let outcome = dispatch(&mut conn, &click_frame, &data, &service, &env).await;

    // Must open dialog (0602) and NOT send the old fake C830 fallback
    assert!(
        outcome.outgoing.iter().any(|f| f.frame == "F44402000602")
            || conn.session.current_event_session.is_some(),
        "Expected real Eve interaction on scene 10951, got: {:?}",
        outcome.outgoing
    );
    assert!(
        !outcome.outgoing.iter().any(|f| f.frame.ends_with("C830")),
        "Must not emit old fake C830 fallback frame"
    );
}
