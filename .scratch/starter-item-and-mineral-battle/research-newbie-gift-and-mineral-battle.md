# Báo Cáo Nghiên Cứu Kỹ Thuật: Khởi Tạo Nhân Vật & Quà Tân Thủ (`newbie`, `31007`) + Cơ Chế Chiến Đấu Quái Khoáng & Trang Bị Cuốc

- **Ngày khảo sát**: 2026-11-16
- **Phạm vi**: Chỉ đọc mã nguồn Rust (`src/`), migration (`migrations/`), kiểm thử (`tests/`), tài liệu/ADR (`docs/`, `.scratch/`), dữ liệu (`Data/items.json`, `Data/Item.dat`, `Data/Npc.dat`) và kiểm tra read-only SQLite (`DB/ts_dream.db`, `DB/ts_dream_test.db`). Không chỉnh sửa mã nguồn hay dữ liệu.

---

## Tóm Tắt Điều Hành (Executive Summary)

1. **Vấn đề 1 — Khởi tạo nhân vật & Quà Tân Thủ (`characters.newbie`, item `31007`)**:
   - Bảng trong schema hiện hành (`migrations/0001_init.sql:42`) là **`characters`** (tên cũ `charactor` / `players` đã bị loại bỏ theo `docs/adr/0001-newbie-rename-tanthu.md`). Cột `newbie INTEGER DEFAULT 0` **đã tồn tại** trong migration và trong `DB/ts_dream_test.db` (lưu ý `DB/ts_dream.db` hiện là file rỗng `4096` bytes chưa chạy migration).
   - Nghiệp vụ tặng **`1x` Vé đoái hoán tâm (`31007`) khi Enter Game (`newbie == 0 -> 1`) hiện CHƯA được triển khai** ở bất kỳ nơi nào trong `src/`.
   - **Xung đột nghiệp vụ cần chốt trước khi code**: Cột `characters.newbie` hiện đang được sử dụng cho hai luồng khác là gift code đặc biệt `TSVN123/TSVN456` (`src/db/item_code.rs:91-120`, `src/server/handlers/system.rs:397-421`, đặt `newbie = 1`) và sách `46238` (`src/server/handlers/use_item/books.rs:191-196`, tăng `newbie += 1`). Nếu Quà Tân Thủ cũng kiểm tra và đặt `newbie = 1` lúc Enter Game, nhân vật mới sẽ không thể nhận `TSVN123/TSVN456` trừ khi tách cờ riêng (ví dụ dùng `character_bit_flags` hoặc cột/bitmask riêng).

2. **Vấn đề 2 — Quái Khoáng & Điều kiện Trang bị Cuốc**:
   - Trong `src/battle/`, **chưa có** logic nhận diện NPC khoáng, kiểm tra vũ khí Cuốc, hay force miss/immune cho quái khoáng.
   - Từ `Data/Npc.dat` (`6.659` NPC hợp lệ): toàn bộ `252` NPC thuộc dải **`25001..=25252`** (không có record `25253..=25300`) đều có **`kind == 16`**, `not_pet == 1` (`can_capture == false`), và đều là Khoáng/Thạch/Thủy Tinh/Quặng. Ngoài dải này **không có NPC nào có `kind == 16`**. Riêng **`22011` (`KhoángChìQuái`)** có `kind == 13`, `not_pet == 0` (`can_capture == true`, `lv = 1`, `hp = 25`), cùng nhóm với `22010` (`Thủy Tinh Quái`).
   - Từ `Data/items.json` và `Data/Item.dat`: 6 item Cuốc (`10001`, `10530..=10534`) đều có `kind = 39`, `fit_type = 3` (`equippos = 3`, Vũ khí). **Phát hiện bug nghiêm trọng ngăn mặc Cuốc**: `ItemDef::to_item()` (`src/data/loaders/item.rs:76`) đang gán `loai: self.kind as i64` thay vì `self.fit_type as i64`, trong khi `handle_equip` (`src/server/handlers/inventory.rs:329`) và `handle_pet_equip` (`:620`) chặn nếu `!(1..=6).contains(&loai)`. Với `kind = 39`, **người chơi và pet hiện không thể trang bị bất kỳ cây Cuốc nào** nếu chưa sửa tách `equip_pos` (`fit_type`) khỏi `kind`.
   - Pet (`typ == 4`) **có đầy đủ 6 slot trang bị riêng** trong `Session.trangbi` tại slot `pet.stt * 10 + 1..=6` (`src/server/character_sheet.rs:37-41`, `src/server/handlers/inventory.rs:596-729`); slot vũ khí của Pet `stt` là `pet.stt * 10 + 3`.

---

## Phần 1: Khởi Tạo Nhân Vật Mới Khi Enter Game & Nghiệp Vụ Quà Tân Thủ (`characters.newbie`, Item `31007`)

### 1.1 Luồng Tạo Nhân Vật Mới (`INSERT INTO characters`)

Trong codebase hiện tại không còn bảng `charactor`; toàn bộ schema SQLite dùng bảng `characters` (xác minh tại `migrations/0001_init.sql:42` và `docs/adr/0001-newbie-rename-tanthu.md`). Có 2 nơi thực hiện `INSERT INTO characters` trong `src/`:

1. **Luồng tạo nhân vật chính thức từ client (Opcode `0x09` Sub `1`)**:
   - Handler: `src/server/handlers/character.rs:94-279` (`handle_character` → `create_char_db`).
   - Repository: `src/db/modern/sqlite/session.rs:546-646` (`SqliteSessionRepository::create_and_seed`).
   - Toàn bộ khởi tạo chạy trong **một SQLite write transaction nguyên tử**:
     - `apply_to_session` (`src/server/handlers/character.rs:224-279`) tính và gán vào `Session`:
       - `level = 1`, `job = 0`, `reborn = 0`, `sex = data.sex` (`0` hoặc `1`), `hair = u16::from(payload[2])` (1 byte tại `payload[2]`, giữ nguyên gap tại `payload[3]`), `thuoctinh = data.thuoctinh` (`1..=4`, mặc định `1`), `color = encoder::hex(&payload[4..12])` (chỉ lưu RAM cho `player_appear`).
       - `hp = hp_max = get_hp_max(0, 0, 1, data.hpx)`, `sp = sp_max = get_sp_max(0, 0, 1, data.spx)`.
       - `int1..agi = data.int1..data.agi`, `point = 0`, `skill_point = 0`, `texp = 6`.
       - Vị trí khởi tạo: `map_id = 10817`, `map_x = 442`, `map_y = 758` (`character.rs:212-214, 254-256`).
       - `gold = 0`, `tiengtam = 1`, `god = 0`, `gocnhin = 0`, `pk = 0`, `tham_chien = 1`.
       - Túi đồ khởi tạo (`homdo`, `storagetype = 1`): **1 entry tại slot `1` là `32012 × 4`**.
       - Trang bị khởi tạo (`trangbi`, `storagetype = 8`): **1 entry tại slot `2` là `19737 × 1`** (`agi1 = 1, loai = 2`).
       - Pet mặc định: **không có** (`pets` rỗng).
       - Lưu ý từ `Session::default()` (`src/server/session.rs:457-492`): `hp_store = 10000`, `sp_store = 10000`, `shop_point = 1000`, `bank_gold = 0`, `newbie = 0`.
     - Câu lệnh SQL tại `src/db/modern/sqlite/session.rs:559-586` chèn:
       - `characters`: đầy đủ cột với `rebornstage=0`, `nextexp=0`, `freepoints=0`, `skillpoint=0`, `pk=0`, `jobtype=0`, **`newbie=0`**, `savemap_id/x/y=0`.
       - `character_money`: `(playerid, gold, bankgold, shoppoint)` = `(account_id, 0, 0, 1000)`.
       - `accounts`: ghi đè `pass1` / `pass2` nếu payload gửi lên không rỗng.
       - `inventories`: chèn toàn bộ item `id > 0` từ `session.containers()` (tức `32012 × 4` ở Bag slot 1 và `19737 × 1` ở Equip slot 2).

