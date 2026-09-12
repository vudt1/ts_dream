# PHÂN TÍCH — Main OP 0x19 (Case 22, `FUN_0079157c` @ `0x0079157C`) — **Bus pass-through đa năng (16 nhánh SubOp, parse nằm trong hàm con)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

---

## 0. Tóm tắt nghiệp vụ

**OP 0x19 CÓ SubOp.** Handler `FUN_0079157c` là một **bộ chia nhánh thuần túy (dispatcher cấp 2)**: đọc 1 byte `SubOp = payload[1] = ECX[0]`, `switch` ra **16 nhánh**, mỗi nhánh chỉ làm đúng 1 việc là **forward nguyên RestPayload cho 1 hàm con + 1 object toàn cục**, không tách field nào ở tầng case, không gọi codec Word/DWORD nào ở tầng này.

- 2/16 nhánh có body để bóc tiếp: SubOp 2 (`FUN_0075aff4`, toast hệ thống theo mã 0–8 + default) và SubOp 0x20 (`FUN_007281ec`, resolve tên + chèn dòng chat hệ thống).
- 14/16 nhánh còn lại là **hộp đen**: các hàm con **không có file body `.c` trong `ts_decompile/functions/`** — chỉ biết object đích + pass nguyên buffer, không suy diễn parse bên trong.
- Khác OP 0x0D (team/toast) và OP 0x10 (GM/quiz): OP 0x19 là **bus rẽ nhánh theo module** (mỗi SubOp trỏ về 1 manager khác nhau).

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_022_0079157C_FUN_0079157c.c:8` — `void FUN_0079157c(void)` @ `0x0079157C` (toàn file 107 dòng).
- `ts_decompile/case_functions/manifest.csv:24` — `22,0x0078AA0E,0x0079157C,EXPORTED,"FUN_0079157c"`.
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 22, body giống file case riêng.

**Dispatcher `FUN_0078a89c`:**
- Nhận `param_2` (**MainOp**), `param_1` (**RestPayload đã cắt MainOp**); tra bảng byte `0x78A8EE` lấy index rồi tra bảng dword `0x78A9B6` để nhảy. Nhánh `case 0x19:` tại **dòng 4475–4550** chứa toàn bộ 16 SubOp, khớp từng dòng với file case riêng → dispatcher không thêm logic, single source of truth là file case.
- Dấu vết khóa XOR: dòng 562–565 `iVar21 = 0xad` — vòng khóa tĩnh `0xAD`.

**Mapping MainOp 0x19 → Case 22 (xác minh 4 nguồn, KHÔNG phải identity):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` — offset `0x19` = **`0x16` = 22 thập phân** → `byte_table[0x19] = 22 = Case 22`.
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` — entry index 22 = **`7C 15 79 00` = `0x0079157C` (LE)** → khớp manifest.
- `manifest.csv:24` + jump-table entry `0x0078AA0E` → khớp.
- `0078a89c_FUN_0078a89c.c:4475` `case 0x19:` → nội dung trùng file case.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD` từng byte. Server speaks first. Token `F4 44` hiện diện trong `token_recv.hex`.

**Cách đọc SubOp (file case dòng 20–27):**
```c
iVar2 = *(int *)(unaff_EBP + -0xc);              // ECX = RestPayload (payload gốc đã cắt byte P[0]=MainOp)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + 0); // SubOp = ECX[0] = payload[1]
switch(SubOp) { case 1 ... case 0x29 ... }        // KHÔNG có case 0, KHÔNG có default
```
- `unaff_EBP-0xc` = RestPayload. **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec.
- Quy ước Delphi `Copy(ECX, p, n)` là **1-based**: `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`. **Handler này không gọi `LStrCopy` lần nào ở tầng case** — toàn bộ parse (nếu có) nằm trong hàm con.
- Guard duy nhất ở tầng case: gửi payload rỗng (thiếu cả SubOp) thì lỗi; còn lại mọi SubOp có trong switch đều được forward.

**Codec helper (đã đọc body — nhưng handler này KHÔNG dùng cái nào ở tầng case):**
- `FUN_0077eb9c` — `b0 + b1*0x100` = **Word LE**. Tầng case **không gọi lần nào**.
- `FUN_0077ef7c` — `b0+b1*0x100+b2*0x10000+b3*0x1000000` = **DWORD LE**. Tầng case **không gọi**; chỉ hàm con `FUN_007281ec` gọi (`Copy(RP,2,4)` → DWORD LE).
- `FUN_0077eb1c` — encode Word → chuỗi 2 byte. Chỉ dùng chiều C→S, không dùng ở S→C 0x19.
- `FUN_0077ee84` — encode DWORD → chuỗi 4 byte. Chỉ dùng chiều C→S, không dùng ở S→C 0x19.
- `FUN_0077f098` — copy tên 8 byte. Handler 0x19 không gọi.

