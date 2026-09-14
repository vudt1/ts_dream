# PHÂN TÍCH — Main OP 0x22 (34) / Case 30 / `FUN_00792ACC` @ `0x00792ACC`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `func_0x007a1218` (parser SubOp 2) **đã có body trong bản dump mới** (`index.csv:6534`, 793B) — phân tích ở §4.2.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Chỉ 2 nhánh (`if==1 / else if==2`, không switch/default).
- SubOp `0x01`: banner số — decode **Word LE** `W = P[2..3]`, dựng `msg = hằng + IntToStr(W) + hằng` (`UNK_007973e0` + số + `UNK_007985c0`, thứ tự trái/phải mức suy luận có ràng buộc), hiện banner 2000ms qua `TSe_TalkMsgFormPlus` (`gvar_007DA084`). Không ghi field player, không sound/light.
- SubOp `0x02`: **không còn passthrough mù** — `func_0x007a1218(obj = *gvar_007DA2B0 Tjo_Charge, text)` đã có body: text mang cấu trúc `[M:1B][W:2B LE][dư...]`; M=1/2 → dựng phiếu ngày-giờ (`StrToDate` trên chuỗi ngày tại `P[5..]`) + dòng tiền `IntToStr(W)` vào memo `obj+0x140`; M=3 → phiếu hạn mức với hằng `350` (`0x15E`). Kết luận cũ "sub-op duy nhất có thể mang text chưa phân loại encoding" **được đính chính**: chuỗi trên dây là **ngày dạng text parse bởi `StrToDate`**, không phải blob VISCII tự do.
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
| `0x02` | `[22][02][M:1B][W:2B LE][text...]` (tối thiểu 5B với M=1..3) | `_LStrLen(RP)`, `_LStrCopy(RP,2,len-1)` → `out = P[2..end]`; callee đọc `M=out[1]`, `W=out[2..3]` | `Tjo_Charge`: phiếu theo mode 1/2 (ngày+tiền) hoặc 3 (hạn mức 350) |
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

### 4.2. SubOp `0x02` — `Tjo_Charge`: phiếu ngày-giờ / phiếu hạn mức (**đã phục hồi từ body mới**)

- **Wire**: `[22][02][M:1B][W:2B LE][text...]` — `out = P[2..end]` do tầng case cắt; trong `out`: `M=out[1]` (mode), `W=out[2..3]` (Word LE, decode bằng `FUN_0077eb9c` ctx `gvar_007D9D30`), phần còn lại = text.
- **Đọc** (body `ts_decompile/functions/007a1218_FUN_007a1218.c`, 793B, `index.csv:6534`):
  - `FUN_007ba180(*(obj+0x140),0)` — **xóa sạch memo** (điều khiển list tại `obj+0x140`) trước mỗi phiếu (`:69`).
  - `_LStrCopy(out,2,2)` → `W = FUN_0077eb9c(*gvar_007D9D30, …)` (`:72-75`).
  - `M = out[1]` (guard `len(out)!=0` → `BoundErr(0)`) (`:79-84`).
- **Xử lý theo M**:
  - **M=1** (`:85-120`): `dayStr = out[4..end]` (`:88-96`) → `StrToDate` (`:98`), cộng hằng double `_DAT_007a153c` (`:99` — giá trị TDateTime, chưa đọc được trực tiếp), `DateToStr` chuẩn hóa (`:101`); thêm 3 dòng vào memo qua `FUN_007b8060(*(obj+0x140), …)`: dòng tiêu đề `&DAT_007a1548` (`:103`), dòng ngày `_LStrCatN(5)` các mảnh `{…, DAT_007a1574, ngày đã StrToDate, DAT_007a159c, ngày DateToStr, DAT_007a15a8}` (`:104-111` — thứ tự chính xác bị Ghidra làm mờ qua register, chỉ chắc bộ 5 mảnh), dòng tiền `_LStrCatN(3)` `{DAT_007a15b4, IntToStr(W), DAT_007a15d8}` (`:112-120`).
  - **M=2** (`:122-156`): y hệt M=1, chỉ khác tiêu đề `&DAT_007a15e8` (`:140`) → hai loại phiếu ngày (nghi "thu/chi" — chưa kết luận).
  - **M=3** (`:158-192`): không dùng text ngày; dựng phiếu **hạn mức** với hằng `0x15E = 350`: dòng tiêu đề `DAT_007a1624` (`:160`), dòng `_LStrCatN(3)` `{DAT_007a1664, IntToStr(350), DAT_007a15d8}` (`:161-169`), dòng `{DAT_007a15b4, IntToStr(W), DAT_007a15d8}` (`:170-178`), dòng `{DAT_007a16a4, IntToStr(350-W), DAT_007a15d8}` (`:179-192`, `_IntOver` khi W>350 chỉ là guard Delphi).
  - **M ∉ {1,2,3}**: chỉ xóa memo rồi thoát (`:69` + không nhánh nào khớp) — không crash.
  - **Kết**: `FUN_007b0094(*(obj+0x138),0)` refresh điều khiển thứ hai `obj+0x138` (`:195`).
