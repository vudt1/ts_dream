# PHÂN TÍCH — Main OP 0x39 (Case 50) — `FUN_00795579` @ `0x00795579` — **Điều khiển popup TSportManage: mở form theo id / đóng form hiện tại / hiện banner (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x39] = 50` → entry `0x0078AA7E` của `jumptable_dword200_0x78A9B6` → target `0x00795579` = **Case 50** (tính trực tiếp từ 2 file `.hex` trong `redump/`). Đối chiếu kép: inline trong dispatcher `0078a89c_FUN_0078a89c.c:6857–6919` (marker SEH `UNK_0079558c/007955d4/0079564b/00795663`) khớp 100% `case_050`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).
> **Thay đổi lớn nhất**: HOLE `0x0055355D–0x00553818` đã được decompile — `func_0x0055374c` **có body** (`FUN_0055374c`), suy đoán `OpenForm(id,flag)` **XÁC MINH ĐƯỢC**, đồng thời định danh class của **cả 6 form** trong bảng id (kể cả `gvar_007DA778 = TRE_ZMChessMain` — khớp nghi vấn các tài liệu anh em). 4 method form id3 thuộc OP 0x3C cũng đã có body (§5.3).

> Phạm vi: **core logic opcode**; chi tiết render/animation của form chỉ nêu 1 dòng để chứng minh consumer dữ liệu.

---

## 1. Tóm tắt nghiệp vụ

OP 0x39 là **"remote control" của `TSportManage`** — manager cụm popup game (nghi là bảng điều khiển cá cược/thể thao theo chính tên class) nằm trên main form. `SubOp = RP[1]` (1-based; `RP` = payload đã bỏ MainOp), switch trên 1/2/3:

| SubOp | Hành động |
| :--- | :--- |
| `1` | **Mở form popup theo id**: `FUN_0055374c(mgr, RP[2], 0)` (HOLE **đã có body** 2026-09-14 — xem §4.1). Nếu `RP[2] == 3` thì đọc thêm `RP[3]`: `1` → ghi `form#3 + 0x38 = 100`; `2` → `= 1000` (value tùy chọn, xem §5.3). |
| `2` | **Đóng & giải phóng form đang mở**: `FUN_00553410(mgr)` — free form theo `mgr+4` hiện tại, set `mgr+4 = 0` (§4.2). Không đụng socket/queue. |
| `3` | **Banner/thông báo 1.2 giây**: `RP[2]==1` → hiện chuỗi `&UNK_00799200`, `RP[2]==2` → `&UNK_00799224`, qua slot `+0x90` của `TSe_TalkMsgFormPlus` (`gvar_007DA084`), duration `0x4b0 = 1200ms` (§4.3). |

- **C→S: KHÔNG có đường gửi 0x39** — `case 0x39:` trong builder `FUN_0077F414` **tồn tại nhưng rỗng** (`break;` không làm gì, `0077f414_FUN_0077F414.c:1020–1021`). Chi tiết mâu thuẫn hiếm: client **có** hàm phát yêu cầu gửi 0x39 (`FUN_00553818`) từ callback nút của form#1, nhưng nó rơi vào case rỗng nên **không frame nào ra lỗi** (§6).
- Chuỗi banner `0x799200/0x799224` **đã dump + decode VISCII 2026-09-14** (§7): `0x799200`="Hiện giờ cấm công cụ này", `0x799224`="người chơi nàyHiện giờ cấm công cụ này" — mock server không cần gửi text (client tự in literal cục bộ).

---

## 2. Entry & cách đọc payload

Framing/dispatcher tuân theo tiền lệ đã xác minh: `[Token F4 44][Len:Word LE][Payload]` ở **không gian đã giải mã**, toàn bộ frame XOR `0xAD` (bao gồm cả token — bằng chứng §8); `TForm1.CY_DelRevQueue` (`00516158_TForm1.CY_DelRevQueue.c:86–93`) cắt `_LStrCopy(msg, 2, Len-1)` → **RP = payload bỏ MainOp**, chuỗi Delphi 1-based: `RP[1]` = SubOp, `RP[2]` = byte đầu tiên của tham số, `*(int*)(RP-4)` = độ dài.

