# PHÂN TÍCH — Main OP 0x05 (Case 6, `FUN_0078ca56` @ `0x0078CA56`) — aLogin.exe

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều chính: **Server → Client (S→C)**
Trạng thái: **Đã xác minh 100% ở tầng handler** (file case + dispatcher inline + bảng jump redump). **6/7 helper ủy quyền payload đã có body** (cập nhật 2026-09-14 — xem từng §4.x); **chỉ còn `func_0x005763c8` (SubOp 0x06) chưa có body** — nhánh đó giữ nguyên ghi chú giới hạn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Đính chính giả định nghiệp vụ

1. **Không có giả định cũ cần bác bỏ**: `handoff-opcode-exploration-guide.md` KHÔNG gán nghiệp vụ cho OP 0x05 (mục 3 ưu tiên chỉ liệt kê 0x02/0x08/0x14/0x17/0x1A/0xC7). → Đây là phân tích gốc đầu tiên.
2. **Đính chính hình dung "handler lớn"**: `FUN_0078ca56` là một **thuần dispatcher** (~60 dòng lệnh): đọc 1 byte SubOp, `switch`, mỗi nhánh **chuyển nguyên con RestPayload cho một procedure cấp phát sẵn** theo khuôn `helper(đối_tượng_toàn_cục, &RestPayload)`. Toàn bộ logic giải mã field nằm **bên trong helper**, không nằm trong handler.
3. **Giới hạn nguồn (cập nhật 2026-09-14)**: cả 6 helper `func_0x00731534`, `func_0x00727164`, `func_0x0072e9a0`, `func_0x00732368`, `func_0x007289b8`, `func_0x0074f9f8` **đã có body** trong `ts_decompile/functions/` (`00727164_FUN_00727164.c` … `0074f9f8_FUN_0074f9f8.c`, đều có dòng trong `index.csv`). **DUY NHẤT `func_0x005763c8` (SubOp 0x06) vẫn thiếu body** → wire layout của sub-op đó vẫn *không xác định*. Với 6 helper đã có body, các §4 tương ứng dưới đây **chuyển từ loại suy sang đặc tả byte-exact**; các nhãn "loại suy unit / chưa kết luận" cũ được thay bằng bằng chứng trực tiếp (hoặc được đính chính khi code nói khác).
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
| SubOp **0x01, 0x02, 0x08** truyền `gvar_007D9C48` — chính là **object cache `TWorldPlayer` 2100 slot** mà OP 0x03/0x04/0x17 dùng — **body mới xác nhận:** 0x01/0x02 = tháo/mặc **một món trang bị** của actor từ xa (cache + actor sống + preview login), 0x08 = vá byte cờ `rec+0x8c` của record cache | Ba nhánh = **patch hồ sơ nhân vật từ xa**; cụ thể hóa tại mục 4.3/4.7 (họ equipment/flag, đúng như loại suy cũ về "appearance/equipment family") |
| SubOp **0x00, 0x05, 0x09** truyền `gvar_007D9D34` — **scene/world manager** (mảng actor `gvar_007DA300` 800 slot, count `+0x5c`, gate `+0x53fc`) — **body mới xác nhận:** 0x00 = thay **danh sách trang bị** của một actor từ xa (bỏ qua self), 0x05 = đổi/phục hồi **diện mạo** theo (charID, code) qua THuman tạm, 0x09 = làm mới **chuỗi tên follower slot 0..4** của actor | Ba nhánh = **cập nhật actor trong cảnh** theo charID (không phải form UI) — xem mục 4.5/4.8 |
| SubOp **0x03** truyền `gvar_007DA7BC` — **object người chơi local** (`+4` = charID, `+9` = tên) — **body mới xác nhận:** là **snapshot trạng thái đầy đủ của self**: class `+0x3e9`, HP `+0x3ea`, chòm stat Word, cờ job-state `+0x3fa`, các ID/flags vùng `+0x13xx`, ID `+0x5f0` tra `gvar_007D9C20`, và mảng đuôi `[code:2B][flag:1B]` tại `+0xfd4` | Không chỉ "patch cờ/trạng thái" như phỏng đoán cũ — là **gói tải toàn bộ hồ sơ state của chính nhân vật ta** (mục 4.4) |
| SubOp **0x06** truyền `gvar_007DA688` — form có **danh sách bản ghi stride 0x105** (buf `+0x174`, đếm `+0x170`, chuỗi khóa `+0x178`), method `FUN_005752a8` **được gọi khi actor local chết** (`*(short*)(actor+0x3ea)==0` trong `FUN_00652454`) và **được gọi 8 lần lặp lại từ trong chính gap `0x00576325..0x00576810` chứa `func_0x005763c8`** (xref `00576453/4a0/4ed/53a/587/5d4/621/66e → FUN_005752a8`) | Nhánh 0x06 = **đổ dữ liệu danh sách chọn (khả năng cao danh sách kiểu "lựa chọn khi chết/hồi sinh" hoặc bảng mục)** vào form; và chính form này **phát C→S `SendCommand(5, CL=6)`** khi người dùng chốt mục → thành vòng request↔response khép kín (mục 5). |