---

## 2. Đối chiếu dispatcher inline

Khối `case 0x19:` trong `0078a89c_FUN_0078a89c.c:4475–4550` khớp 1:1 với file case riêng (cùng 16 label, cùng thứ tự, cùng cặp `(gvar, RestPayload)`; bản dispatcher chỉ thêm gán địa chỉ traceback, không phải logic). → Dispatcher không thêm logic nào ngoài jump-table.

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: **`1, 2, 3, 10 (0xa), 0xb, 0xc, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1f, 0x20, 0x21, 0x22, 0x29`**. **Không có `case 0`, không có `default`** → SubOp 0 / các giá trị vắng mặt (4–9, 0xd–0x13, 0x19–0x1e, 0x23–0x28, ≥0x2a) rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x19`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu (tầng case) | Wire field ở tầng case | Handler tầng case |
|---|---|---|---|
| `0x01` | **2** (`19 01`) | không tách field, forward nguyên RP | `func_0x0075af6c(*(gvar_007DA700), RP)` — hộp đen |
| `0x02` | **2** (`19 02`); hiệu lực đầy đủ cần **3** (`19 02 sel`) vì callee đọc `RP[1]` | forward nguyên RP; parse trong callee | `FUN_0075aff4(*(gvar_007DA700), RP)` — **có body**: sel `RP[1]` + toast |
| `0x03` | **2** (`19 03`) | forward nguyên RP | `func_0x0075b3ec(*(gvar_007DA700), RP)` — hộp đen |
| `0x0A` | **2** (`19 0A`) | forward nguyên RP | `func_0x0075bf7c(*(gvar_007DA77C), RP)` — hộp đen |
| `0x0B` | **2** (`19 0B`) | forward nguyên RP | `func_0x0075b8e0(*(gvar_007DA77C), RP)` — hộp đen |
| `0x0C` | **2** (`19 0C`) | forward nguyên RP | `func_0x0060bbe0(*(gvar_007DA438), RP)` — hộp đen |
| `0x14` | **2** (`19 14`) | forward nguyên RP | `func_0x0062e41c(*(gvar_007DA32C), RP)` — hộp đen |
| `0x15` | **2** (`19 15`) | forward nguyên RP | `func_0x00735184(*(gvar_007DA7BC), RP)` — hộp đen |
| `0x16` | **2** (`19 16`) | forward nguyên RP | `func_0x0062e11c(*(gvar_007DA32C), RP)` — hộp đen |
| `0x17` | **2** (`19 17`) | forward nguyên RP | `func_0x0062e538(*(gvar_007DA32C), RP)` — hộp đen |
| `0x18` | **2** (`19 18`) | forward nguyên RP | `func_0x0062e814(*(gvar_007DA32C), RP)` — hộp đen |
| `0x1F` | **2** (`19 1F`) | forward nguyên RP | `func_0x00728310(*(gvar_007D9C48), RP)` — hộp đen |
| `0x20` | **2** (`19 20`); hiệu lực đầy đủ cần **6** (`19 20 id4`) vì callee `Copy(RP,2,4)` | forward nguyên RP; parse trong callee | `FUN_007281ec(*(gvar_007D9C48), RP)` — **có body**: `id = DWORD LE RP[2..5]` + chat hệ thống |
| `0x21` | **2** (`19 21`) | forward nguyên RP | `func_0x00742a50(*(gvar_007DA7BC), RP)` — hộp đen |
| `0x22` | **2** (`19 22`) | forward nguyên RP | `func_0x00742f78(*(gvar_007D9D34), RP)` — hộp đen |
| `0x29` | **2** (`19 29`) | forward nguyên RP | `func_0x0074df70(*(gvar_007DA7BC), RP)` — hộp đen |

---

## 4. Chi tiết từng SubOp

### 4.1. Các SubOp hộp đen (14 nhánh, 1 mẫu chung)
- Wire tầng case: `[19][sub][rest...]` — **không đọc `RP[1+]` ở tầng case, không Word/DWORD**, forward nguyên `RP` cùng 1 object toàn cục.
- Core mỗi nhánh đúng 1 call:
  - `0x01/0x02/0x03` → object `gvar_007DA700` (cùng nhóm với 2 nhánh có body → cùng module).
  - `0x0A/0x0B` → object `gvar_007DA77C`.
  - `0x0C` → object `gvar_007DA438`.
  - `0x14/0x16/0x17/0x18` → object `gvar_007DA32C` (4 nhánh cùng object).
  - `0x15/0x21/0x29` → object `gvar_007DA7BC`.
  - `0x1F/0x20` → object `gvar_007D9C48` (cùng nhóm; 0x20 có body).
  - `0x22` → object `gvar_007D9D34`.
- **Giới hạn:** 14 hàm con **không có file body trong `ts_decompile/functions/`**. Không suy diễn field nội bộ; Mock Server chỉ cần gửi `[19][sub][bytes tùy ý]`.

### 4.2. SubOp `0x02` — mã chọn + toast hệ thống (có body)
- Wire: `[19][02][sel:P2=RP1, 1B]` (tầng case forward nguyên RP; callee đọc tiếp).
- Đọc trong `0075aff4_FUN_0075aff4.c:41–89`: guard `Len(RP)<2 → BoundErr(1)`; `sel = RP[1]` byte thường.
- Core: `sel` 0–8 → toast với duration **3000ms** (9 hằng `DAT_0075b1e4...LAB_0075b3a0`); `default` (mọi giá trị còn lại) → toast `DAT_0075b3cc` **1000ms**; sau đó luôn refresh form (thuần UI, 1 dòng).
- Chuỗi: **không có file `lit_75b*.hex` nào trong `redump/`** → chỉ ghi địa chỉ + duration, chưa dịch được, không bịa nội dung.

### 4.3. SubOp `0x20` — id DWORD + dòng chat hệ thống có điều kiện (có body)
- Wire: `[19][20][id:P2..P5=RP2..RP5, DWORD LE]`.
- Đọc trong `007281ec_FUN_007281ec.c:56–59`: `_LStrCopy(RP,2,4)`; `id = FUN_0077ef7c` = DWORD LE; `slot = FUN_00722508(obj, id)`.
- Core:
  1. `if (slot==0) return` — id lạ (không resolve được slot) thì bỏ qua, không hiển thị gì.
  2. Ngược lại: nối tên base + hậu tố `LAB_00728300` (46 chars) rồi `FUN_007ab870(TTalkMsgForm, 0, msg, tag=10)` — chèn 1 dòng hệ thống (presentation 1 dòng, tag 10).
- Chuỗi `DAT_007282ec` / `LAB_00728300`: **không có dump trong `redump/`** → chưa dịch được.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`, `TFConnect.SendCommand`):
```c
case 0x19:
  (**(code **)(&DAT_0078540f + (uint)DAT_007853e5 * 4))();
  return;
```
**Kết luận: C→S OP 0x19 KHÔNG phải `break` rỗng** (khác OP 0x0D/0x10). Client có đường gửi 0x19 nhưng thực hiện bằng **nhảy gián tiếp qua bảng con** với key là global runtime (không phải param payload) và Ghidra báo không khôi phục được jumptable. Vì key là trạng thái UI runtime nên **không resolve tĩnh được nhánh gửi nào, không trích được builder payload C→S**. Ghi rõ giới hạn, không suy diễn.

