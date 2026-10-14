# ADR 0004 — Chuyển đổi hệ quản trị cơ sở dữ liệu từ MySQL sang SQLite

- **Ngày**: 2026-09-11
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn grilling)

---

## Bối cảnh

- Trước đây, `ts_dream` sử dụng MySQL 8 thông qua SQLx (`features = ["mysql", ...]`).
- Yêu cầu người vận hành phải cài đặt, cấu hình và duy trì service MySQL độc lập (xem tài liệu legacy `Huong_Dan_Cai_Dat_MySQL_ZIP.md`).
- Nhằm mục đích đơn giản hóa việc triển khai, đóng gói gọn nhẹ (zero-external-dependency), project chuyển đổi toàn bộ tầng lưu trữ sang SQLite với file cơ sở dữ liệu đặt tại `DB/ts_dream.db`.

---

## Quyết định Thiết kế

### 1. Kiến trúc Dual-Pool (Đọc song song, Ghi tuần tự)
- SQLite hoạt động theo mô hình tệp cục bộ. Để tối ưu hiệu năng và triệt tiêu lỗi `SQLITE_BUSY` khi có nhiều luồng ghi đồng thời:
  - **Read Pool (`max_connections = 16`)**: Dành riêng cho các truy vấn đọc dữ liệu đồng thời (`SELECT`), không khóa tệp.
  - **Write Pool (`max_connections = 1`)**: Chỉ mở duy nhất 1 kết nối ghi. Toàn bộ thao tác ghi (`INSERT`, `UPDATE`, `DELETE`, giao dịch thay đổi số dư / giao dịch vật phẩm) đều đi qua kết nối này, tuần tự hóa ghi nhận để loại trừ hoàn toàn contention.
- Struct đại diện:
  ```rust
  pub struct DbPool {
      pub read: SqlitePool,
      pub write: SqlitePool,
  }
  ```

### 2. Cấu hình WAL (Write-Ahead Logging) & Pragmas
- Cơ sở dữ liệu SQLite được khởi tạo với:
  - `PRAGMA journal_mode = WAL;` (cho phép concurrent readers & single writer).
  - `PRAGMA synchronous = NORMAL;` (cân bằng hoàn hảo giữa hiệu năng đĩa và an toàn dữ liệu).
  - `PRAGMA foreign_keys = ON;` (ràng buộc toàn vẹn khóa ngoại).
  - `PRAGMA busy_timeout = 5000;` (timeout 5s).

### 3. Vị trí Lưu trữ & Đóng gói Phân phối
- File cơ sở dữ liệu SQLite mặc định đặt tại `DB/ts_dream.db` (thư mục `DB/` tại root project).
- Đường dẫn kết nối mặc định: `sqlite://DB/ts_dream.db`.
- Biến môi trường hỗ trợ: `TS_DATABASE_URL`, `TS_WAL_CHECKPOINT_INTERVAL_SECS`.
- Tự động tạo thư mục cha `DB/` nếu chưa tồn tại.
- Hàm `Config::resolve_database_url()` tự động tìm kiếm DB theo thứ tự:
  1. Thư mục `DB/` nằm cạnh file thực thi binary (production/release build).
  2. Thư mục `DB/` tại working directory hiện tại (development).
- `build.rs` tự động sao chép thư mục `DB/` vào cạnh file thực thi trong thư mục build profile (`target/debug` hoặc `target/release`), tương tự thư mục `Data/`.

### 4. Cơ chế Flush / Checkpoint WAL theo Lịch trình & Khi Tắt Server
- **Định kỳ (Periodic Checkpoint)**: Mặc định mỗi 5 phút (300 giây, cấu hình qua `wal_checkpoint_interval_secs` hoặc `TS_WAL_CHECKPOINT_INTERVAL_SECS`), một background task chạy `PRAGMA wal_checkpoint(TRUNCATE);` để đưa dữ liệu từ log WAL về file `.db` chính và thu gọn kích thước WAL về 0 byte.
- **Khi dừng TCP Server (`ServerControl::stop()`)**:
  1. Gửi thông báo đếm ngược 5 giây.
  2. Ngắt kết nối client.
  3. Tự động lưu toàn bộ session dirty vào database (`auto_save::save_all_dirty`).
  4. Thực hiện `PRAGMA wal_checkpoint(TRUNCATE);` để đảm bảo dữ liệu ghi trọn vẹn vào file `.db`.
- **Khi tắt tiến trình (Graceful Process Termination - SIGINT / Ctrl+C / SIGTERM)**:
  - Bắt tín hiệu `tokio::signal::ctrl_c()`.
  - Thực thi tuần tự: dừng TCP Server -> lưu dirty sessions -> chạy `pool.checkpoint().await` -> kết thúc sạch sẽ.

### 5. Giao diện Web Dashboard
- Trong [templates/dashboard.html](file:///mnt/d/VUDT/GIT_PCC/ts_dream/templates/dashboard.html), loại bỏ badge kiểm tra trạng thái MySQL (`#db-status-container`) và EventSource lắng nghe SSE `/api/db/status/stream`.
- Giữ lại duy nhất trạng thái hoạt động của Server (`SERVER ONLINE` / `SERVER OFFLINE`).
- Nhãn tài khoản đổi từ "MySQL accounts table" thành "Database accounts".

### 6. Trạng thái Schema Migrations
- **Out-of-scope**: Thiết kế lược đồ (schema) và bảng dữ liệu SQLite được người dùng tự thiết kế và quản lý riêng biệt.
- Hàm `pool::migrate` được chuyển thành placeholder an toàn (không tự chạy migration hay gây lỗi khi khởi động).

---

## Cấu trúc Codebase Cập nhật
- `src/db/modern/mysql/` -> được chuyển thành `src/db/modern/sqlite/`.
- Repository traits (`AccountRepository`, `CharacterRepository`, `InventoryRepository`, `PetRepository`, `QuestRepository`, `SessionRepository`) được điều chỉnh sang `SqliteConnection` / `Sqlite` và nhận `&DbPool`.
- Các câu truy vấn MySQL đặc thù đã được chuyển đổi:
  - `ON DUPLICATE KEY UPDATE col=VALUES(col)` -> `ON CONFLICT(...) DO UPDATE SET col=excluded.col`.
  - `DELETE a FROM table_a a JOIN table_b b ...` -> `DELETE FROM table_a WHERE ...`.
  - `INSERT IGNORE` -> `INSERT OR IGNORE`.
  - Bỏ các cú pháp khóa hàng phân tán không áp dụng cho SQLite như `SELECT ... FOR UPDATE`.
