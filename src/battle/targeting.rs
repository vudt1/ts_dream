//! Targeting pickers for the battle engine (Chapter 6 §6.4).
//!
//! Faithful port of `TheBattle.cs` `GetPosRandom*` / `GetPosAttack*` family
//! (lines 7271-9230). Each picker selects an anchor via its own qualification
//! rule, then expands it by the skill's `SLDanh` into the target list. The
//! expansions are byte-identical across variants; only the anchor rule differs.
//! Terrain (`_Diahinh`) never influences targeting or damage.

/// A grid position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos {
    pub row: u8,
    pub col: u8,
}

impl GridPos {
    pub fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }
}

/// Minimum info about a grid cell needed for target selection.
#[derive(Debug, Clone, Copy)]
pub struct CellInfo {
    pub row: u8,
    pub col: u8,
    pub id: i64,
    pub hp: i64,
    pub team: i64,
    pub type4_id: i64,
}

/// Anchor qualification rules (one per `GetPosRandom*` variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorRule {
    /// Default hostile (`GetPosRandom`): enemy team, hp>0, type4 ∉ {13005,13025,13032}.
    Hostile,
    /// `GetPosRandomTG`: enemy team, hp>0, no type4 exclusion.
    HostileAnyType4,
    /// `GetPosRandomCombo`: requested cell qualifies with `id>0 && enemy` only
    /// (may be dead); the fallback scan uses the full Hostile rule.
    Combo,
    /// `GetPosRandom_Type4` / `GetPosRandom_honLoan`: same team, hp>0.
    Friendly,
    /// `GetPosRandom_GiaiTru`: any entity with id>0 (no team/hp requirement).
    Any,
}

/// `_Type4_Id` exclusion set for the default hostile rules.
pub fn excluded_type4(id: i64) -> bool {
    matches!(id, 13005 | 13025 | 13032)
}

/// Whether `c` qualifies under `rule`. `for_requested` selects the Combo rule's
/// relaxed requested-cell check vs its strict fallback scan.
fn qualifies(c: &CellInfo, myteam: i64, rule: AnchorRule, for_requested: bool) -> bool {
    if c.id <= 0 {
        return false;
    }
    match rule {
        AnchorRule::Hostile => c.hp > 0 && c.team != myteam && !excluded_type4(c.type4_id),
        AnchorRule::HostileAnyType4 => c.hp > 0 && c.team != myteam,
        AnchorRule::Combo => {
            if for_requested {
                c.team != myteam
            } else {
                c.hp > 0 && c.team != myteam && !excluded_type4(c.type4_id)
            }
        }
        AnchorRule::Friendly => c.hp > 0 && c.team == myteam,
        AnchorRule::Any => true,
    }
}

/// Pick the anchor point: the requested cell if it qualifies, else the first
/// qualifying cell in grid (`cells`) order. Returns `NO_TARGET` (99,99) if none.
pub fn pick_anchor(cells: &[CellInfo], myteam: i64, row: u8, col: u8, rule: AnchorRule) -> GridPos {
    if let Some(c) = cells
        .iter()
        .find(|c| c.row == row && c.col == col && qualifies(c, myteam, rule, true))
    {
        return GridPos::new(c.row, c.col);
    }
    for c in cells {
        if qualifies(c, myteam, rule, false) {
            return GridPos::new(c.row, c.col);
        }
    }
    NO_TARGET
}

