//! Web dashboard (Chapter 7) — HTTP on `:8090`, always up once DB is reachable.
//!
//! Server-rendered page + JSON API + SSE log stream, sharing `AppState`.

use askama::Template;
use axum::{
    extract::{FromRequest, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Json,
};
use serde_json::json;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::data::loader::GameData;
use crate::state::{AppState, LogEvent, OnlineEntry};
use crate::web::server_control::ServerControl;

pub type SharedState = Arc<RwLock<AppState>>;

pub const HTMX_JS: &str = include_str!("static/htmx.min.js");

/// Turnstile for dashboard handlers needing the DB, GameData, and ServerControl.
#[derive(Clone)]
pub struct WebState {
    pub app: SharedState,
    pub pool: Option<MySqlPool>,
    pub data: Option<Arc<GameData>>,
    pub server_control: Option<Arc<ServerControl>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AccountRow {
    pub player_id: i64,
    pub pass1: String,
    pub pass2: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NpcRow {
    pub id: i64,
    pub name: String,
    pub lv: i64,
    pub thuoctinh: i64,
    pub hp: i64,
    pub sp: i64,
    pub atk: i64,
    pub def: i64,
    pub agi: i64,
}

impl NpcRow {
    /// Project the loaded NPc tables into display rows, sorted by id.
    pub fn from_data(data: &GameData) -> Vec<NpcRow> {
        let mut npcs: Vec<NpcRow> = data
            .npcs
            .values()
            .map(|n| NpcRow {
                id: n.id,
                name: n
                    .name
                    .iter()
                    .map(|&b| crate::encoding::viscii_to_unicode(b))
                    .collect(),
                lv: n.lv,
                thuoctinh: n.thuoctinh,
                hp: n.hp,
                sp: n.sp,
                atk: n.atk,
                def: n.def,
                agi: n.agi,
            })
            .collect();
        npcs.sort_by_key(|n| n.id);
        npcs
    }
}

/// Respond as HTML when the request came via HTMX (`HX-Request`), else JSON.
fn htmx_or_json(
    headers: &HeaderMap,
    html: impl FnOnce() -> String,
    json: serde_json::Value,
) -> Response {
    if headers.get("HX-Request").is_some() {
        Html(html()).into_response()
    } else {
        Json(json).into_response()
    }
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub running: bool,
    pub perexp: u32,
    pub online_count: usize,
    pub npcs_count: usize,
    pub online_list: Vec<OnlineEntry>,
    pub accounts: Vec<AccountRow>,
    pub npcs: Vec<NpcRow>,
    pub initial_logs: Vec<LogEvent>,
}

impl IntoResponse for DashboardTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template render error: {e}"),
            )
                .into_response(),
        }
    }
}

/// Helper extractor for either Form or JSON payloads.
pub enum FormOrJson<T> {
    Form(T),
    Json(T),
}

impl<T, S> FromRequest<S> for FormOrJson<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("application/json") {
            let Json(val) = Json::<T>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(FormOrJson::Json(val))
        } else {
            let Form(val) = Form::<T>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(FormOrJson::Form(val))
        }
    }
}

/// Build the axum router.
pub fn router(state: WebState) -> axum::Router {
    let s = state.clone();
    axum::Router::new()
        .route("/", get(index))
        .route("/static/htmx.js", get(static_htmx))
        .route("/api/server/status", get(server_status))
        .route("/api/server/start", get(server_start).post(server_start))
        .route("/api/server/stop", get(server_stop).post(server_stop))
        .route("/api/server/announce", get(server_announce).post(server_announce))
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route("/api/npcs", get(list_npcs))
        .route("/api/online", get(list_online))
        .route("/api/log/stream", get(log_stream))
        .route("/api/config/perexp", get(get_perexp).post(set_perexp))
        .with_state(s)
}

async fn static_htmx() -> Response {
    ([(header::CONTENT_TYPE, "application/javascript")], HTMX_JS).into_response()
}

