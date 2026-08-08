# 15 — Trade (op 0x19) + storage transfer (op 0x1E) + bank gold (op 0x1D)

**What to build:** Trao đổi vật phẩm/vàng giữa 2 người chơi (gồm pet trade và transfer vật phẩm 1 chiều), chuyển đồ giữa các kho Homdo↔TienTrang↔LuuLang, và gửi/rút vàng vào ngân hàng. Nguồn tài nguyên và giao dịch.

**Blocked by:** 11 — Inventory base (cần item/slot path).

**Status:** implemented in `bb88915`; database-backed golden capture remains deferred

- [x] Op 0x19 sub 1 open: `data[6..9]` partner; cả 2 nhận `F44406001901`+le32(other); pet trade `F4440600190A` (pet names 28-char pad `6`) (Ch2 §2.3.15).
- [x] Sub 2 — set gold + items: parner nhận `F444`+len+`1903`+gold+item entries.
- [x] Sub 3 confirm/cancel: cả 2 accept → `GoldTransfer`, swap both directions; hết slot → `F4440300190207`; success → `F4440300190204`; cancel → `F4440300190203` partner + `F4440300190209` self, `TradeFinish`.
- [x] Sub 10/11/12 — pet trade: open, offer pet, confirm/cancel (`F4440300190B03/04/07/0A/0F` family).
- [x] Sub 20 — transfer item: recipient bytes 10..13, 9 slot/count; recipient `F4440E001706`+items; sender re-send `F444`+len+`1705`.
- [x] Op 0x1E storage transfer (TienTrang): sub1 TienTrang→Homdo per-move detail + end `F44402001732`; sub2 Homdo→TienTrang per-move `F44404001709`+slot+`32` rồi `F444`+len+`1E04`; sub8 set `SelectMenu=40` (Ch2 §2.3.19).
- [x] Op 0x1D bank gold: request/response amount dùng LE32 theo C#; withdraw/deposit enforce balance và cap `9,999,999`, persist atomically.
- [x] Mọi thao tác bảng cất trữ có `player_id`.
- [ ] Golden: một vài scenario trade/bank nếu capture đủ deterministic.

## Audit kết quả (2026-08-08)

**Kết luận:** Chưa đáp ứng ticket. Rust hiện chỉ có scaffold cục bộ cho trade/bank/storage; chưa có settlement giữa hai Player, persistence đầy đủ, pet trade, item transfer thực tế hoặc LuuLang path. Không được đánh dấu hoàn tất cho đến khi các gap dưới đây được implement và test.

### Rust provenance

- Dispatcher đã route `0x19`, `0x1D`, `0x1E` tới handler tại `src/server/handler.rs:183-196`.
- Trade scaffold tại `src/server/handlers/trade_storage.rs:7-72`: sub 1 chỉ mutate local session và gửi packet local; sub 2 chỉ lưu gold, không parse item slots; sub 3 chỉ set local accepted/cancel; sub 10/11/12 là response stubs; sub 20 không move item/recipient, chỉ gửi `1706` placeholder và dump Homdo.
- Storage scaffold tại `src/server/handlers/trade_storage.rs:74-121`: sub 1 remove trước khi biết Homdo còn slot, bỏ qua kết quả `add_homdo_item`; chỉ gửi dump + end, thiếu per-move detail. Sub 2 dùng `len()+1`, thiếu capacity/rollback/persistence. Chưa có LuuLang handling trong handler. Sub 8 mới set `select_menu=40`.
- Bank scaffold tại `src/server/handlers/trade_storage.rs:124-165`: đọc amount bằng LE16 từ 2 byte (`:129-132`) thay vì request LE32; withdraw có cap gold nhưng deposit thiếu cap bank; amount 0 được chấp nhận; không có persistence.
- Session có các vector `homdo`, `tientrang`, `luulang` và `bank_gold` tại `src/server/session.rs:185-203`, `:288-303`, nhưng `bank_gold` khởi tạo 0 và không được load/save. `TradeState` tồn tại tại `:217-225` nhưng chưa được dùng cho settlement.
- Registry cross-player tồn tại tại `src/server/session.rs:436-443`, nhưng ticket-15 handler không dùng; connection loop snapshot/update tại `src/web/server_control.rs:341-353` cần được thay bằng domain operation atomic dưới lock để tránh lost update.
- Persistence primitives có `update_player` và `upsert_item` với `player_id` tại `src/db/persist.rs:13-62`, `:80-182`, nhưng chưa được gọi bởi ticket-15 handlers. Existing atomic shop transaction tại `src/db/persist.rs:184-221` chỉ phục vụ player shop, không đủ cho trade/storage/bank/pet.
- Schema có `player_id` trong gameplay tables, gồm `homdo` `migrations/0001_init.sql:79-111`, `luulang` `:113-145`, `tientrang` `:226-258`; `players` chưa có bank column tại `:21-73` (Gold ở `:55`). Bank cần column hoặc slot-zero design có scope rõ ràng. Migration hiện còn duplicate `pet.Hpx` tại `:163-165`, cần xử lý trước integration DB test.
- Existing direct tests tại `src/server/handlers/trade_storage.rs:167-226` chỉ cover local trade flags và bank happy path; không cover cross-player, items, pets, storage, LuuLang, persistence, malformed input, caps, rollback hoặc golden capture.

