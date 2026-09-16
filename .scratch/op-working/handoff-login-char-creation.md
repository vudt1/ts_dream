# HANDOFF — Hoàn Tất Nghiên Cứu & Triển Khai Bước 2 & Bước 3 (Login, Tạo Nhân Vật & Chuẩn Hóa Encoding)

**Thời gian lập**: 2026-09-16  
**Module**: `ts_dream` (`src/encoding.rs`, `src/server/handlers/character.rs`, `src/server/handlers/login.rs`, `src/db/modern/`)  
**Tài liệu tiền nhiệm**: `.scratch/op-working/handoff-login-handshake.md`  
**Trạng thái**: Đã hoàn thiện toàn diện Bước 2 & Bước 3, 23/23 unit/integration tests pass. Chưa commit git (tuân thủ quy định tại `AGENTS.md`).

---

## 1. Bối Cảnh & Nhiệm Vụ Đã Thực Hiện

Phiên làm việc tiếp nối sau khi giải quyết vấn đề bắt tay TCP ban đầu (Server gửi greeting packet `F444030001095A` mở form Login).
Yêu cầu của người dùng là hoàn thiện hai mục trong "Đề xuất hướng đi tiếp theo":
- **Bước 2**: Kiểm tra & hoàn thiện luồng phản hồi sau khi nhận gói tin Auth (tạo nhân vật mới qua Opcode `0x09`, vào game qua Opcode `0x03 Sub 0x01` phát chuỗi 21 frame `Logined1`).
- **Bước 3**: Chuẩn hóa encoding VISCII 1.1 cho tên nhân vật và chat, đặc biệt là xử lý Unicode tổ hợp (NFD combining diacritics).
- *(Bước 1: Kiểm thử trực tiếp với client Windows `aLogin.exe` được người dùng chủ động bảo lưu để tự kiểm tra thủ công sau).*

---

## 2. Chi Tiết Các Thay Đổi Trong Mã Nguồn

### 2.1. Bước 3 — Chuẩn Hóa VISCII 1.1 & Bộ Chuyển Đổi NFD $\to$ NFC (`src/encoding.rs`)
- **Vấn đề**: Các bộ gõ tiếng Việt hiện đại (Unikey, EVKey, fcitx trên Linux, macOS IME) thường gửi chuỗi Unicode tổ hợp (NFD) chứa các combining characters (`\u{0300}`..`\u{036F}`). Trước đây `viscii_encode` chỉ tra bảng `char` đơn lẻ nên toàn bộ dấu tổ hợp bị chuyển thành dấu `'?'` (`0x3F`), làm hỏng tên nhân vật và nội dung chat.
- **Giải pháp**:
  - Viết hàm `compose_vietnamese_char(base: char, mark: char) -> Option<char>` ghép chính xác 132 tổ hợp ký tự tiếng Việt chuẩn và 60 tổ hợp đảo thứ tự dấu (dấu thanh trước dấu mũ/móc/trăng).
  - Viết hàm `normalize_vietnamese_nfc(s: &str) -> String` quét chuỗi và chuẩn hóa NFD sang NFC.
  - Tích hợp `normalize_vietnamese_nfc` vào đầu hàm `viscii_encode(s: &str) -> Vec<u8>`, đảm bảo 100% tên tiếng Việt dù gõ theo kiểu nào cũng mã hóa sang đúng byte VISCII 1.1.

### 2.2. Bước 2 — Hoàn Thiện Luồng Tạo Nhân Vật (Opcode 0x09) & Vào Game
- **Xử lý kiểm tra tên nhân vật (Opcode `0x09 Sub 0x02`)** in `src/server/handlers/character.rs`:
  - Thêm `is_valid_char_name(name: &[u8]) -> bool`: kiểm tra độ dài 1..=16 bytes, loại bỏ khoảng trắng đầu/cuối và các byte điều khiển nhị phân.
  - Khi tên không hợp lệ hoặc trùng: lập tức `conn.session.pending_new_char_name.clear()`, phản hồi `CHAR_NAME_INVALID` (`09 03 02`) hoặc `CHAR_NAME_DUPLICATE` (`09 03 01`).
  - Khi tên hợp lệ: lưu vào `pending_new_char_name` và phản hồi `CHAR_NAME_AVAILABLE` (`09 03 00`).
- **Xử lý tạo nhân vật (Opcode `0x09 Sub 0x01`)** in `src/server/handlers/character.rs`:
  - Nới lỏng và bảo vệ `parse_create`: Hỗ trợ cả payload 20 bytes (khi client bỏ qua `pass2`), kẹp hệ nguyên tố (`thuoctinh`) về `1..=4`, chuẩn hóa giới tính (`sex <= 1`).
  - Hỗ trợ payload rỗng: Tương thích với logic server Bear C# (`CreateChar.cs` dòng 28), coi `09 01` rỗng là tín hiệu xác nhận vào game và ủy quyền sang `handle_enter_game`.
  - Phản hồi lỗi thân thiện: Khi DB gặp xung đột (race condition trùng tên), trả về `CHAR_NAME_DUPLICATE` thay vì ép ngắt kết nối (`out.shutdown = true`).
  - Dọn dẹp túi đồ trong `apply_to_session`: Xóa sạch `session.homdo` và `session.trangbi` trước khi nạp trang bị tân thủ (tránh nhân bản vật phẩm khi tạo lại).
