# PHÂN TÍCH — Main OP 0x10 (Case 16, `FUN_0078e593` @ `0x0078E593`) — **Kênh GM / Câu hỏi / Sự kiện Quiz-Day55 + Chat hệ thống (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Kết luận quan trọng nhất

> **Main OP 0x10 CÓ SubOp. `switch(SubOp)` với 25 case label: `1,2,3,4,5,6,7,8,9,10,0xB,0xC,0xD,0xE,0xF,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19`. Ngược với OP 0x0C (không SubOp).**

- **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec (file case dòng 31–38; dispatcher dòng 2739–2748).
- Bản chất nghiệp vụ: **không phải 1 feature mà là 1 "bus đa năng"**:
  - SubOp 1 = nạp danh sách chuỗi Q&A vào form `TAC_Question` (parse phức tạp nhất OP).
  - SubOp 2 = toast/banner hệ thống theo mã chọn 1/2/3/9.
  - SubOp 3–7, 9–13, 14–19, 20–23, 25 = tầng case chỉ **pass nguyên RestPayload** cho object quản lý (chủ yếu `TGmManage`), **nhưng 15/16 callee đã có body mới và tự parse payload** (mục 4).
  - SubOp 8 / 13 (`0xD`) = chèn 1 dòng hệ thống vào `TTalkMsgForm` qua `FUN_007ab870`.
  - SubOp 24 (`0x18`) = refresh form `TWF_Day55EasyForm`, bỏ qua payload.
- **Hộp đen chỉ còn 5/25 đường**: `func_0x00614b58` (SubOp 0x11) vẫn **không có body**; 4 call `VMT+0x8c/0x90` (SubOp 0x14–0x17) không resolve được. 13/14 callee nằm trong HOLE `0x00612E1A→0x00614C38` **đã có body mới** (cộng 2 callee ngoài HOLE `007074cc/00733800` → tổng **15/16**) và được phân tích ở mục 4 (“ĐÃ CÓ BODY” — bảng từng callee) — phát hiện lớn: **không đường nào là "chỉ truyền tay đôi"**; các hàm con tự parse RestPayload bằng đúng codec Word/DWORD LE (`FUN_0077eb9c`/`FUN_0077ef7c`, key `gvar_007D9D30`) và in kết quả vào **memo của form GM** (`gvar_007DA110 + 0x384/+0x2dc/+0x2e4`, control con `+0x208`; bản thân `gvar_007DA110` **chưa định danh được VMT/tên form** — suy đoán "form GM" từ việc toàn bộ handler TGmManage đổ text vào nó), hoặc chat, hoặc toast `TSe_TalkMsgFormPlus`.
- **Đính chính nhận định cũ** ("Handler 0x10 không dùng codec LE — mọi field số nằm trong hộp đen, không suy diễn"): **sai một phần** — codec không chạy ở tầng case nhưng chạy **trong lòng các callee**; mọi id/thời lượng trên nhóm SubOp 3–0x13 đều là Word/DWORD LE qua cùng hàm decode.

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

