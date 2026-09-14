# PHÂN TÍCH — Main OP 0x43 (67) / Case 60 / FUN_00795f48 @ 0x00795F48

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler `func_0x0051f868` **NAY ĐÃ CÓ BODY** (HOLE `0x0051F7C0+–0x0051FB6C` được decompile, redump 2026-09-14, `index.csv:2883` cạnh `FUN_0051fb6c`) — §4.1/§5 viết lại theo body; wire **đã đặc tả được**.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm Tắt Nghiệp Vụ Cốt Lõi

Main OP `0x43` (thập phân: `67`, ánh xạ tới **Case 60**) là **opcode đơn SubOp** — case duy nhất là `SubOp 0x01`, chuyển tiếp toàn bộ Rest Payload cho **một hàm xử lý của form "Giới Thiệu Người Chơi" (Recommend / Referee form)**:

- Đối tượng tiếp nhận: `gvar_007D9F58` — form thuộc cụm `TJC_Referee` (VMT `0x51EF94`, constructor `FUN_0051f074` tạo tại `ts_decompile/functions/0051189c_FUN_0051189c.c:1415-1416`).
- Handler: `func_0x0051f868` @ `0x0051F868` — **ĐÃ CÓ BODY**: `ts_decompile/functions/0051f868_FUN_0051f868.c` (`index.csv:6312`, HOLE `0x0051F7C0+–0x0051FB6C` đã giải trong redump 2026-09-14). Phân tích đầy đủ tại §4.1.
- Đây là kênh **server trả kết quả xác minh mã giới thiệu**: thành công → đổi/cập nhật tên hiển thị nhân vật trên mọi bản sao cache + announce chat (đóng form); các mã lỗi → toast; mã đặc biệt `0x05` → **đóng socket + hiện form chọn server** (đá về `TFrmSelectServer`).

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
| **`0x01`** | `[43][01][Kind:1B]` + (Kind=0: `[CharID:4B LE][NewName:≤17B]`) | 2B (Kind 0: ≥6B) | `gvar_007D9F58` (form `TJC_Referee` / Recommend) | `FUN_0051f868` @ `0x0051F868` (**✔ body mới — §4.1**) | Kết quả xác minh mã giới thiệu: Kind 0 = thành công (đổi/áp tên mới + chat + đóng form); Kind 1..4/`=0x05` = lỗi/kick (§4.1). |

Không có SubOp nào khác.

---

## 4. Chi Tiết Core Logic

### 4.1. SubOp `0x01` — Handler form Recommend (`func_0x0051f868`) — **ĐÃ PHÂN TÍCH TỪ BODY MỚI**

- **Tệp nguồn**: `ts_decompile/functions/0051f868_FUN_0051f868.c` (119 dòng, `index.csv:6312`; HOLE `0x0051F7C0+–0x0051FB6C` đã giải — *đính chính toàn bộ trạng thái "chưa decompile" cũ*).
- **Luồng gọi**: `FUN_00795f48` → `func_0x0051f868(form_007D9F58, RP)`; `param_1` = con trỏ form, `param_2` = toàn bộ Rest Payload.
- **Cấu trúc điều khiển** (`0051f868_FUN_0051f868.c:42-107`):
  1. `RP != 0` và `RP[0]` phải == 1 (`:57`) — re-read đúng SubOp; `len(RP)<2` → `_BoundErr(1)` **ERangeError** (`:58-60`).
  2. `switch (RP[1])` — **mã kết quả** của server:

  | `RP[1]` | Hành vi (đọc từ body) | Dẫn chứng |
  | :---: | :-- | :-- |
  | `0x00` | **Thành công**: `id = DWORD_LE(RP[2..5])`, `name = RP[6..end]` → `FUN_0051fb6c(form, id, name)` (xem dưới) | `:65-78` |
  | `0x01..0x04` | Toast 3000ms từng mã lỗi riêng: `DAT_0051fa8c` / `DAT_0051fabc` / `DAT_0051fae8` / `DAT_0051fb0c` (chuỗi **chưa dump** — cần `lit_51f8xx.hex`) | `:79-94` |
  | `0x05` | **Đứt kết nối**: `TCustomWinSocket_Close(*(gvar_007DA664↑ + 0x3E8 + 0x80))` (socket của `TFConnect` — `opcode_14.md:59`) → virtual `[**gvar_007D9CC4 + 0x20]()` = **hiện form `TFrmSelectServer`** (`login_flow_research.md:186`) → virtual `[form+0x24]()` = đóng form Recommend → toast `DAT_0051fb24` 3000ms | `:95-103` |
  | khác | Toast `DAT_0051fb58` 3000ms | `:104-106` |
