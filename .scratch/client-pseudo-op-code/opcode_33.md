# PHÂN TÍCH — Main OP 0x33 (51) / Case 44 / FUN_0079522c @ 0x0079522C

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh phần vỏ (framing/dispatch/sub-op) từ mã nguồn sơ cấp** (`ts_decompile/` only). **Hàm callee `FUN_0064cae0` ĐÃ có body từ bản redump 2026-09-14** — nội dung nghiệp vụ đã phân tích ở §4.1. Chỉ có **1 nhánh SubOp duy nhất** (`0x01`), không có `default`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Main OP 0x33 là một **lệnh điều khiển đơn giản (thin wrapper)**: nó chỉ đọc 1 byte SubOp từ `RestPayload`; nếu `SubOp == 1` thì gọi **một method của đối tượng `TFightManage`** (con trỏ lấy từ `gvar_007D9CE4`) với chính `RestPayload` làm tham số thứ hai. Mọi SubOp khác (`0x00`, `0x02..0xFF`) **im lặng bỏ qua** (không `default`, không log, không side-effect).
- **Không đọc thêm field nào** trên wire: ngoài 1 byte SubOp, handler **không** gọi `_LStrCopy`, `FUN_0077ef7c`, `FUN_0077f098`, `FUN_0077eb9c`… → không có cấu trúc payload phụ ở tầng handler.
- **Không có banner / chuỗi / VISCII** trong đường đã xác minh: handler không tham chiếu literal `.rodata`, không gọi `_LStrCatN`, không gọi API banner `(VMT+0x90)`.
- **Calldeep**: ~~`func_0x0064cae0` chỉ xuất hiện dưới dạng call site, không có file body~~ **(đính chính 2026-09-14)**: body **đã có** tại `ts_decompile/functions/0064cae0_FUN_0064cae0.c` (199 dòng, 649 byte, entry `0064cae0` đã vào `index.csv`; header ghi rõ caller `sub_0079525c` = đuôi handler case 44 — `0064cae0_FUN_0064cae0.c:8,13`). Nội bộ là bộ parse **chuỗi record 7 byte trên lưới trận** (xem §4.1). Trước đây bản md ghi đúng tại thời điểm viết (0 file/3 call-site); nay HOLE đã được bổ sung.
- **Class hint (suy luận có cơ sở)**: `gvar_007D9CE4` là slot toàn cục giữ con trỏ tới object do `TFightManage_Create` tạo, theo `ts_decompile/functions/0050a4a0_TForm1.FormCreate.c:615-617`:
  ```c
  piVar6 = TFightManage_Create((int *)VMT_63CCDC_TFightManage,'\x01',extraout_ECX_28);
  *(int **)gvar_007D9CE4 = piVar6;
  FUN_006449b4(*(int *)gvar_007D9CE4,0xffffffff);
  ```
  → `*(undefined4*)gvar_007D9CE4` chính là con trỏ `Self` của một **TFightManage**. Do đó OP 0x33 SubOp 1 nhiều khả năng thuộc **hệ thống chiến đấu / võ đài (fight)**. **Đây là suy luận từ ngữ cảnh khởi tạo + họ callee, KHÔNG phải hành vi đã xác minh** (thân hàm không có trong SSOT).
- **Chiều C→S vắng mặt**: `ts_decompile/functions/0077f414_FUN_0077F414.c` **không có `case 0x33:`** (chi tiết ở §6) → client không gửi OP này; kết luận **S→C một chiều**.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x33 (51) → byte_table[0x78A8EE][0x33] = 0x2C (44)
                 → dword_table[0x78A9B6][44] @ 0x0078AA66 = 0x0079522C
                 → FUN_0079522c (Case 44)
```

- Bảng byte 200: `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** (dòng 4 của file) ứng với index `0x30..0x3F`: `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Tại offset `0x33` (byte thứ 4 của hàng) giá trị = **`0x2C` = 44**.
- Bảng dword: `ts_decompile/redump/jumptable_0x78A9B6_case_functions.csv:46` → `44,0x0078AA66,0x0079522C,YES,"FUN_0079522c",0x0079522C,1`.
- Manifest: `ts_decompile/case_functions/manifest.csv:46` → `44,0x0078AA66,0x0079522C,EXPORTED,"FUN_0079522c","0079522c",1,"functions/case_044_0079522C_FUN_0079522c.c"`.
- File chính: `ts_decompile/case_functions/functions/case_044_0079522C_FUN_0079522c.c` (61 dòng).
- Bản gộp jump table: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:8117-8145` (header comment "Case index: 44 / Target: 0x0079522C / Function: FUN_0079522c", hàm tại dòng 8124, call tại dòng 8144).
- Bản inline trong dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6693-6707` — **khớp 1:1** với file case (xem §2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `opcode_00_01.md` §2 và `opcode_2d.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x33` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile, `unaff_EBP + -0xc` = con trỏ dữ liệu của `RestPayload` (kiểu Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RestPayload - 4)` = **độ dài** chuỗi RestPayload (length prefix của AnsiString).

