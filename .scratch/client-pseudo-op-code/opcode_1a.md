# PHÂN TÍCH — Main OP 0x1A (Case 23, `FUN_0079175f` @ `0x0079175F`)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)**
Trạng thái: **Wire layout + luồng xử lý xác minh từ mã nguồn sơ cấp** (case handler + dispatcher inline + helper + jump table nhị phân + `FUN_0077f414`). ~~**9 helper dạng `func_0x…` chưa được Ghidra phục hồi body** → phần *tên nghiệp vụ* của các sub-op 1,2,5,6,7,8,9,10 chỉ đạt mức "phân loại theo cơ chế", có ghi chú độ tin cậy ở từng mục.~~ → **CẬP NHẬT 2026-09-14: cả 9 helper đã có body; toàn bộ nhãn HUD đã giải mã từ hex dump — tên nghiệp vụ của sub-op 1,2,3,5,6,7,8,9,10 đã chốt bằng mã nguồn (xem các mục 4.x và bảng nhãn ở 3).**

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). Hai phát hiện lớn: (1) `func_0x0072B084/0x0072B170` **không phải "predicate thử áp dụng theo ID"** — chúng là **cộng/trừ tiền** trên chính field `player+0x12F8` mà sub-op 0x04 set tuyệt đối; (2) nhãn HUD đã dịch được (VISCII — xem đính chính mã hóa ở `opcode_19.md` §6): `0x798038="Nhận được "`, `0x79804C="Tiền"`, `0x79805C="Thất bại  đoạt được"`, `0x798078="kim"`, `0x798084="Giảm"`, `0x798094="Thất bại   giảm thiểu"`, `0x7980B4="Dung lượng tiền bạc không đủ"`, `0x7980DC="Thủy binh"`, `0x7980F0="Tài bảo"`, `0x798100="Điểm đạn dược"` — **0x1A là kênh tiền + bộ đếm tài nguyên, không phải "ID đối tượng"** như phân loại cũ.

---

## 0. Đính chính giả định "Talk / Chat / Thoại NPC" — **SAI**

Handoff (`.scratch/op-code/handoff-opcode-exploration-guide.md`, mục 3, ưu tiên 5) phỏng đoán OP 0x1A = *"Hệ thống chat, thoại NPC, đối thoại sự kiện và trận đấu"*. **Bằng chứng nguồn sơ cấp bác bỏ hoàn toàn:**

| # | Kiểm chứng | Kết quả |
| :-- | :-- | :-- |
| 1 | `case_023` có gọi hàm nạp dòng chat `FUN_007ab870`? | **KHÔNG.** `FUN_007ab870` chỉ được gọi bởi case **002, 003, 016, 018, 020, 031, 035** (grep toàn bộ `case_functions/functions/`). |
| 2 | `case_023` có đụng form log chat `gvar_007DA1B0` (`TTalkMsgForm`, VMT_7AB774)? | **KHÔNG.** Handler chỉ chạm **4 global**: `gvar_007DA7BC` (18×), `gvar_007DA084` (12×), `gvar_007DA2FC` (3×), `gvar_007DA010` (4×). Helper case 4 chạm thêm `gvar_007D9D30`, `gvar_007DA46C`, `gvar_007DA530`. |
| 3 | Có resolve tên người chơi (`FUN_0075ddb8` / `FUN_00722508`)? | **KHÔNG.** Hai hàm này **không xuất hiện** trong `case_023`. *Xác minh thêm 2026-09-14:* cũng không xuất hiện trong **9 helper mới phục hồi** (`0072b084/0072b170/0072c0f0/00745acc/00745b14/00746774/0074c5f4/0074c64c/0074c6b4`) — các hàm này chỉ đụng field số của player/form tiền. |
| 4 | Có field **chuỗi biến độ dài** (nội dung tin) trong payload? | **KHÔNG.** Mọi phép đọc là fixed-width: `4B LE` (sub 1,2,5,6), `4B+4B LE` (sub 4), `2B LE` (sub 8,10), `1B` (sub 9), không đọc gì (sub 3). ~~(Ngoại lệ duy nhất: sub-op `0x07` trao cả `RestPayload` cho parser chưa phục hồi.)~~ **Đính chính 2026-09-14:** helper `0x00746774` đã có body — nó **chỉ đọc 1 byte `RP[1]`** (`00746774_FUN_00746774.c:23–28`), không cắt chuỗi nào. → "0x1A không có chuỗi biến độ dài" nay là khẳng định **tuyệt đối** cho cả 10 sub-op. |
| 5 | Nội dung hiển thị là gì? | Chuỗi **hằng số compile-time** trong code page (`0x798038…0x798100`), **ghép quanh một CON SỐ** bằng `IntToStr`. Đó là nhãn thông báo, không phải nội dung tin nhắn. **Đã giải mã toàn bộ từ `lit_798038…lit_798118.hex` (VISCII) — xem bảng ở mục 3.** |
| 6 | Chiều C→S? | `FUN_0077f414` (`SendCommand`) có `case 0x1a: break;` **rỗng** (dòng 939–940); **0/110** điểm gọi `FUN_0077f414` truyền opcode 0x1A; `TForm1_CY_AddSedQueue` chỉ có 2 điểm gọi ngoài SendCommand (`Button33Click` = keepalive). → **Không có đường "gửi tin chat" bằng OP 0x1A.** |

