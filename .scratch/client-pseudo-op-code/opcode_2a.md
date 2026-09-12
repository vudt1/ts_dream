# PHÂN TÍCH — Main OP 0x2A (42) / Case 38 / `FUN_00794719` @ `0x00794719`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Fan-out thuần 4 nhánh, `body_size=1` (body hàm con chưa phục hồi).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: **Fan-out thuần túy** kiểu OP 0x24 — `if/else-if` 4 nhánh `0x01–0x04`, không `default`. Không `IntToStr`, không cộng số, không double, không chat-log, không banner ở tầng dispatcher.
- Mỗi nhánh trao nguyên `RP` (gồm cả byte SubOp) cho 1 hàm con + 1 manager riêng:
  - `0x01` → `func_0x005e5a38(*gvar_007DA1D8, RP)`
  - `0x02` → `func_0x005e6ac4(*gvar_007DA0F8, RP)`
  - `0x03` → `func_0x00748b94(*gvar_007D9D34, RP)`
  - `0x04` → `func_0x0074ec14(*gvar_007D9D34, RP)` (chung object với 03, hàm khác)
- 4 hàm con đều chưa có body (grep toàn cây chỉ trúng dispatcher + inline). Mọi decode số/chuỗi nằm trong đó.
- Literal duy nhất thấy được `0x796A8C` là tên debug MainOP trong log `case 0:`, không phải banner.
- Chiều C→S `case 0x2a: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2A (42) → byte_table[0x78A8EE][0x2A] = 0x26 (38)
                 → dword_table[0x78A9B6][38] @ 0x0078AA4E = 0x00794719
                 → FUN_00794719 (Case 38)
```

- File chính: `ts_decompile/case_functions/functions/case_038_00794719_FUN_00794719.c` (71 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6256-6283` (`case 0x2a:`) — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2A`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `*(RP-4)` = length AnsiString.

### 2.3. Đọc SubOp (dòng 20-28)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 (chỉ [2A]) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
if (==1) func_0x005e5a38(...); else if (==2) func_0x005e6ac4(...);
else if (==3) func_0x00748b94(...); else if (==4) func_0x0074ec14(...);
// 0x00 / ≥0x05: no-op → epilogue _LStrArrayClr/_LStrClr (dòng 40-68)
```

Không gọi codec nào (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`) ở tầng dispatcher.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Core logic |
| :---: | :--- | :--- |
| `0x01` | `[2A][01][blob...]` (≥2B) | `func_0x005e5a38(mgr 007DA1D8, RP)` |
| `0x02` | `[2A][02][blob...]` | `func_0x005e6ac4(mgr 007DA0F8, RP)` |
| `0x03` | `[2A][03][blob...]` | `func_0x00748b94(mgr 007D9D34, RP)` |
| `0x04` | `[2A][04][blob...]` | `func_0x0074ec14(mgr 007D9D34, RP)` |
| `0x00`,`≥0x05` | — | no-op |

Mẫu chung: `func_X(manager_obj, RP)` — wire `[2A][SubOp][blob...]`, parser trong hàm con.

---

## 4. Chi tiết từng SubOp (bỏ graphics/sound/animation — tầng này không có)

- Không nhánh nào cắt field ở tầng dispatcher; không banner/chat-log/sound. Mọi tuyên bố sâu hơn (ID/Word/DWORD/text) cần body 4 hàm con + sample live mỗi SubOp.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có text ở tầng dispatcher**: wire chỉ `[2A][SubOp]` + blob opaque. `redump/` không có `lit_796a8c.hex` (literal duy nhất là tên debug).
- Muốn dịch text: phục hồi body 4 hàm con + redump `.rodata` chúng trỏ tới + 1 frame live mỗi SubOp rồi map cp1258/VISCII → NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:978-979`: `case 0x2a: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x2A S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2A][01][blob...] → func_005e5a38 (mgr 007DA1D8)
[2A][02][blob...] → func_005e6ac4 (mgr 007DA0F8)
[2A][03][blob...] → func_00748b94 (mgr 007D9D34)
[2A][04][blob...] → func_0074ec14 (mgr 007D9D34)
ĐỪNG GỬI: [2A][00], ≥0x05 (no-op); L=1 (RangeError).
```

Tối thiểu 2B payload; blob đuôi do hàm con quyết — chỉ gửi 2B test no-crash hoặc replay blob live nguyên vẹn (gồm cả byte SubOp).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_038_00794719_FUN_00794719.c` (71 dòng) | Handler chính 4 nhánh + epilogue |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6256-6283` + `:715-716` | Bản inline + tên debug `0x796a8c` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2A]=0x26`) + `.csv:40` + `manifest.csv:40` | Mapping |
| 4 | `functions/0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098` | Codec (không gọi trực tiếp) |
| 5 | `functions/0077f414_FUN_0077F414.c:978-979` | C→S rỗng |
| 6 | grep 4 callee toàn cây (chỉ dispatcher+inline) | Giới hạn: chưa body |