---

## 6. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ đã xác minh ở `opcode_02.md` mục 5: dump Delphi ansistring `[len:4LE][chars]` giải mã đúng bằng **cp1258 → NFC** (không phải VISCII). Mọi chuỗi toast/chat của OP 0x19 (nếu dump được) cũng phải giải mã theo **cp1258 → NFC**.

Tầng case 0x19 **không tham chiếu hằng chuỗi nào**. Text nằm trong 2 hàm con có body:

| # | Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|---|
| 1 | `DAT_0075b1e4...LAB_0075b3a0` (sel 0–8, toast 3000ms) | `FUN_0075aff4` (SubOp 2) | **Không có `lit_75b*.hex` → chưa dịch được** |
| 2 | `DAT_0075b3cc` (default, toast 1000ms) | `FUN_0075aff4` (SubOp 2) | Như trên |
| 3 | `DAT_007282ec` (tiền tố tên) | `FUN_007281ec` (SubOp 0x20) | **Không có `lit_7282ec.hex` → chưa dịch được** |
| 4 | `LAB_00728300` (hậu tố, 46 chars) | `FUN_007281ec` (SubOp 0x20) | **Không có `lit_728300.hex` → chưa dịch được** |

Không suy đoán nội dung tiếng Việt khi chưa có bytes. Cần dump Delphi ansistring tại các địa chỉ trên từ binary gốc rồi `bytes.decode('cp1258')` + `unicodedata.normalize('NFC', s)`.

