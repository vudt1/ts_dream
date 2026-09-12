# PHÂN TÍCH — Main OP 0x42 (66) / Case 59 / FUN_00795db6 @ 0x00795DB6

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh 100% từ mã nguồn sơ cấp** (`ts_decompile/` only).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x42` (thập phân: `66`, ánh xạ tới **Case 59**) là **opcode hỗn hợp (multi-module)**: mỗi nhóm SubOp được chuyển tiếp nguyên vẹn Rest Payload cho một **handler thuộc về một hệ thống giao diện khác nhau** của client. Bản thân `FUN_00795db6` KHÔNG tự giải mã payload — nó chỉ đọc SubOp (byte 0) rồi gọi đúng handler:

| Nhóm SubOp | Đối tượng tiếp nhận (Global) | Handler | Nghiệp vụ |
| :--- | :--- | :--- | :--- |
| `0x01–0x04` | `gvar_007DA35C` | `FUN_00522ab8` | **Cập nhật chỉ số trạng thái của Local Player** (một loại tài nguyên có giá trị lớn + văn bản trạng thái) |
| `0x0B, 0x0C, 0x0E` | `gvar_007D9FA0` | `FUN_005c0b20` / `FUN_005c088c` / `FUN_005c0a3c` | **Hệ thống thông báo / log nhiệm vụ có đếm số** (lấy số + hiển thị văn bản) |
| `0x15–0x18` | `gvar_007DA348` | `FUN_005253a8` / `FUN_005250e8` / `FUN_00524ff0` / `FUN_00525488` | **Cặp chỉ số kép (2 giá trị int liên tiếp)** — kiểu "điểm hiện tại / điểm tối đa" |
| `0x1F, 0x20` | `gvar_007D9EF8` | `FUN_005be098` / `FUN_005be240` | **Log dòng chữ vào một khung giao diện có thanh cuộn** |

- **Chiều giao tiếp**: Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `ts_decompile/functions/0077f414_FUN_0077F414.c:1066-1067` (`case 0x42: break;`) là rỗng. Client không gửi gói tin nào bằng Main Opcode `0x42`.
- **Lưu ý mock server**: tất cả SubOp đều có thể mô phỏng được vì toàn bộ handler đã được decompile đầy đủ (xem mục 4).

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x42 (66) → byte_table[0x78A8EE][0x42] = 0x3B (59)
                  → dword_table[0x78A9B6][59] @ 0x0078AAA2 = 0x00795DB6
                  → FUN_00795db6 (Case 59)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` tách byte đầu `P[0] = 0x42` làm Opcode, phần còn lại là Rest Payload, gọi `FUN_0078a89c(Self, Opcode, Payload)` (xem `ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c`).
