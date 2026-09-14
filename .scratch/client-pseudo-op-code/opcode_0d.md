# PHÂN TÍCH — Main OP 0x0D (Case 13, `FUN_0078dbae` @ `0x0078DBAE`) — **Đồng bộ team/party + toast hệ thống (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Tóm tắt nghiệp vụ

**OP 0x0D là kênh "đồng bộ team/party + toast hệ thống":** 1 byte SubOp chọn 1 trong 13 đường xử lý. Các đường đọc được body cho thấy: SubOp 2 = cờ 1 byte + toast có điều kiện; SubOp 3 = kind + targetId + toast theo kind (chỉ khi id trùng self); SubOp 4 = rời/kick/disband team + refresh UI; SubOp 6 = parser đồng bộ team nhiều nhóm (vòng lặp teamId + count + members, quan trọng nhất OP); SubOp 7/8 = set/clear id console + refresh. Trước đây 7 đường (1, 5, 9–13) là pass-through mù; **từ 2026-09-14 đã có body 6/7 đường (1, 9, 0x0A, 0x0B, 0x0C, 0x0D)** — còn đúng **một** handler vắng body là `007a20b8` (SubOp 5). Điểm chung các body mới: tất cả đều đi qua bảng actor scene 800 slot (`FUN_0070c20c`/`gvar_007DA300`) và object team `gvar_007D9D64`; bốn SubOp 9/0x0A/0x0B/0x0C là họ "mở dialog / bật–tắt cờ `+0x5d8` theo id", SubOp 0x0D là bảng toast 5 nhánh theo 1 byte mã. Chuỗi toast SubOp 2/3 (`0x796f50/68/84/a4`) **đã dump và giải mã** (xem §6).

---

## 1. Entry, mapping, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_013_0078DBAE_FUN_0078dbae.c:8` — `void FUN_0078dbae(void)` @ `0x0078DBAE`
- `ts_decompile/case_functions/manifest.csv:15` — `13,0x0078A9EA,0x0078DBAE,EXPORTED,FUN_0078dbae`
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — cùng body.

**Dispatcher:** `FUN_0078a89c` tra bảng byte `0x78A8EE` lấy index, rồi tra bảng dword `0x78A9B6` để nhảy tới hàm xử lý (`case_001` đến `case_065`). MainOp `0x0D` → Case 13 → `FUN_0078dbae`.

