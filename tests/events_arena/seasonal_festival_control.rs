//! Integration tests for Ticket 01:
//! SeasonalFestivalManager, SQLite `season_seq` persistence, and Web Admin Control Plane.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use ts_dream::server::events::{EventLifecycleManager, EventPhase, EVENT_DRAGON_BOAT};
use ts_dream::server::seasonal_festival::SeasonalFestivalManager;
use ts_dream::state::AppState;
use ts_dream::web::app::{router, WebState};
use ts_dream::web::server_control::ServerControl;

use crate::common::test_db::setup_test_db;

#[test]
fn test_special_events_table_has_8_events_and_no_seasonal_festival() {
    let mgr = EventLifecycleManager::default();
    let events = mgr.list_events();
    assert_eq!(events.len(), 8, "SpecialEvent table must have exactly 8 events");
    assert!(
        !events.iter().any(|e| e.id == "seasonal_festival"),
        "seasonal_festival placeholder must be removed from SpecialEvent"
    );
    for expected_id in [
        "north_star",
        "continuous_40_npc",
        "rescue_2k",
        "nine_heavens",
        "arena",
        "guild_siege",
        "water_war",
        "dragon_boat",
    ] {
        assert!(
            events.iter().any(|e| e.id == expected_id),
            "SpecialEvent must contain {expected_id}"
        );
    }
}

#[test]
fn test_seasonal_festival_manager_has_11_festivals_and_dragon_boat_is_independent() {
    let fest_mgr = SeasonalFestivalManager::default();
    let festivals = fest_mgr.list_festivals();
    assert_eq!(festivals.len(), 11, "SeasonalFestivalManager must manage 11 festivals");

    let expected_ids = [
        "tet_nguyen_dan",
        "tet_nguyen_tieu",
        "valentine",
        "ca_thang_tu",
        "gio_to_hung_vuong",
        "phat_dan",
        "doan_ngo",
        "that_tich",
        "trung_thu",
        "halloween",
        "giang_sinh",
    ];
    for id in expected_ids {
        assert!(
            festivals.iter().any(|f| f.id == id),
            "missing seasonal festival {id}"
        );
    }

    // dragon_boat in EventLifecycleManager remains completely independent of doan_ngo
    let event_mgr = EventLifecycleManager::default();
    assert_eq!(event_mgr.get_event_phase(EVENT_DRAGON_BOAT), EventPhase::Idle);
    assert!(!fest_mgr.is_festival_active("doan_ngo"));
}

#[tokio::test]
async fn test_start_stop_multiple_festivals_and_season_seq_persistence() {
    let db = setup_test_db().await;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;
    app.write().await.data_loaded = true;

    let control = Arc::new(ServerControl::new(19301, app.clone(), None, Some(db.clone())));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // 1. Start two seasonal festivals simultaneously (tet_nguyen_tieu & valentine)
    let resp1 = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let resp2 = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/valentine/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let f1 = control.festivals.get_festival("tet_nguyen_tieu").unwrap();
    let f2 = control.festivals.get_festival("valentine").unwrap();
    assert!(f1.is_active);
    assert_eq!(f1.season_seq, 1);
    assert!(f2.is_active);
    assert_eq!(f2.season_seq, 1);

    // 2. Stop tet_nguyen_tieu -> Idle, season_seq preserved at 1
    let resp_stop = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/stop")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_stop.status(), StatusCode::OK);

    let f1_after_stop = control.festivals.get_festival("tet_nguyen_tieu").unwrap();
    assert!(!f1_after_stop.is_active);
    assert_eq!(f1_after_stop.season_seq, 1, "season_seq must be preserved on stop");

    // 3. Watchdog 120m must NOT stop active seasonal festivals
    control.events.check_watchdog();
    assert!(
        control.festivals.is_festival_active("valentine"),
        "valentine must remain active after EventLifecycleManager::check_watchdog()"
    );

    // 4. Simulate server restart: fresh ServerControl hydrates from SQLite on start()
    let app2 = Arc::new(RwLock::new(AppState::new(1)));
    app2.write().await.data_loaded = true;
    let control2 = Arc::new(ServerControl::new(19302, app2.clone(), None, Some(db.clone())));
    control2.start().await.expect("start control2");

    let hydrated_tnt = control2.festivals.get_festival("tet_nguyen_tieu").unwrap();
    let hydrated_val = control2.festivals.get_festival("valentine").unwrap();
    assert!(!hydrated_tnt.is_active);
    assert_eq!(hydrated_tnt.season_seq, 1);
    assert!(hydrated_val.is_active);
    assert_eq!(hydrated_val.season_seq, 1);

    // 5. Starting tet_nguyen_tieu again increments season_seq to 2
    let state2 = WebState {
        app: app2.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control2.clone()),
    };
    let resp_restart = router(state2)
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_restart.status(), StatusCode::OK);
    let f1_restarted = control2.festivals.get_festival("tet_nguyen_tieu").unwrap();
    assert!(f1_restarted.is_active);
    assert_eq!(f1_restarted.season_seq, 2);

    let _ = control2.stop_immediate().await;
}

