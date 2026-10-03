# 03 — CÁCH THỨC TRIỂN KHAI SKILL TRONG TRẬN & QUY TẮC COMBO (aLogin.exe)

Ngày khảo sát: 2026-10-01 · Nguồn chính: `../../../../client_pseudo_c/` + đối chiếu
`/mnt/d/VUDT/GIT_PCC/test/ts_decompile/` (SSOT đầy đủ 13 176 file).
Đối chiếu server: `../../../src/battle/*`, `../../../src/server/handlers/{battle,skills}.rs`.

**Mức bằng chứng dùng trong tài liệu**

| Mức | Ý nghĩa |
|:--|:--|
| **(A)** | Đã xác minh trực tiếp từ mã máy/asm client (đọc byte-for-byte, không suy đoán) |
| **(B)** | Suy luận có cơ sở từ (A) + hành vi quan sát được / đối chiếu chéo |
| **(C)** | Chỉ là contract phía server Rust (chưa có bằng chứng client) |

> ⚠️ **Không dùng `case N: break;` rỗng của Ghidra làm bằng chứng.** Mọi kết luận "client
> không làm gì" dưới đây đều dựa trên **bảng dispatch + vùng byte dump**, không dựa vào
> C decompile (xem `post_login_stats_inventory_flow.md` §7).

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — 5 sub `0..4` GAP nay ĐÃ ĐÓNG. Xem §0.1**

---

## 0.1 🔴 ROUND-4 REDUMP — ĐÃ ĐÓNG GAP CỦA §1.1(b)

### 0.1.1 Đã thu hồi gì

| Artifact | File | Coverage |
|---|---|---|
| Bảng 5 entry `0x780440` | `cs0b_780440_jmptbl_5xDWORD.hex` | 20 B / 20 B |
| Builder `0x78048D..0x7807D0` | `asm_cs0b_78048d_builders.asm.txt` | **836 B / 836 B — 100%** |

Nội dung bảng:

```
[0] 0x0078048D   [1] 0x00780511   [2] 0x00780595   [3] 0x00780671   [4] 0x00780721
```

⇒ §1.1(b) nâng từ *"mức A cho 6/9 sub; **GAP cho 0..4**"* → **mức A cho 11 sub**.

### 0.1.2 Wire layout sub `0..4` — **(C) → (A) byte-verified**

Mẫu chung (đọc từ `asm_cs0b_78048d_builders.asm.txt`):

```asm
LEA  EAX,[EBP + -0x34]
MOV  DL, byte ptr [EBP + -0x5]      ; main opcode = 0x0B
MOV  byte ptr [EAX + 0x1], DL
MOV  byte ptr [EAX], 0x1            ; DO DAI ShortString -- KHONG len wire
CALL 0x00402b90                     ; @LStrFromString -> ShortString{0x0B}
MOV  CL, 0x2 / CALL 0x00402b60      ; @PStrNCat cap=2 -> "0B"  (1 byte)
CALL 0x0040402c                     ; @LStrFromString
...                                  ; @LStrCatN cap=N
CALL 0x0051633c                     ; TForm1_CY_AddSedQueue
```

| Sub | Payload sau `[0B][sub]` | Tổng | Field nguồn | Mức |
|:-:|---|--:|---|:-:|
| `0` | `[b:1]` | **2 B** | `*(gvar_007DA5AC)+0x130` | **A** |
| `1` | `[b:1]` | **2 B** | `*(gvar_007DA5AC)+0x130` (temp khác) | **A** |
| **`2`** | **`[b:1]` + `u32 LE` + `u16 LE`** | **9 B** | `gvar_007D9CE4+0x24` / `+0x1E` (qua `FUN_0077EE84`) / `+0x22` (qua `FUN_0077EB1C`) | **A** |
| `3` | `u32 LE` + `u16 LE` | **7 B** | `+0x1E`, `+0x22` | **A** |
| `4` | `u32 LE` + `u16 LE` | **7 B** | `+0x1E`, `+0x22` | **A** |

ASM `sub 2` (`asm_cs0b_78048d_builders.asm.txt:91-133`):

```asm
007805db: MOV DL, byte ptr [EDX + 0x24]     ; b  -> ShortString{0x0B, b}
007805e0: MOV CL, 0x3 / CALL 0x00402b60     ; cap=3 -> 2 byte "0B<b>"
007805fa: CALL 0x0040402c                  ; @LStrFromString
00780612: MOV EDX, dword ptr [EDX + 0x1e]  ; u32
0078061c: CALL 0x0077ee84                  ; u32 -> 4 byte LE
00780634: MOV DX, word ptr [EDX + 0x22]    ; u16
0078063f: CALL 0x0077eb1c                  ; u16 -> 2 byte LE   <<< 2 BYTE
00780655: CALL 0x00404148                  ; @LStrCatN cap=3
00780667: CALL 0x0051633c                  ; send
```

### 0.1.3 ⭐ Phân biệt sub `2` và sub `8` — dễ nhầm, KHÁC NHAU 1 byte

| Sub | Builder | Cách đưa `TFightManage+0x22` vào gói | Tổng |
|:-:|:-:|---|--:|
| **`2`** | `0x780595` | `CALL 0x0077EB1C` = u16 → **2 byte LE** | **9 B** |
| **`8`** | `0x7808E2` | `CALL 0x00403FA0` = `@LStrFromChar` → **chỉ low8** | **8 B** |

ASM sub 8 (`0077f414_FUN_0077f414.asm.txt`):

```asm
00780982: MOV DX, word ptr [EDX + 0x22]
00780986: CALL 0x00403fa0          ; @LStrFromChar -> 1 BYTE (low byte)
00780997: MOV EDX, 0x3
0078099c: CALL 0x00404148          ; @LStrCatN cap=3
```

> §1.1(b) dòng 142 và ghi chú `§147-149` **đã đúng** về sub 8. Nay có thêm xác nhận
> sub 2 dùng **2 byte** ⇒ hai sub này *không* thể dùng chung một builder Rust.

### 0.1.4 ✅ Đóng gap H-3 (`§448`) — Rust ĐÚNG, không off-by-one

> §448 (mức 🟡 H-3, *"builder **chưa có byte** (GAP)"*)

**ĐÃ VERIFY byte-level.** Rust không có off-by-one vì `dispatcher.rs:228` đặt
`payload = decoded.get(6..)` — **đã bỏ qua `[main][sub]`**:

| Wire | `payload` | Rust `battle.rs` | Khớp |
|---|---|---|:-:|
| `0B 02 <b> <u32> <u16>` | `[0]=b` | `:97  inner = payload.first()` | ✅ |
| | `[1..5]=u32` | `:110 u32_le_slice(&payload[1..5])` | ✅ |
| | `[5..7]=u16` | `:117 u16_le_slice(&payload[5..7])` | ✅ |

⇒ **§448 nâng từ 🟡 lên (A). Không sửa code Rust cho sub 2.**

### 0.1.5 ✅ Bảng dispatch `0x780440` nay có byte

> §127: *"bảng 5 phần tử — **CHƯA CÓ BYTE**"* → ✅ **Đã có** (xem §0.1.1).

### 0.1.6 ✅ Xác nhận §126 — chỉ sub `5` là no-op

> §126: `CMP EAX,0x4 ; JA 0x78a4f2  ; *** sub 5 → NO-OP ***`

**Đúng.** Đọc lại dispatch head từ `functions/0077f414_FUN_0077f414.asm.txt`:

