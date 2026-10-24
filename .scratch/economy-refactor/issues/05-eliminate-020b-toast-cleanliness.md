# 05 — Eliminate 02 0B Toast & Protocol Error Cleanliness

Status: completed
Blocked by: 02, 03, 04

---

## 1. Mục tiêu
Xóa bỏ hoàn toàn việc lạm dụng opcode `02 0B` làm thông báo chữ đỏ (Red Message Toast) trong toàn bộ codebase:
- `02 0B` trong Client Delphi nguyên bản là tín hiệu mở Form tạo Quân Đoàn `TFCreateOrganize` (kèm điều kiện `player+0x12F8 >= 100.000` kim). Việc gửi chuỗi thông báo lỗi qua `02 0B` khiến client có nguy cơ bị giật popup tạo bang hội hoặc báo lỗi tiền không hợp lệ.
- Chuyển toàn bộ các thông báo lỗi kinh tế về đúng mã phản hồi gốc của từng opcode.
- Với các thông báo hệ thống tự do, chuyển sang sử dụng kênh Chat Hệ thống chuẩn `02 01 <text>` hoặc System Alert `0x00 <sub>`.

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 Rà soát và Thay thế `02 0B`
- Tại `src/server/handlers/shops.rs`:
  - Xóa bỏ helper `red_message(text)` hoặc sửa nó không phát `02 0B`.
  - Thay thế các đoạn gọi `red_message` khi mua/bán thất bại bằng mã trả về chuẩn của Shop NPC:
    - Khi không đủ tiền: Gửi `1B 01 01`.
    - Khi đầy túi: Gửi `1B 01 02`.
    - Khi số lượng không hợp lệ: Gửi `1B 01 03`.
    - Khi gián đoạn: Gửi `1B 01 04`.
    - Khi đầy tiền vàng: Gửi `1B 01 06`.
- Tại `src/server/handlers/chat.rs`:
  - Rà soát các hàm gửi thông báo hệ thống màu đỏ. Thay thế tiền tố `020B00000000` bằng kênh Chat Hệ thống hợp lệ `02 01 02 <text_viscii>` (kênh thông báo hệ thống) hoặc broadcast thông báo server.
- Tại `src/server/handlers/trade_storage.rs`:
  - Thay thế các thông báo lỗi kho bằng mã lỗi Tiền Trang `1D 03` hoặc mã toast P2P trade `19 02 <code>`.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. Không còn bất kỳ đoạn mã nào trong server gửi chuỗi hex bắt đầu bằng `F444..020B` để hiển thị thông báo lỗi text.
2. Thao tác Shop NPC lỗi chỉ phát ra các mã chuẩn `1B 01 01/02/03/04/06`.
3. Client Delphi không bao giờ bị giật popup Form tạo Quân Đoàn khi gặp lỗi kinh tế.
4. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

- Replaced `020B` frame builder in `src/server/handlers/shops.rs` (`red_message`), `src/server/handlers/use_item/mod.rs` (`red`), and `src/server/spawn.rs` (`sys_msg_frame`) with standard system chat channel `020102<viscii>`.
- Updated all system notices in `src/server/handlers/chat.rs` (cooldown, offline whisper, no party, slash commands) to emit `020102` via `spawn::sys_msg_frame`.
- Updated `tests/chat_test.rs` to assert `020102` channel instead of legacy `020B`.
- Verified that all economy paths in `shops.rs` and `trade_storage.rs` use native protocol return codes (`1B 01/02 <code>`, `1D 03`, `19 02 <code>`).
- Verified zero occurrences of `020B` emitted across all system message / error channels.

