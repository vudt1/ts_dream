# PHÂN TÍCH — Main OP 0x3C (60) / Case 53 / FUN_0079575c @ 0x0079575C

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) là chiều chính**. Chiều C→S: **Mâu thuẫn "bản C `case 0x3c:` rỗng vs ASM có body" ĐÃ CHỐT (2026-09-14)** — bảng dispatch `0x77F474`/`0x77F53C` đã redump: `byte[0x3C]=0x32 ≠ 0`, `dword[0x3C]=0x0078A142 ≠ 0` ⇒ **builder C→S 0x3C CÓ THẬT**; bản C `break;` là artifact của decompiler.
Trạng thái: **Đã xác minh toàn bộ phần lõi từ SSOT** (`ts_decompile/`).
**BỐN method callee của handler (`FUN_00541098`, `FUN_005408f4`, `FUN_0053fd78`, `FUN_0053f8fc`) ĐÃ ĐƯỢC DECOMPILE (2026-09-14)** — HOLE `0x0053F8FC..0x005418F8` đã lấp; phân tích field/wire từng SubOp tại §4.2. **Đối tượng đích `gvar_007DA778` ĐÃ CÓ DÒNG GÁN trực tiếp** (`0055374c_FUN_0055374c.c:41–44` — tạo `TRE_ZMChessMain` qua `VMT_53D8B8_TRE_ZMChessMain`) ⇒ **suy luận tên class §4.1 chính thức được XÁC MINH BẰNG CODE**, không còn là suy luận.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form (chỉ nhắc 1 dòng khi bắt buộc).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3C là opcode **điều khiển form "ZMChess Main"** — instance toàn cục `gvar_007DA778`, **tên class `TRE_ZMChessMain` XÁC MINH TỪ CODE** (không còn suy luận): `0055374c_FUN_0055374c.c:41–44` tạo qua `TRE_ZMChessMain_Create(VMT_53D8B8_TRE_ZMChessMain, 1, param_3)` khi OP 0x39 mở mode 3 (form chính của **minigame Cờ Tướng / Chinese Chess**, §4.1).
- **Payload S→C tối thiểu 1 byte điều khiển**: `RestPayload[0]` = SubOp. Handler **không `switch`** mà dùng chuỗi `if (==1) else if (==2) else if (==3) else if (==4)`; **không có `default`**.
- **4 hành vi hiệu lực** — mỗi SubOp gọi **một method của `TRE_ZMChessMain`** truyền nguyên `RestPayload` làm tham số (callee tự rẽ nhánh tiếp theo `RP[1]` — byte ngay sau SubOp — xem §4.2):
  - `SubOp == 0x01` → `FUN_00541098(gvar_007DA778, RP)` (dòng 29) — đồng bộ bàn/bước cờ/lịch ván.
  - `SubOp == 0x02` → `FUN_005408f4(gvar_007DA778, RP)` (dòng 32) — cập nhật trạng thái cờ + banner người vào/rời bàn.
  - `SubOp == 0x03` → `FUN_0053fd78(gvar_007DA778, RP)` (dòng 35) — điều khiển pha ván đấu (bắt đầu/hết giờ/kết quả/penalty/time-out đối thủ).
  - `SubOp == 0x04` → `FUN_0053f8fc(gvar_007DA778, RP)` (dòng 38) — **bảng 13 mã thông báo lỗi/chú thích** qua banner `gvar_007DA084+0x90` (1500 ms).
- **Cả 4 method callee ĐÃ CÓ BODY (2026-09-14)** — `functions/00541098_FUN_00541098.c` (521 dòng), `005408f4_FUN_005408f4.c` (475 dòng), `0053fd78_FUN_0053fd78.c` (739 dòng), `0053f8fc_FUN_0053f8fc.c` (84 dòng) + `.asm.txt` từng hàm; wire layout sau `RP[0]` khôi phục ở §4.2. (Lưu ý: metadata `Size:` trong header Ghidra của `005408f4/0053fd78` nhỏ hơn thân thực tế — boundary Ghidra cắt tại khe chuỗi code-gap; phân tích theo thân decompile.)
- **Cổng chặn độ dài**: `RestPayload` rỗng (payload `[3C]`, L=1) → `_BoundErr(0)` = **ERangeError** (dòng 22-25). Không có chốt `< n` nào khác.
- **Bối cảnh họ opcode (đối xứng đã xác minh từ S→C)**: bốn opcode liền kề `0x3A/0x3B/0x3C/0x3D` lần lượt điều khiển **bốn singleton** cùng một hệ form-game:
  | MainOp | Case | Handler | Singleton đích |
  | :---: | :---: | :-- | :-- |
  | `0x3A` | 51 | `FUN_007956b9` | `gvar_007DA42C` (state type 1) |
  | `0x3B` | 52 | `FUN_007956f3` | `gvar_007DA0F4` (state type 2) |
  | **`0x3C`** | **53** | **`FUN_0079575c`** | **`gvar_007DA778` (state type 3)** |
  | `0x3D` | 54 | `FUN_007957dc` | `gvar_007D9F98` (state type 4) |
  - Ánh xạ "state type 1..4" xác minh tại `functions/00553410_FUN_00553410.c:29-41` và `functions/00553840_FUN_00553840.c:31-42`.
  - Riêng OP `0x39` (case 50) set `*(gvar_007DA778 + 0x38) = 100/1000` (`case_functions/functions/case_050_00795579_FUN_00795579.c:53-58`), củng cố rằng `gvar_007DA778` là một form-game cùng cụm.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (đã kiểm lại từng mắt xích)

```
MainOp 0x3C (60) → byte_table[0x78A8EE][0x3C] = 0x35 (53)
                 → dword_table[0x78A9B6][53] @ 0x0078AA8A = 0x0079575C
                 → FUN_0079575c (Case 53)
```

