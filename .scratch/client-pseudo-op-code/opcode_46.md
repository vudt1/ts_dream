# PHÂN TÍCH — Main OP 0x46 (70) / Case 62 / FUN_007960bd @ 0x007960BD

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh 100% từ mã nguồn sơ cấp** (`ts_decompile/` only).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x46` (thập phân: `70`, ánh xạ tới **Case 62**) phụ trách toàn bộ **Hệ Thống Sự Kiện Đua Thuyền Rồng (Dragon Boat Race System)**:
1. **Quản lý Vòng đời & Trạng thái Cuộc đua (`Dragon Boat Race Controller` - `gvar_007DA714`)**:
   - Quản lý các giai đoạn của cuộc đua: chuẩn bị, đếm ngược 5 giây xuất phát (hiệu ứng `"02_start"`), mở cửa xuất phát (`"SmallDoorLight"` tại Map `12000`), thời gian đua, và kết thúc cuộc đua (âm thanh/hiệu ứng `"L10788"`).
   - Phát các thông báo sự kiện, biểu ngữ nổi (Toast Alert) và thông báo chatbox về diễn biến trường đua cho người chơi.
2. **Đồng bộ Tọa độ & Cờ Đua Thuyền trong Thời gian thực (`FUN_0051b3f8` & `FUN_0051bda0`)**:
   - Cập nhật liên tục tọa độ đích $(X, Y)$ và cờ trạng thái thuyền đua (`Actor + 0x624`) cho danh sách người chơi tham gia trong Scene thông qua hàm đặt vị trí [`FUN_00720940`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/007209fc_FUN_007209fc.c).
   - Dừng thuyền và khôi phục trạng thái mặc định khi thuyền về đích hoặc hết giờ đua.
3. **Bảng Xếp Hạng Đua Thuyền Rồng (`TLH_DragonBoatStanding` - `gvar_007DA1E8`)**:
   - Nhận và hiển thị bảng thành tích, xếp hạng các đội đua thuyền rồng (tối đa 10 thứ hạng, phân trang tự động 5 mục/trang).
4. **Phát Hoạt cảnh Chuyển Cảnh (`TMovie` - `FUN_0051bca4`)**:
   - Chuyển `SceneMode = 3` (Cutscene mode) và nạp phát file phim kịch bản `.sty` từ thư mục `\sty\<MovieID>.sty`.
5. **Cờ Tham Gia của Người Chơi Bản Địa (`LocalPlayer + 0x157a`)**:
   - Cập nhật trực tiếp 1 byte cờ xác nhận người chơi có đang trong trạng thái tham gia cuộc đua thuyền rồng hay không.

- **Chiều giao tiếp**: Thuần túy **Server → Client (S→C)**. Chiều Client → Server tại `FUN_0077f414:1072` (`case 0x46: break;`) là rỗng (`break;`). Toàn bộ tiến trình cuộc đua do Server kiểm soát và gửi lệnh đồng bộ xuống Client.

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x46 (70) → byte_table[0x78A8EE][0x46] = 0x3E (62)
                  → dword_table[0x78A9B6][62] @ 0x0078AAAE = 0x007960BD
                  → FUN_007960bd (Case 62)
```

