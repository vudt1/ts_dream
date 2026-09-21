# HANDOFF — Phân tích chi tiết Main OP 0x00 & 0x01 (aLogin.exe)

Ngày: 2026-09-10 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code`  
Trạng thái: **Đã xác minh 100% từ decompile jump-table và case functions (sửa đổi và hoàn thiện các phỏng đoán trong handoff trước)**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). Riêng bảng chuỗi OP 0x00 (`0x796418…0x796d38`): **3/53 chuỗi đã có dump và được giải mã** (`0x7967f8`, `0x796a8c`, `0x796ae8` — xem ghi chú dưới bảng); **50 chuỗi còn lại — bao gồm `0x796d10` (SubOp 0x38) và `0x796d38` (default) — VẪN CHƯA CÓ DUMP** (`ls ts_decompile/redump/` không có `lit_796d10/796d38.hex`), giữ nguyên placeholder, KHÔNG đặt tên suy đoán.

---

## 1. Bối cảnh & Đính chính từ Handoff trước

Trong tài liệu handoff trước (`handoff-2026-09-10-protocol-analysis.md`), hàm phân phối `FUN_0078a89c` bị timeout khi decompile bằng Ghidra, dẫn đến việc:
1. Gom dải địa chỉ `0x78ACD6..0x78B686` thành một cụm duy nhất và gán nhãn là `OP_SYSTEM = 0x00`.
2. Dựa vào các điểm gọi hàm con (`call-sites`) rời rạc trong dải đó để suy đoán 11 sub-op (S1..S11).

**Sự thật sau khi redump và trích xuất Jump Table:**
* Bảng byte `0x78A8EE` (`jumptable_byte200_0x78A8EE.hex`) và bảng dword `0x78A9B6` (`jumptable_dword200_0x78A9B6.hex`) xác nhận:
  * **`Main OP 0x00`** -> `byte_table[0x00] = 0x01` -> Jump target `0x78A9BA` = **`FUN_0078aabe`** (`0x0078AABE`, Case 1). Đây là **Hộp thoại thông báo lỗi / Ngắt kết nối hệ thống (System Error Notice)**.
  * **`Main OP 0x01`** -> `byte_table[0x01] = 0x02` -> Jump target `0x78A9BE` = **`FUN_0078b149`** (`0x0078B149`, Case 2). Đây mới chính là cụm nghiệp vụ mà handoff cũ đã phân tích (Thoát map, chuyển Scene, thông báo Toast, đếm ngược).
  * Trong `FUN_0078b149`, **S1, S2, S3, S4, S5, S6 không phải là 6 sub-op độc lập**, mà là một chuỗi xử lý tuần tự nằm trọn vẹn bên trong **`case 1:` duy nhất** (xử lý khi một người chơi biến mất / rời mạng).
  * Cấu trúc switch thực tế của `FUN_0078b149` bao gồm: `case 1`, `case 3`, `case 5`, `case 6`, `case 7`, `case 8`, `case 9`, `case 10`, `case 0x0B`.

---

## 2. Kiến trúc đọc PacketBuffer (S → C)

### 2.1. Tầng Nhận & Khử Khung (Network Framing Layer)
File: `ts_decompile/functions/0050cd6c_TForm1.ClientSocket1Read.c`
1. Dữ liệu TCP thô từ `TCustomWinSocket.ReceiveText` được giải mã XOR toàn bộ với khóa `0xAD` qua `FUN_0050a248`.
2. Chuỗi sau XOR được nối dồn vào buffer tích lũy tại `*(DAT_009264a4 + 8)`.
3. Vòng lặp deframe kiểm tra:
   * Header tối thiểu: `>= 4 bytes`.
   * **Token (2 bytes)**: Cắt `_LStrCopy(..., 1, 2)` so khớp với hằng số `token_recv.hex` (`0x44, 0xF4`). Nếu sai token, client xóa 2 bytes (`_LStrDelete(..., 1, 2)`) để đồng bộ lại.
   * **Length (2 bytes Little-Endian)**: Cắt `_LStrCopy(..., 3, 2)` và giải mã qua `FUN_0077eb9c` (`L = byte0 + byte1 * 256`).
   * **Payload (L bytes)**: Khi buffer tích lũy đủ `L + 4` bytes, client cắt đúng `L` bytes từ vị trí thứ 5 (1-based), xóa frame khỏi đệm tích lũy, và đưa payload vào hàng đợi nhận:
     `TForm1_CY_AddRevQueue` (`ts_decompile/functions/00516108_TForm1.CY_AddRevQueue.c`) -> thêm vào `TStringList DAT_00926e88`.

### 2.2. Tầng Bơm Gói Tin (Tick Pump Layer)
File: `ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c`
1. Chạy định kỳ trên game-tick (~30ms, timer).
2. Kiểm tra cờ tạm dừng: `*(char*)(DAT_009264a4 + 0xC) == 0`.
3. Pop từng payload string `local_c` từ `DAT_00926e88` (tối đa 50 gói/tick):
   * **`Main OP`**: Lấy ký tự đầu tiên `local_c[0]` (1-based index 1), đưa vào thanh ghi `DL`.
   * **`RestPayload`**: Cắt chuỗi từ ký tự thứ 2 trở đi `_LStrCopy(local_c, 2, len - 1)` (1-based), đưa vào thanh ghi `ECX`.
   * Gọi `FUN_0078a89c(EAX=TFConnect, DL=MainOp, ECX=RestPayload)`.

### 2.3. Tầng Điều Phối (Dispatcher Layer)
File: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`
* Tra bảng:
  ```asm
  MOV AL, byte ptr [EAX + 0x78a8ee]
  JMP dword ptr [EAX*0x4 + 0x78a9b6]
  ```
