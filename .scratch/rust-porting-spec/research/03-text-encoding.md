# 03 — Text encoding contract (Vietnamese / game text)

Research subagent findings. All byte-level claims were verified empirically on
`ts_server_old/` (Linux, xxd/Python 3/iconv). Goal: define the exact decode-on-load
and encode-on-send contract so the Rust port round-trips item/npc/player names
byte-exactly with the game client.

## 0. Verdict up front

- The on-wire text encoding of the game client is **VISCII 1.1** (single byte per
  Vietnamese character). The C# code confirms this three independent ways (see §2,
  §3, §4).
- `Npcs.txt` and `Items.txt` are **not** raw VISCII on disk. They are *mojibake*:
  original VISCII bytes were mis-decoded as **Windows-1252** (undefined bytes passed
  through as C1 controls) and re-saved as **UTF-16LE with BOM** (Npcs.txt) and
  **UTF-8 without BOM** (Items.txt). The mojibake is **faithful and reversible**:
  every mojibake char maps back to exactly one VISCII byte.
- **Verdict on Items.txt**: it is a *faithful copy of a legacy codepage* (VISCII)
  that has been through one broken conversion (VISCII → CP1252 → UTF-8). It is
  *not* corrupted-in-repo in a lossy sense — the original VISCII bytes are fully
  recoverable. The `"D¤u Ch¤m Höi"` rendering is exactly `D ấ u` with ấ=0xA4 → `¤`
  and ỏ=0xF6 → `ö`. One genuine anomaly exists: item 48101 contains a literal
  proper-Unicode `ă` (U+0103) mixed into otherwise-mojibake text.
- The C# server's `smethod_13` emits **VISCII bytes for all chars ≤ 0xFF** (which is
  the entire mojibake alphabet) by `AscW(ch).ToString("X2")`. So the existing C#
  server round-trips 99.9% of names byte-exactly. A small number of names (99 items,
  23 NPCs) contain CP1252 punctuation chars (>0xFF) that **do not** round-trip (see
  §5.4); the C# server garbles or aborts those packets, and the Rust port may choose
  to replicate or fix that.

## 1. Encoding census of the data files

Byte evidence for every file loaded by `Server_TS_Online/Data.cs`.

| File | Loader | Encoding on disk | Evidence |
|---|---|---|---|
| `Data/Npcs.txt` | `File.ReadAllLines` (Data.cs:4048) | **UTF-16LE, BOM `FF FE`, LF-only line endings** | `xxd`: `fffe 2f00 2f00 4400...` (BOM + `//Da...` in UTF-16LE); `file`: "Unicode text, UTF-16, little-endian" |
| `Data/Items.txt` | `File.ReadAllLines` (Data.cs:4203) | **UTF-8, no BOM, CRLF** — content is CP1252-mojibake of VISCII | `xxd`: starts `2f2f 4461 7461` (`//Data`); name field for id 10000 = `44 c2 a4 75 20 43 68 c2 a4 6d 20 48 c3 b6 69` = UTF-8 `D¤u Ch¤m Höi` |
| `Data/Skills.txt` | `File.ReadAllLines` (Data.cs:4385) | **UTF-8 (proper Unicode Vietnamese)**, CRLF | `xxd` + decode: `Đấu vật`, `Thuật mưa đá`, `Thái Sơn áp đỉnh`, … |
| `Data/Warps.txt` | `File.ReadAllLines` (Data.cs:4506) | ASCII, CRLF | `file`: "ASCII text" |
| `Data/BattleGate.txt` | `File.ReadAllLines` (Data.cs:4752) | ASCII, CRLF | `file`: "ASCII text" |
| `Data/Dolls.txt` | `File.ReadAllLines` (Data.cs:4792) | ASCII, CRLF | `file`: "ASCII text" |
| `Data/NPConMap.txt` | `File.ReadAllLines` (Data.cs:4891) | ASCII, CRLF | `xxd`: `//MapId\tId\tNpcId...` |
| `Data/ItemonMap.txt` | `File.ReadAllLines` (Data.cs:5349) | ASCII, CRLF | `xxd`: `//MapId\tId\tItemId...` |
| `Data/Member.ini` | `Class6.smethod_1` → `GetPrivateProfileStringW` (kernel32) | ASCII (passwords `1111...`) | `xxd`: `[Account]`… `file`: "ASCII text" |
| `Data/Quests/*.ini` (643) | `IniFile` → `GetPrivateProfileStringW` | UTF-8 / pure ASCII | Python `decode('utf-8')` OK |
| `Data/Quests/*.ini` (144) | same | **unidentified 8-bit** (bytes 0xA0–0xEF) for `Title=` values only | high-byte ranges `{0xA_:654, 0xB_:1062, 0xC_:1195, 0xD_:875, 0xE_:1037}`; tried VISCII/TCVN/VNI/CP1258/CP1252/GBK — none yield clean Vietnamese (see §6.2) |
| `Data/Quests/*.ini` (26) | same | heuristic UTF-16LE hits, no BOM — see §6.2 | not BOM-marked; treated as 8-bit |
| `Data_Client/ITEM.DAT`, `Npc.Dat` | `ItemData.LoadItems` / `NpcData.LoadNpcs` (dev-only `/loaditems`, `/loadnpcs` commands) | binary, XOR-obfuscated fields; names raw-but-reversed | see §6.1 — does NOT match the shipped `Data/*.txt` |