**Nguồn gốc hiểu nhầm (rất có thể):** OP 0x1A hiển thị bằng **`TSe_TalkMsgFormPlus`** (`gvar_007DA084`, gán tại `0051189c_FUN_0051189c.c:1264-1266`) — tên class có chữ **"Talk"**, nhưng đây là **banner/marquee thông báo** (phương thức VMT `+0x90` = `FUN_0063c84c` → `FUN_007bc1c8(form, text, ms, x, y)` — set text `+0xac`, timer `+0x4a` = số ms, cờ `+0x5e`). Chat log thật là class **khác**: `TTalkMsgForm` (`gvar_007DA1B0`) do **Main OP 0x02** cấp dữ liệu (đã chứng minh trong `.scratch/op-code/opcode_02.md`).

### Kết luận nghiệp vụ đúng theo evidence

> **Main OP 0x1A = kênh S→C "đồng bộ bộ đếm/tài khoản số + thông báo sự kiện bằng banner"** (numeric state sync + reward/limit notification).
> Neo chứng nghiệp vụ **chắc chắn 100%** là **sub-op 0x04**: ghi 2 DWORD vào bản ghi người chơi (`+0x12F8`, `+0x1300`) và **làm mới form tiền `Form_Bank_Money`** (`TMoneyAccountForm`, `gvar_007DA46C`) + **caption nút `btn_001` của HUD `TSe_MainStatus`**.
> ~~Các sub-op còn lại là: setter số (0x05, 0x06), parser record (0x07), và "áp dụng theo ID / theo giá trị → banner + tiếng chuông + hiệu ứng sáng tại chân nhân vật" (0x01, 0x02, 0x08, 0x09, 0x0A), cùng một banner tĩnh (0x03).~~
> **Đính chính 2026-09-14 (xác minh được từ body mới — toàn bộ 9 helper):** 0x01 = **CỘNG tiền** `player+0x12F8` (clamp trần 9 999 999) → banner "Nhận được X Tiền" + chuông + sáng; 0x02 = **TRỪ tiền** cùng field → banner "Giảm X Tiền"; 0x05/0x06 = **cộng/trừ** field thứ hai `player+0x1300` (clamp 0x77359400 / không cho âm) — tức cặp delta của chính field mà 0x04 set; 0x07 = byte tham số + banner tĩnh; 0x08/0x09/0x0A = **tăng bộ đếm có trần** tại `player+0x502/+0x513/+0x50e` (predicate luôn trả 1) → banner "Nhận được X <Thủy binh|Tài bảo|Điểm đạn dược>"; 0x03 = banner tĩnh **"Dung lượng tiền bạc không đủ"**. Không còn nhánh "áp dụng theo ID" nào — giả định cũ bị chính các body mới bác bỏ.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Server đẩy **giá trị số** xuống client; client (a) **áp dụng/ghi** vào bản ghi người chơi `**gvar_007DA7BC` (TPlayer/TWorldPlayer — record rất lớn, có tiền tại `+0x12F8/+0x12FC/+0x1300`, bộ đếm byte/word tại `+0x502/+0x50e/+0x513` với trần động `+0x500/+0x50c`, tọa độ `+0x54/+0x58`, map `+0x63a`, bitmap switch `+0x8b2`, mode theo map `+0x145c`) — cụ thể theo body mới: 0x01/0x02 cộng/trừ `+0x12F8`, 0x05/0x06 cộng/trừ `+0x1300`, 0x08/0x09/0x0A tăng các bộ đếm `+0x502/+0x513/+0x50e`, 0x04 set tuyệt đối `+0x12F8/+0x1300`, (b) **refresh UI tiền/HUD** ở sub-op 4 (và cả 1/2 — caption `btn_001` bị ghi lại sau mỗi lần cộng/trừ), và (c) **hiện banner chữ 2000 ms** `TSe_TalkMsgFormPlus` với nội dung `NHÃN + IntToStr(giá_trị) + NHÃN` (nhãn đã dịch ở mục 3), kèm phát `sound\WA0014.wav` và spawn hiệu ứng sáng `TLight "L10694"` tại vị trí người chơi.
- **Không có**: log chat, tên người gửi, nội dung text, NPC dialog, theo dõi kênh. *(Không còn caveat "0x07 unknown" — xem 4.7.)*
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