async fn index(State(s): State<WebState>) -> Response {
    let app_guard = s.app.read().await;
    let running = app_guard.running;
    let perexp = app_guard.perexp;
    let online_list = app_guard.online.clone();
    let initial_logs: Vec<LogEvent> = app_guard.log_buffer.iter().cloned().collect();
    drop(app_guard);

    let accounts = match &s.pool {
        Some(pool) => {
            sqlx::query_as::<_, AccountRow>(
                "SELECT player_id, pass1, pass2 FROM accounts ORDER BY player_id DESC",
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
        None => Vec::new(),
    };

    let mut npcs = Vec::new();
    if let Some(ref data) = s.data {
        npcs = NpcRow::from_data(data);
    }

    let template = DashboardTemplate {
        running,
        perexp,
        online_count: online_list.len(),
        npcs_count: npcs.len(),
        online_list,
        accounts,
        npcs,
        initial_logs,
    };

    template.into_response()
}

async fn server_status(State(s): State<WebState>) -> Json<serde_json::Value> {
    Json(json!({ "running": s.app.read().await.running }))
}

async fn server_start(
    State(s): State<WebState>,
    headers: HeaderMap,
) -> Response {
    if let Some(ref control) = s.server_control {
        let _ = control.start().await;
    } else {
        s.app.write().await.running = true;
    }

    htmx_or_json(
        &headers,
        || String::from(r##"
        <div class="card" id="server-control-card">
            <div class="card-header">Server Lifecycle</div>
            <div class="card-value" style="font-size: 20px; margin-bottom: 14px;">
                <span style="color: var(--accent-green)">State: RUNNING</span>
            </div>
            <div style="display: flex; gap: 10px;">
                <button class="btn btn-green" hx-post="/api/server/start" hx-target="#server-control-card" hx-swap="outerHTML">Start</button>
                <button class="btn btn-red" hx-post="/api/server/stop" hx-target="#server-control-card" hx-swap="outerHTML" onclick="return confirm('Stop game server with 5s countdown?')">Stop</button>
            </div>
        </div>
        "##),
        json!({ "running": true }),
    )
}

async fn server_stop(
    State(s): State<WebState>,
    headers: HeaderMap,
) -> Response {
    if let Some(ref control) = s.server_control {
        match control.stop().await {
            Ok(_) => {}
            Err((code, msg)) => {
                return (code, Json(json!({ "error": msg }))).into_response();
            }
        }
    } else {
        let mut app = s.app.write().await;
        if !app.running {
            return (StatusCode::CONFLICT, Json(json!({ "error": "server not running" }))).into_response();
        }
        app.running = false;
    }

    htmx_or_json(
        &headers,
        || String::from(r##"
        <div class="card" id="server-control-card">
            <div class="card-header">Server Lifecycle</div>
            <div class="card-value" style="font-size: 20px; margin-bottom: 14px;">
                <span style="color: var(--accent-red)">State: STOPPED</span>
            </div>
            <div style="display: flex; gap: 10px;">
                <button class="btn btn-green" hx-post="/api/server/start" hx-target="#server-control-card" hx-swap="outerHTML">Start</button>
                <button class="btn btn-red" hx-post="/api/server/stop" hx-target="#server-control-card" hx-swap="outerHTML" onclick="return confirm('Stop game server with 5s countdown?')">Stop</button>
            </div>
        </div>
        "##),
        json!({ "running": false }),
    )
}

#[derive(Debug, serde::Deserialize)]
pub struct AnnouncePayload {
    pub text: Option<String>,
}

async fn server_announce(
    State(s): State<WebState>,
    payload: Result<FormOrJson<AnnouncePayload>, StatusCode>,
) -> Response {
    let text = match payload {
        Ok(FormOrJson::Json(p)) | Ok(FormOrJson::Form(p)) => p.text.unwrap_or_default(),
        Err(_) => String::new(),
    };

    if let Some(ref control) = s.server_control {
        match control.announce(&text).await {
            Ok(_) => Json(json!({ "ok": true })).into_response(),
            Err((code, msg)) => (code, Json(json!({ "error": msg }))).into_response(),
        }
    } else {
        let app = s.app.read().await;
        if !app.running {
            return (StatusCode::CONFLICT, Json(json!({ "error": "server not running" }))).into_response();
        }
        Json(json!({ "ok": true })).into_response()
    }
}

async fn list_accounts(
    State(s): State<WebState>,
) -> Result<Json<Vec<AccountRow>>, StatusCode> {
    match s.pool {
        Some(ref pool) => {
            let rows: Vec<AccountRow> =
                sqlx::query_as("SELECT player_id, pass1, pass2 FROM accounts ORDER BY player_id")
                    .fetch_all(pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(rows))
        }
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateAccountPayload {
    pub pass1: String,
    pub pass2: String,
}

async fn create_account(
    State(s): State<WebState>,
    headers: HeaderMap,
    payload: Result<FormOrJson<CreateAccountPayload>, StatusCode>,
) -> Response {
    let payload = match payload {
        Ok(FormOrJson::Json(p)) | Ok(FormOrJson::Form(p)) => p,
        Err(status) => return status.into_response(),
    };

    let pool = match s.pool {
        Some(ref p) => p,
        None => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let res = match sqlx::query("INSERT INTO accounts (pass1, pass2) VALUES (?, ?)")
        .bind(&payload.pass1)
        .bind(&payload.pass2)
        .execute(pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let player_id = res.last_insert_id() as i64;
    htmx_or_json(
        &headers,
        || {
            format!(
                "<tr><td><strong>{}</strong></td><td>{}</td><td>{}</td></tr>",
                player_id, payload.pass1, payload.pass2
            )
        },
        json!({
            "player_id": player_id,
            "pass1": payload.pass1,
            "pass2": payload.pass2
        }),
    )
}

async fn list_npcs(State(s): State<WebState>) -> Json<Vec<NpcRow>> {
    let npcs = match &s.data {
        Some(data) => NpcRow::from_data(data),
        None => Vec::new(),
    };
    Json(npcs)
}

async fn list_online(State(s): State<WebState>) -> Json<Vec<OnlineEntry>> {
    let online = s.app.read().await.online.clone();
    Json(online)
}

async fn log_stream(State(s): State<WebState>) -> Response {
    let rx = s.app.read().await.broadcast.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let payload = serde_json::to_string(&ev).unwrap_or_default();
                    let chunk = axum::response::sse::Event::default()
                        .event("log")
                        .data(payload);
                    return Some((Ok::<_, std::convert::Infallible>(chunk), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    axum::response::sse::Sse::new(stream).into_response()
}

async fn get_perexp(State(s): State<WebState>) -> Json<serde_json::Value> {
    let perexp = s.app.read().await.perexp;
    Json(json!({ "perexp": perexp }))
}

#[derive(Debug, serde::Deserialize)]
pub struct SetPerexpPayload {
    pub value: u32,
}

async fn set_perexp(
    State(s): State<WebState>,
    headers: HeaderMap,
    payload: Result<FormOrJson<SetPerexpPayload>, StatusCode>,
) -> Response {
    let v = match payload {
        Ok(FormOrJson::Json(p)) | Ok(FormOrJson::Form(p)) => p.value,
        Err(_) => 0,
    };

    let mut app = s.app.write().await;
    app.perexp = v;

    htmx_or_json(&headers, || format!("{v}"), json!({ "perexp": v }))
}