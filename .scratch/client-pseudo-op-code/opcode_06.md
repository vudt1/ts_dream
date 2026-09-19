# PHÂN TÍCH — Main OP 0x06 (Case 7, `FUN_0078cb99` @ `0x0078CB99`) — aLogin.exe

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều chính: **Server → Client (S→C)**
Trạng thái: **Xác minh từ mã nguồn sơ cấp** (case function + dispatcher inline + jump-table hex thực đo + các helper + nhãn chuỗi literal trong debug HUD + call-site ASM chiều C→S). Không có phỏng đoán suông; mọi kết luận gắn nhãn độ tin cậy.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Đính chính giả định nghiệp vụ

| # | Nhận định trước đây | Kết quả kiểm chứng |
| :--- | :--- | :--- |
| 0.1 | `handoff-opcode-exploration-guide.md` (mục 3) **không gán giả định nào** cho OP 0x06 (không nằm trong lộ trình ưu tiên). | Khoảng trống này nay được lấp: **OP 0x06 = KÊNH ĐỒNG BỘ DI CHUYỂN / VỊ TRÍ ACTOR hai chiều** (S→C: lệnh di chuyển actor từ xa; C→S: báo cáo bước đi của chính mình). Độ tin cậy: **CAO**. |
| 0.2 | `opcode_14.md` (dòng 20 & kết luận) phỏng đoán: *"movement realtime nằm ở C→S OP 0x06/0x07, `case 6` gửi `[0x06][X:Word LE][Y:Word LE]...` (tọa độ tại `gvar_007DA7BC + 0x4C/0x50`)"*. | **ĐÚNG**, và được bổ sung chi tiết: phía sau X/Y còn **2 byte chữ ký/nonce**; byte thứ 2 của payload là **sub-code C→S** (`MOV CL,0x1` / `MOV CL,0x2` trong ASM — xem mục 5). Xác minh trực tiếp tại `0077f414_FUN_0077F414.c` dòng 830–883 + asm call-site. Độ tin cậy: **CAO**. |
| 0.3 | Có thể nhầm "đối xứng 1-1": gói S→C và C→S cùng opcode 0x06 **KHÔNG** phải cùng cấu trúc. | **Bất đối xứng hoàn toàn**: S→C SubOp 1 = `[06][01][actorID:4B][dir:1B][X:2B][Y:2B]` (11 B, điều khiển actor **khác**); C→S = `[06][sub:1B][orient:1B][X:2B][Y:2B][sigA][sigB]` (đúng 9 B, báo cáo vị trí và hướng nhìn **của chính mình**, không có ID). Độ tin cậy: **CAO**. |
| 0.4 | Nhãn trường "`+0x35f` TeamStatus": giá trị **1** dễ bị đoán là "thành viên". | Ngược lại. Trong `FUN_007a273c` (kết lập đội): **Leader** được set `+0x35f = 1` (dòng 140), **mỗi thành viên** được set `+0x35f = 2` (dòng 199); giải tán (`FUN_007a2604`) set `0`. Tức **1 = TRƯỞNG NHÓM (leader), 2 = THÀNH VIÊN (follower), 0 = vô đội**. Độ tin cậy: **CAO** (đọc trực tiếp code ghi). |
| 0.5 | Không có chuỗi constant nào được tham chiếu trong handler | Đúng — `FUN_0078cb99` **không push literal nào**; các giá trị `0x7963xx` ở epilogue là **con trỏ frame dọn dẹp SEH/`_LStrClr`**, không phải dữ liệu nghiệp vụ (tránh nhầm như các phân tích cũ dễ vấp). Bằng chứng nghiệp vụ thay vào đó đến từ **chuỗi literal trong debug HUD** `FUN_0050debc` (mục 4.1.3). |

---

## 1. Tóm tắt nghiệp vụ

**Main OP 0x06 là kênh "Remote Actor Movement / Movement Lock & Echo" (Server điều khiển bước đi của các actor trong cảnh của client):**

- **`SubOp 0x01` — Lệnh di chuyển một actor từ xa (Remote Walk Order).** Server nói *"actor mang `actorID` hãy đi tới tọa độ pixel `(X, Y)`, hướng nhìn `dir`"*. Client tra actor trong mảng 800 slot (`gvar_007DA300`, manager `gvar_007D9D34`), đặt **đích đến** (`actor+0x4C/+0x50`), ghi hướng render (`+0xE4 = dir+8`), tile đích (`+0xF0/+0xF4 = X/20, Y/20` — lưới 20 px), rồi recompute path (`FUN_00715b28`) và hướng logic (`FUN_0070da54`). **Không áp dụng cho chính người chơi local** (ID của self không có trong mảng actor — xem 4.1.4).
- **`SubOp 0x02` — Khóa đi lại + yêu cầu echo vị trí (Movement Lock / Force-Report).** Đặt cờ khóa `player+0x653 = 1` (chặn KeyboardWalk / click-walk cho tới khi OP 0x14 SubOp 0x08 gỡ — khớp máy trạng thái `opcode_14.md`); nếu người chơi **đang dở bước đi** (`+0xE5 == 0`), client **dừng giữa đường** (xóa path `+0x100/+0x104`, đích := vị trí hiện tại) và **tự động gửi lại ngay một gói C→S `[06][2][X][Y][sigA][sigB]`** qua `FUN_0071fdf8` → `SendCommand(6)`.
- **Cổng lọc theo trạng thái đội:** nếu **tôi là leader** (`+0x35f==1`) và actor nhận được lệnh đi **là một thành viên trong bảng partner của tôi** (`+0x550[1..4]`, `+0x578`=PartnerNum) → **gói bị bỏ qua** (leader không đá động tới bước đi broadcast của member). Ngược lại nếu actor di chuyển **chính là LeaderID của tôi** (`+0x548`, khi `+0x466 != 0`) → gọi **virtual VMT+0x20** trên player (dừng action/dọn trạng thái — cơ chế song hành với `FUN_0072b4b8`, kênh báo cáo OP 0x20).
- **Cổng theo bản đồ mini-game:** toàn bộ SubOp 1 bị **tắt trên 5 map đặc thù `0xC2F7..0xC2FB`** (idiom so sánh `(ushort)(MapID + 0x3D09) <= 4`; cùng idiom xuất hiện trong `TForm1.KeyboardWalk` và `TForm1.DXDraw1MouseDown` — trên các map này client dùng cơ chế điều khiển nhập cảnh riêng, không dùng walk-sync thường).
- **Chiều C→S (`SendCommand` case 6):** mỗi bước đi bộ hợp lệ của người chơi (gõ phím / click / mount) được báo cáo `[06][sub:1..2][X:W][Y:W][sigA][sigB]` chỉ khi vị trí báo cáo thay đổi so với lần cuối (mirror `+0x5C/+0x60`).

