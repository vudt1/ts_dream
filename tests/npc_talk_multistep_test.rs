//! Integration test for Checkpoint 4: Talk Continue & Menu Selection
//! (`0x14 Sub 0x06` & `0x14 Sub 0x09`).
//!
//! Scenarios are driven against the **real `Data/eve.emg` content** (surveyed
//! before writing these tests):
//!
//! 1. **Multi-step dialog** — map 10817 (Trác Quận), NPC click id 1
//!    (template 33001): eve 1 has exactly 7 results, all Talk (`type=1`),
//!    under a single unconditional condition. Six Sub 6 presses deliver
//!    results 1..6; the seventh exhausts the queue -> actor unlock
//!    (`F4440700142CE903000002`) + close-dialog (`F44402001408`). Auto-chain
//!    re-resolves the same unconditional chain and is stopped by guard #1
//!    (same `matchedConditionNo`), so no re-open frame is emitted.
//! 2. **Action execution** — map 10817, NPC click id 3 (template 33002 at
//!    950,460): eve 3 fires only when the bag holds item `32012`
//!    (`conditionClass=1 pStyle=1 ops=2 val=0`) and yields two
//!    Action/class-1 results: remove `32012` (`value=-1`) then add `26012`
//!    (`value=+1`) — both with `parameter_style=0`, i.e. the value's sign is
//!    the give/take discriminator. Both run inline on click, then the talk
//!    closes safely.
//! 3. **Menu branching** — map 10851, NPC click id 2 (template 15050 at
//!    1190,200): eve 3's second chain resolves a single Surface result
//!    (`type=6`, `mean_no=3`) on a fresh session. Choice codes then match
//!    `conditionClass=10` chains `(surface=3, code=30|31)` whose first Talk
//!    steps differ (`mean_no` 10534 vs 10535).
//!    NOTE(data): every multi-choice class-10 condition in `eve.emg` uses
//!    ChoiceCode `30..` (0x1E..) — matching the legacy H6 `select_menu`
//!    convention (30=first option, 31=second, 40=close) — not the 1-based
//!    `01/02` the research draft assumed. The handler stores whatever byte
//!    arrives, so both dialects route through the same evaluator.

use std::path::PathBuf;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::eve::auto_chain::EventPhase;
use ts_dream::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use ts_dream::server::dispatcher::{HandleOutcome, OutFrame};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{Conn, InventoryItem};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

/// Position of the first `needle` frame inside `outgoing` (as index).
fn frame_pos(outgoing: &[OutFrame], needle: &str) -> Option<usize> {
    outgoing.iter().position(|f| f.frame == needle)
}

/// All `0x14 Sub 0x01` talk/Surface step frames (`F4 44 11 00 14 01 00 ..`).
fn talk_step_frames(outgoing: &[OutFrame]) -> Vec<&str> {
    outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|f| f.starts_with("F4441100140100"))
        .collect()
}

/// Decode the `result_mean_no` (LE) of a 21-byte talk step frame
/// (`F4 44 11 00 | 14 01 00 | 14B EveResult` — mean_no at byte offset 19).
fn step_mean(frame: &str) -> u16 {
    let bytes = hex::decode(frame).expect("frame must be valid hex");
    assert_eq!(bytes.len(), 21, "talk step frame must be 21 bytes");
    u16::from_le_bytes([bytes[19], bytes[20]])
}

/// Seeded player connection standing on `(map_id, map_x, map_y)`.
fn conn_at(map_id: u16, map_x: u16, map_y: u16) -> Conn {
    let mut conn = Conn::default();
    conn.session.id = 1001; // char id 1001 -> lock hex `E9030000`
    conn.session.name = b"hero".to_vec();
    conn.session.map_id = map_id;
    conn.session.map_x = map_x;
    conn.session.map_y = map_y;
    conn.session.level = 1;
    conn.session.in_world = true;
    conn
}

