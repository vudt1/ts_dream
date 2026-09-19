//! Unit and integration test suite for Movement (Opcode 0x06), State Sync (Opcode 0x05),
//! Map Warp (Opcode 0x0C, 0x14), and Persistence of character coordinates.

use sqlx::Row;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_dream::data::loader::GameData;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::server::auto_save;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::movement;
use ts_dream::server::session::{online_sessions, Conn, Session};
use ts_dream::server::spawn;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

async fn setup_test_db() -> DbPool {
    let pool = bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap in-memory SQLite");
    let schema = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(schema)
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
    pool
}

fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a ts_dream::battle::service::BattleService,
    out: &'a mut HandleOutcome,
    opcode: u8,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

// =========================================================================
// 1. Movement tests (Opcode 0x06 sub 1 and sub 2)
// =========================================================================

#[test]
fn test_movement_voluntary_walk_sub1() {
    let mut conn = Conn::default();
    conn.session.id = 1001;
    conn.session.map_id = 10817;
    conn.session.map_x = 100;
    conn.session.map_y = 200;
    conn.session.gocnhin = 0;

    online_sessions()
        .lock()
        .unwrap()
        .insert(1001, conn.session.clone());

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // 9-byte C->S payload: [sub=1, orient=3, x=240 (0xF0, 0x00), y=480 (0xE0, 0x01), sigA=10, sigB=20]
    // payload passed to handle_move starts at payload[0]=orient=3
    let payload = [3, 240, 0, 224, 1, 10, 20];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x06, 1, &payload);

    movement::handle_move(&mut ctx);

    // Verify session coordinates updated
    assert_eq!(conn.session.gocnhin, 3);
    assert_eq!(conn.session.map_x, 240);
    assert_eq!(conn.session.map_y, 480);

    // Verify shared online registry updated
    let reg = online_sessions().lock().unwrap();
    let s = reg.get(&1001).expect("player in registry");
    assert_eq!(s.gocnhin, 3);
    assert_eq!(s.map_x, 240);
    assert_eq!(s.map_y, 480);
    drop(reg);

    // Verify move broadcast generated
    assert_eq!(out.map_broadcast.len(), 1);
    assert_eq!(out.map_broadcast[0].subject, 1001);
    assert!(out.map_broadcast[0].frame.starts_with("F4440B000601"));

    online_sessions().lock().unwrap().remove(&1001);
}

#[test]
fn test_movement_echo_reconciliation_sub2() {
    let mut conn = Conn::default();
    conn.session.id = 1002;
    conn.session.map_id = 10817;
    conn.session.map_x = 100;
    conn.session.map_y = 200;

    online_sessions()
        .lock()
        .unwrap()
        .insert(1002, conn.session.clone());

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // Sub 2 echo reconciliation payload: orient=5, x=350, y=550
    let payload = [5, 94, 1, 38, 2, 15, 30]; // 350 = 0x015E, 550 = 0x0226
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x06, 2, &payload);

    movement::handle_move(&mut ctx);

    // Coords reconciled
    assert_eq!(conn.session.gocnhin, 5);
    assert_eq!(conn.session.map_x, 350);
    assert_eq!(conn.session.map_y, 550);

    // Shared registry updated
    let reg = online_sessions().lock().unwrap();
    let s = reg.get(&1002).unwrap();
    assert_eq!(s.map_x, 350);
    assert_eq!(s.map_y, 550);
    drop(reg);

    // Server must release client walk lock with [14 08]
    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(out.outgoing[0].frame, "F44402001408");

    // Must NOT broadcast walk for echo reconciliation
    assert!(out.map_broadcast.is_empty());

    online_sessions().lock().unwrap().remove(&1002);
}

#[test]
fn test_opcode_05_non_movement() {
    let mut conn = Conn::default();
    conn.session.id = 1003;
    conn.session.map_x = 100;
    conn.session.map_y = 200;

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // Opcode 0x05 sub 6 (form selection e.g. death/revive)
    let payload = [1, 2, 3];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x05, 6, &payload);
    movement::handle_player_update(&mut ctx);

    // Coords must NOT be altered
    assert_eq!(conn.session.map_x, 100);
    assert_eq!(conn.session.map_y, 200);
    assert!(out.map_broadcast.is_empty());
    assert!(out.outgoing.is_empty());
}