2. **Luồng tạo nhân vật tối giản của `CharacterRepository` (chủ yếu cho test/repository API)**:
   - `src/db/modern/sqlite/characters.rs:13-33` (`SqliteCharacterRepository::create`) và `:100-127` (`CharacterRepository::create`):
   - Chỉ `INSERT INTO characters (playerid, name, level, gender, hair, element, mapid, mapx, mapy)` + `INSERT INTO character_money (playerid)`. Các cột còn lại (bao gồm `newbie`) nhận giá trị `DEFAULT 0` từ schema SQLite.

### 1.2 Luồng Login & Enter Game (`0x01`, `0x03`, `0x09 01`, `0x25`)

Luồng vào thế giới của client `aLogin.exe` đi qua các điểm sau (định tuyến tại `src/server/dispatcher.rs:268-285`):

1. **Kết nối TCP (`src/web/server_control.rs:648`)**: Server gửi ngay `LOGIN_SCENE_GREETING` (`F444030001095A`).
2. **Opcode `0x01` (`OP_AUTH` → `login::handle_login`, `src/server/handlers/login.rs:32-96, 169-230`)**:
   - Kiểm tra `ID_PREFIX` và `version >= MIN_VERSION` (`186`).
   - Với `env.pool = Some(...)`, gọi `login_db`: xác thực `pass1`, kiểm tra `is_suspended`, cập nhật `touch_login`, gán `conn.session.authed = true`.
   - Gọi `repos.sessions().load(id, &mut conn.session)` (`src/db/modern/sqlite/session.rs:19-120`):
     - Nếu **chưa có nhân vật** (`Ok(false)`): gửi `LOGIN_CREATE_CHAR` (`F4440300010300`) và dừng. Sau đó client gửi `0x09` Sub `2` (check tên) và `0x09` Sub `1` (tạo nhân vật → nhận `CHAR_CREATE_SUCCESS` `F44402000901`).
     - Nếu **đã có nhân vật** (`Ok(true)`): gọi `conn.session.hydrate_templates(data)`, kiểm tra double-login qua `hub.login_register(conn.session.id, sender)`, rồi gọi ngay `send_logined_sequence(conn, out)` trong chính handler `0x01`.
3. **Opcode `0x03` Sub `1` (`OP_ENTER_GAME`) hoặc `0x09` Sub `1` payload rỗng (`character.rs:138-141` → `login::handle_enter_game`, `src/server/handlers/login.rs:99-165`)**:
   - Chặn sớm nếu `!conn.session.authed` (trả `ENTER_GAME_CREATE`) hoặc **`if conn.session.logined { return; }`** (`login.rs:110-112`).
   - Với nhân vật vừa tạo ở `0x09 01` (hoặc tài khoản chưa chạy `send_logined_sequence`): gọi `repos.sessions().load(id, &mut conn.session)`, `hydrate_templates(data)`, `hub.login_register(...)`, và **`send_logined_sequence(conn, out)`**.
4. **Danh sách gói tin trong `send_logined_sequence` (`login.rs:18-29` → `spawn::build_logined_sequence_session_ext`, `src/server/spawn.rs:551-678`)**:
   - Đặt `conn.session.recompute_stats()` và `conn.session.logined = true`.
   - Phát tuần tự 23 bước `Logined1`:
     1. `login_start()` (`1F 0A` end-talk + `14 21 00` talk marker)
     2. `player_appear` (`03`)
     3. `StatsPacketBuilder::from_session(s)` (`05 03`)
     4. `pet_summary` (`0F 08`, `0F 14`, trailer `0F 0A`/`0F 12`/`0F 13` — chỉ khi có pet)
     5. `OPCODE_PET_SUMMON` (`13 01` — nếu `active_pet_stt ∈ 1..=4`)
     6. `OPCODE_PK_WAR` (`21 02`)
     7. Inventory dumps: `s.dump_homdo()` (**`17 05`**), `clear_tientrang_frame()` (`1E 08`), `s.dump_tientrang()` (`1E 01`), `s.dump_tuideo()` (`17 2F`), `s.dump_luulang()` (`17 66` nếu có item)
     8. `s.dump_trangbi()` (`17 0B`)
     9. Tiền: `gold_frame` (`1A 04`), bank gold (`1D 04`), `point_frame` (`1A 03`)
     10. `server_name_frame(s.id, "TSDREAM")` (`14 0B`)
     11. `FRAME_LOGIN_TERMINATOR` (`05 04` + `0F 0A`), `FRAME_LOGIN_BATTLE_LITERAL` (`0B 0B ...`), `FRAME_LOGIN_CLOSE` (`1F 0F`)
     12. Time banner + `"Welcome to TS Dream"` qua `sys_msg_frame` (`02 01 02`)
     13. `s.dump_hotkeys()` (`28 01`)
     14. `store_frame` ×3 (`23 04` cho `god`, `hp_store`, `sp_store`)
     15. `quest_sync::sync_frames(s)` (`18 06` / `18 07`)
     16. Reborn 2 dragon balls & hero balls (`17 4D`, `17 4E` nếu `reborn >= 2`).
5. **Opcode `0x25` Sub `1` (`OP_LOGIN_COMPLETE` → `login::handle_login_complete`, `src/server/handlers/login.rs:233-312`)**:
   - Client gửi sau khi load xong tài nguyên map. Server đặt `in_world = true`, broadcast `player_appear` ra map, gửi `server_name_frame(my_id, "TS Online")`, đồng bộ các player khác đang `in_world` trên cùng map và các NPC trên map (`16 01`).
   - **Lưu ý về Registry `online_sessions` (`src/web/server_control.rs:726-743, 796-798`)**: Trong vòng lặp connection, `logined_id` chỉ được gán **sau** khi `dispatcher::dispatch` hoàn tất frame `0x01`/`0x03` (`server_control.rs:796-798`). Vì `logined_id == 0` trước khi `dispatch` frame login kết thúc, khối `if logined_id > 0 { lock_online_sessions().insert(...) }` (`:735-742`) **chưa chạy** trong frame login đó mà chạy ở frame kế tiếp (thường là `0x25`). Ngoài ra `login_register` (`:475`) chỉ đọc `lock_online_sessions().get(&player_id)` chứ không tự chèn vào `online_sessions`.

