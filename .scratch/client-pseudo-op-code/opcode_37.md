# PHÂN TÍCH — Main OP 0x37 (55) / Case 48 / FUN_007954a5 @ 0x007954A5

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)** là chiều chính, **nhưng C→S CÓ THẬT** (`case 0x37` trong `FUN_0077f414`, xem §6).
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh xử lý) từ SSOT** (`ts_decompile/` only).
**HAI method ảo được gọi KHÔNG có body trong SSOT** (`gvar_007DA3B4`+0x20 và `gvar_007DA084`+0x90) → `unknown / cần redump`. **HAI chuỗi banner `UNK_007991d8` / `UNK_007991ec` KHÔNG có dump** → chưa decode được (§5).

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x37 là opcode **điều khiển form nhập "Cafe ID"** (`TSe_CafeIDForm`, `gvar_007DA3B4`) và **phát banner thông báo** trên form thông báo lớn (`TSe_TalkMsgFormPlus`, `gvar_007DA084`).
- **Payload S→C gồm 2 byte điều khiển**: `RestPayload[0]` = SubOp, và với SubOp `0x02` thì `RestPayload[1]` = biến thể banner.
- **Chỉ có 3 hành vi hiệu lực**:
  - `SubOp == 0x01` → gọi **1 method ảo 0 tham số** trên `gvar_007DA3B4` (`vtable+0x20`) — nhiều khả năng là ẩn form (suy luận, §4.5).
  - `SubOp == 0x02` + `RP[1] == 0x01` → hiển thị **banner `UNK_007991d8`, 2000 ms**.
  - `SubOp == 0x02` + `RP[1] == 0x02` → hiển thị **banner `UNK_007991ec`, 2000 ms**, **rồi** gọi method `vtable+0x20` trên `gvar_007DA3B4`.
- **Không có `default`, không có `switch`**: mọi SubOp khác (`0x00`, `>= 0x03`, và `SubOp 2` với `RP[1]` khác 1/2) là **no-op im lặng**.
- **Cổng chặn độ dài**: `RestPayload` rỗng (payload `[37]`, L=1) → `_BoundErr(0)` = **ERangeError**. `SubOp 0x02` mà `RestPayload` chỉ dài 1 (payload `[37][02]`, L=2) → `_BoundErr(1)` = **ERangeError**. Mock phải tránh hai độ dài này.
- **C→S CÓ THẬT (khác OP 0x36 về mặt này? — xem §6)**: client gửi `[0x37][CL][nội dung editor]` qua `TFConnect.SendCommand` (`0077f414.c:1012-1019`). `CL` là byte tham số thứ 3, và phần đuôi là **text đang nhập trong editor của form CafeID** (`gvar_007DA3B4+0x138`→`TSe_Editor`→`+0x1a0` = text).
- **Bối cảnh (suy luận có cơ sở)**: đây là **cặp request/response kiểm tra "Cafe ID"**: client gửi ID đã nhập lên server; server trả `SubOp 1` (ẩn form ≈ hợp lệ) hoặc `SubOp 2/x` (banner lỗi; biến thể `x=2` vừa báo lỗi vừa đóng form). *(Đây là SUY LUẬN từ hành vi 2 method + luồng C→S; thân 2 method không có trong SSOT.)*

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (đã kiểm lại từng mắt xích)

```
MainOp 0x37 (55) → byte_table[0x78A8EE][0x37] = 0x30 (48)
                 → dword_table[0x78A9B6][48] @ 0x0078AA76 = 0x007954A5
                 → FUN_007954a5 (Case 48)
```

- Bảng byte 200: `redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** ứng với index `0x30..0x3F`: `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Byte thứ 8 của hàng (offset `0x37`) = **`0x30` = 48** (kiểm bằng python: `vals[0x37] = 0x30`).
- Bảng dword: `redump/jumptable_0x78A9B6_case_functions.csv:48` → `48,0x0078AA76,0x007954A5,YES,"FUN_007954a5",0x007954A5,1`.
- Đối chiếu hex thô: `redump/jumptable_dword200_0x78A9B6.hex` **hàng 13**, entry #48 = bytes `A5 54 79 00` → **`0x007954A5`** (LE).
- Manifest: `case_functions/manifest.csv:48` → `48,0x0078AA76,0x007954A5,EXPORTED,"FUN_007954a5","007954a5",1,"functions/case_048_007954A5_FUN_007954a5.c",`.
- File case chính: `case_functions/functions/case_048_007954A5_FUN_007954a5.c` (77 dòng); lõi logic tại **dòng 21-45**.
- Bản gộp jump table: `case_functions/jumptable_0x78A9B6_cases.c:8419` (header "Case index: 48"), hàm tại dòng **8425**, lõi logic tại **dòng 8438-8462**.
- Bản inline trong dispatcher tổng: `functions/0078a89c_FUN_0078a89c.c:6816-6848` (nhãn `case 0x37:`) — **khớp 1:1** (§2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `opcode_00_01.md` §2 và `opcode_2d.md` §2. Frame: `[Token 2B: F4 44][Length L: Word LE][Payload]`, toàn khung XOR `0xAD`.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x37` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile: `unaff_EBP + -0xc` = con trỏ dữ liệu `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP-4)` = **độ dài** `RestPayload` (length prefix của AnsiString).

### 2.3. Đọc SubOp (`case_048` dòng 21-31)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // iVar2 = RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {             // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                      // → ERangeError (payload [37], L=1)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
if (*(int *)(unaff_EBP + -0x14) == 1) {      // == 1 (không phải switch)
  (**(code **)(**(int **)gvar_007DA3B4 + 0x20))();               // gọi method vtable+0x20
}
else if (*(int *)(unaff_EBP + -0x14) == 2) { // == 2
  iVar2 = *(int *)(unaff_EBP + -0xc);
  iVar1 = 1;
  if (*(uint *)(iVar2 + -4) < 2) {           // độ dài RestPayload < 2
    iVar1 = _BoundErr(1);                    // → ERangeError (payload [37][02], L=2)
    iVar2 = extraout_EDX_00;
  }
  if (*(char *)(iVar2 + iVar1) == '\x01') {  // RP[1] == 1
    (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_007991d8,2000,0,0);
  }
  else if (*(char *)(iVar2 + iVar1) == '\x02') {   // RP[1] == 2
    (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_007991ec,2000,0,0);
    (**(code **)(**(int **)gvar_007DA3B4 + 0x20))();                // rồi gọi vtable+0x20
  }
}
```

