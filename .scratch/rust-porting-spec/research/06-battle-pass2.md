# 06 — Battle pass 2: resolution of the 6 remaining gaps (TS Dream)

Answers to the gaps left open by `04-battle-engine.md` (ticket **Battle pass 2**, issue 09). Each claim cites `file:line` from the C# ground-truth in `ts_server_old/`.

---

## (1) The checksum / framing `smethod_5` — XOR 0xAD, no appended checksum

The earlier research (§0) guessed `smethod_5` "appends a 1-byte checksum". **That is wrong.** The complete send path is two pure byte transforms and nothing else:

```
hex string packet  --(Class5.smethod_4)-->  byte[]
byte[]             --(Class5.smethod_5)-->  byte[]   (each byte XOR 0xAD)
socket.Send(array)
```

- `Class5.smethod_4(string)` — `Class5.cs:132-152`: converts an even-length ASCII-hex string to bytes via `byte.Parse(..., HexNumber)`. Every battle script builds a hex-string like `"F444..."...` then sends through one of the `Server.SendTo*` helpers.
- `Class5.smethod_5(byte[])` — `Class5.cs:153-166`: returns a NEW array where `array[i] = byte_0[i] ^ 173` (173 = 0xAD). It does NOT append anything; it XORs every byte in place-length.
- Send sites — `Server.cs:524, 550, 577, 609, 641, ...`: `Server.SendToClient(...)`, `SendToAllClient(...)`, `SendToAllClientMapid(...)` all do `Class5.smethod_5(Class5.smethod_4(packet))` then `socket.Send`.

So the byte-faithful Rust port is: build the hex string `"F4440200..."`, hex-decode, XOR each byte with `0xAD`, write all bytes to the socket. No checksum, no CRC, no trailer. (Class7.cs:56 `smethod_5()` is unrelated — it is a localised-string resource accessor.)

This matches the map's standing framing decision: XOR `0xAD`, frame prefix `F4 44`.

---

## (2) `GetTurn(int IdSkill, int LvSKill)` — case 14013 fall-through resolved

`TheBattle.cs:9231-9316`. The prior research (§5.4) left `case 14013` unresolved. Full control flow:

```
num = 3
if IdSkill NOT in GROUP_a {13002,14008,13003,13005,13012}:
  if IdSkill NOT in GROUP_b {10033,10015,10026,13021,13025,13032,10025,14020,12025,14040,14044,14046,14053}:
    if IdSkill NOT in GROUP_c {10004,11002,12024,13011,13030,14015,14029,20018,11024,11032,13020}:
      if IdSkill NOT in GROUP_d {13015,13016,13017,13018,10016,10017,10018,10019}:
        if IdSkill NOT in GROUP_e {11014,20014,20022,20023}:
          if IdSkill NOT in GROUP_f {20025,20026,20027,10010,10031,13014,20024,14012}:
            switch IdSkill:
              default: return 3
              case 14021: return {1->2,2->3,3->4,4->5,5->6, else->3}   // on LvSKill
              case 14013: break          // <-- falls through of the switch body (break)
            // after the switch, control continues to the ladder below:
            switch LvSKill: {1|2|3->2; 4|5|6->3; 7|8|9->4; 10->5; else->3}
          return 5                    // GROUP_e
        return 4                      // GROUP_d
      return 3                        // GROUP_c
    switch LvSKill: {1|2->2; 3|4->3; 5->4; else->3}    // GROUP_b
if LvSKill-1 > 1: if LvSKill-3 <= 2: num=3    else num=2   ; return num   // GROUP_a
```

**Answer for 14013**: `case 14013` matches, `break`s out of the inner `switch(IdSkill)`, and then runs the *same* LvSKill ladder as GROUP_f — i.e. `14013` returns `2/3/4/5` for LvSKill `1-3 / 4-6 / 7-9 / 10`, else `3`. It is effectively grouped with GROUP_f's ladder even though the skill ID is not in that `if` list. Any skill ID not in any group and not 14021/14013 returns `3`.

