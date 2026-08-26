//! MySQL 8 pool bootstrap (Chapter 5).
//!
//! Connect with `MySqlPool`, set connection `charset = latin1` so stored
//! VISCII byte names are never transcoded. Fail-fast on boot.

use crate::error::{Result, TsError};
use crate::state::{AppState, DbStatus};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::MySqlPool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const MAX_CONNECTIONS: u32 = 10;

/// Build a pool from a `mysql://` URL, forcing connection charset latin1.
pub async fn connect(database_url: &str) -> Result<MySqlPool> {
    let opts = server_options(database_url)?; // carries the URL's database path
    pool_options(opts).await
}

/// Build a pool, auto-creating the target database first when it is missing.
///
/// When `auto_create` is set (default, `TS_DB_AUTO_CREATE`), connects to the
/// MySQL server without a schema, issues
/// `CREATE DATABASE IF NOT EXISTS <db> CHARACTER SET latin1 COLLATE latin1_bin`,
/// then returns a pool bound to `<db>`. Databases are created `latin1` so the
/// explicit table charsets in the migration match the DB default; an operator
/// provisioning the DB themselves can pass `auto_create = false` (spec §8.3).
pub async fn bootstrap(database_url: &str, auto_create: bool) -> Result<MySqlPool> {
    if auto_create {
        let full = server_options(database_url)?;
        let dbname = full.get_database().map(str::to_owned);
        if let Some(dbname) = &dbname {
            let server_opts = server_options(&strip_database_path(database_url))?;
            let server_pool = pool_options(server_opts).await?;
            // The name comes from the trusted config URL; backtick + double any
            // embedded backticks defensively. It is never client input.
            let escaped = dbname.replace('`', "``");
            let create = format!(
                "CREATE DATABASE IF NOT EXISTS `{escaped}` \
                 CHARACTER SET latin1 COLLATE latin1_bin"
            );
            sqlx::query(&create)
                .execute(&server_pool)
                .await
                .map_err(TsError::Db)?;
            tracing::info!("ensured database `{dbname}` exists (latin1)");
        } else {
            tracing::info!(
                "db_auto_create enabled but database_url has no database name; skipping"
            );
        }
    }
    connect(database_url).await
}

/// Return the connection URL with any trailing `/database` path and query
/// string removed (e.g. `mysql://u:p@h:3306/ts_dream` -> `mysql://u:p@h:3306`),
/// so a connection can be opened to the server itself rather than a schema.
pub fn strip_database_path(database_url: &str) -> String {
    let mut end = database_url.len();
    if let Some(q) = database_url.find('?') {
        end = end.min(q);
    }
    // The authority is everything between the scheme separator `://` and the
    // first `/` (which begins the path). Keep the scheme + authority only.
    if let Some(sep) = database_url.find("://") {
        let auth_start = sep + 3;
        if let Some(slash) = database_url[auth_start..end].find('/') {
            end = end.min(auth_start + slash);
        }
    }
    database_url[..end].to_string()
}

async fn pool_options(opts: MySqlConnectOptions) -> Result<MySqlPool> {
    MySqlPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(opts)
        .await
        .map_err(TsError::Db)
}

/// Parse + apply the latin1 connection charset for the given URL.
fn server_options(database_url: &str) -> Result<MySqlConnectOptions> {
    let mut opts = MySqlConnectOptions::from_str(database_url)
        .map_err(|e| TsError::Config(format!("invalid database_url: {e}")))?;
    opts = opts.charset("latin1");
    Ok(opts)
}

/// Apply embedded migrations.
pub async fn migrate(pool: &MySqlPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| TsError::Migrate(e.to_string()))
}

/// Spawn the background MySQL liveness probe (Ch7 ticket #22).
///
/// Every `interval` the task runs `SELECT 1`; on success it sets
/// `DbStatus::Connected`, on failure `DbStatus::Disconnected`, broadcasting
/// each change over `AppState.db_status_tx`. Boot stays fail-fast (spec
/// §1.1): this only detects *runtime* DB loss, so `Disconnected` is reachable
/// only after the dashboard is already up (DB died post-boot). `Connecting`
/// shows at startup until the first probe resolves.
/// Pure decision for the liveness probe: a successful ping yields
/// `Connected`, a failed ping yields `Disconnected`.
pub fn probe_next_state(_prev: DbStatus, ping_ok: bool) -> DbStatus {
    if ping_ok {
        DbStatus::Connected
    } else {
        DbStatus::Disconnected
    }
}

pub fn spawn_liveness_probe(pool: MySqlPool, app: Arc<RwLock<AppState>>, interval: Duration) {
    tokio::spawn(async move {
        loop {
            let ok = sqlx::query("SELECT 1").execute(&pool).await.is_ok();
            let next = probe_next_state(app.read().await.db_status, ok);
            let mut app = app.write().await;
            if app.db_status != next {
                app.db_status = next;
                let _ = app.db_status_tx.send(next);
            }
            drop(app);
            sleep(interval).await;
        }
    });
}
