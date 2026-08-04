//! Targeting pickers for the battle engine (Chapter 6 §6.4).
//!
//! Each picker returns a list of `(row, col)` grid positions that qualify
//! for the given skill. The anchor cell is selected first, then expanded
//! according to `sl_danh` (skill area).

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
pub struct CellInfo {
    pub row: u8,
    pub col: u8,
    pub id: i64,
    pub hp: i64,
    pub team: i64,
    pub type4_id: i64,
}

/// Expand an anchor position into a target list based on `sl_danh` (§4 in research).
///
/// The `alive_at` closure checks whether the cell at (row, col) has hp > 0 and id > 0.
pub fn expand_sl_danh<F>(anchor: GridPos, sl_danh: i64, alive_at: F) -> Vec<GridPos>
where
    F: Fn(u8, u8) -> bool,
{
    let mut targets = vec![anchor];
    let r = anchor.row;
    let c = anchor.col;

    match sl_danh {
        1 => { /* single target, just anchor */ }
        2 => {
            // anchor + opposite-row same col
            let opp = r ^ 1;
            if alive_at(opp, c) {
                targets.push(GridPos::new(opp, c));
            }
        }
        3 => {
            // anchor + left + right
            if c > 0 && alive_at(r, c - 1) {
                targets.push(GridPos::new(r, c - 1));
            }
            if c < 4 && alive_at(r, c + 1) {
                targets.push(GridPos::new(r, c + 1));
            }
        }
        4 => {
            // anchor; (r,c-1): alive→add, else anchor; (r,c+1): alive→add then break, else anchor
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
            // anchor + left + right + opposite
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
            // anchor + left + right + opposite + opposite-left + opposite-right
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
            // column: all rows at the anchor column with alive cells
            for row in 0..4u8 {
                if row != r && alive_at(row, c) {
                    targets.push(GridPos::new(row, c));
                }
            }
        }
        8 => {
            // all cells in anchor row + all cells in opposite row
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
        _ => { /* unknown sl_danh, single target */ }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn expand_column() {
        // sl_danh=7: all rows at col=2
        let targets = expand_sl_danh(GridPos::new(0, 2), 7, |_, _| true);
        // anchor + 3 other rows
        assert_eq!(targets.len(), 4);
    }

    #[test]
    fn no_target_sentinel() {
        assert!(!is_valid_target(NO_TARGET));
        assert!(is_valid_target(GridPos::new(0, 0)));
    }
}
