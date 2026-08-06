# TS Dream — Static Data File Formats (byte-exact, from C# server)

Source of truth: `ts_server_old/` (C#). Every section gives the C# function and exact
line numbers so the Rust executor can reimplement without reading C#.

Files referenced (relative to `ts_server_old/`):
- `Server_TS_Online/Data.cs` — all `LoadData*`, `CreatMapNpc`, `CreatMapItem`, `MemberGet*`,
  `GetDataTalk*`, `genTalkInfo*`, `BattleQuestWin`, `CheckQuestRequiredReady`
- `Server_TS_Online/DataStructure.cs` — the structs + static string constants
- `Server_TS_Online.DataTools/` — `ItemData.cs`, `NpcData.cs`, `SceneData.cs`, `IniFile.cs`,
  `TextEncoder.cs` (binary `Data_Client/*.dat` loaders + `writeToFile` producers)
- `Class6.cs` — Win32 `GetPrivateProfileStringW` / `WritePrivateProfileStringW` wrappers
- `Class5.cs` — hex helpers `smethod_11/12/13`, `smethod_9/10`
- `Data/` — the actual data files
- `CSDL/schema.sql`, `Data/shopp_schema.sql` — Access DB schemas (context only)

The C# server is a **decompiled** codebase: line numbers refer to the decompiled
`Data.cs`/`DataStructure.cs`/etc. as committed. Types are exact; only names were
obfuscated (`array`, `text`, `num`, `value`). All integer columns are `int` (i32);
`Conversions.ToInteger` = VB int parse (thrown `FormatException` on garbage, same as
`int.Parse`). `Array.ConvertAll(x, int.Parse)` on a string[] = parse each element.

---

## 0. Global load pipeline and thread chain

Entry point: `FormServer.FormServer_Load` → `Button_Start` path spawns
`Thread(Data.LoadDataItems)` (FormServer.cs:3306). Loaders spawn each other on
background threads; the final loader calls `Data.Loaded()` (Data.cs:4039-4044) which
sets `LoadedData = true` (the "server open" flag). Chain:

```
LoadDataItems  (Data.cs:4201)  ──spawns──▶ CreatMapItem (5347) ──spawns──▶ _RemoveItemOnMap (5414), ItemOnMapShow (5459)
                └──spawns──▶ LoadDataNpcs (4046) ──spawns──▶ CreatMapNpc (4889) ──spawns──▶ NpcOnMapWalk (4939)
                                              └──spawns──▶ LoadDataSkills (4383) ──spawns──▶ LoadDataWarps (4504)
                                              └──spawns──▶ LoadDataDolls  (4790)
                                                                  LoadDataWarps ──spawns──▶ LoadDataTalks (4585)
                                                                      LoadDataTalks ──spawns──▶ LoadDataTexps (4677) [only if !LoadedData]
                                                                          LoadDataTexps ──spawns──▶ LoadDataBattleGates (4750)
                                                                              LoadDataBattleGates ──▶ Loaded() (4039)
```

All threads `IsBackground = true`. `LoadDataTalks` re-reads quests on reload
(`LoadedData == true` → re-`new` the `Data_Talks` dict, Data.cs:4588-4592) and does
NOT re-spawn `LoadDataTexps` (4664-4673).

In-memory tables (declared Data.cs:32-66, initialized in `static Data()` 71-96):

| Field | Type | Loader |
|---|---|---|
| `Data_Npcs` | `Dictionary<int, Npcs>` | `LoadDataNpcs` (4046) |
| `Data_Items` | `Dictionary<int, Items>` | `LoadDataItems` (4201) |
| `Data_Skills` | `Dictionary<int, Skills>` | `LoadDataSkills` (4383) |
| `Data_Warps` | `Dictionary<Key_Warps, Warps>` | `LoadDataWarps` (4504) |
| `Data_Talks` | `Dictionary<Key_Talk, _Talk>` | `LoadDataTalks` (4585) |
| `Texps` | `Dictionary<int, _Texp>` | `LoadDataTexps` (4677) — computed, no file |
| `Data_BattleGates` | `Dictionary<BattleGates_key, BattleGates>` | `LoadDataBattleGates` (4750) |
| `NpcOnMap` | `Dictionary<Key_NpcOnMap, _NpcOnMap>` | `CreatMapNpc` (4889) |
| `_ListKeysNpcOnMap` | `ArrayList` | entries with SoLuong>0 (4913-4916) |
| `ItemOnMap` | `Dictionary<Key_ItemOnMap, _ItemOnMap>` | `CreatMapItem` (5347) |
| `_ListKeysItemOnMap` | `ArrayList` | every ItemOnMap entry (5392) |
| `ItemDropOnMap` | `Dictionary<Key_ItemDropOnMap, _ItemDropOnMap>` | `CreatMapItem` pre-fills 255 slots/map (5362-5383) |
| `_ListKeysItemDropOnMap` | `ArrayList` | every slot key (5378) |
| `Data_Dolls` | `Dictionary<int, Dolls>` | `LoadDataDolls` (4790) |
| `dictionary_0`, `dictionary_1` | `Dictionary<int, string[]>` | **never populated** — dead (only decl 62-64, init 94-95) |
| `MaxLevel` | `int` = 200 | `static Data()` (72) |

---

## 1. Common text-file conventions (applies to all `Data/*.txt` except quests/Member.ini)

- Read with `File.ReadAllLines(path)` (BOM auto-detected; see per-file encoding).
- Iterate `foreach` over lines.
- **Termination rule:** the first line with `text.Length <= 0` (empty string)
  **breaks** the loop — everything after it is ignored. Files have no interior blank
  lines, so in practice this only catches a trailing empty line.
- Each line: `text.Split('\t')` → `string[]` (C# `Split` on a single char; no
  `RemoveEmptyEntries`, so trailing/empty tabs produce empty-string elements).
- Column index = element index `array2[i]`; extra columns are ignored.
- Header/comment rows: first element `array2[0].StartsWith("//")` → skip that row
  (`continue` or equivalent). Items.txt and Npcs.txt have **two** `//` rows
  (`//Data_Client` then the column-header row); all other `.txt` files have one.
  BattleGate.txt additionally contains a commented-out data row (`//12922\t3\t...`
  at file line 69) and Warps.txt has 48 commented-out warps — all skipped by the
  same rule. The column-header row's names are the authoritative column names
  (listed below).
- `Conversions.ToInteger(array2[i])` on a non-numeric value throws → load crashes.
- There are NO default values for missing columns in the text loaders: every row must
  have at least `index_max+1` columns (verified: only some rows have extra trailing
  empty columns, which are ignored).

### 1.1 Encoding facts (byte inspection, `xxd`)

| File | BOM | Encoding | Line endings |
|---|---|---|---|
| `Items.txt` | none | UTF-8, no BOM (contains raw control byte `0x14` and mojibake `¤`/`ö` — see below) | CRLF |
| `Npcs.txt` | **UTF-16LE (`ff fe`)** | UTF-16LE (names are VISCII codepoints zero-extended to UTF-16) | **LF only (`0a00`), no CR** |
| `NpcOnMap.txt` | none | ASCII | CRLF |
| `ItemOnMap.txt` | none | ASCII | CRLF |
| `Warps.txt` | none | ASCII | CRLF |
| `Skills.txt` | none | UTF-8 (proper Vietnamese) | CRLF |
| `BattleGate.txt` | none | ASCII | CRLF |
| `Dolls.txt` | none | ASCII | CRLF |
| `EVe.txt` | none | ASCII | CRLF |
| `Member.ini` | none | ASCII | CRLF |
| `Quests/*.ini` | none | ASCII; `Title=` values are **8-bit VISCII/ANSI** (Thai pages use TIS-620/cp874) | CRLF |

`File.ReadAllLines` uses `StreamReader` with byte-order-mark detection:
- UTF-16LE BOM (`ff fe`) → decoded as UTF-16LE (`Npcs.txt`).
- No BOM → UTF-8 default (`Items.txt`, `Skills.txt` — both are valid UTF-8).
- ASCII files are trivially UTF-8.

**Name strings are opaque.** The server stores decoded text verbatim and never
normalizes it. `Items.txt` names contain mojibake (`D¤u Ch¤m Höi` — the `¤` is
U+00A4; VISCII bytes were Latin-1-misinterpreted and UTF-8-encoded) and one raw
control char `0x14` (item id 11035 `Kiªm \x14 Thiªn`). `Npcs.txt` names are VISCII
codepoints zero-extended into UTF-16 (`Trß½ng Giác`: `ß`=U+00DF, `½`=U+00BD, `á`=U+00E1
are the VISCII bytes as-is). The Rust loader must decode to UTF-16/UTF-8 strings and
preserve the exact code points — do **not** apply any VISCII→Vietnamese conversion.
The `TextEncoder.convertToUniCode` class exists in DataTools but is **never called**
by server code (dead code; only the GM tool commands `/loadnpcs`, `/loaditems`,
`/loadscenes` — FChat.cs:326-372 — use the DataTools on `Data_Client/*.dat`).

Names are transmitted to clients via `Class5.smethod_13` (each char → `AscW` low byte
→ 2 hex digits, Class5.cs:340-353), so code points are preserved on the wire too.

---

## 2. `Data/Items.txt` — `LoadDataItems` (Data.cs:4201-4251)

- Delimiter `\t`. Comment/header rows: `//` prefix.
- File layout (header row, verbatim): `//Id \tName \tLevel \tHp \tSp \tInt1 \tAtk1
  \tDef1 \tHpx1 \tSpx1 \tAgi1 \tFai1 \tInt2 \tAtk2 \tDef2 \tHpx2 \tSpx2 \tAgi2
  \tFai2 \telement \telem_val \tequippos \tRbPetFrom \tRbPetTo \tAddPet`
  (matches `ItemData.writeToFile` header, ItemData.cs:147-173).
- Row count: 8,378 physical lines → **8,376 data rows**
  (8,369 rows × 26 cols, 6 rows × 25 cols, 1 row × 27 cols; all ≥ 25 → safe).
- Column map (index → struct field, Data.cs:4215-4239):

| idx | header | struct | type |
|---|---|---|---|
| 0 | Id | `_id` | int |
| 1 | Name | `_Name` | string (verbatim UTF-8) |
| 2 | Level | `_Lv` | int |
| 3 | Hp | `_Hp` | int |
| 4 | Sp | `_Sp` | int |
| 5 | Int1 | `_Int1` | int |
| 6 | Atk1 | `_Atk1` | int |
| 7 | Def1 | `_Def1` | int |
| 8 | Hpx1 | `_Hpx1` | int |
| 9 | Spx1 | `_Spx1` | int |
| 10 | Agi1 | `_Agi1` | int |
| 11 | Fai1 | `_Fai1` | int |
| 12 | Int2 | `_Int2` | int |
| 13 | Atk2 | `_Atk2` | int |
| 14 | Def2 | `_Def2` | int |
| 15 | Hpx2 | `_Hpx2` | int |
| 16 | Spx2 | `_Spx2` | int |
| 17 | Agi2 | `_Agi2` | int |
| 18 | Fai2 | `_Fai2` | int |
| 19 | element | `_Thuoctinh` | int (element: 0 none,1 earth,2 water,3 fire,4 wind — cf. Class5.smethod_8) |
| 20 | elem_val | `_GiatriThuoctinh` | int |
| 21 | equippos | `_Loai` | int (item category / equip slot type) |
| 22 | RbPetFrom | `_RbPetFrom` | int |
| 23 | RbPetTo | `_RbPetTo` | int |
| 24 | AddPet | `_AddPet` | int |

- `Data_Items.Add(_id, value)` — key is col 0. Duplicate ids: `Dictionary.Add` throws.
- See also `GetDataItem(_id, type)` (4253-4339) and `GetDataItem(_id)` → `HomdoInfo`
  (4341-4376, sets `_Count=1`, `_Doben=0`, `_Long=0`, `_GiatriLong=0`, `_Khang=0`,
  `_TExp=0`).

### 2.1 Binary origin (context, not loaded at startup)

`Data_Client/ITEM.DAT` (2,948,900 bytes; 7,969 records; 370-byte zero header) is
decoded by `ItemData.LoadItems()` (ItemData.cs:58-141) under GM `/loaditems`:
- 370-byte header (`FIELD_LENGTH`, ItemData.cs:13) then 370-byte records
  (`ItemInfo`, ItemInfo.cs — `Pack=1`, size 370).
- Per-field decode: `DecodeItem8` `(b ^ 0x9A) - 9` (ItemData.cs:40-43);
  `DecodeItem16` `(u ^ 0xEFC3) - 9` (35-38); `DecodeItem32` `(u ^ 0xB80F4B4) - 9`
  (25-28); `DecodeItem32s` signed `(i ^ 0xB80F4B4) - 109` (30-33).
- `prop1_val`/`prop2_val` get `+100` when `prop1/prop2` in 65..67 (79-87).
- Name bytes reversed in-place for 10 iterations (117-122); description reversed for
  127 iterations (123-128).
- `writeToFile` (143-403) is what produced `Items.txt` — it only emits
  `prop1/prop2` values for specific property codes (25,26,212,210,211,207,208,214,64
  and 65/67 special) else `0`.

---

## 3. `Data/Npcs.txt` — `LoadDataNpcs` (Data.cs:4046-4098)

- Delimiter `\t`; `//` header rows skipped.
- **Encoding: UTF-16LE with BOM, LF-only line endings** (see §1.1).
- Header row (verbatim, matches `NpcData.writeToFile`, NpcData.cs:153-178):
  `//Id \tName \tLevel \tElement \tHpMax \tSpMax \tHpx \tSpx \tInt \tAtk \tDef
  \tAgi \tSkill1 \tSkill2 \tSkill3 \tSkill4 \tDrop1 \tDrop2 \tDrop3 \tDrop4
  \tDrop5 \tDrop6 \tNotPet \tReborn`
- Row count: 6,676 physical lines (LF) → **6,673 data rows**
  (6,665 rows × 25 cols, 8 rows × 24 cols; both ≥ 24 → safe).
- Column map (Data.cs:4060-4083):

| idx | header | struct | type |
|---|---|---|---|
| 0 | Id | `_Id` | int |
| 1 | Name | `_Name` | string (verbatim UTF-16 VISCII) |
| 2 | Level | `_Lv` | int |
| 3 | Element | `_Thuoctinh` | int |
| 4 | HpMax | `_Hp` | int |
| 5 | SpMax | `_Sp` | int |
| 6 | Hpx | `_Hpx` | int |
| 7 | Spx | `_Spx` | int |
| 8 | Int | `_Int` | int |
| 9 | Atk | `_Atk` | int |
| 10 | Def | `_Def` | int |
| 11 | Agi | `_Agi` | int |
| 12 | Skill1 | `_Skill1` | int (skill id) |
| 13 | Skill2 | `_Skill2` | int |
| 14 | Skill3 | `_Skill3` | int |
| 15 | Skill4 | `_Skill4` | int |
| 16 | Drop1 | `_Item1` | int (drop item id) |
| 17 | Drop2 | `_Item2` | int |
| 18 | Drop3 | `_Item3` | int |
| 19 | Drop4 | `_Item4` | int (rare drop) |
| 20 | Drop5 | `_Item5` | int (rare drop) |
| 21 | Drop6 | `_Item6` | int (rare drop) |
| 22 | NotPet | `_Bat` | int (0 = usable as pet, else not) |
| 23 | Reborn | `_Reborn` | int |

- `Data_Npcs.Add(_Id, value)`.
- After load: spawns `CreatMapNpc`, `LoadDataSkills`, `LoadDataDolls` (4088-4096).

### 3.1 Binary origin (context)

`Data_Client/Npc.Dat` (522,284 bytes; 5,676 records; 92-byte header) decoded by
`NpcData.LoadNpcs()` (NpcData.cs:56-147): `DecodeItem8` `val==200→255 else
(val ^ 0xC8)-1` (44-54); `DecodeItem16` `(u ^ 0x5209)-1` (39-42);
`DecodeItem32(s)` `(v ^ 0xBAEB716)-1` (29-37). Name reversed 7 iterations (103-108).
Builds `NpcData.drop` (drop1..3) and `NpcData.rareDrop` (drop4..6) maps (110-135).
`NpcData.writeToFile` produced `Npcs.txt`.

---

## 4. `Data/Skills.txt` — `LoadDataSkills` (Data.cs:4383-4424)

- Delimiter `\t`; `//` header skipped.
- Encoding: UTF-8, no BOM, CRLF.
- Header row (verbatim): `//Id\tName\tSp\tPoint\tThuocTinh\tIdDK1\tIdDK2\tIdDK3
  \tIdDK4\tIdDK5\tIdDK6\tLvMax\tType\tDoManh\tSLDanh\tReborn\tCombo\tDelay
  \tTroiBuff` (19 columns).
- Row count: 393 physical lines, no trailing empty → **392 data rows** (all 19 cols).
- Column map (Data.cs:4396-4416):

| idx | header | struct | type |
|---|---|---|---|
| 0 | Id | `_ID` | int |
| 1 | Name | `_Name` | string (verbatim UTF-8 Vietnamese) |
| 2 | Sp | `_Sp` | int (SP cost) |
| 3 | Point | `_Point` | int (skill point cost) |
| 4 | ThuocTinh | `_Thuoctinh` | int |
| 5 | IdDK1 | `_IdDK1` | int (prerequisite skill id) |
| 6 | IdDK2 | `_IdDK2` | int |
| 7 | IdDK3 | `_IdDK3` | int |
| 8 | IdDK4 | `_IdDK4` | int |
| 9 | IdDK5 | `_IdDK5` | int |
| 10 | IdDK6 | `_IdDK6` | int |
| 11 | LvMax | `_LvMax` | int (max skill level) |
| 12 | Type | `_Type` | int (skill type) |
| 13 | DoManh | `_DoManh` | int (power) |
| 14 | SLdanh | `_SLdanh` | int (number of targets/hits) |
| 15 | Reborn | `_Reborn` | int |
| 16 | Combo | `_Combo` | int |
| 17 | Delay | `_Delay` | int (ms) |
| 18 | TroiBuff | `_TroiBuff` | int |

- `Data_Skills.Add(_ID, value)`; then spawns `LoadDataWarps` (4420-4422).

---

## 5. `Data/Warps.txt` — `LoadDataWarps` (Data.cs:4504-4545)

- Delimiter `\t`; `//` header skipped.
- Encoding: ASCII, CRLF.
- Header row (verbatim): `//map1\twarpid\tmap2\tx\ty` (5 columns).
- Row count: 5,043 physical lines → **4,994 data rows** (4,991 rows × 5 cols,
  3 rows × 6 cols — the 6th is an empty trailing column). 49 rows start with `//`
  (1 header + **48 commented-out warps**, e.g. lines 3653-5020 `//25000\t51...`),
  all skipped.
- **Termination rule differs:** `if (text.Length < 5) break;` (4509) — lines shorter
  than 5 chars stop the loop. **Skip rule:** `array2[0].StartsWith("//") ||
  array2[2].Length <= 0` → `continue` (4514). So rows whose destination map column is
  empty are silently dropped.
- Column map (Data.cs:4519-4525):

| idx | header | struct | type |
|---|---|---|---|
| 0 | map1 | `_MapId1` | int (source map) |
| 1 | warpid | `_WarpId` | int (source warp entry) |
| 2 | map2 | `_MapId2` | int (destination map) |
| 3 | x | `_X` | int (dest X) |
| 4 | y | `_Y` | int (dest Y) |

- `_Battle` exists in `Warps` struct (DataStructure.cs:382) but is **never loaded**
  (stays 0).
- Key: `GetKey_Warps(_MapId1, _WarpId)` (4495-4502). Duplicate key → skipped
  (`if (!Data_Warps.ContainsKey(key))`, 4527), which hides the crash from `Add`.
- Accessors: `GetDataWarpExits` (4547), `GetDataWarp(_MapId1,_WarpId,type)` (4553,
  supports "MapId1","WarpId","MapId2","X","Y","Battle").
- Then spawns `LoadDataTalks` (4541-4543).

---

## 6. `Data/BattleGate.txt` — `LoadDataBattleGates` (Data.cs:4750-4788)

- Delimiter `\t`; `//` header skipped.
- Encoding: ASCII, CRLF.
- Header row (verbatim): `//Mapid1\tWarpId\tDiahinh\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10`
  (13 columns).
- Row count: 71 physical lines → **68 data rows** (all 13 cols). Two `//` rows:
  the header and a commented-out battle gate (`//12922\t3\t1700\t...` at line 69).
- Termination rule: `if (text.Length < 5) break;` (4755).
- Column map (Data.cs:4763-4777):

| idx | header | struct | type |
|---|---|---|---|
| 0 | Mapid1 | `_MapId` | int |
| 1 | WarpId | `_WarpId` | int |
| 2 | Diahinh | `_Diahinh` | int (battle field/terrain id) |
| 3 | 1 | `_1` | int (defender npc id, row 1) |
| 4 | 2 | `_2` | int |
| 5 | 3 | `_3` | int |
| 6 | 4 | `_4` | int |
| 7 | 5 | `_5` | int |
| 8 | 6 | `_6` | int |
| 9 | 7 | `_7` | int |
| 10 | 8 | `_8` | int |
| 11 | 9 | `_9` | int |
| 12 | 10 | `_10` | int |

- Key: `BattleGates_key { _MapId, _WarpId }` (4778-4782). `Data_BattleGates.Add`
  directly (no duplicate guard — 4783).
- `GetDataBattleGate(_key, type)` accessor at 4831-4878 (also "MapId","WarpId").
- Calls `Loaded()` (4786) — the final link in the startup chain.

---

## 7. `Data/Dolls.txt` — `LoadDataDolls` (Data.cs:4790-4811)

- Delimiter `\t`; `//` header skipped.
- Encoding: ASCII, CRLF.
- Header row (verbatim): `//DollId\tNpcId` (2 columns).
- Row count: 99 physical lines, no trailing empty → **98 data rows** (all 2 cols).
- Termination rule: `if (text.Length <= 0) break;` (4795).
- Column map (4803-4806):

| idx | header | struct | type |
|---|---|---|---|
| 0 | DollId | `_DollId` | int (doll item id) |
| 1 | NpcId | `_NpcId` | int (the NPC/pet that item summons) |

- `Data_Dolls.Add(_DollId, value)`; `GetDataDolls(_id, type)` at 4813-4829.

---

## 8. `Data/NpcOnMap.txt` — `CreatMapNpc` (Data.cs:4889-4937)

- Delimiter `\t`; `//` rows `continue` (skip); empty line → `break` (4894-4896).
- Encoding: ASCII, CRLF.
- Header row (verbatim): `//MapId\tId\tNpcId\tX\tY\tCoord\tSoLuong` (7 columns).
- Row count: 20,266 physical lines, no trailing empty → 20,265 data rows
  (20,264 rows × 7 cols, 1 row × 8 cols with trailing empty tab).
- Column map (4903-4909):

| idx | header | local | type | struct field |
|---|---|---|---|---|
| 0 | MapId | `num` | int | `_MapId` |
| 1 | Id | `id` | int (instance id on that map) | `_Id` |
| 2 | NpcId | `npcId` | int (id into `Data_Npcs`) | `_NpcId` |
| 3 | X | `num2` | int (spawn X) | `_X_First`, `_X` |
| 4 | Y | `num3` | int (spawn Y) | `_Y_First`, `_Y` |
| 5 | Coord | `coord` | int (patrol radius) | `_Coord` |
| 6 | SoLuong | `num4` | int (spawn count / respawn quirk flag) | `_SoLuong` |

- `if (!NpcOnMap.ContainsKey(key))` guard (4911). If `num4 > 0`, key is appended to
  `_ListKeysNpcOnMap` (4913-4916) — the walking/patrol list.
- Runtime defaults set in the struct: `_Delay = 0`, `_IdBattle = 0` (4925-4929).
- Then spawns `NpcOnMapWalk` (4934-4936), a loop (every 900 ms) that moves
  registered NPCs randomly within `[X_First-coord, X_First+coord]` on X and
  `[Y_First-coord, Y_First+coord]` on Y (clamped at 0) — Data.cs:4939-5010+.
- Accessor `GetDataNpcOnMap(_Mapid,_Id,type)` at 5136-5172 (types "MapId","Id",
  "NpcId","X_First","Y_First","Coord","SoLuong").

---

## 9. `Data/ItemOnMap.txt` — `CreatMapItem` (Data.cs:5347-5412)

- Delimiter `\t`; `//` rows `continue`; empty line → `break` (5352-5356).
- Encoding: ASCII, CRLF.
- Header row (verbatim): `//MapId\tId\tItemId\tX\tY\tDelay` (6 columns).
- Row count: 1,162 physical lines, no trailing empty → 1,161 data rows (all 6 cols).
- Column map (5361-5388):

| idx | header | local | type | meaning |
|---|---|---|---|---|
| 0 | MapId | `num` | int | map id |
| 1 | Id | `slot` | int | the ItemDropOnMap slot (1..255) on this map |
| 2 | ItemId | `itemId` | int | item id into `Data_Items` |
| 3 | X | `num3` | int | drop X |
| 4 | Y | `num4` | int | drop Y |
| 5 | Delay | `delay` | int | initial respawn delay |

- **Pre-fill:** the first time a MapId appears, slots 1..255 are created as
  empty `_ItemDropOnMap { _MapId, _Slot }` entries in `ItemDropOnMap` keyed by
  `Key_ItemDropOnMap { _MapId, _Slot }`, each key added to `_ListKeysItemDropOnMap`
  (5362-5383). A second row for the same MapId skips this.
- `ItemOnMap` keyed by `GetKey_ItemOnMap(_Mapid,_ItemId,_X,_Y)` =
  `Key_ItemOnMap { _MapId, _ItemId, _X, _Y }` (5174-5183). Entry:
  `_ItemOnMap { _MapId, _ItemId, _X, _Y, _Delay=delay, _DelayDec=0 }` (5393-5401);
  key appended to `_ListKeysItemOnMap` (5392).
- For each row it then calls `SystemDropItem(mapid, slot, x, y, itemId, 999999)`
  (5403) — the slot-based overload (5278-5345), which **immediately spawns** the item
  into `ItemDropOnMap[mapid,slot]` with `_Delay=999999` (never auto-removes) and
  broadcasts `F44408001703` + item/x/y to the map.
- Runtime threads: `_RemoveItemOnMap` (5414-5457, every 1 s decrements `_Delay`,
  removes slot when it hits 0) and `ItemOnMapShow` (5459-5497, every 1 s decrements
  `_DelayDec`; at 1 → `SystemDropItem(mapid,x,y,itemId,999999)` and `_DelayDec=0`).
- `GetDataItemOnMap(_id)` builds the on-map item list packet for a player (5499-5535).

---

## 10. `Data/EVe.txt` — NOT loaded by the server

20,766 physical lines (ASCII, CRLF), 20,765 data rows, 5 columns, no header row.
Columns appear to be `mapid \t npcid \t <ordinal> \t x \t y` (e.g.
`10701 19018 1 1370 340`). **No C# code reads `EVe.txt`** (grep across all `.cs`:
zero hits). It is legacy/unused by the C# server; the Rust port does not need it
unless replicating legacy behavior. Documented here so it is not mistaken for a
required input.

