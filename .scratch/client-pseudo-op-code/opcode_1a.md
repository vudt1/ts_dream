# PHÂN TÍCH — Main OP 0x1A (Case 23, `FUN_0079175f` @ `0x0079175F`)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)**
Trạng thái: **Wire layout + luồng xử lý xác minh từ mã nguồn sơ cấp** (case handler + dispatcher inline + helper + jump table nhị phân + `FUN_0077f414`). **9 helper dạng `func_0x…` chưa được Ghidra phục hồi body** → phần *tên nghiệp vụ* của các sub-op 1,2,5,6,7,8,9,10 chỉ đạt mức "phân loại theo cơ chế", có ghi chú độ tin cậy ở từng mục.

---

## 0. Đính chính giả định "Talk / Chat / Thoại NPC" — **SAI**

Handoff (`.scratch/op-code/handoff-opcode-exploration-guide.md`, mục 3, ưu tiên 5) phỏng đoán OP 0x1A = *"Hệ thống chat, thoại NPC, đối thoại sự kiện và trận đấu"*. **Bằng chứng nguồn sơ cấp bác bỏ hoàn toàn:**

| # | Kiểm chứng | Kết quả |
| :-- | :-- | :-- |
| 1 | `case_023` có gọi hàm nạp dòng chat `FUN_007ab870`? | **KHÔNG.** `FUN_007ab870` chỉ được gọi bởi case **002, 003, 016, 018, 020, 031, 035** (grep toàn bộ `case_functions/functions/`). |
| 2 | `case_023` có đụng form log chat `gvar_007DA1B0` (`TTalkMsgForm`, VMT_7AB774)? | **KHÔNG.** Handler chỉ chạm **4 global**: `gvar_007DA7BC` (18×), `gvar_007DA084` (12×), `gvar_007DA2FC` (3×), `gvar_007DA010` (4×). Helper case 4 chạm thêm `gvar_007D9D30`, `gvar_007DA46C`, `gvar_007DA530`. |
| 3 | Có resolve tên người chơi (`FUN_0075ddb8` / `FUN_00722508`)? | **KHÔNG.** Hai hàm này **không xuất hiện** trong `case_023` và trong `FUN_0072bcf8` (helper duy nhất được decompile của OP này). |
| 4 | Có field **chuỗi biến độ dài** (nội dung tin) trong payload? | **KHÔNG.** Mọi phép đọc là fixed-width: `4B LE` (sub 1,2,5,6), `4B+4B LE` (sub 4), `2B LE` (sub 8,10), `1B` (sub 9), không đọc gì (sub 3). Không có phép cắt "tới hết payload" kiểu `_LStrCopy(RP, k, len-k)` mà OP 0x02 dùng cho `msg`. (Ngoại lệ duy nhất: sub-op `0x07` trao cả `RestPayload` cho parser chưa phục hồi — xem 4.7.) |
| 5 | Nội dung hiển thị là gì? | Chuỗi **hằng số compile-time** trong code page (`0x798038…0x798100`), **ghép quanh một CON SỐ** bằng `IntToStr`. Đó là nhãn thông báo, không phải nội dung tin nhắn. |
| 6 | Chiều C→S? | `FUN_0077f414` (`SendCommand`) có `case 0x1a: break;` **rỗng** (dòng 939–940); **0/110** điểm gọi `FUN_0077f414` truyền opcode 0x1A; `TForm1_CY_AddSedQueue` chỉ có 2 điểm gọi ngoài SendCommand (`Button33Click` = keepalive). → **Không có đường "gửi tin chat" bằng OP 0x1A.** |

**Nguồn gốc hiểu nhầm (rất có thể):** OP 0x1A hiển thị bằng **`TSe_TalkMsgFormPlus`** (`gvar_007DA084`, gán tại `0051189c_FUN_0051189c.c:1264-1266`) — tên class có chữ **"Talk"**, nhưng đây là **banner/marquee thông báo** (phương thức VMT `+0x90` = `FUN_0063c84c` → `FUN_007bc1c8(form, text, ms, x, y)` — set text `+0xac`, timer `+0x4a` = số ms, cờ `+0x5e`). Chat log thật là class **khác**: `TTalkMsgForm` (`gvar_007DA1B0`) do **Main OP 0x02** cấp dữ liệu (đã chứng minh trong `.scratch/op-code/opcode_02.md`).

### Kết luận nghiệp vụ đúng theo evidence

