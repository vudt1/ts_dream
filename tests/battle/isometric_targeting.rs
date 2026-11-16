//! Integration test for Ticket 22: Isometric Grid AOE Targeting.
//!
//! Validates that `expand_sl_danh` and `get_pos_attack_by_fight_area`
//! strictly follow the Delphi client Isometric diagonal sequence:
//! `0 <-> 1 <-> 2 <-> 4 <-> 3` (where slot 2 is the center).

use ts_dream::battle::targeting::{
    expand_sl_danh, get_pos_attack_by_fight_area, CellInfo, GridPos,
};

#[test]
fn test_expand_sl_danh_row3_center_2_yields_1_2_4() {
    let alive = |_, _| true;
    let targets = expand_sl_danh(GridPos::new(0, 2), 3, alive);
    assert_eq!(
        targets,
        vec![GridPos::new(0, 2), GridPos::new(0, 1), GridPos::new(0, 4)]
    );
}

#[test]
fn test_expand_sl_danh_row3_all_columns_match_isometric_geometry() {
    let alive = |_, _| true;

    // Slot 0 (top-right end of diagonal): only neighbor is slot 1
    assert_eq!(
        expand_sl_danh(GridPos::new(1, 0), 3, alive),
        vec![GridPos::new(1, 0), GridPos::new(1, 1)]
    );

    // Slot 1 (between 0 and 2): neighbors are 0 and 2
    assert_eq!(
        expand_sl_danh(GridPos::new(1, 1), 3, alive),
        vec![GridPos::new(1, 1), GridPos::new(1, 0), GridPos::new(1, 2)]
    );

    // Slot 2 (center): neighbors are 1 and 4
    assert_eq!(
        expand_sl_danh(GridPos::new(1, 2), 3, alive),
        vec![GridPos::new(1, 2), GridPos::new(1, 1), GridPos::new(1, 4)]
    );

    // Slot 4 (between 2 and 3): neighbors are 2 and 3
    assert_eq!(
        expand_sl_danh(GridPos::new(1, 4), 3, alive),
        vec![GridPos::new(1, 4), GridPos::new(1, 2), GridPos::new(1, 3)]
    );

    // Slot 3 (bottom-left end of diagonal): only neighbor is slot 4
    assert_eq!(
        expand_sl_danh(GridPos::new(1, 3), 3, alive),
        vec![GridPos::new(1, 3), GridPos::new(1, 4)]
    );
}

#[test]
fn test_expand_sl_danh_front_back_sl2() {
    let alive = |_, _| true;
    // Row 0's front/back partner is Row 1 (0 ^ 1)
    assert_eq!(
        expand_sl_danh(GridPos::new(0, 2), 2, alive),
        vec![GridPos::new(0, 2), GridPos::new(1, 2)]
    );
    // Row 3's front/back partner is Row 2 (3 ^ 1)
    assert_eq!(
        expand_sl_danh(GridPos::new(3, 4), 2, alive),
        vec![GridPos::new(3, 4), GridPos::new(2, 4)]
    );
}

#[test]
fn test_expand_sl_danh_cross_sl5() {
    let alive = |_, _| true;
    // Center 2: anchor (0, 2), neighbors (0, 1), (0, 4), and opp cell (1, 2)
    assert_eq!(
        expand_sl_danh(GridPos::new(0, 2), 5, alive),
        vec![
            GridPos::new(0, 2),
            GridPos::new(0, 1),
            GridPos::new(0, 4),
            GridPos::new(1, 2),
        ]
    );
}

#[test]
fn test_expand_sl_danh_two_by_three_sl6() {
    let alive = |_, _| true;
    // Center 2: row 0 gets (0, 2), (0, 1), (0, 4); opp row 1 gets (1, 2), (1, 1), (1, 4)
    assert_eq!(
        expand_sl_danh(GridPos::new(0, 2), 6, alive),
        vec![
            GridPos::new(0, 2),
            GridPos::new(0, 1),
            GridPos::new(0, 4),
            GridPos::new(1, 2),
            GridPos::new(1, 1),
            GridPos::new(1, 4),
        ]
    );
}

#[test]
fn test_expand_sl_danh_omits_dead_cells() {
    // Only slots 2 and 1 are alive, slot 4 is dead
    let alive = |r, c| !(r == 0 && c == 4);
    assert_eq!(
        expand_sl_danh(GridPos::new(0, 2), 3, alive),
        vec![GridPos::new(0, 2), GridPos::new(0, 1)]
    );
}

#[test]
fn test_fight_area_by_isometric_grid() {
    let mut cells = Vec::new();
    // Team 2 (enemy) on row 0 and 1
    for r in 0..2u8 {
        for c in 0..5u8 {
            cells.push(CellInfo {
                row: r,
                col: c,
                id: 1000 + (r * 5 + c) as i64,
                hp: 100,
                team: 2,
                type4_id: 0,
            });
        }
    }
    // Team 1 (player) on row 2 and 3
    for r in 2..4u8 {
        for c in 0..5u8 {
            cells.push(CellInfo {
                row: r,
                col: c,
                id: 2000 + (r * 5 + c) as i64,
                hp: 100,
                team: 1,
                type4_id: 0,
            });
        }
    }

    // fight_area = 3, targeting enemy (row 0, col 2)
    let targets = get_pos_attack_by_fight_area(&cells, 1, 0, 2, 3, false);
    assert_eq!(
        targets,
        vec![GridPos::new(0, 2), GridPos::new(0, 1), GridPos::new(0, 4)]
    );

    // fight_area = 5 (Cross), targeting enemy (row 0, col 2)
    let targets_cross = get_pos_attack_by_fight_area(&cells, 1, 0, 2, 5, false);
    assert_eq!(
        targets_cross,
        vec![
            GridPos::new(0, 2),
            GridPos::new(0, 1),
            GridPos::new(0, 4),
            GridPos::new(1, 2)
        ]
    );

    // fight_area = 6 (2 rows x 3 cols), targeting enemy (row 0, col 2)
    let targets_six = get_pos_attack_by_fight_area(&cells, 1, 0, 2, 6, false);
    assert_eq!(
        targets_six,
        vec![
            GridPos::new(0, 2),
            GridPos::new(0, 1),
            GridPos::new(0, 4),
            GridPos::new(1, 2),
            GridPos::new(1, 1),
            GridPos::new(1, 4),
        ]
    );

    // fight_area = 7 (Full Row 5)
    let targets_row = get_pos_attack_by_fight_area(&cells, 1, 0, 2, 7, false);
    assert_eq!(targets_row.len(), 5);
    for c in 0..5u8 {
        assert!(targets_row.contains(&GridPos::new(0, c)));
    }

    // fight_area = 8 (Full Team, attack enemy side 2)
    let targets_team = get_pos_attack_by_fight_area(&cells, 1, 0, 2, 8, false);
    assert_eq!(targets_team.len(), 10);
    for t in &targets_team {
        assert!(t.row < 2);
    }

    // fight_area = 8 with is_heal = true (Full Team, heal own side 1)
    let heal_team = get_pos_attack_by_fight_area(&cells, 1, 2, 2, 8, true);
    assert_eq!(heal_team.len(), 10);
    for t in &heal_team {
        assert!(t.row >= 2);
    }
}
