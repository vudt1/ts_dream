# Handoff: TS Server Refactor (Rust Port based on Kotlin TS Mobile Architecture)

## 1. Executive Summary & Objective

Dự án tái cấu trúc toàn diện Game Server TS Dream (Rust) phỏng theo kiến trúc của Kotlin TS Mobile Server (`ts_mobile_server`), chuyển đổi từ cơ chế ghép chuỗi Hex legacy sang kiến trúc Binary Buffer, Data Loaders nhị phân, Schema 3NF, Domain Systems và Two-tier Opcode Dispatcher.

Toàn bộ ràng buộc cốt lõi của TS PC được bảo toàn 100%:
- **Runtime**: Tokio 1 (`full`) async runtime, 1 client connection = 1 Tokio task.
- **Ports**: TCP Game Server: `6414` | Web Admin Dashboard (Axum + Askama + HTMX): `8090`.
- **Wire Framing**: Header `F4 44`, độ dài 2 bytes Little-Endian, mã hóa XOR toàn gói `0xAD`.
- **Text Encoding**: VISCII 1.1 / Big5 / UTF-8 | Lưu trữ MySQL `latin1_bin` để giữ nguyên raw bytes.
- **Hằng số Server**: `MIN_VERSION = 186`, prefix `"VN"`, name `"TSVN"`, `MAX_LEVEL = 200`.

---

## 2. Trạng Thái 9 Implementation Tickets

| Ticket ID | Tên Ticket | Trạng Thái | Test Coverage & Ghi Chú |
|---|---|---|---|
| [`01`](.scratch/refactor-ts-server/issues/01-binary-packet-reader-writer.md) | **Binary Packet Reader and Writer** | **COMPLETED ✅** | 32 unit tests pass 100% (`src/protocol/`) |
| [`02`](.scratch/refactor-ts-server/issues/02-domain-codecs-thingdata-player-battle.md) | **Standardized Domain Codecs** | **COMPLETED ✅** | 8 integration tests (`tests/codecs.rs`) + unit tests, `ThingData` (35B), `PlayerCard`/`FriendExtra`, `BattleRole` |
| [`03`](.scratch/refactor-ts-server/issues/03-binary-dat-reader-and-loaders.md) | **Binary Dat Reader and Data Loaders** | **COMPLETED ✅** | 14 tests pass 100% (`tests/data.rs`), 9 .Dat loaders |
| [`04`](.scratch/refactor-ts-server/issues/04-eve-emg-container-loader.md) | **Eve.emg Container Parser & Models** | **COMPLETED ✅** | 15 integration tests (`tests/data.rs`) + unit tests (`src/data/loaders/eve.rs`), đọc container 9.8 MB `Data/eve.emg` (3,823 scene entries) |
| [`05`](.scratch/refactor-ts-server/issues/05-eve-script-engine-and-auto-chain.md) | **4-Tier Eve Script Engine** | **COMPLETED ✅** | 47 tests pass 100% (`tests/eve_engine.rs`), `src/eve/` (state/evaluator/resolver/group/auto_chain) |
| [`06`](.scratch/refactor-ts-server/issues/06-mysql-schema-migration-and-repositories.md) | **MySQL Schema 3NF & Repositories** | **COMPLETED ✅** | 12 tests (`tests/db_repositories.rs`, unit luôn chạy; integration DB gate qua `TS_TEST_DB_URL`), `migrations/0002_modern_schema.sql` + `src/db/modern/` |
| [`07`](.scratch/refactor-ts-server/issues/07-two-tier-dispatcher-and-modular-handlers.md) | **Two-Tier Dispatcher & Handlers** | **COMPLETED ✅** | 18 tests (`tests/handlers_test.rs`), `response.rs` / `player_state.rs` / `trade_system.rs` / `auto_save.rs` / `handlers/npc_event.rs`; golden suite vẫn pass 100% |
| [`08`](.scratch/refactor-ts-server/issues/08-test-migration-and-csharp-comment-cleanup.md) | **Test Migration & C# Cleanup** | **COMPLETED ✅** | 331 unit tests chuyển khỏi `src/` vào 6 tệp `tests/` chuyên trách; 431 tests pass 100%, clippy 0 warning, golden pass |
| [`09`](.scratch/refactor-ts-server/issues/09-documentation-and-domain-updates.md) | **Documentation & Domain Updates** | **UNBLOCKED / READY** | Unblocked — 08 completed |