/// Expand an anchor position into a target list based on `sl_danh` (§4 in research).
///
/// The `alive_at` closure checks whether the cell at (row, col) has hp > 0 and id > 0.
/// Expansion rules mirror `TheBattle.cs` `GetPosAttack` switch:
///   1 = anchor; 2 = +opposite-row; 3 = +left/right; 4 = +left/right (dead→anchor);
///   5 = +left/right+opposite; 6 = +left/right+opposite+opposite-diagonals;
///   7 = all alive cells of the anchor ROW (all columns); 8 = anchor row + opposite row.
pub fn expand_sl_danh<F>(anchor: GridPos, sl_danh: i64, alive_at: F) -> Vec<GridPos>
where
    F: Fn(u8, u8) -> bool,
{
    let mut targets = Vec::new();
    let r = anchor.row;
    let c = anchor.col;

    match sl_danh {
        1 => {
            targets.push(anchor);
        }
        2 => {
            targets.push(anchor);
            let opp = r ^ 1;
            if alive_at(opp, c) {
                targets.push(GridPos::new(opp, c));
            }
        }
        3 => {
            targets.push(anchor);
            if c > 0 && alive_at(r, c - 1) {
                targets.push(GridPos::new(r, c - 1));
            }
            if c < 4 && alive_at(r, c + 1) {
                targets.push(GridPos::new(r, c + 1));
            }
        }
        4 => {
            targets.push(anchor);
            if c > 0 {
                if alive_at(r, c - 1) {
                    targets.push(GridPos::new(r, c - 1));
                } else {
                    targets.push(anchor);
                }
            }
            if c < 4 {
                if alive_at(r, c + 1) {
                    targets.push(GridPos::new(r, c + 1));
                } else {
                    targets.push(anchor);
                }
            }
        }
        5 => {
            targets.push(anchor);
            if c > 0 && alive_at(r, c - 1) {
                targets.push(GridPos::new(r, c - 1));
            }
            if c < 4 && alive_at(r, c + 1) {
                targets.push(GridPos::new(r, c + 1));
            }
            let opp = r ^ 1;
            if alive_at(opp, c) {
                targets.push(GridPos::new(opp, c));
            }
        }
        6 => {
            targets.push(anchor);
            if c > 0 && alive_at(r, c - 1) {
                targets.push(GridPos::new(r, c - 1));
            }
            if c < 4 && alive_at(r, c + 1) {
                targets.push(GridPos::new(r, c + 1));
            }
            let opp = r ^ 1;
            if alive_at(opp, c) {
                targets.push(GridPos::new(opp, c));
            }
            if c > 0 && alive_at(opp, c - 1) {
                targets.push(GridPos::new(opp, c - 1));
            }
            if c < 4 && alive_at(opp, c + 1) {
                targets.push(GridPos::new(opp, c + 1));
            }
        }
        7 => {
            // C# case 7 iterates `Y = 0..4` keeping X = anchor row.
            for col in 0..5u8 {
                if alive_at(r, col) {
                    targets.push(GridPos::new(r, col));
                }
            }
        }
        8 => {
            targets.push(anchor);
            for col in 0..5u8 {
                if col != c && alive_at(r, col) {
                    targets.push(GridPos::new(r, col));
                }
            }
            let opp = r ^ 1;
            for col in 0..5u8 {
                if alive_at(opp, col) {
                    targets.push(GridPos::new(opp, col));
                }
            }
        }
        _ => targets.push(anchor),
    }

    targets
}

/// Sentinel "no target" position.
pub const NO_TARGET: GridPos = GridPos { row: 99, col: 99 };

/// Check if a target is valid (not the sentinel).
pub fn is_valid_target(pos: GridPos) -> bool {
    pos.row < 4
}

/// Column iteration order for anchor selection: 2, 1, 3, 0, 4.
pub const COL_ORDER: [u8; 5] = [2, 1, 3, 0, 4];

/// Run one full `GetPosAttack` picker: anchor selection + SLDanh expansion.
pub fn get_pos_attack(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
    rule: AnchorRule,
) -> Vec<GridPos> {
    let anchor = pick_anchor(cells, myteam, row, col, rule);
    if !is_valid_target(anchor) {
        return Vec::new();
    }
    let alive_at = |r: u8, c: u8| {
        cells
            .iter()
            .any(|x| x.row == r && x.col == c && x.id > 0 && x.hp > 0)
    };
    expand_sl_danh(anchor, sl_danh, alive_at)
}

/// `GetPosAttack` — default hostile targeting.
pub fn get_pos_attack_default(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Hostile)
}

/// `GetPosAttackCombo` — same expansion, combo anchor rule.
pub fn get_pos_attack_combo(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Combo)
}

/// `GetPosAttackTG` — hostile with no type4 exclusion.
pub fn get_pos_attack_tg(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(
        cells,
        myteam,
        row,
        col,
        sl_danh,
        AnchorRule::HostileAnyType4,
    )
}

/// `GetPosAttack3_15` — default hostile rule (same as `get_pos_attack_default`).
pub fn get_pos_attack_3_15(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Hostile)
}

/// `GetPosAttack_GiaiTru` — any-entity targeting (dispel/cleanse).
pub fn get_pos_attack_giai_tru(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Any)
}

