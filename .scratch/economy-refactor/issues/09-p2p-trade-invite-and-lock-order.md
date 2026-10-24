# 09: P2P Trade Invite Error Wire Parity & State Lock Ordering (0x19)

**What to build:**
Hoàn thiện giao thức Giao dịch Trực tiếp giữa 2 người chơi (`OP_TRADE = 0x19`) nhằm đảm bảo wire parity với Client Delphi và triệt tiêu triệt để nguy cơ race condition / state drift:
1. **Phản hồi Lỗi Mời Giao dịch (`19 01`)**: Khi người chơi gửi lời mời giao dịch (`TRADE_SUB_INVITE_CS = 0x01`), nếu đối tác không thể nhận lời mời, server phải trả về mã lỗi native theo đúng ADR 0005 §2 thay vì silent drop:
   - Nếu đối tác không online trong registry: Gửi phản hồi S→C `19 02 01` (`F4 44 03 00 19 02 01`) thông báo đối phương offline.
   - Nếu đối tác đang trong trạng thái bận giao dịch (`partner.trade.active`): Gửi phản hồi S→C `19 02 02` (`F4 44 03 00 19 02 02`) thông báo đối phương đang bận.
2. **Toast Hủy Giao dịch do Timeout**: Khi một phiên giao dịch P2P bị quá thời gian chờ (timeout) trong `cancel_timed_out_trade`, server phải gửi đồng thời cả frame đóng form `19 04` (`F4 44 02 00 19 04`) và toast thông báo hủy giao dịch `19 02 03` (`F4 44 03 00 19 02 03`) cho cả hai bên người chơi (theo đúng yêu cầu User Story 23 & 24 trong `spec.md`).
3. **Thứ tự Khóa Two-Phase State Lock chuẩn mực**: Sửa đổi luồng xử lý xác nhận và xử lý giao dịch P2P trong `src/server/handlers/trade_storage.rs`: Phải luôn luôn lấy khóa RAM hai bên theo thứ tự tăng dần `lock_player_operations([id, partner_id])` TRƯỚC KHI đọc dữ liệu phiên `partner` từ session registry. Không đọc dữ liệu trước khi khóa để ngăn chặn nguy cơ state drift giữa thời điểm đọc và thời điểm có khóa.
4. **Integration Tests**: Bổ sung các bài test tự động bao phủ luồng từ chối mời giao dịch (offline và busy), timeout toast, và kiểm tra tính toàn vẹn trạng thái sau khi khóa.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Sửa `src/server/handlers/trade_storage.rs` (nhánh `TRADE_SUB_INVITE_CS`): Trả về frame `19 02 01` khi đối tác offline, và `19 02 02` khi đối tác đang bận (`partner.trade.active`), thay thế cho `return;` im lặng. Khai báo alias `TRADE_SUB_INVITE_CS = 0x01` trong `src/protocol/mod.rs`.
- [x] Sửa `src/server/handlers/trade_storage.rs` (hàm `cancel_timed_out_trade`): Bổ sung gửi frame toast `19 02 03` đến cả 2 bên cùng với frame đóng giao dịch `19 04`. Thêm lock hai bên trước khi thao tác session.
- [x] Tái cấu trúc hàm xử lý giao dịch trong `src/server/handlers/trade_storage.rs`: Đảm bảo `lock_player_operations([id, partner_id])` được gọi trước khi đọc trạng thái session của partner từ session registry trong cả `TRADE_SUB_CONFIRM_CS` và `TRADE_SUB_PET_CONFIRM_CS`.
- [x] Bổ sung các bài test vào `tests/economy_wire_test.rs`:
  - Test gửi lời mời giao dịch tới người chơi offline nhận `19 02 01`.
  - Test gửi lời mời giao dịch tới người chơi đang bận nhận `19 02 02`.
  - Test timeout giao dịch nhận cả `19 04` và `19 02 03`.
- [x] `cargo check --all-targets` và `cargo test --test economy_wire_test` đạt 100% PASS (0 error, 0 warning; 21/21 test passing).

### Implementation Notes:
- **`src/protocol/mod.rs`**: Thêm hằng số alias `TRADE_SUB_INVITE_CS = 0x01`.
- **`src/server/handlers/trade_storage.rs`**:
  - `cancel_timed_out_trade`: Khóa `lock_player_operations` hai bên khi `partner_id > 0` (hoặc `session.id` khi solo), gửi đồng thời `19 04` và `19 02 03` toast tới cả local outcome và partner hub. Đồng bộ session về registry.
  - `TRADE_SUB_REQUEST` & `TRADE_SUB_PET_REQUEST`: Bổ sung kiểm tra `partner_id == 0 || partner_id == id || ctx.conn.session.trade.active` trả về `19 02 02` (busy) ngay từ đầu. Đồng thời áp dụng `lock_player_operations([id, partner_id]).await` TRƯỚC KHI truy vấn `registry_get(partner_id)`, triệt tiêu hoàn toàn race condition khi nhiều người cùng mời 1 mục tiêu.
  - `TRADE_SUB_CONFIRM_CS` & `TRADE_SUB_PET_CONFIRM_CS`: Gọi `lock_player_operations([id, partner_id]).await` trước khi đọc partner registry. Nếu partner bị ngắt kết nối (`None`), lập tức giải phóng `trade.active = false`, gửi `19 04` và `19 02 01` (offline toast) để unfreeze giao diện client.
  - Bổ sung `registry_update_if_online(&ctx.conn.session)` trong toàn bộ các nhánh giao dịch (invite, offer, accept, settle, transfer_item) đảm bảo các session khác và background tasks luôn đọc được trạng thái `trade.accepted` và inventory mới nhất.
- **`tests/economy_wire_test.rs`**:
  - Cập nhật Test 8 để assert cả `F4440300190203` và `F44402001904`.
  - Cập nhật Test 21 (`test_p2p_trade_invite_error_wire_parity`): Kiểm thử invite target offline nhận `19 02 01`, invite target bận nhận `19 02 02`, invite self (kể cả khi không có trong registry) nhận `19 02 02`, và invite khi sender đang bận trade nhận `19 02 02`.
  - Thêm Test 22 (`test_p2p_trade_timeout_emits_close_and_toast_both_players`): Đăng ký partner qua `ServerControl`, kích hoạt timeout và assert cả local session lẫn partner nhận đủ cặp `19 04` + `19 02 03`.
  - Thêm Test 23 (`test_p2p_trade_natural_flow_and_partner_offline_recovery`): Kiểm thử toàn diện quy trình giao dịch P2P giữa 2 người chơi tự nhiên (invite -> offer items + gold -> confirm -> confirm -> settle) không cần chèn manual `sessions.insert` giữa chừng, kiểm tra số dư và vật phẩm hoán đổi, đồng thời kiểm tra phục hồi an toàn khi partner rớt mạng giữa chừng.