```asm
00780420: XOR  EAX,EAX
00780422: MOV  AL, byte ptr [EBP + -0x6]        ; sub
00780425: CMP  EAX,0x6
00780428: JG   0x00780454                       ; sub > 6 -> chuoi 0x780454
0078042a: JZ   0x007807d1                       ; sub == 6 -> BUILDER (KHONG phai no-op)
00780430: CMP  EAX,0x4
00780433: JA   0x0078a4f2                       ; sub == 5 -> EPILOGUE = KHONG GUI
00780439: JMP  dword ptr [EAX*0x4 + 0x780440]   ; sub 0..4
00780454: CMP  EAX,0x9 / JZ 0x7809b8            ; sub 9
0078045f: SUB  EAX,0x7 / JZ 0x780826            ; sub 7
00780468: DEC  EAX     / JZ 0x7808e2            ; sub 8
0078046f: JMP  0x0078a4f2                       ; sub 5,6 rơi tới day
00780474: SUB  EAX,0xa   / JZ 0x780a0d          ; sub 0x0A
0078047d: SUB  EAX,0xf0  / JZ 0x780abd          ; sub 0xFA
00780488: JMP  0x0078a4f2
```

**Tập sub `0x0B` hợp lệ: `{0, 1, 2, 3, 4, 6, 7, 8, 9, 0x0A, 0xFA}`.**

⚠️ **Chỉ sub `5` là no-op.** Sub `6` **CÓ** builder `0x007807D1` và **được gửi**:
`ShortString{0x0B, 0x06}` → `@LStrFromString` → `CY_AddSedQueue` ⇒ payload
`[0B][06]` = **2 byte, không body** — đúng như §1.1(b) dòng 140.

> **Cảnh báo chống hiểu nhầm:** dễ tưởng *"sub 5 và 6 đều rơi vào
> `0x0078046f: JMP 0x0078a4f2`"* ⇒ cả hai chết. **Sai** — `JG 0x780454` tại `0x780428`
> rẽ cả `sub 5` lẫn `sub 6` xuống dưới, nhưng `JZ 0x7807d1` bắt `sub 6` trước.
> Chỉ `sub 5` mới tới `0x780433: JA 0x78a4f2`.

### 0.1.7 ❌ Đính chính §512 — `FUN_00654590` ĐÃ CÓ BODY

> §5.1 mục 5 (🟡): *"`FUN_00654590` (callee OP 0x35 SubOp 4) — vẫn **không có body** trong corpus"*

**SAI.** `ts_decompile/functions/00654590_FUN_00654590.c` + `.asm.txt` **đã tồn tại**
(từ trước Round-4). `battle/02_battle_positions.md:552` và `battle/README.md:310`
nói đúng — **`03` sai**. Xoá khỏi danh sách việc cần làm.

### 0.1.8 §5.1 mục 6 — VMT đã có dữ liệu

`PTR_PTR_00589708` / `PTR_PTR_00577938`: directive `VMT` đã chạy, kết quả nằm trong
`ts_decompile/redump/round3_report.txt` (mục `VMT`). Đọc report để lấy class name —
đây là việc đọc, **không cần redump thêm**.

---

## 0. TIMELINE — người chơi bấm skill → frame nào

```
                    ┌── bấm chuột phải / bấm nút "Đánh" trong ô trận
                    ▼
 TForm1.DXDraw1MouseDown (0x0050BFF8)
   ├─ param_3 == 0 (CHUỘT TRÁI)  → FUN_00644b20(*gvar_007D9CE4)      [chọn mục tiêu]
   └─ param_3 == 1 (CHUỘT PHẢI) → FUN_00654898(*gvar_007DA51C)      [chốt hành động]
                                        │
                                        ├─ FUN_006538bc(panel, skillId)  → unit+0x5A8 = skillId
                                        ├─ FUN_00654950(panel, itemId)   → unit+0x5AA = itemId
                                        ├─ FUN_0079b620(gvar_007DA0B4, mode) [mode 6/7/8 = UI nút]
                                        └─ FUN_0058e0bc(gvar_007DA530)  [quét 10 ô skill-bar bật/tắt]
                    ▼
 các method của TFightManage (family 0x0064xxxx) gọi:
   FUN_0077f414(gvar_007D9D30, DL=0x0B, CL=<sub>)      ← ***KHÔNG BAO GIỜ 0x32***
     ├─ FUN_00642720 : CL=2, [TFight+0x1E]=actorId, [TFight+0x22]=idx, [TFight+0x24]=3
     ├─ FUN_00642c2c : CL=2, [TFight+0x24]=2
     ├─ FUN_00643310 : CL=4, [TFight+0x1E]=actor+0x548, [TFight+0x24]=4
     ├─ FUN_00644078 : CL=3
     ├─ FUN_00643470 : CL=5  → **builder 0x0B sub 5 là no-op ⇒ gói rỗng, không gửi**
     └─ FUN_0064EA20 / FUN_0064ED14 : CL=8, [TFight+0x24]=9
                    ▼
 S→C phản hồi (xem §1)
```

**Hai kết luận quan trọng của timeline (mức A):**

1. **`FUN_0077f414` có 3 tham số register: `EAX` = self (`gvar_007D9D30`), `DL` = MainOp,
   `CL` = SubOp.** Bằng chứng: prologue `0077f414_FUN_0077F414.asm.txt:24`
   `MOV byte ptr [EBP + -0x6],CL` ngay sau `MOV byte ptr [EBP + -0x5],DL` (`:25`) và
   `MOV dword ptr [EBP + -0x4],EAX` (`:26`); call-site tiêu biểu
   `00642720_FUN_00642720.asm.txt:194-196`: `MOV CL,0x2 / MOV DL,0xb / CALL 0x0077f414`.
   ⇒ **ĐÍNH CHÍNH `opcode_32.md` §6.2**: bộ decompile C **bỏ mất tham số thứ 3 (ECX)**;
   `[EBP-0x6]` **không phải rác** mà **chính là SubOp do call-site truyền vào**.

2. **MainOp `0x32` KHÔNG BAO GIỜ được client gửi.** Quét toàn bộ 125 call-site
   `CALL 0x0077f414` trong `ts_decompile/functions/*.asm.txt` (tìm `MOV DL,<imm>` gần
   nhất phía trên): tập opcode thực sự dùng =
   `{0x01,0x03,0x05,0x06,0x08,0x09,0x0A,0x0B,0x0C,0x0D,0x0E,0x0F,0x10,0x14,0x16,0x17,0x19,
     0x1B,0x1C,0x20,0x23,0x25,0x27,0x28,0x29,0x2B,0x2C,0x2D,0x39,0x3A,0x3B,0x3C,0x3D,0x40,
     0x41,0x42,0x46,0x47,0x48}` — **không có `0x32`**. Đối chiếu kép bằng C:
   `grep -o "FUN_0077f414([^;]*" *.c | grep -oE ",0x[0-9a-f]+\)"` → 39 giá trị, cũng
   không có `0x32`.
   ⇒ Builder 0x32 C→S (`0x00788BCA..0x007890B7`) là **code chết** trong build này.

---

## 1. BẢNG TỔNG HỢP FRAME SKILL

### 1.1. Chiều C→S

#### (a) MainOp `0x1C` — bản đồ sub → payload (mức A, byte-for-byte)

Nguồn: `client_pseudo_c/cs_op1c_byte_map_0x785dbb.hex`, `cs_op1c_targets_0x785dbb.hex`,
`cs_op1c_dispatch_cases.asm.txt`.