> **Main OP 0x1A = kênh S→C "đồng bộ bộ đếm/tài khoản số + thông báo sự kiện bằng banner"** (numeric state sync + reward/limit notification).
> Neo chứng nghiệp vụ **chắc chắn 100%** là **sub-op 0x04**: ghi 2 DWORD vào bản ghi người chơi (`+0x12F8`, `+0x1300`) và **làm mới form tiền `Form_Bank_Money`** (`TMoneyAccountForm`, `gvar_007DA46C`) + **caption nút `btn_001` của HUD `TSe_MainStatus`**.
> Các sub-op còn lại là: setter số (0x05, 0x06), parser record (0x07), và "áp dụng theo ID / theo giá trị → banner + tiếng chuông + hiệu ứng sáng tại chân nhân vật" (0x01, 0x02, 0x08, 0x09, 0x0A), cùng một banner tĩnh (0x03).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Server đẩy **giá trị số** (ID đối tượng, tiền/điểm, số lượng, chỉ số giới hạn) xuống client; client (a) **áp dụng/ghi** vào bản ghi người chơi `**gvar_007DA7BC` (TPlayer/TWorldPlayer — record rất lớn, có tiền tại `+0x12F8/+0x12FC/+0x1300`, tọa độ `+0x54/+0x58`, map `+0x63a`, bitmap switch `+0x8b2`, mode theo map `+0x145c`), (b) **refresh UI tiền/HUD** ở sub-op 4, và (c) **hiện banner chữ 2000 ms** `TSe_TalkMsgFormPlus` với nội dung `NHÃN + IntToStr(giá_trị) + NHÃN`, kèm phát `sound\WA0014.wav` và spawn hiệu ứng sáng `TLight "L10694"` tại vị trí người chơi.
- **Không có**: log chat, tên người gửi, nội dung text, NPC dialog, theo dõi kênh.
- **Hướng**: **một chiều (S→C)**. Không có builder C→S cho opcode này.
- **Payload**: tối thiểu 2 byte (`[0x1A][SubOp]`); dài nhất 10 byte (sub-op 0x04). **Không có payload biến độ dài được handler tự cắt.**

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (kiểm chứng lại từ nhị phân, không tin handoff)

```
MainOp 0x1A → jumptable_byte200_0x78A8EE.hex[0x1A] = 23
            → jumptable_dword200_0x78A9B6.hex[23]  = 0x0079175F  (= FUN_0079175f, Case 23)
```
(Đã tự parse 2 file `.hex`: byte_table[0x1A]=23, dw[23]=0x79175f, dw[22]=0x79157c, dw[24]=0x791d80 — khớp bảng handoff; ghi chú thêm: `byte_table[0x1C] = 0` → **MainOp 0x1C inactive**, rơi vào default case `0x00796347`.)

Framing/XOR/dispatcher/tick-pump: giữ nguyên như `opcode_00_01.md` §2 (token `F4 44`, `L:Word LE`, XOR `0xAD`, `_LStrCopy(local_c, 2, len-1)` → **RestPayload** vào `ECX`, `MainOp` vào `DL` → `FUN_0078a89c`).

### 2.2. Quy ước ký hiệu trong bài

- `P[i]` = byte thứ i của **payload** sau header (`P[0] = 0x1A` = MainOp).
- `RP[i]` = byte thứ i của **RestPayload** (`RP[i] = P[i+1]`), 0-based.
- Delphi `_LStrCopy(s, Index, Count, out)` là **1-based**: `_LStrCopy(RP, 2, 4)` → `RP[1..4]` = **`P[2..5]`**.

### 2.3. Đọc SubOp (dòng 25–32 của `case_023_0079175F_FUN_0079175f.c`)

```c
iVar6 = *(int *)(unaff_EBP + -0xc);                     // RP (RestPayload)
iVar3 = 0;
if (*(int *)(iVar6 + -4) == 0) { iVar3 = _BoundErr(0); } // AnsiString len tại [RP-4] == 0 → RangeError
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6 + iVar3);  // SubOp = RP[0] = P[1]
switch (*(undefined4 *)(unaff_EBP + -0x14)) { ... }
```
- `SubOp` = **1 byte thường**, không codec.
- Nếu server gửi frame **chỉ có 1 byte payload** (`L=1`) → `RP` rỗng → `_BoundErr(0)` **ném exception trong dispatcher**.
- `switch` liệt kê **`case 1..10` (0x01–0x0A)**, **không có `case 0`, không có `default`** → SubOp `0x00` hoặc `> 0x0A` **rơi qua switch, không làm gì** (chỉ chạy epilogue dọn chuỗi).
- **Artefact decompile cần biết**: khối `_LStrArrayClr`/`_LStrClr` ở cuối hàm (dòng 215–243) **là epilogue chung của dispatcher** (giống hệt `case_022`, `case_024`), không phải logic của OP 0x1A.

### 2.4. Codec & các API mà handler dùng

