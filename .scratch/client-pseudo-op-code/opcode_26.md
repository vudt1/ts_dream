# PHÂN TÍCH — Main OP 0x26 (38) / Case 34 / `FUN_00793889` @ `0x00793889`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). **`func_0x007349f0` đã có body trong bản dump mới** (`index.csv:6439`, 284B) → wire đóng tới hết payload (6B); nghiệp vụ: **cập XP/điểm tổng + tính lại level cho player** (xem §4).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP đơn-nhánh. Handler không `switch` mà chỉ `if (SubOp==1)` rồi **passthrough nguyên RestPayload** (gồm cả byte SubOp) cho `func_0x007349f0(player = *gvar_007DA7BC, RP)`.
- Không codec ở tầng handler; **callee tự decode 4B → DWORD** (họ `ed68`/`gvar_007D9D30`).
- **Xác minh được từ body mới**: `func_0x007349f0` ghi `player+0x1334 (DWORD)` = value nhận trên dây, **tính lại level Word `player+0x1330`** qua `FUN_0065B484` (đếm tăng dần từ level hiện tại theo ngưỡng `FUN_0065B4D4`, trần 0xC9=200 level, mgr bảng cấp = `*gvar_007DA660` — cùng họ hàm `0065Bxxx` dùng cho level/XP ở `005a7970/005b0274`), và dựng banner `DAT_00734B20 + chuỗi-hóa(value)` 2000ms **khi cờ `player+0x1338 == 0`** (cờ chưa định danh — có thể "tắt thông báo", chưa kết luận).
- `func_0x007349f0` nằm trong vùng từng được cho là gap `0x7349ED–0x734B34`; **đính chính**: body đã được xuất trong batch mới (callers: `sub_007938b9`), không còn suy đoán theo hàm lân cận.
- Kẹp giữa Case 33 (OP 0x25, 6 nhánh họ `0x00729xxx`) và Case 35 (OP 0x27, 43 nhánh họ `0x0075xxxx`) nhưng dùng manager khác (player, không phải `007D9C48/007D9C20`) → OP độc lập.
- Chiều C→S **không tồn tại `case 0x26`** (nhảy cóc từ `0x25` sang `0x27`) → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x26 (38) → byte_table[0x78A8EE][0x26] = 0x22 (34)
                 → dword_table[0x78A9B6][34] @ 0x0078AA3E = 0x00793889
                 → FUN_00793889 (Case 34)
```

- File chính: `ts_decompile/case_functions/functions/case_034_00793889_FUN_00793889.c` (61 dòng; logic dòng 20-29, epilogue dòng 30-58)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5756-5770` (`case 0x26:` outer) — khớp 1:1. Lưu ý `case 0x26` xuất hiện 4 lần trong file (3 lần là SubOp của OP 0x00/0x14/0x17, chỉ dòng 5756 là MainOp 0x26).
- Bản copy thứ ba: `case_functions/jumptable_0x78A9B6_cases.c:6746-6804`.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x26`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `SubOp = RP[0] = P[1]`, 1 byte thường.

### 2.3. Đọc SubOp

```c
if (*(RP-4)==0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);
if (SubOp == 1) func_0x007349f0(*gvar_007DA7BC, RP);
// ≠1: no-op (chỉ epilogue _LStrArrayClr/_LStrClr)
```

---

## 3. Bảng tổng hợp SubOp — chỉ 1 SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[26][01][value u32LE]` (đúng 6B) | `SubOp=RP[0]`, passthrough nguyên `RP` | `func_0x007349f0(player, RP)` — **đã có body**: `player+0x1334 = value`, level `+0x1330` = `FUN_0065B484(*gvar_007DA660, value, level cũ)` (`>0xFFFF` BoundErr), banner `DAT_00734B20 + CurrToStr(value)` 2000ms nếu `player+0x1338 == 0` (xem §4) |
| `0x00`,`0x02–0xFF` | `[26][SubOp≠01][...]` | vẫn đọc + guard rỗng | no-op |