Diễn giải:
- `RestPayload` rỗng → `_BoundErr(0)`; `iVar2 = extraout_EDX`/`iVar1` là đường phục hồi giả của decompiler cho nhánh exception (thực tế là lỗi range). **L=1 → ERangeError.**
- `SubOp = RP[0]`, so sánh `== 1` rồi `else if == 2` (không `switch`).
- Với SubOp 2: re-đọc lại độ dài; **`< 2` → `_BoundErr(1)`** trước khi đọc `RP[1]`.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Không gọi** (không đọc field chuỗi từ wire ở chiều S→C) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 24 và 35) |
| `gvar_007DA3B4` | Con trỏ `TSe_CafeIDForm`; method `vtable+0x20` (dòng 29, 43) |
| `gvar_007DA084` | Con trỏ `TSe_TalkMsgFormPlus`; method `vtable+0x90` = banner (dòng 39, 42) |
| `UNK_007991d8`, `UNK_007991ec` | 2 literal `.rodata`/`.data` truyền làm text banner; **không có dump** |
| `_LStrArrayClr/_LStrClr` (dòng 49-73) | **Dọn dẹp frame cục bộ** của dispatcher (giống hệt `case_047`/`case_050`), **không liên quan wire** |

### 2.5. Đối chiếu file case vs bản inline (1:1)

| Vị trí | `case_048_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 21 `iVar2 = *(int *)(unaff_EBP + -0xc)` | dòng 6818 `iVar21 = local_10` |
| Check rỗng | dòng 23 `if (*(int *)(iVar2 + -4) == 0)` | dòng 6820 `if (*(int *)(local_10 + -4) == 0)` |
| SubOp | dòng 27 `... = *(byte *)(iVar2 + iVar1)` | dòng 6826 `if (*(char *)(iVar21 + iVar20) == '\x01')` |
| SubOp1 call | dòng 29 `(**(code **)(**(int **)gvar_007DA3B4 + 0x20))()` | dòng 6828 (giống) |
| SubOp2 check | dòng 34 `if (*(uint *)(iVar2 + -4) < 2)` | dòng 6833 `if (*(uint *)(local_10 + -4) < 2)` |
| RP[1]==1 | dòng 39 banner `UNK_007991d8,2000,0,0` | dòng 6840 banner `UNK_007991d8,2000` |
| RP[1]==2 | dòng 42-43 banner `UNK_007991ec` + call `+0x20` | dòng 6844-6845 (giống) |

Khác biệt **duy nhất**:
1. Tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX*` vs `iVar20/iVar21/extraout_EDX_x0017*`) và nhãn SEH marker (`&UNK_007954b8/007954db/007954f3`).
2. Bản inline hiển thị **3 tham số** cho banner `(self, &UNK_..., 2000)`, bản case hiển thị **5 tham số** `(self, &UNK_..., 2000, 0, 0)` — **artifact giải mã** (2 tham số `0,0` cuối là hằng 0 dùng chung stack), **không phải khác biệt hành vi**.

**Logic trùng khớp hoàn toàn.**

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler **không dùng `switch`** mà `if (==1) else if (==2)`; **không có `default`**.

