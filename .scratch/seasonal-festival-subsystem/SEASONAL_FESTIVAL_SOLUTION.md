# GIẢI PHÁP TOÀN DIỆN: HỖ TRỢ MODULE SỰ KIỆN THEO MÙA / LỄ HỘI TRÊN SERVER RUST (`ts_dream`)

> **Ngày lập:** 2026-11-18  
> **Trạng thái:** Đã chốt toàn bộ thiết kế qua `/grill-with-docs` (Round 1 Q1–Q7 & Round 2 Q8–Q11).  
> **Tài liệu nền tảng & quyết định liên quan:**
> - Báo cáo audit bằng chứng client/server: [`SEASONAL_FESTIVAL_AUDIT.md`](SEASONAL_FESTIVAL_AUDIT.md)
> - Quyết định kiến trúc đã phê duyệt: [ADR 0011](../../docs/adr/0011-seasonal-festival-lifecycle-and-protocol.md)
> - Từ điển thuật ngữ miền: [`GLOSSARY.md`](../../GLOSSARY.md) (*Sự kiện Đặc biệt vs Lễ hội Theo mùa*, *Bảng Lore Lễ hội*, *Thông báo & Hiệu ứng Mùa*)
> - Quy chuẩn lập trình & kiểm thử: [`CODING_STANDARDS.md`](../../CODING_STANDARDS.md), [`AGENTS.md`](../../AGENTS.md)

---

## 1. Tóm tắt các Quyết định Thiết kế đã Chốt

1. **Phân tách Bounded Context:** Tách `SeasonalFestivalManager` thành phân hệ độc lập quản lý `11` lễ hội (bỏ dòng placeholder `"seasonal_festival"` khỏi bảng `Special Events` cũ, miễn trừ watchdog 120 phút). Giữ `dragon_boat` (`OP 0x46`) độc lập với `doan_ngo`.
2. **Điều khiển thủ công từ Dashboard (Manual-only):** Không dùng scheduler/cron tự động theo ngày; Admin bấm `Start` / `Stop` từng lễ hội, phát `Lore (24 19)` và `Pháo hoa (17 46)` trên Web Admin Dashboard.
3. **Wire Parity `S→C 24 19` & `Festival.Dat`:** Sửa `FestivalDatLoader` đọc `dt as u32` (xóa hằng số sai `0x0121EAC6` trong `src/` và test cũ), giữ nguyên `Data/Festival.Dat` gốc trong repo (chỉ gửi `24 19` cho `rec01` khi file chưa patch và hiện cảnh báo Dashboard cho `rec02..rec07`), đồng thời cung cấp `FestivalDatEncoder` / CLI tool sinh bản patch khóa `1..=7` (và tùy chọn `8..=11` cho Thất Tịch, Trung Thu, Halloween, Giáng Sinh).
4. **Định danh đợt (`season_seq`) & Popup Login:** Lưu `season_seq: u64` trong SQLite (tăng `+1` khi `Idle → Active`, giữ nguyên khi restart). Khi login, gửi chat `17 39` cho tất cả lễ hội đang bật, và đẩy tối đa `1` popup `24 19` chưa nhận (`period_key = "season-<seq>"`).
5. **Gating Bản đồ dùng chung vs Thuần & Evacuation:** Gate chi tiết `(scene_id, door_id / npc_place_id / quest_id)` trên 4 map dùng chung (`12000`, `12001`, `12005`, `12421`); gate toàn bộ `map_id` trên các map lễ hội thuần (`54811..54831`, `12941..12943`, `57535..57536`) kèm di tản an toàn về `savemap` khi bấm `Stop`.
6. **Cân bằng Drop chống farm:** Cấu hình `drop_rate_pct`, `min_enemy_level`, và `daily_drop_cap` (`YYYY-MM-DD` GMT+7) trong `Data/seasonal_festivals.json`.

---

## 2. Kiến trúc tổng thể (6 lớp tích hợp)

