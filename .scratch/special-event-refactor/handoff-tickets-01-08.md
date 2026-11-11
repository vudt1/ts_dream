# HANDOFF: Phân Hệ Sự Kiện Đặc Biệt (Special Events) - Hoàn Thành Toàn Bộ Tickets 01 Đến 08

* **Dự án:** `ts_dream` (Rust Game Server parity với Client TS Online `aLogin.exe`)
* **Thời điểm bàn giao:** 2026-11-11
* **Tệp tài liệu:** `.scratch/special-event-refactor/handoff-tickets-01-08.md`
* **Trạng thái:** Đã hoàn thành 100% việc thực thi và kiểm thử cho **toàn bộ 8 Tickets (01, 02, 03, 04, 05, 06, 07, 08)**. Toàn bộ các test suite liên quan (Unit, Integration, Architecture Guardrails, DB Repositories) đều vượt qua (100% Pass).

---

## 1. Tóm Tắt Chi Tiết 8 Tickets Đã Hoàn Thành

### ✅ Ticket 01: Character God & Ghost State and Real-time Synchronization
- **Tệp ticket:** `.scratch/special-event-refactor/issues/01-god-ghost-state-and-sync.md`
- **Các thành phần đã thực hiện:**
  1. **Database Schema & Persistence:**
     - Bổ sung `ghost INTEGER NOT NULL DEFAULT 0` và `god INTEGER NOT NULL DEFAULT 0` vào `migrations/0001_init.sql`.
     - Cập nhật `src/db/modern/sqlite/session.rs` (`hydrate_session` và `persist_session`) để load/save trường `ghost` và `god`.
     - Cập nhật `src/db/persist.rs` để nhận diện cột `"Ghost" | "ghost"` và `"God" | "god"`.
  2. **Session Model:**
     - Bổ sung `pub ghost: u8` vào `Session` struct (`src/server/session.rs`).
  3. **Map Appearance Codec & Real-time State Sync Frame:**
     - Sửa `src/server/spawn.rs` hàm `player_appear` và `build_logined_sequence_session_ext` truyền chính xác `ghost` (body offset 5, wire offset 6) và `(god > 0) as u8` (body offset 6, wire offset 7).
     - Bổ sung helper `state_sync_frame(char_id: u32, kind: u16, val: u8)` sinh frame `F4 44 09 00 18 08 [char_id: 4B LE][kind: 2B LE][val: 1B]`.
  4. **Exorcism & Buff Consumables:**
     - Consumable 46180 ("Cải vận phù"): xóa `ghost = 0`, phát frame `18 08 (type 1, val 0)`, persist trạng thái vào DB.
     - Consumable 46169 (Sách Thần Tài): cộng dồn `god`, phát frame `18 08 (type 2, val 1)` khi chuyển đổi từ 0 sang > 0.
     - NPC 1752 ("Bà Gọi Hồn"): kiểm tra điều kiện ma theo (`ghost == 1`) và tiền (`gold >= 500`), trừ 500 vàng, gán `ghost = 0`, phát frame `18 08 (type 1, val 0)` và đồng bộ vàng.
  5. **Combat Formula EXP Multiplier & Step Decrement:**
     - Trong `src/battle/combat_formula.rs`: bổ sung `apply_god_ghost_exp(exp, god, ghost)`:
       - Thần tài (`god > 0`): `exp * 1.5`
       - Con ma (`ghost == 1`): `exp * 0.5`
     - Trong `src/battle/runner.rs`: trừ 1 điểm `god` sau mỗi trận thắng dã ngoại; nếu `god` về 0 thì phát frame tắt Thần tài `18 08 (type 2, val 0)`.

---