- **`FUN_0051fb6c(form, id, name)` = thủ tục ĐỔI TÊN / cập nhật tên hiển thị** (`0051fb6c_FUN_0051fb6c.c`, 945B — *đính chính giả thuyết cũ "cập nhật field form +0x134 rồi refresh"*):
  - Nhánh `id == LocalPlayer.CharID` (`gvar_007DA7BC↑+4`, `:66`): toast xác nhận `DAT_0051ff34`; ghép chuỗi `DAT_0051ff48 + <tên cũ shortstring @LocalPlayer+9> + LAB_0051ff54 + <name>` đẩy vào chat `TTalkMsgForm` (`:70-76`); **áp `<name>` làm tên mới** (shortstring copy cap `0x11`=17B) vào `LocalPlayer+9` (`:78`) và bản sao trong `StatusInfoForm` (`gvar_007D9E5C↑+0x3C0+9`, `:80` — `gvar_007D9E5C` = StatusInfoForm theo `opcode_13.md:120`).
  - Nhánh `id != mình` (`:82+`): tra slot cache `FUN_00722508(gvar_007D9C48, id)`; nếu tìm thấy: chat announce tương tự (`:87-97`), cập nhật actor scene `gvar_007DA300[idx]+9` qua `FUN_0070c20c` (`:98-107`), quét bảng `gvar_007DA218` (257 entry stride `0x22`, id khớp → name tại `+6`, cap `0xE`) (`:113-136`); ngoài if/else, quét tiếp bảng `gvar_007DA6E8` (104 entry stride `0x4E`) (`:143-166`) và cache 2100-slot `gvar_007DA6BC` (`+4`=id khớp → `+8` := tên mới) (`:165-188`).
  - Kết luận chung mọi nhánh: `(**[form]+0x24)()` — **đóng form Recommend** (`:189`).
- **Wire format ĐÃ ĐẶC TẢ (thay "unknown" cũ)**:
  - Thành công: `[43][01][00][CharID:4B LE][NewName: ASCII/VISCII 1..17B]` (RestPayload ≥ 6B; tên rỗng nếu `RP[6..]` trống — chưa kiểm client có guard).
  - Lỗi/kick: `[43][01][01..05]` (RestPayload đúng 2B).
- Handler **không ghi field nào của form ngoài virtual close `+0x24`** — giả thuyết cũ "cập nhật form+0x130/0x134 từ payload" bị **bác**; chuỗi nhập của editor (`form+0x134`) chỉ dùng phía C→S OP 0x23.

### 4.1b. Ghi chú còn mở

- 8 chuỗi toast/announce (`DAT_0051fa8c…DAT_0051fb58`, `DAT_0051ff34/48/54`) **chưa có bytes** → cần redump `lit_51fa8c…lit_51ff54.hex`.
- `gvar_007DA664` (TFConnect) — offset socket `+0x3E8+0x80` chưa đối chiếu thêm.

### 4.2. Chu kỳ sống của form Recommend (ngữ cảnh mở/đóng đã xác minh)

| Giai đoạn | Kênh | Bằng chứng |
| :--- | :--- | :--- |
| Server ra lệnh **mở form** | Main OP `0x01` SubOp `0x0B`: `form+0x130 = 1`, virtual `+0x20` | `case_002_0078B149_FUN_0078b149.c:188-191` |
| Mở qua **menu NPC** | `FUN_0076d578`: menu `0xB464` / `0xFE92` set `form+0x18`, `+0x14C`, `+0x14D` (= `player+0x151C`), virtual `+0x20` | `0076d578_FUN_0076d578.c:382-402` |
| Người chơi **nhập mã** | Editor callback `FUN_0051f5c4` → `FUN_0051f5f4`: kiểm tra tiền tố + `StrToIntDef` → `form+0x134` | `0051f5f4_FUN_0051f5f4.c:69-121` |
| Client **gửi mã lên server** | `FUN_0051f5c4`: `FUN_0077f414(gvar_007D9D30, 0x23)` — **Main OP 0x23 chiều C→S** (chi tiết builder xem `opcode_23.md`) | `0051f5c4_FUN_0051f5c4.c:29` |
| Server **đẩy phản hồi** | **Main OP 0x43 SubOp 0x01** → `func_0x0051f868(form, payload)` — Kind 0: `FUN_0051fb6c` đổi tên + chat + virtual `form+0x24` đóng form; Kind 5: close socket + hiện `TFrmSelectServer` | `case_060_00795F48_FUN_00795f48.c:28`; `0051f868_FUN_0051f868.c:64-106`; `0051fb6c_FUN_0051fb6c.c:66-189` |