Handler không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`).

---

## 4. Chi tiết SubOp `0x01` — cập nhật XP/point + level (core logic, bỏ graphics/sound/animation)

### 4.1. Body `func_0x007349f0` (284B, `index.csv:6439`, chữ ký `__register (EAX=player, EDX=RP)`)

- `_LStrCopy(RP,2,4)` → `value = FUN_0077ed68(*gvar_007D9D30, …)` (DWORD LE — bản clone của `ef7c`, cùng codec đã biết) rồi **ghi `player+0x1334 = value`** (`007349f0_FUN_007349f0.c:51-53`).
- Gate banner: `if (*(byte*)(player+0x1338) == 0)` (`:54`) — cờ chưa định danh (đặt qua code chưa export — **chưa kết luận được** ý nghĩa).
- Trong gate:
  - `level = FUN_0065B484(*gvar_007DA660, value, *(Word*)(player+0x1330))` (`:55-56`) — helper **đếm tăng level từ level cũ**: `0065b484_FUN_0065b484.c:31-44` vòng `do { lvl++; } while (FUN_0065B4D4(mgr, lvl) <= value && lvl < 0xC9)` → **trần 200 cấp**, trả `lvl-1`. Ngưỡng `FUN_0065B4D4(level) = Σ_{i=1..level} FUN_0065B52C(i)` (`0065b4d4_FUN_0065b4d4.c:46-65`) với mỗi bước ≈ `round(RTL-multiply) + 0x14` (`0065b52c_FUN_0065b52c.c:30-32`, input qua FPU/register — **công thức đóng chưa xác định được**).
  - `player+0x1330 = level` (`>0xFFFF` → BoundErr, `:57-60`).
  - `FUN_004099B0` (RTL số→chuỗi, kiểu `CurrToStr`/phân nhóm — Ghidra mất tham số qua register) → `_LStrCat3(msg, &DAT_00734B20, chuỗi)` (`:63-64`) → banner `(VMT+0x90)(*gvar_007DA084, msg, 2000, 0, 0)` (`:65`).
- **Không** sửa field nào khác, không gửi ngược, không sound.

### 4.2. Kết luận & đính chính

- Nhận định cũ "từ `P[2]` unknown / không suy ngược được" → **hết hiệu lực**: payload đóng khung `[26][01][value u32LE]`, total 6B; byte thừa sau `P[5]` bị bỏ qua (callee không đọc).
- `local_10` (giá trị **cũ** tại `+0x1334`, đọc ở `:41-50`) không dùng tới — chỉ còn là artifact guard range của decompiler.
- Hai hàm lân cận `00734858`/`00734b34` từng dùng để định hướng: giữ nguyên vai trò tham khảo, **không** gán field cho `0x7349F0` — mọi field dẫn ở trên đều đọc trực tiếp từ body mới.

---

## 5. Chuỗi VISCII → UTF-8

- **Tầng handler: không có gì để decode** — không hằng chuỗi, không `IntToStr`.
- **Body mới lộ 1 literal**: `DAT_00734B20` — tiền tố banner (nối với chuỗi số từ `FUN_004099B0`) tại `007349f0_FUN_007349f0.c:64`, nằm trong dải `.text` `0x7349ED–0x734B34` (đúng vùng gap cũ — **giờ cần redump** `lit_734b20.hex`, chưa có trong `redump/`). Nhiều khả năng là thông điệp "+XP/level" — **chưa dịch được**.
- Nếu còn text dựng động: chỉ qua `FUN_004099B0` (số→chuỗi, không VISCII).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:960-973`: dãy `case 0x20..0x25` rồi **nhảy thẳng `case 0x27`** — không tồn tại `case 0x26`, hàm rơi qua switch rồi epilogue.
- Kết luận: OP 0x26 S→C thuần. Không chờ, không mock chiều này.

---

## 7. Ghi chú cho Mock Server

```
S→C [26][01][value u32LE] ; SubOp duy nhất — ĐÃ ĐẶC TẢ ĐỦ (6B; byte thừa bị bỏ)
S→C [26][SubOp≠01][...]   ; no-op an toàn (test ignore-path)
ĐỪNG GỬI: [26] đơn byte (L=1) → _BoundErr(0) RangeError
C→S [26]: KHÔNG TỒN TẠI
```

1. RP truyền nguyên gồm cả SubOp; callee đọc đúng `RP[1..4]` — `len(RP)<5` → BoundErr trong codec `_LStrCopy/ed68`. Giữ `P[1]=0x01`.
2. **Có thể kiểm tra bằng mắt**: khi `player+0x1338==0`, banner 2000ms `DAT_00734B20 + chuỗi(value)` hiện lên và Word `player+0x1330` (level) nhảy theo bảng ngưỡng — đây là điểm ngắm instrumentation (`+0x1334` DWORD, `+0x1330` Word, `+0x1338` gate).
3. **Đính chính mục cũ**: không còn cần "decompile 0x7349F0" — đã xong. Muốn đóng nốt: redump `lit_734b20.hex` (nội dung banner) + xác nhận công thức `FUN_0065B52C` (đọc asm nếu cần) + 1 gói `[26][01]` live để chốt value là XP hay điểm tích lũy.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_034_00793889_FUN_00793889.c` | Handler chính (`SubOp=RP[0]`, `if==1`) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5756-5770` (+`:579`, `:568-572`) | Bản inline khớp 1:1 (RP=`local_10`, MainOp=`local_9`) |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x26]=0x22`) + `jumptable_0x78A9B6_case_functions.csv:34` + `manifest.csv:34` | Mapping tự parse |
| 4 | `jumptable_0x78A9B6_cases.c:6746-6804` | Bản copy thứ ba |
| 5 | `case_033` + `case_035` + inline lân cận | Đối chứng họ hàng 0x25/0x27, chứng minh 0x26 độc lập |
| 6 | ~~`index.csv` grep `7349` = 0~~ → `index.csv:6439` + `functions/007349f0_FUN_007349f0.c` | **Đính chính**: gap đã được decompile trong batch mới |
| 7 | `functions/00734858 / 00734b34` | Bối cảnh class player (tham khảo thêm, không còn needed) |
| 8 | `functions/0077f414_FUN_0077F414.c:960-973` | C→S thiếu `case 0x26` |
| 9 | `functions/007349f0_FUN_007349f0.c:41-65` | **Mới**: toàn bộ logic SubOp 01 (decode `:51-52`, ghi `+0x1334` `:53`, gate `:54`, level `:55-60`, banner `:63-65`) |
| 10 | `functions/0065b484_FUN_0065b484.c:31-44` + `0065b4d4_FUN_0065b4d4.c:46-65` + `0065b52c_FUN_0065b52c.c:21-32` | **Mới**: chuỗi tính level (đếm tăng, cộng dồn ngưỡng, +20/bước) |
| 11 | `functions/0078a89c_FUN_0078a89c.c:5768` | Call-site inline truyền `*gvar_007DA7BC` (chạm body mới là `sub_007938b9`) |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] Redump `lit_734b20.hex` (tiền tố banner, `.text` trong dải gap cũ `0x7349ED–0x734B34`).
- [ ] `value` là XP tổng hay điểm tích lũy khác; `player+0x1338` là cờ gì — cần thêm call-site/constructor (vẫn ngoài export).
- [ ] `FUN_0065B52C` / bảng trong `*gvar_007DA660`: công thức ngưỡng từng cấp chưa chốt (FPU/register, có thể đọc asm tiếp).
