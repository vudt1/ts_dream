# PHÂN TÍCH — Main OP 0x17 (Case 20, `FUN_007902fb` @ `0x007902FB`) — **Bus đa năng 97 SubOp, pass-through + âm thanh/toast/chat (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ không có body/dump ghi rõ, không suy diễn.

---

## 0. Tóm tắt nghiệp vụ

**OP 0x17 là bus đa năng lớn nhất đã gặp: 1 byte SubOp chọn 1 trong 97 đường.** Hầu hết là `pass-through nguyên RestPayload` cho hàm con `(obj_global, RP)` — logic thật nằm trong callee. Chỉ 9 đường đọc được logic ở tầng case:

- SubOp `0x0F / 0x24` = nối `"sound\\WA0014.wav"` + `FUN_007a7f20` (phát âm thanh, 1 dòng presentation).
- SubOp `0x11` = call + rẽ có điều kiện `*(short... )==-0x4C4A`.
- SubOp `0x15/0x16/0x17` = call + dọn `TFollowNpc` + call UI.
- SubOp `0x19 / 0x3B` = toast VMT `gvar_007DA084+0x90` với hằng `UNK_00797ee8 / UNK_00797f1c`.
- SubOp `0x27` = đọc 1 byte `RP[1]` + `FUN_00734610` (có body).
- SubOp `0x29` = `_LStrFromChar(RP[1])` + `FUN_0077eaa4` (check `==1`) → `*(gvar_007DA7BC+0x1434)`.
- SubOp `0x72` = đường duy nhất dùng codec Word LE: `Copy(RP,2,2)` → `FUN_0077eb9c` → `FUN_00774a84` → `Format(UNK_00797f64)` → `FUN_007ab870` (chat hệ thống).
- Còn lại ~88 đường = hộp đen.

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_020_007902FB_FUN_007902fb.c:8` — `void FUN_007902fb(void)` @ `0x007902FB`, 421 dòng.
- `ts_decompile/case_functions/manifest.csv:22` — `20,0x0078AA06,0x007902FB,EXPORTED,"FUN_007902fb"`.
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 20 (khung cleanup, body nằm file case riêng).

**Dispatcher `FUN_0078a89c`:**
- `ts_decompile/functions/0078a89c_FUN_0078a89c.c:27` — `param_2` = **MainOp**, `param_1` = **RestPayload đã cắt MainOp**.
- `switch` top-level; nhánh **`case 0x17:` tại dòng 3806–4268** chứa switch SubOp con. Đã đối chiếu: nhãn trong khối khớp từng nhãn với file case riêng (file case 97 nhãn, dispatcher khối này 97 nhãn SubOp + nhãn MainOp). Single source of truth là file case.
- Vòng khóa dòng 561–565: `iVar21 = 0xad` = dấu vết **khóa XOR tĩnh `0xAD`**.

**Mapping MainOp 0x17 → Case 20 (xác minh 3 nguồn):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` dòng 2 → `byte_table[0x17] = 0x14` = 20 thập phân = Case 20.
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex`: entry index 20 = `FB 02 79 00` = `0x007902FB` (LE).
- `manifest.csv:22` + jump-table entry `0x0078AA06` → khớp.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR `0xAD` từng byte. Server speaks first.

**Cách đọc SubOp (file case dòng 29–37):**
```c
iVar6 = *(int *)(unaff_EBP + -0xc);   // ECX = RestPayload
if (*(int *)(iVar6 + -4) == 0) _BoundErr(0); // guard RestLen>=1
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6 + 0); // SubOp = ECX[0]
switch(SubOp) { case 2 ... case 0x75 ... }
```
- `unaff_EBP-0xc` = RestPayload = payload gốc bỏ `P[0]=MainOp`.
- **KẾT LUẬN: CÓ SubOp. SubOp = `payload[1]` = `ECX[0]`**, 1 byte thường, không codec.
- Quy ước `_LStrCopy(ECX,p,n)` = Delphi `Copy` **1-based**: `ECX[p-1..p+n-2]` = `payload[p..p+n-1]`. Ví dụ `Copy(ECX,2,4)` = `payload[2..5]`.

**Codec helper (đã đọc body):**
- `FUN_0077eb9c`: `b0+b1*0x100` = **Word LE**. Handler 0x17 chỉ gọi **1 lần** (SubOp `0x72`).
- `FUN_0077ef7c`: `b0+b1*256+b2*65536+b3*2^24` = **DWORD LE**. **Handler 0x17 không gọi lần nào** (ghi rõ để tránh nhầm).
- `FUN_0077eb1c` (encode Word→chuỗi 2B), `FUN_0077ee84` (encode DWORD→chuỗi 4B): chỉ dùng chiều C→S, **không dùng ở S→C này**.
- `FUN_0077f098`: decode khối 8 byte. **Handler không gọi**.
- `FUN_0077eaa4`: `return (byte0 == 1)` — check boolean 1 byte. Dùng ở SubOp `0x29`.
- `FUN_00774a84`: tra bảng `gvar_007DA540` theo Word id → chuỗi. Dùng ở SubOp `0x72`.
- `FUN_007a7f20`: `FileExists` + phát âm thanh (presentation 1 dòng). Dùng ở SubOp `0x0F/0x24`.
- `FUN_007ab870`: chèn dòng chat hệ thống `(form, id, msg, tag)`. Dùng ở SubOp `0x6D/0x72`.

---

## 2. Đối chiếu dispatcher inline

Khối `case 0x17:` dòng 3806–4268 trong `0078a89c_FUN_0078a89c.c` khớp nhãn 1:1 với file case (file case 97 nhãn `case 2...0x75 + case 100`; dispatcher khối này chứa đúng 97 nhãn SubOp đó + nhãn MainOp bao ngoài). Dispatcher không thêm logic; single source là file case.

---

## 3. Bảng tổng hợp toàn bộ SubOp

`switch(SubOp)` có **97 labels**: `2,3,4,5,6,7,8,9,10,0xB,0xC,0xD,0xF,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1A,0x1B,0x1C,0x1E,0x1F,0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2A,0x2B,0x2C,0x2D,0x2E,0x2F,0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,0x3A,0x3B,0x3C,0x3D,0x3E,0x3F,0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4C,0x4D,0x4E,0x4F,0x50,0x51,0x52,100(0x64),0x65,0x66,0x67,0x68,0x69,0x6A,0x6B,0x6C,0x6D,0x6E,0x6F,0x70,0x71,0x72,0x73,0x74,0x75`.
**Không có `case 0,1,0x0E,0x1D,0x53–0x63`, không có `default`** → các SubOp đó rơi qua, chỉ cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x17`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