`FUN_00795579` (`case_050_00795579_FUN_00795579.c:8–77`, ĐÃ ĐỌC TOÀN BỘ):
```pascal
// Tương đương Delphi (bỏ guard BoundErr):
SubOp := Ord(RP[1]);                       // BoundErr nếu RP rỗng
case SubOp of
  1: begin
       TSportManage_OpenForm(RP[2], 0);    // FUN_0055374c — body ĐÃ có (§4.1)
       if RP[2] = 3 then
         case RP[3] of                     // BoundErr nếu thiếu byte
           1: TPortForm3.Field_38 := 100;
           2: TPortForm3.Field_38 := 1000;
         end;
     end;
  2: TSportManage_CloseCurrent;            // FUN_00553410 (§4.2)
  3: case RP[2] of
       1: TalkMsg.Banner(@UNK_00799200, 1200, 0, 0);   // VMT+0x90 (§4.3)
       2: TalkMsg.Banner(@UNK_00799224, 1200, 0, 0);
     end;
end;
// SubOp khác {1,2,3}: im lặng, không làm gì.
```
Toàn bộ `if len < k → _BoundErr` là guard index Delphi (`case_050.c:26,35,42,49,67`); vượt ngưỡng = **ERangeError giữa handler**, không phải no-op. Đoạn `case_050.c:78–105` là SEH cleanup frame dùng chung của dispatcher (`*in_FS_OFFSET`, `LAB_00796408`, immediate `0x7963xx`) — không phải logic.

---

## 3. Wire layout (offset trên Payload, `[0]` = MainOp)

| Offset | Nội dung | Ràng buộc / xử lý client |
| :--- | :--- | :--- |
| `[0]` | `0x39` | — |
| `[1]` | SubOp | chỉ 1/2/3 có nghĩa; khác → bỏ qua im lặng; **payload rỗng → BoundErr** |
| `[2]` (SubOp 1) | `id` form muốn mở | id hợp lệ **xác minh được từ body mới**: đúng `{1,2,3,4,6,0xFF}` (switch `FUN_0055374c`, §4.1) — id khác → return, không mở form, không ghi `mgr+4` |
| `[3]` (SubOp 1, chỉ khi `id==3`) | mode | `1` → form3`+0x38`=100; `2` → =1000; khác → không ghi. **Thiếu byte → BoundErr** |
| `[2]` (SubOp 3) | mã banner | `1` → literal `0x799200`; `2` → `0x799224`; khác → im lặng. Thiếu byte → BoundErr |

Không có trường Word/DWORD/chuỗi nào — mọi thứ là byte đơn. SubOp 2 không có tham số.

---

## 4. Phân tích từng SubOp + helper

### 4.1. SubOp 1 — `FUN_0055374c(mgr, id, 0)` : **XÁC MINH ĐƯỢC từ body mới (2026-09-14)** — đúng là `OpenForm(id, flag)`

HOLE `0x0055355D–0x00553818` đã được decompile: file `ts_decompile/functions/0055374c_FUN_0055374c.c` (62 dòng, 201 byte; header ghi caller duy nhất `sub_007955cf` — đúng khối case 50, `0055374c_FUN_0055374c.c:8,13`). Bản 2026-09-12 mô tả đúng vị trí HOLE (khe giữa `FUN_00553528` kết thúc `0x0055355D` và `FUN_00553818`) — **đính chính: nay không còn là HOLE nữa**.

**Hành vi thực của body** (Delphi register: `param_1`=EAX=`mgr`, `param_2`=EDX=`id` (mask `&0xFF`), `param_3`=ECX=`flag`):

| `id` | Hành động (`0055374c_FUN_0055374c.c`) | Global nhận con trỏ form |
| :---: | :--- | :--- |
| `1` | `TRE_BiDaXiao::Create(VMT_547800, 1, flag)` — dòng 30-31 | `gvar_007DA42C` (object OP 0x3A) |
| `2` | `TSBDManager_Create(VMT_550A2C,1)` — dòng 34-35 | `gvar_007DA0F4` (object OP 0x3B) |
| `3` | `TRE_ZMChessMain_Create(VMT_53D8B8, edx=1, flag)` — dòng 41-44 | `gvar_007DA778` |
| `4` | `TLottoManager_Create(VMT_54D580,1)` — dòng 26-27 | `gvar_007D9F98` |
| `6` | `TMachineManager_Create(VMT_54D6D8,1)` — dòng 48-49 | `gvar_007DA4EC` |
| `0xFF` | `TSportDemo_Create(VMT_552FA0,1,flag)` — dòng 55-56 | `gvar_007DA0A0` |
| khác | **return ngay, không ghi gì** (dòng 38-39, 52-53) | — |

Kết thúc mọi nhánh hợp lệ: `*(byte*)(mgr+4) = id` (dòng 58) — **đây chính là writer của `ActiveId`** mà bản cũ tìm không thấy (xem §5.1 cập nhật).

