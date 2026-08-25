# Spec: Tái Cấu Trúc TS Server (Rust) Theo Kiến Trúc Kotlin TS Mobile Server

Status: ready-for-agent

## Problem Statement

Mã nguồn TCP Game Server TS Online (`ts_dream`) hiện tại được chuyển đổi trực tiếp từ dự án mã nguồn đóng C# cũ, dẫn đến nhiều hạn chế kỹ thuật:
1. **Tầng Giao thức & Payload phân mảnh**: Các handler xử lý gói tin bằng cách ghép và phân tích chuỗi Hex (`format!("{:02X}")`), gây tốn tài nguyên cấp phát bộ nhớ trung gian (`String`), thiếu các abstraction nhị phân Little-Endian chuẩn hóa và thiếu các serializer chuyên biệt cho các thực thể miền (Vật phẩm, Nhân vật, Trận đấu).
2. **Tầng Dữ liệu Tĩnh chưa tương thích định dạng nhị phân gốc**: Server đang nạp các tệp văn bản `.txt`/`.ini` tự sinh, không thể đọc trực tiếp các tệp dữ liệu nhị phân chuẩn của game (`Item.dat`, `Npc.dat`, `Warp.Dat`, `Formula.Dat`, `BlissBag.Dat`, và container kịch bản `eve.emg` chứa hơn 3,800 bản đồ nhiệm vụ).
3. **Cơ sở Dữ liệu phi chuẩn hóa**: Schema MySQL cũ tồn tại 5 bảng túi đồ trùng lặp (`homdo`, `trangbi`, `tientrang`, `tuideo`, `luulang`), gom chung tài khoản và nhân vật 1:1, thiếu các trường thuộc tính mở rộng của vật phẩm (Linh vũ khí, Chuyên vũ, Ngọc khảm, Tẩy luyện dòng, Thời trang), thiếu hệ thống quản lý 4 kho võ tướng và hệ thống cờ trạng thái nhiệm vụ vĩnh viễn (BitFlags).
4. **Gamelogic chưa phân tầng**: Bộ điều phối (Dispatcher) dồn ép hơn 70 trường hợp match trong một tệp lớn, logic đóng gói gói tin bị trộn lẫn với logic nghiệp vụ, thiếu các Domain System chuyên trách.
5. **Codebase thiếu tinh gọn**: Hơn 48 module trong `src/` chứa khối kiểm thử inline `#[cfg(test)]`, cùng nhiều chú thích tham chiếu mã nguồn C# legacy gây rối mắt và khó bảo trì.

## Solution

Tái cấu trúc toàn diện Game Server TS Dream trên nền tảng Rust (Tokio async runtime) dựa trên kiến trúc module hóa và thiết kế sạch (Clean Architecture) của Kotlin TS Mobile Server (`ts_mobile_server`), đồng thời bảo toàn 100% các ràng buộc giao thức và nghiệp vụ của TS Online PC:
1. **Tầng Giao thức Nhị phân Tối ưu (Binary Protocol & Codecs)**: Xây dựng bộ công cụ đọc/ghi nhị phân Zero-Copy (`PacketReader`, `PacketWriter`) hỗ trợ Little-Endian và bảng mã VISCII 1.1; phân tách rành mạch tầng Frame Codec (Header `F4 44`, XOR `0xAD`) khỏi các Serializer miền (`ThingDataCodec` 35-byte, `PlayerInfoCodec`, `BattleRoleSerializer`).
2. **Tầng Nạp Dữ liệu Nhị phân (.Dat & Eve.emg Engine)**: Hiện thực `DatReader` giải mã bảng khóa XOR/offset và bộ parser nhị phân cho toàn bộ 9 tệp `.Dat`; chuyển đổi toàn diện sang bộ đọc `EveDataLoader` và động cơ kịch bản sự kiện 4 tầng (`Eve Engine`) có cơ chế chống lặp vô hạn 4 lớp.
3. **Chuẩn hóa Cơ sở Dữ liệu MySQL 8 (3NF Schema & Repository Layer)**: Redesign quan hệ Account 1:N Characters, hợp nhất 5 bảng túi đồ thành 1 bảng `inventories` chuẩn `ThingData` 20 cột, quản lý 4 kho võ tướng `character_pets` (Tùy thân, Mã xa, Khách sạn, Kho), mở rộng hệ thống Nhiệm vụ, Cờ vĩnh viễn (BitFlags), Điểm bay, Thư từ, Bạn bè; duy trì bảng mã `latin1_bin` cho chuỗi ký tự thô.
4. **Phân rã Hệ thống Điều phối & Domain Systems**: Thiết lập mô hình điều phối 2 cấp (Two-tier Dispatcher), bóc tách tầng `ResponseSender`, xây dựng các Domain Systems chuyên trách (`PlayerStateManager`, `QuestSystem`, `PetSystem`, `TradeSystem`, `AutoSaveService`).
5. **Di chuyển Test & Làm Sạch Mã Nguồn**: Chuyển toàn bộ kiểm thử inline từ `src/` sang các tệp test chuyên trách trong `tests/`, xóa sạch toàn bộ chú thích C# cũ, bảo đảm toàn bộ test suite và 18 golden test capture diffing vượt qua 100%.