---

## 11. `Data/Member.ini` — account store

Path: `<exe>/Data/Member.ini` (Data.cs:520, 538, 549). ASCII, CRLF, 24 physical
lines. Format:

```
[Account]
<id>=<pass1>\t<pass2>
```

Example (verbatim): `300003=1111111111	1111111111`. Ids are the numeric account ids
(they double as the `Player.Id` keys; login sends `IDPrefix + id` on the wire and the
server strips the prefix — Client.cs:998-1000).

### 11.1 Read path

- `MemberGetData(_id, type)` (Data.cs:517-532):
  `Class6.smethod_1(path, "Account", _id.ToString(), "")` → value string, then
  `.Split('\t')`; if `type` lowercased == "pass1" → `array[0]`; if == "pass2" →
  `array[1]`; else "".
- `MemberGetIdExits(_Id)` (535-544): returns true iff the INI read result !=
  the sentinel `"nothing"`.
- `Class6.smethod_1(path, section, key, def)` (Class6.cs:16-30) wraps Win32
  `GetPrivateProfileStringW(section, key, def, buffer(1024), len, path)`; returns
  the left-`num` chars of the 1024-char buffer; **returns `"nothing"` when the key is
  absent or empty** (num == 0).
- Note: Win32 INI section/key matching is **case-insensitive**.

