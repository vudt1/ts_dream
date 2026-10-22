# `WIRE_FORMATS.md` — Khung frame C→S / S→C liên quan KỸ NĂNG

> Ngày: 2026-10-21 · Bundle: `client_skills_handler/`
> Mức độ: ✅ VERIFIED / ⚠️ INFERRED / ❓ OPEN.

---

## 0. Khung chung (đã biết)

```
[F4 44][Len : Word LE][Payload ...]        ← toàn khung XOR 0xAD
```

* `Payload[0]` = **Main OP**.
* S→C: `RestPayload = Copy(Payload, 2, Len-1)` ⇒ `RP[i] = Payload[i+1]`;
  `_LStrCopy(RP, k, n)` (Delphi 1-based) = `Payload[k .. k+n-1]`.
* C→S: builder dựng **ShortString** → `@PStrNCat(cap=2)` → `@LStrFromString` → `@LStrCat`
  → `TForm1.CY_AddSedQueue` (`0x0051633C`).

✅ **VERIFIED (Round-9)** — `TForm1.CY_AddSedQueue` ghép frame như sau:

```asm
005163ca: PUSH 0x5163e0                    ; hằng "F4 44" (PUSH#1)
005163d0: LEA ECX,[EBP-0x10]
005163d3: CALL 0x0077eb1c                  ; Len -> 2 byte LE
005163d6: PUSH dword ptr [EBP-0x10]        ; PUSH#2 = Len
005163d9: PUSH dword ptr [EBP-0x8]         ; PUSH#3 = payload
005163dc: MOV EDX,0x3 ; CALL 0x00404148    ; @LStrCatN -> "F4 44" + Len + payload
```

Hằng tại `0x5163E0` = `F4 44 00 00`, length-prefix `02 00 00 00` tại `0x5163DC`
⇒ hằng là AnsiString **2 ký tự** `"F4 44"`.

> ### 🔴 Khung C→S (byte-chính-xác, đã xác nhận)
> ```
> [ F4 44 ][ Len : u16 LE ][ opcode : u8 ][ sub : u8 ][ payload ... ]
> ```
> (`Len` = số byte của `opcode+sub+payload`.)

⚠️ **Lưu ý khi đọc `.c` của `ts_decompile/functions/`**: nhiều file `.c` ở đó đến từ **build
cũ hơn** và có offset `gself` lệch với `.asm.txt` (ví dụ `00710ab0_FUN_00710ab0.c` hiện
`+0x101/+0x100` trong khi `.asm.txt` hiện `+0x404/+0x400`). **Luôn tin `.asm.txt`.**

---

## 1. Bảng tra Main OP → handler (S→C), 65 OP

Nguồn: `jumptable_byte200_0x78A8EE.hex` (byte map, 200 byte) +
`jumptable_dword200_0x78A9B6.hex` (200 dword).

| Main OP | case | handler | note |
|:-:|:-:|---|---|
| `0x00` | 1 | `0x0078AABE` | |
| `0x01` | 2 | `0x0078B149` | |
| `0x02` | 3 | `0x0078B6FC` | chat |
| `0x03` | 4 | `0x0078BC95` | actor xuất hiện |
| `0x04` | 5 | `0x0078C7DB` | actor rời đi |
| **`0x05`** | **6** | **`0x0078CA56`** | **sub 03 = danh sách kỹ năng** (`FUN_0072E9A0`) |
| `0x06` | 7 | `0x0078CB99` | di chuyển actor |
| `0x07` | 8 | `0x0078CE37` | |
| **`0x08`** | **9** | **`0x0078D125`** | **sub 01 prop 0x6E = cập nhật 1 kỹ năng** |
| `0x09` | 10 | `0x0078D4AF` | |
| `0x0A` | — | `0x00796347` | **HOLE** |
| `0x0B` | 11 | `0x0078D5D1` | trận đấu |
| `0x0C` | 12 | `0x0078D84D` | |
| `0x0D` | 13 | `0x0078DBAE` | party |
| `0x0E` | 14 | `0x0078E01C` | |
| `0x0F` | 15 | `0x0078E367` | pet |
| `0x10` | 16 | `0x0078E593` | |
| `0x11` | 12 | `0x00796347` | **HOLE** |
| `0x12` | 12 | `0x00796347` | **HOLE** |
| `0x13` | 17 | `0x0078EADC` | |
| `0x14` | 18 | `0x0078EC3F` | |
| `0x15` | — | `0x00796347` | **HOLE** |
| `0x16` | 19 | `0x0078FEAF` | |
| `0x17` | 20 | `0x007902FB` | túi đồ |
| `0x18` | 21 | `0x00790ED5` | |
| `0x19` | 22 | `0x0079157C` | **sub 0B = xoá mảng kỹ năng** |
| `0x1A` | 23 | `0x0079175F` | |
| `0x1B` | 24 | `0x00791D80` | |
| **`0x1C`** | — | `0x00796347` | 🔴 **HOLE — S→C 0x1C KHÔNG được xử lý** |
| `0x1D` | 25 | `0x00791F76` | |
| `0x1E` | 26 | `0x007921A6` | |
| `0x1F` | 27 | `0x007922E6` | |
| **`0x20`** | **28** | **`0x0079285A`** | ⚠ **KHÔNG phải 0x1C** — xem §5 |
| `0x21` | 29 | `0x007928E4` | |
| `0x22` | 30 | `0x00792ACC` | |
| `0x23` | 31 | `0x00792BAD` | |
| `0x24` | 32 | `0x0079346B` | |
| `0x25` | 33 | `0x007937C6` | |
| `0x26` | 34 | `0x00793889` | |
| `0x27` | 35 | `0x007938C3` | |
| `0x28` | 36 | `0x007943BF` | |
| `0x29` | 37 | `0x007943F9` | |
| `0x2A` | 38 | `0x00794719` | |
| `0x2B`–`0x31` | — | `0x00796347` | **HOLE** |
| `0x32`–`0x43` | 43–60 | `0x007951DA`…`0x00795F48` | |
| `0x44` | — | `0x00796347` | **HOLE** |
| `0x45`–`0x48` | 61–64 | `0x00795F82`…`0x00796248` | |
| `0x49`–`0xC6` | — | `0x00796347` | **HOLE** |
| `0xC7` | 65 | `0x007962FC` | kênh GM |

