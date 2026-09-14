# PHÂN TÍCH — Main OP 0x1F (31) / Case 27 / `FUN_007922E6` @ `0x007922E6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). ~~Các `func_0x0077xxxx` nhận ủy thác chưa có body — ghi rõ giới hạn.~~ → **2026-09-14: cả 6 handler ủy thác đã có body; SubOp 0x03–0x06/0x0B/0x0E đã đặc tả được layout.**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). **Cấu trúc nội tại của SubOp 0x03/0x04/0x05/0x06/0x0B nay đã rõ**: manager `*gvar_007DA694` chỉ được forward; state thật nằm ở **vùng "trang sách tắt trang bị" 7×57 byte tại `player(**gvar_007DA7BC)+0xbe0…0xbe0+7*0x39`** (mỗi trang: code Word @0, tên 10B @2, một xấp Word @0xbed/0xbef/0xbf1…/0xbf9, byte @0xbfd) và **mảng actor slot `player+0x57c + i*4`, i ≤ 4** — cùng bảng item DB `FUN_00623f00(**gvar_007D9DE4, code)` (xem 4.3–4.6). **Đính chính "byte padding"** ở 0x03/0x05 và **"Set byte có base+5"** ở 0x0B. SubOp 0x0E là **dispatcher con 4 mode** (xem 4.14). Run string `0x79833c/70/a0/e4` của 0x01/0x0D **vẫn chưa có hex dump** — chưa dịch được (chỉ `0x798314` có 4/30 byte trong window dump OP 0x1A).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Server push trạng thái/notice/target: dialog 3 chế độ (SubOp `0x01`), lock-target/select-ID có điều kiện hủy (SubOp `0x02`), **đồng bộ vùng trang sách tắt trang bị 7×57B `player+0xbe0` ↔ 5 actor slot `player+0x57c`** (SubOp `0x03` actor→page, `0x04` xóa page, `0x05` page→actor, `0x06` nạp batch page từ record biến độ dài, `0x0B` ghi struct 56B đang chọn vào actor slot), show/clear cờ form (SubOp `0x07–0x0C`), Toast tĩnh (SubOp `0x0D`), và **dispatcher con 4 mode** cho manager `*gvar_007DA7C0` + bảng hotbar 123B `player+0xe28` (SubOp `0x0E`).
- **14 nhánh**: `0x01..0x0E` (1–14), không có default — SubOp lạ bị bỏ qua.
- Text hiển thị ở tầng này chủ yếu là **chuỗi tĩnh** (`UNK_0079833c/70/a0/e4` — **chưa có dump, chưa dịch được**); **Ngoại lệ mới từ body SubOp 0x06**: `func_0x0077b128` chép **raw bytes biến độ dài từ payload** vào field tên 10B của page (xem 4.6) — text trên dây duy nhất của OP 0x1F, nằm trong record biến độ dài, **không phải chuỗi tĩnh**.
- Chiều C→S `case 0x1f: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1F (31) → byte_table[0x78A8EE][0x1F] = 0x1B (27)
                 → dword_table[0x78A9B6][27] @ 0x0078AA22 = 0x007922E6
                 → FUN_007922e6 (Case 27)
```