* Mapping:
  * `MainOp = 0x00` -> `AL = 0x01` -> Jump `0x0078AABE` (`FUN_0078aabe`)
  * `MainOp = 0x01` -> `AL = 0x02` -> Jump `0x0078B149` (`FUN_0078b149`)

---

## 3. Chi tiết Main OP 0x00: System Error / Disconnect Dialog

* **Entry**: `FUN_0078aabe` (`ts_decompile/case_functions/functions/case_001_0078AABE_FUN_0078aabe.c`) @ `0x0078AABE`
* **Mục đích**: Hiển thị hộp thoại lỗi hệ thống / thông báo từ server và chủ động đưa client vào trạng thái ngắt kết nối.
* **Cấu trúc gói tin**: Đúng 2 bytes payload:
  `[MainOp: 0x00] [SubOp: 1 byte (0x01..0x38)]`
* **Cách đọc PacketBuffer**:
  * Đọc 1 byte đầu tiên của `RestPayload` (`ECX[0]`):
    ```c
    *(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar3 + 0); // SubOp
    ```
  * Không đọc thêm bất kỳ byte nào từ buffer.

### Bảng tra cứu Sub Codes Main OP 0x00:
Lệnh `switch(SubOp)` tra cứu chuỗi thông báo tĩnh trong bộ nhớ:

