# 03 — P2P Trade Protocol Parity & Two-Phase State Lock

Status: completed
Blocked by: 01

---

## 1. Mục tiêu
Sửa toàn diện các sai lệch wire trong Giao dịch 2 người chơi (P2P Trade - Opcode `0x19`):
- Sửa parser C→S `19 02` (Offer): Đọc đúng 16 byte payload gồm 4 byte kim và 5 cặp `(bagSlot: u8, qty: u8)`.
- Sửa parser C→S `19 03` (Confirm/Cancel): Nhận diện đúng `action = 1` là Xác nhận, `action = 0` là Hủy bỏ.
- Thay thế frame hủy giao dịch: Phát S→C `19 04` đóng form giao dịch an toàn và S→C `19 02 03` (đối tác hủy), xóa bỏ hoàn toàn frame `17 3B` (Cắt hàng).
- Triển khai cơ chế **Two-Phase State Lock** (khóa RAM theo `player_id` tăng dần -> SQLite Write Transaction) để chống race condition và dupe đồ.

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 C→S `19 02` (Đưa đồ và tiền lên bàn giao dịch)
- Payload 16 byte:
  - Byte `0..3`: `gold: u32 LE` (số vàng đặt cọc).
  - Byte `4..13`: 5 cặp `(bagSlot: u8, qty: u8)` (mỗi slot 1..25, nếu không đặt thì slot = 0, qty = 0).
  - Byte `14..15`: Padding / zero bytes nếu có.
- Server validation:
  - Kiểm tra `gold <= player_gold`.
  - Kiểm tra các `bagSlot != 0`: Phải thuộc `1..25`, không được trùng lặp slot trong cùng 1 request (chống cheat gửi 5 lần cùng 1 slot), và `qty <= item.count`.
- Phản hồi cho đối tác qua S→C `19 03`:
  - 4 byte kim đối tác (`u32 LE`) + tối đa 5 record 11 byte (`[itemId: u16 LE][qty: u8][attrs: 4B][exp: u32 LE]`).

### 2.2 C→S `19 03` (Xác nhận hoặc Hủy bỏ)
- Byte `0`: `action: u8`
  - `action == 1`: Người chơi bấm nút Khóa / Xác nhận giao dịch.
  - `action == 0`: Người chơi bấm nút Hủy bỏ giao dịch.
- Xử lý khi Hủy bỏ:
  - Clear trạng thái `TradeState` của cả 2 session.
  - Gửi cho cả 2 client frame S→C `F4 44 02 00 19 04` (Đóng form `TTradeForm`).
  - Gửi cho đối tác frame S→C `F4 44 03 00 19 02 03` (Toast thông báo "Đối phương đã hủy bỏ giao dịch").
  - Tuyệt đối không gửi gói tin `17 3B`.

### 2.3 Two-Phase State Lock Execution
- Khi cả 2 bên cùng gửi `action == 1`:
  - Bước 1 (RAM Lock): Lấy đồng thời lock của `Session` player A và player B theo thứ tự `min(idA, idB)` trước, `max(idA, idB)` sau.
  - Bước 2: Kiểm tra lại toàn bộ điều kiện túi đồ và vàng của cả 2 bên.
  - Bước 3 (DB Transaction): Mở transaction trên `write_pool` SQLite, cập nhật `character_money` và hoán đổi `inventories` nguyên tử.
  - Bước 4: Cập nhật RAM state, phát các frame S→C `17 06` (thêm đồ), `17 09` (trừ đồ), `1A 01/02` (vàng), và `19 04` đóng form giao dịch thành công.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. C→S `19 02` parse chính xác 5 cặp `(slot, qty)`, reject ngay nếu có slot trùng lặp.
2. C→S `19 03` với `action = 0` đóng form giao dịch an toàn bằng S→C `19 04`, không gửi `17 3B`.
3. Đối tác nhận được toast `19 02 03` khi giao dịch bị hủy.
4. Giao dịch thành công hoán đổi đồ và vàng nguyên tử, an toàn tuyệt đối trước race condition.
5. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

## Comments
- Đã sửa parser `TRADE_SUB_OFFER_CS` trong `src/server/handlers/trade_storage.rs`: đọc đúng cấu trúc 5 cặp `(slot, qty)` kèm kiểm tra chống trùng lặp slot và `qty <= item.count`.
- Đã cập nhật `TRADE_SUB_CONFIRM_CS`: nhận diện `action == 0` (và 2) để hủy giao dịch an toàn với `19 04` và gửi toast `19 02 03` cho đối tác.
- Đã thay đổi `trade_cancelled_frame()` trả về `F44402001904`, loại bỏ hoàn toàn `17 3B` khỏi toàn bộ luồng hủy trade / pet trade / timeout trade.
- Đã tích hợp Two-Phase State Lock với `lock_player_operations([id, partner_id])` khóa theo ID tăng dần cho cả giao dịch vật phẩm và giao dịch võ tướng (pet trade).
- Xử lý hoàn trả an toàn khi DB transaction commit thất bại trong `TRADE_SUB_CONFIRM_CS`: reset sạch trạng thái `trade = TradeState::default()` và phát frame lỗi `19 02 07` kèm `19 04` cho cả 2 người chơi.
- Cập nhật test `tests/economy_wire_test.rs`, toàn bộ test pass.
