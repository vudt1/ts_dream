# PHÂN TÍCH — Main OP 0x3C (60) / Case 53 / FUN_0079575c @ 0x0079575C

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) là chiều chính**. Chiều C→S: **bản C decompile cho `case 0x3c:` RỖNG** (xem §6); **ASM có một body chưa được gán nhãn dùng đúng đối tượng `gvar_007DA778`** → mâu thuẫn cần redump bảng byte C→S `0x77F474` để chốt.
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh xử lý) từ SSOT** (`ts_decompile/` only).
**BỐN method callee CỦA HANDLER (`func_0x00541098`, `func_0x005408f4`, `func_0x0053fd78`, `func_0x0053f8fc`) NẰM TRONG KHE TRỐNG CHƯA DECOMPILE** (`0x0053F8FC..0x005418F8`) → **unknown / cần redump**. **Đối tượng đích `gvar_007DA778` KHÔNG có dòng gán trong SSOT** → định danh class là **suy luận mạnh** (§4.1), không khẳng định tuyệt đối.

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form (chỉ nhắc 1 dòng khi bắt buộc).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3C là opcode **điều khiển form "ZMChess Main"** — instance toàn cục `gvar_007DA778` (rất nhiều khả năng là `TRE_ZMChessMain`, form chính của **minigame Cờ Tướng / Chinese Chess**, xem §4.1).
- **Payload S→C tối thiểu 1 byte điều khiển**: `RestPayload[0]` = SubOp. Handler **không `switch`** mà dùng chuỗi `if (==1) else if (==2) else if (==3) else if (==4)`; **không có `default`**.
- **4 hành vi hiệu lực** — mỗi SubOp gọi **một method của `TRE_ZMChessMain`** truyền nguyên `RestPayload` làm tham số:
  - `SubOp == 0x01` → `func_0x00541098(gvar_007DA778, RP)` (dòng 29).
  - `SubOp == 0x02` → `func_0x005408f4(gvar_007DA778, RP)` (dòng 32).
  - `SubOp == 0x03` → `func_0x0053fd78(gvar_007DA778, RP)` (dòng 35).
  - `SubOp == 0x04` → `func_0x0053f8fc(gvar_007DA778, RP)` (dòng 38).
- **Cả 4 method callee đều KHÔNG có body trong SSOT** (§4.2) → **ý nghĩa nghiệp vụ chi tiết của từng SubOp là `unknown / cần redump`**. Chỉ biết chắc: cùng một đối tượng đích, cùng một tham số (`RestPayload`).
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
- **Không đọc/parse thêm field nào ở handler** — toàn bộ `RestPayload` (kể cả byte SubOp) được chuyển nguyên cho callee. Layout field phía sau `RP[0]` phụ thuộc callee (hole) → unknown.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** ở handler |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** ở handler |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Không gọi** (handler không cắt field chuỗi) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 23) |
| `gvar_007DA778` | Con trỏ singleton `TRE_ZMChessMain` (đối tượng đích, §4.1) |
| `func_0x00541098` | Callee SubOp 1 (dòng 29) — **hole, không body** |
| `func_0x005408f4` | Callee SubOp 2 (dòng 32) — **hole, không body** |
| `func_0x0053fd78` | Callee SubOp 3 (dòng 35) — **hole, không body** |
| `func_0x0053f8fc` | Callee SubOp 4 (dòng 38) — **hole, không body** |
| `_LStrArrayClr/_LStrClr` (dòng 43-67) | **Dọn dẹp frame cục bộ** của dispatcher (giống hệt `case_051/052/054`), **không liên quan wire** |

