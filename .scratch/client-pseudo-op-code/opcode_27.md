# PHÂN TÍCH — Main OP 0x27 (39) / Case 35 / `FUN_007938C3` @ `0x007938C3`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `switch` 43 nhánh lớn nhất cụm. **12 handler passthrough nay đã có body trong bản dump mới** (`index.csv:6342,6492-6502` — xem §8): SubOp `01,02,03,04,07,08,09,0B,0C,0E,0F,32` đã bóc được wire/cơ chế (§4.8). Các nhánh passthrough còn mù: `0A, 0D, 11–13, 15–22, 36, 37`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP lớn nhất cụm (364 dòng, **43 nhánh** `0x01–0x22, 0x32,0x33,0x35–0x3B`; khuyết `0x00, 0x23–0x31, 0x34, ≥0x3C`, không default). 3 họ:
  - **Họ A — Passthrough** (01–04, 07, 09–13, 15–22, 32, 36, 37 + 33/39/3A): `func_0x0075xxxx(manager = *gvar_007D9C20, RP)`. **Cập nhật từ body mới (§4.8)**: 12 nhánh đã bóc được — 01 = công tắc 7 mã banner/6000ms + nhánh 0 kết nối; 02 = **toàn bộ bảng xếp hạng** (header + N dòng 22+n + 2 double); 03 = gắn id vào `mgr+4` + sự kiện 5000ms trên actor; 04 = banner "tên + hậu tố 1..4"; 07 = từ chối/rời (6 mã banner 1200ms); 08 = **thêm dòng bảng `007DA6E8` từ cache tên**; 09 = set xâu+DWORD `+0x478/+0x5F0` theo id; 0B = đổ chuỗi vào danh sách toàn cục `DAT_0094928C` (**manager `007DA2DC` bị callee bỏ rơi — đính chính**); 0C = chuyển dòng của id vào vùng slot 2..4 của bảng + compaction; 0E = **đổi chỗ dòng id lên hạng 1** với mode 0/2/3; 32 = bản gọn của 33 (`player+0x136A=mode`, `+0x1358+mode*0x13=dword`). Riêng SubOp `0x02` có thêm logic tầng handler (refresh form + chat-log date nếu time!=0) — **xác minh**: chính body `00752c54` set `mgr+0x28` double mà tầng handler so `!= 0.0`; `0x33/0x39/0x3A` đã có body (ghi timer/mode/double vào `player+0x1358–0x1410`).
  - **Họ B — Decode tường minh** (05, 06, 08, 0C, 0D, 0E, 10): `id` DWORD LE / `name` 8B → double / `flag` byte / text đuôi / double thời gian. SubOp `0x05` phức tạp nhất (ghi bảng 1000 entry stride 0xA5 + sound `m004.wav`); `0x06` xóa ID (banner + clear form nếu `id==player+4`); `0x10` đồng bộ double time + chat-log date.
  - **Họ C — Banner/exec tĩnh** (14, 1D, 35, 38, 3B): banner 1200ms (`0x14`) / 9 banner 2000ms theo K (`0x1D`) / clear flag + exec (`35/38/3B`).
- Wire không mang chuỗi tự do (trừ đuôi text SubOp 05 `P[15..]` cần sample live); text hiển thị là hằng `.rodata` (`0x798AC4–0x798E08`, `0x752B84/98`, `0x760854`) — chưa có dump nên chưa dịch VISCII.
- Chiều C→S `case 0x27: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x27 (39) → byte_table[0x78A8EE][0x27] = 0x23 (35)
                 → dword_table[0x78A9B6][35] = 0x007938C3
                 → FUN_007938c3 (Case 35)
```