### ✅ Ticket 02: North Star Quiz In-Battle Encounter
- **Tệp ticket:** `.scratch/special-event-refactor/issues/02-north-star-quiz-battle-encounter.md`
- **Các thành phần đã thực hiện:**
  1. **Ngân hàng câu hỏi trắc nghiệm chuẩn:**
     - Tạo tệp `Data/north_star_questions.json` gồm 35 câu hỏi chuẩn về Tam Quốc / TS Online, định dạng UTF-8 tiếng Việt, 4 phương án lựa chọn, index đáp án 1..4.
  2. **Giao thức gói tin Wire Parity Opcode 0x10:**
     - Gói câu hỏi mở modal S→C 0x10 Sub 0x01: `F4 44 [len: 2B LE] 10 01 [qLen: 1B][q_viscii][opt1Len: 1B][opt1_viscii][opt2Len: 1B][opt2_viscii][opt3Len: 1B][opt3_viscii][opt4_viscii]` (tùy chọn 4 không có prefix độ dài mà trải đều hết byte còn lại).
     - Gói câu trả lời C→S 0x10 Sub 0x01: `[answer_index: 4B LE]`.
     - Phản hồi đúng: S→C `F4 44 03 00 10 02 01`, thưởng Thần Tài (`god += 10`), phát gói `18 08 (type 2, val 1)`, kết thúc trận.
     - Phản hồi sai / Hết giờ: S→C `F4 44 03 00 10 02 02`, phạt Con Ma (`ghost = 1`), rút `hp = 1`, phát gói `18 08 (type 1, val 1)`, kết thúc trận.
  3. **Khóa chống quan chiến (Anti-Spectate Lock):**
     - S→C `F4 44 03 00 21 01 03` gửi trả khi người chơi khác cố gắng quan chiến trận đấu Bắc Tinh Quân. Client Delphi hiển thị `"Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến"` và từ chối vào xem.
  4. **Cô lập Đội trưởng, Thoát trận & Giải phóng Tổ đội (Party Evacuation):**
     - Chỉ Đội trưởng nhận modal câu hỏi và có quyền nộp đáp án.
     - Ngăn chặn triệt để turn combat nền: sử dụng dummy handle trong `start_north_star_battle` để NPC không tự động tấn công người chơi trong lúc đang trả lời câu đố.
     - Khi đội trưởng trả lời xong (đúng hoặc sai), hoặc khi hết giờ, hoặc khi đội trưởng disconnect: toàn bộ thành viên tổ đội được tự động giải phóng khỏi trận (`battle_id = 0`) và nhận gói `battle_trailer()` đưa về map an toàn, trong khi HP, God, Ghost của thành viên vẫn được giữ nguyên độc lập.
  5. **Bộ đếm thời gian 45 giây tự động giải quyết (Asynchronous Countdown Resolver):**
     - Tiến trình định kỳ 1 giây trong `ServerControl` tự động quét các trận vấn đáp quá hạn (> 45 giây) và kích hoạt phạt tự động (`10 02 02`, `hp = 1`, `ghost = 1`, `18 08 [playerId][01 00][01]`, thoát trận).
  6. **Tích hợp di chuyển ngẫu nhiên (Wild Encounter):**
     - Trong `src/server/handlers/movement.rs` (`try_trigger_random_encounter`), nếu sự kiện `north_star` đang mở (`Active`), áp dụng tỷ lệ cấu hình (mặc định 5%) kích hoạt gặp NPC 1643/1644 và mở trận vấn đáp.

---

