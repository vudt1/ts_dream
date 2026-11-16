//! SQLite pool bootstrap with dual-pool architecture (Read vs Write).
//!
//! Dual connections:
//! - `read`: concurrent read pool (`max_connections = 16`).
//! - `write`: dedicated single write connection (`max_connections = 1`) to eliminate SQLITE_BUSY.
//! - WAL mode (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`).
//! - Scheduled WAL checkpointing (`PRAGMA wal_checkpoint(TRUNCATE);`).

use crate::error::{Result, TsError};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub const READ_MAX_CONNECTIONS: u32 = 16;
pub const WRITE_MAX_CONNECTIONS: u32 = 1;
pub const DEFAULT_CACHE_SIZE_KB: i64 = 64000; // 64 MB per connection

/// Dual-pool holder for SQLite.
#[derive(Clone, Debug)]
pub struct DbPool {
    /// Read pool supporting multiple concurrent sessions.
    pub read: SqlitePool,
    /// Dedicated single-session write pool serializing write operations.
    pub write: SqlitePool,
}

impl DbPool {
    /// Flushes all WAL frames into the main database file and truncates the WAL file.
    pub async fn checkpoint(&self) -> Result<()> {
        wal_checkpoint(&self.write).await
    }
}

/// Build both read and write pools for SQLite with WAL mode enabled.
pub async fn bootstrap(database_url: &str, cache_size_kb: Option<i64>) -> Result<DbPool> {
    ensure_parent_dir_exists(database_url)?;

    let cache_kb = cache_size_kb.unwrap_or(DEFAULT_CACHE_SIZE_KB);
    let cache_arg = format!("-{}", cache_kb.abs());

    let opts = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| TsError::Config(format!("invalid sqlite database_url: {e}")))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(5000))
        .pragma("cache_size", cache_arg)
        .pragma("temp_store", "MEMORY");

    // Connect write pool first (max 1 connection)
    let write = SqlitePoolOptions::new()
        .max_connections(WRITE_MAX_CONNECTIONS)
        .connect_with(opts.clone())
        .await
        .map_err(TsError::Db)?;

    // Ensure WAL pragma is active on the database
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&write)
        .await
        .map_err(TsError::Db)?;

    // Connect read pool (up to READ_MAX_CONNECTIONS)
    let read = SqlitePoolOptions::new()
        .max_connections(READ_MAX_CONNECTIONS)
        .connect_with(opts)
        .await
        .map_err(TsError::Db)?;

    tracing::info!(
        "connected to SQLite at {}; WAL mode active (read_pool={}, write_pool={})",
        database_url,
        READ_MAX_CONNECTIONS,
        WRITE_MAX_CONNECTIONS
    );

    Ok(DbPool { read, write })
}

/// Helper to ensure the parent folder (e.g. `DB/`) exists before SQLite attempts to create the file.
fn ensure_parent_dir_exists(database_url: &str) -> Result<()> {
    let clean_path = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))
        .unwrap_or(database_url);

    let file_path = clean_path.split('?').next().unwrap_or(clean_path);
    if file_path != ":memory:" {
        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
                tracing::info!("created SQLite directory: {}", parent.display());
            }
        }
    }
    Ok(())
}

/// Run a WAL checkpoint to flush all uncommitted WAL frames back into the main `.db` file.
/// Mode `TRUNCATE` will checkpoint all frames and truncate the WAL file to zero bytes.
pub async fn wal_checkpoint(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
        .execute(pool)
        .await
        .map_err(TsError::Db)?;
    tracing::debug!("SQLite WAL checkpoint (TRUNCATE) completed");
    Ok(())
}

/// Spawn the periodic background WAL checkpoint task.
pub fn spawn_wal_checkpoint_task(pool: SqlitePool, interval: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await; // skip initial tick
        loop {
            ticker.tick().await;
            if let Err(e) = wal_checkpoint(&pool).await {
                tracing::warn!("periodic WAL checkpoint error: {e}");
            }
        }
    })
}