| Helper | Vai trò (đã kiểm chứng) |
| :-- | :-- |
| `FUN_0077ef7c` | 4 byte → **DWORD LE** `b0 + b1·0x100 + b2·0x10000 + b3·0x1000000`; bounds-check từng byte → payload thiếu byte sẽ `_BoundErr`. |
| `FUN_0077eb9c` | 2 byte → **Word LE** `b0 + b1·0x100`. |
| `IntToStr` | số → chuỗi thập phân, là **mảnh giữa** của thông báo. |
| `_LStrCatN(dest, 3, s1, s2, s3)` | nối 3 chuỗi (varargs push ngược). |
| `(**(gvar_007DA084->VMT)+0x90)(form, text, 2000, 0, 0)` | **banner 2000 ms** (`FUN_0063c84c` → `FUN_007bc1c8`). |
| `FUN_007a7f20(gvar_007DA010 + "sound\WA0014.wav")` | phát chuông báo chung dụng (WA0014 dùng 30 nơi). **Bỏ qua theo yêu cầu.** |
| `FUN_0052bae4(gvar_007DA2FC, P_X, P_Y)` | spawn `TLight` (`VMT_772E1C`, res `"L10694"`) tại `player+0x54/0x58`. **Hiệu ứng ảnh — chỉ ghi nhận, không mổ xẻ.** |

---

## 3. Bảng tổng hợp SubOp (`switch(SubOp)` @ dòng 32)

| SubOp | Độ dài payload | Field đọc | Hàm hội tụ | Thành công → | Thất bại → | Banner? | Sound? | Light@pos? |
| :-: | :-: | :-- | :-- | :-- | :-- | :-: | :-: | :-: |
| `0x01` | 6 | `A = P[2..5]` DWORD LE | `func_0x0072B084(player, A)` → bool | `0x798038 + n + 0x79804c` | `0x79805c + n + 0x798078` | ✔ *(gate `func_0x0072C0F0`)* | ✔ | ✔ *nếu `player+0x145c==2`* |
| `0x02` | 6 | `A = P[2..5]` DWORD LE | `func_0x0072B170(player, A)` → bool | `0x798084 + n + 0x79804c` | `0x798094 + n + 0x798078` | ✔ *(gate `func_0x0072C0F0`)* | ✖ | ✖ |
| `0x03` | 2 | không đọc | — | hằng `0x7980b4` | — | ✔ (luôn) | ✖ | ✖ |
| `0x04` | 10 | `A = P[2..5]`, `B = P[6..9]` DWORD LE | `FUN_0072bcf8(player, RP)` | ghi `player+0x12F8=A`, `+0x1300=B`; refresh `Form_Bank_Money`; caption `btn_001` | — | ✖ | ✖ | ✖ |
| `0x05` | 6 | `D = P[2..5]` DWORD LE | `func_0x00745ACC(player, D)` | (setter nội bộ) | — | ✖ | ✖ | ✖ |
| `0x06` | 6 | `D = P[2..5]` DWORD LE | `func_0x00745B14(player, D)` | (setter nội bộ) | — | ✖ | ✖ | ✖ |
| `0x07` | ≥2 | **cả `RP` như 1 chuỗi** | `func_0x00746774(player, RP)` | parser nội bộ (chưa phục hồi) | — | ✖ | ✖ | ✖ |
| `0x08` | 4 | `W = P[2..3]` Word LE | `func_0x0074C64C(player)` → bool | `0x798038 + n + 0x7980dc` | `0x79805c + n + 0x7980dc` | ✔ (luôn) | ✔ | ✔ (luôn) |
| `0x09` | 3 | `B = P[2]` 1 byte | `func_0x0074C5F4(player)` → bool | `0x798038 + n + 0x7980f0` | `0x79805c + n + 0x7980f0` | ✔ (luôn) | ✔ | ✔ (luôn) |
| `0x0A` | 4 | `W = P[2..3]` Word LE | `func_0x0074C6B4(player)` → bool | `0x798038 + n + 0x798100` | `0x79805c + n + 0x798100` | ✔ (luôn) | ✔ | ✖ |
| `0x00`, `≥0x0B` | — | — | — | **no-op** (rơi khỏi switch) | | ✖ | ✖ | ✖ |

