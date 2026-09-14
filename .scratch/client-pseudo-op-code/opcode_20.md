# PHÂN TÍCH — Main OP 0x20 (32) / Case 28 / `FUN_0079285A` @ `0x0079285A`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Hai callee `func_0x006546E0` / `func_0x0072F534` **đã có body trong bản dump mới** (`index.csv:6410/6434`) — phân tích đã tích hợp ở §4.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Handler nhỏ nhất trong cụm (chỉ 2 nhánh, 0 literal chuỗi, 0 decode ở tầng case). Tầng case chỉ đọc 1 byte SubOp rồi **passthrough nguyên RestPayload** cho hàm con. **Cả hai callee nay đã sáng tỏ và cùng họ ghi trạng thái slot**: đường B ghi qua mảng 21 con-trỏ-slot tại `mgr+0x158` (mgr = `*gvar_007DA51C`); SubOp 02 là **batch** ghi byte cờ vào `slot+0xE3` + mã chuẩn hóa vào `slot+900`.
- SubOp `0x01`: rẽ 3 đường theo cờ 1 byte `player+0x376` (`**gvar_007DA7BC`): `==0` → `FUN_0072C64C(mgr_007D9D34, RP)` (đã phục hồi: set 1 byte `v` cho slot tìm theo `id`); `!=0 && !=4` → `func_0x006546E0(mgr_007DA51C, RP)` (**xác minh được từ body mới**: cùng wire `[id:4B][v:1B]`, tìm slot qua helper `0x00654CE0`, ghi bằng **đúng setter `FUN_0072A7A8`** như đường A, nhưng trên mảng slot riêng của manager `007DA51C`); `==4` → không làm gì.
- SubOp `0x02`: `func_0x0072F534(mgr_007D9D34, RP)` (**xác minh được từ body mới**: lặp các bản ghi 5 byte `[id:4B LE decode][flag:1B]`, tra slot 800 phần tử như đường A, ghi `slot+0xE3=flag`, `slot+900=mã chuẩn hóa qua FUN_007205B8`, rồi gọi hook đổi trạng thái `FUN_0072B390`).
- Không banner, không sound, không hiệu ứng ở bất kỳ tầng nào đã phục hồi. Thuần state.
- Chiều C→S `case 0x20: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x20 (32) → byte_table[0x78A8EE][0x20] = 0x1C (28)
                 → dword_table[0x78A9B6][28] @ 0x0078AA26 = 0x0079285A
                 → FUN_0079285a (Case 28)
```

- File chính: `ts_decompile/case_functions/functions/case_028_0079285A_FUN_0079285a.c` (69 dòng)
- Bản inline đối chiếu: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5179-5203` (`case 0x20:`) — khớp 1:1.
- Dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt:27-31` (bản `.asm.txt` chỉ còn prologue; logic đầy đủ ở bản `.c`).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x20`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).
- Delphi `_LStrCopy(s, Index, Count)` 1-based: `_LStrCopy(RP,2,4)` → `RP[1..4]` = `P[2..5]`.

### 2.3. Đọc SubOp (dòng 20-37)

```c
if (*(RP-4) == 0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);   // RP[0] = P[1], 1 byte thường, không codec
if (SubOp == 1) {...} else if (SubOp == 2) {...}
// Không switch/default → SubOp 0x00 hoặc >=0x03 = no-op (chỉ epilogue)
```

- Khối `_LStrArrayClr/_LStrClr` cuối hàm là epilogue chung dispatcher, không phải nghiệp vụ.
- Tầng case **không gọi helper codec nào** trong 5 helper — callee tự decode.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer tầng case | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[20][01][id:4B LE][v:1B]` (≥7B) | `SubOp = RP[0]`, passthrough nguyên `RP` | Gate theo `player+0x376`: 0 → đường A; 4 → no-op; còn lại → đường B (cùng wire, khác bảng slot) |
| `0x02` | `[20][02]([id:4B LE][flag:1B])×N` (≥7B, 5B/bản ghi) | như trên | `func_0x0072F534(mgr, RP)` — batch ghi cờ slot `+0xE3/+900` + hook |
| `0x00`, `≥0x03` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation — handler này vốn không có)

