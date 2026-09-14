# PHÂN TÍCH — Main OP 0x19 (Case 22, `FUN_0079157c` @ `0x0079157C`) — **Bus pass-through đa năng (16 nhánh SubOp, parse nằm trong hàm con)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp trong `ts_decompile/`**. Chỗ nào không có body / không có dump thì ghi rõ, không suy diễn.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). 13/14 nhánh "hộp đen" cũ đã có body (chỉ còn `0075af6c` của SubOp 0x01); dải hex `lit_7282ec/728300` đã dump và giải mã được (ngõ cụt chat SubOp 0x20 + 3 chuỗi của SubOp 0x1F). **Đính chính mã hóa**: các literal region này là **VISCII (RFC 1456), không phải cp1258** — bằng chứng: byte `0xA7`→ậ, `0xD5`→ạ, `0xEC`→ì, `0xFD`→ư, `0xFE`→ợ khớp VISCII từng byte và cho ra tiếng Việt có nghĩa; bảng cp1258 của Windows/Python chỉ trùng ở các slot ASCII-à-lân-cận (F0=đ, F3=ó, EA=ê…) và trả về vô nghĩa ở phần còn lại ("Nh§n đß₫c" thay vì "Nhận được"). Các bảng "cp1258 → Tiếng Việt" ở `opcode_02.md` mục 5 thực chất là đọc VISCII.

---

## 0. Tóm tắt nghiệp vụ

**OP 0x19 CÓ SubOp.** Handler `FUN_0079157c` là một **bộ chia nhánh thuần túy (dispatcher cấp 2)**: đọc 1 byte `SubOp = payload[1] = ECX[0]`, `switch` ra **16 nhánh**, mỗi nhánh chỉ làm đúng 1 việc là **forward nguyên RestPayload cho 1 hàm con + 1 object toàn cục**, không tách field nào ở tầng case, không gọi codec Word/DWORD nào ở tầng này.

- **15/16 nhánh đã có body để bóc** (cập nhật 2026-09-14): SubOp 2 (`FUN_0075aff4`, toast 0–8 + default), 0x20 (`FUN_007281ec`, resolve tên + chèn dòng chat hệ thống), và 13 nhánh vừa được Ghidra phục hồi: `0075b3ec` (0x03), `0075bf7c` (0x0A), `0075b8e0` (0x0B), `0060bbe0` (0x0C), `0062e41c/0062e11c/0062e538/0062e814` (0x14–0x18), `00735184/00742a50` (0x15/0x21, họ THuman), `00728310` (0x1F), `00742f78` (0x22), `0074df70` (0x29).
- **1/16 vẫn là hộp đen**: SubOp 0x01 → `func_0x0075af6c` — **vẫn chưa có file body** (tính đến bản dump 2026-09-14): glob `functions/0075af6c*` rỗng, `index.csv` 0 entry, chỉ còn call-site `case_022_0079157C_FUN_0079157c.c:29`.
- Khác OP 0x0D (team/toast) và OP 0x10 (GM/quiz): OP 0x19 là **bus rẽ nhánh theo module** (mỗi SubOp trỏ về 1 manager khác nhau).

---

## 1. Entry, mapping, framing, cách đọc SubOp

**Entry:**
- `ts_decompile/case_functions/functions/case_022_0079157C_FUN_0079157c.c:8` — `void FUN_0079157c(void)` @ `0x0079157C` (toàn file 107 dòng).
- `ts_decompile/case_functions/manifest.csv:24` — `22,0x0078AA0E,0x0079157C,EXPORTED,"FUN_0079157c"`.
- `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` — mục Case 22, body giống file case riêng.

**Dispatcher `FUN_0078a89c`:**
- Nhận `param_2` (**MainOp**), `param_1` (**RestPayload đã cắt MainOp**); tra bảng byte `0x78A8EE` lấy index rồi tra bảng dword `0x78A9B6` để nhảy. Nhánh `case 0x19:` tại **dòng 4475–4550** chứa toàn bộ 16 SubOp, khớp từng dòng với file case riêng → dispatcher không thêm logic, single source of truth là file case.
- Dấu vết khóa XOR: dòng 562–565 `iVar21 = 0xad` — vòng khóa tĩnh `0xAD`.

**Mapping MainOp 0x19 → Case 22 (xác minh 4 nguồn, KHÔNG phải identity):**
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` — offset `0x19` = **`0x16` = 22 thập phân** → `byte_table[0x19] = 22 = Case 22`.
- `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` — entry index 22 = **`7C 15 79 00` = `0x0079157C` (LE)** → khớp manifest.
- `manifest.csv:24` + jump-table entry `0x0078AA0E` → khớp.
- `0078a89c_FUN_0078a89c.c:4475` `case 0x19:` → nội dung trùng file case.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD` từng byte. Server speaks first. Token `F4 44` hiện diện trong `token_recv.hex`.

**Cách đọc SubOp (file case dòng 20–27):**
```c
iVar2 = *(int *)(unaff_EBP + -0xc);              // ECX = RestPayload (payload gốc đã cắt byte P[0]=MainOp)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + 0); // SubOp = ECX[0] = payload[1]
switch(SubOp) { case 1 ... case 0x29 ... }        // KHÔNG có case 0, KHÔNG có default
```
- `unaff_EBP-0xc` = RestPayload. **SubOp = `payload[1]` = `ECX[0]`**, đọc bằng 1 byte thường, không codec.
- Quy ước Delphi `Copy(ECX, p, n)` là **1-based**: `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`. **Handler này không gọi `LStrCopy` lần nào ở tầng case** — toàn bộ parse (nếu có) nằm trong hàm con.
- Guard duy nhất ở tầng case: gửi payload rỗng (thiếu cả SubOp) thì lỗi; còn lại mọi SubOp có trong switch đều được forward.

