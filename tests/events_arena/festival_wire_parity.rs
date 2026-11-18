//! Integration tests for Ticket 02:
//! Festival.Dat lookup_key & collision detection, S→C 0x24 0x19 wire parity,
//! SeasonalNotice (S→C 0x17 0x39), Fireworks (S→C 0x17 0x46), and live Dashboard broadcasts.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::festival::FestivalDatLoader;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::festival::{
    build_festival_ui_trigger_raw, build_firework_effect_raw, festival_ui_trigger_frame,
    firework_effect_frame, SeasonalNotice,
};
use ts_dream::server::seasonal_festival::{
    FESTIVAL_TET_NGUYEN_DAN, FESTIVAL_TET_NGUYEN_TIEU, FESTIVAL_TRUNG_THU,
};
use ts_dream::server::session::Conn;
use ts_dream::state::AppState;
use ts_dream::web::app::{router, WebState};
use ts_dream::web::server_control::ServerControl;

use crate::common::test_db::setup_test_db;

#[test]
fn test_festival_dat_loader_derives_lookup_key_and_detects_11_0_collisions() {
    let bytes = std::fs::read("Data/Festival.Dat").expect("read Data/Festival.Dat");
    assert_eq!(bytes.len(), 20310, "Data/Festival.Dat must remain untouched at 20,310 bytes");

    let records = FestivalDatLoader::load(&bytes).expect("parse Festival.Dat");
    assert_eq!(records.len(), 7);

    // Record #1 (Tết Nguyên Đán) is the lowest ID with 11.0 -> gets unique_lore_key = Some(11)
    let rec1 = records.get(&1).expect("record 1 must exist");
    assert_eq!(rec1.date_time, 11.0);
    assert_eq!(rec1.lookup_key, Some(11));
    assert_eq!(rec1.unique_lore_key, Some(11));

    // Records #2..#7 collide on 11.0 -> unique_lore_key MUST be None
    for id in 2..=7 {
        let rec = records.get(&id).expect("record must exist");
        assert_eq!(rec.date_time, 11.0);
        assert_eq!(rec.lookup_key, Some(11));
        assert_eq!(
            rec.unique_lore_key, None,
            "colliding record #{id} must have unique_lore_key = None"
        );
    }

    assert!(
        FestivalDatLoader::has_lore_collisions(&records),
        "unpatched Data/Festival.Dat must report lore collisions"
    );
}

#[test]
fn test_festival_dat_loader_unique_keys_and_invalid_dt_handling() {
    // Build a synthetic 10-record Festival.Dat (7 records + dummy 0 + keyA + keyB)
    // with zero keys (XOR with 0 = identity) and distinct date_time values.
    let rec_size = FestivalDatLoader::RECORD_SIZE;
    let mut buf = vec![0u8; 10 * rec_size];

    let test_dts: [(usize, f64); 7] = [
        (1, 1.0),
        (2, 2.0),
        (3, 0.0),       // invalid: < 1.0
        (4, -5.0),      // invalid: negative
        (5, 5.5),       // invalid: fractional
        (6, f64::NAN),  // invalid: NaN
        (7, 7.0),
    ];

    for (idx, dt) in test_dts {
        let off = idx * rec_size;
        buf[off..off + 8].copy_from_slice(&dt.to_le_bytes());
    }

    let records = FestivalDatLoader::load(&buf).expect("parse synthetic Festival.Dat");
    assert_eq!(records.len(), 7);
    assert_eq!(records[&1].lookup_key, Some(1));
    assert_eq!(records[&1].unique_lore_key, Some(1));
    assert_eq!(records[&2].lookup_key, Some(2));
    assert_eq!(records[&2].unique_lore_key, Some(2));
    assert_eq!(records[&3].lookup_key, None);
    assert_eq!(records[&3].unique_lore_key, None);
    assert_eq!(records[&4].lookup_key, None);
    assert_eq!(records[&5].lookup_key, None);
    assert_eq!(records[&6].lookup_key, None);
    assert_eq!(records[&7].lookup_key, Some(7));
    assert_eq!(records[&7].unique_lore_key, Some(7));
    assert!(!FestivalDatLoader::has_lore_collisions(&records));
}

