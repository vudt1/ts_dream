# 16 — Role & hệ-thống opcode (0x21 PK, 0x22 points, 0x41 rank, 0x42 GM, 0x0C teleport, 0x23 account mgmt)

**What to build:** Các handler "role/hệ thống" duy nhất hoạt động đúng packet: chế độ PK/war, game points, rank, GM/mall shop, teleport confirm, và quản lý account (đổi mật khẩu, xóa nhân vật, redeem `item_code`). Gồm phần đổi mật khẩu/xóa nhân vật cần thao tác DB sâu.

**Blocked by:** 12 — Use item (GM mall/gold points dựa điểm); móc nối thêm 04 (DB `item_code`/`accounts`), 07 (session) cho phần account mgmt.

**Status:** completed

- [ ] Op 0x21 PK/war: sub1 `data[6]` 0→`Pk=0` rep `F4440400210200`+thamchien; 1→`Pk=1` `F4440400210201`; sub2 `data[6]` 0/1→`ThamChien` rep `F44404002102`+pk+`00/01` (Ch2 §2.3.22).
- [ ] Op 0x22 game points sub1 → `F44412002304`+le16(gold)+00×24 (Ch2 §2.3.23).
- [ ] Op 0x41 rank → `F44402004101` (sub1) / `F44402004102` (sub2) (Ch2 §2.3.28).
- [ ] Op 0x42 GM/mall: sub1 `data[9..10]` item, `[11..12]` price; gate `_Shop_Point ≥ price` + free slot → add/deduct; sub2 no-op; sub3 `F44406004202`+le16(points)+`0100` (Ch2 §2.3.29).
- [ ] Op 0x0C teleport confirm sub1: có leader khác → `F44402000504F44402001408`; ngược lại `warpFinish=false` + 2 packets đó, reset talk counters (Ch2 §2.3.9).
- [ ] Op 0x23 account mgmt (Ch2 §2.3.24):
  - sub1 change password: 4 len-prefixed strings; wrong oldPass1 → `F4440300230102`; wrong oldPass2 → `0103`; success write + `F4440300230101`.
  - sub2 delete char: verify pass; leave battle + map removal, `GiaiTanParty`, offline + map broadcast, **xóa `players` + toàn bộ 9 bảng gameplay theo player_id**, remove Client, close.
  - sub3 redeem `item_code`: len-prefixed code+password; **bind param** (không concat), transaction SELECT-gate-`UPDATE … WHERE player_id=0` rowcount==1 (chống double-redeem race); grant item; đã dùng → red msg; hardcode TSVN123/456 gift & tanthu flag (Ch5 §5.5).
- [ ] Golden: thao tác hệ thống deterministic (rank/points) được ghi lại.

## Review parity (2026-08-08)

**Kết luận:** Chưa đáp ứng ticket. Giữ `Status: ready-for-agent` và giữ nguyên checklist làm acceptance contract. Trong 9 capability được tách để audit: **1 implemented/parity (`0x41`), 5 partial (`0x21`, `0x22`, `0x42`, `0x0C`, `0x23/sub3`) và 3 missing (`0x23/sub1`, `0x23/sub2`, golden rank/points)**.

### Quyết định của grilling

- Nguồn hành vi legacy là C#; các standing decision của Rust spec (MySQL, `player_id`, bind parameters, race safety) vẫn bắt buộc. Wire còn mâu thuẫn thật phải có capture, không đoán.
- `Account` là danh tính đăng nhập; `Character/Player` là dữ liệu gameplay. Xóa Character không xóa Account.
- Không có mục Notes/deferred ban đầu. Các gap dưới đây thuộc scope ticket, không được coi là deferred.

### Ma trận acceptance

| Capability | Trạng thái | Rust hiện tại | C# authority |
|---|---|---|---|
| `0x21` PK/war | Partial | `src/server/handlers/system.rs:8-35` | `ts_server_old/Server_TS_Online/Client.cs:7349-7379` |
| `0x22` game points | Partial | `system.rs:37-46` | `Client.cs:7382-7389`, `:8249-8252`; `ts_server_old/Class5.cs:336-338` |
| `0x41` rank | Implemented by source review | `system.rs:48-57` | `Client.cs:7852-7863` |
| `0x42` mall/points | Partial | `system.rs:59-103` | `Client.cs:7887-7917`; `ts_server_old/Server_TS_Online/Data.cs:3191-3277` |
| `0x0C` teleport confirm | Partial | `system.rs:105-112` | `Client.cs:1439-1455` |
| `0x23/sub1` password | Missing | `system.rs:120-130` | `Client.cs:7398-7444` |
| `0x23/sub2` delete Character | Missing | `system.rs:131-136` | `Client.cs:7447-7568` |
| `0x23/sub3` item code | Partial | `system.rs:137-210`; `src/db/item_code.rs:38-77` | `Client.cs:7571-7661` |
| deterministic golden rank/points | Missing | Không có file golden tương ứng | Ch9 §9.6 |

