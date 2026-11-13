# Research & Gap Analysis: Eve Script Engine, NPC Talk (`0x14`), Quest Sync (`0x18`) & Quest Persistence (`ts_dream`)

**Target Directory**: `.scratch/quest-refactor/`  
**Scope**: Pure research and architecture design based strictly on primary sources in `src/`, `tests/`, `migrations/`, `Data/eve.emg`, `.scratch/quest-refactor/sample_data/`, `.scratch/special-event-refactor/data_files/eve/`, `.scratch/op-working/`, `client_pseudo_c/`, `docs/`, and `spec/`.  
**Status**: Zero Rust source, migration, or test files modified. Unverified protocol/data semantics are explicitly marked `UNKNOWN / NEEDS VERIFICATION`.

---

## Executive Summary

`.scratch/quest-refactor/` currently contains **only** `.scratch/quest-refactor/sample_data/` (10 legacy `.ini` quest files); no existing refactor tickets or specs are present there.

Across the four primary-source domains (`Data/eve.emg` + `.ini` samples, `aLogin.exe` Ghidra C/ASM, Rust Eve/handler pipeline, and SQLite persistence), we identified **five systemic root causes** preventing smooth, 100%-compatible Eve quest execution and reliable persistence:

1. **Corrupted Binary Eve Ingestion (`1,120 / 3,901` Scenes Silently Dropped)**:
   [`src/data/loaders/eve.rs:914-920`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L914-L920) skips `win_cond_count * 12` bytes and reads a non-existent `fight_limit: u32`, whereas `Data/eve.emg` uses **13 bytes per `win_cond`** and **0 bytes for `fight_limit`**. Because [`eve.rs:352`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L352) ignores parse errors via `if let Ok(...)`, **1,120 of 3,901 scenes** are silently discarded (reducing `69,404` conditions / `150,098` results / `22,833` fights in `Data/eve.emg` to `47,491` / `94,890` / `2,247` in `.scratch/special-event-refactor/data_files/eve/*.tsv`, and corrupting fight definitions in loaded scenes such as `12001.eve`).
2. **Production Runtime Disabled by Default & Broken Legacy Fallback**:
   [`EVE_EVENTS_ENABLED`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L84) defaults to `false` unless `TS_EVE_EVENTS=1|true` is set ([`src/main.rs:103-105`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L103-L105)), while [`GameData::load()`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L155-L161) skips `load_talks()` and `load_npc_on_map()`. In default production mode on normal maps, both Eve and legacy `Data_Talks` are inactive, falling back to a hardcoded raw stub frame ([`talk.rs:863-866`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L863-L866)).
