# PHÂN TÍCH — Main OP 0x22 (34) / Case 30 / `FUN_00792ACC` @ `0x00792ACC`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `func_0x007a1218` (parser SubOp 2) chưa có body — ghi rõ giới hạn.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Chỉ 2 nhánh (`if==1 / else if==2`, không switch/default).
- SubOp `0x01`: banner số — decode **Word LE** `W = P[2..3]`, dựng `msg = hằng + IntToStr(W) + hằng` (`UNK_007973e0` + số + `UNK_007985c0`, thứ tự trái/phải mức suy luận có ràng buộc), hiện banner 2000ms qua `TSe_TalkMsgFormPlus` (`gvar_007DA084`). Không ghi field player, không sound/light.
- SubOp `0x02`: passthrough **chuỗi biến dài** `P[2..end]` (có thể rỗng) cho form `Tjo_Charge` (`*gvar_007DA2B0`) qua `func_0x007a1218(obj, text)` — body chưa phục hồi nên đây là sub-op duy nhất có thể mang text mà chưa phân loại được encoding.
- Chiều C→S `case 0x22: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x22 (34) → byte_table[0x78A8EE][0x22] = 0x1E (30)
                 → dword_table[0x78A9B6][30] @ 0x0078AA2E = 0x00792ACC
                 → FUN_00792acc (Case 30)
```

- File chính: `ts_decompile/case_functions/functions/case_030_00792ACC_FUN_00792acc.c` (92 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5275-5305` — khớp 1:1 (bóc được varargs `CatN` vs `banner`).
- Framing/XOR/pump như `opcode_00_01.md` §2. `.asm.txt` dispatcher chỉ còn prologue nên mapping xác nhận bằng 2 file `.hex`/`.csv` + manifest + bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x22`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 29-35)

```c
if (*(RP-4)==0) _BoundErr(0);       // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);     // RP[0] = P[1]
if (SubOp==1) {...} else if (SubOp==2) {...}
// 0x00, >=0x03: no-op (chỉ epilogue _LStrArrayClr/_LStrClr cuối hàm)
```

