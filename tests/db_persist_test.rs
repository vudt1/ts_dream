//! DB layer tests — migrated from inline #[cfg(test)] blocks (ticket 08).
//!
//! Every test here is pure logic: it never opens a MySQL connection, so the
//! suite runs quickly and cannot hang waiting for a database. Live-DB
//! integration coverage lives in `tests/db_repositories.rs` behind
//! `TS_TEST_DB_URL`.

use ts_dream::db::persist::DELETE_SYSTEM_SKILLS_SQL;
use ts_dream::db::pool::{probe_next_state, strip_database_path};
use ts_dream::state::DbStatus;

// ---------------------------------------------------------------------------
// pool.rs — connection URL handling + liveness-probe decision
// ---------------------------------------------------------------------------

#[test]
fn strip_database_path_removes_path() {
    assert_eq!(
        strip_database_path("mysql://user:pass@localhost:3306/ts_dream"),
        "mysql://user:pass@localhost:3306"
    );
}

#[test]
fn strip_database_path_removes_query_too() {
    assert_eq!(
        strip_database_path("mysql://u:p@localhost/ts_dream?ssl=true"),
        "mysql://u:p@localhost"
    );
}

#[test]
fn strip_database_path_keeps_url_without_path() {
    assert_eq!(
        strip_database_path("mysql://user:pass@localhost:3306"),
        "mysql://user:pass@localhost:3306"
    );
}

#[test]
fn probe_next_state_maps_ping_result() {
    assert_eq!(
        probe_next_state(DbStatus::Connecting, true),
        DbStatus::Connected
    );
    assert_eq!(
        probe_next_state(DbStatus::Connected, false),
        DbStatus::Disconnected
    );
}

// ---------------------------------------------------------------------------
// persist.rs — write-through SQL invariants
// ---------------------------------------------------------------------------

/// Modern skill purge must resolve the account to one character and keep the
/// basic-skill range scoped to that character.
#[test]
fn delete_system_skills_is_player_scoped() {
    assert!(
        DELETE_SYSTEM_SKILLS_SQL.contains("c.account_id = ?"),
        "skill purge must be player-scoped: {DELETE_SYSTEM_SKILLS_SQL}"
    );
    assert!(
        DELETE_SYSTEM_SKILLS_SQL.contains("s.skill_id >= 0 AND s.skill_id <= 9"),
        "must retain the basic-skill Id range predicate: {DELETE_SYSTEM_SKILLS_SQL}"
    );
}
