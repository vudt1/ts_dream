# PHÂN TÍCH — Main OP 0x05 (Case 6, `FUN_0078ca56` @ `0x0078CA56`) — aLogin.exe

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều chính: **Server → Client (S→C)**
Trạng thái: **Đã xác minh 100% ở tầng handler** (file case + dispatcher inline + bảng jump redump). **8/10 nhánh sub-op ủy quyền toàn bộ payload cho helper CHƯA có body trong `ts_decompile`** — ghi rõ giới hạn từng nhánh, không suy diễn wire layout (theo tiền lệ `opcode_1a.md`).

---

## 0. Đính chính giả định nghiệp vụ

1. **Không có giả định cũ cần bác bỏ**: `handoff-opcode-exploration-guide.md` KHÔNG gán nghiệp vụ cho OP 0x05 (mục 3 ưu tiên chỉ liệt kê 0x02/0x08/0x14/0x17/0x1A/0xC7). → Đây là phân tích gốc đầu tiên.
2. **Đính chính hình dung "handler lớn"**: `FUN_0078ca56` là một **thuần dispatcher** (~60 dòng lệnh): đọc 1 byte SubOp, `switch`, mỗi nhánh **chuyển nguyên con RestPayload cho một procedure cấp phát sẵn** theo khuôn `helper(đối_tượng_toàn_cục, &RestPayload)`. Toàn bộ logic giải mã field nằm **bên trong helper**, không nằm trong handler.
3. **Giới hạn nguồn (quan trọng nhất)**: 7/9 helper đích — `func_0x00731534`, `func_0x00727164`, `func_0x0072e9a0`, `func_0x00732368`, `func_0x005763c8`, `func_0x007289b8`, `func_0x0074f9f8` — **không tồn tại trong `ts_decompile/index.csv`** (Ghidra không tạo được function record cho các entry này — thuộc các "gap" chưa decompile). Chỉ 2 helper có body: `FUN_0079b620` (SubOp 0x04, gọi thêm) và `FUN_0072f970` (SubOp 0x0A). Vì vậy wire layout **chỉ khẳng định được từng byte** cho SubOp 0x04 và 0x0A; các sub còn lại mô tả ở mức *đối tượng bị tác động + ngữ cảnh unit* kèm độ tin cậy.
4. **Cảnh báo tiền lệ C→S**: doc `opcode_14.md` §5.1 đã chứng minh `case N: break;` trong `0077f414.c` **có thể là artifact jump-table bị gộp** (OP 0x14 hiển thị `break` nhưng thực tế client vẫn gửi). Với OP 0x05, đúng tình huống này xảy ra: `.c` nói "rỗng" nhưng **có 2 call-site asm thật** gửi `DL=0x05` (xem mục 5).

---

## 1. Tóm tắt nghiệp vụ

**OP 0x05 = kênh "Đồng bộ trạng thái/sự kiện thế giới & nhân vật trong cảnh" (world/actor state-event sync), server-push.**

Vị trí trong dải OP đã biết: `0x03` (hồ sơ spawn — SELF/OTHER), `0x04` (hồ sơ spawn actor từ xa) tạo thực thể → **`0x05` cập nhật trạng thái/sự kiện cho các thực thể đã tồn tại** (self, cache `TWorldPlayer` 2100 slot, scene manager 800 actor, các form) → `0x06` (di chuyển actor từ xa) → `0x08` (stats).

Bằng chứng định hướng (xếp theo lực mạnh):