### ✅ Ticket 03: Web Admin Hierarchical Special Event Control
- **Tệp ticket:** `.scratch/special-event-refactor/issues/03-web-admin-hierarchical-event-control.md`
- **Các thành phần đã thực hiện:**
  1. **Bộ quản lý vòng đời sự kiện tập trung (`EventLifecycleManager`):**
     - Mô hình FSM 5 pha: `Idle`, `Announcing`, `Active`, `Ending`, `Cooldown`.
     - Bộ đếm thời gian chống treo (Starvation Watchdog): tối đa 120 phút tự động chuyển sang `Ending`/`Cooldown`.
     - Chạy nền tự động (Background Watchdog Task): tiến trình ngầm 5s/1s trong `ServerControl::start` tự động kiểm tra và thực thi chuyển pha cho cả Starvation Watchdog (120m) và Graceful Drain (3m) mà không cần can thiệp thủ công.
     - Strict Mutual Exclusion: từ chối kích hoạt trùng lặp (trả về HTTP 409 Conflict).
     - Rejection Gate: từ chối khởi chạy sự kiện khi Game Server đang dừng (`ServerStatus::Stopped`), trả về 409 Conflict kèm thông báo `"Game server is stopped. Please start the server first"`.
     - Đồng bộ trạng thái: `ServerControl::with_events` đảm bảo `BattleService` và `ServerControl` luôn trỏ chung một phiên bản Arc `EventLifecycleManager`.
  2. **Cascading Emergency Drain:**
     - Khi TCP Game Server dừng (`stop` hoặc `stop_immediate`), tự động xả dọn toàn bộ sự kiện (`emergency_cascading_stop`):
       - Hủy toàn bộ trận đấu sự kiện đang diễn ra.
       - Phát cảnh báo `00 2F` (`EventEndedBeforeBattle`) ngắt người chơi khỏi trận.
       - Lưu dữ liệu trạng thái nhân vật an toàn vào SQLite, xóa sạch `battle_id` cho cả đội trưởng và thành viên.
  3. **Thanh trượt tỷ lệ Encounter (1% - 30%):**
     - API `POST /api/events/north_star/rate` cho phép điều chỉnh tỷ lệ gặp Bắc Tinh Quân tức thời mà không cần restart server.
  4. **Admin Audit Logging:**
     - Bổ sung bảng `admin_audit_log` trong `migrations/0001_init.sql`.
     - Tự động ghi vết mọi hành động Bật (`event_start`), Tắt (`event_stop`), và Thay đổi tỷ lệ (`event_rate_change`) kèm timestamp.
  5. **Web Admin UI HTMX:**
     - Tích hợp Card `#special-events-card` vào `templates/dashboard.html` trên cổng 8090.
     - Cập nhật thời gian thực trạng thái, số trận đang mở, nút Bật/Tắt và thanh trượt tỷ lệ qua HTMX outerHTML swap.

---

### ✅ Ticket 04: Continuous Dungeon 40 NPC In-Engine Wave Chaining
- **Tệp ticket:** `.scratch/special-event-refactor/issues/04-continuous-dungeon-40-npc-wave-chaining.md`
- **Các thành phần đã thực hiện:**
  1. **Registration & Entry Gating:**
     - NPC 15162 ("Lôi Đài Truyền Sư" tại map 12001) trong `src/server/handlers/talk.rs`:
       - Kiểm tra vé 46072 ("Vé ưu đãi lôi đài") trong túi đồ, hoặc thu 1,000 vàng.
       - Gán tiến trình `QuestStep(218 = 1)`, dịch chuyển tới lôi đài map 10851.
       - Lưu dữ liệu người chơi qua SQLite Dual-Pool.
  2. **In-Engine Wave Chaining (Trận chiến liên tục không reload scene):**
     - Mở rộng `Battle` trong `src/battle/construction.rs` và `src/battle/runner.rs`:
       - Thêm `waves: Vec<Vec<(Npc, u8, u8)>>`, `current_wave: usize`.
       - Hàm `advance_wave()`: dọn sạch các ô địch đã bị tiêu diệt (`reset_cell`), nạp đợt NPC tiếp theo (`entity_npc`) trực tiếp vào trận chiến hiện tại mà không phát `battle_trailer` hay reload scene.
       - Tích hợp kiểm tra chuyển đợt trong `run_turn` và `turn_phase4`.
  3. **Disconnection Evacuation:**
     - Trong `src/web/server_control.rs` (`disconnect_player`):
       - Kiểm tra nếu nhân vật ngắt kết nối tại bản đồ liên tục (10851, 10852, 12911..=12928, 49960..=49969), tự động di dời về `savemap` (hoặc lobby tương ứng) và lưu DB ngay lập tức.

---

