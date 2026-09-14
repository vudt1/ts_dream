# PHÂN TÍCH — Main OP 0x02 (Case 3, `FUN_0078b6fc` @ `0x0078B6FC`)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (case function + helper + giải mã chuỗi constant + chiều C→S `FUN_0077f414`).

---

## 0. Kết luận quan trọng nhất (đính chính định hướng nghiệp vụ)

Tài liệu hướng dẫn (`.scratch/op-code/handoff-opcode-exploration-guide.md`, mục 3) **phỏng đoán OP 0x02 = "Chọn / Tạo nhân vật, danh sách nhân vật sau đăng nhập"**. **Phỏng đoán này SAI.** Bằng chứng từ mã nguồn sơ cấp cho thấy:

> **Main OP 0x02 thực chất là KÊNH ĐỒNG BỘ TIN NHẮN / CHAT / THÔNG BÁO (Message & Chat-channel delivery).**

Chuỗi bằng chứng quyết định:
1. **Mọi sub-op đều gọi chung một hàm** `FUN_007ab870` (`ts_decompile/functions/007ab870_FUN_007ab870.c`) với đối số 1 là `gvar_007DA1B0`. Global này được gán trong `ts_decompile/functions/0051189c_FUN_0051189c.c:1590` từ `VMT_7AB774_TTalkMsgForm` → **bảng tin nhắn / chat log** (TTalkMsgForm).
2. **Đối số 4 của `FUN_007ab870` (param_4) = "tag kênh"** (0..7, 0x0B) mà nội bộ nó dùng để **nối tiền tố nhãn kênh** vào dòng hiển thị (`_LStrCat3`/`_LStrCatN`). Các tiền tố này là chuỗi tiếng Việt có dấu (VISCII đơn-byte — xem đính chính mục 5) định vị tại `0x7ABD54`… đã giải mã trực tiếp từ dump hằng số (xem mục 5).
3. **Các byte điều kiện `ECX[1]` được gate bằng cờ kênh** trong `gvar_007DA6E0` = `TFT_ChannelForm` (`0051189c_FUN_0051189c.c:1592`), các offset `+0x168…+0x16D` được **khởi tạo = 1 cho 6 kênh** trong vòng lặp `for(local_10=0..5)` của `ts_decompile/functions/006037dc_FUN_006037dc.c:178`.
4. Hàm "đệ trình" `case 0x0C` ghi trực tiếp dòng `ECX[1..]` như một **thông báo hệ thống thuần** (id=0), không có cấu trúc nhân vật nào (không có byte class/race/name-slot, không dựng grid danh sách).

→ Không hề có logic "liệt kê nhân vật", "chọn slot", hay "tạo nhân vật" trong handler. Nghiệp vụ **chọn/tạo nhân vật** nằm ở các OP khác (không phải 0x02).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Nhận một dòng tin nhắn kèm nhãn từ server và **đẩy vào cửa sổ chat/log phía client**, đồng thời ghi vào bộ đệm vòng (ring-buffer) 100 dòng của form chat.
- **Payload chuẩn gần như mọi nhánh**: `[MainOp=0x02][SubOp][id: 4B LE][message: chuỗi thô tới hết payload]`, trong đó `id` = **mã người chơi/đối tượng nguồn** (để suy tên + gán màu + kiểm tra ignore), `message` = nội dung thô (Delphi string, không prefix độ dài trong payload này).
- **SubOp = "tag kênh"** chọn tiền tố hiển thị (`(Công bố hệ thống)`, `(GM)`, `(Đài)`, `(Đoàn)`, `(Minh)`…), và một số sub-op bị **gạt theo cờ bật/tắt kênh** của `TFT_ChannelForm`.
- **Đồng bộ dữ liệu thực sự**: cập nhật `TTalkMsgForm` (log chat + ring buffer màu/id), và một sub-op (`0x08`) chỉ để **gắn cờ timestamp lên thanh nhập** `TSe_InputBar` (cooldown/typing).

---

## 2. Entry & cách đọc PacketBuffer (S→C)

**Entry**: `FUN_0078b6fc` @ `0x0078B6FC` (Case 3, jump-target `0x0078A9C2` của bảng `0x78A9B6`; ánh xạ `MainOp 0x02 → byte_table[0x02]=0x03` theo `handoff-opcode-exploration-guide.md`).