#[tokio::test]
async fn test_festival_negative_cases_stopped_server_duplicate_and_unknown_id() {
    let db = setup_test_db().await;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = false;

    let control = Arc::new(ServerControl::new(19303, app.clone(), None, Some(db.clone())));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // 1. Stopped server rejects both start and stop with 409 Conflict
    for uri in ["/api/festivals/doan_ngo/start", "/api/festivals/doan_ngo/stop"] {
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
        assert_eq!(resp.status(), StatusCode::CONFLICT, "{uri} must return 409 when server stopped");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let err_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(err_json["error"].as_str().is_some_and(|s| !s.is_empty()));
    }

    // Turn server on
    app.write().await.running = true;

    // 2. Unknown festival ID returns 404 Not Found
    let resp_404 = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/nonexistent_festival/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_404.status(), StatusCode::NOT_FOUND);
    let body_404 = to_bytes(resp_404.into_body(), usize::MAX).await.unwrap();
    let err_404: serde_json::Value = serde_json::from_slice(&body_404).unwrap();
    assert!(err_404["error"].as_str().is_some_and(|s| !s.is_empty()));

    // 3. Stop on already Idle festival returns 409 Conflict
    let resp_stop_idle = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/doan_ngo/stop")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_stop_idle.status(), StatusCode::CONFLICT);
    let body_idle = to_bytes(resp_stop_idle.into_body(), usize::MAX).await.unwrap();
    let err_idle: serde_json::Value = serde_json::from_slice(&body_idle).unwrap();
    assert!(err_idle["error"].as_str().is_some_and(|s| !s.is_empty()));

    // 4. Start -> Active, then duplicate Start returns 409 Conflict
    let resp_ok = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/doan_ngo/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_ok.status(), StatusCode::OK);

    let resp_dup_start = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/doan_ngo/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_dup_start.status(), StatusCode::CONFLICT);

    // Verify dragon_boat in EventLifecycleManager is still Idle (independent of doan_ngo)
    assert_eq!(
        control.events.get_event_phase(EVENT_DRAGON_BOAT),
        EventPhase::Idle,
        "doan_ngo activation must not affect dragon_boat"
    );
}

#[tokio::test]
async fn test_festival_audit_log_and_dashboard_htmx_rendering() {
    let db = setup_test_db().await;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(19304, app.clone(), None, Some(db.clone())));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // Dashboard renders #seasonal-festivals-card
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
    let dash_bytes = to_bytes(dash_resp.into_body(), usize::MAX).await.unwrap();
    let dash_html = String::from_utf8_lossy(&dash_bytes);
    assert!(
        dash_html.contains(r#"id="seasonal-festivals-card""#),
        "dashboard must render #seasonal-festivals-card"
    );
    assert!(
        dash_html.contains("/api/festivals/tet_nguyen_dan/start"),
        "dashboard must include start endpoint for tet_nguyen_dan"
    );

    // GET /api/festivals returns 11 festivals in JSON
    let list_resp = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_bytes = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
    let list_json: serde_json::Value = serde_json::from_slice(&list_bytes).unwrap();
    assert_eq!(list_json.as_array().unwrap().len(), 11);

    // HTMX start returns HTML fragment with #seasonal-festivals-card
    let htmx_start = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/trung_thu/start")
                .method("POST")
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(htmx_start.status(), StatusCode::OK);
    let htmx_bytes = to_bytes(htmx_start.into_body(), usize::MAX).await.unwrap();
    let htmx_html = String::from_utf8_lossy(&htmx_bytes);
    assert!(htmx_html.contains(r#"id="seasonal-festivals-card""#));
    assert!(htmx_html.contains("Active"));

    // Verify admin_audit_log recorded festival_start
    let logs: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT action, target_type, details FROM admin_audit_log WHERE target_type = 'seasonal_festival'",
    )
    .fetch_all(&db.read)
    .await
    .unwrap();
    assert!(
        logs.iter().any(|(a, _, d)| a == "festival_start" && d.as_deref().unwrap_or("").contains("trung_thu")),
        "admin_audit_log must record festival_start for trung_thu"
    );
}
