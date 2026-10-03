# Kết quả Round-4 redump — đã giải 44 144 byte

Ngày: 2026-10-02 · Manifest: `test/Round4Manifest.txt` · Output: **`test/round4/`** (không phải `ts_decompile/`)
Kết quả: `ok=471 fail=1 skip=0 indexRowsAppended=33`

---

## 1. Tổng kết

| Mục | Kết quả |
|---|---|
| Directive chạy | 154 (`ASM` 37 · `HEX` 51 · `FUNC` 40 · `GVAR` 11 · `PTRTAB` 7 · `SCAN` 4 · `VMT` 4) |
| Byte instruction thu hồi | **44 144 B** (mục tiêu: 26 550 B trong 15 khoang + 17 594 B bổ sung) |
| Khối phục hồi | **13 / 15** khoang (2 khoang còn lại là bảng thuần data → đã dump bằng `HEX`) |
| Lỗi decompile | **0** |
| Fail | 1 — `0x0050F4CE` (dự kiến, xem §5) |

### Bảng 15 khoang — kết quả từng khoang

| # | Khoang | Size | Kết quả |
|:-:|---|--:|---|
| 1 | `0x783405..0x785225` — C→S `0x17` (61 sub) | 7 713 | ✅ 7 710 B |
| 2 | `0x781A06..0x782E8A` — C→S `0x09..0x16` | 5 253 | ✅ 5 253 B |
| 3 | `0x787362..0x78820E` — C→S `0x27` bang hội | 3 757 | ✅ 3 756 B |
| 4 | `0x78125D..0x7819F0` — C→S `0x13`/`0x14` | 1 940 | ✅ 1 940 B |
| 5 | `0x7853E5..0x785B4B` — C→S `0x1F`/`0x20` | 1 895 | ✅ 1 892 B |
| 6 | `0x780B87..0x780FA4` — C→S `0x0C`/`0x0D` | 1 054 | ✅ 1 054 B |
| 7 | `0x786598..0x786986` — C→S `0x1E`/`0x1F` | 1 007 | ⚠️ 975 B (thiếu 32 B — có 1 bảng nhỏ lẫn trong đó) |
| 8 | `0x786221..0x786582` — bảng 9 nhánh `0x1E` | 866 | ✅ 866 B |
| 9 | `0x78048D..0x7807D0` — C→S lệnh trận `0x0B` | 836 | ✅ 836 B |
| 10 | `0x77F883..0x77FB47` — C→S `0x01`/`0x02` | 709 | ✅ 709 B |
| 11 | `0x785DBB..0x785FFC` — bảng 5 nhánh `0x1C` | 578 | ✅ 578 B |
| 12 | `0x78A157..0x78A317` — C→S `0x48`/`0xC7` | 449 | ✅ 449 B |
| 13 | `0x77F474..0x77F633` — byte-map + case `0x00` | 448 | 📄 data → `cs_bytemap_77f474_200B.hex` |
| 14 | `0x77F771..0x77F789` | 25 | ✅ 25 B |
| 15 | `0x780440..0x780453` — bảng 5 entry `0x0B` | 20 | 📄 data → `cs0b_780440_jmptbl_5xDWORD.hex` |

---

## 2. 🔴 Kết quả lớn nhất — bảng C→S `0x27` (bang hội) đã giải

`cs27_787362_jmptbl_57xDWORD.hex` — **đúng 57 entry (`0x00..0x38`) như `guild_city_siege_flow.md:2.1` dự đoán.**
Guard `CMP EAX,0x38 / JA 0x78A4F2` xác nhận biên trên.

**Chỉ 28 / 57 sub thật sự gửi được.** 29 trỏ tới `0x0078A4F2` = epilogue `XOR EAX,EAX; RET` ⇒ **không gửi gì**.

