# PHÂN TÍCH — Main OP 0x29 (41) / Case 37 / `FUN_007943F9` @ `0x007943F9`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). File case 159 dòng, `switch` 22 nhánh, không default. **Cập nhật: cả 13/13 callee passthrough đã có body — 22/22 nhánh đều phục hồi được.**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh **fan-out điều khiển actor/NPC + flag player** — kiểu OP 0x24 (exec/passthrough + vài nhánh đọc field thật), không phải kênh số cộng/trừ. **Đính chính (2026-09-14)**: hai callee `0x06/0x09` gọi chat-log `FUN_007ab870` ở tầng hàm con (`00635e20_FUN_00635e20.c:114`, `006367cc_FUN_006367cc.c:211`) — nhận định 'không chat-log' chỉ đúng ở tầng dispatcher.
- 3 họ:
  - **Passthrough blob (12 nhánh)**: `0x01,02,04,05,06,07,09,0x0B,0x0D,0x15,0x17,0x33` — trao nguyên `RP` cho hàm con (manager chính `*gvar_007D9D34` họ `0073/0074xxxx`, `*gvar_007D9C70` họ `0063xxxx`, riêng `0x17` dùng `*gvar_007DA188`). **Cả 12 body đã phục hồi (2026-09-14)** — thực chất KHÔNG phải blob mù: mỗi hàm tự cắt field tường minh (xem §4.7).
  - **Decode tường minh (8 nhánh có body)**: `0x03` (loop list DWORD → `FUN_0071cdec`), `0x08` (clear 201 entry, không đọc wire), `0x0A` (Word → banner số), `0x14` (batch NPC 10B/record), `0x16` (free slot), `0x34` (ghi row 6B) + 3 nhánh ghi byte `0x0C/0x0E/0x32`.
  - **Exec tĩnh**: `0x0F` (`func_0x00637760`) — body mới xác nhận: không đọc wire, ghép chuỗi từ 5 record trong `*gvar_007D9C70` rồi banner 15000ms (`00637760_FUN_00637760.c:43-88`).
- Phát hiện chính từ body mới: bảng slot 201 entry `gvar_007D9FB4` chứa object **`TGarNpc`** (VMT `0x70B3D0`, tạo trong `FUN_007369e8`) — họ SubOp `0x01/0x02/0x07/0x33` vận hành trên bảng này; `*gvar_007D9C70` là manager có **mảng 5 record stride 0x40** mà `0x04/0x05/0x06/0x0F` đọc/ghi.
- Text duy nhất ở tầng dispatcher: cặp hằng `UNK_00798E60/00798EA4` (SubOp 0x0A, banner 2000ms) — vẫn chưa có dump. Trong 13 body mới có **nhiều literal ghim trong `.text`** (`DAT_00635a74/636b30/6378a0/6361e8…`) — xem §5.
- Chiều C→S `case 0x29: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x29 (41) → byte_table[0x78A8EE][0x29] = 0x25 (37)
                 → dword_table[0x78A9B6][37] @ 0x0078AA4A = 0x007943F9
                 → FUN_007943f9 (Case 37)
```

- File chính: `ts_decompile/case_functions/functions/case_037_007943F9_FUN_007943f9.c` (159 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6139-6255` — khớp 1:1 (lộ `2000` ở 0x0A, `GetTickCount` ở 0x32).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x29`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 29-36)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){ case 1..0xF, 0x14..0x17, 0x32..0x34 ... }
// Không default → 0x00, 0x10–0x13, 0x18–0x31, ≥0x35 no-op (chỉ epilogue dòng 131-156)
```

### 2.4. Codec dùng trong OP

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077eb9c` | 2B → Word LE — trực tiếp ở 0x0A (`_LStrCopy(RP,2,2)`); gián tiếp trong `FUN_007450f4` (x/y) |
| `FUN_0077ef7c` | 4B → DWORD LE — gián tiếp trong `FUN_007366c8` (list ID), `FUN_007450f4` (NPC id) |
| `FUN_0077f098` | 8B → double — **không gọi** (không nhánh nào dùng số thực) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều này |
| `IntToStr` | Word → chuỗi (0x0A) |
| Banner `(VMT+0x90)(...,2000,0,0)` | Qua `*gvar_007DA084` (0x0A) |
| `GetTickCount` | 0x32 → timestamp `player+0x1430` |