---

## User Stories

### Nhóm 1: Tầng Giao Thức Mạng & Đóng Gói Payload
1. As a Game Client, I want to establish a TCP connection on port 6414 and communicate using `F4 44` framing with XOR `0xAD` encryption, so that I can interact with the server using standard TS PC network protocols.
2. As a Server Developer, I want a zero-copy `PacketReader` with bounds checking for Little-Endian primitives (u8, u16, u32, i8, i16, i32, f64, boolean, VISCII strings), so that binary payloads are parsed safely without string allocations.
3. As a Server Developer, I want a fluent `PacketWriter` buffer builder, so that domain packets and outgoing frames can be composed efficiently with zero hex-string formatting.
4. As an Inventory System, I want a standardized 35-byte `ThingDataCodec` representing all 20 item attributes (spirit levels, socket gems, exclusive weapons, affixes, durability, delete times), so that items are serialized consistently across bags, trade, and storage.
5. As a Social / Friend System, I want a `PlayerInfoCodec` to serialize character cards and online friend snapshots, so that client UI displays friend status and attributes accurately.
6. As a Battle Engine, I want a `BattleRoleSerializer` to serialize combatant appearances (players, follow pets, summons, enemies), so that battle participants render correctly at combat initialization.

### Nhóm 2: Nạp Dữ Liệu Tĩnh & Động Cơ Sự Kiện (Eve Engine)
7. As a Game Server Administrator, I want the server to load binary `Item.dat` (3.09 MB) on startup, so that full item specifications (attributes, requirements, colors, prices, furnace values) are available in memory.
8. As a Combat Engine, I want the server to parse `Formula.Dat` and `Npc.dat` on startup, so that base stats, skills, capture rates, and mathematical damage multipliers match original game rules.
9. As a Game World, I want the server to load `Warp.Dat`, `BlissBag.Dat`, `Compound.Dat`, `Astrolabe.Dat`, `CityEx.Dat`, and `EVOStatus.Dat`, so that world teleports, lucky boxes, crafting, and evolution operate natively.
10. As a Quest & Event Engine, I want the server to parse `eve.emg` (container of 3,800+ scenes) into immutable structured data, so that all NPC dialogue, menus, door warps, and scripted PvE fights are loaded without manual conversion.
11. As a Player interacting with an NPC, I want the `EveConditionEvaluator` to evaluate 15 condition classes (bag items, quest steps, level, reborn status, dialogue choices, role counts, completed event counts), so that the correct dialogue and quest branches are activated.
12. As a Quest Engine, I want the `EveChainResolver` to sort competing event chains using 4-tier prioritization (highest quest step $\to$ longest condition chain $\to$ result count $\to$ declaration order), so that the most relevant quest dialogue takes precedence over fallback text.
13. As a Player progressing through a multi-step quest, I want the `EveAutoChainEngine` with 4-layer loop protection (same condition, re-question, re-battle, duplicate item) to advance quest chains automatically, so that multi-stage dialogue flows smoothly without freezing or infinite looping.