Kết luận một dòng *(cập nhật 2026-09-14)*: *OP 0x05 là lớp "patch/event" lên các đối tượng thế giới đã tồn tại — 6/7 helper đã có body và lộ rõ bản chất **thiết bị + diện mạo + hồ sơ state** (0x00/0x01/0x02/0x03/0x05/0x08/0x09), bên cạnh hai event đặc thù đã biết là world-ready (0x04) và LevelUP (0x0A); riêng 0x06 (form danh sách chọn) vẫn chờ body.*

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
| `0x00` | `func_0x00731534(gvar_007D9D34, RP)` (d.29-31) | Actor từ xa trong mảng 800 slot (BỎ QUA nếu charID = self) | ✔ `00731534_FUN_00731534.c` | **CAO — byte-exact** (xem 4.5) |
| `0x01` | `func_0x00727164(gvar_007D9C48, RP)` (d.32-34) | Cache `TWorldPlayer` 2100 slot + actor sống + preview đăng nhập | ✔ `00727164_FUN_00727164.c` | **CAO — byte-exact** (xem 4.3) |
| `0x02` | **giống hệt 0x01** (d.35-37) | idem — routine đọc byte RP[0] làm mã thao tác | ✔ | **CAO** (0x01 = tháo item, 0x02 = mặc item) |
| `0x03` | `func_0x0072e9a0(gvar_007DA7BC, RP)` (d.38-40) | **Player local (self)** | ✔ `0072e9a0_FUN_0072e9a0.c` | **CAO** — snapshot trạng thái đầy đủ của self (xem 4.4) |
| `0x04` | inline (d.41-46): `scene+0x53fc=1`; `self+0x348=GetTickCount`; `FUN_0079b620(TMouseInfo,0)` | scene gate + tick + `TMouseInfo` | ✔ (inline + FUN_0079b620) | **CAO — xác minh byte-for-byte** |
| `0x05` | `func_0x00732368(gvar_007D9D34, RP)` (d.47-49) | Actor tự xa HOẶC self (danh sách 6-byte/record) | ✔ `00732368_FUN_00732368.c` | **CAO — byte-exact** (xem 4.5) |
| `0x06` | `func_0x005763c8(gvar_007DA688, RP)` (d.50-52) | Form danh sách record 0x105B (dùng chung `FUN_005752a8` với luồng chết/HP=0) | ✘ **VẪN THIẾU** — không có `005763c8_*.c`/`.asm.txt`, không có dòng `index.csv` (kiểm tra `ls` 2026-09-14); cấu trúc loop đã định vị qua xref 8×→`FUN_005752a8` trong gap `0x576325..0x576810` | TB: đổ/refresh danh sách chọn; wirelayout **chưa xác định** |
| `0x07` | **KHÔNG TỒN TẠI** | — | — | — |
| `0x08` | `func_0x007289b8(gvar_007D9C48, RP)` (d.53-55) | Cache `TWorldPlayer` | ✔ `007289b8_FUN_007289b8.c` | **CAO** — 1 byte → `rec+0x8c` (xem 4.7) |
| `0x09` | `func_0x0074f9f8(gvar_007D9D34, RP)` (d.56-58) | Actor + mảng 5 con trỏ đi kèm `actor+0x57c[0..4]` | ✔ `0074f9f8_FUN_0074f9f8.c` | **CAO** — đổi tên follower (xem 4.8) |
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

