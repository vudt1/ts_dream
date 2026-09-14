# PHÂN TÍCH — Main OP 0x03 (Case 4 · `FUN_0078bc95` @ `0x0078BC95`) — aLogin.exe

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (case function + các helper được gọi + chiều C→S `FUN_0077f414`).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

> **Đính chính quan trọng ngay từ đầu:** Khác với Main OP `0x01` (Case 2, `FUN_0078b149` — có `switch(SubOp)` rõ ràng), **Main OP `0x03` KHÔNG có SubOp và KHÔNG có nhánh switch nào**. Handler `FUN_0078bc95` là một **packet đơn, cấu trúc cố định + biến độ dài**, phân nhánh bằng một phép so sánh `if/else` duy nhất:
> ```c
> _LStrCopy(RestPayload, 1, 4, &tmp);
> charID = FUN_0077ef7c(...);                          // 4 byte đầu = mã định danh nhân vật
> if (*(int *)gvar_007DA7BC + 4 == charID) { ... }     // NHÁNH SELF  (chính tôi)
> else { ... }                                          // NHÁNH OTHER (nhân vật khác xuất hiện)
> ```
> Byte `ECX[0]` (= `payload[1]`) **không phải SubOp** — nó là **byte thấp (LSB) của trường charID 4-byte Little-Endian**. Vì vậy yêu cầu "xác định SubOp = ECX[0] và toàn bộ nhánh switch" được trả lời là: *không tồn tại SubOp/switch ở OP này*; "nhánh" thực sự là **SELF vs OTHER**, quyết định bởi `charID`.

---

## 1. Tóm tắt nghiệp vụ

Main OP `0x03` là **bản ghi "nhân vật đi vào/khởi tạo lại cảnh" (Actor Enter-Scene / Spawn Appearance Record)**. Server chủ động đẩy gói này mỗi khi một thực thể (chính máy khách **hoặc** một người chơi/NPC khác) cần được tạo mới / đồng bộ toàn diện trên màn hình hiện thời, ví dụ: ngay sau khi đăng nhập vào thế giới, sau khi dịch chuyển (teleport/đổi map), hoặc khi một actor bước vào tầm nhìn.

Một gói `0x03` mang **toàn bộ hồ sơ ngoại hình & trạng thái** của một nhân vật, đủ để client dựng actor mà không cần hỏi thêm:

| Nhóm dữ liệu | Nội dung theo evidence |
| :--- | :--- |
| Định danh | `charID` (DWORD LE) |
| Vị trí | `posX`, `posY` (Word LE) → `FUN_0071e2b8` |
| Mã bản đồ/khu vực | Word tại offset đối tượng `+0x63a` → quyết định blend/animation qua `FUN_0070d86c`, phân loại map qua `FUN_00634f3c` và 3 hàm classifier **đã có body** `func_0x0075c184` / `func_0x0054c29c` / `func_0x0054a384` (xem 4.1 bước 3) |
| Ngoại hình (look) | 2 DWORD "mã nén" → `FUN_00745bc0` (mode 1 & 5) giải thành mảng byte tại `obj+0x9b` (~45 byte) |
| Trang bị | `N` cặp Word = item code → tra CSDL vật phẩm `gvar_007DA540` (`FUN_00774af8` = ô thiết bị 0..6, `FUN_0077499c` = sprite), dựng 7 slot trang bị tại `obj+0x2c` |
| Tên hiển thị | Chuỗi ANSI biến độ dài ở phần đuôi → `obj+9` (cắt 0x11 = 17 byte), có tiền tố đặc biệt `DAT_00796EB4` khi đang ở màn hình login |
| Cờ trạng thái | Nhiều byte rời (`+0x08, +0x7a, +0x7b, +0x7c, +0x462, +0x464, +0x465, +0x4b0, +0x4b1`…) |

**Phân nhánh SELF vs OTHER:**
- **SELF** (`charID` khớp ID của chính máy khách, lưu tại `gvar_007DA7BC + 4`): ghi trực tiếp hồ sơ vào object người chơi cục bộ, dựng giao diện trạng thái, dựng 7 ô trang bị, đặt tọa độ, cập nhật camera/scene.
- **OTHER** (`charID` khác): gọi `FUN_0072174c` để **parse & cache** hồ sơ vào mảng cache 2100 slot (`gvar_007DA6BC`, đối tượng `TWorldPlayer`), sau đó **tìm slot actor trống → `TPlayers_Create`**, tăng bộ đếm actor (`gvar_007D9D34 + 0x5c`), nạp hồ sơ từ cache (`FUN_00722950`), đặt vị trí, vẽ lại (`FUN_0072a054`). Có **cổng lọc theo bản đồ**: chỉ spawn nếu mã map của record khớp map hiện tại của client.