**Nhận xét cấu trúc (bằng chứng thêm rằng đây là "kênh thông báo số", không phải chat):**
- Hai hằng `0x798038` (nhánh thành công) và `0x79805c` (nhánh thất bại) **dùng chung cho 4 sub-op 1, 8, 9, 10** → đóng vai trò **mở đầu "thành công / không được"**; mảnh thứ hai (`0x7980dc / 0x7980f0 / 0x798100`) **khác nhau theo từng sub-op** → **danh từ chỉ loại bộ đếm**. Sub-op 2 có cặp mở đầu riêng (`0x798084` / `0x798094`) nhưng **dùng chung mảnh đuôi với sub-op 1** (`0x79804c`, `0x798078`) → 1 & 2 là **hai biến thể của cùng một nghiệp vụ**.
- Ở mọi sub-op có 2 nhánh, **nhánh "thất bại" chỉ ghép chuỗi rồi bỏ** (không gọi banner) → code chết của compiler Delphi; **chỉ nhánh thành công mới hiển thị**.
- Kích thước hằng số (suy từ khoảng cách trong code page, Delphi `[len:4LE][chars][00]` → `≤ gap-5` ký tự): `0x798038 ≤15`, `0x79804c ≤11`, `0x79805c ≤23`, `0x798078 ≤7`, `0x798084 ≤11`, `0x798094 ≤27`, `0x7980b4 ≤35`, `0x7980dc ≤15`, `0x7980f0 ≤11`, `0x798100 ≤19`. **Toàn bộ là nhãn ngắn 7–35 ký tự — không thể là nội dung tin chat.** *(Nội dung byte thật **không đọc được** trong workspace: chưa có dump `lit_7980xx.hex`; các dump `redump/lit_*.hex` hiện có chỉ phủ vùng `0x595…`, `0x77F771`, `0x7A2…`, `0x7AB…`.)*

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — "Áp dụng theo ID số 32-bit" (thành công: banner + chuông + sáng)
**Wire layout** (6 byte payload):
```
P[0]   = 0x1A            MainOp
P[1]   = 0x01            SubOp
P[2..5]= A : DWORD LE    id/số hiệu (FUN_0077ef7c)
```
**Đọc**: `_LStrCopy(RP,2,4,&t)` → `FUN_0077ef7c(ctx,t)` → lưu slot `[EBP-0x3c]` (dòng 34–36).
**Logic** (dòng 37–65):
```c
if (!func_0x0072B084(**gvar_007DA7BC, A))      // body CHƯA phục hồi (index.csv không có entry 0x0072B084)
    msg := LBL(0x79805c) + IntToStr(A) + LBL(0x798078);      // không hiển thị
else {
    msg := LBL(0x798038) + IntToStr(A) + LBL(0x79804c);
    if (func_0x0072C0F0(**gvar_007DA7BC)) {                   // cờ "được phép thông báo"
        banner(msg, 2000, 0, 0);                              // TSe_TalkMsgFormPlus VMT+0x90
        FUN_007a7f20(gvar_007DA010 + "sound\\WA0014.wav");
        if (*(char*)(player + 0x145c) == 2)                   // mode theo map (đọc từ mapId +0x63a)
            FUN_0052bae4(scene /*gvar_007DA2FC*/, player[0x54], player[0x58]);
    }
}
```
**Global/offset chạm tới**: chỉ **đọc** bản ghi player; hiệu ứng sáng ghi vào `scene+0xFBC + i*4` (mảng 4 `TLight`). Không ghi field nào của player ở tầng handler.
**Cao độ**: `func_0x0072B084` nằm sát `FUN_0072b004`/`FUN_0072b250` trong cluster method của cùng class người chơi ⇒ là **method "thử áp dụng/xử lý đối tượng id=A" trả bool**.

### 4.2. SubOp `0x02` — biến thể nhẹ của 0x01 (banner, không âm thanh/hiệu ứng)
Wire giống hệt `0x01` (`[01][02][A:4B LE]`). Khác biệt (dòng 67–93):
- predicate = `func_0x0072B170(player, A)`;
- nhãn: thành công `0x798084 + n + 0x79804c`, thất bại `0x798094 + n + 0x798078`;
- thành công + cờ `func_0x0072C0F0` → **chỉ banner**, không `WA0014.wav`, không `TLight`.
⇒ cùng họ nghiệp vụ với 0x01 nhưng **mức ưu tiên thông báo thấp hơn**.

### 4.3. SubOp `0x03` — banner tĩnh, không field
**Wire**: `[0x1A][0x03]` (payload 2 byte). Dòng 94–97:
```c
banner(&UNK_007980b4 /*hằng ≤35 ký tự*/, 2000, 0, 0);
_LStrLAsg(msg_slot, 0x7980b4);   // dead store
```
Không đọc byte nào ⇒ đây là **một "notice" cố định** của kênh (không mang dữ liệu).

### 4.4. SubOp `0x04` — **ĐỒNG BỘ TIỀN/ĐIỂM (nghiệp vụ chắc chắn nhất)**
**Wire layout** (10 byte payload):
```
P[0]    = 0x1A             MainOp
P[1]    = 0x04             SubOp
P[2..5] = A : DWORD LE     → playerRecord + 0x12F8
P[6..9] = B : DWORD LE     → playerRecord + 0x1300
```
**Handler chỉ làm 1 việc** (dòng 98–100): `FUN_0072bcf8(*(int*)gvar_007DA7BC, RP)`.
**Bên trong `FUN_0072bcf8`** (`0072bcf8_FUN_0072bcf8.c` d.51–61, `.asm.txt` d.20–63):

