# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as markdown files under `.scratch/<feature-slug>/` in this repo (no git remote yet). See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

### Triage labels

Five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).

### Domain docs

Single-context layout — one [`CONTEXT.md`](CONTEXT.md) + `docs/adr/` at the repo root. See [`docs/agents/domain.md`](docs/agents/domain.md).

---

## Tech Stack

- **Ngôn ngữ & Runtime**: Rust (2021 edition) + Tokio 1 (`full` async runtime).
- **Web Framework**: Axum 0.8 (Web admin dashboard phục vụ tại port 8090, chia sẻ cùng Tokio runtime với TCP Server).
- **Template Engine**: Askama 0.12 (Biên dịch HTML thẳng vào binary, kết hợp HTMX).
- **Database & Migration**: MySQL 8 (InnoDB, kết nối qua SQLx 0.8 với `mysql`, `runtime-tokio-rustls`, `migrate`).
- **Mã hóa & Định dạng Wire**: Giao thức TS Online (Header `F4 44`, XOR key `0xAD`, VISCII 1.1 text encoding).
- **Thư viện bổ sung**: Serde, Serde JSON, TOML 0.8, Tracing + tracing-subscriber (env-filter), Anyhow, Thiserror 2, Hex, Chrono, Futures. Dev: Tempfile, Tower.

---

## Cấu trúc Codebase (Codebase Structure)

