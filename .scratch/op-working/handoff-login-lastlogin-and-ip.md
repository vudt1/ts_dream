# HANDOFF — Bổ Sung Ghi Nhận Lần Đăng Nhập & IP (`lastlogin_at`, `lastloginip`) Vào Table `accounts`

**Thời gian lập**: 2026-09-16  
**Module**: `ts_dream` (`src/db/`, `src/server/`, `src/web/`, `tests/db_repository_init_test.rs`)  
**Tài liệu tham chiếu**: `.scratch/op-working/handoff-db-schema-sqlite-alignment.md`, `migrations/0001_init.sql`  
**Trạng thái**: Hoàn thành 100%, 34/34 tests passed trên toàn bộ 7 test suites, `cargo clippy` sạch sẽ, chưa commit git (tuân thủ quy định `AGENTS.md`).

---

## 1. Bối Cảnh & Yêu Cầu Đã Triển Khai

Người dùng yêu cầu: Sau khi người chơi xác thực tài khoản và mật khẩu chính xác (Opcode `0x01` / `login_db`), server ngoài việc trả về payload để client chuyển cảnh in-game, cần tự động lưu lại:
- `lastlogin_at`: Thời gian đăng nhập gần nhất (Unix timestamp mili-giây).
- `lastloginip`: Địa chỉ IP nguồn của client (dạng chuỗi `TEXT`, ví dụ `"192.168.1.50"` hoặc `"127.0.0.1"`).
vào bảng `accounts` trong SQLite database.

---

## 2. Chi Tiết Thay Đổi Trong Mã Nguồn

### 2.1. Quản lý IP kết nối TCP (`Conn`)
- **`src/server/session.rs`**:
  - Bổ sung trường `pub peer_ip: String` vào struct `Conn`.
  - Cập nhật hàm khởi tạo `Conn::new()` (khởi tạo `peer_ip: String::new()`) và bổ sung hàm tiện ích `Conn::with_peer_ip(peer_ip: impl Into<String>)`.
- **`src/web/server_control.rs`**:
  - Trong `handle_client_connection(stream, peer, ...)`: khởi tạo kết nối với IP client thông qua `let mut conn = Conn::with_peer_ip(peer.ip().to_string());` (chỉ lấy địa chỉ IP chuẩn, không kèm port kết nối).

### 2.2. Cập nhật Tầng Truy Vấn Repository SQLite (`accounts`)
- **`src/db/modern/sqlite/accounts.rs`**:
  - Sửa hàm `touch_login(&self, account_id: i64, now_ms: i64, ip: &str) -> RepoResult<()>`:
    - Nếu `ip` rỗng thì bind `None` (`NULL`), ngược lại bind `Some(ip)`.
    - Thực thi truy vấn cập nhật đồng thời:
      `UPDATE accounts SET lastlogin_at = ?, lastloginip = ?, updatedat = ? WHERE playerid = ?`
- **`src/db/accounts.rs`**:
  - Bổ sung trường `pub last_login_ip: Option<String>` vào struct `AccountRow`.
  - Cập nhật câu lệnh truy vấn trong `list()`: `SELECT ..., lastlogin_at AS last_login_at, lastloginip AS last_login_ip FROM accounts`.

### 2.3. Tích Hợp Vào Luồng Xử Lý Login (`login_db`)
- **`src/server/handlers/login.rs`**:
  - Trong hàm `login_db`: Sau khi `repos.accounts().verify_pass1(id, password)` trả về `true` và tài khoản không bị tạm khóa (`is_suspended`), gọi:
    `repos.accounts().touch_login(id, now_ms, &conn.peer_ip).await?;`
  - Nếu mật khẩu sai: server trả về `LOGIN_WRONG_PASS`, tuyệt đối không cập nhật `lastlogin_at` và `lastloginip`.

