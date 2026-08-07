# Hướng Dẫn Executing Tests Cho Dự Án `ts_dream` (Agent Protocol & Reference)

## ⚠️ Quy Tắc Quan Trọng Dành Cho AI Agents (Agent Execution Protocol)

1. **Kích hoạt đơn nhiệm (Single Task Only)**:
   - Trong mỗi lượt/thời điểm, **chỉ được phép kích hoạt DUY NHẤT 1 Task/Command** để chạy test hoặc check build.
   - **TUYỆT ĐỐI KHÔNG** khởi chạy đồng thời nhiều câu lệnh `cargo test` / `cargo check` / `cargo clippy` song song (vì Cargo chia sẻ khoá tập tin `target/`, chạy song song sẽ gây ra xung đột Cargo lock hoặc lãng phí tài nguyên hệ thống).

2. **Chờ hoàn thành (Synchronous Execution & Wait)**:
   - Mỗi câu lệnh test/build Cargo thường mất từ **20 đến 30 giây** (hoặc hơn tùy lượng code thay đổi).
   - Agent **phải chờ đợi** câu lệnh hoàn tất và nhận kết quả trước khi đưa ra quyết định tiếp theo hoặc chạy câu lệnh mới.

3. **Chọn phạm vi test phù hợp (Targeted Execution)**:
   - Khi chỉnh sửa file cục bộ: Ưu tiên chạy unit test module tương ứng (ví dụ: `shops::tests`, `battle::tests`) để có kết quả phản hồi nhanh nhất.
   - Khi chuẩn bị hoàn tất công việc: Chạy toàn bộ test lib (`cargo test --lib`), golden suite (`cargo test --test golden_suite`) và kiểm tra lint (`cargo clippy --all-targets`).

4. **Xử lý Output Stream**:
   - Sử dụng các filter như `tail`, `head`, `grep` để thu gọn log đầu ra, tránh tràn màn hình ngữ cảnh nhưng vẫn bắt đủ thông tin lỗi (`FAILED`, `error`, `panicked`, `warning`).

---

## 📋 Danh Mục Lệnh Execute Test & Linting

### 1. Targeted Unit Tests (Test Đơn Module / Cụm Module)

- **Test cho 1 module cụ thể (Ví dụ: `shops.rs`):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --lib shops::tests 2>&1 | tail -30
  ```

- **Test kết hợp các module chính (Ví dụ: `shops.rs` & `handler.rs`):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --lib "shops::tests" 2>&1 | tail -8 && cargo test --lib "handler::tests" 2>&1 | tail -8
  ```

- **Test cho module Battle (`battle::tests`):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --lib battle::tests 2>&1 | tail -30
  ```

- **Test cho module Server / Session (`session::tests`):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --lib server::session::tests 2>&1 | tail -30
  ```

- **Chạy toàn bộ Lib Unit Tests:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --lib 2>&1 | tail -30
  ```

---

### 2. Integration & Golden Tests (Test Tích Hợp & Golden Suite)

- **Chạy Golden Suite test (Golden Gate):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --test golden_suite 2>&1 | tail -15
  ```

- **Tái tạo Golden file (Regenerate Goldens) & kiểm tra thay đổi file:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --test golden_suite -- --ignored regenerate_goldens 2>&1 | tail -8 && git status --short golden/
  ```

- **Chạy Battle Golden Integration test:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --test battle_golden 2>&1 | tail -20
  ```

- **Chạy Data Integration test:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --test data 2>&1 | tail -20
  ```

- **Chạy Web Dashboard Integration test:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test --test web_dashboard 2>&1 | tail -20
  ```

---

### 3. Typechecking & Code Quality (Linting & Warning Checks)

- **Typecheck nhanh tất cả targets (Lib, Bin, Tests):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo check --all-targets 2>&1 | tail -15
  ```

- **Chạy Clippy Linting toàn dự án:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo clippy --all-targets 2>&1 | grep -E "warning|error" | head -30; echo "EXIT: done"
  ```

- **Lọc Clippy warning theo danh sách file vừa chỉnh sửa:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo clippy --all-targets 2>&1 | grep -A3 "shops.rs\|session.rs\|talk.rs\|server_control.rs\|handler.rs" | grep -E "warning|-->|shops\.rs|session\.rs|talk\.rs|server_control\.rs|handler\.rs" | head -40
  ```

- **Soi chi tiết warning tại vị trí dòng code cụ thể:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo clippy --all-targets 2>&1 | grep -B2 "src/server/session.rs:399\|src/web/server_control.rs:186\|src/web/server_control.rs:210" | head -30; echo "=== shops.rs ==="; cargo clippy --all-targets 2>&1 | grep -B3 -A3 "src/server/handlers/shops.rs" | head -60
  ```

---

### 4. Full Suite & Summaries (Toàn Bộ Test Suite)

- **Chạy Full Test Suite (Xem chi tiết lỗi nếu có):**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test 2>&1 | grep -E "Running|test result|error|FAILED|panicked" | tail -40
  ```

- **Tổng hợp kết quả tất cả các Test Suite:**
  ```bash
  cd /mnt/d/VUDT/GIT_PCC/ts_dream && cargo test 2>&1 | grep -E "^test result|running [0-9]+ tests|FAILED|error" | head -30
  ```