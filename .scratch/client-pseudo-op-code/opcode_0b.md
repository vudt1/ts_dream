# PHÂN TÍCH — Main OP 0x0B (Case 11, `FUN_0078d5d1` @ `0x0078D5D1`) — chiều Server→Client

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server→Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (case function + dispatcher inline + C→S `FUN_0077f414`). Từ 2026-09-14: **cả 12/12 hàm con đã có body** — không còn đường "pass-through không suy diễn" nào.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Tóm tắt nghiệp vụ (1 đoạn)

**OP 0x0B là kênh "đồng bộ menu/form + trạng thái nhân vật" dạng fan-out:** 1 byte SubOp chọn 1 trong 12 đường xử lý, mỗi đường **pass nguyên RestPayload cho 1 form/hàm quản lý khác nhau**, tầng case này **không tự parse Word/DWORD nào** (ngoại trừ SubOp 7 và 9 đọc 1–2 byte cờ). 3 đường đọc được body cho thấy: SubOp 0 = reset/xóa trạng thái hiệu ứng theo `(id DWORD, kind Word)`; SubOp 3 = toast 4 loại theo 1 byte mã; SubOp 5 = đồng bộ danh sách "hỗ trợ chiến đấu" theo record 23 byte; SubOp 9 = ghi 2 byte cờ vào `PlayerRec+0x1308` + toast số khi byte thứ 2 = 1. 8 đường còn lại trước đây chỉ kết luận được "forward nguyên rest"; từ 2026-09-14 cả 8 body đều đã decompile — bức tranh chung: **nhóm SubOp 1/4/5/FA thao tác trường chiến đấu toàn cục `DAT_0098c63c` (object `TFightField`)**, **nhóm 0/6/7/9/0x0A ghi cờ trạng thái actor/PlayerRec**, **nhóm 0x0B/0x0C cập nhật lịch/đếm ngược của menu (`+0x118/+0x120`, `+0x230` + toast hạn)** — chi tiết ở §3.

---

## 1. Entry & cách đọc PacketBuffer

**Entry:** `FUN_0078d5d1` @ `0x0078D5D1` (Case 11, jump-entry `0x0078A9E2` của bảng `0x78A9B6`).

**Mapping MainOp 0x0B → Case 11 (3 bằng chứng độc lập):**
1. `manifest.csv` dòng 13: `11,0x0078A9E2,0x0078D5D1,EXPORTED,FUN_0078d5d1`.
2. `redump/jumptable_byte200_0x78A8EE.hex` dòng 1: `01 02 03 04 05 06 07 08 09 0A 00 0B ...` — index byte `0x0B` = giá trị `0x0B` (MainOp 0x0B → case index 11).
3. `redump/jumptable_dword200_0x78A9B6.hex` dòng 3: entry thứ 12 (index 11) = `D1 D5 78 00` = `0x0078D5D1` (little-endian).
4. Đối chiếu inline: khối `case 0xb:` tại `0078a89c_FUN_0078a89c.c:2179–2280` khớp từng nhánh với file case riêng.

**Dispatcher (bộ điều phối S→C):** `FUN_0078a89c` (`ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`) tra bảng byte `0x78A8EE` lấy index, rồi tra bảng dword `0x78A9B6` để nhảy tới hàm xử lý (`case_001` đến `case_065`). MainOp `0x0B` → Case 11 → `FUN_0078d5d1`.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame cả 2 chiều XOR khóa tĩnh `0xAD` qua `FUN_0050a248` / `FUN_0050a2fc`. Server luôn phát tín hiệu trước (Server speaks first).

**Cách đọc SubOp (đầu file case, dòng 28–35):**
```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload (payload đã cắt MainOp)
if (*(int *)(iVar2 + -4) == 0) _BoundErr(0); // guard Delphi: RestLen >= 1
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + 0); // SubOp = ECX[0]
```
- `unaff_EBP + -0xc` = **RestPayload** = toàn bộ payload trừ byte MainOp (quy ước `_LStrCopy(payload,2,len-1)`).
- Nên `ECX[0]` (0-based C) = `payload[1]` = **SubOp**, đọc bằng **1 byte thường, không codec**.
- `*(iVar2-4)` = độ dài Delphi AnsiString (guard `_BoundErr(n)` = yêu cầu `Len > n`).

**Quy ước `_LStrCopy` 1-based (Delphi `Copy`):**
| Phép đọc | Ý nghĩa trên wire |
|---|---|
| `_LStrCopy(ECX, 1, 4, &t)` | `ECX[0..3]` = `payload[1..4]` |
| `_LStrCopy(ECX, 2, 4, &t)` + `FUN_0077ef7c` | `ECX[1..4]` = `payload[2..5]` = **DWORD LE** |
| `_LStrCopy(ECX, 6, 2, &t)` + `FUN_0077eb9c` | `ECX[5..6]` = `payload[6..7]` = **Word LE** |
| `*(byte*)(ECX+n)` trực tiếp | `payload[n+1]`, 1 byte thường |