| Bằng chứng | Hệ quả |
| :--- | :--- |
| **SubOp 0x0A có body đầy đủ**: `FUN_0072f970` đọc charID, gọi `FUN_0071e004(actor, "LevelUP", 0)` và phát `sound\WA0013.wav` | Nhánh 0x0A = **thông báo LÊN CẤP** cho actor (chuỗi literal `"LevelUP"` nằm ngay trong decompile — không cần đoán). |
| **SubOp 0x04 inline**: set `scene+0x53fc = 1` (cờ gate được `KeyboardWalk`/`DXDraw1MouseDown`/game-tick kiểm tra trước khi cho đi bộ/click/tick actor), stamp `self+0x348 = GetTickCount()`, `FUN_0079b620(TMouseInfo, 0)` xóa `+0x24`/`+0x2c` | Nhánh 0x04 = **"thế giới sẵn sàng / mở quyền điều khiển + giải phóng chế độ chuột"** (mouse-mode clear). Cờ này bị reset 0 ở `FUN_00603f20` (teardown/nạp map mới, có `TMap_Create`) → đúng semantics "bật khi vào world xong". |
| SubOp **0x01, 0x02, 0x08** truyền `gvar_007D9C48` — chính là **object cache `TWorldPlayer` 2100 slot** mà OP 0x03/0x04/0x17 dùng (`FUN_0072174c`/`FUN_00722950`/`FUN_00722cd4`/`FUN_00728a6c` đều có đối số 1 này) | Ba nhánh này **patch bản ghi cache nhân vật từ xa** (họ hàng với parser appearance/equipment của OP 0x17 — `FUN_00728a6c` là sibling sát nhất, có body). |
| SubOp **0x00, 0x05, 0x09** truyền `gvar_007D9D34` — **scene/world manager** (mảng actor `gvar_007DA300` 800 slot, count `+0x5c`, gate `+0x53fc`) | Ba nhánh là **cập nhật trạng thái phía cảnh/actor** (không phải form UI). |
| SubOp **0x03** truyền `gvar_007DA7BC` — **object người chơi local** (`+4` = charID, `+9` = tên) | Patch trạng thái **chính nhân vật ta** (cluster 0x72e-0x72f = hoạt hình/skill-engine, xem 4.4). |
| SubOp **0x06** truyền `gvar_007DA688` — form có **danh sách bản ghi stride 0x105** (buf `+0x174`, đếm `+0x170`, chuỗi khóa `+0x178`), method `FUN_005752a8` **được gọi khi actor local chết** (`*(short*)(actor+0x3ea)==0` trong `FUN_00652454`) và **được gọi 8 lần lặp lại từ trong chính gap `0x00576325..0x00576810` chứa `func_0x005763c8`** (xref `00576453/4a0/4ed/53a/587/5d4/621/66e → FUN_005752a8`) | Nhánh 0x06 = **đổ dữ liệu danh sách chọn (khả năng cao danh sách kiểu "lựa chọn khi chết/hồi sinh" hoặc bảng mục)** vào form; và chính form này **phát C→S `SendCommand(5, CL=6)`** khi người dùng chốt mục → thành vòng request↔response khép kín (mục 5). |

Kết luận một dòng: *OP 0x05 không chứa stat số học (đã có ở 0x08), không chứa hồ sơ spawn (0x03/0x04) — nó là lớp "event/flag/patch" lên các đối tượng thế giới đã tồn tại, trong đó sự kiện được định danh trọn vẹn duy nhất là LevelUP (0x0A).*

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Định tuyến (xác minh lại từ hex gốc)
```
Frame:  [44 F4] [Len L: Word LE] [Payload L bytes]   (XOR toàn khung 0xAD)
payload[0]=0x05 (MainOp) → bảng byte 0x78A8EE: byte_table[0x05]=0x06
→ bảng dword 0x78A9B6[6] = 0x0078CA56 → FUN_0078ca56   (Case 6)
```
Kiểm tra trực tiếp `ts_decompile/redump/`: dòng 1 byte table = `01 02 03 04 05 06 ...` (index 5 → 0x06) ✔; dword table offset 6 = `56 CA 78 00` = 0x0078CA56 ✔.

### 2.2. Đầu handler (file `case_006_0078CA56_FUN_0078ca56.c`, dòng 21–28)
```c
iVar3 = *(int *)(unaff_EBP + -0xc);                  // RestPayload (ECX-slot của dispatcher frame)
if (*(int *)(iVar3 + -4) == 0) iVar1 = _BoundErr(0);  // bounds-check Delphi (Length=0 → runtime err)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar3 + iVar1); // SubOp = RestPayload[0] = payload[1]
switch(*(undefined4 *)(unaff_EBP + -0x14)) { ... }
```
- **`SubOp = ECX[0] = payload[1]`**, đọc byte thường (không codec).
- `EBP-0xc` = biến AnsiString `RestPayload` của dispatcher (`Copy(payload, 2, len-1)` — xem `opcode_00_01.md` §2.2).
- **Cách các nhánh gọi helper** (khuôn Delphi `procedure X(Obj; const Pkt: string)`):
  `func_0x00727164(*(gvar_007D9C48), *(EBP-0xc))` → param_1 = **trỏ tới object toàn cục** (dereference con trỏ gvar), param_2 = **chuỗi RestPayload**. Helper tự **đọc lại SubOp tại RP[0]** (vì `Copy(RP,2,...)` bắt đầu field từ byte 2) và tự parse field — nên *mọi field wire đều nằm ở `payload[2..]` trở đi, không trừ thêm byte nào*.
- **Epilogue** (dòng 62–90): `*in_FS_OFFSET = ...; _LStrArrayClr/_LStrClr(...)` — **KHÔNG phải logic**, chỉ là SEH-unwind + dọn AnsiString local của frame dispatcher (giống hệt `case_000` default).
- Codec chuẩn: `_LStrCopy(RP, p, n)` = 1-based trên RestPayload ⇒ `payload[p .. p+n-1]`; `FUN_0077ef7c` = **DWORD LE**; `FUN_0077eb9c` = **Word LE**. *Lưu ý decompile*: đối số đầu của 2 codec (và của `FUN_0077f414`) thường là `*(gvar_007D9D30)` / `EBP-4` — **artifact thanh ghi EAX** (slot managed-result của Delphi), **không phải tham số nghiệp vụ** (thân `FUN_0077ef7c` chỉ dùng param_2 — đã đọc body xác nhận).

