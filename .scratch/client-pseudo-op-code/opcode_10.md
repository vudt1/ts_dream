# PHÂN TÍCH — Main OP 0x10 (Case 16, `FUN_0078e593` @ `0x0078E593`) — **Kênh GM / Câu hỏi / Sự kiện Quiz-Day55 + Chat hệ thống (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

---

## 0. Kết luận quan trọng nhất

> **Main OP 0x10 CÓ SubOp. `switch(SubOp)` với 25 case label: `1,2,3,4,5,6,7,8,9,10,0xB,0xC,0xD,0xE,0xF,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19`. Ngược với OP 0x0C (không SubOp).**

- **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec (file case dòng 31–38; dispatcher dòng 2739–2748).
- Bản chất nghiệp vụ: **không phải 1 feature mà là 1 "bus đa năng"**:
  - SubOp 1 = nạp danh sách chuỗi Q&A vào form `TAC_Question` (parse phức tạp nhất OP).
  - SubOp 2 = toast/banner hệ thống theo mã chọn 1/2/3/9.
  - SubOp 3–7, 9–13, 14–19, 20–23, 25 = **pass-through nguyên RestPayload** cho object quản lý (chủ yếu `TGmManage`).
  - SubOp 8 / 13 (`0xD`) = chèn 1 dòng hệ thống vào `TTalkMsgForm` qua `FUN_007ab870`.
  - SubOp 24 (`0x18`) = refresh form `TWF_Day55EasyForm`, bỏ qua payload.
- **14/25 đường là hộp đen**: các `func_0x00613xxx/00614xxx`, `func_0x00733800`, `func_0x007074cc` và các call `VMT+0x8c/0x90` **không có file body** — chỉ biết target object + pass nguyên buffer.

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_016_0078E593_FUN_0078e593.c:8` — `void FUN_0078e593(void)` @ `0x0078E593` (255 dòng).
- `ts_decompile/case_functions/manifest.csv:18` — `16,0x0078A9F6,0x0078E593,EXPORTED,"FUN_0078e593"`.
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 16 (cùng target).

**Dispatcher `FUN_0078a89c`:**
- Nhận `param_2` (**MainOp**), `param_1` (**RestPayload** đã cắt MainOp); tra bảng byte `0x78A8EE` + bảng dword `0x78A9B6` rồi `switch`.
- Nhánh `case 0x10:` tại **dòng 2738–2967** chứa toàn bộ SubOp 1..0x19, khớp từng dòng với file case riêng → dispatcher không thêm logic, single source of truth là file case.
- Vòng khóa `iVar21 = 0xad` = dấu vết **khóa XOR tĩnh `0xAD`**.

**Mapping MainOp 0x10 → Case 16 (xác minh 3 nguồn):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` dòng 2 → `byte_table[0x10] = 0x10` (= 16 = Case 16).
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex`: entry index 16 = `93 E5 78 00` = `0x0078E593` (LE).
- `manifest.csv:18` + jump-table entry `0x0078A9F6` → khớp.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD` từng byte. Server speaks first. Token `F4 44` hiện diện trong `token_recv.hex`.

**Cách đọc SubOp (file case dòng 31–38):**
```c
iVar4 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload (payload gốc đã cắt byte P[0]=MainOp)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar4 + 0);  // SubOp = ECX[0] = payload[1]
switch(SubOp) { case 1 ... case 0x19 ... }   // KHÔNG có case 0, KHÔNG có default
```
- `unaff_EBP-0xc` = RestPayload. **SubOp = `payload[1]` = `ECX[0]`**, 1 byte thường, không codec.
- Quy ước `_LStrCopy(ECX, p, n)` là Delphi `Copy` **1-based**: `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`. Ví dụ `Copy(ECX,2,4)` = `payload[2..5]`.