> 🔴 **Đính chính tài liệu cũ.** `reference/analysis_docs/light_dark_skill_unlock_gap.md` §2.4
> nói `1C 01` / `1C 02` là S→C — **sai**. Frame đó là **opcode `0x20` sub 01/02**.
> Server Bear gửi `1C …` S→C sẽ bị client bỏ rơi.

---

## 2. S→C `05 03` — danh sách kỹ năng + chỉ số ⭐

✅ **VERIFIED** — handler `FUN_0072E9A0` (file `0072e9a0_FUN_0072e9a0.asm.txt`),
được `FUN_0078CA56` gọi ở `0x0078CAF7` với `RP` (payload trừ byte Main OP).

```
payload[0x00]        = 0x05                 (Main OP)
payload[0x01]        = 0x03                 (Sub OP)
payload[0x02]        = u8   → gself+0x3E9   (element / lớp nghề — chọn tab cây kỹ năng)
payload[0x03..0x04]  = u16  → gself+0x3EA
payload[0x05..0x06]  = u16  → gself+0x3EC   (điểm dùng để học — so với TFSkill+0x1C)
payload[0x07..0x08]  = u16  → gself+0x3EE
payload[0x09..0x0A]  = u16  → gself+0x3F0
payload[0x0B..0x0C]  = u16  → gself+0x3F2
payload[0x0D..0x0E]  = u16  → gself+0x3F4
payload[0x0F..0x10]  = u16  → gself+0x3F6
payload[0x11..0x12]  = u16  → gself+0x3F8
payload[0x13]        = u8   → gself+0x3FA
payload[0x14..0x17]  = u32  → gself+0x3FC   (EXP)
payload[0x18..0x19]  = u16  → gself+0x400   (skill point)
payload[0x1A..0x1B]  = u16  → gself+0x402   (điểm chỉ số chưa dùng)
payload[0x1C..0x1F]  = u32  → gself+0x440   (tier; +0x43E = FUN_0065B30C)
payload[0x20..0x21]  = u16  → gself+0x404   (MaxHP)
payload[0x22..0x23]  = u16  → gself+0x406   (MaxSP)
payload[0x24..0x27]  = u32  → gself+0x40C   (ATK bonus)
payload[0x28..0x2B]  = u32  → gself+0x410   (DEF bonus)
payload[0x2C..0x2F]  = u32  → gself+0x414   (INT bonus)
payload[0x30..0x33]  = u32  → gself+0x418   (AGI bonus)
payload[0x34..0x37]  = u32  → gself+0x41C   (MaxHP rank)
payload[0x38..0x3B]  = u32  → gself+0x420   (MaxSP rank)
payload[0x3C..0x45]  = 10   → gself+0x430..0x438  (5 × u16)
payload[0x46..0x70]  = 43   → gself+0x1334, +0x1305/6, +0x5F0, +0x1308, +0x408, +0x40A …
```

**Tail — danh sách kỹ năng (KHÔNG có count field):**

```
payload[0x71 + 3k .. 0x72 + 3k]  skillId : u16 LE  → gself[0xFD4 + idx*3]
payload[0x73 + 3k]               level   : u8      → gself[0xFD6 + idx*3]
        idx = FUN_00759B04(skillId)   ; = u16 [0x9AE128 + skillId*2], guard ≤ 0xFF
        N = (payloadLen - 0x71) / 3
```

ASM (`0x0072F2E3`):

```
0072f2e3: MOV dword ptr [EBP + -0x10], 0x71     ; pos = 0x71
         CALL @LStrLen(RP) ; SUB EAX,0x70 ; IDIV ECX (ECX=3)
0072f308: CMP EAX,0xFF → @BoundErr             ; guard N ≤ 255
```

⭐ **Hệ quả quan trọng** — ở **CUỐI** hàm này, client gọi `FUN_005A0388` (file
`005a0388_FUN_005a0388.asm.txt`) để **dựng lại bảng cây kỹ năng runtime** tại `0x94B440`:

```asm
MOV  EAX,0x94b440 ; XOR ECX,ECX ; MOV EDX,0x2fd ; CALL 0x00402cd0  ; FillChar(765, 0)
MOV  DX,0x2711 / 0x2af9 / 0x2ee1 / 0x32c9 ; CALL 0x00597954        ; 4 gốc
; vòng lặp i = 0..0xFF trên player[0xFD4+i*3] / [0xFD6+i*3]
```

⇒ **Tail của `05 03` KHÔNG chỉ là "danh sách kỹ năng đã học"** — nó còn quyết định
**bố cục cây kỹ năng** mà client vẽ. Chi tiết: [`SKILL_DATA_MODEL.md` §9](./SKILL_DATA_MODEL.md)
và [`BLOCKED_DUMPS.md`](./BLOCKED_DUMPS.md).

🔴 **Bug đã biết ở server Rust:** `src/server/spawn.rs` ghi **90** byte 0 thay vì **43**
⇒ header lệch 47 byte ⇒ client đọc hỏng toàn bộ danh sách kỹ năng.

---

## 3. S→C `08 01` — cập nhật 1 kỹ năng (prop `0x6E`) ⭐

✅ **VERIFIED** — handler `FUN_00710AB0` (file `00710ab0_FUN_00710ab0.asm.txt`).