/// Placeholder migration runner (schema migrations currently out-of-scope per ADR 0004).
pub async fn migrate(_pool: &SqlitePool) -> Result<()> {
    tracing::info!("SQLite migrations are skipped (managed externally per ADR 0004)");
    Ok(())
}

/// Startup schema sanity-check verifying core tables from `0001_init.sql` and
/// normalized columns/keys from `0002_eve_quest_persistence.sql`.
///
/// Migrations remain manual per ADR 0004; if the database is empty (`0` tables)
/// or missing migration `0002`, this returns a clear `TsError::Migrate` with
/// instructions instead of silently failing later on player login/save.
pub async fn verify_schema(pool: &SqlitePool) -> Result<()> {
    use sqlx::Row;
    use std::collections::HashSet;

    const ADR_0004_HINT: &str = "Per ADR 0004, migrations are not applied automatically at startup. \
        Apply migrations/0001_init.sql and migrations/0002_eve_quest_persistence.sql manually \
        (e.g. `sqlite3 DB/ts_dream.db < migrations/0001_init.sql && sqlite3 DB/ts_dream.db < migrations/0002_eve_quest_persistence.sql`).";

    const REQUIRED_TABLES: &[&str] = &[
        "accounts",
        "characters",
        "character_money",
        "inventories",
        "character_pets",
        "character_skills",
        "character_quest_tasks",
        "character_quest_dont",
        "character_bit_flags",
        "character_completed_events",
    ];

    let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'")
        .fetch_all(pool)
        .await
        .map_err(TsError::Db)?;

    let existing_tables: HashSet<String> = rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("name").ok())
        .collect();

    let missing_tables: Vec<&str> = REQUIRED_TABLES
        .iter()
        .copied()
        .filter(|t| !existing_tables.contains(*t))
        .collect();

    if !missing_tables.is_empty() {
        return Err(TsError::Migrate(format!(
            "SQLite schema sanity-check failed: database has {} table(s) and is missing required table(s): [{}]. {}",
            existing_tables.len(),
            missing_tables.join(", "),
            ADR_0004_HINT
        )));
    }

    let char_cols_rows = sqlx::query("PRAGMA table_info(characters)")
        .fetch_all(pool)
        .await
        .map_err(TsError::Db)?;
    let char_cols: HashSet<String> = char_cols_rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("name").ok())
        .collect();
    let missing_savemap: Vec<&str> = ["savemap_id", "savemap_x", "savemap_y"]
        .into_iter()
        .filter(|c| !char_cols.contains(*c))
        .collect();
    if !missing_savemap.is_empty() {
        return Err(TsError::Migrate(format!(
            "SQLite schema sanity-check failed: `characters` is missing column(s) [{}] from migrations/0002_eve_quest_persistence.sql. {}",
            missing_savemap.join(", "),
            ADR_0004_HINT
        )));
    }

    let completed_rows = sqlx::query("PRAGMA table_info(character_completed_events)")
        .fetch_all(pool)
        .await
        .map_err(TsError::Db)?;
    let mut pk_cols: Vec<(i64, String)> = Vec::new();
    let mut has_scene_id = false;
    for row in completed_rows {
        let name: String = row.try_get("name").map_err(TsError::Db)?;
        let pk: i64 = row.try_get("pk").map_err(TsError::Db)?;
        if name == "scene_id" {
            has_scene_id = true;
        }
        if pk > 0 {
            pk_cols.push((pk, name));
        }
    }
    pk_cols.sort_by_key(|(pk, _)| *pk);
    let pk_names: Vec<&str> = pk_cols.iter().map(|(_, n)| n.as_str()).collect();
    if !has_scene_id || pk_names.as_slice() != ["playerid", "scene_id", "eventid"] {
        return Err(TsError::Migrate(format!(
            "SQLite schema sanity-check failed: `character_completed_events` must have `scene_id` and PRIMARY KEY (playerid, scene_id, eventid) from migrations/0002_eve_quest_persistence.sql (found PK {:?}). {}",
            pk_names,
            ADR_0004_HINT
        )));
    }

    Ok(())
}
