# 21 — Battle inputs (op 0x0B triggers + op 0x32 commands) + resolve/rewards

**What to build:** Người chơi thực sự điều khiển battle: khởi tạo trận qua op 0x0B (attack NPC, PK challenge, join), gửi lệnh hành động qua op 0x32 (skill/use item), và khi thắng tất cả side-effects của `BattleQuestWin` (consume item, reward, warp) chạy pefectly. Trận chiến PK/quest/drop chạy đến cùng một cách đúng.

**Blocked by:** 20 — Battle engine (construction/turn/damage đã dựng); 11 — Inventory (drop gửi về void); 14 — Shops (dùng potion trong battle); 09/18 — Music player communication.

**Status:** ready-for-agent

- [ ] Op 0x0B battle control (Ch2 §2.3.8):
  - sub1 leave battle: `data[6]==3` → clear battle id, `F44408000B00`+id4+`0000`.
  - sub2 PK: gate `_MyIdBattle==0`, `_My_Pk==1`, target online/not-in-battle; target Pk==0 → `F4440300210101`; Pk==1 → start PK battle (Ch6, DiaHinh 112).
  - sub3 attack NPC: gate; npc id = bytes7..; **blocked** nếu npc ∈ `[20000,22000)`/`[23000,25000)`/`[26000,27000)`; else start battle (DiaHinh 112, idNpcOnMap = 11..12).
  - sub4 join: first free `ListQS` slot, register, build join packet + `F44403000B0A01`.
  - sub5 `JamPlayerToBattle` no-op; sub6 broadcast `F44406000B06`+id4.
- [ ] Op 0x32 battle commands (Ch2 §2.3.27): sub1 skill (row/col/rowAttack/colAttack/skill id LE16; range-check; cell must exist `_Id>0`; `SkillGet`/pet skill level; set `_LvSKill/_RowAttack/_ColumnAttack/_IdSkill/_Attacked=1`; broadcast `F44404003505`+row+col). sub2 use item (heal cell `_Hp`/`_Sp` cho player+pet, remove 1, `_Attacked=1`).
- [ ] Auto-action nhập vào trong turn loop của NPc/berserk (Ch6 §6.2 step 3).
- [ ] `BattleQuestWin` full ordered side-effects (Ch6 §6.7): consume items → red msg → each guaranteed `WinRewards` → **1 random** `WinRandomRewards` (fresh independent RNG) → grant {item,count} leader + (shareToParty) members → use-items (self `…0617030011`+slot, status update, equip; else pet) → save leader quests → player enhance delta → add skill (learn packet) → add pet → warp/end (`Warped` leader + members + `F44408000B00` hoặc `F44402001408`). All DB writes mang `player_id`.
- [ ] Battle end frames + drop `F44408003504` + status packet (`F4440C000801`, type, sign, abs, zeros).
- [ ] Golden: battle scenario (thắng/ra vật huấn) được ghi lại (Ch9 §9.6).