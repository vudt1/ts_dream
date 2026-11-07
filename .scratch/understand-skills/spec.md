---
title: Chuẩn hóa Giao thức Kỹ năng, Khung Định danh 05 03, Hệ thống Nghề Chuyển sinh & Tẩy điểm (Skill Protocol Parity, Stats Packet, Job System & Respec)
status: ready-for-agent
---

## Problem Statement

When players connect to the server using the reference Win32 client (`aLogin.exe`), multiple critical defects and unimplemented protocols prevent character skill progression:

1. **Skill Loading Failure**: The server attempts to load skills from an inexistent text file, resulting in an empty skill dictionary at runtime. As a consequence, players cannot learn or upgrade any skills via basic UI interactions.
2. **Reborn and Job Identity Loss in Login Frame (`05 03`)**: The server fills the tail of the fixed 113-byte identity header with zero bytes. This transmits `reborn = 0` and `job = 0` regardless of the player's true state in the database. The client UI locks tabs 6 through 9 on the skill tree, displaying an error popup stating that players cannot learn skills of other jobs, effectively locking players out of their chosen profession.
3. **Ignored Eve Script Skill Rewards**: When completing quests or interacting with NPCs designed to grant Mind and Life skills (such as Mining, Crafting, Riding, or Vending), the action result class for skills is completely ignored by the event execution handler, preventing players from acquiring life skills through gameplay.
4. **Missing Bulk Skill Tree Commit (`0x1C 04`)**: When players allocate multiple skill points on the graphical skill tree or hero skill form and click Confirm, the client transmits an unhandled subcode, discarding all pending skill allocations.
5. **Loss of Reborn 2 Customization (`0x1C 05`)**: When players commit their Reborn 2 configuration, the server drops the dragon balls list and completely ignores the 8 hero ball skill slots, failing to persist them or synchronize them via the required server-to-client frames.
6. **Inert Job Skills in Combat**: Active and passive Job skills (such as Bá ý, Thiên thần thuẫn, Ngưng khí hộ thân, Bá khí, Anh khí hộ thân) are either unhandled or treated as empty stubs in the battle engine, granting no combat advantage to advanced jobs.
7. **Unimplemented Respec System (`0x2A`)**: Players cannot reset skills or attribute points using Memory Manuals or Careful Pouches because the entire opcode is routed to an unimplemented stub.

## Solution

Implement an authoritative, byte-exact skill protocol engine matching the decompiled Delphi client specifications:

1. **Canonical Skill Data Bootstrapping**: Deserialize the canonical 376 skill records directly from the structured JSON asset into server memory, establishing a single source of truth for skill definitions, element affinities, prerequisites, and point costs.
2. **Byte-Exact `StatsPacketBuilder`**: Construct a dedicated packet builder for opcode `0x05 Subcode 0x03` that enforces exact byte placement for current SP (offset 5..6), remaining skill points (offset 24..25), reborn stage (offset 96), and job identifier (offset 97), followed by the dynamic skill array tail. This immediately unlocks the client UI's Job skill tabs.
3. **Authoritative Eve Script Skill Grants**: Wire the skill action result class within the event executor to grant skills free of skill point deductions at the script-specified level, immediately synchronizing the learned skill to the client via property update packets and persisting changes to SQLite.
4. **Comprehensive Opcode `0x1C` Handler**:
   - Subcode `0x01`: Validate element affinities, prerequisite skill mastery, point costs, and enforce strict Job matching (requiring mastery of the root Great Heavenly Cycle skill before allowing any Job skill).
   - Subcode `0x02`: Upgrade existing pet skills by resolving pet roster positions.
   - Subcode `0x04`: Process bulk skill commits from graphical skill tree forms in an atomic transaction.
   - Subcode `0x05`: Parse all three sections of the Reborn 2 commitment packet (dragon balls, hero skills, and 8 hero ball slots), saving them to the character record and echoing synchronization frames.
