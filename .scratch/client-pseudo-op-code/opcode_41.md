# PHÂN TÍCH — Main OP 0x41 (65) / Case 58 / FUN_00795ce4 @ 0x00795CE4

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). 3 callee trước đây "khe chưa decompile" (`0x005ba4c8`, `0x0074dbc4`, `0x005bb9f8`) **nay đã có body** (redump 2026-09-14, `index.csv:6347,6349,6475`) — SubOp 0x02 xác nhận suy đoán cũ; SubOp 0x04 và 0x07 **bị đính chính**.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x41` (thập phân: `65`, ánh xạ tới **Case 58**) phụ trách quản lý **Hệ Thống Khí Quan / Cơ Quan (Apparatus System)** và **Cơ Quan Thú Đồng Hành (Pet / Apparatus Companion `TPetNpc`)**:
1. **Quản lý Menu Khí Quan (`TLH_ApparatusMenu` - `gvar_007D9D00`)**:
   - Quản lý trạng thái kích hoạt khí quan của người chơi (bật/tắt cờ `+0x101`, kích hoạt animation bánh răng xoay `"Light_Gear0"`, chuyển đổi chế độ điều khiển và hiển thị các nút thao tác chiến đấu cơ quan: `Btn_Attack`, `Btn_Skill2`, `Btn_Defense`, `Btn_Off_Gray`).
   - Quản lý 3 ô trang bị / vật phẩm cơ quan (`slot 1..3`) với cấu trúc dữ liệu `[Slot: 1B][ItemID: 2B LE][Durability: 2B LE]`, liên kết tra cứu thông số và hình ảnh từ CSDL vật phẩm `gvar_007DA540`.
2. **Quản lý Thú Cơ Quan Đồng Hành Trên Bản Đồ (`TPetNpc` - `Actor + 0x620`)**:
   - Điều khiển vòng đời xuất hiện (Spawn) hoặc biến mất (Despawn) của cơ quan thú đi kèm nhân vật (`TPetNpc`) thông qua Scene World Manager (`gvar_007D9D34`) — **đơn lẻ cho 1 nhân vật (SubOp 0x03) hoặc hàng loạt cho mọi actor khác bản địa (SubOp 0x04, body mới)**.
   - Tự động ánh xạ loại khí quan của nhân vật sang mã hình ảnh sprite / Model ID tương ứng (`40041` đến `40044`) thông qua hàm tra cứu `FUN_005bb3a0`.

- **Chiều giao tiếp**: Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `FUN_0077f414:1064` (`case 0x41: break;`) là rỗng (`break;`). Client không gửi gói tin nào bằng Main Opcode `0x41`.

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x41 (65) → byte_table[0x78A8EE][0x41] = 0x3A (58)
                  → dword_table[0x78A9B6][58] @ 0x0078AA9E = 0x00795CE4
                  → FUN_00795ce4 (Case 58)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` (`ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c:86-93`) lấy gói tin từ hàng đợi, tách byte đầu `P[0] = 0x41` làm Opcode, cắt phần còn lại `_LStrCopy(local_c, 2, len - 1)` làm Rest Payload (`local_10`), sau đó gọi `FUN_0078a89c(Self, Opcode, Payload)`.
