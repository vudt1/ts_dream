# PHÂN TÍCH — Main OP 0x3B (Case 52) — `FUN_007956f3` @ `0x007956F3` — **handler passthrough — body helper chưa export** (Server → Client)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x3B] = 0x34 (=52)` → `jumptable_dword200_0x78A9B6[52]` (entry `0x0078AA86`) → target `0x007956F3` = **Case 52** (đọc file hex bằng python, LE). Đối chiếu kép: CSV `redump/jumptable_0x78A9B6_case_functions.csv:54` (`52,0x0078AA86,0x007956F3,YES,FUN_007956f3`) và inline trong dispatcher `0078a89c_FUN_0078a89c.c:6935–6958` (marker `UNK_0079572f/743/757`) khớp `case_052`.

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

- **CẢ 3 helper nằm trong HOLE `0x00552269–0x005524A0`** (đã verify bằng `index.csv`: hàm liền trước `FUN_00552238` kết thúc tại `0x552238+49=0x552269`, hàm liền sau là `FUN_005524a0`) ⇒ **wire body sau SubOp KHÔNG khôi phục được từ SSOT** (§4).
- **Chiều C→S KHÔI PHỤC ĐƯỢC 100% format** (hiếm, do hàm gửi `FUN_005524a0` nằm ngay mép HOLE và được export): gói gửi lên = **6 byte `3B 01 [DWORD LE = obj+0x28]`** — byte 2 là **hằng `0x01`** (asm sender: `MOV CL,0x1`), gửi giá trị điểm của manager mỗi lần "dirty" (§6).
- Họ manager: `gvar_007DA0F4` là **thành viên tag-2** trong bảng dispatch của `TSportManage` (`FUN_00553410.c:24–52`: tag1→`gvar_007DA42C` = object OP 0x3A/`FUN_00547c84` theo `case_051…c:28`, tag2→`gvar_007DA0F4` (file này), tag3→`gvar_007DA778`, tag4→`gvar_007D9F98`, tag6→`gvar_007DA4EC`, 0xff→`gvar_007DA0A0`). **Xác nhận "cùng một họ UI manager" như giả định đầu bài** — OP 0x39/0x3A/0x3B là các opcode cập nhật từng panel theo mode (§5).
- Bản chất panel mode-2 (suy ra từ các method cùng class đã export): **bảng số chạy / điểm** — field `+0x28` là số lớn, được phân khoảng `100..999 / 1000..3000 / 3001..5000 / 5001..9999 / ≥10000` để chọn sprite ID hiển thị, 3 dòng `TLight` đếm số với **jitter Random(±step)** ⇒ nhiều khả năng là bảng điểm/odds chạy trực tiếp (tên `TSportManage` gợi ý cá cược thể thao — **chưa chốt, UNKNOWN**).

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
| RP`[1..]` | **UNKNOWN — body nằm trong helper HOLE** | không khôi phục được từ SSOT (§4) |

Không có field nào sau SubOp đọc được ở mức dispatcher.

---

## 4. Điều tra helper (việc chính — kết luận HOLE)

Method điều tra theo quy tắc: (a) parse `index.csv` bằng python — mọi entry được export; (b) grep toàn bộ `functions/` + `case_functions/` + `*.asm.txt` với các mẫu `0055226c|005523d0|00552420|005522|005523|005524`.

| Helper | File riêng trong SSOT? | Cover theo index.csv? | Call-site khác (ngoài dispatcher) | Kết luận |
| :--- | :--- | :--- | :--- | :--- |
| `0x0055226C` (SubOp 2) | **KHÔNG** | HOLE `0x552269–0x5524A0` (kề `FUN_00552238` ends `0x552269`) | **0** — chỉ xuất hiện ở `case_052…c:32`, `jumptable_0x78A9B6_cases.c:8758`, `0078a89c.c:6952` (3 bản copy cùng 1 chỗ) | **HOLE — body + wire UNKNOWN** |
| `0x005523D0` (SubOp 1) | **KHÔNG** | cùng HOLE | **0** — `case_052…c:29`, `…_cases.c:8755`, `0078a89c.c:6948` | **HOLE — body + wire UNKNOWN** |
| `0x00552420` (SubOp 3) | **KHÔNG** | cùng HOLE | **0** — `case_052…c:35`, `…_cases.c:8761`, `0078a89c.c:6956` | **HOLE — body + wire UNKNOWN** |

Không có khối inline nào trong 6302 hàm export chứa fragment của 3 helper (grep `func_0x005522/0x005523/0x005524` chỉ trả về đúng 9 dòng dispatcher đã liệt kê). **Không có call-site thứ hai nào tường minh hơn ⇒ không suy được signature param_2 beyond `(RP)`**.