### ✅ Ticket 05: Continuous Dungeon 2K & 9 Tiers of Cửu Thiên
- **Tệp ticket:** `.scratch/special-event-refactor/issues/05-continuous-dungeon-2k-and-nine-heavens.md`
- **Các thành phần đã thực hiện:**
  1. **Giải Cứu 2K (Quest 212):**
     - NPC 15312 ("Tả Từ" tại map 12003) trong `src/server/handlers/talk.rs`:
       - Kiểm tra nghiêm ngặt sĩ số tổ đội: chỉ cho phép đội 1 người hoặc 5 người (`1 + conn.session.id_mem.iter().filter(|&&m| m > 0 && m != conn.session.id).count()`).
       - Từ chối với thông báo hệ thống nếu sĩ số 2..4 người: `"Chi cho phep to doi 1 nguoi hoac 5 nguoi"`.
       - Khởi tạo tiến trình tầng 1: `QuestStep(212 = 1)`, phát broadcast alert `14 29 02`, dịch chuyển người chơi (và các thành viên đội) tới map 12911.
  2. **Chinh Phục Cửu Thiên (Quest 12309..12319):**
     - NPC 61209 ("Hầu Tôn" tại map 33033):
       - Kiểm tra thú cưng đi theo là Hầu Tôn (`FollowPet(14735)`).
       - Kiểm tra quota tuần: `QuestStep(12310 == 0)`. Nếu đã đi trong tuần (`step > 0`), từ chối với `"Hien tai van chua den luc..."`.
       - Khi đủ điều kiện: gán `QuestStep(12309 = 1)`, dịch chuyển vào tầng 1 (map 49960).
     - Vật phẩm 46644 ("Cửu Thiên Chiến Kỳ") trong `src/server/handlers/use_item/books.rs`:
       - Tiêu hao vật phẩm, reset quota: gán `QuestStep(12310 = 0)`, phát thông báo `"Da thiet lap lai luot Chinh Phuc Cuu Thien!"`.
  3. **Floor Chest Rewards (NPC 20001):**
     - Tầng cuối Cửu Thiên (map 49969): trao Đậu Dũng Khí (26051) + Lông Vũ (23086), khóa quota `QuestStep(12310 = 1)`.
     - Các tầng trung gian Cửu Thiên (49960..=49968): trao Đậu Dũng Khí (26051).
     - Các rương chốt 2K (map 12927, 12928): trao Lông Vũ (23086), Đậu Dũng Khí (26051), và Giấy Chứng Nhận Tầng (46049 ở 12927, 46050 ở 12928).

---

### ✅ Ticket 06: Competitive Arena Tournaments, Scoreboards, and 7-Day Timed Title Revocation
- **Tệp ticket:** `.scratch/special-event-refactor/issues/06-competitive-arena-and-timed-rewards.md`
- **Mã nguồn:** `src/server/handlers/arena.rs`, `src/server/dispatcher.rs`
- **Các thành phần đã thực hiện:**
  1. **Global Alert & Match Status Broadcasts:**
     - Broadcaster phát frame cảnh báo toàn server: `S→C 25 03 [noticeCode: u8]` (mã 0x01..0x0A) cho các thông báo chuẩn bị giải đấu, bắt đầu và kết thúc trận đấu xuyên suốt 5 chế độ: Solo PK, Đoàn P (5v5), Loạn Đấu, Tiên Đấu, Sinh Tử.
     - Cập nhật trạng thái trận đấu theo thời gian thực: `S→C 14 28 [state: u8]` (1 = bắt đầu, 2 = kết thúc).
  2. **Tournament Scoreboards & Point Exchange:**
     - Bảng điểm lôi đài: Mã hóa mảng động các bản ghi 7 byte (`mode`, `flag`, `x`, title string) qua `S→C 24 09` và kích hoạt giao diện bảng điểm client qua `S→C 24 0A`.
     - Cửa hàng đổi điểm Arena: Xử lý `C→S 17 6C` đổi điểm chiến tích. Nếu điểm của người chơi dưới 50 điểm, kích hoạt `S→C 17 6E` từ chối giao dịch.
  3. **Monday 00:00 Weekly Reset & Deep Item Revocation:**
     - Hàm `process_weekly_arena_reset` tự động quét danh sách người chơi giữ danh hiệu/nhẫn quán quân tuần.
     - Quét triệt để cả **3 containers**:
       - `session.homdo` (kho túi đồ chính)
       - `session.tuideo` (túi đeo phụ)
       - `session.trangbi` (ô trang bị trên người, slot nhẫn/huân chương)
     - Khi phát hiện vật phẩm quán quân hết hạn thuộc dải `23182..=23190` (Nhẫn Loạn Đấu, Nhẫn Đoàn P...) hoặc `46077..=46079` (Văn Chương Vô Giới...):
       - Xóa vật phẩm khỏi container.
       - Nếu vật phẩm bị thu hồi từ `trangbi`: tự động gọi `session.recompute_stats()` để cập nhật lại chỉ số nhân vật.
       - Phát frame thu hồi `S→C 17 6F [01..03]` cho client hiển thị thông báo.
       - Đồng bộ session online qua `registry_update_if_online`.

