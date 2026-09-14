# PHÂN TÍCH — Main OP 0x40 (64) / Case 57 / FUN_00795c7b @ 0x00795C7B

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). HOLE `0x00607D73–0x00608904` chứa `func_0x00607d94` (SubOp 0x02) **đã được decompile** (đợt redump 2026-09-14) → §4.2 viết lại theo body, suy đoán cũ bị đính chính.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x40` (thập phân: `64`, ánh xạ tới **Case 57**) phụ trách toàn bộ hệ thống **Thủy Chiến (Sea Warfare / Naval Combat)** giữa các chiến thuyền trên mặt nước:
1. **Thực thi kỹ năng và tính toán sát thương Thủy chiến (`TWaterManage` - `gvar_007DA2FC`)**:
   - Xử lý các đợt công kích bằng thuyền, tính toán trừ trực tiếp máu thuyền (Ship HP), năng lượng thuyền (Ship SP) và sĩ khí thuyền (Ship Morale) của người chơi bản địa (`gvar_007DA7BC`).
   - Khấu trừ độ bền / HP của các thuyền bè đồng minh và kẻ địch tham chiến trong danh sách thực thể thủy chiến (`gvar_007DA6DC`).
   - Quản lý vòng đời và hiệu ứng hình ảnh/âm thanh của các thực thể chiến thuyền chiến đấu (`TSeaWarfare` / `TLight`) trên mặt nước.
2. **Quản lý Bảng điều khiển Kỹ năng Chiến thuyền (`Panel17` - `gvar_007DA5BC`, thực tế là instance `Tse_MapFrame`, tạo tại `0051189c_FUN_0051189c.c:1600-1601`)**:
   - SubOp `0x02` (đã có body mới): ghi **một DWORD LE từ `RP[1..4]`** vào `+0x70` của object mà panel tham chiếu qua field `self+0x130`, đồng thời **xóa `+0x74` của object đó về 0** (`00607d94_FUN_00607d94.c:37-40`). *Đính chính suy đoán cũ: payload không chứa trạng thái bật/tắt 8 nút kỹ năng.*
3. **Đồng bộ hóa hàng loạt chỉ số Chiến thuyền (`FUN_0074cdd8`)**:
   - Cập nhật định kỳ chỉ số độ bền / HP vỏ tàu cho toàn bộ danh sách 100 chiến thuyền tham gia chiến trường thủy chiến.

- **Chiều giao tiếp**: Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `FUN_0077f414:1062` (`case 0x40: break;`) là rỗng (`break;`). Toàn bộ hành động thủy chiến được Server tính toán và gửi lệnh cập nhật xuống Client.

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x40 (64) → byte_table[0x78A8EE][0x40] = 0x39 (57)
                  → dword_table[0x78A9B6][57] @ 0x0078AA9A = 0x00795C7B
                  → FUN_00795c7b (Case 57)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` (`ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c:86-93`) lấy gói tin từ hàng đợi, tách byte đầu `P[0] = 0x40` làm Opcode, cắt phần còn lại `_LStrCopy(local_c, 2, len - 1)` làm Rest Payload (`local_10`), sau đó gọi `FUN_0078a89c(Self, Opcode, Payload)`.
2. **Dispatcher Inline**: Nằm tại `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7173-7196`:
   ```c
   case 0x40:
     iVar20 = 0;
     iVar21 = local_10;
     puStack_20 = &stack0xfffffffc;
     if (*(int *)(local_10 + -4) == 0) {
       in_stack_ffffffd4 = (code *)&UNK_00795c8e;
       puStack_20 = &stack0xfffffffc;
       iVar20 = @BoundErr(0);
       iVar21 = extraout_EDX_x00184;
     }
     cVar5 = *(char *)(iVar21 + iVar20); // SubOp = RP[0]
     if (cVar5 == '\x01') {
       in_stack_ffffffd4 = (code *)&UNK_00795cb7;
       FUN_00527674(*(uint *)gvar_007DA2FC, local_10);
     }
     else if (cVar5 == '\x02') {
       in_stack_ffffffd4 = (code *)&UNK_00795ccb;
       func_0x00607d94(*(undefined4 *)gvar_007DA5BC, local_10);
     }
     else if (cVar5 == '\x03') {
       in_stack_ffffffd4 = (code *)&UNK_00795cdf;
       FUN_0074cdd8(*(undefined4 *)gvar_007D9D34, local_10);
     }
     break;
   ```