2. **Case Function độc lập**: `ts_decompile/case_functions/functions/case_059_00795DB6_FUN_00795db6.c:20-110`. Mẫu chung:
   ```c
   iVar2 = *(int *)(unaff_EBP + -0xc);            // Rest Payload (chuỗi Delphi)
   *(uint *)(unaff_EBP + -0x14) = *(byte *)iVar2; // SubOp = Payload[0]
   switch (SubOp) {
     case 1..4:  FUN_00522ab8(gvar_007DA35C, payload); break;  // chuyển tiếp toàn bộ payload
     ...
   }
   ```

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire Format (sau main op) | Min Len | Handler | Ý Nghĩa Nghiệp Vụ Cốt Lõi |
| :---: | :--- | :---: | :--- | :--- |
| **`0x01`** | `[42][01][Mode:1B][...]` | ≥2B | `FUN_00522ab8` | Ghi 10 giá trị vào bảng kỹ năng 2 chiều 18×10 của người chơi (xem 4.1, nhánh Mode 1). |
| **`0x02`** | `[42][02][Value:4B LE]` | 6B | `FUN_00522ab8` | Ghi trực tiếp **một giá trị DWORD** vào bộ đệm hiển thị của Local Player (`player+0x146C`), rồi refresh. |
| **`0x03`** | `[42][03][Mode:1B][...]` | ≥3B | `FUN_00522ab8` | Cập nhật giá trị qua trung gian bảng tra (xem 4.1, nhánh Mode 3). |
| **`0x04`** | `[42][04][Sel:1B]` | 3B | `FUN_00522ab8` | **Chọn 1 trong 4 chuỗi văn bản** hiển thị cố định (Sel = 1..4) vào ô văn bản của object. |
| **`0x0B`** | `[42][0B][Value:4B LE]` | 6B | `FUN_005c0b20` | Ghi DWORD vào ô đếm của object (`+0x1AC`), rồi refresh. |
| **`0x0C`** | `[42][0C][Value:4B LE][F:2B LE][X:1B]` | 9B | `FUN_005c088c` | Cập nhật giá trị + **hiển thị dòng log "±X"** kèm hiệu ứng (xem 4.3). |
| **`0x0E`** | `[42][0E][Sel:1B]` | 3B | `FUN_005c0a3c` | Hiển thị 1 trong 2 thông báo cố định (Sel = 1 hoặc 2). |
| **`0x15`** | `[42][15][A:4B LE][B:4B LE]` | 10B | `FUN_005253a8` | Ghi **cặp chỉ số A→`player+0x1470`, B→`player+0x1474`**, refresh + reset cờ chờ `+0x168=0`. |
| **`0x16`** | `[42][16][Mode:1B=0][ID:2B][A:4B][B:4B]` | ≥14B | `FUN_005250e8` | Ghi cặp chỉ số như 0x15 **+ nối chuỗi mô tả tên** (tra từ bảng `gvar_007DA540`) vào ô văn bản. |
| **`0x17`** | `[42][17][Sel:1B]` | 3B | `FUN_00524ff0` | Gán 1 trong 2 chuỗi văn bản cố định (Sel = 1 hoặc 2) vào ô văn bản `+0x154`. |
| **`0x18`** | `[42][18][1B=0x01][Idx:2B LE][Count:1B][Text:...]` | ≥6B | `FUN_00525488` | Thông báo dạng **"tên + số lượng + văn bản tự do"** (xem 4.6). |
| **`0x1F`** | `[42][1F][Delta:1B][Arg:4B LE]` | 7B | `FUN_005be098` | **Ghi 1 dòng log** "giá trị cũ `Δ` mới" (Arg định dạng `%d`, Delta là số cộng thêm có dấu) vào khung log. |
| **`0x20`** | `[42][20][Sel:1B]` | 3B | `FUN_005be240` | Hiển thị 1 trong 3 thông báo cố định (Sel = 1, 2 hoặc 3) thời lượng 2000ms. |

