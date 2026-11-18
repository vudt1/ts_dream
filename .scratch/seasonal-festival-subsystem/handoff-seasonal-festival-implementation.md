# HANDOFF: Phân Hệ Lễ Hội Theo Mùa (`SeasonalFestival`) — Tiến Độ Ticket 01–04 & Bàn Giao Ticket 05

* **Dự án:** `ts_dream` (Rust Game Server parity với Client TS Online `aLogin.exe`)
* **Thời điểm cập nhật:** 2026-11-18
* **Vị trí tài liệu:** `.scratch/seasonal-festival-subsystem/handoff-seasonal-festival-implementation.md`
* **Trạng thái hiện tại:**
  * **Tickets 01, 02, 03, 04 đã hoàn thành (`Status: completed`)**, đã qua **Code Review 2 trục (Standards & Spec)** và pass toàn bộ **677 tests** (`cargo test --all-targets --no-fail-fast`).
  * Thay đổi đang được giữ nguyên tại working tree (không commit theo quy tắc `AGENTS.md` để user tự commit thủ công).
  * **Điểm bắt đầu tiếp theo (Frontier duy nhất còn lại):** **Ticket 05** (`05-seasonal-battle-drops-antifarm-and-exp.md`).

---

## 1. Suggested Skills & Quy Tắc Bắt Buộc Cho Agent Tiếp Theo

1. **`tdd`**: Thực hiện theo vòng lặp Red → Green → Refactor theo vertical slice tại các seam công khai trong `tests/events_arena/` (đăng ký trong `tests/events_arena_test.rs`).
2. **`code-review`**: Sau khi hoàn thành Ticket 05, chạy review 2 trục **Standards** (`CODING_STANDARDS.md`, `GLOSSARY.md`, `ADR 0011`) và **Spec** (`spec.md` + `issues/05-*.md`).
3. **Ràng buộc kỹ thuật & công cụ (`AGENTS.md` + `CODING_STANDARDS.md`)**:
   * **KHÔNG** đặt `#[cfg(test)]` hoặc pub test helpers trong `src/` (`tests/architecture_guardrails_test.rs`).
   * **KHÔNG** gọi trực tiếp `.insert(...)` vào `lock_online_sessions()` trong handlers hoặc tests; dùng `registry_update_if_online(&Session)` / `registry_register_for_login(&Session)`.
   * **KHÔNG** tự động `git commit`, `cargo update`, hay `cargo clean`.
   * Trên Windows PowerShell không có `rg`; dùng `Select-String` và chỉ tìm trong `src/`, `tests/`, `docs/`, `spec/`, `templates/`, `migrations/`.
   * Mọi test SQLite phải dùng `setup_test_db().await` (`DB/ts_dream_test.db`).

---

## 2. Tóm Tắt Tiến Độ Đã Hoàn Thành (Tickets 01–04)

### Ticket 01 — `SeasonalFestivalManager`, `season_seq` & Web Admin Control Plane (`completed`)
* **`src/server/events.rs`**: Gỡ `EVENT_SEASONAL_FESTIVAL` / `"seasonal_festival"`, đưa bảng `SpecialEvent` về đúng `8` sự kiện (giữ `dragon_boat` độc lập với `doan_ngo`).
* **`migrations/0003_seasonal_festivals.sql` & `tests/common/test_db.rs`**: Tạo bảng `seasonal_festival_state` và `character_festival_claims`, nối `MIGRATION_0003` vào `reset_schema()`.
* **`src/server/seasonal_festival.rs` & `src/lib.rs`**: Triển khai `SeasonalFestivalManager` quản lý `11` lễ hội (JSON `Data/seasonal_festivals.json` + `default_profiles()`), miễn trừ watchdog 120 phút, multi-active, Two-Phase State Lock + rollback RAM, `hydrate_from_db`.
* **`src/web/server_control.rs`, `src/web/app.rs`, `templates/dashboard.html`**: Hydrate khi `ServerControl::start()`, card `#seasonal-festivals-card`, routes `GET /api/festivals`, `POST /api/festivals/{id}/start`, `POST /api/festivals/{id}/stop`, audit log.
* **Tests:** `tests/events_arena/seasonal_festival_control.rs` (5 tests).

