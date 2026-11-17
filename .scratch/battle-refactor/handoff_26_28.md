# HANDOFF: Đợt Refactor PKBattle & Flee (Tickets 26 → 28)

- **Ngày cập nhật**: 2026-11-17
- **Dự án**: `ts_dream` (Rust game server cho TS Online PC `aLogin.exe`)
- **Đặc tả & Nghiên cứu nền tảng**:
  - [spec-pk-and-flee.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec-pk-and-flee.md)
  - [research-pk-and-flee.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/research-pk-and-flee.md)
  - [ADR 0006 §2.5–2.6](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md)
  - [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
- **Trạng thái Git**: Toàn bộ thay đổi Ticket 26 nằm ở working tree (**chưa commit** theo quy chuẩn [`AGENTS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md) — user tự commit thủ công).

---

## 1. Trạng Thái Tiến Độ Đợt Tickets 26 → 28

| # | Ticket | Trạng thái | Tóm tắt phạm vi |
|:-:|---|:-:|---|
| 26 | [`issues/26-battle-kind-npc-pk-and-0bfa-roles.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/26-battle-kind-npc-pk-and-0bfa-roles.md) | ✅ `completed` | `BattleKind` (`Normal`/`Pk`), sửa `0x0B 02 03` (`npc_on_map_id`, `DiaHinh=112`, luôn `PKBattle`), chặn thưởng/trừ `Stat::God` giữ `15001`/`npc_respawn`, chuẩn hóa `0B FA` (`0x03`/`0x05`/`0x04`) & khóa `0x0B 01 03` cho *ListQS* |
| 27 | [`issues/27-pvp-party-symmetry-and-flee-formula-18002.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/27-pvp-party-symmetry-and-flee-formula-18002.md) | 🔜 `ready-for-agent` | PvP Party 2 phe đối xứng (`0x0B 02 02`), guard thách đấu PK trên người chơi, cleanup Phe 2 trong `Battle::finish`, công thức Flee LV `+20%` (`14002`), chặn NPC dùng Flee, fail luôn phát `18002` |
| 28 | [`issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md) | ⏸ `blocked by 27` | Ma trận Flee 4 vai trò tổ đội (PvE + PvP 2 phe), sửa bug Zombie Cell trong `execute_entity`, bỏ `npc_respawn`/trùng `reset_cell` ở member flee, gỡ người trốn khỏi `battle_participants` |

---

## 2. Chi Tiết Đã Hoàn Thành Ở Ticket 26

### 2.1. Phân loại `BattleKind` & Hạn chế phần thưởng `PKBattle`
- [`src/battle/construction.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs):
  - Thêm `pub enum BattleKind { #[default] Normal, Pk }` tách biệt hoàn toàn khỏi `Battle::fight_type` của Eve.
  - Thêm helper `BattleKind::allows_rewards(self) -> bool` (`Normal => true`, `Pk => false`) và builder `Battle::with_kind(mut self, kind: BattleKind) -> Self`; mặc định `Battle::new` gán `kind: BattleKind::Normal`.
- [`src/battle/runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs):
  - Cổng `if !self.kind.allows_rewards() { return; }` tại `Battle::note_npc_hit` (không ghi `killed_npcs`, không cộng `attacker.exp`).
  - Cổng `if !self.kind.allows_rewards()` tại `Battle::process_kills` (dọn `killed_npcs` & `combo_cells`, chặn `Out::Drop`, `35 04`, `Out::Gold`, `reward_exp`, `calc_combo_exp`).
  - Cổng `self.kind.allows_rewards()` tại `Battle::finish` cho Player EXP (`Stat::Texp` + `push_level_up_rewards`), trừ `Stat::God`, và Pet EXP (`Out::PetExp` / `0F 08`).
  - Giữ nguyên toàn bộ combat/status/combo, `Battle::apply_catch` (`15001`), cleanup rời trận (`0B 00`, `0B 01`, `05 04`, `14 08`, hồi `Hp = 1` nếu `<= 0`), và `Battle::npc_respawn` (khi `data.talking_battle > 0`).

### 2.2. Sửa `0x0B 02 03` (PK Võ tướng trên bản đồ) & Định tuyến khởi tạo
- [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs):
  - Thêm `BattleService::start_npc_battle_with_kind(&self, session, npc_id, npc_on_map_id, ground, kind) -> i32`; `start_npc_battle` ủy quyền về `BattleKind::Normal`.
  - Gán `.with_kind(BattleKind::Pk)` trong `BattleService::start_pk_battle`.
  - Giữ nguyên `BattleKind::Normal` cho toàn bộ PvE (`start_npc_battle`, `start_encounter_battle`, `start_eve_encounter`, `start_teamdef_battle`, `start_teamdef_battle_seeded`, `start_40_npc_battle`, `start_north_star_battle`).
- [`src/server/handlers/battle.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs):
  - `handle_attack_npc` (`0x0B 02 03`) đọc 2 byte cuối `payload[5..7]` làm `npc_on_map_id` (mặc định `0` nếu thiếu byte), luôn dùng `trigger_diahinh::PK_AND_NPC` (`112`), và luôn khởi tạo `BattleKind::Pk` bất kể `conn.session.pk == 0` hay `1`.

### 2.3. Chuẩn hóa dấu vai trò `0B FA` (`0x03`/`0x05`/`0x04`) & Cổng `0x0B 01 03` cho *ListQS*
- [`src/battle/packets.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/packets.rs):
  - Định nghĩa hằng số `BATTLE_ROLE_LEADER = 0x03`, `BATTLE_ROLE_SPECTATOR = 0x04`, `BATTLE_ROLE_MEMBER = 0x05`.
  - `packets::battle_open_leader` phát self marker `0x03` (`BATTLE_ROLE_LEADER`).
- [`src/battle/construction.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs):
  - `Battle::member_battle_frame` phát self marker `0x05` (`BATTLE_ROLE_MEMBER`), giữ `03`/`64` cho các ô đồng đội không phải bản thân.
- [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) & [`src/server/handlers/battle.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs):
  - `BattleService::build_join_frame` giữ self marker `0x04` (`BATTLE_ROLE_SPECTATOR`) cho *ListQS*.
  - Bổ sung theo dõi người quan chiến `spectators: Arc<Mutex<HashMap<i64, i32>>>` trong `BattleSinkImpl`, cập nhật tại `join_battle`, `unregister`, `leave_battle`, `battle_ended`; xuất `BattleService::is_spectator(&self, player: i64) -> bool` và `BattleService::leave_spectate(&self, session: &mut Session) -> bool`.
  - Trích xuất helper `BattleService::sync_online_session` dùng chung cho `join_battle` và `leave_battle`.
  - `handle_leave_battle` (`0x0B 01 03`, với `LEAVE_BATTLE_CONFIRM = 3`) chỉ cho phép *ListQS* thoát qua `service.leave_spectate(&mut conn.session)` (xóa khỏi `spectators`, `members`, `battle_participants`), từ chối chiến đấu viên đang tham chiến (giữ nguyên `battle_id`, không phát khung thoát trận).

### 2.4. Kiểm thử & Kết quả xác minh sau chỉnh sửa cuối
- Test mới: [`tests/battle/battle_kind_and_roles.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/battle_kind_and_roles.rs) (đăng ký trong [`tests/battle_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle_test.rs)) gồm 6 tests phủ tầng engine (`BattleKind`, thưởng/God, capture `15001`, `npc_respawn`) và tầng handler/service (`0x0B 02 03`, PvE `Normal`, `0B FA` `03`/`05`/`04`, `0x0B 01 03` *ListQS*-only).
- Test hợp đồng cập nhật có chủ đích:
  - [`tests/battle/open_leader.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/open_leader.rs) (`04` → `03`)
  - [`tests/battle/open_self_kind.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/open_self_kind.rs) (`03` leader / `05` member / `04` ListQS spectator)
  - [`tests/battle/command_channel.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/command_channel.rs) (`sub1_confirm_three_leaves_only_for_spectator_and_rejects_combatant`)
- Kết quả chạy `cargo test --all-targets --no-fail-fast` sau chỉnh sửa cuối cùng:
  - `architecture_guardrails_test`: **8 passed**
  - `battle_test`: **173 passed**
  - `compound_test`: **21 passed**
  - `data_loaders_test`: **36 passed**
  - `db_test`: **21 passed**
  - `economy_inventory_test`: **54 passed**
  - `eve_quest_test`: **130 passed**
  - `events_arena_test`: **61 passed**
  - `server_network_test`: **94 passed**
  - `server_stop_test`: **1 passed**
  - `skill_job_system_test`: **19 passed**
  - **Tổng cộng: 618 passed, 0 failed, 0 ignored.**

---

## 3. Lưu Ý Quan Trọng Cho Tickets 27 & 28 (Từ Code Review & Kiến Trúc)

1. **Đối với Ticket 27 (`start_pk_battle` & Team 2 PvP)**:
   - Hiện tại `BattleService::start_pk_battle` ([`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs)) vẫn đang gọi `battle.member_battle_frame(terrain, opponent)` cho đối thủ Phe 2 (nên sau Ticket 26, đối thủ Phe 2 đang nhận dấu `0x05` thay vì `0x03`, chưa có Pet/Party Phe 2, chưa có `battle_id`/skills cho Phe 2, và `spawn_battle` còn gọi `npc_battle_start_packets` phát `0B 05 03` cho hàng `0..2`).
   - Ticket 27 cần xây dựng `Battle::pk_battle_start_packets` đối xứng hai phe (Đội trưởng hai phe nhận `0x03`, Thành viên hai phe nhận `0x05`, phát `0B 05 05` cho cả Phe 1 hàng `3/2` lẫn Phe 2 hàng `0/1`), đồng bộ `battle_id` & `player_skills` cho mọi thành viên cả 2 phe, thêm các guard thách đấu PK (`self`, khác `map_id`, cùng `id_leader`, GM, *ListQS*, đang bày *Sạp hàng*), và sửa `Battle::finish` duyệt `cell.typ == 2 && cell.id > 0` cho cả `team == 1` và `team == 2`.
2. **Về `npc_respawn` trong `0x0B 02 03` standalone**:
   - Theo Ticket 26, `npc_respawn` hoạt động bình thường khi `data.talking_battle > 0` (`session.talking_battle`). Trong `handle_attack_npc` (`0x0B 02 03`), cố ý **không** ghi đè `session.talking_battle = npc_on_map_id` vì `battle_ended` dùng `s.talking_battle > 0` để kích hoạt `battle_quest_win` / `quest_lose_frames` (`src/battle/service.rs:501`). Nếu về sau cần hồi sinh NPC map khi PK trực tiếp ngoài thoại quest, phải tách một trường riêng (ví dụ `map_npc_respawn_slot`) thay vì dùng chung `session.talking_battle`.
3. **Cập nhật test `eve_quest_test` ở Ticket 27**:
   - `tests/eve_quest/random_encounter.rs` (`test_player_flee_synchronization_and_cooldown` & `test_party_member_individual_flee`) vẫn đang chứa mock/kỳ vọng cũ cho `14002` thắng 100% và `14003`; Ticket 27 cần gỡ mock giả này và chạy `cargo test --test eve_quest_test` cùng `cargo test --test battle_test`.

---

## 4. Frontier & Kế Hoạch Triển Khai Tiếp Theo

1. **Ticket 27** ([`issues/27-pvp-party-symmetry-and-flee-formula-18002.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/27-pvp-party-symmetry-and-flee-formula-18002.md)) — `ready-for-agent`:
   - Hoàn thiện `handle_pk_challenge` guards + `start_pk_battle` đối xứng Phe 1 (row `3/2`) & Phe 2 (row `0/1`) + `Battle::pk_battle_start_packets` + `Battle::finish` cleanup 2 phe.
   - Thêm `damage::get_random_miss_flee_with_skill` (`clamp(60 + (lv1 - lv2) + (lvtb1 - lvtb2) + bonus, 0, 100)`, `+20` cho `14002`), bỏ `|| skill == 14002`, chặn NPC `typ ∉ {2, 4}` khỏi `apply_flee`, sửa fail Flee luôn phát `18002` (`skill_type = 12`).
2. **Ticket 28** ([`issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md)) — thực hiện sau Ticket 27:
   - Xử lý ma trận Flee 4 vai trò (PvE + PvP 2 phe): Leader / Pet của Leader (bất kỳ phe nào) trốn thành công → `Outcome::PlayerFled` kết thúc sạch cả trận; Member / Pet của Member → chỉ rút toàn bộ ô thuộc `owner`.
   - Sửa bug **Zombie Cell** trong `Battle::execute_entity` (không ghi đè lại `attacker` / `target` lên lưới sau khi `apply_flee` đã xóa ô), bỏ `npc_respawn` và bỏ `reset_cell` trùng ở nhánh Member flee, mở rộng `Out::Fled { battle_id, player }` để gỡ khỏi `battle_participants`.

---

## 5. Các Ràng Buộc Bất Biến & Kỹ Năng Đề Xuất

- **Ràng buộc bất biến** ([`AGENTS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md), [`CODING_STANDARDS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)):
  - Mọi test mới đặt làm submodule dưới `tests/<domain>/`; **cấm** `#[cfg(test)]` trong `src/` và không tạo file `*_test.rs` lẻ ở root `tests/`.
  - Không đụng `DB/ts_dream.db`; test DB dùng `tests/common/test_db.rs`.
  - **Không tự ý `git commit`** (giữ thay đổi ở working tree để user commit thủ công).
  - Trong phát triển chạy `cargo test --test battle_test` (và `--test eve_quest_test` ở Ticket 27); trước handoff cuối chạy `cargo test --all-targets --no-fail-fast`.
- **Kỹ năng (Skills) đề xuất cho agent kế tiếp**:
  - `tdd` — triển khai từng vertical slice (PvP guards/symmetry, Flee formula/`18002`, Zombie Cell multi-turn) ở 2 seam đã thống nhất trong `spec-pk-and-flee.md`.
  - `code-review` — rà soát song song Standards + Spec sau khi hoàn tất Ticket 27 và Ticket 28.
  - `domain-modeling` — giữ đồng bộ thuật ngữ (`PKBattle`, `ListQS`, `DiaHinh`, *Ô chiến trận*) với `GLOSSARY.md` và `ADR 0006`.
