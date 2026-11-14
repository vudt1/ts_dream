# 09: Khắc phục Toàn diện Phát hiện Code Review (Tickets 01–08) — Tính Nguyên tử, Giao thức Wire `0x14`/`0x1A`, Kho Võ Tướng & Chuẩn Hóa Kiến Trúc

**What to build:** Khắc phục triệt để toàn bộ các lỗi logic runtime, lệch giao thức wire với `aLogin.exe`, vi phạm quy chuẩn đồng thời `Two-Phase State Lock` / đồng bộ `ONLINE_SESSIONS` registry, xung đột kho võ tướng `PetStorageType`, và dọn dẹp mã nguồn thừa phát hiện trong đợt Code Review (`CODE_REVIEW_01_08.md`), đảm bảo phân hệ Kịch bản Eve & Nhiệm vụ (`0x14` / `0x17` / `0x18` / `0x1A`) đạt chuẩn 100% cả trên trục **Spec** lẫn **Standards**.

**Blocked by:** `08: Chuẩn hóa EveFightData, Đồng bộ BattleService (Lock + Registry) & Hậu chiến khi Disconnect`

**Status:** resolved

## Acceptance Criteria

### 1. Sửa lỗi `Pre-Flight Probe All-or-Nothing` xuyên qua `TalkStep` & Kiểm tra đủ vật phẩm trừ (`[Spec-B01]`)
- [x] Trong `preflight_probe_batch`, **không** dừng (`break`) tại `DomainEffect::TalkStep` (`rt = 1`): bộ mô phỏng phải kiểm tra xuyên suốt toàn bộ các hiệu ứng biến đổi tài nguyên (`MutateItem`, `UpdateQuest`, `RewardPet`, `MutateMoney`, `PlayerEffect`) trong chuỗi `results` hiện tại cho tới khi hết chuỗi hoặc gặp điểm rẽ nhánh tương tác/chuyển cảnh thực sự (`OpenSurface` `rt = 6`, `StartBattle` `rt = 2/3`, `Warp` `rt = 0 rc = 6`).
- [x] Khi mô phỏng `DomainEffect::MutateItem` với `delta < 0` (thu hồi vật phẩm), kiểm tra `trial_homdo` có đủ số lượng `delta.unsigned_abs()` của `item_id` hay không; nếu thiếu vật phẩm đầu vào hoặc nếu phần thưởng phía sau `TalkStep` làm tràn 25 ô `homdo`, tràn 200 ô `Quest Ledger` (`0x18 03`), tràn 4 ô võ tướng mang theo, hoặc vượt trần tiền tệ, từ chối **All-or-Nothing ngay từ đầu chuỗi** trước khi trừ bất kỳ vật phẩm hay phát bất kỳ câu thoại nào.

### 2. Chuẩn hóa Giao thức Wire `0x1A` (Kim/Điểm), `POINT_CAP` & Thông báo Vật phẩm `0x14 01` (`[HV-01]`, `[Spec-B02]`, `[Spec-B05]`)
- [x] Cập nhật `pub const POINT_CAP: u32 = 2_000_000_000;` trong `src/protocol/mod.rs` để khớp với đặc tả miền trong `GLOSSARY.md` (`player+0x1300`, trần `2_000_000_000`).
- [x] Sửa lỗi gửi nhầm gói `S→C 0x1A 03` (vốn là mã báo lỗi *"Số tiền đạt giới hạn"* trong `aLogin.exe` `FUN_0079175f`) khi thực thi `result_class = 5`:
  - `p_style = 3` (cộng Điểm thưởng / Shop Points): cộng `session.shop_point` (kẹp trần `POINT_CAP`) và phát chuẩn gói `S→C 0x1A 05 <add_u32_LE>` (không gọi `0x1A 03`).
  - `p_style = 4` (trừ Điểm thưởng / Shop Points): trừ `session.shop_point` và phát chuẩn gói `S→C 0x1A 06 <sub_u32_LE>` (không gọi `0x1A 03`).