### C# provenance (reference only, không chỉnh sửa)

- Normal trade dispatcher/handler: `ts_server_old/Server_TS_Online/Client.cs:907-908`, `:6099-6426`.
- Trade open both sides and frames `F44406001901`: `Client.cs:6104-6123`.
- Gold + item offer, slot list and `1903` item payload: `Client.cs:6125-6156`.
- Confirm/cancel, capacity result `F4440300190207`, success `...0204`, cancellation `...0203/0209`: `Client.cs:6158-6218`.
- `TradeFinish` reset: `Client.cs:10073-10085`; `GoldTransfer`: `Client.cs:10105-10145`.
- Pet trade open/offer/confirm/cancel and frames `190A/190C/0B03/04/07/0A/0F`: `Client.cs:6220-6357`.
- One-way item transfer parses recipient bytes `[10..13]` and nine slot/count pairs, then sends recipient `1706` and sender Homdo dump: `Client.cs:6359-6422`.
- Storage dispatcher/handler: `Client.cs:919-920`, `:5884-6000`. TienTrang→Homdo per-move packets/detail/end: `:5889-5942`; Homdo→TienTrang update + `1E04`: `:5944-5994`; select menu: `:5996-5999`.
- LuuLang is not handled by `0x1E`; C# implements Homdo→LuuLang at `Client.cs:5761-5817` (op `0x17` sub 51) and LuuLang→Homdo at `:5818-5879` (sub 52), with pet guard and `1766`/`1708`/`1768` packets.
- Bank dispatcher: `Client.cs:7314-7325`; withdraw/deposit in `ts_server_old/Server_TS_Online/FTienTrang.cs:5-49`. Both requests parse LE32 (`packet[6..9]`), enforce `9,999,999` cap, update storage money and player gold, and emit amount packets. Bank slot-zero persistence helpers: `Data.cs:2822-2839`; player gold write helper `Data.cs:233-237`.

### Gap list and handling plan