---

## 3. Bảng tổng hợp SubOp (22 nhánh)

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[29][01][rec]*`, rec = `[slot:1B][id:4B][w:2B][dw:4B][x:4B][y:4B][n:1B][name:nB]` | variable-length record loop | `FUN_007369e8`: tạo/cấp nhật `TGarNpc` vào `gvar_007D9FB4[slot]` + refresh mgr (§4.7) |
| `0x02` | `[29][02][id:4B][slot:1B]*` (5B/record) | loop `(len-1)/5` | `FUN_007367f4`: gắn actor(id) vào entry `gvar_007D9FB4[slot]` (§4.7) |
| `0x03` | `[29][03][id:4B]*` (`2+4k` B) | loop `_LStrCopy(RP,pos,4)`+`EF7C` | Từng id → lookup object → `FUN_0071cdec(obj)`; null → break |
| `0x04` | `[29][04][dw1:4B][cnt:1B][dw2:4B]*` (≥7B) | `EF7C` ×2 + byte đếm | `FUN_00635be4`: clear+ghi `mgr+4/+8`, rồi recheck 11..200 entry `TGarNpc` (§4.7) |
| `0x05` | `[29][05][rec]*` (biến dài, ≥~47B/record) | loop record, slot byte 1–5 | `FUN_00637070`: ghi mảng 5 record stride 0x40 của mgr9C70 (§4.7) |
| `0x06` | `[29][06][v:1B]` (3B) | `v=RP[1]` | `FUN_00635e20`: switch 1–0x10 → 11 banner 1200ms + 4 dòng chat log số (§4.7) |
| `0x07` | `[29][07][slot:1B][flag:1B]*` (2B/record) | loop `(len-1)/2` | `FUN_0073b560`: flag=1 → refresh entry; ngược lại free + null `gvar_007D9FB4[slot]` (§4.7) |
| `0x08` | `[29][08]` (2B) | không đọc | `FUN_007358b0(mgr9D34)`: clear 201 entry `007D9FB4` + `*(mgr+100)=0` |
| `0x09` | `[29][09][v:1B][...]` (≥3B) | `v=RP[1]` switch 1–0xD | `FUN_006367cc`: ghép chuỗi hằng + tên → chat log `FUN_007ab870` (§4.7) |
| `0x0A` | `[29][0A][w:2B LE]` (4B) | `_LStrCopy(RP,2,2)`+`EB9C`→`IntToStr` | Banner `hằng + số + hằng` (`98E60/98EA4`) 2000ms |
| `0x0B` | `[29][0B][kind:1B][n:1B][name:nB][4×DWORD][2×Word]...` (≥n+23B) | chuỗi + số | `FUN_006355c8`: banner 15000ms thông tin dạng bảng (chỉ chạy khi state ≠ 0) (§4.7) |
| `0x0C` | `[29][0C][v:1B]` (3B) | `v=RP[1]` (len≥2) | `*(player+0x1435)=v` |
| `0x0D` | `[29][0D][id:4B][x:4B][y:4B][flag:1B]` (15B) | `EF7C`×3 + byte `P[14]` | `FUN_0073d618`: move actor(id) tới (x,y), cascade +0x550-slot của party, flag → `player+0x1435` (§4.7) |
| `0x0E` | `[29][0E]` (2B) | không đọc | `*(player+0x1436)=0` |
| `0x0F` | `[29][0F]` (2B) | không đọc | `FUN_00637760`: ghép 5 record mgr9C70 → banner 15000ms (exec tĩnh, xác minh được từ body mới) |
| `0x14` | `[29][14][slot][id:4B][x:2B][y:2B][dir]*` (10B/record) | `slot=RP[pos]`, id=`EF7C`, x/y=`EB9C`, dir byte | Batch sinh/cập nhật `TForeignNpc` 11 slot (`007DA10C`) + refresh |
| `0x15` | `[29][15][slot:1B][w1:2B][w2:2B]` (6B) | `slot=RP[1]` (0–10), Word×2 | `FUN_00745520`: `FUN_007443c4(007DA10C[slot], w1, w2)` — di chuyển/sửa NPC (§4.7) |
| `0x16` | `[29][16][slot:1B]` (3B) | `slot=RP[1]` (len≥2, `>10` → BoundErr) | Free + null `007DA10C[slot]` nếu khác null |
| `0x17` | `[29][17][key:1B][val:1B]*` (2B/record) | loop `(len-2)/2`, key 1–5 | `FUN_005bd580`: `obj+0x144+key = val` trên `*gvar_007DA188` (§4.7) |
| `0x32` | `[29][32][v:1B]` (3B) | `v=RP[1]` (len≥2) | `*(player+0x142C)=v` + `*(player+0x1430)=GetTickCount()` |
| `0x33` | `[29][33][id:4B]*` (4B/record) | loop `(len-1)/4` | `FUN_006378d0`: actor lookup → `obj+0x5f4=1`; player → `FUN_0074337c` (§4.7) |
| `0x34` | `[29][34][b:1B][6B]` (8B) | `b=RP[1]` (`1..5`), 6 byte `P[3..8]` | Ghi 6 byte vào row `b` + `FUN_0055b4c0` refresh |

---

## 4. Chi tiết các nhánh decode được (bỏ graphics/sound/animation)

### 4.1. SubOp `0x03` — Kích hoạt hàng loạt theo list DWORD

```c
n = (len(RP)-1)/4; pos = 2;
for each: id = EF7C(Copy(RP,pos,4)); pos += 4;
  obj = (*(player+4)==id) ? player : table007DA300[lookup0070c20c(mgr,id)] (guard >800);
  if (obj==NULL) break; FUN_0071cdec(obj);
