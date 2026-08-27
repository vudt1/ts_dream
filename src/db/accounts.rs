//! Dashboard-facing account repository.
//!
//! The PC wire continues to use numeric `player_id`, `pass1`, and `pass2`.
//! Migration 0007 adds mobile-aligned identity and role metadata without
//! deleting the legacy credential columns.

use sqlx::MySqlPool;

/// One `accounts` row as exposed by the dashboard.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AccountRow {
    pub player_id: i64,
    pub account: String,
    #[serde(skip_serializing)]
    pub pass1: String,
    #[serde(skip_serializing)]
    pub pass2: String,
    pub is_suspended: bool,
    pub gm_level: i32,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
}

/// List every account, newest first (the dashboard table order).
pub async fn list(pool: &MySqlPool) -> Result<Vec<AccountRow>, sqlx::Error> {
    sqlx::query_as::<_, AccountRow>(
        "SELECT player_id, account, pass1, pass2, is_suspended, gm_level,
                created_at, last_login_at
         FROM accounts ORDER BY player_id DESC",
    )
    .fetch_all(pool)
    .await
}

/// Create a legacy-compatible PC account plus the new mobile-aligned metadata.
/// A UUID-derived temporary name is replaced with `legacy_<player_id>` in one
/// transaction, keeping the new NOT NULL unique account column satisfied.
pub async fn create(pool: &MySqlPool, pass1: &str, pass2: &str) -> Result<i64, sqlx::Error> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "INSERT INTO accounts (pass1, pass2, account, created_at, updated_at)
         VALUES (?, ?, CONCAT('pending_', UUID()), ?, ?)",
    )
    .bind(pass1)
    .bind(pass2)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let player_id = row.last_insert_id() as i64;
    sqlx::query("UPDATE accounts SET account = ? WHERE player_id = ?")
        .bind(format!("legacy_{player_id}"))
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(player_id)
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

/// Fetch both passwords for a `player_id` (op 0x23 change/delete gate).
/// Returns `None` when the account does not exist.
pub async fn passwords(
    pool: &MySqlPool,
    player_id: i64,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>("SELECT pass1, pass2 FROM accounts WHERE player_id = ?")
        .bind(player_id)
        .fetch_optional(pool)
        .await
}

/// Update both passwords for a `player_id` in one transaction
/// (op 0x23 sub 1). Returns `false` when the account does
/// not exist (no rows updated).
pub async fn change_pass(
    pool: &MySqlPool,
    player_id: i64,
    pass1: &str,
    pass2: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let now = chrono::Utc::now().timestamp_millis();
    let res =
        sqlx::query("UPDATE accounts SET pass1 = ?, pass2 = ?, updated_at = ? WHERE player_id = ?")
            .bind(pass1)
            .bind(pass2)
            .bind(now)
            .bind(player_id)
            .execute(&mut *tx)
            .await?;
    if res.rows_affected() != 1 {
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}