5. **Full Reborn 2 Synchronization**: Implement frames `0x17 Subcode 0x4D` (dragon ball mask and point pool) and `0x17 Subcode 0x4E` (8 hero ball skill slots) sent upon login and post-reborn commitment.
6. **Combat Integration of 16 Job Skills**: Inject Job passives into combat actor statistics upon battle entry, and implement dedicated combat mechanics for active Job skills (including 100% critical rate for Bá ý, damage absorption/redirection for Thiên thần thuẫn, and percentage damage reduction for Ngưng khí hộ thân).
7. **Respec Dispatcher and Handler (`0x2A`)**: Activate the Level-1 dispatcher for opcode `0x2A`, validating consumable item ownership in inventory under Two-Phase State Lock, deducting 1 skill level or stat point, refunding points, consuming the item, and persisting changes atomically.

## User Stories

1. As a player, I want the server to load all 376 canonical skills from structured data at startup, so that I can learn and upgrade skills without runtime lookup failures.
2. As a player, I want my Reborn level accurately transmitted in packet `05 03` at byte offset 96, so that my client UI correctly recognizes my Reborn status.
3. As a player with a chosen profession (Bá, Hiệp, Hiền, or Tiên), I want my Job code accurately transmitted in packet `05 03` at byte offset 97, so that my client unlocks my profession's skill tab without error dialogs.
4. As a player, I want my current SP and remaining skill points accurately positioned in packet `05 03`, so that skill requirement gates on the client function properly.
5. As a player completing an Eve script or talking to an NPC that awards a skill, I want to receive that skill immediately, so that I can acquire life skills through world interaction.
6. As a player receiving an Eve skill reward, I want the skill to cost zero skill points, so that my character's earned skill points are not depleted by quest rewards.
7. As a player receiving an Eve skill reward, I want the granted level to match the script's specified level (defaulting to level 1 if unstated), so that advanced rewards give higher initial mastery.
8. As a player receiving an Eve skill reward, I want the transaction to persist atomically to SQLite, so that my acquired skill is never lost across server restarts.
9. As a player receiving an Eve skill reward, I want to receive property update packet `08 01 6E 01` and a system chat toast, so that my client interface immediately reflects the new skill.
10. As a player clicking '+' on the basic skill window (`0x1C 01`), I want to upgrade a single skill by 1 level, so that I can progress my character.
11. As a player learning a level 1 skill of my own element, I want to be charged the base point cost, so that standard element learning costs apply.
12. As a player learning a level 1 skill of an opposing element, I want to be charged double the base point cost, matching client validation gates.
13. As a player upgrading an existing skill (level > 1), I want to be charged exactly 1 skill point per level regardless of element, matching client grade point rules.
14. As a player attempting to learn a skill without having learned its prerequisite, I want the server to reject the request, preserving skill tree integrity.
15. As a player attempting to learn a skill of an incompatible elemental school (e.g. Earth trying to learn Fire directly), I want the server to reject the request.
16. As a player with a companion pet, I want to upgrade an existing pet skill via packet `0x1C 02`, so that my pet grows stronger.
17. As a player, I want pet skill upgrades via `0x1C 02` restricted strictly to existing pet skills, preventing unauthorized skill injection into empty pet slots.
18. As a player using the skill tree form (`TSe_SkillTreeForm`), I want to allocate multiple skill points and submit them in one click via packet `0x1C 04`, so that all upgrades apply simultaneously.
19. As a player submitting bulk upgrades via `0x1C 04`, I want the server to validate point costs and prerequisites for every skill in the batch atomically, ensuring partial exploits are rejected.
20. As a player who has not chosen a profession, I want the server to reject any attempt to learn Job skills (`14039..14054`), preventing unauthorized cross-class learning.
21. As a player with a profession, I want the server to reject learning skills of any other profession, ensuring class exclusivity.
22. As a player with a profession, I want the server to require Great Heavenly Cycle (`14038`) at level 1 or higher before allowing any Job skill to be learned, enforcing root tree prerequisites.
23. As a Reborn 2 player, I want to submit my dragon balls, hero skills, and 8 hero ball skill slots via packet `0x1C 05`, so that my Reborn 2 configuration is committed.
24. As a Reborn 2 player, I want the server to store my 12 dragon balls and 8 hero ball skill slots in SQLite, so that my build is permanently saved.
25. As a Reborn 2 player logging into the game, I want to receive packet `17 4D` containing my dragon ball mask and remaining ball points, so that my dragon ball interface renders accurately.
26. As a Reborn 2 player logging into the game, I want to receive packet `17 4E` containing my 8 hero ball skill slots, so that my Reborn 2 skill tree displays all socketed hero skills.
27. As a Bá Vương player entering combat, I want my learned passive skills (Bá khí `14039`, Lực bạt sơn hề `14041`) to boost my physical attack and combat stats in `WarInfo`, so that my physical prowess is heightened.
28. As a Bá Vương player casting Bá ý (`14040`), I want to gain a 100% critical strike buff for 2 to 4 turns, so that I can land guaranteed critical hits.
29. As a Bá Vương player casting Sóng cuồng / Nộ đào chiến đạp (`14042`), I want to deal heavy area physical damage to multiple opponents, so that I can clear enemy groups.
30. As a Hiệp Khách player entering combat, I want my learned passive skills (Anh khí hộ thân `14043`, Tâm nhãn `14045`) to increase my defense and survivability in `WarInfo`, so that I can withstand enemy damage.
31. As a Hiệp Khách player casting Thiên thần thuẫn (`14044`), I want to create a protective barrier on an ally or myself that absorbs 50% incoming damage for several turns, so that I can protect my team.
32. As a Hiệp Khách player casting Ngưng khí hộ thân (`14046`), I want to gain percentage damage reduction for several turns, so that I can tank heavy boss strikes.
33. As a Hiền Nhân player entering combat, I want my learned passive skills (Dụng kế `14047`, Trí thủ `14048`, Ngự địch `14049`, Sách động `14050`) to enhance my intelligence, defense, and tactical status application in `WarInfo`, so that my strategic capabilities are realized.
34. As a Tiên Nhân player entering combat, I want my learned passive skills (Tiên khiếu `14051`, Hóa Huyết Hoàn Hồn `14052`) to boost my magic attack and MP efficiency in `WarInfo`, so that my magical endurance is maximized.
35. As a Tiên Nhân player casting Thánh Linh thuật (`14053`), I want to dispel beneficial buffs and deal single-target holy magic damage matching `Data/Skill.Dat` (`nb_target=1`), and when casting Càn Khôn Thiên Lôi (`14054`), I want to deal area lightning magic damage to the entire enemy team, so that I can dominate magical warfare.
36. As a player using a Memory Manual (`46035 / 46166`), I want the server to open the skill respec interface (`S→C 0x2A 01`), so that I can select which skill to downgrade.
37. As a player confirming a skill downgrade via `C→S 0x2A 01`, I want to lose 1 level from the selected skill, regain 1 skill point, consume 1 Memory Manual from my inventory, and update SQLite atomically under Two-Phase State Lock, so that my inventory and skill points remain strictly synchronized.
38. As a player using a Careful Pouch (`46203`), I want the server to open the attribute respec interface (`S→C 0x2A 02`), so that I can select which attribute point to reclaim.
39. As a player confirming an attribute reduction via `C→S 0x2A 02`, I want to deduct 1 point from the chosen stat, regain 1 unallocated stat point, consume 1 Careful Pouch from my inventory, and persist the transaction atomically, so that my attribute reset is applied reliably.
40. As a player attempting an invalid respec action (e.g. attempting to downgrade a skill at level 0, reducing a stat below its racial minimum, or lacking the item in inventory), I want the server to reject the transaction safely without mutating state, so that exploit attempts fail cleanly.

