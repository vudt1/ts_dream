# PHÂN TÍCH — Main OP 0x3E (62) / Case 55 / FUN_00795a72 @ 0x00795A72

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh gọi) từ SSOT** (`ts_decompile/` only).
**HAI method ủy quyền (`func_0x0075c22c`, `func_0x0075c1c0`) KHÔNG có body trong SSOT** — nằm trong khe trống `[0x0075C045…0x0075CA6C]` → **layout field / ý nghĩa nghiệp vụ của 2 SubOp là `unknown / cần redump`** (xem §4.3, §4.4).

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3E là opcode **điều khiển được ủy quyền hoàn toàn cho `TLifeManage`** (`gvar_007DA250`, VMT `0x0075C0B4`). Handler chỉ **định tuyến theo `SubOp`** rồi chuyển nguyên con trỏ `RestPayload` cho 2 method của `TLifeManage`.
- **Payload S→C là opaque đối với handler**: handler **không tự parse field nào** ngoài `RP[0]` (SubOp); toàn bộ phần đuôi `RP[1..]` do 2 method `TLifeManage` tự đọc.
- **Chỉ có 2 hành vi hiệu lực** (không dùng `switch`, chỉ `if (==1) / else if (==2)`):
  - `SubOp == 0x01` → `func_0x0075c22c(TLifeManage, RestPayload)` (dòng 28).
  - `SubOp == 0x02` → `func_0x0075c1c0(TLifeManage, RestPayload)` (dòng 31).
- **Không có `default`, không có `switch`**: mọi SubOp khác (`0x00`, `>= 0x03`) là **no-op im lặng** (rơi xuống epilogue dọn frame).
- **Cổng chặn độ dài**: `RestPayload` rỗng (payload `[3E]`, L=1) → `_BoundErr(0)` = **ERangeError**. Mock bắt buộc gửi tối thiểu `L=2` (`[3E][SubOp]`).
- **C→S KHÔNG TỒN TẠI**: `FUN_0077f414` (`TFConnect.SendCommand`) **không có nhãn `case 0x3E`** trong `switch(param_2 & 0xff)` (dòng 768); các case lân cận `0x3C/0x3F/0x40…` là `break;` rỗng (§7).
- **Bối cảnh (suy luận — độ tin cậy THẤP)**: `TLifeManage` ("life manage") trong các OP khác được dùng cho **sự sống / hồi phục đơn vị** (`opcode_35.md §3` SubOp 0x0B), và có field `Word` tại `+4` (`00747800.c:62`). Vì 2 method OP 0x3E **không có body**, **không thể khẳng định** chúng là "hồi sinh"/"cập nhật mạng sống" hay bất kỳ nghiệp vụ cụ thể nào. **Giữ nguyên unknown, không suy đoán wire.**

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (đã kiểm lại từng mắt xích)

```
MainOp 0x3E (62) → byte_table[0x78A8EE][0x3E] = 0x37 (55)
                 → dword_table[0x78A9B6][55] @ 0x0078AA92 = 0x00795A72
                 → FUN_00795a72 (Case 55)
```

- Bảng byte 200: `redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** ứng với index `0x30..0x3F`:
  `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Byte thứ 15 của hàng (offset `0x3E`) = **`0x37` = 55** (kiểm bằng python: `vals[0x3E] = 0x37`; `jumptable_byte200_0x78A8EE.hex:4`).
