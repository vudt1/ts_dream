# PHÂN TÍCH — Main OP 0x25 (37) / Case 33 / `FUN_007937C6` @ `0x007937C6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). **Cả 5 helper `func_0x0072xxxx` của SubOp 0x02–0x06 đã có body trong bản dump mới** (`index.csv:6418-6419,6425-6427`); literal `DAT_00729d0c` đã decode từ `redump/lit_729d0c.hex`. OP 0x25 giờ đạt wire+logic 100% cho 5/6 SubOp (riêng 0x04 = hàm rỗng).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Pattern **"dispatcher ủy thác"** — tầng case chỉ tách SubOp, mọi decode nằm trong 6 hàm họ `0x0072xxxx` cùng chữ ký `(manager = *gvar_007D9C48, RP)`. Khác OP 0x1A/0x1F/0x23 vốn cắt field ngay ở tầng case. **Đính chính từ body mới**: họ hàm chia 2 nhóm rõ — **setter byte trạng thái theo id** (02/06, mirror chính xác SubOp 01 nhưng là BYTE) và **nhóm chat-log** (03/05); SubOp 04 là **hàm rỗng** (không phải notice/chat như suy luận cũ).
- SubOp `0x01` (`FUN_00729e48`): ghi **Word trạng thái theo ID** — wire `[25][01][id:4B LE][val:2B LE]` (8B); self (`id == *(player+4)`) ghi `player+0x462 = val`, remote tra 2 bảng (`FUN_00722508` trên `007D9C48`, trần 2100 → `+0x38`; `FUN_0070c20c` trên `007D9D34`, trần 800 → `+0x462`). Yên lặng (không banner/sound/light).
- SubOp `0x02` (`FUN_00729D24`, **đã xác minh**): y hệt 0x01 nhưng **BYTE** — `player+0x464` (self) / bảng A `+0x3A` / bảng B `+0x464`; wire `[25][02][id:4B][v:1B]` (7B).
- SubOp `0x03` (`FUN_00729430`, **đã xác minh**): **10 thông báo chat-log tĩnh** chọn theo 1 byte `K=RP[1]` (1..10) — `FUN_007ab870(chatlog, 0, nhãn_K, 0)`; wire `[25][03][K]` (3B). Nhóm này MỚI chính là "notice" (đính chính suy đoán cũ gán cho 0x04).
- SubOp `0x04` (`FUN_007281a8`, **đã xác minh**): body 56B **hoàn toàn rỗng** (chỉ AddRef/frame/LStrClr/ret, `007281a8_FUN_007281a8.c:16-37` + asm không có call nghiệp vụ) → **no-op được lập trình sẵn** (kênh để trống/gảy code).
- SubOp `0x05` (`FUN_00727f4c`, **đã xác minh**): **chat-log danh sách id theo 3 nhóm**: `[n0:4B][n0 id:4B...] [n1:4B][...] [n2:4B][...]`, mỗi nhóm nối `IntToStr(id)` bằng `_LStrCatN(5)`, nhóm có `n>0` in 1 dòng log (id=self, prefix `IntToStr(1..3)`).
- SubOp `0x06` (`FUN_00729964`, **đã xác minh**): mirror 0x02 ở offset kế — `player+0x465` / A `+0x3B` / B `+0x465`.
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
| `FUN_0077ef7c` | 4B → DWORD LE (`id` ở SubOp 01/02/05/06; `n_j` và id danh sách ở 05) — call-site trong 5 body mới |
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
| `0x02` | `func_0x00729d24` (**có body mới**) | `[25][02][id:4B LE][v:1B]` (7B, `len(RP)>=6`) | Ghi BYTE `v` vào `+0x464` (self) / A `+0x3A` / B `+0x464` |
| `0x03` | `func_0x00729430` (**có body mới**) | `[25][03][K:1B]` (3B, K=1..10) | Chat-log 1 trong **10 thông báo hệ thống tĩnh** (`id=0`, tag 0) |
| `0x04` | `func_0x007281a8` (**có body mới**) | passthrough | **Hàm rỗng** — không làm gì (đính chính suy đoán notice/chat cũ) |
| `0x05` | `func_0x00727f4c` (**có body mới**) | `[25][05]([n:4B][id:4B]×n)×3 nhóm` | Chat-log 3 dòng: "nhóm i: id1, id2, …" cho nhóm có `n>0` |
| `0x06` | `func_0x00729964` (**có body mới**) | `[25][06][id:4B LE][v:1B]` (7B) | Mirror 0x02 tại `+0x465` (self/B) / A `+0x3B` |
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

