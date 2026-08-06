# 04 — Battle Engine Reference (TS Dream)

Primary source: `ts_server_old/Server_TS_Online/TheBattle.cs` (9577 lines). Supporting sources:
`Server.cs` (server registry + send helpers), `Client.cs` (opcode 0x0B `Update_HB` input handling, battle skill/item input, shutdown), `Data.cs` (npc/skill/pet data access, quest win/lose, BattleGate load), `DataStructure.cs` (WarInfo, Type_* column names, status enums), `Class5.cs` (packet hex helpers), `ClientBattle.cs` (empty stub), `Data/BattleGate.txt`, `Data/Quests/*.ini` (`[TEAMDEF]` sections).

This document is the contract for the spec's Battle chapter. A Rust executor must reimplement byte-faithfully **without reading C#**; all packet strings below are written exactly as the C# concatenates them (hex, uppercase).

---

## 0. Packet framing & hex helpers (Class5.cs)

All wire packets are **hex strings**; the server converts them to bytes and appends a trailing checksum byte before sending.

- `Class5.smethod_3(byte[])` → uppercase hex of the bytes. Used for the grid key: `smethod_3(new byte[2]{ row, col })` → `"0A0B"` (row first, column second, each 2 hex chars). Keys are **"row-column"** two-byte hex strings.
- `Class5.smethod_4(hexString)` → bytes.
- `Class5.smethod_11(int)` → **int16 little-endian** hex, 4 chars: `lowByte+highByte`. E.g. `0x0A` → `"0A00"`, `0x1C` → `"1C00"`, `7168` → `"001C"`. Used for every 2-byte numeric field (HP, SP, mapid, lengths, DiaHinh, etc.).
- `Class5.smethod_12(int)` → **int32 little-endian** hex, 8 chars: bytes reversed. Used for entity **IDs** (player ids, npc ids, item ids in drops, warping ids). E.g. id `3` → `"03000000"`.
- Outbound packets use the framing `F444 <len:int16LE> <opcode...>` where `len` counts the bytes **after** the 4-byte `F444+len` header. Incoming packets (Client.cs `UpdateMainGrid`, line 614): read `len` from bytes 4–7, payload length = `8 + len*2` hex chars.
- `smethod_5(byte[])` appends the checksum byte (see server `SendToClient`/`SendToAllClientMapid`/`SendToAllMapid`, Server.cs:543-658, which convert via `smethod_4`→`smethod_5` and `socket.Send`). For a byte-faithful port, replicate `smethod_5` exactly (it appends a 1-byte XOR/checksum — verify in Class5.cs lines ~153-…).

Send helpers:
- `Server.SendToClient(id, packet)` — to one player.
- `Server.SendToAllClientMapid(id, packet)` — to every connected client **on the same map as `id`**, excluding `id` itself (Server.cs:596-626).
- `Server.SendToAllMapid(mapId, packet)` — to every client on mapId.
- `Server.SendToAllClient(id, packet)` — to all clients except `id`.
- `Client.Sendpacket(packet)` — to the client itself.

Global state (Server.cs:11-54):
- `Server.Clients` : `Dictionary<int, Client>` — all logged-in players by id.
- `Server.TheBattles` : `Dictionary<int, TheBattle>` — live battles by battle id.
- `Server.IdBattleCount` — starts at 1, incremented on each battle creation (constructor assigns `_idBattle = Server.IdBattleCount` before increment).
- `Server.PerEXP = 1`, `Server.MaxPerEXP = 1000000`.
- `Server.percent_item1..6 = 25, 23, 20, 4, 3, 1` — cumulative band widths used by drop rolls (see §5.7).
- `Server.IDPrefix = "vn"` (used for member .accdb paths only).

---

## 1. Battle lifecycle

### 1.1 The battle object

Fields (TheBattle.cs:16-30):
- `ListWar` : `Dictionary<string, DataStructure.WarInfo>` — 20 cells, keyed by `"rowcol"` hex string.
- `ListQS` : `Dictionary<int, int>` — 50 spectator/join slots (keys 1..50) mapping slot → player id. Filled by `CreatNewBattle` with 0; used for "join battle in progress" and leader SP regen (see §3.8).
- `_keys` : `ArrayList` — the 20 cell keys in creation order (row 0..3, col 0..4).
- `_idBattle` : int — battle id (== `Server.IdBattleCount` at creation).
- `_Diahinh` : int — terrain id.
- `random_0`, `random_1`, `random_2` : three independent `Random` instances (`.NET Random`, default seed = time-based). `random_0` = drops / skill pick / percent rolls; `random_1` = per-turn `_Random` and damage jitter; `random_2` = npc respawn coordinates. A byte-faithful port must preserve **three independent RNG streams** in this exact role split.

### 1.2 CreatNewBattle() — line 32

- Adds `ListQS[1..50] = 0`.
- Creates 20 `WarInfo` cells: for `row` in 0..3, `col` in 0..4 → `ListWar[hex(row)+hex(col)] = new WarInfo{_Row=row,_Column=col}` and appends the same key to `_keys`.

### 1.3 Grid geometry

- Player party (team 1) occupies **rows 3 and 2**; enemy team (team 2) occupies **rows 0 and 1**.
- Leader always at column 2. Members at columns 1, 3, 0, 4 (in member order Mem1..Mem4).
- The row opposite the leader row (`row ^ 1`) holds that row's pet entities.
- Cells: player leader `(3,2)`; player members `(3,1),(3,3),(3,0),(3,4)`; player pets `(2,2),(2,1),(2,3),(2,0),(2,4)`. Enemy mirror at rows 0/1.
- `team` field: `1` if row == 3, `2` if row == 0 (computed in `AddToBattle`/`SendBattleMem1`).

### 1.4 ChangedWar(...) — line 73

Setter for one cell. Signature args in order:
`_key, Type, Id, IdNpcOnMap, IdChar, HpMax, SpMax, Hp, Sp, Lv, Thuoctinh, LeaderId, IdSkill, RowAttack, ColumnAttack, Team, Int, Atk, Def, Agi, Reborn, Type3_Id, Type3_Lv, Type3_Turn, Type4_Id, Type4_Lv, Type4_Turn, Type15_Id, Type15_Lv, Type15_Turn, Type19_Id, Type19_Lv, Type19_Turn, Attacked, Exp`

Sets all fields, then rebuilds `_Packet`:

```
_Packet = Type:X2
        + smethod_12(Id)            // 4 bytes
        + smethod_11(IdNpcOnMap)    // 2
        + smethod_12(IdChar)        // 4
        + row:X2 + col:X2           // 2  (from smethod_4(_key))
        + smethod_11(HpMax) + smethod_11(SpMax) + smethod_11(Hp) + smethod_11(Sp)  // 8
        + Lv:X2 + Thuoctinh:X2      // 2
```

Total `_Packet` = **23 bytes**. `_Packet` is the per-entity snapshot broadcast to clients (opcode 0B05…).

### 1.5 AddToBattle(IdLeader, IdMem1..IdMem4, _row, _Column) — line 116

- `team = (_row == 0) ? 2 : 1`.
- For each present member id: sets `Clients[id]._My_IdBattle = _idBattle`, `Clients[id]._my_DiaHinh = _Diahinh`, writes the cell at `(row, col)` with `Type=2`, `Id=client._My_Id`, stats `_My_HpMax/_My_SpMax/_My_Hp/_My_Sp/_My_Lv/_My_Thuoctinh`, `LeaderId=IdLeader`, `Team=team`, `Int=Int+Int2`, `Atk=Atk+Atk2`, `Def=Def+Def2`, `Agi=Agi+Agi2`, `Reborn=_My_Reborn`.
- **Leader (row=3 or 0, col=2):** additionally loads up to 4 pets via Access queries on `member/<prefix><id>.accdb` table `Pet`:
  - Stt `my_SttPetXuatChien`   → cell `(row^1, 1)`, Type=4
  - Stt `my_SttPetXuatChien+1` → cell `(row^1, 3)`, Type=4
  - Stt `my_SttPetXuatChien+2` → cell `(row^1, 0)`, Type=4
  - Stt `my_SttPetXuatChien+3` → cell `(row^1, 4)`, Type=4
  - Pet cell data: `Id=pet._Id`, `IdNpcOnMap=pet Stt`, `IdChar=client._My_Id`, stats `HpMax/SpMax/Hp/Sp/Lv/Thuoctinh/Reborn`, `Int/Atk/Def/Agi = base+2` fields, `LeaderId=IdLeader`, `Team=team`.
- Each member (col=1,3,0,4): loads **one** pet at `(row^1, col)` (if `SttPetXuatChien` in 1..4), Type=4, same data mapping.
- Note: member pets overwrite leader's pet cells at `(2,1),(2,3),(2,0),(2,4)` (leader processed first). Whatever wins in dict insertion order is what the grid shows; only the active pet (Stt field) of each player is granted exp at battle end (see §7.3).