1. **Trade cross-session state missing.** Use a single locked online-registry operation; validate both online/trading states; update both trade states and emit packets to both sides in C# order.
2. **Normal item offer/settlement missing.** Parse offered slots, validate ownership and gold before mutation, reserve via trade state, validate destination capacity, atomically swap item rows and gold, then emit `1903`, `1706`, result frames. Preserve C# packet forms while preventing partial data loss.
3. **Pet trade missing.** Store pet status and serialized offer, validate ownership/duplicate/empty destination slots, atomically exchange pet rows and gold, emit the `0B03/04/07/0A/0F` family.
4. **Sub 20 item transfer is placeholder.** Parse nine slot/count pairs, validate recipient and counts, atomically remove/add across player inventories, send recipient `F4440E001706` entries and sender `1705` dump.
5. **Storage moves are unsafe and incomplete.** Implement multi-slot packet parsing, destination free-slot selection, validate-before-remove, persist both tables in one transaction, emit per-move details and terminal packets. Add op `0x17` sub `51/52` LuuLang paths with required pet guard and packet family.
6. **Bank amount/cap/persistence missing.** Parse LE32 request; reject zero/invalid amounts; enforce both balance and `bank + amount <= 9,999,999`; persist gold and bank atomically. Prefer an explicit scoped `players.bank_gold` column unless capture/schema evidence requires slot-zero TienTrang.
7. **`player_id` contract.** Keep `player_id` predicates on every gameplay-table SQL statement, including cross-player transaction helpers and pet/item updates.
8. **Malformed input and rollback.** Treat malformed slot lists as silent no-op where C# is silent, but never remove source state before destination validation; rollback all DB mutations on failure.
9. **Acceptance coverage absent.** Add deterministic unit/integration tests for both-sided trade, cancel/failure, item transfer, pet duplicate/full-slot, all storage directions, bank cap/LE32, persistence scope, and golden packet scenarios where captures are available.

## Notes / deferred

- No `Notes / deferred` section existed in the original ticket; this section records decisions from the audit/grilling session.
- Normative scope includes LuuLang because the ticket explicitly names it, but its legacy wire paths are `0x17` sub 51/52 rather than `0x1E`; implement those paths without inventing new opcodes.
- “C# bug-for-bug” means preserve observable packet literals/order where required, not intentional Rust data loss or unrecoverable partial writes. Rust will use atomic domain operations and DB transactions.
- Cross-player registry mutation must be serialized under one lock or equivalent per-player lock ordering. Do not mutate a copied recipient session and silently discard it.
- Bank request width is LE32 per C# (`FTienTrang.cs:5-49`) and research reference, despite the ticket’s response shorthand mentioning `le16`; response width must follow capture/spec literal once verified.
- Deferred until implementation seam is available: database-backed golden capture with two live MySQL players and deterministic pet/trade fixtures. Unit tests must land first.

## Implementation attempt validation

- `cargo check` passed after making the handler entry points async and adding the initial `BankGold` schema/load/persist path.
- `cargo test --lib server::handlers::trade_storage` passed (2 tests), but these remain local happy-path tests and do not establish ticket completion.
- Full `cargo test` exceeded the 120-second execution limit; no full-suite result is claimed.
- Current Rust changes are partial groundwork only. The ticket remains open because normal/pet trade settlement, atomic cross-player mutation, sub-20 transfer persistence, storage persistence, LuuLang sub 51/52, and deterministic golden coverage are not complete.
- No files under `ts_server_old/` or `docs/` were edited. No commit was created.

## Latest implementation status (2026-08-08 10:18 +07)

Phần audit ở trên mô tả trạng thái trước implementation attempt. Trạng thái mã nguồn hiện tại được cập nhật như sau; các mục chưa hoàn chỉnh vẫn giữ checkbox mở.

### Đã thay đổi

- Dispatcher gọi ba handler ticket #15 theo async tại `src/server/handler.rs:183-196`.
- Bank request đã parse LE32 và reject payload ngắn/amount 0 tại `src/server/handlers/trade_storage.rs:125-136`.
- Withdraw đã có gate `bank >= amount` và `gold + amount <= 9_999_999`; deposit đã có gate đủ gold và cap bank bằng `saturating_add` tại `trade_storage.rs:138-197`.
- Bank response amount hiện encode LE32 theo C# `smethod_12`, không còn LE16 như scaffold cũ (`trade_storage.rs:160-165`, `:191-196`). Ticket line 15 ghi `le16` là shorthand cũ và không còn là width implementation mục tiêu.
- Schema thêm `players.BankGold` tại `migrations/0001_init.sql:55-57`; login load `BankGold` vào session tại `src/db/players.rs:267-325`; persistence whitelist có `BankGold` tại `src/db/persist.rs:13-44`.
- Bank handler hiện gọi persistence cho cả `Gold` và `BankGold` tại `trade_storage.rs:145-158`, `:176-189`.
- Ticket #14 đã bổ sung ordered per-player operation locks tại `src/server/session.rs:443-475` và connection loop dùng lock từ snapshot load tới publish tại `src/web/server_control.rs:333-366`. Đây là seam có thể tái sử dụng cho trade buyer/partner, nhưng `handle_trade` hiện chưa khóa/mutate partner.

