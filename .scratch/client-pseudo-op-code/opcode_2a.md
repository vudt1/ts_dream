# PHÂN TÍCH — Main OP 0x2A (42) / Case 38 / `FUN_00794719` @ `0x00794719`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Fan-out thuần 4 nhánh. **Cập nhật: cả 4/4 callee đã có body — không còn `body_size=1`.**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: **Fan-out thuần túy** kiểu OP 0x24 — `if/else-if` 4 nhánh `0x01–0x04`, không `default`. Không `IntToStr`, không cộng số, không double, không chat-log, không banner ở tầng dispatcher.
- Mỗi nhánh trao nguyên `RP` (gồm cả byte SubOp) cho 1 hàm con + 1 manager riêng:
  - `0x01` → `func_0x005e5a38(*gvar_007DA1D8, RP)`
  - `0x02` → `func_0x005e6ac4(*gvar_007DA0F8, RP)`
  - `0x03` → `func_0x00748b94(*gvar_007D9D34, RP)`
  - `0x04` → `func_0x0074ec14(*gvar_007D9D34, RP)` (chung object với 03, hàm khác)
- 4 hàm con **đã có body (2026-09-14)**; tổng hợp: mọi nhánh chỉ đọc byte `RP[1]` (sub-code) — **không có record nhiều field**, chỉ 0x01/0x02 là có thêm hành vi (banner + sound + virtual call), 0x03/0x04 là message log/banner.
- Literal `0x796A8C` (tên debug MainOP, log `case 0:`) **đã decode từ `redump/lit_796a8c.hex`**: chuỗi AnsiString đầu tại `0x796A8C` = decode VISCII: **"Sửa đổi tư liệu chiến đấu"** (đính chính dựng dấu cp1258 cũ "tài liệu") — tên nhân đọc được của OP 0x2A, xác nhận hướng "cập nhật trạng thái chiến đấu". Vùng dump còn chứa cả dải tên các opcode kế tiếp (`0x796AA8`… — cùng bảng tên debug `0x796xxx`).
- Chiều C→S `case 0x2a: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2A (42) → byte_table[0x78A8EE][0x2A] = 0x26 (38)
                 → dword_table[0x78A9B6][38] @ 0x0078AA4E = 0x00794719
                 → FUN_00794719 (Case 38)
```

- File chính: `ts_decompile/case_functions/functions/case_038_00794719_FUN_00794719.c` (71 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6256-6283` (`case 0x2a:`) — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2A`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `*(RP-4)` = length AnsiString.

### 2.3. Đọc SubOp (dòng 20-28)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 (chỉ [2A]) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
if (==1) func_0x005e5a38(...); else if (==2) func_0x005e6ac4(...);
else if (==3) func_0x00748b94(...); else if (==4) func_0x0074ec14(...);
// 0x00 / ≥0x05: no-op → epilogue _LStrArrayClr/_LStrClr (dòng 40-68)
```

Không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`) ở tầng dispatcher.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Core logic |
| :---: | :--- | :--- |
| `0x01` | `[2A][01][v:1B]` (3B) | `FUN_005e5a38(mgr 007DA1D8, RP)`: v→banner 1200ms + sound `WA0014.wav`; luôn gọi virtual `(mgr VMT+0x24)()` |
| `0x02` | `[2A][02][v:1B]` (3B) | `FUN_005e6ac4(mgr 007DA0F8, RP)`: pattern tương tự 0x01 (banner set khác) + `(mgr VMT+0x24)()` |
| `0x03` | `[2A][03][v:1B][n:1B]` (4B) | `FUN_00748b94`: v 1–6 → chat log `prefix + IntToStr(n) + suffix`; v 7/8 → banner 2000ms |
| `0x04` | `[2A][04][v:1B]` (3B) | `FUN_0074ec14`: v 1–6 → chat log `prefix + 1 trong 6 literal`; default im lặng |
| `0x00`,`≥0x05` | — | no-op |

Mẫu chung (xác minh từ body mới): `func_X(manager_obj, RP)` — wire `[2A][SubOp][v(:1B)][n?]`; parser trong hàm con chỉ đọc `RP[1]` (và `RP[2]` ở 0x03), không có blob biến dài.

---

## 4. Chi tiết từng SubOp (bỏ graphics/sound/animation — tầng này không có)

**(Đính chính: "không banner/chat-log/sound" chỉ đúng ở tầng dispatcher — cả 4 callee đều làm việc đó.)**

### 4.1. SubOp `0x01` — `FUN_005e5a38` (`005e5a38_FUN_005e5a38.c:40-63`)

- Guard len≥2 → `v=RP[1]`. v==0 → banner `DAT_005e5b44` duration 1200ms (`0x4b0`) **kèm sound**: `_LStrCat3(path, *gvar_007DA010, "sound\WA0014.wav")` → `FUN_007a7f20` (`:52-56`). v==4 → banner `DAT_005e5bb4`; v∈{1,2,3,5,6} hoặc v==260 (`v-5==0xFA`) → banner `DAT_005e5b88`; còn lại im lặng.
- Mọi đường đều kết bằng virtual call `(**(mgr VMT+0x24))()` (`:63`) — refresh manager. Literal banner nằm trong `.text`, chưa dump.

### 4.2. SubOp `0x02` — `FUN_005e6ac4` (`005e6ac4_FUN_005e6ac4.c:40-63`)

