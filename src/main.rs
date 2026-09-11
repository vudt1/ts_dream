//! TS Dream — binary entrypoint (Chapter 1 §1.3 startup sequence).

use std::sync::Arc;
use tokio::sync::RwLock;
use ts_dream::config::Config;
use ts_dream::data::loader::GameData;
use ts_dream::state::AppState;
use ts_dream::web::app::WebState;
use ts_dream::web::server_control::ServerControl;

/// Register every spawned static drop (ItemOnMap.txt) into the server-global
/// `map_drops` registry. The boot-time broadcast fires with no clients
/// connected — a no-op we do not repeat.
fn seed_static_drops(data: &GameData) {
    let spawned: Vec<_> = data
        .item_drop_on_map
        .values()
        .filter(|d| d.item_id != 0)
        .collect();
    for drop in &spawned {
        let item = ts_dream::server::session::InventoryItem {
            slot: drop.slot as u8,
            id: drop.item_id as u16,
            count: 1,
            lv: drop.lv as u8,
            int1: drop.int1 as i16,
            atk1: drop.atk1 as i16,
            def1: drop.def1 as i16,
            hpx1: drop.hpx1 as i16,
            spx1: drop.spx1 as i16,
            agi1: drop.agi1 as i16,
            fai1: drop.fai1 as i16,
            int2: drop.int2 as i16,
            atk2: drop.atk2 as i16,
            def2: drop.def2 as i16,
            hpx2: drop.hpx2 as i16,
            spx2: drop.spx2 as i16,
            agi2: drop.agi2 as i16,
            fai2: drop.fai2 as i16,
            loai: drop.loai as u8,
            thuoctinh: drop.thuoctinh as u8,
            giatri_thuoctinh: drop.giatri_thuoctinh as u8,
            long_val: drop.long_val as u8,
            giatri_long: drop.giatri_long as u8,
            ..Default::default()
        };
        ts_dream::server::map_drops::drop(
            drop.map_id as u16,
            drop.slot as u8,
            item,
            drop.map_x as u16,
            drop.map_y as u16,
        );
    }
    tracing::info!(
        "seeded {} static map drops into the drop registry",
        spawned.len()
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 1. Load config.
    let cfg = Config::load().map_err(|e| anyhow::anyhow!("config load failed: {e}"))?;
    tracing::info!(
        "config loaded: game={} web={} data_dir={}",
        cfg.game_port,
        cfg.web_port,
        cfg.data_dir.display()
    );

    // 2. Connect SQLite dual-pool (fail-fast)
    let db_url = cfg.resolve_database_url();
    let pool = ts_dream::db::pool::bootstrap(&db_url, Some(cfg.sqlite_cache_size_kb)).await?;
    tracing::info!("connected to SQLite ({db_url})");

    // 3. Shared AppState
    let mut app_state_inner = AppState::new(cfg.perexp_default);
    app_state_inner.db_status = ts_dream::state::DbStatus::Connected;
    let app_state = Arc::new(RwLock::new(app_state_inner));

    // 3b. Periodic WAL checkpoint background task (flushes WAL to main db every N secs, default 300s)
    let wal_interval = std::time::Duration::from_secs(cfg.wal_checkpoint_interval_secs);
    ts_dream::db::pool::spawn_wal_checkpoint_task(pool.write.clone(), wal_interval);
    tracing::info!(
        "scheduled periodic WAL checkpoint task every {}s",
        cfg.wal_checkpoint_interval_secs
    );

    // 3c. AutoSaveService (ticket 07): every 3 minutes, fingerprint the online
    //     sessions and batch-write dirty ones in one transaction each.
    ts_dream::server::auto_save::spawn(pool.clone());

    // 3d. Eve-event activation gate (ticket 07): off by default so wire parity
    //     with the golden captures holds; operators opt in via `TS_EVE_EVENTS=1`.
    if std::env::var("TS_EVE_EVENTS").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")) {
        ts_dream::server::handlers::npc_event::set_eve_events_enabled(true);
        tracing::info!("Eve event engine enabled (TS_EVE_EVENTS)");
    }

    // 4. Load static data (DataLoaded gate). `resolve_data_dir` prefers the
    //    configured path (repo `./Data/`), then the exe-adjacent build.rs copy.
    let data_dir = cfg.resolve_data_dir();
    let data = match GameData::load(&data_dir) {
        Ok(d) => Arc::new(d),
        Err(e) => {
            tracing::warn!("static data loading failed: {e}; running with empty GameData");
            Arc::new(GameData::default())
        }
    };
    tracing::info!(
        "data loaded: {} npcs, {} items, {} skills, {} talks (data_dir={})",
        data.npcs.len(),
        data.items.len(),
        data.skills.len(),
        data.talks.len(),
        data_dir.display()
    );
    if data.is_loaded() {
        let assets = data.binary_asset_metadata();
        if let Err(e) = ts_dream::db::catalog::replace_asset_catalog(&pool, &assets).await {
            tracing::warn!("failed to persist binary asset catalog: {e}");
        } else {
            tracing::info!("persisted {} binary asset provenance rows", assets.len());
        }
    }

    // Set DataLoaded flag in AppState
    app_state.write().await.data_loaded = data.is_loaded();

    // Seed the runtime drop registry from the loaded static ItemOnMap drops so
    // they are pickable on map load.
    seed_static_drops(&data);

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
        match tokio::net::TcpListener::bind(web_addr).await {
            Ok(listener) => {
                tracing::info!("web dashboard running on {web_addr}");
                if let Err(e) = axum::serve(listener, web_router).await {
                    tracing::error!("web dashboard serve error on {web_addr}: {e}");
                }
            }
            Err(e) => {
                tracing::error!("web dashboard failed to bind {web_addr}: {e}");
            }
        }
    });

    // 7. Start initial Game TCP Server listener
    if let Err(e) = server_control.start().await {
        tracing::error!("initial game server start failed: {e}");
    }

    // 8. Graceful shutdown: listen for Ctrl+C / SIGINT signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("received shutdown signal (Ctrl+C); initiating graceful exit...");

    // Stop TCP server (broadcasts 020C, kicks clients, auto-saves & WAL checkpoints)
    if let Err((code, msg)) = server_control.stop().await {
        tracing::warn!("server control stop returned {code}: {msg}");
    }

    // Extra safety: final auto-save pass and WAL flush
    let saved = ts_dream::server::auto_save::save_all_dirty(&pool).await;
    if saved > 0 {
        tracing::info!("graceful shutdown: saved {saved} dirty session(s)");
    }
    if let Err(e) = pool.checkpoint().await {
        tracing::error!("graceful shutdown: final WAL checkpoint error: {e}");
    } else {
        tracing::info!("graceful shutdown: final WAL checkpoint (TRUNCATE) completed successfully");
    }

    tracing::info!("TS Dream server shutdown complete.");
    Ok(())
}