**Codec helper (đã đọc body — nhưng handler này KHÔNG dùng Word/DWORD LE):**
- `FUN_0077eb9c`: `b0 + b1*0x100` = **Word LE**. **Handler 0x10 không gọi lần nào.**
- `FUN_0077ef7c`: `b0+b1*0x100+b2*0x10000+b3*0x1000000` = **DWORD LE**. **Handler 0x10 không gọi lần nào** (khác OP 0x0C/0x02 dùng codec này để đọc id).
- `FUN_0077eb1c` (encode Word), `FUN_0077ee84` (encode DWORD): chỉ dùng chiều C→S, không dùng ở S→C này.
- `FUN_0077f098` (copy tên 8 byte): handler này không gọi.
- Handler 0x10 chỉ đọc **byte độ dài thô** + `_LStrCopy` cắt chuỗi — mọi field số nhiều-byte (nếu có) nằm trong các hàm con hộp đen, không suy diễn.

---

## 2. Đối chiếu dispatcher inline

Khối `case 0x10:` trong `0078a89c_FUN_0078a89c.c:2738–2966` khớp 1:1 với file case riêng (SubOp, SubOp 1 parse chuỗi, SubOp 2 sel + hằng, pass-through `TGmManage`, chat `FUN_007ab870`, VMT câu hỏi ngày, `FUN_0070736c`).

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: **1,2,3,4,5,6,7,8,9,10,0xB,0xC,0xD,0xE,0xF,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19**. **Không có `case 0`, không có `default`** → SubOp 0 / ≥0x1A rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x10`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu | Wire layout | Logic core (1 dòng hiển thị/VMT) |
| :---: | :---: | :--- | :--- |
| `0x01` | biến động, ≥3B để qua guard đầu; full cần ≥6B + 3 chuỗi con + chuỗi đuôi | `[10][01][lenN: P2][name: P3..][lenA,strA][lenB,strB][lenC,strC][tail: còn lại]` (chi tiết mục 4) | Nạp tên + 3 đáp án + chuỗi đuôi vào form `TAC_Question`, refresh 4 VMT, set `+0x148=1` |
| `0x02` | **3B** `[10][02][sel]` | `[10][02][sel: P2, 1B]` sel ∈ {1,2,3,9} | Toast/banner `TSe_TalkMsgFormPlus` với duration 0x4B0/1000 + refresh có điều kiện; sel khác → no-op |
| `0x03` | **2B** | pass nguyên RP | `func_0x00613e48(TGmManage, RP)` — hộp đen |
| `0x04` | **2B** | pass nguyên RP | `func_0x00614318(TGmManage, RP)` — hộp đen |
| `0x05` | **2B** | pass nguyên RP | `func_0x00614bb0(TGmManage, RP)` — hộp đen |
| `0x06` | **2B** | pass nguyên RP | `func_0x006140d4(TGmManage, RP)` — hộp đen |
| `0x07` | **2B** | pass nguyên RP | `func_0x00733800(obj_007DA7BC, RP)` — hộp đen |
| `0x08` | **2B** `[10][08]` | không đọc field nào thêm | `FUN_007ab870(TTalkMsgForm, 0, UNK_00797120, 0)` + `TSe_SendMailForm+0x131=0` (1 dòng hiển thị) |
| `0x09` | **2B** | pass nguyên RP | `func_0x00614724(TGmManage, RP)` — hộp đen |
| `0x0A` | **2B** | pass nguyên RP | `func_0x00613b50(TGmManage, RP)` — hộp đen |
| `0x0B` | **2B** | pass nguyên RP | `func_0x006144f8(TGmManage, RP)` — hộp đen |
| `0x0C` | **2B** | pass nguyên RP | `func_0x00613240(TGmManage, RP)` — hộp đen |
| `0x0D` | **2B** `[10][0D]` | không đọc field nào thêm | `FUN_007ab870(TTalkMsgForm, 0, UNK_0079714c, 0)` + `TSe_SendMailForm+0x132=0` (1 dòng hiển thị) |
| `0x0E` | **2B** | pass nguyên RP | `func_0x006136f0(TGmManage, RP)` — hộp đen |
| `0x0F` | **2B** | pass nguyên RP | `func_0x00613920(TGmManage, RP)` — hộp đen |
| `0x10` | **2B** | pass nguyên RP | `func_0x00613230(TGmManage, RP)` — hộp đen |
| `0x11` | **2B** | pass nguyên RP | `func_0x00614b58(TGmManage, RP)` — hộp đen |
| `0x12` | **2B** | pass nguyên RP | `func_0x00613454(TGmManage, RP)` — hộp đen |
| `0x13` | **2B** | pass nguyên RP | `func_0x006146e4(TGmManage, RP)` — hộp đen |
| `0x14` | **2B** | pass nguyên RP | `VMT+0x8c(TKC_Question, RP)` — hộp đen |
| `0x15` | **2B** | pass nguyên RP | `VMT+0x90(TKC_Question, RP)` — hộp đen |
| `0x16` | **2B** | pass nguyên RP | `VMT+0x8c(TWF_day55QuestionForm, RP)` — hộp đen |
| `0x17` | **2B** | pass nguyên RP | `VMT+0x8c(TWF_day55hardQuestion, RP)` — hộp đen |
| `0x18` | **2B** `[10][18]` | bỏ qua toàn bộ payload | `FUN_0070736c(TWF_Day55EasyForm)` — reset + 2 dòng + refresh (body đã đọc) |
| `0x19` | **2B** | pass nguyên RP | `func_0x007074cc(obj_007DA1D0, RP)` — hộp đen |

Gửi thiếu byte so với cột "tối thiểu" → guard gọi `_BoundErr`, không phải no-op (riêng nhóm pass-through 2B: gửi đúng `[10][SubOp]` là đủ để hàm con được gọi; field bên trong do hàm con tự guard).

---

## 4. Chi tiết từng SubOp (wire layout + logic)

### SubOp `0x01` — Nạp bộ câu hỏi vào `TAC_Question` (parse phức tạp nhất OP)

Wire (Delphi `Copy` 1-based trên RP; `RP[i]=P[i+1]`):
```
[0]=0x10
[1]=0x01                                   (SubOp = RP[0])
[2]=lenN: 1B  (RP[1], guard len(RP)<2 → _BoundErr)
[3..3+lenN-1]=name                         (Copy(RP,3,lenN))
[lenA:1B][strA:lenA B][lenB:1B][strB:lenB B][lenC:1B][strC:lenC B]  (vòng 3 lần:
                                            mỗi vòng đọc len tại cursor, Copy(cursor+2,len), cursor += len+1)
