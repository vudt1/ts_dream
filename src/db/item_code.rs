//! `item_code` redeem repository (Chapter 5 §5.5, op 0x23 sub 3).
//!
//! MySQL is mandatory here — there is no no-DB degrade branch. `code` /
//! `password` are always bind parameters (never SQL-concatenated). The redeem
//! runs in a transaction guarded by `rows_affected() == 1` so a concurrent
//! double-redeem of the same code cannot grant the reward twice.
//!
//! Unlike the C# version (which reserves the code and grants the item in two
//! separate statements), here the reservation **and** the `homdo` grant live in
//! the same InnoDB transaction: if the inventory insert fails the redeem
//! rolls back, so a used code never "disappears" while the reward was lost.

use crate::server::session::InventoryItem;
use sqlx::{MySql, MySqlPool, Transaction};

/// The reward granted a successful redeem.
#[derive(Debug, Clone)]
pub struct Redeem {
    pub item_id: i64,
    pub count: i64,
}

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
    Granted { item_id: i64, count: i64, tanthu: bool },
    /// Code did not exist, or was already redeemed by someone (`player_id != 0`).
    InvalidOrUsed,
    /// The once-only `TSVN123/TSVN456` gift was already claimed.
    AlreadyGifted,
}

const HOMDO_COLS: &str = "(player_id, Slot, Id, `Count`, Lv, DoBen, Int1, Atk1, Def1, Hpx1, Spx1, Agi1, \
     Fai1, Int2, Atk2, Def2, Hpx2, Spx2, Agi2, Fai2, Hp, Sp, `Long`, GiatriLong, Khang, \
     Thuoctinh, GiatriThuoctinh, Loai, Texp)";
const HOMDO_PLACEHOLDERS: &str = "(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

/// Insert one inventory row into `homdo` inside `tx` (same column layout as
/// [`crate::db::persist::upsert_item`], but operating on an open transaction so
/// the redeem + grant are atomic).
async fn insert_homdo_tx(
    tx: &mut Transaction<'_, MySql>,
    player_id: i64,
    slot: i64,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let q = format!(
        "INSERT INTO homdo {HOMDO_COLS} VALUES {HOMDO_PLACEHOLDERS} \
         ON DUPLICATE KEY UPDATE Id = VALUES(Id), `Count` = VALUES(`Count`)"
    );
    sqlx::query(&q)
        .bind(player_id)
        .bind(slot)
        .bind(i64::from(item.id))
        .bind(i64::from(item.count))
        .bind(i64::from(item.lv))
        .bind(i64::from(item.doben))
        .bind(i64::from(item.int1))
        .bind(i64::from(item.atk1))
        .bind(i64::from(item.def1))
        .bind(i64::from(item.hpx1))
        .bind(i64::from(item.spx1))
        .bind(i64::from(item.agi1))
        .bind(i64::from(item.fai1))
        .bind(i64::from(item.int2))
        .bind(i64::from(item.atk2))
        .bind(i64::from(item.def2))
        .bind(i64::from(item.hpx2))
        .bind(i64::from(item.spx2))
        .bind(i64::from(item.agi2))
        .bind(i64::from(item.fai2))
        .bind(i64::from(item.item_hp))
        .bind(i64::from(item.item_sp))
        .bind(i64::from(item.long_val))
        .bind(i64::from(item.giatri_long))
        .bind(i64::from(item.khang))
        .bind(i64::from(item.thuoctinh))
        .bind(i64::from(item.giatri_thuoctinh))
        .bind(i64::from(item.loai))
        .bind(i64::from(item.texp))
        .execute(&mut **tx)
        .await?;
    Ok(())
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
    insert_homdo_tx(&mut tx, player_id, slot, item).await?;

    tx.commit().await?;
    Ok(RedeemOutcome::Granted {
        item_id: row.item_id,
        count: row.count,
        tanthu: false,
    })
}

/// The once-only `TSVN123/TSVN456` special gift (Chapter 5 §5.5, Client.cs:7591-7617).
///
/// Grants the five hard-coded items (46197 + 20711 + 19711 + 23549 + 11001)
/// and sets the player's `tanthu` flag in the same transaction as the `homdo`
/// inserts, guarded against a concurrent double-claim. The `item_code`
/// reservation is not consulted (C# updates it unconditionally); the once-only
/// guard is the `tanthu` flag.
pub async fn redeem_special_gift(
    pool: &MySqlPool,
    player_id: i64,
    items: &[InventoryItem],
) -> Result<RedeemOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let tanthu = sqlx::query_scalar::<_, i64>(
        "SELECT tanthu FROM players WHERE player_id = ? FOR UPDATE",
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

    sqlx::query("UPDATE players SET tanthu = 1 WHERE player_id = ?")
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
    for (i, item) in items.iter().enumerate() {
        insert_homdo_tx(&mut tx, player_id, (i as i64) + 1, item).await?;
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

/// Legacy reservation-only redeem: flips the code to used without granting an
/// item in the same transaction. Prefer [`redeem_and_grant`] for op 0x23 sub 3;
/// this keeps the old `SELECT`/`UPDATE` semantics for any caller that needs it.
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
        return Ok(None);
    }

    tx.commit().await?;
    Ok(Some(Redeem {
        item_id: row.item_id,
        count: row.count,
    }))
}