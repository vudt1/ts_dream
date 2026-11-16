# Handoff: Quest Refactor — Tickets 11 through 18 (Progress: 11–18 Resolved)

> **Spec reference:** [`.scratch/quest-refactor/spec-p0-p2.md`](spec-p0-p2.md)  
> **Prior handoff:** [`.scratch/quest-refactor/HANDOFF_01_10.md`](HANDOFF_01_10.md)

## Summary of Completed Work (Tickets 11–17)

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

### Ticket 14 ([`issues/14-when-happen-group-use-mode-and-and-chain-results.md`](issues/14-when-happen-group-use-mode-and-and-chain-results.md)) — `resolved`
1. **Door `when_happen` Separation (`src/eve/auto_chain.rs`, `src/server/handlers/npc_event.rs`)**:
   - Added `EventTriggerKind::matches_when_happen(self, when_happen: [bool; 4]) -> bool`:
     - `ClickDoor`: requires `when_happen[0]` (`on_click_obj`).
     - `MeetDoor`: requires `when_happen[1] || when_happen[2]` (`on_stroke_obj || on_into_area`).
     - `ClickNpc`: accepts `when_happen[0] || when_happen[1] || when_happen[3]` (with all-false fallback), preserving `3,122` `on_ser_stroke` NPC events.
     - `WalkStep`: returns `true` (not filtered by `when_happen`), preserving `1,835` `on_click_obj` encounters.
   - Enforced `matches_when_happen` in both `npc_event::activate` and `EveAutoChainEngine::try_auto_chain`.
2. **`GroupData.use_mode == 2` (`src/eve/group.rs`)**:
   - `select_member` now checks `group_data.use_mode == 2` before `pick_member` and weighted random selection, returning `KEEP_ALL` to retain every group member; `use_mode == 1` and `use_mode == 3` continue through the existing `pick_member` / weighted random path.
3. **`EveResolver::resolve` `chain[1..].results` (`src/eve/resolver.rs`)**:
   - `resolve` now checks total result count across the entire AND chain (`chain.iter().map(|c| c.results.len()).sum::<usize>() > 0`) instead of `head.results.is_empty()`.
   - Collects `results` from all links `winner.chain[0..]` in order, uses total `result_count` as the tertiary tie-breaker, and updates doc comments to match the 3 comparison tiers + declaration-order tie-break.
4. **Tests (`tests/tickets_14_15_test.rs`, `tests/door_triggers_walkstep_and_party_warp_test.rs`, `tests/tickets_11_12_13_test.rs`)**:
   - Updated synthetic `ClickDoor` / `MeetDoor` fixtures in `door_triggers_walkstep_and_party_warp_test.rs` and `tickets_11_12_13_test.rs` with explicit `when_happen` flags.
   - Added 6 tests in `tests/tickets_14_15_test.rs`:
     - `ticket14_door_when_happen_separates_click_door_and_meet_door`
     - `ticket14_click_npc_on_ser_stroke_and_walkstep_encounter_on_click_obj_preserved`
     - `ticket14_auto_chain_door_respects_when_happen`
     - `ticket14_group_data_use_mode_2_keeps_all_while_1_and_3_pick_one`
     - `ticket14_e2e_group_use_mode_2_grants_all_items_and_preserves_adjacent_homdo_slots`
     - `ticket14_resolver_collects_chain_1_plus_results_in_order` & `ticket14_e2e_and_chain_tail_results_execute`

---

### Ticket 15 ([`issues/15-quest-dont-step1-and-cc7-p2-regression.md`](issues/15-quest-dont-step1-and-cc7-p2-regression.md)) — `resolved`
1. **Removed `mark_dont_on_step1` (`src/eve/planner.rs`, `src/server/handlers/eve_runner.rs`)**:
   - Removed the `mark_dont_on_step1: bool` field from `QuestOp::Advance` in `src/eve/planner.rs` (no other consumers).
   - Removed the automatic `set_quest_dont(session, out, mark, 1)` call on `step == 1` in `eve_runner::execute_update_quest`; `rt=0, rc=2, p_style=1` now only updates the `Quest Ledger`.
   - Verified `set_quest_dont(..., 1)` (`S->C 0x18 05`) is only called via `QuestOp::MarkDont` (`p_style=5`) and `QuestOp::ExplicitMark` (`p_style=3` with `result_value > 0`, guarded in `eve_runner.rs`).