**Codec helper (đã đọc body, xác minh 100%):**
- `FUN_0077eb9c` (`0077eb9c_FUN_0077eb9c.c:166–174`): `b0 + b1*0x100` = **Word LE**.
- `FUN_0077ef7c` (`0077ef7c_FUN_0077ef7c.c:207–240`): `b0 + b1*0x100 + b2*0x10000 + b3*0x1000000` = **DWORD LE**.
- `FUN_0077eb1c`: encode U16 → chuỗi 2 byte LE (chỉ dùng chiều C→S).
- `FUN_0077ee84`: encode U32 → chuỗi 4 byte LE (chỉ dùng chiều C→S).
- `FUN_0077f098`: `_LStrToString + copy 8 byte` — **không được handler này gọi**.

> Quan trọng: **bản thân `FUN_0078d5d1` không gọi bất kỳ hàm Word/DWORD LE nào.** Mọi decode số nằm trong các hàm con (chỉ 3 hàm có body mới thấy được).

---

## 2. Bảng tổng hợp toàn bộ SubOp

`switch(SubOp)` phân 3 tầng: `<8`, `<0x0C`, `==0x0C` / `==0xFA`. **Các case tồn tại: `0,1,3,4,5,6,7,9,10,11(0x0B),12(0x0C),250(0xFA)`. Không có `2, 8`, không có `default`.**

| SubOp | Độ dài payload tối thiểu (tầng case) | Wire | Handler (arg1 = object, arg2 = RestPayload) | Tình trạng body |
|---|---|---|---|---|
| `0x00` | **2** (`0B 00`) | pass nguyên ECX | `FUN_00641fa4(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x01` | **2** (tầng case) — callee đòi **4** | 2 byte: `[i1=RP1 (0..3)][i2=RP2 (0..4)]` | `FUN_0064bb24(*(gvar_007D9CE4), ECX)` | **CÓ body mới** (`0064bb24_FUN_0064bb24.c`) — xem §3 SubOp 1 |
| `0x03` | **2** | pass nguyên ECX | `FUN_00644b84(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x04` | **2** (tầng case) — callee đọc record 8B | `n=(Len-1)/8` record `[kind:1B][id:4B LE][w:2B LE][val:1B]` từ `RP[1]` | `FUN_00642244(*(gvar_007D9CE4), ECX)` | **CÓ body mới** (`00642244_FUN_00642244.c`) |
| `0x05` | **2** | pass nguyên ECX | `FUN_00644294(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x06` | **2** (tầng case) — callee đòi **6** | `id: DWORD LE RP[1..4]` | `FUN_0072c9e8(*(gvar_007D9D34), ECX)` | **CÓ body mới** (`0072c9e8_FUN_0072c9e8.c`) — (đính chính: file `0072c988` trước đây bị nhầm là hàm khác; body thật đã có) |
| `0x07` | **4** (`0B 07 B1 B2`) — guard `Len<3 → BoundErr(2)` + `Len<2 → BoundErr(1)` | `[0B][07][B1=RP1][B2=RP2]` | `func_0x00733650(*(gvar_007DA7BC=self), B1, B2?)` + cờ UI | **CÓ body mới** (`00733650_FUN_00733650.c`) — (đính chính: **không phải chỉ dùng B1**, xem §3) |
| `0x09` | **4** (`0B 09 A B`) — guard `Len<2` và `Len<3` | `[0B][09][A=ECX1][B=ECX2]` | inline: `PlayerRec+0x1308 := A`; nếu `B==1` thì toast số | inline toàn bộ |
| `0x0A` (10) | **2** (tầng case) — callee đòi **3** | `mode: 1B RP[1]` | `FUN_0064e978(*(gvar_007D9CE4), ECX)` | **CÓ body mới** (`0064e978_FUN_0064e978.c`) |
| `0x0B` (11) | **2** (tầng case) — callee đòi **10** | `blob8: 8B RP[1..8]` → double | `FUN_005ba604(*(gvar_007D9D00), ECX)` | **CÓ body mới** (`005ba604_FUN_005ba604.c`) |
| `0x0C` (12) | **2** (tầng case) — callee đòi **10** | `blob8: 8B RP[1..8]` → double (deadline) | `FUN_0060e2a8(*(gvar_007DA4F8), ECX)` | **CÓ body mới** (`0060e2a8_FUN_0060e2a8.c`) |
| `0xFA` (250) | **2** (tầng case) — callee đòi **4 + 24·n** | `mode=RP1, sub=RP2` + n record 24B | `FUN_006438ac(*(gvar_007D9CE4), ECX)` | **CÓ body mới** (`006438ac_FUN_006438ac.c`) |
| `0x02`, `0x08`, các giá trị khác | — | rơi qua switch, chỉ cleanup chuỗi | no-op | — |