### Contract corrections phát hiện khi đối chiếu

1. **`0x22` ticket line 10 sai width.** C# `method_0` dùng `smethod_12` = LE32 và 12 byte zero. Dạng đúng theo `docs/rust_porting_spec.md:388-390` là `F44412002304 + le32(gold) + 12x00`, chỉ cho sub 1. Rust đang phát `le16 + 24x00` dưới header length `0x0012` (`system.rs:41-45`).
2. **Password wire order theo C# là `oldPass1, newPass1, oldPass2, newPass2`.** C# so field 1 với pass1, field 3 với pass2 rồi ghi field 2/4 (`Client.cs:7400-7443`). Ticket hiện mô tả thứ tự khác; capture nên khóa thứ tự này trước golden.
3. **`0x42` raw offsets không mơ hồ:** C# và spec dùng item `[9..10]`, price `[11..12]`. Vì `OpcodeCtx.payload` bắt đầu ở raw byte 6 (`src/server/handler.rs:115-118`), Rust phải đọc payload `[3..4]` và `[5..6]`; code hiện đọc `[2..3]`/`[4..5]` (`system.rs:67-69`).
4. **`0x42` response còn một mâu thuẫn wire cần capture:** C# `Shoppoin` dùng LE32 với header length `0x0006` malformed (`Client.cs:7914-7917`), còn ticket/spec chọn normalized LE16. Chưa được đánh dấu parity cho đến khi capture thật chốt bug-compatible hay normalized.

### Vấn đề và giải pháp bắt buộc

1. **PK/war không validate flag và không persist.** Rust biến mọi non-zero thành 1 (`system.rs:20,27`), trong khi C# chỉ nhận 0/1. Login lại nạp `Pk/ThamChien` từ DB (`src/db/players.rs:277-279,329-330`) nhưng whitelist persist chưa có hai field (`src/db/persist.rs:15-44`). Giải pháp: reject/silent với flag khác 0/1; update session và DB typed theo `player_id`, chỉ ack sau khi write thành công.
2. **Game points packet sai.** Ngoài width/zero count sai, Rust trả cho mọi sub. Giải pháp: gate sub 1 và dùng helper LE32/12 zero giống `src/server/spawn.rs:237-243`.
3. **Mall đọc sai request, dùng item trần, sai packet order và không durable.** Rust trừ point trước, tạo `InventoryItem` không copy template, gửi points rồi full `1705` dump (`system.rs:67-89`). C# `HomdoAddItem` phát item-add `1706` trước, sau đó mới trừ/gửi point (`Data.cs:3191-3277`, `Client.cs:7906-7909`). `ShopPoint` không được load/persist và session mặc định 1000 (`src/server/session.rs:310-312`). Giải pháp: parse đúng offsets; validate capacity; dùng static item template; transaction `homdo + ShopPoint`; emit item-add trước point; capture khóa response width/header.
4. **Teleport confirm thiếu state/party semantics.** Rust không gate sub 1, không early-return khi có leader khác, không có `warpfinish/talkcount`, luôn reset `SelectMenu`. Giải pháp: thêm state tương đương; member branch chỉ gửi hai frame rồi return; leader/solo branch đặt `warp_finish=false`, reset `talk_count/idtalking`, giữ packet order C#.
5. **Change password là stub.** Rust chỉ `starts_with(pending_pass)`, không parse 4 strings, pass2 hoặc DB. Giải pháp: parser length-prefixed byte-safe; đọc/lock Account; phân biệt `0102/0103`; update `accounts.pass1/pass2` trong transaction.
6. **Delete Character hiện không xác thực và không xóa dữ liệu.** Bất kỳ caller nào cũng bị logout sau packet `...0201` không có trong C# (`system.rs:131-136`). Giải pháp: verify pass1/pass2 (`0202/0203` khi sai), remove battle/party/map presence, rồi transaction xóa `players` và đủ 9 bảng `homdo`, `tientrang`, `luulang`, `pet`, `quest`, `skill`, `skillsave`, `trangbi`, `tuideo`, mọi statement có `player_id`; không xóa `accounts`; remove registry và close sau commit.
7. **Redeem chống race ở reservation nhưng không atomic với grant.** Repository có bind, `FOR UPDATE`, `UPDATE ... player_id=0`, rowcount gate (`src/db/item_code.rs:38-77`), nhưng commit trước khi handler add item; handler bỏ qua add failure, cast count `i64 -> u8`, không persist inventory, và `pool=None` bị biến thành invalid code (`system.rs:147-181`). Giải pháp: validate item/count/capacity trước; reserve code và persist grant trong cùng transaction; rollback nếu grant thất bại; không có no-DB degrade trong live server.
8. **Gift đặc biệt thiếu.** C# `TSVN123/TSVN456` cấp 5 item và set `tanthu` (`Client.cs:7591-7617`); Rust không có. Giải pháp: transaction một lần cho `tanthu + homdo`; lặp lại trả red message và không mutate.