- File chính: `ts_decompile/case_functions/functions/case_035_007938C3_FUN_007938c3.c` (364 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5771-6122` — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x27`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 37-44)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){...43 nhánh...}
```
Epilogue `_LStrArrayClr/_LStrClr` cuối hàm (dòng 333-361) là dọn chung dispatcher.

### 2.4. Codec & API

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | 4B → DWORD LE (05, 06, 08, 0C, 0D, 0E, 33) |
| `FUN_0077ed68` | Clone `ef7c` (33/39/3A, ctx `gvar_007D9D30`) |
| `FUN_0077eb9c` | 2B → Word LE (33, 3A) |
| `FUN_0077f098` | 8B → double qua FPU (05, 10); so với `0.0` (`_DAT_007988ac`) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Encode C→S, không gọi ở chiều này |
| `FUN_007ab870` | Nạp dòng vào chat-log `TTalkMsgForm` (`*gvar_007DA1B0`) |
| Banner `(VMT+0x90)` | Qua `*gvar_007DA084`, ms = 2000 hoặc `0x4B0`=1200 |
| `FUN_007a7f20` | Phát `.wav` (`m004.wav` SubOp 05...) — bỏ qua chi tiết |

---

## 3. Bảng tổng hợp SubOp (43 nhánh)

| SubOp | Wire (payload) | Core logic |
| :---: | :--- | :--- |
| `0x01` | `[27][01][code:1B]` (3B; code∈{0..5,0xFF}) | `func_0x007536fc` (§4.8.1): 6 banner 6000ms + code 0 = "nhận lời" (gate `player+0x474`, bảng hạng) |
| `0x02` | `[27][02][hdrLen][hdr][page][count][N×(22+n)][listLen][list][D1 8B][D2 8B][flag]` | `func_0x00752c54` (§4.8.2): **nạp nguyên bảng xếp hạng** `007DA6E8` (double D1 → `mgr+0x28`); tầng handler: refresh + `FUN_00765db0` + chat-log date khi `mgr+0x28≠0` |
| `0x03` | `[27][03][id:4B LE]` (6B) | `func_0x00754540` (§4.8.3): `mgr+4=id`; actor có trong 800 → sự kiện 5000ms `FUN_0063C8FC` (text `0x75462C`, closure form) |
| `0x04` | `[27][04][id:4B][sub:1B]` (7B) | `func_0x00754094` (§4.8.4): banner **tên actor + 1 trong 4 hậu tố** sub∈1..4, 1200ms |
| `0x05` | `[27][05][id:4B][name:8B][flag:1B][text...]` (≥15B) | Ghi bảng 1000 entry stride 0xA5 + sound `m004.wav` (xem 4.2) |
| `0x06` | `[27][06][id:4B]` (6B) | Xóa ID: banner `name` + dồn mảng + clear form nếu `id==player+4` (xem 4.3) |
| `0x07` | `[27][07][code:1B]`, code 0 thêm `[id:4B]` (3/7B) | `func_0x00753abc` (§4.8.5): 6 banner 1200ms; code 0 = chấp nhận id — nếu self: clear `player+0x474`, đóng 4 form |
| `0x08` | `[27][08][id:4B]` (6B) | `func_0x00752104(chatmgr,id)` (§4.8.6): **thêm dòng** bảng hạng `007DA6E8` từ cache tên `007DA6BC` + refresh `007DA358` |
| `0x09` | `[27][09]([id:4B][D:4B][n:1B][text:n])×N` | `func_0x00754360` (§4.8.7): gán `+0x478=text`, `+0x5F0=D` cho self/actor + 2 refresh |
| `0x0A` | `[blob...]` | `func_0x00753df0` — chưa body |
| `0x0B` | `[27][0B][str...]` (≥3B) | `func_0x0056d3e8` (§4.8.8): **tham số manager bị bỏ**; đổ `P[2..end]` vào `TStrings` toàn cục `DAT_0094928C` qua `FUN_00563E80` |
| `0x0C` | `[27][0C][id:4B]` (6B) | `func_0x00754d44` (§4.8.9): chuyển dòng của id vào vùng hạng 2..4 + dồn bảng + `count--` |
| `0x0D` | `[id:4B]` | `func_0x00754f80(chatmgr,id)` — chưa body |
| `0x0E` | `[27][0E][b:1B][id:4B]` (7B) | `func_0x00754724(chatmgr, id, b)` (§4.8.10): **đẩy dòng id lên hạng 1**, mode b∈{0,2,3} khác nhau ở cách giữ/dòng cũ |
| `0x0F` | `[27][0F][K:1B]` (3B) | `func_0x007551f4` (§4.8.11): **26 banner 1200ms** theo K∈{1..0x1B, 0x64, 0x65, 0xFF}; K=0x12 đặc biệt (form `007D9D6C`) |
| `0x11`–`0x13` | `[blob...]` | `func_0x00755c5c/00755d74/00755f14` — chưa body (0x10 thuộc họ B, xem dòng `0x10` dưới) |
| `0x14` | `[27][14]` (2B) | Banner `UNK_00798b50` 1200ms |
| `0x15`–`0x22` | `[blob...]` | Họ `func_0x00756xxx–00759xxx` (+ refresh `007DA0B0` ở 17/1A/1B) |
| `0x1D` | `[27][1D][K:1B]` (3B, K=1–9) | 9 banner tĩnh `00798b74..00798e08` 2000ms |
| `0x10` | `[27][10][D:8B]` (10B) | `*(chatmgr+0x28)=D` + refresh; D==0 → chat-log tĩnh, D!=0 → date log |
| `0x32` | `[27][32][mode:1B][dword:4B]` (7B) | `func_0x007564d0` (§4.8.12): bản gọn của 0x33 — `player+0x136A=mode`, `+0x1358+mode*0x13=dword`; **mode=0 → BoundErr (crash)**; mode≥6 ghi lệch ngoài cụm field (bug client) |
| `0x36`/`0x37` | `[blob...]` | `func_0x007571a4/00756fc0` — chưa body |
| `0x33` | `[mode:1B][dword:4B][word:2B]` (≥9B) | Ghi `player+0x1358+mode*0x13 / +0x1360 / +0x136A`, tick `+0x1408`, timer `+0x140C` theo mode |
| `0x39` | `[dword:4B]` (6B) | `player+0x13F2 = DWORD` |
| `0x3A` | `[dword:4B][b:1B][word:2B]` (9B) | Như 39 + `+0x13FA=b`, tick, `+0x140C=3600000`, `+0x1410=word`, `+0x13F1=1` |
| `0x35`/`0x38`/`0x3B` | 2B, không param | Clear `+0x136A/+0x13CA/+0x13F1` + `FUN_0073a22c` + exec(8) |

Khuyết (no-op): `0x00, 0x23–0x31, 0x34, ≥0x3C`.

---

## 4. Chi tiết các nhánh decode được (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x02` — Nhánh duy nhất có logic tầng handler ngoài call

