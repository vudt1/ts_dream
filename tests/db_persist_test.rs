//! DB layer tests — migrated from inline #[cfg(test)] blocks (ticket 08).
//!
//! Every test here is pure logic: it never opens a MySQL connection, so the
//! suite runs quickly and cannot hang waiting for a database. Live-DB
//! integration coverage lives in `tests/db_repositories.rs` behind
//! `TS_TEST_DB_URL`.

use ts_dream::db::persist::DELETE_SYSTEM_SKILLS_SQL;
use ts_dream::db::players::{item_table, starter_rows, StarterRow};
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
// players.rs — starter-inventory template
// ---------------------------------------------------------------------------

#[test]
fn starter_rows_match_newchar_template() {
    // The two behavioral rows of the new-character inventory template.
    assert_eq!(
        starter_rows(),
        vec![
            StarterRow {
                table: "homdo",
                slot: 1,
                id: 32012,
                count: 4,
                agi1: 0,
                loai: 0
            },
            StarterRow {
                table: "trangbi",
                slot: 2,
                id: 19737,
                count: 1,
                agi1: 1,
                loai: 2
            },
        ]
    );
}

#[test]
fn starter_tables_are_whitelisted_for_insert() {
    for row in starter_rows() {
        assert!(
            item_table(row.table).is_some(),
            "{} not whitelisted",
            row.table
        );
    }
    assert!(item_table("tientrang").is_none());
}

// ---------------------------------------------------------------------------
// persist.rs — write-through SQL invariants
// ---------------------------------------------------------------------------

/// §5.4 note 2: every DELETE over the 9 gameplay tables must carry a
/// `player_id` predicate — the unscoped predicate (`WHERE Id >= 0 AND Id <= 9`)
/// would clear every player's basic skills in the shared schema.
#[test]
fn delete_system_skills_is_player_scoped() {
    assert!(
        DELETE_SYSTEM_SKILLS_SQL.contains("player_id = ?"),
        "skill purge must be player-scoped: {DELETE_SYSTEM_SKILLS_SQL}"
    );
    assert!(
        DELETE_SYSTEM_SKILLS_SQL.contains("Id >= 0 AND Id <= 9"),
        "must retain the basic-skill Id range predicate: {DELETE_SYSTEM_SKILLS_SQL}"
    );
}