#[test]
fn test_party_movement_follow() {
    let mut leader_conn = Conn::default();
    leader_conn.session.id = 2001;
    leader_conn.session.id_leader = 2001;
    leader_conn.session.id_mem = [2002, 0, 0, 0];
    leader_conn.session.map_id = 10817;
    leader_conn.session.map_x = 100;
    leader_conn.session.map_y = 100;

    let member_session = Session {
        id: 2002,
        id_leader: 2001,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
        ..Default::default()
    };

    online_sessions()
        .lock()
        .unwrap()
        .insert(2001, leader_conn.session.clone());
    online_sessions()
        .lock()
        .unwrap()
        .insert(2002, member_session.clone());

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    // Leader moves to (300, 400), dir 2
    let payload = [2, 44, 1, 144, 1, 0, 0]; // 300 = 0x012C, 400 = 0x0190
    let mut ctx = test_ctx(&mut leader_conn, &data, &service, &mut out, 0x06, 1, &payload);
    movement::handle_move(&mut ctx);

    // Leader coords updated
    assert_eq!(leader_conn.session.map_x, 300);
    assert_eq!(leader_conn.session.map_y, 400);

    // Member coords in online registry updated
    let reg = online_sessions().lock().unwrap();
    let mem = reg.get(&2002).unwrap();
    assert_eq!(mem.map_x, 300);
    assert_eq!(mem.map_y, 400);
    assert_eq!(mem.gocnhin, 2);
    drop(reg);

    // Two broadcasts: one for leader, one for follower
    assert_eq!(out.map_broadcast.len(), 2);
    assert_eq!(out.map_broadcast[0].subject, 2001);
    assert_eq!(out.map_broadcast[1].subject, 2002);

    let mut member_conn = Conn {
        session: member_session,
        ..Default::default()
    };
    let mut member_out = HandleOutcome::default();
    let mut member_ctx = test_ctx(
        &mut member_conn,
        &data,
        &service,
        &mut member_out,
        0x06,
        1,
        &payload,
    );
    movement::handle_move(&mut member_ctx);
    assert_eq!(
        member_conn.session.map_x, 100,
        "Member cannot move independently"
    );
    assert!(member_out.map_broadcast.is_empty());

    online_sessions().lock().unwrap().remove(&2001);
    online_sessions().lock().unwrap().remove(&2002);
}

// =========================================================================
// 2. Warp tests (Opcode 0x0C, 0x14 sub 8, 0x0C sub 1)
// =========================================================================

#[tokio::test]
async fn test_warp_confirm_and_teleport_confirm_cycle() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();

    let mut conn = Conn::default();
    conn.session.id = 3001;
    conn.session.map_id = 49902;
    conn.session.map_x = 200;
    conn.session.map_y = 200;
    conn.session.in_world = true;

    online_sessions()
        .lock()
        .unwrap()
        .insert(3001, conn.session.clone());

    // Step 1: Client triggers warp gate 1 at map 49902
    // (In Warps.txt, (49902, 1) warps to map 49901, x=222, y=295)
    let decoded_gate = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x08, 0x01, 0x00];
    let out_gate = dispatch(&mut conn, &decoded_gate, &data, &service, &env).await;

    // Must send [14 07] fade screen
    assert!(
        out_gate.outgoing.iter().any(|f| f.frame == "F44402001407"),
        "Must send 14 07 fade screen"
    );

    // Must send Opcode 0x0C relocate packet (13 bytes payload)
    let expected_relocate = spawn::build_relocate_packet(3001, 49901, 222, 295, 1);
    assert!(
        out_gate.outgoing.iter().any(|f| f.frame == expected_relocate),
        "Must send Opcode 0x0C relocate packet"
    );

    // Must broadcast hide from old map 49902
    assert_eq!(out_gate.map_broadcast.len(), 1);
    assert_eq!(out_gate.map_broadcast[0].map_id, Some(49902));
    assert_eq!(
        out_gate.map_broadcast[0].frame,
        ts_dream::battle::packets::hide_from_map(3001)
    );

    // Session coordinates updated to destination
    assert_eq!(conn.session.map_id, 49901);
    assert_eq!(conn.session.map_x, 222);
    assert_eq!(conn.session.map_y, 295);

    // Step 2: Client loads new map and sends confirm Opcode 0x0C Sub 1
    let decoded_confirm = [0xF4, 0x44, 0x02, 0x00, 0x0C, 0x01];
    let out_confirm = dispatch(&mut conn, &decoded_confirm, &data, &service, &env).await;

    // Must send [05 04] World-Ready and [14 08] Clear Walk Lock
    assert!(
        out_confirm
            .outgoing
            .iter()
            .any(|f| f.frame == "F44402000504F44402001408"),
        "Must send 05 04 and 14 08 on teleport confirm"
    );

    // Must broadcast player appearance on new map
    assert_eq!(out_confirm.map_broadcast.len(), 1);
    assert_eq!(out_confirm.map_broadcast[0].subject, 3001);
    assert_eq!(out_confirm.map_broadcast[0].map_id, None); // default to new map

    // Online registry must reflect new map
    let reg = online_sessions().lock().unwrap();
    let s = reg.get(&3001).unwrap();
    assert_eq!(s.map_id, 49901);
    assert_eq!(s.map_x, 222);
    assert_eq!(s.map_y, 295);
    drop(reg);

    online_sessions().lock().unwrap().remove(&3001);
}