- Bảng byte 200: `redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** ứng với index `0x30..0x3F`: `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Byte thứ 13 của hàng (offset `0x3C`) = **`0x35` = 53** (kiểm chéo bằng python: `vals[0x3C] = 0x35`).
- Bảng dword: `redump/jumptable_0x78A9B6_case_functions.csv:55` → `53,0x0078AA8A,0x0079575C,YES,"FUN_0079575c",0x0079575C,1`.
- Đối chiếu hex thô: `redump/jumptable_dword200_0x78A9B6.hex` **hàng 14**, entry #53 = bytes `5C 57 79 00` → **`0x0079575C`** (LE). (Địa chỉ entry = `0x78A9B6 + 53*4 = 0x0078AA8A`.)
- Manifest: `case_functions/manifest.csv:55` → `53,0x0078AA8A,0x0079575C,EXPORTED,"FUN_0079575c","0079575c",1,"functions/case_053_0079575C_FUN_0079575c.c",`.
- File case chính: `case_functions/functions/case_053_0079575C_FUN_0079575c.c` (71 dòng); lõi logic tại **dòng 20-39**.
- Bản gộp jump table: `case_functions/jumptable_0x78A9B6_cases.c:8797` (header "Case index: 53"), hàm tại dòng **8803**, lõi logic tại **dòng 8815-8834**.
- Bản inline trong dispatcher tổng: `functions/0078a89c_FUN_0078a89c.c:6959-6986` (nhãn `case 0x3c:`) — **khớp 1:1** (§2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `handoff-opcode-exploration-guide.md:16-19` và `opcode_00_01.md` §2. Frame: `[Token 2B: F4 44][Length L: Word LE][Payload L Bytes]`, toàn khung XOR `0xAD`.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x3C` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile: `unaff_EBP + -0xc` = con trỏ dữ liệu `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP-4)` = **độ dài** `RestPayload` (length prefix của AnsiString).

### 2.3. Đọc SubOp (`case_053` dòng 20-27)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // iVar2 = RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {             // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                      // → ERangeError (payload [3C], L=1)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
iVar2 = *(int *)(unaff_EBP + -0x14);         // iVar2 = SubOp (so sánh dạng int)
if (iVar2 == 1) {                            // chuỗi if/else if, KHÔNG switch
  func_0x00541098(*(undefined4 *)gvar_007DA778,*(undefined4 *)(unaff_EBP + -0xc));
}
else if (iVar2 == 2) { ... }                 // dòng 31-33
else if (iVar2 == 3) { ... }                 // dòng 34-36
else if (iVar2 == 4) { ... }                 // dòng 37-39
```

Diễn giải:
- `RestPayload` rỗng → `_BoundErr(0)`; `iVar2 = extraout_EDX`/`iVar1` là đường phục hồi giả của decompiler cho nhánh exception (thực tế là lỗi range). **L=1 → ERangeError.**
- `SubOp = RP[0]`, so sánh `== 1..4` (không `switch`), **không có `default`**.
- **Không đọc/parse thêm field nào ở handler** — toàn bộ `RestPayload` (kể cả byte SubOp) được chuyển nguyên cho callee. Layout field phía sau `RP[0]` do callee parse — **đã khôi phục §4.2 (2026-09-14)**.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** ở handler |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** ở handler |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Không gọi** (handler không cắt field chuỗi) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 23) |
| `gvar_007DA778` | Con trỏ singleton `TRE_ZMChessMain` (đối tượng đích, §4.1) |
| `func_0x00541098` | Callee SubOp 1 (dòng 29) — **ĐÃ CÓ BODY §4.2** (`00541098_FUN_00541098.c`) |
| `func_0x005408f4` | Callee SubOp 2 (dòng 32) — **ĐÃ CÓ BODY §4.2** (`005408f4_FUN_005408f4.c`) |
| `func_0x0053fd78` | Callee SubOp 3 (dòng 35) — **ĐÃ CÓ BODY §4.2** (`0053fd78_FUN_0053fd78.c`) |
| `func_0x0053f8fc` | Callee SubOp 4 (dòng 38) — **ĐÃ CÓ BODY §4.2** (`0053f8fc_FUN_0053f8fc.c`) |
| `_LStrArrayClr/_LStrClr` (dòng 43-67) | **Dọn dẹp frame cục bộ** của dispatcher (giống hệt `case_051/052/054`), **không liên quan wire** |

> Không có helper giải mã nhị phân nào được gọi trong handler: 4 SubOp chỉ là **tín hiệu**; việc parse nằm trong callee — **callee parse bằng `_LStrCopy` + codec `FUN_0077ef7c` (4B→DWORD LE) và đọc byte trực tiếp (§4.2)**.

### 2.5. Đối chiếu file case vs bản inline (1:1)

| Vị trí | `case_053_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 20 `iVar2 = *(int *)(unaff_EBP + -0xc)` | dòng 6961-6962 `iVar21 = local_10` |
| Check rỗng | dòng 22 `if (*(int *)(iVar2 + -4) == 0)` | dòng 6963 `if (*(int *)(local_10 + -4) == 0)` |
| SubOp | dòng 26 `... = *(byte *)(iVar2 + iVar1)` | dòng 6969 `cVar5 = *(char *)(iVar21 + iVar20)` |
| SubOp1 call | dòng 29 `func_0x00541098(*(undefined4 *)gvar_007DA778, ...)` | dòng 6972 (giống) |
| SubOp2 call | dòng 32 `func_0x005408f4(...)` | dòng 6976 (giống) |
| SubOp3 call | dòng 35 `func_0x0053fd78(...)` | dòng 6980 (giống) |
| SubOp4 call | dòng 38 `func_0x0053f8fc(...)` | dòng 6984 (giống) |

Khác biệt **duy nhất**:
1. Tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX` vs `iVar20/iVar21/extraout_EDX_x00179`) và nhãn SEH marker (`&UNK_0079576f/9b/af/d7`).
2. Bản inline dùng `local_10` (alias của `RestPayload`) và `cVar5` (char) thay vì `*(uint *)`.

**Logic trùng khớp hoàn toàn.** Cả hai bản **không** gọi `_BoundErr` cho các SubOp khác.

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler **không dùng `switch`** mà `if (==1) else if (==2) else if (==3) else if (==4)`; **không có `default`**.

