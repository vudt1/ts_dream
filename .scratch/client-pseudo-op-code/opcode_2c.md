# PHÂN TÍCH — Main OP 0x2C (44) / Case 40 / `FUN_00794910` @ `0x00794910`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler mini 3 nhánh, không default, không codec.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Fan-out mini cùng 1 manager `*gvar_007DA6C8` cho cả 3 nhánh, kiểu OP 0x2A nhưng không có blob biến dài tường minh, không nhánh số, không banner ở tầng dispatcher.
- `0x01` → `func_0x0055da08(mgr, RP)` (forward nguyên RP, body chưa phục hồi).
- `0x02/0x03` → `func_0x0055dfac(mgr, 1/2)` (hằng immediate, bỏ đuôi wire — pattern setter/enum giống OP 0x24 SubOp 0B/0E).
- 2 hàm con đều chưa có body (`0055d584/0055d8e8` nhảy lên `0055e958`, vắng `0055da08/0055dfac`).
- Chiều C→S `case 0x2c: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2C (44) → byte_table[0x78A8EE][0x2C] = 0x28 (40)
                 → dword_table[0x78A9B6][40] @ 0x0078AA56 = 0x00794910
                 → FUN_00794910 (Case 40)
```

- File chính: `ts_decompile/case_functions/functions/case_040_00794910_FUN_00794910.c` (68 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6348-6371` — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2C`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `*(RP-4)` = length.

### 2.3. Đọc SubOp (dòng 20-28)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 (chỉ [2C]) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
if (==1) func_0x0055da08(...); else if (==2) func_0x0055dfac(...,1); else if (==3) func_0x0055dfac(...,2);
// 0x00 / ≥0x04: no-op → epilogue _LStrArrayClr/_LStrClr (dòng 37-65)
```

Không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`), không `IntToStr`/banner/chat-log.

---

## 3. Bảng tổng hợp SubOp (3 nhánh)

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2C][01]` (2B) | `SubOp=RP[0]` | `func_0x0055da08(*gvar_007DA6C8, RP)` — forward nguyên RP |
| `0x02` | `[2C][02]` (2B) | `SubOp=RP[0]` | `func_0x0055dfac(*gvar_007DA6C8, 1)` — hằng 1, bỏ đuôi |
| `0x03` | `[2C][03]` (2B) | `SubOp=RP[0]` | `func_0x0055dfac(*gvar_007DA6C8, 2)` — hằng 2, bỏ đuôi |
| `0x00`,`≥0x04` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (bỏ graphics/sound/animation — tầng này không có)

- Không nhánh nào cắt field; đuôi thừa sau `P[1]` vô nghĩa với 02/03 và opaque với 01. Mọi parse nằm trong 2 hàm con chưa phục hồi.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có payload text**: 3 nhánh chỉ mang SubOp (+ hằng trong code). Literal duy nhất `0x796AE8` là tên debug MainOP trong log `case 0:`, không phải nội dung wire. `redump/` không có `lit_796ae8.hex` → chưa decode được.
- Muốn dịch nghiệp vụ thật: phục hồi body `0055da08/0055dfac` + redump `.rodata` chúng trỏ tới + 1 frame live mỗi SubOp.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:982-983`: `case 0x2c: break;` — rỗng (kẹp giữa `0x2b` và `0x2d`).
- Kết luận: OP 0x2C S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2C][01] → func_0055da08 (mgr 007DA6C8, RP)
[2C][02] → func_0055dfac (mgr, 1)
[2C][03] → func_0055dfac (mgr, 2)
ĐỪNG GỬI: [2C][00], ≥0x04 (no-op); L=1 (RangeError).
```

Tối thiểu 2B cho cả 3 nhánh; tail thừa vô nghĩa (02/03) / opaque (01) — test no-crash gửi 2B, fuzz tail replay blob live.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_040_00794910_FUN_00794910.c` | Handler chính 3 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6348-6371` + `:721-722` | Bản inline + tên debug `0x796ae8` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2C]=0x28`) + `manifest.csv:42` + `.csv:42` + `jumptable_0x78A9B6_cases.c` | Mapping |
| 4 | `functions/0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098` | Xác minh không gọi |
| 5 | `functions/0077f414_FUN_0077F414.c:982-983` | C→S rỗng |
| 6 | ls `functions/` (vắng `0055da08/0055dfac`) + ls `redump/` (vắng `lit_796ae8`) | Giới hạn |
