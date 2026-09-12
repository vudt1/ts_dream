# PHÂN TÍCH — Main OP 0x26 (38) / Case 34 / `FUN_00793889` @ `0x00793889`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Hàm ủy thác `func_0x007349f0` chưa có body → từ `P[2]` trở đi unknown.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP đơn-nhánh. Handler không `switch` mà chỉ `if (SubOp==1)` (Ghidra không dựng switch vì 1 nhánh) rồi **passthrough nguyên RestPayload** (gồm cả byte SubOp) cho `func_0x007349f0(player = *gvar_007DA7BC, RP)`.
- Không codec, không `IntToStr`, không hằng chuỗi, không banner/chat-log ở tầng handler — toàn bộ parse nằm trong hàm con.
- `func_0x007349f0` nằm trong gap chưa phục hồi `0x7349ED–0x734B34` (giữa `FUN_00734858` và `FUN_00734b34`, cùng vùng method class player), không entry trong `index.csv`, chỉ 1 điểm gọi toàn cây → không suy ngược được.
- Kẹp giữa Case 33 (OP 0x25, 6 nhánh họ `0x00729xxx`) và Case 35 (OP 0x27, 43 nhánh họ `0x0075xxxx`) nhưng dùng manager khác (player, không phải `007D9C48/007D9C20`) → OP độc lập.
- Chiều C→S **không tồn tại `case 0x26`** (nhảy cóc từ `0x25` sang `0x27`) → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x26 (38) → byte_table[0x78A8EE][0x26] = 0x22 (34)
                 → dword_table[0x78A9B6][34] @ 0x0078AA3E = 0x00793889
                 → FUN_00793889 (Case 34)
```

- File chính: `ts_decompile/case_functions/functions/case_034_00793889_FUN_00793889.c` (61 dòng; logic dòng 20-29, epilogue dòng 30-58)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5756-5770` (`case 0x26:` outer) — khớp 1:1. Lưu ý `case 0x26` xuất hiện 4 lần trong file (3 lần là SubOp của OP 0x00/0x14/0x17, chỉ dòng 5756 là MainOp 0x26).
- Bản copy thứ ba: `case_functions/jumptable_0x78A9B6_cases.c:6746-6804`.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x26`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `SubOp = RP[0] = P[1]`, 1 byte thường.

### 2.3. Đọc SubOp

```c
if (*(RP-4)==0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);
if (SubOp == 1) func_0x007349f0(*gvar_007DA7BC, RP);
// ≠1: no-op (chỉ epilogue _LStrArrayClr/_LStrClr)
```

---

## 3. Bảng tổng hợp SubOp — chỉ 1 SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[26][01][blob...]` (≥2B, phần sau chưa đặc tả) | `SubOp=RP[0]`, passthrough nguyên `RP` | `func_0x007349f0(player, RP)` — chưa body |
| `0x00`,`0x02–0xFF` | `[26][SubOp≠01][...]` | vẫn đọc + guard rỗng | no-op |

Handler không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`).

---

## 4. Chi tiết SubOp `0x01` — ủy thác nguyên RP (core logic, bỏ graphics/sound/animation — tầng này không có)

- **Wire**: `P[0]=0x26, P[1]=0x01, P[2..]=blob` do hàm con tự cắt (format chưa biết).
- **Xử lý**: `func_0x007349f0(*gvar_007DA7BC, RP)` — tham số 1 là bản ghi player, tham số 2 là toàn bộ RP. Pattern passthrough giống OP 0x1F SubOp 06/0E.
- **Giới hạn đã kiểm chứng**: không entry `7349` trong `index.csv`, không file `*007349f0*`; nằm trong gap `0x7349ED–0x734B34` (asm `00734858` epilogue tại `0x7349E9`); grep toàn cây chỉ 2 hit (case + inline) → wire tới `P[1]` là đóng, từ `P[2]` unknown. Hai hàm lân cận cùng class player (`00734858`: `+0x1318/0xE5/0x1320...`; `00734b34`: `+0x5C0/0x3FA/0x63A...`) chỉ mang tính định hướng, không gán offset cho `0x7349F0`.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có gì để decode**: handler không hằng chuỗi (`UNK_/DAT_0079xxxx`), không `IntToStr`, không chat-log, không banner. `redump/` cũng không có `lit_7938xx.hex`.
- Nếu hàm con dựng text thì nằm trong body chưa phục hồi — cần redump `.rodata` quanh `0x7349F0` + string-ref của nó.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:960-973`: dãy `case 0x20..0x25` rồi **nhảy thẳng `case 0x27`** — không tồn tại `case 0x26`, hàm rơi qua switch rồi epilogue.
- Kết luận: OP 0x26 S→C thuần. Không chờ, không mock chiều này.

---

## 7. Ghi chú cho Mock Server

```
S→C [26][01][blob...]    ; SubOp duy nhất; blob từ P[2] chưa đặc tả — phát đúng blob capture, đừng tự chế
S→C [26][SubOp≠01][...]  ; no-op an toàn (test ignore-path)
ĐỪNG GỬI: [26] đơn byte (L=1) → _BoundErr(0) RangeError
C→S [26]: KHÔNG TỒN TẠI
```

1. Payload tối thiểu 2B. Giữ đúng `P[1]=0x01` ở đầu blob (RP truyền nguyên gồm cả SubOp — cắt sai 1 byte là sai toàn bộ parse trong).
2. Không ACK, không text để kiểm tra bằng mắt — quan sát qua state player (cần instrumentation sau khi có body `0x7349F0`).
3. Muốn đóng 100%: decompile/asm `0x007349F0` (gap `0x7349ED–0x734B34`) + string-ref/rodata + 1 gói `[26][01]...` thật.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_034_00793889_FUN_00793889.c` | Handler chính (`SubOp=RP[0]`, `if==1`) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5756-5770` (+`:579`, `:568-572`) | Bản inline khớp 1:1 (RP=`local_10`, MainOp=`local_9`) |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x26]=0x22`) + `jumptable_0x78A9B6_case_functions.csv:34` + `manifest.csv:34` | Mapping tự parse |
| 4 | `jumptable_0x78A9B6_cases.c:6746-6804` | Bản copy thứ ba |
| 5 | `case_033` + `case_035` + inline lân cận | Đối chứng họ hàng 0x25/0x27, chứng minh 0x26 độc lập |
| 6 | `index.csv` (grep `7349` = 0) + `00734858` (size 405) + `00734b34` + asm `JZ/JMP 0x7349e9` | Gap chưa phục hồi |
| 7 | `functions/00734858 / 00734b34` | Bối cảnh class player (định hướng) |
| 8 | `functions/0077f414_FUN_0077F414.c:960-973` | C→S thiếu `case 0x26` |
