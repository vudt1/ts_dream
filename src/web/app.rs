//! Web dashboard (Chapter 7) — HTTP on `:8090`, always up once DB is reachable.
//!
//! Server-rendered page + JSON API + SSE log stream, sharing `AppState`.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json,
};
use serde_json::json;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::state::AppState;

pub type SharedState = Arc<RwLock<AppState>>;

/// Turnstile for dashboard handlers needing the DB.
#[derive(Clone)]
pub struct WebState {
    pub app: SharedState,
    pub pool: Option<MySqlPool>,
}

/// Build the axum router.
pub fn router(state: WebState) -> axum::Router {
    let s = state.clone();
    axum::Router::new()
        .route("/", get(index))
        .route("/api/server/status", get(server_status))
        .route("/api/server/start", get(server_start))
        .route("/api/server/stop", get(server_stop))
        .route("/api/server/announce", get(server_announce))
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route("/api/npcs", get(list_npcs))
        .route("/api/online", get(list_online))
        .route("/api/log/stream", get(log_stream))
        .route("/api/config/perexp", get(set_perexp))
        .with_state(s)
}

async fn index(State(_s): State<WebState>) -> &'static str {
    // Askama page goes here (Chapter 7 §7.3).
    "TS Dream dashboard"
}

async fn server_status(State(_s): State<WebState>) -> Json<serde_json::Value> {
    Json(json!({ "running": _s.app.read().await.running }))
}

async fn server_start(State(_s): State<WebState>) -> Json<serde_json::Value> {
    let mut app = _s.app.write().await;
    app.running = true;
    Json(json!({ "running": true }))
}

async fn server_stop(State(_s): State<WebState>) -> Response {
    // Chapter 7 §7.3: stop when not running -> 409.
    let mut app = _s.app.write().await;
    if !app.running {
        return (StatusCode::CONFLICT, Json(json!({ "error": "server not running" })))
            .into_response();
    }
    app.running = false;
    (StatusCode::OK, Json(json!({ "running": false }))).into_response()
}

async fn server_announce(State(_s): State<WebState>) -> Response {
    let app = _s.app.read().await;
    if !app.running {
        return (StatusCode::CONFLICT, Json(json!({ "error": "server not running" })))
            .into_response();
    }
    Json(json!({ "ok": true })).into_response()
}

async fn list_accounts(
    State(s): State<WebState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match s.pool {
        Some(pool) => {
            let rows: Vec<(i64, String, String)> =
                sqlx::query_as("SELECT player_id, pass1, pass2 FROM accounts ORDER BY player_id")
                    .fetch_all(&pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json!(rows)))
        }
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn create_account(
    State(s): State<WebState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pass1 = body["pass1"].as_str().unwrap_or("").to_string();
    let pass2 = body["pass2"].as_str().unwrap_or("").to_string();
    match s.pool {
        Some(pool) => {
            let res = sqlx::query("INSERT INTO accounts (pass1, pass2) VALUES (?, ?)")
                .bind(&pass1)
                .bind(&pass2)
                .execute(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json!({ "player_id": res.last_insert_id(), "pass1": pass1, "pass2": pass2 })))
        }
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn list_npcs(
    State(_s): State<WebState>,
) -> Json<serde_json::Value> {
    // In-memory Data_Npcs (Chapter 7 §7.3 GET /api/npcs).
    Json(json!({ "count": 0 }))
}

async fn list_online(State(_s): State<WebState>) -> Json<serde_json::Value> {
    let online = _s.app.read().await.online.clone();
    Json(json!(online))
}

async fn log_stream(State(_s): State<WebState>) -> Response {
    // SSE: event log + data {level, ts, msg} from the broadcast receiver.
    let rx = _s.app.read().await.broadcast.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(ev) => {
                let payload = serde_json::to_string(&json!({
                    "level": ev.level, "ts": ev.ts, "msg": ev.msg
                }))
                .unwrap_or_default();
                let chunk =
                    axum::response::sse::Event::default().data(payload);
                Some((Ok::<_, std::convert::Infallible>(chunk), rx))
            }
            Err(_) => None,
        }
    });
    axum::response::sse::Sse::new(stream).into_response()
}

async fn set_perexp(
    State(s): State<WebState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let v = body["value"].as_u64().unwrap_or(0) as u32;
    let mut app = s.app.write().await;
    app.perexp = v;
    Json(json!({ "perexp": v }))
}