```
payload[0x00]        = 0x08          (Main OP)
payload[0x01]        = 0x01          (Sub OP = "cập nhật chỉ số của chính mình")
payload[0x02]        = propCode = 0x6E
payload[0x03]        = deltaType     (2 ⇒ lấy đối của value1)
payload[0x04..0x07]  = value1 : u32 LE   (= LEVEL, tuyệt đối)
payload[0x08..0x0B]  = value2 : u32 LE   (= SKILL ID)
```

ASM (`0x00710AA7`):

```
00710aa5: MOV word ptr [EDX + EAX*0x1 + 0xfd4], 0x0     ; nếu value1 < 1 ⇒ XOÁ skill
00710aa7: MOV word ptr [EDX + EAX*0x1 + 0xfd4], CX      ; skillId
00710b16: MOV byte ptr [EDX + EAX*0x1 + 0xfd6], CL      ; level
007117a8: CMP word ptr [EDX + EAX*0x1 + 0xfd4], 0x36b3   ; id đặc biệt
```

⚠️ **`deltaType` phải = 0** cho prop `0x6E` (level tuyệt đối). Gửi `deltaType = 2` sẽ
xoá level và unlearn skill.
⚠️ Rust hiện gửi mode khác ⇒ 🔴 có thể **xoá nhầm skill đã học**.

**Các prop code quan trọng khác của cùng handler** (đọc từ `.asm.txt`):
`0x18`→`+0x3E9`, `0x19`→`+0x3EA`, `0x1A`→`+0x3EC`, `0x1B`→`+0x3EE`, `0x1C`→`+0x3F0`,
`0x1D`→`+0x3F2`, `0x1E`→`+0x3F4`, `0x1F`→`+0x3F6`, `0x20`→`+0x3F8`, `0x23`→`+0x3FA`,
`0x24`→`+0x3FC`, `0x25`→`+0x400` (skill point), `0x26`→`+0x402`, `0x32`→`+0x42E+idx*2`,
`0x3E`→`+0x43E`, `0x3F`→`+0x440`, `0x42`→`+0x43C`.

---

## 4. S→C `08 05` — danh sách kỹ năng tàu (4 slot)

✅ **VERIFIED** — handler `FUN_0074C340` (file `0074c340_FUN_0074c340.asm.txt`).

```
payload[0x00] = 0x08
payload[0x01] = 0x05
payload[0x02..0x03] = skillId u16 LE   ; slot 1
payload[0x04]       = level   u8
payload[0x05..0x06] = skillId u16 LE   ; slot 2
payload[0x07]       = level   u8
payload[0x08..0x09] = skillId u16 LE   ; slot 3
payload[0x0A]       = level   u8
payload[0x0B..0x0C] = skillId u16 LE   ; slot 4
payload[0x0D]       = level   u8
```

Ghi vào `gself+0x509 + slot*12` (skillId) và `gself+0x50B + slot*12` (level).

---

## 5. S→C `0x20 01` / `0x20 02` — ⚠ (KHÔNG phải `0x1C`)

✅ **VERIFIED** — handler `FUN_0079285A` (file `case_028_0079285A_FUN_0079285a.c`).

```
RP[0] == 1 :  gself+0x376 == 0  → FUN_0072C64C(gvar_007D9D34, RP)
              gself+0x376 != 4  → FUN_006546E0(gvar_007DA51C, RP)
RP[0] == 2 :  FUN_0072F534(gvar_007D9D34, RP)
```

Cả 3 hàm đọc giống nhau (`0072c64c.c:41-66`, `006546e0.c:34-60`):

```
_LStrCopy(RP, 2, 4, &tmp)     ; tmp = RP[1..4]  (4 byte)
idx   = FUN_0077EF7C(gvar_007D9D30, tmp)
level = RP[5]
FUN_0072A7A8(array[idx*4], level)
```

⇒ wire: `[20][01][ skillId : u32 LE ][ level : u8 ]` — record 5 byte.

* `FUN_0072C64C` → index vào `gvar_007DA300[]`, **bound 800** (danh sách NPC dạy skill).
* `FUN_006546E0` → `FUN_00654CE0(obj, id)` → index vào `obj+0x158[]`, **bound 21**
  (danh sách kỹ năng của nhân vật).

❓ **Mở** — ngữ nghĩa chính xác (NPC dạy skill? hay báo "bạn vừa học"?). Bear **không**
gửi opcode này nên không có tham chiếu.

---

## 6. C→S `0x1C` — học / nâng cấp kỹ năng ⭐  ★ ĐÃ CHỐT `0x1C 01/02/04/05` (Round-9)

> ### 🔴 Calling convention của bộ gửi (Round-9, VERIFIED)
> `FUN_0077F414` = **`SendPacket(encoder, opcode, sub)`** — `EAX`=encoder, `DL`=main op, `CL`=sub:
>
> ```asm
> 0077f417: PUSH ECX                  ; ECX (sub) -> [EBP-4]
> 0077f425: XCHG dword ptr [EBP-4],ECX
> 0077f42b: MOV byte ptr [EBP-6],CL   ; ⭐ sub  = CL
> 0077f42e: MOV byte ptr [EBP-5],DL   ; ⭐ main = DL
> 0077f431: MOV dword ptr [EBP-4],EAX ; encoder = EAX
> ```
>
> ⇒ Bảng nhảy `0x785DBB` là **bảng sub-code của opcode `0x1C`** (7 entry, sub 0..6).
>
> | sub | target | nguồn payload | hàm gọi |
> |:-:|:-:|---|---|
> | `1` | `0x785DD7` | `(*gvar_007D9EB8)+0x1A8` | `FUN_005990E0` @`0x59924B` |
> | `2` | `0x785E45` | `(*gvar_007D9EB8)+0x1A8` | `FUN_005990E0` @`0x599314`, `FUN_0058484C` @`0x5849D1` — ⭐ **học skill cho THÚ** |
> | `4` | `0x785EB3` | `(*gvar_007D9E68)+0x28C` | ⭐ `FUN_0057C840` |
> | `5` | `0x785F21` | `(*gvar_007D9E68)+0x290` | ⭐ `FUN_0057C840` @`0x57CD24`,`0x57CD90` |
> | `6` | `0x785F8F` | `(*gvar_007DA114)+0xE8` | `FUN_005B3DBC` @`0x5B3DCC` |
>
> ### 🔴 Quy tắc `@LStrCatN` (Round-9, VERIFIED) — nền tảng đọc layout
> `@LStrCatN` (`0x00404148`) đọc varargs **từ slot cao nhất xuống** và **copy slot cao nhất
> vào đầu buffer trước** ⇒ **kết quả = arg push đầu tiên + arg thứ hai + …**.
> ```asm
> 00404166: MOV EAX,[ESP + EBX*0x4 + 0x14]   ; arg push đầu tiên (slot cao nhất)
> 00404175: CALL 0x0040292c                 ; Move(dest, arg, len)
> 0040417a: DEC EBX / JNZ 00404166
> ```

