//! `item_code` redeem repository (Chapter 5 §5.5, op 0x23 sub 3).

use crate::db::pool::DbPool;
use crate::server::session::InventoryItem;

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
        newbie: bool,
    },
    /// Code did not exist, or was already redeemed by someone (`player_id != 0`).
    InvalidOrUsed,
    /// The once-only `TSVN123/TSVN456` gift was already claimed.
    AlreadyGifted,
}

/// Redeem `code`/`password` and grant the reward atomically.
pub async fn redeem_and_grant(
    pool: &DbPool,
    player_id: i64,
    code: &str,
    password: &str,
    slot: i64,
    item: &InventoryItem,
) -> Result<RedeemOutcome, sqlx::Error> {
    let mut tx = pool.write.begin().await?;

    let row = sqlx::query_as::<_, CodeRow>(
        "SELECT itemid AS item_id, count FROM item_code \
         WHERE code = ? AND password = ? AND playerid = 0",
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
         SET playerid = ?, usedat = ? \
         WHERE code = ? AND password = ? AND playerid = 0",
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
        newbie: false,
    })
}

/// The once-only `TSVN123/TSVN456` special gift (Chapter 5 §5.5).
pub async fn redeem_special_gift(
    pool: &DbPool,
    player_id: i64,
    items: &[InventoryItem],
) -> Result<RedeemOutcome, sqlx::Error> {
    let mut tx = pool.write.begin().await?;

    let newbie = sqlx::query_scalar::<_, i64>(
        "SELECT newbie FROM characters WHERE playerid = ?",
    )
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await?;

    match newbie {
        None | Some(1) => return Ok(RedeemOutcome::AlreadyGifted),
        _ => {}
    }

    let used_at = chrono::Utc::now().timestamp();
    let res = sqlx::query(
        "UPDATE item_code \
         SET playerid = ?, usedat = ? \
         WHERE code = 'TSVN123' AND password = 'TSVN456' AND playerid = 0",
    )
    .bind(player_id)
    .bind(used_at)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() != 1 {
        return Ok(RedeemOutcome::AlreadyGifted);
    }

    sqlx::query("UPDATE characters SET newbie = 1 WHERE playerid = ?")
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
        newbie: true,
    })
}

/// Preview the reward a code would grant, **without reserving or consuming it**.
pub async fn reward_for(
    pool: &DbPool,
    code: &str,
    password: &str,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT itemid AS item_id, count FROM item_code \
         WHERE code = ? AND password = ? AND playerid = 0",
    )
    .bind(code)
    .bind(password)
    .fetch_optional(&pool.read)
    .await?;
    Ok(row)
}
