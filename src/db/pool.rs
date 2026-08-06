//! MySQL 8 pool bootstrap (Chapter 5).
//!
//! Connect with `MySqlPool`, set connection `charset = latin1` so stored
//! VISCII byte names are never transcoded. Fail-fast on boot.

use crate::error::{Result, TsError};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::MySqlPool;
use std::str::FromStr;

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
fn strip_database_path(database_url: &str) -> String {
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
        .map_err(|e| TsError::Db(e))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_database_path_removes_path() {
        assert_eq!(
            strip_database_path("mysql://user:pass@localhost:3306/ts_dream"),
            "mysql://user:pass@localhost:3306"
        );
    }

    #[test]
    fn strip_database_path_removes_query_too() {
        assert_eq!(
            strip_database_path("mysql://u:p@localhost/ts_dream?ssl=true"),
            "mysql://u:p@localhost"
        );
    }

    #[test]
    fn strip_database_path_keeps_url_without_path() {
        assert_eq!(
            strip_database_path("mysql://user:pass@localhost:3306"),
            "mysql://user:pass@localhost:3306"
        );
    }
}
