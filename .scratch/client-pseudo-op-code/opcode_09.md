# PHÂN TÍCH — Main OP 0x09 (Case 10) `FUN_0078d4af` @ `0x0078D4AF` — **Lệnh điều khiển phiên / State-machine phụ (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code`
Trạng thái: **Đã xác minh từ decompile case function + dispatcher inline**. OP 0x09 có **4 SubOp hoạt động (1, 3, 4, 5)**; SubOp 3 có thêm **SubSubOp (ss = 0/1/2)**. **Cập nhật 2026-09-14: cả ba handler SubOp 1/4/5 (`00713308`, `0074def4`, `007508e4`) đều đã có body** — phân tích thật thay cho ghi chú "không có body" cũ; **toàn bộ 15 chuỗi toast của SubOp 3 đã dump + giải mã được** (mục 7).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

| SubOp | Bản chất | Handler |
|---|---|---|
| `0x01` | Lệnh xác nhận cho self: đặt byte `gself+0x63c = 1` rồi **client tự đáp C→S `SendCommand(0x03, CL=0)`** (payload `[03][01]`), có gate theo cờ `gvar_007DA37C` | `func_0x00713308` — **body đã có** (`00713308_FUN_00713308.c`, xem 4.1) |
| `0x03` | State-machine của object `007DA4A8` + toast (ss=0 chạy machine, ss=1/2 báo message — **toasts đã dịch được**, xem 4.2/7) | inline + `FUN_005d1884` |
| `0x04` | Ghi tick + 1 DWORD payload vào scene manager (`+0x5530/+0x5534`) | `func_0x0074def4` — **body đã có** (`0074def4_FUN_0074def4.c`, xem 4.3) |
| `0x05` | Ghi tick + cờ + 2 DWORD vào scene manager (`+0x5538..+0x5544`) và **bắn toast 5000 ms** (2 biến thể theo byte cuối payload) | `func_0x007508e4` — **body đã có** (`007508e4_FUN_007508e4.c`, xem 4.4) |

OP 0x09 là **100% server-push** (client không chủ động gửi, xem mục 5). Hai ngoại lệ đáp tự động: sau `09 03 00`, `FUN_005d1884` tự gọi `SendCommand(9)`; và sau `09 01`, `00713308` tự gọi `SendCommand(3)` (payload `[03][01]`).

---

## 2. Entry & cách đọc PacketBuffer

### 2.1. Đường vào

1. `ClientSocket1Read` → XOR `0xAD` (`FUN_0050a248`) → deframe `[44 F4][Len:Word LE][Payload]` → `TForm1.CY_AddRevQueue`.
2. `CY_DelRevQueue` tách `MainOp = payload[0]`, `RestPayload = Copy(payload,2,Len-1)` → `FUN_0078a89c(EAX, DL=MainOp, ECX=RestPayload)`.
3. Dispatcher: `byte_table[0x78A8EE][0x09] = 0x0A` → `jumptable[0x78A9B6][10] = 0x0078D4AF` → `FUN_0078d4af` (Case 10, entry `0x0078A9DE`).
   (Xác minh chéo: khối `case 9:` trong `ts_decompile/functions/0078a89c_FUN_0078a89c.c` dòng 2130–2178 khớp byte-for-byte với file case riêng.)

### 2.2. Quy ước index

- `payload[k]`: 0-based, `payload[0] = 0x09`.
- `ECX[r]` (0-based C) = `payload[r+1]`.
- `_LStrCopy(ECX, p, n, &tmp)` = Delphi `Copy` 1-based → `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`.
- Codec: `FUN_0077eb9c` = Word LE; `FUN_0077ef7c` = DWORD LE.

### 2.3. Đọc SubOp trong handler (`case_010_0078D4AF_FUN_0078d4af.c` dòng 21–27)

```c
SubOp = (uint)*(byte*)(ECX + 0);  // = payload[1]
if (SubOp==1) ...
else if (SubOp==3) { ss = ECX[1]; ... }  // ss = payload[2], có guard Len<2 → _BoundErr
else if (SubOp==4) ...
else if (SubOp==5) ...
// SubOp 0,2,6..255: không nhánh → no-op, chỉ cleanup chuỗi
```

→ **`SubOp = payload[1]`** (1, 3, 4, 5 có nghĩa); **`SubSubOp ss = payload[2]`** chỉ khi `SubOp==3` (0/1/2 có nghĩa).