---

## 3. Chi tiết từng SubOp

Quy ước: `P[i]` = byte payload gốc (`P[0]=0x0B`, `P[1]=SubOp`); `R[i]` = RestPayload (`R[0]=P[1]`).

### SubOp `0x00` — `FUN_00641fa4` (CÓ BODY — reset hiệu ứng theo id+kind)
- Wire (tầng case): `[0B][00][rest...]` (≥2 byte, pass nguyên). Parse thật nằm trong `00641fa4_FUN_00641fa4.c:55–58`:
  - `id = DWORD LE của Copy(R,2,4)` = `P[2..5]`; `kind = Word LE của Copy(R,6,2)` = `P[6..7]` → **thực tế cần payload ≥ 8 byte** (`0B 00 id[4] kind[2]`), nếu thiếu thì `_BoundErr` bên trong con.
- Logic core (`00641fa4.c:59–125`):
  - `kind==0` + `id==selfId (*(gvar_007DA7BC+4))` → nhánh self: gọi `FUN_00595a70(InputBar, 0)` (tắt cờ thanh nhập) + các call VMT hiển thị (1 dòng: đóng/refresh form).
  - `kind==0` + `id!=self` → tra actor `FUN_0070c20c(mapScene, id)`, set `actor+0x376=0` (xóa trạng thái), có thể gọi hiệu ứng `FUN_00666bf0` (graphics — 1 dòng).
  - `kind!=0` → tra bảng `gvar_007D9FB4[kind]` (giới hạn `kind ≤ 200`), nếu `slot+4==id` thì `slot+0x376=0`.
- Tóm 1 dòng graphics: mọi hiển thị chỉ qua VMT `+0x24` và `FUN_00666bf0/FUN_00595a70`.

### SubOp `0x01` — `FUN_0064bb24` (CÓ BODY MỚI — xóa/dọn dòng đơn vị trong trường chiến đấu `DAT_0098c63c`)
- Wire: `[0B][01][i1:P2=RP1, 1B][i2:P3=RP2, 1B]` — 2 byte thường, guard `Len<2→BoundErr(1)`, `Len<3→BoundErr(2)` (`0064bb24_FUN_0064bb24.c:50–62`); `i1` bị chặn `≤3` (BoundErr `:65–67`), `i2` bị chặn `≤4` (`:74–77`).
- **Xác minh được từ body mới: field mà handler dùng là object `TFightField` toàn cục `DAT_0098c63c`** (trước đây không biết field). Toàn bộ thao tác đi qua `DAT_0098c63c`, còn **object `gvar_007D9CE4` truyền vào (param_1) chỉ được cất vào stack, không dùng** (`0064bb24_FUN_0064bb24.c:42`).
- Logic (`0064bb24_FUN_0064bb24.c:63–181`):
  - Null-guard `if (DAT_0098c63c != 0)` (`:63`).
  - Tra dòng bảng record `[i1]` (stride `0x23*2=70` byte, base `DAT_0098c63c`), kiểm entry `+0x48 + i2*14 ≠ 0` (`:64–83`); nếu 0 → không làm gì.
  - Đọc byte `local_f = *(record+0x4C + i2*14)` (`:103`) = **mã slot actor 0..0x14** (guard `≤0x14`), tra `actor = *(DAT_0098c63c+0x158+code*4)` (`:108`).
  - Nếu `actor+0x5B4 == 0` (`:108`): gọi 5 hàm gỡ UI list `FUN_0066d450/66d804/66de54/66e37c/6bac4c(*(DAT_0098c63c + 0x200/0x254/0x2A8/0x2FC/0x350 + code*4), 0xB)` (`:113–133` — 5 mảng con stride 0x54).
  - Nhánh hiệu ứng (`:138–178`): nếu `actor+0x391 ∈ {7,8}` → `FUN_006540b0(actor)` (refresh actor); ngược lại nếu `DAT_0098c63c+0x3A4 < 0x14`: khi `actor+0x79==2 && actor+0x574≠0` → `FUN_006540b0(actor)`, còn lại `FUN_00666bf0(*(DAT_0098c63c+0x1AC+code*4), 6, code, 0)` (ẩn hiệu ứng); nếu `+0x3A4 ≥ 0x14` → `FUN_006540b0(actor)`.
- Tóm 1 dòng graphics: mọi hiển thị qua `FUN_00666bf0`/`FUN_006540b0`/5 hàm list đã kể (presentation, không bóc thêm).

