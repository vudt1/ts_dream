# 20 — Battle engine: construction + turn loop + damage pipeline (Ch6)

**What to build:** Cốt lõi battle engine — tạo battle (các trigger `TheBattle`), grid `ListWar` + `_keys`, 3 RNG streams riêng biệt, vòng turn `Battling` (async task per battle), targeting, damage pipeline (banker's rounding), HP/SP/EXP tính bằng công thức, và bộ packet battle byte-faithful. Một trận player-vs-NPC chạy đúng byte-wire.

**Blocked by:** 03 — Static data (Npcs, Skills); 13 — Skills (battle dùng skill của player); 17 — Pets (pet vào battle); 08 — Move (battle-state kiểm tra đang battle); 19 — Talk quests (trigger battle từ `[TEAMDEF]`).

**Status:** resolved

- [x] Domain: `WarInfo` đầy đủ (type/id/idNpcOnMap/idChar/row/col/HP/SP/Lv/thuoctinh/leader/skills/buffs/_Attacked/_Random/_Exp/_Packet — `_Packet` = 23-byte snapshot, Ch6 §6.0).
- [x] 3 RNG streams (`.NET`-style time-seeded, riêng biệt): `random_0`(drop/skill pick), `random_1`(tie-break per-turn `_Random` + jitter), `random_2`(npc respawn) — không merge (Ch6 §6.0).
- [x] Construction triggers map sang Diahinh: PK `TheBattle(id1,id2,112)`; NPC attack 112; Quest/TeamDef `TeamDef[0]`; ActiveNPC `4712`. `IdBattle = IdBattleCount++` (start 1, assign trước increment) (Ch6 §6.1).
- [x] `AddToBattle` (team row==0?1:2; load HP/SP; sum Int2..; leader col2 load ≤4 pets `(row^1, col)` từ SttPet..+3, Type4; mỗi member col1/3/0/4 một pet; overlaps member overwrite theo dict order). `AddNPCToBattle` Type/npc, Team2. `ChangedWar` rebuild `_Packet` (Ch6 §6.1).
- [x] Vòng `Battling` (async task): win/lose check (enemy dead vs player dead); reset & buff ticks (burn/poison + DB write cho non-npc + broadcast `{3201}`); input wait ≤~21s poll 100ms (auto npc skill `GetRandomSkillNPC`); turn order `Attacked DESC, Agi DESC, Random DESC`; action execution (Ch6 §6.2/§6.3).
- [x] Targeting pickers (GetPosAttack/combo/TG/3_15/GiaiTru/Type4/honLoan) — `_Diahinh` không ảnh hưởng targeting/damage (Ch6 §6.4).
- [x] Damage pipeline đúng: `double` + `Math.Round` (banker's) → int; Type1 physical (Atk×Element×2.0−Def×1.6, lv-diff, GetDamageSkillInt×DoManh×(1+skillLv×0.033), num37 2.0/2.6 combo, hit-roll miss %, min 1 + jitter, reflect 13003, buffs); Type2 magic (dùng `_Int`, no num37); status 3/4/15/19, catch(bắt)/flee/heal/cleanse (Ch6 §6.5).
- [x] HP/SP/EXP: `getHpMax` (rb0 floor(lv^0.35+1)*hpx*2+80+lv …), `getSpMax`, `TexpGetLvUp` (loop Texps, max 200) — exponent 0.35/0.25 exact (Ch6 §6.6).
- [x] Battle packets byte-faithful (Ch6 §6.8): open `F4441C000BFA`, entity `F4441A000B0503/0505`+`_Packet`, `F4440A000B0402`, hide `F44408000B00`, reposition `F44405000B01`, clear pet `F44404000B01`, your-turn `F44402003401`, acting `F44404003505`, action `{3201}`, skillcast `F444130032010F00`, buff-end `F44407003501`, drop `F44408003504`, status `F4440C000801`, battle-end `F44403000B0A01`. DiaHinh echo (PK-member open 7000).
- [x] Battle state trên mỗi async task (race-free, deterministic) (Ch1 §1.4).
- [x] Golden: 1–2 battle sample scenarios được ghi lại (Ch9 §9.6).

---

## Code reference matrix (Rust vs C#) — audit 2026-08-12

Trạng thái: **ready-for-agent**. Mỗi tiêu chí đính kèm `file:line` Rust + C# để đối chiếu. Các GAP đã chốt scope (implement trong ticket 20) ở bảng cuối.

| # | Tiêu chí | Trạng thái | Rust (file:line) | C# (file:line) |
|---|---|---|---|---|
| 1 | WarInfo đầy đủ + `_Packet` 23-byte | ✅ MATCH | `src/battle/engine.rs:20-84` (struct + `packet_hex`), test `:133-147` | `DataStructure.cs:661-746` (`WarInfo`); `_Packet` `TheBattle.cs:110` |
| 2 | 3 RNG streams riêng biệt | ✅ MATCH | `src/battle/rng.rs:119-146` (`DotNetRandom:21-114`); `random_0` `runner.rs:2507`, `random_1` `runner.rs:256,1217`, `random_2` `runner.rs:2600-2601` | `TheBattle.cs:26-30`; seed `:451-453`; `random_2` `:3206-3207,4719-4720` |
| 3 | Construction triggers + IdBattle | ⚠️ PARTIAL — ActiveNPC 4712 thiếu | PK `handlers/battle.rs:81-103` + `service.rs:455-489`; NPC `battle.rs:107-124` + `service.rs:407-423`; TeamDef `quest.rs:814-819` + `service.rs:492-520` + `handler.rs:197-206`; ActiveNPC: hằng `construction.rs:544`, test `:872`; IdBattle `service.rs:341,400-402` (bản sao `manager.rs:76-78`) | PK `Client.cs:1300`; NPC `Client.cs:1323`; TeamDef `FTalk.cs:182,225,240,254`; ActiveNPC `Data.cs:5026-5080`; `IdBattleCount` `Server.cs:33,51`, gán trước increment `TheBattle.cs:461-465` |
| 4 | AddToBattle / AddNPCToBattle / ChangedWar | ✅ MATCH | `construction.rs:85-205` (team rule `:87`, pets `:117-176`, overwrite test `:686-712`) | `TheBattle.cs:73-113` (ChangedWar), `:116-422` (AddToBattle), `:424-442` (AddNPCToBattle) |
| 5 | Vòng Battling | ✅ MATCH (sai lệch nhỏ: deadline-recv thay 100ms poll — hành vi tương đương; leader SP regen thiếu) | `runner.rs:238-415`; `manager.rs:178-214`; ghi chú SP regen `runner.rs:225-227` | poll 100ms/21s `TheBattle.cs:1259-1441`; turn order `:1468-1475`; SP regen quan-su `:4146-4247` |
| 6 | Targeting pickers, `_Diahinh` vô hiệu | ✅ MATCH | `targeting.rs:216-318`, doc `:7` | pickers `TheBattle.cs:7317-9229`; `_Diahinh` chỉ echo (v.d. `:128,243,684,950,5047`) |
| 7 | Damage pipeline | ✅ MATCH | `damage.rs:84-243` (banker `:84-86`), `runner.rs:1142-2439`; element `damage.rs:13-81` | `TheBattle.cs:2148-2385` (Type1), `:2387-2668` (Type2), `:2669-3577` (status 3/4/11/12/14/15/16/18/19), element `:7191-7269` |
| 8 | HP/SP/EXP | ✅ MATCH | `engine.rs:96-126`; `data/texps.rs:16-31,40-55`, test `:90-107` | `getHpMax` `Data.cs:5537-5551`, `getSpMax` `:5553-5567`, `TexpGetLvUp` `:4701-4747` |
| 9 | Battle packets byte-faithful | ✅ MATCH (2 builder chết) | `packets.rs:9-153`, `skilling_*` `:167-256`; dead `battle_open_text`/`battle_open_pk_member` `:18-37` | literal `TheBattle.cs:684,748,1231,3611-3612,5047,6931`; spectator fan-out `:5006-5012` |
| 10 | 1 async task + channel input | ✅ MATCH | `manager.rs:130-176`; `submit_command` `service.rs:584-599`; `join_battle` `:523-536` | thiết kế lại theo Ch1 §1.4 (không copy race C#) |
| 11 | Golden battle | ⚠️ PARTIAL (1/2) | `golden/03-battle-win.golden` (seeds 12345/67890/1111); `tests/battle_golden.rs:103-170`; `tests/common/mod.rs:392-466` | §9.6 yêu cầu 1–2 |

### GAP đã chốt scope (implement trong ticket 20)

| GAP | Rust | C# | Quyết định |
|---|---|---|---|
| G1 — ActiveNPC 4712 trigger | hằng `construction.rs:544` chưa được dùng | `NpcOnMapWalk` `Data.cs:4939-5134`; trigger battle `:5026-5080`; frame di chuyển `F44408001602` `:5012,5094`; countdown delay `:5117-5121` | **Full walk parity**: `NpcWorld` registry runtime (map_id/id/npc_id/x,y/x_first,y_first/coord/delay/id_battle/so_luong) + task 900ms (kiểm tra mỗi tick thứ 3 ≈ 2.7s); player lẻ/đội trưởng, không trong trận, `dist(round) ≤ Coord` → set `_IdBattle=1`, `_My_TalkingBattle=id`, battle diahinh `4712` với `so_luong` bản sao npcId (slot mapping `_id3`; `_id3,_id4`; `_id2..4`; `_id2,3,4,8`; `_id1..5`); wander `F44408001602` gửi per-client (Sendpacket) + chase gửi map-wide (SendToAllMapid); **RNG stream thứ 4 `random_3`** riêng, time-seeded (Data.cs:46,87) |
| G2 — NPC respawn | `npc_respawn` `runner.rs:2588-2614` chỉ gọi ở flee `:2075`; `finish()` thiếu win-path `:2621-2736`; sink no-op `service.rs:132-134` | win `TheBattle.cs:4689-4724`; flee `:3176-3212`; `SendToAllMapid` `:3209,4722`; `_Delay=10` `:3210,4723`; điều kiện `_Delay==0` mới gửi | **Share `Arc<RwLock<NpcWorld>>` vào battle task** (qua `BattleData`): đọc `_Delay` TRƯỚC khi draw `random_2` (RNG-ordering `:4701,4719`); set `_Delay=10`; gọi `npc_respawn()` ở cả win + flee; broadcast map-wide `F44406001603`+le16(id)+`0A00`+`F44408001605`+le16(id)+le16(x)+le16(y); golden không đổi (world=None `manager.rs:147`) |
| G3 — Persist HP/SP/EXP | trong-memory `service.rs:241-285`; `persist_sessions_transaction` `persist.rs:235-290` chỉ Gold/BankGold + quest/homdo/trangbi | `PlayerUpdateDataId` `Data.cs:266+` | **Batch persist cuối trận**: mở rộng `persist_sessions_transaction` thêm `UPDATE players` Hp/Sp/Texp/Lv/Hpmax/Spmax/Point/SkillPoint + `upsert_pet` (pet texp) ở `battle_ended` cho mọi member (win/lose/flee), None pool = no-op; HP=1 khi thua (`runner.rs:2716-2720`) cũng persist |
| G4 — Leader SP regen (quan-su) | `Session.id_qs` chưa tồn tại; `id_mem/id_leader` `session.rs:227-234`; party op 0x0D chưa port (dispatcher `handler.rs:211-297`) | set quan-su op 0x0D sub 5 `Client.cs:1590-1638`, clear sub 6 `:1640-1648`; block regen `TheBattle.cs:4146-4247` | **Field + block + handler**: thêm `Session.id_qs`; block regen đầu vòng trong `run_turn` (`num109 = Round((QS._My_Int + QS._My_Int2) / 15.0)` cho leader cell + pet `(row^1,col)` + từng member cell (khác col leader) + pet, cap SpMax, DB write); handler tối thiểu op 0x0D sub 5 (chỉ leader, `num3 ∈ _IdMem1..4`) + sub 6 (clear) |
| G7 — Berserk fresh RNG | `runner.rs:1229-1230,1506-1507` | ephemeral `new Random()` `TheBattle.cs:2254,2471` | Giữ nguyên (đúng parity C#); deterministic chỉ vỡ khi buff type15 10016..10019 — golden không chạm |
| Golden 2 — TeamDef | — | — | **Seeded teamdef golden**: `start_teamdef_battle_seeded` (mở rộng `service.rs:426-452`), assert drop `F44408003504` + exp + `{3201}` + BattleQuestWin; NPC chọn từ `Data/` (TEAMDEF level thấp, 1–2 turn) |

Vị trí chính: battle engine C# `ts_server_old/Server_TS_Online/TheBattle.cs`; NPC walk `Data.cs`; party/quan-su `Client.cs` (op 0x0D).
## Comments

### 2026-08-12 — Implementation complete (ticket 20)

All four real GAPs + golden 2 were implemented; status moved `ready-for-agent → resolved`.

- **G1 — ActiveNPC 4712 trigger**: new `src/battle/npc_world.rs` — `NpcWorld` registry (`NpcEntry` with map_id/id/npc_id/x/y/x_first/y_first/coord/delay/so_luong/id_battle), `random_3` 4th stream, and `walk_tick` (900 ms cadence, chase/wander on every 3rd tick, `F44408001602` per-client + map-wide frames, `_IdBattle`/`_Delay` lifecycle, SoLuong→TeamDef slot mapping `_id3`/`_id3,_id4`/`_id2..4`/`_id2,3,4,8`/`_id1..5` with DiaHinh 4712). `BattleService::spawn_npc_walk` drives the task; `BattleService` implements `WorldSink`.
- **G2 — NPC respawn**: `Arc<RwLock<NpcWorld>>` shared into the battle task via `BattleData.world`; `npc_respawn` reads `_Delay` before drawing `random_2`, writes `_Delay=10`, and the win path (`finish(won=true)`) respawns alongside flee; map-wide `F44406001603`/`F44408001605` via the sink. Golden unchanged (`world=None`).
- **G3 — Batch persist**: `persist_sessions_transaction` gained a `"stats"` table (Hp/Sp/Texp/Lv/HpMax/SpMax/Point/SkillPoint) and `battle_ended` persists every member (win/lose/flee) with `["stats","pet"]` (pet texp); None pool = no-op.
- **G4 — Quan-su SP regen**: `Session.id_qs` + `Battle.leader_id_qs`/`leader_qs_int` snapshots; per-turn regen block in `run_turn` (`Round((QS.Int+QS.Int2)/15.0)` for leader/member cells + pets, capped at SpMax, DB writes); op 0x0D sub 5 (leader-designates-member) + sub 6 (clear) in new `src/server/handlers/party.rs`, dispatched in `handler.rs`.
- **G7 — Berserk RNG**: kept as-is (C# parity), documented only.
- **Golden 2 — TeamDef**: `start_teamdef_battle_seeded` (DiaHinh 4712, one low-level defender, seeds 4/8/2) — `tests/battle_golden.rs::teamdef_battle_win_golden_replay` asserts drop `F44408003504`, exp write, `{3201}` turn action, and BattleQuestWin red message + EndTalk.

Verification: `cargo test` fully green (294 lib + golden suite + battle goldens + web dashboard); existing `golden/03-battle-win.golden` unchanged.

#### 2026-08-12 — Code-review follow-up (commits 4106774, 509c4b7)

Two-axis review (Standards + Spec) ran against commit 4106774; the fixes in 509c4b7:

- **G2**: `npc_respawn` no longer writes the drawn X/Y into the world entry — C# writes only `_Delay = 10` (TheBattle.cs:4723); coords are broadcast map-wide but not stored.
- **G1**: `teamdef_for_so_luong` → `Option`; a SoLuong outside 1..=5 chases (frame + map broadcast) but never engages a battle (C# switch has no default case). Test added.
- Removed dead `WarInfo.id_qs`/`qs_int` cell fields (regen uses the Battle-level leader snapshot).
- Dedup: shared `patrol_bounds()` (walk + respawn clamp) and `send_to_map()` (wander/chase + respawn fan-out) in `npc_world.rs` / `service.rs`.
- Documented the G4 member-gate fidelity note: members aren't grid cells in this port, so the C# `_My_IdMem1..4` gate is vacuous until members are grid cells; the QS Int snapshot at spawn is the race-free async-task contract.
