# PHÂN TÍCH — Main OP 0x0E (Case 14, `FUN_0078e01c` @ `0x0078E01C`) — **Sự kiện / thông báo hệ thống có tham số (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Tóm tắt nghiệp vụ (1 đoạn)

**OP 0x0E là kênh "sự kiện / thông báo hệ thống có tham số" dạng fan-out:** 1 byte SubOp chọn 1 trong 7 đường xử lý. Đọc được body cho thấy: SubOp 1 = lưu 1 record (id + chuỗi đuôi + số 8-byte + 1 byte cờ) vào bảng của `gvar_007D9CC8` rồi phát âm thanh `sound\m004.wav` + set cờ + refresh; SubOp 3 = resolve tên theo id rồi toast 2000ms, chọn tiền tố/hậu tố theo 1 byte kind (`0x01/0x02/còn lại`); SubOp 5 = parser danh sách nhiều record (id + tên + 2 DWORD + 1 byte) rồi refresh 2 form; SubOp 6 = toast trần 2000ms không tham số. **Trước đây 3 đường (2, 4, 7) forward cho hàm con chưa decompile; từ 2026-09-14 cả 7 đường + các callee `0075d5b8/0075d7ec/0075f9d8/0075ffe4/00747a20` đều đã có body** — hóa ra `gvar_007D9F1C` là **manager danh sách lời mời bạn hữu (friend invite)**: SubOp 2 = lưu id + hiển thị toast tương tác 10 giây có callback, SubOp 3 kind=1 thêm record mời vào bảng pending `DAT_009ce154`, SubOp 4 = xóa id khỏi bảng pending (kèm toast + refresh). Toàn bộ hiển thị/âm thanh chỉ tóm 1 dòng theo yêu cầu.

**Kết luận về SubOp:** handler **CÓ `switch(SubOp)`** với các label `1..7` (khác OP 0x0C không có SubOp). **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec.

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_014_0078E01C_FUN_0078e01c.c:8` — `void FUN_0078e01c(void)` @ `0x0078E01C`
- `ts_decompile/case_functions/manifest.csv:16` — `14,0x0078A9EE,0x0078E01C,EXPORTED,"FUN_0078e01c"`
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 14 (chỉ khung cleanup, body nằm ở file case riêng).

**Dispatcher:** `FUN_0078a89c` (`ts_decompile/functions/0078a89c_FUN_0078a89c.c`) `switch(local_9)` với `local_9` = MainOp; nhánh **`case 0xe:` tại dòng 2535–2639** chứa toàn bộ SubOp 1..7, khớp logic với file case riêng. Ngoài ra dispatcher còn có 1 switch nhãn log ở dòng 634–636: `case 0xe: @LStrLAsg(...,0x796618)` — chỉ là chuỗi tên debug cho MainOp, không phải logic (địa chỉ `0x796618` **không có dump trong `redump/`** → chưa dịch được).

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD`. Server speaks first.

**Cách đọc SubOp (file case dòng 33–40):**
```c
iVar5 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload (payload đã cắt MainOp)
if (*(int *)(iVar5 + -4) == 0) _BoundErr(0); // guard: RestLen >= 1
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar5 + 0); // SubOp = ECX[0]
switch(SubOp) { case 1 ... case 7 ... }      // không có case 0, không có default
```
- `unaff_EBP-0xc` = **RestPayload** = payload gốc bỏ byte `P[0]=MainOp`.
- **SubOp = `payload[1]` = `ECX[0]`**, 1 byte thường, không codec.
- `SubOp == 0` hoặc `>= 8` → rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm (no-op, không crash).

**Quy ước `_LStrCopy` 1-based (Delphi `Copy`):** `_LStrCopy(ECX,p,n)` = `ECX[p-1..p+n-2]` = `payload[p..p+n-1]`. Ví dụ `Copy(ECX,2,4)` = `payload[2..5]`.

