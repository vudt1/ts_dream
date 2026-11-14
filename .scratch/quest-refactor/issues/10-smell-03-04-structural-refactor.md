# 10: Refactor Cấu trúc `Smell-03` & `Smell-04` — Type-Safe `DomainEffect` / `EventTriggerKind` & Tách `eve_runner.rs`

**What to build:** Hoàn thiện nốt 2 hạng mục refactor cấu trúc được trì hoãn từ đợt Code Review (`CODE_REVIEW_01_08.md`) nhằm loại bỏ trùng lặp rẽ nhánh giữa Planner/Pre-flight/Executor (`Smell-03`) và tách trách nhiệm cho `src/server/handlers/talk.rs` (`Smell-04`), đảm bảo giữ nguyên 100% hành vi wire, bảo toàn tương thích ngược qua re-export, và không phá vỡ bất kỳ test nào.

**Blocked by:** `09: Khắc phục Toàn diện Phát hiện Code Review (Tickets 01–08)`

**Status:** resolved

## Acceptance Criteria

### 1. Loại bỏ `raw_result: &EveResult` & Typed Sub-Enum cho `DomainEffect` (`Smell-03` phần 1)
- [x] Trong `src/eve/planner.rs`, định nghĩa các sub-enum định danh rõ cho các nhánh hay phân tích lặp theo `parameter_style` / `result_class`:
  - `MoneyOp` (cộng/trừ Gold, Point, cộng/trừ Shop Point) cho `DomainEffect::MutateMoney`
  - `QuestOp` (cập nhật step, xóa quest, đánh dấu `quest_dont`) cho `DomainEffect::UpdateQuest`
  - `PlayerOp` (`savemap`, `GetArmy`, `texp`, mở shop, v.v.) cho `DomainEffect::PlayerEffect`
  - `PetOp` (tặng pet, thu hồi pet) cho `DomainEffect::RewardPet`
- [x] Giữ `EveResult` **chỉ** ở các variant cần phát frame `S→C 0x14 01` trực tiếp (`TalkStep`, `AwaitMenuChoice`, `MoveEntity`, `PlayMovie`, `NpcTeam`).
- [x] Cả `preflight_probe_batch` lẫn tầng executor đều match trực tiếp trên `DomainEffect` (và các sub-enum tương ứng), loại bỏ hoàn toàn `execute_action_result_non_skill(raw_result: &EveResult)` và mọi pha `match result.result_class` / `match result.parameter_style` lặp lại ở tầng handler.

### 2. Enum hóa `EventSession.trigger_kind` (`Smell-03` phần 2)
- [x] Định nghĩa enum `EventTriggerKind` (`ClickNpc`, `ClickDoor`, `MeetDoor`, `WalkStep` — kèm helper chuyển đổi tường minh từ/sang mã Eve `1, 4, 8, 16` nếu cần cho `EveAutoChainEngine`) trong `src/server/session.rs` (hoặc `src/eve/`).
- [x] Thay thế trường `EventSession.trigger_kind: u8` bằng `EventTriggerKind` và cập nhật toàn bộ chỗ tạo/match trong `talk.rs`, `npc_event.rs`, `movement.rs`, `battle/service.rs` và các test liên quan dùng exhaustive `match`.

### 3. Tách module `src/server/handlers/eve_runner.rs` (`Smell-04` phần 1)
- [x] Tạo `src/server/handlers/eve_runner.rs` và khai báo trong `src/server/handlers/mod.rs`, chuyển toàn bộ:
  - Máy trạng thái `EventRunner` (`execute_event_step*`, `auto_chain_after`, `resume_eve_after_battle*`)
  - `preflight_probe_batch` và `PreFlightError`
  - Các hàm thực thi `DomainEffect` (`homdo`, `quest`, `pet`, `money`, `texp`, `LearnSkill`, v.v.)
- [x] `src/server/handlers/talk.rs` chỉ giữ các handler giải mã/dispatch gói tin `0x14` (`handle_talk_*`) và bổ sung `pub use super::eve_runner::*;` (cho các symbol public đang được import qua `ts_dream::server::handlers::talk::*`) để giữ tương thích ngược cho toàn bộ integration test.

### 4. Hợp nhất vòng lặp `execute_event_step` Sync/Async (`Smell-04` phần 2)
- [x] Hợp nhất hai vòng lặp `execute_event_step` (sync) và `execute_event_step_with_pool_opts` (async) thành **một hàm `async` duy nhất** nhận `pool: Option<&DbPool>` (và tùy chọn `persist_on_finish` / `emit_unlock` tương ứng).
- [x] Nếu còn caller/test đồng bộ gọi `execute_event_step` trực tiếp, cung cấp thin wrapper hoặc chuyển các test đó sang `#[tokio::test]`, đảm bảo không nhân đôi logic `Trailing Effects` / `Warp` / `completed_eve_counts` / `AutoChain`.

### 5. Kiểm chứng Không Hồi Quy
- [x] Không thay đổi hành vi wire `0x14`, `0x17`, `0x18`, `0x1A`, `0x0F`, `Two-Phase State Lock` hay `registry_update_if_online`.
- [x] `cargo clippy --all-targets` exit code `0`.
- [x] `cargo test --all-targets --no-fail-fast` xanh 100%.
