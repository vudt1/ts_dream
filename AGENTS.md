# AGENTS.md

## Context Pointers

- **Issue tracker & triage**: `.scratch/<feature-slug>/`; hướng dẫn tại [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md) và [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- **Domain terminology**: [`GLOSSARY.md`](GLOSSARY.md) + `docs/adr/` (xem [`docs/agents/domain.md`](docs/agents/domain.md)).
- **Coding & review standards**: [`CODING_STANDARDS.md`](CODING_STANDARDS.md) (Two-Phase State Lock, `registry_update_if_online`, `sys_msg_frame`, wire error parity, test & DB rules).
- **Architecture ADRs** (đọc trước khi sửa phân hệ):
  - Kinh tế, kho, trade: [`0005`](docs/adr/0005-economy-wire-parity-and-transactions.md)
  - Battle protocol & lượt: [`0006`](docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md)
  - Crafting (Compound / khảm lông): [`0007`](docs/adr/0007-compound-wire-parity-and-crafting-engine.md)
  - Công thức PC vs mobile: [`0002`](docs/adr/0002-pc-server-mobile-formula.md)
- **Wire protocol**: [`docs/client-opcodes.md`](docs/client-opcodes.md), `spec/exhaustive_by_opcode/opcode_<XX>_full_payloads.md`, `spec/client_*_handler.md`. Bảng opcode thật nằm tại [`src/server/dispatcher.rs`](src/server/dispatcher.rs) và hằng số `OP_*` tại [`src/protocol/mod.rs`](src/protocol/mod.rs).
- **Production ops**: [`PRODUCTION_RUNBOOK.md`](PRODUCTION_RUNBOOK.md).

## Tech Stack

- **Rust 2021 + Tokio 1**: Game TCP port `6414`, Web Admin (Axum 0.8 + Askama 0.12 + HTMX) port `8090`.
- **SQLite (SQLx 0.8)**: Dual-pool `read` (16) / `write` (1), WAL + `synchronous = NORMAL` tại `DB/ts_dream.db`.
- **Wire & Config**: Frame `F4 44`, XOR `0xAD`, text VISCII 1.1; cấu hình TOML + env `TS_*` ([`src/config.rs`](src/config.rs)).
- **Build & Data**: `build.rs` đồng bộ `Data/` và `DB/ts_dream.db` cạnh binary (không watch thư mục `DB/` để tránh rebuild khi test ghi `DB/ts_dream_test.db`).

## Testing

- Chi tiết quy chuẩn kiểm thử xem [`CODING_STANDARDS.md`](CODING_STANDARDS.md) §3; chốt chặn cơ học tại [`tests/architecture_guardrails_test.rs`](tests/architecture_guardrails_test.rs).
- Các test suite chuẩn (thêm test mới vào submodule dưới `tests/<domain>/`, không tạo `*_test.rs` lẻ ở `tests/`):
  `architecture_guardrails_test`, `battle_test`, `compound_test`, `data_loaders_test`, `db_test`, `economy_inventory_test`, `eve_quest_test`, `events_arena_test`, `server_network_test`, `server_stop_test`, `skill_job_system_test`.
- Khi phát triển: `cargo test --test <domain>_test`; trước handoff: `cargo test --all-targets --no-fail-fast`.

## Tool Execution Constraints

- User tự commit thủ công; giữ thay đổi ở working tree.
- Khi grep/tìm kiếm, giới hạn trong `src/`, `tests/`, `docs/`, `spec/` (tránh `Data/`, `DB/`, `target/`, `client_delphi/`).
- Chỉ chạy `cargo update`, `cargo clean` hoặc xóa `Cargo.lock` khi user yêu cầu cụ thể.
