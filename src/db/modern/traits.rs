//! Async repository traits for the modern schema (ticket 06).
//!
//! Domain code depends on these traits, never on `MySqlPool` directly, so the
//! persistence layer stays swappable and unit-testable. All methods are
//! native `async fn`, used via static dispatch only (the MySQL
//! implementations live in [`super::mysql`]); dyn-compatibility is not a goal,
//! so the `async_fn_in_trait` lint is intentionally allowed.
#![allow(async_fn_in_trait)]

use crate::db::modern::model::{
    InventorySlot, MissionRow, Money, PetRecord, PetStorageType, SkillRow, StorageType,
};

/// Result alias shared by every repository method.
pub type RepoResult<T> = std::result::Result<T, sqlx::Error>;

/// Account credential lookups against `accounts` (0001 table reused as root).
pub trait AccountRepository {
    /// Byte-exact pass1 check (`latin1_bin`, compared via HEX so VISCII bytes
    /// never transcode).
    async fn verify_pass1(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool>;

    /// Byte-exact pass2 (secondary password) check.
    async fn verify_pass2(&self, account_id: i64, pass: &[u8]) -> RepoResult<bool>;
}

/// Character lifecycle against `characters` + `character_money`.
pub trait CharacterRepository {
    /// Creates the character row plus its empty money ledger; returns the new
    /// character id. Atomic: both inserts commit or roll back together.
    async fn create(&self, account_id: i64, name: &[u8], seed: &CharacterSeed) -> RepoResult<i64>;

    /// Lists the characters under one account (login character-select data).
    async fn list_by_account(&self, account_id: i64) -> RepoResult<Vec<CharacterSummary>>;

    /// Resolves a character id by exact VISCII name bytes.
    async fn find_id_by_name(&self, name: &[u8]) -> RepoResult<Option<i64>>;

    /// Loads the currency ledger row.
    async fn load_money(&self, character_id: i64) -> RepoResult<Money>;

    /// Deletes a character and every dependent row in one transaction.
    async fn delete(&self, character_id: i64) -> RepoResult<()>;
}

/// Creation defaults applied by [`CharacterRepository::create`].
#[derive(Debug, Clone)]
pub struct CharacterSeed {
    pub level: i64,
    pub sex: i64,
    pub hair: i64,
    pub element: i64,
    pub map_id: i64,
    pub map_x: i64,
    pub map_y: i64,
}

impl Default for CharacterSeed {
    fn default() -> Self {
        Self {
            level: 1,
            sex: 0,
            hair: 0,
            element: 0,
            map_id: 0,
            map_x: 0,
            map_y: 0,
        }
    }
}

/// Character-select summary row.
#[derive(Debug, Clone)]
pub struct CharacterSummary {
    pub id: i64,
    pub name: Vec<u8>,
    pub level: i64,
    pub sex: i64,
    pub hair: i64,
    pub element: i64,
}

/// Item container access against `inventories`.
pub trait InventoryRepository {
    /// Loads every non-empty slot of one container ordered by slot.
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: StorageType,
    ) -> RepoResult<Vec<InventorySlot>>;

    /// Upserts one slot (INSERT ... ON DUPLICATE KEY UPDATE on all 20
    /// ThingData columns). Accepts a pool handle or an open transaction so
    /// callers can compose multi-slot moves atomically.
    async fn save_slot<'e, E>(
        &self,
        character_id: i64,
        slot: &InventorySlot,
        executor: E,
    ) -> RepoResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::MySql>;

    /// Resets a slot back to vacancy (`item_id = 0` row retained).
    async fn clear_slot<'e, E>(
        &self,
        character_id: i64,
        storage_type: StorageType,
        slot: u16,
        executor: E,
    ) -> RepoResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::MySql>;

    /// Loads one slot, if occupied.
    async fn load_slot(
        &self,
        character_id: i64,
        storage_type: StorageType,
        slot: u16,
    ) -> RepoResult<Option<InventorySlot>>;
}

/// General storage access against `character_pets` (4 storages per character).
pub trait PetRepository {
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
    ) -> RepoResult<Vec<PetRecord>>;

    async fn save_pet(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
        pet: &PetRecord,
    ) -> RepoResult<()>;

    async fn delete_pet(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
        slot: u16,
    ) -> RepoResult<()>;
}

/// Mission bookkeeping: progress rows, permanent bit flags, completed events.
pub trait QuestRepository {
    async fn upsert_mission(
        &self,
        character_id: i64,
        mission: &MissionRow,
    ) -> RepoResult<()>;

    async fn list_missions(&self, character_id: i64) -> RepoResult<Vec<MissionRow>>;

    async fn set_bit_flag(&self, character_id: i64, flag_index: u32, now: i64) -> RepoResult<()>;

    async fn has_bit_flag(&self, character_id: i64, flag_index: u32) -> RepoResult<bool>;

    async fn mark_event_completed(
        &self,
        character_id: i64,
        event_id: i64,
        completed_at: i64,
    ) -> RepoResult<()>;

    async fn has_completed_event(&self, character_id: i64, event_id: i64) -> RepoResult<bool>;

    /// Replaces the learned-skill set atomically inside the caller's
    /// transaction (reborn / respec).
    async fn replace_skills(
        &self,
        character_id: i64,
        skills: &[SkillRow],
        tx: &mut sqlx::MySqlConnection,
    ) -> RepoResult<()>;
}