Dispatcher (asm:4-8):
```asm
00785da6: XOR EAX,EAX
00785da8: MOV AL,byte ptr [EBP + -0x6]     ; SubOp  (tham số CL)
00785dab: CMP EAX,0x6
00785dae: JA  0x0078a4f2                    ; >6 → im lặng, KHÔNG gửi
00785db4: JMP dword ptr [EAX*0x4 + 0x785dbb]
```
Bảng đích 7 phần tử tại `0x785DBB` (hex: `F2A47800 D75D7800 455E7800 F2A47800 B35E7800 215F7800 8F5F7800`):

| Sub | Đích | Payload (plain, trước khi XOR 0xAD) | Mức |
|:--:|:--|:--|:--|
| `0` | `0x78A4F2` (epilogue) | **không gửi** | A |
| `1` | `0x785DD7` | `[1C][01]` ++ AnsiString `[[*gvar_007D9EB8]+0x1A8]` | A |
| `2` | `0x785E45` | `[1C][02]` ++ AnsiString `[[*gvar_007D9EB8]+0x1A8]` — **cùng nguồn với sub 1** ✅ Round-4 | A |
| `3` | `0x78A4F2` | **không gửi** | A |
| `4` | `0x785EB3` | `[1C][04]` ++ AnsiString `[[*gvar_007D9E68]+0x28C]` | A |
| `5` | `0x785F21` | `[1C][05]` ++ AnsiString `[[*gvar_007D9E68]+0x290]` | A |
| `6` | `0x785F8F` | `[1C][06]` ++ AnsiString `[[*gvar_007DA114]+0x0E8]` | A |

Khuôn builder (ví dụ sub 1, `cs_op1c_dispatch_cases.asm.txt:22-51`):
```
[01][MainOp]  → FUN_00402B90 (copy shortstring)  → AnsiString "0x1C"
[01][SubOp]   → @PStrNCat(buf, sub, 2)          → "0x1C 0x01"
@LStrFromString (0x0040402C) : shortstring → AnsiString, **bỏ byte độ dài**  (asm:41)
@LStrCat      (0x00404090) : dest := dest ++ src                            (asm:46)
TForm1.CY_AddSedQueue (0x0051633C)                                            (asm:50)
```
Định danh form (mức A, `ts_decompile/functions/0051189c_FUN_0051189c.c`):
* `gvar_007DA114` = **`TShipSkillTreeForm`** (VMT `0x58A764`), dòng `:1669-1670`.
* `gvar_007D9EB8` = `FUN_005980a0(...)` (`:1536-1537`) — form skill-tree thứ 2.
* `gvar_007D9E68` = `FUN_00577CCC(...)` (`:1533-1534`).

> ❗ **Kết luận quan trọng (A):** MainOp `0x1C` **không mang field skill dạng số**. Nó
> gửi **một AnsiString nguyên blob** lấy thẳng từ field của form UI. Đây là kênh
> "học/nâng cấp skill" theo kiểu **gửi cả danh sách/serialization**, không phải record
> `{u16 skillId, u8 lv}` mà server Rust đang giả định.
> Nội dung blob (VISCII text hay binary 3-byte record?) — **chưa kết luận được** (cần traffic).

#### (b) MainOp `0x0B` — bản đồ sub → payload (🔴 **mức A cho 11/11 sub** — Round-4 đã đóng 5 GAP)

> **Round-4:** 5 sub `0..4` trước đây là **GAP** nay đã **byte-verified**. Xem §0.1.2.
> Tập sub hợp lệ = `{0,1,2,3,4,6,7,8,9,0x0A,0xFA}` — **chỉ sub `5` là no-op**. Xem §0.1.6.

Nguồn: `0077f414_FUN_0077f414.asm.txt:737-768` (dispatch) + `asm_cs0b_78048d_builders.asm.txt` (builder `0..4`).
Dispatcher:
```asm
00780420: XOR EAX,EAX
00780422: MOV AL,byte ptr [EBP + -0x6]    ; SubOp
00780425: CMP EAX,0x6 ; JG 0x780454
0078042a: JZ  0x7807d1                   ; sub 6  -> BUILDER (KHONG phai no-op)
00780430: CMP EAX,0x4 ; JA 0x78a4f2      ; *** sub 5 -> NO-OP ***
00780439: JMP dword ptr [EAX*0x4 + 0x780440]  ; sub 0..4  ✅ ĐÃ CÓ BYTE (Round-4)
00780454: CMP EAX,0x9 ; JZ 0x7809b8 ; SUB EAX,7 → 0x780826 (7) ; DEC → 0x7808e2 (8)
00780474: SUB EAX,0xa → 0x780a0d (0x0A) ; SUB EAX,0xf0 → 0x780abd (0xFA) ; else no-op
```

| Sub | Payload plain | Nguồn asm | Mức |
|:--:|:--|:--|:--|
| `0` | `[0B][00]` ++ `[b:1]` = `*(gvar_007DA5AC)+0x130` — **2 B** | `0x78048D` | **A** ✅ |
| `1` | `[0B][01]` ++ `[b:1]` = `*(gvar_007DA5AC)+0x130` — **2 B** | `0x780511` | **A** ✅ |
| **`2`** | `[0B][02]` ++ `[b:1]`(`+0x24`) ++ `u32LE`(`+0x1E`) ++ **`u16LE`**(`+0x22`) — **9 B** — **đây là kênh lệnh trận** | `0x780595` | **A** ✅ |
| **`3`** | `[0B][03]` ++ `u32LE`(`+0x1E`) ++ **`u16LE`**(`+0x22`) — **7 B** | `0x780671` | **A** ✅ |
| `4` | `[0B][04]` ++ `u32LE`(`+0x1E`) ++ **`u16LE`**(`+0x22`) — **7 B** | `0x780721` | **A** ✅ |
| `5` | **KHÔNG GỬI** (`JA 0x78a4f2`, asm:743) | asm:742-743 | A |
| `6` | `[0B][06]` (2 byte, không body) | asm:758-780 | A |
| `7` | `[0B][07]` ++ `[PlayerRec+0x384]` ++ `[PlayerRec+0xE3]` | asm:781-829 (đọc ở `:802`) | A |
| `8` | `[0B][08]` ++ `[TFightManage+0x24]` ++ `DWORD LE(TFightManage+0x1E)` ++ `low8(TFightManage+0x22)` | asm:830-883 (đọc ở `:849,865,873`) | A |
| `9` | `[0B][09]` (2 byte) | asm:884-906 | A |
| `0x0A` | `[0B][0A]` ++ `[TFightManage+0x24]` ++ `DWORD LE(TFightManage+0x1E)` | asm:907-954 (đọc ở `:926,943`) | A |
| `0xFA` | `[0B][FA]` (2 byte) | asm:955-974 | A |

Ghi chú quan trọng về sub 8/0x0A: `+0x22` (Word) được đưa vào gói bằng
`CALL 0x00403FA0` = **`@LStrFromChar`** → chỉ phát **1 byte** (low byte), không phải 2 byte
LE. (`opcode_3c.md`/`opcode_37.md` đã xác nhận `0x00403FA0 = @LStrFromChar`.)

#### (c) MainOp `0x32` — builder C→S (mức A về layout, nhưng **KHÔNG BAO GIỜ ĐƯỢC GỌI**)

Nguồn: `0077f414_FUN_0077F414.asm.txt:3256` (preamble `0x788BCA`), `:3279-3283` (dispatch sub),
`:3285-3419` (sub 1), `:3420-3553` (sub 2).