### 2.3. Đọc SubOp (`case_044` dòng 20-29)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);      // iVar2 = RestPayload (con trỏ data)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {         // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                  // → RangeError (L=1, payload chỉ có [33])
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
if (*(int *)(unaff_EBP + -0x14) == 1) {                           // SO SÁNH == 1
  func_0x0064cae0(*(undefined4 *)gvar_007D9CE4,                   // this = TFightManage
                  *(undefined4 *)(unaff_EBP + -0xc));             // arg = RestPayload
}
```

Diễn giải:
- Nếu `RestPayload` **rỗng** → `_BoundErr(0)`; nhánh `iVar2 = extraout_EDX` và `iVar1` chỉ là đường phục hồi giả của decompiler cho nhánh exception (thực tế là lỗi range). **L=1 → RangeError** (giống `opcode_2d.md` §2.3).
- Ngược lại `SubOp = RP[0]`, rồi so sánh **`== 1`** (không phải `switch`). Chỉ đúng `0x01` mới gọi callee.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, **không gọi** ở chiều này |
| `_BoundErr` | Chốt chặn `RestPayload` rỗng (2 dòng 22-25) |
| `FUN_0064cae0` | **Callee duy nhất**; body **đã có** (2026-09-14) — `functions/0064cae0_FUN_0064cae0.c` |
| `gvar_007D9CE4` | Slot giữ con trỏ `TFightManage`; truyền `Self` cho callee |
| `_LStrArrayClr/_LStrClr` (dòng 30-57) | **Dọn dẹp frame cục bộ** của dispatcher (artifact dùng chung cho khối case), không liên quan wire |

### 2.5. Đối chiếu file case vs bản inline (1:1)

| Vị trí | `case_044_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 20 `iVar2 = *(int*)(unaff_EBP + -0xc)` | dòng 6695 `iVar21 = local_10` (`local_10` = RestPayload) |
| Check rỗng | dòng 22 `if (*(int*)(iVar2 + -4) == 0)` | dòng 6697 `if (*(int *)(local_10 + -4) == 0)` |
| SubOp | dòng 26 `... = *(byte*)(iVar2 + iVar1)` | dòng 6703 `if (*(char *)(iVar21 + iVar20) == '\x01')` |
| Call | dòng 28 `func_0x0064cae0(*(undefined4*)gvar_007D9CE4, *(undefined4*)(unaff_EBP + -0xc))` | dòng 6705 `func_0x0064cae0(*(undefined4*)gvar_007D9CE4, local_10)` |

Khác biệt duy nhất: tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX` vs `iVar20/iVar21/extraout_EDX_x00165`) và nhãn handler exception `&UNK_0079523f` (dòng 6698). **Logic trùng khớp hoàn toàn.**

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler **không dùng `switch`** mà so sánh `== 1`; **không có `default`**.

| SubOp (RP[0]) | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| *(rỗng)* | `[33]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **RangeError** (dòng 22-25) |
| `0x00` | `[33][00]` | `SubOp = RP[0] = 0` | Không khớp `==1` → **im lặng, no-op** |
| `0x01` | `[33][01]` + chuỗi record 7B (`[grp][slot][flag][DWORD]`×n, xem §4.1) | `SubOp = RP[0]` | **Gọi `FUN_0064cae0(*(gvar_007D9CE4), RP)`** — body **đã có** (2026-09-14): áp hiệu ứng/trạng thái lên unit tại ô trận |
| `0x02..0xFF` | `[33][xx]` | `SubOp = RP[0] = xx` | Không khớp `==1` → **im lặng, no-op** |

**Tổng: đúng 1 nhánh có hiệu lực (`0x01`), 255 giá trị SubOp còn lại là no-op.** Đây là OP "một nhánh" — tương phản với OP 0x2D (16 nhánh) hay OP 0x35 (case 46, 12 nhánh).