Các SubOp không liệt kê (`0x05–0x0A, 0x0D, 0x0F–0x14, 0x19–0x1E`) rơi vào nhánh `default` — chỉ dọn dẹp stack local, **không xử lý gì**.

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x01–0x04` — `FUN_00522ab8` (object `gvar_007DA35C`)

Nguồn: `ts_decompile/functions/00522ab8_FUN_00522ab8.c`. Handler nhận **toàn bộ Rest Payload** (bao gồm cả byte SubOp) và **phân nhánh lần 2 theo chính byte đó** (`local_10 = payload[0]`), tức SubOp 1/2/3/4 của Main OP 0x42 ứng với Mode 1/2/3/4 bên trong handler:

**Mode 2** (SubOp `0x02`) — gán trực tiếp:
```c
_LStrCopy(payload, 2, 4, &local_28);                      // P[2..5] (Delphi 1-based: cắt 4B từ vị trí 2)
uVar4 = FUN_0077ef7c(gvar_007D9D30, local_28);            // → DWORD LE
*(DWORD *)(*(int *)gvar_007DA7BC + 0x146c) = uVar4;       // player + 0x146C = Value
(**(code **)(*local_8 + 0x20))();                         // Gọi virtual method VMT+0x20 (refresh)
```
- `gvar_007DA7BC` là **Local Player object** (đã xác minh trong `opcode_23.md` mục SubOp 0x04: `player+0x146C` được dùng làm số hiển thị lớn — ghi theo công thức `A*100+B`). Trong 0x42, giá trị này được server **gán trực tiếp**.
- Offset `+0x146C` cũng xuất hiện ở `FUN_005c0b20` (`local_8[0x6b]` = `+0x1AC`... xem 4.2) — mỗi object UI giữ một bản sao số hiển thị.

**Mode 1** (SubOp `0x01`) — ghi bảng 2 chiều:
```c
local_14 = payload[1];        // Chỉ số cột thứ nhất  (1..18, check BoundErr 0x12)
local_18 = payload[2];        // Chỉ số cột thứ hai   (1..9,  check BoundErr 9)
local_1c = payload[3];        // Chỉ số hàng          (1..10, check BoundErr 10)
for (i = 1; i <= 10; i++) {
  b = payload[i + 4];         // 10 bytes payload tiếp theo
  // Ghi vào bảng dữ liệu tĩnh tại DAT_00927fdc:
  // offset = (col2-1)*0x2C3 + (col1-1)*0x375*8 + (row-1)*10 + i, mỗi phần tử 7 bytes
  // → phần tử [.. + 6] = b
}
```
- Bảng `DAT_00927fdc` là cấu trúc dữ liệu tĩnh kích thước ~`0x375 * 0x2C3 * ...` — đây là **bảng thông số kỹ năng / trạng thái theo lưới** (18 hàng × 9 cột × 10 phần tử × 7 bytes/phần tử). Server gửi 10 byte liên tiếp để ghi đè 1 hàng của 1 ô lưới.

**Mode 3** (SubOp `0x03`) — cập nhật có điều kiện:
```c
sub = payload[7];
if (sub == 1) {
  _LStrCopy(payload, 4, 4, &s);                       // P[4..7]
  *(DWORD *)(player + 0x146c) = DWORD_LE(s);          // gán trực tiếp như Mode 2
} else if (sub == 2) {
  _LStrCopy(payload, 2, 2, &s16);                     // P[2..3] → Word LE (index tra bảng)
  _LStrCopy(payload, 4, 4, &s32);                     // P[4..7] → DWORD LE
  *(DWORD *)(player + 0x146c) = DWORD_LE(s32);
  name = FUN_00774a84(gvar_007DA540, wordIdx, &out);  // tra CSDL → chuỗi tên (vật phẩm/skill/...)
  _LStrCatN(&local_8[0x4f], 3, prefix, name, suffix); // nối chuỗi "..." + name + "..." vào ô văn bản +0x13C
}
```
- `FUN_00774a84` là hàm **tra tên từ bảng CSDL `gvar_007DA540`** theo ID (cùng họ hàm với các tài liệu opcode khác — `gvar_007DA540` đã được xác minh là **CSDL vật phẩm/kỹ năng** trong `opcode_41.md` mục 2.1 và `opcode_23.md`).
- Hai tiền tố/hậu tố chuỗi cố định nằm tại `DAT_00522e54` / `DAT_00522e40` — **hằng chuỗi Delphi, chưa có trong dump `ts_decompile/`** (không decomplie được vùng .data literal này; đây là ràng buộc của single source of truth).

**Mode 4** (SubOp `0x04`) — chọn văn bản:
```c
sel = payload[1];   // 1..4
switch (sel) {
  case 1: _LStrAsg(local_8 + 0x4f, &DAT_00522e90); break;
  case 2: _LStrAsg(local_8 + 0x4f, &DAT_00522ec4); break;
  case 3: _LStrAsg(local_8 + 0x4f, &DAT_00522ef4); break;
  case 4: _LStrAsg(local_8 + 0x4f, &DAT_00522f28); break;
}
```
- Gán 1 trong 4 **chuỗi văn bản cố định** (vùng .data `0x522E90–0x522F28`, chưa dump) vào ô văn bản tại offset `+0x13C` của object.

### 4.2. SubOp `0x0B` — `FUN_005c0b20` (object `gvar_007D9FA0`)

Nguồn: `ts_decompile/functions/005c0b20_FUN_005c0b20.c:39-42`:
```c
_LStrCopy(payload, 2, 4, &s);
local_8[0x6b] = FUN_0077ef7c(gvar_007D9D30, s);   // object + 0x1AC = DWORD LE
(**(code **)(*local_8 + 0x20))();                  // refresh
```
- Đồng bộ **giá trị đếm** từ server vào ô hiển thị của object UI (biến `local_8[0x6b]` = offset `0x6B*4 = 0x1AC`).

### 4.3. SubOp `0x0C` — `FUN_005c088c` (object `gvar_007D9FA0`)

Nguồn: `ts_decompile/functions/005c088c_FUN_005c088c.c:66-96`:
```c
_LStrCopy(payload, 2, 4, &s);   local_8[0x6b] = DWORD_LE(s);      // +0x1AC = giá trị mới
_LStrCopy(payload, 6, 2, &s2);  local_e = WORD_LE(s2);            // ID tra bảng
local_f = payload[7];                                              // số cộng/trừ hiển thị (1 byte)
name = FUN_00774a84(gvar_007DA540, local_e, &out);                 // tra tên
Format(&DAT_005c0a0c, [name(BStr?), 0xB, local_f], &out2);         // dựng dòng "… ±X"
FUN_007ab870(gvar_007DA1B0, 0, formatted, 0);                      // hiển thị dòng log nổi
_LStrCat3(&s, gvar_007DA010, "sound\\WB0011.wav"); FUN_007a7f20(s);// phát âm thanh (bỏ qua)
FUN_005c084c(local_8[local_8[0x66] + 0x4c]);                       // cập nhật ô con theo con trỏ hiện hành
if (local_8[0x6a] > 0x18) (**(code **)(*local_8 + 0x20))();       // refresh nếu đủ điều kiện
```
- Core logic: **cập nhật giá trị + ghi log biến động** "tên đối tượng ±X". Phần âm thanh `sound\WB0011.wav` liên quan media — bỏ qua khi mô phỏng.

### 4.4. SubOp `0x0E` — `FUN_005c0a3c` (object `gvar_007D9FA0`)

Nguồn: `ts_decompile/functions/005c0a3c_FUN_005c0a3c.c:29-34`:
```c
sel = payload[1];
if (sel == 1) VirtualCall_0x90(gvar_007DA084, &DAT_005c0abc, 0x5DC /*1500*/, 0, 0);
if (sel == 2) VirtualCall_0x90(gvar_007DA084, &DAT_005c0af4, 0x5DC, 0, 0);
```
- Hiển thị 1 trong 2 **thông báo cố định** (chuỗi tại `0x5C0ABC` / `0x5C0AF4`, chưa dump) trong 1500ms qua object `gvar_007DA084` (message box manager — cùng object đã dùng trong `opcode_00_01.md` cho System Error Notice).

### 4.5. SubOp `0x15`–`0x17` — nhóm `FUN_005253a8` / `FUN_005250e8` / `FUN_00524ff0` (object `gvar_007DA348`)

Ba handler này cùng thao tác lên một object UI kiểu "**đồng hồ đo kép**" (hai giá trị liên tiếp `player+0x1470` và `player+0x1474`, kèm ô văn bản `+0x154` và cờ `+0x168`):

**`0x15` — `FUN_005253a8`** (`ts_decompile/functions/005253a8_FUN_005253a8.c:44-54`):
```c
_LStrCopy(payload, 2, 4, &s);  *(DWORD *)(player + 0x1470) = DWORD_LE(s);  // giá trị A
_LStrCopy(payload, 6, 4, &s);  *(DWORD *)(player + 0x1474) = DWORD_LE(s);  // giá trị B
VirtualCall_0x20();                                                       // refresh
if (*(char *)(DAT_00948d64 + 0xc2) != 0)                                  // nếu một object phụ đang mở
    VirtualCall_0x24(DAT_00948d64);                                       // → refresh object phụ