### 2.3. Danh sách switch (chính xác, từ cả 3 bản: file case, `jumptable_0x78A9B6_cases.c:1536-1566`, dispatcher `.c:1744-1787` — khớp nhau 100%)
`case 0, 1, 2, 3, 4, 5, 6, 8, 9, 10` → **SubOp hợp lệ: 0x00–0x06, 0x08, 0x09, 0x0A**. **Không có `case 7`**, **không có `default`** → SubOp 7 hoặc >0x0A rơi thẳng xuống epilogue, vô hại.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Handler làm gì (dòng case_006) | Đối tượng đích / định danh | Body helper trong dump? | Độ tin cậy nghiệp vụ |
|:---:| :--- | :--- | :---: | :--- |
| `0x00` | `func_0x00731534(gvar_007D9D34, RP)` (d.29-31) | Scene/world manager (800 actor) | ✘ gap `0x731433..0x731a9b` | Thấp-TB: "cập nhật actor/cảnh" |
| `0x01` | `func_0x00727164(gvar_007D9C48, RP)` (d.32-34) | Cache `TWorldPlayer` 2100 slot | ✘ gap `0x7241ca..0x7281ec` | TB: patch bản ghi cache (unit 0x72x = appearance/cache) |
| `0x02` | **giống hệt 0x01** (d.35-37) | idem | ✘ | TB: biến thể thứ 2 của cùng routine |
| `0x03` | `func_0x0072e9a0(gvar_007DA7BC, RP)` (d.38-40) | **Player local (self)** | ✘ gap `0x72e994..0x72f6a4` | TB: patch state/anim của self |
| `0x04` | inline (d.41-46): `scene+0x53fc=1`; `self+0x348=GetTickCount`; `FUN_0079b620(TMouseInfo,0)` | scene gate + tick + `TMouseInfo` | ✔ (inline + FUN_0079b620) | **CAO — xác minh byte-for-byte** |
| `0x05` | `func_0x00732368(gvar_007D9D34, RP)` (d.47-49) | Scene manager | ✘ gap `0x73231c..0x73262c` | Thấp-TB |
| `0x06` | `func_0x005763c8(gvar_007DA688, RP)` (d.50-52) | Form danh sách record 0x105B (dùng chung `FUN_005752a8` với luồng chết/HP=0) | ✘ gap `0x576325..0x576810` (nhưng bên trong có xref 8×→`FUN_005752a8`) | TB: đổ/refresh danh sách chọn |
| `0x07` | **KHÔNG TỒN TẠI** | — | — | — |
| `0x08` | `func_0x007289b8(gvar_007D9C48, RP)` (d.53-55) | Cache `TWorldPlayer` | ✘ gap `0x7282df..0x728a6c` (sibling `FUN_00728a6c` ✔ có body) | TB: patch cache record (họ appearance) |
| `0x09` | `func_0x0074f9f8(gvar_007D9D34, RP)` (d.56-58) | Scene manager (cluster 0x74f = patcher vùng record `+0x15xx`) | ✘ gap `0x74f80c..0x74fb94` | Thấp-TB |
| `0x0A` | `FUN_0072f970(gvar_007D9D34, RP)` (d.59-61) | Scene manager + self/actor lookup | ✔ `0072f970_FUN_0072f970.c` | **CAO — LevelUP** |
| khác | rơi qua switch → epilogue dọn frame | — | — | an toàn |

---

## 4. Chi tiết từng SubOp (wire layout + logic)

Quy ước: `p[k]` = byte payload gốc 0-based (`p[0]=0x05`, `p[1]=SubOp`); `RP[r] = p[r+1]`; endian **LE**; `_LStrCopy(RP, x, n)` → `p[x .. x+n-1]`.

