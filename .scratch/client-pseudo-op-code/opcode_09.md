# PHÂN TÍCH — Main OP 0x09 (Case 10) `FUN_0078d4af` @ `0x0078D4AF` — **Lệnh điều khiển phiên / State-machine phụ (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code`
Trạng thái: **Đã xác minh từ decompile case function + dispatcher inline**. OP 0x09 có **4 SubOp hoạt động (1, 3, 4, 5)**; SubOp 3 có thêm **SubSubOp (ss = 0/1/2)**. Hai handler của SubOp 4/5 **không có body trong ts_decompile** (ghi rõ giới hạn, không suy diễn).

---

## 1. Tóm tắt nghiệp vụ

| SubOp | Bản chất | Handler |
|---|---|---|
| `0x01` | Lệnh loại 1 cho object self, chỉ mang hằng `1` | `func_0x00713308(*(gvar_007DA7BC), 1)` — ngoài phạm vi decompile |
| `0x03` | State-machine của object `007DA4A8` + toast (ss=0 chạy machine, ss=1/2 báo message) | inline + `FUN_005d1884` |
| `0x04` | Pass-through rest payload cho object `007D9D34` | `func_0x0074def4` — không có body |
| `0x05` | Pass-through rest payload cho object `007D9D34` | `func_0x007508e4` — không có body |

OP 0x09 là **100% server-push** (client không chủ động gửi, xem mục 5). Ngoại lệ duy nhất: sau khi nhận `09 03 00`, handler `FUN_005d1884` tự gọi `SendCommand(9)` đáp lại.

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
| `0x04` | **≥2** (`09 04 rest...`) | Rest biến thiên, pass nguyên ECX | `func_0x0074def4(*(gvar_007D9D34), ECX)` |
| `0x05` | **≥2** (`09 05 rest...`) | Rest biến thiên, pass nguyên ECX | `func_0x007508e4(*(gvar_007D9D34), ECX)` |

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — Lệnh loại 1 cho self

**Wire layout (2 bytes):**
```
[0]=0x09 [1]=0x01
```

**Cách đọc (dòng 29–31):** không đọc thêm byte nào, không Word/DWORD, không check Len ngoài guard rỗng.

**Gọi:** `func_0x00713308(*(gvar_007DA7BC), 1)` — `arg1` là object player/self chính (cùng object dùng khắp dispatcher), `arg2 = 1` hằng.

**Giới hạn source:** hàm `0x00713308` **không có body trong `ts_decompile/`** (grep chỉ trúng 3 call-site: file case + jumptable + dispatcher). File lân cận `00713354` là hàm khác (fill map đệ quy), không dùng. → Tại tầng case chỉ kết luận "server ra lệnh loại 1 cho self"; chi tiết nằm ngoài decompile, không suy diễn.

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
  - Mode `0`: nếu `param_1[0x42]+0x1a0==0` → init + toast `DAT_005d2088`; nếu flag `0x1e9==0` → thử `FUN_005d4648`, fail → toast `DAT_005d20a4`, success → **`FUN_0077f414(gvar_007D9D30, 0x09)`** (tự gửi lại OP 9).
  - Mode `2/3`: validate 4 slot `param_1[0x71..0x74]+0x1a0`, so `_LStrCmp`, check độ dài chuỗi `6..10`, toast tương ứng.
  - Cuối: `param_1[0x79]++`, nếu `>= (flag 0x1eb ? 3 : 4)` thì chốt `param_1[0x43]`, tính 2 số thập phân từ digit bytes tại `param_1[0x3c]+0xcc+0x9b..0xb5` (cộng dồn `*100000000..*1`) vào `param_1[0x5a/0x5b]`, rồi `FUN_0077f414(...,9)` + virtual `(*param_1+0x24)()`.
  - Nói gọn: state-machine `0→4` cho object `007DA4A8`, flag `0x1e9/0x1eb` quyết ngưỡng 3/4, chốt số + gửi `SendCommand(9)`.

#### b) `ss == 1` (dòng 45–48) và c) `ss == 2` (dòng 49–52)

```c
*(gvar_007DA4A8 + 0x1e9) = 0;
virtual_ShowMessage(gvar_007DA084, &STR_xx, 2000, 0, 0);
```

- `ss==1` dùng chuỗi `UNK_00796ec0`, `ss==2` dùng `UNK_00796ef0` (resource string chưa resolve tên, chỉ biết là 2 message khác nhau).
- `gvar_007DA084+0x90` là virtual ShowMessage/toast hiển thị 2000ms — gặp ở mọi case. (Presentation: 1 dòng.)

