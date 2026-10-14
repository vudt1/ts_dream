# 01 — Phase 1: Container metadata (single source of truth)

Status: resolved
Blocked by: —
Spec: `../README.md`, `../05-phased-roadmap.md` Phase 1 (tasks 1-1..1-7), `../04-db-persistence.md` §5, §6

## What

Make `StorageType` the single source of truth for `storagetype ↔ table name ↔ dump code ↔ capacity ↔ entry layout`. **No wire byte changes** in this ticket (17 0B stays 12B here; ticket 02 switches it).

- 1-1 `EntryLayout` enum + `table_name`/`dump_code`/`capacity`/`entry_layout`/`from_table_name` on `StorageType` (`src/db/modern/model.rs`).
- 1-2 `Session::containers()` (and `containers_mut()` only if needed).
- 1-3 Replace the duplicated `(1u8, &session.homdo) …` tuple literals in `src/db/modern/sqlite/session.rs` with `containers()`.
- 1-4 `load_items` takes `StorageType` instead of `u8`.
- 1-5 `persist::storage_type` → `StorageType::from_table_name`.
- 1-6 `free_slot_in(bag, capacity)`; keep `free_slot` for the 25-slot bag; bank paths use `StorageType::Bank.capacity()` (G-09, G-19).
- 1-7 Split `dump_table` into `encode_entry_12` / `encode_entry_10` + `dump_container(items, StorageType)`; add `biased_giatri_long` (+100 wire bias, `04-db-persistence.md` §6). Keep 17 0B output identical for now (still 12B) — ticket 02 flips it.

## Acceptance

- `cargo build` clean, no new warnings; `cargo test --all-targets --no-fail-fast` passes.
- `grep -rn "1u8, &session.homdo" src/` → 0.
- New `tests/inventory_storage_type_test.rs` asserts the 5 mappings from `../01-container-mapping.md`.