**Codec helper (đã đọc body):**
| Hàm | File body | Kết luận |
|---|---|---|
| `FUN_0077ef7c` DWORD LE | `0077ef7c_FUN_0077ef7c.c:207–240` | `b0+b1*256+b2*65536+b3*2^24`, guard từng byte. Handler dùng **5 lần** (id các SubOp 1,2,3,4) + 2 lần trong `FUN_0076676c` |
| `FUN_0077eb9c` Word LE | `0077eb9c_FUN_0077eb9c.c:166–174` | `b0+b1*256`. **Handler 0x0E không gọi lần nào** (ghi rõ để tránh nhầm) |
| `FUN_0077eb1c` | encode Word→chuỗi 2 byte, chỉ dùng chiều C→S. Không dùng ở S→C này |
| `FUN_0077ee84` | encode DWORD→chuỗi 4 byte, chỉ dùng chiều C→S. Không dùng ở S→C này |
| `FUN_0077f098` | `_LStrToString + copy 8 byte` → decode **khối 8 byte thành 1 số double/int64**. Handler dùng **1 lần** (SubOp 1) |

---

## 2. Đối chiếu dispatcher inline

Khối `case 0xe:` trong `0078a89c_FUN_0078a89c.c:2535–2639` khớp logic 1:1 với file case riêng (cùng thứ tự `_LStrCopy` + `FUN_0077ef7c`/`FUN_0077f098`, cùng các call, cùng hằng `sound\m004.wav`, cùng `UNK_00797048`). Một khác biệt hiển thị duy nhất: ở SubOp 3, file case thể hiện call VMT toast với **7 đối số**, còn bản dispatcher inline thể hiện gọn **3 đối số** — cùng một call-site, chỉ khác cách decompiler rút gọn. Không ảnh hưởng wire layout.

---

## 3. Bảng tổng hợp toàn bộ SubOp

`switch(SubOp)` có các label: **`1,2,3,4,5,6,7`. Không có `case 0`, không có `default`.**

