# 06 — Create character (op 0x09 sub 1/2) + name check

**What to build:** Người chơi từ giao diện client đăng ký nhân vật mới — tạo xong dòng `players` + seed `SkillSave` + build bảng `Skill` + cập nhật `accounts` trong **một transaction atomic**, và check tên. Nền cho login (ticket 07).

**Blocked by:** 01 — Scaffold; 03 — Static data load (stat formula đọc từ data trong compute); 04 — DB schema (các bảng `players`/`SkillSave`/`Skill`/`accounts`).

**Status:** completed

- [x] Sub 1 (create): parse layout — `sex`, hair, `thuoctinh`, color(8B→hex), 6 stats (int atk def hpx spx agi), `pass1/pass2` (length-prefix). (Ch2 §2.3.7).
- [x] **Một transaction atomic** (`create_char_db`): INSERT `players` (stats computed qua `get_hp_max`/`get_sp_max`, reborn 0 / job 0 / lv 1; map khởi tạo 10817/442/758; explicit `player_id`) + INSERT `SkillSave` 1..10/IdSkill=0 (mandatory seed) + rebuild bảng `Skill` (DELETE stale — nhân vật mới chưa có skill) + `UPDATE accounts.pass1/pass2`. Reply `F44402000901`. Exception → `shutdown()` (Ch5 §5.6).
- [x] Sub 2 name-check: query `players.Name` (khớp qua `HEX(Name) = HEX(?)` để VISCII byte-safe) — tồn tại → `F4440300090301`; free → `F4440300090300` và nhớ candidate.
- [x] Mọi INSERT explicit `player_id` (schema shared — Ch5 §5.4).
- [x] Tạo qua giao diện client chạy được: handler trở thành `async`, lấy DB pool + client registry qua `OpcodeCtx.env`. Khi không có pool (golden replay) rơi về stub in-memory giữ **byte-exact**.

## Implementation notes

- Layout parse (`parse_create`): `[0] sex [2] hair [4..12] color(hex) [12] thuoctinh [13..19] int atk def hpx spx agi [19] pass1_len [20..20+len] pass1 [20+len+1..] pass2`. Tên nhân vật đến từ name-check sub2 (khớp C# `string_1`), không nằm trong packet create.
- Sau khi thành công `apply_to_session` phản ánh chỉ số vào session cho lần login/`Logined1` sau.
- Câu INSERT khớp từng cột C# `Update_H9` case 1 (nhánh MySQL tương đương `NewChar.accdb`).
- **Lưu ý:** đường DB chỉ được compile-check; cần runtime-test với MySQL thật (tạo nhân vật → login) trước khi tích hợp thực tế.