| SubOp | Độ dài payload | Field đọc | Hàm hội tụ (body ĐÃ phục hồi) | Thành công → | Thất bại → | Banner? | Sound? | Light@pos? |
| :-: | :-: | :-- | :-- | :-- | :-- | :-: | :-: | :-: |
| `0x01` | 6 | `A = P[2..5]` DWORD LE | `0072b084(player, A)` — **CỘNG tiền `player+0x12F8`** clamp `≤9.999.999`; trả 1 nếu ghi thành công (`0072b084_FUN_0072b084.c:41–63`) | `"Nhận được " + A + "Tiền"` | `"Thất bại  đoạt được" + A + "kim"` | ✔ *(gate `func_0x0072C0F0`)* | ✔ | ✔ *nếu `player+0x145c==2`* |
| `0x02` | 6 | `A = P[2..5]` DWORD LE | `0072b170(player, A)` — **TRỪ tiền `player+0x12F8`** (không cho âm; `0072b170_FUN_0072b170.c:43–60`) | `"Giảm" + A + "Tiền"` | `"Thất bại   giảm thiểu" + A + "kim"` | ✔ *(gate `func_0x0072C0F0`)* | ✖ | ✖ |
| `0x03` | 2 | không đọc | — | hằng `0x7980b4` = **"Dung lượng tiền bạc không đủ"** | — | ✔ (luôn) | ✖ | ✖ |
| `0x04` | 10 | `A = P[2..5]`, `B = P[6..9]` DWORD LE | `FUN_0072bcf8(player, RP)` | ghi `player+0x12F8=A`, `+0x1300=B`; refresh `Form_Bank_Money`; caption `btn_001` | — | ✖ | ✖ | ✖ |
| `0x05` | 6 | `D = P[2..5]` DWORD LE | `00745acc(player, D)` — **CỘNG `player+0x1300`**, chặn nếu tổng `≥0x77359401` (≈2·10⁹), trả 1/0 (`00745acc_FUN_00745acc.c:25–37`) | (setter nội bộ) | — | ✖ | ✖ | ✖ |
| `0x06` | 6 | `D = P[2..5]` DWORD LE | `00745b14(player, D)` — **TRỪ `player+0x1300`**, không cho âm, trả 1/0 (`00745b14_FUN_00745b14.c:26–39`) | (setter nội bộ) | — | ✖ | ✖ | ✖ |
| `0x07` | 3 | `x = P[2]` 1 byte *(đính chính — không còn "cả RP như 1 chuỗi")* | `00746774(player, RP)` — chỉ switch `RP[1]`; **param_1 không dùng** (`00746774_FUN_00746774.c:23–29`) | `x==1` → banner tĩnh `DAT_007467c8` **1500ms** (chưa dump) | `x≠1` → không gì | ✔ *(chỉ x=1)* | ✖ | ✖ |
| `0x08` | 4 | `W = P[2..3]` Word LE | `0074c64c(player, W)` — `ushort +0x502 += W`, **clamp theo trần động tại `+0x500`**; **LUÔN trả 1** (`0074c64c_FUN_0074c64c.c:23–40`) | `"Nhận được " + W + "Thủy binh"` | (bất khả thi — predicate hằng 1) | ✔ (luôn) | ✔ | ✔ (luôn) |
| `0x09` | 3 | `B = P[2]` 1 byte | `0074c5f4(player, B)` — `byte +0x513 += B`, clamp `0xFA`=250; **LUÔN trả 1** (`0074c5f4_FUN_0074c5f4.c:23–40`) | `"Nhận được " + B + "Tài bảo"` | (bất khả thi) | ✔ (luôn) | ✔ | ✔ (luôn) |
| `0x0A` | 4 | `W = P[2..3]` Word LE | `0074c6b4(player, W)` — `ushort +0x50e += W`, **clamp theo trần tại `+0x50c`**; **LUÔN trả 1** (`0074c6b4_FUN_0074c6b4.c:32–51`) | `"Nhận được " + W + "Điểm đạn dược"` | (bất khả thi) | ✔ (luôn) | ✔ | ✖ |
| `0x00`, `≥0x0B` | — | — | — | **no-op** (rơi khỏi switch) | | ✖ | ✖ | ✖ |

**Nhận xét cấu trúc (bằng chứng thêm rằng đây là "kênh thông báo số", không phải chat):**
- Hai hằng `0x798038` (nhánh thành công) và `0x79805c` (nhánh thất bại) **dùng chung cho 4 sub-op 1, 8, 9, 10** → đóng vai trò **mở đầu "thành công / không được"** — xác nhận bằng text dịch: `"Nhận được "` / `"Thất bại  đoạt được"`; mảnh thứ hai (`0x7980dc / 0x7980f0 / 0x798100`) **khác nhau theo từng sub-op** → **danh từ chỉ loại bộ đếm**: `"Thủy binh"` / `"Tài bảo"` / `"Điểm đạn dược"`. Sub-op 2 có cặp mở đầu riêng (`0x798084="Giảm"` / `0x798094="Thất bại   giảm thiểu"`) nhưng **dùng chung mảnh đuôi với sub-op 1** (`0x79804c="Tiền"`, `0x798078="kim"`) → 1 & 2 **đã xác minh được từ body mới là cặp CỘNG/TRỪ cùng một field tiền `+0x12F8`** — không còn là suy luận.
- ~~Ở mọi sub-op có 2 nhánh, "nhánh thất bại chỉ ghép chuỗi rồi bỏ" → code chết~~ — **chính xác hơn từ body mới**: với sub 0x08/0x09/0x0A predicate **luôn trả 1** → nhánh "thất bại" **vĩnh viễn không chạy được**; với sub 0x01/0x02 predicate có thể trả 0 (tràn trần/âm) → nhánh "thất bại" chạy được nhưng **chỉ ghép chuỗi vào slot tạm, không gọi banner** (đúng như nhận xét cũ — xác minh được từ body mới).
- **Bảng nhãn HUD đã giải mã** (VISCII từ `redump/lit_7980xx.hex`; nội dung là bản Việt-hóa máy dịch, giữ NGUYÊN VĂN cả dấu cách thừa):

  | Địa chỉ (content) | Dump | Độ dài (ký tự/byte) | Chuỗi nguyên văn |
  |---|---|---|---|
  | `0x00798038` | `lit_798038.hex` | 10 | `Nhận được ` *(có space đuôi)* |
  | `0x0079804C` | `lit_79804c.hex` | 4 | `Tiền` |
  | `0x0079805C` | `lit_79805c.hex` | 19 | `Thất bại  đoạt được` *(2 space)* |
  | `0x00798078` | `lit_798078.hex` | 3 | `kim` *(ASCII thuần — trùng literal "kim" ở OP 0x1D/sub 0x04)* |
  | `0x00798084` | `lit_798084.hex` | 4 | `Giảm` |
  | `0x00798094` | `lit_798094.hex` | 21 | `Thất bại   giảm thiểu` *(3 space)* |
  | `0x007980B4` | `lit_7980b4.hex` | 28 | `Dung lượng tiền bạc không đủ` |
  | `0x007980DC` | `lit_7980dc.hex` | 9 | `Thủy binh` |
  | `0x007980F0` | `lit_7980f0.hex` | 7 | `Tài bảo` |
  | `0x00798100` | `lit_798100.hex` | 13 | `Điểm đạn dược` |
  | `0x00798118` | `lit_798118.hex` | 20 | `Giao dịch thành công` — **Đính chính: KHÔNG thuộc OP 0x1A.** Hai điểm tham chiếu `0078a89c_FUN_0078a89c.c:4762` và `:4807` đều nằm trong `case 0x1b:` (bắt đầu tại `0078a89c_FUN_0078a89c.c:4740`) — nhãn này của **OP 0x1B**. |
