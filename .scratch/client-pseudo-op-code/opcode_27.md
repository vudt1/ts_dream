# PHÂN TÍCH — Main OP 0x27 (39) / Case 35 / `FUN_007938C3` @ `0x007938C3`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `switch` 43 nhánh lớn nhất cụm; họ passthrough `func_0x0075xxxx` chưa có body — tên nghiệp vụ mức cơ chế.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP lớn nhất cụm (364 dòng, **43 nhánh** `0x01–0x22, 0x32,0x33,0x35–0x3B`; khuyết `0x00, 0x23–0x31, 0x34, ≥0x3C`, không default). 3 họ:
  - **Họ A — Passthrough** (30 nhánh: 01–04, 07, 09–13, 15–22, 32, 36, 37 + 33/39/3A có body): `func_0x0075xxxx(manager = *gvar_007D9C20, RP)`, parser trong hàm con. Riêng SubOp `0x02` có thêm logic tầng handler (refresh form + chat-log date nếu time!=0); `0x0B` dùng manager khác (`007DA2DC`); `0x33/0x39/0x3A` đã có body (ghi timer/mode/double vào `player+0x1358–0x1410`).
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
| `0x01` | `[27][01][blob...]` | `func_0x007536fc(mgr9C20, RP)` |
| `0x02` | `[27][02][blob...]` | `func_0x00752c54` + `FUN_00765db0` + refresh form + `FUN_00596a7c` + chat-log date nếu time!=0 |
| `0x03` | `[27][03][blob...]` | `func_0x00754540(mgr9C20, RP)` |
| `0x04` | `[27][04][blob...]` | `func_0x00754094(mgr9C20, RP)` |
| `0x05` | `[27][05][id:4B][name:8B][flag:1B][text...]` (≥15B) | Ghi bảng 1000 entry stride 0xA5 + sound `m004.wav` (xem 4.2) |
| `0x06` | `[27][06][id:4B]` (6B) | Xóa ID: banner `name` + dồn mảng + clear form nếu `id==player+4` (xem 4.3) |
| `0x07` | `[27][07][blob...]` | `func_0x00753abc` |
| `0x08` | `[27][08][id:4B]` | `func_0x00752104(chatmgr,id)` + refresh 2 form |
| `0x09`–`0x0A` | `[blob...]` | `func_0x00754360 / 0x00753df0` |
| `0x0B` | `[blob...]` | `func_0x0056d3e8(*gvar_007DA2DC, RP)` (manager khác) |
| `0x0C`/`0x0D` | `[id:4B]` | `func_0x00754d44 / 0x00754f80(chatmgr,id)` |
| `0x0E` | `[27][0E][b:1B][id:4B]` (7B) | `func_0x00754724(chatmgr, id, b)` |
| `0x0F`–`0x13` | `[blob...]` | `func_0x007551f4/00755c5c/00755d74/00755f14` |
| `0x14` | `[27][14]` (2B) | Banner `UNK_00798b50` 1200ms |
| `0x15`–`0x22` | `[blob...]` | Họ `func_0x00756xxx–00759xxx` (+ refresh `007DA0B0` ở 17/1A/1B) |
| `0x1D` | `[27][1D][K:1B]` (3B, K=1–9) | 9 banner tĩnh `00798b74..00798e08` 2000ms |
| `0x10` | `[27][10][D:8B]` (10B) | `*(chatmgr+0x28)=D` + refresh; D==0 → chat-log tĩnh, D!=0 → date log |
| `0x32`/`0x36`/`0x37` | `[blob...]` | `func_0x007564d0/007571a4/00756fc0` |
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
Wire `[27][02][blob...]`, không cắt ở tầng này.

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

---

## 5. Chuỗi VISCII → UTF-8

- Wire không mang chuỗi tự do (trừ đuôi SubOp 05 `P[15..]` — cần sample live; handler `_LStrToString` → `_PStrNCpy 0x97`).
- Text khác là hằng `.rodata`: `00798ac4` (format date), `00798ad8/00798aec` (prefix/log), `00798b50..00798e08` (banner), `00752b84/00752b98` (xóa ID), `00760854` (full), `"sound\m004.wav"`.
- `redump/` không có `lit_798xxx.hex` → **chưa decode được**. Cần redump `.rodata` `0x00798AC4–0x00798E08` + 1 frame SubOp 05 live rồi map cp1258/VISCII → NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:972-973`: `case 0x27: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x27 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[27][01][blob]/[03]/[04]/[07]/[09]/[0A]/[0B]/[0F]/[11]..[13]/[15]..[22]/[32]/[36]/[37]  passthrough
[27][02][blob]                              + date log nếu time!=0
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

Lưu ý: SubOp 05 full-table → banner thay vì ghi; SubOp 06 chỉ banner nếu id có trong bảng; SubOp 10/02 chỉ date-log khi time != 0.0.

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
| 7 | `index.csv` (họ `0075xxxx` không entry) | Giới hạn passthrough |
