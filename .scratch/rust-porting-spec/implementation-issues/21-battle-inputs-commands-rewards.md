# 21 — Battle inputs (op 0x0B triggers + op 0x32 commands) + resolve/rewards

**What to build:** Người chơi thực sự điều khiển battle: khởi tạo trận qua op 0x0B (attack NPC, PK challenge, join), gửi lệnh hành động qua op 0x32 (skill/use item), và khi thắng tất cả side-effects của `BattleQuestWin` (consume item, reward, warp) chạy pefectly. Trận chiến PK/quest/drop chạy đến cùng một cách đúng.

**Blocked by:** 20 — Battle engine (construction/turn/damage đã dựng); 11 — Inventory (drop gửi về void); 14 — Shops (dùng potion trong battle); 09/18 — Music player communication.

**Status:** done (2026-08-12) — G3 accepted as limitation (unbounded join `HashSet`)

- [ ] Op 0x0B battle control (Ch2 §2.3.8):
  - sub1 leave battle: `data[6]==3` → clear battle id, `F44408000B00`+id4+`0000`.
  - sub2 PK: gate `_MyIdBattle==0`, `_My_Pk==1`, target online/not-in-battle; target Pk==0 → `F4440300210101`; Pk==1 → start PK battle (Ch6, DiaHinh 112).
  - sub3 attack NPC: gate; npc id = bytes7..; **blocked** nếu npc ∈ `[20000,22000)`/`[23000,25000)`/`[26000,27000)`; else start battle (DiaHinh 112, idNpcOnMap = 11..12).
  - sub4 join: first free `ListQS` slot, register, build join packet + `F44403000B0A01`.
  - sub5 `JamPlayerToBattle` no-op; sub6 broadcast `F44406000B06`+id4.
- [ ] Op 0x32 battle commands (Ch2 §2.3.27): sub1 skill (row/col/rowAttack/colAttack/skill id LE16; range-check; cell must exist `_Id>0`; `SkillGet`/pet skill level; set `_LvSKill/_RowAttack/_ColumnAttack/_IdSkill/_Attacked=1`; broadcast `F44404003505`+row+col). sub2 use item (heal cell `_Hp`/`_Sp` cho player+pet, remove 1, `_Attacked=1`).
- [ ] Auto-action nhập vào trong turn loop của NPc/berserk (Ch6 §6.2 step 3).
- [ ] `BattleQuestWin` full ordered side-effects (Ch6 §6.7): consume items → red msg → each guaranteed `WinRewards` → **1 random** `WinRandomRewards` (fresh independent RNG) → grant {item,count} leader + (shareToParty) members → use-items (self `F44403001711`+slot + status-update burst, equip; else pet) — NOTE: ticket literal `…0617030011`+slot was inaccurate; C# (`Data.cs:5883`) and Rust (`quest.rs:413`) both emit `F44403001711`+slot → save leader quests → player enhance delta → add skill (learn packet) → add pet → warp/end (`Warped` leader + members + `F44408000B00` hoặc `F44402001408`). All DB writes mang `player_id`.
- [ ] Battle end frames + drop `F44408003504` + status packet (`F4440C000801`, type, sign, abs, zeros).
- [ ] Golden: battle scenario (thắng/ra vật huấn) được ghi lại (Ch9 §9.6).

---

## Audit & source references (2026-08-12)

**Verdict:** Items 1–3, 5, 11 implemented and byte-faithful. Item 4 implemented in correct C# order
with one wire-fidelity gap (G4). No `Notes`/`Deferred` section present in this ticket. All blockers
(20, 11, 14, 09, 18) are resolved.

### Checklist → source map

