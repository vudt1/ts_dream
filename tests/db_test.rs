//! Consolidated database repository & migration test suite.

mod common;

#[path = "db/repository_init.rs"]
mod repository_init;
#[path = "db/sqlite_migration_0002.rs"]
mod sqlite_migration_0002;