### SubOp `0x03` — `FUN_00644b84` (CÓ BODY — toast 4 loại theo 1 byte mã)
- Wire (tầng case): `[0B][03][rest...]` pass nguyên. Parse thật trong `00644b84_FUN_00644b84.c:41–49`: guard `RestLen<2 → BoundErr(1)`, đọc `code = R[1]` (= `P[2]`, 1 byte thường) → **thực tế cần payload ≥ 3 byte**.
- Logic core (dòng 50–61): `code==1/2/3/4` → toast `DAT_00644c7c / 00644cac / 00644cf0 / 00644d14` qua `(gvar_007DA084+0x90)(...,1000,0,0)` (hiển thị 1000ms); các giá trị khác → không làm gì.
- Chuỗi: 4 địa chỉ `0x00644C7C...` **không có file `lit_644c*.hex` trong `redump/` → chưa dịch được, không bịa nội dung.**

### SubOp `0x04` — `FUN_00642244` (CÓ BODY MỚI — set byte trạng thái `+0x376` theo danh sách record 8 byte)
- Wire (tầng case): `[0B][04][rest...]` pass nguyên. Parse thật (`00642244_FUN_00642244.c:55–155`): `n = (RestLen-1)/8` record, mỗi record 8 byte từ `RP[1]` (Delphi 1-based, pos = `8i+2`):
  - `kind = RP[8i+1]` (byte thường, `:87`), `id = DWORD LE RP[8i+2..5]` (`:94–95` + `FUN_0077ef7c`), `w = Word LE RP[8i+6..7]` (`:102–104`), `val = RP[8i+8]` (byte, `:115`).
- Rẽ theo `kind` (cùng kiểu "tra bảng rồi ghi cờ" như SubOp 0 của `FUN_00641fa4`):
  - `kind==2` (`:116–125`): `idx=FUN_0070c20c(gvar_007D9D34, id)`; nếu actor tồn tại → `actor+0x376 := val` (1 byte).
  - `kind==6` (`:126–138`): `w ≤ 200` → nếu `gvar_007D9FB4[w] != 0` → `*(gvar_007D9FB4[w])+0x376 := val`.
  - `kind==0x10` (`:139–151`): `w ≤ 10` → bảng `gvar_007DA10C[w]` tương tự.
  - kind khác → record bị bỏ qua (vẫn tính 8 byte).
- **Xác minh được từ body mới:** guess cũ "forward nguyên rest cho form" đúng ở tầng case; tầng callee **có parse** (id/word/byte) và ghi byte `+0x376` (trạng thái hiệu ứng/cờ) — thống nhất với offset `+0x376` đã biết từ SubOp 0.

### SubOp `0x05` — `FUN_00644294` (CÓ BODY — đồng bộ danh sách hỗ trợ chiến đấu, record 23 byte)
- Wire (tầng case): `[0B][05][rest...]` pass nguyên. Parse thật trong `00644294_FUN_00644294.c:107–245`:
  - `local_d = R[1]` (= `P[2]`, 1 byte mã loại); số record `n = (RestLen-2)/0x17` (23 thập phân), loop từng record 23 byte từ offset `R[2]`:
  - Mỗi record: `local_e = +0`, `id = DWORD LE +1 (4B)`, `local_22 = Word LE +5 (2B)`, `local_18 = DWORD LE +7 (4B)`, `local_f = +11 (1B)`, `local_10 = +12 (1B)`, 4×Word LE `+13/+15/+17/+19`, `local_24/local_23 = +21/+22 (1B+1B)`.
- Logic core: `switch(local_e)` rẽ `2/3/4/6/7/9/0xB/0xC/0xD/0xE/0xF/0x10/0x12` gọi `FUN_006469b8 / FUN_00646a34 / FUN_00659358` với `(DAT_0098c63c, actor hoặc cache tên, ...)` — đều là cập nhật bảng quản lý chiến đấu + tra actor (`FUN_0070c20c`) / cache tên (`FUN_00722508`). Hiển thị/effect chỉ trong các hàm đó (ngoài phạm vi, tóm 1 dòng).

### SubOp `0x06` — `FUN_0072c9e8` (CÓ BODY MỚI — gán mã hiệu ứng `0x76` cho actor theo id)
- Wire: `[0B][06][id:P2..P5=RP1..4, DWORD LE]` (`0072c9e8_FUN_0072c9e8.c:39–40` `_LStrCopy(RP,2,4)`+`FUN_0077ef7c`). **Xác minh được từ body mới**: callee thật sự parse `id` DWORD.
- Logic (`0072c9e8_FUN_0072c9e8.c:41–48`): `idx = FUN_0070c20c(scene gvar_007D9D34, id)`; nếu `idx==0` (actor chưa có) → không làm gì; nếu `idx>800` → `_BoundErr`.
  Khi actor tồn tại: `FUN_0072a7a8(*(gvar_007DA300+idx*4), 0x76)`.