| Nhóm sub | Builder | Số |
|---|---|--:|
| `0x00` | epilogue | 1 |
| **`0x01`–`0x13`** | `0x787446 … 0x787E4A` | **19** |
| `0x14`–`0x1E` | epilogue | 11 |
| **`0x1F`, `0x20`** | `0x787E69`, `0x787ED7` | **2** |
| `0x21`–`0x31` | epilogue | 17 |
| **`0x32`–`0x38`** | `0x787F2C … 0x7881D6` | **7** |

Địa chỉ builder cụ thể:

```
0x01 0x787446   0x02 0x7874BA   0x03 0x78753A   0x04 0x7875DF
0x05 0x7876A1   0x06 0x787744   0x07 0x787799   0x08 0x787819
0x09 0x78786E   0x0A 0x7878E2   0x0B 0x787962   0x0C 0x7879E2
0x0D 0x787A62   0x0E 0x787B07   0x0F 0x787BAC   0x10 0x787C1A
0x11 0x787CF1   0x12 0x787D72   0x13 0x787DFB
0x1F 0x787E69   0x20 0x787ED7
0x32 0x787F2C   0x33 0x787F81   0x34 0x787FD6   0x35 0x788044
0x36 0x7880B2   0x37 0x788107   0x38 0x78818B
```

**Kết luận cho Rust:** `0x27` C→S **chỉ có 28 sub hợp lệ**. Mọi sub trong khoảng
`0x14..0x1E` và `0x21..0x31` mà server tài liệu hoá là **client không bao giờ gửi** ⇒
handler server cho chúng là dead code.

---

## 3. 🔴 `0x0B` lệnh trận — layout byte đã verify, khớp Rust

`cs0b_780440_jmptbl_5xDWORD.hex` + `asm_cs0b_78048d_builders.asm.txt`:

```
[0] 0x78048D  [1] 0x780511  [2] 0x780595  [3] 0x780671  [4] 0x780721
```

Đọc tay builder cho wire payload (giống hệt mẫu `ShortString{[EBP-5]=[main], [EBP-6]=[sub]}`
→ `_PStrNCat` → `LStrFromString` → `LStrCatN` → `CY_AddSedQueue`):

| sub | Payload sau `0B<sub>` | Tổng | Nguồn field |
|:--:|---|--:|---|
| 0 | `[b:1]` = `*(gvar_007DA5AC)+0x130` | 2 B | 1 byte |
| 1 | `[b:1]` = `*(gvar_007DA5AC)+0x130` | 2 B | 1 byte |
| **2** | **`[inner:1]` + `u32 LE` + `u16 LE`** | **9 B** | inner=`gvar_007D9CE4+0x24`, u32=`+0x1E`, u16=`+0x22` |
| 3 | `u32 LE` + `u16 LE` | 7 B | `+0x1E`, `+0x22` |
| 4 | `u32 LE` + `u16 LE` | 7 B | `+0x1E`, `+0x22` |
| **7** | `[b:1]` = `*(gvar_007DA7BC)+0x384` + … | ≥3 B | `player+0x384` |
| **8** | sub-8 riêng (`0x7808E2`) | — | đọc `*(0x7D9F10)+0x15D` |
| **9** | sub-9 riêng (`0x7809B8`) | — | đọc `*(0x7DA7BC)+0x4D8/+0x4D9/+0x4DD` |

Dispatch head (`asm 0077f414` + mới):

```
00780439: JMP [EAX*4 + 0x780440]   ; sub 0..4
00780454: CMP EAX,0x9 / JG 0x780474
00780459: JZ 0x007809B8            ; sub 9
0078045F: SUB EAX,7 / JZ 0x00780826 ; sub 7
00780468: DEC EAX  / JZ 0x007808E2 ; sub 8
0078046F: JMP 0x0078A4F2           ; sub 5, 6  -> KHONG GUI
00780474: SUB EAX,0xA / JZ 0x00780A0D   ; sub 0x0A
0078047D: SUB EAX,0xF0 / JZ 0x00780ABD  ; sub 0xF5
00780488: JMP 0x0078A4F2           ; con lai -> KHONG GUI
```

