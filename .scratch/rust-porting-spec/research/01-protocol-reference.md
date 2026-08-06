# TS Dream — Wire Protocol Reference (byte-exact, from C# server)

Source of truth: `ts_server_old/` (C#). Every section gives the C# function and exact
line numbers so the Rust executor can reimplement without reading C#.

Files referenced (relative to `ts_server_old/`):
- `Class5.cs` — byte/hex helpers
- `Server_TS_Online/Client.cs` — per-connection handlers, `Sendpacket`, `Logined1`, `shutdown`
- `Server_TS_Online/Server.cs` — broadcasts, player online/offline, party helpers
- `Server_TS_Online/Data.cs` — pet status, item moves, warp, quest reward packets
- `Server_TS_Online/FTalk.cs` — action/talk (opcode 0x14)
- `Server_TS_Online/FChat.cs` — chat (opcode 0x02)
- `Server_TS_Online/FTienTrang.cs` — bank gold (opcode 0x1D)
- `Server_TS_Online/FWalk.cs` — move (opcode 0x06 sub 1)
- `Server_TS_Online/DataStructure.cs` — structs + string constants (Type_Status etc.)
- `Server_TS_Online/TheBattle.cs` — battle engine

Notation used below:
- `smethod_11(v)` = v encoded as 2-byte little-endian hex (e.g. 300003 → `E393`). Class5.cs:332-335
- `smethod_12(v)` = v encoded as 4-byte little-endian hex (e.g. 1234567 → `D6120000`... actually `smethod_12(1234567)`: 1234567=0x12D687 → bytes LE → `876D1200`). Class5.cs:336-339
- `smethod_13(s)` = each char → 2 hex digits of `AscW(char)` low byte (ASCII-safe; Vietnamese high bytes are mangled). Class5.cs:340-353
- `smethod_9([b0,b1])` = little-endian u16 from 2 bytes (Class5.cs:314-321)
- `smethod_10([b0..b3])` = little-endian u32 from 4 bytes (Class5.cs:322-331)
- `smethod_3(bytes)` = uppercase hex of bytes (Class5.cs:119-131)
- `smethod_4(hex)` = hex string → bytes (Class5.cs:132-152)
- `smethod_5(bytes)` = XOR each byte with 0xAD (173) (Class5.cs:153-166)
- `_Header` = `"F444"` (DataStructure.cs:13-18)

---

## 1. TRANSPORT & FRAMING

### 1.1 Socket
- TCP only. `new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp)`, bind `IPEndPoint(IPAddress.Any, 6414)`, `Listen(5)`, `BeginAccept`. FormServer.cs:3309-3313.
- No handshake on accept; nothing is sent until the client speaks. FormServer.cs:3356-3357.
- Accept is gated on `Data.LoadedData == true`. FormServer.cs:3334-3337.

### 1.2 Receive path
- `_buffer = new byte[8192]`; `BeginReceive(_buffer, 0, 8192, ...)`. Client.cs:451, 573-586.
- `OnRecievedData`: resize buffer to actual byte count → `UpdateMainGrid(_buffer)` → re-arm. 0 bytes received → `shutdown()`. Client.cs:588-612.

### 1.3 Frame decoding (Client.cs:614-644, `UpdateMainGrid`)
1. XOR every byte with 0xAD (`Class5.smethod_5`), convert to uppercase hex (`smethod_3`).
2. Length field = hex substring at offset 4 (chars 4..7 = bytes 2..3), parsed little-endian (`smethod_14`). This is the byte count **after** the 4-byte header.
3. A complete frame is `4 + length` bytes = `8 + length*2` hex chars.
4. Frames are concatenated on the wire; the parser splits them in a loop. A partial trailing frame is kept in `string_2` and prepended to the next received chunk (multi-frame coalescing both directions).
5. Each complete frame is dispatched to `UpdateMainGrid_Recv` on its own thread. Client.cs:628-633.

### 1.4 Frame layout (after XOR decode)
| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 2 | magic `F4 44` (hex `F444`, `_Header`) |
| 2 | 2 | length, LE u16 = number of bytes following the 4-byte header |
| 4 | 1 | opcode |
| 5 | 1 | sub-opcode |
| 6.. | n | payload (opcode-specific) |

### 1.5 Send path
`Client.Sendpacket(string hex)` — hex → bytes → XOR 0xAD → `_socket.Send(...)`. Client.cs:8264-8279.
`Server.SendToClient(id, hex)` — same, sent to one client. Server.cs:543-562.
`Server.SendToAllClient(fromId, hex)` — every connected client except `fromId`. Server.cs:564-594.
`Server.SendToAllClientMapid(fromId, hex)` — every connected client on the same map as `fromId`, except `fromId`. Server.cs:596-626.
`Server.SendToAllMapid(mapId, hex)` — every connected client on that map, including sender. Server.cs:628-658.

### 1.6 String→hex conversion (`smethod_13`)
Per character: `Strings.AscW(ch).ToString("X2")` → the **low byte** of the UTF-16 code unit. ASCII text round-trips exactly; Vietnamese accented chars (as used in server-sent names/messages) are lossy — the C# server applies `Class5.smethod_17` (Class5.cs:420-462) to some strings ("TSVN", welcome banner) to pre-map Vietnamese chars. A Rust port must replicate the exact byte produced by `AscW(...) & 0xFF` (i.e., `ch as u16 & 0xFF`).

---

## 2. CLIENT → SERVER OPCODES (dispatch table)

Dispatch: `UpdateMainGrid_Recv(byte[] packet)` switches on `packet[4]`. Client.cs:859-957.
All handlers are wrapped in an empty `try/catch` (Client.cs:954-956) — any exception is silently swallowed and the socket stays open. Packets with unknown opcode are silently ignored.

| Opcode | Handler | Line |
|--------|---------|------|
| 0x00 | `Update_H0` | Client.cs:959 |
| 0x01 | `Update_H1` | Client.cs:967 |
| 0x02 | `Update_H2` | Client.cs:1029 |
| 0x03 | `Update_H3` | Client.cs:1047 |
| 0x06 | `Update_H6` | Client.cs:1055 |
| 0x08 | `Update_H8` | Client.cs:1064 |
| 0x09 | `Update_H9` | Client.cs:1144 |
| 0x0B | `Update_HB` | Client.cs:1241 |
| 0x0C | `Update_HC` | Client.cs:1439 |
| 0x0D | `Update_HD` | Client.cs:1458 |
| 0x0F | `Update_HF` | Client.cs:1774 |
| 0x13 | `Update_H13` | Client.cs:1926 |
| 0x14 | `Update_H14` | Client.cs:2076 |
| 0x17 | `Update_H17` | Client.cs:2101 |
| 0x19 | `Update_H19` | Client.cs:6099 |
| 0x1B | `Update_H1B` | Client.cs:6428 |
| 0x1C | `Update_H1C` | Client.cs:7132 |
| 0x1D | `Update_H1D` | Client.cs:7314 |
| 0x1E | `Update_H1E` | Client.cs:5884 |
| 0x1F | `Update_H1F` | Client.cs:6002 |
| 0x20 | `Update_H20` | Client.cs:7327 |
| 0x21 | `Update_H21` | Client.cs:7349 |
| 0x22 | `Update_H22` | Client.cs:7382 |
| 0x23 | `Update_H23` | Client.cs:7392 |
| 0x28 | `Update_H28` | Client.cs:7669 |
| 0x2C | `RebornPet` | Client.cs:9860 |
| 0x32 | `Update_H32` | Client.cs:7691 |
| 0x41 | `Update_H41` | Client.cs:7852 |
| 0x42 | `Update_H42` | Client.cs:7870 |

NOT handled (silently ignored, no socket action): 0x04, 0x05, 0x07, 0x0A, 0x0E, 0x10, 0x12, 0x15, 0x16, 0x18, 0x1A, 0x24..0x27, 0x29..0x2B, 0x2D..0x31, 0x33..0x40, 0x43..0xC7. `ClientBattle.cs` is an empty stub (ClientBattle.cs:3-7).

---

### 2.1 Opcode 0x00 — Hello (Client.cs:959-965)

Exact payload match: the whole frame must equal hex `F444010000` (opcode 0x00, **no sub byte**, length 1). If so, reply `F4440300010901`. Any other payload → silently ignored (no reply, no close).

### 2.2 Opcode 0x01 — Auth/Login (Client.cs:967-1027)

| Offset | Size | Field |
|--------|------|-------|
| 5 | 1 | sub (ignored, conventionally 0x00) |
| 6-9 | 4 | account ID, LE u32 |
| 10-11 | 2 | server prefix ASCII, must equal `Server.IDPrefix` = `"vn"` (case-insensitive compare, Server.cs:53); mismatch → silent return, connection stays open |
| 12-13 | 2 | client version, LE u16, must be >= 186 (`int_0=186`, Client.cs:383); below → `shutdown()` |
| 14..end | n | password bytes, each byte `Strings.Chr(packet[i])` concatenated (plain ASCII text) |

Server logic order (Client.cs:996-1025):
1. version check → too low = `shutdown()`
2. `Data.MemberGetIdExits(id)` — account key exists in `Data/Member.ini` section `[Account]` (value `pass1\tpass2`); missing → `shutdown()` (Client.cs:1019)
3. password equals `MemberGetData(id,"pass1")` (first column before tab); wrong → `WrongPass()` = send `F44402000106`, keep connection open (Client.cs:1014, 9551-9554)
4. double-login guard `Server.Clients.ContainsKey(id)` → `shutdown()` (Client.cs:1002-1009)
5. success → `_My_Id = id; Logined()` (Client.cs:1004-1005)

### 2.3 Opcode 0x02 — Chat (Client.cs:1029-1045)

| Sub | Handler | Line | Behaviour |
|-----|---------|------|-----------|
| 2 | `FChat.H2` | FChat.cs:11-451 | Global or map chat; message = `packet[6..]` ASCII bytes. Text > 60 chars → silently dropped. Admin (`_My_Id < 300012`, Client.cs:10163-10170) slash-commands: `/additem ID[,count]`, `/addpet ID`, `/addskpoint N`, `/where`, `/warp mapid`, `/test N`, `/reloadtalks`, `/battle N`, `/packet ...`, `/sendpacket HEX`, `/endtalk`, `/loadnpcs`, `/loaditems`, `/loadscenes`; all clients: `/sleep`, `/openhotel`, `/openstore`, `/openbank`. Normal broadcast: if `Trangbi slot 6 ID == 23100` → `Toan` (all clients, opcode 0x02 sub 0x01), else `Gan` (same map, opcode 0x02 sub 0x02) |
| 3 | `FChat.H3` | FChat.cs:453-469 | Whisper. Target ID = `packet[6-9]` LE u32, message = `packet[10..]` ASCII. Text > 60 → dropped. Both sender and recipient receive opcode 0x02 sub 0x03 packet with the **recipient** ID |
| 4 | (empty) | Client.cs:1042-1043 | no-op |
| 5 | `FChat.H5` | FChat.cs:471-529 | Party chat, message `packet[6..]` ASCII, > 60 dropped. Sent to leader + mem1..mem4 via `Server.SendToClient` (opcode 0x02 sub 0x05, sender ID embedded) |

Chat packet wire forms (FChat.cs:484-529):
- Toan: `F444` + len(`6+chatlen`) + `0201` + `smethod_12(sender)` + chathex
- Gan:  `F444` + len(`6+chatlen`) + `0202` + `smethod_12(sender)` + chathex
- ThiTham: `F444` + len(`6+chatlen`) + `0203` + `smethod_12(recipient)` + chathex
- Doi:  `F444` + len(`6+chatlen`) + `0205` + `smethod_12(sender)` + chathex

### 2.4 Opcode 0x03 — Enter-game confirm (Client.cs:1047-1053)

Exact match whole frame `F44402000301` → `Logined()`. If already logged in, `Server.Clients.Add` throws (duplicate key) and the try/catch swallows it → packet is a no-op. If not yet authed (`_My_Id==0`), `Logined()` falls to `CreatChar()` → `F4440300010300`.

### 2.5 Opcode 0x06 — Move (Client.cs:1055-1062)

Sub 1 → `FWalk.H1` (FWalk.cs:5-70). Layout:
| Offset | Field |
|--------|-------|
| 6 | gocnhin (view dir) |
| 7-8 | x, LE u16 |
| 9-10 | y, LE u16 |
| 11-12 | (unused, read and discarded) |

If `_My_IdBattle > 0` → ignore. If party leader (`_My_IdLeader == _My_Id`), also moves each online member (`Walked` each). `Walked(id,x,y,gocnhin)` (Client.cs:9543-9549) broadcasts `F4440B000601` + `smethod_12(id)` + `gocnhin(1B)` + `smethod_11(x)` + `smethod_11(y)` to all clients on the same map.

### 2.6 Opcode 0x08 — Stat point allocation (Client.cs:1064-1141)

Sub must be 1 and not in battle (`_My_IdBattle == 0`). `packet[8]` = stat id, `packet[9]` = points to spend. Requires `_My_Point >= packet[9]` (and `> 0`). Stat id map → DB column + reply via `Data.PlayerUpdateDataId` (which emits opcode 0x08 sub 0x01 stat packets):
- 25 → Hpmax (recompute `getHpMax(reborn, job, lv, Hpx+n) + Hpx2`)
- 26 → Spmax (recompute)
- 27 → Int (cap 400)
- 28 → Atk (cap 400)
- 29 → Def (cap 400)
- 30 → Agi (cap 400)
- 31 → Hpx (cap 400; also Hpmax recompute)
- 32 → Spx (cap 400; also Spmax recompute)

### 2.7 Opcode 0x09 — Create character (Client.cs:1144-1239)

Sub 1 — create (Client.cs:1150-1213):
| Offset | Field |
|--------|-------|
| 6 | sex |
| 7 | (unused, 0) |
| 8 | hair |
| 9 | thuoctinh |
| 10-17 | color, 8 raw bytes converted to hex (not per-char string) |
| 18..24 | int, atk, def, hpx, spx, agi, (unused) — each 1 byte |
| 25 | name length |
| 26.. | name ASCII |
| after name | pass1 length | pass1 bytes ... (lengths are single bytes before each string; text2 = second password) |

Actions: copy `CSDL/NewChar.accdb` → `member/vn{id}.accdb` (Client.cs:1153-1154), `INSERT INTO Player` (1200-1202), `MemberChangedPass` (1203), reply `F44402000901` (`CreatedCharacter`). On any exception → `shutdown()`.

Sub 2 — name check (Client.cs:1214-1236): `packet[6..]` ASCII = candidate name; `SELECT * FROM Player WHERE Name='...'`. Exists → `F4440300090301`; free → `F4440300090300` and stash name in `string_1`.

### 2.8 Opcode 0x0B — Battle control (Client.cs:1241-1437)

Sub 1 — leave battle. Requires `packet[6] == 3`. Clears queue slot, `_My_IdBattle = 0`, sends `F44408000B00` + `smethod_12(_My_Id)` + `0000`. (Client.cs:1245-1274)

Sub 2 — PK challenge. Inner sub `packet[6]`:
- 2: target = `packet[7-10]` LE u32; target must exist online; requires own `_My_IdBattle==0`, own `_My_Pk==1`, target not in battle. If target Pk==0 → `F4440300210101`; if Pk==1 → `new TheBattle(_My_Id, target, 112)`. (Client.cs:1278-1304)
- 3: attack NPC. `_My_IdBattle==0`; npc id = `packet[7-10]` LE u32; if npc id is in range 20000..22000 / 23000..25000 / 26000..27000 → blocked (guard ranges); else `new TheBattle(_My_Id, npcId, idNpcOnMap, 112)` where `idNpcOnMap` = `packet[11-12]` LE u16. (Client.cs:1306-1325)
- 4: join battle. target = `packet[7-10]`; target must be in battle and requester must not be. Finds first free slot in `ListQS`, registers, sets `_My_IdBattle`, builds battle-join packet (see §3 battle packets) then sends it plus `F44403000B0A01`. (Client.cs:1327-1412)
- 5: `ClientBattle.JamPlayerToBattle(this, target, packet)` — stub module, no-op. (Client.cs:1414-1426)

Sub 3/4/5 — empty cases (no-op). (Client.cs:1429-1432)

Sub 6 — broadcast `F44406000B06` + `smethod_12(_My_Id)` to map. (Client.cs:1433-1435)

### 2.9 Opcode 0x0C — Teleport confirm (Client.cs:1439-1456)

Sub 1: if party leader exists and is not self → send `F44402000504` + `F44402001408` and return. Otherwise `warpfinish = false`, send the same two packets, reset `talkcount`/`idtalking`.

### 2.10 Opcode 0x0D — Party (Client.cs:1458-1772)

Sub 1 — request to join: target = `packet[6-9]` LE u32, must be != self and online. Sets `_My_IdXinVaoNhom`/`_My_IdMoiVaoNhom`, sends target `F44406000D09` + `smethod_12(self)`. (1462-1478)

Sub 2 — empty. (1769-1770)

Sub 3 — accept join request: `packet[6]` = accept flag (1), `packet[7-10]` = requester id. Guards: requester == self → ignore; flag==1 and requester == `_My_IdMoiVaoNhom` → `Walked(requester,...)`, join into leader's member slots, broadcast `F4440A000D05` + leader + member pairs, send `F44407000D0301` + `smethod_12(member)` to self and requester, `PartySendStatus` both ways, sync all member structs. (1480-1576)

Sub 4 — leave/disband: `packet[6-9]` = id → `GiaiTanParty(id)` (Client.cs:9320-9533).

Sub 5 — change leader: `packet[6-9]` = new leader; only current leader may act; sets `_My_IdQS`, sends `F44406000D08`/`F44406000D07`/`F44406000D0B` + new leader id to self and map. (1590-1638)

Sub 6 — QS quit: if `_My_IdQS > 0` send `F44406000D08`/`F44406000D0C` + qs id to self+map, clear QS. (1640-1649)

Sub 7 — invite to group: target = `packet[6-9]`, != self, online; sets `_My_IdXinVaoNhom`/`_My_IdMoiVaoNhom`, sends target `F44406000D01` + `smethod_12(self)`. (1650-1666)

Sub 8 — accept invite: `packet[6]`=flag(1), `packet[7-10]`=inviter. Guards inviter==self → ignore; flag==1 and inviter==`_My_IdXinVaoNhom` → `Walked(inviter,...)`, require self to have leader slot; set leader + member slots both sides, send `F4440A000D05` + self + member, `PartySendStatus`, `F44407000D0301` to inviter. (1668-1767)

### 2.11 Opcode 0x0F — Pet actions (Client.cs:1774-1923)

Sub 2 — release pet: `packet[6]` = stt. If it's the active pet (`_My_SttPetXuatChien`) → clear flag. `Data.Removepet` → broadcast `F44407000F02` + `smethod_12(self)` + stt to map. (1780-1788)

Sub 3 — store pet to stable: `packet[6]` = stt (battle slot 1-4). Find first empty stable slot (5-8) via `SwitchPet(stt+4, slot)`. On success: `Data.SendStatusPet`, reply `F44405001F06` + stt + `0000`, `UpdateStatusPetWhenUseItem`, map-broadcast `F4440C000F01` + self + stt + pet id (LE4) + `01`; finally `F44402001F0C`. (1790-1812)

Sub 7 — take pet from stable: `packet[6]` = stable index (1-4 → row 5-8). If pet is active → red message + `F44402001F09` and stop. Finds first empty battle slot (5..10), `SwitchPet(packet[6], slot)`. Sends `F444` + len(`nameLen+9`) + `1F06` + `(slot-4)` + `smethod_11(petid)` + `lv` + `smethod_11(hp)` + `nameLen` + name hex; map-broadcast `F44407000F02` + self + `packet[6]`; then `F44402001F09`. (1814-1850)

Sub 8 — swap active pet: `packet[6]` = stable slot index, `packet[7]` = active stt to replace. If that stt is fighting → red msg + `F44402001F09F44402001F0C`. Else `SwitchPet(packet[6]+4, packet[7])`, send pet summary frame (like sub 7), `SendStatusPet`, `UpdateStatusPetWhenUseItem`, map-broadcast `F4440C000F01` + self + stt + new pet id + `00`, then `F44402001F09F44402001F0C`. (1852-1878)

Sub 4 — mount horse: `packet[6-9]` = pet id; requires pet id in range 18000..19000, != current active pet id, and `PetExitsMangtheo`. Sets `_My_Horse`, sends `F4440E000F05` + self + pet id + `00000000` to self and all clients. (1880-1898)

Sub 5 — unmount: if `_My_Horse > 0`, clear it, send `F44406000F06` + `smethod_12(self)` to self + all clients. (1899-1906)

Sub 6 — rename pet: `packet[6]` = stt, `packet[7..]` = new name ASCII. `Data.ChangeNamePet` → map-broadcast `F444` + len + `0F09` + `smethod_12(self)` + stt + namehex. (1907-1919)

### 2.12 Opcode 0x13 — Pet summon/recall (+ in-battle) (Client.cs:1926-2074)

Out of battle (`_My_IdBattle == 0`):
- Sub 1 — summon: `packet[6-9]` = pet id LE u32. If not already riding, get `PetGetStt`, require stt <= 4, set `SttPetXuatChien`, send `F44406001301` + `smethod_12(petid)`. (1932-1951)
- Sub 2 — recall: if active pet exists, clear flag, send `F44402001302`. (1952-1962)

In battle (`_My_IdBattle > 0`): find own `WarInfo` with `_Attacked==0`; sub 1 loads the pet row from DB, removes player from battle grid (row XOR 1), sends `F44404003505`+row+col, `F44404000B01`+(row^1)+col, spawns pet unit via `ChangedWar` type 4, broadcasts `F4441A000B0505`+warPacket, sends `F44406001301`+petid; sub 2 recalls pet similarly and sends `F44402001302`. (1980-2068)

### 2.13 Opcode 0x14 — Action/Talk (Client.cs:2076-2099)

| Sub | Handler | Line |
|-----|---------|------|
| 1 | `FTalk.H1` | FTalk.cs:10-261 |
| 4 | `FTalk.H4` (→ `EndTalk`) | FTalk.cs:263-266 |
| 6 | `FTalk.H6` | FTalk.cs:268-3241 |
| 8 | `FTalk.H8` (warp talk) | FTalk.cs:3243-3386 |
| 9 | `FTalk.H9` (set `SelectMenu = packet[6]`) | FTalk.cs:3388-3391 |
| default | `EndTalk()` = `F44402001408` + reset talkcount/idtalking/SelectMenu | Client.cs:7919-7925 |

`FTalk.H1` — begin NPC talk. `packet[6-7]` = map object id (LE u16); sets `Typetalk="NPC"`, resolves `idnpctalking` from NpcOnMap. Distance gate ±150 on x/y. Specials:
- npc 16080/16004/16011/16015: `F44402000602` + `F44411001401000000010603` + idtalking(2B) + `0000000000000100`
- npc 15002/16001/16016: `F44402000602` + `F44411001401000000010603` + idtalking(2B) + `0000000000000200`
- npc 16012: silent return.
- npc 20001: quest-check → `F44411001401000000020103` + idtalking + `00000000000000BB` (quest not ready) or `F444110014010000000101070000000000000077A7`; else `BattleQuestWin` + if no talk data `F44411001401000000010103` + idtalking + `000000000000C830`.
- If talk data exists → `F44402000602` then `TalkMessages(...)` (splits dialog string on `F444`, sends each fragment, 500 ms apart; fragment `F44402001408` also sets SelectMenu=40). If talk count == 0 and team-def present → start battle. (FTalk.cs:77-261)

`FTalk.H6` — menu/continue handler (huge data-driven engine). Handles:
- bank/store NPCs (16080/16004/16011/16023) SelectMenu 30 → `F44403001D0900` + `F44406001D04`+bankgold + `F44402001D05` + `F44402001409`; 31 → `F44402001D06` + `F44402001409`; 40 → EndTalk. (302-329)
- inn/hotel NPCs (15002/16001/16016/15118) SelectMenu 30 → `F44411001401000000010603010000000000000100`; 31 → `Sleep()`+EndTalk; 32 → `OpenHotel()`; 33 → set savemap + add item 46016×2 + EndTalk; 40 → EndTalk. (331-353)
- npc 16015: SelectMenu 30 → `F44411001401000000010603010000000000000200`; 31 → Sleep+EndTalk+`method_2(10)`; 32 → OpenHotel; 33 → savemap; 40 → EndTalk. (356-381)
- npc 16012: silent. (382-383)
- generic data-driven path (2640-2820): post-battle win/lose dialogs; `GetDataTalkExits` miss → EndTalk; `talkcount++`; if next dialog starts with `F444110014010000000106` and `_RequireSelectMenu` mismatched → `LoseDialogs[0]` or EndTalk; quest-required check fail → `F4441100140100000001010700000000000000493C` or `F44411001401000000020103`+id+`00000000000000BB`; else `TalkMessages(dialog)`; when exhausted, team-def → battle, else special NPC branches (59411, etc.), each sending `F444110014...` dialog frames and item/gold/pet rewards.
- hundreds of hardcoded NPC branches (idnpctalking-based) sending `F44411001401...`/`F44411001404...` dialog frames and reward packets (gold: `F4440A001A04`+gold+`00000000`; skill learn: `F4440C0008016E01`+lv+skillid; equip: `F44403001711`+slot / `F44404001717`+stt+slot).

`FTalk.H8` — warp talk. `packet[6-7]` = warp id. `Typetalk="WARP"`. Talk-data → dialogs / `BattleQuestWin` / `F4441100140100000001010700000000000000493C` on requirement fail. Else `GetDataWarp` → `Data.Warped` (party warp if leader). `Data_BattleGates` entry → battle (diaHinh from gate, 10 monsters). Map 59841 special → quest-gated `F44411001401000000010607000000000000000300`, `F444060016030A000000`, or battle. (FTalk.cs:3243-3386)

### 2.14 Opcode 0x17 — Inventory / items (Client.cs:2101-5882) — sub-opcode dispatch

| Sub | Meaning | Lines | Notes |
|-----|---------|-------|-------|
| 2 | pick up map drop | 2107-2135 | `packet[6]` = drop slot; distance gate ±150; `Data.PickupItemOnMap`; delay 999999 handling |
| 3 | drop item | 2136-2178 | `packet[6]`=homdo slot, `packet[7]`=count; `Data.HomdoDropItem` |
| 10 | move/stack item | 2179-2186 | `packet[6]`=oldslot, `packet[7]`=count, `packet[8]`=newslot; `Data.HomdoMoveItem` echoes whole raw packet back verbatim on success |
| 11 | equip player item | 2187-2204 | `packet[6]`=homdo slot; loai 1..6, `_My_Lv >= lv`; reply `F44403001711`+slot; `HomdoUseItemTB`; `UpdateStatusWhenUseItem`; `ServerSend_EquitItem` |
| 12 | unequip player | 2205-2219 | `packet[6]`=trangbi slot, `packet[7]`=homdo dest slot (must be empty); reply `F44404001710`+slot+dest; `ServerSend_UnEquitItem` |
| 14 | compound/craft (gems) | 2220-3799 | `packet[6..11]` = gem1 slot+count, gem2 slot+count, result slot+count; several recipes (loai/level gates + `44043` "magic stone" gates) each with a big RNG item table; sends `F44404001709`+slot+count ×3, `F4440E001708`+..., `F4440600170D`+... ; requires `GiatriLong < 20` for gem slots |
| 15 | use item | 3801-5361 | `packet[6]`=homdo slot, `packet[7]`=count, `packet[8]`=target (0=player, 1-4=pet). Huge item-ID dispatch: warps (e.g. 46016→savemap 410,510 …), add-pet items (`_AddPet>10000`), sleep item 46167 (leader only), lucky-box rolls (99999, 46129, 46627, …), stat books (46185-46190 → pet stats; 46240 → Hpx+1; 46238 → Spx2/Spmax/+tanthu), HP/SP store items (26456/26457/46145/46146), doll summon 48001-48097/48101 (`F44408000505`+self+`npcid`+`F444040017091301F4440200170F`), God books (46169), Texp books (46211-46219, Lv<=200), skill books (46132-46136 learn 8 skills; 46230-46233/46246 → learn skill + `F4440C0008016E01`+lv+skillid), reborn items (46170 → RESET Lv=1/Reborn=1 then close socket; 46247-46250 → reborn 2 with job), point books (50010 → Point+1; 50011 → SkillPoint+1), gold/FAI items (46092 → `F44404000B0702FF`+`F44404001709`+slot+count+`F4440200170F`; 46041/46093 → `F44404000B09FF01`+…), HP/SP/FAI potions (uses `_Hp*_Sp*_Fai1`), party buffs 46048/46049/46173/46174 (`F44407001737`+self+rand), 46018 (`F44402001726F44404001709`+slot+`00F4440200170F`), 46089/46179 (`F444040017090D01F4440B001748`+self+`0000271000`), 46953 (random gem), shop items 46015/46013/46014/46091/46042 (no-op). Always ends with `F44404001709`+slot+count+`F4440200170F` unless an explicit branch returned |
| 17 | equip pet item | 5362-5381 | `packet[6]`=pet stt, `packet[7]`=homdo slot; pet level >= item lv, loai 1..6, pet not fighting; reply `F44404001717`+stt+slot; `HomdoUseItemTB_Pet` |
| 18 | unequip pet | 5382-5398 | `packet[6]`=stt, `packet[7]`=trangbi slot, `packet[8]`=dest homdo slot (must be empty); reply `F44405001716`+stt+slot1+slot2 |
| 30 | open player shop | 5399-5437 | `packet[6]`=name length, name = `packet[7..]` ASCII (reversed build but result is normal), then pairs [slot, price(4B LE)] 5 bytes each to end of packet; reply `F444`+len+`171E`+nameLen+name+items; map-broadcast `F444`+len+`171F`+`smethod_12(self)`+name |
| 31 | close player shop | 5438-5448 | map-broadcast + self `F444`+len+`1720`+`smethod_12(self)`; clears shop state |
| 32 | open someone's shop | 5449-5464 | `packet[6-9]`=owner id; `OpenPlayerShop` → `F444`+len+`1721`+`0000000000000000000000000000000000`+items |
| 33 | buy from shop | 5466-5545 | `packet[10]`=item index, `packet[11]`=count. Validates owner online/shop open, buyer slot space and gold. Replies: seller/buyer gold `F4440A001A04`+gold+`00000000`, equipment buys → `F444`+len+`1706`+id+count+`0000`+long+(100+giatriLong)+khang+texp(4B) |
| 36 | homdo → tuideo | 5547-5603 | requires pet 22029/41187/18023; `packet[6..]` = slot list (n = `packet[2]-3` entries); reply `F44404001709`+slot+`32` per move + `F444`+len+`172F`+tuideo items |
| 37 | tuideo → homdo | 5604-5665 | same guard; reply `F4440E001708`+items per move + `F44404001731`+slot+`32` + `F44402001732` |
| 46 | reborn (job change) | 5666-5752 | if any Trangbi slot <=6 has item → `F44411001401000000010103`+idtalking+`00000000002451` (refuse). Else `packet[6]`=hair, `packet[7-14]`=color hex; UPDATE Player (rebirth formulas), `DELETE FROM Skill...`, reply `F44402002C01`, quest step update, `F4441100140100000001010302000000000000F476`, sleep 2000, close DB + socket |
| 48 | warp finish ack | 5754-5760 | send `F44402000504` + `F44402001408`; `warpfinish=true`, reset talk |
| 51 | homdo → luulang | 5761-5817 | requires pet 41187/18023; reply `F44404001709`+slot+`32` per move + `F444`+len+`1766`+items |
| 52 | luulang → homdo | 5818-5879 | requires pet 41187/18023; reply `F4440E001708`+items + `F44404001768`+slot+`32` + `F44402001732` |

### 2.15 Opcode 0x19 — Trade (Client.cs:6099-6426)

- Sub 1 — open trade: `packet[6-9]` = partner id; both sides get `F44406001901` + `smethod_12(other)`; sets `_Trader_Id` both ways. (6106-6123)
- Sub 2 — set gold+items: `packet[6-9]` gold LE u32, `packet[10..]` homdo slot list. Sends partner `F444`+len+`1903`+goldhex+per-item(id LE2,count,doben,long,(100+giatriLong),khang,texp LE4). (6125-6156)
- Sub 3 — confirm/cancel: `packet[6]`=1 confirm, 2 cancel. Confirm: both must accept → `GoldTransfer`; item exchange (both directions) with `F444`+len+`1706`+item entries; insufficient slots → `F4440300190207` both, `TradeFinish`; success → `F4440300190204` both. Cancel → `F4440300190203` to partner, `F4440300190209` to self, `TradeFinish`. (6158-6218)
- Sub 10 — open pet trade: like sub 1 but `F4440600190A`. (6220-6238)
- Sub 11 — offer pet: `packet[6-9]` gold, `packet[10]` pet stt (>0). Pet data → 28-char padded name (pad char `6`), full pet block; sends partner `F444`+len+`190C`+... (6240-6287)
- Sub 12 — confirm/cancel pet trade: `packet[6]`=1/2. Confirm: `GoldTransfer`; pet dup check → `F4440300190B07`; no pet slot → `F4440300190B0A`; success → `F4440300190B04` both. Cancel → `F4440300190B03` partner + `F4440300190B0F` self. (6290-6357)
- Sub 20 — mail/transfer item to player: `packet[10-13]` = recipient id, then 9 slot pairs (`packet[14..22]`, 2 bytes each: slot+count, 0 = skip). Moves items, recipient gets `F4440E001706`+item entries, sender re-sends full homdo `F444`+len+`1705`+all items. (6359-6422)

### 2.16 Opcode 0x1B — NPC shop buy / sell (Client.cs:6428-7130)

Context-dependent: uses `idtalking` (select menu), `_My_MapId`, `idnpctalking`, gold. Pattern per branch: check gold >= price → `Data.HomdoAddItem(id, itemId, 1)`, `PlayerUpdateDataId(Gold, gold-price)`, `Sendpacket("F4440A001A04"+smethod_12(gold)+"00000000")`, `SendRedMessage("Khách quan mua hàng thành công")`. Hundreds of hardcoded (map, menu) → (item, price) pairs (e.g. map 12223 menu 0 → item 26041 @5; map 12002 menu 0-14 → 20023/19723/19755/... @58800; map 19241; map 12201; map 12244; map 12007; map 12204; map 12990; map 12001 (item 27156 @115, 52015 @1); map 20001; map 11011; map 9999 free items 18001/27156/52015). Selling: if map>10000 and `idnpctalking` is 16005/99999 → scan items 26001..26455, sell each found (price = `packet[7]` count added to gold), reply `F4440A001A04`+gold+`00000000`; if `idnpctalking` 16002/99999 → scan 27001..27165.

### 2.17 Opcode 0x1C — Learn/upgrade skills (Client.cs:7132-7312)

- Sub 1 — player skills: sequence of triples `packet[6..]`: skill id (2B LE) + target level (1B). Validates LvMax/Reborn/`GetDKThuoctinh`/prereq skills/SkillPoint. Each success → `F4440C0008016E01` + `smethod_12(level)` + `smethod_12(skillid)`; ends with `SendSkillPointtoClient(remaining)` = `F4440C0008012501` + `smethod_12(count)` + `00000000`.
- Sub 2 — pet skills: `packet[6]`=stt, `packet[7-8]`=skill id LE u16, `packet[9]`=target level. Only upgrades existing pet skill slots 1-4, requires pet SkillPoint; reply `F4440F00080204` + `smethod_11(stt)` + `6E01` + `smethod_12(level)` + `smethod_12(skillid)`.

### 2.18 Opcode 0x1D — Bank gold (Client.cs:7314-7325)

Sub 1 → `FTienTrang.H1` (withdraw, FTienTrang.cs:5-26): `packet[6-9]` amount LE u32. Guards: bank balance >= amount and `gold + amount <= 9999999`. Reply `F44406001D02` + `smethod_12(amount)` and `F44406001A01` + `smethod_12(amount)`.
Sub 2 → `FTienTrang.H2` (deposit, FTienTrang.cs:28-49): guards `gold >= amount`, `bank + amount <= 9999999`. Reply `F44406001D01` + `smethod_12(amount)` and `F44406001A02` + `smethod_12(amount)`.

### 2.19 Opcode 0x1E — Storage transfer (Client.cs:5884-6000)

- Sub 1 — TienTrang → Homdo: `packet[6..]` slot list (count = `packet[2]-3`). Per move: `F4440E001708` + item detail (if item), and `F44404001E05` + slot + `32`; end `F44402001732`. (5889-5942)
- Sub 2 — Homdo → TienTrang: same list parsing; per move `F44404001709` + slot + `32`, then `F444`+len+`1E04` + item details. (5944-5994)
- Sub 8 — sets `SelectMenu = 40`. (5996-5999)

### 2.20 Opcode 0x1F — Pet stable menu (Client.cs:6002-6097)

Sub 2 — store to stable: `packet[6]` battle stt; find empty stable slot (5-8), `SwitchPet(stt+4, slot)`; reply `F44405001F06` + stt + `0000`; `SendStatusPet`; map-broadcast `F4440C000F01` + self + stt + petid + `01`; then `F44402001F0C`. (6006-6028)
Sub 3 — take from stable: like 0x0F sub 7 (guard active pet, find 5..10, `F444`+len+`1F06`+..., map `F44407000F02`, `F44402001F09`). (6030-6066)
Sub 4 — swap: like 0x0F sub 8. (6068-6095)

### 2.21 Opcode 0x20 — Expressions (Client.cs:7327-7347)

- Sub 1 — `packet[6]` action; map-broadcast `F44407002001` + self + action.
- Sub 2 — `packet[6]` → `_My_Dongtac`; map-broadcast `F44407002002` + self + action.
- Sub 3 — `_My_Dongtac = 0` (no packet).

### 2.22 Opcode 0x21 — PK / war mode (Client.cs:7349-7380)

- Sub 1 — `packet[6]`: 0 → reply `F4440400210200` + `_My_ThamChien`, set `Pk=0`; 1 → reply `F4440400210201` + `_My_ThamChien`, set `Pk=1`.
- Sub 2 — `packet[6]`: 0 → reply `F44404002102` + `_My_Pk` + `00`, set `ThamChien=0`; 1 → reply `F44404002102` + `_My_Pk` + `01`, set `ThamChien=1`.

### 2.23 Opcode 0x22 — Game points (Client.cs:7382-7389)

Sub 1 → `method_0(_My_Gold)` sends `F44412002304` + `smethod_12(gold)` + 24 zero bytes. (Client.cs:8249-8252)

### 2.24 Opcode 0x23 — Account management (Client.cs:7392-7667)

Sub 1 — change password. Layout: len1(1B) + pass1 + len2 + pass2 + len3 + newpass1 + len4 + newpass2 (4 length-prefixed ASCII strings). Compare pass1/pass2 with Member.ini. Replies: wrong old pass1 → `F4440300230102`; wrong old pass2 → `F4440300230103`; success → `MemberChangedPass` + `F4440300230101`. (7398-7445)

Sub 2 — delete character. len1+pass1, len2+pass2. Wrong pass1 → `F4440300230202`; wrong pass2 → `F4440300230203`; success: leave battle (broadcasts `F44404000B01`+row^1+col, `F44408000B00`+id+`0000`, `F44405000B01`+row+col+`00`, map `F44408000B00`+id+`0000`), `GiaiTanParty`, `ServerSend_PlayerOffline`, map `F44406000D04`+self, close DB + socket, delete `Player` row and member .accdb, remove from `Server.Clients`. (7447-7569)

Sub 3 — redeem item code: len1 + code + len2 + password (length-prefixed ASCII). Hardcoded `TSVN123`/`TSVN456` → special gift (item 46197 + 20711 + 19711 + 23549 + 11001, once, sets `tanthu`). Else `SELECT * FROM item_code WHERE code=... AND password=...` (MySQL); unused code → grant `item_id`×`count`; used → red message with owner. (7571-7664)

### 2.25 Opcode 0x28 — Hotkey/skillbar save (Client.cs:7669-7689)

Sub 1: `packet[7-8]` skill id LE u16, `packet[9]` = hotbar slot (1..10). `SkillSaveUpdateId(slot, skillid)`; no response packet.

### 2.26 Opcode 0x2C — Reborn pet (Client.cs:9860-10000)

`stt = int(hex(packet).substring(13))` — i.e. bytes 6-7 as hex → LE u16. Requires homdo slot whose `_RbPetFrom == stt` and `_RbPetTo` is a valid NPC (search slots 1..25). Recomputes pet to NPC template (level 1, skills from NPC, `_RbPetFrom`/`_RbPetTo` consumed, 30/60 level threshold for bonus points). Packets: map `F44407000F02` + self + stt; map `F4440C000F01` + self + stt + newpetid + `01`; `SendStatusPet`; self `F44406001301` + `smethod_12(newpetid)`; `F44402002C01`. Guards fail → silent return.

### 2.27 Opcode 0x32 — Battle commands (Client.cs:7691-7850)

Requires being in battle (`Server.TheBattles.ContainsKey(_My_IdBattle)`) and `packet.Length >= 12`.
- Sub 1 — skill attack: `packet[6]`=row, `packet[7]`=col, `packet[8]`=rowAttack, `packet[9]`=colAttack, `packet[10-11]`=skill id LE u16. Range-checks row 0..3, col 0..4. Finds `WarInfo` at (row,col), requires `_Id > 0` and `_Attacked == 0`. For type 2 (player) skill level from player `SkillGet`; for pets from active pet skill slots 1-4. Sets `_LvSKill/_RowAttack/_ColumnAttack/_IdSkill/_Attacked=1` and broadcasts `F44404003505` + row + col. The Battle engine consumes it on its turn. (7696-7781)
- Sub 2 — use item in battle: same row/col fields + `packet[10-11]` item id LE u16. If item id 26001..27165 → heal WarInfo + pet by item `_Hp`/`_Sp`, remove 1 item, `_Attacked=1`. Other items ignored (reads only). (7783-7847)

### 2.28 Opcode 0x41 — Rank (Client.cs:7852-7863)

Sub 1 → `F44402004101`; Sub 2 → `F44402004102`. Other subs ignored.

### 2.29 Opcode 0x42 — GM shop / points (Client.cs:7870-7925)

- Sub 1 — `ShoppingMall(packet)`: `packet[9-10]` item id LE u16, `packet[11-12]` price LE u16. If `_My_Shop_Point >= price` and free slot → `HomdoAddItem(id, 1)`, deduct points, then `Shoppoin`. (7887-7912)
- Sub 2 — no-op. (7878-7879)
- Sub 3 — `Shoppoin`: reply `F44406004202` + `smethod_12(_My_Shop_Point)` + `0100`. (7914-7917)

---

## 3. SERVER → CLIENT PACKETS

All frames use the §1.4 layout. `<...>` denotes computed fields.

### 3.1 Full Logined1 sequence (Client.cs:7927-8212) — sent on successful login

In order, exactly:

1. `F44402001408F4440300142100` (two frames: end-talk, then opcode 0x03 sub 0x14 sub-sub 0x21 data 00) — Client.cs:8059
2. Player self-appear — opcode 0x03 sub 0x03:
   `F444` + len + `03` + `smethod_12(_My_Id)` + sex(1B) + ghost(1B) + god(1B) + `smethod_11(_My_MapId)` + `smethod_11(MapX)` + `smethod_11(MapY)` + gocnhin(1B) + `smethod_11(_My_Hair)` + `_My_Color` (8 hex) + itemCount(1B = equipped count) + equipped item ids (each `smethod_11`) + `0000000005` + reborn(1B) + job(1B) + name hex (`smethod_13`). len = `33 + (equipHexLen/2) + nameLen`. Client.cs:8060
3. Stats — opcode 0x05 sub 0x03:
   `F444` + len(`skillsHexLen/2 + 113`) + `0503` + thuoctinh(1B) + `smethod_11(Hp)` + `smethod_11(Sp)` + `smethod_11(Int)` + `smethod_11(Atk)` + `smethod_11(Def)` + `smethod_11(Agi)` + `smethod_11(Hpx)` + `smethod_11(Spx)` + lv(1B) + `smethod_12(Texp)` + `smethod_11(SkillPoint)` + `smethod_11(Point)` + `smethod_12(Tiengtam)` + `smethod_11(HpMax)` + `smethod_11(SpMax)` + `smethod_12(Atk2)` + `smethod_12(Def2)` + `smethod_12(Int2)` + `smethod_12(Agi2)` + `smethod_12(Hpx2)` + `smethod_12(Spx2)` + literal `F401F401F401F401F401` + 90 zero bytes + skill list (`GetlistSkill` = `smethod_11(id)` + lv per learned skill). Client.cs:8061
4. `Server.ServerSend_PlayerOnline(...)` — opcode 0x03 sub 0x04 to all clients: same layout as #2 but with `000000000006` marker, len = `36 + equipHex/2 + nameLen`. Server.cs:76-104. Plus (in `SendPalyerOnline`) `Server.SendPalyerOnline(_My_Id)` (Server.cs:121-272) which sends self the appear packet (opcode 0x04) + pet block (opcode 0x0F sub 0x07) for every online player, battle flag `F4440A000B0402`+id+`000003`, emote/mount frames, and player-shop name frames.
5. `Data.SendStatusAllPet(_My_Id)` — Data.cs:2135-2210, single send:
   `F444`+len(`petStats/2+2`) + `0F08` + petStats +
   `F444`+len(`slots/2+2`) + `0F14` + slotPairs +
   `F44402000F0A` +
   `F44405000F12010000` + `F44405000F12020000` + `F44405000F12030000` + `F44405000F12040000` +
   `F44404000F130100`
   where each petStats entry = `stt(1B)` + `smethod_11(id)` + `smethod_12(Texp)` + lv(1B) + `smethod_11(Hp)` + `smethod_11(Sp)` + `smethod_11(Int)` + `smethod_11(Atk)` + `smethod_11(Def)` + `smethod_11(Agi)` + `smethod_11(Hpx)` + `smethod_11(Spx)` + `00` + fai(1B) + quest(1B) + `smethod_11(SkillPoint)` + nameLen(1B) + namehex + `LvSkill1..3`(1B each) + 6×(`smethod_12(equipped id)` + `000000000000`) + `00 00 00 00 00 00 00` + LvSkill4 + `00 00 00 00`. slotPairs = `stt + "0000"` per pet. (Data.cs:2195-2206)
6. `Server.SendAllParty(_My_Id)` — Server.cs:431-509: sends any party leader frames `F44406000D07`/`F44406000D0B` + `smethod_12(qs)` (to self + map) and party member list `F444`+len+`0D06`+`smethod_12(leader)`+memberCount+`smethod_12`×members.
7. Active pet summon: `F44406001301` + `smethod_12(petId)` if `1 <= _My_SttPetXuatChien <= 4`. Client.cs:8066-8069
8. `Data.UpdateStatusPetWhenUseItemLogin(_My_Id)` — computes pet stat bonuses from pet equipment (no packet directly). Client.cs:8070
9. PK/war state: `F44404002102` + `_My_Pk` + `_My_ThamChien`. Client.cs:8071
10. Inventory dumps, all in ONE `Sendpacket` (concatenated frames):
    - Homdo: `F444`+len(`2+items/2`)+`1705`+items (item = slot(1B)+`smethod_11(id)`+count+doben+long+(100+giatriLong)+khang+`smethod_12(texp)`)
    - TienTrang: `...`+`1E01`+items
    - Tuideo: `...`+`172F`+items
    - Luulang: `...`+`1766`+items
    Client.cs:8076-8160
11. Equipped items: `F444`+len+`170B`+6×(`smethod_11(id)`+doben+long+(100+giatriLong)+khang+`smethod_12(texp)`) for slots 1-6 with id>0. Client.cs:7984-8061, 8161
12. Gold: `F4440A001A04` + `smethod_12(_My_Gold)` + `00000000`. Client.cs:8162
13. Server name: `F444`+len(`nameLen+11`) + `2709` + `smethod_12(_My_Id)` + `C4000000` + nameLen + `smethod_13(smethod_17("TSVN"))`. Client.cs:8163-8164
14. `F44402000504F44402000F0A` (two frames). Client.cs:8165
15. `F4440A000B0B0000000000002040`. Client.cs:8166
16. `F44402001F0F`. Client.cs:8167
17. `Sendpacket("")` — empty, sends nothing. Client.cs:8168
18. Time banner: `F444`+len(`6+msgLen`)+`020B00000000`+`smethod_13("Thời gian: yyyy-MM-dd H:mm:ss")`. Client.cs:8169-8170
19. Welcome banner: same template with `smethod_17("TS offline RebuildVN Thanks: Duong Van Truong && Somchai choosawai")`. Client.cs:8171-8172
20. Skill hotbar save: `F444`+len+`2801`+`02`+`smethod_11(skillId)`+slot(1B) per non-empty SkillSave row (slots 1..10). Client.cs:8173-8188
21. God/HP_Store/SP_Store: three × `F44412002304` + `smethod_12(value)` + 24 zeros (`method_0`). Client.cs:8190-8192, 8249-8252
22. DB cleanup; `_My_Logined = 1`. Client.cs:8210

### 3.2 Server→Client packet catalogue (grouped by opcode)

**Opcode 0x01 (login responses)**
- `F4440300010901` — hello reply (Client.cs:963)
- `F44402000106` — wrong password (Client.cs:9553)
- `F4440300010300` — no character, go to creation (Client.cs:9558)

**Opcode 0x02 (chat/misc UI)**
- `F44402000504` — stop movement / warp-prep, used in warp & sleep flows (Client.cs:1446, 1451, 5555?/5755, FTalk 274-275)
- `F44402001408` — end talk (Client.cs:7921)
- `F44402001409` — continue talk after bank/store open (Client.cs:10058, 10070)
- `F44402001407` — warp start (Data.cs:3935)
- `F44402000F0A` — pet UI frame terminator (Data.cs:2206/2277)
- `F44402000602` — talk-open (FTalk.cs:82, 95, 130, 158, 188)
- `F44402001F0A` — sleep broadcast (Client.cs:652, 692, 732, 772, 814)
- `F44403001F0100` — sleep done (Client.cs:685, 728, 768, 808, 850)
- `F44402001F09` — pet can't stay in stable (Client.cs:1822, 1849, 6038, 6065, 6076, 6093)
- `F44402001F0C` — pet stable menu close (Client.cs:1811, 6027, 6076, 6093)
- `F44402001F07` — hotel UI close (Client.cs:10032)
- `F44402001F0F` — (login sequence frame, Client.cs:8167)
- `F4440200170F` — use-item end marker (Client.cs:3653/3658/5358, Data.cs:3653)
- `F44402001726` — item 46018 special (Client.cs:4735)
- `F44402001732` — storage transfer end (Client.cs:5662, 5876, 5940)
- `F44402002C01` — reborn complete (Client.cs:5729, 9998)
- `F44402004101` / `F44402004102` — rank replies (Client.cs:7857, 7860)
- `F44402001302` — pet recalled (Client.cs:1959, 2061)
- `F44402001B03` — (FTalk.cs:860, 873, 886)
- `F44403001D0900` — open bank UI (Client.cs:10055)
- `F44402001D05` — bank UI gold (Client.cs:10057)
- `F44402001D06` — open store UI (Client.cs:10063)

**Opcode 0x03 (char/game state)**
- `F4440300142100` — (paired with 0x02 0x14 0x08, Client.cs:8059)
- `F4440300010901`/`F4440300010300` — see 0x01
- `F4440300090300` — name available (Client.cs:9563)
- `F4440300090301` — name taken (Client.cs:9568)
- `F44403001B0102` — inventory full (Data.cs:3236, 3244)
- `F44403001711`+slot — equipment equipped (Client.cs:2198, Data.cs:5883)
- `F4440300210101` — PK challenge target response (Client.cs:1297)
- `F4440300190203/04/07/09` — trade gold results (Client.cs:6176-6177, 6209-6210, 6215-6216)
- `F4440300190B03/04/07/0A/0F` — pet trade results (Client.cs:6310-6311, 6318-6319, 6331-6332, 6339-6340, 6348-6349, 6354-6355)
- `F44403000B0A01` — battle end marker (TheBattle.cs:684, 950, 5047; Client.cs:1405)
- `F4440300230101/02/03` — password change success/wrong1/wrong2 (Client.cs:7434, 7438, 7443)
- `F4440300230202/03` — delete-char wrong pass1/pass2 (Client.cs:7467, 7471)
- `F44403001D0900` — bank UI open (Client.cs:10055)

**Opcode 0x04 (appear/misc)**
- Player appear frame (opcode 0x04) built in `Server.SendPalyerOnline` (Server.cs:177)
- `F4440400210200/01`+thamchien — PK set reply (Client.cs:7357, 7361)
- `F44404002102`+pk+`00/01` — war-mode set reply (Client.cs:7370, 7374)
- `F44404001709`+slot+count — item decrement/consume (Data.cs:3419, 3437, 3565, 3653, 3658; Client.cs:3984, 4015, 5586, 5800, 5976)
- `F44404001702`+slot — map drop removed (Data.cs:3800, 3848, 3865, 5445)
- `F44404001710`+slot+dest — unequip player (Client.cs:2213)
- `F44404001717`+stt+slot — pet equip (Client.cs:5375, Data.cs:5893)
- `F44404003505`+row+col — battle attack marker (Client.cs:2020, 2052, 3114-3115, TheBattle.cs:1235, 3648)
- `F44404000B01`+row+col — battle grid move/clear (Client.cs:2021, 2053, 7505, 7510, 7522, TheBattle.cs:3120, 3132)
- `F44404000B0702FF`+`F44404001709`+slot+count+`F4440200170F` — gold item 46092 use (Client.cs:5181-5184)
- `F44404000B09FF01`+`F44404001709`+slot+count+`F4440200170F` — FAI item 46041/46093 use (Client.cs:5337-5340)
- `F44405001716`+stt+slot1+slot2 — pet unequip (Client.cs:5392)

**Opcode 0x05 (stats/appearance updates)**
- `F44405001F06`+stt+`0000` — pet stored to stable (Client.cs:1807, 6023)
- `F44405000F12010000`..`00040000` — pet slot list (Data.cs:2206, 2277)
- `F44405001805020000`, `F44405001805620100`, `F44405001805910100` — party disband UI (Client.cs:9333-9335, 9430-9432, 9522-9524)
- `F44405001702`+slot+`01` — drop pickup (Data.cs:3863)
- `F44405001707`+id+count — item removed (Data.cs:3364, 3372, 3399, 3402, 3408)
- `F44405000B01`+row+col+`00` — battle cell cleared (Client.cs:7511, 7523, TheBattle.cs:3120, 3132)
- `F44405001716` — pet unequip (see 0x04)

**Opcode 0x06 (movement/warp/pets)**
- `F44406001301`+`smethod_12(petid)` — pet summoned/active (Client.cs:1947, 2040, 8068, 9997)
- `F44406001A01`+`smethod_12(amount)` — gold received (FTienTrang.cs:23)
- `F44406001A02`+`smethod_12(amount)` — gold sent (FTienTrang.cs:46)
- `F44406001D01`+`smethod_12(amount)` — bank deposit ack (FTienTrang.cs:45)
- `F44406001D02`+`smethod_12(amount)` — bank withdraw ack (FTienTrang.cs:22)
- `F44406001D04`+`smethod_12(bankgold)` — bank gold display (Client.cs:10056, FTalk.cs:308)
- `F44406004202`+`smethod_12(points)`+`0100` — shop points display (Client.cs:7916)
- `F44406001A02`+gold+12 zeros — `method_2` (Client.cs:8261)
- `F44406000F06`+`smethod_12(id)` — horse dismount (Client.cs:1903-1904)
- `F44406000D01`+`smethod_12(self)` — party invite (to invited) (Client.cs:1661)
- `F44406000D04`+`smethod_12(id)` — player left party (Client.cs:7548, 9336, 9433-9435, 9525, 9527)
- `F44406000D07`+`smethod_12(id)` — party leader set (Client.cs:1605, 1607, 1613, 1617, 1623, 1627, 1631, 1635)
- `F44406000D08`+`smethod_12(id)` — (party QS/leader ack, Client.cs:1604, 1612, 1622, 1630, 1643, 1645, 9328-9331, 9424-9427, 9517-9520)
- `F44406000D09`+`smethod_12(self)` — join request to target (Client.cs:1473)
- `F44406000D0B`+`smethod_12(id)` — (party set leader, Client.cs:1606, 1608, 1614, 1618, 1624, 1632, 1636)
- `F44406000D0C`+`smethod_12(id)` — (party QS quit ack, Client.cs:1644, 1646, 9329-9331, 9425-9427, 9518-9520)
- `F44406001603`+`smethod_11(warpid)`+`0A00` — quest warp battle start (TheBattle.cs:3190, 4703)
- `F4440600 1A 02` — see above
- `F44406001A01` — see above

**Opcode 0x07**
- `F44407002001`+`smethod_12(id)`+action — expression (Client.cs:7334)
- `F44407002002`+`smethod_12(id)`+action — emote (Client.cs:7340, Server.cs:212, 366)
- `F44407000F02`+`smethod_12(id)`+stt — pet removed from map (Client.cs:1847, 6063, 9993, Data.cs:2311, 2326)
- `F44407000D0301`+`smethod_12(id)` — party member joined (Client.cs:1522, 1719)
- `F4440700142C`+`smethod_12(leader)`+`01` — party warp marker (Data.cs:3950)
- `F44407001737`+`smethod_12(id)`+rand — party-buff item cast (Client.cs:4728)
- `F44407003501`+row+col+troiend+`0000` — battle debuff end (TheBattle.cs:9340)

**Opcode 0x08 (stats/skills)**
- `F4440C000801`+type+`01/02`+`smethod_12(value)`+`00000000` — player stat update. Types (DataStructure.Type_Status, DataStructure.cs:1001-1023): `19`=Hp, `1A`=Sp, `1B`=Int, `1C`=Atk, `1D`=Def, `1E`=Agi, `1F`=Hpx, `20`=Spx, `23`=Lv, `24`=TExp, `25`=SkillPoint, `26`=Point, `CF`=Hpx2, `D0`=Spx2, `D2`=Atk2, `D3`=Def2, `D4`=Int2, `D6`=Agi2, `3E`=Tiengtam, `40`=Fai. `01` = increase, `02` = decrease (value always positive). (Data.cs:266-363, 513)
- `F4440C0008016E01`+`smethod_12(level)`+`smethod_12(skillid)` — skill learned/upgraded (Client.cs:5145, 7185, 7203, 7234, Data.cs:5944, FTalk.cs:991)
- `F4440C0008012501`+`smethod_12(count)`+`00000000` — skill points remaining (Client.cs:9267)
- `F44410000803`+`smethod_12(id)`+type+`01/02`+`smethod_12(value)`+`00000000` — party member stat (Data.cs:480-496; Client.cs:9537-9540: `1901`=Hp, `1F01`=Hpx, `D701`=?, `CF01`=Hpx2)
- `F44408000B00`+`smethod_12(id)`+`0000` — battle unit despawn (Client.cs:1272, 7511-7513, 7523-7525, 5992, TheBattle.cs:3121, 3133, 4678, 4844)
- `F44408000505`+`smethod_12(self)`+`smethod_11(npcid)`+`F444040017091301F4440200170F` — doll summon item (Client.cs:4871, 4887)
- `F44408000500`+`smethod_12(leader)`+`F628` — party warp finish marker (Data.cs:3973)
- `F44408001602`+`smethod_11(id)`+`smethod_11(x)`+`smethod_11(y)` — item dropped on map (TheBattle.cs:5012, 5094)
- `F44408001605`+`smethod_11(warpid)`+`smethod_11(x)`+`smethod_11(y)` — (TheBattle.cs:3208, 4721)
- `F44408001703`+`smethod_11(id)`+`smethod_11(x)`+`smethod_11(y)` — drop broadcast (Data.cs:5343)
- `F44408003504`+`smethod_11(id)`+row+col+row+col — battle projectile (TheBattle.cs:3648, 3653, 3892, 3897)

**Opcode 0x09 (char creation)**
- `F44402000901` — character created (Client.cs:9573)

**Opcode 0x0B (battle)**
- `F4440A000B0402`+`smethod_12(id)`+`000003` — player enters battle (map broadcast; TheBattle.cs:686, 912, 921, 930, 939, 948, 951, 5049, 5081, 5115; Server.cs:206, 308)
- `F4440A000B0402`+`smethod_12(id)`+`000005` — ally enters battle
- `F4441C000BFA`+`smethod_11(DiaHinh)`+`03`+warPacket+`F44403000B0A01` — battle board open (leader) (TheBattle.cs:684, 5047)
- `F444`+len(`4+board/2`)+`0BFA`+`smethod_11(DiaHinh)`+`05`+warPacket+`03`+... etc. +`F44403000B0A01` — battle board open (member; leader packet `03`, members `64` marker) (TheBattle.cs:907-950)
- `F4441A000B0503`+warPacket — enemy unit in battle (TheBattle.cs:748, 958-998)
- `F4441A000B0505`+warPacket — ally unit in battle (TheBattle.cs:687, 700, 702, 707, 954, 5061, 5081, 5095, 5115, 5129)
- warPacket (from `ChangedWar`, TheBattle.cs:73-113) = `type(1B)` + `smethod_12(id)` + `smethod_11(idNpcOnMap)` + `smethod_12(idChar)` + row(1B) + col(1B) + `smethod_11(HpMax)` + `smethod_11(SpMax)` + `smethod_11(Hp)` + `smethod_11(Sp)` + lv(1B) + thuoctinh(1B)
- `F444130032010F00`+row+col+skillid(2B)+`0101`+row+col+`010301E0000000` — skill cast animation (TheBattle.cs:2143, 2533)
- `F44413003201`+text — battle skill command (TheBattle.cs:3217)
- `F44406000B06`+`smethod_12(id)` — (Client.cs:1434)
- `F4440A000B0B0000000000002040` — (login frame, Client.cs:8166)

**Opcode 0x0C (warp)**
- `F4440D000C`+`smethod_12(id)`+`smethod_11(mapid)`+`smethod_11(x)`+`smethod_11(y)`+`smethod_11(gocnhin)` — teleport (Data.cs:3982, 3951)
- `F4440B000C`+`smethod_12(id)`+`smethod_11(mapid)`+`smethod_11(x)`+`smethod_11(y)` — teleport appear (Data.cs:3972)

**Opcode 0x0D (party)**
- `F4440A000D05`+`smethod_12(leader)`+`smethod_12(member)` — party member appear broadcast (Client.cs:1521, 1523, 1717, 1720, 9303-9315)
- `F44406000D07/08/09/0B/0C/01/04` — see opcode 0x06 group
- `F444`+len+`0D06`+`smethod_12(leader)`+count+members — party list (Server.cs:489-491)

**Opcode 0x0F (pets)**
- `F4440C000F01`+`smethod_12(id)`+stt+`smethod_12(petid)`+`00/01` — pet appear (Client.cs:1809, 1876, 6025, 6092, 9994, Data.cs:2415)
- `F444`+len+`0F07`+`smethod_12(id)`+per-pet(`stt`+`smethod_12(id)`+`0000000000`+quest(1B)+nameLen+namehex) — pet following list (Data.cs:2419, Server.cs:200, 251, 359, 408)
- `F444`+len+`0F08`+stats — pet status list (Data.cs:2206, 2277)
- `F444`+len+`0F14`+slots — pet inventory slots
- `F4440F00080204`+`smethod_11(stt)`+type+`01/02`+`smethod_12(value)`+`00000000` — pet stat update (Data.cs:629, 2689); type = same Type_Status codes + `6E` = skill level (Client.cs:7275, 7285, 7295, 7305)
- `F4440F000F09`? — pet rename is `F444`+len+`0F09`+self+stt+name (Data.cs:2295-2296)

**Opcode 0x12 (0x12 0x23)** — see §3.1 item 21 (`F44412002304`)

**Opcode 0x14 (talk)** — dialog engine, `F444110014...` family:
- `F44411001401000000010603`+idtalking(2B)+`0000000000000100/0200` — merchant talk open (FTalk.cs:83, 96, 336, 362, 855, 881)
- `F44411001401000000010103`+idtalking+`000000000000C830` — generic talk (FTalk.cs:159)
- `F44411001401000000020103`+idtalking+`00000000000000BB` — quest-not-ready talk (FTalk.cs:141, 2724)
- `F444110014010000000101070000000000000077A7` — talk fail (FTalk.cs:138)
- `F4441100140100000001010700000000000000493C` — warp talk fail (FTalk.cs:3265, 2721)
- `F44411001401000000010603010000000000000100/0200` — sub-menus (FTalk.cs:336, 362)
- `F44411001401000000010607000000000000000300` — (FTalk.cs:3351)
- `F44411001401000000010603040000000000000200` (FTalk.cs:868)
- `F4441100140100000001010301000000000000435D` etc. — various quest dialog frames (FTalk.cs:802, 812, 822, 903-943, ...)
- `F4441100140400000001010301000000000000013C` — dialog continue frames (FTalk.cs:672, 678, 832-855, 964-1550, ...)
- `F44411001401000000010103030000000000004529`/`00000000004929` — (FTalk.cs:672, 678)
- `F4441100140100000001010302000000000000F476` — reborn confirm (Client.cs:5746)
- `F44411001401000000010103000000000000002451` / `F44411001401000000010103000000000000002451` — reborn refuse (Client.cs:5673)
- `F4441100140100000001010300000000000`... note some literals include `01000000010103` prefix; all are opcode 0x14 sub 0x01.

**Opcode 0x17 (inventory)**
- `F444`+len+`1705`+items — full Homdo dump (Client.cs:6420, 8143)
- `F444`+len+`1706`+items — item added (single; Data.cs:5541, Client.cs:6189, 6203)
- `F444`+len+`170B`+equipped — equipped items (Client.cs:8161)
- `F444`+len+`1704`+drops — map drop list (Data.cs:5531)
- `F444`+len+`171E`+shopdef — player shop open (Client.cs:5430)
- `F444`+len+`171F`+`smethod_12(id)`+name — player shop appear (Client.cs:5434, Server.cs:224, 378)
- `F444`+len+`1720`+`smethod_12(id)` — shop closed (Client.cs:5444-5446)
- `F444`+len+`1721`+`0000000000000000000000000000000000`+items — open player shop contents (Client.cs:10159)
- `F444`+len+`172F`+items — Tuideo dump (Client.cs:5600, 8151)
- `F444`+len+`1766`+items — Luulang dump (Client.cs:5814, 8155)
- `F444`+len+`1E01`+items — TienTrang dump (Client.cs:8147)
- `F444`+len+`1E04`+items — TienTrang items after move (Client.cs:5990)
- `F4440E001706`+id(2B)+count(1B)+`000000000000000000` — item added (Data.cs:3232, 3276, 3311)
- `F4440E001708`+slot(1B)+id(2B)+count+doben+long+(100+giatriLong)+khang+`smethod_12(texp)` — item detail update (Client.cs:2287, 2321, 5651, 5865, 5929, 5975)
- `F4440E001706`+id+count+`00`+doben+long+(100+giatriLong)+khang+texp(4B) — item transferred (Client.cs:6386)
- `F4440600170D`+`smethod_11(id)`+slot+dest — compound result (Client.cs:2288, 2322)
- `F44404001709`+slot+count — item decrement (see 0x04)
- `F44405001707`+id+count — item removed from inventory (Data.cs:3364-3408)
- `F44409001703`+`smethod_11(id)`+`smethod_11(x)`+`smethod_11(y)`+`01` — drop created (to owner, Data.cs:3553)
- `F44408001703`+`smethod_11(id)`+`smethod_11(x)`+`smethod_11(y)` — drop created (map broadcast, Data.cs:3554)
- `F4440B001748`+`smethod_12(self)`+`smethod_12(10000)`+`00` — item 46089/46179 special (Client.cs:4741)
- `F44407001737`+self+rand — party-buff item (Client.cs:4728)

**Opcode 0x1A (gold)**
- `F4440A001A04`+`smethod_12(gold)`+`00000000` — gold update (Client.cs:5528, 5536, 6477-6638, 8162, 10116, 10124, 10135, 10143)

**Opcode 0x19 (trade)** — see §2.15; result frames listed under 0x03.

**Opcode 0x23 (account)** — see §2.24.

**Opcode 0x27 (system)**
- `F444`+len+`2709`+`smethod_12(id)`+`C4000000`+nameLen+namehex — server name (Client.cs:8164)

**Opcode 0x28 (hotkeys)**
- `F444`+len+`2801`+`02`+`smethod_11(skillId)`+slot — skill bar dump (Client.cs:8187-8188)

---

## 4. RESPONSE CATALOGUE — literal hex strings found in source

| Hex (literal, full frame) | Meaning | Origin |
|---------------------------|---------|--------|
| `F4440300010901` | hello reply | Client.cs:963 |
| `F44402000106` | wrong password | Client.cs:9553 |
| `F4440300010300` | create character screen | Client.cs:9558 |
| `F44402000901` | character created | Client.cs:9573 |
| `F4440300090300` | name available | Client.cs:9563 |
| `F4440300090301` | name taken | Client.cs:9568 |
| `F44402000504` | movement/warp stop | Client.cs:1446, 1451, 5755; FTalk.cs:274 |
| `F44402001408` | end talk | Client.cs:7921 |
| `F44402001409` | continue after bank/store | Client.cs:10058, 10070 |
| `F44402001407` | warp start | Data.cs:3935 |
| `F4440300142100` | login state frame | Client.cs:8059 |
| `F44402000F0A` | pet UI terminator | Data.cs:2206, 2277 |
| `F44402000602` | talk-open | FTalk.cs:82, 95, 130, 158, 188 |
| `F44402001F0A` | sleep start | Client.cs:652 |
| `F44403001F0100` | sleep done | Client.cs:685 |
| `F44402001F09` | pet can't stay | Client.cs:1822, 1849, 6038, 6065, 6076, 6093 |
| `F44402001F0C` | stable menu close | Client.cs:1811, 6027, 6076, 6093 |
| `F44402001F07` | hotel close | Client.cs:10032 |
| `F44402001F0F` | login frame | Client.cs:8167 |
| `F4440200170F` | use-item end | Client.cs:3653, 5358; Data.cs:3653 |
| `F44402001726` | item 46018 frame | Client.cs:4735 |
| `F44402001732` | storage end | Client.cs:5662, 5876, 5940 |
| `F44402002C01` | reborn done | Client.cs:5729, 9998 |
| `F44402004101` | rank reply 1 | Client.cs:7860 |
| `F44402004102` | rank reply 2 | Client.cs:7857 |
| `F44402001302` | pet recalled | Client.cs:1959, 2061 |
| `F44402001B03` | trade/talk close frame | FTalk.cs:860, 873, 886 |
| `F44403001D0900` | open bank | Client.cs:10055 |
| `F44402001D05` | bank gold | Client.cs:10057 |
| `F44402001D06` | open store | Client.cs:10063 |
| `F44403001B0102` | inventory full | Data.cs:3236, 3244 |
| `F4440300210101` | PK challenge target ack | Client.cs:1297 |
| `F4440300190203` | trade cancelled (partner) | Client.cs:6215 |
| `F4440300190204` | trade complete | Client.cs:6209-6210 |
| `F4440300190207` | trade fail (slots) | Client.cs:6176-6177 |
| `F4440300190209` | trade cancel self | Client.cs:6216 |
| `F4440300190B03` | pet trade cancel (partner) | Client.cs:6354 |
| `F4440300190B04` | pet trade complete | Client.cs:6348-6349 |
| `F4440300190B07` | pet trade fail (dup pet) | Client.cs:6310-6311, 6331-6332 |
| `F4440300190B0A` | pet trade fail (no slot) | Client.cs:6318-6319, 6339-6340 |
| `F4440300190B0F` | pet trade cancel self | Client.cs:6355 |
| `F44403000B0A01` | battle end | TheBattle.cs:684, 950, 5047; Client.cs:1405 |
| `F4440300230101` | password changed | Client.cs:7443 |
| `F4440300230102` | wrong old pass1 | Client.cs:7434 |
| `F4440300230103` | wrong old pass2 | Client.cs:7438 |
| `F4440300230202` | delete-char wrong pass1 | Client.cs:7467 |
| `F4440300230203` | delete-char wrong pass2 | Client.cs:7471 |
| `F4440400210200`+tt | PK off | Client.cs:7357 |
| `F4440400210201`+tt | PK on | Client.cs:7361 |
| `F44404002102`+pk+`00` | war off | Client.cs:7370 |
| `F44404002102`+pk+`01` | war on | Client.cs:7374 |
| `F4440A000B0402`+id+`000003` | enter battle (map) | TheBattle.cs:686 |
| `F4440A000B0B0000000000002040` | login frame | Client.cs:8166 |
| `F4441C000BFA`+... | battle board | TheBattle.cs:684, 5047 |
| `F4441A000B0503`+wp | enemy unit | TheBattle.cs:748 |
| `F4441A000B0505`+wp | ally unit | TheBattle.cs:687 |
| `F444130032010F00`+... | skill cast | TheBattle.cs:2143, 2533 |
| `F44404003505`+r+c | attack marker | TheBattle.cs:1235 |
| `F44404000B01`+r+c | grid move | TheBattle.cs:3120 |
| `F44405000B01`+r+c+`00` | grid clear | TheBattle.cs:3120 |
| `F44408000B00`+id+`0000` | unit despawn | TheBattle.cs:3121 |
| `F44407003501`+r+c+t+`0000` | debuff end | TheBattle.cs:9340 |
| `F44408000505`+id+npc+... | doll summon | Client.cs:4871 |
| `F44408000500`+id+`F628` | warp finish | Data.cs:3973 |
| `F4440A001A04`+gold+`00000000` | gold update | Client.cs:5528 |
| `F44406004202`+pts+`0100` | shop points | Client.cs:7916 |
| `F44412002304`+val+24×`00` | god/hp/sp store | Client.cs:8251 |
| `F44406001A01`+amt | gold in | FTienTrang.cs:23 |
| `F44406001A02`+amt | gold out | FTienTrang.cs:46 |
| `F44406001D01`+amt | bank deposit | FTienTrang.cs:45 |
| `F44406001D02`+amt | bank withdraw | FTienTrang.cs:22 |
| `F44406001D04`+amt | bank balance | Client.cs:10056 |
| `F4440C0008016E01`+lv+skill | skill learn | Client.cs:5145 |
| `F4440C0008012501`+cnt+`00000000` | skillpoints | Client.cs:9267 |
| `F4440F00080204`+stt+... | pet stat | Data.cs:629 |
| `F44410000803`+id+... | party stat | Data.cs:480 |
| `F44405001805020000` | party disband UI | Client.cs:9333 |
| `F44405001805620100` | party disband UI | Client.cs:9334 |
| `F44405001805910100` | party disband UI | Client.cs:9335 |
| `F44407000F02`+id+stt | pet removed | Client.cs:1847 |
| `F4440C000F01`+id+stt+pid+`01` | pet appear | Client.cs:1809 |
| `F44406000F06`+id | dismount | Client.cs:1903 |
| `F4440E000F05`+id+pet+`00000000` | mount | Client.cs:1894 |
| `F44405001F06`+stt+`0000` | pet to stable | Client.cs:1807 |
| `F44402001F0A` | sleep | Client.cs:652 |
| `F44403001F0100` | sleep done | Client.cs:685 |
| `F44406001301`+petid | pet summoned | Client.cs:1947 |
| `F44406001A02`+gold+12×`00` | `method_2` | Client.cs:8261 |
| `F4440D000C`+... | teleport | Data.cs:3982 |
| `F4440B000C`+... | teleport appear | Data.cs:3972 |
| `F4440A000D05`+a+b | party appear | Client.cs:1521 |
| `F44407000D0301`+id | party join | Client.cs:1522 |
| `F44406000D04`+id | party leave | Client.cs:7548 |
| `F44406000D07`/`08`/`09`/`0B`/`0C`/`01`+id | party frames | §3 opcode 0x06 |
| `F44411001401...` | talk dialogs | FTalk.cs (many) |
| `F44411001404...` | talk dialogs (cont) | FTalk.cs (many) |
| `F4440700142C`+id+`01` | party warp | Data.cs:3950 |
| `F444`+len+`1704` | map drop list | Data.cs:5531 |
| `F444`+len+`1705` | homdo dump | Client.cs:6420 |
| `F444`+len+`1706` | item added | Data.cs:5541 |
| `F444`+len+`170B` | equipped items | Client.cs:8161 |
| `F444`+len+`171E` | shop open | Client.cs:5430 |
| `F444`+len+`171F` | shop appear | Client.cs:5434 |
| `F444`+len+`1720` | shop closed | Client.cs:5444 |
| `F444`+len+`1721` | shop contents | Client.cs:10159 |
| `F444`+len+`172F` | tuideo dump | Client.cs:5600 |
| `F444`+len+`1766` | luulang dump | Client.cs:5814 |
| `F444`+len+`1E01` | tienTrang dump | Client.cs:8147 |
| `F444`+len+`1E04` | tienTrang items | Client.cs:5990 |
| `F4440E001706`+... | item added | Data.cs:3232 |
| `F4440E001708`+... | item detail | Client.cs:2287 |
| `F4440600170D`+... | compound result | Client.cs:2288 |
| `F44409001703`+... | drop owner | Data.cs:3553 |
| `F44408001703`+... | drop map | Data.cs:3554 |
| `F4440B001748`+... | 46089/46179 | Client.cs:4741 |
| `F44407001737`+... | party buff | Client.cs:4728 |
| `F44404001709`+slot+count | item decrement | Data.cs:3419 |
| `F44405001707`+id+count | item removed | Data.cs:3364 |
| `F44403001711`+slot | equip | Client.cs:2198 |
| `F44404001710`+s1+s2 | unequip | Client.cs:2213 |
| `F44404001717`+stt+slot | pet equip | Client.cs:5375 |
| `F44405001716`+stt+s1+s2 | pet unequip | Client.cs:5392 |
| `F44404000B0702FF`+... | gold item | Client.cs:5181 |
| `F44404000B09FF01`+... | fai item | Client.cs:5337 |
| `F44402001B03` | (trade/talk) | FTalk.cs:860 |
| `F44406001603`+id+`0A00` | quest warp | TheBattle.cs:3190 |
| `F44408001605`+... | quest warp items | TheBattle.cs:3208 |
| `F44408001602`+... | drop map | TheBattle.cs:5012 |
| `F44402001B03` | (trade/talk) | FTalk.cs:860 |
| `F44402001B03` | (trade/talk) | FTalk.cs:860 |
| `F44411001401000000010603`+id+`0000000000000100` | merchant menu 1 | FTalk.cs:83 |
| `F44411001401000000010603`+id+`0000000000000200` | merchant menu 2 | FTalk.cs:96 |
| `F444110014010000000101070000000000000077A7` | talk fail | FTalk.cs:138 |
| `F44411001401000000020103`+id+`00000000000000BB` | quest not ready | FTalk.cs:141 |
| `F44411001401000000010103000000000000C830` | generic dialog | FTalk.cs:159 |
| `F4441100140100000001010700000000000000493C` | warp fail | FTalk.cs:2721, 3265 |
| `F44411001401000000010607000000000000000300` | quest warp | FTalk.cs:3351 |
| `F4441100140100000001010302000000000000F476` | reborn confirm | Client.cs:5746 |
| `F44411001401000000010103000000000000002451` | reborn refuse | Client.cs:5673 |
| `F44404000F130100` | pet frame | Data.cs:2206 |
| `F44405000F12010000` ×4 | pet slot frames | Data.cs:2206 |
| `F44402001D05` | bank gold | Client.cs:10057 |

---

## 5. GAPS / NOTES FOR THE PORT

1. **FTalk.H6** (Client.cs 0x14 sub 6) is a ~3000-line, NPC-id-hardcoded dialog tree. This document captures its framing and packet templates; a faithful port either reproduces the data table or re-expresses it as data. The generic talk-data path (FTalk.cs:2640-2820) covers the majority of NPCs.
2. **TheBattle.cs** (9577 lines) contains the full battle engine (initiative, damage formulas `GetDamageThuoctinh`, `GetDamageSkillInt`, combos, drops `GetRandomMissDrop`, AI). Only the wire packets are catalogued here (§3 opcode 0x0B/0x32/0x35); the numeric combat model is out of protocol scope but must be reproduced for byte-identical behaviour.
3. **String encoding**: `smethod_13` truncates to the low byte of each UTF-16 char; `smethod_17` pre-maps Vietnamese; chat is ASCII-decoded (`Encoding.ASCII`, FChat.cs:17, 464, 474). The client's own display encoding is unknown — port must emit exactly these bytes.
4. **`Update_H1B`** merchant table is fully hardcoded (map, menu) → (itemId, price). A Rust port must replicate the full enumeration (Client.cs:6472-7129) or accept behavioural divergence.
5. **Item-craft RNG tables** in `Update_H17` sub 14 (Client.cs:2220-3799) — hundreds of `case n → HomdoAddItem(id, 1)` entries. Rewards are drawn with `Random.Next(0, N)`; the map of case→item is fixed and must be transcribed exactly.
6. **MySQL dependency**: opcode 0x23 sub 3 (item codes) queries a `item_code` table via `MySqlDbConnection`. Without MySQL, that branch degrades to red-message errors.
7. **Multi-threading**: each received frame is handled on its own thread (Client.cs:628-633) → out-of-order handling is possible in C#; a Rust port may choose a single-threaded deterministic model.
8. **`shutdown()`** (Client.cs:466-570): on 0-byte receive, login rejection, or double-login. Sends nothing to the peer; broadcasts leave-battle + offline frames if `_My_Id > 0`.
9. Frame length is **2 bytes LE**; max frame payload 65535 bytes — large inventory dumps are still single frames but split across multiple `Send` calls by concatenation.
10. `smethod_11` of values > 0xFFFF truncates to low 16 bits (`.ToString("X4")`); `smethod_12` of values > 0xFFFFFFFF truncates to low 32 bits. IDs stay below 2^32 in practice.