#[test]
fn test_seasonal_notice_1_byte_2_byte_and_exclusion_parity() {
    // 1-byte sub2 variants
    assert_eq!(
        SeasonalNotice::TetCountdownStart.to_raw_frame(),
        vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x39, 0x01]
    );
    assert_eq!(
        SeasonalNotice::DoanNgoZongzi.to_raw_frame(),
        vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x39, 0x03]
    );
    assert_eq!(
        SeasonalNotice::NoelLuLongStart.to_raw_frame(),
        vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x39, 0x15]
    );

    // 2-byte sub2 + sub3 variants
    assert_eq!(
        SeasonalNotice::NianBeast(2).to_raw_frame(),
        vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, 0x44, 0x02]
    );
    assert_eq!(
        SeasonalNotice::NoelPoke(1).to_raw_frame(),
        vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, 0x2B, 0x01]
    );
    assert_eq!(
        SeasonalNotice::QixiFestival(2).to_raw_frame(),
        vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, 0x35, 0x02]
    );
    assert_eq!(
        SeasonalNotice::MidAutumnFestival(6).to_raw_frame(),
        vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, 0x39, 0x06]
    );
    assert_eq!(
        SeasonalNotice::DoanNgoLeiFengTower(3).to_raw_frame(),
        vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, 0x33, 0x03]
    );

    // Valid client branch range: 0x01..=0x0D (13) + 0x15..=0x4A (54) = 67 branches (gap 0x0E..=0x14 excluded)
    let valid_count = (0x01u8..=0xFF)
        .filter(|&b| SeasonalNotice::is_valid_client_branch(b))
        .count();
    assert_eq!(valid_count, 67, "FUN_00722cd4 has 67 branches (0x01..=0x0D and 0x15..=0x4A)");

    // Gap 0x0E..=0x14 must be rejected
    for gap in 0x0Eu8..=0x14 {
        assert!(!SeasonalNotice::is_valid_client_branch(gap));
        assert!(SeasonalNotice::from_festival_code(gap, None).is_none());
        assert!(SeasonalNotice::from_festival_code(gap, Some(1)).is_none());
    }

    // Non-festival codes must be excluded from festival notice emission
    for non_fest in [0x07u8, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x36, 0x37, 0x38, 0x40, 0x41] {
        assert!(
            SeasonalNotice::from_festival_code(non_fest, Some(1)).is_none(),
            "non-festival code 0x{non_fest:02X} must be rejected"
        );
    }

    // 2-byte variants require valid sub3
    assert!(SeasonalNotice::from_festival_code(0x44, None).is_none());
    assert!(SeasonalNotice::from_festival_code(0x44, Some(3)).is_none());
    assert_eq!(
        SeasonalNotice::from_festival_code(0x44, Some(1)),
        Some(SeasonalNotice::NianBeast(1))
    );
}

#[test]
fn test_firework_effect_17_46_raw_and_hex_parity() {
    let raw = build_firework_effect_raw(1, 500, 400, 5);
    assert_eq!(
        raw,
        vec![
            0xF4, 0x44, 0x08, 0x00, // header + len=8
            0x17, 0x46, 0x01,       // op 17, sub 46, sub3=1
            0xF4, 0x01,             // x = 500 (0x01F4 LE)
            0x90, 0x01,             // y = 400 (0x0190 LE)
            0x05                    // count = 5
        ]
    );
    assert_eq!(
        firework_effect_frame(1, 500, 400, 5),
        "F4440800174601F401900105"
    );
}

#[tokio::test]
async fn test_handle_festival_24_19_dispatcher_only_emits_for_active_unique_lore_key() {
    let mut data = GameData::default();
    let bytes = std::fs::read("Data/Festival.Dat").expect("read Data/Festival.Dat");
    data.festivals = FestivalDatLoader::load(&bytes).expect("parse Festival.Dat");
    let data_arc = Arc::new(data);

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;
    let control = Arc::new(ServerControl::new(0, app, Some(data_arc.clone()), None));
    let service = BattleService::default();

    let c2s_24_19 = vec![0xF4, 0x44, 0x02, 0x00, 0x24, 0x19];
    let env = ServerEnv {
        pool: None,
        repos: None,
        hub: Some(&control),
        sender: None,
    };

    // 1. All festivals Idle -> emits nothing
    let mut conn = Conn::default();
    let out_idle = dispatch(&mut conn, &c2s_24_19, &data_arc, &service, &env).await;
    assert!(
        out_idle.outgoing.is_empty(),
        "C→S 0x24 0x19 must emit nothing when all festivals are Idle"
    );

    // 2. Activate colliding festival #2 (tet_nguyen_tieu) -> still emits nothing!
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_TIEU, true, None)
        .await
        .unwrap();
    let out_colliding = dispatch(&mut conn, &c2s_24_19, &data_arc, &service, &env).await;
    assert!(
        out_colliding.outgoing.is_empty(),
        "C→S 0x24 0x19 must NOT emit u32=11 for colliding festival #2"
    );

    // 3. Activate #1 (tet_nguyen_dan) -> emits S→C 0x24 0x19 with u32 LE = 11
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, None)
        .await
        .unwrap();
    let out_tet = dispatch(&mut conn, &c2s_24_19, &data_arc, &service, &env).await;
    assert_eq!(out_tet.outgoing.len(), 1);
    assert_eq!(out_tet.outgoing[0].frame, "F444060024190B000000");

    // 4. Verify build_festival_ui_trigger_raw / festival_ui_trigger_frame helper
    assert_eq!(
        build_festival_ui_trigger_raw(11),
        vec![0xF4, 0x44, 0x06, 0x00, 0x24, 0x19, 0x0B, 0x00, 0x00, 0x00]
    );
    assert_eq!(festival_ui_trigger_frame(11), "F444060024190B000000");

    // 5. ServerEnv::none() -> emits nothing
    let out_none = dispatch(&mut conn, &c2s_24_19, &data_arc, &service, &ServerEnv::none()).await;
    assert!(out_none.outgoing.is_empty());
    let _ = FESTIVAL_TRUNG_THU;
}