Note the `14021` case returns via its own LvSKill switch and does **not** reach the ladder.

Port guidance: transcribe the exact nested `if` + `switch` structure, NOT a flat lookup — the fall-through order is observable.

---

## (3) Healing / mana item values — `GetDataItem`

Value source: `Data.GetDataItem(int _id, string type)` (`Data.cs:4253+`) returns the item's stat from the `Items` data (loaded from Items.txt → `Data_Items`). The two fields for healing:

- `case "Hp": result = items._Hp;` (`Data.cs:4270-4272`)
- `case "Sp": result = items._Sp;` (`Data.cs:4273-4275`)

So a heal/mana item's per-use value is `GetDataItem(itemId, "Hp")` and `GetDataItem(itemId, "Sp")` — the `_Hp`/`_Sp` columns of the item record.

The compensated after-battle HP/SP restoration (relevant to battle, though not an in-battle item): `Client.BattleStopped()` (`Client.cs:9646-9701`) — when the player leaves battle with `_My_SP_Store > 10000 && _My_HP_Store > 10000`:
- `num = _My_SpMax - _My_Sp`, `num2 = _My_HpMax - _My_Hp`.
- If stores exceed the remaining deficit → `_My_SP_Store -= num; _My_Sp_new += num` (same for HP).
- Else → `_My_Sp_new += _My_SP_Store; _My_SP_Store = 0` (same for HP).
- If an active pet (`_My_SttPetXuatChien > 0`) and stores still cover both deficits, the pet's HP/SP are set to `PetGetData(Hpmax/Spmax)` for the 4 carried pet stts.

Port note: the in-battle heal/mana amount is purely the item record's `_Hp`/`_Sp`; whether the heal is capped to `_HpMax`/`_SpMax` is at the consume site — capture-based tests should verify the cap by diffing a real consume (see ticket **Thiết kế schema MySQL 8** and the acceptance harness ticket for capture infra).

---

## (4) HP/SP max curves & the exp/level-up curve