### 4.3. SubOp `0x04` — Pass-through rest payload

**Wire layout:** `09 04 rest...` (≥2 bytes; tầng case không tách trường nào).

**Cách đọc (dòng 54–56):** `func_0x0074def4(gvar_007D9D34_obj, ECX)` — truyền nguyên chuỗi ECX, không đọc `ECX[1]`, không Word/DWORD. Mọi parse nằm trong handler.

**Giới hạn source:** `func_0x0074DEF4` **không có file body** trong `ts_decompile/functions/` (ls/grep chỉ trúng call-site). Signature suy từ call: `(gvar_007D9D34_obj, RestPayload_str)`. `gvar_007D9D34` là object dùng chung cho SubOp 4/5 và nhiều handler battle/team. → Mock chỉ cần forward đúng rest nguyên vẹn; tầng case không crash (không có `_BoundErr`).

### 4.4. SubOp `0x05` — Pass-through rest payload

**Wire layout:** `09 05 rest...` — giống hệt 0x04.

**Cách đọc (dòng 57–59):** `func_0x007508e4(gvar_007D9D34_obj, ECX)`.

**Giới hạn source:** `func_0x007508E4` cũng **không có body** (lý do như trên). Cùng object, cùng convention pass-through.

---

## 5. Chiều Client → Server (C→S) của OP 0x09

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)` dòng 768–899):

```
case 8: break;   // rỗng
case 9: break;   // <-- dòng 898-899: CLIENT KHÔNG GỬI OP 0x09
case 10: break;
```

**Kết luận: client không bao giờ chủ động gửi OP 0x09 qua `SendCommand`.** Chiều duy nhất là S→C. Ngoại lệ duy nhất là handler `FUN_005d1884` (dòng 79, 292) gọi `FUN_0077f414(..., 9)` — tức sau khi nhận `S→C [09][03][00]`, client đáp `C→S` OP 0x09 (payload do `005d1884` tự dựng). Không có builder payload C→S nào khác cho OP 9.

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
| SubOp 4 min | `09 04` | `F4 44 02 00 09 04` | `59 E9 AF AD A4 A9` |
| SubOp 5 min | `09 05` | `F4 44 02 00 09 05` | `59 E9 AF AD A4 A8` |

3. **Độ dài:** SubOp 3 bắt buộc đủ 3 bytes (thiếu → `_BoundErr(1)`); SubOp 1 chỉ cần 2 bytes; SubOp 4/5 thực tế server gửi dài hơn 2 bytes — mock cứ gửi `09 04/05 + rest tùy ý`, client pass-through nên không crash ở tầng case.
4. **Kịch bản test gợi ý:** gửi `09 03 01` / `09 03 02` trước (chỉ toast, an toàn) → gửi `09 03 00` (chạy state-machine, có thể phát sinh đáp `C→S` OP 9) → gửi `09 01` → cuối cùng thử `09 04/05` với rest rỗng.
5. **Không cần** mock chiều C→S cho OP 0x09 (ngoại trừ lắng nghe đáp từ `005d1884` sau `09 03 00`).

---

## 7. Chuỗi hiển thị / mã hóa tiếng Việt (VISCII hay không?)

**Kết luận: OP 0x09 CÓ 15 tham chiếu chuỗi toast, nhưng KHÔNG dịch được từ single source hiện tại — và mã hóa đúng là cp1258, không phải VISCII.**

### 7.1. Vì sao không phải VISCII

Tiền lệ đã xác minh ở `opcode_02.md` mục 5: các nhãn kênh chat dump từ `ts_decompile/redump/lit_7ABD*.hex` (Delphi ansistring `[len:4LE][chars]`) giải mã đúng bằng **cp1258 → NFC** (ví dụ byte `F4 AF AE` → "ố ệ"). Không có bằng chứng nào trong `ts_decompile/` cho thấy game dùng VISCII. Mọi chuỗi toast của OP 0x09 (nếu dump được) cũng phải giải mã theo **cp1258 → NFC**, không phải VISCII → UTF-8.

### 7.2. Danh sách chuỗi trong OP 0x09 (địa chỉ + trạng thái dump)

Payload trên wire của OP 0x09 **không mang text** (chỉ `09 | SubOp | ss`) — text nằm ở **hằng số trong binary**, client chỉ hiển thị khi nhận đúng SubOp:

| # | Địa chỉ | Nơi tham chiếu | Ngữ cảnh | Trạng thái |
|---|---|---|---|---|
| 1 | `DAT_005d2088` | `005d1884.c:68` (ss0, mode 0, init) | Toast khi `param_1[0x42]+0x1a0==0` | **Chưa có dump `lit_5d2088.hex` → chưa dịch được** |
| 2 | `DAT_005d20a4` | `005d1884.c:75` (ss0, verify fail) | Toast khi `FUN_005d4648` trả 0 | Chưa có dump → chưa dịch được |
| 3 | `DAT_005d20d0` | `005d1884.c:85` (mode 2) | Toast nhánh `0x6f != 0` | Chưa có dump → chưa dịch được |
| 4 | `DAT_005d20fc` | `005d1884.c:109` | Toast so sánh cặp 1 khác nhau | Chưa có dump → chưa dịch được |
| 5 | `DAT_005d2124` | `005d1884.c:115` | Toast so sánh cặp 2 khác nhau | Chưa có dump → chưa dịch được |
| 6 | `DAT_005d2150` | `005d1884.c:120` | Toast slot 1 rỗng | Chưa có dump → chưa dịch được |
| 7 | `DAT_005d2180` | `005d1884.c:125` | Toast slot 2 rỗng | Chưa có dump → chưa dịch được |
| 8 | `DAT_005d21b0` | `005d1884.c:130` | Toast slot 3 rỗng | Chưa có dump → chưa dịch được |
| 9 | `DAT_005d21e4` | `005d1884.c:135` | Toast slot 4 rỗng | Chưa có dump → chưa dịch được |
| 10 | `DAT_005d2218` | `005d1884.c:147` | Toast độ dài < 6 | Chưa có dump → chưa dịch được |
| 11 | `DAT_005d224c` | `005d1884.c:161` | Toast độ dài > 10 | Chưa có dump → chưa dịch được |
| 12 | `DAT_005d2280` | `005d1884.c:288` | `SetText (FUN_007b372c)` khi chốt số, flag `0x1eb != 0` | Chưa có dump → chưa dịch được |
| 13 | `DAT_005d2290` | `005d1884.c:289` | `SetText` cặp còn lại | Chưa có dump → chưa dịch được |
| 14 | `UNK_00796ec0` | `case_010.c:47` (ss==1) | Toast 2000ms, flag `0x1e9=0` | Chưa có dump `lit_796ec0.hex` → chưa dịch được |
| 15 | `UNK_00796ef0` | `case_010.c:51` (ss==2) | Toast 2000ms, flag `0x1e9=0` | Chưa có dump → chưa dịch được |

Cách gọi chung: `(virtual gvar_007DA084+0x90)(obj, &STR, 2000, 0, 0)` — toast hiển thị 2000ms. Đây là **presentation thuần túy**, không ảnh hưởng core logic mock server.

### 7.3. Vì sao chưa dịch được (trung thực với single source)

- `ts_decompile/redump/` hiện chỉ có dump vùng `0x595…`, `0x77F771`, `0x7A2…`, `0x7AB…` — **không có file nào cho vùng `0x5D20xx` hay `0x796Exx`**.
- Decompile `.c` chỉ giữ **địa chỉ tham chiếu** (`&DAT_005d2088`), không giữ byte chuỗi. File `.asm.txt` cũng chỉ có `MOV EDX,0x5d2088`, không có bytes.
- Không suy đoán nội dung tiếng Việt khi chưa có bytes (tránh bịa đặt). Handler vắng mặt `00713308 / 0074def4 / 007508e4` cũng có thể chứa chuỗi riêng, nhưng không có body nên càng không thể kết luận.

### 7.4. Cần làm gì để dịch được (bước tiếp theo)

1. Từ binary gốc (`aLogin.exe`) dump Delphi ansistring tại 15 địa chỉ trên (mỗi chuỗi: `[len:4LE][bytes][00]`), lưu thành `ts_decompile/redump/lit_5d2088.hex`, `lit_5d20a4.hex`, …, `lit_796ec0.hex`, `lit_796ef0.hex` — đúng quy ước như `lit_7ABD54.hex`.
2. Giải mã mỗi dump bằng **cp1258 → NFC** (Python: `bytes.decode('cp1258')` + `unicodedata.normalize('NFC', s)`), đối chiếu mẫu `opcode_02.md` mục 5.
3. Khi có bytes, cập nhật bảng 7.2 thêm 2 cột `Byte` và `Tiếng Việt (NFC)` + cột `Vai trò`, rồi mới điền vào mục 4.2 của tài liệu này.

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
| 9 | Giới hạn ghi rõ: `00713308 / 0074def4 / 007508e4` | chỉ trúng call-site, `ls functions/ \| grep 0074de\|007508` rỗng | Không suy diễn ngoài source |