```text
+-------------------------------------------------------------------------+
| Web Admin Dashboard (Port 8090) — templates/dashboard.html + web/app.rs |
|   Card "Seasonal Festivals Control Plane" (11 lễ hội độc lập)           |
|   [Start | Stop | Lore (24 19) | Trigger Fireworks (17 46)]             |
+------------------------------------+------------------------------------+
                                     | (HTTP / HTMX + Server Gate 409)
                                     v
+-------------------------------------------------------------------------+
| ServerControl — src/web/server_control.rs                               |
|   - Hydrate trạng thái & season_seq từ SQLite khi start TCP server      |
|   - Broadcast S→C 17 39 / 17 46 / 24 19 tới toàn bộ client online       |
|   - Đồng bộ spawn NPC (16 01) và di tản map thuần về savemap khi Stop   |
+------------------------------------+------------------------------------+
                                     |
         +---------------------------+---------------------------+
         v                                                       v
+---------------------------------------+   +-----------------------------+
| SeasonalFestivalManager (RAM + DB)    |   | GameData Static Catalogs    |
|  - Trạng thái Idle / Active + seq     |   |  - Festival.Dat (7+ lore)   |
|  - Kiểm tra tính duy nhất khóa 24 19  |   |  - seasonal_festivals.json  |
|  - Không thuộc watchdog 120 phút      |   |  - eve.emg, Npc, Item, Map  |
+----+------------------+---------------+   +-----------------------------+
     |                  |
     | (Check Gate)     | (Login / Map Enter / Battle Win)
     v                  v
+-------------------------------------------------------------------------+
| Game Handlers & Engine                                                  |
|  - login.rs / system.rs : Notice 17 39 + tối đa 1 popup 24 19 + NPC 1601|
|  - talk.rs / movement.rs: Gate chi tiết map chung & gate map thuần      |
|  - battle/service.rs    : Drop vật phẩm mùa (anti-farm) + EXP x2        |
|  - festival.rs          : S→C 24 19, 17 39 (SeasonalNotice), 17 46      |
+-------------------------------------------------------------------------+
```

---

## 3. Thiết kế chi tiết từng phân hệ

### 3.1. Lớp Protocol & `Festival.Dat` (`src/data/loaders/festival.rs`, `src/server/handlers/festival.rs`)

#### A. Sửa Wire Parity `S→C 24 19` & Bộ Encoder `Festival.Dat`
1. **`FestivalDatLoader::load`:**
   - Giải mã dual-XOR (`keyA` = record `n-2` cho chẵn, `keyB` = record `n-1` cho lẻ) như hiện tại.
   - Đọc `date_dword` trực tiếp từ `dt` (`if dt.is_finite() && dt >= 0.0 && dt <= u32::MAX as f64 && (dt - dt.round()).abs() < 1e-9 { Some(dt.round() as u32) } else { None }`), **bỏ hoàn toàn** hằng số sai `FESTIVAL_PARITY_DATE_DWORD = 0x0121EAC6`.
   - Tính bản đồ chỉ số hỗ trợ `is_lore_key_unique(id)`: đếm số bản ghi có cùng `date_dword`. Với `Data/Festival.Dat` gốc (tất cả `11.0`), chỉ `id == 1` (*Tết Nguyên Đán*) là hợp lệ để phát `24 19`; các `id >= 2` trùng `11.0` bị đánh dấu `unpatched_collision = true` và **không** phát `24 19`.
2. **`FestivalDatEncoder` & CLI Helper:**
   - Cung cấp `FestivalDatEncoder::encode(records: &[FestivalWriteRecord]) -> Result<Vec<u8>>`: ghi `record[0]` toàn `0x00`, ghi `records` từ `1..=k` (`k <= 0x16E = 366`) với `dt = key as f64`, `word_flag = 0xB4C2`, ShortString tiêu đề VISCII (`<= 20` bytes), `text1` (`<= 999` bytes), `text2` (`<= 999` bytes), và XOR bằng 2 bản ghi khóa cuối (`keyA`, `keyB`).
   - Hỗ trợ tái sinh `Festival.Dat` patch với khóa `1..=7` cho 7 lễ hội gốc, và tùy chọn thêm bản ghi `8..=11` (*Lễ Thất Tịch*, *Tết Trung Thu*, *Halloween*, *Lễ Giáng Sinh*) ra file đầu ra riêng để admin chép sang client (giữ `Data/Festival.Dat` gốc trong repo nguyên vẹn).
3. **Chính sách Login Popup `24 19`:**
   - Ở mỗi lần đăng nhập (`handle_login_complete`), chỉ gửi tối đa `1` frame `S→C 24 19` cho lễ hội đầu tiên đang `Active` thỏa cả 2 điều kiện: (a) có khóa `Festival.Dat` hợp lệ (không bị trùng `11.0`) và (b) nhân vật chưa có dòng `lore_shown` ở `period_key = "season-<season_seq>"` trong `character_festival_claims`.