Độ dài tối thiểu: đa số chỉ cần guard `RestLen>=1` ở đầu → **payload tối thiểu 2B (`17 sub`)**. Ngoại lệ:
- SubOp `0x27`: guard `Len<2→BoundErr(1)` → payload ≥3B.
- SubOp `0x29`: `_LStrFromChar(RP[1])` → payload ≥3B.
- SubOp `0x72`: `_LStrCopy(RP,2,2)` → payload ≥4B (`17 72 xx xx`).

| SubOp | Payload min | Wire | Handler tầng case |
|---|---|---|---|
| `0x02`–`0x0D` (trừ `0x0E` vắng) | 2 | `[17][sub]` pass RP | từng `func_0x0076xxxx/0077xxxx/0062xxxx` — hộp đen |
| `0x0F` | 2 | `[17][0F]` không field | nối `"sound\\WA0014.wav"` + `FUN_007a7f20` (âm thanh 1 dòng) |
| `0x10` | 2 | pass | `FUN_0077c64c(gvar_007D9E8C, RP)` — có body |
| `0x11` | 2 | pass + rẽ cờ | `func_0x0076c248(...)` + `if (*(short... )==-0x4C4A) func_0x00603d48(...,4)` |
| `0x12`–`0x14` | 2 | pass | hộp đen |
| `0x15`/`0x16`/`0x17` | 2 | pass + dọn pet | call riêng + cụm chung dọn `TFollowNpc` + UI |
| `0x18`–`0x1C` (trừ `0x1D` vắng) | 2 | pass | hộp đen |
| `0x19` | 2 | `[17][19]` | toast VMT `UNK_00797ee8` 1200ms |
| `0x1E`–`0x26` | 2 | pass (riêng `0x26` truyền cả buffer thô + param_3) | hộp đen / `FUN_0073439c` (có body) |
| `0x27` | **3** | `[17][27][b:P2=RP1,1B]` | `FUN_00734610(gvar_007DA7BC, b)` — có body |
| `0x28` | 2 | pass | hộp đen |
| `0x29` | **3** | `[17][29][b:P2,1B]` | `_LStrFromChar(b)` + `FUN_0077eaa4` → `*(gvar_007DA7BC+0x1434)` |
| `0x2A` | 2 | `[17][2A]` | `FUN_0073c690(gvar_007DA7BC)` — bỏ qua payload |
| `0x2B`–`0x52` (trừ khuyết) | 2 | pass nguyên RP | từng `func_0x.../FUN_0x...` — hộp đen |
| `100 (0x64)`, `0x65`–`0x71`, `0x73`–`0x75` | 2 | pass | hộp đen |
| `0x3B` | 2 | `[17][3B]` | toast VMT `UNK_00797f1c` 2000ms |
| `0x41` | 2 | `[17][41]` | `*(gvar_007DA7BC+0x1453) = 0` — clear cờ |
| `0x6D` | 2 | `[17][6D]` | `FUN_007ab870(TTalkMsgForm,0,&UNK_00797f38,0)` (chat 1 dòng) |
| `0x72` | **4** | `[17][72][w:P2..P3 Word LE]` | `Copy(RP,2,2)`→Word LE→tra tên→`Format(UNK_00797f64)`→`FUN_007ab870` |

