//! `item_code` redeem repository (Chapter 5 §5.5, op 0x23 sub 3).
//!
//! MySQL is mandatory here — there is no no-DB degrade branch. `code` /
//! `password` are always bind parameters (never SQL-concatenated). The redeem
//! runs in a transaction guarded by `rows_affected() == 1` so a concurrent
//! double-redeem of the same code cannot grant the reward twice.
//!
//! The reservation **and** the `homdo` grant live in
//! the same InnoDB transaction: if the inventory insert fails the redeem
//! rolls back, so a used code never "disappears" while the reward was lost.

use crate::server::session::InventoryItem;
use sqlx::MySqlPool;

/// One unused `item_code` row (the reward a code grants).
#[derive(sqlx::FromRow)]
struct CodeRow {
    item_id: i64,
    count: i64,
}

/// Outcome of a redeem attempt, distinguishing a real DB read error from every
/// user-visible result (used/not-found/special-gift states).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemOutcome {
    /// Code was unused and the reward has been granted (item now in `homdo`).
    Granted {
        item_id: i64,
        count: i64,
        tanthu: bool,
    },
    /// Code did not exist, or was already redeemed by someone (`player_id != 0`).
    InvalidOrUsed,
    /// The once-only `TSVN123/TSVN456` gift was already claimed.
    AlreadyGifted,
}

/// Redeem `code`/`password` and grant the reward atomically.
///
/// - No matching unused row -> `Ok(RedeemOutcome::InvalidOrUsed)`
///   (invalid or already-redeemed code).
/// - Matching row -> the code is marked used AND the item is inserted into
///   `homdo` (at `slot`) inside the same transaction; only then it commits.
///
/// `item` carries the exact in-memory `InventoryItem` the handler has already
/// added to the session (id/count/stat copy). The DB write and the session
/// mutation agree on slot and item.
pub async fn redeem_and_grant(
    pool: &MySqlPool,
    player_id: i64,
    code: &str,
    password: &str,
    slot: i64,
    item: &InventoryItem,
) -> Result<RedeemOutcome, sqlx::Error> {
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
        // Not found or already used: drop the transaction (no writes).
        return Ok(RedeemOutcome::InvalidOrUsed);
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
        return Ok(RedeemOutcome::InvalidOrUsed);
    }

    // Same transaction: grant the item (failure rolls everything back).
    crate::db::persist::upsert_item_tx(&mut tx, player_id, slot as u8, item).await?;

    tx.commit().await?;
    Ok(RedeemOutcome::Granted {
        item_id: row.item_id,
        count: row.count,
        tanthu: false,
    })
}

/// The once-only `TSVN123/TSVN456` special gift (Chapter 5 §5.5).
///
/// Grants the five hard-coded items (46197 + 20711 + 19711 + 23549 + 11001)
/// and sets the player's `tanthu` flag in the same transaction as the `homdo`
/// inserts, guarded against a concurrent double-claim. The `item_code`
/// reservation is not consulted; the once-only
/// guard is the `tanthu` flag.
pub async fn redeem_special_gift(
    pool: &MySqlPool,
    player_id: i64,
    items: &[InventoryItem],
) -> Result<RedeemOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let tanthu = sqlx::query_scalar::<_, i64>(
        "SELECT tanthu FROM characters WHERE account_id = ? FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await?;

    match tanthu {
        None | Some(1) => return Ok(RedeemOutcome::AlreadyGifted),
        _ => {}
    }

    let used_at = chrono::Utc::now().timestamp();
    let res = sqlx::query(
        "UPDATE item_code \
         SET player_id = ?, used_at = ? \
         WHERE code = 'TSVN123' AND password = 'TSVN456' AND player_id = 0",
    )
    .bind(player_id)
    .bind(used_at)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() != 1 {
        return Ok(RedeemOutcome::AlreadyGifted);
    }

    sqlx::query("UPDATE characters SET tanthu = 1 WHERE account_id = ?")
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
    for (i, item) in items.iter().enumerate() {
        crate::db::persist::upsert_item_tx(&mut tx, player_id, (i as u8) + 1, item).await?;
    }

    tx.commit().await?;
    Ok(RedeemOutcome::Granted {
        item_id: items.first().map(|i| i64::from(i.id)).unwrap_or(0),
        count: items.first().map(|i| i64::from(i.count)).unwrap_or(0),
        tanthu: true,
    })
}

/// Preview the reward a code would grant, **without reserving or consuming it**
/// (op 0x23 sub 3 pre-validation: the handler checks the item template and the
/// bag capacity before committing to the atomic redeem). Returns the reusable
/// `(item_id, count)` when an unused row matches.
pub async fn reward_for(
    pool: &MySqlPool,
    code: &str,
    password: &str,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT item_id, `count` FROM item_code \
         WHERE code = ? AND password = ? AND player_id = 0",
    )
    .bind(code)
    .bind(password)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
