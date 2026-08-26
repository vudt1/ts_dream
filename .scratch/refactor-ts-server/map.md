# Refactor Rust TS Server theo Kiến Trúc Kotlin TS Mobile Server

## Destination

Tái cấu trúc toàn diện Game Server TS Dream (Rust) dựa trên mô hình kiến trúc sạch và module hóa của TS Mobile Server (Kotlin), tập trung vào:
1. Hệ thống xử lý Payload & Packet Codec (PacketReader, PacketWriter, Codec chuyên biệt theo entity).
2. Tầng đọc dữ liệu nhị phân game chuẩn (.Dat với XOR/offset tables, Eve.emg container & script engine).
3. Redesign Schema cơ sở dữ liệu MySQL 8 & Repository Layer (Characters, Inventories với 35-byte ThingData, CharacterPets, Skills, Mails, Friends, Dispatches, BitFlags...).
4. Phân rã Gamelogic thành các Domain System độc lập và Handler chuyên trách theo Opcode.
5. Di chuyển toàn bộ test case từ `src/` sang `tests/`, loại bỏ chú thích legacy C#, cập nhật `AGENTS.md` & `CONTEXT.md`.
Đảm bảo 100% giữ nguyên các ràng buộc cốt lõi của TS PC: TCP port 6414, XOR 0xAD, header F4 44, LE length, MIN_VERSION 186, prefix "VN", server "TSVN", MAX_LEVEL 200, Web Dashboard, và vượt qua toàn bộ test suite.

## Notes

- **Domain**: TS Online PC / Mobile Game Server Architecture, Binary Protocol, Turn-based RPG Combat Engine.
- **Tham chiếu chuẩn**: Kotlin TS Mobile Server tại `ts_mobile_server/`.
- **Ràng buộc bất biến TS PC (PC Invariants)**:
  - Async runtime Tokio 1 (`full`), mô hình 1 client connection = 1 Tokio task.
  - TCP Port: 6414 (Game Server) & Port 8090 (Web Admin Dashboard).
  - Khung gói tin TS PC: Header `F4 44`, độ dài 2 bytes Little-Endian, mã hóa XOR toàn gói `0xAD`.
  - Hằng số hệ thống: `MIN_VERSION = 186`, Server/Account prefix `"VN"`, Server name `"TSVN"`, `MAX_LEVEL = 200`.
  - Bảng mã văn bản client PC: VISCII 1.1 / Big5 / UTF-8.
  - Cơ sở dữ liệu: Tiếp tục lưu trữ tên nhân vật và text dạng raw bytes / `latin1_bin` để giữ tính tương thích tuyệt đối.
- **Tiêu chuẩn chất lượng**:
  - Không phá vỡ các golden test hiện có (`golden/01` → `golden/18`).
  - Mọi test case phải nằm tại `tests/`, không để `#[cfg(test)]` nằm rải rác trong `src/`.
  - Không còn chú thích tham chiếu C# cũ trong code Rust.

## Implementation Tickets (Tracked in issues/)

- [01 — Binary Packet Reader and Writer](issues/01-binary-packet-reader-writer.md) (Completed ✅)
- [02 — Standardized Domain Codecs](issues/02-domain-codecs-thingdata-player-battle.md) (Completed ✅)
- [03 — Binary Dat Reader and Data Loaders](issues/03-binary-dat-reader-and-loaders.md) (Completed ✅)
- [04 — Eve.emg Container Parser and Script Models](issues/04-eve-emg-container-loader.md) (Completed ✅)
- [05 — 4-Tier Eve Script Engine and Auto-Chain Resolver](issues/05-eve-script-engine-and-auto-chain.md) (Completed ✅)
- [06 — MySQL Schema Migration 3NF and Repository Layer](issues/06-mysql-schema-migration-and-repositories.md) (Completed ✅)
- [07 — Two-Tier Dispatcher and Modular Handlers](issues/07-two-tier-dispatcher-and-modular-handlers.md) (Completed ✅)
- [08 — Test Suite Migration and Legacy C# Comment Cleanup](issues/08-test-migration-and-csharp-comment-cleanup.md) (Completed ✅)
- [09 — Documentation and Domain Updates](issues/09-documentation-and-domain-updates.md) (Completed ✅)

## Out of scope

- Thay đổi định dạng wire header từ `F4 44` sang `C0 91` (header của client TS Mobile).
- Thay thế Tokio runtime hoặc chuyển sang mô hình thread-per-client.
- Chỉnh sửa logic hoặc giao diện Web Admin Dashboard (Axum + Askama + HTMX tại port 8090).
- Thay đổi cấu trúc và quy tắc phân giải cổng TCP 6414.