✅ **VERIFIED** — bảng nhảy `0x00785DBB` (7 entry), dispatcher `0x00785DA6`:

```asm
00785da6: XOR  EAX,EAX
00785da8: MOV  AL, byte ptr [EBP + -0x6]      ; sub
00785dab: CMP  EAX, 0x6
00785dae: JA   0x0078a4f2                      ; sub > 6 → KHÔNG gửi
00785db4: JMP  dword ptr [EAX*0x4 + 0x785dbb]  ; sub 0..6
```

| sub | target | hành vi |
|:-:|:-:|---|
| `0` | `0x0078A4F2` | epilogue — **không gửi** |
| `1` | `0x00785DD7` | ✅ gửi — payload = `(*gvar_007D9EB8) + 0x1A8` |
| `2` | `0x00785E45` | ✅ gửi — payload = `(*gvar_007D9EB8) + 0x1A8` (**cùng nguồn sub 1**) |
| `3` | `0x0078A4F2` | epilogue — **không gửi** |
| `4` | `0x00785EB3` | ✅ gửi — payload = `(*gvar_007D9E68) + 0x28C` |
| `5` | `0x00785F21` | ✅ gửi — payload = `(*gvar_007D9E68) + 0x290` (tái sinh lần 2) |
| `6` | `0x00785F8F` | ✅ gửi — payload = `(*gvar_007DA114) + 0xE8` (kỹ năng tàu) |

**Khuôn builder (mẫu sub 5, `0x00785F21`):**

```asm
00785f21: LEA  EAX,[EBP + -0x34]
00785f24: MOV  DL, byte ptr [EBP + -0x5]     ; main = 0x1C
00785f27: MOV  byte ptr [EAX + 0x1], DL
00785f2a: MOV  byte ptr [EAX], 0x1           ; len ShortString (KHÔNG lên wire)
00785f4a: MOV  CL, 0x2
00785f4c: CALL 0x00402b60                    ; @PStrNCat cap=2 → "1C" (1 BYTE)
00785f5a: CALL 0x0040402c                    ; @LStrFromString
00785f65: MOV  EDX,[0x007d9e68]
00785f6d: MOV  EDX,[EDX + 0x290]             ; field nguồn
00785f73: CALL 0x00404090                    ; @LStrCat → nối chuỗi
00785f85: CALL 0x0051633c                    ; TForm1_CY_AddSedQueue → gửi
```

⇒ Wire: `[1C][sub]` ++ **AnsiString** (độ dài **thay đổi**, không phải record cố định).

### 6.1 ⭐⭐ LAYOUT `0x1C 01` — ĐÃ CHỐT (Round-7)

✅ **VERIFIED** — builder `FUN_005990E0` (`005990e0_FUN_005990e0.asm.txt`),
nút "học / nâng cấp" của cây kỹ năng:

```asm
005991d8: MOV EAX,dword ptr [EDX + EAX*0x4 + 0x118]   ; slot[i]
005991df: MOV EDX,dword ptr [EAX + 0xe4]              ; <<< skillId
005991f9: CALL 0x0077eb1c                             ; -> 2 byte skillId LITTLE-ENDIAN
00599212: MOV EDX,dword ptr [EAX + 0x108]             ; <<< level HIEN TAI
00599222: ADD EDX,0x1                                 ; <<< LEVEL MUC TIEU = level + 1
0059922c: CALL 0x00403fa0                             ; @LStrFromChar -> 1 byte
0059923d: CALL 0x004040d4                             ; @LStrCat3(dest, skillId, level)
00599249: MOV CL,0x1                                  ; <<< SUB = 01
0059924b: MOV DL,0x1c                                 ; <<< MAIN = 1C
0059924d: CALL 0x0077f414                             ; GUI
```

> ### 🔴 PAYLOAD `0x1C 01` = `{ skillId : u16 LITTLE-ENDIAN }{ targetLevel : u8 }` — **3 byte, ID TRƯỚC VALUE**
>
> ⇒ **KHỚP CHÍNH XÁC** server Bear (`ModifySkillHandler.cs`: `read16(data,i)` + `data[i+2]`)
> ⇒ **KHỚP CHÍNH XÁC** server Rust (`skills.rs:114-120`)
> ⇒ Claim cũ (G-05 trong `light_dark_skill_unlock_gap.md`): "client gửi `[level u8][skillId u16]`"
> là **SAI** cho gói `0x1C 01`. **G-05 ĐÓNG.**

### 6.2 `0x1C 02` — ⭐ **HỌC KỸ NĂNG CHO THÚ** — ✅ **VERIFIED (Round-10, ĐÃ SỬA)**

