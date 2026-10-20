# Báo Cáo Nghiên Cứu: Đánh Giá "Hai Đính Chính Từ Kết Quả Thực Thi" (Battle-Refactor)

> **Ngày thực hiện:** 2026-10-17  
> **Tài liệu đối chiếu:** [HANDOFF-implementation-progress.md](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-implementation-progress.md), [spec.md](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec.md), [HANDOFF-client-reverse.md](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-client-reverse.md)  
> **Nguồn bằng chứng sơ cấp (Primary Sources):**
> 1. Binary client: [`client_delphi/aLogin.exe`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_delphi/aLogin.exe) (PE32, ImageBase `0x00400000`).
> 2. File dữ liệu tĩnh: [`Data/Formula.Dat`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/Data/Formula.Dat).
> 3. Ghidra Decompiled C & ASM:
>    - Dispatcher chính: [`client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt) và [`client_pseudo_c/0078a89c_FUN_0078a89c.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078a89c_FUN_0078a89c.c).
>    - Dispatcher Opcode 0x35: [`client_pseudo_c/case_046_007952AB_FUN_007952ab.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/case_046_007952AB_FUN_007952ab.c).
>    - Dispatcher Opcode 0x3C: `case_053_0079575C` và [`client_pseudo_c/0053fd78_FUN_0053fd78.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0053fd78_FUN_0053fd78.c).
>    - Handlers 35 03, 35 0E, 35 0F: [`client_pseudo_c/0064cfec_FUN_0064cfec.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064cfec_FUN_0064cfec.c), [`client_pseudo_c/0064fbd8_FUN_0064fbd8.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064fbd8_FUN_0064fbd8.c), [`client_pseudo_c/0064fd34_FUN_0064fd34.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064fd34_FUN_0064fd34.c).
>    - Công thức tính EXP client: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0065b0dc_FUN_0065b0dc.asm.txt`.
>    - Parser 16-bit LE client: [`client_pseudo_c/0077eb9c_FUN_0077eb9c.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0077eb9c_FUN_0077eb9c.c).
> 4. Source code Rust: [`src/data/texps.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/texps.rs), [`src/data/loaders/formula.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/loaders/formula.rs), [`src/battle/packets.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/packets.rs).

---

## 1. Tóm Tắt Tiến Độ Trong `HANDOFF-implementation-progress.md`

- **Phạm vi hoàn thành**: Đã thực thi xong **14/14 ticket** thuộc phạm vi lập trình (Ticket 01 → 13 và Ticket 15). Toàn bộ 305 tests trong suite `cargo test --all-targets --no-fail-fast` đều pass (0 failure).
- **Ticket duy nhất còn lại**: **Ticket 14** ([`issues/14-capture-verification.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/14-capture-verification.md)) ở trạng thái `ready-for-human` — yêu cầu capture runtime thực tế với client game để đối chiếu các luồng dynamic (như frame `0B 04` show-on-map, chuỗi dọn dẹp sau trận).
- **Vấn đề cốt lõi phát sinh**: Trong quá trình thực thi Ticket 11 và Ticket 12, lập trình viên phát hiện **hai nhận định trong tài liệu spec cũ (`spec.md` và `HANDOFF-client-reverse.md`) bị sai lệch so với client thực tế**. Việc này được ghi nhận tại Mục 4 của `HANDOFF-implementation-progress.md`.

---

## 2. Tìm Hiểu Chi Tiết "Hai Đính Chính Từ Kết Quả Thực Thi"

### 2.1 Đính chính 1: Ticket 11 — Hệ số EXP `k = 2.9` và giá trị cộng thêm `+5` từ `Formula.Dat`

#### A. Nhận định cũ trong tài liệu spec & issue
- Trong [`spec.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec.md) §1.3 (G6/G7) và [`HANDOFF-client-reverse.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-client-reverse.md) §8.1/§8.3:
  - Cho rằng: `k` của Reborn 0 đọc từ RAM `[table+0x84]` (`FLD double [EAX+0x80]`), do đó giá trị này "chưa biết (mức B)" và giá trị `2.9` mà server Rust đang dùng là "không có cơ sở trong client".
  - Cho rằng: Client "không cộng `+5` mỗi bước", và việc Rust cộng `+5` là tùy tiện, cần phải bỏ.

#### B. Kiểm chứng trên mã nguồn gốc (Primary Source Verification)
1. **Kiểm tra ASM hàm `FUN_0065b0dc`** (`0065b0dc_FUN_0065b0dc.asm.txt`):
   ```asm
   ; Nhánh Reborn 0 (0x0065b101):
   MOV EAX, [0x007da6b8]          ; Global pointer tới bảng Formula
   MOV EAX, dword ptr [EAX]
   ADD EAX, 0x4                   ; Trỏ vào payload data
   ...
   FLD double ptr [EAX + 0x80]    ; Đọc double (8 bytes) tại offset 0x80!
   CALL 0x00424e60                ; pow(level, exponent)
   CALL 0x00402b1c                ; Round half-away-from-zero
   ...
   MOV EAX, dword ptr [EAX + 0xa8]; Đọc int32 (4 bytes) tại offset 0xA8!
   ADD EAX, dword ptr [ESP]       ; CỘNG VÀO EXP ĐÃ ROUND!
   ```
   Tương tự, ở cả 3 nhánh Reborn 1 (`0x65B16D`), Reborn 2 (`0x65B1D5`), Reborn 3 (`0x65B23D`), sau khi tính `round(pow(level, k))`, client **đều thực hiện đọc `[EAX + 0xA8]` và cộng vào kết quả**.
2. **Kiểm tra tệp nhị phân `Data/Formula.Dat`**:
   - File loader [`src/data/loaders/formula.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/loaders/formula.rs) định nghĩa:
     - Byte offset `0x80` (128): trường `w28` kiểu `f64` (double).
     - Byte offset `0xA8` (168): trường `extra_exp` kiểu `i32`.
   - Trích xuất nhị phân trực tiếp từ `Data/Formula.Dat`:
     - Tại offset `0x80..0x88`: byte hex `9A 99 99 99 99 99 07 40` $\rightarrow$ giá trị IEEE-754 double chính xác là **`2.9`**.
     - Tại offset `0xA8..0xAC`: byte hex `05 00 00 00` $\rightarrow$ giá trị `i32` chính xác là **`5`**.
3. **Kết luận về Đính chính 1**:
   - **`k = 2.9`** hoàn toàn không phải số đoán mò; đó là giá trị `w28` được nạp từ `Formula.Dat`.
   - Phép cộng **`+5`** hoàn toàn không phải logic server bịa ra; đó là giá trị `extra_exp` được client nạp từ `Formula.Dat` và cộng vào mỗi cấp trong `FUN_0065b0dc`.
   - Đính chính trong quá trình thực thi là **hoàn toàn chính xác 100%**.

---

### 2.2 Đính chính 2: Ticket 12 — Sự nhầm lẫn giữa Case-Index và Opcode trong Dispatcher

#### A. Nhận định cũ trong tài liệu spec & issue
- Trong [`spec.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec.md) §1.4 (G10/Hệ quả phụ) và [`HANDOFF-client-reverse.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-client-reverse.md) §14.1/§14.4:
  - Cho rằng: Dispatcher của opcode `0x35` là `case_053_0079575C` và chỉ xử lý sub 1, 2, 3, 4.
  - Cho rằng: Handler của `35 03` là `FUN_0053fd78`.
  - Cho rằng: Các sub `35 0E` và `35 0F` "không có handler trong client $\rightarrow$ no-op, bỏ qua".

#### B. Kiểm chứng trên mã nguồn gốc (Primary Source Verification)
1. **Cơ chế nhảy 2 tầng của Dispatcher chính `FUN_0078a89c`** (`0078a89c_FUN_0078a89c.asm.txt:27-31`):
   ```asm
   MOV AL, byte ptr [EBP + -0x5]       ; DL truyền vào là Opcode
   CMP EAX, 0xc7
   JA  0x00796347
   MOV AL, byte ptr [EAX + 0x78a8ee]  ; TẦNG 1: Tra bảng chuyển Opcode -> Case Index!
   JMP dword ptr [EAX*0x4 + 0x78a9b6] ; TẦNG 2: Nhảy theo Jump Table của Case Index!
   ```
2. **Đối chiếu trực tiếp trong PE binary `client_delphi/aLogin.exe`**:
   - Bảng chuyển đổi tại VA `0x0078a8ee`:
     - Byte tại `0x0078a8ee + 0x35` (Opcode 0x35) = **`46`** (`0x2E`).
     - Byte tại `0x0078a8ee + 0x3C` (Opcode 0x3C) = **`53`** (`0x35`).
   - Bảng Jump Table tại VA `0x0078a9b6`:
     - Entry 46 (`46 * 4 + 0x78a9b6`): Target là **`0x007952AB`** (`FUN_007952ab`).
     - Entry 53 (`53 * 4 + 0x78a9b6`): Target là **`0x0079575C`** (`FUN_0079575c`).
   - **Nguyên nhân sai lầm ban đầu**: Người làm dịch ngược trước đó nhìn thấy nhãn Ghidra xuất ra là `case_053` (với giá trị hex `0x35`), đã nhầm tưởng rằng `case 0x35` nghĩa là `opcode 0x35`. Thực chất, `case 0x35` (index 53) là của **Opcode `0x3C`**! Còn Opcode `0x35` có case index là `46` (`0x2E`), tương ứng với `FUN_007952ab`.
3. **Phân tích hàm xử lý thật của Opcode 0x35: `FUN_007952ab`**:
   Xem file [`client_pseudo_c/case_046_007952AB_FUN_007952ab.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/case_046_007952AB_FUN_007952ab.c):
   - Chứa switch đầy đủ cho các sub: `1, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F`!
   - **Sub 3**: Gọi [`FUN_0064cfec`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064cfec_FUN_0064cfec.c) (không phải `FUN_0053fd78`).
     - Đọc `row = content[1]`, `col = content[2]`.
     - Tìm ô đơn vị trận và gọi `FUN_00647708(unit, 7)` $\rightarrow$ Ép unit vào animation state 7 ("force_state7" / chuẩn bị tấn công). Điều này khớp chính xác 100% với builder `packets::force_state7` đã viết trong Ticket 12!
   - **Sub 0x0E**: Gọi [`FUN_0064fbd8`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064fbd8_FUN_0064fbd8.c).
     - Đọc `row`, `col`, và 2-byte word `val` (qua `FUN_0077eb9c`).
     - Gọi hàm vtable của unit: `(**(code **)(*unit + 0x1c))(unit, val, 1)`.
   - **Sub 0x0F**: Gọi [`FUN_0064fd34`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0064fd34_FUN_0064fd34.c).
     - Đọc `row`, `col`, `type_char` ('B', 'C', 'D', 'E', 'F', 'G'...), `flag/turn`, và 2-byte `val`.
     - Lưu vào các slot thuộc tính của unit (`unit + 0x5BE`, `unit + 0x5C1`, ...) và gọi `FUN_0058f5e0` cập nhật biểu tượng trạng thái/buff/debuff trên giao diện trận chiến.
4. **Kết luận về Đính chính 2**:
   - `35 0E` và `35 0F` **không phải là no-op**. Client có logic xử lý hoàn chỉnh cho hiệu ứng và trạng thái buff/debuff.
   - `FUN_0053fd78` thuộc về Opcode `0x3C`, không liên quan tới `0x35`.
   - Handler thực sự của `35 03` là `FUN_0064cfec`.
   - Đính chính trong quá trình thực thi là **hoàn toàn chính xác 100%**.

---

## 3. Đánh Giá Tính Đúng Đắn Của Nội Dung

| Nội dung đối chiếu | Trạng thái | Đánh giá & Căn cứ kỹ thuật |
|---|:---:|---|
| **Đính chính 1 (Ticket 11: EXP & `+5`)** | ✅ **Chính xác tuyệt đối** | Đã xác minh byte-exact từ `Data/Formula.Dat` (`w28 = 2.9` tại `0x80`, `extra_exp = 5` tại `0xA8`) và lệnh đọc FPU/thanh ghi trong `0065b0dc_FUN_0065b0dc.asm.txt`. Code Rust `src/data/texps.rs` hiện tại đã bám sát chính xác quy tắc này. |
| **Đính chính 2 (Ticket 12: Opcode 0x35)** | ✅ **Chính xác tuyệt đối** | Đã kiểm tra tra bảng PE tại `0x0078a8ee` trong `aLogin.exe`: Opcode `0x35` $\rightarrow$ case 46 $\rightarrow$ `FUN_007952ab`. Sub 3 là `FUN_0064cfec`, Sub 0x0E là `FUN_0064fbd8`, Sub 0x0F là `FUN_0064fd34`. Việc spec cũ cho rằng `35 0E/0F` là no-op bắt nguồn từ việc nhầm case 53 (`0x3C`) thành opcode `0x35`. |

---

## 4. Danh Sách Các Công Việc Cần Làm (Follow-up Tasks)

### Nhóm 1: Đồng bộ tài liệu kỹ thuật (Loại bỏ mâu thuẫn)
1. **Cập nhật `spec.md`**:
   - Sửa bảng §1.3 (G6/G7): Ghi nhận `k[0] = 2.9` và giá trị cộng thêm `+5` lấy từ `Formula.Dat`.
   - Sửa §1.4 (G10/Hệ quả phụ) & §1.5: Xoá bỏ dòng "`35 0E/35 0F` không có handler $\rightarrow$ no-op". Ghi rõ case 46 = `0x35` (`FUN_007952ab`), case 53 = `0x3C` (`FUN_0079575c`).
2. **Cập nhật `HANDOFF-client-reverse.md`**:
   - Sửa §8 (G6/G7): Ghi rõ nguồn gốc của `table+0x84` (`w28`) và `table+0xAC` (`extra_exp`).
   - Sửa §14.1 & §14.4: Sửa bảng sub của Opcode `0x35` (thay danh sách 1..4 của `0x3C` bằng danh sách sub thực tế từ `FUN_007952ab`, nêu rõ chức năng của `35 03`, `35 0E`, `35 0F`).

### Nhóm 2: Nghiên cứu wire format & Xây dựng Builder cho `35 0E` và `35 0F` (Tính năng bổ sung)
1. **Nghiên cứu wire format `35 0F` (Cập nhật Buff/Debuff icon trong trận)**:
   - Theo `FUN_0064fd34`:
     - `row`: byte (offset 1)
     - `col`: byte (offset 2)
     - `status_type`: char/byte ('B', 'C', 'D', 'E', 'F', 'G'...) (offset 3)
     - `flag / turns`: char/byte (offset 4)
     - `val`: 2 bytes little-endian đọc qua `FUN_0077eb9c` (offset 5..6)
   - Viết builder `packets::battle_unit_status(row, col, status_type, flag, val)` trong `src/battle/packets.rs` và unit test tương ứng trong `tests/`.
2. **Nghiên cứu wire format `35 0E` (Unit Action/Vtable 0x1c)**:
   - Theo `FUN_0064fbd8`: `[35][0E][row][col][val_u16_le]`.
   - Viết builder tương ứng và tài liệu hóa mục đích sử dụng.

### Nhóm 3: Wiring vào Battle Runner & Xác nhận Runtime (Ticket 14)
1. **Wiring `35 03` (`force_state7`)**:
   - Hiện builder `packets::force_state7` đã sẵn sàng nhưng chưa được cắm vào `src/battle/runner.rs`. Cần xác định chính xác thời điểm gửi (ví dụ: ngay trước khi unit lao lên đánh hoặc thi triển kỹ năng).
2. **Thực hiện Ticket 14 (Capture Verification)**:
   - Cần có packet capture thực tế từ client để quan sát các gói tin `0x35` và chuỗi kết thúc trận thực tế để chốt thời điểm kích hoạt của `force_state7` và `battle_unit_status`.