**Codec helper (đã đọc body — nhưng handler này KHÔNG dùng cái nào ở tầng case):**
- `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c` — `b0 + b1*0x100` = **Word LE**. Tầng case **không gọi lần nào**; các hàm con mới phục hồi gọi nhiều nơi (`FUN_0075b3ec.c:94`, `FUN_0062e538.c:63`, `FUN_007707a0`-họ 0x1E…).
- `FUN_0077ef7c` — `b0+b1*0x100+b2*0x10000+b3*0x1000000` = **DWORD LE**. Tầng case **không gọi**; hàm con gọi rộng (`FUN_007281ec.c:57`, `FUN_0075b3ec.c:66`, `FUN_0075bf7c.c:38`, `FUN_0062e41c.c:42`, `FUN_00735184.c:44`, `FUN_00742a50.c:41`, `FUN_00742f78.c:63`, `FUN_0060bbe0.c:89`…).
- `0077ed68` — body mới đọc: giải mã **4 byte → số nguyên 32-bit LE** (như `FUN_0077ef7c`, thêm handling dấu); chỉ hàm con dùng (`FUN_0075b3ec.c:157`).
- `FUN_0077eb1c` — encode Word → chuỗi 2 byte. Chỉ dùng chiều C→S, không dùng ở S→C 0x19.
- `FUN_0077ee84` — encode DWORD → chuỗi 4 byte. Chỉ dùng chiều C→S, không dùng ở S→C 0x19.
- `FUN_0077f098` — copy tên 8 byte. Handler 0x19 không gọi.

---

## 2. Đối chiếu dispatcher inline

Khối `case 0x19:` trong `0078a89c_FUN_0078a89c.c:4475–4550` khớp 1:1 với file case riêng (cùng 16 label, cùng thứ tự, cùng cặp `(gvar, RestPayload)`; bản dispatcher chỉ thêm gán địa chỉ traceback, không phải logic). → Dispatcher không thêm logic nào ngoài jump-table.

---

## 3. Bảng tổng hợp SubOp

`switch(SubOp)` có các label: **`1, 2, 3, 10 (0xa), 0xb, 0xc, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1f, 0x20, 0x21, 0x22, 0x29`**. **Không có `case 0`, không có `default`** → SubOp 0 / các giá trị vắng mặt (4–9, 0xd–0x13, 0x19–0x1e, 0x23–0x28, ≥0x2a) rơi qua switch, chỉ chạy cleanup chuỗi cuối hàm, không crash.

Quy ước `P[.]` = payload gốc (`P[0]=0x19`), `RP[.]` = RestPayload (`RP[0]=P[1]=SubOp`).

| SubOp | Payload tối thiểu (tầng case) | Wire field ở tầng case | Handler tầng case |
|---|---|---|---|
| `0x01` | **2** (`19 01`) | không tách field, forward nguyên RP | `func_0x0075af6c(*(gvar_007DA700), RP)` — **vẫn hộp đen** (không body) |
| `0x02` | **2** (`19 02`); hiệu lực đầy đủ cần **3** (`19 02 sel`) vì callee đọc `RP[1]` | forward nguyên RP; parse trong callee | `FUN_0075aff4(*(gvar_007DA700), RP)` — **có body**: sel `RP[1]` + toast (param_1 bị callee bỏ qua) |
| `0x03` | **2** (`19 03`); hiệu lực đầy đủ cần **6** rồi bội số 11-byte record (xem 4.4) | forward nguyên RP; callee parse | `func_0x0075b3ec` — **có body**: count DWORD + bảng record 11B → form `gvar_007DA3C4` |
| `0x0A` | **2** (`19 0A`); hiệu lực cần **6** (`Copy(RP,2,4)`) | forward nguyên RP; callee parse | `func_0x0075bf7c` — **có body**: id DWORD LE → `FUN_0060b188(**gvar_007DA438, id)`; **param_1 (gvar_007DA77C) không dùng** |
| `0x0B` | **2** (`19 0B`); hiệu lực cần **3** (`RP[1]`) | forward nguyên RP; callee parse | `func_0x0075b8e0` — **có body**: switch sel 0–13 → toast 3000ms, default → 1000ms; refresh `**gvar_007DA438`; **param_1 không dùng** |
| `0x0C` | **2** (`19 0C`); hiệu lực đầy đủ cần **≥18 + chuỗi + 10B/record + 57B đuôi** (xem 4.7) | forward nguyên RP; callee parse chi tiết | `func_0x0060bbe0` — **có body**: form chi tiết vật phẩm (`**gvar_007DA438` + field form) |
| `0x14` | **2** (`19 14`); hiệu lực cần **≥7** (`id DWORD` + `RP[5]`) | forward nguyên RP; callee parse | `func_0x0062e41c` — **có body**: flag trạng thái tại `player+0x1344` + gọi 2 method của `param_1` |
| `0x15` | **2** (`19 15`); hiệu lực cần **≥6** (`id DWORD` + `RP[5]`) | forward nguyên RP; callee parse | `func_0x00735184` — **có body**: lazy-create `THuman` (VMT_70B1C4) tại `player+0x1348` + đặt vị trí theo mode |
| `0x16` | **2** (`19 16`); hiệu lực cần **3** (sel `RP[1]`), nhánh 1 cần **5** | forward nguyên RP; callee parse | `func_0x0062e11c` — **có body**: switch sel 1–6 → toast/banner 1500ms (nhánh 1 tra tên theo mã) |
| `0x17` | **2** (`19 17`); hiệu lực cần **≥9** (id DWORD + Word + 2 byte) | forward nguyên RP; callee parse | `func_0x0062e538` — **có body**: banner 1200ms + dòng chat `"…<tên>…"` theo id resolve (họ giao dịch/quà) |
| `0x18` | **2** (`19 18`); hiệu lực cần **3** (`RP[1]` = số trang 0–5) | forward nguyên RP; callee parse | `func_0x0062e814` — **có body**: copy/tráo bản ghi 57B giữa bản ghi hoạt động `param_1+0x1a0` và trang `+0x1a0+b*4` |
| `0x1F` | **2** (`19 1F`); hiệu lực cần **3** (`RP[1]` = mã 1–3) | forward nguyên RP; callee parse | `func_0x00728310` — **có body** (đính chính — KHÔNG phải "name prefix"): 3 tin nhắn quà tặng cố định (đã dịch, xem 4.10) |
| `0x20` | **2** (`19 20`); hiệu lực đầy đủ cần **6** (`19 20 id4`) vì callee `Copy(RP,2,4)` | forward nguyên RP; parse trong callee | `FUN_007281ec(*(gvar_007D9C48), RP)` — **có body**: `id = DWORD LE RP[1..4]` + chat hệ thống `"Bạn đã nhận được<tên>Bó hoa gửi đi"` |
| `0x21` | **2** (`19 21`); hiệu lực cần **≥6** | forward nguyên RP; callee parse | `func_0x00742a50` — **có body**: song sinh 0x15 với THuman thứ hai tại `player+0x135c` |
| `0x22` | **2** (`19 22`); hiệu lực đầy đủ theo bội số 6-byte record | forward nguyên RP; callee parse | `func_0x00742f78` — **có body**: batch `[id DWORD][Word]` → ghi `actor+0x488` cho tối đa 800 actor |
| `0x29` | **2** (`19 29`); guard `len≥2` nhưng byte không dùng | forward nguyên RP; callee chỉ guard | `func_0x0074df70` — **có body**: ghi `player+0x1510 = (player+0x440)&0xFF ^ 0x7B` rồi **gửi ACK 0x19 C→S** (xem 4.12, 5) |