```c
func_0x00752c54(chatmgr, RP); FUN_00765db0(*gvar_007DA70C);
... refresh form 007D9F1C/007DA0B0 ...; FUN_00596a7c(chatmgr2,1); FUN_00566118(...);
if (*(double*)(mgr+0x28) != 0.0) { FormatDateTime(UNK_00798ac4) + CatN(3) + FUN_007ab870(chatlog,0,tmp,...); }
```
Wire của 0x02 **được callee cắt** — cấu trúc đầy đủ xem §4.8.2 (header + N dòng 22+n + list + 2 double + flag); `mgr+0x28` mà tầng handler test chính là double D1 do body `00752c54` ghi (`00752c54_FUN_00752c54.c:783-785`).

### 4.2. SubOp `0x05` — Ghi bảng 1000 entry (phức tạp nhất)

- `id = DWORD(P[2..5])`, `name = 8B(P[6..13]) → double`, `flag = P[14]` (guard `len≥14`), `text = P[15..]` (đuôi AnsiString).
- `FUN_007605ec(mgr70C, id, text, flag, double)`: ghi record stride `0xA5` (id→`+0xE8`, name→`+0xEC` len `0x97`, double→`+0x185/0x189`, flag→`+0x184`, count `+0x28618`++), refresh `FUN_0075f1a4`; kèm sound `m004.wav` + `FUN_005952cc`.
- Full-table (`count 0x3E9=1001`) → banner `DAT_00760854` thay vì ghi.

### 4.3. SubOp `0x06` — Xóa ID

- `id = DWORD(P[2..5])` → `FUN_0075284c`: tìm trong bảng `007DA6E8` (stride `0x27*2`), banner `DAT_00752b98+name+DAT_00752b84` 2000ms, xóa + dồn mảng, giảm count. Nếu `id == *(player+4)`: clear thêm 5 form + refresh `007D9F1C/007DA0B0`.

### 4.4. SubOp `0x08/0C/0D/0E` — Theo ID

