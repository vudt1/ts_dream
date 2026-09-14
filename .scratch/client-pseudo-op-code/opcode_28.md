# PHÂN TÍCH — Main OP 0x28 (40) / Case 36 / `FUN_007943BF` @ `0x007943BF`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler 61 dòng, 1 nhánh `if`, không switch, không codec. **`func_0x005ab3f8` đã có body trong bản dump mới** (`index.csv:6345`, 325B) → đuôi blob **đã đặc tả**: mảng bản ghi 4 byte `[type][W lo][W hi][b]`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Cổng **passthrough 1 nhánh** — không phải kênh số ở tầng handler (không DWORD/Word/double, không `IntToStr`), không banner/chat-log.
- Chỉ **01 SubOp hoạt động: `0x01`**: trao nguyên `RP` (gồm cả byte SubOp) cho `func_0x005ab3f8(*gvar_007DA238, RP)`. **Parser đã bóc từ body mới (§4)**: ĐUÔI KHÔNG PHẢI BLOB MÙ — là **mảng bản ghi 4 byte `[type u8][W u16LE][b u8]`**, dispatch `type==1 → FUN_005ABCF0`, `type==2 → FUN_005AB678` (cả hai đã có body, thao tác bảng đối tượng + player inventory → gợi ý kênh thêm/bớt item — chưa kết luận nghiệp vụ).
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
| `0x01` | `[28][01]([type u8][W u16LE][b u8])×N` (4B/bản ghi, `N = (len(RP)-1)/4 ≤ 255`) | `SubOp=RP[0]`, guard rỗng | `func_0x005ab3f8(*gvar_007DA238, RP)` — **đã có body**: lặp bản ghi 4B, dispatch type 1/2 (§4) |
| `0x00`,`0x02–0xFF` | `[28][xx]` | cùng guard | no-op |

---

## 4. Chi tiết SubOp `0x01` — Passthrough duy nhất (bỏ graphics/sound/animation — tầng này không có)

### 4.1. Body `func_0x005ab3f8` (325B, `index.csv:6345`, chữ ký `__register (EAX=mgr, EDX=RP)`)

- `N = (len(RP)-1) / 4` (số bản ghi nguyên; `N > 0xFF` → `_BoundErr`) (`005ab3f8_FUN_005ab3f8.c:45-58`). Con chạy Delphi-index bắt đầu `2` (= `RP[1]` 0-based, sau byte SubOp).
- Mỗi bản ghi 4 byte tại `RP[1+4k .. 4+4k]` (0-based):
  - `type = RP[1+4k]` (`:63-69`),
  - `W = Word LE` tại `RP[2+4k..3+4k]` decode `FUN_0077eb9c(*gvar_007D9D30, …)` (`:71-77`),
  - `b = RP[4+4k]` (`:78-88`),
  - mỗi deref có guard `len` riêng → **bản ghi cụt giữa chừng = BoundErr (crash)**, không phải bỏ qua.
- Dispatch (`:89-99`): `type==1` → `FUN_005ABCF0(mgr, b, W, 0)`; `type==2` → `FUN_005AB678(mgr, b, W, 0)`; **type khác → record bị bỏ, lặp tiếp** (không default-error).
- `+0xFB` overflow-check của con chạy (`:100-104`) là guard Delphi, không phải logic.

### 4.2. Hai đích dispatch (đã có body từ trước, nay nối được chuỗi phân tích)

- `FUN_005ABCF0(int, uint b, uint W, char 0)` (`005abcf0_FUN_005abcf0.c:34`) và `FUN_005AB678(int, uint b, uint W, char 0)` (`005ab678_FUN_005ab678.c:48`) đều đọc/ghi **kho/ô đối tượng của player**: `*gvar_007DA540`/`*gvar_007DA554` (lookup `FUN_00774B50`/`FUN_00759B04` theo id 16-bit), `player+0x145C` (đếm), `player+0x9E0`/`+0x509` (bảng slot) → **mạnh dạn xếp đây là kênh "thêm/bớt vật phẩm theo danh sách"** — nhãn 1=thêm, 2=bớt là **suy đoán chưa có bằng chứng gọi tên**, chưa kết luận được.
- Handler tầng case **không** touch `gvar_007DA084/007DA1B0/007DA7BC` — không banner/chat/player ở tầng này (giữ nguyên như cũ).

---

## 5. Chuỗi VISCII → UTF-8

- **Không có gì để decode**: handler không literal; **body mới `005ab3f8` cũng không lộ hằng chuỗi nào** (chỉ codec số `eb9c`) → kết luận cũ giữ nguyên nhưng nay đã kiểm chứng tận callee, không còn "chưa loại trừ".

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:974-975`: `case 0x28: break;` — rỗng (kẹp giữa `0x27` và `0x29`).
- Kết luận: OP 0x28 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[28][01]([type][W u16LE][b])×N   ; type∈{1,2} hợp lệ; 4B/bản ghi — N = (len(RP)-1)/4
[28][00] / [28][02..FF]           ; no-op (không hiệu ứng)
ĐỪNG GỬI: L=1 ([28] đơn độc) → RangeError BoundErr(0)
```

1. Tối thiểu 2B để qua guard tầng case; **đính chính**: `[28][01]` trần giờ là **an toàn** (N=0 → không lặp, `005ab3f8.c:59`).
2. Bản ghi phải **đủ 4 byte** — thiếu byte giữa bản ghi → `_BoundErr` bên callee (crash); `N > 255` → BoundErr (`:55-57`).
3. `type∉{1,2}` = record bị bỏ — dùng làm ignore-path test. Đuôi dài bao nhiêu: bội số 4 byte.
4. Test khói: `[28][01][01][00 00][00]` (type=1, W=0, b=0) chạm `FUN_005ABCF0` — chỉ nên gửi khi có manager `7DA238` khởi tạo đầy đủ; còn `[28][01][03 00 00 00]` (type lạ) im lặng tuyệt đối.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_036_007943BF_FUN_007943bf.c` | Handler chính (`if==1` + call dòng 28) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6124-6138` | Bản inline khớp 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x28]=0x24`) + `jumptable_dword200_0x78A9B6.hex` (`BF 43 79 00`) + `.csv:38` + `manifest.csv:38` | Mapping tự parse |
| 4 | `functions/0077eb9c/0077f098/0077eb1c/0077ee84/0077ef7c` | Tầng case: khẳng định không gọi; **callee mới gọi `eb9c`** (Word codec, `005ab3f8.c:77`) |
| 5 | `functions/0077f414_FUN_0077F414.c:974-975` | C→S rỗng |
| 6 | ~~grep `005ab3f8` vắng body~~ → `functions/005ab3f8_FUN_005ab3f8.c:45-107` + `index.csv:6345` | **Mới**: body parser — lặp bản ghi 4B + dispatch |
| 7 | `functions/005abcf0_FUN_005abcf0.c:34,103-133` + `functions/005ab678_FUN_005ab678.c:48,142-224` | **Mới**: hai đích dispatch (kho/ô đối tượng qua `gvar_007DA540/554`, `player+0x145C/0x9E0/0x509`) |

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] Nghiệp vụ `type` 1 vs 2 (thêm vs bớt? — suy đoán từ field kho, chưa có tên symbol).
- [ ] Ý nghĩa `b` (byte 1) và `W` (id 16-bit?) trong từng record — cần đọc sâu `005ABCF0/005AB678` hoặc sample live.
- [ ] Định danh `*gvar_007DA238` (manager nhận) — chưa thấy gán khởi tạo trong export.