```asm
00788c24: MOV AL,byte ptr [EBP + -0x6]
00788c27: DEC AL ; JZ 0x00788c38      ; sub 1
00788c2b: DEC AL ; JZ 0x00788e7a      ; sub 2
00788c33: JMP 0x0078a4f2              ; sub khác → KHÔNG gửi
```

**Sub 1 = dùng skill / tấn công — 10 byte:**
```
P[0] = 0x32
P[1] = 0x01
P[2] = unit[panel+0xE78].0x56E      ← nhóm (group) của ô ĐANG ĐỨNG
P[3] = unit[panel+0xE78].0x56F      ← cột (slot) của ô ĐANG ĐỨNG
P[4] = unit[panel+0xEAA].0x56E      ← nhóm mục tiêu
P[5] = unit[panel+0xEAA].0x56F      ← cột mục tiêu
P[6..7] = Word LE  unit[panel+0xE78].0x5A8   ← *** SKILL ID ***
P[8]  = Random(byte[EBP-0x19])      (asm:3399-3400)
P[9]  = Random(0x100)                (asm:3405-3406)
```
**Sub 2 = dùng vật phẩm — 10 byte:** y hệt, nhưng `P[6..7] = Word LE unit[...].0x5AA`
(`asm:3528`) ⇒ **item id**.

Định danh field (mức A):
* `panel = *gvar_007DA51C` — panel trận, có mảng `+0x158 + idx*4` (21 unit) — **cùng layout
  với `battle+0x158`** (xem `opcode_35.md` §5).
* `panel+0xE78` (byte) = chỉ số đơn vị **đang được điều khiển** (đọc trong
  `FUN_006538bc.c:...`, `FUN_00658c30.c`, `FUN_00650a24.c:113` set = 0 mỗi vòng).
* `panel+0xEAA` (byte) = chỉ số đơn vị **mục tiêu được chọn**.
* `unit+0x56E/+0x56F` (byte) = grp/slot — **cùng field mà OP 0x35 SubOp 7 ghi**
  (`0064e3e0_FUN_0064e3e0.c:313,318`).
* `unit+0x5A8` (Word) = **skill id đã chọn** — set bởi `FUN_006538bc(panel, skillId)`
  (`006538bc.c:112`), khởi tạo `0x2710` (10000 = đánh thường) tại
  `0063cda0_TFightHum.Create.asm.txt:48`.
* `unit+0x5AA` (Word) = **item id** — set bởi `FUN_00654950(panel, itemId)`
  (`00654950_FUN_00654950.c:136`).

Hai byte random là **PRNG LCG tất định** (`FUN_00402CF0`,
`00402cf0_FUN_00402cf0.c:760-765`: `seed = seed*0x8088405 + 1`) ⇒ **padding/bloat có chủ
đích**, không mang ngữ nghĩa. `byte[EBP-0x19]` được seed từ `Random(0xDC)` hoặc
`Random(0x38)+200` (asm:3256-3277) → cũng chỉ là nhiễu.

### 1.2. Chiều S→C

#### (a) MainOp `0x32` — kết quả hành động (mức A)

Handler: `case_043_007951DA_FUN_007951da.c:20-32` — chỉ đọc `RP[0]` rồi rẽ
`1 → FUN_00647164`, `2 → FUN_0064DD74`, không có `default`.

**SubOp 1 — `FUN_00647164` (`ts_decompile/functions/00647164_FUN_00647164.c`)**:
đầu hàm reset `battle+0xE5B = 0xFF`, `battle+0xE68 = 0xFF`, `battle+6 = 0`, flush 21 khe FX
(`FUN_00651824`), `self+0x1C = 1`. ⚠️ **không guard `battle != 0`**.

Record (RP 0-based, `RP[0]=SubOp`; `pos` 1-based bắt đầu = 2; vòng lặp `pos < Len(RP)`;
stride = `W + 2`):

| Off | Kích thước | Ý nghĩa | Mức |
|:--|:--|:--|:--|
| `RP[0]` | 1 | SubOp = `0x01` | A |
| `RP[1..2]` | 2 | `W` = Word LE, độ dài phần thân record (đếm từ `RP[3]`) | A |
| `RP[3]` | 1 | `srcGrp` (BoundErr nếu > 3) | A |
| `RP[4]` | 1 | `srcSlot` (BoundErr nếu > 4) | A |
| `RP[5..6]` | 2 | `code` = Word LE — **skill/item id** | A |
| `RP[7]` | 1 | `b2` (→ `FUN_00648154(..., b2, code)`) | A |
| `RP[8]` | 1 | `b3` = **số sub-block** trong `text` | A |
| `RP[9]` | 1 | `dstGrp` | A |
| `RP[10]` | 1 | `dstSlot` | A |
| `RP[9 .. 9+W-7]` | `W-6` | `text` — **CHỒNG LẤN lên `dstGrp`/`dstSlot`** (hành vi nguyên văn) | A |

Remap đặc biệt (A, `00647164.c:190-206`): nếu `code == 10000` thì remap theo `unit+4`:
`0x9090/0x9091 → 0x3E81 (16001)`, `0xA4B0/0xA47B/0xA47C/0xA4BA/0xA4C8 → 0x3E82 (16002)`.

**Sub-block `text` (mức A — parse ở `00651e3c_FUN_00651e3c.c:130-215`)**:
`b3` lần lặp, mỗi lần:
```
text[p+0] = dstGrp       text[p+1] = dstSlot
text[p+2] = ?            text[p+3] = ?  (byte mode 0/1/2 → unit.VMT+0x18(unit, mode))
text[p+4] = n            (số hit item)
n × 4 byte: [u8 troi/hit-kind][u16 LE damage][u8 buff/flag]
p += n*4 + 5
```
Client dựng record nội bộ **58 byte** (`0x3A`) cho mỗi sub-block và đẩy vào
`unit+0x5A4` (một `TList`):
`[u32 code][u8 grp][u8 slot][u8 b?][u8 mode][u8 n][items…][u8 steps]` (`00651e3c.c:290`
`TList_Add(*(int**)(local_8+0x5A4), ...)`).

Consumer: `FUN_00653020` (`00653020_FUN_00653020.c:57-70`) duyệt list ngược, tra
`FUN_0065178c(battle, rec[4], rec[5])` để tìm unit đích, rồi `FUN_00652454` áp từng hit.
`FUN_00652188` là biến thể "áp hit cụ thể theo (grp,slot) rồi xoá record".

**SubOp 2 — `FUN_0064DD74`**: flush FX + `self+0x1C = 1`, parse record với stride `n*8+5`;
các field record **đọc nhưng không tiêu thụ** (`0064dd74_FUN_0064dd74.c:130-137` dead store).

#### (b) MainOp `0x35` — biến cố trận (mức A, bổ sung)

Đã có bảng đầy đủ ở `opcode_35.md` §3. Bổ sung liên quan skill:

| SubOp | Layout | Ngữ nghĩa (mức) |
|:--:|:--|:--|
| `0x01` | `[35][01] × n{ [grp][slot][type 1..5][code Word LE] }`, `n=(Len(RP)-1)/5` | **ĐÂT KỸ NĂNG**: set animation theo **5 kênh** → A |
| `0x03` | `[35][03][grp][slot]` | ép unit sang **state 7** (di chuyển/tấn công) → A |
| `0x04` | `[35][04][id Word][grp][slot] × m{(b1,b2)}` | `FUN_00654590(unit, b1, b2, id)` → (B) |
| `0x0E` | `[35][0E][grp][slot][w Word]` | `unit.VMT+0x1C(unit, w, 1)` → (B) |
| `0x0F` | `[35][0F][grp][slot][mode 'B'..'G'][byte][w Word]` | 6 cặp `(byte,Word)` vào `unit+0x5BE..0x5CE` → A |