### Vẫn chưa đạt

1. **Normal trade vẫn local-only.** `trade_storage.rs:14-45` chỉ set `TradeState` của caller; partner không được mở trade, không nhận offer/cancel, và confirm gửi success ngay khi một bên accept.
2. **Gold/item settlement chưa tồn tại.** Sub 2 chưa parse slot list hoặc validate offered gold; sub 3 chưa kiểm tra cả hai accept, capacity, swap item, transfer gold, rollback hoặc persist hai Player.
3. **Pet trade vẫn là stub.** Sub 10 chỉ gửi caller frame; sub 11/12 trả literal cố định, không serialize pet name/data, validate duplicate/full slot hoặc chuyển ownership (`trade_storage.rs:47-61`).
4. **Sub 20 chưa chuyển item.** Hiện chỉ đọc recipient id, tạo placeholder `1706` chứa recipient id và dump Homdo không thay đổi; chưa parse chín slot/count pairs (`trade_storage.rs:62-69`).
5. **Storage vẫn unsafe và single-slot.** TienTrang→Homdo remove source trước khi biết add thành công, có thể mất item khi Homdo full (`trade_storage.rs:81-97`). Homdo→TienTrang dùng `len()+1`, có thể collision khi slot có gap, và payload `1E04` chưa chứa item detail thật (`:99-115`).
6. **Storage chưa persistence/transaction.** Không có `persist::upsert_item` hoặc transaction cho `homdo`/`tientrang`; multi-slot packet và rollback chưa được implement.
7. **LuuLang vẫn thiếu.** Op `0x17` sub 51/52 chưa được route/implement; chỉ có session vector và login dump/load (`src/server/session.rs:185-191`, `:357-370`; `src/db/players.rs:23-40`).
8. **Bank persistence chưa atomic.** Hai lần `persist::update_player` là best-effort độc lập; một write có thể thành công và write còn lại thất bại. Cần một transaction cập nhật đồng thời `Gold` + `BankGold`, rollback in-memory khi commit lỗi.
9. **Bank schema migration compatibility chưa giải quyết.** Thêm column vào `CREATE TABLE IF NOT EXISTS players` không alter database đã tồn tại. Cần migration mới `ALTER TABLE ... ADD COLUMN BankGold` hoặc migration versioned tương đương trước khi deploy trên DB cũ.
10. **Bank packet parity cần capture.** Handler gửi thêm full gold status `F4440A001A04...` sau hai amount frames (`trade_storage.rs:162-165`, `:193-196`), trong khi C# `FTienTrang.cs:5-49` chỉ xác nhận hai amount frames trong nhánh này. Cần capture diff trước khi giữ packet thứ ba.
11. **Test coverage vẫn chưa đủ.** Chỉ có local trade flag và bank happy path tại `trade_storage.rs:203-263`; chưa có cap/reject/atomic DB/cross-player/storage/pet/sub20/LuuLang tests hoặc ticket-specific golden.

### Validation mới nhất

- `cargo check`: **passed**.
- `cargo test server::handlers::trade_storage -- --nocapture`: **2/2 passed**.
- Full suite gần nhất trên cùng worktree: `cargo test -- --test-threads=1`: **261 unit tests + toàn bộ battle/data/golden/web integration tests passed**; một golden-regeneration test intentionally ignored.
- Các kết quả pass chỉ xác nhận code hiện tại compile và không làm hỏng suite hiện hữu; không đủ để đánh dấu ticket #15 hoàn tất vì các capability chính phía trên chưa có test.
- `docs/` và `ts_server_old/` vẫn không được chỉnh sửa hoặc đưa vào commit.

