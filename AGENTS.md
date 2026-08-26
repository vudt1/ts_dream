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

## Hướng Dẫn Kiểm Thử (Testing Guide)

Toàn bộ test suite được tổ chức tập trung trong thư mục `tests/` (không chứa khối `#[cfg(test)]` inline trong `src/`):

- **Chạy toàn bộ tests**:
  ```bash
  cargo test --all-targets --no-fail-fast
  ```
- **Chạy nhóm Golden Packet Diffing (Replay không cần DB)**:
  ```bash
  cargo test --test golden --test golden_suite --test battle_golden
  ```
- **Chạy nhóm Eve Script Engine**:
  ```bash
  cargo test --test eve_engine
  ```
- **Chạy nhóm Binary Codecs & Data Loaders**:
  ```bash
  cargo test --test codecs --test data --test data_loader_test
  ```
- **Chạy nhóm Handlers & Systems**:
  ```bash
  cargo test --test handlers_test --test server_state_test --test protocol_test --test battle_engine_test
  ```
- **Chạy kiểm thử Database Repositories (với MySQL sống)**:
  ```bash
  TS_TEST_DB_URL=mysql://root:password@localhost:3306/ts_dream_test cargo test --test db_repositories --test db_persist_test
  ```

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
├── Data/                       # Dữ liệu tĩnh game (Item.dat, Npc.dat, Warp.Dat, eve.emg 9.8MB, Formula.Dat, BlissBag.Dat, Compound.Dat, Astrolabe.Dat, CityEx.Dat, EVOStatus.Dat, v.v.)
├── templates/
│   └── dashboard.html          # Template HTML duy nhất cho Web Dashboard (Askama + HTMX)
│
├── spec/
│   └── codebase_design.md      # Thiết kế kiến trúc ban đầu
├── migrations/
│   ├── 0001_init.sql           # SQLx migration schema legacy MySQL 8
│   └── 0002_modern_schema.sql  # SQLx migration schema 3NF MySQL 8 (characters 1:1, inventories 20-col, character_pets 4 kho, missions, flags)
├── golden/                     # 18 golden packets (01-hello → 18-player-trade) để diffing khi test
├── tests/                      # Toàn bộ Integration & Unit Tests tập trung
│   ├── golden.rs / golden_suite.rs / battle_golden.rs  # Golden diffing (replay không cần DB)
│   ├── codecs.rs               # Test ThingData (35B), PlayerCard, FriendExtra, BattleRoleSerializer
│   ├── data.rs                 # Test nạp dữ liệu tĩnh (.Dat) & container eve.emg
│   ├── data_loader_test.rs     # Test chi tiết từng bộ Data Loader nhị phân
│   ├── eve_engine.rs           # Test 4-tier Eve Script Engine & AutoChain
│   ├── protocol_test.rs        # Test PacketReader, PacketWriter, Codec, Framing
│   ├── handlers_test.rs        # Test các modular opcode handlers
│   ├── battle_engine_test.rs   # Test Battle Engine, grid, turns, targeting, RNG, damage
│   ├── server_state_test.rs    # Test session state, registry, character sheet, drops
│   ├── db_repositories.rs      # Test MySQL 3NF Repositories & Transactions
│   ├── db_persist_test.rs      # Test persistence layer
│   ├── web_dashboard.rs        # Test Web admin dashboard
│   └── common/mod.rs           # Test harness & helpers dùng chung
└── src/
    ├── main.rs                 # Entry point: Config → MySQL bootstrap → seed map drops → Web Admin (8090) + Game TCP (6414) + AutoSave
    ├── lib.rs                  # Module root cho thư viện ts_dream
    ├── config.rs               # Xử lý file cấu hình + env TS_* (port, db URL, data_dir, db_auto_create)
    ├── state.rs                # AppState chia sẻ dữ liệu qua Arc<RwLock<AppState>>
    ├── error.rs                # Định nghĩa lỗi
    ├── encoding.rs             # Xử lý mã hóa VISCII 1.1 / Big5 / UTF-8
    ├── harness.rs              # Test harness hỗ trợ kiểm thử packet capture diffing
    ├── db/                     # Repository layer — mọi SQL tập trung, unit-testable
    │   ├── mod.rs              # Tổ chức module db
    │   ├── pool.rs             # MySQL Pool, auto-create database & chạy SQLx migration khi boot
    │   ├── accounts.rs         # Truy vấn accounts (login)
    │   ├── players.rs          # Truy vấn/transaction players + bảng gameplay + item_code
    │   ├── persist.rs          # Ghi-through players/skills/items/pets (no-op khi Option<&Pool> là None)
    │   ├── quest.rs            # Truy vấn quest legacy
    │   ├── item_code.rs        # Nhận mã quà (item_code), degrade khi không có DB
    │   └── modern/             # Schema 3NF: models + repository traits + MySQL impls + transactions nguyên tử (trade/shop/bank)
    ├── protocol/               # Bộ mã hóa/giải mã XOR 0xAD, PacketReader, PacketWriter, Codecs
    │   ├── mod.rs              # Hằng số giao thức (MAGIC, XOR_KEY, MIN_VERSION, MAX_LEVEL)
    │   ├── frame.rs            # Phân tách khung tin (Frame F4 44)
    │   ├── codec.rs            # Codec XOR 0xAD & opcode/subcode
    │   ├── reader.rs           # PacketReader Zero-copy Little-Endian & VISCII string
    │   ├── writer.rs           # PacketWriter Fluent builder buffer nhị phân
    │   ├── encoder.rs          # Primitive encoders
    │   └── codecs/             # Domain codecs
    │       ├── mod.rs          # Export codecs
    │       ├── thing_data.rs   # ThingDataCodec (35-byte item struct chuẩn hóa)
    │       ├── player_info.rs  # PlayerCard (15B + name) & FriendExtra (20B)
    │       └── battle_role.rs  # BattleRoleSerializer (42B base header combatants)
    ├── eve/                    # 4-Tier Eve Script Engine (Động cơ sự kiện kịch bản)
    │   ├── mod.rs              # Export Eve Engine modules
    │   ├── state.rs            # PlayerEventState snapshot & EveStateBuilder
    │   ├── evaluator.rs        # EveConditionEvaluator (15 condition classes, 6 ops)
    │   ├── resolver.rs         # EveChainResolver (ghép chuỗi AND & sắp xếp 4 tầng ưu tiên)
    │   ├── group.rs            # GroupData weighted random resolver
    │   └── auto_chain.rs       # EveAutoChainEngine (4 lớp chống lặp: same cond, re-question, re-battle, dup item)
    ├── server/                 # TCP Server listener (Port 6414), Session, Systems & Handlers
    │   ├── dispatcher.rs       # Bộ điều phối Level-1 phân luồng theo Main Opcode
    │   ├── response.rs         # ResponseSender abstraction (đóng gói typed response packets)
    │   ├── player_state.rs     # PlayerStateManager (quản lý HP, SP, base stats, equipment bonus)
    │   ├── trade_system.rs     # TradeSystem (giao dịch 2 người chơi 2-phase atomic)
    │   ├── auto_save.rs        # AutoSaveService (background interval task tự động lưu dirty state mỗi 3 phút)
    │   ├── session.rs          # Quản lý phiên kết nối (Session, InventoryItem, offline state)
    │   ├── spawn.rs            # Spawner quản lý đối tượng/entity trong map
    │   ├── character_sheet.rs  # Bảng chỉ số nhân vật
    │   ├── inventory.rs        # Quản lý túi đồ nhân vật
    │   ├── pet_box.rs          # Quản lý hộp thú nuôi
    │   ├── map_drops.rs        # Registry drop vật phẩm trên map (ItemOnMap.txt)
    │   └── handlers/           # Level-2 Modular Opcode Handlers
    │       ├── mod.rs          # Khai báo và dispatch các handler con
    │       ├── login.rs        # Đăng nhập / xác thực (Opcode 0x01)
    │       ├── character.rs    # Tạo / xóa nhân vật (Opcode 0x01)
    │       ├── movement.rs     # Di chuyển / chuyển map (Opcode 0x05, 0x06)
    │       ├── chat.rs         # Chat + lệnh slash (Opcode 0x02)
    │       ├── party.rs        # Nhóm / Party / Quân sư (Opcode 0x14)
    │       ├── skills.rs       # Học kỹ năng, reborn (Opcode 0x1C)
    │       ├── stats.rs        # Phân bổ chỉ số (Opcode 0x08)
    │       ├── battle.rs       # Battle handler (Opcode 0x0B, 0x32)
    │       ├── quest.rs        # Nhiệm vụ legacy (quest) + hội thoại H6 (Opcode 0x18)
    │       ├── npc_event.rs    # Tích hợp Eve Engine xử lý sự kiện NPC/Door/Fight (Opcode 0x18)
    │       ├── inventory.rs    # Túi đồ / sắp xếp / thả vật phẩm (Opcode 0x17)
    │       ├── use_item/       # Sử dụng vật phẩm (Opcode 0x17 sub 15)
    │       │   ├── mod.rs      # Dispatcher & helper chung
    │       │   ├── rewards.rs  # Lucky-box random rewards + fixed multi-item packs
    │       │   ├── books.rs    # Sách skill / Texp / god / pet-stat / HP-store
    │       │   ├── misc.rs     # Doll summon, dice, special frames, no-op ids, full-heal
    │       │   └── reborn.rs   # Reborn-by-item + close socket
    │       ├── shops.rs        # Cửa hàng NPC & người chơi (Opcode 0x17 sub 30..33, 0x0C)
    │       ├── trade_storage.rs# Giao dịch & kho (Opcode 0x17 sub 51..52, 0x19, 0x1A)
    │       ├── pet_actions.rs  # Hành động thú nuôi (Opcode 0x0F, 0x2C)
    │       ├── expressions.rs  # Biểu cảm (Opcode 0x04)
    │       ├── talk.rs         # Hội thoại NPC cơ bản (Opcode 0x18)
    │       └── system.rs       # Hệ thống / ping / time (Opcode 0x00, 0x0A)
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
    │   ├── rng.rs              # Random number generation (3 luồng độc lập)
    │   ├── construction.rs     # Khởi tạo battle instances (Player vs NPC, TeamDef, PK)
    │   ├── npc_world.rs        # NPC tuần tra trên bản đồ (NpcOnMapWalk)
    │   └── packets.rs          # Packet battle đi/đến client
    └── data/                   # Binary Data Loaders & Data structures
        ├── loader.rs           # Loader chính (`GameData`, nạp .Dat & eve.emg, gate DataLoaded)
        ├── reader.rs           # `DatReader` nhị phân (Little-Endian, XOR decoding, reverse string)
        ├── loaders/            # 9 bộ Binary Data Loaders
        │   ├── mod.rs          # Export loaders
        │   ├── item.rs         # `ItemDatLoader` (Item.dat 3.09 MB, 54 trường dữ liệu)
        │   ├── npc.rs          # `NpcDatLoader` (Npc.dat 612 KB, chỉ số, skill, drop)
        │   ├── eve.rs          # `EveDataLoader` (eve.emg 9.8 MB, 3,823 scene entries, 9 sections)
        │   ├── warp.rs         # `WarpDatLoader` (Warp.Dat / Warp_C.dat)
        │   ├── formula.rs      # `FormulaDatLoader` (Formula.Dat)
        │   ├── bliss_bag.rs    # `BlissBagDatLoader` (BlissBag.Dat)
        │   ├── compound.rs     # `CompoundDatLoader` (Compound.Dat)
        │   ├── astrolabe.rs    # `AstrolabeDatLoader` (Astrolabe.Dat)
        │   ├── city_ex.rs      # `CityExDatLoader` (CityEx.Dat)
        │   └── evo_status.rs   # `EVOStatusDatLoader` (EVOStatus.Dat)
        ├── ini.rs              # Đọc INI files legacy
        ├── tables.rs           # Bảng tra cứu dữ liệu (Element, Job, Exp, etc.)
        └── texps.rs            # Load công thức tăng điểm kinh nghiệm (TEXP)
```
