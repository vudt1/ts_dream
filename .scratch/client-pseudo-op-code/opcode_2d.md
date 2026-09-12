# PHÂN TÍCH — Main OP 0x2D (45) / Case 41 / `FUN_00794977` @ `0x00794977`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `switch` đủ `0x01..0x10` (16 nhánh), không default.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh lai OP 0x24 + OP 0x29 — banner + ghi state player (`+0x1438` DWORD, `+0x1440` double, `+0x1448` byte, `+0x144C` qua SubOp 07) + exec tĩnh + lookup/blob passthrough.
- 3 họ:
  - **Banner + state (01,02,03,06,08,0D)**: đọc 0–12B thật; 01 theo `kind` (kind 0 ghi DWORD+double + banner, 1–5/FF banner), 02 flag (0 clear + banner, 1/FF banner; DWORD đọc ra bỏ không dùng), 03 ghi full state không banner, 06 banner theo kind, 08 chỉ `v==02` banner, 0D `player+0x1448=v`.
  - **Exec tĩnh (04,05)**: `(VMT+0x20)()` không đọc wire.
  - **Lookup/blob (07,09,0A,0B,0C,0E,0F,10)**: 09/0A lookup tên `FUN_0072288c` (0A thêm double + text đuôi + `func_0x005749f0`), 0C loop batch `n` record 8B, 07 có body (`id→+0x144C`, `idx` Word 0..100, lookup actor), 0B/0E/0F/10 passthrough (chưa body).
- Text hiển thị toàn literal `.rodata` (18 địa chỉ `0x798FD8–0x7991C8` + `0x798EA4`) — chưa có dump. Wire chỉ số, trừ đuôi text SubOp 0A `P[14..end]` (opaque, cần sample live).
- Chiều C→S `case 0x2d: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2D (45) → byte_table[0x78A8EE][0x2D] = 0x29 (41)
                 → dword_table[0x78A9B6][41] @ 0x0078AA5A = 0x00794977
                 → FUN_00794977 (Case 41)
```

- File chính: `ts_decompile/case_functions/functions/case_041_00794977_FUN_00794977.c` (332 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6372-6648` — khớp 1:1 (lộ `2000`, thứ tự CatN).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2D`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 46-53)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){ case 1 ... case 0x10 ... }
// Không default → 0x00 / ≥0x11 no-op (chỉ epilogue dòng 301-329)
```

### 2.4. Codec & API

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | 4B → DWORD LE: trực tiếp ở 01,02,03,09,0A,0C |
| `FUN_0077f098` | 8B → double: 01,03,0A |
| `FUN_0077eb9c` | 2B → Word LE: gián tiếp trong `FUN_0074384c` (SubOp 07: `P[6..7]`) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều này |
| Banner `(VMT+0x90)(...,2000,0,0)` | 01,02,06,08,09 qua `*gvar_007DA084` |
| `gvar_007DA7BC` | player: `+0x1438` DWORD, `+0x1440` double, `+0x1448` byte, `+0x144C` (qua 07) |

---

## 3. Bảng tổng hợp SubOp (16 nhánh)

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2D][01][kind:1B][...]` | `kind=RP[1]`; kind==0 thêm DWORD `P[3..6]` + double `P[7..14]` | kind 0: ghi `+0x1438/+0x1440` + banner `98FD8`; 1–5/FF: banner `98FF4/99018/9903C/99058/99070/990A0`; khác im lặng |
| `0x02` | `[2D][02][dw:4B][flag:1B]` (7B) | `dw=P[2..5]` (đọc ra bỏ), `flag=RP[5]=P[6]` | flag 0: clear `+0x1438/+0x1440/+0x1448` + banner `990B0`; 1/FF: banner `990CC/990A0` |
| `0x03` | `[2D][03][dw:4B][dbl:8B][b:1B]` (15B) | `dw→+0x1438`, `dbl→+0x1440`, `b=RP[13]→+0x1448` | Ghi full state, không banner |
| `0x04` | `[2D][04]` (2B) | không đọc | `(***gvar_007D9C1C+0x20)()` |
| `0x05` | `[2D][05]` (2B) | không đọc | `(***gvar_007DA1BC+0x20)()` |
| `0x06` | `[2D][06][kind:1B]` (3B) | `kind=RP[1]` | Banner `0→990E8, 1→99108, 2→99130, 3→99150, FF→990A0`; khác im lặng |
| `0x07` | `[2D][07][blob...]` (≥8B) | passthrough `FUN_0074384c(player,RP)` | `id=P[2..5]→+0x144C`, `idx=P[6..7]` Word (0..100), lookup actor + refresh |
| `0x08` | `[2D][08][v:1B]` (3B) | `v=RP[1]` | Chỉ `v==02` → banner `99180` |
| `0x09` | `[2D][09][id:4B][num:4B]` (10B) | `id→FUN_0072288c`→tên; `num→IntToStr` | `_LStrCatN(5)` `{991A8, tên, 991C8, số, 98EA4}` → banner 2000ms |
| `0x0A` | `[2D][0A][id:4B][dbl:8B][text...]` (≥14B) | `id→tên`, `dbl`, `trail=P[14..end]` | `_LStrCatN(3)` + `func_0x005749f0(mgr490,id,cat)` — log số thực + text đuôi |
| `0x0B` | `[2D][0B][blob...]` (≥2B) | passthrough `func_0x00743b6c(player,RP)` | Chưa body |
| `0x0C` | `[2D][0C][n:4B][rec 8B]*` (`6+8n` B) | clear `FUN_00573f50` trước; `n=P[2..5]`; record `a/b` DWORD | Loop `func_0x00573820(mgr,b,a)`; `n≤0` chỉ clear |
| `0x0D` | `[2D][0D][v:1B]` (3B) | `v=RP[1]` | `player+0x1448=v` |
| `0x0E` | `[2D][0E][blob...]` | passthrough `func_0x00743d54(mgr9D34,RP)` | Chưa body |
| `0x0F` | `[2D][0F][blob...]` | passthrough `func_0x00743e70(player,RP)` | Chưa body |
| `0x10` | `[2D][10][blob...]` | passthrough `func_0x005b1ee0(mgr600,RP)` | Chưa body |