**Đầu hàm (dòng 36–43 của file C)**:
```c
iVar7 = *(int *)(unaff_EBP + -0xc);           // ECX = RestPayload (đã cắt bỏ MainOp)
...                                              // (bounds-check Delphi qua *(iVar7-4))
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar7 + 0); // SubOp = RestPayload[0]
switch(*(undefined4 *)(unaff_EBP + -0x14)) { ... }
```
- `unaff_EBP + -0xc` = **RestPayload**: toàn bộ payload trừ 1 byte MainOp (xem `opcode_00_01.md` mục 2.2: `_LStrCopy(local_c, 2, len-1)`). Do đó `RestPayload[0]` (0-based) = **SubOp** = byte thứ 2 của frame payload gốc.
- **SubOp** đọc bằng 1 byte thường (không codec).

**Cách đọc field trong từng nhánh** (1-based `_LStrCopy`, tài liệu hướng dẫn mục Nguồn 2):

| Phép đọc | Ý nghĩa trên wire | Giải mã |
| :--- | :--- | :--- |
| `_LStrCopy(RestPayload, 2, 4, &tmp)` | `RestPayload[1..4]` = **payload[2..5]** = `id` | `FUN_0077ef7c` → **DWORD Little-Endian** (b0+b1·256+b2·65536+b3·2^24) |
| `uVar5=_LStrLen(RestPayload); uVar6=uVar5-5; _LStrCopy(RestPayload, 6, uVar6, &msg)` | `RestPayload[5..end]` = **message** | giữ nguyên chuỗi thô (ghi vào `param_3` của `FUN_007ab870`) |
| `_LStrCopy(RestPayload, 2, uVar5-1, &msg)` (chỉ `case 0xC`) | `RestPayload[1..end]` = **message** (không có id) | `id` truyền hằng số `0` |

**Không có** nhánh nào dùng `FUN_0077eb9c` (Word LE) hay `_LStrArrayClr` phân tích payload; toàn bộ dùng DWORD LE (4B) cho id + chuỗi biến độ dài tới hết gói.