| Bước | Code | Ý nghĩa đã kiểm chứng |
| :-- | :-- | :-- |
| 1 | `_LStrCopy(RP,2,4)` → `FUN_0077ef7c` → `**gvar_007DA7BC + 0x12F8 = A` | ghi **tiền A** |
| 2 | `_LStrCopy(RP,6,4)` → `FUN_0077ef7c` → `**gvar_007DA7BC + 0x1300 = B` | ghi **tiền/điểm B** |
| 3 | `FUN_005d895c(*(gvar_007DA46C))` | refresh **`TMoneyAccountForm`** (`Form_Bank_Money`, gán global tại `0051189c…c:1495`): `form+0x114 := Format('%7d', player+0x12FC)`, `form+0x118 := Format('%7d', player+0x12F8)` (bắt buộc field này `≤ 9 999 999` — xem clamp tại `FUN_0072b9b8/0072ba0c`). **Chú ý: `+0x12FC` KHÔNG bị gói này ghi**, chỉ được hiển thị lại. |
| 4 | `FUN_007b0628(**(gvar_007DA530)+0x128, "kim" + IntToStr(A) + LBL@0x72BDF8)` | set **caption** của `TSe_MainStatus.btn_001` (`0058a964_FUN_0058a964.c:160-190`: 9 `TSe_FixedButton` chỉ số `+0x4A…+0x52`, `+0x128/4 = 0x4A`). `FUN_007b0628` ghi `obj+0xA9 := (text≠nil)`, `obj+0xAC := text`. |

`"kim"` là literal ASCII 3 ký tự Ghidra render từ hằng `0x72BE0C`; **cùng khuôn** xuất hiện ở `FUN_005dd000:400` (`_LStrCatN(&s,4,"kim", IntToStr(n), " ", <tên item>)` dùng để **gán nhãn 25 ô nút** ⇒ `"kim"` là **prefix nhãn bộ đếm/tài khoản**, không phải text hội thoại.
**Kết luận sub-op 0x04**: đây là **gói đồng bộ số dư tiền/điểm (bank/currency sync)** — đúng kiểu "server là quyền uy, client chỉ hiển thị" (ADR-0001).

### 4.5. SubOp `0x05` — setter DWORD, im lặng
**Wire**: `[0x1A][0x05][D:4B LE]` (6 byte). Dòng 101–106:
```c
D = FUN_0077ef7c(_LStrCopy(RP,2,4));
func_0x00745ACC(**gvar_007DA7BC, D);   // body CHƯA phục hồi
```
Không banner, không sound, không hiệu ứng ⇒ **ghi thuần field số** vào bản ghi player (cluster `0x00745…` thuộc class player: `FUN_00745b58` bên cạnh ghi `player[+0x57C + idx*4] + 0x565`).

### 4.6. SubOp `0x06` — setter DWORD thứ hai (đôi với 0x05)
**Wire**: `[0x1A][0x06][D:4B LE]` (6 byte). Dòng 107–112 → `func_0x00745B14(player, D)` (nằm ngay sau `0x00745ACC`, **cách nhau đúng 0x48 byte** ⇒ cặp setter field kề nhau trong cùng class).

### 4.7. SubOp `0x07` — **truyền nguyên vẹn RestPayload cho parser nội bộ**
**Wire**: `[0x1A][0x07][<record biến độ dài…>]` (≥2 byte). Dòng 113–115:
```c
func_0x00746774(**gvar_007DA7BC, RP);   // RP = P[1..end] — gồm cả byte SubOp
```
- **Không** có `_LStrCopy`/codec nào ở tầng handler → toàn bộ việc cắt field nằm trong `func_0x00746774` (**chưa phục hồi**).
- Đây là **sub-op duy nhất có khả năng mang payload biến độ dài** (và do đó, nếu OP 0x1A có phần "chuỗi" nào đó, thì nó nằm ở đây). **Tuyên bố "0x1A không có text" chỉ tuyệt đối cho các sub-op 1–6, 8–10**; với `0x07` phải để mức *unknown*.

### 4.8. SubOp `0x08` — giá trị Word + giới hạn → banner + chuông + sáng (không gate)
**Wire**: `[0x1A][0x08][W:2B LE]` (4 byte). Đọc: `_LStrCopy(RP,2,2)` → `FUN_0077eb9c` → `& 0xFFFF` (+ `_BoundErr` nếu vượt giới hạn — artefact range-check Delphi). Dòng 116–147:
```c
if (!func_0x0074C64C(player))  msg := LBL(0x79805c) + IntToStr(W) + LBL(0x7980dc);   // không hiển thị
else { msg := LBL(0x798038) + IntToStr(W) + LBL(0x7980dc);
       banner(msg,2000,0,0); sound(WA0014);
       FUN_0052bae4(scene, player+0x54, player+0x58); }        // KHÔNG gate bởi +0x145c
```
Khác 0x01: **không kiểm tra `func_0x0072C0F0`** → luôn bật thông báo nếu predicate đúng.

### 4.9. SubOp `0x09` — giá trị BYTE + giới hạn → banner + chuông + sáng
**Wire**: `[0x1A][0x09][B:1B]` (3 byte). **Đọc byte trực tiếp, không `_LStrCopy`** (dòng 149–156):
```c
iVar6 = RP; iVar3 = 1;
if (*(uint*)(iVar6-4) < 2) iVar3 = _BoundErr(1);   // yêu cầu len(RP) ≥ 2
bVar1 = *(byte*)(iVar6 + 1);                        // = P[2]
```
Nhánh thành công: `LBL(0x798038) + IntToStr(B) + LBL(0x7980f0)` → banner + `WA0014.wav` + `TLight` tại tọa độ; thất bại: `LBL(0x79805c) + … + LBL(0x7980f0)` (không hiển thị). Predicate: `func_0x0074C5F4(player)`.

### 4.10. SubOp `0x0A` — giá trị Word + giới hạn → banner + chuông (không sáng)
**Wire**: `[0x1A][0x0A][W:2B LE]` (4 byte), đọc như 0x08. Predicate `func_0x0074C6B4(player)`; thành công: `LBL(0x798038) + n + LBL(0x798100)` + banner + `WA0014.wav`; **không** gọi `FUN_0052bae4`.

> **Ghi chú thứ tự ghép chuỗi**: `A + IntToStr(n) + B` được xác định theo quy ước `_LStrCatN` (varargs push ngược) — **đã kiểm chứng chéo bằng listing thật** `0072bcf8_FUN_0072bcf8.asm.txt:49-58` (`PUSH 0x72bdf8` → `IntToStr` → `PUSH 0x72be0c` ⇒ thứ tự nguồn = `"kim", <số>, 0x72bdf8`). Trong `case_023` Ghidra **mất tham số varargs**, chỉ còn các lệnh gán slot `[EBP-8]`/`[EBP-4]`/`[EBP+0]`; hai hằng luôn **kẹp quanh** `IntToStr`, nhưng **trái/phải của từng hằng có thể đảo** so với bảng ở mục 3. Giá trị địa chỉ là chắc chắn; nhãn trái/phải là **suy luận**.

---

## 5. Chiều Client → Server liên quan

`ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`, `switch (param_2 & 0xff)` tại dòng 768):
```c
case 0x1a:
  break;              // dòng 939–940  → KHÔNG có builder cho OP 0x1A
```
Kiểm chứng bổ sung (toàn cây `ts_decompile/functions`):
- **110** điểm gọi `FUN_0077f414`; **không** điểm nào truyền low-byte = `0x1A`.
- Hàng đợi gửi `TForm1_CY_AddSedQueue` (`0051633c`) chỉ có **2** điểm gọi ngoài SendCommand: `TForm1.Button33Click` (`00519fe0`, khung keepalive) và SendCommand. ⇒ mọi frame C→S đều đi qua switch nói trên.
- **Đối chiếu "chat"**: OP nhận chat là **0x02** (`case 2: break;` cũng rỗng — xem `opcode_02.md` §6). Các khuôn "opcode + text tự do" thực sự nằm ở **case 0x37** (dòng 1012–1019: `_LStrFromChar(op)` + `_LStrCat(…, **(gvar_007DA3B4+0x138) + 0x1a0)` — text của input chat) và case 0x1d/0x3a/0x3b. ⇒ **nếu cần dựng "gửi chat", phải tìm opcode của các khuôn này, KHÔNG phải 0x1A.**

