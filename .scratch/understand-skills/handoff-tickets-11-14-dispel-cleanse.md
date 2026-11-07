# Handoff: Battle Dispel & Cleanse Matrix Parity (Tickets 11 - 14)

**Date:** 2026-11-04  
**Author:** Antigravity  
**Branch:** `ss2`  
**Feature Spec / Handoff Plan:** [handoff-battle-dispel-cleanse-matrix-parity.md](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/handoff-battle-dispel-cleanse-matrix-parity.md)  
**Ticket Tracking:**
- [Ticket 11: Core Dispel Domain & Specific Status Counters](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/11-core-dispel-domain-and-specific-counters.md)
- [Ticket 12: Mass Cleanse Friendly-Only Rule & Buff Preservation](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/12-mass-cleanse-friendly-buff-preservation.md)
- [Ticket 13: Cleanse & Heal Dynamic Scaling & SP Loop Gate](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/13-cleanse-heal-scaling-and-sp-loop-gate.md)
- [Ticket 14: Runner Integration & Legacy Cleanup](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/14-runner-integration-and-legacy-cleanup.md)

---

## 1. Executive Summary

Tickets 11 through 14 have been fully implemented, resolving discrepancies and legacy fragmentation in combat debuff removal, CC cleansing, barrier dispelling, and hybrid cleanse-heal mechanics in `ts_dream`.

All dispel and cleanse logic now lives in a dedicated, decoupled domain module: [`src/battle/dispel.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/dispel.rs), cleanly integrated into [`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs) through a single dispatch point `apply_dispel`.

---

## 2. Key Changes & Architecture

### A. Central Dispel Domain Engine ([`src/battle/dispel.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/dispel.rs))
Exposes the core function:
```rust
pub fn execute_dispel(
    data: &BattleData,
    attacker: &WarInfo,
    target: &mut WarInfo,
    skill_id: i64,
    skill_lv: i64,
    skill_type: i64,
) -> DispelExecutionOutcome
```

#### Dispel Archetypes:
1. **Specific Counter Dispels (Archetype A):**
   - **Băng tan** (`11009`, `11015`): Dispels Freeze `type3_id == 11008 / 11014`, resets Type 3, clears `PURGE_TYPE_DISABLE` + `STATUS_SEAL_RESIST_PCT`, returns `troi_byte::TYPE3`.
   - **Thức tỉnh / Giải hôn** (`11004`, `14007`): Dispels Sleep `type3_id == 11003 / 14008`, resets Type 3, clears disable status, returns `troi_byte::TYPE3`.
   - **Giải độc** (`14016`, `14014`): Dispels Poison `type15_id == 14015`, resets Type 15, clears `PURGE_TYPE_DEBUFF` + ATK/DEF modifiers, returns `troi_byte::TYPE15`.
   - **Giải hỗn loạn** (`14022`, `20014`): Dispels Confusion `type15_id == 14021 / 20014`, resets Type 15, clears debuffs, returns `troi_byte::TYPE15`.
   - **Giải kết giới** (`11014`, `10009`): Dispels hostile Barrier `type4_id == 10010`, returns `troi_byte::TYPE4`.
   - **Giải trừ kính** (`14007`, `10014`): Dispels hostile Mirror `type4_id == 10015`, returns `troi_byte::TYPE4`.
   - *Negative Rule:* Any specific counter cast on a target lacking the matching buff/debuff yields `miss_status::MISS` without packet broadcasts or state mutation.

2. **Mass Cleanses (Archetype B):**
   - Covers `11012` (Trạng thái giải trừ - 1 target), `11025` (3 targets), `11031` (Siêu Trọng Giải Thuật - 6 targets), `22005` (Thần thánh tẩy lễ).
   - **Friendly-Only Rule:** Returns `miss_status::MISS` if cast on an enemy.
   - **Buff Preservation Rule:** Purges Type 3 CC and Type 15 debuffs, while **strictly leaving Type 4 (Shield/Mirror) and Type 19 (Job auras) untouched**.
   - **Clean Ally Rule:** If ally has no Type 3 or Type 15 debuffs, returns `miss_status::MISS`.
   - **Wire Parity:** Emits action fragment with exact count (`1 + purged_type3 + purged_type15`) followed by 4-byte sub-effects (`DD000001` for Type 3, `DF000001` for Type 15) to maintain perfect client alignment in Delphi packet parser `00651e3c_FUN_00651e3c.c`.

3. **Hybrid Cleanse & Heal (Archetype C - Type 18):**
   - Covers Thủy Thần (`11016..=11019`), Cường hiệu giải trừ (`20050`), Tứ nhân trạng giải (`20015`), and Giải thoát (`20019`).
   - Dynamic scaling based on `skill_lv` and `attacker.int1`.
   - **SP Loop Gate:** When self-targeting (`attacker.row == target.row && attacker.col == target.col`), SP restored is forced to `0` to prevent infinite self-sustain loops. Allies in multi-target AOE receive full SP recovery.
   - **Clean Target Recovery:** Allies without active debuffs still receive full HP and SP recovery (`hit = ATTACK`).
   - Generates action frames with subcodes `19` (HP restored) and `1A` (SP restored).

### B. Runner Integration ([`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs))
- Replaced redundant, divergent implementations of `apply_dispel5`, `apply_dispel16`, and `apply_cleanse18` with unified `apply_dispel(...)` delegation to `dispel::execute_dispel`.
- Cleaned up unused legacy variables, comments, and suppressions.
- Maintained exact turn delay and animation timing parity.

### C. Comprehensive Integration Test Suite ([`tests/battle_dispel_cleanse_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_dispel_cleanse_test.rs))
A 12-test comprehensive suite covering:
1. `test_specific_dispel_wake_sleep`
2. `test_specific_dispel_thaw_freeze`
3. `test_specific_dispel_cure_poison_and_confusion` (including advanced confusion `20014`)
4. `test_specific_dispel_hostile_barrier_and_mirror`
5. `test_specific_dispel_negative_cases`
6. `test_mass_cleanse_buff_preservation`
7. `test_mass_cleanse_enemy_and_clean_ally_rejected`
8. `test_multi_target_cleanse_coverage_3_and_6_targets` (isometric coordinates `(3, 2)`, `(3, 1)`, `(2, 4)`, `(3, 4)`, `(2, 2)`, `(2, 1)`)
9. `test_type18_clean_target_heals_hp_and_sp`
10. `test_type18_sp_infinite_loop_gate`
11. `test_type18_skill_level_scaling_progression`
12. `test_type18_enemy_target_rejected`

---

## 3. Verification & Quality Gates

All checks and tests are green:
- `cargo check --all-targets` (0 warnings, 0 errors)
- `cargo test --test battle_dispel_cleanse_test` (12/12 passed)
- `cargo test --test battle_timing_wiring_test` (6/6 passed)
- `cargo test --test battle_buff_debuff_lifecycle_test` (5/5 passed)
- `cargo test --test architecture_guardrails_test` (5/5 passed)

---

## 4. Suggested Skills for Next Agent

- **`code-review`**: To run a final review on the working tree changes against repo standards before commit.
- **`tdd`**: If adding new combat skill mechanics or expanding remaining skill types (e.g. Type 19 job auras, stealth, terrain traps).
- **`reverse-engineer-anything`**: If investigating additional pseudo-C client handlers (`client_pseudo_c/`) for remaining unported main opcodes or combat subroutines.