```text
ts_dream/
├── Cargo.toml                  # Khai báo crate & phụ thuộc
├── build.rs                    # Đóng gói Data/ vào cạnh binary khi cargo build
├── CONTEXT.md                  # Từ vựng miền (Domain Glossary & Ubiquitous Language)
├── AGENTS.md                   # Hướng dẫn Agent, Tech Stack & Cấu trúc Codebase
├── LICENSE & README.md         # Giấy phép & hướng dẫn dựng dự án
├── Huong_Dan_Cai_Dat_MySQL_ZIP.md  # Hướng dẫn cài đặt MySQL
├── TS_Server_OP_Code_basic.md  # Đặc tả opcode giao thức TS Online tham khảo
├── Data/                       # Dữ liệu tĩnh game (Items.txt, Npcs.txt, Skills.txt, Warps.txt, Quests/ 800+ file) — runtime resolve qua Config::resolve_data_dir (ưu tiên ./Data rồi bản build.rs cạnh exe)
├── templates/
│   └── dashboard.html          # Template HTML duy nhất cho Web Dashboard (Askama + HTMX)
│
├── spec/
│   └── codebase_design.md      # Thiết kế kiến trúc ban đầu
├── migrations/
│   └── 0001_init.sql           # SQLx migration schema MySQL 8 (accounts, players, gameplay, item_code)
├── golden/                     # 15 golden packets (01-hello → 15-npc-shop-buy) để diffing khi test
├── tests/                      # Integration tests
│   ├── golden.rs / golden_suite.rs / battle_golden.rs  # Golden diffing (replay không cần DB)
│   ├── data.rs                 # Load dữ liệu tĩnh
│   ├── web_dashboard.rs        # Web admin dashboard
│   └── common/mod.rs           # Test harness dùng chung
└── src/
    ├── main.rs                 # Entry point: Config → MySQL bootstrap → seed map drops → Web Admin (8090) + Game TCP (6414)
    ├── lib.rs                  # Module root cho thư viện ts_dream
    ├── config.rs               # Xử lý file cấu hình + env TS_* (port, db URL, data_dir, db_auto_create)
    ├── state.rs                # AppState chia sẻ dữ liệu qua Arc<RwLock<AppState>>
    ├── error.rs                # Định nghĩa lỗi
    ├── encoding.rs             # Xử lý mã hóa VISCII 1.1
    ├── harness.rs              # Test harness hỗ trợ kiểm thử packet capture diffing
    ├── db/                     # Repository layer — mọi SQL tập trung, unit-testable
    │   ├── mod.rs              # Tổ chức module db
    │   ├── pool.rs             # MySQL Pool, auto-create database & chạy SQLx migration khi boot
    │   ├── accounts.rs         # Truy vấn accounts (login)
    │   ├── players.rs          # Truy vấn/transaction players + bảng gameplay + item_code
    │   ├── persist.rs          # Ghi-through players/skills/items/pets (no-op khi Option<&Pool> là None)
    │   └── item_code.rs        # Nhận mã quà (item_code), degrade khi không có DB
    ├── protocol/               # Bộ mã hóa/giải mã XOR 0xAD, phân tách khung tin
    │   ├── mod.rs              # Tổ chức module protocol
    │   ├── frame.rs            # Phân tách khung tin (Frame F4 44)
    │   ├── codec.rs            # Codec XOR 0xAD & opcode/subcode
    │   └── encoder.rs          # Đóng gói/buffer packet đi
    ├── server/                 # TCP Server listener (Port 6414), Session management & Spawner
    │   ├── handler.rs          # Phân luồng opcode chính (dispatcher match 70+ opcode)
    │   ├── session.rs          # Quản lý phiên kết nối (InventoryItem, offline state)
    │   ├── spawn.rs            # Spawner quản lý đối tượng/entity trong map
    │   ├── character_sheet.rs  # Bảng chỉ số nhân vật
    │   ├── inventory.rs        # Quản lý túi đồ nhân vật
    │   ├── pet_box.rs          # Quản lý hộp thú nuôi
    │   ├── map_drops.rs        # Registry drop vật phẩm trên map (ItemOnMap.txt)
    │   └── handlers/           # Handler chi tiết từng nhóm Opcode
    │       ├── mod.rs          # Khai báo các handler con
    │       ├── login.rs        # Đăng nhập/xác thực
    │       ├── character.rs    # Tạo/xóa nhân vật
    │       ├── movement.rs     # Di chuyển (move)
    │       ├── chat.rs         # Chat + lệnh slash
    │       ├── skills.rs       # Học kỹ năng, reborn
    │       ├── stats.rs        # Chỉ số/điểm
    │       ├── battle.rs       # Battle handler (input, commands, rewards)
    │       ├── quest.rs        # Nhiệm vụ (quest) + hội thoại H6
    │       ├── inventory.rs    # Túi đồ / sắp xếp / thả vật phẩm
    │       ├── use_item.rs     # Sử dụng vật phẩm (sách, thuốc)
    │       ├── shops.rs        # Cửa hàng NPC & người chơi (shop/mall buy)
    │       ├── trade_storage.rs# Giao dịch & kho (storage)
    │       ├── pet_actions.rs  # Hành động thú nuôi
    │       ├── expressions.rs  # Biểu cảm
    │       ├── talk.rs         # Hội thoại NPC
    │       └── system.rs       # Hệ thống/ping/time
    ├── web/                    # Web Admin Server (Axum)
    │   ├── app.rs              # Router Dashboard (SSE log packet realtime, tạo tài khoản)
    │   ├── server_control.rs   # Điều khiển server (start/stop)
    │   └── static/             # Asset tĩnh (htmx.min.js)
    ├── battle/                 # Battle Engine: Turn-based combat, RNG, damage math, targeting
    │   ├── engine.rs           # Core battle engine (turn loop)
    │   ├── manager.rs          # Battle manager
    │   ├── runner.rs           # Battle runner (vòng đấu, hành động)
    │   ├── service.rs          # Service layer (battle requests/rewards)
    │   ├── damage.rs           # Damage math
    │   ├── targeting.rs        # Chọn mục tiêu
    │   ├── rng.rs              # Random number generation (battle RNG)
    │   ├── construction.rs     # Khởi tạo battle instances
    │   └── packets.rs          # Packet battle đi/đến client
    └── data/                   # Loader dữ liệu tĩnh
        ├── loader.rs           # Loader chính (GameData, gate DataLoaded)
        ├── ini.rs              # Đọc INI files
        ├── tables.rs           # Load bảng dữ liệu
        └── texps.rs            # Load công thức tăng điểm kinh nghiệm (TEXP)
```