**5 kênh animation của SubOp 1** (mức A, từ `0064cd74_FUN_0064cd74.c:112-127` + 5 method):

| `type` | Method | Mảng FX | Hàm set | Byte tương ứng (đọc ở `00652454.c:428-451`) | Số code |
|:--:|:--|:--|:--|:--|--:|
| 1 | `FUN_0064BE2C` | `battle+0x200` | `FUN_0066D450` | `0xDD` | 29 |
| 2 | `FUN_0064C108` | `battle+0x254` | `FUN_0066D804` | `0xDE` | 27 |
| 3 | `FUN_0064C49C` | `battle+0x2A8` | `FUN_0066DE54` | `0xDF` | 27 |
| 4 | `FUN_0064C7A8` | `battle+0x2FC` | `FUN_0066E37C` | `0xE0` | 4 |
| 5 | `FUN_0064C880` | `battle+0x350` | `FUN_0066BAC4C` | `0xE1` | 18 |

Đây chính là nguồn của `troi_byte::{TYPE3=0xDD, TYPE4=0xDE, TYPE15=0xDF, TYPE19=0xE1}`
trong `src/battle/packets.rs:266-276` — **Rust đã khớp** (xem §4).

#### (c) MainOp `0x0B` SubOp 7 — `FUN_00733650` (mức A) — **KHÔNG phải cooldown**

`00733650_FUN_00733650.c:48-92`:
1. `self+0x1305 := B1` (byte) — **chế độ auto** (1 / 2 / 3 / 5 xem `FUN_00658C30.c:79-175`).
2. `self+0x1306 := min(0xFF, self[0x1306] + param_3)` — **bộ đếm bão hòa** (`:49-67`).
3. Nếu `+0x1305 == 1` hoặc `== 2` → toast `prefix + IntToStr(counter) + suffix`, 1500 ms.
4. `FUN_00658C30(*gvar_007DA51C)` — refresh panel hành động.

`FUN_00658C30` (`00658c30_FUN_00658c30.c:41-175`) là **bộ chọn hành động tự động**:
* case 1 → `FUN_006531B8(panel, 1|2, 1)` tự chọn mục tiêu, `unit+0x5A8 = 10000`.
* case 2/4 → như trên **+** nếu người chơi đủ SP thì gán `unit+0x5A8 = skillId`
  (`FUN_0075A9F8` kiểm `skill_type ∈ {1,2}`; `FUN_0075A9A0` lấy `sp_cost`;
  `FUN_00658B0C(skillId, sp_cost, …)` kiểm `sp_cost ≤ PlayerRec+0x3EC` **và** đã học skill).
* case 5 → `unit+0x5A8 = 0x4269`.

⇒ `0x0B sub 7` = **bật/tắt chế độ đánh tự động + số lượt còn lại**, **không** phải cooldown
skill. (Mức A.)

---

## 2. PHÂN TÍCH CHI TIẾT

### 2.1. Mô hình lựa chọn skill phía client

| Hàm | Việc | Mức |
|:--|:--|:--|
| `FUN_00759B2C` | **Nạp `Data\Skill.Dat`** (`_LStrCat3(..., "\Data\Skill.Dat")`), `Reset(F, 0x56)` ⇒ **record 86 byte**, đọc vào `obj+4`, dựng bảng tra `skillId → index` tại `DAT_009AE128[skillId]` cho 3000 slot | A (`00759b2c_FUN_00759b2c.c:95-160`) |
| `FUN_00759DF0` | Giải mã/kiểm mã record: **đảo ngược chuỗi** `name[0..21]` + `des[0x37..0x56]`, `XOR 0xFD` cho byte `record+0x15` (`= (v^0xFD)-4`), `(v^0x6E90)-4` cho word, `(v^0x0BAEB716)-2` cho dword | A |
| `FUN_0075A948` | `GetSkillRecord(skillId, out)` — copy **nguyên 86 byte** vào buffer | A |
| `FUN_0075AA4C` | `GetSkillName` — `record+0x00` (Pascal shortstring ≤ 21) | A |
| `FUN_0075AAAC` | `record+0x15` — `type` | A |
| `FUN_0075A9A0` | `record+0x18` — `sp_cost` | A |
| `FUN_0075AB54` | `record+0x1A` — `elem` | A |
| `FUN_0075A9F8` | `record+0x21` — `skill_type` | A |
| `FUN_0075AB00` | `record+0x2A` — số cấp yêu cầu của skill tiền điều kiện (so với `actor+0x572 + i*3`) | A |

**Cross-check với `src/data/loaders/skill_pc.rs`** (record 86 byte, offset đều khớp):
`+0x15 type`, `+0x16 id`, `+0x18 sp_cost`, `+0x1A elem`, `+0x21 skill_type`, `+0x00 name`,
`+0x37 des` — **Rust parser khớp 100% với accessor của client**. (Mức A.)

Danh sách skill đã học của nhân vật: `PlayerRec + 0xFD4 + idx*3`, mỗi entry
**`{u16 skillId, u8 skillLv}`** (stride **3**) — `006538bc.c:135-148`,
`005ab678_FUN_005ab678.c:224-232`.

### 2.2. Các mã skill đặc biệt client hard-code

| Mã | Ý nghĩa suy ra | Bằng chứng |
|:--|:--|:--|
| `10000 (0x2710)` | đánh thường | init `0x5A8=0x2710` (`0063cda0.asm:48`) |
| `14001 (0x36B1)` | skill đặc biệt có cảnh báo giới hạn | `006538bc.c:55-63` |
| `14998 (0x3A9A)` | UI mode 8 | `006538bc.c:127`, `00650a24.c:114` |
| `16001/16002 (0x3E81/2)` | remap của đánh thường theo loại actor | `00647164.c:190-206` |
| `17001 (0x4269)` | UI mode 7, không cần học | `006538bc.c:130`, `00658c30.c:169` |
| `18001 (0x4651)` | như 17001 | `006538bc.c:130` |
| `20006 (0x4E26)` | **PSEUDO-SKILL COMBO** | `00648154.c:1652` (FX `0x95`), `0064c7a8.c:22-31` |
| `20007 (0x4E27)` | **chân combo** | `00648154.c:1660` (FX `0x96`), `0064c7a8.c:32-41` |
| `20008 (0x4E28)` | FX `0x0E` | `0064c7a8.c:42-50` |
| `25129 (0x61FD)` | dùng vật phẩm đặc biệt (UI mode 8) | `00654950.c:143` |

---

## 3. QUY TẮC COMBO

### 3.1. Kết luận

> ## **COMBO là quyết định 100% PHÍA SERVER.**
> Client **không có bất kỳ bảng quy tắc combo nào**, không có state "đang combo",
> không có cửa sổ thời gian nối skill, không có kiểm tra "skill này có thể nối tiếp
> skill kia". Client chỉ biết **3 mã giả** `20006/20007/20008` và ánh xạ chúng sang
> animation.

### 3.2. Bằng chứng (mức A — 5 lớp, độc lập nhau)

