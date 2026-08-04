//! TS Dream — binary entrypoint (Chapter 1 §1.3 startup sequence).

use ts_dream::config::Config;
use ts_dream::data::loader::GameData;

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

    // 3. Spawn web server (always up once DB is reachable).
    let app_state = std::sync::Arc::new(tokio::sync::RwLock::new(
        ts_dream::state::AppState::new(cfg.perexp_default),
    ));
    let web_state = ts_dream::web::app::WebState {
        app: app_state.clone(),
        pool: Some(pool),
    };
    let web_router = ts_dream::web::app::router(web_state);
    let web_addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.web_port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(web_addr).await;
        if let Ok(listener) = listener {
            let _ = axum::serve(listener, web_router).await;
        }
    });
    tracing::info!("web dashboard on {}", cfg.web_port);

    // 4. Load static data (DataLoaded gate).
    let data = std::sync::Arc::new(GameData::load(&cfg.data_dir)?);
    tracing::info!(
        "data loaded: {} npcs, {} items, {} skills, {} talks",
        data.npcs.len(),
        data.items.len(),
        data.skills.len(),
        data.talks.len()
    );

    // 5. Game TCP accept loop (gated on DataLoaded).
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.game_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("game server listening on {}", cfg.game_port);
    // Keep the process alive, accepting connections is wired in a follow-up
    // ticket; for now loop forever so the web dashboard stays up.
    loop {
        let (stream, peer) = listener.accept().await?;
        let app = app_state.clone();
        let data = data.clone();
        tokio::spawn(async move {
            let _ = (stream, peer, app, data);
        });
    }
}