**Hàm hội tụ `FUN_007ab870(int form, int id, char* msg, char tag)`** (`007ab870_FUN_007ab870.c`):
```c
if (myId==id) name := PlayerRecord+9                 // gvar_007DA7BC -> self
else          FUN_0075ddb8(id,&name)                 // tra cache tên 2100 slot (gvar_007DA6BC) qua FUN_00722508
if (FUN_005631c4(id) != 0) return;                    // id nằm trong danh sách IGNORE (DAT_00949284) → BỎ TIN
switch(tag) { ...name := tiền_tố_nhãn + name ... }     // các case 0..7,0x0A,0x0B (xem bảng)
if ((id<100)||(id>400)) FUN_007ad614(owner+0x130, line, tag, id, 1)   // đẩy vào log, color=tag
else                    FUN_007ad614(owner+0x130, line, 4,   id, 1)   // NPC/kênh hệ thống → color=4
```
- `FUN_007ad614` (`007ad614_FUN_007ad614.c`): tách `NAME:msg` theo `":"`, **kiểm tra bộ lọc từ** qua `FUN_0051ac3c` (form `gvar_007DA37C`); ghi vào **mảng vòng 100 phần tử** `[string, color, id]` stride 3 (`local_8[0x51]`), rồi gọi **VMT+0x88** của form để hiển thị. Đây chính là **bộ đệm + widget danh sách chat**.
- `FUN_005631c4` (`005631c4_FUN_005631c4.c`): duyệt mảng `DAT_00949284` so `id` → **danh sách chặn/ignore**; khớp thì cả tin bị丢弃 (return sớm trong `FUN_007ab870`).

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` tại dòng 43; các **case tồn tại**: `0,1,2,3,4,5,6,7,8,0xB,0xC` (**không có** `9, 0xA`, **không có `default`**).

| SubOp | Cờ gate (byte trong payload) | Đọc field | `tag`→`FUN_007ab870` | Tiền tố / hiệu ứng | Ghi chú |
| :---: | :--- | :--- | :---: | :--- | :--- |
| `0x00` | không gate | id(4B LE)+msg | `0` | `(Công bố hệ thống)` (`0x7ABD54`) | Thông báo toàn server / broadcast hệ thống |
| `0x01` | gate kép: `ChannelForm+0x16D` **hoặc** `PlayerRec+0x44+4==0xB3B6` ; và `class`∈[5..8] (`FUN_004c9bf0`-5U<4 qua `!(3<...)`) | id+msg | `1` | `(Thần)Thiên thần` (`0x7ABD70`); id∈[100,400] → `+IntToStr(id)+"(Gần)Thiên thần"` và `FUN_005964c8` (InputBar+0x19c/0x1a0) | Thư "thiên thần" / GM-broadcast có điều kiện; có thể đổi tag→`1` |
| `0x02` | `ChannelForm+0x168` | id+msg (truy `FUN_0070c20c` map-actor) | `2` | `(Gần)Thiên thần` (`0x7ABDC0`) / `(Gần)` (`0x7ABDF0`); id∈[100,400] → `"Hơi nhỏ tiếng"` (`0x7ABDD8`) | Chat kênh gần (near/normal). `FUN_0070c20c` chỉ tra actor, **kết quả không dùng thêm** |
| `0x03` | `ChannelForm+0x169` | id+msg | `3` | id==100 → `Hệ thống:` (`0x7ABE00`); id∈[100,400] → `(GM)Thiên thần` (`0x7ABE14`) + `IntToStr`; else `(Thì Thầm)` (`0x7ABE40`) + `FUN_005961a0` (log whisper tới NPC-list) | Kênh riêng/thì thầm (whisper), có xử lý NPC id 100..400 |
| `0x04` | không gate | id+msg | `4` | `(GM)` (`0x7ABE54`) | Tin nhãn GM (self hay không đều gắn `(GM)`) |
| `0x05` | `ChannelForm+0x16A` | id+msg (tên tự resolve) | `5` | `(Đài)` (`0x7ABE64`) | Kênh loa/đài phát (megaphone). **Chỉ hiển thị nếu resolve được tên** (`-0x30 != 0`) |
| `0x06` | `ChannelForm+0x16B` | id+msg | `6` | `(Đoàn)` (`0x7ABE74`) | Kênh đoàn/phái (guild/party) |
| `0x07` | `ChannelForm+0x16C` | id+msg | `7` | `(Minh)` (hardcode) | Kênh tự nói/bản địa |
| `0x08` | không đọc field | — | — | `FUN_005965e8(InputBar)` | **Không phải tin nhắn.** Gắn cờ `InputBar+0x1A5=1`, `+0x1A8=GetTickCount` khi `class`∈[5..8] |
| `0x0B` | không gate | id+msg | `0x0B` | `(Tổng Cũ)` (`0x7ABDAC`) rồi `#end`-terminate | Thư/memo dài: tích lũy vào `form+0x16C` tới khi gặp `"#end"` → flush qua `VMT+0x34` |
| `0x0C` | không gate | msg (id=0) | `0` | `(Công bố hệ thống)` | Dòng hệ thống thuần, id cố định `0`, msg=`RestPayload[1..]` |

`case 9`, `case 0xA`: **không có nhánh → gói rơi qua switch, không làm gì** (thanh ghi an toàn cho Mock Server).

**Toàn bộ các hàm hiển thị/đồ họa/animation** chỉ đi qua VMT (`+0x34` Add, `+0x88` AddLines, `FUN_005961a0` thêm item, `FUN_005964c8`/`FUN_005965e8` set cờ) → **tóm lược 1 dòng, không mổ xẻ** theo yêu cầu.

---

## 4. Chi tiết từng SubOp (wire layout + logic)

Quy ước: `P[.]` = byte payload gốc sau header (`P[0]=0x02 MainOp`, `P[1]=SubOp`); `RP[.]` = RestPayload (`RP[0]=P[1]`). endian **LE**.