> Không có helper giải mã nhị phân nào được gọi trong handler: 4 SubOp chỉ là **tín hiệu**; việc parse nằm trong callee (hole).

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
| `0x01` | `[3C][01]` | `SubOp = 1` | **`func_0x00541098(gvar_007DA778, RP)`** (dòng 29) — **unknown** |
| `0x02` | `[3C][02]` | `SubOp = 2` | **`func_0x005408f4(gvar_007DA778, RP)`** (dòng 32) — **unknown** |
| `0x03` | `[3C][03]` | `SubOp = 3` | **`func_0x0053fd78(gvar_007DA778, RP)`** (dòng 35) — **unknown** |
| `0x04` | `[3C][04]` | `SubOp = 4` | **`func_0x0053f8fc(gvar_007DA778, RP)`** (dòng 38) — **unknown** |
| `0x00`, `>= 0x05` | `[3C][xx]` | không khớp `1..4` | **no-op im lặng** |
| `0x01..0x04` + byte dư | `[3C][01..04][...]` | khớp `1..4` | gọi callee với **toàn bộ RP** (byte dư chuyển tiếp; callee xử lý ra sao = unknown) |

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
- **Suy luận định danh class = `TRE_ZMChessMain` (Cờ Tướng) — độ tin cậy CAO nhưng KHÔNG khẳng định**:
  - Vùng code `0x0053DB88..0x00541F8C` là **subsystem ZMChess**: `functions/0053db88_TRE_ZMChessMain.Create.c:22-49` (constructor `TRE_ZMChessMain`), `functions/0053f450_FUN_0053f450.c:60-70` (tạo `TRE_ZMChessPlayer`, `VMT_4DC5F8_TRE_ZMChessPlayer`).
  - Hàm cleanup type-3 `functions/00541c6c_FUN_00541c6c.c:187` phát `"Sound\\WA0045.wav"`; `:45` đọc `param_1+8`; `:197-204` gọi Show/Hide qua `DAT_00948dd8+0x20/+0x24` — **cùng file/region** dùng `DAT_00948dd0`/`DAT_00948dd8` với `TRE_ZMChessMain.Create` (`0053db88...c:42-43`).
  - `gvar_007DA778 + 0x38 = 100/1000` (OP 0x39, `case_050...c:53-58`) khớp vùng field của form-game này.
  - **Giới hạn**: **KHÔNG có dòng gán `gvar_007DA778 = <kết quả Create>` trong toàn bộ SSOT** (grep `007DA778` chỉ ra 22 match, toàn call-site + `= 0`) → constructor/đăng ký nằm trong vùng HOLE. Nhãn `TRE_ZMChessMain` là **suy luận**, muốn chốt phải redump.

### 4.2. Bốn callee nằm trong khe trống chưa decompile

- `func_0x00541098`, `func_0x005408f4`, `func_0x0053fd78`, `func_0x0053f8fc` — **không có file `.c`** trong `functions/` và **không có dòng trong `index.csv`**.
- Khe trống xác định: hàm export trước là `FUN_0053f88c` @ `0x0053F88C` (`index.csv:3048`, size 109); hàm export sau là `FUN_005418f8` @ `0x005418F8` (`index.csv:3049`). Cả 4 callee (`0x0053F8FC`, `0x0053FD78`, `0x005408F4`, `0x00541098`) nằm trong khoảng `0x0053F8FC..0x005418F8` → **HOLE**.
- Do đó: **layout field của `RestPayload` sau `RP[0]` và nghiệp vụ từng SubOp là `unknown / cần redump`**. Đây là 4 method nội bộ của `TRE_ZMChessMain` (mỗi SubOp một method).

### 4.3. Vì sao không còn nhánh nào khác

- `case_053` chỉ có `if (==1)` / `else if (==2)` / `else if (==3)` / `else if (==4)`; **không `else`, không `switch`, không `default`** → mọi SubOp khác rơi thẳng xuống epilogue (dòng 40-68) → **im lặng**.
- Epilogue `_LStrArrayClr/_LStrClr` (dòng 43-67) là **dọn dẹp stack cục bộ dùng chung cho mọi case** (giống hệt `case_051` dòng 33-57 và `case_054` dòng 102-130), **không phải logic nghiệp vụ**.

### 4.4. Ghi chú về SEH marker

- Các nhãn `&UNK_0079576f`, `&UNK_0079579b`, `&UNK_007957af`, `&UNK_007957d7` (bản inline `0078a89c...c:6964,6971,6975,6979`,6983) là **SEH handler label** của frame dispatch, **không phải string constant** → không cần decode VISCII.

---

## 5. Chuỗi VISCII → UTF-8

- **Handler `FUN_0079575c` KHÔNG tham chiếu bất kỳ string constant / `UNK_xxxx` / `DAT_xxxx` dạng text nào** — chỉ có `gvar_007DA778` và 4 con trỏ hàm. Vì vậy **không có gì để decode VISCII ở chiều S→C cho OP 0x3C**.
- Các literal `UNK_0079576f/9b/af/d7` là **nhãn SEH** (§4.4), không phải dữ liệu chuỗi.
- Bảng `lit_*.hex` trong `redump/` **không chứa** địa chỉ nào quanh `0x007957xx` dùng bởi handler này.

