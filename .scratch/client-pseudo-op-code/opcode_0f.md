# PHÂN TÍCH — Main OP 0x0F (Case 15, `FUN_0078e367` @ `0x0078E367`) — **Đồng bộ NPC / Cart / Buff-effect (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body thì ghi rõ, không suy diễn.

---

## 0. Kết luận quan trọng nhất

> **Main OP 0x0F CÓ SubOp. Handler là một `switch` 20 nhánh trên 1 byte `payload[1] = ECX[0]`.**

Khác OP 0x0C (không SubOp, struct cố định), OP này giống họ OP 0x02/0x0D: 1 byte SubOp chọn đường xử lý, mỗi đường forward nguyên RestPayload cho một hàm con kèm một object form (`TFNpcManage`, `TWF_CartManage`, …). Trong 20 đường, **chỉ 5 đường đọc được body** (`0x05, 0x09, 0x0A, 0x0E, 0x0F`); 15 đường còn lại gọi hàm **không có file body** — Mock Server với các SubOp đó chỉ có thể forward mù.

Nghiệp vụ (suy trực tiếp từ object form + body đọc được): **đồng bộ NPC (TFNpcManage) + xe/cart (TWF_CartManage) + trạng thái buff/effect trên actor + vài form hệ thống**. Không phải chat (chat là OP 0x02), không mang text tiếng Việt nào trên wire.

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_015_0078E367_FUN_0078e367.c:8` — `void FUN_0078e367(void)` @ `0x0078E367`
- `ts_decompile/case_functions/manifest.csv:17` — `15,0x0078A9F2,0x0078E367,EXPORTED,"FUN_0078e367"`
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 15, body trùng file case.

**Dispatcher:** `FUN_0078a89c` nhận `param_2` chính là **MainOp**, `param_1` là **RestPayload (đã cắt MainOp)**, tra bảng byte `0x78A8EE` + bảng dword `0x78A9B6` rồi `switch`. Nhánh `case 0xf:` tại **dòng 2640–2737** chứa toàn bộ 20 SubOp, **khớp từng dòng** với file case riêng (khác duy nhất: bản inline gán thêm địa chỉ return-tracking của decompiler, không phải logic).

**Mapping MainOp 0x0F → Case 15 (xác minh 3 nguồn):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` hàng 1 → `byte_table[0x0F] = 0x0F` (index 15).
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` hàng 4: entry 15 = `0x0078E367` (LE). Khớp manifest.
- `manifest.csv:17` + `jumptable_0x78A9B6_cases.c` (Case 15).

**Khung mạng:** Header `[Token 2B: F4 44] [Length L: Word LE 2B] [Payload L Bytes]`, toàn bộ frame XOR từng byte với khóa tĩnh `0xAD` (hàm giải mã chiều nhận `FUN_0050a248` được `TForm1.ClientSocket1Read` gọi với `0xAD`). Server speaks first.

**Cách đọc SubOp** (file case dòng 21–28):
```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + 0);  // SubOp = ECX[0]
switch(SubOp) { case 1 ... case 0x15 ... }   // KHÔNG có case 0, case 3, KHÔNG có default
```
- `unaff_EBP-0xc` = RestPayload = payload gốc bỏ byte `P[0]=MainOp`.
- **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, có guard `_BoundErr(0)` nếu rỗng.
- Quy ước `_LStrCopy(ECX, p, n)` là Delphi `Copy` **1-based**: `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`. Ví dụ `Copy(ECX,2,4)` = `payload[2..5]`.

**Codec helper (đã đọc body):**
- `FUN_0077eb9c` = **Word LE** (`b0 + b1*0x100`), guard từng byte.
- `FUN_0077ef7c` = **DWORD LE** (`b0+b1*0x100+b2*0x10000+b3*0x1000000`).
- `FUN_0077eb1c` (encode Word), `FUN_0077ee84` (encode DWORD), `FUN_0077f098` (copy tên 8B): **tồn tại file body nhưng handler này và cả 5 hàm con đọc được đều KHÔNG gọi** — chúng chỉ dùng chiều C→S.

---

## 2. Đối tượng form (arg 1 của từng đường — đã xác minh nơi khởi tạo)

| Global | Class | Nguồn xác minh |
|---|---|---|
| `gvar_007D9E00` | **TFNpcManage** (quản lý NPC) | `0051189c_FUN_0051189c.c`: `FUN_007a40a4(VMT_7A1894_TFNpcManage)` → `gvar_007D9E00` |
| `gvar_007DA454` | **TWF_CartManage** (quản lý xe/cart) | `0050a4a0_TForm1.FormCreate.c:668-669`: `TWF_CartManage_Create(...)` → `gvar_007DA454` |
| `gvar_007D9F70` | **TSe_SystemForm** | `0051189c`: `FUN_005a64d4(VMT_589FAC_TSe_SystemForm)` → `gvar_007D9F70` |
| `gvar_007DA7BC` | **Player record self** (`+4` = myId) | dùng `*(+4)==id`, cùng quy ước `opcode_0c.md` |
| `gvar_007D9D34` | **Scene actor container** (tra actor trong mảng 800 slot `gvar_007DA300`) | cùng quy ước `opcode_0c.md` §4 |
| `gvar_007DA2FC` | Manager của SubOp 0x0D–0x0F (vùng item/buff) | **chưa tìm thấy dòng khởi tạo `Create` → chưa xác định tên class chính xác, chỉ rõ vai trò** |

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: `1, 2, 4, 5, 6, 7, 8, 9, 10(0xa), 11(0xb), 12(0xc), 13(0xd), 14(0xe), 15(0xf), 16(0x10), 17(0x11), 18(0x12), 19(0x13), 20(0x14), 21(0x15)`. **Không có `case 0`, không có `case 3`, không có `default`** → SubOp 0 / 3 / ≥22 rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x0F`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu | Wire field (tầng case) | Hàm con / object | Body? |
|---|---|---|---|---|
| `0x01` | ≥2 (`0F 01`) | forward nguyên RP | `func_0x007a3ed0(TFNpcManage, RP)` | KHÔNG |
| `0x02` | ≥2 | forward nguyên RP | `func_0x007a40e8(TFNpcManage, RP)` | KHÔNG |
| `0x04` | ≥2 | forward nguyên RP | `func_0x007a4694(TFNpcManage, RP)` | KHÔNG |
| `0x05` | **14** (`0F 05` + 12B) | 3×DWORD LE | `FUN_007a49c0(TFNpcManage, RP)` | **CÓ** |
| `0x06` | ≥2 | forward nguyên RP | `func_0x007a42ec(TFNpcManage, RP)` | KHÔNG |
| `0x07` | ≥2 | forward nguyên RP | `func_0x007a43c8(TFNpcManage, RP)` | KHÔNG |
| `0x08` | ≥2 | forward nguyên RP | `func_0x007a5010(TFNpcManage, RP)` | KHÔNG |
| `0x09` | **7** (`0F 09` + 4B + 1B) | id DWORD + flag 1B + msg tới hết | `FUN_007a4b08(TFNpcManage, RP)` | **CÓ** |
| `0x0A` | **9** (`0F 0A` + 8B) | vòng lặp ≤4 entry `[idx:1B][w:2B][b:1B][w:2B][len:1B][str]` | `FUN_00751d28(TWF_CartManage, RP)` | **CÓ** |
| `0x0B` | **3** (`0F 0B` + 1B) | 1 byte thường (guard `Len<2→BoundErr(1)`) | `func_0x00751c7c(TWF_CartManage, byte)` | KHÔNG |
| `0x0C` | ≥2 | forward nguyên RP | `func_0x00746fc8(self, RP)` | KHÔNG |
| `0x0D` | ≥2 | forward nguyên RP | `func_0x0052a898(mgr_2FC, RP)` | KHÔNG |
| `0x0E` | **8** (`0F 0E` + 4B + 2B) | id DWORD + w Word LE | `FUN_0052a7b4(mgr_2FC, RP)` | **CÓ** |
| `0x0F` | **5** (`0F 0F` + 2B + 1B) | w Word LE + b byte thường | `FUN_0052aacc(mgr_2FC, RP)` | **CÓ** |
| `0x10` | ≥2 | forward nguyên RP | `func_0x0074dfd4(scene, RP)` | KHÔNG |
| `0x11` | ≥2 | forward nguyên RP | `func_0x0074e400(scene, RP)` | KHÔNG |
| `0x12` | ≥2 | forward nguyên RP | `func_0x0074edd0(self, RP)` | KHÔNG |
| `0x13` | ≥2 | forward nguyên RP | `func_0x005a71bc(SystemForm, RP)` | KHÔNG |
| `0x14` | ≥2 | forward nguyên RP | `func_0x007a5b74(TFNpcManage, RP)` | KHÔNG |
| `0x15` | ≥2 | forward nguyên RP | `func_0x007a4e94(TFNpcManage, RP)` | KHÔNG |