### SubOp `0x00` — Broadcast hệ thống
- Wire: `[0x02][0x00][id: P2..P5 = RP1..RP4, 4B LE][msg: RP5..end]`
- Logic: `id=FUN_0077ef7c(RP[1..4])`, `msg=RP[5..]`, gọi `FUN_007ab870(TTalkMsgForm, id, msg, 0)`. `tag 0` → `_LStrCat3(name, "(Công bố hệ thống)"@0x7ABD54, msg)`. Nếu `id` bị ignore →丢弃. Không gate cờ kênh.

### SubOp `0x01` — Thiên thần / GM-broadcast có điều kiện
- Wire: `[0x02][0x01][id: 4B LE][msg]`
- Gate (dòng 61–64): `class=FUN_004c9bf0(PlayerRec+4)` (mã lớp nhân vật của **người nhận**) phải ∈[5..8] (`!(3<(class-5))`), **VÀ** `(**form_0x404_VMT+0xB4)()` khác 0; **VÀ** (`ChannelForm+0x16D != 0` **HOẶC** `*(short*)(PlayerRec+0x44+4) == 0xB3B6 (45974)`). Chỉ khi đó mới xử lý.
- Logic: đọc `id`+`msg`, đặt `in_stack_00000000=1`, gọi `FUN_007ab870(...,tag=1)`. Trong `FUN_007ab870` case 1: id∈[100,400] → `"Thiên thần"+msg+":"+name+"(Thần)Thiên thần"+IntToStr(id)+"…"` và `FUN_005964c8(InputBar)`; ngược lại nối `(Thần)Thiên thần`.

### SubOp `0x02` — Chat kênh gần
- Wire: `[0x02][0x02][id: 4B LE][msg]`
- Gate: `*(char*)(ChannelForm+0x168) != 0`.
- Logic: ngoài đọc id+msg còn gọi `FUN_0070c20c(gvar_007D9D34=map scene, id)` (tra actor trong 800 slot — `0070c20c_FUN_0070c20c.c`) gán `local_18`, **nhưng không dùng giá trị trả về** cho bất kỳ quyết định nào; sau đó `FUN_007ab870(...,tag=2)`. Nhãn `(Gần)`.

### SubOp `0x03` — Thì thầm / Whisper (có xử lý NPC)
- Wire: `[0x02][0x03][id: 4B LE][msg]`
- Gate: `*(char*)(ChannelForm+0x169) != 0`.
- Logic: `FUN_007ab870(...,tag=3)`. Trong tag 3: `id==100` → prefix `Hệ thống:`; `id∈[100,400]` (NPC/kênh hệ thống) → `"…(GM)Thiên thần"+IntToStr(id)+": "`; **else** (id<100 hoặc >400, người chơi thường) → gọi `FUN_005961a0(InputBar, name, 0)` (thêm vào danh sách whisper-nguồn) rồi prefix `(Thì Thầm)` (`0x7ABE40`).

### SubOp `0x04` — Nhãn GM
- Wire: `[0x02][0x04][id: 4B LE][msg]`. Không gate. `FUN_007ab870(...,tag=4)` → `_LStrCatN(name, msg, ":", name, "(GM)"@0x7ABE54)` cho cả 2 nhánh self/khác (hành vi như nhau).

### SubOp `0x05` — Loa/Đài (chỉ khi resolve được tên)
- Wire: `[0x02][0x05][id: 4B LE][msg]`
- Gate: `*(char*)(ChannelForm+0x16A) != 0`.
- Logic (dòng 145–165): `local_30 := ""`; nếu `id==myId` → `local_30 := PlayerRec+9` (tên self), **ngược lại** `idx=FUN_00722508(gvar_007D9C48,id)` (tìm trong cache 2100 slot `gvar_007DA6BC`, `00722508_FUN_00722508.c`) → `local_30 := rec[idx]+8`. **Chỉ khi `local_30 != 0` (resolve được tên)** mới gọi `FUN_007ab870(...,tag=5)` → prefix `(Đài)` (`0x7ABE64`).

### SubOp `0x06` — Đoàn/Phái — `[0x02][0x06][id:4B][msg]`, gate `ChannelForm+0x16B`, tag 6 → `(Đoàn)` (`0x7ABE74`).

### SubOp `0x07` — Bản địa/Minh — `[0x02][0x07][id:4B][msg]`, gate `ChannelForm+0x16C`, tag 7 → `(Minh)` (hardcode trong asm).

