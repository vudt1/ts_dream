# PHÂN TÍCH — Main OP 0x18 (Case 21, `FUN_00790ed5` @ `0x00790ED5`) — **Kho / vật phẩm + đồng bộ cờ trạng thái (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

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
| `0x03` | **2** (`18 03`) | không field | toast `DAT_00797fe0` 2000ms — **đã dịch (§6)**: "Dung lượng nhiệm vụ đã đầy" |
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
- Core: toast hằng số `DAT_00797fe0` 2000ms (`case_021.c:147`, VMT `gvar_007DA084+0x90(text, 2000, 0, 0)`) — **đã dịch từ `redump/lit_797fe0.hex` (§6)**: "Dung lượng nhiệm vụ đã đầy".

### 4.4. SubOp `0x04` — xóa sạch 1 mã vật phẩm
- Wire: `[18][04][id:P2..P3 Word LE]` (chỉ `Copy(RP,2,2)`, không có byte số lượng).
- Core: `slot = FUN_00720f00(self,id)` (có body): tra slot; nếu có → giảm đếm ô, zero 3 byte slot, set cờ. Rẽ `slot==0` / `!=0` ghép toast tên vật phẩm. Hiển thị 1 dòng.

### 4.5. SubOp `0x05` — thao tác 1 mã + số lượng (void)
- Wire: `[18][05][id:P2..P3 Word LE][n:P4, 1B]`.
- Core: `FUN_00721088(self,id,n)` — **đã bóc đủ body 2026-09-14** (`00721088_FUN_00721088.c:26-209`), thực chất 2 phần:
  1. **Toast so số lượng, chỉ cho 7 mã item đặc thù**: tra loại item `gvar_007D9FD8[id]` (bảng 3000, stride 0x81, field loại tại `+0x104`); nếu loại ∈ {`0x2717, 0x29B2, 0x2952, 0x2A01, 0x2A03, 0x2DB2, 0x2EFF`} → `count = FUN_0072147c(self,id)`; `count < n` → toast `DAT_00721444` (timeout 0); `n < count` → toast `DAT_00721464` (2000ms hoặc 1ms tùy cờ `self+0x376`) (`:123-134`). Hai hằng chưa dump → chưa dịch.
  2. **Bật/tắt BIT cờ theo mã item**: `n>1` ép về 0; `n==1` → set bit `1<<((id − (Round(…)-1)*8 −1)&31)`, `n==0` → clear bit, tại **byte array `LocalActor+0x8B2..0x9DE` (300 byte = 2400 bit)**, index byte `Round(…)−1 ≤ 299` (`:135-207`); kết thúc set dirty `self+0x9DF=1` (`:208`). Công thức bit phụ thuộc giá trị ST0 mà Ghidra không model (`FUN_00424e00`=Round, tham số mất) → **công thức chính xác chưa kết luận được**, bán chất "1 bit/mã item" đã kiểm.
- ⇒ SubOp 5 = "cập nhật bit trạng thái sở-hữu/kích-hoạt cho 1 mã vật phẩm" + toast phụ cho vài loại đặc thù.

### 4.6. SubOp `0x06` — đồng bộ hàng loạt entry 4-byte (CÓ BODY)
- Wire tầng case: `[18][06][rest...]`, forward nguyên RP cho `FUN_0072bb6c(self,RP)`.
- Parse thật: `count = (Len(RP)-1)/4`; con trỏ `pos=2` (Delphi 1-based); mỗi vòng đọc `[slot:1B][code:2B Word LE][val:1B]`, ghi `code/val` vào kho self theo slot, tăng đếm; cuối set cờ. `slot>200 → BoundErr`. Payload ngắn hơn entry đang đọc → `_BoundErr`.

### 4.7. SubOp `0x07` — đồng bộ hàng loạt entry 3-byte (CÓ BODY)
- Wire tầng case: `[18][07][rest...]`, forward nguyên RP cho `FUN_0072ba54(self,RP)`.
- Parse thật: `count = (Len(RP)-1)/3`; mỗi vòng `[code:2B Word LE][val:1B]`, ghi `val` vào bảng theo `code` (guard `code-1 ≤ 299`); cuối set cờ.