| SubOp (RP[0]) | RP[1] | Payload S→C | Điều kiện đọc | Core logic |
| :---: | :---: | :--- | :--- | :--- |
| *(rỗng)* | — | `[37]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **ERangeError** (dòng 23-26) |
| `0x00` | — | `[37][00]` | `SubOp = 0` | Không khớp `1`/`2` → **no-op im lặng** |
| `0x01` | — | `[37][01]` | `SubOp = 1` | **`(**(code**)(**(int**)gvar_007DA3B4 + 0x20))()`** (dòng 29) — method 0 tham số, body **unknown** |
| `0x02` | `0x01` | `[37][02][01]` | độ dài `RP ≥ 2`, `RP[1] = 1` | Banner **`UNK_007991d8`, 2000 ms** trên `gvar_007DA084` (dòng 39) |
| `0x02` | `0x02` | `[37][02][02]` | độ dài `RP ≥ 2`, `RP[1] = 2` | Banner **`UNK_007991ec`, 2000 ms** (dòng 42) **+ gọi `gvar_007DA3B4`+0x20** (dòng 43) |
| `0x02` | `>= 3` | `[37][02][xx]` | độ dài `RP ≥ 2`, `RP[1]=xx` | Không khớp `1`/`2` → **no-op im lặng** |
| `0x02` | *(thiếu)* | `[37][02]` (L=2) | `*(uint*)(RP-4) < 2` | `_BoundErr(1)` → **ERangeError** (dòng 34-37) |
| `0x03..0xFF` | — | `[37][xx]` | `SubOp = xx` | Không khớp `1`/`2` → **no-op im lặng** |

**Tổng: 3 nhánh có hiệu lực** (`0x01`, `0x02/01`, `0x02/02`) + **2 dạng gây ERangeError**. Mọi trường hợp còn lại là no-op. Byte dư sau `RP[1]` (vd `[37][02][01][FF]`) **bị bỏ qua**.

---

## 4. Chi tiết các nhánh (bỏ graphics/sound/animation)

### 4.1. `SubOp == 0x01` → method `gvar_007DA3B4` +0x20

```c
if (*(int *)(unaff_EBP + -0x14) == 1) {                        // case_048 dòng 28
  (**(code **)(**(int **)gvar_007DA3B4 + 0x20))();             //      dòng 29
}
```

- **Không đọc thêm field nào** — kể cả khi payload có byte dư.
- **`gvar_007DA3B4`** = instance `TSe_CafeIDForm`, xác minh tại `functions/0051189c_FUN_0051189c.c:1186-1189`:
  - dòng 1186 `piVar3 = FUN_005b0774((int *)VMT_58A680_TSe_CafeIDForm,'\x01',extraout_ECX_03);`
  - dòng 1187 `*(int **)gvar_007DA3B4 = piVar3;`
  - dòng 1189 `FUN_007b075c(...gvar_007DA3B4);` (đăng ký với manager)
- **`vtable+0x20`**: xem §4.5 — **body KHÔNG có trong SSOT**.

### 4.2. `SubOp == 0x02`, `RP[1] == 0x01` → banner `UNK_007991d8`

```c
if (*(char *)(iVar2 + iVar1) == '\x01') {                      // case_048 dòng 38
  (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_007991d8,2000,0,0);
}
```

- **`gvar_007DA084`** = instance `TSe_TalkMsgFormPlus`, xác minh tại `functions/0051189c_FUN_0051189c.c:1265-1277`:
  - dòng 1265 `piVar3 = FUN_0063bb3c((int *)VMT_63B39C_TSe_TalkMsgFormPlus,'\x01',extraout_ECX_15);`
  - dòng 1266 `*(int **)gvar_007DA084 = piVar3;`
  - dòng 1275 `(**(code **)(**(int **)gvar_007DA084 + 8))(*gvar_007DA084,"panel10",0xfa);`
  - dòng 1277 đăng ký với manager.
- **`vtable+0x90`** = method banner (nhận `self, textPtr, duration, 0, 0`). Body **không có trong SSOT** (§4.5). Ở nhánh này **không** ẩn form.
- Literal `UNK_007991d8` **chưa có dump** → chưa biết nội dung (§5).

### 4.3. `SubOp == 0x02`, `RP[1] == 0x02` → banner `UNK_007991ec` + ẩn form

```c
else if (*(char *)(iVar2 + iVar1) == '\x02') {                 // case_048 dòng 41
  (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_007991ec,2000,0,0);
  (**(code **)(**(int **)gvar_007DA3B4 + 0x20))();             //      dòng 43
}
```

- Khác nhánh §4.2 **chỉ ở literal** (`UNK_007991ec`) **và** việc **gọi thêm `gvar_007DA3B4`+0x20**.
- Diễn giải (suy luận): biến thể này = "hiện banner lỗi rồi đóng form".

### 4.4. Vì sao không còn nhánh nào khác

- `case_048` chỉ có `if (==1)` / `else if (==2)`; **không `else`, không `switch`, không `default`** → mọi SubOp khác rơi thẳng xuống epilogue (dòng 46-74) → **im lặng**.
- Epilogue `_LStrArrayClr/_LStrClr` (dòng 49-73) là **dọn dẹp stack cục bộ dùng chung cho mọi case** (giống hệt `case_047` dòng 43-70 và `case_050` dòng 78-105), **không phải logic nghiệp vụ**.

### 4.5. Hai method ảo — bằng chứng "no body" và suy luận VMT

#### 4.5.1. `gvar_007DA3B4` + `0x20` (0 tham số)

- **Toàn bộ call-site trong SSOT** (grep `gvar_007DA3B4 + 0x20`): **đúng 6 match, tất cả là call-site**, không có định nghĩa method:
  1. `case_functions/functions/case_048_007954A5_FUN_007954a5.c:29`
  2. `case_functions/functions/case_048_007954A5_FUN_007954a5.c:43`
  3. `case_functions/jumptable_0x78A9B6_cases.c:8446`
  4. `case_functions/jumptable_0x78A9B6_cases.c:8460`
  5. `functions/0078a89c_FUN_0078a89c.c:6828`
  6. `functions/0078a89c_FUN_0078a89c.c:6845`
- **Không có file body** method này; **không có dump VMT** `VMT_58A680_TSe_CafeIDForm` trong `ts_decompile/` (grep `58A680` chỉ thấy 2 chỗ: `0051189c.c:1186` + `0051189c.asm.txt:1187` — chỉ là con trỏ VMT, không phải nội dung bảng).
- **Suy luận slot (raw offset, KHÔNG khẳng định tên)**:
  - `**(int **)gvar_007DA3B4` = `Self` (đối tượng). `*(Self)` = con trỏ VMT.
  - `VMT + 0x20` = byte offset **32** = **entry dword thứ 9** / **index 8 (0-based)** trong bảng method ảo (mỗi slot 4 byte).
  - Trong bố cục VMT Delphi, các mục RTTI/hook (`vmtSelfPtr`, `vmtClassName`, `vmtDestroy`…) nằm ở **offset ÂM** (ví dụ `vmtDestroy = -4`), nên **offset 0** là mục method ảo dương đầu tiên; do đó `+0x20` là slot index 8 tính từ mốc 0.
  - **Quy ước nội bộ dự án** (đã xác minh ở tài liệu khác): trên một form VCL, `vtable+0x20` = **`Hide`**, `vtable+0x24` = **`Show`** (cặp liền kề `TControl.Hide`/`Show`). Củng cố từ SSOT: `functions/0051f5c4_FUN_0051f5c4.c:30` gọi `(**(code **)(*param_1 + 0x24))();` **ngay sau** khi gửi `SendCommand(0x23)` trên chính form này (§6). → `+0x20` **nhiều khả năng = `Hide`**.
  - **Nhãn `Hide` là SUY LUẬN**; muốn chốt phải redump VMT `0x0058A680` (hoặc dump bảng method của `TSe_CafeIDForm`).
- **Call form**: `(**(code **)(**(int **)gvar_007DA3B4 + 0x20))();` — decompiler hiển thị **0 tham số**; theo quy ước register Delphi, `Self` được truyền ngầm (đối tượng đã nằm trong thanh ghi), nên callee nhận `Self = TSe_CafeIDForm`. **Đây là method ảo, không phải hàm toàn cục.**

#### 4.5.2. `gvar_007DA084` + `0x90` (self, text, duration, x, y)

- **Call-site rất nhiều**: grep trong `case_functions/functions/` có **>130 match**; trong `functions/` có **>80 match**. Đây là method **dùng chung toàn hệ thống** để phát banner/thông báo (toast) trên `TSe_TalkMsgFormPlus`.
- **Ví dụ tiêu biểu** (tất cả cùng chữ ký `(self, textPtr, duration, 0, 0)`):
  - `case_048_...c:39,42` — duration `2000`, text `UNK_007991d8` / `UNK_007991ec` (chính OP 0x37).
  - `case_functions/functions/case_029_007928E4_FUN_007928e4.c:39-60` — 8 banner `UNK_00798418…UNK_00798594`, duration `2000`.
  - `case_functions/functions/case_031_00792BAD_FUN_00792bad.c:63-162` — nhiều banner `DAT_007985f0…DAT_00798864`, duration `2000`.
  - `case_functions/functions/case_050_00795579_FUN_00795579.c:72,75` — `UNK_00799200` / `UNK_00799224`, duration `0x4b0` (1200) — **anh em của OP 0x37, xem §4.6**.
  - `case_functions/functions/case_054_007957DC_FUN_007957dc.c:50-82` — duration `0x898` (2200) / `0x4b0` (1200).
  - `functions/0077f414_FUN_0077F414.c:110,115` — `DAT_0051f7e4, 3000` / `DAT_0051f800, 2000` trong luồng validate CafeID.
- **Suy luận vai trò**: "hiển thị một dòng thông báo văn bản với thời lượng `duration` (ms)"; 2 tham số cuối (`0,0`) chỉ là hằng 0 (khả năng cao là tọa độ/định vị mặc định). **Đây là SUY LUẬN**; **thân method `TSe_TalkMsgFormPlus`+0x90 KHÔNG có trong SSOT** (chỉ có `FUN_0063bb3c` = constructor; không có file method nào khác của class này).
- **Giới hạn**: vì text pointer là **con trỏ dữ liệu thô**, không đọc được nội dung nếu literal không có dump (§5).

### 4.6. So sánh với các OP anh em

| OP | Case / handler | Cấu trúc payload | Có dùng `gvar_007DA084`+0x90 (banner)? | Có dùng `gvar_007DA3B4`? |
| :-- | :-- | :-- | :---: | :---: |
| **0x36** | 47 / `FUN_00795494` | **không SubOp**: dãy cặp `[idx][level]` lặp, gọi `FUN_00708900(gvar_007D9CC4, idx, lv)` | **KHÔNG** | **KHÔNG** |
| **0x37** | 48 / `FUN_007954a5` | `[SubOp]` hoặc `[SubOp][variant]` | **CÓ** (2 banner, 2000 ms) | **CÓ** (method +0x20) |
| **0x39** | 50 / `FUN_00795579` | `[SubOp=FG16LE]`; SubOp 1 → `func_0x0055374c(gvar_007D9D88, RP[1], 0)`; SubOp 2 → `FUN_00553410(gvar_007D9D88)`; SubOp 3 + `RP[1]=1/2` → banner | **CÓ** (2 banner `UNK_00799200/24`, **1200 ms**) | KHÔNG |

Chi tiết đối chiếu:
- **OP 0x36** (`case_047_00795494_FUN_00795494.c:23-42`): vòng lặp `while _LStrLen(RP)>=1` lấy `a=RP[0]`, `b=RP[1]`, gọi `FUN_00708900(gvar_007D9CC4,a,b)` rồi `_LStrDelete(RP,1,2)` — **không phải baner, chỉ set mức tín hiệu server**.
- **OP 0x39** (`case_050_00795579_FUN_00795579.c:32-77`): SubOp `1` đọc `RP[1]` → `func_0x0055374c(*(gvar_007D9D88), RP[1], 0)`; nếu `RP[1]==3` đọc tiếp `RP[2]` để set `*(gvar_007DA778+0x38)=100/1000`. SubOp `2` → `FUN_00553410(gvar_007D9D88)`. SubOp `3` + `RP[1]` = banner 1200 ms. → OP 0x39 và 0x37 **chia sẻ đúng khuôn `gvar_007DA084+0x90` + literal `.data` + ngay sau `case_047/048/050` trong jump table** (byte table hàng 4: `0x36→2F`, `0x37→30`, `0x38→31`, `0x39→32`).

---

## 5. Chuỗi VISCII → UTF-8

### 5.1. Trạng thái dump

Grep toàn bộ `ts_decompile/` cho các địa chỉ `007991d8|007991ec|00799200|00799224` → **chỉ đúng 8 match, tất cả là call-site truyền con trỏ**:

| Địa chỉ | Nơi tham chiếu | Dump bytes? |
| :-- | :-- | :-- |
| `0x007991D8` | `case_048_...c:39`; `jumptable_0x78A9B6_cases.c:8456` | **KHÔNG** |
| `0x007991EC` | `case_048_...c:42`; `jumptable_0x78A9B6_cases.c:8459` | **KHÔNG** |
| `0x00799200` | `case_050_...c:72`; `jumptable_0x78A9B6_cases.c:8626` | **KHÔNG** |
| `0x00799224` | `case_050_...c:75`; `jumptable_0x78A9B6_cases.c:8629` | **KHÔNG** |

- `redump/` **chỉ có 16 file `lit_*.hex`**: các địa chỉ `5957D8, 595800, 595810, 595828, 595844, 596078, 77F771, 78A854, 7A2094, 7A20A8, 7ABD54, 7ABDAC, 7ABDF0, 7ABE40, 7ABE64, 7ABE74` — **KHÔNG có file nào tại `0x007991xx`/`0x007992xx`**.
- Không có thư mục `.rodata`/`strings` riêng trong SSOT (`find -type d` chỉ ra `functions/`, `case_functions/`, `redump/`).
- Tiền tố **`UNK_`** (khác `DAT_`/`s_`) cho thấy IDA **không nhận diện được vùng nhớ này là dữ liệu** tại thời điểm dump → **không có bytes**.

### 5.2. Kết luận decode

> **Chưa có dump trong SSOT → chưa decode được. Cần redump tới null-terminator tại `0x007991D8`, `0x007991EC` (OP 0x37) và `0x00799200`, `0x00799224` (OP 0x39).**

| Địa chỉ | Raw bytes | Decoded (VISCII/cp1258 → UTF-8) |
| :-- | :-- | :-- |
| `0x007991D8` | *không có trong SSOT* | **N/A — cần redump** |
| `0x007991EC` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799200` | *không có trong SSOT* | **N/A — cần redump** |
| `0x00799224` | *không có trong SSOT* | **N/A — cần redump** |