---

### ✅ Ticket 07: Guild Warfare: City Siege, Water War, and Dragon Boat Racing
- **Tệp ticket:** `.scratch/special-event-refactor/issues/07-guild-warfare-siege-water-boat-race.md`
- **Mã nguồn:** `src/server/handlers/guild.rs`, `src/server/dispatcher.rs`, `src/db/domain.rs`
- **Các thành phần đã thực hiện:**
  1. **Dispatcher Routing & 0x3F Porting:**
     - Khắc phục lỗ hổng định tuyến: `OP_ARENA_WATERWAR` (`0x3F`) trước đó bị bỏ sót trong wildcard `OP_UNIMPLEMENTED`.
     - Cập nhật `src/server/dispatcher.rs` chuyển tiếp `0x3F` tới `guild::handle_water_war`.
  2. **Guild Lifecycle & Two-Phase State Locking (Opcode 0x27):**
     - Tạo bang, mời vào bang (`0x27 0x02`), khai trừ thành viên (`0x27 0x04`), chuyển giao Quân Chủ (`0x27 0x05`), upload biểu tượng bang 288-byte (`0x27 0x06`).
     - Áp dụng Two-Phase State Locking (`lock_player_operations([min(id1, id2), max(id1, id2)])`) trên mọi thao tác 2 người chơi.
     - Sửa lỗi gửi thông báo mời bang: Frame `S→C 27 02 [inviter_id][guild_id]` gửi trực tiếp tới `target_id` qua `hub.send_to(...)`, đồng thời cập nhật `guild_id` vào session online của mục tiêu khi chấp thuận.
     - Kiểm tra quyền hạn nghiêm ngặt: Chỉ Quân Chủ (`guilds.leader_id`) mới có quyền trục xuất thành viên hoặc chuyển quyền Quân Chủ. Khi chuyển giao, cựu Quân Chủ được giáng cấp về rank 0 trước khi phong Quân Chủ mới lên rank 2.
  3. **City Siege 5 Thành Trì (Opcode 0x29):**
     - Quản lý 5 đại thành trì: Thọ Xuân, Trường An, Giang Lăng, Khúc A, Lạc Dương.
     - Đồng bộ quyền chiếm giữ thành trì qua bảng `S→C 29 0F` và 13 thông báo chiến sự qua `S→C 29 09`.
     - Chặn di chuyển qua cổng thành bằng frame va chạm cổng `S→C 03`.
  4. **Water War Thủy Chiến (Opcode 0x3F, 0x40):**
     - Cấp túi đồ thuyền 25 slot chuyên biệt: `C→S 3F 01` khởi tạo túi thuyền `S→C 17 64`; `C→S 3F 02` rời map dọn sạch túi thuyền `S→C 17 65`.
     - Hải pháo ném đá/bắn tên: `C→S 40 01 [action, targetId, param]` tiêu hao đạn dược trong túi thuyền, tính sát thương, đồng bộ hiệu ứng hình ảnh `S→C 40 01..03` và broadcast ra toàn bản đồ.
  5. **Dragon Boat Racing Đua Thuyền Rồng (Opcode 0x46):**
     - Nhận tín hiệu chèo thuyền Trái/Phải luân phiên `C→S 46 01` trên các map 60510..60521.
     - Token-bucket rate limiter: Chặn macro chèo vượt quá 5 lần/giây.
     - Cập nhật tiến trình qua `registry_update_if_online` và broadcast chuyển động thuyền tới tất cả người chơi trong tầm nhìn map.

