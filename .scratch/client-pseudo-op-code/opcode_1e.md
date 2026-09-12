# PHÂN TÍCH — Main OP 0x1E (30) / Case 26 / `FUN_007921A6` @ `0x007921A6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Dispatcher + 1 handler con đã xác minh từ mã nguồn sơ cấp**; 10/11 handler con chưa có body decompile — ghi rõ giới hạn, không suy đoán layout params.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Dispatcher cấp 2. `FUN_007921A6` chỉ đọc 1 byte SubOp rồi **ủy thác nguyên RestPayload** cho 11 handler con theo 2 nhóm object:
  - **Nhóm A** (`*gvar_007DA58C`, 8 SubOp: `0x01,02,03,04,05,09,0A,0B`) — cùng 1 manager (chưa định danh class từ source hiện có).
  - **Nhóm B** (`*gvar_007DA7C0`, 2 SubOp: `0x0C,0x0D`) — module họ `0x0077bxxx` riêng.
  - **Nhóm C** (SubOp `0x08`, `FUN_0076fdb8()`) — reset 50 slot, không cần payload (duy nhất có full source).
- SubOp `0x06, 0x07` là slot trống (không có case → client bỏ qua).
- Trong dispatcher **không có** literal chuỗi, Toast, caption, hay xử lý font/charset — nên **không có bằng chứng VISCII** ở tầng này. Nếu params chứa text Việt thì nằm trong 10 handler chưa decompile.
- Chiều C→S `case 0x1e: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1E (30) → byte_table[0x78A8EE][0x1E] = 0x1A (26)
                 → dword_table[0x78A9B6][26] @ 0x0078AA1E = 0x007921A6
                 → FUN_007921a6 (Case 26)
```

- File chính: `ts_decompile/case_functions/functions/case_026_007921A6_FUN_007921a6.c:1-4` (header Case 26).
- Dispatcher tổng: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt:26-31`.
- Framing/XOR/pump như `opcode_00_01.md` §2. `EBP-0xC` trong hàm = RestPayload (ECX lúc entry).

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x1E`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).

### 2.3. Đọc SubOp (dòng 20-26)

```c
if (*(RestPayload-4) == 0) _BoundErr(0);   // RP rỗng → RangeError
SubOp = (uint)*(byte*)(RestPayload + 0);   // RP[0] = P[1]
switch(SubOp){ case 1:..; case 2:..; case 3:..; case 4:..; case 5:..;
  case 8:..; case 9:..; case 0xA:..; case 0xB:..; case 0xC:..; case 0xD:..; }
// Không có case 6/7/default → rơi qua switch, chỉ cleanup
```

- Epilogue chung (dòng 61-89): `_LStrArrayClr/_LStrClr` ~15 biến tạm — dọn chuỗi Delphi, không phải nghiệp vụ.
- Dispatcher **không gọi codec nào** trực tiếp; chỉ `_BoundErr` + cleanup. Các codec là kiến thức nền để giải params khi có thêm body handler con.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Handler | Object | Tình trạng source |
| :---: | :--- | :--- | :--- | :--- |
| `0x01` | `[1E][01][params...]` (opaque) | `func_0x007707a0` | `*gvar_007DA58C` | Chưa decompile |
| `0x02` | `[1E][02][params...]` | `func_0x00770454` | `*gvar_007DA58C` | Chưa decompile |
| `0x03` | `[1E][03][params...]` | `func_0x00770170` | `*gvar_007DA58C` | Chưa decompile |
| `0x04` | `[1E][04][params...]` | `func_0x007705d8` | `*gvar_007DA58C` | Chưa decompile |
| `0x05` | `[1E][05][params...]` | `func_0x0077021c` | `*gvar_007DA58C` | Chưa decompile |
| `0x06` | — | — | — | Slot trống, client ignore |
| `0x07` | — | — | — | Slot trống, client ignore |
| `0x08` | `[1E][08]` (không params) | `FUN_0076fdb8()` | none (dùng `gvar_007DA7BC` nội bộ) | **Có full source** |
| `0x09` | `[1E][09][params...]` | `func_0x007714a0` | `*gvar_007DA58C` | Chưa decompile |
| `0x0A` | `[1E][0A][params...]` | `func_0x007702d0` | `*gvar_007DA58C` | Chưa decompile |
| `0x0B` | `[1E][0B][params...]` | `func_0x007700bc` | `*gvar_007DA58C` | Chưa decompile |
| `0x0C` | `[1E][0C][params...]` | `func_0x0077b72c` | `*gvar_007DA7C0` | Chưa decompile |
| `0x0D` | `[1E][0D][params...]` | `func_0x0077b938` | `*gvar_007DA7C0` | Chưa decompile |

