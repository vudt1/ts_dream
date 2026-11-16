# Handoff: Quest Refactor — Tickets 11 through 18 (Progress: 11–13 Resolved)

> **Spec reference:** [`.scratch/quest-refactor/spec-p0-p2.md`](spec-p0-p2.md)  
> **Prior handoff:** [`.scratch/quest-refactor/HANDOFF_01_10.md`](HANDOFF_01_10.md)

## Summary of Completed Work (Tickets 11–13)

### Ticket 11 ([`issues/11-remove-should-skip-event.md`](issues/11-remove-should-skip-event.md)) — `resolved`
1. **Removed `should_skip_event` & `has_later_completed_event` (`src/eve/auto_chain.rs`, `src/server/handlers/npc_event.rs`)**:
   - Completely removed `should_skip_event` (Rule 1: `completed_count > 0` without `cc=12`; Rule 2: later completed event lock) and `has_later_completed_event` from `src/eve/auto_chain.rs`.
   - `EveAutoChainEngine::try_auto_chain` no longer skips when `battle_result == 0`.
   - `npc_event.rs` no longer calls `should_skip_event` on either `WalkStep` encounter activation or `ClickNpc` / `ClickDoor` / `MeetDoor` activation (including with `chain_filter`).
2. **Preserved Loop Guards & Condition-Based Branching (`src/eve/auto_chain.rs`)**:
   - Guard #1 (`detect_same_chain`) uses `continue` instead of early `return AutoChainResult::NoMatch`, allowing subsequent events/conditions in the same `EveNo` to be evaluated while keeping all 4 infinite-loop guards (`detect_same_chain`, `detect_re_question`, `detect_re_battle`, `detect_duplicate_items`) active.
   - Branching now relies solely on `EveCondition` (`cc=2`, `cc=12`, etc.) and `EveResolver` priority order.