[tail: toàn bộ bytes còn lại tới hết RP]   (len-tail = LStrLen(RP)-cursor, Copy(cursor+1,len-tail))
```

Logic core:
1. `FUN_007ba180(form+0x134, 0)` — xóa/reset khối hiển thị (clear + zero 3 con trỏ khi `param_2==0`).
2. `VMT+0x7c(form+0x138)` — xóa list (1 dòng hiển thị).
3. `FUN_007b8060(form+0x134, name)` + gán `form+0x14c = name` — chèn tên (tiêu đề câu hỏi) lên đầu buffer với tiền tố `"\r"` rồi refresh.
4. Vòng 3 lần: mỗi chuỗi đáp án `Copy` ra rồi `VMT+0x6c(form+0x138, str)` — thêm 1 dòng đáp án (3 dòng hiển thị).
5. Chuỗi đuôi `Copy` + `VMT+0x6c` — thêm 1 dòng (gợi ý/giải thích?).
6. `VMT+0x20(form)` + `form+0x148=1` + 3 call `VMT+0x24` — show/refresh 3 panel (3 dòng hiển thị).
- Định danh form: `gvar_007D9D94` = `TAC_Question`.

### SubOp `0x02` — Toast/banner hệ thống theo mã chọn
```
[0]=0x10
[1]=0x02
[2]=sel: 1B (RP[1], guard len<2 → _BoundErr)
```
- `sel==1`: toast `UNK_0079706c` 1200ms (1 dòng hiển thị).
- `sel==2`: toast `UNK_007970a8` 1200ms + refresh 2 form.
- `sel==3`: toast `UNK_007970cc` 1000ms + refresh 2 form.
- `sel==9`: toast `UNK_007970f8` 1000ms + refresh 2 form.
- Mọi giá trị khác (0,4–8,10–255): **rơi qua, không làm gì**.
- Định danh: `gvar_007DA084` = `TSe_TalkMsgFormPlus`.
- Nội dung 4 chuỗi: **không có `lit_7970*.hex` trong `redump/` → chưa dịch được**.

### SubOp `0x03`–`0x07`, `0x09`–`0x0C`, `0x0E`–`0x13` — Pass-through cho `TGmManage` / object toàn cục
Mỗi case chỉ 1 call duy nhất, truyền **nguyên RestPayload**:
- `gvar_007DA160` = `TGmManage`.
- **Không có file body** cho bất kỳ `func_0x00613xxx/00614xxx` nào → **không suy diễn** field bên trong; Mock Server chỉ cần gửi `[10][SubOp][bytes tùy ý]`.
- SubOp 7 target `gvar_007DA7BC` (hàm `func_0x00733800` cũng không có body) → tương tự hộp đen.

### SubOp `0x08` và `0x0D` — Chèn dòng hệ thống vào chat
```c
// SubOp 8:
*(TSe_SendMailForm+0x131) = 0;
FUN_007ab870(TTalkMsgForm, 0, &UNK_00797120, '\0');
// SubOp 0xD:
FUN_007ab870(TTalkMsgForm, 0, &UNK_0079714c, '\0');
*(TSe_SendMailForm+0x132) = 0;
```
- `FUN_007ab870(form, id=0, msg, tag=0)` → hiển thị `"(Công bố hệ thống)" + msg` (tiền lệ `opcode_02.md`), đồng thời clear flag mail `+0x131/+0x132` của `TSe_SendMailForm`.
- Nội dung `UNK_00797120` / `UNK_0079714c`: **không có dump → chưa dịch được**.

### SubOp `0x14`/`0x15`/`0x16`/`0x17` — Pass-through VMT câu hỏi ngày
- `VMT+0x8c(TKC_Question, RP)` / `VMT+0x90(TKC_Question, RP)` / `VMT+0x8c(TWF_day55QuestionForm, RP)` / `VMT+0x8c(TWF_day55hardQuestion, RP)`.
- Method VMT không có body → hộp đen.

### SubOp `0x18` — Refresh form dễ (không đọc payload)
- `FUN_0070736c(*(TWF_Day55EasyForm*)gvar_007D9F00)`: reset → clear → gán hằng → 2 dòng → show + flag `+0x52=1` (tóm 1 dòng hiển thị).
- Các hằng `DAT_007073fc/00707460/00707474`: **không có dump → chưa dịch được**.

### SubOp `0x19` — Pass-through cuối
- `func_0x007074cc(obj, RP)` **không có file body** → hộp đen.

---

## 5. Chiều ngược lại Client → Server (C→S)

`ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`, `TFConnect.SendCommand`):
```c
case 0x10:
  break;