---

## 4. Chi tiết các nhánh chính

### 4.1. Nhánh duy nhất `SubOp == 0x01` — chuyển tiếp sang `TFightManage`

```c
if (*(int *)(unaff_EBP + -0x14) == 1) {                         // case_044 dòng 27
  func_0x0064cae0(*(undefined4 *)gvar_007D9CE4,                 //      dòng 28
                  *(undefined4 *)(unaff_EBP + -0xc));
}
```

- **Tham số 1 (`this`)**: `*(undefined4*)gvar_007D9CE4` = con trỏ `TFightManage` (suy ra từ `FormCreate.c:615-617`; xem §1).
- **Tham số 2 (`arg`)**: chính con trỏ dữ liệu `RestPayload` (`unaff_EBP + -0xc`), **truyền nguyên** — tức callee nhận cả byte SubOp `0x01` ở offset 0 và tự parse phần còn lại.
- **Body callee — PHÂN TÍCH ĐẦY ĐỦ (mới, 2026-09-14, `functions/0064cae0_FUN_0064cae0.c`)**:
  - Chữ ký `void __register FUN_0064cae0(undefined4 param_1, int param_2)` (dòng 16) — param_2 = RP (AnsiString).
  - Số record: `n = (len(RP) − 1) / 7` (dòng 62-67); lặp `i = 0..n-1`, record thứ i bắt đầu tại vị trí 1-based `2 + i*7` (dòng 75-86). `n ≤ 0` → hàm không làm gì thêm.
  - **Wire mỗi record (7 byte)**, quy 0-based (`RP[0] = SubOp`):

    ```
    RP[1]     grp   : byte — nhóm (tra ô, BoundErr trong FUN_0065178c nếu >3)
    RP[2]     slot  : byte — ô (BoundErr nếu >4)
    RP[3]     flag  : byte — mã hiệu ứng/trạng thái
    RP[4..7]  value : DWORD LE (FUN_0077ef7c)
    ```
  - Với mỗi record, `u = FUN_0065178c(battle, grp, slot)` — unit index tại ô (trả `0xFFFFFFFF` nếu ô trống; `0064cae0_FUN_0064cae0.c:122-123`). Nếu tìm thấy:
    1. **Nhánh đặc biệt `flag == 0x19` (25)** (dòng 125-180): gọi `FUN_007108f4(unit, 0x19)`; nếu kết quả `< 1` **và** `value > 0` thì: ghi `*(byte*)(unit+0x391) = 0` (**về lại state 0** — byte STATE đã xác minh ở `opcode_35.md` §4.1), gọi virtual `**(unit.VMT+0x18)(unit, 0x10)`, rồi `FUN_0066e710(*(battle+0x1AC+u*4), 2,3,1, 0x70a3d70a, 0x3fd70a3d, 0,0, 0x32,1,0x1e, unit+0x58, unit+0x54, 2, 0x17,0x17,1, 0x1e, 0x279d)` — **dựng FX** trên khe FX của unit (mảng `battle+0x1AC` = 21 FX slot, đã biết ở OP 0x35; `unit+0x54/+0x58` = tọa độ X/Y, khớp các field ghi bởi OP 0x35 SubOp 7).
    2. **Luôn gọi** `FUN_00710ab0(unit, flag, value, 0)` (dòng 187) — body **đã có** trong SSOT (`functions/00710ab0_FUN_00710ab0.c`) nhưng chưa đọc chi tiết ở tài liệu này ⇒ tác động cụ thể theo từng `flag`: **chưa kết luận được** (cần đào sâu `FUN_00710ab0`).
  - **Không** có flush FX đầu hàm, không ghi battle-record flag, không gọi handler `gvar_007DA7BC`/`gvar_0098c63c` nào khác; không tham chiếu literal chuỗi.
- **Bằng chứng "không có body" (lỗi thời — giữ lai lịch)**:
  - Bản 2026-09-12: `glob ts_decompile/**/*0064cae0*` → 0 file; grep → 3 call-site (`case_044_...c:28`, `jumptable_0x78A9B6_cases.c:8144`, `0078a89c_FUN_0078a89c.c:6705`).
  - Bản 2026-09-14: các call-site trên giữ nguyên, **cộng thêm** file body `0064cae0_FUN_0064cae0.c` + `.asm.txt` và entry `index.csv` (`FUN_0064cae0`, 649 B) → HOLE liên quan **đã đóng**.

