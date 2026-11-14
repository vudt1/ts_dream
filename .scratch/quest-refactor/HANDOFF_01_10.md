# Handoff: Quest Refactor — Tickets 01 through 08

## Summary of Completed Work

### Ticket 01 (`01-eve-binary-loader-and-default-on.md`) — `resolved`
1. **Binary Loader `Data/eve.emg` (`src/data/loaders/eve.rs`)**:
   - Fixed `TWinCondition` stride in `parse_fight_data_section` from `12` bytes to `13` bytes (`+0x04..+0x16`).
   - Preserved `BKMusic` (`pub bk_music: u8` on `EveFightData`) and removed `fight_limit` (`0` bytes on disk).
   - Fixed `EveResult` bounds check in `parse_npc_event_section` from `15` bytes to `14` bytes.
   - Propagated `parse_scene_data` and out-of-bounds offset errors in `EveDataLoader::load` instead of swallowing them; parser stops cleanly after Section 9 (`62007.eve` trailing 64 bytes ignored as expected).
   - Loads all `3,897` non-empty scenes from `Data/eve.emg` (including previously dropped scenes like `10951`, `10954`, and `12001` with 9 fights).
2. **Read-only Lookup Indices (`src/data/loaders/eve.rs`, `src/data/loader.rs`)**:
   - Added `SceneEveData::build_indices()` populating `npc_events_by_npc`, `door_events_by_door`, `encounter_by_id`, and `encounter_events_by_id`.
   - Added `GameData::eve_event(scene_id, eve_no)`, `eve_npc_events`, `eve_door_events`, and `eve_encounter_events`.
3. **Eve Engine Default-On & Removal of `.ini` / Fake Fallback (`src/server/handlers/npc_event.rs`, `src/server/handlers/talk.rs`, `src/data/loader.rs`)**:
   - Changed `EVE_EVENTS_ENABLED` default to `true`.
   - Removed `Quests/*.ini` loading from `GameData::load` (`load_talks` marked `#[allow(dead_code)]`).
   - Removed `data.talks` fallback and hardcoded fake `C830` talk frame from `handle_talk_start` / `handle_talk_continue` in `src/server/handlers/talk.rs` (falls back to `end_talk`).
4. **Tests**:
   - Added `tests/eve_loader_and_default_on_test.rs` (3 tests) and updated `tests/random_encounter_test.rs` (`bk_music: 0`).

### Ticket 02 (`02-item-0x17-vs-quest-ledger-0x18-mark.md`) — `resolved`
1. **Removed `quest_items (0x18)` (`src/server/session.rs`, `src/server/handlers/quest_sync.rs`, `src/server/auto_save.rs`, `src/db/modern/sqlite/session.rs`)**:
   - Removed `Session::quest_items`, `add_quest_item`, `remove_quest_item`, `clear_quest_item`, `QUEST_ITEM_STACK_CAP`, and `character_quest_items` load/save/fingerprint logic.
   - Note: `character_quest_items` table cleanup in `characters::delete` and migration `0002` belong to Ticket 03 (`03-sqlite-migration-0002-and-atomic-save.md`).
2. **Strict `quest_tasks` Slot Allocation `1..=200` (`src/server/handlers/quest_sync.rs`, `src/battle/construction.rs`, `src/server/handlers/event_npcs.rs`)**:
   - Added `quest_sync::upsert_quest_task_slot(session, quest_id, mark_step)` which strictly allocates/reuses slots in `1..=200` (repairing any legacy `slot == 0`).
   - Replaced all `slot = 0` insertions (`construction.rs` quest `218`, `event_npcs.rs` quests `212`, `12309`, `218`) with `upsert_quest_task_slot`.
   - `set_quest_task` refuses All-or-Nothing and emits `S→C 0x18 03` when all 200 slots are full.
