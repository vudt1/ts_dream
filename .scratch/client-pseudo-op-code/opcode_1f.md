# PHÂN TÍCH — Main OP 0x1F (31) / Case 27 / `FUN_007922E6` @ `0x007922E6`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Các `func_0x0077xxxx` nhận ủy thác chưa có body — ghi rõ giới hạn.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Server push trạng thái/notice/target: dialog 3 chế độ (SubOp `0x01`), lock-target/select-ID có điều kiện hủy (SubOp `0x02`), set byte vào manager `*gvar_007DA694` (SubOp `0x03/04/05/0B`), blob biến dài cho 2 manager (SubOp `0x06` → `007DA694`, `0x0E` → `007DA7C0`), show/clear cờ form (SubOp `0x07–0x0C`), Toast tĩnh (SubOp `0x0D`).
- **14 nhánh**: `0x01..0x0E` (1–14), không có default — SubOp lạ bị bỏ qua.
- Text hiển thị đều là **tham chiếu chuỗi tĩnh** (`UNK_0079833c/70/a0/e4`), không có payload text VISCII trên dây ở tầng này.
- Chiều C→S `case 0x1f: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x1F (31) → byte_table[0x78A8EE][0x1F] = 0x1B (27)
                 → dword_table[0x78A9B6][27] @ 0x0078AA22 = 0x007922E6
                 → FUN_007922e6 (Case 27)
```

- File chính: `ts_decompile/case_functions/functions/case_027_007922E6_FUN_007922e6.c` (250 dòng); đối chiếu bản inline `functions/0078a89c_FUN_0078a89c.c:4971-5178` (`case 0x1f:`).
- Manifest: `case_functions/manifest.csv:29` + `jumptable_0x78A9B6_case_functions.csv:29`.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x1F`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).
- `*(EBP-0x0c)` = con trỏ RestPayload; độ dài tại `*(ptr-4)`; `RP[0]` = SubOp.

### 2.3. Đọc SubOp (dòng 31-38)

```c
SubOp = (uint)*(byte*)(RestPayload + 0);  // RP[0] = P[1]
switch(SubOp){ case 1:..; ... case 0xE:..; }
```

### 2.4. Codec

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | **Có gọi** — decode `id` DWORD LE cho SubOp `0x02` |
| `FUN_0077eb9c` | Không gọi trong Case 27; là đối tác decode Length framing + Word chung |
| `FUN_0077eb1c` / `FUN_0077ee84` | Không gọi trong Case 27; chỉ dùng chiều C→S |
| `FUN_0077f098` | Không gọi trong Case 27 (call-sites ở vùng `00792xxx`, case 28+) |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[1F][01][mode:1B]` (3B) | `mode = RP[1]`, guard `len>=2` | Dialog 3 chế độ (xem 4.1) |
| `0x02` | `[1F][02][id:4B LE][flag:1B]` (7B) | `_LStrCopy(RP,2,4)`+`FUN_0077ef7c`→id, `RP[5]`=flag (guard `len>=6` khi `id<1`) | Lock-target / hủy chọn (xem 4.2) |
| `0x03` | `[1F][03][v:1B](+[pad:1B])` | `v=RP[1]`, guard `len>=3` nhưng chỉ đọc `RP[1]` | `func_0x0077a55c(manager694, v)` |
| `0x04` | `[1F][04][v:1B]` | `v=RP[1]`, guard `len>=2` | `func_0x0077b0e4(manager694, v)` |
| `0x05` | `[1F][05][v:1B](+[pad:1B])` | `v=RP[1]`, guard `len>=3` | `func_0x0077abd0(manager694, v)` |
| `0x06` | `[1F][06][data...]` (biến dài) | passthrough nguyên `RP` | `func_0x0077b128(manager694, RP)` |
| `0x07` | `[1F][07]` (2B) | không đọc | Show `*gvar_007DA504` (VMT+0x20) |
| `0x08` | `[1F][08]` | không đọc | Clear `*(007DA504+0x129)=0` + `*(007DA694+4)=0` |
| `0x09` | `[1F][09]` | không đọc | Clear `*(007DA504+0x12a)=0` |
| `0x0A` | `[1F][0A]` | không đọc | Set `*(007DA788+0x144)=1`, `+0x145=1`, `*(007DA37C+0x0c)=6` |
| `0x0B` | `[1F][0B][v:1B]` | `v=RP[1]`, guard `len>=2` | `func_0x0077b3c0(manager694, manager694+5, v)` |
| `0x0C` | `[1F][0C]` | không đọc wire; đọc state `idx=*(007DA504+0xf8)` | Clear như 0x08+0x09 rồi resync selection index (xem 4.12) |
| `0x0D` | `[1F][0D]` | không đọc | Toast `UNK_007983e4` 2000ms |
| `0x0E` | `[1F][0E][data...]` (biến dài) | passthrough nguyên `RP` | `func_0x0077bd2c(manager7C0, RP)` |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Dialog 3 chế độ

- **Wire**: `[1F][01][mode 1B]`. **Đọc**: `mode = (char)RP[1]`.
- **Xử lý**:
  1. `FUN_007ba180(*(gvar_007DA788+0x130), '\0')` — xóa text cũ (UI).
  2. `mode==0`: set text tĩnh `UNK_0079833c`, không show.
  3. `mode==1`: set text `UNK_00798370` + show modal `(**gvar_007DA788+0x20)()`.
  4. `mode==2`: set text `UNK_007983a0` + set cờ `*(gvar_007DA5A0+0xa096)=1` (giữ lại — đây là state).
  5. `mode` khác: không làm gì.

### 4.2. SubOp `0x02` — Lock-target / select-ID có điều kiện hủy

