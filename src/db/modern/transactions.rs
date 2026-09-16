//! Atomic multi-step money and item operations (ticket 06).
//!
//! Each function owns one transaction: guarded balance updates plus inventory
//! writes commit together or not at all, so a crash mid-trade can never
//! duplicate or destroy an item.

use crate::db::modern::model::{InventorySlot, Money, StorageType};
use crate::db::modern::sqlite::characters::money_row;
use crate::db::modern::sqlite::inventories::SqliteInventoryRepository;
use crate::db::modern::traits::InventoryRepository;
use crate::db::pool::DbPool;

/// Failure modes surfaced to handlers so they can send the matching client
/// error frame instead of a bare 500-style rejection.
#[derive(Debug)]
pub enum TxError {
    InsufficientFunds,
    SourceSlotEmpty,
    DestinationSlotOccupied,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for TxError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

/// Moves `|amount|` gold between pocket and bank in whichever direction the
/// sign says (`> 0` deposits, `< 0` withdraws). The `gold >= cost` /
/// `bank_gold >= cost` predicate inside the UPDATE makes overdrafts
/// impossible even under concurrent transfers.
pub async fn bank_transfer(
    pool: &DbPool,
    character_id: i64,
    amount: i64,
) -> Result<Money, TxError> {
    let mut tx = pool.write.begin().await?;

    let (gold_delta, bank_delta, guard_col) = if amount >= 0 {
        (-amount, amount, "gold")
    } else {
        (-amount, amount, "bankgold")
    };
    let cost = amount.abs();

    let updated = sqlx::query(&format!(
        "UPDATE character_money SET gold = gold + ?, bankgold = bankgold + ? \
         WHERE playerid = ? AND {guard_col} >= ?"
    ))
    .bind(gold_delta)
    .bind(bank_delta)
    .bind(character_id)
    .bind(cost)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(TxError::InsufficientFunds);
    }

    let money = money_row(&mut *tx, character_id).await?;
    tx.commit().await?;
    Ok(money)
}

/// Deducts `price` gold and grants the purchased item in one transaction:
/// either both the ledger and the bag row land, or neither does.
pub async fn shop_buy(
    pool: &DbPool,
    character_id: i64,
    price: i64,
    purchase: &InventorySlot,
) -> Result<(), TxError> {
    let mut tx = pool.write.begin().await?;

    let updated = sqlx::query(
        "UPDATE character_money SET gold = gold - ? WHERE playerid = ? AND gold >= ?",
    )
    .bind(price)
    .bind(character_id)
    .bind(price)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(TxError::InsufficientFunds);
    }

    ensure_slot_vacant(&mut tx, character_id, purchase.storage_type, purchase.slot).await?;

    SqliteInventoryRepository { pool }
        .save_slot(character_id, purchase, &mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Peer-to-peer trade: moves the item in `(from_character, offer)` to
/// `(to_character, destination)` atomically.
pub async fn p2p_trade(
    pool: &DbPool,
    from_character_id: i64,
    offer: (StorageType, u16),
    to_character_id: i64,
    destination: (StorageType, u16),
) -> Result<(), TxError> {
    let mut tx = pool.write.begin().await?;
    let repo = SqliteInventoryRepository { pool };

    lock_occupied_slot(&mut tx, from_character_id, offer.0, offer.1).await?;
    let Some(item) = repo
        .load_slot(from_character_id, offer.0, offer.1)
        .await?
        .filter(|s| !s.is_empty())
    else {
        return Err(TxError::SourceSlotEmpty);
    };

    ensure_slot_vacant(&mut tx, to_character_id, destination.0, destination.1).await?;

    repo.clear_slot(from_character_id, offer.0, offer.1, &mut *tx)
        .await?;
    let moved = InventorySlot {
        storage_type: destination.0,
        slot: destination.1,
        item: item.item,
    };
    repo.save_slot(to_character_id, &moved, &mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

/// Locks the inventory row for the given cell and fails when it
/// is occupied. Running inside the caller's transaction makes the vacancy
/// check race-free until commit.
async fn ensure_slot_vacant(
    tx: &mut sqlx::SqliteConnection,
    character_id: i64,
    storage_type: StorageType,
    slot: u16,
) -> Result<(), TxError> {
    let occupied = lock_inventory_cell(tx, character_id, storage_type, slot)
        .await?
        .unwrap_or(0);
    if occupied != 0 {
        return Err(TxError::DestinationSlotOccupied);
    }
    Ok(())
}

/// Locks the cell and fails when there is no live (non-zero) item in it.
async fn lock_occupied_slot(
    tx: &mut sqlx::SqliteConnection,
    character_id: i64,
    storage_type: StorageType,
    slot: u16,
) -> Result<(), TxError> {
    match lock_inventory_cell(tx, character_id, storage_type, slot).await? {
        Some(id) if id != 0 => Ok(()),
        _ => Err(TxError::SourceSlotEmpty),
    }
}

/// `SELECT` on one inventory cell; `None` when the row does
/// not exist yet. Serialized by the exclusive single-writer connection.
async fn lock_inventory_cell(
    tx: &mut sqlx::SqliteConnection,
    character_id: i64,
    storage_type: StorageType,
    slot: u16,
) -> Result<Option<i64>, TxError> {
    let found = sqlx::query_scalar::<_, i64>(
        "SELECT itemid FROM inventories \
         WHERE playerid = ? AND storagetype = ? AND slot = ?",
    )
    .bind(character_id)
    .bind(storage_type.value())
    .bind(slot)
    .fetch_optional(&mut *tx)
    .await?;
    Ok(found)
}