| SubOp (RP[0]) | Payload S→C | Điều kiện đọc | Core logic |
| :---: | :--- | :--- | :--- |
| *(rỗng)* | `[3C]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **ERangeError** (dòng 22-25) |
| `0x01` | `[3C][01]` | `SubOp = 1` | **`FUN_00541098(gvar_007DA778, RP)`** (dòng 29) — đồng bộ bàn/nước đi/người chơi, switch tiếp `RP[1]` (§4.2) |
| `0x02` | `[3C][02]` | `SubOp = 2` | **`FUN_005408f4(gvar_007DA778, RP)`** (dòng 32) — cập nhật trạng thái + banner vào/rời ghế (§4.2) |
| `0x03` | `[3C][03]` | `SubOp = 3` | **`FUN_0053fd78(gvar_007DA778, RP)`** (dòng 35) — máy trạng thái pha đấu, đồng hồ, phạt (§4.2) |
| `0x04` | `[3C][04]` | `SubOp = 4` | **`FUN_0053f8fc(gvar_007DA778, RP)`** (dòng 38) — 13 mã banner thông báo, 1500 ms (§4.2) |
| `0x00`, `>= 0x05` | `[3C][xx]` | không khớp `1..4` | **no-op im lặng** |
| `0x01..0x04` + byte dư | `[3C][01..04][...]` | khớp `1..4` | gọi callee với **toàn bộ RP**; parse chi tiết theo `RP[1]` (cmd) — đã khôi phục §4.2 |

**Tổng: 4 nhánh có hiệu lực** (`0x01..0x04`) + **1 dạng gây ERangeError** (`[3C]`, L=1). Mọi SubOp khác là no-op.

---

## 4. Chi tiết core logic

### 4.1. Đối tượng đích `gvar_007DA778`

- **Call form**: `func_0x00xxxxxx(*(undefined4 *)gvar_007DA778, *(undefined4 *)(unaff_EBP + -0xc))` — tham số 1 là con trỏ object, tham số 2 là `RestPayload`.
- **Bằng chứng `gvar_007DA778` là một singleton form-game** (state type 3):
  - `functions/00553410_FUN_00553410.c:38-41` — nhánh `bVar1 == 3` → `TObject_Free(*(int **)gvar_007DA778); *(undefined4 *)gvar_007DA778 = 0;` (giải phóng form theo state type).
  - `functions/00553840_FUN_00553840.c:40-42` — `bVar1 == 3` → `FUN_00541c6c(*(uint *)gvar_007DA778);` (update/timer form).
  - `functions/0055391c_FUN_0055391c.c:40-42` — `bVar1 == 3` → `FUN_00541f8c(*(uint *)gvar_007DA778);`.
  - `functions/005538cc_FUN_005538cc.c:30-32` — `cVar1 == 3` → `FUN_00541f78();`.
- **Định danh class = `TRE_ZMChessMain` — XÁC MINH TRỰC TIẾP (2026-09-14)**:
  - **Dòng gán tìm thấy**: `0055374c_FUN_0055374c.c:41–44` — `piVar3 = TRE_ZMChessMain_Create((int *)VMT_53D8B8_TRE_ZMChessMain, 1, param_3); *(int **)gvar_007DA778 = piVar3;` (nhánh `mode==3` của dispatcher tạo scene theo OP 0x39, caller `case_050_00795579_FUN_00795579.c:39`). Tên symbol + VMT do Ghidra đặt từ RTTI — **tên class đúng, không phải suy luận**.
  - Các suy luận cũ vẫn đứng vững và nay là nhất quán: vùng code `0x0053DB88..0x00541F8C` là subsystem ZMChess (`0053db88_TRE_ZMChessMain.Create.c`, `0053f450_FUN_0053f450.c:60–70` tạo `TRE_ZMChessPlayer`), cleanup type-3 `00541c6c_FUN_00541c6c.c:187` phát `"Sound\\WA0045.wav"`, `gvar_007DA778 + 0x38 = 100/1000` (OP 0x39, `case_050...c:53-58`).
  - **Ghi chú thêm từ 4 callee mới**: cùng class còn có các alias global `DAT_00948dcc` / `DAT_00948dd0` / `DAT_00948dd4` được dùng trực tiếp như instance con trỏ trong `00541098.c:151–156,503` (field `+0x10/+0x18/+0x98[·]/+0x144/+0x148` của chúng trùng bộ field ZMChess) — nhiều khả năng là biến trỏ-chùng-cùng-object của form bàn cờ và form con; chưa kết luận được chính xác từng cái.
- **Grep nơi ghi field trên các body mới (2026-09-14)**: `+0x34` — duy nhất `005418f8.c:51,63` (AnsiString, clear `:67`, xem §6.2); `+0x38` (100/1000) — vẫn chỉ `0078a89c_FUN_0078a89c.c:6891,6894` (= inline OP 0x39, cùng nguồn `case_050` đã cite) — 4 callee + các hàm ZMChess mới (`00541c6c/00541f8c/00541bf8/005419c8` đã decompile) **không ghi `+0x38`**; no new writer.

### 4.2. Bốn callee — PHÂN TÍCH TỪ BODY MỚI (2026-09-14)

Cả 4 hàm đều có signature `(Self: TRE_ZMChessMain, RP: AnsiString nguyên)` và **tự switch trên `cmd = RP[1]`** (byte ngay sau SubOp; chỉ số là 0-based trong bài này). Kí hiệu `board = *(Self+0x20)` — con trỏ **bàn cờ phụ** (field: `+8` ghế đang tới lượt, `+0x10` trạng thái/bước, `+0x11`, `+0x12` quân được chọn, `+0x14[1..5]`, `+0x1a[1..5]` mã quân theo vị trí). `Now()` = timer; các hàm refresh `FUN_0053f88c` (redraw) / `FUN_00541bf8` / `FUN_0053da48` / `FUN_0053d930` / `FUN_005419c8` thuộc cùng subsystem.

**`FUN_00541098` — SubOp 0x01: đồng bộ bàn & nước cờ** (`00541098_FUN_00541098.c:99–513`, switch `RP[1]`):
| `RP[1]` | Hành vi (field ghi) | Line |
| :-- | :-- | :-- |
| 1 | `Self+0x1d := RP[2]`; `Self+0xac[1..4] := RP[3..6]` (4 tọa độ nước đi); `board+0x12 := +0xac[board+8]`; `RP[7] → board+0x1a[board+0x12]`; `board+0x10 := 1`; redraw; `DAT_00948dcc+0x10 := Now`, `+0x18 := 5` | `:100–152` |
| 2 | lời chess-nước kiểu bảng: `FUN_005465dc(DAT_00948dd4,1,0)`; `FUN_0053e2b4(Self, board+8, RP[2], RP, DAT_00948dd4+0x148)`; `DAT_00948dd4+0x144 := 1` | `:154–176` |
| 3 | như cmd 1 nhưng `Self+0x1c := RP[2]`, `board+0x10 := 3` | `:178–230` |
| 4 | clear `board+0x14[1..5] := 0` | `:232–242` |
| 5 | **nước đi có quân**: như cmd 1 + `Self+0x4d[Self+0x1d] := RP[7]` (mảng mã quân 33 phần tử); `board+0x10 := 5`; `RP[9] → board+0x11`, `board+0x1a[board+0x11] := 0`; `Now → Self+0x40`; `Self+0x31 := Self+0x1e`; `Self+0x1e := RP[8]`; `FUN_0053da48`; **play `"Sound\\WB0007.wav"`** (move sound); cần `len(RP) ≥ 10` | `:244–318` |
| 7 | `Self+0x1e := Self+0x4b := RP[2]`; `board+0x1a[1..4] := RP[3..6]`; `Now → Self+0x40` | `:320–351` |
| 9 | `board+8 := RP[2]` (đổi ghế tới lượt) | `:353–360` |
| 0xB | **đồng bộ danh sách 4 người chơi**: `board+8 := RP[2]`; ghế mình `gvar_007DA7BC^+4 → Self+0x70[i*4]`, tên `+9 → Self+0x98[i*4]` (`_LStrFromString`), `gvar_007DA7BC^+0x12F8 → Self+0x84[i*4]`; vòng 4 ghế: `FUN_00541bf8`… đọc từng **2 × 4-byte LE** qua codec `FUN_0077ef7c` (`_LStrCopy(RP,5,4)` / `(RP,9,4)`) → `Self+0x70[i]` = player ID, `Self+0x84[i]`; tra actor slot `FUN_0070c20c(gvar_007D9D34^, id)` (cận 800 — cùng cơ chế slot actor `opcode_03/04.md`), lấy tên `gvar_007DA300^+slot*4+9 → Self+0x98[i]`; tên rỗng → **`"Gamer" + IntToStr(i)`** (literal tại `:479`); `Self+0xb1[i] := 1` | `:362–494` |
| *(đuôi)* | nếu `RP[1]==5`: clear cờ `+0x59` của 4 quân (`DAT_00948dd0+0x1a0+i*4`); ngược lại `FUN_005419c8(Self, board)`; nếu `RP[1] ∈ 1..5`: `board+0x13 := 0` — **chính latch "đã gửi 0x3C" của `FUN_005418f8` (§6.2) được server reset** | `:496–513` |

**`FUN_005408f4` — SubOp 0x02: cập nhật trạng thái bàn + banner vào/rời** (`005408f4_FUN_005408f4.c:98–468`, switch `RP[1]`): các cmd 1/3/5/7/9/10:
| `RP[1]` | Hành vi | Line |
| :-- | :-- | :-- |
| 1 | `Self+0x1d := RP[2]`; `+0xac[1..4] := RP[3..6]`; **`Self+0x101[Self+0x1e*3] := 1`** (bảng cờ 3 entry/ghế) | `:99–137` |
| 2 | như 0x01-cmd2: `FUN_0053e2b4(Self, RP[2], RP[3], RP, DAT_00948dd4+0x148)` | `:138–162` |
| 3 | `Self+0x1c := RP[2]`; `+0xac[1..4]`; clear 4 quân `+0x59`; **`board+0x20[1..5] := 2`** | `:163–216` |
| 5 | quân đi: `+0x4d[+0x1d] := RP[7]`; `Now→+0x40`; `+0x31:=+0x1e`; `+0x1e := RP[8]`; clear `Self+0xfc[+0x30][+0x31] := 0` (bảng record 3 entry/ghế stride 2); `FUN_0053da48`; `"Sound\\WB0007.wav"` | `:217–290` |
| 7 | `Self+0x1c := RP[2]`; `+0xac[1..4]`, và khi `+0xac[i] ≠ 0` → `Self+0xfc[+0x1e][i] := 1`; `"Sound\\WA0017.wav"` | `:292–348` |
| 9 | **một ghế nhận người**: `i := RP[2]`; `RP[3..6]` → `Self+0x70[i]` (player ID, 4B LE qua `FUN_0077ef7c`), `RP[7..10]` → `Self+0x84[i]`; tên từ actor slot (`FUN_0070c20c`/`gvar_007DA300`), rỗng → `"Gamer"+i`; `+0xb1[i] := 1`; **banner = text `[DAT_00948dcc+0x98[i]] + &DAT_00541068` (1500 ms, vtable+0x90 `gvar_007DA084`)** — mẫu chuỗi thông báo chưa dump | `:350–433` |
| 10 | ghế rời: `+0xb1[i] := 2`; banner `+ &LAB_00541084` (1500 ms); giảm counter `Self+0x49` | `:435–464` |

**`FUN_0053fd78` — SubOp 0x03: máy trạng thái pha đấu** (`0053fd78_FUN_0053fd78.c:99–510`; gate `RP[1] ∈ 1..9` → `Self+8 := RP[1]`, `Self+0x3d := 1`):
- cmd **1/7**: bắt đầu ván — `Self+0x1c9 := 0`; (7: `FUN_0053f2a8`); `FUN_0054698c(DAT_00948dd4,1)`; `Self+0x24 := RP[2]`; bảng đồng hồ 7 dòng `+0x158[1..7]` (mã), `+0x159[·] := 1`, `+0x15b[·] := Now`, `+0x163[·] := 1000` (ms); gọi vtable `DAT_00948dd0+0x24/+0x20` (show/hide); clear `_FillChar(Self+0xfc[i],6,0)` 4 ghế (`:103–170`).
- cmd **3**: kết/trừ giờ — `+0x31 := +0x1e`; `+0x1e := RP[2]`; `FUN_0053da48`; nếu `DAT_00948dd0[99]+0x58 == 5` và `board+0x12 ≠ 0`: trừ **5 + 2·n** đơn vị giờ từng quân `DAT_00948dd0[+0x62+i]` (`:172–230`), redraw.
- cmd **5/6**: kết quả — clear record `_FillChar(+0xfc[i],6,0)`; (5: `FUN_0053ee00`; 6: đặt đồng hồ dòng 1 := 4, `Self+0x4a := 7`); cập nhật Now từng dòng active; clear `+0x59` 4 quân; đọc **mảng n_i = RP[·] rồi n_i mã quân** ghi `board+0x1a[·]` (ghế mình) / `Self+0xfc[ghế][·]`; nếu `Self+0x11b[i] == 6` → copy `Self+0x4d[+0x1d]` vào `board+0x1f`/`Self+0x101[i]` và đặt `+0xac[i] := 5`; redraw; **phát WAV theo `switch(Self+0x4a)` với 8 tên chuỗi tại khe mã `DAT_00540854…DAT_005408e8` — CHƯA DUMP** (`:488–510`).
- cmd **8**: timeout/phạt ghế khác — nếu `i := RP[2]` ∈1..4 và ≠ ghế mình: `FUN_005465dc`, `DAT_00948dd4+0x144 := 1`, `FUN_0053ec8c(Self, i)`, chạy đồng hồ, clear `+0x59`, parse 4 ghế từ offset RP[3], `FUN_0053ee00` (`:512–649`).
- cmd **9**: reset quân — `+0xac[1..4] := 4`; `_FillChar(+0xfc[i],5,1)` ghế khác; `Self+0x1c := 0x10`; nếu chưa kết (`DAT_00948dd0[99]+0x58 ≠ 5`): `Now→Self+0x28`, `FUN_0053d930`, cộng **5+2·n** giờ từng quân, `+0x58 := 5` (`:651–729`).

**`FUN_0053f8fc` — SubOp 0x04: bảng mã thông báo (banner)** (`0053f8fc_FUN_0053f8fc.c:28–80`): đọc `code = RP[1]`; với `code ∈ {1..6, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0xFF}` phát `gvar_007DA084^.vtable+0x90(text, 1500ms, 0, 0)` với text là một trong **13 con trỏ chuỗi `DAT_0053FB4C…DAT_0053FD5C` (khe trong code-gap của chính hàm này — chưa redump)**; `code` ngoài tập → im lặng. Đây là kênh "lý do từ chối/cảnh báo bàn cờ" (ý nghĩa từng mã chưa kết luận được vì thiếu chuỗi).

**Bảng SubOp cập nhật (thay cho §3 các dòng "unknown")**: xem 4 bảng trên; các điều kiện độ dài tối thiểu thực tế từng cmd đều do callee kiểm `_BoundErr` (client exception, không crash) — ví dụ SubOp 1 cmd 5 yêu cầu `len(RP) ≥ 10` (`00541098.c:292–316`); SubOp 2 cmd 9 yêu cầu `len(RP) ≥ 11` (`005408f4.c:369–376` cắt `RP[7..10]`).

### 4.3. Vì sao không còn nhánh nào khác

- `case_053` chỉ có `if (==1)` / `else if (==2)` / `else if (==3)` / `else if (==4)`; **không `else`, không `switch`, không `default`** → mọi SubOp khác rơi thẳng xuống epilogue (dòng 40-68) → **im lặng**.
- Epilogue `_LStrArrayClr/_LStrClr` (dòng 43-67) là **dọn dẹp stack cục bộ dùng chung cho mọi case** (giống hệt `case_051` dòng 33-57 và `case_054` dòng 102-130), **không phải logic nghiệp vụ**.

### 4.4. Ghi chú về SEH marker

- Các nhãn `&UNK_0079576f`, `&UNK_0079579b`, `&UNK_007957af`, `&UNK_007957d7` (bản inline `0078a89c...c:6964,6971,6975,6979`,6983) là **SEH handler label** của frame dispatch, **không phải string constant** → không cần decode VISCII.

---

## 5. Chuỗi VISCII → UTF-8

- **Handler `FUN_0079575c` vẫn KHÔNG tham chiếu string constant** — chỉ có `gvar_007DA778` và 4 con trỏ hàm; `UNK_0079576f/9b/af/d7` là **nhãn SEH** (§4.4). Bảng `lit_*.hex` không chứa `0x007957xx`.
- **ĐÍNH CHÍNH 2026-09-14 — 4 callee CÓ chứa chuỗi** (handler passthrough nên trước đây không thấy):
  - **ASCII literal nằm ngay trong code** (không cần redump): `"Gamer"` (`00541098.c:479`, `005408f4.c:410` — fallback tên người chơi), `"Sound\\WB0007.wav"` (`00541098.c:318`, `005408f4.c:290` — sound đi quân), `"Sound\\WA0017.wav"` (`005408f4.c:348`), `"Sound\\WA0045.wav"` (cleanup `00541c6c.c:187`).
  - **13 con trỏ chuỗi banner trong code-gap của `FUN_0053f8fc`**: `DAT_0053FB4C, 053FB68, 053FB94, 053FBAC, 053FBC8, 053FBFC, 053FC18, 053FC44, 053FC70, 053FCB4, 053FCE0, 053FD28, 053FD5C` (một cho mỗi mã 1..6, 0xB..0x10, 0xFF — `0053f8fc.c:31–79`) — dải `0x0053FB4C..0x0053FD6C` **chưa có `lit_*.hex`** → cần redump (đây là bảng "mã lỗi bàn cờ" SubOp 4).
  - **8 tên WAV theo `switch(Self+0x4a)` trong `FUN_0053fd78`**: `0x00540854…0x005408E8` (`0053fd78.c:490–508`) — cùng loại gap, **chưa dump**.
  - 2 chuỗi ghép banner SubOp 2: `&DAT_00541068` / `&LAB_00541084` (`005408f4.c:424,455`) — sau `FUN_00541098`, code-gap, **chưa dump**.

> **Kết luận §5 (mới)**: bản thân packet OP 0x3C **không mang chuỗi**; nhưng các handler con chứa nhiều chuỗi tĩnh (banner mã lỗi, tên WAV, hậu tố "Gamer"). Muốn đọc nội dung banner 0x3C-SubOp4 → redump dải `0x0053FB4C–0x0053FD6C` (VISCII).

---

## 6. Chiều Client → Server (C→S)

### 6.0. CHỐT MÂU THUẪN BẰNG BẢNG ĐỎ (2026-09-14)

`redump/table_0x77F474_200B.hex` (bảng byte C→S, 200 B) và `redump/table_0x77F53C_dword200.hex` (200 dword) — parse python, little-endian:

| Index | byte @ `0x77F474` | dword @ `0x77F53C` | Interpretation |
| :-- | :-- | :-- | :-- |
| 0x3A | `0x30` | `0x00789F7A` | builder case 0x3A (khớp C `case 0x3a` có body) |
| 0x3B | `0x31` | `0x00789FDA` | builder case 0x3B (khớp C) |
| **0x3C** | **`0x32` (≠ 0)** | **`0x0078A142` (≠ 0)** | **builder case 0x3C TỒN TẠI — C `break;` là artifact decompiler** |
| 0x3D | `0x33` | `0x0078A318` | builder case 0x3D (khớp C `case 0x3d` 7-byte) |

⇒ **Kết luận §3.1 cũ ("cần redump bảng để chốt") đã đóng**: C→S OP 0x3C **CÓ THẬT**, với entry `0x0078A142` nằm giữa entry 0x3B (`0x00789FDA`) và 0x3D (`0x0078A318`) — tức chính khối asm `0077f414_FUN_0077F414.asm.txt:3686–3765` (mô tả §6.2). Builder thật **tự dispatch tiếp theo byte CL**: `MOV AL,[EBP-0x6]; DEC AL; JZ 0x007892D1; DEC AL; JZ 0x0078933C; DEC AL; JZ 0x00789391; JMP <return>` (asm:3686–3693) — **chỉ CL ∈ {1,2,3} mới gửi** (nhảy tới 3 body dùng chung với builder `0x32` ở `0x7892B9–0x7893E5`; payload tự lấy op từ `[EBP-0x5] = DL` nên một body phục vụ cả hai opcode), CL khác → trở về không gửi gì.

### 6.1. Bản C decompile: `case 0x3c:` RỖNG (artifact — đã giải thích)

`functions/0077f414_FUN_0077F414.c:1042-1043` (trong `switch(param_2 & 0xff)`):

```c
case 0x3c:
  break;
