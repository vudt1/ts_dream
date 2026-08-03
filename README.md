# TS Dream — Private Game Server

Private server cho game TS Online, viết lại bằng **Rust**. Server game kết nối qua **TCP port 6414** (giao thức XOR `0xAD`, frame header `F4 44`), kết hợp **web admin dashboard** để quản lý. Chạy được trên **Windows / Linux / macOS** với một binary duy nhất.

## Tính năng

- **Game server (TCP)**
  - Async runtime Tokio, mô hình 1 client = 1 task
  - Giao thức TS: XOR `0xAD`, header `F4 44`, length little-endian, opcode/sub
  - Dispatcher `match` 70+ opcode (được LLVM tối ưu thành jump table)
- **Web admin (HTTP)**
  - Axum chia sẻ cùng Tokio runtime với TCP server
  - Dashboard Askama + HTMX, nhúng toàn bộ vào một binary
  - Quản lý: start/stop server, xem người chơi online, xem log packet realtime (SSE)
- **Lưu trữ**
  - SQLite (sqlx) — gọn nhẹ, không cần cài thêm dịch vụ
- **Cross-platform**: Windows, Linux, macOS — single binary

## Yêu cầu

- Rust toolchain (cài qua [rustup.rs](https://rustup.rs))

## Cài đặt & chạy

```bash
# Build (release)
cargo build --release

# Chạy server
./target/release/ts_dream
```

Cấu hình mặc định:

| Mục | Giá trị |
| --- | ------- |
| Game TCP port | `6414` |
| Web admin port | `8090` |
| Database | SQLite (gọn nhẹ, không cần cài đặt gì thêm) |

Có thể ghi đè qua file cấu hình (xem `src/config.rs`).

## Cấu trúc thư mục

```text
ts_dream/
├── Cargo.toml
├── templates/                  # HTML cho Web Dashboard (Askama)
├── docs/agents/                # Cấu hình kỹ năng agent (issue tracker, triage, domain docs)
└── src/
    ├── main.rs                 # Entry point: khởi tạo AppState, chạy song song TCP + Web
    ├── config.rs               # Cấu hình (port, db URL...)
    ├── db.rs                   # Kết nối SQLite (sqlx)
    ├── state.rs                # AppState chia sẻ giữa Web và TCP
    ├── network/
    │   ├── tcp_server.rs       # Lắng nghe TCP 6414, quản lý client connections
    │   ├── packet.rs           # Frame F4 44, length, opcode, sub, data
    │   └── crypto.rs           # Mã hóa/giải mã XOR 0xAD
    ├── game/
    │   ├── dispatcher.rs       # Phân luồng 70+ opcodes
    │   └── handlers/           # Login, Chat, Move, Skill, Item...
    └── web/
        ├── routes.rs           # Định tuyến API + HTML (Axum)
        ├── handlers.rs         # Logic HTTP (start/stop server, view online)
        └── sse.rs              # Server-Sent Events stream log packet
```

## Giao thức (tóm tắt)

## Cách build
### Windows
```
cargo build --target x86_64-pc-windows-gnu --release
```
### Linux
```
cargo build --release
```

## License

[MIT](LICENSE)