---

## 4. Chi tiết từng SubOp

### 4.1. Các hộp đen còn lại — chỉ còn 1 nhánh (`0x01`)
- Wire tầng case: `[19][sub][rest...]` — **không đọc `RP[1+]` ở tầng case, không Word/DWORD**, forward nguyên `RP` cùng 1 object toàn cục (giữ nguyên như bản 2026-09-12 — đã đối chiếu lại toàn bộ 16 call-site trong `case_022_0079157C_FUN_0079157c.c:28–74`).
- Phân nhóm object theo dispatcher (không đổi): `0x01/0x02/0x03` → `gvar_007DA700`; `0x0A/0x0B` → `gvar_007DA77C`; `0x0C` → `gvar_007DA438`; `0x14/0x16/0x17/0x18` → `gvar_007DA32C`; `0x15/0x21/0x29` → `gvar_007DA7BC`; `0x1F/0x20` → `gvar_007D9C48`; `0x22` → `gvar_007D9D34`.
- **Phát hiện từ body mới:** nhiều callee **không dùng param_1** được dispatcher truyền (`FUN_0075aff4`, `FUN_0075b8e0`, `FUN_0075bf7c`, `FUN_00770170`-họ…) — chúng thao tác trực tiếp qua global; nên "cùng object ⇒ cùng module" chỉ còn là gợi ý phân nhóm call-site, không còn là ràng buộc nghiệp vụ.
- **Giới hạn còn lại (duy nhất):** `func_0x0075af6c` (SubOp 0x01) không có body — glob rỗng, `index.csv` 0 entry, chỉ có call-site `case_022…c:29`. Không suy diễn; Mock chỉ gửi `[19 01][bytes]` và log.

### 4.2. SubOp `0x02` — mã chọn + toast hệ thống (có body)
- Wire: `[19][02][sel:P2=RP1, 1B]` (tầng case forward nguyên RP; callee đọc tiếp).
- Đọc trong `0075aff4_FUN_0075aff4.c:43–90`: guard `Len(RP)<2 → BoundErr(1)` (d.43–47); `switch(RP[1])` d.49.
- Core: `sel` 0–8 → toast duration **3000ms** (9 hằng `DAT_0075b1e4…LAB_0075b3a0`, d.52–84); `default` (mọi giá trị còn lại) → toast `DAT_0075b3cc` **1000ms** (d.88); sau đó luôn refresh `(VMT+0x24)(**(gvar_007DA3C4))` (d.90 — **đính chính nhỏ**: form được refresh là `gvar_007DA3C4`, không phải `gvar_007DA700` mà nhánh truyền vào). param_1 không dùng.
- Chuỗi: **vẫn không có `lit_75b*.hex`** (dải `0x75b1e4…0x75bf5c` chưa dump) → chỉ ghi địa chỉ + duration, chưa dịch được, không bịa nội dung.

### 4.3. SubOp `0x20` — id DWORD + dòng chat hệ thống "quà hoa" (có body, chuỗi đã dịch)
- Wire: `[19][20][id:P2..P5=RP1..RP4, DWORD LE]` (6 byte payload).
- Đọc trong `007281ec_FUN_007281ec.c:56–59`: `_LStrCopy(RP,2,4)`; `id = FUN_0077ef7c` = DWORD LE (d.57); `slot = FUN_00722508(obj, id)` (d.59).
- Core (`007281ec_FUN_007281ec.c:60–71`):
  1. `if (slot==0) return` — id lạ (không resolve được slot) thì bỏ qua, không hiển thị gì.
  2. Ngược lại dựng shortstring: `FUN_00402b90(local_40, &DAT_007282ec)` (d.62 — copy prefix) + `_PStrNCat(local_40, *(gvar_007DA6BC + slot*4) + 8, 0x21)` (d.67 — nối **tên** trong bảng tên toàn cục `gvar_007DA6BC`, slot bounds `0x834`=2100, maxLen 33) + `_PStrNCat(local_70, &LAB_00728300, 0x2e)` (d.69 — nối suffix).
  3. `FUN_007ab870(**gvar_007DA1B0 = TTalkMsgForm, 0, msg, tag='\n'=10)` (d.71) — chèn 1 dòng chat hệ thống, tag 10.