**Khung mạng:** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD`. Server speaks first.

**Mapping MainOp 0x0D → Case 13:**
- Dispatcher S→C là `ts_decompile/functions/0078a89c_FUN_0078a89c.c:27` — `local_9 = param_2` chính là **MainOp**, `local_10 = param_1` là **RestPayload (đã cắt MainOp)**.
- `switch(local_9)`; nhánh `case 0xd:` tại **dòng 2399** chứa toàn bộ SubOp 1..13, byte-for-byte giống file case riêng.
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex`: hàng đầu `01 02 ... 0C 0D 0E 0F` → index `0x0D` = giá trị `0x0D` (identity).
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex`: entry thứ 13 là `AE DB 78 00` = `0x0078DBAE` (LE). Khớp manifest.

**Cách đọc SubOp (dòng 26–33 file case):**
```c
iVar3 = *(int *)(unaff_EBP + -0xc);   // ECX = RestPayload
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar3 + 0); // SubOp = ECX[0]
switch(SubOp) { case 1 ... case 0xd ... }  // không có case 0, không có default
```
- `unaff_EBP-0xc` = RestPayload = payload gốc bỏ byte `P[0]=MainOp`.
- **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec.
- Quy ước `_LStrCopy(ECX, p, n, &tmp)` là Delphi `Copy` **1-based**: `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`. Ví dụ `Copy(ECX,2,4)` = `payload[2..5]` (4 byte).

**Codec helper (đã đọc body):**
- `FUN_0077ef7c` — đọc 4 byte `b0+b1*0x100+b2*0x10000+b3*0x1000000` = **DWORD Little-Endian**. Đây là codec duy nhất handler này dùng để decode field.
- `FUN_0077eb9c` — `b0+b1*0x100` = **Word LE**. **Handler 0x0D không gọi hàm này lần nào** (ghi rõ để tránh nhầm).
- `FUN_0077eb1c` — encode Word → chuỗi 2 byte (dùng ở chiều C→S, không dùng ở S→C 0x0D).
- `FUN_0077ee84` — encode DWORD → chuỗi 4 byte (dùng ở C→S, không dùng ở S→C 0x0D).
- `FUN_0077f098` — copy tên 8 byte, **không được handler 0x0D gọi**.

---

## 2. Đối chiếu dispatcher inline

Khối `case 0xd:` trong `0078a89c_FUN_0078a89c.c:2399–2534` khớp 1:1 với file case riêng → Dispatcher không thêm logic nào ngoài jump-table; single source of truth là file case.

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: `1,2,3,4,5,6,7,8,9,10(0xa),11(0xb),12(0xc),13(0xd)`. **Không có `case 0`, không có `default`** → SubOp 0 / ≥14 rơi qua, chỉ cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x0D`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu | Wire field | Handler tầng case |
|---|---|---|---|
| `0x01` | **2** (case) — callee đòi **6** | `id:P2..P5, DWORD LE` (`Copy(RP,2,4)`) — (đính chính: callee CÓ parse id) | `FUN_0063aaf4(gvar_007D9E90, RP)` — có body mới |
| `0x02` | **3** (`0D 02 flag`) | `flag:P2=RP1, 1B` (guard `Len<2→BoundErr(1)`) | nếu `flag==1` → toast `UNK_00796f50` 2000ms |
| `0x03` | **7** (`0D 03 kind + dword`) | `kind:P2=RP1,1B` + `id:P3..P6=RP2..RP5, DWORD LE` | nếu `id==self` thì toast theo `kind` 1/2/3 |
| `0x04` | **6** (`0D 04 dword`) | `id:P2..P5, DWORD LE` (`Copy(RP,2,4)`) | `FUN_007a1964` + điều kiện UI + `FUN_005a3018` |
| `0x05` | **≥2** (`0D 05 rest...`) | pass-through nguyên RP | `func_0x007a20b8(gvar_007D9D64, RP)` — **VẪN THIẾU BODY** |
| `0x06` | **≥2** (`0D 06 rest...`) | pass-through, parse nằm trong callee | `FUN_007a273c` (parser team, có body) |
| `0x07` | **6** (`0D 07 dword`) | `id:P2..P5, DWORD LE` | `*(gvar_007DA7BC+0x54c)=id; FUN_005a3018` |
| `0x08` | **2** (`0D 08`) | không field | `*(+0x54c)=0; FUN_005a3018` |
| `0x09` | **2** (case) — callee đòi **6** | `id:P2..P5, DWORD LE` (`Copy(RP,2,4)`) | `FUN_0063b16c(gvar_007D9D48, RP)` — có body mới |
| `0x0A` | **2** (case) — callee đòi **7** | `kind=RP1 1B` + `id:P3..P6=RP2..5, DWORD LE` (`Copy(RP,3,4)`) | `FUN_007a363c(gvar_007D9D64, RP)` — có body mới |
| `0x0B` | **≥2** — `n=(Len-1)/4` × `id DWORD LE` | mảng id 4 byte liên tiếp từ `RP[1]` | `FUN_007a3894(gvar_007D9D64, RP)` — có body mới |
| `0x0C` | **2** (case) — callee đòi **6** | `id:P2..P5, DWORD LE` | `FUN_007a3aa0(gvar_007D9D64, RP)` — có body mới |
| `0x0D` | **3** (`0D 0D mã`) | `mã:RP1, 1B` | `FUN_007a3b5c(gvar_007D9D64, RP)` — có body mới |

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — `FUN_0063aaf4` (CÓ BODY MỚI — lưu target id + mở dialog xác nhận với actor)
- Wire: `[0D][01][id:P2..P5, DWORD LE]` (`0063aaf4_FUN_0063aaf4.c:38–39` `_LStrCopy(RP,2,4)`+`FUN_0077ef7c`).
- **(đính chính)** Guess cũ "không tách field, forward nguyên RP" chỉ đúng ở tầng case; **callee parse id DWORD** (`0063aaf4_FUN_0063aaf4.c:38`).
- Core (`0063aaf4_FUN_0063aaf4.c:40–52`):
  1. `*(gvar_007D9D64 + 8) := id` — lưu id đích vào object team/manager (field `+8`) (`:40`).
  2. `FUN_007aff84(obj_form, self+0x54, self+0x58)` — 2 DWORD từ PlayerRec làm tham số (`:41–42`).
  3. `idx = FUN_0070c20c(gvar_007D9D34, id)` (`:43`); actor không tồn tại → dừng.
  4. Khi có actor: `FUN_0077680c(gvar_007DA53C)` (`:45`, hàm đóng/reset message form), `FUN_0063c1d0(obj_form, actor, 0)` (`:50` — setter "đối tượng đích" dùng chung ở hàng chục form), rồi `FUN_0063c674(obj_form, 0x63abf8, 0, 0, 0x63aa8c, obj, 0x63aab8, obj)` (`:52`) — khuôn **toast/confirm 3 nút + callback** (`FUN_0063c674` cất con trỏ callback vào `obj+0x1D0..0x1F4` rồi gọi VMT `+0x90`, xem `0063c674_FUN_0063c674.c:46–52`). Text dialog nằm tại các constant code `0x63ABF8/0x63AA8C/0x63AAB8` — **không có dump → chưa dịch được, chưa kết luận được nội dung**.

