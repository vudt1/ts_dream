# PHÂN TÍCH — Main OP 0x28 (40) / Case 36 / `FUN_007943BF` @ `0x007943BF`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler 61 dòng, 1 nhánh `if`, không switch, không codec.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Cổng **passthrough 1 nhánh** — không phải kênh số (không DWORD/Word/double, không `IntToStr`), không banner/chat-log ở tầng handler.
- Chỉ **01 SubOp hoạt động: `0x01`**: trao nguyên `RP` (gồm cả byte SubOp) cho `func_0x005ab3f8(*gvar_007DA238, RP)`. Parser thật nằm trong hàm con — **chưa có body** (không file `005ab3f8*.c`, chỉ có họ `005ab0a0/005ab5c8/005ab678/005abcf0`).
- Mọi SubOp khác (`0x00, 0x02–0xFF`) no-op.
- Chiều C→S `case 0x28: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x28 (40) → byte_table[0x78A8EE][0x28] = 0x24 (36)
                 → dword_table[0x78A9B6][36] @ 0x0078AA46 = 0x007943BF
                 → FUN_007943bf (Case 36)
```

- File chính: `ts_decompile/case_functions/functions/case_036_007943BF_FUN_007943bf.c` (61 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6124-6138` (`case 0x28:`) — khớp 1:1.
- Copy thứ ba: `case_functions/jumptable_0x78A9B6_cases.c:7180-7231`.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x28`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). `*(RP-4)` = `Length(RP)`.

### 2.3. Đọc SubOp (dòng 20-29)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 (payload chỉ [28]) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
if (SubOp == 1) func_0x005ab3f8(*gvar_007DA238, RP);
// else: no-op → epilogue _LStrArrayClr/_LStrClr (dòng 30-58)
```

Không `_LStrCopy`, không guard `len≥2/3`, không cắt field cố định. Không gọi codec nào trong 5 helper.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[28][01][blob...]` (≥2B, đuôi do hàm con parse) | `SubOp=RP[0]`, guard rỗng | `func_0x005ab3f8(*gvar_007DA238, RP)` — passthrough nguyên RP |
| `0x00`,`0x02–0xFF` | `[28][xx]` | cùng guard | no-op |

---

## 4. Chi tiết SubOp `0x01` — Passthrough duy nhất (bỏ graphics/sound/animation — tầng này không có)

- **Wire**: tối thiểu `[28][01]` (2B). Đuôi `P[2..end]` không cắt ở handler — hàm con tự parse.
- **Xử lý**: 1 lệnh gọi duy nhất. Không decode số, không banner, không ghi `player`, không chạm `gvar_007DA084/007DA1B0/007DA7BC`.
- Muốn biết format đuôi: live-capture frame `[28][01]...` + reverse body `005ab3f8` sau.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có gì để decode**: handler không literal (`DAT_00798xxx`/`UNK_`/sound/format), không cắt chuỗi đuôi ở tầng này. `redump/` không có `lit_7943xx.hex` và cũng không cần.
- Đuôi blob SubOp 01 chưa phân loại được (parser trong hàm chưa phục hồi + chưa có sample live).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:974-975`: `case 0x28: break;` — rỗng (kẹp giữa `0x27` và `0x29`).
- Kết luận: OP 0x28 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[28][01][blob...]        passthrough → func_005ab3f8(mgr 7DA238, RP), replay blob nguyên vẹn
[28][00] / [28][02..FF]  no-op (không hiệu ứng)
ĐỪNG GỬI: L=1 ([28] đơn độc) → RangeError BoundErr(0)
```

Tối thiểu 2B để qua guard; đuôi dài bao nhiêu do hàm con quyết. Test khói: `[28][01]` trần không crash/không banner; `[28][02]` im lặng tuyệt đối.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_036_007943BF_FUN_007943bf.c` | Handler chính (`if==1` + call dòng 28) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6124-6138` | Bản inline khớp 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x28]=0x24`) + `jumptable_dword200_0x78A9B6.hex` (`BF 43 79 00`) + `.csv:38` + `manifest.csv:38` | Mapping tự parse |
| 4 | `functions/0077eb9c/0077f098/0077eb1c/0077ee84/0077ef7c` | Đối chiếu — khẳng định không gọi |
| 5 | `functions/0077f414_FUN_0077F414.c:974-975` | C→S rỗng |
| 6 | grep `005ab3f8` (3 hit) + glob `005ab*.c` (vắng body) | Giới hạn passthrough |
