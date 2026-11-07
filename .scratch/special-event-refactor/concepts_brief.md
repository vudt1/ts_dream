# Special Events & Guild Subsystem Refactoring

* **Author:** tuan.vu
* **Date:** 2026-11-03

## Overview

This brief defines the architectural and implementation strategy for the Special Events engine and the Guild (Quân đoàn) subsystem in the `ts_dream` Rust game server, achieving protocol parity with the legacy PC client `aLogin.exe`.

Special events in TS Online (including Competitive Arena / Lôi Đài, 40 NPC, Rescue 2K, 9 Tiers of Nine Heavens / Chinh Phục Cửu Thiên, Dragon Boat Racing, Water War, and City Siege) operate under a server-authoritative push model:
1. Scheduled or Trigger-Based Lifecycle: The server executes scheduled timers or processes manual Web Admin commands (Port 8090) to transition events through explicit states, broadcasting binary alert frames to connected client sessions.
2. Scripted Multi-Stage Progression: Several events (2K, 40 NPC, Cửu Thiên) run via the `eve.emg` script engine using `0x14` dialog frames and turn-based combat (`0x0B`).
3. Guild Prerequisite: Competitive warfare events (City Siege, Water War) strictly require players to belong to a registered Guild (*Quân đoàn*).

The current Rust server lacks guild database tables, session guild state, an event lifecycle runner, web admin event control APIs, and wire handlers for these event opcodes (currently falling through to `unimplemented::handle`). This document establishes the complete blueprint to implement these systems cleanly across 6 phased milestones.

---

## Analysis

### 1. Codebase Gap Analysis & Wire Protocols

#### A. Guild Subsystem (Quân đoàn — Opcode `0x27`, `TCY_OrganManage`)
- **Database Schema**: `migrations/0001_init.sql` contains zero tables for guilds. Calling `list_guilds` in `src/db/domain.rs` fails at runtime.
- **Session State**: `Session` lacks guild fields (`guild_id`, `guild_name`, `guild_duty`, `guild_badge`).
- **Wire Dispatcher**: `0x27` is incorrectly labeled `OP_RANK_ANNOUNCE` in `src/protocol/mod.rs` and routed to `unimplemented::handle`. Client disassembly confirms `0x27` handles 28 C→S and 38 S→C sub-opcodes:
  - `C→S 27 01`: Guild creation (20-byte VISCII name, 10,000 gold deduction).
  - `C→S 27 02`: Invite player (`target_player_id: u32 LE`).
  - `C→S 27 03`: Accept/Decline invite (`choice: u8`).
  - `C→S 27 07`: Kick member (`member_id: u32 LE`).
  - `C→S 27 0A`: Transfer leadership (`new_leader_id: u32 LE`).
  - `C→S 27 12`: Upload 288-byte raw bitmap guild badge.
  - `C→S 27 32..38`: Siege declaration and appointment timing.
- **Concurrency & State Locking**: All two-player operations (invitation, promotion, leadership transfer) require strict Two-Phase State Locking (`lock_player_operations([min(id1, id2), max(id1, id2)])`) before reading `registry_get`, followed by an atomic SQLite write transaction on `write_pool`.

#### B. Event Lifecycle Management & Scheduling Engine
- **Current Runtime**: The server only contains two background timers: `spawn_wal_checkpoint_task` and `auto_save::spawn`.
- **Event Lifecycle State Machine**: Every event is governed by an explicit 5-phase Finite State Machine (FSM):
  - `Idle`: Inactive, awaiting scheduler trigger or Web Admin start command.
  - `Announcing`: Pre-warning broadcasts sent to players (T - 15m, T - 5m) via `S→C 25 03`, `S→C 29 09`, `S→C 14 28`, or `020102 sys_msg_frame`.
  - `Active`: Event open; registration accepted, instances initialized, map gates open.
  - `Ending / Graceful Drain`: Registration closed; 3-minute countdown broadcast; ongoing combat instances allowed to finish naturally.
  - `Cooldown / Cleanup`: Remaining combat aborted via `S→C 00 2F` / `00 36`; temporary boat items cleared (`S→C 17 65`); players warped to `savemap`.
- **Strict Mutex Policy**: Because maps in `Scene.Dat` are static (e.g. Arena `10851`, Siege cities `25000..25024`), only one instance of a specific event type runs at any time. Duplicate triggers from Web Admin return `409 Conflict`. If a manual event is running when a scheduled timer fires, the scheduler logs a warning and skips the run.
- **Starvation Watchdog**: To prevent manual events from blocking automated schedules indefinitely, every active event enforces a maximum duration ceiling (120 minutes max). Upon hitting the ceiling, the event transitions into `Ending / Graceful Drain` automatically.