3. **Case Function độc lập**: `ts_decompile/case_functions/functions/case_057_00795C7B_FUN_00795c7b.c:26-36`.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Tên nghiệp vụ | Đối tượng tiếp nhận | Callee chính | Min Payload Len | Tình trạng SSOT |
| :---: | :--- | :--- | :--- | :---: | :---: |
| **`0x01`** | Thi triển Kỹ năng & Khấu trừ Sát thương Thủy chiến | `*(uint *)gvar_007DA2FC` (`TWaterManage`) | `FUN_00527674` | Action 1: 17B<br>Action 2: 14B | ✔ Đã decompile |
| **`0x02`** | Ghi 1 DWORD vào control của Panel17 (`Tse_MapFrame`) | `*(undefined4 *)gvar_007DA5BC` (Panel17) | `func_0x00607d94` | 5B (`RP[0..4]`) | ✔ **Body mới** (HOLE đã giải) |
| **`0x03`** | Đồng bộ hàng loạt Chỉ số Độ bền / HP Chiến thuyền | `*(undefined4 *)gvar_007D9D34` (World Entity Mgr) | `FUN_0074cdd8` | $1 + 3 \times K$ Bytes | ✔ Đã decompile |

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x01` — Hành Động & Sát Thương Thủy Chiến (`FUN_00527674`)
- **Tệp nguồn**: `ts_decompile/functions/00527674_FUN_00527674.c` (718 dòng).
- **Đối tượng**: `*(uint *)gvar_007DA2FC` (Instance của `TWaterManage`).
- **Phân giải trường cơ sở**:
  + `RP[1]` (`local_36`): **Action Subtype** (Loại hành động thủy chiến: `0x01` hoặc `0x02`).
  + `RP[2]` (`local_1d`): **Combat Mode / Target Flag** (Cờ trạng thái mục tiêu).

#### Nhánh Action Subtype `0x01` — Thiết Lập Thông Số Đợt Tấn Công
- Đọc 3 giá trị 16-bit LE (Word Little-Endian):
  + `RP[3..4]`: Word LE $\rightarrow$ Gán `*(uint *)(TWaterManage + 0xbcc)` (Lượng sát thương HP tạm thời).
  + `RP[5..6]`: Word LE $\rightarrow$ Gán `*(uint *)(TWaterManage + 0xbd0)` (Lượng sát thương SP tạm thời).
  + `RP[7..8]`: Word LE $\rightarrow$ Gán `*(uint *)(TWaterManage + 0xbd4)` (Lượng sát thương Sĩ khí tạm thời).
- Đọc 2 giá trị 32-bit LE (DWORD Little-Endian):
  + `RP[9..12]`: DWORD LE $\rightarrow$ `local_10` (ID thuyền / thực thể nguồn tấn công).
  + `RP[13..16]`: DWORD LE $\rightarrow$ `local_1c` (ID thuyền / thực thể đích bị tấn công).
- Gọi hàm bổ trợ `FUN_0052731c(local_1d, ...)` liên kết cặp thực thể nguồn/đích.
- **Wire Layout**: `[40][01][01][Mode: 1B][HP_Dmg: 2B LE][SP_Dmg: 2B LE][Morale_Dmg: 2B LE][SrcID: 4B LE][DstID: 4B LE]` (17 bytes).

#### Nhánh Action Subtype `0x02` — Thực Thi Kỹ Năng & Trừ Chỉ Số Tàu Thuyền
- Đọc 2 giá trị 32-bit LE:
  + `RP[3..6]`: DWORD LE $\rightarrow$ `local_10` (Attacker ID).
  + `RP[7..10]`: DWORD LE $\rightarrow$ `local_1c` (Target ID).
- Gọi `FUN_00527480()` kiểm tra điều kiện kích hoạt. Nếu hợp lệ:
  + `RP[11..12]`: Word LE $\rightarrow$ `local_38` (Skill ID / Effect ID Thủy chiến).
  + `RP[13]`: 1 byte `local_3c[3]` (Cờ phân bổ sát thương: đơn thể hay đa mục tiêu).
  + **Khấu trừ Sĩ khí (Morale) người chơi bản địa**:
    Nếu `(local_1d == 3 || local_1d == 4)` và `local_10 == LocalPlayer.ID` (`*(int *)(gvar_007DA7BC + 4)`):
    ```c
    *(short *)(Player + 0x50e) -= *(int *)(TWaterManage + 0xbd4);
    ```
    Quét 4 ô kỹ năng tàu thuyền (`Player + 0x509 + slot * 12`), nếu trùng `local_38` thì cập nhật timestamp hồi chiêu `Now()` vào `Player + 0x50c + slot * 12`, đặt cờ `0x514 = 1`, gọi `FUN_0058ed70` làm mới giao diện.
  + **Khấu trừ HP & SP người chơi bản địa**:
    Nếu `(local_1d == 2 || local_1d == 4)` và `local_1c == LocalPlayer.ID`:
    ```c
    *(short *)(Player + 0x502) = max(0, *(short *)(Player + 0x502) - *(int *)(TWaterManage + 0xbcc)); // Trừ HP thuyền
    *(short *)(Player + 0x508) = max(0, *(short *)(Player + 0x508) - *(int *)(TWaterManage + 0xbd0)); // Trừ SP thuyền
    ```
  + **Khấu trừ HP các chiến thuyền khác trong trận**:
    Nếu `(local_1d == 1 || local_1d == 3)`:
    Lấy thực thể thuyền từ mảng `gvar_007DA6DC[local_1c]`:
    ```c
    *(short *)(TargetUnit + 0x3ea) = max(0, *(short *)(TargetUnit + 0x3ea) - *(int *)(TWaterManage + 0xbcc));
    ```
  + **Khởi tạo hiệu ứng thực thể `TSeaWarfare`**:
    * Tìm slot trống trong mảng 250 slot của `TWaterManage` qua `FUN_005265d4`.
    * Cấp phát thực thể hiệu ứng: `TLight_Create(VMT_525E84_TSeaWarfare)`.
    * Thiết lập thời gian duy trì hiệu ứng theo `Now()`.
- **Wire Layout**: `[40][01][02][Mode: 1B][AttackerID: 4B LE][TargetID: 4B LE][SkillID: 2B LE][DistFlag: 1B]` (14 bytes).

---

### 4.2. SubOp `0x02` — Ghi DWORD Vào Control Của Panel17 (`func_0x00607d94`) — **ĐÃ CÓ BODY**

- **Tệp nguồn**: `ts_decompile/functions/00607d94_FUN_00607d94.c` (129 bytes, HOLE `0x00607D73–0x00608904` đã giải).
- **Đối tượng**: `*(undefined4 *)gvar_007DA5BC` — instance **`Tse_MapFrame`** (VMT `0x604620`, tạo tại `0051189c_FUN_0051189c.c:1600-1601`; giao diện tạo tại `00607cb0.c:31` label `"panel17"`).
- **Core logic** (nguyên văn):
  ```c
  void FUN_00607d94(int param_1, int param_2) {   // param_1 = Panel17, param_2 = RestPayload
    local_10 = 0;
    _LStrCopy(local_c, 2, 4, &local_10);          // RP[1..4]
    uVar1 = FUN_0077ef7c(*(gvar_007D9D30), local_10);   // DWORD LE
    *(undefined4 *)(*(int *)(local_8 + 0x130) + 0x70) = uVar1;   // Panel17+0x130 → sub-obj, offset +0x70 := value
    *(undefined4 *)(*(int *)(local_8 + 0x130) + 0x74) = 0;       // offset +0x74 := 0 (reset)
    ...
  }
  ```
  (`00607d94_FUN_00607d94.c:37-40`; `Copy` 1-based index 2 → `RP[1..4]`, tức **bỏ qua byte SubOp `RP[0]`**).
- **Đối tượng con `Panel17 + 0x130`**: field `+0x130` của Panel17 trỏ tới một object khác (đọc tại `00607530.c:47,97` = control có `+0x14`, `+0x18` là counter/limit int; `00607728.c:26` byte `+0x38`). SubOp 0x02 **ghi 1 giá trị DWORD vào `subobj+0x70` và reset `subobj+0x74 := 0`** (cặp field kiểu "current/limit" hoặc "value/elapsed" — **chưa kết luận được** bản chất nghiệp vụ).
- **Điều kiện hiển thị panel**: giữ nguyên ghi chú bản cũ (`Player+0x378 == 2` — kiểm tra dạng này có ở `00603f20_FUN_00603f20.c:117`); điều kiện `+0x145C` của bản cũ **chưa tái kiểm chứng được**, không có citation dòng.
- **Ý nghĩa (đã grounded)**: SubOp 0x02 = "cập nhật một bộ đếm/threshold 32-bit trên control của panel thủy chiến và xóa bộ đếm kèm" — **KHÔNG** "đồng bộ trạng thái 8 nút kỹ năng" (suy đoán cũ, **đính chính**).
- **Wire layout**: `[40][02][Value: 4B LE]` (5 bytes; `Value` ghi vào `subobj+0x70`).

---

### 4.3. SubOp `0x03` — Đồng Bộ Hàng Loạt Thuộc Tính Chiến Thuyền (`FUN_0074cdd8`)
- **Tệp nguồn**: `ts_decompile/functions/0074cdd8_FUN_0074cdd8.c` (113 dòng).
- **Đối tượng**: `*(undefined4 *)gvar_007D9D34` (World Entity Manager).
- **Core Logic giải mã mảng lặp**:
  ```c
  void FUN_0074cdd8(undefined4 param_1, int param_2) {
    ...
    uVar3 = _LStrLen(param_2);
    iVar5 = uVar3 - 1;                           // Độ dài phần dữ liệu sau SubOp
    local_14 = (int)((longlong)iVar5 * 0x55555556 >> 0x20); // Phép chia nguyên cho 3 (K = iVar5 / 3)
    local_18 = 2;                                // Bắt đầu từ byte thứ 2 (1-based Delphi string)
    while (local_14 != 0) {
      local_1c = (uint)*(byte *)(param_2 + local_18 - 1); // Đọc 1 byte: Unit Index (1..100)
      _LStrCopy(param_2, local_18 + 1, 2, &local_28);     // Đọc 2 bytes: Stat Value (WORD LE)
      local_20 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_28);
      
      if (local_1c <= 100 && *(int *)(gvar_007DA6DC + local_1c * 4) != 0) {
        // Cập nhật thuộc tính 0x3ea (HP/Độ bền thuyền)
        *(short *)(*(int *)(gvar_007DA6DC + local_1c * 4) + 0x3ea) = (short)local_20;
      }
      local_18 += 3;
      local_14--;
    }
  }
  ```
- **Ý nghĩa nghiệp vụ**:
  Mỗi phần tử chiếm đúng **3 bytes**: `[UnitIndex: 1B][StatValue: 2B LE]`.
  Server đồng bộ định kỳ lượng HP / Độ bền vỏ tàu hiện tại của toàn bộ tối đa 100 chiến thuyền đang tham chiến trên bản đồ.

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan (Global Objects)

1. **`gvar_007DA2FC` (`TWaterManage`)**:
   - **Constructor**: `00526448_TWaterManage.Create.c` (gọi tại `0050a4a0_TForm1.FormCreate.asm.txt:876-879`).
   - Quản lý mảng 250 con trỏ thực thể hiệu ứng thủy chiến `TSeaWarfare` (`+0x004` đến `+0x3EC`).
   - `+0x3F0`: Số lượng thực thể hiệu ứng đang hoạt động.
   - `+0xBCC`: Lượng sát thương HP tạm thời của đòn đánh thủy chiến.
   - `+0xBD0`: Lượng sát thương SP tạm thời.
   - `+0xBD4`: Lượng giảm sĩ khí/năng lượng chiến thuyền.
2. **`gvar_007DA5BC` (`Panel17`)**:
   - Giao diện thanh công cụ Thủy chiến (Ship Control UI), chứa 8 nút chiêu thức chiến thuyền.
3. **`gvar_007DA7BC` (`TPlayer` - Local Player)**:
   - `+0x378`: Trạng thái phương tiện di chuyển (`0x02` = đang lái chiến thuyền).
   - `+0x502`: Máu chiến thuyền của bản thân (Ship HP, 2B short).
   - `+0x508`: Năng lượng chiến thuyền của bản thân (Ship SP, 2B short).
   - `+0x50e`: Sĩ khí chiến thuyền của bản thân (Ship Morale, 2B short).
   - `+0x509 + slot * 12`: 4 slot kỹ năng tàu thuyền trang bị.
4. **`gvar_007DA6DC`**:
   - Mảng 100 con trỏ tới các thực thể chiến thuyền/đơn vị tham chiến trên bản đồ. Thuộc tính `+0x3ea` lưu trữ HP/độ bền của từng thuyền.

---

## 6. Chiều Client → Server (C → S)

- Kiểm tra tại `ts_decompile/functions/0077f414_FUN_0077F414.c`, dòng 1062:
  ```c
  case 0x40:
    break;
  ```
- **Kết luận**: Nhánh `case 0x40:` hoàn toàn là `break;`. Client **không bao giờ phát sinh gói tin gửi lên máy chủ với Main Opcode 0x40**. Opcode `0x40` là giao thức cập nhật một chiều từ **Server xuống Client (S → C)**.

---

## 7. Chuỗi Hiển Thị, VISCII / CP1258 & Literal Strings

- Không có chuỗi văn bản người dùng đọc được trong toàn bộ cấu trúc payload của Opcode 0x40.
- Các hằng số định danh nội bộ chỉ xuất hiện trong code khởi tạo giao diện điều khiển (`"panel17"`, `"btn_015"` tại `00607cb0.c`).
- Toàn bộ gói tin được truyền dưới dạng các trường số học nhị phân thuần túy: `Byte`, `Word LE`, `DWORD LE`.

---

## 8. Cấu Trúc Rest Payload & Kiểu Dữ Liệu

### 8.1. SubOp 0x01: Action 1 (Khởi tạo sát thương đợt đánh)
```
Offset   Kiểu        Tên trường        Mô tả
+00      uint8       SubOp             0x01
+01      uint8       ActionType        0x01
+02      uint8       CombatMode        Cờ chế độ mục tiêu
+03      uint16 LE   HPDamage          Lượng sát thương HP
+05      uint16 LE   SPDamage          Lượng sát thương SP
+07      uint16 LE   MoraleDamage      Lượng sát thương Sĩ khí
+09      uint32 LE   SourceEntityID    ID thuyền/thực thể tấn công
+13      uint32 LE   TargetEntityID    ID thuyền/thực thể bị tấn công
```
*Độ dài cố định:* 17 bytes.

### 8.2. SubOp 0x01: Action 2 (Thực thi chiêu thức & Khấu trừ chỉ số)
```
Offset   Kiểu        Tên trường        Mô tả
+00      uint8       SubOp             0x01
+01      uint8       ActionType        0x02
+02      uint8       CombatMode        Cờ chế độ mục tiêu
+03      uint32 LE   SourceEntityID    ID thuyền tấn công
+07      uint32 LE   TargetEntityID    ID thuyền bị tấn công
+11      uint16 LE   SkillID           ID chiêu thức Thủy chiến
+13      uint8       DamageDistFlag    Cờ phân bổ sát thương (đơn thể / diện rộng)
```
*Độ dài cố định:* 14 bytes.

### 8.3. SubOp 0x02: Ghi Giá Trị 32-bit Vào Control Panel17
```
Offset   Kiểu        Tên trường        Mô tả
+00      uint8       SubOp             0x02
+01..04  uint32 LE   Value             Ghi vào *(Panel17+0x130)+0x70; +0x74 của cùng object bị reset 0
```
*Độ dài RestPayload:* 5 bytes (cần `len(RP)>=5`, `_LStrCopy(RP,2,4)` lấy nguyên 4 byte — `00607d94_FUN_00607d94.c:37`).

### 8.4. SubOp 0x03: Đồng bộ danh sách chỉ số Chiến thuyền
```
Offset   Kiểu        Tên trường        Mô tả
+00      uint8       SubOp             0x03
--- Lặp lại K lần (mỗi bản ghi chiếm 3 bytes): ---
  +00    uint8       ShipSlotIndex     Vị trí thuyền trong mảng gvar_007DA6DC (1..100)
  +01    uint16 LE   ShipStatValue     Giá trị cập nhật vào Ship[0x3ea] (HP/Độ bền thuyền)