2. **Dispatcher Inline**: Nằm tại `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7197-7236`:
   ```c
   case 0x41:
     iVar20 = 0;
     iVar21 = local_10;
     puStack_20 = &stack0xfffffffc;
     if (*(int *)(local_10 + -4) == 0) {
       in_stack_ffffffd4 = (code *)&UNK_00795cf7;
       puStack_20 = &stack0xfffffffc;
       iVar20 = @BoundErr(0);
       iVar21 = extraout_EDX_x00185;
     }
     switch(*(undefined1 *)(iVar21 + iVar20)) { // Byte 0 của Rest Payload: SubOp
     case 1:
       in_stack_ffffffd4 = (code *)&UNK_00795d3f;
       FUN_005ba554(*(int *)gvar_007D9D00);
       break;
     case 2:
       in_stack_ffffffd4 = (code *)&UNK_00795d50;
       func_0x005ba4c8(*(undefined4 *)gvar_007D9D00);
       break;
     case 3:
       in_stack_ffffffd4 = (code *)&UNK_00795d64;
       FUN_0074da00(*(undefined4 *)gvar_007D9D34, local_10);
       break;
     case 4:
       in_stack_ffffffd4 = (code *)&UNK_00795d78;
       func_0x0074dbc4(*(undefined4 *)gvar_007D9D34, local_10);
       break;
     case 5:
       in_stack_ffffffd4 = (code *)&UNK_00795d89;
       FUN_005bb420(*(int **)gvar_007D9D00);
       break;
     case 6:
       in_stack_ffffffd4 = (code *)&UNK_00795d9d;
       FUN_005bb43c(*(int *)gvar_007D9D00, local_10);
       break;
     case 7:
       in_stack_ffffffd4 = (code *)&UNK_00795db1;
       func_0x005bb9f8(*(undefined4 *)gvar_007D9D00, local_10);
     }
     break;
   ```
