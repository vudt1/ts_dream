# PHÂN TÍCH — Main OP 0x3B (Case 52) — `FUN_007956f3` @ `0x007956F3` — **handler passthrough 3 SubOp → `TSBDManager` (bàn Domino — mode 2 của `TSportManage`)** (Server → Client)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x3B] = 0x34 (=52)` → `jumptable_dword200_0x78A9B6[52]` (entry `0x0078AA86`) → target `0x007956F3` = **Case 52** (đọc file hex bằng python, LE). Đối chiếu kép: CSV `redump/jumptable_0x78A9B6_case_functions.csv:54` (`52,0x0078AA86,0x007956F3,YES,FUN_007956f3`) và inline trong dispatcher `0078a89c_FUN_0078a89c.c:6935–6958` (marker `UNK_0079572f/743/757`) khớp `case_052`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). **HOLE `0x00552269–0x005524A0` ĐÃ ĐƯỢC DECOMPILE** — cả 3 helper SubOp 1/2/3 có body (`005523d0.c`, `0055226c.c`, `00552420.c`) → wire S→C sau SubOp **không còn opaque** (§4). **Creator `gvar_007DA0F4` tìm thấy**: `0055374c_FUN_0055374c.c:33–35` tạo **`TSBDManager`** (VMT `VMT_550A2C_TSBDManager`) — định danh class mode-2 đã chốt (bổ trợ: `00551d1c_TSBDManager.Create.c` có sẵn trong SSOT, xác nhận sprite "ShiBaDo"/"GdMoney3..5"/"L10252"). Hai toast `DAT_00552180/DAT_005521A0` đã dump + giải mã VISCII: **"Người chơi bài Domino"** / **"Domino trên vi tính"** → panel mode-2 là **bàn Domino** (đính chính: không còn "chưa chốt UNKNOWN").

> Phạm vi: **core logic opcode**; phần render sprite/TLight chỉ nêu để chứng minh đối tượng tiêu thụ dữ liệu.

---

## 1. Tóm tắt nghiệp vụ

**OP 0x3B là opcode cập nhật "sub-manager mode 2" của họ TSportManage** — handler thuần passthrough 3 SubOp, truyền **nguyên con RP (gồm cả byte SubOp)** vào 3 helper:

| SubOp (`RP[0]`) | Helper nhận `(gvar_007DA0F4^, RP)` | Nguồn |
| :--- | :--- | :--- |
| 1 | `func_0x005523d0` | `case_052_007956F3_FUN_007956f3.c:28–29` |
| 2 | `func_0x0055226c` | `case_052…c:31–32` |
| 3 | `func_0x00552420` | `case_052…c:34–35` |
| khác | **silent drop** (không có `else`) | `case_052…c:28–36` |

- **CẢ 3 helper TRƯỚC ĐÂY ở HOLE `0x00552269–0x005524A0` — NAY ĐÃ CÓ BODY** (`005523d0_FUN_005523d0.c`, `0055226c_FUN_0055226c.c`, `00552420_FUN_00552420.c`, decompile 2026-09-14) ⇒ wire body sau SubOp **khôi phục được** (§4): SubOp 1 = đặt mode chờ `+0x84 ∈ {0,1}`; SubOp 2 = **cập nhật 3 giá trị xúc xắc 1..6 + 1 byte <8 vào bảng `+0x48[5·mode][1..4]`**; SubOp 3 = đặt byte trạng thái `+0x20 ∈ {0,1,2}` + reset hiệu ứng (§4.3).
- **Chiều C→S KHÔI PHỤC ĐƯỢC 100% format** (hiếm, do hàm gửi `FUN_005524a0` nằm ngay mép HOLE và được export): gói gửi lên = **6 byte `3B 01 [DWORD LE = obj+0x28]`** — byte 2 là **hằng `0x01`** (asm sender: `MOV CL,0x1`), gửi giá trị điểm của manager mỗi lần "dirty" (§6).
- Họ manager: `gvar_007DA0F4` là **thành viên tag-2** trong bảng dispatch của `TSportManage` (`FUN_00553410.c:24–52`: tag1→`gvar_007DA42C` = object OP 0x3A/`FUN_00547c84` theo `case_051…c:28`, tag2→`gvar_007DA0F4` (file này), tag3→`gvar_007DA778`, tag4→`gvar_007D9F98`, tag6→`gvar_007DA4EC`, 0xff→`gvar_007DA0A0`). **Xác nhận "cùng một họ UI manager" như giả định đầu bài** — OP 0x39/0x3A/0x3B là các opcode cập nhật từng panel theo mode (§5).
- Bản chất panel mode-2 (**ĐÃ XÁC MINH 2026-09-14 từ chuỗi toast dump + Create**): là **bàn Domino** — class **`TSBDManager`**; hai chế độ `+0x85`: **1 → banner "Domino trên vi tính"** (chơi với máy), **2 → banner "Người chơi bài Domino"** (`00552008.c:47–52`, text decode VISCII §7). `TSBDManager.Create` (`00551d1c_TSBDManager.Create.c`) load sprite-set **"ShiBaDo"** (index lưu `+0x04`), sprite điểm **"GdMoney3/4/5"** → `+0x10/+0x14/+0x18`, ảnh **"L10252"** → `+0x1C`, mặc định `+0x21:=2`, `+0x85:=2` (`:77–78`), `+0x24/+0x28 := 0` (`:82–84`). Field `+0x28` vẫn là số chính (điểm/tiền) được phân khoảng chọn sprite như mô tả cũ.