**Kết luận bằng chứng:** Không có hằng số dạng chuỗi "label" mô tả opcode; nhưng toàn bộ dấu vết (tạo `TPlayers`, cache `TWorldPlayer`, `FUN_0071e2b8` đặt tọa độ, `FUN_0071fae0`/`FUN_00778510` camera, `FUN_00745bc0` giải mã ngoại hình, tra CSDL item `gvar_007DA540`, tên tại `obj+9`, tiền tố `0x00796EB4`) đều chỉ về một nghiệp vụ duy nhất: **đồng bộ "nhân vật xuất hiện vào cảnh" đầy đủ (spawn/enter-world appearance sync)**.

---

## 2. Entry & cách đọc PacketBuffer

- **Entry S→C:** `FUN_0078bc95` @ `0x0078BC95` (Case index 4, jump-table entry `0x0078A9C6`).
- **Định tuyến:** `MainOp = 0x03` → bảng byte `0x78A8EE` cho index `4` → bảng dword `0x78A9B6` → `0x0078BC95`.
- **Đầu vào:** `ECX = RestPayload` (Delphi `AnsiString`, đã trừ byte MainOp). Quy ước chỉ số trong bài:
  - `p[k]` = byte thứ `k` của **payload gốc** (0-based, `p[0] = MainOp = 0x03`).
  - Vì `RestPayload = payload[1..]`, vị trí 1-based của Delphi trên `ECX` trùng đúng `p[q]`: `_LStrCopy(ECX, start, n)` → `p[start .. start+n-1]`; đọc trực tiếp `*(ECX + off)` → `p[off+1]`.
- **Codec primitives:**
  - `_LStrCopy(s, start, n, &out)` — cắt `n` byte (Delphi 1-based).
  - `FUN_0077eb9c` → **Word Little-Endian** (`b0 + b1*256`).
  - `FUN_0077ef7c` → **DWORD Little-Endian** (`b0 + b1*256 + b2*65536 + b3*16777216`).
  - `_BoundErr` chỉ là kiểm tra biên chuỗi Delphi (không phải nghiệp vụ).

**Block khởi tạo (dòng 66–70, ngoài 2 nhánh):** nếu cờ `*(gvar_007DA37C + 5) == 0` thì copy `gself + 0x640` (playerID lưu lúc login) vào `gself + 4` (trường charID của actor) rồi set cờ = 1. Đây là thao tác "bơm ID nhân vật vào object actor lần đầu", đảm bảo phép so sánh `charID` bên dưới hoạt động.

---

## 3. Bảng tổng hợp "SubOp" (thực chất là bảng phân nhánh)

| Điều kiện | Ý nghĩa | Hàm xử lý | Layout |
| :--- | :--- | :--- | :--- |
| `gself+4 == charID` | **SELF** — hồ sơ của chính máy khách | thân `if` (dòng 74–397) | **Bảng A** |
| `gself+4 != charID` | **OTHER** — actor khác vào cảnh | thân `else` (dòng 398–505) + `FUN_0072174c` | **Bảng B** |

Không có `default`, không có SubOp thứ hai. Toàn bộ payload được hiểu theo một trong hai layout dưới đây tùy nhánh.

---

## 4. Chi tiết từng nhánh (wire layout + logic)

### 4.1. NHÁNH SELF — Bảng A (`FUN_0078bc95` dòng 74–397)

**Wire layout (chỉ số `p` payload gốc, 0-based):**