- `FUN_0072a7a8(actor, code)` (`0072a7a8_FUN_0072a7a8.c:56–66`) ghi: `actor+0x3D8 := (char)code`, `+0x3D9=0`, `+0x3DA=1`, `+0x3DB := FUN_007c9b38(gvar_007D9ED8, "…"&IntToStr(code))` (handle ảnh/biểu tượng theo mã), `+0x3DF := Now()` (double), `+0x3E7=0`. Nghĩa nghiệp vụ của mã `0x76=118` nằm ngoài tầng decompile — **chưa kết luận được** (chỉ ghi nhận: gán trạng thái-hiệu ứng có mã + mốc thời gian cho actor).

### SubOp `0x07` — `FUN_00733650` (CÓ BODY MỚI — cờ + bộ đếm trên self, toast khi cờ = 1/2)
- Wire: `[0B][07][B1=RP1][B2=RP2]`; code yêu cầu `RestLen ≥ 3`.
- **(đính chính)** Guess cũ "chỉ dùng B1" **sai**: body có **3 tham số register** `(param_1=self EAX, param_2=B1 EDX, param_3 ECX)` và `param_3` được dùng để cộng vào bộ đếm (`00733650_FUN_00733650.c:48–67`). Call-site trong case chỉ hiển thị 2 đối số theo thói quen decompiler; byte `RP[2]` chính là guard `Len<3→BoundErr(2)` (`case_011_0078D5D1_FUN_0078d5d1.c:38`) — khớp `param_3=RP[2]` theo quy ước register, nhưng Ghidra không hiện dòng nạp ECX → mức chốt: *rất có thể* (chưa 100%).
- Logic body (`00733650_FUN_00733650.c:48–92`):
  1. `self+0x1305 := B1` (1 byte, `:48`) — cờ chế độ.
  2. `self+0x1306 := min(0xFF, self[+0x1306] + param_3)` — bộ đếm byte có bão hòa tại `0xFF` (`:49–67`, `_IntOver` khi tràn).
  3. Nếu `self[+0x1305]==1` → toast `DAT_007337b4 + IntToStr(self[+0x1306]) + DAT_007337dc` qua `(gvar_007DA084+0x90)(..., 0x5DC=1500ms)` (`:68–78`); nếu `==2` → cùng cấu trúc với `DAT_007337e8` (`:79–89`). Cờ khác → không toast.
  4. Nếu `gvar_007DA51C != 0` → `FUN_00658c30(gvar_007DA51C)` (`:90–92`) (refresh UI, 1 dòng).
- Chuỗi `0x007337B4 / 7337DC / 7337E8` là constant vùng code, **không có trong `redump/` → chưa dịch được** (ghi §5).
- Phần inline tầng case giữ nguyên như mô tả cũ: cờ `+0xEB4` của `gvar_007DA51C`, reset `InputBar+0x18C` + `FUN_00595a70(InputBar,1)`. (`FUN_00595a70`, có body, `00595a70_FUN_00595a70.c:46–119`: setter cờ `param_1+0x18C := param_2`; đổi skin/button `btn_021/btn_024` qua `FUN_007c9b38+007b0628` và bật/tắt 7 nút `+0x104..+0x114` — UI thuần, tóm 1 dòng.)

### SubOp `0x09` — ghi cờ + toast số (INLINE TOÀN BỘ, không cần hàm con)
- Wire: `[0B][09][A][B]` (payload đúng 4 byte tối thiểu; `A=R[1]=P[2]`, `B=R[2]=P[3]`).
- Logic:
  - `*(gvar_007DA7BC self +0x1308) := A` (1 byte).
  - Nếu `B==0x01`: `IntToStr(A) + _LStrCatN(3 phần: UNK_00796f1c, số, UNK_00796f44)` → toast `(gvar_007DA084+0x90)(...,2000)` 2000ms. Nếu `B!=1` → chỉ ghi cờ, không toast.
- Chuỗi: `UNK_00796F1C`, `UNK_00796F44` **nằm dưới `0x796F50` — block dump mới `lit_796f50.hex` bắt đầu tại `0x796F50` nên KHÔNG phủ hai địa chỉ này → ghi rõ địa chỉ + chưa dịch được, không bịa.**

### SubOp `0x0A` — `FUN_0064e978` (CÓ BODY MỚI — đặt mode trường chiến đấu + byte nhịp theo mode)
- Wire: `[0B][0A][mode=RP1, 1B]`; callee guard `Len<2→BoundErr(1)` (`0064e978_FUN_0064e978.c:39–43`).
- Logic: `*(DAT_0098c63c + 4) := mode` (1 byte — chính là field mode của object `TFightField` mà SubOp `0xFA` tạo ra, xem §3 SubOp FA) (`:44`); rồi set byte `*(DAT_0098c63c + 0xE5A)`:
  - `mode ∈ {2,4,7,8,9}` → `0x1E (30)`; `mode == 0x0E` → `10`; còn lại → `0x14 (20)` (`0064e978_FUN_0064e978.c:46–54`).
