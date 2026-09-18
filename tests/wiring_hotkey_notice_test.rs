//! Wiring + hotkey + client-verified notice frames (0x19/0x1B/0x1F/0x28/0x17/0x0B).
//!
//! Ground truth: `client_pseudo_c/0077f414` (aLogin never sends 0x1B/0x1F/0x28
//! C→S; its only C→S 0x19 is the ACK after S→C 0x19/0x29) and
//! `.scratch/client-pseudo-op-code/opcode_*.md` (S→C bus semantics).
//! Bear C# is a sample *server* (reference for C→S dialects only).

use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::stats::handle_hotkey;
use ts_dream::server::session::Conn;
use ts_dream::server::spawn;
use ts_dream::server::dispatcher::OpcodeCtx;

/// Drive one handler directly with a stub env.
async fn run_hotkey(sub: u8, payload: &[u8], hotkeys: [u16; 11]) -> [u16; 11] {
    let mut conn = Conn::default();
    conn.session.hotkeys = hotkeys;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let env = ServerEnv::none();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x28,
        sub,
        payload,
    };
    handle_hotkey(&mut ctx).await;
    conn.session.hotkeys
}

/// Full dispatcher round-trip for a raw decoded frame.
async fn dispatch_frame(decoded: &[u8]) -> ts_dream::server::dispatcher::HandleOutcome {
    let mut conn = Conn::default();
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();
    dispatch(&mut conn, decoded, &data, &service, &env).await
}

#[tokio::test]
async fn test_hotkey_assign_kind2() {
    // payload = [kind=2][skill LE16][slot]: skill 0x1234 into slot 5.
    let hotkeys = run_hotkey(1, &[2, 0x34, 0x12, 5], [0; 11]).await;
    assert_eq!(hotkeys[5], 0x1234);
}

#[tokio::test]
async fn test_hotkey_clear_kind0() {
    let mut initial = [0u16; 11];
    initial[3] = 0x2222;
    let hotkeys = run_hotkey(1, &[0, 0, 0, 3], initial).await;
    assert_eq!(hotkeys[3], 0);
    assert_eq!(hotkeys[5], 0);
}

#[tokio::test]
async fn test_hotkey_guards() {
    // sub != 1 (Bear logs "unknown subcode").
    assert_eq!(run_hotkey(2, &[2, 0x34, 0x12, 5], [0; 11]).await, [0; 11]);
    // Unknown kind ignored (Bear only handles 0/2).
    assert_eq!(run_hotkey(1, &[1, 0x34, 0x12, 5], [0; 11]).await, [0; 11]);
    // Slot 0 rejected (Bear would index -1 and crash; must not touch [0]).
    assert_eq!(run_hotkey(1, &[2, 0x34, 0x12, 0], [0; 11]).await, [0; 11]);
    // Slot 11 out of range.
    assert_eq!(run_hotkey(1, &[2, 0x34, 0x12, 11], [0; 11]).await, [0; 11]);
    // Short payload.
    assert_eq!(run_hotkey(1, &[2, 0x34], [0; 11]).await, [0; 11]);
}

#[tokio::test]
async fn test_wired_opcodes_ignore_unknown_stubs() {
    // 0x19 sub 9 (unknown): handler `_` arm — silent, no shutdown.
    // decoded = [F4 44][len][19][09]
    let out = dispatch_frame(&[0xF4, 0x44, 0x02, 0x00, 0x19, 0x09]).await;
    assert!(out.outgoing.is_empty());
    assert!(!out.shutdown);

    // 0x19 with empty body at sub 1: length guard — silent.
    let out = dispatch_frame(&[0xF4, 0x44, 0x02, 0x00, 0x19, 0x01]).await;
    assert!(out.outgoing.is_empty());
    assert!(!out.shutdown);

    // 0x1B empty: npc-shop guards (menu/count 0, no shelf) — silent.
    let out = dispatch_frame(&[0xF4, 0x44, 0x02, 0x00, 0x1B, 0x01]).await;
    assert!(out.outgoing.is_empty());
    assert!(!out.shutdown);

    // 0x1F unknown sub: stable `_` arm — silent.
    let out = dispatch_frame(&[0xF4, 0x44, 0x02, 0x00, 0x1F, 0x09]).await;
    assert!(out.outgoing.is_empty());
    assert!(!out.shutdown);

    // 0x1F sub 2 with empty body: delegated pet handler guards empty payload.
    let out = dispatch_frame(&[0xF4, 0x44, 0x02, 0x00, 0x1F, 0x02]).await;
    assert!(out.outgoing.is_empty());
    assert!(!out.shutdown);
}

#[test]
fn test_client_verified_notice_frames() {
    // opcode_17.md §3: [17][19] pickup-fail toast, [17][3B] trade-cancel toast.
    assert_eq!(spawn::ITEM_PICKUP_FAIL, "F44402001719");
    assert_eq!(spawn::TRADE_CANCELLED, "F4440200173B");
    // opcode_0b.md §2: [0B][09][A][B] flag write (4B min), [0B][03][code],
    // [0B][07][B1][B2] counter (4B min).
    assert_eq!(spawn::battle_flag_frame(5, 1), "F44404000B090501");
    assert_eq!(spawn::battle_toast_frame(3), "F44403000B0303");
    assert_eq!(spawn::battle_counter_frame(1, 7), "F44405000B070107");
}

#[tokio::test]
async fn test_pickup_empty_slot_toasts_fail() {
    // No map drops seeded: pickup on an empty slot must toast 17 19
    // instead of replying with nothing.
    let out = dispatch_frame(&[0xF4, 0x44, 0x03, 0x00, 0x17, 0x02, 0x00]).await;
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::ITEM_PICKUP_FAIL),
        "expected ITEM_PICKUP_FAIL, got {:?}",
        out.outgoing.iter().map(|f| &f.frame).collect::<Vec<_>>()
    );
    assert!(!out.shutdown);
}
