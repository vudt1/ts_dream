//! `accounts` repository — byte-exact credential checks (0001 table reused).

use crate::db::modern::traits::{AccountRepository, RepoResult};
use sqlx::MySqlPool;

/// Account metadata required by the live PC login and GM authorization paths.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AccountAccess {
    pub account_id: i64,
    pub account_name: String,
    pub is_suspended: bool,
    pub suspended_until: Option<i64>,
    pub gm_level: i32,
}

pub struct MySqlAccountRepository<'a> {
    pub pool: &'a MySqlPool,
}

impl MySqlAccountRepository<'_> {
    pub async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(self.pool, account_id, "pass1", pass).await
    }

    pub async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(self.pool, account_id, "pass2", pass).await
    }

    /// Load role/suspension state after the numeric PC account id is known.
    pub async fn access(&self, account_id: i64) -> RepoResult<Option<AccountAccess>> {
        sqlx::query_as::<_, AccountAccess>(
            "SELECT player_id AS account_id, account AS account_name,
                    is_suspended, suspended_until, gm_level
             FROM accounts WHERE player_id = ?",
        )
        .bind(account_id)
        .fetch_optional(self.pool)
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

    pub async fn touch_login(&self, account_id: i64, now_ms: i64) -> RepoResult<()> {
        sqlx::query("UPDATE accounts SET last_login_at = ?, updated_at = ? WHERE player_id = ?")
            .bind(now_ms)
            .bind(now_ms)
            .bind(account_id)
            .execute(self.pool)
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
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE accounts SET gm_level = ?, updated_at = ?
             WHERE player_id = ? AND gm_level <= ?",
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
        .execute(self.pool)
        .await
        .map(|_| ())
    }
}

impl AccountRepository for MySqlAccountRepository<'_> {
    async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        self.verify_pass1(account_id, pass).await
    }

    async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        self.verify_pass2(account_id, pass).await
    }
}

/// HEX comparison keeps the latin1-stored bytes and the wire bytes identical;
/// the column name is a compile-time constant, never user input.
async fn verify(pool: &MySqlPool, account_id: i64, column: &str, pass: &[u8]) -> RepoResult<bool> {
    let sql = format!(
        "SELECT COUNT(*) FROM accounts \
         WHERE player_id = ? AND HEX({column}) = HEX(?)"
    );
    let hits: i64 = sqlx::query_scalar(&sql)
        .bind(account_id)
        .bind(pass)
        .fetch_one(pool)
        .await?;
    Ok(hits > 0)
}

#[cfg(test)]
mod tests {
    use super::{AccountAccess, MySqlAccountRepository};

    #[test]
    fn suspension_expiry_is_enforced_without_mutating_storage() {
        let active = AccountAccess {
            account_id: 1,
            account_name: "a".into(),
            is_suspended: true,
            suspended_until: Some(2_000),
            gm_level: 0,
        };
        assert!(MySqlAccountRepository::is_suspended(&active, 1_999));
        assert!(!MySqlAccountRepository::is_suspended(&active, 2_000));
        let permanent = AccountAccess {
            suspended_until: None,
            ..active
        };
        assert!(MySqlAccountRepository::is_suspended(&permanent, 9_999));
    }
}