- **Chuỗi đã giải mã (hex dump `lit_7282ec.hex` / `lit_728300.hex`, VISCII):**
  - `0x007282EC` (shortstring len `0x10`=16): **`Bạn đã nhận được`** (16 ký tự, không có space đuôi).
  - `0x00728300` (shortstring len `0x0D`=13): **`Bó hoa gửi đi`** (13 ký tự).
  - **Đính chính:** "hậu tố 46 chars" là đọc nhầm tham số `0x2e` (=46) của `_PStrNCat` — đó là **maxLen** của buffer đích (`local_70` byte[48]), **không phải độ dài chuỗi**; độ dài thật là **13 ký tự** (đã kiểm: 13 byte, khớp length byte `0x0D`).
- Thông điệp ghép: `Bạn đã nhận được<tên>Bó hoa gửi đi` (không có space giữa các mảnh — đúng như byte trong binary).

### 4.4. SubOp `0x03` — đổ bảng record 11 byte vào form `gvar_007DA3C4` (có body mới)
- Body `0075b3ec_FUN_0075b3ec.c` (đỉnh 190 dòng, size 37B header + thân). Bỏ qua param_1.
- Wire parse: `Copy(RP,2,4)` → **count DWORD LE** (d.65–66) → `IntToStr` → `FUN_007b372c` = **set caption** control `**(gvar_007DA3C4) + 0x208` (d.67–68).
- Số record: `(Len(RP) - 5) div 11` (d.69–74); mỗi record **11 byte** bắt đầu từ RP index 6: `code Word LE` (d.93–94) + 5 byte rời (d.95–149) + `q 4B LE` qua `FUN_0077ed68` (d.151–157); `FUN_0065b580(*gvar_007DA660, code, 0, q)` → byte màu/cờ (d.158–161); ghi qua `FUN_00609910` vào object `**(gvar_007DA3C4) + 0x130 + idx*4` với idx chạy từ 1, **bounds 25** (d.170–174).
- Ngữ nghĩa cơ chế: **đổ một bảng tối đa 25 dòng (mã vật phẩm + 5 tham số byte + số 4B) vào form `gvar_007DA3C4`, caption = tổng count**. Không khẳng định tên nghiệp vụ (chưa định danh được form).

### 4.5. SubOp `0x0A` — id → hàm đơn trên `**gvar_007DA438` (có body mới)
- `0075bf7c_FUN_0075bf7c.c:37–39`: `Copy(RP,2,4)` → `id DWORD LE` → `FUN_0060b188(**(gvar_007DA438), id)`. Chỉ 1 call, không UI trực tiếp. param_1 (`gvar_007DA77C`) bị bỏ qua — **đối tượng thực sự là form `gvar_007DA438`** (cùng form với SubOp 0x0C, xem 4.7).

### 4.6. SubOp `0x0B` — bảng toast 15 mã (có body mới)
- `0075b8e0_FUN_0075b8e0.c:40–110`: guard `Len(RP)<2`; `switch(RP[1])` case `0…0xD` (14 mã) → banner `(VMT+0x90)(**gvar_007DA084, LBL, 3000)` với các literal `DAT_0075bbb4 / bbec / bc20 / bc58 / bc7c / bca8 / bce8 / bd28 / bd70 / bd9c / bde0 / LAB_0075be30 / be84 / bec4 / bf14`; **default** (mọi mã ≥14) → `DAT_0075bf5c` **1000ms** (d.106–108). Luôn chạy `(VMT+0x24)(**gvar_007DA438)` sau switch (d.110).
- param_1 (`gvar_007DA77C`) không dùng. Toàn bộ literal nằm trong dải **`0x75bbb4…0x75bf5c` — chưa dump** (cùng họ gap `lit_75b*` với SubOp 0x02) → chưa dịch.
- **Đính chính phân nhóm:** SubOp 0x0B **không** hoạt động trên `gvar_007DA77C`; đích thực là `gvar_007DA084` (banner) + `gvar_007DA438` (form).

### 4.7. SubOp `0x0C` — form chi tiết vật phẩm, parse sâu 429 dòng (có body mới)
- `0060bbe0_FUN_0060bbe0.c` (2435 byte) trên `param_1 = **(gvar_007DA438)` (object form lớn, field `+0x13c/+0x140/+0x180/+0x1a0…/+0x1d0/1d4/1d8/+0x1e0/+0x208`):
  - `RP[1..4]` DWORD → IntToStr → caption `+0x1e0` (d.88–91).
  - `RP[5..6]` Word = **mã vật phẩm** → gọi VMT+0x1c của record tại `param_1+0x13c` (load item theo mã, d.93–95); `RP[7]` → record+`0x3fa` (d.96–102).
  - 4 Word tiếp: `RP[8..9]→+0x404`, `RP[10..11]→+0x3ea`, `RP[12..13]→+0x406`, `RP[14..15]→+0x3ec` (d.104–118).
  - `RP[16]` = **N** → `Copy(RP,18,N)` = **chuỗi biến độ dài tự do** → caption `+0x180` (d.119–128).
  - byte **ngay sau chuỗi** = `RP[17+N]` (0-based) → `uVar5 = FUN_006250d4(*gvar_007D9DE4, record+0x13c→+4)` (d.148) rồi `FUN_0065b3f4(*gvar_007DA660, player+0x3fa, uVar5&0xFF, byte đó)` → clamp ≤0xFF → record `+0x55e` (d.150–155); nếu `record+4≠0`: caption `+0x180 = _LStrCatN(3)` với operand visible `DAT_0060c578` + `IntToStr(byte 0x55e)` (operand thứ ba Ghidra mất — d.156–165).
  - 3 khối `+0x1d0/+0x1d4/+0x1d8`: byte RP dịch chuyển → hoặc clear control, hoặc set `+0x4c = FUN_007c9b38(*gvar_007D9ED8, "6064"/"4083"/"4086")` (tra **bảng chuỗi tài nguyên theo key số**, d.185/212) + caption `_LStrCat3(DAT_0060c5a0, IntToStr(n))` (d.190/217) rồi SetCaption (d.176–246).
  - Loop 10-byte records (số lượng = byte RP đếm được): mỗi record `[code Word][4 byte][q DWORD]` → tra `FUN_00774af8/0077499c/00774a84` trên `gvar_007DA540` (bảng tên item) rồi ghi vào trang điều khiển `param_1+0x1a0 + i*4`, **bounds 6** (d.270–405).
  - Đuôi: `Copy(...,0x39=57 byte)` → `Move` vào `param_1+0x140`, gắn con trỏ vào `+0x208→+0x130`, `FUN_00612174` (d.406–417).