---

## 4. Chi tiết từng SubOp đọc được body

### 4.1. SubOp `0x05` — set 3 DWORD lên actor + VMT
- Wire: `[0F][05][id:P2..P5 DWORD LE][a:P6..P9 DWORD LE][b:P10..P13 DWORD LE]` — 3 lần `Copy(RP,2/6/10,4)` + `FUN_0077ef7c`.
- Core:
  1. `if (myId==id) obj=self else { idx=FUN_0070c20c(scene,id); obj=idx? gvar_007DA300[idx] : NULL }` — rẽ self/actor khác, actor không tồn tại → bỏ qua.
  2. `(VMT[obj]+0x18)(obj,8)` — call hiển thị (1 dòng).
  3. `FUN_0071dae0(obj,a,b)` — ghi cặp DWORD lên actor.

### 4.2. SubOp `0x09` — id + flag byte + chuỗi tới hết payload
- Wire: `[0F][09][id:P2..P5 DWORD LE][flag:P6, 1B thường][msg:P7.. tới hết]`. Guard: RP len<6 → `_BoundErr(5)`; `msg=Copy(RP,7,Len-6)` (rỗng được).
- Core: rẽ self/actor như trên, rồi `FUN_0071c718(obj, flag, msg)` — áp 1 byte + chuỗi lên actor (1 dòng hiển thị).