### 4.2. Vì sao không có nhánh nào khác

- `case_044` dòng 27 là `if (... == 1)`, **không có `else if`, không có `else`, không có `switch`, không `default`**. Sau khối `if` là epilogue dọn dẹp chuỗi cục bộ (dòng 30-58) rồi `return` (dòng 58).
- Do đó mọi `SubOp != 1` rơi thẳng xuống epilogue → **im lặng**.

### 4.3. Bối cảnh họ callee (dùng để đối chiếu, KHÔNG phải bằng chứng hành vi OP 0x33)

> Các quan sát dưới đây là **bối cảnh họ hàm chung `gvar_007D9CE4`** (không dùng để suy hành vi OP 0x33 — hành vi callee đã được xác minh trực tiếp ở §4.1 kể từ 2026-09-14).

- **Main OP 0x32 (Case 43, `0x007951DA`)**: `case_043_007951DA_FUN_007951da.c:27-32` — SubOp `1` → `func_0x00647164`, SubOp `2` → `func_0x0064dd74`. (`0078a89c_FUN_0078a89c.c:6684-6690`.)
- **Main OP 0x35 (Case 46, `0x007952AB`)**: `case_046_007952AB_FUN_007952ab.c:27-69` — `switch` SubOp `1,3,4,5,6,7,8,9,10,0xb,0xc,0xd,0xe,0xf` → lần lượt `func_0x0064cd74`, `FUN_0064cfec`, `func_0x0064d128`, `func_0x0064dfa4`, `func_0x0064e064`, `func_0x0064e3e0`, `func_0x0064e8f4`, `func_0x0064f9f0`, `func_0x0074789c`, `func_0x0075c518`, `func_0x0064faa8`, `func_0x0076a944`, `func_0x0064fbd8`, `func_0x0064fd34`. (Bản inline `0078a89c_FUN_0078a89c.c:6733-6789`.)
- **Main OP 0x16 (Case 19)**: `case_019_0078FEAF_FUN_0078feaf.c:130` dùng `FUN_0064246c(*(undefined4*)gvar_007D9CE4, ...)`.
- **Main OP 0x0B (Case 11)**: `case_011_0078D5D1_FUN_0078d5d1.c:59-117` dùng một loạt `FUN_00641fa4/func_0x0064bb24/FUN_00644b84/func_0x00642244/FUN_00644294/...` trên cùng `gvar_007D9CE4`.
- **Các call site khác ngoài dispatcher** (chứng minh `gvar_007D9CE4` là singleton dùng rộng khắp framework): `0050a4a0_TForm1.FormCreate.c:616-617`, `0050bff8_TForm1.DXDraw1MouseDown.c:96,100`, `0050c640_TForm1.DXDraw1DblClick.c:31`, `005aa09c_FUN_005aa09c.c:128`, `005cddd0_FUN_005cddd0.c:419`, `005f650c_FUN_005f650c.c:1865`, `0070d43c_FUN_0070d43c.c:113-157`, v.v.

→ Tất cả đều xác nhận `gvar_007D9CE4` = **TFightManage singleton** (`FormCreate.c:615`), và nhóm địa chỉ `0x0064xxxx` là unit cài đặt của TFightManage. **Bản 2026-09-14: `0064cae0` đã có body** (§4.1) — các callee OP 0x32/0x33/0x35 trong cùng dải đều đã được decompile (xem `opcode_32.md` §4.2-4.3, `opcode_35.md`), củng cố kết luận "họ method TFightManage".
>
> Hai helper của OP 0x0B nêu ở dòng trên cũng **vừa có body** (2026-09-14):
> - `FUN_0064bb24` (`functions/0064bb24_FUN_0064bb24.c`, 765 B): đọc `[grp][slot]` = `RP[1..2]`, nếu ô có unit và `unit+0x5B4 == 0` thì **dội 5 mảng FX** của unit (`FUN_0066d450/0066d804/0066de54/0066e37c/006bac4c`, id `0xb`, trên các mảng `battle+0x200/0x254/0x2A8/0x2FC/0x350` hệ số 21), rồi hoặc reset state (`FUN_006540b0` nếu `unit+0x391 ∈ {7,8}` hoặc `+0x79==2 && +0x574≠0`) hoặc bắn FX qua `FUN_00666bf0(battle+0x1AC[u*4], 6, u, 0)`.
> - `FUN_00642244` (`functions/00642244_FUN_00642244.c`, 540 B): chuỗi record 8 byte `[type][DWORD id][WORD idx][byte val]`; `type==2` → `*(actor+0x376)=val` với actor tra bằng `FUN_0070c20c(*gvar_007D9D34, id)` (mảng `gvar_007DA300`, biên 800); `type==6` → `*(gvar_007D9FB4[idx≤200]+0x376)=val`; `type==0x10` → `*(gvar_007DA10C[idx≤10]+0x376)=val` — **ghi byte trạng thái `+0x376`** lên 3 họ thực thể theo kiểu selector.