**⇒ sub `0x0B` hợp lệ: `0,1,2,3,4,7,8,9,0x0A,0xF5`. Sub `5` và `6` KHÔNG BAO GIỜ được gửi.**

### ⚠️ Rust đang xử lý sub không tồn tại

`src/server/handlers/battle.rs:26` có `5 => handle_flee_battle(...)` và
`:31` có `6 => ...`. Theo ASM, **client không gửi sub 5 hay 6 của `0x0B`** ⇒ 2 nhánh này
là dead code (không gây hại, nhưng đáng để biết). Sub 0x0A **chưa** có handler Rust.

### ✅ Xác nhận Rust ĐÚNG (không có off-by-one)

`dispatcher.rs:228` đặt `payload = decoded[6..]` — **đã bỏ qua `[main][sub]`**.
Nên với wire `0B 02 <inner> <u32> <u16>`:
- `payload[0]` = `inner` ✅ khớp `battle.rs:97`
- `payload[1..5]` = u32 target ✅ khớp `battle.rs:110`
- `payload[5..7]` = u16 ground ✅ khớp `battle.rs:117`

`battle/03:448` (mức "GAP, chưa có byte") **nay đã lên mức A (byte-level verified)**.

---

## 4. 🔴 `0x1C` học kỹ năng — phát hiện SAI trong tài liệu

`cs1c_785dbb_jmptbl_5xDWORD.hex` (bảng đúng 5 entry tại `0x785DBB`):

```
[0] 0x0078A4F2  epilogue  -> sub 0 KHONG GUI
[1] 0x00785DD7  builder   -> sub 1
[2] 0x00785E45  builder   -> sub 2
[3] 0x0078A4F2  epilogue  -> sub 3 KHONG GUI   <<<<
[4] 0x00785EB3  builder   -> sub 4
```

> ⚠️ **ĐÍNH CHÍNH 2026-10-02 — mục này SAI, đã sửa.** Manifest chỉ xin
> `PTRTAB 0x00785DBB 5` ⇒ file hex bị **cắt cụt ở 5 dword**, dễ đọc nhầm thành
> "bảng 5 entry". Bảng thật là **7 entry**. Dispatch head **đã có sẵn** trong
> `cs_op1c_dispatch_cases.asm.txt`:
>
> ```asm
> 00785dab: CMP EAX,0x6
> 00785dae: JA  0x0078a4f2
> 00785db4: JMP dword ptr [EAX*0x4 + 0x785dbb]
> ```
>
> ⇒ 7 entry, `[0]`/`[3]` = `0x0078A4F2` (epilogue). **`light_dark §2.1` ĐÚNG**:
> gửi thật là **`01`, `02`, `04`, `05`, `06`**. Layout: `[1C][sub] ++ @LStrCat(field)`.
> Chi tiết: `light_dark_skill_unlock_gap.md §0.1.1-0.1.1c`.
>
> Hệ quả thực tế: `1C 05` mà `light_dark §3 G-19` phân tích ("skill reborn 2",
> `ball_count`) **CÓ TỒN TẠI** — dùng `[*gvar_007D9E68]+0x290`.