```
Wire `2+4k` B. Thiếu byte record cuối → `BoundErr` trong codec.

### 4.2. SubOp `0x0A` — Banner số Word duy nhất

- `_LStrCopy(RP,2,2)` → `w = EB9C(...) & 0xFFFF` → `IntToStr(w)` → `_LStrCatN(3)` với `{UNK_00798E60, số, UNK_00798EA4}` (thứ tự varargs chưa chắc, chỉ chắc tập 2 hằng + số) → banner `*gvar_007DA084` 2000ms. Thiếu 2 byte → `BoundErr`.

### 4.3. SubOp `0x14` — Batch NPC 10B/record (phức tạp nhất)

- `n = (len(RP)-1)/10`; mỗi record: `slot=RP[pos]` (0–10, `>10` → BoundErr), `id=EF7C`, `x/y=EB9C`, `dir` byte → sinh mới (hoặc free + sinh lại) `TForeignNpc` vào `007DA10C[slot]` (`+0x1C=x, +0x20=y, +0x376=dir, +0x4C/50/54/58` từ x/y, `FUN_0074442c`), xong `FUN_00745610(mgr)`.
- Record theo `P`: `P[2+10k]=slot`, `P[3..6]=id`, `P[7..8]=x`, `P[9..10]=y`, `P[11]=dir`. Tối thiểu 12B cho 1 record.

### 4.4. SubOp `0x16` + `0x34`

- `16`: `slot=RP[1]` → free + null `007DA10C[slot]` nếu khác null. Wire 3B.
- `34`: `b=RP[1]` (`1..5`) + 6 byte `P[3..8]` ghi vào `param_1 + b*64 + 5 + k` (k=1..6) + refresh `FUN_0055b4c0`. Wire 8B.

### 4.5. Nhánh ghi byte / clear (`0x08/0x0C/0x0E/0x32`)

- `08`: loop `0..200` free + null `007D9FB4[i]`, `*(mgr+100)=0` — reset manager.
- `0C`: `*(player+0x1435)=v`. `0E`: `*(player+0x1436)=0`. `32`: `*(player+0x142C)=v` + tick `+0x1430`.

### 4.6. Passthrough + exec (`01/02/04–07/09/0B/0D/15/17/33`, `0F`)

- **Đính chính (2026-09-14)**: nhận định cũ "17/22 handler vắng mặt, mock tối thiểu 2B, blob cần sample live" đã lỗi thời — **13/13 callee passthrough giờ đều có body** và tự cắt field tường minh (§4.7). Không còn nhánh nào là blob mù; wire thật phải theo đúng record layout bên dưới, ngắn/cụt hơn sẽ `BoundErr`.

### 4.7. Phân tích 13 body callee mới (cập nhật 2026-09-14)

**Nhóm `gvar_007D9FB4` (mảng 201 slot object `TGarNpc`, VMT `0x70B3D0`) — param_1 = `*gvar_007D9D34`:**

- **`0x01` → `FUN_007369e8`** (`007369e8_FUN_007369e8.c:78-380`): loop record biến dài tại pos=2 (1-based). Record = `[slot:1B][id:4B EF7C][w:2B EB9C][dw:4B ED68][x:4B][y:4B][n:1B][name:nB]` (20+n B). `slot` guard `0..200` (`:92`). Nếu slot trống → `TGarNpc.Create(VMT_70B3D0_TGarNpc,1,kíchthước)` (`:117`); nếu có → `TObject.Free` rồi tạo lại (`:130-163`). Kích thước = `slot+800+100+4 + **(gvar_007D9C28+0x101b8)` (`:96-116`). Ghi fields: `+0x3ea=w`, `+0x550=dw`, `+0x1c=x`, `+0x20=y`, `+0x554=string name` (`:191-255`); mirror `+0x1c→+0x4c/+0x54`, `+0x20→+0x50/+0x58` (`:269-308`); clear `+0x560/+0x559` (`:313-318`); `FUN_0071a774(obj)` (`:323`); trạng thái từ `FUN_00635258(*gvar_007D9C70, obj+0x550)`: `==2` → `+0x55b=2` và nếu `+0x558 ∈ {2..5,7}` → `+0x9b=9,+0x9c=1,+0x9d=0`, ngược lại `+0x55b=1` (`:328-365`); `FUN_00634f3c(*gvar_007D9C70, word player+0x63a, 3)==5` → `+0xe3=0xD` (`:366-373`); refresh `FUN_0071a8e0(obj)` (`:379`); cuối hàm `FUN_0072a0a0(mgr)` (`:382`). Trước đó gọi virtual `(obj VMT+0x1c)()` với id (`:171-177`).
- **`0x02` → `FUN_007367f4`** (`007367f4_FUN_007367f4.c:61-150`): `n=(len-1)/5` record `[id:4B][idx:1B]`. Actor = player nếu `*(player+4)==id` (nhân bản từ `case 0x03`) hoặc `gvar_007DA300[FUN_0070c20c(mgr9D34,id)]` (guard 800, `:96-110`); null → break. `idx` guard `≤200` (`:113`); nếu entry `gvar_007D9FB4[idx]` tồn tại, `entry+0x78≠0` và `entry+0x559==0` → `entry+0x548 = actor+4` (id actor), `entry+0x559=1`, rồi `FUN_0071d818(actor, idx)` (`:123-145`).
- **`0x07` → `FUN_0073b560`** (`0073b560_FUN_0073b560.c:29-88`): `n=(len-1)/2` record `[slot:1B guard≤200][flag:1B]`. flag==1 → `FUN_0071a8e0(entry)` nếu entry khác null (`:69-74`); flag≠1 → `TObject.Free(entry)` + gán null (`:81-85`).
- **`0x33` → `FUN_006378d0`** (`006378d0_FUN_006378d0.c:45-85`): `n=(len-1)/4` record `[id:4B]`. Nếu id là player → `FUN_0074337c()` (`:66-68`), ngược lại lookup actor (guard 800); nếu tìm thấy → `*(obj+0x5f4)=1` (`:81`).

**Nhóm manager `*gvar_007D9C70` (họ 0063xxxx — có mảng 5 record stride 0x40 tại các offset −0x34..+0x2 quanh con trỏ self):**

- **`0x04` → `FUN_00635be4`** (`00635be4_FUN_00635be4.c:47-134`): `_FillChar(mgr+4,8,0)` clear 2 DWORD `+4/+8` (`:47`); `mgr+4 = EF7C(RP[1..4] = P[2..5])` (`:48-53`); `count=RP[5]` (`:60`), loop `Copy(RP,pos,4)`→ED68 ghi `mgr+4+i*4` — guard decompile `if (i!=1) BoundErr` nên **chỉ iter đầu ghi vào `mgr+8`; các iter sau hành xử lỗi → chưa kết luận được** ý đồ (`:65-82`). Sau đó quét slot `11..200`: entry khác null → cùng logic trạng thái `FUN_00635258` như `0x01` (`+0x55b`, `+0x9b/9c/9d`) (`:84-134`).
- **`0x05` → `FUN_00637070`** (`00637070_FUN_00637070.c:127-684`): loop record biến dài ≥47B, stride advance = 45+n+n2. Record: `[slot 1..5][b1][b2][n][name nB][8×DWORD qua EF7C/ED68][n2][chuỗi 2 ≤? ][DWORD]`. Ghi vào `mgr + off + slot*64`: byte `−0x34`, `−0x33`; string `−0x32`; DWORD `−0x1a,−0x16,−0x12,−0xe,−0xa,−6,−2,+2`; DWORD ED68 `−0x2e`, độ dài chuỗi 2 tại byte `−0x26`… **và 2 field dẫn xuất**: `−0x2a = FUN_00759050(*gvar_007D9C20, [−0x2e])`, `−0x1e = FUN_00759050(*gvar_007D9C20, [−0x22])` (`:626-662`). Ngữ nghĩa nghiệp vụ từng field **chưa kết luận được** (không có literal `.text` để đối chiếu).
- **`0x06` → `FUN_00635e20`** (`00635e20_FUN_00635e20.c:51-174`): `v=RP[1]` (guard len≥2). v∈{1..10, 0x10} → banner `*gvar_007DA084` duration `0x4b0`=1200ms với 11 literal `.text` (`DAT_006361e8…006367ac`, chưa dump). v∈{0xB..0xE}: đọc byte `player+0x1437` (guard 1..5) → DWORD `mgr−0x32+idx*64` → `Format(literal)` → chat log `FUN_007ab870(*gvar_007DA1B0,0,…)` (`:99-166`). v=0xF: chat log literal `DAT_00636750` (`:167-170`).
- **`0x09` → `FUN_006367cc`** (`006367cc_FUN_006367cc.c:55-211`): `v=RP[1]` switch 1..0xD — dựng 1 chuỗi từ các literal `.text` (`0x636b30…0x636fd8`, chưa dump); các case 3/4/5/9 đọc thêm byte `RP[2]` (guard 1..5) để index bảng chuỗi `DAT_0098be04 + v*0xAB*2`; case 0xC copy `Copy(RP,3,len)` (toàn bộ `RP[2..]`), case 0.D copy `Copy(RP,3,len-2)` (`RP[2..len-3]`) nối vào (`:188-207`). Cuối: chat log `FUN_007ab870(*gvar_007DA1B0,0,s)` (`:211`).
- **`0x0B` → `FUN_006355c8`** (`006355c8_FUN_006355c8.c:77-252`): gate — `state = FUN_00634f3c(mgr, word `player+0x63a`, 2)`; **state==0 → không làm gì** (`:77-79`). state 1..5 (guard `:81`) index bảng chuỗi `DAT_0098be04`. Đọc `kind=RP[1]` (1..3 chọn literal tiền tố `0x635a9c/635abc/635adc`), `n=RP[2]`, name `Copy(RP,4,n)`; rồi 4×DWORD tại `RP[n+3..n+18]` (ED68) mỗi cái đi qua `FUN_006354c8(...,4/3/1/2/5)`, 2×Word tại `RP[n+19..22]` → `IntToStr` (`:127-250`). Banner 15000ms (`:251-252`).
- **`0x0F` → `FUN_00637760`** (`00637760_FUN_00637760.c:42-88`): không đọc wire. Loop idx 1..5: nối literal `DAT_006378a0/6378ac/6378b8`; nếu DWORD `mgr−0x2e+idx*64 == 0` → dùng `DAT_006378b8`, ngược đó chèn chuỗi con trỏ tại `mgr−0x32+idx*64`; banner 15000ms (`:87-88`). **Xác minh được từ body mới**: đúng là exec tĩnh trên mảng record của `007D9C70` (cùng field nhóm 0x05/0x06).

**Nhóm khác:**

- **`0x0D` → `FUN_0073d618`** (`0073d618_FUN_0073d618.c:50-132`): `id=EF7C(RP[1..4]=P[2..5])`, `x=RP[5..8]=P[6..9]`, `y=RP[9..12]=P[10..13]`, `flag=RP[13]=P[14]` (guard len≥14). Lookup actor (player hoặc bảng 800). Nếu thấy: `FUN_0071fae0(obj,x,y)` + `FUN_0071e2b8(obj,x,y)` (move + walk/anim); nếu actor là player → `FUN_00778510(*gvar_007D9C28,x,y)` (camera) + `player+0x1435=flag` (`:80-84`). Cascade party: byte số thành viên tại `obj+0x578`, mảng slot tại `obj+0x550+i*8` (id) / `obj+0x554+i*8`: thành viên là player → move player + flag; ngược lại move actor bảng 800 (`:85-131`).
- **`0x15` → `FUN_00745520`** (`00745520_FUN_00745520.c:46-69`): `slot=RP[1]` guard `0..10`; nếu `gvar_007DA10C[slot]≠0` → 2 Word `EB9C(Copy(RP,3,2))` và `EB9C(Copy(RP,5,2))` → `FUN_007443c4(npc, w1, w2)` — cùng bảng NPC 11 slot với SubOp 0x14/0x16 (mục đích chính xác của `FUN_007443c4` chưa đọc body → chưa kết luận được move hay resize).
- **`0x17` → `FUN_005bd580`** (`005bd580_FUN_005bd580.c:31-72`): receiver là **object** `*gvar_007DA188` (md cũ ghi `*gvar_007DA188` — xác minh được từ body mới, param_1 dùng trực tiếp làm base, không deref thêm). n = (len−2+1)/2 record `[key:1B][val:1B]`; guard `key−1 ≤ 4` → ghi `obj + 0x144 + key = val` (5 byte flags `+0x145..+0x149`).

**Kết luận sửa đổi**: "passthrough blob" chỉ còn là cách gọi dispatch; **13/13 nhánh đều parse số/string tường minh**. Riêng mảng chuỗi `DAT_0098be04` và các literal `.text` của họ 0063xxxx **chưa dump** → chưa định danh được chủ đề nghiệp vụ của nhóm 0x04–0x0F (nghi "bảng ghi chiến/quảng", chưa kết luận được).

---

## 5. Chuỗi VISCII → UTF-8

- Wire **có mang text raw** từ body mới: SubOp `0x01` (tên NPC `n` byte tại cuối record, ghi `obj+0x554` — `007369e8_FUN_007369e8.c:255`), `0x0B` (name `Copy(RP,4,n)` — `006355c8_FUN_006355c8.c:136`), `0x05` (2 chuỗi trong record stride 0x40) và đuôi `0x09` case 0xC/0xD (`Copy(RP,3,...)`). Đây là tên động từ server, không phải literal.
- Tầng dispatcher vẫn chỉ có 2 literal `UNK_00798E60/00798EA4` (0x0A); `redump/` vẫn không có `lit_798e*.hex`.
- **Giới hạn mới phát hiện**: hàng loạt literal của các callee 0063xxxx nằm **ghim trong `.text`** (`0x635a74–0x635b48`, `0x6361e8–0x6367ac`, `0x636b30–0x636fd8`, `0x6378a0–0x6378b8`) + bảng chuỗi `DAT_0098be04` (stride 0xAB×2) — quy ước `lit_*.hex` hiện tại không phủ vùng `.text` → chưa decode được. Cần redump bổ sung các địa chỉ này.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:976-977`: `case 0x29: break;` — rỗng (kẹp giữa `0x28` và `0x2a`).
- Kết luận: OP 0x29 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[29][01][slot][id u32][w u16][dw u32][x u32][y u32][n][name n]  record 20+n B
[29][02][id u32][slot]  bội 5; [29][07][slot][flag] bội 2; [29][33][id u32]* bội 4
[29][04][dw u32][cnt][dw u32]*   ≥7B   [29][06][v]/[09][v][..]  ≥3B
[29][0B][kind][n][name][4×u32][2×u16] ≥n+23B (state!=0 mới hiện banner)
[29][0D][id][x][y][flag]         15B   [29][15][slot][w1 u16][w2 u16] 6B
[29][17][key 1–5][val]*          bội 2 [29][05][rec≥47B]*  biến dài (layout §4.7)
[29][03][id u32]*              2+4k B, null → dừng
[29][08]                       clear 201 entry
[29][0A][w u16]                4B, banner 2000ms
[29][0C][v] / [29][32][v]      3B, ghi flag + (32 thêm tick)
[29][0E] / [29][0F]            2B
[29][14][slot][id][x][y][dir]  10B/record, slot 0–10
[29][16][slot]                 3B, free slot
[29][34][b 01–05][6B]          8B, ghi row + refresh
ĐỪNG GỬI: [29][00],[10–13],[18–31],≥0x35 (no-op); L=1 (RangeError).
```

Độ dài chính xác (cập nhật từ §4.7): exec/flag 2B; 1-byte param 3B; `0x0A` 4B; `0x34` 8B; `0x03` bội 4 sau 2B; `0x14` bội 10 sau 2B; `0x02` bội 5; `0x07` bội 2; `0x33` bội 4; `0x17` bội 2; `0x0D` đúng 15B; `0x15` 6B; `0x04` ≥7B (đệ nhị 4B/record sau byte đếm); `0x01/0x05/0x0B` record biến dài có prefix độ dài. Record cuối thiếu block → `BoundErr`.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_037_007943F9_FUN_007943f9.c` (159 dòng) | Handler chính đủ 22 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6139-6255` | Bản inline (`2000`, `GetTickCount`, thứ tự CatN) |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x29]=0x25`) + `.csv:37` + `manifest.csv:39` | Mapping |
| 4 | `functions/0077eb9c` (trực tiếp) / `0077ef7c` (gián tiếp) / `0077f098` (không gọi) / `0077eb1c+0077ee84` (builder) | Codec |
| 5 | `functions/007366c8/007358b0/007450f4/007456cc/006379f8` | 5 nhánh có body |
| 6 | `functions/0077f414_FUN_0077F414.c:976-977` | C→S rỗng |
| 7 | 13 body mới: `functions/007369e8/007367f4/00635be4/00637070/00635e20/0073b560/006367cc/006355c8/0073d618/00637760/00745520/005bd580/006378d0` (.c) | Phân tích §4.7 — 22/22 nhánh giờ có body |
| 8 | ls `redump/` (vẫn vắng `lit_798e*`; literals `.text` họ 0063xxxx + bảng `0x0098BE04` chưa dump) | Giới hạn VISCII còn lại |