#### B. Chuẩn hóa Type-Safe `S→C 17 39` (`SeasonalNotice`) & `S→C 17 46`
- Thay `build_seasonal_announcement_raw(id: u8)` bằng `enum SeasonalNotice` ánh xạ đúng từng nhánh trong `FUN_00722cd4` (`client_pseudo_c/00722cd4_FUN_00722cd4.c`):
  - Nhánh 1 byte (`sub2`): `0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x15, 0x16, 0x1B..0x20, 0x25, 0x29, 0x2A, 0x3A`.
  - Nhánh 2 byte (`sub2, sub3`): `0x2B <1|2>`, `0x2C <1..=3>`, `0x33 <1..=3>`, `0x34 <1..=3>`, `0x35 <1|2>`, `0x39 <1..=6>`, `0x3B <1..=5>`, `0x3D <1..=4>`, `0x3F <1..=3>`, `0x42 <1..=4>`, `0x43 <1..=4>`, `0x44 <1|2>`, `0x45 <1|2>`, `0x46 <1..=4>`, `0x47 <1|2>`, `0x49 <1|2>`.
- Bổ sung `build_firework_effect_raw(sub: u8, x: u16, y: u16, count: u8)` cho `S→C 0x17 0x46` khớp `FUN_0062024c` (`[0x17, 0x46, sub, x_lo, x_hi, y_lo, y_hi, count]`).

---

### 3.2. Danh mục 11 Lễ hội & Quy tắc Gate / Drop (`Data/seasonal_festivals.json`)

| Festival ID | Tên hiển thị | `Festival.Dat` ID | Kiểu Gate Map / EVE | Drop & Hiệu ứng đặc thù |
|---|---|---|---|---|
| `tet_nguyen_dan` | Tết Nguyên Đán & Niên Thú | `1` | Gate Scene `55002` / NPC `1604, 1659, 3998, 4962, 4945` | Drop `46335`/`46338` Bao lì xì; Notice `0x01/0x08/0x44/0x45/0x46/0x47`; Pháo hoa `17 46` |
| `tet_nguyen_tieu` | Tết Nguyên Tiêu | `2` | Gate thuần `54811..54812`; Gate chi tiết trên `12001` | Notice `0x2C/0x3D/0x3F/0x49`; tùy chọn `exp_multiplier_pct = 200` |
| `valentine` | Lễ Tình Nhân | `3` | Gate thuần `54821..54824` (nếu có) | Notice `0x34 <1..=3>` (sub 3 kích hoạt pháo hoa) |
| `ca_thang_tu` | Ngày Cá Tháng Tư | `4` | Thông báo & Lore | Lore `24 19` (khi patch `Festival.Dat`) + `sys_msg_frame` |
| `gio_to_hung_vuong` | Giỗ Tổ Hùng Vương | `5` | Thông báo & Lore | Lore `24 19` (khi patch `Festival.Dat`) + `sys_msg_frame` |
| `phat_dan` | Lễ Phật Đản | `6` | Thông báo & Lore | Lore `24 19` (khi patch `Festival.Dat`) + `sys_msg_frame` |
| `doan_ngo` | Tết Đoan Ngọ | `7` | Gate thuần `12941..12943` (Tháp Lôi Phong); Gate chi tiết trên `12000` | Drop `46220` Túi Đoan Ngọ; Notice `0x03/0x04/0x33/0x42`; **độc lập với `dragon_boat`** |
| `that_tich` | Lễ Thất Tịch | *(8 nếu patch)* | Gate thuần `54825`; Gate chi tiết trên `10995`, `12001` (Quest `10786`) | NPC `1825, 1879, 3453`; Notice `0x35 <1|2>` |
| `trung_thu` | Tết Trung Thu | *(9 nếu patch)* | Gate chi tiết trên `12005` (Hội nướng thịt) | NPC `3468, 3886`; Notice `0x1E/0x1F/0x25/0x39/0x3A`, `0x14 2D/2E` |
| `halloween` | Halloween & Quỷ Thành | *(10 nếu patch)* | Gate thuần `57518`, `57531..57536`, `61051`, `61178`, `61551..61567` | Drop `46073` Kẹo phá đám; NPC `1259, 3875`; Notice `0x05/0x06/0x1B..0x1D` |
| `giang_sinh` | Lễ Giáng Sinh (Noel) | *(11 nếu patch)* | Gate thuần `54827..54831`; Gate chi tiết trên `12421` (Quest `10044, 11542`) | NPC `3865..4003`; Notice `0x15/0x16/0x29..0x2B/0x3B/0x43`; Tuyết rơi client `FUN_006221e8` |

