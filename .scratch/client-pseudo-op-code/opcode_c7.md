# PHÂN TÍCH — Main OP 0xC7 (199) / Case 65 / FUN_007962fc @ 0x007962FC

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh phần khung và dispatch từ mã nguồn sơ cấp** (`ts_decompile/` only). Cả hai hàm callee (`func_0x00613634`, `func_0x0061353c`) **KHÔNG có body trong SSOT (chưa decompile / cần redump)**.

- Handler `FUN_007962fc` là Case cuối cùng (Case 65) trong bảng nhảy `0x78A9B6` của Main Dispatcher.
- Mọi nhánh của Opcode này đều thao tác trên singleton `gvar_007DA160`, chính là đối tượng quản trị viên **`TGmManage` (Game Master Management Object)**.
- Chiều C→S: `FUN_0077f414:1078` (`case 199: }`) đóng switch mà không có lệnh nào — Client không gửi Opcode `0xC7` qua `SendCommand`.

---

## 1. Tóm Tắt Nghiệp Vụ

- **Đối tượng đích**:
  - `gvar_007DA160`: Thể hiện duy nhất (Singleton) của lớp `TGmManage` (VMT `VMT_613160_TGmManage` @ `0x00613160`).
  - Được khởi tạo tại hàm `0050a4a0_TForm1.FormCreate.c:625`:
    ```c
    piVar6 = TGmManage_Create((int *)VMT_613160_TGmManage, '\x01', extraout_ECX_32);
    *(int **)gvar_007DA160 = piVar6;
    ```
- **Hệ thống nghiệp vụ**:
  - `TGmManage` là bộ quản lý các chức năng, công cụ kiểm soát và lệnh đặc quyền của Game Master (GM / Quản trị viên).
  - Trong Main Dispatcher, Main OP `0x10` (Case 16, dòng 2875–2941) cũng phân phối hàng loạt lệnh điều hành GM tới `TGmManage` thông qua các hàm thuộc dải địa chỉ `0x00613xxx` và `0x00614xxx`.
  - Main OP `0xC7` đóng vai trò là kênh mở rộng điều khiển GM thứ hai (kênh chuyên biệt cho 2 loại phản hồi GM cấp cao).
- **Đính chính quan trọng so với tài liệu bên ngoài**:
  - Một số tài liệu không chính thức hoặc dự án giả lập (như `ts_dream/src/protocol/mod.rs:171`) từng đặt tên cho `0xC7` là `OP_RECONNECT`.
  - Tuy nhiên, theo **nguồn sơ cấp duy nhất (SSOT)** tại `ts_decompile/`, Opcode `0xC7` **không có bất kỳ liên hệ nào tới socket hay tái kết nối**, mà 100% điều hướng về `TGmManage`.

---

## 2. Entry & Cách Đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0xC7 (199) → byte_table[0x78A8EE][0xC7] = 0x41 (65)
                  → dword_table[0x78A9B6][65] @ 0x0078AABA = 0x007962FC
                  → FUN_007962fc (Case 65)
