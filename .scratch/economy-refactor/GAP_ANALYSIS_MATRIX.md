# BÁO CÁO ĐỐI SOÁT CHUYÊN SÂU GIAO THỨC KINH TẾ (GAP ANALYSIS MATRIX)
**Giữa Tài liệu Đảo ngược Client TS Online (`ECONOMY_CLIENT_SPEC.md`) và Mã nguồn Rust Server (`ts_dream`)**

---

## 1. TỔNG QUAN & PHẠM VI ĐỐI SOÁT

- **Mục tiêu**: Thực hiện đối soát 1-1 toàn diện giữa tài liệu dịch ngược mã máy thân client TS Online (`aLogin.exe` Delphi) tại `.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md` và mã nguồn Rust hiện tại của máy chủ `ts_dream`.
- **Hệ quy chiếu chuẩn**:
  - Client Spec: Đạt Mức tin cậy A (98.5% đối chiếu trực tiếp mã máy ASM & Hex literals).
  - Scope kiểm tra: Toàn bộ 25 Opcode liên quan kinh tế, 13 phát hiện điều hành cốt tử, cơ chế bảo vệ tràn số, mô hình bộ đếm tài nguyên, máy trạng thái giao diện UI, và hệ thống test wire hiện có.
- **Tài liệu tham chiếu phía server**:
  - `src/server/handlers/shops.rs`
  - `src/server/handlers/trade_storage.rs`
  - `src/server/handlers/talk.rs`
  - `src/server/handlers/inventory.rs`
  - `src/server/handlers/use_item/`
  - `src/server/dispatcher.rs`
  - `src/server/session.rs`
  - `src/server/spawn.rs`
  - `src/server/trade_system.rs`
  - `src/protocol/mod.rs`
  - `src/protocol/codecs/thing_data.rs`
  - `src/db/` (`persist.rs`, `modern/model.rs`, `modern/sqlite/session.rs`, `modern/transactions.rs`)
  - `tests/economy_wire_test.rs` và các test suite liên quan.

---

## 2. MA TRẬN ĐỐI SOÁT 1-1 CHO 13 PHÁT HIỆN CỐT TỬ (MỤC 6.2)

Bảng đối chiếu chi tiết giữa 13 kết luận đảo ngược client và hiện trạng cài đặt trong codebase Rust:

| # | Phát hiện cốt tử Client Spec (§6.2) | Hiện trạng phía Server Rust (`ts_dream`) | Đánh giá Parity | Điểm sai lệch / Ghi chú kỹ thuật & File:Line Citation |
|---|---|---|:---:|---|
| **1** | **Danh mục hàng và giá Shop NPC không đi trên mạng**: Server chỉ gửi S→C `14 01..06` (`B2 = 6`, `W3 = shopSurfaceIndex`). Client tự đọc `Data\Eve.emg` và tra giá từ `Data\Item.Dat` (`+0x58` mua, `+0x5C` bán). | • [`shops.rs:159-193`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L159-L193) đọc `scene_eve_data.surface_datas` và `item_defs`, không gửi catalog qua wire.<br>• [`talk.rs:737-751`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L737-L751) gửi `14 01` với `B2 = 6`. | **Khớp 80% / Lệch kích hoạt** | • **Khớp wire**: Server không đẩy catalog lên mạng.<br>• **Lệch logic**: Khi người chơi chọn menu 30 ("Mua"), server tại [`talk.rs:738`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L738) gửi đồng thời cả `1B 03` (mở form bán) và `14 01` (mở form mua)! Client Delphi bật cả 2 form đè lên nhau. |
| **2** | **Frame `1B 03` là S→C mở form bán (`TThingSaleForm`), tuyệt đối không phải C→S**: Chiều C→S chỉ có `sub = 1` và `sub = 2`, `sub >= 3` nhảy về Epilogue RET. S→C `1B 03` dài đúng 2 byte. | • [`shops.rs:20-23`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L20-L23) định nghĩa `open_npc_shop_frame() -> "F44402001B03"`.<br>• [`shops.rs:222-395`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L222-L395) chỉ nhận `sub = 1` và `sub = 2`. | **Khớp Wire / Lệch Ngữ nghĩa** | • **Khớp cấu trúc**: S→C `1B 03` dài đúng 2 byte, server không nhận C→S `1B 03`.<br>• **Lệch ngữ nghĩa**: Server đặt tên hàm là `open_npc_shop_frame()` và gửi khi mở shop chung hoặc Eve effect 7 param 3 ([`talk.rs:504`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L504)), trong khi `1B 03` trong client là **chuyên biệt cho Form Bán đồ**. |
| **3** | **Bất đối xứng Mua (`1B 01`) vs Bán (`1B 02`)**:<br>• `1B 01`: `index: u8` (0-based) + `count: u16 LE` (1..1250).<br>• `1B 02`: `slot: u8` (1-based `1..25`) + `qty: u8` (1..50, 1 byte). | • [`shops.rs:224-229`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L224-L229): parse `item_index: u8` (0-based) và `count: u16 LE`.<br>• [`shops.rs:331-332`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L331-L332): parse `slot: u8` (1-based) và `qty: u8`. | **Khớp Wire / Lệch Guard** | • **Khớp Wire Layout**: Đúng kích thước và endian.<br>• **Lệch Validation**: Tại [`shops.rs:235-238`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L235-L238), server ép trần `if count > 255` và trả lỗi `NPC_SHOP_BUY_QTY_FULL` (mã 0x03), trong khi client ASM tại `0x005D7259` cho phép `count` lên tới **1250** (`0x4E2`). |
| **4** | **Chiều Ngân hàng `1D 01` vs `1D 02`**:<br>• C→S: `1D 01` Rút kim, `1D 02` Nộp kim.<br>• S→C: `1D 01` Cộng tiền trang (`+Δ`), `1D 02` Trừ tiền trang (`-Δ`).<br>• S→C kim túi: `1A 01` Cộng (`+Δ`), `1A 02` Trừ (`-Δ`). | • [`trade_storage.rs:836-867`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L836-L867): `handle_bank_gold` xử lý Rút (`sub = 1`) gửi `1D 02` + `1A 01`; Nộp (`sub = 2`) gửi `1D 01` + `1A 02`. | **Khớp Logic Wire / Thiếu Frame UI** | • **Khớp Wire Logic**: Đã đảo chiều S→C đúng với hàm ASM client `FUN_0072b9b8` / `FUN_0072ba0c`.<br>• **Thiếu Frame Mở khoá UI**: Client khi bấm nút Rút khoá `form+0x11D = 1`, bấm Nộp khoá `form+0x11C = 1`. Server **chưa bao giờ gửi `1D 07` / `1D 08`** để mở khoá nút bấm, khiến nút bị tê liệt sau 1 lần bấm! |
| **5** | **Cấu trúc `17 06` nhận vật phẩm dài 14 byte, không có `slot`**: `17 06 <itemId:u16 LE> <qty:u16 LE> <b1:u8> <b3:u8> <b4:u8> <b5:u8> <exp:u32 LE>`. Client tự tìm slot bằng `FUN_0072b6c8`. | • [`inventory.rs:36-47`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/inventory.rs#L36-L47): `item_added_frame` đóng gói payload 12B data + 2B opcode = 14B wire, không có byte slot. | **Khớp 100% (Matched)** | Server đã hiện thực chính xác cấu trúc 14 byte chuẩn, loại bỏ hoàn toàn byte `slot` giả định trong các server C# Bear cũ. |
| **6** | **Cấu trúc `17 21` quầy sạp người khác**: Header **18 byte wire** (`17 21 <vendorUid:u32 LE> <card1:u32> <card2:u32> <cardPrice:u32>`), sau đó là `N` record **16 byte** (`[slot:u8][itemId:u16][qty:u8][b4:u8][price:u32][b9:u8][b10:u8][b11:u8][exp:u32]`). | • [`shops.rs:404-429`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L404-L429): `player_shop_catalog_frame` bọc header 18B (`17 21` + `vendorUid` + 12 zero) và mỗi item đúng 16 byte. | **Khớp 100% (Matched)** | Server tuân thủ hoàn toàn cấu trúc header 18 byte và stride 16 byte được chứng minh trong `00610290_FUN_00610290.c`. |
| **7** | **Frame C→S `17 1E` mở sạp dài 109 byte + bảng hiệu**: Mảng 10 ô × 10 byte `[bagSlot:u8][qty:u8][price:u32 LE][vitality:u32 LE]` + `boardName` + `cardIdx:u8` + `cardData:4B`. S→C `17 1E` phản hồi kết quả dài **3 byte** (`17 1E <res:u8>`). | • [`shops.rs:572-631`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L572-L631): Server parse `payload[0] = name_len`, `name`, `image`, và các cặp 5 byte `(slot:u8, price:u32)`.<br>• Phản hồi S→C: gửi frame chuỗi dài `171E` tự chế. | **Sai lệch Nghiêm trọng (P0 Mismatch)** | • **C→S sai hoàn toàn thứ tự byte**: Client gửi 100 byte item trước, rồi mới tới tên bảng hiệu. Server đọc `payload[0]` làm `name_len` khiến dữ liệu sạp bị hỏng hoàn toàn.<br>• **S→C sai cấu trúc**: Server gửi frame chuỗi dài thay vì frame 3 byte `F4440300171E01`. Client `FUN_0076f92c` đọc `RP[1]` sẽ hiểu nhầm là lỗi. |
| **8** | **Bảo vệ tràn số tiền tệ: Delta vs Absolute**:<br>• Delta (`1A 01/02`, `1D 01/02`): Client từ chối nguyên tử nếu vượt trần `9.999.999` hoặc âm.<br>• Absolute (`1A 04`, `1D 04`, `1A 03`): Ghi đè trực tiếp, không clamp. | • [`protocol/mod.rs:4`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L4): `GOLD_CAP = 9_999_999`.<br>• [`shops.rs:11-18`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L11-L18): clamp `gold_frame`.<br>• [`trade_storage.rs:839-843`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L839-L843): kiểm tra `saturating_add(amount) <= GOLD_CAP`. | **Khớp Logic Cap / Sai Frame Wire** | • Server có kiểm tra `GOLD_CAP` chống tràn cho giao dịch Shop và Bank.<br>• Nhưng tại [`shops.rs:11-18`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L11-L18) và [`spawn.rs:615-617`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L615-L617), server gửi `1A 04` dài 10 byte ghép chung `shop_point`, sai lệch wire format so với client ASM. |
| **9** | **Tiền mềm / Point (`+0x1300`, trần 2 tỷ)**: Chỉ có frame cộng delta `1A 05` và set tuyệt đối `1A 03`. Không có frame trừ S→C. Trừ điểm bắt buộc dùng `1A 03 <newTotal>`. | • [`protocol/mod.rs:171,174`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L171): Server gán `MONEY_SUB_CAPACITY_FULL = 0x03` và `MONEY_SUB_SUB_POINTS = 0x06`.<br>• Server chưa từng gửi `1A 03` để set point. | **Sai lệch Nghiêm trọng (P0 Mismatch)** | • **Đảo lộn Sub-opcode**: `1A 03` trong client là **Set Point**, `1A 06` là **Dung lượng tiền bạc không đủ**. Server định nghĩa ngược hoàn toàn.<br>• Client hoàn toàn không cập nhật được điểm khi server gửi `1A 04` ghép `shop_point`. |
| **10** | **Giao dịch P2P `0x19` tách biệt 2 kênh**:<br>• Kênh Item/Kim: `19 01..04`. Payload `19 02` gửi 16 byte (`u32 gold` + 5 cặp `(slot:u8, qty:u8)`). Payload `19 03` S→C trả 11 byte/item.<br>• Kênh Võ tướng: `19 0A..0D`, `19 14..15`. | • [`trade_storage.rs:246,286`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L246): Server có tách `19 01` và `19 0A`.<br>• [`trade_storage.rs:335`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L335): parse `for slot in &payload[4..]`.<br>• [`trade_storage.rs:371`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L371): check `action == 2`.<br>• [`trade_storage.rs:43`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L43): gửi `17 3B` khi huỷ. | **Sai lệch Nghiêm trọng (P0 Mismatch)** | • **Parse `19 02` sai lệch**: Client gửi 5 cặp `(slot, qty)`, server parse từng byte làm `slot` riêng biệt, dẫn tới đọc nhầm `qty` thành slot tiếp theo.<br>• **Huỷ trade `19 03` sai mã**: Client gửi `action = 0` khi huỷ, server lại chờ `action == 2`.<br>• **Gửi sai frame đóng form**: Server gửi `17 3B` (Cắt hàng) thay vì `19 04` (Đóng form trade) hoặc `19 02 03`. |
| **11** | **Lỗi thiếu nhánh switch trong `0x1B` (`inner = 5` hoặc `>= 7`)**: Trong client gây hiện Banner rỗng. `inner = 0` không hiện banner. Server chỉ được dùng các mã hợp lệ `0, 1, 2, 3, 4, 6`. | • [`protocol/mod.rs:188-201`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L188-L201): Định nghĩa các hằng số kết quả mua/bán bỏ qua mã `5` (dùng `0, 1, 2, 3, 4, 6`). | **Khớp 100% (Matched)** | Server đã chủ động loại trừ mã lỗi `5`, bảo vệ client không bị dính bug banner rỗng của game gốc. |
| **12** | **Opcode chỉ có S→C, không có C→S (`0x35`, `0x38`, `0x3E`)**: C→S builder trỏ về Epilogue RET (`0x0078A4F2`). Client không bao giờ gửi 3 opcode này. | • [`dispatcher.rs:364-394`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L364-L394): Các opcode này không có nhánh xử lý riêng, gom vào fallback `unimplemented::handle`. | **Khớp 100% (Matched)** | Server không parse C→S cho 3 opcode này, hoàn toàn phù hợp với đặc tính client Delphi. |
| **13** | **Đính chính `23 04`, `02 0B`, `0x2C`**:<br>• `23 04`: Hệ số giá nhà trọ/thương nhân (`player+0x146C = A*100 + B`), không phải HP/SP store.<br>• `02 0B`: Mở form tạo Quân Đoàn (100k kim), không phải chat/toast.<br>• `0x2C`: Vòng quay kỹ năng trong trận đánh (`TFWheel`), không phải sòng bạc/reborn pet. | • [`shops.rs:37`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L37), [`chat.rs:40`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/chat.rs#L40): dùng `02 0B` làm red message toast.<br>• [`dispatcher.rs:344`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L344): `0x23` gán cho account management.<br>• [`dispatcher.rs:353`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L353): `0x2C` gán cho `OP_REBORN_PET`. | **Sai lệch Toàn diện (P1 Mismatch)** | • `02 0B` bị lạm dụng làm toast chat, trong khi client kích hoạt mở form tạo bang hội.<br>• `23 04` bị bỏ quên không xử lý hệ số giá nhà trọ.<br>• `0x2C` bị định danh nhầm thành Pet Reborn thay vì Battle Wheel. |

---

## 3. PHÂN LOẠI CHI TIẾT 3 NHÓM (MATCHED, MISMATCHES, MISSING)

### 3.1. Nhóm 1: Khớp chuẩn xác (Matched) — Parity 100%

1. **Cấu trúc frame S→C `17 06` (Thêm vật phẩm vào túi)**:
   - **Bằng chứng Server**: [`src/server/inventory.rs:36-47`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/inventory.rs#L36-L47) hàm `item_added_frame(&InventoryItem)`.
   - **Bằng chứng Client Spec**: §6.5.3 mục 1 ([`0076b458_FUN_0076b458.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L436)).
   - **Đánh giá**: Chuẩn xác 14 byte payload (`17 06 <id:u16> <qty:u16> <b1> <b3> <b4> <b5> <exp:u32>`), hoàn toàn không chứa byte slot dư thừa.
2. **Cấu trúc Catalog quầy sạp S→C `17 21`**:
   - **Bằng chứng Server**: [`src/server/handlers/shops.rs:404-429`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L404-L429) hàm `player_shop_catalog_frame`.
   - **Bằng chứng Client Spec**: §6.5.3 mục 2 ([`00610290_FUN_00610290.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L449)).
   - **Đánh giá**: Đúng cấu trúc header 18 byte (`17 21 <vendorUid:u32> 12_zeros`) và stride 16 byte/record, bao gồm cả biến đổi thuộc tính `biased_giatri_long`.
3. **Mã kết quả Giao dịch Shop NPC (`0x1B`) tránh bẫy switch**:
   - **Bằng chứng Server**: [`src/protocol/mod.rs:188-201`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L188-L201).
   - **Bằng chứng Client Spec**: §6.8.1 ([`00791d80_FUN_00791d80.asm.txt`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L746)).
   - **Đánh giá**: Sử dụng đúng các mã `0x00, 0x01, 0x02, 0x03, 0x04, 0x06`, loại bỏ mã `0x05` tránh lỗi banner rỗng client.
4. **Đảo chiều wire S→C Rút / Nộp Tiền trang (`0x1D`)**:
   - **Bằng chứng Server**: [`src/server/handlers/trade_storage.rs:862-867`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L862-L867).
   - **Bằng chứng Client Spec**: §6.4.2 & §6.5.2 ([`00791f76_FUN_00791f76.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L412)).
   - **Đánh giá**: Rút tiền gửi `1D 02` + `1A 01`; Nộp tiền gửi `1D 01` + `1A 02`. Đúng 100% với hàm bộ nhớ client.
5. **Cấu trúc Item trong Giao dịch P2P S→C `19 03`**:
   - **Bằng chứng Server**: [`src/server/handlers/trade_storage.rs:73-83`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L73-L83) hàm `item_wire`.
   - **Bằng chứng Client Spec**: §6.5.4 ([`0075b3ec_FUN_0075b3ec.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L465)).
   - **Đánh giá**: Đúng cấu trúc 11 byte (`itemId:u16, qty:u8, 4 attr bytes, exp:u32`), không có byte slot.
6. **Bảo vệ trần tiền tệ `GOLD_CAP = 9.999.999` trong Database & Core**:
   - **Bằng chứng Server**: [`src/protocol/mod.rs:4`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L4), [`src/db/modern/transactions.rs:33-80`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/db/modern/transactions.rs#L33-L80).
   - **Bằng chứng Client Spec**: §6.7.1 ([`0072b084_FUN_0072b084.c`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md#L703)).
   - **Đánh giá**: Ngăn chặn số dư âm và vượt trần 9.999.999 trên SQLite và RAM session.

---

### 3.2. Nhóm 2: Sai lệch Wire / Logic (Mismatches) — Cần sửa gấp

```
                                  DANH SÁCH CÁC ĐIỂM SAI LỆCH WIRE / LOGIC
+----+-----------------------------------------------------+-------------------------------------------------------+
| STT| Client Spec Expectation (Mã máy Delphi)              | Server Implementation Hiện tại (Rust)                 |
+----+-----------------------------------------------------+-------------------------------------------------------+
| 1  | S->C 1A 04 dài 6B: 1A 04 <gold:u32 LE>              | shops.rs:13 gửi 1A 04 dài 10B ghép cả shop_point      |
| 2  | S->C 1A 03 dài 6B: Set Point (+0x1300)              | protocol/mod.rs:171 gán 1A 03 = CAPACITY_FULL!        |
| 3  | C->S 17 1E: 100B (10 items x 10B) + name + card     | shops.rs:573 parse name_len ở byte đầu tiên           |
| 4  | S->C 17 1E: 3B phản hồi kết quả (17 1E 01)          | shops.rs:631 gửi frame chuỗi dài tự chế               |
| 5  | C->S 19 02: 16B (gold + 5 cặp slot,qty)             | trade_storage.rs:335 parse payload[4..] thành slice slot|
| 6  | C->S 19 03: action = 0 là Huỷ, action = 1 là Chốt   | trade_storage.rs:371 bắt buộc action == 2 mới Huỷ     |
| 7  | Huỷ Trade: S->C 19 04 (Đóng form) hoặc 19 02 03     | trade_storage.rs:43 gửi F4440200173B (Cắt hàng!)      |
| 8  | S->C Kho 0x1E: 1E 07 rút đồ bank, 1E 06 thêm đồ     | trade_storage.rs:789 gửi 1E 05 rút, 1E 04 gửi         |
| 9  | Ba Đậu Yêu Lưu Lãng: 17 66/67/68 (10 ô player+0xF74) | trade_storage.rs:807 gửi 17 66, 813 gửi 17 68 (ĐÚNG)   |
| 10 | S->C 02 0B (2B): Mở form tạo Quân Đoàn (100k kim)   | shops.rs:37, chat.rs dùng 02 0B làm Toast chat đỏ     |
| 11 | C->S 1B 01: count u16 tối đa 1250                   | shops.rs:235 chặn count > 255                         |
| 12 | S->C 17 20: 7 byte (accId:u32 + state:u8)           | shops.rs:649 gửi 6 byte (thiếu state:u8)              |
| 13 | Mở Shop Mua chỉ gửi 14 01 (B2=6)                    | talk.rs:738 gửi thừa 1B 03 kích hoạt cả Form Bán      |
+----+-----------------------------------------------------+-------------------------------------------------------+
```

#### Chi tiết trích dẫn code sai lệch:

1. **Sai lệch Frame `1A 04` và Đảo lộn Sub-opcode `0x1A`**:
   - *Client Spec*: `1A 04` dài 6 byte (`F4 44 06 00 1A 04 <gold:u32 LE>`). Client handler `FUN_0079175f` chỉ đọc 4 byte vào `player+0x12F8`. Để cập nhật Điểm (`player+0x1300`), client cần frame `1A 03 <point:u32 LE>`.
   - *Server Code*: Tại [`shops.rs:11-18`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L11-L18) và [`spawn.rs:615-617`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L615-L617):
     ```rust
     pub fn gold_frame(gold: u32, shop_point: u32) -> String {
         let clamped_gold = gold.min(GOLD_CAP);
         format!(
             "F4440A001A04{}{}",
             encoder::le32(clamped_gold),
             encoder::le32(shop_point)
         )
     }
     ```
   - *Hậu quả*: Client bỏ qua 4 byte sau, điểm `shop_point` trên client không bao giờ được cập nhật, đồng thời wire thừa 4 byte. Tại [`protocol/mod.rs:171`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L171) khai báo nhầm `MONEY_SUB_CAPACITY_FULL = 0x03` (thực chất `1A 03` là Set Point) và `MONEY_SUB_SUB_POINTS = 0x06` (thực chất `1A 06` là Thông báo đầy tiền).

2. **Sai lệch nghiêm trọng Frame Mở sạp C→S & S→C `17 1E`**:
   - *Client Spec*: Client `0x007840A7..0x00784187` gửi: `17 1E` + 100 byte (10 items × 10B) + `boardName` + `cardIdx:u8` + `cardData:4B`. S→C phản hồi 3 byte: `F4440300171E01`.
   - *Server Code*: Tại [`shops.rs:572-585`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L572-L585):
     ```rust
     crate::protocol::ITEM_SUB_STALL_OPEN => {
         let Some(name_len) = payload.first().copied().map(usize::from) else {
             return;
         };
         let listings_start = name_len + 2;
         ...
     ```
   - *Hậu quả*: Server đọc byte đầu tiên (là `bagSlot` của ô 0) làm độ dài chuỗi tên bảng hiệu! Tính năng mở sạp của client chính thống sẽ sập hoặc bị reject 100%. Phản hồi S→C tại dòng 631 gửi chuỗi dài thay vì 3 byte khiến client không tắt được form setup sạp.

3. **Sai lệch cấu trúc Giao dịch P2P C→S `19 02` và `19 03`**:
   - *Client Spec*:
     - C→S `19 02` dài 16 byte: 4 byte kim + 10 byte (5 cặp `[slot:u8, qty:u8]`).
     - C→S `19 03`: `action = 1` là đồng ý, `action = 0` là huỷ.
     - Huỷ giao dịch: S→C gửi `19 04` (đóng form) hoặc `19 02 03`.
   - *Server Code*:
     - Tại [`trade_storage.rs:335`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L335): `for slot in &payload[4..]` coi mỗi byte là 1 slot độc lập, làm lệch toàn bộ số lượng vật phẩm.
     - Tại [`trade_storage.rs:371`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L371): `if action == 2` mới huỷ; client gửi `action = 0` bị server bỏ qua.
     - Tại [`trade_storage.rs:43-45`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L43-L45): gửi `F4440200173B` (Cắt hàng) để huỷ giao dịch!

4. **Sai lệch Opcode Kho / Tiền trang đồ `0x1E`**:
   - *Client Spec*:
     - `1E 04`: Xoá trắng 25 ô Kho nhà trọ (`FUN_00770528`).
     - `1E 05`: Đồng bộ toàn bộ 50 ô Tiền trang đồ (`FUN_007707a0`).
     - `1E 06`: Thêm vật phẩm vào Tiền trang đồ (`FUN_007701d8`).
     - `1E 07 <slot> <qty>`: Rút/xoá vật phẩm khỏi Tiền trang đồ (`FUN_007703c8`).
   - *Server Code*: Tại [`trade_storage.rs:788-817`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L788-L817):
     - Tiền trang rút đồ: server gửi `F44404001E05{:02X}{:02X}` (dùng nhầm lệnh bulk sync 50 ô thành lệnh trừ ô!).
     - Tiền trang gửi đồ: server gửi `frame("1E04", &entries)` (dùng nhầm lệnh xoá sạch kho nhà trọ thành lệnh nạp đồ!).
     - Ba Đậu Yêu Lưu Lãng (`storagetype = 16`): server gửi `17 66` khi gửi đồ và `17 68` khi rút đồ. **Đây là hành vi CHÍNH XÁC** đã được xác minh qua Decompile Client binary `FUN_0076a320` (`player+0xF74`, kho xe/ngựa 10 ô `form_Horse`), cần giữ lại và chuẩn hoá sức chứa tối đa là 10 ô.

5. **Lạm dụng Frame S→C `02 0B` làm Toast Chat**:
   - *Client Spec*: S→C `02 0B` dài 2 byte (`F4 44 02 00 02 0B`). Client tại `0078b6fc_FUN_0078b6fc.c:45` kiểm tra `player+0x12F8 >= 100.000` kim để mở Form tạo Quân Đoàn `TFCreateOrganize`.
   - *Server Code*: [`shops.rs:37-41`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L37-L41) và [`chat.rs:40`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/chat.rs#L40) gói chuỗi thông báo lỗi VISCII vào `020B + 00000000 + text`.
   - *Hậu quả*: Mỗi khi server báo lỗi shop/chat qua hàm này, client Delphi có nguy cơ giật popup tạo Quân Đoàn nếu tiền >= 100k, hoặc hiện banner "Số tiền trên người bạn không đủ nhé!".

---

### 3.3. Nhóm 3: Còn thiếu (Missing) — Chưa hiện thực

1. **Các frame điều khiển UI Tiền trang (`0x1D`)**:
   - Thiếu S→C `1D 07` (Xoá khoá nút Lưu trữ `form+0x11C = 0`).
   - Thiếu S→C `1D 08` (Xoá khoá nút Rút nhận `form+0x11D = 0`).
   - Thiếu S→C `1D 09 <mode:u8>` (Đồng bộ chế độ hiển thị `TThingAccountForm+0x418`).
2. **Kênh Ngân hàng Thủy binh & Đóng góp Quân Đoàn (`0x18`, C→S `0x1A`)**:
   - C→S `1A 01..04`: Đóng góp/rút quỹ Quân Đoàn từ các form `0x7D9C6C` và `0x7DA5C4` (hiện server đang route nhầm sang `npc_event::handle_pc_talk`).
   - C→S `18 01..02` & S→C `18 01..06`: Ngân hàng Thủy binh `TBankForm`.
3. **Quản lý Hệ số Giá Dịch vụ Nhà trọ (`23 04`)**:
   - Chưa hỗ trợ frame `23 04 <A:u8> <B:u8>` thiết lập hệ số tính tiền phòng/phục hồi `player+0x146C = A*100 + B`.
4. **Hệ thống Cờ bạc / Sòng bạc / Lotto (`0x36`, `0x37`, `0x39`–`0x3D`)**:
   - Toàn bộ nhóm opcode này đang để trống trong [`dispatcher.rs:377-391`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L377-L391). Đặc biệt thiếu cờ chặn giao dịch P2P khi đang mở `TSportManage` (`0x36`).
5. **Hệ thống Timeout tự động cho Giao dịch P2P**:
   - Client không có timer tự đóng form `TTradeForm`. Server đã có helper `is_trade_timed_out` (60s) trong `trade_storage.rs` nhưng chỉ kích hoạt khi có packet mới tới; thiếu background timer quét session treo.

---

## 4. BẢNG ĐỐI SOÁT 25 OPCODE LIÊN QUAN KINH TẾ

*(Ánh xạ 100% khớp với Mục 6.3.2 `OPCODE_MAP.csv`)*

| Opcode | Tên Nghiệp vụ | Chiều | Client Spec Expectation (Mã máy Delphi) | Hiện trạng Server Rust (`ts_dream`) | File Phụ Trách Chính | Mức Rủi Ro |
|:--:|:---|:--:|---|---|---|:--:|
| `0x01` | Khởi tạo Session / Tiền tệ | C↔S | Login nạp Gold (`+0x12F8`), Bank (`+0x12FC`), Point (`+0x1300`). Gửi `1A 04` (6B), `1D 04` (6B), `1A 03` (6B). | Gửi `1A 04` (10B ghép point), gửi `1D 04` (6B). Thiếu `1A 03`. | [`spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L615) | **P1** |
| `0x02` | Tạo Quân Đoàn (`02 0B`) | C↔S | `02 0B` S→C 2B: Kiểm tra 100k kim mở `TFCreateOrganize`. | Server dùng `020B` làm prefix chuỗi chat đỏ (Red Message). | [`shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L37), [`chat.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/chat.rs#L40) | **P1** |
| `0x08` | Tài nguyên Thủy binh / Quân đoàn | C↔S | `08 01` (Set), `08 06` (Delta) cho Thủy binh (`0x26`), Tài bảo (`0x27`), Đạn dược (`0x28`). | Chỉ mới xử lý chỉ số cơ bản nhân vật trong `handle_stat_allocation`. | [`stats.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/stats.rs) | **P2** |
| `0x0C` | Bản đồ / Chuyển cảnh | C↔S | Reset trạng thái giao dịch / thương nhân khi chuyển map. | Đã có logic reset session map trong movement. | [`movement.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs) | **P2** |
| `0x0F` | Quản lý Pet / Võ tướng | C↔S | Đổi tên, thả pet, cưỡi ngựa, gửi pet nhà trọ. | Xử lý tốt trong `pet_actions.rs`. | [`pet_actions.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs) | **P2** |
| `0x14` | Mở Shop NPC Mua (`B2=6`) | C↔S | S→C `14 01..06` (`B2=6`, `W3=surfaceIdx`). Client tự nạp Eve.emg + Item.Dat. | Đã gửi đúng `14 01` với `B2=6`, nhưng gửi thừa `1B 03` ở menu 30. | [`talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L738) | **P1** |
| `0x17` | Túi đồ, Sạp, Trứng, Cắt hàng | C↔S | S→C `17 06` (14B), S→C `17 21` (18B+16B/rec). C→S `17 1E` (100B items + name + card). | • `17 06`, `17 21` catalog: Khớp 100%.<br>• `17 1E` C→S và S→C: Sai hoàn toàn layout byte.<br>• `17 3B` bị dùng nhầm làm trade cancel. | [`inventory.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs), [`shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L572) | **P0** |
| `0x18` | Ngân hàng Thủy binh | C↔S | C→S `18 01..02` (5B). S→C `18 01..06` cập nhật kho lương/quỹ. | Chưa hiện thực, gom trong unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L367) | **P2** |
| `0x19` | Giao dịch 2 người chơi (P2P) | C↔S | C→S `19 02` (16B: gold + 5 cặp slot/qty). C→S `19 03` (1=Commit, 0=Cancel). S→C `19 04` đóng form. | • Parse `19 02` sai thành danh sách slot 1 byte.<br>• Parse `19 03` chờ `action == 2`.<br>• Gửi nhầm `17 3B` khi huỷ. | [`trade_storage.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L326), [`trade_system.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs) | **P0** |
| `0x1A` | Đồng bộ Kim & Điểm | C↔S | S→C `1A 01` (+Kim), `1A 02` (-Kim), `1A 03` (=Point), `1A 04` (=Kim), `1A 05` (+Point), `1A 06` (Đầy tiền). | `gold_frame` gửi 10B ghép chung gold+point. Đảo lộn hằng số `0x03` và `0x06` trong `protocol/mod.rs`. C→S 1A bị route nhầm sang pc_talk. | [`shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L11), [`protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L171) | **P0** |
| `0x1B` | Giao dịch Shop NPC Mua/Bán | C↔S | C→S `1B 01` (idx, count u16 <= 1250). C→S `1B 02` (slot 1..25, qty u8). S→C `1B 03` mở form bán. | Khớp wire mua/bán và mã kết quả. Bị lỗi ép trần mua `count <= 255`. Gửi thừa `1B 03` khi mở shop mua. | [`shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L223) | **P1** |
| `0x1D` | Tiền trang (Bank Ngân lượng) | C↔S | C→S `1D 01` Rút, `1D 02` Nộp. S→C `1D 01` (+Bank), `1D 02` (-Bank). S→C `1D 07/08` mở khoá nút. | Khớp logic đảo chiều Rút/Nộp. Thiếu S→C `1D 07` và `1D 08` mở khoá UI nút bấm. | [`trade_storage.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L836) | **P1** |
| `0x1E` | Kho đồ / Tiền trang vật phẩm | C↔S | S→C `1E 05` (Sync 50 ô Bank), `1E 06` (Thêm ô Bank), `1E 07` (Xoá ô Bank), `1E 04` (Xoá kho nhà trọ). | Lệch opcode Tiền trang (dùng 1E 05 để xoá ô, 1E 04 để thêm ô). Giữ lại 17 66/67/68 cho Ba Đậu Yêu Lưu Lãng. | [`trade_storage.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L654) | **P0** |
| `0x23` | Quản lý Quân Đoàn / Giá trọ | C↔S | S→C `23 04 <A><B>`: Thiết lập hệ số giá nhà trọ/thương nhân `player+0x146C = A*100 + B`. | Chỉ dùng cho Account Management (đổi pass, xoá char), chưa có `23 04`. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L344) | **P2** |
| `0x2C` | Vòng quay kỹ năng chiến đấu | C↔S | `TFWheel`: Vòng quay kỹ năng trong trận đánh. Không phải sòng bạc, không phải Pet Reborn. | Khai báo nhầm thành `OP_REBORN_PET` và dispatch vào `handle_pet_reborn`. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L353) | **P2** |
| `0x35` | Bảng trạng thái Mini-game | **Chỉ S→C** | C→S trỏ về Epilogue RET (`0x0078A4F2`). S→C cập nhật trạng thái bảng cờ/thể thao. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L393) | **P2** |
| `0x36` | Khu giải trí `TSportManage` | C↔S | C→S `36` (1B). S→C `36 <mode>` mở/đóng. Khi mở, client cấm giao dịch P2P. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L378) | **P2** |
| `0x37` | Máy đánh bạc / Quay thưởng | C↔S | C→S `37 <sub> <text>`. S→C `37 01..05`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L393) | **P2** |
| `0x38` | Kết quả ván cược | **Chỉ S→C** | C→S trỏ về Epilogue RET. S→C gửi kết quả cược hiển thị banner. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L393) | **P2** |
| `0x39` | Máy Slot / Đua ngựa | C↔S | `TMachineManager`: C→S `39 01`. S→C `39 01..0A`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L390) | **P2** |
| `0x3A` | Cược Tài Xỉu (`TRE_BiDaXiao`) | C↔S | C→S `3A <side:u8> <wager:u32 LE>`. S→C `3A 01..03`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L379) | **P2** |
| `0x3B` | Domino (`TSBDManager`) | C↔S | C→S `3B <sub:u8> <val:u32 LE>`. S→C `3B 01..03`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L380) | **P2** |
| `0x3C` | Cờ tướng (`TRE_ZMChessMain`) | C↔S | C→S `3C 01..03` (Nước đi, Hoà/Hàng, Rời bàn). S→C `3C 01..05`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L381) | **P2** |
| `0x3D` | Mua vé số Lotto | C↔S | C→S `3D <sub> <5 số:u8>`. S→C `3D 01..06`. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L382) | **P2** |
| `0x3E` | Kết quả quay số Lotto | **Chỉ S→C** | C→S trỏ về Epilogue RET. S→C `3E 01..02` công bố giải Jackpot. | Chưa hiện thực, gom vào unimplemented. | [`dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L393) | **P2** |

---

## 5. ĐÁNH GIÁ HỆ THỐNG TEST HIỆN HÀNH (`economy_wire_test.rs`)

Tập test `tests/economy_wire_test.rs` (1.319 dòng) có giá trị kiểm thử hồi quy cục bộ tốt, nhưng **đang chứa đựng nhiều assertion củng cố các quan niệm sai lầm từ server C# Bear cũ**:

1. **Test 1 ([`line 74 & 102`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs#L74))**: Assert frame `1A 04` chứa 10 byte (`F4440A001A04 + gold + shop_point`). Cần cập nhật thành: `1A 04` gửi 6 byte gold (`F44406001A04`), và `1A 03` gửi 6 byte point (`F44406001A03`).
2. **Test 6 ([`line 728 & 756`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs#L728))**: Assert `1E 04` để thêm đồ vào bank và `1E 05` để xoá đồ khỏi bank. Cần cập nhật thành: `1E 06` thêm đồ bank 50 ô, `1E 07` xoá đồ bank 50 ô, `1E 05` bulk sync 50 ô.
3. **Test 6 ([`line 828`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs#L828))**: Assert frame `17 68` cho Luulang. Đây là opcode không tồn tại trong client PC, cần loại bỏ.
4. **Test 8 ([`line 1012 & 1069`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs#L1012))**: Giả lập C→S cancel trade với `action == 2` và assert S→C `17 3B`. Cần cập nhật thành: C→S cancel gửi `action == 0`, và server phản hồi S→C `19 04` hoặc `19 02 03`.

---

## 6. KẾT LUẬN & ĐỀ XUẤT LỘ TRÌNH KHẮC PHỤC (ACTIONABLE ROADMAP)

Để đưa server Rust đạt chuẩn **100% Wire Parity** với client TS Online Delphi mà không gây hồi quy (regression), lộ trình được chia thành 4 giai đoạn ưu tiên theo mức độ rủi ro:

```
               LỘ TRÌNH TÁI THIẾT GIAO THỨC KINH TẾ (ROADMAP)
   
  [ PHASE 1: SỬA LỖI P0 WIRE CORRUPTION ] ---> [ PHASE 2: SỬA LỖI P1 UI & TOAST ]
  - Khắc phục C->S & S->C 17 1E Mở sạp          - Tách 1A 04 (Gold) & 1A 03 (Point)
  - Khắc phục C->S 19 02 & 19 03 Trade P2P      - Thêm 1D 07 & 1D 08 mở khoá nút Bank
  - Chuẩn hoá Opcode Kho 0x1E (1E 05..07)       - Bỏ lạm dụng 02 0B làm Red Message
  - Bỏ 17 3B làm Trade Cancel (dùng 19 04)      - Sửa trần Mua NPC Shop 1250 (1B 01)
                    |                                          |
                    v                                          v
  [ PHASE 3: HOÀN THIỆN TEST PARITY ]      ---> [ PHASE 4: TÍNH NĂNG MỞ RỘNG P2 ]
  - Viết lại tests/economy_wire_test.rs         - Hệ số giá Nhà trọ 23 04
  - Loại bỏ các assertion sai lệch cũ           - Ngân hàng Thủy binh 0x18, C->S 0x1A
  - Bổ sung Golden Packet diffing               - Cụm Opcode Sòng bạc / Lotto 0x36..3E
```

### Chi tiết các bước thực hiện:

#### Giai đoạn 1: Khắc phục các lỗi Wire biến dạng dữ liệu (Ưu tiên P0 — Nguy cơ sập client / Dupe đồ)
1. **Sạp hàng người chơi (`17 1E`)**:
   - Viết lại bộ giải mã C→S `ITEM_SUB_STALL_OPEN` trong `shops.rs`: Đọc đúng 100 byte payload mảng 10 ô trước (`bagSlot: u8`, `qty: u8`, `price: u32 LE`, `vitality: u32 LE`), sau đó mới đọc chuỗi tên bảng hiệu `boardName` và thẻ sạp.
   - Sửa frame S→C phản hồi mở sạp: Gửi đúng 3 byte `F4440300171E01` khi thành công, hoặc `02/03` khi lỗi.
2. **Giao dịch 2 người chơi (`0x19`)**:
   - Sửa parser C→S `19 02`: Đọc `gold: u32 LE` + 5 cặp `(bagSlot: u8, qty: u8)` (tổng 16 byte payload). Bổ sung kiểm tra trùng `bagSlot` chống cheat dupe.
   - Sửa parser C→S `19 03`: Chấp nhận `action = 0` là Huỷ giao dịch, `action = 1` là Xác nhận giao dịch.
   - Sửa frame S→C huỷ giao dịch: Thay thế hoàn toàn `17 3B` bằng `19 04` (đóng form) kết hợp `19 02 03` (đối phương huỷ).
3. **Chuẩn hoá Kho / Tiền trang đồ (`0x1E`)**:
   - Chuyển lệnh rút vật phẩm Tiền trang 50 ô sang S→C `1E 07 <slot:u8> <qty:u8>`.
   - Chuyển lệnh thêm vật phẩm Tiền trang 50 ô sang S→C `1E 06 <itemId:u16> <qty:u8> <attrs:4B> <exp:u32>`.
   - Chuyển lệnh đồng bộ toàn bộ Tiền trang 50 ô sang S→C `1E 05` (bulk sync).
   - Xoá bỏ các mã opcode tự chế `17 66` và `17 68`.

#### Giai đoạn 2: Sửa lỗi Đồng bộ UI, Tiền tệ & Mã lỗi (Ưu tiên P1 — Nguy cơ lệch UI / Treo giao diện)
1. **Tách biệt Frame Kim (`1A 04`) và Điểm (`1A 03`)**:
   - Sửa hàm `gold_frame` thành frame 6 byte chuẩn `F44406001A04<gold:u32 LE>`.
   - Bổ sung hàm `point_frame(point: u32) -> String` gửi frame 6 byte `F44406001A03<point:u32 LE>`.
   - Sửa đổi `spawn.rs`: Khi nhân vật login, gửi độc lập cả `1A 04` (Kim) và `1A 03` (Điểm).
   - Cập nhật định nghĩa trong `protocol/mod.rs`: Sửa `MONEY_SUB_SET_POINT = 0x03` và `MONEY_SUB_CAPACITY_FULL = 0x06`.
2. **Bổ sung Frame Mở khoá UI Ngân hàng (`1D 07 / 08`)**:
   - Trong `handle_bank_gold`, sau khi nạp/rút thành công hoặc thất bại, gửi kèm S→C `F44402001D07` (nếu nộp) hoặc `F44402001D08` (nếu rút) để client mở khoá nút bấm.
   - Gửi `1A 06` khi không thể rút do kim trên người vượt trần.
3. **Chấm dứt lạm dụng `02 0B` làm Toast Chat**:
   - Chuyển đổi toàn bộ các lời gọi `red_message` (`02 0B`) sang cơ chế Chat System chuẩn hoặc mã lỗi nghiệp vụ của opcode tương ứng (ví dụ dùng `1B 01/02 <code>` cho shop, hoặc kênh chat `02 01`).
4. **Mở rộng Giới hạn Mua Shop NPC (`1B 01`)**:
   - Cho phép `count` lên tới `1250` trong `handle_npc_shop`, tự động phân tách thành nhiều slot trong túi (mỗi slot tối đa 50 món).
   - Loại bỏ việc gửi thừa `1B 03` khi mở Shop Mua ở `talk.rs`.

#### Giai đoạn 3: Tái cấu trúc Suite Test (`economy_wire_test.rs`)
1. Cập nhật lại toàn bộ các unit test và integration test trong `tests/economy_wire_test.rs` để assert theo cấu trúc wire chuẩn hoá mới (Test 1, Test 6, Test 7, Test 8).
2. Đảm bảo chạy `cargo test --test economy_wire_test --no-fail-fast` vượt qua 100% mà không còn bất kỳ dấu vết nào của các giao thức C# Bear cũ.

#### Giai đoạn 4: Mở rộng tính năng Kinh tế phụ (Ưu tiên P2)
1. Thêm xử lý S→C `23 04 <A:u8><B:u8>` lưu hệ số giá trọ vào `conn.session.inn_rate`.
2. Hỗ trợ kênh đóng góp Quân Đoàn C→S `1A 01..04` và Ngân hàng Thủy binh `0x18`.
3. Khung sườn mini-games / cờ bạc `0x36..0x3E` với cờ khoá giao dịch an toàn.
