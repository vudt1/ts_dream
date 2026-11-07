# Handoff Document: Hoàn Thành Toàn Diện Phân Hệ Kỹ Năng (Skills, Reborn 2, Job System & Respec - Tickets 01 Đến 10)

**Ngày cập nhật:** 2026-11-03  
**Trạng thái:** Hoàn tất 10/10 tickets, toàn bộ test suite (`19/19` tests) và architecture guardrails (`5/5` tests) đạt 100% pass rate.  
**Phiên làm việc:** Nghiệm thu và hoàn thiện trọn vẹn toàn bộ 10 tickets trong phân hệ Kỹ Năng & Nghề ([`.scratch/understand-skills/issues/`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/)).

---

## 1. Tóm Tắt Toàn Diện Tiến Độ Phân Hệ (Executive Summary)

Phân hệ Kỹ Năng & Nghề (`understand-skills`) đã được hoàn thành trọn vẹn qua 10 vertical slices:

### Giai đoạn 1: Nền tảng cốt lõi (Tickets 01 – 06)
1. **Ticket 01: Core Skill Data Bootstrapping & StatsPacketBuilder Wire Parity**
   - Nạp canonical 376 bản ghi từ `Data/skills.json` vào `GameData.skills`.
   - Xây dựng [`StatsPacketBuilder`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/stats.rs) tạo header 113 byte chuẩn byte-exact cho gói `05 03` (Current SP tại offset 5..6, Skill Points tại 24..25, Reborn tại 96, Job tại 97) kèm danh sách dynamic `{skill_id, level}`.
2. **Ticket 02: Eve Script Skill Rewards Execution**
   - Triển khai nhánh `EveResultClass::Skill = 4` trong [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs), cấp kỹ năng miễn phí không trừ SP, phát frame `08 01 6E 01` và toast hệ thống VISCII 1.1.
3. **Ticket 03: Job Skill Learning Gates & Bulk Skill Tree Commits (`0x1C 01` & `0x1C 04`)**
   - Kiểm tra điều kiện nghiêm ngặt cho 16 kỹ năng nghề theo `job` (1: Hiệp, 2: Bá, 3: Hiền, 4: Tiên) và tiên quyết `14038` Đại chu thiên >= 1.
   - Commit cây kỹ năng hàng loạt (`0x1C 04`) cho cả kỹ năng cơ bản và hero.
4. **Ticket 04: Reborn 2 Dragon Balls & Hero Ball Slots (`0x1C 05`)**
   - Hỗ trợ `dragon_balls` (bitmask 12 bi rồng) và `rb2_slots` (8 slot bóng `skill_rb2`) lưu trữ SQLite và đồng bộ frame `S→C 0x17 0x4D` và `0x17 0x4E`.
5. **Ticket 05: Battle Combat Mechanics Cho 16 Job Skills**
   - Quét nạp passive nghề vào `WarInfo` khi khởi tạo battle grid.
   - Hỗ trợ các kỹ năng nghề: `14040` Bá ý (crit buff), `14042` Sóng cuồng (AOE vật lý), `14044` Thiên thần thuẫn (khiên bảo hộ), `14046` Ngưng khí hộ thân (giảm sát thương), `14053` Thánh Linh thuật (dispel buff), `14054` Càn Khôn Thiên Lôi (toàn map).
