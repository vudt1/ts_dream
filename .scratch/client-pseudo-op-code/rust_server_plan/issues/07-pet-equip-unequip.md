# 07 — Phase 4: pet equip / unequip actually attach items

Status: resolved
Blocked by: 03
Spec: `../05-phased-roadmap.md` Phase 4 (4-3, 4-4); `../02-gap-analysis.md` §12 G-14

## What

- 4-3 `pet_equip` (`src/server/handlers/inventory.rs`, ~554-580) currently only removes the item from `homdo`. Make it store the item on the pet equipment (spec: `trangbi[stt*10+sub]` style mapping — verify against source C# `TS_Server_Bear/` pet equip handling and the client dumps in `client_pseudo_c/` before choosing the storage; prefer whatever model the codebase already has for pet equipment, e.g. `pet_box.rs` / pet structs / `pets` table).
- 4-4 `pet_unequip` (~582-599) actually removes the item from the pet and returns it to a free `homdo` slot (`free_slot`), failing cleanly if the bag is full.
- Persist both directions with existing `persist::*` / pets repo so that equip → disconnect → reconnect keeps the pet's gear. Reply frames must match what source C# sends; do not invent new wire layouts — if a layout cannot be confirmed, keep the current reply frames and add a `TODO(capture)` comment.

## Acceptance

- Tests: equip moves item bag → pet; unequip moves it back; state survives a save/load round-trip.
- `cargo test --all-targets --no-fail-fast` passes.
