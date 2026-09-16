# HANDOFF — Hoàn Tất Nghiên Cứu & Triển Khai Gói Tin Khởi Tạo Kết Nối TCP (Login Handshake)

**Ngày lập**: 2026-09-16  
**Module**: `.scratch/op-working/`  
**Feature**: `login-handshake-connection`  
**Trạng thái**: Đã phân tích, vá lỗi mã nguồn và kiểm thử thành công (11/11 tests pass). Chưa commit git (chờ user commit thủ công).

---

## 1. Tóm Tắt Bối Cảnh & Vấn Đề Đã Giải Quyết

### 1.1. Vấn đề phát hiện
Khi máy khách TS Online (`aLogin.exe`) kết nối TCP đến máy chủ Rust `ts_dream` (cổng `6414`):
- **Phía Client**: Hàm `TForm1.ClientSocket1Connect` (`0x0050CF4C`) chỉ ghi log IP và lưu cấu trúc mạng; hoàn toàn **không** gửi bất kỳ byte dữ liệu nào. Client hoàn toàn thụ động chờ Server ra lệnh.
- **Phía Server**: Hàm `handle_client_connection` ([`src/web/server_control.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs)) trước đây sau khi `accept()` socket liền lập tức đi vào vòng lặp `read_half.read(&mut buf).await`.
- **Hậu quả**: Cả Client và Server cùng đợi nhau đọc dữ liệu $\to$ **Deadlock / Treo kết nối vĩnh viễn**. Màn hình đăng nhập của máy khách không bao giờ xuất hiện.

### 1.2. Phân tích gói tin chuẩn
- **Nguyên lý cốt lõi**: **Server phải chủ động nói trước (Server speaks first)**.
- **Gói tin Greeting chuẩn**: Server cần gửi ngay gói tin chuyển sang màn hình Login Scene:
  - Wire format thô (Plaintext): `F4 44 03 00 01 09 5A`
    - Token: `F4 44`
    - Length LE: `03 00` (3 bytes payload)
    - Payload: `01 09 5A` (`MainOp = 0x01`, `SubOp = 0x09`, `sceneMode = 0x5A` tức 90 decimal).
  - Dữ liệu mã hóa XOR `0xAD` trên socket: `59 E9 AE AD AC A4 F7` (7 bytes).
- **Hành vi máy khách khi nhận `sceneMode = 90` (`0x5A`)**:
  - Hàm `FUN_00504c9c` (`0x00504C9C`) kiểm tra `(sceneMode % 100) + 0xA6 < 10` (tối ưu bù 2 của Borland Delphi cho điều kiện `sceneMode % 100 ∈ [90..99]`) trả về `TRUE`.
  - Client ẩn hộp thoại chọn server (`TFrmSelectServer`), mở Form Login (`TFrmLogin`), và nếu có tài khoản/mật khẩu lưu sẵn trong bộ nhớ thì kích hoạt Auto-login (`FUN_007095c0`).
- **Gói tin xác thực Client gửi lên (`SendCommand(1)`)**:
  - Cấu trúc: `[F4 44] [Len LE] [0x01] [lenPw: 1B] [PlayerID: 4B LE] [Prefix: 2B] [0x00BC: 2B LE] [Raw Password]`
  - Server bóc tách: `ctx.opcode = 0x01`, `ctx.sub = lenPw`, `payload = [PlayerID 4B LE][Prefix 2B][Version 2B LE][Password]`.

---

## 2. Các Thay Đổi Đã Thực Hiện Trong Mã Nguồn

### 2.1. Đổi tên và chuẩn hóa hằng số trong [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs)
- Thay thế hằng số cũ `HELLO_REPLY = "F4440300010901"` bằng:
  ```rust
  /// Server Greeting Packet gửi ngay khi Client kết nối TCP thành công.
  /// Op: 0x01 (OP_AUTH), Sub: 0x09 (Scene Switch), SceneMode: 0x5A (90 - Login Scene).
  /// Payload 3 bytes: 01 09 5A -> Frame: F444030001095A.
  pub const LOGIN_SCENE_GREETING: &str = "F444030001095A";
  ```
  *(Đổi tên từ `HELLO_REPLY` sang `LOGIN_SCENE_GREETING` vì đây không phải là reply mà là gói tin chủ động từ server thiết lập login scene).*

### 2.2. Gửi Greeting Packet trong [`src/web/server_control.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs)
- Tại hàm `handle_client_connection`, trước khi vào vòng lặp `while !close`:
  ```rust
  // TS Online Client (aLogin.exe) hoàn toàn thụ động chờ Server gửi gói tin đầu tiên.
  // Gửi ngay gói tin chào mừng chuyển sang Login Scene (Op 0x01, Sub 0x09, Scene 90 / 0x5A).
  let _ = tx.send(crate::server::spawn::LOGIN_SCENE_GREETING.to_string());
  ```

### 2.3. Tinh chỉnh xác thực mật khẩu trong [`src/server/handlers/login.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs)
- Giữ nguyên tiền tố duy nhất `"VN"` theo chỉ định của người dùng.
- Sử dụng `ctx.sub` (`lenPw`) để cắt đúng chiều dài mật khẩu thực tế, tránh dính byte rác hoặc padding:
  ```rust
  let len_pw = ctx.sub as usize;
  let password = if len_pw > 0 && payload.len() >= 8 + len_pw {
      &payload[8..8 + len_pw]
  } else {
      &payload[8..]
  };
  ```

### 2.4. Tài liệu chi tiết
- [`.scratch/client-pseudo-op-code/login_first_step_analysis.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/login_first_step_analysis.md): Báo cáo nghiên cứu toàn văn, bảng ánh xạ địa chỉ C/Assembly x86 và chứng minh toán học chi tiết.

---

## 3. Trạng Thái Mã Nguồn & Kiểm Thử

- **Biên dịch**: `cargo check` thành công không lỗi.
- **Kiểm thử**: `cargo test --all-targets --no-fail-fast` đạt **11/11 tests pass** (0 failed):
  - `ground_test.rs`: 2 passed
  - `rank_test.rs`: 3 passed
  - `system_alert_test.rs`: 4 passed
  - `warps_test.rs`: 2 passed
- **Git Status**: Toàn bộ thay đổi đang ở working tree, **chưa commit** (người dùng sẽ tự commit thủ công theo quy tắc dự án trong `AGENTS.md`).

---

## 4. Đề Xuất Hướng Đi Tiếp Theo (Next Steps)

Sau khi bước kết nối ban đầu và màn hình đăng nhập đã được khơi thông, các bước tiếp theo cần triển khai gồm:

### Bước 1: Kiểm thử End-to-End với Client `aLogin.exe` thật
- Khởi động server: `cargo run` (Game TCP port 6414, Web Admin 8090).
- Chạy client `aLogin.exe` kết nối tới IP/Port của server.
- Quan sát UI client:
  - Form Login xuất hiện ngay lập tức (không còn bị treo màn hình chọn server).
  - Nhập tài khoản `VN1001` và mật khẩu để test gửi gói tin Auth `SendCommand(1)`.

### Bước 2: Kiểm tra & hoàn thiện luồng phản hồi sau khi nhận gói tin Auth
- **Trường hợp tài khoản mới / chưa có nhân vật**:
  - Server gửi `LOGIN_CREATE_CHAR` (`F4 44 03 00 01 03 00`).
  - Client sẽ chuyển sang giao diện tạo nhân vật (chọn hệ Phong/Địa/Thủy/Hỏa, kiểu tóc, màu tóc, cộng điểm thuộc tính ban đầu).
  - Cần đối chiếu với handler Opcode `0x09` ([`src/server/handlers/character.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs)) và tài liệu [`.scratch/client-pseudo-op-code/opcode_09.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_09.md).
- **Trường hợp tài khoản đã có nhân vật**:
  - Server gửi chuỗi 22 gói tin `Logined1` (`build_logined_sequence_session` trong [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs)).
  - Kiểm tra xem client có load map thành công và xuất hiện nhân vật trong game hay có bị crash/disconnect ở frame nào trong chuỗi 22 frame này.

### Bước 3: Chuẩn hóa encoding VISCII 1.1 cho tên nhân vật và chat
- Kiểm tra lại các hàm mã hóa/giải mã trong [`src/encoding.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/encoding.rs) khi client nhập tên tiếng Việt có dấu trong giao diện tạo nhân vật.

---

## 5. Suggested Skills Cho Agent Phiên Tiếp Theo

- **`research`**: Dùng khi cần tiếp tục đọc mã nguồn C/Assembly trong thư mục `client_pseudo_c/` cho Opcode `0x09` (Tạo nhân vật) hoặc Opcode `0x01` Sub `0x03` (Chuyển scene tạo nhân vật).
- **`plan`**: Lập lộ trình thử nghiệm chi tiết cho luồng Tạo nhân vật $\to$ Vào map tân thủ.