---

## 4. Chi tiết các nhánh chính (bỏ graphics/sound/animation)

### 4.1. SubOp `0x01/02/03` — Ghi state + banner

- `01`: `kind=RP[1]` (len≥2); kind 0 đọc thêm DWORD + double (15B) → ghi + banner; kind 1–5/FF banner tương ứng; khác im lặng.
- `02` (7B): `dw` đọc ra bỏ (di sản debug); `flag=P[6]` (len≥6): 0 → clear 3 field + banner; 1/FF → banner; khác im lặng.
- `03` (15B): ghi full `+0x1438/+0x1440/+0x1448`, không banner. Thiếu byte → BoundErr trong codec.

### 4.2. SubOp `0x06/08/0D` — Banner/flag 1 byte

- `06`: `kind=P[2]` → 5 banner; khác im lặng. `08`: chỉ `v==02` banner. `0D`: ghi `+0x1448=v` (không banner).

### 4.3. SubOp `0x07` — Có body (`FUN_0074384c`)

- `id=P[2..5]→+0x144C`, `idx=P[6..7]` Word (`>100` → BoundErr), lookup `FUN_0070c20c` (guard 800) + refresh theo idx. Tối thiểu 8B.

### 4.4. SubOp `0x09/0A` — Tên + số/thực

- `09` (10B): `id→FUN_0072288c` (lookup `00722508`, `>0x834` BoundErr, `+8` là tên) → banner ghép 5 mảnh (thứ tự varargs chưa chắc).
- `0A` (≥14B): thêm double + text đuôi → `func_0x005749f0` (parser đuôi trong hàm con).

### 4.5. SubOp `0x0C` — Loop batch

- Clear `FUN_00573f50` trước; `n=DWORD(P[2..5])`; mỗi record 8B (`a=P[10+8i..]`, `b=P[6+8i..]`) → `func_0x00573820(mgr,b,a)`. `n≤0` chỉ clear. Record cuối thiếu → BoundErr.

---

## 5. Chuỗi VISCII → UTF-8

- Wire không mang chuỗi có nghĩa trừ đuôi `0x0A` `P[14..end]` (opaque, cần sample live).
- Text toàn literal `.rodata` (18 địa chỉ `798FD8–7991C8` + `798EA4`); `redump/` không có `lit_798*/799*.hex` → **chưa decode được cái nào**. Cần redump 18 địa chỉ tới null-terminator rồi map VISCII/cp1258 → UTF-8 NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:984-985`: `case 0x2d: break;` — rỗng (kẹp giữa `0x2c` và `0x2e`).
- Kết luận: OP 0x2D S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2D][01][00][dw][dbl 8B]  15B → ghi state + banner
[2D][01][01|02|03|04|05|FF] 3B → banner
[2D][02][dw][00|01|FF]    7B → clear/banner (dw điền 0)
[2D][03][dw][dbl][b]      15B → ghi full, không banner
[2D][04]/[05]             2B → exec tĩnh
[2D][06][00|01|02|03|FF]  3B → banner
[2D][07][id][idx u16]     ≥8B → idx 0..100
[2D][08][02]              3B → banner; khác im lặng
[2D][09][id][num]         10B → banner tên+số
[2D][0A][id][dbl][txt]    ≥14B → log
[2D][0B|0E|0F|10][blob]   ≥2B → passthrough (test 2B, blob replay live)
[2D][0C][n][rec 8B]*      6+8n B (test n=0)
[2D][0D][v]               3B → +0x1448=v
ĐỪNG GỬI: [00],≥0x11 (no-op); L=1 (RangeError).
```

Số nguyên LE, double 8B kiểu `F098`. Độ dài sai/thiếu block cuối → BoundErr.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_041_00794977_FUN_00794977.c` (332 dòng) | Handler chính 16 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6372-6648` | Bản inline (lộ `2000`, CatN) |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2D]=0x29`) + `.csv:43` + `manifest.csv:43` | Mapping |
| 4 | `functions/0077ef7c/0077eb9c/0077f098/0077eb1c/0077ee84` | Codec |
| 5 | `functions/0074384c/0072288c/00573f50` | 3 callee có body |
| 6 | `functions/0077f414_FUN_0077F414.c:984-985` | C→S rỗng |