### 11.2 Write path

- `MemberChangedPass(_Id, pass1, pass2)` (547-551):
  `Class6.smethod_0(path, "Account", _Id.ToString(), pass1 + "\t" + pass2)`
  → `WritePrivateProfileStringW(section, key, value, path)` (Class6.cs:8-15).
- Account creation (FormServer.cs:3422, 3433) writes default
  `"1111111111\t1111111111"`.
- Win32 `WritePrivateProfileStringW` writes in Unicode/ANSI depending on file state;
  because Member.ini is an ASCII file it stays ASCII in practice. For a Rust port a
  plain UTF-8/ASCII rewrite that preserves `[Account]\n<id>=<p1>\t<p2>\n` order is
  acceptable — but note the Win32 API may insert the key at the top of the section
  and preserves the section header; byte-for-byte reproduction of arbitrary
  reorderings is not guaranteed by the C# code.
- `FormServer.method_3` (FormServer.cs:3498-3524) also parses Member.ini by hand for
  the account list UI: skips lines starting with `[`, splits each line on first `=`,
  then splits the value on `\t` into `[pass1, pass2]`.

---

## 12. `Data/Quests/*.ini` — `LoadDataTalks` (Data.cs:4585-4675)

`Directory.GetFiles(<exe>/Data/Quests, "*.ini", TopDirectoryOnly)` — **only
top-level** `*.ini`, 813 files. Each file parsed via `IniFile.IniReadValue(section,
key)` → Win32 `GetPrivateProfileStringW` with a 1024-char buffer (IniFile.cs:26-35);
**absent key returns the literal string `"nothing"`**. Section and key matching are
**case-insensitive** (Win32 behavior — the code queries "ONWIN"/"ONLOSE" but files
write `[OnWin]`/`[OnLose]`; queries "AddPet" and files may use `Addpet`).

