# PHÂN TÍCH — Main OP 0x24 (36) / Case 32 / `FUN_0079346B` @ `0x0079346B`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `switch(SubOp)` đủ `0x01..0x19` (25 nhánh), không khuyết, không default. **8 callee passthrough/setter nay đã có body** (`0052e950/00535310/00537d90/0053c87c/005e1c34/005e20b4/005e2270/005e3ed8`, `index.csv:6315-6318,6353-6356`) — wire của **6 nhánh passthrough (04/05/06/08/09/0C)** cùng đích của 3 nhánh setter (0B/0E → `537D90`, 13 → `535310`) đã bóc được ở §4.4–4.5.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh **fan-out điều khiển UI/manager** — không phải kênh số (không cộng/trừ như OP 0x1A/0x23), không phải chat-log (không gọi `FUN_007ab870`, không chạm `gvar_007DA1B0`).
- 2 họ hành vi:
  - **Exec / setter cố định (15 nhánh)**: `0x01,03,0x0A,0x0B,0x0D,0x0F,0x10,0x11,0x12,0x14,0x15,0x16,0x17,0x18` — gọi `VMT+0x20` hoặc `func_0x00537d90(...,1,0)`, không đọc wire (trừ `0x0E` đọc 1 byte + pad).
  - **Passthrough blob (8 nhánh)**: `0x02,04,05,06,08,09,0x0C,0x19` — trao nguyên `RP` (gồm cả byte SubOp) cho hàm con họ `005e/0053/0052/0051`; **parser của cả 6 nhánh mù còn lại (04/05/06/08/09/0C) nay đã bóc từ body mới** (02 đã rõ từ tầng case, 19 có body cũ) — xem §4.5: 04/05/06/08 đổ vào memo của sub-form `mgr+0x130`, 09 ghi bảng động stride 7 + record 72B `mgr+0x164/0x168`, 0C ghi byte cờ `+0x108` + 4 byte `+0x121..0x124`. `0x02` đặc biệt: copy `P[2..end]` vào `*(gvar_007DA07C)+0x138` rồi gọi `FUN_005e0808`.
- 3 nhánh **đọc field thật ở tầng handler**: `0x07` (clear cờ + banner có điều kiện kép 2 byte), `0x0E` (1 byte + pad), `0x13` (flag 1B + DWORD LE).
- Text duy nhất là hằng `UNK_00798A50` (SubOp 0x07, banner 2000ms). Wire không mang chuỗi biến dài.
- Chiều C→S `case 0x24: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x24 (36) → byte_table[0x78A8EE][0x24] = 0x20 (32)
                 → dword_table[0x78A9B6][32] @ 0x0078AA36 = 0x0079346B
                 → FUN_0079346b (Case 32)
```

- File chính: `ts_decompile/case_functions/functions/case_032_0079346B_FUN_0079346b.c` (188 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5564-5719` (`case 0x24:`) — khớp 1:1.
- Manifest: `case_functions/manifest.csv:34` + `jumptable_0x78A9B6_case_functions.csv:34` (`32,0x0078AA36,0x0079346B`).
- Framing/XOR/pump như `opcode_00_01.md` §2. `.asm.txt` chỉ còn prologue nên mapping xác nhận bằng `.hex`/`.csv` + manifest + bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x24`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 29-36)

```c
if (*(RP-4)==0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){ case 1: ... case 0x19: ... }
// Không default → 0x00 / ≥0x1A no-op (chỉ epilogue _LStrArrayClr/_LStrClr cuối hàm)
```

