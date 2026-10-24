# 01 — Currency & Bank Protocol Wire Parity

Status: completed
Blocked by:

---

## 1. Mục tiêu
Chuẩn hóa các gói tin đồng bộ tiền tệ (`0x1A`) và ngân hàng (`0x1D`) để đạt 100% wire parity với Client Delphi:
- Tách biệt hoàn toàn Vàng (`1A 04`, 6 byte) và Điểm tích lũy (`1A 03`, 6 byte).
- Sửa định nghĩa sai lệch sub-opcode của `0x1A` trong protocol hằng số (`MONEY_SUB_SET_POINT = 0x03`, `MONEY_SUB_CAPACITY_FULL = 0x06`).
- Bổ sung frame mở khóa nút bấm UI Tiền Trang `1D 07` (khi nộp) và `1D 08` (khi rút) để giao diện client không bị đơ sau khi bấm.
- Phản hồi mã lỗi `1A 06` khi số vàng trên người vượt trần `9.999.999`.

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 Tiền tệ S→C (`0x1A`)
- `gold_frame(gold: u32) -> String`:
  - Wire: `F4 44 06 00 1A 04 <gold: u32 LE>` (đúng 6 byte payload).
  - Không ghép 4 byte `shop_point` vào sau `gold`.
- `point_frame(point: u32) -> String`:
  - Wire: `F4 44 06 00 1A 03 <point: u32 LE>` (đúng 6 byte payload).
- Tại `src/server/spawn.rs`: Khi gửi dữ liệu đăng nhập ban đầu, gửi độc lập cả 2 frame `1A 04` và `1A 03`.
- Tại `src/protocol/mod.rs`:
  - Sửa `MONEY_SUB_SET_POINT = 0x03` (thay vì gán 0x06).
  - Sửa `MONEY_SUB_CAPACITY_FULL = 0x06` (thay vì gán 0x03).

### 2.2 Tiền Trang S→C (`0x1D`)
- Sau khi xử lý C→S `1D 01` (Rút tiền: Bank → Túi):
  - Thành công: Gửi S→C `1D 02 <amount: u32 LE>` + S→C `1A 01 <amount: u32 LE>` + S→C `1D 08` (mở khóa nút Rút `form+0x11D = 0`).
  - Thất bại hoặc vượt trần: Gửi S→C `1D 03` + S→C `1D 08`. Nếu túi đầy vàng (>= 9.999.999), gửi thêm S→C `1A 06`.
- Sau khi xử lý C→S `1D 02` (Nộp tiền: Túi → Bank):
  - Thành công: Gửi S→C `1D 01 <amount: u32 LE>` + S→C `1A 02 <amount: u32 LE>` + S→C `1D 07` (mở khóa nút Nộp `form+0x11C = 0`).
  - Thất bại hoặc vượt trần: Gửi S→C `1D 03` + S→C `1D 07`.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. `gold_frame` trả về đúng 6 byte payload (`F44406001A04...`), không còn 10 byte.
2. `point_frame` trả về đúng 6 byte payload (`F44406001A03...`).
3. Mọi thao tác nộp tiền thành công hoặc thất bại đều phát ra frame `F44402001D07`.
4. Mọi thao tác rút tiền thành công hoặc thất bại đều phát ra frame `F44402001D08`.
5. Login sequence phát đầy đủ cả `1A 04`, `1D 04`, và `1A 03`.
6. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

## Comments
- Đã sửa constants trong `src/protocol/mod.rs`: `MONEY_SUB_SET_POINT = 0x03`, `MONEY_SUB_CAPACITY_FULL = 0x06`.
- Đã chuẩn hóa `gold_frame(gold: u32) -> String` thành frame 6 byte (`1A 04`) và thêm `point_frame(point: u32) -> String` thành frame 6 byte (`1A 03`).
- Đã cập nhật `spawn.rs` Step 12 để gửi độc lập `1A 04` (Gold), `1D 04` (Bank Gold) và `1A 03` (Shop Point).
- Đã cập nhật `handle_bank_gold` trong `src/server/handlers/trade_storage.rs` để gửi frame mở khóa UI `1D 07` (nộp) và `1D 08` (rút) cho cả nhánh thành công và thất bại; gửi `1A 06` khi số vàng trên người vượt trần.
- Toàn bộ 9 test trong `tests/economy_wire_test.rs` đều PASS.