- `08/0C/0D`: `id = DWORD(P[2..5])` → `func_0x00752104/00754d44/00754f80(chatmgr,id)` (08 refresh thêm 2 form).
- `0E`: `b = P[2]` + `id = DWORD(P[3..6])` (7B) → `func_0x00754724(chatmgr, id, b)`.

### 4.5. SubOp `0x10` — Đồng bộ double time

- `D = double(P[2..9])`; `*(chatmgr+0x28)=D` + refresh form; `D==0` → chat-log `UNK_00798aec`; `D!=0` → `FormatDateTime(UNK_00798ac4)` + chat-log date.

### 4.6. SubOp `0x33/0x39/0x3A` — Timer/mode/double (có body)

- `33`: `P[2]=mode → +0x136A`; `P[3..6]` DWORD → `+0x1358+mode*0x13` (mode 1..5); `P[7..8]` Word → `+0x1360`; tick `+0x1408`; `+0x140C = 600000/300000/3600000` + `FUN_0072b390(player,0x22/0x2A)` theo mode.
- `39`: `P[2..5]` DWORD → `+0x13F2`. `3A`: thêm `P[6]→+0x13FA`, tick, `+0x140C=3600000`, `P[7..8]` Word→`+0x1410`, `+0x13F1=1`.

### 4.7. Họ C — Banner/exec tĩnh

- `14`: banner `00798b50` 1200ms. `1D`: `K=P[2]` (1..9) → 9 banner `00798b74..00798e08` 2000ms. `35/38/3B`: clear `+0x136A/+0x13CA/+0x13F1` + `FUN_0073a22c` + exec(8).

### 4.8. Mười hai handler passthrough — bóc từ body mới (2026-09-14)

Quy ước offset: `RP[k]` 0-based (`RP[0]=SubOp`, `RP[1]=P[2]`). Semua codec là `FUN_0077ef7c/ed68` (DWORD LE) và `eb9c` (Word LE), ctx `gvar_007D9D30`. `mgr = *gvar_007D9C20` (trừ 0B).

#### 4.8.1. SubOp `0x01` — `0x007536FC` (`007536fc_FUN_007536fc.c`, guard `len(RP)>=2`)

- `code=RP[1]` (`:28`). `code∈{1,2,3,4,5,0xFF}` → 6 banner tĩnh **6000ms** `DAT_007539A8/9D8/A10/A38/A60/A98` (`:45-59`).
- `code==0` (`:33-43`): `player+0x474 := *(player+4)` (self id — "đích đang hoạt động"), `mgr+0xC := self`, **`gvar_007DA6D0 := 4`** (counter bảng hạng — see 4.8.2/6), exec form `007D9F14`, `FUN_0058EA00(*gvar_007DA530,1)`, `mgr+8 := *(*(gvar_007DA4A4+0x140)+0x1A0)` (copy chuỗi từ form khác), `FUN_007B37B0(*(gvar_007DA4A4+0x140))`, gán hằng `DAT_007538CC` vào `*(*gvar_007DA2DC+0x138)+0x16C`.
- Cặp 0x01(code0)/0x07(code0) = nhận/từ chối một lời mời loại unknown — **chưa kết luận được** đối tượng lời mời.

#### 4.8.2. SubOp `0x02` — `0x00752C54` (2518B) — **toàn bộ bảng xếp hạng**

- `hdrLen=RP[1]` → `mgr+8 := RP[2..1+hdrLen]`; `page := RP[?]` một byte vào `mgr+0x1C`; `count := byte+4` ghi thẳng **`gvar_007DA6D0`** (`00752c54_FUN_00752c54.c:170-205`).
- Vòng `count` dòng, mỗi dòng `22+n` byte (`:212-760`): `[id:4B][n:1B][name:n][f1..f4:4×1B][D1:4B][D2:4B][x:1B][D3:4B]` → ghi **bảng `gvar_007DA6E8`** stride `0x27*2=78`: row[0]=id (`:278`), name→`row+6` cap `0xE` (`:291`), f1..f4→`row+0x15..0x18` (`:309-375`), D1→`+0x19`, D2→`+0x1D` (`:394,413`), x→`+0x25` (`:435`), D3→`+0x21` (`:454`).
- Dòng 1..3: id copy vào `mgr+0xC..0x14`; dòng 1 → `player+0x474` (`:256-265`); nếu dòng có id==self → `FUN_0058EA00(*007DA530,1)` (`:266-267`). Dòng vượt `mgr+0x1C` (page-start) bị **xóa id** (`:218-227`) — cơ chế phân trang.
- Đuôi: `[listLen][list]` → `mgr+0x20` + nạp vào `*gvar_007D9F14` qua `FUN_00563E80` (`:774-776`); `[D1:8B]→mgr+0x28` (chính double mà tầng handler test `!=0.0` — **xác minh link §4.1**), `[D2:8B]→mgr+0x30`, byte cuối → `mgr+0xB2` (`:783-805`); refresh `007DA358` nếu đang mở (`:806`).