| Offset p | Size | Kiểu/Đọc bằng | Gán vào / Hiệu ứng |
| :--- | :--- | :--- | :--- |
| `p[0]` | 1 | MainOp | `0x03` |
| `p[1..4]` | 4 | `FUN_0077ef7c` | `charID`; so `gself+4` (điều kiện SELF) |
| `p[5]` | 1 | `*(ECX+4)` | `gself+0x08` (cờ trạng thái A); mirror cache rec `+0x1c` |
| `p[6]` | 1 | `*(ECX+5)` | `gself+0x448` (cờ) |
| `p[7]` | 1 | `*(ECX+6)` | `gself+0x455` (cờ) |
| `p[8..9]` | 2 | `_LStrCopy(8,2)`+`FUN_0077eb9c` | `gself+0x63a` = **MapID/khu vực (Word)** |
| `p[10..11]` | 2 | `_LStrCopy(10,2)`+`FUN_0077eb9c` | `posX` (biến tạm `-0x4c`) |
| `p[12..13]` | 2 | `_LStrCopy(12,2)`+`FUN_0077eb9c` | `posY` (biến tạm `-0x50`) |
| `p[14]` | 1 | `*(ECX+13)` | `gself+0x7a` (cờ) |
| `p[15]` | 1 | `*(ECX+14)` | `gself+0x7b` (cờ) |
| `p[16]` | 1 | `*(ECX+15)` | `gself+0x7c` (Word←byte) |
| `p[17..20]` | 4 | `_LStrCopy(17,4)`+`FUN_0077ef7c` | `FUN_00745bc0(gself+0x9b, code1, mask=-1, mode=1)` |
| `p[21..24]` | 4 | `_LStrCopy(21,4)`+`FUN_0077ef7c` | `FUN_00745bc0(gself+0x9b, code2, mask=-1, mode=5)` |
| `p[25]` | 1 | `*(ECX+24)` | `N` = số cặp item trang bị |
| `p[26 + 2*i]` | 2·N | `_LStrCopy(i*2+26,2)`+`FUN_0077eb9c` (i=0..N-1) | item code → dựng 7 ô trang bị (xem dưới) |
| `p[2N+26..27]` | 2 | `_LStrCopy(N*2+26,2)`+`FUN_0077eb9c` | `gself+0x462` (Word) |
| `p[2N+28]` | 1 | `*(ECX + (2N+27))` | `gself+0x464` (cờ) |
| `p[2N+29]` | 1 | `*(ECX + (2N+28))` | `gself+0x465` (cờ) |
| `p[2N+30]` | 1 | `*(ECX + (2N+29))` | *(chỉ khi đang ở màn hình login)* tiền tố tên = `DAT_00796EB4` + `IntToStr(byte)` → `gself+9` |
| `p[2N+31]` | 1 | `*(ECX + (2N+30))` | `gself+0x4b0` (cờ, điều khiển TCartNpc qua `FUN_0071e080`) |
| `p[2N+32]` | 1 | `*(ECX + (2N+31))` | `gself+0x4b1` (cờ, đi kèm `+0x4b0`) |
| `p[2N+33 .. end]` | biến | `_LStrCopy` + `_LStrCat` | **chuỗi tên hiển thị** nối vào `gself+9`, ép kiểu P-string cắt `0x11` (17) byte qua `_PStrNCpy` |

**Core logic SELF:**
1. Set `gself+0x78 = 1` (cờ "đã có hồ sơ").
2. Ghi các byte cờ, MapID (`+0x63a`), tọa độ.
3. `gself+0x1437 = FUN_00634f3c(itemDB0x98BE0E_tbl, MapID, 3)`; các cờ phái sinh `+0x1458/+0x1459/+0x145c` từ ba **hàm phân loại MapID đã có body** (gọi tại `case_004...c:103/107/112`, đọc MapID Word từ `gself+0x63a`; **param_1 của cả ba hàm không được body sử dụng — artifact EAX**):
   - `func_0x0075c184` → `gself+0x1458`: **kiểm tra dải hằng số thuần** — trả `2` nếu MapID ∈ `[0xD50D..0xD515]`, trả `1` nếu MapID ∈ `[0xD517..0xD51A] ∪ [0xD51C..0xD51F]`, ngược lại `0` (`0075c184_FUN_0075c184.c:25-34`). Đây là 3 dải MapID liền mạch (9+4+4 = 17 giá trị) quanh `0xD50D..0xD51F`, đánh dấu map "đặc biệt" kiểu hardcode.
   - `func_0x0054c29c` → `gself+0x1459`: **tra bảng tĩnh tại `DAT_009490B0`** (3 record, stride 0xAA byte): MapID khớp word tại `record+0` → trả `1`; khớp một trong 10 word tại `record+0x06+0x06·k` → trả `2`; không khớp → `0` (`0054c29c_FUN_0054c29c.c:34-78`).
   - `func_0x0054a384` → `gself+0x145c`: **tra bảng tĩnh tại `DAT_00948DF8`** (5 dòng, stride 0x36 byte): khớp word `+0x07` → trả `1`; khớp word `+0x0D+0x06·k` (k=1..5) → trả `2`; khớp word `+0x31+0x06` → trả `3`; không → `0` (`0054a384_FUN_0054a384.c:44-152`).
   - **Xác minh được từ body mới:** nhãn "kiểu bản đồ/scene đặc biệt" đúng về cơ chế (phân lớp MapID qua bảng/dải hằng, trả 0..3). **Giới hạn còn lại:** nội dung hai bảng `DAT_00948DF8`/`DAT_009490B0` là vùng `.data` **chưa được dump** trong `ts_decompile/redump/` → chưa liệt kê được MapID nào thuộc lớp nào (chỉ dải `0x75c184` là đọc được từ mã).