**(1) Frame lệnh C→S không có chỗ nào để biểu đạt chuỗi skill.**
Cả builder 0x32 sub 1/sub 2 (10 byte) và các builder 0x0B đều mang **đúng 1 mã hành
động**. Không có field `comboStep`, `nextSkill`, `chainId`, `sequence`. Lượt chơi là
**server điều phối** (server báo "đến lượt ô nào"), nên về mặt kiến trúc client không
thể phát biểu "mình muốn nối tiếp".

**(2) `Skill.Dat` không có field "skill kế tiếp / combo".**
Bản đặc tả record 86 byte (`skill_pc.rs:10-43`, cross-check bằng 7 accessor client §2.1)
không có trường nào trỏ sang skill khác ngoài `require_sk` (+0x2B, điều kiện học) và
`unk17/19/21/22` — mà client **không accessor nào** đọc tới +0x2B/+0x2D/+0x30/+0x33/+0x35.
`FUN_0075AB00` đọc `+0x2A` và so với danh sách skill của actor → đó là **yêu cầu số cấp của
skill tiền điều kiện**, không phải liên kết combo.

**(3) Mã `20006` (combo) được MIỄN KIỂM "đã học skill".**
`005ab678_FUN_005ab678.c:218`:
```c
if ((local_10 != 0x4e26) && ((int)local_24 < 0x100)) {
    ... kiểm local_10 có nằm trong danh sách skill đã học không ...
}
```
Tức `20006` **được miễn** bước validate "người chơi đã học skill này chưa" — dấu hiệu
rõ ràng nó là **skill ảo do server sinh ra**, không phải skill người chơi bấm.
Cùng bằng chứng: `FUN_00648154` **không** có entry nào trong `Skill.Dat` (id max
`23019` theo `CONTEXT.md:206`), 20006 nằm ngoài bảng.

**(4) Client chỉ map `code → animation`, không kiểm tra gì.**
* `FUN_00648154` (12 018 byte, `00648154_FUN_00648154.c`) là giant switch
  **241 entry** `code → FX id`, thuần `FUN_00666BF0(battle+0x1AC+idx*4, fx, tgtIdx, 0)`.
* 5 method `FUN_0064BE2C/C108/C49C/C7A8/C880` là 5 switch nhỏ (29/27/27/4/18 code) map
  `code → FX id` theo 5 kênh.
* Không có nhánh nào đọc "skill trước" / `unit+0x5A8` cũ / thời gian.

**(5) Client không giữ state combo.**
Grep toàn bộ `ts_decompile/functions/*.c` cho `0x4e26` chỉ ra 5 nơi, **không nơi nào** ghi
state hay so sánh với giá trị trước đó. Trường `unit+0x5A8` chỉ được **đọc để truyền đi**
đúng một lần — tại `0077f414_FUN_0077F414.asm.txt:3393` (`MOV DX,word ptr [EAX + 0x5a8]`),
tức **bên trong builder 0x32 đã chết**. Trong code sống, `0x5A8` chỉ dùng cho quyết định
UI (`FUN_00650A24`, `FUN_00651094`, `FUN_006538BC`, `FUN_00658C30`).

### 3.3. Quy tắc combo thật sự (mức C — chỉ có ở server Rust)

`src/battle/runner.rs:1098-1167` (`detect_combo`):

```rust
// điều kiện tiên quyết: skill_type == 1 (vật lý) của người ra  (runner.rs:810-812)
let (nr, nc) = sorted[idx + 1];                 // đơn vị KẾ TIẾP trong thứ tự lượt
if next.id > 0 && next.team == attacker.team {
  if next_type == 1                            // skill vật lý
     && next.row_attack == attacker.row_attack // CÙNG mục tiêu
     && next.col_attack == attacker.col_attack
     && target_alive {
        if ts.combo_hi == 0 {                  // mở chuỗi
            if |next.agi - attacker.agi| <= 800 {   // ← NGƯỠNG AGI
                combo_lo/hi = min/max(agi)
                combo_active = get_random_miss_combo()   // luôn = 1 (100%)
                delay = next_delay
                num37 *= 1.3                            // ← ×1.3 damage mỗi mắt xích
                add_combo_cell(attacker); add_combo_cell(next)
            }
        } else {                                // nối tiếp
            if |next.agi - (combo_lo+combo_hi)/2| <= 800 { … mở rộng lo/hi … }
        }
     }
}
ts.combo_active = 0;                            // không khớp → tắt combo
```

Kèm theo:
* Hệ số damage `num37 *= 1.3` cộng dồn khi `ts.combo_active == 1` (`runner.rs:800-802`).
* Mục tiêu dùng `targeting::get_pos_attack_combo` thay vì `..._default` (`runner.rs:1197-1199`).
* Packet phát ra:
  * `packets::combo(row,col)` → `[32][01][0F 00][row][col][26 4E][01][01][row][col][01 03 01 E0 00 00]`
    ⇒ `W=15`, `code=20006`, `b2=1`, `b3=1`, `text = row col 01 03 | 01 | E0 00 00`
    ⇒ **1 sub-block, n=1, 1 hit `{troi=0xE0, dmg=0, buff=0}`**. (Mức C)
  * `packets::combo_footer_20007(row,col)` ⇒ `code=20007`, hit `{troi=0x19(HP), 0, 0}`.
  * ⚠️ frame lắp trong `runner.rs:1057-1063`: `F444 + le16(len) + 3201 + text9` —
    **thiếu byte MainOp?** Không: `text_combo` đứng trước `F444` ⇒ frame thực tế là
    `combo-frame || F444 || len || 3201 || text9` — cần traffic để xác nhận thứ tự.

### 3.4. Đối chiếu 3 mã combo với client

| Mã | Rust | Client đọc | Kết quả |
|:--|:--|:--|:--|
| `20006` | `packets::combo()` | `FUN_00648154.c:1652` → FX **`0x95`**; `FUN_0064C7A8.c:22-31` → FX `0x95` | ✅ **khớp** |
| `20007` | `combo_footer_20007()` | `00648154.c:1660` → FX **`0x96`**; `0064C7A8.c:32-41` → FX `0x0B` | ⚠️ **2 nơi cho 2 FX khác nhau** |
| `20008` | *(không dùng)* | `0064C7A8.c:42-50` → FX `0x0E` | — |

`20007` xuất hiện 2 lần trong client với 2 FX khác nhau (`0x96` ở `FUN_00648154`,
`0x0B` ở `FUN_0064C7A8`) — nhiều khả năng `FUN_0064C7A8` là **fallback** (default) chứ không
phải nhánh chính; đánh dấu **(B)**.

---

## 4. ĐỐI CHIẾU SERVER RUST + MISMATCH

### 4.1. `src/server/handlers/battle.rs`