Kết luận & hệ quả:
- Suy đoán `OpenForm(id,flag)` của bản cũ → **xác minh được từ body mới**; bảng id→class trên **khớp 6/6** với map đóng của `FUN_00553410` (§4.2), xác nhận cơ chế create/close đối xứng.
- **Không có guard "đã mở form nào chưa"**: nếu server gửi SubOp 1 khi `mgr+4 ≠ 0`, form cũ bị **rò rỉ** (slot global bị ghi đè, form cũ không Free) — mock nên luôn phát SubOp 2 trước khi đổi id (hoặc xác nhận bằng live).
- `flag` (ECX) chỉ được chuyển tiếp vào constructor của id 1/3/0xFF; id 2/4/6 nhận `flag` bị bỏ (constructor 2 tham số). Với call-site OP 0x39, `flag = 0` hằng định.
- Chi tiết id==3 (`RP[3]` → form3`+0x38` = 100/1000) **không nằm trong** `FUN_0055374c` — vẫn do dispatcher `case_050.c:53-57` xử lý (như mô tả cũ).

### 4.2. SubOp 2 — `FUN_00553410(gvar_007D9D88)` : đóng form hiện tại (PHÂN TÍCH KỸ)
`00553410_FUN_00553410.c:24–53` (211 bytes):
```pascal
id := TPortManage(mgr).ActiveId;            // byte @ mgr+4
if id <> 0 then begin
  case id of
    1:   Form_DA42C.Free;   Form_DA42C  := nil;   // :32-34   ← object của OP 0x3A
    2:   Form_DA0F4.Free;   Form_DA0F4  := nil;   // :36-38   ← object của OP 0x3B
    3:   Form_DA778.Free;   Form_DA778  := nil;   // :40-42   ← form của SubOp 1 mode 3
    4:   Form_D9F98.Free;   Form_D9F98  := nil;   // :28-30
    6:   Form_DA4EC.Free;   Form_DA4EC  := nil;   // :44-47
    0xFF:Form_DA0A0.Free;   Form_DA0A0  := nil;   // :48-51
  end;
  TPortManage(mgr).ActiveId := 0;                 // :52
end;
```
- Chỉ gọi `TObject.Free @ 0x403074` (`00553410.c:11`) — **không** socket, **không** `CY_AddSedQueue`, **không** ngược về C→S. Đây là lệnh đóng của server, không có ack.
- Cùng khuôn map id này có 2 "anh em" là method khác của TSportManage (cùng dải địa chỉ): `FUN_00553840` (`00553840.c:28–50`, caller tại `0x00515d8e` — **HOLE** vùng tick loop `~0x5157E6`, theo handoff) gọi **hàm tick/refresh từng form** (`0x548080/0x5524c4/0x541c6c/0x54e51c/0x54fbac/0x5531f4`), và `FUN_0055391c` (`0055391c.c:28–50`, caller `0x00515c15` — cùng HOLE tick) gọi nhịp thứ hai (`0x548e98/0x552e58/0x541f8c/...`). Tức `ActiveId` quyết định form nào được pump mỗi tick.
- `FUN_00603f20` (hàm "rời world/reset kết nối", `00603f20.c:7,100–101`) cũng gọi `FUN_00553410` tại `0x006043e5` — khớp tiền lệ `opcode_36.md §5.2` (cùng hàm này Hide form chọn server).

### 4.3. SubOp 3 — gọi `VMT+0x90` của `gvar_007DA084`: **TỰ KIỂM CHỨNG (không kế thừa opcode_2b/33)**
**Danh tính**: `gvar_007DA084` = instance **`TSe_TalkMsgFormPlus`** — classref `VMT_63B39C`, sinh trong FormCreate (`0051189c_FUN_0051189c.c:1265–1266`), đặt panel `"panel10"` Y=250 ngay sau đó (`:1275`), đăng ký vào widget-manager `gvar_007DA234` (`:1277`, global này sinh tại `0051189c.c:1156`).

**Xác định gốc VMT**: nhãn `VMT_63B39C` chính là giá trị classref lưu trong object. Chứng minh: hàm `FUN_007AFA94` (virtual hệ thống, ~190 class có) nằm tại `classref+0x7C` cho MỌI class kiểm tra được — `0x63b418-0x63b39c=0x7C` (TalkMsg), `0x5c709c-0x5c7020=0x7C` (TFightForm1), `0x707c64-0x707be8=0x7C` (form danh sách server), `0x5f3520-0x5f34a4=0x7C` (TAC_FaceSel)... ⇒ **slot +0x90 = `*[0x63B39C+0x90] = *[0x63B42C]`**, và header `007badb0_FUN_007badb0.c:26` xác nhận `0063b42c -> 007badb0 [DATA]`. ⇒ **VMT+0x90 = `FUN_007BADB0`**, không phải override cục bộ của TalkMsg.