Đã kiểm `index.csv` (6302 dòng): trong 11 handler con chỉ `0076fdb8` có mặt. Pattern truyền nguyên `RP` (gồm cả byte SubOp) cho handler tự cắt tiếp — giống Case 25/27.

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1–4.5. SubOp `0x01–0x05` — Nhóm A (opaque)

- **Wire**: `[1E][01..05][params...]` — layout params chưa phục hồi.
- **Đọc**: passthrough nguyên `RP` cho handler con.
- **Xử lý**: thao tác trên object `*gvar_007DA58C`. Không có bằng chứng text/Toast ở tầng dispatcher. Không suy đoán thêm.

### 4.6. SubOp `0x06, 0x07` — Reserved

- Không có case → bỏ qua hoàn toàn. Server không nên gửi; đây là 2 slot trống duy nhất trong dải 1–13.

### 4.7. SubOp `0x08` — Reset 50 slot (DUY NHẤT CÓ SOURCE)

- **Wire**: `[1E][08]` — 2 bytes, không params. Chứng cứ: signature `void FUN_0076fdb8(void)` (`0076fdb8_FUN_0076fdb8.c:19`), dispatcher gọi trần không tham số.
- **Đọc**: không đọc buffer.
- **Xử lý** (thuần logic, không chuỗi, không codec):
  ```c
  for (i = 1; i != 0x33; i++)   // 1..50
    FUN_0077250c(*(gvar_007DA7BC + 0xA48 + i*4), 0x32 /*=50*/);
  ```
  Ngữ nghĩa: reset/clear 50 slot (mảng con trỏ tại `player + 0xA48`, hằng `0x32=50` trùng mẫu pop tối đa 50 gói/tick). Khả năng cao là clear buff/item-page/slot khi đổi map/logout — mức suy luận, cần đối chiếu thêm `FUN_0077250c`.

### 4.8–4.10. SubOp `0x09, 0x0A, 0x0B` — Nhóm A (opaque)

- **Wire**: `[1E][09/0A/0B][params...]`. Handler `0x007714a0` nằm xa cụm `0x007700xx` nên có thể là hàm "nặng" (list lớn/mở form) — mức suy luận, không khẳng định.

### 4.11–4.12. SubOp `0x0C, 0x0D` — Nhóm B (opaque)

- **Wire**: `[1E][0C/0D][params...]` trên object `*gvar_007DA7C0` (module họ `0x0077bxxx`, cùng họ với `func_0x0077bd2c` ở OP 0x1F SubOp 0x0E).

---

## 5. Chuỗi VISCII → UTF-8

- **Không có bằng chứng VISCII trong phạm vi source of truth của OP 0x1E**: dispatcher không literal, không `FUN_007b372c` (set caption), không `_LStrCat`, không Toast `(...+0x90, 2000)`; `FUN_0076fdb8` thuần số + con trỏ.
- Nếu params SubOp 1–5/9–13 chứa tên item/thông báo Việt thì nằm trong 10 handler chưa decompile → cần decompile bổ sung `0x007700bc..0x007714a0` + `0x0077b72c/0x0077b938` mới kết luận. Không suy đoán bảng mã.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077F414_FUN_0077F414.c:956-957`: `case 0x1e: break;` — rỗng hoàn toàn, không build frame.
- Kết luận: OP 0x1E là S→C một chiều. Mock chỉ cần craft S→C `[TOKEN][LEN][1E][SubOp][params]` (XOR `0xAD`).

---

## 7. Ghi chú cho Mock Server / Fuzzer

```
S→C: [TK0][TK1][L0][L1][0x1E][SubOp][params...]
  SubOp ∈ {01,02,03,04,05,08,09,0A,0B,0C,0D}
  08: params rỗng (chắc chắn) — an toàn replay
  còn lại: params opaque (chưa đặc tả được — đừng gửi bừa, chỉ log)
  06,07,giá trị khác: client ignore
C→S: không tồn tại
```

Việc còn lại để full spec: decompile 10 hàm `007700bc, 00770170, 0077021c, 007702d0, 00770454, 007705d8, 007707a0, 007714a0, 0077b72c, 0077b938`, định danh `gvar_007DA58C` / `gvar_007DA7C0`.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_026_007921A6_FUN_007921a6.c:26-60` | Toàn bộ switch SubOp |
| 2 | `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_0x78A9B6_case_functions.csv:26` | Mapping |
| 3 | `functions/0076fdb8_FUN_0076fdb8.c:19` | SubOp 8: loop reset 50 slot |
| 4 | `functions/0077F414_FUN_0077F414.c:956-957` | C→S rỗng |
| 5 | `functions/0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | Codec nền (xác nhận loại trừ ở tầng dispatcher) |
| 6 | `index.csv` (6302 dòng) | Xác nhận 10 handler con vắng mặt |