#### C. Web Admin Control Plane (Port 8090 — Axum + HTMX) & Server Lifecycle Hierarchy
- **Current State**: Web dashboard on `:8090` manages accounts, log streams, and static drop seeding, and controls Game TCP Server lifecycle via `/api/server/start` and `/api/server/stop`.
- **Hierarchical Dependency Gate**: Web Admin on port 8090 manages the lifecycle of the Game Server TCP listener (port 6414). **Special events can ONLY be initiated through an actively RUNNING Game Server**.
  - Attempting to start an event when the Game Server is `STOPPED` immediately returns `409 Conflict` ("Cannot start event: Game server is stopped").
  - Stopping the Game Server via `/api/server/stop` initiates a **Cascading Emergency Drain**: all active events immediately abort combat (`S→C 00 2F` / `00 36`), clear temporary items (`17 65`), warp participants to `savemap`, before the 5-second TCP disconnect countdown fires.
- **Event Admin API**:
  - `GET /api/events`: Lists all registered events, schedule definitions, current FSM states, elapsed time, participant counts, and current Game Server running state.
  - `POST /api/events/{id}/start`: Manually initiates an event through `ServerControl`, guarded by `ServerControl.is_running()`.
  - `POST /api/events/{id}/stop`: Manually triggers the 3-minute graceful drain and cleanup sequence.
- **Security & Audit Model**: Operates under an Internal Trust model (protected by network firewall). All start/stop actions record entries into `admin_audit_log` with system admin credentials and timestamps.

#### D. Continuous Dungeon Progression (2K, 40 NPC, Chinh Phục Cửu Thiên)
- **40 NPC (`Quest 218`)**:
  - Registration at map `12001` NPC 15162 (`LôiĐàiTruyềnSư`), costing 1,000 gold or item `46072`.
  - Progresses across 40 escalating enemy waves on arena map `10851`.
  - In-Engine Wave Progression: Warping back to the map between each wave causes 40 client asset reloads and UI flicker. `BattleService` must maintain an open combat session, increment the wave index on victory, and issue next-round battle initialization packets without exiting battle mode until wave 40 or defeat.
- **Rescue 2K (`Quest 212`)**:
  - Registration at Diệp Thành `12003` NPC 15312 (Tả Từ).
  - Climbs 18 floors (`12911..12928`) with 18 sequential battles. Requires party validation (`PartySize == 5` or `1`).
  - Rewards distributed via Eve `Group` evaluation (`23086 Lông Nhị Kiều`, `46049/46050 Chứng nhận`).
- **Chinh Phục Cửu Thiên (`Quest 12309..12319`, `12338..12345`)**:
  - Entry at `33033` NPC 61209 (Hầu Tôn). Traverses 9 tiers on maps `49960..49969`.
  - 18 boss checkpoints (2 per floor) and floor treasure chests (`NPC 20001`).
  - Gated by companion follower check (`FollowPet(14735)` Hầu Tôn) and weekly quota (`Quest 12310 == 0`, overrideable by consumable `46644 Cửu Thiên Chiến Kỳ`).
- **Immediate Disconnect Fail Policy**: If a player disconnects or crashes during continuous dungeons, the combat instance terminates immediately. The character is warped back to `savemap` upon reconnect, preventing orphaned battle memory leaks.

#### E. Competitive Arena & Tournaments (Opcode `0x25`, `0x24`, `0x3E`)
- **Wire Formats**:
  - Global Alerts: `S→C 25 03 [noticeCode: u8]` (10 notice strings: Solo PK, Team 5v5 Đoàn P, Loạn Đấu, Tiên Đấu, 40 NPC).
  - Status Alerts: `S→C 14 28 [state: u8]` (1 = start, 2 = end).
  - Leaderboard Sync: `S→C 24 09 [mode: u8][records: 7B each]` and `S→C 24 0A` (display window).
  - Survival Arena / Ác Đảo: `S→C 3E 01 / 02`.
- **Timed Reward Revocation**: 7-day timed titles and rings (`Nhẫn loạn đấu`, `Nhẫn đoàn P`, `Văn chương vô giới`). Upon weekly reset (Monday 00:00) or loss of first place, the server sends `S→C 17 6F [01..03]` to revoke expired items. Point redemption under 50 points triggers `S→C 17 6E`.