---

## 3. Bảng tổng hợp SubOp

| SubOp | Độ dài payload tối thiểu | Nội dung | Đối tượng / Handler |
|:---:|:---:|---|---|
| `0x01` | **2** (`09 01`) | Không trường phụ | self `*(gvar_007DA7BC)` + `func_0x00713308(obj, 1)` |
| `0x03` | **3** (`09 03 ss`) | `ss:1B` = 0/1/2 | `ss==0`: set flag `007DA4A8+0x1e9=1` + `FUN_005d1884`; `ss==1/2`: set flag `=0` + toast message |
| `0x04` | **6** (`09 04 D:4B`) | `D`: DWORD LE | `func_0x0074def4`: scene `+0x5530=GetTickCount()`, `+0x5534=D` |
| `0x05` | **11** (`09 05 D1:4B D2:4B b:1B`) | `D1,D2`: DWORD LE; `b`: byte chọn toast | `func_0x007508e4`: scene `+0x5538=GetTickCount()`, `+0x553c=1`, `+0x5540=D1`, `+0x5544=D2`, toast 5000ms theo `b` |

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — Lệnh loại 1 cho self

**Wire layout (2 bytes):**
```
[0]=0x09 [1]=0x01
```

**Cách đọc (dòng 29–31):** không đọc thêm byte nào, không Word/DWORD, không check Len ngoài guard rỗng.

**Gọi:** `func_0x00713308(*(gvar_007DA7BC), 1)` — `arg1` là object player/self chính (cùng object dùng khắp dispatcher; **không được hàm dùng tới** — đọc global thẳng), `arg2 = 1` hằng.

**Phân tích body (thay ghi chú "không có body" cũ — `00713308_FUN_00713308.c:16-25`, asm kèm theo):**
```c
if (*(char*)(*(int*)gvar_007DA37C + 5) != 0 &&      // cờ "đã nạp playerID" (case_004 OP 0x03 d.66-70 set 1)
    *(char*)(*(int*)gvar_007DA37C + 4) == 0) {      // gate thứ hai (chưa định danh)
  *(char*)(gself + 0x63c) = (char)param_2;          // = 1
  SendCommand(TFConnect = gvar_007D9D30, MainOp=3, SubSel=0);   // asm: XOR ECX,ECX; MOV DL,0x3
}
```
- Nếu cả hai gate thỏa: **ghi byte `gself+0x63c = 1`** (byte ngay sau MapID Word `+0x63a`) rồi **client tự phát C→S `SendCommand(0x03, CL=0)`**. Theo asm builder `case 3` trong `0077f414_FUN_0077F414.asm.txt:183-205`, payload 2 byte dựng ra là **`[0x03][byte gself+0x63c]`** ⇒ wire thực tế `[03][01]`. Đây chính là call-site tường minh đầu tiên của nhánh C→S `case 3` OP 0x03 (trước đây ghi "không tìm thấy" — đã đính chính trong `opcode_03.md` §5).
- Nếu một trong hai gate sai: **không làm gì, không phản hồi gì** (im lặng).
- **Ý nghĩa `+0x63c` và của cặp gate `gvar_007DA37C+5/+4`: chưa kết luận được** — chỉ quan sát được: `+0x63c` còn được đọc trong builder C→S OP 0x05 SubSel 6 (xem `opcode_05.md` §5) → nhiều khả năng là byte "chế độ/xác nhận phiên" cạnh MapID; không đặt tên.

### 4.2. SubOp `0x03` — State-machine + toast (CORE của OP 0x09)

**Wire layout (3 bytes):**
```
[0]=0x09 [1]=0x03 [2]=ss:1B (0x00/0x01/0x02)
```

**Cách đọc (dòng 32–39):** guard `if (Len<2) _BoundErr(1)` đảm bảo có `ECX[1]`, rồi `ss = *(byte*)(ECX+1)` = `payload[2]`. `ss>=3` → no-op.

#### a) `ss == 0` — chạy state-machine (dòng 41–44)

```c
*(*(gvar_007DA4A8) + 0x1e9) = 1;
FUN_005d1884(*(gvar_007DA4A8));
```