### 4.8. SubOp `0x08` — đồng bộ 1 byte cờ trạng thái theo id (CÓ BODY)
- Wire tầng case: `[18][08][rest...]` forward nguyên RP; parse thật trong `FUN_00729a88`: `_LStrCopy(RP,2,4)` → `id` DWORD LE = `P[2..5]`; `_LStrCopy(RP,6,2)` → `kind` Word LE = `P[6..7]`; byte `RP[7]` (= `P[8]`, guard `Len<8→BoundErr(7)`) → `flag`. Payload tối thiểu thật **9 byte**.
- Core:
  - **Chi tiết field đã xác minh lại từ body `00729a88_FUN_00729a88.c:60-121`**: `kind==1` (`:71`): nếu `id==self` → cờ tại **`LocalActor+0x448`** (`:73,79`) — chỉ khi **giá trị cũ khác flag mới và flag mới == 0** mới phát `"sound\WA0006.wav"` + toast `DAT_00729d0c` 2000ms (`:73-78`), rồi ghi cờ. `id` khác self → ghi vào record cache `gvar_007DA6BC[FUN_00722508(gvar_007D9C48,id)*4]+0x36` (`:82-88`) **và** actor registry `gvar_007DA300[FUN_0070c20c(gvar_007D9D34,id)*4]+0x448` (`:90-96`, guard ≤800).
  - `kind==2` (`:100`): y hệt với cờ thứ hai **`LocalActor+0x455`** / record `+0x37` / actor `+0x455` — **không có âm thanh/toast** cho kind 2 (`:100-121`).
  - `kind` khác 1/2 → không làm gì.
  - Toast SubOp 8 **đã dịch (§6, VISCII)**: "Thần xui đã rời xa bạn!" — kết hợp cờ `+0x448` + WA0006; chủ thể "Thần xui" (linh vật/đồng hành?) chưa kết luận được nghiệp vụ.

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

## 6. Chuỗi hiển thị / mã hóa tiếng Việt — **ĐÃ DỊCH TỪ DUMP MỚI (2026-09-14)**

**Phương pháp:** `redump/lit_797fe0.hex`, `lit_729d0c.hex`, `lit_7967f8.hex` (base = chính địa chỉ DAT, 512B, parse `python3`). **ĐÍNH CHÍNH BẢNG MÃ:** recipe "cp1258→NFC" của tài liệu cũ **sai cho data này** — decode sạch 100% là **VISCII** (đơn-byte tiền tổ hợp, khớp các neo đã chốt ở `opcode_13.md` mục 7; kiểm chứng chéo: cp1258 cho `M§t kh¦u`, VISCII cho `Mất khẩu`). Phát hiện thêm: một số record cùng run là **tiếng Trung phồn thể mã Big5** (game song ngữ Việt–華).

| # | Địa chỉ | Nơi dùng | Chuỗi VISCII decode (chính xác theo byte) |
|---|---|---|---|
| 1 | `DAT_00797FE0` (dump `lit_797fe0` off 0x00, 26B; trùng run `lit_797ee8` off 0xF8 — khớp 100%) | SubOp 3 toast 2000ms (`case_021.c:147`) | **"Dung lượng nhiệm vụ đã đầy"** (nguyên văn; nghĩa game-design của "nhiệm vụ" ở đây chưa kết luận được) |
| 2 | `DAT_00729D0C` (dump `lit_729d0c` off 0x00, 23B — **chỉ 1 chuỗi, ngay sau đó là CODE** `55 8B EC 83 C4 E8…`, không phải run packed) | SubOp 8 toast 2000ms khi self tắt cờ (`00729a88.c:76`) | **"Thần xui đã rời xa bạn!"** — chủ thể "Thần xui" (linh vật/đồng hành?) **chưa kết luận được nghiệp vụ** |
| 3 | `"sound\WA0006.wav"` | SubOp 8 | literal ASCII | asset âm thanh |
| 4 | `0x7967F8` | **ĐÍNH CHÍNH LỚN**: KHÔNG phải "nhãn log debug của MainOp" cho OP 0x18. Chuỗi nằm trong **bảng lý-do-ngắt-kết của MainOp 0x00**: `0078a89c_FUN_0078a89c.c:617` outer `case 0:` → inner `switch(byte RP[0])` → `case 0x18:` `@LStrLAsg(…, 0x7967f8)` (dòng 665). Không liên quan OP 0x18 |

**Run `lit_7967f8.hex` (base 0x007967F8) = bảng thông báo disconnect (phụ lục để đối chiếu — thuộc MainOp 0x00, không thuộc OP 0x18):**