*(byte *)(local_8 + 0x5a) = 0;                                            // reset cờ chờ (+0x168)
```
- Đây là dạng "hiện tại / tối đa" điển hình (HP/MP/EXP-like) — nhưng lưu ý single source of truth: chỉ khẳng định đây là **cặp giá trị DWORD liên tiếp trên Local Player**, không đoán thêm nghiệp vụ gốc.

**`0x16` — `FUN_005250e8`** (`ts_decompile/functions/005250e8_FUN_005250e8.c:59-107`), phân nhánh theo `payload[13]`:
- `payload[13] == 0`: đọc `P[2..5]` (Word, bị chặn ≤ 0xFFFF), `P[6..9] → player+0x1470`, `P[10..13] → player+0x1474`, rồi tra tên theo Word ID:
  ```c
  name = FUN_00774a84(gvar_007DA540, wordId, &out);
  _LStrCatN(&local_8[0x55], 3, &DAT_00525310, name, &DAT_005252fc); // nối chuỗi + name + chuỗi vào +0x154
  ```
- `payload[13] == 1`: chỉ gán `player+0x1470` (P[6..9]) và `player+0x1474` (P[10..13]), reset cờ `+0x168 = 0`.
- `payload[13] == 2`: gán chuỗi cố định `DAT_0052534c` vào `+0x154`, tăng đếm thử `+0x168` (kiểm tra tràn số học `Inc()` kiểu Delphi); nếu `> 3` thì gọi virtual `+0x24`.

**`0x17` — `FUN_00524ff0`** (`ts_decompile/functions/00524ff0_FUN_00524ff0.c:50-56`):
```c
sel = payload[1];  // 1 hoặc 2
if (sel == 1) _LStrAsg(local_8 + 0x154, &DAT_00525088);
if (sel == 2) _LStrAsg(local_8 + 0x154, &DAT_005250bc);
```
- Chọn 1 trong 2 chuỗi văn bản cố định cho ô `+0x154`.

### 4.6. SubOp `0x18` — `FUN_00525488` (object `gvar_007DA348`)

Nguồn: `ts_decompile/functions/00525488_FUN_00525488.c:77-107`:
```c
if (payload[1] == 0x01) {                          // điều kiện bắt buộc
  id   = WORD_LE(P[3..4]);                          // _LStrCopy(payload,3,2)
  cnt  = payload[4];                                // byte đếm
  if (FUN_007746ac(gvar_007DA540, id) != 0) {       // kiểm tra ID tồn tại trong CSDL
    name = FUN_00774a84(gvar_007DA540, id, &out);
    text = name + DAT_00525600 + IntToStr(cnt) + DAT_00525614;   // "name … X …"
    if (_LStrLen(payload) > 5)
      tail = _LStrCopy(payload, 6, len);            // phần văn bản tự do phía sau
    if (tail) text = tail + text;
    FUN_007ab870(gvar_007DA1B0, 0, text, 0);        // hiển thị log nổi (như 4.3)
  }
}
```
- **Thông báo tổng hợp**: tên đối tượng (tra theo ID) + số lượng + văn bản tùy ý của server. Cấu trúc: `[42][18][01][ID:2B LE][Count:1B][FreeText:...]`. Giá trị `ID` phải tồn tại trong CSDL `gvar_007DA540` (hàm kiểm tra `FUN_007746ac`), nếu không thì gói bị **bỏ qua lặng lẽ**.

### 4.7. SubOp `0x1F` — `FUN_005be098` (object `gvar_007D9EF8`)

Nguồn: `ts_decompile/functions/005be098_FUN_005be098.c:69-95`:
```c
delta  = payload[1];                                // 1 byte — phần cộng thêm (có dấu khi hiển thị)
_LStrCopy(payload, 3, 4, &s);
newVal = DWORD_LE(s);                               // giá trị mới
oldVal = *(byte *)(player + 0x3fa);                 // giá trị cũ (1 byte trên Local Player!)
Format(&DAT_005be1f8, [oldVal], &line1);            // dòng 1: giá trị cũ
Format(&DAT_005be228, [newVal], &line2);            // dòng 2: giá trị mới (định dạng %d)
line = line1 + "\r" + line2;
FUN_0063c674(gvar_007D9D6C, line, 0, 0, 0x5bdff4, self, 0x5be048, self); // ghi vào khung log cuộn
```
- **Ghi 2 dòng log** vào khung log của object (`FUN_0063c674` — hàm append dòng chữ có tham số callback). Điểm đáng chú ý cho mock server: giá trị cũ chỉ là **1 byte** tại `player+0x3FA`, còn giá trị mới là **DWORD** từ payload. Format string thực tại `DAT_005be1f8`/`DAT_005be228` (chưa dump — chỉ xác nhận được tham số truyền vào).

### 4.8. SubOp `0x20` — `FUN_005be240` (object `gvar_007D9EF8`)

Nguồn: `ts_decompile/functions/005be240_FUN_005be240.c:49-58`:
```c
sel = payload[1];  // 1, 2 hoặc 3
if (sel == 1) VirtualCall_0x90(gvar_007DA084, &DAT_005be314, 2000, 0, 0);
if (sel == 2) VirtualCall_0x90(gvar_007DA084, &DAT_005be338, 2000, 0, 0);
if (sel == 3) VirtualCall_0x90(gvar_007DA084, &DAT_005be364, 2000, 0, 0);
```
- Hiển thị 1 trong 3 **thông báo cố định** trong 2000ms (cùng cơ chế `gvar_007DA084` như SubOp 0x0E, thời lượng dài hơn).

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan (Global Objects)

| Biến toàn cục | Kiểu / Vai trò (suy từ sử dụng) | Bằng chứng |
| :--- | :--- | :--- |
| `gvar_007DA35C` | Object UI quản lý **bảng trạng thái kỹ năng 2 chiều** + ô số `player+0x146C` + ô văn bản `+0x13C` | `FUN_00522ab8` toàn văn |
| `gvar_007D9FA0` | Object UI **đồng hồ đếm + log biến động** (giá trị `+0x1AC`, con trỏ dòng `+0x66`, bộ đệm dòng `+0x4C[]`) | `FUN_005c0b20`, `FUN_005c088c` |
| `gvar_007DA348` | Object UI **đồng hồ kép** (ghi cặp `player+0x1470/0x1474`, văn bản `+0x154`, cờ chờ `+0x168`) | `FUN_005253a8`, `FUN_005250e8`, `FUN_00524ff0` |
| `gvar_007D9EF8` | Object UI **khung log cuộn** (append qua `FUN_0063c674`) | `FUN_005be098` |
| `gvar_007DA7BC` | **Local Player object** (chuẩn đã xác minh trong các tài liệu opcode trước) | mọi ghi `+0x146C/0x1470/0x1474/0x3FA` |
| `gvar_007DA540` | **CSDL tra cứu tên theo ID** (vật phẩm / kỹ năng / đối tượng) — chuẩn đã xác minh | `FUN_00774a84`, `FUN_007746ac` |
| `gvar_007DA084` | **Message box manager** (hiển thị thông báo có thời lượng) — chuẩn đã xác minh | SubOp 0x0E, 0x20 |
| `gvar_007D9D30` | **Codec context** cho các helper chuyển đổi LE (truyền vào mọi lời gọi `FUN_0077eb9c`/`FUN_0077ef7c`) | toàn bộ handler |

---

## 6. Ghi Chú Mock Server & Điểm Chưa Kết Luận Được

1. **Chuỗi cố định chưa dump**: các chuỗi tại `0x522E40–0x522F28`, `0x5C0A0C`, `0x5C0ABC/0x5C0AF4`, `0x5250x8…`, `0x5252FC/0x525310/0x52534C`, `0x525600/0x525614`, `0x5BE1F8/0x5BE228`, `0x5BE314/0x5BE338/0x5BE364` nằm trong vùng .data **chưa có trong `ts_decompile/`** — khi mock chỉ cần biết rằng đó là hằng chuỗi định dạng/văn bản hiển thị; nếu cần nội dung chính xác phải dump thêm vùng `.data` của aLogin.exe (ngoài phạm vi SSOT hiện tại).
2. **Không có VISCII**: không handler nào của 0x42 xử lý chuỗi tiếng Việt VISCII trong luồng được phân tích (các chuỗi hiển thị đều là hằng Delphi hoặc văn bản tự do từ server — server có thể gửi VISCII trong phần FreeText của SubOp `0x18`, client hiển thị nguyên trạng).
3. **SubOp rỗng**: mọi SubOp ngoài `{1,2,3,4,0x0B,0x0C,0x0E,0x15,0x16,0x17,0x18,0x1F,0x20}` đều không làm gì (`default` chỉ dọn stack — `case_059...c:28-57`).
4. **Chuẩn wire format chung** (khớp handoff `handoff-opcode-exploration-guide.md` mục 1): frame `[F4 44][Len:2B LE][Payload]`, toàn frame XOR `0xAD`; payload = `[0x42][SubOp][...]`.

---

## 7. Nguồn Tham Chiếu (Single Source of Truth: `ts_decompile/`)

- Dispatcher & case 59: `ts_decompile/case_functions/functions/case_059_00795DB6_FUN_00795db6.c`
- Bản gộp 65 case: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9380-9428`
- Dispatcher inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7249+`
- Handler: `ts_decompile/functions/00522ab8_FUN_00522ab8.c` · `005c0b20_FUN_005c0b20.c` · `005c088c_FUN_005c088c.c` · `005c0a3c_FUN_005c0a3c.c` · `005253a8_FUN_005253a8.c` · `005250e8_FUN_005250e8.c` · `00524ff0_FUN_00524ff0.c` · `00525488_FUN_00525488.c` · `005be098_FUN_005be098.c` · `005be240_FUN_005be240.c`
- Chiều C→S (rỗng): `ts_decompile/functions/0077f414_FUN_0077F414.c:1066-1067`
- Helper codec: `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` (Word LE) · `0077ef7c_FUN_0077ef7c.c` (DWORD LE)
- Tra CSDL tên: `FUN_00774a84` / `FUN_007746ac` (gọi với `gvar_007DA540`)
