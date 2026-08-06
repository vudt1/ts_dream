//! MySQL data-access layer (repositories).
//!
//! Every SQL statement against the `ts_dream` schema lives here, grouped by
//! table so the queries stay centralized and unit-testable. Handlers and the
//! web dashboard call these typed functions instead of writing `sqlx` inline.
//!
//! Write-through helpers take `Option<&MySqlPool>` and no-op when `None` so
//! golden replay (no live DB) shares the same code path; read/transactional
//! functions take `&MySqlPool` and are only invoked from the live-DB branch.

pub mod accounts;
pub mod item_code;
pub mod persist;
pub mod players;
pub mod pool;