3. **Incomplete Eve Condition/Result Coverage & Broken Context Hydration**:
   - `evaluator.rs` supports only `9` of `17` raw `condition_class` values (`0, 1, 2, 7, 8, 9, 10, 12, 14`); unsupported classes/parameters (`7` param != `{0,2,3,6}`, `12` param != `0`) fall through to `actual = 0`, evaluating `0 == 0 -> true` instead of failing.
   - `npc_event.rs:142-192` hardcodes empty `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `raw_flags`, and `mark_defs`, ignores `role_count_values`, and puts `quest_dont` (`1..=300`) directly into `mission_flags` keyed by `quest_id`—while [`quest_sync.rs:197-212`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest_sync.rs#L197-L212) wrongly caps `0x18 05` at `300` instead of `Mark.dat` `bit_id (1..=2400)` vs `0x18 07` (`1..=300` byte bitmasks). So `condition_class = 2 pStyle = 3` never reads `quest_dont` back.
   - `talk.rs` skips `result_type` `4, 5, 7, 8, 9, 10` and `result_class` `0, 3 (NpcTeam), 5, 6, 8 (RewardPet), 9..22, 80`, restricts `result_class = 2` with a hardcoded Bear save-gate that fires `quest_dont` on every saved step, silently drops `homdo` (`0x17`) overflow without All-or-Nothing pre-flight checks, discards `map_broadcast`/`direct_messages` in `resume_eve_after_battle` ([`talk.rs:525-527`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L525-L527)), and lacks a `ConditionClass::DialogChoice` filter on `handle_talk_select_menu` ([`talk.rs:1097`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1097)).
4. **Client Wire Parity Gaps vs `aLogin.exe` (`client_pseudo_c/`)**:
   - Reverse engineering `FUN_0078ec3f` (`0x14` S->C), `FUN_005eb530` (15-byte `EveResult` client executor), and `FUN_00790ed5` (`0x18` S->C) + `Data/Mark.dat` proves two foundational protocol facts:
     - **Client `0x18` (`0x01/0x02/0x04/0x06`, `LocalActor + 0x654`, 200 slots) is purely the `Mark.dat` (`10001..=12461`) Active Quest Ledger**, NEVER an `Item.dat` quest-item bag (`session.quest_items` in [`session.rs:294-296`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L294-L296) is a false premise). All `Item.dat` items go through `homdo` (`0x17`), while `0x14 01 (rt=0, rc=1)` shows an `Item.dat` toast only.
     - **`0x18 05` (`FUN_00721088`) accepts `Mark.dat` `bit_id (1..=2400)`** (`position` reaches `1673`) and sets/clears bit `(bit_id - 1) % 8` in the 300-byte array `LocalActor + 0x8b2`, while **`0x18 07` (`FUN_0072ba54`) sets full byte masks** `[byte_idx (1..=300): u16][bitmask: u8]`.
   - The client executes ONLY `result_type` `0..6` locally (`0=Action` `[1=item toast only, 2=local 200-slot Mark.dat quest log write, 3=NPC anim, 8=pet dialog]`, `1=Talk`, `2=Warp prep`, `3=Battle trigger -> 0x0B`, `4=Move entity`, `5=Movie`, `6=Surface/Menu UI`). Server-only `result_type`s (`7, 8, 9, 10`) and stat/pet/skill/inventory rewards require the server to execute mutations and emit explicit sync opcodes (`0x18`, `0x17`, `0x16`, `0x08`, `0x1A`, `0x0C`).
   - `ClickDoor` and `MeetDoor` triggers are dead in Rust (`NpcTrigger::ClickDoor`/`MeetDoor` are never constructed), and `0x1A` `handle_pc_talk` is a warning-only stub.
5. **Non-Atomic SQLite Persistence, Cross-Scene `eve_no` Collisions & Registry/Party Races**:
   - `SqliteSessionRepository::save` ([`src/db/modern/sqlite/session.rs:402-415`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L402-L415)) commits core tables in Transaction 1, opens Transaction 2 for `save_eve_state`, swallows errors with `tracing::debug!`, and returns `Ok(())`.
   - `character_completed_events` ([`migrations/0001_init.sql:238-244`](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql#L238-L244)) keys by `(playerid, eventid)` globally without `scene_id`, causing cross-map collisions because `eve_no` in `Data/eve.emg` is scoped per scene.
   - `Session.savemap`, `quest_steps`, `warp_steps`, `completed_quests`, and `character_bit_flags` are not persisted; `SqliteQuestRepository` is dead code; non-skill Eve actions have no write-through (180s autosave window); `BattleService::battle_ended` ([`src/battle/service.rs:482-551`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L482-L551)) lacks `lock_player_operations` and fails to call `registry_update_if_online`, letting the next TCP packet overwrite post-battle rewards; party warps/quests omit member locking and persistence; and `arena`/`event_npcs` corrupt `quest_tasks` with invalid `slot = 0`.

---

## Part 1: Primary Source Inventory & Verification Method

| Source | Path / Scope Inspected | Key Verification |
|---|---|---|
| Quest Refactor Dir & Samples | [`.scratch/quest-refactor/sample_data/`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/sample_data/) | Contains 10 `.ini` files (`12001` NPC 24/1/2, `12136` NPC 1 step 0..4); no other ticket/spec files exist yet in `.scratch/quest-refactor/`. |
| Binary Eve Archive | `Data/eve.emg` (`9,839,580` bytes, `3,901` scenes) | Verified exact section sizes directly against raw bytes (`win_cond=13B, fight_limit=0B` aligns `3,900/3,901` scenes to exact `pos + size`). |
| Pre-dumped TSV Census | [`.scratch/special-event-refactor/data_files/eve/eve_logic.tsv`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_logic.tsv) (+ `eve_scene_index`, `eve_npcs`, `eve_surface`, `eve_groups`, `eve_fights`) | Cross-checked against `Data/eve.emg`; proved TSV was truncated by the `eve.rs` Section 9 bug. |
| Client Ghidra Dump | [`client_pseudo_c/`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/) (`FUN_0078a89c`, `case_018` `FUN_0078ec3f`, `FUN_005eb530`, `case_021` `FUN_00790ed5`, `FUN_0077f414` + ASM builders, `case_009`, `011`, `012`, `016`, `019`, `020`, `023`, `037`) | Direct disassembly + C decompilation verification of S->C and C->S wire layouts and client local state mutations. |
| Spec & Docs | [`docs/client-opcodes.md`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md), [`spec/exhaustive_by_opcode/opcode_{14,18,1A}_full_payloads.md`](file:///D:/VUDT/GIT_PCC/ts_dream/spec/exhaustive_by_opcode/), [`.scratch/op-working/`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/op-working/), [`CODING_STANDARDS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md) | Checked claimed parity docs vs actual code and tests. |
| Rust Server Codebase | [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs), [`src/eve/*`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/), [`src/server/handlers/{talk,npc_event,quest,quest_sync,movement,event_npcs}.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/), [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs), [`src/db/modern/sqlite/*`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/), [`migrations/0001_init.sql`](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql) | Line-by-line audit of evaluator, resolver, auto-chain, handlers, battle/party hooks, SQLite persistence, and guardrails. |

---

## Part 2: `Data/eve.emg` Binary Loader & Sample `.ini` Analysis

### 2.1 Critical Binary Bugs in `src/data/loaders/eve.rs`

`Data/eve.emg` begins with `scene_count: u16` (`3,901`), followed by `scene_count × 32B` directory entries (`name_len: u8`, `name_bytes: [u8; 23]`, `position: i32`, `size: i32`), then each scene payload starting at `entry.position + 103` across 9 sections ([`src/data/loaders/eve.rs:316-391`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L316-L391)).

Two binary discrepancies exist in `eve.rs`:

1. **Section 9 (`FightData`) Stride/Field Bug ([`eve.rs:914-920`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L914-L920))**:
   - `eve.rs:914-917` skips `win_cond_count * 12` bytes, and `eve.rs:920` reads `fight_limit: u32` (4 bytes).
   - **Actual `Data/eve.emg` binary format**: each `win_cond` record is **13 bytes** (not 12), and `fight_limit` (4 bytes) **does not exist**.
   - Direct byte check across all `3,901` scenes in `Data/eve.emg`: with `win_cond = 13B` and `fight_limit = 0B`, **3,900 of 3,901 scenes** end at `offset == pos + size` to the exact byte (the sole exception, `62007.eve`, parses cleanly through `fcnt = 0` with 64 trailing bytes after `fcnt = 0`).
   - Because [`EveDataLoader::load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L352) swallows `Err` via `if let Ok(scene) = ...`, **1,120 scenes are silently dropped** (e.g., `10951.eve`, `10963.eve`, `10964.eve`, `10965.eve`, `10984.eve`), and loaded scenes with fights suffer misaligned `fight_datas` (e.g., `10954.eve` reads `fight_limit = 65793` from shifted bytes; `12001.eve` has **9 fights** in `Data/eve.emg`, whereas `eve_fights.tsv` shows only 2 rows with the 2nd corrupted as `eve_no = 256`).
2. **`EveResult` Guard Mismatch ([`eve.rs:127, 797`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L797) vs `803-810`)**:
   - Comment at `:127` and bounds check `result_count * 15 > reader.remaining()` at `:797` assume **15 bytes**, while `:803-810` actually read **14 bytes** (`2+1+1+1+2+1+4+2 = 14`). Real `Data/eve.emg` records are **14 bytes** (the extra `0x00` byte in `0x14 0x01` wire frames is injected on the wire, not stored in `eve.emg`).

### 2.2 Exhaustive Census: Truncated TSV Dump vs Full `Data/eve.emg`

| Metric | `eve_*.tsv` (Generated by buggy `eve.rs`) | `Data/eve.emg` (Full Parse: `win_cond=13B, fight_limit=0B`) | Impact / Notes |
|---|---:|---:|---|
| Directory entries | — | **3,901** | All 3,901 numeric, `pos > 0, size > 0` |
| Non-empty scenes | 2,777 | **3,897** | **1,120 scenes dropped** by `eve.rs` |
| Scenes with `NpcEventData` | 2,772 (`eve_logic.tsv`) | **3,892** | 1,120 dropped |
| `NpcEventData` events | 13,738 | **23,659** | `when_happen`: `[T,F,F,F]=10530`, `[F,T,F,F]=9989`, `[F,F,F,T]=3127`, `[T,T,F,F]=9`, `[F,F,T,F]=2`, `[F,F,F,F]=2` |
| `EveCondition` entries | 47,491 | **69,404** | `71` distinct raw `(cc_u8, pstyle)`; `CTimeValue != 0`: `11` |
| `EveResult` entries (non-empty) | 94,890 | **150,098** | `156` distinct raw `(rt_u8, rc_u8, ps)` combinations |
| `EveNpcPlacement` (`id > 0`) | 12,897 | **21,534** | — |
| `EveDoorPlacement` (`id > 0`) | 6,201 | **9,887** | `ClickDoor` / `MeetDoor` unused in Rust |
| `EveEncounterPlacement` | 522 | **1,918** | — |
| `EveSurfaceData` (`id > 0`) | 2,068 | **2,285** | `sentence` `(data_kind, style)`: `(1,1):2072, (1,2):14, (1,3):5089, (1,4):2275, (2,3):1420, (2,4):1, (3,3):154` |
| `EveSceneInfo` (`eve_no > 0`) | — | **8,882** | — |
| `EveGroupData` (`eve_no > 0`) | 4,128 | **7,969** | `use_mode 1:3968, 2:3271, 3:730` |
| `EveFightData` | 2,247 | **22,833** | `fight_events=45,383`, `win_conds=45,773`; `16` distinct `fight_type` values (`1:22285, 2:157, 32:52, 34:140, 7:88`, etc.) |
| `GoodsData` (currently skipped) | — | **1,174** | `13B` each |

Raw `condition_class` (`69,404` total), `result_type` (`150,098`), and `result_class` (`150,098`) distributions in `Data/eve.emg`:
- **`condition_class` (`u8`)**: `0=16731`, `1=4292`, `2=25023`, `3=807`, `4=45`, `5=181`, `7=7984`, `8=5780`, `9=1889`, `10=5715`, `11=77`, `12=703`, `13=110`, `14=5`, `15=4`, `16=42`, `17=16`.
  - `condition_ops`: `0:35637, 1:2929, 2:6557, 3:88, 4:806, 5:21844, 6:1542, 142:1`.
- **`result_type` (`u8`)**: `0=49728`, `1=61458`, `2=10336`, `3=22815`, `4=210`, `5=240`, `6=2571`, `7=691`, `8=351`, `9=1600`, `10=98`.
- **`result_class` (`u8`)**: `0=23118`, `1=8604`, `2=9595`, `3=82918`, `4=35`, `5=430`, `6=9`, `7=24753`, `8=560`, `9=1`, `10=9`, `11=35`, `12=21`, `13=1`, `15..22=1 each`, `80=1`.

### 2.3 Sample `.ini` Quests vs `Data/eve.emg` (`eve_logic.tsv`)

All 10 `.ini` files in [`.scratch/quest-refactor/sample_data/`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/sample_data/) encode `Dialogs=` using `F4 44 [len:u16_LE] 14 01 [15B]` (15B = `00 00 00 [result_no:u8] [result_type:u8] [result_class:u8] [param:u16_LE] [pstyle:u8] [result_value:i32_LE] [result_mean_no:u16_LE]`) except one `F4440200140A` frame. Cross-referencing all 4 sample quests against `eve_logic.tsv` / `eve_npcs.tsv` / `eve_surface.tsv` / `eve_groups.tsv` / `Data/eve.emg` proves the `.ini` files are **lossy legacy/hand-authored quest overrides**, NOT canonical mirrors of `eve.emg`:

1. **`12001 bắt sơn tặc sĩ triết.ini`** (`MapId=12001, NPC Id=24`):
   - `eve_npcs.tsv`: `place_id=24, npc_id=19023, event_bytes=36,40`.
   - `eve=40 cond=1` (`QuestStep 10544 ps=2 ops=0 val=0`): `rno=1 Talk NpcTeam(3) mean=16531` matches `.ini` frame `[0]`, **but** `rno=2` in `eve.emg` is `Surface Player(7) param=0 mean=21`.
   - **Discrepancy**: `.ini` injects frame `[1]` (`Talk Player(7) mean=16532`, which in `eve_surface.tsv` is an *option sentence inside `surface_id=21`*, not a Talk step) and encodes `Surface 21` as `NpcTeam(3) param=24` instead of `Player(7) param=0`. `.ini` also omits `cond=2..15` (`FollowPet 14131/14158`, gold/quest continuation) and `eve=36` (post-step dialogue).
2. **`Giản Ung Khảo Nghiệm 12136 NPC 1 step 0..4.ini`** (`MapId=12136, NPC Id=1`, `eve=2`):
   - Note: Uses **Eve Dialog Choice / Surface menus (`result_type=6`, `SelectMenu=30/32/32/31`)**, NOT `0x10` `TAC_Question` timed quiz!
   - **Discrepancies vs `eve=2`**:
     - Step 0 `.ini` includes `[OnWin] SaveLeaderQuests=AUTO`, whereas `eve_logic.tsv` `cond=11` (`DialogChoice 1 ps=30`) ONLY opens `Surface 2` (it does NOT complete quest `10049` at step 0).
     - Step 1 & Step 2 in `eve` emit `Talk 10628` **AND** open next `Surface 3` / `Surface 4` (`SelectMenu=32` -> `cond=12`/`15`).
     - Step 3 `[OnWin]` matches `cond=18` (`SelectMenu=31` -> 3 `Talk`s `10600, 10601, 10632`, `AddSkill 14001`, `Quest 10049 ps=1 val=1`, `result_group=3 use_mode=2` = execute all members).
     - Step 4 `Talk 10592` simplifies `result_group=1` (`use_mode=1, members=1,2, prob=50,50` — 50/50 random between `10592` and `10632`).
     - All multi-frame `.ini` dialogs reset `result_no=1` instead of `1, 2, 3`, and `.ini` ignores `eve=2 cond=1..7` (`QuestStep 10021` + fight `12136.eve fight[0]` against 11× NPC `12039`) on the same NPC.
3. **`Tống Nhân 12001 NPC 1 step 0..1.ini`** (`MapId=12001, NPC Id=1`, `eve=15`):
   - Step 0 uses `Surface 1` + `SelectMenu=32` (`cond=8`), giving `AddPet=14105` and `Talk 10177`; `eve_logic.tsv` additionally executes `Action NpcTeam 1 ps=3 val=30`, `Action Quest 10027 ps=1 val=1`, and `Action Player 2 ps=1 val=5` (`rno=2,4,5`).
   - Step 1 (`Talk 10614`) simplifies `result_group=6` (`20%/40%/40%` weighted random across `10343, 10613, 10614`). Same NPC also hosts `eve=84` (`QuestStep 12396/12397` + fight `eve_no=9`), omitted by `.ini`.
4. **`Tống dũng 12001 NPC 2 step 0..1.ini`** (`MapId=12001, NPC Id=2`, `eve=16`):
   - Step 0 `Surface 6` -> `SelectMenu=30` (`cond=6`) -> Battle (`eve_no=1`): `.ini` `[TEAMDEF] Diahinh=628, Npcs=0\t0\t14106\t0...` (0-based slot 2) aligns with `Data/eve.emg` `12001.eve fight[0]` (`left=[(no=1, npc_id=14106, location_pos=3, ai=1)]`, 1-based slot 3), and `[OnWin]` aligns with `cond=7` (`RewardPet 14106` + `Action NpcTeam 2 ps=3 val=30`, `Quest 10028 ps=1 val=1`, `Player 2 ps=1 val=5`).
   - **Discrepancy**: Step 1 `.ini` has `Dialogs=F4440200140A` + `[TEAMDEF]`, whereas `eve_logic.tsv` `eve=16 cond=1` (`QuestStep 10028 >= 1`) has `result_group=8` (`30%/70%` random `Talk 10341 / 10612`). Same NPC also hosts `eve=83`.

---

## Part 3: Client Protocol Parity (`aLogin.exe` / `client_pseudo_c/`)

Verified directly against [`client_pseudo_c/`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/) C decompilations and raw x86 ASM (necessary where Ghidra C removed nested `FUN_0077f414` switch arms).

### 3.1 Main Opcodes Involved in Quest/Talk Flows

| Opcode | Direction | Client Function / ASM | Exact Verified Behavior |
|---|---|---|---|
| `0x14` | C->S | `FUN_0077f414` (`asm_cs14_78301f_builders.asm.txt` L4–187) | Only 4 active builders: **`sub 1/2/3`** -> `[0x14, *(u8*)(0xa089), *(u16_LE*)(0xa085)]` (sets `0xa094=1`; guarded if `FUN_004c9bf0` in `5..8`); **`sub 4`** -> `[0x14, *(u8*)(0xa08e), *(u16_LE*)(0xa08a)]` (EndTalk/Door, sets `0xa094=1`); **`sub 6`** -> `[0x14, 0x06]` (Continue, sets `0xa094=1`); **`sub 9`** -> `[0x14, 0x09, *(u8*)(0xa104)]` (Menu/InterForm choice; does NOT set `0xa094=1`). Sub `0,5,7,8,>=0x0A` are no-ops. |
| `0x14` | S->C | `case_018_0078EC3F_FUN_0078ec3f.c` (`0x00..0x38`) | **`0x01..0x06`** (`L64–121`) ALL jump to `0x0078ed7d`: copy **15B** `P[1..15]` into `player+0xa09b..0xa0a9`, clear `0xa094=0`, call `FUN_005eb530`. **`0x07`**: `0xa097=1, 0xa094=0, 0xa096=1`. **`0x08`**: EndTalk (`0xa095=0`, closes `TInterForm`, clears `0xa076,0xa097,0xa096,0xa094,0x653,0xa0fc,0xa100,0xa085`, resets `0xa089` timer). **`0x09`**: `0xa094=0`. **`0x0A,0x0B,0x0D..0x11`**: `0xa094=0, 0xa096=1`. **`0x0C`**: `0xa076=1, 0xa094=1`. **`0x21`/`0x22`**: update `+0x62c` / `+0x628,+0x62a`. **`0x2A`** (`FUN_005f16f4`): ExpUP/status toast. **`0x2C`** (`FUN_0074641c`): `[id:u32_LE][1=lock/2=unlock]`. **`0x31..0x38`**: pet/special system toasts & timers (`+0x1481`). **`0x12..0x20,0x23..0x29,0x2B,0x2D..0x30,0x37`**: toast strings + `WA0014.wav`. |
| `0x18` | S->C | `case_021_00790ED5_FUN_00790ed5.c` (`0x00..0x08`) | Always calls `FUN_006221e8(gvar_007DA34C, map_id)` post-switch to refresh NPCs/quest markers (`L224`)! **`0x01..0x04, 0x06` (`Mark.dat` `gvar_007D9FD8` ONLY — NOT `Item.dat`!)**: **`0x01`**: `[quest_id:u16][delta:u8]` add quest entry in `player+0x654` 200-slot `[id:u16][val:u8]` + `Mark.dat` toast (`FUN_00720ca8`); **`0x02`**: `[quest_id:u16][delta:u8]` sub + delete if 0 (`FUN_00720df0`); **`0x03`**: capacity-full toast; **`0x04`**: `[quest_id:u16]` delete from `0x654` (`FUN_00720f00`); **`0x06`**: bulk active quest dump, `4B` records `[slot:u8][quest_id:u16][mark_step:u8]` (`FUN_0072bb6c`). **`0x05`**: `[bit_id:u16 (1..=2400)][val:u8]` (`FUN_00721088` L136-174 checks `Mark.dat` and sets/clears bit `(bit_id-1)%8` in byte `0x8b2 + (bit_id-1)/8`, e.g. `Data/Mark.dat` has `2,394` records with `pos` up to `1673`). **`0x07`**: bulk `quest_dont` byte-mask dump, `3B` records `[byte_idx:u16 (1..=300)][bitmask:u8]` -> `player+0x8b2+(byte_idx-1)` (`FUN_0072ba54`). **`0x08`**: `[actor_id:u32][action:u16][state:u8]` (`FUN_00729a88`, `action==1` updates `+0x448/+0x36` & `WA0006.wav`). |
| `0x18` | C->S | `FUN_0077f414` (`@00785226`), `005d5ae0_FUN_005d5ae0.c` | `sub 1/2` originates from dev/GM test form `TMissionForm` (`editorBG`/`btn_module`): `[0x18, 0x01|0x02, *(u16_LE*)(TMissionForm+0x130), *(u8*)(TMissionForm+0x134)]`. Server `dispatcher.rs:414` currently routes `0x18` C->S to `unimplemented::handle` (and per ADR 0009, C->S `0x18 01/02` must be rejected even for GM). |
| `0x10` | S->C & C->S | `case_016_0078E593_FUN_0078e593.c`, `asm_cs10_7819f1_builders.asm.txt` | Timed Quiz (`TAC_Question`, `TKC_Question`, `TWF_day55*`): S->C `0x10 0x01` `[q_len][q][l1][o1][l2][o2][l3][o3][o4]`, `0x10 0x02` `[result]`; C->S (`00781a9a`/`00782c6f`) sends `[0x10, subop, answer:u32_LE]`. Distinct from Eve `Surface` menus (`0x14 0x09`). |
| `0x08` | S->C & C->S | `case_009_0078D125_FUN_0078d125.c` | S->C `0x01` player stat `[prop:u8][op:u8][delta:u32][new:u32]` (+ level-up UI if `player[0x43e] > 0xc`), `0x02` pet stat, `0x03..0x08`. |
| `0x0B` | S->C & C->S | `case_011_0078D5D1_FUN_0078d5d1.c`, `asm_cs0b` | Battle protocol. Note `FUN_005eb530` L681 (`result_type == 3`) calls `FUN_0077f414(gvar_007D9D30, 0x0B)` directly (NOT `0x14 0x0B`!). |
| `0x0C` | S->C & C->S | `case_012_0078D84D_FUN_0078d84d.c` | S->C has **no sub-opcode**: 12B `[char_id:u32][map_id:u16][x:u16][y:u16][dir:u16]`. C->S: `[0x0C, 0x01]`. |
| `0x16` | S->C & C->S | `case_019_0078FEAF_FUN_0078feaf.c` | S->C `1..10`: e.g. `0x02` `[slot:u16][x:u16][y:u16]` -> `FUN_007127bc`, `0x06` `[slot:u16][val:u8]`. C->S `sub 1`: `[0x16, 0x01, slot:u16]`. |
| `0x1A` | S->C ONLY (`OP_MONEY_SYNC`) | `case_023_0079175F_FUN_0079175f.c`, [`.scratch/client-pseudo-op-code/opcode_1a.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_1a.md) | **100% one-way S->C Money/Resource sync** (`FUN_0077f414` `case 0x1a` is empty; `0/110` C->S calls): `1/2` `+/- Gold` (`+0x12F8`), `3` gold-full banner, `4` set `Gold`+`Shop Points` (`FUN_0072bcf8`), `5/6` `+/- Shop Points` (`+0x1300`), `7` static flag banner (`P[2]==1`), `8/9/10` Thủy binh / Tài bảo / Đạn dược. Rust `npc_event::handle_pc_talk` is a mistaken legacy stub misreading S->C payloads as C->S talk selectors. |

### 3.2 How `aLogin.exe` Executes the 15-Byte `EveResult` (`FUN_005eb530`)

When server sends `0x14 0x01..0x06` + `15B` (`b[0..15]`), `FUN_0078ec3f` writes `0xa09b..0xa0a9` (`0xa09f = result_type`, `0xa0a0 = result_class`, `0xa0a1 = parameter:u16`, `0xa0a3 = pstyle:u8`, `0xa0a4 = value:i32`, `0xa0a8 = mean_no:u16`) and calls `FUN_005eb530` ([`client_pseudo_c/005eb530_FUN_005eb530.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c)):

- **`result_type == 0` (`L217–539`)**:
  - `class == 1` (`L220–239`): **Toast ONLY via `gvar_007DA540` (`Item.dat`)** (no inventory mutation — ALL actual `Item.dat` additions/removals expect `0x17` `homdo` packets from server).
  - `class == 2` (`L240–394`): **Mutates client local 200-slot `Mark.dat` (`gvar_007D9FD8`) quest array `0x656 + slot*3`** (`pstyle 1` adds, `pstyle 2` subtracts `(u8)val`) + toast. Note: this shares the same `0x654` 200-slot array updated by `0x18 01/02/04/06` — NOT an item bag!
  - `class == 3` (`L395–480`): NPC sprite/motion/scale (`pstyle 1..5`: `FUN_0071a160`, `FUN_00712940`, `FUN_00712a88`, etc.).
  - `class == 8` (`L483–537`): `MessageBoxA` pet add/remove notification (`pstyle 1|2`).
  - *Note*: `class 4 (Skill)`, `class 5`, `class 6`, `class 7 (Player gold/stats)` are **NOT handled** in `FUN_005eb530` — server must execute them and emit `0x08` / `0x1A` / skill / pet sync packets!
- **`result_type == 1` (Talk, `L540–661`)**: Shows dialog `mean_no` with speaker resolved from `result_class` (`3=NPC slot param`, `7=Player`, `0xC=Pet`) and coordinates from `gvar_007DA6DC[param]`.
- **`result_type == 2` (Door prep, `L662–669`)**: Sets `0xa096=1` and copies `0xa0dc`/`0xa070` into `0xa08a`/`0xa08c` (`0xa0aa`/`0xa0ac`). Actual teleport requires S->C `0x0C`!
- **`result_type == 3` (Battle trigger, `L670–682`)**: Stores `*(player + mean_no*4 + 0x9a10)` in `0xa0d0`, sets `0xa094=1`, calls `FUN_0077f414(..., 0x0B)`. *(Client `CL` sub-opcode at L681 needs ASM verification.)*
- **`result_type == 4` (Move entity, `L683–740`)**: When `result_class == 3`, splits decimal `result_value` into `x` (prefix) and `y` (last 4 digits) and calls `FUN_007127bc(entity, x*20 - 10, y*20)`. *(Note: `Data/eve.emg` has `210` entries of `rt=4, rc=3` currently unhandled by Rust `talk.rs`!)*
- **`result_type == 5` (Movie, `L741–766`)**: Plays movie ID `result_value` via `TMovie_Create`.
- **`result_type == 6` (Surface UI, `L767–2501`)**: Looks up surface pointer at `player + mean_no*4 + 0x9d34`; switches `+(0x74)`: `1` opens `TInterForm` grid (sends `0x14 0x09 <0x14..0x1D>`), `2` opens secondary form (`0x005eee28/005eed2c`), `0` renders dialog option buttons (`ChoiceCode`).
- **`result_type >= 7` (`7, 8, 9, 10` in `Data/eve.emg`)**: Not in `FUN_005eb530` switch (`L216–2502`). Server-executed only; never forward raw `rt >= 7` via `0x14 0x01`.

**Wire Framing Subtlety**: Server [`NpcTalkCodec::build_talk_step_hex`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs) emits `F4 44 11 00 14 01 00 [14B EveResult]`, matching `.ini` `00 00 00 [12B]` and putting the leading `00` into `0xa09b`, so `0xa09f..0xa0a9` align with `EveResult` bytes `3..13`.

---

## Part 4: Rust Eve Engine & Handler Audit (`src/`)

### 4.1 Evaluator & State Hydration Gaps (`src/eve/evaluator.rs`, `src/eve/state.rs`, `src/server/handlers/npc_event.rs`)

1. **Missing `ConditionClass` Coverage (`evaluator.rs:34-235`, `eve.rs:41-69`)**:
   - Missing classes: **`3` (`807`x in `eve.emg`), `4` (`45`x), `5` (`181`x), `11` (`77`x), `13` (`110`x), `15` (`4`x), `16` (`42`x), `17` (`16`x)** -> `warn!` and `false`.
   - **Silent `0 == 0 -> true` Fallthrough Bug (`evaluator.rs:124-131, 194-205`)**: For `condition_class = 7` (`PlayerAttribute`) with `param != 0|2|3|6` and `condition_class = 12` (`SceneEventCount`) with `param != 0`, `actual` defaults to `0` and still passes into `compare_step(0, ops, value)` — so `ops=0|1, value=0` falsely succeeds!
   - **Arg Order & Semantics Inconsistencies**: `BagItem` (`1`) and `PlayerAttribute` (`7`, params `0,2`) pass `(expected, ops, actual)` to `compare_step`, whereas `QuestStep` (`2`), `PlayerAttribute` (`3`), `SceneEventCount` (`12`), and `RoleCount` (`14`) pass `(actual, ops, expected)`. Additionally `Data/eve.emg` contains `QuestStep` `p_style ∈ {4,5,6,7,9,10}`, `FollowPet` `p_style ∈ {2,3}`, `SceneEventCount` `p_style ∈ {2..6}`, and `1` occurrence of `condition_ops = 142`.
2. **Broken `snapshot_state_with_context` Hydration (`npc_event.rs:142-192`)**:
   - `follow_npc_ids: &[]`, `inn_npc_ids: &[]`, `cart_npc_ids: &[]` (`184-186`) are **hardcoded empty** -> `FollowPet (9)` and `PlayerAttribute param=6` ALWAYS see 0 pets.
   - `role_count_values` (`condition_class = 14`) is omitted from `PlayerStateInputs` -> ALWAYS `0`.
   - `bag_slots` (`146-151`) reads `session.homdo` (which actually matches `aLogin.exe` semantics where ALL `Item.dat` items live in `homdo 0x17`; the stale `session.quest_items` field in `session.rs:294-296` is based on a mistaken `0x18` premise and must be removed).
   - `raw_flags: &[]`, `mark_defs: &HashMap::new()` (`174-175`), while `:188-190` inserts `session.quest_dont` (`mark` `1..=300`) directly as keys of `state.mission_flags`, which `evaluator.rs:64-68` indexes by `quest_id` (e.g. `10037`). Meanwhile [`quest_sync.rs:197-212`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest_sync.rs#L197-L212) wrongly clamps `0x18 05` `mark` at `300`, rejecting real `Mark.dat` `bit_id`s (`301..=1673+`), and emits `0x18 07` without byte bitmask packing. **Key-space & bit-packing mismatch**: `quest_dont` marks never match `QuestStep p_style = 3`.
   - `EveStateBuilder::find_fallback_talk` (`state.rs:156`) and `is_within_range` (`state.rs:173`) are dead code (`talk.rs:115,122` hardcodes `±150` bounding box instead).

### 4.2 Resolver & `AutoChain` Bugs (`src/eve/resolver.rs`, `src/eve/auto_chain.rs`, `src/eve/group.rs`)

1. **`chain_filter` Unused (`resolver.rs:76-80, 152` & `talk.rs:1097`)**:
   `resolve_event` hardcodes `chain_filter = None`. When `handle_talk_select_menu` (`0x14 0x09`) fires, it does NOT require the matched chain to contain `ConditionClass::DialogChoice(surface_id, choice_code)`, allowing higher-`step_score` or equal-score earlier non-menu chains to shadow the user's menu selection!
2. **`detect_duplicate_items` `parameter_style` Bug (`auto_chain.rs:218-232`)**:
   Checks `r.parameter_style == 1`, missing real `eve.emg` item grants/takes where `parameter_style == 0` and `result_value` is signed (`249` occurrences in `eve.emg`).
3. **`GroupData` `pick_member` Edge Case (`group.rs`)**:
   When `group_datas.get(&group_id)` is `None`, falls back to `KEEP_ALL`; `pick_member > 1` with `use_mode == 1` vs `use_mode == 2/3` needs complete parity verification across all `7,969` `GroupData` records (`use_mode 1:3968, 2:3271, 3:730`).

### 4.3 Handler, Execution & Battle-Integration Bugs (`talk.rs`, `npc_event.rs`, `movement.rs`, `battle/service.rs`)

1. **Missing Trigger Wiring**:
   - `talk.rs:85-99`: Only handles `0x14` subs `1, 4, 6, 8, 9`; `NpcTrigger::ClickDoor` (kind `4`) and `NpcTrigger::MeetDoor` (kind `8`) are never constructed (`movement.rs` only rolls random encounters; `talk.rs` routes unknown subs to `end_talk`).
   - `npc_event.rs:67-82`: `0x1A` `handle_pc_talk` is a warning-only stub.
2. **Missing `result_type` & `result_class` Executors (`talk.rs:197-305, 564-745`)**:
   - Unhandled `result_type`: `4` (`MoveEntity`, `210`x), `5` (`Animation`/movie, `240`x), `7` (`691`x), `8` (`351`x), `9` (`NpcAction`, `1600`x), `10` (`98`x).
   - `result_class = 1` (Item, `:572-585`): Mutates `session.homdo` (`0x17`) ONLY (correct per `aLogin.exe`!), but ignores `parameter_style` and silently drops overflow when `homdo` (`25` slots) is full instead of rejecting All-or-Nothing pre-flight; meanwhile `quest_sync::sync_frames` and `quest_sync::remove_quest_item`/`clear_quest_item` mistakenly emit `0x18 01/02/04` for `session.quest_items` (polluting the client's `Mark.dat` Quest Log).
   - `result_class = 2` (Quest, `:589-645`): Uses hardcoded Bear save-set (`(ps==1 && step in {1,2,3,10,30,50,70}) || ps==4 || (ps==2 && step in {1,9})`), skipping `ps=5` etc.; adds `step` blindly on revisit; and fires `set_quest_dont` (`18 05`) on **every** saved step whenever `quest_marks.get(&quest_id)` exists rather than only on quest completion.
   - `result_class = 3` (`NpcTeam`, `15,713+` `ps=3`, `13,072+` `ps=4`, etc.) & `8` (`RewardPet`, `560`x): **completely skipped** (`:737-743`). Notice `NpcTeam` is the **#1 most common Action class in `Data/eve.emg`** (`29,858` actions across `ps=1,3,4,5`!) — e.g., `12001` NPC `1` & `2` both emit `Action NpcTeam param=npc ps=3 val=30` alongside pet reward.
   - `result_class = 5` (Gold/Points, `:648-680`): `gold` addition omits `GOLD_CAP` check.
   - `result_class = 7` (Player, `:682-734`): Fallthrough bug on `param == 3` (`open_npc_shop_frame()` then logs skipped AND if `p_style == 1` triggers `save_map`); `param == 1, p_style == 4` adds `texp` with **no level-up / stat growth / HP-SP restore**; `param == 2` and `param >= 4` skipped.
   - Duplicate sync/async `execute_event_step` (`:310-338`) vs `execute_event_step_with_pool` (`:342-404`): Sync fallback drops DB `upsert_skill`.
3. **Talk/Battle/Warp Flow Bugs**:
   - `handle_talk_start` (`:835-842`) emits `06 02` + `14 2C Lock` even when `results[0]` immediately starts a battle (`out.eve_battle`).
   - `finish_event_session` (`:412-436, 452-470`) emits `14 2C Unlock` + `14 08 EndTalk` *before* `auto_chain_after`, causing unlock/lock flicker if `auto_chain` succeeds.
   - `resume_eve_after_battle` (`:525-527`) copies `out.frames`, **discarding `out.direct_messages` and `out.map_broadcast`**, breaking post-battle party warps/animations!
   - `BattleService::start_encounter_battle` (`src/battle/service.rs:1049-1182`) ignores `right_enemies`, `fight_type` (`16` variants in `eve.emg`), `link_to_result`, and all `45,383` `fight_events` / `win_conds`.
4. **Docs vs Code/Test Discrepancies in `.scratch/op-working/`**:
   - `handoff-opcode-14-18-checkpoint-1-2-3-4-5-6.md` & `plan-checkpoint-6-remediation-and-parity.md` claim AutoChain on loss (`p0_blockers_test.rs:289-305` only tests `NoMatch`), claim `Mark.Dat` `quest_dont` sets completion bit cleanly (contradicted by `npc_event.rs:188-190` mismatch), claim real Trác Quận E2E parity (`checkpoint6_parity_test.rs:475-512` only checks `resolve_npc_event` non-empty without executing any step/battle/warp), and describe `TS_EVE_EVENTS=1` as optional when production `loader.rs` leaves `data.talks` empty.

---

## Part 5: SQLite Persistence, Autosave, Battle & Party Concurrency Defects

### 5.1 Persistence & Schema Gaps (`migrations/0001_init.sql`, `src/db/`)

1. **Split-Transaction Non-Atomicity & Swallowed Errors ([`src/db/modern/sqlite/session.rs:120-126, 297-415`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L297-L415))**:
   `save()` commits Transaction 1 (`characters`, `character_money`, `inventories`, `character_pets`, `character_skills`, `character_hotkeys`) at `:402`, then runs `save_eve_state` in a **second transaction** (`:407, 424-512`) and swallows `Err` with `tracing::debug!`, returning `Ok(())`. `auto_save::run_cycle` ([`auto_save.rs:157-161`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/auto_save.rs#L157-L161)) then advances `SaveLedger`, silently losing quest state on failure/crash.
2. **Global `eve_no` Collision Keying (`migrations/0001_init.sql:238-244`, `session.rs:176-186, 488-509`, `evaluator.rs:185-192`)**:
   `eve_no: u16` in `Data/eve.emg` is scoped **per `scene_id`**, yet `Session.completed_eve_counts` and `character_completed_events` (`PRIMARY KEY (playerid, eventid)`) omit `scene_id`, causing completions in one map to satisfy/pollute `conditionClass=12` in other maps. `save_eve_state` also never deletes entries removed from `completed_eve_counts`.
3. **No Write-Through on Eve Progression**:
   Only `StepOutcome::SkillReward` writes through (`talk.rs:374`). Items, `quest_tasks`, `quest_dont`, `completed_eve_counts`, gold, stat/skill points, `texp`, and Door warps stay purely in RAM for up to 180s ([`auto_save.rs:20`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/auto_save.rs#L20)), risking rollback on crash.
4. **Unpersisted Session Fields & Dead Repository**:
   - `Session.savemap` (`server/session.rs:266`), `quest_steps`, `warp_steps`, `completed_quests` (`:315`), and `click_npc_id` have **no DB schema** and vanish on logout/restart (`auto_save::fingerprint` even hashes `quest_steps` despite `save()` dropping it!).
   - `SqliteQuestRepository` (`src/db/modern/sqlite/quests.rs:10-137`) is dead code (`character_missions`, `character_mission_flags`, `character_bit_flags` are never loaded or saved by `SqliteSessionRepository`; `mark_event_completed` omits `completioncount`).
   - `SqliteCharacterRepository::delete` ([`characters.rs:42-75, 163-205`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/characters.rs#L42-L75)) omits `character_quest_tasks`, `character_quest_dont`, and `character_quest_items`, leaving orphan rows.
5. **Pet Storage Corruption & Fingerprint Gaps**:
   - `session.rs:247-251, 363-367` collapses `PetStorageType` `Cart (2)`, `Hotel (3)`, and `Warehouse (4)` -> overwrites `Cart`/`Warehouse` back as `storagetype = 3`.
   - `auto_save::fingerprint()` (`auto_save.rs:38-124`) omits `ghost`, `newbie`, `pk`, `job`, `reborn`, `hair`, `guild_id`, `hotkeys`, base stats, and `pet.quest`. `login.rs` also omits `0x18 Sub 0x08` (`send_actor_state_flag`).
   - `arena` / `event_npcs.rs` (`construction.rs:158-165`, `event_npcs.rs:372, 446, 565`) write `quest_tasks.insert(..., (0, step))` with invalid `slot = 0` (`1..=200` invariant broken, overwriting/clashing with client UI and persisting `(questid=218, slot=0, markstep=0)` instead of deleting).

### 5.2 Battle, Party & Concurrency Guardrail Violations (`CODING_STANDARDS.md` §1–2)

1. **`BattleService::battle_ended` Violates Registry & Lock Guardrails ([`src/battle/service.rs:355-552`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L355-L552))**:
   - Runs without `lock_player_operations`.
   - Copies `s` into `lock_online_sessions()` at `:413-416` **before** running `quest_lose_frames` (`:493`) and `battle_quest_win` (`:523`), and **never calls `registry_update_if_online`** afterwards. The next packet in `server_control.rs:726-732` reloads the pre-`battle_quest_win` snapshot and wipes post-battle quest/item/skill/warp changes in RAM.
   - `register_sender` (`:790-791`) leaves `BattleService.online[id].session` stale.
   - Disconnecting during an Eve battle skips `resume_eve_after_battle` (`:421-435`), dropping battle outcomes permanently.
   - Under `CurrentThread` runtime (`:378-386`), falls back to `pool = None` sync path, skipping skill DB write-through.
2. **Party Warps & Quests Omit Locking, Member Sync & Member Persistence**:
   - `SaveMemberQuests` is parsed (`loader.rs:1001`, `tables.rs:318`) but **never read anywhere in `src/`**.
   - `perform_warp` (`quest.rs:883-907`), `battle_quest_win_impl` (`quest.rs:522-528`), and `event_npcs.rs` (`15312`, `15313`, `61210`) mutate party members directly in `lock_online_sessions()` or `BattleService.online` without `lock_player_operations([min..max])` and persist ONLY the leader (`&[&conn.session]`).

---

## Part 6: Recommended Architecture & Algorithm for 100% Eve Quest Support

To eliminate ad-hoc branches, ensure crash-safe SQLite durability, and handle all `23,659` Eve events / `150,098` results cleanly, we recommend refactoring the module around a **Pure Eve Planner + Explicit Effect Outbox + Atomic Two-Phase Executor**.

### 6.1 Layered Module Boundaries

1. **Layer 1 — Binary Ingestion & Scene-Indexed Catalog (`src/data/loaders/eve.rs`)**:
   - Fix `parse_fight_data_section`: `win_cond = 13B`, `fight_limit = 0B` (no `u32`), `EveResult` guard = `14B`. Propagate errors instead of `if let Ok(...)` so all `3,900+` scenes load reliably.
   - Normalize `SceneEveData` with fast read-only indices keyed by `(scene_id, eve_no)` (and retain `npc_id -> Vec<eve_no>`, `door_id -> Vec<eve_no>`, `encounter_id -> Vec<eve_no>`).
   - Unify production flag: promote Eve as primary engine (`EVE_EVENTS_ENABLED` default `true`, removing `.ini` overrides — settled in ADR 0009).
2. **Layer 2 — Pure Eve Planner (`src/eve/`) — Zero I/O, Zero Tokio Locks**:
   - **Typed Triggers**:
     `enum EveTrigger { ClickNpc(u16), ClickDoor(u16), MeetDoor(u16), WalkStep, MenuChoice { surface_id: u16, choice_code: u8 }, BattleEnd { eve_no: u16, fight_eve_no: u16, outcome: BattleOutcome } }`
   - **Complete `PlayerEventSnapshot` (`npc_event.rs` builder fixed)**:
     - Read `session.homdo` (`0x17`) for all `Item.dat` `bag_counts(item_id)` (removing stale `session.quest_items`).
     - Populate `follow_npc_ids` (equipped/following pet), `cart_npc_ids`, `inn_npc_ids`, `role_count_values`, `completed_eve_counts` keyed by `(scene_id, eve_no)`, `quest_tasks[quest_id]` (slots `1..=200`), `quest_dont` bitmask (`bit_id 1..=2400`) mapped via `Mark.dat` (`quest_id <-> bit_id`), and `bit_flags`.
     - Remove silent `actual = 0` fallback in `evaluator.rs`: unsupported `(cc, param)` MUST return `EvalResult::Unsupported` -> `false` (fail-closed with telemetry warning), never `0 == 0 -> true`.
   - **Deterministic Resolution with `ChainFilter`**:
     - `MenuChoice { surface_id, choice_code }` passes `ChainFilter::RequireDialogChoice(surface_id, choice_code)` into `resolver::resolve` (impossible for non-menu chains to shadow menu choices).
     - `BattleEnd` evaluates chains matching `ConditionClass::BattleResult(outcome)`.
   - **Normalized `Vec<DomainEffect>` Plan (Single Interpretation Point)**:
     Translate winning chain's `GroupData`-selected `EveResult`s (including `rt=4,5,7,8,9,10` and `rc=0..80`) into typed `DomainEffect`s:
     - *Client Presentation*: `SendEvePacket(u8 /*0x01..0x06*/, EveResult)` (ONLY for client-handled `rt 0..6` where needed), `LockActor`/`UnlockActor`, `EndTalk`, `ShowSystemToast`.
     - *World/NPC Visuals*: `MoveNpcOrEntity` (`0x16 0x02`), `NpcAction` (`rt=9`), `PlayMovie` (`rt=5`).
     - *State Mutations (Leader for standard Eve; locked party members for Warps/special map events)*:
       - `AddOrRemoveItem { item_id, delta }` (strictly in `homdo 0x17` with Pre-flight probe All-or-Nothing capacity check before mutation)
       - `SetQuestTask { quest_id, step, mode }` / `ClearQuestTask { quest_id }` (allocating unified slot `1..=200` strictly via `quest_sync`, never slot `0`; if 200-slot Quest Ledger is full, reject All-or-Nothing and emit `0x18 03`)
       - `SetQuestDont { bit_id, val }` (`0x18 Sub 05` with `bit_id 1..=2400` / login bulk byte-mask `0x18 Sub 07`, gated ONLY on explicit quest completion or explicit mark action, NOT on intermediate steps)
       - `SetBitFlag { flag_id }` (`character_bit_flags`)
       - `IncrementSceneEventCount { scene_id, eve_no }`
       - `AddOrRemovePet { npc_id, action }` (`0x14`/`0x19` pet sync)
       - `LearnSkill { skill_id, rank }` / `MutateStats { attr, delta }` (`0x08` + level-up check on `texp`)
       - `MutateMoney { gold_delta, point_delta }` (enforcing `GOLD_CAP` / `POINT_CAP`)
       - `SetSaveMap { scene_id, x, y }`
     - *Control Flow Transitions*: `AwaitMenuChoice { surface_id }` (stops step loop and unlocks choice UI), `StartBattle { fight_eve_no }` (terminates dialog cleanly *before* battle transition), `Warp { map_id, x, y, dir, include_party }`, `FinishChain { allow_auto_chain }`.

### 6.2 Unified Execution & Auto-Chain State Machine (`EventRunner`)

Replace fragmented `handle_talk_start` / `handle_talk_continue` / `handle_talk_select_menu` / `resume_eve_after_battle` / `auto_chain_after` flows with a **single loop** running after Pure Planning:

```text
Trigger -> Build PlayerEventSnapshot -> Plan Initial Chain + Group Selection
  loop (depth <= MAX_CHAIN_DEPTH):
    1. Apply state mutations in-memory on locked target session(s)
       (if target includes party: acquire lock_player_operations(sorted_ids) first!)
    2. Inspect terminating / suspending result at head of queue:
       - Talk / Animation / Action: advance cursor; flush wire syncs (`0x18`, `0x17`, `0x08`, `0x14 01..06`); if queue empty -> step 3
       - AwaitMenuChoice(surface_id): send 14 2C Lock (only once) + Surface frame, set EventPhase::AwaitingChoice, COMMIT DB CHECKPOINT, break
       - StartBattle(fight_no): close/unlock any open talk UI cleanly FIRST, COMMIT DB CHECKPOINT, hand off to BattleService, break
       - Warp(target): emit post-state sync + `0x0C` warp for leader & locked party, COMMIT DB CHECKPOINT, reset talk context, break
    3. Queue finished (`FinishChain`):
       - Increment `(scene_id, eve_no)` completion count if state-changing
       - Attempt `auto_chain::next_chain` (fixing `detect_duplicate_items` for `pstyle == 0`!)
       - If NextChain found (and NOT StartBattle/Door): stay locked without Unlock/Lock flicker and loop!
       - Else: COMMIT DB CHECKPOINT, emit `14 2C Unlock` + `14 08 EndTalk` ONCE, break