Sections found across all 813 files: `[BASE]` (813), `[REQUIRES]` (560), `[OnWin]`
(551), `[OnLose]` (430), `[TEAMDEF]` (316), `[DESCRIPTION]` (170). No other sections
exist.

Keys found (count): `Dialogs` 1236, `Type` 813, `Step` 813, `MapId` 813, `Id` 813,
`SaveLeaderQuests` 464, `Quests` 329, `Npcs` 316, `Diahinh` 316, `Title` 170,
`SelectMenu` 138, `Rewards` 111, `Items` 79, `Level` 78, `RandomRewards` 72,
`Reborn` 46, `WarpTo` 40, `Addpet`/`AddPet` 11, `PlayerEnhanceData` 8, `AddSkill` 3,
`Wears` 1, `UseItems` 1. `SaveMemberQuests` and `ClickNpcId` are parsed by the loader
but never present in current data. `Step` ranges 0..50; steps 0/1 dominate.

### 12.1 `[BASE]` — identity + dialog list

Keys: `MapId` (int), `Type` ("NPC" or "WARP"; 546 NPC / 267 WARP), `Id` (int;
for NPC the NpcOnMap instance id, for WARP the warp id), `Step` (int; current quest
step), `Dialogs` (tab-separated list of client packet hex strings, e.g.
`F44411001401000000010603010000000000000100`).