> ### 🔴🔴 SỬA LẦM ROUND-7 — LAYOUT `0x1C 02`
> **Claim cũ (SAI):** `{ count u8 }{ skillId u16 LE }{ targetLevel u8 }` — "học nhiều kỹ năng".
>
> **Claim ĐÚNG (Round-10, đọc từ client + khớp 100% Bear):**
> ```
> [1C][02]
> [ petSlot : u8 ]                    ← chỉ số THÚ (1..4) — KHÔNG phải count!
> [ skill1 : u16 LE ][ lvl1 : u8 ]
> [ skill2 : u16 LE ][ lvl2 : u8 ]     (tối đa 3 skill / 1 lần gửi)
> ```
>
> ⇒ **`0x1C 02` = học / nâng cấp kỹ năng cho THÚ CƯNG.**

✅ **VERIFIED** — `FUN_0058484C` (mới, 448 B) — form học skill cho thú:

```asm
005848b3: CMP dword ptr [EBP-8],5              ; vong 1..4 (tim slot thu)
005848c0: ADD EAX,0x1a8                        ; &TSe_SkillForm.shared
005848c5: MOV EDX,dword ptr [EBP-0xc]          ; ⭐ petSlot (1..4)
005848c8: CALL 0x00403fa0                      ; @LStrFromChar(shared, petSlot)
005848e6: MOV EAX,dword ptr [EDX + EAX*4 + 0xf4] ; node = self+0xF4 + i*4
005848ed: CMP byte ptr [EAX + 0x109],0x0       ; node->0x109 != 0 ?
005848fa: PUSH dword ptr [EBP-0x10]
   ...   FUN_0077eb1c -> skillId 2 byte LE ; level = node->0x108 + node->0x109
00584997: CALL 0x00404148                      ; @LStrCatN -> skillId(2)+level(1)
005849be: CALL 0x00404090                      ; @LStrCat(shared, skills)
005849cf: MOV CL,0x2 / 005849d1: MOV DL,0x1c
005849d3: CALL 0x0077f414                      ; ⭐ GUI 0x1C 02
```

✅ **VERIFIED** — `FUN_0071D234(player, X)` = **tìm slot thú theo loài**, KHÔNG phải đếm:

```c
for (local_14 = 1; local_14 <= 4; local_14++) {
    if (player->0x57C + local_14*4 != 0) {           // ⭐ 4 con trỏ THÚ
        if (player->0x57C[local_14]->0x78 != 0) {     // ⭐ thú còn sống
            if (player->0x57C[local_14]->0x55C == X)   // ⭐ loài thú (byte)
                return (char)local_14;                // ⭐ trả về chỉ số 1..4
        }
    }
}
return 0;
```

✅ **ĐỐI CHIẾU BEAR — KHỚP 100%** (`ModifySkillHandler.cs`):

```csharp
case 2:
    for (i++; i < data.Length; i += 3)
        client.getChar().pet[data[2] - 1].setSkill(PacketReader.read16(data, i), data[i + 2]);
```

⇒ `data[2]` = petSlot 1-based; skills set cho **thú** ✓

> ✅ **Server Rust ĐÃ ĐÚNG** — `src/server/handlers/skills.rs` → `handle_pet_skill_upgrade`
> đọc `payload[0]` làm `stt` thú. Vì `payload = decoded[6..]` (`decoded[4]`=op,
> `decoded[5]`=sub) ⇒ `payload[0] = data[2]` = petSlot ✓ ⇒ **chỉ tài liệu là sai.**

#### Bối cảnh (Round-7, giữ lại như tham chiếu)

✅ **VERIFIED** — nhánh `self->0x194 != 0` của `FUN_005990E0`:

```asm
00599275: CALL 0x0071d234                 ; -> AL : count (số lượng nhóm)
00599283: CALL 0x00403fa0                 ; @LStrFromChar(count)
00599288: PUSH [EBP-0x18]                  ; PUSH#1 = count        (1 byte)
005992bf: CALL 0x0077eb1c                 ; skillId -> 2 byte LE
005992c4: PUSH [EBP-0x1c]                  ; PUSH#2 = skillId      (2 byte LE)
005992f6: PUSH [EBP-0x20]                  ; PUSH#3 = targetLevel  (1 byte)
00599301: MOV EDX,0x3 ; CALL 0x00404148   ; @LStrCatN(dest, 3, ...)
00599312: MOV CL,0x2                       ; <<< SUB = 02
```

⇒ Theo quy tắc `@LStrCatN` (slot cao nhất copy trước) — **thứ tự push = thứ tự nối**:

> ### 🔴 PAYLOAD `0x1C 02` — xem §6.2 đầu trang (đã sửa Round-10)
> `{ petSlot u8 }{ skillId u16 LE }{ targetLevel u8 }` — **không phải count**.

### 6.3 ⭐ GATE: 28 skill id "gốc hệ" bị chặn

Trong `FUN_005990E0`, **trước khi gửi**:

```c
if ( skillId thuộc {0x2724..0x272A, 0x2B0C..0x2B12, 0x2EF4..0x2EFA, 0x32DB..0x32E1}
     && player[0x4B0] == 0 ) {
    show_message(&UNK_00599358, 2000);   // "khong the hoc ky nang nay"
    return;                              // KHONG GUI
}
```

| Nhóm | Dải id | Ý nghĩa |
|---|---|---|
| Kim/Địa | **10020..10026** | 7 skill gốc hệ |
| Thủy | **11020..11026** | 7 skill gốc hệ |
| Hỏa | **12020..12026** | 7 skill gốc hệ |
| Phong | **13021..13027** | 7 skill gốc hệ |

⇒ **28 skill id chỉ học được khi `player+0x4B0 != 0`** (đã gặp NPC thầy skill).