2. **Tests (`tests/tickets_14_15_test.rs`, `tests/checkpoint6_parity_test.rs`, `tests/pure_planner_event_runner_and_preflight_test.rs`)**:
   - Updated `checkpoint6_parity_test.rs` (`test_class2_quest_save_triggers_quest_dont_when_mark_present`) and `pure_planner_event_runner_and_preflight_test.rs` (Quest 10049 `p_style=1 step=1` no longer marks `quest_dont` bit 27).
   - Added 12 tests in `tests/tickets_14_15_test.rs` (including real `Data/eve.emg` `use_mode == 2` & `15` AND chains / `29` tail nodes, `ExplicitMark` `p_style=3` positive-only guard, real `Data/Mark.dat` 7 multi-step quests, and `cc=7, param=2, p_style=4` regression).

---

---

### Ticket 16 ([`issues/16-movement-walkstep-hotpath-and-sync-future-isolation.md`](issues/16-movement-walkstep-hotpath-and-sync-future-isolation.md)) — `resolved`
1. **WalkStep Encounter Fast-Path & Snapshot Coordination (`src/server/handlers/movement.rs`, `src/server/handlers/npc_event.rs`)**:
   - `handle_move_dispatch` now checks for a matching `MeetDoor` (±60 px) or `WalkStep` encounter zone (`scene.encounters.iter().any(|enc| enc.contains(px, py))`) before seeding RNG or calling `resolve_npc_event_with_filter_and_state`.
   - `resolve_npc_event_with_filter_and_state` also gates `NpcTrigger::WalkStep` against `scene.encounters` **before** calling `snapshot_state_with_data`.
   - `try_trigger_random_encounter` returns `Option<PlayerEventState>` when it builds a snapshot and does not start a battle, allowing `handle_move_dispatch` to reuse that snapshot for `MeetDoor` / `WalkStep` without building a duplicate snapshot.
2. **Zero-Copy Borrowed `quest_marks` (`src/eve/state.rs`, `src/server/handlers/npc_event.rs`)**:
   - Changed `PlayerStateInputs::mark_defs` from `&'a HashMap<i32, i32>` to `Option<&'a HashMap<u16, u16>>` borrowed directly from `GameData.quest_marks`.
   - Removed the per-snapshot 2,394-entry `HashMap<i32, i32>` allocation in `npc_event::snapshot_state_with_data`; `EveStateBuilder::build_player_state` skips mark projection entirely when `raw_flags` is empty.
3. **Sync Wrapper Isolation (`src/server/handlers/eve_runner.rs`, `src/server/handlers/talk.rs`)**:
   - Marked `execute_event_step`, `resume_eve_after_battle_sync`, and `resume_eve_after_battle_sync_full` as `#[doc(hidden)]` test-harness-only wrappers; production handlers exclusively use async `*_with_pool(...).await`.
4. **Tests (`tests/eve_quest/tickets_16_17.rs`)**:
   - `ticket16_walkstep_outside_encounter_zone_skips_cleanly_and_inside_reuses_snapshot`
   - `ticket16_quest_marks_borrowed_reference_evaluates_cc2_pstyle3_correctly`
   - `ticket16_no_production_caller_of_run_sync_future_or_sync_wrappers`

---

### Ticket 17 ([`issues/17-party-two-phase-lock-and-db-persist.md`](issues/17-party-two-phase-lock-and-db-persist.md)) — `resolved`
1. **Two-Phase State Lock & Multi-Member DB Persist (`src/server/handlers/event_npcs.rs`, `src/server/handlers/battle.rs`)**:
   - Event NPCs `15312`, `15313`, and `61210` now collect sorted, deduplicated `participant_ids`, acquire `lock_player_operations(participant_ids).await` before reading/updating member sessions via `registry_get`, persist both leader and all warped members via `persist_sessions_transaction(..., &["stats", "quest"])`, and sync via `registry_update_if_online`.
   - `battle::handle_quiz_answer` (`0x10`) now acquires `lock_player_operations(participant_ids).await` before evacuating party members, syncs leader + members via `registry_update_if_online`, and passes both leader and evacuated members to `persist_sessions_transaction`.
2. **Movement Party Sync & 40-NPC Review (`src/server/handlers/movement.rs`, `src/battle/construction.rs`, `src/battle/service.rs`)**:
   - `handle_move_dispatch` acquires `lock_player_operations` in ascending ID order around `handle_move_inner` for party leaders with online members (dropping the guard before `MeetDoor`/`WalkStep` so downstream `perform_party_warp_and_persist` cannot deadlock) and uses `registry_update_if_online(&conn.session)` for the leader.
   - Verified `BattleService::battle_ended` already holds `lock_player_operations(lock_ids).await` when syncing `quest_tasks[218]` and persisting `ended_sessions`, and `BattleConstruction::advance_wave` holds `lock_online_sessions()` only briefly for synchronous `upsert_quest_task_slot`.