### 4.1. SubOp `0x04` — World-ready / mở gate điều khiển + reset chuột — **độ tin cậy CAO (inline, byte-exact)**
- **Wire**: `[0x05][0x04]` — **2 byte payload, không field thêm** (handler không đọc thêm byte nào của RP).
- **Logic** (case_006 d.41–46; dispatcher `.c` d.1761–1767):
  1. `*(*(int*)gvar_007D9D34 + 0x53fc) = 1` — bật gate cảnh. Chuỗi bằng chứng gate: `TForm1.KeyboardWalk` chỉ cho đi bộ khi `*(DAT_0092322c + 0x53fc) != 0` (DAT_0092322c alias của scene object trong FormCreate); `DXDraw1MouseDown` d.130 cùng kiểm tra; `FUN_00717e78`/`FUN_00719170`/`FUN_0073ce00` (vòng tick actor 1..800) chỉ xử lý khi `+0x53fc != 0`; và `FUN_00603f20` (luồng rời/reset map, tạo `TMap_Create`) **set lại 0** (d.234). ⇒ semantics: **"thế giới đã nạp xong, được tương tác"**.
  2. `*(*(int*)gvar_007DA7BC + 0x348) = kernel32.GetTickCount()` — stamp thời gian lên player. Vùng `+0x348` là họ "tick mốc/ready-at" (`FUN_006a96d0` **đọc** so tick) — *suy luận: mốc "được hành động tiếp"*; **độ tin cậy TB** cho tên gọi, CAO cho thao tác ghi.
  3. `FUN_0079b620(*(int*)gvar_007DA0B4, 0)` — `gvar_007DA0B4 = TMouseInfo_Create(VMT_79B548_TMouseInfo)` (`0050a4a0_TForm1.FormCreate.c` d.472-473); thân `FUN_0079b620`: `if (param_2 != -1) obj+0x24 = param_2; obj+0x2c = 0` → **xóa mode lệnh chuột (`+0x24=0`) và cờ pending (`+0x2c=0`)**. (Các hàm UI gọi cùng hàm này với 3 = bật mode; `DXDraw1MouseMove` ghi `TMouseInfo+4/+8` = tọa độ.)
- Trình bày/hiệu ứng: không có.

### 4.2. SubOp `0x0A` — **LevelUP (lên cấp)** — `FUN_0072f970` @ `0x0072F970` — **độ tin cậy CAO (byte-exact)**
- **Wire**: `[0x05][0x0A][charID: p2..p5 = RP[1..4], DWORD LE]` — 6 byte.
- **Đọc**: `_LStrCopy(RP, 2, 4, &tmp)` → `FUN_0077ef7c` → `charID = local_10`.
- **Logic**:
  - **Nếu `charID == self`** (`*(int*)(gvar_007DA7BC+4) == charID`): `FUN_0071e004(self, "LevelUP", 0)` — **gắn nhãn sự kiện lên actor**: `actor+0x490 := "LevelUP"` (chuỗi), `+0x48c := 0` (arg), `+0x498 := Now()` (double — mốc thời gian để fade), `+0x4a0 := 0`. Tiếp đó nối đường dẫn `gvar_007DA010 + "sound\\WA0013.wav"` và gọi `FUN_007a7f20` phátเสียง. *(Âm thanh = presentation — 1 dòng.)*
  - **Nếu là actor khác**: `idx = FUN_0070c20c(scene=gvar_007D9D34, charID)` (tìm trong 800 actor, mảng con trỏ `gvar_007DA300`); nếu `idx != 0` (1..800): `FUN_0071e004(gvar_007DA300[idx], "LevelUP", 0)` — **không phát tiếng** cho người khác.
- **Hiệu ứng nghiệp vụ**: chỉ **gắn tag trạng thái "LevelUP" có timestamp** cho hệ thống render nameplate/anim; **không sửa stat** (HP/EXP/Level thuộc OP 0x08 attrCode `0x24/0x25`). Đây là kênh *event*, không phải *data*.

### 4.3. SubOp `0x01`, `0x02` — `func_0x00727164(cache TWorldPlayer, RP)` (patch bản ghi nhân vật từ xa) — **body helper ✘**
- **Wire đã biết ở tầng handler**: `[0x05][0x01|0x02][... không xác định, helper tự đọc từ RP[1]...]` — handler **không cắt byte nào**; độ dài field tối thiểu ≥ 1 byte RP để helper không rớt `_BoundErr`.
- **Bằng chứng suy luận phạm vi** (độ tin cậy TB):
  - `func_0x00727164` nằm trong gap `0x7241CA..0x7281EC` của **unit cache `gvar_007D9C48`** — cùng unit với `FUN_0072174c` (parse hồ sơ spawn, OP 0x03/0x04), `FUN_00722464` (cấp slot), `FUN_00722508` (tìm slot theo charID tại `gvar_007DA6BC`), `FUN_00722950` (nạp cache→actor), `FUN_00722cd4` (parser record lớn, dispatcher OP 0x17), `FUN_007281ec` (OP 0x19).
  - Toàn bộ các "packet-parser" cùng hình dạng `(cacheObj, RP)` đã decompile trong unit này **mở đầu bằng đúng một khuôn**: `_LStrCopy(RP,2,4)+FUN_0077ef7c → charID` rồi `FUN_00722508` để định vị record (chứng minh ở `FUN_007281ec`, `FUN_00728a6c`, và `FUN_0072f970` ở mục 4.2). ⇒ **Giả định loại cao** cho layout: `[05][01|02][charID:4B LE][payload theo loại]`; **nội dung field sau charID: chưa xác định**.
  - Hai SubOp 0x01/0x02 gọi **cùng một routine** → khác biệt (nếu có) chỉ ở một mã loại đọc trong payload hoặc record field — **không kết luận được**.