---

## 3. Chi Tiết Các Thành Phần Đã Hoàn Thành

### A. Protocol Layer & Domain Codecs (`src/protocol/`) — Tickets 01 & 02
- **`PacketReader<'a>`** ([`src/protocol/reader.rs`](src/protocol/reader.rs)):
  - Zero-copy cursor trên `&'a [u8]`.
  - Hỗ trợ đầy đủ primitive LE, VISCII pascal/fixed, bounds checking.
  - Tích hợp hàm giải mã domain: `read_thing_data()`, `read_player_card()`, `read_friend_extra()`, `read_battle_role()`.
- **`PacketWriter`** ([`src/protocol/writer.rs`](src/protocol/writer.rs)):
  - Fluent builder buffer nhị phân, framing header `F4 44`, mã hóa XOR `0xAD`.
  - Tích hợp hàm đóng gói domain: `write_thing_data()`, `write_player_card()`, `write_friend_extra()`, `write_battle_role()`.
- **`ThingDataCodec` & `ThingData`** ([`src/protocol/codecs/thing_data.rs`](src/protocol/codecs/thing_data.rs)):
  - Chuẩn hóa 35-byte item struct chuẩn xác theo Mobile (`Item.lua:81-100` / `Item.lua:138-158`) và PC.
  - Hỗ trợ đầy đủ 20 thuộc tính (ID, count, damage, element, gem, enhance, grow, OADate deleteTime, lock, affixes...).
- **`PlayerInfoCodec`** ([`src/protocol/codecs/player_info.rs`](src/protocol/codecs/player_info.rs)):
  - Đóng gói `PlayerCard` (15B cố định + VISCII Pascal name).
  - Đóng gói `FriendExtra` (chuẩn 20B cố định).
- **`BattleRoleSerializer`** ([`src/protocol/codecs/battle_role.rs`](src/protocol/codecs/battle_role.rs)):
  - Đóng gói/giải mã toàn diện thực thể chiến đấu (`BattleRoleData`) với 42-byte base header cho opcodes `0x0B` và `0x32`.
  - Hỗ trợ các phân nhánh hiển thị ngoại hình: Người chơi (`PLAYER`, `PLAYERS`, `DIVIDE`, `AUTOMANUAL_PLAYER`), Võ tướng theo sau (`FOLLOW_NPC`, `AUTOMANUAL_NPC`), Quái vật / NPC (`MINE_NPC`, v.v.).
- **Integration Test** ([`tests/codecs.rs`](tests/codecs.rs)):
  - 8/8 tests PASS 100%.

### B. Game Data Loader Layer (`src/data/`) — Tickets 03 & 04
- **`DatReader`** ([`src/data/reader.rs`](src/data/reader.rs)):
  - Đọc số nguyên LE kết hợp giải mã XOR (`xor1`, `xor2`, `xor4`) và trừ `number_offset`.
  - Tối ưu hóa đọc số nguyên trực tiếp (zero-copy `from_le_bytes`) và phương thức `skip(size)` bỏ qua cấp phát bộ nhớ.
  - Đọc chuỗi PC (1B length + reverse byte array + giải mã VISCII / Big5 / UTF-8).
  - Đọc chuỗi Unicode (2B LE length + UTF-16LE).
  - Thuật toán `decode_all(record_size, count)` giải mã khối hai khóa cho `Astrolabe.Dat`, `EVOStatus.Dat`, `CityEx.Dat`, `Warp.Dat`.