### 4.2. SubOp `0x02` — `func_0x00729D24` — BYTE trạng thái `+0x464` (đã phục hồi)

- Đọc (`00729d24_FUN_00729d24.c:43-51`): `id = FUN_0077ef7c(*gvar_007D9D30, _LStrCopy(RP,2,4))`; `v = RP[5]` (guard `len(RP)>=6` → `_BoundErr(5)`).
- Xử lý (`:52-72`): `player+4 == id` → `player+0x464 = v`; ngược lại `idx=FUN_00722508(*gvar_007D9C48,id)` (`>0x834` BoundErr) → entry A `+0x3A = v`; `idx2=FUN_0070c20c(*gvar_007D9D34,id)` (`>800` BoundErr) → actor B `+0x464 = v`.
- Cùng khuôn 3-view với 0x01, chỉ khác size BYTE và offset `+0x464` (kề `+0x462` Word — cụm field trạng thái liền mạch). **Xác minh được từ body mới** suy đoán cũ "cùng họ setter trạng thái".

### 4.3. SubOp `0x03` — `func_0x00729430` — 10 thông báo chat-log tĩnh (đã phục hồi)

- Đọc (`00729430_FUN_00729430.c:22-27`): `K = RP[1]` (guard `len(RP)>=2` → `_BoundErr(1)`), `switch(K)` 1..10, không default → K khác no-op.
- Mỗi case: `FUN_007ab870(*gvar_007DA1B0, 0, &DAT_007295xx, 0)` — đẩy **hằng số VISCII** vào chat-log với `id=0` (hệ thống) và tag `0` (kênh "(Công bố hệ thống)" theo `opcode_02.md` §5) (`:29-56`).
- 10 nhãn: `0x00729590/5F4/660/6E0/734/788/7D8/82C(LAB)/880/914` — nằm trong `.text` cụm hàm này, **chưa có dump** (§9).
- **Đính chính**: đây mới là nhánh "notice/chat" của OP 0x25 (suy đoán cũ gắn cho 0x04 dựa trên khoảng cách địa chỉ là **sai**).

### 4.4. SubOp `0x04` — `func_0x007281a8` — hàm rỗng (đã phục hồi, đính chính)

- Body 56B: `_LStrAddRef` → setup/hủy exception frame → `_LStrClr` → `ret` (`007281a8_FUN_007281a8.c:16-37`); asm xác nhận **không có lệnh nghiệp vụ nào** (không call, không store).
- → **Đính chính hoàn toàn** suy đoán "có thể notice/chat (kề 0x007281EC 0x44 byte)": hàm kề chat-log nhưng **không làm gì** — kênh bị khai tử/chèn chỗ. Frame `RP` truyền vào bị bỏ.

### 4.5. SubOp `0x05` — `func_0x00727f4c` — chat-log danh sách id 3 nhóm (đã phục hồi)

- Mảng `AnsiString[3]` (`00727f4c_FUN_00727f4c.c:51`), con chạy `p = RP[1]` (Delphi 2). Vòng ngoài `j = 1..3` (`:55-108`):
  - `n_j = FUN_0077ef7c(_LStrCopy(RP,p,4))` (`:64-65`), `p += 4`.
  - Nếu `n_j > 0`: lặp `n_j` lần: `id = decode 4B` → `IntToStr` → nối vào chuỗi nhóm bằng `_LStrCatN(5)` với hằng phân cách `DAT_0072818C` (`:89-97`, chưa dump), `p += 4`.
- Vòng in (`:109-126`): với mỗi nhóm `n_j > 0`: dựng dòng `IntToStr(j)` + nội dung (`_LStrCatN(5)`) rồi `FUN_007ab870(*gvar_007DA1B0, *(player+4), dòng, tag)` — tag là byte đọc từ stack, **artifact decompile, chưa kết luận được** (`:116-123`).
- **Wire**: `[25][05]([n u32LE][id u32LE]×n)×3`; tổng 12B khi cả 3 nhóm rỗng. Không crash khi thiếu (codec BoundErr).

### 4.6. SubOp `0x06` — `func_0x00729964` — BYTE trạng thái `+0x465` (đã phục hồi)

- Frame đọc/xử lý **giống hệt 0x02** (`00729964_FUN_00729964.c:43-51,52-72`), chỉ khác field: self/B `+0x465`, bảng A `+0x3B` — byte thứ hai của cặp word `+0x464/+0x465` (so `+0x3A/+0x3B` ở view A). **Xác minh được từ body mới** "cùng cluster setter".

---

## 5. Chuỗi VISCII → UTF-8