## Implementation Decisions

### 1. Canonical Skill Catalog Bootstrapping
- Replace the legacy text file reader in the data loader with a direct JSON deserializer loading `Data/skills.json` into the server's skill dictionary.
- The 376 canonical skill definitions provide wire IDs, names, elements (0=None/Light/Dark, 1=Earth, 2=Water, 3=Fire, 4=Wind, 5=Mind), maximum levels, base point costs, prerequisite skill arrays, and reborn requirements.
- Retain the binary skill definitions loaded from `Data/Skill.Dat` for byte-exact validation of internal properties (`atk_type`, `nb_target`, `skill_type`, `delay`).

### 2. Standardized `StatsPacketBuilder` (`05 03`)
- Encapsulate the construction of opcode `0x05 Subcode 0x03` into a dedicated builder that guarantees byte-exact offset alignment for the fixed 113-byte header:
  - Byte 0..1: Opcode `0x05 0x03`
  - Byte 2: Element (`thuoctinh`, 1B)
  - Byte 3..4: Current HP (`hp`, 2B LE)
  - Byte 5..6: Current SP (`sp`, 2B LE)
  - Byte 7..8: Base Int (`int1`, 2B LE)
  - Byte 9..10: Base Atk (`atk`, 2B LE)
  - Byte 11..12: Base Def (`def`, 2B LE)
  - Byte 13..14: Base Agi (`agi`, 2B LE)
  - Byte 15..16: Base Hpx (`hpx`, 2B LE)
  - Byte 17..18: Base Spx (`spx`, 2B LE)
  - Byte 19: Level (`lv`, 1B)
  - Byte 20..23: Total EXP (`texp`, 4B LE)
  - Byte 24..25: Remaining Skill Points (`skill_point`, 2B LE)
  - Byte 26..27: Remaining Attribute Points (`point`, 2B LE)
  - Byte 28..31: Reputation (`tiengtam`, 4B LE)
  - Byte 32..33: Max HP (`hp_max`, 2B LE)
  - Byte 34..35: Max SP (`sp_max`, 2B LE)
  - Byte 36..59: Gear bonus stats (Atk2, Def2, Int2, Agi2, Hpx2, Spx2; each 4B LE)
  - Byte 60..69: Five literal `500` constants (`F4 01` × 5, 10B)
  - Byte 70..95: Padding zeroes (26B)
  - Byte 96 (offset `0x60`): Reborn stage (`reborn`, 1B: 0=None, 1=Reborn 1, 2=Reborn 2)
  - Byte 97 (offset `0x61`): Job code (`job`, 1B: 0=None, 1=Hiệp, 2=Bá, 3=Hiền, 4=Tiên)
  - Byte 98..112: Trailing header padding zeroes (15B)
  - Tail (from byte 113): Repeated 3-byte tuples `{ skillId: u16 LE, level: u8 }` for all learned skills in the player's session.

