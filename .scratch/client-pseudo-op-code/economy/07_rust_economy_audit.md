# 07 — Rust Economy Audit (buy / sell / trade / currency)

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

> **Phạm vi**: 100% phía Rust server của `ts_dream/` (schema, state, persistence, invariants).
> **Ngoài phạm vi**: decompile client (`test/ts_decompile/`, `client_pseudo_c/`) — chỉ tham chiếu chéo
> từ `TS_Server_Bear/TS_Server/` (C# reference server) cho tên cột / schema.
> **Quy ước**: mọi khẳng định đều kèm `file:line`. Chỗ nào chỉ là suy đoán sẽ đánh dấu `[suy đoán]`.
> Trạng thái: **hoàn tất** (2026-10-02). Tổng 65 GAP (G1–G61 + M1–M4), 50 test đề xuất.

---

## 1. Kiến trúc kinh tế hiện tại

### 1.1 Data-flow tổng thể

```text
                    ┌──────────────────────── TCP :6414 (F4 44 + XOR 0xAD) ────────────────────────┐
                    │                                                                            │
  C→S frame ────────► dispatcher::handle()  src/server/dispatcher.rs:252-397                     │
                    │   match ctx.opcode  (MainOp 1 byte, không có bảng phụ)                        │
                    │                                                                            │
                    ├── 0x1B ──► shops::handle_npc_shop()          shops.rs:166                   │
                    ├── 0x17 ──► inventory::handle_inventory()      handlers/inventory.rs         │
                    │        └─ sub 30..33 ─► shops::handle_player_shop()  shops.rs:412             │
                    ├── 0x19 ──► trade_storage::handle_trade()      trade_storage.rs (2-phase)     │
                    ├── 0x1D ──► trade_storage::handle_bank_gold()  trade_storage.rs:707           │
                    ├── 0x1E ──► trade_storage::handle_storage_transfer()                           │
                    ├── 0x42 ──► system::handle_gm_shop()           system.rs:116                  │
                    ├── 0x48 ──► unimplemented::handle()  (log-only, KHÔNG trả lời)  dispatcher.rs:391
                    ├── 0x3A ──► unimplemented::handle()                          dispatcher.rs:380
                    ├── 0x23 ──► system::handle_account_mgmt()  (pass2 / delete / item_code)       │
                    ├── 0x2C ──► skills::handle_pet_reborn()                                   │
                    ├── 0x0F ──► pet_actions::handle_pet_actions()                                  │
                    ├── 0x0C ──► system::handle_teleport_confirm()  (KHÔNG phải shop)               │
                    └── 0x4B ──► (không có arm) ─► _ => unimplemented::handle()   dispatcher.rs:394
                    │
  Session (RAM) ◄──┴──► handlers mutate trực tiếp `session.gold` / `session.homdo` …
                    │            src/server/session.rs:196 `pub gold: u32`
                    │            src/server/session.rs:242 `pub bank_gold: u32`
                    │            src/server/session.rs:243 `pub shop_point: u32`
                    ▼
        ┌──────────── SESSION LAYER: không có service/repository cho tiền ────────────┐
        │  shop_point, giá niêm yết, tiền khách mua đang trả … đều là biến RAM     │
        └──────────────────────────────────────────────────────────────────────────────┘
                    │  (a) write-through ngay trong handler
                    ▼
        persist::persist_shop_transaction()  src/db/persist.rs:227-262
        persist::persist_shop_point_and_item()  src/db/persist.rs:324-354
        persist::update_player(.., "Gold"|"BankGold"|"ShopPoint")  persist.rs:81-101
                    │  SQLx transaction trên `pool.write` (1 session độc quyền)
                    ▼
        ┌───────────────────── SQLite `DB/ts_dream.db` (WAL) ──────────────────────┐
        │  character_money(playerid, gold, bankgold, shoppoint)  0001_init.sql:83  │
        │  inventories(playerid, storagetype, slot, itemid, quantity …) :96        │
        │  character_pets(..) :130   mails(gold, attachmentitemid…) :254           │
        │  KHÔNG có bảng nào cho: shop catalogue / price list / NPC stock / stall    │
        └──────────────────────────────────────────────────────────────────────────┘
                    │  (b) write-back định kỳ
                    ▼
        auto_save::run_cycle()  auto_save.rs:127-164  (mỗi 180s)
          → fingerprint(session)  auto_save.rs:38-120  (mix gold, bank_gold, shop_point, items…)
          → persist::persist_sessions_transaction()  persist.rs:264-278
          → db::modern::sqlite::session::SqliteSessionRepository::save()  session.rs:296 (UPSERT 3 cột tiền)
                    │
                    ▼
        S→C frames:  shops::gold_frame(gold)      shops.rs:11-13   → "F4440A001A04" + le32(gold) + "00000000"
                     trade_storage::gold_status_frame(gold)  :125  → "F4440A001A04" + le32(gold) + "00000000"
                     spawn.rs:561-564 (login announce)          → cùng code 1A04
                     shops::red_message(msg)                   shops.rs:18-22 → code 020B (toast đỏ)
```

### 1.2 Nhận xét kiến trúc

- **Không có tầng service cho tiền.** Handler ghi thẳng `session.gold`, rồi gọi
  `persist::persist_shop_transaction`. Không có `MoneyService` / `PriceService` tương đương
  `BattleService` (`src/battle/service.rs`). Hệ quả: mọi invariant (cap, đủ tiền, rollback) phải
  nhân bản thủ công ở từng handler.
- **Hai đường ghi DB không thống nhất**: (a) write-through trong handler (đúng ngay lập tức,
  có transaction), (b) auto-save 180 giây ghi **toàn bộ** snapshot session
  (`persist_sessions_transaction` → `SqliteSessionRepository::save`). Hai đường có thể ghi
  chồng: nếu auto-save chạy giữa lúc handler đang xử lý, snapshot cũ có thể đè lên giá trị
  mới hơn `[suy đoán — không có test nào chứng minh thứ tự]`.
- **`shop_point` chỉ đi qua 1 đường**: `system::handle_gm_shop` (`system.rs:116-161`) →
  `persist_shop_point_and_item`. Không có nguồn nạp/tiêu nào khác.
- **Stall (shop người chơi) là state RAM thuần**: `PlayerShopState` (`session.rs:121-127`)
  + `ShopItem` (`session.rs:112-117`). Không có bảng, không có restore khi restart.

---

## 2. Bảng tiền tệ

### 2.1 Danh sách field tiền trong Rust

| Field | Struct / `path:line` | Kiểu (width) | Persist? | Đọc từ DB | Ghi ra DB | Đẩy S→C |
|---|---|---|---|---|---|---|
| `Session.gold` | `src/server/session.rs:196` | `u32` (4B LE) | ✅ | `db/modern/sqlite/session.rs:27,67` (`COALESCE(m.gold,0)` → `as u32`) | `sqlite/session.rs:296-297` (UPSERT trong tx), `persist.rs:245,248` (tx shop), `persist.rs:81-101` (`update_player`) | ✅ `1A04` — `shops.rs:11-13`, `trade_storage.rs:125`, `spawn.rs:561-564` |
| `Session.bank_gold` | `src/server/session.rs:242` | `u32` | ✅ | `sqlite/session.rs:28,68` | `sqlite/session.rs:296-297`, `persist.rs:307-308` map `bank_gold`→`bankgold` | ⚠️ Chỉ qua `1A04` khi bank **chuyển** tiền (`trade_storage.rs:751` code `1A01/1A02`) và qua `chat.rs:414` (bank screen). Không có frame push riêng cho số dư bank. |
| `Session.shop_point` | `src/server/session.rs:243` | `u32` (default **1000**) | ✅ | `sqlite/session.rs:29,69` | `sqlite/session.rs:296-297`, `persist.rs:324-354` | ❌ **KHÔNG có frame nào push `shop_point`**. GM shop (`system.rs:116-161`) chỉ trả `020B` + item. |
| `Money` model (repo) | `src/db/modern/model.rs:97-99` | `i64` × 3 | — | `db/modern/sqlite/characters.rs:85-93` | — | — |
| `Session.hp_store` | `src/server/session.rs:199` | `u32` | ❌ không thấy trong `characters` | — | — | ❌ |
| `Session.sp_store` | `src/server/session.rs:200` | `u32` | ❌ | — | — | ❌ |
| `Session.god` | `src/server/session.rs:198` | `u32` | ✅ (auto-save mix, `auto_save.rs:52`) | — | qua `SqliteSessionRepository::save` (cột `god`?) | qua `0x22 handle_game_points` |
| `Session.texp` | `src/server/session.rs:195` | `u32` | ✅ (`auto_save.rs:48`) | — | ✅ | — |

**Xác nhận: KHÔNG có field `xu` / cash / cash-coin / point-ví-điện-tử nào trong Rust.**
Grep `xu\b|\bXU\b|cash|Cash|coin|Coin` trên `src/` chỉ trúng:
`src/battle/service.rs:284-297` (`apply_gold` — dùng `s.gold`),
`src/harness.rs` (từ "golden"), `src/battle/runner.rs:230` (comment "PvE gold reward").
→ **Không tồn tại loại tiền thứ 4+** trong mô hình hiện tại.

### 2.2 Bề rộng & phép chuyển đổi (range)

- Tất cả 3 field đều là `u32` ở tầng session nhưng `i64` ở tầng DB (`model.rs:97-99`) và khi ghi
  lại bị ép `as u32` (`sqlite/session.rs:67-69`). Một giá trị DB âm (nếu có) sẽ **wrap thành
  số rất lớn** khi load — `row.get::<i64,_>("gold") as u32` không có `try_into`/kiểm tra dấu.
  Đây là đường vào `gold > GOLD_CAP` giả tạo.
- Không có `CHECK (gold >= 0)` trên schema (`0001_init.sql:85-87`).
- Client nhận `le32(gold)` + 4 byte `00000000` dựng sẵn trong frame string (`shops.rs:12`).
  Nghĩa là 4 byte trailing đó **không phải** phần của số tiền mà là padding cố định của
  frame `1A04` `[suy đoán: cần đối chiếu client-side reader]`.

### 2.3 `GOLD_CAP`

- Định nghĩa: `src/server/trade_system.rs:18` → `pub(crate) const GOLD_CAP: u32 = 9_999_999;`
  doc comment `trade_system.rs:17`: *"Maximum gold either side may hold after a trade settles."*
- **Nơi áp dụng**:
  - `trade_system.rs:85` và `trade_system.rs:139` — kiểm tra cap sau khi settle trade P2P
    (trả `Err("gold")`).
  - `trade_storage.rs:724, 728` — kiểm tra cap khi rút/gửi ngân hàng (0x1D).
- **Nơi KHÔNG áp dụng** (GAP — xem §5):
  - `shops::handle_npc_shop` mua NPC (`shops.rs:245`) — chỉ kiểm `gold >= price`, không cap kết quả.
  - `shops::try_sell_range` (`shops.rs:42`) — `saturating_add` không cap.
  - `shops::complete_shop_buy` (`shops.rs:366`) — dùng **magic number `9_999_999` viết lại
    tay** thay vì `GOLD_CAP` (đúng giá trị, nhưng trôi khỏi hằng số → dễ lệch khi sửa).
  - `battle/service.rs:291,297` — `apply_gold` (thưởng PvE) `saturating_add`, không cap.
  - `gm.rs:270` — `/addgold` `saturating_add`, không cap.
- Nghĩa là `GOLD_CAP` **không phải invariant toàn cục**, chỉ là guard cục bộ cho 2 luồng
  (trade + bank). NPC shop / battle reward / GM add đều có thể vượt.

### 2.4 Frame tiền duy nhất

`gold_frame()` (`shops.rs:11-13`) và `gold_status_frame()` (`trade_storage.rs:125`) **trùng
nội dung byte**: `format!("F4440A001A04{}00000000", encoder::le32(gold))`. Đây là
`F4 44` + len `0A 00` + code `1A04` + 8 byte body (le32 gold + `00000000`).
Không có frame nào cho `bank_gold` hay `shop_point` định kỳ — xem §5 GAP-C3.

---
## 3. Lược đồ DB liên quan

### 3.1 DANH SÁCH ĐẦY ĐỦ mọi bảng trong `migrations/`

Chỉ có **một** file migration: `migrations/0001_init.sql` (26 797 bytes). 18 bảng:

| # | Tên bảng | `file:line` | Có liên quan kinh tế? |
|---|---|---|---|
| 1 | `accounts` | `0001_init.sql:18` | ⚠️ `gmlevel` (cổng GM shop) |
| 2 | `characters` | `0001_init.sql:42` | ⚠️ `level`, `mapid`, `jobtype`… (không có tiền) |
| 3 | **`character_money`** | **`0001_init.sql:83`** | ✅ **bảng tiền duy nhất** |
| 4 | **`inventories`** | **`0001_init.sql:96`** | ✅ item mọi loại túi |
| 5 | `character_pets` | `0001_init.sql:130` | ✅ pet (vật phẩm có giá trị) |
| 6 | `character_skills` | `0001_init.sql:177` | ❌ |
| 7 | `character_hotkeys` | `0001_init.sql:189` | ❌ |
| 8 | `character_missions` | `0001_init.sql:199` | ❌ |
| 9 | `character_mission_flags` | `0001_init.sql:211` | ❌ |
| 10 | `character_bit_flags` | `0001_init.sql:223` | ❌ |
| 11 | `character_completed_events` | `0001_init.sql:233` | ❌ |
| 12 | `friends` | `0001_init.sql:244` | ❌ |
| 13 | **`mails`** | **`0001_init.sql:254`** | ⚠️ có `gold`, `attachmentitemid`, `attachmentcount`, `claimed` — **chưa dùng** |
| 14 | `item_code` | `0001_init.sql:272` | ⚠️ gift code — có `itemid`/`count` |
| 15 | `gm_audit_log` | `0001_init.sql:287` | ⚠️ audit (0x42 deny path) |
| 16 | `character_quest_tasks` | `0001_init.sql:303` | ❌ |
| 17 | `character_quest_dont` | `0001_init.sql:314` | ❌ |
| 18 | `character_quest_items` | `0001_init.sql:325` | ❌ (túi quest, tách khỏi `inventories`) |

Index: `accounts_suspended_idx`, `accounts_gm_level_idx` (`:32-33`),
`inventories_item` (`:123`), `mails_receiver` (`:270`), `item_code_code` + `item_code_redeem` (`:285-286`).

### 3.2 `character_money` — chi tiết

```sql
CREATE TABLE IF NOT EXISTS character_money (     -- migrations/0001_init.sql:83-88
    playerid  INTEGER PRIMARY KEY,
    gold      INTEGER DEFAULT 0,   -- :85  tiền mang theo
    bankgold  INTEGER DEFAULT 0,   -- :86  tiền gửi (Tiền trang)
    shoppoint INTEGER DEFAULT 0    -- :87  Kỳ Trân Các / shop đặc biệt
);
```

- **Không có `NOT NULL`, không có `CHECK (… >= 0)`, không có index phụ** (PK là đủ cho lookup theo `playerid`).
- `DEFAULT 0` cho cả 3 cột → tài khoản mới tạo mà không insert sẽ có `shoppoint = 0`, **khác** với
  `Session::default()` (`session.rs:373`) là `1000`. Xem GAP-P2.
- Kiểu `INTEGER` của SQLite = signed 64-bit; **không ràng buộc trần**. `GOLD_CAP = 9_999_999`
  chỉ là quy ước ở tầng Rust, không được DB cưỡng chế.

**Đối chiếu Bear C#**: `grep -rn "shoppoint|bankgold|BankGold|ShopPoint" TS_Server_Bear/` trả về
**0 kết quả** → Bear dùng tên khác hoặc lưu money ở bảng khác. Xác nhận Bear chỉ đọc
`client.getChar().gold` (`TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs:22,28`)
và `ItemInfo.BuyingPrice` / `SellingPrice` (`DataTools/ItemInfo.cs:51,53`;
đọc từ `Item.dat` tại `DataTools/ItemData.cs:101-102`) — **giá nằm trong Item.dat, không nằm
trong DB**. Rust đã load 2 field này (`src/data/loaders/item.rs:34-35`) nhưng **không dùng**
(grep `\.price` trong `src/` trừ `shops.rs` → không có hit nào đọc `ItemDef.price`).

### 3.3 `inventories` — chi tiết

```sql
PRIMARY KEY (playerid, storagetype, slot)   -- 0001_init.sql:120
storagetype: 1=HomDo 2=TienTrang 4=TuiDeo 8=TrangBi 16=LuuLang   -- :98
```
23 cột ánh xạ 1:1 với wire `ThingData` 35-byte (`:97-119`).

**Quan sát quan trọng**: schema định nghĩa **22 cột** đặc tính (damage, element, elementvalue,
proofkind, growlevel, growexp, specialkind, stoneattr, stonelevel, enhancelevel, deletetime,
damageditemid, islocked, reinforced, affix1..3, stylelevel) nhưng **Rust chỉ ghi 4 cột**:
`INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage)` tại
`persist.rs:221-223` và `sqlite/session.rs:311-312`.
→ Mọi affix / enhance / lock / deletime của equipment **mất khi restart** (hoặc khi auto-save ghi đè).
Xem GAP-D3.

### 3.4 Đường ghi DB (write paths)

| Hàm | `path:line` | Transaction? | Guard tiền? | Ghi `bankgold`/`shoppoint`? |
|---|---|---|---|---|
| `persist::update_player(.., "Gold"/"BankGold"/"ShopPoint")` | `persist.rs:52-101` (money arm `81-101`) | ❌ **không** — `execute(&pool.write)` trực tiếp | ❌ | ✅ theo tên cột |
| `persist::update_player(.., cột khác)` | `persist.rs:102-114` | ❌ | ❌ | — |
| `persist::upsert_item` | `persist.rs:162-209` | ✅ `begin()` `172` | — | — |
| **`persist::persist_shop_transaction`** | `persist.rs:227-262` | ✅ `pool.write.begin()` `237` | ❌ **không có `WHERE gold >= …`**; chỉ `INSERT … ON CONFLICT DO UPDATE SET gold=excluded.gold` | ❌ chỉ `gold` |
| `persist::replace_inventory_tx` | `persist.rs:280-298` | (nằm trong tx gọi) | — | — |
| `persist::update_money_tx` | `persist.rs:300-322` | (nằm trong tx gọi) | ❌ | ✅ nhận `"gold"/"bank_gold"/"shop_point"` `306-311` |
| `persist::persist_shop_point_and_item` | `persist.rs:324-354` | ✅ `331` | ❌ (kiểm ở handler `system.rs:116`) | ✅ `shoppoint` |
| `persist::persist_sessions_transaction` | `persist.rs:264-278` | ❌ **vòng lặp tuần tự**, mỗi session một tx riêng (`sqlite/session.rs:283`) | ❌ | ✅ cả 3 |
| `SqliteSessionRepository::save` | `sqlite/session.rs:278-…` | ✅ 1 tx/player | ❌ | ✅ `296-297` |
| `transactions::bank_transfer` | `db/modern/transactions.rs:33-65` | ✅ `38` | ✅ **có** `WHERE gold >= cost` / `bankgold >= cost` `47-50` + `rows_affected()==0 → InsufficientFunds` `58-60` | ✅ |
| `transactions::shop_buy` | `db/modern/transactions.rs:69-97` | ✅ `75` | ✅ `WHERE playerid = ? AND gold >= ?` `77-84` | ❌ chỉ `gold` |
| `transactions::p2p_trade` | `db/modern/transactions.rs:101-133` | ✅ `107` | — (chỉ item) | — |

> **Quan trọng**: `transactions::{bank_transfer, shop_buy, p2p_trade}` là 3 hàm transactional
> **có guard tiền đúng chuẩn** — nhưng **KHÔNG handler nào gọi chúng** (grep `src/` ngoài chính
> `transactions.rs` → 0 hit; chỉ `tests/db_repository_init_test.rs:1024` gọi). Đây là code chết.
> Xem GAP-P1.

### 3.5 Idempotency / crash-safety

- `persist_shop_transaction` là **read-modify-write toàn bộ túi**: `replace_inventory_tx`
  (`persist.rs:289`) `DELETE FROM inventories WHERE playerid=? AND storagetype=?` rồi re-INSERT
  từng dòng. Trong cùng tx → atomic ✅. Nhưng nếu tiến trình chết **giữa** `DELETE` và `INSERT`
  mà tx chưa commit → SQLite rollback ✅. OK.
- **Không có idempotency key / transaction log** cho giao dịch mua-bán. Client gửi lại frame
  `0x17` sub 33 sau timeout → mua 2 lần.
- `SqliteSessionRepository::save` cũng `DELETE FROM inventories WHERE playerid = ?` (**không** lọc
  `storagetype`) rồi re-insert cả 5 loại túi (`sqlite/session.rs:299-314`) → cũng atomic.
- **Hai đường ghi cạnh tranh**: handler ghi write-through (ví dụ `persist_shop_transaction` chỉ
  `storagetype=1`) trong khi auto-save ghi full-snapshot. Nếu auto-save chạy sau buy nhưng với
  session clone cũ → snapshot cũ **xoá** các dòng `storagetype != 1` vừa được bank/storage handler
  ghi? Thực tế `save()` re-insert từ RAM nên vẫn nhất quán với RAM — nhưng nếu RAM sai
  (storage handler chỉ sửa RAM rồi không persist) thì auto-save vẫn ghi đúng RAM. Xem §4.

### 3.6 Stall / shop catalogue — KHÔNG có bảng nào

- ❌ Không bảng `player_shop`, `stall`, `npc_shop`, `price_list`, `shop_items`, `stock`.
- Stall chỉ là RAM: `PlayerShopState` (`session.rs:121-127`) + `ShopItem` (`session.rs:112-117`),
  lưu trong `session.shop` (`session.rs:238`).
- `auto_save::fingerprint` (`auto_save.rs:38-120`) **không mix** `shop.*` → stall không được
  coi là dirty và không được persist. `SqliteSessionRepository::save` không đọc/ghi `session.shop`.
- ⇒ **Toàn bộ trạng thái stall mất khi restart server**, và cả khi restart server cũng mất luôn
  `online_sessions` registry nên không còn ai mua được.

---

## 4. Routing hiện tại

Nguồn: `src/server/dispatcher.rs:252-397` (`async fn handle`). Bảng đầy đủ cho các MainOp
liên quan kinh tế:

| MainOp | Hằng số (`protocol/mod.rs`) | Handler | Trạng thái | `file:line` |
|---|---|---|---|---|
| `0x0C` | `OP_RELOCATE` (`:90`) | `system::handle_teleport_confirm` | ✅ Implemented (**KHÔNG phải shop** — teleport confirm) | `dispatcher.rs:277`; handler `system.rs:177` |
| `0x0F` | `OP_PET` (`:98`) | `pet_actions::handle_pet_actions` | ✅ Implemented (release/store/mount/rename) | `dispatcher.rs:281` |
| `0x17` | `OP_ITEM` (`:114`) | `inventory::handle_inventory` → sub 30..33 → `shops::handle_player_shop` | 🟡 **Partial**: sub 30/31/32/33 có; ~108 sub khác của inventory xem `handlers/inventory.rs` | `dispatcher.rs:297`; shop `shops.rs:412` |
| `0x19` | `OP_TRADE` (`:120`) | `trade_storage::handle_trade` | 🟡 Partial (2-phase P2P item/pet/gold; theo comment `dispatcher.rs:300-304` aLogin dialect không dùng) | `dispatcher.rs:305` |
| `0x1A` | `OP_MONEY_SYNC` (`:124`) | `npc_event::handle_pc_talk` | 🟡 **Rerouted** — không phải money sync; mang talk selector (PC dialect) | `dispatcher.rs:293` |
| `0x1B` | `OP_NPC_SHOP` (`:130`) | `shops::handle_npc_shop` | 🟡 Partial (hardcoded price table; không có frame catalogue/list) | `dispatcher.rs:312`; handler `shops.rs:166` |
| `0x1D` | `OP_BANK` (`:134`) | `trade_storage::handle_bank_gold` | 🟡 Partial — dùng `GOLD_CAP`, **không persist write-through** (chỉ auto-save) | `dispatcher.rs:318`; handler `trade_storage.rs:707` |
| `0x1E` | `OP_STORAGE` (`:136`) | `trade_storage::handle_storage_transfer` | 🟡 Partial | `dispatcher.rs:321` |
| `0x23` | `OP_ACCOUNT` (`:148`) | `system::handle_account_mgmt` | 🟡 Partial — pass/delete/gift-code; **không** sendpoint/voucher dù comment `:147` có nhắc | `dispatcher.rs:342` |
| `0x2C` | `OP_REBORN_PET` (`:178`) | `skills::handle_pet_reborn` | ✅ Implemented (không phải economy-critical) | `dispatcher.rs:351` |
| `0x42` | `OP_ITEM_MALL` (`:242`) | `system::handle_gm_shop` | 🟡 Partial — **chỉ GM** (`gm_level > PLAYER_GM_LEVEL`), dùng `shop_point` | `dispatcher.rs:360`; handler `system.rs:84` |
| `0x48` | `OP_SLOT_MACHINE` (`:264`) | `unimplemented::handle` | ❌ **Missing** — chỉ log, không trả lời | `dispatcher.rs:391`; `handlers/unimplemented.rs:16-35` |
| `0x3A` | `OP_DICE_BIDAXIAO` (`:212`) | `unimplemented::handle` | ❌ **Missing** | `dispatcher.rs:380` |
| `0x4B` | `OP_SALE_ROOM` (`:272`) | — | ❌ **Missing** — rơi vào `_ =>` ở `dispatcher.rs:394` (không có tên trong danh sách 365-392, cũng không match arm nào) | `dispatcher.rs:394` |
| `0x3D` | `OP_LOTTO` (`:224`) | `unimplemented::handle` | ❌ Missing (list trong 383) | `dispatcher.rs:383` |
| `0x38` | `OP_RESERVED_38` (`:206`) | — | ❌ Missing (`_ =>`) | `dispatcher.rs:394` |
| `0x4C` | `OP_EXP_SLOT` (`:274`) | — | ❌ Missing (`_ =>`) | `dispatcher.rs:394` |
| `0x49`..`0x4E` | `OP_WORLD_BOSS`…`OP_ASTROLABE` (`:268-278`) | — | ❌ Missing (`_ =>`) | `dispatcher.rs:394` |

Ghi chú: `SERVER_MAIN_OPCODES` (`protocol/mod.rs:285-298`) liệt kê **73** opcode, nhưng
`handle()` chỉ có ~35 arm thật + 1 arm `unimplemented`. 38 opcode rơi vào `_ =>`
`dispatcher.rs:394` → `unimplemented::handle` → **log-only, zero response**
(`handlers/unimplemented.rs:16-35`). Đây là "hố đen" đối với client: mọi request không hiểu
sẽ treo UI cho tới khi timeout client-side.


## 5. Bảng GAP

Quy ước mức độ:
- **CAO** = mất tiền / mất item / crash / exploit money, hoặc chặn tính năng không chạy được.
- **TRUNG BÌNH** = sai số liệu, thiếu khả năng, hoặc frame sai khiến UI hỏng.
- **THẤP** = vệ sinh code / khả năng mở rộng / ghi chú.

### 5.1 GAP — Mô hình tiền tệ

| # | Mức | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G1 | **CAO** | Không có bảng **giá bán** (sell price) | `try_sell_range()` cộng `count` gold **1:1**, không bảng giá (`shops.rs:27-48`, cụ thể `shops.rs:42`) | Thêm `sell_price` lookup. `ItemDef.sell_price` **đã được load từ `Item.dat`** (`data/loaders/item.rs:35,229,293`) nhưng `Item` (`data/tables.rs:39-64`) **không giữ field này** → phải: (a) thêm `price`/`sell_price` vào `tables::Item` + `ItemDef::to_item()` (`loaders/item.rs:69-79`), hoặc (b) truy cận `GameData.item_defs` (`data/loader.rs:66`) trực tiếp trong handler | Frame bán hiện có: `020B` (`shops.rs:18-22`) + `1A04` (`shops.rs:11-13`). Bear dùng `ItemInfo.SellingPrice` (`TS_Server_Bear/TS_Server/DataTools/ItemInfo.cs:53`) |
| G2 | **CAO** | `GOLD_CAP` không phải invariant toàn cục | Định nghĩa `trade_system.rs:18`; chỉ áp ở `trade_system.rs:85,139` + `trade_storage.rs:724,728`. **Không áp** ở: NPC buy (`shops.rs:245`), NPC sell (`shops.rs:42`), battle reward (`battle/service.rs:291,297`), GM `/addgold` (`gm.rs:270`), Eve gold type 1 (`talk.rs:472`) | Centralize: `Session::credit_gold(±amount) -> Result<(), MoneyError>` kiểm `GOLD_CAP`, thay mọi `s.gold = s.gold.saturating_add(..)`. Sửa `shops.rs:366` dùng `GOLD_CAP` thay literal `9_999_999` | `1A04` cho mọi thay đổi; thêm frame lỗi "vượt trần" |
| G3 | **CAO** | Ép `as u32` không kiểm dấu khi load tiền | `sqlite/session.rs:67-69`: `row.get::<i64,_>("gold") as u32`. DB cho phép âm (không `CHECK`) → `-1` thành `4 294 967 295` | `u32::try_from(v).unwrap_or(GOLD_CAP)` hoặc `clamp_u32` như `clamp_u8/u16` (`sqlite/session.rs:559-564`). Thêm `CHECK (gold >= 0 AND gold <= 9999999)` vào `0001_init.sql:85-87` | `1A04` ngay khi login (đã có `spawn.rs:561-564`) sẽ đẩy giá trị sai lên UI |
| G4 | **CAO** | `shop_point` **không có frame S→C** ngoài GM shop | `Session.shop_point` (`session.rs:243`) chỉ push bằng `shop_points_frame()` → code `4202` (`system.rs:12-17`) từ `handle_gm_shop` (`system.rs:161,167`) — mà handler đó gate `gm_level > PLAYER_GM_LEVEL` (`system.rs:89`). Người chơi thường **không bao giờ** thấy số shop point | Cần frame push `shop_point` khi: login (cùng batch `1A04`), sau mọi giao dịch shop-point, và khi mở form shop-point | Đã có `4202` (`system.rs:16`). Cần xác nhận client đọc frame nào ngoài GM context `[suy đoán]` |
| G5 | TRUNG BÌNH | `shop_point` mặc định lệch giữa RAM và DB | `Session::default()` = **1000** (`session.rs:373`); `character_money.shoppoint DEFAULT 0` (`0001_init.sql:87`). Nếu `create_and_seed` không chạy → DB = 0, RAM = 1000 | Chọn 1 nguồn sự thật. `create_and_seed` đã insert `session.shop_point` (`sqlite/session.rs:514-519`) → nên **đặt default `Session.shop_point = 0`** hoặc set `DEFAULT 1000` trong SQL | `4202` sau login |
| G6 | TRUNG BÌNH | `bank_gold` không có frame push số dư định kỳ | Chỉ push khi **chuyển** tiền: `1A01`/`1A02` + amount (`trade_storage.rs:748-751`), và khi mở form bank: `1D09 00` / `1D04 <le32 bank_gold>` / `1D05` (`chat.rs:411-416`, `talk.rs:711-716`). Không có `1A0?` mang **số dư** bank | Thêm frame số dư bank tại login + sau mỗi lần đổi | `1D04` (`chat.rs:413`) là frame đã xác nhận mang số dư |
| G7 | THẤP | Không có loại tiền thứ 4 (`xu`/cash) | Grep `xu\b|cash|coin` trên `src/` → chỉ trúng `apply_gold` (`battle/service.rs:284`) và từ "golden" (`harness.rs`). **Không tồn tại** | Nếu client có UI "Xu" thì cần thêm field + cột. Chưa xác minh client có hỗ trợ → xem `06_currency_model.md` | — |

### 5.2 GAP — Bảng giá & catalogue

| # | Mức | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G8 | **CAO** | **Không có frame push catalogue shop NPC** | `handle_npc_shop` (`shops.rs:166`) chỉ phản hồi khi đã có `menu` trong payload. Client mở form shop → gửi 0x1B với menu → **server không gửi danh sách hàng**, chỉ trả toast khi mua | Thêm frame S→C liệt kê kệ hàng (item_id + price) mỗi lần NPC mở shop | ⚠️ **Frame chưa xác định** — phải đọc `01_shop_read_flow.md` / `01_npc_shop.md` |
| G9 | **CAO** | `get_npc_shop_price` là bảng hardcode 3-khoá, **không có giá theo item** | `shops.rs:51-160` — `match (idtalking, map_id, menu)`, 71 dòng, trả `(item_id, price)`. Giá **không** đến từ `Item.dat` | Bỏ hardcode, dùng `ItemDef.price` (`loaders/item.rs:34`) + `EveData.listNpcShopOnmap` cho danh sách NPC bán (Bear: `EveData.listNpcShopOnmap[mapid].FindAll(item.shopsID == shopid && indexId == index)`, `NpcShopsHandler.cs:21`). Rust **chưa** có `listNpcShopOnmap` | Giống G8 |
| G10 | **CAO** | `get_npc_shop_price` key sai/thiếu phạm vi | Key = `(idtalking: i32, map_id: u16, menu: u8)`. Bear dùng `mapid` + `shopid` + `indexId` (`NpcShopsHandler.cs:19-21`). Rust: `idtalking` lấy từ `conn.session.idtalking` (`shops.rs:174`) — là **NPC template id trong talk**, không phải shop id. Chỉ có **11** NPC/map combo | Nạp bảng NPC-shop từ data file; index theo `(mapid, shopid, index)`; hỗ trợ NPC chưa có trong bảng → trả "cửa hàng đang đóng" thay vì im lặng | `020B` (`shops.rs:18-22`) cho lỗi |
| G11 | **CAO** | NPC buy: `count` bị bỏ qua — luôn mua **1** | `shops.rs:240`: `from_template(data, item_id, 1)` — hardcode count=1. `count` đọc ở `shops.rs:173` (`payload[1]`) nhưng chỉ dùng cho **sell** (`shops.rs:180,182`). Bear: `uint num2 = count * value2.BuyingPrice` (`NpcShopsHandler.cs:24`) | `let n = clamp(count, 1, 255)`; `total = price * n` (kiểm overflow); `can_add_item` với `count=n` | `1A04` + `1706` (`shops.rs:598-608` là builder sẵn) |
| G12 | **CAO** | Giá `58800` không đồng nhất với `Session.gold` cap | `(16,12002,*)` → giá `58800` (`shops.rs:70-84`), `(15,12002,*)` → `19900` (`shops.rs:86-101`). Trong khi `GOLD_CAP = 9_999_999` (`trade_system.rs:18`) và giá shop khác là `5/10/15/20/50/115` | Kiểm tra đơn vị: `[suy đoán]` `58800` có thể là **shop point** hoặc giá **đơn vị khác** bị gộp nhầm vào bảng gold. Cần đối chiếu `Item.dat` `price` của `20023` | — |
| G13 | TRUNG BÌNH | NPC buy **không** dùng `ItemDef.price` | `shops.rs:238` lấy price từ bảng hardcode, không đọc `data.item_defs` | Dùng `ctx.data.item_defs.get(&item_id).price` (`data/loader.rs:66` + `loaders/item.rs:34`) | — |
| G14 | TRUNG BÌNH | NPC sell chỉ bán được id trong 2 dải cứng | `shops.rs:179-185`: `26001..=26455` (NPC `16005`) hoặc `27001..=27165` (NPC `16002`), cần `map_id > 10000`. Ngoài đó `sold = false` → rơi xuống nhánh buy | Bán bằng `sell_price` của item bất kỳ (`itemid` từ payload slot) thay vì quét dải id | Frame `020B` + `1A04` |
| G15 | TRUNG BÌNH | NPC sell chọn item bằng **quét dải id**, không theo slot client chỉ định | `shops.rs:31-45`: `for item_id in item_range` → tìm item đầu tiên có đủ count. Client gửi slot cụ thể nhưng server bỏ qua | Parse slot từ payload, bán đúng slot đó | — |

### 5.3 GAP — Stall / shop người chơi (0x17 sub 30..33)

| # | Mứl | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G16 | **CAO** | **Stall không được persist** | `PlayerShopState` (`session.rs:121-127`) chỉ trong RAM (`session.rs:238`). `auto_save::fingerprint` (`auto_save.rs:38-120`) **không mix** `shop.*`; `SqliteSessionRepository::save` (`sqlite/session.rs:278-…`) không đọc `session.shop`. Không có bảng nào trong `migrations/` | Thêm bảng `player_stall_listings(playerid, slot, itemid, quantity, price, shopname, image, opened_at)` + `player_stall_state(playerid, active, name, image)`. Hoặc chấp nhận thiết kế "stall tự hủy khi disconnect" và **ghi rõ** vào doc + đóng stall khi logout | `171F` mở (`shops.rs:483-492`), `1720` đóng (`shops.rs:500`) |
| G17 | **CAO** | (G1 đã biết) `171E` gửi `(slot, price4)` thay vì `(slot, item_id4)` | `shops.rs:465-466`: `items_hex.push_str(slot); items_hex.push_str(le32(price))` | Đổi thành `le32(item.id)` (đã có sẵn ở `listings[].item_id`, `shops.rs:461`) | `171E` (`shops.rs:482`) |
| G18 | **CAO** | (G2 đã biết) `1720` uid lệch 1 byte | `shops.rs:500`: `F44406001720{} + le32(id)`; client `FUN_00731388` đọc uid tại `payload[3..6]` | Cần 1 byte header/padding trước uid (hoặc đổi format `code`). Chưa rõ byte đó là gì → đọc `04_stall_vendor.md` | `1720` (`shops.rs:500`) |
| G19 | **CAO** | Seller session bị **clone + ghi lại registry** → race mất tiền | `shops.rs:536-540` clone seller từ `online_sessions()`; `shops.rs:564-567` `insert(seller_id, seller_session.clone())` — **ghi đè toàn bộ seller session** sau khi settle. Hai buyer đồng thời → cùng đọc clone cũ → cùng trừ hàng → **mất tiền lặp lại** (seller nhận tiền 2 lần, mất item 2 lần → lệch) | Giữ `Mutex<Session>` / dùng `Hub::with_session_write`. Hiện tại `battle/service.rs` đã có `try_write_session_with_retry` (`service.rs:290`) — dùng chung pattern | — |
| G20 | **CAO** | `open_shop_id` không ràng buộc vùng / vị trí | `shops.rs:515`: chỉ check `target != id && s.shop.active && !s.shop.items.is_empty()`. Không check **cùng bản đồ**, không check khoảng cách, không check `map_id`. `open_shop_id` reset khi nào? grep: chỉ set ở `shops.rs:515`, default 0 (`session.rs:370`) | Check `seller.map_id == buyer.map_id` + khoảng cách (pattern `PICKUP_RANGE` ở `handlers/inventory.rs:14`) | `020B` lỗi |
| G21 | TRUNG BÌNH | Không giới hạn số listing / giá | `shops.rs:436-468`: duyệt hết payload, `seen_slots` chống trùng slot ✅, nhưng **không giới hạn số lượng listing** và **không cap `price`** (`u32` từ client). Giá `u32::MAX` chặn bằng `price == 0` (`shops.rs:456`) nhưng không cap trần | Cap số listing (vd 12) + `price <= GOLD_CAP` | `171E` |
| G22 | TRUNG BÌNH | `complete_shop_buy` không kiểm `count` chống stack-split | `shops.rs:344`: `seller_item.count < count` → `NotEnoughStock` ✅. Nhưng `shops.rs:373` `seller.homdo[seller_pos].count -= count` dùng `u8` — nếu `count > seller_item.count` đã chặn ở `344` ✅. OK | (không cần) | — |
| G23 | TRUNG BÌNH | Không kiểm trùng `item_id` ở nhiều slot khi mua | `shops.rs:386-388`: `add_item(&mut buyer.homdo, copy)` — `add_item` merge theo id (`inventory.rs:46-63`) ✅ nhưng `can_add_item` đã check trước (`shops.rs:359`) ✅ | OK | — |
| G24 | TRUNG BÌNH | Buyer equip check dùng `loai` thô | `shops.rs:355`: `let equip = (1..=6).contains(&seller_item.loai)` — magic range, không đọc `equippos` từ data | Đọc `equippos` từ `ItemDef` | `1706` (`shops.rs:572`) |
| G25 | TRUNG BÌNH | Không gửi frame cho người **không phải người mua** khi shop của họ bị bán | `shops.rs:574-578` chỉ `hub.send_to(seller_id, gold_frame)`. Người đứng cạnh xem shop (`1721` đã push) không được cập nhật → giá hiển thị stale | Broadcast `1721` mới cho cả map sau mỗi giao dịch | `1721` (`shops.rs:269-293`) |

### 5.4 GAP — Persistence

| # | Mức | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G26 | **CAO** | **NPC shop buy/sell path KHÔNG dùng guard tiền ở tầng DB** | `persist_shop_transaction` (`persist.rs:227-262`) chỉ `INSERT … ON CONFLICT DO UPDATE SET gold=excluded.gold` (`persist.rs:245`) — **không** `WHERE gold >= price`. Handler check `session.gold >= price` (`shops.rs:239`) là **RAM-only** | Dùng `transactions::shop_buy` (`db/modern/transactions.rs:69-97`) — **đã có sẵn** `WHERE playerid=? AND gold>=?` + `rows_affected()==0 → InsufficientFunds` (`transactions.rs:77-87`) — nhưng chưa ai gọi | — |
| G27 | **CAO** | `transactions::{bank_transfer, shop_buy, p2p_trade}` là **code chết** | Grep `src/` ngoài `transactions.rs` → **0 hit**. Chỉ `tests/db_repository_init_test.rs:1024` gọi | Wire vào handler, xoá đường `persist_shop_transaction` song song | — |
| G28 | **CAO** | `persist_shop_transaction` trả `true` khi `pool == None` | `persist.rs:236`: `let Some(pool) = pool else { return true }` — coi như thành công. `shops.rs:197` `if !persisted { rollback }` → **không** rollback khi không có DB → RAM lệch DB | Đổi thành `false`, hoặc thêm biến `db_available` để caller biết chế độ | — |
| G29 | **CAO** | Stall **không** persist nhưng `save()` vẫn ghi `homdo` — seller restart mất hàng đã bán | Xem G16. Nếu chấp nhận "stall = tạm", phải **trả lại item về túi** khi disconnect, không để item treo trong listing không tồn tại | `1720` khi logout | `1720` |
| G30 | **CAO** | `update_player(…, "Gold")` **không transaction** | `persist.rs:92-96`: `execute(&pool.write)` không `begin()`. GM `/addgold` (`gm.rs:278`) dùng đường này | Bọc tx hoặc chuyển GM sang `persist_shop_transaction` | `2304` (`spawn.rs:325-330`) |
| G31 | **CAO** | `SqliteSessionRepository::save` chỉ chạy khi `db_character_id > 0` | `sqlite/session.rs:279-282`: `if character_id <= 0 { return Ok(()) }` — **`Ok(())` = coi như thành công**. `persist_sessions_transaction` (`persist.rs:271-277`) chỉ `Err` khi `save()` trả `Err` → auto-save **ghi ledger fingerprint thành công** dù không ghi gì (`auto_save.rs:154-157`) | `save()` trả `Err` hoặc cờ "skipped" để ledger không đánh dấu dirty | — |
| G32 | **CAO** | `persist_sessions_transaction` là **N tx tuần tự**, không atomic toàn cục | `persist.rs:271-277`: `for session in sessions { repo.save(session).await }` — mỗi vòng 1 tx (`sqlite/session.rs:283`). Trade settle 2 người (`trade_storage.rs:299-304`) → **tx A commit, tx B fail** = mất tiền 1 chiều | Tx bao ngoài cả N session, hoặc bù trừ khi tx 2 fail (compensating tx) | — |
| G33 | TRUNG BÌNH | `inventory` chỉ ghi 4/22 cột | `persist.rs:221-223` và `sqlite/session.rs:311-312`: chỉ `itemid, quantity, damage`. 18 cột affix/enhance/lock/deletime **mất khi auto-save ghi đè** | `ThingData` 35-byte đã có đủ (`protocol/codecs/thing_data.rs`); map hết cột | `1708`/`1706` |
| G34 | TRUNG BÌNH | Bank/storage `1D` **không** dùng `bank_transfer` | `trade_storage.rs:736-741` sửa RAM rồi `persist_sessions_transaction(pool, &[session], &[])` (`trade_storage.rs:742`) — `_tables` rỗng, `persist.rs:267` bỏ qua tham số `_tables` → `save()` ghi **full snapshot** (cả 5 túi + pets + skills). Mỗi lần gửi tiền ngân hàng = viết lại **toàn bộ** character | Dùng `bank_transfer` (1 UPDATE, có guard) | `1D01`/`1D02` + `1A01`/`1A02` |
| G35 | TRUNG BÌNH | NPC sell không trả item về DB nếu `try_sell_range` partial | `shops.rs:41-43`: `remove_homdo_item` có thể trả < count; `shops.rs:42` vẫn `+count` gold trước khi kiểm `removed == count` (`43`). Rollback ở `shops.rs:198` chỉ khi `persist` fail | Chỉ credit `removed`, không credit `count` | `1A04` |
| G36 | TRUNG BÌNH | `mails` table (gold + attachment) **không dùng** | `0001_init.sql:254-270` có `gold`, `attachmentitemid`, `attachmentcount`, `claimed`. Chỉ `DELETE FROM mails` trong `sqlite/characters.rs:65,192` (xóa char) — **không INSERT, không SELECT** | Nếu client hỗ trợ "thư kèm tiền" thì cần implement `claimed` idempotency (chống claim 2 lần) | — |
| G37 | TRUNG BÌNH | `item_code` (gift code) không persist item/điểm đúng cách | `db/item_code.rs` — `degrade` khi không có DB | Nếu gift code cho `gold`/`shop_point` thì cần `update_money_tx` | — |
| G38 | THẤP | Không có idempotency cho retry client | Không khóa sequence cho `0x17 sub 33` / `0x1B`. Client timeout + resend = mua 2 lần | Cấu trúc payload có seq/uid chưa? `[suy đoán]` |

### 5.5 GAP — Routing / handler thiếu

| # | Mứl | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G39 | **CAO** | `0x48` SlotMachine **missing** | `dispatcher.rs:391` → `unimplemented::handle` (`handlers/unimplemented.rs:16-35`) chỉ `tracing::log`, **không gửi frame** | Implement: reel 1..16, trả thưởng `[UID][ItemID][Count]`, broadcast. Đây là nguồn tiền/item | Xem `05_special_economy.md` |
| G40 | **CAO** | `0x4B` OP_SALE_ROOM (Kỳ Trân Các) **missing** | Hằng số có (`protocol/mod.rs:272`), không có match arm → `_ =>` `dispatcher.rs:394` | Đây là hệ thống **shop point** duy nhất theo comment `0001_init.sql:87` | Xem `05_special_economy.md` |
| G41 | **CAO** | `0x3A` DiceBĐX, `0x3D` Lotto, `0x3B` Domino **missing** | `dispatcher.rs:380-383` → `unimplemented` | Cổng cược bằng gold/shop point | — |
| G42 | TRUNG BÌNH | 38 opcode rơi vào `_ =>` **không log** tên | `dispatcher.rs:394`: `_ => unimplemented::handle(ctx)` — không có arm cho `0x38, 0x45..0x4E`. Client treo UI do không có response | Thêm arm liệt kê từng opcode + `tracing::info` tên từ `opcode_name()` (`protocol/mod.rs:309`) | — |
| G43 | TRUNG BÌNH | `0x23` OP_ACCOUNT: comment hứa **sendpoint/voucher** nhưng chưa có | `protocol/mod.rs:147` nói "sendpoint/voucher (Shared)"; `dispatcher.rs:342` → `system::handle_account_mgmt` | Implement sub nạp điểm (→ `shop_point`?) | — |
| G44 | TRUNG BÌNH | `0x1A` OP_MONEY_SYNC bị reroute sang talk | `dispatcher.rs:290-293` — comment giải thích PC dialect mang talk selector | Kiểm tra xem C→S 0x1A có bao giờ là "client xin đồng bộ tiền" không → nếu có cần arm money | `1A04` |
| G45 | THẤP | `0x22` GamePoints vs `shop_point` | `dispatcher.rs:336` `system::handle_game_points` — dùng `session.god` (`session.rs:198`), **không** dùng `shop_point` | Làm rõ 2 hệ thống điểm này có tách không | `2204`? |

### 5.6 GAP — Invariant & an toàn

| # | Mức | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| G46 | **CAO** | Bag-full khi mua NPC: **im lặng** | `shops.rs:241-243`: `if !can_add_item { return }` — **không gửi frame nào**, client treo | Gửi toast lỗi. Có sẵn mẫu `F44403001B0102` ("Inventory full") dùng ở `system.rs:125` | `1B01 02` (`system.rs:125`) |
| G47 | **CAO** | Không đủ gold khi mua NPC: **im lặng** | `shops.rs:239`: `if gold >= price { … }` — thiếu `else`, không gửi gì. Bear gửi announce (`NpcShopsHandler.cs:34`) | Thêm `else` → toast "Không đủ vàng" | `020B` (`shops.rs:18-22`) |
| G48 | **CAO** | Không đủ `shop_point`: **im lặng** | `system.rs:116-118`: `if shop_point < price { return; }` — comment tự thừa nhận *"under-funded buys are silently ignored"* | Gửi `4202` với số dư thật (client tự hiện) | `4202` (`system.rs:16`) |
| G49 | **CAO** | `price` trong `handle_gm_shop` là **u16** | `system.rs:114`: `u32::from(encoder::u16_le(payload[5], payload[6]))` — client gửi u32 sẽ bị đọc sai | Xác nhận width từ client; nếu u32 thì đọc `payload[5..9]` | — |
| G50 | TRUNG BÌNH | `persist_shop_transaction` không guard `i64` overflow | `persist.rs:245` bind `i64::from(buyer_gold)` — `u32` → `i64` không overflow. Nhưng `transactions::shop_buy` (`transactions.rs:80`) nhận `i64` price — nếu truyền `price as i64 * count` với `count=255, price=58800` → 14 994 000, an toàn. Với `price` từ client (stall) `u32::MAX * 255` → **overflow `i64`** nếu làm nhân trước khi clamp | `shops.rs:348-350` đã clamp đúng (`u64::from` + `.min(u32::MAX)`) ✅ — nhưng hãy giữ nguyên khi refactor | — |
| G51 | TRUNG BÌNH | `complete_shop_buy` **không rollback RAM** khi fail giữa chừng | `shops.rs:365-398` mutate dần; nếu `free_slot(...).unwrap()` (`shops.rs:380`) panic khi túi vừa đầy → panic trong handler | Đã có probe `can_add_item`/`free_slot` check (`358-362`) nên `unwrap` an toàn — nhưng thay bằng `ok_or(NoFreeSlot)` cho chắc | — |
| G52 | TRUNG BÌNH | Rollback NPC buy chỉ khôi phục **RAM**, DB đã commit? | `shops.rs:244-260`: `before = clone` → `gold -= price` → `add_homdo_item` → `persist_shop_transaction`. Nếu persist **trả true nhưng chỉ ghi `storagetype=1`** (`persist.rs:246`) còn `trangbi`/`tientrang` thay đổi trong RAM thì không persist — nhưng auto-save sẽ ghi sau | OK vì auto-save ghi full snapshot; nhưng **cửa sổ 180s** mất dữ liệu nếu crash | — |
| G53 | TRUNG BÌNH | `add_homdo_item` return giá trị bị **bỏ qua** | `system.rs:139` `conn.session.add_homdo_item(item);` — kết quả (danh sách slot bị ghi) bị bỏ; `shops.rs:213` `|| add_homdo_item(item).is_empty()` ✅ có check | `system.rs:139` phải check `is_empty()` | `1706` |
| G54 | TRUNG BÌNH | `STACK_CAP = 50` hardcode, không khớp `ThingData`/client | `inventory.rs:13` `STACK_CAP: u16 = 50`; `HOMDO_SLOTS = 25` (`inventory.rs:9`). Nhưng `character_quest_items` dùng stack ≤ 255 (`0001_init.sql:325-331` comment) | Xác nhận stack cap thật của client từ `01_shop_read_flow.md` | — |
| G55 | TRUNG BÌNH | `HOMDO_SLOTS = 25` cho **mọi** loại túi | `inventory.rs:9` dùng cho cả `homdo` lẫn `tientrang`/`luulang` (qua `add_storage_item`). Schema comment `0001_init.sql:99` nói "1..25 hoặc 1..50 tùy loại túi" | Tách `SLOTS` theo storagetype | `1E04`, `1766` |
| G56 | THẤP | Không có `CHECK` constraint nào trên `character_money` | `0001_init.sql:83-88` | Thêm `CHECK (gold >= 0)` etc. (nhưng phải theo ghi chú đầu file `:10-11`: **không** dùng `ALTER`, phải tạo file migration mới) | — |

### 5.7 GAP — Test coverage (chi tiết ở §7)

| # | Mứl | Hạng mục | Hiện trạng |
|---|---|---|---|
| G57 | **CAO** | **0 integration test nào** cho `handle_npc_shop` | Grep `complete_shop_buy\|player_shop_catalog_frame\|get_npc_shop_price\|try_sell_range` trong `tests/` → **0 hit** |
| G58 | **CAO** | **0 test** cho `handle_player_shop` (stall) | Như trên |
| G59 | TRUNG BÌNH | `gold_frame` chỉ test qua Eve gold action | `tests/checkpoint6_parity_test.rs:19,121,136,151`, `tests/npc_eve_e2e_test.rs:42,347,358,366` |
| G60 | TRUNG BÌNH | `GOLD_CAP` **không test** | Grep `GOLD_CAP` trong `tests/` → 0 hit |
| G61 | TRUNG BÌNH | `persist_shop_transaction` **không test** | Grep trong `tests/` → 0 hit (chỉ `transactions.rs` API được test ở `db_repository_init_test.rs:1021-1100`) |

### 5.8 GAP — Lỗi logic cụ thể (cross-reference từ §7)

| # | Mứl | Hạng mục | Hiện trạng | Cần làm | Phụ thuộc frame |
|---|---|---|---|---|---|
| **M1** | **CAO** | `ShopItem.count` **không bao giờ giảm** sau khi bán một phần | `complete_shop_buy` so `listing.count < count` (`shops.rs:344`) nhưng **không** `listing.count -= count` sau `shops.rs:373`. Listing chỉ bị xoá khi slot rỗng hẳn (`shops.rs:396-398`). ⇒ Người mua thứ 2 yêu cầu `count = listing.count` cũ sẽ nhận `NotEnoughStock` dù còn hàng | `listing.count = listing.count.saturating_sub(count)` sau khi trừ | `1721` (`shops.rs:269-293`) |
| **M2** | TRUNG BÌNH | `player_shop_catalog_frame` `idx` là `u8` `wrapping_add` | `shops.rs:271,289-290`: `let mut idx = 1u8; … idx = idx.wrapping_add(1)` — wrap về 0 sau 255 listing | Cap số listing < 255 (G21) hoặc đổi `idx` sang `u16` | `1721` |
| **M3** | TRUNG BÌNH | Header `1721` dựng bằng magic number | `shops.rs:270`: `let mut body = "0".repeat(34);` (17 byte 0), không có hằng giải thích layout | `const CATALOG_HEADER_ZERO_BYTES: usize = 17` + comment layout | `1721` |
| **M4** | TRUNG BÌNH | `handle_npc_shop` đọc `count` cho buy nhưng không dùng | `shops.rs:173` đọc `payload[1]`, chỉ truyền cho `try_sell_range` (`:180`, `:182`). Nhánh buy (`:238-264`) không dùng `count` | Dùng `n = clamp(count, 1, 255)` (G11) | — |


---

## 6. Serializer sẵn có để tái dùng

### 6.1 Tầng primitive (`src/protocol/encoder.rs`)

| Hàm | `path:line` | Emits |
|---|---|---|
| `le16(u16) -> String` | `encoder.rs:8-10` | 2 hex char LE |
| `le32(u32) -> String` | `encoder.rs:13-21` | 4 hex char LE |
| `u16_le(b0,b1)`, `u32_le(b0..b3)`, `u32_le_slice(&[u8])` | `encoder.rs:24-44` | **parse** từ payload thô |
| `hex(&[u8])`, `strhex(&[u8])`, `strhex_of(&str)` | `encoder.rs:47-49, 78-89` | hex uppercase |
| `bytes(&str) -> Option<Vec<u8>>` | `encoder.rs:55-67` | parse hex → bytes |
| `xor01(&[u8])` | `encoder.rs:70-72` | XOR `0xAD` (dùng ở tầng frame) |

### 6.2 Tầng frame

| Hàm | `path:line` | Emits |
|---|---|---|
| `protocol::frame(code, body)` | `protocol/mod.rs:411-414` | `F444` + `le16(len)` + code + body. **Điểm vào duy nhất** cho mọi frame |
| `PacketWriter` | `protocol/writer.rs:12-223` | Builder byte-oriented: `write_u8/u16_le/u32_le/i16_le/bool/viscii_pascal/viscii_fixed`, `write_thing_data`, `write_player_card`, `write_friend_extra`, `write_battle_role`, `write_eve_result`, `write_quest_task_entry`, `write_quest_dont_entry`, `build_frame()`, `build_wire()`. **Chưa dùng cho frame kinh tế** — handlers kinh tế vẫn dùng `frame()` + hex string |

### 6.3 Builder S→C dùng được ngay cho kinh tế

| Builder | `path:line` | Frame code | Payload | Dùng cho |
|---|---|---|---|---|
| `shops::gold_frame(gold)` | `shops.rs:11-13` | `1A04` | `le32(gold)` + `00000000` | ✅ **gold update** (đã dùng ở `shops.rs:202,262,568`, `talk.rs:473,482`, `spawn.rs:561-564`) |
| `trade_storage::gold_status_frame(gold)` | `trade_storage.rs:125` | `1A04` | giống hệt `gold_frame` | ✅ **gold update** (duplicate — nên hợp nhất) |
| `shops::red_message(msg)` | `shops.rs:18-22` | `020B` | `00000000` + VISCII msg | ✅ **error toast** (đã dùng cho mọi lỗi stall `shops.rs:201,233,518,530,542,561,581-589`) |
| `system::shop_points_frame(points)` | `system.rs:12-17` | `4202` | `le16(points as u16)` + `0100` | ⚠️ **shop point** — `as u16` **mất dữ liệu** nếu `shop_point > 65535` |
| `spawn::store_frame(point)` | `spawn.rs:325-330` | `2304` | `le32(point)` + 12 zero | ⚠️ Dùng để đẩy **gold** ở `system.rs:62` và `gm.rs:280` — code khác `1A04` |
| `shops::equip_item_frame(item)` | `shops.rs:598-609` | `1706` | `le16(id)` + count + `0000` + long + giatri + khang + `le32(texp)` | ✅ **buy result (equip)** |
| `handlers::inventory::item_added_frame(item)` | `handlers/inventory.rs:70-85` | `1706` | như trên + `doben` | ✅ **buy result (item)** |
| `trade_storage::storage_entry(item)` | `trade_storage.rs:52-64` | (dùng trong `1E04`, `1766`, `1708`) | slot + `le16(id)` + count + doben + long + giatri + khang + `le32(texp)` | ✅ **storage listing** |
| `trade_storage::normal_trade_add_wire(item)` | `trade_storage.rs:27-37` | (dùng trong `1706`) | id + count + `0000` + long + giatri + khang + `le32(texp)` | ✅ **trade item grant** |
| `trade_storage::trade_offer_frame(s)` | `trade_storage.rs:75-80` | `1903` | `le32(trade.gold)` + items | ✅ **trade offer** |
| `trade_storage::pet_offer_frame(gold, pet)` | `trade_storage.rs:82-…` | `190C` | `le32(gold)` + pet fields | ✅ **trade pet offer** |
| `trade_storage::gold_code` inline | `trade_storage.rs:746-751` | `1D01`/`1D02` + `1A01`/`1A02` | `le32(amount)` | ⚠️ **bank delta**, không phải balance |
| `chat.rs` bank form inline | `chat.rs:411-416`, `talk.rs:711-716` | `1D09 00` / `1D04` / `1D05` / `1D06` | `1D04` mang `le32(bank_gold)` | ✅ **bank balance** |
| `Session::dump_table(items, code)` | `session.rs:450-468` | tuỳ `code` | 12 byte/entry: slot1 + id2 + count1 + doben1 + long1 + giatri1 + khang1 + texp4 | ✅ **wire entry 12 byte** (dùng cho `1705` homdo `session.rs:426`, `170B` trangbi `:431`, `1E01` tientrang `:436`, `172F` tuideo `:441`, `1766` luulang `:446`) |
| `ResponseSender::send_bag_items` / `send_equipment_items` / `send_storage_items` | `response.rs:54-69` | `1705` / `170B` / `1E01` | gọi `dump_*` | ✅ push full túi sau giao dịch |
| `ResponseSender::raw` / `broadcast` | `response.rs:34-46` | bất kỳ | frame dựng sẵn | ✅ escape hatch |
| `dispatcher::build_stat_update(code, value)` | `handlers/stats.rs` (qua `response.rs:84`) | `0x19`/`0x1A`/… | `le32(value)` | ✅ point/skill push |

### 6.4 Còn THIẾU (phải viết mới)

| Cần | Vì sao | Gợi ý |
|---|---|---|
| **Shop catalogue push** | Không có builder nào emit danh sách kệ hàng NPC (G8) | Dựng từ `frame()` + `le16(item_id)` + `le32(price)` theo mẫu `player_shop_catalog_frame` (`shops.rs:269-293`) — nhưng phải xác nhận layout từ `01_shop_read_flow.md` |
| **Price list push riêng** | Nếu client cần bảng giá độc lập | Thiết kế mới hoàn toàn |
| **Buy-result frame chuẩn** | `1706` dùng cho cả equip lẫn item (`shops.rs:572` vs `system.rs:135`) — 2 layout khác nhau | Chuẩn hoá 1 builder + 1 flag |
| **Error toast phân loại** | `020B` dùng chung cho mọi lỗi; không có frame lỗi riêng cho "không đủ tiền"/"túi đầy" | `1B01 02` (`system.rs:125`) là mẫu inventory-full; thêm mẫu insufficient-funds |
| **`shop_point` push ngoài GM** | `shop_points_frame` chỉ gọi trong `handle_gm_shop` (G4) | Tái dùng `shop_points_frame` nhưng cần biết frame client đọc ngoài GM |
| **Bank balance push** | Chỉ có form-open `1D04` (`chat.rs:413`) | Tái dùng layout `1D04` cho push định kỳ |

---

## 7. Test cần viết

Quy tắc repo (`AGENTS.md` §"Hướng Dẫn Kiểm Thử"): **mọi test mới đặt trong `tests/`**,
**cấm** `#[cfg(test)]` trong `src/`. Test DB dùng in-memory `sqlite::memory:?cache=shared`
hoặc tempfile; **cấm** chạm `DB/ts_dream.db`.

Hiện trạng coverage kinh tế (grep toàn `tests/`):
- `gold_frame`: `tests/checkpoint6_parity_test.rs:19,121,136,151` + `tests/npc_eve_e2e_test.rs:42,347,358,366` — chỉ qua Eve gold action.
- `Money`/session load-save: `tests/db_repository_init_test.rs:158-159,291-292,387-388,434-435,751-752`.
- `transactions::{bank_transfer, shop_buy, p2p_trade}`: `tests/db_repository_init_test.rs:1021-1100` — **API có test nhưng production không dùng**.
- **`complete_shop_buy`, `player_shop_catalog_frame`, `get_npc_shop_price`, `try_sell_range`, `handle_npc_shop`, `handle_player_shop`, `GOLD_CAP`, `persist_shop_transaction`: 0 test.**

### 7.1 File đề xuất

#### `tests/economy_money_test.rs` (mới — không cần DB, thuần logic)

| # | Test | Assert |
|---|---|---|
| T1 | `test_gold_cap_constant_is_9_999_999` | `GOLD_CAP == 9_999_999` — khóa hằng số để refactor không đổi ngầm |
| T2 | `test_credit_gold_rejects_over_cap` | Sau khi thêm `Session::credit_gold`: `credit_gold(GOLD_CAP)` → `Err`, `credit_gold(GOLD_CAP - gold)` → `Ok`, `session.gold` không đổi khi reject |
| T3 | `test_debit_gold_rejects_insufficient` | `debit_gold(gold + 1)` → `Err(insufficient)`, `session.gold` **không đổi** (chống `saturating_sub` nuốt tiền) |
| T4 | `test_gold_frame_bytes_are_stable` | `gold_frame(0x1234)` == `"F4440A001A04" + le32(0x1234) + "00000000"`; len = `0x000A` |
| T5 | `test_shop_points_frame_truncates_above_u16` | `shop_points_frame(70_000)` — **document bug**: `le16` mất bit cao. Assert giá trị hiện tại + `// TODO` để fail loud khi sửa |
| T6 | `test_complete_shop_buy_caps_seller_gold` | seller gold = `GOLD_CAP - 5`, giá `1000` × count `2` → seller gold == `GOLD_CAP` (không vượt), buyer gold trừ đúng `2000` |
| T7 | `test_complete_shop_buy_rejects_zero_count` | `count == 0` → `Err(InvalidIndex)`, **cả 2 session clone không đổi 1 byte** |
| T8 | `test_complete_shop_buy_rejects_overspend` | `buyer.gold = 10`, `total = 2000` → `Err(NotEnoughGold{total:2000})`; seller gold **không đổi** |
| T9 | `test_complete_shop_buy_total_overflow_saturates` | `price = u32::MAX, count = 255` → `total == u32::MAX` (không wrap), `Err(NotEnoughGold)` |
| T10 | `test_complete_shop_buy_removes_listing_when_sold_out` | mua hết count của slot → `seller.shop.items` rỗng; mua một nửa → listing **còn** với `count` cũ (⚠️ xác minh: `shops.rs:344` so `listing.count` chứ không cập nhật → xem GAP-M1) |
| T11 | `test_complete_shop_buy_rejects_closed_shop` | `seller.shop.active = false` → `Err(ShopClosed)` |
| T12 | `test_try_sell_range_is_1to1_no_price_table` | item count `3`, `count = 2` → `session.gold += 2` — **test hiện trạng sai lệch**, kèm comment TODO(G1) để fail khi sửa giá |
| T13 | `test_try_sell_range_partial_removal_still_credits_full` | ⚠️ **document bug G35**: count yêu cầu vượt số có thật → `removed < count` nhưng `gold` vẫn `+count`. Assert hành vi hiện tại + TODO |
| T14 | `test_get_npc_shop_price_key_is_npc_map_menu` | `get_npc_shop_price(16, 12002, 0) == Some((20023, 58800))`; `get_npc_shop_price(999, 12002, 0) == None`; `get_npc_shop_price(16, 12002, 15) == None` (ngoài menu) |
| T15 | `test_get_npc_shop_price_unknown_npc_returns_none` | `(0, 0, 0)` → `None` → handler rơi vào **không gửi frame** (G47) — assert để chốt hành vi |
| T16 | `test_homdo_stack_cap_and_free_slot` | `add_item` 60 item cùng id → 2 stack (50 + 10), `free_slot` trả slot trống kế tiếp, `can_add_item` cho 60 → `false` |
| T17 | `test_add_homdo_item_returns_affected_slots` | `add_item` straddle → trả **cả** 2 slot (chống regression mất increment khi reload) |

#### `tests/economy_persist_test.rs` (mới — cần SQLite in-memory)

| # | Test | Assert |
|---|---|---|
| T18 | `test_persist_shop_transaction_is_atomic_on_failure` | Inject lỗi SQLx giữa `update_money_tx` và `replace_inventory_tx` → **cả gold lẫn inventories giữ nguyên** (rollback `persist.rs:258`) |
| T19 | `test_persist_shop_transaction_writes_only_homdo` | Sau buy 1 item: `SELECT * FROM inventories WHERE storagetype=2` **vẫn còn** (không bị `DELETE ... storagetype=1` ăn mất) |
| T20 | `test_persist_shop_transaction_none_pool_reports_failure` | ⚠️ **document bug G28**: `persist_shop_transaction(None, …)` trả `true` — assert hành vi hiện tại + TODO |
| T21 | `test_persist_shop_transaction_two_party_is_one_tx` | Mua từ stall: buyer gold giảm **và** seller gold tăng **và** cả 2 `inventories` đúng — hoặc **không cái nào** đổi |
| T22 | `test_update_player_gold_without_transaction` | ⚠️ document G30: `update_player(pool, id, "Gold", 500)` hoạt động nhưng không rollback test được → assert giá trị |
| T23 | `test_character_money_has_no_check_constraint` | ⚠️ document G3/G56: insert `gold = -1` **thành công** → chứng minh thiếu `CHECK`; kèm TODO thêm constraint |
| T24 | `test_load_negative_gold_wraps_u32` | ⚠️ document G3: `session.gold` load từ DB `gold = -1` → `4_294_967_295` — assert hành vi hiện tại, TODO sửa |
| T25 | `test_auto_save_persists_gold_bank_and_shoppoint` | Sửa 3 field → `run_cycle` → đọc lại `character_money` khớp `Session::default()` |
| T26 | `test_auto_save_fingerprint_changes_on_gold_mutation` | `fingerprint(s)` khác nhau khi `gold` ±1; **giống nhau** khi đổi `shop.items` (⚠️ document G16 stall không dirty) |
| T27 | `test_auto_save_skips_session_without_db_character_id` | `db_character_id = 0` → `save()` trả `Ok(())` → `run_cycle` đánh dấu ledger thành công dù **không ghi gì** (⚠️ document G31) |
| T28 | `test_save_wipes_only_written_inventory_columns` | ⚠️ document G33: insert hàng `inventories` có `affix1 = 7` → `save()` → `affix1 == 0` |
| T29 | `test_save_persists_all_five_storage_types` | items ở `homdo/tientrang/tuideo/trangbi/luulang` → sau `save()` + `load()` còn đủ (⚠️ note `trade_storage.rs:661-662` nói không có bảng — sai, schema có) |
| T30 | `test_transactions_shop_buy_guards_against_double_spend` | gold `1000`, giá `600` → OK (gold `400`); lần 2 cùng giá → `Err(InsufficientFunds)` + gold giữ nguyên `400` |
| T31 | `test_transactions_bank_transfer_overdraft_blocked_at_sql` | `bank_transfer(-2000)` khi `gold=1000` → `Err(InsufficientFunds)`; `UPDATE … WHERE gold >= cost` chặn ở tầng SQL |
| T32 | `test_stall_state_not_persisted` | ⚠️ document G16: mở stall (`shop.active = true`, 3 listing) → `save_all_dirty` + reload → `session.shop` mặc định |

#### `tests/economy_shop_flow_test.rs` (mới — end-to-end handler qua `test_ctx`)

Dùng pattern sẵn có của `tests/npc_eve_e2e_test.rs` (`dispatcher::test_ctx`, `dispatcher.rs:406-424`).

| # | Test | Assert |
|---|---|---|
| T33 | `test_npc_shop_buy_sends_red_toast_then_gold_frame` | `get_npc_shop_price` trả `(20023, 58800)`; session gold `60_000`; sub/menu hợp lệ → 2 frame: `020B` "mua hàng thành công" (`shops.rs:261`) rồi `1A04` (`shops.rs:262`); gold trong túi **-1** |
| T34 | `test_npc_shop_buy_insufficient_gold_is_silent` | ⚠️ **document G47**: gold `10`, giá `58800` → **`outgoing` rỗng**; gold giữ nguyên |
| T35 | `test_npc_shop_buy_bag_full_is_silent` | ⚠️ **document G46**: túi đầy 25 slot → **`outgoing` rỗng**; gold **không** bị trừ |
| T36 | `test_npc_shop_buy_ignores_count_field` | ⚠️ **document G11**: payload `count = 10` → vẫn chỉ nhận **1** item (`shops.rs:240`) |
| T37 | `test_npc_shop_buy_persists_via_transaction` | Với pool thật → sau handler, `character_money.gold` và `inventories` khớp RAM |
| T38 | `test_npc_shop_sell_credits_one_gold_per_item` | ⚠️ document G1: `idnpctalking = 16005`, `map > 10000`, túi có 3× item `26001`; `count = 2` → gold `+2` |
| T39 | `test_npc_shop_sell_out_of_range_id_is_noop` | item `id = 30000`, `idnpctalking = 16005` → `sold = false` → không phản hồi |
| T40 | `test_player_shop_open_persists_nothing` | ⚠️ document G16/G17: sub 30 với `(name, image, [(slot, price)])` → `session.shop` đúng trong RAM, **DB không có bảng nào để kiểm**; assert `171E` frame shape |
| T41 | `test_player_shop_171e_payload_uses_item_id_not_price` | ⚠️ **document G17**: parse frame `171E`, assert entry chứa `le32(item.id)` — sẽ FAIL cho tới khi fix, đó là chính xác mục đích |
| T42 | `test_player_shop_buy_notifies_seller_gold` | sub 32 (view shop) rồi sub 33 (buy) → seller nhận `1A04` qua `hub.send_to` (`shops.rs:576-577`); seller gold `+total` |
| T43 | `test_player_shop_buy_rejects_offline_seller` | `open_shop_id` trỏ tới id không có trong registry → `020B` "Người bán đang offline" (`shops.rs:542`) |
| T44 | `test_player_shop_close_broadcasts_1720` | sub 31 → `session.shop.items` rỗng + frame `1720` (`shops.rs:500`) |
| T45 | `test_player_shop_buy_does_not_check_map_distance` | ⚠️ **document G20**: buyer ở `map 1`, seller ở `map 2` → buy **thành công** (assert hành vi hiện tại + TODO) |
| T46 | `test_bank_gold_rejects_over_cap` | `bank_gold = GOLD_CAP`, sub 1 rút `10` → **không** gửi frame (`trade_storage.rs:732-733`), gold giữ nguyên |
| T47 | `test_bank_gold_zero_amount_is_noop` | `amount = 0` → return sớm (`trade_storage.rs:717-718`) |
| T48 | `test_bank_gold_emits_1d_and_1a_pair` | sub 1 rút `500` → 2 frame `1D02` + `1A01` (`trade_storage.rs:748-751`) |
| T49 | `test_gm_shop_denies_normal_player` | `gm_level = 0` → `sys_msg_frame("Ban khong co quyen GM.")` (`system.rs:91`) + 1 row `gm_audit_log` |
| T50 | `test_gm_shop_insufficient_points_is_silent` | ⚠️ **document G48**: `shop_point = 10`, `price = 50` → **không** frame nào (`system.rs:116-118`) |


---

## 8. Rủi ro / bẫy đã biết

### R1. Hai đường ghi DB có thể đè lẫn nhau (nguy hiểm nhất)

`persist_shop_transaction` (write-through, chỉ `storagetype=1` + `gold`) **và**
`auto_save::run_cycle` (full snapshot, mọi 5 túi + pets + skills + money) cùng ghi vào
cùng bảng. `run_cycle` clone session **dưới lock** (`auto_save.rs:137-148`) rồi mới ghi DB
(`auto_save.rs:153`) — giữa lúc clone và lúc commit, handler có thể đã sửa RAM. Nếu commit
xảy ra sau, giá trị cũ **đè** giá trị mới ⇒ **mất tiền / mất item**.
- Hiện tại chỉ an toàn **nếu** mọi mutation đều ghi write-through và auto-save chỉ ghi lại
  đúng RAM. Nhưng G26/G28 cho thấy `persist_shop_transaction` **trả `true` khi không có DB**,
  và G31 cho thấy `save()` **trả `Ok` khi `db_character_id <= 0`** → ledger đánh dấu "đã lưu"
  dù chưa lưu gì.
- Khuyến nghị: **mọi mutation kinh tế phải ghi write-through** và `save()` phải báo skip.

### R2. `persist_shop_transaction` ghi `DELETE storagetype=1` — nhưng tên hàm gợi ý ghi một slot

`replace_inventory_tx` (`persist.rs:289-293`) `DELETE FROM inventories WHERE playerid=? AND storagetype=?`
rồi re-insert **toàn bộ** `items` truyền vào. Ở `shops.rs:191,224,251,556` hàm được gọi với
`&conn.session.homdo` — nên đúng. Nhưng nếu ai đó sau này gọi với `trangbi` mà quên đổi
`storage` từ `1` (`persist.rs:246` hardcode `1`), túi trang bị sẽ bị **xoá sạch**.
→ Rủi ro latent, không phải bug hiện tại.

### R3. `online_sessions()` là `Mutex<HashMap<u32, Session>>` — snapshot-and-overwrite

`session.rs:517-521`. Mọi luồng cross-player (stall `shops.rs:508,536,564`; trade
`trade_storage.rs:263,277,310`) đều `.cloned()` ra, sửa, rồi `.insert()` lại toàn bộ `Session`.
→ **Lost update** chắc chắn khi 2 request đồng thời. `battle/service.rs:288-298` dùng
`try_write_session_with_retry` (2 path khác nhau!) nghĩa là đã **tồn tại** 2 cơ chế cạnh tranh
cùng sống trong 1 codebase. Xem G19.

### R4. `shop_points_frame` ép `as u16` — mất dữ liệu shop point

`system.rs:14`: `encoder::le16(points as u16)`. `Session.shop_point` là `u32`
(`session.rs:243`), default 1000 (`session.rs:373`). Nếu nạp 70 000 điểm → client hiện `4464`.
**Không có test nào** cho việc này (T5 là test mới phải viết).

### R5. `gold_frame` vs `store_frame` — hai frame mang "tiền"

`1A04` (`shops.rs:12`) mang **gold**. `2304` (`spawn.rs:329`) mang **god/store point**
(`spawn.rs:592-594`) — nhưng lại được dùng để đẩy **gold** ở `system.rs:62` và `gm.rs:280`.
→ Kiểm chứng client đọc frame nào ở đâu; hiện tại có **3** frame chứa số tiền
(`1A04`, `2304`, `1D04`) + `4202` mang shop point. Rủi ro lệch hiển thị.

### R6. `trade_storage.rs:661-662` — comment sai

> `// TienTrang / LuuLang have no backing tables (0001 usage audit) — their pouches persist in Session memory only, so just homdo here.`

Sai: `inventories.storagetype` **có** `2` (Tiền trang) và `16` (Lưu lãng) từ `0001_init.sql:93,98`,
và `SqliteSessionRepository::save` ghi cả hai (`sqlite/session.rs:305,308`). Thực tế tham số
`_tables` bị **bỏ qua hoàn toàn** (`persist.rs:267`), nên `save()` luôn ghi full snapshot
→ comment này đang **dẫn dắt sai** người đọc tin rằng storage không persist.

### R7. `GOLD_CAP` duplicate literal

`trade_system.rs:18` (hằng) vs `shops.rs:366` (literal `9_999_999`) vs `SystemInline`
comment `shops.rs:316` "capped at 9,999,999". Ba chỗ, một sự thật. Sửa `GOLD_CAP` → 2 chỗ
còn sai.

### R8. `unimplemented::handle` không trả lời → treo UI client

`handlers/unimplemented.rs:16-35`. Với 38 opcode rơi vào `_ =>` (`dispatcher.rs:394`) và các
opcode được liệt kê tường minh (`:365-392`), mỗi request client gửi lên sẽ **không nhận
gì**. Client TS có timeout riêng nhưng UI thường đứng ở trạng thái "đang tải" ⇒ người chơi
báo lỗi "bấm không ăn". Ưu tiên tối thiểu: trả `020B` "chưa hỗ trợ" cho mọi opcode chưa port
(G42).

### R9. `character_id_tx` tra `characters.playerid`, không phải `accounts.playerid`

`persist.rs:13-21`. Doc `persist.rs:4-5` nói `player_id` là shared PK 1:1, đúng với schema
hiện tại. Nhưng nếu ai đó tách PK về sau, mọi `persist_shop_transaction` sẽ **âm thầm no-op**
(`character_id_tx` trả `None` → `Ok(())` tại `persist.rs:313-314`) — hàm vẫn trả `true` cho
handler ⇒ RAM lệch DB, không có log lỗi. **Đây là failure mode im lặng.**

### R10. `Item` (tables.rs) đã mất `price` / `sell_price` khi chuyển từ `ItemDef`

`ItemDef` có `price` (`loaders/item.rs:34`) và `sell_price` (`:35`), được parse từ `Item.dat`
(`:229`). `ItemDef::to_item()` (`loaders/item.rs:69-79`) build `Item` **không copy** 2 field này,
và `tables::Item` (`tables.rs:39-64`) cũng không khai báo. ⇒ Dữ liệu giá **đã được load rồi vứt**.
Sửa G1/G13 chỉ cần thêm 2 `pub` + 2 dòng copy — rẻ, nhưng phải nhớ `to_item()` dùng
`self.id as i64` style, cần `self.price as i64`.

### R11. `db/pool.rs::migrate()` là no-op

`pool.rs:130-133`: `migrate()` in log và `Ok(())`. Schema do operator quản lý thủ công.
→ Thêm `CHECK` constraint (G56) **không** thể dùng `ALTER TABLE ADD COLUMN` (SQLite không
hỗ trợ `IF NOT EXISTS` cho `ALTER` — `0001_init.sql:10-11`) ⇒ phải tạo file migration mới
`migrations/0002_*.sql`, nhưng `migrate()` không chạy nó ⇒ phải sửa cả runner.

### R12. Test DB có thể đụng production

`AGENTS.md` cấm test ghi vào `DB/ts_dream.db`. Các test mới T18–T32 **phải** dùng
`sqlite::memory:?cache=shared` hoặc tempfile. Lưu ý `pool.rs:45` dùng
`.create_if_missing(true)` ⇒ URL sai sẽ **tạo file mới** chứ không fail — dễ test "pass" trên
DB rỗng. Test hiện có (`db_repository_init_test.rs`) đã theo đúng pattern này.

---

## Ghi chú cuối

- File này **chỉ** phân tích phía Rust. Mọi frame wire layout phía client (đặc biệt catalogue
  shop NPC, `171E`, `1720`, frame shop point) **phải** đối chiếu `01_npc_shop.md`,
  `01_shop_read_flow.md`, `04_stall_vendor.md`, `03_trade_and_stall.md`, `05_special_economy.md`,
  `06_currency_model.md` trước khi code.
- Các GAP có `[suy đoán]` cần xác minh client trước khi coi là chắc chắn: G4, G8, G9, G12,
  G17, G18, G38, G44.
- `transactions.rs` (3 hàm, có guard tiền đúng) là **tài nguyên đã có sẵn chưa dùng** —
  ưu tiên wire chúng vào handler sẽ đóng được cả G26, G27, G30, G33 (một phần).