### 4.1. SubOp `0x01` — Update slot có điều kiện theo `player+0x376`

- **Wire**: `P[0]=0x20, P[1]=0x01, P[2..]=data` (format do callee quyết).
- **Đọc**: 1 byte `RP[0]==1`, passthrough.
- **Xử lý**:
  ```c
  if (*(char*)(player + 0x376) == 0)
    FUN_0072C64C(*gvar_007D9D34, RP);        // đường A
  else if (*(char*)(player + 0x376) != 4)
    func_0x006546E0(*gvar_007DA51C, RP);     // đường B (đã phục hồi — xem 4.1.2)
  // ==4 → không làm gì
  ```

#### 4.1.1. Đường A — `FUN_0072C64C` (đã phục hồi, `0072c64c_FUN_0072c64c.c:50-66`)

Nhận `(mgr, RP)` với RP vẫn gồm cả byte SubOp:

1. `_LStrCopy(RP,2,4)` → `FUN_0077ef7c` decode **DWORD LE** `id = P[2..5]` (bỏ byte SubOp nhờ Index=2).
2. `idx = FUN_0070C20C(mgr_007D9D34, id)`: duyệt tuyến tính `gvar_007DA300[1..count]` (`count = *(mgr+0x60)`, trần 800), khớp khi `*(slot+0x4)==id`; không thấy → trả 0.
3. `if (idx==0) return` — id lạ bỏ qua, không crash.
4. `v = RP[5]` (Delphi index 6, guard `len(RP)>=6` nếu không `_BoundErr(5)`) — 1 byte tại `P[6]`.
5. `FUN_0072A7A8(*(gvar_007DA300+idx*4), v)` ghi slot:
   ```c
   *(slot+0x3d8) = v; *(slot+0x3d9) = 0; *(slot+0x3da) = 1;
   *(double*)(slot+0x3df) = Now();
   *(slot+0x3db) = FUN_007C9B38(..., PREFIX_0x72A870 + IntToStr(v));
   *(slot+999) = 0;
   ```
   Thuần state (byte + cờ + timestamp + handle). Không banner/sound/light.

- **Wire thực tế đường A**: tối thiểu 7 byte payload `[20][01][id:4B LE][v:1B]` (`len(RP)>=6`). Thừa byte bị bỏ qua.

#### 4.1.2. Đường B — `func_0x006546E0(*gvar_007DA51C, RP)` — **đã phục hồi (đính chính: không phải unknown)**

Body mới `ts_decompile/functions/006546e0_FUN_006546e0.c` (187B, `index.csv:6410`, chữ ký `__register (param_1=EAX=mgr, param_2=EDX=RP)`):

1. `_LStrCopy(RP,2,4)` → `id = FUN_0077ef7c(*gvar_007D9D30, ...)` — **giống hệt khuôn decode DWORD LE của đường A** (`006546e0_FUN_006546e0.c:43-44`).
2. `idx = FUN_00654CE0(mgr, id)` — helper tra bảng riêng của manager `007DA51C` (**chưa có body**, không trong `index.csv`); trả `char`, `-1` = không thấy; `idx == -1` → bỏ qua im lặng (`006546e0_FUN_006546e0.c:45-47`).
3. `v = RP[5]` (0-based; guard `len(RP)>=6` → `_BoundErr(5)`) — cùng vị trí byte giá trị như đường A (`006546e0_FUN_006546e0.c:50-54`).
4. Guard `idx > 0x14` → `_BoundErr` — bảng slot của manager này có **21 khe (0..20)** (`006546e0_FUN_006546e0.c:56-58`).
5. `FUN_0072A7A8(*(mgr + 0x158 + idx*4), v)` — **chính xác setter ghi slot của đường A** (`+0x3D8=v`, `+0x3D9=0`, `+0x3DA=1`, timestamp `+0x3DF`, handle `+0x3DB`), nhưng con trỏ slot lấy từ mảng `mgr+0x158` thay vì `gvar_007DA300` (`006546e0_FUN_006546e0.c:59`).

