# PHÂN TÍCH — Main OP 0x3E (62) / Case 55 / FUN_00795a72 @ 0x00795A72

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**  
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh gọi) từ SSOT** (`ts_decompile/` only).
**HAI method ủy quyền (`FUN_0075c22c`, `FUN_0075c1c0`) ĐÃ CÓ BODY** trong đợt redump 2026-09-14 (HOLE `0x0075C045…0x0075CA6C` đã giải một phần lớn) → layout field / ý nghĩa 2 SubOp **đã phân tích được** (§4.1, §4.2). Phần vẫn trống: constructor `TLifeManage.Create @ 0x75C10C` (khe `0x75C045–0x75C183`) và bytes nội dung 6+3 hằng AnsiString `DAT_0075c338…DAT_0075c6c8` (chưa dump).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3E là opcode **điều khiển được ủy quyền hoàn toàn cho `TLifeManage`** (`gvar_007DA250`, VMT `0x0075C0B4`). Handler chỉ **định tuyến theo `SubOp`** rồi chuyển nguyên con trỏ `RestPayload` cho 2 method của `TLifeManage`.
- **Payload S→C là opaque đối với handler**: handler **không tự parse field nào** ngoài `RP[0]` (SubOp); toàn bộ phần đuôi `RP[1..]` do 2 method `TLifeManage` tự đọc.
- **Chỉ có 2 hành vi hiệu lực** (không dùng `switch`, chỉ `if (==1) / else if (==2)`):
  - `SubOp == 0x01` → `func_0x0075c22c(TLifeManage, RestPayload)` (dòng 28) — **hiển thị 1 trong 6 thông báo cố định** (2 dòng chat + 4 toast) chọn theo `RP[1]` (xem §4.1).
  - `SubOp == 0x02` → `func_0x0075c1c0(TLifeManage, RestPayload)` (dòng 31) — **ghi Word `RP[1..2]` (LE) vào field `self+4`** của `TLifeManage` (xem §4.2).
- **Không có `default`, không có `switch`**: mọi SubOp khác (`0x00`, `>= 0x03`) là **no-op im lặng** (rơi xuống epilogue dọn frame).
- **Cổng chặn độ dài**: `RestPayload` rỗng (payload `[3E]`, L=1) → `_BoundErr(0)` = **ERangeError**. Mock bắt buộc gửi tối thiểu `L=2` (`[3E][SubOp]`). **Bổ sung từ body mới**: cả 2 method đều tự chốt chặn — SubOp 1 ném `ERangeError` nếu `len(RP) < 2` (`0075c22c_FUN_0075c22c.c:23-24`); SubOp 2 ném qua codec `FUN_0077eb9c` khi chuỗi cắt ra `< 2` ký tự (`0077eb9c_FUN_0077eb9c.c:169-170`) ⇒ thực tế cần `L >= 4`.
- **C→S KHÔNG TỒN TẠI**: `FUN_0077f414` (`TFConnect.SendCommand`) **không có nhãn `case 0x3E`** trong `switch(param_2 & 0xff)` (dòng 768); các case lân cận `0x3C/0x3F/0x40…` là `break;` rỗng (§7).
- **Bối cảnh (NÂNG lên TB sau khi có body)**: `TLifeManage` (self `gvar_007DA250`) hiện chỉ được OP 0x3E dùng để: (a) lưu 1 Word tại `+4` (cập nhật từ server, SubOp 2), và (b) phát thông báo hiển thị cố định (SubOp 1, không đụng field nào của self). Field `+4` **xác minh được từ body mới**: `0075c1c0` ghi `*(word*)(self+4)` (dòng 37), khớp nơi đọc `00747800_FUN_00747800.c:62`. **Không kết luận được ý nghĩa nghiệp vụ của con Word đó** (tên class gợi ý "mạng sống/sự sống" — vẫn chỉ là gợi ý).

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
| `func_0x0075c22c` | Method `TLifeManage` cho SubOp `0x01` — **đã có body** `ts_decompile/functions/0075c22c_FUN_0075c22c.c` (§4.1) |
| `func_0x0075c1c0` | Method `TLifeManage` cho SubOp `0x02` — **đã có body** `ts_decompile/functions/0075c1c0_FUN_0075c1c0.c` (§4.2) |
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
| `0x01` | `[3E][01][m]` | `SubOp = 1` | `func_0x0075c22c` (dòng 28): `switch(RP[1])` phát 1/6 thông báo hiển thị định trước (§4.1) | `TLifeManage` |
| `0x02` | `[3E][02][wL][wH]` | `SubOp = 2` | `func_0x0075c1c0` (dòng 31): `self+4 := Word LE(RP[1..2])` (§4.2) | `TLifeManage` |
| `0x03..0xFF` | `[3E][xx][...]` | `SubOp = xx` | Không khớp `1`/`2` → **no-op im lặng** | — |