#### F. High-Order Guild Warfare & Naval Battles (Opcode `0x29`, `0x3F`, `0x40`, `0x46`)
- **City Siege (`TCityManage` Opcode `0x29`)**:
  - 5 siege cities: Thọ Xuân (`25000..25004`), Trường An (`25005..25009`), Giang Lăng (`25010..25014`), Khúc A (`25015..25019`), Lạc Dương (`25020..25024`).
  - 13 announcements via `S→C 29 09 [code: u8]`. City status table for 5 cities via `S→C 29 0F`.
  - Gate collision barriers: Broadcast gate condition in `0x03` to prevent attackers from walking through intact gates.
- **Water War & Naval Combat (`0x3F`, `0x40`)**:
  - Dedicated boat inventory: Server sends `S→C 17 64` (25 slots) on naval entry, and `S→C 17 65` to clear temporary items on exit.
  - Naval bombardment: `C→S 40 01 [action, targetId, param]` and sync `S→C 40 01..03`.
- **Dragon Boat Racing (`0x46`)**:
  - Alternate Left/Right key pacing via `C→S 46 01`.
  - Anti-Macro Rate Limiter: Enforce a token bucket (max 5 paddle packets/second) to prevent automated key injection exploits.

#### G. North Star Quiz Encounter (0x10), God/Ghost Buffs (0x18 08) & Seasonal Festivals (0x24 19, 0x17 39)
- **North Star Quiz (`0x10`) Encounter & Combat Mechanics**:
  - **Encounter Trigger**: When walking in adventure maps (`try_trigger_random_encounter`), active North Star event rolls a special PvE battle encounter against **NPC 1643 ("Bắc Tinh Quân")** or **NPC 1644 ("Nam Tinh Quân")**.
  - **Anti-Spectate Lock**: External spectate attempts are rejected with `S→C 21 01 03` (*"Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến"* `0x00798478`).
  - **Modal Quiz Lock (`0x10 01`) & 45s Timeout**: Server emits `S→C 10 01` (question + 4 VISCII answers loaded from static JSON `Data/north_star_questions.json`), opening client modal dialog (`FUN_0060FF34`). Normal combat buttons (skill, attack, run) are locked. Player has 45 seconds to answer (`C→S 10 01 [answer_index: u32 LE]`); expiring the 45s timer automatically counts as an incorrect answer.
  - **Party Scope**: When traveling in a Party, only the Leader is granted the question modal and answers; only the Leader receives the God buff or Ghost penalty/HP reduction (trailing party members remain unaffected).
  - **Correct Answer Reward — Thần Tài (`FGod`)**: Server returns `S→C 10 02 01` (*"Thật thông minh! Chúc mừng bạn đã trả lời đúng!!"*), grants +10 battles to `session.god` (max 240, x1.5 EXP boost in subsequent PvE battles), emits `S→C 18 08 [playerId][02 00][01]` to display the floating God icon, and concludes combat with victory.
  - **Incorrect Answer Penalty — Con Ma (`FGhost`)**: Server returns `S→C 10 02 02` (*"Ồ! bạn trả lời sai rồi!"*), drops Leader HP to 1, assigns `session.ghost = 1` (-50% EXP penalty in PvE, receiving 0.5x EXP), emits `S→C 18 08 [playerId][01 00][01]` to display the floating Ghost icon, and ends combat.
  - **Dispel Ghost ("Bà Gọi Hồn")**: Ghost is dispelled exclusively by interacting with **NPC 1752 ("Bà Gọi Hồn")** at town maps (fee: 500 gold) or consuming Item 46180 ("Cải vận phù"). Pets do not have exorcism capabilities. Upon removal, server sends `S→C 18 08 [playerId][01 00][00]`, triggering the client to play `sound\WA0006.wav` and toast `0x00729D0C` (*"Thần xui đã rời xa bạn!"*).
  - **Anti-Bot 3-Strike & Jail**: If triggered via GM anti-bot check, 3 consecutive wrong answers trigger `S→C 10 02 03` + disconnect `S→C 00 02` / `00 2E`, or jail warp `10 02 09`.
- **Player Appear Sync (`0x03`)**:
  - `case_004` (`FUN_0078bc95`) reads `packet[5]` as `ghost` and `packet[6]` as `god`. `spawn::player_appear` must pass `s.ghost` and `(s.god > 0) as u8` instead of hardcoded `0, 0`.
- **Festival.Dat Static Parity**: All 7 records in client `Festival.Dat` have timestamp `11.0` (`1900-01-10`). The server must emit date `19000110` (`0x0121EAC6`) in `S→C 24 19` for the client dialog to match and open properly.

---

## Goals vs. Non-Goals