### 4.3. SubOp `0x01`, `0x02` — `func_0x00727164` — **THÁO/MẶC MỘT MÓN TRANG BỊ của actor từ xa — CAO (body mới, byte-exact)**
- **Wire:** `[05][01|02][charID:4B LE][itemCode:Word LE]` — **đúng 8 byte payload**. `charID = Copy(RP,2,4)+FUN_0077ef7c` (`00727164_FUN_00727164.c:118-119`); `itemCode = Copy(RP,6,2)+FUN_0077eb9c` (d.121-122).
- **Đính chính phỏng đoán cũ:** "(khác biệt giữa 0x01/0x02 chỉ ở một mã loại đọc trong payload — không kết luận được)" → **mã thao tác CHÍNH LÀ byte RP[0]** (tức SubOp — helper đọc lại RP[0] làm mode: `local_1f = *(char*)(RP+0)`, d.123-129): **`0x01` = THÁO (unequip)**, **`0x02` = MẶC (equip)**. Hai nhánh case gọi cùng một routine nên không cần byte loại riêng trong payload.
- **Điều kiện:** nếu `charID == gself+4` → **không làm gì cả** (d.120 — đây là packet dành cho người khác).
- **Ô trang bị không nằm trên wire**: `slot = byte tại rec+0x34 của bản ghi CSDL item` tra theo itemCode (`FUN_007746ac(gvar_007DA540, code)`, guard rec ≤ 10000, slot ≤ 6; d.130-140) — cùng nguồn CSDL với vòng spawn OP 0x03/0x04.
- **Nhánh 0x01 (tháo)** — trên cache record `gvar_007DA6BC[idx]` (idx = `FUN_00722508`, d.141-143): `rec+0x70[slot*4]`: `+4=0`, `+0x10=0`, `+0x14=0xFFFFFFFF` (d.144-176); **riêng slot == 5** (vũ khí?): khôi phục 3 byte ngoại hình `rec+0x5a/0x5b/0x5c := rec+0x6c/0x6d/0x6e` (d.177-214). Nếu `gself+0x376 != 0` (cờ "phiên bản xem trước ở màn hình nhân vật"? — chưa kết luận tên) → vá nốt object preview `gvar_007DA51C+0x158+i*4` (i = `FUN_006441bc(…, charID, 2, 0)`, guard ≤0x14) y hệt (d.215-297).
- **Nhánh 0x02 (mặc)** — cache record: `+4=itemCode`, `+0x10=slot`, `+0x14 = FUN_007c9b38(gfx-mgr gvar_007D9ED8, IntToStr(Word rec+0x1c của DB item)))` (d.299-343); rồi nếu byte `rec+0x1c` (guard 1..2) ≠ 0: **4 lớp phủ ngoại hình** `FUN_00745bc0(cache+0x42, dword tại DB+0x32/0x3a/0x42/0x4a + kind*4, slot, mode 1..4)` (d.349-449); preview tương tự (d.450-611).
- **Sau cache, luôn thử trên actor sống:** `idx = FUN_0070c20c(gvar_007D9D34, charID)`; nếu actor tồn tại → đúng cùng các thao tác mode 1/mode 2 trên `actor+0x2c[slot*4]`, `actor+0x9b`, với `kind = actor+0x08` (guard 1..2) và slot-5-mirror `actor+0xb3/0xb4/0xb5 := actor+0xc5/0xc6/0xc7` (d.615-843).
- Kết luận: phỏng đoán cũ "patch bản ghi cache, họ appearance" → **đúng hướng**, cụ thể là **cập nhật thiết bị đơn (equip/unequip) + vẽ lại sprite/overlay, đồng bộ cache + actor sống + preview login**.

### 4.4. SubOp `0x03` — `func_0x0072e9a0` — **SNAPSHOT TRẠNG THÁI ĐẦY ĐỦ CỦA SELF — CAO (body mới, byte-exact)**
- **Định dạng cố định**: phần đầu đúng `0x71` byte payload (`p[0]=0x05, p[1]=0x03`, field từ `p[2]`), phần đuôi là dãy record 3 byte. param_1 = object self (`gself`), param_2 = RP. Body là một chuỗi "đọc-Copy-ghi offset" thuần túy (`0072e9a0_FUN_0072e9a0.c:88-313`):