Mỗi profile trong `Data/seasonal_festivals.json` có block cấu hình chống farm:
- `bonus_drops`: `[{ item_id, drop_rate_pct, min_enemy_level, daily_drop_cap }]`
- `exp_multiplier_pct`: `Option<u32>` (vd. `Some(200)` cho Nguyên Tiêu).

---

### 3.3. Vòng đời `SeasonalFestivalManager`, Gating & Di Tản

1. **Gỡ bỏ placeholder cũ:** Xóa `EVENT_SEASONAL_FESTIVAL` khỏi `EventLifecycleManager::new` (`src/server/events.rs`), giữ lại đúng `8` sự kiện đặc biệt (`north_star`, `continuous_40_npc`, `rescue_2k`, `nine_heavens`, `arena`, `guild_siege`, `water_war`, `dragon_boat`).
2. **State & `season_seq`:** `SeasonalFestivalManager` giữ `HashMap<String, SeasonalFestivalState>` gồm `{ is_active: bool, season_seq: u64, started_at: Option<i64> }`:
   - `start(id)`: yêu cầu TCP server đang running (`409` nếu stopped); nếu đang `Idle` thì set `is_active = true`, tăng `season_seq += 1`, ghi DB, broadcast `17 39` mở màn + spawn NPC động (`16 01`) cho người chơi trên map liên quan.
   - `stop(id)`: set `is_active = false` (giữ nguyên `season_seq`), ghi DB, broadcast `17 39` kết thúc, và kích hoạt **di tản các map phụ bản lễ hội thuần** về `savemap` (nếu `savemap == 0` thì về `12001`; nếu player đang `battle_id > 0` thì đánh dấu `pending_festival_evacuate` để đưa về `savemap` ngay khi kết thúc trận hiện tại).
3. **Granular Gating (`talk.rs`, `movement.rs`, `npc_event.rs`):**
   - Trước khi khởi chạy `resolve_npc_event` / `MeetDoor` / `ClickDoor` / `handle_warp_confirm`: kiểm tra `SeasonalFestivalManager::check_access(map_id, door_id, npc_place_id, quest_id)`:
     - Nếu `map_id` nằm trong danh sách **map lễ hội thuần** của một lễ hội đang `Idle` → chặn vào và gửi `sys_msg_frame`.
     - Nếu `(map_id, door_id)` hoặc `(map_id, npc_place_id)` hoặc `quest_id` nằm trong danh sách **gate chi tiết của 4 map dùng chung (`12000`, `12001`, `12005`, `12421`)** và lễ hội đó đang `Idle` → chặn riêng tương tác đó (các NPC/cổng cốt truyện thường trên cùng map vẫn chạy bình thường).
4. **Bonus Drop & Anti-Farm (`src/battle/service.rs`):**
   - Trong luồng trả thưởng thắng trận PvE `Normal` (tuyệt đối không chạy trong `PKBattle`), kiểm tra từng lễ hội đang `Active`: nếu cấp trung bình/cao nhất của quái `>= min_enemy_level`, roll `DotNetRandom` theo `drop_rate_pct`, kiểm tra bộ đếm ngày trong `character_festival_claims` (`period_key = YYYY-MM-DD` GMT+7, `claim_type = "drop_<item_id>"`), nếu chưa đạt `daily_drop_cap` thì cộng item (`17 06`), gọi `registry_update_if_online` và tăng bộ đếm DB.

---

### 3.4. Persistence SQLite (`migrations/0003_seasonal_festivals.sql`)

