# PHÂN TÍCH — Main OP 0x18 (Case 21, `FUN_00790ed5` @ `0x00790ED5`) — **Kho / vật phẩm + đồng bộ cờ trạng thái (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

---

## 0. Tóm tắt nghiệp vụ

**OP 0x18 là kênh "kho / vật phẩm + cờ trạng thái":** 1 byte SubOp chọn 1 trong 8 đường xử lý. Đọc được toàn bộ body cho thấy: SubOp 1 = cộng dồn vật phẩm (mã Word + số lượng byte) vào kho self; SubOp 2 = trừ/tiêu hao vật phẩm; SubOp 3 = toast trần 2000ms; SubOp 4 = xóa sạch 1 mã vật phẩm; SubOp 5 = thao tác 1 mã + số lượng (có tra bảng tên); SubOp 6 = đồng bộ hàng loạt entry 4-byte; SubOp 7 = đồng bộ hàng loạt entry 3-byte; SubOp 8 = đồng bộ 1 byte cờ trạng thái theo id (có âm thanh + toast khi self tắt cờ). Toàn bộ hiển thị/âm thanh chỉ tóm 1 dòng theo yêu cầu.

**Kết luận về SubOp:** handler **CÓ `switch(SubOp)`** với các label `1..8`. **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec.

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_021_00790ED5_FUN_00790ed5.c:8` — `void FUN_00790ed5(void)` @ `0x00790ED5` (229 dòng)
- `ts_decompile/case_functions/manifest.csv:23` — `21,0x0078AA0A,0x00790ED5,EXPORTED,"FUN_00790ed5"`
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 21 (cùng target)

**Dispatcher:** `FUN_0078a89c` nhận `param_2` (**MainOp**), `param_1` (**RestPayload** đã cắt MainOp), tra bảng byte `0x78A8EE` + bảng dword `0x78A9B6` rồi `switch`. Nhánh **`case 0x18:` tại dòng 4269–4474** chứa toàn bộ SubOp 1..8, khớp logic với file case riêng → dispatcher không thêm logic, single source of truth là file case.

**Mapping MainOp 0x18 → Case 21 (xác minh 3 nguồn):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` — index `0x18` = giá trị `0x15` = 21 thập phân.
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` — entry index 21 = `D5 0E 79 00` = `0x00790ED5` (LE). Khớp manifest.
- `manifest.csv:23` + jump-table entry `0x0078AA0A` → khớp.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD` từng byte. Server speaks first. Token `F4 44` hiện diện trong `token_recv.hex`.

**Cách đọc SubOp (file case dòng 25–32):**
```c
iVar6 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload (payload đã cắt MainOp)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6 + 0); // SubOp = ECX[0]
switch(SubOp) { case 1 ... case 8 ... }      // không có case 0, không có default
```
- `unaff_EBP-0xc` = **RestPayload** = payload gốc bỏ byte `P[0]=MainOp`.
- **SubOp = `payload[1]` = `ECX[0]`**, 1 byte thường, không codec. Guard `_BoundErr(0)` nếu rỗng.
- `SubOp == 0` hoặc `>= 9` → rơi qua switch, chỉ chạy refresh + cleanup cuối hàm, không crash.

**Quy ước `_LStrCopy` 1-based (Delphi `Copy`):** `_LStrCopy(ECX,p,n)` = `ECX[p-1..p+n-2]` = `payload[p..p+n-1]`. Ví dụ `Copy(ECX,2,2)` = `payload[2..3]` (2 byte Word).

**Codec helper (đã đọc body):**
| Hàm | File body | Kết luận |
|---|---|---|
| `FUN_0077eb9c` Word LE | `b0 + b1*0x100`, guard từng byte | Handler dùng **4 lần** (mã vật phẩm các SubOp 1,2,4,5) + bên trong các hàm kho |
| `FUN_0077ef7c` DWORD LE | `b0+b1*256+b2*65536+b3*2^24` | **Tầng case không gọi lần nào** — chỉ dùng bên trong `FUN_00729a88` (id SubOp 8) |
| `FUN_0077eb1c` | encode Word → chuỗi 2 byte, chỉ dùng chiều C→S. Không dùng ở S→C này |
| `FUN_0077ee84` | encode DWORD → chuỗi 4 byte, chỉ dùng chiều C→S. Không dùng ở S→C này |
| `FUN_0077f098` | copy 8 byte | Handler 0x18 **không gọi** |

---

## 2. Đối chiếu dispatcher inline

