# Handoff: Hoàn Thành Triển Khai & Nghiệm Thu Ticket 07 và Ticket 08 (Phân Hệ Kỹ Năng)

**Ngày hoàn thành:** 2026-11-02  
**Nhánh:** `ss2` (Mã nguồn nằm trên working tree, người dùng tự commit thủ công theo quy chuẩn repo)  
**Trạng thái:** Hoàn tất 100% Ticket 07 và Ticket 08.  
**Tài liệu tham chiếu:**
- Kế hoạch bàn giao trước đó: [`.scratch/understand-skills/handoff-skills-review-tickets-07-08.md`](.scratch/understand-skills/handoff-skills-review-tickets-07-08.md)
- Đặc tả phân hệ: [`.scratch/understand-skills/spec.md`](.scratch/understand-skills/spec.md)
- Ticket 07: [`.scratch/understand-skills/issues/07-concurrency-atomic-transactions-and-respec-integrity.md`](.scratch/understand-skills/issues/07-concurrency-atomic-transactions-and-respec-integrity.md)
- Ticket 08: [`.scratch/understand-skills/issues/08-battle-job-skills-mechanics-and-test-seam-cleanup.md`](.scratch/understand-skills/issues/08-battle-job-skills-mechanics-and-test-seam-cleanup.md)

---

## 1. Tóm Tắt Kết Quả Triển Khai (Executive Summary)

Phiên làm việc đã giải quyết triệt để toàn bộ các vấn đề được phát hiện trong đợt code review hai trục và hiện thực hóa đầy đủ các yêu cầu của Ticket 07 và Ticket 08:

1. **Ticket 07: Concurrency, Atomic Transactions, Respec Floor & RAM Rollback Integrity**
   - **RAM Snapshot Rollback**: Bổ sung chụp ảnh `Session` snapshot (`before_session`) trước khi thao tác trong `src/server/handlers/respec.rs`. Khi SQLite ghi thất bại, khôi phục nguyên vẹn trạng thái phiên trong RAM (kỹ năng, chỉ số tiềm năng, số lượng vật phẩm trong túi đồ), gửi khung phản hồi lỗi nguyên bản của Delphi client (`F4 44 03 00 2A 01 01` / `F4 44 03 00 2A 02 01`) và không phát khung thành công.
   - **Starting Base Stat Floor**: Bổ sung hàm `calculate_starting_base_stat` chặn không cho người chơi hạ điểm tiềm năng xuống thấp hơn mức sàn ban đầu theo hệ/thuộc tính cấp 1.
   - **Giao dịch nguyên tử (Atomic Transactions)**:
     - Bọc toàn bộ quá trình nâng cấp kỹ năng hàng loạt (`0x1C 04`) và Reborn 2 (`0x1C 05` gồm 3 sections: bi rồng, kỹ năng hero, 8 slot bóng) trong SQLite write transaction (`pool.write.begin()`). Rollback toàn bộ nếu có bất kỳ lỗi nào.
   - **Loại bỏ `tokio::spawn` detached**: Trong `src/server/handlers/talk.rs` (`EveResultClass::Skill = 4`), chuyển đổi lưu DB sang gọi inline `await` với error handling trực tiếp trên pool.
   - **Trích xuất hàm tái sử dụng**:
     - `calculate_skill_upgrade_cost`: Dùng chung cho cả `handle_player_skill_bulk` và `handle_skill_reborn_2`.
     - `deduct_homdo_item_at`: Dùng chung cho trừ vật phẩm tẩy điểm kỹ năng (`46035/46166`) và tẩy điểm thuộc tính (`46203`).
   - **Domain Constants**: Khai báo các hằng số tiếng Anh kèm chú thích tiếng Việt cho respec subcodes và item IDs (`ITEM_MEMORY_MANUAL_PRIMARY`, `ITEM_CAREFUL_POUCH`, `RESPEC_SUB_SKILL`, `RESPEC_STATUS_SUCCESS`, ...).

2. **Ticket 08: Battle Job Skills Mechanics & Test Seam Cleanup**
   - **17 Hằng Số Kỹ Năng Nghề Hệ Tâm**: Khai báo tập trung trong `src/protocol/mod.rs` (`SKILL_GREAT_CYCLE` -> `SKILL_HEAVEN_THUNDER`, ID `14038..14054`) kèm chú thích tiếng Việt.
   - **Thiên Thần Thuẫn (`14044` Celestial Shield)**: Chuẩn hóa nhắm mục tiêu đồng đội hoặc bản thân (`Type 4 targeting`), gắn buff `type19_id = 14044` giảm 50% toàn bộ sát thương nhận vào trong công thức chiến đấu (`calculate_attack`).
   - **Thánh Linh Thuật (`14053` Holy Spirit)**: Chuẩn hóa đơn mục tiêu (`nb_target = 1`, Type 19) theo đúng `Data/Skill.Dat`. Dispel sạch các buff loại 4, 15, 19, phát native packet `purge_status` và `clear_unit_status`, trừ HP qua `apply_target_hp` (xử lý `write_hp` và `fly_out`), hiển thị floating damage.
   - **Sách Động (`14050` Instigation)**: Khởi tạo `sach_dong_lv` trong `WarInfo`, cộng thưởng `sach_dong_lv * 2` vào xác suất trúng hiệu ứng chiến thuật trong `combat_formula.rs` cũng như `runner.rs`.
   - **Dọn Dẹp Test Seam**: Xóa bỏ hoàn toàn hàm production seam `pub fn apply_buff19_for_test` khỏi `src/battle/runner.rs`. Mọi kiểm thử chiến đấu nay đều đi qua public API `run_turn` của `Battle`.
   - **Khử Magic Numbers**: Thay thế toàn bộ mã số kỹ năng cứng trong `construction.rs`, `combat_formula.rs` và `runner.rs` bằng các hằng số domain đã định nghĩa.