3. **Tests (`tests/tickets_11_12_13_test.rs`, `tests/p0_blockers_test.rs`, `tests/pure_planner_event_runner_and_preflight_test.rs`)**:
   - Added 4 tests in `tests/tickets_11_12_13_test.rs`:
     - `ticket11_multi_step_same_eve_no_without_cc12_succeeds` (E2E via `dispatcher::dispatch`)
     - `ticket11_npc_with_multiple_independent_quests_later_does_not_lock_earlier`
     - `ticket11_auto_chain_battle_result_zero_and_same_condition_guard` (positive `battle_result == 0` + negative Guard #1)
     - `ticket11_negative_one_time_quest_with_cc12_still_blocked`
   - Updated `tests/p0_blockers_test.rs` and `tests/pure_planner_event_runner_and_preflight_test.rs` for `cc=12` / `AutoChain` behavior after `should_skip_event` removal.

---

### Ticket 12 ([`issues/12-eve-scene-info-warp-rt2.md`](issues/12-eve-scene-info-warp-rt2.md)) — `resolved`
1. **Parsed `EveSceneInfo.dest_map_id` (`src/data/loaders/eve.rs`)**:
   - Added `pub dest_map_id: u16` to `EveSceneInfo` and read `reader.read_u16()` in place of `reader.skip(2)` between `scene_info_id` and `player_appear_x`.
2. **Unified `resolve_warp_destination` & `rt=2` Priority (`src/server/handlers/quest.rs`, `src/server/handlers/eve_runner.rs`, `src/battle/service.rs`, `src/eve/planner.rs`)**:
   - Added `quest::resolve_warp_destination(data, map_id, warp_id)`: checks `data.scene_eve_data.get(&map_id).and_then(|s| s.scene_infos.get(&warp_id))` first; when present, requires `dest_map_id > 0 && player_appear_x > 0 && player_appear_y > 0` and converts tile to pixel via `(tile * 20).saturating_sub(10)` (returning `None` without falling back to `data.warps` if the `EveSceneInfo` entry is invalid). Falls back to `data.warps` (`Warps.txt`) only when `scene_infos` lacks `warp_id`.
   - Wired `resolve_warp_destination` across `eve_runner.rs`, `quest.rs` (`perform_party_warp_and_persist` with `Two-Phase State Lock` + atomic DB write-through, and `handle_warp_confirm`), and `battle/service.rs` (both online and offline `eve_warp` paths).
   - Unified all `"Warps.dat"` references/comments in `src/` to `"Warps.txt"` (`quest.rs`, `eve/planner.rs`).
   - *Minor non-blocking note:* `src/server/handlers/quest.rs:1032` still logs `"Warp id={} map={} not found in Warps.txt"` when `resolve_warp_destination` returns `None`.
3. **Tests (`tests/tickets_11_12_13_test.rs`, `tests/movement_warp_test.rs`, `tests/npc_eve_e2e_test.rs`, `tests/door_triggers_walkstep_and_party_warp_test.rs`)**:
   - Added 5 tests in `tests/tickets_11_12_13_test.rs`:
     - `ticket12_eve_emg_parses_dest_map_id_for_all_10336_rt2_warps` (verifies `10,336 / 10,336` `rt=2` results in `Data/eve.emg`)
     - `ticket12_e2e_rt2_warp_uses_eve_scene_info_and_persists_party` (E2E via `dispatcher::dispatch` + DB persistence for leader & member)
     - `ticket12_e2e_rt2_warp_prefers_eve_scene_info_over_conflicting_warps_txt`
     - `ticket12_post_battle_deferred_warp_uses_eve_scene_info`
     - `ticket12_negative_missing_both_or_zero_dest_map_id_skips_safely`

---

### Ticket 13 ([`issues/13-ground-server-mmg-and-diahinh.md`](issues/13-ground-server-mmg-and-diahinh.md)) — `resolved`
1. **Extracted `Data/ground_server.mmg` & Removed `Data/ground.mmg` (`examples/extract_ground_server.rs`, `src/data/loaders/ground.rs`, `src/data/loaders/mod.rs`)**:
   - Added `GroundMapMeta { width: u16, height: u16, geol_base_att: u16 }` (re-exported in `src/data/loaders/mod.rs`) and `GroundMmgLoader::extract_server_mmg`.
   - Removed per-map `bytes.to_vec()` buffer cloning in `read_map_meta`, reading directly on `&[u8]`.
   - Generated `Data/ground_server.mmg` (`217,346` bytes for all `4,528` maps, preserving `MapData.New` index, `width`, `height`, and `geolBaseAtt` while zeroing client visual/block/scene/npc/eve arrays) and deleted `Data/ground.mmg` (`26.7 MB`) from the working tree.
2. **Wired `GameData::map_diahinh` & Standardized Battle `DiaHinh` (`src/data/loader.rs`, `src/server/handlers/eve_runner.rs`, `src/server/handlers/movement.rs`, `GLOSSARY.md`)**:
   - `GameData::load` / `load_from_dir` loads `ground_server.mmg` (fallback `ground.mmg`) into `ground_metas` and `ground_terrains`, exposing `GameData::map_diahinh(map_id)` with fallback `112`.
   - Updated `eve_runner.rs` (EVE `rt=6` battle) and `movement.rs` (random encounter `WalkStep` & `WalkConfirm`) to use `data.map_diahinh(...)` for `0x0B 0xFA` `DiaHinh` instead of `scene_infos.get(&1).background_no`.
   - Updated `GLOSSARY.md` definitions for `DiaHinh` and `geolBaseAtt`.
3. **Tests (`tests/tickets_11_12_13_test.rs`, `tests/ground_test.rs`)**:
   - Updated `tests/ground_test.rs` to load `Data/ground_server.mmg` and assert all `4,528` maps have positive `width` and `height`.
   - Added 3 tests in `tests/tickets_11_12_13_test.rs`:
     - `ticket13_gamedata_loads_ground_server_mmg_and_map_diahinh`
     - `ticket13_eve_battle_0x0b_fa_uses_ground_diahinh_not_scene_info_background_no`
     - `ticket13_random_encounter_0x0b_fa_uses_ground_diahinh_and_fallback_112`

---

## Verification Record (Tickets 11–13)

- `cargo test --test tickets_11_12_13_test`: **12 passed, 0 failed**.
- `cargo test --all-targets --no-fail-fast`: **exit 0** (all test binaries passed, including `architecture_guardrails_test` 5/5).
- `cargo clippy --all-targets`: **exit 0**, 0 warnings in modified/added files (`quest.rs`, `tickets_11_12_13_test.rs`, etc.; pre-existing warnings in unrelated modules remain untouched).
- **Unverified at runtime:** Live `aLogin.exe` TCP gameplay for `FField<N>` / `BKMusic` rendering from `geolBaseAtt` and repeatable-quest `AutoChain` feel (verified at wire-byte `0x0B 0xFA` and E2E test level only).

---

## Next Tickets for Continuation

Tickets **14 through 18** in `.scratch/quest-refactor/issues/` remain `ready-for-agent`:
- `14-when-happen-group-use-mode-and-and-chain-results.md`
- `15-quest-dont-step1-and-cc7-p2-regression.md`
- `16-movement-walkstep-hotpath-and-sync-future-isolation.md`
- `17-party-two-phase-lock-and-db-persist.md`
- `18-event-session-preplan-index-startup-db-and-docs.md`

## Suggested Skills

- `tdd` — For writing failing tests first / verifying E2E & negative tests per ticket in `tests/`.
- `code-review` — For auditing changes against `CODING_STANDARDS.md` and `spec-p0-p2.md` before handoff.
- `diagnosing-bugs` — If any regression or SQLite/concurrency edge case surfaces during Tickets 14–18.