- Sibling `FUN_00728a6c` (body ✔, dùng cho OP 0x17) cho thấy loại field mà unit này xử lý: `charID + byte kiểu + 2×DWORD appearance (FUN_00745bc0 mode 1/5) + 2 byte cờ` → vừa vá actor sống (`obj+0x7b`, `obj+0x9b`), vừa vá record cache (`rec+0x8e/0x3d/0x42`), rồi `FUN_0072fa98` dựng lại trang thiết bị + `FUN_0075eec0` refresh party. ⇒ 0x01/0x02/0x08 nhiều khả năng là **các bản vá nhẹ hơn cùng họ (appearance/flag/tên của actor từ xa)**. Đánh dấu **TRUNG BÌNH — loại suy unit, chưa phải đặc tả**.

### 4.4. SubOp `0x03` — `func_0x0072e9a0(self gvar_007DA7BC, RP)` — **body helper ✘**
- Handler: forward nguyên RP. **Wire: chưa xác định ngoài `[05][03]...`**.
- Ngữ cảnh cluster `0x72e-0x72f`: `FUN_0072e620` đọc `gself+0x376`, `param+0x391==7`, ghi `+0x467` (máy trạng thái hoạt hình); `FUN_0072e724` đọc stat qua getter `FUN_007108f4` và được **game-tick** (`FUN_00717e78`) cùng `FUN_006547a8` (vùng skill-effect engine) gọi; `FUN_0072f83c/FUN_0072fa98` = dựng lại diện mạo. ⇒ Đối số là **self** nên khả năng nhất: **cập nhật cờ/trạng thái hành động của chính nhân vật** (độ tin cậy THẤP-TB).

### 4.5. SubOp `0x00` — `func_0x00731534(scene, RP)` và SubOp `0x05` — `func_0x00732368(scene, RP)` — **body helper ✘**
- **Wire: chưa xác định** (`[05][00|05]...`), cùng giới hạn như trên.
- Sibling sát nhất `FUN_00731388` (cluster của `func_0x00731534`; được dispatcher OP 0x17 gọi): khuôn y hệt — đọc `charID = Copy(RP,2,4)+ef7c`, nếu là self thì gọi **VMT+0x24** của player, ngược lại `FUN_0070c20c` tìm actor rồi gọi **VMT+0x24** của actor (method "làm mới sau thay đổi"). `FUN_00731a9c` (cùng cluster) được `DXDraw1MouseDown` gọi (click chọn/tương tác actor); `FUN_00731ffc` được `KeyboardWalk` gọi (rời vị trí + SendCommand). ⇒ 0x73xxxx = **bộ điều khiển tương tác/tân trạng thái actor trong cảnh**; SubOp 0x00/0x05 gần như chắc thuộc loại *"state/action event cho một charID"* (độ tin cậy THẤP-TB — đánh dấu rõ là loại suy).

### 4.6. SubOp `0x06` — `func_0x005763c8(form gvar_007DA688, RP)` — **body helper ✘, nhưng cấu trúc loop ĐÃ định vị được**
- **Wire: chưa xác định** (`[05][06]...`).
- Bằng chứng định danh mạnh nhất: **gap chứa `func_0x005763c8` (0x576325..0x576810) có 8 điểm gọi `FUN_005752a8`** (`References to entry` của `005752a8_FUN_005752a8.c`: `00576453, 005764a0, 005764ed, 0057653a, 00576587, 005765d4, 00576621, 0057666e` — cách đều 0x4d = cùng một khối lặp/bảng nhánh). `FUN_005752a8(form, itemID)` tra `itemID` trong **buf bản ghi stride 0x105 tại form+0x174**, chuyển mục giữa hai danh sách con (`+0x14c` list A, grid `+0x150/+0x15c/+0x184`), set cờ `rec+0x104 = 1`, và **khi điều kiện byte đạt, gọi `SendCommand(DL=5, CL=6)`** (asm `005752a8` d.136–142) — chính là phía gửi đã bị "gộp break" trong `.c` (mục 5).
- Form này còn được **`FUN_00652454` (vòng hiệu ứng/skill) gọi cấp mục khi `*(short*)(actor+0x3ea) == 0`** (HP hiện tại = 0 → actor chết) và khi `actor` là self (`actor+4 == self+4`), cùng `FUN_005ee30c` (tick pump OP 0x14). ⇒ Bức tranh loại cao: **form "danh sách lựa chọn" do server đổ dữ liệu qua SubOp 0x06, người dùng chốt → client gửi (5, CL=6)** — ngữ cảnh xuất hiện gồm cả tình huống *chết/hồi sinh*. Đánh dấu **TRUNG BÌNH** cho kết luận vai trò, **KHÔNG có kết luận** cho layout byte.

### 4.7. SubOp `0x08` — `func_0x007289b8(cache TWorldPlayer, RP)` — **body helper ✘**
Như 4.3, nhưng là hàm **độc lập** nằm ngay sau `FUN_007281ec` và trước `FUN_00728a6c` (cửa sổ 0x7282DF..0x728A6C, size ≤ 0xB4): cùng unit cache. **Wire: chưa xác định ngoài `[05][08]...`** (TB).