**Thứ khôi phục được quanh HOLE** (các hàm ngay sát mép, cùng đơn vị — xem §5): `FUN_005524a0` (0x5524A0) là method **gửi C→S 0x3B của chính class này**; `FUN_005524c4` là render-tick; `FUN_00552238` reset field; vì vậy 3 helper HOLE rất nhiều khả năng là **setter ghi vào các field mà render-tick đọc (`+0x28/+0x2c/+0x44[]/+0xa0`…)** — đây là **suy đoán loại suy, KHÔNG phải đặc tả**; body thật cần redump HOLE `0x00552269–0x005524A0`.

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
| `+0x20`,`+0x24` | byte trạng thái 0/1/2 + dword (reset tại `+0x30=1`) | `00552238.c:21–25`; dùng hiệu ứng `005527d4.c:23–31` |
| **`+0x28`** | **Số chính — thứ mà gói C→S 0x3B mang lên** | `005524f8.c:34`; `0077f414.c:1038` |
| `+0x2C` | Mirror từ game-state singleton: `obj+0x2c = gvar_007DA7BC^+0x12F8` (cùng singleton ghi `+0x640..+0x64e` ở mọi khối build packet) | `00552e58.c:21`; `0077f414.c:774–778` |
| `+0x34[1..3]` | 3 con trỏ **`TLight`** (class `VMT_772E1C_TLight`) tạo khi thiếu | `00552008.c:64–71` |
| `+0x44[1..4]` | Counter bước animation từng dòng | `005528c0.c:70,151,405–410`; reset `00551ca4.c:32` |
| `+0x48[5×idx]`,`+0x80`,`+0x85` | Bảng con theo `+0x85` (mode 1/2/…), `+0x80` ∈ 2..7 chọn layout, `>7` trigger `FUN_007b0094(DAT_00949228+0x138,1)` (sound/anim — graphics, 1 dòng) | `005526c0.c:46`; `005528c0.c:329–398` |
| `+0x70[1..4]` | ID 4 ô của dải | `005526c0.c:54` |
| `+0x84[1..3]`/`+0x90[1..3]` | Cặp giá trị gốc từng dòng (tween lower/upper) | `00552008.c:76,81`; `005528c0.c:76,82` |
| `+0xA0` | **Dirty flag** — method gửi 0x3B xóa ngay sau khi enqueue | `005524a0.c:22`; render chỉ vẽ dải khi `+0xA0≠0` (`005526c0.c:45`) |
| `+0xA8`,`+0xAC` | Double easing / counter (reset ở `00552238`) | `005527d4.c:24–28` |
| `+0xB0` | Byte **step** cho `FUN_005521b4 = value ± Random(step)` (50/50 theo `Random(100)<50`) | `005521b4.c:31–58` |
6. **Nơi GÁN/khởi tạo `gvar_007DA0F4`**: KHÔNG tồn tại trong SSOT — không có dòng `MOV [EDX],EAX` nào với `[0x007da0f4]` ở mọi asm export (grep `007DA0F4|007da0f4` toàn tree: chỉ 10 điểm đọc + 1 điểm zero `00553410.c:37`); constructor của class cũng không export (`FUN_00552008` là init/refresh, header `00552008.c:7–19` không có caller/reference nào → được gọi qua VMT/ trong HOLE). ⇒ **tên class và điểm tạo = UNKNOWN**.

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
- Helper HOLE: không có asm → không có literal để thử cp1258/NFC.
- Chuỗi thuộc class trong vùng export (dùng cho tính đầy đủ của hồ sơ, không thuộc payload): `&DAT_00552180` và `&DAT_005521A0` (2 toast string theo `obj+0x85` = 2/1, đẩy qua virtual `+0x90` của `gvar_007DA084` — `00552008.c:48–53`). Vùng `redump/` **không có `lit_*.hex` nào phủ `0x00552xxx`** (danh sách chỉ có `lit_59…`, `lit_77F7…`, `lit_78A8…`, `lit_7A2…`, `lit_7AB…`) ⇒ **chưa dump — cần redump, không bịa nội dung**.
- Tên debug opcode 0x3B (bảng tên quanh `0x00796CAC` như đã ghi ở opcode_36 §7): cũng chưa dump.

---

## 8. Ghi chú cho Mock Server

