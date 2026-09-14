# PHÂN TÍCH — Main OP 0x0F (Case 15, `FUN_0078e367` @ `0x0078E367`) — **Đồng bộ NPC / Cart / Buff-effect (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Kết luận quan trọng nhất

> **Main OP 0x0F CÓ SubOp. Handler là một `switch` 20 nhánh trên 1 byte `payload[1] = ECX[0]`.**

Khác OP 0x0C (không SubOp, struct cố định), OP này giống họ OP 0x02/0x0D: 1 byte SubOp chọn đường xử lý, mỗi đường forward nguyên RestPayload cho một hàm con kèm một object form. **Cập nhật 2026-09-14: cả 20/20 đường đã có body** (15 handler vừa được decompile). Kết quả verify lớn nhất: các handler 0x01/0x02/0x04/0x06/0x07/0x08/0x11/0x12/0x14/0x15 **không hoạt động trên `TFNpcManage`** (param_1 của 0x01–0x07 chỉ được cất rồi bỏ, trừ 0x08/0x14/0x15 dùng làm arg refresh `FUN_007a5d54/7a5e1c`); chúng thao tác **mảng 5 con trỏ slot `PlayerRec(self)+0x57C + slot*4`** (guard `slot ≤ 4`, các field `+0x55F…+0x568, +0x3EA…+0x40A` của sub-object) — cùng hệ "slot" đã gặp ở SubOp 0x0A (parser cart). (đính chính: suy đoán cũ "mỗi đường → một form NPC/cart khác nhau" chỉ đúng với 0x0B/0x0C (cart), 0x0D/0x0E/0x0F (buff-actor), 0x10/0x11 (scene), 0x12 (self), 0x13 (SystemForm); nhóm 0x01/02/04/06/07/08/14/15 là **đồng bộ state 5 slot của self qua id**.)
Nghiệp vụ (suy trực tiếp từ object + body đọc được): **đồng bộ NPC (TFNpcManage) + xe/cart (TWF_CartManage) + trạng thái buff/effect trên actor + vài form hệ thống**. Không phải chat (chat là OP 0x02), không mang text tiếng Việt nào trên wire (ngoài 2 hằng chuỗi vùng code — xem §6).

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
| `gvar_007DA2FC` | Manager của SubOp 0x0D–0x0F (vùng item/buff) | **Vẫn chưa tìm thấy dòng `Create` → tên class chưa xác định.** Cập nhật 2026-09-14: SubOp 0x0D (`FUN_0052a898`) **bỏ qua arg1** trong body — chỉ dùng RP + scene global |

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: `1, 2, 4, 5, 6, 7, 8, 9, 10(0xa), 11(0xb), 12(0xc), 13(0xd), 14(0xe), 15(0xf), 16(0x10), 17(0x11), 18(0x12), 19(0x13), 20(0x14), 21(0x15)`. **Không có `case 0`, không có `case 3`, không có `default`** → SubOp 0 / 3 / ≥22 rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x0F`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu (sau body mới) | Wire field (bên trong callee) | Hàm con / object | Body? |
|---|---|---|---|---|
| `0x01` | **12** | `id:RP1..4 DWORD` + `mã:RP5 1B` + `a:RP6..9 DWORD` + `b:RP10 1B` | `FUN_007a3ed0(TFNpcManage→bỏ qua, RP)` | **CÓ (mới)** |
| `0x02` | **6** | `id:RP1..4 DWORD` + `slot:RP5 1B` | `FUN_007a40e8(TFNpcManage→bỏ qua, RP)` | **CÓ (mới)** |
| `0x04` | **2**, sau đó chuỗi record `[id:4][n:1][n×(12+len)]` | parser lồng (bỏ `7·n` byte nếu actor absent — xem §4.8) | `FUN_007a4694(TFNpcManage→bỏ qua, RP)` | **CÓ (mới)** |
| `0x05` | **14** (`0F 05` + 12B) | 3×DWORD LE | `FUN_007a49c0(TFNpcManage, RP)` | **CÓ** |
| `0x06` | **6** | `id:RP1..4 DWORD` | `FUN_007a42ec(TFNpcManage→bỏ qua, RP)` | **CÓ (mới)** |
| `0x07` | **6**, sau đó chuỗi record `12+len` byte từ `RP[5]` | id cha + vòng entry slot | `FUN_007a43c8(TFNpcManage→bỏ qua, RP)` | **CÓ (mới)** |
| `0x08` | **31+** — chuỗi record ~`99+len` byte | blob state slot đầy đủ (header 29B + name + 60B blocks) | `FUN_007a5010(TFNpcManage, RP)` | **CÓ (mới)** |
| `0x09` | **7** (`0F 09` + 4B + 1B) | id DWORD + flag 1B + msg tới hết | `FUN_007a4b08(TFNpcManage, RP)` | **CÓ** |
| `0x0A` | **9** (`0F 0A` + 8B) | vòng lặp ≤4 entry `[idx:1B][w:2B][b:1B][w:2B][len:1B][str]` | `FUN_00751d28(TWF_CartManage, RP)` | **CÓ** |
| `0x0B` | **3** (`0F 0B` + 1B) | 1 byte thường (guard `Len<2→BoundErr(1)`) | `FUN_00751c7c(TWF_CartManage, byte)` | **CÓ (mới)** |
| `0x0C` | **3** (`0F 0C` + 1B) | `mode:RP1 1B` | `FUN_00746fc8(self, RP)` | **CÓ (mới)** |
| `0x0D` | **2**, chuỗi record `[id:4][w:2]` tới hết | per-id buff apply | `FUN_0052a898(mgr_2FC→bỏ qua, RP)` | **CÓ (mới)** |
| `0x0E` | **8** (`0F 0E` + 4B + 2B) | id DWORD + w Word LE | `FUN_0052a7b4(mgr_2FC, RP)` | **CÓ** |
| `0x0F` | **5** (`0F 0F` + 2B + 1B) | w Word LE + b byte thường | `FUN_0052aacc(mgr_2FC, RP)` | **CÓ** |
| `0x10` | **11** | `id:RP1..4` + `slot:RP5` + `w1:RP6..7` + `w2:RP8..9` (LE) | `FUN_0074dfd4(scene, RP)` | **CÓ (mới)** |
| `0x11` | **7** | `kind:RP1 (1..5)` + `id:RP2..5 DWORD` | `FUN_0074e400(scene, RP)` | **CÓ (mới)** |
| `0x12` | **5** | `slot:RP1 (1..4)` + `b1:RP2` + `b2:RP3` | `FUN_0074edd0(self, RP)` | **CÓ (mới)** |
| `0x13` | **3** (branch 1 cần 4) | `mã:RP1`; nếu `==1`: `char:RP2` | `FUN_005a71bc(SystemForm, RP)` | **CÓ (mới)** |
| `0x14` | **2**, chuỗi `[slot:1][b1:1][b2:1]` × `(Len-1)/3` | batch của 0x15 | `FUN_007a5b74(TFNpcManage, RP)` | **CÓ (mới)** |
| `0x15` | **5** | `slot:RP1 (≤4)` + `b1:RP2` + `b2:RP3` | `FUN_007a4e94(TFNpcManage, RP)` | **CÓ (mới)** |

---

## 4. Chi tiết từng SubOp (toàn bộ 20 đường đã có body — 4.1–4.6 cũ + 4.7 mới 2026-09-14)

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

### 4.4. SubOp `0x0B` — `FUN_00751c7c` (**CÓ BODY MỚI** — clear 1 WORD slot trên self)
- Wire: `[0F][0B][b:P2, 1B]` — guard `Len<2→BoundErr(1)` ngay tầng case, `func_0x00751c7c(cartObj, b)`.
- Body (`00751c7c_FUN_00751c7c.c:22–31`): guard `b ≤ 4` (`_BoundErr` nếu lớn hơn); rồi `*(Word*)(*(gvar_007DA7BC) + 0xD6F + b*16) := 0` — **xóa 2 byte tại `PlayerRec+0xD6F + slot·16`** (mảng 5 slot stride 16). Arg1 `TWF_CartManage` không dùng. Ý nghĩa field `+0xD6F` chưa kết luận được (liên quan hệ slot 5-entry như `+0x57C`).

### 4.5. SubOp `0x0E` — apply buff/effect lên actor
- Wire: `[0F][0E][id:P2..P5 DWORD LE][w:P6..P7 Word LE]`.
- Core: `idx=FUN_0070c20c(scene,id)`; nếu actor tồn tại → `FUN_0074b7c8(actorObj, w)`.
- `FUN_0074b7c8(obj, w)`: tra cứu buff theo mã word qua bảng; nếu trúng thì điền struct buff tại actor (tên chuỗi, timer, các word chỉ số, cờ), refresh UI (và thêm refresh nếu là self). Tóm 1 dòng: **áp buff/effect có tra bảng lên actor**. `w` không trúng bảng → không làm gì.

### 4.6. SubOp `0x0F` — set/clear buff state của self
- Wire: `[0F][0F][w:P2..P3 Word LE][b:P4, 1B thường]` — guard RP len<4 → `_BoundErr(3)`.
- Core:
  - `w==0` → `FUN_0074bd70(self)`: duyệt 4 slot, gỡ hiển thị slot, xóa struct buff — tóm 1 dòng: **xóa buff self**.
  - `w!=0` → `FUN_0074b7c8(self,w)` (apply buff như 4.5) + ghi byte `b` + cờ + refresh.

### 4.7. Các SubOp trước đây "không body" — **NAY ĐÃ CÓ BODY (2026-09-14)**, phân tích từng handler

Quy ước chung rút ra từ 15 body mới: các handler nhóm slot resolve actor theo `id` (self nếu `id==myId`, ngược lại `FUN_0070c20c(scene)` — actor không tồn tại thì **bỏ qua gói**), và ghi vào **sub-object slot `*(PlayerRec+0x57C + slot*4)`** (slot guard `≤4`). Lá `FUN_0071c3a0(obj, slot, dword, flag)` **vẫn không có body** — chỉ biết chữ ký.

- **0x01 — `FUN_007a3ed0`** (`007a3ed0_FUN_007a3ed0.c:52–112`): `id=DWORD RP[1..4]`; guard `Len<6→BoundErr(5)` → `mã=RP[5]` (byte, guard `≤0xFF`); `a=DWORD RP[6..9]`; guard `Len<0xB→BoundErr(10)` → `b=RP[10]`. Ghi: `FUN_0071c3a0(obj, mã, a, 1)` (`:90`); global `*(gvar_007DA694+4) := mã` (`:95` — byte lưu "slot đang xử lý", guard `≤4`); `*(subobj(mã)+0x55F) := b` (`:108`); nếu obj là self → `FUN_0063afa4(gvar_007DA0CC)` + `FUN_005a3018(gvar_007D9D7C)` refresh (`:109–112`).
- **0x02 — `FUN_007a40e8`** (`007a40e8_FUN_007a40e8.c:51–102`): `id=DWORD RP[1..4]`; guard `Len<6` → `slot=RP[5]` (guard `≤4`). `FUN_0071c8dc(obj, slot)` (`:80` — lá 189 dòng: xóa slot, có phát `sound\WA0014.wav` `0071c8dc.c:175`). Nếu obj==self: đọc tên shortstring `subobj(slot)+9` (`:88–89`); refresh `FUN_0063afa4` + `FUN_005a3018`; clear `self+0x151C` nếu trùng slot (`:93–95`); nếu tên ≠ rỗng và `self+0x376 ≠ 0` → ghi dòng chat-log `tên + DAT_007A42B0` qua `FUN_007ab870(gvar_007DA1B0, myId, msg, tag=0)` (`:96–101`). → "gỡ/dismount slot + thông báo chat".
- **0x04 — `FUN_007a4694`** (`007a4694_FUN_007a4694.c:78–221`): parser lồng: ngoài `pos=2`: `id=DWORD` (`:82–83`), `n=RP[pos+4]` byte (`:94`), `pos+=5`; nếu actor **không** tồn tại → `pos += n*7` (`:102–109` — **mâu thuẫn với độ dài entry 12+len của nhánh có actor, chưa kết luận được**); nếu có: lặp `n` entry `[slot:1][a:4][b:4][flag:1][x:1][len:1][name:len]` (`:118–194`): `FUN_0071c3a0(obj,slot,a,0)` (`:196`); `flag==1` → `FUN_0071dae0(obj,a,b)` (`:197–199`); `subobj(slot)+0x55F := x` (`:204`); `len≠0` → `_PStrNCpy(subobj+9, name, 0x11)` (`:213`).
- **0x06 — `FUN_007a42ec`** (`007a42ec_FUN_007a42ec.c:40–58`): `id=DWORD RP[1..4]`; resolve obj (self/scene); `FUN_0071d058(obj)` (`:57`) — lá có body (`0071d058_FUN_0071d058.c:26–40`): nếu `obj[0x360]!=0` → clear `+0x360`, clear `subobj+0x554`, 2 call VMT `+0x24` (gvar_007DA39C/007DA3BC), `TObject_Free(obj+0x37C)` + set 0, VMT `+0x18(obj,0)`. → "đóng/hủy phương tiện slot của actor".
- **0x07 — `FUN_007a43c8`** (`007a43c8_FUN_007a43c8.c:67–192`): `id=DWORD RP[1..4]`; **chỉ chạy khi actor đã tồn tại** (`FUN_0070c20c != 0`, `:69–70`); vòng entry từ `pos=6` (RP[5]): `[slot:1][a:4][b:4][flag:1][x:1][len:1][name]` (12+len byte, `:81–141`); ghi y hệt 0x04 (`FUN_0071c3a0` flag 0 `:156`, `FUN_0071dae0` khi flag==1 `:162`, `subobj+0x55F := x` `:174`, name → `subobj+9` `:190`).
- **0x08 — `FUN_007a5010`** (835 dòng, blob state slot): vòng record từ `pos=2` (`007a5010_FUN_007a5010.c:142–145`), mỗi record dài `99+len` byte (`:817`): header 29B: `slot=RP[0]`, `w=Word` → `FUN_0071c3a0(self, slot, w, 0)` + `FUN_005752a8(gvar_007DA688, w)` + `FUN_007a5e1c(mgr, slot)` (`:160–165`); `DWORD→subobj+0x3FC` (`:181`); byte→`+0x3FA` (`:198`); 8×Word→`+0x3EA..+0x3F8` (`:214–326`); 3 byte→`+0x55D/55E/55F` (`:343–377`); Word→`+0x400` (`:393`); byte `len` + name→`subobj+9` (`:415–423`); 4 byte→`+0x560..0x563` (`:463`); 6 block 10 byte → `FUN_0072fa98(subobj, w, ...)` mỗi block (`:470–690`); byte→`+0x565` (`:715`); char→byte `+0x566` qua `FUN_0077eaa4` (`:752`); Word→`+0x568` (`:791`); cuối record `FUN_007a5d54(mgr, slot)` (`:821`). **Toàn bộ field này là bản "full state" của cùng sub-object slot mà 0x01/0x02/0x04/0x07 ghi từng phần** — tên field nghiệp vụ chưa kết luận được.
- **0x0C — `FUN_00746fc8`** (`00746fc8_FUN_00746fc8.c:26–133`): `mode=RP[1]` byte → `self+0x1358`; đảm bảo 2 object `THuman` (`VMT_70B1C4_TPlayers→THuman_Create`) tại `self+0x1350/+0x1354` (`:31–38`); chọn animation id theo `self+0x4B0/0x4B1` (0x7536/0x753D..0x7540, và 0x4655) qua VMT `+0x1c` (`:39–57`); ghi 2 float `+0x340/+0x344` (`:59–63`). `mode==1` → `self+0x134D=1` + `FUN_0053b3cc`; `mode==2` → copy tọa độ `self+0x1C/0x20/0x54/0x58 ±400/-50` vào 2 THuman + `FUN_0070dc78` (`:69–122`); `mode==3` → `TObject_Free` cả 2 + clear (`:124–132`). → **dựng/hủy "người đẩy xe" hiển thị khi mount/dismount cart** (suy từ offset cart `+0x4B0/+0x4B1` đã biết ở OP 0x0C §4).
- **0x0D — `FUN_0052a898`** (`0052a898_FUN_0052a898.c:45–79`): chuỗi record `[id:4 DWORD][w:2 Word]` từ `RP[1]`; actor không tồn tại → skip 2 byte; có → `FUN_0074b7c8(actor, w)` — **nhiều bản buff của SubOp 0x0E** (lá buff đã biết ở §4.5).
- **0x10 — `FUN_0074dfd4`** (`0074dfd4_FUN_0074dfd4.c:55–117`): `id=DWORD RP[1..4]`; guard `Len<6→BoundErr(5)` → `slot=RP[5]` (guard `≤4`); `w1=Word RP[6..7]`; `w2=Word RP[8..9]`; resolve obj (self/scene); nếu `subobj(slot)+4 ≠ 0` (slot đang dùng): đọc tên `subobj+9`, ghi `subobj+0x568 := w2` (`:99`), VMT `+0x1c` (`:104`), copy lại tên (`:112`); nếu `obj+0x37C` (cửa sổ slot đang mở) có `+0xD == slot` → `FUN_007209fc(obj+0x37C, w1, w2)` (`:113–116`). → **cập nhật state/giá của slot đang dùng**.
- **0x11 — `FUN_0074e400`** (`0074e400_FUN_0074e400.c:45–98`): `kind=RP[1]` (chỉ nhận 1..5, `:51–52`); `id=DWORD RP[2..5]` (`Copy(RP,3,4)` `:54`); resolve obj; mỗi kind chọn cặp animation id từ {0x4668..0x4673} (`:74–95`) rồi `FUN_0074e20c(obj, &danh_sách_4_id, 3, 4, obj+0x57C, ...)` (`:97`) — **phát bộ animation lên mảng slot** (lá không bóc).
- **0x12 — `FUN_0074edd0`** (`0074edd0_FUN_0074edd0.c:35–68`): `slot=RP[1]`; điều kiện chạy `slot-1 < 4` theo **unsigned** ⇒ `slot ∈ 1..4` — `slot==0` hoặc `≥5` → **dừng im lặng, không BoundErr** (`:43`); guard `Len<3→BoundErr(2)`, `Len<4→BoundErr(3)`; ghi `subobj(slot)+0x56A := RP[2]`, `subobj(slot)+0x56B := RP[3]` (2 byte; chỉ số dùng nguyên `slot`, guard `slot≤4` thừa). Payload tối thiểu 5.
- **0x13 — `FUN_005a71bc`** (`005a71bc_FUN_005a71bc.c:45–71`): `mã=RP[1]` (guard `Len<2`); `==1`: guard `Len<3`, `FUN_0077eaa4(char RP[2])` → byte 0/1 (`0077eaa4.c:51-52`: `char=='\x01'`) → `form+0x180 := kết quả` + `FUN_005a7584(form)` — đổi ảnh nút **"btn_ExpressOn"/"btn_ExpressOff"** (`005a7584_FUN_005a7584.c:21–29`) → **bật/tắt chế độ "tốc hành/express" của SystemForm**; `==2`: ghi dòng chat-log hằng `DAT_005A7280` (tag 0) (`:70`).
- **0x14 — `FUN_007a5b74`** (`007a5b74_FUN_007a5b74.c:37–118`): batch của 0x15: `n=(Len-1)/3` record `[slot:1][b1:1][b2:1]`; mỗi record: `subobj(slot)+0x5CA := b1`, `+0x5C9 := b2`, `+0x408 (Short) := b1*50`, `+0x40A (Short) := b2*10`, rồi `FUN_007a5d54(mgr, slot)` (`:116`).
- **0x15 — `FUN_007a4e94`** (`007a4e94_FUN_007a4e94.c:37–93`): một record `[slot:1][b1:1][b2:1]` (3 guard `Len<2/3/4` → payload ≥5; slot guard `≤4`); ghi y hệt 0x14 (`+0x5CA`, `+0x5C9`, `+0x408 = b1*50`, `+0x40A = b2*10`) + `FUN_007a5d54(mgr, slot)` (`:93`).
- Lá `FUN_007a5d54` / `FUN_007a5e1c` (refresh mgr theo slot) **không có file body** — giữ mức call-site.

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

- Handler tầng case **không tham chiếu bất kỳ hằng chuỗi nào** (chỉ `gvar_*` + địa chỉ hàm).
- 15 body mới (2026-09-14) xác nhận: **không có chuỗi nào trên wire**; các handler gọi `FUN_0077ef7c/eb9c` decode số. Chỉ tìm thấy **3 hằng chuỗi vùng code** trong callee — tất cả **không có trong `redump/` → chưa dịch được, không bịa**:
  - `DAT_007A42B0` — hậu tố dòng chat-log của SubOp 0x02 (`007a40e8_FUN_007a40e8.c:98`).
  - `DAT_005A7280` — dòng chat-log của SubOp 0x13 nhánh `mã==2` (`005a71bc_FUN_005a71bc.c:70`).
  - `"btn_ExpressOn" / "btn_ExpressOff"` — tên asset nút trong `FUN_005a7584` (SubOp 0x13, ASCII thuần, không cần giải mã).
- Tiền lệ mã hóa: **VISCII đơn-byte tiền tổ hợp** (đính chính 2026-09-14 — bản cũ ghi cp1258 theo `opcode_02.md` mục 5; cp1258 đã bị bác bỏ bằng chứng byte ở `opcode_09.md §7.1`), dùng khi dump literal các chuỗi còn treo.

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

3. **Thứ tự test an toàn (cập nhật 2026-09-14):** `0F 0B 01` (clear WORD slot 1, hàm đã rõ) → `0F 0F 00 00 00` (clear buff self) → `0F 09 …` với id lạ (dừng ở `FUN_0070c20c==0`) → `0F 05/0E/0D` với id tồn tại → `0F 01/02/06/10/11/12/14/15` (giờ đã biết chính xác field: dùng id tồn tại + slot 0..4) → `0F 04/07/0A/08` (parser vòng lặp, dễ BoundErr nếu record cụt). Chú ý: `0C/12/13/14/15` không chứa id — ghi thẳng lên self/form; còn các gói theo id khi actor chưa tồn tại đa số **rơi im lặng** (bỏ qua).
4. **Không gửi SubOp `0x00 / 0x03 / ≥0x16`** — rơi qua switch (vô hại nhưng vô nghĩa). Payload ngắn hơn mức tối thiểu trong bảng §3 → `_BoundErr` trong callee, coi như gói lỗi. Lưu ý 0x12: `slot=0` hoặc `≥5` **không lỗi, chỉ im lặng bỏ qua**.
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
| 14 | `007a3ed0 / 007a40e8 / 007a4694 / 007a42ec / 007a43c8 / 007a5010 / 00751c7c / 00746fc8 / 0052a898 / 0074dfd4 / 0074e400 / 0074edd0 / 005a71bc / 007a5b74 / 007a4e94` (`*_FUN_*.c`) | **15 body mới 2026-09-14** | verify từng SubOp §3/§4.7 — field offsets, guard, lá |
| 15 | `0071d058 / 0071c8dc / 0072a7a8(qua 0b) / 007209fc(không body) / 0074e20c / 0072fa98 / 005a7584 / 0077eaa4 / 0077ed68` | lá được gọi từ body mới | phân loại "presentation / setter slot" |

**Giới hạn (không suy diễn) — cập nhật 2026-09-14:**
- ~~15/20 hàm con không có body~~ → **cả 20/20 đường đã có body**. Khoảng trống còn lại chỉ là LÁ: `FUN_0071c3a0`, `FUN_007a5d54`, `FUN_007a5e1c`, `FUN_0074e20c`, `FUN_007209fc` (không file `.c`) — các field slot `+0x55F..+0x568`, `+0x3EA..+0x40A` ghi từ body mới có tên/ý nghĩa **chưa kết luận được**, chỉ mô tả offset+size đúng code.
- Mâu thuẫn ghi nhận nguyên trạng ở SubOp 0x04: nhánh "actor không tồn tại" skip `7·n` byte trong khi nhánh kia tiêu thụ `12+len`/entry → **chưa kết luận được** wire thật sự hay bug.
- Tên class của `gvar_007DA2FC` chưa tìm thấy dòng `Create` trực tiếp (body 0x0D lại bỏ qua arg1 — vai trò vẫn mở).
- Ý nghĩa game-design chi tiết (buff nào, slot dùng ở đâu, chuỗi `007A42B0/005A7280`) nằm ngoài tầng case; các hằng chuỗi chưa dump.