- File chính: `ts_decompile/case_functions/functions/case_027_007922E6_FUN_007922e6.c` (250 dòng); đối chiếu bản inline `functions/0078a89c_FUN_0078a89c.c:4971-5178` (`case 0x1f:`).
- Manifest: `case_functions/manifest.csv:29` + `jumptable_0x78A9B6_case_functions.csv:29`.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x1F`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).
- `*(EBP-0x0c)` = con trỏ RestPayload; độ dài tại `*(ptr-4)`; `RP[0]` = SubOp.

### 2.3. Đọc SubOp (dòng 31-38)

```c
SubOp = (uint)*(byte*)(RestPayload + 0);  // RP[0] = P[1]
switch(SubOp){ case 1:..; ... case 0xE:..; }
```

### 2.4. Codec

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | **Có gọi** (tầng case SubOp `0x02` + trong các handler con mới phục hồi `FUN_0074df70`-họ 0x19; riêng Case 27 chỉ SubOp 0x02) |
| `FUN_0077eb9c` | Tầng case không gọi, nhưng **handler con có gọi**: `FUN_0077bd2c` (SubOp 0x0E) decode Word LE các record hotbar (vd. `0077bd2c_FUN_0077bd2c.c:92`) |
| `FUN_0077ed68` / `FUN_0077eaa4` | Decoder 4B/1B — được ghi nhận ở các OP kề (0x1E); handler 0x1F chỉ dùng `0077eaa4` trong SubOp 0x0E mode 2 (qua `0077bd2c`) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Không gọi trong Case 27; chỉ dùng chiều C→S |
| `FUN_0077f098` | Không gọi trong Case 27 (call-sites ở vùng `00792xxx`, case 28+) |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[1F][01][mode:1B]` (3B) | `mode = RP[1]`, guard `len>=2` | Dialog 3 chế độ (xem 4.1) |
| `0x02` | `[1F][02][id:4B LE][flag:1B]` (7B) | `_LStrCopy(RP,2,4)`+`FUN_0077ef7c`→id, `RP[5]`=flag (guard `len>=6` khi `id<1`) | Lock-target / hủy chọn (xem 4.2) |
| `0x03` | `[1F][03][v1:1B][v2:1B]` | guard `len>=3`; C layer chỉ hiển thị byte đầu — **callee thực nhận 2 byte** (param_2 = `RP[1]` = page ≤6; param_3 = slot ≤4 — nguồn param_3 nhiều khả năng là `RP[2]`, chưa kết luận được vì asm case không export) | `func_0x0077a55c` — **có body**: sao bản ghi actor slot `player+0x57c[slot≤4]*4` → entry trang `player+0xbe0+page*0x39` (xem 4.3) |
| `0x04` | `[1F][04][v:1B]` | `v=RP[1]`, guard `len>=2` | `func_0x0077b0e4` — **có body**: `FillChar(player+0xbe0 + v*0x39, 0x39, 0)` xóa trắng trang v, **bound v ≤ 6** (`0077b0e4_FUN_0077b0e4.c:21–35`) |
| `0x05` | `[1F][05][v1:1B][v2:1B]` | như 0x03 (guard `len>=3`, 2 byte) | `func_0x0077abd0` — **có body**: **chiều NGƯỢC 0x03** — entry trang `player+0xbe0+page*0x39` (code ≠0, kiểm DB) → actor slot `param_3 ≤4` (xem 4.5) |
| `0x06` | `[1F][06][TLV records]` biến dài | passthrough nguyên `RP` | `func_0x0077b128` — **có body**: record `[page∈1..6][code:2B][b:1B][code2:2B][n:1B][name:n]`, ≤20 record, ghi `player+0xbe0+page*0x39` (xem 4.6) |
| `0x07` | `[1F][07]` (2B) | không đọc | Show `*gvar_007DA504` (VMT+0x20) |
| `0x08` | `[1F][08]` | không đọc | Clear `*(007DA504+0x129)=0` + `*(007DA694+4)=0` |
| `0x09` | `[1F][09]` | không đọc | Clear `*(007DA504+0x12a)=0` |
| `0x0A` | `[1F][0A]` | không đọc | Set `*(007DA788+0x144)=1`, `+0x145=1`, `*(007DA37C+0x0c)=6` |
| `0x0B` | `[1F][0B][v:1B]` | `v=RP[1]`, guard `len>=2` | `func_0x0077b3c0(manager694, manager694+5, v)` — **có body**: ghi **record 56B tại manager+5** vào actor slot v (≤4) (xem 4.11) |
| `0x0C` | `[1F][0C]` | không đọc wire; đọc state `idx=*(007DA504+0xf8)` | Clear như 0x08+0x09 rồi resync selection index (xem 4.12) |
| `0x0D` | `[1F][0D]` | không đọc | Toast `UNK_007983e4` 2000ms |
| `0x0E` | `[1F][0E][mode:1B][…]` | passthrough nguyên `RP` | `func_0x0077bd2c` — **có body**: dispatcher con 4 mode, mode 2 nạp bảng hotbar `player+0xe28` 123B (xem 4.14) |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Dialog 3 chế độ