```

→ **Theo bản C decompile, client KHÔNG dựng payload nào cho opcode 0x3C** (rỗng). Nếu chỉ dựa vào file C, kết luận là **C→S KHÔNG tồn tại**.

### 6.2. Bản ASM: các body C→S thật (đã gán nhãn nhờ bảng §6.0)

**Call-site gửi 0x3C ĐÃ TÌM THẤY trong export (2026-09-14)** — hai sender, hết nghi vấn "call-site nằm trong HOLE":

1. **`FUN_005418f8`** (ngay mép phải của HOLE cũ, giờ có body — `005418f8_FUN_005418f8.c` + `.asm.txt`): method của `TRE_ZMChessMain` (caller: `0x542add/542b2d/542b7d/542be1/542d4d` — click bàn cờ). Logic: latch `board+0x13 == 0` (đặt 1 — chính latch mà SubOp 1 §4.2 reset khi server ack); nếu `param_3 ∈ 1..4`: **`Self+0x34 := _LStrFromChar(char tại (DAT_00948dcc^+0x20)^+0x10)`** (1 ký tự); nếu `param_3 == 5`: `Self+0x34 := shortstring [0x01, byte board+0x11]` (2 ký tự) qua `_PStrNCat`; rồi `SendCommand(gvar_007D9D30^, op=0x3C)` với **`CL = 1`** (`005418f8_FUN_005418f8.asm.txt:62–64`: `MOV CL,0x1; MOV DL,0x3c; CALL 0x0077f414`); cuối cùng `_LStrClr(Self+0x34)`. ⇒ **field `+0x34` được CHỨNG MINH là AnsiString** (`_LStrFromChar/_LStrFromString/_LStrClr` trên chính nó — `.c:51,63,67`), hết suy luận kiểu.
2. **`FUN_00546798`** (`00546798_FUN_00546798.c:22–29`): gửi `SendCommand(0x3C)` với **`CL = 2`** (`00546798...asm.txt:10–12`: `MOV CL,0x2; MOV DL,0x3c`), không đụng `+0x34`; kèm `FUN_00545d0c`, vtable `+0x24`, `DAT_00948dcc^+0x1c9 := 1`.

Khối asm `0077f414_FUN_0077F414.asm.txt:3694-3721` (trích ở dưới) là **body CL=1** của builder — khớp đối xứng: sender CL=1 duy nhất là `FUN_005418f8`, và sender này vừa ghi `Self+0x34` vừa gọi gửi, còn builder đọc đúng `[Self+0x34]` tại asm:3715. Hai khối gửi thô `[op][CL]` 2-byte tại asm:**3722–3743** và **3744–3765** nằm cùng vùng builder 0x3C/0x3D; body 0x7892D1/0x78933C/0x789391 (CL=1/2/3 dùng chung với op 0x32) nằm trong vùng builder 0x32. (Ghidra `case_032` C-truncated — xem `opcode_32.md`; mapping dòng↔địa chỉ tuyệt đối không recover được 100% từ asm.txt vì file không kèm cột address, nhưng **vị trí region đã chốt bằng bảng dword**.)

Body asm `3694–3721` (nguyên văn cũ, nay là **builder xác minh được**, payload = `[0x3C][CL=1]` + toàn bộ text của `Self+0x34`):

```asm
3694 LEA EAX,[EBP + -0x34]
3695 MOV DL,byte ptr [EBP + -0x5]      ; DL = OpCode (= 0x3C)
3696 MOV byte ptr [EAX + 0x1],DL
3697 MOV byte ptr [EAX],0x1             ; buf = [01][OpCode]
3698 LEA EDX,[EBP + -0x34]
3699 LEA EAX,[EBP + -0x38]
3700 CALL 0x00402b90                    ; FUN_00402b90 (copy length-prefixed)
3701 LEA EAX,[EBP + -0x3c]
3702 MOV DL,byte ptr [EBP + -0x6]      ; DL = CL (tham số thứ 3)
3703 MOV byte ptr [EAX + 0x1],DL
3704 MOV byte ptr [EAX],0x1             ; buf2 = [01][CL]
3705 LEA EDX,[EBP + -0x3c]
3706 LEA EAX,[EBP + -0x38]
3707 MOV CL,0x2
3708 CALL 0x00402b60                    ; @PStrNCat → [02][OpCode][CL]
3709 LEA EDX,[EBP + -0x38]
3710 LEA EAX,[EBP + 0xfffff7dc]
3711 CALL 0x0040402c                    ; @LStrFromString (shortstring → AnsiString)
3712 LEA EAX,[EBP + 0xfffff7dc]         ; dest
3713 MOV EDX,dword ptr [0x007da778]     ; EDX = gvar_007DA778
3714 MOV EDX,dword ptr [EDX]            ; EDX = Self (TRE_ZMChessMain)
3715 MOV EDX,dword ptr [EDX + 0x34]     ; EDX = Self.field_34 (AnsiString)
3716 CALL 0x00404090                    ; @LStrCat(dest, field_34)
3717 MOV EDX,dword ptr [EBP + 0xfffff7dc]
3718 MOV EAX,[0x007da664]
3719 MOV EAX,dword ptr [EAX]
3720 CALL 0x0051633c                    ; TForm1.CY_AddSedQueue → gửi
3721 JMP 0x0078a4f2                      ; return
```

Dịch ngược:
- `[0x007da778] → [EDX] → [EDX+0x34]`: lấy **trường `+0x34` của `TRE_ZMChessMain`** và nối trực tiếp vào payload qua `@LStrCat` (khác `case 0x3a/0x3b` dùng `FUN_0077ee84` để mã hoá DWORD → 4 byte LE; ở đây `+0x34` được đối xử như **AnsiString/PChar**).
- Payload suy ra: **`[OpCode][CL][bytes của Self+0x34]`**.

### 6.3. Gán nhãn block ASM 3694: ĐÃ XÁC MINH (không còn là "đối xứng suy luận")

Trước 2026-09-14, block `3694-3721` chỉ được gán cho `case 0x3C` bằng **lập luận đối xứng + loại trừ**; block nằm đúng trong vùng `[dword[0x3C], dword[0x3D]) = [0x0078A142, 0x0078A318)` và **call-site `FUN_005418f8` (CL=1) vừa ghi `Self+0x34` vừa gửi 0x3C** — trùng khít với việc block đọc `[Self+0x34]` (`asm:3715`). Đối chiếu vùng đã xác minh bằng python trên bảng dword:

| MainOp | S→C handler dùng singleton | C→S body (theo asm) dùng singleton |
| :---: | :-- | :-- |
| `0x3A` | `gvar_007DA42C` | `asm:3616-3651` → `gvar_007DA42C+0xc` (khớp C `case 0x3a`) |
| `0x3B` | `gvar_007DA0F4` | `asm:3652-3685` → `gvar_007DA0F4+0x28` (khớp C `case 0x3b`) |
| **`0x3C`** | **`gvar_007DA778`** | **`asm:3694-3721` → `gvar_007DA778+0x34`** |
| `0x3D` | `gvar_007D9F98` | `asm:3768-3855` → `gvar_007D9F98+0x6a..0x6e` (khớp C `case 0x3d`) |

- Ba opcode `0x3A/0x3B/0x3D` khớp **hoàn hảo** giữa hai chiều (bản C cũng xác nhận 0x3A/0x3B/0x3D). **Đối tượng `gvar_007DA778` (state type 3) chỉ xuất hiện duy nhất tại block asm 3694** — và **bảng `0x77F53C` xác nhận entry builder 0x3C = `0x0078A142` nằm chính trong vùng block này** (§6.0).
- ⇒ Bản C decompile đã **bỏ sót nhãn** cho `case 0x3c` (Ghidra xuất `break;` — giống đúng loại lỗi đã ghi nhận cho `case 0x32` ở `opcode_32.md`). **Kết luận CHỐT: C→S 0x3C TỒN TẠI** với payload `[0x3C][CL][Self.field_34]` (CL=1) hoặc `[0x3C][CL]` thuần (CL=2/3).

### 6.4. Kết luận C→S (ĐÃ XÁC MINH — cập nhật 2026-09-14)

```
Payload C→S (CL=1, từ FUN_005418f8): [0x3C][0x01][bytes AnsiString *(*(gvar_007DA778)+0x34)]   (1–2 ký tự mã quân)
Payload C→S (CL=2, từ FUN_00546798): [0x3C][0x02]                                              (2 byte, theo builder body)
CL khác {1,2,3}: builder quay về, KHÔNG gửi (asm:3686–3693)
```

- ~~Mâu thuẫn SSOT~~ **ĐÃ GIẢI QUYẾT**: bảng đỏ `0x77F474`/`0x77F53C` (dump 2026-09-14) chứng minh entry 0x3C khác 0 ⇒ C `break;` là lỗi decompiler (§6.0).
- ~~`CL` và call-site `SendCommand(0x3C)` unknown~~ **ĐÃ TÌM THẤY**: `MOV DL,0x3c` xuất hiện trong đúng 2 asm export — `005418f8_FUN_005418f8.asm.txt:62–64` (`CL=1`) và `00546798_FUN_00546798.asm.txt:10–12` (`CL=2`).
- Kiểu field `+0x34`: **AnsiString — xác minh bằng `_LStrFromChar`/`_LStrFromString`/`_LStrClr` tại `005418f8.c:51,63,67`** (không còn "suy luận từ @LStrCat").

---

## 7. Ghi chú cho Mock Server

### 7.1. S→C (server gửi xuống client)

Payload sau MainOp; khung ngoài = `[Token F4 44][Length L: Word LE][Payload]`, **toàn khung XOR 0xAD** (L tính từ byte MainOp `0x3C`):

```
[3C]          L=1  → ERangeError (BoundErr(0))       — ĐỪNG GỬI
[3C][00]      L=2  → no-op im lặng
[3C][01]      L=2  → FUN_00541098(TRE_ZMChessMain, RP) — callee đọc RP[1] (cmd): L=2 chỉ vừa đủ để vào switch, cmd nào cũng BoundErr bên trong (exception SEH swallow — không crash)
[3C][02]      L=2  → FUN_005408f4(TRE_ZMChessMain, RP)   — như trên
[3C][03]      L=2  → FUN_0053fd78(TRE_ZMChessMain, RP)   — gate cmd 1..9 tại RP[1]
[3C][04]      L=2  → FUN_0053f8fc(TRE_ZMChessMain, RP)   — cmd=RP[1] ∈ {1..6,B..10,FF} → banner; thiếu → BoundErr swallow
[3C][xx>=05]  L=2  → no-op im lặng
Gói mẫu CÓ HIỆU LỰC (từ §4.2):
  [3C][01][09][seat]                L=4  → đặt ghế tới lượt (board+8)
  [3C][01][0B][seat][id:4B LE][pts:4B LE]×4   L≥... → đồng bộ 4 người (id tra actor, tên rỗng → "Gamer<i>")
  [3C][01][01][side][x1 y1 x2 y2][ma]  L=10 → bước chuẩn bị nước
  [3C][01][05][side][x1 y1 x2 y2][maQuan][?][seatNext]  L=11 (RP[9]) → đi quân + sound WB0007
  [3C][04][01]                      L=3  → banner mã lỗi 1 (text chưa dump)
  [3C][03][01][?][?x4 tọa độ]       L=8  → bắt đầu ván (đồng hồ 1000ms)