### 4.2. SubOp `0x02` — cờ 1 byte + toast có điều kiện
- Wire: `[0D][02][flag:P2, 1B]`
- Đọc: guard `if (Len<2) _BoundErr(1)`, `flag=(uint)*(byte*)(RP+1)` = byte thường.
- Core: `if (flag==1) (VMT gvar_007DA084+0x90)(obj, &UNK_00796f50, 2000,0,0)` — toast 2000ms. (Presentation: 1 dòng.)
- Chuỗi `UNK_00796f50` (AnsiString, content tại `0x796F50`, 23 byte): **đã dump + decode** `ts_decompile/redump/lit_796f50.hex` (giải mã **VISCII** — đính chính recipe, xem `opcode_09.md §7.1`). Nội dung chốt: **"Đối phương đang bận rộn"** — không còn là phỏng đoán dựng dấu.

### 4.3. SubOp `0x03` — kind + targetId + toast theo kind (CORE hiển thị của OP này)
- Wire: `[0D][03][kind:P2=RP1,1B][id:P3..P6=RP2..RP5, DWORD LE]`
- Đọc: `*(gvar_007D9E90+0x210)=RP[1]` (byte thường); `_LStrCopy(RP,3,4,&tmp)` = `RP[2..5]`; `id=FUN_0077ef7c(tmp)` = DWORD LE.
- Core:
  1. `if (id != *(gvar_007D9D64+4)) return` — chỉ xử lý khi id trùng id self lưu ở `gvar_007D9D64+4`. Ngược lại bỏ qua.
   2. `kind==1/2/3` → lấy tên base, nối hậu tố rồi toast 1000ms:
      - kind 1: `_PStrNCat(buf,&UNK_00796f68,0x2b=43)`
      - kind 2: `_PStrNCat(buf,&UNK_00796f84,0x2d=45)`
      - kind 3: `_PStrNCat(buf,&UNK_00796fa4,0x1e=30)`
   3. `kind` khác 1/2/3 → không làm gì.
- Chuỗi `0x00796f68 / 0x00796f84 / 0x00796fa4`: **đã dump + decode** (`ts_decompile/redump/lit_796f68.hex`, `lit_796f84.hex`, `lit_796fa4.hex`; shortstring `[len:1B][chars][00]`, decode VISCII):
  - `0x796F68`: len byte `0x1A=26` tại `+0x00`, content `0x796F69..0x796F82` — decode VISCII: **"Đồng ý sự gia nhập của bạn"** (chốt).
  - `0x796F84`: len `0x1C=28` tại `+0x00`, content `0x796F85..0x796F9F` — decode VISCII: **"Cự tuyệt sự gia nhập của bạn"** (chốt).
  - `0x796FA4`: len `0x0D=13` tại `+0x00`, content `0x796FA5..0x796FB1` — decode VISCII: **"Không hồi ứng"** (chốt).
  - Ngữ cảnh 3 biến thể accept/reject/no-response khớp vai trò "trả lời lời mời team" của SubOp 3 — nhưng bản dịch chính xác vẫn đánh dấu phỏng đoán (bảng glyph đơn byte của game chưa chốt, xem thêm ghi chú §6).