`_TalkDialogs = genTalkInfoDialog(Dialogs, '\t')` (4611; helper 6045-6052): if value
== `"nothing"` → empty `string[0]`; else `Split('\t')` (empty elements preserved).
**Empty `Dialogs` ⇒ battle quest** (FTalk treats `GetDataTalkCount()==0` as
"start a TEAMDEF battle" — FTalk.cs:161-190, 2748-2768).

The `F444...` strings are client dialog packets relayed verbatim:
`TalkMessages` (FTalk.cs:3439-3456) splits each `_TalkDialogs` element on the
literal `"F444"` and sends each chunk as `"F444" + chunk` with a 500 ms sleep.
`GetDataTalkString(map,type,id,step,talk)` returns `_TalkDialogs[talk-1]`
(Data.cs:575-587). `GetDataTalkCount` = `_TalkDialogs.Length` (553-573).

### 12.2 `[REQUIRES]` — entry conditions

| Key | Parse fn | Encoding in file | Meaning / struct field |
|---|---|---|---|
| `Level` | `genTalkInfoCondition` (4612) | `120\t>=` (tab, then operator) | `_RequireLevel = [value, opIndex]` |
| `Reborn` | `genTalkInfoCondition` (4613) | `1\t>=` | `_RequireReborn = [value, opIndex]` |
| `Thuoctinh` | raw int (4614-4615) | `1..4` | `_RequireThuoctinh` (0 if absent) |
| `Quests` | `genTalkInfoListInt` (4616) | `13518-0-3-1` (tab-separated tuples, `-` inside) | `_RequireQuests: List<int[]>` |
| `Items` | `genTalkInfoListInt` (4617) | `31088-1-1` | `_RequireItems: List<int[]>` |
| `Wears` | `genTalkInfoListInt` (4618) | `19737-0` | `_RequireWears: List<int[]>` |
| `SelectMenu` | raw int (4619) | `30` | `_RequireSelectMenu` (0 if absent) |