### 1.6 AddNPCToBattle(ID, _IdNpcOnMap, _row, _Column, _Type) — line 424

Fills cell `(row,col)` with `Type=_Type`, `Id=ID` (npc id), `IdNpcOnMap=_IdNpcOnMap`, stats read from `Data.GetDataNpc(ID, Type_Npc._Hp/_Sp/_Lv/_Thuoctinh/_Reborn/_Int/_Atk/_Def/_Agi)`. HpMax=Hp=_Hp, SpMax=Sp=_Sp, `Team=2`, everything else 0 (no LeaderId, no pets).

### 1.7 Constructors

All three share the same pattern: spin-wait `while(true) { try { _Diahinh=DiaHinh; _idBattle=Server.IdBattleCount; if (!Server.TheBattles.ContainsKey(_idBattle)) { register; IdBattleCount++; CreatNewBattle(); setup teams; spawn thread; break; } } catch {} }`.

- `TheBattle(IdLeader1, IdLeader2, DiaHinh)` — line 444. **Player-vs-player battle.** `AddToBattle(IdLeader1, …, 3, 2)` (team 1) and `AddToBattle(IdLeader2, …, 0, 2)` (team 2). Thread → `BattlePkPlayer(DiaHinh)`.
- `TheBattle(IdLeader, IdNpc, IdNpcOnMap, DiaHinh)` — line 485. **NPC battle.** `AddToBattle(IdLeader, …, 3, 2)` + `AddNPCToBattle(IdNpc, IdNpcOnMap, 0, 2, 3)` (enemy boss at (0,2), **Type 3**). Thread → `BattleNpc(DiaHinh)`.
- `TheBattle(IdLeader, TeamDeffender _Teamdef, DiaHinh)` — line 526. **Quest/TeamDef battle.** `AddToBattle(IdLeader, …, 3, 2)`, then for `_id1.._id10 > 0`: `AddNPCToBattle(_Teamdef._idN, N, 0, col, 7)` with **Type 7**, positions: id1→(0,0), id2→(0,1), id3→(0,2), id4→(0,3), id5→(0,4), id6→(1,0), id7→(1,1), id8→(1,2), id9→(1,3), id10→(1,4); `IdNpcOnMap = 1..10`. Thread → `BattleNpc(DiaHinh)`.

**Entity `_Type` semantics:** `2` = player; `3` = hostile npc (field/quest boss); `4` = pet; `7` = TeamDef npc. Types 3 and 7 are "npc-like": they never get DB HP/SP writes, never get exp, are catchable (§5.6), and their entity packets use opcode `0B0503` vs `0B0505` for players/pets.

### 1.8 How battles are triggered (Client.cs)

**PK battle — `Update_HB` case 2 (opcode 0x0B sub 2 sub 2), line 1278-1304:** attacker reads target id from bytes 7-10 (`smethod_10`). If `_My_IdBattle==0 && attacker._My_Pk==1 && target._My_IdBattle==0`:
- target `_My_Pk == 0` → send self `F4440300210101` (denied).
- target `_My_Pk == 1` → `new TheBattle(_My_Id, num5, 112)` — **DiaHinh hardcoded 112**.

**NPC battle — `Update_HB` case 2 sub 3, line 1306-1325:** if `_My_IdBattle == 0`, read npc id (bytes 7-10) and `IdNpcOnMap` (bytes 11-12, `smethod_9`). Guard: skip if npc id in `[20000,22000)`, `[23000,25000)`, `[26000,27000)` (these are quest-flag/doll npc ranges). Else `new TheBattle(_My_Id, num4, idNpcOnMap, 112)` — DiaHinh 112.

**Quest/TeamDef battle — FTalk.cs:163-183:** when a talk has zero dialogs (`GetDataTalkCount==0`) and `GetDataTalkTeamDefs` returns an 11-element array whose sum > 0: `TeamDef = {dataTalkTeamDefs[1..10]}`, set `_My_TalkingBattle = idtalking`, `new TheBattle(_My_Id, teamdef, dataTalkTeamDefs[0])` — DiaHinh = `[0]`.

**Active-NPC (so-luong) battle — Data.cs:5016-5083 (NpcOnMap wander loop):** when a player (no party leadership restrictions except `IdLeader==0||IdLeader==Id`) is within `coord` of an npc with `_SoLuong>=3`: builds a TeamDeffender from `npcId` per `_SoLuong` (1→id3, 2→id3+id4, 3→id2/id3/id4, 4→id2/id3/id4/id8, 5→id1..id5), sets `_My_TalkingBattle = id`, `value._IdBattle = 1`, `new TheBattle(my_Id, teamdef, 4712)` — **DiaHinh hardcoded 4712**.

**BattleGate.txt (Data.cs:4750-4788):** tab-separated `MapId, WarpId, Diahinh, id1..id10`, loaded into `Data_BattleGates` (rows starting with `//` skipped). `GetDataBattleGate` returns any column. This file is the source for warp-triggered team battles (a Warp entry with `_IdBattle`-style handling exists in `Data_Talks` via `[TEAMDEF]`); the actual battle construction for warps reuses the TeamDefender constructor path (FTalk.cs:2748, 3191).

### 1.9 Battle start broadcast — BattlePkPlayer (line 606) / BattleNpc (line 676)

**BattlePkPlayer:** sends the whole grid to each player:
1. `SendBattleLeader(DiaHinh, 3, 2)` — team-1 leader's full grid view.
2. If cell `(3,1)` id>0 → `SendBattleMemPkPlayer(DiaHinh, 3, 1)`; same for `(3,3)`, `(3,0)`, `(3,4)` (member views).
3. `SendBattleLeaderPlayerPK(DiaHinh, 0, 2)` — team-2 leader's grid view.
4. If `(0,1)` id>0 → `SendBattleMemPkPlayer(DiaHinh, 0, 1)`; same for `(0,3)`, `(0,0)`, `(0,4)`.
5. `Battling()`.

**BattleNpc:** reads leader at `(3,2)`; if `_Id <= 0` returns. Sends to the leader:
- `"F4441C000BFA" + smethod_11(DiaHinh) + "03" + leader._Packet + "F44403000B0A01"` (setup).
- `"F4440A000B0402" + smethod_12(leaderId) + "000003"` (leader appears on map).
- `SendEntityPacketIfExists(leader, 2, 2, "F4441A000B0505")` — player's own pet at (2,2).
- For cols [1,3,0,4]: if `(3,col).id>0`: send `"F4441A000B0505"+member._Packet+"F4440A000B0402"+smethod_12(id)+"000005"`, then pet entity, then `SendBattleMem1(DiaHinh, 3, col)`.
- `SendEnemyEntities(leader)` — rows 0..1 cols 0..4, each `"F4441A000B0503" + _Packet`.
- `Battling()`.

---

## 2. WarInfo entity model (DataStructure.cs:661-746)

Fields (all `int` except `_Packet: string`):
`_Type, _Id, _IdNpcOnMap, _IdChar, _Row, _Column, _HpMax, _SpMax, _Hp, _Sp, _Lv, _Thuoctinh, _LeaderId, _IdSkill, _RowAttack, _ColumnAttack, _Int, _Atk, _Def, _Agi, _Team, _Random, _LvSKill, _Reborn, _Type3_Id, _Type3_Lv, _Type3_Turn, _Type4_Id, _Type4_Lv, _Type4_Turn, _Type5_Id, _Type5_Lv, _Type5_Turn, _Type15_Id, _Type15_Lv, _Type15_Turn, _Type19_Id, _Type19_Lv, _Type19_Turn, _Attacked, _Exp, _Packet`

