//! Official test database bootstrap (`CODING_STANDARDS.md` §3.4).
//!
//! Every DB-backed test runs against the file `DB/ts_dream_test.db` (override
//! with `TS_TEST_DB_URL`), using the same WAL + dual-pool bootstrap as
//! production. Because all tests share one file, [`setup_test_db`] takes a
//! process-wide lock and resets the schema; the returned [`TestDb`] holds that
//! lock until it is dropped, so DB tests inside one test binary run one at a
//! time while non-DB tests stay parallel.

use std::ops::Deref;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::Duration;

use tokio::sync::{Mutex, OwnedMutexGuard};
use ts_dream::db::pool::{bootstrap, DbPool};

/// Environment variable overriding the test database URL.
pub const TEST_DB_URL_ENV: &str = "TS_TEST_DB_URL";

const DB_LOCK_TIMEOUT: Duration = Duration::from_secs(15);
const SCHEMA: &str = include_str!("../../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../../migrations/0002_eve_quest_persistence.sql");

fn db_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

fn lock_holder() -> &'static StdMutex<Option<String>> {
    static HOLDER: OnceLock<StdMutex<Option<String>>> = OnceLock::new();
    HOLDER.get_or_init(|| StdMutex::new(None))
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

impl Drop for TestDb {
    fn drop(&mut self) {
        if let Ok(mut holder) = lock_holder().lock() {
            *holder = None;
        }
    }
}

/// Lock the shared test DB (with diagnostic timeout), drop every schema object,
/// and re-run migrations.
#[track_caller]
pub fn setup_test_db() -> impl std::future::Future<Output = TestDb> {
    let caller = std::panic::Location::caller();
    let thread_name = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_string();
    let requester = format!(
        "{}:{}:{} (thread `{}`)",
        caller.file(),
        caller.line(),
        caller.column(),
        thread_name
    );

    async move {
        let guard = tokio::time::timeout(DB_LOCK_TIMEOUT, db_lock().lock_owned())
            .await
            .unwrap_or_else(|_| {
                let current = lock_holder()
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                panic!(
                    "Timed out after {}s waiting for test DB lock at {} (currently held by: {})",
                    DB_LOCK_TIMEOUT.as_secs(),
                    requester,
                    current
                );
            });

        if let Ok(mut holder) = lock_holder().lock() {
            *holder = Some(requester);
        }

        let url = test_db_url();
        let pool = bootstrap(&url, None)
            .await
            .unwrap_or_else(|e| panic!("bootstrap test SQLite at {url}: {e}"));
        reset_schema(&pool).await;
        TestDb { pool, _guard: guard }
    }
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

    sqlx::raw_sql(MIGRATION_0002)
        .execute(&pool.write)
        .await
        .expect("execute 0002_eve_quest_persistence.sql migration");
}
