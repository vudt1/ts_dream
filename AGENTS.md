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
- **Thư viện bổ sung**: Serde, Serde JSON, TOML 0.8, Tracing, Anyhow, Thiserror, Hex, Chrono, Futures.

---

## Cấu trúc Codebase (Codebase Structure)

```text
ts_dream/
├── Cargo.toml                  # Khai báo crate & phụ thuộc
├── CONTEXT.md                  # Từ vựng miền (Domain Glossary & Ubiquitous Language)
├── AGENTS.md                   # Hướng dẫn Agent, Tech Stack & Cấu trúc Codebase
├── templates/                  # Template HTML cho Web Dashboard (Askama)
├── docs/                       # Tài liệu đặc tả & cấu hình Agent
│   ├── rust_porting_spec.md    # Đặc tả chi tiết porting server (29 opcodes, battle engine, VISCII)
│   └── agents/                 # Quy chuẩn kỹ năng Agent (domain, issue-tracker, triage)
├── spec/
│   └── codebase_design.md      # Thiết kế kiến trúc ban đầu
└── src/
    ├── main.rs                 # Entry point: Khởi tạo Config, MySQL Pool, AppState, Web Admin (8090) & Game TCP Server (6414)
    ├── lib.rs                  # Module root cho thư viện ts_dream
    ├── config.rs               # Xử lý file cấu hình (port, db URL, data_dir)
    ├── state.rs                # AppState chia sẻ dữ liệu qua Arc<RwLock<AppState>>
    ├── error.rs & encoding.rs  # Định nghĩa lỗi & Xử lý mã hóa VISCII 1.1
    ├── harness.rs              # Test harness hỗ trợ kiểm thử packet capture diffing
    ├── db/
    │   └── pool.rs             # Kết nối MySQL Pool & Chạy SQLx migration khi boot
    ├── protocol/               # Bộ mã hóa/giải mã XOR 0xAD, phân tách khung tin (Frame F4 44, Opcode/Subcode)
    ├── server/                 # TCP Server listener (Port 6414), Session management & Spawner
    │   ├── handler.rs          # Phân luồng opcode chính
    │   └── handlers/           # Handler chi tiết từng nhóm Opcode (login, chat, move, battle, quest, inventory, shops,...)
    ├── web/                    # Web Admin Server (Axum), ServerControl & Router Dashboard
    ├── battle/                 # Battle Engine: Turn-based combat, RNG, damage math, targeting, battle runner & manager
    └── data/                   # Loader dữ liệu tĩnh (INI files, tables, talks, NPC data)
```