- **Bộ 9 Loaders Nhị Phân** ([`src/data/loaders/`](src/data/loaders/)):
  - `ItemDatLoader` (`item.rs`): Nạp 8,371 bản ghi `Item.dat` (370 bytes/bản ghi) với 54 trường dữ liệu.
  - `NpcDatLoader` (`npc.rs`): Nạp 6,659 bản ghi `Npc.dat` (92 bytes/bản ghi) với đầy đủ chỉ số, kỹ năng, vật phẩm rơi.
  - `FormulaDatLoader` (`formula.rs`): Nạp 28 tham số `Formula.Dat`.
  - `BlissBagDatLoader` (`bliss_bag.rs`): Nạp 307 túi may mắn `BlissBag.Dat` (190 bytes/bản ghi).
  - `CompoundDatLoader` (`compound.rs`): Nạp 1,394 công thức hợp thành `Compound.Dat`.
  - `AstrolabeDatLoader` (`astrolabe.rs`): Nạp 7 chòm sao x 10 cấp `Astrolabe.Dat`.
  - `EVOStatusDatLoader` (`evo_status.rs`): Nạp 43 trạng thái `EVOStatus.Dat`.
  - `CityExDatLoader` (`city_ex.rs`): Nạp 110 dòng hệ số thành trì `CityEx.Dat`.
  - `WarpDatLoader` (`warp.rs`): Nạp cổng warp từ `Warp.Dat` (PC) hoặc `Warp_C.dat` (Mobile).
- **`EveDataLoader` & Eve Domain Models** ([`src/data/loaders/eve.rs`](src/data/loaders/eve.rs)) — Ticket 04:
  - Đọc Scene Directory (32 bytes/entry) từ `Data/eve.emg` (9.8MB, 3,823 scene entries).
  - Đọc 9 section nhị phân theo offset `position + 103`: NpcData, GoodsData, DoorData, MineData, SurfaceData, SceneInfoData, GroupData, NpcEventData, FightData.
  - Định nghĩa 15 Condition Classes (`EveConditionClass`), 6 Condition Ops (`EveConditionOps`), 7 Result Types (`EveResultType`), 6 Result Classes (`EveResultClass`).
  - Hỗ trợ đầy đủ các struct domain: `EveResult`, `EveCondition`, `NpcEventData`, `EveNpcPlacement`, `EveDoorPlacement`, `EveSceneInfo`, `EveFightData`, `EveFightEnemy`, `EveSentence`, `EveSurfaceData`, `EveGroupData`, `SceneEveData`.
- **`GameData`** ([`src/data/loader.rs`](src/data/loader.rs)):
  - Tích hợp `pub scene_eve_data: HashMap<u32, SceneEveData>`.
  - Hàm `resolve_data_file` tự động tìm và nạp `eve.emg` / `Eve.emg` / `CompreseData/Eve.emg`.
- **Integration Test** ([`tests/data.rs`](tests/data.rs)):
  - 15/15 tests PASS 100% (bao gồm `eve_loader_loads_major_scenes` kiểm tra Tân Thủ Thôn 10801, Trác Quận 12001).

---

## 4. Kế Hoạch Cho Agent Tiếp Theo

Ticket đang ở trạng thái **UNBLOCKED / READY TO IMPLEMENT**:

### Triển khai Ticket 09 — `09-documentation-and-domain-updates.md`
- **Mục tiêu**: Cập nhật tài liệu miền (CONTEXT.md, docs/adr/) và các mô tả kiến trúc sau khi toàn bộ refactor kỹ thuật hoàn tất.