**Thân `FUN_007BADB0`** (asm `007badb0_FUN_007badb0.asm.txt:4–25`): lưu `EAX=self, DL, ECX`; `PUSH [EBP+8]` (arg stack đầu = con trỏ chuỗi); gọi `FUN_007AFBF8` rồi `FUN_007AFEF8`; `FUN_007AFEF8` (`007afef8.c:66–76`) duyệt chuỗi node `node := node[+8]` đến node cuối (tail of list), kiểm tra byte `node+0x14` — nếu busy thì `RET 0x4` (pop 1 arg) **bỏ qua thông báo**; đuôi `0x7AFC1F–0x7AFC24` của `FUN_007AFBF8` **vẫn cắt cụt sau re-export 2026-09-14** — `.asm.txt` kết thúc đúng tại `JZ 0x007afc1f` (`007afbf8_FUN_007afbf8.asm.txt:14`) rồi `CMP/MOV ESP,EBP/POP EBP/RET 0x4`; khối đích tại `0x007AFC1F` (nơi enqueue/format thực tế) không thuộc file export nào ⇒ phần enqueue của slot `+0x90` **tiếp tục unseen**.
Slot +0x90 này xuất hiện ở nhiều widget `TSe_*` cùng hàm (`0x7af124/0x7af204/0x7af2c8/0x7af3a8/...` trong `007badb0.c:15–35`; `0x7af204 = VMT_7AF174 + 0x90` của `TSe_FixedButton`) — **API banner chung của họ widget**.

**Đọc nghĩa theo giả định gọi (cross-check 20+ call-site)**: luôn là `(self, <ptr chuỗi literal>, ms, 0, <0|1>)` với `ms ∈ {1000, 0x4b0=1200, 2000, 3000, 5000, 6000, 10000}` trong chuỗi lời lỗi/validate UI (`00506bf4.c:28-47`, `00508204.c:36`, `0051f5f4.c:110-125`, `0054790c.c:28-42`, `00552008.c:49-52`...). Một thuật toán banner đầy đủ THỰC SỰ được export ở **override slot +0xDC** cùng class: `*[0x63B478] = FUN_0063C84C` (`0063c84c.c:20`) → wrapper `FUN_007BC1C8(self, msg:string, ms, styleByte, altFlag)` (`007bc1c8.c:44–77`: gắn timestamp từ tick counter `**(gvar_007D9D20)`, lưu chuỗi qua `FUN_007BC998`, `ms<1` → tắt auto-hide (+0x5e), ngược lại lưu duration; `altFlag==0` → gọi virtual +0x20 để hiện/refresh). **Kết luận**: đọc "VMT+0x90 = hiện banner/thông báo tạm thời" (như `opcode_2b.md §1` gọi "banner") **PHÙ HỢP TOÀN BỘ bằng chứng**; `0x4b0 = 1200ms` cùng đơn vị/cùng thang đo với các call-site anh em — **không** còn là giả định thuần. (Chênh lệch `+0x90` vs `+0xDC` là 2 entry khác nhau cùng họ widget; giữ nguyên gọi `+0x90` như dispatcher ghi.)

**Literal** `&UNK_00799200` / `&UNK_00799224` (cách nhau 0x24): đã dịch (banner "cấm công cụ" — gợi ý cơ chế anti-cheat/lock tool, chưa chốt tên tính năng): xem §7.

**Đối chiếu inline `0078a89c.c:6912,6916`**: cùng 1 gọi, inline chỉ hiện 2 arg (`..., &UNK_00799200, 0x4b0)`) vs standalone 4 arg (`..., 0x4b0, 0, 0`) — nhiều suy luận tham số của Ghidra (asm đúng `RET 0x4`), bản standalone giữ đầy đủ hơn; thuật toán giống hệt.

---

## 5. Global & ngữ cảnh sống

### 5.1. `gvar_007D9D88` = **TSportManage** (VERIFIED)
- Sinh trong `TForm1.FormCreate`: `piVar6 = TSportManage_Create((int *)VMT_5532F4_TSportManage,1,...); *(int**)gvar_007D9D88 = piVar6;` (`0050a4a0_TForm1.FormCreate.c:629–630`); constructor thuần TObject, không init field (`005534e4_TSportManage.Create.c:20–42`) → byte `+4` = 0 lúc khởi động (zero-fill).
- **`mgr+4` = ActiveId** — popup đang mở (bảng id→form §4.2). ~~Writer của `+4`: KHÔNG có trong SSOT export~~ → **XÁC MINH ĐƯỢC từ body mới**: writer là `FUN_0055374c.c:58` (`*(byte*)(mgr+4) = id`, chạy sau khi tạo form thành công); writer清零 là `FUN_00553410.c:52` (đã biết). Người đọc `+4` (hữu ích cho mock — chứng minh "1 form/lần"): guard chuột/phím `005186e4.c:68`, `0050bff8.c:135`, `00566884.c:27`, `005eee60.c:60`, `00611330.c:30`, `00717e78.c:165`, `0073ce00.c:35`, `007c511c.c:29`, và `00777448.c:34`.
- Click/nút của form#1 được router tới TSportManage: `FUN_00547110` (`00547110.c:24`, gắn trong bảng callback tại `0x547081` ∈ `FUN_00546F8C`) → `FUN_005467D0` (`005467d0.c:24`) → §6.