### 4.4. SubOp `0x04` — id + đồng bộ rời nhóm/giải tán + refresh UI team
- Wire: `[0D][04][id:P2..P5, DWORD LE]`
- Đọc: `_LStrCopy(RP,2,4)` + `FUN_0077ef7c` → `id`.
- Core (bỏ graphics 1 dòng):
  - `FUN_007a1964(gvar_007D9D64, id)` — xử lý rời/kick/disband + toast (có body, xem 4.9).
  - So sánh cờ `*(gvar_007DA7BC+0x35f)` trước/sau để phát hiện đổi trạng thái team; nếu đổi và `*(gvar_007DA1DC+0x15c)==0x05` → `FUN_005952f4(obj,2)` (chuyển tab kênh chat — thuần UI).
  - Nếu `id==myId (*(gvar_007DA7BC+4))` hoặc `+0x35f==0` → `FUN_0058e944(gvar_007DA530,1)` (set `+0x26d=1`).
  - Luôn `FUN_005a3018(gvar_007D9D7C)` (refresh panel team — thuần UI).

### 4.5. SubOp `0x05` — pass-through
- Wire: `[0D][05][rest...]`
- Core: `func_0x007a20b8(gvar_007D9D64, RP)`.
- **Giới hạn:** không có body `007a20b8` trong `ts_decompile/`. Không suy diễn.

### 4.6. SubOp `0x06` — parser đồng bộ team nhiều nhóm (có body, quan trọng nhất OP)
- Wire tầng case: `[0D][06][rest...]` forward nguyên RP cho `FUN_007a273c(gvar_007D9D64, RP)`.
- Body `ts_decompile/functions/007a273c_FUN_007a273c.c` tự parse vòng lặp:
  - `len=_LStrLen(RP)`; con trỏ `pos=2` (1-based Delphi → `RP[1]` = `P[2]`).
  - Mỗi vòng: `_LStrCopy(RP,pos,4)` → `teamId=FUN_0077ef7c` (DWORD LE); `pos+=4`; đọc 1 byte `count=RP[pos]` (byte thường); tiến con trỏ.
  - Lặp `count` lần: mỗi member `_LStrCopy(RP,pos,4)` → `memberId` DWORD LE, resolve `slot=FUN_0070c20c(mapScene,id)`, lưu cặp `(memberId,slot)` vào mảng tạm.
  - Ghi vào struct actor: `+0x548=teamId`, `+0x578=count` (byte), `+0x550/+0x554` từng slot 8 byte; set `+0x35f=1` cho chủ team, `=2` cho member; clear 5 cờ `+0x57c`.
  - Cuối mỗi team: nếu là self thì `FUN_005a3018` refresh UI.
- Đây là logic **gán team/party nhiều thành viên**, không phải chat.

### 4.7. SubOp `0x07` — set console/team-id + refresh
- Wire: `[0D][07][id:P2..P5, DWORD LE]`
- Đọc: `Copy(RP,2,4)` + DWORD LE.
- Core: `*(gvar_007DA7BC+0x54c)=id; FUN_005a3018(gvar_007D9D7C)` — lưu id console hiện tại rồi refresh panel. (1 dòng UI.)

### 4.8. SubOp `0x08` — clear console + refresh
- Wire: `[0D][08]` (2 byte, không field).
- Core: `*(+0x54c)=0; FUN_005a3018(...)`.