**Vị trí trong bức tranh tổng thể:** OP 0x14 (`opcode_14.md`) lo **teleport/script/map-transition** (tọa độ jump-cut); OP 0x03 (`opcode_03.md`) lo **spawn hồ sơ actor**; còn **OP 0x06 chính là "chân đi" — kênh move-broadcast realtime S→C cho actor từ xa**, khép lại khoảng trống "realtime movement nằm ở OP nào" mà `opcode_14.md` từng để ngỏ.

---

## 2. Entry & cách đọc PacketBuffer

### 2.1. Định tuyến (đã thực đo trên hex dump, không tin bảng chép tay)
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex`: byte tại index `0x06` = **`0x07`** (Case 7). ✓
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex`: dword tại index `7` (địa chỉ `0x0078A9D2`) = **`0x0078CB99`**. ✓ (khớp header file case: `Jump table entry: 0x0078A9D2`.)
- Cross-verify: khối `case 6:` inline trong dispatcher `functions/0078a89c_FUN_0078a89c.c` (dòng ~1791–1888) tái hiện **100%** logic file case riêng.

### 2.2. Chuỗi vào (giữ nguyên kiến trúc đã chốt ở `opcode_00_01.md` mục 2)
`Frame = [F4 44][Len:Word LE][Payload]`, toàn khung XOR `0xAD` (`FUN_0050a248`) → `ClientSocket1Read` deframe → `CY_AddRevQueue` → tick pump `CY_DelRevQueue` tách `MainOp = payload[0]`, `RestPayload = Copy(payload,2,Len-1)` → `FUN_0078a89c(EAX=TFConnect, DL=MainOp, ECX=RestPayload)`.

### 2.3. Đọc đầu handler (`case_007_0078CB99_FUN_0078cb99.c` dòng 27–33)
```c
iVar6 = *(int *)(unaff_EBP + -0xc);                      // ECX = RestPayload (Delphi AnsiString)
if (*(int *)(iVar6 + -4) == 0) _BoundErr(0);             // Length==0 → RANGE ERROR runtime!
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6);   // SubOp = ECX[0] = payload[1]
```
- **Payload rỗng chỉ có byte `[06]` sẽ kích `_BoundErr` (exception Delphi)** — mock server không bao giờ được gửi frame `[06]` trụi. Độ tin cậy: CAO.
- Quy ước chỉ số dùng cho toàn bài: `payload[k]` = byte thứ k **0-based** của payload gốc (`payload[0]=0x06`); `RP[i]` = `ECX[i]` (0-based) = `payload[i+1]`; `_LStrCopy(ECX, p, n)` (Delphi **1-based**) = `payload[p .. p+n-1]`.
- Codec: `FUN_0077eb9c` = 2 byte → **Word LE**; `FUN_0077ef7c` = 4 byte → **DWORD LE**.

### 2.4. Cấu trúc nhánh (KHÔNG phải `switch` ngữ pháp, nhưng là switch đã compile)
Handler dùng chuỗi `if (SubOp == 1) {...} else if (SubOp == 2) {...}`, mọi lối đều rơi về **một epilogue dọn chuỗi chung** (`switchD_00792e22_caseD_0:` dòng 125; bản dispatcher gọi đích này là `code_r0x00796347` — trùng với entry[0] của bảng dword = điểm "no-op" mặc định). Kết luận:
- **Case tồn tại: đúng `0x01` và `0x02`.**
- **Không có `default` xử lý nghiệp vụ** — SubOp `0x00`, `0x03+` **bỏ qua âm thầm**. Độ tin cậy: CAO (cả 2 bản decompile khớp nhau).

---

## 3. Bảng tổng hợp SubOp

| SubOp | Độ dài payload tối thiểu | Bản chất | Field đọc | Tác động dữ liệu chính |
|:---:|:---:|---|---|---|
| `0x01` | **11** | **Remote Walk Order** — ra lệnh cho actor `actorID` đi tới `(X,Y)` | `actorID`:4B LE · `dir`:1B · `X`:2B LE · `Y`:2B LE | Mảng actor 800 slot: `+0x4C/0x50` đích, `+0xE4` hướng render, `+0xF0/0xF4` tile đích; path recomputation. Có 3 cổng lọc (map mini-game / partner-table của leader / LeaderID+0x466). |
| `0x02` | **2** | **Movement Lock + Force Echo** — khóa đi lại của local player; nếu đang dở bước đi thì đứng lại tại chỗ và tự gửi `[06][2][X][Y][sigA][sigB]` lên server | không có field | `player+0x653 := 1` (khóa; gỡ bởi OP 0x14 SubOp 0x08); xóa path `+0x100/+0x104`; `+0x4C/0x50 := +0x1C/0x20`; `+0xE4 := +0xE3`; `SendCommand(6, CL=2)`. |
| khác | — | no-op (rơi về epilogue) | — | — |

