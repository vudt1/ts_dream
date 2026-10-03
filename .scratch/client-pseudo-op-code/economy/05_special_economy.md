# 05 — Kinh tế đặc biệt (Special Economy) — vòng quay, Trân Bảo Các, bank, store

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

Ngày: 2026-10-01 · Nguồn chính: `test/ts_decompile/functions/` (6588 file `.c` / 13176 fn), `ts_dream/client_pseudo_c/`
Trạng thái: **HOÀN TẤT v1** — mỗi claim layout đều gắn `file:line`; suy luận đánh dấu **Confidence C**;
13 câu hỏi còn mở nêu ở §6.

> Quy ước: `P[i]` = byte thứ i của payload sau header (`P[0]` = MainOp). `RP[i]` = byte thứ i của
> RestPayload = `P[i+1]`. `_LStrCopy(s, idx, cnt)` là **1-based** (`RP[idx-1 .. idx-1+cnt-1]`).
> Codec: `FUN_0077ef7c` = read u32 LE · `FUN_0077eb9c` = read u16 LE · `FUN_0077ed68` = read u32 LE ·
> `FUN_0077f098` = read double LE · `FUN_0077ee84` = write string · `FUN_0077eb1c` = write u16 LE ·
> `FUN_0077ea74` = write u8.
> Global quan trọng: `gvar_007DA7BC` = con trỏ tới record người chơi (`**player`).
> `gvar_007DA084` = `TSe_TalkMsgFormPlus` (banner), `gvar_007DA1B0` = `TTalkMsgForm` (chat log).

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — U9 ĐÃ ĐÓNG, U3/U4/U5/U14 đã có body**

---

## 0.1 🔴 ROUND-4 REDUMP

### 0.1.1 ✅ U9 đóng — bảng 9 nhánh C→S `0x1E`

> §493 (U9): *"Body 9 nhánh của **C→S `0x1E`** — Jump table ở `0x00786221` bị Ghidra coi là
> **data**, không có `.asm.txt` phía sau → Dredump `0x00786221–0x00786583`"*

**ĐÃ ĐÓNG.** `asm_cs1e_786221_builders.asm.txt` (**866 B / 866 B — 100%**) +
`cs1e_786221_jmptbl_9xDWORD.hex`.

| Entry | Builder | Nghĩa |
|:-:|:-:|---|
| `[0]` | `0x0078A4F2` epilogue | **KHÔNG GỬI** |
| `[1]` | `0x00786245` | gửi |
| `[2]` | `0x007862B3` | gửi |
| `[3]` | `0x00786321` | gửi |
| `[4]` | `0x00786376` | gửi |
| `[5]` | `0x007863E4` | gửi |
| `[6]` | `0x00786452` | gửi |
| `[7]` | `0x007864C0` | gửi |

⇒ **`0x1E` C→S có đúng 8 nhánh thật** (sub `1..8`); sub `0` là no-op.
Điều này **xác nhận** claim C của tài liệu rằng `case 0x1E:` rỗng trong Ghidra là
recovery artifact, không phải bằng chứng client không gửi.

⚠️ Bảng 9 entry cần 36 B; cửa sổ hex `0x786221..0x786242` chỉ đọc được 34 B
(cắt 2 byte) — nhưng vì `[0]` là epilogue nên **8 nhánh là đủ**.

### 0.1.2 ✅ U3/U4/U5/U14 đóng — 3 handler stall nay có body

> §111-115: *"`17 1F` → `func_0x007311c8` — **không có body**"*
> *"`17 21` → `func_0x00610290` — **không có body**"*
> *"`17 22` → `func_0x0060e3cc` — **không có body**"*

**ĐÃ ĐÓNG** (Round-4 tạo function trong các HOLE):

| Handler | Frame | File |
|---|---|---|
| `FUN_007311C8` | `17 1F` | `007311c8_FUN_007311c8.{c,asm.txt}` |
| `FUN_00610290` | `17 21` | `00610290_FUN_00610290.{c,asm.txt}` |
| `FUN_0060E3CC` | `17 22` | `0060e3cc_FUN_0060e3cc.{c,asm.txt}` |

Quan sát đầu tiên:

- `0x00610290`: `_LStrCopy(_, 2, 4, _)` → đọc **4 byte tại `RP[2..5]`**
- `0x0060E3CC`: bound check `MOV EAX,0x2` → đọc `RP[2]`

⇒ **Đọc trực tiếp 3 file này để chốt layout byte.** Đây là việc đọc, không cần redump.

### 0.1.3 ✅ U11 đóng — `TFRollEgg` caller đã có

> §485 (U1): *"`TFRollEgg.Create` (`0x00772924`) chỉ được gọi từ `0x00772788`, thuộc hàm
> **Ghidra không tạo function**"*

**ĐÃ ĐÓNG.** `SCAN 0x007726F0 0x00772924` đã tạo `FUN_00772788` +
`FUN_007726F0` (`.c` + `.asm.txt` tồn tại).

### 0.1.4 ❌ U2 — `0x9D1610` / `0x9D1620` **KHÔNG ĐỌC ĐƯỢC**

`rg_wager_9d1610.hex` và `rg_wager_9d1620.hex` đều trả **`readable 0`**.
Địa chỉ nằm **ngoài vùng memory đã map** (biên `0x7DA800`) — xem
`light_dark_skill_unlock_gap.md §0.1.3`. Cần sửa Memory Map của project Ghidra.

### 0.1.5 ⚠️ U12 — `TLottoManager` (`VMT_54D580`) đã có dữ liệu

`SCAN 0x0054D580 0x0054D700` + `VMT 0x0054D580 48` đã chạy; đọc
`ts_decompile/redump/round3_report.txt` (mục `VMT` / `SCAN`) để lấy tên slot.

### 0.1.6 ⚪ Vẫn cần traffic thật

| # | Câu hỏi | Vì sao |
|:-:|---|---|
| §484 (G2) | `17 20` có thật sự 1 byte pad không | cần packet capture |
| §485 (G3) | sub-op C→S thật của stall open/close/price | cần log thật |
| §486 (G4) | độ dài thật của prefix `17 21` | cần capture |
| §495 (U11) | tên nghiệp vụ tiền #2 (`+0x12FC`) | cần runtime / `.FLST` |

## 0. Bản đồ MainOp → handler S→C (tự dựng lại, ĐÃ ĐỐI CHIẾU 5 neo)

Parse: `client_pseudo_c/jumptable_byte200_0x78A8EE.hex` (200 byte, 16/line, index = MainOp) →
`jumptable_dword200_0x78A9B6.hex` (200 dword LE) → `jumptable_0x78A9B6_case_functions.csv`.

| MainOp | case | handler | MainOp | case | handler |
|---|---|---|---|---|---|
| `0x17` | 20 | `0x007902FB` ✅* | `0x1E` | 26 | `0x007921A6` ✅* |
| `0x19` | 22 | `0x0079157C` ✅* | `0x1F` | 27 | `0x007922E6` |
| `0x1A` | 23 | `0x0079175F` ✅* | `0x20` | 28 | `0x0079285A` |
| `0x1B` | 24 | `0x00791D80` ✅* | `0x21` | 29 | `0x007928E4` |
| `0x1E` | 26 | `0x007921A6` ✅* | `0x22` | 30 | `0x00792ACC` |
| `0x23` | 31 | `0x00792BAD` | `0x39` | 50 | `0x00795579` ← **TSportManage** |
| `0x0F` | 15 | `0x0078E367` ← **TWF_CartManage** | `0x10` | 16 | `0x0078E593` ← **TWF_Day55QuestManage** |
| `0xC7` | 65 | `0x007962FC` | `0x0A/0x11/0x12/0x15/0x1C` | — | default `0x00796347` |

`*` = 5 neo đã cho đều khớp ⇒ **bảng parse bảo đảm đúng**.

**MainOp không có handler (byte_table = 0)** → rơi vào `caseD_0 @ 0x00796347`:
`0x0A, 0x11, 0x12, 0x15, 0x1C, 0x2F, 0x30, 0x31, 0x33, 0x34, 0x35, 0x38, 0x3C, 0x3E, 0x40, 0x44,
0x49…0xC6`. Riêng `0x1C` (skill) là **inactive** — xác nhận chéo với `opcode_1a.md`.

### 0b. Bản đồ MainOp → builder C→S (tự dựng lại)

Parse: `cs_dispatch_byte200_0x77f474.hex` (index = MainOp) → `table_0x77F53C_dword200.hex` → địa chỉ builder
(đã khớp 100% với `cs_builder_dword200_0x77f53c.hex`). Builder nằm trong `FUN_0077f414` ("SendCommand"),
asm đầy đủ ở `functions/0077f414_FUN_0077f414.asm.txt`.