### 3. Eve Script Skill Grant Execution
- Within `execute_action_result` in the event handler, add a dedicated branch for `EveResultClass::Skill = 4`:
  - Extract `skill_id = result.parameter` and `target_level = max(1, result.result_value as u8)`.
  - Validate that `skill_id` exists in the canonical skill catalog.
  - If the player already possesses the skill, upgrade it only if `target_level` exceeds the currently learned level. If the player does not possess the skill, append it.
  - Apply zero skill point deduction.
  - Persist the skill record to the SQLite database.
  - Emit property update frame `08 01 6E 01 [target_level: u32 LE][skill_id: u32 LE]`.
  - Emit a system chat toast informing the player of the acquired skill.

### 4. Database Schema Expansion & Reborn 2 State
- In the SQLite `characters` table, add two columns via migration:
  - `dragon_balls INTEGER NOT NULL DEFAULT 0`: A composite integer storing a 12-bit mask for socketed dragon balls (bits 0..11) and an 8-bit integer for remaining unallocated ball points (bits 12..19).
  - `rb2_slots TEXT NOT NULL DEFAULT ''`: A comma-separated string or hex serialization storing the 8 hero ball skill IDs socketed in slots 6 through 13.
- In `Session`, add matching fields: `ball_point: u8`, `dragon_balls: u16`, and `skill_rb2: [u16; 8]`.
- Load and hydrate these fields during character authentication, and persist them when modified.

### 5. Opcode `0x1C` Comprehensive Handlers
- **Subcode `0x01` (Single Skill Learn/Upgrade)**:
  - Wire layout: `[skill_id: u16 LE][target_level: u8]`.
  - Check maximum skill level and reborn requirements against the catalog.
  - Enforce Job gates for skills `14039..14054`: verify `session.job` matches the skill's job branch, and verify that skill `14038` is learned with `level >= 1`.
  - Check elemental school compatibility via standard element conflict rules.
  - Calculate costs: learning level 1 costs `base_point` (doubled if opposing element); upgrading level > 1 costs exactly 1 skill point per level (no doubling).
  - Deduct skill points, mutate `session.skills`, persist to SQLite, and emit `08 01 6E 01` and `08 01 25 01`.
- **Subcode `0x02` (Pet Skill Upgrade)**:
  - Wire layout: `[pet_slot: u8 (1..4)][skill_id: u16 LE][target_level: u8]`.
  - Resolve pet by roster slot position rather than arbitrary state indices.
  - Restrict mutations strictly to skills already present on the pet; reject new skill additions.
  - Deduct pet skill points, mutate pet skills, persist pet state, and emit `08 02 04 [slot: u16 LE] 6E 01 [target_level: u32 LE][skill_id: u32 LE]`.