### 5.2. Bảng popup id→form (cột **Class** mới — xác minh được từ body `FUN_0055374c` 2026-09-14)

| id | Global | **Class (mới, từ OpenForm)** | Tick/refresh (nhánh id của `FUN_00553840`/`0055391c`) | Ghi chú khác |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `gvar_007DA42C` | **`TRE_BiDaXiao`** (VMT `0x547800`) | `FUN_00548080` (`00548080.c`, máy trạng thái animation `+0x61`) | Case `0x3A` dispatcher `:6932` bơm payload vào `FUN_00547C84`; **gọi xin gửi 0x39** §6 |
| 2 | `gvar_007DA0F4` | **`TSBDManager`** (VMT `0x550A2C`) | `0x5524c4` / `0x552e58` | `00553528.c:21–23`: double-click → `Form2.Close(vmt+4)`; **chính là object của OP 0x3B** (`opcode_3b.md`) |
| 3 | `gvar_007DA778` | **`TRE_ZMChessMain`** (VMT `0x53D8B8`) — khớp nghi vấn "TRE_ZMChessMain" của `opcode_3a/3c.md`, **xác minh được** | `0x541c6c` / `0x541f8c` | **form mục tiêu của SubOp 1 mode==3**; chi tiết dưới |
| 4 | `gvar_007D9F98` | **`TLottoManager`** (VMT `0x54D580`) | `0x54e51c` / `0x54f224` | |
| 6 | `gvar_007DA4EC` | **`TMachineManager`** (VMT `0x54D6D8`) | `0x54fbac` / `0x550700` | |
| 0xFF | `gvar_007DA0A0` | **`TSportDemo`** (VMT `0x552FA0`) | `0x5531f4` / `0x5532a4` | `00553528.c:25–27`: double-click → `FormFF.Close(vmt+4)` |

`00553840/0055391c` được pump từ vùng tick `0x5157E6` (HOLE — handoff). Nội dung 2 hàm tick id3: `FUN_00541c6c.c:45–205` — vòng quay 8 ô × 7 phần tử (`+0x158` đếm, `+0x159+i*7` record), đếm ngược `+0x18`, âm thanh `"Sound\WA0045.wav"` (`:187`), vẽ số tại tọa độ `(0x186,0x122)` (`FUN_0079c150 :194`) — 1 dòng chứng minh consumer: giống **form quay số/số gà nước**. `FUN_00541f8c.c:64–189` — nhận `+(0x84)` từ `gvar_007DA7BC+0x12f8` (player/avatar), giật chuỗi `+0x1c`... (chi tiết UI, bỏ qua).

### 5.3. `gvar_007DA778` (form id3) và field `+0x38`
- 16 reference duy nhất trong SSOT: free/reset (`00553410.c:40–41`), 2 tick (§5.2), **2 write của chính OP 0x39** (`case_050.c:54,57`), và 4 method `00541098/005408F4/0053FD78/0053F8FC` được **OP 0x3C** bơm payload (dispatcher `:6972–6984`, `case_053_0079575C.c:29–38`) — ~~cả 4 đều trong HOLE~~ → **2026-09-14: CẢ 4 ĐÃ CÓ BODY** (`functions/00541098/005408f4/0053fd78/0053f8fc_FUN_*.c`; HOLE `0x0053F8FC..0x005418F8` đã decompile). Cross-check nhanh `00541098` (OP 0x3C SubOp 1 — caller `sub_00795796` khớp `case_053`): method của **form id3**, `switch(RP[1])` 5 nhánh `1..5` (dòng 100-244) ghi byte vào `self+0x1D` và 4 byte chuỗi vào `self+0xAC..0xAF` rồi nhân bản sang sub-object `*(self+0x20)+0x12` — **bơm dữ liệu game vào form**. Xem `opcode_3c.md` (không thuộc phạm vi file này).
- `+0x38` (int): **chỉ được ghi 100/1000 bởi OP 0x39 SubOp 1; KHÔNG có bất kỳ read/export nào** (quét `+ 0x38)` trong khoảng địa chỉ 0x53E000–0x542800 = 0 kết quả). **Re-check 2026-09-14**: grep `+ 0x38` trong **cả 4 body mới** của các method id3 → **0 hit** ⇒ nghi vấn "giá trị cửa/cược" cho 2 chế độ của form quay số vẫn **UNKNOWN, không khẳng định** (consumer có thể nằm ngoài dải đã dump).
- Khai sinh object `gvar_007DA778`: ~~không có `*(int **)gvar_007DA778 = ...` trong toàn bộ export ⇒ sinh lazy trong HOLE (nhiều khả năng chính `func_0x0055374c`)~~ → **XÁC MINH ĐƯỢC từ body mới**: đúng là `func_0x0055374c` — `*(int **)gvar_007DA778 = TRE_ZMChessMain_Create(...)` tại `0055374c_FUN_0055374c.c:41-44` (id==3). Grep lại toàn bộ re-export 2026-09-14: đây là **writer duy nhất** ngoài lệnh gán `0` của `00553410.c:41`.