`server_note.md` only lists .NET v4.0 assemblies; no encoding hints.

### 1.1 How `File.ReadAllLines` decodes these

.NET Framework `File.ReadAllLines(path)` uses UTF-8 **with BOM auto-detection**
(UTF-16LE BOM → UTF-16 decode; no BOM → UTF-8). Consequences:
- `Npcs.txt` → proper Unicode chars `\u00DF \u00BD \u00D0 \u00A3 …` (the mojibake).
- `Items.txt` → the same mojibake alphabet as Unicode chars.
- `Skills.txt` → genuine Vietnamese Unicode.

So in-memory, all three files converge to **Unicode strings made of the same
mojibake codepoint alphabet** (mostly U+0080–U+00FF, see §5.3).

## 2. What `TextEncoder.cs` does

`Server_TS_Online.DataTools/TextEncoder.cs` defines a hard-coded **VISCII 1.1**
byte→Unicode table:

- `VISCII_char[102]` = VISCII byte values (includes C0 control bytes `0x02,0x05,0x06,
  0x14,0x19,0x1E` for Ẳ/Ẵ/Ẫ/Ỷ/Ỹ/Ỵ and duplicate ỗ at `0x92`/`0xB2` — all standard
  VISCII 1.1 quirks).
- `Unicode_char[102]` = the matching Vietnamese Unicode letters.
- `convertToUniCode(byte[] text, int init, int length)`: byte → Unicode char via the
  table; **fallback `(char)byte`** (Latin-1 pass-through) for bytes not in the table
  (e.g. `0xD0` = `Ð`).

**Critical:** `convertToUniCode` is **never called** anywhere in the codebase
(grep confirms). It is a dead helper from the original project and is only useful as
*documentation of the game's on-wire encoding*: it proves the server developers knew
the client speaks VISCII. It does **not** participate in the runtime data path.

## 3. How the C# builds name → packet bytes, and packet bytes → string

### 3.1 Outgoing (server → client): `Class5.smethod_13` (Class5.cs:340–353)

```csharp
foreach char: text += Strings.AscW(ch).ToString("X2");
```

- `AscW(ch)` = the UTF-16 code unit as `int`; `ToString("X2")` = uppercase hex,
  **minimum 2 digits, no upper padding**.
- Char ≤ 0xFF → **2 hex digits = exactly the VISCII byte**. This is the whole game
  text path: every mojibake char in `Npcs.txt`/`Items.txt` is ≤ 0xFF (bar the one
  `ă`), so names round-trip to VISCII bytes.
- Char 0x1000–0xFFFF → 4 hex digits (e.g. `„`=U+201E → `"201E"` → parsed by
  `smethod_4` as bytes `20 1E` — 2-byte garbage).
- Char 0x100–0xFFF → **3 hex digits** (e.g. `ă`=U+0103 → `"103"`). The hex→bytes
  parser `smethod_4` assumes 2 digits/byte; a 3-digit group throws, pops a `MsgBox`
  on the server and returns a truncated array. Effect: packet is aborted/garbled.

Called from every name-bearing packet: Server.cs:103,177,192,244,351,401 (player/
horse/pet names); Data.cs:2195,2270,2295,2416 (item/NPC/pet-rename names);
Client.cs:1841,1869,6057,6085,6264,6279,8164,8170,8172,9856,10020,10097 (names,
chat, announcements); FChat.cs:111; FormServer.cs:3438,3461.

