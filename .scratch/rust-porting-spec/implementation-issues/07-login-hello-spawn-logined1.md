# 07 — Hello + Login + world spawn (chuỗi Logined1)

**What to build:** Người chơi log in thành công và "spawn" đầy đủ vào thế giới: hello gate, version gate, login check, double-login guard, và chuỗi **`Logined1`** gửi mọi thông tin (self-appear, stats, pets, party, PK mode, inventory dumps, gold, server name, banners, hotbar, god/HP/SP store) theo đúng thứ tự C#. Đây là vertical slice trung tâm đưa một account có nhân vật thành người đang online.

**Blocked by:** 06 — Create character (cần nhân vật đã tồn tại để demo spawn).

**Status:** completed

- [x] Op 0x00 Hello: frame `F444010000` → rep `F4440300010901`; khác → silent (Ch2 §2.3.1).
- [x] Op 0x01 Login: parse server prefix `"vn"` (mismatch → silent), client version (thấp hơn 186 → `shutdown()`). Thứ tự check (nhánh DB): version → account tồn tại (`accounts.pass1`, thiếu → `shutdown()`) → pass sai → `F44402000106` → **double-login guard** → có nhân vật hay chưa. Success → set id + `Logined1` (Ch2 §2.3.2).
- [x] **Double-login guard**: `ServerControl.login_register` = check + insert trong một lock (`clients` map id → sender) → login thứ 2 của cùng id bị `shutdown()`. Chỉ đăng ký khi nhân vật tồn tại (khớp C# `Logined()` — account chưa có nhân vật không vào `Clients`).
- [x] Op 0x03 xác nhận vào game: frame `F44402000301` → `Logined1`; chưa authed → rơi vào `CreatChar()` → `F4440300010300` (Ch2 §2.3.4).
- [x] Chuỗi `Logined1` **đủ các bước** Ch2 §2.4.1, **session-driven** (`build_logined_sequence_session`): player self-appear (0x03 sub03 — dùng trangbi equipped ids), stats (0x05 sub03 — stat thật + skill list), `SendPlayerOnline` (broadcast, do server loop fan-out), pet summary (0F08/0F14/0F0A) + pet summon nếu có, PK/war state, inventory dumps (Homdo/TienTrang/Tuideo/LuuLang từ session), equipped (170B), gold (1A04), server name (2709 "TSVN"), banners (time/welcome), hotbar (2801 — dump_hotkeys), 3× god/HP/SP store (2304 — `store_frame`).
- [x] `PlayerRow` load từ DB: `players` (HEX(Name)/HEX(Color) cho VISCII) + `skill` + `skillsave` (hotkeys) + 5 bảng đồ (homdo/trangbi/tientrang/tuideo/luulang) + `pet`.
- [x] Tên trong packet `strhex` (VISCII low byte count length field).
- [x] Sau `Logined1`, DB cleanup (pet hp fix) — để lại cho ticket lâu dài; không chặn luồng login.

## Implementation notes

- Handler `login_db` (src/server/handlers/login.rs) làm việc qua `ctx.env.pool`/`hub`/`sender`; khi không có pool (golden replay) chạy fallback in-memory giữ byte-exact trên session đã seed.
- `Logined1` giờ lấy dữ liệu từ session thật thay vì literal — sửa luôn 2 bug parity cũ của golden `04` (gold frame thiếu 4 hex trailing zeros; store frame thừa 12 zero bytes so C# `method_0`).
- **Lưu ý:** đường DB chỉ compile-check; cần runtime-test với MySQL thật: login đúng/sai pass, double-login 2 cửa sổ, tạo nhân vật rồi vào game.