### 5.4. Global phụ
- `gvar_007DA084` = `TSe_TalkMsgFormPlus` (§4.3).
- `gvar_007D9D30` = **TFConnect** — kênh game-server thứ hai theo handoff (`handoff-2026-09-10-protocol-analysis.md`, mục Wire format); được truyền làm `param_1` của `FUN_0077F414` ở §6 và được Form id1 "tự-inject" message `DL=0x1A` vào dispatcher (`00548080.c:154–160`). **Re-check 2026-09-14**: grep re-export toàn bộ `.c` (functions/ + case_functions/) cho các biến thể `gvar/DAT/UNK_007D9D30 =` — **vẫn không có dòng gán slot** (chỉ có hàng chục lượt đọc làm codec self) ⇒ writer của nó vẫn nằm ngoài vùng dump; giữ nguyên trạng thái suy đoán.
- `gvar_007DA5A0` = object dữ liệu lớn (mảng `+0x9a10+i*4`, flag `+0xa096`) — `FUN_00553818` set flag `=1` (§6); ~~sinh trong HOLE~~ → **re-check 2026-09-14**: các body HOLE mới (kể cả `0055374c`) **không chứa lệnh gán slot** `gvar_007DA5A0` (chỉ ghi field nội bộ vd `007994a4.c:52-103`); creator **vẫn chưa tìm thấy — giữ là suy đoán**.
- `gvar_007D9D20` = con trỏ tick-counter dùng làm timestamp banner (§4.3).

---

## 6. Chiều C→S — case TỒN TẠI nhưng RỖNG (feature bị tắt/dang dở)

**a) Builder**: `0077f414_FUN_0077F414.c:768` `switch (param_2 & 0xff)` (gate `*gvar_007DA3A0 != 0` = còn kết nối, `:767`):
```c
case 0x39:
  break;                       // :1020–1021 — KHÔNG build gì, KHÔNG gửi
```
⇒ **Client hiện tại không hề gửi frame 0x39.** Mock server không cần chờ packet 0x39.

**b) Đường "định gửi" bị cắt**: `FUN_00553818` (`00553818.c:20–26`; asm `00553818.asm.txt`: `MOV CL,1; MOV DL,0x39; CALL 0x77f414`):
```c
FUN_0077f414(*gvar_007D9D30, 0x39 /*EDX*/, CL=1);  // → case 0x39: break → rơi vào rỗng
*(byte *)(**gvar_007DA5A0 + 0xa096) = 1;           // dirty/pending flag
```
Hai call-site: (1) chuỗi click nút form#1: `FUN_00547110` (`00547110.c:24`, móc bảng callback `0x547081`) → `FUN_005467D0` (`005467d0.c:24` `FUN_00553818(mgr, p2)`) → refresh `FUN_00545D0C` + `vmt+0x24`; (2) trong máy trạng thái form#1 `FUN_00548080` (ref `0x5485cc`, header `00553818.c:9–10`). ⇒ **Đáng lẽ client GUI từng "gửi 0x39 sub 1" (CL=1) khi người dùng tác động form id1, nhưng builder rỗng khiến nó thành dead-request + flag.** Không loại trừ khả năng bản build khác/server game có đọc 0x39 — riêng **binary này: không gửi**.

---

## 7. Chuỗi literal & encoding

