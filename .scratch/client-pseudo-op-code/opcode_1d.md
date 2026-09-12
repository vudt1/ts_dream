# PHÂN TÍCH — Main OP 0x1D (29) / Case 25 / `FUN_00791F76` @ `0x00791F76`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **2 chiều (S→C chủ đạo + C→S đối xứng tiền)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Hai delegate SubOp 4/9 chưa có body decompile — ghi rõ giới hạn.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Quản lý **số dư tiền của nhân vật** (`*gvar_007DA7BC + 0x12FC`) + form tiền + Toast.
- SubOp `0x01` cộng tiền, `0x02` trừ tiền (cả hai clamp: cộng không quá trần `9 999 999`, trừ không cho âm), kèm Toast `nhãn + IntToStr(số tiền) + nhãn` 2000ms.
- SubOp `0x03` Toast tĩnh, `0x05/0x06` Show 2 form, `0x07/0x08` clear 2 cờ byte của form tiền, `0x04/0x09` ủy thác nguyên RestPayload cho 2 hàm chưa decompile (`func_0x0072be10` / `func_0x00749c30`).
- Chiều C→S có `case 0x1d` thật: client đẩy field `+0x110` của form tiền lên server (DWORD LE) — cặp đồng bộ tiền 2 chiều.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1D (29) → byte_table[0x78A8EE][0x1D] = 0x19 (25)
                 → dword_table[0x78A9B6][25] @ 0x0078AA1A = 0x00791F76
                 → FUN_00791f76 (Case 25)
```

- File chính (rút gọn): `ts_decompile/case_functions/functions/case_025_00791F76_FUN_00791f76.c` (111 dòng); bản đầy đủ đối chiếu: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:4846-4914` (`case 0x1d:`).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x1D`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).
- Delphi 1-based: `_LStrCopy(RP, 2, 4)` → `RP[1..4]` = `P[2..5]`.

### 2.3. Đọc SubOp

```c
SubOp = (uint)*(byte*)(RestPayload + 0);  // RP[0] = P[1]
switch(SubOp){ case 1:..; case 2:..; case 3:..; case 4:..; case 5:..;
               case 6:..; case 7:..; case 8:..; case 9:..; }
// Không có default → SubOp lạ bị bỏ qua, chỉ cleanup _LStrClr cuối hàm
```

### 2.4. Codec dùng trong handler

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | Decode **4B → DWORD LE** `b0+b1*256+b2*65536+b3*16777216` cho SubOp 1/2 |
| `FUN_0077ee84` | Encode **DWORD → 4B LE** ở chiều C→S `case 0x1d` |
| `FUN_0077eb9c` | Không gọi trực tiếp trong handler; là tầng decode Length framing chung |
| `FUN_0077eb1c` / `FUN_0077f098` | Không gọi trong OP này (liệt kê để xác nhận loại trừ) |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[1D][01][amount:4B LE]` (6B) | `_LStrCopy(RP,2,4)` + `FUN_0077ef7c` | Cộng tiền (clamp trần), Toast số tiền |
| `0x02` | `[1D][02][amount:4B LE]` (6B) | như trên | Trừ tiền (không cho âm), Toast số tiền |
| `0x03` | `[1D][03]` (2B) | không đọc thêm | Toast literal `@0x798314` 2000ms |
| `0x04` | `[1D][04][tail...]` (đuôi opaque) | passthrough nguyên `RP` | `func_0x0072be10(player, RP)` — chưa decompile |
| `0x05` | `[1D][05]` (2B) | không đọc | Show form `*gvar_007DA46C` (VMT+0x20) |
| `0x06` | `[1D][06]` (2B) | không đọc | Show form `*gvar_007DA0A4` (VMT+0x20) |
| `0x07` | `[1D][07]` (2B) | không đọc | `*(*gvar_007DA46C + 0x11C) = 0` |
| `0x08` | `[1D][08]` (2B) | không đọc | `*(*gvar_007DA46C + 0x11D) = 0` |
| `0x09` | `[1D][09][tail...]` (đuôi opaque) | passthrough nguyên `RP` | `func_0x00749c30(player, RP)` — chưa decompile |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Cộng tiền

- **Wire**: `P[0]=0x1D, P[1]=0x01, P[2..5]=amount DWORD LE`.
- **Đọc**: `_LStrCopy(RP,2,4,&tmp)` → `amount = FUN_0077ef7c(...)`. Yêu cầu `len(RP) >= 5` nếu không `_BoundErr`.
- **Xử lý**:
  1. `ok = FUN_0072b9b8(*gvar_007DA7BC, amount)`: `cap = 9999999 - player[0x12FC]`; nếu `amount <= cap` thì `player[0x12FC] += amount`, gọi `FUN_005d895c(*gvar_007DA46C)` refresh form tiền, trả 1; ngược lại trả 0 (tràn trần, không ghi).
  2. `IntToStr(amount)` rồi `_LStrCatN(3)` nối quanh 2 literal: thành công → cặp ở `UNK_007982B8`, thất bại → `UNK_007982C8`. Ghidra mất varargs nên chỉ chắc khuôn `nhãn + số + nhãn`, chưa rõ thứ tự trái/phải. Toast 2000ms qua `gvar_007DA084`.

### 4.2. SubOp `0x02` — Trừ tiền