| MainOp | builder | MainOp | builder |
|---|---|---|---|
| `0x22` | `0x00786C1C` | `0x23` | **`0x00786CD6`** ← account (đổi MK / xoá role / nạp thẻ) |
| `0x24` | `0x007871AF` | `0x25` | `0x007872D7` |

---

## 1. Bản đồ subsystem

| Subsystem | Client class | VMT / entry | Global singleton | Trigger (S→C) | C→S trigger |
|---|---|---|---|---|---|
| **"Vòng quay"** | `TFRollEgg` | Create `0x00772924`, Load `0x0077299C`, Decode `0x00772B94` | *(không có global — đi qua VMT)* | **chưa xác định** | **chưa xác định** |
| **Trân Bảo Các** | `TFBlissBag` | Create `0x004DEF58`, Load `0x004DF424`, Post `0x004DEF9C` | `DAT_009264D4` | **chưa xác định** (không case nào ref global) | **chưa xác định** |
| **Thủy binh Bank** | `TWaterWarBankManage` / `TWaterWarBankThing` | Manage.Create `0x0077B8F4`, Thing.Create `0x007721BC`, Thing.Destroy `0x00772204` | `gvar_007DA7C0`; slot array `player+0xDC0` (26) | **`0x1E` sub `0x0C`/`0x0D`** | không (S→C only) |
| **Bank / kho (2 bảng)** | `TBankThing` (ctor gọi tại `0x007726F0`) | `TBankThingManage.Create` → `gvar_007DA58C` | slot arrays `player+0xA48` (51), `+0xB14` (51) | **`0x1E` sub `0x01..0x0B`** | không (S→C only) |
| **TSportManage** (xe/sport) | `TSportManage` | Create `0x005534E4` | `gvar_007D9D88` | **`0x39` (`case 50`, `FUN_00795579`)** | xem §2.5 |
| **TSe_CashGiftForm** (quà tiền mặt) | `TSe_CashGiftForm` | ctor `0x0057410C`, VMT `0x00571BA0`, method `0x0057479C` | xem §2.6 | xem §2.6 | xem §2.6 |
| **Account store / MK** | `TSe_ChangePassWordForm`, `TSe_DeleteRoleForm`, `TSe_ReFillForm` | ctors `0x005AE974` / `0x005AE01C` / `0x005AF5E8` | `gvar_007DA4C8` / `gvar_007DA684` / `gvar_007D9CC0` | **không có** (`23` chỉ banner) | **`0x23` sub `01/02/03/05`** |
| **"God/HP/SP store"** (Rust) | — | — | — | `23 04` — **SAI NGỮ NGHĨA** | — |
| `TFWheel` | `TFWheel` | Create `0x0067AE64`, VMT `0x0065D32C` | — | **không phải economy** | — |

---

## 2. Từng subsystem

### 2.1 `TFWheel` — **ĐÍNH CHÍNH: KHÔNG phải vòng quay casino** ⚠

Hypothesis ban đầu ("`TFWheel` = vòng quay") **SAI**. `TFWheel` là **hiệu ứng skill trong trận đánh**:

- `functions/0067ae64_TFWheel.Create.c:38-41` — ctor gọi `TFallWood_Create(param_1,'\0',…)` rồi
  `FUN_007c9b38(*gvar_007D9ED8, "Shadow")` ⇒ là sprite effect (khối gỗ/rơi + shadow), không có UI.
- `functions/00666bf0_FUN_00666bf0.c` — factory tạo hiệu ứng skill theo effect-id; `case 0x5b:` →
  `TFWheel_Create((int*)VMT_65D32C_TFWheel, '\x01', …)` và gán vào `player+0x3C` (con trỏ effect).
- Cùng factory có `case 0x50` → `VMT_65CE2C_TFireWheel` cũng gọi `TFallWood_Create` ⇒ **hai lớp
  effect "bánh xe lửa"** của skill, không liên quan tiền.

**Kết luận:** vòng quay casino là `TFRollEgg` + `Data/RG.Dat` (§2.2), **không phải** `TFWheel`.

### 2.2 `TFRollEgg` — vòng quay (RG.Dat)

| Hạng mục | Bằng chứng |
|---|---|
| **Trigger (mở form)** | **Chưa xác định.** `TFRollEgg.Create` (`0x00772924`) chỉ được gọi từ `0x00772788`, thuộc một hàm **Ghidra không tạo function** ⇒ không có `.c`/`.asm.txt` để đọc. Class không có global singleton ⇒ không xuất hiện trong bất kỳ `case_0NN_*.c` nào (đã grep `DAT_009264d4`-style toàn bộ `case_functions/functions/`). |
| **Bảng giá/thưởng** | **CLIENT-SIDE — `\Data\RG.Dat`.** `functions/0077299c_FUN_0077299c.c:67,76` `_LStrCat3(&local_164, local_c, "\Data\RG.Dat")`; `:83-85` nếu `FileExists()==0` → `ShowMessage(DAT_00772b68)`. |
| **Header file** | `0077299c…c:87` `FUN_0040673a(local_160, path)` (open) → `:88` `FUN_00406cf9(local_160, 7)` → `:89-93` `FUN_00402864()` (read) trừ 1 → **`*(short*)(this+0x1B0) = (short)(readU16() - 1)`** ⇒ `this+0x1B0` = số segment N (1..60). |
| **Thân bảng** | `0077299c…c:95-101` đọc N bản ghi vào `this+4` (`FUN_004067dc(file, this+4, uVar3, 0)`), rồi `FUN_00772b94(this)` giải mã. |
| **Layout 1 bản ghi = 7 byte** (`FUN_00772b94`, `functions/00772b94_FUN_00772b94.c:24-197`) | `off+0` : **u8 tier**, XOR `0xC8` (200) — bound `≤ 3` ⇒ **4 tier** (`:157 if (3 < uVar1) _BoundErr`) · `off+1` : **u16 LE**, XOR `0x5209` · `off+3` : **u16 LE**, XOR `0x5209` · `off+5` : **u16 LE**, XOR `0x5209` (= **trọng số / xác suất**) |
| **Giới hạn** | `if (0x3c < idx) _BoundErr` ⇒ **tối đa 60 ô vòng quay** (`00772b94…c:47,63,88,…`). |
| **Tổng hợp tier** | `00772b94…c:22` `_FillChar(&DAT_009D1610, 0x10, 0)`; vòng lặp tính `DAT_009D1610[tier] = max(DAT_009D1610[tier], rec.off5)` ⇒ **mảng global 4×u32 = trọng số lớn nhất mỗi tier**. |
| **UI / animation** | `FUN_00772DE4` (`functions/00772de4_FUN_00772de4.c:22`) chỉ là **tick counter** `_DAT_009D1620++`. `FUN_00772F14` (`00772f14…c:21-38`) là **timer `Now()`-based**: `(Now() - *(double*)(this+0x48)) * _DAT_00772F6C` → `_ROUND()`, so sánh ngưỡng `param_2`, lưu lại `this+0x48` ⇒ **timer quay vòng quay** (spin duration). |
| **Frame S→C kết quả** | **Chưa xác định** (xem §6). |
| **Currency** | **Chưa xác định** (xem §6). |

> **Confidence A** cho layout file + XOR; **Confidence C** cho "tier = 4 nhóm giải" (chỉ có bound ≤3, chưa thấy bảng tên).

### 2.3 `TFBlissBag` — Trân Bảo Các (`BlissBag.Dat`)

| Hạng mục | Bằng chứng |
|---|---|
| **Khởi tạo** | `functions/0050a4a0_TForm1.FormCreate.c:593-594` — `DAT_009264D4 = TFBlissBag_Create((int*)VMT_4DEF00_TFBlissBag,'\x01',…)` rồi `FUN_004DF424(DAT_009264D4)`. |
| **Bảng giá/thưởng** | **CLIENT-SIDE — `\Data\BlissBag.Dat`.** `functions/004df424_FUN_004df424.c:68,77` `_LStrCat3(…,"\Data\BlissBag.Dat")`; `:81-82` `ShowMessage(DAT_004df634)` nếu thiếu file. |
| **Layout bộ nhớ** | `004df424…c:88` `_FillChar(this+4, 0x173D6, 0)` ⇒ **bảng phẳng 95 190 byte** tại `this+4` · `:89` `FUN_00406cf9(f, 0xBE)` ⇒ **record size = 190 (0xBE) byte** · `:94-99` `*(short*)(this+0x173DA) = (short)(readU16() - 1)` ⇒ **số record** · `:100-107` kiểm tra `count*0xBE ≤ 0x173D6` (⇒ **max 950 record**) |
| **Post-process** | `004df424…c:110` `FUN_004DEF9C(this)` — giải mã XOR (giống kiểu `FUN_00772B94`). |
| **Rust tương đương** | `src/data/loaders/bliss_bag.rs:39-88` — đã implement đúng: record `[bagid u16][nb_items u8][nb_getitems u8]` + `nb_items × [item u16][qty u8][odds u16][nb_get u8]`; giải mã `decode_u16/decode_u8` (XOR). **Khớp 190 B/record** (4 + N×6). |
| **Frame S→C kết quả** | **Chưa xác định** — không `case_0NN_*.c` nào tham chiếu `DAT_009264D4` (grep toàn bộ `case_functions/functions/` → 0 hit). |
| **Currency** | **Chưa xác định** (xem §6). |