- **`0x00799200` và `0x00799224`: ĐÃ DUMP + DECODE (mới 2026-09-14)** — `lit_799200.hex`: content `0x799200` len 25 = **"Hiện giờ cấm công cụ này"**; content `0x799224` len 36 = **"người chơi nàyHiện giờ cấm công cụ này"** (văn nguyên game — hai chuỗi ghép liền không dấu cách, đúng như byte). Pipeline decode: VISCII (đính chính `opcode_09.md §7.1`).
- Khu vực này là **block chuỗi của họ banner**: OP 0x37 cũng gọi `+0x90` với `&UNK_007991d8`, `&UNK_007991ec` (2000ms) (`0078a89c.c:6840,6844`); khoảng cách 0x20–0x24/entry. **Đề nghị redump**: từ `0x007991C0` đến hết `0x00799248` (cắt tới null-terminator từng entry) rồi giải bằng **VISCII** (`iconv -f VISCII` — recipe đã chốt bằng chứng byte, `opcode_09.md §7.1`; cp1258 đã bác bỏ). KHÔNG bịa nội dung. Tham chiếu cùng run `lit_799200.hex`/`lit_799254.hex` đã decode cho thấy cả dải `0x799200+` thuộc block banner Xổ số/cấm công cụ.
- Chuỗi ASCII thuần đã đọc: `"Sound\\WA0045.wav"` (`00541c6c.c:187`), `"panel10"/"panel13"` (`0051189c.c:1275`).
- Bảng tên debug opcode trong dispatcher (`:583–759`) **không có case 0x39** (dừng ở `0x38 → 0x796d10`, default `0x796d38` `:592`) → log trace 0x39 in tên mặc định; các địa chỉ tên `0x796xxx` cũng chưa dump.

---

## 8. Ghi chú cho Mock Server

Payload dưới đây là **đã giải mã** (trước XOR). Frame thật trên dây: **XOR 0xAD TOÀN BỘ cả header lẫn payload, kể cả token** — bằng chứng: nhận XOR trước rồi mới so token (`0050cd6c_TForm1.ClientSocket1Read.c:67,75–76` với `redump/token_recv.hex` = `#2'F4 44'` ở không gian decode), gửi build xong XOR cả chuỗi rồi `SendText` (`005163e4_TForm1.CY_DelSedQueue.asm.txt:53–55`: `MOV CL,0xAD; CALL 0050a2fc`). Tức trên dây 2 byte đầu là `59 E9` (= `F4^AD 44^AD`).

| Mục đích | Bytes payload (decode) | Ghi chú |
| :--- | :--- | :--- |
| Mở form id1 | `39 01 01` | id hợp lệ theo bảng §5.2: 01/02/03/04/06/FF |
| Mở form id3, mode 1 | `39 01 03 01` | form3`+0x38` = 100 |
| Mở form id3, mode 2 | `39 01 03 02` | form3`+0x38` = 1000; **thiếu byte cuối → BoundErr** |
| Đóng form đang mở | `39 02` | nếu `ActiveId==0`: no-op an toàn |
| Banner 1 (1.2s) | `39 03 01` | chuỗi cục bộ `0x799200` — client tự hiển thị |
| Banner 2 (1.2s) | `39 03 02` | `0x799224` |

Ví dụ full frame `39 01 03 02`: decode `F4 44 | 04 00 | 39 01 03 02` → trên dây (XOR AD hết): `59 E9 A9 AD 94 AC AE AF`.