- **Ý nghĩa**: `Tjo_Charge` hiển thị **biên lai nạp/cước phí**: mode 1/2 = phiếu "ngày + số tiền W" (hai nhãn khác nhau), mode 3 = phiếu "tổng hạn mức 350 / đã dùng W / còn lại 350−W". Nhãn từng dòng là hằng `.rodata` vùng `0x007A1548–0x007A16A4` — **chưa có dump** (xem §5).

---

## 5. Chuỗi VISCII → UTF-8

- **SubOp 01**: `W` là số; 2 mảnh là hằng `UNK_007973e0` + `UNK_007985c0`. `redump/` (kể cả batch mới) không có `lit_7973E0/7985C0.hex` → **chưa decode được**. Cần redump `.rodata` tại 2 địa chỉ rồi map cp1258/VISCII → UTF-8 NFC như OP 0x02.
- **SubOp 02 (đính chính)**: text trên dây (`P[5..end]` khi M=1/2) là **chuỗi ngày tháng đưa thẳng vào `StrToDate`** (`007a1218_FUN_007a1218.c:98,135`) → ASCII/format locale, không phải blob VISCII tự do. **Đính chính nhận định cũ**: encoding không còn "chưa phân loại được" với phần payload có ý nghĩa.
- **Hằng mới lộ từ body `007a1218`** (vùng `.text` ngay sau hàm, **chưa có dump** — bổ sung vào danh sách redump): `0x007A153C` (double), `0x007A1548`, `0x007A1574`, `0x007A159C`, `0x007A15A8`, `0x007A15B4`, `0x007A15D8`, `0x007A15E8`, `0x007A1624`, `0x007A1664`, `0x007A16A4` — toàn bộ nhãn phiếu `Tjo_Charge`.
- Không có codec XOR riêng ngoài XOR frame `0xAD` chung.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:964-965`: `case 0x22: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x22 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [22][01][W u16LE]   ; banner "nhãn + W + nhãn" 2000ms, đủ 4B payload
S→C [22][02][M u8][W u16LE][ngày ascii]  ; phiếu Tjo_Charge; M=1/2 cần chuỗi ngày, M=3 không cần
S→C [22][00]/[22][03+]  ; no-op, đừng gửi
S→C [22] (L=1)          ; CẤM — _BoundErr(0) RangeError
C→S [22]: KHÔNG TỒN TẠI
```

1. Thừa byte SubOp 1 bị bỏ qua; thiếu 1 byte → RangeError.
2. Test yên lặng → dùng SubOp 2 (không banner/sound/light; chỉ đổ memo form Charge nếu form đang mở); SubOp 1 luôn banner.
3. **Đóng được một phần tên nghiệp vụ SubOp 2**: form Charge hiển thị **biên lai theo ngày (M=1/2) hoặc hạn mức 350 (M=3)** với số tiền `W` — từ body mới, không còn là "form nhận chuỗi" mù. Nhãn cụ thể chờ dump hằng `0x7A15xx`.
4. Wire SubOp 2 tối thiểu 5B `[22][02][M][Wlo][Whi]`; M=1/2 cần thêm chuỗi ngày hợp lệ `StrToDate` — sai định dạng ngày → exception `ConvertError` của Delphi (có thể crash client test). **Đính chính ghi chú cũ** "`[22][02]` (out rỗng) vẫn gọi hàm an toàn": callee vẫn được gọi nhưng xóa memo xong rồi ném `_BoundErr(0)` khi đọc M (`007a1218_FUN_007a1218.c:69,79-84`) → frame `[22][02]` trần **không còn an toàn**; chỉ các `M∉{1,2,3}` kèm đủ 3 byte đầu là im lặng.
5. Cần thêm: (a) `lit_7973E0` + `lit_7985C0`, (b) ~~body `0x007A1218`~~ **đã có**, thay bằng: dump dải hằng `0x007A153C–0x007A16A4`.

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
| 8 | `functions/007a1218_FUN_007a1218.c:69-195` + `index.csv:6534` | **Mới**: body SubOp 2 — xóa memo `+0x140` (`:69`), Word W (`:72-75`), mode (`:84`), phiếu ngày (`:85-156`), phiếu 350 (`:158-192`), refresh `+0x138` (`:195`) |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] Redump dải hằng `0x007A153C–0x007A16A4` (11 nhãn phiếu Tjo_Charge, nằm trong `.text` ngay sau hàm — batch dump mới chưa phủ).
- [ ] `lit_7973E0` + `lit_7985C0` (banner SubOp 01) — vẫn thiếu.
- [ ] Thứ tự chính xác 5 mảnh `_LStrCatN` dòng ngày (`007a1218:104-109`) — Ghidra lẫn register, chưa kết luận được thứ tự trái/phải các nhãn `0x7A1574/159C/15A8`.
- [ ] Khác biệt nghiệp vụ M=1 vs M=2 (hai tiêu đề `0x7A1548` vs `0x7A15E8`) — chờ dump hằng.
