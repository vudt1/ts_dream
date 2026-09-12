# PHÂN TÍCH — Main OP 0x3D (61) / Case 54 / FUN_007957dc @ 0x007957DC

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)** là chiều chính, **và C→S CÓ THẬT** (`case 0x3d` trong `FUN_0077f414`, xem §6).
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh xử lý) từ SSOT** (`ts_decompile/` only).
**SÁU callee KHÔNG có body trong SSOT** (`func_0x0054f18c`, `func_0x0054f230`, `func_0x0054f818`, `func_0x00728598`, `func_0x00728734`, `func_0x007287e4`) → `unknown / cần redump`. **CHÍN chuỗi banner `DAT_00799254…DAT_00799450` KHÔNG có dump** → chưa decode được (§5). **Định danh lớp chính xác của 3 gvar đích KHÔNG có trong SSOT** (hàm khởi tạo nằm trong khe trống) → họ `TMachineManager` chỉ là suy luận có bằng chứng gián tiếp (§4.10).

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3D là opcode **điều khiển đa mục đích cho các "máy"/bảng minigame** (machine) và một **bảng UI game dùng chung**. Đây là handler lớn nhất trong cụm `0x3A–0x3D` (133 dòng, 10 nhánh SubOp + 1 `switch` lồng 9 nhánh).
- **Ba đối tượng đích** (dispatch theo SubOp):
  - `gvar_007D9F98` — một **instance "machine"** (đăng ký kiểu 4 trong bảng máy, xem §4.10); nhận SubOp `1, 2, 3`.
  - `gvar_007DA4EC` — một **instance "machine"** khác (kiểu 6); nhận SubOp `7, 8, 9`.
  - `gvar_007D9C48` — một **đối tượng UI/game lớn dùng chung** (họ hàm `FUN_0072xxxx`); nhận SubOp `4, 10, 11`.
- **SubOp `5`** là nhánh duy nhất **không dùng form đích** mà phát **banner/thông báo nổi** trên `gvar_007DA084` (`TSe_TalkMsgFormPlus`, `vtable+0x90`) — 9 biến thể theo `RP[1] = 1..9`.
- **Cấu trúc chung**: `SubOp = RP[0]`; các nhánh `1,2,3,4,5,7,8,9,10,11` — **KHÔNG có `default`, KHÔNG có `case 6`**. Mọi giá trị khác (`0x00`, `0x06`, `>= 0x0C`) là **no-op im lặng**.
- **Hai cổng chặn độ dài**:
  - `RestPayload` rỗng (payload `[3D]`, L=1) → `_BoundErr(0)` = **ERangeError**.
  - `SubOp 5` mà `RestPayload` chỉ dài 1 (payload `[3D][05]`, L=2) → `_BoundErr(1)` = **ERangeError**.
- **C→S CÓ THẬT**: client gửi payload **7 byte** `[3D][CL][obj+0x6a][obj+0x6b][obj+0x6c][obj+0x6d][obj+0x6e]` qua `TFConnect.SendCommand` (`0077f414_FUN_0077F414.c:1044-1059`, asm `3785-3854`). `CL` là byte tham số ẩn; 5 byte còn lại đọc từ `gvar_007D9F98`. Call-site nạp `CL` **không có trong asm export** → `CL` = `unknown`.
- **Bối cảnh (suy luận có cơ sở)**: đây là **kênh điều khiển/đổ dữ liệu cho các bảng "máy"** (danh sách + chỉ số dạng current/max/% + "ball" 1..42), cộng với **kênh banner** dùng chung. Danh tính chính xác của từng máy **chưa xác minh được** (§4.10).

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (đã kiểm lại từng mắt xích)

```
MainOp 0x3D (61) → byte_table[0x78A8EE][0x3D] = 0x36 (54)
                 → dword_table[0x78A9B6][54] @ 0x0078AA8E = 0x007957DC
                 → FUN_007957dc (Case 54)
```

- **Bảng byte 200**: `redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** ứng với index `0x30..0x3F`: `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Byte thứ 14 của hàng (offset `0x3D`) = **`0x36` = 54** (khớp `Case 54`).
- **Bảng dword**: `redump/jumptable_dword200_0x78A9B6.hex` **hàng 7** (entries 48–55): entry #54 = bytes `DC 57 79 00` → **`0x007957DC`** (LE).
- **CSV**: `redump/jumptable_0x78A9B6_case_functions.csv:56` → `54,0x0078AA8E,0x007957DC,YES,"FUN_007957dc",0x007957DC,1`.
- **Manifest**: `case_functions/manifest.csv:56` → `54,0x0078AA8E,0x007957DC,EXPORTED,"FUN_007957dc","007957dc",1,"functions/case_054_007957DC_FUN_007957dc.c",`.
- **File case chính**: `case_functions/functions/case_054_007957DC_FUN_007957dc.c` (133 dòng); SubOp read tại **dòng 21-28**, switch tại **dòng 28-101**.
- **Bản gộp jump table**: `case_functions/jumptable_0x78A9B6_cases.c:8869` (header "Case index: 54"), hàm tại dòng **8875**, lõi logic tại **dòng 8888-8967**.
- **Bản inline trong dispatcher tổng**: `functions/0078a89c_FUN_0078a89c.c:6987-7081` (nhãn `case 0x3d:`) — **khớp 1:1** (§2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `opcode_00_01.md` §2 và `opcode_2d.md` §2. Frame: `[Token 2B: F4 44][Length L: Word LE][Payload]`, toàn khung XOR `0xAD`.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x3D` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile: `unaff_EBP + -0xc` = con trỏ dữ liệu `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP-4)` = **độ dài** `RestPayload` (length prefix của AnsiString).
- `_LStrCopy(s, index, count, &out)` là **1-based**: `_LStrCopy(RP, 3, 4)` = `RP[2..5]` = `P[3..6]`.
- `gvar_007D9F98`, `gvar_007DA4EC`, `gvar_007D9C48` là con trỏ `void**` tới instance (dereference 2 lần: `*(int*)gvar` = `Self`).

### 2.3. Đọc SubOp & `switch` (`case_054` dòng 21-28)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // iVar2 = RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {             // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                      // → ERangeError (payload [3D], L=1)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
switch(*(undefined4 *)(unaff_EBP + -0x14)) {
case 1: ...
```