### 1.3 Schema Bảng `characters` & Hiện Trạng Cột `newbie`

- **Migration (`migrations/0001_init.sql:77`)**:
  ```sql
  newbie INTEGER DEFAULT 0, -- Đánh dấu tân thủ nhận quà đặc biệt (0 = Chưa nhận, 1 = Đã nhận)
  ```
- **Kiểm tra thực tế DB trên đĩa (read-only)**:
  - `DB/ts_dream.db` (`4096` bytes): `SELECT name FROM sqlite_master WHERE type='table'` trả `[]` (chưa migrate).
  - `DB/ts_dream_test.db`: `PRAGMA table_info(characters)` xác nhận cột index `34` là `('newbie', 'INTEGER', 0, '0', 0)` (nullable=0, default `'0'`).
- **Tất cả nơi đang đọc/ghi `newbie` trong Rust**:
  1. `Session.newbie: u32` (`src/server/session.rs:297`), mặc định `0` (`:459`).
  2. `SqliteSessionRepository::load` (`src/db/modern/sqlite/session.rs:30, 74`): `SELECT c.newbie ...`, gán `session.newbie = row.get::<i64, _>("newbie") as u32`.
  3. `SqliteSessionRepository::save_in_tx` (`src/db/modern/sqlite/session.rs:315, 322`): `UPDATE characters SET ..., newbie=?, ...`.
  4. `SqliteSessionRepository::create_and_seed` (`src/db/modern/sqlite/session.rs:563, 565`): ghi cứng `0`.
  5. `persist::character_column` (`src/db/persist.rs:42`): ánh xạ `"Newbie" | "newbie" => "newbie"`.
  6. `auto_save::session_fingerprint` (`src/server/auto_save.rs:67`): đưa `s.newbie` vào hash dirty-check.
  7. `db::item_code::redeem_special_gift` (`src/db/item_code.rs:84-131`) & `handlers::system::handle_account_mgmt` Sub `3` (`src/server/handlers/system.rs:396-435`):
     - Nếu `conn.session.newbie == 1` hoặc trong DB `SELECT newbie FROM characters` không thỏa (`None | Some(1)`), trả `AlreadyGifted`.
     - Khi hợp lệ, cập nhật `UPDATE characters SET newbie = 1 WHERE playerid = ?` và cấp 5 item `[46197, 20711, 19711, 23549, 11001]`.
  8. `handlers::use_item::books` (`src/server/handlers/use_item/books.rs:191-196`):
     - Khi dùng sách `46238`, tăng `ctx.conn.session.newbie += 1` và gọi `persist::update_player(ctx.pool, pid, "newbie", i64::from(ctx.conn.session.newbie)).await`.

### 1.4 Kiểm Chứng Dữ Liệu Item `31007` & Hiện Trạng Nghiệp Vụ

- **Dữ liệu `Data/items.json`**:
  ```json
  {
    "id": 31007,
    "name": "Vé đoái hoán tâm",
    "type": 22,
    "level": 0,
    "description": "Vật để đóai hóan ba đậu yêu",
    "iconNum": 2204,
    "largeIconNum": 2204,
    "equippos": 0,
    "elemType": 0,
    "elemVal": -100,
    "sellPrice": 0,
    "buyPrice": 0
  }
  ```
- **Dữ liệu `Data/Item.dat`** (đối chiếu qua layout `src/data/loaders/item.rs`):
  - `id = 31007`, `kind = 22`, `fit_type = 0` (không phải trang bị), `level = 0`, `open_used = 12`, `element = 0`, `element_value = -100`.
- **Giới hạn stack (`stack_max`)**: `ItemDef`, `Data/Item.dat` và `Data/items.json` không có trường `stack_max` theo từng item. Server `ts_dream` áp dụng hằng số chung cho túi `homdo` (`src/server/inventory.rs:9-13`): `HOMDO_SLOTS = 25` (slot `1..=25`) và `STACK_CAP = 50`.
- **Liên kết trong `src/` và `spec/`**: Quét toàn bộ `src/`, `tests/`, `docs/`, `spec/` cho thấy **0 tham chiếu** đến `31007`. Chưa có script, quest, shop, hay handler `use_item` nào xử lý `31007`, và **chưa có** nghiệp vụ tự động tặng `1x 31007` khi Enter Game.

### 1.5 Đánh Giá Rủi Ro & Phương Án Triển Khai Chuẩn Mực (Quà Tân Thủ)

1. **Rủi ro xung đột cột `characters.newbie` (CRITICAL)**:
   - Hiện `newbie == 1` đang khóa quyền nhận gift code `TSVN123/TSVN456` (`src/db/item_code.rs:98-101`).
   - Nếu Quà Tân Thủ dùng `characters.newbie == 0 -> 1` và giữ nguyên `redeem_special_gift`, mọi nhân vật mới vừa vào game sẽ tự động có `newbie = 1` và mất quyền nhận `TSVN123/TSVN456`.
   - **Khuyến nghị**: Xác nhận lại với owner xem Quà Tân Thủ dùng:
     - *Phương án A (giữ nguyên `characters.newbie` theo yêu cầu)*: chấp nhận `newbie = 1` dùng cho gói quà tân thủ `31007` (và sửa hoặc bỏ/tách `TSVN123/TSVN456` sang `character_bit_flags` nếu vẫn cần `TSVN123`).
     - *Phương án B (tách cờ hoàn toàn)*: giữ `characters.newbie` cho `TSVN123`/`46238`, còn quà tân thủ lúc tạo/enter game dùng bit flag riêng trong `character_bit_flags` (ví dụ `flagindex = 1` hoặc cấp trực tiếp trong `create_and_seed` + fallback bit flag cho nhân vật cũ).
   - Ngoài ra lưu ý bug nhỏ ở `src/db/item_code.rs:99`: `match newbie { None | Some(1) => AlreadyGifted }` — nếu sách `46238` tăng `newbie` lên `2`, `Some(2)` sẽ lọt qua guard `Some(1)`! Nếu dùng `newbie` làm cờ tân thủ, phải kiểm tra `newbie == 0` (chặn `!= 0` hoặc `> 0`) và câu SQL nguyên tử phải là `WHERE playerid = ? AND newbie = 0`.