Khối `case 0x18:` trong `0078a89c_FUN_0078a89c.c:4269–4474` khớp logic 1:1 với file case riêng (cùng `switch` 1..8, cùng `_LStrCopy` + `FUN_0077eb9c`, cùng các call kho, cùng hằng `DAT_00797fe0`). Dispatcher không thêm logic.

---

## 3. Bảng tổng hợp toàn bộ SubOp

`switch(SubOp)` có các label: **`1,2,3,4,5,6,7,8`. Không có `case 0`, không có `default`.**

Quy ước: `P[.]` = payload gốc (`P[0]=0x18`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu | Wire field (tầng case) | Handler tầng case |
|---|---|---|---|
| `0x01` | **5** (`18 01` + 2 + 1) | `id:P2..P3 Word LE` + `n:P4 1B` (guard `Len<4→BoundErr(3)`) | `FUN_00720ca8(self,id,n)` cộng dồn + toast tên vật phẩm |
| `0x02` | **5** | `id:P2..P3 Word LE` + `n:P4 1B` | `FUN_00720df0(self,id,n)` trừ/tiêu hao + toast tên vật phẩm |
| `0x03` | **2** (`18 03`) | không field | toast `DAT_00797fe0` 2000ms |
| `0x04` | **4** (`18 04` + 2) | `id:P2..P3 Word LE` (`Copy(RP,2,2)`) | `FUN_00720f00(self,id)` xóa sạch + toast tên vật phẩm |
| `0x05` | **5** | `id:P2..P3 Word LE` + `n:P4 1B` | `FUN_00721088(self,id,n)` (void) + toast tên vật phẩm |
| `0x06` | **≥2** (`18 06` + rest) | pass nguyên RP | `FUN_0072bb6c(self,RP)` — parser vòng lặp entry 4B bên trong (có body) |
| `0x07` | **≥2** | pass nguyên RP | `FUN_0072ba54(self,RP)` — parser vòng lặp entry 3B bên trong (có body) |
| `0x08` | **2 ở tầng case, 9 để qua guard callee** | pass nguyên RP ở tầng case; parse thật trong callee | `FUN_00729a88(obj_007D9C48,RP)`: `id DWORD + kind Word + flag byte` (có body) |
| `0x00`, `≥0x09` | — | rơi qua switch | chỉ refresh + cleanup (no-op) |

---

## 4. Chi tiết từng SubOp (wire layout + logic core)

### 4.1. SubOp `0x01` — cộng dồn vật phẩm (mã Word + số lượng)
- Wire: `[18][01][id:P2..P3 Word LE][n:P4, 1B thường]`
- Đọc: `_LStrCopy(ECX,2,2)` + `FUN_0077eb9c` → `id`; byte trực tiếp `ECX[3]` (= `P[4]`, guard `Len<4→BoundErr(3)`) → `n`.
- Core:
  1. `slot = FUN_00720ca8(self, id, n)` — cộng `n` vào kho self theo mã (có body): tra slot; slot chưa có + còn ô trống → cấp ô mới, cộng dồn; slot đã có → cộng dồn `n`. Trả về index slot, `0` nếu không cấp được.
  2. `if (slot==0)` → ghép chuỗi toast 5 mảnh (mã + tên tra qua `FUN_007a9018`); `else` → ghép 9 mảnh (thêm số lượng tồn). Hiển thị thuần — 1 dòng.

### 4.2. SubOp `0x02` — trừ / tiêu hao vật phẩm
- Wire: `[18][02][id:P2..P3 Word LE][n:P4, 1B]` (guard giống SubOp 1).
- Core: `slot = FUN_00720df0(self,id,n)` (có body): tra slot; nếu có và số tồn `>= n` → trừ `n`, về 0 thì giảm đếm ô; trả về slot, `0` nếu không tìm thấy / không đủ. Rẽ `slot==0` / `!=0` ghép toast tên vật phẩm (5 vs 9 mảnh). Hiển thị 1 dòng.

### 4.3. SubOp `0x03` — toast trần
- Wire: `[18][03]` (2 byte, không field).
- Core: toast hằng số `DAT_00797fe0` 2000ms (1 dòng hiển thị).

### 4.4. SubOp `0x04` — xóa sạch 1 mã vật phẩm
- Wire: `[18][04][id:P2..P3 Word LE]` (chỉ `Copy(RP,2,2)`, không có byte số lượng).
- Core: `slot = FUN_00720f00(self,id)` (có body): tra slot; nếu có → giảm đếm ô, zero 3 byte slot, set cờ. Rẽ `slot==0` / `!=0` ghép toast tên vật phẩm. Hiển thị 1 dòng.

### 4.5. SubOp `0x05` — thao tác 1 mã + số lượng (void)
- Wire: `[18][05][id:P2..P3 Word LE][n:P4, 1B]`.
- Core: `FUN_00721088(self,id,n)` (có body: tra bảng tên vật phẩm + kiểm tra kiểu, thao tác mã + số lượng lên struct); sau đó ghép toast mã + tên. Hiển thị 1 dòng.

### 4.6. SubOp `0x06` — đồng bộ hàng loạt entry 4-byte (CÓ BODY)
- Wire tầng case: `[18][06][rest...]`, forward nguyên RP cho `FUN_0072bb6c(self,RP)`.
- Parse thật: `count = (Len(RP)-1)/4`; con trỏ `pos=2` (Delphi 1-based); mỗi vòng đọc `[slot:1B][code:2B Word LE][val:1B]`, ghi `code/val` vào kho self theo slot, tăng đếm; cuối set cờ. `slot>200 → BoundErr`. Payload ngắn hơn entry đang đọc → `_BoundErr`.

### 4.7. SubOp `0x07` — đồng bộ hàng loạt entry 3-byte (CÓ BODY)
- Wire tầng case: `[18][07][rest...]`, forward nguyên RP cho `FUN_0072ba54(self,RP)`.
- Parse thật: `count = (Len(RP)-1)/3`; mỗi vòng `[code:2B Word LE][val:1B]`, ghi `val` vào bảng theo `code` (guard `code-1 ≤ 299`); cuối set cờ.

### 4.8. SubOp `0x08` — đồng bộ 1 byte cờ trạng thái theo id (CÓ BODY)
- Wire tầng case: `[18][08][rest...]` forward nguyên RP; parse thật trong `FUN_00729a88`: `_LStrCopy(RP,2,4)` → `id` DWORD LE = `P[2..5]`; `_LStrCopy(RP,6,2)` → `kind` Word LE = `P[6..7]`; byte `RP[7]` (= `P[8]`, guard `Len<8→BoundErr(7)`) → `flag`. Payload tối thiểu thật **9 byte**.
- Core:
  - `kind==1`: nếu `id==self` và cờ đổi thành 0 → phát `"sound\WA0006.wav"` + toast `DAT_00729d0c` 2000ms, rồi set cờ; nếu id khác → set cờ cache/scene (2 dòng hiển thị/âm thanh gộp 1 dòng).
  - `kind==2`: tương tự với cờ thứ hai.
  - `kind` khác 1/2 → không làm gì.

### 4.9. Các hàm kho (đã đọc body)
- `FUN_00720ca8`: **cộng dồn vật phẩm vào kho** (cấp ô mới nếu chưa có).
- `FUN_00720df0`: **trừ/tiêu hao vật phẩm** (trừ `n`, về 0 thì thu hồi ô).
- `FUN_00720f00`: **xóa sạch 1 mã vật phẩm**.
- `FUN_00721088`: **áp/thao tác mã vật phẩm + số lượng có tra bảng** (void).
- `FUN_007a9018`: tra chỉ số tên vật phẩm trong bảng 3000 mục.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`):
```c
case 0x18:
  break;
```
**Kết luận: client không bao giờ chủ động gửi OP 0x18.** Chiều duy nhất là S→C. Không có builder payload C→S nào (giống OP 0x0D/0x0E/0x0F/0x10).

---

## 6. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ đã xác minh: dump Delphi ansistring `[len:4LE][chars]` giải mã đúng bằng **cp1258 → NFC** (không phải VISCII). Mọi chuỗi toast của OP 0x18 (nếu dump được) cũng phải giải mã theo **cp1258 → NFC**.

Payload trên wire của OP 0x18 **không mang text** (chỉ `18 | SubOp | id/n/flag` số) — text nằm ở **hằng số trong binary** + tên vật phẩm runtime:

| # | Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|---|
| 1 | `DAT_00797fe0` | SubOp 3 (toast 2000ms) | **Không có `lit_797fe0.hex` → chưa dịch được** |
| 2 | `DAT_00729d0c` | SubOp 8 (toast 2000ms khi self tắt cờ) | **Không có `lit_729d0c.hex` → chưa dịch được** |
| 3 | `"sound\WA0006.wav"` | SubOp 8 (literal ASCII) | Không cần giải mã; asset âm thanh |
| 4 | `0x7967f8` | Dispatcher nhãn log debug của MainOp | **Không có dump → chưa dịch được** |

Không suy đoán nội dung tiếng Việt khi chưa có bytes. Cần dump Delphi ansistring tại các địa chỉ trên từ binary gốc rồi `bytes.decode('cp1258')` + `unicodedata.normalize('NFC', s)`.

---

## 7. Ghi chú cho Mock Server

1. **Phạm vi:** OP 0x18 là **kho/vật phẩm + cờ trạng thái**, không phải chat (0x02), không phải team (0x0D).
2. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes trên socket = plain XOR `0xAD` từng byte (`byte^0xAD`: `18^AD=B5`, `01^AD=AC`, `02^AD=AF`, `03^AD=AE`, `04^AD=A9`, `05^AD=A8`, `06^AD=AB`, `07^AD=AA`, `08^AD=A5`).
3. **Bảng frame tính sẵn (đã verify XOR tay):**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 3 min (toast trần, an toàn nhất) | `18 03` | `F4 44 02 00 18 03` | `59 E9 AF AD B5 AE` |
| SubOp 4 id=1 | `18 04 01 00` | `F4 44 04 00 18 04 01 00` | `59 E9 A9 AD B5 A9 AC AD` |
| SubOp 1 id=1 n=1 | `18 01 01 00 01` | `F4 44 05 00 18 01 01 00 01` | `59 E9 A8 AD B5 AC AC AD AC` |
| SubOp 2 id=1 n=1 | `18 02 01 00 01` | `F4 44 05 00 18 02 01 00 01` | `59 E9 A8 AD B5 AF AC AD AC` |
| SubOp 5 id=1 n=1 | `18 05 01 00 01` | `F4 44 05 00 18 05 01 00 01` | `59 E9 A8 AD B5 A8 AC AD AC` |
| SubOp 8 id=1 kind=1 flag=0 | `18 08 01 00 00 00 01 00 00` | `F4 44 09 00 18 08 01 00 00 00 01 00 00` | `59 E9 A4 AD B5 A5 AC AD AD AD AC AD AD` |

4. **Thứ tự test an toàn:** `18 03` (toast trần) → `18 04 <id lạ>` (rẽ `slot==0`, ít tác động) → `18 01/02/05` với id tồn tại → `18 08` với id lạ → cuối cùng `18 06/07` (parser vòng lặp, tăng dần 1 entry).
5. **Lưu ý SubOp 8:** chỉ phát âm thanh + toast khi `id == id self` và cờ chuyển thành 0. Gửi id lạ sẽ chỉ set cờ cache/scene, không toast.
6. **Không gửi SubOp `0x00` / `≥0x09`** — rơi qua switch (vô hại nhưng vô nghĩa). Payload ngắn hơn mức tối thiểu → `_BoundErr`.
7. **Không cần mock C→S cho 0x18** (builder rỗng). Chỉ phát S→C.

---

## 8. Source trail + giới hạn trung thực (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_021_00790ED5_FUN_00790ed5.c` | @`0x00790ED5`, toàn file 229 dòng | Handler chính, danh sách SubOp, wire field |
| 2 | `ts_decompile/case_functions/manifest.csv` | dòng 23 | Case index 21, entry `0x0078AA0A` → `0x00790ED5` |
| 3 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 21 | Xác nhận mapping |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0x18:` d.4269–4474; nhãn log; khóa `0xAD` | Đối chiếu inline 1:1 |
| 5 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x18: break;` | Kết luận C→S rỗng |
| 6 | `0077eb9c` | Word LE (mã vật phẩm) | Codec chính OP này |
| 7 | `0077ef7c` | DWORD LE — tầng case **không dùng**, chỉ trong `FUN_00729a88` | Xác minh phạm vi dùng |
| 8 | `0077eb1c`, `0077ee84`, `0077f098` | encode/copy | Xác minh không dùng ở S→C 0x18 |
| 9 | `00720ca8 / 00720df0 / 00720f00 / 00721088` | SubOp 1/2/4/5 | Core kho |
| 10 | `0072bb6c / 0072ba54` | SubOp 6/7 vòng lặp entry | Parser hàng loạt |
| 11 | `00729a88` | SubOp 8: id DWORD + kind Word + flag | Đồng bộ cờ + âm thanh/toast |
| 12 | `007a9018` | Tra bảng tên vật phẩm | Toast tên vật phẩm |
| 13 | `redump/jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex` | mapping | Xác minh MainOp→Case |
| 14 | `redump/token_recv.hex` | `F4 44` | Framing token |
| 15 | `redump/` (vắng `lit_797fe0/729d0c/7967f8.hex`) | — | Chuỗi chưa dịch được |

**Giới hạn (không suy diễn):** nửa sau body `FUN_00721088` chưa bóc từng dòng; các call toast/refresh là presentation 1 dòng; 3 hằng chuỗi không có dump; ý nghĩa game-design chi tiết (vật phẩm nào, cờ nào) nằm ngoài tầng case — chỉ kết luận ở mức "kho/vật phẩm + cờ trạng thái".
