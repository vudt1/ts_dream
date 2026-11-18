# 01: Tách `SeasonalFestivalManager`, Persistence `season_seq` & Bảng Điều khiển Lễ hội trên Web Admin

**What to build:** Từ góc nhìn người vận hành trên Web Admin (Port 8090), tách phân hệ *Lễ hội Theo mùa (`SeasonalFestival`)* ra khỏi bảng *Sự kiện Đặc biệt (`SpecialEvent`)* để bảng sự kiện đặc biệt chỉ còn đúng `8` mục chịu watchdog 120 phút, còn `11` lễ hội theo mùa có card điều khiển riêng (`Seasonal Festivals Control Plane`) cho phép bật/tắt thủ công độc lập nhiều lễ hội cùng lúc, lưu bền vững trạng thái và bộ đếm đợt (`season_seq`) qua các lần khởi động lại server.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Gỡ bỏ dòng placeholder `seasonal_festival` khỏi `EventLifecycleManager`, giữ lại đúng `8` *Sự kiện Đặc biệt* (`north_star`, `continuous_40_npc`, `rescue_2k`, `nine_heavens`, `arena`, `guild_siege`, `water_war`, `dragon_boat`), và giữ `dragon_boat` vận hành độc lập hoàn toàn với `doan_ngo`.
- [x] Tạo `SeasonalFestivalManager` quản lý `11` *Lễ hội Theo mùa* (*Tết Nguyên Đán*, *Tết Nguyên Tiêu*, *Valentine*, *Ngày Cá Tháng Tư*, *Giỗ Tổ Hùng Vương*, *Lễ Phật Đản*, *Tết Đoan Ngọ*, *Lễ Thất Tịch*, *Tết Trung Thu*, *Halloween*, *Lễ Giáng Sinh*) nạp từ cấu hình JSON tùy biến kèm tập mặc định built-in; miễn trừ hoàn toàn khỏi watchdog 120 phút và cho phép nhiều lễ hội ở trạng thái `Active` đồng thời.
- [x] Thêm migration SQLite cho bảng `seasonal_festival_state` (lưu `festival_id`, `is_active`, `season_seq`, `started_at`, `updated_at`), tăng `season_seq += 1` chỉ khi chuyển `Idle -> Active`, giữ nguyên `season_seq` khi `Stop`, và hydrate lại trạng thái từ SQLite khi Game TCP Server khởi động.
- [x] Bổ sung card **Seasonal Festivals Control Plane** trên Web Admin Dashboard cùng các endpoint `/api/festivals` (`GET`), `/api/festivals/{id}/start` (`POST`), `/api/festivals/{id}/stop` (`POST`) trả về HTML fragment HTMX hoặc JSON tùy `HX-Request` và ghi đầy đủ vào `admin_audit_log`.
- [x] Áp dụng đầy đủ các kiểm tra từ chối (Negative Tests): từ chối HTTP `409 Conflict` khi Game TCP Server đang `STOPPED`, khi bấm `Start` trên lễ hội đã `Active`, khi bấm `Stop` trên lễ hội đã `Idle`, và `404 Not Found` với mã lễ hội không tồn tại; kiểm chứng test trên `DB/ts_dream_test.db` qua seam Web Admin Router + SQLite.

## Comments

- **2026-11-18 — DeepCoder + Two-Axis Code Review (`Standards` & `Spec`)**:
  - Đã hoàn thành toàn bộ 5 mục của Ticket 01 trong working tree:
    - `src/server/events.rs`: gỡ `EVENT_SEASONAL_FESTIVAL` / `"seasonal_festival"`, giữ đúng `8` sự kiện đặc biệt và giữ `dragon_boat` độc lập với `doan_ngo`.
    - `migrations/0003_seasonal_festivals.sql` + `tests/common/test_db.rs`: tạo bảng `seasonal_festival_state` và `character_festival_claims`.
    - `src/server/seasonal_festival.rs` + `src/lib.rs`: triển khai `SeasonalFestivalManager` (`11` lễ hội, JSON `Data/seasonal_festivals.json` + built-in `default_profiles()`, Two-Phase State Lock + rollback RAM nếu ghi SQLite lỗi, miễn trừ `check_watchdog()`).
    - `src/web/server_control.rs`, `src/web/app.rs`, `templates/dashboard.html`: hydrate trạng thái từ SQLite khi `ServerControl::start()`, bổ sung card `#seasonal-festivals-card` và routes `GET /api/festivals`, `POST /api/festivals/{id}/start`, `POST /api/festivals/{id}/stop`.
    - `tests/events_arena/seasonal_festival_control.rs` + `tests/events_arena_test.rs`: 5 integration/negative tests.
  - Kết quả review 2 trục: **0 lỗi chuẩn bắt buộc**, **0 sai lệch spec** (ghi chú nhỏ: có thể gom `start_festival`/`stop_festival` thành helper `transition_festival` và assert thêm JSON body `{"error": ...}` trong negative test khi chạm lại file ở Ticket 02).
