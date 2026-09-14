# PHÂN TÍCH — Main OP 0x2E (46) / Case 42 / FUN_00795171 @ 0x00795171

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh toàn bộ từ mã nguồn sơ cấp** (`ts_decompile/` only). **Cập nhật 2026-09-14: cả 3 callee `005045a0 / 00504f54 / 00505104` đã có body** → 3 nhánh không còn "chưa biết"; nội dung: **chuyển kênh/server (reconnect) + banner thông báo + dialog xác nhận**, xem §4.2–4.4.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x2E là **bộ định tuyến mỏng (thin router)**. Handler chỉ đọc 1 byte SubOp rồi uỷ quyền (delegate) toàn bộ RestPayload cho 1 trong 3 hàm con thuộc một đối tượng toàn cục `gvar_007DA6EC`:
  - `SubOp == 1` → `func_0x005045a0(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - `SubOp == 2` → `func_0x00504f54(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - `SubOp == 3` → `func_0x00505104(*(undefined4 *)gvar_007DA6EC, RestPayload)`
  - **Mọi SubOp khác = no-op im lặng** (không có `else` / `default`).
- **Bản thân handler không đọc/không ghi field nào ngoài SubOp** (`case_042_00795171_FUN_00795171.c:20-36`). Layout payload nằm trong 3 hàm con — **cả 3 đã có body (2026-09-14)**, wire từng nhánh xác minh được: 0x01 = 2 byte, 0x02/0x03 = 1 byte (§4.2–4.4).
- **Không có chuỗi VISCII nào trong đường đi đã xác minh** (handler không tham chiếu literal nào).
- **Chiều C→S rỗng**: client không bao giờ gửi OP này (`0077f414_FUN_0077F414.c:986-987`).
- 3 hàm con (NAY đã có body) xác nhận: chúng là họ phương thức của **đối tượng cấu hình/điều khiển toàn cục `gvar_007DA6EC`** — nhánh 1 đổi server + reconnect TClientSocket port 6414, nhánh 3 mở dialog `FUN_0063c674`. Tên lớp VCL cụ thể vẫn **không xác minh được** (không có dòng tạo object trong SSOT — xem §4.5).

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
| `func_0x005045a0` | Callee nhánh `SubOp 1` — **đã có body** (switch server + reconnect, §4.2) |
| `func_0x00504f54` | Callee nhánh `SubOp 2` — **đã có body** (4 banner, §4.3) |
| `func_0x00505104` | Callee nhánh `SubOp 3` — **đã có body** (config lookup + dialog, §4.4) |
| `gvar_007DA6EC` | Đối tượng toàn cục truyền làm tham số 1 (xem §4.4) |
| `_BoundErr` | Chặn `len(RP) == 0` (`case_042_...c:23`) |
| `_LStrArrayClr` / `_LStrClr` | Chỉ dọn dẹp frame khi thoát (`case_042_...c:37-64`), không liên quan wire |

Ghi chú: khối dọn dẹp `_LStrArrayClr(...-0x56c, 99)` v.v. ở `case_042_...c:37-64` là **artefact frame lớn của dispatcher được tách ra kèm case**, không dùng biến nào trong 3 nhánh, không mang ý nghĩa wire.

---

## 3. Bảng tổng hợp SubOp (3 nhánh + no-op)

| SubOp (`RP[0]`) | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2E][01][c1:1B][c2:1B]` (4B) | callee đọc `RP[1]`, `RP[2]` (+ `RP[1]` làm index) | `FUN_005045a0`: chọn server/kênh — hợp lệ → set socket port `0x190e` (6414) + address từ `FUN_00504ce4` + `Open` + `FUN_0077f414` (§4.2) |
| `0x02` | `[2E][02][v:1B]` (3B) | callee đọc `RP[1]` | `FUN_00504f54`: v=1..4 → 1 trong 4 banner 2000ms (§4.3) |
| `0x03` | `[2E][03][idx:1B]` (3B) | callee đọc `RP[1]` | `FUN_00505104`: `FUN_00504ce4(self, idx, 2)` → chuỗi config → dialog `FUN_0063c674(*gvar_007D9D6C, …)` (§4.4) |
| `0x00`, `0x04`..`0xFF` | `[2E][xx][...]` | `SubOp` vẫn được đọc | **im lặng (no-op)**, chỉ chạy epilogue dọn frame (`case_042_...c:37-65`) |

**Đính chính (2026-09-14)**: nhận định cũ "3 callee không có body → không thể xác minh wire layout từng nhánh" đã được thay thế — wire thật của 3 nhánh **ngắn và cố định** (4B / 3B / 3B; byte thừa bị bỏ qua), chi tiết §4.2–4.4.

Không có `default`. Bất kỳ SubOp ngoài `{1,2,3}` đều rơi xuống cuối hàm và return (`case_042_...c:36-65`).

---

## 4. Chi tiết các nhánh

### 4.1. Cấu trúc dispatch (`case_042_...c:20-36`)

- Toàn bộ thân hàm là 1 cây `if / else if / else if` trên `SubOp`; **không switch**, không bảng nhảy nội bộ.
- Chỉ có 3 lời gọi hàm; mỗi lời gọi truyền **đúng 2 tham số**:
  1. `*(undefined4 *)gvar_007DA6EC` — con trỏ đối tượng toàn cục (đọc rồi dereference).
  2. `*(undefined4 *)(unaff_EBP + -0xc)` — chính là con trỏ `RestPayload` (bao gồm cả SubOp ở byte 0).

### 4.2. Nhánh `SubOp 0x01` — `FUN_005045a0` (body mới: `005045a0_FUN_005045a0.c`)

- **Wire**: guard `len>=3` → đọc `c1=RP[1]` (`P[2]`), `c2=RP[2]` (`P[3]`) (`:53-66`).
- **Điều kiện hợp lệ**: `(c1∈{1,2} && c2∈{2,3}) || (c1==0 && c2==7)` (`:67-68`).
- Nếu hợp lệ: ghi global byte `*PTR_gvar_007D748C (→gvar_007DA20C) = c1` (server/kênh đang chọn), `*PTR_gvar_007D7488 (→gvar_007DA078) = *gvar_007DA06C` (backup) (`:75-76`); validate bằng `FUN_00504c9c(self,c1) || FUN_00504c9c(self, *gvar_007DA078)` (`:77-78`):
  - **OK → RECONNECT**: `*(gvar_007DA37C obj+0x1e)=1`; `FUN_00603f20()`; `TClientSocket(*(gvar_007DA664 obj)+1000).SetPort(0x190E = 6414)`; address := `FUN_00504ce4(self, c1, 1)` (parser `Server.ini` đã có body — xác nhận vai trò đối tượng) → `SetAddress`; `_LStrClr(*gvar_007D9D9C)`; `TAbstractSocket.Open`; `FUN_0077f414(*gvar_007D9D30, 0)` (bật lại pump nhận package) (`:79-92`).
  - **FAIL**: dựng chuỗi lỗi từ literal `.text` `0x5049b4` + `FUN_00504ce4(self,c1,2)` + `DAT_005049d0/9e8` → `FUN_00603f20()` (`:93-104`).
- Nếu không hợp lệ: switch `c1`∈{3,4,5,7,8,A} + switch `c2`∈{1,5,6,8,9} nối các literal `.text` (`0x504a20…0x504c20`, chưa dump) → nếu chuỗi ≠ rỗng: **banner 2000ms** (`:106-150`).
- Kết luận: SubOp 1 = **lệnh chuyển server/kênh chơi (port 6414) kèm thông báo điều kiện sử dụng**.

### 4.3. Nhánh `SubOp 0x02` — `FUN_00504f54` (`00504f54_FUN_00504f54.c:37-57`)

- Guard `len>=2` → `v=RP[1]`; v∈{1,2,3,4} → 1 trong 4 banner `*gvar_007DA084` 2000ms (literal `.text` `DAT_00505048/00505090/005050B0/005050D8`, chưa dump); khác → im lặng. Wire `[2E][02][v]` = 3B.

### 4.4. Nhánh `SubOp 0x03` — `FUN_00505104` (`00505104_FUN_00505104.c:42-54`) + xác minh suy luận cũ

- Guard `len>=2` → `idx=RP[1]`; `FUN_00504ce4(self, idx, 2, out)` (đọc config); chuỗi hiển thị = `DAT_005051c8 (prefix .text) + out` (`_LStrCat3`); rồi `FUN_0063c674(*gvar_007D9D6C, msg, 0, 0, 0x005051E8, self, 0x005051F4, self)` — dialog/hộp thoại với **2 callback ghim trong `.text`** (`0x5051e8`, `0x5051f4` — nhiều khả năng Yes/No, chưa xác minh được nội dung). Wire `[2E][03][idx]` = 3B.
- **Suy luận cũ (bản 2026-09-12) nay được xác minh từ body mới**: đúng là body `00505104` chứa 2 call-site `0x00505157 → FUN_00504ce4` (dòng 51) và `0x0050518C → FUN_0063c674` (dòng 54). *Lưu ý*: header dump ghi `Size: 69 bytes` (tương ứng `0x00505104–0x00505149`) là **undercount của Ghidra** — code + exception frame thực tế extends tới ≥ `0x005051B4` (`LAB_005051b4` được tham chiếu ngay trong file).
- (Xóa) Các mục "bằng chứng không có body" (grep chỉ trúng call-site, glob rỗng) không còn giá trị: `functions/005045a0/00504f54/00505104_FUN_*.c` đã tồn tại từ đợt redump 2026-09-14.

### 4.5. `gvar_007DA6EC` là gì? (suy luận cũ + bằng chứng mới xác nhận hướng)

- Grep `gvar_007DA6EC` toàn `ts_decompile/` được 31 match; **mọi match đều là đọc** dạng `*(undefined4 *)gvar_007DA6EC` (không có match gán/khởi tạo). Nghĩa là `gvar_007DA6EC` là **ô nhớ toàn cục chứa con trỏ tới một đối tượng**, và đối tượng đó **không được tạo trong phần code đã decompile** → không xác định được lớp VCL.
- Vị trí dùng làm tham số 1 (Self) cho một họ hàm cùng vùng `0x00504xxx`:
  - `functions/00504c9c_FUN_00504c9c.c` — `*(u32*)gvar_007DA6EC` là `param_1` của `FUN_00504c9c` (caller: `0053d194:110`, `0072174c:751`, `0076d578:484,553`, `0075ad70:55`, `0075b66c:55`, `00642720:50`, `00642c2c:115`, `00643470:99`, `0064f580:43`, `0060ecdc:35`, `005afb00:33`, `0051aae0:58,60`).
  - `functions/00504c50_FUN_00504c50.c` — caller: `case_027_007922E6_FUN_007922e6.c:104`, `functions/0070f7b4_FUN_0070f7b4.c:395`.
  - `functions/00504ce4_FUN_00504ce4.c` — caller: `functions/0053d194_FUN_0053d194.c:110`.
- Bằng chứng nhận dạng (từ body đã có, dùng để suy luận):
  - `FUN_00504ce4` (`00504ce4_...c:82-83`) tạo `TStringList` (`VMT_410860_TStringList`) và ghép đường dẫn literal ASCII `"Server.ini"`, rồi parse dòng theo dấu `.` và `*` (`:95-97`) — đây là **hàm đọc file cấu hình server**. param_1 (`gvar_007DA6EC`) trong hàm này bị gán vào `local_8` rồi không dùng (`:80`) — tức trong các hàm con này, param_1 đóng vai `Self`/receiver nhưng không được đọc.
  - `FUN_00504c50` (`00504c50_...c:36,41`) tra bảng `short` tại `DAT_007d747e` (index 1..4) — một bảng hằng 4 phần tử, không có tên lớp.
  - `FUN_00504c9c` (`00504c9c_...c:58-65`) chỉ kiểm tra modulo/khoảng của `param_2`.
- **Bằng chứng mới từ 3 body**: cả 3 callee là phương thức của chính đối tượng này, dùng nó để tra `Server.ini` (`FUN_00504ce4(self, idx, 1/2)` → address/string — `005045a0_FUN_005045a0.c:86,96`; `00505104_FUN_00505104.c:51`) rồi **điều khiển TClientSocket reconnect port 6414** (`005045a0_FUN_005045a0.c:84-89`) → vai trò **"server/channel selector"** xác nhận được ở mức hành vi.
- **Suy luận** (không xác minh được tên lớp): `gvar_007DA6EC` là một **đối tượng toàn cục thuộc họ cấu hình/điều khiển** (cùng nhóm phương thức `0x00504xxx` xử lý `Server.ini`, bảng tra cứu và kiểm tra khoảng), nhiều khả năng là một VCL form/object quản lý cấu hình. **Không thể khẳng định tên lớp cụ thể** vì: (a) đối tượng không được khởi tạo trong SSOT; (b) (lý do cũ "3 hàm nhánh không có body" đã hết hiệu lực sau redump 2026-09-14); (c) kể cả 3 body mới cũng không tham chiếu VMT/virtual call để lộ class. Cần tìm dòng tạo object (HOLE) để chốt.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có chuỗi nào trong đường đi đã xác minh.** Handler `case_042_00795171_FUN_00795171.c` (68 dòng) không tham chiếu literal nào; chỉ có code + cleanup frame + `_BoundErr`.
- Ba callee nhánh **đã có body (2026-09-14)** nhưng **toàn bộ literal của chúng ghim trong `.text`** (`0x5049b4–0x504c20`, `0x505048–0x5050d8`, `0x5051c8`) — ngoài quy ước `lit_*` (`.rodata/.data`) → **vẫn chưa decode được**; cần redump các địa chỉ `.text` này.
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
[2E][01][c1][c2]   4B → chuyển server/kênh (c1∈{1,2}&c2∈{2,3} hoặc c1=0&c2=7) — CÓ TÁC DỤNG MẠNG (reconnect port 6414)!
[2E][02][v]        3B → banner v=1..4
[2E][03][idx]      3B → tra config + dialog xác nhận
[2E][00] / [2E][04..FF]       ≥2B → im lặng (no-op)
ĐỪNG GỬI: [2E] trần (L=1) → BoundErr(0) RangeError.
```