- Cơ chế: **điền toàn bộ một form chi tiết/giao dịch vật phẩm** (caption, item code, tên động, 3 slot đặc biệt, bảng record 10B, block 57B). Literal `0x60be71/0x60c5a0/0x60c…` chưa dump → không dịch text.

### 4.8. SubOp `0x14/0x16/0x17/0x18` — họ trạng thái quanh `**gvar_007DA32C` (có body mới)
- **`0x14` (`0062e41c_FUN_0062e41c.c:41–68`)**: `id DWORD LE RP[1..4]`; `idx = FUN_0071d1bc(**gvar_007DA7BC, id)` (tra slot nhân vật tổ đội? bounds 4 — xem 4.9). Nếu idx≠0 đọc `RP[5]` (guard len≥6): `=1` → `player+0x1344=1` + `FUN_0062ea58/0062ead0(param_1)`; `=2` → (nếu `player+0x35f==0`: `*(*(player+0x57c+idx*4)+0x35c)=1`) + `player+0x1344=0` + `FUN_0062de08/0062eaf4(param_1)`. Cờ 1 byte tại `player+0x1344` bật/tắt kèm gọi 2 method của object `param_1`.
- **`0x16` (`0062e11c_FUN_0062e11c.c:50–75`)**: `switch(RP[1])` case 1: `Copy(RP,3,2)` Word → `FUN_00774a84(**gvar_007DA540, code, &name)` (tên theo mã) + nối `DAT_0062e2b8` → banner **1500ms**; case 2–6: banner tĩnh `DAT_0062e2ec/32c/378/3b8/3e8` **1500ms**; case >6 im lặng. Literal `0x62e2b8…` chưa dump.
- **`0x17` (`0062e538_FUN_0062e538.c:60–119`)**: `id = DWORD LE RP[2..5]` (`Copy(RP,3,4)`, d.60), `code = Word LE RP[6..7]` (d.62–63), `n = RP[8]` (d.64–70), `slot = FUN_00722508(**gvar_007D9C48, id)` (d.71). Nếu slot≠0 và `RP[1]∈{1,2}`: dựng shortstring `LBL(DAT_0062e7d0 ≤0x1b)+tên(slot)` (nhánh 1) hoặc `LBL(DAT_0062e7f4 ≤0x2e)+tên` (nhánh 2), nối `_LStrCatN(5)` quanh `tên-theo-code` + `IntToStr(n)` + `DAT_0062e7e4/0062e7f0` → banner **1200ms** (`0x4b0`, d.95–96) **và** dòng chat `FUN_007ab870(**gvar_007DA1B0, id, msg, '\n'=10)` (d.98). Message kiểu "giao/nhận <n> <vật phẩm> từ <tên>" — **không khẳng định chiều nào là mua/bán** (literal chưa dump).
- **`0x18` (`0062e814_FUN_0062e814.c:24–103`)**: `b = RP[1]` (bounds 5). Nếu `b≠1`: dùng `idx = *(param_1+0x1a0→+0x131)` (≤0x19): `*(param_1+0xe8+idx*4 → +0x130)=0`, `+0x10c=1`, `*(*(player**gvar_007DA7BC)+0x9e0+idx*4 → +0x24)=0`; rồi **copy nguyên bản ghi trang b → bản ghi hoạt động**: các field `+0x4c`, `+0x108`, `+0xe4`, caption `+0xac` (SetCaption `FUN_007b0628`), `+0x131`; sau đó **clear trang b**: `+0x4c=-1`, `+0x108=0`, `+0xe4=0`, caption=nil, `+0x131=0`; chốt `param_1+0x3c4=0`. Cơ chế "áp bản ghi trang b vào trang active rồi xóa trang b".

### 4.9. SubOp `0x15/0x21` — cặp spawn `THuman` (có body mới)
- `00735184_FUN_00735184.c` (SubOp 0x15, `param_1 = **gvar_007DA7BC`): `id DWORD LE RP[1..4]` (d.43–44); lazy-create `THuman_Create(VMT_70B1C4)` tại `player+0x1348` (d.45–48); gọi VMT+0x1c `(human, id, 0)` — **load dữ liệu theo id** (d.49); set 2 field float `human+0x340/0x344` (d.51–52); `mode = RP[5]` (guard len≥6) lưu `player+0x134c` (d.53–59).
  - mode 1 (d.61–84): `idx = FUN_0071d1bc(player, id)`; nếu idx∈[1..4]: clear `*(player+0x57c+idx*4)+0x35c` và `human+0xe5`, copy tọa độ `player+0x1c/0x20/0x54/0x58` → human cùng offset, `human+0x4c = **gvar_007D9C28+0x1c`, rồi `FUN_0070dc78(human, x, y, …)` đặt vào scene → cờ `human+0xe3`.
  - mode 2 và mode 4 rơi vào **cùng một khối** (d.86, d.110 → d.112–131): `player+0x1c + 400 → human+0x1c` và `player+0x54 + 400 → human+0x54` (cả hai cặp tọa độ đều lệch +400), các field còn lại như mode 1 (nhưng `+0x4c = player+0x1c`, không qua `gvar_007D9C28`). mode 3 (d.87–108) cùng khuôn +400, sai khác thứ tự gán `+0x4c/+0x50`. **Chưa kết luận được** ý nghĩa nghiệp vụ của từng mode (chỉ thấy offset 400 trên hai field vị trí).