> **Kết luận §5**: OP 0x3C không mang chuỗi. Muốn tìm chuỗi (nếu có) phải redump 4 callee trong hole `0x0053F8FC..0x005418F8` (ví dụ tên/side/chat của bàn cờ) — hiện **không có dump trong SSOT**.

---

## 6. Chiều Client → Server (C→S)

### 6.1. Bản C decompile: `case 0x3c:` RỖNG

`functions/0077f414_FUN_0077F414.c:1042-1043` (trong `switch(param_2 & 0xff)`):

```c
case 0x3c:
  break;
```

→ **Theo bản C decompile, client KHÔNG dựng payload nào cho opcode 0x3C** (rỗng). Nếu chỉ dựa vào file C, kết luận là **C→S KHÔNG tồn tại**.

### 6.2. Bản ASM: có một body chưa được C gán nhãn

`functions/0077f414_FUN_0077f414.asm.txt:3694-3721` — một case body hoàn chỉnh (kết thúc bằng `JMP 0x0078a4f2` = return), dựng payload và gửi:

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

### 6.3. Vì sao block ASM 3694 gần như chắc chắn là `case 0x3C` (đối xứng hai chiều)

Ánh xạ opcode ↔ singleton **đã xác minh ở chiều S→C** (§1) và **đối chiếu được ở chiều C→S**:

| MainOp | S→C handler dùng singleton | C→S body (theo asm) dùng singleton |
| :---: | :-- | :-- |
| `0x3A` | `gvar_007DA42C` | `asm:3616-3651` → `gvar_007DA42C+0xc` (khớp C `case 0x3a`) |
| `0x3B` | `gvar_007DA0F4` | `asm:3652-3685` → `gvar_007DA0F4+0x28` (khớp C `case 0x3b`) |
| **`0x3C`** | **`gvar_007DA778`** | **`asm:3694-3721` → `gvar_007DA778+0x34`** |
| `0x3D` | `gvar_007D9F98` | `asm:3768-3855` → `gvar_007D9F98+0x6a..0x6e` (khớp C `case 0x3d`) |

- Ba opcode `0x3A/0x3B/0x3D` khớp **hoàn hảo** giữa hai chiều (bản C cũng xác nhận 0x3A/0x3B/0x3D). **Đối tượng `gvar_007DA778` (state type 3) chỉ xuất hiện duy nhất tại block asm 3694** — mà theo đối xứng, `0x3C` là opcode gắn với state type 3.
- ⇒ Bản C decompile đã **bỏ sót nhãn** cho `case 0x3c` (IDA xuất `break;`), trong khi ASM chứa body thật. Kết luận: **C→S 0x3C RẤT CÓ KHẢ NĂNG TỒN TẠI** với payload `[0x3C][CL][Self.field_34]`.

### 6.4. Kết luận C→S (kèm mức tin cậy)

```
Payload C→S (suy luận): [0x3C][CL][bytes của *(*(gvar_007DA778)+0x34)]   (field_34 là AnsiString)
```

- **Mâu thuẫn SSOT**: bản C (`0077f414_FUN_0077F414.c:1042`) nói rỗng; bản ASM (`0077f414_FUN_0077f414.asm.txt:3694-3721`) nói có body.
- **Chưa thể chốt 100%** vì **bảng byte C→S `0x77F474` (200 bytes, dword table `0x77F53C` tại `0077f414_FUN_0077f414.asm.txt:33-34`) KHÔNG có dump trong `redump/`** → không kiểm tra được giá trị byte của index `0x3C`.
- **Cần redump `0x77F474` (200B) + `0x77F53C`** để xác nhận `byte[0x3C]` trỏ tới block `asm:3694`.
- `CL` = byte tham số thứ 3 của `TFConnect.SendCommand` (lưu ở `[EBP-0x6]`, `0077f414...asm.txt:17`); **giá trị `CL` và call-site gọi `SendCommand(0x3C)` KHÔNG có trong asm export** → **unknown** (nghi vấn vùng HOLE).

---

## 7. Ghi chú cho Mock Server

### 7.1. S→C (server gửi xuống client)

Payload sau MainOp; khung ngoài = `[Token F4 44][Length L: Word LE][Payload]`, **toàn khung XOR 0xAD** (L tính từ byte MainOp `0x3C`):