- Bảng dword: `redump/jumptable_0x78A9B6_case_functions.csv:57` → `55,0x0078AA92,0x00795A72,YES,"FUN_00795a72",0x00795A72,1`.
- Đối chiếu hex thô: `redump/jumptable_dword200_0x78A9B6.hex` **hàng 14**, entry #55 = bytes `72 5A 79 00` → **`0x00795A72`** (LE). (Hàng 14 = entries 52..55, entry #55 là dword thứ 4.)
- Manifest: `case_functions/manifest.csv:57` → `55,0x0078AA92,0x00795A72,EXPORTED,"FUN_00795a72","00795a72",1,"functions/case_055_00795A72_FUN_00795a72.c",`.
- Địa chỉ con trỏ bảng (entry base): `0x78A9B6 + 55*4 = 0x78AA92` — trùng header `case_055_00795A72_FUN_00795a72.c:2` (`Jump table entry: 0x0078AA92`).
- File case chính: `case_functions/functions/case_055_00795A72_FUN_00795a72.c` (64 dòng); lõi logic tại **dòng 20-32**.
- Bản gộp jump table: `case_functions/jumptable_0x78A9B6_cases.c:9003` (header `Case index: 55`), hàm tại dòng **9009**, lõi logic tại **dòng 9021-9033**.
- Bản inline trong dispatcher tổng: `functions/0078a89c_FUN_0078a89c.c:7082-7100` (nhãn `case 0x3e:` đúng của MainOp 0x3E, gọi `gvar_007DA250` + `func_0x0075c22c/1c0`) — **khớp 1:1** (§2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `opcode_00_01.md §2` và `opcode_37.md §2`. Frame: `[Token 2B: F4 44][Length L: Word LE][Payload]`, toàn khung XOR `0xAD`.

> ⚠️ **Bẫy trùng nhãn**: trong `0078a89c_FUN_0078a89c.c` còn một `case 0x3e:` **KHÁC** tại **dòng 4102** — đó là `switch` con của một opcode khác (`gvar_007DA0D0` = `TThingManage`, gọi `func_0x0076af1c`), **KHÔNG liên quan MainOp 0x3E**. Chỉ dòng **7082** mới là MainOp 0x3E.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x3E` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile: `unaff_EBP + -0xc` = con trỏ dữ liệu `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP-4)` = **độ dài** `RestPayload` (length prefix của AnsiString).

### 2.3. Đọc SubOp (`case_055` dòng 20-26)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // iVar2 = RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {             // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                      // → ERangeError (payload [3E], L=1)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
```

- `_BoundErr(0)`: truy cập chỉ mục 0 của chuỗi rỗng → **ERangeError** (Delphi range check). Đây là **chốt chặn độ dài duy nhất do handler tự làm**.
- Sau khi có `SubOp`, handler **không đọc thêm byte nào**; nó chuyển nguyên `RestPayload` xuống method.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** ở handler |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** ở handler |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Không gọi** (handler không cắt field) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 23) |
| `gvar_007DA250` | Con trỏ `TLifeManage` (self-object) — truyền làm tham số 1 (dòng 28, 31) |
| `func_0x0075c22c` | Method `TLifeManage` cho SubOp `0x01` — **body không có trong SSOT** |
| `func_0x0075c1c0` | Method `TLifeManage` cho SubOp `0x02` — **body không có trong SSOT** |
| `_LStrArrayClr/_LStrClr` (dòng 33-60) | **Dọn dẹp frame cục bộ** của dispatcher (giống `case_046`/`case_054`), **không liên quan wire** |
| `LAB_00796408`, `UNK_00795a85/ab/bf` | Nhãn SEH / cleanup frame — **không phải chuỗi text** |

### 2.5. Đối chiếu file case vs bản inline (1:1)

| Vị trí | `case_055_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 20 `iVar2 = *(int *)(unaff_EBP + -0xc)` | dòng 7084 `iVar21 = local_10` |
| Check rỗng | dòng 22 `if (*(int *)(iVar2 + -4) == 0)` | dòng 7086 `if (*(int *)(local_10 + -4) == 0)` |
| SubOp read | dòng 26 `... = *(byte *)(iVar2 + iVar1)` | dòng 7092/7096 `*(char *)(iVar21 + iVar20)` |
| SubOp1 call | dòng 28 `func_0x0075c22c(..., RP)` | dòng 7094 (giống) |
| SubOp2 call | dòng 31 `func_0x0075c1c0(..., RP)` | dòng 7098 (giống) |
| Kết thúc nhánh | `}` dòng 32 | `break;` dòng 7100 |

Khác biệt **duy nhất**: tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX` vs `iVar20/iVar21/extraout_EDX_x00182`) và nhãn SEH. **Logic trùng khớp hoàn toàn.**

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler **không dùng `switch`** mà `if (==1) else if (==2)`; **không có `default`**.