### 4.3. SubOp `0x0A` — parser mảng cart ≤4 entry lên self (quan trọng nhất OP)
- Wire tầng case: `[0F][0A][rest...]` forward nguyên RP; parse nằm trong callee `FUN_00751d28`.
- Body: vòng lặp `pos=2` (Delphi 1-based → `RP[1]`), tối đa 4 vòng, mỗi entry:
  - `idx = RP[pos]` byte thường (guard `idx>4 → BoundErr` → slot 0..4);
  - `w1 = Word LE RP[pos+1..pos+2]`, `b2 = RP[pos+3]` byte, `w3 = Word LE RP[pos+4..pos+5]`, `slen = RP[pos+6]` byte, `str = RP[pos+7 .. +slen-1]`;
  - ghi vào self (`gvar_007DA7BC`): tên + word/cờ theo slot (tên tối đa 10 ký tự, chỉ khi `slen!=0`).
  - tiến `pos += 8+slen`; dừng khi đủ 4 vòng hoặc hết chuỗi. Payload ngắn hơn entry đang đọc → `_BoundErr`, không phải no-op.

### 4.4. SubOp `0x0B` — 1 byte cho CartManage (không body)
- Wire: `[0F][0B][b:P2, 1B]` — guard `Len<2→BoundErr(1)` ngay tầng case, `func_0x00751c7c(cartObj, b)`.
- **Giới hạn:** `00751c7c` không có file body — không suy diễn ý nghĩa byte này.

### 4.5. SubOp `0x0E` — apply buff/effect lên actor
- Wire: `[0F][0E][id:P2..P5 DWORD LE][w:P6..P7 Word LE]`.
- Core: `idx=FUN_0070c20c(scene,id)`; nếu actor tồn tại → `FUN_0074b7c8(actorObj, w)`.
- `FUN_0074b7c8(obj, w)`: tra cứu buff theo mã word qua bảng; nếu trúng thì điền struct buff tại actor (tên chuỗi, timer, các word chỉ số, cờ), refresh UI (và thêm refresh nếu là self). Tóm 1 dòng: **áp buff/effect có tra bảng lên actor**. `w` không trúng bảng → không làm gì.

### 4.6. SubOp `0x0F` — set/clear buff state của self
- Wire: `[0F][0F][w:P2..P3 Word LE][b:P4, 1B thường]` — guard RP len<4 → `_BoundErr(3)`.
- Core:
  - `w==0` → `FUN_0074bd70(self)`: duyệt 4 slot, gỡ hiển thị slot, xóa struct buff — tóm 1 dòng: **xóa buff self**.
  - `w!=0` → `FUN_0074b7c8(self,w)` (apply buff như 4.5) + ghi byte `b` + cờ + refresh.

### 4.7. Các SubOp không body (`0x01,02,04,06,07,08,0x0B,0x0C,0x0D,0x10–0x15`)
- Tầng case chỉ forward nguyên RP cho `(objectForm, RP)` như bảng §3 — không tách field nào ở tầng case.
- **Giới hạn:** 15 hàm đều **không có file body trong `ts_decompile/functions/`**. Không suy diễn parse bên trong; Mock Server chỉ forward nguyên rest.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

`ts_decompile/functions/0077f414_FUN_0077F414.c`:
```c
case 0xf:
  break;
```
**Client không bao giờ chủ động gửi OP 0x0F** — chiều duy nhất là S→C, không có builder payload C→S (giống OP 0x0C/0x0D).

---

## 6. Chuỗi tiếng Việt / mã hóa

