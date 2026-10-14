# 05 — Phase 3: persist move/stack + auto-save fingerprint all containers

Status: resolved
Blocked by: 01
Spec: `../05-phased-roadmap.md` Phase 3 (3-7, 3-8)

## What

- 3-7 (G-10) `handle_move_stack` in `src/server/handlers/inventory.rs` gets `pool: Option<&DbPool>` and persists **every** touched slot (swap = 2 slots, split = 2, merge may zero a slot → delete/upsert-empty consistently with how other handlers persist removals). Use existing `persist::*` helpers.
- 3-8 `src/server/auto_save.rs` `fingerprint` mixes all 5 containers (`homdo`, `tientrang`, `tuideo`, `trangbi`, `luulang`) — preferably via `Session::containers()` from ticket 01 — and all instance fields.

## Acceptance

- Test: move/stack then reload from DB ⇒ layout preserved.
- Test: changing only `tientrang` changes the fingerprint (needs `fingerprint` or an equivalent to be reachable from `tests/` through the public API).
- `cargo test --all-targets --no-fail-fast` passes.