→ **Kết luận**: đường B không phải kênh khác — cùng wire `[id:4B][v:1B]`, cùng semantics ghi trạng thái, khác **bảng đối tượng** (manager `007DA51C` với 21 slot tại `+0x158`, chưa định danh được class). Wire thực tế: tối thiểu 7 byte payload `[20][01][id u32LE][v u8]`, `len(RP)>=6`.

### 4.2. SubOp `0x02` — `func_0x0072F534(*gvar_007D9D34, RP)` — **đã phục hồi**

Body mới `ts_decompile/functions/0072f534_FUN_0072f534.c` (356B, `index.csv:6434`, `(param_1=EAX=mgr, param_2=EDX=RP)`). **Vòng lặp batch** trên chính manager `007D9D34` của đường A:

1. Con chạy `i = 2` (Delphi 1-based, tức sau byte SubOp); lặp `while i < len(RP)` (`0072f534_FUN_0072f534.c:45-48,91`).
2. Mỗi bản ghi 5 byte: `id = FUN_0077ef7c(*gvar_007D9D30, _LStrCopy(RP,i,4))` (`:50-51`); `flag = byte tại RP[i+4]` (`:57-62`); `i += 5` (`:63-64`).
3. `idx = FUN_0070C20C(mgr, id)` — **cùng hàm tra slot tuyến tính** (trần 800) như đường A; `idx == 0` → bỏ qua bản ghi (`:68-70`).
4. `code = FUN_007205B8(slot, flag)` — hàm chuẩn hóa: switch `flag 0..0x3B → mã nhóm` (0/8/0x10/0x12/0x1A/0x1C/0x1E/0x20/0x22/0x24/0x26/0x28/0x2A/0x2C/0x2E/0x32/0x3A/0x3B, default 0) — `007205b8_FUN_007205b8.c:33-135`.
5. Ghi `slot+0xE3 = flag` (byte thô) và `slot+900 (0x384) = code` (`0072f534_FUN_0072f534.c:79,84`).
6. `FUN_0072B390(slot, code)` — hook đổi trạng thái (`:89`): nếu thỏa điều kiện cờ `slot+0x79/+0xE6` + `player+0x63A`, thì free object `player+0x618`, set dirty `slot+0x466`, gọi **VMT+0x18 của slot với mã mới**; nếu là slot local (`+0x79==1`) còn gọi `FUN_0071FDF8(player,0)` + `FUN_0077F414(*gvar_007D9D30, 0x20)` — chính là builder C→S OP 0x20, mà `case 0x20: break;` rỗng (§6) → **không thực phát gói** (`0072b390_FUN_0072b390.c:64-86`).

→ **Kết luận**: SubOp 0x02 = **cập nhật hàng loạt byte trạng thái theo id** (wire `[20][02]([id u32LE][flag u8])×N`), cùng họ "ghi state slot" với SubOp 0x01; vẫn thuần state, không banner/sound. Ý nghĩa nghiệp vụ của `flag`/`code` (nhóm 0..0x3B) **chưa kết luận được** — chỉ có cơ chế chuẩn hóa.

---

## 5. Chuỗi VISCII → UTF-8