```
**OP 0x10 phía gửi là rỗng / no-op.** Client không bao giờ chủ động gửi OP 0x10; chỉ nhận S→C. Mock Server không cần mock chiều này.

---

## 6. Chuỗi hằng / tiếng Việt

Mã hóa đúng của game là **cp1258 → NFC** (tiền lệ `opcode_02.md` mục 5), không phải VISCII. OP này tham chiếu các hằng sau, **tất cả đều chưa có dump nên chưa dịch được — ghi rõ, KHÔNG bịa nội dung**:

| Địa chỉ | Dùng ở | Trạng thái dump |
| :--- | :--- | :--- |
| `UNK_0079706c` | SubOp 2 sel=1, toast 0x4B0 | **không có `lit_7970*.hex` → chưa dịch được** |
| `UNK_007970a8` | SubOp 2 sel=2, toast 0x4B0 | như trên |
| `UNK_007970cc` | SubOp 2 sel=3, toast 1000 | như trên |
| `UNK_007970f8` | SubOp 2 sel=9, toast 1000 | như trên |
| `UNK_00797120` | SubOp 8, `FUN_007ab870(...,tag 0)` | **không có `lit_7971*.hex` → chưa dịch được** |
| `UNK_0079714c` | SubOp 0xD, `FUN_007ab870(...,tag 0)` | như trên |
| `DAT_007073fc`, `DAT_00707460`, `DAT_00707474` | `FUN_0070736c` (SubOp 0x18) | **không có `lit_7073/7074*.hex` → chưa dịch được** |

`redump/` hiện chỉ có `lit_7ABDxx`, `lit_77F771/78A854/7A2094/7A20A8`, `lit_5957xx/596078` — không có file nào phủ các địa chỉ trên.

---

## 7. Ghi chú cho Mock Server

1. **Luôn gửi `[0x10][SubOp]...`.** Gửi `[10]` đơn độc (thiếu SubOp) → `_BoundErr(0)`.
2. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes từ Token tới payload XOR `0xAD` từng byte.
3. **Ví dụ tính sẵn** (`byte^0xAD`: `0x10^0xAD=0xBD`, `0x02^0xAD=0xAF`, `0x09^0xAD=0xA4`, `0xF4^0xAD=0x59`, `0x44^0xAD=0xE9`):
   - SubOp 2 sel=9 tối thiểu: payload plain `10 02 09` → socket payload `BD AF A4`.
   - Plain frame (7B): `F4 44 03 00 10 02 09` → socket frame: `59 E9 AE AD BD AF A4`.
   - SubOp 1 tối thiểu có tên `"A"`: payload plain `10 01 01 41` → socket payload `BD AC AC EC`. Lưu ý gói này vẫn thiếu 3 chuỗi đáp án + tail nên client sẽ đọc lố/guard — chỉ dùng để test guard, không phải gói hợp lệ đầy đủ.
4. **Kịch bản test an toàn → nguy hiểm:**
   - Trước: SubOp `0x02` với sel lạ (ví dụ `10 02 05`) → rơi qua no-op, quan sát không crash.
   - Sau: SubOp `0x08` / `0x0D` (`10 08`, `10 0D`) → chèn dòng hệ thống, nội dung phụ thuộc hằng chưa dịch — quan sát text hiện ra để đoán nội dung thật.
   - Cuối: SubOp `0x01` full (tên + 3 đáp án + đuôi) và nhóm pass-through `0x03...` (hộp đen, có thể đổi trạng thái GM) — test cách ly, log crash.
5. **Không gửi SubOp `0x00` / `≥0x1A`** — rơi qua switch (vô hại nhưng vô nghĩa).
6. **Không cần** mock chiều C→S cho OP 0x10.

---

## 8. Source trail + giới hạn trung thực (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_016_0078E593_FUN_0078e593.c` | toàn file 255 dòng | Handler chính, danh sách SubOp, wire layout |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | dispatcher; khóa `0xAD`; `case 0x10:` d.2738–2966 | Đối chiếu inline khớp 1:1, framing key |
| 3 | `ts_decompile/case_functions/manifest.csv` | d.18 | Case 16, entry `0x0078A9F6` → `0x0078E593` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 16 | Xác nhận mapping lần 2 |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | dòng 2 | `byte_table[0x10]=0x10` |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry 16 `93 E5 78 00` | `= 0x0078E593` |
| 7 | `ts_decompile/redump/token_recv.hex` | `F4 44` | Token framing |
| 8 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x10: break;` | C→S rỗng |
| 9 | `0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | body codec | Word/DWORD LE + xác nhận handler này KHÔNG gọi chúng |
| 10 | `0070736c / 007ba180 / 007b8060 / 007ab870` | body helper | Reset/prepend/chat (tóm 1 dòng hiển thị) |
| 11 | `0051189c / 0050a4a0` | gán global | `TGmManage`, `TAC_Question`, `TKC_Question`, `TWF_day55*`, `TWF_Day55EasyForm`, `TSe_SendMailForm`, `TTalkMsgForm`, `TSe_TalkMsgFormPlus` |

**Giới hạn (không suy diễn):**
- Các `func_0x00613xxx/00614xxx`, `func_0x00733800`, `func_0x007074cc` **không có file body** — field nội bộ của nhóm pass-through là hộp đen.
- Các call `VMT+0x8c/0x90/0x6c/0x7c/0x20/0x24` là method ảo, không resolve được body từ tầng case — chỉ tóm "hiển thị/refresh 1 dòng".
- 9 hằng chuỗi **không có dump** — ghi địa chỉ + chưa dịch, không bịa nội dung.
- Ý nghĩa game-design (đáp án đúng/sai, điều kiện GM, nội dung quiz Day55) nằm ngoài decompile tầng case — chỉ kết luận ở mức "bus GM/câu hỏi/sự kiện + chat hệ thống".