### 4.8. SubOp `0x09` — `func_0x0074f9f8(scene, RP)` — **body helper ✘**
- Cluster 0x74f = "patcher vùng bản ghi mở rộng của người chơi": sibling `FUN_0074f034` (OP 0x45) đọc `[code:1B][val:1B][word:2B LE]` từ `RP[1..4]` rồi switch `code 0x1B..0x20` ghi cặp `self+0x1553+2·i` (Word) + `self+0x155f+i` (Byte) — đúng kiểu **attrCode→offset** của `FUN_00710ab0` nhưng cho vùng `+0x15xx`; sibling `FUN_0074fb94` (OP 0x45) đọc `size=Word(RP[1..2])`, nếu `==0x5D` copy **0x5D byte RP[3..end] nguyên khối** vào `self+0x151d` và bắn toast (`(**gvar_007DA084 + 0x90)(..., msg 0x74fc88, 2000)`) khi byte đầu record = 2; sibling `FUN_0074f504` mở `TLH_ChildSkillDelForm` (gvar_007D9E3C, định danh tại `0051189c` d.1479-1480). ⇒ Nhiều khả năng SubOp 0x09 là **block-patch vùng record trạng thái mở rộng**, nhưng object nhận là **scene** chứ không phải self ⇒ **không khẳng định**; **wire chưa xác định** (THẤP-TB).

### 4.9. graphics/sound/animation — tóm lược 1 dòng (theo yêu cầu)
Âm thanh duy nhất bị chạm trực tiếp trong các body đã decompile của OP 0x05 là `sound\WA0013.wav` (LevelUP-self, `FUN_0072f970` → `FUN_007a7f20`, base `gvar_007DA010`) và toast qua VMT+0x90 của `gvar_007DA084` (họ sibling 4.8); còn lại là các method VMT dựng lại actor (`FUN_0072fa98`, `FUN_00745bc0`...) — **presentation, không phải đồng bộ dữ liệu**.

---

## 5. Chiều Client → Server (C→S) liên quan — `TFConnect.SendCommand` (`0077f414_FUN_0077F414.c`)

**Phát hiện 2 tầng (phải đọc cả hai mới đúng):**

1. **Tầng decompile C**: `switch(param_2 & 0xff)` có `case 5: break;` (dòng **828–829**) — *trông như* OP 0x05 không phía gửi, giống `case 2` (OP 0x02) và `case 8` (OP 0x08) vốn đã xác nhận rỗng thật.
2. **Tầng asm — bằng chứng ngược (xác nhận client CÓ phát lệnh với MainOp=5)** — đúng hiệu ứng "jump-table bị gộp" mà `opcode_14.md` §5.1 đã cảnh báo cho OP 0x14:
   - `005752a8_FUN_005752a8.asm.txt` d.138–142: `MOV EAX,[0x7d9d30]; MOV CL,0x6; MOV DL,0x5; CALL 0x0077f414` ⇒ **SendCommand(MainOp=0x05, SubSel=6)** — phát khi người dùng **chọn/chốt mục** trên form `gvar_007DA688` (mục 4.6).
   - `00766638_FUN_00766638.asm.txt` d.79–83: `MOV CL,0x7; MOV DL,0x5; CALL 0x0077f414` ⇒ **SendCommand(MainOp=0x05, SubSel=7)** — phát khi **bấm chuyển tab/trang 1..3** của form `gvar_007D9F1C` (constructor `FUN_0075ca6c` — họ panel đội viên/quân-đoàn-style, cùng object mà `FUN_0075eec0`/`FUN_007665cc` refresh; handler còn set `self+0x1451 := 0|1|2` theo tab).
   - Kiến trúc SendCommand: `param_2 = MainOp | (SubSel << 8)` — byte thấp là opcode (đã chứng minh ở OP 0x14: `MOV DL=opcode, MOV CL=subsel`), dispatch ngoài qua **bảng byte `0x77F474` + bảng dword `0x77F53C`**; SubSel chỉ chọn *template payload nội bộ*.
   - **Candidate builder (loại suy, chưa chốt)**: trong asm của `0077f414` có chuỗi kiểm SubSel `SUB AL,0x6 → JZ 0x0077fbb2` / `DEC AL (==0x7) → JZ 0x0077fc13`, khớp đúng cặp SubSel 6/7 mà hai call-site trên dùng: khối `0x77fbb2` dựng payload **2 byte** `[0x05][byte self+0x63c]` (khuôn `PStrNCat(...,2)` + `AddSedQueue [0x7da664]`); khối `0x77fc13` dựng prefix `[0x05]` + chuỗi ngắn `0x78A880` (2 ký tự) rồi **nối danh sách khóa của form gvar_007DA688** qua `FUN_00576810(form, &out)` (hàm này serialize `form+0x170` byte tại `form+0x178` thành chuỗi). **KHÔNG được coi là đặc tả** cho tới khi redump được 2 bảng `0x77F474`/`0x77F53C`.