### In-Scope Goals
1. **Guild Subsystem (Quân đoàn)**:
   - Relational 3NF SQLite schema: `guilds`, `guild_members`, `guild_ranks`, `guild_logs`, `guild_cities`.
   - Denormalized `guild_id` on `characters` for fast login and map appearance rendering.
   - Session integration and Level-1 / Level-2 wire handlers for `0x27`.
   - Concurrency safety: Two-Phase State Lock on all player-to-player guild interactions.
2. **Event Lifecycle & Scheduling Engine**:
   - Central 5-phase FSM (`Idle` → `Announcing` → `Active` → `Ending` → `Cooldown`).
   - Tokio recurring cron ticker checking schedule definitions every 60 seconds.
   - Strict Mutex policy with 120-minute maximum watchdog.
   - Graceful Drain protocol (3-minute countdown before clean `00 2F` / `00 36` abort and savemap evacuation).
3. **Web Admin Control Plane (Port 8090)**:
   - REST and HTMX dashboard endpoints to list, trigger, and stop events on-demand.
   - Real-time S→C packet broadcast triggered immediately via `ServerControl`.
   - Comprehensive audit logging into `admin_audit_log`.
4. **Dungeon & Continuous Combat Progressions**:
   - In-engine wave combat runner for 40 NPC (`Quest 218`).
   - 18-floor battle chaining and party sync for Rescue 2K (`Quest 212`).
   - Cửu Thiên 9 tiers, companion Hầu Tôn follow validation, and floor treasure chests (`Quest 12309..12319`).
   - Immediate combat termination on disconnect.
5. **Competitive Arena, High-Order Warfare & Festivals**:
   - Arena matchmaking, scoreboards (`24 09/0A`), and weekly title revocation (`17 6F`).
   - City Siege 5 cities, gate barriers, and weapon vehicle sync (`0x29`).
   - Water War boat inventory (`17 64/65`) and naval combat (`0x40`).
   - Dragon Boat anti-macro paddle rate limiter (`0x46`).
   - North Star anti-bot quiz (`0x10`) and Festival notification broadcaster (`Festival.Dat`, `17 39`).

### Out-of-Scope Non-Goals
1. Modifying the client executable `aLogin.exe` or altering static binary `.Dat` assets in `Data/`.
2. Multi-server cluster event synchronization (all events are local to the running server instance).
3. Custom non-canonical event mechanics not present in client disassembly or `SPECIAL_EVENTS_SPEC.md`.

---

## Alternatives Weighed

1. **Monolithic Event Loop vs. Event-Bus Architecture**:
   - *Monolithic loop in `main.rs`*: Check every event condition inside a huge `tokio::select!` block. Rejected: Violates modularity, creates tight coupling, and makes isolated testing impossible.
   - *Event Engine with Subscribed Controllers (Chosen)*: A central `EventLifecycleManager` dispatches lifecycle state transitions to modular controllers (`ArenaController`, `SiegeController`, `FestivalController`).
2. **Guild State: Pure DB vs. Denormalized Hybrid Cache**:
   - *Pure DB queries on every packet*: Query SQLite on every movement and chat packet to look up guild name. Rejected: Severe SQLite read-pool contention.
   - *Denormalized Hybrid Cache (Chosen)*: Store `guild_id` directly in `characters` and cache `guild_id`/`guild_duty` in `Session`. On guild disband or kick, execute an atomic batch DB update and synchronize online sessions.
3. **40 NPC Progression: Map Warp vs. In-Engine Wave Chaining**:
   - *Warping player back to map between waves*: Triggers 40 client asset reloads and UI flicker. Rejected: Poor gameplay ergonomics.
   - *In-Engine Wave Chaining (Chosen)*: Maintain the combat session, advance wave index on victory, and issue next-round battle initialization packets without exiting combat.
4. **Web Admin Control: Polling DB Flags vs. Direct ServerControl Bus**:
   - *Polling DB flag writes*: Server polls `is_active` in SQLite. Rejected: Polling lag, DB overhead, and race conditions.
   - *Direct ServerControl Bus (Chosen)*: Web Admin invokes `ServerControl` directly via Tokio message channel/Arc, triggering immediate state transitions and real-time S→C packet broadcast.

---

## Key Decisions & Staging for Next Steps

### Settled Architectural Decisions