#### 4.8.3. SubOp `0x03` — `0x00754540` (214B)

- `id = DWORD(RP[1..4])` → `mgr+4 := id` (`00754540_FUN_00754540.c:38-40`); `FUN_0070C20C(*gvar_007D9D34,id)` tra actor 800 (`:41`); nếu thấy: `FUN_0063C1D0(*gvar_007D9D6C, actor, 0)` (`:47`) + `FUN_0063C8FC(*gvar_007D9D6C, text=0x75462C, 5000, 0,0, 0x752BE8/mgr, 0x752C0C/mgr, 0x752C30/mgr)` (`:49`) — cấu hình timeout/notification 5000ms + 3 callback/method-con trỏ vào mgr (các con trỏ `0x752Bxx` nằm cạnh 2 literal xóa-ID đã biết `0x752B84/98` — nhiều khả năng thêm literal, chưa dump). **Chưa kết luận được** nội dung sự kiện.

#### 4.8.4. SubOp `0x04` — `0x00754094` (613B)

- `id=RP[1..4]` (`00754094_FUN_00754094.c:55-56`) + `sub=RP[5]` (guard `len(RP)>=6`); actor tồn tại thì `banner = tên actor (slot+9, shortstring 14) + suffix`, `_PStrNCat` cap `0x1E..0x2C` theo sub: 1→`DAT_00754308`, 2→`0x754324`, 3→`0x754340`, 4→`0x754350` (`:74-113`) — **1200ms**. sub ngoài 1..4 hoặc id lạ: im lặng.

#### 4.8.5. SubOp `0x07` — `0x00753ABC` (603B)

- `code=RP[1]` (`00753abc_FUN_00753abc.c:47`): 1→`DAT_00753D74`, 2→`DAT_00753D98`, 3→`DAT_00753DC4`, 0xFF→`DAT_00753DE0` banner 1200ms.
- `code==0`: `id=RP[2..5]`; nếu `id==self`: banner `DAT_00753D2C`, `player+0x474:=0` (`:58`), `FUN_00753638(mgr)`, `FUN_0058EA00(007DA530,0)` nếu `007DA530+0x26E≠0`, `FUN_00596A7C(007DA1DC, player+0x43E<0xD ?0:1)` (`:63`), đóng 4 form (`VMT+0x24` tại `007D9F14/007DA0B0`; `007D9F1C` xử theo cờ `+0x1FC/+0x1FD` — `FUN_00760AE4/00760E94`). id ≠ self → banner `DAT_00753D4C` (`:86`).

#### 4.8.6. SubOp `0x08` — `0x00752104` (1569B)

- Nhận thẳng `id` (tầng handler decode). `idx=FUN_00722508(*gvar_007D9C48,id)` (`00752104_FUN_00752104.c:95`); nếu có tên trong cache: **`gvar_007DA6D0++` (cap 0x67→BoundErr) và thêm dòng 78B vào `gvar_007DA6E8`**: row[0]=id (`:111`), name từ cache `+8` cap `0xE` (`:177`), cờ `+0x3C→row[5]`, `+0x1C→row[0x17]`, `+0x3D→row[0x18]`, `+0x1E→row[0x15]`, `+0x1D→row[0x16]`, số thập phân 9 chữ số dựng từ cache `+0x42..0x4A` (byte ×10^k) (`:216-316`), số thứ hai từ `+0x4B..0x5C` (`:327-...`), `+0x8E→row[0x25]` (`:449`); cuối: refresh `FUN_00570EF4(*gvar_007DA358)` (`:450`). id không có trong cache: **im lặng, không thêm dòng** (nhánh `else` không tồn tại — chỉ `if (uVar3 != 0)` `:97`).
- → 0x08 = "thêm/cập nhật 1 người vào bảng xếp hạng từ cache tên", khớp khuôn dòng với 4.8.2.