- **Lưu mật khẩu cá nhân / mã cấp 2 (`pass2`)**:
  - Bổ sung `update_pass2` vào trait `AccountRepository` (`src/db/modern/traits.rs`) và triển khai tại `ModernAccountRepository` (`src/db/modern/sqlite/accounts.rs`).
  - Lưu `pass2` vào bảng `accounts` khi nhân vật mới được tạo có điền mã cấp 2.
- **Đồng bộ cờ xác thực (`conn.session.authed`)**:
  - Tại `src/server/handlers/login.rs`: Bật `conn.session.authed = true` ngay khi tài khoản đăng nhập thành công (kể cả khi chưa có nhân vật), giúp client sau khi tạo nhân vật gửi `Opcode 0x03 Sub 0x01` sẽ không bị chặn cổng bảo mật.
  - Xử lý `handle_enter_game`: Phát sinh đầy đủ 21 frame của chuỗi `Logined1` (`spawn::build_logined_sequence_session`), đưa người chơi vào map tân thủ Trác Quận (`10817`).

---

## 3. Trạng Thái Mã Nguồn & Kiểm Thử

- **Kiểm thử tự động**: Chạy `cargo test --all-targets --no-fail-fast` $\to$ **23/23 tests pass**:
  - `tests/encoding_test.rs`: 7 tests pass (bao quát 134 ký tự tiếng Việt, C0 control slots, và kiểm tra triệt để NFD normalization).
  - `tests/login_char_flow_test.rs`: 5 tests pass (kiểm tra đầy đủ chu trình: Login $\to$ Name Check $\to$ Create Char $\to$ Enter Game 21 frames).
  - Các tests khác (`ground_test`, `rank_test`, `system_alert_test`, `warps_test`): 11 tests pass.
- **Linter**: `cargo clippy --all-targets -- -A unknown_lints` đạt chuẩn, không có cảnh báo nào trong các file sửa đổi.
- **Git Status**: Toàn bộ thay đổi nằm trong working tree, **chưa commit** (người dùng sẽ tự commit thủ công).

---

## 4. Danh Sách File Đã Thay Đổi & Tạo Mới

- `src/encoding.rs`: Bộ chuyển đổi NFD $\to$ NFC và tích hợp `viscii_encode`.
- `src/server/handlers/character.rs`: Bộ kiểm tra tên, phân tích tạo nhân vật an toàn, lưu `pass2`, hỗ trợ `09 01` rỗng.
- `src/server/handlers/login.rs`: Cắt mật khẩu theo `lenPw`, đồng bộ cờ `authed`.
- `src/db/modern/traits.rs`: Bổ sung phương thức `update_pass2` vào `AccountRepository`.
- `src/db/modern/sqlite/accounts.rs`: Triển khai SQL lưu `pass2` vào bảng `accounts`.
- `tests/encoding_test.rs`: File kiểm thử mới cho encoding & NFD normalization.
- `tests/login_char_flow_test.rs`: File kiểm thử mới cho trọn vẹn luồng đăng nhập $\to$ tạo nhân vật $\to$ vào game.

---

## 5. Hướng Dẫn Tiếp Tục (Cho Session / Agent Kế Tiếp)

### Bước Tiếp Theo Ưu Tiên:
1. **Thực hiện Bước 1 (Kiểm thử thực tế với `aLogin.exe`)**:
   - Chạy server: `cargo run`.
   - Kết nối bằng client `aLogin.exe` từ máy thật:
     - Kiểm tra màn hình Login hiển thị ngay lập tức.
     - Đăng nhập tài khoản mới (chưa có nhân vật) $\to$ chuyển sang màn hình Tạo nhân vật.
     - Nhập tên tiếng Việt có dấu, chọn hệ Phong/Địa/Thủy/Hỏa, chỉ số, mã cá nhân $\to$ bấm Tạo.
     - Xác nhận vào game $\to$ kiểm tra nhân vật xuất hiện trên map Trác Quận tân thủ (`10817`).
2. **Nghiên cứu các opcode tiếp theo khi đã vào map**:
   - Di chuyển trên bản đồ: Opcode `0x05` / `0x06` (Movement & Warp map).
   - Chat trên bản đồ: Opcode `0x02` (Chat public / mật / đội).
   - Tương tác NPC cơ bản: Opcode `0x18` (Talk / Event / Eve Engine).

---

## 6. Suggested Skills

- **`research`**: Dùng khi cần tra cứu các hàm decompile C/Assembly trong `client_pseudo_c/` hoặc các ghi chú opcode trong `.scratch/client-pseudo-op-code/` khi gặp opcode mới phát sinh trong quá trình di chuyển hoặc tương tác NPC.
- **`plan`**: Dùng khi cần thiết kế lộ trình triển khai tính năng di chuyển đồng bộ nhiều người chơi (multiplayer movement broadcasting) hoặc hệ thống nhiệm vụ Eve Engine.