### Nhóm 3: Cơ Sở Dữ Liệu & Hệ Thống Lưu Trữ (Persistence)
14. As an Account Administrator, I want a 1:N relationship between `accounts` and `characters`, so that a single user account can own multiple game avatars.
15. As a Player, I want my character name and raw strings to be preserved in `latin1_bin` collation, so that Vietnamese VISCII characters never suffer from encoding corruption or byte distortion.
16. As a Player, I want my items in bag, equipment, secondary bag, bank, and temporary warehouse to be stored in a unified `inventories` table with composite key `(character_id, storage_type, slot)`, so that inventory operations are ACID-compliant.
17. As a Pet Owner, I want my companions to be organized across 4 storage tiers (`character_pets`: Follow 1..4, Cart 1..4, Inn 1..30, Warehouse 1..150) with complete stats, skills, pills, and rebirth counts, so that no pet data is lost when transferring between storage.
18. As a Player completing quests and achievements, I want my quest steps, bit flags, forever flags (`character_bit_flags`), role counts, and completed event counts saved reliably in MySQL, so that my world progression is persistent.
19. As a Game Server, I want an `AutoSaveService` running periodically in the background to batch-save dirty session states to MySQL every 3 minutes, so that database I/O is minimized while preventing data loss.
20. As a Database Developer, I want repository operations exposed as async Rust traits (`AccountRepository`, `CharacterRepository`, `InventoryRepository`, `PetRepository`, `QuestRepository`), so that database access is decoupled and unit-testable.

### Nhóm 4: Điều Phối Gói Tin & Hệ Thống Nghiệp Vụ
21. As a Game Server, I want a Two-tier Dispatcher where Level 1 routes by main Opcode and Level 2 modular handlers process subcodes, so that opcode handling logic is clean and decoupled.
22. As a Handler Developer, I want a `ResponseSender` abstraction to construct and dispatch typed response packets, so that handler methods focus purely on business logic rather than packet formatting.
23. As a Combatant, I want a `PlayerStateManager` to manage dynamic HP, SP, base attributes, and equipment bonus stats in-memory with thread-safe access, so that combat computations are instant and accurate.
24. As a Trader, I want a 2-phase atomic `TradeSystem` between two players, so that item and gold exchanges succeed completely or roll back safely upon cancellation or disconnect.
25. As a Party Member, I want a `PartySystem` managing up to 5 players with military advisor (quân sư) SP regeneration, leader delegation, and team movement sync, so that group gameplay operates reliably.

### Nhóm 5: Kiểm Thử & Tinh Gọn Mã Nguồn
26. As a Core Developer, I want all unit and integration tests migrated from `src/` to categorized files in `tests/`, so that the production source code is clean, readable, and free of inline test bloat.
27. As a Maintainer, I want all legacy comments referencing C# removed from the entire codebase, so that the code speaks purely in terms of domain ubiquitous language.
28. As a QA Engineer, I want all existing test suites (including 18 golden packet capture diffing fixtures) to pass with 100% success on `cargo test`, so that no regressions occur during refactoring.

---

## Implementation Decisions

### 1. Protocol Layer Redesign
- **Frame Demarcation vs. Payload Serialization**: Tách biệt rõ ràng tầng mạng (Network Framing) và tầng nghiệp vụ (Payload Serialization).
  - Tầng Framing: Đóng gói và giải mã Header `F4 44`, độ dài 2-byte Little-Endian, và mã hóa XOR toàn gói `0xAD`.
  - Tầng Payload: `PacketReader` (tham chiếu mảng byte không cấp phát thêm) và `PacketWriter` (bộ đệm mở rộng tự động).