| Offset | Địa chỉ | Chuỗi VISCII decode |
|---|---|---|
| 0x00 | 0x7967F8 | "Mật khẩu quá ngắn, mất kết nối" |
| 0x28 | 0x796820 | "Trùng lập tên, mất kết nối" |
| 0x4C | 0x796844 | "Sự kiện phạm luật, mất kết nối" |
| 0x74 | 0x79686C | "Mất kết nối do đăng nhập sai" |
| 0x9C | 0x796894 | "Phòng vệ mất kết nối" (nguyên văn "Phòng vệ") |
| 0xBC | 0x7968B4 | "Dữ liệu quá nhiều" |
| 0xD8 | 0x7968D0 | "Khóa tài khoản, mất kết nối" |
| 0xFC | 0x7968F4 | "Không thể sử dụng ID này" |
| 0x120 | 0x796918 | "Cảnh chiến đấu bị lỗi" |
| 0x140 | 0x796938 | "Ký hiệu và quang cảnh không phù hợp, mất kết nối" |
| 0x17C | 0x796974 | "Đăng nhập lại qua Server" |
| 0x1A0 | 0x796998 | "Hiệp định đăng nhập" |
| 0x1BC | 0x7969B4 | "Phạm vi - ID không phù hợp" |
| 0x1FC | — | record len 0x21 **bị cắt do dump 512B** (đầu chuỗi: "Rất mừng do kênh cứng…", không đủ cơ sở dịch) |

**Run `lit_797fe0.hex` tiếp sau SubOp-3-toast là các mảnh UI *giao dịch/hạn mức* cùng module** (không phải chuỗi OP 0x18 khác; VISCII decode): 0x798038 "Nhận được ", 0x79804C "Tiền", 0x79805C "Thất bại  đoạt được", 0x798078 "kim", 0x798084 "Giảm", 0x798094 "Thất bại   giảm thiểu", 0x7980B4 "Dung lượng tiền bạc không đủ", 0x7980DC "Thủy binh", 0x7980F0 "Tài bảo", 0x798100 "Điểm đạn dược", 0x798118 **"Giao dịch thành công"**, 0x798138 **"Xin lỗi, vàng của bạn không đủ!"**, 0x798160 **"Xin lỗi, bảng vật phẩm của bạn đã đầy!"**, 0x798190 **"Xin lỗi, Số lượng vật phẩm bạn mua đã đầy!"**, 0x7981C4 **"Xin lỗi, giao dịch thất bại"**; 2 record **Big5 phồn thể**: 0x798004 `整個刪除 !!` ("toàn bộ xóa bỏ!!"), 0x798018 `刪除失敗` ("xóa bỏ thất bại") + mảnh nối `+`, ` = `. (Run `lit_797ee8` của OP 0x17 phủ trùng 0x797FE0–0x7980DC, số liệu khớp 100%; Big5: `標記` 0x797F80, `個標記` 0x797FA8, `增加失敗` 0x797FB8, `減少失敗` 0x797FCC.)

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
| 15 | `redump/lit_797fe0.hex`, `lit_729d0c.hex`, `lit_7967f8.hex` (**mới**, 512B/run, parse `python3` — decode **VISCII**, đính chính cp1258 sai) | §6 | **Đã dịch** toast SubOp 3 ("Dung lượng nhiệm vụ đã đầy"), toast SubOp 8 ("Thần xui đã rời xa bạn!"); đính chính `0x7967F8` thuộc bảng disconnect OP 0x00 |
| 16 | `functions/00729a88_FUN_00729a88.c:60-121` | SubOp 8 | Field cờ thật: `LocalActor+0x448` (kind1) / `+0x455` (kind2); id lạ → record `gvar_007DA6BC[...]+0x36/0x37` + actor `gvar_007DA300[...]+0x448/0x455`; chỉ kind 1 có WAV+toast |

**Giới hạn (không suy diễn — cập nhật 2026-09-14):** `FUN_00721088` đã bóc cả hai nửa (§4.5 — chỉ còn công thức index bit mất tham số Round); các call toast/refresh là presentation 1 dòng; 2 hằng toast nhánh đặc thù `DAT_00721444/00721464` chưa có dump; **toast SubOp 3 + SubOp 8 đã dịch** (§6, decode VISCII sạch; record Big5 `0x798004/0x798018` đã đọc là `整個刪除 !!` / `刪除失敗`; record cuối `lit_7967f8` + record `lit_729d0c` sau off 0x1FC/0x17 bị cắt dump 512B); chủ thể "Thần xui" của toast SubOp 8 chưa rõ nghiệp vụ; ý nghĩa game-design chi tiết (vật phẩm nào, cờ nào) nằm ngoài tầng case — chỉ kết luận ở mức "kho/vật phẩm + cờ trạng thái".