3. **`quest_dont` `1..=2400` & `0x18 07` Bitmask & `0x18 08` Login Sync (`src/server/handlers/quest_sync.rs`)**:
   - Expanded `QUEST_DONT_MARK_MAX` to `2400` (`1..=2400`).
   - `sync_frames` packs `quest_dont` into `[byte_idx (1..=300): u16 LE][bitmask: u8]` via `0x18 07`, and emits `0x18 08` for active `ghost` (`kind=1`) and `god` (`kind=2`) states on login.
4. **Reject `C→S 0x18` (`src/server/dispatcher.rs`, `src/server/handlers/quest_sync.rs`)**:
   - Routed `OP_ITEM_INFO` (`0x18`) to `quest_sync::handle_client_quest_sync`, which logs `tracing::warn!` and rejects all `C→S 0x18` packets (including `0x18 01/02` from `TMissionForm`) for all accounts including GM (`gm_level > 0`) without mutating state or emitting quest frames.
5. **Tests**:
   - Rewrote `tests/quest_sync_18_test.rs` (7 tests) and updated `tests/npc_eve_e2e_test.rs` & `tests/db_repository_init_test.rs`.

---

### Ticket 03 (`03-sqlite-migration-0002-and-atomic-save.md`) — `resolved`
1. **SQLite Migration `0002` (`migrations/0002_eve_quest_persistence.sql`, `tests/common/test_db.rs`)**:
   - Rebuilt `character_completed_events` with `scene_id INTEGER NOT NULL DEFAULT 0` and primary key `(playerid, scene_id, eventid)` (migrating any existing rows with `scene_id = 0`).
   - Added `savemap_id`, `savemap_x`, `savemap_y` (`INTEGER NOT NULL DEFAULT 0`) to `characters`.
   - Dropped obsolete `character_quest_items` table.
   - Updated `tests/common/test_db.rs` to execute `0002_eve_quest_persistence.sql` after `0001_init.sql` on every schema reset.
2. **Session State & Scene-Scoped `completed_eve_counts` (`src/server/session.rs`, `src/server/handlers/quest.rs`, `src/server/handlers/talk.rs`, `src/server/handlers/npc_event.rs`)**:
   - Added `savemap_x`, `savemap_y`, and `bit_flags: HashSet<u32>` to `Session`, and `storage_type: PetStorageType` (`#[default] Carried = 1`) to `PetState`.
   - Updated `quest::save_map` to capture `map_id`, `map_x`, and `map_y` into `savemap`, `savemap_x`, and `savemap_y`.
   - Scoped `Session::completed_eve_counts` by `(u16, i32)` (`(scene_id, eve_no)`) across `talk.rs` and `npc_event::snapshot_state_with_context` so two scenes with the same `eve_no` never collide.
3. **Single-Transaction Atomic Save & Delete Cleanup (`src/db/modern/sqlite/session.rs`, `src/db/modern/sqlite/characters.rs`)**:
   - Merged `save_eve_state_tx` (`character_quest_tasks`, `character_quest_dont`, `character_bit_flags`, `character_completed_events`) into the single `SqliteSessionRepository::save` transaction before `tx.commit()`, propagating all SQL errors without swallowing `Err`.
   - `save_eve_state_tx` deletes stale `(scene_id, eventid)` rows from `character_completed_events` that were removed from `Session.completed_eve_counts` while preserving `completedat` on surviving rows.
   - Preserved `PetStorageType` (`Carried = 1`, `Cart = 2`, `Hotel = 3`, `Warehouse = 4`) in `load_pets` and `save`.
   - Wired `savemap_id`, `savemap_x`, `savemap_y`, and `character_bit_flags` into `load`, `save`, and `create_and_seed`.
   - Updated `SqliteCharacterRepository::delete` (both inherent and trait impls) to delete `character_quest_tasks` and `character_quest_dont` alongside `character_bit_flags` and `character_completed_events`.
