//! `accounts` repository (Chapter 5 §5.8).
//!
//! Accounts are created exclusively through the web dashboard; there is no
//! import of `Member.ini` at bootstrap. Passwords stay plaintext (parity with
//! the C# server). The PK column is `player_id` (also the character/login id).
//! Every access is scoped by `player_id` — never the C# unscoped query.

use sqlx::MySqlPool;

/// One `accounts` row as exposed by the dashboard.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AccountRow {
    pub player_id: i64,
    pub pass1: String,
    pub pass2: String,
}

/// List every account, newest first (the dashboard table order).
pub async fn list(pool: &MySqlPool) -> Result<Vec<AccountRow>, sqlx::Error> {
    sqlx::query_as::<_, AccountRow>(
        "SELECT player_id, pass1, pass2 FROM accounts ORDER BY player_id DESC",
    )
    .fetch_all(pool)
    .await
}

/// Insert a new account and return its auto-incremented `player_id` via
/// `last_insert_id()` (not `max+1`, which races under concurrent creation).
pub async fn create(pool: &MySqlPool, pass1: &str, pass2: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("INSERT INTO accounts (pass1, pass2) VALUES (?, ?)")
        .bind(pass1)
        .bind(pass2)
        .execute(pool)
        .await?;
    Ok(row.last_insert_id() as i64)
}

/// Resolve `pass1` for a `player_id` (login gate). Returns `None` when the
/// account does not exist.
pub async fn pass1(pool: &MySqlPool, player_id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_as::<_, (String,)>("SELECT pass1 FROM accounts WHERE player_id = ?")
        .bind(player_id)
        .fetch_optional(pool)
        .await
        .map(|r| r.map(|(p,)| p))
}
