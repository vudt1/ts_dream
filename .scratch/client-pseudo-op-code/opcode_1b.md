# PHÂN TÍCH — Main OP 0x1B (27) / Case 24 / `FUN_00791D80` @ `0x00791D80`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Không suy đoán nội dung chuỗi.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh thông báo văn bản tĩnh của server. Server chọn **nhóm văn bản** bằng byte ngoài (Outer = `0x01` / `0x02`), chọn **câu cụ thể** bằng byte trong (Inner = `0x00/01/02/03/04/06`), client hiện **Toast chữ chạy 2000ms** qua `(**gvar_007DA084 + 0x90)(..., 2000, 0, 0)` — cùng hàm Toast đã xác định ở OP 0x01 SubOp `0x05/06/07`.
- **Nhánh riêng**: Outer `0x03` không hiện Toast mà gọi 1 phương thức ảo không tham số `(**gvar_007DA720 + 0x20)()` — kích hoạt/refresh 1 form (không rõ tên form từ source hiện có, chỉ ghi nhận hành vi).
- **Không có**: ghi state game, cộng/trừ số liệu, vòng lặp, struct, gửi ACK, disconnect.
- **Hướng**: một chiều S→C. Chiều C→S `case 0x1b:` trong `FUN_0077F414` rỗng hoàn toàn → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1B (27) → byte_table[0x78A8EE][0x1B] = 0x18 (24)
                 → dword_table[0x78A9B6][24] @ 0x0078AA16 = 0x00791D80
                 → FUN_00791d80 (Case 24)
```

- File chính: `ts_decompile/case_functions/functions/case_024_00791D80_FUN_00791d80.c`
- Manifest: `ts_decompile/case_functions/manifest.csv` dòng 26 (`24,0x0078AA16,0x00791D80,...`)
- Dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt` — 2 lệnh `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Framing/XOR/tick-pump giữ nguyên như `opcode_00_01.md` §2 (token `F4 44`, Length Word LE qua `FUN_0077eb9c`, XOR `0xAD` toàn frame, `MainOp = payload[0]`, `RestPayload = _LStrCopy(payload,2,len-1)` vào `ECX`).

### 2.2. Quy ước ký hiệu

- `P[i]` = byte thứ i của payload sau deframe (`P[0] = 0x1B`).
- `RP[i]` = byte thứ i của RestPayload (`RP[i] = P[i+1]`), 0-based trong C.
- Delphi `_LStrCopy` là 1-based, nhưng hàm này **không dùng `_LStrCopy`** — đọc byte trực tiếp `*(byte*)(ptr+offset)`.

### 2.3. Cách đọc SubOp 2 tầng (khác OP 0x00/0x01)

```c
// Tầng 1 — Outer (dòng 24-32):
RestPayload = *(EBP-0x0C);                    // ECX lúc entry
if (*(RestPayload-4) == 0) _BoundErr(0);      // RP rỗng → RangeError
Outer = *(byte*)(RestPayload + 0);            // RP[0] = P[1]
if (Outer==1) {...} else if (Outer==2) {...} else if (Outer==3) {...}
// Outer khác: rơi xuống cleanup, bỏ gói

// Tầng 2 — Inner (lặp lại trong nhánh 1 và 2):
if (len(RestPayload) < 2) _BoundErr(1);
Inner = *(byte*)(RestPayload + 1);            // RP[1] = P[2]
switch(Inner){ case 0:..; case 1:..; case 2:..; case 3:..; case 4:..; case 6:..; }
// KHÔNG có case 5, KHÔNG có default

// Điều kiện hiển thị:
if (*(char*)(RestPayload+1) != 0)
  (**gvar_007DA084+0x90)(gvar_007DA084, msg_slot, 2000, 0, 0); // Toast 2000ms