```
[3C]          L=1  → ERangeError (BoundErr(0))       — ĐỪNG GỬI
[3C][00]      L=2  → no-op im lặng
[3C][01]      L=2  → func_0x00541098(TRE_ZMChessMain, RP)   (unknown)
[3C][02]      L=2  → func_0x005408f4(TRE_ZMChessMain, RP)   (unknown)
[3C][03]      L=2  → func_0x0053fd78(TRE_ZMChessMain, RP)   (unknown)
[3C][04]      L=2  → func_0x0053f8fc(TRE_ZMChessMain, RP)   (unknown)
[3C][xx>=05]  L=2  → no-op im lặng
[3C][01..04][...dư] L>2 → gọi callee với toàn RP (byte dư do callee xử lý — unknown)
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
- **Hiệu ứng từng SubOp chưa kiểm chứng được** (4 callee trong hole) — mock nên gửi tuần tự `[3C][01..04]` và log thay đổi UI/logic của form Cờ Tướng để dò ngược.

### 7.2. C→S (client gửi lên — server phải nhận)

```
Payload nhận được (sau khi bỏ Token/Length và giải XOR):
  [0x3C][CL][... bytes của TRE_ZMChessMain+0x34 (AnsiString) ...]
```

- **Cảnh báo**: đây là **suy luận** từ asm + đối xứng (§6.3); bản C decompile thể hiện `case 0x3c: break;`. **Cần redump bảng byte `0x77F474`** để xác nhận trước khi mock hard-code.
- Server nên **đọc `0x3C` là "thao tác trên form Cờ Tướng"**, parse phần text phía sau (encoding dự kiến VISCII/cp1258 — chưa xác minh) và trả lời bằng một trong các gói S→C ở §7.1.
- `CL` là byte thứ 2 — **ý nghĩa chưa xác định**; mock nên log cả `CL` để dò khi có traffic thật.

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
| 12 | grep `007DA778` (22 match, toàn call-site + `= 0`, **không có dòng gán**) | Xác nhận constructor/registration nằm trong HOLE |
| 13 | `index.csv:3048` (`FUN_0053f88c` @`0053F88C`), `index.csv:3049` (`FUN_005418f8` @`005418F8`); grep 4 callee → 0 file/0 dòng | Xác nhận 4 callee nằm trong HOLE `0x0053F8FC..0x005418F8` |
| 14 | `functions/0077f414_FUN_0077F414.c:1042-1043` (`case 0x3c: break;`) | Chiều C→S theo bản C (rỗng) |
| 15 | `functions/0077f414_FUN_0077f414.asm.txt:3694-3721` + `:33-34` (byte table `0x77F474`, dword `0x77F53C`) | Body C→S chưa gán nhãn + xác nhận bảng byte C→S không có dump |
| 16 | `functions/00402b90_FUN_00402b90.c:392-411`; `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181` | Copy shortstring + đóng gói khung `[Token][L 2B LE][payload]` |

### Điểm chưa xác minh được từ SSOT (không suy đoán)

1. **Body + nghiệp vụ 4 method callee** (`func_0x00541098`, `func_0x005408f4`, `func_0x0053fd78`, `func_0x0053f8fc`): nằm trong HOLE `0x0053F8FC..0x005418F8` → **unknown / cần redump**. Do đó **layout field sau `RP[0]` và ý nghĩa từng SubOp chưa biết**.
2. **Dòng gán `gvar_007DA778`** (constructor/đăng ký): **không có trong SSOT** → định danh `TRE_ZMChessMain` là **suy luận mạnh** (từ region ZMChess + `DAT_00948dd0/dd8` + sound `WA0045.wav` + state type 3), **chưa xác minh bằng dòng gán**.
3. **Bảng byte C→S `0x77F474` (200B) và dword table `0x77F53C`**: **không có dump trong `redump/`** → không chốt được `byte[0x3C]` → **chiều C→S của OP 0x3C là suy luận (asm + đối xứng), cần redump để xác nhận**.
4. **Giá trị byte `CL`** của C→S `case 0x3c` và **call-site gọi `SendCommand(0x3C)`**: không có trong asm export → **unknown** (nghi vấn vùng HOLE).
5. **Kiểu dữ liệu chính xác của `TRE_ZMChessMain+0x34`**: ASM đối xử như **AnsiString/PChar** (`@LStrCat` trực tiếp); tên/ý nghĩa trường là **suy luận** (không có dòng khởi tạo `+0x34` trong các file export).
6. **Encoding chuỗi** trong payload C→S (VISCII/cp1258/ASCII) **chưa xác minh**.
