# PHÂN TÍCH — Main OP 0xC7 (199) / Case 65 / FUN_007962fc @ 0x007962FC

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh phần khung và dispatch từ mã nguồn sơ cấp** (`ts_decompile/` only). Hai callee (`func_0x00613634`, `func_0x0061353c`) **NAY ĐÃ CÓ BODY** (HOLE `0x00612E1A→0x00614C38` đã giải phần lớn trong redump 2026-09-14) → wire + hành vi đặc tả xong tại §4.1.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

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
  - Main OP `0xC7` (cập nhật từ body mới): kênh **chat announce có UID** — SubOp 4 = tag 3 (GM/thì thầm), SubOp 6 = tag 1 (Thiên thần) + sound `WA0033.wav`; không thao tác state nào của `TGmManage` (§4.1).
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
| **`0x04`** | `[C7][04][UID:4B LE][Msg:RP[5..]]` | 6B | `FUN_00613634` (**body mới**) | **✔** | Tin nhắn chat **tag 3** (kênh `(GM)`/`Thì Thầm`, `opcode_02.md:157-158`) tới `TTalkMsgForm`. |
| **`0x06`** | `[C7][06][UID:4B LE][Msg:RP[5..]]` | 6B | `FUN_0061353c` (**body mới**) | **✔** | Tin nhắn chat **tag 1** (kênh `Thiên thần`, `opcode_02.md:106`) + phát `sound\WA0033.wav`. |
| **Khác** | `[C7][00]`, `[C7][01]`... | 2B | Không có | — | **No-op im lặng** (bỏ qua không xử lý). |

---

## 4. Chi Tiết Các Nhánh & Hiện Trạng Callee

### 4.1. Hai callee OP 0xC7 — ĐÃ PHÂN TÍCH TỪ BODY MỚI (2026-09-14)

- `FUN_0061353c` (`index.csv:6368`, 208B; caller `sub_00796342` = nhánh SubOp 6 của `case_065`) — `functions/0061353c_FUN_0061353c.c:46-59`:
  `uid = DWORD_LE(RP[1..4])` (`_LStrCopy(RP,2,4)` + `FUN_0077ef7c`, codec ERangeError nếu chuỗi <4 ký tự ⇒ payload L≥7 kể cả MainOp), `msg = RP[5..end]` (`Copy(RP,6,len-5)`) → **`FUN_007ab870(gvar_007DA1B0↑, uid, msg, tag=1)`** (kênh `Thiên thần`; id∈[100,400] đổi hiệu ứng theo `opcode_02.md:106`) → **`FUN_007a7f20(gvar_007DA010 + "sound\WA0033.wav")`**. Self `TGmManage` **không dùng field nào**.
- `FUN_00613634` (`index.csv:6369`, 174B; caller `sub_00796331` = nhánh SubOp 4) — `functions/00613634_FUN_00613634.c:44-55`: cùng cấu trúc, **tag=3** (`(GM)`/`Thì Thầm` — `opcode_02.md:157-158`), **không phát âm thanh**.
- ⇒ **Đính chính lớn**: 0xC7 **không phải** "kênh lệnh điều hành GM nhiều loại" — 2 SubOp duy nhất chỉ là **chat announce có UID** (GM broadcast). Nhận định chống `OP_RECONNECT` ở §1 vẫn giữ nguyên giá trị.
- Bối cảnh HOLE cũ (giữ làm sử liệu): `index.csv:3971` `FUN_006128bc` end `0x612E1A`, `index.csv:3972` `FUN_00614c38` — toàn bộ `0x00612E1A→0x00614C38` từng chưa export; **nay đã export 15 hàm** trong dải (`index.csv:6365-6379`: `00613230`…`00614bb0`), chi tiết phân tích các hàm dùng chung với OP 0x10 xem `opcode_10.md` (§ "ĐÃ CÓ BODY").

### 4.2. Mối liên hệ với Main OP 0x10