| SubOp (RP[0]) | Payload S→C | Điều kiện đọc | Core logic | Đích |
| :---: | :--- | :--- | :--- | :--- |
| *(rỗng)* | `[3E]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **ERangeError** (dòng 22-25) | — |
| `0x00` | `[3E][00]` | `SubOp = 0` | Không khớp `1`/`2` → **no-op im lặng** | — |
| `0x01` | `[3E][01][...]` | `SubOp = 1` | `func_0x0075c22c(*(gvar_007DA250), RP)` (dòng 28) | `TLifeManage` |
| `0x02` | `[3E][02][...]` | `SubOp = 2` | `func_0x0075c1c0(*(gvar_007DA250), RP)` (dòng 31) | `TLifeManage` |
| `0x03..0xFF` | `[3E][xx][...]` | `SubOp = xx` | Không khớp `1`/`2` → **no-op im lặng** | — |

**Tổng: 2 nhánh có hiệu lực** (`0x01`, `0x02`) + **1 dạng gây ERangeError** (`[3E]` L=1). Mọi trường hợp còn lại là no-op. Byte dư sau `RP[0]` **được chuyển nguyên cho method đích** (không bị bỏ qua ở handler).

---

## 4. Chi tiết các nhánh

### 4.1. `SubOp == 0x01` → `func_0x0075c22c(gvar_007DA250, RP)`

```c
if (*(int *)(unaff_EBP + -0x14) == 1) {                                    // case_055 dòng 27
  func_0x0075c22c(*(undefined4 *)gvar_007DA250,*(undefined4 *)(unaff_EBP + -0xc));  // dòng 28
}
```

- **Tham số 1** = `*(gvar_007DA250)` = **đối tượng `TLifeManage`** (self).
- **Tham số 2** = `*(undefined4 *)(unaff_EBP + -0xc)` = **con trỏ `RestPayload`** (AnsiString; `RP[0]` vẫn là SubOp, `RP[1..]` là tham số tuỳ method).
- **Body method KHÔNG có trong SSOT** → không biết wire/layout/ý nghĩa (§4.3).

### 4.2. `SubOp == 0x02` → `func_0x0075c1c0(gvar_007DA250, RP)`

```c
else if (*(int *)(unaff_EBP + -0x14) == 2) {                               // case_055 dòng 30
  func_0x0075c1c0(*(undefined4 *)gvar_007DA250,*(undefined4 *)(unaff_EBP + -0xc));  // dòng 31
}
```

- Cấu trúc gọi **giống hệt** SubOp 1, chỉ khác method đích. **Body KHÔNG có trong SSOT.**

### 4.3. Định danh `gvar_007DA250` = `TLifeManage` (đã xác minh)

- `functions/0050a4a0_TForm1.FormCreate.c:631-632`:
  ```c
  piVar6 = TLifeManage_Create((int *)VMT_75C0B4_TLifeManage,'\x01',extraout_ECX_36);
  *(int **)gvar_007DA250 = piVar6;
  ```
- ASM xác nhận cùng vị trí: `functions/0050a4a0_TForm1.FormCreate.asm.txt:824-825` — `MOV EAX,[0x0075c0b4]` → `CALL 0x0075c10c` (constructor `TLifeManage.Create @ 0075c10c`, khai báo tại `0050a4a0_TForm1.FormCreate.c:134`).
- **Đính chính** (kế thừa `opcode_35.md`): `gvar_007DA250` **không phải** `TVenderManage` (dù `TVenderManage.Create` nằm kề `0x0075C004`). `TVenderManage` thật là `gvar_007D9C74` (`0050a4a0:622-623`).

### 4.4. Vì sao 2 method là `unknown` — bằng chứng "khe trống" (HOLE)

- `func_0x0075c22c` / `func_0x0075c1c0` **không có file `.c` lẫn `.asm.txt`** trong SSOT (grep toàn repo: chỉ có 6 match, **tất cả là call-site** — 2 trong `case_055`, 2 trong `jumptable_0x78A9B6_cases.c:9029,9032`, 2 trong `0078a89c.c:7094,7098`).
- `index.csv` **không có** entry `0075c10c` (TLifeManage.Create) lẫn `0075c1c0`/`0075c22c`/`0075c184`/`0075c518`.
- Khe trống theo `index.csv`: entry liền trước vùng là `TVenderManage.Create` @ `0075c004` (size 65 → kết thúc ~`0x75c045`, `index.csv:5585`); entry liền sau là `FUN_0075ca6c` @ `0075ca6c` (`index.csv:5586`). **Toàn bộ `[0x0075C045…0x0075CA6C]` — chứa `TLifeManage.Create` (`0x75C10C`) + 4 method `0x75C1xx..0x75C5xx` — không được export.**
- Cùng khe trống này chứa các method `TLifeManage` đã biết qua call-site:
  | Method | Call-site trong SSOT | Vai trò đã quan sát |
  | :-- | :-- | :-- |
  | `func_0x0075c184` | `case_004_0078BC95_FUN_0078bc95.c:103-106` | Nhận 1 `ushort` itemID → trả `byte`, ghi vào `gvar_007DA7BC + 0x1458` |
  | `func_0x0075c1c0` | `case_055_...c:31` | **OP 0x3E SubOp 2 (chưa rõ)** |
  | `func_0x0075c22c` | `case_055_...c:28` | **OP 0x3E SubOp 1 (chưa rõ)** |
  | `func_0x0075c518` | `case_046_007952AB_FUN_007952ab.c:56` | OP 0x35 SubOp `0x0B` (chưa rõ) |
- ⇒ **Không thể dựng layout field cho `RP[1..]` của OP 0x3E từ SSOT. Bắt buộc redump vùng `0x0075C045…0x0075CA6C` (ưu tiên `0x75C1C0`, `0x75C22C`).**

### 4.5. Field struct / layout `RP` (mức xác minh được)

| Offset | Kích thước | Tên | Trạng thái |
| :---: | :---: | :--- | :--- |
| `RP[0]` = `P[1]` | 1 B | `SubOp` | **XÁC MINH** (handler, dòng 26) |
| `RP[1..]` = `P[2..]` | ? | *tham số method* | **UNKNOWN** — do `func_0x0075c22c`/`func_0x0075c1c0` parse nội bộ; body không có trong SSOT |

- 2 method nhận **nguyên `RestPayload`** (kể cả byte `RP[0]`), nên nếu method đọc 0-based thì **field đầu tiên nằm tại `RP[1]`**; nếu method tự re-read SubOp thì vị trí có thể khác. **Không khẳng định.**

### 4.6. Ghi chú ngữ cảnh `TLifeManage` (SUY LUẬN — độ tin cậy THẤP)

- `TLifeManage` có một field `Word` tại `+4`: `functions/00747800_FUN_00747800.c:62` — `uVar3 = (uint)*(ushort *)(*(uint *)gvar_007DA250 + 4);` (giá trị này được truyền tiếp cho `FUN_0079c150`).
- Trong `opcode_35.md §3/§4.2`, `TLifeManage` (SubOp `0x0B` của OP 0x35) được xếp nhóm **"nghi liên quan sự sống/hồi phục đơn vị"**, cũng **opaque**.
- **Kết luận**: tên class gợi ý "quản lý mạng sống/trạng thái sống", nhưng **KHÔNG có bằng chứng SSOT** cho nội dung 2 SubOp của OP 0x3E → **không gán nhãn nghiệp vụ** (tránh bịa). Chờ redump.

---

## 5. Wire format

Khung ngoài: `[Token 2B: F4 44][Length L: Word LE 2B][Payload L bytes]`, **toàn khung XOR `0xAD`**. `L` tính từ byte MainOp `0x3E`.

```
Payload S→C:
  [3E]                    L=1  → ERangeError (BoundErr(0))     — ĐỪNG GỬI
  [3E][00]                L=2  → no-op im lặng
  [3E][01]                L=2  → TLifeManage.func_0x0075c22c (không tham số)
  [3E][01][...args...]    L=2+n→ TLifeManage.func_0x0075c22c (args chưa biết)
  [3E][02]                L=2  → TLifeManage.func_0x0075c1c0 (không tham số)
  [3E][02][...args...]    L=2+n→ TLifeManage.func_0x0075c1c0 (args chưa biết)
  [3E][xx>=03]            L=2  → no-op im lặng
