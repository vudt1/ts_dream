//! Ticket 08 — the one team-from-row rule (`row < 2` ⇒ side 2, else side 1).
//!
//! The client derives the battle side as `grp < 2` ⇒ side 2 and `grp >= 2`
//! ⇒ side 1 (`aLogin.exe`, `HANDOFF-client-reverse.md §0.2`). The server used
//! to compute the team with `row == 0`, which mislabels every row-1 unit.
//!
//! `team_for_row` is the single shared helper consumed by the three grid
//! builders (`add_player`, `load_leader_pets`, `load_member_pet`).

use ts_dream::battle::construction::Battle;
use ts_dream::battle::engine::team_for_row;
use ts_dream::server::session::Session;

/// ---- Pure logic: the row → team mapping --------------------------------

#[test]
fn team_for_row_matches_client_grp_rule() {
    // Enemy side (grp < 2 in the client) ⇒ team 2.
    assert_eq!(team_for_row(0), 2);
    assert_eq!(team_for_row(1), 2);
    // Player side (grp >= 2) ⇒ team 1.
    assert_eq!(team_for_row(2), 1);
    assert_eq!(team_for_row(3), 1);
}

#[test]
fn row_one_is_not_treated_as_team_one() {
    // Regression for the old `row == 0` expression, which returned 1 for row 1.
    assert_ne!(team_for_row(1), 1);
    assert_eq!(team_for_row(1), team_for_row(0));
}

/// ---- Battle: `add_player` stamps the shared helper's team ---------------

#[test]
fn add_player_row_zero_is_team_two() {
    let mut battle = Battle::new(1, 0);
    let session = Session::new();
    battle.add_player(&session, 0, 0, 2);
    assert_eq!(battle.cell(0, 2).unwrap().team, 2);
}

#[test]
fn add_player_row_one_is_team_two() {
    let mut battle = Battle::new(1, 0);
    let session = Session::new();
    battle.add_player(&session, 0, 1, 2);
    assert_eq!(battle.cell(1, 2).unwrap().team, 2);
}

#[test]
fn add_player_player_rows_two_and_three_are_team_one() {
    let mut battle = Battle::new(1, 0);
    let session = Session::new();

    battle.add_player(&session, 0, 2, 2);
    assert_eq!(battle.cell(2, 2).unwrap().team, 1);

    battle.add_player(&session, 0, 3, 2);
    assert_eq!(battle.cell(3, 2).unwrap().team, 1);
}
