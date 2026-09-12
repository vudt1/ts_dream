# PHÂN TÍCH — Main OP 0x45 (69) / Case 61 / FUN_00795f82 @ 0x00795F82

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh 100% từ mã nguồn sơ cấp** (`ts_decompile/` only).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x45` (thập phân: `69`, ánh xạ tới **Case 61**) phụ trách toàn bộ **Hệ Thống Con Cái (Child System / Hài Nhi / Con Mọn / Oa Oa)** trong TS Online:
1. **Quản lý Vòng Đời & Trạng Thái Con Cái (Child Lifecycle & Growth Stages)**:
   - Hệ thống con cái đồng hành được định danh thông qua cấu trúc khối dữ liệu tại `TPlayer + 0x151d` (dài đúng **93 bytes** / `0x5D` bytes).
   - Byte đầu tiên của khối dữ liệu (`TPlayer + 0x151d`) quản lý các giai đoạn phát triển:
     - `0x01`: Giai đoạn chuẩn bị sinh / mang thai / chưa kích hoạt.
     - `0x02`: Giai đoạn sơ sinh / đã sinh / con cái đang theo người chơi (bế bồng, đi theo nhân vật ngoài bản đồ và trong trận chiến). Khi vừa chuyển sang trạng thái này, client kích hoạt biểu ngữ nổi (Toast Alert) chúc mừng sinh con (`DAT_0074fc88`) trong thời lượng 2000 ms.
     - `0x03`: Giai đoạn trưởng thành (Adult). Con cái tự động tiến hóa thành người lớn khi điểm thân mật / kinh nghiệm trưởng thành (`TPlayer + 0x1565`) vượt qua ngưỡng **399** (đạt $\ge 400$).
2. **Khối Dữ Liệu Đồng Bộ Toàn Diện 93 Bytes (`SubOp 0x07` - `FUN_0074fb94`)**:
   - Server gửi nguyên khối 93 bytes đồng bộ toàn bộ thông tin cơ bản, ngoại hình, giới tính, tên tuổi và thuộc tính cốt lõi của con cái vào thẳng vùng nhớ nhân vật chính `TPlayer + 0x151d`.
3. **Quản Lý 6 Vị Trí Trang Bị Con Cái (`SubOp 0x05` - `FUN_0074f034`)**:
   - Con cái sở hữu 6 ô trang bị độc lập với người chơi, đánh số từ `0x1B` đến `0x20` (Vũ khí, Nón, Áo, Cổ tay, Giày, Phụ kiện đặc thù).
   - Mỗi ô trang bị lưu trữ cặp giá trị: `ItemID` (2 bytes Word LE tại `+0x1553 .. +0x155d`) và `Thuộc tính/Cấp cường hóa` (1 byte tại `+0x155f .. +0x1564`).
4. **Điểm Trưởng Thành & Tiến Hóa (`SubOp 0x08` - `FUN_0074efac`)**:
   - Nhận giá trị trưởng thành (2 bytes Word LE), cập nhật vào `TPlayer + 0x1565`. Nếu giá trị này lớn hơn 399, client tự động thăng cấp trạng thái con cái lên Adult (`+0x151d = 3`).
5. **Hệ Thống Kỹ Năng & Quản Lý Quên/Xóa Kỹ Năng Con Cái (`TLH_ChildSkillDelForm` & `FUN_0074f504`)**:
   - Con cái lưu giữ 9 kỹ năng chia làm 3 bộ (mỗi bộ 3 kỹ năng) tại các offset `+0x573 .. +0x58b` trong cấu trúc thực thể `TFollowNpc` (`NPC Type = 4`).
   - Tương tác với form `TLH_ChildSkillDelForm` (`gvar_007D9E3C`) cho phép tẩy/xóa từng bộ kỹ năng con cái, có kiểm tra điều kiện (nếu cả 3 kỹ năng trong bộ đều bằng 0 thì bật cảnh báo và từ chối) và hiện hộp thoại xác nhận.
6. **Đồng Bộ Roster Đồng Hành (`SubOp 0x09` - `func_0x007a4c28`)**:
   - Quản lý con cái thông qua bộ điều phối đồng hành `TFNpcManage` (`gvar_007D9E00`), cho phép con cái xuất hiện hoặc ẩn trong danh sách quản lý NPC/pet của người chơi.
7. **Chiều Giao Tiếp Mạng**:
   - Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `FUN_0077f414:1070-1071` (`case 0x45: break;`) là rỗng (`break;`). Client không bao giờ chủ động gửi Main Opcode 0x45 lên server.

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x45 (69) → byte_table[0x78A8EE][0x45] = 0x3D (61)
                  → dword_table[0x78A9B6][61] @ 0x0078AAAA = 0x00795F82
                  → FUN_00795f82 (Case 61)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` ([`ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c)) tách byte đầu `P[0] = 0x45` làm Opcode, cắt Rest Payload (`local_10`), gọi `FUN_0078a89c(Self, Opcode, Payload)`.
2. **Dispatcher Inline**: Nằm tại [`ts_decompile/functions/0078a89c_FUN_0078a89c.c:7317-7372`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.c#L7317-L7372).
3. **Case Function độc lập**: [`ts_decompile/case_functions/functions/case_061_00795F82_FUN_00795f82.c:20-60`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/functions/case_061_00795F82_FUN_00795f82.c#L20-L60).

### 2.2. Kiểm tra độ dài & Đọc SubOp
```c
iVar2 = *(int *)(unaff_EBP + -0xc); // RestPayload (RP)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {     // Kiểm tra Length(RP) == 0
  iVar1 = _BoundErr(0);              // Báo lỗi Range Error nếu không có RestPayload
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // SubOp = RP[0]
```
- `RP[0]` (byte đầu của RestPayload, tức byte thứ 2 của toàn bộ frame mạng) là **SubOp** (giá trị từ `1` đến `11`).
- Khối lệnh `switch` phân phối tới 11 nhánh xử lý độc lập. Kết thúc hàm thực hiện giải phóng mảng biến chuỗi tạm thời bằng `_LStrArrayClr` và dọn dẹp bộ nhớ stack frame an toàn.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Hex | Tên Nghiệp Vụ | Đối Tượng Tiếp Nhận | Hàm Callee | Min Len | Tình trạng SSOT |
| :---: | :---: | :--- | :--- | :--- | :---: | :---: |
| **`1`** | `0x01` | Khởi tạo thông tin / Kích hoạt sinh con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x0074fca0` | 1B | ✔ Codebase |
| **`2`** | `0x02` | Đồng bộ chỉ số chi tiết của con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x0074fe94` | 1B | ✔ Codebase |
| **`3`** | `0x03` | Cập nhật tâm trạng / Hành động con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x0074ee74` | 1B | ✔ Codebase |
| **`4`** | `0x04` | Phản hồi nuôi dưỡng / Chăm sóc con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x0074ef3c` | 1B | ✔ Codebase |
| **`5`** | `0x05` | Cập nhật trang bị con cái (Slots 0x1B..0x20) | `gvar_007DA7BC` (`TPlayer`) | [`FUN_0074f034`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f034_FUN_0074f034.c) | 5B | ✔ Đã decompile |
| **`6`** | `0x06` | Đồng bộ danh sách & Cấp độ kỹ năng con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x0074f1ac` | 1B | ✔ Codebase |
| **`7`** | `0x07` | Nạp khối dữ liệu con cái 93 bytes & Báo sinh | `gvar_007DA7BC` (`TPlayer`) | [`FUN_0074fb94`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074fb94_FUN_0074fb94.c) | 96B | ✔ Đã decompile |
| **`8`** | `0x08` | Cập nhật điểm trưởng thành & Chuyển Adult | `gvar_007DA7BC` (`TPlayer`) | [`FUN_0074efac`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074efac_FUN_0074efac.c) | 3B | ✔ Đã decompile |
| **`9`** | `0x09` | Đồng bộ danh sách con cái trong Roster quản lý | `gvar_007D9E00` (`TFNpcManage`) | `func_0x007a4c28` | 1B | ✔ Codebase |
| **`10`**| `0x0A` | Phản hồi / Đồng bộ form xóa kỹ năng con cái | `gvar_007D9E3C` (`TLH_ChildSkillDelForm`) | `func_0x005e7f90` | 1B | ✔ Codebase |
| **`11`**| `0x0B` | Reset trạng thái hành động / Toggle con cái | `gvar_007DA7BC` (`TPlayer`) | `func_0x00750240` | 1B | ✔ Codebase |

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x05` — Cập Nhật Trang Bị Con Cái ([`FUN_0074f034`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f034_FUN_0074f034.c))
- **Đối tượng**: `*(int *)gvar_007DA7BC` (`Local Player Instance`).
- **Phân tích mã nguồn**:
  ```c
  void FUN_0074f034(int param_1, int param_2) {
    local_c = param_2; // RestPayload
    local_8 = param_1; // TPlayer
    
    local_d = *(undefined1 *)(param_2 + 1); // RP[1] = SlotID (0x1B..0x20)
    local_e = *(undefined1 *)(param_2 + 2); // RP[2] = Thuộc tính / Grade (Byte)
    _LStrCopy(local_c, 4, 2, &local_14);    // RP[3..4] = 2 bytes ItemID
    local_10 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_14); // Word Little-Endian

    switch(local_d) {
    case 0x1b: // Slot 27: Vũ khí con cái
      *(undefined1 *)(local_8 + 0x155f) = local_e;
      *(undefined2 *)(local_8 + 0x1553) = local_10;
      break;
    case 0x1c: // Slot 28: Nón con cái
      *(undefined1 *)(local_8 + 0x1560) = local_e;
      *(undefined2 *)(local_8 + 0x1555) = local_10;
      break;
    case 0x1d: // Slot 29: Áo con cái
      *(undefined1 *)(local_8 + 0x1561) = local_e;
      *(undefined2 *)(local_8 + 0x1557) = local_10;
      break;
    case 0x1e: // Slot 30: Cổ tay con cái
      *(undefined1 *)(local_8 + 0x1562) = local_e;
      *(undefined2 *)(local_8 + 0x1559) = local_10;
      break;
    case 0x1f: // Slot 31: Giày con cái
      *(undefined1 *)(local_8 + 0x1563) = local_e;
      *(undefined2 *)(local_8 + 0x155b) = local_10;
      break;
    case 0x20: // Slot 32: Phụ kiện / Đặc thù con cái
      *(undefined1 *)(local_8 + 0x1564) = local_e;
      *(undefined2 *)(local_8 + 0x155d) = local_10;
      break;
    }
  }
  ```
- **Ý nghĩa & Bản đồ bộ nhớ**:
  - `SlotID` phải nằm trong đoạn `0x1B .. 0x20` (27 .. 32). Nếu nằm ngoài, switch thoát mà không làm thay đổi trạng thái.
  - Vùng thuộc tính trang bị: Mảng 6 bytes tại `TPlayer + 0x155f .. 0x1564`.
  - Vùng Item ID trang bị: Mảng 6 Words (12 bytes) tại `TPlayer + 0x1553 .. 0x155d`.
  - Đối chiếu liên thông: Hàm [`FUN_005b6dd0`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/005b6dd0_FUN_005b6dd0.c) đọc trực tiếp 6 Word này để cập nhật chỉ số vào các thuộc tính cơ bản `+0x3ee, +0x3f0, +0x3f2, +0x3f4, +0x3f6, +0x3f8` (INT, ATK, DEF, AGI, HP, SP) của con cái khi xuất chiến.

---

### 4.2. SubOp `0x07` — Nạp Khối Dữ Liệu Con Cái 93 Bytes ([`FUN_0074fb94`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074fb94_FUN_0074fb94.c))
- **Đối tượng**: `*(int *)gvar_007DA7BC` (`Local Player Instance`).
- **Phân tích mã nguồn**:
  ```c
  void FUN_0074fb94(int param_1, int param_2) {
    local_c = param_2; // RestPayload
    local_8 = param_1; // TPlayer
    
    _LStrCopy(param_2, 2, 2, local_18); // RP[1..2] = Block Length (Word LE)
    local_18[2] = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_18[0]) & 0xffff;
    
    if (local_18[2] == 0x5d) { // BẮT BUỘC ĐỘ DÀI = 0x5D (93 bytes)
      _LStrCopy(local_c, 4, 0x5d, local_18 + 1); // Cắt 93 bytes dữ liệu
      uVar1 = _UniqueStringA(local_18 + 1);
      
      // Sao chép nguyên khối vào TPlayer + 0x151d
      Move((undefined4 *)(uVar1 + iVar2), (undefined4 *)(local_8 + 0x151d), 0x5d);
      
      // Nếu byte trạng thái đầu khối == 0x02 (Đã sinh con / Active)
      if (*(char *)(local_8 + 0x151d) == '\x02') {
        // Bật popup toast thông báo chúc mừng sinh con trong 2000ms
        (**(code **)(**(int **)gvar_007DA084 + 0x90))
                 (*(int **)gvar_007DA084, &DAT_0074fc88, 2000, 0, 0);
      }
    }
  }
  ```
- **Ý nghĩa cốt lõi**:
  - Gói tin nạp cấu trúc toàn diện của con cái.
  - Client ràng buộc nghiêm ngặt: trường độ dài phải bằng đúng `0x005D` (93). Nếu khác 93 bytes, toàn bộ gói tin bị bỏ qua.
  - Khi `*(TPlayer + 0x151d) == 0x02`, client nhận diện con cái vừa được sinh ra hoặc đang kích hoạt đồng hành, lập tức phát thông báo popup toast `DAT_0074fc88` trong 2000 ms qua `TToastManager` (`gvar_007DA084 + 0x90`).
  - Trong [`FUN_00750278`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00750278_FUN_00750278.c#L78-L90), client kiểm tra `*(TPlayer + 0x151d) == 0x02` để quyết định có render hình ảnh con cái đi theo trên bản đồ và trong trận đánh hay không.

---

### 4.3. SubOp `0x08` — Cập Nhật Điểm Trưởng Thành & Tiến Hóa ([`FUN_0074efac`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074efac_FUN_0074efac.c))
- **Đối tượng**: `*(int *)gvar_007DA7BC` (`Local Player Instance`).
- **Phân tích mã nguồn**:
  ```c
  void FUN_0074efac(int param_1, int param_2) {
    local_c = param_2; // RestPayload
    local_8 = param_1; // TPlayer
    
    _LStrCopy(param_2, 2, 2, &local_10); // RP[1..2] = Maturity Points (Word LE)
    uVar1 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_10);
    
    *(undefined2 *)(local_8 + 0x1565) = uVar1; // Cập nhật điểm trưởng thành
    
    // Kiểm tra ngưỡng trưởng thành
    if (399 < *(ushort *)(local_8 + 0x1565)) {
      *(undefined1 *)(local_8 + 0x151d) = 3;   // Chuyển sang trạng thái Trưởng Thành (Adult)
    }
  }
  ```
- **Ý nghĩa cốt lõi**:
  - Gói tin cập nhật điểm trưởng thành tích lũy (Maturity / Growth points) của con cái vào `+0x1565`.
  - Ngưỡng tiến hóa cố định: nếu giá trị $> 399$ (tức từ 400 điểm trở lên), client cập nhật cờ trạng thái con cái `+0x151d` thành `0x03` (Trưởng thành). Khi đạt trạng thái này, con cái chuyển sang mô hình thiếu niên/người lớn và mở khóa toàn bộ tính năng chiến đấu nâng cao.

---

### 4.4. Cơ Chế Xóa Kỹ Năng Con Cái Liên Quan ([`FUN_0074f504`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c))
Module con cái sở hữu logic xóa kỹ năng liên kết chặt chẽ với form `TLH_ChildSkillDelForm` (`gvar_007D9E3C`):
- Khi người chơi yêu cầu quên/xóa kỹ năng con cái, hàm `FUN_0074f504` nhận tham số `param_2`:
  1. `param_2 == -0x1ff (-511)`: Đóng form xóa kỹ năng: `(**vtable + 0x20)()`.
  2. `param_2 == -0x1fe (-510)`: Chọn xóa **Bộ Kỹ Năng 1**:
     - Kiểm tra 3 kỹ năng tại `+0x573, +0x576, +0x579` của `TFollowNpc` con cái (`TPlayer + 0x57c + slot*4`).
     - Nếu cả 3 kỹ năng đều bằng 0 (chưa học): Hiển thị thông báo Toast cảnh báo `DAT_0074f814` trong 1500 ms và đặt lại cờ form `+0x146 = 0`.
     - Nếu có ít nhất 1 kỹ năng: Đặt `Form + 0x159 = 1`, hiển thị hộp thoại xác nhận Yes/No với câu hỏi `0x74f85c` qua `FUN_0063c674`.
  3. `param_2 == -0x1fd (-509)`: Chọn xóa **Bộ Kỹ Năng 2**:
     - Kiểm tra 3 kỹ năng tại `+0x57c, +0x57f, +0x582`.
     - Nếu chưa học: Bật Toast `DAT_0074f8bc` trong 1500 ms.
     - Nếu có kỹ năng: Đặt `Form + 0x159 = 2`, hiển thị hộp thoại xác nhận Yes/No với câu hỏi `0x74f904`.
  4. `param_2 == -0x1fc (-508)`: Chọn xóa **Bộ Kỹ Năng 3**:
     - Kiểm tra 3 kỹ năng tại `+0x585, +0x588, +0x58b`.
     - Nếu chưa học: Bật Toast `DAT_0074f960` trong 1500 ms.
     - Nếu có kỹ năng: Đặt `Form + 0x159 = 3`, hiển thị hộp thoại xác nhận Yes/No với câu hỏi `0x74f9a4`.

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan

| Biến Toàn Cục | Kiểu Dữ Liệu / Lớp | Khởi Tạo & VMT | Vai Trò Nghiệp Vụ |
| :--- | :--- | :--- | :--- |
| `gvar_007DA7BC` | `TPlayer` | `00716370_TPlayer.Create.c` | Đối tượng người chơi chính. Chứa toàn bộ mảng con cái `+0x57c`, slot active `+0x151c`, dữ liệu con cái `+0x151d`, trang bị `+0x1553..+0x1564`, độ trưởng thành `+0x1565`. |
| `gvar_007D9E00` | `TFNpcManage` | `VMT_7A1894` @ `0051189c.c:1389` | Trình quản lý danh sách NPC / Pet / Con cái đồng hành. Tiếp nhận SubOp 0x09. |
| `gvar_007D9E3C` | `TLH_ChildSkillDelForm` | `VMT_5D4B5C` @ `0051189c.c:1479` | Bảng giao diện xóa / quên kỹ năng con cái. Tiếp nhận SubOp 0x0A. |
| `gvar_007DA75C` | `TSe_ChildForm` | `VMT_5893A0` @ `0051189c.c:1560` | Bảng giao diện thông tin & quản lý nuôi dạy con cái tổng thể của người chơi. |
| `gvar_007DA084` | `TSe_TalkMsgFormPlus` | `VMT_7B7A24` @ `0051189c.c:1265` | Quản lý biểu ngữ nổi (Toast Alert). Offset vtable `+0x90` là hàm `ShowToast(msg, duration_ms, 0, 0)`. |
| `gvar_007D9D6C` | `TSe_TalkMsgFormPlus` | `VMT_7B7A24` @ `0051189c.c:1267` | Quản lý hộp thoại xác nhận modal (Yes/No Question) qua `FUN_0063c674`. |
| `gvar_007D9D30` | `TNetworkDecoder` | Quản lý giải mã nhị phân | Context giải mã Word LE (`FUN_0077eb9c`) và Dword LE (`FUN_0077ef7c`). |

---

## 6. Chiều Client → Server (C → S)

- Kiểm tra tại [`ts_decompile/functions/0077f414_FUN_0077F414.c:1070-1071`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077f414_FUN_0077F414.c#L1070-L1071):
  ```c
  case 0x45:
    break;
  ```
- **Kết luận**: Nhánh `case 0x45:` rỗng (`break;`). Client **không bao giờ chủ động phát gói tin mang Main Opcode 0x45** lên server. Main OP `0x45` là giao thức cập nhật một chiều từ **Server xuống Client (S → C)**.
- **Phản hồi nghiệp vụ liên quan**: Khi người chơi xác nhận xóa kỹ năng trên form con cái (`TLH_ChildSkillDelForm`), client gửi gói tin lệnh hành động NPC (`MainOp = 0x17` hoặc qua lệnh socket tương ứng) mang danh mục bộ kỹ năng cần xóa (`1, 2, 3`) và chỉ số slot con cái (`Form + 0x146`).

---

## 7. Chuỗi Hiển Thị, VISCII / CP1258 & Literal Strings

Toàn bộ các chuỗi liên quan đến sự kiện con cái được lưu trong phân đoạn mã của `FUN_0074fb94` và `FUN_0074f504`:
1. **`DAT_0074fc88`** (tham chiếu tại [`0074fb94_FUN_0074fb94.c:58`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074fb94_FUN_0074fb94.c#L58)):
   - Phương thức hiển thị: `ShowToast(&DAT_0074fc88, 2000 ms)` qua `gvar_007DA084 + 0x90`.
   - Ngữ cảnh: Thông báo nổi khi con cái chào đời / kích hoạt đồng hành thành công (`State = 0x02`).
2. **`DAT_0074f814`** (tham chiếu tại [`0074f504_FUN_0074f504.c:44`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L44)):
   - Phương thức hiển thị: `ShowToast(&DAT_0074f814, 1500 ms)`.
   - Ngữ cảnh: Cảnh báo khi người chơi chọn xóa kỹ năng Bộ 1 nhưng con cái chưa học kỹ năng nào trong bộ này.
3. **`0x74f85c`** (tham chiếu tại [`0074f504_FUN_0074f504.c:51`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L51)):
   - Hộp thoại câu hỏi xác nhận Yes/No: Hỏi người chơi có chắc chắn muốn xóa bộ kỹ năng 1 của con cái hay không.
4. **`DAT_0074f8bc`** (tham chiếu tại [`0074f504_FUN_0074f504.c:71`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L71)):
   - Phương thức hiển thị: `ShowToast(&DAT_0074f8bc, 1500 ms)`.
   - Ngữ cảnh: Cảnh báo con cái chưa học kỹ năng nào thuộc Bộ 2.
5. **`0x74f904`** (tham chiếu tại [`0074f504_FUN_0074f504.c:78`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L78)):
   - Hộp thoại câu hỏi xác nhận Yes/No về việc xóa bộ kỹ năng 2.
6. **`DAT_0074f960`** (tham chiếu tại [`0074f504_FUN_0074f504.c:98`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L98)):
   - Phương thức hiển thị: `ShowToast(&DAT_0074f960, 1500 ms)`.
   - Ngữ cảnh: Cảnh báo con cái chưa học kỹ năng nào thuộc Bộ 3.
7. **`0x74f9a4`** (tham chiếu tại [`0074f504_FUN_0074f504.c:105`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c#L105)):
   - Hộp thoại câu hỏi xác nhận Yes/No về việc xóa bộ kỹ năng 3.

---

## 8. Cấu Trúc Rest Payload & Wire Format Chi Tiết

### 8.1. SubOp 0x05 (Cập Nhật Trang Bị Con Cái)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x05
+01      uint8       SlotID        Vị trí ô trang bị (0x1B .. 0x20)
+02      uint8       AttrGrade     Thuộc tính / Cấp cường hóa trang bị
+03      uint16 LE   ItemID        Mã định danh vật phẩm (2 bytes Little-Endian)
```
*Độ dài:* 5 bytes.