**`Data.getHpMax(int rb, int job, int level, int hpx)`** — `Data.cs:5537-5551` (formula, not table):
```
rb 0: floor( (level^0.35 + 1.0) * hpx * 2.0 + 80.0 + level )
rb 1: floor( (level^0.35 + 2.0) * hpx * 2.0 + 180.0 + level )
rb>=2: job switch:
   1: floor( (level^0.35 * 2.0 + 25.0) * hpx + 280.0 + level )
   2: floor( (level^0.35 * 3.0 + 30.0) * hpx + 380.0 + level )
   3: floor( (level^0.35 + 11.5) * hpx * 2.0 + 180.0 + level )
   else: floor( (level^0.35 + 10.5) * hpx * 2.0 + 180.0 + level )
```
All in `double`, rounded with `Math.Round` (banker's rounding), cast `(int)`. Note consistent `Math.Pow(level, 0.35)`.

**`Data.getSpMax(int rb, int job, int level, int spx)`** — `Data.cs:5553-5567`:
```
rb 0: floor( level^0.25 * spx * 2.0 + 60.0 + level )
rb 1: floor( level^0.25 * spx * 2.0 + 110.0 + level )
rb>=2: job:
   1: floor( level^0.25 * spx * 2.0 + 160.0 + level )
   2: floor( level^0.25 * spx * 2.0 + 160.0 + level )
   3: floor( level^0.25 * spx * 3.0 + 310.0 + level )
   else: floor( level^0.25 * spx * 3.5 + 410.0 + level )
```
Same `Math.Pow(level, 0.25)` for all branches.

**Exp curve — `Data.TexpGetLvUp(int _Lv, int _Reborn, int _Texp)`** — `Data.cs:4701-4747`. It is a **data-driven loop over the `Texps[]` table** (loaded from the `Texps.txt` data file), not a look-up formula:
```
result = 0
if _Lv < MaxLevel:
  for i in _Lv .. MaxLevel-1:
    switch _Reborn:
      0: if _Texp < Texps[i]._0 : return result
         if _Texp >= Texps[i]._0 : result = i - _Lv + 1
      1: (same with Texps[i]._1)
      2: (same with Texps[i]._2)
return result
```
So it returns the number of level-ups gained by falling below the current level's threshold. `MaxLevel`, `Texps[][]` (thresholds per reborn column 0/1/2) come from the Texps data (see the Data File Formats research asset for how `Texps.txt` loads). The port should replicate this exact ascending-loop semantics — not a closed form.

---

## (5) `BattleQuestWin` — all side-effects, in order

`Data.cs:5812-5998`, `void BattleQuestWin(Client _client, Key_Talk key)`.

Precondition: `if (!Data_Talks.ContainsKey(key)) return;` — the talk must exist. Then effects in order:

1. **Consume required items** (`Data_Talks[key]._RequireItems`, list of `int[3] {itemId, count, ???}`): for each, find the compacted slot via `HomdoGetSlotExits(conn, item[0])`; if slot found (`num>0`), item count `>= item[1]`, and `item[2] > 0` → `HomtoRemoveItem(_client._My_Id, item[0], item[2])`.
2. **Send red message**: `_client.SendRedMessage(Data_Talks[key]._Message)` if non-null.
3. **Collect guaranteed win-reward list**: all `_WinRewards` (each `int[2]{itemId,count}`).
4. **Random win reward**: if `_WinRandomRewards.Count>0` → `new Random()`, `index = random.Next(_WinRandomRewards.Count)`, add `_WinRandomRewards[index]` (one random pick — independent time-seeded Random, not battle streams).
5. **Grant items** to leader and optionally members: each reward `{list[i][0], list[i][1]}` added via `HomtoAddItem(_client._My_Id, ...)`; if `list[i][2] > 0` also `HomtoAddItem` to each `_My_IdMem1..4 > 0`.
6. **Use items** (`_WinUseItems`, each `int[3]{itemId, ???=0/1, ...}`): for `num3==0`: `Sendpacket("0617030011"+slot:X2)`, `HomtoUseItemTB`, `UpdateStatusWhenUseItem`, `Server.ServerSend_EquitItem(_My_Id, itemId)`. For `num3 != 0` (pet used): via `_My_SttPetXuatChien`, `tbslot = loai + my_SttPetXuatChien*10`, packet `"F44404001717"+...`, `HomtoUseItemTB_Pet`, `UpdateStatusPetWhenUseItem`.
7. **Save leader quests** (`_WinSaveLeaderQuests`): `target` `int[3]{npcId, npcVal, warpVal, plus}` — if `npcVal>0`: `QuestUpdateDataNpc(item[0], item[1], item[3])`; if `warpVal>0`: `QuestUpdateDataWarp(...)`.
8. **Player-enhance** (`_WinPlayerEnhanceData`): each `object[2]{field, delta}` — `PlayerGetDataById(_My_Id, field)`; if not null, `PlayerUpdateDataId(_My_Id, field, cur+delta)`.
9. **Add skill**: `_WinAddSkill` = `int[2]{skillId, lv}`. If `Data_Skills.ContainsKey(skillId)` and `skillId>0` and `!SkillExits` → `SkillAdd(skillId, lv, cost, 0)`, `SendRedMessage("เรียนสก�ล "+name+" สำเร็จ")`, packets `"F4440C0008016E01"+smethod_12(lv)+smethod_12(skillId)` and skillpoint packet; else a failing red message.
10. **Add pet**: if `_WinAddPet>0` → `Addpet(_My_Id, petId)`.
11. **Warp/end**: `_WinWarpTo` (`int[3]{mapId,x,y}`) — if `Typetalk=="WARP"` and no array, resolves from `Data.GetDataWarp(_My_MapId, idtalking, MapId2/X/Y)`; if `array[0]>0` → `Warped(...)` leader + members + `_My_WarpingId=0`, broadcast `"F444080B00"+smethod_12(_My_Id)+"0000"`. Else `Sendpacket("F44402001408")`.

These DB operations are the exact set that the shared-schema `player_id` scoping rule (issue 11) must ride on: remove-item / add-item / skill / pet / player-stat updates, plus the `F444...` packets that must be reproduced byte-for-byte by the executor.

---

## (6) RNG parity — the H6 daily-quest generator

Located at the FTalk handler for daily-quest NPC (map `12711` branch start `FTalk.cs:385-513`). **Critical finding**: the whole block uses a **freshly created `Random random = new Random()` at `FTalk.cs:387`** — a separate, time-seeded `.NET Random` per invocation, NOT one of the three battle streams (`random_0/1/2`). So battle RNG state is entirely irrelevant here; the port must spin up an independent RNG for this block.

Exact `random.Next(...)` call sequence (in call order), each consuming one draw:

| # | call | range | var |
|---|------|-------|-----|
| 1 | `Next(0,7)` | 0..6 | `num3` |
| 2 | `Next(0,6)` | 0..5 | `num4` |
| 3 | `Next(0,4)` | 0..3 | `num5` |
| 4 | `Next(0,9)` | 0..8 | `num6` |
| 5 | `Next(0,150)` | 0..149 | `num7` |
| 6 | `Next(47028,47369)` | | `id` |
| 7 | `Next(48031,48104)` | | `id2` |
| 8 | `Next(47028,47369)` | | `id3` |
| 9 | `Next(48031,48104)` | | `id4` |
| 10 | `Next(47028,47369)` | | `id5` |
| 11 | `Next(61029,61091)` | | `id6` |
| 12 | `Next(61097,61223)` | | `id7` |
| 13 | `Next(61029,61091)` | | `id8` |
| 14 | `Next(61097,61223)` | | `id9` |
| 15 | `Next(46184,46204)` | | `id10` |
| 16 | `Next(62838,62845)` | | `iD` |
| 17 | `Next(46900,46907)` | | `iD2` |
| 18 | `Next(14283,14286)` | | `id11` |
| 19 | `Next(46395,46399)` | | `num8` |
| 20 | `Next(46395,46399)` | | `iD3` |
| 21 | `Next(0,7)` | 0..6 | `num10` |

(That is **21 draws** for this block, in this exact order. `num5/num6/num7` and the item ids `id Id2..iD3` are all pre-computed here even if the chosen menu branch then uses only some of them; a byte-faithful port MUST not elide unused draws — every `random.Next` consumes state.)

Item-id / reward formula (`FTalk.cs:500-507`): the reward uses `num3` (0..6 → 7 groups) and `num4` (0..5):
- `num35 = 62001 + num3*100` ; `iD4 = 62101 + num4*100`
- `num36 = 62002 + num3*100` ; `iD5 = 62102 + num4*100`
- `num37 = 62003 + num3*100` ; `iD6 = 62103 + num4*100`
- `num38 = 62004 + num3*100` ; `iD7 = 62104 + num4*100`

The reward magnitudes are the precomputed `value1..value48` set (`FTalk.cs:452-499`) selected per menu branch later in the `switch (num2)`; the mapping to each menu option is below the switch (not transcribed here — it just references `valueN` and the computed item ids).

Port guidance: keep the untouched helper `random`, consume exactly these 21 draws in order, and do NOT convert `new Random()` seed timing into battle determinism — capture tests are packet-drift checks, not RNG replay.

---

## Summary of acceptance-relevant hard facts

- Send = hex→bytes → XOR bytes with `0xAD` → single `write` (no checksum/trailer) — `Class5.cs:132-166`, `Server.cs:524-554`.
- 14013: ladder `LvSKill 1-3→2/4-6→3/7-9→4/10→5/else→3`.
- `getHpMax/getSpMax` are closed-form power curves; the exp curve is a data-loop over `Texps[]`.
- Both `QuestWin` and the H6 block use an abstract fresh `new Random()` — independent of the three battle streams.