```

### 6.3 Atomic Two-Phase DB Persistence & Registry Sync

1. **Single-Transaction `SqliteSessionRepository::save`**:
   Merge `save_eve_state` directly into the main SQLite transaction (`tx`). **Never swallow errors**; propagate `Err` so `auto_save` does NOT advance `SaveLedger` on failure.
2. **Schema Updates (`0002_quest_eve_parity.sql`)**:
   - Migrate `character_completed_events` to `PRIMARY KEY (playerid, scene_id, eventid)` (backfilling `scene_id = 0` or mapped scene for existing rows as needed).
   - Add `savemap_id, savemap_x, savemap_y` columns to `characters` (and wire `character_bit_flags` hydration/save into `Session` + `SqliteSessionRepository`, deleting dead `SqliteQuestRepository` or delegating cleanly through `SqliteSessionRepository`).
   - Fix `SqliteCharacterRepository::delete()` to cascade-delete `character_quest_tasks`, `character_quest_dont`, and drop obsolete `character_quest_items` writes.
   - Fix `PetStorageType` persistence (`Cart=2, Hotel=3, Warehouse=4` stored losslessly).
   - Update `auto_save::fingerprint()` to hash all persistent fields (`ghost`, `god`, `bit_flags`, `savemap`, `completed_eve_counts`, `pet.quest`, etc.) and stop hashing unpersisted transient vecs.
3. **Critical Write-Through Checkpoints & Registry Discipline**:
   - Run atomic `persist_sessions_transaction` immediately at the 3 settled checkpoints (**FinishChain**, **Battle End (Win/Lose/Flee & Offline!)**, and **Door Warp (Leader + Party)**).
   - Refactor `BattleService::battle_ended`: acquire `lock_player_operations(sorted_participant_ids)` *before* touching sessions; apply `quest_lose_frames` / `battle_quest_win` / `resume_eve_after_battle`; update `lock_online_sessions()` via `registry_update_if_online` for every participant; update `BattleService::register_sender` to always refresh the stored session handle; and queue pending post-battle rewards cleanly if a player disconnected mid-battle.

---

## Part 7: `UNKNOWN / NEEDS VERIFICATION` Matrix & User Confirmation Questions

Because guessing binary meanings is forbidden, every unverified item from `Data/eve.emg`, `.ini`, and `client_pseudo_c/` is listed explicitly below for confirmation or packet-capture/binary verification:

### 7.1 Unknown Semantics in `Data/eve.emg` & `.ini` Samples
1. **Unmapped Raw `EveConditionClass` (`8 / 17` codes)**:
   - `cc = 3` (`807`x, e.g. `pstyle=4`: `(10951, eve=33, cond=1)`), `cc = 4` (`45`x), `cc = 5` (`181`x), `cc = 11` (`77`x), `cc = 13` (`110`x), `cc = 15` (`4`x), `cc = 16` (`42`x), `cc = 17` (`16`x). Also `condition_ops = 142` (`1` occurrence).
2. **Unmapped Raw `EveResultType` & `EveResultClass`**:
   - `rt = 4` (`210`x — `client_pseudo_c/005eb530` L683 proves `rt=4 rc=3` moves entity on map via `FUN_007127bc`, need confirm `val` XY packing for all `210` cases), `rt = 7` (`691`x, mostly `rc=7 Player`), `rt = 8` (`351`x), `rt = 10` (`98`x).
   - `rc ∈ {0, 5 (430x), 6 (9x), 9 (1x), 10 (9x), 11 (35x), 12 (21x), 13 (1x), 15..22 (1x each), 80 (1x)}`, plus exact sub-actions of `rc=3` (`NpcTeam` `pstyle=1,3,4,5`) and `rc=7` (`Player` `param >= 2`).
3. **Unimplemented `EveFightData` Fields**:
   - Semantics of all `16` distinct `fight_type` bitflags/enums, `link_to_result`, and the `45,383` `fight_events` / `45,773` `win_conds` (`13B` layout parsed by length, inner fields un-decoded).
4. **`.ini` Specifics**:
   - `F4440200140A` in `Tống dũng 12001 NPC 2 step 1.ini`: **Already specified** in [`.scratch/client-pseudo-op-code/opcode_14.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L78) (`0x14 0x0A` sets transition lock `0xa096=1` and clears pending `0xa094=0`).
   - Exact mapping of `[TEAMDEF] Diahinh=628` to battle background / terrain IDs remains to be confirmed.