| Trường wire (payload) | Size | Ghi vào `gself+…` | Dòng |
|:---|:---:|:---|:---|
| `p[2]` | 1B (guard ≥2) | `+0x3e9` (Class/Job — trùng field OP 0x04) | d.88-99 |
| `p[3..4]` | Word | `+0x3ea` (HP hiện tại — `opcode_06` dùng `+0x3ea==0` làm điều kiện chết) | d.100-103 |
| `p[5..18]` | 7×Word | `+0x3ec/+0x3ee/+0x3f0/+0x3f2/+0x3f4/+0x3f6/+0x3f8` (chòm stat) | d.104-131 |
| `p[0x13]` | 1B (guard ≥0x13) | `+0x3fa` (job-state byte — chính là byte chữ ký sigA của C→S OP 0x06) | d.132-138 |
| `p[0x14..0x17]` | DWORD | `+0x3fc` | d.139-142 |
| `p[0x18..0x1b]` | 2×Word | `+0x400/+0x402` | d.143-150 |
| `p[0x1c..0x1f]` | DWORD | `+0x440`; sau đó `+0x43e = FUN_0065b30c(gvar_007DA660, val)` (tra CSDL, 1 byte) | d.151-156 |
| `p[0x20..0x23]` | 2×Word | `+0x404/+0x406` | d.157-164 |
| `p[0x24..0x3b]` | 6×DWORD | `+0x40c/+0x410/+0x414/+0x418/+0x41c/+0x420` | d.165-188 |
| `p[0x3c..0x45]` | 5×Word | `+0x430/+0x432/+0x434/+0x436/+0x438` | d.189-208 |
| `p[0x46..0x49]` | DWORD (`FUN_0077ed68` = codec 4 byte, xem 4.4b) | `+0x1334`; `+0x1330 = Word(FUN_0065b484(gvar_007DA660, val, 0))` (guard ≤0xffff) | d.209-217 |
| `p[0x4a], p[0x4b]` | 2×1B (guard) | `+0x1305`, `+0x1306` | d.218-231 |
| `p[0x4c..0x67]` | 7×DWORD | `+0x136b/+0x137e/+0x1391/+0x13a4/+0x13cb/+0x13de/+0x13b7` — mỗi cái kèm **1 byte tra CSDL** `gvar_007DA660` ghi vào `+0x1373/…/+0x13bf` (guard ≤0xff) | d.232-294 |
| `p[0x68..0x6b]` | DWORD | `+0x5f0`; sau đó `+0x47c = FUN_00759050(gvar_007D9C20, val)` | d.295-315 |
| `p[0x6c]` | 1B (guard) | `+0x1308` | d.299-305 |
| `p[0x6d..0x70]` | 2×Word | `+0x408/+0x40a` | d.306-313 |
| Đuôi: `n = (Len(RP)-0x70)/3` record **3 byte** `[itemCode:Word][flag:Byte]` từ `p[0x71]` | 3·n | `idx = FUN_00759b04(gvar_007DA554, code)` (≤0xff) → `gself+0xfd4+idx*3 := code`, `+0xfd6+idx*3 := flag`; **nếu code == `0x36b3` và flag ≠ 0 → `FUN_007b0094(gvar_007DA108+0x13c, 1)`** (trước vòng lặp luôn set cờ = 0) | d.316-405 |

- **Kết thúc hàm (UI)**: `FUN_0058dbbc(gvar_007DA530, self+0x3e9)`; nếu `gvar_007D9EB8+400 ∈ {self, 0}` → `FUN_005a0388(gvar_007D9EB8, self)`; nếu `self+0x43e > 0xc` → `FUN_00596a7c(gvar_007DA1DC, 1)` (d.406-413). *(presentation, 1 dòng)*
- **4.4b — codec `FUN_0077ed68`**: đọc 4 byte thường `b0 + b1·0x100 + b2·0x10000 + b3·0x1000000`, kèm guard BoundErr/IntOver từng byte — **tương đương `FUN_0077ef7c`** (`0077ed68_FUN_0077ed68.c:88-123`). Kết quả trả về EAX (Ghidra hiển thị void + `extraout` trong các caller).
- **Đính chính phỏng đoán cũ** ("cập nhật cờ/trạng thái hành động của chính nhân vật, loại suy cluster 0x72e"): cluster đúng (unit hoạt hình/engine), nhưng nội dung thực là **toàn bộ hồ sơ trạng thái số học của self** — class, HP, chòm stat Word, các ID/flags vùng `+0x13xx`, một ID lớn `+0x5f0` tra bảng bạn hữu, và mảng record 3 byte `[code][flag]` tại `+0xfd4` (256 slot, DB `gvar_007DA554`) — **không phải event nhỏ**. Ý nghĩa từng field cụ thể (attr code nào là gì) vẫn **chưa kết luận được** ngoài các offset/size/hướng ghi ở bảng trên; các reader/writer đã biết: `+0x3e9` Class, `+0x3ea` HP (đọc ở OP 0x14/`FUN_00652454`), `+0x3fa` job-state (chữ ký sigA OP 0x06).

### 4.5. SubOp `0x00` — `func_0x00731534` — **THAY DANH SÁCH TRANG BỊ actor từ xa** và SubOp `0x05` — `func_0x00732368` — **DANH SÁCH ĐỔI/PHỤC HỒI DIỆN MẪO theo code** — **CAO (body mới, byte-exact)**
- **`0x00` (`00731534_FUN_00731534.c`) — wire:** `[05][00][charID:4B LE][code_1:Word LE][code_2:Word LE]…` với `N = (Len(RP)−5) div 2` Word mã item bắt đầu từ `p[6]` (d.141-146, 157-170).
  - **Nếu charID == self → no-op** (d.76); `idx = FUN_0070c20c(scene, charID)`, `idx==0` → no-op (d.77-78).
  - Actor tìm được (`gvar_007DA300[idx]`): **reset 6 ô trang bị `[1..6]`** (không đụng ô 0): `slotObj+4 = 0`, `slotObj+0x14 = 0xFFFFFFFF` (d.79-104); mirror `+0xc5/0xc6/0xc7 → +0xb3/0xb4/0xb5` (d.105-140).
  - Mỗi code ≠ 0: `recIdx = FUN_007746ac(gvar_007DA540, code)` (≤10000); `slot = byte DB+0x34+recIdx*0x172` (≤6); `actor+0x2c+slot*4`: `+4=code`, `+0x10=slot`, `+0x14 = FUN_007c9b38(gvar_007D9ED8, IntToStr(Word DB+0x1c+…))`; nếu `kind = actor+0x08 ∈ {1,2}` → 4×`FUN_00745bc0(actor+0x9b, dword DB+0x32/0x3a/0x42/0x4a + kind·4, slot, mode 1..4)` (d.174-345).
  - ⇒ Cùng khuôn CSDL/sprite/overlay với §4.3 và vòng equip của spawn OP 0x03/0x04, nhưng là **bản vá lại toàn bộ list trang bị của một actor đã spawn** (thay vì hồ sơ spawn đầy đủ).