4. `FUN_0052abe4()`; `FUN_006221e8(gvar_007DA34C, MapID)` — cập nhật UI/panel theo map *(hiển thị, 1 dòng)*.
5. Mirror một phần hồ sơ sang cache record: `*(gvar_007DA6BC)+0x1c/+0x3c/+0x3d/+0x3e` từ `gself+0x08/0x7a/0x7b/0x7c` để các lần spawn actor khác dùng lại.
6. `FUN_004c9bf0(charID)` trả **nhóm/map loại**; nếu thuộc nhóm `[5..8]` (`(byte)(r-5)<4`) thì buộc `gself+0x08 = 0` và `gself+0x7c = 0x78` (120) — *ép trạng thái/anim cho một số loại map*.
7. Giải mã ngoại hình: 2 DWORD → `FUN_00745bc0` sang mảng byte `gself+0x9b..0xc8`; sao `0xb3/0xb4/0xb5 → 0xc5/0xc6/0xc7`; copy 0x2d byte từ `gself+0x9b` vào cache `+0x42`.
8. **Vòng lặp trang bị (i = 0..N-1):**
   - `itemCode = FUN_0077eb9c(_LStrCopy(ECX, i*2+26, 2))`.
   - `slot = FUN_00774af8(gvar_007DA540, itemCode)` (0..6 — **kiểu ô thiết bị**, tra CSDL item).
   - `slotObj = *(gself + 0x2c + slot*4)`; `slotObj+4 = itemCode`; `slotObj+0x10 = slot`; `slotObj+0x14 = FUN_0077499c(gvar_007DA540, itemCode)` (sprite/graphics id).
   → Dựng diện mạo 7 vị trí trang bị. *(hiển thị)*.
9. Các byte cờ đuôi + chuỗi tên (dòng 254–383), gồm tiền tố `DAT_00796EB4` ở màn hình login.
10. **Kết thúc nghiệp vụ:** `FUN_0071e2b8(gself, posX, posY)` đặt tọa độ thế giới; `FUN_0070d86c(gself)` chọn chế độ blend/anim theo MapID *(đồ họa)*; `func_0x00509e84(gvar_007DA37C)` — **đã có body, xác minh được từ body mới**: đây là routine **"tái khởi tạo theo map"** (không phải hàm phụ trợ vô danh): đọc MapID từ `+0x63a` của player (`DAT_0092530c` = alias object của `gself`) và dựng đường dẫn tài nguyên `"data\" + IntToStr(MapID) + …` để nạp qua `FUN_00777700/FUN_00777f88(DAT_00923228, …)` (`00509e84_FUN_00509e84.c:71-80` — literal `"data\\"` tại d.72); reset 3 global (`gvar_007DA6FC/007DA780/007DA394 = 0`, d.84-86), clear byte `+6` của object session `gvar_007DA37C` + gọi `FUN_0051ae38(obj,1)` (d.95-96), stamp `DAT_00923214+0x484 = GetTickCount()` (d.97-98), reset form `gvar_007DA688` (d.99), **clear 50 DWORD tại scene `DAT_0092322c+0x540c[1..50]`** (d.101-109), **phát C→S `SendCommand(MainOp=0x25, CL=1)`** (d.110 + asm `00509e84_FUN_00509e84.asm.txt:127-130` `MOV CL,0x1; MOV DL,0x25`), tạo `TBKSimServer` mới vào `gvar_007DA274` (d.111-112), ghi 5 bản sao vị trí `+0x1c/+0x20 → +0x590/+0x594[k*8], k=0..4` (d.114-126), FillChar hai buffer `gvar_007DA488/007DA484` (d.127-128), reset ~9 form/panel UI khác (d.129-141, gồm `gvar_007DA4A8` — object của state-machine OP 0x09 SubOp 3 — qua `FUN_005d43c0`), gọi VMT+8 với literal `"panel4"` (d.143-144); `FUN_00778510(gvar_007D9C28, posX, posY)` hiệu chỉnh camera/frame theo vị trí *(đồ họa)*; `FUN_0071e080(gself, +0x4b0, +0x4b1)` bật/tắt đối tượng "xe đẩy/NPC đồng hành" `TCartNpc` *(1 dòng)*; `FUN_0071fae0(gself, posX, posY)` tính lại neo render/animation theo camera *(đồ họa)*. Nếu `gself+0x378 == 0` (d.392) thì gọi bộ ba trên object `gvar_007DA24C` (creator không có trong export): `FUN_005c2420` — **dựng lại mảng lưới ô nhìn**: count = `(cam+0x1c/600)*(cam+0x20/600)` ghi `+0x108`, FinalizeDynArray tại `+0x104`, `+0x10e = 0xb` (`005c2420_FUN_005c2420.c:51-65` — `gvar_007D9C28` = camera/view object); `func_0x005c2658(obj, 0x20)` — setter thuần `obj+0x10d = 0x20` (`005c2658_FUN_005c2658.c` d.21); `func_0x005c280c(obj, 3)` — với mỗi phần tử của `count-1` phần tử đầu mảng `obj+0x104` (bản ghi stride 24B, record size 0x17 từ hàm 5c2420), ghi `elem+0x14 = 3 − FUN_00402cf0(3)`, tức **giá trị ngẫu nhiên 1..3 bằng LCG PRNG `FUN_00402cf0`** (đã định danh nonce-PRNG ở `opcode_06.md` §5.1) (`005c280c_FUN_005c280c.c:29-51`). *Ý nghĩa nghiệp vụ của cờ `gself+0x378` và lưới ô này: **chưa kết luận được** (mô tả cơ chế nguyên văn; giả đoán "hiệu ứng/mưa-tuyết theo vùng" không có bằng chứng chuỗi hay call nào chống lưng).*