### 7.2 Unverified Client Branches (`client_pseudo_c/`) & Dedicated Research Backlog
*(Note: Full standalone research backlog for all genuinely unverified items — excluding `14 0A` and `0x1A` which are verified in `opcode_14.md` / `opcode_1a.md` — is tracked separately at [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md).)*
1. `0x14` C->S / Surface: What `ChoiceCode` bytes are written to `0xa104` by non-`TInterForm` dialog option callbacks `0x005eecf4`, `0x005eecbc`, `0x005eee28`, `0x005eed2c` (no `.c` files dumped for those addresses).
2. `0x14` `FUN_005eb530` L681: Exact `CL` sub-opcode register value passed to `FUN_0077f414(..., 0x0B)` on `result_type == 3`, plus 4 unrecovered sub-event helper bodies (`0x2A`, `0x32`, `0x33`, `0x38 sel=2`) noted in `opcode_14.md`.
3. `0x18` S->C `case 8 action == 2` (`FUN_00729a88` L100+). *(Note: `0x18` S->C `case 5` `FUN_00721088` and `0x18` C->S `TMissionForm` `005d5ae0` are now fully verified.)*
4. `0x10` C->S exact sub-opcode LUT at `0x00781a06` (`0x00..0x24`) and full `0x17` (`case_020_FUN_007902fb` L151–421).

