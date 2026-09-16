//! `accounts` repository — byte-exact credential checks (0001 table reused).

use crate::db::modern::traits::{AccountRepository, RepoResult};
use crate::db::pool::DbPool;
use sqlx::SqlitePool;

/// Account metadata required by the live PC login and GM authorization paths.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AccountAccess {
    pub account_id: i64,
    pub account_name: String,
    pub is_suspended: bool,
    pub suspended_until: Option<i64>,
    pub gm_level: i32,
}

pub struct SqliteAccountRepository<'a> {
    pub pool: &'a DbPool,
}

impl SqliteAccountRepository<'_> {
    pub async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(&self.pool.read, account_id, "pass1", pass).await
    }

    pub async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(&self.pool.read, account_id, "pass2", pass).await
    }

    pub async fn update_pass2(&self, account_id: i64, pass2: &[u8]) -> RepoResult<()> {
        let s = std::str::from_utf8(pass2).unwrap_or("");
        if !s.is_empty() {
            let now_ms = chrono::Utc::now().timestamp_millis();
            sqlx::query("UPDATE accounts SET pass2 = ?, updatedat = ? WHERE playerid = ?")
                .bind(s)
                .bind(now_ms)
                .bind(account_id)
                .execute(&self.pool.write)
                .await?;
        }
        Ok(())
    }

    /// Load role/suspension state after the numeric PC account id is known.
    /// `account_name` is derived from `player_id` (no `account` column exists;
    /// shared PK `accounts.player_id = characters.character_id`).
    pub async fn access(&self, account_id: i64) -> RepoResult<Option<AccountAccess>> {
        sqlx::query_as::<_, AccountAccess>(
            "SELECT playerid AS account_id, CAST(playerid AS TEXT) AS account_name,
                    issuspended AS is_suspended, suspendeduntil AS suspended_until, gmlevel AS gm_level
             FROM accounts WHERE playerid = ?",
        )
        .bind(account_id)
        .fetch_optional(&self.pool.read)
        .await
    }

    /// An expired suspension is inactive without silently mutating stored data.
    pub fn is_suspended(access: &AccountAccess, now_ms: i64) -> bool {
        access.is_suspended
            && access
                .suspended_until
                .map(|until| until <= 0 || until > now_ms)
                .unwrap_or(true)
    }

    pub async fn touch_login(&self, account_id: i64, now_ms: i64, ip: &str) -> RepoResult<()> {
        let ip_val = if ip.is_empty() { None } else { Some(ip) };
        sqlx::query("UPDATE accounts SET lastlogin_at = ?, lastloginip = ?, updatedat = ? WHERE playerid = ?")
            .bind(now_ms)
            .bind(ip_val)
            .bind(now_ms)
            .bind(account_id)
            .execute(&self.pool.write)
            .await
            .map(|_| ())
    }

    /// Server-side GM mutation. The actor policy is checked here as a second
    /// line of defense; callers must still apply command-level authorization.
    pub async fn set_gm_level(
        &self,
        actor_account_id: i64,
        actor_gm_level: i32,
        target_account_id: i64,
        target_gm_level: i32,
        action: &str,
        details: &str,
    ) -> RepoResult<bool> {
        const GM_MAX_LEVEL: i32 = 99;
        if actor_gm_level < 50
            || !(0..=GM_MAX_LEVEL).contains(&target_gm_level)
            || target_account_id == actor_account_id
            || target_gm_level > actor_gm_level
        {
            return Ok(false);
        }
        let mut tx = self.pool.write.begin().await?;
        let result = sqlx::query(
            "UPDATE accounts SET gmlevel = ?, updatedat = ?
             WHERE playerid = ? AND gmlevel <= ?",
        )
        .bind(target_gm_level)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(target_account_id)
        .bind(actor_gm_level)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO gm_audit_log
             (actor_account_id, actor_gm_level, target_account_id, action, details, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(actor_account_id)
        .bind(actor_gm_level)
        .bind(target_account_id)
        .bind(action)
        .bind(details)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn write_gm_audit(
        &self,
        actor_account_id: i64,
        actor_gm_level: i32,
        target_account_id: Option<i64>,
        action: &str,
        details: &str,
    ) -> RepoResult<()> {
        sqlx::query(
            "INSERT INTO gm_audit_log
             (actor_account_id, actor_gm_level, target_account_id, action, details, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(actor_account_id)
        .bind(actor_gm_level)
        .bind(target_account_id)
        .bind(action)
        .bind(details)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&self.pool.write)
        .await
        .map(|_| ())
    }
}

impl AccountRepository for SqliteAccountRepository<'_> {
    async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        self.verify_pass1(account_id, pass).await
    }

    async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        self.verify_pass2(account_id, pass).await
    }

    async fn update_pass2(&self, account_id: i64, pass2: &[u8]) -> RepoResult<()> {
        self.update_pass2(account_id, pass2).await
    }
}

/// HEX comparison keeps the stored bytes and the wire bytes identical;
/// the column name is a compile-time constant, never user input.
async fn verify(pool: &SqlitePool, account_id: i64, column: &str, pass: &[u8]) -> RepoResult<bool> {
    let sql = format!(
        "SELECT COUNT(*) FROM accounts \
         WHERE playerid = ? AND HEX({column}) = HEX(?)"
    );
    let hits: i64 = sqlx::query_scalar(&sql)
        .bind(account_id)
        .bind(pass)
        .fetch_one(pool)
        .await?;
    Ok(hits > 0)
}