---

### 4.2. NHÁNH OTHER — Bảng B (`FUN_0078bc95` dòng 398–505 + `FUN_0072174c`)

Khác biệt chính so với SELF: **có thêm 2 byte cờ** ở đầu và **MapID nằm ở `p[10..11]`**, do đó mọi field sau bị **dịch +2 byte** so với Bảng A. Nhánh này gồm 2 phần: (a) `FUN_0072174c` parse & cache toàn bộ hồ sơ, (b) `case_004` dùng tọa độ đã đọc để spawn actor thật.

**(a) Layout cache (đọc trong `FUN_0072174c`):**

| Offset p | Size | Kiểu | Gán vào cache record `TWorldPlayer` |
| :--- | :--- | :--- | :--- |
| `p[1..4]` | 4 | DWORD | `charID` → cấp slot qua `FUN_00722464`, tạo `TWorldPlayer_Create` |
| `p[5]` | 1 | byte | rec `+0x1c` |
| `p[6]` | 1 | byte | rec `+0x1d` |
| `p[7]` | 1 | byte | rec `+0x1e` |
| `p[8]` | 1 | byte | rec `+0x36` |
| `p[9]` | 1 | byte | rec `+0x37` |
| `p[10..11]` | 2 | Word | rec `+0x1a` = **MapID** (dùng cho cổng lọc ở (b)) |
| `p[16]` | 1 | byte | rec `+0x3c` |
| `p[17]` | 1 | byte | rec `+0x3d` |
| `p[18]` | 1 | byte | rec `+0x3e` (Word←byte) |
| `p[19..22]` | 4 | DWORD | `FUN_00745bc0(rec+0x42, code1, mask=-1, mode=1)` |
| `p[23..26]` | 4 | DWORD | `FUN_00745bc0(rec+0x42, code2, mask=0xFF, mode=5)` |
| `p[27]` | 1 | byte | `N` = số cặp trang bị |
| `p[28 + 2*i]` | 2·N | Word | item code → 7 ô trang bị cache `rec+0x70` (reset trước = 0/-1) |
| `p[2N+28]` | 2 | Word | rec `+0x38` |
| `p[2N+30..33]` | 1×4 | byte | rec `+0x3a, +0x8c, +0x3b, +0x8d` |
| `p[2N+34..35]` | 1×2 | byte | rec `+0x8e, +0x8f` |
| `p[2N+36 .. end]` | biến | chuỗi | **tên** → rec `+8` (cắt 0x11); nếu login thì `DAT_00796EB4 + IntToStr(rec+0x8d)` |