## Latest implementation status (2026-08-08 11:56 +07)

Mục này thay thế kết luận trạng thái tại phần `10:18 +07`; phần audit cũ phía trên được giữ làm lịch sử provenance.

### Hoàn tất trong commit `bb88915`

- Trade normal dùng online-session registry authority và ordered per-player operation locks. Open/offer/accept/cancel cập nhật cả hai Player; settlement validate ownership, gold và capacity trên state clone trước khi mutate/persist.
- Settlement normal trade chuyển item/vàng hai chiều, emit gold status, `1706`, success/capacity/cancel packet đúng family, reset `TradeState`, và rollback caller/partner khi transaction thất bại.
- Pet trade hỗ trợ open, pet hoặc gold-only offer, tên pet pad 28 ký tự `6`, duplicate/full-slot validation, ownership transfer, gold transfer, donor `0F02`, recipient `0F07`/`0F01`, cùng `190B` result family.
- Sub 20 parse recipient tại request bytes `[10..13]`, tối đa chín cặp slot/count, reject duplicate/malformed slot, validate destination trước khi remove, persist hai Homdo atomically, gửi recipient `1706` có `DoBen` và sender `1705` dump.
- Op `0x1E` hỗ trợ multi-slot TienTrang→Homdo và Homdo→TienTrang với free-slot validation, transaction cho cả hai bảng, packet `1708`/`1E05`/`1732` và `1709`/`1E04` theo hướng tương ứng.
- Op `0x17` sub `51/52` đã route cho Homdo↔LuuLang, giữ pet gate `41187/18023`, dùng packet family `1766`/`1708`/`1768`, validate-before-remove và persistence transaction.
- Bank parse LE32, reject amount `0`/payload ngắn, enforce balance và cap cả hai hướng, update `Gold` + `BankGold` trong một transaction và rollback memory khi DB fail. Bank branch chỉ emit hai amount frames theo `FTienTrang.cs`.
- Thêm migration versioned `migrations/0002_bank_gold.sql`; không đưa thay đổi checksum của migration `0001` vào commit.
- Item persistence round-trip thêm `Lv/Hp/Sp`; pet load/persistence round-trip toàn bộ base stats, `_2` stats, `Thd`, quest và bốn skill. Mọi DELETE/INSERT/UPDATE gameplay đều scope bằng `player_id`.

### Acceptance coverage

- Sáu test handler deterministic cover normal trade success, full-destination rollback, pet ownership/gold settlement, sub-20 partial-count transfer, TienTrang/LuuLang directions và bank LE32/cap.
- `cargo check`: **passed** sau code-review fixes.
- `cargo test --lib server::handlers::trade_storage -- --nocapture`: **6/6 passed**.
- `cargo test --lib db:: -- --nocapture`: **6/6 passed**.
- `cargo test --lib server::handler::tests::dispatch_trade_bank_pk_and_pets -- --exact`: **passed**.
- Full suite được chạy một lần trước review fixes: **264/265 passed**; failure duy nhất là expectation bank cũ đòi packet thứ ba. Expectation đã sửa theo C# và targeted regression sau sửa đã pass. Full suite không được chạy lại lần hai, nên không ghi nhận full-suite green sau commit.
- `/code-review` được chạy nhiều vòng. Các finding High về rollback, packet storage/sub20, migration checksum, persistence loss, gold/pet UI synchronization đã được xử lý trước commit.

### Còn deferred

- Checkbox Golden vẫn mở: chưa có database-backed golden capture với hai MySQL Player online và fixture trade/pet deterministic.
- Chưa chạy integration test trực tiếp trên MySQL 8 cho migration `0002` và transaction cross-player; coverage hiện tại dùng in-memory handler seam và DB repository unit tests.
- `migrations/0001_init.sql` vẫn có duplicate legacy `pet.Hpx` trong baseline. Commit ticket không sửa file migration đã áp dụng để tránh SQLx checksum mismatch; cần một quyết định migration-repair riêng cho fresh database bootstrap.
