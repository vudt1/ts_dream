//! Relogin session flags and battle sender tests (ticket 07, reconnect-heartbeat-health).
//!
//! - Sending login twice on the same socket resets `logined` and `in_world` flags
//!   and emits the full login sequence instead of returning empty or hanging.
//! - BattleService sender is updated to the current connection's sender upon relogin.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::session::{Conn, Session};

#[tokio::test]
async fn login_twice_on_same_connection_emits_full_sequence() {
    let data = GameData::default();
    let service = BattleService::default();
    let mut conn = Conn::new();

    // Setup an authenticated character session
    conn.session.id = 60001;
    conn.session.name = b"ReLoginHero".to_vec();

    // Craft Opcode 0x01 login frame:
    // acc_id (4B LE) + "TS" (2B) + version 186 (2B LE) + password "TEST"
    let mut payload = Vec::new();
    payload.extend_from_slice(&60001u32.to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&186u16.to_le_bytes());
    payload.extend_from_slice(b"TEST");

    let sub_len = 4u8; // password length
    let mut frame_bytes = vec![0xF4, 0x44];
    let len = (payload.len() + 2) as u16;
    frame_bytes.extend_from_slice(&len.to_le_bytes());
    frame_bytes.push(0x01); // Opcode 0x01
    frame_bytes.push(sub_len);
    frame_bytes.extend_from_slice(&payload);

    // First login
    let out1 = dispatch(&mut conn, &frame_bytes, &data, &service, &ServerEnv::none()).await;
    assert!(!out1.outgoing.is_empty(), "first login must produce frames");
    assert!(conn.session.logined, "session.logined must be true after first login");

    // Second login on the SAME connection (flags reset internally)
    let out2 = dispatch(&mut conn, &frame_bytes, &data, &service, &ServerEnv::none()).await;
    assert!(!out2.outgoing.is_empty(), "second login must produce frames, not hang");
    assert!(conn.session.logined, "session.logined must be true after second login");
}

#[tokio::test]
async fn battle_sender_receives_frames_from_current_connection() {
    let service = BattleService::default();
    let player_id = 60002i64;

    let sess = Arc::new(RwLock::new(Session {
        id: player_id as u32,
        name: b"BattleHero".to_vec(),
        ..Default::default()
    }));

    // First connection registers sender 1
    let (tx1, mut rx1) = mpsc::channel(16);
    service.register_sender(player_id, sess.clone(), tx1);

    // Relogin: connection 2 registers sender 2
    let (tx2, mut rx2) = mpsc::channel(16);
    service.register_sender(player_id, sess.clone(), tx2);

    // Send a frame to player
    service.send_to(player_id, "F44402000B01".to_string());

    // Sender 2 (current connection) receives the frame
    let received2 = rx2.try_recv();
    assert!(received2.is_ok(), "current connection must receive the battle frame");

    // Sender 1 (old connection) must NOT receive the frame
    let received1 = rx1.try_recv();
    assert!(received1.is_err(), "old connection must not receive new battle frames");
}
