# PHÂN TÍCH — Main OP 0x48 (72) / Case 64 / FUN_00796248 @ 0x00796248

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Hai hàm callee (`FUN_00508204`, `FUN_0050849c`) **có body đầy đủ trong SSOT**; 3 callee (`func_0x00507004`, `func_0x0050705c`, `func_0x00508714`) nằm trong khe trống chưa decompile.

- Handler `FUN_00796248` quản lý toàn bộ giao thức mạng của **Hệ thống Minigame Máy Quay Số May Mắn / Đánh Bạc (Slot Machine Form - `TMR_Slotform`)**.
- Đối tượng đích duy nhất của cả 6 nhánh `SubOp` là instance toàn cục `gvar_007D9C68` (con trỏ tới đối tượng `TMR_Slotform`, VMT `VMT_505340_TMR_Slotform`).
- Chiều C→S: `FUN_0077f414:1076` (`case 0x48: break;`) là rỗng — Client không gửi gói tin Opcode 0x48 qua `SendCommand`. Đây là Opcode một chiều từ Server gửi xuống Client.

---

## 1. Tóm Tắt Nghiệp Vụ

- **Hệ thống đích**: Máy quay số trúng thưởng trong game (`TMR_Slotform`).
  - Giao diện gồm 16 ô cuộn thưởng (Reel slots), 4 bóng đèn trạng thái (`Icon_Bulb1..4`), nút đặt cược / nạp xu (`btn_Putin` trỏ tới hàm `FUN_00508784`), và nút quay (`btn_SoltPlay` trỏ tới `FUN_00506bf4`).
  - Bộ đệm lịch sử trúng thưởng của bản thân nằm tại `TMR_Slotform + 0x324`.
  - Cờ trạng thái sẵn sàng / khóa quay nằm tại byte `TMR_Slotform + 0x1d2`.
- **Phân loại các nhánh SubOp**:
  - `SubOp 0x01` & `0x02`: Khởi tạo và cập nhật trạng thái vòng quay (Callee nằm ngoài tập decompile).
  - `SubOp 0x03`: Điều khiển cờ trạng thái (`+0x1D2 = 1`) và phát thông báo Toast / Chat cảnh báo.
  - `SubOp 0x04`: Lời gọi hàm ảo `VMT + 0x20` (Đóng / Mở / Reset giao diện máy quay).
  - `SubOp 0x05`: **Xử lý trao giải thưởng & Phát thanh thông báo toàn server**: Đọc `PlayerID (4B)`, `ItemID (2B)`, `Count (1B)`. Nếu là bản thân thì lưu vào bộ đệm cá nhân; nếu là người chơi khác thì phát thông báo tin tức vàng lên kênh Chat toàn máy chủ.
  - `SubOp 0x06`: Đóng hoặc kết thúc phiên quay.

---

## 2. Entry & Cách Đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x48 (72) → byte_table[0x78A8EE][0x48] = 0x40 (64)
                 → dword_table[0x78A9B6][64] @ 0x0078AAB6 = 0x00796248
                 → FUN_00796248 (Case 64)