### 4.9. Hàm `FUN_007a1964` (được SubOp 4 gọi — đã đọc body)
- File `ts_decompile/functions/007a1964_FUN_007a1964.c`.
- Core: resolve qua `FUN_0070c20c`; nếu `+4==id` và `+548==myTeam` → toast `DAT_007a2094` + `FUN_007ab870(tag=10)` (chat log); ngược lại xử lý xóa member khỏi mảng `+0x550` (dồn mảng, giảm `+0x578`), sao chép danh sách sang object còn lại, gọi `FUN_007a2604` dọn icon. Toast hiển thị 1200ms — presentation 1 dòng.
- Chuỗi `DAT_007a2094` / `LAB_007a20a8`: có `redump/lit_7A2094.hex`, `lit_7A20A8.hex` nhưng dump chỉ 68 byte trong khi header Delphi khai `len 195 / 251` và phần còn lại lẫn opcode → **không giải mã chắc chắn được, không bịa nội dung tiếng Việt**. Ghi rõ giới hạn.

### 4.10. SubOp `0x09 / 0x0A / 0x0B / 0x0C / 0x0D` — pass-through, **CÓ BODY MỚI 2026-09-14**
- Wire: `[0D][sub][rest...]`, tầng case không tách field (giữ nguyên như bảng §3).
- **0x09 — `FUN_0063b16c(gvar_007D9D48, RP)`** (`0063b16c_FUN_0063b16c.c:38–51`): đọc `id = DWORD LE RP[1..4]`; `idx=FUN_0070c20c(gvar_007D9D34, id)`; nếu actor tồn tại: cất `obj+0x214 := id` (`local_8[0x85]`, `:42`) rồi cùng khuôn dialog với SubOp 1 (`FUN_0077680c` + `FUN_0063c1d0` + `FUN_0063c674` callback `0x63b254/0x63b104/0x63b130` — text không dump, chưa dịch). → **"mở dialog xác nhận với actor id"** (khác SubOp 1 ở object nguồn `gvar_007D9D48` và field lưu `+0x214` vs `gvar_007D9D64+8`).
- **0x0A — `FUN_007a363c(gvar_007D9D64, RP)`** (`007a363c_FUN_007a363c.c:52–101`): `kind = RP[1]` (byte, guard `Len<2→BoundErr(1)`), `id = DWORD LE RP[2..5]` (`Copy(RP,3,4)`); điều kiện chạy: `FUN_0070c20c(scene, id) != 0` (actor phải có trong scene) — **khác SubOp 3** (so id với `gvar_007D9D64+4`); toast 1000ms = shortstring tên actor tại slot `*(gvar_007DA654+0x148)` (chỉ số bảng actor — **chưa kết luận được ý nghĩa con trỏ này**) + nối `DAT_007a3848/LAB_007a3864/DAT_007a3884` theo kind 1/2/3 — **độ dài `_PStrNCat` 43/45/30 đúng bằng bộ ba chuỗi SubOp 3** ⇒ khả năng cao là bản sao chuỗi "Đồng ý/Cự tuyệt/Không hồi ứng của <actor>" nằm trong code region, không có dump → chưa chốt.
- **0x0B — `FUN_007a3894(gvar_007D9D64, RP)`** (`007a3894_FUN_007a3894.c:46–134`): mảng id: `n=(Len-1)/4`, mỗi `id = DWORD LE` tại `RP[1+4i]`. Với mỗi id: resolve actor (self nếu `id==myId`); xét team qua field `+0x548 (teamId)`: nếu `actor+0x548 == myId` lấy self, ngược lại resolve actor của `+0x548`; nếu object đó có `+0x578 (count) ≠ 0`: lặp slot 1..min(count,4) qua mảng member `+0x554+slot*8` (chứa **chỉ số bảng actor**, 0=self) và gọi `FUN_0071fa00` cho từng người (`actor+0x5d8 := 0`, `0071fa00_FUN_0071fa00.c:21`); cuối cùng `FUN_0071dc60(actor)` (`actor+0x5d8 := 1`, `0071dc60_FUN_0071dc60.c:21`). Cơ chế giống hệt SubOp 0x0C (xem dưới) nhưng hàng loạt + theo cặp team. `+0x5d8` = cờ trạng thái team-member nào đó — **chưa kết luận được** semantic game.
- **0x0C — `FUN_007a3aa0(gvar_007D9D64, RP)`** (`007a3aa0_FUN_007a3aa0.c:37–50`): `id = DWORD LE RP[1..4]`; nếu id==myId → `FUN_0071fa00(self)`, nếu actor tồn tại → `FUN_0071fa00(actor)`. Một dòng: **clear cờ `+0x5d8` của actor theo id** (bản đơn của 0x0B).
- **0x0D — `FUN_007a3b5c(gvar_007D9D64, RP)`** (`007a3b5c_FUN_007a3b5c.c:25–76`): `mã = RP[1]` byte (guard `Len<2→BoundErr(1)`), `switch(mã)`:
  - `1/2/3` → toast hằng số code `DAT_007a3ce0 / 007a3d04 / 007a3d2c`, **1200ms** (0x4B0) — không có dump → chưa dịch.
  - `4` → nếu mapId self `PlayerRec+0x63a == 0x2AEF (11007)` toast `DAT_007a3dc8` 2000ms; nếu `mapId ∈ [0x32A9..0x32B6] (12937..12950)` toast `DAT_007a3d6c` 2000ms.
  - `5` → cùng điều kiện map với toast `DAT_007a3e8c` (map 11007) / `DAT_007a3e18` (12937..12950).
  → **toast có điều kiện theo bản đồ** (6 chuỗi constant vùng code, tất cả **chưa có dump**).