3. **Case Function độc lập**: `ts_decompile/case_functions/functions/case_058_00795CE4_FUN_00795ce4.c:26-48`.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire Format | Min Len | Đối Tượng Tiếp Nhận | Callee & Trạng Thái SSOT | Ý Nghĩa Nghiệp Vụ Cốt Lõi |
| :---: | :--- | :---: | :--- | :--- | :--- |
| **`0x01`** | `[41][01]` | 2B | `gvar_007D9D00` (`TLH_ApparatusMenu`) | `FUN_005ba554` (✔ Đã decompile) | **Bật trạng thái khí quan**: Đặt cờ `+0x101 = 1`, kích hoạt hiệu ứng bánh răng xoay `"Light_Gear0"`. |
| **`0x02`** | `[41][02]` | 2B | `gvar_007D9D00` (`TLH_ApparatusMenu`) | `FUN_005ba4c8` (**✔ body mới**) | **Tắt trạng thái khí quan** — **xác minh được từ body mới**: `self+0x101:=0`, `self+0xF4:=0`, `FUN_005baa1c` (dừng gear, đối ứng `FUN_005ba944` của SubOp 1), flag ngoại bộ `[gvar_007DA32C+0xE8+n*4]+0x10C := 1` (đảo giá trị với SubOp 1) (`005ba4c8_FUN_005ba4c8.c:19-33`). |
| **`0x03`** | `[41][03][CharID:4B][Active:1B]` | 7B | `gvar_007D9D34` (`Scene Actor Mgr`) | `FUN_0074da00` (✔ Đã decompile) | **Spawn / Despawn Pet Khí Quan (`TPetNpc`)**: Quản lý vòng đời thú máy đi kèm tại `Actor + 0x620`. |
| **`0x04`** | `[41][04] + N*[CharID:4B][Act:1B]` | ≥2B | `gvar_007D9D34` (`Scene Actor Mgr`) | `FUN_0074dbc4` (**✔ body mới**) | **Spawn/Despawn pet hàng loạt cho actor KHÁC local** — *đính chính: không phải "trạng thái phụ trợ"*; là bản lặp của SubOp 0x03 (tối thiểu 600 vòng, cap theo `TPlayers+0x5C`; CharID tra ra 0 = local → bỏ qua record) (`0074dbc4_FUN_0074dbc4.c:52-58,74-160`). |
| **`0x05`** | `[41][05]` | 2B | `gvar_007D9D00` (`TLH_ApparatusMenu`) | `FUN_005bb420` (✔ Đã decompile) | **Mở / Toggle Menu Khí Quan**: Gọi phương thức ảo `VMT + 0x20` nếu menu chưa mở. |
| **`0x06`** | `[41][06] + N*[Slot:1B][Item:2B][Dura:2B]` | 2B | `gvar_007D9D00` (`TLH_ApparatusMenu`) | `FUN_005bb43c` (✔ Đã decompile) | **Cập nhật danh sách trang bị khí quan**: Đọc mảng lặp 5B/slot nạp ItemID & Độ bền vào 3 ô trang bị. |
| **`0x07`** | `[41][07][m]` | 2B | `gvar_007D9D00` (`TLH_ApparatusMenu`, **self không dùng**) | `FUN_005bb9f8` (**✔ body mới**) | **Toast 1200ms**: `RP[1]==1` → `DAT_005bba70`, `==2` → `DAT_005bbac4` (chuỗi chưa dump) — *đính chính: không có "phản hồi lệnh" nào ngoài 2 toast* (`005bb9f8_FUN_005bb9f8.c:20-29`). |

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x01` — Bật Trạng Thái Khí Quan (`FUN_005ba554`)
- **Tệp nguồn**: `ts_decompile/functions/005ba554_FUN_005ba554.c` (54 dòng).
- **Mã nguồn C**:
  ```c
  void FUN_005ba554(int param_1) {
    byte bVar1;
    uint uVar2;
    
    *(undefined1 *)(param_1 + 0x101) = 1;         // Đặt cờ kích hoạt trạng thái khí quan
    FUN_005ba944(param_1);                        // Kích hoạt hiệu ứng animation bánh răng "Light_Gear0"
    bVar1 = FUN_00768d94(*(undefined4 *)gvar_007DA0D0, 0xb454, '\0');
    uVar2 = (uint)bVar1;
    if (0x19 < uVar2) uVar2 = _BoundErr(uVar2);
    *(undefined1 *)(*(int *)(*(int *)gvar_007DA32C + 0xe8 + uVar2 * 4) + 0x10c) = 0;
    
    if (*(char *)(param_1 + 0xee) != '\0') {
      FUN_005ba68c(param_1);                      // Đồng bộ tọa độ: copy LocalPlayer tọa độ vào Menu
    }
    if (*(int *)gvar_007DA51C != 0) {
      *(undefined1 *)(*(int *)gvar_007DA51C + 0xeb4) = 1;
      FUN_00595a70(*(int *)gvar_007DA1DC, '\0');
      FUN_00658c30(*(uint *)gvar_007DA51C);
    }
    (**(code **)(*DAT_0094b804 + 0x24))();        // Kích hoạt callback delegate
  }
  ```
- **Ý nghĩa nghiệp vụ**: Thiết lập chế độ cơ quan khí quan cho nhân vật, bắt đầu hiển thị các hiệu ứng bánh răng và chuyển đổi giao diện nút bấm hành vi.

---

### 4.2. SubOp `0x02` — Tắt Trạng Thái Khí Quan (`func_0x005ba4c8`) — **ĐÃ CÓ BODY**

- **Tệp nguồn**: `ts_decompile/functions/005ba4c8_FUN_005ba4c8.c` (139B, `index.csv:6347`).
- **Core logic** (`:19-33`): đúng chiều đối ứng SubOp 1:
  1. Nếu `gvar_007DA51C != 0`: `FUN_00659230(gvar_007DA51C↑)` + `FUN_00595a70(gvar_007DA1DC↑, 0)` (cùng cặp helper đóng/mở HUD như nhánh SubOp 1).
  2. Slot ngoại bộ: `n = FUN_00768d94(gvar_007DA0D0↑, 0xB454, 0)` (bound `≤0x19`); `[gvar_007DA32C↑ + 0xE8 + n*4] + 0x10C := 1` — **đảo ngược** phép gán `= 0` của SubOp 1.
  3. `FUN_005baa1c(self)` (đối ứng `FUN_005ba944` = hiệu ứng gear của SubOp 1 — tên effect cụ thể chưa kiểm trong body này).
  4. **`self+0x101 := 0`** (xóa cờ kích hoạt — **xác minh được từ body mới**, khớp suy đoán cũ) và **`self+0xF4 := 0`** (field phụ thêm, tên chưa biết; lưu ý mảng slot trang bị của SubOp 0x06 nằm `+0xF1..+0xFF`, nên `+0xF4` **có thể trùng** vùng dữ liệu slot — chưa kết luận được).
- **Kết luận**: nhánh "Tắt trạng thái khí quan" **được xác minh từ body mới**.

### 4.2b. SubOp `0x04` — Spawn/Despawn Pet Khí Quan **Hàng Loạt** (`func_0x0074dbc4`) — **ĐÃ CÓ BODY (đính chính)**

- **Tệp nguồn**: `ts_decompile/functions/0074dbc4_FUN_0074dbc4.c` (802B, `index.csv:6475`).
- **Vòng lặp record** từ `RP[1]` (1-based `Copy` idx 2): mỗi record `[CharID:4B LE][Act:1B]`:
  - Điều kiện dừng: số record đã xử lý **vượt** `*(TPlayers↑ + 0x5C)` (counter trong scene mgr) **hoặc** vượt 600 (`0074dbc4...c:52-58` — cap 600 actor).
  - `idx = FUN_0070c20c(TPlayers↑, CharID)`; **idx == 0 (chính là LocalPlayer) → bỏ qua record** (chỉ nhảy 1 byte cờ) — khác SubOp 0x03 vốn map idx 0 về LocalPlayer.
  - Ngược lại áp dụng **chính xác logic vòng đời `TPetNpc` của SubOp 0x03** cho `gvar_007DA300[idx]`: `Act==0` → `FreeAndNil(actor+0x620)`; `Act!=0` → tạo `TPetNpc_Create(VMT_70B4D8)` nếu trống, tra model `FUN_005bb3a0(gvar_007D9D00↑, actor+0x3E9)`, set `pet+0x1c virtual` (model), `pet+0x550 = ownerIdx`, `pet+0x35C = 1`, `pet+0x554 = actor+0xE3` (hướng chủ), `FUN_0074d7d4` đồng bộ vị trí (`:74-160`).
- **Kết luận**: SubOp 0x04 = **"đồng bộ hàng loạt pet khí quan cho mọi actor khác người chơi bản địa"** — *đính chính suy đoán cũ "cập nhật hành động/trạng thái phụ trợ"*.
- **Wire**: `[41][04] + N×[CharID:4B LE][Act:1B]`.

### 4.2c. SubOp `0x07` — Hai Toast Khí Quan (`func_0x005bb9f8`) — **ĐÃ CÓ BODY (đính chính)**

- **Tệp nguồn**: `ts_decompile/functions/005bb9f8_FUN_005bb9f8.c` (112B, `index.csv:6349`).
- Guard `len(RP)<2` → `_BoundErr(1)` (`:22-23`); `RP[1]==1` → toast `(**gvar_007DA084+0x90)(…, &DAT_005bba70, 0x4B0, 0, 0)`; `==2` → `&DAT_005bbac4`; còn lại im lặng (`:25-28`). Self `TLH_ApparatusMenu` **không được đụng tới**.
- *Đính chính*: "thao tác mở rộng / phản hồi kết quả lệnh" — thực tế chỉ là **2 thông báo toast 1200ms** (nội dung chuỗi chưa dump, cần `lit_5bba70/5bbac4.hex`).

---

### 4.3. SubOp `0x03` — Quản Lý Vòng Đời Pet Khí Quan Trên Map (`FUN_0074da00`)
- **Tệp nguồn**: `ts_decompile/functions/0074da00_FUN_0074da00.c` (116 dòng).
- **Core Logic giải mã**:
  ```c
  void FUN_0074da00(undefined4 param_1, int param_2) {
    // param_2 là chuỗi Rest Payload
    _LStrCopy(param_2, 2, 4, &local_20);          // Đọc 4 bytes từ offset 2 (Pascal 1-based)
    local_14 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30, local_20); // CharID (DWORD LE)
    _LStrCopy(param_2, 6, 1, &local_24);          // Đọc 1 byte tại offset 6
    local_d = FUN_0077eaa4(*(undefined4 *)gvar_007D9D30, local_24);  // IsActive (BYTE)
    
    uVar2 = FUN_0070c20c(*(int *)gvar_007D9D34, local_14);           // Tra cứu Actor Index (0..800)
    local_18 = uVar2;
    if (uVar2 == 0) {
      local_1c = *(undefined **)gvar_007DA7BC;    // 0 = Bản thân (Local Player)
    } else {
      if (800 < uVar2) uVar2 = _BoundErr(uVar2);
      local_1c = *(undefined **)(gvar_007DA300 + uVar2 * 4); // Nhân vật khác trên bản đồ
    }
    
    if (local_d == '\0') {
      // IsActive == 0: HỦY PET KHÍ QUAN
      if (*(int *)(local_1c + 0x620) != 0) {
        FreeAndNil((undefined4 *)(local_1c + 0x620)); // Giải phóng thực thể TPetNpc
      }
    } else {
      // IsActive != 0: KHỞI TẠO HOẶC CẬP NHẬT PET KHÍ QUAN
      if (*(int *)(local_1c + 0x620) == 0) {
        piVar3 = TPetNpc_Create((int *)VMT_70B4D8_TPetNpc, '\x01', -1, local_18);
        *(int **)(local_1c + 0x620) = piVar3;     // Lưu con trỏ TPetNpc vào Actor + 0x620
      }
      if (*(char *)(*(int *)(local_1c + 0x620) + 0x35c) == '\0') {
        // Tra Model ID qua FUN_005bb3a0 theo loại khí quan local_1c[0x3e9]:
        // 1 -> 40041, 2 -> 40042, 3 -> 40043, 4 -> 40044
        local_10 = FUN_005bb3a0(*(undefined4 *)gvar_007D9D00, local_1c[0x3e9]);
        (**(code **)(**(int **)(local_1c + 0x620) + 0x1c))(*(int **)(local_1c + 0x620), local_10, 0); // Đặt Model Sprite
        *(undefined1 *)(*(int *)(local_1c + 0x620) + 0x35c) = 1;
        *(uint *)(*(int *)(local_1c + 0x620) + 0x550) = local_18;           // Gán Owner Actor Index
        *(uint *)(*(int *)(local_1c + 0x620) + 0x554) = (uint)(byte)local_1c[0xe3]; // Gán hướng nhìn của Chủ sở hữu
        FUN_0074d7d4(*(int *)(local_1c + 0x620), ...);                      // Đồng bộ vị trí
      } else {
        FUN_0074d7d4(*(int *)(local_1c + 0x620), ...);
      }
    }
  }
  ```
- **Ý nghĩa nghiệp vụ**:
  Quản lý việc xuất hiện hoặc thu hồi thú máy cơ quan / xe nỏ mini theo sau nhân vật trên bản đồ.

---

### 4.4. SubOp `0x05` — Mở Menu Khí Quan (`FUN_005bb420`)
- **Tệp nguồn**: `ts_decompile/functions/005bb420_FUN_005bb420.c` (25 dòng).
- **Mã nguồn C**:
  ```c
  void FUN_005bb420(int *param_1) {
    if ((char)param_1[5] == '\0') {               // Kiểm tra cờ tại offset +0x14
      (**(code **)(*param_1 + 0x20))();           // Gọi phương thức ảo VMT + 0x20 (Show / Toggle Menu)
    }
    return;
  }
  ```
- **Ý nghĩa nghiệp vụ**: Máy chủ gửi lệnh mở bảng giao diện quản lý trang bị khí quan trên màn hình người chơi.

---

### 4.5. SubOp `0x06` — Cập Nhật Danh Sách Trang Bị Khí Quan (`FUN_005bb43c`)
- **Tệp nguồn**: `ts_decompile/functions/005bb43c_FUN_005bb43c.c` (96 dòng).
- **Core Logic giải mã mảng lặp**:
  ```c
  void FUN_005bb43c(int param_1, int param_2) {
    ...
    uVar3 = _LStrLen(param_2);
    iVar5 = uVar3 - 1;                           // Độ dài sau SubOp
    // Chia cho 5.0 (hằng số dấu phẩy động tại 0x005bb5b8)
    local_10 = (int)ROUND((double)(iVar5) / 5.0); 
    local_14 = 2;                                // Bắt đầu từ offset 2 (Pascal 1-based)
    while (local_10 != 0) {
      local_18 = (uint)*(byte *)(param_2 + local_14 - 1); // Đọc 1 byte: SlotIndex (1..3)
      _LStrCopy(param_2, local_14 + 1, 2, &local_24);     // Đọc 2 bytes: ItemID (Word LE)
      uVar3 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_24);
      _LStrCopy(param_2, local_14 + 3, 2, &local_28);     // Đọc 2 bytes: Durability (Word LE)
      uVar4 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_28);
      
      if (local_18 - 1 < 3) {
        // Ghi vào mảng 3 slot của TLH_ApparatusMenu
        *(short *)(param_1 + 0xf1 + local_18 * 4) = (short)uVar3; // ItemID
        *(short *)(param_1 + 0xf3 + local_18 * 4) = (short)uVar4; // Durability / Count
      }
      local_14 += 5;
      local_10--;
    }
    FUN_005bb5bc(param_1);                       // Làm mới hiển thị icon và thông số từ CSDL vật phẩm
  }
  ```
- **Ý nghĩa nghiệp vụ**:
  Mỗi bản ghi trang bị khí quan chiếm đúng **5 bytes**: `[Slot: 1B][ItemID: 2B LE][Durability: 2B LE]`.
  Cập nhật thông số và độ bền cho 3 ô trang bị khí quan của người chơi:
  + Slot 1: `+0xF5` (ItemID), `+0xF7` (Durability)
  + Slot 2: `+0xF9` (ItemID), `+0xFB` (Durability)
  + Slot 3: `+0xFD` (ItemID), `+0xFF` (Durability)

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan (Global Objects)

| Biến toàn cục | Kiểu dữ liệu / Lớp | VMT / Khởi tạo | Vai trò nghiệp vụ |
| :--- | :--- | :--- | :--- |
| `gvar_007D9D00` | `TLH_ApparatusMenu` | `VMT_5B7FD0` @ `0051189c.c:1774` | Quản trị viên giao diện và trang bị Khí Quan. |
| `gvar_007D9D34` | `TPlayers` | `Scene Actor Manager` | Quản lý thực thể nhân vật và vòng đời pet `TPetNpc`. |
| `gvar_007DA7BC` | `TPlayer` | `Local Player Instance` | Thực thể người chơi bản địa (`+0x620` con trỏ `TPetNpc`). |
| `gvar_007DA300` | `TPlayer[800]` | Mảng 800 Actor | Danh sách thực thể người chơi/NPC trên bản đồ. |
| `gvar_007DA540` | `TItemTable` | Bảng CSDL Vật phẩm | Bản ghi $0xB9$ bytes/item; tra cứu tên, icon, thông số vật phẩm. |
| `gvar_007D9D30` | `TPacketDecoder` | Tiện ích giải mã | Cung cấp các hàm giải mã nhị phân Word LE, DWORD LE. |

---

## 6. Chiều Client → Server (C → S)

- Kiểm tra tại `ts_decompile/functions/0077f414_FUN_0077F414.c`, dòng 1064:
  ```c
  case 0x41:
    break;
  ```
- **Kết luận**: Nhánh `case 0x41:` hoàn toàn là `break;`. Client **không bao giờ gửi gói tin nào mang Opcode 0x41** lên server. Opcode `0x41` là giao thức cập nhật một chiều từ **Server xuống Client (S → C)**.

---

## 7. Chuỗi Hiển Thị, VISCII / CP1258 & Literal Strings

- Toàn bộ gói tin truyền trên đường truyền mạng cho Opcode 0x41 không chứa chuỗi văn bản người dùng (không có VISCII hay CP1258).
- Các định danh nội bộ trong mã nguồn gồm tên hoạt cảnh và sprite giao diện:
  + `"Light_Gear0"` (`005ba944.c:41`): Sprite hiệu ứng bánh răng khí quan phát sáng xoay tròn.
  + `"Btn_Off_Gray"`, `"Btn_Attack"`, `"Btn_Skill2"`, `"Btn_Defense"` (`005ba21c.c`): Tên các sprite nút bấm thao tác trên thanh menu khí quan.
- Tên vật phẩm và mô tả chi tiết của trang bị khí quan được nạp động từ CSDL `gvar_007DA540` theo `ItemID`.
- **Bổ sung từ body mới (2026-09-14)**: SubOp 0x07 tham chiếu 2 hằng AnsiString `DAT_005bba70`, `DAT_005bbac4` — **chưa có trong `redump/`**, cần `lit_5bba70/5bbac4.hex` để decode VISCII; không có tên tĩnh nào khác trong 3 body mới.

---

## 8. Cấu Trúc Rest Payload & Kiểu Dữ Liệu

### 8.1. SubOp 0x01 (Bật Trạng Thái Khí Quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x01
```
*Độ dài:* 1 byte.