**Tổng: 2 nhánh có hiệu lực** (`0x01`, `0x02`) + **1 dạng gây ERangeError ở handler** (`[3E]` L=1) — cộng thêm 2 dạng ERangeError **bên trong method** khi payload cụt (§6). Byte dư sau field **được chuyển nguyên cho method nhưng method bỏ qua** (không đọc quá `RP[1..2]`).

---

## 4. Chi tiết các nhánh

### 4.1. `SubOp == 0x01` → `func_0x0075c22c(gvar_007DA250, RP)` — ĐÃ PHÂN TÍCH ĐƯỢC

```c
if (*(int *)(unaff_EBP + -0x14) == 1) {                                    // case_055 dòng 27
  func_0x0075c22c(*(undefined4 *)gvar_007DA250,*(undefined4 *)(unaff_EBP + -0xc));  // dòng 28
}
```

- **Tham số 1** = `*(gvar_007DA250)` = **đối tượng `TLifeManage`** (self) — **thân method KHÔNG đụng self**: không đọc/ghi field `+0xNN` nào; đây là method liên kết tĩnh (call-site gọi trực tiếp, không qua VMT).
- **Tham số 2** = con trỏ `RestPayload` (AnsiString).
- **Body mới** (`ts_decompile/functions/0075c22c_FUN_0075c22c.c:22-46`, 230 byte):
  1. Chốt chặn: `if (*(uint*)(RP + -4) < 2) _BoundErr(1)` → `len(RP) < 2` (tức payload `[3E][01]` cụt, L=2) ném **ERangeError** (dòng 23-24).
  2. `switch (RP[1])` — byte ngay sau SubOp làm **chọn lệnh con** (`RP[2..]` bị bỏ qua hoàn toàn):

  | `RP[1]` | Hành vi | Dẫn chứng |
  | :---: | :-- | :-- |
  | 1 | `FUN_007ab870(*(int*)gvar_007DA1B0, 0, &DAT_0075c338, 0)` — ghi dòng chat/hệ thống cố định vào `TTalkMsgForm` (`gvar_007DA1B0`, tag 0 — họ hội tụ OP 0x02, `opcode_02.md:15`) | `:29` |
  | 2 | nt. với `&DAT_0075c3c0` | `:32` |
  | 3 | `(**gvar_007DA084 + 0x90)(gvar_007DA084↑, &DAT_0075c438, 2000, 0, 0)` — **toast 2000ms** (virtual ShowMessage, pattern `opcode_09.md:110`) | `:35` |
  | 4 | nt. với `&DAT_0075c468` | `:38` |
  | 5 | nt. với `&DAT_0075c4c4` | `:41` |
  | 6 | nt. với `&DAT_0075c4ec` | `:44` |
  | khác | không case → return | `:27-45` |
- **Nội dung 6 chuỗi** `DAT_0075c338/3c0/438/468/4c4/4ec` **chưa có bytes** (không có `lit_75c*.hex` trong `redump/`) → text cụ thể **unknown / cần redump**; chỉ kết luận được dạng hiển thị (2 dòng chat + 4 toast 2s).
- **Kết luận nghiệp vụ (grounded)**: SubOp 1 = "server bảo client phát 1 thông báo định trước số thứ tự `RP[1]` (1..6)". **Chưa kết luận được** chủ đề từng thông báo (chưa có chuỗi).

### 4.2. `SubOp == 0x02` → `func_0x0075c1c0(gvar_007DA250, RP)` — ĐÃ PHÂN TÍCH ĐƯỢC

```c
else if (*(int *)(unaff_EBP + -0x14) == 2) {                               // case_055 dòng 30
  func_0x0075c1c0(*(undefined4 *)gvar_007DA250,*(undefined4 *)(unaff_EBP + -0xc));  // dòng 31
}
```