### 8.2. SubOp 0x07 (Nạp Toàn Bộ Cấu Trúc Dữ Liệu Con Cái 93 Bytes)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x07
+01      uint16 LE   Length        Bắt buộc cố định = 0x005D (93 bytes)
+03      byte[93]    ChildData     Toàn bộ 93 bytes dữ liệu cấu trúc con cái
  ↳ +03  uint8       State         Byte đầu tiên: 0x01 (chưa sinh), 0x02 (đã sinh / active), 0x03 (trưởng thành)
```
*Độ dài:* 96 bytes.

### 8.3. SubOp 0x08 (Cập Nhật Điểm Trưởng Thành)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x08
+01      uint16 LE   Maturity      Điểm trưởng thành con cái (nếu > 399 chuyển State = 3)
```
*Độ dài:* 3 bytes.

### 8.4. Các SubOp Khác (0x01, 0x02, 0x03, 0x04, 0x06, 0x09, 0x0A, 0x0B)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         Mã SubOp tương ứng
+01..    bytes       Payload       Dữ liệu tham số truyền vào hàm xử lý con
```

---

## 9. Ma Trận Kiểm Thử / Hướng Dẫn Mock Server

Dưới đây là danh sách test cases chi tiết để kiểm thử độc lập Mock Server giao tiếp với Client TS Online:

| Test ID | Kịch bản kiểm thử | Wire Payload (Hex) | Kết quả kỳ vọng trên Client |
| :---: | :--- | :--- | :--- |
| **TC-01** | Sinh con / Kích hoạt con cái (`SubOp 0x07`) | `45 07 5D 00 02 [92 bytes 0x00...]` | Client copy 93 bytes vào `TPlayer + 0x151d`. Phát hiện `State = 0x02`, bật popup toast `DAT_0074fc88` trong 2.0s. |
| **TC-02** | Hủy xử lý khi sai Length (`SubOp 0x07`) | `45 07 20 00 02 [31 bytes 0x00...]` | Length = 0x20 (khác 0x5D). Client kiểm tra `Length == 0x5d` thất bại, bỏ qua gói tin mà không gây crash. |
| **TC-03** | Mặc Vũ khí con cái (`SubOp 0x05`, Slot 0x1B) | `45 05 1B 01 12 2F` | Slot `0x1B`, Attr = `0x01`, ItemID = `12050` (`0x2F12`). Cập nhật `+0x155f = 1` và `+0x1553 = 12050`. |
| **TC-04** | Mặc Nón con cái (`SubOp 0x05`, Slot 0x1C) | `45 05 1C 00 E2 32` | Slot `0x1C`, Attr = `0x00`, ItemID = `13026` (`0x32E2`). Cập nhật `+0x1560 = 0` và `+0x1555 = 13026`. |
| **TC-05** | Mặc Phụ kiện con cái (`SubOp 0x05`, Slot 0x20)| `45 05 20 03 64 00` | Slot `0x20`, Attr = `0x03`, ItemID = `100` (`0x0064`). Cập nhật `+0x1564 = 3` và `+0x155d = 100`. |
| **TC-06** | Slot ngoài phạm vi (`SubOp 0x05`) | `45 05 10 01 01 00` | Slot `0x10` không khớp switch (`0x1B..0x20`). Client bỏ qua an toàn, không ghi đè dữ liệu. |
| **TC-07** | Tăng điểm trưởng thành chưa vượt ngưỡng | `45 08 FA 00` | Maturity = 250 (`0x00FA`). Ghi `+0x1565 = 250`. Chưa vượt 399 nên giữ nguyên trạng thái hiện tại. |
| **TC-08** | Tiến hóa sang trạng thái Trưởng thành | `45 08 90 01` | Maturity = 400 (`0x0190`). Ghi `+0x1565 = 400`. Đạt điều kiện `400 > 399`, client tự động gán `+0x151d = 3`. |

---

## 10. Bảng Đối Chiếu Nguồn Sơ Cấp (`ts_decompile/` only)

| Thành phần | Đường dẫn tệp trong `ts_decompile/` | Vị trí / Dòng |
| :--- | :--- | :--- |
| **Case 61 Function** | [`ts_decompile/case_functions/functions/case_061_00795F82_FUN_00795f82.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/functions/case_061_00795F82_FUN_00795f82.c) | Dòng 1–93 |
| **Main Dispatcher** | [`ts_decompile/functions/0078a89c_FUN_0078a89c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.c) | Dòng 7317–7372 (`case 0x45`) |
| **SubOp 5 Handler** | [`ts_decompile/functions/0074f034_FUN_0074f034.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f034_FUN_0074f034.c) | Dòng 21–95 (Trang bị Slots 0x1B..0x20) |
| **SubOp 7 Handler** | [`ts_decompile/functions/0074fb94_FUN_0074fb94.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074fb94_FUN_0074fb94.c) | Dòng 23–66 (Nạp 93 bytes vào `+0x151d` & Toast sinh con) |
| **SubOp 8 Handler** | [`ts_decompile/functions/0074efac_FUN_0074efac.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074efac_FUN_0074efac.c) | Dòng 20–50 (Độ trưởng thành `+0x1565` & Chuyển Adult) |
| **Child Skill Forget Handler** | [`ts_decompile/functions/0074f504_FUN_0074f504.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0074f504_FUN_0074f504.c) | Dòng 19–109 (Kiểm tra 3 bộ kỹ năng & Dialog xác nhận) |
| **Child Attribute Sync** | [`ts_decompile/functions/005b6dd0_FUN_005b6dd0.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/005b6dd0_FUN_005b6dd0.c) | Dòng 18–34 (Đồng bộ trang bị `+0x1553..+0x155d` vào thuộc tính chiến đấu) |
| **Child Render & Follow Logic** | [`ts_decompile/functions/00750278_FUN_00750278.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00750278_FUN_00750278.c) | Dòng 74–95 (Kiểm tra `+0x151d == 2` để vẽ con cái đi theo) |
| **TPlayer Creation** | [`ts_decompile/functions/00716370_TPlayer.Create.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00716370_TPlayer.Create.c) | Dòng 71–98 (Khởi tạo mảng 5 `TFollowNpc` tại `+0x57c`) |
| **Child Form Creation** | [`ts_decompile/functions/0051189c_FUN_0051189c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051189c_FUN_0051189c.c) | Dòng 1560–1562 (`TSe_ChildForm` @ `gvar_007DA75C`) |
| **Child Skill Delete Form** | [`ts_decompile/functions/0051189c_FUN_0051189c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051189c_FUN_0051189c.c) | Dòng 1479–1481 (`TLH_ChildSkillDelForm` @ `gvar_007D9E3C`) |
| **Follow NPC Manager** | [`ts_decompile/functions/0051189c_FUN_0051189c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051189c_FUN_0051189c.c) | Dòng 1389–1390 (`TFNpcManage` @ `gvar_007D9E00`) |
| **NPC Type = 4 Check** | [`ts_decompile/functions/0076d578_FUN_0076d578.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0076d578_FUN_0076d578.c) | Dòng 663 (`*(Npc + 0x56c) == 4` là con cái) |
| **Word LE Decoder** | [`ts_decompile/functions/0077eb9c_FUN_0077eb9c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077eb9c_FUN_0077eb9c.c) | Dòng 166–180 |
| **C → S Dispatcher** | [`ts_decompile/functions/0077f414_FUN_0077F414.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077f414_FUN_0077F414.c) | Dòng 1070–1071 (`case 0x45: break;`) |