#[tokio::test]
async fn test_festival_start_stop_lore_and_firework_live_broadcasts_and_audit() {
    let db = setup_test_db().await;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(19310, app.clone(), None, Some(db.clone())));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
    assert!(control.login_register(1001, &tx).await);

    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // 1. Start tet_nguyen_dan -> broadcasts S→C 17 39 01
    let resp_start = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_dan/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_start.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().expect("must broadcast 17 39 on start"),
        SeasonalNotice::TetCountdownStart.to_frame().frame
    );

    // 2. Stop tet_nguyen_dan -> broadcasts S→C 17 39 02
    let resp_stop = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_dan/stop")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_stop.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().expect("must broadcast 17 39 on stop"),
        SeasonalNotice::TetCountdownEnd.to_frame().frame
    );

    // 3. POST /api/festivals/tet_nguyen_dan/lore -> 200 OK, broadcasts 24 19 [11 LE], logs audit
    let resp_lore = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_dan/lore")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_lore.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().expect("must broadcast 24 19 on lore"),
        "F444060024190B000000"
    );

    // 4. POST /api/festivals/firework -> 200 OK, broadcasts 17 46, logs audit
    let resp_fw = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/firework")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"sub":1,"x":500,"y":400,"count":5}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_fw.status(), StatusCode::OK);
    assert_eq!(
        rx.try_recv().expect("must broadcast 17 46 on firework"),
        "F4440800174601F401900105"
    );

    // 5. Verify admin_audit_log recorded festival_lore and festival_firework
    let logs: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT action, details FROM admin_audit_log WHERE target_type = 'seasonal_festival'",
    )
    .fetch_all(&db.read)
    .await
    .unwrap();
    assert!(logs.iter().any(|(a, d)| a == "festival_lore" && d.as_deref().unwrap_or("").contains("tet_nguyen_dan")));
    assert!(logs.iter().any(|(a, d)| a == "festival_firework" && d.as_deref().unwrap_or("").contains("count=5")));
}

#[tokio::test]
async fn test_festival_lore_and_firework_negative_cases_and_dashboard_ui() {
    let db = setup_test_db().await;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = false;

    let control = Arc::new(ServerControl::new(19311, app.clone(), None, Some(db.clone())));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // 1. Stopped server -> 409 Conflict + JSON {"error": ...} for lore and firework
    for uri in ["/api/festivals/tet_nguyen_dan/lore", "/api/festivals/firework"] {
        let resp = router(state.clone())
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method("POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT, "{uri} must return 409 when stopped");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().is_some_and(|s| !s.is_empty()));
    }

    // Start server
    app.write().await.running = true;

    // 2. Unknown festival ID -> 404 Not Found + JSON {"error": ...}
    let resp_404 = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/unknown_fest/lore")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_404.status(), StatusCode::NOT_FOUND);
    let body_404 = to_bytes(resp_404.into_body(), usize::MAX).await.unwrap();
    let json_404: serde_json::Value = serde_json::from_slice(&body_404).unwrap();
    assert!(json_404["error"].as_str().is_some_and(|s| !s.is_empty()));

    // 3. Colliding festival (#2..#7 on unpatched Festival.Dat) -> 409 Conflict + JSON {"error": ...}
    let resp_collision = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/lore")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_collision.status(), StatusCode::CONFLICT);
    let body_col = to_bytes(resp_collision.into_body(), usize::MAX).await.unwrap();
    let json_col: serde_json::Value = serde_json::from_slice(&body_col).unwrap();
    assert!(
        json_col["error"]
            .as_str()
            .is_some_and(|s| s.contains("collision") || s.contains("unpatched")),
        "error JSON must explain collision: {json_col}"
    );

    // 4. Dashboard renders Lore Key column, collision warning, disabled Lore button for #2, and Firework button
    let dash_resp = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dash_resp.status(), StatusCode::OK);
    let dash_html = String::from_utf8_lossy(&to_bytes(dash_resp.into_body(), usize::MAX).await.unwrap()).to_string();
    assert!(dash_html.contains("Lore Key"));
    assert!(dash_html.contains("Key 11"));
    assert!(dash_html.contains("Collision"));
    assert!(dash_html.contains("/api/festivals/firework"));
    assert!(dash_html.contains("/api/festivals/tet_nguyen_dan/lore"));
}