- **Body mới** (`ts_decompile/functions/0075c1c0_FUN_0075c1c0.c:28-42`, 97 byte):
  1. `local_10 = Copy(RP, 2, 2)` (1-based) = cắt **2 byte `RP[1..2]`** (ngay sau SubOp) — `_LStrCopy(param_2,2,2,...)` (dòng 35).
  2. `uVar1 = FUN_0077eb9c(*(gvar_007D9D30), local_10)` (dòng 36) = codec **2B→Word LE** đã biết; tham số 1 là artifact EAX của Delphi managed-func (`opcode_05.md:60`; thân `0077eb9c_FUN_0077eb9c.c:160-179` chỉ dùng param_2 và chính nó ném `_BoundErr` nếu chuỗi `< 2` ký tự ⇒ payload phải đủ `RP[1..2]`, tức `L >= 4`).
  3. `*(word*)(self + 4) = uVar1` (dòng 37) — **ghi Word vào field `+4` của `TLifeManage`**.
- ⇒ **Field `+4` Word của `TLifeManage` xác minh được từ body mới**: chính SubOp 2 của OP 0x3E là nơi **ghi** giá trị (trước đây chỉ thấy nơi đọc `00747800_FUN_00747800.c:62`). Giá trị Word truyền từ wire được lưu nguyên, không biến đổi, không kiểm trần (chỉ bound ERangeError qua codec).
- **Ý nghĩa con Word**: chưa kết luận được (không có bằng chứng trong body).

### 4.3. Định danh `gvar_007DA250` = `TLifeManage` (đã xác minh)

- `functions/0050a4a0_TForm1.FormCreate.c:631-632`:
  ```c
  piVar6 = TLifeManage_Create((int *)VMT_75C0B4_TLifeManage,'\x01',extraout_ECX_36);
  *(int **)gvar_007DA250 = piVar6;
  ```
- ASM xác nhận cùng vị trí: `functions/0050a4a0_TForm1.FormCreate.asm.txt:824-825` — `MOV EAX,[0x0075c0b4]` → `CALL 0x0075c10c` (constructor `TLifeManage.Create @ 0075c10c`, khai báo tại `0050a4a0_TForm1.FormCreate.c:134`).
- **Đính chính** (kế thừa `opcode_35.md`): `gvar_007DA250` **không phải** `TVenderManage` (dù `TVenderManage.Create` nằm kề `0x0075C004`). `TVenderManage` thật là `gvar_007D9C74` (`0050a4a0:622-623`).

### 4.4. Tình trạng HOLE `0x0075C045…0x0075CA6C` — ĐÃ GIẢI PHẦN LỚN (đính chính 2026-09-14)

- **(đính chính)** Toàn bộ nhận xét "2 method không có file `.c`/`.asm.txt`, index.csv không có entry" của bản cũ **đã lỗi thời**: 4/5 hàm trong vùng đã được export:
  | Địa chỉ | index.csv | File |
  | :-- | :-- | :-- |
  | `FUN_0075c184` (58B) | `index.csv:6506` | `functions/0075c184_FUN_0075c184.c` |
  | `FUN_0075c1c0` (97B) | `index.csv:6507` | `functions/0075c1c0_FUN_0075c1c0.c` |
  | `FUN_0075c22c` (230B) | `index.csv:6508` | `functions/0075c22c_FUN_0075c22c.c` |
  | `FUN_0075c518` (380B) | `index.csv:6509` | `functions/0075c518_FUN_0075c518.c` |