/// Dispatch one decoded frame through the real dispatcher.
async fn dispatch_frame(
    conn: &mut Conn,
    data: &GameData,
    service: &BattleService,
    frame: &[u8],
) -> HandleOutcome {
    let env = ts_dream::server::dispatcher::ServerEnv::none();
    ts_dream::server::dispatcher::dispatch(conn, frame, data, service, &env).await
}

/// C -> S frames (decoded, pre-XOR): `[F4 44 lenLE 14 sub ..payload]`.
const CLICK_NPC1: [u8; 8] = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x01, 0x00];
const CLICK_NPC3: [u8; 8] = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x03, 0x00];
const CLICK_NPC2: [u8; 8] = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, 0x02, 0x00];
const TALK_CONTINUE: [u8; 6] = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x06]; // Sub 6, empty payload

/// Sub 9 menu select with the given ChoiceCode byte.
fn select_menu_frame(choice: u8) -> [u8; 7] {
    [0xF4, 0x44, 0x03, 0x00, 0x14, 0x09, choice]
}

/// Scenario 1: 7-step Talk dialog walked entirely with `0x14 Sub 0x06`.
#[tokio::test]
async fn test_multistep_talk_continue_walks_to_unlock() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
    let service = BattleService::default();
    set_eve_events_enabled(true);

    let mut conn = conn_at(10817, 590, 540); // Trác Quận tân thủ

    // Click NPC 1: eve 1 resolves to 7 Talk results; results[0] is delivered.
    let out = dispatch_frame(&mut conn, &data, &service, &CLICK_NPC1).await;
    let ev = conn
        .session
        .current_event_session
        .as_ref()
        .expect("Eve session must activate for NPC 1");
    assert_eq!(ev.eve_no, 1);
    assert_eq!(ev.results.len(), 7, "eve 1 must have 7 results");
    assert!(
        ev.results.iter().all(|r| r.result_type == 1),
        "eve 1 results are all Talk steps"
    );
    assert_eq!(ev.current_index, 0, "queue index sits on the delivered step");
    assert_eq!(talk_step_frames(&out.outgoing).len(), 1, "click delivers results[0]");
    let mut last_step = talk_step_frames(&out.outgoing)[0].to_string();

    // Six Sub 6 presses deliver results[1..=6], one step per press.
    for press in 1..=6 {
        let out = dispatch_frame(&mut conn, &data, &service, &TALK_CONTINUE).await;
        let ev = conn
            .session
            .current_event_session
            .as_ref()
            .expect("session must stay active mid-dialog");
        assert_eq!(
            ev.current_index, press,
            "press {press} must advance the queue index"
        );
        let steps = talk_step_frames(&out.outgoing);
        assert_eq!(steps.len(), 1, "press {press} delivers exactly one step");
        assert_ne!(steps[0], last_step, "press {press} must deliver a new step");
        last_step = steps[0].to_string();
        assert!(
            frame_pos(&out.outgoing, "F44402001408").is_none(),
            "press {press} must keep the dialog open"
        );
    }

    // Seventh press exhausts the queue: unlock, then close-dialog.
    let out = dispatch_frame(&mut conn, &data, &service, &TALK_CONTINUE).await;
    assert!(
        conn.session.current_event_session.is_none(),
        "session must clear when the queue is exhausted"
    );
    assert!(talk_step_frames(&out.outgoing).is_empty(), "no further steps");

    let unlock_hex = NpcTalkCodec::build_talk_lock_hex(1001, TalkLockMode::Unlock);
    assert_eq!(unlock_hex, "F4440700142CE903000002");
    let unlock_idx = frame_pos(&out.outgoing, &unlock_hex).expect("must emit unlock frame");
    let close_idx = frame_pos(&out.outgoing, "F44402001408").expect("must emit close frame");
    assert!(unlock_idx < close_idx, "unlock must precede close-dialog");
    assert!(
        frame_pos(&out.outgoing, "F44402000602").is_none(),
        "auto-chain must not re-open: eve 1's unconditional chain trips guard #1"
    );
    assert_eq!(conn.session.idtalking, 0, "legacy talk context reset");
    assert_eq!(conn.session.select_menu, 0, "legacy talk context reset");
    assert_eq!(conn.session.talk_count, 0, "legacy talk context reset");
}