4. **Auto-Save Fingerprint (`src/server/auto_save.rs`)**:
   - Updated `auto_save::fingerprint` to cover all persisted fields (`ghost`, `god`, `bit_flags`, `savemap`, `savemap_x`, `savemap_y`, `completed_eve_counts` with `scene_id`, `pet.quest`, `pet.storage_type`, `hotkeys`, stats, etc.) and removed transient `quest_steps`.
5. **Tests (`tests/sqlite_migration_0002_test.rs`, `tests/npc_eve_e2e_test.rs`, `tests/p0_blockers_test.rs`, `tests/db_repository_init_test.rs`)**:
   - Added `tests/sqlite_migration_0002_test.rs` (5 tests) covering migration `0002` schema, round-trip `savemap`/`bit_flags`/`PetStorageType`, scene-scoped `completed_eve_counts` & stale-row deletion, negative mid-transaction SQL error rollback + `SaveLedger` preservation, `characters::delete` orphan-free cleanup, and `auto_save::fingerprint`.
   - Updated `tests/npc_eve_e2e_test.rs`, `tests/p0_blockers_test.rs`, and `tests/db_repository_init_test.rs` for `(scene_id, eve_no)` keys, new `Session` fields, and atomic rollback contract.

---

### Ticket 04 (`04-snapshot-fail-closed-and-menu-filter.md`) — `resolved`
1. **Full `PlayerEventState` Hydration (`src/eve/state.rs`, `src/server/session.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/movement.rs`)**:
   - Added `bit_flags: HashSet<u32>` and `follow_pet_count: usize` to `PlayerEventState`, `role_count_values` & `bit_flags` inputs to `PlayerStateInputs`, and `role_count_values: HashMap<i32, i32>` to `Session`.
   - Added `npc_event::snapshot_state_with_data(session, data, active)` hydrating `homdo`, `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `role_count_values`, `bit_flags`, scene-scoped `completed_eve_counts`, `quest_tasks`, and `Mark.dat` `quest_id -> bit_id` projection for `quest_dont` (`cc=2 pStyle=3`) without polluting `mission_flags` with raw `bit_id` keys when `GameData` is supplied.
   - Wired `resolve_npc_event_with_filter`, `auto_chain_after`, and `movement.rs` to `snapshot_state_with_data(..., Some(data), ...)`.
2. **Fail-Closed Evaluator (`src/eve/evaluator.rs`)**:
   - Changed `compare_step` `_ =>` arm from `true` to `false` with `tracing::warn!` (fixes `condition_ops = 142` at `55100.eve eve=5 cond=14`).
   - Eliminated `actual = 0 -> 0 == 0 -> true` fallthroughs for unknown `condition_parameter` in `cc=7` and `cc=12`, and unknown `p_style` in `cc=7 param=5/6` and `cc=8`.
   - Implemented `cc=7 param=5` carried-pet capacity check (`p_style=1` full `>= 4`, `p_style=2` has space `< 4`) required by `Data/eve.emg` (`12001` Tống Nhân `eve=15` and Tống Dũng `eve=16`).
3. **`0x14 09` `ChainFilter` Menu Filtering (`src/eve/resolver.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/talk.rs`)**:
   - Added `resolver::chain_has_dialog_choice(chain, surface_id, choice_code)` and `npc_event::resolve_npc_event_with_filter`.
   - Updated `talk::handle_talk_select_menu` to validate `TSurface` choice codes (`20, 21, 40, 30..=39, 61..`) and pass `chain_has_dialog_choice` as `ChainFilter` so normal/unconditional chains never override menu choices and unmatched choices (e.g. `40` cancel) close cleanly with `14 2C Unlock` + `14 08 EndTalk`.
4. **Tests (`tests/snapshot_fail_closed_and_menu_filter_test.rs`, `tests/db_repository_init_test.rs`)**:
   - Added `tests/snapshot_fail_closed_and_menu_filter_test.rs` (4 tests) verifying full snapshot hydration + `Mark.dat` `cc=2 ps=3`, fail-closed evaluator (`condition_ops=142` at `55100.eve`, `cc ∈ {3,4,5,11,13,15,16,17}`, `cc=7`/`cc=12`/`cc=8` zero-fallthrough), `12136` Giản Ung & `12001` Tống Nhân menu branching + `40`/invalid cancel, and `12001` NPC 24 `FollowPet 14131`.
   - Updated `tests/db_repository_init_test.rs` for `Session::role_count_values`.

---

### Ticket 05 (`05-pure-planner-event-runner-and-preflight.md`) — `resolved`
1. **Pure Eve Planner (`src/eve/planner.rs`, `src/eve/mod.rs`)**:
   - Added pure, lock-free `EveTrigger`, `DomainEffect`, `PlannedChain`, `plan_result`, `plan_results`, and `plan_chain`.
   - `result_type >= 7` (except server-side `rt=9`) maps to `DomainEffect::Unsupported` and is never emitted raw via `0x14 01..06`.
2. **`Pre-Flight Probe All-or-Nothing` & Unified `EventRunner` (`src/server/handlers/talk.rs`, `src/server/session.rs`, `src/eve/auto_chain.rs`)**:
   - Added `Session::talk_locked` to emit `14 2C Lock` once across `AutoChain` (no unlock/lock flicker), skip locking when step 0 is `StartBattle`, and cleanly emit `14 2C Unlock` + `14 08 EndTalk` before mid-talk `StartBattle` or once at `FinishChain` (`NoMatch`).
   - Added `preflight_probe_batch` simulating `homdo (0x17)` (adds/removes in chunks) and `Quest Ledger (0x18)` 200-slot capacity before mutating any state; rejects All-or-Nothing (emitting `S→C 0x18 03` on full Quest Ledger) without touching `homdo`, `gold`, `quest_tasks`, or `quest_dont`.
   - Removed Bear's `result_class=2` save-gate, added support for `p_style ∈ {1, 2, 3, 4, 5}`, and restricted `quest_dont` (`0x18 05`) to true completion (`p_style=1, step=1`) or explicit mark actions (`p_style=3 val>0`, `p_style=5`).
   - Fixed `EveAutoChainEngine::detect_duplicate_items` to handle `p_style=0` with signed `result_value` (negative takes do not block subsequent gives).
   - Added atomic DB write-through (`persist_sessions_transaction`) at `FinishChain` (`NoMatch`), `StartBattle`, and `LearnSkill` (`08 01 6E`, free of `skill_point`).
3. **Tests (`tests/pure_planner_event_runner_and_preflight_test.rs`, `tests/npc_eve_e2e_test.rs`)**:
   - Added `tests/pure_planner_event_runner_and_preflight_test.rs` (4 tests) covering Pure Planner `rt >= 7` safety, `detect_duplicate_items` `p_style=0` negative, All-or-Nothing pre-flight rejection on 25-slot `homdo` & 200-slot `Quest Ledger` (`0x18 03`), step-0 `StartBattle` no-lock, `12001` Sĩ Triết turn-in (`10544` + `FollowPet 14131` + adjacent slot preservation), and `12136` Giản Ung (`0x14 09` -> free skill `14001` + `08 01 6E` + `Quest 10049` + `quest_dont` bit `27` + `FinishChain` DB write-through reload).
   - Updated `tests/npc_eve_e2e_test.rs` (`action_class2_saves_intermediate_step_without_marking_quest_dont`) for the removed Bear save-gate.

---

### Ticket 06 (`06-missing-eve-effect-executors.md`) — `resolved`
1. **Missing Eve Effect Executors (`src/server/handlers/talk.rs`, `src/protocol/codecs/npc_talk.rs`, `src/protocol/mod.rs`)**:
   - Implemented `NpcTeam (rt=0, rc=3)` emitting `S→C 0x14 01` for `p_style ∈ 1..=5` (with warning skip for unsupported styles).
   - Implemented `RewardPet (rt=0, rc=8)` via `execute_reward_pet`: allocates `1..=4` via `pet_box::next_active_slot`, hydrates `PetState` from `data.npcs` (or fallback defaults), emits `0x14 01` + `0F 08` + `0F 01` (or `0x14 01` + `0F 02` on removal), and calls `registry_update_if_online`.
   - Implemented `MoveEntity (rt=4)` (validated `rc == 3 && result_value >= 10_000` -> `0x14 01`), `PlayMovie (rt=5)` (`0x14 01`), and server-side `NpcAction (rt=9)` (never emitted raw via `0x14`). `UNKNOWN` `result_type`/`result_class` log `tracing::warn!` and skip fail-closed.
2. **`Player (rc=7)` & `Gold/Points (rc=5)` & `Pre-Flight Probe` (`src/server/handlers/talk.rs`)**:
   - Fixed `Player (rc=7)` fallthrough bug on `param == 3` and separated `savemap` (`param=0/1/3, p_style=1`) from `GetArmy (param=2)`.
   - Completed `texp` (`param=1, p_style=4`): computes `texp_get_lv_up`, updates `level`, `recompute_stats()`, heals `hp`/`sp` to max, grants `point` (`+2/lv`) & `skill_point` (`+1/lv`), emits `0x14 0x2A 0x01 <exp_delta: u32_LE>`, `0x08 01 24/23/CD/19/CE/1A/26/25`, and `05 0A`.
   - Added `POINT_CAP` (`src/protocol/mod.rs`) and extended `preflight_probe_batch` to check `GOLD_CAP`, `POINT_CAP`, `InsufficientGold`/`InsufficientPoint`, and 4-slot `RewardPet` capacity All-or-Nothing; `rc=5` emits `0x1A 01/02/04/05` + `0x1A 03` (`point_frame`).
3. **Tests (`tests/missing_eve_effect_executors_test.rs`)**:
   - Added 4 E2E tests covering `12001` Tống Nhân (`0x14 09` choice `32` -> `RewardPet 14105`, `NpcTeam`, `Quest 10027`, DB write-through), `Player rc=7` `texp` level-up + `savemap` + `param=3` no-fallthrough, `MoveEntity`/`PlayMovie`/`NpcAction`/`UNKNOWN` fail-closed, and negative All-or-Nothing rejection on `GOLD_CAP`, `POINT_CAP`, and full 4-slot pet bag.

---

### Ticket 07 (`07-door-triggers-walkstep-and-party-warp.md`) — `resolved`
1. **Door & WalkStep Triggers (`src/server/handlers/npc_event.rs`, `src/server/handlers/talk.rs`, `src/server/handlers/movement.rs`, `src/server/dispatcher.rs`)**:
   - Wired `NpcTrigger::ClickDoor` on `0x14 04 <door_id: u16_LE>` (`handle_talk_click_door`, preserving bare `F4 44 02 00 14 04` as `end_talk`).
   - Added `NpcTrigger::WalkStep { x, y }` to `NpcTrigger` (`kind = 16`, skipping pure `rt=3` random-encounter battles so step-counter encounters remain unaffected).
   - Added `movement::handle_move_dispatch` and `movement::handle_player_update_dispatch` (wired in `dispatcher.rs`) to trigger `NpcTrigger::MeetDoor` (closest door within ±60 px) and `NpcTrigger::WalkStep` into `EventRunner`.
2. **Party `Door Warp` (`Two-Phase State Lock` + Write-Through) (`src/server/handlers/quest.rs`, `src/server/session.rs`, `src/server/handlers/talk.rs`)**:
   - Added `session::registry_get` and `quest::perform_party_warp_and_persist`: validates `Warps.dat` (`map2 > 0`), acquires `lock_player_operations` **before** calling `registry_get(member_id)`, updates leader & online members' `(map_id, map_x, map_y)`, calls `registry_update_if_online`, emits `14 07` + `0x0C` + `hide_from_map`, and persists leader + all warped members atomically via `persist_sessions_transaction`.
   - Wired both `execute_event_step_with_pool` (`DomainEffect::Warp`) and `quest::handle_warp_confirm` (`0x14 08`) to `perform_party_warp_and_persist`.
3. **Tests (`tests/door_triggers_walkstep_and_party_warp_test.rs`)**:
   - Added 3 E2E tests covering `ClickDoor` party warp + DB write-through for Leader & Member (plus bare `0x14 04` EndTalk), `WalkStep` + `MeetDoor` party warp + DB persistence, and negative unmet door condition / invalid warp destination leaving Leader & Member RAM/registry/SQLite untouched.

---

### Ticket 08 (`08-eve-battle-sync-registry-and-disconnect.md`) — `resolved`
1. **`EveFightData` & Pre-Battle Talk Unlock (`src/battle/service.rs`, `src/battle/construction.rs`, `src/server/handlers/talk.rs`)**:
   - `BattleService::start_encounter_battle` propagates `fight_data.fight_type`, spawns `left_enemies` on Team 2 (rows `0..=1`) and `right_enemies` on Team 1 (rows `2..=3`), and falls back to terrain `112` when `SceneInfo.background_no == 0`.
   - Talk UI (`Session::talk_locked`) is cleanly unlocked (`14 2C Unlock` + `14 08 EndTalk`) before entering battle.
2. **Post-Battle `Two-Phase State Lock`, Registry Sync & Deferred Warp (`src/battle/service.rs`, `src/server/handlers/talk.rs`, `src/server/dispatcher.rs`)**:
   - `BattleService::battle_ended` collects sorted, deduplicated participant IDs (`battle_participants` + `members` + `offline_eve_sessions` + `active_40_npc`) and acquires `lock_player_operations(lock_ids)` before mutating sessions or reading registry.
   - Added `HandleOutcome::eve_warp` and `talk::resume_eve_after_battle_deferred_warp` so post-battle `DomainEffect::Warp` steps execute `quest::perform_party_warp_and_persist` **after** `drop(_op_guards)`, preventing re-entrant `lock_player_operations` deadlocks and lock-order inversions while preserving `direct_messages` and `map_broadcast`.
   - Calls `registry_update_if_online(&s)` for online participants, executes atomic SQLite write-through (`persist_sessions_transaction`) at `Battle End` (Win, Lose, Flee), and preserves `register_sender` session handle refresh.
3. **Mid-Battle Disconnect & Party Scope (`src/battle/service.rs`, `src/eve/auto_chain.rs`)**:
   - `unregister` and `leave_battle` snapshot parked `AwaitingBattle` Eve sessions into `offline_eve_sessions` so `battle_ended` still runs `resume_eve_after_battle_deferred_warp` and persists post-battle rewards/quests to SQLite for offline players.
   - Standard Eve quest progression runs only for solo players or the party Leader (`s.id_leader == 0 || s.id_leader == s.id`); party Members clear parked `AwaitingBattle` sessions without advancing standard Eve quests.
   - Updated `EveAutoChainEngine::try_auto_chain` to prioritize `cc=8` (`BattleResult`) chains when `completed.battle_result != 0` without skipping `completed.eve_no` via `completed_eve_counts`, ensuring `12001` Tống Dũng (`eve_no=16` `cond 7`) resolves `OnWin` (`RewardPet 14106`, `Quest 10028`) instead of falling through to `eve_no=83` (`50225`).
4. **Tests (`tests/eve_battle_sync_registry_and_disconnect_test.rs`)**:
   - Verified all 5 E2E tests: `12001` Tống Dũng win (`14106` + `NpcTeam` + `Quest 10028` + registry + movement packet safety + SQLite), negative Lose/Flee (no `[OnWin]` + SQLite persistence), party Member standard Eve quest preservation, mid-battle disconnect DB persistence for relogin, and `EveFightData` `right_enemies`/terrain `112` + post-battle `Warp` `direct_messages`/`map_broadcast`, plus `architecture_guardrails_test` and full `cargo test --all-targets --no-fail-fast`.

---

### Ticket 09 (`09-code-review-remediation.md`) — `resolved`
1. **Pre-Flight Probe All-or-Nothing & Item Sufficiency (`src/server/handlers/talk.rs`)**:
   - `preflight_probe_batch` continues through `DomainEffect::TalkStep` (`rt = 1`) instead of breaking early, checking all resource mutations (`MutateItem`, `UpdateQuest`, `RewardPet`, `MutateMoney`, `PlayerEffect`) until the chain ends or hits a real branch/transition (`OpenSurface`, `StartBattle`, `Warp`).
   - Validates `trial_homdo` has sufficient quantity for `MutateItem` removals (`delta < 0`) and filters `RewardPet` (`p_style = 2`) by `is_carried()` before mutating any state or emitting dialogue.
2. **Wire Protocol `0x1A`, `POINT_CAP` & `0x14 01` Notification (`src/protocol/mod.rs`, `src/server/handlers/talk.rs`)**:
   - Updated `POINT_CAP` to `2_000_000_000` (`GLOSSARY.md` parity).
   - Fixed `MutateMoney` (`rc = 5`): `p_style = 3` emits `0x1A 05 <add_u32_LE>` and `p_style = 4` emits `0x1A 06 <sub_u32_LE>` (no erroneous `0x1A 03` gold-cap alert).
   - `MutateItem` (`rt = 0, rc = 1, qty != 0`) emits `0x14 01` (`NpcTalkCodec::build_eve_result_hex`) alongside `dump_homdo()` (`0x17 05`).
3. **`PetStorageType` Isolation (`src/server/session.rs`, `src/server/handlers/pet_actions.rs`, `src/battle/service.rs`, `src/server/handlers/talk.rs`)**:
   - Added `PetState` storage helpers (`is_carried()`, `effective_storage_and_slot()`), restricted `BattleService::pet_slots` to `PetStorageType::Carried` (`stt` `1..=4`), and filtered `pet_actions` (`0x0F 03`/`0x1F 02`, `0x0F 07`/`0x1F 03`) and `execute_reward_pet` (`p_style = 2`) by `storage_type` while updating `pet.storage_type` on transfers.
4. **Two-Phase State Lock, `registry_update_if_online` & Atomic Multi-Session SQLite Tx (`src/server/handlers/talk.rs`, `src/battle/service.rs`, `src/db/persist.rs`, `src/server/handlers/quest.rs`)**:
   - Added `registry_update_if_online` calls after `rc == 1` (`homdo`), `rc == 2` (`quest_tasks`/`quest_dont`), and `BattleService` wave `218` updates.
   - Refactored `persist_sessions_transaction` to save all passed sessions inside a single SQLite transaction (`pool.write.begin()` -> save all -> `tx.commit()`).
   - Refactored `quest::perform_party_warp_and_persist` to hold `_guards` across SQLite commit, capture `before_state`, rollback leader RAM on DB failure, and only update registry / emit `14 07` + `0x0C` on commit success.
5. **Post-Warp Event Completion, Disconnect Warp Persistence & `WalkStep` `AutoChain` (`src/server/handlers/talk.rs`, `src/battle/service.rs`, `src/eve/auto_chain.rs`)**:
   - Both sync and async `execute_event_step` loops run trailing non-blocking effects and increment `completed_eve_counts[(scene_id, eve_no)]` when `DomainEffect::Warp` executes.
   - `BattleService::battle_ended` holds `_op_guards` across post-battle warp handling, syncs party members' `online_entries` `sess_arc` coordinates, and persists post-battle `Warp` coordinates for `offline_parked` sessions in the battle-end SQLite transaction.
   - Added `trigger_kind = 16` (`WalkStep`) to `AUTO_CHAIN_TRIGGER_KINDS`, `EveAutoChainEngine::event_list_for` (`scene.encounters`), and `talk::handle_talk_select_menu` (`0x14 09`).
6. **Cleanup, Codec Helpers, Evaluator Clarity & Deterministic Test Fixes (`src/db/modern/`, `src/eve/evaluator.rs`, `src/protocol/codecs/npc_talk.rs`, `tests/code_review_remediation_test.rs`, `tests/battle_combo_rules_test.rs`)**:
   - Removed dead `SqliteQuestRepository` (`src/db/modern/sqlite/quests.rs`, `QuestRepository` in `traits.rs` & `mod.rs`) and untracked `.agents/plugins/rea/{mcp_config_sample,plugin}.json`.
   - Replaced raw hex literals with named codec/helper functions, replaced `lock_online_sessions().insert(...)` in `tests/pure_planner_event_runner_and_preflight_test.rs` & `tests/sqlite_migration_0002_test.rs`, clarified reversed `Data/eve.emg` comparison helpers in `evaluator.rs`, and deduplicated repeated logic in `talk.rs` / `characters.rs`.
   - Added 7 integration tests in `tests/code_review_remediation_test.rs` (including async `BattleService` offline-warp persistence), updated `rt=0` frame filtering in `tests/npc_talk_multistep_test.rs`, and pinned `Battle::with_seeds(100, 12345, 42, 42, 42)` in `tests/battle_combo_rules_test.rs::test_8_combo_damage_130_percent_multiplier` to eliminate RNG flakiness.

---

### Ticket 10 (`10-smell-03-04-structural-refactor.md`) — `resolved`
1. **Type-Safe `DomainEffect` Sub-Enums & Removal of `raw_result: &EveResult` (`src/eve/planner.rs`, `src/eve/mod.rs`, `src/server/handlers/eve_runner.rs`)**:
   - Added `MoneyOp`, `QuestOp`, `PlayerOp`, and `PetOp` in `planner.rs`; `plan_result` parses `result_class` / `parameter_style` once so `preflight_probe_batch` and executors dispatch directly on typed `DomainEffect` variants without `execute_action_result_non_skill(raw_result: &EveResult)`.
2. **Typed `EventTriggerKind` (`src/eve/auto_chain.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/talk.rs`)**:
   - Replaced `EventSession.trigger_kind: i32` with `EventTriggerKind` (`ClickNpc = 1`, `ClickDoor = 4`, `MeetDoor = 8`, `WalkStep = 16`), eliminating catch-all branches across `auto_chain.rs`, `npc_event.rs`, `talk.rs`, and all Eve test suites.
3. **Separated `eve_runner.rs` & Unified `execute_event_step_core` (`src/server/handlers/eve_runner.rs`, `src/server/handlers/talk.rs`, `src/server/handlers/mod.rs`)**:
   - Extracted `EventRunner`, `PreFlightError`, `preflight_probe_batch`, and `DomainEffect` executors into `src/server/handlers/eve_runner.rs`.
   - Slimmed `src/server/handlers/talk.rs` to `0x14` transport/sub-opcode handlers with `pub use super::eve_runner::{...};` backward-compatible re-exports.
   - Unified the duplicate sync/async `execute_event_step` loops into a single `async fn execute_event_step_core`, with `run_sync_future` (`std::thread::scope` + `current_thread` runtime) backing the thin sync wrappers `execute_event_step` and `resume_eve_after_battle_sync_full`.

---

## Next Tickets for Continuation

All tickets `01` through `10` in `.scratch/quest-refactor/issues/` are now `resolved`.
- **Runtime / live client checks remaining:** Live `aLogin.exe` rendering of `0x14 01` on `MutateItem` and `0x1A 05/06` without `0x1A 03`, plus a dedicated 40-NPC multi-wave `advance_wave` + `battle_ended` test asserting `quest_tasks[218]` preservation in `sess_arc`.
- **Important note for production DB:** Existing `DB/ts_dream.db` files created from `0001_init.sql` must have `migrations/0002_eve_quest_persistence.sql` applied before running the updated binary (since `db::pool::migrate` is a no-op per ADR 0004).
