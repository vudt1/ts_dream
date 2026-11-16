# Handoff: Starter Item (`31007`), Pickaxe Equip Unlock & Mineral Combat (Tickets 01, 02 & 03)

**Date:** 2026-11-16
**Completed Tickets:**
- [01-starter-item-31007.md](issues/01-starter-item-31007.md) — `Status: resolved`
- [02-equip-pos-pickaxe-unlock.md](issues/02-equip-pos-pickaxe-unlock.md) — `Status: resolved`
- [03-mineral-npc-pickaxe-combat.md](issues/03-mineral-npc-pickaxe-combat.md) — `Status: resolved`

**Next Ticket (Unblocked):**
- None (all 3 tickets in `.scratch/starter-item-and-mineral-battle/` are `resolved`).

---

## 1. What Was Completed

### Ticket 01: Starter Item `1x 31007` (*Vé đoái hoán tâm*)
- **`src/server/handlers/character.rs` (`apply_to_session`)**: Added `InventoryItem { slot: 2, id: 31007, count: 1, ..Default::default() }` to `session.homdo` right after `32012 × 4` at slot `1`. Both DB path (`create_and_seed` atomic transaction) and in-memory fallback path share `apply_to_session`, keeping `characters.newbie = 0` untouched and not backfilling `31007` on `handle_enter_game` / `handle_login`.
- **`src/db/modern/sqlite/session.rs`**: Updated comment on `create_and_seed` step 4 to reflect `31007 × 1` at slot `2`.
- **`tests/server_network/create_char_atomic.rs`**:
  - Extended `test_atomic_create_with_password_overwrite` to assert `31007 × 1` at slot `2` in RAM and DB, and `characters.newbie == 0`.
  - Added `test_starter_item_31007_enter_game_and_relogin_no_duplication` (verifies `17 05` contains `021F7901` and `010C7D04`, and relogin does not duplicate).
  - Added `test_existing_char_newbie_zero_not_granted_31007_on_enter_game` (negative test: existing `newbie = 0` char is not backfilled).
  - Added `test_in_memory_fallback_create_char_seeds_31007_and_keeps_newbie_zero`.

### Ticket 02: Separate `equip_pos` (`fit_type`) from `kind` (`loai`) & Unlock Pickaxe Equipping
- **`src/data/tables.rs` (`Item`)**: Added `pub equip_pos: i64`.
- **`src/data/loaders/item.rs` (`ItemDef::to_item`)**: Populates `equip_pos: self.fit_type as i64` while keeping `loai: self.kind as i64` intact for Compound/Shop/Trade.
- **`src/data/loader.rs` (`load_items` for `Items.txt`)**: Set `equip_pos: 0` (falls back to `loai` when `1..=6`).
- **`src/server/session.rs` (`InventoryItem`)**:
  - Added `pub equip_pos: u8`.
  - Added `InventoryItem::effective_equip_pos(&self) -> u8` (returns `self.equip_pos` if `> 0`, else `self.loai` if `(1..=6).contains(&self.loai)`, else `0`).
  - Populated `equip_pos` in `InventoryItem::from_template` and `Session::hydrate_templates`.
- **`src/server/handlers/inventory.rs`**:
  - `handle_equip`: Checks `item.effective_equip_pos()` in `1..=6` instead of `item.loai`, equips into `trangbi` slot `equip_pos` (slot `3` for pickaxes).
  - `handle_pet_equip`: Added `conn.session.battle_id > 0` guard; checks `item.effective_equip_pos()` in `1..=6` and equips into `pet_stt * 10 + equip_pos`.
  - `handle_pet_unequip`: Added `conn.session.battle_id > 0` guard.
  - Homdo move/stack: Checks `(1..=6).contains(&src.effective_equip_pos())` so `kind = 39, equip_pos = 3` items move/swap as equipment rather than stackables.
- **`tests/economy_inventory/inventory_roundtrip.rs`**: Added `equip_pos: 1` to `sample_item` fixture.
- **`tests/economy_inventory/inventory_template_hydrate.rs`**:
  - Added `test_pickaxe_itemdef_preserves_kind_39_and_equip_pos_3` (verifies `loai == 39`, `equip_pos == 3`, `29335` `fit_type == 0` -> `0`, and manual `loai = 2, equip_pos = 0` fallback -> `2`).
  - Added `test_real_item_dat_pickaxes_and_tools_equip_pos` (loads real `Data/Item.dat` via `ItemDatLoader::load` and verifies all 6 pickaxe IDs `[10001, 10530, 10531, 10532, 10533, 10534]` have `kind = 39`, `fit_type = 3`, hydrate to `equip_pos = 3`, and `29335`/`31007` have `fit_type = 0`).
- **`tests/economy_inventory/pet_equip_unequip.rs`**: Added `test_player_and_pet_pickaxe_equip_swap_unequip_and_rejections` (player equip/swap/unequip `10001`/`10534`, pet equip/unequip in slot `13`, rejection of `29335`, level gate `10530`, battle lock `battle_id > 0`, and collateral slot integrity).

