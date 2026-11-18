//! TCP end-to-end tests for map 10817 (`14 08 01 00` & Bạch Lão Nhân `15009`),
//! concurrent `merge_post_dispatch` protection, and `sys_msg_frame` / `announce_frame` wire parity.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use ts_dream::data::loader::GameData;
use ts_dream::db::pool::DbPool;
use ts_dream::protocol::frame::Decoder;
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{lock_online_sessions, merge_post_dispatch, InventoryItem, Session};
use ts_dream::server::spawn;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

use crate::common::shared_game_data;
use crate::common::test_db::setup_test_db;

fn xor_wire(hex: &str) -> Vec<u8> {
    let raw = ts_dream::protocol::encoder::bytes(hex).expect("valid hex frame");
    raw.into_iter().map(|b| b ^ 0xAD).collect()
}

fn build_login_hex(player_id: u32, password: &[u8]) -> String {
    let mut payload = Vec::new();
    payload.extend_from_slice(&player_id.to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&186u16.to_le_bytes());
    payload.extend_from_slice(password);
    let op_sub = format!("01{:02X}", password.len());
    ts_dream::protocol::frame(&op_sub, &ts_dream::protocol::encoder::strhex(&payload))
}

fn build_move_hex(x: u16, y: u16) -> String {
    let mut payload = Vec::with_capacity(7);
    payload.push(2); // orient
    payload.extend_from_slice(&x.to_le_bytes());
    payload.extend_from_slice(&y.to_le_bytes());
    payload.push(0);
    payload.push(0);
    ts_dream::protocol::frame("0601", &ts_dream::protocol::encoder::strhex(&payload))
}

async fn seed_account_and_char(pool: &DbPool, player_id: u32, map_id: u16, x: u16, y: u16) {
    let name = format!("Hero{player_id}");
    sqlx::query(
        "INSERT OR REPLACE INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '11111111', '11111111', 1)",
    )
    .bind(i64::from(player_id))
    .execute(&pool.write)
    .await
    .expect("seed account");

    sqlx::query(
        "INSERT OR REPLACE INTO characters (playerid, name, level, curhp, maxhp, cursp, maxsp, mapid, mapx, mapy) \
         VALUES (?, ?, 1, 100, 100, 100, 100, ?, ?, ?)",
    )
    .bind(i64::from(player_id))
    .bind(name.as_bytes())
    .bind(i64::from(map_id))
    .bind(i64::from(x))
    .bind(i64::from(y))
    .execute(&pool.write)
    .await
    .expect("seed character");
}

async fn read_frames_until<F>(
    sock: &mut TcpStream,
    decoder: &mut Decoder,
    timeout_ms: u64,
    mut predicate: F,
) -> Vec<String>
where
    F: FnMut(&[String]) -> bool,
{
    let mut collected = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = [0u8; 4096];
    loop {
        if predicate(&collected) {
            return collected;
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return collected;
        }
        let remain = deadline - now;
        match tokio::time::timeout(remain, sock.read(&mut buf)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => return collected,
            Ok(Ok(n)) => {
                collected.extend(decoder.feed(&buf[..n]));
            }
        }
    }
}

async fn connect_and_login(
    port: u16,
    data: Arc<GameData>,
    pool: DbPool,
    player_id: u32,
    x: u16,
    y: u16,
) -> (ServerControl, TcpStream, Decoder) {
    set_eve_events_enabled(true);
    seed_account_and_char(&pool, player_id, 10817, x, y).await;

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;
    let control = ServerControl::new(port, app, Some(data), Some(pool));
    control.start().await.expect("start test server");

    let mut sock = loop {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", port)).await {
            break s;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let mut decoder = Decoder::new();

    // 1. Wait for initial greeting `01 09`
    let greeting = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.contains("0109"))
    })
    .await;
    assert!(
        greeting.iter().any(|f| f.contains("0109")),
        "expected greeting 0109, got {greeting:?}"
    );

    // 2. Send C->S 0x01 login
    let login_hex = build_login_hex(player_id, b"11111111");
    sock.write_all(&xor_wire(&login_hex)).await.expect("write login");

    // 3. Wait for Logined1 (`03` / `sys_msg_frame`)
    let _ = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.contains("020C"))
    })
    .await;

    // 4. Send C->S 0x25 0x01 ("127.0.0.1") and wait for 27 09 (`OPCODE_SERVER_NAME`)
    sock.write_all(&xor_wire("F4440B0025013132372E302E302E31"))
        .await
        .expect("write 25 01");
    let after_25 = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.contains("2709"))
    })
    .await;
    assert!(
        after_25.iter().any(|f| f.contains("2709")),
        "expected 2709 server name frame, got {after_25:?}"
    );

    (control, sock, decoder)
}

#[test]
fn sys_msg_and_announce_wire_parity() {
    let sys_frame = spawn::sys_msg_frame("Welcome to TS Dream");
    let sys_bytes = ts_dream::protocol::encoder::bytes(&sys_frame).expect("valid hex");
    assert_eq!(sys_bytes[0], 0xF4);
    assert_eq!(sys_bytes[1], 0x44);
    assert_eq!(sys_bytes[4], 0x02);
    assert_eq!(sys_bytes[5], 0x0C);
    // No 4-byte id or stray 0x02 prefix: message starts immediately at index 6 ('W')
    assert_eq!(&sys_bytes[6..], b"Welcome to TS Dream");

    let ann_frame = spawn::announce_frame("Server Maintenance");
    let ann_bytes = ts_dream::protocol::encoder::bytes(&ann_frame).expect("valid hex");
    assert_eq!(ann_bytes[4], 0x02);
    assert_eq!(ann_bytes[5], 0x00);
    assert_eq!(&ann_bytes[6..10], &[0, 0, 0, 0]);
    assert_eq!(&ann_bytes[10..], b"Server Maintenance");
}