Packet name length fields use `name.Length` **in chars** (e.g. Server.cs:103,
Data.cs:2416) — consistent only when each char is one byte, i.e. for the mojibake
alphabet. This confirms the client's 1-byte-per-char (VISCII) expectation.

### 3.2 Server-generated text: `Class5.smethod_17` (Class5.cs:420–462)

`smethod_17(s)` maps proper-Unicode Vietnamese → single-byte codepoints by positional
lookup in two parallel string tables. Extracted tables (verified by script):

- `uni` = `"áàảãạăắằẳẵặâấầẩẫậéèẻẽẹêếềểễệíìỉĩịóòỏõọôốồổỗộơớờởỡợúùủũụưứừửữựýỳỷỹỵđÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬÉÈẺẼẸÊẾỀỂỄỆÍÌỈĨỊÓÒỎÕỌÔỐỒỔỖƠỚỜỞỠÚÙỦŨỤƯỨỪỬỮỰÝỲỶỸỴĐ"`.
- `enc` = chars whose code = the target byte. The byte assignments **match the
  VISCII table** from TextEncoder.cs for the whole shared range (e.g. ấ→0xA4, ắ→0xA1,
  ờ→0xB6, ỏ→0xF6, ố→0xAF, ợ→0xFE) and additionally supply the uppercase letters
  (À→0xC0, Á→0xC1, Đ→0xD0, …) that TextEncoder lacks.
- Because `enc` has fewer entries than `uni`, some letters collapse: Ỏ/Õ/Ọ/Ỷ/Ỹ/Ỵ →
  `'?'` (0x3F), Ẳ/Ẵ → `'A'`. Lossy for those, irrelevant in practice.

Call sites (all are server-authored strings, never data-file names):
Client.cs:8163 (`"TSVN"`), Client.cs:8171 (welcome banner), FChat.cs:110
(`/where` reply), FormServer.cs:3435,3458 (announcements).

The output of `smethod_17` is then fed to `smethod_13`, so the wire bytes are VISCII.

### 3.3 Incoming (client → server)

Every client-packet text field is parsed as **one char per byte**:

```csharp
text += Conversions.ToString(Strings.Chr(packet[i]));   // char code == byte value
```

Examples: Client.cs:983 (login H1), 1189/1196 (new character name + secret codes),
1221 (char-name check), 7412/7418/7424/7430 (change-password), 7457+ (gift codes),
7579/7587. No `TextEncoder` is applied — bytes are treated as 0–255 codepoints.
Player names, pet names, and passwords therefore live in C# as chars 0–255
(= VISCII bytes) and are stored to Access/`Member.ini` in that form.

Chat: `FChat.H2` first `Encoding.ASCII.GetString(...)`s the payload for command
parsing (FChat.cs:17), then relays the **raw byte array** unchanged
(FChat.cs:484 `… + Class5.smethod_3(chat)`). Chat text is never re-encoded.

Wire obfuscation (orthogonal to text): both directions XOR every byte with **0xAD**
(`Class5.smethod_5`); outgoing via `Sendpacket` = `smethod_5(smethod_4(hex))`
(Client.cs:8268), incoming = `smethod_5(smethod_4(recv))` (Server.cs:524,550,577,
609,641).

## 4. The mojibake story (provenance of Items.txt / Npcs.txt)

Recovery pipeline that reproduces every byte of both files (Python-verified):

1. Original source: VISCII bytes, e.g. `44 A4 75 20 43 68 A4 6D 20 48 F6 69`
   (`Dấu Chấm Hỏi` — ấ=0xA4, ỏ=0xF6).
2. A tool decoded them as **Windows-1252** (0x80–0x9F → `€ ‚ ƒ „ … † ‡ ˆ ‰ Š ‹ Œ Ž
   ' ' " " • – — ˜ ™ š › œ ž Ÿ`; undefined bytes 0x81/0x8D/0x8F/0x90/0x9D passed
   through as C1 controls; 0xA0–0xFF → Latin-1-identical).
3. Result re-saved as Unicode: **UTF-16LE+BOM** for `Npcs.txt`, **UTF-8** for
   `Items.txt`.