- `genTalkInfoCondition(text, '\t')` (6054-6067): value absent → `int[0]`; else split
  on tab, `[ToInteger(el0), Array.IndexOf(["=",">=",">","<=","<","!="], el1)]`.
  Operator indices: 0 `=`, 1 `>=`, 2 `>`, 3 `<=`, 4 `<`, 5 `!=`. Evaluated by
  `isOperationCompare(numRequire, numChar, type)` (6000-6043) and checked in
  `CheckQuestRequiredReady` (5779-5789).
- `genTalkInfoListInt(text, '\t', '-')` (6069-6084): absent → empty list; each
  tab-element split on `-` and each piece `int.Parse`d → `List<int[]>`.
  - `Quests` tuple = `[mapId, npcId, warpId, step]`. Requirement check
    (CheckQuestRequiredReady 5747-5778): if `warpId>0` require
    `QuestGetDataWarp(mapId, warpId) >= step` else `QuestGetDataNpc(mapId, npcId)
    >= step`. Message title resolves via `_DescTitle` of the referenced step.
  - `Items` tuple = `[itemId, count, removeCount]`. Check requires
    `HomdoGetDataItem(slot)._Count >= count` (5710-5729); on quest win, if the
    player has enough AND `removeCount>0`, removes `removeCount` (5820-5828).
  - `Wears` tuple = `[itemId, playerOrPet]`; 0 → must wear in Trangbi Slot ≤ 6
    (player), 1 → Slot ≥ 11 (pet) (5730-5746).

### 12.3 `[TEAMDEF]` — battle defenders

Keys: `Diahinh` (int; battle field id, e.g. 121, 365, 5479, 28274), `Npcs`
(tab-separated **exactly 10** npc ids).

- `genTalkInfoTeamDefDiahinh` (6119-6126): absent → 0.
- `genTalkInfoTeamDefNpcs(text, '\t')` (6128-6140): absent → `int[10]` zeros; present
  but not exactly 10 elements → `int[10]` zeros; else the 10 parsed ints.
