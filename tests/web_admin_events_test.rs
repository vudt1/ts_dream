//! Integration tests for Ticket 03: Web Admin Special Events Control Plane,
//! Server Gating, Strict Mutex, Cascading Emergency Drain, and Audit Logging.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use ts_dream::server::events::{EVENT_NORTH_STAR, EventPhase};
use ts_dream::server::session::{lock_online_sessions, Session};
use ts_dream::state::AppState;
use ts_dream::web::app::{router, WebState};
use ts_dream::web::server_control::ServerControl;

mod common;
use common::test_db::setup_test_db;

#[tokio::test]
async fn test_stopped_server_rejection_gate_returns_409() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    // Server is stopped (running = false)
    app.write().await.running = false;

    let control = Arc::new(ServerControl::new(19221, app.clone(), None, None));
    let state = WebState {
        app: app.clone(),
        pool: None,
        data: None,
        server_control: Some(control.clone()),
    };
    let app_router = router(state);

    let req = Request::builder()
        .uri("/api/events/north_star/start")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let resp = app_router.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "stopped server must reject event start with 409 Conflict"
    );

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(
        body_str.contains("Game server is stopped. Please start the server first"),
        "error response must contain exact client message: got {}",
        body_str
    );
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Idle,
        "event must remain Idle"
    );
}

#[tokio::test]
async fn test_strict_mutex_duplicate_start_returns_409() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(19222, app.clone(), None, None));
    let state = WebState {
        app: app.clone(),
        pool: None,
        data: None,
        server_control: Some(control.clone()),
    };

    // 1. First start: success -> Active
    let req1 = Request::builder()
        .uri("/api/events/north_star/start")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let resp1 = router(state.clone()).oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Active,
        "event must transition to Active"
    );

    // 2. Second start while Active: rejected with 409 Conflict (Strict Mutex)
    let req2 = Request::builder()
        .uri("/api/events/north_star/start")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let resp2 = router(state).oneshot(req2).await.unwrap();
    assert_eq!(
        resp2.status(),
        StatusCode::CONFLICT,
        "Strict Mutex must reject duplicate activation with 409 Conflict"
    );

    let bytes = to_bytes(resp2.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(
        body_str.contains("already running"),
        "error response must indicate duplicate event"
    );
}

#[tokio::test]
async fn test_graceful_stop_and_starvation_watchdog() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(19223, app.clone(), None, None));
    let state = WebState {
        app: app.clone(),
        pool: None,
        data: None,
        server_control: Some(control.clone()),
    };

    // Activate event first
    let _ = control.events.start_event(EVENT_NORTH_STAR, true);
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Active
    );

    // Graceful stop via POST /api/events/{id}/stop -> transitions to Ending (3-minute drain)
    let stop_req = Request::builder()
        .uri("/api/events/north_star/stop")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let stop_resp = router(state.clone()).oneshot(stop_req).await.unwrap();
    assert_eq!(stop_resp.status(), StatusCode::OK);
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Ending,
        "event must enter Ending phase for 3-minute drain"
    );

    // Stopping already non-active event returns 409
    let _ = control.events.force_idle(EVENT_NORTH_STAR);
    let stop_idle_req = Request::builder()
        .uri("/api/events/north_star/stop")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let stop_idle_resp = router(state).oneshot(stop_idle_req).await.unwrap();
    assert_eq!(
        stop_idle_resp.status(),
        StatusCode::CONFLICT,
        "stopping Idle event must return 409 Conflict"
    );

    // Starvation watchdog: simulate event running past 120 minutes
    let _ = control.events.start_event(EVENT_NORTH_STAR, true);
    control.events.check_watchdog(); // Not yet expired
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Active
    );
}