### 7.3 Finalized Architecture Decisions (Settled via `/grill-with-docs` -> ADR 0009, Spec & Tickets)

All design questions (`Q1..Q13`) have reached shared understanding and are formally recorded in [`docs/adr/0009-eve-quest-engine-and-persistence.md`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md), [`GLOSSARY.md`](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md), [`spec.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md), and [`.scratch/quest-refactor/issues/`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/issues/):

1. **Canonical Quest Source (`Q1`)**: 100% `Data/eve.emg` (`EVE_EVENTS_ENABLED = true` default; remove `.ini` overrides after fixing `eve.rs` Section 9).
2. **Unknown Raw Eve Codes (`Q2`)**: Fail-closed (`false`/skip + telemetry warning); genuinely unverified codes are tracked in [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md) for a dedicated future research session.
3. **Containers & `0x18` `Mark.dat` Bitmask (`Q3 + Q11 + Q12`)**:
   - Remove `session.quest_items` / `0x18` item routing completely; route 100% of `Item.dat` items (including quest items `rc=1`) through `homdo` (`0x17`, 25 slots) with Pre-flight probe All-or-Nothing.
   - Align `0x18 05` (`bit_id 1..=2400` from `Mark.dat.position`) and `0x18 07` (`[byte_idx 1..=300: u16][bitmask: u8]`), setting `quest_dont` ONLY on actual quest completion or explicit mark action.
   - Reject C->S `0x18 01/02` (`TMissionForm`) unconditionally including GM; reject All-or-Nothing + emit `0x18 03` if the 200-slot `Mark.dat` Quest Ledger is full.
4. **DB Migration & Atomicity (`Q4`)**: New SQLx migration (`0002_quest_eve_parity.sql`) updating `character_completed_events` PK to `(playerid, scene_id, eventid)`, adding `savemap` columns, wiring `character_bit_flags`, fixing `PetStorageType` and `delete()` cascade, and merging `save_eve_state` into the single atomic `save()` transaction.
5. **Party Scope & Concurrency (`Q5 + Q7`)**: Leader-only advances standard Eve quests & persists DB (party `quest_tasks` sync kept only for special map events in `event_npcs.rs` with valid `1..=200` slots); party Door Warps acquire `lock_player_operations([min..max])` and persist `(mapid, x, y)` for all locked participants.
6. **Write-Through Checkpoints & Battle Disconnect (`Q6 + Q8`)**: Immediate atomic SQLite write-through at `FinishChain`, `Battle End`, and `Door Warp`; if a player disconnects mid-Eve battle, `Battle End` still runs `resume_eve_after_battle` (win/lose/flee effects) and writes DB directly for the offline character while refreshing online registry handles via `registry_update_if_online`.
7. **Execution Plan & Tickets (`Q13` -> `/to-spec` -> `/to-tickets`)**: Specification finalized in [`spec.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md) and split into 8 vertical-slice tickets (`01`..`08`) under [`.scratch/quest-refactor/issues/`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/issues/).