/// Scenario 2: Action results execute inline (item swap) on click, then the
/// talk closes safely with no Talk step ever shown.
#[tokio::test]
async fn test_action_results_execute_item_swap_and_close() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
    let service = BattleService::default();
    set_eve_events_enabled(true);

    let mut conn = conn_at(10817, 950, 460); // NPC 3 position (scene eve data)
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 32012,
        count: 1,
        ..Default::default()
    });

    let out = dispatch_frame(&mut conn, &data, &service, &CLICK_NPC3).await;

    // eve 3: [-32012, +26012] both Action/class-1 — bag must be swapped.
    let count_of = |item_id: u16| -> u32 {
        conn.session
            .homdo
            .iter()
            .filter(|i| i.id == item_id)
            .map(|i| u32::from(i.count))
            .sum()
    };
    assert_eq!(count_of(32012), 0, "item 32012 must be consumed");
    assert_eq!(count_of(26012), 1, "item 26012 must be granted");

    // Wire order: open -> lock -> bag sync(s) -> unlock -> close, and never
    // a talk step (pure-action event).
    let open_idx = frame_pos(&out.outgoing, "F44402000602").expect("open dialog frame");
    let lock_idx = frame_pos(
        &out.outgoing,
        &NpcTalkCodec::build_talk_lock_hex(1001, TalkLockMode::Lock),
    )
    .expect("lock frame");
    let bag_idx = out
        .outgoing
        .iter()
        .position(|f| f.frame.contains("1705"))
        .expect("bag sync frame (op 0x05/1705) after item mutation");
    let unlock_idx = frame_pos(
        &out.outgoing,
        &NpcTalkCodec::build_talk_lock_hex(1001, TalkLockMode::Unlock),
    )
    .expect("unlock frame");
    let close_idx = frame_pos(&out.outgoing, "F44402001408").expect("close frame");
    assert!(open_idx < lock_idx, "open must precede lock");
    assert!(lock_idx < bag_idx, "bag sync must follow the lock");
    assert!(bag_idx < unlock_idx, "unlock must follow the action");
    assert!(unlock_idx < close_idx, "unlock must precede close-dialog");
    assert!(
        talk_step_frames(&out.outgoing).is_empty(),
        "pure-action event must not emit a talk step"
    );
    assert!(
        conn.session.current_event_session.is_none(),
        "session must clear after the action queue drains"
    );
    assert_eq!(conn.session.idtalking, 0, "legacy talk context reset");
}