- `gvar_007DA4A8` là object form/logic thứ hai (khác player self), offset `0x1e9` là flag byte.
- `FUN_005d1884` (`ts_decompile/functions/005d1884_FUN_005d1884.c` dòng 41–296) — core logic (bỏ graphics/sound, toast chỉ ghi 1 dòng):
  - `param_1[0x79]` là mode/state (`0 / 2 / 3 / ++`).
  - Mode `0`: nếu `param_1[0x42]+0x1a0==0` → init + toast `DAT_005d2088` = **"Hãy cập nhập họ tên"** (bỏ trống input tên); nếu flag `0x1e9==0` → thử `FUN_005d4648`, fail → toast `DAT_005d20a4` = **"Họ tên nhập mang chữ cái không đúng"**, success → **`FUN_0077f414(gvar_007D9D30, 0x09)`** (tự gửi lại OP 9).
  - **`FUN_005d4648(obj, chuỗi)` — đã có body** (`005d4648_FUN_005d4648.c:24-86`): duyệt **bảng 10001 record, stride 0x5C byte tại `gvar_007D9DE4+4`** so `_LStrCmp` với chuỗi nhập; khớp bất kỳ record nào (trước phần tử cuối) → trả 0; hết bảng → gọi thêm `FUN_0051a100(gvar_007DA37C, chuỗi)` và trả 0 nếu hàm đó trả 0. ⇒ kiểm tra tên **không được trùng/không được nằm trong danh sách** (nội dung danh sách runtime, chưa dump).
  - Mode `2/3`: validate 4 slot input `param_1[0x71..0x74]+0x1a0`, so `_LStrCmp`, check độ dài chuỗi `6..10`; mỗi nhánh fail bắn đúng một toast trong bảng 7.2: so cặp 1 (`0x71` vs `0x72`) khác → **"Mật mã 2 tổ không tương đồng"**; cặp 2 (`0x73` vs `0x74`) → **"Mã cá nhân 2 tổ không tương đồng"**; slot 1..4 rỗng → **"Chưa cập nhập dòng mật mã thiết định" / "Xác nhận dòng mật mã chưa được cập nhập" / "Chưa cập nhập dòng Mã cá nhân thiết định" / "Xác nhận dòng Mã cá nhân chưa được cập nhập"**; ngắn <6 → **"Mật mã và Mã cá nhân phải trên 6 chữ cái"**; dài >10 → **"…phải dưới 10 chữ cái"**; mode 2 mà `param_1[0x6f] != 0` → **"Điểm số chưa được phân chia xong"**.
  - Cuối: `param_1[0x79]++`, nếu `>= (flag 0x1eb ? 3 : 4)` thì chốt `param_1[0x43]`, tính 2 số thập phân từ digit bytes tại `param_1[0x3c]+0xcc+0x9b..0xb5` (cộng dồn `*100000000..*1`) vào `param_1[0x5a/0x5b]`, rồi `FUN_0077f414(...,9)` + virtual `(*param_1+0x24)()`. **Hai nhãn SetText khi chốt (flag `0x1eb != 0`)** là literal ASCII thuần `DAT_005d2280 = "opui"` và `DAT_005d2290 = "hjkl"` (4 byte mỗi chuỗi — tổ hợp phím QWERTY; đúng nguyên văn byte, không có dấu): ghi vào field của `param_1[0x72]` và `param_1[0x74]` (`005d1884_FUN_005d1884.c:288-289`).
  - Nói gọn: state-machine `0→4` cho object `007DA4A8`, flag `0x1e9/0x1eb` quyết ngưỡng 3/4, chốt số + gửi `SendCommand(9)`. **Bằng chứng chuỗi mới cho thấy các input được machine kiểm tra là "họ tên / mật mã / mã cá nhân" (4 ô, so từng cặp, độ dài 6..10)** — hợp lệ về mặt dữ liệu; còn việc gắn machine này cho form đăng ký/đổi thông tin cụ thể nào là *suy luận*, chưa kết luận được.

#### b) `ss == 1` (dòng 45–48) và c) `ss == 2` (dòng 49–52)

```c
*(gvar_007DA4A8 + 0x1e9) = 0;
virtual_ShowMessage(gvar_007DA084, &STR_xx, 2000, 0, 0);
```