- **Tầng Case 28: không có literal nào** — chỉ so sánh 2 byte, không có gì để decode.
- **Hai body mới (`006546E0`, `0072F534`) cũng không lộ literal chuỗi mới** — mọi field trên dây là số (`id` DWORD + byte).
- **Tầng `FUN_0072A7A8`** (cả đường A lẫn đường B đều dùng): có 1 literal prefix `@LAB_0072a870` (`prefix + IntToStr(v)`). Bản redump mới **vẫn chưa phủ** `0x0072A870` → chưa decode được. Cần redump `.rodata` tại đó.
- Không có payload text trên dây ở nhánh đã phục hồi (mọi field là số: DWORD id + 1 byte v).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:960-961`: `case 0x20: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x20 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [20][01][id u32LE][v u8]        ; đường A (player+0x376==0) hoặc B (!=0,!=4) — cùng format
S→C [20][02]([id u32LE][flag u8])×N ; batch ghi cờ slot +0xE3/+900 + hook FUN_0072B390
C→S [20]: KHÔNG TỒN TẠI (case rỗng; lời gọi FUN_0077F414(ctx,0x20) trong FUN_0072B390 rơi vào break)
```

1. **Đính chính ghi chú cũ**: SubOp 02 giờ đã đặc tả được wire (`[id u32LE][flag u8]` lặp lại, 5B/bản ghi, bắt đầu tại `RP[1]`); chỉ còn **nghiệp vụ của mã flag 0..0x3B là chưa kết luận**.
2. `len(RP)>=6` bắt buộc cho cả hai đường của SubOp 01; ngắn hơn → RangeError. Đường B additionally: `idx>0x14` (không khớp 21 slot) hoặc id không tìm thấy (`FUN_00654CE0` trả -1) → im lặng / RangeError tùy nhánh.
3. Khi test đường A đảm bảo `player+0x376==0` (`4` → no-op, còn lại → đường B — cùng payload, khác bảng nhận).
4. Quan sát: `slot+0x3D8` (byte v), `+0x3D9/0x3DA` (cờ 0/1), `+0x3DF` (timestamp); SubOp 02: `slot+0xE3` (flag thô), `slot+0x384/900` (mã chuẩn hóa).
5. SubOp 02: bản ghi cuối phải đủ 5 byte — `len(RP)-1` không chia hết cho 5 → byte thừa khiến `_BoundErr` bên trong codec; id ngoài 2 bảng (0) bị bỏ qua. Không gửi SubOp `0x00/≥0x03`, không gửi frame `L=1` (`_BoundErr(0)`).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_028_0079285A_FUN_0079285a.c` | Handler chính, gate `+0x376`, 2 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5179-5203` | Bản inline đối chiếu 1:1 |
| 3 | `functions/0072c64c_FUN_0072c64c.c:50-66` | Decode đường A (`_LStrCopy(RP,2,4)` + `EF7C` + `RP[5]`) |
| 4 | `functions/0070c20c_FUN_0070c20c.c:121-150` | Tìm slot tuyến tính, trần 800 |
| 5 | `functions/0072a7a8_FUN_0072a7a8.c:52-66` | Ghi slot `+0x3D8/0x3D9/0x3DA/0x3DB/0x3DF/999` |
| 6 | `functions/0077f414_FUN_0077F414.c:960-961` | C→S rỗng |
| 7 | `functions/006546e0_FUN_006546e0.c:43-59` + `index.csv:6410` | **Mới**: body đường B (`:43-44` decode id, `:45` tra `0x654CE0`, `:50-54` v, `:56-58` guard 0x14, `:59` setter) |
| 8 | `functions/0072f534_FUN_0072f534.c:45-91` + `index.csv:6434` | **Mới**: batch SubOp 02 (`:50-51` id, `:62` flag, `:68-70` tra slot, `:74` chuẩn hóa, `:79/:84` ghi `+0xE3/+900`, `:89` hook) |
| 9 | `functions/007205b8_FUN_007205b8.c:33-135`, `functions/0072b390_FUN_0072b390.c:64-86` | **Mới**: bảng chuẩn hóa flag + hook đổi trạng thái (kể cả lời gọi `FUN_0077F414(ctx,0x20)` rơi vào `case 0x20: break;`) |

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] `FUN_00654CE0` (tra bảng slot của manager `007DA51C`) vẫn chưa có body — chỉ suy được trả `char`, `-1` = không thấy.
- [ ] `LAB_0072a870` (prefix handle trong `FUN_0072A7A8`) vẫn chưa có dump `.rodata`.
- [ ] Định danh class manager `007DA51C` + ngữ nghĩa 21 slot `+0x158` và bộ mã flag `0..0x3B` (switch `007205B8`) — chưa kết luận được.