3. **Tests (`tests/eve_quest/tickets_16_17.rs`)**:
   - `ticket17_e2e_event_npcs_15312_15313_61210_party_lock_registry_and_sqlite_persist` (including negative member-click rejection)
   - `ticket17_e2e_north_star_quiz_0x10_party_lock_registry_and_sqlite_persist`
   - `ticket17_e2e_movement_party_sync_updates_member_coords_safely`
   - `ticket17_negative_db_failure_rolls_back_ram_and_suppresses_relocate_frames`

---

### Ticket 18 ([`issues/18-event-session-preplan-index-startup-db-and-docs.md`](issues/18-event-session-preplan-index-startup-db-and-docs.md)) — `resolved`
1. **Pre-planned `EventSession::planned_effects` (`src/eve/auto_chain.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/eve_runner.rs`)**:
   - Added `planned_effects: Vec<DomainEffect>` to `EventSession`, populated once on open (`npc_event::activate` via `plan_chain`) and on `AutoChain` (`EveAutoChainEngine::try_auto_chain` via `plan_chain`).
   - Updated `eve_runner::execute_event_step_core` to read `ev.planned_effects` sequentially by `current_index` (including after battle resume), eliminating per-step `Vec<EveResult>` clones and repeated `plan_results` calls.
2. **Unified `u16` `eve_no` Index Lookup (`src/server/handlers/npc_event.rs`)**:
   - `npc_event::activate` reads `&[u16]` directly from `SceneEveData` indices (`npc_events_by_npc`, `door_events_by_door`, `encounter_events_by_id`), falling back to `&[u8]` (`u16::from`) only when unindexed, eliminating temporary `Vec<u8>` allocations and `u16 -> u8` narrowing casts.
3. **Startup SQLite Schema Sanity-Check (`src/db/pool.rs`, `src/main.rs`)**:
   - Added `db::pool::verify_schema(&pool.read)` called at startup in `src/main.rs` right after `db::pool::bootstrap`.
   - Verifies all core tables from `0001_init.sql` + `0002_eve_quest_persistence.sql`, `characters.{savemap_id, savemap_x, savemap_y}`, and `character_completed_events` composite PK `(playerid, scene_id, eventid)`. Returns `TsError::Migrate` with clear manual migration instructions per ADR 0004 when missing (including 0-table empty DBs).
4. **Synced `HANDOFF_01_10.md` (`.scratch/quest-refactor/HANDOFF_01_10.md`)**:
   - Updated title to `Tickets 01 through 10`, `Warps.dat` -> `Warps.txt`, `eve_runner.rs` extraction, `DomainEffect::Unsupported`, `EveCondition.to_result` + `results`, `rc=5` support, `0x46 BoatRace`/`0x41 Rank` dispatch status, and verified `Data/eve.emg` counts (`15` AND chains, `223` `cc=12 param != 0`, `rt=9 p_style ∈ {1, 2}`).
5. **Tests (`tests/eve_quest/ticket_18.rs`, `tests/db/startup_schema_check.rs`, and `EventSession` test fixtures)**:
   - Added `ticket18_event_session_preplanned_effects_drive_execution_and_auto_chain` and `ticket18_npc_event_uses_scene_u16_index_without_u8_truncation` in `tests/eve_quest/ticket_18.rs`.
   - Added 3 `verify_schema` tests (0-table DB, `0001`-only DB, full `0001 + 0002` DB) in `tests/db/startup_schema_check.rs`.

---

## Verification Record (Tickets 11–18)

- `cargo test --test eve_quest_test`: **129 passed, 0 failed**.
- `cargo test --test db_test`: **21 passed, 0 failed** (including 3 new `startup_schema_check` tests).
- `cargo test --test architecture_guardrails_test`: **8 passed, 0 failed**.
- `cargo test --test events_arena_test`: **61 passed, 0 failed**.
- `cargo clippy --all-targets`: **exit 0**.
- `cargo test --all-targets --no-fail-fast`: **exit 0** (all test binaries passed).
- **Unverified at runtime:** Live `aLogin.exe` TCP gameplay for `FField<N>` / `BKMusic` rendering from `geolBaseAtt`, repeatable-quest `AutoChain` feel, and live `Data/eve.emg` door/AND-chain quests end-to-end on `aLogin.exe`.

---

## Next Tickets for Continuation

All tickets **11 through 18** in `.scratch/quest-refactor/issues/` are now `resolved`.

## Suggested Skills

- `tdd` — For writing failing tests first / verifying E2E & negative tests per ticket in `tests/`.
- `code-review` — For auditing changes against `CODING_STANDARDS.md` and `spec-p0-p2.md` before handoff.
- `diagnosing-bugs` — If any regression or SQLite/concurrency edge case surfaces during Tickets 14–18.