#### 4.8.7. SubOp `0x09` — `0x00754360` (467B)

- Lặp record `9+n`: `[id:4B][D:4B][n:1B][text:n]` tới hết RP (`00754360_FUN_00754360.c:53-116`).
- `id==self` → `player+0x478 := text`, `player+0x5F0 := D`, `mgr+8 := text` (`:97-99`); ngược lại tra actor (≤800) → `actor+0x478 := text`, `actor+0x5F0 := D` (`:108-113`).
- Kết: `FUN_007590C8(mgr)` + `FUN_007592A0(mgr)` (`:118-119`). Field `+0x478` (xâu — có thể tên bang hội/clan?) chưa kết luận.

#### 4.8.8. SubOp `0x0B` — `0x0056D3E8` (107B)

- `_LStrCopy(RP,2,len-1)` → `FUN_00563E80(DAT_0094928C, str)` (`0056d3e8_FUN_0056d3e8.c:38-42`): **tham số 1 (manager `007DA2DC`) hoàn toàn không được dùng** — đính chính "dùng manager khác": mọi phiên bản call cùng chỉ ghi vào global `DAT_0094928C` (vùng .data `0x949xxx` — cùng cụm với ignore-list `DAT_00949284` của `opcode_02.md` §8) → gợi ý **danh sách chuỗi toàn cục (ignore/blocked?)**, chưa kết luận.

#### 4.8.9. SubOp `0x0C` — `0x00754D44` (572B)

- Tìm slot trống `mgr+0x10..0x18` → ghi id, giữ index `local_e` (2..4) (`00754d44_FUN_00754d44.c:44-62`).
- Nếu `gvar_007DA6D0 > 4`: quét dòng 5..cap `0x67` tìm `row[0]==id`; thấy → **copy dòng 78B từ vị trí tìm được về dòng `local_e`** (`:81-110`), dồn các dòng sau (`:119-157`), `FillChar` dòng cuối 0x4E, **`gvar_007DA6D0--`**, `mgr+0x1C++` (`:160-178`).
- Refresh form `007DA0B0` (`FUN_0056B124`) nếu mở; nếu id==self: `FUN_00566118(007D9F14)` + `FUN_0058EA00(007DA530,1)` (`:186-196`).

#### 4.8.10. SubOp `0x0E` — `0x00754724` (1568B) — `(mgr, id, op)` với `op=RP[1]`

- Backup **dòng hạng 1** (`gvar_007DA6E8+0x4E`, 78B → stack `local_60`) (`00754724_FUN_00754724.c:63-68`); tìm `id` ở dòng 2..`gvar_007DA6D0-1` (`:71-97`); thấy → **copy dòng tìm được lên hạng 1**, `mgr+0xC := id`, `player+0x474 := id` (`:100-109`); rồi phân nhánh `op` (param_3):
  - `op==0`: xóa dòng gốc (dồn bảng, `gvar_007DA6D0--`) (`:206-275`) — "kéo tụt lên xếp hạng".
  - `op==2`: đặt id của dòng backup vào slot trống `mgr+0x10..0x18`, **restore backup về đúng dòng vừa tìm** (`:277-315`) — hoán đổi hạng 1 ↔ hạng k.
  - `op==3`: nếu dòng <5 thì `gvar_007DA6D0++`; restore backup vào dòng k; khi backup là self → đóng/refresh 5 form `007DA0B0/007DA4FC/007DA358/007D9ED4` + `FUN_0058EA00(...,0)` (`:396-460`).
  - op khác: không restore.
- Kết: id==self → `FUN_0058EA00(*007DA530,1)`; refresh `007D9F14`/`007DA0B0` nếu mở (`:461-470`). **Tên chính xác của 3 mode (giáng/xóa/chèn?) chưa kết luận được** — chỉ có cơ chế.

#### 4.8.11. SubOp `0x0F` — `0x007551F4` (1186B) — bảng 26 banner