---

## 4. Chi tiết từng SubOp (wire layout + logic)

### 4.1. SubOp `0x01` — Remote Walk Order (nhánh chính, dòng 34–121)

#### 4.1.1. Wire layout (11 byte payload)

```
p[0]      0x06            MainOp
p[1]      0x01            SubOp
p[2..5]   actorID         DWORD LE   ← _LStrCopy(ECX,2,4) + FUN_0077ef7c   (dòng 36–38)
p[6]      dir (raw D)     1 byte     ← đọc trực tiếp ECX[5], guard Length≥6 (dòng 60–73)
p[7..8]   destX           Word LE    ← _LStrCopy(ECX,7,2) + FUN_0077eb9c   (dòng 74–76)
p[9..10]  destY           Word LE    ← _LStrCopy(ECX,9,2) + FUN_0077eb9c   (dòng 77–79)
```
- Byte `dir`: trước khi dùng, client tính **`D + 8`** (có kiểm tra carry `IntOver` và kẹp `≤ 0xFF`) thành giá trị ghi vào `actor+0xE4` (dòng 66–73). Nghĩa wire: server gửi **D = hướng − 8** (khớp vùng giá trị 0..7 của hướng logic `+0xE3` do `FUN_0070da54` sinh ra → `+0xE4 ∈ 8..15`). Bản chất phép lệch +8: **suy luận loại trung bình** (byte là "facing/pose render", xem 4.1.5); số học `+8` thì **CAO** (đọc nguyên văn 2 bản decompile).
- Nếu payload ngắn hơn 11 byte: X/Y đọc được cụt/sai → actor đi lệch; **luôn gửi đủ 11 byte**.

#### 4.1.2. Cổng #0 — Map guard (dòng 35)
```c
if (4 < (ushort)(*(short *)(gself + 0x63a) + 0x3d09U)) { ...xử lý... }
```
- `gself + 0x63a` = **MapID Word** (đây chính field OP 0x03 SELF ghi vào — `opcode_03.md` Bảng A). Idiom `(ushort)(x + 0x3D09) <= 4` ≡ `x ∈ [0xC2F7, 0xC2FB]` (vì `0x3D09 = −0xC2F7 mod 2^16`).
- **Nghĩa: SubOp 1 bị TẮT khi đang ở 5 map 0xC2F7..0xC2FB.** Bằng chứng đồng bộ: **cùng idiom** xuất hiện ở `TForm1.KeyboardWalk` (dòng 43: trên 5 map đó, phím mũi tên KHÔNG đi bộ thường mà chuyển sang `FUN_004cef60/FUN_0051bf04` — điều khiển ngang/dọc kiểu mini-game) và `TForm1.DXDraw1MouseDown` (dòng 58: click trên 5 map đó được chuyển hết vào `FUN_007c84b8` + `SendCommand(0x0F)`), cùng phép `- 0xc2f7` index bảng ở `FUN_0058b5a4`/`FUN_00590ba8`/`FUN_005eb2ec` (khóa mở panel theo map). Độ tin cậy: **CAO** về mặt cơ chế (đúng 5 map bị loại trừ); **TRUNG BÌNH** về nghiệp vụ (nhiều khả năng là chuỗi map event/instance "cưỡi/đua", không thể đặt tên chính xác từ static).

#### 4.1.3. Cổng #1 — Lọc partner của leader → bỏ gói (dòng 39–55)
```c
if (gself[0x145c] != 2 && gself[0x35f] == 1 && gself[0x578] != 0) {
    // i = 1..4 (BoundErr cap 4), count = byte gself[0x578]
    if ((gself[0x550 + i*8] == actorID)) goto epilogue;   // DISCARD packet
}
```
**Định danh các offset bằng chính chuỗi literal của debug HUD** `functions/0050debc_FUN_0050debc.c`:
| Offset (object player `gself`/`DAT_0092530c`) | Nhãn literal (dòng) | Writer duy nhất | Ý nghĩa |
|:---|:---|:---|:---|
| `+0x35f` | `"TeamStatus: "` (0050debc:153) | `FUN_007a273c`:140 (=**1**, gán cho **leader**, kèm `0x548:=leaderID`, `0x578:=số partner`, bảng `0x550`); :199 (=**2**, gán cho **từng member**); `FUN_007a2604`:50 (=0 giải đội) | Trạng thái đội: 0 vô đội / **1 trưởng nhóm** / **2 thành viên** (xem đính chính 0.4). `KeyboardWalk` chặn đi bộ khi `==2` (dòng 60). |
| `+0x548` | `"LeaderID: "` (0050debc:155) | như trên | ID trưởng nhóm (leader lưu chính ID mình → tự khớp). |
| `+0x578` | `"PartnerNum: "` (0050debc:158) | như trên / `FillChar` zero 0x28 bytes tại `0x550` khi giải tán (007a2604:52–53) | Số thành viên trong bảng, **tối đa 4**. |
| `+0x550 + i*8` (i=1..4) | duyệt trong 0050debc:169–185 (`id` + `+0x554` aux) | `FUN_007a273c`:158–159 | Bảng partner: `+0x550+i*8` = **charID thành viên**; `+0x554+i*8` = **index actor 1..800** (0 nếu chính là self — vì self không có trong mảng actor). |
| `+0x145c` | (không nhãn) | chỉ `case_004` OP 0x03 dòng 115, nhận từ `func_0x0054a384(MapID)` — **body ĐÃ export** (`0054a384_FUN_0054a384.c:44-152`, xác minh được từ body mới): hàm tra **bảng tĩnh tại `DAT_00948DF8`** (5 dòng, stride 54 byte): MapID khớp word `+0x07` → trả **1**; khớp word `+0x0D+6·k` (k=1..5) → trả **2**; khớp word `+0x31+6` → trả **3**; không khớp → 0 (param_1 không dùng — EAX-artifact) | "scene-class đặc biệt"; giá trị **2** (= MapID nằm ở cột thứ hai của bảng) vô hiệu hóa bộ lọc partner. Độ tin cậy cơ chế: **CAO** (lookup bảng, không phải tính toán); **nội dung bảng chưa dump** (`DAT_00948DF8` thuộc `.data` runtime) → vẫn chưa liệt kê được map nào class 2, và **ý nghĩa nghiệp vụ của class 2: chưa kết luận được** |
| `+0x466` | (không nhãn) | `FUN_0072b390`:73 set **1** (mỗi lần client thi hành 1 action qua `VMT+0x18(code)` rồi `SendCommand(0x20)`); `FUN_0072b4b8`:38 clear **0**; `DXDraw1MouseDown`:74 yêu cầu `!=0` mới cho click-walk | Cờ "đang ở trạng thái hành-động/hoạt-động-world". Ý nghĩa chính xác: **TRUNG BÌNH**; cơ chế set/clear: **CAO**. |