**Chưa xác định được** đây là `PChar` (null-terminated) hay Delphi `AnsiString` (có length prefix tại `ptr-4`); cả hai khả năng đều cần bytes thực tế mới phân biệt được. **Không suy đoán nội dung.**

- Ghi chú ngữ cảnh: `0x00799200` và `0x00799224` là **anh em trực tiếp** của `0x007991D8/EC` về vị trí (cùng nằm quanh `0x799200`, cách nhau `0x14`/`0x24`), dùng cùng method banner, chỉ khác **duration 1200 ms** thay vì 2000 ms → gợi ý **cùng một "bảng chuỗi thông báo" cục bộ dùng chung**, nhưng **không thể xác minh nội dung**.

---

## 6. Chiều Client → Server

### 6.1. Vị trí

`functions/0077f414_FUN_0077F414.c:1012-1019` — `case 0x37:` của `switch(param_2 & 0xff)` (switch mở tại dòng 768):

```c
case 0x37:
  local_24 = &stack0xfffffffc;
  FUN_00402b90(&stack0xffffffc4,&stack0xffffffc8);
  _PStrNCat(&stack0xffffffc4,&stack0xffffffc0,2);
  _LStrFromString((int *)&local_810,&stack0xffffffc4);
  _LStrCat((int *)&local_810,*(undefined4 **)(*(int *)(*(int *)gvar_007DA3B4 + 0x138) + 0x1a0));
  TForm1_CY_AddSedQueue(*(undefined4 *)gvar_007DA664,local_810);
  break;
```