> **Confidence A** cho layout file + khớp loader Rust; **Confidence C** cho "record chứa odds ⇒ client roll" (chưa thấy hàm roll).

### 2.4 `TWaterWarBankManage` / Thủy binh Bank — **ĐÃ XÁC MINH ĐẦY ĐỦ**

| Hạng mục | Bằng chứng |
|---|---|
| **Dispatcher** | `case_functions/functions/case_026_007921A6_FUN_007921a6.c:46-52` → sub `0x0C`/`0x0D` gọi `func_0x0077B72C` / `func_0x0077B938` với `*gvar_007DA7C0` (= `TWaterWarBankManage`). |
| **Slot array** | `player+0xDC0`, **26 phần tử** kiểu `TWaterWarBankThing` — `functions/00716370_TPlayer.Create.c:179-188` (`TWaterWarBankThing_Create(VMT_7672D8,…)` × 0..0x19, `param_1[i+0x370]` ⇒ byte `+0xDC0 + i*4`). |
| **`1E 0C` — ghi 1 ô** | `functions/0077b72c_FUN_0077b72c.c:70-128`. Layout: `[1E][0C][slot u8=RP[1]][item u16 LE=RP[2..3]][flag char=RP[4]][a=RP[5]][b=RP[6]][c=RP[7]][d=RP[8]][serial u32 LE=RP[9..12]]`. `slot` bound **≤ 0x19 (25)** (`:122-124`). `item` đi qua `FUN_0065B580(*gvar_007DA660, item, 0, serial)` (`:107`) để tô màu → `local_14` (byte). Rồi `(**(code**)(*slotObj+4))(slotObj, item, flag, serial, color, d, c, b, a)` (`:127-129`). |
| **`1E 0D` — trừ số lượng** | `functions/0077b938_FUN_0077b938.c:47-63`. Layout `[1E][0D][slot u8][n u8]` (4 byte). Gọi `FUN_0077250C(player+0xDC0+slot*4, n)`. |
| **`FUN_0077250C` = Take(n)** | `functions/0077250c_FUN_0077250c.c:25-45`: `q = *(u8*)(this+7) - n`; nếu `q ≤ 0` ⇒ set `this+7 = 0`; ngược lại trừ; **trả về số lượng thực lấy**; khi `this+7 == 0` ⇒ gọi `VMT+8` (giải phóng ô). ⇒ **`slotObj+7` = số lượng (u8) của một stack trong ô Thủy binh bank.** |
| **Cấu trúc `TWaterWarBankThing`** | Create `0x007721BC` → `TBankThing_Create @ 0x007726F0` (file `.c` cho 0x007726F0 **không có** trong dump ⇒ layout chi tiết offset +0..+7 là **Undetermined**). Destroy `0x00772204`. |
| **Currency** | **KHÔNG có** — bank chỉ chứa **vật phẩm**, không đổi tiền. Frame `1E 0C/0D` không mang field tiền nào. |
| **C→S** | **Không có.** `case_functions/functions/case_026…` không đọc ngược; và `FUN_0077f414` (SendCommand) `case 0x1E:` (`functions/0077f414_FUN_0077f414.c:1000`) là `break;` **rỗng**. *(Về mặt pháp lý: đây là `break` trong pseudo-C ⇒ Confidence C cho "client không gửi 0x1E"; cần ASM để khẳng định tuyệt đối — xem §6.)* |
| **Rust hiện tại** | `src/server/handlers/` **không có** handler `0x1E` ⇒ gap. |

### 2.4b Ba bảng slot của `player` (bối cảnh cho Trân Bảo Các)

Từ `functions/00716370_TPlayer.Create.c` (`param_1` là **con trỏ dword**, nên byte offset = ×4):

| Byte offset | Số slot | Kiểu slot | Hàm ctor | Ghi bởi sub-op của `0x1E` |
|---|---|---|---|---|
| `+0xA00` | 26 | `TThing` (`VMT_767344`) | `:100-107` | — (không có sub nào đụng) |
| `+0xA48` | **51** | `TBankThing` | `:112-119` | `0x01` batch, `0x04` single, `0x05` giảm, `0x08` clear |
| `+0xB14` | **51** | `TBankThing` | `:120-127` | `0x09` batch, `0x0B` giảm |
| `+0xEA4` | 26 | `TThing` | `:132-139` | — |
| `+0xF0C` | 26 | `TThing` | `:144-151` | — |
| `+0xF74` | 11 | `TThing` | `:155-162` | — |
| `+0xFA0` | 11 | `TThing` | `:166-173` | — |
| `+0xDC0` | **26** | `TWaterWarBankThing` | `:177-188` | `0x0C` single, `0x0D` giảm |

Ghi chú thêm: `TPlayer.Create.c:190-193` — `param_1[0x51B]` (byte `+0x146C`) = 0;
`:194-195` `_FillChar(this+0x12D4, 13, 0)`; `:195` `_FillChar(this+0x5204, 0x6C, 0)`.

### 2.5 `TSportManage` — manager popup **cờ bạc** (đã định danh đủ 6 panel)

**Khởi tạo**: `functions/0050a4a0_TForm1.FormCreate.c:629-630` —
`*gvar_007D9D88 = TSportManage_Create((int*)VMT_5532F4_TSportManage,'\x01',…)`. **Không có** lệnh
load `.Dat` tương ứng (đối chiếu `0050a4a0…c:583-681`: trong 33 manager/form được tạo ở đó, chỉ
`TFMark/WeapBody/Suit/Rank/BlissBag/SceneSet/GS/Scene/TsoCn/TsoTw/Warp/Compound/Talk/Npc/Formula/
Skill2/Skill/Item/Festival` có loader `.Dat` ⇒ `TSportManage` **không** có bảng client-side).
`TSportManage` **không phải 1 form** mà là **manager 6 panel giải trí**.

**Trigger mở/đóng panel** — MainOp `0x39` (`case 050`, `FUN_00795579`), xem `opcode_39.md` §4:

| SubOp | Payload | Hành động | Bằng chứng |
|---|---|---|---|
| `0x01` | `[39][01][formId u8][opt u8?]` | mở panel theo id; nếu `id==3` và `RP[3]`=1/2 thì `form3+0x38 = 100/1000` | `case_050_00795579_FUN_00795579.c:38-57` |
| `0x02` | `[39][02]` | Free panel hiện tại, `mgr+4 = 0` | `case_050…c:62` → `FUN_00553410` |
| `0x03` | `[39][03][variant u8]` | banner 1200 ms: `1`→`UNK_00799200`, `2`→`UNK_00799224` | `case_050…c:70-76` |

**Bảng `formId → class`** (`FUN_0055374C`, tạo form; `FUN_00553410`, giải phóng — đối xứng 6/6):

| id | class | global | MainOp cập nhật | C→S sender |
|---|---|---|---|---|
| `1` | `TRE_BiDaXiao` (比大小 – Tài/Xỉu) | `gvar_007DA42C` | **`0x3A`** (`case_051`, `0x007956B9`) | `FUN_00547FBC` → `[3A][…]` |
| `2` | `TSBDManager` (bàn Domino) | `gvar_007DA0F4` | **`0x3B`** (`case_052`, `0x007956F3`) | `FUN_005524A0` → `3B 01 [u32 LE điểm]` |
| `3` | `TRE_ZMChessMain` (Cờ Tướng) | `gvar_007DA778` | **`0x3C`** (`case_053`, `0x0079575C`) | `FUN_00546798`, `FUN_005418F8` |
| `4` | **`TLottoManager`** (Lotto) | `gvar_007D9F98` | **`0x36` sub 1/2/3** (`case_054`, `0x007957DC`) | `FUN_0054E4F0` → `[3D][…]` |
| `6` | **`TMachineManager`** (máy đánh) | `gvar_007DA4EC` | **`0x36` sub 7/8/9** | — |
| `0xFF` | `TSportDemo` | `gvar_007DA0A0` | — | — |

**`0x36` — kênh Lotto + Machine** (`case_054_007957DC_FUN_007957dc.c:26-98`):