- **Wire**: `[1F][01][mode 1B]`. **Đọc**: `mode = (char)RP[1]`.
- **Xử lý**:
  1. `FUN_007ba180(*(gvar_007DA788+0x130), '\0')` — xóa text cũ (UI).
  2. `mode==0`: set text tĩnh `UNK_0079833c`, không show.
  3. `mode==1`: set text `UNK_00798370` + show modal `(**gvar_007DA788+0x20)()`.
  4. `mode==2`: set text `UNK_007983a0` + set cờ `*(gvar_007DA5A0+0xa096)=1` (giữ lại — đây là state).
  5. `mode` khác: không làm gì.

### 4.2. SubOp `0x02` — Lock-target / select-ID có điều kiện hủy

- **Wire**: `[1F][02][id 4B LE][flag 1B]` = 7 bytes dây (6 bytes RP).
- **Đọc**: `id = FUN_0077ef7c(_LStrCopy(RP,2,4))`, ghi `*(gvar_007D9D44+0x148) = id`; nếu `id<1` đọc thêm `flag = RP[5]` (guard `len>=6`).
- **Xử lý**:
  - `id<1`: `flag==0` → clear text + set `UNK_007983a0` + show modal `007DA788` (nhánh hủy chọn); `flag!=0` → show `(**gvar_007D9D44+0x20)()`.
  - `id>=1` (nhánh giữ chọn, nhưng 5 điều kiện ép về 0): slot type `0x03` qua `FUN_00634f3c`, `*(short*)(player+0x63a)==10991 (0x2AEF)` hoặc `==-15627`, `FUN_00504c50(map)!=0`, `*(player+0x1458)!=0` → `id=0`. Cuối cùng luôn show `(**gvar_007D9D44+0x20)()`.

### 4.3–4.5. SubOp `0x03/04/05` — vùng trang bị nhanh 7×57B của player (các hàm con ĐÃ có body — ĐÍNH CHÍNH "1 byte vào manager 694")

Cả ba handler **không dùng manager `*gvar_007DA694` được forward** (param_1 chỉ được cất rồi bỏ); state thật nằm trên `**gvar_007DA7BC`:
- **vùng trang bị nhanh**: `player + 0xbe0 + page*0x39`, `page ∈ 0..6` (7 trang × 57 byte). Layout một trang (suy từ các lệnh đọc/ghi): `+0x00` code Word · `+0x02` tên shortstring 10B · `+0x1D` byte (0xbfd) · `+0x0D` và xấp 6 Word (`+0x0D=0xbed-0xbe0` …) · phần đuôi `+0x2a…` 6 Word.
- **mảng actor slot**: `player + 0x57c + slot*4`, `slot ∈ 0..4` (5 con trỏ); record actor dùng lại layout field y hệt trang ở offset khác: `+4` code · `+9` tên 17B · `+0x3ea…+0x3f8` xấp Word · `+0x55d…+0x562` bytes · `+0x2c+i*4` sub-con trỏ.
- **DB kiểm tra code**: `FUN_00623f00(*gvar_007D9DE4, code)` — code không tồn tại ⇒ không copy gì.