### Ticket 02 — Wire Parity `24 19`, `SeasonalNotice` (`17 39`), Pháo hoa (`17 46`) & Broadcast (`completed`)
* **`src/data/loaders/festival.rs` & `src/data/loaders/mod.rs`**: Xóa hằng số sai `FESTIVAL_PARITY_DATE_DWORD` (`0x0121EAC6`), giữ nguyên `Data/Festival.Dat` gốc; thêm `derive_lookup_key` đọc `(double)u32` thực tế và phát hiện collision `unique_lore_key` (chỉ `#1` *Tết Nguyên Đán* có `Some(11)` trên file gốc).
* **`src/server/handlers/festival.rs`**: Chuẩn hóa `enum SeasonalNotice` (`0x01..=0x0D`, `0x15..=0x4A`, tách 1-byte `sub2` và 2-byte `sub2 + sub3`), `build_firework_effect_raw` (`0x17 0x46`), và `handle_festival` (`C→S 0x24 0x19` chỉ trả khi có lễ hội `Active` mang `unique_lore_key`).
* **`src/web/app.rs` & `templates/dashboard.html`**: Tự động broadcast `17 39` khi Start/Stop (cho 8 lễ hội có nhánh client), thêm `POST /api/festivals/{id}/lore` và `POST /api/festivals/firework`, hiển thị cột `Lore Key` + cảnh báo collision & vô hiệu hóa nút Lore khi `unique_lore_key.is_none()`.
* **Tests:** `tests/events_arena/festival_wire_parity.rs` (7 tests), cập nhật `tests/events_arena/festival_quiz.rs`.

### Ticket 03 — Đồng bộ Đăng nhập (`17 39` + Max 1 Popup `24 19`) & `FestivalDatEncoder` (`completed`)
* **`src/server/seasonal_festival.rs`**: Thêm `has_festival_claim` (`&pool.read`), `record_festival_claim` (`&pool.write`), `season_period_key`, `CLAIM_TYPE_LORE_SHOWN`.
* **`src/server/handlers/festival.rs` & `src/server/handlers/login.rs`**: Thêm `emit_login_festival_sync` trong `handle_login_complete` phát `17 39` cho tất cả lễ hội `Active` và tối đa `1` khung `24 19` chưa claim theo `season-<season_seq>`.
* **`src/data/loaders/festival.rs`, `src/bin/patch_festival_dat.rs`, `Cargo.toml`**: Thêm `FestivalDatEncoder` (`0x7EF` byte/record, tối đa `0x16E = 366` bản ghi, dual-XOR) và CLI `patch_festival_dat` (từ chối ghi đè `Data/Festival.Dat` gốc).
* **Tests:** `tests/events_arena/festival_login_and_encoder.rs` (3 tests round-trip `1..=7` / `1..=11` và multi-login/relog/`season_seq`).

### Ticket 04 — Gating Map Dùng chung vs Map Thuần, NPC Động (`16 01`) & Di tản An toàn (`completed`)
* **`src/server/seasonal_festival.rs`**: Thêm `SharedMapGate`, `DynamicFestivalNpc`, `exclusive_maps`, `shared_map_gates`, `dynamic_npcs`, cùng `check_door_access`, `check_npc_access`, `check_quest_access`, `active_dynamic_npcs_for_map`, `evacuable_maps_on_stop`.
* **`src/server/handlers/talk.rs`, `quest.rs`, `movement.rs`, `src/eve/planner.rs`**: Nối `check_npc_access`, `check_door_access`, và `check_quest_access` (`check_event_session_quest_access`) tại `0x14 01`, `0x14 04`, `0x14 08`, `0x06 01`, từ chối qua `sys_msg_frame`.
* **`src/server/spawn.rs`, `login.rs`, `system.rs`**: Thêm `npc_appear_frame`, phát `16 01` khi đăng nhập/chuyển map và broadcast ngay khi Admin bấm `Start`.
* **`src/server/session.rs`, `src/web/server_control.rs`, `src/web/app.rs`, `src/battle/service.rs`**: Thêm `pending_evacuate_map`, `evacuate_festival_players` (đồng bộ `BattleService::sync_online_session`, `registry_update_if_online`, SQLite về `savemap` hoặc `12001`), và `apply_pending_evacuation` khi kết thúc trận (`on_player_fled`, `on_battle_ended`, `leave_battle`, kèm `hide_from_map`).
* **Tests:** `tests/events_arena/festival_gating_and_evacuation.rs` (5 tests dùng `registry_register_for_login`).