6. **Ticket 06: Respec System Cho Kỹ Năng & Thuộc Tính (`OP_RESET = 0x2A`)**
   - Định tuyến Level-1 Dispatcher sang [`src/server/handlers/respec.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs).
   - Hỗ trợ Cẩm nang tâm tư (`46035 / 46166` - tẩy kỹ năng) và Túi gấm cẩn thận (`46203` - tẩy tiềm năng).

### Giai đoạn 2: Gia cố đồng thời, giao dịch nguyên tử & dọn dẹp test seam (Tickets 07 – 08)
7. **Ticket 07: Concurrency, Atomic Transactions, Respec Floor & RAM Rollback Integrity**
   - Thêm cơ chế RAM snapshot rollback: nếu SQLite ghi thất bại, khôi phục nguyên vẹn RAM session, phát mã lỗi Delphi `F4 44 03 00 2A 01 01` / `2A 02 01`.
   - Khống chế sàn chỉ số ban đầu `calculate_starting_base_stat` theo hệ nguyên tố cấp 1.
   - Bọc toàn bộ bulk upgrade `0x1C 04` và Reborn 2 `0x1C 05` trong SQLite write transaction (`pool.write.begin()`).
   - Loại bỏ `tokio::spawn` detached trong xử lý trao kỹ năng Eve.
8. **Ticket 08: Battle Job Skills Mechanics & Test Seam Cleanup**
   - Khai báo 17 hằng số kỹ năng nghề (`SKILL_GREAT_CYCLE`..`SKILL_HEAVEN_THUNDER`, `14038..14054`) trong [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs).
   - Xóa bỏ test seam `apply_buff19_for_test` khỏi `runner.rs`, chuyển toàn bộ test sang public API `run_turn`.

### Giai đoạn 3: Chuẩn hóa quy chuẩn coding, tối ưu Eve loop & chính xác hóa chiến đấu (Tickets 09 – 10)
9. **Ticket 09: Respec Coding Standards & Eve Post-Battle Persistence Integrity**
   - **Loại bỏ chuỗi literal hex**: Thay toàn bộ hex `F44404001709...` trong [`respec.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs) bằng canonical helper [`crate::server::inventory::item_count_changed_frame`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/inventory.rs).
   - **Domain Constants Hệ Nguyên Tố**: Định nghĩa `ELEMENT_EARTH`, `ELEMENT_WATER`, `ELEMENT_FIRE`, `ELEMENT_WIND` trong [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs) và cập nhật `calculate_starting_base_stat`.
   - **Khử trùng lặp vòng lặp Eve**: Hợp nhất logic điều phối bước sự kiện trong [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) qua hàm stepping đơn lẻ `advance_event_step`.
   - **Đảm bảo tính toàn vẹn phần thưởng kỹ năng sau trận đấu**: Truyền `pool` qua [`resume_eve_after_battle`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) tại [`src/battle/service.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/service.rs), bảo vệ kiểm tra `RuntimeFlavor` của Tokio chống crash trên single-threaded runtime.
   - **Bổ sung guardrail test**: [`test_respec_handler_uses_item_count_changed_frame`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/architecture_guardrails_test.rs).
10. **Ticket 10: Battle Holy Spirit Dispel & Combat Cleanups**
    - **Sửa lỗi logic Thánh Linh Thuật (`14053`)**: Không gán đè `type19_id = 14053` lên mục tiêu; chỉ phát frame xóa Aura và status khi mục tiêu thực sự có buff Type 19; giải trừ chuẩn các buff loại 4 và 15.
    - **Scaling Sát Thương Thánh**: Tính toán sát thương theo INT người tấn công: `((attacker.int1 / 2) + (skill_lv * 40)).max(50)`.
    - **Dọn dẹp dead code & helper Sách Động**: Xóa nhánh điều kiện thừa trong `get_skill_targets`; trích xuất helper [`apply_sach_dong_reroll`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs) dùng chung cho các vị trí đổ xúc xắc cứu đòn.

---

## 2. Danh Mục Các Tệp Tin Đã Chỉnh Sửa & Bổ Sung

### Core Engine & Handlers
- [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs): 17 hằng số kỹ năng nghề (`SKILL_GREAT_CYCLE`..`SKILL_HEAVEN_THUNDER`) và 4 hằng số hệ nguyên tố (`ELEMENT_EARTH`..`ELEMENT_WIND`).
- [`src/protocol/codecs/stats.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/stats.rs): `StatsPacketBuilder` đóng gói khung `05 03` byte-exact.
- [`src/server/handlers/respec.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs): Handler tẩy điểm kỹ năng (`2A 01`) và thuộc tính (`2A 02`), snapshot rollback, dùng canonical `item_count_changed_frame`.
- [`src/server/handlers/skills.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/skills.rs): Gates học nghề, transaction commit cho `0x1C 04` và `0x1C 05`.
- [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs): Nhánh `EveResultClass::Skill`, stepping helper thống nhất `advance_event_step`, persist kỹ năng sau trận đấu qua `resume_eve_after_battle`.
- [`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs): Cơ chế buff/dispel chiến đấu, Thánh Linh thuật chuẩn hóa, helper `apply_sach_dong_reroll`, loại bỏ hoàn toàn test seam.
- [`src/battle/service.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/service.rs): Plumbing `DbPool` vào luồng kết thúc trận đấu gọi `resume_eve_after_battle`, bảo vệ `RuntimeFlavor`.
- [`src/battle/construction.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs) & [`src/battle/combat_formula.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/combat_formula.rs): Khởi tạo passive nghề, tính toán giảm sát thương và bonus tỷ lệ hiệu ứng.

### Tests
- [`tests/skill_job_system_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/skill_job_system_test.rs): 19 integration tests toàn diện bao phủ toàn bộ 10 tickets.
- [`tests/architecture_guardrails_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/architecture_guardrails_test.rs): 5 guardrail tests (kiểm tra không inline `#[cfg(test)]`, không gọi `lock_online_sessions().insert()`, dùng `item_count_changed_frame` trong crafting & respec handlers).

