//! Targeting primitives for the 4x5 battle grid.

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

/// Mobile `BattleGrid.resolveTargets` using fightArea values 1–8.
///
/// For single/column/row/jump/cross/six/row-five, the positions are expanded
/// exactly as the Kotlin grid helper. Area 8 is the living opposing side, and
/// invalid or empty positions are omitted. `is_heal` makes area 8 select the
/// attacker's own side, matching mobile heal targeting.
pub fn get_pos_attack_mobile(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    fight_area: u8,
    is_heal: bool,
) -> Vec<GridPos> {
    let alive_at = |r: i16, c: i16| {
        cells
            .iter()
            .any(|cell| cell.row == r as u8 && cell.col == c as u8 && cell.id > 0 && cell.hp > 0)
    };
    let push_if_alive = |targets: &mut Vec<GridPos>, r: i16, c: i16| {
        if (0..4).contains(&r) && (0..5).contains(&c) && alive_at(r, c) {
            targets.push(GridPos::new(r as u8, c as u8));
        }
    };

    match fight_area {
        1 => {
            let mut targets = Vec::new();
            push_if_alive(&mut targets, row as i16, col as i16);
            targets
        }
        2 => {
            let mut targets = Vec::new();
            push_if_alive(&mut targets, row as i16, col as i16);
            push_if_alive(&mut targets, row as i16 - 1, col as i16);
            if targets.len() < 2 {
                push_if_alive(&mut targets, row as i16 + 1, col as i16);
            }
            targets
        }
        3 => {
            let mut targets = Vec::new();
            for cell in cells
                .iter()
                .filter(|cell| cell.row == row && cell.id > 0 && cell.hp > 0 && cell.team != myteam)
            {
                targets.push(GridPos::new(cell.row, cell.col));
            }
            targets.into_iter().take(3).collect()
        }
        4 => {
            let mut targets = Vec::new();
            push_if_alive(&mut targets, row as i16, col as i16);
            push_if_alive(&mut targets, row as i16 - 2, col as i16);
            push_if_alive(&mut targets, row as i16 + 2, col as i16);
            targets
        }
        5 => {
            let mut targets = Vec::new();
            push_if_alive(&mut targets, row as i16, col as i16);
            push_if_alive(&mut targets, row as i16 - 1, col as i16);
            push_if_alive(&mut targets, row as i16 + 1, col as i16);
            push_if_alive(&mut targets, row as i16, col as i16 - 1);
            push_if_alive(&mut targets, row as i16, col as i16 + 1);
            targets
        }
        6 => {
            let mut targets = Vec::new();
            for r in 0..4 {
                push_if_alive(&mut targets, r, col as i16);
            }
            targets
        }
        7 => {
            let mut targets = Vec::new();
            for c in 0..5 {
                push_if_alive(&mut targets, row as i16, c);
            }
            targets
        }
        8 => {
            let wanted_team = if is_heal { myteam } else { -myteam };
            cells
                .iter()
                .filter(|cell| {
                    cell.id > 0
                        && cell.hp > 0
                        && if is_heal {
                            cell.team == wanted_team
                        } else {
                            cell.team != myteam && cell.team != wanted_team
                        }
                })
                .map(|cell| GridPos::new(cell.row, cell.col))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Anchor qualification rules for the original PC targeting path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorRule {
    Hostile,
    HostileAnyType4,
    Combo,
    Friendly,
    Any,
}

pub fn excluded_type4(id: i64) -> bool {
    matches!(id, 13005 | 13025 | 13032)
}

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

pub fn expand_sl_danh<F>(anchor: GridPos, sl_danh: i64, alive_at: F) -> Vec<GridPos>
where
    F: Fn(u8, u8) -> bool,
{
    let mut targets = Vec::new();
    let r = anchor.row;
    let c = anchor.col;
    match sl_danh {
        1 => targets.push(anchor),
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
                targets.push(if alive_at(r, c - 1) {
                    GridPos::new(r, c - 1)
                } else {
                    anchor
                });
            }
            if c < 4 {
                targets.push(if alive_at(r, c + 1) {
                    GridPos::new(r, c + 1)
                } else {
                    anchor
                });
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

pub const NO_TARGET: GridPos = GridPos { row: 99, col: 99 };

pub fn is_valid_target(pos: GridPos) -> bool {
    pos.row < 4
}

pub const COL_ORDER: [u8; 5] = [2, 1, 3, 0, 4];

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

pub fn get_pos_attack_default(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Hostile)
}

pub fn get_pos_attack_combo(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Combo)
}

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

pub fn get_pos_attack_3_15(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack_default(cells, myteam, row, col, sl_danh)
}

pub fn get_pos_attack_type4(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Friendly)
}

pub fn get_pos_attack_hon_loan(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack_default(cells, myteam, row, col, sl_danh)
}

pub fn get_pos_attack_giai_tru(
    cells: &[CellInfo],
    myteam: i64,
    row: u8,
    col: u8,
    sl_danh: i64,
) -> Vec<GridPos> {
    get_pos_attack(cells, myteam, row, col, sl_danh, AnchorRule::Any)
}