**Codec helper (đã đọc body):**
- `FUN_0077eb9c`: `b0 + b1*0x100` = **Word LE**. Tầng case 0x10 không gọi; **callee HOLE gọi nhiều lần** (`006136f0.c:56`, `00613920.c:56`, `00614318.c:57,61,64`).
- `FUN_0077ef7c`: `b0+b1*0x100+b2*0x10000+b3*0x1000000` = **DWORD LE**. Tầng case không gọi; **callee HOLE dùng dày đặc** (`00613240.c:97,114`, `00613454.c:45`, `00613b50.c:105,122`, `00613e48.c:59,68`, `006140d4.c:45`, `006144f8.c:88,102`) — tất cả decode qua key `*(gvar_007D9D30)`. (đính chính ghi chú cũ "Handler 0x10 không gọi codec, field số nằm trong hộp đen không suy diễn" → nay đã bóc được từng field)
- `FUN_0077eb1c` (encode Word), `FUN_0077ee84` (encode DWORD): chỉ dùng chiều C→S, không dùng ở S→C này.
- `FUN_0077f098` (copy tên 8 byte): handler này không gọi.
- Handler 0x10 tầng case chỉ đọc **byte độ dài thô** + `_LStrCopy` cắt chuỗi; **mọi field số nằm trong các hàm con và đã được bóc từ body mới** (bảng §4 "ĐÃ CÓ BODY").

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
| `0x03` | **11B** | `[10][03][idA:4B LE P2..5][mode:1B P6][idB:4B LE P7..10]` | `func_0x00613e48` — tra tên 2 actor, chèn dòng chat + set cờ `TSe_SendMailForm+0x131` (mục 4 "ĐÃ CÓ BODY") |
| `0x04` | **12B** | `[10][04][id:4B LE][w1:2B][w2:2B][w3:2B]` | `func_0x00614318` — in 1 dòng `id + 3 Word` vào memo form GM |
| `0x05` | **2B** | pass nguyên RP | `func_0x00614bb0` — body chỉ cấp phát buffer ~13KB rồi dọn chuỗi, **không thấy tác dụng** (chưa kết luận được) |
| `0x06` | **3B+** | `[10][06][n:1B][header:n][lặp: [id:4B][len:1B][name]]` | `func_0x006140d4` — in tiêu đề + danh sách `[id, name]` vào memo GM |
| `0x07` | **3B** | `[10][07][sel:1B]` sel 1–3 | `func_0x00733800(obj_007DA7BC, RP)` — chèn 1/3 dòng chat hệ thống (hằng mã hóa) + cờ `+0x131/+0x132` |
| `0x08` | **2B** `[10][08]` | không đọc field nào thêm | `FUN_007ab870(TTalkMsgForm, 0, UNK_00797120, 0)` + `TSe_SendMailForm+0x131=0` (1 dòng hiển thị) |
| `0x09` | **3B** | `[10][09][sel:1B]` sel 1–8 | `func_0x00614724` — in 1 trong 8 hằng mã hóa vào memo GM (sel 6–8 đổi memo đích) |
| `0x0A` | **2B+** | lặp record `[len][name][4B][4B ms][len2][str2]` | `func_0x00613b50` — in "name GMID:… ngày/giờ/phút" vào memo GM |
| `0x0B` | **2B+** | lặp record `[len][str][4B][4B ms]` | `func_0x006144f8` — in `str + id + phút` vào memo GM |
| `0x0C` | **2B+** | lặp record `[len][name][4B][4B ms]` | `func_0x00613240` — in "name GMID:<id> <phút>" vào memo GM |
| `0x0D` | **2B** `[10][0D]` | không đọc field nào thêm | `FUN_007ab870(TTalkMsgForm, 0, UNK_0079714c, 0)` + `TSe_SendMailForm+0x132=0` (1 dòng hiển thị) |
| `0x0E` | **4B+** | `[10][0E][count:2B][lặp count: [id:4B][len:1B][name]]` | `func_0x006136f0` — đổ danh sách `[id, name]` vào memo tab GM, count=0 → `ShowMessage` |
| `0x0F` | **4B+** | layout y hệt `0x0E` | `func_0x00613920` — như 0x0E nhưng template dòng khác (bản 2 của cùng UI list) |
| `0x10` | **2B** | không parse gì | `func_0x00613230` — **body rỗng `return;`** → no-op thật sự |
| `0x11` | **2B** | pass nguyên RP | `func_0x00614b58(TGmManage, RP)` — **VẪN HỘP ĐEN** (body chưa có trong `ts_decompile/`) |
| `0x12` | **6B+** | `[10][12][x:4B][msg: còn lại]` | `func_0x00613454` — chèn `msg` làm dòng chat hệ thống + phát `sound\WA0033.wav` |
| `0x13` | **2B** | không parse gì | `func_0x006146e4` — chỉ dọn chuỗi tham chiếu → **no-op** |
| `0x14` | **2B** | pass nguyên RP | `VMT+0x8c(TKC_Question, RP)` — hộp đen |
| `0x15` | **2B** | pass nguyên RP | `VMT+0x90(TKC_Question, RP)` — hộp đen |
| `0x16` | **2B** | pass nguyên RP | `VMT+0x8c(TWF_day55QuestionForm, RP)` — hộp đen |
| `0x17` | **2B** | pass nguyên RP | `VMT+0x8c(TWF_day55hardQuestion, RP)` — hộp đen |
| `0x18` | **2B** `[10][18]` | bỏ qua toàn bộ payload | `FUN_0070736c(TWF_Day55EasyForm)` — reset + 2 dòng + refresh (body đã đọc) |
| `0x19` | **3B** | `[10][19][sel:1B]` sel 1–10 | `func_0x007074cc` — toast `TSe_TalkMsgFormPlus` 5000ms với 1 trong 10 hằng mã hóa `DAT_007076b4…00707af0` |