---

## 2. Entry & cách đọc payload

Framing/dispatcher: `[F4 44][Len:Word LE][Payload]` XOR `0xAD`; pipeline `TForm1.CY_DelRevQueue` (`00516158_TForm1.CY_DelRevQueue.c:86–93`) cắt `_LStrCopy(msg,2,Len-1)` → **RP = payload bỏ MainOp**, chuỗi 1-based: `RP[0]` = SubOp; rồi `FUN_0078a89c(RP, msg[0])`.

`FUN_007956f3` (`case_052…c:20–36`, ĐÃ ĐỌC TOÀN BỘ):
```c
iVar2 = RP;                                   // EBP-0xc
if (*(int*)(RP-4) == 0) _BoundErr(0);         // RP rỗng → ERangeError (client lỗi gói, không crash process — SEH dispatcher)
SubOp = RP[0];
case SubOp of
  1: func_0x005523d0( gvar_007DA0F4^ , RP );  // CHÚ Ý: truyền RP GỐC — helper tự đọc lại RP[0]=SubOp
  2: func_0x0055226c( gvar_007DA0F4^ , RP );
  3: func_0x00552420( gvar_007DA0F4^ , RP );
else { }                                      // drop im lặng
```
Khác nhiều opcode khác (helper chỉ nhận phần sau SubOp), ở đây **RP vào helper vẫn đầy đủ từ byte SubOp** — dấu hiệu các helper là method kiểu `DoSub(RP: string)` tự rẽ nhánh.

Vòng đệm SEH cuối handler (`LAB_00796408`, `case_052…c:37–64`) là cleanup frame dùng chung dispatcher — không phải logic.

---

## 3. Wire layout (S→C)

| Offset payload | Nội dung | Ràng buộc client |
| :--- | :--- | :--- |
| `[0]` | `0x3B` (trước khi cắt vào RP) | — |
| RP`[0]` | `SubOp ∈ {1,2,3}` | khác → silent drop; RP rỗng → `_BoundErr` |
| RP`[1..]` | **ĐÃ KHÔI PHỤC (§4)**: SubOp 1 = 1 byte `∈{0,1}`; SubOp 2 = 3 byte xúc xắc `∈{1..6}` + 1 byte `<8` (tối thiểu RP[1..4]); SubOp 3 = 1 byte `∈{0,1,2}` | body helper mới decompile §4.1–4.3 |

Không có field nào sau SubOp đọc được ở mức dispatcher.

---

## 4. Phân tích helper (HOLE ĐÃ LẤP — decompile 2026-09-14)

Cả 3 helper giờ có file riêng: `functions/005523d0_FUN_005523d0.c` (78B), `functions/0055226c_FUN_0055226c.c` (344B), `functions/00552420_FUN_00552420.c` (125B) + `.asm.txt` — header Ghidra xác nhận caller duy nhất là các nhánh dispatcher (`sub_0079572a/0079573e/00795752` = address trong `case_052`). **Mọi helper nhận `(Self, RP nguyên)` và tự đọc `RP[0]`=SubOp để tự kiểm** — đúng như suy đoán cũ "method kiểu DoSub(RP)".

