# HANDOFF: Phân Hệ Sự Kiện Đặc Biệt (Special Events) - Hoàn Thành Tickets 01, 02, 03, 04, 05

* **Dự án:** `ts_dream` (Rust Game Server parity với Client TS Online `aLogin.exe`)
* **Thời điểm bàn giao:** 2026-11-05
* **Đích đến:** `.scratch/special-event-refactor/handoff-tickets-01-04-05.md`
* **Trạng thái:** Đã hoàn thành 100% việc thực thi và kiểm thử cho **Ticket 01, 02, 03, 04, 05**. Toàn bộ 30 test liên quan (Unit, Integration, Architecture Guardrails) đều vượt qua (100% Pass). Sẵn sàng để agent tiếp theo bắt tay vào Ticket 06, 07 hoặc 08.

---

## 1. Tóm Tắt Toàn Bộ Các Ticket Đã Hoàn Thành

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

## 2. Kết Quả Kiểm Thử (Verification Record)

Toàn bộ **50 tests** tích hợp, unit và guardrails trên toàn bộ workspace đều vượt qua 100% (`cargo test --all-targets --no-fail-fast`):

1. **`tests/festival_quiz_test.rs`** (14 tests):
   - `test_db_persistence_god_ghost`: VƯỢT QUA
   - `test_player_appear_god_ghost_offsets`: VƯỢT QUA
   - `test_state_sync_frame_structure`: VƯỢT QUA
   - `test_npc_1752_exorcism_dialog`: VƯỢT QUA
   - `test_consumable_46180_cai_van_phu`: VƯỢT QUA
   - `test_combat_formula_god_ghost_exp`: VƯỢT QUA
   - `test_quiz_modal_frame_emission_and_viscii_layout`: VƯỢT QUA
   - `test_anti_spectate_rejection_frame`: VƯỢT QUA
   - `test_quiz_correct_answer_awards_god_buff`: VƯỢT QUA
   - `test_quiz_incorrect_answer_penalizes_ghost_and_hp1`: VƯỢT QUA
   - `test_quiz_45s_timeout_resolves_incorrect`: VƯỢT QUA
   - `test_quiz_party_leader_modal_and_member_isolation`: VƯỢT QUA
   - `test_quiz_party_members_evacuated_on_answer`: VƯỢT QUA
   - `test_quiz_party_members_evacuated_on_leader_disconnect`: VƯỢT QUA

2. **`tests/web_admin_events_test.rs`** (7 tests):
   - `test_stopped_server_rejection_gate_returns_409`: VƯỢT QUA
   - `test_strict_mutex_duplicate_start_returns_409`: VƯỢT QUA
   - `test_graceful_stop_and_starvation_watchdog`: VƯỢT QUA
   - `test_cascading_emergency_stop_on_tcp_shutdown`: VƯỢT QUA
   - `test_north_star_encounter_rate_slider_api`: VƯỢT QUA
   - `test_encounter_rate_audit_log_persisted`: VƯỢT QUA
   - `test_admin_audit_log_persistence`: VƯỢT QUA

3. **`tests/special_events_dungeon_test.rs`** (7 tests):
   - `test_40_npc_entry_gating`: VƯỢT QUA
   - `test_in_engine_wave_chaining`: VƯỢT QUA
   - `test_rescue_2k_party_size_gating`: VƯỢT QUA
   - `test_rescue_2k_floor_rewards`: VƯỢT QUA
   - `test_cuu_thien_gating_and_quota_reset`: VƯỢT QUA
   - `test_cuu_thien_floor_chest_and_final_quota`: VƯỢT QUA
   - `test_dungeon_disconnect_evacuation_to_savemap`: VƯỢT QUA

4. **`tests/architecture_guardrails_test.rs`** (5 tests):
   - `test_crafting_handler_uses_item_count_changed_frame`: VƯỢT QUA
   - `test_no_legacy_aliases_in_compound_def`: VƯỢT QUA
   - `test_respec_handler_uses_item_count_changed_frame`: VƯỢT QUA
   - `test_no_direct_lock_online_sessions_insert_in_handlers`: VƯỢT QUA
   - `test_no_inline_cfg_test_in_src`: VƯỢT QUA

5. **`tests/db_repository_init_test.rs`** (10 tests):
   - Khắc phục lỗi thiếu trường `ghost: 0` gây compile error trong cấu trúc `Session`. Toàn bộ 10 tests đều vượt qua.

6. **`tests/battle_finish_generalization_test.rs`** (10 tests) & **`tests/battle_levelup_frames_test.rs`** (7 tests):
   - Khắc phục lỗi thiếu trường `ghost: 0, god: 0` trong `PlayerSnapshot`. Toàn bộ 17 tests đều vượt qua.

---

## 3. Bản Đồ Frontier Cho Agent Tiếp Theo

Các ticket còn lại trong phân hệ Special Events (`.scratch/special-event-refactor/issues/`):

- **Ticket 06:** `06-competitive-arena-and-timed-rewards.md` (Lôi Đài, bảng điểm 7-byte và thu hồi nhẫn 7 ngày `17 6F`).
- **Ticket 07:** `07-guild-warfare-siege-water-boat-race.md` (Công Thành Chiến, Thủy Chiến túi thuyền 25 slot, Đua Thuyền Rồng).
- **Ticket 08:** `08-seasonal-festivals-and-static-dat-parity.md` (`Festival.Dat` date parity `19000110` và thông báo `17 39`).

### 🎯 Điểm Bắt Đầu Đề Xuất: Ticket 06
- **Tệp ticket:** `.scratch/special-event-refactor/issues/06-competitive-arena-and-timed-rewards.md`
- Phụ thuộc: Đã có sẵn hạ tầng `EventLifecycleManager` (Ticket 03) và `BattleService` wave/arena logic.
