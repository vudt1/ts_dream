//! Secondary table lookup index for `Compound.Dat` matching Delphi `FUN_0079A890`.

use crate::data::loaders::compound::CompoundDef;

pub struct CompoundCatalog;

impl CompoundCatalog {
    /// Queries the sorted compound recipe list for matching recipes with `search_key` and `input_deg`.
    ///
    /// Collects up to 10 matching candidate `result_kind`s.
    pub fn find_candidate_recipes(
        recipes: &[CompoundDef],
        search_key: u32,
        input_deg: [u8; 3],
    ) -> Vec<u16> {
        // Binary search to find the first recipe with lookup_key >= search_key
        let start = recipes.partition_point(|r| r.lookup_key < search_key);
        let mut candidates = Vec::with_capacity(10);

        for rec in &recipes[start..] {
            if rec.lookup_key > search_key {
                break; // Early exit matching ASM JA 0x0079B036
            }

            // Check degree match for all 3 slots
            let mut recipe_matched = true;
            for k in 0..3 {
                let mut slot_matched = false;
                if k == 0 && input_deg[0] == 0 {
                    slot_matched = true;
                } else if input_deg[k] >= 1 {
                    for d in (1..=input_deg[k]).rev() {
                        if rec.degrees[k] == d {
                            slot_matched = true;
                            break;
                        }
                    }
                }
                if !slot_matched {
                    recipe_matched = false;
                    break; // Any slot failure rejects the entire recipe candidate
                }
            }

            if recipe_matched {
                candidates.push(rec.result_kind);
                if candidates.len() >= 10 {
                    break;
                }
            }
        }

        candidates
    }
}