✅ **`player+0x4B0` = cờ "ĐÃ HỌC CHUYỂN SINH"** — xác nhận Round-9 (2 nguồn độc lập):
* `FUN_005990E0` chặn 28 id gốc hệ khi `player->0x4B0 == 0`.
* `FUN_0057DD0C` hiện cây nhánh nghề khi `player->0x4B1 != 0` ⇒ **`player+0x4B1` =
  cờ "đã mở nhánh nghề"**, hiển thị ở **tab 6..9**.

```c
// FUN_005990E0
iVar1 = *(int *)(*(int *)(local_8 + 0x118 + param_2*4) + 0xE4);   // skillId
if ((((iVar1-0x2724U<7)||(iVar1-0x2B0C<7)||(iVar1-0x2EF4<7)||(iVar1-0x32DB<7))
     && (*(char *)(*(int *)gvar_007DA7BC + 0x4B0) == '\0')) {
    show_message(&UNK_00599358, 2000, 0, 0);   // KHÔNG gửi
    return;
}
```

### 6.4 `0x1C 06` — kỹ năng tàu (✅ VERIFIED)

`FUN_005B34C0` (`005b34c0_FUN_005b34c0.asm.txt`):

```asm
005b3606: MOV EDX,dword ptr [EAX + 0xE4]     ; skillId
005b3620: CALL 0x0077eb1c                   ; -> 2 byte LE
005b3630: CALL 0x00403e4c                   ; payload(form+0xE8) += skillId
005b3638: CALL 0x005b3dbc                   ; GUI 0x1C 06
```

⇒ **Payload `0x1C 06` = `{ skillId : u16 LE }` — 2 byte** (1 skill / 1 lần gửi).

> ### ⭐ Danh tính form (đọc `vmtClassName` = VMT−0x2C, Round-10)
> | Global | VMT | Tên lớp |
> |---|---:|---|
> | `gvar_007D9E68` | `0x577984` | **`TSe_SkillTreeForm`** (form cây kỹ năng NPC) |
> | `gvar_007D9EB8` | `0x589754` | **`TSe_SkillForm`** (chủ buffer dùng chung `+0x1A8`) |
> | `gvar_007DA5A8` | `0x5D5A20` | `TInterForm` (nguồn `0x1B 01`) |
> | `gvar_007DA720` | `0x5D5438` | `TThingSaleForm` (nguồn `0x1B 02`) |
> | `gvar_007D9E30` | `0x577A30` | `TSe_NPCSkillTreeForm` |
> | `PTR_DAT_007da3d4` | `0x577B0C` | `TSe_ChildSkillTreeForm` |
> | `gvar_007DA05C` | `0x5897EC` | `TSe_MiniSkillForm` |

### 6.5 `0x1C 05` (nâng cấp / tái sinh lần 2) — ✅ **ĐÃ CHỐT (Round-9)**

✅ **VERIFIED (Round-9)** — `gvar_007D9E68` là form **`TSe_SkillTreeForm`**
(RTTI field table tại `0x577A00`: field `published` ở `+0x24C`, `+0x25C`, `+0x288`,
`+0x28C`, `+0x290`; tên lớp `"TSe_SkillTreeForm"` tại `0x577A1E`;
lớp con `"TSe_NPCSkillTreeForm"` tại `0x577AF6`).

**Hàm xây payload = `FUN_0057C840`** (1419 B, đã có trong corpus):

```asm
0057c840: 55 8B EC                     PUSH EBP; MOV EBP,ESP
0057c860: 8B 45 FC 05 8C 02 00 00      ADD EAX,0x28C
0057c866: E8 89 75 E8 FF                CALL 0x00402CF0   ; FillChar(self+0x28C,?,0)
0057c86d: 8B 45 FC 05 88 02 00 00      ADD EAX,0x288
0057c874: E8 7C 75 E8 FF                CALL 0x00402CF0   ; FillChar(self+0x288,?,0)
0057c87b: 8B 45 FC 05 90 02 00 00      ADD EAX,0x290
0057c882: E8 6F 75 E8 FF                CALL 0x00402CF0   ; FillChar(self+0x290,?,0)
0057c894: 66 81 B8 3A 06 00 00 CD 29   CMP word [EAX+0x63A],0x29CD  ; map id 10637
```

**Hàm lắp payload = `FUN_00583A98`** (`self`, `s288`, `skillList`, `s28C`):

```c
_LStrClr(self + 0x290);
len288   = LStrLen(s288);
numSkill = LStrLen(skillList) / 3;      // ⭐ 3 BYTE / KỸ NĂNG
len28C   = LStrLen(s28C);
FUN_0077ee84(enc, len288,   &A);         // A = 4 byte LE  (FUN_0077ee84 = ghi DWORD LE)
FUN_0077ee84(enc, numSkill, &B);         // B = 4 byte LE
FUN_0077ee84(enc, len28C,   &C);         // C = 4 byte LE
_LStrCatN(self + 0x290, 6, A, s288, B, skillList, C, s28C);
```

> ### 🔴 PAYLOAD `0x1C 05` — LAYOUT BYTE-CHÍNH-XÁC
> ```
> [1C][05]                                    ← main + sub
> [u32 LE] L1 = len(self+0x288)              ← độ dài khối "tab đang bật"
> [L1 byte]  nội dung self+0x288
> [u32 LE] L2 = len(skillList) / 3            ← SỐ KỸ NĂNG
> [L2*3]     mỗi kỹ năng: { skillId : u16 LE }{ targetLevel : u8 }
> [u32 LE] L3 = len(self+0x28C)              ← độ dài khối "nhánh nghề"
> [L3 byte]  nội dung self+0x28C
> ```

### 6.5b `0x1C 04` — học kỹ năng nhánh nghề — ✅ **ĐÃ CHỐT (Round-9)**