---

### ✅ Ticket 08: Seasonal Festivals & Static Dat Parity
- **Tệp ticket:** `.scratch/special-event-refactor/issues/08-seasonal-festivals-and-static-dat-parity.md`
- **Mã nguồn:** `src/data/loaders/festival.rs`, `src/server/handlers/festival.rs`
- **Các thành phần đã thực hiện:**
  1. **Delphi Dual XOR Key Binary Parity Cho `Festival.Dat`:**
     - Khắc phục triệt để lỗi đọc nhầm bản mã thô: Thực thi chuẩn xác thuật toán giải mã nhị phân Delphi `FUN_0051e00c` + `FUN_0051decc`.
     - Đọc 2 khối XOR key từ đuôi tệp: `keyA` (record 8, `[n-2]`) và `keyB` (record 9, `[n-1]`).
     - Giải mã 7 bản ghi lễ hội (record 1..=7): Record chẵn giải mã bằng `keyA`, record lẻ giải mã bằng `keyB`.
     - Parse chính xác: `date_time: f64` (`11.0` Delphi TDateTime, hex `0x0000000000002640`), cờ word `0xB4C2`, và chuỗi tiêu đề VISCII 1.1 Pascal ShortString.
  2. **Festival UI Trigger Frame (Opcode 0x24 0x19):**
     - Phát frame `S→C 24 19 [date_dword: 4B LE]` với giá trị `19000110` (`0x0121EAC6`) khớp byte-exact với client Delphi form `*gvar_007DA1EC`.
  3. **Seasonal Announcements Broadcaster:**
     - Phát 74 thông báo sự kiện lễ hội theo lịch mùa qua frame `S→C 17 39 [viscii_string]`.

---

## 2. Ma Trận Kiểm Thử & Nghiệm Thu Toàn Diện (Test Verification Matrix)

Tất cả các test suite từ Ticket 01 đến Ticket 08 đều vượt qua **100% PASS** trên toàn bộ workspace:

| Test Suite File | Số Test | Trạng Thái | Mô Tả Phạm Vi Kiểm Thử |
| :--- | :---: | :---: | :--- |
| **`tests/arena_test.rs`** | **6** | **PASS** | Match status broadcast `14 28`, Scoreboard dynamic array `24 09`, Point exchange dưới 50 rejection `17 6E`, Notice frame 5 chế độ `25 03`, Thu hồi nhẫn tuần Monday 00:00 trên cả `homdo`, `tuideo` và `trangbi` (recompute stats). |
| **`tests/festival_quiz_test.rs`** | **18** | **PASS** | Giải mã nhị phân 7 bản ghi `Festival.Dat` khớp `11.0` TDateTime và `0xB4C2`, Date DWORD parity `19000110`, Thông báo lễ hội `17 39`, Thần tài/Ma `18 08`, Đố vui Bắc Đẩu modal `10 01/02`, Timeout 45s, Anti-spectate `21 01 03`, Tổ đội cô lập và sơ tán an toàn. |
| **`tests/guild_war_test.rs`** | **7** | **PASS** | Tạo bang & mời bang 2-Phase Lock, Phân quyền Quân Chủ chuyển giao & kick, Bảng chiếm đóng 5 thành `29 0F`, Vòng đời túi thuyền 25 slot `17 64/65`, Dispatcher `0x3F` Water War, Đua thuyền rồng rate limiter & map broadcast. |
| **`tests/special_events_dungeon_test.rs`** | **7** | **PASS** | Phụ bản liên tục 40 NPC wave chaining không reload scene, Gating sĩ số 1 hoặc 5 người Giải Cứu 2K, Quota tuần Cửu Thiên & Cửu Thiên Chiến Kỳ, Rương thưởng NPC 20001, Sơ tán an toàn khi ngắt kết nối. |
| **`tests/web_admin_events_test.rs`** | **7** | **PASS** | FSM 5 pha quản lý sự kiện, Starvation Watchdog 120m, Emergency Cascading Drain khi tắt server, Thanh trượt tỷ lệ encounter 1%-30%, Admin audit log. |
| **`tests/architecture_guardrails_test.rs`** | **5** | **PASS** | **Hard Guardrails:** Tuyệt đối không chứa `#[cfg(test)]` trong `src/`; Không handler nào được gọi trực tiếp `lock_online_sessions().insert()`; Tuân thủ chuẩn frame wire parity. |
| **`tests/db_repository_init_test.rs`** | **11** | **PASS** | Khởi tạo bảng, tài khoản, nhân vật, session hydration đầy đủ trường `god` và `ghost`. |
| **`tests/inventory_storage_type_test.rs`** | **6** | **PASS** | Toàn vẹn 5 containers đồ đạc, cách ly `trangbi` 10 byte layout, gửi luulang sequence. |