- **Không có null-guard** trên `DAT_0098c63c` (khác SubOp 1) → gửi SubOp 0x0A ngoài trận đấu nhiều khả năng gây lỗi tham chiếu — chưa kiểm chứng runtime, đánh dấu **chưa kết luận được** hậu quả chính xác.

### SubOp `0x0B` — `FUN_005ba604` (CÓ BODY MỚI — lịch menu: giá trị + mốc `Now()+value`)
- Wire: `[0B][0B][blob8:P2..P9=RP1..8]` — `_LStrCopy(RP,2,8)` (`005ba604_FUN_005ba604.c:36`) rồi `FUN_0077f098` decode 8 byte → số double trong ST0 (`:37`).
- Ghi vào object `gvar_007D9D00` = `TLH_ApparatusMenu` (xác minh `0051189c.c:1774–1775`): `obj+0x118 := value` (double, `:38`), `obj+0x120 := Now() + value` (`:39–40`). **Xác minh được từ body mới**: callee chỉ decode 1 số 8 byte và ghi 2 field double — không chuỗi, không id.

### SubOp `0x0C` — `FUN_0060e2a8` (CÓ BODY MỚI — deadline menu + toast hạn 5000ms)
- Wire: `[0B][0C][blob8:P2..P9=RP1..8]` — cùng khuôn decode 8 byte → double (`0060e2a8_FUN_0060e2a8.c:41–43`), ghi `obj+0x230 := value` với `gvar_007DA4F8` = `TCY_VenderMenu` (`0051189c.c:1363–1364`).
- Nếu `Now() < obj+0x230` (`0060e2a8_FUN_0060e2a8.c:44–45`): `FormatDateTime("YYYY/MM/DD hh:mm", value)` (`:46`) → `_LStrCat3(&DAT_0060E39C, chuỗi ngày)` (`:47`) → toast `(gvar_007DA084+0x90)(..., 5000, 0, 0)` (`:48`). Chuỗi tiền tố `DAT_0060E39C` (vùng code, **không có trong `redump/` → chưa dịch được**); `"YYYY/MM/DD hh:mm"` là format mask ASCII, không cần giải mã.

### SubOp `0xFA` — `FUN_006438ac` (CÓ BODY MỚI — tạo lại `TFightField` + nạp danh sách đơn vị, record 24 byte)
- Wire: `[0B][FA][mode=RP1][sub=RP2][n record 24B từ RP3]`; guard `Len<2→BoundErr(1)`, `Len<3→BoundErr(2)` (`006438ac_FUN_006438ac.c:94–106`); `n = (Len-3)/0x18` (`:107–112`).
- **Phát hiện quan trọng từ body mới**: `DAT_0098c63c := TFightField_Create(VMT_63CC84_TFightField, 1, mode, sub)` (`006438ac_FUN_006438ac.c:114`) — SubOp FA **tạo object trường chiến đấu mới** và ghi đè con trỏ toàn cục mà SubOp 1/0x05/0x0A dùng. Đây là bằng chứng trực tiếp danh tính `DAT_0098c63c` = instance `TFightField`.
- Mỗi record 24 byte (pos 1-based `4+24i`): `kind=RP[3+24i]` (`:138`), `type=RP[4+24i]` (`:149`), `id=DWORD LE RP[5..8+24i]` (`:156–157`), `w100=Word LE RP[9..10+24i]` (`:164–165`, dùng ở case 3 với guard `≤100`), `dword=RP[11..14+24i]` (`:172–173`), `bA=RP[15+24i]`, `bB=RP[16+24i]` (`:184,195`), 4×Word `RP[17..24+24i]` (`:202–227`), 2 byte `RP[25..26+24i]` (`:238,249`).
- `switch(type)` (`:250–391`) — cùng họ lá với SubOp 5 (`FUN_006469b8/FUN_00646a34/FUN_00659358` nạp đơn vị vào `DAT_0098c63c`):
  - `type==2` (`:251–307`): nếu `id==myId` → cập nhật self: `PlayerRec+0x376 := kind`; `kind∈{4,7}` → `+0x1307=1`; `kind==4` → `DAT_0098c640+0x25=1` + `FUN_0079b620(gvar_007DA0B4,3)`; `gvar_007DA37C+0xC := 2`; nếu `kind∈{2,5}` và `PlayerRec+900==';'` → `FUN_0072b390(self,8)`; `bA,bB≠0xFF` → `FUN_006469b8(field, self, bA, &kind, &words, 1, 0, bB)`. Nếu id khác → resolve actor (`FUN_0070c20c`) hoặc cache tên (`FUN_00722508`) rồi `FUN_006469b8/FUN_00659358` (`:273–306`).
  - `type==3` (`:308–321`): đích là `gvar_007DA6DC[w100]` (bảng 101 slot).
  - `type==4` (`:322–325`) → `FUN_00646a34(field, id, bA, 1, &kind, &words, 1, dword, 4, bB)`; `type∈{6,0xB,0xC,0xD,0xF,0x10,0x12}` (`:326–333,384–391`) → cùng call với flag 0 và type truyền nguyên; `type==7` (`:334–337`); `type==9` (`:338–379`) → như type 2 nhưng không ghi cờ self; `type==0xE` (`:380–383`).