- Các nhãn 7–28 ký tự — **xác minh nhận định cũ "không thể là nội dung tin chat"**. Run dump `lit_798038.hex` còn phủ tiếp các dải hàng xóm đã ghi ở `opcode_1d.md`/`opcode_19.md` (`0x7982B8…0x7982F8` "Lưu trữ/Thất bại lưu trữ/Rút nhận/Thất bại rút nhận"…); **giới hạn còn lại của vùng này: `0x00798314` chỉ nằm trong window 512B tới 4 byte đầu ("Dung", tổng len 30) — chưa đủ dump để dịch**.

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — **CỘNG TIỀN** vào `player+0x12F8` (thành công: banner + chuông + sáng)
**Đính chính lớn (đính chính từ body mới):** tên cũ "Áp dụng theo ID số 32-bit" là **SAI**. `func_0x0072B084` không liên quan gì đến ID — nó là hàm **cộng tiền có clamp** trên đúng field mà sub-op 0x04 ghi tuyệt đối.
**Wire layout** (6 byte payload):
```
P[0]   = 0x1A            MainOp
P[1]   = 0x01            SubOp
P[2..5]= A : DWORD LE    số tiền cộng thêm (FUN_0077ef7c)
```
**Đọc**: `_LStrCopy(RP,2,4,&t)` → `FUN_0077ef7c(ctx,t)` → lưu slot `[EBP-0x3c]` (dòng 34–36).
**Logic** (dòng 37–65):
```c
if (!func_0x0072B084(**gvar_007DA7BC, A))      // body ĐÃ phục hồi — xem dưới
    msg := LBL(0x79805c) + IntToStr(A) + LBL(0x798078);      // "Thất bại  đoạt được" + A + "kim" — không hiển thị
else {
    msg := LBL(0x798038) + IntToStr(A) + LBL(0x79804c);      // "Nhận được " + A + "Tiền"
    if (func_0x0072C0F0()) {                                  // gate — xem ghi chú dưới
        banner(msg, 2000, 0, 0);                              // TSe_TalkMsgFormPlus VMT+0x90
        FUN_007a7f20(gvar_007DA010 + "sound\\WA0014.wav");
        if (*(char*)(player + 0x145c) == 2)                   // mode theo map (đọc từ mapId +0x63a)
            FUN_0052bae4(scene /*gvar_007DA2FC*/, player[0x54], player[0x58]);
    }
}
```
**Body mới `0072b084_FUN_0072b084.c:41–63`** (signature `__register (int player, uint A)`):
1. `cap = 9999999 - player+0x12F8` (d.41–48); nếu `A <= cap`: `player+0x12F8 += A` (d.51–53), cờ trả về = 1 (d.57), **refresh `TMoneyAccountForm`** `FUN_005d895c(*gvar_007DA46C)` (d.58) và **đặt lại caption HUD btn_001**: `_LStrCatN(3)` quanh `IntToStr(player+0x12F8 mới)` với 2 hằng **`0x72B16C` (trái) và `0x72B158` (phải)** (d.59–62; asm `0072b084_FUN_0072b084.asm.txt:34,40` `PUSH 0x72b158 … PUSH 0x72b16c` ⇒ push ngược = push trước là mảnh CUỐI). Ngược lại không ghi gì, trả 0.
2. ⇒ nghiệp vụ: **credit tiền có trần 9.999.999** — trần y hệt clamp của `FUN_0072b9b8` (OP 0x1D), nhưng trên field `+0x12F8` thay vì `+0x12FC`.**Global/offset chạm tới**: `player+0x12F8` (đọc+ghi), `gvar_007DA46C`, `gvar_007DA530+0x128`; hiệu ứng sáng ghi vào `scene+0xFBC + i*4` (mảng 4 `TLight`). Tầng handler không ghi field player nào.
**Đính chính gate `func_0x0072C0F0`**: body mới `0072c0f0_FUN_0072c0f0.c:18–31` là hàm **void, không nhận player** — trả 1 khi **cờ byte `+0x14` của `**gvar_007DA46C` (Form_Bank_Money) và của `**gvar_007DA5A8` đều bằng 0** (d.25–29). Nghĩa là "được thông báo" ⇔ **hai form này không ở trạng thái +0x14≠0** (cờ form không định danh được từ source — chưa kết luận được chính xác là Visible/Modal hay gì). Giả định cũ "cờ trên object người chơi" là **SAI** — hàm chẳng nhận tham số nào.