### Thành phẩm mới sau Ticket 08 (Test Migration & Cleanup)
- **Không còn unit test inline**: toàn bộ `#[cfg(test)]` đã rời `src/` (0 khối còn lại); lib test binary giờ chạy 0 tests — mọi test nằm trong `tests/` và truy cập public API qua `ts_dream::...`.
- **6 tệp test chuyên trách mới/mở rộng**:
  - `tests/protocol_test.rs` (60) · `tests/battle_engine_test.rs` (80) · `tests/data_loader_test.rs` (16) · `tests/db_persist_test.rs` (7) · `tests/server_state_test.rs` (46) · `tests/handlers_test.rs` (140, gồm 122 chuyển từ handlers/** + 18 cũ ticket 07).
- **Chống flaky global-state**: `map_drops::clear_map(map_id)` mới (reset scoped thay cho `clear_all` trong test); các test dùng registry chia nhau id dải riêng (77xx map drops, player id ≥ 300002 / 930001+, map 61_001); `tests/server_state_test.rs` có `REGISTRY_LOCK` (tokio Mutex) cho nhóm test registry.
- **C# provenance sạch 100%**: comment/doc/assert message không còn `C#`, `.cs`, `smethod_NN`, `.kt`; `grep -rn 'C#' src tests` → rỗng.
- **Clippy --all-targets = 0 warning** (đã sửa ~42 warning cũ; các allow `too_many_arguments` còn lại đều có lý do wire-layout kèm chú thích).
- **Lưu ý môi trường**: `Data/Skills.txt` không có sẵn trong repo checkout này (chỉ có `Skill.Dat`) — `tests/data.rs::loads_real_data_with_expected_counts` skip phần assert skills khi thiếu file (lỗi môi trường có từ trước ticket 08).
- **Xác thực cuối ticket 08**: `cargo test --all-targets --no-fail-fast` → 16 target, 431 tests, 100% pass, 0 warning; golden diffing pass.

### Thành phẩm mới sau Ticket 07 (Two-Tier Dispatcher & Domain Systems)
- **Level-1 dispatcher thuần opcode**: nhánh sub inline của op `0x17` đã dồn xuống `handlers/inventory.rs` (player-shop 30..=33, storage 51|52, base ops) — dispatcher chỉ match Main Opcode.
- **`ResponseSender`** (`src/server/response.rs`): bọc `Conn + HandleOutcome`; phương thức chuẩn hóa `send_bag_items`, `send_equipment_items`, `send_storage_items`, `send_dialog_talk`, `end_talk`, `send_stat_update`, `send_hp_sp_updates`, `broadcast`. Handler mới nên dùng lớp này thay vì tự ghép hex.
- **`PlayerStateManager`** (`src/server/player_state.rs`): facade thread-safe trên registry `online_sessions()` — `update/snapshot/recompute_stats/adjust_hp/adjust_sp`, trả `None` khi player offline.
- **`TradeSystem`** (`src/server/trade_system.rs`): engine settle thuần (item + pet, chống duplicate qua probe-first) tách khỏi wire handler; coordinator `begin/cancel/accept` khóa 2 người theo id ổn định qua `lock_player_operations`. `handlers/trade_storage.rs` giờ chỉ còn wire-format + persist.
- **`AutoSaveService`** (`src/server/auto_save.rs`): background Tokio task mỗi 3 phút (đã wire vào `main.rs`); dirty detection bằng fingerprint FNV-1a toàn bộ trạng thái volatile (gold/hp/stats/items/pets/quest steps), mỗi cycle 1 transaction InnoDB cho mỗi session dirty; `forget(player_id)` cho disconnect path.
- **`handlers/npc_event.rs`**: bridge Eve Engine (ticket 05) — snapshot `Session` → `PlayerEventState`, resolve NPC/door event, auto-chain continuation với guard depth + map-consistency. Mặc định TẮT (`TS_EVE_EVENTS=1` để bật) nhằm giữ golden parity; mission flags/mark_defs chưa nạp (ticket 08).
- Lưu ý: 42 clippy warnings còn lại là mã cũ (is_multiple_of, redundant pattern...) — thuộc phạm vi ticket 08.

### Thành phẩm mới sau Ticket 06 (`migrations/0002_modern_schema.sql`, `src/db/modern/`)
- **Migration 3NF**: `characters` (accounts 1:1 — PC chỉ hỗ trợ 1 nhân vật/tài khoản, UNIQUE key trên `account_id`), `character_money`, `inventories` (hợp nhất các bảng túi đồ còn dùng: homdo/trangbi; composite PK `(character_id, storage_type, slot)`; storage_type 1=Bag, 2=Secondary, 4=Bank, 8=Equip, 16=Warehouse; đủ 20 cột ThingData), `character_pets` (4 kho: 1=Tùy thân, 2=Mã xa, 3=Khách sạn, 4=Kho), `character_skills`, `character_hotkeys`, `character_missions`, `character_mission_flags`, `character_bit_flags`, `character_completed_events`, `friends`, `mails`. Toàn bộ text giữ `latin1_bin`; không FK (parity legacy).
- **Dọn dẹp 0001**: audit usage — xóa `luulang`, `tientrang`, `tuideo` (không có caller nào); giữ `accounts`, `players`, `homdo`, `trangbi`, `pet`, `quest`, `skill`, `skillsave`, `item_code`. Session pouches tương ứng vẫn tồn tại in-memory (wire ops 0x1E01/0x172F/0x1766) nhưng không persist nữa.
- **Lưu ý checksum migration**: 0001 đã sửa nội dung → database dev đã chạy migration cũ phải tạo lại (xóa `_sqlx_migrations` hoặc drop DB) trước khi boot.
- **Models** (`model.rs`): `StorageType`, `PetStorageType`, `InventorySlot` (ThingData-backed), `Money`, `SkillRow`, `MissionRow`, `PetRecord`.
- **Traits** (`traits.rs`): `AccountRepository`, `CharacterRepository`, `InventoryRepository`, `PetRepository`, `QuestRepository` — async fn native, static dispatch.
- **MySQL impls** (`mysql/`): `MySqlRepositories` bundle; HEX-compare cho password/name VISCII byte-exact; upsert/clear slot zero toàn bộ cột.
- **Transactions** (`transactions.rs`): `bank_transfer` (guard `gold/bank_gold >= cost` ngay trong UPDATE), `shop_buy`, `p2p_trade` — mỗi op 1 transaction nguyên tử, trả `TxError::{InsufficientFunds, SourceSlotEmpty, DestinationSlotOccupied}`.
- **Tests** (`tests/db_repositories.rs`): 4 unit tests chạy mọi nơi + 8 integration tests tự skip khi thiếu MySQL; bật bằng `TS_TEST_DB_URL=mysql://root:...@localhost:3306/ts_dream_test cargo test --test db_repositories`. Lưu ý: migration SQL chưa từng chạy trên MySQL thật trong môi trường dev (không có server); lần boot đầu với DB live sẽ xác nhận.

### Thành phẩm mới sau Ticket 05 (`src/eve/`)
- `state.rs`: `PlayerEventState` snapshot + `EveStateBuilder` (build_player_state / find_fallback_talk / is_within_range).
- `evaluator.rs`: `compare_step` (7 ops) + `evaluate` (15 condition classes; class=1 đảo ngược, class=7 param=0/2 đảo ngược).
- `resolver.rs`: `build_chains` (andNum), `resolve` (4 tầng ưu tiên + chainFilter), `resolve_event` (chain + GroupData).
- `group.rs`: `apply_group_data` weighted random qua `DotNetRandom`.
- `auto_chain.rs`: `EventSession`/`EventPhase`, `EveAutoChainEngine` (4 guards + should_attempt + should_skip + try_auto_chain thuần dữ liệu; caller sở hữu việc activate session — ticket 07 sẽ wire vào dispatcher).
- Lưu ý: `src/eve/mod.rs` giờ là module root thật (`pub mod eve;` trong lib.rs); các module khác trong lib.rs vẫn khai báo inline theo quy ước cũ.

---

## 5. Suggested Skills

- **`cuder` / `implement`**: Dùng khi bắt đầu triển khai code cho Ticket 02 hoặc Ticket 04.
- **`code-reviewer`**: Dùng để rà soát cấu trúc binary struct và đảm bảo tính tương thích byte-level với client TS PC.

---

## 6. References & Artifacts

- **Bản đồ tổng thể**: [`.scratch/refactor-ts-server/map.md`](.scratch/refactor-ts-server/map.md)
- **Đặc tả kỹ thuật kiến trúc**: [`.scratch/refactor-ts-server/spec.md`](.scratch/refactor-ts-server/spec.md)
- **Danh sách Tickets**: [`.scratch/refactor-ts-server/issues/`](.scratch/refactor-ts-server/issues/)
- **Quy tắc dự án**: [`AGENTS.md`](AGENTS.md)
- **Từ điển Ubiquitous Language**: [`CONTEXT.md`](CONTEXT.md)