- **`0x03` = actor → page** (`0077a55c_FUN_0077a55c.c:68–423`): guard case `len≥3`; callee nhận **2 byte**: `param_2 = RP[1]` (page, bound 6), `param_3` (slot, bound 4) — C của case chỉ hiển thị một byte, byte thứ hai đi qua thanh ghi ECX theo convention Ghidra không khôi phục trong call-site ⇒ **param_3 = RP[2] là suy luận từ guard `len≥3` + signature; chưa kết luận được 100%**. Nếu `actor+4 ≠ 0` và DB命中: `page+0xbe0 = code`, `_PStrNCpy(page+0xbe2, actor+9, 10)`, copy các Word `actor+0x3ea/0x3ec/0x3ee…0x3f8` → `page 0xbed/0xbef/0xbf1…0xbf9`, `actor+0x55d..0x55f` → `page 0xbf2?…` (chuỗi copy trường dài, kết thúc tại `page+0xc0a-0xbe0+7*2` region), rồi **nếu `*(gvar_007DA504+0xf8) ≠ 0 → = slot`** và `if (*(char *)(*(int *)gvar_007DA504 + 0x114) != '\0') *(byte*)(*gvar_007DA504+0x114) = page` (d.421–424).
- **`0x04` = xóa trang** (`0077b0e4_FUN_0077b0e4.c:20–34`): **đính chính**: đây là **`FillChar(**gvar_007DA7BC + 0xbe0 + v*0x39, 0x39, 0)`** — tham số `RP[1]` là **số trang 0..6**, không phải "value byte" vô nghĩa; xóa trắng cả trang trang bị nhanh.
- **`0x05` = page → actor** (`0077abd0_FUN_0077abd0.c:19–255`): chiều ngược của 0x03, cùng guard DB, cùng 2 byte (page ≤6 ở param_2, slot ≤4 ở param_3), kết thúc `FUN_007a5d54(*gvar_007D9E00, param_3)` (d.255) + đồng bộ `007DA504+0x114` như 0x03. **Byte "padding" thứ 3 là byte slot có thật — đính chính hoàn toàn ghi chú cũ.**

### 4.6. SubOp `0x06` — batch fill trang bị nhanh từ payload biến độ dài (CÓ BODY — không còn "parser nằm trong hàm con chưa đọc")

- **Wire**: `[1F][06]` + chuỗi **record biến độ dài**, tối đa **20 record** (counter bound `0x14`, d.64) cho tới khi cạn RP; mỗi record **`7+n` byte**:
  `[slot:1B, bắt buộc 1..6 — vượt thì break (d.72)] [code:2B LE → player+0xbe0+slot*0x39] [b1:1B → +0xbfd+slot*0x39] [w2:2B LE → +0xbed+slot*0x39] [n:1B] [text:n byte → _LStrToString → _PStrNCpy(…, 10) vào +0xbe2+slot*0x39]`
  (`0077b128_FUN_0077b128.c:55–176`; offset chạy `+= 7+n` d.166–175.)
- **name trên dây bị cắt còn 10 byte** khi ghi (buffer shortstring trong trang 57B); text là **raw bytes từ server** — theo thông lệ binary thì là VISCII; chưa có bytes mẫu để kiểm chứng.
- ⇒ Đây là **chuỗi động lực thực sự duy nhất của payload 0x1F** (không phải "không có payload text" như nhận định cũ — nhận định cũ chỉ đúng cho các SubOp khác).
- Đối chiếu chéo: khuân `player+0xbe0 + slot*0x39` giống hệt vùng mà SubOp 0x03/0x04/0x05 thao tác ⇒ SubOp 0x06 = "nạp nhiều trang", 0x04 = "xóa một trang", 0x03/0x05 = "chuyển trang ↔ actor".

### 4.7. SubOp `0x07` — Show object `007DA504`

- **Wire**: `[1F][07]`. `(***gvar_007DA504 + 0x20)()`.

### 4.8. SubOp `0x08` — Clear 2 cờ

- **Wire**: `[1F][08]`. `*(007DA504+0x129)=0; *(007DA694+4)=0`.

### 4.9. SubOp `0x09` — Clear 1 cờ