- `00742a50_FUN_00742a50.c` (SubOp 0x21): **song sinh** của 0x15 trên slot THuman thứ hai `player+0x135c` (d.42–46), thêm set `+0x3d4=5`, `+0xdd=0xAC` (d.50–51), mode `RP[5]`→`player+0x1360` (d.54–58), mode 1 = copy tọa độ hiện tại (d.59–73), mode 2 = tọa độ +400 (d.74–95). Cùng khuôn — khẳng định 0x15/0x21 là **hai biến thể spawn/di chuyển avatar phụ trợ**.

### 4.10. SubOp `0x1F` — 3 tin nhắn quà tặng cố định (có body mới — ĐÍNH CHÍNH nhãn cũ "name prefix")
- **Đối chiếu literal thất bại**: `00728310` không nối tên, không dùng `DAT_007282ec`. Body `00728310_FUN_00728310.c:41–59`: guard `Len(RP)<2 → BoundErr(1)`; `switch(RP[1])`:
  - `==1` → dòng chat `FUN_007ab870(**gvar_007DA1B0, 0, &DAT_007283d0, '\n'=10)` (d.48–51).
  - `==2` → dòng chat `&DAT_00728404`, tag 10 (d.52–55).
  - `==3` → banner `(VMT+0x90)(**gvar_007DA084, &DAT_00728430, 0x5dc=1500ms)` (d.56–59).
  - khác → không làm gì.
- **Chuỗi đã giải mã** (`lit_7282ec.hex` + `lit_728300.hex` — hai dump phủ chồng cùng vùng; run const AnsiString `[FF FF FF FF][len:4LE][chars][00]`, decode VISCII):
  - `0x007283D0` (len 40): **`Bó hoa của bạn đã thuận lợi giao đến nơi`** (40 ký tự).
  - `0x00728404` (len 34): **`Rào vật phẩm của đối phương đã đầy`** (34 ký tự, nguyên văn cả chữ "Rào" — bản Việt hóa gốc máy dịch).
  - `0x00728430` (len 13): **`Không đủ tiền`** (13 ký tự).
- Nghiệp vụ (theo đúng text): mã 1 = hoa gửi đi đã giao thành công; mã 2 = "rào/đóng gói" vật phẩm phía đối phương đã đầy; mã 3 = báo thiếu tiền (banner). param_1 (`gvar_007D9C48`) không dùng.

### 4.11. SubOp `0x22` — batch Word theo id actor (có body mới)
- `00742f78_FUN_00742f78.c:52–99` (`param_1 = **(gvar_007D9D34)`): số record `= (Len(RP)-1) div 6`; mỗi record `[id: DWORD LE][w: Word LE]` (d.62–72). Nếu `id == **gvar_007DA7BC + 4` (id của chính người chơi) → đích = player; ngược lại `slot = FUN_0070c20c(param_1, id)` (bounds 800) → `*(gvar_007DA300 + slot*4)` (d.79–92). Ghi `Word` vào `actor+0x488` (d.93–95).
- Nếu batch chạm chính mình: resync 2 manager UI `**gvar_007DA32C +0x290` và `**gvar_007D9E5C +0x3c0` khi cờ điều kiện (`+0x14`, `+0x294≠0`, đích `+0x79==1`) (d.100–113) — **cùng cặp manager resync với OP 0x1F SubOp 0x0C**.

### 4.12. SubOp `0x29` — cờ XOR + **ACK C→S** (có body mới)
- `0074df70_FUN_0074df70.c:21–29`: guard `Len(RP)<2 → BoundErr(1)` — **byte `RP[1]` bị đọc-vì-guard nhưng không dùng**.
- Ghi `player**gvar_007DA7BC → +0x1510 = (param_1+0x440)&0xFF XOR 0x7B` (d.24–28).
- **Bằng chứng dương đầu tiên cho chiều C→S của OP 0x19** (d.29): gọi `FUN_0077f414(**gvar_007D9D30, <low byte = 0x19>)` — `gvar_007D9D30` chính là object xuất hiện làm self ở mọi codec (`FUN_0077ef7c/eb9c/ed68`) ⇒ **TFConnect**. Client gửi ngược một frame 0x19 ngay khi nhận SubOp 0x29. Tham số payload (ECX) không hiện trong decompile Ghidra (artefact `CONCAT31`) — **chưa kết luận được** payload ACK dài bao gồm gì.

---

## 5. Đối chiếu chiều Client → Server trong `FUN_0077F414`

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`switch(param_2 & 0xff)`, `TFConnect.SendCommand`):
```c
case 0x19:
  (**(code **)(&DAT_0078540f + (uint)DAT_007853e5 * 4))();
  return;
```
**Kết luận: C→S OP 0x19 KHÔNG phải `break` rỗng** (khác OP 0x0D/0x10). Client có đường gửi 0x19 nhưng thực hiện bằng **nhảy gián tiếp qua bảng con** với key là global runtime (không phải param payload) và Ghidra báo không khôi phục được jumptable. Vì key là trạng thái UI runtime nên **không resolve tĩnh được nhánh gửi nào, không trích được builder payload C→S**. Ghi rõ giới hạn, không suy diễn.