### Ticket 03: Mineral NPC Immunity & Pickaxe Weapon Combat
- **`src/battle/engine.rs`**:
  - Added `pub weapon_id: u16` to `WarInfo` (excluded from 23-byte `packet_hex()`).
  - Added `PICKAXE_IDS: [u16; 6] = [10001, 10530, 10531, 10532, 10533, 10534]`, `is_pickaxe(weapon_id: u16) -> bool`, and `is_mineral_npc(npc: &Npc) -> bool` (`npc.kind == 16 || npc.id == 22010 || npc.id == 22011`).
- **`src/battle/construction.rs`**:
  - `add_player`: Populates `cell.weapon_id` from `session.trangbi` slot `3`.
  - `load_leader_pets` & `load_member_pet`: Independently populate `cell.weapon_id` from `session.trangbi` slot `pet.stt * 10 + 3`.
  - `add_pet`, `add_npc`, `clear_cell`: Reset `cell.weapon_id = 0`.
- **`src/battle/runner.rs`**:
  - Added `is_mineral_target(data, target)` (scoped to `matches!(target.typ, 3 | 7)`, excluding `typ == 4` player pets) and `is_mineral_immune(data, attacker, target)`.
  - Enforced per-target/per-attacker mineral immunity in `apply_physical`, `apply_damage_to_rear` (evaluated on `rear`), `apply_magic`, `apply_status3`, `apply_buff15`, and `apply_buff19` (`SKILL_HOLY_SPIRIT` `14053`), plus skipping `note_npc_hit` when immune.
  - Blocked capture (`15001`) in `apply_catch` Gate 4 when `is_mineral_target(data, target)`.
- **`tests/battle/mineral_pickaxe.rs`** & **`tests/battle_test.rs`**:
  - Added 8 comprehensive integration tests covering `weapon_id` loading & 23-byte snapshot parity, all 6 pickaxes positive cases, non-pickaxe / `10497`/`10499`/`10500` negative cases across all harmful skill types, player-pet strict independence, mixed AoE & mixed Combo, Rear Guard (`20006`), capture rejection (`15001`), and PvP `typ == 4` pet isolation.

---

## 2. Verification Status

- `cargo test --test battle_test`: **158 passed, 0 failed** (including 8 new tests in `mineral_pickaxe`)
- `cargo test --test server_network_test`: **94 passed, 0 failed**
- `cargo test --test economy_inventory_test`: **52 passed, 0 failed**
- `cargo test --all-targets --no-fail-fast`: **Exit code 0 — all test suites passed**
- All changes remain uncommitted in the working tree per `AGENTS.md`.

---

## 3. Guidance for Next Agent (Ticket 03: Mineral NPC & Pickaxe Combat)

Ticket `03-mineral-npc-pickaxe-combat.md` is now unblocked. Follow `spec.md` §Implementation Decisions #3–5 and §Testing Decisions #3:

1. **`WarInfo` weapon tracking**:
   - Add `pub weapon_id: u16` (default `0`) to `WarInfo` without changing the 23-byte wire snapshot (`0B 05` / `0B FA`).
   - Populate `weapon_id` when loading the player from `session.trangbi` slot `3`, and when loading each pet (both leader pets and party member pets) from the owner's `trangbi` slot `pet.stt * 10 + 3`.
2. **Pure identification helpers**:
   - Pickaxe IDs: `[10001, 10530, 10531, 10532, 10533, 10534]` -> `is_pickaxe(weapon_id: u16) -> bool`.
   - Mineral NPCs: `is_mineral_npc(npc)` when `npc.kind == 16 || npc.id == 22010 || npc.id == 22011`.
   - Mineral target: `is_mineral_target(data, target)` when `matches!(target.typ, 3 | 7)` and `data.npcs.get(&target.id)` satisfies `is_mineral_npc`. (Note: player pets `typ == 4` are explicitly excluded.)
3. **Per-target / per-attacker battle rules**:
   - Enforce `is_mineral_target(data, target) && !is_pickaxe(attacker.weapon_id)` in `apply_physical`, `apply_damage_to_rear` (evaluate on `rear`!), `apply_magic`, `apply_buff19` (`Holy Spirit`), `apply_status3`, `apply_buff15`.
   - Blocked hits return `MISS` (`miss_status::MISS = 0`, `damage = 0`), apply no debuff/SP drain/reflect, while still consuming attacker SP and setting `attacked = true`.
   - In `apply_catch` (Gate 4, skill `15001`), reject capture if `is_mineral_target(data, target)` (blocks `22010`, `22011`, and `25001..=25252`).
4. **Tests**:
   - Add comprehensive tests in `tests/battle/` (declared in `tests/battle_test.rs`) covering positive, negative, mixed AoE, mixed Combo, Rear Guard (`20006`), capture rejection (`15001` on `22010`/`22011`/`25002`), and PvP `typ == 4` pet isolation.
