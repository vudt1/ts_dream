# PHÂN TÍCH — Main OP 0x1D (29) / Case 25 / `FUN_00791F76` @ `0x00791F76`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **2 chiều (S→C chủ đạo + C→S đối xứng tiền)**
Trạng thái: ~~Hai delegate SubOp 4/9 chưa có body decompile — ghi rõ giới hạn.~~ → **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only), cả hai delegate đã có body.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). Hai delegate `func_0x0072be10`/`func_0x00749c30` đã có body ⇒ SubOp 0x04/0x09 **không còn đuôi opaque**. Window dump `lit_7980xx.hex` của OP 0x1A phủ tới `0x7982F8` ⇒ 4/5 literal Toast của OP này **đã dịch được** (VISCII — xem đính chính mã hóa ở `opcode_19.md` §6); chỉ còn `0x798314` dở (30 byte, có 4 byte trong window).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Quản lý **số dư tiền của nhân vật** (`*gvar_007DA7BC + 0x12FC`) + form tiền + Toast.
- SubOp `0x01` cộng tiền, `0x02` trừ tiền (cả hai clamp: cộng không quá trần `9 999 999`, trừ không cho âm), kèm Toast `nhãn + IntToStr(số tiền) + nhãn` 2000ms.
- SubOp `0x03` Toast tĩnh, `0x05/0x06` Show 2 form, `0x07/0x08` clear 2 cờ byte của form tiền, ~~`0x04/0x09` ủy thác nguyên RestPayload cho 2 hàm chưa decompile~~ → **`0x04` = SET tuyệt đối số dư `player+0x12FC`** (`func_0x0072be10`, đã có body) và **`0x09` = ghi byte `RP[1]` vào form `*gvar_007DA0A4 + 0x418` rồi redraw** (`func_0x00749c30`, đã có body).
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
| `0x04` | `[1D][04][money:4B LE]` (6B) — **đính chính: không còn đuôi opaque** | `Copy(RP,2,4)` + `FUN_0077ef7c` trong callee | `func_0x0072be10`: **`player+0x12FC = money` (SET tuyệt đối)** + refresh form tiền (`0072be10_FUN_0072be10.c:37–40`) |
| `0x05` | `[1D][05]` (2B) | không đọc | Show form `*gvar_007DA46C` (VMT+0x20) |
| `0x06` | `[1D][06]` (2B) | không đọc | Show form `*gvar_007DA0A4` (VMT+0x20) |
| `0x07` | `[1D][07]` (2B) | không đọc | `*(*gvar_007DA46C + 0x11C) = 0` |
| `0x08` | `[1D][08]` (2B) | không đọc | `*(*gvar_007DA46C + 0x11D) = 0` |
| `0x09` | `[1D][09][x:1B]` (3B) — **đính chính: không còn đuôi opaque** | guard `len(RP)≥2`, đọc `RP[1]` trong callee | `func_0x00749c30`: `**gvar_007DA0A4+0x418 = (DWORD)x` + `FUN_005db9a4(**gvar_007DA0A4, 2)` redraw (`00749c30_FUN_00749c30.c:21–28`) |

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

### 4.4. SubOp `0x04` — **SET tuyệt đối số dư `player+0x12FC`** (thân `func_0x0072be10` đã phục hồi)

- **Wire**: `[1D][04][money:4B LE]` (6B). ~~đuôi opaque~~ → **đính chính: không có parse ẩn**.
- **Đọc** (`0072be10_FUN_0072be10.c:37–38`): `_LStrCopy(RP,2,4)` → `FUN_0077ef7c` = DWORD LE.
- **Xử lý**: `*(*gvar_007DA7BC + 0x12FC) = money` **ghi trực tiếp, không clamp** (d.39 — khác SubOp 1/2 là cộng/trừ clamp qua `0072b9b8/0072ba0c`; đây chính là "set" đối xứng với C→S đẩy `+0x110` lên), sau đó `FUN_005d895c(*gvar_007DA46C)` refresh form tiền (d.40). param_1 bị bỏ qua — hàm dùng thẳng global.
- Mock: an toàn để replay `[1D][04][E8 03 00 00]` → số dư = 1000 (lưu ý field 7-chữ-số `%7d` trên form — giá trị > 9 999 999 sẽ hiển thị lỗi nhưng **không** bị chặn tĩnh ở đây).

### 4.5. SubOp `0x05` — Show form tiền

- **Wire**: `[1D][05]`. `(***gvar_007DA46C + 0x20)()`. Không đổi dữ liệu.

### 4.6. SubOp `0x06` — Show form thứ hai

- **Wire**: `[1D][06]`. `(***gvar_007DA0A4 + 0x20)()`. Không đổi dữ liệu.

### 4.7. SubOp `0x07` — Clear cờ `+0x11C`

- **Wire**: `[1D][07]`. `*(*gvar_007DA46C + 0x11C) = 0` (1 byte). Không Toast.

### 4.8. SubOp `0x08` — Clear cờ `+0x11D`

- **Wire**: `[1D][08]`. `*(*gvar_007DA46C + 0x11D) = 0`. Đối xứng 4.7.

### 4.9. SubOp `0x09` — Ghi field `+0x418` + redraw form `*gvar_007DA0A4` (`func_0x00749c30` đã phục hồi)

- **Wire**: `[1D][09][x:1B]` (3B). ~~đuôi opaque~~ → **đính chính: chỉ 1 byte**.
- **Đọc** (`00749c30_FUN_00749c30.c:21–27`): guard `Len(RP)<2 → BoundErr(1)`; `x = RP[1]` byte thường.
- **Xử lý**: `*(*gvar_007DA0A4 + 0x418) = (uint)x` (d.27 — ghi 4 byte từ 1 byte), rồi `FUN_005db9a4(*gvar_007DA0A4, 2)` (d.28 — vẽ/refresh form với tham số mode 2; `gvar_007DA0A4` chính là form được Show ở SubOp 0x06). param_1 (player) bị bỏ qua.
- Chưa kết luận được ý nghĩa nghiệp vụ của field `+0x418` — chỉ thấy cơ chế ghi byte + redraw.

