# 08 — Deferred: items that need a real `aLogin.exe` capture

Status: ready-for-human
Blocked by: capture (Phase 0 in `../05-phased-roadmap.md`)

Not implemented in this round because evidence is `[C-call-only]` / `[ASSUMPTION]`:

| Item | Gap | Needs |
|---|---|---|
| 2B-1/2B-2/2B-4 `08 01` prop code table, `hp2`/`sp2`, GearBonuses by prop | G-06 | client parser `FUN_0078D125` or capture |
| 2B-3 `05 03` tail (90 zero vs 43 zero + 32B `_3`/`_4`) | G-07 | parser `FUN_0078C7DB` + capture |
| 2B-5 `17 66` layout (or move warehouse to `1E 09`) | G-02 | body `FUN_0076A320`; currently behind `send_luulang_frame=false` |
| 2B-6 / 2A-4 `17 1E` value field, `17 20` padding byte | G-11, G-12 | body `FUN_00768638` + capture |
| 4-1 / 4-2 `17 36` → `17 32`/`17 09`/`17 30`, `17 37` move secondary bag | G-13, G-20 | client `17 30`/`17 32` parsers |
| `17 0B` `doben` (durability) path | G-01 note | capture `17 0C` / soul frame |
| 4-7 persist `tiengtam`/`god`/`color`/stores | G-18 | product decision + migration `0002` |
| 4-8 `1A 04` second currency pool value | — | semantics of `player+0x1300` |
| 4-9 / D-10 affix ↔ stat mapping | G-03 | verify `Item.dat` loader vs columns |

See `../06-tests-and-capture.md` for the capture checklist.