Gửi thiếu byte so với cột "tối thiểu" → guard gọi `_BoundErr`, không phải no-op. **Đính chính ghi chú cũ:** các callee HOLE **không** tự chịu đựng payload ngắn — `0x09`/`0x19`/`0x12` BoundErr ngay khi thiếu byte chọn; nhóm lặp record (`0x0A–0x0F`) chỉ an toàn khi payload đúng layout.

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

### SubOp `0x03`–`0x07`, `0x09`–`0x0C`, `0x0E`–`0x13` — **ĐÃ CÓ BODY (HOLE `0x00612E1A→0x00614C38` đã được phủ)**. Không còn là "pass-through thuần":

**Khuôn chung (xác minh được từ body mới):** tất cả là method của `TGmManage` (`gvar_007DA160`, VMT_613160, khởi tạo `0050a4a0_TForm1.FormCreate.c:624-625`) nhận `(self=EAX, RestPayload=EDX)`; `self` được lưu nhưng **hầu như không dùng** — logic thao tác thẳng vào global. Field số đọc bằng đúng codec `FUN_0077ef7c` (DWORD LE) / `FUN_0077eb9c` (Word LE) với key `*(gvar_007D9D30)`; chuỗi nhúng trong các hàm này là **hằng mã hóa** đi qua bộ giải mã chuỗi `FUN_00614c38 → FUN_007c8000 → FUN_007c699c` (`00614c38_FUN_00614c38.c:50-60`, `007c699c_FUN_007c699c.c:157-595`) → **không dump `redump/` là không dịch được**.

