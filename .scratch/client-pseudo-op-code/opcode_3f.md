# PHÂN TÍCH — Main OP 0x3F (63) / Case 56 / FUN_00795ac4 @ 0x00795AC4

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh 100% từ mã nguồn sơ cấp** (`ts_decompile/` only).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x3F` (thập phân: `63`, ánh xạ tới **Case 56**) phụ trách hai phân hệ chiến đấu đặc thù trong game:
1. **Hệ thống Lôi Đài / Đấu Trường Võ Thuật (`TMRBatterManage` & `TPlayers` / Scene Actor Manager)**:
   - Tiếp nhận và phát các thông báo giải đấu lôi đài, cập nhật hệ ngũ hành áp dụng cho trận đấu (Địa, Thủy, Hỏa, Phong), thông báo tên NPC phụ trách lôi đài, mức tiền cược thi đấu (5.000 lượng hoặc 10.000 lượng) và danh tính tuyển thủ tham chiến.
   - Đồng bộ hóa trực tiếp các chỉ số / trạng thái thi đấu lôi đài vào cấu trúc dữ liệu của người chơi bản địa (`Local Player` - `gvar_007DA7BC` tại offset `+0x1464` và `+0x1468`).
   - Quản lý giao diện bảng trợ giúp và thông tin quy tắc thi đấu lôi đài (`TMR_BattleHelp` - `gvar_007DA7E0`).
2. **Hệ thống Chiến Trường Thủy Chiến (Water Battle - `TMR_WaterBattleManage` & `TMR_WBManageOrgManage`)**:
   - Quản lý trạng thái khởi tạo, tiến trình và kết thúc trận thủy chiến (`TMR_WaterBattleManage` - `gvar_007DA27C`).
   - Quản lý cơ cấu tổ chức, phân bổ phe phái / quân đoàn tham gia thủy chiến (`TMR_WBManageOrgManage` - `gvar_007DA164`).
   - Quản lý bảng tin và giao diện hướng dẫn thủy chiến (`TMR_WaterBattleHelp` - `gvar_007D9F10`).

- **Chiều giao tiếp**: Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `FUN_0077f414:1060` (`case 0x3f: break;`) là rỗng. Mọi tương tác của người chơi với lôi đài/thủy chiến được thực hiện qua các gói tin hành động NPC hoặc lệnh click giao diện thông thường.

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x3F (63) → byte_table[0x78A8EE][0x3F] = 0x38 (56)
                  → dword_table[0x78A9B6][56] @ 0x0078AA96 = 0x00795AC4
                  → FUN_00795ac4 (Case 56)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` (`ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c:86-93`) lấy gói tin từ hàng đợi, tách byte đầu `P[0] = 0x3F` làm Opcode, cắt phần còn lại `_LStrCopy(local_c, 2, len - 1)` làm Rest Payload (`local_10`), sau đó gọi `FUN_0078a89c(Self, Opcode, Payload)`.
2. **Dispatcher Inline**: Nằm tại `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7101-7172`.
3. **Case Function độc lập**: `ts_decompile/case_functions/functions/case_056_00795AC4_FUN_00795ac4.c`.

### 2.2. Kiểm tra độ dài & Đọc SubOp
```c
iVar2 = *(int *)(unaff_EBP + -0xc); // RestPayload (RP)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {     // Kiểm tra Length(RP) == 0
  iVar1 = _BoundErr(0);              // Báo lỗi biên mảng (Range Error) nếu gói tin chỉ có byte 0x3F
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // SubOp = RP[0]
```
- `RP[0]` (byte thứ 2 của toàn bộ frame payload: `P[1]`) đóng vai trò là **SubOp**.
- Sau đó hàm thực hiện lệnh `switch(*(undefined4 *)(unaff_EBP + -0x14))` để điều phối đến 15 nhánh xử lý tương ứng. Không có nhánh `default`; các giá trị SubOp không định nghĩa sẽ bị bỏ qua im lặng.