### 4.1. `FUN_005523d0` — SubOp 1: đặt **mode chờ** `+0x84`
- `005523d0.c:29` — yêu cầu `len(RP) ≥ 2` (BoundErr). Đọc byte `RP[1]` (byte ngay sau SubOp); asm `005523d0.asm.txt:12–27`: `SUB AL,0x2; JNC <return>` ⇒ chỉ nhận giá trị **0 hoặc 1**; khi đó `obj+0x84 := RP[1]` (`.c:44`). Giá trị ≥2 → im lặng không ghi.
- Ý nghĩa (xác minh chéo §4.2 + `FUN_00551fb8`): `+0x84` = **mode YÊU CẦU (pending)**, `+0x85` = mode ĐANG DÙNG (active). Create khởi tạo `+0x85 := 2` (`00551d1c_TSBDManager.Create.c:78`).

### 4.2. `FUN_0055226c` — SubOp 2: **nạp 3 con xúc xắc (1..6) + 1 byte phụ vào bảng `+0x48[5·mode][·]`**
- Mở đầu gọi `FUN_00551fb8(obj)` (`0055226c.c:42`) — helper đổi mode: nếu `+0x85(old)==2` và `+0x84==1` thì **set dirty `+0xA0 := 1`** (`00551fb8.c:21–22` — tức bảng điểm sẽ được gửi C→S 0x3B), play sound `FUN_007b0094(DAT_00949228+0x138, 0)`, rồi `+0x85 := +0x84` (`00551fb8.c:25`).
- Vòng `i = 1..3` (`0055226c.c:44–95`): đọc `RP[i]` (0-based — 3 byte sau SubOp; cần `len(RP) ≥ 4`); **ràng buộc từng byte `RP[i] ∈ {1..6}`** (`:60`: `(byte)(v-1) > 5` → `return 0` — abort toàn bộ, các byte đã ghi ở vòng trước vẫn còn); ghi `obj + 0x48 + ((+0x85)*5 + i)*4 := RP[i]` (`:74–92`, bounds `+0x85 ≤ 2`, index `i ≤ 4`).
- Byte cuối: `RP[4]` (0-based; cần `len(RP) ≥ 5`, `:98–103`); **điều kiện `RP[4] < 8`** (`:104–105`); ghi `obj + 0x58 + (+0x85)*20 := RP[4]` — tức **slot [4] của cùng bảng `+0x48`** (`:125`). `RP[4] ≥ 8` → bỏ qua đoạn sau (không refresh).
- Đuôi: `+0xA1 := (+0x80 == 7)` (`:126–131`); gọi `FUN_00552008(obj)` refresh/redraw (`:133`).
- **Kết luận**: bảng `+0x48[5·(+0x85)+1..4]` chính là **3 ô xúc xắc + 1 giá trị phụ (<8)** của hàng theo mode đang dùng — "suy đoán setter" cũ Nay **xác minh được từ body mới**.

### 4.3. `FUN_00552420` — SubOp 3: đặt **byte trạng thái `+0x20`** + reset hiệu ứng + sound
- `00552420.c:28` yêu cầu `len(RP) ≥ 2`; asm `00552420.asm.txt:12–27`: `SUB AL,0x3; JNC <return>` ⇒ chỉ nhận `RP[1] ∈ {0,1,2}`.
- `obj+0x20 := RP[1]` (`.c:41`); gọi `FUN_00552e78(obj)` (`.c:42` — **function này vẫn chưa có trong `index.csv`/SSOT** — check python: không hàm nào phủ `0x552e78`; nhiều khả năng stop-animation, chưa kết luận được); `obj+0x30 := 0`, `obj+0xA8 := 0`, `obj+0xAC := 0` (reset cặp double easing — đúng bộ field `00552238` từng reset, `.c:43–45`); play sound `FUN_007b0094(DAT_00949228+0x138, 1)` (`.c:46`).

### 4.4. Bảng cập nhật

| Helper | File | Tóm tắt hành vi (từ body) |
| :--- | :--- | :--- |
| `0x0055226C` (SubOp 2) | `0055226c_FUN_0055226c.c` (344B) | Đổi mode + set dirty `+0xA0`; ghi 3 byte xúc xắc 1..6 → `+0x48[5·+0x85][1..3]`, 1 byte <8 → slot [4]; refresh `FUN_00552008` |
| `0x005523D0` (SubOp 1) | `005523d0_FUN_005523d0.c` (78B) | `+0x84 := RP[1]` khi `RP[1] ∈ {0,1}` (đặt mode chờ) |
| `0x00552420` (SubOp 3) | `00552420_FUN_00552420.c` (125B) | `+0x20 := RP[1]` khi `RP[1] ∈ {0,1,2}`; reset `+0x30/+0xA8/+0xAC`; sound; gọi `FUN_00552e78` (vẫn thiếu body) |