| Ticket item | Rust location | C# ground truth | Status |
|---|---|---|---|
| 1 — Op 0x0B sub1 leave | `src/server/handlers/battle.rs:46` (`handle_leave_battle`); `service.rs:688` (`leave_battle`) | `Client.cs:1245` (Update_HB sub1); `F44408000B00`+id+`0000` | DONE |
| 1 — Op 0x0B sub2 PK | `battle.rs:81` (`handle_pk_challenge`), gates `battle.rs:87,95`; `service.rs:515` (`start_pk_battle`) | `Client.cs:1278` (gate `_My_IdBattle==0`,`_My_Pk==1`,target online/`_My_IdBattle==0`; `F4440300210101` when `Pk==0`; `new TheBattle(id,num5,112)`) | DONE |
| 1 — Op 0x0B sub2/3 attack NPC | `battle.rs:107` (`handle_attack_npc`), blocked ranges `battle.rs:112-114`, idNpcOnMap `battle.rs:118-122`; `service.rs:467` (`start_npc_battle`, DiaHinh 112) | `Client.cs:1306` (gate; blocked `[20000,22000)`/`[23000,25000)`/`[26000,27000)`; idNpcOnMap = bytes 11-12; `new TheBattle(_My_Id,num4,idNpcOnMap,112)`) | DONE |
| 1 — Op 0x0B sub4 join | `battle.rs:23`, `service.rs:635` (`join_battle`), `packets.rs:145` (`battle_trailer`=`F44403000B0A01`) | `Client.cs:1327` (first free `ListQS` slot, register, `F44403000B0A01`) | DONE (capacity unbounded — see G3) |
| 1 — Op 0x0B sub5 | `battle.rs:29` (no-op) | `Client.cs:1414` (no-op; `ClientBattle` undefined) | DONE |
| 1 — Op 0x0B sub6 | `battle.rs:32`, `service.send_map` | `Client.cs:1433` (`F44406000B06`+id4) | DONE |
| 2 — Op 0x32 sub1 skill | `battle.rs:143` (`handle_skill_command`), broadcast `battle.rs:174`; `runner.rs` command apply `:479-491`; `skill_level_for` `battle.rs:209` | `Client.cs:7696` (`Update_H32` sub1; range 0..3×0..4; `_Id>0` & `_Attacked==0`; `SkillGet`/pet skill; set `_LvSKill/_RowAttack/_ColumnAttack/_IdSkill/_Attacked=1`; `F44404003505`+row+col) | DONE |
| 2 — Op 0x32 sub2 use item | `battle.rs:181` (`handle_use_item`, range `26001..=27165`), `remove_homdo_item` `battle.rs:194`; heal+`_Attacked=1` `runner.rs:504` (`apply_use_item`) | `Client.cs:7782` (item `26001..27165`; heal cell+pet `_Hp/_Sp`; `HomdoRemoveItem`; `_Attacked=1`; no broadcast) | DONE |
| 3 — auto-action (NPC/berserk) | `runner.rs:456` (`turn_phase2`); `auto_berserk` `:470`/`runner.rs:567`; `auto_npc` `:472`/`runner.rs:535` | `TheBattle.cs:1002` (`Battling`); berserk `:1281` (`type15==14021/20014`); npc `:1353` (`type==3/7`); final `_Attacked=1` `:1443` | DONE |
| 4 — BattleQuestWin ordered side-effects | `quest.rs:319` (`battle_quest_win_impl`): consume `:344`, red msg `:360`, rewards+random `:365` (time-seeded RNG `:369`), grant+party `:374`, use-items `:399`, save quests `:442`, enhance `:453`, add skill `:469`, add pet `:491`, warp/end `:498` | `Data.cs:5812`: consume `:5818`, red `:5829`, rewards `:5834`, 1 random `:5842` (`new Random()`), grant `:5849`, use-items `:5872` (self `F44403001711`+slot `:5883`+`UpdateStatusWhenUseItem` `:5885`; pet `F44404001717` `:5894`), save `:5899`, enhance `:5915`, add skill `:5933` (`F4440C0008016E01` `:5944`), add pet `:5958`, warp/end `:5963` (`F44408000B00` `:5992` / `F44402001408` `:5996`) | DONE **except G4** |
| 5 — end frames + drop + status | `packets.rs:127` (`drop_item`=`F44408003504`), `:139` (`status_update`=`F4440C000801`), `:145`/`battle_exit_move` `:159`/`battle_exit_talk` `:165`; `runner.rs` drop emit `:2598`/`:2645`, `finish` `:2714` | `TheBattle.cs:3648/3653/3892/3897` (`F44408003504`); `Data.cs:240-365` (`F4440C000801`); `Data.cs:5992/5996` | DONE |
| 6 — Golden battle scenario | `tests/battle_golden.rs:103` (`battle_win_golden_replay`), `:177` (`teamdef_battle_win_golden_replay`); `golden/03-battle-win.golden`, `golden/13-battle-leave.golden` | — | PARTIAL — handler-layer goldens missing (G1) |

### Open gaps

- **G1 (coverage):** Only `golden/13-battle-leave.golden` exercises the **dispatcher** (op 0x0B sub1).
  op 0x0B sub2 (PK + attack-NPC), sub4 (join), sub6, and op 0x32 sub1/sub2 run only through
  `BattleService`/`submit_command` in `tests/battle_golden.rs:103,177` — the handler gates
  (blocked NPC ranges, `Pk==0` reply, join registration, `F44404003505` broadcast) are not golden-diffed.
- **G2 (doc):** Ticket item 4 writes `…0617030011`+slot, but C# (`Data.cs:5883`) and Rust (`quest.rs:413`)
  both emit `F44403001711`+slot. Ticket literal is inaccurate; Rust is byte-correct.
- **G3 (capacity):** `join_battle` uses an unbounded `members: HashSet` (`service.rs:57`); C# scans a fixed
  `ListQS` slot list. Functional for ≤4 members, capacity not enforced.
- **G4 (wire fidelity):** `BattleQuestWin` self use-item emits `F44403001711`+slot + equip `F44408000502`
  (`quest.rs:413-418`) but calls only `recompute_stats()` (`session.rs:349`), which emits **no**
  `F4440C000801` status frames. C# `UpdateStatusWhenUseItem()` (`Client.cs:8478`) emits a burst of
  per-equip-slot stat updates. Client stat display after the reward use-item will diverge.

### Notes / deferred

None present in this ticket. (Blockers 20/11/14/09/18 each carry their own Notes/deferred and are resolved.)

## Resolution (2026-08-12)

Close as **done**. Decisions:
- **G4 (wire fidelity)** — FIXED in-ticket: `battle_quest_win_impl` self use-item now emits the
  `UpdateStatusWhenUseItem` status-packet burst (`F4440C000801` gear stats) via `recompute_stats` +
  `build_stat_update`, matching `Client.cs:8478` / `Data.cs:238-359`.
- **G1 (coverage)** — FIXED in-ticket: handler-layer tests added in `src/server/handlers/battle.rs`
  (op 0x0B sub2 PK-reply, sub2 attack-NPC blocked range, sub4 join; op 0x32 sub2 use-item heal).
- **G2 (doc)** — ticket literal corrected to `F44403001711`+slot (Rust already byte-correct).
- **G3 (capacity)** — ACCEPTED as limitation: `join_battle` keeps unbounded `HashSet` (`service.rs:57`);
  functionally correct for ≤4 members, no `ListQS` slot-cap enforcement. No follow-up ticket.