#[tokio::test]
async fn test_party_warp_follow() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let service = ts_dream::battle::service::BattleService::default();

    let mut leader_conn = Conn::default();
    leader_conn.session.id = 4001;
    leader_conn.session.id_leader = 4001;
    leader_conn.session.id_mem = [4002, 0, 0, 0];
    leader_conn.session.map_id = 49902;
    leader_conn.session.map_x = 200;
    leader_conn.session.map_y = 200;
    leader_conn.session.in_world = true;

    let member_session = Session {
        id: 4002,
        id_leader: 4001,
        map_id: 49902,
        map_x: 200,
        map_y: 200,
        in_world: true,
        ..Default::default()
    };

    online_sessions()
        .lock()
        .unwrap()
        .insert(4001, leader_conn.session.clone());
    online_sessions()
        .lock()
        .unwrap()
        .insert(4002, member_session.clone());

    let env = ServerEnv::none();

    // Leader triggers warp at gate 1 (map 49902 -> map 49901, 222, 295)
    let decoded_gate = [0xF4, 0x44, 0x04, 0x00, 0x14, 0x08, 0x01, 0x00];
    let out = dispatch(&mut leader_conn, &decoded_gate, &data, &service, &env).await;

    // Leader updated
    assert_eq!(leader_conn.session.map_id, 49901);

    // Member in online registry updated to new map
    let reg = online_sessions().lock().unwrap();
    let mem = reg.get(&4002).unwrap();
    assert_eq!(mem.map_id, 49901);
    assert_eq!(mem.map_x, 222);
    assert_eq!(mem.map_y, 295);
    drop(reg);

    // Both leader and member have hide broadcasts on old map 49902
    assert_eq!(out.map_broadcast.len(), 2);
    assert_eq!(out.map_broadcast[0].subject, 4001);
    assert_eq!(out.map_broadcast[0].map_id, Some(49902));
    assert_eq!(out.map_broadcast[1].subject, 4002);
    assert_eq!(out.map_broadcast[1].map_id, Some(49902));

    online_sessions().lock().unwrap().remove(&4001);
    online_sessions().lock().unwrap().remove(&4002);
}

// =========================================================================
// 3. Database Persistence & Dirty Fingerprint tests
// =========================================================================

#[test]
fn test_auto_save_fingerprint_dirty_on_movement() {
    let mut s = Session {
        id: 5001,
        map_id: 10817,
        map_x: 100,
        map_y: 200,
        ..Default::default()
    };

    let fp_initial = auto_save::fingerprint(&s);

    // Change X
    s.map_x = 105;
    let fp_x_changed = auto_save::fingerprint(&s);
    assert_ne!(
        fp_initial, fp_x_changed,
        "Fingerprint must change when map_x changes"
    );

    // Change Y
    s.map_y = 205;
    let fp_y_changed = auto_save::fingerprint(&s);
    assert_ne!(
        fp_x_changed, fp_y_changed,
        "Fingerprint must change when map_y changes"
    );

    // Change map_id
    s.map_id = 10818;
    let fp_map_changed = auto_save::fingerprint(&s);
    assert_ne!(
        fp_y_changed, fp_map_changed,
        "Fingerprint must change when map_id changes"
    );
}

#[tokio::test]
async fn test_disconnect_persists_coordinates_to_db() {
    let pool = setup_test_db().await;

    // Create a character row in SQLite
    let account_id = 6001i64;
    sqlx::query("INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)")
        .bind(account_id)
        .execute(&pool.write)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO characters (playerid, name, level, jobtype, gender, hair, element, rebornstage, curhp, maxhp, cursp, maxsp, freepoints, skillpoint, baseint, baseatk, basedef, basehpx, basespx, baseagi, mapid, mapx, mapy, pk, curexp, newbie)
         VALUES (?, X'746573746572', 1, 0, 1, 1, 1, 0, 100, 100, 50, 50, 0, 0, 10, 10, 10, 10, 10, 10, 10817, 100, 200, 0, 0, 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert character");

    let app = Arc::new(RwLock::new(AppState::new(100)));
    let control = ServerControl::new(6414, app, None, Some(pool.clone()));

    // Populate active online session
    let session = Session {
        id: account_id as u32,
        db_character_id: account_id,
        authed: true,
        name: b"tester".to_vec(),
        map_id: 49902, // Moved to new map
        map_x: 522,
        map_y: 495,
        ..Default::default()
    };

    online_sessions()
        .lock()
        .unwrap()
        .insert(account_id as u32, session);

    // Call disconnect_player
    control.disconnect_player(account_id as u32).await;

    // Verify session removed from online_sessions
    assert!(online_sessions()
        .lock()
        .unwrap()
        .get(&(account_id as u32))
        .is_none());

    // Query SQLite database to verify updated coordinates were saved!
    let row = sqlx::query("SELECT mapid, mapx, mapy FROM characters WHERE playerid = ?")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("fetch character");

    let mapid: i64 = row.get("mapid");
    let mapx: i64 = row.get("mapx");
    let mapy: i64 = row.get("mapy");

    assert_eq!(
        (mapid, mapx, mapy),
        (49902, 522, 495),
        "Coordinates must be persisted into SQLite on disconnect"
    );
}