Diễn giải:
- `RestPayload` rỗng → `_BoundErr(0)`; `iVar2 = extraout_EDX`/`iVar1` là đường phục hồi giả của decompiler cho nhánh exception (thực tế là lỗi range). **L=1 → ERangeError.**
- `SubOp = RP[0]`; decompile dùng **`switch` thật** (khác OP 0x37 dùng `if/else if`).
- **Không có `case 6`, không có `default`**.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Gián tiếp** qua callee `FUN_0054f900` (dòng 72, 80) — giải mã 2 dword từ RP |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** trực tiếp ở handler |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Gián tiếp** qua callee (`FUN_0054f900:71,79`) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 24 và 45) |
| `gvar_007D9F98` | Instance "machine" (SubOp 1,2,3) |
| `gvar_007DA4EC` | Instance "machine" (SubOp 7,8,9) |
| `gvar_007D9C48` | Object UI/game dùng chung (SubOp 4,10,11) |
| `gvar_007DA084` | Con trỏ `TSe_TalkMsgFormPlus`; method `vtable+0x90` = banner (SubOp 5) |
| `DAT_00799254 … DAT_00799450` | 9 literal `.data` truyền làm text banner; **không có dump** (§5) |
| `switchD_00792e22::caseD_0()` | **Artifact decompiler** = nhánh cleanup/epilogue dùng chung (đi kèm `return`), **không phải logic wire** |
| `_LStrArrayClr/_LStrClr` (dòng 105-129) | **Dọn dẹp frame cục bộ** của dispatcher (danh sách rất dài: 99, 14, 45, 35, … phần tử), **không liên quan wire** |

### 2.5. Đối chiếu file case vs bản inline vs bản gộp (1:1)

| Vị trí | `case_054_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 21 | dòng 6989 `iVar21 = local_10` |
| Check rỗng | dòng 23 | dòng 6991 |
| SubOp | dòng 27-28 `switch` | dòng 6997 `switch` |
| SubOp1 call | dòng 30 `FUN_0054e2f8` | dòng 7000 (giống) |
| SubOp5 check | dòng 44 `if (*(uint*)(iVar2+-4) < 2)` | dòng 7017 (giống) |
| SubOp5 case 1 | dòng 50 banner `DAT_00799254,0x898,0,0` | dòng 7024 banner `DAT_00799254,0x898` |
| SubOp5 case 9 | dòng 82 banner `DAT_00799450,0x4b0,0,0` | dòng 7056 banner `DAT_00799450,0x4b0` |
| SubOp7/8/9 | dòng 88/91/94 | dòng 7063/7067/7071 |
| SubOp10/11 | dòng 97/100 | dòng 7075/7079 |

Khác biệt **duy nhất**:
1. Tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX*` vs `iVar20/iVar21/extraout_EDX_x0018x*`) và nhãn SEH marker (`&UNK_007957ef…`).
2. Bản inline hiển thị **3 tham số** cho banner `(self, &literals, duration)`, bản case hiển thị **5 tham số** `(self, &literals, duration, 0, 0)` — **artifact giải mã** (2 tham số `0,0` cuối là hằng 0 dùng chung stack), **không phải khác biệt hành vi**.

**Logic trùng khớp hoàn toàn.** Bản gộp `jumptable_0x78A9B6_cases.c:8875-8998` cũng trùng 1:1.

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler dùng `switch`; **không có `default`, không có `case 6`**. Nhánh `5` có **`switch` lồng** trên `RP[1]`.

