# 06 — Phase 4: pet slot `u16 as u8` overflow + hotkey kind 1 (item)

Status: resolved (merged a0be351 into feat/container-normalize)
Blocked by: —
Spec: `../05-phased-roadmap.md` Phase 4 (4-5, 4-6); `../03-wire-codecs.md` §8; `../02-gap-analysis.md` §12

## What

- 4-5 Fix silent `u16 as u8` truncation in `move_pet_slot` (`src/server/handlers/pet_actions.rs`, around lines 50, 96-97): validate range and reject/ignore out-of-range values instead of wrapping.
- 4-6 Hotkey `kind 1` = item quickslot. Client evidence: `client_pseudo_c/005ab3f8_FUN_005ab3f8.*` (stride 4: `type, id LE16, slot`; `type 1` → item, `type 2` → skill). Server `handle_hotkey` (`src/server/handlers/stats.rs`) currently handles only kind 0 (clear) and kind 2 (skill). Add kind 1 so the item hotkey is stored and re-sent in `28 01` with `type = 1`. Look at how hotkeys are stored on `Session` and persisted; if the storage cannot distinguish item vs skill, extend it minimally (no new SQL migration unless unavoidable — if unavoidable, add `migrations/0002_*.sql`, never edit `0001`). Also check the C→S hotkey packet shape against `TS_Server_Bear` (source C#) if present.

## Acceptance

- Tests in `tests/` for the overflow guard and for `28 01` encoding of kind 1 + kind 2 entries.
- `cargo test --all-targets --no-fail-fast` passes.
