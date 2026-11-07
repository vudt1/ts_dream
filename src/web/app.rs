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
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::data::loader::GameData;
use crate::db;
use crate::db::accounts::AccountRow;
use crate::db::pool::DbPool;
use crate::state::{AppState, LogEvent, OnlineEntry};
use crate::web::server_control::ServerControl;

pub type SharedState = Arc<RwLock<AppState>>;

pub const HTMX_JS: &str = include_str!("static/htmx.min.js");

/// Turnstile for dashboard handlers needing the DB, GameData, and ServerControl.
#[derive(Clone)]
pub struct WebState {
    pub app: SharedState,
    pub pool: Option<DbPool>,
    pub data: Option<Arc<GameData>>,
    pub server_control: Option<Arc<ServerControl>>,
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
                name: crate::encoding::viscii_decode(&n.name),
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
    pub assets: Vec<crate::data::loader::BinaryAssetMeta>,
    pub guilds: Vec<db::domain::GuildRow>,
    pub world_bosses: Vec<db::domain::WorldBossRow>,
    pub initial_logs: Vec<LogEvent>,
    pub events: Vec<crate::server::events::SpecialEvent>,
    pub north_star_rate: u8,
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
        .route(
            "/api/server/announce",
            get(server_announce).post(server_announce),
        )
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route("/api/guilds", get(list_guilds))
        .route("/api/world-bosses", get(list_world_bosses))
        .route("/api/npcs", get(list_npcs))
        .route("/api/data/assets", get(list_data_assets))
        .route("/api/online", get(list_online))
        .route("/api/log/stream", get(log_stream))
        .route("/api/config/perexp", get(get_perexp).post(set_perexp))
        .route("/api/db/status", get(db_status))
        .route("/api/db/status/stream", get(db_status_stream))
        .route("/api/events", get(list_special_events))
        .route("/api/events/{id}/start", axum::routing::post(start_special_event))
        .route("/api/events/{id}/stop", axum::routing::post(stop_special_event))
        .route("/api/events/north_star/rate", axum::routing::post(set_north_star_encounter_rate))
        .with_state(s)
}

async fn static_htmx() -> Response {
    ([(header::CONTENT_TYPE, "application/javascript")], HTMX_JS).into_response()
}

async fn index(State(s): State<WebState>) -> Response {
    let app_guard = s.app.read().await;
    let running = app_guard.running;
    let perexp = app_guard.perexp;
    let online_list = current_online_entries(&app_guard.online);
    let initial_logs: Vec<LogEvent> = app_guard.log_buffer.iter().cloned().collect();
    drop(app_guard);

    let accounts = match &s.pool {
        Some(pool) => db::accounts::list(pool).await.unwrap_or_default(),
        None => Vec::new(),
    };

    let mut npcs = Vec::new();
    let mut assets = Vec::new();
    if let Some(ref data) = s.data {
        npcs = NpcRow::from_data(data);
        assets = data.binary_asset_metadata();
    }

    let guilds = match &s.pool {
        Some(pool) => db::domain::list_guilds(pool).await.unwrap_or_default(),
        None => Vec::new(),
    };
    let world_bosses = match &s.pool {
        Some(pool) => db::domain::list_world_bosses(pool)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let (events, north_star_rate) = match &s.server_control {
        Some(control) => (control.events.list_events(), control.events.get_north_star_rate()),
        None => (Vec::new(), 5),
    };

    let template = DashboardTemplate {
        running,
        perexp,
        online_count: online_list.len(),
        npcs_count: npcs.len(),
        online_list,
        accounts,
        npcs,
        assets,
        guilds,
        world_bosses,
        initial_logs,
        events,
        north_star_rate,
    };

    template.into_response()
}

async fn server_status(State(s): State<WebState>) -> Json<serde_json::Value> {
    Json(json!({ "running": s.app.read().await.running }))
}

async fn server_start(State(s): State<WebState>, headers: HeaderMap) -> Response {
    if let Some(ref control) = s.server_control {
        let _ = control.start().await;
    } else {
        s.app.write().await.running = true;
    }

    htmx_or_json(
        &headers,
        || {
            String::from(
                r##"
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
        "##,
            )
        },
        json!({ "running": true }),
    )
}

async fn server_stop(State(s): State<WebState>, headers: HeaderMap) -> Response {
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
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "server not running" })),
            )
                .into_response();
        }
        app.running = false;
    }

    htmx_or_json(
        &headers,
        || {
            String::from(
                r##"
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
        "##,
            )
        },
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
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "server not running" })),
            )
                .into_response();
        }
        Json(json!({ "ok": true })).into_response()
    }
}