**Độ tin**: `case 0x1a` rỗng là **bằng chứng trực tiếp**; kết luận "client không bao giờ gửi 0x1A" có **2 caveats**: (a) `FUN_0077f414` được decompile **có suy hao** (13 jump-table trong `.asm.txt`, riêng `case 0x19` phải redump `subtable_0x7853E5/0x78540F`), (b) 9 method `func_0x…` của class player chưa phục hồi nên *về nguyên tắc* một builder 0x1A có thể ẩn ở đó. **Không có bằng chứng dương** cho hướng gửi 0x1A.

---

## 6. Ghi chú cho Mock Server

1. **Xử lý OP 0x1A như kênh downlink-only.** Không cần (và không nên) chờ client gửi `0x1A`.
2. **Định dạng frame** (như mọi OP): `[0xF4 0x44][L:Word LE][payload]`, **XOR `0xAD` toàn frame**, `L = độ dài payload`, payload = `[0x1A][SubOp][…]`.
3. **Bảng phát (tóm tắt)**
   - `0x04` — quan trọng nhất, **bắt buộc** nếu mock có shop/bank: `[0x1A][0x04][A:4B LE][B:4B LE]` → `player+0x12F8=A`, `+0x1300=B`. Giữ **A ≤ 9 999 999** để `Form_Bank_Money` hiển thị `%7d` đúng; nếu A=0 thì caption HUD thành `"kim0…"`.
   - `0x03` — notice tĩnh: `[0x1A][0x03]`.
   - `0x01/0x02` — `[…][A:4B LE]`; `0x05/0x06` — `[…][D:4B LE]`; `0x08/0x0A` — `[…][W:2B LE]`; `0x09` — `[…][B:1B]`.
   - `0x07` — **chưa đặc tả được**; nếu chưa biết format, **đừng gửi**.