Builder `0x785EB3` (sub 4) đọc `(*gvar_007D9E68) + 0x28C`. Buffer `+0x28C` do
`FUN_0057C840` dựng trong vòng 3 (bảng `DAT_0094B3A4`, chỉ lấy entry **6..13**):

```asm
0057cc0a: CALL 0x00403fa0                 ; @LStrFromChar(index)   -> 1 byte
0057cc37: CALL 0x0077eb1c                 ; skillId (u16) -> 2 byte LE
0057cc4c: CALL 0x00404148                 ; @LStrCatN -> nối 3 byte
```

> ### 🔴 PAYLOAD `0x1C 04`
> ```
> [1C][04]
> [entry]*   mỗi entry: { slotIndex : u8 }{ skillId : u16 LE }     (3 byte/entry)
> ```

### 6.5c Luồng nghiệp vụ của `FUN_0057C840`

```
FillChar(self+0x28C / +0x288 / +0x290)
if (player->mapId (+0x63A) == 0x29CD (10637)) → báo "Tại địa điểm này chức năng này
                                               tạm không được sử dụng"; return
V1: duyệt self->0x24C (mảng 2 chiều node TSe_TreeNodeBtn)
    node->0x109 != 0 → skillList += { node->0x10C (u16 LE) }{ node->0x108 + node->0x109 }
V2: duyệt DAT_0094B330[1..28]   (28 "gốc hệ")
    rec->0x111 != 0 → skillList += { rec->0x114 (u16 LE) }{ rec->0x110 + rec->0x111 }
V3: duyệt DAT_0094B3A4[1..13]   (13 node nhánh nghề)
    rec->0x111 != 0 → skillList += { rec->0x114 }{ rec->0x110 + rec->0x111 }
                      nếu index > 5 → self+0x28C += { index u8 }{ rec->0x114 }
V4: duyệt 12 nút self->0x138..0x164
    nút->0x108 != 0 → self+0x288 += 1 ký tự chỉ số (1..12)

if (skillList == 0 && self+0x288 == 0) → báo "Bạn chưa thực hiện kỹ năng tăng cấp
                                          biến đổi nào" (param 2000)
else if (self+0x288 != 0)              → FUN_00583a98(); Send(0x1C, 05)
else if (skillList != 0 && bVar1)      → FUN_00583a98(); Send(0x1C, 05)
else if (skillList != 0)               → Send(0x1C, 01)
```

### 6.6 `0x1B 01` — ✅ **ĐÃ CHỐT (Round-9)**, ⭐ **đính chính Round-11**

> **Bảng field đầy đủ của `TInterForm` / `TThingSaleForm` (Round-11):**
>
> | Form | Offset | Kiểu | Ý nghĩa |
> |---|---:|---|---|
> | `TInterForm` (`gvar_007DA5A8`) | `+0x152` | u8 | index NPC trong danh sách (0..255) |
> | `TInterForm` | `+0x154` | u16 | mã NPC (1..1250) — payload `0x1B 01` |
> | `TInterForm` | `+0x156` | u8 | cờ "đã chọn NPC" |
> | `TThingSaleForm` (`gvar_007DA720`) | `+0x160` | u8 | payload `0x1B` (case thứ 2) |
> | `TThingSaleForm` | `+0x162` | u16 | payload `0x1B` (case thứ 2) |

```asm
00786011: LEA EAX,[EBP-0x34]; MOV DL,[EBP-5]   ; A = main op (0x1B)
00786028: LEA EAX,[EBP-0x3c]; MOV DL,[EBP-6]   ; B = sub (0x01)
0078603c: CALL 0x00402b60                     ; @PStrNCat(cap=2) -> "1B 01"
00786057: MOV DL,byte ptr [EDX + 0x152]       ; ⭐ 1 byte từ form NPC thầy
0078606b: CALL 0x00402b60                     ; @PStrNCat(cap=3) -> "1B 01 <byte>"
00786092: MOV DX,word ptr [EAX + 0x154]       ; ⭐ skillId (u16)
007860a0: CALL 0x0077eb1c                     ; -> 2 byte LE
007860ac: CALL 0x00404090                     ; @LStrCat(dest, skillId)
007860be: CALL 0x0051633c                     ; gửi
```

> ### 🔴 PAYLOAD `0x1B 01` = `{ 0x1B }{ 0x01 }{ npcIndex : u8 }{ npcId : u16 LE }` — **5 byte**
>
> ⚠ **KHÁC `0x1C 01`** — `0x1B 01` **không** có byte level.
> ⇒ Rust `src/server/handlers/skills.rs` cần tách riêng handler `0x1B`.
>
> ### ✅ ĐÍNH CHÍNH Round-11 — nguồn của 2 byte payload
>
> | Byte | Nguồn | Ý nghĩa | Bằng chứng |
> |:-:|---|---|---|
> | 3 | `TInterForm + 0x152` (u8) | **index NPC trong danh sách** (0..255) | `0x5D74B7..0x5D74C9` — set cùng lúc với `self->0x124` (list control) được `FUN_007C340C` chọn item thứ index |
> | 4-5 | `TInterForm + 0x154` (u16 LE) | **mã NPC** (1..1250) | `0x5D6F4C..0x5D6F54` — `FUN_005D7218` đọc field chuỗi → `StrToInt`, chỉ nhận 1..1250 |
>
> Cờ phụ: `TInterForm + 0x156` (u8) = "đã chọn NPC" (0/1) — `FUN_005D6F7C` chỉ gửi
> `0x1B 01` khi cờ này khác 0.
>
> ⚠ **Round-9** ghi byte u16 là `skillId` ⇒ **SAI**, đã sửa ở Round-11.

---

## 6.7 ⭐⭐ Bảng dispatch S→C đầy đủ của `FUN_0077F414` (Round-12)

Dispatcher **2 tầng**:

```asm
0077f467: MOV AL,byte ptr [EAX + 0x77f474]      ; ⭐ byte map 200 byte: main_op → index
0077f46d: JMP dword ptr [EAX*0x4 + 0x77f53c]    ; ⭐ dword table: index → handler
```

| Tầng | Địa chỉ | Nội dung |
|:-:|---|---|
| 1 | `0x77F474` (200 B) | `main_op → index`; index `0` = không xử lý |
| 2 | `0x77F53C` (dword) | `index → handler VA` |
| 3 | mỗi handler | switch trên `sub` (vd `0x1C` → bảng nhảy 7 entry `@0x785DBB`) |

⇒ **17 main-op** được xử lý; bảng đầy đủ 200 dòng:
[`data/cs_dispatch_map.tsv`](./data/cs_dispatch_map.tsv).

| main-op | handler | Nội dung |
|:-:|:-:|---|
| `0x05` | `0x77FBA2` | danh sách kỹ năng `05 03` |
| `0x08` | `0x77FF2E` | cập nhật kỹ năng `08 01` |
| `0x19` | `0x7853CA` | thanh kỹ năng |
| `0x1B` | `0x785FFD` | học skill từ NPC thầy (sub 1 = `TInterForm`, sub 2 = `TThingSaleForm`) |
| `0x1C` | `0x785DA6` | **học / nâng cấp kỹ năng** |
| `0x1D` | `0x786185` | (block kế bên `0x1B`) |
| `0x20` | `0x786987` | cây kỹ năng |
| `0x29` | `0x788288` | hotkey |
| `0x32` | `0x788BCA` | dùng kỹ năng theo id (code chết) |

> ⭐ **Đính chính Round-11**: block đọc `TThingSaleForm+0x160/+0x162` tại `0x7860C8`
> **vẫn thuộc opcode `0x1B`** (block `0x1B` = `0x785FFD..0x786184`), **không phải
> opcode riêng**.

---

## 7. C→S `0x1B` — 1 skill từ NPC thầy

✅ **VERIFIED** — builder `0x00786011` (dump Round-9: `asm_00786011_007861b7_builder_1b.asm.txt`).

```
[1B][01] ++ byte (gvar_007DA5A8 + 0x152) ++ Word LE (gvar_007DA5A8 + 0x154)
```

* `gvar_007DA5A8` = `TInterForm` (dialog thầy skill).
* ⭐ **KHÁC `0x1C 01`**: không có byte level; byte thứ 3 là **chỉ số NPC**.
* `0x1B 02` (`0x007860C8`) dùng form khác (`gvar_007DA720`): byte tại `+0x160`,
  skillId tại `+0x162`.
* Caller `FUN_005D6F7C` **loại trừ NPC `0x2EE1` (12001)** và **`0x2F1D` (12061)** ⇒ 2 NPC thầy skill.
* `FUN_005D6F7C` cũng ghi `*(param_1 + 0x154) = 1` ngay trước khi gửi.

⚠️ Rust đang hiểu `0x1B` là **mua/bán ở NPC shop** (`dispatcher.rs:307-312`). Hai nghiệp vụ
**cùng hình dạng wire** ⇒ cần capture thật để chốt.

---

## 8. C→S `0x32 01` — dùng kỹ năng theo id (⚠ CODE CHẾT)

✅ **VERIFIED** — builder `0x00788C38` (dead code):

```
[32][01] ++ srcGrp ++ srcSlot ++ dstGrp ++ dstSlot ++ Word LE (skill id) ++ 2 byte PRNG
```

⚠️ **Không có call-site nào** gọi builder này (đã quét 131 call-site `CALL 0x0077F414`)
⇒ **code chết** trong build này. ⇒ Client **không** gửi skill id khi dùng kỹ năng trong
trận; server phải tự suy ra skill từ frame lệnh `0x0B 02/03/04`.

---

## 9. Frame liên quan khác

| Frame | Handler | Ghi chú |
|---|---|---|
| S→C `05 0A` | `FUN_0072F970` | thông báo lên cấp (`"LevelUP"` + `sound\WA0013.wav`) |
| S→C `19 0B` | `FUN_0075B8E0` | **xoá toàn bộ 3000 ô kỹ năng** (`gself+0xFD4`) |
| S→C `08 02` | `FUN_00710AB0` | prop của actor khác (`gself+0x57C + index*4`) — cũng hỗ trợ prop `0x6E` |
| S→C `08 04` | `FUN_0073B458` | prop theo index entity |
| C→S `17 0F` | `0x00783B81` | dùng vật phẩm (sách kỹ năng) — server resolve `Item.dat.skill_link` |
| C→S `0B 02/03/04` | `0x00780595`… | lệnh trận đấu — **không** mang skill id |
| C→S `28`/`29` | `0x0078820F` | hotkey / thanh kỹ năng |

---

## 10. Quy tắc gate phía client (không qua mạng)

| Gate | Điều kiện | Hàm |
|---|---|---|
| "Đã học chưa?" (danh sách id cứng) | `gself+0xFD4+idx*3 == skillId` | `FUN_006538BC` |
| "Bắt buộc phải học" | `TFSkill+0x19 (type) == 17` | `FUN_006538BC` |
| "Hệ hợp lệ khi dùng" | `elem == 5` ⇒ OK; `elem != target.elem` ⇒ reject | `FUN_0076D578` |
| "Đủ điểm học" | `player+0x3EC >= TFSkill+0x1C` | `FUN_0059F090` |
| "Đủ level" | `player+0x400 (Level) >= yêu cầu` | `FUN_0059F090` (`0x406`–`0x432`) |
| "Ô bị khoá" | `requireByte == 0xF5` | `FUN_0059F090` |
| "Đã học" khi click NPC | id `0x36C7` / `0x36D2` | `FUN_0073D558` |