Helper gọi trong Case 30: **chỉ `FUN_0077eb9c`** (SubOp 1). `0077ef7c/0077eb1c/0077ee84/0077f098` không được gọi ở đây.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[22][01][W:2B LE]` (4B) | `_LStrCopy(RP,2,2)` + `FUN_0077eb9c` → `W & 0xFFFF` | Banner `hằng + IntToStr(W) + hằng` 2000ms |
| `0x02` | `[22][02][text...]` (biến dài, tối thiểu 2B) | `_LStrLen(RP)`, `_LStrCopy(RP,2,len-1)` → `out = P[2..end]` | Passthrough cho `Tjo_Charge` |
| `0x00`,`≥0x03` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Banner số Word + 2 nhãn tĩnh

- **Wire**: `P[0]=0x22, P[1]=0x01, P[2..3]=W WORD LE` (`b0+b1*256`).
- **Đọc**: `_LStrCopy(RP,2,2,&tmp)` → `W = FUN_0077eb9c(...) & 0xFFFF` (thiếu byte → `_BoundErr` trong helper; thừa byte bị cắt).
- **Xử lý**:
  1. `s = IntToStr(W)` — mảnh giữa.
  2. `msg = _LStrCatN(3)` với tập `{UNK_007973e0, s, UNK_007985c0}` (Ghidra mất varargs nên 3 con trỏ hiện lẫn vào args banner; bản inline chỉ còn `CatN(&msg,3)` + `VMT+0x90(form,msg,2000)`). Thứ tự khả dĩ nhất `0x7973e0 + số + 0x7985c0` theo khuôn `opcode_1a` (listing asm PUSH ngược) và `case_018` (cùng dùng `0x7973e0` kẹp quanh `IntToStr`) — mức suy luận có ràng buộc, địa chỉ chắc chắn.
  3. Banner 2000ms qua `gvar_007DA084 = TSe_TalkMsgFormPlus` (gán tại `0051189c:1265-1266`). Đây là banner/marquee (`FUN_0063c84c → FUN_007bc1c8`), không phải log chat — nguồn hiểu nhầm "Talk" như đã chứng minh ở `opcode_1a.md` §0.
- Không ghi player, không sound/light. Thuần hiển thị.

### 4.2. SubOp `0x02` — Passthrough chuỗi cho `Tjo_Charge`

- **Wire**: `[22][02][text...]` — mọi byte sau `P[2]` đều thuộc `out`; `[22][02]` (RP len=1) → chuỗi rỗng vẫn gọi hàm.
- **Đọc**: `len=_LStrLen(RP)`, `cnt=len-1`, `_LStrCopy(RP,2,cnt,&out)`. Không codec.
- **Xử lý**: `func_0x007a1218(*gvar_007DA2B0, out)` với `gvar_007DA2B0 = Tjo_Charge` (gán tại `0051189c:1463-1464` qua `VMT_79D22C`). Hàm này không có file `007a1218*`, `index.csv` không entry — mọi parse nằm trong method đó. Không banner/sound, không ghi player ở tầng handler.

---

## 5. Chuỗi VISCII → UTF-8

- **SubOp 01**: `W` là số; 2 mảnh là hằng `UNK_007973e0` + `UNK_007985c0`. `redump/` không có `lit_7973E0/7985C0.hex` (grep chỉ thấy 2 địa chỉ này tại case_030 + case_018 dùng chung `0x7973e0`) → **chưa decode được**. Cần redump `.rodata` tại 2 địa chỉ rồi map cp1258/VISCII → UTF-8 NFC như OP 0x02.
- **SubOp 02**: text trên dây (`P[2..end]`), encoding phụ thuộc body `0x007A1218` chưa có → không kết luận VISCII hay raw byte.
- Không có codec XOR riêng ngoài XOR frame `0xAD` chung.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:964-965`: `case 0x22: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x22 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [22][01][W u16LE]   ; banner "nhãn + W + nhãn" 2000ms, đủ 4B payload
S→C [22][02][bytes...]  ; passthrough Tjo_Charge, có thể rỗng ([22][02])
S→C [22][00]/[22][03+]  ; no-op, đừng gửi
S→C [22] (L=1)          ; CẤM — _BoundErr(0) RangeError
C→S [22]: KHÔNG TỒN TẠI
```

1. Thừa byte SubOp 1 bị bỏ qua; thiếu 1 byte → RangeError.
2. Test yên lặng → dùng SubOp 2 (không banner/sound/light); SubOp 1 luôn banner.
3. Chưa đặt tên nghiệp vụ cho SubOp 2 (`Tjo_Charge` gợi ý nạp/thanh toán nhưng body chưa có — chỉ dừng ở "form Charge nhận chuỗi").
4. Cần thêm: (a) `lit_7973E0` + `lit_7985C0`, (b) body `0x007A1218`.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_030_00792ACC_FUN_00792acc.c` | Handler chính, 2 branch |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5275-5305` | Bản inline: bóc varargs `CatN` vs `banner` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x22]=0x1E`) + `jumptable_0x78A9B6_case_functions.csv:32` + `manifest.csv:32` | Mapping |
| 4 | `functions/0077eb9c_FUN_0077eb9c.c:160-180` | Codec Word LE SubOp 1 |
| 5 | `functions/0051189c_FUN_0051189c.c:1265-1266` + `:1463-1464` | Định danh `TSe_TalkMsgFormPlus` + `Tjo_Charge` |
| 6 | `case_018...c:265-278` | Đối chứng khuôn `0x7973e0 + IntToStr + hằng` |
| 7 | `functions/0077f414_FUN_0077F414.c:964-965` | C→S rỗng |