- **Wire**: `[1F][09]`. `*(007DA504+0x12a)=0`.

### 4.10. SubOp `0x0A` — Set bộ 3 cờ

- **Wire**: `[1F][0A]`. `*(007DA788+0x144)=1; *(007DA788+0x145)=1; *(007DA37C+0x0c)=6`.

### 4.11. SubOp `0x0B` — ghi record 57 byte của manager vào actor slot (CÓ BODY — đính chính "Set byte có base+5")

- **Wire**: `[1F][0B][v:1B]` (guard `len≥2`), đúng như bảng cũ; call-site giữ nguyên `func_0x0077b3c0(*gvar_007DA694, *gvar_007DA694 + 5, v)` — nhưng **`manager+5` không phải "địa chỉ base cộng 5" để callee tự tính: đó là con trỏ tới một RECORD mà callee copy**.
- **Body `0077b3c0_FUN_0077b3c0.c:50–116`**: callee **copy record 57 byte** (`14×DWORD + 1×BYTE` từ `param_2`, d.50–55 — trùng cỡ 0x39 của trang trang bị nhanh) vào stack; nếu `record.code (ushort, +0) ≠ 0` **và** `FUN_00623f00(*gvar_007D9DE4, code) ≠ 0` (code hợp lệ trong DB):
  - `actor = **gvar_007DA7BC + 0x57c + (v≤4)*4`;
  - `actor+4 = code` (d.68); `_PStrNCpy(actor+9, record+2, 0x11)` (d.69 — copy tên 17B);
  - hàng loạt field: `actor+0x3ea/0x3ec/…` ← record (d.71+), `actor+0x55d/0x55e` bytes, `actor+0x400` Word;
  - 2 vòng lặp đuôi: `actor+0x560+i ← record byte (i=1..3)` (d.90–96) và `*(actor+0x2c+i*4)+4 ← record Word (i=1..6)` (d.100–111);
  - `FUN_007a5d54(*gvar_007D9E00, v)` (d.114) và **`if (*(*gvar_007DA504+0xf8) ≠ 0 → = v`** (d.115–116) — đồng bộ selection index, cùng cặp global với SubOp 0x0C (4.12).
- Cơ chế: "apply bản ghi đang chọn ở manager 694+5 (57B) vào nhân vật slot v". v>4 ⇒ `BoundErr(4)` (d.62–66).

### 4.12. SubOp `0x0C` — Resync selection index

- **Wire**: `[1F][0C]`, không đọc wire thêm; đọc state nội `idx = *(byte*)(*(007DA504)+0xf8)`.
- **Xử lý**: clear như 0x08+0x09, rồi nếu `idx!=0` (guard `idx>4 → BoundErr`): copy con trỏ resync `player+0x57c+idx*4` vào `*(007DA32C+0x294)` (nếu `*(007DA32C+0x298)!=0`) và vào `*(007D9E5C+0x3bc)` (nếu `*(007D9E5C+0x4b4)!=0`). Phần gọi VMT vẽ (`+0x290`/`+0x3c0`) bỏ qua theo yêu cầu — giữ phép resync con trỏ.

### 4.13. SubOp `0x0D` — Toast 2000ms

- **Wire**: `[1F][0D]`. `(**gvar_007DA084+0x90)(..., &UNK_007983e4, 2000, 0, 0)` — cùng pattern OP 0x01 SubOp 05/06/07.

### 4.14. SubOp `0x0E` — dispatcher con 4 mode trên object `*gvar_007DA7C0` + vùng hotbar của player (thay cho "blob opaque")