```
*Tổng độ dài:* $1 + 3 \times K$ bytes ($K \ge 0$).

---

## 9. Ma Trận Kiểm Thử / Hướng Dẫn Mock Server

1. **Test Case 1: Đợt đánh thủy chiến gây sát thương (`SubOp 0x01 - Action 1 & 2`)**:
   - Bước 1 (Gửi Action 1): `[40][01][01][Mode: 02][HP: 500 (F4 01)][SP: 100 (64 00)][Morale: 50 (32 00)][SrcID: 1001][DstID: LocalPlayerID]`.
   - Bước 2 (Gửi Action 2): `[40][01][02][Mode: 02][SrcID: 1001][DstID: LocalPlayerID][SkillID: 101 (65 00)][DistFlag: 0]`.
   - Kỳ vọng: `Player + 0x502` giảm 500 HP, `Player + 0x508` giảm 100 SP, hiển thị hoạt cảnh thuyền trúng đòn.
2. **Test Case 2: Đồng bộ độ bền chiến thuyền (`SubOp 0x03`)**:
   - Frame Wire: `F4 44 0A 00 40 03 01 E8 03 02 DC 05` (mã hóa XOR tĩnh 0xAD).
   - Payload giải mã: `[40][03] [Slot 1: 1000 HP] [Slot 2: 1500 HP]`.
   - Kỳ vọng: Cập nhật `gvar_007DA6DC[1] + 0x3ea = 1000` và `gvar_007DA6DC[2] + 0x3ea = 1500`.

---

## 10. Bảng Đối Chiếu Nguồn Sơ Cấp (`ts_decompile/`)

| Thành phần | Đường dẫn tệp | Vị trí / Dòng |
| :--- | :--- | :--- |
| **Case 57 Function** | `ts_decompile/case_functions/functions/case_057_00795C7B_FUN_00795c7b.c` | Dòng 1–66 |
| **Main Dispatcher** | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | Dòng 7173–7196 (`case 0x40`) |
| **SubOp 1 Handler** | `ts_decompile/functions/00527674_FUN_00527674.c` | Dòng 37–715 |
| **SubOp 2 Handler** | `ts_decompile/functions/00607d94_FUN_00607d94.c` | Dòng 16–46 (HOLE `0x607D73–0x608904` đã giải, redump 2026-09-14; `index.csv:6362`) |
| **SubOp 3 Handler** | `ts_decompile/functions/0074cdd8_FUN_0074cdd8.c` | Dòng 23–113 |
| **TWaterManage Create** | `ts_decompile/functions/00526448_TWaterManage.Create.c` | Dòng 21–46 |
| **Slot Allocator** | `ts_decompile/functions/005265d4_FUN_005265d4.c` | Dòng 19–38 |
| **Panel17 Control UI** | `ts_decompile/functions/00607cb0_FUN_00607cb0.c` | Khởi tạo bảng điều khiển thuyền |
| **Word LE Decoder** | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` | Dòng 166–174 |
| **DWord LE Decoder**| `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c` | Dòng 1–252 |
| **C → S Dispatcher**| `ts_decompile/functions/0077f414_FUN_0077F414.c` | Dòng 1062 (`case 0x40: break;`) |