| Handler | Payload Rust đọc | Client thực tế | Mức mismatch |
|:--|:--|:--|:--|
| `handle_battle_command` sub 1 (`battle.rs:181-215`) | `payload[0]=row, [1]=col, [2]=row_attack, [3]=col_attack, [4..5]=skill_id LE16` | **ĐÚNG THỨ TỰ** so với builder 0x32 chết (chỉ thiếu 2 byte junk cuối) — **nhưng client build này không bao giờ gửi 0x32** | ⚠️ **H-1** |
| `handle_battle_command` sub 2 (`battle.rs:222-247`) | `[0..1] row/col`, `[2..3] tgt`, `[4..5] item_id LE16`, gate `26001..=27165` | tương tự sub 1 nhưng đọc `unit+0x5AA`; gate của Rust là **suy đoán** | ⚠️ **H-2** |
| `handle_battle` sub 8 (`battle.rs:35-46`) | `u32_le(payload[1..5])` = target id | **KHỚP CHÍNH XÁC** `[0B][08][kind][DWORD id][low byte]` | ✅ |
| `handle_battle` sub 2 (`battle.rs:90-110`) | `payload[0]=inner (2/3/4/5)`, `[1..5]=u32 target`, `[5..7]=word ground` | ✅ **Round-4 đã verify byte-level**: builder `0x780595` = `ShortString{0x0B,inner}` + `u32LE(+0x1E)` + **`u16LE(+0x22)`**. Xem §0.1.2/§0.1.4 | ✅ **H-3 ĐÃ ĐÓNG** |
| `handle_battle` sub 5 | "flee battle" | **client builder `0x0B` sub `5` là NO-OP** (`asm:742-743`); `FUN_00643470` gọi nó nhưng không gửi được gì | ⚠️ **H-4** |
| `handle_battle` sub 1 | "leave battle, `data[6]==3`" | `FUN_0064ED14`/`FUN_0064EA20` gửi sub 8 không phải sub 1 | 🟡 |
| `handle_battle` sub 6 | broadcast `0B06 + id` | client **có** builder `0x0B` sub 6, nhưng payload chỉ `[0B][06]` = **2 byte, không body** ⇒ không mang id ⇒ Rust tự chế id | 🟡 **H-4b** |

### 4.2. `src/server/handlers/skills.rs` (MainOp 0x1C)

Rust `handle_player_skill_learn` (`skills.rs:116-125`) đọc **record 3 byte lặp**:
```rust
while idx + 2 < payload.len() {
    let skill_id = encoder::u16_le(payload[idx], payload[idx + 1]);
    let target_lv = payload[idx + 2];
    idx += 3;
```
Client gửi `[1C][01] ++ <AnsiString field form>`. **Nếu** AnsiString đó đúng là blob 3-byte
record thì layout khớp; nếu là text VISCII thì không. → **UNVERIFIED** (mức B/C).
Rust **không** xử lý sub 4/5/6 (client có gửi sub 5 và 6).

> ✅ **Round-4 ĐÃ CHỐT (2026-10-02).** Dispatch head **đã có sẵn** trong
> `cs_op1c_dispatch_cases.asm.txt` (window `0x785DA6..0x78620B`) — **không cần redump**:
>
> ```asm
> 00785da6: XOR EAX,EAX
> 00785da8: MOV AL, byte ptr [EBP + -0x6]
> 00785dab: CMP EAX, 0x6
> 00785dae: JA  0x0078a4f2                     ; sub > 6 -> KHONG gui
> 00785db4: JMP dword ptr [EAX*0x4 + 0x785dbb]
> ```
>
> ⇒ bảng **7 entry** (`0x00..0x06`), sub `0` và `3` = epilogue.
> **5 sub thật sự gửi: `01`, `02`, `04`, `05`, `06`** — dòng "client có gửi sub 5 và 6"
> ở trên **đã được chứng minh**. §1.1(a) bảng 7 phần tử ngay phía trên **đã đúng từ đầu**.
>
> ⚠️ **Chi tiết đã sửa:** sub `2` đọc `[*gvar_007D9EB8]+0x1A8` (**cùng field với sub 1**),
> không phải `[*gvar_007D9E68]+0x28C`. Layout đầy đủ 5 sub: `light_dark §0.1.1b`.
>
> ⚠️ **Độ dài thay đổi**: cả 5 sub đều là `[1C][sub] ++ @LStrCat(field)`, **không phải
> record byte cố định**. Vì vậy mục `1C 01` ở §4.2 vẫn đúng là **B/C** — giờ đã biết chắc
> nó là *chuỗi*, chỉ cần traffic thật để chốt nội dung.

### 4.3. Frame S→C `0x32` sub 1 — Rust **KHỚP**

`runner.rs:1040-1050`:
```rust
let block = format!("{:02X}{:02X}{}{:02X}{:02X}{}", row, col, le16(skill), num34 as u8, count, ts.text10);
ts.text9.push_str(&le16(block.len() as u16 / 2));
ts.text9.push_str(&block);
```
→ `[32][01][W LE16][row][col][skill LE16][num34][count][ count × {row,col,miss,adl,1 | troi,dmg LE16,buff} ]`

Đối chiếu §1.2(a): `W` ✓, `srcGrp/srcSlot` ✓, `code` ✓, `b2=num34` ✓ (SlDanh),
**`b3=count`** ✓ (số sub-block), `text` = **`count` sub-block 9 byte** ✓ — mỗi sub-block
đúng cấu trúc `5 + 4n` mà `FUN_00651E3C` parse. **KHÔNG có mismatch.** ✅

`packets::combo()` / `combo_footer_20007()` cũng đúng cấu trúc (`W=15`, `count=1`, 1 hit). ✅

### 4.4. Mismatch còn lại (xếp theo mức nghiêm trọng)

| # | Vấn đề | Mức |
|:--|:--|:--|
| **H-1** | Server Rust chờ `0x32 sub 1/2` để nhận lệnh dùng skill, nhưng **client build này không gửi `0x32`** — lệnh trận đi qua `0x0B sub 2/8`. Frame đó nay đã xác minh byte-level (§0.1.2) nhưng **Rust vẫn chưa viết lại handler** | **A** |
| **H-2** | Gate item `26001..=27165` là thuần contract Rust (C); client chỉ đọc `unit+0x5AA` | C |
| **H-3** | ~~Byte bảng dispatch `0x780440` + builder `0x78048D..0x7807D0` không có trong corpus~~ **→ ĐÃ ĐÓNG (Round-4)**: dump 836/836 B. Frame lệnh trận sub `2` = `[0B][02][b:1][u32 LE][u16 LE]` đã byte-verified. Rust đọc **đúng** offsets | ✅ **A** |
| **H-4** | Server xử lý `0x0B sub 5` (flee) nhưng client **không gửi** (`JA 0x78a4f2`) ⇒ `battle.rs:26` là dead code. **Lưu ý: sub `6` thì CÓ được gửi** (2 byte, không body) — dễ nhầm | A |
| **H-5** | `Skill.combo` (`tables.rs:91`) đọc từ `Skills.txt` — file **không có** trong `Data/` ⇒ `combo_field` luôn = 0; và biến này bị `let _ = (...)` bỏ qua (`runner.rs:1301`, `:1478`) | A |
| **H-6** | `data.skills` (Skills.txt) rỗng ⇒ `skill_row = None` ⇒ `skill_type` chỉ lấy được từ `BinarySkillDef.kind` (`runner.rs:772-777`). Nhưng **`BinarySkillDef.kind` là `record+0x15` = `type`**, còn `skill_type` client-side là **`record+0x21`** (`FUN_0075A9F8`) — **hai field khác nhau**! `skill_pc.rs` map `skill_type → kind` | **A** |
| **H-7** | `num34` server gửi ở `b2` = `fight_area` (`runner.rs:783-786`, fallback `sl_danh`) — client dùng `b2` làm tham số thứ 4 của `FUN_00648154(..., b2, code)`; ngữ nghĩa chưa rõ | B |
| **H-8** | Client có `troi_start` = `[35][01][grp][slot][01][skillId LE16]` (SubOp 1 record type 1..5 + Word LE) — Rust `packets::troi_start` gửi `[35][01][row][col][01][skillId LE16]` ✅ khớp | ✅ |