---

## 2. Danh Mục Các Tệp Thay Đổi (Changeset Manifest)

| Tệp tin | Loại thay đổi | Chi tiết |
|---|---|---|
| [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs) | Cập nhật | Thêm 17 hằng số kỹ năng nghề (`SKILL_GREAT_CYCLE` đến `SKILL_HEAVEN_THUNDER`) |
| [`src/server/handlers/respec.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs) | Cập nhật | Snapshot rollback, chặn mức sàn `calculate_starting_base_stat`, `deduct_homdo_item_at`, domain constants |
| [`src/server/handlers/skills.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/skills.rs) | Cập nhật | Write transaction cho `0x1C 04` và `0x1C 05`, trích xuất `calculate_skill_upgrade_cost` |
| [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) | Cập nhật | Inline `await` persist DB cho Eve script skill rewards, async hóa event step pipeline |
| [`src/battle/combat_formula.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/combat_formula.rs) | Cập nhật | Thay magic numbers bằng hằng số, giảm 50% damage khi có Celestial Shield, bonus xác suất cho Sách động |
| [`src/battle/construction.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs) | Cập nhật | Thay hằng số nghề, gắn `sach_dong_lv` khi khởi tạo cell trận đấu |
| [`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs) | Cập nhật | Xóa test seam `apply_buff19_for_test`, chuẩn hóa Holy Spirit và Celestial Shield targeting & effects |
| [`tests/skill_job_system_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/skill_job_system_test.rs) | Cập nhật | Test rollback RAM khi DB fail, test chặn mức sàn, test Celestial Shield qua `run_turn`, test Holy Spirit dispel qua `run_turn` |
| [`.scratch/understand-skills/issues/07-concurrency-atomic-transactions-and-respec-integrity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/07-concurrency-atomic-transactions-and-respec-integrity.md) | Cập nhật | Đánh dấu hoàn tất toàn bộ các mục công việc của Ticket 07 |
| [`.scratch/understand-skills/issues/08-battle-job-skills-mechanics-and-test-seam-cleanup.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/08-battle-job-skills-mechanics-and-test-seam-cleanup.md) | Cập nhật | Đánh dấu hoàn tất toàn bộ các mục công việc của Ticket 08 |

---

## 3. Kết Quả Kiểm Thử (Verification Summary)

Toàn bộ các bài test đơn vị và tích hợp liên quan đã được kiểm chứng độc lập:
1. `cargo test --test skill_job_system_test`: **15/15 tests passed** (100% pass rate).
   - `test_skills_data_loader_canonical_376`: Pass
   - `test_stats_packet_builder_byte_exact_offsets`: Pass
   - `test_spawn_initial_frames_reborn_and_job_wire_parity`: Pass
   - `test_eve_script_skill_reward`: Pass
   - `test_job_skill_learning_gates_subcode_01`: Pass
   - `test_bulk_skill_commit_subcode_04_atomic`: Pass
   - `test_bulk_skill_commit_atomicity_on_insufficient_sp`: Pass
   - `test_reborn_2_dragon_balls_and_hero_ball_slots`: Pass
   - `test_reborn_2_rollback_atomicity_on_invalid_hero_skill`: Pass
   - `test_combat_job_skills_passives_and_mechanics`: Pass
   - `test_respec_memory_manual_and_careful_pouch_opcode_0x2a`: Pass
   - `test_respec_attr_floor_enforcement`: Pass
   - `test_respec_db_failure_restores_ram_snapshot_and_preserves_inventory`: Pass
   - `test_celestial_shield_ally_targeting_and_reduction_via_battle_runner`: Pass
2. `cargo test --test architecture_guardrails_test`: **4/4 passed**
   - Không còn inline `#[cfg(test)]` trong `src/`.
   - Không còn gọi trực tiếp `lock_online_sessions().insert()` trong handlers.
   - Không phát sinh vi phạm quy chuẩn kiến trúc repo.

---

## 4. Hướng Dẫn Tiếp Theo Cho Người Dùng (Next Steps)

1. **Kiểm tra working tree**:
   ```bash
   git status
   git diff --stat
   ```
2. **Commit thủ công**:
   Toàn bộ thay đổi đang ở working tree trên nhánh `ss2`, sẵn sàng để người dùng thực hiện commit theo thông điệp chuẩn của dự án:
   ```bash
   git add src/ tests/ .scratch/
   git commit -m "feat(skills): implement tickets 07 and 08 (transactions integrity, battle mechanics, test seam cleanup)"
   ```
