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
    let mut opts = MySqlConnectOptions::from_str(database_url)
        .map_err(|e| TsError::Config(format!("invalid database_url: {e}")))?;
    opts = opts.charset("latin1");
    MySqlPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(opts)
        .await
        .map_err(|e| TsError::Db(e))
}

/// Apply embedded migrations.
pub async fn migrate(pool: &MySqlPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| TsError::Migrate(e.to_string()))
}