| SubOp | Body | Hành vi đã xác minh |
|---|---|---|
| `0x03` | `00613e48_FUN_00613e48.c` | `Copy(RP,2,4)`→DWORD `idA` (`:58-59`), byte `RP[5]`→`mode` (`:60-66`), `Copy(RP,7,4)`→DWORD `idB` (`:67-68`). Tra tên từng id: nếu trùng `*(gvar_007DA7BC+4)` (self) thì lấy tên self `+9` (`:87-88`), ngược lại `FUN_00722508(gvar_007D9C48, id)` → index ≤ 0x834 → tên tại `gvar_007DA6BC[idx*4]+8` (`:74-83`). Cả hai tên hợp lệ: `mode==1` → `TSe_SendMailForm(gvar_007DA548)+0x131=1` + chat `FUN_007ab870(gvar_007DA1B0, selfId, msg)`; `mode==2` → cờ `=0` + chat (`:103-116`). msg = ghép hằng mã hóa `DAT_006140b4/006140c8` (`:105,112`). **Đính chính:** mục tiêu không phải "lệnh GM trừu tượng" — là **thông báo hệ thống dạng chat về 2 actor** (khả năng liên quan mail, chưa kết luận được nghiệp vụ). |
| `0x04` | `00614318_FUN_00614318.c` | Đọc `id DWORD (Copy 2,4)` + 3 Word `Copy(6,2)/(8,2)/(10,2)` (`:53-66`) → ghép 8 mảnh `IntToStr` + 4 hằng mã hóa (`:67-82`), decode, thêm vào memo `gvar_007DA110+0x384(+900)+0x208` (`:88`). |
| `0x05` | `00614bb0_FUN_00614bb0.c` | Chỉ stack-probe 2 buffer lớn (`local_336c[2038]` ~8KB) rồi `_LStrClr` dọn 3 chuỗi; **không có lệnh nào khác** (`:29-50`). hoặc stub hoặc decompile chưa đủ — **chưa kết luận được**. |
| `0x06` | `006140d4_FUN_006140d4.c` | Byte `RP[1]`=n → `Copy(RP,3,n)` làm tiêu đề, prepend hằng `DAT_006142e4`, in memo (`:61-72`). Vòng lặp tới hết RP: `[id:4B LE][len:1B][name:len]` → in `"… Name:"+id+name` (`:80-129`). Điều kiện dừng dùng `LStrLen` nhưng bản C hiện hằng địa chỉ (`iVar5=0x61417d`) — artifact decompiler, bán chất vòng lặp đã kiểm qua `local_1c += len+5`. |
| `0x07` | `00733800_FUN_00733800.c` | Guard `len<2`, byte `RP[1]` (`:24-28`); sel 1 → chat `FUN_007ab870(gvar_007DA1B0,0,DAT_007338ac,0)` + `gvar_007DA548+0x131=1`; sel 2 → hằng `007338dc` + cờ `+0x131=1`; sel 3 → `00733910` + `+0x132=1` (`:29-40`). **Không có sel khác**. |
| `0x09` | `00614724_FUN_00614724.c` | Guard `len<2→BoundErr`, byte `RP[1]` làm **switch 1..8** (`:58-65`); mỗi nhánh: `FUN_00614c38(hằng mã hóa DAT_00614a08…00614b28)` → `_LStrFromWStr` → memo. sel 1–5 → memo `+0x384`; **sel 6–8 → memo khác `gvar_007DA110+0x2e4+0x208`** (`:113,121,129`). |
| `0x0A` | `00613b50_FUN_00613b50.c` | Lặp khi `len(RP)>2` (`:67-69`): record `[len:1B][name:len][4B (decode, bỏ kết quả)][4B = ms][len2:1B][str2:len2]`; cursor += `len+len2+10` (`:123-167`). In `"name"+ " GMID:" (literal thô tại `:169`) + IntToStr + ms→ **ngày/86400000, giờ/%86400000/3600000, phút/%3600000/60000** (`:175-183`) + str2 → memo `+0x384` (`:189-191`). |
| `0x0B` | `006144f8_FUN_006144f8.c` | Lặp record `[len][str:len][4B→id][4B→ms]`, cursor += `len+9` (`:57-111`); in `id + ms/60000` (phút) qua 7 mảnh (`:112-115`) → memo `+0x384` (`:116`). |
| `0x0C` | `00613240_FUN_00613240.c` | Cùng khuôn 0x0B (`[len][name][4B id][4B ms]`, `:61-123`); in `" GMID:"` (`:125`) + `IntToStr(id)` (`:127`) + `ms/60000` (`:131`, giá trị nhân đôi nhãn `&DAT_00613418` là artifact) → memo `+0x384` (`:135-141`). |
| `0x0E` | `006136f0_FUN_006136f0.c` | `count=Word(Copy(RP,2,2))` (`:55-56`); `count==0` → `ShowMessage(DAT_006138e0)` (`:59`); else: gọi VMT+0x40 xóa memo (`gvar_007DA110+0x2dc`'s control `+0x208`, `:62`), lặp `count` record `[id:4B][len:1B][name]` (`:66-100`) in `IntToStr(id)+name` (VMT+0x34 add, `:116`), rồi `TPageControl_SetActivePageIndex(gvar_007DA110+0x2d0, 0)` + `FUN_0044ce98(gvar_007DA110,…)` (`:120-121`). |
| `0x0F` | `00613920_FUN_00613920.c` | **Bản sao gần như 1-1 của 0x0E** (cùng layout, cùng UI; template dòng hằng `DAT_00613b10/00613b48` khác) — `:55-116`. |
| `0x10` | `00613230_FUN_00613230.c` | **Body rỗng** — 16 byte, `return;` (`:16-20`). Gửi `[10][10]` = không tác dụng gì. |
| `0x12` | `00613454_FUN_00613454.c` | `Copy(RP,2,4)` → DWORD (decode xong **không dùng**, `:44-45`); `Copy(RP,6, len(RP)-5)` = toàn bộ message → chat hệ thống `FUN_007ab870(gvar_007DA1B0, 0, msg, 0)` (`:53-55`) + phát `sound\WA0033.wav` qua `FUN_007a7f20` (`:56-57`). |
| `0x13` | `006146e4_FUN_006146e4.c` | 53 byte, chỉ setup/cleanup exception frame + `_LStrClr` (`:27-36`) → **no-op**. |

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

### SubOp `0x19` — Toast 5000ms theo selector (ĐÃ CÓ BODY: `007074cc_FUN_007074cc.c`)
- Guard `len<2` (BoundErr), đọc byte `RP[1]`, `switch` 1–10 (`:39-74`).
- Mỗi nhánh: **toast `TSe_TalkMsgFormPlus`** qua VMT `gvar_007DA084+0x90` với tham số `(text=DAT_007076b4…00707af0, 5000ms, 0, 1)`; sel >10 hoặc 0 → rơi qua.
- 10 hằng `DAT_007076b4…00707af0` **không có `redump/` → chưa dịch được** (đúng khuôn 4 byte duration + arg cuối=1 như các toast OP 0x18).

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

Ghi chú bảng mã **(đính chính 2026-09-14)**: các dump `lit_7967f8/797ee8/797fe0.hex` decode sạch bằng **VISCII**, không phải cp1258 (cp1258 cho mojibake) — đối chiếu `opcode_17.md` §6 / `opcode_13.md` §7. Kết luận OP 0x10 không đổi: các hằng dưới đây **vẫn chưa có dump** nên chưa dịch được dù dùng bảng nào. OP này tham chiếu các hằng sau, **tất cả đều chưa có dump nên chưa dịch được — ghi rõ, KHÔNG bịa nội dung**:

| Địa chỉ | Dùng ở | Trạng thái dump |
| :--- | :--- | :--- |
| `UNK_0079706c` | SubOp 2 sel=1, toast 0x4B0 | **không có `lit_7970*.hex` → chưa dịch được** |
| `UNK_007970a8` | SubOp 2 sel=2, toast 0x4B0 | như trên |
| `UNK_007970cc` | SubOp 2 sel=3, toast 1000 | như trên |
| `UNK_007970f8` | SubOp 2 sel=9, toast 1000 | như trên |
| `UNK_00797120` | SubOp 8, `FUN_007ab870(...,tag 0)` | **không có `lit_7971*.hex` → chưa dịch được** |
| `UNK_0079714c` | SubOp 0xD, `FUN_007ab870(...,tag 0)` | như trên |
| `DAT_007073fc`, `DAT_00707460`, `DAT_00707474` | `FUN_0070736c` (SubOp 0x18) | **không có `lit_7073/7074*.hex` → chưa dịch được** |

`redump/` hiện chỉ có `lit_7ABDxx`, `lit_77F771/78A854/7A2094/7A20A8`, `lit_5957xx/596078`, `lit_797ee8/797fe0/...` (không phủ `0x0079706c–0x0079714c`, `0x007073xx–0x00707af0`) — không có file nào phủ các địa chỉ trên.

**Bổ sung 2026-09-14 — hai cơ chế chuỗi mới phát hiện từ body HOLE:**
1. Literal thô đọc được ngay trong code: `" GMID:"` (`00613240_FUN_00613240.c:125`, `00613b50_FUN_00613b50.c:169`), `" Name:"` (`006140d4_FUN_006140d4.c:119`) — nhãn danh sách GM.
2. Ngoài các literal trên, **mọi nhãn khác trong nhóm callee `0x00613xxx/0x00614xxx` (`DAT_006134xx/43xx/44xx/8e0/918/b10/b48/de4…e3c`, `DAT_006142e4…00614b28`) là chuỗi MÃ HÓA Delphi-style**, chỉ ra text sau khi qua `FUN_00614c38 → FUN_007c8000 → FUN_007c699c` — bảng escape-jumptable tại `007c699c_FUN_007c699c.c:165-585`. Ghidra hiển thị chúng thànhwide-literal rác (`L"痿棄㑌a..."`). **Không có dump các `DAT_` này + chưa áp bảng escape → chưa dịch được** — ghi rõ, không bịa.

---

## 7. Ghi chú cho Mock Server

1. **Luôn gửi `[0x10][SubOp]...`.** Gửi `[10]` đơn độc (thiếu SubOp) → `_BoundErr(0)`.
2. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes từ Token tới payload XOR `0xAD` từng byte.
3. **Ví dụ tính sẵn** (`byte^0xAD`: `0x10^0xAD=0xBD`, `0x02^0xAD=0xAF`, `0x09^0xAD=0xA4`, `0xF4^0xAD=0x59`, `0x44^0xAD=0xE9`):
   - SubOp 2 sel=9 tối thiểu: payload plain `10 02 09` → socket payload `BD AF A4`.
   - Plain frame (7B): `F4 44 03 00 10 02 09` → socket frame: `59 E9 AE AD BD AF A4`.
   - SubOp 1 tối thiểu có tên `"A"`: payload plain `10 01 01 41` → socket payload `BD AC AC EC`. Lưu ý gói này vẫn thiếu 3 chuỗi đáp án + tail nên client sẽ đọc lố/guard — chỉ dùng để test guard, không phải gói hợp lệ đầy đủ.
4. **Kịch bản test an toàn → nguy hiểm:**
    - Trước: SubOp `0x02` với sel lạ (ví dụ `10 02 05`) → rơi qua no-op, quan sát không crash. `10 10` và `10 13` cũng đã xác minh là **no-op thật** từ body mới (`00613230`, `006146e4`) — an toàn để test framing.
    - Sau: SubOp `0x08` / `0x0D` (`10 08`, `10 0D`) → chèn dòng hệ thống, nội dung phụ thuộc hằng chưa dịch — quan sát text hiện ra để đoán nội dung thật.
    - ĐÃ BÓC ĐƯỢC (không còn là "hộp đen cuối cùng"): `0x03,0x04,0x06,0x07,0x09,0x0A,0x0B,0x0C,0x0E,0x0F,0x12,0x19` parse payload thật — gửi đúng layout §3; sai layout → `_BoundErr` giữa package (client pop-up lỗi/rơi exception frame, test cách ly + log).
    - Vẫn phải test cách ly: SubOp `0x01` full (tên + 3 đáp án + đuôi), `0x05` (body không tác dụng quan sát được), và `0x11` (callee `0x00614b58` **chưa có body**).
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
| 9 | `0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | body codec | Word/DWORD LE — tầng case không gọi, **callee HOLE gọi** (xem §1) |
| 10 | `0070736c / 007ba180 / 007b8060 / 007ab870` | body helper | Reset/prepend/chat (tóm 1 dòng hiển thị) |
| 11 | `0051189c / 0050a4a0` | gán global | `TGmManage`, `TAC_Question`, `TKC_Question`, `TWF_day55*`, `TWF_Day55EasyForm`, `TSe_SendMailForm`, `TTalkMsgForm`, `TSe_TalkMsgFormPlus` |
| 12 | **body HOLE mới** `00613230 / 00613240 / 00613454 / 006136f0 / 00613920 / 00613b50 / 00613e48 / 006140d4 / 00614318 / 006144f8 / 006146e4 / 00614724 / 00614bb0 / 007074cc / 00733800` (`ts_decompile/functions/*.c`, có trong `index.csv`) | toàn file | **Bóc field + hành vi 15 callee**; `00614b58` vẫn vắng mặt |
| 13 | `00614c38_FUN_00614c38.c:37-61`, `007c8000_FUN_007c8000.c:51`, `007c699c_FUN_007c699c.c:157-595` | chuỗi mã hóa | Bộ giải mã chuỗi Delphi (`FUN_007c699c`) dùng bởi các memo-line GM |
| 14 | `00722508` (tra index actor→`gvar_007DA6BC`), `0070c20c` (id→slot), `007a7f20` (play WAV), `0044ce98`/`TPageControl_SetActivePageIndex` | callee | Hỗ trợ đọc ý nghĩa `0x03/0x06/0x0E/0x12` |

**Giới hạn (cập nhật 2026-09-14):**
- `func_0x00614b58` (SubOp 0x11) **vẫn chưa có body** trong `ts_decompile/functions/` (không có trong `index.csv`) → còn hộp đen.
- `00614bb0` (SubOp 0x05): decompile chỉ thấy cấp phát + dọn → hoặc rỗng thật hoặc mất code; **chưa kết luận được**.
- `00613e48` (SubOp 0x03), `00614bb0`, `006140d4`: có hằng số biểu diễn thành wide-literal rác (hằng chuỗi Delphi nội bộ `0x0061409c…`) — tên nội dung chưa dịch.
- Nội dung toast `0x19` (10 hằng `0x007076b4…`) + 6 hằng `UNK_007970xx/00797120/0079714c`: **vẫn chưa có `redump/` phủ** → chưa dịch, không bịa.
- Các call `VMT+0x8c/0x90/0x6c/0x7c/0x20/0x24` vẫn là method ảo chưa resolve từ tầng case.
- Ý nghĩa game-design (chức năng GM cụ thể, quiz Day55) nằm ngoài tầng case.
