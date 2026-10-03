# Luồng Client — Kết Hôn (0x2D) & Sinh Con / Nuôi Con (0x45) — Phân Tích & Gap Server

Ngày: 2026-10-02 · Cập nhật xác minh: 2026-10-02
Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Tài liệu nền: [`post_login_stats_inventory_flow.md`](./post_login_stats_inventory_flow.md), [`opcode_2d.md`](./opcode_2d.md), [`opcode_45.md`](./opcode_45.md), [`light_dark_skill_unlock_gap.md`](./light_dark_skill_unlock_gap.md)

**Nguồn chính:** `client_pseudo_c/` (C + ASM + HEX), `test/ts_decompile/`, `Data/*.dat|emg`, `src/`.
**Phương pháp:** mọi claim ghi `✅` đã được tự đối chiếu lại bằng script/ASM trong phiên này (không
dựa vào tài liệu cũ). Decoder VISCII dựng lại từ chính `src/encoding.rs::viscii_to_unicode`
(+ fallback Latin-1 cho `0xA0..0xFF` đúng như hàm Rust).

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — §2.4 ĐÃ VERIFY BYTE-EXACT, framing ĐÃ CHỐT. Xem §0.1**

---

## 0.1 🔴 XÁC NHẬN SAU ROUND-4 REDUMP

### 0.1.1 ✅ `0x45` C→S — **byte-exact, khớp tuyệt đối §2.4**

Builder đọc tay từ `functions/0077f414_FUN_0077f414.asm.txt` (entry `0x00789DDB`):

```
00789ddb: MOV AL,[EBP + -0x6]        ; sub
00789dde: DEC AL / JZ 0x00789def     ; sub 1
00789de2: DEC AL / JZ 0x00789ebe     ; sub 2
00789dea: JMP 0x0078a4f2             ; con lai -> KHONG GUI

; --- sub 1 @ 0x00789DEF ---
00789df5: MOV DL,[EBP + -0x5]        ; 0x45  -> ShortString{0x45, 0x01}
00789dfe: MOV byte ptr [EAX], 0x1    ; do dai ShortString
00789e18: MOV CL, 0x2                ; _PStrNCat cap=2  -> 2 byte "45 01"
00789e2d: CALL 0x0040402c            ; @LStrFromString
00789e40: MOV DX, word ptr [EAX+0x144]  ; *(gvar_007D9E3C)+0x144
00789e4e: CALL 0x0077eb1c            ; u16 -> 2 byte LE          (PUSH)
00789e67: MOV DL, byte ptr [EDX+0x158]  ; +0x158
00789e6d: CALL 0x00403fa0            ; 1 raw byte                (PUSH)
00789e86: MOV DL, byte ptr [EDX+0x146]  ; +0x146
00789e8c: CALL 0x00403fa0            ; 1 raw byte                (PUSH)
00789ea2: CALL 0x00404148            ; @LStrCatN cap=4
00789eb4: CALL 0x0051633c            ; TForm1_CY_AddSedQueue

; --- sub 2 @ 0x00789EBE ---
... MOV DL,[EDX + 0x159] -> LStrFromChar   (PUSH)
... MOV DL,[EDX + 0x146] -> LStrFromChar   (PUSH)
00789f4d: MOV CL, 0x4 / CALL 0x404148  ; @LStrCatN cap=4
```

| Builder | Wire | §2.4 dự đoán | Kết quả |
|---|---|---|:-:|
| `0x00789DEF` | `[45][01]` + `u16 @+0x144` + `u8 @+0x158` + `u8 @+0x146` = **5 B** | giống hệt | ✅ **khớp tuyệt đối** |
| `0x00789EBE` | `[45][02]` + `u8 @+0x159` + `u8 @+0x146` = **4 B** | giống hệt | ✅ **khớp tuyệt đối** |

Ngoài ra xác nhận: `0x45` chỉ có **2 sub**; mọi sub khác rơi vào `JMP 0x0078A4F2` = **không gửi**.

### 0.1.2 ✅ Câu hỏi framing `01 … 01 …` — **ĐÃ CHỐT: không có byte framing**

§0 #4 nêu nghi vấn "byte `MOV [EAX],0x1` là độ dài ShortString chứ không phải byte wire".
**Đúng như giả thuyết.** Bằng chứng mạnh nhất là chính builder `0x45` ở §0.1.1:
`_PStrNCat(cap=2)` trên `ShortString{0x45, 0x01}` sinh ra **đúng 2 byte** `45 01`.

Ba nguồn độc lập cùng kết luận:

| Nguồn | Bằng chứng |
|---|---|
| `0x45` builder `0x789DEF` | `ShortString{45,01}` + `cap=2` ⇒ 2 byte `45 01` |
| `0x0B` builder `0x780595` | `ShortString{0B,inner}` + `cap=3` ⇒ `[0B][inner]` |
| `case_020_007902FB:51` | truyền thẳng `RP` (bắt đầu tại byte sub) cho handler |

**`FUN_0040402C = @LStrFromString` nay ĐÃ CÓ BODY**
(`ts_decompile/functions/0040402c__LStrFromString.c`) ⇒ §8 Q10 / framing không còn mở.

⇒ **Header C→S = `[main][sub]`, không có byte `0x01` framing.** Đóng `guild §6 R5`.

### 0.1.3 ✅ Byte-map `0x77F474` xác nhận

`cs_bytemap_77f474_200B.hex` đọc trực tiếp: `byte[0x2D] = 0x2A` (42) ✓ và
`byte[0x45] = 0x39` (57) ✓ — khớp §1.

### 0.1.4 §6.2 (Q10) — **đóng một phần**

`FUNC 0x00794977` (case 41) và `FUNC 0x00795F82` (case 61) **nay đã có `.c` + `.asm.txt`**
(`ts_decompile/functions/00794977_*.{c,asm.txt}`, `00795f82_*`).
→ **Không cần capture `aLogin.exe` để đọc ASM nữa**; chỉ cần capture để xác nhận
*ngữ nghĩa* (`Q1`, `Q6`) chứ không phải *layout byte*.

### 0.1.5 ⚠️ §6.2 (G24) — literal con cái đã dump

| Nhóm | Địa chỉ | Trạng thái |
|---|---|---|
| 3 nhãn thuộc tính con **còn thiếu** | `0x7501CC`, `0x7501E4`, `0x7501F8` | ✅ đã dump hex |
| hậu tố nối chuỗi | `0x75020C`, `0x750228` | ✅ |
| 5 literal chat | `0x74FC88`, `0x74F4D0`, `0x74FDE4`, `0x74FE10`, `0x74FE3C` | ✅ |
| `lit_74fdb8` (bị cắt) | `0x74FDB8..0x74FE0F` | ✅ đã dump rộng |
| `Xoá kỹ năng thất bại` | `0x5E82F0` | ✅ |
| `2D 0E` / `2D 0F` chat | `0x743AA4..0x743B04`, `0x74403C..0x744048` | ✅ |
| `2D 0A` popup | `0x7988B8` | ✅ |

**⇒ Dùng `tools_decode_literals.py` trên các file mới để đóng §4.3 + Q5 (token
`Đời 2 võ tướng`) — không cần redump thêm.**

### 0.1.6 §8 — cập nhật trạng thái câu hỏi

| # | Câu hỏi | Trước | Sau Round-4 |
|:-:|---|:-:|---|
| Q2 | Ý nghĩa `2D 01`/`2D 02` C→S | ❓ | ✅ **ĐÃ ĐÓNG** — ASM đầy đủ trong `functions/0077f414_FUN_0077f414.asm.txt:3101-3229` |
| Q8 | `2D 0E` gọi `FUN_0071DC74` | ❓ | ✅ **ĐÃ ĐÓNG** — `FUN_0071DC74` có body (`0071dc74_FUN_0071dc74.c`) |
| Q9 | `2D 10` ghi vào `*(gvar_007DA600+0xE8)` | ❓ | ⚠️ vẫn cần đọc `FUN_005B1EE0` |
| Q1 | `player+0x1450` là gì | ❓ | ❓ **vẫn mở** (cần capture) |
| Q6 | `+0x151D == 2` là sống hay chết | ❓ | ❓ **vẫn mở** (mâu thuẫn nội tại client) |
| Q3 | Ngưỡng đẳng cấp kết hôn | ❓ | ❓ vẫn mở — thiết kế |
| Q4 | Có NPC hôn nhân không | ❓ | ❓ vẫn mở |

---

## 0. Kết luận điều hành