Proof items:
- `"D¤u Ch¤m Höi"` = VISCII `44 A4 75 … 48 F6 69` under CP1252 → `¤`(U+00A4), `ö`(U+00F6).
- Recovered names are coherent Vietnamese: `Trương Giác`, `Trương Bảo`, `Trương
  Lương`, `Trình Viễn Chí`, `Ðặng Mậu` (Npcs); `Cuốc`, `Ðoản đao`, `Ðao Ngắn vàng`
  (Items). Uppercase Đ is byte 0xD0 (renders `Ð` U+00D0, consistent with the
  smethod_17 table's 0xD0→Đ).
- `Npcs.txt` "Trß½ng" = VISCII `54 72 DF BD 6E 67` (ư=0xDF, ơ=0xBD) under CP1252 →
  `ß`(0xDF), `½`(0xBD).
- CP1252-only codepoints present (`„`×88, `†`×6, `'`×1, `€`×1, `™`×1, `œ`×1,
  `Š`×1) prove the mis-decode was **Windows-1252, not Latin-1** (Latin-1 would have
  yielded C1 controls instead). GNU `iconv -f CP1252` rejects undefined bytes
  (verified), so the original tool was not iconv; .NET/Java-style "best-fit with
  pass-through" is consistent.

### 4.1 Reverse map (mojibake char → VISCII byte) — required by the port

- U+0000–U+007F → same byte (ASCII).
- U+0080–U+009F → byte = codepoint (C1 pass-through; only U+008F actually occurs).
- U+00A0–U+00FF → byte = codepoint (Latin-1 = CP1252 in this range).
- CP1252 punctuation (defined in 0x80–0x9F) → reverse CP1252:
  `€→80 ‚→82 ƒ→83 „→84 …→85 †→86 ‡→87 ˆ→88 ‰→89 Š→8A ‹→8B Œ→8C Ž→8E
  '→91 '→92 "→93 "→94 •→95 –→96 —→97 ˜→98 ™→99 š→9A ›→9B œ→9C ž→9E Ÿ→9F`.
- Not reversible: the one `ă` (U+0103) at Items.txt id 48101.

Full-file check: Items.txt has **no unmappable char except that `ă`**; Npcs.txt has
**none**.

## 5. Recommended Rust encoding contract

### 5.1 Storage model

Keep every name as a **`Vec<u8>` of VISCII bytes** (the client's alphabet) plus
(optionally) a `String` of proper Unicode derived via the VISCII table. The C#
server's "mojibake codepoint" strings and the Rust "VISCII byte string" are the same
thing; the port should store the bytes directly.

### 5.2 Load-time decode (per file)

- `Data/Npcs.txt` — read bytes; strip UTF-16LE BOM `FF FE`; decode UTF-16LE;
  then apply the §4.1 reverse map char-by-char → VISCII bytes; VISCII→Unicode only
  for display. Line splitting: LF only.
- `Data/Items.txt` — decode UTF-8 (no BOM); apply §4.1 reverse map → VISCII bytes.
  Decide explicitly how to treat the single `ă` (U+0103) at item 48101: (a) replicate
  C# = feed `0x103` through smethod_13 semantics (packet aborts — bad), or
  (b) normalize it to VISCII `ă`=0xE5 (recommended: client displays correctly).
- `Data/Skills.txt` — UTF-8, proper Unicode; skill names are server-GUI-only
  (SendRedMessage, Data.cs:5943,5950; Client.cs:5144,5150), never sent in packets.
  Any representation is fine; do not re-encode into packets.
- `Data/Warps|BattleGate|Dolls|NPConMap|ItemonMap.txt`, `Data/Member.ini` — ASCII.
- `Data/Quests/*.ini` — parse as bytes. Keys/ints/hex are ASCII. `Dialogs=` values
  are pre-built packet hex (opaque; forward verbatim). `Title=` is an unidentified
  8-bit blob used only for server-side GUI strings (Data.cs:5770) — preserve raw or
  ignore; **never sent to clients**.

### 5.3 Send-time encode (server → client)

`String → packet bytes`: for each code point, write the **VISCII byte** if the char
is ≤ 0xFF, else replicate C# `smethod_13` semantics (`AscW.ToString("X2")`, variable
width). In practice every value that reaches a name-bearing packet must be a VISCII
byte string; enforce at the boundary so the 0x80–0x9F VISCII control-char bytes
(Ẳ=0x02, Ẵ=0x05, Ẫ=0x06, Ỷ=0x14, Ỹ=0x19, Ỵ=0x1E) survive hex round-tripping
(`02`/`05`/`06`/`14`/`19`/`1E` are valid hex pairs, so they do).

For server-authored text (announcements, welcome banner, `/where`): replicate
`smethod_17`'s Unicode→VISCII positional table (§3.2) then hex-encode; or simply
emit VISCII bytes directly for the ASCII+VISCII subset actually used.

Name length fields in packets = **byte count** (== C# `name.Length` only when every
char ≤ 0xFF, i.e. always for VISCII-byte strings).

Hex-encode, then the wire layer XORs each byte with 0xAD (`smethod_5`).

### 5.4 The 122 names that do NOT round-trip in C# (bug-for-bug decision)

- Items.txt: 99 names contain chars >0xFF (98 CP1252-punctuation occurrences + the
  `ă`), e.g. `Thái „t binh pháp`, `Áo „u Bi`, `Áo †n Phong`, `†nPhânThân Huy Hi®u`.
- Npcs.txt: 23 names, e.g. `Quái †n Sî`, `T× „p`, `„t Ð¸a Gia Lan`, `Lão †n Sî`.

The C# server emits `201E`/`2020`/… (2 bytes, `20 1E` …) or aborts on 3-digit
groups (`œ`=U+0153). The Rust port should **fix** these by mapping the CP1252
punctuation back to the intended VISCII byte (§4.1) before sending — that is what the
game client actually expects. 99.9% of names are unaffected.

### 5.5 Required tables

1. VISCII byte→Unicode (TextEncoder.cs, verified; add 0xD0→Đ, 0xDD→Đ from
   smethod_17 for full coverage) — for display/DB.
2. Reverse mojibake map (§4.1) — for Items.txt/Npcs.txt load.
3. Unicode→VISCII positional table (smethod_17, §3.2) — for server-authored text.

## 6. Open questions / caveats

### 6.1 `Data_Client/ITEM.DAT` + `Npc.Dat` do not match the loader

`ItemData.LoadItems`/`NpcData.LoadNpcs` (dev-only `/loaditems`, `/loadnpcs`) expect a
raw struct stream (ItemInfo=373 B, NpcInfo=?, header 370/92) with names reversed in
place and fields XOR-decoded (`(v^0x9A)-9` etc.). The shipped `ITEM.DAT` (2,948,900 B)
does **not** parse with those structs: first record yields `namelength=3`, name
`"???"`; a full scan shows garbage `namelength`s (up to 255) and no occurrence of any
known item name in VISCII, reversed, XORed, or mojibake form. It is a different
client-data version and the shipped `Items.txt`/`Npcs.txt` were **not** generated
from it. The `.DAT` files are irrelevant to the runtime path; ignore unless the
regeneration tool is being ported (then the struct formats in ItemInfo.cs/NpcInfo.cs
are the spec, but the sample file won't validate them).

### 6.2 Quest ini `Title=` 8-bit encoding unidentified

144 of 813 quest inis carry `Title=` values in an 8-bit encoding using bytes
0xA0–0xEF. VISCII, TCVN-5712 (iconv), VNI, CP1258, CP1252, GBK, Big5, and a dozen
others all fail to decode to clean Vietnamese. The values are only concatenated into
server-GUI strings (Data.cs:5770) and are never sent to clients, so the port can
treat them as opaque bytes. The 26 "UTF-16LE" hits from a lenient heuristic have no
BOM and are almost certainly misclassified binary; treat them as 8-bit too. `Dialogs=`
hex values are pre-built packets and must be forwarded verbatim.

### 6.3 Client-side confirmation gap

The actual TS Dream client is not in this repo, so the VISCII-on-wire conclusion is
inferred from: TextEncoder.cs's VISCII table, smethod_17's VISCII byte table,
smethod_13's `AscW` behavior on the mojibake files, `Strings.Chr` byte-per-char
incoming parsing, and the source-level mojibake string `"Th¶i gian:"` (Client.cs:8169)
where `¶`=0xB6=ờ. If a packet trace (e.g. `Data/packet.txt`, a raw log of `F444…`
hex strings) is available for a Vietnamese name, decode its tail with the VISCII
table to confirm — recommended sanity check before the port goes live.