/// Scenario 3: Surface menu + conditionClass=10 branching via `0x14 Sub 0x09`.
#[tokio::test]
async fn test_menu_surface_and_choice_branching() {
    let data = GameData::load(&data_dir()).expect("Failed to load GameData");
    let service = BattleService::default();
    set_eve_events_enabled(true);

    // Click NPC 2 on map 10851: fresh session matches eve 3's "quest flag
    // unset" chain, whose single result is Surface mean_no=3.
    let mut conn = conn_at(10851, 1190, 200);
    let out = dispatch_frame(&mut conn, &data, &service, &CLICK_NPC2).await;
    {
        let ev = conn
            .session
            .current_event_session
            .as_ref()
            .expect("Eve session must activate for 10851/NPC 2");
        assert_eq!(ev.eve_no, 3);
        assert_eq!(ev.results.len(), 1, "Surface-only result queue");
        assert_eq!(ev.results[0].result_type, 6, "first result is a Surface");
        assert_eq!(ev.phase, EventPhase::AwaitingChoice, "waiting for Sub 9");
        assert_eq!(ev.last_surface_id, 3, "surface id recorded for cls10");
        assert_eq!(ev.last_choice_code, -1, "no choice yet");
        assert_eq!(ev.current_index, 0, "queue index on the delivered Surface");
    }
    assert_eq!(talk_step_frames(&out.outgoing).len(), 1, "Surface frame emitted");
    assert!(
        frame_pos(&out.outgoing, "F44402001408").is_none(),
        "menu must wait for the choice, not close"
    );

    // ChoiceCode 0 = "nothing selected": ignored, state untouched.
    let out = dispatch_frame(&mut conn, &data, &service, &select_menu_frame(0)).await;
    assert!(out.outgoing.is_empty(), "ChoiceCode 0 must emit nothing");
    let ev = conn.session.current_event_session.as_ref().unwrap();
    assert_eq!(ev.phase, EventPhase::AwaitingChoice, "still awaiting a real choice");
    assert_eq!(ev.last_choice_code, -1, "ChoiceCode 0 must not be recorded");

    // Choose the first option (ChoiceCode 30 = 0x1E): branch chain
    // (surface=3, code=30, conditionNo=3) starts with Talk mean_no 10534.
    let out = dispatch_frame(&mut conn, &data, &service, &select_menu_frame(30)).await;
    let branch_a_mean;
    {
        let ev = conn
            .session
            .current_event_session
            .as_ref()
            .expect("branch session must activate");
        assert_eq!(ev.phase, EventPhase::Executing, "branch resumed execution");
        assert_eq!(ev.last_choice_code, 30, "choice recorded on the session");
        assert_eq!(ev.last_surface_id, 3, "Surface context carried into the branch");
        assert_eq!(ev.matched_condition_no, 3, "cls10 chain 30 must win");
        assert_eq!(ev.current_index, 0, "fresh branch queue");
        branch_a_mean = ev.results[0].result_mean_no;
    }
    assert_eq!(conn.session.select_menu, 30, "legacy select_menu mirrors the choice");
    let steps_a = talk_step_frames(&out.outgoing);
    assert_eq!(steps_a.len(), 1, "branch delivers exactly its first step");
    assert_eq!(step_mean(steps_a[0]), 10534, "branch 30 first Talk mean_no");
    assert!(
        frame_pos(&out.outgoing, "F44402001408").is_none(),
        "dialog stays open when entering the branch"
    );

    // Fresh connection picks the second option (ChoiceCode 31 = 0x1F):
    // branch chain (surface=3, code=31, conditionNo=4) -> Talk 10535.
    let mut conn2 = conn_at(10851, 1190, 200);
    let _ = dispatch_frame(&mut conn2, &data, &service, &CLICK_NPC2).await;
    let out = dispatch_frame(&mut conn2, &data, &service, &select_menu_frame(31)).await;
    let branch_b_frame;
    {
        let ev = conn2
            .session
            .current_event_session
            .as_ref()
            .expect("branch session must activate");
        assert_eq!(ev.last_choice_code, 31, "choice recorded on the session");
        assert_eq!(ev.matched_condition_no, 4, "cls10 chain 31 must win");
        branch_b_frame = ev.results[0].result_mean_no;
    }
    let steps_b = talk_step_frames(&out.outgoing);
    assert_eq!(steps_b.len(), 1, "branch delivers exactly its first step");
    assert_eq!(step_mean(steps_b[0]), 10535, "branch 31 first Talk mean_no");
    assert_ne!(
        steps_a[0], steps_b[0],
        "the two choices must take different branches"
    );
    assert_ne!(
        branch_a_mean, branch_b_frame,
        "branch result queues must differ"
    );

    // Legacy fallback: without an Eve session the raw byte is still stored.
    let mut free_conn = Conn::default();
    let out = dispatch_frame(&mut free_conn, &data, &service, &select_menu_frame(30)).await;
    assert_eq!(free_conn.session.select_menu, 30, "legacy select_menu stored");
    assert!(out.outgoing.is_empty(), "legacy path emits nothing");
}