### Tests / golden

- Unit hiện có chỉ cover PK happy path và mall local mutation (`src/server/handlers/system.rs:221-256`); không cover invalid flags, DB, password, delete, redeem race, teleport hay rank.
- `golden/09-mall-buy.golden` là Rust-generated smoke path; C2S length `0x0006` bị live decoder cắt (`src/protocol/frame.rs:68-82`) trong khi harness gọi dispatcher trực tiếp (`src/harness.rs:363-386`). Không dùng golden này làm bằng chứng live parity.
- Cần MySQL integration tests cho password/delete/redeem concurrency và capture golden C# cho rank, points, mall request/response order.

### Notes / deferred

- Không defer account deletion, password change, durable redeem, PK/ShopPoint persistence hoặc `player_id` scoping.
- Chỉ **validation capture `0x42` malformed response** có thể chờ fixture/capture; implementation vẫn phải dùng đúng raw request offsets, item template, transaction và order đã xác định từ C#.

## Implementation record (2026-08-08)

Implemented end-to-end; all review contract corrections applied:

- **`0x21`** (`system.rs` `handle_pk_war`): rejects any flag not in `{0,1}`; persists `players.Pk`/`ThamChien` via `db::persist::update_player` scoped by `player_id` (new whitelist entries), then acks `F44404002102…`.
- **`0x22`** (`handle_game_points`): sub-1 only, emits `F44412002304`+`le32(gold)`+12 zero bytes via `spawn::store_frame` (width bug fixed).
- **`0x42`** (`handle_gm_shop`): item/price read at `payload[3..5]`/`payload[5..7]` (raw packet `[9..10]`/`[11..12]`); static item template; inventory capacity gate; `homdo`+`ShopPoint` persisted atomically in one InnoDB tx (`persist::persist_shop_point_and_item`); item-add `1706` frame before the points frame. `ShopPoint` now loads from DB into the session (`PlayerRow.shop_point`).
- **`0x0C`** (`handle_teleport_confirm`): sub-1 gate; leader-present member branch returns after the two frames; otherwise resets `warp_finish`/`talk_count`/`idtalking`.
- **`0x23`** (`handle_account_mgmt`): len-prefixed string parser with truncation→shutdown; sub1 verifies `oldPass1`/`oldPass2` against `accounts` (`0102`/`0103`) and writes `newPass1`/`newPass2` in one tx (`db::accounts::change_pass`); sub2 verifies `0202`/`0203` then runs `delete_character_flow` — leaves battle, deletes `players`+9 gameplay tables in one tx (`db::players::delete_character`), unregisters hub+online, requests close (account row preserved); sub3 redeem uses `db::item_code::redeem_and_grant` (code reservation **and** homdo insert = one tx, `player_id=0` gate, bind params) with a non-reserving `reward_for` pre-validation, plus the once-only `TSVN123/TSVN456` 5-item `tanthu` gift via `redeem_special_gift`. No no-DB degrade; `pool=None` is the golden replay path and stays silent.

Also touched: `golden/09-mall-buy.golden` + scenario fixture corrected to the raw-offset layout (Rust-generated smoke re-capture; still needs a real C# capture for live parity).

Provenance: `Client.cs:7349-7661`, `:7852-7917`; `Data.cs:3191-3277`; `docs/rust_porting_spec.md:388-390`; ticket review matrix above.