---

## 5. Chuỗi VISCII → UTF-8

- Đường đã xác minh của OP 0x33 **không chứa bất kỳ chuỗi nào**:
  - Không có `_LStrCopy` / `_LStrCatN` / `_LStrFromString` trên payload.
  - Không tham chiếu literal `.rodata` (không có `&DAT_...` / địa chỉ `0x79xxxx` nào trong `case_044`).
  - Không gọi API banner `(VMT+0x90)`.
- `RestPayload` (kể cả byte `0x01` và mọi byte sau) được truyền vào `FUN_0064cae0`. **Xác minh được từ body mới (2026-09-14)**: callee chỉ cắt payload thành **record nhị phân 7 byte** (`_LStrCopy` + `FUN_0077ef7c` DWORD LE, `0064cae0_FUN_0064cae0.c:120-121`) và không gọi bất kỳ helper chuỗi văn bản nào (`_LStrCatN`/`_LStrFromChar`/literal) ⇒ **callee OP 0x33 SubOp 1 không parse chuỗi VISCII**.
- **Kết luận: KHÔNG có chuỗi VISCII ở cả tầng handler lẫn tầng callee OP 0x33** (đã khép được câu hỏi bỏ ngỏ bản cũ).

---

## 6. Chiều Client → Server

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (hàm `TFConnect.SendCommand`) có `switch(param_2 & 0xff) {` tại **dòng 768**, đóng tại **dòng 1079**.

Kết quả grep `case 0x(2e|2f|30|31|32|33|34|35)` trong file:

| Dòng | Nội dung |
| :-- | :-- |
| 986 | `case 0x2e:` (rỗng → `break;` dòng 987) |
| 988 | `case 0x32:` (có code, `break;` dòng 1006) |
| 1007 | `case 0x36:` (có code, `break;` dòng 1011) |
| 1012 | `case 0x37:` (có code) |
| — | **KHÔNG có `case 0x33:`** (cũng không có `0x2f`, `0x30`, `0x31`, `0x34`, `0x35`) |

- Vùng lân cận đã đọc trực tiếp: dòng 978-1006 gồm `case 0x2a/0x2b/0x2c/0x2d/0x2e` (đều `break;` rỗng) rồi tới `case 0x32`; dòng 1007-1019 gồm `0x36/0x37`. **`0x33` vắng mặt hoàn toàn.**
- `switch` **không có `default:`** (grep chỉ thấy duy nhất dòng 768 là `switch`; không có `default:`), và đóng bằng `case 199: }` tại dòng 1078-1079.