### 4.2. SubOp `0x02` — **TRỪ TIỀN** khỏi `player+0x12F8` (banner, không âm thanh/hiệu ứng)
Wire giống hệt `0x01`. Khác biệt (dòng 67–93):
- predicate = `func_0x0072B170(player, A)` — body mới `0072b170_FUN_0072b170.c:43–60`: **điều kiện `A <= player+0x12F8`** (không cho âm) → `-= A`, cờ trả về 1, refresh bank + caption HUD với cặp hằng **`0x72B24C` (trái) + `0x72B238` (phải)** (asm `0072b170_FUN_0072b170.asm.txt:31,37`).
- nhãn: thành công `0x798084 + n + 0x79804c` = `"Giảm" + A + "Tiền"`; thất bại `0x798094 + n + 0x798078` = `"Thất bại   giảm thiểu" + A + "kim"`;
- thành công + cờ `func_0x0072C0F0` → **chỉ banner**, không `WA0014.wav`, không `TLight`.
⇒ **Đính chính:** "hai biến thể cùng nghiệp vụ, ưu tiên thấp hơn" nay đã rõ: 0x01/0x02 là **cặp CỘNG/TRỪ cùng field tiền `+0x12F8`** — đối xứng hoàn chỉnh với 0x05/0x06 trên `+0x1300`.

### 4.3. SubOp `0x03` — banner tĩnh **"Dung lượng tiền bạc không đủ"**
**Wire**: `[0x1A][0x03]` (payload 2 byte). Dòng 94–97:
```c
banner(&UNK_007980b4, 2000, 0, 0);   // 0x7980b4 = "Dung lượng tiền bạc không đủ" (28 ký tự, đã dịch)
_LStrLAsg(msg_slot, 0x7980b4);   // dead store
```
Không đọc byte nào. **Đính chính từ hex dump mới:** đây không phải "notice vô nghĩa" chung chung — nội dung là **cảnh báo thiếu tiền/bộ đếm tiền đầy dung lượng** (bản Việt-hóa máy dịch, giữ nguyên văn).

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

### 4.5. SubOp `0x05` — **CỘNG field `player+0x1300`** (im lặng)
**Wire**: `[0x1A][0x05][D:4B LE]` (6 byte). Dòng 101–106. Body mới `00745acc_FUN_00745acc.c:25–38`: `if (player+0x1300 + D < 0x77359401)` thì `player+0x1300 += D`, trả 1; ngược lại không ghi, trả 0. ⇒ **setter delta trên chính field B mà sub-op 0x04 ghi tuyệt đối**, với trần 2.000.000.000 (`0x77359400`). Không banner, không sound, không hiệu ứng. *(Giả định cũ "field `+0x57C+idx*4` của họ `0x00745…`" — **đính chính**: field thật là `+0x1300`.)*

### 4.6. SubOp `0x06` — **TRỪ field `player+0x1300`** (đôi với 0x05)
**Wire**: `[0x1A][0x06][D:4B LE]` (6 byte). Dòng 107–112 → `func_0x00745B14(player, D)`. Body mới `00745b14_FUN_00745b14.c:25–39`: `if (player+0x1300 - D >= 0)` thì `-= D` (không cho âm), trả 1. Cặp setter kề nhau trên `+0x1300` **xác minh được từ body mới** (đúng như phỏng đoán cũ "cặp setter field kề nhau").

### 4.7. SubOp `0x07` — **ĐÍNH CHÍNH: không phải parser record biến độ dài** — chỉ một byte + banner tĩnh
**Wire**: `[0x1A][0x07][x:1B]` (3 byte; guard `len(RP) ≥ 2`). Dòng 113–115 gọi `func_0x00746774(player, RP)`.
Body mới `00746774_FUN_00746774.c:22–29`: guard `BoundErr(1)`; **đọc đúng `RP[1]`**; nếu `== 1` → banner `(**gvar_007DA084, &DAT_007467c8, 0x5dc=1500ms, 0, 0)`; ngoài ra **không làm gì**. `param_1` bị bỏ qua, **không** có `_LStrCopy`/codec/cắt chuỗi nào.
⇒ Toàn bộ tuyên bố cũ "sub-op duy nhất có khả năng mang payload biến độ dài / text có thể nằm ở đây" bị **bác bỏ bằng body**. Không còn nhánh chưa phục hồi nào → phát biểu "**OP 0x1A không có chuỗi biến độ dài trên dây**" trở thành **tuyệt đối** (mọi sub-op 1–10 đã có body).
**Còn thiếu**: nội dung `DAT_007467c8` — **chưa dump** (`lit_7467c8.hex` vắng mặt; dải `0x746xxx` không nằm trong window nào của các dump mới).