- Sau vòng lặp (`:396–402`): nếu `PlayerRec+0x376==4` → `*(*(DAT_0098c63c+0x158))+0x35C=0` + `gvar_007DA37C+0x1C=1`; ngược lại `FUN_0058e76c(gvar_007DA530,1)` (UI 1 dòng).
- Ý nghĩa từng `type` theo game-design (loại đơn vị nào) **chưa kết luận được** — chỉ kết luận ở mức "khởi tạo lại trường chiến đấu + nạp/xóa đơn vị".

---

## 4. Đối chiếu chiều Client→Server trong `FUN_0077F414`

`0077f414_FUN_0077F414.c:902–910` (`switch(param_2 & 0xFF)`):
```c
case 0xb:
  FUN_00402b90(...); _PStrNCat(...,2); FUN_00402b90(...); _PStrNCat(...,3);
  _LStrFromString(...); TForm1_CY_AddSedQueue(*(gvar_007DA664), ...);
  break;
```
- **Case 11 CÓ builder** (không phải `break` rỗng như OP 0x02/0x09). Nó dựng chuỗi gửi từ các `ShortString` nội bộ rồi đẩy vào hàng đợi gửi `CY_AddSedQueue`. Vì decompile không giữ tên biến nguồn nên chỉ kết luận: **client có thể chủ động gửi OP 0x0B (gói ngắn, không field nghiệp vụ rõ ràng ở tầng này)** — ngược với OP 0x02/0x09 là one-way S→C.

---

## 5. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ `opcode_02.md` mục 5 cũ ghi "cp1258 → NFC" — **đính chính 2026-09-14**: chính các dump `lit_7ABD*.hex` đó decode sạch bằng **VISCII** (`iconv -f VISCII`, cp1258 trả garbage — `opcode_09.md §7.1`). Mọi chuỗi toast của OP 0x0B (khi dump được) giải mã theo **VISCII**. (Các chuỗi `0x644Cxx/0x796F1C/44` đến từ `lit_796ec0.hex` đã decode xong — xem bảng run `opcode_09.md §7.2`: `0x796F1C`="Sử dụng tự động bổ huyết đan", `0x796F44`="lần"; riêng `0x644Cxx` vẫn chưa có window dump.)

| Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|
| `DAT_00644C7C / AC / F0 / 14` | `FUN_00644b84` (SubOp 3, toast 1000ms) | **Chưa có dump `lit_644c*.hex` → chưa dịch được** |
| `UNK_00796F1C / UNK_00796F44` | SubOp 9 inline (toast 2000ms) | **Chưa có dump (block `lit_796f50.hex` mới chỉ phủ từ `0x796F50`) → chưa dịch được** |
| `DAT_0060E39C` | SubOp 0x0C (tiền tố toast 5000ms) | Constant vùng code, **không có dump → chưa dịch được** |
| `DAT_007337B4 / DAT_007337DC / DAT_007337E8` | SubOp 7 (`FUN_00733650`, toast 1500ms dạng tiền-tố+số+hậu-tố) | Constant vùng code, **không có dump → chưa dịch được** |
| `LAB_0072A870` | tiền tố trong `FUN_0072a7a8` (SubOp 6, ghép `IntToStr(0x76)`) | Constant vùng code, **không có dump → chưa dịch được** |
| `"YYYY/MM/DD hh:mm"` | SubOp 0x0C `FormatDateTime` | Literal ASCII — không cần giải mã |

Payload trên wire của OP 0x0B **không mang text trực tiếp** (chỉ id/kind/cờ số) — text nằm ở **hằng số trong binary**. Không suy đoán nội dung tiếng Việt khi chưa có bytes (tránh bịa đặt). Cần dump Delphi ansistring tại các địa chỉ trên từ binary gốc rồi giải mã theo **VISCII** (`iconv -f VISCII` — cp1258 đã bị bác bỏ, xem `opcode_09.md §7.1`).

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `[F4 44][Len:Word LE][payload]`, `Len` = độ dài payload (không tính 4 byte header); toàn bộ bytes trên socket = plain XOR `0xAD` từng byte.
2. **Bảng frame tính sẵn:**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 0 min | `0B 00` | `F4 44 02 00 0B 00` | `59 E9 AF AD A6 AD` |
| SubOp 9 (A=5,B=1 → toast) | `0B 09 05 01` | `F4 44 04 00 0B 09 05 01` | `59 E9 A9 AD A6 A4 A8 AC` |
| SubOp 7 (B1=1) | `0B 07 01 00` | `F4 44 04 00 0B 07 01 00` | `59 E9 A9 AD A6 AA AC AD` |

