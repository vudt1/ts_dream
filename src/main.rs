//! TS Dream — binary entrypoint (Chapter 1 §1.3 startup sequence).

use std::sync::Arc;
use tokio::sync::RwLock;
use ts_dream::config::Config;
use ts_dream::data::loader::GameData;
use ts_dream::state::AppState;
use ts_dream::web::app::WebState;
use ts_dream::web::server_control::ServerControl;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
    )
    .init();

    // 1. Load config.
    let cfg = Config::load()
        .map_err(|e| anyhow::anyhow!("config load failed: {e}"))?;
    tracing::info!(
        "config loaded: game={} web={} data_dir={}",
        cfg.game_port,
        cfg.web_port,
        cfg.data_dir.display()
    );

    // 2. Connect MySQL pool (fail-fast).
    let pool = ts_dream::db::pool::connect(&cfg.database_url).await?;
    tracing::info!("connected to MySQL; running migrations");
    ts_dream::db::pool::migrate(&pool).await?;

    // 3. Shared AppState
    let app_state = Arc::new(RwLock::new(AppState::new(cfg.perexp_default)));

    // 4. Load static data (DataLoaded gate).
    let data = match GameData::load(&cfg.data_dir) {
        Ok(d) => Arc::new(d),
        Err(e) => {
            tracing::warn!("static data loading failed: {e}; running with empty GameData");
            Arc::new(GameData::default())
        }
    };
    tracing::info!(
        "data loaded: {} npcs, {} items, {} skills, {} talks",
        data.npcs.len(),
        data.items.len(),
        data.skills.len(),
        data.talks.len()
    );

    // Set DataLoaded flag in AppState
    app_state.write().await.data_loaded = data.is_loaded();

    // 5. ServerControl handle
    let server_control = Arc::new(ServerControl::new(
        cfg.game_port,
        app_state.clone(),
        Some(data.clone()),
        Some(pool.clone()),
    ));

    // 6. Spawn web server (always up once DB is reachable).
    let web_state = WebState {
        app: app_state.clone(),
        pool: Some(pool.clone()),
        data: Some(data.clone()),
        server_control: Some(server_control.clone()),
    };
    let web_router = ts_dream::web::app::router(web_state);
    let web_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.web_port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(web_addr).await;
        if let Ok(listener) = listener {
            let _ = axum::serve(listener, web_router).await;
        }
    });
    tracing::info!("web dashboard running on http://0.0.0.0:{}", cfg.web_port);

    // 7. Start initial Game TCP Server listener
    if let Err(e) = server_control.start().await {
        tracing::error!("initial game server start failed: {e}");
    }

    // Keep process alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}