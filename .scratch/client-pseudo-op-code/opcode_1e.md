# PHÂN TÍCH — Main OP 0x1E (30) / Case 26 / `FUN_007921A6` @ `0x007921A6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: ~~**Dispatcher + 1 handler con đã xác minh từ mã nguồn sơ cấp**; 10/11 handler con chưa có body decompile — ghi rõ giới hạn, không suy đoán layout params.~~ → **2026-09-14: 11/11 handler con đã có body và đã xác minh toàn bộ layout params.**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). **Phát hiện quan trọng nhất: mọi handler đều BỎ QUA `param_1`** — object `*gvar_007DA58C` (nhóm A) / `*gvar_007DA7C0` (nhóm B) mà dispatcher truyền vào không hề được dùng; state thật nằm trong **3 mảng con trỏ slot của player `**gvar_007DA7BC`: `+0xa48` (51 slot), `+0xb14` (51 slot), `+0xdc0` (26 slot)** + các hàm `0072c038 / 0072be9c / 00749c78` và bảng màu `*gvar_007DA660`. "Nhóm A/B theo object" chỉ còn là phân nhóm call-site, không còn là phân nhóm module.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Dispatcher cấp 2. `FUN_007921A6` chỉ đọc 1 byte SubOp rồi **ủy thác nguyên RestPayload** cho 11 handler con. Theo **nhóm call-site** (dispatcher truyền object):
  - **Nhóm A** (8 SubOp: `0x01,02,03,04,05,09,0A,0B`) — nhận `*gvar_007DA58C` nhưng **callee không dùng** (xác minh từ 8/8 body).
  - **Nhóm B** (2 SubOp: `0x0C,0x0D`) — nhận `*gvar_007DA7C0`, **callee cũng không dùng**.
  - **Nhóm C** (SubOp `0x08`, `FUN_0076fdb8()`) — không cần payload.
- **Sau khi có 10 body mới**: bản chất OP 0x1E = **đồng bộ 3 bảng slot của player `**gvar_007DA7BC`**: mảng `+0xa48` (51 slot, SubOp 0x01 batch / 0x04 single / 0x05 decrement / 0x08 clear-by-decrement), mảng `+0xb14` (51 slot, SubOp 0x09 batch / 0x0B decrement), mảng `+0xdc0` (26 slot, SubOp 0x0C single / 0x0D decrement); mỗi record 12B `[slot:1B][code:Word LE][5 byte][q:4B LE]`, code được tô màu qua `FUN_0065b580(*gvar_007DA660, code, 0, q)`; SubOp 0x02/0x0A = bản ghi đơn 11B chuyển cho `FUN_0072be9c`/`FUN_00749c78`, SubOp 0x03 = `(Word, byte)` → `FUN_0072c038`. (Chi tiết từng nhánh ở mục 4.)
- SubOp `0x06, 0x07` là slot trống (không có case → client bỏ qua).
- Trong dispatcher **và cả 11 handler con** không có literal chuỗi, Toast, caption, hay xử lý font/charset ⇒ **không có bằng chứng VISCII trong toàn bộ OP 0x1E** (grep `DAT_/UNK_/LAB_` trong 10 body mới: 0 hit ngoài header).
- Chiều C→S `case 0x1e: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1E (30) → byte_table[0x78A8EE][0x1E] = 0x1A (26)
                 → dword_table[0x78A9B6][26] @ 0x0078AA1E = 0x007921A6
                 → FUN_007921a6 (Case 26)
```

- File chính: `ts_decompile/case_functions/functions/case_026_007921A6_FUN_007921a6.c:1-4` (header Case 26).
- Dispatcher tổng: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt:26-31`.
- Framing/XOR/pump như `opcode_00_01.md` §2. `EBP-0xC` trong hàm = RestPayload (ECX lúc entry).

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x1E`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).

### 2.3. Đọc SubOp (dòng 20-26)

```c
if (*(RestPayload-4) == 0) _BoundErr(0);   // RP rỗng → RangeError
SubOp = (uint)*(byte*)(RestPayload + 0);   // RP[0] = P[1]
switch(SubOp){ case 1:..; case 2:..; case 3:..; case 4:..; case 5:..;
  case 8:..; case 9:..; case 0xA:..; case 0xB:..; case 0xC:..; case 0xD:..; }
// Không có case 6/7/default → rơi qua switch, chỉ cleanup
```