```

**Ví dụ khung hoàn chỉnh** (đã XOR):

```
# [3C][01]:
payload  = 3C 01
L (LE)   = 02 00
pre-XOR  = F4 44 02 00 3C 01
on-wire  = 59 E9 AF AD 91 AC

# [3C][02]:
on-wire  = 59 E9 AF AD 91 AF
# [3C][03]:
on-wire  = 59 E9 AF AD 91 AE
# [3C][04]:
on-wire  = 59 E9 AF AD 91 A9

# [3C] (L=1) — GÂY ERangeError, ĐỪNG GỬI:
on-wire  = 59 E9 AC AD 91
```

- **Bắt buộc `L ≥ 2`**: nếu chỉ gửi `[3C]` (L=1) → `_BoundErr(0)` (delphi exception) ngay trong handler.
- **Hiệu ứng từng SubOp ĐÃ KIỂM CHỨNG TỪ BODY (§4.2)** — không còn phải "gửi dò UI": payload theo từng cmd được liệt kê ở §7.1; ràng buộc `_BoundErr` là exception nội bộ Delphi (SEH dispatcher nuốt), nhưng **cmd không đạt độ dài sẽ không ghi gì** (callee return sớm).

### 7.2. C→S (client gửi lên — server phải nhận)

```
Payload nhận được (sau khi bỏ Token/Length và giải XOR):
  [0x3C][0x01][text AnsiString Self+0x34]   — từ FUN_005418f8 (click quân/ô; text = 1–2 ký tự mã đi)
  [0x3C][0x02]                              — từ FUN_00546798 (2 byte thuần)