**Kết luận**: client **không bao giờ gửi Main OP 0x33**; nếu có giá trị 0x33 lọt vào `SendCommand`, nó rơi khỏi `switch` (không nhánh nào khớp, không `default`) → **no-op**. Vậy **OP 0x33 là kênh S→C một chiều**. Không có format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[33][01]                       2B  → gọi TFightManage.FUN_0064cae0(this, RP) — 0 record, không hiệu ứng
[33][01][grp][slot][flag][dw×4] 8B → 1 record: áp flag/value lên unit tại ô (grp,slot)   (§4.1)
[33][00] / [33][xx], xx != 1   2B  → no-op im lặng
[33]                            1B  → L=1 → BoundErr(0) = RangeError
```

- **Update 2026-09-14**: payload SubOp 1 **đã build được**: `n = (len(RP)-1)/7` record 7 byte `[grp≤3][slot≤4][flag][DWORD value LE]`; ô trống → record bị bỏ qua im lặng; flag `0x19` + `value>0` có nhánh FX đặc biệt (§4.1). **Lưu ý**: `FUN_0064cae0` không guard `battle == 0` ở đầu — nó gọi `FUN_0065178c(DAT_0098c63c, ...)` ngay trong record đầu (`0064cae0_FUN_0064cae0.c:122`), helper này deref tham số (`0065178c_FUN_0065178c.c:68`) ⇒ gửi có record **ngoài trận** = EAccessViolation. Chỉ mock trong trận.
- **Không có field nào được handler tự đọc** ngoài SubOp; layout record suy từ callee (mục §4.1), không còn phải đoán.
- **Không gửi** `[33]` một mình (L=1) vì gây `RangeError`.
- **Hiệu ứng hiển thị cuối cùng** (unit nhận flag/value qua `FUN_00710ab0` nghĩa là gì) cần đọc tiếp thân `FUN_00710ab0` (ĐÃ có body ở `functions/00710ab0_FUN_00710ab0.c` — chưa khảo sát trong tài liệu này) và theo dõi live; trước mắt phát `[33][01]` + record hợp lệ và ghi nhận phản ứng client.
- Số nguyên LE: callee dùng `FUN_0077ef7c` (DWORD LE) cho `value` (khớp codec chuẩn của các OP khác).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_044_0079522C_FUN_0079522c.c` (61 dòng) | Handler chính (đọc SubOp, so sánh `==1`, gọi callee) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6693-6707` | Bản inline dispatcher — xác nhận 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, offset `0x33` = `0x2C`) + `redump/jumptable_0x78A9B6_case_functions.csv:46` + `case_functions/manifest.csv:46` | Mapping MainOp 0x33 → Case 44 → `0x0079522C` |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:8117-8145` | Bản gộp case 44 (call tại dòng 8144) |
| 5 | `functions/0050a4a0_TForm1.FormCreate.c:615-617` | `gvar_007D9CE4` = con trỏ `TFightManage` (class hint) |
| 6 | `case_functions/functions/case_043_007951DA_FUN_007951da.c:27-32`; `case_functions/functions/case_046_007952AB_FUN_007952ab.c:27-69`; `functions/0078a89c_FUN_0078a89c.c:6684-6690,6723-6789` | OP cùng họ `gvar_007D9CE4` (đối chiếu bối cảnh) |
| 7 | `functions/0077f414_FUN_0077F414.c:768,978-1019,1078-1079` | Chiều C→S — xác nhận **không có `case 0x33`** |
| 8 | ~~Glob `*0064cae0*` (0 file) + grep `func_0x0064cae0|0064CAE0` (3 call site)~~ | *(lỗi thời 2026-09-14 — xem dòng 9-11)* |
| 9 | `functions/0064cae0_FUN_0064cae0.c:16,62-67,91-121,122-187` | **Body callee (mới)**: n=(L−1)/7, record `[grp][slot][flag][DWORD]`, nhánh flag 0x19 reset state + FX `FUN_0066e710`, `FUN_00710ab0(unit, flag, value, 0)` |
| 10 | `functions/0064bb24_FUN_0064bb24.c` (190 dòng), `functions/00642244_FUN_00642244.c` (165 dòng) | Body mới hai helper họ `gvar_007D9CE4` dùng ở OP 0x0B (bối cảnh §4.3) |
| 11 | `functions/0065178c_FUN_0065178c.c:50-77`; `functions/00710ab0_FUN_00710ab0.c` (tồn tại, chưa khảo sát) | Tra ô → unitIdx; callee tầng cuối của SubOp 1 |

### Điểm chưa xác minh được từ SSOT (không suy đoán)
1. ~~Thân hàm `func_0x0064cae0`~~ → **ĐÃ XÁC MINH 2026-09-14** (body + phân tích §4.1).
2. **Ý nghĩa nghiệp vụ cụ thể của OP 0x33 / SubOp 1** — wire ĐÃ xác minh; nhưng tác động cuối của từng `flag` đi qua `FUN_00710ab0(unit, flag, value, 0)` (body đã tồn tại trong SSOT, chưa đọc) ⇒ nhãn ngữ nghĩa từng flag: **chưa kết luận được**.
3. ~~Liệu callee có parse chuỗi VISCII~~ → **xác minh: KHÔNG** (record nhị thuần, §5).
4. **Mọi kết luận gắn nhãn "suy luận"** ở §1, §4.3 (quan hệ TFightManage/fight) chỉ dựa trên ngữ cảnh khởi tạo; các body mới (`0064cae0`, `00647164`, `0064dd74` + cả họ 0x35) đều làm việc trên battle-record → củng cố mạnh, nhưng tên class chính thức vẫn chưa có trong export.