---

## 4. Chi tiết từng SubOp (wire + core, graphics 1 dòng)

- **Nhóm pass-through thuần (không tách field ở tầng case):** đa số 97 nhánh. Wire `[17][sub][rest...]`, core = 1 call `(gvar, RP)`. Toàn bộ `func_0x0076xxxx/0077xxxx/0074xxxx/0060xxxx/0063xxxx/0052xxxx/005cxxxx` trong nhóm này **không có file body trong `ts_decompile/functions/`** → không suy diễn parse bên trong; Mock chỉ forward rest.
- **Có body (tóm 1 dòng hiển thị):** `FUN_0077c64c`, `FUN_00731388`, `FUN_0073439c`, `FUN_00734610`, `FUN_0073c690` và các helper `FUN_00774a84`, `FUN_0077eaa4`, `FUN_0077eb9c`, `FUN_007a7f20`, `FUN_007ab870`.
- **SubOp `0x0F` / `0x24`:** `_LStrCat3(buf, *gvar_007DA010, "sound\\WA0014.wav"); FUN_007a7f20(buf)` — phát file âm thanh nếu tồn tại (presentation 1 dòng). Không đọc thêm field.
- **SubOp `0x11`:** `func_0x0076c248(gvar_007DA0D0, RP)` (hộp đen) + `if (*(short*)(*(gvar_007DA7BC+0x44)+4) == -0x4C4A) func_0x00603d48(gvar_007DA6E0, 4)`.
- **SubOp `0x15` / `0x16` / `0x17`:** call riêng + cụm chung: nếu pet theo sau tồn tại và là `TFollowNpc` thì dọn (`FUN_0059d96c; FUN_0059d780`); nếu object UI tồn tại thì call UI (1 dòng).
- **SubOp `0x19`:** toast/banner 1200ms với `UNK_00797ee8` (1 dòng).
- **SubOp `0x3B`:** toast 2000ms với `UNK_00797f1c`.
- **SubOp `0x26`:** `FUN_0073439c(*(gvar_007DA7BC), rawBuf, param_3)` — truyền cả buffer thô + tham số thứ 3 (khác mọi nhánh khác chỉ truyền RP).
- **SubOp `0x27`:** guard `Len<2→BoundErr(1)`, `b = RP[1]` byte thường, `FUN_00734610(*(gvar_007DA7BC), b)`.
- **SubOp `0x29`:** `_LStrFromChar(RP[1])` → `FUN_0077eaa4(...)` (check `==1`) → `*(gvar_007DA7BC+0x1434) = result`. Là cờ boolean từ 1 byte wire.
- **SubOp `0x2A`:** `FUN_0073c690(*(gvar_007DA7BC))` — bỏ qua toàn bộ payload.
- **SubOp `0x41`:** `*(gvar_007DA7BC+0x1453) = 0` — clear cờ, không đọc payload.
- **SubOp `0x6D`:** `FUN_007ab870(*(gvar_007DA1B0), 0, &UNK_00797f38, 0)` — chèn 1 dòng hệ thống (1 dòng hiển thị).
- **SubOp `0x72` (phức tạp nhất OP):** `_LStrCopy(RP,2,2)` = `P[2..3]`; `w = FUN_0077eb9c` Word LE; `FUN_00774a84(gvar_007DA540, w, &s)` tra tên; `Format(&UNK_00797f64, args, 1, &out)`; `FUN_007ab870(gvar_007DA1B0, 0, out, 0)` — chat hệ thống có tham số (presentation 1 dòng).

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`):
```c
case 0x17:
  break;