**Hiệu ứng:** tôi là **trưởng nhóm** (`+0x35f == 1`) → mọi gói đi-broadcast `[06 01]` cho **chính 4 thành viên của tôi** (`+0x550[1..4]`) bị **nuốt (discard)** hoàn toàn (bỏ cả phần đặt đích). Giải thích hợp lý: client leader đang tự quản lý đội và tự tính toán kéo các thành viên cục bộ trên máy mình (panel `FUN_005a3018` đọc đúng bảng `0x550/0x578` này) nên chủ động không để server can thiệp vào bước đi của member trên màn hình leader. Độ tin cậy cơ chế: **CAO**.

#### 4.1.4. Cổng #2 — "Leader của tôi vừa đi" (dòng 56–59)
```c
if (gself[0x466] != 0 && gself[0x548] == actorID)
    (**(int**)(*gself) + 0x20)();      // virtual VMT+0x20 trên TPlayer
```
- Với **member** (`0x548`=LeaderID): actor vừa nhận lệnh đi **chính là trưởng nhóm** và tôi đang ở trạng thái action (`0x466≠0`) → gọi virtual `+0x20`. Với **leader** (`0x548`=ID của chính mình): gói echoed targeting ID mình kích hoạt cùng nhánh.
- Định danh virtual: vùng VMT lớp `THuman` trỏ tới `FUN_0072b4b8` = *"clear +0x466 → play action 0 (idle) qua VMT+0x18 → nếu kind==1 (TPlayer) gửi `SendCommand(0x20)`"*. `FUN_00715b28` cũng gọi `VMT+0x20` lên actor rồi xóa path. → Kết luận: **`VMT+0x20` ≈ "dừng action/dọn trạng thái di chuyển (kèm báo cáo OP 0x20)"**; leader bước đi ⇒ member đang làm gì cũng **thôi để nối bước**. Độ tin cậy: **TRUNG BÌNH**.
- Lưu ý quan trọng từ `FUN_007a273c`:104–109 và `0050debc`:185: **ID của chính người chơi local KHÔNG tồn tại trong mảng actor `gvar_007DA300`** (mọi chỗ tra self đều special-case → 0). Do đó **SubOp 1 không bao giờ dời actor self**; sửa vị trí self là việc của SubOp 2 (dừng-tại-chỗ + echo) hoặc OP 0x14 (teleport kịch bản). Độ tin cậy: **CAO**.

#### 4.1.5. Nhân xử lý — đặt đích cho actor (dòng 80–119)
```c
idx = FUN_0070c20c(gvar_007D9D34 /*world*/, actorID);      // scan 1..count(+0x60) mảng 800 slot gvar_007DA300, khớp actor+4==ID
if (idx != 0) {
    actor = *(gvar_007DA300 + idx*4);
    func_0x00712d4c(actor, X, Y);       // [body ĐÃ export — SetDestination, xem dưới]
    actor[0xE4] = dir + 8;              // hướng "render"
    actor[0xF0] = X / 20;               // tile đích X  (lưới 0x14 = 20px)
    actor[0xF4] = Y / 20;               // tile đích Y
    FUN_00715b28(actor);                // (graphics/path) tính lại route node +0x108+i*8 — tóm 1 dòng
    FUN_0070da54(actor);                // (graphics) Recompute hướng logic +0xE3 ∈ 0..7 — tóm 1 dòng
}
```
- **Model bộ nhớ di chuyển của actor** (chốt bằng 3 nguồn: `FUN_0070da54`, `FUN_00731ffc`, `FUN_00712d7c`):
  - `+0x1C/+0x20` (DWORD) = **vị trí hiện tại** (pixel);
  - `+0x4C/+0x50` (DWORD) = **đích đến** (destination — chính là cặp được báo C→S ở case 6 và được `KeyboardWalk` +80px mỗi lần gõ phím);
  - `+0x5C/+0x60` = **mirror "lần cuối đã báo server"** (chỉ case 6 C→S cập nhật);
  - `+0xE3` = hướng logic (0..7 bởi `FUN_0070da54`; `0xC` khi teleport bởi `FUN_0071e2b8`), `+0xE4` = hướng sang phía render, `+0xE5` = cờ **"đã tới nơi/đứng yên"** (Create=1; bắt đầu đi=0 bởi `00731ffc`:105 **và bởi `00712d4c`:21 — body mới xác minh**; tới nơi=1 bởi `0072b390`:80);
  - `+0xE8/+0xEC` tile hiện tại, `+0xF0/+0xF4` tile đích;
  - `+0x100` số waypoint, `+0x104` write-cursor, `+0x108 + i*8` danh sách node path.