Meaning:
- `_Type` 2/3/4/7 as §1.7.
- `_Id` = player id (type 2/4) or npc id (type 3/7). `_IdChar` = owner player id for pets (0 for players/npcs). `_IdNpcOnMap` = npc-on-map instance id for npcs, pet Stt for pets, 0 for players.
- `_IdSkill/_RowAttack/_ColumnAttack/_LvSKill` — current turn command (targeting + chosen skill + its level). Reset each turn for all living entities.
- `_Attacked` — 0 = waiting for input, 1 = command submitted, 2 = acted (set at end of the entity's turn).
- `_Type3/_Type4/_Type15/_Type19` triplets — active buffs/debuffs (id, lv, remaining turns). Type5 exists in the struct but is never used by the engine.
- `_Exp` — accumulated exp from kills (only meaningful on player entities).
- `_Random` — per-turn random `random_1.Next(0,100)` used as a tie-breaker in turn order.

---

## 3. Turn engine — Battling() (line 1002-4950)

Runs on a dedicated background thread. Local state: `num2,num5` = average levels of team1/team2 (`num3/num4`, `num6/num7`), `text` = TroiStart packets, `text2` = combo packet, `text9` = accumulated turn packet buffer, `text10` = current entity action buffer, `text11` = reflect-damage buffer, `arrayList` = combo participants (encoded `"row.col/lv"`), `arrayList2` = killed npcs (same encoding), `num` = outcome (0 = running, 1 = player win, 2 = player lose), `num18` = combo state, `num19/num20` = combo agi bounds, `num21` = current skill delay, `num22` = loop index, `num23` = reflect damage accumulator, `num8=0.03`, `num9` = 13020 speed bonus, `num10=200` = speed-buff agi delta.

### 3.1 Win/lose checks (top of loop, line 1025-1036)

- `flag2` = ALL of rows 0-1 (`(0,2),(0,1),(0,3),(0,0),(0,4),(1,2),(1,1),(1,3),(1,0),(1,4)`) have `_Hp <= 0` → enemy team wiped → `break` (→ player **win**).
- `flag4` = ALL of rows 2-3 dead → `num=2`, proceed to loser flow.

### 3.2 Turn phase 1 — reset & pre-buff ticks (line 1038-1239)

For every key in `_keys` (20 cells, grid order), if `_Id > 0`:
- Reset `_IdSkill=_RowAttack=_ColumnAttack=_Attacked=0`; `_Random = random_1.Next(0,100)`.
- Accumulate avg level: team1 (`num3 += _Lv; num4++`), team2 (`num6 += _Lv; num7++`) — **only while avg is 0.0** (i.e. first living pass; `if (num2 == 0.0)`).
- Decrement buff timers: `if (_Type3_Id > 1) _Type3_Turn--`; same for Type4, Type15, Type19.
- **Burn (Type3 10004 / 10033)**: damage `= 10 + Type3_Lv*2`, or `30 + Type3_Lv*10` for 10033. Non-npc entities (type != 3/7) get a DB HP write (player via `Data.PlayerUpdateDataId(id, Type_Player._Hp, hp)`; pet via `Data.PetUpdateData(idChar, IdNpcOnMap, Type_Pet._Hp, hp)`). Broadcast: `"F444" + len + "3201" + row:X2 + col:X2 + smethod_11(20001) + "0101" + SkillingInt(row,col,1,0,1,_Hp,dmg,1)`.
- **Poison (Type15 14015)**: damage `= 30 + Type15_Lv*15`, same DB-write rules, broadcast with skill `20003`.
- **Buff end**: when `_Type3_Turn == 1` → clear Type3, `TroiEnd(_idBattle,row,col,Type_TroiBuffEnd._Type3)`. Type4→`_Type4`(2), Type15→`_Type15`(3), Type19→`_Type19`(5). On Type15 end: if `_Type15_Id ∈ {10016,10017,10018,10019,10025,20022}` → `_Agi += 200`. On Type19 end: if `_Type19_Id==13020` → `_Agi -= num9; num9=0`.
- **Turn prompt**: `if (_Type==2) Server.SendToClient(id, "F44402003401")`. If `_Type3_Turn > 0` → `SendSKillingToParty("F44404003505" + row:X2 + col:X2)`.

### 3.3 Turn phase 2 — wait for input (line 1259-1441)

- Computes `num2 = num3/num4`, `num5 = num6/num7` (team avg levels).
- **Auto-action selection** is executed here, in grid order, for entities with `_Id>0 && _Hp>0`:
  - If `_Type15_Id==14021 || 20014` (berserk): picks a random enemy row (rows 0-1 if entity in rows 0-1, else rows 2-3 via `RandomizeArrayWithPercent(row,row^?,50)`), builds candidate column list from cells with `_Hp>0` (cols 2,1,3,0,4 in that order), `RandomizeArray` pick; sets `_IdSkill=10000,_LvSKill=1,_RowAttack,_ColumnAttack,_Attacked=1`.
  - If `_Type3_Turn==0 && _IdSkill==0` and entity is **npc** (type 3/7): picks row via `RandomizeArrayWithPercent(2,3,50)` and a random column from alive cells (cols 2,1,3,0,4), sets `_IdSkill = GetRandomSkillNPC(npc Lv, npc Reborn, npc Skill1..3)`, `_LvSKill = Data.GetDataSkill(_IdSkill, _LvMax)`, `_Attacked=1`.
  - If `_IdSkill>0` (player input already given) or `_Type3_Turn>0` (silenced) → `_Attacked=1`.
  - Entities with `_Id<=0 || _Hp<=0` → `_Attacked=1`.
  - `num14 *= _Attacked` (product over all 20 cells; note dead/empty cells contribute 1).
- Loop: `do { Thread.Sleep(100); num13++; …recompute… } while (!(num13 >= 210 || num14 > 0))` — i.e. polls every 100 ms up to **21 s** until every entity has `_Attacked==1`, then proceeds.
- **Force-set** all entities `_Attacked=1` (line 1443-1455) before ordering.

During this window players submit commands via opcode 0x35 (see §6.3).

### 3.4 Turn phase 3 — turn order (line 1456-1475)

Builds an in-memory `DataTable` with columns Row, Column, Agi, Random, Attacked from all 20 cells, then sorts:
**`"Attacked DESC, Agi DESC, Random DESC"`** and iterates rows 0..19. (So: non-input actors first, then highest Agi, then the per-turn random as the final tie-break.)

### 3.5 Turn phase 4 — action execution (line 1485-3934)

For the sorted row at index `num22` (guard `num22 < 19` used for combo lookahead), with `warInfo` = cell `(Row,Column)`:

Per-entity pre-reads: `_Type, _Id, _IdNpcOnMap, _IdChar, _HpMax, _Hp, _Sp, _Lv, _Thuoctinh, _Reborn, _LeaderId, _IdSkill, _RowAttack, _ColumnAttack, _Team, _Int, _Atk, _Agi, _LvSKill, _Attacked`, buff fields, `dataSkill = GetDataSkill(_IdSkill,_Sp)`, `dataSkill2 = GetDataSkill(_Type19_Id,_SLdanh)`. Also `dictionary` of per-level debuff rates: `{13012: 0.033, 14053: 0.1/dataSkill2, 14040: 0.1/dataSkill2, 12025: 0.05/dataSkill2}`.

SP cost gate (`_Type3_Id==0 && _Attacked==1`):
- If `_Sp >= dataSkill` (skill SP cost): for non-npc entities write new SP to DB (player or pet); `warInfo._Sp -= dataSkill`.
- Else → skill becomes **10000**, `_LvSKill=1` (basic attack fallback).

Skill-validity gate (line 1560): if `_Hp>0 && _IdSkill>0 && _RowAttack<4 && _ColumnAttack<5 && (GetDataSkillExits(_IdSkill) || _Type==3 || _Type==7)`:
- Special skill set `{10000,15001,15002,15003,17001,18001,18002,19001}` are always allowed (`arrayList6`).
- Type 4 (pet): skill allowed if ∈ npc `Skill1..4`.
- Type 2 (player): allowed if `Data.PlayerGetDataSkillId(client.conn, _IdSkill, _Lv) > 0`; then `_LvSKill =` learned level.
- Type 3/7 (npc): always allowed.

Skill-type dispatch (`dataSkill3 = GetDataSkill(_IdSkill,_Type)`), with `num34 = GetDataSkill(_IdSkill,_SLdanh)`, `dataSkill4 = GetDataSkill(_IdSkill,_Combo)`, `dataSkill5 = GetDataSkill(_IdSkill,_Delay)`, `num37=2.0`:

- **Skill 13008** → coerced to 13012, `num34=2`, `num30=3`.
- Default target list: `GetPosAttack(_idBattle, _Team, _RowAttack, _ColumnAttack, num34)`.
- If `num18==1` (combo): `num37 *= 1.3`, list = `GetPosAttackCombo(...)`.
- **Type 8** (single-target "focus"): clear; target = the cell `(_RowAttack,_ColumnAttack)` itself only if its `_Id>0 && same team` (buffs cast on self/ally).
- **Type 17**: clear; target only if `currentRow==_RowAttack && currentCol==_ColumnAttack`.
- **Type 3 or 15** (AoE): list = `GetPosAttack3_15(...)`.
- **Type 4/6/7/14/19** (heal/buff): list = `GetPosAttack_Type4(...)` (own team). If skill ∈ {11010,11009,11026,11030} scale `num34` by `_LvSKill`: 1→1, 2-3→3, 4-6→5, 7-9→6, 10→8.
- **Type15_Id ∈ {14021,20014}** (berserk): skill=10000, num34=1, list = `GetPosAttack_honLoan(...)` (same-team spread).
- **Skills {10016,11016,12016,13042,13015} or Type 18** (multi-level AoE): scale num34 by lv (1-3→3, 4-6→5, 7-9→6, 10→8) and **upgrade the skill id**: 10016→10017/10018/10019, 11016→11017/18/19, 12016→12017/18/19, 13015→13016/17/18, 13042→13029 (always). List = `GetPosAttackTG(...)`.
- **Type 5/16/17/18** (dispel/cleanse): list = `GetPosAttack_GiaiTru(...)`.

**Combo detection (only `num22 < 19`, Type 1 skills, line 1768-1908):** looks at the next entity in sorted order (`dataView[num22+1]`). If it is **same team** and its skill `Type==1` and its `(_RowAttack,_ColumnAttack) == current target cell` and that cell `_Id>0 && _Hp>0`:
- First combo (num20==0): if `|agi2 - nextAgi| <= 800` → `num20=max`, `num19=min`, `num18 = GetRandomMissCombo(avgTeam…)` (team1 uses `GetRandomMissCombo(round(num2), round(num5))`), `num21 = dataSkill5`, and if `num18==1 && text10 empty` → `num37 *= 1.3`, add both cells to `arrayList`.
- Subsequent (num20>0): window shrinks to `(num19+num20)/2 ± 800`, expands bounds, re-rolls combo, `num21 = max(num21, dataSkill5)`.
- Else `num18=0`.

**Damage/effect application** — `switch (dataSkill3)`:

- **Type 1** (line 1953-2386) — physical attack (detailed in §5.1).
- **Type 2** (line 2387-2668) — magic attack (uses `_Int` instead of `_Atk`, §5.2).
- **Type 3** (line 2669-2812) — Type3 debuff/buff (silence-style; §5.3).
- **Type 4** (line 2813-2844) — Type4 buff/debuff (miss roll `GetRandomMissTroi`).
- **Type 5** (line 2845-2922) — dispel Type4/cure: skill-specific counters (11014 clears 10010, 14007 clears 14008, 14014 clears 14015, 14022 clears 10021); default: clears Type3/4/15/19 all and sends the "all clear" effect `"DD000001DE000001DF000001E1000001"`.
- **Type 6** (line 2923-2977) — SP restore. `num72 = round(_Int*0.25)`; skill 11009 → `round(_Int*0.05*_LvSKill)`; 11006 → `round(_Int*0.1*_LvSKill)`. 0 if target==caster or self-cell. Clamped to SpMax. Effect byte `_Sp` (26).
- **Type 7** (line 2978-3024) — HP restore. `num48 = round(_Int*0.5)`; 11010 → `_Int*0.1*lv`; 11007 → `_Int*0.2*lv`. Clamped to HpMax. Effect `_Hp` (25).
- **Type 8** (line 3025-3051) — revive. If target `_Hp<=0`: `_Hp = round(_HpMax / (10.0/_LvSKill))`, DB write, Attack status; else Miss/Def.
- **Type 11** (line 3052-3100) — catch pet (see §5.6).
- **Type 12** (line 3101-3222) — flee (§5.5).
- **Type 14** (line 3223-3309) — heal HP+SP: default `round(_Int*0.5)` each; 11004 → `round(_Int/1.7)+3*lv` HP, `round(_Int/3.7)+lv` SP; 11026 → `round(_Int/2.7)+3*lv`, `round(_Int/7.0)+2*lv`; 11030 → `round(_Int/1.7)+3*lv`, `round(_Int/4.7)+3*lv`. Sends a 2-effect SkillingInt.
- **Type 15** (line 3310-3391) — Type15 buff/debuff: miss roll; on success, if skill ∈ {10016,10017,10018,10019,10025,20022} → `target._Agi -= 200`; sets `_Type15_Id/Lv/Turn` (Turn from `GetTurn`). Special 10026 case affects caster via `TroiStart`.
- **Type 16** (line 3392-3424) — dispel Type4 by pair (10014 clears 10015; 10009 clears 10010).
- **Type 18** (line 3425-3540) — cleanse+heal. If target is **same team**: clear Type3+Type15, heal HP 400/500/600/700 and SP 100/150/200/250 for skills 11016/11017/11018/11019 (fixed values), send `"DF000001"+Hp effect+SP effect`. If **enemy**: clear Type4+Type19, send `"E1000001"`.
- **Type 19** (line 3541-3577) — Type19 debuff. Skill 13020: `num9 = ceil(target._Agi * 0.03 * caster._LvSKill)`; `target._Agi += num9`; Type19 stored.

After the switch, `warInfo`/`ListWar[cell]` is updated with any changes; if `_Hp<=0 && _IdSkill==20006` → `_IdSkill=0` (shield drops).

**Turn packet assembly (line 3593-3623):** if `text10` non-empty:
- If `num47>0` (reflect/self-damage pending): `text10 = row:X2 + col:X2 + skillId(smethod_11) + num34:X2 + (count+1):X2 + text10 + SkillingInt(row,col,1,0,1,_Hp,num47,0)`.
- Else `text10 = row:X2 + col:X2 + skillId + num34:X2 + count:X2 + text10`.
- `text9 += smethod_11(len(text10)/2) + text10`.
- If `text9.Length>0 && num18==0`: `SendSKillingToParty(text2 + "F444" + smethod_11(len(("3201"+text9))/2) + "3201" + text9)`; `text9=""`; `Thread.Sleep(num21)` (delay from skill, ms); flush `text11` (reflect), then drop/exp processing (§7.2).

### 3.6 Drop & exp accumulation per turn (line 3621-3905)

After each flushed turn, for each entry in `arrayList2` (killed **npc** entities, encoded `"row.col/lv"`):
- If dead cell's `_Hp<=0`: `num99 = GetRandomMissDrop(cell._Id)` (item roll).
- If `arrayList.Count==0` (no combo): item `num99>0` granted to `idChar2` (pet owner) else `id3` (caster), via `Data.HomdoAddItem`, broadcast `"F44408003504" + smethod_11(num99) + npcRow:X2 + npcCol:X2 + casterRow:X2 + casterCol:X2`.
- Exp: `lv5 = caster._Lv`; if `|lv5 - npcLv| <= 20` compute `num100` from level-diff table (diff 0-2→`round(5+lv/5)`, 3-5→`round(4+lv/5)`, 6-10→`round(3+lv/5)`, 11-15→`round(2+lv/5)`, 16-20→`round(1+lv/5)`; if diff<0 → `round((npcLv-lv5) + npcLv/5)`). Add full `num100` to `caster._Exp` if npc dead, else `round(num100/10)`.

For each combo participant in `arrayList` (`"row.col/lv"`): same level-diff table, but `num105 = round(base * 1.086)`; granted to `idChar7` else `id8`; drop packet uses participant's own row/col in the last two bytes.

### 3.7 End-of-battle (line 3953-4449)

- If enemy team empty → `num=1` (win). If player team empty → `num=2` (lose). The `IL_e459/IL_ea89` blocks re-count survivors from both teams (cells with `_Hp>0`) before looping; the loop terminates when both sides have zero alive entities.
- **Leader SP regen per turn (IL_caac, line 4147-4247):** for each player leader (`_Id>0 && _Id==_LeaderId`), if their `_My_IdQS` is set: `num109 = round((leader._My_Int+_My_Int2)/15)`; add to leader SP, leader's pet (`row^1,col` cell), and every party member + their pets (cells in the same row), each clamped to SpMax, with DB writes.
- Outcome `num` survives to reward processing.

### 3.8 Rewards & cleanup (line 4458-4949)

For columns 0..4 of **row 3** (player team leaders/members), for each `_Type==2` client:
- **Player exp**: `num112 = round(PerEXP * cell._Exp * (God<=0?1:2) * (Ghost<=1?1:(Ghost>2?0:0.5)))`. If `_My_Lv<200 && hp>0 && num112>0 && !flag(fled)`: if `_My_Reborn==2` halve; `PlayerUpdateDataId(_TExp, _TExp+num112)`; `num113 = TexpGetLvUp(lv, reborn, newTexp)`; if `>0` → set Hp/Hpmax/Sp/Spmax via `getHpMax/getSpMax`, `_Lv += num113`, `_Point += 2*num113`, `_SkillPoint += num113`.
- **Pet exp** (for the 4 pet cells in row 2 at cols 0,1,3,4, mapped to Stt `SttPetXuatChien..+3`): `num117 = PerEXP * petCell._Exp`; if pet lv<200 && hp>0 && num117>0: reborn remap (1→0, 2→1), halve if reborn==1; `PetUpdateData(_Texp, _Texp+num117)`; `TexpGetLvUp` then `PetUp` per level gained.
- **Fled/dead cleanup**: if player `_My_Hp<=0` → `PlayerUpdateDataId(_Hp, 1)`. For each pet slot 1..4 with id>0 and hp<=0: decrement `Fai`; if `Fai<20` → `Data.Removepet(id, slot)` else set `Fai` and `Hp=1`.
- `Client._My_IdBattle = 0`.
- Broadcast to map: `"F44408000B00" + smethod_12(id) + "0000"` (hide), plus party buffer `"F44408000B00"+id+"0000F44405000B01"+row:X2+col:X2+"00"`, `SendSKillingToParty("F44402000504")`, `SendSKillingToParty("F44402001408")` (battle exit UI), `_My_WarpingId=0`.

Same for row 0 (enemy players in PK) — exp is NOT granted (only hp/fai reset + idbattle clear), then all `ListQS` spectators get `_My_IdBattle=0` + `"F44408000B00"+id+"0000"`.

**Quest win/lose integration (line 4682-4848, only for `_LeaderId==0 || _LeaderId==self`):**
- If `_My_TalkingBattle>0` and the talk exists for current step (`Data.GetDataTalkExits(mapId,"NPC",talkingId,step)`): win if `hp>0 && num==1` → `_My_AfterBattleType=1`, send `_WinDialogs[0]` if present else `Data.BattleQuestWin(client, key)` + clear talk state; lose → `_My_AfterBattleType=-1`, send `_LoseDialogs[0]` if present else clear talk state. Then `client.EndTalk()`.
- If `_My_WarpingId>0`: similar for `"WARP"` talks; on win with no talk entry → `Data.Warped` the leader and all members to `GetDataWarp(MapId, WarpId, _MapId2/_X/_Y)`.

---

## 4. Targeting

Shared `Point`-based selection. Default "sentinel" = `(99,99)` meaning "no target found"; `GetPosAttack*` only proceeds when `0 <= X < 4`.

Common pattern — `GetPosRandomX(…, rowAttack, columnAttack)` picks an **anchor point**:
- If the requested cell qualifies → anchor = requested cell.
- Else iterate `ListWar.Values` (dict order) and take the **first** qualifying cell (break).

Then `GetPosAttackX(…, SLDanh)` expands the anchor into the target list, `SLDanh` = skill's `SLdanh`:

| SLDanh | targets added |
|---|---|
| 1 | anchor |
| 2 | anchor, opposite-row cell `(anchor.X ^ 1, anchor.Y)` if alive |
| 3 | anchor, `(X,Y-1)` if alive, `(X,Y+1)` if alive |
| 4 | anchor; `(X,Y-1)`: alive→add, else `(X,Y)`; `(X,Y+1)`: alive→add then **break**, else `(X,Y)` |
| 5 | anchor, `(X,Y±1)` alive, opposite-row cell if alive |
| 6 | anchor, `(X,Y±1)` alive, opposite-row if alive, `(opposite.X, Y±1)` alive |
| 7 | anchor column: all rows at `Y` with alive cells |
| 8 | anchor column Y (both rows? no — all cells at `(X, 0..4)` alive) **plus** all cells at opposite row `(X^1, 0..4)` alive |

Target qualification per variant (all require `_Id>0 && _Hp>0`, plus team/status conditions):

- **`GetPosRandom` / `GetPosAttack`** (line 7551/7599) — default hostile: `team != myteam` **and** `_Type4_Id ∉ {13005,13025,13032}`.
- **`GetPosRandomCombo` / `GetPosAttackCombo`** (line 7833/7880) — same as default but **no HP requirement on the requested anchor** (anchor qualifies on `_Id>0 && team!=myteam` only; the fallback scan still requires `_Hp>0`).
- **`GetPosRandomTG` / `GetPosAttackTG`** (line 8114/8160) — `team != myteam`, no Type4 exclusion.
- **`GetPosRandom3_15` / `GetPosAttack3_15`** (line 8394/8442) — `team != myteam` + Type4 exclusion `{13005,13025,13032}`.
- **`GetPosRandom_GiaiTru` / `GetPosAttack_GiaiTru`** (line 8676/8717) — **any** entity with `_Id>0` (no team/HP requirement on anchor; fallback scan requires only `_Id>0`).
- **`GetPosRandom_Type4` / `GetPosAttack_Type4`** (line 8951/8997) — `team == myteam` (own-team buffs/heals).
- **`GetPosRandom_honLoan` / `GetPosAttack_honLoan`** (line 7271/7317) — `team == myteam` (berserk same-team splash).

The `GetPosAttack*` expansions are byte-identical across all variants except the anchor picker; only `GetPosAttack_Type4`/`_honLoan` target own team. **No terrain (Diahinh) influence exists anywhere in targeting or damage** — `_Diahinh` is only echoed into the `0BFA` setup packet and stored on clients.

---

## 5. Damage formulas

### 5.1 Element tables

`GetDamageThuoctinh(MyTT, TTAttack)` (line 7191) — returns double. Rows = MyTT (attacker element), cols = TTAttack (defender element). Elements: 1,2,3,4 (defaults 1.0):

| MyTT\TT | 1 | 2 | 3 | 4 |
|---|---|---|---|---|
| 1 | 1.0 | 1.55 | 1.3 | 0.65 |
| 2 | 0.6 | 1.0 | 1.7 | 1.0 |
| 3 | 1.6 | 0.7 | 1.0 | 1.9 |
| 4 | 1.7 | 1.3 | 0.8 | 1.3 |

`GetDamageSkillInt(TTSkill, TTAttack)` (line 7231) — **int** additive per-level component:

| TTSkill\TT | 1 | 2 | 3 | 4 |
|---|---|---|---|---|
| 1 | 18 | 27 | 21 | 13 |
| 2 | 10 | 19 | 29 | 19 |
| 3 | 26 | 15 | 27 | 34 |
| 4 | 54 | 42 | 29 | 42 |
| other | 10 | | | |

`GetThuoctinhKhac(t1, t2)` (line 9505) — returns **2** if t1 beats t2, **1** if t2 beats t1, else 0. Beat graph: 1 beats 4, loses to 2; 2 beats 1, loses to 3; 3 beats 2, loses to 4; 4 beats 3, loses to 1.

### 5.2 Physical attack (skill Type 1)

Base (skill 10000 or Combo 84):
`num36 = round(Atk * GetDamageThuoctinh(attackerTT, targetTT) * 2.0 - Def * 2.0)`
Combo 87 (uses Int): `round(Int * GetDamageThuoctinh * 1.6 - Def * 1.6)`
Then generic refinement:
`num36 = round(Atk * GetDamageThuoctinh * 2.0 - Def * 1.6)`
`num36 += round((attackerLv - targetLv)/1.5) + round(attackerLv/20.0)*8`
`num36 = round(num36 + GetDamageSkillInt(skillTT, targetTT) * DoManh * (1.0 + skillLv*0.033))`
`num36 = round(num36 * num37)` where num37 = 2.0 (or 2.6 during combo, or 1.3*2.0 when combo triggers; see §3.5 — careful: `num37` starts 2.0 and is multiplied by 1.3 on combo; the `* num37` step uses the current value).
Buff modifiers (applied in this exact order):
- target `_Type3_Id==11014` → `+round(num36*0.01*Type3_Lv)` (attack buff)
- target `_Type4_Id==11002` → `-round(num36*0.01*Type4_Lv)`
- target `_Type4_Id==12025` → `+round(num36*0.02*Type4_Lv)`
- attacker `_Type4_Id==13012` → `+round(num36*Type4_Lv*0.033)`
- target `_Type4_Id==13012` → `-round(num36*Type4_Lv*0.033)`
- target `_Type15_Id==13011` → `+round(num36*Type15_Lv*0.033)`
- attacker `_Type15_Id==13011` → `-round(num36*Type15_Lv*0.033)`
- attacker `_Type19_Id ∈ {14053,14040,12025}` → `+round(num36*Type19_Lv*dictionary[id])` where dictionary = `{13012:0.033, 14053:0.1/SLdanh, 14040:0.1/SLdanh, 12025:0.05/SLdanh}`
- if `num34>1` → `num36 = round(num36/(num34*0.75))` (AoE falloff)

Hit roll `num35 = GetRandomMissAttack(attackerLv, targetLv, round(avgTeamLv), round(targetTeamAvgLv))`:
- `GetRandomMissAttack` (line 9427): `percent = 100 + round((lv1-lv2)/10) + round((lvtb1-lvtb2)/10)`; roll `RandomizeArrayWithPercent(1, 0, percent)` → returns **1 = Attack lands (Type_StatusAttackMiss._Attack=1)** or **0 = Miss (._Miss=0)**.
- `RandomizeArrayWithPercent(v1, v2, p)` (line 9492): `p = min(p,100)`; `random_0.Next(1,1000) <= p*10` → v1 else v2. (p>100 is clamped to 100 here — so a "hit percent" above 100 still only hits 100%.)

On **Miss (num35==0)**: `num36=0`; `attack_Def_Lantranh = Attack(0)`; if target's current `_IdSkill==17001` → `Def(1)`; if target `_Type4_Id==13003` → `Lantranh(2)` (counter stance shows).

On **Hit (num35==1)**:
- `attack_Def_Lantranh = Attack(0)`; if target `_IdSkill==17001` → `Def(1)` **and** damage becomes element-reduced: `num36 = (elementRelation==2) ? 1 : (relation!=1 ? num36/5 : num36/3)` (element relation via `GetThuoctinhKhac` of skillTT vs targetTT; for skill 10000 use attacker's own TT).
- `num36 = (num36<1) ? 1 : num36 + random_1.Next(0,2)`.
- Target `_Type4_Id ∈ {10010,10015,10031,13021}` → `num36 = 0`, `Attack_Def_Lantranh = Def(1)`, and for the **self**-target case the absorbed value goes to `num23` (reflect/self-damage applied later, see §5.4).
- Target `_Type4_Id==13003` → `num36=0`, `Attack_Def_Lantranh=Lantranh(2)`, force `num35=Miss`.
- Attacker `_Type15_Id ∈ {10016..10019}`: fresh `Random.Next(1,3)`; 1 → `num36=0, Lantranh`; else `num36 = max(1, num36/10)`.
- HP subtraction: non-npc entities (type!=3/7) get DB writes (`PlayerUpdateDataId`/`PetUpdateData`) with `max(0, hp-dmg)`; npc types just subtract locally.

**Shield (20006)** — when the target cell is at row 3 or 0 (leader rows), the entity **behind** it at `(row^1, col)` is checked: if that rear cell has `_IdSkill==20006 && _Hp>0`, the entire damage is applied to the **rear cell** instead (with its own def, element, buffs, miss roll), and the skill packet targets `(row^1, col)`. The shield skill (20006) auto-drops when its holder dies (`hp2<=0 && _IdSkill==20006 → _IdSkill=0`).

**Status-debuff skills (13007/13029)**: on a successful hit vs a target without Type3: set `target._Type3_Id=skill, _Type3_Lv, _Type3_Turn=3`, and the effect bytes use `Skilling + "02" + SkillingHieuUng(_Hp, dmg, 1) + SkillingHieuUng(_Type3, 0, 1)`.

### 5.3 Magic attack (skill Type 2)

Base: `num36 = round(Int * GetDamageThuoctinh(attackerTT, targetTT) * 2.0 - Def * 1.6)` (then same `+lv terms + GetDamageSkillInt...` pipeline as §5.2, **no initial `num36 *= num37` combo multiplier** — num37 is not applied in Type 2).
- Skills 12016-12019 (multi-hit magic): `if num34>1 → num36 = round(num36/(num34*0.5)) + skillLv*50`.
- Same hit/miss/shield/reflect logic as Type 1.
- Exp granted for kills uses `lv2 - lv4 <= 20` (with full/1/10 rule, see §7.2).

### 5.4 Status skill types (3, 4, 15, 19)

- `GetTurn(skillId, skillLv)` (line 9231) → turn count. Group A `{13002,14008,13003,13005,13012}`: `lv-1>1 → 3; lv∈{2,3}→2` else 3. Group B `{10033,10015,10026,13021,13025,13032,10025,14020,12025,14040,14044,14046,14053}`: lv 1-2→2, 3-4→3, 5→4, else 3. Group C `{10004,11002,12024,13011,13030,14015,14029,20018,11024,11032,13020}`: 3. Group D `{13015..13018,10016..10019}`: 4. Group E `{11014,20014,20022,20023}`: 5. Group F `{20025,20026,20027,10010,10031,13014,20024,14012}`: lv 1-3→2, 4-6→3, 7-9→4, 10→5. `14021`: lv 1..5 → 2..6. Everything else: 3.
- `GetRandomMissTroi(lv1,lv2,avg1,avg2,int1,atk1,spx2,reborn1,reborn2)` (line 9456): `percent = 30 + max(int1,atk1)/30 - spx2/30 + round((lv1-lv2)/20) + round((avg1-avg2)/20) + reborn1*5 - reborn2*5`; roll `RandomizeArrayWithPercent(1,0,percent)`; **1 = effect lands** (Type_StatusAttackMiss._Attack), 0 = Miss.
- Type3 skill: if target already has Type3 → Miss. On land: for 13015-13018 (SP drain): `target._Sp -= skillLv*30` (clamped ≥0, DB-written), caster heals HP by drained amount (accumulated into `num47`, applied to caster cell and emitted at packet build). 10026 → caster gets the Type3 (guard). Else `target._Type3_Id/Lv/Turn` set.
- Type4/Type19: land → set the buff on the **target**; miss → shows Miss.
- Skill 13020 (Type19): `num9 = ceil(target._Agi * 0.03 * caster._LvSKill)`; `target._Agi += num9`; stored; reverted when Type19 ends.

### 5.5 Flee (skill Type 12)

`num35 = GetRandomMissChayTron(lv1, lv2, avg1, avg2)` (line 9439): `percent = 60 + (lv1-lv2) + (avg1-avg2)`; roll. If **landed (1)** or skill==14002:
- If the fleeing entity is the **leader** (`_Id==_LeaderId || _IdChar==_LeaderId`): battle **ends with `num=3`** (go to `end_IL_005c`, treated as "fled"; no rewards — `flag=true` prevents exp at §3.8).
- Else: flee just that party member: remove leader+pet cells, send `"F44404003505"+row:X2+col:X2` (both rows), `"F44404000B01"+(row^1):X2+col:X2` (pet cell hide), `"F44408000B00"+id+"0000"` + `"F44405000B01"+row+col+"00"` map packets, map broadcast of player hide, `_My_WarpingId=0`, `F44402000504`+`F44402001408`, and respawn the triggering npc (via `_My_TalkingBattle`, random coords, `"F44406001603"+id+"0A00"` + `"F44408001605"+id+x+y`, `_Delay=10`). The member's `_My_IdBattle=0`, HP restored to 1 if ≤0, pets set to HP 1.
- On failure: shows miss packet, `Thread.Sleep(num21)` (delay).

### 5.6 Catch pet (skill Type 11)

Caster = `idChar2 != 0 ? idChar2 : id3`. Conditions: `!PetExits(conn, targetId) && target._Type!=2 && target._Type!=4 && GetDataNpc(targetId,_Bat)==0 && _Type3_Id==0 && (casterLv - npcLv >= 5) && round(target._Hp/_HpMax*100) < 50`.
- Roll `RandomizeArrayWithPercent(1, 0, 50 + round((casterLv-npcLv)/2))`; **1 = success** if a free pet slot exists (any of Stt 1-4 with id 0): remove the npc from grid (`ChangedWar(...,0,...)`), `Data.Addpet(caster, npcId)`, sleep 2000ms, battle continues. Else if success but no slot → `num26=15002` (failure animation). Roll 0 → `num26=15002`. On failure: emits a Miss effect packet.
- 15001/15002/15003 are the "catch" animation skills (always-allowed set).

### 5.7 Drops — GetRandomMissDrop(npcId) (line 9355)

`num = random_0.Next(1,1000)`; band thresholds `Server.percent_item1..6` = **25,23,20,4,3,1** (cumulative): `[0,25]→Item1`, `(25,48]→Item2`, `(48,68]→Item3`, `(68,72]→Item4`, `(72,75]→Item5`, `(75,76]→Item6`, else 0. Note percent values are static (not read from any config).

### 5.8 NPC skill pick — GetRandomSkillNPC (line 9395)

Defaults missing skills to 10000. Roll 1: `random_0.Next(1,100) <= 5*(reborn+1)` → Skill3. Roll 2: `<= 15*(reborn+1)` → Skill2. Roll 3: `<= 30*(reborn+1)` → Skill2. Else 10000. (Each roll draws fresh random values; the `random_0.Next` calls matter for RNG parity.)

### 5.9 GetRandomMissCombo (line 9348)

`RandomizeArrayWithPercent(1, 0, 100)` — i.e. always returns 1 (never misses). `RandomizeArray` (line 9473) sequentially folds `RandomizeArrayWithPercent(prev, item, 50)` across the list.

---

## 6. Packet output — the full battle packet inventory

Packet framing recap: `F444 <len:LE16> <payload>`. Length counts payload bytes after the 4-byte header. `len = round(hexString.Length/2)` for the payload part. When multiple packets are concatenated into one buffer (e.g. `text2 + "F444…" + "F444…"`), each keeps its own `F444` frame.

### 6.1 `_Packet` (23-byte entity snapshot)

```
Type:X2 | Id:LE32 | IdNpcOnMap:LE16 | IdChar:LE32 | row:X2 | col:X2 |
HpMax:LE16 | SpMax:LE16 | Hp:LE16 | Sp:LE16 | Lv:X2 | Thuoctinh:X2
```

### 6.2 Entity placement / map visibility

| Packet | Meaning | When |
|---|---|---|
| `F4440A000B0402` + `LE32(id)` + `000003` | show entity on map (in battle state) | battle start / member join; sent via `SendToAllClientMapid`; also included in `SendPalyerOnline`/`SendPlayerOnMap` for clients with `_My_IdBattle>0` |
| `F4440A000B0402` + `LE32(id)` + `000005` | show entity variant | member setup |
| `F4440A000B0402` + `LE32(id)` + `000002` | show entity (leader marker) | leader PK setup |
| `F4441A000B0503` + `_Packet` | place npc entity (type 3) in battle grid | grid setup, enemy entities |
| `F4441A000B0505` + `_Packet` | place player/pet entity (type 2/4) in battle grid | grid setup, pet swap |
| `F4441A000B0505` + `_Packet` + `F4440A000B0402` + `LE32(id)` + `000005` | place member + show on map | member setup |
| `F44408000B00` + `LE32(id)` + `0000` | remove entity from map | battle end, flee, disconnect, quit |
| `F44405000B01` + `row:X2` + `col:X2` + `00` | move entity to grid position on map | flee/battle-end repositioning |
| `F44404000B01` + `(row^1):X2` + `col:X2` | hide/clear pet cell in grid | pet dismiss, flee, disconnect |

### 6.3 Player turn input (inbound opcode 0x35) — Client.cs ~7695

Frame: opcode `0x35`, `packet[5]`:
- `01` skill/attack command (len ≥ 12): `packet[6..7]` = caster grid `row,col`, `packet[8..9]` = target `row,col`, `packet[10..11]` (LE16 via smethod_9) = skill id. Validated: rows 0-3, cols 0-4, battle exists, cell exists, `_Id>0`, `_Attacked==0`. Sets `_LvSKill` (player: `SkillGet(skillId,_Lv)`; pet caster: match skill id against pet `IdSkill1..4` → `LvSkill1..4`), `_RowAttack`, `_ColumnAttack`, `_IdSkill`, `_Attacked=1`, and broadcasts `F44404003505` + row + col. (Skills 13008/13032 are accepted with no extra packet.)
- `02` use-item command: `packet[6..9]` = cell row,col, `packet[10..11]` = item id. Items `26001..27165` (potions): reads `Type_Item._Hp/_Sp`; applies `min(hp+hpItem, hpMax)` to the **grid cell** and to the active pet DB record; `HomdoRemoveItem(id,item,1)`; `_Attacked=1`.

### 6.4 Battle setup / start

| Packet | When |
|---|---|
| `F4441C000BFA` + `LE16(DiaHinh)` + `03` + `_Packet(leader)` + `F44403000B0A01` | battle start frame (leader) |
| `F444` + `LE16(4 + textLen/2)` + `0BFA` + `LE16(DiaHinh)` + `text` + `F44403000B0A01` | battle start frame (member/PK), where `text` = marker+packet list (below) |
| `F444` + `LE16(4 + textLen/2)` + `0BFA7000` + `text` + `F44403000B0A01` | PK member frame (**note fixed `7000` instead of DiaHinh**, `SendBattleMemberPlayerPK` line 6931) |

**Grid marker bytes inside `0BFA` text** (concatenated `marker + _Packet`):
- `05` = the recipient's own row entity.
- `03` = leader-adjacent entities: `(row,2)` then `(row^1,2)` — used in `SendBattleMem1` / `SendBattleMem` (NPC battles).
- `02` = leader block in PK variants (`SendBattleMemPkPlayer`, `SendBattleLeaderPlayerPK`, `SendBattleMemberPlayerPK`): `(row,2)` then `(row^1,2)`; plus in the LeaderPK form `(row^1,col)`'s leader-row `(3,col)/(2,col)` pair.
- `64` = party member `(row,col)` then their pet `(row^1,col)` — for each of cols 1,3,0,4 (excluding the recipient's own cell in member frames).
- The member frames also emit, right after `0BFA…`, the map-placement packet `F4440A000B0402`+id+`000002` (leader PK) / `000003` / `000005` per member block.

### 6.5 Turn action packets

| Packet | When |
|---|---|
| `F44402003401` | "your turn" prompt to each player entity each turn |
| `F44404003505` + `row:X2` + `col:X2` | turn-taking / entity-acting indicator (also sent on silenced entities, pet swap, skill input, flee) |
| `F444` + `LE16(len("3201"+text9)/2)` + `3201` + `text9` | **turn action frame** — `text9` = concatenation of per-entity blocks, each `LE16(blockLen)` + `row:X2 + col:X2 + skillId:LE16 + SLdanh:X2 + count:X2 + <effects>` |
| effect (SkillingInt, 10 bytes) | `row:X2 + col:X2 + Miss/Attack:X2 + Attack/Def/Lantranh:X2 + CountHieuUng:X2 + TroiBuffHpSp:X2 + damage:LE16 + BuffOrAttack:X2` |
| effect (Skilling full, 17 bytes) | `0F00 + row + col + skillId:LE16 + SLdanh:X2 + skillType:X2 + rowAttack + colAttack + miss + adl + count + troi + dmg:LE16 + buff` |
| `F444130032010F00` + `row:X2` + `col:X2` + `264E0101` + `row:X2` + `col:X2` + `010301E0000000` | **combo packet** (`text2`) — appended in front of the turn frame when a combo formed |
| `F444130032010F00` + row + col + `LE16(20007)` + `0101` + row + col + `01030119000000` | skill-20007 combo footer (sent after turn flush if `text2` was set) |
| `F44407003501` + `row:X2` + `col:X2` + `troiend:X2` + `0000` | buff/Troi ended (`TroiEnd`) |
| `F44407003501` + row + col + `1` + `LE16(skillId)` | buff started on caster (`TroiStart`, e.g. guard/self-buff) |
| `F44408003504` + `LE16(itemId)` + `npcRow:X2` + `npcCol:X2` + `row:X2` + `col:X2` | item dropped into inventory (drop reward) |
| `F44402000504` / `F44402001408` | leave battle / battle UI exit (sent on win/lose/flee to each member) |
| `F44403000B0A01` | battle start trailer (appended to every `0BFA` frame) |

**TroiBuffHpSp byte values** (DataStructure.cs:1481-1509): `_Miss=0`, `_Type3=0xDD (221)`, `_Type4=0xDE (222)`, `_Type15=0xDF (223)`, `_Type19=0xE1 (225)`, `_Hp=0x19 (25)`, `_Sp=0x1A (26)`, `_Hochu=0x0E (14)`.

**TroiBuffEnd byte values**: `_Type3=1`, `_Type4=2`, `_Type15=3`, `_Type19=5`.

**Attack/Def/Lantranh byte values**: `_Attack=0`, `_Def=1`, `_Lantranh=2`. **Miss/Attack byte**: `_Attack=1`, `_Miss=0`.

### 6.6 Status update packets (DB writes)

`Data.PlayerUpdateDataId` / `Data.PetUpdateData` write the DB **and** push `F4440C000801` + `<status:hex>` + `01`/`02`(up/down sign) + `LE32(abs(value))` + `00000000` to the player, plus a party status packet. Status bytes (Type_Status): `_Hp=19`, `_Sp=1A`, `_Int=1B`, `_atk=1C`, `_def=1D`, `_agi=1E`, `_hpx=1F`, `_spx=20`, `_lv=23`, `_TExp=24`, `_SkillPoint=25`, `_Point=26`.

---

## 7. Edge cases

### 7.1 Death mid-battle
- Dead entities (`_Hp<=0`) are skipped for input, deal no damage, and their `_Attacked` is forced to 1 so the turn never stalls. Cells are never removed — `_Id` stays (kill detection relies on `_Hp<=0`). An entity with `_Hp<=0` and skill 20006 loses the shield.
- When a pet dies (hp<=0) the DB pet is set to 0 HP; at battle end, HP is restored to 1 (or the pet is removed if `Fai < 20`).

### 7.2 Kill exp and drops
Accumulated during turns (§3.6) into `_Exp` on player cells; paid out at battle end (§3.8). Level-diff table and the `round(x*1.086)` combo bonus are in §3.6. Level-up side effects: new Hp/Sp maxes from `getHpMax(getSpMax)`, +2 point/level, +1 skill point/level.

### 7.3 Quest integration
- `_My_TalkingBattle` / `_My_WarpingId` set at trigger time link the battle to the `Data_Talks` entry (keyed `MapId-Type-Id-Step`, step read from the per-player `Quest` table).
- Win (`num==1`): `_WinDialogs[0]` sent, else `Data.BattleQuestWin` (consumes `RequireItems`, grants `WinRewards`, one random `WinRandomRewards`, `WinWarpTo`, `WinSaveLeaderQuests`/`WinSaveMemberQuests` quest step updates, `WinUseItems`, `WinAddSkill`, `WinAddPet`, `WinPlayerEnhanceData`). Lose: `_LoseDialogs[0]` sent.
- `_My_AfterBattleType` set to `1` (win) / `-1` (lose) before dialog display; cleared when the dialog flow completes.

### 7.4 Disconnect during battle (Client.cs `shutdown`, line 466-545)
- The disconnecting player's cells (own row+pet) are cleared (`ChangedWar` zeros), the party sees `F44404000B01`+pet cell, `F44408000B00`+id+`0000`, `F44405000B01`+row+col+`00`; map broadcast of the hide packet; `_My_IdBattle=0`.
- If the **leader** disconnects, each party member in battle has their cell + pet cell cleared, `_My_IdBattle=0`, hide broadcast, and `GiaiTanParty` runs.
- `ListQS` entries are only cleared by explicit leave (`Update_HB` case 1: find and zero the slot, `_My_IdBattle=0`, send `F44408000B00`+id+`0000`).

### 7.5 Party disband mid-battle (GiaiTanParty)
Sets `_LeaderId=0` on every cell whose `_Id` or `_IdChar` is a party member (so battle end logic treats the disbanded players as leaders of their own).

### 7.6 Join-in-progress battle (`Update_HB` case 4, line 1327-1413)
An out-of-battle player can join a battle their leader is in: find a free `ListQS` slot, assign it, set `_My_IdBattle`, then send a full custom frame `F444<len>0BFA + LE16(leader._my_DiaHinh)` + `0402` + `LE32(newPlayerId)` + `000000000000FFFF` + `LE16(HpMax)LE16(SpMax)LE16(Hp)LE16(Sp)` + `Lv:X2` + `Thuoctinh:X2` + per-cell records: `marker:X2 + type:X2 + LE32(id) + LE16(idNpcOnMap) + LE32(idChar) + row:X2 + col:X2 + LE16(hpMax)+LE16(spMax)+LE16(hp)+LE16(sp)+Lv:X2+TT:X2` where `marker = 3` if `id==leaderId || idChar==leaderId` else `0x64 (100)`; `idNpcOnMap` is zeroed for non-npc types. Trailer `F44403000B0A01`. (`ClientBattle.JamPlayerToBattle` is an **empty stub**.)

### 7.7 Npc respawn after battle/flee
When a battle ends by flee or the npc's map instance has `_Delay==0`, the npc is repositioned at a random point within its `_Coord` box (packets `F44406001603` + id + `0A00` and `F44408001605` + id + `LE16(x)LE16(y)`), and `_Delay=10` (delayed respawn counter, decremented elsewhere).

---

## 8. Battle packet template summary table

| Template | Opcode | Payload | Purpose |
|---|---|---|---|
| `F4441C000BFA` + `LE16(DH)` + `03` + `_P` + `F44403000B0A01` | 0B FA | DH, marker, entity, trailer | battle start (leader) |
| `F444`+`LE16(4+t/2)`+`0BFA`+`LE16(DH)`+`markers`+`F44403000B0A01` | 0B FA | DH + marker/entity list | battle start (member) |
| `F444`+`LE16(4+t/2)`+`0BFA7000`+`markers`+`F44403000B0A01` | 0B FA | fixed `70 00` + list | battle start (PK member) |
| `F4440A000B0402` + `LE32(id)` + `00 00 03` | 0B 04 02 | id, tail | show player on map |
| `F4440A000B0402` + `LE32(id)` + `00 00 05` | 0B 04 02 | id, tail | show player on map (member) |
| `F4440A000B0402` + `LE32(id)` + `00 00 02` | 0B 04 02 | id, tail | show player on map (leader PK) |
| `F4441A000B0503` + `_P` | 0B 05 03 | entity snapshot | npc entity in grid |
| `F4441A000B0505` + `_P` | 0B 05 05 | entity snapshot | player/pet entity in grid |
| `F44408000B00` + `LE32(id)` + `00 00` | 0B 00 | id | remove entity from map |
| `F44405000B01` + `row col 00` | 0B 01 | pos | reposition on map |
| `F44404000B01` + `(row^1) col` | 0B 01 | pos | clear pet grid cell |
| `F44402003401` | 34 01 | – | your turn |
| `F44404003505` + `row col` | 35 05 | pos | acting indicator |
| `F444`+`LE16`+`3201`+`blocks` | 32 01 | action blocks | turn actions |
| `F444130032010F00`+`rc`+`264E0101`+`rc`+`010301E0000000` | 32 01 | combo frame | combo display |
| `F44407003501` + `row col end 0000` | 35 01 | buff end | TroiEnd |
| `F44408003504` + `LE16(item)` + `4 pos bytes` | 35 04 | item+pos | drop reward |
| `F44402000504` / `F44402001408` | 05 04 / 14 08 | – | battle exit UI |
| `F44403000B0A01` | 0B 0A 01 | – | battle start trailer |
| `F4440C000801` + `st` + `sign` + `LE32(v)` + `00000000` | 08 01 | status | stat change push |

`_P` = the 23-byte `_Packet` (§6.1); `DH` = DiaHinh (`LE16`); all lengths `LE16`; ids `LE32`.

---

## 9. Notes / traps for the Rust port

- **RNG parity**: three `Random` streams with the exact role split in §1.1. `RandomizeArrayWithPercent` clamps p to 100; drop roll uses `Next(1,1000)` inclusive; skill-pick uses four separate `Next(1,100)` draws; damage jitter uses `random_1.Next(0,2)`; Type15 dodge uses a **fresh `new Random()`** (not the instance streams). Combo never misses (`GetRandomMissCombo` always 1).
- **`.NET` rounding**: all damage steps use `Math.Round` (banker's rounding — .NET `MidpointRounding.ToEven`) on doubles then cast/assign to int. Lengths use `Math.Round(len/2.0)`.
- **Division semantics**: `num36 / 3`, `/ 5` are **integer divisions** (`int`), but `num36 / (num34*0.75)` is double→round. `10.0/num30` etc. are double.
- **Avg level math**: `num2 = num3/num4` double division of ints; `Math.Round` when passed to miss/exp formulas.
- **Combo `num37`**: starts 2.0; set to 1.3 multiplier on combo trigger; the damage pipeline multiplies once (`* num37`). `num37 *= 1.3` can stack when multiple combo triggers occur.
- **Order of DB writes vs local mutations**: DB HP/SP writes happen for players/pets with clamped `max(0, …)` values; npc types (3/7) only mutate local cells. `PlayerUpdateDataId`/`PetUpdateData` also push status packets and must be replicated.
- **PK battles use DiaHinh 112; active-NPC battles use 4712; quest battles use `TeamDef[0]`; hardcoded `7000` in the PK-member frame.**
- **`SendBattleMem` (line 6939) and `SendBattleMemberPlayerPK` (line 6651)** are public but have **zero callers** (dead code). The live call graph is: `BattlePkPlayer` → `SendBattleLeader`, `SendBattleMemPkPlayer`, `SendBattleLeaderPlayerPK`; `BattleNpc` → `SendBattleMem1`. Keep the markers (`02`/`03`/`05`/`64`) exact per frame type.
- **Threading**: battle runs on its own background thread; input arrives on client threads mutating the same `ListWar`/`_keys` (no locking anywhere — replicate the data-race-tolerant structure only if the Rust design is single-threaded per battle, which is safe).

---

## 10. Open gaps / items needing a second pass

1. **`smethod_5` checksum byte** (Class5.cs) — exact algorithm not captured in this doc; needed for byte-faithful wire output.
2. **`GetTurn` edge cases** for `14013` (falls through to the Group-F switch) — confirm mapping.
3. **`SkillGet`** (player skill level lookup on input) and **`Data.GetDataNpcOnMap`**, **`GetDataItem(_Hp/_Sp)`** potion values — referenced but live outside TheBattle.
4. **`Math.Round` on `num112`/`num117` exp** and `getHpMax/getSpMax` tables (`Data.cs:5537/5553`) — separate research item.
5. **`Data.BattleQuestWin`** full side effects (win reward item/quest/use-item application) — partially captured; recommend a dedicated quest-integration doc.
6. **`RandomizeArray` fold order** and `RandomizeArrayWithPercent(prev, item, 50)` — call-count parity for RNG streams across the 20-cell loop must be verified against a captured live trace (no tests exist in the repo).