- **Vẫn trống**: `TLifeManage.Create @ 0x75C10C` — `index.csv` vẫn không có entry (đếm grep `0075c10c` = 0; chỉ xuất hiện comment header `0050a4a0_TForm1.FormCreate.c:134`). Khe chưa export còn lại: **`0x75C045–0x75C183`** (giữa `TVenderManage.Create` end `0x75C045` — `index.csv:5585` — và `FUN_0075c184` — `index.csv:6506`).
- **Cập nhật bảng method cùng class** (thay bảng "chưa rõ" cũ):
  | Method | Call-site | Vai trò — ĐÃ rõ từ body |
  | :-- | :-- | :-- |
  | `FUN_0075c184` | `case_004_...c:103-106` + 1 call-site mới `sub_00799862` (header `0075c184_FUN_0075c184.c:9`) | **Phân loại itemID thuần hàm số** (param_2, self không dùng): ID ∈ `0xD50D..0xD515` → trả 2; ID ∈ `0xD517..0xD51A` ∪ `0xD51C..0xD51F` → trả 1; còn lại → 0 (`0075c184_FUN_0075c184.c:24-35`). **Đính chính** mô tả cũ "ghi vào `gvar_007DA7BC+0x1458`": việc ghi là của caller; thân hàm không ghi gì. |
  | `FUN_0075c1c0` | `case_055_...c:31` | OP 0x3E SubOp 2 — **ghi Word `RP[1..2]` vào `self+4`** (§4.2). |
  | `FUN_0075c22c` | `case_055_...c:28` | OP 0x3E SubOp 1 — **6 thông báo hiển thị cố định** (§4.1). |
  | `FUN_0075c518` | `case_046_...c:56` | OP 0x35 SubOp 0x0B — **vòng đọc bản ghi TLV**: tối đa 21 records `[tag:1][len:1][text:len]` bắt đầu từ `RP[1]`, mỗi record nối tiền tố `DAT_0075c6a8` (tag 1) / `DAT_0075c6b8` (tag 2) / `DAT_0075c6c8` (tag 3) vào text rồi đưa vào `TTalkMsgForm` qua `FUN_007ab870(form,0,msg,0)`; tag khác = bỏ; read ngoài chuỗi → `_BoundErr` ERangeError (`0075c518_FUN_0075c518.c:50-113`). **Đính chính** trạng thái "chưa rõ" cũ của `opcode_35.md` cho hàm này. |
- 9 hằng AnsiString mới lộ địa chỉ (`DAT_0075c338/3c0/438/468/4c4/4ec` + `DAT_0075c6a8/b8/c8`) **chưa có dump bytes** → cần `lit_75c3xx.hex` (xem §11).

### 4.4b. Dump VMT `0x75C0B4` (`redump/vmt_75C0B4_TLifeManage.hex`) — đã có, giải theo index.csv

- File chứa **64 byte = 16 dword LE** (kích thước dump thực tế nhỏ hơn 195 byte vì 195 là số *ký tự* của file hex).
- Kết quả đối chiếu từng slot khác 0 với `index.csv` (chỉ trỏ LE, không suy diễn ngữ nghĩa slot):
  | Slot @địa chỉ | Giá trị | Resolve |
  | :-- | :-- | :-- |
  | `+0x00` @`0x75C0B4` | `0x0075C100` | **không có entry trong index.csv** — nằm trong khe chưa export `0x75C045–0x75C183`, cách `TLifeManage.Create @ 0x75C10C` đúng 12 byte (chưa kết luận được bản chất) |
  | `+0x04..+0x1C` @`0x75C0B8–0x75C0D0` | `0x00000000` ×7 | rỗng |
  | `+0x20` @`0x75C0D4` | `0x0075C100` | nt. (trùng giá trị slot 0) |
  | `+0x24` @`0x75C0D8` | `0x00000008` | không phải con trỏ hợp lệ (số nguyên nhỏ) — ghi nhận thô |
  | `+0x28` @`0x75C0DC` | `0x00401100` | không có entry (dưới entry đầu tiên `0x00402864` của index.csv — vùng RTL Borland, chưa export) |
  | `+0x2C..+0x3C` @`0x75C0E0–0x75C0F0` | `0x00403298`, `0x004032A4`, `0x004032A8`, `0x004032AC`, `0x004032A0` | không có entry (nằm khe giữa `FUN_00403284` end `0x00403295` và `FUN_00403440` — RTL stub, chưa export) |
- **Hệ quả cho câu hỏi "2 method OP 0x3E có phải slot ảo không"**: trong cửa sổ 16 dword được dump **không có slot nào trỏ tới `0x75C1C0`/`0x75C22C`**; khớp với việc call-site `case_055` gọi **trực tiếp** 2 địa chỉ này (không qua `[EAX+ofs]`). ⇒ **liên kết tĩnh, không phải method ảo của cửa sổ VMT này** (chưa loại trừ khả năng VMT còn dài hơn 64B chưa dump).
- **Cửa sổ dump kết thúc tại `0x75C0F4`**, không phủ tới `0x75C10C` (constructor) → **bố cục đầy đủ của VMT vẫn chưa đủ dữ liệu**; cần redump dài hơn nếu muốn đối chiếu constructor/destructor.