2. **Vị trí hook & đồng bộ Packet Wire**:
   - Có 2 điểm vào thế giới đều gọi `send_logined_sequence(conn, out)`:
     - `login::login_db` (`src/server/handlers/login.rs:223`) khi tài khoản đã có nhân vật đăng nhập bằng `0x01`.
     - `login::handle_enter_game` (`src/server/handlers/login.rs:135`) khi tài khoản vừa tạo nhân vật xong qua `0x09 01` rồi gửi `0x03 01` hoặc `0x09 01` rỗng.
   - **Thời điểm cấp & thứ tự frame**:
     - Nếu cấp **TRƯỚC** `send_logined_sequence(conn, out)` (tốt nhất trích xuất thành một helper chung `grant_newbie_gift_if_eligible(conn, out, pool, data)` gọi ngay trước `send_logined_sequence` ở cả `login_db` và `handle_enter_game` — hoặc cấp ngay trong `send_logined_sequence` nếu truyền `pool` và `data` vào):
       1. `conn.session.homdo` đã chứa `1x 31007` **trước khi** `build_logined_sequence_session_ext` gọi `s.dump_homdo()` (`17 05`). Client nhận `17 05` sẽ thấy ngay `31007` ở đúng slot trong túi mà không bị sai lệch state.
       2. Sau các frame `Logined1` (hoặc sau `Welcome to TS Dream`), push thêm `sys_msg_frame("Ban nhan duoc qua tan thu: 1x Ve doai hoan tam!")` (kênh `02 01 02` theo `CODING_STANDARDS.md` §2.1). Nếu muốn phát animation item add `17 06` (`item_added_frame`) thay vì chỉ dựa vào `17 05`, cần gửi `17 06` **sau** `build_logined_sequence_session_ext` (khi client đã nhận `17 05` không chứa item mới, hoặc chỉ gửi `17 05` + `sys_msg_frame` để tránh nhân đôi slot trên client — vì `17 05` đã dump toàn bộ túi đồ). **Khuyến nghị an toàn nhất**: cấp trước `send_logined_sequence` để `17 05` mang dữ liệu chính xác từ đầu + nối `sys_msg_frame` thông báo sau `Logined1` (hoặc hook ở `0x25` `handle_login_complete` sau khi `logined == true`, lúc đó gửi `17 06` + `sys_msg_frame`, nhưng lưu ý nếu client chưa gửi `0x25` thì chưa nhận quà và phải chống re-trigger nếu client gửi lại `0x25`).
     - Để đồng bộ `online_sessions` (tránh race với AutoSave / các luồng khác): gọi `registry_update_if_online(&conn.session)` và lưu ý với `server_control.rs:735-742` rằng `logined_id` được gán sau `dispatch` frame login.

3. **DB Atomicity & Xử lý Edge Cases**:
   - **DB Atomic Conditional Update**: Khi `pool` có mặt (`Some(pool)`), chạy trong 1 SQLite write transaction (`pool.write.begin()`):
     1. Kiểm tra `can_add_item(&conn.session.homdo, &gift_item)` **trước** (nếu túi đầy `25/25` ô và không gộp được → **không** sửa `newbie`, không tặng, có thể gửi `sys_msg_frame` báo túi đầy).
     2. Thực thi:
        ```sql
        UPDATE characters SET newbie = 1 WHERE playerid = ? AND newbie = 0
        ```
        Kiểm tra `rows_affected() == 1`. Nếu `rows_affected() == 0` (đã nhận hoặc race condition từ kết nối khác/reconnect) → rollback và **không** thêm item vào RAM.
     3. Thêm item vào bản sao thử nghiệm hoặc tính slot đích, gọi `crate::db::persist::upsert_item_tx` / `upsert_inventory_slot(&mut *tx, ...)` cho **tất cả các slot bị ảnh hưởng** (`affected`), rồi `tx.commit().await?`.
     4. Chỉ sau khi `commit()` thành công mới mutate `conn.session.homdo` (thêm item) và `conn.session.newbie = 1` (theo đúng Two-Phase State Lock / DB-first commit trong `CODING_STANDARDS.md` §1.1).
   - **Double Enter Game / Reconnect**:
     - `handle_enter_game` đã chặn `if conn.session.logined { return; }`.
     - Trong cùng một phiên kết nối, `login_db` và `handle_enter_game` không bao giờ cùng chạy `send_logined_sequence` vì `login_db` đã đặt `logined = true` (`send_logined_sequence` tại `login.rs:20`).
     - Khi reconnect, `repos.sessions().load()` đọc `newbie = 1` từ DB, và câu lệnh `WHERE newbie = 0` đảm bảo idempotency tuyệt đối (chỉ cấp đúng 1 lần trong đời nhân vật).
   - **Test cases đề xuất** (đặt tại `tests/server_network/` theo `CODING_STANDARDS.md` §3.1):
     1. Positive: Tạo nhân vật mới (`0x09 01`) → Enter Game (`0x03 01`) → DB `characters.newbie == 1`, `inventories` có `31007` count `1`, frame `17 05` chứa `31007` và `02 01 02` thông báo.
     2. Reconnect / Idempotency: Đăng nhập lại (`0x01`) không nhận thêm `31007` (số lượng vẫn là `1`).
     3. Double enter game: Gửi `0x03 01` lần 2 sau khi đã `logined == true` → no-op, không phát quà lần 2.
     4. Negative (túi đầy): Nhân vật có `newbie = 0` nhưng `homdo` đầy 25 ô khác item `31007` → không cập nhật `newbie = 1`, túi không đổi, phát thông báo phù hợp.
     5. Collateral integrity (`CODING_STANDARDS.md` §3.2): Slot phụ cận (ví dụ slot 1 có `32012 × 4`) còn nguyên vẹn sau khi cấp `31007` vào slot 2.

---

## Phần 2: Cơ Chế Trận Chiến Với NPC Khoáng – Thạch – Thủy Tinh – Quặng & Điều Kiện Trang Bị Cuốc

### 2.1 Hiện Trạng Trong Battle Engine (`src/battle/`) & Cách Biểu Diễn Trang Bị

1. **Tình trạng hiện tại trong `src/battle/`**:
   - Đã rà soát toàn bộ `src/battle/` (`runner.rs`, `damage.rs`, `combat_formula.rs`, `construction.rs`, `engine.rs`, `manager.rs`, `service.rs`, `targeting.rs`, `packets.rs`, `dispel.rs`, `npc_world.rs`).
   - **Chưa có** bất kỳ logic nào kiểm tra vũ khí Cuốc, nhận diện NPC khoáng (`kind == 16`, `25001..=25300`, `22011`), hoặc xử lý trạng thái miss/immune khi đánh khoáng không cầm Cuốc.