### SubOp `0x08` — Cờ thanh nhập (KHÔNG phải tin nhắn)
- Wire: `[0x02][0x08]` (2 byte payload, **không field**).
- Logic: `FUN_005965e8(gvar_007DA1DC=TSe_InputBar)` (`005965e8_FUN_005965e8.c`): nếu `class(PlayerRec+4)`∈[5..8] → `InputBar+0x1A5=1`, `InputBar+0x1A8=GetTickCount()`. Không đụng tới TTalkMsgForm → **đây là hiệu ứng UI/cooldown trạng thái nhập**, không phải đồng bộ dữ liệu chat.

### SubOp `0x0B` — Memo/thư dài (`#end`-terminated)
- Wire: `[0x02][0x0B][id: 4B LE][msg]`
- Logic: `FUN_007ab870(...,tag=0x0B)`; trong tag 0x0B (`007ab870.c:323–352`): nếu `form+0x162==0` (đang tích lũy) → `_LStrCat(form+0x16C, "(Tổng Cũ)"+msg)`; khi chuỗi chứa `"#end"` (`0x7ABE94`) → cắt bỏ `#end`, flush `form+0x16C` qua `VMT+0x34` (Add) rồi xóa bộ đệm; nếu chưa có `#end` tiếp tục gom. `form+0x162=1`, clear `+0x16C`. → **cơ chế nhận message chia nhiều gói**.

### SubOp `0x0C` — Dòng hệ thống thuần (id=0)
- Wire: `[0x02][0x0C][msg: RP1..end]` (**không có id**)
- Logic: `msg=_LStrCopy(RestPayload,2,len-1)`, gọi `FUN_007ab870(TTalkMsgForm, 0, msg, 0)`. Với `id=0`: `FUN_005631c4(0)` thường false, `FUN_0075ddb8(0)` → name rỗng; tag 0 → hiển thị `"(Công bố hệ thống)" + msg`.

---

## 5. Giải mã bảng hằng số nhãn kênh (bằng chứng chuỗi)

Trích từ `ts_decompile/redump/lit_7ABD54/7ABDAC/7ABDF0/7ABE40/7ABE64/7ABE74.hex` (Delphi `ansistring`: `[len:4LE][chars][00...]`; giải mã **VISCII đơn-byte tiền tổ hợp → NFC**). Đây là các chuỗi `param_4` được `FUN_007ab870` nối vào tên:

> **Đính chính 2026-09-14**: bản cũ ghi recipe là "cp1258 → NFC". Khi đối chiếu bằng chứng byte mới (15 toast `opcode_09.md §7.1`, `lit_797ee8` `opcode_17.md`, `lit_7282ec/728300` `opcode_19.md`): pipeline `cp1258+NFC` trả kết quả vô nghĩa (`63 A7 70` → `c§p`), còn bảng **VISCII (RFC 1456, đơn-byte tiền tổ hợp)** đọc đúng 100% — áp dụng cả cho chính `lit_7ABD54.hex` ở bảng dưới (`f4=ô, af=ố, ae=ệ`). Cột "Byte" giữ nguyên; nhãn mã hóa đúng là **VISCII → NFC**.