- **Subcode `0x04` (Skill Tree Bulk Commit)**:
  - Wire layout: `N × [skill_id: u16 LE, target_level: u8]`.
  - Parse consecutive 3-byte tuples until the payload is exhausted.
  - Execute atomic validation across the entire batch: check total skill points required, check elemental and Job gates, and check prerequisite chains.
  - If valid, mutate all skills in session memory, commit all changes in one database transaction, and emit individual `08 01 6E 01` updates followed by a final `08 01 25 01` remaining points update.
- **Subcode `0x05` (Reborn 2 Tree Commitment)**:
  - Wire layout: `[L1: u32 LE][L1 × u8 ballIdx][L2: u32 LE][L2 × 3B {skillId, level}][L3: u32 LE][L3 bytes ({slot: u8, skillId: u16 LE})]`.
  - Decode `L1` dragon balls and update `session.dragon_balls`.
  - Decode `L2` hero skills and apply upgrades matching subcode `0x04` rules.
  - Decode `L3` hero ball slots (where count = `L3 / 3` and slots map to 6..13), updating `session.skill_rb2`.
  - Commit all updates atomically to SQLite.
  - Emit synchronization frames `17 4D` (dragon balls) and `17 4E` (hero ball slots).

### 6. Combat Integration for 16 Job Skills
- In `construction::add_player`, inspect the player's learned skills and apply passive Job bonuses directly to `WarInfo`:
  - Bá branch: `14039` Bá khí (+Atk), `14041` Lực bạt sơn hề (+Atk/MaxHP).
  - Hiệp branch: `14043` Anh khí hộ thân (+Def), `14045` Tâm nhãn (+Agi/Hit).
  - Hiền branch: `14047` Dụng kế (+Int), `14048` Trí thủ (+Def/Int), `14049` Ngự địch (+Def), `14050` Sách động (+Status rate).
  - Tiên branch: `14051` Tiên khiếu (+Int/MaxSP), `14052` Hóa Huyết Hoàn Hồn (+Turn HP/SP regen).
- In `runner::apply_buff19` and combat resolution:
  - `14040` Bá ý: Set a combat status flag granting a 100% critical strike rate on physical attacks for `N` turns.
  - `14044` Thiên thần thuẫn: Set a combat status flag on an ally or self reducing incoming damage by 50% for `N` turns.
  - `14046` Ngưng khí hộ thân: Set a combat status flag reducing all incoming damage by a defined percentage for `N` turns.
  - `14042` Nộ đào chiến đạp: Implement as an active multi-target physical attack dealing scaled area damage.
  - `14053` Thánh Linh thuật: Implement as an active single-target (`nb_target=1` per `Skill.Dat`) holy attack applying full dispel of beneficial buffs (types 4, 15, 19) and holy damage.
  - `14054` Càn Khôn Thiên Lôi: Implement as an active full-team area lightning magic attack (fight area 8 override).

### 7. Respec Engine (`Opcode 0x2A`)
- Register `OP_RESET = 0x2A` in the Level-1 dispatcher, routing to `handlers::respec`.
- In item use handlers, bind consumable items to trigger respec UI frames:
  - Memory Manuals (`46035`, `46166`): Send `S→C 0x2A 01` (open `TBK_SkillRegressForm`).
  - Careful Pouches (`46203`): Send `S→C 0x2A 02` (open `TBK_AttrRegressForm`).
- In `handlers::respec`:
  - `C→S 0x2A 01 { skill_id: u16 LE }`: Acquire Two-Phase State Lock on player. Verify player has the target skill at `level >= 1` and possesses at least 1 Memory Manual in inventory. Deduct 1 skill level (or remove skill if reduced to 0), credit 1 `skill_point`, consume 1 Memory Manual, persist inventory and skill state in an atomic database transaction, emit item count update `17 09`, skill update `08 01 6E 02`, remaining skill points `08 01 25 01`, and emit ACK/close frame `S→C 0x2A 01 00` (`F4 44 03 00 2A 01 00`) to play `sound\WA0014.wav`, show success toast, and close `TBK_SkillRegressForm`.
  - `C→S 0x2A 02 { attr_code: u8 }`: Acquire Two-Phase State Lock. Verify player has allocated points in the specified attribute above starting base and possesses at least 1 Careful Pouch. Deduct 1 point from the attribute, credit 1 unallocated `point`, consume 1 Careful Pouch, commit atomically, emit item count update `17 09`, stat update `08 01 26 01`, modified stat frame, and emit ACK/close frame `S→C 0x2A 02 00` (`F4 44 03 00 2A 02 00`) to play `sound\WA0014.wav`, show success toast, and close `TBK_AttrRegressForm`.