- Epilogue chung (dòng 61-89): `_LStrArrayClr/_LStrClr` ~15 biến tạm — dọn chuỗi Delphi, không phải nghiệp vụ.
- Dispatcher **không gọi codec nào** trực tiếp; chỉ `_BoundErr` + cleanup. Các codec là kiến thức nền để giải params khi có thêm body handler con.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Handler | Object theo dispatcher | Tình trạng source | Đích thật (từ body) |
| :---: | :--- | :--- | :--- | :--- | :--- |
| `0x01` | `[1E][01][12B record]×n` (`n = (Len(RP)-1) div 12`) | `func_0x007707a0` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | batch ghi `**player+0xa48[slot≤50]` VMT+4 |
| `0x02` | `[1E][02][code:2B][5×1B][q:4B]` (12B) | `func_0x00770454` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | `FUN_0072be9c(**player, …)` (`00770454…c:54–99`) |
| `0x03` | `[1E][03][w:2B LE][b:1B]` (5B) | `func_0x00770170` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | `FUN_0072c038(**player, w, b)` (`00770170…c:51–57`) |
| `0x04` | `[1E][04][12B record]` | `func_0x007705d8` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | single ghi `**player+0xa48[slot≤50]` VMT+4 |
| `0x05` | `[1E][05][slot:1B][n:1B]` (4B) | `func_0x0077021c` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | `FUN_0077250c(+0xa48[slot≤50], n)` — trừ số lượng (`0077021c…c:63`) |
| `0x06` | — | — | — | Slot trống, client ignore | — |
| `0x07` | — | — | — | Slot trống, client ignore | — |
| `0x08` | `[1E][08]` (không params) | `FUN_0076fdb8()` | none (dùng `gvar_007DA7BC` nội bộ) | **Có full source** | trừ 50 khỏi **cả 50 slot mảng `+0xa48`** (d.31) — xem 4.7 đính chính |
| `0x09` | `[1E][09][12B record]×n` | `func_0x007714a0` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | batch ghi `**player+0xb14[slot≤50]` VMT+4 (d.172) |
| `0x0A` | `[1E][0A][code:2B][5×1B][q:4B]` (12B) | `func_0x007702d0` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | `FUN_00749c78(**player, …)` (`007702d0…c:54–99`) |
| `0x0B` | `[1E][0B][slot:1B][n:1B]` (4B) | `func_0x007700bc` | `*gvar_007DA58C` (không dùng) | ✅ Có body 2026-09-14 | `FUN_0077250c(+0xb14[slot≤50], n)` (`007700bc…c:63`) |
| `0x0C` | `[1E][0C][12B record]` | `func_0x0077b72c` | `*gvar_007DA7C0` (không dùng) | ✅ Có body 2026-09-14 | single ghi `**player+0xdc0[slot≤25]` VMT+4 (d.124–128) |
| `0x0D` | `[1E][0D][slot:1B][n:1B]` (4B) | `func_0x0077b938` | `*gvar_007DA7C0` (không dùng) | ✅ Có body 2026-09-14 | `FUN_0077250c(+0xdc0[slot≤25], n)` (`0077b938…c:63`) |

~~Đã kiểm `index.csv` (6302 dòng): trong 11 handler con chỉ `0076fdb8` có mặt.~~ → **Cập nhật 2026-09-14**: cả 11 handler con đều có entry + `.c` + `.asm.txt` trong `ts_decompile/`. Pattern truyền nguyên `RP` (gồm cả byte SubOp) cho handler tự cắt tiếp — giống Case 25/27 — **được xác nhận bởi cả 10 body mới**.

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1–4.5. SubOp `0x01–0x05` — Nhóm A (ĐÃ PHỤC HỒI — cơ chế bảng slot)

**Khuôn record 12 byte** (áp dụng cho `0x01` batch, `0x04` single) — đọc từ `007707a0_FUN_007707a0.c` / `007705d8_FUN_007705d8.c`:
```
[slot : 1B]                     // chỉ số ≤ 0x32 (50), BoundErr nếu vượt
[code : Word LE]                // Copy(RP,3,2) → FUN_0077eb9c
[b1 : 1B][b2 : 1B][b3 : 1B][b4 : 1B][b5 : 1B]   // RP[4..8]
[q : 4B LE]                     // Copy(RP,10,4) → FUN_0077ed68 (signed-int32 LE)
color := FUN_0065b580(*gvar_007DA660, code, 0, q)  // byte màu/flags tra từ code+q, clamp ≤0xFF
→ gọi method VMT+4 của slot object  *(**gvar_007DA7BC + 0xa48 + slot*4)  với (code, b1, q, color, b5, b4, b3, b2)
```
- `0x01` = chạy **n = (Len(RP)-1) div 12** record liên tiếp (`007707a0…c:68–176`, stride `local_24 += 0xc` d.176); `0x04` = đúng 1 record (`007705d8…c:70–128`). Kết quả method được so với `b1` rồi **gán vào biến cục bộ không dùng** — artefact, không có hành vi phụ.
- `0x02` (`00770454…c:54–99`) và `0x0A` (`007702d0…c:54–99`): **cùng khuôn 11B** (Word code + 5 byte + q 4B, không có slot) nhưng **không đụng mảng slot nào** — chuyển toàn bộ sang `FUN_0072be9c(**player, …)` / `FUN_00749c78(**player, …)` với cùng cặp `(color)` tính qua `FUN_0065b580`; khác biệt giữa 2 SubOp chỉ là hàm đích. Bên trong `0072be9c/00749c78` chưa mổ — chưa kết luận được field ghi.
- `0x03` (`00770170…c:51–57`): `[code:Word LE][b:1B]` → `FUN_0072c038(**player, w, b)`.
- `0x05` (`0077021c…c:47–64`): `[slot≤50][n]` → `FUN_0077250c(**player+0xa48+slot*4, n)`: **TRỪ n khỏi byte đếm `slotObj+7`**; nếu kết quả ≤0 → set 0; khi counter về 0 → gọi `VMT+8` của slot (handler "hết hàng"). Kết quả trả về được so với `n` vào biến không dùng.
- **Đính chính**: các hàm này **không dùng `param_1`** — mọi thao tác là trên player toàn cục `**gvar_007DA7BC` + bảng `*gvar_007DA660`.

