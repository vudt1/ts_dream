//! `accounts` repository — byte-exact credential checks (0001 table reused).

use crate::db::modern::traits::{AccountRepository, RepoResult};
use sqlx::MySqlPool;

pub struct MySqlAccountRepository<'a> {
    pub pool: &'a MySqlPool,
}

impl AccountRepository for MySqlAccountRepository<'_> {
    async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(self.pool, account_id, "pass1", pass).await
    }

    async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool> {
        verify(self.pool, account_id, "pass2", pass).await
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