### 4.8. SubOp `0x08` — **CỘNG BỘ ĐẾM WORD `player+0x502`** (clamp theo trần động `+0x500`) → banner + chuông + sáng (không gate)
**Wire**: `[0x1A][0x08][W:2B LE]` (4 byte). Đọc: `_LStrCopy(RP,2,2)` → `FUN_0077eb9c` → `& 0xFFFF` (+ `_BoundErr` nếu vượt giới hạn — artefact range-check Delphi). Dòng 116–147:
```c
if (!func_0x0074C64C(player, W))  msg := LBL(0x79805c) + IntToStr(W) + LBL(0x7980dc);   // không hiển thị
else { msg := LBL(0x798038) + IntToStr(W) + LBL(0x7980dc);       // "Nhận được " + W + "Thủy binh"
       banner(msg,2000,0,0); sound(WA0014);
       FUN_0052bae4(scene, player+0x54, player+0x58); }        // KHÔNG gate bởi +0x145c
```
**Đính chính chữ ký**: helper nhận **2 tham số** `(player, W)` — bảng cũ ghi `func_0x0074C64C(player)` là thiếu. Body mới `0074c64c_FUN_0074c64c.c:23–40`: `t = player+0x502 + W`; nếu vượt trần `player+0x500` (ushort) thì set `+0x502 = +0x500` (clamp), không thì `+= W`; **luôn return 1** → nhánh "thất bại" của sub-op này **vĩnh viễn không chạy**. Khác 0x01: **không kiểm tra `func_0x0072C0F0`** → luôn thông báo.

### 4.9. SubOp `0x09` — **CỘNG BỘ ĐẾM BYTE `player+0x513`** (clamp 250) → banner + chuông + sáng
**Wire**: `[0x1A][0x09][B:1B]` (3 byte). **Đọc byte trực tiếp, không `_LStrCopy`** (dòng 149–156):
```c
iVar6 = RP; iVar3 = 1;
if (*(uint*)(iVar6-4) < 2) iVar3 = _BoundErr(1);   // yêu cầu len(RP) ≥ 2
bVar1 = *(byte*)(iVar6 + 1);                        // = P[2]
```
Nhánh thành công: `LBL(0x798038) + IntToStr(B) + LBL(0x7980f0)` = `"Nhận được " + B + "Tài bảo"` → banner + `WA0014.wav` + `TLight` tại tọa độ. Predicate `func_0x0074C5F4(player, B)` — body mới `0074c5f4_FUN_0074c5f4.c:23–40`: nếu `player+0x513 + B < 0xFB` thì `+= B`, ngược lại **set cứng 0xFA (250)**; **luôn return 1** (nhánh thất bại bất khả thi — đính chính cho chữ ký 1 tham số ở bảng cũ).

### 4.10. SubOp `0x0A` — **CỘNG BỘ ĐẾM WORD `player+0x50e`** (clamp theo trần động `+0x50c`) → banner + chuông (không sáng)
**Wire**: `[0x1A][0x0A][W:2B LE]` (4 byte), đọc như 0x08. Predicate `func_0x0074C6B4(player, W)` — body mới `0074c6b4_FUN_0074c6b4.c:32–51`: clamp vào `+0x50c`, **luôn return 1**; thành công: `LBL(0x798038) + n + LBL(0x798100)` = `"Nhận được " + n + "Điểm đạn dược"` + banner + `WA0014.wav`; **không** gọi `FUN_0052bae4`.

> **Ghi chú thứ tự ghép chuỗi**: `A + IntToStr(n) + B` được xác định theo quy ước `_LStrCatN` (varargs push ngược) — **đã kiểm chứng chéo bằng listing thật** `0072bcf8_FUN_0072bcf8.asm.txt:49-58` (`PUSH 0x72bdf8` → `IntToStr` → `PUSH 0x72be0c` ⇒ thứ tự nguồn = `"kim", <số>, 0x72bdf8`). Trong `case_023` Ghidra **mất tham số varargs**, chỉ còn các lệnh gán slot `[EBP-8]`/`[EBP-4]`/`[EBP+0]`; hai hằng luôn **kẹp quanh** `IntToStr`, nhưng **trái/phải của từng hằng có thể đảo** so với bảng ở mục 3. Giá trị địa chỉ là chắc chắn; nhãn trái/phải là **suy luận**. **Bổ sung từ body mới**: các helper HUD caption dùng cùng khuôn và thứ tự đã đọc được từ asm của chúng — `0072b084.asm.txt:35,40` ⇒ caption HUD sau khi cộng tiền = `LBL@0x72B16C + IntToStr(số dư) + LBL@0x72B158`; `0072b170.asm.txt:31,37` ⇒ `LBL@0x72B24C + số dư + LBL@0x72B238`. Bốn hằng này **chưa dump** — chỉ chắc cấu trúc, chưa chắc text.

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

**Độ tin**: `case 0x1a` rỗng là **bằng chứng trực tiếp**; kết luận "client không bao giờ gửi 0x1A" có caveat: (a) `FUN_0077f414` được decompile **có suy hao** (13 jump-table trong `.asm.txt`, riêng `case 0x19` phải redump `subtable_0x7853E5/0x78540F`). ~~(b) 9 method `func_0x…` của class player chưa phục hồi nên về nguyên tắc một builder 0x1A có thể ẩn ở đó~~ — **bổ sung 2026-09-14**: cả 9 helper đã có body và **không hàm nào gọi `FUN_0077f414`** (so từng file), nên caveat (b) đã được loại bỏ; chỉ còn caveat (a). **Không có bằng chứng dương** cho hướng gửi 0x1A.

---

## 6. Ghi chú cho Mock Server

