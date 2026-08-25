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
| [`06`](.scratch/refactor-ts-server/issues/06-mysql-schema-migration-and-repositories.md) | **MySQL Schema 3NF & Repositories** | **READY (Unblocked)** | Unblocked by 02 completion |
| [`07`](.scratch/refactor-ts-server/issues/07-two-tier-dispatcher-and-modular-handlers.md) | **Two-Tier Dispatcher & Handlers** | **QUEUED** | Blocked by 01, 02, 05, 06 |
| [`08`](.scratch/refactor-ts-server/issues/08-test-migration-and-csharp-comment-cleanup.md) | **Test Migration & C# Cleanup** | **QUEUED** | Blocked by 07 |
| [`09`](.scratch/refactor-ts-server/issues/09-documentation-and-domain-updates.md) | **Documentation & Domain Updates** | **QUEUED** | Blocked by 08 |

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

### Triển khai Ticket 06 — `06-mysql-schema-migration-and-repositories.md`
- **Mục tiêu**: Thiết kế schema MySQL 3NF và repository layer tích hợp cấu trúc 35-byte `ThingData`:
  - Migration SQL: bảng `inventories` (character_id, storage_type, slot, item_id, ... 20 cột ThingData), `characters`, `character_pets` (4 kho tướng).
  - Repositories: `CharacterRepository`, `InventoryRepository`, `PetRepository`.

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