Quy ước: `P[.]` = payload gốc (`P[0]=0x0E`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu | Wire field | Handler tầng case |
|---|---|---|---|
| `0x01` | **15** (`0E 01` + 4 + 8 + 1 + đuôi rỗng) | `id:P2..P5 DWORD LE` + `blob8:P6..P13`→số + `flag:P14 1B` + `tail:P15..` chuỗi | `FUN_007605ec` + phát `sound\m004.wav` + `FUN_005952cc` + `FUN_0075d7ec` (có body mới) |
| `0x02` | **6** (`0E 02` + id) | `id:P2..P5 DWORD LE` (`Copy(RP,2,4)`) | `FUN_0075d5b8(gvar_007D9F1C, id)` — **có body mới** (toast 10000ms + callback) |
| `0x03` | **7** (`0E 03` + id + kind) | `id:P2..P5 DWORD LE` + `kind:P6=RP5 1B` (guard `Len<6→BoundErr(5)`) | `FUN_0075ddb8` resolve tên + `FUN_0075f9d8` (nếu kind=1, **có body mới**) + toast 2000ms theo kind (chuỗi **đã dump §6**) |
| `0x04` | **6** | `id:P2..P5 DWORD LE` | `FUN_0075ffe4(gvar_007D9F1C, id)` — **có body mới** (xóa record mời + toast) |
| `0x05` | **≥2** (`0E 05` + rest) | pass nguyên RP | `FUN_0076676c(gvar_007D9F1C, RP)` — parser danh sách bên trong (có body) |
| `0x06` | **2** (`0E 06`) | không field | toast `UNK_00797048` 2000ms — **đã dump §6** |
| `0x07` | **2** (case) — callee đòi **3** | `byte:RP1` | `FUN_00747a20(gvar_007D9D34, RP)` — **có body mới** |
| `0x00`, `≥0x08` | — | rơi qua switch | no-op (chỉ cleanup) |

Cách suy độ dài tối thiểu: guard `_BoundErr` trong từng nhánh. SubOp 1 cần `RP Len ≥ 14` → payload ≥15. SubOp 3 cần Len≥6. SubOp 2/4 cần Len≥5 (`Copy(RP,2,4)`).

---

## 4. Chi tiết từng SubOp (wire layout + logic core)

### 4.1. SubOp `0x01` — lưu record + âm thanh + refresh (phức tạp nhất OP)
- Wire: `[0E][01][id:P2..P5 DWORD LE][blob8:P6..P13 8B][flag:P14 1B][tail:P15.. chuỗi tới hết]`
- Đọc:
  - `_LStrCopy(ECX,2,4)` + `FUN_0077ef7c` → `id`.
  - `_LStrCopy(ECX,0xF,Len-10)` → chuỗi đuôi `tail` = `RP[14..]`.
  - `_LStrCopy(ECX,6,8)` + `FUN_0077f098` → số 8-byte (double).
  - byte trực tiếp `ECX[0x0D]` (= `P[14]`, guard `Len<0x0E→BoundErr(0x0D)`) → `flag`.
- Core:
  1. `FUN_007605ec(*(gvar_007D9CC8), id, tail, flag, dbl)` — **có body**: ghi `id` + `tail` (tối đa 0x97 chars) + `flag` + double vào 1 slot của bảng stride `0xA5`, tối đa 1000 slot; rồi refresh. Nếu bảng đầy (≥1000) → toast `DAT_00760854` 2000ms thay vì ghi.
  2. Nối `"sound\\m004.wav"` + `FUN_007a7f20(path)` — phát file âm thanh (kiểm tra `FileExists`, bỏ qua nếu không có file — thuần presentation, 1 dòng).
  3. `FUN_005952cc(*(gvar_007DA1DC))` — **có body**: `obj+0x171 := 1` (set 1 byte cờ).
  4. `FUN_0075d7ec(*(gvar_007DA09C))` — **có body mới** (`0075d7ec_FUN_0075d7ec.c:19`): `*(double*)(obj+0x130) := (double)*(gvar_007D9D20)` — copy giá trị global (đồng hồ/giờ hệ thống?) vào field `+0x130` dạng double của form. Không còn là khoảng trống.
  5. Chuỗi toast "bảng đầy" `DAT_00760854` (`007605ec` khi ≥1000 slot, 2000ms): **đã dump + decode** (`lit_760854.hex`, content `0x760854..0x76086E`, 27 byte): raw NFC `Tin tÑc đă đªn mÑc gi¾i hƠn` → **phỏng đoán "Tin tức đã đến mức giới hạn"** ✓ khớp ngữ cảnh bảng đầy (toàn bộ dump nằm trong vùng .data, không lẫn code).

### 4.2. SubOp `0x02` — `FUN_0075d5b8` (**CÓ BODY MỚI** — toast tương tác 10s + gắn callback manager mời)
- Wire: `[0E][02][id:P2..P5 DWORD LE]`
- Core (`0075d5b8_FUN_0075d5b8.c:42–56`): `*(obj+0x1f0) := id`; `FUN_0075ddb8(id,&name)` resolve tên; dựng chuỗi 3 phần (`_LStrCatN`) từ template + tên; gọi `FUN_0063c8fc(gvar_007DA084, msg, 10000, ...)` — tức **toast/confirm 10000ms** (khác toast thường ở VMT+0x90 2000ms). `FUN_0063c8fc` còn **cài callback** vào form: `+0x1D4=FUN_0075d738`, `+0x1D0=obj` (`0063c8fc_FUN_0063c8fc.c:42–47`) → hàm `0075d738` (không có body) chạy khi người dùng bấm. Template toast `&DAT_0075d66c` / `&LAB_00760870` (vùng code, **không dump → chưa dịch**). Đây chính là lời mời "bạn hữu" cần xác nhận (khớp nhóm string §6).

### 4.3. SubOp `0x03` — resolve tên + toast theo kind
- Wire: `[0E][03][id:P2..P5 DWORD LE][kind:P6=RP5 1B]` (guard `Len<6→BoundErr(5)`)
- Core:
  1. `*(gvar_007D9F1C+0x1F0) := id`.
  2. `FUN_0075ddb8(id, &name)` — **có body**: tra cache 2100 slot `gvar_007DA6BC` qua `FUN_00722508`; tìm thấy → `name := rec+8`, không thấy → chuỗi rỗng.
  3. Rẽ theo `kind` (byte thường):
     - `kind==0x01`: `func_0x0075f9d8(gvar_007D9F1C, id)` (**có body mới** — thêm record mời vào bảng pending `.data`: `if id==0x65(101)` bỏ qua (`0075f9d8_FUN_0075f9d8.c:64`); counter `DAT_009d0378++` (guard ≤0x100, `:65–73`); ghi vào bảng `DAT_009ce154` stride 34B (`*0x11*2`): `+0=id` (`:78`), từ record cache tên `gvar_007DA6BC[rec]`: `+5 := rec+0x3c`, `+6.. := 14 byte tên` (`:145` `_PStrNCpy ...,0xe`), `+0x15/0x16 := rec+0x1e/0x1d`, `+0x17/0x18 := rec+0x1c/0x3d`, `+0x19 (DWORD) := số thập phân 9 chữ số lắp từ 9 byte rec+0x42..0x4a (mỗi byte ×10^k, k=0..8)` (`:184–275`), `+0x1D (DWORD) := tương tự từ rec+0x4b..0x50,0x5a..0x5c` (`:280–371`), `+0x21 := rec+0x8e` (`:372–387`); cuối cùng `FUN_0075eec0(mgr)` refresh (form `+0x1fd` chọn `FUN_00763444/7633f0`). **Ý nghĩa 2 con số lớn (EXP? gold?) chưa kết luận được**) + toast 2000ms với tiền tố `UNK_00796fe8` + `name` + hậu tố `UNK_00796fd4`.
     - `kind==0x02`: toast 2000ms với `UNK_0079700c` + `name` + `UNK_00796fd4`.
     - còn lại: toast 2000ms với `UNK_00797030` + `name` + `UNK_00796fd4`.
     - **Cả 5 chuỗi trên đã dump + decode (VISCII, xem §6)** — bộ ba "Tiếp nhận lời mời bạn hữu / Cự tuyệt gia nhễp bạn hũu (nguyên văn typo game) / Không hồi ứng" + tiền tố "Bạn chơi" ⇒ SubOp 3 = **kết quả trả lời lời mời bạn hữu**.
  - Toast qua `(VMT gvar_007DA084+0x90)(...,2000,...)` — hiển thị thuần, 1 dòng.

### 4.4. SubOp `0x04` — `FUN_0075ffe4` (**CÓ BODY MỚI** — xóa 1 id khỏi bảng mời pending + toast + refresh)
- Wire: `[0E][04][id:P2..P5 DWORD LE]`
- Core (`0075ffe4_FUN_0075ffe4.c:59–175`): duyệt bảng pending `DAT_009ce154` (stride 34B, count `DAT_009d0378`) tìm record có `id` khớp (`:72`); nếu thấy → toast 2000ms chuỗi `DAT_007601F8 + <tên entry+6> + DAT_0076020C` (3 phần, `:84–92`), rồi xóa record (dồn mảng các record sau lên, `:120–152`), zero record cuối + giảm counter (`:154–167`); cuối hàm luôn gọi `FUN_0075eec0(mgr)` refresh (`:174`) + `*(mgr+0xec)=0` (`:175`). Template `007601F8/0076020C` là hằng vùng code — **không có dump, chưa dịch được**. Đây là nhánh "chấp nhận/từ chối đã được server xác nhận → remove request đang chờ".

### 4.5. SubOp `0x05` — parser danh sách nhiều record (CÓ BODY, quan trọng thứ hai OP)
- Wire tầng case: `[0E][05][rest...]`, forward nguyên RP cho `FUN_0076676c(*(gvar_007D9F1C), RP)`.
- Parse thật nằm trong `0076676c_FUN_0076676c.c:94–318`: con trỏ `pos=2` (1-based Delphi → `RP[1]` = `P[2]`), vòng lặp `while (Len > pos)`:
  - `_LStrCopy(RP,pos,4)` → `id = FUN_0077ef7c` (DWORD LE); `pos+=4`.
  - 1 byte `n = RP[pos]`; `pos+=1`.
  - `_LStrCopy(RP,pos+1,n)` → chuỗi tên `n` byte; `pos += n`.
  - `a = FUN_0077ef7c` (DWORD LE); `b = FUN_0077ef7c` (DWORD LE); 1 byte `c`; `pos += 9`.
  - Mỗi record = `[id:4][n:1][name:n][a:4][b:4][c:1]`, ghi vào bảng (stride `0x11`, tối đa 0x100 mục; `id==0x65` thì reset đếm).
- Cuối: refresh 2 form (UI thuần, 1 dòng).

### 4.6. SubOp `0x06` — toast trần
- Wire: `[0E][06]` (2 byte, không field).
- Core: toast hằng số `UNK_00797048` 2000ms (1 dòng). **Chuỗi ĐÃ DUMP + decode (§6)**: `Thùng thß bƠn hæu đă đ¥y` → phỏng đoán **"Thùng thư bạn hữu đã đầy"** — cùng họ chuỗi friend-invite với SubOp 3 (khớp: bảng thư bạn hữu tràn).

### 4.7. SubOp `0x07` — `FUN_00747a20` (**CÓ BODY MỚI** — điều kiện 1 byte, toast 1500ms)
- Wire: `[0E][07][b:P2=RP1, 1B]`; callee guard `Len<2→BoundErr(1)` (`00747a20_FUN_00747a20.c:23–26`).
- Core (`:27–29`): **nếu `RP[1] == 1` → toast `DAT_00747A74` 1500ms (0x5DC)**; giá trị khác → không làm gì. Object `gvar_007D9D34` (scene) truyền vào **không được dùng** trong body (đính chính so với giả định cũ "forward xử lý scene"). `DAT_00747A74` là hằng vùng code — **không có dump → chưa dịch được**.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xFF)`):

```c
case 0xe:
  break;