Không còn call-site thứ hai nào ngoài dispatcher. **Wire S→C tối thiểu**: SubOp 2 cần payload `[3B][02][d1][d2][d3][x]` = 6 byte; SubOp 1/3 cần `[3B][01|03][v]` = 3 byte.

---

## 5. Global `gvar_007DA0F4` — định danh từng bước (bằng chứng file:line)

Chuỗi suy luận (KHÔNG có tài liệu `.md` nào trước đây định danh global này — đã grep `.scratch/op-code/*.md`: 0 kết quả; toàn bộ dưới đây tự xác minh từ `ts_decompile/`):

1. **Là con trỏ instance Delphi (double-pointer)**: mọi chỗ dùng đều `*(int*)gvar_007DA0F4` rồi mới deref tiếp (vd `case_052…c:29`; `0077f414.asm.txt:3674–3676`: `MOV EAX,[0x007da0f4]; MOV EAX,[EAX]; MOV EDX,[EAX+0x28]`).
2. **Thuộc bảng manager theo mode của `TSportManage`** — 6 dispatcher cùng khuôn `case byte @ (TSportManage+4)` trên 6 global:
   - `FUN_00553410.c:24–52` — **Free manager theo tag**: tag2 → `TObject_Free(gvar_007DA0F4^); gvar_007DA0F4^ = 0` (`:36–37`). Callers: `FUN_00603f20` (reset out-world — đã ghi nhận ở opcode_36 §5.2) + `0x553574` (HOLE `0x55355D–0x553840`).
   - `FUN_00553840.c:37–38` — render: tag2 → `FUN_005524c4(gvar_007DA0F4^)`; caller `0x515d8e` (HOLE `0x515xxx`).
   - `FUN_0055391c.c:37–38` — sync-from-game: tag2 → `FUN_00552e58(gvar_007DA0F4^)`; caller `0x515c15` (HOLE).
   - `FUN_005538cc.c:27–28` — tag2 → `FUN_005527c8` (**stub rỗng** `005527c8.c:18–22`); caller `0x515f92` (HOLE).
   - `FUN_005533c8.c:24–25` — click: tag2 → gọi **VMT slot 0** của manager; `FUN_00553528.c:21–22` — dblclick: tag2 → **VMT +4** (theo quy ước VMT Delphi: slot 0 = `Destroy`, +4 = `DefaultHandler` — cách hiểu quy ước, không phải tên đã export).
3. **Chứng minh `param_1` của các dispatcher trên chính là TSportManage**: `TForm1.DXDraw1Click.c:24` gọi `FUN_005533c8(*(int*)gvar_007D9D88)`; `TForm1.DXDraw1DblClick` (caller header `00553528.c:8`). Và `gvar_007D9D88 = TSportManage instance`: `0050a4a0_TForm1.FormCreate.asm.txt:819–822` — `MOV EAX,[0x005532f4]` (class cell) → `CALL 0x005534e4 (TSportManage.Create)` → `MOV EDX,[0x007d9d88]; MOV [EDX],EAX`. Constructor: `005534e4_TSportManage.Create.c:20–42`.
4. **Họ OP 0x3A cùng khuôn**: `case_051_007956B9_FUN_007956b9.c:27–28` — SubOp 1 → `FUN_00547c84(gvar_007DA42C^, RP)` (tag1 của cùng bảng `FUN_00553410`) ⇒ OP 0x39/0x3A/0x3B = 3 opcode "cập nhật panel" song song cho các mode.
5. **Class của manager mode-2** (đơn vị code `0x00551CA4–0x00552E58`, mọi hàm nhận obj làm tham số 1 và đụng cùng bộ field — đủ đồng nhất để quy về MỘT class):

