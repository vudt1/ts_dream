# 03 — Phase 3: hydrate item templates after load

Status: resolved
Blocked by: 01
Spec: `../05-phased-roadmap.md` Phase 3 (3-1, 3-2), `../04-db-persistence.md` §2, §3.3

## What

- 3-1 (G-05) `pub fn Session::hydrate_templates(&mut self, data: &GameData)` — copy template fields (`lv`, `int1..fai1`, `int2..fai2`, `item_hp`, `item_sp`, `loai`, `thuoctinh`, `giatri_thuoctinh`, … whatever `InventoryItem::from_template` sets) from `Item.dat` for every item in all 5 containers, **never** overwriting instance fields (`slot`, `id`, `count`, `doben`, `long_val`, `giatri_long`, `khang`, `texp`), then `recompute_stats()`.
- 3-2 Call it right after a successful session load in every login branch in `src/server/handlers/login.rs` (and any other place that loads a session from DB, e.g. GM/reload paths — grep).

## Acceptance

- `tests/inventory_template_hydrate_test.rs`: after hydrate, `loai`/`lv`/stat fields match `Item.dat` (use the real `Data/` loader or a minimal GameData built in-test), instance fields unchanged.
- `tests/inventory_gear_bonus_test.rs`: hydrate + recompute ⇒ `int2..agi2` match `character_sheet` gear bonus.
- `cargo test --all-targets --no-fail-fast` passes.
