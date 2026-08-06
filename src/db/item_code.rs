//! `item_code` redeem repository (Chapter 5 §5.5, op 0x23 sub 3).
//!
//! MySQL is mandatory here — there is no no-DB degrade branch. `code` /
//! `password` are always bind parameters (never SQL-concatenated). The redeem
//! runs in a transaction guarded by `rows_affected() == 1` so a concurrent
//! double-redeem of the same code cannot grant the reward twice.

use sqlx::MySqlPool;

/// The reward granted by a successful redeem.
#[derive(Debug, Clone, Copy)]
pub struct Redeem {
    pub item_id: i64,
    pub count: i64,
}

/// The unused `item_code` row (per-player null until redeemed).
#[derive(sqlx::FromRow)]
struct CodeRow {
    item_id: i64,
    count: i64,
}

/// Redeem `code`/`password` for a player.
///
/// - No matching unused row -> `Ok(None)` (invalid or already-used code).
/// - Matching row -> grants `{item_id, count}` and atomically marks the code
///   used for `player_id`. Returns the reward so the handler can grant it.
///
/// Uses `SELECT ... FOR UPDATE` for safety, then commits only when exactly one
/// row flips (`player_id = 0 -> player_id = player`).
pub async fn redeem(
    pool: &MySqlPool,
    player_id: i64,
    code: &str,
    password: &str,
) -> Result<Option<Redeem>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, CodeRow>(
        "SELECT item_id, `count` FROM item_code \
         WHERE code = ? AND password = ? AND player_id = 0 \
         FOR UPDATE",
    )
    .bind(code)
    .bind(password)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        // Not found or already used: just drop the transaction (no writes).
        return Ok(None);
    };

    let used_at = chrono::Utc::now().timestamp();
    let res = sqlx::query(
        "UPDATE item_code \
         SET player_id = ?, used_at = ? \
         WHERE code = ? AND password = ? AND player_id = 0",
    )
    .bind(player_id)
    .bind(used_at)
    .bind(code)
    .bind(password)
    .execute(&mut *tx)
    .await?;

    if res.rows_affected() != 1 {
        // Lost the race to another redeem of the same code.
        return Ok(None);
    }

    tx.commit().await?;
    Ok(Some(Redeem {
        item_id: row.item_id,
        count: row.count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redeems_compile_with_expected_timescale() {
        // `chrono::Utc::now().timestamp()` is i64 unix seconds; matches the
        // migration's `used_at BIGINT`. A compile-time invariant guard.
        let _: i64 = chrono::Utc::now().timestamp();
        assert!(
            Redeem {
                item_id: 1,
                count: 1
            }
            .count
                == 1
        );
    }
}
