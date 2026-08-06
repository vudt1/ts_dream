# 04 — Schema + bootstrap MySQL 8 (ts_dream)

**What to build:** Database MySQL 8 `ts_dream` đầy đủ bảng và migration áp dụng được lúc boot, pool kết nối latin1, fail-fast. Nền tảng cho mọi thao tác lưu trữ người chơi.

**Blocked by:** 01 — Scaffold dự án + config + startup sequence (cần `database_url`/pool bootstrap); 02 — Protocol framing (đồng bộ byte với `players.Name`/`Pet.Name` VISCII).

**Status:** done ✅

- [x] Migration DDL đúng Ch5: 12 bảng — `players`, `accounts`, 9 gameplay (`homdo`, `tientrang`, `luulang`, `pet`, `quest`, `skill`, `skillsave`, `trangbi`, `tuideo`), `item_code`. Mỗi bảng **explicit** `CHARACTER SET latin1` (`COLLATE latin1_bin` khuyến nghị) trên cột text / table DEFAULT — không dựa vào server default utf8mb4.
- [x] PK composite `player_id` per-player: `(player_id, slot)` homdo/luulang/tientrang/trangbi/tuideo, `(player_id, stt)` pet, `(player_id, Id)` skill, `(player_id, ID)` skillsave; `quest` **không PK**, chỉ KEY `QuestId`. Index KEY như Ch5 §5.3.
- [x] Kiểu cột: mọi numeric Access `DOUBLE` → `BIGINT`; DEFAULT giữ nguyên (`ShopPoint 0`, `SP_Store/HP_Store 10000`, DTT/TLP/TCP/TTP/savemap/tanthu/phien/PTS 0); **không FK / no NOT NULL** ngoài Access; `accounts.id BIGINT AUTO_INCREMENT PRIMARY KEY`.
- [x] Pool: `MySqlPool` + `max_connections`, **connection charset latin1** để không transcode tên.
- [x] `sqlx::migrate!` chạy boot trước khi bind listener; MySQL unreachable / migration fail → hard exit rõ ràng (Ch5 §5.7).
- [x] `item_code` DDL theo Ch5 §5.5 (code lưu `VARCHAR(64) charset latin1`, player_id, used_at, item_id, count).

**Ghi chú scoping (Ch5 §5.4, bắt buộc khi port handler):** mọi SQL trên 9 bảng gameplay phải mang `player_id` ở predicate — các pattern nguy hiểm (SkillSaveGetId/UpdateId `WHERE Id=n`; DELETE+rồi build Skill lúc login theo Id range 10001..13033 / 0..9; họ `DELETE FROM Quest WHERE MapId`) đều phải cộng `player_id`. Ticket này chỉ đảm bảo DDL ready; việc áp predicate truyền xuống token handler.

---

## Cập nhật sau khi triển khai (review 08-2026)

### Quyết định đã chốt

- **Cột PK của `accounts` là `player_id`** (đồng nhất với code hiện tại: `login.rs`/`web/app.rs` đều truy vấn theo `player_id`). Chấp nhận lệch chữ so với spec §5.3/§5.8 ghi `id` — ghi nhận để không gây nhầm lẫn. `players.player_id` và `accounts.player_id` dùng chung giá trị = id tài khoản kiêm nhân vật.
- **`accounts` bắt đầu `AUTO_INCREMENT = 300000`** để id tài khoản mới nằm trên ngưỡng admin 300012 (§1.5) — bổ sung (không ghi trong spec), cần lưu ý.

### Tính năng mới / thay đổi

- **Tự tạo database**: `src/db/pool.rs::bootstrap()` tự `CREATE DATABASE IF NOT EXISTS ts_dream ... latin1` khi flag `db_auto_create` bật. Mặc định `true`; tắt bằng `TS_DB_AUTO_CREATE=false` hoặc `db_auto_create=false` trong `ts_dream.toml` (operator tự provision theo §8.3).
- **Repository layer tập trung** tại `src/db/`: `accounts.rs`, `players.rs`, `persist.rs`, `item_code.rs`, `pool.rs`. Đã di dời toàn bộ SQL khỏi handler (`login.rs`, `character.rs`, `system.rs`) và `web/app.rs`; handler chỉ gọi hàm repository typed. `persist.rs` gom mọi ghi-through players/skills/items/pets.
- **`item_code` redeem chạy DB** (Ch5 §5.5): `system.rs`/op 0x23 sub 3 dùng transaction + `FOR UPDATE` + rowcount==1 chống double-redeem; code/password là bind param. Không còn stub trong bộ nhớ.
- **README.md** đã cập nhật: bỏ mô tả SQLite lỗi thời, thêm bảng env `TS_*` (`TS_DATABASE_URL`, `TS_DB_AUTO_CREATE`, ...), sơ đồ cấu trúc theo repo hiện tại.

### Còn lại (quan sát, ngoài phạm vi DDL)

- Login chưa chạy lại bước "DELETE + rebuild `Skill` như C#" (§5.4 note 2) — hiện chỉ `SELECT skill`; `delete_reborn_skills` (rebirth) và transaction tạo nhân vật đều mang `player_id` đúng chuẩn. Rà soát khi triển khai handler tương ứng.
- Hạng mục hoàn thành: `cargo test --lib` (232 pass), `cargo check --all-targets` clean.