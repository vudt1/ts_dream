//! Official test database bootstrap (`CODING_STANDARDS.md` §3.4).
//!
//! Every DB-backed test runs against the file `DB/ts_dream_test.db` (override
//! with `TS_TEST_DB_URL`), using the same WAL + dual-pool bootstrap as
//! production. Because all tests share one file, [`setup_test_db`] takes a
//! process-wide lock and resets the schema; the returned [`TestDb`] holds that
//! lock until it is dropped, so DB tests inside one test binary run one at a
//! time while non-DB tests stay parallel.

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use tokio::sync::{Mutex, OwnedMutexGuard};
use ts_dream::db::pool::{bootstrap, DbPool};

/// Environment variable overriding the test database URL.
pub const TEST_DB_URL_ENV: &str = "TS_TEST_DB_URL";

const SCHEMA: &str = include_str!("../../migrations/0001_init.sql");

fn db_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

/// Resolve the test DB URL: `TS_TEST_DB_URL`, else `<repo>/DB/ts_dream_test.db`.
pub fn test_db_url() -> String {
    let url = std::env::var(TEST_DB_URL_ENV).unwrap_or_else(|_| {
        format!(
            "sqlite://{}/DB/ts_dream_test.db",
            env!("CARGO_MANIFEST_DIR").replace('\\', "/")
        )
    });
    let path = url.split('?').next().unwrap_or(&url);
    assert!(
        !path.ends_with("/ts_dream.db") && !path.ends_with(":ts_dream.db"),
        "{TEST_DB_URL_ENV} must not point at the production DB/ts_dream.db (got `{url}`)"
    );
    url
}

/// A freshly reset test database plus the lock that keeps it exclusive.
///
/// Derefs to [`DbPool`], so `&db`, `db.write`, `db.read` and `db.clone()`
/// (which yields a `DbPool`) work as with a plain pool.
pub struct TestDb {
    pool: DbPool,
    _guard: OwnedMutexGuard<()>,
}

impl Deref for TestDb {
    type Target = DbPool;

    fn deref(&self) -> &DbPool {
        &self.pool
    }
}

/// Lock the shared test DB, drop every schema object, and re-run `0001_init.sql`.
pub async fn setup_test_db() -> TestDb {
    let guard = db_lock().lock_owned().await;
    let url = test_db_url();
    let pool = bootstrap(&url, None)
        .await
        .unwrap_or_else(|e| panic!("bootstrap test SQLite at {url}: {e}"));
    reset_schema(&pool).await;
    TestDb { pool, _guard: guard }
}

async fn reset_schema(pool: &DbPool) {
    let objects: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, name FROM sqlite_master \
         WHERE type IN ('view', 'trigger', 'table') AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool.write)
    .await
    .expect("list test DB schema objects");

    let mut sql = String::from("PRAGMA foreign_keys = OFF;\n");
    for (kind, name) in &objects {
        let kind = match kind.as_str() {
            "view" => "VIEW",
            "trigger" => "TRIGGER",
            _ => "TABLE",
        };
        sql.push_str(&format!("DROP {kind} IF EXISTS \"{}\";\n", name.replace('"', "\"\"")));
    }
    sql.push_str("PRAGMA foreign_keys = ON;\n");
    sqlx::raw_sql(&sql)
        .execute(&pool.write)
        .await
        .expect("drop test DB schema objects");

    sqlx::raw_sql(SCHEMA)
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
}