| Offset | Nội dung (từ hàm export) | Bằng chứng |
| :--- | :--- | :--- |
| `+4` | Image index nền (`-1` = ẩn) — `FUN_007cadf8(ImageManager gvar_007D9ED8, obj+4,…)` | `0055279c.c:21–22` |
| `+0x0C[1..3]`,`+0x10`,`+0x14`,`+0x18` | Sprite ID chọn theo **khoảng giá trị `+0x28`**: 100..999→`+0x10`; 1000..3000→`+0x14`; 3001..5000→`+0x18`; 5001..9999→3×`+0x14`; ==0/≥10000→`+0x0C[k]` (kích thước 200..230px qua `FUN_007cb380`) | `005524f8.c:34–123` |
| `+0x1C` | Image set cho dải 4 ô (pitch 59px, y=30·k+30) | `005526c0.c:45,54–87` |
| `+0x20`,`+0x24` | byte trạng thái 0/1/2 + dword (reset tại `+0x30=1`) | `00552238.c:21–25`; dùng hiệu ứng `005527d4.c:23–31`; **`+0x20` ghi từ gói S→C SubOp 3 — xác minh `00552420.c:41`** |
| **`+0x28`** | **Số chính — thứ mà gói C→S 0x3B mang lên**; khởi tạo `=0` trong `TSBDManager.Create` (`00551d1c.c:83`) | `005524f8.c:34`; `0077f414.c:1038`; `00551d1c.c:83` |
| `+0x2C` | Mirror từ game-state singleton: `obj+0x2c = gvar_007DA7BC^+0x12F8` (cùng singleton ghi `+0x640..+0x64e` ở mọi khối build packet) | `00552e58.c:21`; `0077f414.c:774–778` |
| `+0x34[1..3]` | 3 con trỏ **`TLight`** (class `VMT_772E1C_TLight`) tạo khi thiếu | `00552008.c:64–71` |
| `+0x44[1..4]` | Counter bước animation từng dòng | `005528c0.c:70,151,405–410`; reset `00551ca4.c:32` |
| `+0x48[5×idx]`,`+0x80`,`+0x85` | Bảng con theo `+0x85` (**= mode đang dùng 0..2, bounds `≤2` verify tại `0055226c.c:76`**), `+0x80` ∈ 2..7 chọn layout, `>7` trigger `FUN_007b0094(DAT_00949228+0x138,1)` (sound/anim — graphics, 1 dòng); **ô `[1..3]` = 3 xúc xắc, `[4]` = byte phụ <8, ghi từ SubOp 2 — verify §4.2** | `005526c0.c:46`; `005528c0.c:329–398`; `0055226c.c:92,125` |
| `+0x70[1..4]` | ID 4 ô của dải | `005526c0.c:54` |
| `+0x84` (byte), `+0x88..+0x9C` (dword) | **byte `+0x84` = mode YÊU CẦU (pending), ghi bởi SubOp 1 (`005523d0.c:44`), đọc/copy sang `+0x85` bởi `FUN_00551fb8.c:21–25`**; các dword `+0x88..` = cặp giá trị gốc từng dòng (tween lower/upper), khởi tạo tọa độ trong `TSBDManager.Create` (`00551d1c.c:106–111`) | `00552008.c:76,81`; `005528c0.c:76,82`; `00551d1c.c:106–111` |
| `+0xA0` | **Dirty flag** — method gửi 0x3B xóa ngay sau khi enqueue; **nơi SET đầu tiên tìm thấy: `FUN_00551fb8.c:22` (`+0xA0:=1` khi chuyển mode 2→1 lúc nhận SubOp 2)** | `005524a0.c:22`; `00551fb8.c:21–22`; render chỉ vẽ dải khi `+0xA0≠0` (`005526c0.c:45`) |
| `+0xA8`,`+0xAC` | Double easing / counter (reset ở `00552238`) | `005527d4.c:24–28` |
| `+0xB0` | Byte **step** cho `FUN_005521b4 = value ± Random(step)` (50/50 theo `Random(100)<50`) | `005521b4.c:31–58` |
6. **Nơi GÁN/khởi tạo `gvar_007DA0F4` — ĐÃ TÌM THẤY (2026-09-14)**: `0055374c_FUN_0055374c.c:33–35` — trong dispatch theo mode của OP 0x39, `mode==2` gọi `TSBDManager_Create(VMT_550A2C_TSBDManager,'\x01')` rồi `*(undefined4*)gvar_007DA0F4 = uVar2`; cuối hàm ghi `*(byte*)(TSportManage+4) = mode` (`:58`, hết nghi vấn "nơi ghi byte mode"). Hàm được gọi từ `case_050_00795579_FUN_00795579.c:39` (OP 0x39). **Tên class chốt: `TSBDManager`** — constructor `00551d1c_TSBDManager.Create.c` (436B, có sẵn, đã đối chiếu field init §1). Free: `00553410.c:36–37`; render/sync: `00553840.c:38`, `0055391c.c:38`.