- `func_0x00712d4c` **đã có body** (47 byte @`0x00712D4C`, `ts_decompile/functions/00712d4c_FUN_00712d4c.c:16-23`) — phỏng đoán cũ "SetDestination(actor, X, Y)" **được xác minh từ body mới**: `param_1+0x4c = param_2 (X)`, `param_1+0x50 = param_3 (Y)`, **và thêm một tác dụng mới thấy**: `param_1+0xE5 = 0` — tức hàm **xóa luôn cờ "đã tới nơi/đứng yên"** (khớp chính xác model ở mục dưới: bắt đầu đi ⇒ `+0xE5 = 0`; trước đây writer này chưa được liệt kê, chỉ có `00731ffc:105`). Độ tin cậy: **CAO** (không còn là suy luận data-flow). Gọi duy nhất từ `sub_0078cd6b` (chính handler OP 0x06).
- Hai hàm cuối thuộc tầng **pathfinding/render** → theo yêu cầu, **tóm 1 dòng**, không mổ xẻ animation.

### 4.2. SubOp `0x02` — Movement Lock + Force Echo (dòng 122–124)

**Wire:** đúng `[0x06][0x02]` (2 byte, **không field** — mọi byte thừa đều bị bỏ qua).

**Logic:** `FUN_0071fdf8(player /*gself*/, 1)` (`0071fdf8_FUN_0071fdf8.c`, 32 dòng):
```c
if (player[0x653] != 1) {                 // chưa khóa
    player[0x653] = 1;                    // KHÓA đi lại (walk-lock)
    if (player[0xE5] == 0) {              // ...và ĐANG dở bước đi (chưa đứng yên)
        player[0x100] = 0; player[0x104] = 0;      // xóa path/waypoint
        player[0x4C] = player[0x1C];               // đích := vị trí hiện tại (dừng tại chỗ)
        player[0x50] = player[0x20];
        player[0xE4] = player[0xE3];               // đồng bộ hướng render
        SendCommand(TFConnect /*gvar_007D9D30*/, op=6 /*CL=2 trong asm*/);  // ECHO vị trí hiện tại lên server
    }
}
```
- **Cờ khóa `+0x653`** ("server-driven walk lock"): các **reader** đã xác minh — `TForm1.KeyboardWalk`:62 (chặn gõ phím), `TForm1.DXDraw1MouseDown`:127 và `FUN_00712d7c`:72 (chặn click-walk). Các **writer**: `case_006/OP 0x06 SubOp 2` set 1; `FUN_0072b390`:76 → `fdf8(player, 0)` clear 0; dispatcher inline `0078a89c:3111` (bên **OP 0x14 SubOp 0x08 TRANSITION-END** — khớp dòng "xóa LocalActor+0x653" của `opcode_14.md`); `0061ba70` khởi tạo 0. → **Vòng đời khóa**: server khóa bằng `[06][02]`, mở bằng `[14][08]` hoặc action-complete. Độ tin cậy: **CAO**.
- **Chuỗi tương tác đóng (Closed-loop position reconciliation):**
  1. Server gửi lệnh khóa S→C: `[06][02]`.
  2. Client nhận gói, kích hoạt `FUN_0071fdf8(player, 1)`: đặt `player+0x653 = 1` (khóa phím/chuột).
  3. Nếu client đang dở bước đi (`player+0xE5 == 0`), dừng tại chỗ: xóa path/waypoint, snap đích về vị trí hiện tại (`player+0x4C := player+0x1C`, `player+0x50 := player+0x20`).
  4. Client tự động gửi ngay gói C→S `[06][02][orient][X][Y][sigA][sigB]` qua `SendCommand(6, CL=2)`.
  5. Server ghi nhận tọa độ reconciled và phát `[14 08]` để giải phóng khóa di chuyển cho client.

---

## 5. Chiều Client → Server (C→S) liên quan — `TFConnect.SendCommand` `case 6`

### 5.1. Wire (xác định bằng ASM call-site + luật nối chuỗi `_LStrCatN` đảo ngược)
`functions/0077f414_FUN_0077F414.c` dòng 830–883 (`case 6:` của `switch(param_2 & 0xff)`):

