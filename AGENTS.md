# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as markdown files under `.scratch/<feature-slug>/` in this repo (no git remote yet). See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

### Triage labels

Five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).

### Domain docs

Single-context layout — one [`GLOSSARY.md`](GLOSSARY.md) + `docs/adr/` at the repo root. See [`docs/agents/domain.md`](docs/agents/domain.md).

### Coding standards

Mọi quy chuẩn viết mã, đồng thời (Two-Phase State Lock), đồng bộ Registry, wire parity và kiểm thử toàn vẹn slot **bắt buộc** phải tuân thủ [`CODING_STANDARDS.md`](CODING_STANDARDS.md).

### Architecture ADRs

Đọc ADR tương ứng trong `docs/adr/` trước khi sửa phân hệ đó:

- Giao dịch nguyên tử, Two-Phase State Lock & 5 Containers (kinh tế, kho, trade): [`0005`](docs/adr/0005-economy-wire-parity-and-transactions.md).
- Battle protocol & vòng đời lượt: [`0006`](docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md).
- Compound / khảm lông (crafting): [`0007`](docs/adr/0007-compound-wire-parity-and-crafting-engine.md).
- Công thức PC vs mobile, đổi tên module legacy: [`0002`](docs/adr/0002-pc-server-mobile-formula.md).

### Wire protocol reference

Tra cứu payload theo opcode: [`docs/client-opcodes.md`](docs/client-opcodes.md) (ánh xạ client ↔ server), `spec/exhaustive_by_opcode/opcode_<XX>_full_payloads.md` (payload đầy đủ từng opcode), `spec/client_*_handler.md` (đặc tả handler client đã dịch ngược).

---

## Tech Stack

- **Ngôn ngữ & Runtime**: Rust (2021 edition) + Tokio 1 (`full`).
- **Web Admin**: Axum 0.8 + Askama 0.12 + HTMX, port 8090, chung Tokio runtime với Game TCP (port 6414).
- **Database**: SQLite qua SQLx 0.8, Dual-Pool `read` (16 kết nối) / `write` (1 kết nối độc quyền, chống `SQLITE_BUSY`), WAL + `synchronous = NORMAL`. File `DB/ts_dream.db`; WAL được checkpoint định kỳ và khi dừng server.
- **Wire**: Header `F4 44`, XOR key `0xAD`, text VISCII 1.1.
- **Cấu hình**: file TOML + biến môi trường `TS_*` (xem [`src/config.rs`](src/config.rs)).

---

## Hướng Dẫn Kiểm Thử (Testing Guide)

Chi tiết đầy đủ tại `CODING_STANDARDS.md` §3. Các điểm thao tác hằng ngày:

- **Vị trí test**: mọi test đặt trong `tests/<feature>_test.rs`, import qua public API `ts_dream::...`; helper dùng chung ở `tests/common/`. `src/` không chứa `#[cfg(test)]` (chốt chặn bởi `tests/architecture_guardrails_test.rs`).
- **Negative test**: mỗi User Story trong spec có test từ chối tương ứng, assert cả thông báo lỗi/Toast gửi về client.
- **DB test**: gọi `common::test_db::setup_test_db()` — DB chính thức `DB/ts_dream_test.db` (override bằng `TS_TEST_DB_URL`), tự reset schema và khóa tuần tự (chi tiết §3.4). Production `DB/ts_dream.db` chỉ dành cho server.
- **Vật phẩm & đa phiên**: số lượng lẻ + assert slot phụ cận nguyên vẹn; P2P trade đồng bộ tự nhiên qua dispatcher (`registry_update_if_online`).
- **Chu kỳ chạy**: khi phát triển chạy `cargo test --test <feature>_test`; chạy `cargo test --all-targets --no-fail-fast` một lần ở bước chốt trước handoff.

---

## Cấu trúc Codebase (Codebase Structure)

Bảng opcode → handler: nguồn sự thật duy nhất là [`src/server/dispatcher.rs`](src/server/dispatcher.rs) (Level-1) và hàm `handle_*` trong từng handler (Level-2 sub-opcode); hằng số `OP_*` ở [`src/protocol/mod.rs`](src/protocol/mod.rs).