### 4.5. Field struct / layout `RP` (NÂNG mức xác minh từ body mới)

| Offset | Kích thước | Tên | Trạng thái |
| :---: | :---: | :--- | :--- |
| `RP[0]` = `P[1]` | 1 B | `SubOp` | **XÁC MINH** (handler, dòng 26) |
| `RP[1]` | 1 B | SubOp 1: **mã thông báo 1..6** (`switch` — `0075c22c...c:27-45`) | **XÁC MINH từ body mới** |
| `RP[1..2]` | 2 B | SubOp 2: **Word LE** ghi vào `TLifeManage+4` (`0075c1c0...c:35-37`) | **XÁC MINH từ body mới** |
| `RP[2..]` | ? | SubOp 1: **bỏ qua** (method không đọc thêm) | **XÁC MINH** |
| `RP[3..]` | ? | SubOp 2: **bỏ qua** (method chỉ cắt đúng `RP[1..2]`) | **XÁC MINH** |

- 2 method nhận **nguyên `RestPayload`** (kể cả byte `RP[0]`) nhưng **không re-read SubOp** — SubOp 1 đọc thẳng `RP[1]`, SubOp 2 `Copy(RP,2,2)` = `RP[1..2]` (1-based index 2). Field đầu tiên luôn tại **`RP[1]`**.

### 4.6. Ghi chú ngữ cảnh `TLifeManage` (cập nhật)

- `TLifeManage` có một field `Word` tại `+4`: **xác minh được từ body mới** — `0075c1c0_FUN_0075c1c0.c:37` ghi `*(word*)(self+4)` từ wire; `00747800_FUN_00747800.c:62` đọc cùng offset (truyền tiếp cho `FUN_0079c150`). Instance size hàm chứa trong VMT dump có ô trị số `8` (`+0x24`) — **không kết luận được** đó là InstanceSize hay gì khác.
- Trong `opcode_35.md §3/§4.2`, method `FUN_0075c518` (OP 0x35 SubOp `0x0B`) **đã rõ**: chỉ hiển thị danh sách message vào chat form, không đụng field nào của `TLifeManage` (§4.4).
- **Kết luận**: sau redump, cả 2 SubOp của OP 0x3E **không có thao tác "hồi sinh/HP" nào nhìn thấy được** — chỉ (a) phát thông báo, (b) cập nhật 1 Word `+4`. Nhãn nghiệp vụ "life/revive" **không được body ủng hộ**; giữ trung lập cho tới khi biết ý nghĩa con Word.

---

## 5. Wire format

Khung ngoài: `[Token 2B: F4 44][Length L: Word LE 2B][Payload L bytes]`, **toàn khung XOR `0xAD`**. `L` tính từ byte MainOp `0x3E`.

```
Payload S→C:
  [3E]                    L=1  → ERangeError (BoundErr(0))     — ĐỪNG GỬI
  [3E][00]                L=2  → no-op im lặng
  [3E][01]                L=2  → func_0x0075c22c → len(RP)<2 → ERangeError (đính chính: bản cũ ghi "không tham số, an toàn")
  [3E][01][m]             L=3  → func_0x0075c22c: phát thông báo #m (m=1..6: 1-2 → dòng chat, 3-6 → toast 2000ms; m khác → không hiển thị)
  [3E][01][m][dư…]        L>3  → nt. — byte dư bị BỎ QUA
  [3E][02]                L=2  → func_0x0075c1c0 → Copy ra chuỗi cụt → ERangeError trong FUN_0077eb9c (đính chính bản cũ)
  [3E][02][wL][wH]        L=4  → func_0x0075c1c0: TLifeManage+4 = Word LE([wL][wH])
  [3E][02][wL][wH][dư…]   L>4  → nt. — byte dư bị BỎ QUA
  [3E][xx>=03]            L=2  → no-op im lặng
```

**Ví dụ khung hoàn chỉnh** (đã XOR `0xAD`):