```

**Kết luận: client không bao giờ chủ động gửi OP 0x0E.** Chiều duy nhất là S→C. Không có builder payload C→S nào cho OP này (giống OP 0x02/0x09/0x0C/0x0D).

---

## 6. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ `opcode_02.md` mục 5 cũ ghi "cp1258 → NFC" — **đính chính 2026-09-14**: bảng đúng của game là **VISCII đơn-byte tiền tổ hợp** (xem `opcode_09.md §7.1`); các chuỗi đã dump dưới đây decode VISCII sạch 100%, là **văn nguyên game** (giữ nguyên typo của game).

Payload trên wire của OP 0x0E **không mang text trực tiếp** ở SubOp 2/3/4/6/7 (chỉ id/kind/cờ số); text nằm ở **hằng số trong binary** + chuỗi đuôi/record ở SubOp 1/5 (do server gửi). **Cập nhật 2026-09-14: các `lit_796*.hex` đã có trong `redump/`** — `lit_796fd4.hex` bao trọn cụm AnsiString packed từ `0x796FD4` trở đi, đủ cho 5 chuỗi toast của OP này; decode **VISCII** (đính chính recipe — xem `opcode_09.md §7.1`), cột kết quả là **văn nguyên game đã chốt** (giữ nguyên typo của game):

| # | Địa chỉ | Nơi tham chiếu | Trạng thái (mới) |
|---|---|---|---|
| 1 | `UNK_00796fd4` | SubOp 3, phần ghép chung cả 3 kind | **ĐÃ DUMP** — content `0x796FD4..0x796FDB` (8B): decode VISCII → **"Bạn chơi"** (chốt) |
| 2 | `UNK_00796fe8` | SubOp 3 kind=1 | **ĐÃ DUMP** — `0x796FE8..0x797000` (25B): decode VISCII → **"Tiếp nhận lời mời bạn hữu"** (chốt) |
| 3 | `UNK_0079700c` | SubOp 3 kind=2 | **ĐÃ DUMP** — `0x79700C..0x797024` (25B): decode VISCII → **"Cự tuyệt gia nhễp bạn hũu"** (chốt — nguyên văn typo "nhễp/hũu" của game) |
| 4 | `UNK_00797030` | SubOp 3 kind khác 1/2 | **ĐÃ DUMP** — `0x797030..0x79703C` (13B): decode VISCII → **"Không hồi ứng"** (chốt) |
| 5 | `UNK_00797048` | SubOp 6, toast trần 2000ms | **ĐÃ DUMP** — `0x797048..0x79705F` (24B): decode VISCII → **"Thùng thư bạn hữu đã đầy"** (chốt) |
| 6 | `0x796618` | Dispatcher nhãn log debug của MainOp | Vẫn không có dump → chưa dịch được |
| 7 | `"sound\\m004.wav"` | SubOp 1 (literal ASCII trong code) | Không cần giải mã; đường dẫn asset âm thanh (bản AnsiString thật cũng nằm trong block dump, content tại `0x796FBC`, 14B) |
| 8 | `DAT_00760854` | `FUN_007605ec` khi bảng đầy (toast 2000ms) | **ĐÃ DUMP** `lit_760854.hex` — content `0x760854..0x76086E` (27B, kết thúc 00 trước code `55 8B EC`): decode VISCII → **"Tin tức đã đến mức giới hạn"** (chốt) ("bảng đầy") |

Ghi chú: cùng block dump còn lộ các chuỗi nằm ngoài phạm vi OP 0x0E, ví dụ `0x797064`(49B) "Thật thông minh...Chúc mừng bạn đã trả lời đúng!!", `0x7970A8/0x7970CC/0x7970F8` (chuỗi "trả lời sai...lần thứ 3..."), `0x797118/0x79714C` ("cấm 15 phút..."), `0x797174`(79B, "hãy quay lại sau 2 tiếng nhé!") — **không được handler 0x0E tham chiếu**, chỉ ghi nhận để các file md sau dùng. Trong `ts_decompile/redump/` hiện **đã có** `lit_796*.hex`. Không suy đoán thêm nội dung khi chưa có bytes.

---

## 7. Ghi chú cho Mock Server

1. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes trên socket = plain XOR `0xAD` từng byte.
2. **Bảng frame tính sẵn** (`byte^0xAD`: `F4^AD=59`, `44^AD=E9`, `0E^AD=A3`, `01^AD=AC`, `02^AD=AF`, `03^AD=AE`, `06^AD=AB`):

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 6 min (toast trần, an toàn nhất) | `0E 06` | `F4 44 02 00 0E 06` | `59 E9 AF AD A3 AB` |
| SubOp 2 id=1 | `0E 02 01 00 00 00` | `F4 44 06 00 0E 02 01 00 00 00` | `59 E9 AB AD A3 AF AC AD AD AD` |
| SubOp 3 id=1 kind=1 | `0E 03 01 00 00 00 01` | `F4 44 07 00 0E 03 01 00 00 00 01` | `59 E9 AA AD A3 AE AC AD AD AD AC` |
| SubOp 4 id=1 | `0E 04 01 00 00 00` | `F4 44 06 00 0E 04 01 00 00 00` | `59 E9 AB AD A3 A9 AC AD AD AD` |

3. **Độ dài an toàn:** SubOp 6 chỉ cần 2 byte; SubOp 7 cần **3 byte** (callee guard `Len<2→BoundErr(1)`; gửi `0E 07 00` → byte≠1, an toàn không toast — đính chính so với ghi chú cũ "6/7 chỉ cần 2 byte"); SubOp 2/4 cần 6 byte; SubOp 3 cần 7 byte; SubOp 1 cần **≥15 byte** (thiếu → `_BoundErr` ở tầng case). SubOp 5 chỉ cần 2 byte ở tầng case nhưng parser đòi record đủ dài — nên gửi `0E 05` rỗng trước (vòng lặp không chạy, chỉ refresh), rồi tăng dần record `[id:4][n:1][name:n][a:4][b:4][c:1]`.
4. **Thứ tự test gợi ý:** `0E 06` (toast trần) → `0E 05` rỗng → `0E 02/04` với id tồn tại → `0E 03` kind=1/2/3 → `0E 07` → cuối cùng `0E 01` (ghi bảng + phát âm thanh, tác động nhiều nhất).
5. **Không gửi SubOp `0x00` / `≥0x08`** — không có nhánh, rơi qua switch (vô hại nhưng vô nghĩa).
6. **Không cần mock C→S cho 0x0E** (builder rỗng). Chỉ phát S→C.

---

## 8. Source trail + giới hạn trung thực (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_014_0078E01C_FUN_0078e01c.c` | toàn file 175 dòng | Handler chính, danh sách SubOp, wire field |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0xe:` d.2535–2639; nhãn log d.634–636 | Đối chiếu inline 1:1 + nhãn debug `0x796618` |
| 3 | `ts_decompile/case_functions/manifest.csv` | dòng 16 | Case 14, entry `0x0078A9EE` → `0x0078E01C` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 14 | Xác nhận mapping |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | dòng 1 | `byte_table[0x0E]=0x0E` |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry index 14 = `1C E0 78 00` | `case 14 → 0x0078E01C` |
| 7 | `ts_decompile/redump/token_recv.hex`, `token_send.hex` | mở đầu `F4 44` | Framing token |
| 8 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0xe: break;` d.915–916 | C→S rỗng |
| 9 | `0077eb9c / 0077ef7c / 0077f098` | body codec | Word LE (xác minh **không dùng**) / DWORD LE / decode 8-byte |
| 10 | `0077eb1c`, `0077ee84` | body encode | Xác minh không dùng ở S→C này |
| 11 | `007605ec_FUN_007605ec.c` | SubOp 1: bảng stride 0xA5 × 1000 | Ghi record |
| 12 | `007a7f20_FUN_007a7f20.c` | SubOp 1: `FileExists` + phát âm thanh | Presentation 1 dòng |
| 13 | `005952cc_FUN_005952cc.c:21` | SubOp 1: `+0x171:=1` | Set cờ |
| 14 | `0075ddb8_FUN_0075ddb8.c:32–40` | SubOp 3: resolve tên | Tra cache tên |
| 15 | `0076676c_FUN_0076676c.c:94–320` | SubOp 5: vòng lặp record | Parser danh sách |
| 16 | `ts_decompile/redump/lit_796fd4.hex`, `lit_796f50.hex` (block kế), `lit_760854.hex` | **dump mới 2026-09-14** | Decode 5 chuỗi toast OP này + "bảng đầy" (§6) |
| 17 | `ts_decompile/functions/0075d5b8 / 0075d7ec / 0075f9d8 / 0075ffe4 / 00747a20` (`*_FUN_*.c`) | **body mới 2026-09-14** | SubOp 2/4/7 + 2 callee SubOp 1/3 — bảng mời pending `DAT_009ce154` (`0075f9d8.c:64–388`, `0075ffe4.c:59–175`) |
| 18 | `0063c8fc_FUN_0063c8fc.c:42–50`, `0075eec0_FUN_0075eec0.c:31–36` | lá | Callback toast 10s (SubOp 2) + refresh bảng mời |

