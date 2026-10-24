# 10: Bank Withdraw Overflow Error Parity & Economy Code Cleanliness (0x1D, 0x1A)

**What to build:**
Hoàn thiện logic báo lỗi rút tiền ngân hàng và dọn dẹp các điểm duplicate / code smells trong hệ thống kinh tế:
1. **Phân biệt Chính xác Lỗi Rút Tiền Ngân Hàng (`handle_bank_gold`)**:
   - Khi người chơi rút tiền từ ngân hàng (`BANK_SUB_WITHDRAW` = 1):
     - Nếu số dư ngân hàng không đủ (`bank_gold < amount`): Server chỉ gửi frame báo lỗi `1D 03` (`F4 44 02 00 1D 03`) kèm frame mở khóa nút bấm `1D 08`. Tuyệt đối không gửi frame `1A 06`.
     - Nếu số dư ngân hàng đủ nhưng số dư túi sau khi rút sẽ vượt trần (`gold.saturating_add(amount) > GOLD_CAP`): Server gửi frame báo lỗi `1D 03`, frame mở khóa nút bấm `1D 08`, VÀ frame thông báo tràn vàng túi `1A 06` (`F4 44 02 00 1A 06`).
2. **Khử Duplicate & Primitive Obsession trong Bank Handler**:
   - Viết helper chuyên biệt `send_bank_ui_unlock(sub: u8, out: &mut HandleOutcome)` để phát frame `1D 08` (cho rút tiền) hoặc `1D 07` (cho nộp tiền), thay thế 4 đoạn `if ctx.sub == 1 { ... } else { ... }` trùng lặp trong `handle_bank_gold`.
   - Thay thế các magic literal `1` trong so sánh `ctx.sub` bằng hằng số định danh miền `crate::protocol::BANK_SUB_WITHDRAW`.
3. **Hợp nhất Helper Gửi Tin nhắn Hệ thống Kênh Chat `020102`**:
   - Hợp nhất các hàm định dạng tin nhắn hệ thống VISCII qua kênh `02 01 02` (`red_message` trong `shops.rs`, `red` trong `use_item/mod.rs`, `sys_msg_frame` trong `spawn.rs`) sử dụng chung một hàm chuẩn hóa duy nhất tại `src/server/spawn.rs::sys_msg_frame`.
4. **Integration Tests**: Bổ sung test case phân biệt rõ giữa trường hợp rút tiền thiếu số dư ngân hàng (chỉ nhận `1D 03`, không nhận `1A 06`) và trường hợp rút tiền làm tràn ví (nhận cả `1D 03` và `1A 06`).

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Sửa `src/server/handlers/trade_storage.rs` (hàm `handle_bank_gold`): Tách biệt điều kiện kiểm tra hợp lệ; chỉ phát `F44402001A06` khi và chỉ khi ngân hàng đủ tiền (`bank_gold >= amount`) nhưng ví tiền vượt trần (`gold.saturating_add(amount) > GOLD_CAP`).
- [x] Tạo helper `send_bank_ui_unlock(sub, out)` trong `src/server/handlers/trade_storage.rs`, loại bỏ 4 khối `if ctx.sub == 1` lặp lại và sử dụng hằng số `crate::protocol::BANK_SUB_WITHDRAW`.
- [x] Thống nhất các hàm định dạng tin nhắn hệ thống VISCII qua kênh `02 01 02` sử dụng `crate::server::spawn::sys_msg_frame(msg)`, loại bỏ mã trùng lặp ở `shops.rs` và `use_item/mod.rs`.
- [x] Bổ sung bài test trong `tests/economy_wire_test.rs`:
  - Rút tiền khi ngân hàng không đủ: Kiểm tra chỉ nhận `1D 03` và `1D 08`, khẳng định KHÔNG xuất hiện frame `1A 06`.
  - Rút tiền khi ngân hàng đủ nhưng túi tràn: Kiểm tra nhận đủ `1D 03`, `1D 08`, và `1A 06`.
- [x] `cargo check --all-targets` và `cargo test --all-targets` đạt 100% PASS (0 error, 0 warning).
