//! Dashboard-facing account repository.
//!
//! The PC wire authenticates with numeric `player_id` + plaintext `pass1`
//! (login gate) and `pass2` (op 0x23 change/delete gate), compared byte-exact
//! via `HEX(pass1) = HEX(?)`. The `accounts` table (migration 0001) carries no
//! hashed-password column — plaintext parity with the C# server is intentional.

use crate::db::pool::DbPool;

/// One `accounts` row as exposed by the dashboard.
/// PK là `player_id` (shared PK với `characters.character_id`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AccountRow {
    pub player_id: i64,
    #[serde(skip_serializing)]
    pub pass1: String,
    #[serde(skip_serializing)]
    pub pass2: String,
    pub is_suspended: bool,
    pub gm_level: i32,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub last_login_ip: Option<String>,
}

/// List every account, newest first (the dashboard table order).
pub async fn list(pool: &DbPool) -> Result<Vec<AccountRow>, sqlx::Error> {
    sqlx::query_as::<_, AccountRow>(
        "SELECT playerid AS player_id, pass1, pass2, issuspended AS is_suspended, gmlevel AS gm_level,
                 createdat AS created_at, lastlogin_at AS last_login_at, lastloginip AS last_login_ip
         FROM accounts ORDER BY playerid DESC",
    )
    .fetch_all(&pool.read)
    .await
}

/// Create a PC account — chỉ cần `pass1`/`pass2`, PK `player_id` tự tăng.
/// Không còn cột `account`; identity duy nhất là `player_id` và được dùng làm
/// `characters.character_id` (shared PK 1:1).
pub async fn create(pool: &DbPool, pass1: &str, pass2: &str) -> Result<i64, sqlx::Error> {
    let now = chrono::Utc::now().timestamp_millis();
    let row = sqlx::query(
        "INSERT INTO accounts (pass1, pass2, createdat, updatedat)
         VALUES (?, ?, ?, ?)",
    )
    .bind(pass1)
    .bind(pass2)
    .bind(now)
    .bind(now)
    .execute(&pool.write)
    .await?;
    Ok(row.last_insert_rowid())
}

/// Password policy shared by the dashboard create path and op 0x23 change:
/// 8..=10 bytes, every byte printable ASCII (`0x21..=0x7E`) — no space, no
/// control chars, and never UTF-8/VISCII, so the byte-exact `HEX(pass)=HEX(?)`
/// login compare always round-trips.
pub fn is_valid_password(pass: &[u8]) -> bool {
    (8..=10).contains(&pass.len()) && pass.iter().all(|&b| (0x21..=0x7E).contains(&b))
}

/// Resolve `pass1` for a `player_id` (login gate). Returns `None` when the
/// account does not exist.
pub async fn pass1(pool: &DbPool, player_id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_as::<_, (String,)>("SELECT pass1 FROM accounts WHERE playerid = ?")
        .bind(player_id)
        .fetch_optional(&pool.read)
        .await
        .map(|r| r.map(|(p,)| p))
}

/// Fetch both passwords for a `player_id` (op 0x23 change/delete gate).
/// Returns `None` when the account does not exist.
pub async fn passwords(
    pool: &DbPool,
    player_id: i64,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>("SELECT pass1, pass2 FROM accounts WHERE playerid = ?")
        .bind(player_id)
        .fetch_optional(&pool.read)
        .await
}

/// Update both passwords for a `player_id` in one transaction
/// (op 0x23 sub 1). Returns `false` when the account does
/// not exist (no rows updated).
pub async fn change_pass(
    pool: &DbPool,
    player_id: i64,
    pass1: &str,
    pass2: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    let now = chrono::Utc::now().timestamp_millis();
    let res =
        sqlx::query("UPDATE accounts SET pass1 = ?, pass2 = ?, updatedat = ? WHERE playerid = ?")
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