**Giới hạn (không suy diễn) — cập nhật 2026-09-14:**
- `func_0x0075d5b8`, `func_0x0075f9d8`, `func_0x0075ffe4`, `func_0x0075d7ec`, `func_0x00747a20` **đã có body đầy đủ** (bỏ khỏi danh sách khoảng trống; phân tích ở §4.2–4.7).
- Còn thiếu body: callback `FUN_0075d738` (nút bấm của toast SubOp 2) và các lá `FUN_00763444/007633f0` (refresh bảng mời) — chưa dump.
- Template/hằng toast mới phát hiện trong body, **không có dump**: `DAT_0075d66c` / `LAB_00760870` (SubOp 2), `DAT_007601F8` / `DAT_0076020C` (SubOp 4), `DAT_00747A74` (SubOp 7).
- Ý nghĩa game-design của `id`, `blob8`, `flag`, các DWORD trong record SubOp 5, và **2 con số 9 chữ số BCD trong record SubOp 3 kind=1** nằm ngoài tầng case — chỉ kết luận ở mức "sự kiện/thông báo có tham số + toast + âm thanh"; nghiệp vụ friend/invite là **suy luận có cơ sở** từ bộ chuỗi §6 + bảng pending, chưa phải kết luận tuyệt đối.

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x0078E01C` — C `client_pseudo_c/case_014_0078E01C_FUN_0078e01c.c` (có); asm `client_pseudo_c/0078e01c_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x0E` — 6 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `1` | `0x0078E06B` | sound\m004.wav | sound\m004.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x0078E193` | Bạn chơi | Bạn chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x0078E193` | Tiếp nhận lời mời bạn hữu | Tiếp nhận lời mời bạn hữu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x0078E193` | Cự tuyệt gia nhễp bạn hũu | Cự tuyệt gia nhễp bạn hũu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x0078E193` | Không hồi ứng | Không hồi ứng. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `6` | `0x0078E331` | Thùng thư bạn hữu đã đầy | Thùng thư bạn hữu đã đầy. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x0078E01C`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