```text
ts_dream/
├── AGENTS.md / CODING_STANDARDS.md / GLOSSARY.md
├── PRODUCTION_RUNBOOK.md       # Vận hành production (port, profile PC/aLogin, quy trình chạy)
├── build.rs                    # Copy Data/ và DB/ cạnh binary khi cargo build
├── Data/                       # Dữ liệu tĩnh game: *.Dat/*.dat, eve.emg, ground.mmg, *.json
├── DB/                         # SQLite: ts_dream.db (production), ts_dream_test.db (test, gitignored)
├── migrations/0001_init.sql    # Schema SQLx
├── templates/dashboard.html    # Template Askama duy nhất của Web Dashboard
├── docs/
│   ├── adr/                    # ADR 0001..000N
│   ├── agents/                 # Hướng dẫn issue tracker, triage labels, domain docs
│   └── client-opcodes.md       # Ánh xạ opcode client Delphi ↔ server
├── spec/                       # Đặc tả dịch ngược: exhaustive_by_opcode/, client_*_handler.md, server_main_opcode.md
├── client_delphi/aLogin.exe    # Binary client PC tham chiếu
├── client_pseudo_c/            # Pseudo-C/ASM decompile từ aLogin.exe (tra cứu, không sửa)
├── examples/                   # Công cụ debug: dump_eve.rs, verify_skill_pc.rs
├── tests/                      # Toàn bộ integration/unit test (<feature>_test.rs)
└── src/
    ├── main.rs                 # Entry: Config → SQLite → seed map drops → Web (8090) + Game TCP (6414) + AutoSave + WAL flush
    ├── lib.rs                  # Module root (khai báo toàn bộ module public)
    ├── config.rs               # Config TOML + env TS_*
    ├── state.rs                # AppState dùng chung (Arc<RwLock<AppState>>)
    ├── error.rs / encoding.rs  # Lỗi chung / VISCII 1.1, Big5, UTF-8
    ├── protocol/               # Frame F4 44, codec XOR 0xAD, PacketReader/Writer, encoder, hằng số OP_*
    │   ├── system_alert.rs     # Frame cảnh báo/ngắt kết nối (0x00, chỉ S→C)
    │   └── codecs/             # thing_data (item 35B), player_info, ehuman, npc_talk (0x14), quest_sync (0x18)
    ├── crafting/               # Domain engine thuần (không I/O)
    │   ├── basic/              # Compound theo Compound.Dat: algorithm, catalog, validator
    │   └── socketing/          # Khảm lông +1..+10: feather, rate_table
    ├── eve/                    # Eve Script Engine: state, evaluator, resolver, group, auto_chain
    ├── battle/                 # Battle engine: engine, manager, runner, service, combat_formula,
    │                           # damage, targeting, rng, construction, npc_world, packets
    ├── data/                   # Loader dữ liệu nhị phân
    │   ├── loader.rs           # GameData: nạp toàn bộ Data/
    │   ├── reader.rs           # DatReader (LE, XOR, reverse string)
    │   ├── loaders/            # Một file mỗi định dạng (item, npc, eve, skill_pc, warp, ground, rank, compound, ...)
    │   └── ini.rs / tables.rs / texps.rs
    ├── db/                     # Repository layer — mọi SQL tập trung tại đây
    │   ├── pool.rs             # DbPool dual-pool, WAL checkpoint & shutdown flush
    │   ├── accounts.rs / persist.rs / item_code.rs
    │   ├── catalog.rs          # Asset provenance (file Data/ đã nạp, size, hash)
    │   ├── domain.rs           # Truy vấn guild, world boss, ghi audit
    │   └── modern/             # Schema 3NF: model, traits, transactions nguyên tử, sqlite/ (impl)
    ├── server/
    │   ├── dispatcher.rs       # Level-1: route theo main opcode
    │   ├── session.rs          # Session, registry online, lock_player_operations
    │   ├── spawn.rs            # Entity trên map, sys_msg_frame
    │   ├── response.rs / player_state.rs / character_sheet.rs / inventory.rs / pet_box.rs
    │   ├── trade_system.rs     # Trade 2 người chơi 2-phase atomic
    │   ├── map_drops.rs / auto_save.rs
    │   ├── gm.rs               # Lệnh GM qua chat
    │   └── handlers/           # Level-2 handlers theo miền: login, character, movement, chat,
    │                           # party, skills, stats, battle, talk, npc_event, quest, quest_sync,
    │                           # inventory, use_item/, crafting, shops, trade_storage,
    │                           # pet_actions, expressions, system, unimplemented (opcode chưa port)
    └── web/                    # Axum: app.rs (router, SSE log), server_control.rs, static/htmx.min.js
```

---

## Quy Chuẩn Đồng Thời & Trạng Thái Phiên (Concurrency Rules)

Chi tiết tại `CODING_STANDARDS.md` §1–2. Ba hard guardrail:

- **Lock ordering**: thao tác 2 người chơi (Trade, Party, PK, mua sạp) gọi `lock_player_operations([min(id1, id2), max(id1, id2)]).await` **trước khi** đọc `registry_get(partner_id)`.
- **Đồng bộ registry**: sau biến động túi đồ/trạng thái, gọi duy nhất `crate::server::session::registry_update_if_online(&conn.session)`. Handler không chạm `lock_online_sessions().insert(...)` (chốt chặn bởi `tests/architecture_guardrails_test.rs`).
- **Thông báo hệ thống**: dùng `crate::server::spawn::sys_msg_frame(msg)` (kênh `020102`); kênh `020B` không dùng cho mục đích này.

## Tool Execution Constraints

- User tự commit thủ công; agent để thay đổi ở working tree.
- Tìm kiếm tôn trọng `.ignore` (Data nhị phân, `Data/npc_drops.json`, `Data/Warps.txt`, `DB/`, `target/`, `client_delphi/`); khi grep, giới hạn trong `src/`, `tests/`, `docs/`, `spec/` để tiết kiệm token và tránh treo tiến trình.