### 6.2. Dịch ngược từng bước (đối chiếu ASM)

ASM thật cùng case: `functions/0077f414_FUN_0077F414.asm.txt:3561-3589`:

```asm
3561 LEA EAX,[EBP + -0x34]          ; buf A
3562 MOV DL,byte ptr [EBP + -0x5]   ; DL = MainOp (= 0x37)
3563 MOV byte ptr [EAX + 0x1],DL    ; A[1] = 0x37
3564 MOV byte ptr [EAX],0x1         ; A[0] = length 1  → shortstring [01][37]
3565 LEA EDX,[EBP + -0x34]          ; src = A
3566 LEA EAX,[EBP + -0x38]          ; dest = buf B
3567 CALL 0x00402b90                ; B = copy(A)
3568 LEA EAX,[EBP + -0x3c]          ; buf C
3569 MOV DL,byte ptr [EBP + -0x6]   ; DL = CL (tham số thứ 3 lưu ở prologue)
3570 MOV byte ptr [EAX + 0x1],DL    ; C[1] = CL
3571 MOV byte ptr [EAX],0x1         ; C[0] = length 1  → shortstring [01][CL]
3572 LEA EDX,[EBP + -0x3c]          ; src = C
3573 LEA EAX,[EBP + -0x38]          ; dest = B
3574 MOV CL,0x2                     ; MaxLen = 2
3575 CALL 0x00402b60                ; @PStrNCat(B, C, 2)
3576 LEA EDX,[EBP + -0x38]          ; src = B (shortstring)
3577 LEA EAX,[EBP + 0xfffff7f4]     ; dest = AnsiString
3578 CALL 0x0040402c                ; @LStrFromString
3579 LEA EAX,[EBP + 0xfffff7f4]
3580 MOV EDX,[0x007da3b4]           ; EDX = gvar_007DA3B4
3581 MOV EDX,[EDX]                  ; EDX = Self (TSe_CafeIDForm)
3582 MOV EDX,[EDX + 0x138]          ; EDX = Self.field_138 (TSe_Editor)
3583 MOV EDX,[EDX + 0x1a0]          ; EDX = editor.text (AnsiString)
3584 CALL 0x00404090                ; @LStrCat(payload, editor.text)
3585 MOV EDX,[EBP + 0xfffff7f4]
3586 MOV EAX,[0x007da664]
3587 MOV EAX,[EAX]
3588 CALL 0x0051633c                ; TForm1.CY_AddSedQueue
```