- **Kết luận có thể dùng ngay cho mock server**: client *có thể* phát frame opcode `05` (kịch bản 6 = "chốt mục danh sách chọn", 7 = "đổi tab panel"); payload chính xác = **độ tin cậy THẤP**, cần động (hook socket) để chốt. Vòng pairing đáng tin: **S→C `[05][06]` đổ danh sách ⇒ C→S `(05,6)` xác nhận** (hai đầu đều bám cùng form + cùng method `FUN_005752a8`).

---

## 6. Ghi chú cho Mock Server

1. **Frame & pacing** như mọi OP S→C: `44 F4 | Len LE | payload`, XOR `0xAD` toàn khung, payload bắt đầu `05 | SubOp`; tối đa 50 gói/tick 30ms.
2. **Chỉ có 2 nhánh "an toàn tuyệt đối" về format** để test sớm:
   - `[05][04]` — bật gate tương tác cảnh + reset chuột. **Nên gửi ngay sau chuỗi spawn (OP 0x03/0x04)** vì `KeyboardWalk`/chuột/tick actor bị khóa cho tới lúc đó (`scene+0x53fc`).
   - `[05][0A][charID:4B LE]` — hiệu ứng LevelUP cho actor có ID chỉ định; id là **charID của actor đang tồn tại** (self qua `gself+4` hoặc actor trong 800 slot) — nếu id không tìm thấy, gói rơi im lặng.
3. **Đừng gửi SubOp `0x07` hoặc > `0x0A`**: không có nhánh → vô nghĩa (không crash, rơi epilogue).
4. **Các nhánh 0x00/0x01/0x02/0x03/0x05/0x06/0x08/0x09**: *chưa đủ bằng chứng byte layout để tự động sinh packet*. Khuyến nghị mock: **chưa phát** các sub này cho tới khi redump được 7 helper (xem mục 7.4). Nếu muốn thử, khuôn an toàn nhất theo loại suy unit là `[05][sub][charID:4B LE][...]` với payload đủ dài (helper sẽ `_BoundErr` nếu chuỗi ngắn hơn field cần đọc ⇒ **thà gửi dư byte đuôi còn hơn thiếu**).
5. **Ràng buộc tồn tại đối tượng**: sub chạm `gvar_007D9C48` (0x01/0x02/0x08) chỉ có nghĩa khi record charID đã nằm trong cache 2100 slot (`gvar_007DA6BC`) do OP 0x03/0x04 đổ vào; sub chạm scene (0x00/0x05/0x09) tương tự với actor trong `gvar_007DA300` — gửi trước khi spawn = nhiều khả năng bị nuốt.
6. **Vòng UI (sub 0x06)**: nếu mock server định dùng danh sách lựa chọn kiểu chết/hồi sinh, cần *redump `func_0x005763c8`* để biết format đổ mục; phía phản hồi của client (`(05,CL=6)`) hiện **không lên wire** nếu `.c` đúng (builder rỗng) — kiểm chứng bằng hook trước khi dựa vào.
7. **Không có request-refresh**: toàn bộ OP 0x05 là server-push sự kiện; mock không cần handler chiều lên (ngoài điểm 6).

---