---

## 5. Chuỗi VISCII → UTF-8 (cập nhật 2026-09-14: 4/5 literal ĐÃ DỊCH)

- Window dump `redump/lit_798038.hex` (base 0x798038 + 512B) và `lit_798118.hex` phủ các address Toast của OP này. Decode **VISCII** (đính chính mã hóa — xem `opcode_19.md` §6; các run `[FF FF FF FF refcount][len:4LE][bytes][00]`):

  | Địa chỉ content | Dump / offset | Len (byte/ký tự) | Chuỗi nguyên văn | Vai trò |
  |---|---|---|---|---|
  | `0x007982B8` | `lit_798118.hex` off 0x1A0 | 7 | `Lưu trữ` | literal SubOp 0x01 **nhánh thành công** (`0078a89c…c:4867`) |
  | `0x007982C8` | `lit_798118.hex` off 0x1B0 | 16 | `Thất bại lưu trữ` | SubOp 0x01 **nhánh thất bại** (`0078a89c…c:4862`) |
  | `0x007982E4` | `lit_798118.hex` off 0x1CC | 8 | `Rút nhận` | SubOp 0x02 **thành công** (`0078a89c…c:4882`) |
  | `0x007982F8` | `lit_798118.hex` off 0x1E0 | 17 | `Thất bại rút nhận` | SubOp 0x02 **thất bại** (`0078a89c…c:4877`) |
  | `0x00798314` | `lit_798118.hex` off 0x1FC | **30** | **CHỈ dump được 4 byte: `Dung`** — header `[FF FF FF FF][1E 00 00 00]` tại 0x79830C, toàn string kết thúc ngoài window 512B | Toast tĩnh SubOp 0x03 — **GIỚI HẠN CÒN LẠI**, cần redump tới null |

- Hệ quả nghiệp vụ (từ chính text): SubOp `0x01` = **gửi/nạp tiền ("Lưu trữ")**, SubOp `0x02` = **rút/nhận tiền ("Rút nhận")** — khớp cơ chế cộng/trừ `+0x12FC` của `0072b9b8/0072ba0c`; "Toast số tiền" nên đọc là toast xác nhận nạp/rút, không phải toast generic.
- Vẫn đúng như ghi chú cũ: `_LStrCatN(...,3)` mất varargs. Bản inline `0078a89c_FUN_0078a89c.c:4856–4884` cho biết **nhánh nào dùng hằng nào**: SubOp 1 — fail (`FUN_0072b9b8` trả 0) → `0x7982C8`, success → `0x7982B8` (d.4861–4868); SubOp 2 — fail → `0x7982F8`, success → `0x7982E4` (d.4876–4883). Tuy nhiên hằng hiện trong C (`in_stack_ffffffd4`) là **first-pushed = mảnh NGOÀI CÙNG BÊN PHẢI** (chuẩn hóa theo `0072bcf8.asm.txt:49,55`: `PUSH 0x72bdf8` trước ⇒ `0x72BDF8` là đuôi caption, `"kim"` push cuối mới là đầu trái) ⇒ toast = `<hằng chưa thấy> + IntToStr(số) + "Lưu trữ"|"Thất bại lưu trữ"|"Rút nhận"|"Thất bại rút nhận">`; **mảnh bên trái vẫn chưa xác định được** — chưa kết luận được ghép đầy đủ.
- Ghi cho mock: Toast SubOp 1/2 hiển thị được với text đã biết (trừ mảnh ghép thứ hai); SubOp 3 — string 30 ký tự bắt đầu bằng "Dung…" (giống họ `"Dung lượng …"` đã thấy ở `0x7980B4`) — chưa đủ bytes, không bịa tiếp.

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

1. Test cộng tiền: `[1D][01][E8 03 00 00]` (=1000) → `player+0x12FC += 1000` nếu tổng ≤ 9 999 999; `[1D][02][...]` để trừ (không cho âm). Toast xác nhận now có text đã dịch: "Lưu trữ…"/"Rút nhận…" (mục 5).
2. `[1D][03]`, `[1D][05..08]` là gói 2 bytes không tham số — an toàn replay.
3. ~~`[1D][04]`, `[1D][09]` forward đuôi thô, chỉ log bytes~~ → **đặc tả đã chốt**: `[1D][04][money:4B LE]` = SET `player+0x12FC` (6B); `[1D][09][x:1B]` = ghi form `+0x418` + redraw (3B). Cả hai an toàn replay nhưng **thay đổi state thật** (số dư tiền / field form 0x7DA0A4).
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
| 6 | `functions/0072be10_FUN_0072be10.c:37–40` | **Body mới**: SubOp 0x04 = SET `player+0x12FC` (không clamp) + refresh `gvar_007DA46C` |
| 7 | `functions/00749c30_FUN_00749c30.c:21–28` | **Body mới**: SubOp 0x09 = byte `RP[1]` → `**gvar_007DA0A4+0x418` + `FUN_005db9a4(…,2)` |
| 8 | `redump/lit_798118.hex` (window 0x798118–0x798318) + `functions/0078a89c_FUN_0078a89c.c:4856–4884` | Literal Toast SubOp 1/2 dịch VISCII (`Lưu trữ/Thất bại lưu trữ/Rút nhận/Thất bại rút nhận`); `0x798314` mới có 4/30 byte — **giới hạn còn lại** |