```

- **Đã xác minh (2026-09-14)** — bỏ mọi cảnh báo "suy luận/cần redump bảng": bảng `0x77F474[0x3C]=0x32` / `0x77F53C[0x3C]=0x0078A142` (§6.0), call-site + `CL` đã tìm thấy (§6.2).
- Server nên **đọc `0x3C` là "request thao tác bàn Cờ Tướng"**: byte 2 = **chọn hành động (CL)**: `0x01` = đi quân/chọn với payload text `+0x34` (mã quân 1 ký tự từ `Self+0x34`; ký tự này là **ASCII/VISCII — bản thân nó là mã quân, không phải chuỗi hiển thị**); `0x02` = hành động phụ (không text — có lẽ "hủy/chờ" theo `FUN_00546798.c:24–27`: gọi `FUN_00545d0c` + vtable+0x24 + đặt cờ `DAT_00948dcc^+0x1c9`); `CL` khác: client không gửi ⇒ không bao giờ thấy.
- Response đề xuất: xác nhận bằng `[3C][01][01|03|05][...]` (§7.1) — cmd 1/3/5 của SubOp 1 cũng reset latch `board+0x13` để client được gửi nước tiếp.

---

## 8. Source trail

| # | Nguồn (trong `ts_decompile/`) | Dùng để |
| :-- | :-- | :-- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, index `0x3C` = `0x35`=53) | Byte table MainOp 0x3C → 53 |
| 2 | `redump/jumptable_0x78A9B6_case_functions.csv:55`; `redump/jumptable_dword200_0x78A9B6.hex` (hàng 14, entry#53 = `5C 57 79 00`); `case_functions/manifest.csv:55` | Dword table entry #53 → `0x0079575C` |
| 3 | `case_functions/functions/case_053_0079575C_FUN_0079575c.c:20-39` (handler, 71 dòng) | Lõi logic S→C |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:8797-8863` | Bản gộp case 53 |
| 5 | `functions/0078a89c_FUN_0078a89c.c:6959-6986` | Bản inline dispatcher — xác nhận 1:1 |
| 6 | `handoff-opcode-exploration-guide.md:16-19,31-43` | Framing + XOR + cấu trúc dispatcher |
| 7 | `functions/00553410_FUN_00553410.c:29-41`; `00553840_FUN_00553840.c:31-42`; `0055391c_FUN_0055391c.c:30-42`; `005538cc_FUN_005538cc.c:27-35` | Ánh xạ state type 1..4 → `gvar_007DA42C/007DA0F4/007DA778/007D9F98` |
| 8 | `case_functions/functions/case_050_00795579_FUN_00795579.c:53-58` | OP 0x39 set `gvar_007DA778+0x38` (củng cớ cụm opcode) |
| 9 | `case_functions/functions/case_051_007956B9_FUN_007956b9.c:27-29`; `case_052_007956F3_FUN_007956f3.c:28-36`; `case_054_007957DC_FUN_007957dc.c:28-101` | Đối xứng opcode 0x3A/0x3B/0x3D ↔ singleton |
| 10 | `functions/0053db88_TRE_ZMChessMain.Create.c:22-49`; `functions/0053f450_FUN_0053f450.c:60-70` | Subsytem ZMChess (suy luận định danh `gvar_007DA778`) |
| 11 | `functions/00541c6c_FUN_00541c6c.c:45,187,197-204`; `0053db88...c:42-43` | Dùng chung `DAT_00948dd0/dd8` + sound `WA0045.wav` |
| 12 | ~~grep `007DA778` không có dòng gán~~ **NAY CÓ**: `0055374c_FUN_0055374c.c:41–44` gán từ `TRE_ZMChessMain_Create` | Xác minh tên class |
| 13 | ~~4 callee nằm trong HOLE~~ **NAY ĐÃ DECOMPILE**: `00541098_FUN_00541098.c` (521 dòng), `005408f4_FUN_005408f4.c` (475 dòng), `0053fd78_FUN_0053fd78.c` (739 dòng), `0053f8fc_FUN_0053f8fc.c` (84 dòng) + `.asm.txt` từng hàm | Wire/cmd layout từng SubOp (§4.2) |
| 14 | `functions/0077f414_FUN_0077F414.c:1042-1043` (`case 0x3c: break;`) | Chiều C→S theo bản C — artifact, đã giải thích |
| 15 | **`redump/table_0x77F474_200B.hex` + `redump/table_0x77F53C_dword200.hex`** (parse python): `byte[0x3C]=0x32`, `dword[0x3C]=0x0078A142` | CHỐT builder 0x3C (§6.0) |
| 16 | `functions/00402b90_FUN_00402b90.c:392-411`; `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181` | Copy shortstring + đóng gói khung `[Token][L 2B LE][payload]` |
| 17 | `functions/005418f8_FUN_005418f8.c` + `.asm.txt:62–64` (`MOV CL,0x1; MOV DL,0x3c`); `functions/00546798_FUN_00546798.c` + `.asm.txt:10–12` (`CL=2`) | 2 call-site C→S 0x3C + giá trị `CL` |
| 18 | `functions/00402b60__PStrNCat.c:584–606` | **RTL `_PStrNCat` ĐÃ CÓ BODY** — semantics nối `min(src[0], maxLen − dest[0])` ký tự data (xác nhận toàn bộ asm builder §6.2) |
| 19 | `functions/00551fb8…c` (mode pending→active, dirty `+0xA0`), `functions/0053e13c/0053e2b4/0053f2a8/0053da48/0053d930/0053ee00/0053f88c/00541bf8/005419c8/005465dc/0054698c` — ZMChess subsystem | Field/bảng `+0x70/0x84/0x98/0xac/0xb1/0x4d/0x11b/0x158/0x1ca/0x101/0xfc/0x1c9` (§4.2) |