1. **Strict Mutex Event Policy**: Single active instance per event type; duplicate triggers return `409 Conflict`; overlapping scheduler runs are skipped; 120-minute maximum watchdog.
2. **Graceful Drain with Timeout**: 3-minute announcement countdown, allowing active fights to finish; expired fights terminated cleanly via `S→C 00 2F` / `00 36`, temporary boat items cleared (`17 65`), and participants evacuated to `savemap`.
3. **Full Foundation First for Guilds**: Complete 3NF SQLite schema, Two-Phase State Locking, and core `0x27` handlers prior to siege/naval events.
4. **Denormalized Guild ID**: `characters.guild_id` for fast login lookups, with batch update on disband/kick.
5. **Internal Trust Web Admin**: Port `:8090` open on internal network; all actions recorded to `admin_audit_log`.
6. **Immediate Disconnect Fail for Continuous Dungeons**: Zero orphan combat sessions; character respawns at `savemap` upon reconnect.
7. **Weekly Event Reset**: Every Monday at 00:00 (resets Cửu Thiên `Quest 12310`, reclaims 7-day Lôi Đài rings via `17 6F`).

### Architectural Phasing Sequence

* **Phase 1: Guild Engine & Opcode 0x27 Subsystem**:
  - SQLite migration (`guilds`, `guild_members`, `guild_ranks`, `guild_logs`, `characters.guild_id`).
  - Session integration and Two-Phase State Lock implementation.
  - Level-1 and Level-2 handlers for `0x27` (Creation, invites, kicks, duty transfers, badge upload).
  - Integration tests in `tests/guild_test.rs`.
* **Phase 2: Event Scheduler, Lifecycle Engine & Web Admin Controls**:
  - Central 5-phase FSM (`Idle` → `Announcing` → `Active` → `Ending` → `Cooldown`).
  - Tokio background cron ticker.
  - Axum REST / HTMX routes on `:8090` and audit logging.
  - S→C push notification encoders (`25 03`, `29 09`, `14 28`, `3F 0C`).
  - Integration tests in `tests/event_scheduler_test.rs` and `tests/web_admin_events_test.rs`.
* **Phase 3: Dungeon / Script-driven Events (2K, 40 NPC, Chinh Phục Cửu Thiên)**:
  - 40 NPC in-engine wave runner in `src/battle/service.rs`.
  - 2K 18-floor battle chaining and party sync (`Quest 212`).
  - Cửu Thiên companion follow check (`NPC 14735`), weekly gating (`Quest 12310`), and treasure chests (`Quest 12309..12319`).
  - Integration tests in `tests/special_events_dungeon_test.rs`.
* **Phase 4: Competitive Arena & Tournaments (Opcode 0x25, 0x24, 0x3E)**:
  - Arena matchmaking, bracket tracking, and scoreboards (`24 09/0A`).
  - Survival Arena (`0x3E`).
  - Weekly title/ring revocation via `S→C 17 6F`.
  - Integration tests in `tests/arena_test.rs`.
* **Phase 5: High-Order Guild Wars (City Siege 0x29, Water War 0x3F/0x40, Boat Race 0x46)**:
  - 5-city siege management, gate collision barriers, weapon vehicles (`0x29`).
  - Water War boat inventory (`17 64/65`) and naval combat (`0x40`).
  - Dragon boat race track state machine and anti-macro rate limiter (`0x46`).
  - Integration tests in `tests/guild_war_test.rs`.
* **Phase 6: North Star Quiz Encounter, God/Ghost State & Seasonal Festivals (0x10, 0x18 08, 0x24 19, 0x17 39)**:
  - `Session` & DB schema enhancement: add `ghost: u8` and persist `god: u32`.
  - Fix `spawn::player_appear` to transmit `s.ghost` and `s.god` (offsets 5 & 6).
  - Real-time God/Ghost status synchronizer: frame builder `S→C 18 08 [playerId][type][value]`.
  - North Star special encounter integration in `try_trigger_random_encounter` (vs NPC 1643/1644).
  - Anti-spectate guard emitting `S→C 21 01 03` (*"Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến"*).
  - Modal Q&A wire handler (`0x10 01`, `0x10 02`): correct reward (+10 battles Thần Tài x1.5 EXP), incorrect penalty (HP=1, Con Ma -50% EXP).
  - Ghost dispel mechanism via NPC 1752 ("Bà Gọi Hồn"), Pet 1752, and Item 46180 ("Cải vận phù"), triggering `sound\WA0006.wav` and toast *"Thần xui đã rời xa bạn!"*.
  - Anti-bot 3-strike disconnect / jail workflow (`0x10 02 03/09`).
  - 74-message festival broadcaster and `Festival.Dat` date parity helper (`19000110`).
  - Integration tests in `tests/festival_quiz_test.rs`.