| # | Kết luận | Mức |
|:-:|---|:-:|
| 1 | **`0x2D` là bus KẾT HÔN / LY HÔN / TẶNG LỄ KIM**, không phải "Reborn". `src/protocol/mod.rs:180` đang đặt tên sai (`OP_REBORN`). | ✅ |
| 2 | **`0x45` là bus HỆ THỐNG CON CÁI.** Tên `OP_CHILD` trong `mod.rs:254` **đã đúng**. | ✅ |
| 3 | **SỬA LỖI LỚN:** `opcode_2d.md:130` và `opcode_45.md:250` nói "client không bao giờ gửi `0x2D` / `0x45`" dựa trên `case 0x2d: break;` — **SAI.** Cả hai **có builder C→S thật**: `0x2D` → `0x0078896A` (4 sub-action), `0x45` → `0x00789DDB` (2 sub-action). | ✅ |
| 4 | **SỬA LỖI framing:** byte `MOV [EAX],0x1` trong ASM builder **không phải byte wire** mà là **độ dài ShortString** Delphi. Header C→S là `[main][sub]` — **không** có byte `01` framing. | ✅ **Xác nhận Round-4, xem §0.1.2** |
| 5 | **SỬA LỖI tên:** "6 slot trang bị" của `0x45` trong `mod.rs:254` sai — đó là **6 thuộc tính** con (INT/ATK/DEF/AGI/HP/SP), có kèm byte *grade*. Không có item-id lookup nào. | ✅ |
| 6 | **SỬA LỖI tên:** `opcode_2d.md:189-202` gán sai 3 literal của sub `0x01` và 2 literal của sub `0x06`. Bảng đúng ở §3.3. | ✅ |
| 7 | Server Rust **chưa có dòng code nào** cho `0x2D`/`0x45`: cả hai rơi vào `unimplemented::handle` (`dispatcher.rs:375,388`). Không có bảng DB quan hệ, không có field `Session`, không có frame trong Logined1. | ✅ |
| 8 | Game data **có** item kết hôn thật: `46046 Lễ hợp thành hôn`, `46047 Lễ pháo thành hôn`, `19760/19761 Lễ Phục Tân Lang/TânNương`, chuỗi quà tình `46001..46005`. | ✅ |
| 9 | `TS_Server_Bear/` (server C# tham chiếu) **không implement** kết hôn/con cái — không có mốc thiết kế nào để sao chép. | ✅ |
| 10 | `src/data/loaders/item.rs:164` giải mã tên item bằng **CP1252 thay vì VISCII** → hỏng ~30% ký tự tiếng Việt. Đã chứng minh bằng `Item.dat` thật. | ✅ |

---

## 1. Xác minh dispatch (S→C)

Router S→C `FUN_0078A89C` — `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt:30-31`:

```asm
MOV AL, byte ptr [EAX + 0x78a8ee]      ; opcode → case index
JMP dword ptr [EAX*0x4 + 0x78a9b6]     ; case index → handler
```

| main | `byte[0x78A8EE+op]` | `dword[0x78A9B6+idx]` | handler | file |
|:--:|:--:|:--:|---|---|
| `0x2D` | `0x29` = 41 | `0x00794977` | `FUN_00794977` | `case_041_00794977_FUN_00794977.c` (332 dòng) |
| `0x45` | `0x3D` = 61 | `0x00795F82` | `FUN_00795F82` | `case_061_00795F82_FUN_00795f82.c` (92 dòng) |

Nguồn: `client_pseudo_c/jumptable_byte200_0x78A8EE.hex` dòng 3 & 5,
`jumptable_0x78A9B6_case_functions.csv:43` và `:63`. ✅

Hai handler **liền kề vùng code `0x0074EC14..0x00750404`** cho thấy chúng là các form
UI riêng (không dùng chung code với `0x2D`).

---

## 2. ⭐ PHÁT HIỂN MỚI — Chiều C→S **CÓ TỒN TẠI**

### 2.1 Bảng builder C→S (đã tự giải mã `table_0x77F474_200B.hex` × `table_0x77F53C_dword200.hex`)

`FUN_0077F414` KHÔNG phải `switch` — nó là **dispatch 2 mức** (`asm:27-35`):

```asm
0077f467: MOV AL, byte ptr [EAX + 0x77f474]     ; opcode → index
0077f46d: JMP dword ptr [EAX*0x4 + 0x77f53c]    ; index  → builder
```

Đối chiếu chéo (byte-map → dword-table → xuất hiện như label thật trong ASM) — tất cả khớp:

| main | byteMap | idx | builder | ASM dòng | sub-action |
|:--:|:--:|--:|---|:--:|---|
| `0x08` | `08` | 8 | `0x0077FF2E` | 445 | 1 (đã biết) |
| `0x09` | `09` | 9 | `0x0078002D` | 465 | 1 (đã biết) |
| `0x2C` | `29` | 41 | `0x007887C2` | — | — |
| **`0x2D`** | **`2A`** | **42** | **`0x0078896A`** | **3101-3229** | **4** |
| `0x2E` | `2B` | 43 | `0x00788B6A` | 3230 | 1 |
| `0x42` | `37` | 55 | `0x00789AE0` | 4226 | 6 |
| `0x43` | `38` | 56 | `0x00789CEF` | 4366 | 2 |
| `0x44` | `00` | 0 | `0x0078A4F2` | 4755 | **epilogue → chết thật** |
| **`0x45`** | **`39`** | **57** | **`0x00789DDB`** | **4428-4529** | **2** |
| `0x46` | `3A` | 58 | `0x00789F7A` | 4530 | 1 |
| `0x47` | `3B` | 59 | `0x00789FDA` | 4556 | 1 |
| `0x48` | `3C` | 60 | `0x0078A142` | 4641 | 6 |
| `0xC7` | `3D` | 61 | `0x0078A318` | 4646 | 2 |

> `0x0078A4F2` là **epilogue** của `FUN_0077F414` (`asm:4755`: `XOR EAX,EAX; POP; PUSH 0x78a822`).
> ⇒ opcode map về `0x78A4F2` **không có builder**.

**Tham số** (`asm:18-20`): `EAX`=context, `DL`=**main opcode**, `CL`=**sub-action**.

### 2.2 ⛔ Sửa lỗi framing: `01 … 01 …` KHÔNG phải byte wire

Pattern lặp lại ở **mọi** builder:

```asm
0078898e: LEA EAX,[EBP + -0x34]
00788991: MOV DL, byte ptr [EBP + -0x5]     ; 0x2D
00788994: MOV byte ptr [EAX + 0x1], DL      ; [EBP-0x33] = 0x2D
00788997: MOV byte ptr [EAX], 0x1           ; [EBP-0x34] = 0x01  <-- ĐỘ DÀI ShortString
007889a0: CALL 0x00402b90                    ; @LStrFromPChar : ShortString{len=1,"\x2D"} -> AnsiString
...
007889b7: MOV CL, 0x2
007889b9: CALL 0x00402b60                    ; @PStrNCat(cap=2) -> ShortString{0x2D, CL}
007889c7: CALL 0x0040402c                    ; @LStrFromString -> AnsiString 2 byte
```

`FUN_00402b90` (`00402b90_FUN_00402b90.c`) = copy `*param_2 + 1` byte từ ShortString →
AnsiString. `FUN_00402b60` (`00402b60__PStrNCat.c`) = nối ShortString với giới hạn
`param_3`. ⇒ byte `0x01` là **length prefix 1-based của Delphi ShortString**, **không
đi lên wire**.

**Proof bằng control case `0x08`** (`asm:445-501`, đã được `post_login_stats_inventory_flow.md`
§6.5 xác minh là `[08][01][u16 @+0xFCC][u8 @+0xFCE][u32 @+0xFCF][u8 @+0xFD3]`): builder dùng
**đúng** pattern `MOV [EAX],0x1` ấy, và payload thật là `08 01` + 8 byte. Nếu `0x01` là byte
wire thì `0x08` sẽ phải là `01 08 01 01 …` — trái với tài liệu đã xác minh.

**Proof thứ hai bằng helper:** `FUN_0077EB1C(ctx, u16, &out)` **không** phải "u16 → chuỗi
thập phân" mà là **u16 → 2 byte raw LE**:
```c
local_11 = (u8)(v & 0xff);  local_12 = (u8)(v >> 8);
FUN_00402b90(local_1c,&local_18);   /* len=1 */
_PStrNCat(local_1c,&local_20,2);    /* cap 2 -> 2 raw byte */
_LStrFromString(param_3, local_1c);
```
Tương tự `FUN_0077EE84(ctx, u32, &out)` = **u32 → 4 byte raw LE**, và `_LStrFromChar(&d, DL)`
(`00403fa0__LStrFromChar.c`) = **1 byte raw**.

⇒ **Toàn bộ C→S payload là byte nhị phân thô**, không phải text.

### 2.3 C→S `0x2D` — 4 sub-action (ASM `0077f414_FUN_0077f414.asm.txt:3101-3229`)

Dispatch con (`asm:3101-3110`): `CL==1`→`0x0078898E`, `CL==2`→`0x007889FC`,
`CL==3`→`0x00788A6A`, `CL==4`→`0x00788B15`, khác → `JMP 0x0078A4F2` (không gửi).

| sub | body | Payload sau `[2D][sub]` | Nguồn byte |
|:--:|---|---|---|
| **`01`** | `0x0078898E` | `<raw bytes của AnsiString @(*gvar_007D9C1C)+0x1D0>` — biến thiên | `asm:3111-3138`: `MOV EDX,[EDX+0x1D0]; CALL 0x404090 (@LStrCat)` rồi `CALL 0x51633C` (send) |
| **`02`** | `0x007889FC` | `<raw bytes của AnsiString @(*gvar_007DA260)+0x150>` — biến thiên | `asm:3139-3166` |
| **`03`** | `0x00788A6A` | **`u32 LE @player+0x144C`** + **`u8 @player+0x1450`** = **5 byte** | `asm:3182` `MOV EDX,[EAX+0x144C]` → `CALL 0x77EE84`; `asm:3196` `MOV DL,[EDX+0x1450]` → `CALL 0x403FA0`; `MOV EDX,0x3; CALL 0x404148 (@LStrCatN)` |
| **`04`** | `0x00788B15` | **rỗng** (chỉ `2D 04`) | `asm:3207-3229`: `CALL 0x40402C` rồi `CALL 0x51633C` ngay, **không** `@LStrCat` |

```
Wire: [2D][01][<string bytes>]                    ← sub 1  (⚠ ngữ nghĩa)
      [2D][02][<string bytes>]                    ← sub 2  (⚠ ngữ nghĩa)
      [2D][03][u32 targetId][u8 actionCode]       ← sub 3  ★ HÀNH ĐỘNG CHÍNH
      [2D][04]                                    ← sub 4  (ack rỗng)
```

> **sub 3 = "cầu hôn / tặng lễ kim cho người đã chọn"**: đọc `player+0x144C` — cùng offset
> mà S→C `0x2D 07` ghi vào (`FUN_0074384C`, `0074384c_FUN_0074384c.c:63-69`) — rồi gửi
> target id + 1 byte mã hành động.

### 2.4 C→S `0x45` — 2 sub-action (ASM `asm:4428-4529`)

Dispatch con (`asm:4428-4433`): `CL==1`→`0x00789DEF`, `CL==2`→`0x00789EBE`, khác → không gửi.

| sub | body | Payload sau `[45][sub]` | Nguồn byte |
|:--:|---|---|---|
| **`01`** | `0x00789DEF` | **`u16 LE @form+0x144`** + **`u8 @form+0x158`** + **`u8 @form+0x146`** = **5 byte** | `asm:4452` `MOV DX,word [EAX+0x144]` → `CALL 0x77EB1C`; `asm:4467` `MOV DL,[EDX+0x158]`; `asm:4486` `MOV DL,[EDX+0x146]`; `MOV EDX,0x4; CALL 0x404148` |
| **`02`** | `0x00789EBE` | **`u8 @form+0x159`** + **`u8 @form+0x146`** = **4 byte** | `asm:4504` `MOV DL,[EDX+0x159]`; `asm:4536` `MOV DL,[EDX+0x146]`; `MOV CL,0x4; CALL 0x402B60` |

Đối tượng `gvar_007D9E3C` là form xóa kỹ năng con — **cùng** object mà S→C `0x45 0A` truyền
vào `func_0x005E7F90` (`case_061_…:56`) và `0x5E82BC` = `"Xoá kỹ năng thành công của …"`.
⇒ **C→S `0x45` là thao tác kỹ năng con** (chọn slot / xác nhận xóa), **không phải cho ăn**.

```
Wire: [45][01][u16 skillSlot][u8 grp][u8 sel]   ← sub 1
      [45][02][u8 grp][u8 sel]                  ← sub 2
```

### 2.5 Vòng lặp khép kín 0x2D

```
S→C  [2D][07][u32 id][u16 idx]      ──► ghi player+0x144C = id ; render portrait 0..100
                                          (FUN_0074384C, 0074384c_FUN_0074384c.c:63-69)
                                              │
C→S  [2D][03][u32 id][u8 code]      ──◄────────┘  đọc lại player+0x144C + player+0x1450
                                          (asm:3189 + asm:3196)
```

⚠️ `player+0x1450` **không có writer** trong `client_pseudo_c/` (grep `0x1450` chỉ ra 1 hit
ở `asm:3196`, là lệnh đọc). Nó do một form/UI chưa được decompile ghi vào.

---

## 3. S→C `0x2D` — Bus Kết Hôn (chi tiết từng sub)

`case_041_00794977_FUN_00794977.c`. Quy ước: `P[0]=0x2D`, `P[1]=sub`, `RP[i]=P[i+1]`.

### 3.1 Bảng sub-op

| sub | Wire (offset trên `P`) | Len | Hành động client | Literal hiện |
|:--:|---|:--:|---|---|
| `01` | `[01][kind:1]`; **kind 0** thêm `+u32 @P2..5` `+f64 @P6..13` | 3 / **15** | kind 0: `+0x1438=u32`, `+0x1440=f64`, banner. kind≠0: banner | xem §3.3 |
| `02` | `[02][dw:4 @P2..5][flag:1 @P6]` — `dw` đọc ra **bỏ** | **7** | flag 0: clear `+0x1438/+0x1440/+0x1444/+0x1448` + banner; 1 / `FF`: banner | §3.3 |
| `03` | `[03][u32 @P2..5][f64 @P6..13][u8 @P14]` | **15** | ghi full state, **không banner** | — |
| `04` | `[04]` | 2 | `(**(*gvar_007D9C1C)+0x20)()` refresh window | — |
| `05` | `[05]` | 2 | `(**(*gvar_007DA1BC)+0x20)()` refresh list manager | — |
| `06` | `[06][kind:1 @P2]` | 3 | banner theo kind | §3.3 |
| `07` | `[07][u32 @P2..5][u16 @P6..7]` | **8** | `+0x144C=id`; lookup actor (`FUN_0070C20C`); idx>100 → RangeError | — |
| `08` | `[08][v:1 @P2]` | 3 | banner **chỉ khi `v==0x02`** | "Đối phương không đồng ý lấy bạn" |
| `09` | `[09][u32 @P2..5][u32 @P6..9]` | **10** | banner ghép 5 mảnh | "Số tiền thu người đến" + tên + "Lễ kim" + số + "Vàng" |
| `0A` | `[0A][u32 @P2..5][f64 @P6..13][text @P14..end]` | **≥14** | `FUN_005749F0` → popup `TSe_FixedForm` | ❓ `0x007988B8` không có hex |
| `0B` | `[0B][u32 id1][u32 id2]` | **10** | `FUN_00743B6C` → chat log ghép 2 tên | "Chúc mừng" / "với" / "kết thành phu thê" ✅ |
| `0C` | `[0C][u32 n @P2..5]` + `n`×rec 8B | **6+8n** | clear list rồi append `n` cặp | "Với" (`0x5739A8`) ✅ |
| `0D` | `[0D][v:1 @P2]` | 3 | `+0x1448 = v`, không banner | — |
| `0E` | `[0E][u32]*` (`(len-1)/4`) | 2+4k | `FUN_0071DC74(actor)` cho từng id | ❓ callee không có body |
| `0F` | `[0F][u32 id1][u32 id2]` | **10** | `FUN_00743E70` → chat log 2 tên (fallback tên mình) | ❓ `0x74403C/48` không có hex |
| `10` | `[10][u32][n:1][name:n][b][b][b][b][u32][u32]` | **n+19** | `FUN_005B1EE0` ghi record vào `*(mgr600+0xE8)` | — |

`0x00` và `≥0x11` → **no-op** (không có `default`).

### 3.2 Record 8 byte của sub `0x0C` (đã tự verify từ `case_041:262-296`)

`FUN_00573F50(mgr)` clear trước. Với `i = 0..n-1`:
- `a = EF7C(@LStrCopy(RP, i*8+10, 4))` → `P[10+8i .. 13+8i]`
- `b = EF7C(@LStrCopy(RP, i*8+6 , 4))` → `P[6+8i  .. 9+8i ]`
- `FUN_00573820(gvar_007DA1BC, a, b)`

⇒ record `i` nằm ở `P[6+8i .. 13+8i]`, **8 byte**. `n ≤ 0` ⇒ chỉ clear.
`FUN_00573820` bỏ qua nếu `a` hoặc `b` bằng id player; ghép
`name(b) + "Với" + name(a)` (`00573820_FUN_00573820.asm.txt:52-57` cho thứ tự push).

### 3.3 ⭐ Bảng literal → sub-op (ĐÃ SỬA, tự decode bằng script)

Decode từ `client_pseudo_c/lit_798ea4_799208.hex` bằng bảng VISCII dựng lại từ
`src/encoding.rs:128-236`, quy tắc framing `FF FF FF FF | len:u32LE | bytes | 00`:

| VA | len | bytes (hex) | VISCII → UTF-8 | sub | giá trị |
|---|--:|---|---|:--:|---|
| `0x00798EA4` | 4 | `56 E0 6E 67` | `Vàng` | `09` | hậu tố đơn vị |
| `0x00798FD8` | 18 | `4B AA 74 20 68 F4 6E 20 74 68 E0 6E 68 20 63 F4 6E 67` | `Kết hôn thành công` | `01` | kind **0** |
| `0x00798FF4` | 24 | `4B 68 F4 6E 67 20 74 68 AC 20 F0 B0 6E 67 20 67 69 BE 69 20 74 ED 6E 68` | `Không thể đồng giới tính` | `01` | kind **1** |
| `0x00799018` | 26 | `43 F3 20 6E 67 DF B6 69 20 6B 68 F4 6E 67 20 F0 FC 20 F0 C6 6E 67 20 63 A4 70` | `Có người không đủ đẳng cấp` | `01` | kind **2** |
| `0x0079903C` | 19 | `43 F3 20 6E 67 DF B6 69 20 F0 E3 20 6B AA 74 20 68 F4 6E` | `Có người đã kết hôn` | `01` | kind **3** |
| `0x00799058` | 13 | `53 65 72 76 65 72 20 F0 E3 20 F0 A5 79` | `Server đã đầy` | `01` | kind **4** |
| `0x00799070` | 36 | `43 F3 … 6B AA 74 20 68 F4 6E` | `Có người không mang dấu hiệu kết hôn` | `01` | kind **5** |
| `0x007990A0` | 7 | `53 61 69 20 6C A5 6D` | `Sai lầm` | `01`/`02`/`06` | kind/flag **0xFF** |
| `0x007990B0` | 17 | `4C 79 20 68 F4 6E 20 74 68 E0 6E 68 20 63 F4 6E 67` | `Ly hôn thành công` | `02` | flag **0** |
| `0x007990CC` | 17 | `42 D5 6E 20 63 68 BD 69 20 F0 B5 63 20 74 68 E2 6E` | `Bạn chơi độc thân` | `02` | flag **1** |
| `0x007990E8` | 22 | `54 A3 6E 67 20 6C AD 20 6B 69 6D 20 74 68 E0 6E 68 20 63 F4 6E 67` | `Tặng lễ kim thành công` | `06` | kind **0** |
| `0x00799108` | 29 | `D0 AF 69 20 70 68 DF BD 6E 67 20 6B 68 F4 6E 67 20 63 F3 20 74 72 EA 6E 20 6D D5 6E 67` | `Đối phương không có trên mạng` | `06` | kind **1** |
| `0x00799130` | 22 | `D0 AF 69 20 70 68 DF BD 6E 67 20 F0 E3 20 68 6F E0 6E 20 68 F4 6E` | `Đối phương đã hoàn hôn` | `06` | kind **2** |
| `0x00799150` | 39 | `42 D5 6E 20 6B 68 F4 6E 67 20 63 F3 20 6E 68 69 AB 75 20 74 69 AB 6E 20 6E 68 DF 20 76 A7 79 20 F0 AC 20 70 68 E1 74` | `Bạn không có nhiều tiền như vậy để phát` | `06` | kind **3** |
| `0x00799180` | 31 | `D0 AF 69 20 70 68 DF BD 6E 67 20 6B 68 F4 6E 67 20 F0 B0 6E 67 20 FD 20 6C A4 79 20 62 D5 6E` | `Đối phương không đồng ý lấy bạn` | `08` | `v==0x02` |
| `0x007991A8` | 21 | `53 AF 20 74 69 AB 6E 20 74 68 75 20 6E 67 DF B6 69 20 F0 AA 6E` | `Số tiền thu người đến` | `09` | mảnh 1 |
| `0x007991C8` | 6 | `4C AD 20 6B 69 6D` | `Lễ kim` | `09` | mảnh 3 |
| `0x007991D8` | 10 | — | `Thành công` | ❓ | generic |
| `0x007991EC` | 8 | — | `Thất bại` | ❓ | generic |

**Literal callee (đã tự decode):**

| VA | loại | bytes | text | dùng cho |
|---|---|---|---|---|
| `0x00743D30` | ShortString (len `09`) | `43 68 FA 63 20 6D D7 6E 67` | `Chúc mừng` | `0B` mảnh 1 |
| `0x00743D3C` | ShortString (len `03`) | `76 BE 69` | `với` | `0B` mảnh 3 |
| `0x00743D40` | ShortString (len `11`) | `6B AA 74 20 74 68 E0 6E 68 20 70 68 75 20 74 68 EA` | `kết thành phu thê` | `0B` mảnh 5 |
| `0x005739A8` | C-string trong `.text` | `56 BE 69 00` | `Với` | `0C` |

> ⚠️ **Đính chính `opcode_2d.md:189-202`**: bảng cũ gán `Có người đã kết hôn` = kind 3 ❌
> (thực tại `0x79903C` là kind 3 nhưng bảng cũ đặt ở basic block khác), và gán sai
> `Đối phương đã hoàn hôn` cho kind 3 (thực tại **kind 2**), `Bạn không có nhiều tiền…`
> cho kind 4 (thực tại **kind 3**). Bảng §3.3 là bảng đúng, đối chiếu trực tiếp với
> `case_041:64-105` (`UNK_0079903c` ở nhánh `bVar2==3`, `UNK_00799130` ở nhánh `bVar2==2`,
> `UNK_00799150` ở nhánh `bVar2==3` của `case 6`).

### 3.4 State field trên `TPlayer` (`gvar_007DA7BC`)

| offset | kiểu | writer | clear | giả thuyết ngữ nghĩa |
|:--:|---|---|---|---|
| `+0x1438` | `u32` | `01`(kind 0), `03` | `02`(flag 0) | **id người phối ngẫu** ⚠️ |
| `+0x1440` (+`44`) | `f64` (8B) | `01`(kind 0), `03` | `02`(flag 0) | mốc thời gian / tổng tiền lễ kim ❓ |
| `+0x1448` | `u8` | `03`(`P14`), `0D`(`P2`) | `02`(flag 0) | **cờ trạng thái hôn nhân** (0 = độc thân) ⚠️ |
| `+0x144C` | `u32` | `07` | — | **id mục tiêu đang chọn** ✅ (C→S sub 3 đọc lại) |
| `+0x1450` | `u8` | ❓ **không có writer** | — | **mã hành động / chỉ số quà** ❓ |

**Consumer duy nhất trong dump set:** `FUN_0074384C` (`0x144C`, sub `07`) và builder C→S
`asm:3189/3196` (`0x144C` + `0x1450`). **Không có renderer HUD** nào trong dump → client hiện
trạng thái hôn nhân ở code chưa decompile. ❓

---

## 4. S→C `0x45` — Hệ Thống Con Cái (chi tiết)

`case_061_00795F82_FUN_00795f82.c:28-59` — 11 nhánh `0x01..0x0B`, không `default`.
Tất cả field **fixed-width**, **không đọc chuỗi biến thiên** từ wire.

| sub | Wire (offset từ `RP`, `RP[0]=sub`) | Len | Hành động client | Guard |
|:--:|---|:--:|---|---|
| `01` | `[01][msgId:1]` | 2 | `FUN_0074FCA0` → 1 trong 6 chat line cố định | `msgId ∉ 1..6` → no-op |
| `02` | `[02][N:1]` + `N`×(`[attr:1][val:u32LE]`) | **2+5N** | `FUN_0074FE94`: toast 4000ms, set `TSe_ChildForm+0x1E8` (tab) | `attr ∉ 0x1B..0x20` → không map tab |
| `03` | `[03][v:u16LE]` | 3 | `FUN_0074EE74`: `+0x1547=v`; nếu 0 → `+0x151D=2` + toast chết | — |
| `04` | `[04][v:u16LE]` | 3 | `FUN_0074EF3C`: `+0x1551=v` | — |
| `05` | `[05][slot:1 @RP1][grade:1 @RP2][val:u16LE @RP3..4]` | **5** | `FUN_0074F034`: 6 nhánh `0x1B..0x20` | slot ngoài `0x1B..0x20` → không ghi |
| `06` | `[06][skillSlot:1][skillId:u16LE][flag:1]` | **5** | `FUN_0074F1AC`: `TFollowNpc[sel]+0x570+3*slot=skillId`, `+0x572+3*slot=flag` | `+0x151C != 0`; `skillSlot ≤ 9` |
| `07` | `[07][len:u16LE @RP1..2]` (**phải = `0x005D`**) + **`[93 byte]`** | **96** | `FUN_0074FB94`: `Move(TPlayer+0x151D, block, 0x5D)`; `block[0]==2` → toast 2000ms | **`len != 0x5D` → cả gói bị bỏ** |
| `08` | `[08][maturity:u16LE]` | 3 | `FUN_0074EFAC`: `+0x1565=v`; **`if 399 < v` → `+0x151D = 3`** | — |
| `09` | `[09][slot:1 @RP1][len:u16LE @RP2..3]` (**= `0x005D`**) + **`[93 byte]`** | **97** | `FUN_007A4C28`: `+0x151C=slot`; `block[0]==4` → `TFollowNpc[slot]+0x56C`; ngược lại → `TPlayer+0x151D` | `len != 0x5D` → bỏ; `slot ≤ 4` |
| `0A` | `[0A][mode:1][result:1][skillIdx:1]` | **4** | `FUN_005E7F90`: xóa skill slot (mode 1 = 1 slot, mode 2 = nhóm 3 slot `3*(i-1)+1..+3`); `TLH_ChildSkillDelForm+0x146=0` | `idx ≤ 9` |
| `0B` | `[0B][state:1]` | 2 | `FUN_00750240`: `TPlayer+0x151D = state` | — |

`0x00`, `≥0x0C` → no-op.

### 4.1 Layout block 93 byte (`0x5D`) — suy ra từ các writer

| off trong block | abs (TPlayer) | abs (TFollowNpc) | rộng | nghĩa | bằng chứng |
|:--:|:--:|:--:|:--:|---|---|
| `+0x00` | `0x151D` | `0x56C` | u8 | **type/lifecycle**: `4` = "đây là con"; `1/2/3` = giai đoạn | `0074fb94:57`, `0074efac:43`, `007a4c28` |
| `+0x01..0x03` | `0x151E..20` | `0x56D..6F` | 3 | — | ❓ |
| `+0x04..0x21` | `0x1521..3F` | `0x570..8E` | 10×(u16+u8) stride 3 | **10 skill slot** `{id:u16, flag:u8}` | `0074f1ac` |
| `+0x22..0x29` | `0x153F..46` | — | 8 | — | ❓ |
| `+0x2A` | `0x1547` | `0x596` | u16 | **counter mạng/đói**; 0 ⇒ chết | `0074ee74:36-41` |
| `+0x2B..0x33` | `0x1548..50` | — | 9 | — | ❓ |
| `+0x34` | `0x1551` | `0x5A0` | u16 | scalar (sub `04`) | `0074efac:37` |
| `+0x36..0x40` | `0x1553..5D` | `0x5A2..AC` | 6×u16 | **6 thuộc tính con** (không phải trang bị) | `0074f034:44-66` |
| `+0x42..0x47` | `0x155F..64` | `0x5AE..B3` | 6×u8 | **grade** tương ứng từng thuộc tính | `0074f034:45-66` |
| `+0x48` | `0x1565` | `0x5B4` | u16 | **điểm trưởng thành**; `≥400` ⇒ adult | `0074efac:41-43` |
| `+0x49..0x5C` | `0x1567..79` | — | 20 | — | ❓ |

⚠️ Lưu ý layout **chồng lấn** giữa hai bản sao: ở `TPlayer` skill array nằm ở `+0x04..0x21`
còn ở `TFollowNpc` nằm ở `+0x04..0x22` — code dispatch dự đúng vào điều này.

**5 slot follower:** `TPlayer+0x57C + 4*i`, `i ∈ 0..4` (Delphi range check `<= 4`) — **cùng**
cơ chế với pet. `TPlayer+0x151C` = index đang active.

### 4.2 ⭐ Sửa lỗi "6 slot trang bị"

`src/protocol/mod.rs:254` ghi `6 slot trang bị`. **SAI.** `FUN_0074F034`
(`0074f034_FUN_0074f034.c:44-66`) cho thấy 6 slot `0x1B..0x20` ghi **u16 value + u8 grade**,
không có bất kỳ lookup item-id nào (`FUN_0077EB9C` chỉ là decoder Word). Nhãn client giải mã
được ở §4.3 xác nhận chúng là **thuộc tính**:

| slot | nhãn client (VA, đã decode) | tab UI (sub `02`) |
|:--:|---|:--:|
| `0x1B` | `Trí lực` (`0x75018C`, `0x74ED2C`) | 1 |
| `0x1C` | `Sức công phá` (`0x75019C`, `0x74ED3C`) | 2 |
| `0x1D` | `Sức phòng vệ` (`0x7501B4`, `0x74ED54`) | 2 |
| `0x1E` | ❓ `0x7501CC` không có hex | 3 |
| `0x1F` | ❓ `0x7501E4` không có hex | 5 |
| `0x20` | ❓ `0x7501F8` không có hex | 5 |

**Đề xuất sửa doc:** `6 slot thuộc tính (INT/ATK/DEF/AGI/HP/SP) + grade`.

### 4.3 Literal con cái (đã tự decode)

| VA | len | text | sub | ghi chú |
|---|--:|---|:--:|---|
| `0x0074ED2C` | 7 | `Trí lực` | (`FUN_0074EC14`, bảng label thứ 2) | |
| `0x0074ED3C` | 12 | `Sức công phá` | ″ | |
| `0x0074ED54` | 12 | `Sức phòng vệ` | ″ | |
| `0x0074EF24` | 22 | `Đứa bé của bạn đã chết` | `03` | ⭐ xác nhận tuyệt đối `0x45` = con |
| `0x0074F4E4` | 2 | `] ` | `06` flag 1 | operand nối chuỗi |
| `0x0074F4F0` | 16 | `Kỹ năng tăng cấp` | `06` flag ≥2 | |
| `0x0074FDB8` | 33 | `Lãnh nuôi Đời 2 võ t…` *(dump cắt)* | `01` msg 2 | ⚠️ token `Đời 2 võ tướng` |
| `0x0074FE70` | 32 | `Bạn đã không nuôi Đời 2 võ tướng` | `01` msg 6 | = thông báo **thất bại** |
| `0x00750180` | 2 | `Dư` | `02` | separator giữa các record |
| `0x0075018C` | 7 | `Trí lực` | `02` | label `0x1B` |
| `0x0075019C` | 12 | `Sức công phá` | `02` | label `0x1C` |
| `0x007501B4` | 12 | `Sức phòng vệ` | `02` | label `0x1D` |
| `0x005E82BC` | 42 | `Xoá kỹ năng thành công của Đời 2 võ tướng ` | `0A` result 0 | |
| `0x005E82F0` | 39 | `Xoá kỹ năng thất bại của Đời 2 võ tướng` | `0A` result 1/2/5/FF | ⚠️ khôi phục theo độ dài |

⚠️ **Token `Đời 2 võ tướng`** xuất hiện verbatim ở 4 literal. Không parse được như tiếng Việt.
Có thể là (a) tên creature/instance cố định bị MT hỏng, hoặc (b) tên NPC mà `Npc.dat` không
có (xem §5.3). ❓ **Cần capture live để chốt.**

### 4.4 Sửa lỗi thứ tự nối chuỗi của sub `02`

`opcode_45.md` ghi "`description + IntToStr(value)`". `FUN_0074FE94` cho thấy
(`0074fe94_FUN_0074fe94.c:60-105`): nếu không phải record đầu thì `_LStrCat(DAT_00750180)`,
rồi `IntToStr(value)`, rồi `_LStrCat(<label theo attr>)`, rồi `_LStrCat(DAT_0075020C)`.
⇒ thứ tự đúng là **`[sep "Dư"] + str(value) + <label> + suffix`**, **số đứng trước nhãn**.

---

## 5. Game Data (`Data/`) — bằng chứng cho thiết kế server

### 5.1 Item kết hôn (giải mã `Item.dat` bằng đúng thuật toán `src/data/loaders/item.rs`)

| id | tên (VISCII) | kind | ghi chú |
|--:|---|:--:|---|
| **19760** | `Lễ Phục Tân Lang` | 10 | **áo cưới nam** (kind 10 = equipment) |
| **19761** | `Lễ Phục TânNương` | 10 | **áo cưới nữ** |
| **46001** | `Tiếng sét ái tình` | 38 | chuỗi quà tình #1 |
| **46002** | `Mối tình đầu` | 38 | #2 |
| **46003** | `Tình yêu say đấm` | 38 | #3 |
| **46004** | `T.yêu cuồng nhiệt` | 38 | #4 |
| **46005** | `Sôcôla chân tình` | 38 | #5 — ⭐ **5 món khớp `kind 1..5` của `2D 01`/`2D 06`** |
| **46046** | `Lễ hợp thành hôn` | 38 | nghi lễ cầu hôn |
| **46047** | `Lễ pháo thành hôn` | 38 | nghi lễ cưới |
| **46137** | `Kim tất lễ hộp` | 38 | ⭐ nghi "kim tất" = hủy hôn |
| 46253/54/55/56 | `Túi bảo chân tình` / `Túi bảo thâm tình` | 38 | quà tình |
| 46257/46258 | `Thần tình yêu` / `Thần tình yêu thật` | 38 | quà tình |
| 23119 | `Khăn đính hôn` | 14 | khăn cưới |
| 23275 | `Hôn thụy huy hiệu` | 14 | |
| 29008 | `Thư từ hôn Tuân Thái` | 20 | thư từ hôn |
| 23191 | `Kim sanh duyên` | 14 | ⭐ "sanh duyên" = sinh con |
| 23192/93/94 | `Nhẫn kim duyên` / `Nhẫn vô song` / `Nhẫn thất giới` | 14/11 | nhẫn cưới |

> **Giả thuyết mạnh (⚠️INFERRED):** `player+0x1450` (byte mã hành động gửi lên ở `2D 03`)
> = **index quà 1..5** → ánh xạ `46001..46005`. Khớp với việc `2D 01`/`2D 06` có **kind 1..5**.
> Cần capture live để xác nhận.

### 5.2 Không có item riêng cho con

Tìm `nuôi|đẻ|trẻ|bé|nôi|mắt sữa` trong 8371 item → chỉ `65167 Bảo châu nuôi người` (kind 65,
buff) và `29209 Hài cốt đứa trẻ` (kind 20, trang sức). ❌ **Không có block item con riêng** ⇒
server phải tự thiết kế, hoặc dùng chung item pool cho quà tăng con.

### 5.3 NPC / map

⚠️ Chưa decode được `Npc.dat` / `eve.emg` trong phiên này. Nhưng:
- `opcode_2d.md` §3 cho thấy handler `0x2D` **không lookup NPC nào** — thuần banner + state.
- Pattern `0x2D 0B/0C/0F` ghép **hai tên người chơi** ⇒ mô hình **player-to-player**, không
  phải NPC-mediated.
- Nguồn `.kilo/worktrees/marvelous-observation/Data/Npcs.txt` (~6700 record, decode bởi C#)
  **không có** NPC nào khớp `Hôn|Cưới|Nhẫn|Lễ kim|Bảo Sanh|Tư Mắm|Tiến Sĩ`.
⇒ ⚠️ **Nhiều khả năng không có NPC hôn nhân cố định**; luồng đi qua UI form + `0x2D 0C`
(danh sách cặp) + `0x2D 0A` (popup trên đầu người chơi). Cần decode `Npc.dat` + `eve.emg` để
chốt.

### 5.4 `TS_Server_Bear/` — KHÔNG có tham chiếu

Grep `marriage|spouse|marry|child|wedding|husband|wife` trong ~300 file `.cs` → **0 hit**.
`TS_Server_Bear/TS_Server/PacketProcessor.cs:100-168` không có `case 45` lẫn `case 0x2D`.
⇒ **Phải thiết kế từ protocol + data, không có mốc server tham chiếu.**

---

## 6. Gap phía Server Rust

### 6.1 Trạng thái hiện tại

| Hạng mục | Trạng thái | Bằng chứng |
|---|---|---|
| Constant `0x2D` | **ĐẶT TÊN SAI** `OP_REBORN` = "Reborn (Chuyển sinh nhân vật)" | `src/protocol/mod.rs:180` |
| Constant `0x45` | Đúng `OP_CHILD`, nhưng doc sai ("6 slot trang bị") | `mod.rs:254` |
| Dispatch `0x2D` | `dispatcher.rs:375` → `unimplemented::handle` | đọc trực tiếp |
| Dispatch `0x45` | `dispatcher.rs:388` → `unimplemented::handle` | đọc trực tiếp |
| Handler `0x2D` | **không tồn tại** | `ls src/server/handlers/` |
| Handler `0x45` | **không tồn tại** | ″ |
| Builder S→C `2D`/`45` | **không tồn tại** | `grep "2D03\|4507" src/` → 0 |
| State trong `Session` | **không có field nào** | `session.rs:131-273` |
| Frame trong Logined1 | **không có** | `spawn.rs:471-604` (22 bước) |
| Bảng DB quan hệ | **không có** (chỉ `friends`, `migrations/0001_init.sql:244-249`) | đọc `0001_init.sql` |
| Auto-save | `SAVE_TABLES = ["stats","homdo","trangbi","quest","pet"]` — thiếu `marriage`/`child` | `auto_save.rs:23` |
| Persist allowlist | `persist.rs:23-49` không có cột mới | đọc |
| Test | `tests/` không có test nào cho `0x2D`/`0x45` | `ls tests/` |

### 6.2 Bảng gap

| # | Mức | Gap | File cần sửa | Ghi chú |
|:-:|:--:|---|---|---|
| **G1** | **BLOCKER** | Đổi tên `OP_REBORN` → `OP_MARRIAGE` (0x2D) | `protocol/mod.rs:180,292,352`; `spec/server_main_opcode.md:43` | rename thuần, alias `#[deprecated]` theo convention repo |
| **G2** | **BLOCKER** | Thêm arm C→S `0x2D` + `0x45` vào dispatcher | `dispatcher.rs:252-397` | parse `2D 01/02/03/04`, `45 01/02` |
| **G3** | **BLOCKER** | Module `src/server/handlers/family.rs` (hoặc `marriage.rs` + `child.rs`) | `handlers/mod.rs:1-22` | entry `handle_marriage`, `handle_child` |
| **G4** | **BLOCKER** | Builder S→C `2D 01/02/03/06/07/09/0B/0D` | file mới | xem §7.1 |
| **G5** | **BLOCKER** | Builder S→C `45 02/03/05/07/08/09/0A/0B` + **93-byte block** | file mới | xem §7.2 |
| **G6** | **BLOCKER** | Migration `00XX_family.sql`: `character_marriage`, `character_children`, `character_child_skills` | `migrations/` | mẫu: `character_pets` `0001_init.sql:130-172` |
| **G7** | **HIGH** | Field `Session::marriage` + `Session::child` | `session.rs:237` (sau `active_pet_stt`) + `Default` tại `session.rs:366` | giữ style group |
| **G8** | **HIGH** | Domain model + repository trait | `db/modern/model.rs`, `db/modern/traits.rs` | `ChildRecord` theo `PetRecord` (`model.rs:122-146`) |
| **G9** | **HIGH** | SQLite impl | `db/modern/sqlite/` | + `persist.rs` `persist_*` fn |
| **G10** | **HIGH** | Thêm `"marriage","child"` vào `SAVE_TABLES` + fingerprint | `auto_save.rs:23,38` | nếu không sẽ không bao giờ auto-save |
| **G11** | **HIGH** | Frame sync trong Logined1 | `spawn.rs:~602` (sau `quest_sync::sync_frames`) | **phải guard theo state** để không phá golden parity |
| **G12** | **HIGH** | Cờ trong `persist.rs::character_column()` | `persist.rs:23-49` | nếu không sẽ `tracing::warn!` mỗi lần ghi |
| **G13** | **MEDIUM** | Business rules kết hôn | file mới | 6 rule từ literal §3.3 + ngưỡng đẳng cấp tự chọn |
| **G14** | **MEDIUM** | Wedding gift: trừ tiền, map `2D 06` result, `2D 09` số tiền | file mới | sub `06` kind 3 = không đủ tiền |
| **G15** | **MEDIUM** | Cờ hôn nhân cho **cả hai vợ chồng** + clear state | file mới | `2D 02` flag 0 clear 4 field |
| **G16** | **MEDIUM** | Con: 5 slot follower + tạo/nuôi/giết/chết/trưởng thành | file mới | ngưỡng 400 |
| **G17** | **MEDIUM** | Con: 6 slot thuộc tính + grade, nguồn tăng điểm | file mới | chưa có nguồn tăng trong server |
| **G18** | **MEDIUM** | Con: skill 10 slot / 3 nhóm xóa | file mới | |
| **G19** | **MEDIUM** | Con hiện trên map + trong battle | `battle/`, `spawn.rs` | client gate `TFollowNpc+0x56C == 4` |
| **G20** | **LOW** | Sửa doc `mod.rs:254` "6 slot trang bị" → "6 slot thuộc tính" | `mod.rs:254` | |
| **G21** | **LOW** | Sửa `opcode_2d.md` §6 + bảng literal §"hiệu đính" | `.scratch/…/opcode_2d.md` | xem §9 |
| **G22** | **LOW** | Sửa `opcode_45.md` thứ tự nối chuỗi sub `02` | `.scratch/…/opcode_45.md` | |
| **G23** | **BLOCKER (khác)** | `src/data/loaders/item.rs:164` dùng `cp1252_to_string` cho tên item | `item.rs:164` | đổi sang `viscii_decode`; đã chứng minh hỏng ~30% tên |
| **G24** | **LOW** | Cần `lit_*.hex` cho: `0x74FC88`, `0x74F4D0`, `0x74FDE4`, `0x74FE10`, `0x74FE3C`, đuôi `0x74FDB8`, `0x7501CC/E4/F8`, `0x75020C`, `0x750228`, `0x5E82F0`, `0x743AA4..B04`, `0x74403C/48`, `0x7988B8` | `client_pseudo_c/` | dump thêm từ `aLogin.exe` |

### 6.3 Điểm chèn đã xác minh

| Vị trí | Nội dung |
|---|---|
| `dispatcher.rs:375` | thay `OP_REBORN` khỏi nhóm unimplemented → `family::handle_marriage` |
| `dispatcher.rs:388` | thay `OP_CHILD` → `family::handle_child` |
| `session.rs:237` | chèn `pub marriage: MarriageState` + `pub child: ChildState` ngay sau `active_pet_stt` (`:236`) |
| `session.rs:366-367` | thêm default tương ứng (sau `pets: Vec::new(), active_pet_stt: 0,`) |
| `session.rs:468+` | thêm `dump_marriage_state()` / `dump_child_block()` cạnh `dump_table` (`:450-468`) |
| `spawn.rs:~602` | `frames.extend(family::sync_frames(s));` ngay sau `quest_sync::sync_frames(s)` — **có guard** |
| `response.rs:69+` | thêm `send_marriage_state()`, `send_child_block()` cạnh `send_storage_items()` (`:66-69`) |
| `auto_save.rs:23` | `["stats","homdo","trangbi","quest","pet","marriage","child"]` |
| `persist.rs:23` | thêm cột mới vào `character_column()` allowlist |
| `handlers/mod.rs` | `pub mod family;` |
| `migrations/000XX_family.sql` | 3 bảng mới |
| `tests/family_flow_test.rs` | theo `AGENTS.md`: **cấm `#[cfg(test)]` trong `src/`** |

---

## 7. Khung wire để implement (contract server)

### 7.1 Push S→C `0x2D` (dùng `crate::protocol::frame("2D03", &body)`)

```
2D 01 00 <u32 spouseId LE> <f64 LE>      → "Kết hôn thành công" + ghi state
2D 01 01                               → "Không thể đồng giới tính"
2D 01 02                               → "Có người không đủ đẳng cấp"
2D 01 03                               → "Có người đã kết hôn"
2D 01 04                               → "Server đã đầy"
2D 01 05                               → "Có người không mang dấu hiệu kết hôn"
2D 01 FF                               → "Sai lầm"
2D 02 <u32 junk> 00                     → clear state + "Ly hôn thành công"
2D 02 <u32 junk> 01                     → "Bạn chơi độc thân"
2D 02 <u32 junk> FF                     → "Sai lầm"
2D 03 <u32 spouseId> <f64> <u8 status>  → ghi full state, KHÔNG banner (login sync)
2D 06 00                               → "Tặng lễ kim thành công"
2D 06 01                               → "Đối phương không có trên mạng"
2D 06 02                               → "Đối phương đã hoàn hôn"
2D 06 03                               → "Bạn không có nhiều tiền như vậy để phát"
2D 06 FF                               → "Sai lầm"
2D 07 <u32 targetId> <u16 idx>          → set player+0x144C + render portrait
2D 08 02                               → "Đối phương không đồng ý lấy bạn"
2D 09 <u32 id> <u32 amount>             → "Số tiền thu người đến <tên> Lễ kim <n> Vàng"
2D 0B <u32 id1> <u32 id2>               → chat "Chúc mừng A với B kết thành phu thê"
2D 0C <u32 n> <(u32 a)(u32 b) × n>      → rebuild danh sách cặp ("<b> Với <a>")
2D 0D <u8 status>                       → set player+0x1448
2D 04 / 2D 05                           → refresh window / list
```

### 7.2 Push S→C `0x45`

```
45 01 <u8 msgId 1..6>                          → chat line cố định
45 02 <u8 N> <(u8 attr)(u32 val LE) × N>        → toast thuộc tính, set tab
45 03 <u16 LE v>                               → v==0 ⇒ client toast "Đứa bé của bạn đã chết"
45 04 <u16 LE v>                               → counter phụ
45 05 <u8 slot 0x1B..0x20> <u8 grade> <u16 LE val>
45 06 <u8 skillSlot 0..9> <u16 LE skillId> <u8 flag>
45 07 <u16 LE 0x005D> <93 byte block>           ← ★ PHẢI len == 0x5D
45 08 <u16 LE maturity>                        ← ≥400 client tự set state=3
45 09 <u8 slot 0..4> <u16 LE 0x005D> <93 byte block>  ← block[0]==4 ⇒ ghi vào follower
45 0A <u8 mode> <u8 result> <u8 skillIdx>        ← kết quả xóa skill
45 0B <u8 state>                                 ← set TPlayer+0x151D
```

### 7.3 Nhận C→S (handler phía server)

```
2D 01 <string bytes>              → ❓ loại 1: có vẻ là tên/đề xuất
2D 02 <string bytes>              → ❓ loại 2
2D 03 <u32 LE targetId> <u8 code> → ★ cầu hôn / tặng quà (code 1..5 ↔ item 46001..46005)
2D 04                            → ack rỗng
45 01 <u16 LE skillSlot> <u8 grp> <u8 sel>  → thao tác skill con (chọn/xác nhận)
45 02 <u8 grp> <u8 sel>                     → xác nhận xóa skill con
```

> ⚠️ `2D 01`/`2D 02` body là **AnsiString thô không length-prefix** — server **không parse được**
> biên (frame đã có length). Phải so khớp byte với tên trong DB.

---

## 8. Câu hỏi còn mở (❓)

| # | Câu hỏi | Ảnh hưởng |
|:-:|---|---|
| Q1 | `player+0x1450` là gì? Không có writer. Nếu = index quà 1..5 thì map `46001..46005`. | Cao — quyết định payload `2D 03` |
| Q2 | `2D 01` và `2D 02` mang ý nghĩa gì? Nguồn: `(*gvar_007D9C1C)+0x1D0` và `(*gvar_007DA260)+0x150` là 2 form khác nhau. | Cao |
| Q3 | Ngưỡng "đẳng cấp" cho kết hôn là bao nhiêu? Không có trong `Npc.dat`/`Item.dat`/protocol. | Trung bình — thiết kế |
| Q4 | Có NPC hôn nhân không? Cần decode `Npc.dat` + `eve.emg`. | Trung bình |
| Q5 | Token `Đời 2 võ tướng` là tên entity hay MT garbage? | Thấp |
| Q6 | `+0x151D == 2` nghĩa là **sống** hay **đã chết**? Sub `03` set `2` kèm toast "đã chết", nhưng `FUN_00750278` coi `==2` là *đang hoạt động*. **Mâu thuẫn thật.** | Cao — quyết định vòng đời con |
| Q7 | 17 byte chưa map trong block 93B (`+0x22..0x29`, `+0x2B..0x33`, `+0x49..0x5C`). | Thấp — có thể zero |
| Q8 | `2D 0E` gọi `FUN_0071DC74(actor)` — tác dụng gì? Không có body. | Thấp |
| Q9 | `2D 10` ghi record vào `*(gvar_007DA600+0xE8)` — đối tượng này là gì? | Trung bình |
| Q10 | Không có ASM cho `0x00794977`/`0x00795F82`; `aLogin.exe` không có trong workspace. | Trung bình |

---

## 9. Danh sách sửa tài liệu cũ

| File:line | Claim cũ | Sửa thành |
|---|---|---|
| `opcode_2d.md:130-131` | "`case 0x2d: break;` → client không bao giờ gửi OP này" | ❌ **SAI.** Builder `0x0078896A`, 4 sub-action. Đây là lỗi recovery Ghidra dạng đã ghi ở `post_login…md` §7.5. |
| `opcode_2d.md:189-202` | bảng literal → sub | Xem §3.3 (kind 3/4/5 của sub `01`, kind 2/3 của sub `06` bị gán sai) |
| `opcode_2d.md:3` | "Chiều C→S … một chiều" | Có cả hai chiều |
| `opcode_2d.md:141` | "`[2D][0F][id1][id2]` 10B" | đúng, nhưng thêm: 2 literal chưa dump |
| `opcode_45.md:250-253` | "`case 0x45: break;` → không có chiều C→S" | ❌ **SAI.** Builder `0x00789DDB`, 2 sub-action |
| `opcode_45.md` §4.5-2 | thứ tự nối chuỗi sub `02` | `[sep] + str(value) + <label> + suffix` (số trước nhãn) |
| `mod.rs:180` | `OP_REBORN` = "Chuyển sinh nhân vật" | → `OP_MARRIAGE` (bus hôn nhân) |
| `mod.rs:254` | "6 slot trang bị" | "6 slot thuộc tính (INT/ATK/DEF/AGI/HP/SP) + grade" |
| `spec/server_main_opcode.md:43,64` | tương ứng | đồng bộ theo trên |
| `item.rs:164` | `cp1252_to_string` cho tên item | `viscii_decode` — `CP1252` hỏng ~30% tên |
| `opcode_2d.md:4` "0x2D không chứa NPC lookup" | — | đúng, giữ |

---

## 10. Thứ tự triển khai đề xuất

1. **G23** — sửa `item.rs:164` (độc lập, giá trị ngay, cần cho mọi bước sau vì tên item sai hỏng).
2. **G1** — rename `OP_REBORN` → `OP_MARRIAGE` + `#[deprecated]` alias.
3. **G6** — migration 3 bảng.
4. **G7/G8/G9** — `MarriageState`, `ChildState` vào `Session` + model + repo + sqlite.
5. **G4/G5** — builder S→C theo §7.1/§7.2 + test golden trong `tests/`.
6. **G11** — `family::sync_frames` có guard trong Logined1 (sau `quest_sync`, để không phá golden).
7. **G2/G3** — handler C→S `0x2D`/`0x45`.
8. **G13/G14/G15** — business rules kết hôn + lễ kim (map `46001..46005`).
9. **G16/G17/G18/G19** — vòng đời con (giải quyết **Q6** trước khi code).
10. **G10/G12** — auto-save + persist allowlist.
11. **G20..G24** — sửa doc + dump hex còn thiếu.
12. **G24** — capture live bằng `aLogin.exe` để chốt **Q1, Q2, Q6**.

---

## 11. Công cụ tái lập (đã commit kèm tài liệu)

```bash
# Giải mã mọi literal Delphi AnsiString / shortstring trong một dải địa chỉ.
# Tự dựng bảng VISCII từ src/encoding.rs::viscii_to_unicode + fallback Latin-1
# (giống hệt hàm Rust), tự đọc dải địa chỉ từ header của từng lit_*.hex.
python3 .scratch/client-pseudo-op-code/tools_decode_literals.py 798ea4 799208
# → "Kết hôn thành công", "Ly hôn thành công", "Tặng lễ kim thành công", …

python3 .scratch/client-pseudo-op-code/tools_decode_literals.py 74ef24 74ef3c
# → "Đứa bé của bạn đã chết"

# Giải mã Item.dat bằng đúng thuật toán src/data/loaders/item.rs, in SIDE-BY-SIDE
# bản giải VISCII (đúng) và bản cp1252 (đang dùng trong item.rs:164 — sai).
python3 .scratch/client-pseudo-op-code/tools_decode_items.py 'thành hôn'
# → 46046 VISCII='Lễ hợp thành hôn'  CP1252='L\xad hþp thành hôn'  kind=38
#    46047 VISCII='Lễ pháo thành hôn' CP1252='L\xad pháo thành hôn' kind=38
```

⚠️ Khi thêm file `lit_*.hex` mới, **hai tool trên tự động hỗ trợ** — không cần sửa code.

---

## 12. Evidence ledger

| Claim | Nhãn | Nguồn |
|---|:-:|---|
| `0x2D` → case 41 → `0x00794977` | ✅ | `jumptable_byte200_0x78A8EE.hex:3`; `jumptable_0x78A9B6_case_functions.csv:43` |
| `0x45` → case 61 → `0x00795F82` | ✅ | `jumptable_byte200_0x78A8EE.hex:5`; csv `:63` |
| C→S builder `0x2D` @ `0x0078896A`, 4 sub | ✅ | `asm:3101-3229` + `table_0x77F474_200B.hex` + `table_0x77F53C_dword200.hex` |
| C→S builder `0x45` @ `0x00789DDB`, 2 sub | ✅ | `asm:4428-4529` + 2 bảng hex |
| `0x44` không có builder (map về epilogue) | ✅ | `table_0x77F474[0x44]=0x00`; `dword[0]=0x0078A4F2`; `asm:4755` |
| Byte `0x01` là ShortString length prefix, không phải wire | ✅ | `00402b90_FUN_00402b90.c` + `00402b60__PStrNCat.c` + control `0x08` (`asm:445-501`) |
| `FUN_0077EB1C` = u16→2 byte LE; `FUN_0077EE84` = u32→4 byte LE | ✅ | `0077eb1c_FUN_0077eb1c.c`, `0077ee84_FUN_0077ee84.c` |
| 14 literal kết hôn + ánh xạ sub | ✅ | decode tự viết từ `lit_798ea4_799208.hex` + `case_041:64-105` |
| `Chúc mừng` / `với` / `kết thành phu thê` | ✅ | `lit_743d30_743d7f.hex` (0x43D30/3C/40) |
| `Với` (shortstring) | ✅ | `lit_5739a8.hex:4` = `56 BE 69 00` |
| `Đứa bé của bạn đã chết` | ✅ | `lit_74ef24.hex:4` = `D0 D1 61 20 62 E9 20 63 FC 61 20 62 D5 6E 20 F0 E3 20 63 68 AA 74` |
| Labels thuộc tính con | ✅ | `lit_74ed2c.hex`, `lit_750180.hex` |
| block 93B (`0x5D`) + ngưỡng 400 | ✅ | `0074fb94_FUN_0074fb94.c:48-57`; `0074efac_FUN_0074efac.c:41-43` |
| 6 slot `0x1B..0x20` = thuộc tính, **không** phải trang bị | ✅ | `0074f034_FUN_0074f034.c:44-66` |
| `player+0x144C` = target id (ghi ở `2D 07`, đọc ở C→S `2D 03`) | ✅ | `0074384c_FUN_0074384c.c:63-69`; `asm:3189` |
| `player+0x1450` không có writer | ✅ | grep `0x1450` trong `client_pseudo_c/*.c` → 0 hit |
| Item `46046/46047/19760/19761/46001..46005` | ✅ | decode `Data/Item.dat` bằng thuật toán `item.rs:119-170` |
| `item.rs:164` CP1252 hỏng tên | ✅ | cùng script, cột `CP1252` vs `VISCII` |
| `TS_Server_Bear` không có kết hôn/con | ✅ | grep 6 từ khoá + `PacketProcessor.cs:100-168` |
| Server Rust chưa implement gì cho `0x2D`/`0x45` | ✅ | `dispatcher.rs:375,388`; `session.rs:131-273`; `migrations/0001_init.sql` |
| Ngữ nghĩa `2D 01`/`2D 02` C→S | ❓ | không có literal đối chiếu |
| `+0x151D==2` nghĩa là sống hay chết | ❓ | mâu thuẫn nội tại client |
| Ngưỡng "đẳng cấp" kết hôn | ❓ | không có trong data |
| Có NPC hôn nhân không | ❓ | chưa decode `Npc.dat`/`eve.emg` |

---

*Tài liệu tạo 2026-10-02. Toàn bộ claim ✅ đã được tự đối chiếu bằng ASM/HEX/script trong phiên
này; các claim ⚠️/❓ được ghi rõ. Nguồn chính `client_pseudo_c/`, đối chiếu
`test/ts_decompile/`, dữ liệu game từ `Data/` với thuật toán của chính `src/data/loaders/`.*