```
payload = [0x06] [sub:1B] [orient:1B] [X:Word LE] [Y:Word LE] [sigA:1B] [sigB:1B]   (đúng 9 byte)
```
- **Bổ sung byte `orient` (Hướng nhân vật):** Kiểm tra chi tiết ASM tại `0x0077FD79`: `MOV DL, [EDX + 0xe4]` (lấy hướng nhìn hiện tại từ `player + 0xE4`) kết hợp `_PStrNCat(buf, ..., 3)` ghép 3 byte đầu vào header `[0x06][sub][orient]`. Payload C→S thực tế là **đúng 9 byte**, hoàn toàn khớp với cách Bear C# bóc tách (`orient = data[2]`, `x = read16(data, 3)`, `y = read16(data, 5)`).
- **Gate gửi (dòng 831–832):** chỉ gửi khi `player[0x5C] != player[0x4C] || player[0x60] != player[0x50]` — tức **đích hiện tại khác "lần báo gần nhất"**; sau khi gửi client cập nhật mirror `0x5C/0x60 := 0x4C/0x50` (dòng 878–880). Đây là lý do `FUN_0071fdf8` SubOp 2 tất bật snap `0x4C:=0x1C` trước khi gọi case 6 — tạo "sự thay đổi" để gói echo lọt gate.
- **Thứ tự field:** `local_88` (header 3 byte `[06][sub][orient]`) → `local_8c` = `FUN_0077eb1c(player[0x4C])` = **X (Word LE)** → `local_90` = **Y (Word LE)** → `local_94` = **sigA** → `local_98` = **sigB**.
- **sub (byte [1] = thanh ghi CL của call-site — đọc từ ASM `.asm.txt`, không phải artifact):**
  - `MOV CL,0x1` — **báo cáo bước đi tự nguyện**: `FUN_00731ffc` asm cuối hàm (KeyboardWalk step), `FUN_00712d7c` asm dòng ~329 (click-walk), `FUN_0074b330` asm 130 & 192 (di chuyển theo mount/vehicle).
  - `MOV CL,0x2` — **echo bị server ép**: `FUN_0071fdf8` asm dòng 40 trước `CALL` (chính là hệ quả S→C SubOp 2), và `FUN_0074b330` asm 168.
- **sigA (dòng 862):** `_LStrFromChar(player[4] % 0xD + byte[player + 0x3FA])` = `(charID mod 13 + byte 0x3FA) and 0xFF`. `0x3FA` là **byte cờ job-state** (attrCode `0x23` của stat-setter `FUN_00710ab0` — `opcode_08.md` §4.2) → chữ ký ràng buộc gói với danh tính + trạng thái nhân vật (chống replay/giả mạo thô). Độ tin cậy công thức: **CAO**.
- **sigB (dòng 834–852):** `FUN_00402cf0` **không phải GetAsyncKeyState** mà là **PRNG LCG**: `seed = seed*0x8088405+1; return (uint)(x*seed)>>32` (đọc nguyên văn `00402cf0_FUN_00402cf0.c`). Giá trị gửi = `rand(0xDC) ∈ [0..219]` — một **nonce**; nhánh `==0xff` cộng `rand(0x38)+200` **không bao giờ xảy ra** (rand(220) < 255) → dead code. Độ tin cậy: **CAO**.

### 5.2. Tính đối xứng S↔C của OP 0x06 (trả lời đúng yêu cầu "kiểm tra tính đối xứng case 6")
| Khía cạnh | S→C SubOp 1 | C→S case 6 |
|---|---|---|
| Mục tiêu | actor **từ xa** (theo `actorID`) | **chính mình** (không cần ID — server đã biết ai) |
| Tọa độ | đích mới `X,Y` (px) | `X,Y` = **`+0x4C/0x50`** (đích/vị-trí-báo-cáo của self) — **KHÔNG phải `+0x1C/0x20`** |
| Hướng | `dir` 0..7 (bias −8) gửi tường minh | `orient` (byte 2, lấy từ `player + 0xE4`) |
| id | 4B LE | không có |
| Chữ ký | không có | `sigA` + `sigB` |
| Kết luận | **Không đối xứng byte-to-byte.** Đây là **hai message khác vai trò dùng chung một kênh số 6**: server→client là *command*, client→server là *report + nonce*. |

### 5.3. Mối quan hệ giữa Di chuyển liên tục (OP 0x06) và Dịch chuyển tức thời / Warp Map (OP 0x0C)
- **Opcode 0x06**: Dùng cho di chuyển bước ngắn, liên tục trong nội bộ cùng một bản đồ (`map_id`). Vị trí được nội suy từng bước (walk interpolation, pathfinding 20px grid).
- **Opcode 0x0C (Relocate Map)**: Dùng khi chuyển sang bản đồ khác hoặc dịch chuyển tức thời qua cổng warp (`Warp.Dat`), phù vân hoặc lệnh GM.
  - Chu trình Warp: Client chạm cổng `[14 08 idGate]` → Server fade màn hình `[14 07]` → Server gửi `0x0C` (13 bytes) → Client nạp map mới xong gửi `[0C 01]` → Server mở khóa `[05 04]` và `[14 08]`, đồng bộ thực thể và drop trên map mới.

### 5.4. Kênh liên đới đã phát hiện (để mock server không bất ngờ)
- `FUN_0072b390` / `FUN_0072b4b8` mỗi lần play/stop action trên `TPlayer` gửi **`SendCommand(0x20)`** — OP 0x20 là kênh **action/pose report** song sinh với 0x06 (flag `+0x466` là cầu nối).
- `case 7:` của `SendCommand` (OP 0x07 C→S, nguồn `gvar_007DA5B0 + 0x138/0x130/0x134`) như `opcode_14.md` đã dẫn — đối tượng **khác** (phương tiện), không phải actor.

---

## 6. Ghi chú cho Mock Server