| SubOp | Payload | Tác dụng | Bằng chứng |
|---|---|---|---|
| `0x01` | `[36][01][m u8][k1][k2][k3][k4][k5]` | **Kết quả vòng rút số**: `mgr+0x68++` (lượt, bound `< 0x15` = 21), `mgr+0x69 = RP[1]`, ghi `mgr[(m+1)*5 + (i-1)] = RP[i+1]` cho i=1..5 | `FUN_0054E2F8`:24-77 |
| `0x02` | `[36][02][pot u32 LE]` | set **tổng giải/jackpot** vào `mgr+0x16C` | `FUN_0054F18C`:41-43 |
| `0x03` | `[36][03][state u8]` | `1`→banner `0x54F2EC` + `mgr+0x170=1`; `2`→banner `0x54F320`; `3`→reset `mgr+0x6F=0, mgr+0x68=0` + banner `0x54F340` | `FUN_0054F230`:25-38 |
| `0x05` | `[36][05][code u8]` | **9 banner lỗi** (2200 ms nếu code=1) | `case_054…c:53-95` |
| `0x07/08/09` | RP | `TMachineManager` (máy đánh) | `case_054…c:96-104` |

**Chuỗi VISCII đã giải mã** (từ `client_pseudo_c/lit_54F2EC/54F320/54F340.hex`; bảng map byte TSVN ở §3.1):

| Địa chỉ | Byte | Text |
|---|---|---|
| `DAT_0054F2EC` | `D0 E3 20 74 D5 6D 20 6E 67 DF 6E 67 20 F0 A3 74 20 63 DF FE 63 2C 20 78 69 6E 20 6D B6 69 6E 20 6C A5 6E 20 73 61 75 20 6E 68 61 21` | "Đã tạm ngưng đặt cược, xin mở lần sau nha!" |
| `DAT_0054F320` | `D0 A3 74 20 63 DF FE 63 20 74 68 A4 74 20 62 D5 69 20 72 B0 69 20 21` | "Đặt cược thắt bại rồi ạ!" |
| `DAT_0054F340` | `D0 A3 74 20 63 DF FE 63 20 74 68 E0 6E 68 20 63 F4 6E 67 20 63 68 FA 63 20 62 D5 6E 20 74 72 FA 6E 67 20 74 68 DF B7 6E 67 20 6C BE 6E 21` | "Đặt cược thành công chúc bạn trúng thưởng lớn!" |

**Currency**: mọi panel cờ bạc **đọc `player+0x12F8` (kim)** để chụp số dư lúc cược:
- `FUN_00547864:31` (BiDaXiao) → `*(u32*)(self+8) = player->0x12F8`
- `FUN_00548E98:21` (BiDaXiao tick) → `*(u32*)(self+8) = player->0x12F8`
- `FUN_00552E58:21` (Domino) → `*(u32*)(self+0x2C) = player->0x12F8`
- `FUN_00541F8C:65` + `FUN_00541098:385` (Cờ Tướng, mỗi ghế) → `self+0x84 + seat*4 = player->0x12F8`

⇒ Client **không tự trừ tiền**; server trừ rồi báo lại bằng `1A 02` (subtract) — xem §3.
⇒ **Confidence A** cho `TLottoManager` + `TMachineManager` là cờ bạc; **Confidence C** cho "Lotto = vòng quay số 5/20" (chỉ có bound 20×5, chưa thấy bảng tên).

### 2.6 `TSe_CashGiftForm` — form nhập quà tiền mặt

| Hạng mục | Bằng chứng |
|---|---|
| **Ctor** | `functions/0057410c_FUN_0057410c.c:67,69,71,73,75` — nạp sprite-set `"icon_Gift"`, `"icon_inputCashGift"`, `"icon_male"`, `"icon_female"`, `"icon_Resource1"`; layout `FUN_007B5CE4(self, 0x40, 0x46, 0x0E, 0x2C)`; tạo 2 `TSe_FixedButton` (`VMT_7AF174`). |
| **VMT / global** | VMT `0x00571BA0`; singleton `gvar_007DA260` — `functions/0051189c_FUN_0051189c.c:1732-1734`. |
| **Method `0x0057479C`** | `functions/0057479c_FUN_0057479c.c:60-99` — `TSe_Form::FUN_007B5DBC(self)` (refresh) rồi vẽ lại `icon_Resource1` + text ghép `local_8[0x59] & local_8[0x5A]` qua `FUN_007D1648`. ⇒ **thuần render/refresh, không gửi frame**. |
| **S→C frame** | **KHÔNG có** — grep `gvar_007DA260` trên toàn bộ `functions/*.c` chỉ ra đúng 1 hit (`0051189c…c:1732`). |
| **C→S frame** | **KHÔNG xác định** — không có builder nào tham chiếu form này (xem §6). |

### 2.7 `0x23` — **KHÔNG phải store**. Đính chính 3 giả định sai

#### 2.7a `23 04` mà Rust đang gửi cho "God/HP/SP store" — **SAI HOÀN TOÀN NGỮ NGHĨA** ⚠

Rust `src/server/spawn.rs:325-330`:
```rust
pub fn store_frame(point: u32) -> String {   // "23 04"
    let mut body = String::new();
    body.push_str(&encoder::le32(point));
    body.push_str(&"00".repeat(12));
    crate::protocol::frame("2304", &body)
}
```
`src/server/spawn.rs:583-585` gọi nó 3 lần (`store_frame(s.god)`, `store_frame(s.hp_store)`, `store_frame(s.sp_store)`), và `src/server/handlers/system.rs:56-63` (`handle_game_points`, opcode `0x22`) cũng gửi `store_frame(conn.session.gold)`.

**Client thực sự làm gì với `23 04`** — `case_functions/functions/case_031_00792BAD_FUN_00792bad.c:167-202`:

| Bước | Line | Hành động |
|---|---|---|
| 1 | `:168` | `_LStrCopy(RP, 2, 4, …)` ⇒ đọc **`P[2..5]` = A (u32 LE)** |
| 2 | `:170` | `_LStrCopy(RP, 6, 4, …)` ⇒ đọc **`P[6..9]` = B (u32 LE)** |
| 3 | `:174-176` | `IntToStr(A + B)` |
| 4 | `:177-178` | `_LStrCat3(msg, DAT_0079889C, IntToStr(A+B))` |
| 5 | `:179` | `_LStrCopy(RP, 10, 8, …)` ⇒ **`P[10..17]` = D (double LE)** |
| 6 | `:180-186` | nếu `D != 0.0` thì nối thêm `Format(D)` (2 chuỗi) |
| 7 | `:187-188` | **banner `TSe_TalkMsgFormPlus` 10 000 ms** với text dựng ở trên |
| 8 | `:189` | `FUN_007AB870(*gvar_007DA1B0, 0, msg, '\n')` ⇒ **ghi 1 dòng chat-log** |
| 9 | `:190-201` | `player->0x146C = (u32)(A*100 + B)` |

⇒ **`23 04` là "banner thông báo số lớn 10 giây + ghi log + set `player+0x146C`".** Không có
field tiền, không có field store, không có UI store nào được bật.
Rust gửi 3 frame này ⇒ **client sẽ hiện 3 banner 10 giây với số `point` (vì B=0, D=0.0) và
ghi đè `player+0x146C = point*100`**, tức **gây nhiễu UI nghiêm trọng** trên client aLogin thật.
**Confidence A.**

#### 2.7b `0x23` C→S = **đổi mật khẩu / xoá nhân vật / nạp thẻ** (KHÔNG phải store)

`FUN_0077F414` ("SendCommand") signature = `(conn, op_sub)` với `param_2 = (SubOp<<8) | MainOp`
(asm: `[EBP-0x5]` = MainOp, `[EBP-0x6]` = SubOp). Builder `0x00786CD6`:

| SubOp | Form | Payload C→S | Bằng chứng |
|---|---|---|---|
| `0x01` | **`TSe_ChangePassWordForm`** (`gvar_007DA4C8`, VMT `0x58A320`, ctor `0x005AE974`) | `[23][01][len(txt+0xF4)][len(txt+0xF8)][len(txt+0x100)][len(txt+0x104)]` | asm `00786CFA…00786E63`; 4 text field dùng `LEN` (`CALL 0x00404088`) |
| `0x02` | **`TSe_DeleteRoleForm`** (`gvar_007DA684`, VMT `0x58A258`, ctor `0x005AE01C`) | `[23][02][len(txt+0xF4)][len(txt+0xFC)]` | asm `00786E6D…00786F5F` |
| `0x03` | **`TSe_ReFillForm`** (`gvar_007D9CC0`, VMT `0x58A4D8`, ctor `0x005AF5E8`) | `[23][03][a][b][c][d][e][f]` — 6 byte ghép (text field `+0xF4/0xFC/…`) | asm `00786F64…00787094`, final `_LStrCatN(…,6)` |
| `0x05` | (chưa rõ form) | payload dài 6 | asm `00787099…` |