```sql
CREATE TABLE IF NOT EXISTS seasonal_festival_state (
    festival_id  TEXT PRIMARY KEY,
    is_active    INTEGER NOT NULL DEFAULT 0,
    season_seq   INTEGER NOT NULL DEFAULT 0,
    started_at   INTEGER,
    updated_at   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS character_festival_claims (
    playerid     INTEGER NOT NULL,
    festival_id  TEXT NOT NULL,
    period_key   TEXT NOT NULL, -- 'season-<seq>' cho lore_shown/quà đợt, hoặc 'YYYY-MM-DD' cho daily_drop_cap
    claim_type   TEXT NOT NULL, -- 'lore_shown' | 'drop_<item_id>' | 'gift_<id>'
    claim_count  INTEGER NOT NULL DEFAULT 1,
    claimed_at   INTEGER NOT NULL,
    PRIMARY KEY (playerid, festival_id, period_key, claim_type)
);
```

---

### 3.5. Web Admin Dashboard (`templates/dashboard.html` & `src/web/app.rs`)

- **Bảng `Special Events Control Plane`:** Chỉ còn `8` sự kiện đặc biệt.
- **Card mới `Seasonal Festivals Control Plane`:** Hiển thị `11` lễ hội với các cột:
  - `ID`, `Tên lễ hội`, `Trạng thái` (`ACTIVE` / `IDLE`), `Đợt (season_seq)`, `Lore Key` (hiện badge cảnh báo `"Unpatched 11.0 (only #1)"` nếu `Festival.Dat` gốc chưa patch khóa cho `rec02..rec07`), `Hành động`:
    - `Start` (`POST /api/festivals/{id}/start`)
    - `Stop` (`POST /api/festivals/{id}/stop`)
    - `Lore (24 19)` (`POST /api/festivals/{id}/lore` — bị disable hoặc trả `409` kèm giải thích nếu `Festival.Dat` chưa patch khóa riêng cho lễ hội đó)
  - Thanh tác vụ nhanh: `Trigger Fireworks (17 46)` (`POST /api/festivals/firework`).
- Tất cả các route trên đều đi qua `admin_audit_log` và trả `409 Conflict` nếu Game TCP server đang `STOPPED`.

---

## 4. Lộ trình triển khai chuẩn bị sẵn (5 Phases)

| Phase | Nội dung triển khai | File chính tác động |
|---|---|---|
| **Phase 1: Protocol & `Festival.Dat` Encoder** | Sửa `FestivalDatLoader` (`dt as u32`, check unique key); thêm `FestivalDatEncoder` + CLI sinh `Festival.Dat` patch `1..=7` (+`8..=11`); viết lại `festival.rs` (`SeasonalNotice` 66 nhánh `17 39`, `24 19`, `17 46`); sửa assertion cũ `0x0121EAC6` trong test. | `src/data/loaders/festival.rs`, `src/server/handlers/festival.rs`, `tests/events_arena/festival_quiz.rs` |
| **Phase 2: `SeasonalFestivalManager` & DB State** | Gỡ `EVENT_SEASONAL_FESTIVAL` khỏi `EventLifecycleManager`; xây `SeasonalFestivalManager` + `Data/seasonal_festivals.json`; thêm `migrations/0003_seasonal_festivals.sql` + queries trong `src/db/domain.rs`. | `src/server/events.rs`, `migrations/0003_seasonal_festivals.sql`, `src/db/domain.rs` |
| **Phase 3: Gameplay Gating, Drop & Evacuation** | Hook `login.rs` (notice `17 39` + tối đa 1 popup `24 19` + NPC `16 01`), `system.rs`, gate chi tiết trong `talk.rs`/`movement.rs`/`npc_event.rs`, bonus drop + anti-farm + EXP trong `battle/service.rs`, di tản map thuần khi Stop. | `login.rs`, `system.rs`, `talk.rs`, `movement.rs`, `npc_event.rs`, `battle/service.rs`, `server_control.rs` |
| **Phase 4: Web Admin Dashboard UX** | Thêm card `Seasonal Festivals Control Plane` trên `templates/dashboard.html` + các route `/api/festivals/...` + badge cảnh báo `Festival.Dat` + audit log. | `templates/dashboard.html`, `src/web/app.rs` |
| **Phase 5: Test Suite & Guardrails** | Bổ sung unit & integration tests (positive + negative) trong `tests/events_arena/` cho toàn bộ luồng `24 19`, `17 39`, `17 46`, `season_seq`, shared-map gating, anti-farm drop, di tản khi Stop, và Web API `409`. | `tests/events_arena/` (`cargo test --test events_arena_test`, `cargo test --all-targets --no-fail-fast`) |