```
**Kết luận: client không bao giờ chủ động gửi OP 0x17.** Không có builder C→S nào (giống OP 0x0D/0x0E/0x10). Mock Server chỉ cần phát S→C.

---

## 6. Chuỗi tiếng Việt / mã hóa

Mã hóa đúng là **cp1258 → NFC** (tiền lệ `opcode_02.md` mục 5, không phải VISCII). OP này tham chiếu:

| # | Địa chỉ | Nơi dùng | Trạng thái |
|---|---|---|---|
| 1 | `"sound\\WA0014.wav"` (literal ASCII) | SubOp `0x0F/0x24` | Không cần giải mã; asset âm thanh |
| 2 | `UNK_00797ee8` | SubOp `0x19` toast 0x4B0 | **Không có `lit_797ee8.hex` → chưa dịch được** |
| 3 | `UNK_00797f1c` | SubOp `0x3B` toast 2000 | Không có dump → chưa dịch được |
| 4 | `UNK_00797f38` | SubOp `0x6D` chat | Không có dump → chưa dịch được |
| 5 | `UNK_00797f64` (mẫu `Format`) | SubOp `0x72` | Không có dump → chưa dịch được |

`redump/` hiện **không có file `lit_797*.hex` nào**. Tuyệt đối không bịa nội dung; cần dump Delphi ansistring tại các địa chỉ trên rồi `bytes.decode('cp1258')` + `normalize('NFC')`.

---

## 7. Ghi chú cho Mock Server

1. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes socket = plain XOR `0xAD` (`byte^0xAD`: `F4^AD=59`, `44^AD=E9`, `17^AD=BA`, `02^AD=AF`, `0F^AD=A2`, `19^AD=B4`, `72^AD=DF`).
2. **Bảng frame tính sẵn:**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp `0x02` min | `17 02` | `F4 44 02 00 17 02` | `59 E9 AF AD BA AF` |
| SubOp `0x0F` (âm thanh) | `17 0F` | `F4 44 02 00 17 0F` | `59 E9 AF AD BA A2` |
| SubOp `0x19` (toast) | `17 19` | `F4 44 02 00 17 19` | `59 E9 AF AD BA B4` |
| SubOp `0x27` + b=`0x01` | `17 27 01` | `F4 44 03 00 17 27 01` | `59 E9 AE AD BA 8A AC` |
| SubOp `0x72` + w=`0x0001` | `17 72 01 00` | `F4 44 04 00 17 72 01 00` | `59 E9 A9 AD BA DF AC AD` |

3. **Thứ tự test an toàn:** `17 2A / 17 41` (no-payload) → `17 02` (pass-through) → `17 0F/24` (âm thanh) → `17 19/3B/6D` (toast/chat, text phụ thuộc hằng chưa dịch) → `17 27 01`, `17 29 01`, `17 72 01 00` (có decode) → cuối cùng nhóm pass-through hộp đen còn lại, cách ly + log crash.
4. **Không gửi SubOp `0x00/0x01/0x0E/0x1D/0x53–0x63/≥0x76`** — rơi qua switch (vô hại nhưng vô nghĩa).
5. **Không cần mock C→S** cho 0x17 (builder rỗng).

---

## 8. Source trail (chỉ `ts_decompile/`) + giới hạn

| # | Nguồn | Dòng/địa chỉ | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_020_007902FB_FUN_007902fb.c` | @`0x007902FB`, toàn file 421 dòng | Handler chính, 97 SubOp, wire |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0x17:` d.3806–4268; khóa `0xAD` | Dispatcher MainOp, đối chiếu inline |
| 3 | `ts_decompile/case_functions/manifest.csv` | dòng 22 | Case 20, entry `0x0078AA06`→`0x007902FB` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 20 | Xác nhận mapping |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | `byte_table[0x17]=0x14` | MainOp→Case |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry 20 `FB 02 79 00` | Case→Entry |
| 7 | `ts_decompile/redump/token_recv.hex` | `F4 44` | Framing token |
| 8 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x17: break;` | C→S rỗng |
| 9 | `0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098/0077eaa4/00774a84/007a7f20/007ab870` | body codec/helper | Codec + helper; xác minh `0077ef7c/0077eb1c/0077ee84/0077f098` **không dùng** ở OP này |
| 10 | `redump/` (vắng `lit_797*.hex`) | — | Chuỗi chưa dịch được |

**Giới hạn trung thực:** ~88/97 hàm con **không có file body** — field nội bộ của nhóm pass-through là hộp đen. Các call VMT/hiển thị chỉ tóm 1 dòng. 4 hằng `UNK_00797exx` không có dump — chỉ ghi địa chỉ, không bịa tiếng Việt. Ý nghĩa game-design từng SubOp số nằm ngoài tầng case — chỉ kết luận ở mức bus đa năng S→C.