| SubOp (Hex) | SubOp (Dec) | Địa chỉ chuỗi thông báo | Ghi chú nghiệp vụ |
| :---: | :---: | :---: | :--- |
| `0x01` | 1 | `0x796418` | Chuỗi lỗi / thông báo hệ thống |
| `0x02` | 2 | `0x796444` | Chuỗi lỗi / thông báo hệ thống |
| `0x03` | 3 | `0x796470` | Chuỗi lỗi / thông báo hệ thống |
| `0x04` | 4 | `0x79648c` | Chuỗi lỗi / thông báo hệ thống |
| `0x05` | 5 | `0x7964b0` | Chuỗi lỗi / thông báo hệ thống |
| `0x06` | 6 | `0x7964e8` | Chuỗi lỗi / thông báo hệ thống |
| `0x07` | 7 | `0x796520` | Chuỗi lỗi / thông báo hệ thống |
| `0x08..0x09` | 8..9 | `0x796554` | Cả 2 sub-op trỏ chung 1 chuỗi |
| `0x0A` | 10 | `0x79657c` | Chuỗi lỗi / thông báo hệ thống |
| `0x0B` | 11 | `0x7965ac` | Chuỗi lỗi / thông báo hệ thống |
| `0x0C` | 12 | `0x7965d8` | Chuỗi lỗi / thông báo hệ thống |
| `0x0D` | 13 | `0x796604` | Chuỗi lỗi / thông báo hệ thống |
| `0x0E` | 14 | `0x796618` | Chuỗi lỗi / thông báo hệ thống |
| `0x0F` | 15 | `0x796650` | Chuỗi lỗi / thông báo hệ thống |
| `0x10` | 16 | `0x796684` | Chuỗi lỗi / thông báo hệ thống |
| `0x11` | 17 | `0x7966b8` | Layout nút đặc biệt (bật nút +0x13c) |
| `0x12` | 18 | `0x7966f0` | Chuỗi lỗi / thông báo hệ thống |
| `0x13` | 19 | `0x796718` | Chuỗi lỗi / thông báo hệ thống |
| `0x14` | 20 | `0x796744` | Chuỗi lỗi / thông báo hệ thống |
| `0x15` | 21 | `0x796770` | Chuỗi lỗi / thông báo hệ thống |
| `0x16` | 22 | `0x7967a4` | Chuỗi lỗi / thông báo hệ thống |
| `0x17` | 23 | `0x7967d8` | Chuỗi lỗi / thông báo hệ thống |
| `0x18` | 24 | `0x7967f8` | **ĐÃ GIẢI MÃ** (`lit_7967f8.hex`, first-str 30B, NUL `0x796816`): VISCII→NFC (đính chính recipe — xem `opcode_09.md §7.1`) = **`Mật khẩu quá ngắn, mất kết nối`** |
| `0x19` | 25 | `0x796820` | Chuỗi lỗi / thông báo hệ thống |
| `0x1A` | 26 | `0x796844` | Chuỗi lỗi / thông báo hệ thống |
| `0x1B` | 27 | `0x79686c` | Chuỗi lỗi / thông báo hệ thống |
| `0x1C` | 28 | `0x796894` | Chuỗi lỗi / thông báo hệ thống |
| `0x1D` | 29 | `0x7968b4` | Chuỗi lỗi / thông báo hệ thống |
| `0x1E` | 30 | `0x7968d0` | Chuỗi lỗi / thông báo hệ thống |
| `0x1F` | 31 | `0x7968f4` | Chuỗi lỗi / thông báo hệ thống |
| `0x20` | 32 | `0x796918` | Chuỗi lỗi / thông báo hệ thống |
| `0x21` | 33 | `0x796938` | Chuỗi lỗi / thông báo hệ thống |
| `0x22` | 34 | `0x796974` | Chuỗi lỗi / thông báo hệ thống |
| `0x23` | 35 | `0x796998` | Chuỗi lỗi / thông báo hệ thống |
| `0x24` | 36 | `0x7969b4` | Chuỗi lỗi / thông báo hệ thống |
| `0x25` | 37 | `0x7969d8` | Chuỗi lỗi / thông báo hệ thống |
| `0x26` | 38 | `0x796a04` | Chuỗi lỗi / thông báo hệ thống |
| `0x28` | 40 | `0x796a3c` | Chuỗi lỗi / thông báo hệ thống |
| `0x29` | 41 | `0x796a68` | Chuỗi lỗi / thông báo hệ thống |
| `0x2A` | 42 | `0x796a8c` | **ĐÃ GIẢI MÃ** (`lit_796a8c.hex`, first-str 25B, NUL `0x796AA5`): raw `SØa đ±i tß li®u chiªn đ¤u` ≈ **`Sửa đổi thông tin chiến đấu`** |
| `0x2B` | 43 | `0x796ab0` | Chuỗi lỗi / thông báo hệ thống |
| `0x2C` | 44 | `0x796ae8` | **ĐÃ GIẢI MÃ** (`lit_796ae8.hex`, first-str 42B): decode VISCII: **`Sự kiện và quang cảnh xẩy ra không phù hợp`** (chốt — văn nguyên game) |
| `0x2D` | 45 | `0x796b1c` | Chuỗi lỗi / thông báo hệ thống |
| `0x2E` | 46 | `0x796b5c` | Chuỗi lỗi / thông báo hệ thống |
| `0x2F` | 47 | `0x796b8c` | Chuỗi lỗi / thông báo hệ thống |
| `0x30` | 48 | `0x796bc4` | Chuỗi lỗi / thông báo hệ thống |
| `0x31` | 49 | `0x796bf0` | Chuỗi lỗi / thông báo hệ thống |
| `0x32` | 50 | `0x796c20` | Chuỗi lỗi / thông báo hệ thống |
| `0x33..0x34` | 51..52 | `0x796c50` | Cả 2 sub-op trỏ chung 1 chuỗi |
| `0x35` | 53 | `0x796c88` | Chuỗi lỗi / thông báo hệ thống |
| `0x36` | 54 | `0x796cac` | Chuỗi lỗi / thông báo hệ thống |
| `0x37` | 55 | `0x796cd8` | Chuỗi lỗi / thông báo hệ thống |
| `0x38` | 56 | `0x796d10` | Chuỗi lỗi / thông báo hệ thống — **chưa dump** (tái xác nhận 2026-09-14: không có `lit_796d10.hex`) |
| `default` | - | `0x796d38` | Mã lỗi không xác định, gán SubOp = `0xFF` — **chuỗi default chưa dump** (không có `lit_796d38.hex`) |