### 6.3. Từng helper

- **`FUN_00402b90(dest, src)` — copy chuỗi dài lượng ngắn** (body thật trong SSOT: `functions/00402b90_FUN_00402b90.c:392-411`):
  ```c
  uVar1 = *param_2 + 1 & 3;       // phần lẻ (theo 4 byte) của (len+1)
  uVar2 = *param_2 + 1 >> 2;      // số khối 4 byte
  ... copy uVar2*4 + uVar1 byte = đúng (*param_2 + 1) byte ...
  ```
  → Copy **`length_byte + length_byte` byte** (tức `*src + 1` byte) từ `src` sang `dest`. Ở đây `A = [01][37]` → `B = [01][37]`.
- **`_PStrNCat(dest, src, 2)` — `@PStrNCat` @ `0x00402B60`** (`case_functions/...` header `functions/0051f5f4_FUN_0051f5f4.c:19` ghi rõ `@PStrNCat @ 00402b60`):
  - **Không có body trong SSOT** (glob `00402b60*` → 0 file; không có trong `index.csv`) — đây là hàm RTL Borland.
  - Ngữ nghĩa (theo RTL + đối chiếu chuỗi ghép): **nối (append) `src` vào `dest`, giới hạn độ dài đích = 2** → `B` từ `[01][37]` thành **`[02][37][CL]`** (thêm đúng 1 ký tự `CL`).
  - **Đây là "ghép header 2 byte `[DL][CL]` = `[OpCode][tham số]`"** — đúng khuôn đã hiệu chuẩn ở `opcode_03.md`/`opcode_06.md` (case 6: `_PStrNCat(buf,...,2)` ghép `[DL][CL]`).
- **`_LStrFromString(dest, src)` — `@LStrFromString` @ `0x0040402C`** (RTL, không body): chuyển **Pascal shortstring** (byte [0] = độ dài) sang **Delphi AnsiString** → **bỏ byte độ dài**, nội dung `dest` = `37 CL` (2 byte).
- **`_LStrCat(dest, src2)` — `@LStrCat` @ `0x00404090`** (RTL, không body): nối AnsiString → payload cuối = **`0x37, CL, <toàn bộ text của editor>`**.
  - `src2` = `*(int*)(*(int*)(*(int*)gvar_007DA3B4 + 0x138) + 0x1a0)`.
- **`gvar_007DA3B4 + 0x138` = control editor của form CafeID** — xác minh từ constructor `functions/005b0774_FUN_005b0774.c`:
  - dòng 49 `piVar1 = TSe_Editor_Create((int *)VMT_7AF318_TSe_Editor,'\x01',param_1);`
  - dòng 50 `param_1[0x4e] = (int)piVar1;` — **`0x4e × 4 = 0x138`** → field `+0x138` = instance `TSe_Editor`.
  - dòng 52 control này được đặt tên **`"editorBG"`** (`(**(code **)(*piVar1 + 8))(piVar1,"editorBG",...)`).
- **`editor + 0x1a0` = nội dung text đang nhập (AnsiString)** — bằng chứng chéo nhiều nơi trong SSOT:
  - `functions/0051f5f4_FUN_0051f5f4.c:69` — `_LStrLAsg(&local_10, *(int *)(*(int *)(param_1 + 0x138) + 0x1a0));` rồi lấy 2 ký tự đầu, `UpperCase`, `StrToIntDef` **phần còn lại** vào `param_1+0x134`; nếu sai → phát banner lỗi. → **`+0x1a0` là text của editor** và **`+0x134` là số đã parse**.
  - `functions/005492dc_FUN_005492dc.c:28-35` — `_LStrLen(*(int*)(param_1[0x4d] + 0x1a0))` + `StrToInt(...)` → `+0x1a0` là text số của edit.
  - `functions/007b3f78_FUN_007b3f78.c:146-152,188-194,273` — `_LStrDelete((int*)(local_8+0x1a0),...)` / `_LStrInsert(*(undefined4**)(local_8+0x1cc), (int*)(local_8+0x1a0), ...)` → `+0x1a0` là **buffer soạn thảo** của editor.
  - → **SUY LUẬN (độ tin cậy CAO): `+0x1a0` = text hiện tại của editor**, và vì `0x138` là editor của form CafeID, payload C→S = **ID mà người chơi đang gõ**.
- **`TForm1.CY_AddSedQueue(queue, payload)`** (body: `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181`): tính `L = _LStrLen(payload)`, mã hoá `L` thành **2 byte LE** qua `FUN_0077eb1c(DAT_009264a0,(ushort)L,...)` (dòng 173), rồi `_LStrCatN(&local_10,3,...,payload,Lbytes,String1)` (dòng 174, `String1 = DAT_005163e0`) — theo quy ước `_LStrCatN` **nối NGƯỢC thứ tự liệt kê** *(quy ước đã hiệu chuẩn ở `opcode_06.md` §5.1, không phải bằng chứng SSOT)* → buffer gửi = **`DAT_005163e0` + `L(2B LE)` + `payload`**, tức **khung `[Token F4 44][L Word LE][payload]`**, đẩy vào queue (`FUN_00412f74(DAT_00926e8c)` dòng 175). → **payload case 0x37 chính là nội dung giữa Length và Token sau khi ghép**.