**(b) Logic spawn trong `case_004` else (dòng 399–505):**
1. `FUN_0072174c(gvar_007D9C48, RestPayload)` — cache hồ sơ vào `gvar_007DA6BC[]`.
2. `FUN_00760a88(charID)` — nếu người này **trong Party** (`DAT_009CE154`) → refresh party `FUN_0075eec0`.
3. `FUN_00764844(charID)` — nếu **trong Quân đoàn** và panel đang mở → refresh `FUN_0056b124`.
4. `FUN_00758318(gvar_007D9C20, charID)` — nếu **trong danh sách Hảo hữu** và panel mở → refresh `FUN_00570ef4`.
5. **Cổng lọc bản đồ:** `mapidRec = FUN_0077eb9c(_LStrCopy(ECX,10,2))`; **chỉ đi tiếp nếu `mapidRec == gself+0x63a`** (đúng map hiện tại).
6. `idx = func_0x0070c158(gvar_007D9D34, charID)` — **đã có body, xác minh được từ body mới**: hàm *không dùng param_1* (chỉ đọc thẳng global `gvar_007DA300`); quét `i = 1..800`, **nếu đã có actor với `actor+4 == charID` thì trả chính idx đó** (respawn cùng người = cập nhật tại chỗ, không cấp slot mới); nếu không có ai khớp thì trả **slot trống đầu tiên** (`gvar_007DA300[i] == 0`); mảng đầy → trả 0 (`0070c158_FUN_0070c158.c:31-63`, guard bound 800 tại d.33/38/54). Nếu `idx != 0`:
   - nếu slot `gvar_007DA300[idx]` trống → `TPlayers_Create(VMT_70B5D0, 1, idx)` gắn vào mảng actor.
   - tăng số actor: `*(gvar_007D9D34 + 0x5c)++` — **tăng vô điều kiện** (kể cả khi `idx` trỏ actor đã tồn tại từ trước, theo body mới của `0070c158`; chỉ `TPlayers_Create` là có guard `slot == 0`).
   - `FUN_00722950(gvar_007D9C48, charID, actorObj)` — **nạp hồ sơ cache → actor** (khuôn mẫu y hệt Bảng A: MapID→`+0x63a`, appearance `+0x9b`, 7 slot trang bị `+0x2c`, tên `+9`, các cờ `+0x462/0x464/0x465/0x4b0/0x4b1`…).
   - `posX=p[12..13]`, `posY=p[14..15]` → `FUN_0071e2b8(actorObj, posX, posY)`; `FUN_0070d86c` *(blend/anim)*; `FUN_0071e080(actorObj,+0x4b0,+0x4b1)` *(TCartNpc)*; `FUN_0071fae0` *(neo render)*.
   - `FUN_004c9bf0(charID)`; nếu thuộc nhóm `[5..8]` → `actorObj+8 = 0`, `actorObj+0x7c = 0x78`.
   - `FUN_0072a054(gvar_007D9D34)` — tính lại chỉ số actor đỉnh (`+0x60`) để vòng render. *(đồ họa)*

> Ghi chú: `FUN_0071e2b8/FUN_0071fae0/FUN_0070d86c/FUN_00778510/FUN_0072a054/FUN_006221e8/FUN_0052abe4` đều thuộc tầng **camera/render/UI**, được tóm 1 dòng; logic đồng bộ dữ liệu cốt lõi nằm ở **charID, tọa độ, MapID, mảng ngoại hình `+0x9b`, 7 ô trang bị `+0x2c`, và tên `+9`**.

---

## 5. Chiều C→S liên quan (`FUN_0077f414` — `TFConnect.SendCommand`)