2. **Biểu diễn Player & Pet trong trận và vị trí lưu trữ Vũ khí**:
   - Grid cell `WarInfo` (`src/battle/engine.rs:31-78`):
     - Player: `typ = 2`, `id = session.id as i64`, `id_char = 0` (`construction.rs:188-190`).
     - Pet: `typ = 4`, `id = pet.id as i64`, `id_npc_on_map = pet.stt as i64`, `id_char = owner_id` (`construction.rs:310-313`).
     - NPC địch: `typ = 3` (NPC thường/Eve) hoặc `typ = 7` (TeamDef), `id = npc.id` (`construction.rs:339-340`).
   - **Trang bị của Player và Pet trong `ts_dream`**:
     - `PetState` (`src/server/session.rs:102-132`) và bảng `character_pets` (`migrations/0001_init.sql:134-176`) **không** có cột `weapon_id` hay mảng trang bị riêng trên struct Pet.
     - Thay vào đó, **toàn bộ trang bị của cả Player và Pet đều nằm chung trong `Session.trangbi`** (`inventories` với `storagetype = 8`):
       - **Player**: `slot ∈ 1..=6` (`src/server/character_sheet.rs:30-32`, `src/server/session.rs:647-653`). Vũ khí là **`slot == 3`** (`fit_type = 3` trong `Item.dat`).
       - **Pet** (`stt ∈ 1..=4`): `slot = pet_stt * 10 + sub` với `sub ∈ 1..=6`, tức `11..=46` (`src/server/character_sheet.rs:37-41`, `src/server/handlers/inventory.rs:629`, `src/server/spawn.rs:494-501`). Vũ khí của Pet `stt` nằm tại **`slot == pet.stt * 10 + 3`** (tức `13`, `23`, `33`, `43`).
       - Như vậy, **Pet hoàn toàn có slot trang bị vũ khí riêng** (`handle_pet_equip` / `handle_pet_unequip` tại `src/server/handlers/inventory.rs:596-729`, và `0F 08` dump đủ 6 slot trang bị pet tại `src/server/spawn.rs:494-501`).
   - **BUG CHẶN TRANG BỊ CUỐC HIỆN TẠI (`ItemDef::to_item` dùng `kind` thay vì `fit_type`)**:
     - `src/data/loaders/item.rs:76` gán `loai: self.kind as i64`, trong khi `fit_type` (`chunk[48]`, `equippos` trong `items.json`) mới là vị trí mặc `1..=6`.
     - `handle_equip` (`src/server/handlers/inventory.rs:327-329`) và `handle_pet_equip` (`:619-621`) kiểm tra `if item.id == 0 || !(1..=6).contains(&loai) || ... { return; }`.
     - Vì mọi Cuốc (`10001`, `10530..=10534`) đều có `kind = 39` và `fit_type = 3`, hàm `to_item()` sinh ra `item.loai = 39`, khiến **cả Player lẫn Pet đều bị từ chối khi gửi lệnh mặc Cuốc (`0x17 Sub 11` / `0x17 Sub 17`)**! (Đây cũng là lý do `character.rs:276` phải hardcode `loai: 2` cho áo `19737` dù `19737` có `kind = 10, fit_type = 2`).
     - Để mặc được Cuốc, cần thêm `pub equip_pos: u8` (hoặc dùng `fit_type`) vào `Item` / `InventoryItem` (giữ `kind` cho `crafting/basic/` tại `src/crafting/basic/algorithm.rs:115` và `validator.rs:54`), và kiểm tra `(1..=6).contains(&equip_pos)` trong `handle_equip` / `handle_pet_equip`.

### 2.2 Xác Minh Dữ Liệu Primary Source (`Data/items.json`, `Data/Item.dat`, `Data/Npc.dat`)

#### A. Dữ liệu Vũ khí Cuốc (`Data/items.json` & `Data/Item.dat`)

| Item ID | Tên (`name`) | `type` / `kind` | `equippos` / `fit_type` | `level` | `atk` (`attr1`) | `elemType` / `elemVal` | `buyPrice` / `sellPrice` | Mô tả |
|---:|---|---:|---:|---:|---:|---:|---:|---|
| `10001` | `Cuốc` | `39` | `3` (Vũ khí) | `0` | `0` (`code 210`) | `0` / `-100` | `10` / `1` | Công cụ có công dụng rộng |
| `10530` | `Cuốc chân kim` | `39` | `3` (Vũ khí) | `90` | `12` (`code 210`) | `0` / `-100` | `0` / `20` | Cuốc vàng mà đại phú gia mới có thể làm ra |
| `10531` | `Cuốc tân truyền` | `39` | `3` (Vũ khí) | `95` | `14` (`code 210`) | `0` / `-100` | `0` / `21` | Cuốc mà Thiên tử dùng để cầu khẩn thượng thiên ban cho mùa bội thu |
| `10532` | `Cuốc hòa điền` | `39` | `3` (Vũ khí) | `100` | `16` (`code 210`) | `0` / `-100` | `0` / `22` | Cuốc dùng ngọc thạch hiếm có làm thành |
| `10533` | `Cuốc đá vũ hoa` | `39` | `3` (Vũ khí) | `105` | `18` (`code 210`) | `0` / `-100` | `0` / `23` | Cuốc đá dùng đá vũ hoa xanh sậm chế thành |
| `10534` | `Cuốc nham thạch đỏ` | `39` | `3` (Vũ khí) | `110` | `20` (`code 210`) | `5` (Tâm) / `1` | `0` / `24` | Cuốc đá dùng long hầu nham đỏ chế thành |

**Kết quả quét toàn bộ `Data/items.json` (`8.675` items) và `Data/Item.dat` (`8.373` records)**:
- **Không còn item "Cuốc" vũ khí nào khác** ngoài 6 ID trên (`10001`, `10530..=10534`), ngoại trừ các lưu ý sau:
  - `10497` (`Hoàng Cân Sừ Đầu`): `kind = 39`, `fit_type = 3`, `level = 85`, `atk = 10`, mô tả `"Dụng cụ quân Khăn Vàng cần để khai quật"`. (Lưu ý: `10499 Đao hộ pháp` và `10500 Đao Phục Thù` cũng có `kind = 39`, `fit_type = 3` dù là Đao; do đó **không** nên nhận diện Cuốc bằng `kind == 39` chung chung, mà nên dùng danh sách ID Cuốc chuẩn `[10001, 10530, 10531, 10532, 10533, 10534]`, có thể cân nhắc bổ sung `10497` nếu thiết kế gameplay cho phép).
  - `29335` (`Cuốc đào`): `type/kind = 14`, `equippos/fit_type = 0`, `level = 0`, mô tả `"Cuốc thích hợp dùng đào đất, nhưng dễ hư"` — là vật phẩm đạo cụ (`fit_type = 0`), **không thể trang bị** vào slot vũ khí.
  - `29243` (`Xẻng sắt`) và `29286` (`Xẻng`): `type/kind = 20`, `equippos/fit_type = 0` — đạo cụ không thể trang bị.

#### B. Dữ liệu NPC Khoáng (`Data/Npc.dat`)

Quét toàn bộ `6.659` NPC có `id != 0` trong `Data/Npc.dat` (record `92` bytes, `NpcDatLoader` tại `src/data/loaders/npc.rs:96-236`):