---

## 3. Tuân Thủ Quy Chuẩn Kiến Trúc & Đồng Thời (Guardrails Compliance)

1. **Two-Phase State Locking:**
   - Mọi thao tác tương tác giữa 2 người chơi (Mời bang `0x27 0x02`, Chuyển giao quyền bang chủ `0x27 0x05`, Thách đấu lôi đài...) đều khóa theo thứ tự `lock_player_operations([min(id1, id2), max(id1, id2)]).await` trước khi đọc session đối phương từ registry.
2. **Đồng Bộ Phiên Trực Tuyến (Online Registry Sync):**
   - Mọi thay đổi trạng thái nhân vật (nhận buff Tiên/Ma, mất túi thuyền khi ra khỏi thủy chiến, nhịp chèo đua thuyền, trừ điểm lôi đài, thu hồi trang bị hết hạn) đều gọi duy nhất `crate::server::session::registry_update_if_online(&conn.session)`.
   - Không có bất kỳ dòng mã nào trong `src/server/handlers/` can thiệp thô bạo vào `lock_online_sessions().insert(...)`.
3. **Dual-Pool SQLite & Phòng Ngừa `SQLITE_BUSY`:**
   - Đọc dữ liệu qua Read Pool (16 kết nối), ghi dữ liệu và chạy transaction nguyên tử qua Write Pool độc quyền (1 kết nối).
4. **Wire Parity & Visual Encoding:**
   - Khung header `F4 44`, giải mã XOR key `0xAD`, text VISCII 1.1 Pascal ShortString và Big5/UTF-8 chuyển đổi chuẩn xác theo đặc tả Delphi client `aLogin.exe`.

---

## 4. Các Lưu Ý & Hướng Đi Tiếp Theo Cho Agent Sau

- **Quản lý dữ liệu giải đấu**: Điểm chiến tích và kết quả giải đấu lôi đài hiện đang được đồng bộ qua bảng session/DB; nếu mở rộng thêm giải đấu liên server, cần xem xét cơ chế pub/sub trung gian.
- **Clock Alignment**: Bộ đếm thời gian reset thứ Hai 00:00 sử dụng UTC. Khi triển khai production ở các múi giờ khác nhau (ví dụ GMT+7), cần cấu hình offset múi giờ phù hợp để người chơi trải nghiệm đúng nửa đêm giờ địa phương.

---

## 5. Suggested Skills Cho Agent Tiếp Theo

Khi tiếp tục phát triển hoặc bảo trì các phân hệ kế tiếp, agent tiếp theo nên ưu tiên sử dụng các skills sau:

- **`code-review`**: Thực hiện rà soát tiêu chuẩn và đặc tả trước khi merge hoặc đóng gói phát hành.
- **`tdd`**: Tiếp tục triển khai test-first đối với các opcode hoặc tính năng mới.
- **`diagnosing-bugs`**: Chẩn đoán nhanh khi phát hiện sai lệch byte wire giữa Delphi client và server.
- **`codebase-design`**: Duy trì tính toàn vẹn và độ sâu của các module domain (như `crafting`, `battle`, `eve`).
