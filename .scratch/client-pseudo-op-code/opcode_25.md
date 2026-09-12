# PHÂN TÍCH — Main OP 0x25 (37) / Case 33 / `FUN_007937C6` @ `0x007937C6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). 5/6 helper `func_0x0072xxxx` chưa có body → chỉ SubOp `0x01` đạt wire+logic 100%.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Pattern **"dispatcher ủy thác"** — tầng case chỉ tách SubOp, mọi decode nằm trong 6 hàm họ `0x0072xxxx` cùng chữ ký `(manager = *gvar_007D9C48, RP)`. Khác OP 0x1A/0x1F/0x23 vốn cắt field ngay ở tầng case.
- SubOp `0x01` (đã có body `FUN_00729e48`): ghi **Word trạng thái theo ID** — wire `[25][01][id:4B LE][val:2B LE]` (8B); self (`id == *(player+4)`) ghi `player+0x462 = val`, remote tra 2 bảng (`FUN_00722508` trên `007D9C48`, trần 2100 → `+0x38`; `FUN_0070c20c` trên `007D9D34`, trần 800 → `+0x462`). Yên lặng (không banner/sound/light).
- SubOp `0x02–0x06` (`func_0x00729d24/00729430/007281a8/00727f4c/00729964`): passthrough nguyên RP, body chưa phục hồi. Suy luận theo cụm địa chỉ: cùng họ setter trạng thái (02, 06), SubOp 04 có thể là notice/chat (kề hàm chat-log `0x007281EC`).
- Chiều C→S `case 0x25: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x25 (37) → byte_table[0x78A8EE][0x25] = 0x21 (33)
                 → dword_table[0x78A9B6][33] = 0x007937C6
                 → FUN_007937c6 (Case 33)
```

- File chính: `ts_decompile/case_functions/functions/case_033_007937C6_FUN_007937c6.c` (77 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5720-5755` (`case 0x25:`) — khớp 1:1. Lưu ý `case 0x25:` xuất hiện 4 lần trong file (3 lần là SubOp của OP 0x00/0x13/0x1D, chỉ dòng 5720 là MainOp 0x25).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x25`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 20-27)

```c
if (*(RP-4)==0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1], 1 byte thường
switch(SubOp){ case 1: ... case 6: ... }
// Không case 0/default → 0x00 / ≥0x07 no-op (chỉ epilogue _LStrArrayClr/_LStrClr cuối hàm)
```

### 2.4. Codec & lookup

| Helper | Vai trò đã kiểm chứng |
| :-- | :-- |
| `FUN_0077ef7c` | 4B → DWORD LE (SubOp 01: `id`) |
| `FUN_0077eb9c` | 2B → Word LE (SubOp 01: `val`) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều này |
| `FUN_0077f098` | 8B → double, không gọi trong OP 0x25 |
| `FUN_00722508` | Lookup `manager(007D9C48) × id → index` (0 = không thấy; `>0x834=2100` → BoundErr) |
| `FUN_0070c20c` | Lookup `manager(007D9D34) × id → index` (0 = không thấy; `>800` → BoundErr) |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Handler | Wire | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `FUN_00729e48` (**có body**) | `[25][01][id:4B LE][val:2B LE]` (8B) | Ghi `val` vào `+0x462` (self) / `+0x38` + `+0x462` (remote) |
| `0x02` | `func_0x00729d24` (chưa body) | passthrough `RP` | Parser trong hàm con (cùng cluster setter) |
| `0x03` | `func_0x00729430` (chưa body) | passthrough | unknown |
| `0x04` | `func_0x007281a8` (chưa body) | passthrough | Có thể là notice/chat (kề hàm chat-log) |
| `0x05` | `func_0x00727f4c` (chưa body) | passthrough | unknown |
| `0x06` | `func_0x00729964` (chưa body) | passthrough | Có thể là setter trạng thái (cùng cluster) |
| `0x00`,`≥0x07` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Ghi Word trạng thái theo ID (100%)