- SubOp 01/02/06: wire chỉ `id:u32 + val:u16/u8`, tên entity resolve từ bảng — không text trên dây.
- SubOp 03: **10 chuỗi VISCII tĩnh** tại `0x00729590/5F4/660/6E0/734/788/7D8/82C/880/914` (đưa vào chat-log tag 0) — vùng `.text` trong cụm, **chưa có dump** → chưa dịch được nội dung.
- SubOp 05: chuỗi trên dây chỉ là **số id** (`IntToStr`) + phân cách hằng `DAT_0072818C` (chưa dump, nhiều khả năng `", "` — chưa kết luận).
- **`DAT_00729d0c` (banner trong `FUN_00729a88`) — ĐÃ DECODE từ `redump/lit_729d0c.hex`**: 23 byte `54 68 A5 6E 20 78 75 69 20 F0 E3 20 72 B6 69 20 78 61 20 62 D5 6E 21` tại `0x00729D0C–0x00729D22` (null tại `0x00729D23`; đuôi dump là code của `FUN_00729d24` — **không phải** gói AnsiString `[FF FF FF FF][len]` mà là PChar thẳng trong `.text`). cp1258-raw: `Th¥n xui đă r¶i xa bƠn!`; **suy dịch VISCII: "Thần xui đã rời xa bạn!"** — khớp logic `FUN_00729a88`: phát `"sound\WA0006.wav"` (`00729a88_FUN_00729a88.c:74`) + banner 2000ms (`:76`) khi trạng thái bị **gỡ** → "đã gỡ debuff".

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:970-971`: `case 0x25: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x25 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [25][01][id u32LE][val u16LE]  ; ghi Word +0x462/+0x38
S→C [25][02][id u32LE][v u8]       ; ghi BYTE +0x464/+0x3A   (đủ 7B payload)
S→C [25][03][K 01..0A]             ; 1/10 thông báo chat-log tĩnh (id=0, tag hệ thống)
S→C [25][04][...]                  ; no-op (hàm rỗng — an toàn tuyệt đối)
S→C [25][05]([n u32][id u32]×n)×3  ; 3 dòng chat-log danh sách id
S→C [25][06][id u32LE][v u8]       ; ghi BYTE +0x465/+0x3B   (đủ 7B payload)
C→S [25]: KHÔNG TỒN TẠI
```

1. Frame `[F4 44][L:u16LE][payload]`, XOR `0xAD`. SubOp 01 cần đủ 8B; 02/06 cần đủ 7B payload (`len(RP)>=6`); 03/05 guard `len(RP)>=2`; `L=1` → RangeError; `SubOp 0/≥7` no-op.
2. `id` là entity id toàn cục (không có trong bảng → nhánh remote im lặng; vượt trần 2100/800 → RangeError có thể crash client test). Test self dùng đúng player id.
3. SubOp 01/02/06 yên lặng — dùng test số liệu không ồn; SubOp 03/05 **in ra chat-log** (thấy được bằng mắt, là kênh kiểm chứng dễ nhất).
4. **Đóng 100% wire**: 5 helper `00729D24/00729430/007281A8/00727F4C/00729964` + `0x00729D0C` đã xong ở bản dump mới. Còn thiếu: **dịch 10 nhãn SubOp 03** (dump `lit_729590/5F4/660/6E0/734/788/7D8/82C/880/914.hex`, cùng dải `0x0072818C`).

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
| 8 | ~~grep 5 helper (chưa body)~~ → `functions/00729d24 / 00729430 / 007281a8 / 00727f4c / 00729964` + `index.csv:6418-6419,6425-6427` | **Mới**: body đủ 5 helper SubOp 02–06 |
| 9 | `redump/lit_729d0c.hex` | **Mới**: decode banner `FUN_00729a88` → "Thần xui đã rời xa bạn!" |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] Redump 10 nhãn chat-log SubOp 03 (`0x00729590/5F4/660/6E0/734/788/7D8/82C/880/914`) + phân cách SubOp 05 (`0x0072818C`) — tất cả nằm trong `.text` cụm `00727–00729`.
- [ ] `player+0x462/+0x464/+0x465` là trạng thái gì (debuff? cờ kỹ năng?) — mới chốt được offset/size, chưa chốt tên; cặp A `+0x38/+0x3A/+0x3B` mirror nhất quán.
- [ ] Tham số tag cuối (`cVar6`) của `FUN_007ab870` trong SubOp 05 là artifact decompile — chưa kết luận được.
- [ ] `FUN_00729E48` (SubOp 01) giữ nguyên như cũ — không đổi bởi batch này.