- `ss==1` toast **`"Tên bị trùng lập, hãy lập lại tên mới"`** (`UNK_00796ec0`, 37 byte @ `0x00796EC0`, dump `redump/lit_796ec0.hex`); `ss==2` toast **`"Tên không hợp lệ, hãy lấy tên khác"`** (`UNK_00796ef0`, 34 byte @ `0x00796EF0`, dump `redump/lit_796ef0.hex`) — **đã decode, xem 7.2** (chính tả "trùng lập/lập lại" là nguyên văn game). Hai nội dung này cho thấy ss=1/2 là **kết quả kiểm tra tên nhân vật** (trùng / không hợp lệ) — *suy luận gắn với đúng 2 chuỗi vừa decode, không bịa thêm*.
- `gvar_007DA084+0x90` là virtual ShowMessage/toast hiển thị 2000ms — gặp ở mọi case. (Presentation: 1 dòng.)

### 4.3. SubOp `0x04` — ghi tick + DWORD vào scene manager — **body đã có (`0074def4_FUN_0074def4.c`)**

**Wire layout (tối thiểu 6 byte):**
```
[0]=0x09 [1]=0x04 [2..5]=D:DWORD (LE)
```

**Cách đọc (case_010 d.54-56):** `func_0x0074def4(*(gvar_007D9D34), ECX)` — param_1 = **object world/scene manager** (`gvar_007D9D34`, alias `DAT_0092322c`), param_2 = RestPayload.

**Body** (`0074def4_FUN_0074def4.c:36-40`):
- `scene + 0x5530 (DWORD) = kernel32.GetTickCount()` (d.36-37).
- `D = Copy(RP,2,4)` decode qua **`FUN_0077ed68`** (codec 4 byte, tương đương `FUN_0077ef7c` — xem `opcode_05.md` §4.4b) → `scene + 0x5534 (DWORD)` (d.38-40).
- **Không đọc thêm byte nào, không chuỗi, không toast, không gửi lại.** Nếu `Len(RP) < 4`, chính codec `FUN_0077ed68` rớt `_BoundErr` (guard từng byte khi đọc tuần tự 4 byte — `0077ed68_FUN_0077ed68.c:82-108`); payload tối thiểu vì thế là `[09][04][4B]`.
- **Diễn giải: chưa kết luận được** nghiệp vụ — chỉ ghi nhận được cơ chế "đóng dấu thời gian nhận + cất 1 giá trị DWORD vào scene manager tại +0x5530/+0x5534" (vùng field ngay sau gate `+0x53fc`; pair field `+0x5538..+0x5544` do SubOp 0x05 ghi). Không gọi tiếp bất kỳ hàm UI/battle nào → mọi tiêu thụ giá trị nằm ở code đọc các offset này (không thuộc phạm vi OP 0x09).

### 4.4. SubOp `0x05` — ghi tick + 2 DWORD + toast 5000 ms — **body đã có (`007508e4_FUN_007508e4.c`)**

**Wire layout (tối thiểu 11 byte):**
```
[0]=0x09 [1]=0x05 [2..5]=D1:DWORD [6..9]=D2:DWORD [10]=b:byte
```

**Cách đọc (case_010 d.57-59):** `func_0x007508e4(*(gvar_007D9D34), ECX)` — cùng object + convention với SubOp 4.

**Body** (`007508e4_FUN_007508e4.c:42-66`):
- `scene+0x5538 = GetTickCount()`; `scene+0x553c = 1` (byte) (d.42-44).
- `D1 = Copy(RP,2,4)` (ef7c) → `scene+0x5540`; `D2 = Copy(RP,6,4)` (ef7c) → `scene+0x5544` (d.45-50).
- `b = RP[9] = p[10]` (guard `Len(RP) ≥ 10` → `_BoundErr(9)`, d.51-57).
- **Toast có điều kiện** (d.58-65): nếu `*(gvar_007DA37C+0x3c) == 0` **và** `*(gvar_007DA1B8 + 0x17c) == 0` thì gọi kênh toast `(**gvar_007DA084 + 0x90)(obj, msg, 5000, 0, 0)` — `msg = DAT_00750a24` nếu `b == 1`, ngược lại `DAT_00750a5c` (**5000 ms**, không phải 2000 ms như các toast SubOp 3).
- **HẸN mới (giới hạn thật, không suy diễn):** hai chuỗi `DAT_00750a24` / `DAT_00750a5c` **chưa có dump** trong `ts_decompile/redump/` (không có `lit_750a24/750a5c.hex`) → nội dung toast của SubOp 0x05 **chưa dịch được**, và hai gate (cờ `gvar_007DA37C+0x3c`, điều kiện `gvar_007DA1B8+0x17c == 0`) **chưa định danh** được form/đối tượng. Ý nghĩa D1/D2/cờ `+0x553c`: **chưa kết luận được**.

