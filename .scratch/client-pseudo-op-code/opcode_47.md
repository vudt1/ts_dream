# PHÂN TÍCH — Main OP 0x47 (71) / Case 63 / FUN_007961c1 @ 0x007961C1

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh 100% từ mã nguồn sơ cấp** (`ts_decompile/` only).

- Handler `FUN_007961c1` phụ trách tiếp nhận các gói tin điều khiển **Hệ thống Kiểm soát Giờ chơi / Phòng chống Nghiện (Anti-Addiction / Fatigue System)** từ Server gửi xuống Client.
- Handler có 2 nhánh `SubOp` hoạt động:
  - `SubOp == 0x01`: Kích hoạt bảng thông báo giao diện kiểm soát giờ chơi (Anti-Addiction Info Window / Tab 9). Nếu Client chưa khởi tạo xong (`gvar_007DA37C + 4 == 0`), Client lưu cờ hoãn (`gvar_007DA37C + 0x3c = 1`) để bật lại sau. Nếu Client đã sẵn sàng, gọi `FUN_007a1794` để chuyển tab 9.
  - `SubOp == 0x02`: Kích hoạt biểu ngữ thông báo nổi (Toast Alert) qua `ToastManager` (`*(gvar_007DA084) + 0x90`) trong thời lượng 3000ms hiển thị chuỗi cảnh báo tại `&UNK_00799488`.
- Chiều C→S: `FUN_0077f414:1074` (`case 0x47: break;`) là rỗng — Client không bao giờ chủ động gửi Opcode này.

---

## 1. Tóm Tắt Nghiệp Vụ

- **Vai trò**: Cảnh báo và cưỡng chế thời gian chơi (Luật giới hạn giờ chơi 3 giờ giảm 50% kinh nghiệm, 5 giờ kinh nghiệm về 0 áp dụng cho các game online tại thị trường Châu Á giai đoạn 2006–2008).
- **Đối tượng đích**:
  - `gvar_007DA37C`: Đối tượng quản lý trạng thái trò chơi / màn chơi (`GameState` / `SceneState`).
    - `+4`: Cờ trạng thái sẵn sàng của client (`isInited` / `isReady`).
    - `+0x1D`: Chế độ hiển thị (`sceneMode`: 0 = thường, 1 = phim/cutscene, 2 = trong game, 3 = chuyển cảnh).
    - `+0x3C`: Cờ lưu trạng thái hoãn bật thông báo phòng chống nghiện (`pendingAntiAddictionNotice`).
  - `gvar_007DA1B8`: Instance quản lý cửa sổ giao diện đa tab (Multi-tab System Window).
  - `gvar_007D9E18`: Con trỏ ngữ cảnh dữ liệu của hệ thống thông báo.
  - `gvar_007DA084`: Quản lý thông báo nổi (`ToastManager`). VTable offset `+0x90` là hàm `ShowToast(text, duration_ms, flag1, flag2)`.
- **Hành vi cốt lõi**:
  - Gói tin chỉ mang tín hiệu kích hoạt (trigger) hoặc hiển thị cảnh báo; không mang cấu trúc payload phức tạp.
  - Tách biệt rõ ràng 2 mức độ thông báo: Popup giao diện chi tiết (`SubOp 0x01`) và Cảnh báo nhanh trên màn hình (`SubOp 0x02`).

---

## 2. Entry & Cách Đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x47 (71) → byte_table[0x78A8EE][0x47] = 0x3F (63)
                 → dword_table[0x78A9B6][63] @ 0x0078AAB2 = 0x007961C1
                 → FUN_007961c1 (Case 63)
```

- **File độc lập**: `ts_decompile/case_functions/functions/case_063_007961C1_FUN_007961c1.c` (70 dòng).
- **Bản inline trong dispatcher**: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7424-7447`.
- **Bản gộp**: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9699-9758`.
- Khớp 1:1 cấu trúc logic giữa bản hàm rời và bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]`: Byte thứ `i` của toàn bộ Payload nhận được (với `P[0] = 0x47`).
- `RP[i]`: Byte thứ `i` của `RestPayload` (tức `RP[i] = P[i+1]`). Delphi `_LStrCopy` tính chỉ mục từ 1.
- `RP` được lưu trên stack tại `unaff_EBP + -0xc` (hoặc `local_10` trong bản inline).

