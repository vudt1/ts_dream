# Handoff: `warp-door-sceneinfo-parity`

- **Ngày tạo**: 2026-11-18
- **Trạng thái**: Hoàn tất triển khai Ticket 01, đã qua kiểm thử toàn bộ suite (`cargo test --all-targets --no-fail-fast`) và rà soát 2 trục (`code-review`). Toàn bộ thay đổi đang giữ ở working tree (chưa commit, theo quy ước `AGENTS.md` để user tự commit thủ công).

---

## 1. Tài liệu tham chiếu (không lặp lại nội dung đã ghi)

- **Spec**: [`.scratch/warp-door-sceneinfo-parity/spec.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/warp-door-sceneinfo-parity/spec.md)
- **Ticket**: [`.scratch/warp-door-sceneinfo-parity/issues/01-warp-door-sceneinfo-and-battlegate-parity.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/warp-door-sceneinfo-parity/issues/01-warp-door-sceneinfo-and-battlegate-parity.md)
- **ADR**: [`docs/adr/0010-warp-door-sceneinfo-and-battlegate.md`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0010-warp-door-sceneinfo-and-battlegate.md)
- **Báo cáo đối chiếu dữ liệu**: [`.scratch/data-files-and-eve-audit/research-warps-eve-and-warp-dat.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/data-files-and-eve-audit/research-warps-eve-and-warp-dat.md)
- **Quy chuẩn**: [`CODING_STANDARDS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md), [`AGENTS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md)

---

## 2. Tóm tắt công việc đã hoàn thành

1. **Tách bạch đường phân giải `SceneInfo ID` vs `Door ID` & chính sách `Eve-first`**
   - Gỡ bỏ `resolve_warp_destination`, `perform_party_warp_and_persist`, `perform_party_warp_for_locked_session` trong [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs).
   - Triển khai `resolve_scene_info_warp(data, map_id, scene_info_id)` cho luồng *Kịch bản Eve* ([`src/server/handlers/eve_runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs) và `eve_warp` tại [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs)).
   - Triển khai `resolve_door_warp(data, session, map_id, door_id)` theo thứ tự `WARP_OVERRIDE_KEYS` (`(25303, 2)`, `(59401, 2)`, `(11000, 20)`, `(27000, 2)`) → `Door -> NpcEvent (snapshot_state_with_data + plan_chain) -> rt=2 -> SceneInfo` (fail-closed khi `Door` có `rt=2` nhưng không thỏa điều kiện) → fallback `data.warps`.
   - Chuẩn hóa dịch chuyển tổ đội qua `perform_party_warp_with_resolved` và `perform_party_warp_locked_with_resolved`, đồng thời chặn thành viên đi theo tự ý kích hoạt warp hoặc `BattleGate`.