- **Wire**: `[1F][0E][mode:1B][...]` — `func_0x0077bd2c(*gvar_007DA7C0, RP)`; **đính chính**: không phải blob một khối — `FUN_0077bd2c` (767B) là **dispatcher cấp 3**, branch theo `RP[1]` (guard `Len(RP)<2 → BoundErr(1)`; `0077bd2c_FUN_0077bd2c.c:76–83`).
- **Callers header** (`0077bd2c_FUN_0077bd2c.c:7–8,12–13`): `sub_00792850` **duy nhất** — khớp `case_functions/functions/case_027_007922E6_FUN_007922e6.c:217` (SubOp 0x0E). ⇒ **hàm này thuộc riêng OP 0x1F**, không phải handler của OP 0x1E (nhất quán với `opcode_1e.md` §4.11–4.12).
- **Mode 1** (d.84–95): `[RP[2]=slot byte][RP[3..4]=Word LE (Copy(RP,4,2))]` → `FUN_0077b614(*gvar_007DA7C0, slot, word)` (callee chưa mổ).
- **Mode 2** (d.97–217) — **nạp lại bảng hotbar của player**:
  - `_FillChar(**gvar_007DA7BC + 0xe28, 0x7b=123, 0)` (d.98) — clear 123 byte của player (`0xe28…0xeA2`).
  - `RP[2]` → `player+0xe2c` (d.110–112).
  - `n = (Len(RP)-3) div 12` record 12B liên tiếp, bắt đầu từ RP[3] (d.101–104, stride d.213): `[slot:1B, BoundErr nếu >5]` (d.121–136) + **5×Word LE** (`Copy` stride 2) → `player+0xe2d + slot*0xb` và `+0xe2d + slot*0xb + i*2`, i=1..4 (d.142, 180) + byte cuối `Copy(RP, rec+11, 1)` qua decoder 1-byte `FUN_0077eaa4` → `player+0xe37 + slot*0xb` (d.190–207).
  - Layout stride `0xb` × 6 slot (bound 5) — **không có literal nào để đối chiếu tên**; chỉ ghi nhận cơ chế.
- **Mode 3** (d.219–227): `[RP[2]=byte]` → `FUN_0077ba20(*gvar_007DA7C0, byte)`.
- **Mode 4** (d.229–230): passthrough toàn `RP` → `FUN_0077baf0(*gvar_007DA7C0, RP)` — **branch duy nhất còn opaque**; ⇒ phần "blob biến dài thật sự" của SubOp 0x0E chỉ còn thu hẹp vào mode 4.
- Mode khác: không làm gì (không default).

---

## 5. Chuỗi VISCII → UTF-8