- `_TeamDef = int[11] { diahinh, npc1..npc10 }` (4621-4635). When a player finishes
  the dialogs, FTalk checks `_TeamDef.Sum() > 0 && length == 11` and starts
  `new TheBattle(idLeader, TeamDeffender{_id1.._id10}, _TeamDef[0])`
  (FTalk.cs:2748-2768; same for NPC path at 161-190).
- Accessor `GetDataTalkTeamDefs` returns the array, or `int[11]` zeros when the key
  is missing (601-615).

### 12.4 `[OnWin]` — win outcomes

| Key | Parse fn (line) | Encoding | Struct field |
|---|---|---|---|
| `Dialogs` | `genTalkInfoDialog` (4636) | tab-separated `F444...` | `_WinDialogs: string[]` |
| `WarpTo` | dialog→`int.Parse` (4637) | `11901\t210\t1230` or 4 fields `49960\t782\t1655\t0` | `_WinWarpTo: int[]` |
| `Rewards` | `genTalkInfoListInt` (4638) | `31221-1-0` per reward | `_WinRewards: List<int[]>` |
| `RandomRewards` | `genTalkInfoListInt` (4639) | `62701-1-0` per candidate | `_WinRandomRewards: List<int[]>` |
| `UseItems` | `genTalkInfoListInt` (4640) | `19001-0` | `_WinUseItems: List<int[]>` |
| `SaveLeaderQuests` | `genTalkInfoSaveQuest` (4641) | `AUTO` and/or `57511-0-2-1` | `_WinSaveLeaderQuests: List<int[]>` |
| `SaveMemberQuests` | `genTalkInfoSaveQuest` (4642) | (unused in data) | `_WinSaveMemberQuests: List<int[]>` |
| `PlayerEnhanceData` | `genTalkPlayerEnhanceData` (4643) | `Point-1\tSkillPoint-1` | `_WinPlayerEnhanceData: List<object[2]>` |
| `AddSkill` | dialog→`int.Parse` (4644) | `14001\t1` (skillId, level) | `_WinAddSkill: int[]` |
| `AddPet` | raw int (4645) | `18005` | `_WinAddPet: int` (0 absent) |
| `ClickNpcId` | raw int (4646-4647) | (unused in data) | `_WinClickNpcId: int` |

Reward tuple = `[itemId, count, shareToParty]`; `shareToParty>0` also grants to
party members 1..4. `RandomRewards`: one tuple picked at random, then same granting
(BattleQuestWin 5842-5871). `UseItems` tuple `[itemId, target]`: `target==0` →
use on self, else use on active pet (5872-5898).

### 12.5 `[OnLose]` — loss outcomes

- `Dialogs` → `_LoseDialogs` (4648).
- **`WarpTo` is read from section `ONWIN`, not ONLOSE** — a C# copy-paste bug:
  `value._LoseWarpTo = Array.ConvertAll(genTalkInfoDialog(
  IniReadValue("ONWIN","WarpTo"), '\t'), int.Parse)` (4649). So `_LoseWarpTo` always
  equals `_WinWarpTo`. Replicate for behavioral parity.

### 12.6 `[DESCRIPTION]`

- `Title` → `_DescTitle` (4650-4653), set only if != `"nothing"`. Used in
  `CheckQuestRequiredReady` requirement messages (5770) to name the required quest.
- Encoding: 8-bit VISCII/ANSI; Thai quest titles are TIS-620 (e.g. `Title=` bytes
  `a1d3a8d1...` decode as cp874/TIS-620; Vietnamese titles decode via the VISCII
  table in TextEncoder.cs:15-42). The C# server stores the Win32-decoded string
  (system ANSI codepage); see §14 for the port recommendation.

### 12.7 Key format and runtime lookup

`Key_Talk { _MapId, _Type, _Id, _Step }` (DataStructure.cs:471-480). `Data_Talks.Add`
has no duplicate guard (4661) — a duplicate `(MapId,Type,Id,Step)` throws.

Lookup helpers:
- `GetDataTalkExits(map,type,id,step)` = `ContainsKey` (589-599)
- `GetDataTalkCount` (553-573), `GetDataTalkString` (575-587, 1-based `talk`),
  `GetDataTalkTeamDefs` (601-615)
- `CheckQuestRequiredReady(client,map,type,id,step,showMsg)` (5697-5810) — full
  requirement evaluation (see §12.2)
- `BattleQuestWin(client,key)` (5812-5998) — full win processing: consume required
  items, grant rewards, use items, save quests, enhance stats, add skill/pet, warp.

`genTalkInfoSaveQuest(text, '\t', '-', mapId, type, id, step)` (6086-6117):
`AUTO` expands to `"<mapId>-<npc>-<warp>-<step+1>"` where `<npc>` = `id` if
`type=="NPC"` else 0, `<warp>` = `id` if `type=="WARP"` else 0. Non-AUTO elements
are split on `-` verbatim. Final tuple `[mapId, npcId, warpId, step]`; on win,
`npcId>0` → `QuestUpdateDataNpc(mapId, npcId, step)`, `warpId>0` →
`QuestUpdateDataWarp(mapId, warpId, step)` (BattleQuestWin 5899-5914).

### 12.8 Quest file samples (as shipped)

- Battle quest (WARP + TEAMDEF): `11021-Trung Thu 1.ini` — `Type=WARP Id=4
  Step=0`, `Diahinh=1719`, `Npcs=48069 48065 48061 ...`, `OnWin`
  `SaveLeaderQuests=AUTO`, `RandomRewards=62701-1-0 ...`.
- Warp with prerequisite quests/items: `12001-HQL - 49734-npc 4.ini` —
  `Quests=49734-0-5-2`, `Items=31088-1-1`, TEAMDEF `Diahinh=876`.
- Simple NPC quest: `10916 NPC 1 step 0.ini` — `SelectMenu=30`, `Rewards=10001-1-0`,
  `SaveLeaderQuests=AUTO`.
- Shop quests: `11201 NPC 1 shop trang bị.ini` — **only `[BASE]` with `Dialogs`**
  (two `F444...` packets). Shop UI is opened client-side by the dialog packet; the
  server has no shop data (shopp.accdb / `shopp` table is not read anywhere).
