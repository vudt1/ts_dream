//! Atomic multi-step money and item operations (ticket 06).
//!
//! Each function owns one transaction: guarded balance updates plus inventory
//! writes commit together or not at all, so a crash mid-trade can never
//! duplicate or destroy an item.

use crate::db::modern::model::{InventorySlot, Money, StorageType};
use crate::db::modern::mysql::characters::money_row;
use crate::db::modern::mysql::inventories::MySqlInventoryRepository;
use crate::db::modern::traits::InventoryRepository;
use sqlx::MySqlPool;

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
    pool: &MySqlPool,
    character_id: i64,
    amount: i64,
) -> Result<Money, TxError> {
    let mut tx = pool.begin().await?;

    let (debit, credit, guard_col) = if amount >= 0 {
        (-amount, amount, "gold")
    } else {
        (amount, -amount, "bank_gold")
    };
    let cost = debit.abs();

    let updated = sqlx::query(&format!(
        "UPDATE character_money SET gold = gold + ?, bank_gold = bank_gold + ? \
         WHERE character_id = ? AND {guard_col} >= ?"
    ))
    .bind(debit)
    .bind(credit)
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
/// either both the ledger and the bag row land, or neither does. The target
/// slot must be vacant (locked `FOR UPDATE` so a concurrent write cannot
/// sneak an item into the same cell between check and insert).
pub async fn shop_buy(
    pool: &MySqlPool,
    character_id: i64,
    price: i64,
    purchase: &InventorySlot,
) -> Result<(), TxError> {
    let mut tx = pool.begin().await?;

    let updated = sqlx::query(
        "UPDATE character_money SET gold = gold - ? WHERE character_id = ? AND gold >= ?",
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

    MySqlInventoryRepository { pool }
        .save_slot(character_id, purchase, &mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Peer-to-peer trade: moves the item in `(from_character, offer)` to
/// `(to_character, destination)` atomically. Both cells are locked
/// `FOR UPDATE` before anything moves, so two concurrent trades can neither
/// double-spend the source item nor race into the same destination.
pub async fn p2p_trade(
    pool: &MySqlPool,
    from_character_id: i64,
    offer: (StorageType, u16),
    to_character_id: i64,
    destination: (StorageType, u16),
) -> Result<(), TxError> {
    let mut tx = pool.begin().await?;
    let repo = MySqlInventoryRepository { pool };

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

/// Locks the inventory row (or its gap) for the given cell and fails when it
/// is occupied. Running inside the caller's transaction makes the vacancy
/// check race-free until commit.
async fn ensure_slot_vacant(
    tx: &mut sqlx::MySqlConnection,
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
    tx: &mut sqlx::MySqlConnection,
    character_id: i64,
    storage_type: StorageType,
    slot: u16,
) -> Result<(), TxError> {
    match lock_inventory_cell(tx, character_id, storage_type, slot).await? {
        Some(id) if id != 0 => Ok(()),
        _ => Err(TxError::SourceSlotEmpty),
    }
}

/// `SELECT ... FOR UPDATE` on one inventory cell; `None` when the row does
/// not exist yet (InnoDB still takes the gap lock under REPEATABLE READ).
async fn lock_inventory_cell(
    tx: &mut sqlx::MySqlConnection,
    character_id: i64,
    storage_type: StorageType,
    slot: u16,
) -> Result<Option<i64>, TxError> {
    let found = sqlx::query_scalar::<_, i64>(
        "SELECT item_id FROM inventories \
         WHERE character_id = ? AND storage_type = ? AND slot = ? FOR UPDATE",
    )
    .bind(character_id)
    .bind(storage_type.value())
    .bind(slot)
    .fetch_optional(&mut *tx)
    .await?;
    Ok(found)
}