**Chắc chắn làm được:**
1. **Nhận C→S**: chấp nhận gói 6 byte `3B 01 [DWORD LE]` (byte 2 luôn `01`); byte 3..6 = điểm hiện tại của panel mode-2 (`obj+0x28`). Gửi lặp lại khi client thấy dirty ⇒ server nên idempotent theo giá trị.
2. **Gửi S→C**: phần TẤT YẾU đúng: `[0x3B][SubOp]` với `SubOp ∈ {1,2,3}`; SubOp khác ⇒ client bỏ qua vô hại; payload rỗng (`[0x3B]` đơn độc) ⇒ client raise `_BoundErr` nội bộ (SEH swallow — không crash nhưng mất gói).
3. Muốn im lặng tuyệt đối với client: **không gửi 0x3B** hoặc gửi SubOp lạ — hành vi đã kiểm chứng ở dispatcher.
4. OP này chỉ có nghĩa khi TSportManage đang ở mode 2 (`TSportManage+4 == 2`) — nếu chưa bật mode, manager có thể nil ⇒ helper HOLE có thể deref nil; **rủi ro không đo được vì body chưa export** — an toàn nhất là mock server KHÔNG emit 0x3B cho tới khi redump xong helper.

**Cảnh báo:** toàn bộ byte sau SubOp là **opaque** — mock không thể tạo gói nghiệp vụ đúng cho SubOp 1/2/3; chỉ passthrough/record khi test client thật.

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng trực tiếp:**
1. `case_functions/functions/case_052_007956F3_FUN_007956f3.c` (toàn bộ 66 dòng) — passthrough + silent drop.
2. `functions/0078a89c_FUN_0078a89c.c:6935–6958` — inline case 0x3B (xác nhận kép).
3. `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` — decode python: `0x3B→52→0x007956F3`; `redump/jumptable_0x78A9B6_case_functions.csv:54`.
4. `index.csv` (6302 entry, python range-cover) — HOLE `0x552269–0x5524A0` chứa cả 3 helper; HOLE `0x551365–0x55165C` (caller `0x551558`); HOLE `0x55355D–0x553840` (caller `0x553574`); các caller `0x515c15/0x515d8e/0x515f92` đều HOLE.
5. Grep toàn tree `0055226c|005523d0|00552420` (+ các prefix 005522/005523/005524) — 9 call-site duy nhất, tất cả là dispatcher.
6. Chuỗi định danh §5: `00553410.c:24–52`, `00553840.c`, `0055391c.c`, `005538cc.c`, `005533c8.c`, `00553528.c`, `0050c608_TForm1.DXDraw1Click.c:24`, `0050a4a0_TForm1.FormCreate.asm.txt:819–822`, `005534e4_TSportManage.Create.c`, `case_051…c:28` (đối chứng họ 0x3A), `005524a0.c`, `00553818.c`, `00548080.c:221`, `005467d0.c:23`.
7. Hồ sơ field class: `00551ca4.c`, `00552008.c`, `005521b4.c`, `00552238.c`, `005524c4.c` (+ 5 reference call trong header `005524c4.c:14–18`), `005524f8.c`, `005526c0.c`, `0055279c.c`, `005527c8.c`, `005527d4.c`, `005528c0.c`, `00552e58.c`.
8. C→S: `0077f414.c:1032–1041` + `0077f414.asm.txt:3660–3685` (header 2 byte từ `[EBP-0x5][EBP-0x6]`, 4 byte từ `[0x007da0f4]+0x28` qua `ee84`) + `0077ee84.c:68–126` (encoder DWORD→4 byte LE, param_1 dead).

**UNKNOWN (cần redump):**
- **Thân 3 helper** `0x0055226C / 0x005523D0 / 0x00552420` → **wire S→C sau SubOp** (HOLE `0x00552269–0x005524A0`).
- Caller `0x551558` của `FUN_005524a0` (HOLE `0x00551365–0x0055165C`) → **điều kiện dirty** khiến client gửi 0x3B (`+0x28` đổi khi nào) — byte sub C→S đã chốt `0x01` từ asm sender (§6), không còn là UNKNOWN.
- **Điểm gán tạo instance `gvar_007DA0F4` và tên class** (không có trong 6302 asm export; constructor không export).
- Nơi ghi byte mode `TSportManage+4` (HOLE `0x0055355D–0x00553840` hoặc ngoài tree).
- Nội dung toast `DAT_00552180/DAT_005521A0` (vùng `0x55xxxx` chưa lit-dump) và tên debug opcode 0x3B (vùng `0x796Cxx`).
- Ý nghĩa nghiệp vụ cuối cùng của panel ("điểm gì/odds gì") — mới chỉ dựng được cấu trúc hiển thị, chưa có chuỗi nhận dạng.
