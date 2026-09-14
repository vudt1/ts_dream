# PHÂN TÍCH — Main OP 0x2C (44) / Case 40 / `FUN_00794910` @ `0x00794910`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler mini 3 nhánh, không default, không codec. **Cập nhật: cả 2/2 callee đã có body + tên debug `0x796AE8` đã decode.**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Fan-out mini cùng 1 manager `*gvar_007DA6C8` cho cả 3 nhánh, kiểu OP 0x2A nhưng không có blob biến dài tường minh, không nhánh số, không banner ở tầng dispatcher.
- `0x01` → `func_0x0055da08(mgr, RP)` — body mới: **đọc byte `RP[1]` (sub-code 0–0xB/0xFF) → 12 banner 2000ms** không phải forward mù.
- `0x02/0x03` → `func_0x0055dfac(mgr, 1/2)` — body mới **xác minh được guess setter/enum**: `*(mgr+0x159) = param_2` (byte) rồi virtual call `(**(mgr VMT+0x20))()` (refresh) (`0055dfac_FUN_0055dfac.c:21-22`).
- ~~2 hàm con đều chưa có body~~ → **cả 2 đã có body (2026-09-14)**; tham số mgr không dùng ở 0x01, chỉ dùng ở 0x02/0x03.
- Tên debug MainOp `0x796AE8` decode từ `redump/lit_796ae8.hex`: decode VISCII: **"Sự kiện và quang cảnh xẩy ra không phù hợp"** (văn nguyên game — đính chính suy đọc "hoàn cảnh xảy" cũ, ASCII trong raw là "quang c-nh x-y") — cùng nhóm chuỗi cảnh báo "phạm luật" với dải tên trong `lit_796a8c.hex` của OP 0x2A (dump phủ `0x796AE8–0x796CD0` = tên các OP 0x2C→0x2E+).
- Chiều C→S `case 0x2c: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2C (44) → byte_table[0x78A8EE][0x2C] = 0x28 (40)
                 → dword_table[0x78A9B6][40] @ 0x0078AA56 = 0x00794910
                 → FUN_00794910 (Case 40)
```

- File chính: `ts_decompile/case_functions/functions/case_040_00794910_FUN_00794910.c` (68 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6348-6371` — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2C`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `*(RP-4)` = length.

### 2.3. Đọc SubOp (dòng 20-28)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 (chỉ [2C]) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
if (==1) func_0x0055da08(...); else if (==2) func_0x0055dfac(...,1); else if (==3) func_0x0055dfac(...,2);
// 0x00 / ≥0x04: no-op → epilogue _LStrArrayClr/_LStrClr (dòng 37-65)
```

Không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`), không `IntToStr`/banner/chat-log.

---