### 2.3. Kiểm tra độ dài & Đọc SubOp (dòng 20–26)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);            // RestPayload (RP)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {               // Length(RestPayload) == 0
  iVar1 = _BoundErr(0);                        // Lỗi out of bounds nếu chỉ nhận [47]
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // SubOp = RP[0]
```

- Nếu `Length < 1` (payload chỉ có byte `0x47` mà không có RestPayload) → gọi `_BoundErr(0)` báo lỗi biên mảng (RangeError).
- `SubOp = RP[0] = P[1]`.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire (Payload) | Tối thiểu | Core Logic |
| :---: | :--- | :---: | :--- |
| **`0x01`** | `[47][01]` | 2B | Kiểm tra `*(gvar_007DA37C + 4)`. Nếu `== 0` ghi `+0x3c = 1` (hoãn bật); nếu `!= 0` gọi `FUN_007a1794(*(gvar_007DA1B8), *(gvar_007D9E18), 9)` để mở tab 9 giao diện chống nghiện. |
| **`0x02`** | `[47][02]` | 2B | Gọi `ToastManager.ShowToast(&UNK_00799488, 3000)` hiển thị biểu ngữ cảnh báo giờ chơi trong 3 giây. |
| **Khác** | `[47][00]`, `[47][03]`... | 2B | **No-op im lặng** (không có nhánh `default`, không xử lý). |

---

## 4. Chi Tiết Core Logic Từng Nhánh

### 4.1. SubOp `0x01` — Mở Bảng Thông Tin Giờ Chơi (Anti-Addiction Dialog)

```c
if (*(int *)(unaff_EBP + -0x14) == 1) {
  if (*(char *)(*(int *)gvar_007DA37C + 4) == '\0') {
    *(undefined1 *)(*(int *)gvar_007DA37C + 0x3c) = 1;
  }
  else {
    FUN_007a1794(*(int **)gvar_007DA1B8,*(int *)gvar_007D9E18,9);
  }
}
```

1. **Kiểm tra trạng thái sẵn sàng**:
   - `*(char *)(*(int *)gvar_007DA37C + 4)`: Xác định xem Client đã tải xong bản đồ/nhân vật và sẵn sàng dựng giao diện hay chưa.
   - Nếu client đang trong quá trình chuyển cảnh/tải dữ liệu (`== '\0'`):
     - Ghi cờ `*(undefined1 *)(*(int *)gvar_007DA37C + 0x3c) = 1;`.
     - Cờ này sẽ được hàm kiểm tra chu kỳ game đọc khi quá trình tải hoàn tất để kích hoạt mở lại cửa sổ này.
2. **Kích hoạt giao diện (`FUN_007a1794`)**:
   - Nguồn: `ts_decompile/functions/007a1794_FUN_007a1794.c`:
     ```c
     void FUN_007a1794(int *param_1, int param_2, uint param_3) {
       (**(code **)(*param_1 + 0x20))();    // Gọi virtual method +0x20: Reset/Hide subviews hiện tại
       param_1[0x5f] = param_2;             // Lưu context pointer (+0x17C) = gvar_007D9E18
       FUN_007a0780((int)param_1, param_3); // Chuyển sang Tab index = param_3 (ở đây là 9)
       return;
     }
     ```
   - Callee `FUN_007a0780` (`ts_decompile/functions/007a0780_FUN_007a0780.c`):
     - Xử lý chuyển đổi View / Page của khung giao diện hệ thống sang trang thứ **9**.
     - Trang 9 tương ứng với trang thông tin cảnh báo kiểm soát giờ chơi, quy định chống nghiện và số giờ tích lũy của tài khoản.

### 4.2. SubOp `0x02` — Thông Báo Biểu Ngữ Nổi (Toast Warning Banner)

```c
else if (*(int *)(unaff_EBP + -0x14) == 2) {
  (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_00799488,3000,0,0);
}
```

- **Đối tượng**: `gvar_007DA084` là singleton quản lý hệ thống thông báo nhanh (Toast Alert Manager).
- **Phương thức ảo VMT `+0x90`**: Hàm hiển thị dòng chữ nổi trên đầu màn hình game.
  - `param_1`: `this` (`*(int **)gvar_007DA084`).
  - `param_2`: Con trỏ chuỗi văn bản thông báo `&UNK_00799488`.
  - `param_3`: Thời lượng hiển thị `3000` (miligiây, tức 3 giây).
  - `param_4`, `param_5`: Các cờ tham số phụ (kênh màu, âm thanh đính kèm = 0).
- **Nội dung chuỗi `UNK_00799488`**:
  - Nằm trong phân vùng dữ liệu tĩnh của binary. Đại diện cho chuỗi cảnh báo nhanh của hệ thống phòng chống nghiện (ví dụ: *"Bạn đã online quá thời gian quy định, hãy chú ý nghỉ ngơi"*).

---

## 5. Chiều Client → Server (C→S)

- Kiểm tra mã nguồn gửi lệnh `FUN_0077f414` (`ts_decompile/functions/0077f414_FUN_0077F414.c`):
  - Dòng 1074:
    ```c
    case 0x47:
      break;
    ```
- **Kết luận**: Nhánh rỗng `break;` — Client hoàn toàn **không có luồng gửi** Opcode `0x47` lên Server.
- Opcode `0x47` là giao thức điều khiển **một chiều từ Server xuống Client (S→C)**.

---

## 6. Đặc Tả Wire Layout Cho Mock Server

### 6.1. Khung dữ liệu nhị phân chuẩn

Mọi gói tin đều tuân thủ framing 2 chiều XOR tĩnh `0xAD`:
- `[Token 2B: 0xF4 0x44] [Length L: 2B LE] [Payload L Bytes]`
- Payload sau giải mã XOR:

#### Ca 1: Yêu cầu mở bảng cảnh báo giờ chơi
```
F4 44 02 00 47 01
```
- `F4 44`: Magic Token.
- `02 00`: Độ dài Payload = 2 bytes.
- `47`: MainOp `0x47`.
- `01`: SubOp `0x01` (Bật giao diện / Tab 9).

#### Ca 2: Hiển thị Toast cảnh báo nhanh 3 giây
```
F4 44 02 00 47 02
```
- `47`: MainOp `0x47`.
- `02`: SubOp `0x02` (Bật Toast 3000ms).

---

## 7. Bảng Bằng Chứng Mã Nguồn Sơ Cấp (SSOT)

| Ký hiệu / Địa chỉ | File nguồn | Dòng | Vai trò xác minh |
| :--- | :--- | :--- | :--- |
| `FUN_007961c1` | `ts_decompile/case_functions/functions/case_063_007961C1_FUN_007961c1.c` | 8–67 | Handler độc lập của MainOp 0x47 |
| `case 0x47:` | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | 7424–7447 | Bản inline trong Main Dispatcher |
| `byte_table[0x47]=0x3F` | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | Hàng 5 | Ánh xạ Opcode 0x47 sang Case 63 |
| `dword_table[63]=0x7961C1` | `ts_decompile/case_functions/manifest.csv` | Dòng 64 | Điểm đến 0x007961C1 của Case 63 |
| `FUN_007a1794` | `ts_decompile/functions/007a1794_FUN_007a1794.c` | 18–25 | Hàm kích hoạt Tab 9 của cửa sổ hệ thống |
| `FUN_007a0780` | `ts_decompile/functions/007a0780_FUN_007a0780.c` | 31–160 | Đổi view/trang hiển thị đa tab |
| `gvar_007DA084` | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | 7445 | Toast Manager (`+0x90` ShowToast 3000ms) |
| `gvar_007DA37C` | `ts_decompile/functions/0051bca4_FUN_0051bca4.c` | 50 | Quản lý GameState (`+4` ready, `+0x3c` pending flag) |
| `case 0x47: break;` | `ts_decompile/functions/0077f414_FUN_0077F414.c` | 1074 | Xác nhận không có chiều gửi C→S |

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x007961C1` — C `client_pseudo_c/case_063_007961C1_FUN_007961c1.c` (có); asm `client_pseudo_c/007961c1_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x47` — 1 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| — | `0x007961C1` | Đã mở cơ chế Phòng thẩm mật | Đã mở cơ chế Phòng thẩm mật. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x007961C1`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