1. **Dải `25001..=25300`**:
   - Thực tế có **`252` NPC** với ID liên tục **`25001..=25252`** (các ID `25253..=25300` không tồn tại trong `Npc.dat`).
   - **Tất cả `252/252` NPC** này đều là khoáng/thạch/thủy tinh/quặng (gồm các tên như `Khoáng.N.Thạch`, `Khoáng Vàng`, `Thủy Tinh Đỏ`, `Thủy Tinh Tím`, `Khoáng Bạc`, `Khoáng Đồng`, `Khoáng Sắt`, `Khoáng Chì`, `Khoáng kẽm`, `Khoáng thiếc`, `Khoáng Đá`, `Khoáng thạch`, `Kim/Ngân/Ngọc/Thiết/Đồng/Thạch/Dương Khoáng`, `... Quảng`, `K.* đầu rồng`, `TT * đầu rồng`...). **Không có bất kỳ NPC nào không phải khoáng trong dải `25001..=25300`.**
   - Thuộc tính đồng nhất của cả `252` NPC:
     - **`kind == 16`** (`chunk[15]`): **100% mang `kind == 16`**.
     - `not_pet == 1` (`can_capture == false`).
     - `body_kind == 0`, `weapon_kind == 0`, `moral == 0`, `unk1 == 0`.
     - `level` từ `2` đến `145`; `hp_base` từ `17` đến `2413`; `element ∈ {1 (Địa), 3 (Hỏa), 4 (Phong)}`; `turn ∈ {0, 1}`; `skills` thuộc `{10001, 10006, 12001, 12006, 13001, 13010}`; `drops` trỏ tới các item quặng/khoáng (`37xxx`, `38xxx`, `44xxx`).
   - **Giá trị `kind == 16` là dấu hiệu phân loại trực tiếp duy nhất trong `Npc.dat`**: Quét toàn bộ `6.659` NPC ngoài `25001..=25300` cho thấy **0 NPC nào có `kind == 16`**.

2. **NPC `22011` (`KhoángChìQuái`) và các NPC tên tương tự ngoài `25001..=25300`**:
   - **`22011` (`KhoángChìQuái`)**: `kind = 13`, `not_pet = 0` (`can_capture = true`), `level = 1`, `hp_base = 25`, `sp_base = 65`, `intelligence = 30`, `atk = 1`, `def = 1`, `agi = 1`, `element = 0`, `skills = [0, 0, 0, 0]`, `drops = [0, 0, 0, 0, 0, 0]`.
   - **`22010` (`Thủy Tinh Quái`)**: tương đồng với `22011`, `kind = 13`, `not_pet = 0` (`can_capture = true`), `level = 3`, `hp_base = 31`, `element = 0`, `skills = [0, 0, 0, 0]`, `drops = [0..0]`.
   - `kind == 13` (`63` records) là nhóm Ba Đậu Yêu / mascot / quái sự kiện (`BaDoYau`, `Bánh Bao`, `Cải Cao Lệ`, `Người Rơm`, `Nhân Sâm`, `Thạch Thải` `22032`, ...), nên **không** thể dùng `kind == 13` để nhận diện khoáng.
   - Các NPC tên có chữ khoáng/thạch khác ngoài `25001..=25300`:
     - `23006` (`Khoáng Mạch`, `kind = 14`, `not_pet = 1`, `lv = 1`, `hp = 25`), `24006` (`Mạch khoángsản`, `kind = 15`, `not_pet = 1`, `lv = 8`, `hp = 51`) — thuộc nhóm bảng biểu/cảnh vật trên map (`kind = 14, 15`).
     - `35013` (`Khoáng Đá Quái`, `kind = 26`, `not_pet = 1`, `lv = 70`, `hp = 2352`), `35025` (`NhamthạchQuái`, `kind = 26`, `not_pet = 1`, `lv = 65`, `hp = 4783`), `20005` (`Loạn thạch đồi`, `kind = 11`, `not_pet = 1`).
     - Lưu ý: `22011` không xuất hiện ở bất kỳ đâu trong `src/` dưới dạng NPC (lần xuất hiện duy nhất của số `22011` trong `src/` là item ID `22011` trong `src/server/handlers/shops.rs:132`). Nếu muốn áp dụng cơ chế Cuốc cho `22011` (theo tên `KhoángChìQuái`), phải kiểm tra tường minh `npc.id == 22011` bên cạnh `npc.kind == 16`, và lưu ý `22011` có `not_pet == 0` (vốn là pet bắt được).

### 2.3 Wire Protocol Cho Trạng Thái Miss / Immune (`imutable`)

1. **Cấu trúc bản tin hiện tại trong `src/battle/packets.rs` và `src/battle/runner.rs`**:
   - Frame hành động `32 01` (`runner.rs:1231-1257`) đóng gói mỗi mục tiêu bằng `packets::skilling_int` (`10` bytes, `src/battle/packets.rs:556-577`):
     ```text
     [row: u8] [col: u8] [miss_attack: u8] [atk_def_lantranh: u8] [count: u8] [troi_hp_sp: u8] [damage: u16 LE] [buff_or_attack: u8]
     ```
   - Hằng số trong `src/battle/packets.rs:648-702`:
     - `miss_status`: `ATTACK = 1`, `MISS = 0` (`2` dùng cho `thunder` ở `runner.rs:1531, 1791`).
     - `attack_status`: `ATTACK = 0`, `DEF = 1`, `LANTRANH = 2`.
     - `troi_byte`: `MISS = 0x00`, `HP = 0x19`, `SP = 0x1A`, `TYPE3 = 0xDD`, `TYPE4 = 0xDE`, `TYPE15 = 0xDF`, `TYPE19 = 0xE1`.
   - Cách `runner.rs` phát khi đánh **Miss** hoặc **0 damage**:
     - Đánh vật lý/phép miss (`apply_physical` `runner.rs:1539-1583`, `apply_magic` `:1799-1832`): gửi `miss_attack = 0` (`miss_status::MISS`), `damage = 0`, `troi_hp_sp = 0x19` (`troi_byte::HP`), `buff_or_attack = 1`, `adl = 0` (hoặc `1` nếu mục tiêu thủ `17001`, `2` nếu có `13003` Lẩn tránh). Khi bị chặn bởi khiên phản/kết giới (`10010 | 10015 | 10031 | 13021`), `hit` vẫn bằng `1` (hoặc `2`), `damage = 0`, `adl = 1` (`DEF`). Khi bị `13003` né hoàn toàn, `hit = 0`, `damage = 0`, `adl = 2` (`LANTRANH`).
     - Kỹ năng khống chế/debuff miss hoặc chống chồng (`apply_status3` `:1868-1895`, `apply_buff15` `:2457-2484`): gửi `miss_attack = 0`, `troi_hp_sp = 0x00` (`troi_byte::MISS`), `damage = 0`, `buff_or_attack = 1`.
   - Frame cập nhật HP/SP `33 01` (`packets::set_hp` tại `packets.rs:127-129`, `write_hp` tại `runner.rs:2741-2754`): gửi 7 bytes `[row][col][0x19][value u32 LE]`. Khi `damage = 0`, `value` bằng `target.hp` không đổi.