- **Wire**: `[1D][02][amount:4B LE]`, đọc y hệt 4.1.
- **Xử lý**:
  1. `ok = FUN_0072ba0c(*gvar_007DA7BC, amount)`: nếu `amount <= player[0x12FC]` thì `player[0x12FC] -= amount`, refresh form tiền, trả 1; ngược lại trả 0 (không cho âm).
  2. Toast `_LStrCatN(3)` quanh số: thành công → `UNK_007982E4`, thất bại → `UNK_007982F8` (thứ tự varargs tương tự — chưa xác định trái/phải).

### 4.3. SubOp `0x03` — Toast tĩnh

- **Wire**: `[1D][03]`, không tham số, không đọc thêm.
- **Xử lý**: `(VMT+0x90)(*gvar_007DA084, &UNK_00798314, 2000, 0, 0)` + `_LStrLAsg(slot, 0x798314)`. Không ghi state, không gửi gói.

### 4.4. SubOp `0x04` — Delegate `func_0x0072be10`

- **Wire**: `[1D][04][tail...]` — đuôi do callee tự parse, source không lộ.
- **Đọc**: passthrough `func_0x0072be10(*gvar_007DA7BC, RP)`.
- **Xử lý**: chưa decompile — không có file `0072be10*` trong `ts_decompile/functions/`, `index.csv` không có entry. Mock chỉ forward/log bytes thô.

### 4.5. SubOp `0x05` — Show form tiền

- **Wire**: `[1D][05]`. `(***gvar_007DA46C + 0x20)()`. Không đổi dữ liệu.

### 4.6. SubOp `0x06` — Show form thứ hai

- **Wire**: `[1D][06]`. `(***gvar_007DA0A4 + 0x20)()`. Không đổi dữ liệu.

### 4.7. SubOp `0x07` — Clear cờ `+0x11C`

- **Wire**: `[1D][07]`. `*(*gvar_007DA46C + 0x11C) = 0` (1 byte). Không Toast.

### 4.8. SubOp `0x08` — Clear cờ `+0x11D`

- **Wire**: `[1D][08]`. `*(*gvar_007DA46C + 0x11D) = 0`. Đối xứng 4.7.

### 4.9. SubOp `0x09` — Delegate `func_0x00749c30`

- **Wire**: `[1D][09][tail...]` đuôi opaque, passthrough `func_0x00749c30(*gvar_007DA7BC, RP)`. Chưa decompile — mock forward bytes thô.

---

## 5. Chuỗi VISCII → UTF-8

- **Không decode được literal nào từ source cho phép**: 4 literal Toast tiền (`0x7982B8/0x7982C8/0x7982E4/0x7982F8`) và literal tĩnh (`0x798314`) không có file `lit_*.hex` tương ứng trong `ts_decompile/redump/` (thư mục chỉ có `lit_5957D8, lit_595800...`, `lit_77F771, lit_78A854, lit_7A2094...`); grep toàn workspace chỉ trúng 2 file decompile, không có bytes thô.
- Thêm nữa `_LStrCatN(...,3)` mất varargs nên dù có bytes cũng cần listing asm push-order mới khẳng định prefix/suffix.
- Ghi cho mock: Toast SubOp 1/2/3 hiển thị được nhưng nội dung text chưa khôi phục — cần redump `.rodata` vùng `0x7982B8–0x798314`.

---

## 6. Chiều Client → Server (`case 0x1d`)

- File: `ts_decompile/functions/0077f414_FUN_0077F414.c:945-955`:
  ```c
  case 0x1d:
    ... _PStrNCat(prefix,2); _LStrFromString(...);
    FUN_0077ee84(..., *(*gvar_007DA46C+0x110), ...); // DWORD LE 4B
    _LStrCat(...); TForm1_CY_AddSedQueue(...);
  ```
- **Wire (payload trước framing)**: `[prefix:2B][value:4B LE]` = 6 bytes, với `value = *(uint*)(*gvar_007DA46C + 0x110)`. Nội dung 2-byte prefix không lộ từ decompile (chỉ biết dài 2) — cần capture live để điền.
- **Ngữ nghĩa**: client đẩy field `+0x110` của form tiền lên — đối xứng với S→C SubOp 1/2.

---

## 7. Ghi chú cho Mock Server

1. Test cộng tiền: `[1D][01][E8 03 00 00]` (=1000) → `player+0x12FC += 1000` nếu tổng ≤ 9 999 999; `[1D][02][...]` để trừ (không cho âm).
2. `[1D][03]`, `[1D][05..08]` là gói 2 bytes không tham số — an toàn replay.
3. `[1D][04]`, `[1D][09]` forward đuôi thô, chỉ log bytes.
4. C→S expect 6 bytes `[2B prefix ?][4B LE]` — cần capture live 2-byte prefix.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_025_00791F76_FUN_00791f76.c` + `functions/0078a89c_FUN_0078a89c.c:4846-4914` | Handler chính + bản đối chiếu đầy đủ |
| 2 | `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv` | Mapping |
| 3 | `functions/0072b9b8*.c`, `0072ba0c*.c` | Clamp cộng/trừ tiền tại `+0x12FC`, trần 9 999 999 |
| 4 | `functions/0077ef7c*`, `0077ee84*`, `0077eb9c*` | Codec DWORD/Word LE |
| 5 | `functions/0077f414_FUN_0077F414.c:945-955` | C→S `case 0x1d` |
