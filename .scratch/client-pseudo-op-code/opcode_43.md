# PHÂN TÍCH — Main OP 0x43 (67) / Case 60 / FUN_00795f48 @ 0x00795F48

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only) — handler chính **chưa được decompile** (khe hở nhị phân, xem mục 5).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x43` (thập phân: `67`, ánh xạ tới **Case 60**) là **opcode đơn SubOp** — case duy nhất là `SubOp 0x01`, chuyển tiếp toàn bộ Rest Payload cho **một hàm xử lý của form "Giới Thiệu Người Chơi" (Recommend / Referee form)**:

- Đối tượng tiếp nhận: `gvar_007D9F58` — form thuộc cụm `TJC_Referee` (VMT `0x51EF94`, constructor `FUN_0051f074` tạo tại `ts_decompile/functions/0051189c_FUN_0051189c.c:1415-1416`).
- Handler: `func_0x0051f868` @ `0x0051F868` — **chưa có file decompile** trong `ts_decompile/functions/` (nằm trong khoảng trống nhị phân giữa `FUN_0051f5f4` kết thúc `0x0051F7C0+` và `FUN_0051fb6c` @ `0x0051FB6C`; kiểm chứng qua `ts_decompile/index.csv` — không có entry `0051f868`).
- Đây là kênh **Server đẩy dữ liệu danh sách người chơi / cập nhật nội dung form giới thiệu (recommend)** xuống client.

### Bối cảnh form `gvar_007D9F58` (TJC_Referee / Recommend Form)

Từ constructor `FUN_0051f074` (`ts_decompile/functions/0051f074_FUN_0051f074.c`), form này:
- Là form kích thước `0x118 × 0x96` (280×150), tên nội bộ `"form_module"`, có icon `"icon_RecommendOn"` (dòng 88) — **định danh nghiệp vụ "Recommend"**.
- Chứa 1 editor `TSe_Editor` (`"icon_EditorOn"`, dòng 90-93) cho phép **nhập mã số**, callback `FUN_0051f5c4` → `FUN_0051f5f4` xử lý chuỗi nhập:
  - Tách 2 ký tự đầu, `UpperCase`, so khớp với hằng chuỗi `DAT_0051f7d8` (`ts_decompile/functions/0051f5f4_FUN_0051f5f4.c:96-97`);
  - Nếu khớp: phần còn lại `StrToIntDef(...,0)` → lưu vào `form+0x134` (mã số nhập vào);
  - Nếu mã = 0 hoặc trùng `player+0x4` (CharID của chính mình) → hiện toast lỗi (`DAT_0051f7e4` 3000ms / `DAT_0051f800` 2000ms) và đóng;
  - Ngược lại set cờ `local_9 = 1` → `FUN_0051f5c4` gửi **`SendCommand(0x23)`** (`ts_decompile/functions/0051f5c4_FUN_0051f5c4.c:29` — opcode 0x23 chiều C→S, kèm biến phụ trợ) rồi gọi virtual `+0x24` (đóng form).
- Có 3 nút: `btn_close_s`, `btn_ok`, `btn_cancel` (dòng 109/142/168).
- Form được **mở** bởi Main OP `0x01` SubOp `0x0B` (đặt cờ `form+0x130 = 1` + gọi virtual `+0x20` — `ts_decompile/case_functions/functions/case_002_0078B149_FUN_0078b149.c:188-191`, đã ghi trong `opcode_00_01.md` mục 4.7), và bởi logic game `FUN_0076d578` (NPC menu `0xB464`/`0xFE92` set `form+0x18` = con trỏ từ `FUN_007afed0(gvar_007DA32C)`, `form+0x14c` = cờ, `form+0x14d` = `player+0x151C`, rồi virtual `+0x20` — `ts_decompile/functions/0076d578_FUN_0076d578.c:382-402`).

⇒ **Sơ đồ nghiệp vụ hoàn chỉnh**: Server mở form bằng OP `0x01/0x0B` (hoặc qua NPC menu) → người chơi nhập mã → client gửi OP `0x23` C→S → server xử lý → **đẩy kết quả về bằng OP `0x43/0x01`** (handler cập nhật lại form).

---

## 2. Entry & Dispatcher (S → C)

### 2.1. Định tuyến gói tin
```
Main OP 0x43 (67) → byte_table[0x78A8EE][0x43] = 0x3C (60)
                  → dword_table[0x78A9B6][60] @ 0x0078AAA6 = 0x00795F48
                  → FUN_00795f48 (Case 60)