### 6.4. Wire C→S (suy ra)

```
Payload C→S = [0x37][CL][bytes của editor.text]        (không XOR ở tầng này; XOR áp khi ghi socket)
```

- `CL` là **tham số thứ 3** của `TFConnect.SendCommand` — được lưu vào `[EBP-0x6]` ở prologue (`0077f414.asm.txt:11` `MOV byte ptr [EBP + -0x6],CL`), trong khi `[EBP-0x5] = DL = MainOp`. Signature decompile (`FUN_0077f414(undefined4 param_1, uint param_2)`) **chỉ hiện 2 tham số**; `CL` là **byte tham số ẩn (thanh ghi ECX)**.
- **Giá trị `CL` cho case 0x37: CHƯA XÁC ĐỊNH** — không tìm thấy call-site nào nạp `MOV DL,0x37` trước `CALL 0x0077f414` trong toàn bộ asm đã export (grep `MOV DL,0x37` → **0 file**). Đối chiếu: opcode submit **0x23** có call-site đã export rõ ràng — `functions/0051f5c4_FUN_0051f5c4.c:27-30` / `.asm.txt:14-16`:
  ```asm
  MOV CL,0x5
  MOV DL,0x23
  CALL 0x0077f414
  ```
  → cùng form CafeID nhưng **opcode 0x23** (khác 0x37). **Nhiều khả năng call-site của 0x37 nằm trong vùng HOLE chưa export** (đối chiếu `opcode_36.md` §6 gặp tình trạng tương tự với `[0x36]`).
- **Độ tin cậy**: wire `[0x37][CL][editor.text]` — **CAO** (asm đọc trực tiếp); **giá trị `CL` và ý nghĩa nghiệp vụ chính xác — UNKNOWN**.
- **Bất đối xứng cần lưu ý**: C→S gửi `[0x37][CL][text]`, còn S→C handler **chỉ đọc `RP[0]` và `RP[1]`** — server **không** parse chuỗi ID vào nhánh `case 0x37` này (chuỗi ID nằm ở **payload gửi lên**, còn nhánh S→C `0x01/0x02` chỉ là **kết quả trả về**). Điều này khớp mô hình **request (C→S `0x37`) → response (S→C `0x37`)**.

---

## 7. Ghi chú cho Mock Server

### 7.1. S→C (server gửi xuống client)

Payload sau MainOp; khung ngoài = `[Token F4 44][Length L: Word LE][Payload]`, **toàn khung XOR 0xAD** (L tính từ byte MainOp `0x37`):

```
[37]              L=1  → ERangeError (BoundErr(0))        — ĐỪNG GỬI
[37][00]          L=2  → no-op im lặng
[37][01]          L=2  → gọi TSe_CafeIDForm.vtable+0x20  (nhiều khả năng = ẩn form)
[37][02]          L=2  → ERangeError (BoundErr(1))        — ĐỪNG GỬI
[37][02][01]      L=3  → banner UNK_007991d8 (2000 ms)
[37][02][02]      L=3  → banner UNK_007991ec (2000 ms) rồi gọi vtable+0x20
[37][02][xx>=03]  L=3  → no-op im lặng
[37][xx>=03]      L=2  → no-op im lặng
[37][01][...dư]   L>2  → vẫn chỉ gọi vtable+0x20 (byte dư bỏ qua)
```

**Ví dụ khung hoàn chỉnh** (đã XOR):

```
# [37][01] ẩn form:
payload  = 37 01
L (LE)   = 02 00
pre-XOR  = F4 44 02 00 37 01
on-wire  = 59 E9 AF AD 9A AC

# [37][02][01] hiện banner A:
payload  = 37 02 01
L (LE)   = 03 00
pre-XOR  = F4 44 03 00 37 02 01
on-wire  = 59 E9 AE AD 9A AF AC

# [37][02][02] hiện banner B rồi ẩn form:
payload  = 37 02 02
L (LE)   = 03 00
pre-XOR  = F4 44 03 00 37 02 02
on-wire  = 59 E9 AE AD 9A AF AF
```

- **Bắt buộc**: nếu phát banner thì **phải đủ 3 byte** (`L≥3`); nếu chỉ muốn ẩn form dùng `[37][01]` (L=2).
- **Không gửi** `[37]` (L=1) và `[37][02]` (L=2) — cả hai gây `ERangeError` (delphi exception) ngay trong handler.
- **Nội dung banner chưa đọc được** (literal chưa dump, §5): mock chỉ cần chọn đúng **địa chỉ literal** bằng cách gửi `RP[1]=01/02`; không thể kiểm tra text client hiển thị.
- **Hiệu ứng `vtable+0x20` (Hide) là suy luận** — mock nên phát `[37][01]` và quan sát UI client để xác nhận.

### 7.2. C→S (client gửi lên — server phải nhận)

```
Payload nhận được (sau khi bỏ Token/Length và giải XOR):
  [0x37][CL][... text ID người chơi đã nhập ...]
```
- Server nên **đọc `0x37` là "submit Cafe ID"**, parse phần text phía sau (encoding dự kiến VISCII/cp1258 — chưa xác minh), và trả lời bằng một trong các gói S→C ở §7.1.
- `CL` là byte thứ 2 — **ý nghĩa chưa xác định**; mock nên log cả `CL` để dò khi có traffic thật.
- **Không có tiền tố độ dài cho chuỗi ID**: chuỗi chạy tới **hết payload** (độ dài do khung `L` quyết định).

---

## 8. Source trail

