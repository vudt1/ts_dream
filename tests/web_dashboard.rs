use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::util::ServiceExt;
use ts_dream::data::loader::GameData;
use ts_dream::state::AppState;
use ts_dream::web::app::{router, WebState};
use ts_dream::web::server_control::ServerControl;

fn mock_web_state() -> WebState {
    let app_state = Arc::new(RwLock::new(AppState::new(0)));
    let data = Arc::new(GameData::default());
    let server_control = Arc::new(ServerControl::new(6414, app_state.clone(), Some(data.clone()), None));

    WebState {
        app: app_state,
        pool: None,
        data: Some(data),
        server_control: Some(server_control),
    }
}

#[tokio::test]
async fn test_index_page_and_static_htmx() {
    let app = router(mock_web_state());

    // 1. GET /
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("TS DREAM ADMIN"));
    assert!(body_str.contains("/static/htmx.js"));

    // 2. GET /static/htmx.js
    let response_js = app
        .oneshot(
            Request::builder()
                .uri("/static/htmx.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response_js.status(), StatusCode::OK);
    assert_eq!(
        response_js.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/javascript"
    );
}

#[tokio::test]
async fn test_server_status_and_lifecycle() {
    let state = mock_web_state();
    let app = router(state.clone());

    // Initial status -> false
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/server/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json: Value = serde_json::from_slice(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json["running"], false);

    // Stop when stopped -> 409 Conflict
    let res_stop_409 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/server/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_stop_409.status(), StatusCode::CONFLICT);

    // Announce when stopped -> 409 Conflict
    let res_ann_409 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/server/announce")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "text": "Hello" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ann_409.status(), StatusCode::CONFLICT);

    // Start -> 200 OK
    let res_start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/server/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_start.status(), StatusCode::OK);

    // Status -> true
    let res_status_true = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/server/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json_status: Value = serde_json::from_slice(&axum::body::to_bytes(res_status_true.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json_status["running"], true);

    // Announce when running -> 200 OK
    let res_ann_ok = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/server/announce")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "text": "Server maintenance in 10 mins" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ann_ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_perexp_configuration() {
    let state = mock_web_state();
    let app = router(state.clone());

    // GET /api/config/perexp -> initial 0
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/config/perexp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // POST /api/config/perexp with JSON -> 10
    let res_set = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/perexp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "value": 10 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_set.status(), StatusCode::OK);
    let json: Value = serde_json::from_slice(&axum::body::to_bytes(res_set.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json["perexp"], 10);

    // Check AppState
    assert_eq!(state.app.read().await.perexp, 10);
}

#[tokio::test]
async fn test_npcs_and_online_endpoints() {
    let state = mock_web_state();
    state.app.write().await.online.push(ts_dream::state::OnlineEntry {
        id: 300001,
        name: "TestPlayer".to_string(),
        ip: "127.0.0.1".to_string(),
    });

    let app = router(state);

    // GET /api/online
    let res_online = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/online")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_online.status(), StatusCode::OK);
    let online_json: Value = serde_json::from_slice(&axum::body::to_bytes(res_online.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(online_json[0]["name"], "TestPlayer");

    // GET /api/npcs
    let res_npcs = app
        .oneshot(
            Request::builder()
                .uri("/api/npcs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_npcs.status(), StatusCode::OK);
}