```

- **File độc lập**: `ts_decompile/case_functions/functions/case_065_007962FC_FUN_007962fc.c` (65 dòng).
- **Bản inline trong dispatcher**: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7484-7502`.
- **Bản gộp**: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9841-9905`.
- Khớp 1:1 giữa bản hàm độc lập và bản inline trong `FUN_0078a89c`.

### 2.2. Kiểm tra độ dài & Đọc SubOp

```c
iVar2 = *(int *)(unaff_EBP + -0xc);            // RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {               // Kiểm tra Length == 0
  iVar1 = _BoundErr(0);                        // Báo lỗi RangeError nếu L < 2
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // SubOp = RP[0]
if (*(int *)(unaff_EBP + -0x14) == 4) {
  func_0x00613634(*(undefined4 *)gvar_007DA160, *(undefined4 *)(unaff_EBP + -0xc));
}
else if (*(int *)(unaff_EBP + -0x14) == 6) {
  func_0x0061353c(*(undefined4 *)gvar_007DA160, *(undefined4 *)(unaff_EBP + -0xc));
}
```

- Nếu `Length < 1` (payload chỉ có 1 byte `[C7]`) → gọi `_BoundErr(0)`.
- `SubOp = RP[0] = P[1]`.
- Chỉ có 2 giá trị SubOp được phân nhánh: `0x04` và `0x06`. Mọi giá trị khác đều rơi xuống phần dọn stack và kết thúc hàm (**no-op im lặng**).

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire (Payload) | Min Len | Callee | Trạng thái SSOT | Ý nghĩa nghiệp vụ |
| :---: | :--- | :---: | :--- | :---: | :--- |
| **`0x04`** | `[C7][04][...]` | 2B | `func_0x00613634` | **Không có body trong SSOT** | Lệnh phản hồi / cập nhật GM loại 4 (`TGmManage`). |
| **`0x06`** | `[C7][06][...]` | 2B | `func_0x0061353c` | **Không có body trong SSOT** | Lệnh phản hồi / cập nhật GM loại 6 (`TGmManage`). |
| **Khác** | `[C7][00]`, `[C7][01]`... | 2B | Không có | — | **No-op im lặng** (bỏ qua không xử lý). |

---

## 4. Chi Tiết Các Nhánh & Hiện Trạng Callee

### 4.1. Hiện trạng chưa decompile của các Callee (`ts_decompile/index.csv`)

Trong file mục lục `ts_decompile/index.csv`:
- Dòng 3971: `FUN_006128bc` (`006128bc`, kích thước 1374 bytes) kết thúc tại địa chỉ `0x00612E1A`.
- Dòng 3972: `FUN_00614c38` (`00614c38`, kích thước 88 bytes) bắt đầu tại địa chỉ `0x00614C38`.
- Cả hai địa chỉ `0x00613634` và `0x0061353c` (cùng các phương thức GM khác như `00613240`, `00613b50`, `00614724`...) đều nằm trong khoảng trống **`0x00612E1A → 0x00614C38` (gần 7.7 KB mã máy)** chưa được decompile thành file C rời.

### 4.2. Mối liên hệ với Main OP 0x10

Trong `ts_decompile/functions/0078a89c_FUN_0078a89c.c:2875-2941` (Case 16 — OP 0x10), dispatcher gọi liên tiếp các phương thức của `gvar_007DA160`:
- `func_0x00613e48`: Lệnh GM truy vấn thông tin tài khoản/nhân vật.
- `func_0x00614318`: Lệnh GM dịch chuyển (Teleport).
- `func_0x00614bb0`: Lệnh GM sinh vật phẩm / quái.
- `func_0x006140d4`: Lệnh GM cấm chat / kick người chơi.
- `func_0x0061353c`: Được gọi tại dòng 7500 (SubOp `0x06` của OP 0xC7).
- `func_0x00613634`: Được gọi tại dòng 7496 (SubOp `0x04` của OP 0xC7).

Điều này xác nhận rằng: `func_0x0061353c` và `func_0x00613634` là các **phương thức thành viên của lớp `TGmManage`**. Khi nhận gói `0xC7 04` hoặc `0xC7 06`, Client thực hiện chuyển giao nguyên khối `RestPayload` cho `TGmManage` giải mã.

---

## 5. Chiều Client → Server (C→S)

- Tại `ts_decompile/functions/0077f414_FUN_0077F414.c:1078`:
  ```c
  case 199:
  }
  ```
- Nhánh `case 199` (`0xC7`) kết thúc ngay trước dấu ngoặc đóng của switch trong `TFConnect.SendCommand`.
- **Kết luận**: Client **không bao giờ gửi** Opcode `0xC7` lên Server. Đây là gói tin một chiều từ Server gửi xuống Client (S→C).

---

## 6. Khuyến Nghị Cho Mock Server / Server Emulator

1. **Không sử dụng 0xC7 cho Reconnect**:
   - Khẳng định lại: Không gửi `0xC7` với kỳ vọng xử lý bắt tay tái kết nối (TCP Reconnect). Luồng kết nối lại của client TS Online được điều khiển qua form đăng nhập / chọn cụm server (`aLogin.exe` / `TFConnect`).
2. **Xử lý tài khoản GM**:
   - `0xC7` chỉ phát sinh khi người chơi có thẩm quyền GM đang đăng nhập và Server gửi các gói tin điều phối công cụ quản trị (GM Tools) xuống client.
   - Đối với người chơi bình thường, Server không bao giờ cần gửi Opcode này.
3. **Cấu trúc gói tối thiểu nếu cần kích hoạt GM**:
   - `F4 44 [Len: 2B LE] C7 04 [Payload GM...]`
   - `F4 44 [Len: 2B LE] C7 06 [Payload GM...]`

---

## 7. Bảng Bằng Chứng Mã Nguồn Sơ Cấp (SSOT)

| Ký hiệu / Địa chỉ | File nguồn | Dòng | Vai trò xác minh |
| :--- | :--- | :--- | :--- |
| `FUN_007962fc` | `ts_decompile/case_functions/functions/case_065_007962FC_FUN_007962fc.c` | 8–62 | Handler độc lập MainOp 0xC7 |
| `case 199:` | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | 7484–7502 | Bản inline trong Main Dispatcher |
| `byte_table[0xC7]=0x41` | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | Hàng 13 | Ánh xạ Opcode 0xC7 (199) sang Case 65 |
| `dword_table[65]=0x7962FC` | `ts_decompile/case_functions/manifest.csv` | Dòng 66 | Địa chỉ nhảy 0x007962FC của Case 65 |
| `gvar_007DA160` | `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c` | 625 | Con trỏ singleton quản lý GM (`TGmManage`) |
| `VMT_613160_TGmManage` | `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c` | 624 | Bảng hàm ảo của lớp `TGmManage` |
| `case 0x10:` | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | 2875–2941 | Đối chiếu MainOp 0x10 cùng gọi `TGmManage` |
| `case 199:` | `ts_decompile/functions/0077f414_FUN_0077F414.c` | 1078 | Xác nhận không có chiều gửi C→S |