### 8.2. SubOp 0x02 (Tắt Trạng Thái Khí Quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x02
```
*Độ dài:* 1 byte.

### 8.3. SubOp 0x03 (Spawn / Despawn Pet Khí Quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x03
+01      uint32 LE   CharID        ID nhân vật sở hữu (4 bytes Little-Endian)
+05      uint8       IsActive      Trạng thái (0: Despawn/Free; 1: Spawn/Update TPetNpc)
```
*Độ dài cố định:* 6 bytes.

### 8.4. SubOp 0x05 (Mở Giao Diện Menu Khí Quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x05
```
*Độ dài:* 1 byte.

### 8.5. SubOp 0x06 (Cập Nhật Danh Sách Trang Bị Khí Quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x06
--- Mảng lặp N phần tử (mỗi phần tử 5 bytes): ---
  +00    uint8       SlotIndex     Chỉ số ô trang bị (1..3)
  +01    uint16 LE   ItemID        Mã vật phẩm khí quan (Word Little-Endian)
  +03    uint16 LE   Durability    Độ bền / số lượng còn lại (Word Little-Endian)
```
*Tổng độ dài:* $1 + 5 \times N$ bytes ($N \ge 0$).

### 8.6. SubOp 0x04 (Spawn/Despawn Pet hàng loạt — actor khác local)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x04
--- Mảng lặp record, mỗi record 5 bytes (dừng khi vượt counter TPlayers+0x5C hoặc 600 record): ---
  +00    uint32 LE   CharID        ID nhân vật (tra actor index)
  +04    uint8       IsActive      0: FreeAndNil TPetNpc; ≠0: create/update TPetNpc