## Testing Decisions

### Testing Philosophy & Seam Placement
- Tests must strictly evaluate observable external behavior via wire frames and database state; implementation details (private helper methods, internal transient variables) must not be asserted directly.
- Testing seam: Integration tests located in `tests/skill_job_system_test.rs` executing against:
  - Public handler entrypoints (`handle_skills`, `handle_respec`, `execute_action_result`) using `OpcodeCtx` and simulated connections.
  - Real SQLite test database configured via `common::test_db::setup_test_db()`.
  - Wire frame verification: Assert exact hex string output for frames `05 03`, `08 01 6E`, `08 01 25`, `17 4D`, `17 4E`, and `17 09`.

### Modules to Test
1. **Data Loading**: Assert that `GameData::load` populates 376 skills from JSON with non-empty attributes and prerequisite arrays.
2. **Stats Packet Builder**: Assert byte offsets 96 (`reborn`) and 97 (`job`) in generated `05 03` frames across various character combinations (unreborn, reborn 1, reborn 2, each job 1..4).
3. **Eve Skill Awards**: Verify that `execute_action_result` with class 4 inserts new skills, updates levels, emits `08 01 6E 01`, and persists to SQLite without consuming `skill_point`.
4. **Skill Learning Gates**:
   - Verify rejection when attempting to learn Job skills without the matching Job.
   - Verify rejection when attempting to learn Job skills without having learned `14038`.
   - Verify elemental cost doubling on level 1 and 1-point cost on subsequent levels.
5. **Bulk & Reborn 2 Commits**:
   - Verify batch processing of `0x1C 04` updates.
   - Verify three-section parsing of `0x1C 05` and assertion of `dragon_balls` bitmask and 8 `skill_rb2` slots in both session and database.
   - Verify emission of frames `17 4D` and `17 4E`.
6. **Combat Integration**:
   - Verify that adding Job passives increases calculated Atk, Def, and Int in `WarInfo`.
   - Verify that casting Bá ý (`14040`) triggers 100% critical hit rolls in `calculate_attack`.
7. **Respec Actions**:
   - Verify skill level reduction, skill point refund, Memory Manual consumption, and atomic rollback on invalid attempts.
   - Verify attribute reduction, stat point refund, Careful Pouch consumption, and atomic rollback on invalid attempts.

### Prior Art
- `tests/compound_test.rs`: Reference for inventory slot verification, consumable item deduction, and atomic SQLite persistence.
- `tests/checkpoint6_parity_test.rs`: Reference for Eve action result execution and packet verification.
- `tests/battle_command_channel_test.rs`: Reference for battle action dispatch, status buff application, and damage calculation.
- `tests/login_char_flow_test.rs`: Reference for `05 03` frame layout testing.

## Out of Scope

1. **Light/Dark Skill Acquisition**: Acquisition workflows for Light (`22001..22022`) and Dark (`23001..23023`) skills via skill books (`Item.Dat type=65`) in `books.rs` or specific Eve quest lines are deferred to a subsequent milestone.
2. **Ship Combat Skill Tree**: The naval warfare skill tree window (`TShipSkillTreeForm`, `0x1C Subcode 0x06`) and related aquatic battle state are deferred.
3. **Second-Generation Child Skill Deletion**: Skill deletion for second-generation child companions (`TLH_ChildSkillDelForm`, `0x2A Subcode 0x04` / `0x45 0A`) is deferred.
4. **Mining Mini-game Mechanics**: Mining field movement loops, pickaxe durability (`Item type=39`), and mine node random encounter triggers are deferred.

## Further Notes

- All system message toasts emitted during skill learning, quest grants, and respec actions must be encoded in VISCII 1.1 format using `sys_msg_frame`.
- Database schema changes (`dragon_balls`, `rb2_slots`) must provide safe defaults (`0` and empty string) to ensure seamless backward compatibility with existing character records.