### 4.6. SubOp `0x06, 0x07` — Reserved

- Không có case → bỏ qua hoàn toàn. Server không nên gửi; đây là 2 slot trống duy nhất trong dải 1–13.

### 4.7. SubOp `0x08` — Trừ 50 khỏi mọi slot mảng `+0xa48` (NGUỒN ĐÃ XÁC MINH + body `FUN_0077250c` mới)

- **Wire**: `[1E][08]` — 2 bytes, không params. Signature `void FUN_0076fdb8(void)` (`0076fdb8_FUN_0076fdb8.c:19`), loop d.31–33.
- ```c
  for (i = 1; i != 0x33; i++)   // 1..50
    FUN_0077250c(**(gvar_007DA7BC) + 0xA48 + i*4, 0x32 /*=50*/);
  ```
- **Đính chính "reset/clear"**: `FUN_0077250c` (`0077250c_FUN_0077250c.c` — body mới) là **phép TRỪ counter `slotObj+7 -= 50`** (clamp 0, gọi `VMT+8` khi về 0) ⇒ SubOp 0x08 **không phải memset**: slot có đếm >50 sẽ còn dư. "Clear khi đổi map/logout" vẫn là suy luận — chưa kết luận được, nhưng cơ chế đã rõ là *mass-decrement*.

### 4.8–4.10. SubOp `0x09, 0x0A, 0x0B` — Nhóm A phần hai (ĐÃ PHỤC HỒI)

- `0x09` (`007714a0…c:68–176`): **bản sao chính xác của `0x01`** nhưng mảng đích là `**player + 0xb14` (d.172) — batch 12B record. ⇒ "handler nặng ở xa cụm" chỉ là vị trí code; logic nhẹ như 0x01.
- `0x0A`: xem 4.1–4.5 (`FUN_00749c78`).
- `0x0B` (`007700bc…c:47–64`): `[slot≤50][n]` → `FUN_0077250c(**player+0xb14+slot*4, n)` — decrement như 0x05 nhưng trên mảng thứ hai.

### 4.11–4.12. SubOp `0x0C, 0x0D` — Nhóm B (ĐÃ PHỤC HỒI)

- `0x0C` (`0077b72c…c:70–128`): single 12B record, giống khuôn 0x04 nhưng mảng `**player + 0xdc0` với **bound 0x19 (≤25)** (d.124–127).
- `0x0D` (`0077b938…c:47–64`): `[slot≤25][n]` → `FUN_0077250c(**player+0xdc0+slot*4, n)`.
- **Đính chính**: không "riêng module" — vẫn player record, chỉ khác mảng slot. Ghi chú cũ "cùng họ với `func_0x0077bd2c` ở OP 0x1F SubOp 0x0E": `0077bd2c` đúng là cùng họ địa chỉ, nhưng đối chiếu header callers (`0077bd2c_FUN_0077bd2c.c:7–8`) thì **chỉ case_027 gọi nó** — không phải handler của OP 0x1E (giữ nhất quán với `opcode_1f.md`).

---

## 5. Chuỗi VISCII → UTF-8