---

## 6. Chiều C→S — CÓ THẬT, khôi phục đủ format

`0077f414_FUN_0077f414.c:1032–1041` (switch `param_2 & 0xff` sau gate `*gvar_007DA3A0 ≠ 0`, `:767`):
```c
case 0x3b:
  FUN_00402b90(&b1, &tmp); _PStrNCat(&b1, &pre, 2);      // header 2 byte [DL][CL] = [0x3B][CL của sender]
  _LStrFromString(&pkt, &b1);                             //   (khuôn đã hiệu chuẩn opcode_37.md:331–334)
  FUN_0077ee84(param_1, *(uint*)(gvar_007DA0F4^ + 0x28), &d); // DWORD obj+0x28 → 4 byte LE (0077ee84.c:68–126; param_1 chỉ được lưu, không dùng — dead arg)
  _LStrCat(&pkt, d);
  TForm1_CY_AddSedQueue(gvar_007DA664, pkt);              // asm: 3683–3685 CALL 0x0051633c
```
Gói gửi: **`[3B][01][b0][b1][b2][b3]`** = 6 byte, trong đó byte 2 là **hằng `0x01`** và 4 byte sau = little-endian DWORD `gvar_007DA0F4^+0x28`.

Sender duy nhất trong tree export: **`FUN_005524a0`** — method của chính class mode-2. Ghidra C (`005524a0.c:20–24`) ghi `CONCAT31((int3)(param_2>>8), 0x3b)` — **đây là artifact của decompiler**; asm mới là quyết định (`005524a0_FUN_005524a0.asm.txt`):
```asm
MOV CL,0x1      ; sub byte = HẰNG 1, gán ngay trong sender
MOV DL,0x3b     ; op
CALL 0x0077f414
MOV EAX,[EBP-0x4]        ; obj
MOV byte ptr [EAX+0xa0],0x0   ; clear dirty flag
```
⇒ ngữ nghĩa chắc chắn: *"gửi điểm hiện tại `+0x28` lên server với sub byte **cố định = 1**, rồi xóa cờ dirty `+0xA0`"*; caller của `FUN_005524a0` tại `0x551558` (HOLE `0x00551365–0x0055165C`) chỉ quyết định **khi nào** gửi (điều kiện dirty), không quyết định nội dung — byte sub C→S **đã chốt = 0x01**, tình cờ trùng tập {1,2,3} chiều S→C. Đối chứng họ: `FUN_00553818` (asm: `MOV CL,1; MOV DL,0x39`) cùng khuôn nhưng builder case 0x39 rỗng nên không frame nào được phát — và `FUN_00547fbc` của OP 0x3A (`MOV CL,1; MOV DL,0x3a`) thì builder 0x3A **bỏ quên CL**, dùng `[obj+4]` thay vào (xem `opcode_3a.md §6`).

---

## 7. Chuỗi literal & encoding

- Handler `FUN_007956f3`: **0 literal** (chỉ marker SEH `UNK_/LAB_`).
- Helper (đã có body): **ZERO literal nội tại** — chỉ có hằng/số chỉ mục; không có chuỗi để decode. (Hai toast `0x552180/0x5521A0` nằm ngoài helper, xem dòng dưới.)
- Chuỗi thuộc class: `&DAT_00552180` và `&DAT_005521A0` (2 toast theo `obj+0x85`: **1 → `DAT_005521A0`, 2 → `DAT_00552180`** — `00552008.c:47–52`, đẩy qua virtual `+0x90` của `gvar_007DA084`). **ĐÃ DUMP + GIẢI MÃ 2026-09-14** (`redump/lit_552180.hex`, `lit_5521a0.hex`, VISCII):
  - `DAT_00552180` = `4E 67 DF B6 69 20 63 68 BD 69 20 62 E0 69 20 44 6F 6D 69 6E 6F` + `00` (21 byte) → **"Người chơi bài Domino"** — **raw trong dump**: con trỏ chỉ thẳng vào content, không có header `[FF FF FF FF][len]` phía trong vùng dump (content bắt đầu tại chính `0x552180`; byte `00` kế tiếp + pad tới `0x552197`).
  - `DAT_005521A0` = `44 6F 6D 69 6E 6F 20 74 72 EA 6E 20 76 69 20 74 ED 6E 68` + `00` (19 byte) → **"Domino trên vi tính"** — **const AnsiString ĐẦY ĐỦ HEADER**: `FF FF FF FF | 13 00 00 00` tại `0x552198–0x55219F` (header này thấy được trong `lit_552180.hex`, ngay sau toast 1).
  - Ngay sau chuỗi thứ hai là code (prologue `55 8B EC…`) — vùng dữ liệu kết thúc tại `0x5521B3`.
