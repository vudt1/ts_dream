# HANDOFF: Phân Hệ Lễ Hội Theo Mùa (`SeasonalFestival`) — Tiếp nối từ Ticket 01 sang Ticket 02–05

* **Dự án:** `ts_dream` (Rust Game Server parity với Client TS Online `aLogin.exe`)
* **Thời điểm bàn giao:** 2026-11-18
* **Vị trí tài liệu:** `.scratch/seasonal-festival-subsystem/handoff-seasonal-festival-implementation.md`
* **Trạng thái hiện tại:**
  * Đã hoàn thành 100% nghiên cứu dịch ngược (`SEASONAL_FESTIVAL_AUDIT.md`), kiến trúc (`SEASONAL_FESTIVAL_SOLUTION.md`), `GLOSSARY.md`, `ADR 0011`, `spec.md`, và chia 5 tickets (`01`..`05`).
  * **Ticket 01 (`01-seasonal-festival-manager-and-dashboard-control.md`) đã hoàn thành, đã qua 2-axis Code Review (Standards & Spec) và pass toàn bộ 662 tests (`cargo test --all-targets --no-fail-fast`).**
  * Thay đổi đang được giữ nguyên tại working tree (không commit theo quy tắc `AGENTS.md` để user tự commit thủ công).
  * **Điểm bắt đầu tiếp theo (Frontier):** **Ticket 02** (`02-festival-wire-parity-and-live-broadcast.md`).

---

## 1. Suggested Skills & Quy Tắc Bắt Buộc Cho Agent Tiếp Theo

1. **`tdd`**: Thực hiện theo vòng lặp Red → Green → Refactor theo từng vertical slice tại các seam công khai trong `tests/events_arena/` (đăng ký trong `tests/events_arena_test.rs`).
2. **`code-review`**: Sau mỗi ticket, đối chiếu diff theo 2 trục **Standards** (`CODING_STANDARDS.md`, `GLOSSARY.md`, `ADR 0011`) và **Spec** (`spec.md` + file ticket tương ứng).
3. **Ràng buộc kỹ thuật & công cụ (`AGENTS.md` + `CODING_STANDARDS.md`)**:
   * **KHÔNG** đặt `#[cfg(test)]` hoặc pub test helpers trong `src/` (được kiểm tra tự động bởi `tests/architecture_guardrails_test.rs`).
   * **KHÔNG** gọi trực tiếp `.insert(...)` vào `lock_online_sessions()` trong handlers; dùng `registry_update_if_online(&Session)`.
   * **KHÔNG** tự động `git commit`, `cargo update`, hay `cargo clean`.
   * Trên Windows PowerShell của workspace này không có `rg`; dùng `Select-String` và chỉ tìm trong `src/`, `tests/`, `docs/`, `spec/`, `templates/`, `migrations/` (tránh `Data/`, `DB/`, `target/`, `client_delphi/`).
   * Mọi test SQLite phải dùng `setup_test_db().await` (`DB/ts_dream_test.db`).

---

## 2. Tóm Tắt Những Gì Đã Hoàn Thành Ở Ticket 01

### 2.1. Các tệp đã tạo mới / chỉnh sửa (đang ở working tree)
1. **Tách `SpecialEvent` về đúng 8 sự kiện (`src/server/events.rs`)**:
   * Đã gỡ hằng số `EVENT_SEASONAL_FESTIVAL` và entry `"seasonal_festival"` khỏi `EventLifecycleManager::new`.
   * Giữ lại đúng `8` sự kiện đặc biệt (`north_star`, `continuous_40_npc`, `rescue_2k`, `nine_heavens`, `arena`, `guild_siege`, `water_war`, `dragon_boat`), với `dragon_boat` độc lập hoàn toàn với `doan_ngo`.
2. **Migration SQLite & Test DB (`migrations/0003_seasonal_festivals.sql`, `tests/common/test_db.rs`)**:
   * Tạo bảng `seasonal_festival_state` (`festival_id`, `is_active`, `season_seq`, `started_at`, `updated_at`) và bảng `character_festival_claims` (`playerid`, `festival_id`, `period_key`, `claim_type`, `claim_count`, `claimed_at`) — sẵn sàng cho cả Ticket 03 & Ticket 05.
   * Đã nối `MIGRATION_0003` vào `reset_schema()` trong `tests/common/test_db.rs`.