| SubOp (RP[0]) | Đích | Payload S→C (P[0]=0x3D) | Điều kiện đọc | Core logic | SSOT callee |
| :---: | :-- | :-- | :-- | :--- | :--- |
| *(rỗng)* | — | `[3D]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **ERangeError** (dòng 23-26) | — |
| `0x01` | `gvar_007D9F98` | `[3D][01][f1][b0..b4]` (L=8) | `RP ≥ 7` (dòng 51-55, 70-73) | `FUN_0054e2f8`: tăng đếm `+0x68` (<0x15), ghi `+0x69=f1`, ghi 5 byte `RP[2..6]` vào slot `(count-1)*5` | **có body** |
| `0x02` | `gvar_007D9F98` | `[3D][02][...]` (L≥2) | — | `func_0x0054f18c(gvar_007D9F98, RP)` | **KHE TRỐNG** |
| `0x03` | `gvar_007D9F98` | `[3D][03][...]` (L≥2) | — | `func_0x0054f230(gvar_007D9F98, RP)` | **KHE TRỐNG** |
| `0x04` | `gvar_007D9C48` | `[3D][04][...]` (L≥2) | — | `func_0x00728598(gvar_007D9C48, RP)` | **KHE TRỐNG** |
| `0x05` | `gvar_007DA084` | `[3D][05][variant]` (L=3) | `RP ≥ 2` (dòng 44-47) | `switch(RP[1]) 1..9` → **9 banner** (xem §4.4). `RP[1]` khác 1..9 → fall-through → no-op | **có body** (banner) |
| `0x06` | — | `[3D][06]` (L=2) | — | **Không có case** → no-op im lặng | — |
| `0x07` | `gvar_007DA4EC` | `[3D][07][...]` (L≥2) | — | `func_0x0054f818(gvar_007DA4EC, RP)` | **KHE TRỐNG** |
| `0x08` | `gvar_007DA4EC` | `[3D][08][slot][A:4B LE][B:4B LE]` (L=11) | `RP ≥ 2` (dòng 58-63); đọc RP[2..9] | `FUN_0054f900`: lưu `A` vào `+100+(slot+1)*8`, `B` vào `+0x68+(slot+1)*8`, `%` vào `+0x90+(slot+1)*4` | **có body** |
| `0x09` | `gvar_007DA4EC` | `[3D][09][ball1..ball6]` (L=8) | đọc RP[1..6], mỗi byte 1..42 (dòng 53) | `FUN_0054f764`: ghi 6 byte vào `+0xa8..+0xad`, rồi `FUN_0054fac0` | **có body** |
| `0x0A` | `gvar_007D9C48` | `[3D][0A][...]` (L≥2) | — | `func_0x00728734(gvar_007D9C48, RP)` | **KHE TRỐNG** |
| `0x0B` | `gvar_007D9C48` | `[3D][0B][...]` (L≥2) | — | `func_0x007287e4(gvar_007D9C48, RP)` | **KHE TRỐNG** |
| `>= 0x0C` | — | `[3D][xx]` (L=2) | — | **Không khớp case** → no-op im lặng | — |

**Tổng: 10 nhánh có hiệu lực** (`1,2,3,4,5,7,8,9,10,11`) + **3 nhánh đọc field có cấu trúc xác minh được** (`1,8,9`) + **9 banner** trong nhánh `5` + **2 dạng gây ERangeError**. Byte dư sau trường cuối **bị bỏ qua**.

---

## 4. Chi tiết các nhánh (bỏ graphics/sound/animation)

### 4.1. `SubOp == 0x01` → `FUN_0054e2f8` (thêm 1 bản ghi 5 byte)

```c
case 1:
  FUN_0054e2f8(*(uint *)gvar_007D9F98,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 30
  break;
```

Body: `functions/0054e2f8_FUN_0054e2f8.c:19-99`:

```c
pcVar1 = (char *)(param_1 + 0x68);
cVar2 = *pcVar1;
*pcVar1 = *pcVar1 + '\x01';                 // tăng bộ đếm +0x68
...
if (*(byte *)(param_1 + 0x68) < 0x15) {     // chỉ xử lý khi count < 21
  if (*(uint *)(param_2 + -4) < 2) { iVar4 = _BoundErr(1); }   // cần RP[1]
  *(undefined1 *)(param_1 + 0x69) = *(undefined1 *)(iVar7 + iVar4);   // +0x69 = RP[1]
  local_10 = 1;
  do {
    ... bound-check RP[local_10 + 1] (tức RP[2..6]) ...
    iVar7 = param_1 + (count-1)*5;
    *(char *)(iVar7 + (local_10 - 1)) = (char)uVar6;   // slot[count-1][0..4] = RP[2..6]
    local_10++;
  } while (local_10 != 6);
}
```

- **State change**: tăng bộ đếm `+0x68` (guard `< 0x15` = 21); ghi `RP[1]` vào `+0x69`; ghi 5 byte `RP[2..6]` vào slot thứ `count-1` của mảng **5 byte/slot**.
- **Wire layout SubOp 1**:
  ```
  P[0]=0x3D  P[1]=0x01  P[2]=f1 (→+0x69)  P[3..7]=b0..b4 (→slot)
  Tối thiểu để đọc đủ 1 slot: L = 8
  ```
- **Giới hạn (bounds)**:
  - `RP` phải dài ≥ 2 để đọc `RP[1]`; nếu không → `_BoundErr(1)` (dòng 51-55).
  - Vòng lặp bound-check từng byte `RP[2..6]` → cần `RP` dài ≥ 7 để ghi đủ 5 byte (dòng 70-73).
- **Suy luận nghiệp vụ**: xây **danh sách/bản ghi 5 byte** (mỗi bản ghi 5 byte, tối đa ~20 bản ghi). Ý nghĩa ngữ nghĩa của 5 byte **chưa xác minh** (không có tên trường).

### 4.2. `SubOp == 0x02` & `0x03` → `func_0x0054f18c` / `func_0x0054f230` (KHE TRỐNG)

```c
case 2: func_0x0054f18c(*(undefined4 *)gvar_007D9F98,*(undefined4 *)(unaff_EBP + -0xc)); break;  // dòng 33
case 3: func_0x0054f230(*(undefined4 *)gvar_007D9F98,*(undefined4 *)(unaff_EBP + -0xc)); break;  // dòng 36
```

- Hai hàm **không có trong SSOT** (không có trong `index.csv`; nằm trong **khe trống** giữa `FUN_0054eef0` (kết thúc ~`0054F180`) và `FUN_0054f224`/`FUN_0054f384`). → **Cấu trúc field wire của SubOp 2/3: `unknown / cần redump`**.
- Chỉ xác minh được: callee nhận `(gvar_007D9F98, RP)` — tức `SubOp = RP[0]` rồi callee tự parse phần còn lại.

### 4.3. `SubOp == 0x04` → `func_0x00728598` (KHE TRỐNG)

```c
case 4: func_0x00728598(*(undefined4 *)gvar_007D9C48,*(undefined4 *)(unaff_EBP + -0xc)); break;  // dòng 39
```

- Không có trong SSOT → **`unknown / cần redump`**. Đích là `gvar_007D9C48` (object UI/game dùng chung).

### 4.4. `SubOp == 0x05` → 9 biến thể banner trên `gvar_007DA084`

```
+----+--------------+----------+---------------------------------------+
| RP[1] | Literal  | Duration | Gọi thêm                              |
| (variant) |       | (ms)     |                                       |
+----+--------------+----------+---------------------------------------+
| 1  | DAT_00799254   | 0x898 (2200) | —                                 |
| 2  | DAT_007992d0   | 0x4b0 (1200) | —                                 |
| 3  | DAT_00799300   | 0x4b0 (1200) | —                                 |
| 4  | LAB_0079932c   | 0x4b0 (1200) | —                                 |
| 5  | DAT_00799350   | 0x4b0 (1200) | —                                 |
| 6  | DAT_00799384   | 0x4b0 (1200) | —                                 |
| 7  | DAT_007993d0   | 0x4b0 (1200) | —                                 |
| 8  | DAT_00799410   | 0x4b0 (1200) | —                                 |
| 9  | DAT_00799450   | 0x4b0 (1200) | —                                 |
| khác | —           | —            | fall-through → no-op               |
+----+--------------+----------+---------------------------------------+
```

```c
case 5:
  iVar1 = 1;
  if (*(uint *)(iVar2 + -4) < 2) { iVar1 = _BoundErr(1); ... }        // dòng 44-47
  switch(*(undefined1 *)(iVar2 + iVar1)) {                           // = RP[1]
  case 1:
    (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&DAT_00799254,0x898,0,0);  // dòng 50
    switchD_00792e22::caseD_0();                                     // artifact → epilogue
    return;
  ...
  }
  break;
```

- **`gvar_007DA084`** = instance `TSe_TalkMsgFormPlus` (đã xác minh ở các OP anh em: `functions/0051189c_FUN_0051189c.c:1265-1277`).
- **`vtable+0x90`** = method phát banner dùng chung `(self, textPtr, duration, 0, 0)` — **thân method KHÔNG có trong SSOT**.
- **Thứ tự giảm dần độ dài**: biến thể `1` (2200 ms) là dài nhất, còn lại đều `1200 ms`.
- **`switchD_00792e22::caseD_0()`** là **artifact decompiler** (nhảy tới epilogue dùng chung) — tương đương `goto cleanup; return;`, **không phải lời gọi hàm nghiệp vụ**.
- **Fall-through khi `RP[1]` khác 1..9**: sau `switch` trong cùng `case 5` có `break;` (dòng 86) → thoát switch ngoài → **no-op im lặng** (không ERangeError vì đã đủ 2 byte).
- **Bounds**: `RP` dài 1 (payload `[3D][05]`, L=2) → `_BoundErr(1)` = **ERangeError** (dòng 44-47).

### 4.5. `SubOp == 0x07` → `func_0x0054f818` (KHE TRỐNG)

```c
case 7: func_0x0054f818(*(undefined4 *)gvar_007DA4EC,*(undefined4 *)(unaff_EBP + -0xc)); break;  // dòng 88
```

- Không có trong SSOT → **`unknown / cần redump`**.

### 4.6. `SubOp == 0x08` → `FUN_0054f900` (cập nhật chỉ số current/max/%)

```c
case 8:
  FUN_0054f900(*(int *)gvar_007DA4EC,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 91
  break;
```

Body: `functions/0054f900_FUN_0054f900.c:22-135`:

```c
if (*(uint *)(param_2 + -4) < 2) { iVar3 = _BoundErr(1); ... }   // dòng 58-63
local_d = *(byte *)(param_2 + iVar3);                            // RP[1] = slot index
if ((byte)(local_d - 1) < 5) {                                   // slot 1..5
  _LStrCopy(local_c,3,4,&local_14);                              // RP[2..5]
  uVar4 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30,local_14);   // → DWORD LE
  *(uint *)(local_8 + 100 + (slot+1)*8) = uVar4;                 // +100+(slot+1)*8 = A
  _LStrCopy(local_c,7,4,&local_18);                              // RP[6..9]
  uVar4 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30,local_18);   // → DWORD LE
  *(uint *)(local_8 + 0x68 + (slot+1)*8) = uVar4;                // +0x68+(slot+1)*8 = B
  ...
  if (A < 1) *(+0x90+(slot+1)*4) = 0;
  else       *(+0x90+(slot+1)*4) = B / A;                        // tỉ lệ %
}
```

- **Field layout SubOp 8** (P[0]=0x3D):

| Byte | Trường | Kiểu | Ghi vào |
| :-- | :-- | :-- | :-- |
| `P[0]` | MainOp `0x3D` | 1B | — |
| `P[1]` | SubOp `0x08` | 1B | — |
| `P[2]` | slot index | 1B (1..5) | chọn slot |
| `P[3..6]` | giá trị A | DWORD LE | `+100+(slot+1)*8` |
| `P[7..10]` | giá trị B | DWORD LE | `+0x68+(slot+1)*8` |

- **State change**: cập nhật mảng giá trị `A`, mảng `B`, và ô tỉ lệ `%` = `B/A` (0 nếu `A<1`).
- **Bounds**: check `RP ≥ 2` cho `RP[1]` (dòng 58-63); mọi truy cập sau dùng `_LStrCopy`/index **có kiểm biên nội bộ trong callee** (`_BoundErr` tại dòng 74-77, 82-85, 88-90, 99-101, 104-106, 111-113, 116-118, 122-124).
- **Wire tối thiểu để có ý nghĩa**: L = 11 (`[3D][08][slot][A 4B][B 4B]`).

### 4.7. `SubOp == 0x09` → `FUN_0054f764` (nạp 6 "ball")

```c
case 9:
  FUN_0054f764(*(int *)gvar_007DA4EC,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 94
  break;
```

Body: `functions/0054f764_FUN_0054f764.c:20-79`:

```c
local_10 = 1;
do {
  ... bound-check RP[local_10] (RP[1..6]) ...
  if ((byte)(RP[local_10] - 1U) < 0x2a) {        // giá trị 1..42
    *(char *)(param_1 + (local_10 - 1) + 0xa8) = (char)uVar3;   // +0xa8.. = balls
  }
  local_10++;
} while (local_10 != 7);
FUN_0054fac0(param_1, ...);                      // hậu xử lý (body có trong SSOT)
```

- **Field layout SubOp 9** (P[0]=0x3D):

| Byte | Trường | Kiểu | Ghi vào |
| :-- | :-- | :-- | :-- |
| `P[0]` | MainOp `0x3D` | 1B | — |
| `P[1]` | SubOp `0x09` | 1B | — |
| `P[2..7]` | 6 "ball" | 6×1B (mỗi byte ∈ 1..42) | `+0xa8..+0xad` |

- **Bằng chứng "ball 1..42"**: `TMachineManager.Create` tạo các resource tên `"Ball01".."Ball42"` và `"LTable01"/"LTable02"` (`functions/0054f500_TMachineManager.Create.c:87-111`). Cận `0x2a` = 42 khớp đúng.
- **Bounds**: vòng lặp bound-check `RP[1..6]` → cần `RP` dài ≥ 6 (payload L ≥ 7) để đọc đủ; byte ngoài 1..42 **bị bỏ qua** (không ghi).
- **Wire tối thiểu**: L = 8 (`[3D][09][b1..b6]`).

### 4.8. `SubOp == 0x0A` & `0x0B` → `func_0x00728734` / `func_0x007287e4` (KHE TRỐNG)

```c
case 10: func_0x00728734(*(undefined4 *)gvar_007D9C48,*(undefined4 *)(unaff_EBP + -0xc)); break;  // dòng 97
case 0xb: func_0x007287e4(*(undefined4 *)gvar_007D9C48,*(undefined4 *)(unaff_EBP + -0xc));         // dòng 100
```

- Không có trong SSOT → **`unknown / cần redump`**. Cả hai nhận `(gvar_007D9C48, RP)`.
- Lưu ý: `case 0xb` **không có `break`** (dòng 100) → **fall-through xuống epilogue**; đây là điểm vào cuối `switch` nên hành vi tương đương các case khác (đều tới epilogue rồi `return`).

### 4.9. Epilogue & artifact `switchD_00792e22`

- Dòng 102-130: `*in_FS_OFFSET = ...; _LStrArrayClr(...); _LStrClr(...); ... return;` — **dọn dẹp stack cục bộ** dùng chung cho mọi case (giống `case_047`/`case_048`/`case_050`). **Không phải logic nghiệp vụ.**
- `switchD_00792e22::caseD_0()` (dòng 51,55,…,83) là **artifact decompiler** trỏ tới epilogue dùng chung; nó buộc `return` sớm trong các nhánh `case 5` (khác `break`). **Không phải call-site thật.**

### 4.10. Danh tính các gvar đích — bằng chứng & giới hạn

| gvar | Nhóm hàm | Bằng chứng gián tiếp trong SSOT | Kết luận |
| :-- | :-- | :-- | :-- |
| `gvar_007D9F98` | `0054e2f8`, `0054f18c`, `0054f230` | Bị `FUN_00553410` free khi "machine type" `+4 == 4` (`functions/00553410_FUN_00553410.c:28-29`); dọn dẹp bởi `FUN_00553840:32` | Instance thuộc **họ máy (machine)**, **vị trí type 4**; tên lớp chính xác **unknown** |
| `gvar_007DA4EC` | `0054f818`, `0054f900`, `0054f764` | Bị `FUN_00553410` free khi `+4 == 6` (`:45-46`); dọn dẹp bởi `FUN_00553840:45` | Instance thuộc **họ máy**, **vị trí type 6**; tên lớp chính xác **unknown** |
| `gvar_007D9C48` | `00728598`, `00728734`, `007287e4` | Dùng ở **rất nhiều** opcode/UI (`FUN_00722508`, `FUN_007281ec` tra cứu entity theo ID 4B — `functions/007281ec_FUN_007281ec.c:56-60`) | Object **UI/game dùng chung**; tên lớp chính xác **unknown** |

- **Bằng chứng "họ máy"**: `functions/0054f500_TMachineManager.Create.c:27` định nghĩa `TMachineManager.Create`; hàm này tạo resource `"machine"`, `"LTable01/02"`, `"Ball01".."Ball42"` (`:77, 87-111`). Các hàm `0054Exxx`/`0054Fxxx` thuộc cùng dải địa chỉ → **cùng họ `TMachineManager`**.
- **Không tìm thấy hàm khởi tạo/`Create` nào gán trực tiếp 3 gvar này trong SSOT** (grep `gvar_007D9F98 =`, `gvar_007DA4EC =`, `gvar_007D9C48 =` chỉ thấy gán `0` khi free) → **không thể chốt tên lớp**: `unknown / cần redump`.

---

## 5. Chuỗi VISCII → UTF-8

### 5.1. Trạng thái dump

Grep toàn bộ `ts_decompile/` cho 9 địa chỉ literal banner của OP 0x3D (`00799254|007992d0|00799300|0079932c|00799350|00799384|007993d0|00799410|00799450`) → **27 match, tất cả là call-site truyền con trỏ**:

| Địa chỉ | Nơi tham chiếu | Dump bytes? |
| :-- | :-- | :-- |
| `0x00799254` | `case_054_...c:50`; `jumptable_...cases.c:8917`; `0078a89c...c:7024` | **KHÔNG** |
| `0x007992d0` | `case_054_...c:54`; `jumptable_...cases.c:8921`; `0078a89c...c:7028` | **KHÔNG** |
| `0x00799300` | `case_054_...c:58`; `jumptable_...cases.c:8925`; `0078a89c...c:7032` | **KHÔNG** |
| `0x0079932c` | `case_054_...c:62`; `jumptable_...cases.c:8929`; `0078a89c...c:7036` | **KHÔNG** |
| `0x00799350` | `case_054_...c:66`; `jumptable_...cases.c:8933`; `0078a89c...c:7040` | **KHÔNG** |
| `0x00799384` | `case_054_...c:70`; `jumptable_...cases.c:8937`; `0078a89c...c:7044` | **KHÔNG** |
| `0x007993d0` | `case_054_...c:74`; `jumptable_...cases.c:8941`; `0078a89c...c:7048` | **KHÔNG** |
| `0x00799410` | `case_054_...c:78`; `jumptable_...cases.c:8945`; `0078a89c...c:7052` | **KHÔNG** |
| `0x00799450` | `case_054_...c:82`; `jumptable_...cases.c:8949`; `0078a89c...c:7056` | **KHÔNG** |

- `redump/` chỉ có **16 file `lit_*.hex`** tại các địa chỉ `5957D8, 595800, 595810, 595828, 595844, 596078, 77F771, 78A854, 7A2094, 7A20A8, 7ABD54, 7ABDAC, 7ABDF0, 7ABE40, 7ABE64, 7ABE74` — **KHÔNG có file nào tại `0x007992xx/7993xx/7994xx`**.
- Không có thư mục `.rodata`/`strings` riêng trong SSOT (`find -type d` chỉ ra `functions/`, `case_functions/`, `redump/`). **Không có `Skill.Dat`** trong SSOT.
- Tiền tố **`DAT_`/`LAB_`** (không phải `s_`) cho thấy IDA truy cập dữ liệu thô nhưng **không xuất bytes** → **không có dump**.

### 5.2. Kết luận decode

> **Chưa có dump trong SSOT → chưa decode được. Cần redump tới null-terminator tại 9 địa chỉ `0x00799254`, `0x007992d0`, `0x00799300`, `0x0079932c`, `0x00799350`, `0x00799384`, `0x007993d0`, `0x00799410`, `0x00799450`.**

| Địa chỉ | Raw bytes | Decoded (VISCII/cp1258 → UTF-8) |
| :-- | :-- | :-- |
| `0x00799254` | *không có trong SSOT* | **N/A — cần redump** |
| `0x007992d0` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799300` | *không có trong SSOT* | **N/A — cần redump** |
| `0x0079932c` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799350` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799384` | *không có trong SSOT* | **N/A — cần redump** |
| `0x007993d0` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799410` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799450` | *không có trong SSOT* | **N/A — cần redump** |

**Chưa xác định được** đây là `PChar` (null-terminated) hay Delphi `AnsiString` (có length prefix tại `ptr-4`); cả hai khả năng đều cần bytes thực tế mới phân biệt được. **Không suy đoán nội dung.**

- Ghi chú ngữ cảnh: 9 literal nằm liền kề trong dải `0x799254..0x799450` (cách nhau `0x7C, 0x30, 0x2C, 0x24, 0x34, 0x4C, 0x40, 0x40`), dùng **cùng method banner** `gvar_007DA084+0x90`, biến thể `1` dài 2200 ms còn lại 1200 ms → gợi ý **một "bảng chuỗi thông báo kết quả"** (thắng/thua/số dư...), nhưng **không thể xác minh nội dung**.

---

## 6. Chiều Client → Server

### 6.1. Vị trí

`functions/0077f414_FUN_0077F414.c:1044-1059` — `case 0x3d:` của `switch(param_2 & 0xff)`:

```c
case 0x3d:
  FUN_00402b90(&stack0xffffffc4,&stack0xffffffc8);
  _PStrNCat(&stack0xffffffc4,&stack0xffffffc0,2);
  FUN_00402b90(local_80,&stack0xffffffc4);
  _PStrNCat(local_80,&stack0xffffffc0,3);
  FUN_00402b90(local_d4,local_80);
  _PStrNCat(local_d4,&stack0xffffffc0,4);
  FUN_00402b90(local_dc,local_d4);
  _PStrNCat(local_dc,&stack0xffffffc0,5);
  FUN_00402b90(local_e4,local_dc);
  _PStrNCat(local_e4,&stack0xffffffc0,6);
  FUN_00402b90(local_410,local_e4);
  _PStrNCat(local_410,&stack0xffffffc0,7);
  _LStrFromString((int *)local_834,local_410);
  TForm1_CY_AddSedQueue(*(undefined4 *)gvar_007DA664,local_834[0]);
  break;
```

### 6.2. Dịch ngược từng bước (đối chiếu ASM)

ASM thật cùng case: `functions/0077f414_FUN_0077F414.asm.txt:3768-3854`; prologue lưu tham số ẩn tại **dòng 17-18** (`MOV [EBP-6],CL` = tham số thứ 3; `MOV [EBP-5],DL` = MainOp).

```asm
3768 LEA EAX,[EBP + -0x34]          ; buf A
3769 MOV DL,byte ptr [EBP + -0x5]   ; DL = MainOp (= 0x3D)
3770 MOV byte ptr [EAX + 0x1],DL    ; A[1] = 0x3D
3771 MOV byte ptr [EAX],0x1         ; A[0] = length 1  → shortstring [01][3D]
3772-3774                            ; B(-0x38) = copy(A)
3775 LEA EAX,[EBP + -0x3c]          ; buf C
3776 MOV DL,byte ptr [EBP + -0x6]   ; DL = CL (tham số 3 ẩn)
3777 MOV byte ptr [EAX + 0x1],DL    ; C[1] = CL
3778 MOV byte ptr [EAX],0x1         ; C[0] = length 1  → [01][CL]
3779-3782                            ; @PStrNCat(B, C, 2)   → B = [3D][CL]
3783-3785                            ; D(-0x7c) = copy(B)
3786 LEA EAX,[EBP + -0x3c]
3787 MOV EDX,[0x007d9f98]
3788 MOV EDX,[EDX]                  ; Self = *(gvar_007D9F98)
3789 MOV DL,byte ptr [EDX + 0x6a]   ; DL = Self.field_6a
3790 MOV byte ptr [EAX + 0x1],DL
3791 MOV byte ptr [EAX],0x1         ; C = [01][+0x6a]
3792-3795                            ; @PStrNCat(D, C, 3)   → D = [3D][CL][6a]
3799 MOV DL,byte ptr [EDX + 0x6b]   ; ... maxlen 4 → [3D][CL][6a][6b]
3812 MOV DL,byte ptr [EDX + 0x6c]   ; ... maxlen 5
3825 MOV DL,byte ptr [EDX + 0x6d]   ; ... maxlen 6
3838 MOV DL,byte ptr [EDX + 0x6e]   ; ... maxlen 7 → 7 byte
3848 LEA EDX,[EBP + 0xfffffbf4]
3849 LEA EAX,[EBP + 0xfffff7d0]
3850 CALL 0x0040402c                  ; @LStrFromString (bỏ byte độ dài)
3851-3854                            ; TForm1.CY_AddSedQueue(queue, payload)
```

### 6.3. Từng helper

- **`FUN_00402b90(dest, src)` — copy chuỗi dài lượng ngắn** (body: `functions/00402b90_FUN_00402b90.c:392-411`): copy đúng `*src + 1` byte (byte độ dài + nội dung). Ở đây dùng để nhân bản buffer shortstring qua từng bước.
- **`_PStrNCat(dest, src, maxLen)` — `@PStrNCat` @ `0x00402B60`** (RTL Borland, **không có body trong SSOT**): **nối (append) `src` vào `dest`, giới hạn độ dài đích = `maxLen`**. Vì `maxLen` tăng dần `2→3→4→5→6→7` và mỗi lần `src` là **1 byte khác nhau**, payload lớn dần đúng 1 byte mỗi bước.
- **`_LStrFromString(dest, src)` — `@LStrFromString` @ `0x0040402C`** (RTL, không body): chuyển **Pascal shortstring** (byte [0] = độ dài) sang **Delphi AnsiString** → bỏ byte độ dài.
- **`TForm1.CY_AddSedQueue(queue, payload)`** (body: `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181`): tính `L = _LStrLen(payload)`, mã hoá `L` thành **2 byte LE** qua `FUN_0077eb1c`, rồi ghép khung `[Token F4 44][L Word LE][payload]` và đẩy vào queue. → **payload case 0x3d chính là nội dung giữa Length và Token sau khi ghép**.

### 6.4. Wire C→S (suy ra)

```
Payload C→S = [0x3D][CL][ *(byte*)(*(gvar_007D9F98)+0x6a) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6b) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6c) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6d) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6e) ]        (tổng 7 byte)
```

| Byte | Ý nghĩa | Nguồn |
| :-- | :-- | :-- |
| `P[0]` | `0x3D` (MainOp) | `[EBP-5] = DL` (asm dòng 18) |
| `P[1]` | `CL` — **byte tham số ẩn #3** (`SendCommand` chỉ hiện 2 tham số decompile) | `[EBP-6] = CL` (asm dòng 17) |
| `P[2]` | field `+0x6a` của `gvar_007D9F98` | asm 3787-3790 |
| `P[3]` | field `+0x6b` | asm 3800-3803 |
| `P[4]` | field `+0x6c` | asm 3813-3816 |
| `P[5]` | field `+0x6d` | asm 3826-3829 |
| `P[6]` | field `+0x6e` | asm 3839-3842 |

- **Đối xứng với S→C**: `gvar_007D9F98` chính là đối tượng nhận SubOp `1/2/3` ở chiều S→C (dòng 30/33/36). Vậy `0x3D` là **request (C→S) → response (S→C)** trên cùng đối tượng máy.
- **Điểm chưa xác minh**: các field `+0x6a..+0x6e` **không thấy được ghi trong SSOT** — `FUN_0054e2f8` (SubOp 1) chỉ ghi `+0x69` và mảng slot. Khả năng cao chúng được ghi bởi `func_0x0054f18c`/`func_0x0054f230` (SubOp 2/3 — **khe trống**). → **ngữ nghĩa 5 field: unknown**.
- **`CL` và call-site**: grep `MOV DL,0x3D` trong toàn bộ `*.asm.txt` → **0 file**. → **call-site gọi `SendCommand(0x3D)` nằm ngoài asm export (vùng HOLE)**; **giá trị `CL`: unknown / cần redump**.

---

## 7. Ghi chú cho Mock Server

### 7.1. S→C (server gửi xuống client)

Payload sau MainOp; khung ngoài = `[Token F4 44][Length L: Word LE][Payload]`, **toàn khung XOR 0xAD** (L tính từ byte MainOp `0x3D`):

```
[3D]                              L=1  → ERangeError (BoundErr(0))        — ĐỪNG GỬI
[3D][01]                          L=2  → ERangeError (BoundErr(1) trong FUN_0054e2f8)  — ĐỪNG GỬI
[3D][01][f1][b0..b4]              L=8  → thêm 1 bản ghi 5 byte vào gvar_007D9F98
[3D][02][...]                     L≥2  → func_0x0054f18c (unknown)
[3D][03][...]                     L≥2  → func_0x0054f230 (unknown)
[3D][04][...]                     L≥2  → func_0x00728598 (unknown)
[3D][05]                          L=2  → ERangeError (BoundErr(1))        — ĐỪNG GỬI
[3D][05][01]                      L=3  → banner DAT_00799254 (2200 ms)
[3D][05][02..09]                  L=3  → banner DAT_007992d0…DAT_00799450 (1200 ms)
[3D][05][xx>09]                   L=3  → no-op im lặng
[3D][06]                          L=2  → no-op im lặng (không có case 6)
[3D][07][...]                     L≥2  → func_0x0054f818 (unknown)
[3D][08][slot][A 4B LE][B 4B LE]  L=11 → cập nhật chỉ số current/max/% trên gvar_007DA4EC
[3D][09][b1..b6]                  L=8  → nạp 6 "ball" (1..42) vào gvar_007DA4EC
[3D][0A][...]                     L≥2  → func_0x00728734 (unknown)
[3D][0B][...]                     L≥2  → func_0x007287e4 (unknown)
[3D][xx>=0C]                      L=2  → no-op im lặng
```

**Ví dụ khung hoàn chỉnh** (đã XOR `0xAD`):

```
# [3D][05][01] banner dài 2200 ms:
payload  = 3D 05 01
L (LE)   = 03 00
pre-XOR  = F4 44 03 00 3D 05 01
on-wire  = 59 E9 AE AD 90 A8 AC

# [3D][08][slot=1][A=100][B=200]:
payload  = 3D 08 01 64 00 00 00 C8 00 00 00
L (LE)   = 0B 00
pre-XOR  = F4 44 0B 00 3D 08 01 64 00 00 00 C8 00 00 00
on-wire  = 59 E9 A6 AD 90 A5 AC C9 AD AD AD 65 AD AD AD

# [3D][09][balls = 05 0A 0F 14 1A 2A]:
payload  = 3D 09 05 0A 0F 14 1A 2A
L (LE)   = 08 00
pre-XOR  = F4 44 08 00 3D 09 05 0A 0F 14 1A 2A
on-wire  = 59 E9 A5 AD 90 A4 A8 A7 A2 B9 B7 87
```

- **Bắt buộc**:
  - Không gửi `[3D]` (L=1) — ERangeError.
  - Với SubOp `01`, gửi đủ `L ≥ 8` để tránh `_BoundErr(1)` trong `FUN_0054e2f8`.
  - Không gửi `[3D][05]` (L=2) — ERangeError.
- **Nội dung 9 banner chưa đọc được** (literal chưa dump, §5): mock chỉ chọn được **đúng địa chỉ literal** bằng `RP[1] = 01..09`; không kiểm tra được text client hiển thị.
- **SubOp 1/8/9** là 3 nhánh có cấu trúc wire **xác minh được**; 6 nhánh `2,3,4,7,10,11` chỉ nên gửi **≥ 2 byte** (`[3D][subop]`) để tránh lỗi, mock cần **log và dò traffic thật** để suy ra phần thân.

### 7.2. C→S (client gửi lên — server phải nhận)

```
Payload nhận được (sau khi bỏ Token/Length và giải XOR):
  [0x3D][CL][obj_6a][obj_6b][obj_6c][obj_6d][obj_6e]        (7 byte)
```

- Server nên **đọc `0x3D` là "gửi trạng thái/5 field của máy"**, parse 7 byte cố định.
- Cả `CL` và 5 field `obj_6a..obj_6e` **chưa có ngữ nghĩa xác minh** — mock nên **log hex toàn bộ** để dò khi có traffic thật.
- **Không có tiền tố độ dài cho chuỗi**: payload cố định 7 byte do khung `L` quyết định.

---

## 8. Source trail

| # | Nguồn (trong `ts_decompile/`) | Dùng để |
| :-- | :-- | :-- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, index `0x3D` = `0x36` = 54) | Byte table MainOp 0x3D → 54 |
| 2 | `redump/jumptable_dword200_0x78A9B6.hex` (hàng 7, entry#54 = `DC 57 79 00`); `redump/jumptable_0x78A9B6_case_functions.csv:56`; `case_functions/manifest.csv:56` | Dword table entry #54 → `0x007957DC` |
| 3 | `case_functions/functions/case_054_007957DC_FUN_007957dc.c:21-101` (handler, 133 dòng) | Lõi logic S→C |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:8869-8998` | Bản gộp case 54 |
| 5 | `functions/0078a89c_FUN_0078a89c.c:6987-7081` | Bản inline dispatcher — xác nhận 1:1 |
| 6 | `functions/0054e2f8_FUN_0054e2f8.c:41-99` | Body SubOp 1 (thêm bản ghi 5 byte) |
| 7 | `functions/0054f900_FUN_0054f900.c:54-134` + `:71-72,79-80` (`FUN_0077ef7c`) | Body SubOp 8 (current/max/%) |
| 8 | `functions/0054f764_FUN_0054f764.c:36-78` | Body SubOp 9 (6 "ball" 1..42) |
| 9 | `functions/0054f500_TMachineManager.Create.c:27,77,87-111` | Bằng chứng họ `TMachineManager` + `"Ball01".."Ball42"` |
| 10 | `functions/00553410_FUN_00553410.c:28-29,45-46`; `functions/00553840_FUN_00553840.c:32,45`; `functions/0055391c_FUN_0055391c.c:32,45` | `gvar_007D9F98` = machine type 4; `gvar_007DA4EC` = machine type 6 |
| 11 | `functions/007281ec_FUN_007281ec.c:56-60` | `gvar_007D9C48` = object UI tra cứu entity theo ID 4B (họ `FUN_0072xxxx`) |
| 12 | Grep `0054f18c|0054f230|0054f818|00728598|00728734|007287e4` trong `index.csv` → 0 match | Xác nhận 6 callee **không có body SSOT** (khe trống) |
| 13 | Grep `00799254…00799450` (27 match, toàn call-site); `ls redump/lit_*.hex` (16 file, không có `7992xx`) | Xác nhận 9 literal banner **chưa có dump** |
| 14 | Grep `gvar_007D9F98 =`/`gvar_007DA4EC =`/`gvar_007D9C48 =` (chỉ thấy gán `0` khi free) | Xác nhận hàm khởi tạo/`Create` **không có trong SSOT** → tên lớp unknown |
| 15 | `functions/0077f414_FUN_0077F414.c:1044-1059` + `.asm.txt:17-18,3768-3854` | Chiều C→S (case 0x3d) — payload 7 byte |
| 16 | `functions/00402b90_FUN_00402b90.c:392-411` | Body `FUN_00402b90` (copy length-prefixed string) |
| 17 | `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181` | Đóng gói khung `[Token][L 2B LE][payload]` bằng `FUN_0077eb1c` |
| 18 | `functions/0051189c_FUN_0051189c.c:1265-1277` | `gvar_007DA084` = `TSe_TalkMsgFormPlus` |
| 19 | Grep `MOV DL,0x3D` trong toàn bộ `*.asm.txt` → 0 file | Xác nhận call-site C→S `0x3D` **không có trong asm export** (vùng HOLE) |

### Điểm chưa xác minh được từ SSOT (không suy đoán)

1. **Thân 6 callee trong khe trống**: `func_0x0054f18c` (SubOp 2), `func_0x0054f230` (SubOp 3), `func_0x0054f818` (SubOp 7), `func_0x00728598` (SubOp 4), `func_0x00728734` (SubOp 10), `func_0x007287e4` (SubOp 11) → **cấu trúc field wire: unknown / cần redump**.
2. **Tên lớp chính xác của `gvar_007D9F98`, `gvar_007DA4EC`, `gvar_007D9C48`**: hàm khởi tạo không có trong SSOT; "họ `TMachineManager`" và "machine type 4/6" là **suy luận** từ `FUN_0054f500` + `FUN_00553410` + dải địa chỉ.
3. **Nội dung 9 literal banner `DAT_00799254…DAT_00799450`**: **không có dump trong SSOT → chưa decode được; cần redump tới null-terminator**. Chưa xác định `PChar` hay Delphi `AnsiString`.
4. **Ngữ nghĩa 5 byte `RP[2..6]` của SubOp 1** và **2 field dword `A`/`B` của SubOp 8** (chỉ biết chúng bị ghi vào các mảng trạng thái): **chưa có tên trường**.
5. **Ngữ nghĩa/người ghi 5 field `+0x6a..+0x6e`** trong payload C→S: chưa xác minh (nghi vấn do SubOp 2/3 ghi, hai hàm này ở khe trống).
6. **Giá trị byte `CL` của C→S `case 0x3d`** và **call-site gọi `SendCommand(0x3D)`**: không có trong asm export → **unknown** (nghi vấn nằm trong vùng HOLE).
7. **Ý nghĩa nghiệp vụ chính xác của OP 0x3D** (điều khiển "máy"/bảng minigame + banner) là **suy luận** từ cấu trúc state + họ `TMachineManager`; chưa có traffic thật để xác nhận.