---

## 5. KHOẢNG TRỐNG / VIỆC CẦN LÀM

1. **✅ ĐÃ XONG (Round-4 2026-10-02)** — Redump `0x780440..0x780453` + `0x78048D..0x7807D0`.
   Artifact: `cs0b_780440_jmptbl_5xDWORD.hex` + `asm_cs0b_78048d_builders.asm.txt` (836/836 B).
   Kết quả ở §0.1. **Xoá khỏi danh sách việc cần làm.**
1b. **✅ ĐÃ XONG (2026-10-02)** — ~~dump `0x00785DA0..0x00785DBB` (dispatch head)~~.
   Dispatch head **đã có sẵn** trong `cs_op1c_dispatch_cases.asm.txt:3-8`. Kết luận:
   `0x1C` có **7 sub** (`0..6`), `0`/`3` = epilogue, **gửi thật: `01/02/04/05/06`**.
   Xem `light_dark_skill_unlock_gap.md §0.1.1-0.1.1c`.
1c. **🟠 Mới (Round-4)**: `0x94B440` (cây kỹ năng 765 B) và `0x9AE128` (bảng `skillId→index`)
   **không đọc được** trong Ghidra — ngoài vùng memory đã map (`> 0x7DA800`).
   **Đã có cách bypass không cần Ghidra:**
   `python3 test/pe_dump.py <aLogin.exe> 0x94B440 0x765 -o skilltree_94b440_765B.hex`.
   Xem `light_dark_skill_unlock_gap.md §0.1.3` và `ROUND4_UPDATE_INDEX.md`.
2. **🔴 Traffic thật**: bắt 1 lượt đánh có dùng skill để đối chiếu
   (a) `0x0B sub 2` layout thật — **Round-4 đã verify byte-level, chỉ còn xác nhận
   ngữ nghĩa `inner ∈ {2,3,4,5}`**, (b) nội dung AnsiString của `0x1C sub 1`.
3. **🟡 Giải mã `Skill.Dat`**: `FUN_00759B2C` + `FUN_00759DF0` đã đủ để viết decoder;
   `skill_pc.rs` đã có nhưng cần đối chiếu từng field với accessor client
   (`+0x15 / +0x16 / +0x18 / +0x1A / +0x21 / +0x2A`) — đã khớp 6/6.
4. **🟡 `FUN_00651E3C` / `FUN_00652454`**: struct 58 byte có **nhiều mảng overlap** do Ghidra
   không phân biệt được (`local_59[4]` + `auStack_55[7]` chồng nhau). Cần đọc ASM để chốt
   offset chính xác của từng item hit. Hiện đã chốt: `[0]=code u32`, `[4]=grp`, `[5]=slot`,
   `[8]=n`, `[57]=steps`; các offset còn lại đánh dấu (B).
5. **✅ ĐÃ ĐÓNG (Round-4)** — ~~`FUN_00654590` vẫn **không có body** trong corpus~~
   **SAI**: `00654590_FUN_00654590.c` + `.asm.txt` đã tồn tại từ trước. Xem §0.1.7.
6. **🟡 Định danh `gvar_007D9EB8` / `gvar_007D9E68`**: VMT nằm sau 2 con trỏ gián tiếp
   (`PTR_PTR_00589708`, `PTR_PTR_00577938`) chưa có tên trong `index.csv`.
7. **🟡 Điểm cấp phát `gvar_007DA51C`** — chỉ thấy 1 lần gán = 0
   (`00603f20_FUN_00603f20.c:154`); class chưa định danh (mức (B): panel trận, layout
   `+0x158` giống `battle+0x158`).
8. **Nghĩa vụ combo**: không có gì để reverse thêm — combo là thuần logic server.

---

## 6. SOURCE TRAIL

| # | Nguồn | Dùng để |
|:--:|:--|:--|
| 1 | `client_pseudo_c/0077f414_FUN_0077F414.asm.txt:16-49` | prologue 3 tham số (EAX/DL/CL) + dispatch main-op |
| 2 | `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex`, `cs_builder_dword62_0x77f53c.hex` | bảng MainOp → builder address |
| 3 | `client_pseudo_c/cs_op1c_dispatch_cases.asm.txt:4-301` | toàn bộ 7 nhánh builder OP 0x1C |
| 4 | `client_pseudo_c/cs_op1c_byte_map_0x785dbb.hex`, `cs_op1c_targets_0x785dbb.hex` | bảng đích sub của 0x1C |
| 5 | `client_pseudo_c/0077f414_FUN_0077F414.asm.txt:737-974` | dispatch + builder OP 0x0B (sub 6/7/8/9/A/FA) |
| 6 | `client_pseudo_c/0077f414_FUN_0077F414.asm.txt:3256-3553` | builder OP 0x32 C→S (sub 1/2) — **code chết** |
| 7 | `ts_decompile/functions/00642720|00642c2c|00643310|00643470|00644078|0064ea20|0064ed14` (`.asm.txt`/`.c`) | call-site `MOV CL,<sub>; MOV DL,0xb; CALL 0x0077f414` + buffer `TFightManage+0x1E/0x22/0x24` |
| 8 | `ts_decompile/functions/0050bff8_TForm1.DXDraw1MouseDown.c:80-100` | timeline chuột trái/phải trong trận |
| 9 | `ts_decompile/functions/00647164_FUN_00647164.c:84-262` | **layout record OP 0x32 S→C sub 1** |
| 10 | `ts_decompile/functions/00651e3c_FUN_00651e3c.c:130-290` | parse `text` → record 58 byte → `unit+0x5A4` |
| 11 | `ts_decompile/functions/00653020 / 00652188 / 00652454` | tiêu thụ record 58 byte |
| 12 | `ts_decompile/functions/00648154_FUN_00648154.c` (2155 dòng) | bảng `code → FX` (241 entry) |
| 13 | `ts_decompile/functions/0064be2c/c108/c49c/c7a8/c880` | 5 kênh animation OP 0x35 SubOp 1 |
| 14 | `ts_decompile/functions/00759b2c / 00759df0 / 00759b04 / 0075a948 / 0075a9a0 / 0075a9f8 / 0075aa4c / 0075aaac / 0075ab00 / 0075ab54` | loader + accessor `Skill.Dat` |
| 15 | `ts_decompile/functions/006538bc / 00654950 / 00650a24 / 00651094 / 00658c30 / 00654898` | chọn skill / item / mục tiêu, `unit+0x5A8` / `+0x5AA` |
| 16 | `ts_decompile/functions/00658b0c / 005ab678` | gate "đủ SP" + "đã học skill", miễn trừ `0x4E26` |
| 17 | `ts_decompile/functions/00733650_FUN_00733650.c:48-92` | OP 0x0B sub 7 = chế độ auto + bộ đếm |
| 18 | `src/battle/runner.rs:746-816, 960-1090, 1098-1200, 1500-1535` | combo detect / ghép frame |
| 19 | `src/battle/packets.rs:85-260` | `combo`, `combo_footer_20007`, `skilling_int/full` |
| 20 | `src/server/handlers/battle.rs:14-250` | handler 0x0B / 0x32 |
| 21 | `src/server/handlers/skills.rs:85-140` | handler 0x1C |
| 22 | `src/data/loaders/skill_pc.rs:1-90` | layout record `Skill.Dat` 86 byte |
| 23 | `.scratch/client-pseudo-op-code/opcode_{32,35,0b}.md` | nền tảng layout S→C |