### Cập nhật 2026-09-14 — những mục đã ĐÓNG

1. ~~Body + nghiệp vụ 4 method callee~~ **XONG (§4.2)**: mỗi callee là `switch(cmd = RP[1])`; layout và field ghi được lập bảng; các hành vi chính (board pointer `+0x20`, seat table `+0x70/0x84/0x98/0xb1`, move coord `+0xac[1..4]`, piece code `+0x4d[33]`, timer rows `+0x158/0x15b/0x163`, latch `board+0x13`) xác minh bằng phép ghi trực tiếp.
2. ~~Dòng gán `gvar_007DA778`~~ **XONG**: `0055374c.c:41–44` (VMT `VMT_53D8B8_TRE_ZMChessMain`) ⇒ `TRE_ZMChessMain` là **tên class xác minh**, không còn suy luận.
3. ~~Bảng byte C→S `0x77F474`/`0x77F53C`~~ **ĐÃ DUMP + CHECK (§6.0)** ⇒ C→S 0x3C **xác nhận tồn tại**, mâu thuẫn C/asm đã giải quyết (Ghidra artifact, giống `case 0x32`).
4. ~~`CL` và call-site `SendCommand(0x3C)`~~ **XONG**: CL=1 (`FUN_005418f8` — đi quân, kèm text `+0x34`) và CL=2 (`FUN_00546798` — thao tác phụ). CL ∉ {1,2,3}: builder return không gửi.
5. ~~Kiểu dữ liệu `TRE_ZMChessMain+0x34`~~ **XONG: AnsiString Delphi chuẩn** — writer `005418f8.c:51,63` dùng `_LStrFromChar/_LStrFromString`, cleanup `:67` `_LStrClr`; builder chỉ `@LStrCat`. (Ý nghĩa field: **buffer chuỗi nhỏ "mã nước đi"** chứa 1–2 ký tự, không phải tên/ngữ cảnh.)