Trong `ts_decompile/functions/0077f414_FUN_0077F414.c` có `case 3:` của `switch(param_2 & 0xff)` (dòng 821–827):
```c
case 3:
  FUN_00402b90(&buf1, &buf2);
  _PStrNCat(&buf1, &lit2byte, 2);      // ghép 2 byte (opcode 0x03 + 1 byte tham số)
  _LStrFromString(&out, &buf1);
  TForm1_CY_AddSedQueue(gvar_007DA664, out[0]);   // đẩy vào hàng đợi gửi
```
- Đây là một **lệnh gửi tối giản 2 byte**: asm của builder `case 3` chứng minh **byte 2 = `gself+0x63c`** (một byte, ngay sau MapID Word `+0x63a`), không phải hằng: `MOV DL,[EBP-5]` → `[0x03]`, rồi `MOV DL,[gself+0x63c]` → byte thứ hai, `_PStrNCat(...,2)` (`0077f414_FUN_0077F414.asm.txt:183-205`).
- **(đính chính — call-site ĐÃ tìm thấy, 2026-09-14)** Trước đây ghi "không tìm thấy call-site `SendCommand(3)`". Nay đã có: `FUN_00713308` — handler **SubOp 1 của Main OP 0x09** — chính là call-site: gate `gvar_007DA37C+5 != 0 && gvar_007DA37C+4 == 0` → ghi `gself+0x63c = 1` → `XOR ECX,ECX; MOV DL,0x3; CALL 0x0077f414` = `SendCommand(MainOp=0x03, SubSel=0)` ⇒ client **tự đáp C→S `[03][01]`** sau khi nhận `09 01` (`00713308_FUN_00713308.c:19-23`, asm d.20-25). Không còn là "legacy không rõ nguồn"; độ tin cậy nâng từ Thấp lên **Cao** cho đường gửi này.
- **Kết luận cho Mock Server:** chiều gửi `0x03` vẫn **không bắt buộc** để dựng luồng vào cảnh (client chỉ phát nó khi server ra lệnh `09 01`, xem `opcode_09.md` §4.1); server chỉ cần **phát** gói S→C `0x03` là client tự spawn.

---

## 6. Ghi chú cho Mock Server

1. **Định dạng frame:** giữ nguyên `[Token F4 44 (2B)] [Length L (Word LE)] [Payload L B]`, toàn khung XOR khóa tĩnh `0xAD`. `L` = độ dài payload (tính từ `p[0]=0x03`).
2. **Không có SubOp:** đừng chèn byte SubOp. `payload[1..4]` phải là **charID DWORD LE**. Client quyết định SELF/OTHER bằng cách so `charID` với ID của chính nó (`gself+4`, được bơm từ `gself+0x640` ở block khởi tạo).
3. **Gửi cho chính người chơi (SELF):** dùng **Bảng A**. Lưu ý byte cờ `+0x08` và các cờ rời phải hợp lệ vì một số loại map (`FUN_004c9bf0 ∈ [5..8]`) sẽ **ghi đè** `+0x08=0`, `+0x7c=120`.
4. **Gửi cho actor khác (OTHER):** dùng **Bảng B** — nhớ **dịch +2 byte** so với SELF vì có **5 byte cờ** trước MapID (`p[5..9]`) và MapID ở `p[10..11]`.
   - **Bắt buộc `p[10..11]` = MapID hiện tại** của người nhận, nếu không `case_004` **sẽ âm thầm không spawn** (cổng lọc `mapid == gself+0x63a`).
   - Cấp `idx != 0` (tức `charID` phải chưa vượt ngưỡng 800 actor; mảng `gvar_007DA300`).
5. **Trường ngoại hình:** 2 DWORD ở `p[17..20]`/`p[21..24]` (SELF) là **mã nén** mà `FUN_00745bc0` tách thành các chữ số 3-3-3 (`code/1e6`, `(code/1e3)%1e3`, `code%1e3`) ghi vào mảng byte `+0x9b`/`rec+0x42`. Nếu chỉ cần actor "trần", gửi `0`.
6. **Trang bị:** `N` = số cặp Word item; mỗi item code **phải tồn tại** trong CSDL item client (`gvar_007DA540`) để `FUN_00774af8` trả ô 0..6 hợp lệ; nếu không sẽ `BoundErr`/để trống slot. `N` sai (lớn) sẽ đọc lệch/vượt biên chuỗi → `_BoundErr` crash. Vì vậy **`N` và độ dài payload phải khớp chính xác**.
7. **Tên hiển thị:** chuỗi đuôi sau `p[2N+32]`/`p[2N+35]` được cắt còn 17 byte (`_PStrNCpy`, `_LStrToString(...,0xff)`). Ở **màn hình login** (`FUN_00504c9c`: map/scene %100 ∈ [90..99]) tên sẽ bị **tiền tố bằng chuỗi hằng `0x00796EB4`** — khi mock ở scene thường thì bỏ qua tiền tố.
8. **Điều kiện bật xử lý:** toàn bộ dispatcher chạy khi `*gvar_007DA3A0 != 0` (bên SendCommand) và gate pause `*(DAT_009264a4+0xC)==0`. Đảm bảo client đã ở trạng thái "đã vào game".
9. **Thứ tự khung cảnh:** server gửi OP 0x01 SubOp 0x09 (chuyển scene) → người dùng login → server gửi OP 0x03 (SELF) để dựng nhân vật → rồi các OP 0x03 (OTHER) cho người/NPC quanh đó.