```

**Ví dụ khung hoàn chỉnh** (đã XOR `0xAD`):

```
# [3E][01]  → func_0x0075c22c (không tham số)
payload  = 3E 01
L (LE)   = 02 00
pre-XOR  = F4 44 02 00 3E 01
on-wire  = 59 E9 AF AD 93 AC

# [3E][02]  → func_0x0075c1c0 (không tham số)
payload  = 3E 02
L (LE)   = 02 00
pre-XOR  = F4 44 02 00 3E 02
on-wire  = 59 E9 AF AD 93 AF

# [3E]     → ERangeError (KHÔNG gửi)
payload  = 3E
L (LE)   = 01 00
pre-XOR  = F4 44 01 00 3E
on-wire  = 59 E9 AC AD 93
```

- **Bắt buộc `L >= 2`**; `[3E]` (L=1) làm handler ném `ERangeError`.
- Phần `[...args...]` (nếu có) **chưa đặc tả được** (§4.4-4.5). Mock an toàn nhất chỉ nên gửi `[3E][01]` / `[3E][02]` và quan sát.

---

## 6. Edge cases & ERangeError bounds

| Tình huống | Wire | Hành vi | Bằng chứng |
| :-- | :-- | :-- | :-- |
| `RestPayload` rỗng | `[3E]` (L=1) | `_BoundErr(0)` → **ERangeError** | `case_055_...c:22-25` |
| `SubOp = 0x00` | `[3E][00]` | no-op im lặng | `case_055_...c:27,30` (không khớp) |
| `SubOp >= 0x03` | `[3E][xx]` | no-op im lặng | nt. |
| Byte dư sau SubOp | `[3E][01][dư…]` | **được chuyển nguyên cho method** (không bị bỏ ở handler) | `case_055_...c:28` |
| Bounds bên trong method | ? | **UNKNOWN** — method tự bound; không có body | §4.4 |

- `extraout_EDX` trên nhánh `_BoundErr` là **đường phục hồi giả của decompiler** (nhánh exception), không phải logic nghiệp vụ.
- Epilogue (`case_055_...c:33-60`) chỉ là `_LStrArrayClr`/`_LStrClr` dọn stack — **dùng chung cho mọi case**, không phải logic.

---

## 7. Chiều Client → Server (C→S)

### 7.1. Kết luận: KHÔNG có C→S

- `functions/0077f414_FUN_0077F414.c:768` — `switch(param_2 & 0xff)` không chứa nhãn `case 0x3E`. Danh sách case lân cận (absolute line):
  - `case 0x3c:` → `break;` (`:1042-1043`)
  - `case 0x3d:` → `FUN_00402b90/_PStrNCat/...` (`:1044-1059`)
  - `case 0x3f:` → `break;` (`:1060-1061`)
  - `case 0x40:` … `case 0x48:` → `break;` (`:1062-1077`)
  - `case 199:` → hết `switch` (`:1078`)
- ⇒ `0x3E` **không phải một opcode gửi đi**; nếu `SendCommand` bị gọi với `0x3E` thì không khớp case nào (rơi về `default`/không đẩy gì vào queue).

### 7.2. Kiểm tra chéo call-site

- Quét **toàn bộ file `.asm.txt`** trong SSOT cho mẫu `MOV DL,0x3E` (nạp MainOp trước `CALL 0x0077f414`): **0 hit**.
- Đối chiếu prologue builder `0077f414.asm.txt`: `MainOp = DL` (`[EBP-0x5]`), `SubSel = CL` (`[EBP-0x6]`) ⇒ muốn gửi 0x3E phải có `MOV DL,0x3E` tại call-site; **không tồn tại**.
- ⇒ **OP 0x3E là kênh server-push một chiều (S→C)**. Server **không cần** parse gói C→S 0x3E.

> Lưu ý phương pháp: việc **thiếu nhãn `case`** trong `switch` của decompiler là bằng chứng chính; nếu một case tồn tại nhưng rỗng, IDA thường render `case 0x3e: case 0x3f: break;`. Ở đây chỉ có `case 0x3f`, nên 0x3E thực sự vắng.

---

## 8. Chuỗi VISCII → UTF-8

- **Nhánh OP 0x3E không tham chiếu bất kỳ literal chuỗi nào.** Trong `case_055_...c` chỉ có: `gvar_007DA250`, 2 `func_0x...`, và các nhãn `LAB_00796408` / `UNK_00795a85` / `UNK_00795aab` / `UNK_00795abf`.
- Các `UNK_00795a**` là **nhãn SEH/frame-cleanup code address**, **không phải text** (không có tiền tố `s_`/`DAT_`).
- ⇒ **Không có gì để decode VISCII ở OP này.**
- **Cảnh báo redump**: nếu `func_0x0075c22c` / `func_0x0075c1c0` (chưa có body) chứa toast/banner text, **hiện chưa dịch được** vì không có bytes. **Mark `unknown / cần redump`.**

---

## 9. Ghi chú cho Mock Server

1. **S→C tối thiểu**: gửi `[3E][01]` (L=2) hoặc `[3E][02]` (L=2) để kích 2 nhánh; phần args chưa biết nên **không gửi kèm**.
2. **Tuyệt đối không gửi** `[3E]` (L=1) — gây `ERangeError` trong handler.
3. `SubOp 0x00` và `>= 0x03` **bị bỏ qua im lặng** — an toàn nhưng vô dụng.
4. **Không có C→S 0x3E** — server không cần nhận/parse.
5. **Chưa thể mock đầy đủ**: layout args + hiệu ứng của 2 method `TLifeManage` **chưa biết**. Bắt buộc **redump vùng `0x0075C045…0x0075CA6C`** (đặc biệt `0x75C1C0`, `0x75C22C`), đối chiếu thêm `0x75C184`/`0x75C518` cùng class để suy bố cục method.

---

## 10. Source trail

| # | Nguồn (trong `ts_decompile/`) | Dùng để |
| :-- | :-- | :-- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex:4` (index `0x3E` = `0x37`=55) | Byte table MainOp 0x3E → 55 |
| 2 | `redump/jumptable_0x78A9B6_case_functions.csv:57`; `redump/jumptable_dword200_0x78A9B6.hex:14` (entry #55 = `72 5A 79 00`); `case_functions/manifest.csv:57` | Dword table entry #55 @ `0x0078AA92` → `0x00795A72` |
| 3 | `case_functions/functions/case_055_00795A72_FUN_00795a72.c:20-32` (handler, 64 dòng) | Lõi logic S→C |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:9003-9033` | Bản gộp case 55 |
| 5 | `functions/0078a89c_FUN_0078a89c.c:7082-7100` | Bản inline dispatcher — xác nhận 1:1 |
| 6 | `functions/0078a89c_FUN_0078a89c.c:4102-4105` | Cảnh báo `case 0x3e:` **khác** (TThingManage, không liên quan) |
| 7 | `functions/0050a4a0_TForm1.FormCreate.c:134,631-632`; `.asm.txt:824-825` | `gvar_007DA250` = `TLifeManage` (VMT `0x75C0B4`, create `0x75c10c`) |
| 8 | `functions/0050a4a0_TForm1.FormCreate.c:622-623` | Đính chính: `TVenderManage` = `gvar_007D9C74` |
| 9 | `functions/00747800_FUN_00747800.c:62,69` | `TLifeManage` có field `Word` tại `+4` (ngữ cảnh) |
| 10 | `case_functions/functions/case_004_0078BC95_FUN_0078bc95.c:103-106` | `func_0x0075c184` (cùng class) — trả `byte` theo itemID |
| 11 | `case_functions/functions/case_046_007952AB_FUN_007952ab.c:56` | `func_0x0075c518` (cùng class) — OP 0x35 SubOp 0x0B |
| 12 | `index.csv:5585` (`0075c004`, size 65) & `index.csv:5586` (`0075ca6c`) | Chứng minh khe trống `[0x75C045…0x75CA6C]` chứa 2 method |
| 13 | Grep `func_0x0075c22c\|func_0x0075c1c0` (6 match, toàn call-site, 0 body) | Xác nhận 2 method **không có trong SSOT** |
| 14 | `functions/0077f414_FUN_0077F414.c:768,1042-1078` | C→S: `switch(param_2&0xff)` **không có** `case 0x3E` |
| 15 | Grep `MOV DL,0x3E` trong toàn bộ `.asm.txt` → 0 hit | C→S: không có call-site gửi 0x3E |
| 16 | `opcode_35.md §3/§4.2` (tham chiếu nội bộ) | Ngữ cảnh `TLifeManage` (suy luận LOW) |

### Điểm chưa xác minh được từ SSOT (không suy đoán)

1. **Wire/field layout `RP[1..]` của cả 2 SubOp** — do `func_0x0075c22c` (`0x0075C22C`) và `func_0x0075c1c0` (`0x0075C1C0`) **không có body** trong SSOT → **unknown**.
2. **Ý nghĩa nghiệp vụ chính xác** của `SubOp 0x01` và `SubOp 0x02` — tên class `TLifeManage` chỉ là gợi ý, **không đủ để kết luận** → **unknown**.
3. **Bố cục object `TLifeManage`** (field `+4` Word đã thấy; các field khác) và **slot VMT** của 2 method — **không có dump VMT `0x75C0B4`** → **unknown**.
4. **Bounds nội bộ** (ERangeError do method tự kiểm) — **unknown** (không có body).
5. **Chuỗi text (nếu có)** bên trong 2 method — **không có dump** → **unknown / cần redump**.
6. **Bản chất 2 method** (có thể là method ảo VMT hay hàm thường của class) — call-site chỉ cho thấy truyền `self`; **không xác minh được slot** → để mở.