### 2.4. Codec dùng trong OP

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | Tầng case gọi **1 chỗ**: SubOp `0x13`, 4B → DWORD LE. **Bổ sung từ body mới**: các callee SubOp 04/05/08/09 cũng gọi ef7c nội bộ (DWORD codec) |
| `FUN_0077eb9c` / `FUN_0077f098` | Không gọi trong Case 32; **callee mới gọi**: `eb9c` ở SubOp 06 (`005e20b4.c:52`), `f098` (8B→double) ở SubOp 04/05 (`005e1c34.c:236`, `005e2270.c:129`) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều S→C |
| `_LStrCopy(RP,2,len-1)` | SubOp `0x02`: cắt `P[2..end]` vào `+0x138` |
| `_LStrCopy(RP,3,4)` | SubOp `0x13`: cắt `P[3..6]` decode DWORD |
| Banner `(VMT+0x90)(...,2000,0,0)` | SubOp `0x07` qua `*gvar_007DA084` |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[24][01]` (2B) | không đọc | `(***gvar_007DA07C+0x20)()` |
| `0x02` | `[24][02][data...]` (biến dài) | `_LStrCopy(RP,2,len-1)` → `*(007DA07C)+0x138` | Ghi blob + `FUN_005e0808(*gvar_007DA07C)` |
| `0x03` | `[24][03]` | không đọc | `(***gvar_007DA128+0x20)()` |
| `0x04` | `[24][04]([id:4B][n][str n][b1][b2][D1:4B][D2:4B][dbl:8B][m][str m])×N` | passthrough `RP` | `func_0x005e1c34(*gvar_007DA128, RP)` — lặp record đổ vào memo `mgr+0x130` (§4.5.1) |
| `0x05` | `[24][05][id:4B][time:8B→dbl][text→hết]` | passthrough | `func_0x005e2270(*gvar_007DA128, RP)` — thêm 1 dòng nhật ký + tự cuộn nếu id=self (§4.5.2) |
| `0x06` | `[24][06][W:2B LE]` (4B) | passthrough (callee decode) | `func_005e20b4(*gvar_007DA128, RP)` — chọn/jump mục `W` trên memo (§4.5.3) |
| `0x07` | `[24][07][A:1B][B:1B]` (4B) | `*(007DA128+0x158)=0`; `A=RP[1]` (len≥2), `B=RP[2]` (len≥3) | `A==01 && B==07` → banner `UNK_00798A50` 2000ms; còn lại im lặng |
| `0x08` | `[24][08]([id:4B][n][str n][b0][b1][b2][D1:4B][D2:4B])×N` | passthrough | `func_0x005e3ed8(*gvar_007DA650, RP)` — cùng khuôn memo với 04, record gọn 16+n (§4.5.4) |
| `0x09` | `[24][09][b1][b2][count:W][count×72B block]` | passthrough | `func_0x0053c87c(*gvar_007DA098, RP)` — bảng header stride 7 + mảng record 72B/row (§4.5.5) |
| `0x0A` | `[24][0A]` | không đọc | `(***gvar_007DA098+0x20)()` |
| `0x0B` | `[24][0B]` | không đọc | `func_0x00537d90(*gvar_007D9C8C,1,0)` — **preset mode 1** (§4.5.6) |
| `0x0C` | `[24][0C][f:1B][b1..b4:4B]` (7B) | passthrough (callee đọc RP[1..5]) | `func_0x0052e950(*gvar_007DA2B4, RP)` — cờ `+0x108`, 4 byte `+0x121..0x124`, exec(0x20) (§4.5.7) |
| `0x0D` | `[24][0D]` | không đọc | `(***gvar_007D9ED0+0x20)()` |
| `0x0E` | `[24][0E][v:1B][pad:1B]` (4B) | `v=RP[1]`; guard kép `len≥3` + `len≥2` | `func_0x00537d90(*gvar_007D9C8C, v)` — **setter mode**; pad không truyền param_3 (§4.5.6) |
| `0x0F` | `[24][0F]` | không đọc | `(***gvar_007DA164+0x20)()` |
| `0x10` | `[24][10]` | không đọc | `(***gvar_007D9F34+0x20)()` |
| `0x11` | `[24][11]` | không đọc | `(***gvar_007DA63C+0x20)()` |
| `0x12` | `[24][12]` | không đọc | `(***gvar_007DA13C+0x20)()` |
| `0x13` | `[24][13][flag:1B][A:4B LE]` (7B) | `flag=RP[1]`; `A=FUN_0077ef7c(_LStrCopy(RP,3,4))` = `P[3..6]` | `func_0x00535310(*gvar_007DA604, flag, A)` — `+0x148=flag`, chuyển A (ECX) tiếp `FUN_00535134`, exec(0x20) (§4.5.8) |
| `0x14` | `[24][14]` | không đọc | `(***gvar_007DA114+0x20)()` |
| `0x15` | `[24][15]` | không đọc | `(***gvar_007DA1AC+0x20)()` |
| `0x16` | `[24][16]` | không đọc | `(***gvar_007DA188+0x20)()` |
| `0x17` | `[24][17]` | không đọc | `(***gvar_007DA75C+0x20)()` |
| `0x18` | `[24][18]` | không đọc | `(***gvar_007DA1E8+0x20)()` |
| `0x19` | `[24][19][blob...]` | passthrough | `FUN_0051d914(*gvar_007DA1EC, RP)` |
| `0x00`,`≥0x1A` | — | — | no-op |

---

## 4. Chi tiết các nhánh đáng chú ý (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x02` — Ghi blob + refresh

