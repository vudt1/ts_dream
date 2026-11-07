# Handoff: Specific Counter Tier Variants, Cleanse-Heal Gate Parity & Packet Constants (Tickets 15 - 17)

**Date:** 2026-11-04  
**Author:** Antigravity  
**Branch:** `ss2`  
**Ticket Tracking:**
- [Ticket 15: Specific Counter Tier Variants, Rule Boundaries & Wire MISS Parity](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/15-specific-counter-tier-variants-and-miss-parity.md) (Status: `closed`)
- [Ticket 16: Cleanse & Heal Coordinate Gate & Advanced Debuff 20014 Resolution](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/16-cleanse-heal-coordinate-gate-and-advanced-debuff-20014.md) (Status: `closed`)
- [Ticket 17: Packet Constants, Code Smells Refactoring & Handoff](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/17-packet-constants-refactoring-and-handoff.md) (Status: `closed`)

---

## 1. Executive Summary

Tickets 15 through 17 complete the battle dispel and cleanse matrix parity in `ts_dream`. Specifically:
1. Missing tier 1 variants for specific counters (Băng tan 1 `11009` curing Freeze 1 `11008`, and Thức tỉnh 1 `11004` waking Sleep 1 `11003`) are implemented.
2. Strict counter boundaries are established: Giải hỗn loạn (`14022`) strictly cures standard Confusion (`14021`) and rejects advanced confusion (`20014`) with a MISS, while Tứ nhân trạng giải (`20015`) properly cures advanced confusion (`20014`).
3. The legacy test alias `11014` for `SKILL_GIAI_KET_GIOI` (`10009`) has been purged from production domain logic, and legacy test call-sites in `tests/battle_timing_wiring_test.rs` updated.
4. Specific dispel MISS wire parity bug fixed: when a specific dispel action misses, `troi_byte::MISS` (`0x00`) is broadcast instead of the hardcoded `troi_byte::TYPE4`.
5. SP Infinite Loop Gate fixed to evaluate board grid coordinates (`attacker.row == target.row && attacker.col == target.col`) rather than entity template ID, preventing same-template ally NPCs in PvE from being incorrectly denied SP recovery.
6. Magic hex string literals (`DF000001`, `DD000001`, and action subcodes `0x19`, `0x1A`) replaced with domain constants and encoder helpers in `src/battle/packets.rs`.
7. Shared domain purge helpers extracted (`purge_type3`, `purge_type15`, `purge_type4`) in `src/battle/dispel.rs`, eliminating code duplication.
8. Attacker/target self-target state synchronization in `runner.rs` cleaned up to prevent stale read / duplicate write anomalies.
9. Full project test suite gate passed (`cargo test --all-targets --no-fail-fast`) with 100% green verification (0 failures, 0 warnings).

---

## 2. Key Changes by Component

### A. Protocol / Packets Layer ([`src/battle/packets.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/packets.rs))
- Added `pub mod action_subcode`:
  - `action_subcode::HP = 0x19`
  - `action_subcode::SP = 0x1A`
- Added `pub mod sub_effect`:
  - `sub_effect::PURGE_TYPE3 = "DD000001"` (Type 3 disable purge sub-effect: troi 0xDD, 0 damage, buff 1)
  - `sub_effect::PURGE_TYPE15 = "DF000001"` (Type 15 debuff purge sub-effect: troi 0xDF, 0 damage, buff 1)
- Added helper functions:
  - `sub_action_hp(hp: u16) -> String`: encodes `19` + LE16(hp) + `00`.
  - `sub_action_sp(sp: u16) -> String`: encodes `1A` + LE16(sp) + `00`.

### B. Dispel Domain Engine ([`src/battle/dispel.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/dispel.rs))
- Added specific counter constants:
  - `SKILL_THUC_TINH_1: i64 = 11004`
  - `SKILL_BANG_TAN_1: i64 = 11009`
- Updated `is_specific_counter`:
  - Removed `| 11014` legacy alias.
  - Added `SKILL_THUC_TINH_1` and `SKILL_BANG_TAN_1`.