1. **Framing:** như mọi OP — `[44 F4][Len:Word LE][payload]`, XOR `0xAD` toàn khung; payload `[0x06][SubOp]...`; `Len` tính cả 2 byte đầu payload.
2. **Cho một NPC/monster/PK khác đi bộ trong cảnh** (dùng nhiều nhất):
   `[06][01] + actorID(4B LE) + dir(1B, 0..7 thường dùng) + destX(2B LE px) + destY(2B LE px)` — **đủ 11 byte**.
   - `actorID` **phải** là ID của actor đã spawn qua OP 0x03 (OTHER) / OP 0x04 — có mặt trong mảng 800 slot (`actor+4 == ID`). ID lạ → `FUN_0070c20c` trả 0 → gói **bị nuốt không hiệu ứng** (vô hại nhưng vô nghĩa).
   - **Không dùng SubOp 1 để sửa vị trí chính người chơi** — self không có trong mảng actor; muốn "kéo" self về 1 điểm: hoặc `[14]` script-teleport (`opcode_14.md` §4.1 B2=1/4), hoặc gửi `[06][02]` để **bắt client chốt tại chỗ + echo**.
   - `destX/destY` nên là bội số theo lưới 20px nếu muốn khớp tile `+0xF0/+0xF4` đẹp; giá trị pixel tự do vẫn hợp lệ (client tự chia).
   - `dir`: gửi `d − 8` với `d ∈ [8..15]` là hướng 0..7 hiển thị; không rõ hướng thì gửi `0` (actor vẫn đi, `FUN_0070da54` sẽ tự tính lại `+0xE3`; chỉ byte render `+0xE4` là theo wire).
3. **Điều kiện để SubOp 1 thực sự chạy trên client người nhận:**
   - Map hiện tại (`+0x63a`, do OP 0x03 SELF đặt) **không** thuộc `0xC2F7..0xC2FB` → tránh 5 map mini-game khi test move-sync thường.
   - Nếu người nhận là **leader** của actor đó (actor nằm trong bảng partner `+0x550[1..4]`, `+0x35f==1`, `+0x145c!=2`) → gói **bị filter**. Muốn member tự đi theo broadcast thì đừng kết nó vào đội của người nhận, hoặc chấp nhận hành vi "leader free-run member".
   - Payload **tối thiểu 6 byte** để đọc `dir` (thực tế luôn gửi 11).
4. **Gửi `[06][02]` khi cần:** đồng bộ/dập tắt drift vị trí trước các pha bất đối xứng (sau combat script, trước transition…). Kèm theo đó client **tự động** trả `[06][02][X][Y][sigA][nonce]` — mock server **phải chấp nhận gói echo này** (sub=2, tọa độ là `+0x4C/0x50` sau snap) và **không** được chờ ACK nào khác.
5. **Khóa đi lại có thời hạn:** `[06][02]` set `+0x653=1` làm **liệt hoàn toàn** keyboard + mouse walk (`KeyboardWalk`, `DXDraw1MouseDown`, `00712d7c` đều gate cờ này). Bắt buộc phải "trả quyền di chuyển" bằng **OP 0x14 SubOp 0x08** (transition-end, `0078a89c:3111` clear 0) khi kịch bản kết thúc — nếu không người chơi đứng yên vĩnh viễn (trừ khi client tự clear qua `FUN_0072b390` khi play action).
6. **Validate chiều C→S (server mock cần check gì):** chỉ 2 trường thực sự có nghĩa nghiệp vụ: **X, Y** (Word LE) và **sub ∈ {1, 2}**; `sigA` có thể verify `(charID % 13 + jobStateByte[0x3FA]) & 0xFF` nếu muốn chống replay thô; `sigB` là **nonce random** — không được so khớp, chỉ ghi log.
7. **Không gửi SubOp khác 0x01/0x02, không gửi payload rỗng** (`_BoundErr`), không gửi `[06][01]` ngắn hơn 11 byte.
8. **Tần suất:** đây là kênh per-step; client pop tối đa 50 frame/tick 30ms — nhịp phát move cho nhiều actor nên ≤ nhịp đó để không dồn queue.

---