- Tên debug opcode 0x3B (bảng tên quanh `0x00796CAC` như đã ghi ở opcode_36 §7): cũng chưa dump.

---

## 8. Ghi chú cho Mock Server

**Chắc chắn làm được:**
1. **Nhận C→S**: chấp nhận gói 6 byte `3B 01 [DWORD LE]` (byte 2 luôn `01`); byte 3..6 = điểm hiện tại của panel mode-2 (`obj+0x28`). Gửi lặp lại khi client thấy dirty ⇒ server nên idempotent theo giá trị.
2. **Gửi S→C**: phần TẤT YẾU đúng: `[0x3B][SubOp]` với `SubOp ∈ {1,2,3}`; SubOp khác ⇒ client bỏ qua vô hại; payload rỗng (`[0x3B]` đơn độc) ⇒ client raise `_BoundErr` nội bộ (SEH swallow — không crash nhưng mất gói).
3. Muốn im lặng tuyệt đối với client: **không gửi 0x3B** hoặc gửi SubOp lạ — hành vi đã kiểm chứng ở dispatcher.
4. OP này chỉ có nghĩa khi TSportManage đang ở mode 2 (`TSportManage+4 == 2`) — nếu chưa bật mode, `gvar_007DA0F4^` có thể nil ⇒ cả 3 helper **deref nil ngay** (`005523d0.c:44`, `0055226c.c:42`, `00552420.c:41` đều không kiểm nil) → AV client. Chỉ emit 0x3B sau khi client vào bàn Domino qua OP 0x39 (mode 2).
5. **Gói nghiệp vụ mẫu (từ body §4)**: `[3B][01][00|01]` đổi mode chờ; `[3B][02][d1 d2 d3][y]` với `d∈1..6`, `y<8` (6 byte) nạp xúc xắc Domino; `[3B][03][0|1|2]` đổi trạng thái + âm thanh.

**Ghi chú cũ (đã gỡ):** toàn bộ byte sau SubOp từng bị coi là opaque; **nay đã khôi phục shape + ràng buộc từ body helper** (§4). Ý nghĩa nghiệp vụ sâu của từng ô vẫn cần traffic thật đối chiếu.

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng trực tiếp:**
1. `case_functions/functions/case_052_007956F3_FUN_007956f3.c` (toàn bộ 66 dòng) — passthrough + silent drop.
2. `functions/0078a89c_FUN_0078a89c.c:6935–6958` — inline case 0x3B (xác nhận kép).
3. `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` — decode python: `0x3B→52→0x007956F3`; `redump/jumptable_0x78A9B6_case_functions.csv:54`.
4. `index.csv` (python range-cover, cập nhật 2026-09-14): **HOLE `0x552269–0x5524A0` ĐÃ LẤP** (3 helper có file + entry mới); HOLE `0x551365–0x55165C` (caller `0x551558` — **vẫn trống**); khe `0x55355D–0x55374C` (caller `0x553574` — **vẫn trống**; `0055374c` là creator mới decompile ngay sau khe); các caller `0x515c15/0x515d8e/0x515f92` vẫn HOLE.
5. `functions/0055374c_FUN_0055374c.c:25–58` + `case_functions/functions/case_050_00795579_FUN_00795579.c:39` — creator 6 manager theo mode (OP 0x39) + nơi ghi `TSportManage+4`.
5a. `functions/00551d1c_TSBDManager.Create.c:69–111` — khởi tạo field (`+4`="ShiBaDo", `+0x10..`= "GdMoney3..5", `+0x1C`="L10252", `+0x20` byte cao=2, `+0x85`=2, `+0x24/+0x28`=0, tọa độ `+0x88..+0x9C`).
5b. `functions/00551fb8_FUN_00551fb8.c:21–25` — pending→active mode + set dirty `+0xA0`.
5c. `redump/lit_552180.hex` / `lit_5521a0.hex` — decode VISCII (§7).
6. Grep toàn tree `0055226c|005523d0|00552420` (+ các prefix 005522/005523/005524) — call-site ngoài dispatcher: các header `0055226c.c:8`, `005523d0.c:8`, `00552420.c:8` xác nhận `sub_0079572a/3e/52` (chính case_052).
7. Chuỗi định danh §5: `00553410.c:24–52`, `00553840.c`, `0055391c.c`, `005538cc.c`, `005533c8.c`, `00553528.c`, `0050c608_TForm1.DXDraw1Click.c:24`, `0050a4a0_TForm1.FormCreate.asm.txt:819–822`, `005534e4_TSportManage.Create.c`, `case_051…c:28` (đối chứng họ 0x3A), `005524a0.c`, `00553818.c`, `00548080.c:221`, `005467d0.c:23`.
8. Hồ sơ field class: `00551ca4.c`, `00552008.c`, `005521b4.c`, `00552238.c`, `005524c4.c` (+ 5 reference call trong header `005524c4.c:14–18`), `005524f8.c`, `005526c0.c`, `0055279c.c`, `005527c8.c`, `005527d4.c`, `005528c0.c`, `00552e58.c`.
9. C→S: `0077f414.c:1032–1041` + `0077f414.asm.txt:3660–3685` (header 2 byte từ `[EBP-0x5][EBP-0x6]`, 4 byte từ `[0x007da0f4]+0x28` qua `ee84`) + `0077ee84.c:68–126` (encoder DWORD→4 byte LE, param_1 dead).