```c
_LStrCopy(RP,2,LStrLen(RP)-1, *(007DA07C)+0x138); // P[2..end], tối thiểu [24][02] (copy rỗng)
FUN_005e0808(*gvar_007DA07C);
```
Wire biến dài, lấy tới hết, không cắt cố định.

### 4.2. SubOp `0x07` — Clear cờ + banner điều kiện kép

```c
*(007DA128+0x158) = 0;              // luôn clear trước
if (RP[1]==0x01 && RP[2]==0x07)     // guard len≥2 rồi len≥3
  banner(UNK_00798A50, 2000,0,0);
```
Đủ 4B `[24][07][01][07]` mới nổ banner; các cặp khác im lặng nhưng vẫn giữ clear.

### 4.3. SubOp `0x0E` — Set mode (bóc từ body mới)

- Guard kép `len≥3` (`BoundErr(2)`) rồi `len≥2` (`BoundErr(1)`), đọc `v=RP[1]`, gọi `func_0x00537d90(*gvar_007D9C8C, v)`. Thực tế cần 4B `[24][0E][v][pad]`, pad **không đi vào param_3** của callee (case chỉ set EAX/EDX — xem §4.5.6; param_3 chỉ có nghĩa khi `v==2` và khi đó là **thanh ghi ECX rác tại call-site** — chưa kết luận được chủ đích).

### 4.4. SubOp `0x13` — Nhánh số duy nhất ở tầng case (bóc tới đích setter)

- `flag=RP[1]` + `A = FUN_0077ef7c(P[3..6])` (7B payload), gọi `func_0x00535310(*gvar_007DA604, flag, A)`. Thiếu byte trong A → `BoundErr` trong codec.
- **Body mới** `00535310_FUN_00535310.c` (53B): `*(byte*)(mgr+0x148) = flag` (`:19`), gọi `FUN_00535134(mgr, flag, A)` — A được **truyền tiếp qua ECX** (asm `00535310_FUN_00535310.asm.txt:13-16`: `MOV ECX,[EBP-0xC]; CALL 0x535134`) — helper `00535134` **vẫn chưa có body** → số phận cuối của A chưa kết luận được; rồi `VMT+0x20` exec (`:21`).

### 4.5. Phân tích 8 body callee mới (passthrough/setter)

Quy ước record: offset tính theo **RP** (0-based; `RP[0]=SubOp`, vòng lặp callee bắt đầu tại `RP[1]=P[2]`), DWORD/Word codec = `FUN_0077ef7c`/`eb9c`, double = `f098`, ctx `gvar_007D9D30`.

#### 4.5.1. SubOp `0x04` — `func_0x005e1c34(*gvar_007DA128, RP)` (973B, `index.csv:6353`)

- Reset sub-form `mgr+0x130`: gọi `VMT+0x7C` của nó + `VMT+0x40` của 2 điều khiển con `+0x160/+0x178` (`005e1c34_FUN_005e1c34.c:112-114`).
- Lặp record tới hết RP (`:115-117,293`), mỗi record **24+n+m byte** (`:269-285`): `[0..3 id DWORD][4 n][5..4+n str1 n bytes][5+n b1][6+n b2][7+n..10+n D1 DWORD][11+n..14+n D2 DWORD][15+n..22+n double][23+n m][24+n..23+n+m str2 m bytes]`.
- Hiển thị: `double → DateTimeToString("yyyy/m/d ampmhh:mm")` (`:286`) → set điều khiển `mgr+0x130 → +0x160` (`:288`); str1 → điều khiển `+0x178` (`:290`); str2 (shortstring cap 60 — `_PStrNCpy 0x3c`, `:267-268`) + cặp byte `b1,b2` → `func_0x005e2f28(list, str2, 0, &b1)` (`:292`) — add dòng memo.
- **Kết luận cơ chế**: 04 = danh sách bản ghi "nhật ký/giao dịch" đổ vào memo của form `007DA128`. Đối tượng 4 (`D1`,`D2`) và cặp `b1/b2` **chưa kết luận được nghiệp vụ**.

#### 4.5.2. SubOp `0x05` — `func_0x005e2270(*gvar_007DA128, RP)` (2253B, `index.csv:6355`)