1. **Hàng đợi mạng**: `TForm1.CY_DelRevQueue` ([`ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c:86-93`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c#L86-L93)) tách byte đầu `P[0] = 0x46` làm Opcode, cắt Rest Payload (`local_10`), gọi `FUN_0078a89c(Self, Opcode, Payload)`.
2. **Dispatcher Inline**: Nằm tại [`ts_decompile/functions/0078a89c_FUN_0078a89c.c:7373-7423`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.c#L7373-L7423).
3. **Case Function độc lập**: [`ts_decompile/case_functions/functions/case_062_007960BD_FUN_007960bd.c:21-98`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/functions/case_062_007960BD_FUN_007960bd.c#L21-L98).

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
- `RP[0]` (byte thứ 2 của toàn bộ frame payload) là **SubOp** (giá trị từ `1` đến `8`).
- Khối lệnh `switch` phân phối tới 8 nhánh xử lý độc lập. Trường hợp `default` dọn dẹp biến chuỗi và thoát an toàn.

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Tên Nghiệp Vụ | Đối Tượng Tiếp Nhận | Hàm Callee | Min Len | Tình trạng SSOT |
| :---: | :--- | :--- | :--- | :---: | :---: |
| **`0x01`** | Thông báo Sự kiện / Toast / Chatbox Đua thuyền | `*(uint *)gvar_007DA714` (`Race Controller`) | [`FUN_0051b600`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b600_FUN_0051b600.c) | Action 1..6,8: 2B<br>Action 7: 6B | ✔ Đã decompile |
| **`0x02`** | Bắt đầu Xuất phát & Mở cửa Trường đua | `*(int *)gvar_007DA714` (`Race Controller`) | [`FUN_0051b2b8`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b2b8_FUN_0051b2b8.c) | 1B | ✔ Đã decompile |
| **`0x03`** | Đồng bộ Tọa độ & Trạng thái Thuyền đua | `*(undefined4 *)gvar_007DA714` | [`FUN_0051b3f8`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b3f8_FUN_0051b3f8.c) | $2 + N \times 9$ Bytes | ✔ Đã decompile |
| **`0x04`** | Kích hoạt Phim cắt cảnh Chuyển màn (`.sty`) | `*(undefined4 *)gvar_007DA714` | [`FUN_0051bca4`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bca4_FUN_0051bca4.c) | 3B | ✔ Đã decompile |
| **`0x05`** | Kết thúc Cuộc đua & Dừng danh sách thuyền | `*(int *)gvar_007DA714` | [`FUN_0051bda0`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bda0_FUN_0051bda0.c) | $2 + N \times 4$ Bytes | ✔ Đã decompile |
| **`0x06`** | Cập nhật Bảng xếp hạng Đua Thuyền Rồng | `*(int *)gvar_007DA1E8` (`TLH_DragonBoatStanding`) | [`FUN_0051cbf0`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051cbf0_FUN_0051cbf0.c) | Biến thiên (tối đa 10 mục) | ✔ Đã decompile |
| **`0x07`** | Đếm ngược 5 giây Chuẩn bị Xuất phát | `*(int *)gvar_007DA714` | [`FUN_0051b5b4`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b5b4_FUN_0051b5b4.c) | 1B | ✔ Đã decompile |
| **`0x08`** | Cập nhật Cờ tham gia Đua thuyền của Local Player | `gvar_007DA7BC` (`Local Player`) | Gán trực tiếp `+0x157a` | 2B | ✔ Đã decompile |

---

## 4. Chi Tiết Core Business Logic Từng SubOp

### 4.1. SubOp `0x01` — Thông Báo Sự Kiện Đua Thuyền Rồng ([`FUN_0051b600`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b600_FUN_0051b600.c))
- **Đối tượng**: `*(uint *)gvar_007DA714`.
- **Cơ chế**: Đọc byte thứ hai `RP[1]` làm `ActionID` ($1..8$):
  1. `Action 1`: Gửi chuỗi thông báo hệ thống tại `&DAT_0051b890` vào khung chat (`FUN_007ab870`).
  2. `Action 2`: Gửi chuỗi thông báo hệ thống tại `&DAT_0051b8f8` vào khung chat (`FUN_007ab870`).
  3. `Action 3`: Gửi chuỗi thông báo tại `&DAT_0051b9c0`, đồng thời gọi [`FUN_0051b070`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b070_FUN_0051b070.c) đóng cửa xuất phát trường đua và đặt lại cờ `gvar_007DA714 + 4 = 0`.
  4. `Action 4`: Hiển thị biểu ngữ thông báo nổi (Toast Alert) qua `ToastManager` (`gvar_007D9D6C + 0x90`) với chuỗi `&DAT_0051ba64`.
  5. `Action 5`: Hiển thị Toast Alert với chuỗi cảnh báo `&DAT_0051bb2c`.
  6. `Action 6`: Hiển thị Toast Alert với chuỗi cảnh báo `&DAT_0051bbd8`.
  7. `Action 7`: 
     - Đọc 4 bytes `CharID` (DWORD LE qua [`FUN_0077ef7c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077ef7c_FUN_0077ef7c.c)) từ vị trí `RP[2..5]`.
     - Lấy tên nhân vật: nếu là Local Player đọc tại `LocalPlayer + 9`; nếu là người chơi khác tra cứu qua bảng cache `gvar_007DA6BC` bằng [`FUN_00722508`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00722464_FUN_00722464.c).
     - Ghép chuỗi thông báo kết quả: `&DAT_0051bc38` + `[Tên Nhân Vật]` + `" "` + `&DAT_0051bc58` + `","` + `&LAB_0051bc78`.
     - Xuất ra kênh chat hệ thống qua `FUN_007ab870`.
  8. `Action 8`: Gọi [`FUN_0051bf84`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bf84_FUN_0051bf84.c) đặt lại trạng thái hiển thị thông báo.

---

### 4.2. SubOp `0x02` — Bắt Đầu Xuất Phát & Mở Cửa Trường Đua ([`FUN_0051b2b8`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b2b8_FUN_0051b2b8.c))
- **Mã nguồn**:
  ```c
  void FUN_0051b2b8(int param_1) {
    *(undefined1 *)(param_1 + 4) = 1; // Bật cờ cuộc đua: đang diễn ra
    if (*(short *)(*(int *)gvar_007DA7BC + 0x63a) == 12000) { // Đang ở Map 12000 (Trường đua)
      // Tính toán tọa độ cửa xuất phát từ gvar_007DA5A0 và gvar_007D9C28
      FUN_007c4f0c(*(uint *)gvar_007DA0C4, "SmallDoorLight", iVar4, 6, 300, 0x18, uVar3);
    }
    FUN_007ab870(*(int *)gvar_007DA1B0, 0, &DAT_0051b370, '\0'); // Chat thông báo xuất phát
    return;
  }
  ```
- **Ý nghĩa**: Bật cờ cuộc đua tại `+4 = 1`, mở hiệu ứng ánh sáng cửa xuất phát `"SmallDoorLight"` tại Map `12000` và gửi thông báo chat xuất phát `DAT_0051b370`.

---

### 4.3. SubOp `0x03` — Đồng Bộ Tọa Độ & Trạng Thái Thuyền Đua ([`FUN_0051b3f8`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b3f8_FUN_0051b3f8.c))
- **Cấu trúc mảng lặp**:
  + `RP[1]`: Số lượng thuyền cập nhật $N$ (1 byte).
  + Bắt đầu từ offset 3 (Pascal 1-based), mỗi phần tử chiếm đúng **9 bytes**:
    * `CharID`: 4 bytes DWORD LE (giải mã qua `FUN_0077ef7c`).
    * `TargetX`: 2 bytes WORD LE (giải mã qua [`FUN_0077eb9c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077eb9c_FUN_0077eb9c.c)).
    * `TargetY`: 2 bytes WORD LE (giải mã qua `FUN_0077eb9c`).
    * `BoatState`: 1 byte (cờ trạng thái / thứ hạng thuyền).
- **Core Logic**:
  ```c
  if (*(uint *)(*(int *)gvar_007DA7BC + 4) == CharID) {
    targetActor = *(int *)gvar_007DA7BC; // Bản thân Local Player
  } else {
    uVar3 = FUN_0070c20c(*(int *)gvar_007D9D34, CharID); // Tra cứu trong mảng 800 actor
    targetActor = *(int *)(gvar_007DA300 + uVar3 * 4);
  }
  *(ushort *)(targetActor + 0x624) = (ushort)BoatState; // Cập nhật cờ thuyền/thứ hạng
  FUN_00720940(targetActor, (uint)TargetX, (uint)TargetY); // Đặt tọa độ đích di chuyển
  ```

---

### 4.4. SubOp `0x04` — Kích Hoạt Phim Cắt Cảnh Chuyển Màn ([`FUN_0051bca4`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bca4_FUN_0051bca4.c))
- **Mã nguồn**:
  ```c
  void FUN_0051bca4(undefined4 param_1, int param_2) {
    *(undefined1 *)(*(int *)gvar_007DA37C + 0x1d) = 3; // Chuyển SceneMode = 3 (Movie/Cutscene)
    if (*(int *)gvar_007DA434 == 0) {
      *(int **)gvar_007DA434 = TMovie_Create((int *)VMT_614FDC_TMovie, '\x01', ...);
    }
    _LStrCopy(param_2, 2, 2, &local_18);
    uVar3 = FUN_0077eb9c(*(undefined4 *)gvar_007D9D30, local_18); // MovieID (WORD LE)
    // Ghép đường dẫn: AppPath + "\sty\" + MovieID + ".sty"
    _LStrCatN(&local_10, 4, __dummy_68, ".sty", IntToStr(uVar3), "\\sty\\", AppPath);
    FUN_00618668(*(int *)gvar_007DA434, local_10); // Phát video / hoạt cảnh
  }
  ```
- **Ý nghĩa**: Tạm dừng chế độ chơi thông thường, phát đoạn phim chuyển cảnh kịch bản `.sty` của trường đua thuyền rồng.

---

### 4.5. SubOp `0x05` — Kết Thúc Cuộc Đua & Dừng Thuyền ([`FUN_0051bda0`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bda0_FUN_0051bda0.c))
- **Cấu trúc mảng lặp**:
  + `RP[1]`: Số lượng thuyền dừng đua $N$ (1 byte).
  + Mỗi phần tử chiếm đúng **4 bytes**: `CharID: 4B LE` (DWORD LE).
- **Core Logic**:
  1. Duyệt từng `CharID`, tìm actor trong Scene:
     - Xóa cờ trạng thái thuyền: `*(undefined2 *)(Actor + 0x624) = 0;`.
     - Cố định vị trí tại chỗ: `FUN_00720940(Actor, Actor->X, Actor->Y);`.
  2. Đặt lại cờ trạng thái: `*(undefined1 *)(gvar_007DA714 + 5) = 0;`.
  3. Tải hiệu ứng âm thanh/kết thúc: `*(undefined4 *)(gvar_007DA714 + 0xc) = FUN_007c9b38(..., "L10788");`.
  4. Ghi nhận mốc thời gian kết thúc `Now()` vào `gvar_007DA714 + 0x18`.

---

### 4.6. SubOp `0x06` — Cập Nhật Bảng Xếp Hạng Đua Thuyền ([`FUN_0051cbf0`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051cbf0_FUN_0051cbf0.c))
- **Đối tượng**: `*(int *)gvar_007DA1E8` (Instance của `TLH_DragonBoatStanding`).
- **Core Logic**:
  1. Xóa vùng nhớ bảng xếp hạng: `_FillChar((param_1 + 0x164), 55, 0);`.
  2. Đọc `RP[1]` làm số lượng người đua $N$ (tối đa 10 người).
  3. Bật cờ hiển thị: `*(undefined1 *)(param_1 + 0x19b) = 1;`.
  4. Tính số trang (5 người/trang): `*(char *)(param_1 + 0x19c) = (N - 1) / 5 + 1;`.
  5. Vòng lặp duyệt từng người đua:
     - `NameLen = RP[offset]` (1 byte).
     - `Name = RP[offset + 1 .. offset + NameLen]` $\rightarrow$ Nạp vào `param_1 + 0x164 + slot * 5`.
     - `ScoreOrRank = RP[offset + 1 + NameLen]` (1 byte) $\rightarrow$ Nạp vào `param_1 + 0x168 + slot * 5`.
     - `offset += 1 + NameLen + 1`.

---

### 4.7. SubOp `0x07` — Đếm Ngược 5 Giây Xuất Phát ([`FUN_0051b5b4`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b5b4_FUN_0051b5b4.c))
- **Mã nguồn**:
  ```c
  void FUN_0051b5b4(int param_1) {
    *(undefined4 *)(param_1 + 8) = FUN_007c9b38(*(uint *)gvar_007D9ED8, "02_start"); // Load Sprite/Sound
    *(undefined4 *)(param_1 + 0x14) = 5; // Countdown = 5 giây
    Now();
    *(double *)(param_1 + 0x18) = (double)in_ST0; // Timestamp bắt đầu
    return;
  }
  ```
- **Ý nghĩa**: Bắt đầu đếm ngược 5 giây chuẩn bị xuất phát trên màn hình người chơi bằng biểu ngữ hoạt họa `"02_start"`.

---

### 4.8. SubOp `0x08` — Cập Nhật Cờ Tham Gia của Local Player
- **Mã nguồn**:
  ```c
  case 8:
    *(undefined1 *)(*(int *)gvar_007DA7BC + 0x157a) = *(undefined1 *)(iVar2 + 1);
  ```
- **Ý nghĩa**: Cập nhật trực tiếp 1 byte cờ tham gia đua thuyền rồng vào cấu trúc dữ liệu của nhân vật chính (`LocalPlayer + 0x157a`).

---

## 5. Khảo Sát Các Biến Toàn Cục Liên Quan

| Biến Toàn Cục | Kiểu Dữ Liệu / Lớp | Khởi Tạo & VMT | Vai Trò Nghiệp Vụ |
| :--- | :--- | :--- | :--- |
| `gvar_007DA714` | `TDragonBoatRaceManager` | Quản lý trường đua | Điều phối cuộc đua: cờ bắt đầu `+4`, đếm ngược `+0x14`, âm thanh `+0x08/+0x0c`, thời gian `+0x18`. |
| `gvar_007DA1E8` | `TLH_DragonBoatStanding` | `VMT_51AF6C` @ `0051189c.c:1794` | Bảng xếp hạng Đua thuyền rồng: danh sách 10 tay đua (`+0x164`), cờ hiển thị `+0x19b`, số trang `+0x19c`. |
| `gvar_007DA7BC` | `TPlayer` | `Local Player Instance` | Nhân vật người chơi cục bộ (`+0x157a` cờ tham gia đua, `+0x624` trạng thái thuyền, `+0x63a` Map ID). |
| `gvar_007DA300` | `TPlayer[800]` | Mảng 800 Actor | Danh sách thực thể người chơi/đối thủ trên map. |
| `gvar_007DA0C4` | `TMapEffectManager` | Quản lý hiệu ứng Map | Quản lý hiệu ứng cửa xuất phát `"SmallDoorLight"` tại Map 12000 qua `FUN_007c4f0c`. |
| `gvar_007DA434` | `TMovie` | `VMT_614FDC` | Phát phim cắt cảnh `.sty` chuyển màn trường đua qua `FUN_00618668`. |
| `gvar_007DA37C` | `TGameState` / `SceneState` | Trạng thái màn chơi | `+0x1d = 3`: Chế độ Cutscene / Movie. |
| `gvar_007DA1B0` | `TChatManager` | Quản lý kênh Chat | Xuất chuỗi thông báo trường đua qua `FUN_007ab870`. |
| `gvar_007D9D6C` | `TToastManager` | Quản lý Toast Alert | Xuất biểu ngữ cảnh báo nổi qua `ToastManager.ShowToast` (`+0x90`). |

---

## 6. Chiều Client → Server (C → S)

- Kiểm tra tại [`ts_decompile/functions/0077f414_FUN_0077F414.c:1072`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077f414_FUN_0077F414.c#L1072):
  ```c
  case 0x46:
    break;
  ```
- **Kết luận**: Nhánh `case 0x46:` hoàn toàn rỗng. Client **không bao giờ gửi gói tin mang Main Opcode 0x46** lên server. Toàn bộ Opcode `0x46` là giao thức cập nhật một chiều từ **Server xuống Client (S → C)**.

---

## 7. Chuỗi Hiển Thị, VISCII / CP1258 & Literal Strings

- Các chuỗi mẫu thông báo được lưu nhị phân trực tiếp trong phân đoạn mã của [`FUN_0051b600`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b600_FUN_0051b600.c) và [`FUN_0051b2b8`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b2b8_FUN_0051b2b8.c):
  + `DAT_0051b890`, `DAT_0051b8f8`, `DAT_0051b9c0`: Chuỗi chat thông báo sự kiện / bắt đầu đua thuyền.
  + `DAT_0051ba64`, `DAT_0051bb2c`, `DAT_0051bbd8`: Chuỗi biểu ngữ Toast cảnh báo trường đua.
  + `DAT_0051bc38`, `DAT_0051bc58`, `LAB_0051bc78`: Chuỗi ghép bọc tên tay đua và kết quả thứ hạng trong khung chat.
  + `DAT_0051b370`: Chuỗi chat thông báo mở cửa trường đua.
  + `"SmallDoorLight"`: Tên hiệu ứng ánh sáng cửa xuất phát trường đua.
  + `"02_start"`: Tên animation đếm ngược 5 giây xuất phát.
  + `"L10788"`: Tên hiệu ứng âm thanh hoàn thành cuộc đua.
  + `"\\sty\\"` & `".sty"`: Phần mở rộng file phim kịch bản cutscene.
- Tên tay đua được nạp động từ CSDL người chơi tại thời điểm chạy. Không có chuỗi VISCII/CP1258 tĩnh nào trong thư mục `ts_decompile/redump/` cho các địa chỉ `0x0051Bxxx`.

---

## 8. Cấu Trúc Rest Payload & Kiểu Dữ Liệu

### 8.1. SubOp 0x01 (Thông Báo Sự Kiện Đua Thuyền)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x01
+01      uint8       ActionID      Mã loại thông báo (1..8)
[Nếu ActionID == 7]:
+02      uint32 LE   CharID        ID nhân vật được vinh danh (4 bytes Little-Endian)
```

### 8.2. SubOp 0x02 (Xuất Phát Cuộc Đua)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x02
```
*Độ dài:* 1 byte.

### 8.3. SubOp 0x03 (Đồng Bộ Vị Trí Thuyền Đua)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x03
+01      uint8       BoatCount     Số lượng thuyền cần cập nhật (N)
--- Mảng lặp N phần tử (mỗi phần tử 9 bytes): ---
  +00    uint32 LE   CharID        ID nhân vật sở hữu thuyền
  +04    uint16 LE   TargetX       Tọa độ X đích đến
  +06    uint16 LE   TargetY       Tọa độ Y đích đến
  +08    uint8       BoatState     Cờ trạng thái / thứ hạng thuyền
```
*Tổng độ dài:* $2 + 9 \times N$ bytes.

### 8.4. SubOp 0x04 (Phát Phim Chuyển Màn)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x04
+01      uint16 LE   MovieID       Mã số file phim cutscene (.sty)
```
*Độ dài:* 3 bytes.

### 8.5. SubOp 0x05 (Kết Thúc Cuộc Đua / Dừng Thuyền)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x05
+01      uint8       BoatCount     Số lượng thuyền dừng đua (N)
--- Mảng lặp N phần tử (mỗi phần tử 4 bytes): ---
  +00    uint32 LE   CharID        ID nhân vật
```
*Tổng độ dài:* $2 + 4 \times N$ bytes.

### 8.6. SubOp 0x06 (Cập Nhật Bảng Xếp Hạng)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x06
+01      uint8       EntryCount    Số lượng bản ghi xếp hạng (N <= 10)
--- Mảng lặp N phần tử (độ dài biến thiên): ---
  +00    uint8       NameLen       Độ dài tên người chơi (L)
  +01    bytes[L]    PlayerName    Chuỗi tên người chơi (L bytes)
  +L+1   uint8       RankOrScore   Thứ hạng / Điểm số
```

### 8.7. SubOp 0x07 (Đếm Ngược 5 Giây)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x07
```
*Độ dài:* 1 byte.

### 8.8. SubOp 0x08 (Gán Cờ Tham Gia Cho Local Player)
```
Offset   Kiểu        Tên trường    Mô tả
+00      uint8       SubOp         0x08
+01      uint8       RacingFlag    Trạng thái tham gia (gán vào LocalPlayer + 0x157a)
```
*Độ dài:* 2 bytes.

---

## 9. Ma Trận Kiểm Thử / Hướng Dẫn Mock Server

1. **Test Case 1: Đếm ngược xuất phát 5 giây (`SubOp 0x07`)**:
   - Frame Wire: `F4 44 03 00 46 07` (mã hóa XOR tĩnh 0xAD).
   - Payload giải mã: `[46][07]`.
   - Kỳ vọng: Client hiển thị đếm ngược 5 giây `"02_start"`, bộ đếm thời gian bắt đầu chạy.
2. **Test Case 2: Bắt đầu cuộc đua & Mở cửa xuất phát (`SubOp 0x02`)**:
   - Frame Wire: `F4 44 03 00 46 02` (XOR 0xAD).
   - Payload giải mã: `[46][02]`.
   - Kỳ vọng: Đặt cờ `gvar_007DA714 + 4 = 1`, mở hiệu ứng `"SmallDoorLight"` tại Map 12000, gửi thông báo chat xuất phát.
3. **Test Case 3: Đồng bộ vị trí thuyền đua (`SubOp 0x03`)**:
   - Frame Wire: `F4 44 0C 00 46 03 01 E8 03 00 00 64 00 C8 00 01` (XOR 0xAD).
   - Payload giải mã: `[46][03] [Count: 1] [CharID: 1000] [X: 100] [Y: 200] [State: 1]`.
   - Kỳ vọng: Thuyền của nhân vật ID 1000 di chuyển tới tọa độ $(100, 200)$, cờ `Actor + 0x624 = 1`.
4. **Test Case 4: Cập nhật cờ tham gia của Local Player (`SubOp 0x08`)**:
   - Frame Wire: `F4 44 04 00 46 08 01` (XOR 0xAD).
   - Payload giải mã: `[46][08][01]`.
   - Kỳ vọng: Cập nhật `LocalPlayer + 0x157a = 1`.

---

## 10. Bảng Đối Chiếu Nguồn Sơ Cấp (`ts_decompile/`)

| Thành phần | Đường dẫn tệp trong `ts_decompile/` | Vị trí / Dòng |
| :--- | :--- | :--- |
| **Case 62 Function** | [`ts_decompile/case_functions/functions/case_062_007960BD_FUN_007960bd.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/functions/case_062_007960BD_FUN_007960bd.c) | Dòng 1–101 |
| **Main Dispatcher** | [`ts_decompile/functions/0078a89c_FUN_0078a89c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.c) | Dòng 7373–7423 (`case 0x46`) |
| **SubOp 1 Handler** | [`ts_decompile/functions/0051b600_FUN_0051b600.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b600_FUN_0051b600.c) | Dòng 28–140 |
| **SubOp 2 Handler** | [`ts_decompile/functions/0051b2b8_FUN_0051b2b8.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b2b8_FUN_0051b2b8.c) | Dòng 20–53 |
| **SubOp 3 Handler** | [`ts_decompile/functions/0051b3f8_FUN_0051b3f8.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b3f8_FUN_0051b3f8.c) | Dòng 25–139 |
| **SubOp 4 Handler** | [`ts_decompile/functions/0051bca4_FUN_0051bca4.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bca4_FUN_0051bca4.c) | Dòng 24–72 |
| **SubOp 5 Handler** | [`ts_decompile/functions/0051bda0_FUN_0051bda0.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051bda0_FUN_0051bda0.c) | Dòng 26–111 |
| **SubOp 6 Handler** | [`ts_decompile/functions/0051cbf0_FUN_0051cbf0.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051cbf0_FUN_0051cbf0.c) | Dòng 21–113 |
| **SubOp 7 Handler** | [`ts_decompile/functions/0051b5b4_FUN_0051b5b4.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051b5b4_FUN_0051b5b4.c) | Dòng 19–31 |
| **Race Controller Tick**| [`ts_decompile/functions/0051c2ec_FUN_0051c2ec.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051c2ec_FUN_0051c2ec.c) | Dòng 23–115 |
| **Scene Map 12000 Tick**| [`ts_decompile/functions/00733b94_FUN_00733b94.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00733b94_FUN_00733b94.c) | Dòng 160–162 (`Map 12000`) |
| **Standing Form Create**| [`ts_decompile/functions/0051189c_FUN_0051189c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051189c_FUN_0051189c.c) | Dòng 1794 (`TLH_DragonBoatStanding`) |
| **Word LE Decoder** | [`ts_decompile/functions/0077eb9c_FUN_0077eb9c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077eb9c_FUN_0077eb9c.c) | Dòng 166–174 |
| **DWord LE Decoder**| [`ts_decompile/functions/0077ef7c_FUN_0077ef7c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077ef7c_FUN_0077ef7c.c) | Dòng 1–252 |
| **C → S Dispatcher**| [`ts_decompile/functions/0077f414_FUN_0077F414.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0077f414_FUN_0077F414.c) | Dòng 1072 (`case 0x46: break;`) |
