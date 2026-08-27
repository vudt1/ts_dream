//! MySQL implementations of the modern repository traits.
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

/// Bundles the concrete pool-backed implementations behind the trait objects
/// callers actually depend on. Construct once at boot and hand references to
/// handlers (ticket 07 wires this into the two-tier dispatcher).
#[derive(Clone)]
pub struct MySqlRepositories {
    pub pool: sqlx::MySqlPool,
}

impl MySqlRepositories {
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self { pool }
    }

    pub fn accounts(&self) -> accounts::MySqlAccountRepository<'_> {
        accounts::MySqlAccountRepository { pool: &self.pool }
    }

    pub fn characters(&self) -> characters::MySqlCharacterRepository<'_> {
        characters::MySqlCharacterRepository { pool: &self.pool }
    }

    pub fn inventories(&self) -> impl InventoryRepository + '_ {
        inventories::MySqlInventoryRepository { pool: &self.pool }
    }

    pub fn pets(&self) -> impl PetRepository + '_ {
        pets::MySqlPetRepository { pool: &self.pool }
    }

    pub fn quests(&self) -> impl QuestRepository + '_ {
        quests::MySqlQuestRepository { pool: &self.pool }
    }

    pub fn sessions(&self) -> session::MySqlSessionRepository<'_> {
        session::MySqlSessionRepository { pool: &self.pool }
    }
}