3. **`SeasonalFestivalManager` (`src/server/seasonal_festival.rs`, `src/lib.rs`)**:
   * Định nghĩa `FestivalProfile`, `SeasonalFestivalInfo`, `SeasonalFestivalManager`.
   * Quản lý `11` lễ hội (`tet_nguyen_dan`, `tet_nguyen_tieu`, `valentine`, `ca_thang_tu`, `gio_to_hung_vuong`, `phat_dan`, `doan_ngo`, `that_tich`, `trung_thu`, `halloween`, `giang_sinh`), nạp từ `Data/seasonal_festivals.json` nếu tồn tại hoặc dùng `SeasonalFestivalManager::default_profiles()`.
   * Miễn trừ hoàn toàn khỏi `EventLifecycleManager::check_watchdog()`, cho phép nhiều lễ hội `Active` đồng thời.
   * Tuân thủ Two-Phase State Lock: `start_festival` (`Idle -> Active`, `season_seq += 1`) và `stop_festival` (`Active -> Idle`, giữ nguyên `season_seq`) cập nhật RAM trước, ghi SQLite qua `&pool.write`, và rollback RAM nếu DB lỗi.
   * `hydrate_from_db` đọc lại `seasonal_festival_state` khi `ServerControl::start()`.
4. **Web Admin & ServerControl (`src/web/server_control.rs`, `src/web/app.rs`, `templates/dashboard.html`)**:
   * Thêm `ServerControl::festivals: Arc<SeasonalFestivalManager>`, hydrate khi `ServerControl::start()`.
   * Thêm card `#seasonal-festivals-card` (**Seasonal Festivals Control Plane**) trên `templates/dashboard.html` (trong `#tab-events`).
   * Thêm routes `GET /api/festivals`, `POST /api/festivals/{id}/start`, `POST /api/festivals/{id}/stop` (hỗ trợ cả HTMX fragment khi có `HX-Request` lẫn JSON), trả `409 Conflict` khi server `STOPPED` hoặc chuyển trạng thái không hợp lệ, `404 Not Found` khi ID lạ, và ghi `admin_audit_log` (`target_type = "seasonal_festival"`).
5. **Test Suite (`tests/events_arena/seasonal_festival_control.rs`, `tests/events_arena_test.rs`)**:
   * 5 test bao phủ: 8 `SpecialEvent`, 11 `SeasonalFestival` + `dragon_boat` độc lập, multi-active + `season_seq` persistence qua restart, negative cases (`409`/`404`), audit log + HTMX render.

### 2.2. Kết quả kiểm chứng sau Ticket 01
* `cargo test --test events_arena_test`: **67 passed, 0 failed**
* `cargo test --test architecture_guardrails_test`: **8 passed, 0 failed**
* `cargo test --all-targets --no-fail-fast`: **662 passed, 0 failed** (`architecture_guardrails_test` 8, `battle_test` 186, `compound_test` 21, `data_loaders_test` 40, `db_test` 21, `economy_inventory_test` 59, `eve_quest_test` 137, `events_arena_test` 67, `server_network_test` 103, `server_stop_test` 1, `skill_job_system_test` 19).
* **Ghi chú Code Review (nên dọn nhẹ khi chạm lại file ở Ticket 02)**:
  * Trong `src/server/seasonal_festival.rs`, `start_festival` và `stop_festival` có logic snapshot/persist/rollback tương đồng — có thể gom thành helper `transition_festival` và clone `FestivalEntry` thay vì tuple 4 phần tử.
  * Trong `tests/events_arena/seasonal_festival_control.rs`, negative test mới assert HTTP status `409`/`404` — nên assert thêm JSON body `{"error": ...}` để khớp `CODING_STANDARDS.md §3.5`.

---

## 3. Lộ Trình Thực Thi Tiếp Theo (Frontier & Dependency Graph)

```text
[Ticket 01: DONE]
       │
       ▼
[Ticket 02: Wire Parity 24 19, SeasonalNotice 17 39, Pháo hoa 17 46 & Broadcast] ◄── BẮT ĐẦU TẠI ĐÂY
       │
       ├────────────────────────────────────────┐
       ▼                                        ▼
[Ticket 03: Login Lore Claims & Encoder]   [Ticket 04: Shared Map Gating, NPC 16 01 & Di tản]  (Chạy song song được)
       │
       ▼
[Ticket 05: Battle Drops Chống Farm & EXP]
```