- **`0x05` (`00732368_FUN_00732368.c`) — wire:** `[05][05]` + `N = Len(RP) div 6` record **6 byte** `[charID:4B LE][code:Word LE]`, record đầu tại `p[2]` (d.54-59, 61-76).
  - Một object tạm `THuman_Create(VMT_70B1C4)` dựng trước vòng lặp, hủy cuối hàm (d.56, 145).
  - Với mỗi record: actor = self nếu charID khớp, ngược lại `FUN_0070c20c(gvar_007D9D34, charID)`; không thấy → bỏ record (d.78-90). Gọi **`VMT+0x1c` trên THuman tạm với (code, 0)`** — method dựng "diện mạo theo code" vào các field `temp+0x7c`, `temp+0x9b` (d.92; nội dung VMT+0x1c **chưa kết luận được** — chỉ biết nó biến code 2 byte thành hồ sơ +0x7c/+0x9b).
  - `code == 0` → **phục hồi**: nếu là self, copy từ record cache số 0 (`*(int*)gvar_007DA6BC`, bản sao hồ sơ SELF mà OP 0x03 ghi — `opcode_03` bước 5): `+0x1c→actor+0x08`, `+0x3c→+0x7a`, `+0x3d→+0x7b`, `+0x3e→+0x7c(Word)`, `+0x42→+0x9b` (0x2d byte) (d.93-107); nếu là actor khác, chỉ `actor+0x08 := rec(charID)+0x1c` (d.108-116).
  - `code != 0` → **áp diện mạo mới**: `actor+0x08 = 0`; `actor+0x7c(Word) = temp+0x7c`; copy `temp+0x9b.. → actor+0x9b` (0x2d byte); `VMT+0x18(actor, 8)` (chơi action/mode 8 — chưa kết luận tên); `actor+0xE4 := actor+0xE3` (đồng bộ hướng render); nếu self → refresh 2 panel `gvar_007D9E5C`/`gvar_007DA32C` (virtual `+0x20`, khi `+0x14 != 0`) (d.118-139).
  - ⇒ Cơ chế là **"đội lốt/tháo lốt" hàng loạt theo (charID, code)** — có thể là hóa trang/đổi diện mạo; **tên nghiệp vụ chưa kết luận được**. Phỏng đoán cũ "state/action event cho một charID" đúng họ nhưng **sai chi tiết**: không phải event trạng thái chung mà là bản vá diện mạo qua THuman mẫu.
- Cả hai helper đều **không dùng param_1 như nghiệp vụ** (`00731534` dùng nó làm đối số `FUN_0070c20c`; `00732368` chỉ lưu `local_8` rồi đọc global).

### 4.6. SubOp `0x06` — `func_0x005763c8(form gvar_007DA688, RP)` — **body helper ✘ VẪN THIẾU (giữ nguyên giới hạn)**
- **Wire: chưa xác định** (`[05][06]...`). **Kiểm tra lại 2026-09-14**: sau đợt bổ sung ~245 body mới, `ts_decompile/functions/` **vẫn không có** `005763c8_*.c`/`.asm.txt`, `index.csv` không có dòng — đây là **helper duy nhất của OP 0x05 còn thiếu**; mọi §4 khác đã upgrade lên CAO. Không suy diễn wire layout.
- Bằng chứng định danh mạnh nhất: **gap chứa `func_0x005763c8` (0x576325..0x576810) có 8 điểm gọi `FUN_005752a8`** (`References to entry` của `005752a8_FUN_005752a8.c`: `00576453, 005764a0, 005764ed, 0057653a, 00576587, 005765d4, 00576621, 0057666e` — cách đều 0x4d = cùng một khối lặp/bảng nhánh). `FUN_005752a8(form, itemID)` tra `itemID` trong **buf bản ghi stride 0x105 tại form+0x174**, chuyển mục giữa hai danh sách con (`+0x14c` list A, grid `+0x150/+0x15c/+0x184`), set cờ `rec+0x104 = 1`, và **khi điều kiện byte đạt, gọi `SendCommand(DL=5, CL=6)`** (asm `005752a8` d.136–142) — chính là phía gửi đã bị "gộp break" trong `.c` (mục 5).
- Form này còn được **`FUN_00652454` (vòng hiệu ứng/skill) gọi cấp mục khi `*(short*)(actor+0x3ea) == 0`** (HP hiện tại = 0 → actor chết) và khi `actor` là self (`actor+4 == self+4`), cùng `FUN_005ee30c` (tick pump OP 0x14). ⇒ Bức tranh loại cao: **form "danh sách lựa chọn" do server đổ dữ liệu qua SubOp 0x06, người dùng chốt → client gửi (5, CL=6)** — ngữ cảnh xuất hiện gồm cả tình huống *chết/hồi sinh*. Đánh dấu **TRUNG BÌNH** cho kết luận vai trò, **KHÔNG có kết luận** cho layout byte.

### 4.7. SubOp `0x08` — `func_0x007289b8(cache, RP)` — **PATCH 1 BYTE `rec+0x8c` — CAO (body mới, byte-exact)**
- **Wire:** `[05][08][charID:4B LE][byte:1B]` — **7 byte payload**. `charID = Copy(RP,2,4)+FUN_0077ef7c` (`007289b8_FUN_007289b8.c:42-43`); guard `Len(RP) ≥ 6` rồi đọc `byte = RP[5] = p[6]` (d.45-52).
- **Logic:** `idx = FUN_00722508(cacheObj, charID)`; **chỉ khi record cache tồn tại** (0 nếu chưa spawn — không cấp slot mới) ghi `gvar_007DA6BC[idx] + 0x8c := byte` (guard 2100 slot, d.53-58). **Không đụng actor sống, không đụng preview.**
- **Kết nối OP 0x03/0x04:** `rec+0x8c` chính là byte cờ được đổ từ `p[2N+31]` lúc parse hồ sơ spawn (bảng `opcode_04` §4.1, dòng `rec+0x8c` "chỉ cache") — SubOp 0x08 là **lệnh vá lại đúng byte cờ đó** khi giá trị đổi mà không cần gửi lại cả hồ sơ. Ý nghĩa nghiệp vụ của byte: **chưa kết luận được** (không có reader nào nằm trong đường đã decompile).

### 4.8. SubOp `0x09` — `func_0x0074f9f8(scene, RP)` — **LÀM MỚI TÊN follower slot i của một actor — CAO (body mới)**
- **Wire:** `[05][09][charID:4B LE][slot i:1B][word:2B LE]` — **9 byte payload**. charID = `Copy(RP,2,4)` (d.52-53); `i = RP[5] = p[6]` guard `Len≥6` (d.54-60); `word = Copy(RP,7,2) = p[7..8]` (d.61-62).
- **Đính chính phỏng đoán cũ** ("block-patch vùng record mở rộng `+0x15xx` của self"): **SAI đối tượng và sai cơ chế**. Body thực không chứa vùng `+0x15xx` nào.
- **Logic thực** (`0074f9f8_FUN_0074f9f8.c:64-95`): actor = self (charID khớp `gself+4`) **hoặc** `gvar_007DA300[FUN_0070c20c(scene,charID)]` (0 → bỏ qua); rồi: `elem = *(actor + 0x57c + i*4)` (guard `i ≤ 4` — **mảng 5 con trỏ đi kèm của actor**, cùng họ follower `+0x57C[i]` mà `FUN_00715b28` đồng bộ — `opcode_06` §7); đọc chuỗi PChar tại `elem+9` → `local_20`; gọi **virtual `VMT+0x1c` trên `elem`** (tham số theo asm: `EDX = word p[7..8]`, `CL = 1` — Ghidra `.c` không hiện; `0074f9f8_FUN_0074f9f8.asm.txt:84-93`); cuối cùng ép `local_20` (bản **đọc trước** lệnh gọi) qua `_LStrToString(...,0xff)` và **ghi ngược** `elem+9 := ... cắt 17 byte` (`_PStrNCpy`).
- ⇒ Cơ chế đọc được nguyên văn: "gọi method +0x1c (chưa có VMT export để định danh) lên object follower thứ i với tham số word, rồi chuẩn hóa chuỗi tên tại `+9` xuống tối đa 17 byte". Đọc ngữ cảnh (mảng +0x57c, tên 17 byte): **nhiều khả năng là đổi tên/thay trạng thái thú đồng hành (follower) của nhân vật** — *interpretation, chưa kết luận được* vì VMT+0x1c của class phần tử chưa định danh và tham số word không có reader nào khác được quan sát.

### 4.9. graphics/sound/animation — tóm lược 1 dòng (theo yêu cầu)
Âm thanh duy nhất bị chạm trực tiếp trong các body đã decompile của OP 0x05 là `sound\WA0013.wav` (LevelUP-self, `FUN_0072f970` → `FUN_007a7f20`, base `gvar_007DA010`); toast qua VMT+0x90 của `gvar_007DA084` nằm ở sibling OP 0x45 `FUN_0074fb94` (không phải trong helper 0x09 — 0074f9f8 không bắn toast); ngoài ra là các method dựng lại actor (`FUN_00745bc0`, `FUN_007c9b38`, VMT+0x18/+0x1c, refresh panel `gvar_007D9E5C/007DA32C`) — **presentation, không phải đồng bộ dữ liệu**.

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
2. **Các nhánh "an toàn về format" đã chốt byte-exact (cập nhật 2026-09-14)** — ngoài `[05][04]` và `[05][0A][charID:4B]` cũ:
   - `[05][04]` — bật gate tương tác cảnh + reset chuột. **Nên gửi ngay sau chuỗi spawn (OP 0x03/0x04)** vì `KeyboardWalk`/chuột/tick actor bị khóa cho tới lúc đó (`scene+0x53fc`).
   - `[05][0A][charID:4B LE]` — hiệu ứng LevelUP cho actor có ID chỉ định; id là **charID của actor đang tồn tại** (self qua `gself+4` hoặc actor trong 800 slot) — nếu id không tìm thấy, gói rơi im lặng.
   - `[05][01|02][charID:4B LE][itemCode:Word LE]` (8 B) — tháo/mặc trang bị cho actor **từ xa** (charID=self → no-op); itemCode **phải tồn tại** trong CSDL item client, vì ô trang bị lấy từ DB (`+0x34`), không từ wire.
   - `[05][00][charID:4B LE][code_1:Word]…[code_N:Word]` — đặt lại **toàn bộ** list trang bị của actor từ xa (reset 6 ô rồi dựng lại; code `0` = bỏ qua mục đó).
   - `[05][05]([charID:4B][code:Word])*` (6 B/record) — đổi/phục hồi diện mạo; `code = 0` → phục hồi từ cache.
   - `[05][08][charID:4B LE][byte:1B]` (7 B) — vá cờ `rec+0x8c` của record cache (yêu cầu đã spawn).
   - `[05][09][charID:4B LE][slot:1B ≤4][word:2B]` (9 B) — đổi tên/refresh follower thứ `slot` (self hoặc actor; không có actor → bỏ qua).
3. **Đừng gửi SubOp `0x07` hoặc > `0x0A`**: không có nhánh → vô nghĩa (không crash, rơi epilogue).
4. **Nhánh còn thiếu layout duy nhất: `0x06`** (`[05][06]...`) — `func_0x005763c8` **vẫn chưa có body** sau đợt bổ sung 2026-09-14 → **chưa phát** sub này cho tới khi redump được (xem mục 7.4). Nếu muốn thử các sub khác theo khuôn đã chốt ở mục 2, lưu ý helper sẽ `_BoundErr` nếu chuỗi ngắn hơn field cần đọc ⇒ **thà gửi dư byte đuôi còn hơn thiếu**.
5. **Ràng buộc tồn tại đối tượng**: sub chạm `gvar_007D9C48` (0x01/0x02/0x08) chỉ có nghĩa khi record charID đã nằm trong cache 2100 slot (`gvar_007DA6BC`) do OP 0x03/0x04 đổ vào (body mới xác nhận: `FUN_00722508` trả 0 → các sub này **không tự cấp record**); sub chạm scene (0x00/0x05/0x09) tương tự với actor trong `gvar_007DA300` (0x00 còn **loại trừ self** ngay từ cửa). Gửi trước khi spawn = bị nuốt.
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
| 11 | **Sáu body helper MỚI (2026-09-14)**: `00727164_FUN_00727164.c` (d.118-843), `007289b8_FUN_007289b8.c` (d.42-58), `0072e9a0_FUN_0072e9a0.c` (d.88-413), `00731534_FUN_00731534.c` (d.74-357), `00732368_FUN_00732368.c` (d.54-150), `0074f9f8_FUN_0074f9f8.c` (+ `.asm.txt:84-93` tham số VMT+0x1c) | như dẫn | Wire + logic byte-exact cho SubOp 0x00/0x01/0x02/0x03/0x05/0x08/0x09 (CAO) |
| 11b | Sibling có body: `00728a6c_FUN_00728a6c.c`, `00731388`, `0074f034`, `0074f504`, `0074fb94` | xem §4 | Ngữ cảnh unit (không còn là căn cứ duy nhất) |
| 12 | `ts_decompile/index.csv` (tra theo dải địa chỉ) + `ls functions/` | 6 helper trên **ĐÃ có entry**; **duy nhất `005763c8` không có** | Bằng chứng phủ định chỉ còn áp dụng cho SubOp 0x06 |
| 13 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `switch(param_2 & 0xff)` d.768; `case 5: break;` d.828–829 | Tầng .c của C→S |
| 14 | `ts_decompile/functions/0077f414_FUN_0077F414.asm.txt` | prologue d.19–20 (`[EBP-6]=CL, [EBP-5]=DL`), d.33–34 (bảng `0x77F474`→`0x77F53C`), chain `SUB AL,6/DEC AL` + builder `0x77fbb2`, `0x77fc13` (d.204–232) | Kiến trúc SubSel + candidate builders (05,6)/(05,7) |
| 15 | `ts_decompile/functions/005752a8_FUN_005752a8.asm.txt` d.136–142; `00766638_FUN_00766638.asm.txt` d.77–83 | `MOV CL,6/7; MOV DL,5; CALL 0x77f414` | 2 call-site gửi OP 0x05 (CAO) |
| 16 | `ts_decompile/functions/005752a8_FUN_005752a8.c` (References d.36–46), `00576810_FUN_00576810.c` (serialize `+0x170/+0x178`), `00652454_FUN_00652454.c` d.565–581 (HP=0 → FUN_005752a8), `005ee30c` d.48–52 | — | Định vị hình dạng func_0x005763c8 + ngữ cảnh form gvar_007DA688 |
| 17 | `ts_decompile/functions/0075ca6c_FUN_0075ca6c.c` (constructor của gvar_007D9F1C; data-xref `0x75cea0 → FUN_00766638`) | header + d.372 | Chủ thực sự của handler tab (5,CL=7) |
| 18 | `ts_decompile/functions/00722508` (2100-slot cache `gvar_007DA6BC`), `0070c20c` (800-actor lookup), `00745bc0` (decode appearance), `0077ef7c/0077eb9c` (codec — đã đọc body xác nhận param_1 là EAX-artifact) | như dẫn | Họ helper dùng chung |
| 19 | `.scratch/op-code/`: `handoff-opcode-exploration-guide.md`, `opcode_00_01.md`, `opcode_02.md`, `opcode_03.md`, `opcode_08.md`, `opcode_1a.md` (tiền lệ "helper không body — ghi rõ giới hạn"), `opcode_14.md` (§5.1–5.2 **tiền lệ artifact `case N: break;` của SendCommand**) | — | Đối chiếu, tránh trùng/phỏng đoán sai |

### 7.4. Việc cần làm để "chốt" phần còn treo (cập nhật trạng thái 2026-09-14)
1. ~~Decompile 7 entry~~ → **ĐÃ có 6/7** (`0x00731534, 0x00727164, 0x0072e9a0, 0x00732368, 0x007289b8, 0x0074f9f8` đã có body). **Còn treo đúng 1 entry: `0x005763c8`** (SubOp 0x06) — decompile để chốt wire layout nhánh cuối.
2. Redump data: `0x77F474` (byte table C→S), `0x77F53C` (dword table C→S), và các literal `0x78A860/0x78A880`, `0x74FC88`, `0x7282EC/0x728300` → chốt payload `(05,6)/(05,7)` + tên toast *(vẫn mở — không nằm trong đợt redump này cho OP 0x05)*.
3. Hook socket động 1 phiên thật để ghi lại frame opcode `05` do client phát (kiểm chứng mục 5) *(vẫn mở)*.
4. **Mới (từ body)**: hai VMT chưa định danh được gọi trong các helper — `VMT+0x1c` trên `THuman (VMT_70B1C4)` và object follower `actor+0x57c[i]` (SubOp 0x05/0x09) — dump VMT để xác nhận semantics (appearance-template / rename). Cờ `gself+0x376` (kích hoạt vá preview `gvar_007DA51C` ở SubOp 0x01/0x02) cũng chưa có nhãn.

*Ghi chú độ tin cậy tổng (cập nhật 2026-09-14): mục 2, 3, 4.1, 4.2 = CAO (mã nguồn sơ cấp, byte-exact). Mục **4.3–4.8 = CAO về wire + thao tác** vì 6/7 helper giờ có body; các offset/size/hướng ghi là nguyên văn từ decompile. Tuy vậy **nhãn ngữ nghĩa** cho nhiều field (attr code nào là gì, VMT+0x1c là method gì, cờ `+0x376`, byte `rec+0x8c`) **vẫn chưa kết luận được** — chỉ mô tả thao tác, không đặt tên nghiệp vụ suông. Mục 4.6 (SubOp 0x06) = **wire chưa xác định** (`func_0x005763c8` vẫn thiếu body) — cam kết chỉ tới `[MainOp][SubOp]`. Mục 5: call-site = CAO; payload candidate = THẤP (chờ redump bảng `0x77F474/0x77F53C`). Không có kết luận nào dựa trên tên hàm tự đặt hay chuỗi chưa giải mã; các chuỗi đã giải chỉ là literal trực tiếp trong decompile (`"LevelUP"`, `"sound\\WA0013.wav"`).*