- **Wire**: `[1F][02][id 4B LE][flag 1B]` = 7 bytes dây (6 bytes RP).
- **Đọc**: `id = FUN_0077ef7c(_LStrCopy(RP,2,4))`, ghi `*(gvar_007D9D44+0x148) = id`; nếu `id<1` đọc thêm `flag = RP[5]` (guard `len>=6`).
- **Xử lý**:
  - `id<1`: `flag==0` → clear text + set `UNK_007983a0` + show modal `007DA788` (nhánh hủy chọn); `flag!=0` → show `(**gvar_007D9D44+0x20)()`.
  - `id>=1` (nhánh giữ chọn, nhưng 5 điều kiện ép về 0): slot type `0x03` qua `FUN_00634f3c`, `*(short*)(player+0x63a)==10991 (0x2AEF)` hoặc `==-15627`, `FUN_00504c50(map)!=0`, `*(player+0x1458)!=0` → `id=0`. Cuối cùng luôn show `(**gvar_007D9D44+0x20)()`.

### 4.3–4.5. SubOp `0x03/04/05` — Set 1 byte vào manager `007DA694`

- Cùng pattern `(manager, 1 byte)`, body nằm trong hàm con chưa phục hồi tên:
  - `0x03`: `func_0x0077a55c(manager, v)` — guard `len>=3` nhưng chỉ đọc `RP[1]`, byte thứ 3 (nếu có) là padding/không dùng.
  - `0x04`: `func_0x0077b0e4(manager, v)` — guard `len>=2`.
  - `0x05`: `func_0x0077abd0(manager, v)` — guard `len>=3`, byte cuối padding.
- Không có logic nội tại khác ở tầng Case 27.

### 4.6. SubOp `0x06` — Blob cho manager `007DA694`

- **Wire**: `[1F][06][data...]` biến dài, passthrough nguyên `RP` cho `func_0x0077b128`. Parser nằm trong hàm con.

### 4.7. SubOp `0x07` — Show object `007DA504`

- **Wire**: `[1F][07]`. `(***gvar_007DA504 + 0x20)()`.

### 4.8. SubOp `0x08` — Clear 2 cờ

- **Wire**: `[1F][08]`. `*(007DA504+0x129)=0; *(007DA694+4)=0`.

### 4.9. SubOp `0x09` — Clear 1 cờ

- **Wire**: `[1F][09]`. `*(007DA504+0x12a)=0`.

### 4.10. SubOp `0x0A` — Set bộ 3 cờ

- **Wire**: `[1F][0A]`. `*(007DA788+0x144)=1; *(007DA788+0x145)=1; *(007DA37C+0x0c)=6`.

### 4.11. SubOp `0x0B` — Set byte có base+5

- **Wire**: `[1F][0B][v 1B]`. `func_0x0077b3c0(manager694, manager694+5, v)`.

### 4.12. SubOp `0x0C` — Resync selection index

- **Wire**: `[1F][0C]`, không đọc wire thêm; đọc state nội `idx = *(byte*)(*(007DA504)+0xf8)`.
- **Xử lý**: clear như 0x08+0x09, rồi nếu `idx!=0` (guard `idx>4 → BoundErr`): copy con trỏ resync `player+0x57c+idx*4` vào `*(007DA32C+0x294)` (nếu `*(007DA32C+0x298)!=0`) và vào `*(007D9E5C+0x3bc)` (nếu `*(007D9E5C+0x4b4)!=0`). Phần gọi VMT vẽ (`+0x290`/`+0x3c0`) bỏ qua theo yêu cầu — giữ phép resync con trỏ.

### 4.13. SubOp `0x0D` — Toast 2000ms

- **Wire**: `[1F][0D]`. `(**gvar_007DA084+0x90)(..., &UNK_007983e4, 2000, 0, 0)` — cùng pattern OP 0x01 SubOp 05/06/07.

### 4.14. SubOp `0x0E` — Blob cho manager `007DA7C0`

- **Wire**: `[1F][0E][data...]` biến dài, passthrough `func_0x0077bd2c(manager7C0, RP)` (cùng họ `0x0077bxxx` với OP 0x1E SubOp 0x0C/0x0D).

---

## 5. Chuỗi VISCII → UTF-8

- Case 27 **không chứa payload text trên dây** — mọi text là chuỗi tĩnh: `UNK_0079833c` (SubOp1/mode0), `UNK_00798370` (mode1), `UNK_007983a0` (mode2 + SubOp2/hủy), `UNK_007983e4` (SubOp 0x0D Toast).
- `ts_decompile/redump/` hiện **không có dump** cho 4 địa chỉ này → không decode VISCII được từ source cho phép. Cần redump `.rodata` tại `0x0079833C/70/A0/E4`.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:958-959`: `case 0x1f: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x1F là S→C thuần. Không có format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [1F][01][mode 00|01|02]
S→C [1F][02][id u32LE][flag u8]   ; id<1 + flag==0 → dialog hủy; còn lại show 007D9D44
S→C [1F][03][u8](+1 pad)          ; cần đủ 3B RP mới qua guard BoundErr(2)
S→C [1F][04][u8]
S→C [1F][05][u8](+1 pad)          ; cần đủ 3B RP
S→C [1F][06][blob...]
S→C [1F][07] / [08] / [09] / [0A] / [0C] / [0D]   ; no param
S→C [1F][0B][u8]
S→C [1F][0E][blob...]
C→S [1F]: KHÔNG TỒN TẠI
```

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_027_007922E6_FUN_007922e6.c` | Handler chính, 14 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:4971-5178` | Bản inline đối chiếu |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` + `manifest.csv:29` | Mapping |
| 4 | `functions/0077ef7c*` | Decode id SubOp 0x02 |
| 5 | `functions/0077f414_FUN_0077F414.c:958-959` | C→S rỗng |