#[test]
fn merge_post_dispatch_preserves_concurrent_stall_sale_and_local_walk() {
    let mut base = Session::default();
    base.id = 300010;
    base.map_id = 10817;
    base.map_x = 442;
    base.map_y = 758;
    base.gold = 100;
    base.sync_revision = 5;
    base.homdo.push(InventoryItem {
        slot: 1,
        id: 31007,
        count: 1,
        ..Default::default()
    });

    // Local handler only walked (did not touch gold/homdo)
    let mut local = base.clone();
    local.map_x = 600;
    local.map_y = 700;
    local.last_chat_ms = 9999;

    // Concurrent buyer in shops.rs updated gold and removed item under lock_player_operations
    let mut remote = base.clone();
    remote.gold = 500;
    remote.homdo.clear();
    remote.sync_revision = 6;

    let merged = merge_post_dispatch(&base, &local, &remote);
    assert_eq!(merged.gold, 500, "remote stall sale gold must be preserved");
    assert!(merged.homdo.is_empty(), "remote stall item removal must be preserved");
    assert_eq!(merged.map_x, 600, "local walk map_x must be preserved");
    assert_eq!(merged.map_y, 700, "local walk map_y must be preserved");
    assert_eq!(merged.last_chat_ms, 9999, "local cooldown must be preserved");
    assert_eq!(merged.sync_revision, 7);
}

#[tokio::test]
async fn tcp_map_10817_door_14_08_does_not_deadlock() {
    let _db = setup_test_db().await;
    let pool = (*_db).clone();
    let data = Arc::clone(shared_game_data());
    let player_id = 300081u32;

    let (control, mut sock, mut decoder) =
        connect_and_login(19451, data, pool, player_id, 442, 758).await;

    // Send C->S F444040014080100 (Door 1 on map 10817)
    sock.write_all(&xor_wire("F444040014080100"))
        .await
        .expect("write 14 08 01 00");

    // Must NOT deadlock: expect 14 07 + 0x0C warp response within 2s
    let warp_frames = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.len() >= 12 && &f[8..12] == "1407")
    })
    .await;
    assert!(
        warp_frames.iter().any(|f| f.len() >= 12 && &f[8..12] == "1407"),
        "14 08 01 00 must not deadlock; expected 1407 response, got {warp_frames:?}"
    );

    // Verify socket read loop is still alive by sending `/where` chat
    let where_hex = ts_dream::protocol::frame("0202", &ts_dream::protocol::encoder::strhex(b"/where"));
    sock.write_all(&xor_wire(&where_hex)).await.expect("write /where");
    let chat_frames = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.contains("020C"))
    })
    .await;
    assert!(
        chat_frames.iter().any(|f| f.contains("020C")),
        "connection must stay responsive after 14 08 01 00, got {chat_frames:?}"
    );

    drop(sock);
    let _ = control.stop().await;
    lock_online_sessions().remove(&player_id);
}

#[tokio::test]
async fn tcp_map_10817_bach_lao_nhan_15009_completes_without_deadlock() {
    let _db = setup_test_db().await;
    let pool = (*_db).clone();
    let data = Arc::clone(shared_game_data());
    let player_id = 300082u32;

    // Spawn directly within +/-150 px of Bạch Lão Nhân (15009, MapObjectID=4 at 1430, 660)
    let (control, mut sock, mut decoder) =
        connect_and_login(19452, data, pool, player_id, 1430, 660).await;

    // Confirm position with 06 01 walk step
    sock.write_all(&xor_wire(&build_move_hex(1430, 660)))
        .await
        .expect("write move");

    // Click NPC 4 (Bạch Lão Nhân, id# 15009): 14 01 04 00
    sock.write_all(&xor_wire("F444040014010400"))
        .await
        .expect("write 14 01 04 00");

    let first_talk = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
        fs.iter().any(|f| f.len() >= 12 && &f[8..12] == "1401")
    })
    .await;
    assert!(
        first_talk.iter().any(|f| f.len() >= 12 && &f[8..12] == "1401"),
        "expected 1401 dialog from Bạch Lão Nhân, got {first_talk:?}"
    );

    // Advance 6 dialog steps (14 06) until DomainEffect::Warp triggers 14 07 + 0C
    let mut warped = false;
    for _ in 0..7 {
        sock.write_all(&xor_wire("F44402001406"))
            .await
            .expect("write 14 06");
        let step_frames = read_frames_until(&mut sock, &mut decoder, 2000, |fs| {
            fs.iter().any(|f| {
                f.len() >= 12
                    && (&f[8..12] == "1401" || &f[8..12] == "1407" || &f[8..10] == "0C")
            })
        })
        .await;
        if step_frames
            .iter()
            .any(|f| f.len() >= 12 && (&f[8..12] == "1407" || &f[8..10] == "0C"))
        {
            warped = true;
            break;
        }
    }
    assert!(
        warped,
        "Bạch Lão Nhân (15009) eve_no=4 must complete and warp without deadlock"
    );

    drop(sock);
    let _ = control.stop().await;
    lock_online_sessions().remove(&player_id);
}