* **Hành vi sau giải mã**:
  1. Nối chuỗi thông báo với mã số lỗi `IntToStr(SubOp)`.
  2. Cập nhật nhãn text của hộp thoại: `FUN_007b372c(*(gvar_007DA258 + 0x140), text)`.
  3. Cấu hình nút bấm (ẩn/hiện) dựa trên bitmask tại `gvar_00796D7C`.
  4. Hiển thị hộp thoại modal: `(**gvar_007DA258 + 0x20)()`.
  5. Gửi gói tin ACK ngược về server: `FUN_0077f414(TFConnect, 0)` (gửi frame rỗng `TOKEN + 00 00`).
  6. Ngắt kết nối socket client: `FUN_0050f2f8(*(gvar_007DA664), 1)`.

---

## 4. Chi tiết Main OP 0x01: System State / Despawn / Scene Switch

* **Entry**: `FUN_0078b149` (`ts_decompile/case_functions/functions/case_002_0078B149_FUN_0078b149.c`) @ `0x0078B149`
* **Mục đích**: Đồng bộ trạng thái người chơi, cảnh game, đăng nhập tự động, thông báo toast.
* **Cách đọc PacketBuffer**:
  * Byte `ECX[0]` (tức `payload[1]`) là `SubOp`.
  * Thực hiện lệnh `switch(SubOp)` theo danh sách chuẩn sau:

### 4.1. `SubOp = 0x01` — Player Despawn / Logout (Rời cảnh / Thoát game)
* **Wire Layout**: `[01] [01] [charID: 4 bytes Little-Endian]` (6 bytes payload).
* **Đọc PacketBuffer**:
  * `_LStrCopy(ECX, 2, 4, &temp)` cắt 4 bytes `payload[2..5]`.
  * `FUN_0077ef7c` chuyển thành số nguyên `DWORD charID`.
* **Xử lý client**:
  * Tìm tên nhân vật trong cache 2100 slot qua `FUN_00722508`.
  * Nếu đang mở kênh chat riêng với người này, gửi thông báo hệ thống và đóng kênh.
  * Xóa khỏi cache tên qua `FUN_007223f8`.
  * Xóa khỏi danh sách Party (`DAT_009CE154`) nếu có qua `FUN_00760a88` -> `FUN_0075eec0`.
  * Xóa khỏi danh sách Quân đoàn nếu có qua `FUN_00764844` -> `FUN_0056b124`.
  * Xóa khỏi danh sách Hảo hữu nếu có qua `FUN_00758318` -> `FUN_00570ef4`.
  * Tìm actor trong danh sách 800 actor trên map qua `FUN_0070c20c`:
    * Hủy liên kết thú cưỡi/xe/đồng hành qua `FUN_007a1964`.
    * Giảm số actor `*(gvar_007D9D34 + 0x5c) -= 1`.
    * Yêu cầu vẽ lại cảnh qua `FUN_0072a054`.
    * Gọi `TObject_Free` giải phóng đối tượng actor khỏi RAM.