2. **Bằng chứng phía Client `aLogin.exe` trong `spec/` và `.scratch/`**:
   - Trong `.scratch/client-pseudo-op-code/battle/03_skill_and_combo.md:378-395` (`FUN_00651e3c` và `FUN_00652454`), sub-block 10 byte của `32 01` được parse thành: `dstGrp`, `dstSlot`, byte `[+2]` (`miss_attack`), byte `[+3]` (`mode 0/1/2` gọi `unit.VMT+0x18(unit, mode)`), `n`, và `n × 4` byte `[troi, dmg u16 LE, buff]`.
   - **Tài liệu `spec/` và `.scratch/client-pseudo-op-code/` hiện CHƯA có bằng chứng giải mã cho một byte `immune`/`imutable` riêng biệt cho quái khoáng** (như hiệu ứng chữ "Miễn dịch" hay mã `miss_attack`/`troi` đặc thù khác với `0` / `0x00` / `0x19`). Nếu server phát `miss_status::MISS` (`0`), `damage = 0`, `troi_byte::HP` (`0x19`) (hoặc `troi_byte::MISS = 0x00` cho debuff) thì khớp hoàn toàn với đường Miss hiện có của engine; trường hợp muốn xác nhận `aLogin.exe` có phân biệt mã `imutable` riêng hay không cần kiểm chứng thêm qua capture runtime (Ticket 14) hoặc decompile sâu nhánh `FUN_00652454` + `unit.VMT+0x18`.

### 2.4 Đề Xuất Thiết Kế Giải Thuật & Kiến Trúc Tích Hợp (Battle Refactor V2)

#### 1. Nhận diện mục tiêu Khoáng (Per-Target)
- Tạo helper thuần tại `src/battle/runner.rs` (hoặc `src/battle/combat_formula.rs`):
  ```rust
  pub fn is_mineral_target(data: &BattleData<'_>, target: &WarInfo) -> bool {
      if !matches!(target.typ, 3 | 7) {
          return false;
      }
      data.npcs
          .get(&target.id)
          .is_some_and(|npc| npc.kind == 16)
      // Lưu ý: nếu game design muốn gộp cả 22011 (KhoángChìQuái, kind == 13),
      // bổ sung tường minh `|| npc.id == 22011`.
  }
  ```
- **Vì sao kiểm tra per-target (không theo team)**: Một trận đấu (đặc biệt trong Eve `FightData` hoặc `TeamDef`) có thể xếp lẫn lộn quái thường và quái khoáng trong đội hình địch (rows `0..=1`). Kiểm tra trên từng `target` bảo đảm quái thường vẫn nhận sát thương bình thường còn quái khoáng kiểm tra điều kiện Cuốc độc lập.

#### 2. Nạp & Kiểm tra Vũ khí Cuốc cho Player (`typ == 2`) và Pet (`typ == 4`)
- **Bước 0 (Điều kiện tiên quyết để mặc được Cuốc)**: Tách rõ `equip_pos` (`ItemDef.fit_type`, `1..=6`) khỏi `kind` (`ItemDef.kind`, `39` cho Cuốc) trong `Item` / `InventoryItem` / `handle_equip` / `handle_pet_equip`, để `10001` và `10530..=10534` (có `fit_type = 3`, `kind = 39`) có thể mặc vào slot `3` của Player hoặc slot `pet_stt * 10 + 3` của Pet.
- **Nạp `weapon_id` vào `WarInfo` (`src/battle/engine.rs` & `src/battle/construction.rs`)**:
  - Thêm `pub weapon_id: u16` (mặc định `0`) vào `WarInfo` (`engine.rs:31`). Lưu ý `WarInfo::packet_hex()` (`:89-106`) giữ nguyên 23 bytes, không thay đổi wire format `0B 05` / `0B FA`.
  - Trong `Battle::add_player` (`construction.rs:183`): đọc vũ khí từ `session.trangbi.iter().find(|i| i.id > 0 && i.slot == 3).map(|i| i.id).unwrap_or(0)`.
  - Trong `Battle::add_pet` (`construction.rs:297`): thêm hoặc tra cứu từ `trangbi: &[InventoryItem]` (từ các caller `load_leader_pets`, `load_member_pet`, hoặc lưu thêm `pet_weapons` nếu muốn giữ nguyên chữ ký public của `add_pet`) để lấy `slot == pet.stt * 10 + 3`.
  - Lưu ý: nếu làm theo mẫu Ticket 24 (không đổi chữ ký `add_player`/`add_pet` cũ nếu có nhiều test gọi trực tiếp), có thể đồng thời giữ `WarInfo.weapon_id` với default `0` **hoặc** bổ sung `pub unit_weapons: Option<&'a HashMap<(i64, i64), u16>>` (key `(owner_id, stt)` với player là `(id, 0)` và pet là `(owner_id, pet.stt)`) trên `BattleData` giống `.with_player_skills(...)` (`runner.rs:81, 234`). Tuy nhiên, gắn `weapon_id: u16` trực tiếp lên `WarInfo` (và set trong `Battle::add_player` / `load_leader_pets` / `load_member_pet`) gọn hơn, giúp `combat_formula` / `runner` đọc trực tiếp $O(1)$ không cần tra HashMap.
- **Hàm kiểm tra Cuốc**:
  ```rust
  pub const PICKAXE_IDS: [u16; 6] = [10001, 10530, 10531, 10532, 10533, 10534];
  pub fn is_pickaxe(item_id: u16) -> bool {
      PICKAXE_IDS.contains(&item_id)
  }
  ```
  - Kiểm tra trên **chính đấu sĩ đang hành động (`attacker`)**:
    - Nếu `attacker.typ == 2` (Player): kiểm tra `is_pickaxe(attacker.weapon_id)` (slot `3` của chính người chơi đó — không tính Cuốc của đồng đội hay của Pet khác).
    - Nếu `attacker.typ == 4` (Pet): kiểm tra `is_pickaxe(attacker.weapon_id)` (slot `pet.stt * 10 + 3` của chính Pet đó — không mượn Cuốc từ chủ tướng).
    - Nếu `matches!(attacker.typ, 3 | 7)` (NPC/TeamDef, ví dụ bị Hỗn loạn đánh nhầm khoáng): NPC không mang trang bị (`weapon_id == 0`) → miss vào khoáng (hoặc tùy quy tắc nếu NPC đánh đồng đội).

#### 3. Bao phủ Toàn bộ Kịch bản Chiến đấu trong `runner.rs`
1. **Đánh thường (`NormalAttack 10000`) & Kỹ năng vật lý (`skill_type == 1`) & Phép thuật (`skill_type == 2`)**:
   - Ngay đầu `apply_physical` (`runner.rs:1490`) và `apply_magic` (`:1753`), kiểm tra:
     `let mineral_blocked = is_mineral_target(data, target) && !is_pickaxe(attacker.weapon_id);`
   - Nếu `mineral_blocked`:
     - Bỏ qua sát thương & hiệu ứng: `*num36 = 0`, `hit = miss_status::MISS as i64` (`0`), không gán `inflicted_status`, không kích hoạt `ts.reflect`, không chạy nhánh debuff phụ `13007 | 13029` (`runner.rs:1593`), không cộng `exp` trong `note_npc_hit` (`runner.rs:1042, 1064` — chỉ gọi `note_npc_hit` khi `!mineral_blocked` hoặc `dmg > 0`).
     - Vẫn phát `packets::skilling_int(r, c, miss_status::MISS, adl, 1, troi_byte::HP, 0, 1)` (trong đó `adl` giữ đúng trạng thái `DEF` nếu target đang thủ `17001`, `LANTRANH` nếu có `13003`, còn lại `ATTACK`).
   - Nếu `is_mineral_target(data, target) && is_pickaxe(attacker.weapon_id)`:
     - Thực thi tính toán sát thương/hit bình thường qua `combat_formula::calculate_attack(...)`.
