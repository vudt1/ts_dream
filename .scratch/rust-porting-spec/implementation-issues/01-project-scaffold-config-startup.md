# 01 — Scaffold dự án + config + startup sequence

**What to build:** Binary server Rust chạy lên được, đọc config, kết nối MySQL fail-fast, chạy migration, spawn cả hai listener (web dashboard + game TCP) theo đúng thứ tự startup của spec. Nền tảng mọi slice khác.

**Blocked by:** None — can start immediately.

**Status:** completed

- [x] Khởi tạo workspace Rust (tokio + axum + sqlx 0.8 features `mysql`/`runtime-tokio-rustls`/`migrate` + askama), Cargo build ra đúng một binary.
- [x] `ts_dream.toml` + các env key `TS_GAME_PORT`/`TS_WEB_PORT`/`TS_DATA_DIR`/`TS_DATABASE_URL`/`TS_PEREXP_DEFAULT` load once lúc boot; các key SQLite cũ (`account_db_path`/`member_dir`/`template_db_path`) **bị loại** (báo rõ nếu blob).
- [x] Startup đúng chuỗi: load config → connect MySQL pool → `sqlx::migrate!()` → spawn web :8090 → load static data (`DataLoaded`) → bắt đầu TCP accept :6414 (gated trên `DataLoaded`).
- [x] MySQL không reachable / migration fail → **hard exit** với diagnostic rõ ràng (không để dashboard lên cùng DB chết).
- [x] Cấu trúc module chia tách phù hợp để các ticket sau (framing, data load, DB, battle, dashboard) gắn được vào (seam cho framework/encoder). Hằng số giao thức XOR 0xAD / magic F4 44 / prefix `vn` / min version 186 / name TSVN **hardcode**, mở qua ngoài config (Ch1 §1.2, Ch8 §8.2).
- [x] Prefactor: đặt sẵn các **encode helpers** nơi framing (ticket 02) và **common-layer module boundaries** để 24 slice không chồng lấn state.

## Implementation notes

- Hoàn thành lần cuối tại commit `d246bd7` (branch `main`): enforce gate `DataLoaded` trong `ServerControl::start()` (từ chối bind TCP tới khi static data load xong, push log `error`), `login.rs` dùng `protocol::MIN_VERSION`/`ID_PREFIX` thay literal, gỡ builder trùng `codec::frame` (giữ `protocol::frame(code, body)` làm seam framing duy nhất), tách `config::from_file()` để test rejection key SQLite.
- Toàn bộ test suite xanh: 219 pass / 0 fail (`cargo test`); `cargo check --all-targets` sạch.
- Ghi nhận (ngoài phạm vi ticket): nếu data load fail, `main.rs` fallback `GameData::default()` + `start()` từ chối → game server không accept và **không có retry** khi data load sau — cần ticket riêng nếu muốn đường phục hồi.