- **Loại bỏ Hoàn toàn Định dạng Hex String**: Thay thế toàn bộ các thao tác `format!("{:02X}")` và chuỗi hex trung gian bằng các kiểu nhị phân nguyên bản (`u8, u16, u32, i8, i16, i32, f64`).
- **Domain Serializers**:
  - `ThingDataCodec`: Cấu trúc nhị phân 35 bytes đại diện cho vật phẩm chuẩn hóa.
  - `PlayerInfoCodec`: Đóng gói thẻ nhân vật và snapshot bạn bè.
  - `BattleRoleSerializer`: Đóng gói thực thể chiến đấu cho các opcode `0x0B` và `0x32`.

### 2. Game Data Loaders & Eve Event Engine
- **Bộ Đọc Nhị phân `DatReader`**: Hỗ trợ Little-Endian, các khóa giải mã XOR (`xor1, xor2, xor4`), độ lệch giá trị (`number_offset`), và cơ chế đọc chuỗi PC (đọc ngược mảng byte kèm độ dài 1 byte).
- **Bộ Nạp Bảng Dữ liệu Tĩnh**:
  - `ItemDatLoader`: Nạp `Item.dat` (3.09 MB) vào bộ nhớ với đầy đủ 54 trường dữ liệu (`ItemDef`).
  - `NpcDatLoader`: Nạp `Npc.dat` (612 KB) vào bộ nhớ (`NpcDef`).
  - `WarpDatLoader`, `FormulaDatLoader`, `BlissBagDatLoader`, `CompoundDatLoader`, `AstrolabeDatLoader`, `CityExDatLoader`, `EVOStatusDatLoader`.
- **Động cơ Sự kiện `Eve.emg`**:
  - `EveDataLoader`: Parser giải mã container 3,800+ scene script và 9 section nhị phân.
  - Chuyển đổi 100% sang hệ thống `Eve.emg` chuẩn Kotlin, thay thế toàn bộ hệ thống quest `.txt`/`.ini` cũ.
  - `EveConditionEvaluator`: Đánh giá 15 lớp điều kiện (`ConditionClass`).
  - `EveChainResolver`: Giải thuật ghép chuỗi AND và sắp xếp 4 tầng ưu tiên (`stepScore` $\to$ độ dài chuỗi $\to$ số kết quả $\to$ thứ tự xuất hiện).
  - `EveAutoChainEngine`: Tự động nối tiếp sự kiện với 4 lớp chống lặp (Same Condition, Re-Question, Re-Battle, Duplicate Item).

### 3. Database Schema & Persistence Layer
- **Chuẩn Hóa Quan Hệ (3NF)**:
  - `accounts` (1) $\to$ (N) `characters`.
  - Hợp nhất 5 bảng túi đồ cũ thành 1 bảng duy nhất `inventories (character_id, storage_type, slot)` với `storage_type` (1=Bag, 2=Secondary, 4=Bank, 8=Equip, 16=Warehouse) lưu trữ đầy đủ 20 trường thuộc tính `ThingData`.
  - `character_pets`: Quản lý 4 kho võ tướng theo `storage_type` (1=Tùy thân, 2=Mã xa, 3=Khách sạn, 4=Kho).
  - Bảng nhiệm vụ và cờ: `character_missions`, `character_mission_flags`, `character_bit_flags` (Forever Flags), `character_completed_events`.
  - Bảng mở rộng sẵn sàng: `character_mounts`, `character_mount_equips`, `character_fly_points`, `character_role_counts`, `friends`, `friend_invites`, `mails`, `mail_attachments`, `character_dispatches`.
- **Bảo Toàn Bảng Mã**: Tiếp tục sử dụng `VARCHAR(...) CHARACTER SET latin1 COLLATE latin1_bin` cho tên nhân vật và văn bản thô để đảm bảo tương thích 100% với VISCII 1.1.
- **Lớp Repository & Tự Động Lưu Trữ**:
  - Tách biệt Domain Models khỏi Database Entities thông qua các Rust Repository Traits.
  - `AutoSaveService`: Tokio background interval task chạy định kỳ mỗi 3 phút, quét dirty state và ghi MySQL an toàn.

