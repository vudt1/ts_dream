# 20 — Battle engine: construction + turn loop + damage pipeline (Ch6)

**What to build:** Cốt lõi battle engine — tạo battle (các trigger `TheBattle`), grid `ListWar` + `_keys`, 3 RNG streams riêng biệt, vòng turn `Battling` (async task per battle), targeting, damage pipeline (banker's rounding), HP/SP/EXP tính bằng công thức, và bộ packet battle byte-faithful. Một trận player-vs-NPC chạy đúng byte-wire.

**Blocked by:** 03 — Static data (Npcs, Skills); 13 — Skills (battle dùng skill của player); 17 — Pets (pet vào battle); 08 — Move (battle-state kiểm tra đang battle); 19 — Talk quests (trigger battle từ `[TEAMDEF]`).

**Status:** ready-for-agent

- [ ] Domain: `WarInfo` đầy đủ (type/id/idNpcOnMap/idChar/row/col/HP/SP/Lv/thuoctinh/leader/skills/buffs/_Attacked/_Random/_Exp/_Packet — `_Packet` = 23-byte snapshot, Ch6 §6.0).
- [ ] 3 RNG streams (`.NET`-style time-seeded, riêng biệt): `random_0`(drop/skill pick), `random_1`(tie-break per-turn `_Random` + jitter), `random_2`(npc respawn) — không merge (Ch6 §6.0).
- [ ] Construction triggers map sang Diahinh: PK `TheBattle(id1,id2,112)`; NPC attack 112; Quest/TeamDef `TeamDef[0]`; ActiveNPC `4712`. `IdBattle = IdBattleCount++` (start 1, assign trước increment) (Ch6 §6.1).
- [ ] `AddToBattle` (team row==0?1:2; load HP/SP; sum Int2..; leader col2 load ≤4 pets `(row^1, col)` từ SttPet..+3, Type4; mỗi member col1/3/0/4 một pet; overlaps member overwrite theo dict order). `AddNPCToBattle` Type/npc, Team2. `ChangedWar` rebuild `_Packet` (Ch6 §6.1).
- [ ] Vòng `Battling` (async task): win/lose check (enemy dead vs player dead); reset & buff ticks (burn/poison + DB write cho non-npc + broadcast `{3201}`); input wait ≤~21s poll 100ms (auto npc skill `GetRandomSkillNPC`); turn order `Attacked DESC, Agi DESC, Random DESC`; action execution (Ch6 §6.2/§6.3).
- [ ] Targeting pickers (GetPosAttack/combo/TG/3_15/GiaiTru/Type4/honLoan) — `_Diahinh` không ảnh hưởng targeting/damage (Ch6 §6.4).
- [ ] Damage pipeline đúng: `double` + `Math.Round` (banker's) → int; Type1 physical (Atk×Element×2.0−Def×1.6, lv-diff, GetDamageSkillInt×DoManh×(1+skillLv×0.033), num37 2.0/2.6 combo, hit-roll miss %, min 1 + jitter, reflect 13003, buffs); Type2 magic (dùng `_Int`, no num37); status 3/4/15/19, catch(bắt)/flee/heal/cleanse (Ch6 §6.5).
- [ ] HP/SP/EXP: `getHpMax` (rb0 floor(lv^0.35+1)*hpx*2+80+lv …), `getSpMax`, `TexpGetLvUp` (loop Texps, max 200) — exponent 0.35/0.25 exact (Ch6 §6.6).
- [ ] Battle packets byte-faithful (Ch6 §6.8): open `F4441C000BFA`, entity `F4441A000B0503/0505`+`_Packet`, `F4440A000B0402`, hide `F44408000B00`, reposition `F44405000B01`, clear pet `F44404000B01`, your-turn `F44402003401`, acting `F44404003505`, action `{3201}`, skillcast `F444130032010F00`, buff-end `F44407003501`, drop `F44408003504`, status `F4440C000801`, battle-end `F44403000B0A01`. DiaHinh echo (PK-member open 7000).
- [ ] Battle state trên mỗi async task (race-free, deterministic) (Ch1 §1.4).
- [ ] Golden: 1–2 battle sample scenarios được ghi lại (Ch9 §9.6).