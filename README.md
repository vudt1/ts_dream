# TS Dream — Private Game Server

Private server cho game TS Online, viết lại bằng **Rust**. Server game kết nối qua **TCP port 6414** (giao thức XOR `0xAD`, frame header `F4 44`), kết hợp **web admin dashboard** để quản lý. Chạy được trên **Windows / Linux / macOS** với một binary duy nhất.

## Tính năng

- **Game server (TCP)**
  - Async runtime Tokio, mô hình 1 client = 1 task
  - Giao thức TS: XOR `0xAD`, header `F4 44`, length little-endian, opcode/sub
  - Dispatcher `match` 70+ opcode (được LLVM tối ưu thành jump table)
- **Web admin (HTTP)**
  - Axum chia sẻ cùng Tokio runtime với TCP server, phục vụ tại port `8090`
  - Dashboard Askama + HTMX, nhúng toàn bộ vào một binary
  - Quản lý: start/stop server, tạo tài khoản, xem người chơi online, xem log packet realtime (SSE)
- **Lưu trữ (MySQL 8)**
  - Một database **`ts_dream`** (InnoDB), migration bằng SQLx chạy ở boot
  - Bảng dữ liệu nhân vật/gameplay bằng charset **latin1** để giữ nguyên byte VISCII (tên nhân vật)
  - **Tự tạo database khi chưa tồn tại** (bật mặc định, xem `db_auto_create`)
- **Cross-platform**: Windows, Linux, macOS — single binary

## Yêu cầu cài đặt

- Rust toolchain (cài qua [rustup.rs](https://rustup.rs))
- **MySQL 8** đang chạy tại `localhost:3306`
  - Mặc định hệ thống **tự tạo database `ts_dream`** (charset latin1) khi chưa có (flag `db_auto_create = true`), nên chỉ cần một user có quyền tạo DB.
  - Người vận hành có thể tự tạo trước và tắt flag nếu muốn (xem bảng env dưới).

## Cài đặt & chạy

```bash
# Build (release)
cargo build --release

# Chạy server (đọc cấu hình từ ts_dream.toml / biến môi trường TS_*)
./target/release/ts_dream
```

### Cấu hình mặc định

| Mục | Giá trị |
| --- | ------- |
| Game TCP port | `6414` |
| Web admin port | `8090` |
| Database | MySQL 8 — `mysql://user:pass@localhost:3306/ts_dream` |
| Tự tạo DB khi thiếu | `true` |

### Biến môi trường (`TS_*` — ghi đè lên file `ts_dream.toml`)

| Biến | Mặc định | Ý nghĩa |
| --- | --- | --- |
| `TS_GAME_PORT` | `6414` | Port TCP game server |
| `TS_WEB_PORT` | `8090` | Port web admin dashboard |
| `TS_DATA_DIR` | `./Data` | Thư mục dữ liệu tĩnh (`ts_server_old/Data/`) |
| `TS_DATABASE_URL` | `mysql://user:pass@localhost:3306/ts_dream` | Chuỗi kết nối MySQL |
| `TS_PEREXP_DEFAULT` | `0` | Giá trị PerEXP ban đầu runtime |
| `TS_DB_AUTO_CREATE` | `true` | Tự tạo database khi chưa tồn tại (`true`/`false`) |

Ví dụ:
```bash
export TS_DATABASE_URL="mysql://root:s3cret@localhost:3306/ts_dream"
export TS_DB_AUTO_CREATE="false"
./target/release/ts_dream
```

## Cấu trúc thư mục

```text
ts_dream/
├── Cargo.toml
├── migrations/
│   └── 0001_init.sql           # Schema MySQL 8: players, accounts, 9 bảng gameplay, item_code
├── templates/                  # HTML cho Web Dashboard (Askama)
├── docs/                       # rust_porting_spec.md, agents/...
├── golden/                     # Packet "vàng" để diffing khi test
├── tests/                      # Integration tests (golden diffing, data, battle, web)
└── src/
    ├── main.rs                 # Entry point: Config -> MySQL bootstrap -> Web + TCP
    ├── config.rs               # Cấu hình (port, db URL, env TS_*)
    ├── db/                     # Repository layer (tất cả SQL tập trung)
    │   ├── pool.rs             # Pool + auto-create DB + migration
    │   ├── accounts.rs         # Truy vấn accounts
    │   ├── players.rs          # Truy vấn/transaction players + 9 bảng gameplay + item_code
    │   ├── persist.rs          # Ghi-through players/skills/items/pets
    │   ├── item_code.rs        # Nhận mã quà (item_code)
    ├── server/                 # TCP server, session, handler, spawning
    ├── protocol/               # XOR 0xAD, frame F4 44
    ├── battle/                 # Battle engine
    ├── web/                    # Web admin dashboard (Axum + SSE)
    └── data/                   # Loader dữ liệu tĩnh
```

## Giao thức (tóm tắt)

- Frame: header `F4 44` + length LE u16 + opcode + sub + payload
- Mã hóa: XOR từng byte với `0xAD`, gửi/nhận qua hex
- Text: VISCII 1.1 (giữ nguyên byte, không transcode UTF-8)

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