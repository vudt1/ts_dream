# TS Dream — Rust Server Porting Specification

**Version:** 1.0
**Status:** Published (handoff deliverable)
**Normative reference source:** `ts_server_old/` (the legacy C# server) — used only as ground truth and as the location of the raw data files. An executor must be able to build the server **without reading any C# source**. Verbose extracts with exact `file:line` provenance live under `.scratch/rust-porting-spec/research/`; each chapter cites its asset. Those assets are the authoritative detail store; this document is the normative contract.

## Scope

- **In scope:** full behavioural parity with the C# server — the 29 client-to-server opcode handlers (Chapter 2), the battle engine (Chapter 6), quests, pets, trading, player shops, NPC shops, storage, account management, and the web admin dashboard (Chapter 7).
- **Reference only, not a target:** the repo file `TS_Server_OP_Code_basic.md` (a 69-opcode document). Only what the C# server actually implements is in scope.
- **Out of scope:** the legacy WinForms UI (`FormServer.cs`), SQLite storage, `MySqlDbConnection.cs` (legacy MySQL surface), the game client itself, migrating existing player `.accdb` files (fresh start), and any opcode the C# server does not implement.

## Non-negotiable global decisions (settled; do not reopen)

1. **Wire encoding is VISCII 1.1** (Chapter 4). Bytes 0x00–0xFF are transmitted verbatim. **Do not** convert to UTF-8 / utf8mb4 anywhere in the wire or in game-text columns.
2. **Database is MySQL 8** (InnoDB), a single shared database `ts_dream`, running locally at `localhost:3306`. MySQL is an external service; the binary remains a single file.
3. **Every game-text column must declare `CHARACTER SET latin1` explicitly** (recommended `COLLATE latin1_bin`), because the server default and the `ts_dream` database both inherit `utf8mb4`, which corrupts VISCII high bytes (0x80–0xFF).
4. **Scoping contract:** because the schema is shared (no per-character file), every SQL statement ported that touches the 9 gameplay tables **must carry a `player_id` predicate** or a composite-PK term (Chapter 5 §5.4).
5. **Fidelity goal:** byte-exact with real client↔C#-server traffic; acceptance is a capture-based diffing harness (Chapter 9).
6. **Player migration:** none — fresh start.
7. **Stack:** `tokio` + `axum` + `sqlx` (MySQL) + `askama`/HTMX, single binary. Protocol constants are hardcoded (Chapter 8): XOR `0xAD`, frame magic `F4 44`, ID prefix `vn`, minimum version `186`, server name `TSVN`.

---

# Table of Contents

1. [Architecture](#chapter-1--architecture)
2. [Protocol](#chapter-2--protocol)
3. [Static Data Files](#chapter-3--static-data-files)
4. [Text Encoding Contract](#chapter-4--text-encoding-contract)
5. [Database (MySQL 8)](#chapter-5--database-mysql-8)
6. [Battle Engine](#chapter-6--battle-engine)
7. [Web Dashboard](#chapter-7--web-dashboard)
8. [Config](#chapter-8--config)
9. [Acceptance](#chapter-9--acceptance)

**Appendix A** — FTalk.H6 menu table, pre-dispatch rules, and exceptions (protects the Protocol chapter).

---

# Chapter 1 — Architecture

## 1.1 Runtime shape

- **One binary** that brings up two listeners:
  - Game server: TCP `0.0.0.0:6414`, framed protocol of Chapter 2.
  - Web dashboard: HTTP `0.0.0.0:8090` (Chapter 7).
- **MySQL 8** is reached over TCP `localhost:3306`; the binary does not embed a database. If MySQL is unreachable at boot the process **hard-exits** with a clear diagnostic (§5.7). The HTTP dashboard is never served with a dead database.

## 1.2 Locked crate set

- `tokio` (async runtime)
- `axum` (HTTP)
- `sqlx` version `0.8`, features `["mysql", "runtime-tokio-rustls", "migrate"]`
- `askama` + HTMX (server-rendered dashboard page)
- `tokio::sync::broadcast` (live-log SSE fan-out)
- `tokio::sync::RwLock` + `Arc` (shared `AppState`)

## 1.3 Start-up sequence

```
main()
|-: load config (Chapter 8)                     # toml + TS_ env overrides
|-: connect MySQL 8 pool (MySqlPool)         # FAIL-FAST exit on error
|-: sqlx::migrate!()                          # BEFORE binding any listener
|-: spawn web server (port 8090)            # always up once DB is reachable
|-: load static data (Chapter 6->3)         # on complete -> DataLoaded flag
`--: start TCP accept loop (port 6414)      # gated on DataLoaded == true
```

## 1.4 Concurrency model

The C# server dispatched each received frame on its own thread and mutated global state with **no locking** (data races it tolerated). A Rust port **must not replicate the mechanism** — it must reproduce only the *observable behaviour* (packet order, RNG consumption). Use:

- **`Server` registry** — `Clients` (logged-in players by id), `TheBattles` (live battles by id), `IdBattleCount` (i32, starts 1, incremented at each battle creation), shared via `Arc<RwLock<…>>`.
- **Per-connection task** — owns one socket, performs framing (XOR + hex split, §2.1), and dispatches to opcode handlers.
- **Per-battle task** — one async task per battle that owns its grid state (`ListWar`, `_keys`, the three RNG streams); player turn input arrives over a channel and is consumed on that task. This keeps battle state race-free and deterministic.

Because the C# server allowed out-of-order packet handling (thread-per-frame) and the acceptance harness only asserts on deterministic scenarios (Chapter 9), a serial, per-connection task is the correct and safer model.

## 1.5 Hardcoded server constants

| Constant | Value | Provenance |
|---|---|---|
| Frame magic | `F4 44` | `_Header` (DataStructure.cs:13-18) |
| XOR key | `0xAD` (173) | `Class5.smethod_5` |
| Game port | `6414` | FormServer.cs:3309 |
| Web port | `8090` | standing decision |
| ID prefix | `vn` | Server.cs:53 |
| Min client version | `186` | Client.cs:383 |
| Server name | `"TSVN"` | Client.cs:8163 |
| Max level | `200` | Data.cs:72 |
| PerEXP default | `0` (dashboard overrides runtime value; not persisted) | standing decision |
| Drop band widths | `25, 23, 20, 4, 3, 1` | Server.cs:33 |
| Admin-id threshold | `300012` (ids below are treated as server/admin) | Client.cs |

---

# Chapter 2 — Protocol

Normative source: `.scratch/rust-porting-spec/research/01-protocol-reference.md`. The wire contract is byte-exact; where a packet is written as a literal, the literal is authoritative.

## 2.1 Transport and framing

- TCP socket; bind `0.0.0.0:6414`; listen backlog 5; accept is gated on `DataLoaded == true`. No handshake on accept; nothing is sent until the client speaks first.
- Read buffer 8192 bytes; a 0-byte receive triggers `shutdown()` (sends nothing to the peer; if the session id > 0 it first broadcasts the leave-battle + offline frames).
- **Receiver decode:** XOR every received byte with `0xAD`, convert to uppercase hex. The length field = hex at offset 4 (chars 4..7, little-endian u16) = byte count **after** the 4-byte header. A complete frame is `4 + length` bytes = `8 + length*2` hex chars. Frames are concatenated on the wire; split them in a loop; a partial trailing frame is buffered and prepended to the next chunk.
- **Send path is a pure transform — no checksum, no trailer.** Build the hex-string packet → hex-decode to bytes → XOR every byte with `0xAD` → one `write`. (Research 06 §(1) resolved an earlier guess; do not append a checksum byte.)

### Frame layout (after XOR decode)

| Offset | Size | Meaning |
|---|---|---|
| 0 | 2 | magic `F4 44` |
| 2 | 2 | length LE u16 = bytes after the 4-byte header |
| 4 | 1 | opcode |
| 5 | 1 | sub-opcode |
| 6.. | n | payload |

### Primitive encoders (must be exact)

| Name | Semantics | Example |
|---|---|---|
| `le16(v)` | 2-byte little-endian hex | `7168 → "001C"` |
| `le32(v)` | 4-byte little-endian hex | `3 → "03000000"` |
| `u16_le(b0,b1)` | little-endian u16 from two bytes | — |
| `u32_le(b0..b3)` | little-endian u32 from four bytes | — |
| `hex(bytes)` | uppercase hex of a byte array | `[0A,0B] -> "0A0B"` |
| `bytes(h)` | hex string → bytes | `"03000000" → 03 00 00 00` |
| `xor01(b)` | same as above, XOR each byte with `0xAD` | — |
| `strhex(s)` | per char, 2 ASCII hex digits of the **low byte** (`& 0xFF`) | — |

Name length fields in packets are **byte counts** (equal to string length when all chars ≤ 0xFF, i.e. for the VISCII alphabet).

## 2.2 Client → server dispatch

`UpdateMainGrid_Recv` switches on byte `[4]`. **All handlers are wrapped in an empty `try/catch`** — a thrown exception is silently swallowed and the socket stays open. Unknown opcodes are silently ignored (no reply, no close).

Handled opcodes (29):

| Op | Handler | § |
|---|---|---|
| 0x00 | Hello | 2.3.1 |
| 0x01 | Auth / Login | 2.3.2 |
| 0x02 | Chat | 2.3.3 |
| 0x03 | Enter-game confirm | 2.3.4 |
| 0x06 | Move | 2.3.5 |
| 0x08 | Stat allocation | 2.3.6 |
| 0x09 | Create character | 2.3.7 |
| 0x0B | Battle control | 2.3.8 |
| 0x0C | Teleport confirm | 2.3.9 |
| 0x0D | Party | 2.3.10 |
| 0x0F | Pet actions | 2.3.11 |
| 0x13 | Pet summon/recall | 2.3.12 |
| 0x14 | Action/Talk | 2.3.13 |
| 0x17 | Inventory/items | 2.3.14 |
| 0x19 | Trade | 2.3.15 |
| 0x1B | NPC shop buy/sell | 2.3.16 |
| 0x1C | Learn/upgrade skills | 2.3.17 |
| 0x1D | Bank gold | 2.3.18 |
| 0x1E | Storage transfer | 2.3.19 |
| 0x1F | Pet stable menu | 2.3.20 |
| 0x20 | Expressions | 2.3.21 |
| 0x21 | PK / war mode | 2.3.22 |
| 0x22 | Game points | 2.3.23 |
| 0x23 | Account management | 2.3.24 |
| 0x28 | Hotkey / skill bar | 2.3.25 |
| 0x2C | Reborn pet | 2.3.26 |
| 0x32 | Battle commands | 2.3.27 |
| 0x41 | Rank | 2.3.28 |
| 0x42 | GM shop / points | 2.3.29 |

Not handled (silently ignored, no socket action): `0x04, 0x05, 0x07, 0x0A, 0x0E, 0x10, 0x12, 0x15, 0x16, 0x18, 0x1A, 0x24..0x27, 0x29..0x2B, 0x2D..0x31, 0x33..0x40, 0x43..0xC7`. `ClientBattle` is an empty stub (no-op).

## 2.3 Opcode handler contracts

### 2.3.1 Opcode 0x00 — Hello (exact `F444010000`)

If the whole frame equals `F444010000` (opcode 0x00, **no sub byte**, length 1) → reply `F4440300010901`. Anything else → silently ignored.

### 2.3.2 Opcode 0x01 — Login

| Offset | Size | Field |
|---|---|---|
| 5 | 1 | sub (ignored) |
| 6–9 | 4 | account id, LE u32 |
| 10–11 | 2 | server prefix ASCII, must equal `"vn"` (case-insensitive); mismatch → silent return |
| 12–13 | 2 | client version LE u16; `< 186` → `shutdown()` |
| 14..end | n | password bytes (plain ASCII, `chr(packet[i])`) |

Order of checks:
1. version gate → too low = `shutdown()`.
2. account exists (in the `accounts` table) → missing → `shutdown()`.
3. password == `pass1`; wrong → send `F44402000106` and keep the connection open.
4. double-login guard (`Clients` already contains id) → `shutdown()`.
5. success → `_My_Id = id`, run `Logined()` (§2.4.1).

### 2.3.3 Opcode 0x02 — Chat

- Sub 2 — global/map chat. Message = `data[6..]` (ASCII). Text > 60 characters → dropped. Server/admin (`_My_Id < 300012`) slash-commands: `/additem ID[,count]`, `/addpet ID`, `/addskpoint N`, `/where`, `/warp mapid`, `/test N`, `/reloadtalks`, `/battle N`, `/packet …`, `/sendpacket HEX`, `/endtalk`, `/loadnpcs`, `/loaditems`, `/loadscenes`; all players additionally `/sleep`, `/openhotel`, `/openstore`, `/openbank`. Normal broadcast: if `Trangbi` slot 6 id == `23100` → all clients (op 0x02 sub 0x01), else only same map (op 0x02 sub 0x02).
- Sub 3 — whisper. Target = bytes 6–9 LE u32; message = bytes 10..; both sender and recipient receive an op 0x02 sub 0x03 frame carrying the **recipient** id.
- Sub 4 — no-op.
- Sub 5 — party chat; sent to leader + members via `SendToClient` (op 0x02 sub 0x05, sender id embedded).

Chat wire forms (FChat.cs:484-529):

| Chat | Frame |
|---|---|
| Toan (global) | `F444` + len(`6+chat`) + `0201` + `sender` + chathex |
| Gan (map) | `F444` + len(`6+chat`) + `0202` + `sender` + chathex |
| ThiTham (whisper) | `F444` + len(`6+chat`) + `0203` + `recipient` + chathex |
| Doi (party) | `F444` + len(`6+chat`) + `0205` + `sender` + chathex |

`sender` / `recipient` are `le32(id)`. Chat payload is the raw byte array; never re-encoded.

### 2.3.4 Opcode 0x03 — Enter-game confirm (`F44402000301`)

Entire frame must equal `F44402000301`. → `Logined()`. If already logged in, the duplicate registration throws and is swallowed (no-op). If not authed yet, `Logined()` falls to `CreatChar()` → `F4440300010300`.

### 2.3.5 Opcode 0x06 — Move (sub 1)

| Offset | Size | Field |
|---|---|---|
| 6 | 1 | view direction |
| 7–8 | 2 | x (LE u16) |
| 9–10 | 2 | y (LE u16) |

If in battle → ignore. If party leader, also move each online member. `Walked(id,x,y,gocnhin)` broadcasts to the same map: `F4440B000601` + `le32(id)` + dir(1B) + `le16(x)` + `le16(y)`.

### 2.3.6 Opcode 0x08 — Stat allocation

Sub 1, not in battle. `data[8]` = stat id, `data[9]` = points. Gate `Point >= points && points > 0`. Stat id → column + recompute:

| Id | Effect |
|---|---|
| 25 | Hpmax — recompute `getHpMax(reborn, job, lv, Hpx + n) + Hpx2` |
| 26 | Spmax — recompute |
| 30 | Atk (cap 400) |
| 29 | Def (cap 400) |
| 27 | Int (cap 400) |
| 28 | Agi (cap 400) |
| 31 | Hpx (cap 400; also Hpmax recompute) |
| 32 | Spx (cap 400) |

All changes flow through the `PlayerUpdateDataId` path which emits op 0x08 stat packets (§2.4/opcode 0x08).

### 2.3.7 Opcode 0x09 — Create character

**Sub 1 (create)** layout (`payload` = `decoded[6..]`; C# `Client.cs:1150-1200`):
`sex(1) | unused(1) | hair(1) | unused(1) | color(8 raw bytes → hex) | thuoctinh(1) | int atk def hpx spx agi (6×1B) | pass1_len(1B) + pass1 | pass2 (len-prefixed)`.
Byte offsets: `[0]` sex, `[2]` hair (single byte — the byte at `[3]` is an unused gap), `[4..12]` color (8 raw bytes → hex), `[12]` thuoctinh, `[13..19]` the six stats, `[19]` pass1 length, `[20..20+len]` pass1, `[20+len+1..]` pass2. The character **name is not in this packet** — it comes from the name-check (sub 2), which stashes it (`string_1` / `pending_new_char_name`).

Action = **one atomic DB transaction** (§5.6):
- INSERT `players` row (stats computed; remaining columns rely on DEFAULT).
- INSERT `SkillSave` rows for `Id` 1..10, `IdSkill=0`.
- mutate `Skill` table as the C# build.
- update `accounts.pass1/pass2`.
- reply `F44402000901`. Any exception → `shutdown()`.

**Sub 2** — name check: `data[6..]` = candidate; exists → `F4440300090301`; free → `F4440300090300` and remember the candidate.

### 2.3.8 Opcode 0x0B — Battle control

- Sub 1 — leave battle (`data[6] == 3`): clear `_My_IdBattle`, send `F44408000B00` + `id4` + `0000`.
- Sub 2 — PK challenge (inner sub `data[6]`):
  - 2: target = bytes 7–10 LE; guards `_My_IdBattle==0`, `_My_Pk==1`, target offline/in battle. Target `Pk==0` → `F4440300210101`; `Pk==1` → start a PK battle (Chapter 6, DiaHinh 112).
  - 3: attack NPC: `_My_IdBattle==0`; npc id = bytes 7..; **blocked** if npc id ∈ `[20000,22000)` / `[23000,25000)` / `[26000,27000)`; else start NPC battle (DiaHinh 112, `idNpcOnMap` = bytes 11–12).
  - 4: join a battle — first free `ListQS` slot, register, build the battle-join packet (§6.8) + `F44403000B0A01`.
  - 5: `JamPlayerToBattle` — no-op stub.
- Sub 6 — broadcast `F44406000B06` + `id4` to map.

### 2.3.9 Opcode 0x0C — Teleport confirm

Sub 1: if a party leader exists and is not self → send `F44402000504F44402001408` and return. Otherwise `warpFinish=false`, send the same two packets, reset talk counters.

### 2.3.10 Opcode 0x0D — Party

- Sub 1 — request join: target != self and online; set pending ids; send target `F44406000D09` + `id4`.
- Sub 3 — accept join (`data[6]==1`, requester bytes 7–10): requester==held; `Walked`, join into leader member slots; broadcast `F4440A000D05` + leader + member pairs; send `F44407000D0301` + member id to self + requester; `PartySendStatus` both; sync member structs.
- Sub 4 — leave/disband → `GiaiTanParty(id)`.
- Sub 5 — change leader: current leader may only; set `_My_IdQS`; send `F44406000D0B`/`F44406000D08` + new leader id to self and map.
- Sub 6 — QS quit: if `_My_IdQS>0` send `F44406000D08`/`F44406000D0C` + qs id, clear.
- Sub 7 — invite: target != self; send target `F44406000D01` + `id4`.
- Sub 8 — accept invite (`data[6]==1`): inviter==held; require self has leader slot; join; `F4440A000D05`, `PartySendStatus`; `F44407000D0301` to inviter.

### 2.3.11 Opcode 0x0F — Pet actions

- Sub 2 — release: `data[6]`=stt; if active, clear; `Removepet`; broadcast `F44407000F02` + `id4` + stt to map.
- Sub 3 — store to stable: find first empty stable slot (5–8) via `SwitchPet(stt+4, slot)`; on success `SendStatusPet`, reply `F44405001F06`+stt+`0000`, `UpdateStatusPetWhenUseItem`, broadcast `F4440C000F01`+self+stt+petid4+`01`, then `F44402001F0C`.
- Sub 7 — take from stable: if the pet is active → red msg + `F44402001F09` and stop; find first free battle slot; switch; send pet summary + `F44402001F09` (detail in research §2.11).
- Sub 8 — swap: if that stt fights → red msg + `F44402001F09F44402001F0C`; else switch, pet summary, status, map broadcast, then `F44402001F09F44402001F0C`.
- Sub 4 — mount horse: `data[6..9]` pet id, must be `18000..19000`, != current, must exist; set `_My_Horse`, send `F4440E000F05` + `id4` + pet + `00000000` to self + all.
- Sub 5 — unmount: clear `_My_Horse`, send `F44406000F06` + `id4` to self + all.
- Sub 6 — rename pet: `data[6]`=stt, `data[7..]` new name; broadcast `F444`+len+`0F09`+`id4`+stt+namehex.

### 2.3.12 Opcode 0x13 — Pet summon / recall

Out of battle: Sub 1 — `data[6..9]` pet id; if not riding and stt ≤ 4 → set active, send `F44406001301`+`id4`. Sub 2 — if active exists clear + `F44402001302`.
In battle (cell with `_Attacked==0`): Sub 1 loads pet row, removes player's battle cells (XOR row), spawns pet via `ChangedWar` type 4, broadcasts `F4441A000B0505`+warPacket + `F44406001301`+id; Sub 2 recalls pet similarly + `F44402001302`.

### 2.3.13 Opcode 0x14 — Action/Talk (the largest surface)

Sub 1 → `FTalk.H1` (start talk), Sub 6 → `FTalk.H6` (menu/continue engine; see Appendix A), Sub 4 → `EndTalk`, Sub 8 → `FTalk.H8` (warp talk), Sub 9 → set `SelectMenu = data[6]`, default → `EndTalk()`.

`EndTalk()` = `F44402001408` + reset talkcount/idtalking/SelectMenu.

**Sub 1 (start talk).** `data[6..7]` = the map object id (LE u16). `Typetalk="NPC"`; resolves the NPC instance. Distance gate ±150 (x/y). Special NPC ids:
- `16080 / 16004 / 16011 / 16015: `F44402000602` + `F44411001401000000010603` + idtalking(2B) + `0000000000000100`.
- `15002 / 16001 / 16016`: same but tail `…0000 02 00`.
- `16012`: silent.
- generic: if talk data exists (in `Data_Talks`) → `F44402000602` then `TalkMessages(...)` (split the dialog hex on the literal `"F444"`, send each fragment as `F444…`, 500 ms apart). If the talk has zero dialogs **and** a `[TEAMDEF]` block → start a battle quest. If no talk data → npc-body branches (Appendix A).

**Sub 6 (H6)** — menu/continue engine (data-driven) — see **Appendix A**.

### 2.3.14 Opcode 0x17 — Inventory / items (subject to sub-dispatch, huge)

| Sub | Meaning |
|---|---|
| 2 | pick up map drop (distance gate) |
| 3 | drop item |
| 10 | move/stack — on success echo the whole raw packet back |
| 11 | equip player |
| 12 | unequip player |
| 14 | compound/craft (gems) — fixed recipes + big RNG item table |
| 15 | use item (discover the huge dispatch below) |
| 17 | equip pet item |
| 18 | unequip pet |
| 30 | open player shop |
| 31 | close player shop |
| 32 | open someone's shop |
| 33 | buy from shop |
| 36 | homdo → tuideo (requires a special pet) |
| 37 | tuideo → homdo |
| 46 | reborn (job change) |
| 48 | warp finish ack |
| 51 | homdo → luulang |
| 52 | luulang → homdo |

**Sub 15 (use item) highlights** (byte-faithful): warps, add-pet items, sleep item (leader only), lucky-box rolls, stat books, HP/SP store restores, doll summon `F44408000505`+`id4`+`npcid`+`F444040017091301F4440200170F`, god books, Texp books (Lv ≤ 200), skill books (learn → `F4440C0008016E01`+`le32(lv)`+`le32(skillid)`), point books, gold/FAI items, potions (restore `Hp*Sp*Fai1`...), party buffs. Each normal use ends with `F44404001709`+slot+count+`F4440200170F` unless an explicit branch returned.

**Sub 14 (craft/compound)** — a fixed map of `case → item id` with `Random.Next(0,N)` draws. Transcribe the full enumeration from `Client.cs:2220-3799` (captured in research §2.14).

**Sub 30–33 — player shop**: reply + broadcast frames (see the catalogue below) for open, place/name broadcast, open-contents, buy.

**Sub 1B (NPC shop)** — see 2.3.16.

**Sub 46 (reborn)** — requires no equipment in slots ≤ 6; updates player rebirth formula columns, `DELETE FROM Skill` (scoped), replies `F44402002C01`, quest step, death/close socket.

Do NOT invent sub-opcodes; the dispatch treats unrecognized subs per `0x17` as described in research §2.14.

### 2.3.15 Opcode 0x19 — Trade

- Sub 1 — open: `data[6..9]` partner; both get `F44406001901` + `le32(other)`; pet trade uses `F4440600190A`.
- Sub 2 — set gold + items: `data[6..9]` gold u32, `data[10..]` slot list. Partner gets `F444`+len+`1903` + gold + item entries.
- Sub 3 — confirm/cancel (`data[6]==1/2`): confirm requires both accepted; `GoldTransfer`, swap items both directions; no slot → `F4440300190207` both; success → `F4440300190204`; cancel → `F4440300190203` partner + `F4440300190209` self, `TradeFinish`.
- Sub 10/11/12 — pet trade: open, offer pet (28-char padded name, pad `6`), confirm/cancel (`F4440300190B03/04/07/0A/0F` family).
- Sub 20 — transfer item to player: `data[10..13]` recipient, then 9 slot/count pairs; move items; recipient `F4440E001706`+items; sender re-sends `F444`+len+`1705`.

### 2.3.16 Opcode 0x1B — NPC shop buy/sell

Hundreds of hardcoded `(map, menu) → (itemId, price)` pairs. Gather them verbatim from `Client.cs` (see research §2.16). Buy: check gold ≥ price → `HomdoAddItem`, `PlayerUpdateDataId(Gold)` send `F4440A001A04`+gold+`00000000`, red message. Selling: on a sell with `idnpctalking ∈ {16005, 99999}` scan inventory items `26001..26455` (or `27001..27165` for `16002 / 99999`), each sold adds `data[7]` count to gold, reply `F4440A001A04`+gold+`00000000`.

### 2.3.17 Opcode 0x1C — Learn/upgrade skills

Sub 1 player: sequence of {skill id (LE16) + target level}; validate LvMax/Reborn/prereqs/SkillPoint; each success → `F4440C0008016E01`+`le32(lv)`+`le32(skill)`; end → `F4440C0008012501`+`le32(count)`+`00000000`.
Sub 2 pet: `data[6]` stt, `[7–8]` skill id, `[9]` level; only upgrade existing slot; reply `F4440F00080204`+stt+`6E01`+`le32(lv)`+`le32(skill)`.

### 2.3.18 Opcode 0x1D — Bank gold

Sub 1 withdraw `data[6..9]`: gate bank ≥ amount and gold+amount ≤ 9999999 → `F44406001D02`+`le16(amount)` and `F44406001A01`+`le16(amount)`. Sub 2 deposit ≈. (`F44406001D01`+`le16`+`F44406001A02`+`le16`).

### 2.3.19 Opcode 0x1E — Storage transfer (TienTrang)

Sub 1 TienTrang→Homdo: per move a slot detail + end `F44402001732`. Sub 2 Homdo→TienTrang: per move `F44404001709`+slot+`32`, then `F444`+len+`1E04`. Sub 8 set `SelectMenu=40`.

### 2.3.20 Opcode 0x1F — Pet stable menu

Semantics identical to op 0x0F sub 3/7/8 but the reply frames are `F44405001F06`+stt+`0000` and the menu ends with `F44402001F09`/`F44402001F0C`; see research §2.20 for the exact fields.

### 2.3.21 Opcode 0x20 — Expressions

Sub 1: `data[6]` action → map broadcast `F44407002001`+`id4`+action. Sub 2: set `_My_Dongtac`, broadcast `F44407002002`+`id4`+action. Sub 3: clear, no packet.

### 2.3.22 Opcode 0x21 — PK / war mode

Sub 1: `data[6]` 0 → `Pk=0`, reply `F4440400210200`+`_My_ThamChien`; 1 → `Pk=1`, reply `F4440400210201`+`_My_ThamChien`. Sub 2: `data[6]` 0/1 → set `ThamChien=0/1`, reply `F44404002102`+`_My_Pk`+`00/01`.

### 2.3.23 Opcode 0x22 — Game points

Sub 1 → `F44412002304`+`le16(gold)`+`00`×24 (the "god/points" panel).

### 2.3.24 Opcode 0x23 — Account management

- Sub 1 — change password: 4 len-prefixed strings (oldPass1, oldPass2, newPass1, newPass2). wrong oldPass1 → `F4440300230102`; wrong oldPass2 → `F4440300230103`; success → write + `F4440300230101`.
- Sub 2 — delete character (2 len-prefixed strings). Verify old pass1/pass2; on success: leave battle (battle/map removal packets), `GiaiTanParty`, server-offline + map broadcast, **delete the `players` row and all 9 gameplay-table rows for that `player_id`**, remove from `Clients`, close.
- Sub 3 — redeem item code (§5.6). Len-prefixed `code`, `password`.

### 2.3.25 Opcode 0x28 — Hotkey / skill bar

`data[7..8]` skill id LE16, `data[9]` slot 1..10 → `SkillSaveUpdateId(slot, skill)`; no response.

### 2.3.26 Opcode 0x2C — Reborn pet

`stt = u16(data[6..7])`. Find homdo slot with `RbPetFrom==stt` and valid `RbPetTo` (scan slots 1..25); recompute pet from NPC template (level 1, skills from NPC, 30/60 threshold bonuses), consume Rb fields. Packets: `F44407000F02`+self+stt, `F4440C000F01`+self+stt+newid+`01`, status, `F44406001301`+newid, `F44402002C01`. Guards fail → silent.

### 2.3.27 Opcode 0x32 — Battle commands (input)

Sub 1 — skill: `data[6]`row, `7`col, `8`rowAttack, `9`colAttack, `10..11` skill id LE16. Range-check rows/cols; cell must exist, `_Id>0`, `_Attacked==0`. Level via `SkillGet` (player) or pet skill match. Set `_LvSKill/_RowAttack/_ColumnAttack/_IdSkill/_Attacked=1`; broadcast `F44404003505`+row+col.
Sub 2 — use item: `data[10..11]` item id; if in `26001..27165` → heal cell cell+pet by `_Hp`/`_Sp`, remove 1, `_Attacked=1`.

### 2.3.28 Opcode 0x41 — Rank → `F44402004101` (sub 1) / `F44402004102` (sub 2).

### 2.3.29 Opcode 0x42 — GM shop / points

Sub 1 — mall: `data[9..10]` item id, `[11..12]` price; if `_Shop_Point ≥ price` and free slot → `HomdoAddItem`, deduct, `Shoppoin`. Sub 2 — no-op. Sub 3 — `F44406004202` + `le16(points)` + `0100`.

## 2.4 Server → client catalogue (wire vocabulary)

Reuse the literal strings verbatim; compute the marked `<fields>`.

### 2.4.1 The `Logined1` sequence — in exact order

1. `F444020002000F0A08F4440300142100` — actually: `F44402001408` + `F4440300142100`.
2. Player self-appear (op 0x03 sub 0x03): `F444`+len+`03`+`le32(id)`+sex+ghost+god+`le16(map)`+`le16(MapX)`+`le16(MapY)`+dir+`le16(hair)`+color(8)+equipCount+equippedIds(`le16` each)+`0000000005`+reborn+job+namehex. `len = 33 + equipCountHex/2 + nameLen`.
3. Stats (op 0x05 sub 0x03): `F444`+len(`skillsHex/2 + 113`)+`0503`+thuoctinh+`le16(Hp)`+`le16(Sp)`+`le16(Int)`+`le16(Atk)`+`le16(Def)`+`le16(Agi)`+`le16(Hpx)`+`le16(Spx)`+lv+`le32(Texp)`+`le16(SkillPoint)`+`le16(Point)`+`le16(Tiengtam)`+`le16(HpMax)`+`le16(SpMax)`+`le32(Atk2)`+`le32(Def2)`+`le32(Int2)`+`le32(Agi2)`+`le32(Hpx2)`+`le32(Spx2)`+literal `F401F401F401F401F401`+90 zero bytes+skill list.
4. `SendPlayerOnline` broadcasts the appear frame for every online player + pet block(s).
5. Pet summary: `F444`+len(`petStats/2+2`)+`0F08` + per-pet entries + `F444`+len+`0F14` slots + `F44402000F0A`.
6. Party frames.
7. Active pet summon `F44406001301`+`le32(petid)`.
8. Pet stat recompute (no packet).
9. PK/war: `F44404002102`+`_My_Pk`+`_My_ThamChien`.
10. Inventory dumps in one `Sendpacket`: `F444`+len+`1705` (Homdo), `F444`+len+`1E01` (TienTrang), `F444`+len+`172F` (Tuideo), `F444`+len+`1766` (LuuLang).
11. Equipped `F444`+len+`170B`.
12. Gold `F4440A001A04`+`le16(gold)`+`00000000`.
13. Server name `F444`+len(`nameLen+11`)+`2709`+`le32(id)`+`C4000000`+nameLen+`strhex("TSVN")`.
14. `F44402000504F44402000F0A`.
15. `F4440A000B0B0000000000000020 40` (`F4440A000B0B0000000000002040`).
16. `F44402001F0F`.
17. Empty send (`""`) — nothing.
18. Time banner: `F444`+len(`6+msg`)+`020B00000000`+`strhex("Thoi gian: yyyy-MM-dd H:mm:ss")` (Vietnamese string via the smethod_17 table).
19. Welcome banner: same template, text `"TS offline RebuildVN Thanks: Duong Van Truong && Somchai choosawai"`.
20. Skill hotbar: `F444`+len+`2801`+`02`+`le16(skillId)`+slot per non-empty SkillSave row.
21. God/HP store/SP store: three × `F44412002304`+`le16(value)`+24 zeros.
22. DB cleanup; `_My_Logined=1`.

Capture-based golden tests (Chapter 9) verify this exact byte stream.

### 2.4.2 Packet catalogue (grouped by opcode)

**Opcode 0x01 (login):** `F4440300010901` hello; `F44402000106` wrong pass; `F4440300010300` create-character.

**Opcode 0x02 (misc UI):** `F44402000504` stop-move/warp; `F44402001408` end talk; `F44402001409` continue; `F44402001407` warp start; `F44402000F0A` pet terminator; `F44402000602` talk open; `F44402001F0A` sleep; `F44403001F0100` sleep done; `F44402001F09` pet can't stay; `F44402001F0C` stable close; `F44402001F07` hotel close; `F4440200170F` use-item end; `F44402001726` (item 46018); `F44402001732` storage end; `F44402002C01` reborn done; `F44402004101`/`02`; `F44402001302` pet recalled; `F44402001B03`.

**Opcode 0x03 (char/game):** `F4440300142100`; `F4440300090300/01`; `F44403001B0102` inventory full; `F44403001711`+slot equip; `F4440300210101` PK ack; `F4440300190203/04/07/09` trade results; `F4440300190B03/04/07/0A/0F` pet trade; `F44403000B0A01` battle-end marker; `F4440300230101/02/03`; `F4440300230202/03`; `F44403001D0900` open bank.

**Opcode 0x04:** player-appear frame (`SendPalyerOnline`, Server.cs:177); `F4440400210200/01`+thamchien; `F44404001709`+slot+count; `F44404001702`+slot; `F44404001710`+s1+s2; `F44404001717`+stt+slot; `F44404003505`+row+col; `F44404000B0100...`.

**Opcode 0x05 (stats/appearance):** `F44405001F06`+stt+`0000` stable-store; `F44405001F12010000`×4 slot frames; `F444050018…` party-disband frames; `F4440500170F2`+slot+`01` pickup; `F44405001707`+id+count item removed; `F44405000B01`+row+col+`00` cell cleared.

**Opcode 0x06 (movement/warp/pets):** `F44406001301`+pet; `F44406001A01`+gold; `F44406001A02`+gold; `F44406001D01`+bank; `F44406001D02`+bank; `F44406001D04`+bank balance; `F44406001104202`+points+`0100`; `F44406000F06`+id; party frames `F44406000D01/04/07/08/09/0B/0C`+id; `F44406001603`+warpid+`0A00`.

**Opcode 0x07:** `F44407002001`+id+action; `F44407002002`+id+action; `F44407000F02`+id+stt; `F44407000D0301`+id; `F4440700142C`+leader+`01`; `F44407001737`+id+rand; `F44407003501`+row+col+troiend+`0000`.

**Opcode 0x08 (stats/skills):** player stat update `F4440C000801`+type+sign(`01`/`02`)+`le32(abs value)`+`00000000`. Type ids (Type_Status): `19`Hp,`1A`Sp,`1B`Int,`1C`Atk,`1D`Def,`1E`Agi,`1F`Hpx,`20`Spx,`23`Lv,`24`TExp,`25`SkillPoint,`26`Point,`CF`Hpx2`D0`Spx2`D2`Atk2`D3`Def2`D4`Int2`D6`Agi2`3E`Tiengtam`40`Fai. Skill-learn `F4440C0008016E01`+`le32(lv)`+`le32(skill)`. Skillpoints `F4440C0008012501`+`le32(count)`+`00000000`. Party stat `F44410000803`+`le32(id)`+type+sign+`le32(v)`+`00000000`. Plus `F44408000B00`+id+`0000` (despawn), `F44408000505` (doll), `F44408000500`+`F628` (warp finish), `F44409001703`/`F44408001703` drop frames, `F44408003504` battle drop.

**Opcode 0x09 (char):** `F44402000901` created.

**Opcode 0x0B (battle):** covered in Chapter 6; board open `F4441C000BFA`+..., entity placement `F4440A000B0402`+id+`000003/...`; units `F4441A000B0503`(type3/7)/`F4441A000B0505`(type2/4)+warPacket.

**Opcode 0x0C (warp):** `F4440D000C`+`le32(id)`+`le16(map)`+`le16(x)`+`le16(y)`+`le16(dir)`; `F4440B000C`+id+map+x+y (appear).

**Opcode 0x0D (party):** `F4440A000D05`+leader+member; `F444`+len+`0D06`+leader+count+members list.

**Opcode 0x0F (pets):** `F4440C000F01`+id+stt+petid+`00/01`; `F444`+len+`0F07`+id+pet-following; `F444`+len+`0F08`+stats; `F444`+len+`0F14`+slots; `F444`+len+`0F09` pet rename.

**Opcode 0x12 (system):** `F44412002304`+`le16`+24 zeros (god, HP store, SP store).

**Opcode 0x14 (talk):** the `F444110014…` family — literal dialogs reproduced verbatim (see the response catalogue). Examples: merchant open `F44411001401000000010603`+id+`0000000000000100`/`0200`; home quest-check; talk frames in research §3.

**Opcode 0x17 (inventory):** `F444`+len+`1705` Homdo dump; `1706`; `170B` equipped; `1704` drops; `171E/1F/20/21` shops; `172F`; `1766`; `1E01`; `1E04`; `1708` item detail; `170D` compound; `09001703` drop owner; `08001703` drop map.

**Opcode 0x1A (gold):** `F4440A001A04`+`le16(gold)`+`00000000`.

**Opcode 0x27 (system):** `F444`+len+`2709`+`le32(id)`+`C4000000`+nameLen+name.

**Opcode 0x28 (hotkey):** `F444`+len+`2801`+`02`+`le16(skill)`+slot.

## 2.5 Response catalogue — literals to replicate verbatim

`F4440300010901`, `F44402000106`, `F4440300010300`, `F44402000901`, `F4440300090300`, `F4440300090301`, `F44402000504`, `F44402001408`, `F44402001409`, `F44402001407`, `F4440300142100`, `F44402000F0A`, `F44402000602`, `F44402001F0A`, `F44403001F0100`, `F44402001F09`, `F44402001F0C`, `F44402001F07`, `F44402001F0F`, `F4440200170F`, `F44402001726`, `F44402001732`, `F44402002C01`, `F44402004101`, `F44402004102`, `F44402001302`, `F44402001B03`, `F44403001D0900`, `F44402001D05`, `F44402001D06`, `F44403001B0102`, `F4440300210101`, `F4440300190203/04/07/09`, `F4440300190B03/04/07/0A/0F`, `F44403000B0A01`, `F4440300230101/02/03`, `F4440300230202/03`, `F4440400210200/01`+flag, `F44404002102`+pk+`00/01`, `F4440A000B0402`+id+`000003/02/05`, `F4440A000B0B0000000000002040`, `F4441C000BFA`, `F44413003201`+… (combo), `F44404003505`+row+col, `F44404000B01`+row+col, `F44405000B01`+row+col+`00`, `F44408000B00`, `F44407003501`+…, `F44408000505`+…, `F44408000500`+id+`F628`, `F4440A001A04`+gold, `F44406004202`+pts+`0100`, `F44412002304`+val+24×`00`, `F44406001A01/02`, `F44406001D01/02/04`, `F4440C000801`+…, `F44410000803`, `F444050018…`+…, `F44407000F02`, `F4440C000F01`, `F44406000F06`, `F4440E000F05`, `F44405001F06`, `F44406001301`, `F4440D000C`/`F4440B000C`, `F4440A000D05`, `F44407000D0301`, `F44406000D04`, and the full `F444110014…` set — the complete literal inventory is in research §4. For exclusions, rely on the golden captures (Chapter 9); do not invent packets.

## 2.6 Appendix A — FTalk.H6 menu engine

**H6 is not text** — it is a menu/action logic tree (~2,975 C# lines) that drives item changes, pet summon, shops, hotel/sleep, and quest flows. The text strings live in `Data_Talks` INI dialog packets (Chapter 6). Port H6 as the **data-driven table** + pre-dispatch rules + exceptions below, not as transcribed C#.

### 2.6.1 Pre-dispatch rules (applied by NPC-id / context first)

| Context | Behaviour (packets, in order) |
|---|---|
| Banker/store `16080/16004/16011/16023` | SelectMenu 30 → `F44403001D0900`+`F44406001D04`+bank+`F44402001D05`+`F44402001409`; 31 → `F44402001D06`+`F44402001409`; 40 → EndTalk |
| Inn/hotel `15002/16001/16016/15118` | SM30 → `F44411001401000000010603010000000000000100`; 31 → `Sleep()`+EndTalk; 32 → `OpenHotel()`; 33 → savemap + add item 46016×2 + EndTalk; 40 → EndTalk |
| NPC `16015` | SM30 → `…0200`; SM31 → Sleep+End+`method_2`; SM32 → OpenHotel; SM33 → savemap; SM40 → End |
| NPC `16012` | silent |

### 2.6.2 H6 data table + exceptions

The full compiled body (≈45 map cases, ~228 idtalking branches, 176 literal packets) and the **exceptions** — the daily-quest generator (map `12711`, `FTalk.cs:385-513`) with its **exactly 21 `random.Next` draws** (see below and research `06-battle-pass2.md` §(6)), pet-reborn `55002/59102/59011`, and the bespoke maps — are filed as the body of Appendix A in this repo's `spec/` addendum. The executor ports them verbatim; the golden harness (Chapter 9) is the diff-gate.

> **H6 RNG parity (mandatory):** the daily-quest block uses a **fresh, time-seeded `Random`** (NOT the three battle streams) and must consume exactly the 21 draws in the exact order given in research `06-battle-pass2.md` §(6) and the `spec/` H6 addendum — even those whose values the chosen menu branch later ignores.

### 2.6.3 Generic talk-data path (the majority)

For every non-special NPC the flow: on select → if `Data_Talks` missing → EndTalk; if the next dialog is the leading pattern and `_RequireSelectMenu` mismatches → `LoseDialogs[0]` or EndTalk; quest-requirement failure → `{…010107…}` or `…01 03`+id+`BB`; else `TalkMessages(dialog)`; when exhausted and a `[TEAMDEF]` exists → battle; specific branches (59k NPCs) emit dialog + reward packets (skill-learn, gold, equip). Transcript in research §2.13.

---

# Chapter 3 — Static Data Files

Normative: research `02-data-file-formats.md`. Keep `ts_server_old/Data/` byte-identical; load at runtime. Only player state goes to MySQL.

## 3.0 Load pipeline (startup order)

```
LoadDataItems → CreatMapItem → RemoveItemOnMap, ItemOnMapShow
            → LoadDataNpcs → CreatMapNpc → NpcOnMapWalk
              → LoadDataSkills → LoadDataWarps → LoadDataTalks → LoadDataTexps
              → LoadDataDolls
                              LoadDataWarps → LoadDataTalks → LoadDataBattleGates → Loaded()
```

Final `Loaded()` sets `DataLoaded=true` (accept gate). Order matters for *building* the tables; a deterministic single pass is allowed.

## 3.1 Common text-file conventions (`*.txt` except quests/Member.ini)

- Read lines with per-file encoding + BOM handling (§4); do **not** normalize names.
- `Split('\t')`; column index = element index; extra columns ignored.
- Row whose first element starts with `//` is a header/comment → skip (the `.txt` files have two `//` rows for Items/Npcs).
- **Termination rule:** on a line with `length <= 0` (empty), `break` — ignore everything after. (`Warps`/`BattleGate` instead stop on `text.Length < 5`.)
- A non-numeric field in a numeric column → load crash (reproduce as a load failure).
- **No defaults for missing columns** — every rendered row must have enough columns.

## 3.2 Tables, files, row counts

| In-memory | File | Rows |
|---|---|---|
| `Data_Npcs` | `Npcs.txt` | 6,673 |
| `Data_Items` | `Items.txt` | 8,376 |
| `Data_Skills` | `Skills.txt` | 392 |
| `Data_Warps` | `Warps.txt` | 4,994 (48 warps commented out) |
| `Data_Talks` | `Quests/*.ini` | 813 files |
| `Texps` | (computed, no file) | MaxLevel-1 |
| `Data_BattleGates` | `BattleGate.txt` | 68 |
| `NpcOnMap` | `NpcOnMap.txt` | 20,265 |
| `ItemOnMap` | `ItemOnMap.txt` | 1,161 |
| `ItemDropOnMap` | (computed per map 1..255 slots) | — |
| `Data_Dolls` | `Dolls.txt` | 98 |
| `dictionary_0/1` | never populated — port their **absence** only | — |

`EVe.txt`, `shopp`/`shopp.accdb`, and `Data_Client/ITEM.DAT`/`Npc.DAT` are **not loaded** by the server — exclude from the contract (documented for reference).

## 3.3 Column maps (authoritative)

### Items.txt (`LoadDataItems`)
Columns: `Id Name Level Hp Sp Int1 Atk1 Def1 Hpx1 Spx1 Agi1 Fai1 Int2 Atk2 Def2 Hpx2 Spx2 Agi2 Fai2 element elem_val equippos RbPetFrom RbPetTo AddPet` → fields `_id _Name _Lv _Hp _Sp _Int1 _Atk1 _Def1 _Hpx1 _Spx1 _Agi1 _Fai1 _Int2 _Atk2 _Def2 _Hpx2 _Spx2 _Agi2 _Fai2 _Thuoctinh _Value _Loai _RbPetFrom _RbPetTo _AddPet`. `Element`: 0=none,1=earth,2=water,3=fire,4=wind.

### Npcs.txt (`LoadDataNpcs`) — UTF-16LE+BOM, LF
`Id | Name | Level | Element | HpMax | SpMax | Hpx | Spx | Int | Atk | Def | Agi | Skill1..4 | Drop1..6 | NotPet | Reborn` → `_Id _Name _Lv _Thuoctinh _Hp _Sp _Hpx _Spx _Int _Atk _Def _Agi _Skill1..4 _Item1..6 _Bat _Reborn`.

### Skills.txt — proper UTF-8 (names server-GUI only, never in packets)
`Id | Name | Sp | Point | ThuocTinh | IdDK1..6 | LvMax | Type | DoManh | SLDanh | Reborn | Combo | Delay | TroiBuff`.

### Warps.txt — ASCII
`map1 | warpid | map2 | x | y`. Key `(map1, warpid)`. (`Warps._Battle` never loaded.)

### BattleGate.txt — ASCII
`Mapid1 | WarpId | Diahinh | 1..10` (the 10 defender NPC ids).

### Dolls.txt — ASCII
`DollId | NpcId`.

### NpcOnMap.txt — ASCII (map spawns/patrol list)
`MapId | Id | NpcId | X | Y | Coord | SoLuong`. If `SoLuong>0` the NPC joins the walk/patrol list (`NpcOnMapWalk` every 900 ms moves within `[X±Coord, Y±Coord]`, clamped ≥0).

### ItemOnMap.txt — ASCII
`MapId | Id(slot 1..255) | ItemId | X | Y | Delay`. First time a MapId appears, `ItemDropOnMap` slots 1..255 are created empty. Each row calls `SystemDropItem(map, slot, x, y, item, 999999)` — immediately spawns the static drop with `_Delay=999999` (never auto-removed) and broadcasts `F44408001703`+item+x+y to the map. Separate threads decremental the respawn counters.

### Quests/*.ini (`LoadDataTalks`, 813 files)
Only top-level `*.ini`. Win32 INI semantics are mandatory (§3.4). Sections found: `[BASE]`(813), `[REQUIRES]`(560), `[OnWin]`(551), `[OnLose]`(430), `[TEAMDEF]`(316), `[DESCRIPTION]`(170). `[BASE]` keys: `MapId`, `Type` (`"NPC"` 546 / `"WARP"` 267), `Id`, `Step`, `Dialogs` (tab-separated pre-built `F444…` packet hex). **Empty `Dialogs` + a `[TEAMDEF]` whose `Diahinh` is non-zero ⇒ a battle quest.** `[TEAMDEF]` keys `Npcs` = exactly 10 npc ids → `int[11]{diahinh, n1..n10}`. `[OnWin]` keys: `Dialogs`, `WarpTo`, `Rewards`, `RandomRewards`, `UseItems`, `SaveLeaderQuests`, `SaveMemberQuests`, `PlayerEnhanceData`, `AddSkill`, `AddPet`, `ClickNpcId`. Tuples use `-` separators; operator tokens (`=`,`>=`,`>`,`<=`,`<`,`!=`) in conditions.

## 3.4 Port-mandatory INI semantics

1. **Absent key → the literal string `"nothing"`** (the loader compares against this sentinel).
2. **Section and key matching is case-insensitive** (files write `[OnWin]`, code queries `"ONWIN"`).
3. Value buffer capped at **1024 chars**.
4. `Dialogs=` values are prebuilt packet hex — **forward verbatim**, splitting on the literal `"F444"` for `TalkMessages`.
5. **`[OnLose]` WarpTo is read from `ONWIN`** (C# copy-paste bug, `_LoseWarpTo == _WinWarpTo`) — replicate.
6. Key type: `Key_Talk = { MapId, Type, Id, Step }`; `Key_Warp = { map1, warpid }`; `Key_NpcOnMap = { MapId, Id }`; `Key_ItemOnMap = { MapId, ItemId, X, Y }`. Duplicate-key behaviour: mostly `Add` throws; keep the guarded/un-guarded parity (the current data has no duplicates on the unguarded tables).

## 3.5 Texps — computed (no file)

For level `i` in `0..MaxLevel-1` (`MaxLevel=200`): `Texp0[i] = Texp0[i-1] + int(Round(Pow(i+1, 2.9))) + 5`, `Texp1[i] = … (i+1, 3.0) … + 5`, `Texp2[i] = … (i+1, 3.05) … + 5` — cumulative total-EXP thresholds for reborn 0/1/2. Consumed by `TexpGetLvUp` (Chapter 6 §6.6).

## 3.6 Accounts — NOT from `Member.ini`

`accounts` now come from the MySQL `accounts` table only (web dashboard). Do **not** read or import `Member.ini` at bootstrap.

---

# Chapter 4 — Text Encoding Contract

Normative: `.scratch/rust-porting-spec/research/03-text-encoding.md`. **This chapter is the authority on Vietnamese text round-trip; read it before any code touches a name.**

## 4.1 Wire encoding — VISCII 1.1

The client speaks **VISCII 1.1**: every byte 0x00–0xFF maps to one Vietnamese character (single byte per character, with the standard C0 control-byte exotica: Ẳ=0x02, Ẵ=0x05, Ẫ=0x06, Ỷ=0x14, Ỹ=0x19, Ỵ=0x1E). The C# confirms this three independent ways (TextEncoder's VISCII table, `smethod_17`'s VISCII table, and `smethod_13`'s low-byte-per-char output).

**Wire invariance = bytes 0x00–0xFF.** Never transcode. Never "upgrade" to UTF-8 for the wire or for game-text DB columns.

## 4.2 Round-trip contract

- **In memory:** every name is a `Vec<u8>` of VISCII bytes. (A separate Unicode `String` for dashboard/log display may exist via the table below, but the wire/DB path uses the bytes.)
- **Load-time decode (per file):**
  - `Npcs.txt` — UTF-16LE with BOM (`FF FE`), LF only. Strip BOM, decode UTF-16LE, then apply the **reverse mojibake map** (§4.4) char-by-char → VISCII bytes.
  - `Items.txt` — UTF-8 (no BOM), CRLF. Decode UTF-8, apply the reverse map. The single genuine `ă` (U+0103) at item 48101: **normalize to VISCII `ă`=0xE5** (recommended) — do NOT feed it through the C# garble/abort path.
  - `Skills.txt` — UTF-8, proper Vietnamese; skill names are server-GUI **only**; never encode into packets.
  - `Warps`/`BattleGate`/`Dolls`/`NpcOnMap`/`ItemOnMap`/`Member.ini` — ASCII.
  - `Quests/*.ini` — parse raw; keys/ints/hex ASCII; `Dialogs=` opaque packet hex; `Title=` is byte BLOB (0xA0–0xEF, unidentified CID) — server-GUI only, keep raw, **never sent**.
- **Send-time (server→client):** each byte → 2 uppercase hex. Enforce the byte-string boundary so the VISCII control-char bytes survive (their hex pairs are valid). Length fields = byte count.
- **Server-authored text** (announcements, welcome banner, `/where`): map proper-Unicode Vietnamese → VISCII via the `smethod_17` positional table (§4.4) then hex. The strings actually used are the ASCII+VISCII subset; names `"TSVN"`, the welcome banner, time banner, use it.

## 4.3 Garble exceptions (bug-for-bug — intentional)

99 item + 23 NPC names contain CP1252 punctuation codepoints (`>0xFF`: `„` U+201E, `†` U+2020, `€`, `™`, `œ`). In C#, `smethod_13` emits them as hex groups and `smethod_4` parses each group into either **2 garbage bytes** (4-digit groups like `…→20 1E`) or **aborts the whole packet** (3-digit groups, e.g. `œ`→U+0153).

**Decision (issue 10): replicate the C# garble byte-for-byte**, so the Rust output diffs exactly against the captured C# traffic. Proper VISCII "fixing" would diverge and fail the harness. The git spec's "garble" appendix lists the 122 concrete names + the exact hex the C# emits so the executor knows these are intentional, not bugs. Display may be clean; the wire must carry the garble.

## 4.4 Tables you must implement

1. **VISCII byte→Unicode** (from `TextEncoder`, add `0xD0→Đ`, `0xDD→Đ` from `smethod_17`) — for display/DB.
2. **Reverse mojibake map** — mojibake char → VISCII byte. Given in §4.1 of research 03: ASCII passes; `0x80–0x9F` passes through as C1; `0xA0–0xFF` Latin-1 = byte; CP1252 punctuation in 0x80–0x9F maps to the VISCII byte (e.g. `„→80`, `†→86`, …). Only `ā` is unmappable.
3. **Unicode→VISCII positional table** (`smethod_17`'s `uni`/`enc` strings) for server-authored text.

These tables are spelled out verbatim in `research/03-text-encoding.md` §3.2 and §5.5. Import them character-for-character.

## 4.5 Q&A

- *Why not `utf8mb4` for game text columns?* — The wire is VISCII; `utf8mb4` only preserves bytes for the ASCII subset and actively converts/rejects VISCII high bytes (0x80–0xFF are invalid UTF-8). `utf8mb4` is acceptable only for metadata/dashboard text, **never** game text — see Chapter 5.
- *The single `ă` at item 48101?* — normalize to `0xE5` at load; it then round-trips.

## 4.6 Garble exceptions — the 122 names, byte-for-byte

The table below is generated from the actual `Data/Items.txt` and `Data/Npcs.txt`. For each garble name it gives the exact byte string the C# server's `smethod_13` builds and therefore what reaches the client: characters > 0xFF are hex-encoded as 4-digit groups (`201E` for „) which the 2-byte-at-a-time parser turns into **2 garbage bytes**, or as 3-digit groups (`153` for œ) which **abort the whole packet**. Replicate exactly; this is the bug-for-bug fidelity contract.

| Kind | Id | Extracted name (mojibake) | C# emits on wire (smethod_13) | effect |
|---|---|---|---|---|
| I | 18973 | `Thái „t binh pháp` | `5468E16920201E742062696E68207068E170` | 2 garbage bytes |
| I | 19706 | `Áo „u Bi` | `C16F20201E75204269` | 2 garbage bytes |
| I | 19710 | `Áo †n Phong` | `C16F2020206E2050686F6E67` | 2 garbage bytes |
| I | 19739 | `Áo †n thân` | `C16F2020206E207468E26E` | 2 garbage bytes |
| I | 20118 | `Thanh „t Trø` | `5468616E6820201E74205472F8` | 2 garbage bytes |
| I | 20714 | `Khån Phiêu †n ` | `4B68E56E20506869EA752020206E20` | 2 garbage bytes |
| I | 21430 | `Gång tay Phong „n` | `47E56E67207461792050686F6E6720201E6E` | 2 garbage bytes |
| I | 21612 | `Gång tay „p S½n ` | `47E56E672074617920201E702053BD6E20` | 2 garbage bytes |
| I | 23075 | `Ng÷c †n thân` | `4E67F7632020206E207468E26E` | 2 garbage bytes |
| I | 23255 | `†nPhânThân Huy Hi®u` | `20206E5068E26E5468E26E20487579204869AE75` | 2 garbage bytes |
| I | 25050 | `Bùa †n thân ` | `42F9612020206E207468E26E20` | 2 garbage bytes |
| I | 26087 | `Trái ‘i` | `5472E16920201869` | 2 garbage bytes |
| I | 29019 | `Ng÷c „n Truy«n Qu¯c ` | `4E67F76320201E6E2054727579AB6E205175AF6320` | 2 garbage bytes |
| I | 29026 | `„n ÐÕi Phong36` | `201E6E20D0D5692050686F6E673336` | 2 garbage bytes |
| I | 29144 | `Quan „n Hào Úy` | `5175616E20201E6E2048E06F20DA79` | 2 garbage bytes |
| I | 29145 | `Ð½n Du Kim „n` | `D0BD6E204475204B696D20201E6E` | 2 garbage bytes |
| I | 29194 | `Thái „t L®nh ti­n` | `5468E16920201E74204CAE6E68207469AD6E` | 2 garbage bytes |
| I | 31092 | `„n vÕn hµ h¥u` | `201E6E2076D56E2068B52068A575` | 2 garbage bytes |
| I | 34027 | `G² l¾n An L€c` | `47B2206CBE6E20416E204C20AC63` | 2 garbage bytes |
| I | 47447 | `KT „t Ð¸a Gia Lan` | `4B5420201E7420D0B86120476961204C616E` | 2 garbage bytes |
| I | 48101 | `BB Thái Văn C½ 3` | `4242205468E16920561036E2043BD2033` | aborts packet |
| I | 51297 | `TT „t Ð¸a Gia Lan` | `545420201E7420D0B86120476961204C616E` | 2 garbage bytes |
| I | 51501 | `Kim „n Tß·ng Cán` | `4B696D20201E6E2054DFB76E672043E16E` | 2 garbage bytes |
| I | 51502 | `Kim „n Tri®u Nga` | `4B696D20201E6E20547269AE75204E6761` | 2 garbage bytes |
| I | 51503 | `Kim „n Na LÜ` | `4B696D20201E6E204E61204CDC` | 2 garbage bytes |
| I | 51504 | `Kim „n C.TônToän` | `4B696D20201E6E20432E54F46E546FE46E` | 2 garbage bytes |
| I | 51505 | `Kim „n Diêm Ph¯` | `4B696D20201E6E204469EA6D205068AF` | 2 garbage bytes |
| I | 51506 | `Kim „n Tôn Thi®u` | `4B696D20201E6E2054F46E20546869AE75` | 2 garbage bytes |
| I | 51507 | `Kim „n Vån Sính` | `4B696D20201E6E2056E56E2053ED6E68` | 2 garbage bytes |
| I | 51508 | `Kim „n T× Th¸nh` | `4B696D20201E6E2054D7205468B86E68` | 2 garbage bytes |
| I | 51509 | `Kim „n Trình Bïnh` | `4B696D20201E6E205472EC6E682042EF6E68` | 2 garbage bytes |
| I | 51510 | `Kim „n Tào Phi` | `4B696D20201E6E2054E06F20506869` | 2 garbage bytes |
| I | 51511 | `Kim „n Lý Nho` | `4B696D20201E6E204CFD204E686F` | 2 garbage bytes |
| I | 51512 | `Kim „n Quách Vi®n` | `4B696D20201E6E205175E16368205669AE6E` | 2 garbage bytes |
| I | 51513 | `Kim „n Cao Lãm` | `4B696D20201E6E2043616F204CE36D` | 2 garbage bytes |
| I | 51514 | `Kim „n Gia Cát Quân` | `4B696D20201E6E204769612043E174205175E26E` | 2 garbage bytes |
| I | 51515 | `Kim „n Viên Thi®u` | `4B696D20201E6E205669EA6E20546869AE75` | 2 garbage bytes |
| I | 51516 | `Kim „n Trß½ng Nghi` | `4B696D20201E6E205472DFBD6E67204E676869` | 2 garbage bytes |
| I | 51517 | `Kim „n Thái Vån C½` | `4B696D20201E6E205468E1692056E56E2043BD` | 2 garbage bytes |
| I | 51519 | `Kim„n Trß½ngXuânHoa` | `4B696D201E6E205472DFBD6E675875E26E486F61` | 2 garbage bytes |
| I | 51521 | `Kim „n Ð£ng Ngäi` | `4B696D20201E6E20D0A36E67204E67E469` | 2 garbage bytes |
| I | 51522 | `Kim „n Læ Linh Kh·i` | `4B696D20201E6E204CE6204C696E68204B68B769` | 2 garbage bytes |
| I | 51523 | `Kim „n Vån ¿½ng` | `4B696D20201E6E2056E56E20BFBD6E67` | 2 garbage bytes |
| I | 51524 | `Kim „n Cao Thu§n` | `4B696D20201E6E2043616F20546875A76E` | 2 garbage bytes |
| I | 51525 | `Kim „n Mã T¡c` | `4B696D20201E6E204DE32054A163` | 2 garbage bytes |
| I | 51526 | `Kim „n Tào Nhân` | `4B696D20201E6E2054E06F204E68E26E` | 2 garbage bytes |
| I | 51527 | `Kim „n Chu Phù` | `4B696D20201E6E20436875205068F9` | 2 garbage bytes |
| I | 51528 | `Kim „n Pháp Chính` | `4B696D20201E6E205068E170204368ED6E68` | 2 garbage bytes |
| I | 51529 | `Kim „n B° Nguyên` | `4B696D20201E6E2042B0204E677579EA6E` | 2 garbage bytes |
| I | 51530 | `Kim „n Trß½ng Giác` | `4B696D20201E6E205472DFBD6E67204769E163` | 2 garbage bytes |
| I | 51531 | `Kim „n Trß½ngLß½ng` | `4B696D20201E6E205472DFBD6E674CDFBD6E67` | 2 garbage bytes |
| I | 51532 | `Kim „n Trß½ng Bäo` | `4B696D20201E6E205472DFBD6E672042E46F` | 2 garbage bytes |
| I | 51533 | `Kim „n Tôn Dñc` | `4B696D20201E6E2054F46E2044F163` | 2 garbage bytes |
| I | 51534 | `Kim „n Quan Bình` | `4B696D20201E6E205175616E2042EC6E68` | 2 garbage bytes |
| I | 51535 | `Kim „n HoàngN.Anh` | `4B696D20201E6E20486FE06E674E2E416E68` | 2 garbage bytes |
| I | 51536 | `Kim „n Trß½ng Hþp` | `4B696D20201E6E205472DFBD6E672048FE70` | 2 garbage bytes |
| I | 51537 | `Kim „n Trình Døc` | `4B696D20201E6E205472EC6E682044F863` | 2 garbage bytes |
| I | 51538 | `Kim „n Chu Thß½ng` | `4B696D20201E6E20436875205468DFBD6E67` | 2 garbage bytes |
| I | 51539 | `Kim „n Trß½ng Hoành` | `4B696D20201E6E205472DFBD6E6720486FE06E68` | 2 garbage bytes |
| I | 51540 | `Kim „n Bàng Døc` | `4B696D20201E6E2042E06E672044F863` | 2 garbage bytes |
| I | 51541 | `Kim „n HÑa ChØ` | `4B696D20201E6E2048D161204368D8` | 2 garbage bytes |
| I | 51542 | `Kim „n Tào Xung` | `4B696D20201E6E2054E06F2058756E67` | 2 garbage bytes |
| I | 51543 | `Kim „n Ngøy Diên` | `4B696D20201E6E204E67F879204469EA6E` | 2 garbage bytes |
| I | 51544 | `Kim „n TQ.Nguyên` | `4B696D20201E6E2054512E4E677579EA6E` | 2 garbage bytes |
| I | 51545 | `Kim „n Chu Du` | `4B696D20201E6E20436875204475` | 2 garbage bytes |
| I | 51546 | `Kim „n Trß½ng Liêu` | `4B696D20201E6E205472DFBD6E67204C69EA75` | 2 garbage bytes |
| I | 51547 | `Kim „n Hoàng Trung` | `4B696D20201E6E20486FE06E67205472756E67` | 2 garbage bytes |
| I | 51548 | `Kim „n KÖ Linh` | `4B696D20201E6E204BD6204C696E68` | 2 garbage bytes |
| I | 51549 | `Kim „n Vß½ng D¸` | `4B696D20201E6E2056DFBD6E672044B8` | 2 garbage bytes |
| I | 51550 | `Kim „n Lßu B¸` | `4B696D20201E6E204CDF752042B8` | 2 garbage bytes |
| I | 51551 | `Kim „n Quan Vû` | `4B696D20201E6E205175616E2056FB` | 2 garbage bytes |
| I | 51552 | `Kim „n Trß½ng Phi` | `4B696D20201E6E205472DFBD6E6720506869` | 2 garbage bytes |
| I | 51553 | `Kim „n Cát Bình` | `4B696D20201E6E2043E1742042EC6E68` | 2 garbage bytes |
| I | 51555 | `Kim „n Tôn Sách` | `4B696D20201E6E2054F46E2053E16368` | 2 garbage bytes |
| I | 51557 | `Kim „n Ti¬u Ki«u` | `4B696D20201E6E205469AC75204B69AB75` | 2 garbage bytes |
| I | 51558 | `Kim „n ÐÕi Ki«u` | `4B696D20201E6E20D0D569204B69AB75` | 2 garbage bytes |
| I | 51559 | `Kim „n T× ThÑ` | `4B696D20201E6E2054D7205468D1` | 2 garbage bytes |
| I | 51560 | `Kim „n Th¦m Ph¯i` | `4B696D20201E6E205468A66D205068AF69` | 2 garbage bytes |
| I | 51566 | `Kim „n Tr¥n Cung` | `4B696D20201E6E205472A56E2043756E67` | 2 garbage bytes |
| I | 51567 | `Kim „n Mã ÐÕi` | `4B696D20201E6E204DE320D0D569` | 2 garbage bytes |
| I | 51570 | `Kim „n Tôn Quy«n` | `4B696D20201E6E2054F46E20517579AB6E` | 2 garbage bytes |
| I | 51571 | `Kim „n Bàng ÐÑc` | `4B696D20201E6E2042E06E6720D0D163` | 2 garbage bytes |
| I | 51572 | `Kim „n Tuân Úc` | `4B696D20201E6E205475E26E20DA63` | 2 garbage bytes |
| I | 51573 | `Kim „n Quách Gia` | `4B696D20201E6E205175E1636820476961` | 2 garbage bytes |
| I | 51574 | `Kim „n Tôn Kiên` | `4B696D20201E6E2054F46E204B69EA6E` | 2 garbage bytes |
| I | 51575 | `Kim „n Chân M§t` | `4B696D20201E6E204368E26E204DA774` | 2 garbage bytes |
| I | 51576 | `Kim „n Tào Tháo` | `4B696D20201E6E2054E06F205468E16F` | 2 garbage bytes |
| I | 51577 | `Kim „n Mã Siêu` | `4B696D20201E6E204DE3205369EA75` | 2 garbage bytes |
| I | 51578 | `Kim „n L² TÕp` | `4B696D20201E6E204CB22054D570` | 2 garbage bytes |
| I | 51579 | `Kim „n Thß Thø` | `4B696D20201E6E205468DF205468F8` | 2 garbage bytes |
| I | 51580 | `Kim „n Ði¬n Vi` | `4B696D20201E6E20D069AC6E205669` | 2 garbage bytes |
| I | 51581 | `Kim „n Tß·ng Tª` | `4B696D20201E6E2054DFB76E672054AA` | 2 garbage bytes |
| I | 51583 | `Kim „n Khäo L²` | `4B696D20201E6E204B68E46F204CB2` | 2 garbage bytes |
| I | 51584 | `Kim „n Na Âu` | `4B696D20201E6E204E6120C275` | 2 garbage bytes |
| I | 51658 | `KA „t Ð¸a Gia Lan` | `4B4120201E7420D0B86120476961204C616E` | 2 garbage bytes |
| I | 56025 | `„n ð±i v§n` | `201E6E20F0B1692076A76E` | 2 garbage bytes |
| I | 58123 | `„t Ð¸a Gia Lan Mê` | `201E7420D0B86120476961204C616E204DEA` | 2 garbage bytes |
| I | 62712 | `CÑu Pháp œng` | `43D175205068E170201536E67` | aborts packet |
| I | 62856 | `Læ Qu¯c DÕ ™an` | `4CE6205175AF632044D5202122616E` | 2 garbage bytes |
| N | 14211 | `Quái †n Sî` | `5175E1692020206E2053EE` | 2 garbage bytes |
| N | 14558 | `T× „p` | `54D720201E70` | 2 garbage bytes |
| N | 14606 | `„t Ð¸a Gia Lan` | `201E7420D0B86120476961204C616E` | 2 garbage bytes |
| N | 14647 | `†n giä VÕn An` | `20206E206769E42056D56E20416E` | 2 garbage bytes |
| N | 16028 | `NhàBuôn D¤u „n` | `4E68E04275F46E2044A47520201E6E` | 2 garbage bytes |
| N | 16029 | `NhàBuôn D¤u „n` | `4E68E04275F46E2044A47520201E6E` | 2 garbage bytes |
| N | 17166 | `Lão †n Sî` | `4CE36F2020206E2053EE` | 2 garbage bytes |
| N | 17287 | `„p Lâu Man` | `201E70204CE275204D616E` | 2 garbage bytes |
| N | 17326 | `Th.Lâu †n Sî` | `54682E4CE2752020206E2053EE` | 2 garbage bytes |
| N | 17327 | `Thü Tiªt †n Sî` | `5468FC205469AA742020206E2053EE` | 2 garbage bytes |
| N | 17345 | `†n Ðà Thü` | `20206E20D0E0205468FC` | 2 garbage bytes |
| N | 40065 | `„u thú Phì Di ` | `201E75207468FA205068EC20446920` | 2 garbage bytes |
| N | 40068 | `„uthú Th.Hoàng` | `201E757468FA2054682E486FE06E67` | 2 garbage bytes |
| N | 40069 | `„uthú th.hoàng` | `201E757468FA2074682E686FE06E67` | 2 garbage bytes |
| N | 40119 | `Ð±ng Š` | `D0B16E6720160` | aborts packet |
| N | 41447 | `„t Ð¸a Gia Lan` | `201E7420D0B86120476961204C616E` | 2 garbage bytes |
| N | 45291 | `„t Ð¸a Gia Lan` | `201E7420D0B86120476961204C616E` | 2 garbage bytes |
| N | 47152 | `Thái„tChânNhân` | `5468E169201E744368E26E4E68E26E` | 2 garbage bytes |
| N | 47211 | `TÖ „n` | `54D620201E6E` | 2 garbage bytes |
| N | 47212 | `KÖ „n` | `4BD620201E6E` | 2 garbage bytes |
| N | 50206 | `„t Ð¸a Gia Lan` | `201E7420D0B86120476961204C616E` | 2 garbage bytes |
| N | 61141 | `„t Ð¸a Gia Lan` | `201E7420D0B86120476961204C616E` | 2 garbage bytes |
| N | 61277 | `(†n danh)` | `2820206E2064616E6829` | 2 garbage bytes |

---

# Chapter 5 — Database (MySQL 8)

Normative: research `05-mysql-8.md` + issue 11. This supersedes the old SQLite schema (issue 05, kept only as historical reference).

## 5.1 Shape

One database **`ts_dream`** (InnoDB) on `localhost:3306`. MySQL is an external service you bring up once; the binary holds a connection pool. Connect with `MySqlPool::connect("mysql://user:pass@localhost:3306/ts_dream")`, `max_connections(n)`; **set connection `charset = latin1`** so the client/server layer never transcodes stored names.

## 5.2 Character set / byte preservation (critical)

- Wire is VISCII; names must round-trip as single bytes through MySQL.
- The server default (`character-set-server=utf8mb4`) and the `ts_dream` database both inherit `utf8mb4`; a bare `VARCHAR` column falls back to `utf8mb4`, and VISCII high bytes (0x80–0xFF) are invalid UTF-8 → corrupted/invalid.
- **Therefore the DDL MUST declare the charset explicitly**: `VARCHAR(n) CHARACTER SET latin1` (+ recommended `COLLATE latin1_bin`) on the table DEFAULT or on every game-text column. Do **not** rely on server defaults.
- `utf8mb4` is valid only for metadata/dashboard text, never game text.
- Do **not** convert to utf8/utf8mb4 "for better Vietnamese" — the wire is VISCII; latin1 is the byte-preserving store.

## 5.3 Table layout — shared, all-in-one

| (Superseded SQLite design) | MySQL 8 design |
|---|---|
| `account.db`: `Player`+`accounts`; one `member/vn{id}.db` per char | one DB `ts_dream`: `players` + `accounts` + the same 9 gameplay tables + `item_code`, all shared, composite PK includes `player_id` |
| template binary copy + seed SkillSave | character creation = one transaction of INSERTs (explicit `player_id`, seeded SkillSave) |
| PRAGMA FK/WAL | InnoDB defaults; no FK (parity with Access); connection pool |

Access `DOUBLE` columns → `BIGINT` (all observed values are integral).

### Tables
- `players` — one row per created character (its id doubles as the account/login id).
- `accounts` — credential pairs (id, pass1, pass2), created only through the web dashboard.
- **9 gameplay tables**: `homdo`, `tientrang`, `luulang`, `pet`, `quest`, `skill`, `skillsave`, `trangbi`, `tuideo` — every row carries `player_id`.
- `item_code` — redeem codes (op 0x23 sub 3).

### Column types & defaults
- All numeric Access columns → `BIGINT`.
- **DEFAULT values kept verbatim** (`ShopPoint 0`, `SP_Store`/`HP_Store` `10000`, `DTT/TLP/TCP/TTP/savemap/tanthu/phien/PTS 0`) — the C# INSERT does not supply them; they rely on DEFAULT.
- **No FOREIGN KEY, no NOT NULL** beyond what Access declares — C# uses no FKs, and adding them blocks legal behaviour (e.g. `Homdo.Id` can point at a nonexistent item, `Pet.Idskill=0`).
- Text columns → `VARCHAR(n) CHARACTER SET latin1 [COLLATE latin1_bin]`.
- `accounts.id BIGINT AUTO_INCREMENT PRIMARY KEY`.

### Composite PKs (per-player isolation replaces the old per-file PK)

| Table | PK |
|---|---|
| Homdo, LuuLang, TienTrang, Trangbi, Tuideo | `(player_id, slot)` |
| Pet | `(player_id, stt)` |
| Skill | `(player_id, Id)` |
| SkillSave | `(player_id, ID)` |
| Quest | **no PK** — keep a KEY on `QuestId` (Access has none; do not invent NOT NULL/UNIQUE) |

### Indexes (KEY form)
`KEY players(MapId)`, `KEY pet(IdSkill1..4)`, `KEY quest(QuestId)`, `KEY skillsave(IdSkill)`.

## 5.4 Scoping contract (MANDATORY)

Because the schema is shared (per-player isolation is gone), **every** C# statement over the 9 gameplay tables must be ported with a `player_id` predicate (or a composite-PK that includes `player_id`). Porting verbatim is a bug. The dangerous C# patterns to remember:

1. `SkillSaveGetId` / `SkillSaveUpdateId` — `SELECT`/`UPDATE SkillSave WHERE Id = n` (`SkillSave.Id` 1..10 repeats for every player) → add `AND player_id = ?`.
2. Login `DELETE`-and-rebuild of `Skill` — `DELETE FROM Skill WHERE Id >= 10001 AND Id <= 13033` and `WHERE Id >= 0 AND Id <= 9` → each must carry `player_id`.
3. `DELETE FROM Quest WHERE MapId = …` (FTalk quest-step resets) → add `AND player_id = ?`.

Every reward/consumption statement in `BattleQuestWin` (§6.7) must carry `player_id` too.

## 5.5 `item_code` table + redeem (functional, no degrade)

C# surface `Client.cs:7571-7659` (op 0x23 sub 3). MySQL is mandatory ⇒ there is **no "no-DB" degrade branch**; port the fully functional redeem.

DDL:
```sql
CREATE TABLE item_code (
  code       VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
  password   VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
  player_id  BIGINT NOT NULL DEFAULT 0,
  used_at    BIGINT NULL,          -- unix seconds
  item_id    BIGINT,
  count      BIGINT
) ENGINE=InnoDB;
```

Redeem semantics:
- **Never concatenate** client-supplied `code`/`password` into SQL — always use bind parameters (sqlx).
- Wrap in a **transaction**: `SELECT * FROM item_code WHERE code=? AND password=? AND player_id=0`; if a row exists, grant `item_id` × `count`, then `UPDATE … SET player_id=?, used_at=? WHERE code=? AND password=? AND player_id=0` and check **rowcount == 1** to guard against concurrent double-redeem (race).
- If already used (rowcount 0 or `player_id != 0`) → red message.

The hardcoded `TSVN123`/`TSVN456` special gift (item 46197 + 20711 + 19711 + 23549 + 11001) and the once-only `tanthu` flag stay as in C#.

## 5.6 Character creation — one atomic transaction (opcode 0x09 sub 1)

1. `INSERT INTO players` (computed stats; other columns rely on DEFAULT; explicit `player_id`).
2. `INSERT INTO SkillSave` rows `Id` 1..10, `IdSkill=0` (**mandatory seed** — C# never INSERTs SkillSave, only UPDATEs).
3. `Skill` table build/DELETE as C# does.
4. Update `accounts.pass1/pass2`.

Login still runs the DELETE+rebuild of `Skill` as C# does.

## 5.7 Bootstrap / migrations

- `sqlx::migrate!("./migrations")` runs at boot **before** binding any listener.
- **Fail-fast:** if MySQL is unreachable or a migration fails → **hard exit** with a clear diagnostic; the HTTP dashboard must never run with a dead DB.
- `item_code` and `accounts` are created by the migration.

## 5.8 Accounts

- **Accounts are created exclusively through the web dashboard** (`POST /api/accounts`). There is **no import of `Member.ini`** at bootstrap.
- `accounts(id BIGINT AUTO_INCREMENT PRIMARY KEY, pass1 VARCHAR(64) CHARACTER SET latin1 NOT NULL, pass2 VARCHAR(64) CHARACTER SET latin1 NOT NULL)`.
- On create, read the new id via `last_insert_id()` — do **not** use `max+1` (race under concurrent creation).
- Keep `pass1`/`pass2` **plaintext** (parity with the C# server).

## 5.9 Config impact

`database_url = "mysql://user:pass@localhost:3306/ts_dream"`, env key `TS_DATABASE_URL`. The old SQLite keys `account_db_path` / `member_dir` / `template_db_path` are **removed** (Chapter 8).

---

# Chapter 6 — Battle Engine

Normative: `.scratch/rust-porting-spec/research/04-battle-engine.md` + `06-battle-pass2.md`. Reimplement the *behaviour*, not the race-tolerant C# mechanism.

## 6.0 Domain objects

- **`WarInfo`** — one grid cell: `_Type,_Id,_IdNpcOnMap,_IdChar,_Row,_Column,_HpMax,_SpMax,_Hp,_Sp,_Lv,_Thuoctinh,_LeaderId,_IdSkill,_RowAttack,_ColumnAttack,_Int,_Atk,_Def,_Agi,_Reborn,_Team`, buff/debuff triplets (`_Type3/_Type4/_Type15/_Type19` = id, level, remaining turns), `_Attacked,_Random,_Exp,_Packet`. `_Packet` is the 23-byte entity snapshot (§6.4).
- **Grid `ListWar`**: 20 cells keyed `row-col` (`hex(row)+hex(col)`), row 0..3, col 0..4; `_keys` preserves creation order (row-major).
- **`ListQS`**: 50 slots (keys 1..50) for join-in-progress and leader SP regen.
- **Types**: `2`=player, `3`=hostile npc, `4`=pet, `7`=TeamDef npc. Types 3/7 are "npc-like": never get DB HP/SP writes, never get exp, are catchable.
- **Three independent RNG streams** (port `.NET`-style: global, time-seeded):
  - `random_0` — drop rolls, skill pick, `RandomizeArrayWithPercent`.
  - `random_1` — per-turn `_Random` tie-breaker, damage jitter `Next(0,2)`.
  - `random_2` — npc respawn coordinates.
  Port three independent streams; never merge them.

## 6.1 Battle construction

| Trigger | Constructor | Diahinh | Note |
|---|---|---|---|
| PK (op 0x0B sub 2 sub 2) | `TheBattle(Id1, Id2, 112)` | `112` | AddToBattle team1 (3,2), then team2 (0,2) |
| NPC attack (0x0B s2 s3) | `TheBattle(Id1,npcId,onMap,112)` | `112` | leader (3,2), boss (0,2) **Type 3** |
| Quest/TeamDef (FTalk) | `TheBattle(Id, teamdef[], TeamDef[0])` | `TeamDef[0]` | id1..id10 → row0/row1 cols 0..4, **Type 7** |
| Active-NPC (SoLuong≥3) | `…,4712` | `4712` | teamdef built from the SoLuong |

**IdBattle** = `Server.IdBattleCount` (starts 1, incremented at creation; assignment before increment).

`AddToBattle(leaderId, mem1..4, row, col)`: team `= (row==0 ? 2 : 1)`; load HP/SP/etc; sum `Int2..` for Int/Atk/Def/Agi. Leader (col==2) additionally loads up to 4 pets at `(row^1, col)` from stt `SttPet..+3`, Type 4, `_IdChar=owner`. Each member (cols 1,3,0,4) loads one pet at `(row^1, col)`. Members overwrite leader pet cells in dict-insertion order for the ones that overlap.

`AddNPCToBattle`: fills the cell with `Type`, npc id, stats from the `Npcs` record; `Team=2`; no pets.

`ChangedWar` rebuilds `_Packet` = `Type:X2 | le32(Id) | le16(IdNpcOnMap) | le32(IdChar) | row, col | le16(HpMax) | le16(SpMax) | le16(Hp) | le16(Sp) | lv, TT` — **total 23 bytes**.

## 6.2 Turn engine (`Battling`, per-battle async task)

Loop:
1. **Win/lose check:** all enemy cells (rows 0–1) dead → player win (break). All player cells (rows 2–3) dead → `num=2` lose.
2. **Reset & buff ticks** (per cell, grid order): reset `_IdSkill/_RowAttack/_ColumnAttack/_Attacked=0`; `_Random = random_1.Next(0,100)`; accumulate team avg while `num2==0`; decrement buff timers; apply burn (Type3 10004 `10+lv*2`, 10033 `30+lv*10`) / poison (Type15 14015 `30+lv*15`) with DB write for non-npc and broadcast `{3201}`; buff-end clears; turn prompt `F44402003401` for players.
3. **Input wait (≤ ~21 s):** poll every 100 ms until all `_Attacked==1`; auto-actions for npc (pick skill via `GetRandomSkillNPC`), berserking, etc.; players submit via opcode 0x32. Force `_Attacked=1` after the window.
4. **Turn order:** sort cells `Attacked DESC, Agi DESC, Random DESC`; iterate.
5. **Action execution** with the skill-type dispatch + damage pipeline (§6.5).

## 6.3 Turn action frame (`{…3201}`)

Per-entity block = `le16(blockLen)` + row + col + `le16(skillId)` + SLdanh + count + effects; concatenated into `text9`; flushed per the phase-4 buffer logic with a `Delay` ms sleep. Reflect-damage accumulator appended when present; drop/exp processed batch after each flush. Keep the `text2` combo footer and `text11` reflect ordering.

## 6.4 Targeting (no terrain influence)

SLDanh expansion table and variant pickers are in research §4. Team-rule summary:

| Picker | Applies to |
|---|---|
| `GetPosAttack` | hostile (different team) + no Type4 13005/13025/13032 |
| `GetPosAttackCombo` | no HP requirement on the anchor, else same |
| `GetPosAttackTG` | hostile, no Type4 exclusion |
| `GetPosAttack3_15` | hostile + Type4 exclusion |
| `GetPosAttack_GiaiTru` | any `_Id>0` |
| `GetPosAttack_Type4` | own team |
| `GetPosAttack_honLoan` | own team (berserk splash) |

`_Diahinh` has **no** effect on targeting or damage — it is echoed only into the setup packet.

## 6.5 Damage pipeline (skill Type 1 and Type 2)

- All arithmetic on `double` with `Math.Round` (banker's rounding → `round_to_even`) then cast to int; integer divisions where the C# uses ints.
- **Type 1 (physical):** `num36 = Round(Atk * Element(attTT,defTT) * 2.0 - Def * 1.6)`; `+= Round((attLv-defLv)/1.5) + Round(attLv/20)*8`; `+= Round(GetDamageSkillInt(skillTT,defTT) * DoManh * (1.0 + skillLv*0.033))`; `*= num37` (2.0, or 2.6 during combo). Then the ordered buff modifiers (attack/def buffs, element relation), AoE roll `num36/=num34*0.75` when `num34>1`. Hit roll `GetRandomMissAttack` (percent = 100 + Round((lv diff)/10)×2 …; roll via `RandomizeWithPercent(1, 0, min(p,100))`). On miss: `num36=0` + counter/def packets. On hit: min `1`, add `random_1.Next(0,2)`; absorb/reflect (def/`13003`), attacker debuffs. Type 3/7 skip DB writes; players/pets get DB writes with clamped values.
- **Type 2 (magic):** uses `_Int`, no `num37`.
- **Element tables** (research §5.1): `GetDamageThuoctinh` matrix; `GetDamageSkillInt` per-level additive.
- **Status types (3,4,15,19), Type 11 catch, Type 12 flee, Type 14 heal, Type 16/18 cleanse** — per research §5.3–§5.9, incl. buff-effect bytes.

## 6.6 HP/SP/EXP — resolved exactly

**`getHpMax(rb, job, lvl, hpx)`** (Data.cs:5537):
```
rb 0   : floor((lvl^0.35 + 1.0)*hpx*2.0 + 80.0  + lvl)
rb 1   : floor((lvl^0.35 + 2.0)*hpx*2.0 + 180.0 + lvl)
rb>=2 job1 : floor((lvl^0.35 * 2.0 + 25.0)*hpx + 280.0 + lvl)
job2 : floor((lvl^0.35 * 3.0 + 30.0)*hpx + 380.0 + lvl)
job3 : floor((lvl^0.35 + 11.5)  *hpx*2.0 + 180.0 + lvl)
other: floor((lvl^0.35 + 10.5)  *hpx*2.0 + 180.0 + lvl)
```
**`getSpMax(rb, job, lvl, spx)`** (Data.cs:5553): rb0 / rb1 : `floor(lvl^0.25 * spx*2.0 + 60.0/110.0 + lvl)`; rb≥2 job1/2 `*2.0 +160.0`, job3 `*3.0 +310.0`, other `*3.5 +410.0` (+`lvl`). Note: the exponents (`0.35`, `0.25`) must match exactly.

**Exp — `TexpGetLvUp(lv, reborn, texp)`** (loop over `Texps[]`, not a closed form):
```
result = 0
if lvl < MaxLevel:
  for i in lvl .. MaxLevel-1:
    if texp < Texps[i]._{reborn} : return result
    if texp >= Texps[i]._{reborn} : result = i - lvl + 1
return result
```
Returns the number of level-ups. `MaxLevel=200`.

## 6.7 BattleQuestWin — full ordered side effects

Precondition: the talk must exist. Order (bind all DB writes with `player_id`):
1. Consume required items.
2. Red message.
3. Each guaranteed `WinRewards`.
4. **One random** `WinRandomRewards` via a **fresh independent `Random`** (NOT battle streams).
5. Grant `{itemId,count}` to leader + (if `shareToParty>0`) each of `_IdMem1..4`.
6. Use-items: `target==0` (self) sends `…0617030011`+slot, use, status update, equip; else pet path.
7. Save leader quests (`QuestUpdateDataNpc`/`Warp`).
8. Player enhance delta.
9. Add skill (validate id/skill-point/exists) — learn packet `F4440C0008016E01`+lv+skill + red "skill learned".
10. Add pet.
11. Warp/end: `WinWarpTo` → `Warped(...)` leader + members, broadcast `F44408000B00`; or `F44402001408`.

## 6.8 Battle packets — the byte-faithful set

Key frames: open board `F4441C000BFA`+`le16(DiaHinh)`+text; entity-snapshot `F4441A000B0503`/`0505`+`_Packet`; show-on-map `F4440A000B0402`+`le32(id)`+tail; hide `F44408000B00`+id+`0000`; reposition `F44405000B01`+row+col+`00`; clear pet `F44404000B01`+(row^1)+col; your-turn `F44402003401`; acting `F44404003505`+row+col; action `{…3201}` block; skillcast `F444130032010F00`…; buff end `F44407003501`+row+col+troiend+`0000`; drop `F44408003504`+…; status `F4440C000801`+type+…; battle-end `F44403000B0A01`. Transport: same XOR 0xAD framing, hex-parse → XOR → write.

`BattleGate.txt` exception: for warp-triggered team `Data_BattleGates` battles, DiaHinh comes from the gate file (10 monsters).

## 6.11 DiaHinh summary

`DiaHinh=112` for PK and normal NPC battles; `4712` for active-NPC (SoLuong) battles; a quest battle uses `TeamDef[0]`; the PK-member open frame uses the fixed value `7000` in place of DiaHinh (`SendBattleMemberPlayerPK`).

## 6.12 Notes / traps

- **`.NET` rounding** (banker's) on every double → int.
- **Division**: int division on the int expressions; double division on `/(n*0.75)`.
- **Avg level** double division; round on miss/exp inputs.
- **`num37` combo stacking**.
- **DB writes** order vs local mutation (clamped), then status packets.
- Keep battle state on one async task (race-free, deterministic).

---

# Chapter 7 — Web Dashboard

Standalone HTTP server on `0.0.0.0:8090`, running **always** once the DB is reachable. It talks to the game server through a shared `Arc<RwLock<AppState>>` + a `tokio::sync::broadcast` channel. A single dashboard page (server-rendered via askama/HTMX) + JSON API + SSE.

## 7.1 Authentication

None — the dashboard is open, no login.

## 7.2 Shared AppState

`Arc<RwLock<AppState>>` fields:
- `online: Vec<OnlineEntry { id, name, ip }>`.
- `running: bool`.
- `perexp: u32` (runtime; default 0; not persisted).
- `log_buffer`: ring buffer of the last **500** log lines (so a reconnect/reload still sees recent history).
- `broadcast: tokio::sync::broadcast<LogEvent>` for log + server-status fan-out (SSE multi-subscriber).

## 7.3 Routes & behaviour

| Method/Path | Behaviour |
|---|---|
| `GET /` | dashboard page: online list + start/stop/announce controls + live log + account + NPC list |
| `GET /api/server/status` | `{"running": bool}` |
| `POST /api/server/start` | bind listener `:6414` and accept (if not already running) |
| `POST /api/server/stop` | 5-second countdown broadcasting `020C` "Server will be closed in N second(s)" (keeps the C# `method_2` countdown), then close every client socket and the listener; HTTP stays up |
| `POST /api/server/announce` | `{text}` → opcode 0x02 sub 0x0C sent to all (server chat) |
| `GET /api/accounts` | list `[{id, pass1, pass2}]` from the `accounts` table |
| `POST /api/accounts` | create account `{pass1, pass2}` → INSERT `accounts`; return new id via `last_insert_id()` |
| `GET /api/npcs` | in-memory NPC list (from `Data_Npcs`) |
| `GET /api/online` | `[{id, name, ip}]` (columns ID/PlayerName/IP) |
| `GET /api/log/stream` | SSE |
| `POST /api/config/perexp` | `{value}` → set runtime `AppState.perexp` (not persisted) |

Behaviour when the server is not running: `status` → `{running:false}`; `start` allowed; `stop` when not running → **409** + reason; `announce` when not running → **409**. `perexp` is still settable (it's just a number). HTTP always serves the dashboard so you can press Start.

## 7.4 SSE schema

One event type:
```
event: log
data: { "level": "...", "ts": 1234567890, "msg": "..." }
```
Levels (8): `log`, `system`, `warning`, `packet`, `error`, `debug`, `c2s`, `s2c`.

## 7.5 Live packet log (new Rust feature, per decision)

Log **every frame hex in full** (e.g. `F444…`, already after XOR) with direction and player id. Note the C# never called the C2S/S2C level helpers — this is an intentional addition for the dashboard, matching the standing dashboard/logging decision. Use the ring buffer (500 lines) + broadcast.

## 7.6 Stop server

Closes the listener and kicks all clients; HTTP 8090 stays alive; `running=false`; a later Start re-binds.

---

# Chapter 8 — Config

## 8.1 File + env

- A TOML file `ts_dream.toml` (default) with per-key **environment overrides** (env key = `TS_` + KEY unless noted). Loaded once at boot (Chapter 1 §1.3).

| Key | Default | Env | Meaning |
|---|---|---|---|
| `game_port` | `6414` | `TS_GAME_PORT` | TCP game listener |
| `web_port` | `8090` | `TS_WEB_PORT` | HTTP dashboard |
| `data_dir` | `./Data` | `TS_DATA_DIR` | the static data directory (`ts_server_old/Data/`) |
| `database_url` | `mysql://user:pass@localhost:3306/ts_dream` | `TS_DATABASE_URL` | MySQL connection |
| `perexp_default` | `0` | `TS_PEREXP_DEFAULT` | initial PerEXP runtime value |

- Removed old keys (SQLite era): `account_db_path`, `member_dir`, `template_db_path` — do not accept them.

## 8.2 Protocol constants are NOT configurable

XOR `0xAD`, magic `F4 44`, ID prefix `vn`, min version `186`, server name `TSVN` — hardcoded. Changing them breaks parity with the real client. Do not expose as config.

## 8.3 Required local ops (not in the binary)

- MySQL server up; create the `ts_dream` database and a dedicated user; the binary connects via `database_url`. (Dump/backup/upgrade of the ops DB are operator concerns, out of spec scope.)
- Data files present under `data_dir` (byte-identical to `ts_server_old/Data/`).

# Chapter 9 — Acceptance

Normative: issue 07. The acceptance goal is **capture-based, byte-level** verification against the real C# server's network traffic.

## 9.1 Strategy

- **Golden packet sets** live in the repo under `golden/`, one text file per test case, versioned with the repo. The spec references them by filename; they are **not** embedded here.
- **Harness code ships in the repo** — a capture proxy tool and a test runner — so the executor can run them directly.

## 9.2 Capture methodology

- A TCP **proxy** records traffic in both directions: `client → proxy → C# server`. It doesn't modify the client or the server. Run against the **real** game client to get authentic Vietnamese/byte-VISCII traffic.
- Both directions are logged as **plaintext hex after XOR** (frames split on `F444` + length), using the packet.txt convention already present in `Data/packet.txt`.
- Scenarios must be **deterministic** (no timing-dependent broadcast frames, e.g. the 020C timer), or explicitly marked as such and excluded when choosing captures.

## 9.3 Golden file format

- Plaintext hex, one frame per line.
- `<<` = client→server (C2S), `>>` = server→client (S2C), `//` = comment, blank lines group related frames.
- No binary.

## 9.4 Golden runner (cargo integration test)

- Reads a golden file.
- Connects to the running Rust server.
- Sends the `<<` C2S frames in order.
- Collects the `>>` S2C responses.
- Diffs **frame-by-frame** against the golden S2C — exact byte equality; any difference fails the test.

## 9.5 C2S / S2C model

Two **sequential, independent** streams; no real-time interleaving of timed pairs. A deterministic-only policy — timing-dependent frames are not golden-locked.

## 9.6 Scenario scope (~10–15)

Login success / wrong password; create character; move; chat; buy from mall; use an item; warp; a quest (via the FTalk.H6 branch); 1–2 battle samples; pet. Each scenario is one golden file under `golden/`.

## 9.7 Protocol constants for the harness

XOR `0xAD`, magic `F4 44`, prefix `vn`, version `186`, name `TSVN` — hardcoded in the harness, matching Chapter 8.

## 9.8 Tooling language

The spec (this document) is the build contract in English; the harness/tooling code is English (Rust). The `golden/` data + proxy + runner all live in the repo tracked with it.

---

# Appendix A (reference) — FTalk.H6 table & exceptions

This appendix is the executable summary of the Protocol chapter's §2.6. The full compiled table (≈45 map cases, ~228 idtalking branches, 176 literal packets) is captured in the repo under `spec/` (the H6 addendum) and is the diff-gate source for the H6 scenarios. The three subsections that must be transcribed there:

1. **H6 pre-dispatch rules** — banker/hotel/shared NPC groups (see §2.6.1).
2. **H6 data table** — the per-NPC menu branches (add/remove item, add pet, hotel/sleep, packets).
3. **H6 exceptions** — daily-quest generator (map 12711, 21 fixed `random.Next` draws, item-id formulas `62001+num3*100` / `62101+num4*100`, reward magnitudes `value1..48`), and pet-reborn `55002/59102/59011`.

The executor ports the table + exceptions verbatim; the golden harness validates the result.

---

## End

This porting spec is the complete build contract for the TS Dream Rust server. Architecture, protocol, data, encoding, database, battle, config, dashboard, and acceptance are each specified to the level where an executor can implement without reading C# source. Where a decision affects bytes on the wire, it is pinned here and verified by the capture harness in Chapter 9.