- **Một record duy nhất**: `[id:4B][time:8B→double][text→hết RP]` (`005e2270_FUN_005e2270.c:125-140`), text cắt 60 (`:141-142`), time format `"yyyy/m/d ampmhh:mm"` (`:145`).
- Nếu `id == *(player+4)`: 2 số nguyên phụ dựng bằng `CatN(9)`→`StrToInt` từ **chuỗi byte của chính player** (`+0x9B..0xA3` cho số thứ nhất `:152-180`; `+0xA4..0xA9` cho số thứ hai — decompile lặp lại vài byte `+0xA4..0xA6` ở cuối do artifact varargs, `:182-211`); ngược lại tra cache tên `FUN_00722508(*gvar_007D9C48,id)` (trần 0x834) rồi dựng từ **entry `gvar_007DA6BC`** (`+0x42..0x4A` và `+0x4B..0x50`, cùng artifact) (`:214-393`); id lạ → cặp giá trị `0x1A7DAF1C` (hằng fallback, `:220-221`).
- Trần memo: `FUN_007bb730(list) > 499` → gọi `VMT+0x74(list,0)` trước khi add (`:396-400`).
- Add dòng text (`func_0x005e2f28(list, text, 0)` — Ghidra mất arg 4 kiểu `&byte` so với bản 0x04, `:403`), set `+0x160 := chuỗi ngày` (`:406`) và `+0x178 := <mất trong decompile — unaff_EDI>` (`:409`), nếu self → `FUN_007bb9d4(list, count-5)` — **tự cuộn xuống cuối** (`:410-421`), clear cờ `mgr+0x158` (`:422`), `FUN_005e1ae0(mgr)` refresh (`:424`).

#### 4.5.3. SubOp `0x06` — `func_0x005e20b4(*gvar_007DA128, RP)` (288B, `index.csv:6354`)

- `W = Word(RP[1..2])` (`005e20b4_FUN_005e20b4.c:51-52`).
- Đọc số mục điều khiển `+0x178` (`VMT+0x14`) trừ chỉ nhớ `+0xE4` của nó; `<6` → cờ "gần cuối" (`:54-62`).
- `VMT+0x74(*(mgr+0x130), W)` — **chọn/jump tới mục W** (`:63`).
- Nếu gần cuối → `FUN_007bb9d4(list, count-5)` cuộn (`:64-72`); clear `mgr+0x158`; `FUN_005e1ae0` (`:73-74`).
- **Xác minh được từ body mới**: 06 không phải blob — payload đúng 4B `[24][06][W lo][W hi]` (thiếu byte → BoundErr trong codec). Cross-reference thú vị: `mgr+0x158` cũng chính là cờ mà SubOp `0x07` clear ở tầng case (§4.2) — cùng một form.

#### 4.5.4. SubOp `0x08` — `func_0x005e3ed8(*gvar_007DA650, RP)` (727B, `index.csv:6356`)

- Cùng khuôn memo với 04 nhưng **không double, không str2**: record **16+n byte**: `[0..3 id D][4 n][5..4+n str][5+n b0][6+n b1][7+n b2][8+n..11+n D1][12+n..15+n D2]`, lặp tới hết (`005e3ed8_FUN_005e3ed8.c:79-81,84-198`).
- Mỗi record: `+0x160 := IntToStr(b0)` (`:192-194`), `+0x178 := &DAT_005e420c` (**hằng mới lộ, chưa dump**, `:196`), add dòng str + cặp `(b1,b2)` qua `func_0x005e2f28(list,str,0,&b1)` (`:197`).

#### 4.5.5. SubOp `0x09` — `func_0x0053c87c(*gvar_007DA098, RP)` (795B, `index.csv:6318`)

- `blob = RP[1..end]` (`0053c87c_FUN_0053c87c.c:83-89`) tách qua `FUN_0077f298` (**raw copier 200-byte chunks, không codec** — `0077f298_FUN_0077f298.c:78-116`) vào buffer stack; header blob: `[b1:1B][b2:1B][count:W]` (`:91`, đọc tại `local_1cad/local_1cac/local_1cab`).
- Số dòng hiện có = `Length(*(mgr+0x168))` (`FUN_00405b1c`, `:92-97`); tìm row khớp `(b1,b2)` trong **mảng header stride 7 byte** tại `*(mgr+0x164)` (`:112-131`).
  - Chưa có row → `FUN_0053bab8(mgr,b1,b2,count)` cấp row + set `mgr+0x149=1` (`:134-147`).
  - Đã có → xóa row dữ liệu (`FUN_00405cd8` finalize `RTTI_53B988_DynArray__4`) và ghi `row[2]=count` (`:148-174`).