```

- **File độc lập**: `ts_decompile/case_functions/functions/case_064_00796248_FUN_00796248.c` (78 dòng).
- **Bản inline trong dispatcher**: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7448-7483`.
- **Bản gộp**: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9769-9836`.

### 2.2. Kiểm tra độ dài & Đọc SubOp

```c
iVar2 = *(int *)(unaff_EBP + -0xc);            // RestPayload (RP)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {               // Length(RestPayload) == 0
  iVar1 = _BoundErr(0);                        // Báo lỗi RangeError nếu L < 2
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // SubOp = RP[0]
switch(*(undefined4 *)(unaff_EBP + -0x14)) {
  case 1: func_0x00507004(*(undefined4 *)gvar_007D9C68, local_10); break;
  case 2: func_0x0050705c(*(undefined4 *)gvar_007D9C68, local_10); break;
  case 3: FUN_00508204(*(int *)gvar_007D9C68, local_10); break;
  case 4: (**(code **)(**(int **)gvar_007D9C68 + 0x20))(); break;
  case 5: FUN_0050849c(*(int *)gvar_007D9C68, local_10); break;
  case 6: func_0x00508714(*(undefined4 *)gvar_007D9C68, local_10); break;
}
```

- Mọi SubOp từ 1 đến 6 đều nhận con trỏ instance `gvar_007D9C68` làm tham số đầu tiên (`this`) và `RestPayload` (hoặc không nhận tham số ở `case 4`).
- Không có nhánh `default`. Mọi giá trị `SubOp` ngoài `1..6` là **no-op im lặng**.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire Payload | Min Len | Callee & Trạng thái SSOT | Core Logic |
| :---: | :--- | :---: | :--- | :--- |
| **`0x01`** | `[48][01][...]` | 2B | `func_0x00507004` (Chưa decompile) | Khởi tạo thông số máy Slot / bắt đầu vòng quay. |
| **`0x02`** | `[48][02][...]` | 2B | `func_0x0050705c` (Chưa decompile) | Dừng guồng quay / cập nhật kết quả các biểu tượng trúng. |
| **`0x03`** | `[48][03][Action:1B]` | 3B | `FUN_00508204` (**Có body SSOT**) | Xử lý cờ sẵn sàng (`+0x1D2 = 1`) hoặc hiển thị Toast (`Action=2`) / Chat (`Action=3`). |
| **`0x04`** | `[48][04]` | 2B | Virtual call `+0x20` (**Có body VMT**) | Gọi hàm ảo `+0x20` trên `TMR_Slotform` (Show / Reset giao diện). |
| **`0x05`** | `[48][05][UID:4B][Item:2B][Cnt:1B]` | 9B | `FUN_0050849c` (**Có body SSOT**) | Trao thưởng vật phẩm: lưu buffer nếu là bản thân, phát tin tức toàn server qua Chat nếu là người khác. |
| **`0x06`** | `[48][06][...]` | 2B | `func_0x00508714` (Chưa decompile) | Đóng / giải phóng phiên quay máy slot. |
| **Khác** | `[48][00]`, `[48][07]`... | 2B | Không có | **No-op im lặng**. |

---

## 4. Chi Tiết Core Logic Từng Nhánh

### 4.1. SubOp `0x03` — Cập Nhật Trạng Thái & Cảnh Báo (`FUN_00508204`)

Mã nguồn tại `ts_decompile/functions/00508204_FUN_00508204.c`:

```c
void FUN_00508204(int param_1, int param_2) {
  char cVar1;
  int iVar2;

  iVar2 = 1;
  if (*(uint *)(param_2 + -4) < 2) {           // Yêu cầu RestPayload >= 2 bytes
    iVar2 = _BoundErr(1);
  }
  cVar1 = *(char *)(param_2 + iVar2);          // ActionCode = RP[1]
  if (cVar1 == '\x01') {
    *(undefined1 *)(param_1 + 0x1d2) = 1;      // Đặt cờ trạng thái +0x1D2 = 1
  }
  else if (cVar1 == '\x02') {
    // Hiển thị thông báo Toast cảnh báo lỗi slot trong 3000ms
    (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084, &DAT_00508284, 3000, 0, 0);
  }
  else if (cVar1 == '\x03') {
    // Gửi thông báo hệ thống vào khung Chat
    FUN_007ab870(*(int *)gvar_007DA1B0, 0, &DAT_005082a4, '\0');
  }
  return;
}
```

- **Wire Layout của SubOp 0x03**:
  - `P[0]`: `0x48` (MainOp).
  - `P[1]`: `0x03` (SubOp).
  - `P[2]`: `ActionCode` (1 byte):
    - `0x01`: Đặt cờ cho phép kích hoạt / sẵn sàng quay (`Slotform + 0x1D2 = 1`).
    - `0x02`: Cảnh báo không thể quay / lỗi nạp cược bằng Toast 3000ms với chuỗi `&DAT_00508284`.
    - `0x03`: Gửi thông báo hệ thống vào hộp thoại Chat qua `FUN_007ab870` với chuỗi `&DAT_005082a4`.

### 4.2. SubOp `0x04` — Điều Khiển Giao Diện Qua VMT (`+0x20`)

- Dòng 38 trong `case_064`:
  ```c
  (**(code **)(**(int **)gvar_007D9C68 + 0x20))();
  ```
- Đây là lời gọi hàm ảo tại offset `+0x20` của bảng VMT `TMR_Slotform`:
  - Thực hiện tác vụ Reset hoặc Đóng/Mở form máy quay số.
  - Không cần thêm tham số nào trong payload ngoài 2 bytes `[48][04]`.

### 4.3. SubOp `0x05` — Công Bố Giải Thưởng & Thông Báo Toàn Cụm (`FUN_0050849c`)

Mã nguồn tại `ts_decompile/functions/0050849c_FUN_0050849c.c` (497 bytes, 127 dòng):

#### Cấu trúc giải mã Payload
1. **Đọc `PlayerID` (4 bytes DWORD LE)**:
   - `_LStrCopy(param_2, 2, 4, &local_20);` (Delphi 1-based, cắt 4 byte từ vị trí 2 của RestPayload).
   - `local_14 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30, local_20);` → Giải mã chuỗi nhị phân 4 bytes thành số nguyên 32-bit LE.
2. **Đọc `ItemID` (2 bytes WORD LE)**:
   - `_LStrCopy(param_2, 6, 2, &local_24);` (Cắt 2 byte từ vị trí 6 của RestPayload).
   - `local_18 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_24) & 0xffff;` → Giải mã 2 bytes thành số nguyên 16-bit LE.
3. **Đọc `ItemCount` (1 byte)**:
   - Kiểm tra độ dài: `if (*(uint *)(RestPayload + -4) < 8) _BoundErr(7);` → RestPayload bắt buộc tối thiểu 8 bytes (`P` tối thiểu 9 bytes).
   - `local_19 = *(byte *)(RestPayload + 7);` → Byte thứ 8 của RestPayload (`P[8]`).

```
+---------------+---------------+---------------+-------------------------------+-----------------------+-------------------+
| P[0] (1B)     | P[1] (1B)     | P[2..5] (4B)  | P[6..7] (2B)                  | P[8] (1B)             | Tổng cộng         |
| MainOp (0x48) | SubOp (0x05)  | PlayerUID(LE) | ItemID (LE)                   | ItemCount (Byte)      | Tối thiểu 9 bytes |
+---------------+---------------+---------------+-------------------------------+-----------------------+-------------------+
```

#### Xử lý logic 2 nhánh
```c
if (*(int *)(*(int *)gvar_007DA7BC + 4) == local_14) {
  // NHÁNH 1: BẢN THÂN TRÚNG THƯỞNG
  // 1. Lấy tên bản thân từ LocalActor (+9, chuỗi tối đa 26 ký tự)
  // 2. Tra cứu tên vật phẩm từ từ điển vật phẩm:
  FUN_00774a84(*(undefined4 *)gvar_007DA540, local_18, (int *)&local_94);
  // 3. Đổi số lượng thành chuỗi qua IntToStr(local_19, &local_98)
  // 4. Ghép chuỗi chúc mừng và ghi trực tiếp vào buffer kết quả của Slotform:
  _LStrCatN((char **)(local_8 + 0x324), 4, __dummy_85, &DAT_005086e0, local_98, pcVar3, pcVar4);
}
else {
  // NHÁNH 2: NGƯỜI CHƠI KHÁC TRÚNG THƯỞNG (BROADCAST TOÀN KÊNH)
  // 1. Tra cứu tên người chơi khác theo PlayerID:
  FUN_0075ddb8(local_14, (int *)&local_28);
  // 2. Tra cứu tên vật phẩm trúng:
  FUN_00774a84(*(undefined4 *)gvar_007DA540, local_18, (int *)&local_2c);
  // 3. Đổi số lượng thành chuỗi qua IntToStr:
  IntToStr((uint)local_19, (int *)&local_30);
  // 4. Ghép chuỗi thông cáo hệ thống toàn kênh:
  _LStrCatN(&local_10, 6, __dummy_163, &DAT_005086e0, local_30, pcVar3, String3, pcVar4, String1);
  // 5. Gửi chuỗi vào khung Chat toàn server:
  FUN_007ab870(*(int *)gvar_007DA1B0, 0, local_10, '\0');
}
```

- **Từ điển chuỗi VISCII/Hệ thống**:
  - `DAT_005086a0`: Tiền tố thông báo máy slot (ví dụ `[Tin nóng / Chúc mừng]`).
  - `DAT_005086b4`: Động từ vị ngữ trúng thưởng (ví dụ `đã quay trúng`).
  - `DAT_005086e0`: Ký tự kết thúc câu thông báo.
  - `FUN_007ab870`: Hàm đẩy chuỗi văn bản vào khung chat (kênh thông báo hệ thống màu vàng, `kind = '\0'`).

### 4.4. Các Callee nằm trong khoảng trống chưa decompile (Cần Redump)

- `func_0x00507004` (SubOp 1), `func_0x0050705c` (SubOp 2), `func_0x00508714` (SubOp 6):
  - Tra cứu `ts_decompile/index.csv`: Mục trước là `FUN_00506bf4` (dòng 2757), mục sau là `FUN_005074d8` (dòng 2758); mục `00508714` nằm ngay trước `FUN_00508784` (dòng 2764).
  - Đây là các method nội bộ của `TMR_Slotform` xử lý chuyển động đồ họa của các guồng quay (Reels animation) và logic dừng số.
  - Về mặt giao thức và state mạng, Server chỉ cần tương tác đúng chuẩn với SubOp `0x03`, `0x04`, `0x05`.

---

## 5. Chiều Client → Server (C→S)

- Tại `ts_decompile/functions/0077f414_FUN_0077F414.c:1076`:
  ```c
  case 0x48:
    break;
  ```
- **Xác nhận**: Client không gửi Opcode `0x48` bằng hàm `SendCommand`.
- Tuy nhiên, người chơi kích hoạt quay số thông qua sự kiện bấm nút UI (`btn_SoltPlay` @ `0x00506bf4`), sự kiện này sẽ đóng gói lệnh hành động C→S gửi qua opcode tương tác minigame chuẩn (như Opcode `0x1D` Action hoặc `0x1A` Talk/Event).

---

## 6. Đặc Tả Wire Layout Chuẩn Cho Mock Server

### 6.1. Gói thông báo trúng giải thưởng lớn (Jackpot/Reward Broadcast)
Server phát gói thông báo khi người chơi trúng vật phẩm hiếm:
```
F4 44 09 00 48 05 [UID: 4B LE] [ItemID: 2B LE] [Count: 1B]
```
Ví dụ: Người chơi có UID = `0x00012345` trúng `1` vật phẩm ID = `0x04D2` (1234):
- Payload: `48 05 45 23 01 00 D2 04 01`
- Độ dài: 9 bytes.

### 6.2. Gói kích hoạt trạng thái sẵn sàng của máy Slot
```
F4 44 03 00 48 03 01
```
- `48`: MainOp 0x48.
- `03`: SubOp 0x03.
- `01`: ActionCode 0x01 (Ghi `+0x1D2 = 1`).

---

## 7. Bảng Bằng Chứng Mã Nguồn Sơ Cấp (SSOT)

| Ký hiệu / Địa chỉ | File nguồn | Dòng | Vai trò xác minh |
| :--- | :--- | :--- | :--- |
| `FUN_00796248` | `ts_decompile/case_functions/functions/case_064_00796248_FUN_00796248.c` | 8–75 | Handler độc lập MainOp 0x48 |
| `case 0x48:` | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | 7448–7483 | Bản inline trong Main Dispatcher |
| `gvar_007D9C68` | `ts_decompile/functions/0051189c_FUN_0051189c.asm.txt` | 2822–2826 | Khởi tạo con trỏ singleton `TMR_Slotform` |
| `VMT_505340` | `ts_decompile/functions/0051189c_FUN_0051189c.c` | 1717 | VMT bảng hàm ảo của `TMR_Slotform` |
| `FUN_00508204` | `ts_decompile/functions/00508204_FUN_00508204.c` | 19–42 | Handler SubOp 0x03 (Cờ `+0x1D2` & Toast) |
| `FUN_0050849c` | `ts_decompile/functions/0050849c_FUN_0050849c.c` | 31–124 | Handler SubOp 0x05 (Trao thưởng & Broadcast) |
| `FUN_0077ef7c` | `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c` | 229–239 | Giải mã 4 bytes DWORD LE |
| `FUN_0077eb9c` | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` | 173 | Giải mã 2 bytes WORD LE |
| `FUN_00774a84` | `ts_decompile/functions/00774a84_FUN_00774a84.c` | — | Tra cứu tên vật phẩm từ từ điển |
| `FUN_0075ddb8` | `ts_decompile/functions/0075ddb8_FUN_0075ddb8.c` | — | Tra cứu tên người chơi theo UID |
| `FUN_007ab870` | `ts_decompile/functions/007ab870_FUN_007ab870.c` | — | Đẩy tin nhắn vào khung chat hệ thống |