| Địa chỉ | Byte | VISCII (trước NFC) | Tiếng Việt (NFC) | Vai trò |
| :--- | :--- | :--- | :--- | :--- |
| `0x7ABD54` | `28 43 f4 6e 67 20 62 af …` | `(Công bâ… h® th¯ng)` | **(Công bố hệ thống)** | tag 0 / 0x0C |
| `0x7ABD70` | `28 54 68 a5 6e 29 …` | `(Th¥n)Thiên th¥n` | **(Thần)Thiên thần** | tag 1 |
| `0x7ABDAC` | `28 54 a4 74 20 43 e4 29` | `(T¤t Cä)` | **(Tổng Cũ)** | tag 0x0B |
| `0x7ABDC0` | `28 47 a5 6e 29 …` | `(G¥n)Thiên th¥n` | **(Gần)Thiên thần** | tag 2 |
| `0x7ABDD8` | `48 d5 6f …` | `HƠo nhö tiªng` | **Hơi nhỏ tiếng** | tag 2 (id∈[100,400]) |
| `0x7ABDF0` | `28 47 a5 6e 29` | `(G¥n)` | **(Gần)** | tag 2 |
| `0x7ABE00` | `48 ae 20 … 3a` | `H® th¯ng:` | **Hệ thống:** | tag 3 (id==100) |
| `0x7ABE14` | `28 47 4d 29 …` | `(GM)Thiên th¥n` | **(GM)Thiên thần** | tag 3 (100≤id≤400) |
| `0x7ABE40` | `28 54 68 ec …` | `(Th́ Th¥m)` | **(Thì Thầm)** | tag 3 (whisper) |
| `0x7ABE54` | `28 47 4d 29` | `(GM)` | **(GM)** | tag 4 |
| `0x7ABE64` | `28 d0 b5 69 29` | `(Đµi)` | **(Đài)** | tag 5 (megaphone) |
| `0x7ABE74` | `28 d0 6f e0 6e 29` | `(Đoàn)` | **(Đoàn)** | tag 6 (guild) |
| `0x7ABE84` | `28 4d 69 6e 68 29` | `(Minh)` | **(Minh)** | tag 7 (inline) |
| `0x7ABE94` | `23 65 6e 64` | `#end` | **#end** | sentinel tag 0x0B |

Toàn bộ đều là **nhãn kênh chat / thông báo**, xác nhận chắc chắn kết luận mục 0.

---

## 6. Chiều ngược lại Client → Server (C→S) liên quan

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`, `switch(param_2 & 0xff)`):

```c
case 2:
  break;      // ← dòng 819–820
```

**OP 0x02 phía gửi (C→S) là rỗng / no-op.** Client **không** phát hành động gói tin nào bằng opcode 0x02; nó chỉ **nhận** đồng bộ chat. Các hành động gửi tin của người chơi (đánh chat, đổi kênh, …) nằm ở opcode **0x1A** (Talk — xem `opcode_1a.md`), **không phải 0x02**. → Không có bất đối xứng cần dựng cho mock server ở opcode này.

---

## 7. Ghi chú cho Mock Server

1. **Không dùng OP 0x02 cho "chọn/tạo nhân vật"**. Nếu cần luồng nhân vật, tìm OP khác (0x02 chỉ chat).
2. **Định dạng phát tin**: `payload = [0x02][SubOp][id:4B LE][msg_bytes]`; XOR khóa tĩnh `0xAD` toàn frame, header `[0xF4 0x44][L:2B LE]` (L không tính header). Gửi đúng pacing: client nhận tối đa 50 gói/tick-30ms.
3. **Chọn SubOp theo kênh**:
   - Broadcast/hệ thống: `0x00` hoặc `0x0C` (0x0C không cần id).
   - Chat thường: `0x02`; whisper: `0x03`; GM: `0x04`; loa: `0x05`; đoàn/phái: `0x06`; tự nói: `0x07`.
   - Thư dài chia mảnh: `0x0B` (kết thúc bằng `#end`).
4. **id phải nằm trong khoảng hợp lệ để không bị ép màu**: client phân nhánh hiển thị theo `id<100 || id>400` (người chơi) vs `100≤id≤400` (NPC/hệ thống). Với người chơi, `id` phải **tồn tại trong cache tên 2100 slot** (`gvar_007DA6BC`) để sub-op `0x05` hiển thị (điều kiện `name!=0`). Nếu `id==myId` client tự gắn tên chính mình.
5. **Cờ kênh**: các sub-op `0x01..0x07` bị **lọc theo cờ bật kênh** (`TFT_ChannelForm +0x168..+0x16D`, mặc định =1 khi khởi tạo). Mock server không cần dựng; nhưng khi test client hãy để nguyên form chat/kênh đã init.
6. **Ignore-list**: nếu `id` có trong mảng `DAT_00949284`, tin bị丢弃 không hiển thị (`FUN_005631c4`).
7. **Không gửi SubOp `9`/`0xA`** — client không có nhánh, rơi qua switch (vô hại nhưng vô nghĩa).
8. **Không gửi SubOp `0x08` kèm field thừa** — nhánh này bỏ qua payload, chỉ tác động cờ thanh nhập.