### VẪN CÒN HỞ (giới hạn hiện tại — ghi đúng theo bằng chứng)

1. **13 chuỗi banner SubOp 4** (`0x0053FB4C..0x0053FD6C`) và **8 tên WAV SubOp 3** (`0x00540854..0x005408E8`) + 2 hậu tố banner SubOp 2 (`0x00541068/0x00541084`) — code-gap, **chưa có `lit_*.hex`** → nghiệp vụ từng mã banner/chưa dịch được.
2. **Mapping body↔CL của builder 0x3C**: khối asm `3694–3721` gán CL=1 bằng suy luận khớp (writer `+0x34` duy nhất); hai body CL=2/3 (jump `0x7892D1/0x78933C/0x789391`, dùng chung builder `0x32`) **chưa chỉ ra được 1-1 vì asm.txt không kèm cột address**; chưa kết luận được body nào đúng cho CL=2 hay CL=3.
3. **Ý nghĩa 13 mã SubOp 4** — chỉ là mã số, nội dung text thiếu.
4. **`DAT_00948dcc / DAT_00948dd0 / DAT_00948dd4`** được callee dùng trực tiếp như instance — quan hệ chính xác với `gvar_007DA778^` (cùng object hay bàn phụ/player con) **chưa kết luận được**.
5. **Đối tượng `board = *(Self+0x20)`**: layout (`+8/+0x10..+0x12/+0x14[1..5]/+0x1a[1..5]/+0x1f/+0x20[1..5]/+0x13`) dựng từ các phép ghi; class của nó chưa định danh (nhiều khả năng `TRE_ZMChessPlayer`/bàn con — chưa kết luận được).
6. **Ngữ nghĩa cuối cùng của các cmd** (tọa độ quân cờ kiểu gì — `+0xac` 4 byte / `+0x4d[33]` mã quân / `+0xfc` record 3 entry) — mới có shape + ràng buộc; cần traffic thật hoặc chuỗi để chốt.