---

## 7. Ghi chú cho Mock Server

1. **Phạm vi:** OP 0x19 là **bus rẽ nhánh S→C**, mỗi SubOp thuộc 1 module khác nhau. Muốn test hiển thị thì tập trung SubOp `0x02` (toast) và `0x20` (chat hệ thống); các SubOp còn lại là hộp đen — test cách ly, log crash.
2. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes socket = plain XOR `0xAD` từng byte (`b^0xAD`: `F4→59`, `44→E9`, `00→AD`, `19→B4`, `02→AF`, `20→8D`).
3. **Bảng frame tính sẵn (payload tối thiểu tầng case = 2B):**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 1 min | `19 01` | `F4 44 02 00 19 01` | `59 E9 AF AD B4 AC` |
| SubOp 2 sel=0 (toast 3000ms) | `19 02 00` | `F4 44 03 00 19 02 00` | `59 E9 AE AD B4 AF AD` |
| SubOp 2 default (toast 1000ms, sel=9) | `19 02 09` | `F4 44 03 00 19 02 09` | `59 E9 AE AD B4 AF A4` |
| SubOp 0x20 id=1 | `19 20 01 00 00 00` | `F4 44 06 00 19 20 01 00 00 00` | `59 E9 AB AD B4 8D AC AD AD AD` |
| SubOp 0x29 min | `19 29` | `F4 44 02 00 19 29` | `59 E9 AF AD B4 84` |

4. **Thứ tự test an toàn:** `19 01` / `19 03` (hộp đen, quan sát) → `19 02 09` (toast default 1000ms) → `19 02 00..08` (toast 3000ms từng mã) → `19 20 <id sniff được>` (chỉ hiện khi id resolve được slot; id lạ = no-op) → cuối cùng mới thử các object khác.
5. **Không gửi SubOp `0x00` / các giá trị vắng mặt** (`04–09`, `0D–13`, `19–1E`, `23–28`, `≥2A`) — rơi qua switch (vô hại nhưng vô nghĩa).
6. **Chiều C→S 0x19 không mock được tĩnh** (nhảy gián tiếp qua key runtime); chỉ mock chiều S→C.

---

## 8. Source trail (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_022_0079157C_FUN_0079157c.c` | @`0x0079157C`, toàn file 107 dòng | Handler chính, 16 SubOp, wire field |
| 2 | `ts_decompile/case_functions/manifest.csv` | dòng 24 | Case index 22, entry `0x0078AA0E` → `0x0079157C` |
| 3 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 22 | Xác nhận mapping lần 2 |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | khóa `0xAD`; `case 0x19:` d.4475–4550 | Dispatcher MainOp, đối chiếu inline 1:1 |
| 5 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x19:` | C→S nhảy gián tiếp (không rỗng) |
| 6 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | offset `0x19` = `0x16` = 22 | Xác minh MainOp→Case |
| 7 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry 22 `7C 15 79 00` | `= 0x0079157C` |
| 8 | `ts_decompile/redump/token_recv.hex` | (`F4 44`) | Token framing |
| 9 | `ts_decompile/redump/subtable_0x7853E5.hex`, `subtable_0x78540F.hex` | — | Bảng con chiều C→S (không resolve key runtime) |
| 10 | `0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | codec | Xác minh không dùng ở tầng case |
| 11 | `0075aff4_FUN_0075aff4.c` | — | Core SubOp 2 (sel + toast) |
| 12 | `007281ec_FUN_007281ec.c` | — | Core SubOp 0x20 (id DWORD + chat) |
| 13 | Giới hạn: 14 hàm con không body | glob rỗng | Không suy diễn |
| 14 | Giới hạn chuỗi | glob `lit_75b*/lit_728*` rỗng | Chưa dịch, không bịa |

*Ghi chú trung thực: mọi kết luận nghiệp vụ (bus pass-through, toast theo sel, chat theo id) đều suy trực tiếp từ branch + call trong source trên. Phần hiển thị chỉ tóm 1 dòng theo yêu cầu. 14/16 nhánh và toàn bộ nội dung text là giới hạn đã ghi rõ.*