---

## 3. Kết Quả Kiểm Thử (Verification Status)

- **Test Suite Kỹ Năng & Nghề:**
  ```bash
  cargo test --test skill_job_system_test
  ```
  **19/19 tests passed (100%)**:
  1. `test_skills_data_loader_canonical_376`
  2. `test_stats_packet_builder_byte_exact_offsets`
  3. `test_spawn_initial_frames_reborn_and_job_wire_parity`
  4. `test_eve_script_skill_reward`
  5. `test_eve_skill_reward_db_failure_rolls_back_ram_and_suppresses_frame`
  6. `test_post_battle_eve_skill_reward_persistence`
  7. `test_job_skill_learning_gates_subcode_01`
  8. `test_bulk_skill_commit_subcode_04_atomic`
  9. `test_bulk_skill_commit_atomicity_on_insufficient_sp`
  10. `test_reborn_2_dragon_balls_and_hero_ball_slots`
  11. `test_reborn_2_rollback_atomicity_on_invalid_hero_skill`
  12. `test_combat_job_skills_passives_and_mechanics`
  13. `test_celestial_shield_ally_targeting_and_reduction_via_battle_runner`
  14. `test_holy_spirit_dispel_integrity_and_damage_scaling`
  15. `test_instigation_status_bonus_mechanics`
  16. `test_sach_dong_reroll_shared_helper`
  17. `test_respec_memory_manual_and_careful_pouch_opcode_0x2a`
  18. `test_respec_attr_floor_enforcement`
  19. `test_respec_db_failure_restores_ram_snapshot_and_preserves_inventory`

- **Architecture Guardrails:**
  ```bash
  cargo test --test architecture_guardrails_test
  ```
  **5/5 tests passed (100%)**.

- **Biên dịch & Chẩn đoán Lint:**
  `cargo check --tests` hoàn thành thành công không có lỗi biên dịch.

---

## 4. Các Điểm Cần Lưu Ý Cho Vận Hành & Phát Triển Tiếp Theo

1. **Working Tree & Commit**:
   - Toàn bộ thay đổi đang được giữ nguyên vẹn trên working tree (nhánh `ss2`).
   - Theo quy định repo ([`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md)), người dùng sẽ kiểm tra và commit thủ công.
2. **Kỹ năng Quang & Ám (Đợt tiếp theo)**:
   - Các kỹ năng hệ Quang (`22001..22022`) và Ám (`23001..23023`) sẽ được triển khai cùng phân hệ Sách Kỹ Năng / Bí Kíp mở rộng trong `src/server/handlers/use_item/books.rs`.
3. **Môi trường Runtime Tokio**:
   - `resume_eve_after_battle` đã được lập trình an toàn với cơ chế fallback nếu chạy trong test đơn luồng (`RuntimeFlavor::CurrentThread`). Trong production server (`main.rs`), Tokio multi-threaded runtime đảm bảo lưu trữ SQLite hoạt động đồng bộ và mượt mà.

---

## 5. Suggested Skills (Kỹ Năng Đề Xuất Cho Agent Tiếp Theo)

Khi bắt đầu phiên làm việc tiếp theo, agent nên kích hoạt các kỹ năng sau:
- **`code-review`**: Rà soát tổng hợp git diff working tree trước khi commit để kiểm tra lại toàn bộ quy chuẩn Two-Phase State Lock và registry sync.
- **`tdd`**: Áp dụng khi tiếp tục xây dựng các tính năng mới (ví dụ: Quest chains phức tạp, Skill Books Quang/Ám, hoặc Map Drop Seed).
- **`diagnosing-bugs`**: Nếu cần chẩn đoán chuyên sâu các vấn đề liên quan đến tương tác gói tin socket TCP thời gian thực hoặc đồ họa hiển thị trên client Delphi `aLogin.exe`.

---

## 6. Danh Mục Tài Liệu & Artifacts Tham Chiếu

- Quy chuẩn lập trình: [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)
- Domain Glossary: [`GLOSSARY.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
- ADR Phân hệ Kỹ năng: [`docs/adr/0008-skill-protocol-parity-and-job-system.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/adr/0008-skill-protocol-parity-and-job-system.md)
- Kế hoạch triển khai Ticket 09 & 10: [`.scratch/understand-skills/plan-tickets-09-10-implementation.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/plan-tickets-09-10-implementation.md)
- Chi tiết 10 Tickets: Thư mục [`.scratch/understand-skills/issues/`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/)