---

## 5. Chiều Client → Server (C→S) của OP 0x09

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)` dòng 768–899):

```
case 8: break;   // rỗng
case 9: break;   // <-- dòng 898-899: CLIENT KHÔNG GỬI OP 0x09
case 10: break;
```

**Kết luận: client không bao giờ chủ động gửi OP 0x09 qua `SendCommand`.** Chiều duy nhất là S→C. Ngoại lệ thứ nhất: handler `FUN_005d1884` (dòng 79, 292) gọi `FUN_0077f414(..., 9)` — tức sau khi nhận `S→C [09][03][00]`, client đáp `C→S` OP 0x09 (payload do `005d1884` tự dựng). Không có builder payload C→S nào khác cho OP 9. **Ngoại lệ thứ hai (mới — từ body `00713308`):** sau `S→C [09][01]`, nếu qua được gate `gvar_007DA37C`, client đáp **C→S OP 0x03** (`[03][01]` — xem §4.1 và `opcode_03.md` §5) — không phải OP 9 nhưng là phản hồi tự động phát sinh từ OP 9.

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `F4 44 | Len:Word LE | payload`, socket bytes = plain XOR `0xAD` từng byte.
2. **Bảng frame tính sẵn** (verify công thức với ADR-0001):

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 1 | `09 01` | `F4 44 02 00 09 01` | `59 E9 AF AD A4 AC` |
| SubOp 3 ss0 | `09 03 00` | `F4 44 03 00 09 03 00` | `59 E9 AE AD A4 AE AD` |
| SubOp 3 ss1 | `09 03 01` | `F4 44 03 00 09 03 01` | `59 E9 AE AD A4 AE AC` |
| SubOp 3 ss2 | `09 03 02` | `F4 44 03 00 09 03 02` | `59 E9 AE AD A4 AE AF` |
| SubOp 4 min | `09 04 D1D1D1D1` | `F4 44 06 00 09 04 .. .. .. ..` | XOR AD từng byte |
| SubOp 5 min | `09 05 D1×4 D2×4 bb` | `F4 44 0B 00 09 05 …(9B)` | XOR AD từng byte |

3. **Độ dài:** SubOp 3 bắt buộc đủ 3 bytes (thiếu → `_BoundErr(1)`); SubOp 1 chỉ cần 2 bytes; **SubOp 4 cần đúng 6 bytes** và **SubOp 5 cần đúng 11 bytes** — *(đính chính từ body mới)*: hai handler **không** pass-through vô điều kiện như ghi chú cũ; `FUN_0077ed68`/`_LStrCopy`+guard trong `0074def4`/`007508e4` sẽ `_BoundErr` (runtime error kiểu Delphi) nếu payload cụt. Byte `b` của SubOp 5 (`p[10]`) chỉ quyết định **nhánh toast** (==1 → msg `DAT_00750a24`, khác → msg `DAT_00750a5c` — cả hai chuỗi chưa dump).
4. **Kịch bản test gợi ý:** gửi `09 03 01` / `09 03 02` trước (chỉ toast "Tên bị trùng lập…" / "Tên không hợp lệ…", an toàn) → gửi `09 03 00` (chạy state-machine, có thể phát sinh đáp `C→S` OP 9) → gửi `09 01` (coi chừng: nếu qua gate, client sẽ **tự gửi `[03][01]` C→S** — mock server cần chấp nhận) → cuối cùng thử `09 04 + 4B` / `09 05 + 9B` với payload **đủ độ dài** ở mục 3.
5. **Không cần** mock chiều C→S cho OP 0x09 (ngoại trừ lắng nghe đáp OP 9 từ `005d1884` sau `09 03 00`, và đáp **OP 0x03** `[03][01]` từ `00713308` sau `09 01` khi qua gate `gvar_007DA37C`).

---

## 7. Chuỗi hiển thị / mã hóa tiếng Việt — **ĐÃ DUMP + ĐÃ GIẢI MÃ (2026-09-14)**

**Kết luận mới: OP 0x09 có 17 tham chiếu chuỗi (15 cũ + 2 mới từ handler SubOp 5); 15/17 đã dump và giải mã được; 2 chuỗi của SubOp 5 (`0x750a24/0x750a5c`) vẫn chưa có dump.**

### 7.1. Bảng mã thực đo (đính chính)

**(đính chính quan trọng — lật lại kết luận cũ của mục này)**: Tiền lệ `opcode_02.md` mục 5 gọi bảng mã của game là "cp1258 → NFC". Khi bytes **thật** của 15 chuỗi toast về tay, phép thử cho thấy:
- Pipeline `bytes.decode('cp1258') + NFC` của Python trả kết quả **vô nghĩa**: `48 E3 79… 63 A7 70…` → `"Hăy c§p nh§p h÷ tên"` (Windows-1258 chuẩn là bảng *tách âm*: `ậ` phải là `0xE2 0xF2`, không có ký tự tiền tổ hợp ở `0xA7`).
- Bảng **đơn-byte tiền tổ hợp VISCII/TCVN-6909 (RFC 1456)** đọc **chính xác 100% cả 15 chuỗi thành tiếng Việt có nghĩa** (và khớp hoàn toàn các neo đã chốt ở `opcode_13.md:174`: `E4=ả, E1=á, F0=đ, B6=ờ, F6=ỏ, FB=ũ`; thêm: `A7=ậ, E3=ã, F7=ọ, AF=ố, AE=ệ, B4=Ơ…`).
⇒ **Mã hóa đúng của game là bảng đơn-byte tiền tổ hợp kiểu VISCII, KHÔNG phải Windows-1258 decomposed.** Kết luận "không phải VISCII" trong bản 2026-09-12 của tài liệu này là **sai** (sai vì khi đó chưa có bytes để thử — phương pháp "không suy đoán khi chưa có bytes" đã đúng, nhưng nhãn cp1258 vay từ tiền lệ chưa kiểm chứng thì sai). Lưu ý chính tả game không chuẩn: "cập nhập" (=cập nhật), "trùng lập" (=trùng lặp) — giữ nguyên văn theo byte.

### 7.2. Danh sách chuỗi trong OP 0x09 (đã decode — dump `ts_decompile/redump/`)

Cách giải mã: `redump/lit_XXXXXXXX.hex` = bytes tính **tại chính địa chỉ nội dung** (DAT pointer trỏ tới); một *run* AnsiString hằng đóng gói: chuỗi đầu = raw bytes tới `00`, sau đó lặp `[FF FF FF FF refcount][len:4LE][bytes][00]`. Offset tuyệt đối = base file + offset byte đầu nội dung. Đã dùng script Python nhỏ parse + decode **VISCII → NFC** (xem 7.1).

| # | Địa chỉ nội dung | len (B) | Chuỗi (VISCII→NFC, nguyên văn game) | Nơi tham chiếu / Ngữ cảnh |
|---|---|---|---|---|
| 1 | `0x005D2088` | 19 | `Hãy cập nhập họ tên` | `005d1884.c:68` — ss0, mode 0, ô họ tên rỗng |
| 2 | `0x005D20A4` | 35 | `Họ tên nhập mang chữ cái không đúng` | `005d1884.c:75` — `FUN_005d4648` trả 0 (verify fail) |
| 3 | `0x005D20D0` | 32 | `Điểm số chưa được phân chia xong` | `005d1884.c:85` — mode 2, `param_1[0x6f] != 0` |
| 4 | `0x005D20FC` | 28 | `Mật mã 2 tổ không tương đồng` | `005d1884.c:109` — `_LStrCmp` cặp slot `0x71` vs `0x72` khác nhau |
| 5 | `0x005D2124` | 32 | `Mã cá nhân 2 tổ không tương đồng` | `005d1884.c:115` — cặp slot `0x73` vs `0x74` khác nhau |
| 6 | `0x005D2150` | 36 | `Chưa cập nhập dòng mật mã thiết định` | `005d1884.c:120` — slot `0x71` rỗng |
| 7 | `0x005D2180` | 39 | `Xác nhận dòng mật mã chưa được cập nhập` | `005d1884.c:125` — slot `0x72` (ô xác nhận mật mã) rỗng |
| 8 | `0x005D21B0` | 40 | `Chưa cập nhập dòng Mã cá nhân thiết định` | `005d1884.c:130` — slot `0x73` rỗng |
| 9 | `0x005D21E4` | 43 | `Xác nhận dòng Mã cá nhân chưa được cập nhập` | `005d1884.c:135` — slot `0x74` rỗng |
| 10 | `0x005D2218` | 40 | `Mật mã và Mã cá nhân phải trên 6 chữ cái` | `005d1884.c:147` — có chuỗi dài < 6 |
| 11 | `0x005D224C` | 41 | `Mật mã và Mã cá nhân phải dưới 10 chữ cái` | `005d1884.c:161` — có chuỗi dài > 10 |
| 12 | `0x005D2280` | 4 | `opui` (ASCII thuần) | `005d1884.c:288` — `SetText` vào field `param_1[0x72]` khi chốt (flag `0x1eb != 0`) |
| 13 | `0x005D2290` | 4 | `hjkl` (ASCII thuần) | `005d1884.c:289` — `SetText` vào field `param_1[0x74]` |
| 14 | `0x00796EC0` | 37 | `Tên bị trùng lập, hãy lập lại tên mới` | `case_010...c:47` — SubOp 3 ss==1 (toast 2000ms, cờ `0x1e9=0`) |
| 15 | `0x00796EF0` | 34 | `Tên không hợp lệ, hãy lấy tên khác` | `case_010...c:51` — SubOp 3 ss==2 |
| 16 | `0x00750A24` | ? | **CHƯA DUMP** | `007508e4.c:61` — SubOp 5, toast 5000ms khi byte `p[10] == 1` |
| 17 | `0x00750A5C` | ? | **CHƯA DUMP** | `007508e4.c:64` — SubOp 5, toast 5000ms khi byte `p[10] != 1` |

Ghi chú pack (bằng chứng cấu trúc): #1–#13 nằm trong **một run liên tục** tại `0x5D2088..0x5D2294` (`lit_5d2088.hex` chứa trọn run; các file `lit_5d20a4.hex`…`lit_5d2290.hex` chỉ là các window cùng run — mọi chuỗi xuất hiện trùng khớp giữa các file, xác minh chéo offset); #14/#15 là **hai chuỗi đầu của một run khác** bắt đầu tại `0x796EC0` (`lit_796ec0.hex`/`lit_796ef0.hex`) — các chuỗi #16+# tiếp theo của run đó (`0x796F1C "Sử dụng tự động bổ huyết đan"`, `0x796F44 "lần"`, `0x796F50 "Đối phương đang bận rộn"`, `0x796FBC "sound\m004.wav"`, `0x796FD4 "Bạn chơi"`, `0x796FE8 "Tiếp nhận lời mời bạn hữu"`, `0x79700C "Cự tuyệt gia nhễp bạn hũu"`, `0x797030 "Không hồi ứng"`, `0x797048 "Thùng thư bạn hữu đã đầy"`, `0x79706C "Thật thông minh...Chúc mừng bạn đã trả lời đúng!!"`, `0x7970A8 "Ồ...bạn trả lời sai rồi!"`, `0x7970CC "Ồ... Bạn đã sai đến lần thứ 3 rồi!"`) **thuộc các OP khác** (0x0D/0x0E/0x17/0x18/0x2A/0x2C — theo `missing_opcode_sources.md` §2) nên chỉ ghi nhận ở đây làm bằng chứng giải mã, không kết luận cho OP 0x09.

Cách gọi chung: `(virtual gvar_007DA084+0x90)(obj, &STR, ms, 0, 0)` — toast (2000ms cho SubOp 3, **5000ms cho SubOp 5**). Đây là **presentation thuần túy**, không ảnh hưởng core logic mock server.

### 7.3. Trạng thái còn treo (thành thật với single source)

- 15/17 chuỗi **đã decode xong** (mục 7.2) — các dòng "chưa dump → chưa dịch được" của bản cũ đã được thay bằng nội dung thật; không còn mục nào "chưa dịch" trong dải `0x5D20xx/0x796Exx`.
- **Còn thiếu:** đúng 2 chuỗi SubOp 5 (`DAT_00750a24`, `DAT_00750a5c` — literal nằm lọt trong `.text` ngay sau `FUN_007508e4`, kết thúc `0x750A10`) chưa có `lit_750a24/750a5c.hex` → **chưa dịch được, không bịa**.
- Ba handler trước đây bị coi là "chứa chuỗi riêng không kiểm chứng được" (`00713308/0074def4/007508e4`): hai hàm đầu xác nhận **không** tham chiếu chuỗi nào; chỉ `007508e4` có chuỗi (2 mục #16/#17).

### 7.4. Cần làm gì để đóng nốt (bước tiếp theo)

1. Từ binary gốc dump 2 AnsiString/PChar tại `0x00750A24` và `0x00750A5C` (tới null-terminator) → đặt `ts_decompile/redump/lit_750a24.hex`, `lit_750a5c.hex`.
2. Giải mã theo **bảng đơn-byte tiền tổ hợp VISCII** (RFC 1456) — *không dùng pipeline cp1258 nữa* (xem đính chính 7.1; cùng hướng đã chốt ở `opcode_13.md:174`). Sau khi có bytes, điền #16/#17 và cập nhật §4.4.
3. ~~Các doc khác đang ghi "cp1258 → NFC" cần rà lại~~ **ĐÃ RÀ XONG 2026-09-14**: `opcode_02.md §5` đã đính chính nhãn recipe; các doc dùng bytes mới (`09/0b/0c/0d/0e/0f/13/17/18/19/1a/1d/25/2a/2c/3a/3b/3d/42/00_01`) đều đã chuyển sang decode VISCII.

---

## 8. Source trail (chỉ `ts_decompile/` + `docs/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` | dòng 16–24, 39–49, 112–141 | Framing, dispatcher, mapping `0x09 → Case 10 → 0x0078D4AF`, codec helpers |
| 2 | `ts_decompile/case_functions/functions/case_010_0078D4AF_FUN_0078d4af.c` | toàn file 91 dòng; SubOp 21–27; ss 33–39; nhánh 29–59 | Switch SubOp/SubSubOp, wire tối thiểu |
| 3 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 9:` 2130–2178 | Xác minh inline 1:1 |
| 4 | `ts_decompile/case_functions/manifest.csv` | dòng 12 | Case index 10, entry 0x0078A9DE |
| 5 | `ts_decompile/functions/005d1884_FUN_005d1884.c` | 41–296 (mode 64–82, gửi 9 tại 79/292, tính số 186–285) | State-machine của SubOp 3 ss0 |
| 6 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `switch` 768; `case 9` 898–899 | Kết luận không có chiều C→S chủ động |
| 7 | `ts_decompile/functions/0077eb9c / 0077ef7c` | decode Word/DWORD LE | Codec |
| 8 | `docs/adr/0001-client-prediction-not-authority.md` | 24, 64–85 | Frame/XOR/Len |
| 9 | `ts_decompile/functions/00713308_FUN_00713308.c` (+`.asm.txt`) | d.16–25 | Handler SubOp 1 — **đã có body (2026-09-14)**: gate `gvar_007DA37C+5/+4`, ghi `gself+0x63c`, `SendCommand(3)` |
| 9b | `ts_decompile/functions/0074def4_FUN_0074def4.c` | d.36–40 | Handler SubOp 4 — scene `+0x5530/0x5534` |
| 9c | `ts_decompile/functions/007508e4_FUN_007508e4.c` | d.42–66 | Handler SubOp 5 — scene `+0x5538..0x5544` + toast 5000ms `DAT_00750a24/00750a5c` (**2 chuỗi này chưa dump — giới hạn duy nhất còn lại**) |
| 9d | `ts_decompile/functions/005d4648_FUN_005d4648.c` | d.24–86 | Kiểm tên: duyệt bảng 10001 record tại `gvar_007D9DE4+4` + `FUN_0051a100` |
| 9e | `ts_decompile/redump/lit_5d2088.hex` … `lit_5d2290.hex`, `lit_796ec0.hex`, `lit_796ef0.hex` (decode VISCII — script, xem 7.1/7.2) | offset 0x5D2088–0x5D2294, 0x796EC0–0x796EF0+ | 15 chuỗi toast đã giải mã (7.2) |

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x0078D4AF` — C `client_pseudo_c/case_010_0078D4AF_FUN_0078d4af.c` (có); asm `client_pseudo_c/0078d4af_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x09` — 2 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| — | `0x0078D4AF` | Tên bị trùng lập, hãy lập lại tên mới | Tên đã tồn tại. Vui lòng chọn tên mới. | Hiệu đính ngữ nghĩa/câu chữ |
| — | `0x0078D4AF` | Tên không hợp lệ, hãy lấy tên khác | Tên không hợp lệ. Vui lòng chọn tên khác. | Hiệu đính ngữ nghĩa/câu chữ |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x0078D4AF`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