- **Không có bằng chứng VISCII trong toàn bộ OP 0x1E (2026-09-14: mức CAO hơn — đã kiểm cả 11 body)**: dispatcher không literal; cả 10 handler con mới phục hồi cũng **không có tham chiếu `DAT_/UNK_/LAB_`** nào ngoài header, không `FUN_007b372c` (set caption), không `_LStrCat`, không Toast `(+0x90, …)`; toàn bộ là số học + con trỏ slot. String chỉ có thể xuất hiện qua các hàm sâu chưa mổ (`FUN_0065b580`, `FUN_0072be9c`, `FUN_0072c038`, `FUN_00749c78`, method VMT+4/+8 của object slot) — ngoài phạm vi kết luận của file này.
- ~~Nếu params SubOp 1–5/9–13 chứa tên item/thông báo Việt thì nằm trong 10 handler chưa decompile → cần decompile bổ sung…~~ → đã decompile đủ 10 handler: **params hoàn toàn là số fixed-width, không có trường chuỗi nào trên dây 0x1E**.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077F414_FUN_0077F414.c:956-957`: `case 0x1e: break;` — rỗng hoàn toàn, không build frame.
- Kết luận: OP 0x1E là S→C một chiều. Mock chỉ cần craft S→C `[TOKEN][LEN][1E][SubOp][params]` (XOR `0xAD`).

---

## 7. Ghi chú cho Mock Server / Fuzzer

```
S→C: [TK0][TK1][L0][L1][0x1E][SubOp][params...]
  SubOp ∈ {01,02,03,04,05,08,09,0A,0B,0C,0D}
  08: params rỗng (chắc chắn) — an toàn replay (nhớ: TRỪ 50 khỏi 50 slot +0xa48, có thể kéo counter về 0 → trigger VMT+8)
  (quy ước độ dài tính trên RP = SubOp + params, không tính byte 0x1E)
  01: RP = 1 + 12*n byte; 09: như 01 nhưng đánh vào +0xb14
  04/0C: RP = 13 byte (1 SubOp + 12B record)
  02/0A: RP = 12 byte; 03: RP = 4 byte; 05/0B/0D: RP = 3 byte
  slot byte phải ≤ 50 (0x01,04,05,08,09,0B — mảng +0xa48/+0xb14) hoặc ≤ 25 (0x0C,0D — mảng +0xdc0)
    → vượt bound: _BoundErr → RangeError trong handler (client có thể crash/kêu)
  06,07,giá trị khác: client ignore
C→S: không tồn tại
```

~~Việc còn lại để full spec: decompile 10 hàm `007700bc … 0x0077b938`, định danh `gvar_007DA58C` / `gvar_007DA7C0`.~~ → **Hoàn tất 2026-09-14 (trừ phần không còn cần thiết)**: 10 hàm đã có body ⇒ layout params đã full-spec như trên. `gvar_007DA58C/007DA7C0` **không còn là blocker** — các callee bỏ qua chúng; các việc còn lại thật sự: (a) định danh 3 mảng slot `player+0xa48/+0xb14/+0xdc0` (buff? item-page? hotkey?) qua creator của slot object + method `VMT+4/VMT+8` (hiện là con trỏ trong header của chính các slot — chưa tìm ra file đích), (b) mổ `FUN_0065b580` (bảng `*gvar_007DA660` = color/quality DB?), `FUN_0072be9c / FUN_0072c038 / FUN_00749c78`.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_026_007921A6_FUN_007921a6.c:26-60` | Toàn bộ switch SubOp |
| 2 | `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_0x78A9B6_case_functions.csv:26` | Mapping |
| 3 | `functions/0076fdb8_FUN_0076fdb8.c:19,31–33` | SubOp 8: loop trừ 50 khỏi 50 slot `+0xa48` |
| 4 | `functions/0077F414_FUN_0077F414.c:956-957` | C→S rỗng |
| 5 | `functions/0077eb9c / 0077ef7c / 0077eb1c / 0077ee84 / 0077f098` | Codec nền (xác nhận loại trừ ở tầng dispatcher) |
| 6 | ~~`index.csv` (6302 dòng) — 10 handler con vắng mặt~~ | **CẬP NHẬT 2026-09-14**: cả 10 handler đã có file — dùng ở dòng 7–10 |
| 7 | `functions/007707a0 / 007714a0` (.c) | d.68–176: batch 12B record → mảng `+0xa48` / `+0xb14` |
| 8 | `functions/007705d8 / 0077b72c` (.c) | d.70–128: single 12B record → `+0xa48` (bound 50) / `+0xdc0` (bound 25) |
| 9 | `functions/0077021c / 007700bc / 0077b938` (.c) | d.47–64: `[slot][n]` decrement qua `FUN_0077250c` trên `+0xa48/+0xb14/+0xdc0` |
| 10 | `functions/00770454 / 007702d0 / 00770170` (.c) | d.54–99 / d.54–99 / d.51–57: delegations `FUN_0072be9c / FUN_00749c78 / FUN_0072c038` |
| 11 | `functions/0077250c_FUN_0077250c.c` (thân d.22–44) | **Body mới**: `slotObj+7 -= n`, clamp 0, `VMT+8` khi về 0 ⇒ đính chính SubOp 0x08 là decrement, không phải memset |
| 12 | `functions/0077ed68_FUN_0077ed68.c` | Decoder 4B (ký) dùng cho trường `q` |