### 4.2. `SubOp = 0x03` — Cập nhật cờ hệ thống & Bộ đếm
* **Wire Layout**: `[01] [03] [value: 1 byte]` (3 bytes payload).
* **Đọc PacketBuffer**: Đọc byte tại `ECX[1]` (tức `payload[2]`).
* **Xử lý client**: Ghi byte vào `*(gvar_007DA4A8 + 0x1eb)`, gọi hàm hiển thị VMT `+0x20` và tăng biến đếm `*(gvar_007DA6F8 + 0x18)` lên 1.

### 4.3. `SubOp = 0x05, 0x06, 0x07` — Thông báo Toast lỗi đăng nhập (2000ms)
* **Wire Layout**: `[01] [05]`, `[01] [06]`, `[01] [07]` (2 bytes payload, không có tham số phụ).
* **Đọc PacketBuffer**: Không đọc thêm byte nào.
* **Xử lý client**:
  * Mở form đăng nhập `gvar_007D9E7C` (VMT `+0x20`).
  * Xóa trắng nội dung 2 ô nhập tài khoản (`+0x13c`) và mật khẩu (`+0x140`) qua `FUN_007b372c`.
  * Hiển thị thanh thông báo chữ chạy Toast trong 2 giây (2000ms) qua `(**gvar_007DA084 + 0x90)(..., 2000, 0, 0)`:
    * `0x05`: Chuỗi lỗi `DAT_00796dbc`.
    * `0x06`: Chuỗi lỗi `DAT_00796e08`.
    * `0x07`: Chuỗi lỗi `DAT_00796e28`.

### 4.4. `SubOp = 0x08` — Bộ đếm thời gian / Giá trị số
* **Wire Layout**: `[01] [08] [value: 4 bytes Little-Endian]` (6 bytes payload).
* **Đọc PacketBuffer**:
  * Cắt 4 bytes `payload[2..5]` qua `_LStrCopy(ECX, 2, 4, &out)`.
  * Chuyển thành DWORD qua `FUN_0077ef7c`.
* **Xử lý client**:
  * Làm tròn số qua `FUN_00424e00` (`Round`).
  * Format `IntToStr` và gắn vào caption của `*(gvar_007DA258 + 0x140)`.
  * Bật hiển thị hộp thoại `gvar_007DA258`.

### 4.5. `SubOp = 0x09` — Chuyển cảnh & Tự động Đăng nhập (Scene Switch / Auto-Login)
* **Wire Layout**: `[01] [09] [sceneMode: 1 byte]` (3 bytes payload).
* **Đọc PacketBuffer**:
  * Gọi vào `FUN_0051aae0`.
  * Đọc 1 byte tại vị trí `payload[2]` gán vào `DAT_00926fc6` (`sceneMode`).
* **Xử lý client**:
  * Kiểm tra qua `FUN_00504c9c`: Điều kiện `(sceneMode % 100) + 0xA6 < 10` tương đương **`(sceneMode % 100) ∈ [90..99]`** (màn hình đăng nhập).
  * **Nếu là cảnh đăng nhập (90..99)**:
    * Lấy tài khoản lưu tại `DAT_0092530c + 0x644` điền vào form `gvar_007D9E7C + 0x13c`.
    * Lấy mật khẩu lưu tại `DAT_0092530c + 0x648` điền vào form `gvar_007D9E7C + 0x140`.
    * Tự động gọi lệnh submit đăng nhập: `FUN_007095c0` (Auto-submit Login).
  * **Nếu không phải cảnh đăng nhập**: Mở form đăng nhập hoặc nếu `sceneMode == 0x14` (20) thì repaint màn hình game.

