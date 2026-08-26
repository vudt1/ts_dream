//! Targeting pickers for the battle engine (Chapter 6 §6.4).
//!
//! Each picker selects an anchor via its own qualification
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
    /// Default hostile anchor: enemy team, hp>0, type4 ∉ {13005,13025,13032}.
    Hostile,
    /// Hostile variant: enemy team, hp>0, no type4 exclusion.
    HostileAnyType4,
    /// Combo anchor: requested cell qualifies with `id>0 && enemy` only
    /// (may be dead); the fallback scan uses the full Hostile rule.
    Combo,
    /// Friendly anchor: same team, hp>0.
    Friendly,
    /// Any-entity anchor: any entity with id>0 (no team/hp requirement).
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
/// Expansion rules:
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
            // Area 7 iterates columns 0..4 keeping the anchor row.
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

/// Run one full picker: anchor selection + SLDanh expansion.
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

/// Default hostile targeting.
pub fn get_pos_attack_default(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Hostile)
}

/// Same expansion, combo anchor rule.
pub fn get_pos_attack_combo(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Combo)
}

/// Hostile with no type4 exclusion.
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

/// Default hostile rule (same as `get_pos_attack_default`).
pub fn get_pos_attack_3_15(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Hostile)
}

/// Any-entity targeting (dispel/cleanse).
pub fn get_pos_attack_giai_tru(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Any)
}

/// Own-team buffs/heals.
pub fn get_pos_attack_type4(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Friendly)
}

/// Own-team splash (berserk).
pub fn get_pos_attack_hon_loan(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Friendly)
}