- `K=RP[1]` (guard `len(RP)>=2`, `007551f4_FUN_007551f4.c:24-28`). Map K→literal (tất cả **1200ms**, form `007DA084`): 1→`7556FC`, 2→`755724`, 3→`755754`, 4→`755774`, 5→`7557AC`, 6→`7557D8`, 7→`755810`, 8→`755854`, 9→`75587C`, 0xA→`7558BC`, 0xB→`7558E0`, 0xC→`75591C`, 0xD→`75594C`, 0xE→`75597C`, 0xF→`7559AC`, 0x10→`7559C4`, 0x11→`7559F0`, 0x13→`755A50`, 0x14→`755A80`, 0x15→`755AB8`, 0x16→`755AE4`, 0x17→`755B04`, 0x18→`755B40`, 0x19→`755B74`, 0x1A→`755BA8`, 0x1B→`755BC8`, 0x64→`LAB_755BE0`, 0x65→`755C10`, 0xFF→`755C3C`.
- **Ngoại lệ K=0x12** (`:107-111`): `FUN_0063C1D0(*gvar_007D9D6C, player, 0)` rồi banner qua **form khác** `*gvar_007D9D6C` với `DAT_00755A0C` và tham số `(0,0,1)` (frame 5-arg khác khuôn ms=1200 — tham số 3 có thể không phải ms; chưa chốt).
- K còn lại (0, 0x1C..0x63, 0x66..0xFE): no-op.

#### 4.8.12. SubOp `0x32` — `0x007564D0` (180B)

- `player+0x136A := RP[1]` (guard `len(RP)>=2`, `007564d0_FUN_007564d0.c:44-50`); `dword = decode(RP[2..5])` (`:51-52`); `player+0x1358 + mode*0x13 := dword` (`:58-64`). Guard `mode-1 > 4` (`:53-57`) **ném `_BoundErr` lên biến decode trước khi dùng, còn index ghi thì bằng đúng `mode`** → `mode==0` crash; `mode≥6` **không crash nhưng ghi lệch ra `player+0x1423+`** (out-of-range im lặng) — bug của chính client. Không word, không tick, không `FUN_0072B390` — **bản gọn của 0x33** (§4.6).

---

## 5. Chuỗi VISCII → UTF-8

