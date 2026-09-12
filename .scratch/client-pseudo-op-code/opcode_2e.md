# PHÂN TÍCH — Main OP 0x2E (46) / Case 42 / FUN_00795171 @ 0x00795171

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh phần dispatch từ mã nguồn sơ cấp** (`ts_decompile/` only). Nhánh `SubOp 0x01/0x02/0x03` gọi 3 hàm **không có body trong SSOT (chưa decompile)** → nội dung nghiệp vụ của 3 nhánh là **chưa biết / cần redump**.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x2E là **bộ định tuyến mỏng (thin router)**. Handler chỉ đọc 1 byte SubOp rồi uỷ quyền (delegate) toàn bộ RestPayload cho 1 trong 3 hàm con thuộc một đối tượng toàn cục `gvar_007DA6EC`:
  - `SubOp == 1` → `func_0x005045a0(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - `SubOp == 2` → `func_0x00504f54(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - `SubOp == 3` → `func_0x00505104(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - **Mọi SubOp khác = no-op im lặng** (không có `else` / `default`).
- **Bản thân handler không đọc/không ghi field nào ngoài SubOp** (`case_042_00795171_FUN_00795171.c:20-36`). Nghĩa là: layout payload thực sự nằm bên trong 3 hàm con, mà 3 hàm con này **không có body trong SSOT**, nên **không thể xác minh wire layout của từng nhánh**.
- **Không có chuỗi VISCII nào trong đường đi đã xác minh** (handler không tham chiếu literal nào).
- **Chiều C→S rỗng**: client không bao giờ gửi OP này (`0077f414_FUN_0077F414.c:986-987`).
- 3 hàm con nằm cùng vùng mã `0x00504xxx` với các helper cấu hình đã có body (`00504c50`, `00504c9c`, `00504ce4`), gợi ý đây là một họ phương thức của **một đối tượng cấu hình/điều khiển toàn cục** — nhưng tên lớp VCL cụ thể **không xác minh được từ SSOT** (xem §4.4, đánh dấu *suy luận*).

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2E (46) → byte_table[0x78A8EE][0x2E] = 0x2A (42)
                 → dword_table[0x78A9B6][42] @ 0x0078AA5E = 0x00795171
                 → FUN_00795171 (Case 42)
```

- `redump/jumptable_byte200_0x78A8EE.hex:3` (hàng thứ 3 = index `0x20..0x2F`): `1C 1D 1E 1F 20 21 22 23 24 25 26 27 28 29 2A 00` → index `0x2E` = `0x2A` = 42.
- `case_functions/manifest.csv:44`: `42,0x0078AA5E,0x00795171,EXPORTED,"FUN_00795171","00795171",1,...`.
- `redump/jumptable_0x78A9B6_case_functions.csv:44`: `42,0x0078AA5E,0x00795171,YES,"FUN_00795171",0x00795171,1`.
- File chính: `ts_decompile/case_functions/functions/case_042_00795171_FUN_00795171.c` (68 dòng).
- Bản inline trong dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6650-6673` — **khớp 1:1** (xem §2.4).
- Bản gộp toàn bộ case: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:7990-8030` (trùng nguyên văn với file chính).
- Framing/XOR/pump xem `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2E`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`, tức `RP[0]=P[1]`).
- Delphi string layout: con trỏ trỏ vào byte đầu; **length DWORD nằm tại `[ptr-4]`**, refcount tại `[ptr-8]`.
- `_LStrCopy` Delphi 1-based (không dùng ở OP này vì handler không cắt chuỗi).

### 2.3. Đọc SubOp (dòng 20-27)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);        // iVar2 = RestPayload (RP)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {           // length(RP) == 0 ?
  iVar1 = _BoundErr(0);                    // → BoundErr(0)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);  // SubOp = RP[0] = P[1]
iVar2 = *(int *)(unaff_EBP + -0x14);
if (iVar2 == 1) { ... }
else if (iVar2 == 2) { ... }
else if (iVar2 == 3) { ... }
// KHÔNG có else / default → mọi giá trị khác im lặng
```

- Nguồn RestPayload: biến `-0xc` của frame (`case_042_...c:20`, `:29`, `:32`, `:35`).
- `_BoundErr(0)` được gọi khi **độ dài RestPayload bằng 0** (`case_042_...c:22-25`) — tức frame `[2E]` trần (L=1) sẽ lỗi RangeError trước cả khi đọc SubOp.
- SubOp lưu vào local `-0x14` (`case_042_...c:26-27`).
- **Không đọc thêm byte nào** trong handler: mọi field còn lại (nếu có) chỉ được 3 hàm con diễn giải.

### 2.4. Bản inline 1:1 (dòng 6650-6673)

```c
case 0x2e:
  iVar20 = 0;
  iVar21 = local_10;                 // local_10 = RestPayload
  puStack_20 = &stack0xfffffffc;
  if (*(int *)(local_10 + -4) == 0) {
    in_stack_ffffffd4 = (code *)&UNK_00795184;
    puStack_20 = &stack0xfffffffc;
    iVar20 = @BoundErr(0);
    iVar21 = extraout_EDX_x00163;
  }
  cVar5 = *(char *)(iVar21 + iVar20);          // SubOp = RP[0]
  if (cVar5 == '\x01') { func_0x005045a0(*(undefined4 *)gvar_007DA6EC, local_10); }
  else if (cVar5 == '\x02') { func_0x00504f54(*(undefined4 *)gvar_007DA6EC, local_10); }
  else if (cVar5 == '\x03') { func_0x00505104(*(undefined4 *)gvar_007DA6EC, local_10); }
  break;
```

So với file tách riêng: cùng logic, chỉ khác kiểu biến (`int` so `char`) và tên biến decompile; **cùng tập SubOp (1,2,3), cùng `BoundErr(0)` khi length == 0, cùng không default**. Xác nhận 1:1.

### 2.5. Codec & API

| Helper / đối tượng | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi trực tiếp** trong handler |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi trực tiếp** trong handler |
| `FUN_0077f098` (8B → double) | **Không gọi trực tiếp** trong handler |
| `FUN_0077eb1c` / `FUN_0077ee84` (builder C→S) | Không dùng (chiều này S→C) |
| `func_0x005045a0` | Callee nhánh `SubOp 1` — **không có body trong SSOT** |
| `func_0x00504f54` | Callee nhánh `SubOp 2` — **không có body trong SSOT** |
| `func_0x00505104` | Callee nhánh `SubOp 3` — **không có body trong SSOT** |
| `gvar_007DA6EC` | Đối tượng toàn cục truyền làm tham số 1 (xem §4.4) |
| `_BoundErr` | Chặn `len(RP) == 0` (`case_042_...c:23`) |
| `_LStrArrayClr` / `_LStrClr` | Chỉ dọn dẹp frame khi thoát (`case_042_...c:37-64`), không liên quan wire |

Ghi chú: khối dọn dẹp `_LStrArrayClr(...-0x56c, 99)` v.v. ở `case_042_...c:37-64` là **artefact frame lớn của dispatcher được tách ra kèm case**, không dùng biến nào trong 3 nhánh, không mang ý nghĩa wire.

---

## 3. Bảng tổng hợp SubOp (3 nhánh + no-op)

| SubOp (`RP[0]`) | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2E][01][...phần còn lại...]` | chỉ `SubOp=RP[0]`; phần còn lại **passthrough nguyên khối** | `func_0x005045a0(*(u32*)gvar_007DA6EC, RP)` — **không có body trong SSOT** |
| `0x02` | `[2E][02][...]` | chỉ `SubOp=RP[0]`; phần còn lại passthrough | `func_0x00504f54(*(u32*)gvar_007DA6EC, RP)` — **không có body trong SSOT** |
| `0x03` | `[2E][03][...]` | chỉ `SubOp=RP[0]`; phần còn lại passthrough | `func_0x00505104(*(u32*)gvar_007DA6EC, RP)` — **không có body trong SSOT** |
| `0x00`, `0x04`..`0xFF` | `[2E][xx][...]` | `SubOp` vẫn được đọc | **im lặng (no-op)**, chỉ chạy epilogue dọn frame (`case_042_...c:37-65`) |

Không có `default`. Bất kỳ SubOp ngoài `{1,2,3}` đều rơi xuống cuối hàm và return (`case_042_...c:36-65`).

---

## 4. Chi tiết các nhánh

### 4.1. Cấu trúc dispatch (`case_042_...c:20-36`)

- Toàn bộ thân hàm là 1 cây `if / else if / else if` trên `SubOp`; **không switch**, không bảng nhảy nội bộ.
- Chỉ có 3 lời gọi hàm; mỗi lời gọi truyền **đúng 2 tham số**:
  1. `*(undefined4 *)gvar_007DA6EC` — con trỏ đối tượng toàn cục (đọc rồi dereference).
  2. `*(undefined4 *)(unaff_EBP + -0xc)` — chính là con trỏ `RestPayload` (bao gồm cả SubOp ở byte 0).

### 4.2. Nhánh `SubOp 0x01` — `func_0x005045a0` (`case_042_...c:28-30`)

- Không có body trong SSOT. **Nội dung xử lý: chưa biết / cần redump.**
- Suy ra được từ call-site: callee nhận `(Self, RestPayload)`; RestPayload vẫn còn nguyên byte SubOp `0x01` ở vị trí 0, nên callee có thể đọc thêm dữ liệu tại `RP[1..]`.

### 4.3. Nhánh `SubOp 0x02` — `func_0x00504f54` (`case_042_...c:31-33`) và `SubOp 0x03` — `func_0x00505104` (`case_042_...c:34-36`)

- Cả hai đều không có body trong SSOT. **Nội dung xử lý: chưa biết / cần redump.**
- Bằng chứng duy nhất đáng tin về `func_0x00505104` (nhánh 3): trong bảng "References to entry" của `00504ce4_FUN_00504ce4.c:23` có dòng `00505157 -> 00504ce4 [UNCONDITIONAL_CALL]`. Địa chỉ `0x00505157` nằm **trong** khoảng của hàm bắt đầu tại `0x00505104` (cách đầu hàm `0x53` byte) → *nếu* `func_0x00505104` chính là hàm chứa địa chỉ `0x00505157` (suy luận, vì không có body để xác nhận biên hàm), thì nhánh 3 gọi tiếp parser `FUN_00504ce4`. Dấu hiệu phụ: `0063c674_FUN_0063c674.c:46` có `0050518c -> 0063c674 [UNCONDITIONAL_CALL]`, cũng nằm trong cùng khoảng `0x005051xx`.
  - **Cảnh báo SSOT**: không có file `functions/00505104_*.c` để chứng minh biên hàm, nên liên kết này chưa được xác minh 100%; ghi ở đây như *suy luận có cơ sở địa chỉ*, không phải sự thật đã xác minh.

### 4.4. `func_0x005045a0` / `func_0x00504f54` / `func_0x00505104` — bằng chứng "không có body"

- Grep toàn `ts_decompile/` cho `005045a0|00504f54|00505104` chỉ trả về **đúng các call-site**, không có file định nghĩa:
  - `case_functions/functions/case_042_00795171_FUN_00795171.c:29,32,35`
  - `case_functions/jumptable_0x78A9B6_cases.c:8011,8014,8017`
  - `functions/0078a89c_FUN_0078a89c.c:6663,6667,6671`
- Kiểm tra thư mục: `functions/00504568_*.c` có tồn tại, nhưng **không có** `005045a0`, `00504f54`, `00505104` (glob `ts_decompile/**/*005045a0*` = rỗng; `**/*00504*` chỉ có `00504c50`, `00504c9c`, `00504ce4`, `00504568`). Các hàm kề `00504c50`, `00504c9c` tạo thành một "lỗ hổng" quanh 3 mục tiêu.
- Kết luận: **3 hàm này chưa decompile**. Mọi mô tả wire/payload của 3 nhánh chỉ có thể là giả thuyết cho tới khi redump `0x005045a0`, `0x00504f54`, `0x00505104`.

### 4.5. `gvar_007DA6EC` là gì? (suy luận, có dẫn chứng)

- Grep `gvar_007DA6EC` toàn `ts_decompile/` được 31 match; **mọi match đều là đọc** dạng `*(undefined4 *)gvar_007DA6EC` (không có match gán/khởi tạo). Nghĩa là `gvar_007DA6EC` là **ô nhớ toàn cục chứa con trỏ tới một đối tượng**, và đối tượng đó **không được tạo trong phần code đã decompile** → không xác định được lớp VCL.
- Vị trí dùng làm tham số 1 (Self) cho một họ hàm cùng vùng `0x00504xxx`:
  - `functions/00504c9c_FUN_00504c9c.c` — `*(u32*)gvar_007DA6EC` là `param_1` của `FUN_00504c9c` (caller: `0053d194:110`, `0072174c:751`, `0076d578:484,553`, `0075ad70:55`, `0075b66c:55`, `00642720:50`, `00642c2c:115`, `00643470:99`, `0064f580:43`, `0060ecdc:35`, `005afb00:33`, `0051aae0:58,60`).
  - `functions/00504c50_FUN_00504c50.c` — caller: `case_027_007922E6_FUN_007922e6.c:104`, `functions/0070f7b4_FUN_0070f7b4.c:395`.
  - `functions/00504ce4_FUN_00504ce4.c` — caller: `functions/0053d194_FUN_0053d194.c:110`.
- Bằng chứng nhận dạng (từ body đã có, dùng để suy luận):
  - `FUN_00504ce4` (`00504ce4_...c:82-83`) tạo `TStringList` (`VMT_410860_TStringList`) và ghép đường dẫn literal ASCII `"Server.ini"`, rồi parse dòng theo dấu `.` và `*` (`:95-97`) — đây là **hàm đọc file cấu hình server**. param_1 (`gvar_007DA6EC`) trong hàm này bị gán vào `local_8` rồi không dùng (`:80`) — tức trong các hàm con này, param_1 đóng vai `Self`/receiver nhưng không được đọc.
  - `FUN_00504c50` (`00504c50_...c:36,41`) tra bảng `short` tại `DAT_007d747e` (index 1..4) — một bảng hằng 4 phần tử, không có tên lớp.
  - `FUN_00504c9c` (`00504c9c_...c:58-65`) chỉ kiểm tra modulo/khoảng của `param_2`.
- **Suy luận** (không xác minh được tên lớp): `gvar_007DA6EC` là một **đối tượng toàn cục thuộc họ cấu hình/điều khiển** (cùng nhóm phương thức `0x00504xxx` xử lý `Server.ini`, bảng tra cứu và kiểm tra khoảng), nhiều khả năng là một VCL form/object quản lý cấu hình. **Không thể khẳng định tên lớp cụ thể** vì: (a) đối tượng không được khởi tạo trong SSOT; (b) 3 hàm nhánh không có body; (c) các hàm con có body không tham chiếu VMT/virtual call để lộ class. Cần redump để xác định.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có chuỗi nào trong đường đi đã xác minh.** Handler `case_042_00795171_FUN_00795171.c` (68 dòng) không tham chiếu literal nào; chỉ có code + cleanup frame + `_BoundErr`.
- Ba callee nhánh (`func_0x005045a0`, `func_0x00504f54`, `func_0x00505104`) **không có body**, nên không thể kiểm tra literal bên trong chúng.
- `redump/` không có file `lit_*` nào trỏ tới vùng `0x795xxx` của handler (danh mục chỉ có `lit_77F771`, `lit_78A854`, `lit_5957D8`.., `lit_7A2xxx`, `lit_7ABxxx`) — không có literal nào gắn với OP 0x2E.
- Ghi chú *suy luận* (ngoài đường đi đã xác minh): helper `FUN_00504ce4` dùng literal **ASCII** `"Server.ini"` (`00504ce4_FUN_00504ce4.c:83`) và các ký tự `.` / `*` (`:95,97`). Đây không phải chuỗi VISCII mang nghiệp vụ hiển thị, và việc nó có nằm trong nhánh `SubOp 3` hay không còn phụ thuộc suy luận §4.3.
- Kết luận: **chưa cần map VISCII/cp1258 → UTF-8 cho OP 0x2E ở trạng thái hiện tại**; nếu redump 3 callee phát sinh literal `.rodata`, mới áp quy trình như OP 0x02 (đọc tới null-terminator, map VISCII/cp1258 → UTF-8 NFC).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:986-987`:
  ```c
  case 0x2e:
    break;
  ```
  → **rỗng**, kẹp giữa `case 0x2d:` (`:984-985`) và `case 0x32:` (`:988`).
- Kết luận: OP 0x2E **S→C một chiều**. Client không bao giờ dựng/gửi OP này; mock server không cần format C→S.

---

## 7. Ghi chú cho Mock Server

```
[2E][01][...phần còn lại...]  ≥2B → gọi func_0x005045a0(Self,RP)  (body chưa có → chưa biết field)
[2E][02][...]                 ≥2B → gọi func_0x00504f54(Self,RP)  (body chưa có → chưa biết field)
[2E][03][...]                 ≥2B → gọi func_0x00505104(Self,RP) (body chưa có → chưa biết field)
[2E][00] / [2E][04..FF]       ≥2B → im lặng (no-op)
ĐỪNG GỬI: [2E] trần (L=1) → BoundErr(0) RangeError.
```

- Chỉ byte `P[1]` là được handler diễn giải; phần `P[2..]` là **passthrough** cho callee.
- Vì 3 callee chưa decompile, **không nên gửi bất kỳ payload giả định nào** cho `SubOp 1/2/3` cho tới khi có body. Nếu buộc phải test, chỉ gửi `SubOp` không thuộc `{1,2,3}` để xác nhận no-op an toàn.
- Nhánh no-op có thể dùng làm "probe an toàn": `[2E][00]` phải không gây thay đổi trạng thái.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_042_00795171_FUN_00795171.c` (68 dòng) | Handler chính: đọc SubOp, BoundErr, 3 nhánh gọi callee |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6650-6673` | Bản inline dispatcher — đối chiếu 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex:3` (`[0x2E]=0x2A`) + `manifest.csv:44` + `redump/jumptable_0x78A9B6_case_functions.csv:44` + `case_functions/jumptable_0x78A9B6_cases.c:7990-8030` | Mapping MainOp → Case 42 → `0x00795171` |
| 4 | grep `005045a0|00504f54|00505104` (9 match, chỉ call-site) + glob `functions/*0050*` | Chứng minh 3 callee **không có body** |
| 5 | grep `gvar_007DA6EC` (31 match) + `functions/00504c9c`, `00504c50`, `00504ce4` (body) | Nhận dạng (suy luận) đối tượng toàn cục, họ phương thức `0x00504xxx` |
| 6 | `functions/00504ce4_FUN_00504ce4.c:23` (`00505157 -> 00504ce4`) + `functions/0063c674_FUN_0063c674.c:46` (`0050518c -> 0063c674`) | Suy luận nhánh 3 gọi tiếp parser/config — chưa xác minh |
| 7 | `functions/0077f414_FUN_0077F414.c:986-987` | C→S rỗng (client không gửi) |
| 8 | `redump/` (danh mục `lit_*`) | Không có literal nào gắn OP 0x2E → không có VISCII để decode |