```

### 8.7. SubOp 0x07 (Toast khí quan)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x07
+01      uint8       MsgCode       1 → toast @0x5BBA70; 2 → toast @0x5BBAC4 (1200ms); khác: im lặng
```
*Độ dài:* 2 bytes RestPayload.

---

## 9. Ma Trận Kiểm Thử / Hướng Dẫn Mock Server

1. **Test Case 1: Xuất hiện Pet Khí Quan (`SubOp 0x03 - Spawn`)**:
   - Frame Wire: `F4 44 08 00 41 03 E8 03 00 00 01` (mã hóa XOR tĩnh 0xAD).
   - Payload giải mã: `[41][03][CharID: 1000 (E8 03 00 00)][IsActive: 1]`.
   - Kỳ vọng: Khởi tạo thực thể `TPetNpc` tại `Actor + 0x620`, tải sprite `40041`, thú máy cơ quan xuất hiện đi sau nhân vật.
2. **Test Case 2: Thu hồi Pet Khí Quan (`SubOp 0x03 - Despawn`)**:
   - Frame Wire: `F4 44 08 00 41 03 E8 03 00 00 00` (XOR 0xAD).
   - Payload giải mã: `[41][03][CharID: 1000][IsActive: 0]`.
   - Kỳ vọng: Gọi `FreeAndNil` giải phóng `TPetNpc`, thú máy biến mất khỏi màn hình.
