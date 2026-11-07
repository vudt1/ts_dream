# Spec: Battle Dispel & Cleanse Matrix Parity

*   **Feature Slug:** `battle-dispel-cleanse-matrix-parity`
*   **Author:** tuan.vu
*   **Status:** `ready-for-agent`
*   **Date:** 2026-11-04
*   **Target File:** `.scratch/understand-skills/spec-battle-dispel-cleanse-matrix-parity.md`
*   **Domain Glossary Terms:** Dispel Matrix, SP Infinite Loop Gate, Bounded Context Battle Engine, WarInfo, Turn Action Finish Frame.
*   **Architecture Reference:** [`docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md)

--------------------------------------------------------------------------------

## Problem Statement

In TS Online combat, tactical counterplay hinges on status effect removal: players rely on single-target counters (such as Băng tan/Melt to thaw Freeze, Giải độc/Antidote to cure Poison, Giải hôn/Wake to break Sleep, and Giải kết giới to shatter enemy Barrier), mass team cleanses (Trạng thái giải trừ / Trọng giải thuật), and dual cleanse-and-heal master skills (Thủy thần, Cường hiệu giải trừ, Tứ nhân trạng giải, Giải thoát).

The current battle engine suffers from severe architectural defects and data-mapping bugs in status removal:

1.  **Corrupted Specific Dispel Routing & Cross-Channel Hallucinations**:
    *   **Băng tan (`11015`)**: Completely omitted from the match dispatch table. Casting Băng tan falls through to the wildcard handler, which blindly wipes all four status channels (Type 3 CC, Type 4 Barrier, Type 15 DOT/Curse, and Type 19 Aura) on the target like a master wipeout. Furthermore, the legacy code accidentally matches `11014` (the Freeze debuff itself) and checks `target.type4_id == 10010` (Barrier), confusing a defensive buff channel with an immobilization debuff.
    *   **Giải hôn (`14007`)**: Evaluates `target.type4_id == 14008`. Because Hôn mê (Sleep) is an immobilization debuff stored in `type3_id`, this condition is mathematically impossible to satisfy. Casting Giải hôn always returns `miss_status::MISS`, rendering Sleep uncureable by its specific counter.
    *   **Giải độc (`14014`)**: Evaluates `target.type4_id == 14015`. Because Bỏ độc (Poison) is a curse/DOT stored in `type15_id`, this check fails 100% of the time, returning `miss_status::MISS`.
    *   **Giải hỗn loạn (`14022`)**: Evaluates `target.type4_id == 10021`. Hỗn loạn (Confusion) has skill ID `14021` and resides in `type15_id`. This double error (wrong skill ID and wrong channel) ensures Confusion can never be dispelled by Giải hỗn loạn.

2.  **Friendly Buff Stripping by Cleanse Skills (Trạng Thái Giải Trừ)**:
    *   Mass cleanse skills (`11012` Trạng thái giải trừ, `11025` Tiên Giai Trọng Giải, `11031` Siêu Trọng Giải Thuật, `22005` Quang du thuật) hit the wildcard branch of the legacy handler. This branch resets `type3`, `type4`, `type15`, and `type19` without checking target alignment.
    *   When a player casts Trọng giải to rescue a frozen or poisoned teammate, the engine inadvertently strips the teammate's active Kính (Mirror), Kết giới (Barrier), and Job Specialization Auras (such as Thiên thần thuẫn / Celestial Shield and Bá ý).

3.  **Static Level-Suppression and Infinite SP Loop in Dual Cleanse-and-Heal (Type 18)**:
    *   Dual skills (`20050` Cường hiệu giải trừ, `20015` Tứ nhân trạng giải, `20019` Giải thoát) belong to Type 18, which combines debuff cleansing with HP and SP restoration.
    *   In the legacy runner, `skill_lv` is suppressed with `let _ = skill_lv;`. The amount of HP and SP restored is clamped to a static `int1 * 0.5`, completely ignoring skill progression from Level 1 to Level 5 or 10.
    *   Furthermore, there is no SP loop guard when the caster targets themselves, creating an exploit where casters regenerate more SP than they spend.

4.  **Architectural Fragmentation Across Three Handlers**:
    *   Dispel logic is fragmented across three separate runner functions (`apply_dispel5`, `apply_dispel16`, `apply_cleanse18`), duplicating packet assembly, frame purging, and target validation with inconsistent rules.

--------------------------------------------------------------------------------

## Solution

1.  **Unified Dispel & Cleanse Engine**:
    Create a dedicated battle domain module (`ts_dream::battle::dispel`) establishing a deterministic **Dispel Matrix** that cleanly separates three functional archetypes:
    *   **Archetype A: Specific Dispel (Type 5 / Type 16)**: 1-to-1 exact counter skills that cure exactly one targeted status effect.
    *   **Archetype B: Mass Cleanse (Type 5 Trọng Giải)**: Friendly-only debuff cleansing that strips all negative effects (Type 3 CC and Type 15 DOT/Curse) while strictly preserving friendly protective barriers (Type 4) and job auras (Type 19).
    *   **Archetype C: Cleanse & Heal (Type 18)**: Team debuff cleansing combined with dynamic HP and SP recovery scaling with caster Intelligence and skill level, protected by an SP Infinite Loop Gate.

2.  **Strict Channel and Target Alignment**:
    *   Enforce that specific dispels verify the correct status channel (`type3_id`, `type4_id`, or `type15_id`) matching canonical skill IDs.
    *   Enforce that Trọng giải skills are strictly Friendly-Only (`attacker.team == target.team`). Casting on an enemy returns `miss_status::MISS`.
    *   Enforce that hostile barrier/mirror dispels (`10009` Giải kết giới, `10014` Giải trừ kính) only remove hostile Type 4 buffs.

3.  **Dynamic Skill Level Progression & SP Gate for Type 18**:
    *   Scale HP and SP recovery for Type 18 skills based on caster Intelligence (`int1`) and active skill level (`skill_lv`).
    *   Enforce the SP Infinite Loop Gate: if the caster targets themselves or falls within the multi-target AOE, the caster's SP restore amount is forced to 0 (`if attacker.id == target.id { sp = 0; }`), while full HP and all debuff removals still apply.

4.  **Clean Target Behavior**:
    *   For Type 18 (Cleanse + Heal): If targets have zero debuffs, the skill executes with `ATTACK` status and applies HP/SP healing.
    *   For Type 5 (Dispel without heal): If the target lacks the requisite status, the skill returns `miss_status::MISS` without false purge broadcasts.

--------------------------------------------------------------------------------

## User Stories

1.  As a Water element tactician, I want Băng tan (`11015`) to thaw an ally frozen by Băng phong (`11014`), so that my ally is freed to act in the current turn.
2.  As a combatant, I want Băng tan (`11015`) to return MISS when cast on an ally who is not frozen, so that animation time is not wasted on unneeded cures.
3.  As a player facing crowd control, I want Giải hôn (`14007`) to awaken an ally incapacitated by Hôn mê (`14008`), so that sleep effects can be reliably countered in PvP and PvE.
4.  As a player facing Poison damage-over-time, I want Giải độc (`14014`) to cleanse Bỏ độc (`14015`) from `type15_id`, so that my ally stops taking poison damage each round.
5.  As a party leader, I want Giải hỗn loạn (`14022`) to clear Hỗn loạn (`14021`) from `type15_id`, so that confused teammates do not randomly attack friendly units.
6.  As an Earth element tactician, I want Giải kết giới (`10009`) to dispel enemy Kết giới (`10010`), so that the enemy loses their invulnerability barrier.
7.  As an Earth element tactician, I want Giải trừ kính (`10014`) to dispel enemy Kính (`10015`), so that our team's physical and magic attacks are not reflected back onto us.
8.  As a Water support player, I want Trạng thái giải trừ (`11012`) cast on an ally to remove Freeze and Poison while keeping their active Kính (Mirror) and Celestial Shield intact, so that supportive buffs are not destroyed by my own cleanse.
9.  As a player, I want Trạng thái giải trừ (`11012`, `11025`, `11031`) to return MISS if mistakenly targeted at an enemy unit, so that players cannot inadvertently buff or cleanse opposing combatants.
10. As a party support, I want Tiên Giai Trọng Giải (`11025`) to cleanse debuffs from up to 3 friendly units simultaneously, so that multi-target crowd control can be efficiently countered.
11. As a party support, I want Siêu Trọng Giải Thuật (`11031`) to cleanse debuffs across all 6 positions on the friendly side, so that whole-team debuff wipes can be achieved in one action.
12. As a player using Cường hiệu giải trừ (`20050`), I want the skill to cleanse debuffs on up to 5 allies and restore HP and SP proportional to my skill level (Lv 1 to Lv 5) and Intelligence, so that investing skill points yields noticeably higher recovery.
13. As a player using Tứ nhân trạng giải (`20015`), I want the skill to cleanse debuffs on up to 5 allies and restore HP and SP scaling from Lv 1 to Lv 10, so that high-level characters receive substantial survival restoration.
14. As a player using Giải thoát (`20019`), I want the skill to cleanse and heal up to 6 allies across the entire team formation, scaling recovery with skill level up to Lv 10.
15. As a high-tier summoner casting Thủy thần (`11016..=11019`), I want the summon tier base values (400..700 HP, 100..250 SP) to receive additional recovery scaling based on my active skill level, rewarding skill mastery.
16. As a battle engine balance maintainer, I want Cường hiệu giải trừ (`20050`), Tứ nhân trạng giải (`20015`), and Giải thoát (`20019`) to clamp caster SP gain to 0 when self-targeting, so that infinite SP recycling loops are impossible.
17. As an ally with full health and zero debuffs, I want to receive SP recovery when targeted by Type 18 skills, so that team SP battery tactics function smoothly.
18. As a client player, I want all successful status dispels to emit byte-exact Delphi frames: `0x35 0x01` (Purge FX) followed by `0x35 0x0F` (HUD clear), so that the visual UI matches server state without desynchronization.
19. As a client player, I want successful Type 18 cleanses to emit numeric floating text frames for HP (subcode `19`) and SP (subcode `1A`), so that healing feedback is immediately visible.
20. As a game observer, I want clean targets with no active debuffs targeted by pure dispels (Type 5) to receive `miss_status::MISS`, so that no phantom purge frames are broadcast.

--------------------------------------------------------------------------------

## Implementation Decisions

### 1. Centralized Domain Dispel Module

A dedicated domain module will be introduced within the combat engine architecture:
- **Module responsibility**: Encapsulates all status removal logic, channel validation, friendly-vs-hostile alignment rules, dynamic HP/SP recovery calculations, and packet payload generation.
- **Integration with Runner**: The battle runner delegates all Level-2 dispel/cleanse skill execution (dispatched from `skill_type` 5, 16, and 18) to this domain module, returning a structured execution result.

### 2. Dispel Result Prototype Specification

The domain module computes a strongly typed result representing the combat impact:

```rust
pub struct DispelExecutionOutcome {
    pub hit: bool,
    pub purged_type3: bool,
    pub purged_type4: bool,
    pub purged_type15: bool,
    pub purged_type19: bool,
    pub hp_healed: i64,
    pub sp_restored: i64,
    pub broadcast_packets: Vec<String>,
}
```

### 3. Canonical Dispel Matrix Table

The module strictly evaluates according to the canonical matrix:

| Skill ID | Skill Name | Archetype | Target Rule | Target Channel | Matched Status ID | Heals HP/SP? |
|---|---|---|---|---|---|---|
| `11015` | Băng tan | Specific Dispel | Friendly | `type3_id` | `11014` (Băng phong) | No |
| `14007` | Giải hôn | Specific Dispel | Friendly | `type3_id` | `14008` (Hôn mê) | No |
| `14014` | Giải độc | Specific Dispel | Friendly | `type15_id` | `14015` (Bỏ độc) | No |
| `14022` | Giải hỗn loạn | Specific Dispel | Friendly | `type15_id` | `14021` (Hỗn loạn) | No |
| `10009` | Giải kết giới | Specific Dispel | Hostile | `type4_id` | `10010` (Kết giới) | No |
| `10014` | Giải trừ kính | Specific Dispel | Hostile | `type4_id` | `10015` (Kính) | No |
| `11012` | Trạng thái giải trừ (1T) | Mass Cleanse | Friendly-Only | `type3` + `type15` | Any `> 0` | No |
| `11025` | Tiên Giai Trọng Giải (3T) | Mass Cleanse | Friendly-Only | `type3` + `type15` | Any `> 0` | No |
| `11031` | Siêu Trọng Giải Thuật (6T) | Mass Cleanse | Friendly-Only | `type3` + `type15` | Any `> 0` | No |
| `22005` | Quang du thuật (3T) | Mass Cleanse | Friendly-Only | `type3` + `type15` | Any `> 0` | No |
| `20050` | Cường hiệu giải trừ (5T) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Scaled Lv 1..5) |
| `20015` | Tứ nhân trạng giải (5T) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Scaled Lv 1..10) |
| `20019` | Giải thoát (6T) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Scaled Lv 1..10) |
| `11016` | Thủy thần (Tier 1) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Base 400/100 + scale) |
| `11017` | Thủy thần (Tier 2) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Base 500/150 + scale) |
| `11018` | Thủy thần (Tier 3) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Base 600/200 + scale) |
| `11019` | Thủy thần (Tier 4) | Cleanse & Heal | Friendly-Only | `type3` + `type15` | Any `> 0` | Yes (Base 700/250 + scale) |

### 4. Mathematical Recovery Formulas for Type 18 Cleanse & Heal

*   **Thủy Thần (`11016..=11019`)**:
    $$\text{HP} = \text{base\_hp} + \text{skill\_lv} \times 30$$
    $$\text{SP} = \text{base\_sp} + \text{skill\_lv} \times 15$$
    *(where base_hp is 400, 500, 600, 700 and base_sp is 100, 150, 200, 250 across tiers 1..4)*.
*   **Cường Hiệu Giải Trừ (`20050`, max Lv 5)**:
    $$\text{HP} = \text{round}(\text{caster.int1} \times 0.40) + \text{skill\_lv} \times 40$$
    $$\text{SP} = \text{round}(\text{caster.int1} \times 0.20) + \text{skill\_lv} \times 20$$
*   **Tứ Nhân Trạng Giải (`20015`, max Lv 10) & Giải Thoát (`20019`, max Lv 10)**:
    $$\text{HP} = \text{round}(\text{caster.int1} \times 0.35) + \text{skill\_lv} \times 25$$
    $$\text{SP} = \text{round}(\text{caster.int1} \times 0.15) + \text{skill\_lv} \times 12$$

### 5. SP Infinite Loop Guard

For all Type 18 skills, when evaluating healing on a cell where `attacker.id == target.id`, the calculated SP recovery is strictly overridden to 0:
```rust
if attacker.id == target.id {
    sp = 0;
}
```
HP recovery and debuff removals remain fully effective on the caster.

### 6. Wire Protocol Parity and Broadcast Sequencing

When debuffs are removed, broadcast frames must maintain byte-exact ordering compliant with ADR 0006:
1.  **Purge Frame**: `0x35 0x01` (`PURGE_TYPE_DISABLE` for Type 3, `PURGE_TYPE_DEBUFF` for Type 15, `PURGE_TYPE_BUFF` for Type 4).
2.  **Clear Status HUD**: `0x35 0x0F` (`STATUS_SEAL_RESIST_PCT` for Type 3, `STATUS_DEF_MODIFIER` and `STATUS_ATK_MODIFIER` for Type 15/4).
3.  **Action Frame (`0x32 0x01`)**: Embeds `skilling_int` with action status, troi byte, and recovery amounts (subcode `19` for HP, subcode `1A` for SP).

--------------------------------------------------------------------------------

## Testing Decisions

### 1. External Behavior vs. Implementation Detail
Tests must execute against the highest possible public domain boundary:
- Construct deterministic in-memory `Battle` instances using fixed RNG seeds (`Battle::with_seeds`).
- Execute actions via `battle.run_turn(&data, &commands, &mut out)`.
- Assert cell state mutations (`cell.type3_id`, `cell.type4_id`, `cell.type15_id`, `cell.hp`, `cell.sp`) and verify byte-exact wire frames in the broadcast output queue.
- Do not mock internal helper methods or inspect private fields directly.

### 2. Comprehensive Test Suite Structure
A dedicated integration test file will be created: `tests/battle_dispel_cleanse_test.rs`.

The suite will contain focused test cases:
1.  **Specific Dispel Parity**:
    - Thawing Freeze (`11015` vs `11014`) removes Type 3 and emits disable purge.
    - Waking Sleep (`14007` vs `14008`) removes Type 3.
    - Curing Poison (`14014` vs `14015`) removes Type 15 and stops DOT ticks.
    - Dispelling Barrier (`10009` vs `10010`) removes Type 4 from enemy.
    - Dispelling Mirror (`10014` vs `10015`) removes Type 4 from enemy.
    - Each specific dispel returns `miss_status::MISS` when targeted at a clean unit or wrong status.
2.  **Mass Cleanse Friendly-Only & Buff Preservation**:
    - Seed an ally carrying Type 3 (Freeze), Type 15 (Poison), Type 4 (Mirror), and Type 19 (Job Aura).
    - Cast `11012` Trạng thái giải trừ.
    - Assert Type 3 and Type 15 are removed (`id == 0`).
    - Assert Type 4 (Mirror) and Type 19 (Aura) remain completely intact (`id > 0`, turns unchanged).
    - Cast `11012` on an enemy $\to$ assert returns `miss_status::MISS` and leaves enemy state untouched.
3.  **Multi-Target Cleanse Concurrency (`11025`, `11031`)**:
    - Seed a multi-unit friendly party with varied debuffs.
    - Cast `11025` (3 targets) and `11031` (full team 6 targets).
    - Assert all friendly targets within the area are cleansed.
4.  **Type 18 Level-Scaled Recovery & Self-Cast SP Loop Gate**:
    - Cast `20050` at Lv 1 vs Lv 5 with identical Intelligence $\to$ assert Lv 5 heals strictly more HP and SP.
    - Cast `20050` self-targeted $\to$ assert caster receives full HP heal, full debuff cleanse, but exactly 0 SP restored. Teammates in the AOE receive full SP restored.
    - Cast `20015` and `20019` at Lv 1 vs Lv 10 $\to$ assert correct recovery progression.
5.  **Clean Target Execution Verification**:
    - Cast Type 18 on allies with zero debuffs $\to$ assert `ATTACK` status, HP and SP increase.
    - Cast Type 5 pure cleanse on allies with zero debuffs $\to$ assert returns `miss_status::MISS` without sending empty purge frames.

### 3. Prior Art
- `tests/battle_buff_debuff_lifecycle_test.rs`: Test fixtures, unit seeding, and frame parsing helpers.
- `tests/battle_status_frames_test.rs`: Wire frame byte-level comparison.
- `tests/battle_timing_wiring_test.rs`: Purge and clear sequence validation.

--------------------------------------------------------------------------------

## Out of Scope

1.  **No Delphi Client Binary Modification**: `aLogin.exe` binary layout, rendering engine, and memory structs are unedited.
2.  **No Binary Record Layout Changes**: `Data/Skill.Dat` 86-byte static record layout is preserved.
3.  **No Non-Combat or Inventory Items**: Combat items (medicines, scrolls) retain existing `apply_use_item` routing; item-based cleanses are out of scope.
4.  **No Elemental Multipliers on Dispel Landing**: Dispel accuracy remains deterministic based on canonical miss/hit rules without custom elemental affinities.

--------------------------------------------------------------------------------

## Further Notes

- **Coding Standards §1-§3 Compliance**: No inline `#[cfg(test)]` in `src/`. All tests reside in `tests/`. Test runtime must execute under 100ms in-memory without database queries.
- **Glossary Synchronization**: Canonical terms `Dispel Matrix` and `SP Infinite Loop Gate` have been formally recorded in `GLOSSARY.md`.