- Reborn/level gated: `12021 triệu quảng 7 step 0.ini` — `Level=20\t>=`,
  `Reborn=1\t>=`, TEAMDEF `Diahinh=5479`, `Rewards`, `RandomRewards`.
- Stat enhancement: `Cs Bàng Cổ Cự Thú 13519 Warp3 step 2.ini` —
  `PlayerEnhanceData=Point-1\tSkillPoint-1`, `Rewards=46070-1-0`.
- Skill grant: `Giản Ung Khảo Nghiệm 12136 NPC 1 step 3.ini` — `AddSkill=14001\t1`.
- Pet grant: `12001 NPC 9 step 1.ini` — `AddPet=18005`; `12003 NPC 13 Đổi Tâm
  tâm.ini` — `Items=31024-1-1`, `Addpet=22036`.
- Item consumption: `namtinhquan 10851 buoc 0.ini` — `Rewards`, `UseItems=19001-0`.
- Warp to location: `11021-Van Du dao Si.ini` — `WarpTo=11901\t210\t1230`.
- Equip requirement: `Quest tai sinh 1 buoc 18.ini` — `Quests=59452-3-0-1`,
  `Level=120\t>=`, `Reborn=1\t>=`, `Wears=19737-0`.
- Description quests: `Cs Bàng Cổ Cự Thú 13519 Warp3 step 1.ini` —
  `[DESCRIPTION] Title=<TIS-620>`, `Quests=13518-0-3-1`, TEAMDEF.
- Multi-step: `10916 NPC 1 step 1.ini` is a bare `[BASE]` (dialogs only) —
  stepping is driven by the player's `Quest` DB row.

---

## 13. Loaded-data → struct mapping summary

All in `DataStructure.cs` (structs: Npcs 116-165, Skills 240-279, Items 317-368,
Warps 370-383, _Texp 401-410, _Talk 412-469, Key_Talk 471-480, _NpcOnMap 528-551,
Key_NpcOnMap 521-526, _ItemOnMap 564-577, Key_ItemOnMap 553-562, _ItemDropOnMap
586-659, Key_ItemDropOnMap 579-584, Dolls 2016-2021, BattleGates 288-315,
BattleGates_key 281-286).

- `Texps` is **computed** (Data.cs:4677-4699): for level `i` in `0..MaxLevel-1`
  (MaxLevel=200), `_0(i) = _0(i-1) + (int)(Round(Pow(i+1, 2.9)) + 5)`,
  `_1(i) = _1(i-1) + (int)(Round(Pow(i+1, 3.0)) + 5)`,
  `_2(i) = _2(i-1) + (int)(Round(Pow(i+1, 3.05)) + 5)` — cumulative total-EXP
  thresholds for reborn 0/1/2. Consumed by `TexpGetLvUp` (4701-4747).
  **No file is read.**
- Derived runtime state (not from files): `_ItemDropOnMap` slots are filled by
  `SystemDropItem` (5194-5345); `ItemOnMapShow`/`_RemoveItemOnMap` mutate `_Delay`,
  `_DelayDec`. `NpcOnMapWalk` mutates `_X`, `_Y`, `_Delay`, `_IdBattle`.
- `dictionary_0`/`dictionary_1` are initialized but never written — do not port
  their "content", only their absence.

---

## 14. Rust port notes / gaps

1. **Encoding must be per-file** (see §1.1): BOM-detect; `Npcs.txt` is UTF-16LE+LF
   only, `Items.txt`/`Skills.txt` are UTF-8, everything else ASCII. Do not
   normalize Vietnamese.
2. **INI semantics must match Win32**, not a generic INI crate:
   - absent key → sentinel `"nothing"` (the loader compares against this literal),
   - section & key matching case-insensitive,
   - values limited to 1024 chars (largest current value: 1004 — safe, but keep the
     cap for parity),
   - lines starting with `;` are comments (none in current data), `//...` lines are
     not comments in data.
3. **`_LoseWarpTo` bug** (§12.5) must be replicated (reads `ONWIN` WarpTo).
4. **Trailing `//` header convention**: every `.txt` loader skips rows whose first
   column starts with `//`; Warps/BattleGate additionally stop at `length < 5`.
5. **Duplicate-key behavior**: `Dictionary.Add` throws on duplicates; the loaders
   mostly guard (Warps, NpcOnMap, ItemOnMap) or not (Items, Npcs, Skills, Talks,
   BattleGates, Dolls). Data has no duplicates for the unguarded tables today.
6. **EVe.txt is unused** — exclude unless legacy parity is wanted.
7. **shopp.accdb / `shopp` table** (`tab2`, `vitri`, `soluong`, `tab1`) is schema-
   only; no server code reads it. Shop UI is driven by client dialog packets.
8. Quest `Title` text: Win32 decodes 8-bit bytes using the server machine's ANSI
   codepage. For the Rust port pick a fixed decode (TIS-620 for the Thai titles,
   VISCII table from TextEncoder.cs:15-42 for Vietnamese) or preserve raw bytes —
   the only server use is user-facing requirement messages, so byte parity is not
   protocol-critical.
9. `File.ReadAllLines` splits on `\r\n`, `\n`; the Rust loader should split on both.
10. Row-count sanity: Items 8,376 data rows; Npcs 6,673; Skills 392; Warps 4,994
    (48 warps commented out); BattleGate 68; Dolls 98; NpcOnMap 20,265;
    ItemOnMap 1,161; quests 813 files; Member.ini 23 accounts (ids 300003..300025).

### Open questions (unresolved)

- Exact ANSI codepage used for quest `Title` on the original server (system
  dependent; see note 8).
- Whether `Member.ini` write ordering/duplicate-account behavior matters (Win32 API
  details; current writes only add at the end of `[Account]`).
- `EVe.txt` purpose (map→npc spawn coordinates?) — confirmed unused by code.
- `Items._Loai` (equippos) enum meaning — only consumed as a `HomdoInfo._Loai`
  integer and slot math in FTalk (e.g. `loai + stt*10` for pet gear, 5892);
  no file-derived enum.
