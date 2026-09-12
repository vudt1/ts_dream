# PHÂN TÍCH — Main OP 0x29 (41) / Case 37 / `FUN_007943F9` @ `0x007943F9`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). File case 159 dòng, `switch` 22 nhánh, không default.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh **fan-out điều khiển actor/NPC + flag player** — kiểu OP 0x24 (exec/passthrough + vài nhánh đọc field thật), không phải kênh số cộng/trừ, không chat-log `FUN_007ab870`.
- 3 họ:
  - **Passthrough blob (12 nhánh)**: `0x01,02,04,05,06,07,09,0x0B,0x0D,0x15,0x17,0x33` — trao nguyên `RP` cho hàm con (manager chính `*gvar_007D9D34` họ `0073/0074xxxx`, `*gvar_007D9C70` họ `0063xxxx`, riêng `0x17` dùng `*gvar_007DA188`). Body 12/14 hàm con chưa phục hồi.
  - **Decode tường minh (8 nhánh có body)**: `0x03` (loop list DWORD → `FUN_0071cdec`), `0x08` (clear 201 entry, không đọc wire), `0x0A` (Word → banner số), `0x14` (batch NPC 10B/record), `0x16` (free slot), `0x34` (ghi row 6B) + 3 nhánh ghi byte `0x0C/0x0E/0x32`.
  - **Exec tĩnh**: `0x0F` (`func_0x00637760`, chưa body).
- Text duy nhất: cặp hằng `UNK_00798E60/00798EA4` (SubOp 0x0A, banner 2000ms) — chưa có dump. Wire chỉ số + blob.
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
| `0x01` | `[29][01][blob...]` | passthrough | `func_0x007369e8(mgr9D34, RP)` (chưa body) |
| `0x02` | `[29][02][blob...]` | passthrough | `func_0x007367f4(mgr9D34, RP)` (chưa body) |
| `0x03` | `[29][03][id:4B]*` (`2+4k` B) | loop `_LStrCopy(RP,pos,4)`+`EF7C` | Từng id → lookup object → `FUN_0071cdec(obj)`; null → break |
| `0x04` | `[29][04][blob...]` | passthrough | `func_0x00635be4(mgr9C70, RP)` (chưa body) |
| `0x05` | `[29][05][blob...]` | passthrough | `func_0x00637070(mgr9C70, RP)` |
| `0x06` | `[29][06][blob...]` | passthrough | `func_0x00635e20(mgr9C70, RP)` |
| `0x07` | `[29][07][blob...]` | passthrough | `func_0x0073b560(mgr9D34, RP)` |
| `0x08` | `[29][08]` (2B) | không đọc | `FUN_007358b0(mgr9D34)`: clear 201 entry `007D9FB4` + `*(mgr+100)=0` |
| `0x09` | `[29][09][blob...]` | passthrough | `func_0x006367cc(mgr9C70, RP)` |
| `0x0A` | `[29][0A][w:2B LE]` (4B) | `_LStrCopy(RP,2,2)`+`EB9C`→`IntToStr` | Banner `hằng + số + hằng` (`98E60/98EA4`) 2000ms |
| `0x0B` | `[29][0B][blob...]` | passthrough | `func_0x006355c8(mgr9C70, RP)` |
| `0x0C` | `[29][0C][v:1B]` (3B) | `v=RP[1]` (len≥2) | `*(player+0x1435)=v` |
| `0x0D` | `[29][0D][blob...]` | passthrough | `func_0x0073d618(mgr9D34, RP)` |
| `0x0E` | `[29][0E]` (2B) | không đọc | `*(player+0x1436)=0` |
| `0x0F` | `[29][0F]` (2B) | không đọc | `func_0x00637760(mgr9C70)` (chưa body) |
| `0x14` | `[29][14][slot][id:4B][x:2B][y:2B][dir]*` (10B/record) | `slot=RP[pos]`, id=`EF7C`, x/y=`EB9C`, dir byte | Batch sinh/cập nhật `TForeignNpc` 11 slot (`007DA10C`) + refresh |
| `0x15` | `[29][15][blob...]` | passthrough | `func_0x00745520(mgr9D34, RP)` |
| `0x16` | `[29][16][slot:1B]` (3B) | `slot=RP[1]` (len≥2, `>10` → BoundErr) | Free + null `007DA10C[slot]` nếu khác null |
| `0x17` | `[29][17][blob...]` | passthrough | `func_0x005bd580(*gvar_007DA188, RP)` |
| `0x32` | `[29][32][v:1B]` (3B) | `v=RP[1]` (len≥2) | `*(player+0x142C)=v` + `*(player+0x1430)=GetTickCount()` |
| `0x33` | `[29][33][blob...]` | passthrough | `func_0x006378d0(mgr9C70, RP)` |
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

- Chuyển nguyên RP + manager; cắt field trong hàm con chưa phục hồi (17/22 handler vắng mặt). Mock ở mức tối thiểu 2B, blob cần sample live.

---

## 5. Chuỗi VISCII → UTF-8

- Không decode được từ source cho phép; wire không mang text. 2 literal duy nhất `UNK_00798E60/00798EA4` (0x0A); `redump/` không có `lit_798e*.hex`. Chuỗi ghép thêm sinh từ số (`IntToStr`).
- Cần redump `.rodata` tại 2 địa chỉ (tới null-terminator) rồi map VISCII/cp1258 → UTF-8 NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:976-977`: `case 0x29: break;` — rỗng (kẹp giữa `0x28` và `0x2a`).
- Kết luận: OP 0x29 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[29][01|02|04|05|06|07|09|0B|0D|15|17|33][blob...]  passthrough (tối thiểu 2B)
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

Độ dài chính xác: exec/flag 2B; 1-byte param 3B; `0x0A` 4B; `0x34` 8B; `0x03` bội 4; `0x14` bội 10 sau 2B. Record cuối thiếu block → `BoundErr`.

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
| 7 | ls `functions/` (17/22 vắng) + ls `redump/` (vắng `lit_798e*`) | Giới hạn passthrough + VISCII |