1. **Sequencing**: đẩy SubOp 1 trước để form mở, rồi `0x3C...` (case_053) bơm nội dung vào form id3; SubOp 2 đóng khi xong. Nếu mở form id3 mà chưa gửi dữ liệu `0x3C`, client vẫn chạy tick với trạng thái `+0x38` gần nhất.
2. Không gửi SubOp khác 1/2/3 (an toàn nhưng vô nghĩa); payload rỗng (`39` đơn độc) → **BoundErr** — hạn chế.
3. Client **không bao giờ gửi 0x39** (§6a) — không cần xử lý ở server; nếu mô phỏng server game "thật", lưu ý có yêu cầu `0x39 sub 1` bị missing do case rỗng.
4. Banner SubOp 3 chỉ hiệu lực khi widget TalkMsg còn sống (từ FormCreate đến `00603F20`); `node+0x14 busy` → banner bị drop im lặng (`007afbf8.asm.txt`).

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng trực tiếp:**
1. `case_functions/functions/case_050_00795579_FUN_00795579.c:8–105` — toàn bộ handler (+ SEH tail `:78–105` = cleanup chung).
2. `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` — tính python: `byte[0x39]=50`, entry `0x0078AA7E`, target `0x00795579`; sanity `0x36→47→0x00795494` khớp `opcode_36.md`.
3. `functions/0078a89c_FUN_0078a89c.c:6857–6919` (inline case, đối chiếu kép; `:6912,6916` vs standalone `:72,75`), `:583–759` (bảng tên), `:6840–6844` (banner OP 0x37 cùng block literal), `:6920–6984` (case 0x3A/0x3C liên quan form id1/id3).
4. `functions/00553410_FUN_00553410.c:19–55` — đóng form; `functions/00553840_FUN_00553840.c:23–52`, `0055391c_FUN_0055391c.c:23–52` — pump tick; `00603f20_FUN_00603f20.c:7,100–150` — out-world free.
5. `functions/0050a4a0_TForm1.FormCreate.c:629–630` + `005534e4_TSportManage.Create.c` — danh tính `TSportManage` (VMT_5532F4); `00553528.c:18–29` — double-click close.
6. `functions/0051189c_FUN_0051189c.c:1156,1263–1277` — `TSe_TalkMsgFormPlus`/`panel10`/widget-manager; quét DATA-ref toàn `functions/` xác lập slot VMT: `+0x7C`=0x7AFA94 (~190 class), `[0x63b42c]=FUN_007BADB0` (`007badb0.c:26`), `[0x63b478]=FUN_0063C84C` (`0063c84c.c:20`); `007badb0.asm.txt:4–25`, `007afbf8.asm.txt` (cắt cụt), `007afef8.c:66–76`, `007bc1c8.c:24–77`.
7. `functions/00541c6c_FUN_00541c6c.c:45–205`, `00541f8c.c:64–189` — tick form id3; `case_053_0079575C.c:24–38` — OP 0x3C bơm payload vào form id3.
8. `functions/0077f414_FUN_0077F414.c:767–768,1020–1021` — case 0x39 rỗng; `functions/00553818.c` + `.asm.txt` — dead-send + flag `0xa096`; `005467d0.c`, `00547110.c` — chuỗi click; `00548080.c:80–215` — máy trạng thái form#1.
9. `index.csv` (python): ~~HOLE `0x0055355D–0x00553818` (chứa `func_0x0055374c`), method form id3 `0x0053F/0x00540/0x00541`~~ → **2026-09-14: hai HOLE này ĐÃ decompile** (`0055374c_FUN_0055374c.c` + `00541098/005408f4/0053fd78/0053f8fc_FUN_*.c`); vẫn còn: caller tick `0x515C15/0x515D8E` (HOLE `0x00514C62–0x00516108`); `redump/` không phủ `0x799xxx`; `00516158.c:86–93` — RP pipeline (tiền lệ); `0050cd6c.c:67` + `005163e4.asm.txt:53–55` + `token_recv/send.hex` — XOR phủ toàn frame.
10. **(mới 2026-09-14)** `functions/0055374c_FUN_0055374c.c:23-59` — `OpenForm(id,flag)` + bảng class 6 form + writer `mgr+4`; `functions/00541098_FUN_00541098.c:94-244` — cross-check method form id3 (OP 0x3C SubOp 1, không đụng `+0x38`); grep re-export writers `gvar_007DA778/007D9D30/007DA5A0` (§5.3, §5.4).

**UNKNOWN (còn lại — cập nhật 2026-09-14):**
- ~~Thân `func_0x0055374c` = `OpenForm`: nhánh id 1/2/4/6/0xFF, hành xử vs id trùng/lặp, nghi thức đặt `mgr+4`~~ → **ĐÃ XÁC MINH §4.1** (6 nhánh id, writer `mgr+4`, KHÔNG guard form đang mở → mở chồng id khác khi chưa đóng sẽ lọt form cũ).
- 2 banner `0x799200/0x799224` đã redump + decode VISCII xong (§7). Còn treo: `0x7991d8/0x7991ec` (OP 0x37) chưa nằm trong window dump; khi có bytes decode bằng **VISCII** (cp1258 đã bị bác bỏ — `opcode_09.md §7.1`).
- Consumer của form3`+0x38` (100/1000): **vẫn không thấy** — đã quét cả 4 body method id3 mới decompile, 0 hit `+ 0x38` (§5.3).
- Đuôi `0x7AFC1F–0x7AFC24` của `FUN_007AFBF8` (enqueue/format slot +0x90) — **asm dump vẫn cắt cụt sau re-export 2026-09-14** (§4.3).
- Khai sinh `gvar_007D9D30/007DA5A0` (writer slot vẫn không có trong toàn bộ re-export — giữ là suy đoán); ~~`gvar_007DA778`~~ **đã xác minh: `FUN_0055374c.c:41-44` (TRE_ZMChessMain_Create)**; ý nghĩa flag `**(gvar_007DA5A0)+0xa096`; giá trị `CL=1` trong dead-send (sub-op ?) — không kết luận.