2. **Đồng bộ `BattleGate` trên `MeetDoor` & `0x14 08` và hoàn thiện hậu chiến**
   - Thêm `pending_battle_gate: Option<(u16, u16)>` và `passed_battle_gates: HashSet<(u16, u16)>` vào `Session` ([`src/server/session.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs)).
   - Dùng chung `try_trigger_battle_gate` tại cả `handle_move_dispatch` (`MeetDoor` ở [`src/server/handlers/movement.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs)) và `handle_warp_confirm` (`0x14 08` ở [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs)).
   - Trong `BattleService::battle_ended` ([`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs)), trích xuất helper `resolve_battle_gate_win`: khi `Outcome::PlayerWin`, ghi nhận `(gate_map, gate_door)` vào `passed_battle_gates` và đưa qua luồng `post_battle_warps` (`perform_party_warp_locked_with_resolved` + `Two-Phase State Lock` + `14 07` / `0x0C` / `hide_from_map` + write-through SQLite cho cả đội, kể cả khi chủ đội ngắt kết nối giữa trận); khi `PlayerLose` / `PlayerFled`, chỉ xóa `pending_battle_gate`.

3. **Thu gọn dữ liệu tĩnh (`Data/Warps.txt`, `Data/BattleGate.txt`)**
   - [`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Warps.txt): thu gọn còn `31` dòng dữ liệu hoạt động (`4` override + `27` fallback thiếu `rt=2`, không có dòng trắng giữa các nhóm comment vì `load_warps` dừng ở dòng trống).
   - [`Data/BattleGate.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/BattleGate.txt): thu gọn còn `10` cổng thiếu `rt=3` (`49901..49908` warp `2`, `15481:4`, `21000:40`).
   - Giữ nguyên `Data/Warp.Dat` và `WarpDatLoader`.

4. **Kiểm thử & Rà soát (`code-review`)**
   - Tạo suite mới [`tests/eve_quest/warp_door_sceneinfo_parity.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/warp_door_sceneinfo_parity.rs) (khai báo ở [`tests/eve_quest_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest_test.rs)) kiểm chứng đầy đủ 3 seam (`dispatch`, `BattleService::battle_ended`, dữ liệu production qua `shared_game_data`).
   - Cập nhật [`tests/data_loaders/warps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders/warps.rs) và các test cũ phụ thuộc `resolve_warp_destination` / `Warps.txt` fallback cho `EveSceneInfo` (`checkpoint6_parity.rs`, `door_triggers_walkstep_and_party_warp.rs`, `eve_battle_sync_registry_and_disconnect.rs`, `npc_eve_e2e.rs`, `p0_blockers.rs`, `tickets_11_12_13.rs`, `code_review_remediation.rs`, `movement_warp.rs`, `repository_init.rs`).
   - Đã chạy `code-review` 2 trục (`Standards` & `Spec`) và xử lý các điểm phát hiện trước khi chạy lại toàn bộ test suite.

---

## 3. Danh sách tệp thay đổi trong phạm vi task

- **Dữ liệu**: `Data/Warps.txt`, `Data/BattleGate.txt`
- **Source (`src/`)**:
  - `src/server/handlers/quest.rs`
  - `src/server/handlers/movement.rs`
  - `src/server/handlers/eve_runner.rs`
  - `src/server/dispatcher.rs`
  - `src/server/session.rs`
  - `src/battle/service.rs`
- **Tests (`tests/`)**:
  - `tests/eve_quest/warp_door_sceneinfo_parity.rs` (mới)
  - `tests/eve_quest_test.rs`
  - `tests/data_loaders/warps.rs`
  - `tests/db/repository_init.rs`
  - `tests/eve_quest/{checkpoint6_parity,door_triggers_walkstep_and_party_warp,eve_battle_sync_registry_and_disconnect,npc_eve_e2e,p0_blockers,tickets_11_12_13}.rs`
  - `tests/server_network/{code_review_remediation,movement_warp}.rs`

---

## 4. Kết quả kiểm chứng

- Lệnh chốt chặn cuối cùng (sau lần sửa cuối):
  ```powershell
  cargo test --all-targets --no-fail-fast
  ```
  - **Kết quả**: Exit code `0` — toàn bộ các target (`architecture_guardrails_test`, `battle_test`, `compound_test`, `data_loaders_test`, `db_test`, `economy_inventory_test`, `eve_quest_test`, `events_arena_test`, `server_network_test`, `server_stop_test`, `skill_job_system_test`) đều **PASS**.

---

## 5. Việc cần làm tiếp (Next Steps)

1. **User review & commit thủ công** các thay đổi trong working tree (lưu ý `GLOSSARY.md` và `docs/adr/0010-warp-door-sceneinfo-and-battlegate.md` cũng đang nằm trong working tree từ bước spec/ADR trước đó).
2. **Backlog ngoài phạm vi đợt này** (nếu mở ticket tiếp theo):
   - Sửa `WarpDatLoader` (`Data/Warp.Dat`) & triển khai opcode `0x44` (truyền tống 28 thành thị).
   - Migration SQLite nếu cần lưu bền vững `passed_battle_gates` qua restart server cho các cổng ngoài chuỗi quest.

---

## 6. Suggested Skills (dành cho phiên agent tiếp theo)

- **`code-review`**: Nếu user muốn rà soát lại diff trước khi commit/merge.
- **`pr`**: Nếu user cần soạn mô tả Pull Request sau khi commit.
- **`tdd`**: Nếu tiếp tục triển khai các ticket ngoài phạm vi (như `WarpDatLoader` / `0x44` hoặc migration `passed_battle_gates`).