- `count > 0`: copy **count block 72 byte (9 DWORD)** từ phần đuôi blob vào mảng động của row (`mgr+0x168[slot]`), trần 100 record (guard index 99, `:175-234`).
- Refresh `FUN_0053c6f4(row, len)` (`:253`); khi số row `mgr+0x149` đạt `slot+1` → `FUN_0053d194(mgr, ...)` (`:254-264`).
- **Kết luận cơ chế**: 09 = nạp **bảng 2 chiều loại (b1,b2) × tối đa 100 record 36 DWORD**. Nghiệp vụ cụ thể của bảng **chưa kết luận được**.

#### 4.5.6. SubOp `0x0B` & `0x0E` — `func_0x00537d90(*gvar_007D9C8C, mode[, p3])` (122B, `index.csv:6317`)

- `mgr+0x1DA = mode` (`00537d90_FUN_00537d90.c:21`).
- `mode==1`: `+0x1DB=0`, `+0x1D8=0x10`, `+0x1D9=0x15` (`:22-26`) → **SubOp 0B chính xác là preset này** (call-site `(mgr,1,0)`).
- `mode==2`: `+0x1DB=p3`, `+0x1D8=0x15`, `+0x1D9=0x1A` (`:27-31`) → với SubOp 0E (`v=2`), `p3`=ECX không được case set → **hành vi phụ thuộc rác thanh ghi — chưa kết luận được; các v khác (≠1,2): chỉ ghi +0x1DA**.
- Kết: `VMT+0x20` exec (`:32`). Cặp giá trị `0x10/0x15` và `0x15/0x1A` gợi ý 2 **config độ dài/số dòng** của form (pattern byte đôi tại +0x1D8/+0x1D9) — suy luận, chưa có bằng chứng tên.

#### 4.5.7. SubOp `0x0C` — `func_0x0052e950(*gvar_007DA2B4, RP)` (200B, `index.csv:6315`)

- `f = RP[1]` (guard `len(RP)>=2`) → `*(byte*)(mgr+0x108) = f` (`0052e950_FUN_0052e950.c:44-49`).
- `f != 0`: `_FillChar(mgr+0x120, 5, 0)` (`:51`); copy **4 byte `RP[2..5]`** → `mgr+0x121..0x124` (guard `len>=6`) (`:53-72`); `VMT+0x20` exec (`:73`).
- **Wire**: `[24][0C][f][b1 b2 b3 b4]` = 7B payload khi `f≠0`; `f==0` chỉ cần 3B. **Đính chính bảng cũ** "blob biến dài không đặc tả": đây là setter có layout cố định, không phải parser mù.

#### 4.5.8. SubOp `0x13` — xem §4.4 (body `00535310` đã bóc tới `FUN_00535134` — helper cuối vẫn vắng body).

---

## 5. Chuỗi VISCII → UTF-8

