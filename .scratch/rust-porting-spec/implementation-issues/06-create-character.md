# 06 — Create character (op 0x09 sub 1/2) + name check

**What to build:** Người chơi từ giao diện client đăng ký nhân vật mới — tạo xong dòng `players` + seed `SkillSave` + build bảng `Skill` + cập nhật `accounts` + seed vật phẩm khởi tạo trong **một transaction atomic**, và check tên. Nền cho login (ticket 07).

**Blocked by:** 01 — Scaffold; 03 — Static data load (stat formula đọc từ data trong compute); 04 — DB schema (các bảng `players`/`SkillSave`/`Skill`/`accounts`).

**Status:** completed

- [x] Sub 1 (create): parse layout — `sex`, `hair` (1 byte), `thuoctinh`, `color`(8B→hex), 6 stats (int atk def hpx spx agi), `pass1/pass2` (length-prefix). (Ch2 §2.3.7).
- [x] **Một transaction atomic** (`create_char_db`): INSERT `players` (stats computed qua `get_hp_max`/`get_sp_max`, reborn 0 / job 0 / lv 1; map khởi tạo 10817/442/758; explicit `player_id`) + INSERT `SkillSave` 1..10/IdSkill=0 (mandatory seed) + rebuild bảng `Skill` (DELETE stale — nhân vật mới chưa có skill) + **seed starter `homdo`/`trangbi`** + `UPDATE accounts.pass1/pass2`. Reply `F44402000901`. Exception → `shutdown()` (Ch5 §5.6).
- [x] Sub 2 name-check: query `players.Name` (khớp qua `HEX(Name) = HEX(?)` để VISCII byte-safe) — tồn tại → `F4440300090301`; free → `F4440300090300` và nhớ candidate.
- [x] Mọi INSERT explicit `player_id` (schema shared — Ch5 §5.4).
- [x] Tạo qua giao diện client chạy được: handler trở thành `async`, lấy DB pool + client registry qua `OpcodeCtx.env`. Khi không có pool (golden replay) rơi về stub in-memory giữ **byte-exact**.

## Follow-up fixes (triển khai sau code review)

- [x] **Seed vật phẩm khởi tạo (P1):** transaction `db::players::create` giờ thêm INSERT `homdo` slot 1 = item `32012` × 4 và `trangbi` slot 2 = item `19737` × 1, `Agi1=1`, `Loai=2` — tương đương `NewChar.accdb`/`NewChar_init.sql`, scoped `player_id`, dữ liệu từ pure helper `db::players::starter_rows()`. Trước đây nhân vật mới có túi đồ/trang bị **rỗng** (lệch C#). Row `trangbi` slot 1 `Id=0` của template được bỏ (no-op).
- [x] `hair` parse đúng **1 byte (P2):** `parse_create` chỉ đọc `payload[2]` làm hair; byte `[3]` (C# packet[9]) là gap unused, không còn gộp thành u16.
- [x] `apply_to_session` phản ánh **đầy đủ** row `players` (P4): level=1, job/reborn=0, hp/sp/max tính qua formula, map 10817/442/758, Texp=6, tiengtam=1, tham_chien=1, + seed starter `homdo`/`trangbi` vào session — comment giờ khớp hành vi.
- [x] Sửa bảng layout trong `docs/rust_porting_spec.md` §2.3.7 + `research/01-protocol-reference.md` §2.7 (thuoctinh đặt SAU color, hair 1 byte, bỏ `name_len` trong packet).

## Layout chính xác (đối chiếu C# `Client.cs:1150-1205`)

`payload = decoded[6..]`: `[0] sex [1] unused [2] hair(1B) [3] unused [4..12] color(8B→hex) [12] thuoctinh [13..19] int atk def hpx spx agi [19] pass1_len [20..20+len] pass1 [20+len+1..] pass2`. Tên nhân vật từ name-check sub2 (khớp C# `string_1`), **không nằm trong packet create**.

## Implementation notes

- Layout parse (`parse_create`): `[2] hair(1B)` — byte `[3]` là gap unused; `[12] thuoctinh` đặt SAU 8 bytes color `[4..12]`; 6 stats `[13..19]`; `[19] pass1_len`; `[20..20+len] pass1`; `[20+len+1..] pass2`. Tên nhân vật đến từ name-check sub2, không nằm trong packet create.
- Sau khi thành công `apply_to_session` phản ánh chỉ số + starter items vào session cho lần login/`Logined1` sau.
- Câu INSERT khớp từng cột C# `Update_H9` case 1 (nhánh MySQL tương đương `NewChar.accdb`).
- **Lưu ý:** đường DB chỉ được compile-check; cần runtime-test với MySQL thật (tạo nhân vật → login) trước khi tích hợp thực tế.