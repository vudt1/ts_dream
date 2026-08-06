# 19 — Talk quests + bảng H6 + exceptions (op 0x14, data-driven)

**What to build:** Toàn bộ máy quest/menu data-driven của FTalk.H6 — bảng H6 compiled (~45 map cases, ~228 idtalking branches, 176 literal packets), daily-quest generator (đúng **21 RNG draws**), pet-reborn NPCs, và quest warps/rewards qua `[TEAMDEF]`/`[OnWin]`. Hoàn thiện tương tác NPC + hệ quest. Nền để battle/quest-win (Ch6) và các battle trigger `[TEAMDEF]`.

**Blocked by:** 18 — TALK core/dialog/H6 menus (H6 dispatch đã dựng); 03 — Static data (`Quests/*.ini`); 13 — Skills (quest reward AddSkill); 11 — Inventory (reward/give item; use-items).

**Status:** ready-for-agent

- [ ] Bảng H6 data table transcribe verbatim từ `spec/` addendum (≈45 map cases, ~228 idtalking, 176 packets) (Ch2 §2.6.2).
- [ ] Daily-quest generator (map `12711`): đúng **21 `random.Next` draws** thứ tự trong research `06` §(6); item formulas `62001+num3*100` / `62101+num4*100`; reward `value1..48`; **RNG fresh time-seeded, riêng khỏi 3 battle streams** (Ch2 §2.6.2, H6 RNG parity).
- [ ] Pet-reborn NPC `55002/59102/59011` exceptions.
- [ ] Điều kiện quest failure: requirement fail → `…010107…` / `…01 03`+id+`BB`; `_RequireSelectMenu` mismatch → `LoseDialogs[0]`/EndTalk (Ch2 §2.6.3).
- [ ] warp-talk (H8) hoạt động: hoàn chỉnh warp path (vào `0x0C` confirm).
- [ ] `[TEAMDEF]` triggers battle: khi dialog exhaustion + `[TEAMDEF]` non-zero Diahinh → đưa tới battle (ticket 20). Rewards `[OnWin]`/`[OnLose]` đầy đủ (Dialogs, WarpTo, Rewards, RandomRewards(1 random draw fresh RNG), UseItems, SaveLeaderQuests, PlayerEnhanceData, AddSkill, AddPet, ClickNpcId).
- [ ] Quest step book keeping với `player_id` (DELETE Quest scoped Ch5 §5.4).
- [ ] Golden: quest scenario (FTalk.H6) & warp được ghi xác minh byte.