4. **Không gửi `SubOp = 0x00` hoặc `≥ 0x0B`** — không crash nhưng vô nghĩa (rơi khỏi switch). **Không gửi payload chỉ có opcode (`L=1`)** → `RP` rỗng → `_BoundErr(0)` **RangeError trong dispatcher**.
5. **Độ dài phải chính xác**: sub-op 1/2/5/6 yêu cầu **đủ 4 byte** kể từ `P[2]`; 8/10 yêu cầu **đủ 2 byte**; 9 yêu cầu `len(RP) ≥ 2`. Thừa byte → `_LStrCopy` cắt cố định nên **thừa bị bỏ qua** (trừ `0x07`).
6. **Điều kiện để thấy được phản hồi** (với 0x01/0x02): predicate `func_0x0072B084/0x0072B170` phải trả **true** *và* `func_0x0072C0F0` phải khác 0. Hai hàm này **chưa có body** ⇒ khi test trên client thật, nếu **không thấy banner**, nguyên nhân nằm ở state của bản ghi player, **không phải ở framing**. Sub-op 8/9/10 **không** phụ thuộc `func_0x0072C0F0`.
7. **Hiệu ứng phụ khi mock phát 0x01/0x08/0x09**: client sẽ **phát `sound\WA0014.wav`** và **bật `TLight "L10694"` tại tọa độ người chơi** (`0x01` chỉ khi `player+0x145c == 2`). Muốn "yên lặng" khi test số liệu → dùng `0x04`, `0x05`, `0x06`.
8. **Không dùng OP 0x1A cho chat/thoại NPC.** Chat & nhãn kênh: **OP 0x02** (đã đặc tả ở `opcode_02.md`). Hội thoại NPC dạng form/nút chọn: không nằm ở 0x1A (handler không đụng form dialog nào; không gọi `FUN_007ab870`/`TTalkMsgForm`).
9. **Thứ tự & nhịp**: client bơm tối đa 50 gói/tick ~30 ms (`TForm1.CY_DelRevQueue`). **Không có ACK** cho OP 0x1A (khác OP 0x00 vốn gửi frame rỗng `FUN_0077f414(.., 0)`).
10. **Cần dump thêm để đóng 100%**: (a) `lit_798038…lit_798118.hex` (nội dung nhãn — cp1258 → NFC, như đã làm cho OP 0x02); (b) decompile/dump asm `0x0072B084`, `0x0072B170`, `0x0072C0F0`, `0x00745ACC`, `0x00745B14`, `0x00746774`, `0x0074C5F4`, `0x0074C64C`, `0x0074C6B4`; (c) hằng `0x72BDF8` & `0x72BE0C` (đuôi caption HUD). Có (a)+(b) là đổi được tên các sub-op từ "phân loại cơ chế" sang "tên nghiệp vụ".

---