- [x] Khi thực thi `DomainEffect::MutateItem` (`rt = 0, rc = 1`, `qty != 0`), phát kèm khung kết quả sự kiện `S→C 0x14 01` (`NpcTalkCodec::build_eve_result_hex`) cùng với `dump_homdo()` (`0x17 05`) để `FUN_005eb530` trên `aLogin.exe` hiển thị thông báo nhận/mất vật phẩm từ `Item.dat`.

### 3. Tách biệt Kho Võ Tướng theo `PetStorageType` (`Carried`, `Cart`, `Hotel`, `Warehouse`) (`[Spec-B03]`, `[Smell-05]`)
- [x] Bổ sung helper phương thức trên `PetState` (ví dụ `is_carried()`, `effective_storage_and_slot()`) để gom logic phân vùng lưu trữ về một nơi duy nhất trên `PetState`.
- [x] Sửa `BattleService::pet_slots` chỉ lấy các võ tướng đang mang theo bên người (`pet.storage_type == PetStorageType::Carried` và `(1..=4).contains(&pet.stt)`), tuyệt đối không lôi võ tướng đang gửi ở `Cart (2)`, `Hotel (3)`, `Warehouse (4)` vào trận đấu.
- [x] Sửa các thao tác rút/gửi võ tướng trong `pet_actions.rs` (`0x0F 03` / `0x1F 02` rút từ Khách sạn ra mang theo, `0x0F 07` / `0x1F 03` gửi vào Khách sạn) và `execute_reward_pet` (`p_style = 2` thu hồi võ tướng): lọc chính xác theo `storage_type` (không để võ tướng ở `Cart` có `stt = 5` bị rút nhầm khi thao tác với ô 1 `Hotel` cũng có `stt = 5`), đồng thời cập nhật đúng `pet.storage_type` khi chuyển giữa `Carried (1)` và `Hotel (3)`.

### 4. Tuân thủ `Two-Phase State Lock`, Đồng bộ `registry_update_if_online` & Giao dịch SQLite Nguyên tử Liên Người Chơi (`[HV-02]`, `[HV-03]`)
- [x] Gọi `crate::server::session::registry_update_if_online(session)` đầy đủ sau khi biến đổi trạng thái ở nhánh `rc == 1` (`homdo`) và `rc == 2` (`quest_tasks` / `quest_dont`) trong `talk.rs`.
- [x] Trong `Battle::advance_wave` / `BattleService`: khi cập nhật bước nhiệm vụ `218` lúc chuyển đợt trận đấu, đồng bộ trên cả session của `BattleService` lẫn `ONLINE_SESSIONS` registry để khi `battle_ended` kết thúc trận không bị bản `sess_arc` cũ ghi đè làm mất bước đợt mới.
- [x] Sửa `persist::persist_sessions_transaction(pool, sessions, ...)`: khi truyền nhiều `Session` (ví dụ Đội trưởng + các Thành viên tổ đội trong `Door Warp` hoặc `Battle End`), thực thi lưu toàn bộ các phiên đó trong **cùng 1 SQLite transaction duy nhất** (`pool.write.begin().await` -> lưu từng session trên `&mut tx` -> `tx.commit().await`), để nếu bất kỳ thành viên nào lỗi DB thì toàn bộ tổ đội cùng rollback nguyên tử.
- [x] Sửa `quest::perform_party_warp_and_persist`: tuân thủ đúng thứ tự 2 pha của `CODING_STANDARDS.md` §1.1 — giữ bản sao trạng thái trước khi warp (`before_state`), thực thi commit SQLite trong khi đang giữ `_guards` (`lock_player_operations`); chỉ khi commit DB thành công mới cập nhật `registry_update_if_online` và đẩy các khung wire `14 07` / `0x0C` vào `HandleOutcome`. Nếu commit DB thất bại, khôi phục RAM của Đội trưởng về `before_state`, không sửa registry và không phát gói dịch chuyển.