| # | Nguồn (trong `ts_decompile/`) | Dùng để |
| :-- | :-- | :-- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, index `0x37` = `0x30`=48) | Byte table MainOp 0x37 → 48 |
| 2 | `redump/jumptable_0x78A9B6_case_functions.csv:48`; `redump/jumptable_dword200_0x78A9B6.hex` (hàng 13, entry#48 = `A5 54 79 00`); `case_functions/manifest.csv:48` | Dword table entry #48 → `0x007954A5` |
| 3 | `case_functions/functions/case_048_007954A5_FUN_007954a5.c:21-45` (handler, 77 dòng) | Lõi logic S→C |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:8419-8462` | Bản gộp case 48 |
| 5 | `functions/0078a89c_FUN_0078a89c.c:6816-6848` | Bản inline dispatcher — xác nhận 1:1 |
| 6 | `functions/0051189c_FUN_0051189c.c:1186-1189` | `gvar_007DA3B4` = `TSe_CafeIDForm` (create + register) |
| 7 | `functions/0051189c_FUN_0051189c.c:1265-1277` | `gvar_007DA084` = `TSe_TalkMsgFormPlus` (create + `+8`/"panel10" + register) |
| 8 | Grep `gvar_007DA3B4 + 0x20` (6 call-site, 0 body); grep `58A680` (2 chỗ, không phải VMT dump) | Xác nhận method `+0x20` không có body/không có VMT dump |
| 9 | Grep `gvar_007DA084 + 0x90` (>130 call-site trong `case_functions/functions/`, >80 trong `functions/`); `functions/0063bb3c_FUN_0063bb3c.c` (chỉ constructor) | Xác nhận `+0x90` = method banner dùng chung, **không có body** |
| 10 | `case_functions/functions/case_029_...c:39-60`, `case_031_...c:63-162`, `case_050_...c:72,75`, `case_054_...c:50-82` | Đối chiếu họ banner `+0x90` |
| 11 | `case_functions/functions/case_047_00795494_FUN_00795494.c:23-42`; `case_050_00795579_FUN_00795579.c:32-77` | So sánh OP 0x36 / 0x39 |
| 12 | Grep `007991d8\|007991ec\|00799200\|00799224` (8 match, toàn call-site); `ls redump/lit_*.hex` (16 file, không có `7991xx`) | Xác nhận literal **chưa có dump** |
| 13 | `functions/0077f414_FUN_0077F414.c:1012-1019` + `.asm.txt:3561-3589` | Chiều C→S (case 0x37) |
| 14 | `functions/00402b90_FUN_00402b90.c:392-411` | Body `FUN_00402b90` (copy length-prefixed string) |
| 15 | `functions/005b0774_FUN_005b0774.c:49-52` (`param_1[0x4e]` = `TSe_Editor` "editorBG") | `gvar_007DA3B4+0x138` = editor |
| 16 | `functions/0051f5f4_FUN_0051f5f4.c:69-126`; `005492dc_FUN_005492dc.c:28-35`; `007b3f78_FUN_007b3f78.c:146-152` | `editor+0x1a0` = text |
| 17 | `functions/0051f5c4_FUN_0051f5c4.c:27-31` + `.asm.txt:12-19` (`MOV CL,0x5; MOV DL,0x23`) | Call-site mẫu + xác nhận convention `DL=OpCode, CL=tham số`; `+0x24` = Show |
| 18 | `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181` | Đóng gói khung `[Token][L 2B LE][payload]` bằng `FUN_0077eb1c` |
| 19 | Grep `MOV DL,0x37` trong `*.asm.txt` → 0 file | Xác nhận call-site C→S `0x37` **không có trong asm export** (nghi vấn vùng HOLE) |

### Điểm chưa xác minh được từ SSOT (không suy đoán)

1. **Thân method `gvar_007DA3B4` + `0x20`** (`TSe_CafeIDForm`): không có body, không có dump VMT `0x0058A680` → **unknown**. Nhãn `Hide` chỉ là **suy luận** từ cặp slot `+0x20`/`+0x24` và quy ước VCL.
2. **Thân method `gvar_007DA084` + `0x90`** (`TSe_TalkMsgFormPlus`): không có body (chỉ có constructor `FUN_0063bb3c`) → **unknown**. Vai trò "show banner text + duration" là **suy luận mạnh** từ 200+ call-site nhưng chưa xác minh thân.
3. **Nội dung `UNK_007991d8` / `UNK_007991ec`** (banner OP 0x37) và `UNK_00799200` / `UNK_00799224` (OP 0x39): **không có dump trong SSOT → chưa decode được; cần redump tới null-terminator**. Chưa xác định `PChar` hay Delphi `AnsiString`.
4. **Giá trị byte `CL` của C→S `case 0x37`** và **call-site gọi `SendCommand(0x37)`**: không có trong asm export → **unknown** (nghi vấn nằm trong vùng HOLE).
5. **Ý nghĩa nghiệp vụ chính xác của OP 0x37** (luồng "submit Cafe ID" / "ẩn form / banner lỗi") là **suy luận** từ cặp C→S `0x37` + S→C `0x01/0x02`; chưa có traffic thật để xác nhận.
6. **Encoding chuỗi ID** trong payload C→S (VISCII/cp1258/ASCII) **chưa xác minh**.
7. **Định danh chính xác của `gvar_007DA3B4+0x138`** đã xác minh là `TSe_Editor` (từ `param_1[0x4e]`), **nhưng** nhãn "editor nhập Cafe ID" là **suy luận** từ tên class + luồng validate `FUN_0051f5f4` (2 ký tự đầu + số phía sau, so với `DAT_0051f7d8`).