Định danh form: `functions/0051189c_FUN_0051189c.c:1181-1195`.
**Chỉ gửi ĐỘ DÀI chuỗi** (`_LStrLength @ 0x00404088`), **không gửi nội dung** ⇒ nội dung MK/role
được đọc ở phía server-side handler cũ / hoặc `TSe_Editor` tự mã hóa — **Confidence C** cho phần
"độ dài bao nhiêu là bao nhiêu".

> **Sửa định danh:** trong `opcode_23.md` nói "Chiều C→S `case 0x23: break;` rỗng" — đó là
> `case 0x23:` của **switch trong `FUN_0077F414`** tại `0077f414_FUN_0077f414.c:1010-1011`, thuộc
> **nhánh switch "pre-flight validation"** chứ không phải builder; builder thật nằm ở `0x00786CD6`
> theo bảng `0x77F474`/`0x77F53C` (`byte[0x23]=33 → dword[33]=0x00786CD6`, đã verify khớp 100%).

### 2.8 Cửa hàng NPC (shop) — bảng giá **client-side**, tiền = `+0x12F8`

Đây là nơi `+0x12F8` được dùng để **chặn mua hàng ở phía client** (3 hàm, đều không có caller
trực tiếp ⇒ được gọi qua VMT):

| Hàm | Form | Kiểm tra | Frame C→S | Bằng chứng |
|---|---|---|---|---|
| `FUN_006090A8` | form có **26 ô** `self+0x198 + i*4` (i=1..0x19), ô text `self+0x20C` | `if (player->0x12F8 < StrToInt(qty))` → banner `DAT_006091F8` (1200 ms); ngược lại gửi frame | **`0x19`** | `006090a8…c:49-70` |
| `FUN_0060B230` | form có ô text `self+0x1DC` | `if (player->0x12F8 < StrToInt(qty))` → banner `DAT_0060B710` | **`0x19`** | `0060b230…c:148-160` |
| `FUN_00611330` | form shop **10 tab**: `tab = self[0x15C]` (bound ≤ 9), **bảng giá `self+0x168 + tab*0x11`** (stride **17 B**), giá là u32 đầu record | `if (player->0x12F8 < self+0x168 + tab*0x11)` → banner `DAT_00611484`; nếu đủ thì `FUN_00768D38()` rồi gửi | **`0x17`** | `00611330…c:30-60` |
| `FUN_00632E3C` | form hiển thị | `IntToStr(player->0x12F8)` (d.58) và `IntToStr(player->0x1300)` (d.93) | — | `00632e3c…c:58,93` |
| `FUN_00593428` | form | `FUN_00593B54(self, 1, player->0x12F8)` | — | `00593428…c:54` |

⇒ **Bảng giá shop do SERVER PUSH** (frame nào điền `self+0x168 + tab*0x11` chưa xác định — §6),
nhưng **đơn vị tiền là `player+0x12F8` (kim)** và **client tự chặn trước khi gửi `0x17`/`0x19`**.
⇒ **Confidence A** cho currency + bound 10 tab; **Confidence C** cho "server push giá" (chỉ suy từ
việc bảng giá nằm trong form chứ không nằm trong `Data/*.Dat` — đã đối chiếu §2.5, không form shop nào load `.Dat`).

---

## 3. Tiền tệ

### 3.1 Bảng mã byte VISCII (TSVN flavour) — suy ra từ 10 chuỗi đã giải mã trong `opcode_1a.md`

| byte | ký tự | byte | ký tự | byte | ký tự |
|---|---|---|---|---|---|
| `A3` | ạ | `AB` | ề | `D0` | Đ |
| `A4` | ấ | `AC` | ể | `D5` | ạ |
| `A5` | ầ | `B0` | ồ | `DF` | ư |
| `A7` | ậ | `B4` | ả | `E0` | à |
| | | `B6` | ở | `E3` | ã |
| | | `B7` | ỡ | `E4` | ả |
| | | `BE` | ớ | `E6` | ộ |
| | | | | `F4` | ú |
| | | | | `FA` | ờ |
| | | | | `FC` | ủ |
| | | | | `FE` | ợ |

**Confidence A** cho các byte đã dùng để khớp 10 chuỗi ground-truth của `opcode_1a.md`
(`0x798038/04C/05C/078/084/094/0B4/0DC/0F0/100`) — byte thô dump từ `client_pseudo_c/lit_*.hex`.

### 3.2 Bảng tiền tệ — **client chỉ theo dõi 2 field tiền thật** (đã đếm đủ)

Đếm bằng `grep -l "0x12f8"` trên **toàn bộ 6588 file `.c`** ⇒ **đúng 16 hàm** chạm `player+0x12F8`.
Tương tự `0x12fc` ⇒ **5 hàm**, `0x1300` ⇒ **3 hàm**.

