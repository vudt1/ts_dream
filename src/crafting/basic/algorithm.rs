//! Pure domain implementation of Delphi compound algorithm `FUN_0079A890`.

use crate::crafting::basic::catalog::CompoundCatalog;
use crate::crafting::basic::validator::{validate_compound_inputs, CraftInputSlot};
use crate::crafting::error::CraftError;
use crate::data::loader::GameData;

/// Replicates the Delphi FISTP `@ROUND` instruction (IEEE-754 round half to even).
pub fn delphi_round(val: f64) -> i64 {
    val.round_ties_even() as i64
}

/// Delphi standard LCG random generator (`@RandInt` @ 0x00402CF0).
#[derive(Debug, Clone)]
pub struct DelphiRng {
    pub seed: i32,
}

impl DelphiRng {
    pub fn new(seed: i32) -> Self {
        Self { seed }
    }

    /// Generates a random integer in [0, range - 1].
    pub fn random_int(&mut self, range: i32) -> i32 {
        if range <= 0 {
            return 0;
        }
        let new_seed = (self.seed as u32)
            .wrapping_mul(0x0808_8405)
            .wrapping_add(1);
        self.seed = new_seed as i32;
        let prod = (new_seed as u64) * (range as u32 as u64);
        (prod >> 32) as i32
    }
}

/// Output of a successful compound operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftOutput {
    pub item_id: u16,
    pub qty: u8,
    pub target_rank: u8,
    pub chosen_kind: u16,
}

/// Executes compounding of 3 slots according to Delphi `FUN_0079A890`.
pub fn execute_compound(
    slots: &[CraftInputSlot; 3],
    game_data: &GameData,
    rng: &mut DelphiRng,
) -> Result<CraftOutput, CraftError> {
    // 1. Validate inputs (FUN_0079A5E4)
    let valid_slots = validate_compound_inputs(slots, game_data)?;

    // 2. Extract (compound_material, compound_rank, qty) for all 3 slots
    let mut raw_mat = [0u8; 3];
    let mut raw_rank = [0u8; 3];
    let mut raw_qty = [0u8; 3];

    for (k, slot) in slots.iter().enumerate() {
        if slot.item_id > 0 && slot.qty > 0 {
            if let Some(def) = game_data.item_defs.get(&slot.item_id) {
                raw_mat[k] = def.compound_material;
                raw_rank[k] = def.compound_rank;
                raw_qty[k] = slot.qty;
            }
        }
    }

    // 3. Compute target_rank from average ratB (compound_rank)
    let sum_rank: u32 = raw_rank.iter().map(|&r| r as u32).sum();
    let n = valid_slots.len() as f64;
    let r_avg = (delphi_round(sum_rank as f64 / n) as u8).max(1);

    // Roll target_rank in [1, r_avg + 1], capped at 10
    let u = rng.random_int(r_avg as i32 + 1);
    let mut target_rank = (u + 1) as u8;
    if target_rank > 10 {
        target_rank = 10;
    }

    // 4. Sort (mat, qty) ascending by mat to build search_key
    let (search_key, _sorted_mat, sorted_qty) =
        crate::data::loaders::compound::CompoundDatLoader::compute_key_and_sort(raw_mat, raw_qty);

    // 5. Compute min_qty, out_qty and input_deg[0..3]
    let mut asc_qty = sorted_qty;
    asc_qty.sort();
    let min_qty = asc_qty.iter().copied().find(|&q| q > 0).unwrap_or(1);
    let mut out_qty = min_qty;

    let mut input_deg = [0u8; 3];
    for k in 0..3 {
        let mut d = sorted_qty[k] / min_qty;
        if d > 5 {
            d = 5;
        }
        input_deg[k] = d;
    }

    // 6. Query Compound.Dat secondary table for up to 10 candidates
    let mut recipe_candidates =
        CompoundCatalog::find_candidate_recipes(&game_data.compounds, search_key, input_deg);

    // 7. Fallback when no recipe matches (cand_count == 0)
    if recipe_candidates.is_empty() {
        let pick = rng.random_int(valid_slots.len() as i32) as usize;
        let chosen_slot = valid_slots[pick].1;
        let chosen_item = game_data
            .item_defs
            .get(&chosen_slot.item_id)
            .ok_or(CraftError::InvalidItemForCompound)?;

        recipe_candidates.push(chosen_item.kind as u16);
        target_rank = 1; // Fallback forces target rank to 1
    }

    // 8. Choose 1 result_kind from recipe candidates
    let cand_count = recipe_candidates.len();
    let r1 = rng.random_int(100) as u8;
    let step1 = delphi_round(100.0 / (cand_count as f64)).max(1) as u8;
    let mut idx1 = (r1 / step1) as usize;
    if idx1 >= cand_count {
        idx1 = cand_count - 1;
    }
    let chosen_kind = recipe_candidates[idx1];

    // Check prohibited result kind {20, 21, 22}
    if (20..=22).contains(&chosen_kind) {
        return Err(CraftError::ForbiddenResultKind(chosen_kind));
    }

    // Weapons / Armors (kind 1..=14) force out_qty = 1
    if (1..=14).contains(&chosen_kind) {
        out_qty = 1;
    }

    // 9. Scan Item.dat for items with (kind == chosen_kind && compound_rank == target_rank)
    let mut item_candidates: Vec<u16> = game_data
        .item_defs
        .values()
        .filter(|item| item.kind as u16 == chosen_kind && item.compound_rank == target_rank)
        .map(|item| item.id)
        .collect();

    // Sort item IDs ascending to match sequential scan of Item.dat
    item_candidates.sort();
    item_candidates.truncate(10);

    if item_candidates.is_empty() {
        return Err(CraftError::NoMatchingItem);
    }

    // 10. Choose 1 item from candidates
    let item_cand_count = item_candidates.len();
    let r2 = rng.random_int(100) as u8;
    let step2 = delphi_round(100.0 / (item_cand_count as f64)).max(1) as u8;
    let mut idx2 = (r2 / step2) as usize;
    if idx2 >= item_cand_count {
        idx2 = item_cand_count - 1;
    }
    let item_id = item_candidates[idx2];

    Ok(CraftOutput {
        item_id,
        qty: out_qty,
        target_rank,
        chosen_kind,
    })
}