## 7. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
| :-- | :-- | :-- | :-- |
| 1 | `ts_decompile/case_functions/functions/case_023_0079175F_FUN_0079175f.c` | @`0x0079175F`; SubOp d.25–31; switch d.32; case 1→10 = d.33–214; epilogue d.215–243 | Handler chính, danh sách nhánh, mọi phép đọc payload |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | inline OP 0x1A, d.4551–4739 | Bản decompile thứ 2 (độc lập) — khớp 1:1, xác nhận không có nhánh nào bị mất |
| 3 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` / `jumptable_dword200_0x78A9B6.hex` | `[0x1A]=23`, `dw[23]=0x79175F`; `[0x1C]=0` | Ánh xạ MainOp→Case→Target (tái sinh, không copy handoff) |
| 4 | `ts_decompile/functions/0072bcf8_FUN_0072bcf8.c` + `.asm.txt` | c: d.51–61; asm: d.20–63 (`PUSH 0x72bdf8`, `PUSH 0x72be0c`, `CALL 0x404148`) | **Toàn bộ** wire + ghi `+0x12F8/+0x1300` + refresh bank + caption HUD; chuẩn hóa thứ tự `_LStrCatN` |
| 5 | `ts_decompile/functions/005d895c_FUN_005d895c.c` | d.55–70 (`Format('%7d', player+0x12FC/0x12F8)` → `form+0x114/0x118`) | Chứng minh `0x12F8/0x12FC` là **field tiền 7 chữ số** |
| 6 | `ts_decompile/functions/005d7d18_FUN_005d7d18.c` | d.107–121, `Form_Bank_Money`, `btn_Bank_Gold_R/L` | Định danh `gvar_007DA46C` = **TMoneyAccountForm** (form tiền) |
| 7 | `ts_decompile/functions/0072b9b8 / 0072ba0c` (.c) | clamp/sub `player+0x12FC`, max `9 999 999` | Trần giá trị của field tiền |
| 8 | `ts_decompile/functions/00632e3c_FUN_00632e3c.c` | d.58 (`IntToStr(+0x12F8)`), d.93 (`+0x1300`) | Field `+0x1300` cũng là số hiển thị được (paint routine) |
| 9 | `ts_decompile/functions/007b0628_FUN_007b0628.c` | `obj+0xA9 := text≠nil`; `obj+0xAC := text` | Đây là **SetCaption**, không phải log chat |
| 10 | `ts_decompile/functions/0058a964_FUN_0058a964.c` | d.160–190 (`btn_001..btn_009` = `+0x4A…+0x52`, `0x4A*4 = 0x128`) | Định danh `**gvar_007DA530 + 0x128` = **TSe_MainStatus.btn_001** |
| 11 | `ts_decompile/functions/005dd000_FUN_005dd000.c` | d.400 (`_LStrCatN(…,4,"kim",IntToStr(n)," ",<tên>)`) | Đối chứng ngữ nghĩa prefix `"kim"` = nhãn bộ đếm |
| 12 | `ts_decompile/functions/0063c84c_FUN_0063c84c.c` → `007bc1c8_FUN_007bc1c8.c` | d.21 / d.505–517 (timer `+0x4a`, text `FUN_007bc998`) | Phương thức VMT `+0x90` = **banner có thời lượng (ms)** |
| 13 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | d.1264–1266 (`gvar_007DA084=VMT_63B39C_TSe_TalkMsgFormPlus`), d.1495 (`gvar_007DA46C=TMoneyAccountForm`), d.1577 (`gvar_007DA530=TSe_MainStatus`) | Định danh global (nguồn gốc hiểu nhầm "Talk") |
| 14 | `ts_decompile/functions/007ab870_FUN_007ab870.c` | danh sách Callers (không có `0x0079175F`); switch tag d.253–332 | **Bác bỏ**: hàm nạp dòng chat không được OP 0x1A dùng |
| 15 | grep `gvar_007DA1B0` trong `case_functions/functions` | chỉ case 002,003,016,018,020,031,035 | **Bác bỏ**: không form log chat ở case 023 |
| 16 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0x1a: break;` d.939–940; `case 0x37` d.1012–1019; `case 2: break;` d.819–820 | C→S rỗng cho 0x1A; khuôn "op + text" nằm ở opcode khác |
| 17 | `ts_decompile/functions/0051633c_TForm1.CY_AddSedQueue.c`, `00519fe0_TForm1.Button33Click.c` | danh sách file gọi | Mọi frame C→S đi qua SendCommand |
| 18 | `ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c` | d.80–93 (`_LStrCopy(local_c,2,len-1)`, `FUN_0078a89c`) | Quy ước **RestPayload** |
| 19 | `ts_decompile/functions/0077ef7c…c` / `0077eb9c…c` | codec | DWORD LE / Word LE |
| 20 | `ts_decompile/functions/0052bae4_FUN_0052bae4.c` | `TLight_Create(VMT_772E1C)`, `"L10694"`, mảng `+0xFBC` | Xác định "hiệu ứng sáng tại tọa độ" (bỏ qua theo yêu cầu) |
| 21 | `ts_decompile/functions/007a7f20_FUN_007a7f20.c` + grep `"sound\WA0014.wav"` | 30 lượt dùng, vd. `case_020…c:76` | WA0014 = chuông báo chung dụng (bỏ qua) |
| 22 | `ts_decompile/index.csv` (parse `entry_point`+`size`) | **không có** entry chứa `0x72b084, 0x72b170, 0x72c0f0, 0x745acc, 0x745b14, 0x746774, 0x74c5f4, 0x74c64c, 0x74c6b4` | Giới hạn phân tích (helper chưa phục hồi) → mục 6.10 |
| 23 | `ts_decompile/functions/00721088 / 0072ba54 / 0074c4c8 / 00745b58` (.c) | bitmap `player+0x8B2` (idx 1..300), `+0x57C[i]`, `+0x502…0x513` | Xác nhận class của các `func_0x0074…` = **cùng class bản ghi player** (setter số, không phải text) |
| 24 | `.scratch/op-code/handoff-opcode-exploration-guide.md`, `opcode_00_01.md`, `opcode_02.md` | framing, dispatcher, TTalkMsgForm/nhãn kênh, bảng case | Kiến trúc nền + **đối chiếu chéo nghiệp vụ chat (0x02) vs 0x1A** |

**Bản đồ độ tin cậy**: mục 0, 2, 3 (cột field/wire/hàm), 4.1–4.10 (wire + offset ghi), 5 (trạng thái `case 0x1a` rỗng) = **xác minh trực tiếp từ mã nguồn sơ cấp**. Mục 3 (cột nhãn trái/phải), 4.4 (hằng `"kim"`/`0x72BDF8`), 5 (kết luận "client không gửi 0x1A"), và **toàn bộ tên nghiệp vụ của sub-op 1,2,5,6,7,8,9,10** = **suy luận có ràng buộc**, cần dump thêm (mục 6.10) mới chốt được.