---

## 3. Bảng Tổng Hợp Toàn Bộ SubOp

| SubOp (Hex) | SubOp (Dec) | Đối Tượng Đích (`param_1`) | Hàm Callee | Min Len | Tình trạng SSOT | Ý Nghĩa Nghiệp Vụ Cốt Lõi |
| :---: | :---: | :--- | :--- | :---: | :---: | :--- |
| **`0x01`** | 1 | `gvar_007D9DFC` (`TMRBatterManage`) | `FUN_0054c348` | 2B | ✔ Đã decompile | Quản lý 8 loại sự kiện/thông báo võ đài (ngũ hành, NPC, tiền cược 5k/10k, tuyển thủ). |
| **`0x02`** | 2 | `gvar_007D9DFC` (`TMRBatterManage`) | `func_0x0054cbc0` | ≥1B | ✘ Khe chưa decompile | Cập nhật dữ liệu danh sách/bảng thi đấu võ đài (`TMRBatterManage`). |
| **`0x03`** | 3 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `func_0x00747ca4` | ≥1B | ✘ Khe chưa decompile | Đăng ký / thêm đối thủ lôi đài vào danh sách thực thể Scene. |
| **`0x04`** | 4 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `func_0x00747cdc` | ≥1B | ✘ Khe chưa decompile | Xóa / gỡ bỏ đối thủ lôi đài khỏi thực thể Scene. |
| **`0x05`** | 5 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `FUN_00747d50` | 5B | ✔ Đã decompile | Đọc DWORD LE, cập nhật chỉ số lôi đài vào `LocalPlayer + 0x1468`. |
| **`0x06`** | 6 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `FUN_00747dc4` | 5B | ✔ Đã decompile | Đọc DWORD LE, cập nhật trạng thái lôi đài vào `LocalPlayer + 0x1464`. |
| **`0x07`** | 7 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `func_0x00748110` | ≥1B | ✘ Khe chưa decompile | Thiết lập cờ trạng thái chiến đấu võ đài cho Scene Actor. |
| **`0x08`** | 8 | `gvar_007DA7E0` (`TMR_BattleHelp`) | `func_0x005581cc` | ≥1B | ✘ Khe chưa decompile | Cập nhật nội dung bảng giao diện Trợ giúp Võ đài. |
| **`0x0A`** | 10 | `gvar_007D9D34` (`TPlayers` - Scene Mgr) | `func_0x0074c5bc` | ≥1B | ✘ Khe chưa decompile | Đồng bộ điểm trận / thời gian thi đấu lôi đài trong Scene. |
| **`0x0B`** | 11 | `gvar_007DA27C` (`TMR_WaterBattleManage`) | `func_0x0054a4f8` | ≥1B | ✘ Khe chưa decompile | Khởi tạo / cập nhật dữ liệu chiến trường Thủy chiến. |
| **`0x0C`** | 12 | `gvar_007DA27C` (`TMR_WaterBattleManage`) | `func_0x0054aaa0` | ≥1B | ✘ Khe chưa decompile | Đồng bộ trạng thái kết thúc / điểm số chiến trường Thủy chiến. |
| **`0x0D`** | 13 | `gvar_007DA164` (`TMR_WBManageOrgManage`) | `func_0x0054b5e8` | ≥1B | ✘ Khe chưa decompile | Quản lý danh sách phe phái / bang hội tham gia Thủy chiến. |
| **`0x14`** | 20 | `gvar_007DA164` (`TMR_WBManageOrgManage`) | `func_0x0054b6d0` | ≥1B | ✘ Khe chưa decompile | Cập nhật phân bổ vị trí thành viên quân đoàn Thủy chiến. |
| **`0x15`** | 21 | `gvar_007D9F10` (`TMR_WaterBattleHelp`) | `func_0x00558d80` | ≥1B | ✘ Khe chưa decompile | Cập nhật / hiển thị bảng hướng dẫn & quy tắc Thủy chiến. |
| **`0x16`** | 22 | `gvar_007D9F10` (`TMR_WaterBattleHelp`) | `func_0x00558d1c` | ≥1B | ✘ Khe chưa decompile | Đóng / ẩn giao diện Hướng dẫn Thủy chiến. |

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x01` — Quản Lý Thông Báo & Sự Kiện Võ Đài (`FUN_0054c348`)
- **Tệp nguồn**: `ts_decompile/functions/0054c348_FUN_0054c348.c`
- **Đối tượng**: `*(undefined4 *)gvar_007D9DFC` (Instance `TMRBatterManage`).
- **Cơ chế phân nhánh Sub-SubOp**:
  Đọc byte tại `RP[1]` (tức `P[2]`): `cVar1 = *(char *)(iVar10 + 1)`.
  Phân nhánh thành 8 trường hợp thông báo võ đài:

#### 1. Sub-SubOp `0x01` — Thông Báo Bắt Đầu Lôi Đài
- Gửi chuỗi thông báo hệ thống tại `&DAT_0054c994` vào khung chat:
  ```c
  FUN_007ab870(*(int *)gvar_007DA1B0, 0, &DAT_0054c994, 0);
  ```
- **Payload Wire**: `[3F][01][01]` (3 bytes).

#### 2. Sub-SubOp `0x02` — Thông Báo Trạng Thái Lôi Đài
- Gửi chuỗi thông báo hệ thống tại `&DAT_0054c9ec` vào khung chat:
  ```c
  FUN_007ab870(*(int *)gvar_007DA1B0, 0, &DAT_0054c9ec, 0);
  ```
- **Payload Wire**: `[3F][01][02]` (3 bytes).

#### 3. Sub-SubOp `0x03` — Cập Nhật Hệ Ngũ Hành Lôi Đài
- **Cấu trúc trường**:
  + `RP[2]` (`local_d`): Mã thuộc tính ngũ hành ($0..3$: 0 = Địa, 1 = Thủy, 2 = Hỏa, 3 = Phong).
  + `RP[3]` (`local_e`): Cờ loại thông báo (1 hoặc 2).
- **Core Logic**:
  + Tra tên hệ ngũ hành từ mảng chuỗi toàn cục `DAT_009490ac` (mỗi mục cách nhau $0x55 = 85$ bytes).
  + Nếu `local_e == 1`: Ghép chuỗi `DAT_009490ac[local_d]` với `&DAT_0054ca58`.
  + Nếu `local_e == 2`: Ghép chuỗi `DAT_009490ac[local_d]` với `&DAT_0054ca70`.
  + Gửi kết quả hoàn chỉnh qua `FUN_007ab870`.
- **Payload Wire**: `[3F][01][03][Element: 1B][Flag: 1B]` (5 bytes).

#### 4. Sub-SubOp `0x04` — Thông Báo Đặc Tính Võ Đài Ngũ Hành
- **Cấu trúc trường**:
  + `RP[2]` (`local_d`): Mã thuộc tính ngũ hành ($0..3$).
- **Core Logic**:
  + Ghép chuỗi định danh: `&DAT_0054caa0` + `&DAT_0054ca84` + Tên hệ ngũ hành (`DAT_009490ac[local_d]`).
  + Gửi thông báo qua `FUN_007ab870`.
- **Payload Wire**: `[3F][01][04][Element: 1B]` (4 bytes).

#### 5. Sub-SubOp `0x05` — Ghép NPC Võ Đài Với Thuộc Tính Ngũ Hành
- **Cấu trúc trường**:
  + `RP[2]` (`local_d`): Thuộc tính ngũ hành ($0..3$).
  + `RP[3..6]`: 4 bytes nạp vào `_LStrCopy(..., 4, 4)` $\rightarrow$ giải mã `NpcID` (DWORD LE qua `FUN_0077ef7c`).
- **Core Logic**:
  + Tra cứu chỉ mục NPC: `local_1a = FUN_00623f00(*(undefined4 *)gvar_007D9DE4, (ushort)NpcID)`.
  + Đọc tên NPC tại `*(int *)gvar_007D9DE4 + 4 + local_1a * 0x5C`.
  + Ghép chuỗi: `[Tên NPC]` + `&DAT_0054cadc` + `&DAT_0054cabc` + `[Tên hệ ngũ hành]`.
  + Xuất ra kênh thông báo hệ thống.
- **Payload Wire**: `[3F][01][05][Element: 1B][NpcID: 4B LE]` (8 bytes).

#### 6. Sub-SubOp `0x06` — Thông Báo Tiền Cược Lôi Đài (5.000 / 10.000 Lượng)
- **Cấu trúc trường**:
  + `RP[2..5]`: 4 bytes NpcID (DWORD LE qua `FUN_0077ef7c`).
  + `RP[6..7]`: 2 bytes Số tiền cược `local_1c` (WORD LE qua `FUN_0077eb9c`).
- **Core Logic**:
  + Tra tên NPC qua `FUN_00623f00`.
  + Kiểm tra hạn mức đặt cược:
    * Nếu `local_1c == 5000`: Ghép `[Tên NPC]` với chuỗi thông báo cược 5.000 lượng tại `&DAT_0054cae8` (độ dài 61 bytes).
    * Nếu `local_1c == 10000`: Ghép `[Tên NPC]` với chuỗi thông báo cược 10.000 lượng tại `&DAT_0054cb18` (độ dài 51 bytes).
  + Gửi kết quả qua `FUN_007ab870`.
- **Payload Wire**: `[3F][01][06][NpcID: 4B LE][BetAmount: 2B LE]` (9 bytes).

#### 7. Sub-SubOp `0x07` — Thông Báo Đối Thủ Lôi Đài & Hệ Ngũ Hành
- **Cấu trúc trường**:
  + `RP[2]`: Thuộc tính ngũ hành ($0..3$).
  + `RP[3..6]`: 4 bytes NpcID (DWORD LE).
- **Core Logic**:
  + Tra tên NPC, ghép chuỗi: `[Tên NPC]` + `&DAT_0054ca84` + `[Tên hệ ngũ hành]` + `&DAT_0054cb48`.
  + Xuất thông báo.
- **Payload Wire**: `[3F][01][07][Element: 1B][NpcID: 4B LE]` (8 bytes).

#### 8. Sub-SubOp `0x08` — Thông Báo Tuyển Thủ Lôi Đài Tham Chiến
- **Cấu trúc trường**:
  + `RP[2..5]`: 4 bytes `PlayerID` (DWORD LE qua `FUN_0077ef7c`).
- **Core Logic**:
  + Phân biệt người chơi bản địa và người chơi khác:
    * Nếu `PlayerID == *(int *)(*(int *)gvar_007DA7BC + 4)` (Local Player): Lấy tên người chơi bản địa tại `*(int *)gvar_007DA7BC + 9`.
    * Nếu khác Local Player: Gọi `FUN_00722508(*(undefined4 *)gvar_007D9C48, PlayerID)` lấy chỉ số slot trong mảng cache Scene Player `gvar_007DA6BC`, trích xuất tên tại `*(int *)(gvar_007DA6BC + slot * 4) + 8`.
  + Ghép chuỗi hoàn chỉnh: `&DAT_0054cb78` + `[Tên Người Chơi]` + `&DAT_0054cb64`.
  + Xuất ra kênh chat.
- **Payload Wire**: `[3F][01][08][PlayerID: 4B LE]` (7 bytes).

---

### 4.2. SubOp `0x05` — Đồng Bộ Thuộc Tính Lôi Đài `+0x1468` (`FUN_00747d50`)
- **Tệp nguồn**: `ts_decompile/functions/00747d50_FUN_00747d50.c`
- **Mã nguồn đầy đủ**:
  ```c
  void FUN_00747d50(undefined4 param_1, int param_2) {
    uint uVar1;
    undefined4 *in_FS_OFFSET;
    int local_10;
    
    local_10 = 0;
    _LStrCopy(param_2, 2, 4, &local_10); // Cắt 4 bytes từ vị trí 2 (bỏ byte SubOp)
    uVar1 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30, local_10); // Decode DWORD LE
    *(undefined4 *)(*(int *)gvar_007DA7BC + 0x1468) = uVar1; // Cập nhật LocalPlayer + 0x1468
    _LStrClr(&local_10);
    return;
  }
  ```
- **Ý nghĩa**: Cập nhật chỉ số thuộc tính lôi đài (điểm tích lũy / số vòng thắng đấu trường) của nhân vật bản địa.
- **Payload Wire**: `[3F][05][Value: 4B LE]` (6 bytes).

---

### 4.3. SubOp `0x06` — Đồng Bộ Thuộc Tính Lôi Đài `+0x1464` (`FUN_00747dc4`)
- **Tệp nguồn**: `ts_decompile/functions/00747dc4_FUN_00747dc4.c`
- **Mã nguồn đầy đủ**:
  ```c
  void FUN_00747dc4(undefined4 param_1, int param_2) {
    uint uVar1;
    undefined4 *in_FS_OFFSET;
    int local_10;
    
    local_10 = 0;
    _LStrCopy(param_2, 2, 4, &local_10); // Cắt 4 bytes từ vị trí 2
    uVar1 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30, local_10); // Decode DWORD LE
    *(undefined4 *)(*(int *)gvar_007DA7BC + 0x1464) = uVar1; // Cập nhật LocalPlayer + 0x1464
    _LStrClr(&local_10);
    return;
  }
  ```
- **Ý nghĩa**: Cập nhật cờ trạng thái lôi đài / trạng thái báo danh võ đài của nhân vật bản địa.
- **Payload Wire**: `[3F][06][Value: 4B LE]` (6 bytes).

---

### 4.4. Đánh Giá 12 Callee Nằm Trong Khoảng Trống (Gaps)

12 hàm sau không có file mã nguồn rời trong `ts_decompile/functions/` (vắng mặt trong `index.csv`), tuy nhiên ngữ cảnh đối tượng và vai trò nghiệp vụ đã được xác minh đối chiếu qua địa chỉ VMT và Constructor trong `FormCreate` (`0050a4a0.c`) và `LoadingThread` (`0051189c.c`):

1. **`func_0x0054cbc0`**: Thuộc class `TMRBatterManage` (`gvar_007D9DFC`, VMT: `0x0054C1D4`). Khoảng trống `0x54C95A - 0x54D784`. Cập nhật bảng xếp hạng / dữ liệu cặp đấu võ đài.
2. **`func_0x00747ca4`**: Thuộc class `TPlayers` (`gvar_007D9D34`). Khoảng trống `0x747C7D - 0x747D50`. Khởi tạo thực thể đối thủ lôi đài trên Scene.
3. **`func_0x00747cdc`**: Thuộc class `TPlayers` (`gvar_007D9D34`). Khoảng trống `0x747C7D - 0x747D50`. Hủy thực thể đối thủ lôi đài trên Scene.
4. **`func_0x00748110`**: Thuộc class `TPlayers` (`gvar_007D9D34`). Khoảng trống `0x748102 - 0x74927C`. Đặt hiệu ứng / cờ chiến đấu võ đài cho Scene Actor.
5. **`func_0x005581cc`**: Thuộc class `TMR_BattleHelp` (`gvar_007DA7E0`, VMT: `0x00553E84`). Khoảng trống `0x557DFB - 0x558BFC`. Cập nhật nội dung bảng trợ giúp võ đài.
6. **`func_0x0074c5bc`**: Thuộc class `TPlayers` (`gvar_007D9D34`). Khoảng trống `0x74C5AF - 0x74CC84`. Đồng bộ thời gian / điểm số trận võ đài.
7. **`func_0x0054a4f8`**: Thuộc class `TMR_WaterBattleManage` (`gvar_007DA27C`, VMT: `0x0054A0DC`). Khoảng trống `0x54A381 - 0x54AFE0`. Khởi tạo / cập nhật chiến trường Thủy chiến.
8. **`func_0x0054aaa0`**: Thuộc class `TMR_WaterBattleManage` (`gvar_007DA27C`, VMT: `0x0054A0DC`). Khoảng trống `0x54A381 - 0x54AFE0`. Đồng bộ kết quả / trạng thái Thủy chiến.
9. **`func_0x0054b5e8`**: Thuộc class `TMR_WBManageOrgManage` (`gvar_007DA164`, VMT: `0x0054A140`). Khoảng trống `0x54B52D - 0x54B804`. Quản lý danh sách phe phái Thủy chiến.
10. **`func_0x0054b6d0`**: Thuộc class `TMR_WBManageOrgManage` (`gvar_007DA164`, VMT: `0x0054A140`). Khoảng trống `0x54B52D - 0x54B804`. Phân bổ quân số / đơn vị Thủy chiến.
11. **`func_0x00558d80`**: Thuộc class `TMR_WaterBattleHelp` (`gvar_007D9F10`, VMT: `0x00553F70`). Khoảng trống `0x558CF5 - 0x558E20`. Mở và cập nhật giao diện Trợ giúp Thủy chiến.
12. **`func_0x00558d1c`**: Thuộc class `TMR_WaterBattleHelp` (`gvar_007D9F10`, VMT: `0x00553F70`). Khoảng trống `0x558CF5 - 0x558E20`. Đóng giao diện Trợ giúp Thủy chiến.

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan (Global Objects)

| Biến toàn cục | Kiểu dữ liệu / Lớp | VMT / Khởi tạo | Vai trò nghiệp vụ |
| :--- | :--- | :--- | :--- |
| `gvar_007D9DFC` | `TMRBatterManage` | `VMT_54C1D4` @ `0050a4a0.c:633` | Quản lý logic đấu trường / võ đài / lôi đài. |
| `gvar_007D9D34` | `TPlayers` | `Scene Actor Manager` | Quản trị viên thực thể nhân vật trên map (800 slot actor `gvar_007DA300`). |
| `gvar_007DA7E0` | `TMR_BattleHelp` | `VMT_553E84` @ `0051189c.c:1693` | Giao diện hiển thị luật và hướng dẫn võ đài. |
| `gvar_007DA27C` | `TMR_WaterBattleManage` | `VMT_54A0DC` @ `0050a4a0.c:635` | Quản lý tiến trình và thông số chiến trường Thủy chiến. |
| `gvar_007DA164` | `TMR_WBManageOrgManage` | `VMT_54A140` @ `0051189c.c:1720` | Quản lý tổ chức / quân đoàn phe phái trong Thủy chiến. |
| `gvar_007D9F10` | `TMR_WaterBattleHelp` | `VMT_553F70` @ `0051189c.c:1672` | Giao diện hướng dẫn và nhiệm vụ Thủy chiến. |
| `gvar_007DA7BC` | `TPlayer` | `Local Player Instance` | Nhân vật người chơi cục bộ (`+0x1464` trạng thái võ đài, `+0x1468` điểm võ đài). |
| `gvar_007DA6BC` | `TWorldPlayer[]` | Cache 2100 người chơi | Danh sách hồ sơ người chơi khác trong tầm nhìn (đọc tên nhân vật). |
| `gvar_007D9DE4` | `TFNpc` | Bảng dữ liệu NPC | Bảng danh mục NPC toàn cục (tra cứu tên NPC qua `FUN_00623f00`). |
| `gvar_007DA1B0` | `TChatManager` | Quản lý kênh chat | Đầu ra hiển thị chuỗi thông báo qua `FUN_007ab870`. |
| `DAT_009490ac` | Mảng chuỗi hệ ngũ hành | 4 chuỗi $\times$ 85 bytes | Chứa tên 4 thuộc tính ngũ hành: Địa, Thủy, Hỏa, Phong. |

---

## 6. Chiều Client → Server (C → S)

- Kiểm tra tại `ts_decompile/functions/0077f414_FUN_0077F414.c`, dòng 1060:
  ```c
  case 0x3f:
    break;
  ```
- **Kết luận**: Nhánh `case 0x3f:` hoàn toàn rỗng. Client **không bao giờ chủ động phát gói tin mang Opcode 0x3F**. Main OP `0x3F` thuần túy là giao thức cập nhật dữ liệu một chiều từ **Server xuống Client (S → C)**.

---

## 7. Chuỗi Hiển Thị, VISCII / CP1258 & Literal Strings

- Các chuỗi mẫu thông báo trong `FUN_0054c348` được lưu nhị phân trực tiếp trong phân đoạn mã:
  + `DAT_0054c994`, `DAT_0054c9ec`: Chuỗi thông báo trạng thái/bắt đầu võ đài.
  + `DAT_0054ca58`, `DAT_0054ca70`: Chuỗi ghép thông báo hệ ngũ hành.
  + `DAT_0054ca84`, `DAT_0054caa0`: Chuỗi ghép đặc tính lôi đài.
  + `DAT_0054cabc`, `DAT_0054cadc`: Chuỗi ghép NPC với hệ ngũ hành.
  + `DAT_0054cae8`: Chuỗi thông báo mức cược 5.000 lượng (độ dài 61 bytes).
  + `DAT_0054cb18`: Chuỗi thông báo mức cược 10.000 lượng (độ dài 51 bytes).
  + `DAT_0054cb48`: Chuỗi kết thúc thông tin NPC võ đài.
  + `DAT_0054cb64`, `DAT_0054cb78`: Chuỗi tiền tố / hậu tố bao bọc tên tuyển thủ tham chiến.
- Thư mục `ts_decompile/redump/` hiện không có file trích xuất riêng `lit_54xxxx.hex`. Tên người chơi và tên NPC được lấy động từ CSDL tại thời điểm thực thi. Mọi chuỗi thông báo được đưa vào kênh chat hệ thống qua `FUN_007ab870`.

---

## 8. Cấu Trúc Rest Payload & Kiểu Dữ Liệu

### 8.1. SubOp 0x01 (Chi tiết các biến thể Sub-SubOp)
```
Sub-SubOp 0x01: [0x01][0x01]                                       (2 bytes)
Sub-SubOp 0x02: [0x01][0x02]                                       (2 bytes)
Sub-SubOp 0x03: [0x01][0x03][Element: 1B][Flag: 1B]                (4 bytes)
Sub-SubOp 0x04: [0x01][0x04][Element: 1B]                          (3 bytes)
Sub-SubOp 0x05: [0x01][0x05][Element: 1B][NpcID: 4B LE]           (7 bytes)
Sub-SubOp 0x06: [0x01][0x06][NpcID: 4B LE][BetAmount: 2B LE]      (8 bytes)
Sub-SubOp 0x07: [0x01][0x07][Element: 1B][NpcID: 4B LE]           (7 bytes)
Sub-SubOp 0x08: [0x01][0x08][PlayerID: 4B LE]                     (6 bytes)
```

### 8.2. SubOp 0x05 (Đồng bộ điểm võ đài)
```
[Offset] [Kiểu]       [Tên trường]    [Mô tả]
+0       BYTE         SubOp           0x05
+1       DWORD_LE     BattleScore     Giá trị cập nhật vào LocalPlayer + 0x1468
```
*Tổng độ dài Payload:* 5 bytes.

### 8.3. SubOp 0x06 (Đồng bộ trạng thái võ đài)
```
[Offset] [Kiểu]       [Tên trường]    [Mô tả]
+0       BYTE         SubOp           0x06
+1       DWORD_LE     BattleState     Giá trị cập nhật vào LocalPlayer + 0x1464
```
*Tổng độ dài Payload:* 5 bytes.

---

## 9. Ma Trận Kiểm Thử / Hướng Dẫn Mock Server

1. **Test Case 1: Thông báo mức cược 5.000 lượng**:
   - Frame Wire: `F4 44 0A 00 3F 01 06 E8 03 00 00 88 13` (mã hóa XOR tĩnh 0xAD).
   - Payload giải mã: `[3F][01][06][NpcID: 1000][BetAmount: 5000]`.
   - Kỳ vọng: Client hiển thị thông báo NPC kèm mức cược 5.000 lượng trong khung chat.
2. **Test Case 2: Thông báo mức cược 10.000 lượng**:
   - Frame Wire: `F4 44 0A 00 3F 01 06 E8 03 00 00 10 27` (XOR 0xAD).
   - Payload giải mã: `[3F][01][06][NpcID: 1000][BetAmount: 10000]`.
   - Kỳ vọng: Client hiển thị thông báo NPC kèm mức cược 10.000 lượng trong khung chat.
3. **Test Case 3: Đồng bộ trạng thái võ đài (`SubOp 0x06`)**:
   - Frame Wire: `F4 44 06 00 3F 06 01 00 00 00` (XOR 0xAD).
   - Payload giải mã: `[3F][06][01 00 00 00]`.
   - Kỳ vọng: Client cập nhật `LocalPlayer + 0x1464 = 1`.
4. **Test Case 4: Đồng bộ điểm lôi đài (`SubOp 0x05`)**:
   - Frame Wire: `F4 44 06 00 3F 05 64 00 00 00` (XOR 0xAD).
   - Payload giải mã: `[3F][05][100 (0x64)]`.
   - Kỳ vọng: Client cập nhật `LocalPlayer + 0x1468 = 100`.

---

## 10. Bảng Đối Chiếu Nguồn Sơ Cấp (`ts_decompile/`)

| Thành phần | Đường dẫn tệp | Dòng / Ghi chú |
| :--- | :--- | :--- |
| **Case 56 Function** | `ts_decompile/case_functions/functions/case_056_00795AC4_FUN_00795ac4.c` | Dòng 1–105 |
| **Main Dispatcher** | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | Dòng 7101–7172 (`case 0x3f`) |
| **SubOp 1 Handler** | `ts_decompile/functions/0054c348_FUN_0054c348.c` | Dòng 37–265 |
| **SubOp 5 Handler** | `ts_decompile/functions/00747d50_FUN_00747d50.c` | Dòng 22–45 |
| **SubOp 6 Handler** | `ts_decompile/functions/00747dc4_FUN_00747dc4.c` | Dòng 22–45 |
| **Word LE Decoder** | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` | Dòng 166–174 |
| **DWord LE Decoder**| `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c` | Dòng 1–252 |
| **NPC Lookup** | `ts_decompile/functions/00623f00_FUN_00623f00.c` | Dòng 20–55 |
| **Chat Output** | `ts_decompile/functions/007ab870_FUN_007ab870.c` | Đẩy chuỗi vào chatbox |
| **FormCreate Globals**| `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c` | Khởi tạo `gvar_007D9DFC`, `gvar_007DA27C` |
| **LoadingThread Globals**| `ts_decompile/functions/0051189c_FUN_0051189c.c` | Khởi tạo `gvar_007DA7E0`, `gvar_007DA164`, `gvar_007D9F10` |
| **C → S Dispatcher**| `ts_decompile/functions/0077f414_FUN_0077F414.c` | Dòng 1060 (`case 0x3f: break;`) |