### 5. Hoàn thiện Chuỗi Sự Kiện có `Warp`, Hậu Chiến khi Disconnect & `WalkStep` `AutoChain` (`[Spec-B04]`, `[Spec-B06]`)
- [x] Khi thực thi `DomainEffect::Warp` trong chuỗi sự kiện Eve:
  - Thực thi hết các hiệu ứng không chặn (hoặc cập nhật hoàn tất chuỗi) và tăng bộ đếm `session.completed_eve_counts[(scene_id, eve_no)]` nếu chuỗi có thay đổi trạng thái, tránh việc điều kiện `ConditionClass = 12` (`SceneEventCount`) luôn đọc được `0` sau khi warp.
- [x] Trong `BattleService::battle_ended`:
  - Giữ `_op_guards` (`lock_player_operations`) bao trùm cả bước xử lý `Warp` hậu chiến và cập nhật `sess_arc` trước khi nhả khóa, xóa bỏ khoảng hở khóa (lock gap).
  - Đảm bảo người chơi bị mất kết nối giữa trận (`offline_parked`) nếu có hiệu ứng `Warp` trong chuỗi hậu chiến thì tọa độ bản đồ đích vẫn được áp dụng vào `offline_s` và lưu bền vững xuống SQLite trong cùng giao dịch kết thúc trận.
- [x] Hỗ trợ đầy đủ `WalkStep` (`trigger_kind = 16`):
  - Bổ sung `16` vào `AUTO_CHAIN_TRIGGER_KINDS` và tra cứu `scene.encounters` theo `ev.npc_click_id` trong `EveAutoChainEngine::event_list_for` để sự kiện vùng bước chân có thể tự nối chuỗi (`AutoChain`).
  - Trong `talk::handle_talk_select_menu` (`0x14 09`), khi `ev.trigger_kind == 16`, tra cứu đúng vùng `encounter` có `id == ev.npc_click_id` thay vì quét lại từ đầu theo tọa độ `(x, y)`.

### 6. Dọn dẹp Mã Nguồn, Codec Hex, Chuẩn hóa `Planner`/`Evaluator` & Guardrail Test (`[HV-04]`, `[HV-05]`, `[Spec-M01..M03]`, `[Spec-SC01]`, `[Smell-01..04]`)
- [x] Xóa bỏ dead code `SqliteQuestRepository` (`src/db/modern/sqlite/quests.rs`, `QuestRepository` trong `traits.rs` và `mod.rs`) chứa các câu lệnh SQL lỗi thời không có caller.
- [x] Gỡ bỏ 2 file cấu hình cục bộ `.agents/plugins/rea/mcp_config_sample.json` và `.agents/plugins/rea/plugin.json` khỏi git tracking (hoặc đưa vào `.gitignore`).
- [x] Thay thế các chuỗi hex literal rời rạc trong handler (`"F44402000602"`, `"F44402001407"`, `"F4440C000F01..."`, `"F44407000F02..."`, `"F44406001A01/02/05/06..."`) bằng các hàm helper/codec định danh rõ ràng; thay thế 3 chỗ gọi `lock_online_sessions().insert(...)` trong `tests/pure_planner_event_runner_and_preflight_test.rs` và `tests/sqlite_migration_0002_test.rs` bằng `registry_register_for_login` / `registry_update_if_online`.
- [x] Kết nối các chỉ mục tra cứu nhanh (`SceneEveData` / `GameData::eve_*`) và `plan_chain` vào luồng giải quyết sự kiện thực tế (hoặc lược bỏ hàm thừa không dùng); làm rõ hàm so sánh đảo chiều toán hạng của `Data/eve.emg` cho `cc = 1` và `cc = 7 (param = 0, 2)` trong `evaluator.rs` (bảo toàn 100% bảng chân trị `Data/eve.emg`), và hợp nhất các cụm code lặp trong `talk.rs` / `characters.rs`.
- [x] Viết bộ kiểm thử tích hợp trong `tests/code_review_remediation_test.rs` bao phủ toàn bộ các tình huống trên (kèm negative test cho từng mục) và chạy xanh `cargo test --all-targets --no-fail-fast`.
