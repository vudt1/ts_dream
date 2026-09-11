//! SQLite implementations of the modern repository traits.
//!
//! Each submodule owns the SQL for exactly one table group so queries stay
//! centralized (same rule as the legacy `src/db` modules).

pub mod accounts;
pub mod characters;
pub mod inventories;
pub mod pets;
pub mod quests;
pub mod session;

use crate::db::modern::traits::{InventoryRepository, PetRepository, QuestRepository};
use crate::db::pool::DbPool;

/// Bundles the concrete pool-backed implementations behind the trait objects
/// callers actually depend on. Construct once at boot and hand references to
/// handlers (ticket 07 wires this into the two-tier dispatcher).
#[derive(Clone)]
pub struct SqliteRepositories {
    pub pool: DbPool,
}

impl SqliteRepositories {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn accounts(&self) -> accounts::SqliteAccountRepository<'_> {
        accounts::SqliteAccountRepository { pool: &self.pool }
    }

    pub fn characters(&self) -> characters::SqliteCharacterRepository<'_> {
        characters::SqliteCharacterRepository { pool: &self.pool }
    }

    pub fn inventories(&self) -> impl InventoryRepository + '_ {
        inventories::SqliteInventoryRepository { pool: &self.pool }
    }

    pub fn pets(&self) -> impl PetRepository + '_ {
        pets::SqlitePetRepository { pool: &self.pool }
    }

    pub fn quests(&self) -> impl QuestRepository + '_ {
        quests::SqliteQuestRepository { pool: &self.pool }
    }

    pub fn sessions(&self) -> session::SqliteSessionRepository<'_> {
        session::SqliteSessionRepository { pool: &self.pool }
    }
}