## 7. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
| :--- | :--- | :--- | :--- |
| 1 | `ts_decompile/case_functions/functions/case_006_0078CA56_FUN_0078ca56.c` | @`0x0078CA56`; SubOp d.21–27; switch d.28–61; epilogue d.62–90 | Danh sách case (0–6,8,9,10; không 7/default), khuôn gọi helper |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 5:` d.1734–1788 (inline song song) | Đối chiếu cross-check byte-for-byte |
| 3 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | d.1536–1566 | Bản gộp xác nhận lần 3 |
| 4 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex` | index 5 → 0x06; dword[6] = 0x0078CA56 | Ánh xạ MainOp 0x05 → Case 6 |
| 5 | `ts_decompile/functions/0072f970_FUN_0072f970.c` | @`0x0072F970`; Copy(2,4)+ef7c d.52–53; branch self/other d.54–67; literal `"LevelUP"`, `"sound\\WA0013.wav"` | Wire + logic SubOp 0x0A (CAO) |
| 6 | `ts_decompile/functions/0079b620_FUN_0079b620.c` | @`0x0079B620`; body d.99–107 | Semantics `+0x24/+0x2c` của SubOp 0x04 bước 3 |
| 7 | `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c` | d.470–473 (`TMouseInfo_Create → gvar_007DA0B4`) | Định danh TMouseInfo |
| 8 | `ts_decompile/functions/005186e4_TForm1.KeyboardWalk.c` (d.64), `0050bff8_TForm1.DXDraw1MouseDown.c` (d.130), `00717e78`(d.199,550), `00719170`(d.82), `0073ce00`(d.65), `00603f20`(d.234) | — | Vai trò gate `scene+0x53fc` (bật ở sub 0x04, reset khi đổi map) |
| 9 | `ts_decompile/functions/0071e004_FUN_0071e004.c` | body: ghi `+0x490` string, `+0x48c`, `+0x498=Now()`, `+0x4a0=0` | Bản chất "nhãn sự kiện có timestamp" của LevelUP |
| 10 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | d.1479–1480 (`gvar_007D9E3C=TLH_ChildSkillDelForm`), d.1594–1596 (`gvar_007DA688` ← `FUN_00575590`) | Định danh form/globals |
| 11 | `ts_decompile/functions/00728a6c_FUN_00728a6c.c` (body full), `007281ec`, `00731388`, `0074f034`, `0074f504`, `0074fb94` | xem §4.3/4.5/4.8 | Sibling có body — cơ sở loại suy unit cho các helper gap |
| 12 | `ts_decompile/index.csv` (tra theo dải địa chỉ) + `ls functions/` | 7 helper KHÔNG có entry | Bằng chứng phủ định "body không tồn tại trong dump" |
| 13 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `switch(param_2 & 0xff)` d.768; `case 5: break;` d.828–829 | Tầng .c của C→S |
| 14 | `ts_decompile/functions/0077f414_FUN_0077F414.asm.txt` | prologue d.19–20 (`[EBP-6]=CL, [EBP-5]=DL`), d.33–34 (bảng `0x77F474`→`0x77F53C`), chain `SUB AL,6/DEC AL` + builder `0x77fbb2`, `0x77fc13` (d.204–232) | Kiến trúc SubSel + candidate builders (05,6)/(05,7) |
| 15 | `ts_decompile/functions/005752a8_FUN_005752a8.asm.txt` d.136–142; `00766638_FUN_00766638.asm.txt` d.77–83 | `MOV CL,6/7; MOV DL,5; CALL 0x77f414` | 2 call-site gửi OP 0x05 (CAO) |
| 16 | `ts_decompile/functions/005752a8_FUN_005752a8.c` (References d.36–46), `00576810_FUN_00576810.c` (serialize `+0x170/+0x178`), `00652454_FUN_00652454.c` d.565–581 (HP=0 → FUN_005752a8), `005ee30c` d.48–52 | — | Định vị hình dạng func_0x005763c8 + ngữ cảnh form gvar_007DA688 |
| 17 | `ts_decompile/functions/0075ca6c_FUN_0075ca6c.c` (constructor của gvar_007D9F1C; data-xref `0x75cea0 → FUN_00766638`) | header + d.372 | Chủ thực sự của handler tab (5,CL=7) |
| 18 | `ts_decompile/functions/00722508` (2100-slot cache `gvar_007DA6BC`), `0070c20c` (800-actor lookup), `00745bc0` (decode appearance), `0077ef7c/0077eb9c` (codec — đã đọc body xác nhận param_1 là EAX-artifact) | như dẫn | Họ helper dùng chung |
| 19 | `.scratch/op-code/`: `handoff-opcode-exploration-guide.md`, `opcode_00_01.md`, `opcode_02.md`, `opcode_03.md`, `opcode_08.md`, `opcode_1a.md` (tiền lệ "helper không body — ghi rõ giới hạn"), `opcode_14.md` (§5.1–5.2 **tiền lệ artifact `case N: break;` của SendCommand**) | — | Đối chiếu, tránh trùng/phỏng đoán sai |

### 7.4. Việc cần làm để "chốt" phần còn treo (đề xuất redump)
1. Decompile (tăng timeout / Ghidra script force-analyze) 7 entry: `0x00731534, 0x00727164, 0x0072e9a0, 0x00732368, 0x005763c8, 0x007289b8, 0x0074f9f8` → đủ wire layout cho 8 sub-op.
2. Redump data: `0x77F474` (byte table C→S), `0x77F53C` (dword table C→S), và các literal `0x78A860/0x78A880`, `0x74FC88`, `0x7282EC/0x728300` → chốt payload `(05,6)/(05,7)` + tên toast.
3. Hook socket động 1 phiên thật để ghi lại frame opcode `05` do client phát (kiểm chứng mục 5).

*Ghi chú độ tin cậy tổng: mục 2, 3, 4.1, 4.2 = CAO (mã nguồn sơ cấp, byte-exact). Mục 4.3–4.8 (phần wire) = chỉ cam kết tới `[MainOp][SubOp]`; phần còn lại là loại suy unit, đã dán nhãn TB/THẤP-TB tại chỗ, **không phải đặc tả**. Mục 5: call-site = CAO; payload candidate = THẤP. Không có kết luận nào dựa trên tên hàm tự đặt hay chuỗi chưa giải mã; các chuỗi đã giải chỉ là literal trực tiếp trong decompile (`"LevelUP"`, `"sound\\WA0013.wav"`).*