---

## 8. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
| :--- | :--- | :--- | :--- |
| 1 | `ts_decompile/case_functions/functions/case_003_0078B6FC_FUN_0078b6fc.c` | @`0x0078B6FC`; switch d.43; SubOp d.42; các `_LStrCopy`/`FUN_0077ef7c` | Handler chính, danh sách case, wire field |
| 2 | `ts_decompile/functions/007ab870_FUN_007ab870.c` | @`0x007AB870`; switch tag d.253–332; gate ignore d.251 | Hàm hội tụ hiển thị + bảng tiền tố theo tag |
| 3 | `ts_decompile/functions/007ab870_FUN_007ab870.asm.txt` | `PUSH 0x7ABD54/D70/DAC/DC0/DF0/E00/E14/E40/E54/E64/E74/DA0`, `CALL 0x00404148`(LStrCatN), `CALL 0x004040d4`(LStrCat3) | Xác minh thứ tự nối chuỗi + địa chỉ hằng số |
| 4 | `ts_decompile/functions/007ad614_FUN_007ad614.c` | @`0x007AD614`; ring buffer `[str,color,id]` d.107–138; filter d.99; VMT+0x88 d.158 | Bộ đệm chat 100 dòng + màu + lọc từ |
| 5 | `ts_decompile/redump/lit_7ABD54.hex` … `lit_7ABE74.hex` | `0x7ABD54`…`0x7ABE94` | Giải mã VISCII→NFC nhãn kênh (bằng chứng nghiệp vụ) |
| 6 | `ts_decompile/functions/00722508_FUN_00722508.c` | @`0x00722508`; 2100 slot `gvar_007DA6BC` | Tra id→tên (cache) |
| 7 | `ts_decompile/functions/0075ddb8_FUN_0075ddb8.c` | @`0x0075DDB8` | Wrapper resolve tên (gọi FUN_00722508) |
| 8 | `ts_decompile/functions/005631c4_FUN_005631c4.c` | @`0x005631C4`; mảng `DAT_00949284` | Ignore-list |
| 9 | `ts_decompile/functions/0070c20c_FUN_0070c20c.c` | @`0x0070C20C`; 800 actor `gvar_007DA300` | Tra actor bản đồ (sub-op 0x02, k/quả bỏ) |
| 10 | `ts_decompile/functions/004c9bf0_FUN_004c9bf0.c` | @`0x004C9BF0` | Hàm phân lớp `class` (dùng ở gate 0x01 & 0x08) |
| 11 | `ts_decompile/functions/005965e8_FUN_005965e8.c` / `005964c8_FUN_005964c8.c` | @`0x005965E8`/`0x005964C8` | Cờ timestamp `TSe_InputBar` (sub-op 0x08) |
| 12 | `ts_decompile/functions/0051ac3c_FUN_0051ac3c.c` | @`0x0051AC3C` (`gvar_007DA37C`) | Bộ lọc tên trong FUN_007ad614 |
| 13 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | `gvar_007DA1B0=TTalkMsgForm` (d.1590), `gvar_007DA6E0=TFT_ChannelForm` (d.1592), `gvar_007DA1DC=TSe_InputBar` (d.1581) | Định danh form/global |
| 14 | `ts_decompile/functions/006037dc_FUN_006037dc.c` | d.178 `+0x168..=1` (lặp 0..5) | Cờ bật 6 kênh |
| 15 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 2: break;` d.819–820 | Xác nhận C→S OP 0x02 rỗng |
| 16 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | body inline OP 0x02 d.1007–1175 | Đối chiếu song song (bản thân dispatcher chứa inline của cùng case) |
| 17 | `ts_decompile/functions/0077ef7c_FUN_0077ef7c.c`, `0077eb9c_FUN_0077eb9c.c` | codec | DWORD LE / Word LE |
| 18 | `.scratch/op-code/handoff-opcode-exploration-guide.md`, `opcode_00_01.md` | framing, PacketBuffer, dispatcher | Kiến trúc nền & đối chiếu nghiệp vụ |

*Ghi chú độ tin cậy:* mục 0/4/5/6/7 xác minh **100% từ mã nguồn sơ cấp**.
