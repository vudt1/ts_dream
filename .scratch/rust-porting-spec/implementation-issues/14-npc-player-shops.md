# 14 — NPC shops (op 0x1B) + player shop (op 0x17 sub 30–33)

**What to build:** Người chơi mua/bán vật phẩm với NPC shop (bảng giá hardcoded `(map,menu)→(item,price)`) và mở quầy hàng cá nhân mua/bán được với người khác. Vertical slice kinh tế nguyên bản.

**Blocked by:** 11 — Inventory base (thao tác slot/gold); 12 — Use item (gold chênh / game economy). Chỉ kích hoạt được khi mở menu qua talk (ticket 18) — gắn dispatch vào chỗ đó.

**Status:** completed (verified 2026-08-08)

- [x] Op 0x17 sub 0x1B (NPC shop) — buy: check gold and complete inventory preflight before `HomdoAddItem`; persist Gold + Homdo atomically; send `F4440A001A04`+gold+`00000000` and red message after success (Rust `src/server/handlers/shops.rs:204-245`, persistence `src/db/persist.rs:184-302`).
- [x] Sell: with `idnpctalking ∈ {16005, 99999}` scan inv `26001..26455` (or `27001..27165` for `16002/99999`); credit `data[7]` count and atomically persist Gold + Homdo (Rust `shops.rs:182-207`, C# `Client.cs:7095-7122`).
- [x] Bảng `(map, menu) → (itemId, price)` gồm đủ 89 buy branches C# tại `Client.cs:6472-7094`; Rust table `shops.rs:50-159`, exhaustive shelf/count test `shops.rs` test `npc_price_table_has_every_csharp_branch`.
- [x] Player shop (op 0x17 sub 30/31/32/33): strict listing validation, online active seller checks, self-buy rejection, atomic buyer/seller Gold + Homdo persistence, and map-scoped `171F/1720` broadcasts (Rust `shops.rs:393-571`, `src/server/inventory.rs:70-91`, `src/db/persist.rs:184-302`; C# `Client.cs:5399-5545`).
- [x] Golden: buy-from-mall/mall-buy scenario được ghi lại (Ch9 §9.6).

## Audit đối chiếu Rust ↔ spec ↔ C#

Kết luận: **đã đáp ứng ticket** theo các quyết định grilling: parity có bảo vệ, transaction economy atomic, strict validation, online-only player shop và map-scoped broadcast.

### Provenance implementation

The findings below record the pre-fix audit; the implementation status and verification at the end of this ticket are authoritative for the current Rust tree.

| Tiêu chí | Rust hiện tại | C# ground truth / spec | Đánh giá |
|---|---|---|---|
| Dispatcher | `src/server/dispatcher.rs:174-187` route op `0x17` sub `30..33` và op `0x1B` | `ts_server_old/Server_TS_Online/Client.cs:859-957`, `Client.cs:2101-2106` | Đạt route cơ bản |
| NPC price table | `src/server/handlers/shops.rs:53-164`, hiện chỉ là tập mapping hardcoded hữu hạn | `Client.cs:6428-6471`, các nhánh buy `Client.cs:6472-7094`; contract tại `research/01-protocol-reference.md:324-326` | **Chưa đạt exhaustive**; cần chứng minh/port toàn bộ mapping C# |
| NPC buy | `shops.rs:204-213` | `Client.cs:6472-7094` | Gate gold và packet có; nhưng gọi `add_homdo_item` rồi bỏ qua slots affected/empty trước khi trừ gold, nên bag-full có thể mất tính nhất quán |
| NPC sell | `shops.rs:24-51`, `182-191` | `Client.cs:7095-7122` | Range và count có; Rust chỉ persist Gold, chưa persist các Homdo row đã giảm/xóa |
| Player shop open | `shops.rs:345-412` | `Client.cs:5399-5437`; spec research `01-protocol-reference.md:303` | Có frame `171E/171F`, nhưng broadcast dùng `broadcast_except` global tại `shops.rs:400-410`, trong khi C# dùng map scope |
| Player shop close | `shops.rs:413-423` | `Client.cs:5438-5448` | Có `1720`, nhưng broadcast global thay vì cùng map |
| Open catalog | `shops.rs:217-243`, `424-439` | `Client.cs:5449-5464`, `OpenPlayerShop` tại `Client.cs:10147-10159`; spec `research/01-protocol-reference.md:305` | Có `1721`; cần strict target/active validation và authoritative live-state guarantee |
| Player shop buy | `shops.rs:264-343`, `440-492` | `Client.cs:5466-5545`; spec `research/01-protocol-reference.md:306` | Có transfer và protected gold check; nhưng chỉ gate free slot cho equipment, không gate add-item thành công cho consumable; không persist inventory mutations; không chống self-buy/stale listing/index race |
| Online lifecycle | `src/server/session.rs:436-443`, `src/web/server_control.rs:341-354,387-390` | `Client.cs:5468-5479`, `Server.Clients` | Online-only snapshot có; cần đảm bảo shop state/catalog không bị stale giữa concurrent buyer và seller connection |
| Tests | `shops.rs:513-924`, `tests/common/mod.rs:204-206`, `golden/15-npc-shop-buy.golden` | Capture parity yêu cầu tại spec Ch9 | Unit tests pass, nhưng chưa có exhaustive price-table diff, DB transaction test, map-scope test, malformed-input/self-buy/catalog-stale tests, hoặc full player-shop golden |

### Danh sách vấn đề và giải pháp

1. **NPC price table chưa chứng minh/đủ toàn bộ C# branches.** Rust table nằm tại `shops.rs:57-160`, trong khi C# source of truth là `Client.cs:6472-7094` với nhiều nhánh `else if`. Chỉ test một số mẫu tại `shops.rs:753-775` không chứng minh exhaustive parity.
   - Giải pháp: tạo bảng/const Rust từ toàn bộ `(idtalking, map, menu) -> (item, price)` của `Client.cs:6472-7094`; thêm test enumerate mọi row và test unknown row silent. Không đọc `shopp.accdb`: research xác nhận server C# không load nó (`research/02-data-file-formats.md:639-640,701-702`).

2. **NPC buy có thể trừ vàng dù inventory không nhận item.** `shops.rs:208-212` bỏ qua `Vec<u8>` slots trả về bởi `Session::add_homdo_item` (`session.rs:418-422`; rule `inventory.rs:30-67`). C# cũng không xử lý đẹp bag-full, nhưng mục tiêu đã chốt là parity có bảo vệ và không làm mất economy.
   - Giải pháp: tính khả năng add trước, hoặc thực hiện mutation in-memory rồi rollback nếu không có affected slot; chỉ persist gold sau khi item add thành công; phát packet success sau commit.

3. **NPC sell không persist inventory.** `shops.rs:43-46` giảm Homdo nhưng `shops.rs:189-191` chỉ gọi `update_player(..., Gold)`. C# giảm item tại `Client.cs:7101/7115` và cập nhật gold tại `Client.cs:7102/7116`.
   - Giải pháp: transaction ghi gold và upsert/delete toàn bộ Homdo slots bị ảnh hưởng. Không dùng các helper best-effort hiện tại (`db/persist.rs:46-60,112-182`) cho giao dịch economy nếu chúng có thể thành công một phần.

4. **Player-shop buy không persist item mutations.** `shops.rs:459-480` chỉ persist Gold của buyer/seller; các thay đổi `seller.homdo` và `buyer.homdo` từ `complete_shop_buy` (`shops.rs:308-335`) chỉ tồn tại trong snapshot registry/connection.
   - Giải pháp: một MySQL transaction atomic cho buyer và seller: lock/read authoritative rows, kiểm tra listing/stock/gold/free slot, update/delete Homdo rows hai bên và update players.Gold; chỉ phát packet khi commit thành công. Schema shared bắt buộc mọi row có `player_id` theo `docs/rust_porting_spec.md:18` và Chapter 5 §5.4.

5. **Free-slot gate lệch phạm vi.** `complete_shop_buy` chỉ kiểm tra free slot khi `loai 1..=6` (`shops.rs:303-305`), nhưng sau đó consumable vẫn có thể `add_item` fail silently (`shops.rs:323-325`). C# kiểm tra `Data.HomdoGetSlotNothing(conn)` trước giao dịch cho mọi item (`Client.cs:5480-5484`).
   - Giải pháp: dùng một hàm preflight add/stack cho mọi item; nếu không thể chứa đủ count thì reject trước mọi mutation.

6. **Strict input chưa đủ.** Sub 30 parser (`shops.rs:359-390`) nhận tên UTF-8-lossy, không kiểm tra slot tồn tại, duplicate slot, item identity/count/price, listing đang bán, hoặc payload trailing hợp lệ. Sub 33 (`shops.rs:440-448`) không chống buyer mua chính mình, index stale sau catalog thay đổi, và lỗi index được gom chung.
   - Giải pháp: validate name/byte boundary theo wire contract, slot thuộc Homdo và item còn đúng slot, price hợp lệ, không duplicate, shop active; reject self-buy; resolve listing atomically trong seller state/DB; malformed frame phải silent hoặc red-message đúng nhánh, tuyệt đối không panic.

7. **Broadcast player shop không map-scoped.** `shops.rs:400-410` và `413-423` gọi `ServerControl::broadcast_except`, implementation global tại `src/web/server_control.rs:217-228`. C# sử dụng `SendToAllClientMapid` tại `Client.cs:5434-5435` và `Client.cs:5444-5446`; map-scoped mechanism Rust đã có tại `server_control.rs:230-269` nhưng handler chưa dùng.
   - Giải pháp: phát `171F/1720` qua map-scoped broadcast, lọc theo map snapshot của seller và loại seller khỏi fan-out; thêm test hai player khác map không nhận frame.

8. **Snapshot registry có nguy cơ stale/concurrent lost update.** Connection loop copy registry vào `conn.session` trước mỗi frame (`server_control.rs:341-348`), handler sub 33 clone seller rồi ghi lại (`shops.rs:449-463`), sau đó connection seller có thể ghi snapshot cũ ngược lại (`server_control.rs:350-354`).
   - Giải pháp: player-shop transaction phải dùng authoritative per-player state/serialized lock, cập nhật seller connection snapshot qua một đường duy nhất; tối thiểu không overwrite seller nếu generation/state đã đổi. Đây là phần deferred quan trọng, không nên coi registry clone hiện tại là atomic transaction.

9. **Wire payload sub 30 cần golden byte diff thực tế.** C# tạo `171E` từ `text11` tại `Client.cs:5415-5431`, parser Rust bắt đầu listing tại `shops.rs:370` và phát body tại `shops.rs:392-399`. Unit test `shops.rs:643-668` chỉ kiểm tra contains, chưa assert toàn bộ frame/length/listing nhiều dòng.
   - Giải pháp: thêm golden vectors cho name VISCII bytes, zero listing, nhiều listing, malformed/trailing payload; diff exact với capture C# theo Ch9.

### Notes / deferred cần xử lý

Ticket hiện không có mục `Notes / deferred`; các deferred thực tế phát hiện trong audit là DB atomicity, authoritative online state, exhaustive table proof, và map-scoped broadcasts. Chúng không nên bị deferred nếu ticket vẫn tuyên bố “vertical slice kinh tế nguyên bản”, vì đều ảnh hưởng trực tiếp đến mất item/gold hoặc client ở map khác nhận catalogue.

Nếu tách follow-up, thứ tự đề xuất là: (1) exhaustive table + generated parity test, (2) preflight/atomic inventory mutation, (3) MySQL transaction cho NPC/player shop, (4) map-scoped + authoritative live registry, (5) malformed-input and capture golden suite.

### Verification đã chạy

- `cargo test server::handlers::shops -- --nocapture`: **18 passed** sau khi bổ sung exhaustive table, validation, exact-slot và broadcast regression tests.
- `cargo check`: passed.
- `cargo test -- --test-threads=1`: **258 unit tests + all integration/golden/web tests passed**.
- Unit tests cover all 89 C# NPC buy branches, inventory preflight, exact listing-slot removal, player-shop active seller/self-buy/invalid listing paths, and `MapBroadcast` output. Live MySQL service integration remains environment-dependent; transaction SQL is exercised by compile/type checks but requires a configured MySQL instance for an end-to-end DB test.

### Hardening follow-up (2026-08-08)

- Player-shop names now remain raw VISCII bytes end-to-end instead of passing through UTF-8-lossy conversion (`src/server/session.rs`, `src/server/handlers/shops.rs`). Regression test: `player_shop_name_preserves_viscii_bytes`.
- The live connection loop now uses ordered per-player operation locks from authoritative snapshot load through handler completion and registry publish. Shop purchases lock buyer+seller, ordinary frames/disconnects lock only the affected player, preventing lost updates without blocking unrelated sessions (`src/server/session.rs`, `src/web/server_control.rs`).
- Failed DB persistence restores the buyer snapshot; seller mutations are not published unless the atomic persistence transaction commits.
- NPC sell now requires the full requested count, credits exactly `data[7]`, and emits the C# packet order (success message before gold update).
- Validation: `cargo check` passed; `cargo test server::handlers::shops -- --nocapture` passed 20/20; `cargo test -- --test-threads=1` passed 261 unit tests plus all battle/data/golden/web integration tests (one golden-regeneration test intentionally ignored).
