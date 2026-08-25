# 08 — Test Suite Migration and Legacy C# Comment Cleanup

**What to build:** Di chuyển toàn bộ 48+ khối `#[cfg(test)]` từ `src/` sang các tệp test chuyên biệt trong `tests/` (`tests/protocol_test.rs`, `tests/handlers_test.rs`, `tests/battle_engine_test.rs`, `tests/data_loader_test.rs`, `tests/db_persist_test.rs`, `tests/server_state_test.rs`), xóa bỏ 100% chú thích C# cũ, và xác thực 100% test pass (bao gồm cả 18 golden test diffing suites).

**Blocked by:** 07 — Two-Tier Dispatcher and Modular Handlers

**Status:** ready-for-agent

- [ ] Di chuyển toàn bộ unit tests inline trong `src/` sang các tệp kiểm thử chuyên biệt tại `tests/`.
- [ ] Xóa bỏ toàn bộ các comment nhắc đến C# (`// C# ...`, `// Ported from C#`, `// Matches C# behavior`) trong toàn bộ mã nguồn Rust.
- [ ] `cargo test --all-targets` chạy thành công 100% không có cảnh báo hoặc lỗi.
- [ ] Toàn bộ 18 golden test fixtures (`golden/01` $\to$ `golden/18`) vượt qua kiểm tra diffing chính xác.