- Wire không mang chuỗi tự do ở tầng handler (đuôi SubOp 05 `P[15..]` cần sample live); **nhưng các body mới lộ thêm chuỗi trên dây ở callee**: SubOp 02 mang **2 chuỗi tự do** (header `P[3..]` → `mgr+8` và list `mgr+0x20` đổ vào form `007D9F14` — `00752c54.c:171,774-776`), SubOp 09 mang `text` (`00754360.c:89-99`), SubOp 0B nguyên `P[2..]` là một chuỗi (`0056d3e8.c:38-42`) → các chuỗi này vào thẳng control/TStrings → nhiều khả năng VISCII; chưa có sample live để chốt.
- Text hiển thị là hằng `.rodata`: `00798ac4` (format date), `00798ad8/00798aec` (prefix/log), `00798b50..00798e08` (banner), `00752b84/00752b98` (xóa ID), `00760854` (full), `"sound\m004.wav"`.
- **Hằng MỚI lộ từ 12 body (§4.8), chưa dump — bổ sung vào danh sách redump**: `0x007538CC`, `0x007539A8–0x00753A98` (6 banner 0x01), `0x00753D2C–0x00753DE0` (5 banner 0x07), `0x00754308/4324/4340/4350` (4 hậu tố 0x04), `0x0075462C` + `0x00752BE8/C0C/C30` (0x03), và dải `0x007556FC–0x00755C3C` (~29 banner 0x0F). Tất cả nằm trong `.text` cụm `0x752B–0x755C`.
- `redump/` chưa có `lit_798xxx.hex` → các hằng cũ vẫn **chưa decode được**. Cần redump `.rodata` `0x00798AC4–0x00798E08` + các dải mới ở trên + 1 frame SubOp 05 live rồi map cp1258/VISCII → NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:972-973`: `case 0x27: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x27 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[27][01][code]/[03][id]/[04][id+sub]/[07][code(+id)]/[08][id]/[09]/[0B][str]/[0C][id]/[0E][b+id]/[0F][K]/[32][mode+dword]  ĐÃ BÓC (§4.8)
[27][0A]/[0D]/[11]..[13]/[15]..[22]/[36]/[37][blob...]  passthrough CÒN MÙ
[27][02][bảng xếp hạng]                   §4.8.2 (+ date log nếu mgr+0x28!=0)
[27][05][id u32][name 8B][flag u8][text...] ≥15B, có sound m004
[27][06][id u32]                            xóa + banner (+ clear form nếu id==player+4)
[27][08|0C|0D][id u32]
[27][0E][b u8][id u32]                      7B
[27][10][D 8B]                              10B
[27][14]                                    banner 1200ms
[27][1D][K 01..09]                          3B, banner 2000ms
[27][33][mode][dword][word]                 ≥9B, mode 1..5
[27][39][dword]                             6B
[27][3A][dword][b][word]                    9B
[27][35]/[38]/[3B]                          exec, không param
ĐỪNG GỬI: 0x00/0x23–0x31/0x34/≥0x3C (no-op); L=1 (RangeError).
```

Lưu ý: SubOp 05 full-table → banner thay vì ghi; SubOp 06 chỉ banner nếu id có trong bảng; SubOp 10/02 chỉ date-log khi time != 0.0. **Thêm từ §4.8**: SubOp 0x01 dùng mã banner 6000ms code∈{1..5,0xFF}, code0 thay đổi state mạnh (`gvar_007DA6D0:=4`) — chỉ gửi khi bảng đã sẵn sàng; SubOp 0x07 code0 ĐÓNG 4 form (dùng làm reset test rất tốt); SubOp 0x0F `K=0x12` vẽ lên form KHÁC (`007D9D6C`); SubOp 0x32 **mode=0 crash, mode≥6 ghi tràn field player** — chỉ mock mode 1..5; SubOp 0x0B/0x09 chứa chuỗi tự do → soi encoding VISCII ngay trên màn hình (dễ verify nhất cụm).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_035_007938C3_FUN_007938c3.c` | Handler chính 43 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5771-6122` | Bản inline đối chiếu |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x27]=0x23`) + `.csv:37` + `manifest.csv:35` | Mapping |
| 4 | `functions/0077ef7c/0077eb9c/0077f098/0077eb1c/0077ee84/0077ed68` | Codec |
| 5 | `functions/0075284c/00756590/0075960c/00759680/007605ec` | Body 5 helper |
| 6 | `functions/0077f414_FUN_0077F414.c:972-973` | C→S rỗng |
| 7 | ~~`index.csv` (họ `0075xxxx` không entry)~~ → **Mới**: `index.csv:6342,6492-6502` + `functions/0056d3e8 / 00752104 / 00752c54 / 007536fc / 00753abc / 00754094 / 00754360 / 00754540 / 00754724 / 00754d44 / 007551f4 / 007564d0` | 12 body handler → §4.8 |
| 8 | `functions/0063c1d0_FUN_0063c1d0.c` + `0063c8fc_FUN_0063c8fc.c:36-47` | ngữ nghĩa slot-config của sự kiện 0x03 (mô tả cơ chế) |
| 9 | `case_035...c:46,49,71,74,139,145,154,160,166,184,188,302` | Đối chiếu case→handler 12 nhánh mới |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] 15 nhánh passthrough còn mù: `0A (00753df0)`, `0D (00754f80)`, `11–13 (00755c5c/5d74/5f14)`, `15–22 (họ 00756–00759)`, `36 (007571a4)`, `37 (00756fc0)`.
- [ ] Redump toàn bộ dải literal mới (§5): `0x7538CC`, `0x7539A8–0x753A98`, `0x753D2C–0x753DE0`, `0x754308–0x754350`, `0x75462C`, `0x752BE8–0x752C30`, `0x7556FC–0x755C3C`.
- [ ] Cơ chế bảng hạng (`007DA6D0` count + `007DA6E8` row 78B + slot `mgr+0xC..0x18` + `mgr+0x1C` page): ý nghĩa từng cột row (D1/D2/D3, f1..f4, x) và bản chất 3 mode của SubOp 0x0E — mới tới mức cơ chế, **chưa kết luận được** tên nghiệp vụ.
- [ ] `DAT_0094928C` (đích SubOp 0x0B) và `FUN_00563E80`: chưa có body/dump vùng `.data` 0x949xxx để gọi tên danh sách.