#[tokio::test]
async fn test_cascading_emergency_stop_on_tcp_shutdown() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(19224, app.clone(), None, None));

    // 1. Activate special events
    let _ = control.events.start_event(EVENT_NORTH_STAR, true);
    let _ = control.events.start_event(ts_dream::server::events::EVENT_40_NPC, true);
    assert!(control.events.is_event_active(EVENT_NORTH_STAR));
    assert!(control.events.is_event_active(ts_dream::server::events::EVENT_40_NPC));

    // 2. Register an active quiz battle
    let (_q, _frame) = control.events.register_quiz(888, 8001, None);
    assert!(control.events.is_north_star_battle(888));

    // 3. Place an online player inside a continuous dungeon map (e.g. 10851)
    let mut session = Session::default();
    session.id = 8002;
    session.map_id = 10851;
    session.savemap = 12001;
    lock_online_sessions().insert(8002, session);

    // 4. Trigger TCP shutdown stop_immediate
    let _ = control.stop_immediate().await;

    // 5. Verify cascading emergency drain:
    // - All events cascaded to Idle
    assert_eq!(
        control.events.get_event_phase(EVENT_NORTH_STAR),
        EventPhase::Idle,
        "North Star must cascade to Idle"
    );
    assert_eq!(
        control.events.get_event_phase(ts_dream::server::events::EVENT_40_NPC),
        EventPhase::Idle,
        "40 NPC must cascade to Idle"
    );
    // - Active quiz sessions cleared
    assert!(
        !control.events.is_north_star_battle(888),
        "active quiz battles must be terminated"
    );
    // - Server stopped
    assert!(!control.is_running().await);
}

#[tokio::test]
async fn test_admin_audit_log_persistence() {
    let db = setup_test_db().await;

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(
        19225,
        app.clone(),
        None,
        Some(db.clone()),
    ));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // Trigger start
    let start_req = Request::builder()
        .uri("/api/events/north_star/start")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let resp = router(state.clone()).oneshot(start_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Trigger stop
    let stop_req = Request::builder()
        .uri("/api/events/north_star/stop")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let resp = router(state).oneshot(stop_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Query admin_audit_log
    let logs: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT action, target_type, details FROM admin_audit_log ORDER BY id ASC",
    )
    .fetch_all(&db.read)
    .await
    .expect("query admin_audit_log");

    assert!(logs.len() >= 2, "must have at least 2 audit entries");
    let has_start = logs.iter().any(|(act, tgt, det)| {
        act == "event_start"
            && tgt.as_deref() == Some("special_event")
            && det.as_deref() == Some("north_star")
    });
    let has_stop = logs.iter().any(|(act, tgt, det)| {
        act == "event_stop"
            && tgt.as_deref() == Some("special_event")
            && det.as_deref() == Some("north_star")
    });

    assert!(has_start, "audit log must record event_start");
    assert!(has_stop, "audit log must record event_stop");
}

#[tokio::test]
async fn test_north_star_encounter_rate_slider_api() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    let control = Arc::new(ServerControl::new(19226, app.clone(), None, None));
    let state = WebState {
        app: app.clone(),
        pool: None,
        data: None,
        server_control: Some(control.clone()),
    };

    // Default rate is 5%
    assert_eq!(control.events.get_north_star_rate(), 5);

    // Set rate to 18% via JSON POST /api/events/north_star/rate
    let req = Request::builder()
        .uri("/api/events/north_star/rate")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"rate": 18}"#))
        .unwrap();

    let resp = router(state.clone()).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(control.events.get_north_star_rate(), 18);

    // Test upper clamp at 30%
    let req_max = Request::builder()
        .uri("/api/events/north_star/rate")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"rate": 99}"#))
        .unwrap();
    let _ = router(state.clone()).oneshot(req_max).await.unwrap();
    assert_eq!(control.events.get_north_star_rate(), 30, "rate must be clamped to 30");

    // Test lower clamp at 1%
    let req_min = Request::builder()
        .uri("/api/events/north_star/rate")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"rate": 0}"#))
        .unwrap();
    let _ = router(state).oneshot(req_min).await.unwrap();
    assert_eq!(control.events.get_north_star_rate(), 1, "rate must be clamped to 1");
}

#[tokio::test]
async fn test_encounter_rate_audit_log_persisted() {
    let db = setup_test_db().await;

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;

    let control = Arc::new(ServerControl::new(
        19227,
        app.clone(),
        None,
        Some(db.clone()),
    ));
    let state = WebState {
        app: app.clone(),
        pool: Some(db.clone()),
        data: None,
        server_control: Some(control.clone()),
    };

    // Change rate to 20%
    let req = Request::builder()
        .uri("/api/events/north_star/rate")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"rate": 20}"#))
        .unwrap();

    let resp = router(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify audit log has event_rate_change
    let logs: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT action, target_type, details FROM admin_audit_log WHERE action = 'event_rate_change'",
    )
    .fetch_all(&db.read)
    .await
    .expect("query admin_audit_log");

    assert_eq!(logs.len(), 1, "must have 1 audit entry for rate change");
    assert_eq!(logs[0].0, "event_rate_change");
    assert_eq!(logs[0].1.as_deref(), Some("special_event"));
    assert_eq!(logs[0].2.as_deref(), Some("rate=20"));
}