- Chỉ byte `P[1]` được handler diễn giải; `P[2..]` do callee đọc (0x01 cần thêm **2 byte**, 0x02/0x03 thêm **1 byte**; tail thừa bỏ qua).
- **Cảnh báo mock**: `SubOp 0x01` với cặp byte hợp lệ sẽ đổi server index và mở lại socket (`005045a0_FUN_005045a0.c:82-90`) — chỉ gửi khi mô phỏng chuyển kênh thật; payload cụt → `BoundErr` trong callee.
- Nhánh no-op có thể dùng làm "probe an toàn": `[2E][00]` phải không gây thay đổi trạng thái.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_042_00795171_FUN_00795171.c` (68 dòng) | Handler chính: đọc SubOp, BoundErr, 3 nhánh gọi callee |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6650-6673` | Bản inline dispatcher — đối chiếu 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex:3` (`[0x2E]=0x2A`) + `manifest.csv:44` + `redump/jumptable_0x78A9B6_case_functions.csv:44` + `case_functions/jumptable_0x78A9B6_cases.c:7990-8030` | Mapping MainOp → Case 42 → `0x00795171` |
| 4 | `functions/005045a0_FUN_005045a0.c` + `00504f54_FUN_00504f54.c` + `00505104_FUN_00505104.c` (body mới 2026-09-14) | §4.2–4.4: wire + hành vi 3 nhánh |
| 5 | grep `gvar_007DA6EC` (31 match) + `functions/00504c9c`, `00504c50`, `00504ce4` (body) | Nhận dạng (suy luận) đối tượng toàn cục, họ phương thức `0x00504xxx` |
| 6 | `functions/00504ce4_FUN_00504ce4.c:23` (`00505157 -> 00504ce4`) + `functions/0063c674_FUN_0063c674.c:46` (`0050518c -> 0063c674`) + body mới | Nhánh 1/3 gọi parser config — **đã xác minh từ body** |
| 9 | ls `redump/` (không có `lit_5049xx/504axx/504bxx/505xxx`) | Literal `.text` của 3 callee chưa dump — giới hạn còn lại |
| 7 | `functions/0077f414_FUN_0077F414.c:986-987` | C→S rỗng (client không gửi) |
| 8 | `redump/` (danh mục `lit_*`) | Không có literal `.rodata` nào gắn OP 0x2E; **literal của 3 callee nằm trong `.text` (`0x5049xx–0x5051xx`) — cần redump riêng, chưa decode được** |
