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
- **Database**: SQLite (kết nối qua SQLx 0.8 với `sqlite`, `runtime-tokio-rustls`, `migrate`), kiến trúc Dual-Pool: `read` (tối đa 16 sessions song song) và `write` (độc quyền 1 session chống `SQLITE_BUSY`), chế độ WAL (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`). File cơ sở dữ liệu `DB/ts_dream.db`. Tự động flush WAL về file chính định kỳ và khi dừng TCP server/thoát tiến trình.
- **Mã hóa & Định dạng Wire**: Giao thức TS Online (Header `F4 44`, XOR key `0xAD`, VISCII 1.1 text encoding).
- **Thư viện bổ sung**: Serde, Serde JSON, TOML 0.8, Tracing + tracing-subscriber (env-filter), Anyhow, Thiserror 2, Hex, Chrono, Futures. Dev: Tempfile, Tower.

---

## Hướng Dẫn Kiểm Thử (Testing Guide)

- **Vị trí duy nhất cho test**: Mọi unit test / integration test mới **phải** đặt trong thư mục `tests/` ở repo root. **Cấm** `#[cfg(test)]` inline trong `src/` (kể cả `mod tests` nhỏ). Nếu cần test pure-logic, tạo file `tests/<feature>_test.rs` và import qua `ts_dream::...` public API.
- **Tách biệt DB Test và DB Production**: Tuyệt đối **không** chạy test ghi đè lên file production `DB/ts_dream.db`. Test chạm cơ sở dữ liệu có thể thực hiện theo 2 cách:
  1. **In-memory SQLite (`sqlite::memory:?cache=shared`)**: Tối ưu cho unit test / repository test vì tốc độ thực thi tức thì, 0 disk I/O, độc lập giữa các test runner và tự hủy sau khi xong.
  2. **File DB riêng cho test (`DB/ts_dream_test.db` hoặc tempfile)**: Phù hợp cho integration test kiểm thử WAL checkpoint hoặc persist. Biến môi trường chỉ định:
     ```bash
     TS_TEST_DB_URL=sqlite://DB/ts_dream_test.db cargo test --test <test_name>
     ```
     File `*test.db` luôn được `.gitignore` loại trừ, không commit vào kho mã nguồn.
- **Khi chạy lại test suite** (sau khi scaffold lại):
  ```bash
  cargo test --all-targets --no-fail-fast
  ```

---

## Cấu trúc Codebase (Codebase Structure)

```text
ts_dream/
├── Cargo.toml                  # Khai báo crate & phụ thuộc
├── build.rs                    # Đóng gói Data/ và DB/ vào cạnh binary khi cargo build
├── CONTEXT.md                  # Từ vựng miền (Domain Glossary & Ubiquitous Language)
├── AGENTS.md                   # Hướng dẫn Agent, Tech Stack & Cấu trúc Codebase
├── LICENSE & README.md         # Giấy phép & hướng dẫn dựng dự án
├── TS_Server_OP_Code_basic.md  # Đặc tả opcode giao thức TS Online tham khảo
├── DB/                         # Thư mục chứa cơ sở dữ liệu SQLite (ts_dream.db, ts_dream.db-wal, ts_dream.db-shm)
├── Data/                       # Dữ liệu tĩnh game (Item.dat, Npc.dat, Warp.Dat, eve.emg 9.8MB, Formula.Dat, BlissBag.Dat, Compound.Dat, Astrolabe.Dat, CityEx.Dat, EVOStatus.Dat, v.v.)
├── templates/
│   └── dashboard.html          # Template HTML duy nhất cho Web Dashboard (Askama + HTMX)
│
├── spec/
│   └── codebase_design.md      # Thiết kế kiến trúc ban đầu
├── migrations/
│   ├── 0001_init.sql           # SQLx migration legacy
│   └── 0003_production_domain.sql # Mở rộng production legacy
├── golden/                     # 18 golden packets (01-hello → 18-player-trade) để diffing khi test (giữ lại, không phải test source)
├── tests/                      # Thư mục test tập trung — **hiện trống** (tạm thời không cần test)
│   └── .gitkeep                # Scaffold sẵn cho test tương lai; mọi test mới phải đặt ở đây, cấm #[cfg(test)] trong src/
└── src/
    ├── main.rs                 # Entry point: Config → SQLite bootstrap → seed map drops → Web Admin (8090) + Game TCP (6414) + AutoSave + WAL Flush
    ├── lib.rs                  # Module root cho thư viện ts_dream
    ├── config.rs               # Xử lý file cấu hình + env TS_* (port, database_url, wal_checkpoint_interval_secs, data_dir)
    ├── state.rs                # AppState chia sẻ dữ liệu qua Arc<RwLock<AppState>>
    ├── error.rs                # Định nghĩa lỗi
    ├── encoding.rs             # Xử lý mã hóa VISCII 1.1 / Big5 / UTF-8
    ├── harness.rs              # Test harness hỗ trợ kiểm thử packet capture diffing
    ├── db/                     # Repository layer — mọi SQL tập trung, unit-testable
    │   ├── mod.rs              # Tổ chức module db
    │   ├── pool.rs             # SQLite Dual-Pool (DbPool: read + write), WAL pragmas, periodic checkpoint & shutdown flush
    │   ├── accounts.rs         # Truy vấn accounts (login)
    │   ├── persist.rs          # Ghi-through players/skills/items/pets (no-op khi Option<&DbPool> là None)
    │   ├── item_code.rs        # Nhận mã quà (item_code), degrade khi không có DB
    │   └── modern/             # Schema 3NF: models + repository traits + SQLite impls (src/db/modern/sqlite/) + transactions nguyên tử
    ├── protocol/               # Bộ mã hóa/giải mã XOR 0xAD, PacketReader, PacketWriter, Codecs
    │   ├── mod.rs              # Hằng số giao thức (HEADER_TS_MAGIC, XOR_KEY, MIN_VERSION, MAX_LEVEL)
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

## Tool Execution Constraints
- DO NOT chain multiple search queries or tool calls in parallel.
- Execute codebase searches and file operations sequentially.
- Không tự ý commit source, user sẽ tự commit source thủ công.
---