---

## 5. Ghi Chú Mock Server & Điểm Chưa Kết Luận Được

1. ~~Handler chưa decompile~~ — **ĐÃ GỠ (2026-09-14)**: wire đặc tả xong (§4.1). Rào cản còn lại duy nhất: **nội dung 8 chuỗi toast/announce chưa dump bytes**.
2. **Gửi tối thiểu**: RestPayload **bắt buộc ≥ 2B** (`[01][Kind]` — `[01]` cụt sẽ `_BoundErr(1)` ERangeError trong handler); Kind 0 bắt buộc `[01][00][CharID 4B][Name 1..B]` (codec DWORD `_BoundErr` nếu chuỗi cắt ra `<4` ký tự).
3. **VISCII**: handler không decode chuỗi — tên mới được `_LStrToString/_PStrNCpy` cắt nguyên trạng 17B vào shortstring; chuỗi thông báo là hằng .data (chưa dump).
4. **Chuẩn wire format chung** (khớp `handoff-opcode-exploration-guide.md` mục 1): frame `[F4 44][Len:2B LE][Payload]`, toàn frame XOR `0xAD`; payload = `[0x43][SubOp=0x01][...]`.
5. **Chiều C→S của chính OP 0x43 là rỗng**: `ts_decompile/functions/0077f414_FUN_0077F414.c:1068-1069` (`case 0x43: break;`). Client không gửi gói tin nào bằng OP 0x43; luồng gửi liên quan dùng OP `0x23` (xem 4.2).

---

## 6. Nguồn Tham Chiếu (Single Source of Truth: `ts_decompile/`)

- Case 60 & SubOp duy nhất: `ts_decompile/case_functions/functions/case_060_00795F48_FUN_00795f48.c:20-29`
- Bản gộp 65 case: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:9463`
- Dispatcher inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:7314`
- Handler (**ĐÃ DECOMPILE 2026-09-14**): `ts_decompile/functions/0051f868_FUN_0051f868.c` · `ts_decompile/functions/0051f868_FUN_0051f868.asm.txt` (`index.csv:6312`, size 189B) + helper `0051fb6c_FUN_0051fb6c.c` (`index.csv:2883`)
- Constructor form `TJC_Referee`: `ts_decompile/functions/0051f074_FUN_0051f074.c` (instance đầu tiên được đăng ký vào `gvar_007DA15C` tại `0051189c_FUN_0051189c.c:1415`). **Tái kiểm tra 2026-09-14 (kể cả sau khi HOLE 0x51F7C0+ được giải)**: grep `gvar_007D9F58`/`7d9f58` toàn `functions/*.c` + `*.asm.txt` → **vẫn KHÔNG có dòng gán khởi tạo** (chỉ có đọc + ghi field qua deref: `0076d578.c:384-400`, `0078a89c.c:964-965`, case_060/case_002) — form creator của instance này **chưa tồn tại trong SSOT**, chưa cite được; danh tính vẫn cần dump thêm.
- Editor callback & gửi OP 0x23: `ts_decompile/functions/0051f5c4_FUN_0051f5c4.c` · `ts_decompile/functions/0051f5f4_FUN_0051f5f4.c`
- Mở form qua OP 0x01/0x0B: `ts_decompile/case_functions/functions/case_002_0078B149_FUN_0078b149.c:188-191` · `.scratch/op-code/opcode_00_01.md` mục 4.7
- Mở form qua NPC menu: `ts_decompile/functions/0076d578_FUN_0076d578.c:382-402`
- Chiều C→S (rỗng): `ts_decompile/functions/0077f414_FUN_0077F414.c:1068-1069`