3. **Test Case 3: Cập nhật trang bị khí quan (`SubOp 0x06`)**:
   - Frame Wire: `F4 44 0D 00 41 06 01 20 4E 64 00 02 21 4E C8 00` (XOR 0xAD).
   - Payload giải mã:
     + Slot 1: `ItemID = 20000 (0x4E20)`, `Durability = 100 (0x0064)`.
     + Slot 2: `ItemID = 20001 (0x4E21)`, `Durability = 200 (0x00C8)`.
   - Kỳ vọng: Bảng menu khí quan hiển thị đúng icon và thanh độ bền tương ứng cho slot 1 và slot 2.

---

## 10. Bảng Đối Chiếu Nguồn Sơ Cấp (`ts_decompile/`)

| Thành phần | Đường dẫn tệp | Vị trí / Dòng |
| :--- | :--- | :--- |
| **Case 58 Function** | `ts_decompile/case_functions/functions/case_058_00795CE4_FUN_00795ce4.c` | Dòng 1–81 |
| **Main Dispatcher** | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | Dòng 7197–7236 (`case 0x41`) |
| **SubOp 1 Handler** | `ts_decompile/functions/005ba554_FUN_005ba554.c` | Dòng 21–53 |
| **SubOp 2 Handler** | `ts_decompile/functions/005ba4c8_FUN_005ba4c8.c` | Dòng 16–33 (body mới, `index.csv:6347`) |
| **SubOp 3 Handler** | `ts_decompile/functions/0074da00_FUN_0074da00.c` | Dòng 23–115 |
| **SubOp 4 Handler** | `ts_decompile/functions/0074dbc4_FUN_0074dbc4.c` | Dòng 44–170 (body mới, `index.csv:6475`) |
| **SubOp 7 Handler** | `ts_decompile/functions/005bb9f8_FUN_005bb9f8.c` | Dòng 16–29 (body mới, `index.csv:6349`) |
| **SubOp 5 Handler** | `ts_decompile/functions/005bb420_FUN_005bb420.c` | Dòng 20–25 |
| **SubOp 6 Handler** | `ts_decompile/functions/005bb43c_FUN_005bb43c.c` | Dòng 23–96 |
| **Tra Model Pet** | `ts_decompile/functions/005bb3a0_FUN_005bb3a0.c` | Ánh xạ ID `40041..40044` |
| **Khởi Tạo Menu** | `ts_decompile/functions/0051189c_FUN_0051189c.c` | Dòng 1774–1776 (`TLH_ApparatusMenu`) |
| **Word LE Decoder** | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` | Dòng 166–174 |
| **DWord LE Decoder**| `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c` | Dòng 1–252 |
| **C → S Dispatcher**| `ts_decompile/functions/0077f414_FUN_0077F414.c` | Dòng 1064 (`case 0x41: break;`) |