2. **Kỹ năng AoE (đánh lan trúng cả quái thường và quái khoáng)**:
   - `execute_entity` (`runner.rs:972-1202`) duyệt từng `tpos` trong `targets` và gọi `apply_physical` / `apply_magic` / `apply_status3` / `apply_buff15` cho từng mục tiêu.
   - Do gate `mineral_blocked` đặt ở mức **per-target** bên trong các hàm `apply_*`, khi dùng AoE (ví dụ Ném đá, Hỏa tiễn, Loạn kích, Hồng thủy...) trúng 2 quái thường + 1 quái khoáng:
     - 2 quái thường nhận sát thương như bình thường.
     - Quái khoáng nhận `MISS` (`dmg = 0`) nếu attacker không cầm Cuốc, hoặc nhận sát thương nếu attacker cầm Cuốc — tất cả đóng gói chung vào một frame `32 01` chuẩn wire.
3. **Combo (cùng mục tiêu, người cầm Cuốc và người không cầm Cuốc)**:
   - Cơ chế Combo V2 hiện tại (`runner.rs:946-966, 1246-1280`): **từng thành viên** trong chuỗi combo vẫn tự chạy `execute_entity` và `apply_physical` của riêng mình để ghi vào `ts.text9` / `ts.text10` (chỉ trì hoãn flush `Out::Broadcast` khi `ts.combo_active == 1` để gom các block vào chung 1 frame `32 01`).
   - Do đó, khi Player A (có Cuốc) và Pet/Player B (không có Cuốc) cùng combo vào mục tiêu khoáng:
     - Lượt thực thi của A: `is_pickaxe(A.weapon_id) == true` → gây sát thương (nhân `×1.3` tại mục tiêu chính nếu `is_combo`).
     - Lượt thực thi của B: `is_pickaxe(B.weapon_id) == false` → `mineral_blocked == true` → phát block `MISS` (`dmg = 0`), **không** hưởng `×1.3` trên `0` và không được tính là đánh trúng khoáng.
   - *Lưu ý bổ sung cho `process_kills` (`runner.rs:2866-2916`)*: nếu mục tiêu khoáng chết trong chuỗi combo, thành viên B (không có Cuốc, đánh miss) vẫn nằm trong `ts.combo_cells` nếu không lọc — đây là hành vi chung của `detect_combo` hiện tại (tương tự khi B đánh miss do AGI/hit_rate), nên giữ nhất quán hoặc chỉ thêm `note_npc_hit` cho người thực sự gây damage.
4. **Kỹ năng trạng thái / Debuff / Hút SP / Holy Spirit (`skill_type == 3, 15, 19`)**:
   - Trong `apply_status3` (`runner.rs:1845`) và `apply_buff15` (`:2437`): nếu `is_mineral_target(data, target) && !is_pickaxe(attacker.weapon_id)` → trả về `skilling_int(r, c, miss_status::MISS, attack_status::ATTACK, 1, troi_byte::MISS, 0, 1)`, không trừ SP mục tiêu (`13015..=13018`), không hồi HP cho attacker, không gắn `type3_id`/`type15_id`, và không kích hoạt phản bùa `10026`.
   - Trong `apply_buff19` (`runner.rs:2579-2621`): nhánh `SKILL_HOLY_SPIRIT` gây sát thương phép lên mục tiêu; cần áp dụng cùng kiểm tra `mineral_blocked` trước khi trừ HP khoáng.
5. **Hộ chủ (`20006` — `apply_damage_to_rear` tại `runner.rs:1662-1749`)**:
   - Nếu mục tiêu bị chọn là khoáng ở hàng trước và có đơn vị Hộ chủ ở hàng sau (`rear`), sát thương chuyển vào `rear`. Kiểm tra `is_mineral_target(data, &rear)` (thay vì target ban đầu) trên thực thể thực sự nhận đòn.
6. **SP, Lượt đánh & Bắt thú (`Capture 15001`)**:
   - **Tiêu tốn SP & lượt**: SP đã trừ tại đầu `execute_entity` (`runner.rs:871-881`) và `attacker.attacked = true` ở cuối (`:1282`). Đánh miss vào khoáng vẫn tốn SP và mất lượt bình thường.
   - **Bắt khoáng (`Capture`)**: `apply_catch` (`runner.rs:2221`) đã có Gate 4 kiểm tra `npc.can_capture` (`not_pet == 0`). Toàn bộ `25001..=25252` (`kind == 16`) đều có `not_pet == 1` → luôn bị từ chối bắt. (Chỉ riêng `22011 KhoángChìQuái` có `not_pet == 0` là có thể bắt như pet thường).

---

## Phụ Lục: Danh Sách Nguồn Tham Chiếu Đã Kiểm Chứng

- `CODING_STANDARDS.md`
- `docs/adr/0001-newbie-rename-tanthu.md`
- `migrations/0001_init.sql`, `migrations/0002_eve_quest_persistence.sql`
- `src/db/item_code.rs`, `src/db/persist.rs`, `src/db/modern/sqlite/characters.rs`, `src/db/modern/sqlite/session.rs`, `src/db/modern/sqlite/inventories.rs`, `src/db/pool.rs`
- `src/server/handlers/login.rs`, `src/server/handlers/character.rs`, `src/server/handlers/system.rs`, `src/server/handlers/inventory.rs`, `src/server/handlers/battle.rs`, `src/server/handlers/use_item/books.rs`
- `src/server/session.rs`, `src/server/spawn.rs`, `src/server/inventory.rs`, `src/server/character_sheet.rs`, `src/server/dispatcher.rs`, `src/server/auto_save.rs`, `src/web/server_control.rs`
- `src/data/loaders/item.rs`, `src/data/loaders/npc.rs`, `src/data/tables.rs`, `src/data/loader.rs`, `src/encoding.rs`
- `src/battle/engine.rs`, `src/battle/construction.rs`, `src/battle/runner.rs`, `src/battle/combat_formula.rs`, `src/battle/damage.rs`, `src/battle/packets.rs`, `src/battle/targeting.rs`, `src/battle/service.rs`, `src/battle/manager.rs`
- `spec/exhaustive_by_opcode/opcode_32_full_payloads.md`, `spec/exhaustive_by_opcode/opcode_33_full_payloads.md`, `docs/client-opcodes.md`
- `.scratch/battle-refactor/handoff-battle-refactor.md`, `.scratch/client-pseudo-op-code/battle/02_battle_positions.md`, `.scratch/client-pseudo-op-code/battle/03_skill_and_combo.md`, `.scratch/client-pseudo-op-code/battle/05_rust_battle_audit.md`
- Dữ liệu đọc trực tiếp: `Data/items.json`, `Data/Item.dat`, `Data/Npc.dat`, `DB/ts_dream.db` (read-only), `DB/ts_dream_test.db` (read-only).