- Cùng khuôn với 0x01, khác tập giá trị: v==0 → banner `DAT_005e6bd0` + sound `WA0014.wav`; v==2 → `DAT_005e6c40`; v∈{1,3,4} hoặc v-3==0xFC (255) → `DAT_005e6c14`; rồi `(mgr VMT+0x24)()`.

### 4.3. SubOp `0x03` — `FUN_00748b94` (`00748b94_FUN_00748b94.c:58-175`)

- `switch RP[1]`: case 1–6 — guard len≥3, `IntToStr(RP[2])` nối với cặp literal `.text` theo case (`DAT_00748e08/e48/e84/ec0/efc/f34` + suffix chung `DAT_00748e3c`) → **chat log** `FUN_007ab870(*gvar_007DA1B0, 0, s, 0)` (`:175`). case 7/8 — banner `*gvar_007DA084` 2000ms với `DAT_00748f6c/0x748f94` (`:161-172`). **param_1 (mgr 007D9D34) không dùng** — chỉ là receiver thừa.
- 0x04 cùng họ dispatcher (SubOp 0x04) nhưng hàm khác, xem 4.4.

### 4.4. SubOp `0x04` — `FUN_0074ec14` (`0074ec14_FUN_0074ec14.c:43-74`)

- `switch RP[1]`: 1–6 → chọn 1 trong 6 literal `.text` (`0x74ed2c…0x74ed98`), nối prefix `LAB_0074edac` → chat log `FUN_007ab870`. Default/0 → im lặng. param_1 cũng không dùng.

**Kết luận**: 4 nhánh là kênh **thông báo trạng thái chiến đấu** (banner ngắn 1200/2000ms + chat log + 1 sound cue), không có blob dữ liệu dài — mẫu wire tối giản `[2A][SubOp][v]`.

---

## 5. Chuỗi VISCII → UTF-8

- **Đã decode tên debug**: `redump/lit_796a8c.hex` (content @ `0x796A8C`, AnsiString pack) → chuỗi đầu = decode VISCII: **"Sửa đổi tư liệu chiến đấu"**; các chuỗi kế trong cùng dump (VISCII, offset content đã chốt: `0x796AB0` "Độ sai lệch hình thái chiến đấu của bạn chơi 0", `0x796AE8` "Sự kiện và quang cảnh xẩy ra không phù hợp", `0x796B1C` "Tài khoản sử dụng của bạn đã bị tạm khóa do phạm luật", `0x796B5C` "Gian xảo trong vấn đáp của Bắc Đẩu Quân", `0x796B8C` "Sự kiện kết thúc trước khi chiến đấu kết thúc ", `0x796BC4` "Phi pháp sử dụng kỹ năng Kêu Gọi", `0x796BF0` "Cấm vận gia nhập đối với bạn chưa đủ 18", `0x796C20` "Đăng nhập thi đấu chuyên thuộc Server", `0x796C50` "Không thể đăng nhập thi đấu chuyên thuộc Server", `0x796C88` "Chưa đăng nhập vào Server", `0x796CAC` "Hoạt động lôi đài đấu trận kết thúc") — **toàn bộ là bảng tên debug MainOp 0x2A→0x2E**, không phải text wire.
- Text hiển thị thật của OP nằm trong **4 callee**: toàn bộ literal banner/chat-log ghim trong `.text` (`0x5e5b44…`, `0x748e08…`, `0x74ed2c…`) — **chưa dump** → chưa decode được; bảng tên này không dùng để thay thế.
- Wire không mang text (chỉ `[2A][SubOp][v]`).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:978-979`: `case 0x2a: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x2A S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2A][01][v]      3B → v=0: banner+sound WA0014.wav; 1/2/3/5/6/260: banner; 4: banner khác; mọi v: (mgr+0x24)()
[2A][02][v]      3B → tương tự 01 (v=0 banner+sound; 2/1/3/4/255 banner)
[2A][03][v][n]   4B → v=1..6: chat log prefix+IntToStr(n)+suffix; v=7/8: banner 2000ms
[2A][04][v]      3B → v=1..6: chat log prefix+literal[v]; khác: im lặng
ĐỪNG GỬI: [2A][00], ≥0x05 (no-op); L=1 (RangeError).
```

Tối thiểu 3B cho cả 4 nhánh (`[2A][SubOp][v]`; 0x03 cần 4B); byte thừa sau `v` bị bỏ qua (các callee chỉ đọc RP[1], RP[2]).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_038_00794719_FUN_00794719.c` (71 dòng) | Handler chính 4 nhánh + epilogue |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6256-6283` + `:715-716` | Bản inline + tên debug `0x796a8c` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2A]=0x26`) + `.csv:40` + `manifest.csv:40` | Mapping |
| 4 | `functions/0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098` | Codec (không gọi trực tiếp) |
| 5 | `functions/0077f414_FUN_0077F414.c:978-979` | C→S rỗng |
| 6 | 4 body mới: `functions/005e5a38_FUN_005e5a38.c`, `005e6ac4_FUN_005e6ac4.c`, `00748b94_FUN_00748b94.c`, `0074ec14_FUN_0074ec14.c` | §4.1–4.4 |
| 7 | `redump/lit_796a8c.hex` (decode cp1258→NFC) | Tên debug MainOp 0x2A + bảng tên 0x2A–0x2E |