async fn list_accounts(State(s): State<WebState>) -> Result<Json<Vec<AccountRow>>, StatusCode> {
    match s.pool {
        Some(ref pool) => db::accounts::list(pool)
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
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

    if !db::accounts::is_valid_password(payload.pass1.as_bytes())
        || !db::accounts::is_valid_password(payload.pass2.as_bytes())
    {
        let msg = "Mat ma phai 8-10 ky tu ASCII (khong dau cach / ky tu dieu khien).";
        return if headers.get("HX-Request").is_some() {
            (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    "<tr><td colspan=\"3\" style=\"color: var(--accent-red)\">{msg}</td></tr>"
                )),
            )
                .into_response()
        } else {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
        };
    }

    let player_id = match db::accounts::create(pool, &payload.pass1, &payload.pass2).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    htmx_or_json(
        &headers,
        || {
            format!(
                "<tr><td><strong>{}</strong></td><td>0</td><td>no</td></tr>",
                player_id
            )
        },
        json!({
            "player_id": player_id,
            "gm_level": 0,
            "is_suspended": false
        }),
    )
}

async fn list_guilds(
    State(s): State<WebState>,
) -> Result<Json<Vec<db::domain::GuildRow>>, StatusCode> {
    match s.pool {
        Some(ref pool) => db::domain::list_guilds(pool)
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn list_world_bosses(
    State(s): State<WebState>,
) -> Result<Json<Vec<db::domain::WorldBossRow>>, StatusCode> {
    match s.pool {
        Some(ref pool) => db::domain::list_world_bosses(pool)
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn list_data_assets(
    State(s): State<WebState>,
) -> Json<Vec<crate::data::loader::BinaryAssetMeta>> {
    let assets = s
        .data
        .as_ref()
        .map(|data| data.binary_asset_metadata())
        .unwrap_or_default();
    Json(assets)
}

async fn list_npcs(State(s): State<WebState>) -> Json<Vec<NpcRow>> {
    let npcs = match &s.data {
        Some(data) => NpcRow::from_data(data),
        None => Vec::new(),
    };
    Json(npcs)
}

fn current_online_entries(fallback: &[OnlineEntry]) -> Vec<OnlineEntry> {
    let sessions = crate::server::session::lock_online_sessions();
    let dynamic: Vec<OnlineEntry> = sessions
        .values()
        .filter(|sess| sess.authed && sess.id > 0)
        .map(|sess| OnlineEntry {
            id: sess.id,
            name: crate::encoding::viscii_decode(&sess.name),
            ip: String::new(),
        })
        .collect();
    if dynamic.is_empty() {
        fallback.to_vec()
    } else {
        dynamic
    }
}

async fn list_online(State(s): State<WebState>) -> Json<Vec<OnlineEntry>> {
    let fallback = s.app.read().await.online.clone();
    Json(current_online_entries(&fallback))
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

/// Current MySQL connectivity state (Ch7 ticket #22).
async fn db_status(State(s): State<WebState>) -> Json<serde_json::Value> {
    let st = s.app.read().await.db_status.as_str().to_string();
    Json(json!({ "state": st }))
}

/// SSE stream of `dbstatus` events; fires whenever the liveness probe flips
/// the MySQL connection state (Ch7 ticket #22).
async fn db_status_stream(State(s): State<WebState>) -> Response {
    let rx = s.app.read().await.db_status_tx.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(status) => {
                    let payload = json!({ "state": status.as_str() }).to_string();
                    let chunk = axum::response::sse::Event::default()
                        .event("dbstatus")
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

async fn list_special_events(State(s): State<WebState>) -> Json<Vec<crate::server::events::SpecialEvent>> {
    let events = if let Some(ref control) = s.server_control {
        control.events.list_events()
    } else {
        Vec::new()
    };
    Json(events)
}

async fn start_special_event(
    State(s): State<WebState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(ref control) = s.server_control else {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Game server is stopped. Please start the server first" })),
        )
            .into_response();
    };

    let is_running = control.is_running().await;
    match control.events.start_event(&id, is_running) {
        Ok(()) => {
            if let Some(ref pool) = s.pool {
                let _ = crate::db::domain::write_audit(
                    pool,
                    Some(1),
                    "event_start",
                    Some("special_event"),
                    None,
                    Some(&id),
                )
                .await;
            }
            htmx_or_json(
                &headers,
                || render_events_card(&control.events),
                json!({ "ok": true, "event": id, "status": "Active" }),
            )
        }
        Err((code, msg)) => (code, Json(json!({ "error": msg }))).into_response(),
    }
}

async fn stop_special_event(
    State(s): State<WebState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(ref control) = s.server_control else {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Game server is stopped. Please start the server first" })),
        )
            .into_response();
    };

    match control.events.stop_event(&id) {
        Ok(()) => {
            if let Some(ref pool) = s.pool {
                let _ = crate::db::domain::write_audit(
                    pool,
                    Some(1),
                    "event_stop",
                    Some("special_event"),
                    None,
                    Some(&id),
                )
                .await;
            }
            htmx_or_json(
                &headers,
                || render_events_card(&control.events),
                json!({ "ok": true, "event": id, "status": "Ending" }),
            )
        }
        Err((code, msg)) => (code, Json(json!({ "error": msg }))).into_response(),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RatePayload {
    pub rate: Option<u8>,
}

async fn set_north_star_encounter_rate(
    State(s): State<WebState>,
    payload: Result<FormOrJson<RatePayload>, StatusCode>,
) -> Response {
    let rate = match payload {
        Ok(FormOrJson::Json(p)) | Ok(FormOrJson::Form(p)) => p.rate.unwrap_or(5),
        Err(_) => 5,
    };

    if let Some(ref control) = s.server_control {
        control.events.set_north_star_rate(rate);
        if let Some(ref pool) = s.pool {
            let details = format!("rate={}", control.events.get_north_star_rate());
            let _ = crate::db::domain::write_audit(
                pool,
                Some(1),
                "event_rate_change",
                Some("special_event"),
                None,
                Some(&details),
            )
            .await;
        }
        Json(json!({ "ok": true, "rate": control.events.get_north_star_rate() })).into_response()
    } else {
        (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Server control unavailable" })),
        )
            .into_response()
    }
}

pub fn render_events_card(mgr: &crate::server::events::EventLifecycleManager) -> String {
    let events = mgr.list_events();
    let rate = mgr.get_north_star_rate();
    let mut html = String::from(
        r##"<div class="card" id="special-events-card" style="margin-bottom: 24px;">
        <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
            <span>Special Events Control Plane</span>
            <span style="font-size: 12px; color: var(--text-muted);">Strict Mutex & 120m Starvation Watchdog</span>
        </div>
        <div style="margin-bottom: 16px; padding: 12px; background: rgba(0,0,0,0.2); border-radius: 8px; display: flex; align-items: center; gap: 16px;">
            <label style="font-size: 13px; font-weight: 600; color: var(--text-secondary);">North Star Encounter Rate: <span id="rate-display">"##,
    );
    html.push_str(&format!("{rate}%"));
    html.push_str(
        r##"</span></label>
            <input type="range" name="rate" min="1" max="30" value=""##,
    );
    html.push_str(&format!("{rate}"));
    html.push_str(
        r##"" style="flex: 1;" hx-post="/api/events/north_star/rate" hx-trigger="change" hx-swap="none" oninput="document.getElementById('rate-display').innerText = this.value + '%'">
        </div>
        <table class="data-table" style="width: 100%;">
            <thead>
                <tr>
                    <th>Event ID</th>
                    <th>Name</th>
                    <th>Status</th>
                    <th>Participants</th>
                    <th>Action</th>
                </tr>
            </thead>
            <tbody>"##,
    );
    for e in events {
        let badge_class = match e.phase {
            crate::server::events::EventPhase::Active => "status-online",
            crate::server::events::EventPhase::Ending => "status-connecting",
            _ => "status-offline",
        };
        html.push_str(&format!(
            r##"<tr>
                <td style="font-weight: 600;">{}</td>
                <td>{}</td>
                <td><span class="status-badge {}">{}</span></td>
                <td>{}</td>
                <td>
                    <button class="btn btn-green" style="padding: 4px 10px; font-size: 12px;" hx-post="/api/events/{}/start" hx-target="#special-events-card" hx-swap="outerHTML">Start</button>
                    <button class="btn btn-red" style="padding: 4px 10px; font-size: 12px; margin-left: 6px;" hx-post="/api/events/{}/stop" hx-target="#special-events-card" hx-swap="outerHTML">Stop</button>
                </td>
            </tr>"##,
            e.id, e.name, badge_class, e.phase, e.participants, e.id, e.id
        ));
    }
    html.push_str(r##"</tbody></table></div>"##);
    html
}

