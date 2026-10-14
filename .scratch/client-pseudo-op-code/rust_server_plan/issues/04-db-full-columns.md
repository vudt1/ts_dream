# 04 — Phase 3: load/save all inventory columns

Status: resolved
Blocked by: 01
Spec: `../05-phased-roadmap.md` Phase 3 (3-3..3-6), Phase 4 task 4-10; `../04-db-persistence.md` §3.1, §3.2, §4, §8

## What

- 3-6 Shared `INVENTORY_COLUMNS` (23 columns) + placeholder helper; share the select-column constant currently private in `src/db/modern/sqlite/inventories.rs` instead of rewriting it.
- 3-3 (G-03) `load_items` reads the 8 wire/instance fields: `slot, itemid, quantity, damage→doben, element→long_val, elementvalue→giatri_long, proofkind→khang, growexp→texp`.
- 3-4 (G-04) `save` inserts all 23 columns (keep DELETE + re-insert strategy). The 10 columns without a Rust field (`growlevel`, `specialkind`, `stoneattr`, `stonelevel`, `enhancelevel`, `deletetime`, `damageditemid`, `islocked`, `reinforced`, `affix1..3`, `stylelevel`) are written as 0 — affix mapping is unverified (D-10), do NOT guess.
- 3-5 `persist::upsert_inventory_tx` writes the same 23 columns (remove the `let _ = item;` leftover). Also `create_and_seed` if it inserts inventories.
- 4-10 (G-15) `clear_items` / `replace_inventory_tx` in `src/db/persist.rs` use `StorageType` instead of raw table strings/u8.

## Acceptance

- `tests/inventory_roundtrip_test.rs` (in-memory SQLite `sqlite::memory:?cache=shared`, see AGENTS.md): save → load keeps the 8 instance fields × 5 containers.
- Per-item upsert keeps the 8 fields too.
- `cargo test --all-targets --no-fail-fast` passes.