## 7. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
| :--- | :--- | :--- | :--- |
| 1 | `ts_decompile/case_functions/functions/case_007_0078CB99_FUN_0078cb99.c` | @`0x0078CB99`; SubOp d.27–33; map-guard d.35; partner-loop d.39–55; 0x466/0x548 d.56–59; dir d.60–73; X/Y d.74–79; move-apply d.80–119; SubOp2 d.122–124 | **Handler chính** — toàn bộ wire + logic |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 6:` inline d.1791–1888; `653=0` d.3111 (OP 0x14/0x08) | Cross-verify 100% với (1); nguồn clear walk-lock |
| 3 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` / `jumptable_dword200_0x78A9B6.hex` | byte[6]=0x07; dword[7]=`0x0078CB99` (entry `0x78A9D2`); dword[0]=`0x00796347` | **Thực đo routing OP→Case** + đích no-op/default |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | d.1687 (`func_0x00712d4c`) | Bản gộp đối chiếu |
| 5 | `ts_decompile/functions/0070c20c_FUN_0070c20c.c` | @`0x0070C20C` (scan `gvar_007DA300[1..800]`, so `+4==ID`, count tại `gvar_007D9D34+0x60`) | Tra actor index theo ID |
| 6 | `ts_decompile/functions/0071fdf8_FUN_0071fdf8.c` + `.asm.txt` | asm `MOV CL,2; MOV DL,6; CALL 0x0077f414` | **SubOp 2** = lock + snap + echo |
| 7 | `ts_decompile/functions/0070da54_FUN_0070da54.c` | @`0x0070DA54` (đọc `+0x1C/0x20` vs `+0x4C/0x50`, ghi `+0xE3`∈0..7) | Hướng logic; chốt model offset di chuyển |
| 8 | `ts_decompile/functions/00715b28_FUN_00715b28.c` | @`0x00715B28`; VMT+0x20 ở entry; gate `0x145c==2`/`0x35f==2`+`0x548`; path `+0x100/104/108+i*8` | Route recompute; đồng bộ follower `+0x35E`(NpcCount)/`+0x57C[i]` |
| 9 | `ts_decompile/functions/00712d4c_FUN_00712d4c.c` (body mới) | d.16-23 | Phân vai **xác minh**: `func_0x00712d4c` = SetDestination (`actor+0x4C/+0x50 := X/Y`, `actor+0xE5 := 0` — xóa cờ "đứng yên", writer mới của `+0xE5`) |
| 9b | `ts_decompile/functions/0071e2b8 / 00712c58 / 00712d7c (+asm CL=1) / 0072b390 (d.73 0x466:=1, d.80 0xE5:=1) / 0072b4b8 (d.37–44)` | — | Model `+0x466`, `+0xE5` (nay có thêm writer `00712d4c`) |
| 10 | `ts_decompile/functions/005186e4_TForm1.KeyboardWalk.c` | d.43 idiom `+0x3d09`; d.62 gate `0x653`; d.85–118 goal ±`0x50`; d.120 → `00731ffc` | Map-guard + walk-lock + sender step |
| 11 | `ts_decompile/functions/0050bff8_TForm1.DXDraw1MouseDown.c` | d.58, d.74 (`0x466`), d.127 (`0x653`) | Xác nhận cùng idiom map-guard; gate click-walk |
| 12 | `ts_decompile/functions/0050debc_FUN_0050debc.c` | **literal** `"TeamStatus: "` d.153, `"LeaderID: "` d.156, `"PartnerNum: "` d.159, `"NpcCount: "` d.312; loop `+0x550+i*8` d.161–197 | **BẰNG CHỨNG TÊN TRƯỜNG** cho các offset gate |
| 13 | `ts_decompile/functions/007a273c_FUN_007a273c.c` (d.125–229) & `007a2604_FUN_007a2604.c` (d.50–61) | writer duy nhất của `0x35f/0x548/0x578/0x550`; `0x35f:=1` leader / `:=2` member; `self→idx 0` (d.104–109) | **Định nghĩa TeamStatus** + self không nằm trong mảng actor |
| 14 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 6:` d.830–883; `case 1:` d.773–818 (hiệu chuẩn thứ tự `_LStrCatN`); `case 7:` d.884–895 | **Chiều C→S** OP 0x06 |
| 15 | ASM call-sites: `00731ffc…asm.txt` (cuối), `00712d7c…asm.txt` d.325–331, `0071fdf8…asm.txt` d.33–40, `0074b330…asm.txt` d.126–194 | `MOV CL,1/2; MOV DL,6` | **sub-code C→S** = 1 (tự nguyện) / 2 (ép) |
| 16 | `ts_decompile/functions/00402cf0_FUN_00402cf0.c` | LCG `0x8088405` | sigB = **nonce PRNG**, không phải key-state |
| 17 | `ts_decompile/functions/0058b5a4_FUN_0058b5a4.c` (`-0xc2f7` index), `0051c2ec`, `00733b94` (`<6`) | — | Corroborate họ map `0xC2F7..` |
| 18 | `ts_decompile/functions/00603f20_FUN_00603f20.c` | d.188–190 `TPlayer_Create(VMT_70B740_TPlayer)` → `gvar_007DA7BC`; d.164–181 free `gvar_007DA300[i]` + `gvar_007D9D34+0x60` reset | **Định danh 3 global** (player-object / world-manager / 800-slot actor array) |
| 19 | `0051189c_FUN_0051189c.c` (registry VMT/form) | quét: **không** chứa `gvar_007DA7BC/007D9D34/007DA300` (chúng tạo động ở (18)) | Loại trừ nghi ngờ, đối chiếu theo yêu cầu |
| 19b | `ts_decompile/functions/0054a384_FUN_0054a384.c` d.44–152 (body mới 2026-09-14) | tra bảng `DAT_00948DF8` → class 0..3 cho `+0x145c` | Nguồn giá trị `+0x145c==2` ở cổng #1 (mục (b) cũ trong danh sách thiếu) |
| 20 | `.scratch/op-code/`: `handoff-opcode-exploration-guide.md`, `opcode_00_01.md` (mẫu + framing), `opcode_02.md`, `opcode_03.md` (`+0x63a` MapID, `0x145c`←`0054a384`), `opcode_08.md` (`0x3FA` job-state, `0x57C` party), `opcode_14.md` (0x653-cleared-by-0x08, C→S 6/7, tile 20px) | — | Đối chiếu helper nghiệp vụ đã biết, tránh trùng phỏng đoán sai |

**Hằng số/cấu trúc còn thiếu cho 100%:** ~~(a) thân `func_0x00712d4c`~~ **ĐÃ BỔ SUNG** — body mới `00712d4c_FUN_00712d4c.c` xác minh SetDestination + xóa `+0xE5`; ~~(b) `func_0x0054a384`~~ **ĐÃ BỔ SUNG** — body mới là hàm **tra bảng MapID tại `DAT_00948DF8`** (trả 0..3, xem bảng §4.1.3; bản thân nội dung bảng vẫn chưa dump); **(c) vẫn mở**: nội dung kinh doanh 5 map `0xC2F7..0xC2FB` (không có chuỗi literal nào gắn với chúng); **(d) vẫn mở**: tên thật của virtual `VMT+0x20` trên `TPlayer` (`VMT_70B740+0x20`). (c)+(d) đã được thế chỗ bằng bằng chứng gián tiếp ≥ trung bình và **không** ảnh hưởng wire layout hay thứ tự phát gói của mock server.
