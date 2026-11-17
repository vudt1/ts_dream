# HANDOFF: Đợt Refactor PKBattle & Flee (Tickets 26 → 28)

- **Ngày cập nhật**: 2026-11-17
- **Dự án**: `ts_dream` (Rust game server cho TS Online PC `aLogin.exe`)
- **Đặc tả & Nghiên cứu nền tảng**:
  - [spec-pk-and-flee.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec-pk-and-flee.md)
  - [research-pk-and-flee.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/research-pk-and-flee.md)
  - [ADR 0006 §2.5–2.6](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md)
  - [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
- **Trạng thái Git**: Toàn bộ thay đổi Ticket 26 và Ticket 27 nằm ở working tree (**chưa commit** theo quy chuẩn [`AGENTS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md) — user tự commit thủ công).

---

## 1. Trạng Thái Tiến Độ Đợt Tickets 26 → 28

| # | Ticket | Trạng thái | Tóm tắt phạm vi |
|:-:|---|:-:|---|
| 26 | [`issues/26-battle-kind-npc-pk-and-0bfa-roles.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/26-battle-kind-npc-pk-and-0bfa-roles.md) | ✅ `completed` | `BattleKind` (`Normal`/`Pk`), sửa `0x0B 02 03` (`npc_on_map_id`, `DiaHinh=112`, luôn `PKBattle`), chặn thưởng/trừ `Stat::God` giữ `15001`/`npc_respawn`, chuẩn hóa `0B FA` (`0x03`/`0x05`/`0x04`) & khóa `0x0B 01 03` cho *ListQS* |
| 27 | [`issues/27-pvp-party-symmetry-and-flee-formula-18002.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/27-pvp-party-symmetry-and-flee-formula-18002.md) | ✅ `completed` | PvP Party 2 phe đối xứng (`0x0B 02 02`), guard thách đấu PK trên người chơi, cleanup Phe 2 trong `Battle::finish`, công thức Flee LV `+20%` (`14002`), chặn NPC dùng Flee, fail luôn phát `18002` |
| 28 | [`issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md) | 🔜 `ready-for-agent` | Ma trận Flee 4 vai trò tổ đội (PvE + PvP 2 phe), sửa bug Zombie Cell trong `execute_entity`, bỏ `npc_respawn`/trùng `reset_cell` ở member flee, gỡ người trốn khỏi `battle_participants` |

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

## 3. Chi Tiết Đã Hoàn Thành Ở Ticket 27

### 3.1. PvP Party hai phe đối xứng (`0x0B 02 02`), Guard PK & Cleanup 2 phe
- [`src/server/handlers/battle.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs):
  - `handle_pk_challenge` (`0x0B 02 02`): bổ sung guard chặn tự PK chính mình, người thách đấu đang *ListQS* / đang mở *Sạp hàng*, đối thủ khác bản đồ, cùng tổ đội (`id_leader` / `id_mem`), mục tiêu thuộc nhóm GM (`gm_level > PLAYER_GM_LEVEL`), đang trong trận / *ListQS*, hoặc đang mở *Sạp hàng*; giữ kiểm tra Option PK (`!target.pk` phát `F4440300210101`). Không kiểm tra Safe Zone trên server.
- [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs):
  - Thêm `PkTargetInfo` + `BattleService::pk_target_info(&self, player: i64)`.
  - Thêm `ExtraBattleParticipants`, `load_pvp_side`, `add_pvp_member` và cập nhật `start_pk_battle` để nạp đầy đủ Đội trưởng, Thành viên tổ đội cùng bản đồ (`cols = [1, 3, 0, 4]`) và *Sủng vật* của cả Phe 1 (hàng `3/2`) lẫn Phe 2 (hàng `0/1`) với `BattleKind::Pk`, đồng bộ `battle_id` lên cả `self.online` lẫn `lock_online_sessions()`.
  - Cập nhật `spawn_battle` để nạp `player_skills` cho tất cả `extra_members` (bao gồm Phe 2) và chọn `battle.pk_battle_start_packets(diahinh)` khi `battle.is_pvp()`.
- [`src/battle/construction.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs):
  - Thêm `Battle::is_pvp(&self) -> bool`, `Battle::pk_battle_start_packets(&self, diahinh: i32)`, và các helper `side_battle_start_packets` / `entity_frame_for_cell` (Đội trưởng hai phe nhận `0x03`, Thành viên hai phe nhận `0x05`, phát `0B 05 05` cho cả Phe 1 và Phe 2).
  - Cập nhật `member_battle_frame` để quét hàng đối thủ đối xứng (`0..2` cho hàng `3`, `2..4` cho hàng `0`) và phát `entity_player` (`0B 05 05`) cho thực thể người chơi/sủng vật (`typ ∈ {2, 4}`).
- [`src/battle/runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs):
  - `Battle::finish`: mở rộng vòng duyệt người chơi sang cả Phe 1 (`team == 1`) lẫn Phe 2 (`team == 2`): hồi `Hp = 1` nếu tử trận (`hp <= 0`), phát `0B 00`, `0B 01`, `05 04`, `14 08`; vẫn chỉ cấp Player EXP / Pet EXP / trừ `Stat::God` khi `self.kind.allows_rewards() && cell.team == 1`.

### 3.2. Công thức Chạy trốn LV +20% (`14002`), Guard NPC & Gói thất bại `18002`
- [`src/battle/damage.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs):
  - Thêm `get_random_miss_flee_with_skill(rng, lv1, lv2, lvtb1, lvtb2, skill_id)` với công thức `clamp(60 + (lv1 - lv2) + (lvtb1 - lvtb2) + spell_bonus, 0, 100)`, trong đó `spell_bonus = 20` cho `14002` (*Thuật đào tẩu*) và `0` cho `18001` (*Chạy trốn*); `get_random_miss_flee` ủy quyền về `skill_id = 18001`.
- [`src/battle/runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs):
  - `Battle::apply_flee`: chặn NPC khỏi nhánh Flee (`if attacker.typ != 2 && attacker.typ != 4 { return Outcome::Running; }`), dùng `damage::get_random_miss_flee_with_skill(..., skill)`, bỏ hoàn toàn nhánh auto-win `|| skill == 14002`, và khi Flee thất bại (cả `18001` và `14002`) luôn lookup `&18002` (fallback `(1, 12)`) phát frame `32 01` với `skill_id = 18002`, `skill_type = 12`, `miss = 0`, `adl = 2` (không phát `14003`).

### 3.3. Kiểm thử & Kết quả xác minh sau chỉnh sửa cuối (Ticket 27)
- Test mới: [`tests/battle/pvp_party_and_flee.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/pvp_party_and_flee.rs) (đăng ký trong [`tests/battle_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle_test.rs)) gồm 6 tests phủ công thức Flee `+20%` (`14002`), frame thất bại `18002`, guard NPC `typ = 3`, `Battle::finish` dọn dẹp 2 phe, các guard thách đấu `0x0B 02 02`, và PvP Party E2E.
- Test cập nhật: [`tests/eve_quest/random_encounter.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/random_encounter.rs) gỡ mock giả `14003` và cập nhật `level: 60` trong `test_player_flee_synchronization_and_cooldown` & `test_party_member_individual_flee` cho phù hợp công thức `+20%`.
- Kết quả chạy `cargo test --all-targets --no-fail-fast` sau chỉnh sửa cuối cùng:
  - `architecture_guardrails_test`: **8 passed**
  - `battle_test`: **179 passed**
  - `compound_test`: **21 passed**
  - `data_loaders_test`: **36 passed**
  - `db_test`: **21 passed**
  - `economy_inventory_test`: **54 passed**
  - `eve_quest_test`: **130 passed**
  - `events_arena_test`: **61 passed**
  - `server_network_test`: **94 passed**
  - `server_stop_test`: **1 passed**
  - `skill_job_system_test`: **19 passed**
  - **Tổng cộng: 624 passed, 0 failed, 0 ignored.**

---

## 4. Lưu Ý Quan Trọng Cho Ticket 28 (Từ Code Review & Kiến Trúc)

1. **Về `npc_respawn` trong `0x0B 02 03` standalone**:
   - Theo Ticket 26, `npc_respawn` hoạt động bình thường khi `data.talking_battle > 0` (`session.talking_battle`). Trong `handle_attack_npc` (`0x0B 02 03`), cố ý **không** ghi đè `session.talking_battle = npc_on_map_id` vì `battle_ended` dùng `s.talking_battle > 0` để kích hoạt `battle_quest_win` / `quest_lose_frames` (`src/battle/service.rs:501`). Nếu về sau cần hồi sinh NPC map khi PK trực tiếp ngoài thoại quest, phải tách một trường riêng (ví dụ `map_npc_respawn_slot`) thay vì dùng chung `session.talking_battle`.
2. **Về `handle_battle` dispatcher signature**:
   - `handle_battle` vẫn giữ chữ ký sync hiện có theo kiến trúc dispatcher (không đổi toàn bộ caller sang async).

---

## 5. Frontier & Kế Hoạch Triển Khai Tiếp Theo

1. **Ticket 28** ([`issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/issues/28-party-pvp-flee-matrix-and-zombie-cell-fix.md)) — `ready-for-agent`:
   - Xử lý ma trận Flee 4 vai trò (PvE + PvP 2 phe): Leader / Pet của Leader (bất kỳ phe nào) trốn thành công → `Outcome::PlayerFled` kết thúc sạch cả trận; Member / Pet của Member → chỉ rút toàn bộ ô thuộc `owner`.
   - Sửa bug **Zombie Cell** trong `Battle::execute_entity` (không ghi đè lại `attacker` / `target` lên lưới sau khi `apply_flee` đã xóa ô), bỏ `npc_respawn` và bỏ `reset_cell` trùng ở nhánh Member flee, mở rộng `Out::Fled { battle_id, player }` để gỡ khỏi `battle_participants`.

---

## 6. Các Ràng Buộc Bất Biến & Kỹ Năng Đề Xuất

- **Ràng buộc bất biến** ([`AGENTS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md), [`CODING_STANDARDS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)):
  - Mọi test mới đặt làm submodule dưới `tests/<domain>/`; **cấm** `#[cfg(test)]` trong `src/` và không tạo file `*_test.rs` lẻ ở root `tests/`.
  - Không đụng `DB/ts_dream.db`; test DB dùng `tests/common/test_db.rs`.
  - **Không tự ý `git commit`** (giữ thay đổi ở working tree để user commit thủ công).
  - Trong phát triển chạy `cargo test --test battle_test`; trước handoff cuối chạy `cargo test --all-targets --no-fail-fast`.
- **Kỹ năng (Skills) đề xuất cho agent kế tiếp**:
  - `tdd` — triển khai từng vertical slice (ma trận Flee 4 vai trò PvE/PvP, Zombie Cell multi-turn, `Out::Fled` sync) ở seam đã thống nhất trong `spec-pk-and-flee.md`.
  - `code-review` — rà soát song song Standards + Spec sau khi hoàn tất Ticket 28.
  - `domain-modeling` — giữ đồng bộ thuật ngữ (`PKBattle`, `ListQS`, `DiaHinh`, *Ô chiến trận*) với `GLOSSARY.md` và `ADR 0006`.