---

## 7. Source trail (đã đọc/kiểm chứng)

- **Handoff kiến trúc & ánh xạ OP:** `.scratch/op-code/handoff-opcode-exploration-guide.md` (mapping `MainOp 0x03 → Case 4 → 0x0078BC95`; codec `_LStrCopy/FUN_0077eb9c/FUN_0077ef7c`).
- **Mẫu định dạng & framing/dispatcher:** `.scratch/op-code/opcode_00_01.md` (tầng PacketBuffer, `TFConnect`, `CY_DelRevQueue`, `FUN_0078a89c`).
- **Handler chính:** `ts_decompile/case_functions/functions/case_004_0078BC95_FUN_0078bc95.c` (SELF/else, vòng trang bị, tên, tiền tố `DAT_00796EB4`).
- **Parser hồ sơ OTHER/cache:** `ts_decompile/functions/0072174c_FUN_0072174c.c` (`TWorldPlayer_Create`, cache `gvar_007DA6BC` 2100 slot, `FUN_00722464`, `FUN_00745bc0`, `FUN_00774af8/6ac`).
- **Codec/Nạp cache→actor:** `00722950_FUN_00722950.c` (map rec→obj `+0x1c/0x36/0x3c/0x3e/0x42/0x70/0x38/0x3a/0x3b/0x8e/0x8f`, tên `+8`).
- **Hàm tiện ích:** `00745bc0_FUN_00745bc0.c` (giải mã ngoại hình), `0071e2b8_FUN_0071e2b8.c` (đặt tọa độ), `0070c158_FUN_0070c158.c` (mới — find-or-first-free slot actor 1..800) & `0070c20c_FUN_0070c20c.c` (tìm theo charID), `004c9bf0_FUN_004c9bf0.c` (phân nhóm ID/map), `00504c9c_FUN_00504c9c.c` (điều kiện scene login %100∈[90..99]), `00774af8_FUN_00774af8.c` (ô trang bị), `0077499c_FUN_0077499c.c` (sprite item), `00634f3c_FUN_00634f3c.c` (danh sách model/look đặc biệt).
- **Helper mới có body (cập nhật 2026-09-14):** `00509e84_FUN_00509e84.c` (re-init theo map + SendCommand 0x25), `0075c184_FUN_0075c184.c` / `0054c29c_FUN_0054c29c.c` / `0054a384_FUN_0054a384.c` (3 classifier MapID → `+0x1458/0x1459/0x145c`), `005c2420/005c2658/005c280c` (lưới ô nhìn + setter + randomize field), `0070c158_FUN_0070c158.c` (slot actor), `00713308_FUN_00713308.c` (call-site `SendCommand(3)`, thuộc OP 0x09).
- **Tầng render/camera/UI (tóm 1 dòng):** `0071fae0_FUN_0071fae0.c`, `0070d86c_FUN_0070d86c.c`, `0071e080_FUN_0071e080.c` (TCartNpc), `0072a054_FUN_0072a054.c`, `00778510_FUN_00778510.c`, `006221e8_FUN_006221e8.c`, `0052abe4_FUN_0052abe4.c`.
- **Chiều C→S:** `ts_decompile/functions/0077f414_FUN_0077F414.c` (`case 3:` dòng 821–827) + `.asm.txt` (bảng nhảy `0x77f53c`).
- **Bằng chứng toàn cục:** `gvar_007DA7BC` = con trỏ player cục bộ (trùng offset `+0x640/0x644/0x648` của `DAT_0092530c` mô tả trong `.scratch/op-code/login_flow_research.md`); `gvar_007DA6BC` = cache 2100 `TWorldPlayer`; `gvar_007D9D34`/`gvar_007DA300` = world-actor manager & mảng 800 actor; `gvar_007DA540` = CSDL vật phẩm.

**Hằng số đã định vị:** `DAT_00796EB4` = chuỗi tiền tố tên (chỉ hiển thị ở scene login) — *vẫn chưa giải được nội dung: các dump mới `lit_796a8c/lit_796ae8.hex` phủ tới `0x796CE7`, `lit_796ec0.hex` bắt đầu từ `0x796EC0`, còn đúng lỗ hổng `0x796CE8..0x796EBF` chưa dump; cần redump dải này để xác minh nội dung chính xác.*