## 3. Bảng tổng hợp SubOp (3 nhánh)

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2C][01][v:1B]` (3B) | `v=RP[1]` trong callee | `FUN_0055da08(*gvar_007DA6C8, RP)`: v∈{0–0xB, 0xFF} → 1 trong 12 banner `*gvar_007DA084` 2000ms; khác im lặng |
| `0x02` | `[2C][02]` (2B) | `SubOp=RP[0]` | `func_0x0055dfac(*gvar_007DA6C8, 1)` — `mgr+0x159=1` + refresh `(VMT+0x20)()` |
| `0x03` | `[2C][03]` (2B) | `SubOp=RP[0]` | `func_0x0055dfac(*gvar_007DA6C8, 2)` — `mgr+0x159=2` + refresh `(VMT+0x20)()` |
| `0x00`,`≥0x04` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (bỏ graphics/sound/animation — tầng này không có)

- **`0x01` — `FUN_0055da08`** (`0055da08_FUN_0055da08.c:23-80`): guard len≥2 → `v=RP[1]`; nhánh `v<7`: switch 0–5 (case 6 riêng) và `7≤v<0xB`: 7–0xA, `v==0xB`, `v==0xFF` — tổng **12 banner 2000ms** với literal `.text` `DAT_0055dc58…0x55df64` (chưa dump). param_1 (mgr `007DA6C8`) không dùng trong 0x01.
- **`0x02/0x03` — `FUN_0055dfac`** (`0055dfac_FUN_0055dfac.c:18-23`): setter 4 lệnh: `*(mgr+0x159)=param_2(1|2)` rồi virtual `(**(*mgr+0x20))()`. Xác minh guess "pattern setter/enum giống OP 0x24 SubOp 0B/0E".
- 12 banner của 0x01 + tên debug `0x796AE8` ("Sự kiện và quang cảnh xẩy ra không phù hợp") gợi ý mạnh đây là **kênh cảnh báo anti-cheat/phạm luật** (đối chiếu nhóm chuỗi "bị tạm khóa do phạm luật" trong `lit_796a8c.hex`) — chưa kết luận được cho từng v vì literal `.text` chưa dump.

---

## 5. Chuỗi VISCII → UTF-8

- **Đã decode tên debug**: `redump/lit_796ae8.hex` (content @ `0x796AE8`) → chuỗi đầu = decode VISCII: **"Sự kiện và quang cảnh xẩy ra không phù hợp"** (chốt) — tên MainOp 0x2C trong log `case 0:`, không phải nội dung wire. Dump còn chứa các tên opcode kế (`0x796B1C` "Tài khoản sử dụng của bạn đã bị tạm khóa do phạm luật", `0x796B5C` "Gian xảo trong vấn đáp của Bắc Đẩu Quân", `0x796B8C` "Sự kiện kết thúc trước khi chiến đấu kết thúc ", `0x796BC4` "Phi pháp sử dụng kỹ năng Kêu Gọi", `0x796BF0` "Cấm vận gia nhập đối với bạn chưa đủ 18", `0x796C20` "Đăng nhập thi đấu chuyên thuộc Server", `0x796C50` "Không thể đăng nhập thi đấu chuyên thuộc Server", `0x796C88` "Chưa đăng nhập vào Server", `0x796CAC` "Hoạt động lôi đài đấu trận kết thúc") — decode VISCII, văn nguyên game đã chốt.
- Text hiển thị thật của OP: 12 literal banner của `FUN_0055da08` ghim trong **`.text`** (`0x55dc58…0x55df64`) — chưa dump, ngoài quy ước `lit_*.hex` (`.rodata/.data`).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:982-983`: `case 0x2c: break;` — rỗng (kẹp giữa `0x2b` và `0x2d`).
- Kết luận: OP 0x2C S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2C][01][v]  3B → v∈{00–0B, FF}: banner 2000ms (12 thông điệp anti-cheat); khác: im lặng
[2C][02]     2B → mgr+0x159=1 + refresh
[2C][03]     2B → mgr+0x159=2 + refresh
ĐỪNG GỬI: [2C][00], ≥0x04 (no-op); L=1 (RangeError).
```

Tối thiểu 2B cho 02/03; **0x01 cần 3B** (`[2C][01][v]` — thiếu `v` → `BoundErr(1)` trong callee, `0055da08_FUN_0055da08.c:24-26`); tail sau `v` bị bỏ qua.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_040_00794910_FUN_00794910.c` | Handler chính 3 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6348-6371` + `:721-722` | Bản inline + tên debug `0x796ae8` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2C]=0x28`) + `manifest.csv:42` + `.csv:42` + `jumptable_0x78A9B6_cases.c` | Mapping |
| 4 | `functions/0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098` | Xác minh không gọi |
| 5 | `functions/0077f414_FUN_0077F414.c:982-983` | C→S rỗng |
| 6 | `functions/0055da08_FUN_0055da08.c` + `0055dfac_FUN_0055dfac.c` (body mới 2026-09-14) | §4: 12 banner + setter `mgr+0x159` |
| 7 | `redump/lit_796ae8.hex` (decode cp1258→NFC) | Tên debug MainOp 0x2C + dải tên 0x2C–0x2E |
| 8 | ls `redump/` (vẫn vắng literal `.text` `0x55dc58…0x55df64`) | Giới hạn còn lại |