| # | Tên (chuỗi VISCII) | Offset trong `**player` | Kiểu / trần | S→C writer | Client-side helper |
|---|---|---|---|---|---|
| **1** | **"Tiền"** / **"kim"** — `0x79804C` = `54 69 AB 6E`, `0x798078` = `6B 69 6D` | **`+0x12F8`** | **i32**, clamp **9 999 999** (`DAT_0098967F`) | **`1A 01`** (`FUN_0072B084` +), **`1A 02`** (`FUN_0072B170` −), **`1A 04`** (`FUN_0072BCF8` set tuyệt đối cả 2 field) | đọc bởi 6 panel cờ bạc (§2.5) + 2 form shop (§2.8) |
| **2** | **`0x7982B8` = `4C DF 75 20 74 72 E6` → "Lưu trộ"** (tên tiền tệ #2; chữ `ộ` từ `lit_54A910` "Hoạt **độ**ng") | **`+0x12FC`** | **i32**, clamp **9 999 999** | **`1D 01`** (`FUN_0072B9B8` +), **`1D 02`** (`FUN_0072BA0C` −), **`1D 04`** (`FUN_0072BE10` set) | hiển thị ở `TMoneyAccountForm+0x45` (ctor) / `+0x114` (refresh) |
| **3** | **không có nhãn** (delta-only, không có caption) | **`+0x1300`** | i32, clamp **0x77359400** | **`1A 05`** (`FUN_00745ACC` +), **`1A 06`** (`FUN_00745B14` −), **`1A 04`** (set) | chỉ đọc ở `FUN_00632E3C:93` (`IntToStr(player->0x1300)`) |
| **4** | "Thủy binh" `0x7980DC` = `54 68 FC 79 20 62 69 6E 68` | **`+0x502`** | byte + trần `+0x500` | `1A 08` (tăng) | `opcode_1a.md` §4.8 |
| **5** | "Tài bảo" `0x7980F0` = `54 E0 69 20 62 E4 6F` | **`+0x513`** | byte + trần `+0x50C` | `1A 09` | idem |
| **6** | "Điểm đạn dược" `0x798100` = `D0 69 AC 6D 20 F0 D5 6E 20 64 DF FE 63` | **`+0x50E`** | byte + trần `+0x50C` | `1A 0A` | idem |

**`+0x146C`** — không phải tiền tệ, là **bộ đếm dạng `A*100+B`** do `23 04` ghi (§2.7a).

**Bằng chứng writer (chi tiết, từng field):**

```
FUN_0072B084  (+0x12F8)  0072b084_FUN_0072b084.c:41  uVar4 = 9999999 - *(int*)(p+0x12f8)
                                :51  *piVar1 = *piVar1 + n      (clamp bằng _IntOver)
                                :56  FUN_005D895C(gvar_007DA46C) ← refresh TMoneyAccountForm
                                :57  IntToStr(*(u32*)(p+0x12f8))
                                :61  FUN_007B0628(*(gvar_007DA530+0x128), "…kim…"+n) ← caption HUD
FUN_0072B170  (−0x12F8)  0072b170…c:43  if (n <= *(int*)(p+0x12f8)) { … }  else fail
FUN_0072BCF8  (set 2)    0072bcf8…c:52-56  *(u32*)(p+0x12F8) = u32@P[2..5];
                                        *(u32*)(p+0x1300) = u32@P[6..9];
                                :58  FUN_005D895C(gvar_007DA46C)
                                :60-63  caption = "kim" + IntToStr(p->0x12F8) + DAT_0072BDF8
FUN_0072B9B8  (+0x12FC)  0072b9b8…c:28-33  (mirror của 0072B084, trần 9999999)
FUN_0072BA0C  (−0x12FC)  0072ba0c…c:27-28  (mirror của 0072B170)
FUN_0072BE10  (set)      0072be10…c:38-39  *(u32*)(p+0x12FC) = u32@P[2..5]
FUN_00745ACC  (+0x1300)  00745acc…c:25-30  (trần 0x77359400)
FUN_00745B14  (−0x1300)  00745b14…c:26-32  (không cho âm)
```

**Dispatcher gọi chúng** (`functions/0078a89c_FUN_0078a89c.c`, bản inline của `FUN_0079175F` / `FUN_00791F76`):
- `case 0x1A`: `:4626` `FUN_0072BCF8(player, RP)` (sub 4) · `:4632` `FUN_00745ACC` (sub 5) · `:4638` `FUN_00745B14` (sub 6) · `:4614`/`:4621` `FUN_0072B084`/`FUN_0072B170` (sub 1/2, qua `case_023…c:34-92`)
- `case 0x1D`: `:4860` `FUN_0072B9B8` (sub 1) · `:4875` `FUN_0072BA0C` (sub 2) · `:4894` `FUN_0072BE10` (sub 4) · `:4899` `(**(*gvar_007DA46C+0x20))()` (sub 5 = **mở form Bank Money**)

**UI tiêu thụ** — `TMoneyAccountForm` (VMT `0x5D5858`, ctor `0x005D7D18`, singleton `gvar_007DA46C`,
tạo tại `functions/0051189c_FUN_0051189c.c:1494-1496`):
- `FUN_005D7D18:111` `caption1 = Format(player->0x12FC)` → `self+0x45`
- `FUN_005D7D18:119` `caption2 = Format(player->0x12F8)` → `self+0x46`
- `FUN_005D7D18:126-130` sprite `"btn_Bank_Gold_R"` / `"btn_Bank_Gold_L"` → `self+0x48/0x49` (**2 nút mũi tên**)
- `FUN_005D7D18:135` `*(float*)self+0x4F = 100.0f` (`0x40590000`) ⇒ **bước nhảy 100 / nút**
- `FUN_005D895C:59` `self+0x114 = Format(player->0x12FC)`; `:68` `self+0x118 = Format(player->0x12F8)`
- Ctor của form kế bên: `TGoldForm` (VMT `0x5D51B0`, ctor `0x005DC148`) — **`gvar_007D9C6C`, `functions/0051189c…c:1498-1500`**
- `TThingAccountForm` (VMT `0x5D5780`, ctor `0x005D9068`) — `gvar_007DA0A4`

**Wire layout các frame tiền (đã xác minh):**

| Frame | Payload | Ghi field | Confidence |
|---|---|---|---|
| `1A 01` | `[1A][01][Δ:u32 LE @P[2..5]]` (6 B) | `+0x12F8 += Δ`, banner `"Nhận được Δ Tiền"` + `sound\WA0014.wav` + `TLight "L10694"` | A |
| `1A 02` | `[1A][02][Δ:u32 LE]` | `+0x12F8 −= Δ` nếu đủ, banner `"Giảm Δ Tiền"` | A |
| `1A 03` | `[1A][03]` (2 B) | *không* — banner `"Dung lượng tiền bạc không đủ"` | A |
| `1A 04` | `[1A][04][kim:u32 LE @P[2..5]][c2:u32 LE @P[6..9]]` (10 B) | `+0x12F8 = kim`, `+0x1300 = c2`; refresh form + caption HUD | A |
| `1A 05` | `[1A][05][Δ:u32 LE]` | `+0x1300 += Δ` | A |
| `1A 06` | `[1A][06][Δ:u32 LE]` | `+0x1300 −= Δ` | A |
| `1A 08/09/0A` | `[1A][08][w:u16 LE]` | `+0x502` / `+0x513` / `+0x50E += 1`, banner tương ứng | C (chưa đọc lại body helper) |
| **`1D 01`** | `[1D][01][Δ:u32 LE @P[2..5]]` (6 B) | `+0x12FC += Δ`, banner `"… Lưu trộ"` / `"Thất bại  lưu trộ"` | A |
| **`1D 02`** | `[1D][02][Δ:u32 LE]` (6 B) | `+0x12FC −= Δ`, banner `"… Rờt nhận"` | A |
| **`1D 03`** | `[1D][03]` (2 B) | *không* — banner `"Dung lượng tiền trang khủng đủ"` (`0x798314`) | A |
| **`1D 04`** | `[1D][04][v:u32 LE @P[2..5]]` (6 B) | `+0x12FC = v` tuyệt đối | A |
| **`1D 05`** | `[1D][05]` (2 B) | *không* — `(**(*gvar_007DA46C+0x20))()` ⇒ **mở `TMoneyAccountForm`** | A |
| `1A 07` | `[1A][07][…]` | `FUN_00746774` chỉ đọc 1 byte `RP[1]`, không ghi field | C |

### 3.3 Rust hiện tại ↔ client

| Tên client | Rust field | Rust writer | Đúng? |
|---|---|---|---|
| Tiền/kim `player+0x12F8` | `Session::gold` | `1A 04` + `le32(gold) || 0000` (`spawn.rs:556-562`) | ✅ (Rust `1A 04` chỉ set `+0x12F8`, `+0x1300=0` — khớp) |
| Tiền #2 `player+0x12FC` | **`Session::bank_gold`** | `handle_bank_gold` (`trade_storage.rs:707`) **gửi `1D 01`/`1D 02`** | ⚠ **Xem §5 gap #1** |
| `player+0x1300` | **không có** | — | ❌ thiếu |
| `+0x502/50E/513` (Thủy binh / Tài bảo / Điểm đạn dược) | **không có** | — | ❌ thiếu |
| `+0x146C` | **không có** | bị `23 04` ghi đè ngoài ý muốn | ❌ |

---

## 4. Bảng frame tổng hợp

### 4.1 S→C (server → client)

| Frame | Subsystem | Payload | Đích | File:line |
|---|---|---|---|---|
| `1A 01` | tiền | `[Δ:u32 LE]` | `player+0x12F8 += Δ` + banner | `case_023_…c:33-65` |
| `1A 02` | tiền | `[Δ:u32 LE]` | `player+0x12F8 −= Δ` + banner | `case_023…c:67-92` |
| `1A 03` | tiền | — | banner "không đủ" | `case_023…c:94-97` |
| `1A 04` | tiền | `[kim:u32][c2:u32]` | `+0x12F8`, `+0x1300` | `case_023…c:98-100` → `0072bcf8…c:52-56` |
| `1A 05/06` | tiền #3 | `[Δ:u32]` | `player+0x1300 ± Δ` | `case_023…c:101-112` → `00745acc/00745b14` |
| `1A 08/09/0A` | bộ đếm | `[w:u16 LE]` | `+0x502/50E/513` + banner | `case_023…c:116-184` |
| `1D 01` | tiền #2 | `[Δ:u32 LE]` | `player+0x12FC += Δ` + banner | `0078a89c…c:4856-4870` → `0072b9b8` |
| `1D 02` | tiền #2 | `[Δ:u32 LE]` | `player+0x12FC −= Δ` + banner | `0078a89c…c:4871-4889` → `0072ba0c` |
| `1D 03` | tiền #2 | — | banner "không đủ" | `0078a89c…c:4890-4893` |
| `1D 04` | tiền #2 | `[v:u32 LE]` | `player+0x12FC = v` | `0078a89c…c:4894` → `0072be10…c:38-39` |
| `1D 05` | tiền #2 | — | mở `TMoneyAccountForm` | `0078a89c…c:4895-4899` |
| **`1E 01`** | bank `+0xA48` | `[12B record] × n`, `n=(len−1)/12` | ghi batch slot `TBankThing` | `case_026…c:28-29` → `007707a0` |
| **`1E 02`** | bank `+0xA48` | `[code:u16][5×u8][q:u32]` (12 B) | `FUN_0072BE9C(player,…)` | `case_026…c:30-31` → `00770454` |
| **`1E 03`** | bank `+0xA48` | `[w:u16 LE][b:u8]` (5 B) | `FUN_0072C038(player,w,b)` | `case_026…c:32-33` → `00770170` |
| **`1E 04`** | bank `+0xA48` | `[12B record]` | ghi 1 slot (bound ≤ 50) | `case_026…c:34-35` → `007705d8` |
| **`1E 05`** | bank `+0xA48` | `[slot:u8][n:u8]` (4 B) | `FUN_0077250C` giảm `n` | `case_026…c:36-37` → `0077021c` |
| **`1E 08`** | bank `+0xA48` | — | giảm 50 khỏi **cả 50 slot** | `case_026…c:38-39` → `FUN_0076FDB8` |
| **`1E 09`** | bank `+0xB14` | `[12B record] × n` | ghi batch (bound ≤ 50) | `case_026…c:40-41` → `007714a0` |
| **`1E 0A`** | bank `+0xB14` | `[12B record]` | `FUN_00749C78(player,…)` | `case_026…c:42-43` → `007702d0` |
| **`1E 0B`** | bank `+0xB14` | `[slot:u8][n:u8]` | `FUN_0077250C` giảm `n` | `case_026…c:44-45` → `007700bc` |
| **`1E 0C`** | **Thủy binh bank `+0xDC0`** | `[slot:u8][item:u16 LE][flag:u8][a:u8][b:u8][c:u8][d:u8][serial:u32 LE]` (14 B) | ghi 1 slot (bound ≤ 25) | `case_026…c:46-47` → `0077b72c…c:70-129` |
| **`1E 0D`** | **Thủy binh bank `+0xDC0`** | `[slot:u8][n:u8]` (4 B) | `FUN_0077250C` giảm `n` | `case_026…c:48-49` → `0077b938…c:47-63` |
| `23 04` | ⚠ **banner số** | `[A:u32][B:u32][D:double]` (17 B) | banner 10 000 ms + `player+0x146C = A*100+B` | `case_031…c:167-202` |
| `36 01` | Lotto | `[m:u8][k1..k5:u8]` (7 B) | `TLottoManager+5` lưới 20×5 | `case_054…c:26-27` → `0054e2f8…c:24-77` |
| `36 02` | Lotto | `[pot:u32 LE]` (5 B) | `mgr+0x16C` jackpot | `case_054…c:28-29` → `0054f18c…c:41-43` |
| `36 03` | Lotto | `[state:u8]` (3 B) | banner + reset | `case_054…c:30-31` → `0054f230…c:25-38` |
| `36 05` | Lotto | `[code:u8]` | 9 banner lỗi | `case_054…c:53-95` |
| `36 07/08/09` | Machine | RP | `TMachineManager` | `case_054…c:96-104` |
| `39 01/02/03` | `TSportManage` | `[id:u8][opt:u8?]` / — | mở / đóng panel / banner | `case_050…c:26-76` |
| `3A 01` | Tài/Xỉu | `[c1][c2][c3][flag][token:u32 LE]` (9 B) | `TRE_BiDaXiao` 3 ô + state | `case_051…c:28`, `opcode_3a.md` §4 |
| `3B 01/02/03` | Domino | RP | `TSBDManager+0x48[5·mode][1..4]` | `case_052…c:28-35` |
| `3C 01..04` | Cờ Tướng | RP | `TRE_ZMChessMain` | `case_053…c:29-38` |
| **`17 05`** | **HÒM ĐỒ** | `[12B record] × n` | **`player+0x9E0 + slot*4`**, **26 slot** (bound 0x19) | `case_020…c`, `FUN_0076BFC8…c:81-84,199-202` |

### 4.2 C→S (client → server)

| MainOp | Subsystem | Payload | Bằng chứng |
|---|---|---|---|
| `0x17` | shop NPC | tab + slot + qty (chi tiết §6) | `00611330…c:50` → `CONCAT31(…,0x17)` |
| `0x19` | shop NPC (2 form) | qty | `006090a8…c:68`, `0060b230…c:157` → `0x19` |
| `0x1D` | **nộp/rút tiền ở `TMoneyAccountForm`** | `[1D][sub][amount:u32 LE]` — **4 byte nhị phân**, không phải ASCII. `MOV EDX,[EAX+0x110]` đọc **số nguyên** từ field của form rồi `CALL 0x0077ee84` (DWORD → 4 byte LE) | asm `0x007861d0–0x007861e4`; `0077ee84_FUN_0077ee84.c` (decompose `param_2` thành 4 byte `&0xff`, `>>8`, `>>16`, `>>24`) → payload **6 byte** |
| `0x1E` | storage | jumptable 9 nhánh tại `0x786221` — **body không có trong dump** | asm `0x0078620C–0x0078621A` |
| `0x23 01` | **đổi mật khẩu** | `[23][01][len₁][len₂][len₃][len₄]` | asm `0x00786CFA–0x00786E63` |
| `0x23 02` | **xoá nhân vật** | `[23][02][len₁][len₂]` | asm `0x00786E6D–0x00786F5F` |
| `0x23 03` | **nạp thẻ** | `[23][03][6 byte ghép]` | asm `0x00786F64–0x00787094` |
| `0x23 05` | (chưa rõ form) | payload 6 byte | asm `0x00787099+` |
| `0x36` | Cafe ID | `[36][a:u8][b:u8] + text editor` | asm `0x007890DD–0x00789151` |
| `0x39` | panel request | `[39][variant]` | `FUN_00553818…c:22` |
| `0x3A` | Tài/Xỉu request | `[3A][variant][token:u32 LE]` | `FUN_00547FBC…c:21`, `opcode_3a.md` §6 |
| `0x3B` | Domino | `3B 01 [u32 LE điểm]` | `FUN_005524A0`, `opcode_3b.md` §6 |
| `0x3C` | Cờ Tướng | sub + payload động | `FUN_00546798…c:22`, `FUN_005418F8…c:66` |
| `0x3D` | **Lotto spin** | `[3D][…]` | `FUN_0054E4F0…c:22-23` |

---

## 5. Gap so với Rust server

| # | Gap | Mức | Bằng chứng |
|---|---|---|---|
| **1** | **`store_frame()` / `23 04` sai ngữ nghĩa.** Rust dùng `23 04` làm "God/HP/SP store" ở `spawn.rs:583-585` và `system.rs:62`. Client coi `23 04` là **banner số 10 s** ⇒ mỗi login hiện 3 banner 10 giây + ghi đè `player+0x146C = point*100`. | 🔴 cao | `case_031_00792BAD…c:167-202` |
| **2** | **Thiếu `player+0x1300`** (tiền tệ #3, có writer `1A 05/06`, clamp `0x77359400`). Không có Rust field. | 🔴 cao | `00745acc…c:25-30`, `00745b14…c:26-32` |
| **3** | **Cần kiểm chứng lại ngữ nghĩa `0x1D` C→S.** Rust đặt `OP_BANK (0x1D) = "Bank gold"` và `handle_bank_gold` đọc **u32 LE 4 byte**. Điều tra mới xác nhận parser đọc **u32 LE** là **đúng** (đính chính claim "ASCII" trước đó — xem dòng `0x1D` trong bảng C→S). Điểm còn mở là **sub nào = nộp / rút**: S→C đã rõ `1D 01 = +0x12FC`, `1D 02 = −0x12FC`, nhưng chiều C→S chưa map được sub→nghĩa. | 🟡 TB | `0072b9b8/0072ba0c/0072be10`; asm `0x00786185–0x00786207` |
| **4** | **Thiếu toàn bộ kênh cờ bạc `0x36` / `0x39` / `0x3A-0x3D`** (Lotto, Tài-Xỉu, Domino, Cờ Tướng, máy đánh). Rust `dispatcher.rs` không có match nào ⇒ rơi vào `unimplemented::handle` (`:394`). | 🔴 cao | §2.5 |
| **5** | **Thiếu S→C `0x1E` ba bảng slot** (`+0xA48` 51, `+0xB14` 51, `+0xDC0` 26). Rust `handle_storage_transfer` (`trade_storage.rs:541`) coi `0x1E` là **C→S storage transfer** và dùng `sub` 1/2/8 — nhưng `0x1E` S→C có **12 sub** với layout 12 B/record, và `0xDC0` là **Thủy binh bank chứ không phải túi**. | 🔴 cao | `case_026…c:26-52`, `TPlayer.Create…c:112-188` |
| **6** | **Sai định danh `player+0xA48`.** Handoff nói "`17 05` inventory nằm ở `player+0xA48`" — thực tế `17 05` (HÒM ĐỒ) ghi vào **`player+0x9E0`, 26 slot** (bound 0x19). `+0xA48` là ô đầu của mảng bank 51-slot do `0x1E` quản lý. | 🟡 trung bình | `FUN_0076BFC8…c:81-84,199-202` |
| **7** | **`0x1B` NPC shop ghi chú sai.** `dispatcher.rs:310-312` gọi `shops::handle_npc_shop` cho `0x1B`; nhưng `0x1B` C→S (`byte[0x1B]=26 → builder 0x00785FFD`) — cần verify thêm. *(chưa kiểm tra trong đợt này)* | 🟡 | `table_0x77F53C_dword200.hex[26]` |
| **8** | **Thiếu bộ đếm `+0x502/+0x50E/+0x513`** (Thủy binh / Điểm đạn dược / Tài bảo) và frame `1A 08/09/0A`. | 🟡 | `case_023…c:116-184` |
| **9** | **`TSportManage` không có Rust model** — không có state `ActiveId`, không có 6 panel. Gửi `39 01` hai lần liên tiếp mà không `39 02` sẽ **rò rỉ form** (đã xác nhận: `FUN_00553410` không tự free form cũ). | 🟡 | `opcode_39.md` §4.1 |
| **10** | **`TSe_CashGiftForm` chưa có frame nào** — Rust không gửi, client cũng không nhận được gì (chỉ có form UI local). | 🟢 thấp | §2.6 |

### 5.1 Hành vi nguy hiểm nếu mock server gửi sai

| Frame | Hậu quả |
|---|---|
| `1E 05/0B/0D` với `n > số lượng` | `FUN_0077250C` set counter về 0 rồi gọi `VMT+8` (giải phóng ô) — an toàn |
| `1E 08` | trừ **50** khỏi **cả 50 slot** `+0xA48` ⇒ dọn sạch bank 1 |
| `1E 0C/04/09` với `slot > 50` (`0xDC0`: `> 25`) | `_BoundErr` ⇒ **ERangeError giữa handler** (không phải no-op) |
| `39 01` khi `mgr+4 ≠ 0` | form cũ **rò rỉ** (global bị ghi đè, không `Free`) |
| `23 04` | banner 10 s + ghi `player+0x146C` |
| `36 01` với `m ≥ 0x15` | `_BoundErr` |
| `17 05` với `slot > 0x19` | `_BoundErr` |

---

## 6. Undetermined

| # | Câu hỏi | Vì sao chưa trả lời được | Hướng đi |
|---|---|---|---|
| **U1** | **Frame S→C kết quả quay `TFRollEgg`** (vòng quay casino) là gì? | Class **không có global singleton**, nên không xuất hiện trong bất kỳ `case_0NN_*.c` nào (grep toàn bộ `case_functions/functions/`). `TFRollEgg.Create` (`0x00772924`) chỉ được gọi từ `0x00772788`, thuộc hàm **Ghidra không tạo function** ⇒ không có `.c`/`.asm.txt` để đọc | Dredump vùng `0x772700–0x772920` từ `aLogin.exe`; tìm caller của hàm đó trong `.text` |
| **U2** | Cột nào của `RG.Dat` là **tiền cược**? | 3 u16 × record 7 B, chỉ biết field `off+5` là trọng số (tính max theo tier) | Đọc tiếp phần dùng `DAT_009D1610[4]` và hàm spin (`0x7728xx`) |
| **U3** | Frame S→C mở form **Trân Bảo Các** (`TFBlissBag`)? | `TFBlissBag` (`DAT_009264D4`) không được `case_0NN_*.c` nào tham chiếu | Cùng U1 |
| **U4** | `TFBlissBag` client có **tự roll** theo `odds` không? | `FUN_004DEF9C` (post-process) chưa đọc; chưa tìm hàm roll | Đọc `004def9c_FUN_004def9c.c` + tìm caller |
| **U5** | C→S của **Trân Bảo Các** mang gì? | Không có builder nào tham chiếu form/class | Giống U1 |
| **U6** | Layout chi tiết của `TBankThing` (`0x007726F0`) và `TWaterWarBankThing` offset `+0..+7`? | **Không có file `.c` cho `0x007726F0`** trong dump (chỉ có `.asm.txt` cho `0x00772688`) | Dredump `0x007726F0–0x00772720` |
| **U7** | S→C nào điền **bảng giá shop** `form+0x168 + tab*0x11`? | Không có `case_0NN_*.c` nào tham chiếu form shop | Grep `+ 0x168` trong `case_functions/` |
| **U8** | Payload C→S chính xác của `0x17` (shop) và `0x19` (shop) | Builder `0x007833F0` / `0x007853CA` có jumptable; `cs_op17_*` chỉ dump 2 case đầu | Đọc `cs_op17_targets_0x783405.hex` + `cs_op19_targets_0x78540f.hex` |
| **U9** | Body 9 nhánh của **C→S `0x1E`** | Jump table ở `0x00786221` bị Ghidra coi là **data**, không có `.asm.txt` phía sau | Dredump `0x00786221–0x00786583` |
| **U10** | `0x23 sub 0x05` dùng form nào? | Builder `0x00787099+` không đọc hết | Đọc tiếp asm `0x00787099–0x007871AF` |
| **U11** | Tên chính xác **tiền tệ #2** (`player+0x12FC`) | Chuỗi client là `"Lưu trộ"` / `"Rờt nhận"` — rõ là **text TSVN hỏng/không chuẩn** | Xem form `TMoneyAccountForm` trong runtime, hoặc `Data/` / file `.FLST` |
| **U12** | `TLottoManager` có phải "vòng quay số 5/20" không? | Chỉ có bound 20 lượt × 5 số, chưa đọc `TLottoManager.Create` | Đọc `0054d580`-vùng (`TLottoManager` VMT `0x54D580`) |
| **U13** | `TSe_CashGiftForm` gửi frame nào khi bấm nút? | Chỉ có method render (`0x0057479C`); callback nút không có trong dump | Tìm caller của `VMT_571BA0` slot button trong `.text` |

---

## 7. Source trail

| # | File | Dùng để |
|---|---|---|
| 1 | `ts_decompile/functions/00716370_TPlayer.Create.c:100-195` | Bảng 8 mảng slot của player (offset byte ×4 từ index dword) |
| 2 | `ts_decompile/case_functions/functions/case_023_0079175F_FUN_0079175f.c:33-112` | Wire `1A 01/02/03/04/05/06` |
| 3 | `ts_decompile/functions/0072b084/0072b170/0072bcf8/0072b9b8/0072ba0c/0072be10/00745acc/00745b14` | Writer của `+0x12F8 / +0x12FC / +0x1300` |
| 4 | `ts_decompile/functions/005d7d18_FUN_005d7d18.c:111-135` + `005d895c_FUN_005d895c.c:59,68` | `TMoneyAccountForm` hiển thị 2 tiền tệ |
| 5 | `ts_decompile/case_functions/functions/case_026_007921A6_FUN_007921a6.c:26-52` + `case_054_007957DC_FUN_007957dc.c:26-104` | Wire `1E 01..0D`, `36 01..09` |
| 6 | `ts_decompile/functions/0077b72c…c:70-129`, `0077b938…c:47-63`, `0077250c…c:25-45` | Layout Thủy binh bank + `slotObj+7` = số lượng |
| 7 | `ts_decompile/functions/0077299c…c:67-110`, `00772b94…c:22-197`, `00772f14…c:21-38` | `RG.Dat` (vòng quay): loader, giải mã 7 B/record ≤60 ô, timer |
| 8 | `ts_decompile/functions/004df424_FUN_004df424.c:68-110` | `BlissBag.Dat`: 190 B/record, ≤950 record |
| 9 | `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c:583-681` | Global singleton của 33 manager/form |
| 10 | `ts_decompile/functions/00666bf0_FUN_00666bf0.c` `case 0x5b` | `TFWheel` là effect skill, **không** phải vòng quay |
| 11 | `ts_decompile/functions/0077f414_FUN_0077f414.asm.txt` `0x7860xx–0x7891xx` | Toàn bộ builder C→S |
| 12 | `ts_dream/client_pseudo_c/jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex`, `jumptable_0x78A9B6_case_functions.csv` | Bản đồ MainOp → handler (§0) |
| 13 | `ts_dream/client_pseudo_c/cs_dispatch_byte200_0x77f474.hex`, `table_0x77F53C_dword200.hex` | Bản đồ MainOp → builder C→S (§0b) |
| 14 | `ts_dream/client_pseudo_c/lit_54F2EC/54F320/54F340.hex` + `lit_797ee8_798440.hex`, `lit_798314.hex` | Chuỗi VISCII của Lotto + tên tiền tệ |
| 15 | `ts_dream/src/server/spawn.rs:325-330,556-562,583-585` | `store_frame` / `1A 04` |
| 16 | `ts_dream/src/server/handlers/trade_storage.rs:541,707` | `handle_storage_transfer` / `handle_bank_gold` |
| 17 | `ts_dream/src/server/dispatcher.rs:295-394` | Bảng dispatch Rust (thiếu `0x36/0x39/0x3A-0x3D`) |
| 18 | `ts_dream/.scratch/client-pseudo-op-code/opcode_1e.md`, `opcode_1a.md`, `opcode_23.md`, `opcode_39.md`, `opcode_3a.md`, `opcode_3b.md`, `opcode_3c.md` | Cross-check định danh class + wire |

---

*Đã viết xong 2026-10-01. Bản đồ MainOp→handler §0 và §0b được **tự dựng lại từ 2 file hex + CSV** và đối chiếu 5/5 neo đã cho — xem `opcode_1a.md §2.1` và `opcode_1e.md §2.1`.*
*Mọi claim layout đều có `file:line`. Claim không có ASM đi kèm được đánh dấu Confidence C và nêu rõ trong §6.*