```

### 2.2. Mã nguồn case function (toàn văn phần xử lý)
`ts_decompile/case_functions/functions/case_060_00795F48_FUN_00795f48.c:20-29`:
```c
iVar2 = *(int *)(unaff_EBP + -0xc);             // Rest Payload (chuỗi Delphi)
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {                // kiểm tra chuỗi rỗng (length header)
  iVar1 = _BoundErr(0);
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = Payload[0]
if (*(int *)(unaff_EBP + -0x14) == 1) {
  func_0x0051f868(*(undefined4 *)gvar_007D9F58, *(undefined4 *)(unaff_EBP + -0xc));  // chuyển tiếp toàn bộ payload
}
// phần còn lại: _LStrArrayClr/_LStrClr dọn dẹp stack locals (không xử lý)
```

- **So sánh trực tiếp `== 1`**, không phải switch: chỉ duy nhất SubOp `0x01` được xử lý; mọi giá trị khác bị bỏ qua lặng lẽ.

### 2.3. Dispatcher inline tương ứng
`ts_decompile/functions/0078a89c_FUN_0078a89c.c:7314`: `func_0x0051f868(*(undefined4 *)gvar_007D9F58, local_10);`

---

## 3. Bảng Tổng Hợp SubOp

| SubOp | Wire Format | Min Len | Đối tượng tiếp nhận | Handler & Trạng Thái SSOT | Ý Nghĩa Nghiệp Vụ |
| :---: | :--- | :---: | :--- | :--- | :--- |
| **`0x01`** | `[43][01][...]` (phần `...` do handler quyết định — chưa decompile) | ≥2B | `gvar_007D9F58` (form `TJC_Referee` / Recommend) | `func_0x0051f868` @ `0x0051F868` (**✘ Chưa decompile — khe hở nhị phân**) | Server đẩy dữ liệu / kết quả xử lý mã giới thiệu vào form Recommend (cập nhật trạng thái form sau khi client gửi OP 0x23). |

Không có SubOp nào khác.

---

## 4. Chi Tiết Core Logic

### 4.1. SubOp `0x01` — Chuyển tiếp cho handler form Recommend

- **Luồng gọi**: `FUN_00795f48` → `func_0x0051f868(form_007D9F58, payload)`.
- Tham số `param_1` = con trỏ form, `param_2` = **toàn bộ Rest Payload** (bao gồm byte SubOp `0x01` ở `[0]` — cùng mô-đun chuyển tiếp như các opcode 0x41/0x42).
- **Nội dung xử lý bên trong `func_0x0051f868` KHÔNG thể xác minh từ SSOT hiện tại**: hàm nằm trong khoảng trống không được xuất ra `ts_decompile/functions/` (giữa `FUN_0051f5f4` và `FUN_0051fb6c`; `ts_decompile/index.csv` không có entry `0051f868`).
- **Suy luận từ ngữ cảnh form** (chỉ ghi nhận như giả thuyết, đánh dấu rõ KHÔNG phải bằng chứng trực tiếp):
  - Form có editor nhập mã (`form+0x4E`), cờ hiển thị `form+0x130`, mã số lưu `form+0x134`, con trỏ dữ liệu `form+0x18`, cờ `form+0x14C/0x14D`.
  - Handler nhiều khả năng cập nhật một trong các field này từ payload (ví dụ kết quả xác minh mã giới thiệu: thành công / thất bại / thông tin người giới thiệu) rồi refresh form qua virtual `VMT+0x20`.

### 4.2. Chu kỳ sống của form Recommend (ngữ cảnh mở/đóng đã xác minh)

| Giai đoạn | Kênh | Bằng chứng |
| :--- | :--- | :--- |
| Server ra lệnh **mở form** | Main OP `0x01` SubOp `0x0B`: `form+0x130 = 1`, virtual `+0x20` | `case_002_0078B149_FUN_0078b149.c:188-191` |
| Mở qua **menu NPC** | `FUN_0076d578`: menu `0xB464` / `0xFE92` set `form+0x18`, `+0x14C`, `+0x14D` (= `player+0x151C`), virtual `+0x20` | `0076d578_FUN_0076d578.c:382-402` |
| Người chơi **nhập mã** | Editor callback `FUN_0051f5c4` → `FUN_0051f5f4`: kiểm tra tiền tố + `StrToIntDef` → `form+0x134` | `0051f5f4_FUN_0051f5f4.c:69-121` |
| Client **gửi mã lên server** | `FUN_0051f5c4`: `FUN_0077f414(gvar_007D9D30, 0x23)` — **Main OP 0x23 chiều C→S** (chi tiết builder xem `opcode_23.md`) | `0051f5c4_FUN_0051f5c4.c:29` |
| Server **đẩy phản hồi** | **Main OP 0x43 SubOp 0x01** → `func_0x0051f868(form, payload)` | `case_060_00795F48_FUN_00795f48.c:28` |

---

## 5. Ghi Chú Mock Server & Điểm Chưa Kết Luận Được

1. **Handler chưa decompile (rào cản lớn nhất)**: `func_0x0051f868` chưa có trong `ts_decompile/`. Muốn đặc tả wire format chính xác của phần `...` sau `[43][01]`, phải **dump thêm hàm từ binary bằng Ghidra** (thủ tục tương tự các khe hở đã ghi trong `opcode_14.md` mục 6.7 và `opcode_41.md` — "Khe chưa decompile"). Tài liệu này KHÔNG đoán cấu trúc payload khi thiếu bằng chứng.
2. **Kích hoạt an toàn**: để smoke-test, mock có thể gửi `[43][01]` 2 byte tối thiểu — client sẽ gọi handler với payload ngắn; hành vi phụ thuộc vào kiểm tra độ dài bên trong handler (chưa xác minh được).
3. **Không có VISCII**: không có bằng chứng handler xử lý chuỗi VISCII trong các phần đã decompile của form.
4. **Chuẩn wire format chung** (khớp `handoff-opcode-exploration-guide.md` mục 1): frame `[F4 44][Len:2B LE][Payload]`, toàn frame XOR `0xAD`; payload = `[0x43][SubOp=0x01][...]`.
5. **Chiều C→S của chính OP 0x43 là rỗng**: `ts_decompile/functions/0077f414_FUN_0077F414.c:1068-1069` (`case 0x43: break;`). Client không gửi gói tin nào bằng OP 0x43; luồng gửi liên quan dùng OP `0x23` (xem 4.2).

---

## 6. Nguồn Tham Chiếu (Single Source of Truth: `ts_decompile/`)

- Case 60 & SubOp duy nhất: `ts_decompile/case_functions/functions/case_060_00795F48_FUN_00795f48.c:20-29`
- Bản gộp 65 case: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9463`
- Dispatcher inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7314`
- Handler (chưa decompile): `func_0x0051f868` @ `0x0051F868` — không có trong `ts_decompile/index.csv` (6303 entry)
- Constructor form `TJC_Referee`: `ts_decompile/functions/0051f074_FUN_0051f074.c` (instance đầu tiên được đăng ký vào `gvar_007DA15C` tại `0051189c_FUN_0051189c.c:1415`). **Lưu ý**: phép gán chính xác `gvar_007D9F58` (biến toàn cục mà OP 0x43 sử dụng) **chưa xuất hiện trong bất kỳ file nào đã decompile** của `ts_decompile/` — đã kiểm tra bằng grep toàn bộ `ts_decompile/functions/*.c` và `*.asm.txt`: chỉ có 4 nơi *đọc* nó (`0078a89c.c:964,965,7314`, `0076d578.c:384-400`, `case_060...c:28`, `case_002...c:190`), không có nơi *ghi*. Nó được khởi tạo trong một hàm chưa trích xuất decompile (khả năng cao thuộc cụm khởi tạo form Recommend thứ hai cùng lớp `TJC_Referee`). Danh tính chính xác cần dump thêm từ binary.
- Editor callback & gửi OP 0x23: `ts_decompile/functions/0051f5c4_FUN_0051f5c4.c` · `ts_decompile/functions/0051f5f4_FUN_0051f5f4.c`
- Mở form qua OP 0x01/0x0B: `ts_decompile/case_functions/functions/case_002_0078B149_FUN_0078b149.c:188-191` · `.scratch/op-code/opcode_00_01.md` mục 4.7
- Mở form qua NPC menu: `ts_decompile/functions/0076d578_FUN_0076d578.c:382-402`
- Chiều C→S (rỗng): `ts_decompile/functions/0077f414_FUN_0077F414.c:1068-1069`