*(Bản gốc của mục này, trước khi sửa, ghi: "⇒ danh sách đúng phải là `01, 02, 04`;
`05` và `06` không nằm trong bảng này" — SAI, do đọc file hex bị cắt.)*

---

## 5. 🟠 `0x1E` storage + các bảng khác

`cs1e_786221_jmptbl_9xDWORD.hex`:

```
[0] 0x0078A4F2 epilogue   |  [1] 0x00786245   [2] 0x007862B3   [3] 0x00786321
[4] 0x00786376            |  [5] 0x007863E4   [6] 0x00786452   [7] 0x007864C0
```

⚠️ Chỉ đọc được **8 dword** trong 34 byte (bảng 9 entry cần 36 B — cửa sổ `0x786221..0x786242`
bị cắt 2 byte). Nhưng sub `0` = epilogue ⇒ **8 nhánh thật** khớp `economy/05:493 (U9)`.

Thêm: `0x78A157` là bảng **6 entry** (`CMP EAX,5`) cho C→S `0x48`.

---

## 6. ✅ `0x45` con cái — verify byte-exact khớp tài liệu

Builder `0x789DEF` / `0x789EBE` (đã có trong `functions/0077f414_FUN_0077f414.asm.txt`):

| Builder | Đọc field | Wire | Đối chiếu `marriage §2.4` |
|---|---|---|---|
| `0x789DEF` | `*(gvar_007D9E3C)+0x144` (word) → `FUN_0077EB1C`; `+0x158` (byte) → `LStrFromChar`; `+0x146` (byte) → `LStrFromChar`; `LStrCatN cap 4` | `45 01` + `u16` + `u8` + `u8` = 5 B | ✅ **khớp tuyệt đối** |
| `0x789EBE` | `+0x159` (byte); `+0x146` (byte); cap 4 | `45 02` + `u8` + `u8` = 4 B | ✅ **khớp tuyệt đối** |

Đây là bằng chứng **mạnh nhất** cho framing `[main][sub]` (không có byte `0x01` framing):
`_PStrNCat(cap=2)` trên `ShortString{0x45, 0x01}` cho ra **đúng 2 byte** `45 01`.

---

## 7. 🔴 BLOCKER — 5 bảng P0 nằm ngoài vùng memory đã map

```
; truncated: requested 1792 readable 0     ← 0x009AE128
; truncated: requested  618 readable 0     ← 0x0094B440
; truncated: requested   16 readable 0     ← 0x009D1610
```

**Không phải lỗi manifest** — Round-3 đã gặp đúng tình trạng này
(`table_0x948DF8_200.hex`, `table_0x9490B0_200.hex`, `table_0x98BE04_200.hex`
đều `readable 0`).

Biên memory đo được từ 2 file đọc được một phần:

| File | Bắt đầu | Đọc được | Kết luận |
|---|---|--:|---|
| `lvlcoef_7da6b8_wide.hex` | `0x007DA6B8` | 328 B | tới `0x7DA7F7` |
| `table_7da6dc_101DWORD.hex` | `0x007DA6DC` | 292 B | tới **`0x7DA7FF`** |

⇒ **Memory đã map kết thúc ở `0x7DA800`.** Mọi địa chỉ ≥ `0x7DA800` đọc được 0 byte.

Trong khi đó các địa chỉ này **chắc chắn có thật** trong code — ASM chứng minh:

```asm
00759b15: MOV AX, word ptr [EAX*0x2 + 0x9ae128]   ; FUN_00759B04 tra skillId -> index
0059f087: ADD EDX, 0x94b440                      ; TSkillTreeRow base
```

### Cần làm gì

`0x94B440` (765 B, cây kỹ năng) và `0x9AE128` (bảng `skillId → index`) là **2 mục P0
quan trọng nhất** — không thể lấy được nếu project chưa map memory.

1. Mở CodeBrowser → **Window → Memory Map**, xem block nào phủ `0x94B440`.
2. Nếu vùng đó là **uninitialized** (`.bss`): cần *Commit* hoặc chạy analysis để khởi tạo.
3. Nếu **không có block nào**: project bị import thiếu section ⇒ **phải re-import `aLogin.exe`**
   (giữ nguyên image base `0x400000`).
4. Kiểm tra nhanh: `Round3Redump.java` với manifest 1 dòng
   `HEX probe.hex 0x0094B400 0x0094B4FF` — nếu vẫn `readable 0` thì chắc chắn là vấn đề project.

> **Lưu ý:** `.scratch/…` và `test/ts_decompile/` đều được tạo từ project có vùng map
> tới `0x7DA7FF`, nên mọi tài liệu hiện có **không thể** chứa dữ liệu vùng này.
> Đây là lý do `light_dark §6 #4, #5` vẫn để ngỏ suốt từ đầu.

---

## 8. ⚠️ Việc cần làm tiếp

### 8.1 Gộp output — ✅ ĐÃ LÀM

Round-4 ghi vào `test/round4/`, tách biệt với `ts_decompile/`. Đã merge có dedup:

| Kết quả | Số |
|---|--:|
| File mới copy vào `ts_decompile/` | **129** |
| Giữ bản cũ (24 `functions/` + 2 `redump/`) | 26 |
| Dòng `index.csv` append | **21** |
| Dòng `index.csv` bỏ qua (đã tồn tại) | 11 |

Sau merge: `functions/` = **13 218** file, `redump/` = **417** file.

> ⚠️ `ts_decompile/index.csv` **đã có sẵn 33 dòng trùng `entry_point`** trước khi merge
> (ví dụ `FUN_0077f414` xuất hiện 3 lần). Đây là vấn đề có sẵn từ Round-3, không do
> Round-4 gây ra. 21 dòng mới đã được lọc trùng.
>
> ⚠️ 2 file `redump/` bị giữ bản cũ: `lit_72a870.hex` (Round-3 đã dump, giống nhau) và
> `round3_report.txt` (**giữ cả 2 bản** — `ts_decompile/redump/round3_report.txt` và
> `round4/redump/round3_report.txt`; bản Round-4 có 471 dòng).

### 8.2 Fail duy nhất

`L369 FUNC 0x0050F4CE NO_FUNCTION` — `0x0050F4CE` nằm **+34 B vào giữa**
`TForm1.Transition` (`0x0050F4AC..0x0050F4DF`, 52 B), nên Ghidra không tạo được function
mới — **hành vi đúng, không phải thiếu sót**.

> ⚠️ **ĐÍNH CHÍNH 2026-10-02 — mục tiêu cũng SAI.** Đọc
> `functions/0050f4ac_TForm1.Transition.c`: đây là **máy trạng thái hiệu ứng chuyển
> cảnh màn hình** (`CopyBufToGridBuf`, `freeGridBuf`, `_GetMem(960000)`,
> `rodraw2.ro_Reduce_Blt/Clipper_Blt/Normal_Blt`, lặp 20 × 15 ô) — **không có timer,
> không có socket**. `0x50F4CE` không liên quan tới send/recv pump.
> Bản gốc mục này ghi *"0x0050F4CE là điểm kết thúc của `TForm1.Transition`"* —
> **SAI**, nó là **giữa**. Và không cần disassemble `0x0050F4CE..0x00511134` nữa.
>
> **Hướng đúng:** 2 call-site thật (`0x00515889`, `0x00515DB2`) đều nằm trong cửa sổ
> `SCAN 0x00514C62..0x00516108` đã chạy. Xem `reconnect_heartbeat_flow.md §0.1.5`.

### 8.3 Khoảng trống còn lại

| Việc | Địa chỉ |
|---|---|
| 32 B còn thiếu trong khoang `0x786598..0x786986` | có 1 bảng nhỏ xen khai |
| ~~Dispatch head của `0x1C` (để chốt sub `05`/`06`)~~ | ✅ **ĐÃ CÓ** trong `cs_op1c_dispatch_cases.asm.txt` — xem mục 5 đã đính chính |
| Bảng `0x78A157` (6 entry) — chưa decode | `0x0078A157..0x0078A16F` |
| Hoàn tất `0x7B347D..0x7B37C7` (mẫu append record kỹ năng) | đã có `asm_tfskill_accessors` nhưng chưa có builder `1C 06` |
| Memory ≥ `0x7DA800` | xem §7 |