1. **Xử lý OP 0x1A như kênh downlink-only.** Không cần (và không nên) chờ client gửi `0x1A`.
2. **Định dạng frame** (như mọi OP): `[0xF4 0x44][L:Word LE][payload]`, **XOR `0xAD` toàn frame**, `L = độ dài payload`, payload = `[0x1A][SubOp][…]`.
3. **Bảng phát (tóm tắt)**
   - `0x04` — quan trọng nhất, **bắt buộc** nếu mock có shop/bank: `[0x1A][0x04][A:4B LE][B:4B LE]` → `player+0x12F8=A`, `+0x1300=B`. Giữ **A ≤ 9 999 999** để `Form_Bank_Money` hiển thị `%7d` đúng; nếu A=0 thì caption HUD thành `"kim0…"`.
   - `0x03` — notice tĩnh: `[0x1A][0x03]`.
    - `0x01/0x02` — `[…][A:4B LE]` (cộng/trừ `+0x12F8` — banner chỉ khi phép toán hợp lệ **và** gate `0072c0f0` đạt); `0x05/0x06` — `[…][D:4B LE]` (cộng/trừ `+0x1300`, im lặng); `0x08/0x0A` — `[…][W:2B LE]`; `0x09` — `[…][B:1B]` (ba bộ đếm này **luôn** banner vì predicate hằng 1).
    - `0x07` — đặc tả đã chốt: `[0x1A][0x07][x:1B]`; `x=1` → banner tĩnh 1500ms (text `0x7467c8` chưa dump), `x≠1` → no-op. An toàn để replay.