- **Đính chính từ body mới**: các SubOp 04/05/08 **có mang chuỗi text trên dây** (`str1/str2` của 04, `text` đuôi của 05, `str` của 08) — đưa thẳng vào memo qua `_LStrFromString`/`func_0x005e2f28` (`005e1c34.c:267-268,291-292`; `005e2270.c:141-142,403`; `005e3ed8.c:109,197`) → nhiều khả năng là VISCII như các chuỗi hiển thị khác, nhưng **chưa có sample live để xác nhận encoding**. SubOp 09 chỉ mang binary blob; 0C chỉ byte.
- Literal duy nhất ở tầng case: `UNK_00798A50` (banner SubOp 07). `redump/` batch mới **vẫn không có** dump cho địa chỉ này → chưa decode được. Cần redump `.rodata` tại `0x00798A50`.
- Hằng mới lộ từ callee: `DAT_005e420c` (nhãn gắn vào control `+0x178` mỗi record SubOp 08, `005e3ed8.c:196`) — **chưa có dump**.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:968-969`: `case 0x24: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x24 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[24][01] / [03] / [0A] / [0D] / [0F] / [10] / [11] / [12] / [14]–[18]  exec tĩnh (2B)
[24][02][blob...]            blob P[2..end] → +0x138 + refresh
[24][04][record...]          id+n+str1+b1+b2+D1+D2+double+m+str2, 24+n+m B/record (×N)
[24][05][id u32][time 8B][text]  nhật ký 1 dòng, cap memo 500
[24][06][W u16LE]            chọn mục W trên memo (4B payload)
[24][08]([id][n][str][b0][b1][b2][D1][D2])×N  16+n B/record
[24][09][b1][b2][count u16][count×72B]  nạp bảng động (cap 100 block)
[24][0C][f][b1 b2 b3 b4]     f: cờ +0x108; f!=0: 4 byte +0x121..124 + exec
[24][19][blob...]            passthrough (FUN_0051d914 — format theo body cũ)
[24][07][01][07]             banner 798A50 2000ms; cặp khác im lặng (vẫn clear)
[24][0B]                     preset mode 1 (+0x1DA=1,+0x1D8=0x10,+0x1D9=0x15)
[24][0E][v][pad]             setter mode v; pad KHÔNG đi vào param_3
[24][13][flag][A u32LE]      đủ 7B; +0x148=flag, A chuyển tiếp ECX→FUN_00535134
ĐỪNG GỬI: [24][00], ≥0x1A (no-op); L=1 (RangeError).
```

Độ dài: exec tĩnh 2B; Sub 07 cần `len(RP)≥3`; Sub 0E cần 4B payload; Sub 13 cần đủ DWORD `P[3..6]`; Sub 02 lấy tới hết; Sub 06 cần đủ 4B payload (Word tại `RP[1..2]`); Sub 0C cần 3B (f==0) / 7B (f≠0); Sub 09 cần ≥ 5B header blob; record 04/08 phải đủ trường (thiếu → BoundErr/IntOver bên callee).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_032_0079346B_FUN_0079346b.c` (188 dòng) | Handler chính đủ 25 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5564-5719` | Bản inline đối chiếu 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x24]=0x20`) + `jumptable_dword200_0x78A9B6.hex` (`dw[32]=0x79346B`) + `.csv:34` + `manifest.csv:34` | Mapping tự parse |
| 4 | `functions/0077ef7c / 0077eb9c / 0077eb1c / 0077ee84 / 0077f098` | Codec tầng case (chỉ `EF7C` gọi ở Sub 0x13); callee mới dùng thêm `eb9c/f098` — xem §2.4 |
| 5 | `functions/0077f414_FUN_0077F414.c:968-969` | C→S rỗng |
| 6 | `jumptable_0x78A9B6_cases.c:6557` + grep `00798a50` | Literal duy nhất tầng case, xác nhận thiếu dump |
| 7 | `functions/0052e950 / 00535310 (+.asm.txt:13-16) / 00537d90 / 0053c87c / 005e1c34 / 005e20b4 / 005e2270 / 005e3ed8` + `index.csv:6315-6318,6353-6356` | **Mới**: 8 body callee — wire §4.5 |
| 8 | `functions/0077f298_FUN_0077f298.c:71-91` | **Mới**: helper tách blob 200B của SubOp 09 (raw copier, không codec) |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] `FUN_00535134` (đích nhận `A` của SubOp 13) vẫn chưa có body → số phận `A` chưa chốt.
- [ ] `FUN_0053bab8`, `FUN_0053c6f4`, `FUN_0053d194`, `FUN_005e1ae0`, `FUN_005e2f28`, `FUN_007bb730/7bb9d4/ba180` — các helper UI của nhóm 04–09: chỉ dùng để mô tả cơ chế, chưa đọc body.
- [ ] Semantics: cặp config `0x10/0x15` vs `0x15/0x1A` (0B/0E), bảng stride 7 + block 72B (09), D1/D2 (04/08), chuỗi thuộc tính `+0x9B..` (05) — **chưa kết luận được nghiệp vụ**.
- [ ] Redump: `0x00798A50` (banner 07), `0x005E420C` (nhãn 08), và nếu cần `0x005E2F28`-liên quan hằng format memo.
- [ ] SubOp 19: `FUN_0051d914` đã có body từ trước nhưng chưa được đối chiếu lại trong batch này.

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x0079346B` — C `client_pseudo_c/case_032_0079346B_FUN_0079346b.c` (có); asm `client_pseudo_c/0079346b_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x24` — 1 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `7` | `0x0079359F` | Bị cấm gửi lời nhắn | Bạn bị cấm gửi tin nhắn. | Hiệu đính ngữ nghĩa/câu chữ |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x0079346B`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