```

- **Không gọi bất kỳ helper codec nào** (`0077eb9c/0077ef7c/0077eb1c/0077ee84/0077f098`). Chỉ `_LStrLAsg` (gán chuỗi literal) + Toast + `_BoundErr`/cleanup.
- Phần cuối hàm (dòng 109–137) chỉ là `_LStrArrayClr/_LStrClr` — epilogue Delphi, không phải nghiệp vụ.

---

## 3. Bảng tổng hợp SubOp

| Outer | Inner | Wire (payload) | Chuỗi literal | Toast? | Ghi chú |
| :---: | :---: | :--- | :--- | :---: | :--- |
| `0x01` | `0x00` | `[1B][01][00]` | `@0x798118` | KHÔNG (Inner==0) | Chỉ gán, không hiện |
| `0x01` | `0x01` | `[1B][01][01]` | `@0x798138` | CÓ 2000ms | Riêng nhóm 1 |
| `0x01` | `0x02` | `[1B][01][02]` | `@0x798160` | CÓ | Riêng nhóm 1 |
| `0x01` | `0x03` | `[1B][01][03]` | `@0x798190` | CÓ | Riêng nhóm 1 |
| `0x01` | `0x04` | `[1B][01][04]` | `@0x7981C4` | CÓ | Chung 2 nhóm |
| `0x01` | `0x06` | `[1B][01][06]` | `@0x7981E8` | CÓ | Chung 2 nhóm |
| `0x02` | `0x00` | `[1B][02][00]` | `@0x798118` | KHÔNG | Chỉ gán |
| `0x02` | `0x01` | `[1B][02][01]` | `@0x798210` | CÓ | Riêng nhóm 2 |
| `0x02` | `0x02` | `[1B][02][02]` | `@0x798238` | CÓ | Riêng nhóm 2 |
| `0x02` | `0x03` | `[1B][02][03]` | `@0x798278` | CÓ | Riêng nhóm 2 |
| `0x02` | `0x04` | `[1B][02][04]` | `@0x7981C4` | CÓ | Chung |
| `0x02` | `0x06` | `[1B][02][06]` | `@0x7981E8` | CÓ | Chung |
| `0x03` | — | `[1B][03]` | — | KHÔNG | Gọi `(**gvar_007DA720+0x20)()` |
| khác | — | — | — | — | Bỏ qua |

- Inner `0x05` và `>=0x07`: **không gán chuỗi mới** (giữ giá trị cũ của slot) nhưng **vẫn hiện Toast** vì `Inner != 0` — hành vi nguyên bản của code, mock giữ nguyên.
- 2 entry biên (`0x00/0x04/0x06`) dùng chung literal giữa 2 nhóm; 3 entry giữa (`0x01/02/03`) phân biệt nhóm.

---

## 4. Chi tiết từng nhánh

### 4.1. Outer `0x01` — Toast nhóm 1

- **Wire**: `[1B][01][Inner 1B]` — payload 3 bytes, RestPayload 2 bytes `[01][Inner]`.
- **Đọc**: `Outer = RP[0]`, `Inner = RP[1]` (yêu cầu `len(RP) >= 2` nếu không `_BoundErr(1)`).
- **Xử lý**: `switch(Inner)` gán literal vào slot `EBP-0x2C` qua `_LStrLAsg`, rồi `if (Inner != 0) Toast(slot, 2000ms)`. Không ghi state, không vòng lặp.

### 4.2. Outer `0x02` — Toast nhóm 2

- **Wire**: `[1B][02][Inner 1B]` — cấu trúc byte y hệt 4.1, chỉ khác 3 literal giữa (`0x798210/0x798238/0x798278` thay cho `0x798138/0x798160/0x798190`).
- **Đọc & xử lý**: y hệt 4.1.

### 4.3. Outer `0x03` — Kích hoạt form

- **Wire**: `[1B][03]` — payload 2 bytes, RestPayload 1 byte, không có Inner.
- **Đọc**: chỉ `RP[0]==3`, không đọc thêm byte nào.
- **Xử lý**: duy nhất `(***gvar_007DA720 + 0x20)()`. Không Toast, không chuỗi, không tham số. `gvar_007DA720` ngoài chỗ này chỉ xuất hiện ở code khởi tạo form (`0051189c`, `FUN_007b0094 +0xE4`) nên chỉ ghi nhận "kích hoạt/refresh 1 form".

### 4.4. Outer khác — bỏ qua, chỉ cleanup chuỗi.

---

## 5. Chuỗi VISCII → UTF-8

- Handler tham chiếu **9 literal**: `0x798118, 0x798138, 0x798160, 0x798190, 0x7981C4, 0x7981E8, 0x798210, 0x798238, 0x798278`.
- **Không decode được từ source cho phép**: grep toàn `ts_decompile/` chỉ trúng code (không có bytes), thư mục `ts_decompile/redump/` không có `lit_7981*.hex` hay dump `.rodata` vùng `0x7981xx`.
- Ghi trong mock: `nội dung @0x7981xx chờ redump vùng rodata 0x00798118–0x00798278 từ aLogin.exe`. Không suy đoán nội dung.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:941-942`: `case 0x1b: break;` — rỗng, không `_LStrCat`, không `CY_AddSedQueue`.
- Kết luận: client không bao giờ gửi OP 0x1B. Mock server không cần parse C→S cho OP này.

---

## 7. Ghi chú cho Mock Server

1. OP 0x1B là downlink-only. Frame: `[F4 44][L:Word LE][payload]`, XOR `0xAD` toàn frame, `L = len(payload)`.
2. Gói an toàn để replay: `[1B][01][01..04,06]`, `[1B][02][01..04,06]`, `[1B][03]`. Gói `[1B][01/02][00]` không hiện gì (chỉ gán).
3. Không gửi payload chỉ có opcode (`L=1`, RP rỗng → `_BoundErr(0)` RangeError).
4. Toast hiển thị được nhưng nội dung text chưa khôi phục — cần redump `.rodata` mới điền được câu chữ Việt.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `ts_decompile/case_functions/functions/case_024_00791D80_FUN_00791d80.c` | Handler chính, toàn bộ switch 2 tầng |
| 2 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `case_functions/manifest.csv:26` | Mapping MainOp→Case→Target |
| 3 | `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt` | Dispatcher 2 lệnh tra bảng |
| 4 | `ts_decompile/functions/0077f414_FUN_0077F414.c:941-942` | C→S rỗng |
| 5 | `opcode_00_01.md` §2, §4.3 | Framing/XOR/pump + định danh hàm Toast 2000ms |
