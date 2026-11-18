# 03: Đồng bộ Đăng nhập (`17 39` + Tối đa 1 Popup `24 19` theo `season_seq`) & Bộ `FestivalDatEncoder`

**What to build:** Từ góc nhìn người chơi đăng nhập vào game khi có lễ hội đang mở và người vận hành muốn mở rộng đủ bảng lore trên client, server tự động gửi thông báo mùa (`17 39`) khi đăng nhập và đẩy tối đa `1` hộp thoại *Bảng Lore Lễ hội* (`S→C 0x24 0x19`) chưa từng nhận trong đợt `season-<seq>` hiện tại; đồng thời cung cấp `FestivalDatEncoder` và công cụ CLI xuất file `Festival.Dat` đã cấp khóa phân biệt `1..=7` (và tùy chọn `8..=11`) để đồng bộ sang client mà giữ nguyên file gốc trong repo.

**Blocked by:** 02: Wire Parity `S→C 0x24 0x19` (`Festival.Dat` Gốc), `SeasonalNotice` (`0x17 0x39`), Pháo hoa (`0x17 0x46`) & Broadcast Trực tiếp

**Status:** completed

- [x] Thêm migration SQLite cho bảng `character_festival_claims` (`PRIMARY KEY (playerid, festival_id, period_key, claim_type)`) lưu vết nhận thưởng/hiển thị theo đợt (`season-<seq>`) và theo ngày (`YYYY-MM-DD`).
- [x] Trong luồng hoàn tất đăng nhập, phát `S→C 0x17 0x39` cho tất cả *Lễ hội Theo mùa* đang `Active` và chọn tối đa `1` lễ hội đủ điều kiện (có khóa `Festival.Dat` duy nhất hợp lệ và chưa có claim `lore_shown` cho `season-<seq>`) để gửi `S→C 0x24 0x19` rồi lưu vết vào `character_festival_claims`.
- [x] Xác minh hành vi đa lần đăng nhập & chuyển đợt (Positive + Negative Tests): relog nhiều lần trong cùng `season-<seq>` không bật lại popup `24 19`; khi nhiều lễ hội hợp lệ cùng `Active`, mỗi lần login chỉ gửi tối đa `1` khung `24 19`; khi Admin `Stop` rồi `Start` lại (`season_seq` tăng), người chơi đăng nhập sẽ nhận lại popup `24 19` một lần cho đợt mới.
- [x] Xây dựng `FestivalDatEncoder` (chuẩn `0x7EF` byte/record, mã hóa XOR hai khóa đối xứng `keyA`/`keyB`, tối đa `0x16E = 366` bản ghi) cùng CLI helper xuất file `Festival.Dat` patch khóa `1..=7` từ `Data/Festival.Dat` gốc và hỗ trợ cấu hình thêm bản ghi `8..=11` (*Thất Tịch*, *Trung Thu*, *Halloween*, *Giáng Sinh*), giữ nguyên file `Data/Festival.Dat` gốc trên đĩa.
- [x] Kiểm chứng round-trip `FestivalDatEncoder::encode` → `FestivalDatLoader::load` bảo toàn toàn bộ tiêu đề và hai đoạn văn bản VISCII 1.1 của `7` (và `11`) bản ghi với khóa duy nhất `1..=7` (và `1..=11`), không còn báo va chạm khóa.

## Comments

- **2026-11-18 — DeepCoder + Two-Axis Code Review (`Standards` & `Spec`)**:
  - Đã hoàn thành toàn bộ 5 mục của Ticket 03 trong working tree:
    - `migrations/0003_seasonal_festivals.sql`: tái sử dụng bảng `character_festival_claims` đã tạo ở Ticket 01; thêm `has_festival_claim`, `record_festival_claim`, `season_period_key`, `CLAIM_TYPE_LORE_SHOWN` trong `src/server/seasonal_festival.rs`.
    - `src/server/handlers/festival.rs` & `src/server/handlers/login.rs`: triển khai `emit_login_festival_sync` phát `17 39` cho mọi lễ hội `Active` và tối đa `1` khung `24 19` chưa claim theo `season-<seq>`.
    - `src/data/loaders/festival.rs`, `src/bin/patch_festival_dat.rs`, `Cargo.toml`: triển khai `FestivalDatEncoder` (`0x7EF` bytes/record, `0x16E = 366` max, dual-XOR) và CLI `patch_festival_dat` (từ chối ghi đè `Data/Festival.Dat`).
    - `tests/events_arena/festival_login_and_encoder.rs` + `tests/events_arena_test.rs`: 3 integration/round-trip tests pass.