### 2.4. Kiểm Thử Tích Hợp (`tests/db_repository_init_test.rs`)
- Cập nhật test case `test_account_creation_and_auth`: truyền IP `"127.0.0.1"` vào `touch_login` và assert kiểm tra `acc.last_login_at` và `acc.last_login_ip`.
- Bổ sung test case tích hợp mới `test_login_success_updates_lastlogin_at_and_ip`:
  1. Tạo tài khoản mới, xác nhận ban đầu `lastlogin_at == None` và `lastloginip == None`.
  2. Thử đăng nhập sai mật khẩu: server phản hồi `LOGIN_WRONG_PASS`, DB vẫn giữ nguyên `None`.
  3. Đăng nhập đúng mật khẩu từ IP `192.168.1.50`: server phản hồi `LOGIN_CREATE_CHAR`, DB cập nhật `lastlogin_at > 0` và `lastloginip = "192.168.1.50"`.
  4. Tạo nhân vật và đăng nhập lại từ IP `10.0.0.8`: server trả về chuỗi payload in-game chuyển cảnh (`conn.session.logined = true`), DB cập nhật `lastloginip = "10.0.0.8"` và timestamp mới hơn.

---

## 3. Trạng Thái Kiểm Thử & Linting

- **Test Suite**: Chạy lệnh `cargo test --all-targets --no-fail-fast` $\to$ **34/34 tests PASSED** trên toàn bộ 7 test suite:
  - `tests/db_repository_init_test.rs`: 11 passed (1 test mới)
  - `tests/encoding_test.rs`: 7 passed
  - `tests/login_char_flow_test.rs`: 5 passed
  - `tests/system_alert_test.rs`: 4 passed
  - `tests/rank_test.rs`: 3 passed
  - `tests/ground_test.rs`: 2 passed
  - `tests/warps_test.rs`: 2 passed
- **Linter**: `cargo clippy --all-targets` không phát sinh bất kỳ warning hay lỗi mới nào.
- **Git Status**: Toàn bộ thay đổi nằm trong working tree, chưa commit (người dùng tự commit thủ công).

---

## 4. Danh Sách File Đã Chỉnh Sửa

- `src/db/accounts.rs`: Thêm `last_login_ip` vào `AccountRow` và query `list()`.
- `src/db/modern/sqlite/accounts.rs`: Cập nhật `touch_login` lưu `lastlogin_at` và `lastloginip`.
- `src/server/session.rs`: Bổ sung `peer_ip` vào `Conn` và constructor `with_peer_ip`.
- `src/server/handlers/login.rs`: Truyền `&conn.peer_ip` vào `touch_login` trong `login_db`.
- `src/web/server_control.rs`: Gán IP của socket client vào `conn.peer_ip`.
- `tests/db_repository_init_test.rs`: Cập nhật `test_account_creation_and_auth` và thêm test case `test_login_success_updates_lastlogin_at_and_ip`.

---

## 5. Hướng Dẫn Tiếp Tục (Cho Session / Agent Kế Tiếp)

1. **Hiển thị thông tin trên Web Admin Dashboard (nếu cần)**:
   - Bảng `accounts` trên template `templates/dashboard.html` hiện tại hiển thị Player ID, GM Level, Suspended. Có thể bổ sung cột Last Login IP và Last Login Time từ `AccountRow`.
2. **Kiểm thử thực tế với Client `aLogin.exe` & SQLite DB**:
   - Khởi động server: `cargo run`.
   - Mở client TS Online đăng nhập và kiểm tra giá trị trong file SQLite `DB/ts_dream.db`:
     `SELECT playerid, lastlogin_at, lastloginip FROM accounts;`
3. **Triển khai các tính năng in-game kế tiếp**:
   - Di chuyển và chuyển map (Opcode `0x05`, `0x06`).
   - Kịch bản nhiệm vụ Eve Engine & tương tác NPC (Opcode `0x18`).

---

## 6. Suggested Skills

- **`research`**: Sử dụng để tra cứu mã xử lý opcode tiếp theo trong `client_pseudo_c/` hoặc `TS_Server_Bear/`.
- **`plan`**: Lập kế hoạch chi tiết cho các hệ thống game tiếp theo (Movement sync, Map warp, Chat system).