### 🎯 Bước Tiếp Theo Ngay Lập Tức: Ticket 02
* **File ticket:** `.scratch/seasonal-festival-subsystem/issues/02-festival-wire-parity-and-live-broadcast.md`
* **Các tệp chính sẽ chạm:**
  * `src/data/loaders/festival.rs` (bỏ `FESTIVAL_PARITY_DATE_DWORD = 0x0121EAC6`, đọc `dt as u32` trực tiếp từ `double @+0x00`, phát hiện collision `11.0` nên chỉ cho phép bản ghi `#1` *Tết Nguyên Đán* gửi `24 19`, đánh dấu `#2..#7` chưa có khóa riêng).
  * `src/server/handlers/festival.rs` (chỉ trả `24 19` khi lễ hội `Active` có khóa duy nhất hợp lệ; định nghĩa `enum SeasonalNotice` cho `66` nhánh `17 39` có tham số `payload[2]` bắt buộc ở các nhánh như `0x44`, `0x2B`, `0x35`, `0x39`, `0x33`, loại bỏ nhánh khuyết `0x0E..0x14` và nhánh phi-lễ-hội; thêm `build_firework_effect_raw` cho `S→C 0x17 0x46 [sub, x: u16 LE, y: u16 LE, count: u8]`).
  * `src/web/app.rs` + `src/web/server_control.rs` + `templates/dashboard.html`: nối broadcast `17 39` khi Start/Stop, thêm `POST /api/festivals/{id}/lore` và `POST /api/festivals/firework` (qua `ServerControl::broadcast_packet`), hiển thị cảnh báo cột `Lore Key` trên Dashboard và vô hiệu hóa nút Lore cho `#2..#7` khi chạy `Festival.Dat` gốc.
  * `tests/events_arena/festival_quiz.rs` (cập nhật các test cũ đang assert `0x0121EAC6` / `19000110` và `17 39 01..4A` cũ) + thêm test parity mới.
* **Bằng chứng client đã xác minh sẵn (không cần audit lại `client_pseudo_c/`):**
  * `0051d914` (`OP 0x24 sub 0x19`) & `0077ed68` & `0051e2dc`: client đọc `u32 LE`, dùng `FILD qword` ép thành `(double)u32` để so `==` trực tiếp với trường `double` trong `Festival.Dat`.
  * `00722cd4` (`OP 0x17 sub 0x39`): 66 nhánh (`0x01..0x0D`, `0x15..0x4A`), chi tiết bảng nhánh nằm trong `SEASONAL_FESTIVAL_AUDIT.md`.
  * `0062024c` (`OP 0x17 sub 0x46`): spawn hạt `TFireLight` theo `[0x17, 0x46, sub, x: u16 LE, y: u16 LE, count: u8]`.

### Các Ticket Sau Ticket 02
* **Ticket 03** (`.scratch/seasonal-festival-subsystem/issues/03-login-lore-claims-and-festival-dat-encoder.md`): `character_festival_claims` đã có sẵn bảng từ migration `0003`; cần thêm logic login (`src/server/handlers/login.rs`) gửi `17 39` cho tất cả lễ hội `Active` + tối đa `1` popup `24 19` chưa claim cho `season-<seq>`, và thêm `FestivalDatEncoder` + CLI tool xuất `Festival.Dat` patched (giữ nguyên file gốc trong repo).
* **Ticket 04** (`.scratch/seasonal-festival-subsystem/issues/04-shared-map-gating-dynamic-npcs-and-evacuation.md`): khai báo `exclusive_maps`, `shared_map_gates` (`12000`, `12001`, `12005`, `12421`), `dynamic_npcs`; nối gating vào `talk.rs`, `movement.rs`, `npc_event.rs`; phát `16 01` khi Start/vào map; di tản battle-safe về `savemap` (fallback `12001`) khi Stop.
* **Ticket 05** (`.scratch/seasonal-festival-subsystem/issues/05-seasonal-battle-drops-antifarm-and-exp.md`): nối `SeasonalFestivalManager` vào `BattleService` (`src/battle/`), áp dụng `bonus_drops` (`min_enemy_level`, `DotNetRandom`, `daily_drop_cap` theo ngày `UTC+7` trong `character_festival_claims`) và `exp_multiplier_pct` chỉ cho trận PvE `Normal` (tuyệt đối không cho `PKBattle`).

---

## 4. Tài Liệu Tham Chiếu Trọng Tâm

1. **Đặc tả chuẩn (PRD):** `.scratch/seasonal-festival-subsystem/spec.md`
2. **Danh sách Tickets:** `.scratch/seasonal-festival-subsystem/issues/01..05-*.md`
3. **Báo cáo Audit Client & Server (bằng chứng ASM/C & danh sách ID map/NPC/item):** `.scratch/special-event-refactor/SEASONAL_FESTIVAL_AUDIT.md`
4. **Tài liệu Giải pháp Kỹ thuật:** `.scratch/special-event-refactor/SEASONAL_FESTIVAL_SOLUTION.md`
5. **ADR Phân hệ:** `docs/adr/0011-seasonal-festival-lifecycle-and-protocol.md`
6. **Từ điển miền:** `GLOSSARY.md` (lines 493–517)
7. **Quy chuẩn lập trình & kiểm thử:** `CODING_STANDARDS.md`