```
# [3E][01]  → func_0x0075c22c — L=2: method ném ERangeError (len(RP)<2) — ĐỪNG GỬI CÚT NÀY
payload  = 3E 01
L (LE)   = 02 00
pre-XOR  = F4 44 02 00 3E 01
on-wire  = 59 E9 AF AD 93 AC

# [3E][01][01] → phát thông báo #1 (dòng chat, text @0x75C338)
payload  = 3E 01 01
L (LE)   = 03 00
pre-XOR  = F4 44 03 00 3E 01 01
on-wire  = 59 E9 AE AD 93 AC AC

# [3E][02][34 12] → TLifeManage+4 = 0x1234 (Word LE)
payload  = 3E 02 34 12
L (LE)   = 04 00
pre-XOR  = F4 44 04 00 3E 02 34 12
on-wire  = 59 E9 A9 AD 93 AF 99 BF

# [3E][02]  → func_0x0075c1c0 — L=2: ERangeError trong FUN_0077eb9c
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

- **Bắt buộc `L >= 3` cho SubOp 1 và `L >= 4` cho SubOp 2**; `[3E]` (L=1) ném ở handler, cụt hơn ném ở method/codec (§4.1-4.2).
- Args **đã đặc tả được** từ body mới (§4.5): SubOp 1 = 1 byte mã thông báo; SubOp 2 = 1 Word LE. Mock gửi `[3E][01][03]` (toast #3) / `[3E][02][wL][wH]` để quan sát.

---

## 6. Edge cases & ERangeError bounds

| Tình huống | Wire | Hành vi | Bằng chứng |
| :-- | :-- | :-- | :-- |
| `RestPayload` rỗng | `[3E]` (L=1) | `_BoundErr(0)` → **ERangeError** | `case_055_...c:22-25` |
| `SubOp = 0x00` | `[3E][00]` | no-op im lặng | `case_055_...c:27,30` (không khớp) |
| `SubOp >= 0x03` | `[3E][xx]` | no-op im lặng | nt. |
| Byte dư sau SubOp | `[3E][01][dư…]` | **được chuyển nguyên cho method** (không bị bỏ ở handler) | `case_055_...c:28` |
| SubOp 1 thiếu mã thông báo | `[3E][01]` (L=2) | `_BoundErr(1)` → **ERangeError** ngay đầu method | `0075c22c_FUN_0075c22c.c:23-24` |
| SubOp 1 mã ngoài 1..6 | `[3E][01][00]/[07]` | switch không case khớp → return im lặng | `0075c22c_FUN_0075c22c.c:27-45` |
| SubOp 2 cụt Word | `[3E][02]`/`[3E][02][x]` (L<4) | `Copy` ra chuỗi <2 ký tự → `_BoundErr` trong codec | `0075c1c0...c:35-36` + `0077eb9c_FUN_0077eb9c.c:169-171` |
| Bounds bên trong method | — | **ĐÃ BIẾT**: cả 2 method đều bound index AnsiString chuẩn Delphi (range-check bật) | §4.1-4.2 |

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

- **Handler `case_055` không tham chiếu literal chuỗi nào** (chỉ `gvar_007DA250`, 2 `func_0x...`, nhãn SEH) — nhận xét cũ vẫn đúng cho handler.
- Các `UNK_00795a**` là **nhãn SEH/frame-cleanup code address**, **không phải text** (không có tiền tố `s_`/`DAT_`).
- **(đính chính 2026-09-14)** 2 method **đã có body và CÓ tham chiếu 9 literal AnsiString** qua `DAT_...`: `DAT_0075c338`, `DAT_0075c3c0` (2 dòng chat SubOp 1), `DAT_0075c438/468/4c4/4ec` (4 toast SubOp 1), `DAT_0075c6a8/b8/c8` (3 tiền tố của `FUN_0075c518` — OP 0x35). **Bytes của 9 chuỗi chưa được dump** (`redump/` không có `lit_75c3*.hex`/`lit_75c6*.hex`) → **chưa decode VISCII được, cần redump tiếp** (nằm trong vùng data `0x75C045–0x75CA6C` còn lại của HOLE).

---

## 9. Ghi chú cho Mock Server

1. **S→C đầy đủ (đã biết wire)**: `[3E][01][m]` với `m∈1..6` (hiển thị thông báo định trước; `m=1..2` → dòng chat `TTalkMsgForm`, `m=3..6` → toast 2000ms) hoặc `[3E][02][wL][wH]` (gán `TLifeManage+4 = Word LE`).
2. **Tuyệt đối không gửi cụt**: `[3E]` (L=1, handler), `[3E][01]` (L=2, method), `[3E][02]`/`[3E][02][x]` (L<4, codec) — đều `ERangeError`.
3. `SubOp 0x00` và `>= 0x03` **bị bỏ qua im lặng**; SubOp 1 với `m=0` hoặc `>=7` cũng im lặng.
4. **Không có C→S 0x3E** — server không cần nhận/parse.
5. **Hệ quả state**: SubOp 2 thay đổi field `+4`; SubOp 1 không đổi state, chỉ UI. **Chỉ còn thiếu** nội dung text của 9 chuỗi tại `0x75C338/0x75C3C0/0x75C438/0x75C468/0x75C4C4/0x75C4EC/0x75C6A8/0x75C6B8/0x75C6C8` (dump tới null-terminator) để hiển thị đúng thông báo.

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
| 12 | `index.csv:5585` (`0075c004`, size 65) & `index.csv:5586` (`0075ca6c`) | Khe `[0x75C045…0x75CA6C]` — **cập nhật**: 4 hàm trong khe đã export tại `index.csv:6506-6509` (`0075c184/1c0/22c/518`); còn trống `0x75C045–0x75C183` (chứa `TLifeManage.Create @ 0x75C10C`) |
| 13 | **(đính chính)** Grep cũ "0 body" đã lỗi thời — body mới: `functions/0075c22c_FUN_0075c22c.c:22-46`, `functions/0075c1c0_FUN_0075c1c0.c:28-42`, `functions/0075c184_FUN_0075c184.c:18-36`, `functions/0075c518_FUN_0075c518.c:41-119` | Phân tích §4.1-4.4 |
| 14 | `functions/0077f414_FUN_0077F414.c:768,1042-1078` | C→S: `switch(param_2&0xff)` **không có** `case 0x3E` |
| 15 | Grep `MOV DL,0x3E` trong toàn bộ `.asm.txt` → 0 hit | C→S: không có call-site gửi 0x3E |
| 16 | `opcode_35.md §3/§4.2` (tham chiếu nội bộ) | Ngữ cảnh `TLifeManage` (nay đã thay bằng bằng chứng body) |
| 17 | `redump/vmt_75C0B4_TLifeManage.hex` (64B) + resolve `index.csv` | §4.4b: không slot nào trỏ 2 method OP 0x3E → liên kết tĩnh |
| 18 | `functions/0077eb9c_FUN_0077eb9c.c:160-179` | Codec 2B→Word LE + ERangeError khi `<2` ký tự (dùng ở SubOp 2) |

### Điểm chưa xác minh được từ SSOT (cập nhật 2026-09-14)

1. ~~Wire/field layout `RP[1..]`~~ — **ĐÃ XÁC MINH** (§4.5): SubOp 1 = 1 byte mã 1..6; SubOp 2 = Word LE tại `RP[1..2]`.
2. **Ý nghĩa nghiệp vụ con Word `+4`** và chủ đề 6 thông báo SubOp 1 — body chỉ cho thấy cơ chế, **không đủ để kết luận** → để mở.
3. **Bố cục object `TLifeManage`** — dump VMT `0x75C0B4` nay đã có (`redump/vmt_75C0B4_TLifeManage.hex`) nhưng chỉ 64B/16 dword, **không slot nào resolve được trong `index.csv`** (§4.4b); các field khác ngoài `+4` vẫn chưa biết; constructor `0x75C10C` (khe `0x75C045–0x75C183`) **vẫn cần redump**.
4. ~~Bounds nội bộ~~ — **ĐÃ XÁC MINH** (§6): method/codec tự `_BoundErr`.
5. **Nội dung 9 chuỗi** `DAT_0075c338…DAT_0075c6c8` — đã có địa chỉ chính xác từ body, **chưa có bytes** → cần `lit_75c3xx/75c6xx.hex`.
6. **Bản chất 2 method** — **đã đóng**: không xuất hiện trong cửa sổ VMT dump, call-site gọi trực tiếp → liên kết tĩnh, không phải virtual slot (§4.4b).