### 4.6. `SubOp = 0x0A` — Hộp thoại thông báo tĩnh
* **Wire Layout**: `[01] [0A]` (2 bytes payload).
* **Đọc PacketBuffer**: Không đọc thêm byte.
* **Xử lý client**: Đặt nhãn hộp thoại thành chuỗi tĩnh `DAT_00796d10` và mở hộp thoại `gvar_007DA258`.

### 4.7. `SubOp = 0x0B` — Kích hoạt Form
* **Wire Layout**: `[01] [0B]` (2 bytes payload).
* **Đọc PacketBuffer**: Không đọc thêm byte.
* **Xử lý client**: Đặt cờ `*(gvar_007DA37C + 5) = 1` và gọi hiển thị VMT `+0x20` của form `gvar_007D9F58`.

---

## 5. Chiều ngược lại Client → Server (C → S) tương ứng

Trong `ts_decompile/functions/0077f414_FUN_0077F414.c`:
* **`case 0` (Gửi phản hồi ACK / Handshake ping)**:
  * Client gọi `TForm1_CY_AddSedQueue(..., "")`: gửi payload rỗng `""`.
  * Trên wire: Chỉ gồm `[TOKEN: 2B] [Length: 00 00]`.
* **`case 1` (Gói tin xác thực Login Auth)**:
  * Wire format: `[0x01] [lenPw: 1B] [charID: 4B LE] [pw[0..1]: 2B] [0x00BC: 2B LE] [pw_full: string]`.

---

## 6. Suggested Skills cho Agent kế tiếp