3. **Độ dài an toàn:** SubOp 7/9 bắt buộc đủ 4 byte payload (thiếu → `_BoundErr` crash client ở tầng case). Các SubOp pass-through chỉ cần 2 byte ở tầng case, nhưng từ 2026-09-14 mọi callee đều đã có body và đều đòi dài hơn: SubOp 0 ≥8 byte (`id`+`kind`), SubOp 1 đúng 4 byte (2 byte chỉ số `≤3`/`≤4`), SubOp 3 ≥3 byte, SubOp 4 `2+8n` byte, SubOp 5 `2+23*n` byte, SubOp 6/0x0A/0x0B/0x0C cần tương ứng ≥6/≥3/≥10/≥10 byte, SubOp FA `4+24n` byte (mode/sub = RP[1..2], record từ RP[3..26]). Gửi thiếu → `_BoundErr`/truy vấn mảng lỗi trong callee, không phải no-op.
4. **Thứ tự test gợi ý:** `0B 09 05 00` (chỉ ghi cờ, an toàn nhất) → `0B 09 05 01` (toast số) → `0B 03 <code 1..4>` (toast 1000ms) → `0B 00 + id/kind` → `0B 04/06/0x0A` với id/mode nhỏ → chỉ chạy `0B 01` trong trận đấu (cần `DAT_0098c63c` khác 0), và `0B FA` (tạo mới `TFightField` — làm mất trạng thái trận hiện có, kiểm soát có chủ đích).
5. **Không gửi SubOp `0x02`/`0x08`** — không có nhánh, rơi qua switch (vô hại nhưng vô nghĩa).

---

## 7. Source trail (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_011_0078D5D1_FUN_0078d5d1.c` | toàn file 150 dòng | Handler chính, danh sách SubOp, guard độ dài |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0xb:` d.2179–2280 | Đối chiếu inline 1:1 với file case |
| 3 | `ts_decompile/case_functions/manifest.csv` | dòng 13 | Case 11, entry `0x0078A9E2` → `0x0078D5D1` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | Case 11 | Xác nhận target |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | dòng 1 | `MainOp 0x0B → case 11` |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | dòng 3 | `case 11 → 0x0078D5D1` |
| 7 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | switch d.768; `case 0xb` d.902–910 | Chiều C→S có builder |
| 8 | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c`, `0077ef7c_FUN_0077ef7c.c` | codec | Word LE / DWORD LE |
| 9 | `ts_decompile/functions/00641fa4_FUN_00641fa4.c` | SubOp 0: `id`+`kind`, core reset | Body SubOp 0 |
| 10 | `ts_decompile/functions/00644b84_FUN_00644b84.c` | SubOp 3: 1 byte mã → 4 toast | Body SubOp 3 |
| 11 | `ts_decompile/functions/00644294_FUN_00644294.c` | SubOp 5: record 23B | Body SubOp 5 |
| 12 | `ts_decompile/functions/00595a70_FUN_00595a70.c` | SubOp 0/7: setter cờ `+0x18C` | Cờ UI |
| 13 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | d.1363–1364, 1774–1775 | `TCY_VenderMenu`, `TLH_ApparatusMenu` |
| 14 | `ts_decompile/functions/0064bb24 / 00642244 / 0064e978 / 0072c9e8 / 00733650 / 005ba604 / 0060e2a8 / 006438ac` (`*_FUN_*.c`) | 8 body mới (2026-09-14) | SubOp 1/4/0x0A/6/7/0x0B/0x0C/FA — hết cảnh "không có body" |
| 15 | `ts_decompile/functions/0072a7a8_FUN_0072a7a8.c` | d.56–66 | Lá ghi hiệu ứng mã `0x76` (SubOp 6) |
| 16 | Giới hạn còn lại: `lit_644c*.hex`, `UNK_00796F1C/44` (dưới `0x796F50`), các constant code `DAT_0060E39C`, `DAT_007337xx`, `LAB_0072A870` | — | Chưa dump, chưa dịch |

*Ghi chú trung thực (cập nhật 2026-09-14): cả 12 SubOp đã có đủ đường tới body/inline; kết luận ghi trường/offset đều trích trực tiếp từ code trên. Phần còn mờ: ý nghĩa nghiệp vụ các mã `type` của SubOp FA/5, mã hiệu ứng `0x76`, hậu quả ghi `DAT_0098c63c` khi ngoài trận — đánh dấu "chưa kết luận được" chứ không bịa. Chuỗi toast `0x644Cxx/0x796F1C/44` và các constant vùng code vẫn chờ dump (decode VISCII — cp1258 đã bác bỏ, `opcode_09.md §7.1`).*
