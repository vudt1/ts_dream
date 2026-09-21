//! Integration test for NPC talk transmission (Checkpoint 3).
//!
//! Verifies wire order for Eve-bridged NPC talk:
//! `F44402000602` (open dialog) → `0x14 0x2C <char_id> 01` (actor lock)
//! → `0x14 0x01 00 [14B EveResult]` (first talk step), and that `end_talk`
//! emits actor unlock (`...02`) before `F44402001408` when an Eve session is
//! active.

use std::path::PathBuf;
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::data::loader::GameData;
use ts_dream::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use ts_dream::server::handlers::npc_event::{
    resolve_npc_event, set_eve_events_enabled, NpcTrigger,
};
use ts_dream::server::session::Session;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

/// Position of the first `needle` frame inside `outgoing` (as index).
fn frame_pos(outgoing: &[ts_dream::server::dispatcher::OutFrame], needle: &str) -> Option<usize> {
    outgoing.iter().position(|f| f.frame == needle)
}

#[tokio::test]
async fn test_click_npc_emits_lock_then_first_talk_step() {
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

    // Click NPC 1: Opcode 0x14 Sub 0x01, MapObjectID = 1 (LE).
    let decoded_click_npc = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x01, 0x00];
    let outcome = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc,
        &data,
        &service,
        &env,
    )
    .await;

    assert!(
        conn.session.current_event_session.is_some(),
        "Eve session must be stored after NPC click"
    );

    // Expected frames.
    let open_idx = frame_pos(&outcome.outgoing, "F44402000602")
        .expect("must emit F44402000602 dialog trigger");

    let lock_hex = NpcTalkCodec::build_talk_lock_hex(1001, TalkLockMode::Lock);
    // F4 44 07 00 14 2C <char_id LE> 01 — char_id 1001 = 0x000003E9 -> E9030000.
    assert_eq!(lock_hex, "F4440700142CE903000001");
    let lock_idx = frame_pos(&outcome.outgoing, &lock_hex)
        .expect("must emit actor-lock frame after dialog open");

    let step_idx = outcome
        .outgoing
        .iter()
        .position(|f| f.frame.starts_with("F4441100140100"))
        .expect("must emit first talk step frame (0x14 Sub 0x01)");

    assert!(open_idx < lock_idx, "0602 must precede lock frame");
    assert!(lock_idx < step_idx, "lock frame must precede talk step");

    // Talk step must carry mean_no 10364 (0x287C -> LE `7C28`).
    let step_frame = &outcome.outgoing[step_idx].frame;
    assert!(
        step_frame.contains("7C28"),
        "talk step must embed mean_no 10364 (LE 7C28), got {step_frame}"
    );
}

#[tokio::test]
async fn test_end_talk_emits_unlock_before_close() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
    let service = ts_dream::battle::service::BattleService::default();
    let env = ts_dream::server::dispatcher::ServerEnv::none();

    set_eve_events_enabled(true);

    let mut conn = ts_dream::server::session::Conn::default();
    conn.session.id = 1001;
    conn.session.name = b"hero".to_vec();
    conn.session.map_id = 10817;
    conn.session.map_x = 590;
    conn.session.map_y = 540;
    conn.session.level = 1;
    conn.session.in_world = true;

    // Start talk (click NPC 1) so an Eve session is active.
    let decoded_click_npc = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x01, 0x00];
    let _ = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc,
        &data,
        &service,
        &env,
    )
    .await;
    assert!(conn.session.current_event_session.is_some());

    // End talk: Opcode 0x14 Sub 0x04.
    let decoded_end_talk = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x04];
    let outcome = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_end_talk,
        &data,
        &service,
        &env,
    )
    .await;

    assert!(conn.session.current_event_session.is_none());

    let unlock_hex = NpcTalkCodec::build_talk_lock_hex(1001, TalkLockMode::Unlock);
    assert_eq!(unlock_hex, "F4440700142CE903000002");

    let unlock_idx = frame_pos(&outcome.outgoing, &unlock_hex)
        .expect("must emit actor-unlock frame");
    let close_idx = frame_pos(&outcome.outgoing, "F44402001408")
        .expect("must emit F44402001408 close-dialog frame");

    assert!(
        unlock_idx < close_idx,
        "unlock frame must precede close-dialog frame"
    );
}

#[tokio::test]
async fn test_talk_step_payload_matches_eve_result() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
    let service = ts_dream::battle::service::BattleService::default();
    let env = ts_dream::server::dispatcher::ServerEnv::none();

    set_eve_events_enabled(true);

    // Resolve the event directly to obtain the authoritative results list.
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"hero".to_vec();
    session.map_id = 10817;
    session.map_x = 590;
    session.map_y = 540;
    session.level = 1;

    let mut rng = DotNetRandom::new(12345);
    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng)
        .expect("NPC 1 must resolve an Eve event");
    assert!(!ev.results.is_empty());
    assert_eq!(
        ev.results[0].result_type, 1,
        "NPC 1 first result must be a Talk step for this test"
    );
    let expected_step = NpcTalkCodec::build_talk_step_hex(&ev.results[0]);

    // Now drive the same click through the real dispatcher.
    let mut conn = ts_dream::server::session::Conn::default();
    conn.session.id = 1001;
    conn.session.name = b"hero".to_vec();
    conn.session.map_id = 10817;
    conn.session.map_x = 590;
    conn.session.map_y = 540;
    conn.session.level = 1;
    conn.session.in_world = true;

    let decoded_click_npc = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x01, 0x00];
    let outcome = ts_dream::server::dispatcher::dispatch(
        &mut conn,
        &decoded_click_npc,
        &data,
        &service,
        &env,
    )
    .await;

    // The emitted talk step must be byte-perfect identical to the codec output
    // for results[0] (the 14-byte EveResult payload).
    let emitted = outcome
        .outgoing
        .iter()
        .find(|f| f.frame.starts_with("F4441100140100"))
        .expect("must emit a talk step frame");
    assert_eq!(
        emitted.frame, expected_step,
        "emitted talk step must match codec output for results[0]"
    );

    // Sanity: the hex decodes to a 21-byte frame: F4 44 11 00 14 01 00 [14B].
    let bytes = hex::decode(&emitted.frame).expect("frame must be valid hex");
    assert_eq!(bytes.len(), 21);
    assert_eq!(&bytes[0..7], &[0xF4, 0x44, 0x11, 0x00, 0x14, 0x01, 0x00]);
}