- Case 27 **có hai loại text**: (a) chuỗi tĩnh `UNK_0079833c` (SubOp1/mode0), `UNK_00798370` (mode1), `UNK_007983a0` (mode2 + SubOp2/hủy), `UNK_007983e4` (SubOp 0x0D Toast); (b) **raw text trên dây ở SubOp 0x06** (tên item ≤10B/record, xem 4.6) — loại (b) chưa từng được ghi nhận ở bản cũ.
- `ts_decompile/redump/` **vẫn không có dump** cho 4 địa chỉ tĩnh: window `lit_798118.hex` dừng ở `0x798317` (file 512B), các address `0x79833C/70/A0/E4` nằm ngoài → **chưa decode được, giữ nguyên giới hạn**. Ghi chú thêm: run 0x798xxx kế tiếp (`0x798138…0x7982F8`) đã dump vì thuộc OP 0x1A/0x1D — dải `0x798330…0x7983FF` vẫn trống.
- Mã hóa để decode khi có dump: **VISCII** (đính chính — xem `opcode_19.md` §6; quy ước "cp1258→NFC" cũ trong các docs là đặt tên sai cho cùng bảng byte này).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:958-959`: `case 0x1f: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x1F là S→C thuần. Không có format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [1F][01][mode 00|01|02]
S→C [1F][02][id u32LE][flag u8]   ; id<1 + flag==0 → dialog hủy; còn lại show 007D9D44
S→C [1F][03][page:u8 (0..6)][slot:u8 (0..4)]   ; actor→page; "pad" cũ là slot thật (4.3)
S→C [1F][04][page:u8 (0..6)]                    ; FillChar xóa trang 57B (0077b0e4:24–33)
S→C [1F][05][page:u8 (0..6)][slot:u8 (0..4)]   ; page→actor (4.5)
S→C [1F][06][record]×≤20                        ; [slot 1..6][code w][b1][code2 w][n][name nB]; n=0 hoặc slot>6 → dừng parse
S→C [1F][07] / [08] / [09] / [0A] / [0C] / [0D]   ; no param
S→C [1F][0B][slot:u8 (0..4)]                    ; áp record 57B tại manager694+5 vào actor slot
S→C [1F][0E][mode:u8 (1..4)][...]               ; mode 2: [x:u8] rồi [slot 0..5][5×word][1B]×n; mode 4 passthrough
C→S [1F]: KHÔNG TỒN TẠI
```
- Vượt bound page/slot ⇒ `_BoundErr` trong handler ⇒ RangeError client.
- **Điều kiện để 0x03/0x05/0x0B thực sự ghi**: actor/page phải có code ≠ 0 **và** `FUN_00623f00(**gvar_007D9DE4, code)` tìm thấy — nếu DB trống thì no-op im lặng.
- **0x0E mode 2 là destructive**: clear 123B `player+0xe28` trước khi nạp — chỉ gửi khi chủ đích test hotbar.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_027_007922E6_FUN_007922e6.c` | Handler chính, 14 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:4971-5178` | Bản inline đối chiếu |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` + `manifest.csv:29` | Mapping |
| 4 | `functions/0077ef7c*` | Decode id SubOp 0x02 |
| 5 | `functions/0077f414_FUN_0077F414.c:958-959` | C→S rỗng |
| 6 | `functions/0077a55c_FUN_0077a55c.c:68–423` | **Body mới** SubOp 0x03: actor→page, DB `00623f00`, đồng bộ `007DA504+0xf8/+0x114` |
| 7 | `functions/0077b0e4_FUN_0077b0e4.c:20–34` | **Body mới** SubOp 0x04: `FillChar(player+0xbe0+page*0x39, 0x39, 0)` |
| 8 | `functions/0077abd0_FUN_0077abd0.c:19–255` | **Body mới** SubOp 0x05: page→actor + `FUN_007a5d54(*gvar_007D9E00, slot)` |
| 9 | `functions/0077b128_FUN_0077b128.c:55–176` | **Body mới** SubOp 0x06: record `7+n`, ≤20, name cắt 10B |
| 10 | `functions/0077b3c0_FUN_0077b3c0.c:50–116` | **Body mới** SubOp 0x0B: copy record 57B (14×DWORD+1B) vào actor slot v≤4 |
| 11 | `functions/0077bd2c_FUN_0077bd2c.c:76–230` | **Body mới** SubOp 0x0E: dispatcher 4 mode + hotbar `player+0xe28` 123B |
| 12 | `functions/0077250c…/ FUN_00623f00 / FUN_0065b580` | Calables được nhắc (chưa mổ) — ghi chú trong 4.x |

**Giới hạn còn lại (2026-09-14)**: (a) 4 literal tĩnh `0x79833C/70/A0/E4` chưa dump → text SubOp 0x01/0x02/0x0D chưa dịch; (b) param_3 của SubOp 0x03/0x05 là suy luận (asm của case không được export — dispatcher asm cắt 71 dòng, file case chỉ có `.c`); (c) nội dung `0x798314` chỉ có 4/30 byte trong window OP 0x1A; (d) `FUN_0077b614/0077ba20/0077baf0/0077250c-đích/0077499c-00774a84-00774af8` (họ 60bbe0 ở OP 0x19) chưa mổ; (e) tên lớp của `gvar_007DA694/007DA7C0/007DA504/007D9DE4/007DA660/007DA5A8` chưa định danh — chỉ mô tả cơ chế.