- **Đọc** (`00729e48.c:52-55`): `_LStrCopy(RP,2,4)` → `id = FUN_0077ef7c(...)`; `_LStrCopy(RP,6,2)` → `val = FUN_0077eb9c(...)` (ctx decode `*gvar_007D9D30`). Thiếu byte → `_BoundErr`; thừa byte bị cắt.
- **Xử lý** (`00729e48.c:56-76`):
  ```c
  if (*(player + 4) == id) *(player + 0x462) = val;      // self
  else {
    idx = FUN_00722508(mgr9C48, id);                     // remote bảng A
    if (idx > 0) { if (idx > 0x834) BoundErr; *(*(007DA6BC+idx*4) + 0x38) = val; }
    idx2 = FUN_0070c20c(mgr9D34, id);                    // remote bảng B
    if (idx2 > 0) { if (idx2 > 800) BoundErr; *(*(007DA300+idx2*4) + 0x462) = val; }
  }
  ```
  Offset `+0x462` mirror ở self + bảng B, `+0x38` ở bảng A → cùng field trạng thái Word qua 3 view. Không banner/sound/log/effect.
- Đối chứng họ: `FUN_00729a88` (cách 0xC0 byte) làm y hệt với `selector Word + flag`: `1 → +0x448/+0x36`, `2 → +0x455/+0x37`, kèm `WA0006.wav` + banner khi xóa — cùng họ "ghi byte/word trạng thái theo id".

### 4.2–4.6. SubOp `0x02–0x06` — Passthrough (chưa body)

- Cùng chữ ký `(manager, RP)`, không cắt field ở tầng case. Địa chỉ kề cluster setter/chat-log (`0x00729D24` giữa 2 setter đã biết; `0x007281A8` cách hàm chat-log `0x007281EC` đúng 0x44 byte) → suy luận có ràng buộc như bảng §3, wire chi tiết unknown — cần decompile 5 hàm con.

---

## 5. Chuỗi VISCII → UTF-8

- SubOp 01: wire chỉ `id:u32 + val:u16`, tên entity resolve từ bảng — không text trên dây.
- SubOp 02/03/05/06: không bằng chứng text ở tầng dispatcher. SubOp 04 có thể liên quan chat-log (khuôn tên + nhãn tĩnh, cp1258 như OP 0x02) nhưng chưa lộ hằng nào.
- Hằng duy nhất lộ trong cụm: `DAT_00729d0c` (banner trong `FUN_00729a88`) + `"sound\WA0006.wav"`. `redump/` không có `lit_729d0c.hex` → **chưa decode được**. Cần redump `.rodata` tại đó (+ các hằng lộ khi decompile 5 hàm con).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:970-971`: `case 0x25: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x25 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [25][01][id u32LE][val u16LE]  ; duy nhất đặc tả đủ — ghi +0x462/+0x38
S→C [25][02..06][blob...]          ; ĐỪNG GỬI — parser chưa biết
C→S [25]: KHÔNG TỒN TẠI
```

1. Frame `[F4 44][L:u16LE][payload]`, XOR `0xAD`. SubOp 01 cần đủ 8B payload; `L=1` → RangeError; `SubOp 0/≥7` no-op.
2. `id` là entity id toàn cục (0 → nhánh remote im lặng nếu không có trong bảng; vượt trần 2100/800 → RangeError có thể crash client test). Test self dùng đúng player id.
3. SubOp 01 yên lặng — dùng test số liệu không ồn.
4. Muốn đóng 100%: decompile `0x00729D24/00729430/007281A8/00727F4C/00729964` + redump `0x00729D0C`.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_033_007937C6_FUN_007937c6.c` | Handler chính, 6 nhánh + epilogue |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5720-5755` | Bản inline khớp 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` (tự parse) + `manifest.csv:33` | Mapping `0x25→33→0x007937C6` |
| 4 | `functions/00729e48_FUN_00729e48.c` | Toàn bộ wire+logic SubOp 01 |
| 5 | `functions/0077eb9c / 0077ef7c` | Codec Word/DWORD LE |
| 6 | `functions/00729a88 / 007281ec / 00728a6c` | Đối chứng cụm setter/chat-log |
| 7 | `functions/0077f414_FUN_0077F414.c:970-971` | C→S rỗng |
| 8 | grep `00729d24/00729430/007281a8/00727f4c/00729964` (chỉ trúng case+inline+jumptable) | Chứng minh 5 helper chưa body |