- Extracted shared purge helper functions:
  - `purge_type3(target: &mut WarInfo, outcome: &mut DispelExecutionOutcome)`: resets Type 3 state to 0 and enqueues `PURGE_TYPE_DISABLE` + `STATUS_SEAL_RESIST_PCT`.
  - `purge_type15(target: &mut WarInfo, outcome: &mut DispelExecutionOutcome)`: resets Type 15 state to 0 and enqueues `PURGE_TYPE_DEBUFF` + ATK/DEF modifiers HUD clear.
  - `purge_type4(target: &mut WarInfo, outcome: &mut DispelExecutionOutcome)`: resets Type 4 state to 0 and enqueues `PURGE_TYPE_BUFF` + DEF modifier HUD clear.
- Replaced repeated purge boilerplate across `apply_specific_dispel`, `apply_mass_cleanse`, and `apply_cleanse_and_heal`.
- Enforced strict boundary on `SKILL_GIAI_HON_LOAN`: only cures `type15_id == 14021`. Targets with advanced confusion `20014` are rejected with `miss_status::MISS`.
- Removed `| 11014` alias from `SKILL_GIAI_KET_GIOI` match branch.
- Fixed `build_action_fragment`: when `outcome.hit == false`, emits `(miss_status::MISS, troi_byte::MISS)`.
- Fixed SP Infinite Loop Gate in `apply_cleanse_and_heal`:
  - Evaluates `if attacker.row == target.row && attacker.col == target.col { raw_sp = 0; }`.
- Replaced string formatting and magic constants with `packets::sub_effect` and `packets::sub_action_hp / sub_action_sp`.
- Removed unused `crate::protocol::encoder` import.

### C. Battle Runner Integration ([`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs))
- Fixed self-target state initialization in the turn targets loop:
  - `let mut target = if tpos.row == row && tpos.col == col { attacker.clone() } else { self.list_war[...].clone() };`
- Fixed post-action state synchronization:
  - If self-targeting (`tpos.row == row && tpos.col == col`), `attacker = target.clone();` synchronizes the mutated target back to attacker *before* evaluating attacker state drops (e.g. Type 4 `13005` drop, shield `20006` drop).
  - Only writes the cell once (`self.write_cell(&mut attacker, (row, col))`) when self-targeting.
- Updated `apply_sp_restore` to check `attacker.row == target.row && attacker.col == target.col` for coordinate-based self SP gate.

### D. Test Updates & Additions
- [`tests/battle_timing_wiring_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_timing_wiring_test.rs):
  - Updated legacy test call-site from `11014` to `ts_dream::battle::dispel::SKILL_GIAI_KET_GIOI` (`10009`).
- [`tests/battle_dispel_cleanse_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_dispel_cleanse_test.rs):
  - Updated `test_specific_dispel_negative_cases` to assert `troi_byte::MISS` wire parity.
  - Updated `test_specific_dispel_cure_poison_and_confusion` Test 3: verifies that `14022` on `20014` results in `miss_status::MISS` with 0 purge frames and leaves debuff untouched.
  - Updated `test_type18_skill_level_scaling_progression` with distinct grid coordinates for caster and ally units.
  - Added `test_specific_dispel_tier_variants_11009_and_11004`: verifies `11009` curing `11008` and `11004` waking `11003` with exact purge and action frames.
  - Added `test_cleanse_heal_same_id_ally_sp_restoration_and_skill_20015`: verifies that same-template NPC allies in AOE receive full SP while self receives 0 SP, and skill `20015` cures advanced confusion `20014`.

---

## 3. Verification Record

- **Test Suite Command:** `cargo test --all-targets --no-fail-fast`
- **Result:** 100% Passed. 0 failures, 0 warnings.
- **Affected Test Files:**
  - `tests/battle_dispel_cleanse_test.rs`: 14 passed; 0 failed
  - `tests/battle_timing_wiring_test.rs`: 6 passed; 0 failed
  - `tests/battle_buff_debuff_lifecycle_test.rs`: 5 passed; 0 failed
  - `tests/architecture_guardrails_test.rs`: 5 passed; 0 failed
  - All other tests across database, networking, codecs, and game data: 100% green.

---

## 4. Next Steps for Downstream Agents

- **Commit Working Tree:** The changes in working tree are verified and ready for commit per repository conventions.
- **Next Features:**
  - Battle turn lifecycle & combat formula edge cases (ADR 0006).
  - Terrain traps, stealth, and Type 19 job aura integrations.