- Handler tầng case **không tham chiếu bất kỳ hằng chuỗi nào** (chỉ `gvar_*` + địa chỉ hàm). Cả 5 hàm con đọc được body **không chứa một `DAT_…/UNK_0079…` chuỗi nào**.
- Do đó **không có gì để tra `ts_decompile/redump/lit_*.hex`, không giải mã gì cho OP này — ghi rõ để tránh bịa đặt**. Ghi nhận tiền lệ đúng của game là **cp1258 → NFC** (theo `opcode_02.md` mục 5), không phải VISCII, để dùng khi cần trong tương lai.

---

## 7. Ghi chú cho Mock Server

1. **Frame:** `F4 44 | Len:Word LE (=số byte payload) | payload`, toàn bộ bytes socket = plain XOR `0xAD` từng byte. Check: `F4^AD=59`, `44^AD=E9`, `0F^AD=A2`.
2. **Bảng frame tính sẵn (đã verify XOR tay):**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 5: id=1,a=2,b=3 | `0F 05 01 00 00 00 02 00 00 00 03 00 00 00` | `F4 44 0E 00 …` | `59 E9 A3 AD A2 A8 AC AD AD AD AF AD AD AD AE AD AD AD` |
| SubOp 9: id=1,flag=7,msg rỗng | `0F 09 01 00 00 00 07` | `F4 44 07 00 …` | `59 E9 AA AD A2 A4 AC AD AD AD AA` |
| SubOp 0x0A: 1 entry rỗng | `0F 0A 00 00 00 00 00 00 00` | `F4 44 09 00 …` | `59 E9 A4 AD A2 A7 AD AD AD AD AD AD AD` |
| SubOp 0x0B: b=1 | `0F 0B 01` | `F4 44 03 00 0F 0B 01` | `59 E9 AE AD A2 A6 AC` |
| SubOp 0x0E: id=1,w=2 | `0F 0E 01 00 00 00 02 00` | `F4 44 08 00 …` | `59 E9 A5 AD A2 A3 AC AD AD AD AF AD` |
| SubOp 0x0F: w=0 (clear buff self),b=0 | `0F 0F 00 00 00` | `F4 44 05 00 …` | `59 E9 A8 AD A2 A2 AD AD AD` |

3. **Thứ tự test an toàn:** `0F 0B 01` (1 byte, ít tác động nhất) → `0F 0F 00 00 00` (clear buff self, hàm lá đã rõ) → `0F 09 …` với id lạ (dừng ở `FUN_0070c20c==0` nếu actor chưa tồn tại) → `0F 05/0E` với id tồn tại → cuối cùng `0F 0A` (parser vòng lặp) và các pass-through mù.
4. **Không gửi SubOp `0x00 / 0x03 / ≥0x16`** — rơi qua switch (vô hại nhưng vô nghĩa). Payload ngắn hơn mức tối thiểu → `_BoundErr`, coi như gói lỗi.
5. **Không cần** mock chiều C→S cho OP 0x0F.

---

## 8. Source trail + giới hạn trung thực (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_015_0078E367_FUN_0078e367.c` | toàn file 126 dòng; 20 case | Handler chính, danh sách SubOp |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0xf:` d.2640–2737 | Đối chiếu inline 1:1 với file case |
| 3 | `ts_decompile/case_functions/manifest.csv` | d.17 | Case 15, entry `0x0078A9F2` → `0x0078E367` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 15 | Xác nhận mapping lần 2 |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | hàng 1 | `byte_table[0x0F]=0x0F` |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | hàng 4 | entry 15 = `0x0078E367` LE |
| 7 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0xf: break;` | C→S rỗng |
| 8 | `007a49c0 / 007a4b08 / 00751d28 / 0052a7b4 / 0052aacc` | toàn file | Body 5 SubOp + wire field |
| 9 | `0074b7c8 / 0074bd70` | toàn file | Lá buff apply/clear của SubOp 0x0E/0x0F |
| 10 | `0077eb9c / 0077ef7c` | codec | Word LE / DWORD LE + guard |
| 11 | `0077eb1c/0077ee84/0077f098` | body tồn tại | Xác minh **không dùng** ở OP này |
| 12 | `0051189c / 0050a4a0` | khởi tạo form | `TFNpcManage`, `TWF_CartManage`, `TSe_SystemForm` |
| 13 | `0050a248`, `token_send.hex`, `token_recv.hex` | XOR + token | Framing `F4 44` + XOR `0xAD` |

**Giới hạn (không suy diễn):**
- 15/20 hàm con không có body — ý nghĩa các SubOp đó chỉ biết ở mức `(objectForm, forward RP)`.
- Tên class của `gvar_007DA2FC` chưa tìm thấy dòng `Create` trực tiếp — vai trò suy từ call-site, đã đánh dấu mức chắc chắn.
- Ý nghĩa game-design chi tiết (buff nào, NPC nào, cart dùng ở đâu) nằm ngoài tầng case — chỉ kết luận ở mức "đồng bộ NPC/cart/buff".