**ĐÃ BỔ SUNG 2026-09-14 (theo `missing_opcode_sources.md`):**
- ~~Thân 3 helper~~ **ĐÃ CÓ BODY**: `0055226c/005523d0/00552420` (HOLE `0x00552269–0x005524A0` đã decompile) → wire S→C sau SubOp khôi phục đầy đủ (§4).
- ~~Điểm gán tạo instance `gvar_007DA0F4` và tên class~~ **ĐÃ CHỐT**: `0055374c.c:33–35` — `TSBDManager_Create(VMT_550A2C_TSBDManager)`; constructor `00551d1c_TSBDManager.Create.c`.
- ~~Nội dung toast `DAT_00552180/DAT_005521A0`~~ **ĐÃ DUMP + DECODE** (§7): "Người chơi bài Domino" / "Domino trên vi tính".
- ~~Nơi ghi byte mode `TSportManage+4`~~ **ĐÃ TÌM THẤY**: `0055374c.c:58`.
- Bảng C→S đối chiếu: `redump/table_0x77F474_200B.hex` byte[0x3B] = `0x31` ≠ 0 và `table_0x77F53C_dword200.hex` entry 0x3B = `0x00789FDA` — builder `case 0x3b` **xác nhận tồn tại** đúng như §6.

**CÒN THIẾU (giới hạn hiện tại):**
- Caller `0x551558` của `FUN_005524a0` — **vẫn là HOLE** `0x00551365–0x0055165C` (kiểm `index.csv` bằng python 2026-09-14: không hàm nào phủ `0x551558`; hàm gần nhất `TSBDManager.Create` bắt đầu `0x551d1c`) → điều kiện chọn thời điểm gửi 0x3B (dirty) **vẫn chưa xác minh** — mặc dù **nơi SET dirty `+0xA0` đầu tiên đã tìm thấy** trong `FUN_00551fb8.c:22` (rõ ràng thuộc đường đi SubOp 2).
- Caller `0x553574` của `FUN_00553410` — **vẫn trong khe** `0x0055355D–0x0055374C` (`FUN_0055374c` chỉ bắt đầu tại `0x55374C`).
- `FUN_00552e78` (callee của SubOp 3, `00552420.c:42`) — **vẫn không có trong `index.csv`** (kiểm python: không hàm phủ `0x552e78`; `FUN_00552e58` size 29 kết thúc `0x552e75`). Nhiều khả năng stop-animation/refresh — **chưa kết luận được**.
- Tên debug opcode 0x3B (vùng `0x796Cxx`) — chưa dump.
- Hành vi nghiệp vụ cuối cùng của các ô `+0x48[..]` (xúc xắc Domino kiểu gì — 3 ô 1..6 + byte <8) — body chỉ chứng minh shape, chưa có chuỗi nhận dạng.