- ~~Giới hạn: cả 5 hàm không có body~~ → chỉ còn **SubOp 5 (`007a20b8`)** là thiếu body; Mock Server vẫn forward nguyên rest cho các SubOp này nhưng giờ có thể chủ động đưa id/kind hợp lệ.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`):

```c
case 0xc: break;
case 0xd: break;   // <-- OP 0x0D phía client
case 0xe: break;
```

**Kết luận: client không bao giờ chủ động gửi OP 0x0D.** Chiều duy nhất là S→C. Không có builder payload C→S nào cho OP này (giống OP 0x09, khác OP chat gửi ở 0x1A).

---

## 6. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ `opcode_02.md` mục 5 cũ ghi "cp1258 → NFC" — **đính chính 2026-09-14**: bảng đúng của game là **VISCII đơn-byte tiền tổ hợp** (xem `opcode_09.md §7.1`); dump Delphi ansistring `[len:4LE][chars]` phải giải mã bằng VISCII (+ NFC).

Payload trên wire của OP 0x0D **không mang text** (chỉ `0D | SubOp | kind/flag/id`) — text nằm ở **hằng số trong binary**, client chỉ hiển thị khi nhận đúng SubOp:

| # | Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|---|
| 1 | `UNK_00796f50` | SubOp 2 (toast 2000ms) | **ĐÃ DUMP** `lit_796f50.hex` — content `0x796F50..0x796F66` (23B), decode VISCII: "Đối phương đang bận rộn" (chốt) |
| 2 | `UNK_00796f68` | SubOp 3 kind 1 | **ĐÃ DUMP** (shortstring len 26 @`0x796F68`, content từ `0x796F69`) — decode VISCII: "Đồng ý sự gia nhập của bạn" (chốt) |
| 3 | `UNK_00796f84` | SubOp 3 kind 2 | **ĐÃ DUMP** (shortstring len 28 @`0x796F84`, content từ `0x796F85`) — decode VISCII: "Cự tuyệt sự gia nhập của bạn" (chốt) |
| 4 | `UNK_00796fa4` | SubOp 3 kind 3 | **ĐÃ DUMP** (shortstring len 13 @`0x796FA4`, content từ `0x796FA5`) — decode VISCII: "Không hồi ứng" (chốt) |
| 5 | `DAT_007a2094` / `LAB_007a20a8` | `FUN_007a1964` (SubOp 4) | Có `lit_7A2094/7A20A8.hex` nhưng **dump dở (68B vs len 195/251, lẫn opcode) → không giải mã chắc chắn, giữ nguyên giới hạn cũ** |
| 6 | `DAT_007a3848 / LAB_007a3864 / DAT_007a3884` | SubOp 0x0A (bản sao toast 43/45/30 theo kind) | Constant vùng code, **không có dump → chưa dịch được** |
| 7 | `DAT_007a3ce0 / d04 / d2c / d6c / dc8 / e18 / e8c` | SubOp 0x0D (toast mã 1–5, 1200/2000ms, có điều kiện mapId 11007 / 12937–12950) | Constant vùng code, **không có dump → chưa dịch được** |
| 8 | `0x63ABF8 / 63AA8C / 63AAB8` (SubOp 1), `0x63B254 / 63B104 / 63B130` (SubOp 9) | text + callback dialog `FUN_0063c674` | Constant vùng code, **không có dump** |

Ghi chú bảng mã (đính chính 2026-09-14): pipeline `cp1258 → NFC` cho kết quả vô nghĩa trên chính các bytes này; bảng đúng là **VISCII đơn-byte tiền tổ hợp** (RFC 1456 — `iconv -f VISCII` decode sạch toàn bộ, khớp `opcode_09.md §7.1`). Mọi chuỗi trong bảng trên là **văn nguyên game đã chốt**, không còn suy đọc; chính tả game không chuẩn ("cập nhập", "trùng lập" kiểu `opcode_09`) được giữ nguyên.

Không suy đoán nội dung tiếng Việt khi chưa có bytes (tránh bịa đặt). Cần dump Delphi ansistring tại các địa chỉ trên từ binary gốc rồi giải mã theo **VISCII** (RFC 1456 — `iconv -f VISCII`), không dùng cp1258 (đã bác bỏ, xem `opcode_09.md §7.1`).

---

## 7. Ghi chú cho Mock Server

1. **Phạm vi:** OP 0x0D là **đồng bộ team/party + toast hệ thống**, không phải chat (chat là 0x02), không phải di chuyển/battle.
2. **Frame:** `F4 44 | Len:Word LE | payload`, toàn bộ bytes socket = plain XOR `0xAD` từng byte.
3. **Bảng frame tính sẵn:**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 1 (id=1) — (đính chính: cần 6 byte, `0D 01` cụt sẽ BoundErr trong callee) | `0D 01 01 00 00 00` | `F4 44 06 00 0D 01 01 00 00 00` | `59 E9 AB AD A0 AC AC AD AD AD` |
| SubOp 2 flag=1 (có toast) | `0D 02 01` | `F4 44 03 00 0D 02 01` | `59 E9 AE AD A0 AF AC` |
| SubOp 3 ví dụ kind=1 id=0x12345678 | `0D 03 01 78 56 34 12` | `F4 44 07 00 0D 03 01 78 56 34 12` | `59 E9 AA AD A0 AE AC D5 FB 99 BF` |
| SubOp 4 id=1 | `0D 04 01 00 00 00` | `F4 44 06 00 0D 04 01 00 00 00` | `59 E9 AB AD A0 A9 AC AD AD AD` |
| SubOp 7 id=2 | `0D 07 02 00 00 00` | `F4 44 06 00 0D 07 02 00 00 00` | `59 E9 AB AD A0 AA AF AD AD AD` |
| SubOp 8 min | `0D 08` | `F4 44 02 00 0D 08` | `59 E9 AF AD A0 A5` |

4. **Thứ tự test an toàn:** `0D 08` (clear + refresh) → `0D 02 00` (no-op vì flag≠1) → `0D 02 01` (toast) → `0D 04/07` với id tồn tại → `0D 0C <id>` / `0D 0D <mã 1..5>` (nay đã rõ field, chỉ ghi cờ `+0x5d8` / toast) → cuối cùng mới thử `0D 06` (parser team phức tạp) và các pass-through `0x01/0x09/0x0A/0x0B` (mở dialog / nối id mảng).
5. **Lưu ý SubOp 3:** chỉ có hiệu ứng khi `id == *(gvar_007D9D64+4)` (id self). Gửi id lạ sẽ bị bỏ qua. Muốn thấy toast phải sniff id self từ gói khác.
6. **Không gửi SubOp `0x00` / `≥0x0E`** — không có nhánh, rơi qua switch (vô hại nhưng vô nghĩa).
7. **Không cần mock C→S cho 0x0D** (builder rỗng). Chỉ lắng nghe S→C.

---

## 8. Source trail (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_013_0078DBAE_FUN_0078dbae.c` | @`0x0078DBAE`, toàn file 187 dòng | Handler chính, danh sách SubOp, wire field |
| 2 | `ts_decompile/case_functions/manifest.csv` | dòng 15 | Case index 13, entry `0x0078A9EA` → `0x0078DBAE` |
| 3 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | Case 13 | Xác nhận mapping |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `switch(local_9)`; `case 0xd:` d.2399–2534 | Dispatcher MainOp, đối chiếu inline 1:1 |
| 5 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0xd: break;` | Kết luận C→S rỗng |
| 6 | `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c` | codec | DWORD LE duy nhất handler dùng |
| 7 | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` | codec | Word LE — xác minh **không dùng** ở OP này |
| 8 | `ts_decompile/functions/0077eb1c_FUN_0077eb1c.c`, `0077ee84_FUN_0077ee84.c`, `0077f098_FUN_0077f098.c` | codec | Xác minh không dùng ở S→C 0x0D |
| 9 | `ts_decompile/functions/007a1964_FUN_007a1964.c` | — | Core SubOp 4 (rời/kick/disband) |
| 10 | `ts_decompile/functions/007a273c_FUN_007a273c.c` | — | Parser team SubOp 6 |
| 11 | `ts_decompile/functions/0058e944_FUN_0058e944.c`, `005952f4_FUN_005952f4.c`, `005a3018_FUN_005a3018.c` | — | UI refresh sau SubOp 4/7/8 (tóm 1 dòng) |
| 12 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex` | byte `0D→0D`, dword `AE DB 78 00` | Xác minh mapping MainOp→Case |
| 13 | `ts_decompile/redump/lit_796f50.hex`, `lit_796f68.hex`, `lit_796f84.hex`, `lit_796fa4.hex` | **dump mới 2026-09-14** | Chuỗi toast SubOp 2/3 — decode VISCII, đã chốt (§4.2/4.3/6) |
| 14 | `ts_decompile/redump/lit_7A2094.hex`, `lit_7A20A8.hex` | dump dở (giữ nguyên) | 68B vs len 195/251, lẫn opcode → **không giải mã chắc chắn, không bịa — vẫn là khoảng trống** |
| 15 | `ts_decompile/functions/0063aaf4_FUN_0063aaf4.c`, `0063b16c_FUN_0063b16c.c`, `007a363c_FUN_007a363c.c`, `007a3894_FUN_007a3894.c`, `007a3aa0_FUN_007a3aa0.c`, `007a3b5c_FUN_007a3b5c.c` | toàn file | **Body mới 2026-09-14** — SubOp 1/9/0x0A/0x0B/0x0C/0x0D |
| 16 | `ts_decompile/functions/0071fa00_FUN_0071fa00.c:21`, `0071dc60_FUN_0071dc60.c:21`, `0063c674_FUN_0063c674.c:46–52`, `0063c1d0_FUN_0063c1d0.c` | lá | Cờ `+0x5d8`, khuôn dialog/callback |
| 17 | Khoảng trống còn lại: `007a20b8` (SubOp 5) **vẫn chưa có body**; text dialog SubOp 1/9 + toast SubOp 0x0A/0x0D (vùng code, chưa dump) | — | Không suy diễn |

*Ghi chú trung thực: mọi kết luận nghiệp vụ team/toast đều suy trực tiếp từ branch + call trong source trên, kể cả các body mới (2026-09-14). Phần hiển thị (VMT `+0x90`, `FUN_005a3018`, `FUN_005952f4`) chỉ tóm 1 dòng theo yêu cầu. Chuỗi `0x796fxx` đã có dump nhưng bảng glyph của game là đơn-byte custom → bản dịch tiếng Việt hiển thị là "phỏng đoán dựng dấu" như tiền lệ `opcode_02`, đã đánh dấu từng từ chắc/chưa chắc. `007a20b8` và `lit_7A2094/7A20A8` vẫn là khoảng trống.*