**Bổ sung 2026-09-14 — bằng chứng dương đầu tiên:** handler S→C SubOp `0x29` gọi **trực tiếp** `FUN_0077f414(**gvar_007D9D30, <low byte 0x19>)` ngay khi xử lý gói (`0074df70_FUN_0074df70.c:29`) ⇒ tồn tại ít nhất một luồng gửi 0x19 kiểu ACK/phản hồi mà **không** đi qua bảng con `0x7853E5/0x78540F`. Nội dung payload của lần gửi này không khôi phục được từ decompile (ECX không hiện) — chưa kết luận được.

---

## 6. Chuỗi hiển thị / mã hóa tiếng Việt

~~Tiền lệ "cp1258 → NFC"~~ — **ĐÍNH CHÍNH 2026-09-14**: giải mã trực tiếp `lit_7282ec.hex`/`lit_728300.hex` cho thấy mã hóa thực là **VISCII (RFC 1456)** — mỗi ký tự có dấu là 1 byte precomposed (`0xA7`=ậ, `0xD5`=ạ, `0xD8`=ử, `0xEC`=ì, `0xFD`=ư, `0xFE`=ợ, `0xB5`=ộ, …). Thử cp1258→NFC trên cùng bytes trả về chuỗi vô nghĩa (`Nh§n đß₫c` thay vì `Nhận được`, `BƠn` thay vì `Bạn`). Bảng "cp1258" trong `opcode_02.md` mục 5 thực ra là bảng VISCII đọc nhầm nhãn. Các chuỗi dump mới đều decode VISCII và cho tiếng Việt hoàn chỉnh, nên kết luận decode là chắc chắn.

Tầng case 0x19 **không tham chiếu hằng chuỗi nào**. Text nằm trong các hàm con có body:

| # | Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|---|
| 1 | `DAT_0075b1e4…LAB_0075b3a0` + `DAT_0075b3cc` (sel 0–8 toast 3000ms + default 1000ms) | `FUN_0075aff4` (SubOp 2) | **Vẫn chưa có `lit_75b*.hex` → chưa dịch được** |
| 1b | `DAT_0075bbb4…DAT_0075bf14`, `LAB_0075be30`, `DAT_0075bf5c` (sel 0–13 toast 3000ms + default 1000ms) | `FUN_0075b8e0` (SubOp 0x0B) | **Cùng dải gap `lit_75b*` → chưa dịch được** |
| 2 | `DAT_007282ec` (tiền tố chat, shortstring len byte `0x10`) | `FUN_007281ec` (SubOp 0x20) | ✅ `lit_7282ec.hex`: **`Bạn đã nhận được`** (16 ký tự) |
| 3 | `LAB_00728300` (hậu tố chat, shortstring len byte `0x0D`) | `FUN_007281ec` (SubOp 0x20) | ✅ `lit_728300.hex`: **`Bó hoa gửi đi`** (13 ký tự — "46 chars" cũ là maxLen `0x2e`, đã đính chính ở 4.3) |
| 4 | `DAT_007283d0` (ansistring len 40) | `FUN_00728310` (SubOp 0x1F mã 1) | ✅ `lit_7282ec.hex` offset 0xE4 (header `[FF FF FF FF][28 00 00 00]` tại 0xDC): **`Bó hoa của bạn đã thuận lợi giao đến nơi`** |
| 5 | `DAT_00728404` (len 34) | `FUN_00728310` (SubOp 0x1F mã 2) | ✅ offset 0x118: **`Rào vật phẩm của đối phương đã đầy`** |
| 6 | `DAT_00728430` (len 13) | `FUN_00728310` (SubOp 0x1F mã 3) | ✅ offset 0x144: **`Không đủ tiền`** |
| 7 | `DAT_0062e2b8, 0062e2ec, 0062e32c, 0062e378, 0062e3b8, 0062e3e8` | `FUN_0062e11c` (SubOp 0x16) | **Chưa dump** → chưa dịch |
| 8 | `DAT_0062e7d0, 0062e7e4, 0062e7f0, 0062e7f4` | `FUN_0062e538` (SubOp 0x17) | **Chưa dump** → chưa dịch |
| 9 | `0x60be71…`, `DAT_0060c5a0` + key chuỗi tài nguyên `"6064"/"4083"/"4086"` | `FUN_0060bbe0` (SubOp 0x0C) | **Chưa dump**; key tra qua `FUN_007c9b38(**gvar_007D9ED8, …)` (bảng chuỗi runtime, không phải hằng code page) |

Không suy đoán nội dung tiếng Việt khi chưa có bytes. Cần dump tiếp theo Delphi ansistring/shortstring tại các dải `0x62e2b8–0x62e7f8`, `0x60b–0x60c` và `0x75b1e4–0x75bf5c` rồi decode **VISCII**.

---

## 7. Ghi chú cho Mock Server