* `research`: Dùng để tiếp tục điều tra các Main OP kế tiếp từ `ts_decompile/case_functions/` (ví dụ: Case 3 ứng với OP 0x02, Case 4 ứng với OP 0x03, Case 9 ứng với OP 0x08 Stats, Case 21 ứng với OP 0x14, Case 27 ứng với OP 0x1A Talk).
* `to-spec`: Tổng hợp các phát hiện thành đặc tả Mock Server hoàn chỉnh tại `.scratch/op-code/spec.md`.
* `wayfinder`: Quản lý lộ trình phân tích các OP còn lại theo định dạng ticket `issues/NN-<slug>.md`.

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x0078AABE` — C `client_pseudo_c/case_001_0078AABE_FUN_0078aabe.c` (có); asm `client_pseudo_c/0078aabe_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.
- Handler `0x0078B149` — C `client_pseudo_c/case_002_0078B149_FUN_0078b149.c` (có); asm `client_pseudo_c/0078b149_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x00` — 59 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `0` | `0x0078AF97` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `0` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `1` | `0x0078ABD1` | Dữ liệu quá nhiều bị mất kết nối | Dữ liệu quá lớn; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `2` | `0x0078ABE3` | Trả lời sai 3 lần sẽ bị mất kết nối | Trả lời sai 3 lần; kết nối sẽ bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `3` | `0x0078ABF5` | Đăng nhập sai 3 lần | Đăng nhập sai 3 lần. | Hiệu đính ngữ nghĩa/câu chữ |
| `4` | `0x0078AC07` | vì Server gặp trục trặc | Do máy chủ gặp sự cố. | Hiệu đính ngữ nghĩa/câu chữ |
| `5` | `0x0078AC19` | Phát hiện sự kiện phạm luật bị mất kết nối 1 | Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `6` | `0x0078AC2B` | Phát hiện sự kiện phạm luật bị mất kết nối 2 | Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `7` | `0x0078AC3D` | Tra không ra sự kiện phải làm, mất kết nối | Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `8` | `0x0078AC4F` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `9` | `0x0078AC61` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `10` | `0x0078AC73` | mất kết nối do version không phù hợp | Kết nối bị ngắt do phiên bản không tương thích. | Hiệu đính ngữ nghĩa/câu chữ |
| `11` | `0x0078AC85` | Kết nối gặp trục trặc, mất kết nối | Kết nối gặp sự cố; đã ngắt kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `12` | `0x0078AC97` | Sử dụng chương trình bất hợp pháp | Phát hiện sử dụng chương trình không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `13` | `0x0078ACA9` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `14` | `0x0078ACBB` | Mất kết nối do sử dụng thêm chương trình khác | Kết nối bị ngắt do phát hiện chương trình bên thứ ba. | Hiệu đính ngữ nghĩa/câu chữ |
| `15` | `0x0078ACCD` | Loại bỏ thành công, xin khởi động lại máy | Đã gỡ bỏ thành công. Vui lòng khởi động lại máy. | Hiệu đính ngữ nghĩa/câu chữ |
| `16` | `0x0078ACDF` | mất kết nối do đăng nhập IP bất hợp pháp | Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `17` | `0x0078ACF1` | Version không phù hợp, xin cập nhật version mới | Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới. | Hiệu đính ngữ nghĩa/câu chữ |
| `18` | `0x0078AD03` | Dữ liệu thay đổi,mất kết nối | Dữ liệu đã thay đổi; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `19` | `0x0078AD15` | Mất kết nối do có sự đăng nhập khác | Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác. | Hiệu đính ngữ nghĩa/câu chữ |
| `20` | `0x0078AD27` | Hệ thống gặp trục trặc bất thường | Hệ thống gặp sự cố bất thường. | Hiệu đính ngữ nghĩa/câu chữ |
| `21` | `0x0078AD39` | Gặp trục trặc với việc lưu trữ, mất kết nối | Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `22` | `0x0078AD4B` | Mất kết nối do dạng dữ liệu không phù hợp | Kết nối bị ngắt do định dạng dữ liệu không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `23` | `0x0078AD5D` | Đổi tên, mất kết nối | Lỗi đổi tên; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `24` | `0x0078AD6F` | Mật khẩu quá ngắn, mất kết nối | Mật khẩu quá ngắn; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `25` | `0x0078AD81` | Trùng lập tên, mất kết nối | Tên đã tồn tại; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `26` | `0x0078AD93` | Sự kiện phạm luật, mất kết nối | Phát hiện sự kiện vi phạm; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `27` | `0x0078ADA5` | Mất kết nối do đăng nhập sai | Kết nối bị ngắt do thông tin đăng nhập không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `28` | `0x0078ADB7` | Phòng vệ mất kết nối | Kết nối bị ngắt bởi cơ chế bảo vệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `29` | `0x0078ADC9` | Dữ liệu quá nhiều | Dữ liệu quá lớn. | Hiệu đính ngữ nghĩa/câu chữ |
| `30` | `0x0078ADDB` | Khóa tài khoản, mất kết nối | Tài khoản đã bị khóa; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `31` | `0x0078ADED` | Không thể sử dụng ID này | Không thể sử dụng ID này. | Hiệu đính ngữ nghĩa/câu chữ |
| `32` | `0x0078ADFF` | Cảnh chiến đấu bị lỗi | Khung cảnh chiến đấu xảy ra lỗi. | Hiệu đính ngữ nghĩa/câu chữ |
| `33` | `0x0078AE11` | Ký hiệu và quang cảnh không phù hợp, mất kết nối | Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `34` | `0x0078AE23` | Đăng nhập lại qua Server | Đăng nhập lại thông qua máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `35` | `0x0078AE35` | Hiệp định đăng nhập | Quy ước/phiên đăng nhập. | Hiệu đính ngữ nghĩa/câu chữ |
| `36` | `0x0078AE47` | Phạm vi - ID không phù hợp | ID nằm ngoài phạm vi hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `37` | `0x0078AE59` | Rớt mạng do không cùng quang cảnh | Mất kết nối do không cùng khung cảnh. | Hiệu đính ngữ nghĩa/câu chữ |
| `38` | `0x0078AE6B` | Mục đích quang cảnh không phù hợp nên rớt mạng | Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `39` | `0x0078AF97` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `39` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `40` | `0x0078AE7D` | Sửa đổi files Tạo Vật bị rớt mạng | Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `41` | `0x0078AE8F` | Lưu lại sau khi đăng nhập | Lưu lại sau khi đăng nhập. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `42` | `0x0078AEA1` | Sửa đổi tư liệu chiến đấu | Dữ liệu chiến đấu đã bị chỉnh sửa. | Hiệu đính ngữ nghĩa/câu chữ |
| `43` | `0x0078AEB3` | Độ sai lệch hình thái chiến đấu của bạn chơi 0 | Dữ liệu trạng thái chiến đấu của người chơi không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `44` | `0x0078AEC5` | Sự kiện và quang cảnh xẩy ra không phù hợp | Sự kiện và khung cảnh không tương thích. | Hiệu đính ngữ nghĩa/câu chữ |
| `45` | `0x0078AED7` | Tài khoản sử dụng của bạn đã bị tạm khóa do phạm luật | Tài khoản của bạn đã tạm khóa do vi phạm quy định. | Hiệu đính ngữ nghĩa/câu chữ |
| `46` | `0x0078AEE9` | Gian xảo trong vấn đáp của Bắc Đẩu Quân | Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân. | Hiệu đính ngữ nghĩa/câu chữ |
| `47` | `0x0078AEFB` | Sự kiện kết thúc trước khi chiến đấu kết thúc | Sự kiện kết thúc trước khi trận chiến hoàn tất. | Hiệu đính ngữ nghĩa/câu chữ |
| `48` | `0x0078AF0D` | Phi pháp sử dụng kỹ năng Kêu Gọi | Sử dụng trái phép kỹ năng Triệu Hồi. | Hiệu đính ngữ nghĩa/câu chữ |
| `49` | `0x0078AF1F` | Cấm vận gia nhập đối với bạn chưa đủ 18 | Người chơi chưa đủ 18 tuổi không được phép tham gia. | Hiệu đính ngữ nghĩa/câu chữ |
| `50` | `0x0078AF2E` | Đăng nhập thi đấu chuyên thuộc Server | Đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `51` | `0x0078AF3D` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `52` | `0x0078AF4C` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `53` | `0x0078AF5B` | Chưa đăng nhập vào Server | Chưa đăng nhập vào máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `54` | `0x0078AF6A` | Hoạt động lôi đài đấu trận kết thúc | Sự kiện đấu trường đã kết thúc. | Hiệu đính ngữ nghĩa/câu chữ |
| `55` | `0x0078AF79` | Mục đích trong lưu trữ tư liệu di dân server | Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `56` | `0x0078AF88` | Server đang bận xin chờ tý xíu. | Máy chủ đang bận. Vui lòng chờ một lát. | Hiệu đính ngữ nghĩa/câu chữ |

