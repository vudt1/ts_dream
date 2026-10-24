# 07 — Comprehensive Wire Integration Test Suite Refactoring

Status: completed
Blocked by: 02, 03, 04, 05, 06

---

## 1. Mục tiêu
Tái cấu trúc toàn bộ tập test tích hợp giao thức kinh tế tại `tests/economy_wire_test.rs`:
- Loại bỏ hoàn toàn các assertion cũ sai lệch (assert `1A 04` 10 byte, assert `1E 04/05` kho cũ, assert `17 3B` cancel trade).
- Viết mới các bài test tích hợp toàn diện bao phủ toàn bộ 6 tickets (01 đến 06).
- Chạy trên SQLite In-Memory (`sqlite::memory:?cache=shared`), độc lập hoàn toàn với database sản xuất, 0 test inline trong `src/`.

---

## 2. Danh mục Bài test cần triển khai

1. **`test_currency_wire_sync_login`**:
   - Xác nhận login sequence phát `1A 04` đúng 6 byte (`F44406001A04<gold: u32 LE>`), `1D 04` đúng 6 byte, và `1A 03` đúng 6 byte.
2. **`test_bank_deposit_withdraw_with_ui_unlock`**:
   - Gửi C→S `1D 01` (Rút): Kiểm tra S→C `1D 02` + `1A 01` + `1D 08` (mở khóa rút).
   - Gửi C→S `1D 02` (Nộp): Kiểm tra S→C `1D 01` + `1A 02` + `1D 07` (mở khóa nộp).
   - Kiểm tra chặn trần `GOLD_CAP = 9.999.999`.
3. **`test_player_stall_open_and_catalog_buy`**:
   - Giả lập C→S `17 1E` với 100 byte danh mục trước rồi mới đến tên bảng hiệu: Xác nhận S→C nhận đúng 3 byte `F4440300171E01`.
   - Giả lập người chơi khác xem sạp: Kiểm tra S→C `17 21` đúng header 18B + records 16B.
   - Mua hàng từ sạp 1-based: Kiểm tra trừ tiền người mua, cộng tiền người bán và gỡ bỏ listing khi hết hàng.
4. **`test_p2p_trade_two_phase_atomic_flow`**:
   - Giả lập 2 session A và B: Gửi `19 02` với 5 cặp `(slot, qty)`, xác nhận S→C nhận 11B/record.
   - Xác nhận chốt deal `19 03 01` ở cả 2 bên: Kiểm tra hoán đổi vật phẩm nguyên tử.
5. **`test_p2p_trade_cancel_wire_safety`**:
   - Giả lập bên A gửi `19 03 00` (Hủy deal): Kiểm tra server phát S→C `19 04` đóng form an toàn và toast `19 02 03`, tuyệt đối không phát `17 3B`.
6. **`test_npc_shop_bulk_buy_all_or_nothing`**:
   - Mua 100 món khi túi có 2 ô trống: Thành công, phát 2 gói `17 06`.
   - Mua 100 món khi túi chỉ có 1 ô trống: Thất bại, trả lỗi `1B 01 02` (`NPC_SHOP_BUY_BAG_FULL`), vàng giữ nguyên 100%.
   - Mua 1.250 món khi túi đủ 25 ô trống: Thành công.
7. **`test_npc_dialogue_menu_shop_segregation`**:
   - Chọn menu 30 (Mua hàng): Khẳng định chỉ phát `14 01` (`B2 = 6`), không phát `1B 03`.
8. **`test_five_containers_wire_and_capacity`**:
   - Kiểm tra `luulang` đúng giới hạn 10 ô, phát `17 66` và `17 68`.
   - Kiểm tra `tientrang` luôn phát mass-clear `1E 08` trước khi dump `1E 01`, rút phát `1E 07`.
   - Kiểm tra `trangbi` phát 10 byte/record không có byte slot.
9. **`test_zero_020b_red_message_toast`**:
   - Quét toàn bộ output của tất cả các kịch bản lỗi: Khẳng định không có frame nào chứa prefix `020B`.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. `cargo test --test economy_wire_test --no-fail-fast` chạy thành công 100% (tất cả 9 bài test đều PASS).
2. Toàn bộ test suite repository `cargo test --all-targets --no-fail-fast` PASS 100%, 0 regression.
3. Không ghi đè hoặc tạo rác vào tệp cơ sở dữ liệu sản xuất `DB/ts_dream.db`.

---

- Implemented all wire-accurate integration tests in `tests/economy_wire_test.rs` matching Delphi client specifications (17 tests total).
- Updated `tests/inventory_storage_type_test.rs` and `tests/login_char_flow_test.rs` for the separated `1A 03` Shop Point frame (+1 frame in spawn packet count: 22 -> 23).
- Updated `tests/chat_test.rs` to assert `020102` system channel instead of deprecated `020B`.
- Added edge case test `test_player_stall_open_rejects_exceeding_gold_cap` and verified Two-Phase State Lock on stall and pet trades.
- Verified `cargo test --all-targets --no-fail-fast` passes completely with 0 failures, 0 regressions across all test suites.