1. **Phạm vi:** OP 0x19 là **bus rẽ nhánh S→C**. Nhánh hiển thị được kiểm chứng đầy đủ: `0x02` (toast), `0x1F` (chat/banner quà tặng — text đã dịch), `0x20` (chat "Bạn đã nhận được…Bó hoa gửi đi" khi id resolve), `0x16/0x17` (banner, text chưa dịch), `0x0B` (toast, text chưa dịch). Còn lại là nhánh UI/state (0x03/0x0A/0x0C/0x14/0x15/0x18/0x21/0x22) và **duy nhất `0x01` là hộp đen** — test cách ly, log crash.
2. **Frame:** `F4 44 | Len:Word LE (= độ dài payload) | payload`, toàn bộ bytes socket = plain XOR `0xAD` từng byte (`b^0xAD`: `F4→59`, `44→E9`, `00→AD`, `19→B4`, `02→AF`, `20→8D`).
3. **Bảng frame tính sẵn (payload tối thiểu tầng case = 2B):**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 1 min | `19 01` | `F4 44 02 00 19 01` | `59 E9 AF AD B4 AC` |
| SubOp 2 sel=0 (toast 3000ms) | `19 02 00` | `F4 44 03 00 19 02 00` | `59 E9 AE AD B4 AF AD` |
| SubOp 2 default (toast 1000ms, sel=9) | `19 02 09` | `F4 44 03 00 19 02 09` | `59 E9 AE AD B4 AF A4` |
| SubOp 0x20 id=1 | `19 20 01 00 00 00` | `F4 44 06 00 19 20 01 00 00 00` | `59 E9 AB AD B4 8D AC AD AD AD` |
| SubOp 0x29 min | `19 29` | `F4 44 02 00 19 29` | `59 E9 AF AD B4 84` |

4. **Thứ tự test an toàn:** `19 01` (hộp đen duy nhất, quan sát) → `19 02 09` (toast default 1000ms) → `19 02 00..08` (toast 3000ms từng mã) → `19 1F 01/02/03` (2 dòng chat + banner "Không đủ tiền" — an toàn, text đã biết) → `19 20 <id sniff được>` (chỉ hiện khi id resolve được slot qua `FUN_00722508`; id lạ = no-op) → các nhánh state/UI (`0x14/0x15/0x18/0x21/0x22`) có điều kiện `19 29` **sẽ khiến client tự gửi ngược một frame 0x19** (ACK) — cân nhắc khi so byte-echo.
5. **Không gửi SubOp `0x00` / các giá trị vắng mặt** (`04–09`, `0D–13`, `19–1E`, `23–28`, `≥2A`) — rơi qua switch (vô hại nhưng vô nghĩa).
6. **Chiều C→S 0x19:** bảng con trong `FUN_0077F414` vẫn không resolve tĩnh được; **nhưng** đã có một caller trực tiếp `0x0074df70` (ACK sau SubOp 0x29) — mock server nên chấp nhận frame 0x19 lên từ client mà không coi là lỗi giao thức.

---

## 8. Source trail (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_022_0079157C_FUN_0079157c.c` | @`0x0079157C`, toàn file 107 dòng | Handler chính, 16 SubOp, wire field |
| 2 | `ts_decompile/case_functions/manifest.csv` | dòng 24 | Case index 22, entry `0x0078AA0E` → `0x0079157C` |
| 3 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 22 | Xác nhận mapping lần 2 |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | khóa `0xAD`; `case 0x19:` d.4475–4550 | Dispatcher MainOp, đối chiếu inline 1:1 |
| 5 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x19:` | C→S nhảy gián tiếp (không rỗng) |
| 6 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | offset `0x19` = `0x16` = 22 | Xác minh MainOp→Case |
| 7 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry 22 `7C 15 79 00` | `= 0x0079157C` |
| 8 | `ts_decompile/redump/token_recv.hex` | (`F4 44`) | Token framing |
| 9 | `ts_decompile/redump/subtable_0x7853E5.hex`, `subtable_0x78540F.hex` | — | Bảng con chiều C→S (không resolve key runtime) |
| 10 | `0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | codec | Xác minh không dùng ở tầng case |
| 11 | `0075aff4_FUN_0075aff4.c` | d.43–90 | Core SubOp 2 (sel + toast + refresh `gvar_007DA3C4`) |
| 12 | `007281ec_FUN_007281ec.c` | d.56–71 | Core SubOp 0x20 (id DWORD + prefix/name/suffix + chat tag 10) |
| 13 | **Giới hạn còn lại: 1 hàm con không body** (`0075af6c`, SubOp 0x01) | glob rỗng, `index.csv` 0 hit, call-site `case_022…c:29` | Không suy diễn |
| 14 | Body mới SubOp 0x03/0x0A/0x0B/0x0C | `0075b3ec_FUN_0075b3ec.c` d.65–174; `0075bf7c…c:37–39`; `0075b8e0…c:40–110`; `0060bbe0…c` (429 dòng) | 4.4–4.7 |
| 15 | Body mới SubOp 0x14–0x18 | `0062e41c…c:41–68`, `0062e11c…c:50–75`, `0062e538…c:60–119`, `0062e814…c:24–103` | 4.8 |
| 16 | Body mới SubOp 0x15/0x21/0x22/0x29 | `00735184…c:43–131`, `00742a50…c:41–95`, `00742f78…c:52–113`, `0074df70…c:21–29` | 4.9–4.12 + §5 ACK |
| 17 | `00728310_FUN_00728310.c` | d.41–59 | Core SubOp 0x1F (3 mã quà tặng) |
| 18 | `ts_decompile/redump/lit_7282ec.hex`, `lit_728300.hex` | content 0x7282EC (16B), 0x728300 (13B), 0x7283D0 (40B), 0x728404 (34B), 0x728430 (13B) | Giải mã VISCII chuỗi SubOp 0x1F/0x20 |
| 19 | **Giới hạn chuỗi còn lại** | `lit_75b*.hex` vẫn **chưa dump** (0x75b1e4–0x75bf5c); dải `0x62e2b8–0x62e7f8`, `0x60be71…` chưa dump | Chưa dịch, không bịa |

*Ghi chú trung thực (cập nhật 2026-09-14): mọi kết luận nghiệp vụ đều suy trực tiếp từ branch + call trong source trên; 5 chuỗi mới dịch từ hex dump VISCII, reproduce nguyên văn kèm độ dài byte đã kiểm. Giới hạn còn lại: `func_0x0075af6c` (1 nhánh) và các literal chưa dump (`lit_75b*`, `0x62e…`, `0x60b…`).*