Trong `ts_decompile/functions/0078a89c_FUN_0078a89c.c:2875-2941` (Case 16 — OP 0x10), dispatcher gọi liên tiếp các phương thức của `gvar_007DA160`:
(Bốn nhãn cũ dưới đây là **suy đoán không có body**; nay cả 4 đã có body — bản phân tích chi tiết nằm ở `opcode_10.md:151-160`, **đính chính**:
- `FUN_00613e48` (`index.csv:6373`): NOT "truy vấn tài khoản" — parse `[idA:4B][mode:1B][idB:4B]`, tra tên 2 nhân vật, chat announce + set cờ `TSe_SendMailForm(gvar_007DA548)+0x131` (mode 1/2).
- `FUN_00614318` (`index.csv:6375`): NOT "teleport" — đọc `id:4B` + 3 Word, ghép 8 mảnh vào **memo form GM** (`gvar_007DA110+0x384`'s control `+0x208`).
- `FUN_00614bb0` (`index.csv:6379`): body thực tế **chỉ cấp phát stack-probe ~13KB rồi dọn chuỗi — không hành vi quan sát được**; nhãn "sinh vật phẩm/quái" không được ủng hộ (stub / decompile chưa đủ — chưa kết luận được).
- `FUN_006140d4` (`index.csv:6374`): NOT "cấm chat/kick" — `RP[1]=n` byte tiêu đề → memo GM qua `FUN_00614c38` (Ansi→Wide qua `FUN_007c8000`), rồi lặp record `[id:4B][len:1B][name]` in `"…id Name:name"`.
Tất cả decode qua codec chuẩn `gvar_007D9D30` giống §4.1.
- `func_0x0061353c`: Được gọi tại dòng 7500 (SubOp `0x06` của OP 0xC7).
- `func_0x00613634`: Được gọi tại dòng 7496 (SubOp `0x04` của OP 0xC7).

Điều này xác nhận rằng: `func_0x0061353c` và `func_0x00613634` là các **phương thức thành viên của lớp `TGmManage`** (liên kết tĩnh — call-site truyền self rồi `CALL` trực tiếp; đã xác nhận bằng body mới). Khi nhận gói `0xC7 04` / `0xC7 06`, Client chuyển giao `RestPayload` cho `TGmManage` tự parse theo công thức §4.1.

### 4.3. Dump VMT `0x00613160` (`redump/vmt_613160_TGmManage.hex`) — đối chiếu `index.csv` (mục mới 2026-09-14)

File chứa **128 byte = 32 dword LE** tính từ `0x613160`. Resolve từng slot khác 0 (chỉ con trỏ LE, **không khẳng định ngữ nghĩa slot**):
| Slot @địa chỉ | Giá trị | Resolve qua `index.csv` |
| :-- | :-- | :-- |
| `+0x00` @`0x613160` | `0x006131AC` | không có entry; vùng đích **chính là chuỗi ngắn tên lớp**: dump tự phủ tới `0x6131AC` = byte `09 'TGmManage'` |
| `+0x04..+0x1C` | `0` ×7 | rỗng |
| `+0x20` @`0x613180` | `0x006131AC` | nt. (trùng slot 0) |
| `+0x24` @`0x613184` | `0x00000008` | không phải con trỏ hợp lệ (số nguyên nhỏ, ghi nhận thô) |
| `+0x28` @`0x613188` | `0x00401100` | không có entry (dưới entry đầu `0x00402864` — RTL Borland, chưa export) |
| `+0x2C..+0x3C` @`0x61318C–0x61319C` | `0x00403298/2A4/2A8/2AC/2A0` | không có entry (khe RTL giữa `FUN_00403284` end `0x403295` và `FUN_00403440`) |
| `+0x40/+0x44` @`0x6131A0/0x6131A4` | `0x00403004`, `0x00403018` | không có entry (RTL) |
| `+0x48` @`0x6131A8` | `0x006131FC` | **không có entry** — vẫn trong vùng chưa export của HOLE cũ |
| `+0x4C` trở đi @`0x6131AC+` | `0x6D475409…` | không còn là con trỏ: bytes tên lớp `\x09TGmManage` rồi **prologue code** (`8B C0 55 8B EC …`, entry chưa export ~`0x6131B6`) |

- **Kết luận an toàn**: (a) **không slot nào** trong cửa sổ 128B trỏ tới 6 method đã phân tích (`0061353c/00613634/00613e48/006140d4/00614318/00614bb0`) — khớp call-site liên kết tĩnh; (b) dump **kết thúc giữa prologue code** nên bảng VMT đầy đủ của `TGmManage` chưa nằm trọn trong file; (c) cấu trúc **y hệt** `redump/vmt_75C0B4_TLifeManage.hex` (slot0 = slot8 = base+0x4C; số `8`; cụm RTL `0x401100`/`0x4032xx`) — ở `TGmManage` đích `base+0x4C` được chứng minh là **tên lớp**, gợi ý `0x75C100` của `TLifeManage` cũng là tên lớp, nhưng **chưa verify** (không có dump bytes).

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
3. **Cấu trúc gói tối thiểu (đã đặc tả từ body mới)**:
   - `F4 44 [Len≥7: 2B LE] C7 04 [UID:4B LE][Msg:1..n]` → chat tag 3 (GM)
   - `F4 44 [Len≥7: 2B LE] C7 06 [UID:4B LE][Msg:1..n]` → chat tag 1 (Thiên thần) + `sound\WA0033.wav`
   - `UID` chỉ là id hiển thị/định tuyến nội bộ `FUN_007ab870` (`opcode_02.md:106`); hai handler không có guard nào khác ngoài codec (`L≥7`).

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
| `FUN_0061353c` | `ts_decompile/functions/0061353c_FUN_0061353c.c` | 46–59 | Handler SubOp 0x06 (body mới, `index.csv:6368`) |
| `FUN_00613634` | `ts_decompile/functions/00613634_FUN_00613634.c` | 44–55 | Handler SubOp 0x04 (body mới, `index.csv:6369`) |
| `FUN_00613e48/006140d4/00614318/00614bb0` | `ts_decompile/functions/00613e48_FUN_00613e48.c` · `006140d4_FUN_006140d4.c` · `00614318_FUN_00614318.c` · `00614bb0_FUN_00614bb0.c` | `index.csv:6373-6375,6379` | Phương thức GM dùng chung OP 0x10 — phân tích chính ở `opcode_10.md:151-160` |
| `VMT dump 0x613160` | `ts_decompile/redump/vmt_613160_TGmManage.hex` (128B) | — | Bảng phân tích slot §4.3 |