### Main opcode `0x01` — 7 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `1` | `0x0078B1A8` | Đối tượng thì thầm | Đối tượng trò chuyện riêng. | Hiệu đính ngữ nghĩa/câu chữ |
| `1` | `0x0078B1A8` | Rời mạng | Đã ngắt kết nối mạng. | Hiệu đính ngữ nghĩa/câu chữ |
| `5` | `0x0078B464` | Không thể kết nối với server, xin bạn đợi sau đó thử lại lần nữa | Không thể kết nối đến máy chủ. Vui lòng chờ rồi thử lại. | Hiệu đính ngữ nghĩa/câu chữ |
| `6` | `0x0078B4CB` | Số liệu mật mã sai lệch | Dữ liệu mã hóa không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `7` | `0x0078B532` | Đối tượng server chưa được liên mạng,xin bạn đợi sau đó thử lại lần nữa | Đối tượng máy chủ chưa kết nối mạng. Vui lòng chờ rồi thử lại. | Hiệu đính ngữ nghĩa/câu chữ |
| `8` | `0x0078B599` | Account này hiện đang bị khóa | Tài khoản này hiện đang bị khóa. | Hiệu đính ngữ nghĩa/câu chữ |
| `10` | `0x0078B674` | Server đang bận xin chờ tý xíu. | Máy chủ đang bận. Vui lòng chờ một lát. | Hiệu đính ngữ nghĩa/câu chữ |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x0078AABE`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.
- `0x0078B149`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