### Kết quả kiểm chứng sau Ticket 04
* `cargo test --all-targets --no-fail-fast`: **677 passed, 0 failed** trên toàn bộ 11 test suites (`architecture_guardrails_test`: **8**, `battle_test`: **186**, `compound_test`: **21**, `data_loaders_test`: **40**, `db_test`: **21**, `economy_inventory_test`: **59**, `eve_quest_test`: **137**, `events_arena_test`: **82**, `server_network_test`: **103**, `server_stop_test`: **1**, `skill_job_system_test`: **19**).

---

## 3. Điểm Bắt Đầu Tiếp Theo (Frontier): Ticket 05

* **File ticket:** [`.scratch/seasonal-festival-subsystem/issues/05-seasonal-battle-drops-antifarm-and-exp.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/seasonal-festival-subsystem/issues/05-seasonal-battle-drops-antifarm-and-exp.md)
* **Mục tiêu cốt lõi:**
  1. Mở rộng `FestivalProfile` (`src/server/seasonal_festival.rs`) với `bonus_drops` (`item_id`, `drop_rate_pct`, `min_enemy_level`, `daily_drop_cap`) — lưu ý `exp_multiplier_pct: Option<u32>` đã có sẵn trên `FestivalProfile`.
  2. Nối `SeasonalFestivalManager` vào `BattleService` (`src/battle/service.rs` / `src/web/server_control.rs` — kiểm tra cách `BattleService` được khởi tạo trong `ServerControl` và `src/server/`).
  3. Tại luồng thắng trận PvE `Normal` (tuyệt đối **không** chạy cho `PKBattle`):
     * Áp dụng `exp_multiplier_pct` nhất quán cùng `perexp`, *Thần Tài (`FGod`)* và *Con Ma (`FGhost`)*.
     * Kiểm tra `min_enemy_level`, roll tỷ lệ `drop_rate_pct` bằng `DotNetRandom`, kiểm tra & tăng bộ đếm `daily_drop_cap` theo ngày `UTC+7` (`YYYY-MM-DD`) trong `character_festival_claims`.
     * Cộng vật phẩm qua quy trình túi đồ chuẩn (All-or-Nothing + khung `17 06` + `registry_update_if_online` + ghi SQLite nguyên tử); túi đầy thì bỏ qua an toàn.
  4. Viết Positive + Negative tests trong `tests/events_arena/` (bao gồm `PKBattle`, quái dưới `min_enemy_level`, chạm `daily_drop_cap` ngày `UTC+7` và mở lại sang ngày mới), chạy `cargo test --test events_arena_test`, `cargo test --test architecture_guardrails_test`, và `cargo test --all-targets --no-fail-fast`.

---

## 4. Tài Liệu Tham Chiếu Trọng Tâm

1. **Đặc tả chuẩn (PRD):** `.scratch/seasonal-festival-subsystem/spec.md`
2. **Danh sách Tickets:** `.scratch/seasonal-festival-subsystem/issues/01..05-*.md`
3. **Báo cáo Audit Client & Server:** `.scratch/seasonal-festival-subsystem/SEASONAL_FESTIVAL_AUDIT.md`
4. **Giải pháp Kỹ thuật:** `.scratch/seasonal-festival-subsystem/SEASONAL_FESTIVAL_SOLUTION.md`
5. **ADR Phân hệ:** `docs/adr/0011-seasonal-festival-lifecycle-and-protocol.md`
6. **Từ điển miền:** `GLOSSARY.md` (lines 493–517)
7. **Quy chuẩn lập trình & kiểm thử:** `CODING_STANDARDS.md`