/// `GetPosAttack_Type4` — own-team buffs/heals.
pub fn get_pos_attack_type4(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Friendly)
}

/// `GetPosAttack_honLoan` — own-team splash (berserk).
pub fn get_pos_attack_hon_loan(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Friendly)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells_from(rows: &[(u8, u8, i64, i64, i64)]) -> Vec<CellInfo> {
        // (row, col, id, hp, team)
        let mut out = Vec::new();
        for (r, c, id, hp, team) in rows {
            out.push(CellInfo {
                row: *r,
                col: *c,
                id: *id,
                hp: *hp,
                team: *team,
                type4_id: 0,
            });
        }
        out
    }

    #[test]
    fn expand_single_target() {
        let targets = expand_sl_danh(GridPos::new(0, 2), 1, |_, _| true);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], GridPos::new(0, 2));
    }

    #[test]
    fn expand_two_target() {
        let targets = expand_sl_danh(GridPos::new(0, 2), 2, |_, _| true);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[1], GridPos::new(1, 2)); // opposite row
    }

    #[test]
    fn expand_three_target_edges() {
        // Anchor at col=0, only right is alive
        let targets = expand_sl_danh(GridPos::new(0, 0), 3, |r, c| r == 0 && c == 1);
        assert_eq!(targets.len(), 2); // anchor + right
    }

    #[test]
    fn expand_seven_targets_anchor_row() {
        // C# case 7: all alive cells of the anchor ROW (columns 0..4).
        let targets = expand_sl_danh(GridPos::new(0, 2), 7, |r, _| r == 0);
        assert_eq!(targets.len(), 5);
        assert_eq!(targets[0], GridPos::new(0, 0));
        assert_eq!(targets[4], GridPos::new(0, 4));
    }

    #[test]
    fn no_target_sentinel() {
        assert!(!is_valid_target(NO_TARGET));
        assert!(is_valid_target(GridPos::new(0, 0)));
    }

    #[test]
    fn picker_default_hostile() {
        // Enemy at (0,2) alive; requested (0,2).
        let cells = cells_from(&[(0, 2, 9001, 100, 2), (3, 2, 300001, 100, 1)]);
        let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
        assert_eq!(t, vec![GridPos::new(0, 2)]);
    }

    #[test]
    fn picker_default_skips_hidden_type4() {
        // Enemy type4=13005 (frozen) should not be picked; falls back to other enemy.
        let mut cells = cells_from(&[(0, 2, 9001, 100, 2), (0, 4, 9002, 100, 2)]);
        cells[0].type4_id = 13005;
        let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
        assert_eq!(t, vec![GridPos::new(0, 4)]);
    }

    #[test]
    fn picker_friendly_targets_own_team() {
        // Heal/buff picks own team; requested cell is friendly.
        let cells = cells_from(&[
            (0, 2, 9001, 100, 2),
            (3, 2, 300001, 100, 1),
            (3, 1, 300002, 100, 1),
        ]);
        let t = get_pos_attack_type4(&cells, 1, 3, 2, 3);
        // Anchor (3,2) then left (3,1) alive.
        assert_eq!(t, vec![GridPos::new(3, 2), GridPos::new(3, 1)]);
    }

    #[test]
    fn picker_combo_accepts_dead_requested() {
        // Requested enemy is dead (hp 0) but combo rule only needs id>0 + enemy.
        let cells = cells_from(&[(0, 2, 9001, 0, 2), (3, 2, 300001, 100, 1)]);
        let t = get_pos_attack_combo(&cells, 1, 0, 2, 1);
        assert_eq!(t, vec![GridPos::new(0, 2)]);
    }

    #[test]
    fn picker_any_ignores_team() {
        let cells = cells_from(&[(0, 2, 9001, 0, 2), (3, 2, 300001, 100, 1)]);
        let t = get_pos_attack_giai_tru(&cells, 1, 0, 2, 1);
        // Requested (0,2) qualifies (id>0) even though dead.
        assert_eq!(t, vec![GridPos::new(0, 2)]);
    }

    #[test]
    fn no_target_returns_empty() {
        let cells = cells_from(&[(3, 2, 300001, 100, 1)]);
        let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
        assert!(t.is_empty());
    }
}