4. **Không gửi `SubOp = 0x00` hoặc `≥ 0x0B`** — không crash nhưng vô nghĩa (rơi khỏi switch). **Không gửi payload chỉ có opcode (`L=1`)** → `RP` rỗng → `_BoundErr(0)` **RangeError trong dispatcher**.
5. **Độ dài phải chính xác**: sub-op 1/2/5/6 yêu cầu **đủ 4 byte** kể từ `P[2]`; 8/10 yêu cầu **đủ 2 byte**; 9 yêu cầu `len(RP) ≥ 2`. Thừa byte → `_LStrCopy` cắt cố định nên **thừa bị bỏ qua** (trừ `0x07`).
6. **Điều kiện để thấy được phản hồi** (với 0x01/0x02): predicate `func_0x0072B084/0x0072B170` phải trả **true** *và* `func_0x0072C0F0` phải khác 0. **Đã xác minh được từ body mới:** điều kiện thứ nhất ⇔ phép cộng không tràn `9 999 999` / phép trừ không làm âm `player+0x12F8`; điều kiện thứ hai ⇔ **cờ `+0x14` của `**gvar_007DA46C` và `**gvar_007DA5A8` đều bằng 0** (hàm không nhận player — `0072c0f0…c:24–30`). ⇒ không thấy banner = do số dư tiền hoặc state hai form, không phải framing. Sub-op 8/9/10 **không** phụ thuộc `func_0x0072C0F0` và **luôn** banner (predicate hằng 1).
7. **Hiệu ứng phụ khi mock phát 0x01/0x08/0x09**: client sẽ **phát `sound\WA0014.wav`** và **bật `TLight "L10694"` tại tọa độ người chơi** (`0x01` chỉ khi `player+0x145c == 2`). Muốn "yên lặng" khi test số liệu → dùng `0x04`, `0x05`, `0x06`.
8. **Không dùng OP 0x1A cho chat/thoại NPC.** Chat & nhãn kênh: **OP 0x02** (đã đặc tả ở `opcode_02.md`). Hội thoại NPC dạng form/nút chọn: không nằm ở 0x1A (handler không đụng form dialog nào; không gọi `FUN_007ab870`/`TTalkMsgForm`).
9. **Thứ tự & nhịp**: client bơm tối đa 50 gói/tick ~30 ms (`TForm1.CY_DelRevQueue`). **Không có ACK** cho OP 0x1A (khác OP 0x00 vốn gửi frame rỗng `FUN_0077f414(.., 0)`).
10. **Checklist "cần bổ sung" — trạng thái 2026-09-14**: (a) ~~`lit_798038…lit_798118.hex`~~ **ĐÃ dump & dịch** (bảng nhãn mục 3; lưu ý decode bằng **VISCII**, không phải cp1258 như ghi chú cũ). (b) ~~decompile/dump asm 9 helper `0x0072B084 … 0x0074C6B4`~~ **ĐÃ có đủ `.c` + `.asm.txt` và đã phân tích ở mục 4**. (c) **VẪN THIẾU**: hằng `0x72BDF8` & `0x72BE0C` (đuôi caption HUD sub 0x04) — kiểm tra mới: `index.csv` 0 entry, `functions/0072bdf8*`/`0072be0c*` không tồn tại ⇒ vẫn là **data trong code page, chưa có hex dump**; mở rộng thêm 4 hằng cùng dạng vừa phát hiện từ asm: `0x72B158/0x72B16C` (caption HUD sau cộng tiền) và `0x72B238/0x72B24C` (sau trừ tiền); cộng thêm `0x7467C8` (banner SubOp 0x07). Có đủ 7 literal này + decompile `00774a84`-họ tra tên item là đóng được 100% nội dung hiển thị của OP 0x1A.

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
| 22 | ~~`ts_decompile/index.csv`: không có entry `0x72b084…0x74c6b4`~~ | **CẬP NHẬT 2026-09-14**: cả 9 helper đều có entry + file `.c`/`.asm.txt` (vd. `index.csv` dòng `FUN_0072b084 … size 190 … OK`) | Giới hạn cũ đã được khôi phục; phân tích ở mục 4 |
| 23 | `ts_decompile/functions/00721088 / 0072ba54 / 0074c4c8 / 00745b58` (.c) | bitmap `player+0x8B2` (idx 1..300), `+0x57C[i]`, `+0x502…0x513` | Xác nhận class của các `func_0x0074…` = **cùng class bản ghi player** (setter số, không phải text) — **xác minh được từ body mới**: `0074c5f4/0074c64c/0074c6b4` đúng là increment clamped trên `player+0x513/+0x502/+0x50e` |
| 24 | `.scratch/op-code/handoff-opcode-exploration-guide.md`, `opcode_00_01.md`, `opcode_02.md` | framing, dispatcher, TTalkMsgForm/nhãn kênh, bảng case | Kiến trúc nền + **đối chiếu chéo nghiệp vụ chat (0x02) vs 0x1A** |
| 25 | `ts_decompile/functions/0072b084_FUN_0072b084.c` + `.asm.txt` | c: d.41–63; asm: d.34/40 (`PUSH 0x72b158`/`PUSH 0x72b16c`) | Sub-op 0x01 = CỘNG `player+0x12F8` clamp 9.999.999 + refresh bank + caption HUD |
| 26 | `ts_decompile/functions/0072b170_FUN_0072b170.c` + `.asm.txt` | c: d.43–60; asm: d.31/37 (`PUSH 0x72b238`/`PUSH 0x72b24c`) | Sub-op 0x02 = TRỪ `player+0x12F8` |
| 27 | `ts_decompile/functions/0072c0f0_FUN_0072c0f0.c` | d.24–30 | Gate banner = cờ `+0x14` của 2 form `gvar_007DA46C`/`gvar_007DA5A8` (hàm void — **đính chính** giả định cũ "cờ trên player") |
| 28 | `ts_decompile/functions/00745acc_FUN_00745acc.c` / `00745b14_FUN_00745b14.c` | d.25–38 / d.26–39 | Sub-op 0x05/0x06 = cộng/trừ `player+0x1300` (trần `0x77359400` / không âm) |
| 29 | `ts_decompile/functions/00746774_FUN_00746774.c` | d.22–29 | Sub-op 0x07 = byte + banner tĩnh `DAT_007467c8` 1500ms (**đính chính**: không phải parser biến độ dài) |
| 30 | `ts_decompile/functions/0074c5f4 / 0074c64c / 0074c6b4` (.c) | d.23–40 / d.23–40 / d.32–51 | Sub-op 0x08/0x09/0x0A = bộ đếm clamp `+0x502(≤+0x500)` / `+0x513(≤250)` / `+0x50e(≤+0x50c)`, **luôn trả 1** |
| 31 | `ts_decompile/redump/lit_798038.hex` … `lit_798118.hex` (11 file) | nội dung tại `0x798038/04C/05C/078/084/094/0B4/0DC/0F0/100/118` | Bảng nhãn HUD mục 3 (VISCII); window `lit_798038/118` còn phủ `0x798138…0x7982F8` (giao dịch/shop — dùng cho các OP lân cận) |
| 32 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c:4740,4762,4807` | nhãn `0x798118` nằm trong `case 0x1b:` | **Đính chính**: `0x798118` thuộc OP 0x1B, không phải OP 0x1A |

**Bản đồ độ tin cậy (cập nhật 2026-09-14)**: mục 0, 2, 3 (field/wire/hàm/**nhãn đã dịch**), 4.1–4.10 (wire + offset ghi + **hành vi 9 helper từ body mới**), 5 (trạng thái `case 0x1a` rỗng; caveat về helper đã gỡ bỏ) = **xác minh trực tiếp từ mã nguồn sơ cấp**. Còn lại mức **suy luận có ràng buộc**: (i) thứ tự trái/phải của hai nhãn quanh `IntToStr` trong các banner (Ghidra mất varargs ở `case_023`), (ii) text 7 hằng đuôi caption/banner `0x72B158/16C/238/24C/DF8, 0x72BE0C, 0x7467C8` chưa dump, (iii) ý nghĩa chính xác cờ `+0x14` của hai form trong gate `0072c0f0`. Tên nghiệp vụ của sub-op 1,2,5,6,7,8,9,10 **không còn là suy luận** — đã chốt bằng field ghi + nhãn dịch (cộng/trừ tiền `+0x12F8`, cộng/trừ `+0x1300`, bộ đếm `+0x502/+0x513/+0x50e`, banner thiếu tiền, byte-trigger).

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x0079175F` — C `client_pseudo_c/case_023_0079175F_FUN_0079175f.c` (có); asm `client_pseudo_c/0079175f_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x1A` — 17 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `1` | `0x007917BA` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x007917BA` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x007917BA` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `2` | `0x007918E3` | Thất bại giảm thiểu | Thất bại giảm thiểu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x007919AF` | Dung lượng tiền bạc không đủ | Dung lượng tiền bạc không đủ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `8` | `0x00791A82` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `8` | `0x00791A82` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `8` | `0x00791A82` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `8` | `0x00791A82` | Thủy binh | Thủy binh. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `9` | `0x00791B93` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `9` | `0x00791B93` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `9` | `0x00791B93` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `9` | `0x00791B93` | Tài bảo | Tài bảo. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `10` | `0x00791C93` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `10` | `0x00791C93` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `10` | `0x00791C93` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `10` | `0x00791C93` | Điểm đạn dược | Điểm đạn dược. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x0079175F`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