### 4. Gamelogic Systems & Two-tier Dispatcher
- **Level 1 Dispatcher**: Tiếp nhận gói tin đã giải mã và phân phối theo Opcode chính (MainKind).
- **Level 2 Handlers (36 Modules Chuyên Trách)**: Mỗi module chịu trách nhiệm 1 Opcode chính, phân nhánh theo Subcode.
- **ResponseSender Abstraction**: Tách toàn bộ việc đóng gói byte packet ra khỏi nghiệp vụ của handler.
- **Domain Systems Độc Lập**: `PlayerStateManager`, `QuestSystem`, `PetSystem`, `TradeSystem`, `AutoSaveService`.

### 5. Di Chuyển Kiểm Thử & Làm Sạch Mã Nguồn
- **Di Chuyển Toàn Bộ Tests Sang `tests/`**: Phân chia thành các tệp kiểm thử tích hợp chuyên biệt:
  - `tests/protocol_test.rs`
  - `tests/handlers_test.rs`
  - `tests/battle_engine_test.rs`
  - `tests/data_loader_test.rs`
  - `tests/db_persist_test.rs`
  - `tests/server_state_test.rs`
- **Xóa Bỏ Toàn Bộ Chú Thích C#**: Rà soát và thay thế toàn bộ comment có nhắc tới C# bằng thuật ngữ miền Ubiquitous Language chuẩn.

---

## Testing Decisions

### Tiêu Chuẩn Kiểm Thử Chất Lượng (Quality Seams)
- **Kiểm Thử Hành Vi Bên Ngoài (Black-box / Golden Diffing)**: Ưu tiên kiểm thử tại đường ranh giới cao nhất (Highest Seam) — gửi chuỗi byte nhị phân mô phỏng client vào server và đối sánh chính xác từng byte phản hồi với các mẫu chuẩn (`golden/01` $\to$ `golden/18`).
- **Kiểm Thử Nạp Dữ Liệu Thực Tế**: Tải trực tiếp các tệp `.Dat` và `eve.emg` thực tế từ thư mục `Data/`, kiểm tra số lượng bản ghi nạp được và tính toàn vẹn của các trường dữ liệu quan trọng.
- **Kiểm Thử Đơn Vị Hệ Thống Nghiệp Vụ**: Kiểm thử độc lập các thuật toán tính toán sát thương (`damage.rs`), công thức chiến đấu (`formulas.rs`), sắp xếp thứ tự chuỗi sự kiện (`EveChainResolver`), và máy trạng thái giao dịch (`TradeSystem`).
- **Vị Trí Kiểm Thử Tập Trung**: Toàn bộ mã kiểm thử phải nằm trong thư mục `tests/`, không để lại bất kỳ khối `#[cfg(test)]` nào rải rác trong `src/`.

---

## Out of Scope

- Thay đổi định dạng Header gói tin sang `0xC0 0x91` của client TS Mobile (PC Client bắt buộc dùng `F4 44`).
- Thay thế runtime Tokio hoặc chuyển đổi mô hình 1 client connection = 1 Tokio task.
- Chỉnh sửa giao diện hoặc luồng xử lý của Web Admin Dashboard (Axum + Askama + HTMX tại cổng 8090).
- Thay đổi cấu hình cổng TCP Game Server 6414.

---

## Further Notes

- Quá trình chuyển đổi từ nạp dữ liệu `.txt`/`.ini` sang nạp nhị phân `.Dat` và `eve.emg` sẽ tự động khắc phục lỗi thiếu tệp `Npcs.txt` đang gặp phải trong bài kiểm thử `tests/data.rs`.
- Sau khi bản đặc tả này được chấp thuận, bước tiếp theo sẽ là khởi chạy `/to-tickets` để phân rã spec thành các tracer-bullet implementation issues chi tiết và tiến hành code theo chu trình TDD.
