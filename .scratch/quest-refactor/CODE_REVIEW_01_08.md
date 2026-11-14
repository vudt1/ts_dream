# Báo Cáo Code Review (`git diff cdc9067...HEAD`)

- **Điểm mốc cố định (Fixed Point)**: `cdc9067` (*added spec*) → `HEAD` (`db88cd5`, bao gồm 2 commit triển khai: `f626eea` và `db88cd5`).
- **Tài liệu đặc tả đối chiếu**: [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md), [HANDOFF_01_08.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/HANDOFF_01_08.md), các ticket `01..08` trong `.scratch/quest-refactor/issues/`, và [0009-eve-quest-engine-and-persistence.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md).
- **Tài liệu quy chuẩn đối chiếu**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md), [AGENTS.md](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md), [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md), [0005-economy-wire-parity-and-transactions.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0005-economy-wire-parity-and-transactions.md), [0006-battle-protocol-parity-and-turn-lifecycle.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md).

---

## Standards

### Phần I: Vi Phạm Quy Chuẩn Đã Tài Liệu Hóa (Hard Violations)

#### 1. [HV-01] Hằng số `POINT_CAP` lệch với quy chuẩn miền trong `GLOSSARY.md`
- **Quy chuẩn vi phạm**: [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L280-L285) (§"Tiền tệ (Currencies: Kim, Tiền trang, Điểm)"):
  > *"Kim trên người (Player Gold): Lưu tại `player+0x12DC`, trần `9_999_999` (`GOLD_CAP`)."*  
  > *"Điểm tích lũy / Tiền mềm (Shop Point): Lưu tại `player+0x1300`, trần `2_000_000_000`."*
- **Vị trí code**:
  - [mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L29-L32):
    ```rust
    pub const GOLD_CAP: u32 = 9_999_999;
    pub const POINT_CAP: u32 = 9_999_999;
    ```
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L302-L309) (`preflight_probe_batch`, nhánh `p_style = 3`) và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1264-L1270) (`execute_action_result_non_skill`, nhánh `rc == 5, p_style == 3`).
- **Giải thích dễ hiểu (Cơ chế & Tác động)**:
  - Trong TS Online, Vàng trên người (`gold`) bị giới hạn ở `9,999,999`, nhưng Điểm tích lũy / Shop Point (`shop_point`) có trần là `2,000,000,000`.
  - Việc khai báo `POINT_CAP = 9_999_999` khiến bất kỳ người chơi nào đang tích lũy hơn 9.99 triệu điểm khi làm nhiệm vụ có thưởng điểm (`rc=5, p_style=3`) sẽ bị `preflight_probe_batch` chặn toàn bộ bước nhiệm vụ hoặc bị cắt cụt điểm thưởng về `9,999,999`.
- **Cách khắc phục**: Đổi `pub const POINT_CAP: u32 = 2_000_000_000;` tại [mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L32) (hoặc cập nhật lại [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L282) nếu chủ đích hạ trần Shop Point).

---

#### 2. [HV-02] Thiếu đồng bộ `registry_update_if_online` khi biến đổi túi đồ/nhiệm vụ giữa chuỗi hội thoại và ghi đè ngược `quest_tasks[218]` khi qua đợt trận đấu (`advance_wave`)
- **Quy chuẩn vi phạm**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L24-L30) (§1.2 "Đồng bộ Session Registry (`registry_update_if_online`)") và [AGENTS.md](file:///D:/VUDT/GIT_PCC/ts_dream/AGENTS.md) (§"Quy Chuẩn Đồng Thời & Trạng Thái Phiên"):
  > *"Sau bất kỳ biến động trạng thái hoặc vật phẩm nào trên phiên hiện tại (`conn.session`), bắt buộc phải gọi: `crate::server::session::registry_update_if_online(&conn.session);`"*
- **Vị trí code**:
  1. [construction.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs#L164-L171) (`Battle::advance_wave`):
     ```rust
     let next_step = self.current_wave.saturating_add(1) as u8;
     for &ch_id in &human_ids {
         if let Some(session) = crate::server::session::lock_online_sessions().get_mut(&ch_id) {
             crate::server::handlers::quest::upsert_quest_task_slot(session, 218, next_step);
         }
     }
     ```
  2. [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1103-L1120) (nhánh `rc == 1`: thêm/trừ vật phẩm `session.homdo`) và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1126-L1224) (nhánh `rc == 2`: cập nhật `session.quest_tasks` và `session.quest_dont`).
- **Giải thích dễ hiểu (Cơ chế & Tác động)**:
  - **Trường hợp 1 (`Battle::advance_wave`)**: Server giữ 2 bản sao `Session` khi người chơi đang đánh trận: một bản trong bảng đăng ký chung (`ONLINE_SESSIONS` registry) và một bản nằm trong `BattleService::online` (`sess_arc`). Khi qua đợt (`advance_wave`), code sửa trực tiếp bản trong `ONLINE_SESSIONS` nhưng **không** sửa bản trong `sess_arc`. Tới lúc trận đấu kết thúc ([service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L462-L543)), `battle_ended` lấy bản `sess_arc` (vẫn đang mang bước đợt cũ) rồi gọi `registry_update_if_online(&s)`, **ghi đè xóa mất tiến độ đợt vừa cập nhật**!
  - **Trường hợp 2 (`talk.rs` nhánh `rc == 1` và `rc == 2`)**: Các nhánh `rc == 5, 7, 8` đều gọi `registry_update_if_online(session)`, nhưng `rc == 1` (thêm/xóa đồ `homdo`) và `rc == 2` (cập nhật sổ nhiệm vụ) lại quên gọi. Nếu kịch bản Eve vừa trao/trừ đồ ở đầu chuỗi rồi dừng lại ở hộp thoại (`TalkStep` hoặc `OpenSurface`) chờ người chơi bấm tiếp, bản `Session` trong `ONLINE_SESSIONS` vẫn là bản cũ. Nếu đúng lúc đó `AutoSave` quét hoặc người chơi khác kiểm tra trạng thái, dữ liệu trong registry bị lệch với `conn.session`.
- **Cách khắc phục**:
  - Gọi `crate::server::session::registry_update_if_online(session)` ở cuối nhánh `rc == 1` ([talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1119)) và `rc == 2` ([talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1223)).
  - Tại `advance_wave`, đồng bộ tiến độ `218` vào cả `sess_arc` trong `BattleService` thay vì chỉ chọc thẳng vào `lock_online_sessions()`.

---

#### 3. [HV-03] Vi phạm Quy chuẩn Khóa Hai Pha (Two-Phase State Lock): Nhả khóa trước khi hoàn tất sửa Session, thiếu Rollback RAM khi lưu DB lỗi, và tách giao dịch SQLite đa người chơi
- **Quy chuẩn vi phạm**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L14-L23) (§1.1 "Thứ tự Khóa Hai Pha (Two-Phase State Lock)") và [0005-economy-wire-parity-and-transactions.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0005-economy-wire-parity-and-transactions.md#L24-L27):
  > *"Phase 2 — SQLite Write Transaction: Thực thi câu lệnh ghi nguyên tử vào SQLite trong khi vẫn giữ guard của Phase 1. Nếu commit DB thành công: Phát các frame xác nhận wire cho cả hai bên. Nếu commit DB thất bại: Bắt buộc rollback RAM state về snapshot trước giao dịch (`before_state`) và hủy bỏ toàn bộ các gói tin phản hồi thành công."*
- **Vị trí code**:
  1. [quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L964-L1031) (`perform_party_warp_and_persist`).
  2. [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L642-L660) (`BattleService::battle_ended` — phần xử lý `eve_warps` sau `drop(_op_guards)`).
  3. [persist.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/persist.rs#L298-L312) (`persist_sessions_transaction`).
- **Giải thích dễ hiểu (Cơ chế & Tác động)**:
  - **Trong `perform_party_warp_and_persist` ([quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L972-L1029))**: Code đổi tọa độ `map_id/map_x/map_y` trên RAM, cập nhật `registry_update_if_online` và đẩy các gói dịch chuyển `14 07` + `0x0C` vào hàng đợi gửi đi **trước khi** ghi SQLite. Nếu `persist_sessions_transaction` thất bại (dòng 1022), code chỉ ghi `tracing::warn!` mà **không rollback tọa độ RAM/registry** và vẫn gửi gói dịch chuyển cho client (gây lệch vị trí giữa RAM và DB).
  - **Trong `BattleService::battle_ended` ([service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L642-L660))**: Khi hậu chiến có lệnh `Warp`, code `drop(_op_guards)` ở dòng 642, sau đó gọi `perform_party_warp_and_persist` (khóa lại rồi nhả ra khi hàm kết thúc), và tới dòng `656-660` mới lấy `sess_arc.write().await` để gán `s.map_id = warp_sess.map_id` **khi không còn giữ bất kỳ `PlayerOpGuard` nào**. Khoảng trống giữa hai lần giữ khóa cho phép gói tin khác của người chơi chen vào giữa.
  - **Trong `persist_sessions_transaction` ([persist.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/persist.rs#L305-L310))**: Mặc dù tên hàm là `_transaction` và nhận mảng `sessions: &[&Session]` (dùng khi lưu cả Đội trưởng lẫn Thành viên tổ đội lúc `Door Warp` hoặc `Battle End`), vòng lặp `for session in sessions { repo.save(session).await }` lại mở **một transaction SQLite riêng biệt cho từng người**. Nếu Đội trưởng commit thành công nhưng Thành viên thứ 2 gặp lỗi DB, dữ liệu của Đội trưởng đã commit không thể rollback, phá vỡ tính nguyên tử liên người chơi.
- **Cách khắc phục**:
  - Mở một `pool.write.begin().await` duy nhất bao ngoài vòng lặp lưu toàn bộ `sessions` trong `persist_sessions_transaction`.
  - Trong `perform_party_warp_and_persist`, lưu `before_state` của đội trưởng và thành viên; chỉ phát gói `14 07` / `0x0C` và cập nhật registry sau khi commit DB thành công (rollback nếu DB trả về `false`).
  - Trong `battle_ended`, thực hiện cập nhật tọa độ warp và `sess_arc` trước khi `drop(_op_guards)`.

---

#### 4. [HV-04] Gọi trực tiếp `lock_online_sessions().insert(...)` trong Test thay vì API chuẩn
- **Quy chuẩn vi phạm**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L68-L71) (§3.3 "Độ trung thực Đa phiên (Multi-session Fidelity)"):
  > *"Cấm gọi `sessions.insert(conn.session.clone())` thủ công giữa các bước test để ép state; bài test phải chứng minh bản thân handler tự động cập nhật registry qua `registry_update_if_online`."*
- **Vị trí code**:
  - [pure_planner_event_runner_and_preflight_test.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/pure_planner_event_runner_and_preflight_test.rs#L581)
  - [sqlite_migration_0002_test.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/sqlite_migration_0002_test.rs#L230) và [sqlite_migration_0002_test.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/sqlite_migration_0002_test.rs#L255)
- **Giải thích dễ hiểu & Cách khắc phục**:
  - Các test này thao tác trực tiếp lên `lock_online_sessions().insert(...)`. Cần thay bằng `registry_register_for_login(&session)` khi khởi tạo phiên online trong test, và dùng `registry_update_if_online(&session)` (hoặc để handler tự gọi) khi cập nhật trạng thái.

---

#### 5. [HV-05] Chuỗi Hex Literal (`F444...`) nằm trực tiếp trong Handler thay vì Codec / Helper
- **Quy chuẩn vi phạm**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L90-L91) (§4 "Quy ước Đặt tên & Cấu trúc Mã nguồn"):
  > *"Tránh Magic Number / Primitive Obsession: Sử dụng các hằng số định danh miền đã được khai báo trong `src/protocol/mod.rs`. Các chuỗi literal hex (`F444...`) phải được thay thế bằng helper như `sys_msg_frame` hoặc các codec tương ứng trong `src/protocol/codecs/`."*
- **Vị trí code**:
  - [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L85), [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L115), [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1469), [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1737): hardcode `"F44402000602"`.
  - [quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L977), [quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L999): hardcode `"F44402001407"`.
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L480-L506) (`execute_reward_pet`): ghép chuỗi `"F4440C000F01..."` và `"F44407000F02..."`.
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1242-L1271) (`execute_action_result_non_skill`): ghép chuỗi `"F44406001A01..."`, `"F44406001A02..."`, `"F44406001A05..."`.
- **Cách khắc phục**: Chuyển việc đóng gói các khung `06 02`, `14 07`, `0F 01/02`, `1A 01/02/05` vào các hàm builder có tên rõ nghĩa trong `src/protocol/codecs/` hoặc module giao thức tương ứng.

---

### Phần II: Mùi Mã Nguồn Cơ Bản (Baseline Code Smells — Judgement Calls)

#### 6. [Smell-01] Duplicated Code (Trùng lặp mã nguồn ở 5 cụm)
1. **Khối kích hoạt `EventSession` + khóa bước chân `F44402000602` lặp lại 4 lần**: tại [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L77-L89), [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L110-L119), [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1458-L1471), và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1729-L1740). → *Cách sửa*: Trích xuất hàm dùng chung `activate_event_session(session, event_session, out)`.
2. **Nhân đôi toàn bộ vòng lặp `EventRunner` giữa bản sync (không có `DbPool`) và bản async (có `Option<&DbPool>`)**: tại [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L712-L775) (`execute_event_step`) song song với [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L777-L890) (`execute_event_step_with_pool_opts`), và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L898-L928) (`finish_event_session_iter`) song song với [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L930-L969) (`finish_event_session_with_pool_iter`). → *Cách sửa*: Giữ một hàm thực thi thống nhất nhận `Option<&DbPool>`.
3. **Lặp lại 3 lần khối xử lý hậu chiến (`resume_eve_after_battle_deferred_warp` + gửi tin nhắn + broadcast map)**: tại [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L477-L518) (`online_entries`), [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L554-L586) (`registry_get`), và [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L593-L622) (`offline_parked`). → *Cách sửa*: Trích xuất helper dùng chung cho cả 3 nhánh.
4. **Lặp lại công thức tính `next_step` giữa `p_style == 1` và `p_style == 4` của `rc == 2`**: tại [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1145-L1156) và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1193-L1204).
5. **Lặp lại danh sách 9 bảng con khi xóa nhân vật**: tại [characters.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/characters.rs#L45-L58) (inherent `delete`) và [characters.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/characters.rs#L170-L183) (trait `CharacterRepository::delete`). → *Cách sửa*: Cho hàm trong trait gọi thẳng hàm inherent `Self::delete(self, character_id).await`.

#### 7. [Smell-02] Speculative Generality (Khái quát hóa đầu cơ — Code mới viết nhưng không có nơi gọi)
- **Vị trí code**: `EveTrigger`, `PlannedChain`, và `plan_chain` trong [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L15-L23) và [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L92-L97), [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L171-L186).
- **Giải thích**: Các kiểu và hàm này được tạo mới và export qua `src/eve/mod.rs` nhưng không được gọi ở bất kỳ đâu trong `src/` hay `tests/` (`npc_event.rs` vẫn dùng `NpcTrigger` riêng). → *Cách sửa*: Tích hợp thật vào `npc_event.rs` hoặc xóa bỏ phần dư thừa.

#### 8. [Smell-03] Primitive Obsession & Repeated Switches (Rẽ nhánh lặp lại trên `EveResult` và `trigger_kind: u8`)
- **Vị trí code**: [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L50-L73), [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L195-L335), [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L685-L691), và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1095-L1399).
- **Giải thích**: `planner::plan_result` đã phân tích `EveResult` ra `DomainEffect`, nhưng khi thực thi tại [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L685-L691), 4 biến thể `MutateItem | UpdateQuest | MutateMoney | PlayerEffect` lại bị gom chung và truyền ngược `raw_result: &EveResult` vào `execute_action_result_non_skill` để `match result.result_class` và `match result.parameter_style` lại từ đầu (khiến cùng một bảng mã bị switch ở 3 nơi: `planner.rs`, `preflight_probe_batch`, và `execute_action_result_non_skill`). Tương tự, `EventSession.trigger_kind` lưu bằng `u8` (`1, 4, 8, 16`) thay vì enum.

#### 9. [Smell-04] Divergent Change (`src/server/handlers/talk.rs` phình to ~1,816 dòng, ôm 4 trách nhiệm khác nhau)
- **Vị trí code**: [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1-L1816).
- **Giải thích**: File `talk.rs` hiện vừa giải mã gói tin `0x14`, vừa chứa máy trạng thái `EventRunner`, vừa chứa `preflight_probe_batch`, vừa chứa toàn bộ logic cộng/trừ đồ, cấp phát nhiệm vụ, thưởng pet, lên cấp EXP. → *Cách sửa*: Tách `preflight_probe_batch` và bộ thực thi `DomainEffect` sang module riêng (ví dụ `src/server/handlers/eve_runner.rs`).

#### 10. [Smell-05] Feature Envy (Logic suy luận kho chứa `PetState` nằm ngoài `PetState`)
- **Vị trí code**: [npc_event.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L168-L185) (`snapshot_state_with_data`) và [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L240-L245) / [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L371-L384).
- **Giải thích**: Nhiều module bên ngoài phải tự kiểm tra `pet.storage_type` kết hợp với `pet.stt` (`1..=4` hay `>= 5`) để đoán vị trí pet. → *Cách sửa*: Đưa thành phương thức trên `impl PetState` trong [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs).

---

## Spec

### (a) Yêu cầu trong Spec / Issues 01..08 còn thiếu hoặc mới hoàn thiện một phần (Missing / Partial)

#### 1. [Spec-M01] Chưa gỡ bỏ dead code `SqliteQuestRepository` (`src/db/modern/sqlite/quests.rs`) — và code này hiện chứa SQL hỏng
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L125): *"Liên kết đầy đủ `character_bit_flags` vào `Session` và kho lưu trữ phiên (gỡ bỏ repository dead code thừa)."*
  - [03-sqlite-migration-0002-and-atomic-save.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/issues/03-sqlite-migration-0002-and-atomic-save.md#L9): *"Gỡ bỏ truy vấn/bảng `character_quest_items` lỗi thời hoặc dead code `SqliteQuestRepository` không có caller."*
- **Vị trí trong Codebase**:
  - [quests.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/quests.rs#L10-L137) (`pub struct SqliteQuestRepository<'a>`)
  - [mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/mod.rs#L10) và [mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/mod.rs#L45-L47) (`pub fn quests(&self)`)
- **Giải thích dễ hiểu**:
  - `SqliteQuestRepository` không có bất kỳ nơi nào gọi nhưng vẫn chưa được dọn bỏ theo yêu cầu của Spec & Ticket 03.
  - Nguy hiểm hơn, nếu ai đó vô tình gọi các phương thức của `SqliteQuestRepository`:
    - `upsert_mission` và `list_missions` ([quests.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/quests.rs#L15-L49)) truy vấn bảng `character_missions` — bảng này hoàn toàn không tồn tại trong DB.
    - `mark_event_completed` ([quests.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/quests.rs#L76-L92)) INSERT vào `character_completed_events` mà thiếu cột `scene_id`, làm sai lệch khóa chính `(playerid, scene_id, eventid)` vừa đổi ở Migration `0002`.

---

#### 2. [Spec-M02] Chưa chuẩn hóa thứ tự đối số `compare_step` giữa các lớp điều kiện trong `src/eve/evaluator.rs`
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L93): *"Sửa chuẩn hóa thứ tự đối số `compare_step` trong bộ đánh giá điều kiện; ép fail-closed (`false` + log telemetry)..."*
  - [04-snapshot-fail-closed-and-menu-filter.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/issues/04-snapshot-fail-closed-and-menu-filter.md#L10): *"Chuẩn hóa `evaluator.rs`: đồng nhất thứ tự đối số của `compare_step` trên mọi lớp điều kiện"*
  - [RESEARCH_AND_GAP_ANALYSIS.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/RESEARCH_AND_GAP_ANALYSIS.md#L163): Chỉ rõ sự bất nhất giữa `BagItem (1)` / `PlayerAttribute (7, params 0, 2)` gọi `compare_step(cond.condition_value, cond.condition_ops, actual)` so với các lớp khác gọi `compare_step(actual, cond.condition_ops, cond.condition_value)`.
- **Vị trí trong Codebase**:
  - [evaluator.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L19) (`pub fn compare_step(actual: i32, ops: u8, expected: i32) -> bool`)
  - [evaluator.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L54) (`condition_class = 1` — `BagItem`)
  - [evaluator.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L108-L109) (`condition_class = 7, param = 0 | 2` — `Level` / `Reborn`)
- **Giải thích dễ hiểu**:
  - Hàm `compare_step(actual, ops, expected)` đã được đặt tên tham số rất rõ: tham số đầu là `actual` (giá trị thực tế của người chơi), tham số thứ ba là `expected` (giá trị yêu cầu trong kịch bản).
  - Tuy nhiên ở dòng 54 (`BagItem`) và dòng 108–109 (`Level`, `Reborn`), code vẫn truyền ngược `compare_step(cond.condition_value, cond.condition_ops, state.level)` — tức là nhét `expected` vào vị trí của `actual`, còn các nhánh `cc = 2, 7 (param 3,5,6), 8, 9, 10, 12, 14` lại truyền xuôi `compare_step(actual, cond.condition_ops, cond.condition_value)`.

---

#### 3. [Spec-M03] `Pure Eve Planner` (`EveTrigger`, `plan_chain`) và các chỉ mục `GameData::eve_*` đã viết nhưng chưa nối vào luồng chạy thật
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L69): *"Xây dựng chỉ mục tra cứu nhanh chỉ-đọc theo khóa `(scene_id, eve_no)` cùng chỉ mục theo NPC, Door và Encounter."*
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L80-L90): *"Tách riêng hoàn toàn khỏi I/O và Tokio lock; nhận vào ngữ cảnh bất biến (`PlayerEventSnapshot`) và một `EveTrigger` có kiểu rõ ràng..."*
- **Vị trí trong Codebase**:
  - [loader.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L397-L429) (`GameData::eve_event`, `eve_npc_events`, `eve_door_events`, `eve_encounter_events`)
  - [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L15-L23) (`EveTrigger`) và [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L171-L186) (`plan_chain`)
  - [npc_event.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L305-L349) và [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L730) / [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L794)
- **Giải thích dễ hiểu**:
  - Khi server khởi động, `SceneEveData::build_indices` tốn RAM xây dựng các `HashMap` chỉ mục (`npc_events_by_npc`, `door_events_by_door`, `encounter_events_by_id`), nhưng lúc chạy thật thì `npc_event.rs` và `auto_chain.rs` không hề gọi các chỉ mục này mà vẫn tự duyệt mảng thô như cũ.
  - Tương tự, `plan_chain` không được gọi một lần khi mở sự kiện, mà `execute_event_step_with_pool_opts` gọi `plan_results(&rem)` lặp lại ở **mỗi vòng lặp bước đơn lẻ** trên phần đuôi `&ev.results[ev.current_index..]`.

---

### (b) Hành vi trong diff không được yêu cầu (Scope Creep)

#### 4. [Spec-SC01] Commit kèm file cấu hình cục bộ `.agents/plugins/rea/` chứa đường dẫn máy cá nhân
- **Vị trí trong Codebase**:
  - [mcp_config_sample.json](file:///D:/VUDT/GIT_PCC/ts_dream/.agents/plugins/rea/mcp_config_sample.json#L1-L15)
  - [plugin.json](file:///D:/VUDT/GIT_PCC/ts_dream/.agents/plugins/rea/plugin.json#L1-L3)
- **Giải thích dễ hiểu**: Hai file cấu hình plugin REA (chứa đường dẫn ổ đĩa cá nhân như `AppData\Roaming\fnm\node-versions\v24.22.3...`, `\ghidra_12.1.4_PUBLIC`) đã bị đưa nhầm vào commit `f626eea`, không thuộc phạm vi đặc tả Quest Refactor.

---

### (c) Các yêu cầu nhìn bề ngoài đã triển khai nhưng có lỗi tinh vi hoặc lệch chuẩn Spec / Wire Protocol

#### 5. [Spec-B01] [Nghiêm trọng] `Pre-Flight Probe` (`preflight_probe_batch`) dừng kiểm tra ngay tại câu thoại `TalkStep` đầu tiên thay vì kiểm tra toàn chuỗi, làm vỡ nguyên tắc `All-or-Nothing`
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L43): *"12. Là một Người chơi nhận chuỗi phần thưởng có chứa vật phẩm nhưng `homdo` (`0x17`) không đủ ô trống (hoặc vượt giới hạn xếp chồng/tiền tệ `GOLD_CAP`/`POINT_CAP`), tôi muốn server kiểm tra sức chứa trước khi thực thi (`Pre-Flight All-or-Nothing`) và từ chối toàn bộ bước thực thi đó (không trừ nguyên liệu đầu vào, không đánh dấu bước quest, không cộng thưởng một phần)..."*
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L98): *"Thực hiện Pre-Flight Probe trên toàn bộ `DomainEffect` biến đổi tài nguyên trước khi ghi nhận bất kỳ thay đổi nào vào RAM/DB..."*
- **Vị trí trong Codebase**:
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L191-L194):
    ```rust
    for effect in effects {
        if effect.is_blocking_or_terminal() {
            break;
        }
    ```
  - [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L76-L86): `DomainEffect::TalkStep { .. }` được đánh dấu `is_blocking_or_terminal() == true`.
- **Giải thích dễ hiểu (Chuyện gì xảy ra ở Runtime)**:
  - Trong kịch bản `Data/eve.emg`, rất nhiều chuỗi trả nhiệm vụ có cấu trúc:
    `[Bước 1: Trừ vật phẩm nhiệm vụ] -> [Bước 2: NPC nói câu thoại cảm ơn (TalkStep)] -> [Bước 3: Trao vật phẩm thưởng / Trao võ tướng / Ghi sổ nhiệm vụ]`.
  - Do `preflight_probe_batch` gặp `TalkStep` là `break` ngay lập tức (dòng 193), nên khi bắt đầu chuỗi, server **chỉ kiểm tra Bước 1** mà không hề kiểm tra xem túi đồ 25 ô, sổ nhiệm vụ 200 ô hay túi pet 4 ô có đủ chỗ cho Bước 3 hay không!
  - Kết quả: Server lập tức **trừ mất vật phẩm nhiệm vụ** của người chơi ở Bước 1 và hiện câu thoại ở Bước 2. Khi người chơi bấm tiếp tục qua câu thoại để tới Bước 3, nếu lúc này túi đồ/sổ nhiệm vụ/túi pet đang đầy, Bước 3 bị từ chối nhưng **vật phẩm nhiệm vụ ở Bước 1 đã bị trừ mất và không được hoàn trả**!
- **Cách khắc phục**: Trong `preflight_probe_batch` ([talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L191-L194)), không `break` tại `DomainEffect::TalkStep`; cần mô phỏng xuyên suốt toàn bộ các hiệu ứng trong cùng chuỗi `ev.results` (ít nhất cho tới khi gặp `OpenSurface` rẽ nhánh hoặc `StartBattle`). Đồng thời khi mô phỏng `MutateItem` với `*delta < 0` ([talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L201-L206)), cần kiểm tra người chơi có đủ số lượng vật phẩm bị trừ hay không.

---

#### 6. [Spec-B02] [Nghiêm trọng] Gửi nhầm gói `S→C 0x1A 03` khi cộng/trừ Điểm thưởng (`rc=5, p_style=3/4`) khiến Client `aLogin.exe` bật thông báo lỗi *"Số tiền đạt giới hạn"*
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L49): *"cộng/trừ Kim hoặc Điểm (`Gold/Points` `rc=5`)... tôi muốn tất cả hiệu ứng này hoạt động thật và phát đúng các gói đồng bộ chuẩn (`0x08`, `0x16`, `0x17`, `0x18`, `0x1A`, `0x14`, `0x0C`) cho client."*
  - [RESEARCH_AND_GAP_ANALYSIS.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/RESEARCH_AND_GAP_ANALYSIS.md#L132) (dịch ngược `aLogin.exe` `FUN_0079175f` tại `case_023_0079175F_FUN_0079175f.c`):
    - `0x1A 01 <u32>`: Cộng Kim (`+Gold`)
    - `0x1A 02 <u32>`: Trừ Kim (`-Gold`)
    - `0x1A 03`: **Thông báo đạt giới hạn tiền vàng** (`FUN_00729d84(0xf, 0x37)` — *"Số tiền đạt giới hạn"*)
    - `0x1A 04 <u32_gold><u32_point>`: Ghi đè đồng bộ tổng Gold & Shop Point
    - `0x1A 05 <u32>`: Cộng Điểm (`+Shop Points` tại `+0x1300`)
    - `0x1A 06 <u32>`: Trừ Điểm (`-Shop Points` tại `+0x1300`)
- **Vị trí trong Codebase**:
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1264-L1284):
    ```rust
    3 => {
        if amount > 0 {
            let add = amount as u32;
            session.shop_point = session.shop_point.saturating_add(add).min(crate::protocol::POINT_CAP);
            out.send(format!("F44406001A05{}", encoder::le32(add)));
            out.send(crate::server::handlers::shops::point_frame(session.shop_point));
            crate::server::session::registry_update_if_online(session);
        }
    }
    4 => {
        if amount > 0 {
            let sub = amount as u32;
            session.shop_point = session.shop_point.saturating_sub(sub);
            out.send(crate::server::handlers::shops::point_frame(session.shop_point));
            crate::server::session::registry_update_if_online(session);
        }
    }
    ```
  - [shops.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L42-L44): `point_frame` đóng gói `F44406001A03<le32>`.
- **Giải thích dễ hiểu (Chuyện gì xảy ra ở Runtime)**:
  - Trong `aLogin.exe`, sub-opcode `0x1A 03` **không phải** là gói cập nhật tổng điểm thưởng, mà là gói báo lỗi tràn ví tiền vàng!
  - Do đó:
    1. Khi người chơi được **cộng điểm thưởng** (`p_style = 3`), server gửi `0x1A 05` (client đã tự cộng `add` vào biến điểm `+0x1300` và hiện dòng chữ nhận điểm), rồi gửi bồi thêm `0x1A 03` làm client bật thông báo lỗi *"Số tiền đạt giới hạn"*!
    2. Khi người chơi bị **trừ điểm thưởng** (`p_style = 4`), server không gửi `0x1A 06 <sub_u32>` (nên biến điểm trên client không hề giảm), mà lại gửi `0x1A 03` báo lỗi *"Số tiền đạt giới hạn"*.
- **Cách khắc phục**: Ở `p_style = 3`, chỉ gửi `0x1A 05 <add_u32>` (hoặc đồng bộ qua `0x1A 04 <gold_u32><shop_point_u32>`). Ở `p_style = 4`, gửi `0x1A 06 <sub_u32>`.

---

#### 7. [Spec-B03] [Nghiêm trọng] Va chạm chỉ số `stt` (`5..=10`) giữa 3 kho võ tướng `Cart (2)`, `Hotel (3)`, `Warehouse (4)` khi nạp vào `Session.pets`
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L58) (User Story 27) & [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L126): *"Bảo toàn nguyên vẹn `PetStorageType` (`Cart = 2`, `Hotel = 3`, `Warehouse = 4`) khi đọc/ghi sủng vật."*
- **Vị trí trong Codebase**:
  - [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L240-L246) (`load_pets`):
    ```rust
    stt: if storage_type == PetStorageType::Carried {
        slot
    } else {
        slot.saturating_add(4)
    },
    ```
  - [pet_actions.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L32-L35), [pet_actions.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L47-L51), [pet_actions.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L74-L97)
  - [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1749-L1755) (`BattleService::pet_slots`)
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L513-L532) (`execute_reward_pet`, nhánh `p_style = 2`)
- **Giải thích dễ hiểu (Chuyện gì xảy ra ở Runtime)**:
  - Trong bảng `character_pets`, khóa chính là `(playerid, storagetype, slot)`. Một người chơi có thể có 1 võ tướng ở Xe ngựa (`Cart = 2, slot = 1`) và 1 võ tướng ở Khách sạn (`Hotel = 3, slot = 1`).
  - Khi `load_pets` nạp từ DB lên `session.pets`, cả hai võ tướng này đều bị gán **`stt = 1 + 4 = 5`**!
  - Trong khi đó, các nơi xử lý võ tướng lại đang giả định `stt` là duy nhất trong `session.pets`:
    1. **Rút/Gửi nhầm tướng ở Khách sạn (`pet_actions.rs`)**: Khi người chơi rút tướng ở ô 1 Khách sạn (`stt = 5`), `move_pet_slot` tìm phần tử đầu tiên có `p.stt == 5`. Vì `load_pets` sắp xếp theo `ORDER BY storagetype, slot`, tướng ở `Cart (2)` đứng trước `Hotel (3)` nên server **rút nhầm tướng từ Xe ngựa ra thay vì tướng ở Khách sạn**! Ngoài ra khi rút ra `Carried`, `move_pet_slot` chỉ đổi `p.stt` về `1..=4` mà **không cập nhật `p.storage_type = PetStorageType::Carried`**.
    2. **Đưa nhầm tướng trong kho vào trận đấu (`BattleService::pet_slots` tại [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1751))**: Hàm viết `for (i, pet) in session.pets.iter().take(4).enumerate()` mà không lọc `pet.storage_type == PetStorageType::Carried`. Nếu người chơi chỉ mang 1 tướng theo người nhưng có 3 tướng gửi ở Xe ngựa/Khách sạn, cả 3 tướng trong kho cũng bị lôi vào danh sách slot tướng của trận đánh!
- **Cách khắc phục**:
  - Mọi thao tác tìm kiếm/di chuyển trong `pet_actions.rs`, `BattleService::pet_slots`, và `execute_reward_pet` phải lọc rõ theo `pet.storage_type` (và cập nhật `pet.storage_type` khi di chuyển giữa `Carried` ↔ `Hotel`/`Cart`).

---

#### 8. [Spec-B04] [Nghiêm trọng] Lỗi khi chuỗi sự kiện kết thúc bằng `Warp` (hoặc có `Warp` sau trận đấu): không tăng `completed_eve_counts`, bỏ sót hiệu ứng sau `Warp`, và làm rớt `Warp` của người chơi disconnect giữa trận
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L51) (User Story 20): *"Là một Người chơi tham gia trận chiến nhiệm vụ Eve và bị mất kết nối mạng (disconnect) giữa trận, tôi muốn khi trận chiến kết thúc trên server, hệ thống vẫn chạy đầy đủ hậu sự kiện chiến đấu (`Win`/`Lose`/`Flee`) và lưu trực tiếp trạng thái mới xuống DB, để khi đăng nhập lại tôi nhận đúng tiến độ/hậu quả của trận đó."*
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L55) (User Story 24): *"số lần hoàn thành sự kiện (`completed_eve_counts`) được tách biệt hoàn toàn theo cặp `(scene_id, eve_no)`"*
- **Vị trí trong Codebase**:
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L803-L828) (`DomainEffect::Warp` trong `execute_event_step_with_pool_opts`)
  - [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L589-L624) và [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L644-L678) (`BattleService::battle_ended`)
- **Giải thích dễ hiểu (Chuyện gì xảy ra ở Runtime)**:
  1. **Không tăng `completed_eve_counts[(scene_id, eve_no)]` khi chuỗi có `Warp` ([talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L803-L828))**:
     Khi gặp `DomainEffect::Warp`, hàm lập tức xóa `session.current_event_session = None; return;` mà **không tăng `completed_eve_counts[(scene_id, eve_no)]`** (vốn chỉ nằm trong `finish_event_session_with_pool_iter`). Do đó mọi điều kiện `ConditionClass = 12` kiểm tra xem người chơi đã đi qua sự kiện dịch chuyển đó chưa sẽ luôn thấy `0`. Ngoài ra, nếu trong mảng `results` của sự kiện đó có các hiệu ứng đặt sau lệnh `Warp` (ví dụ cộng bước nhiệm vụ sau khi warp), chúng bị xóa bỏ hoàn toàn.
  2. **Thứ tự ghi DB ngược đối với người chơi rớt mạng giữa trận có `Warp` hậu chiến ([service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L589-L654))**:
     Khi người chơi disconnect giữa trận (`offline_parked`), `resume_eve_after_battle_deferred_warp` được gọi với `defer_warp = true` (dòng 595). Khi gặp `Warp`, nó **dừng ngay tại `Warp`** mà chưa đổi tọa độ `offline_s.map_id` và cũng không chạy các hiệu ứng nằm sau `Warp`. Sau đó ở dòng 623–639, `offline_s` (với tọa độ cũ) được lưu xuống DB lần 1, rồi nhả khóa `_op_guards` ở dòng 642, rồi mới chạy `perform_party_warp_and_persist` ở dòng 646 ngoài khóa!

---

#### 9. [Spec-B05] Thiếu phát gói `S→C 0x14 01` khi thực thi `MutateItem` (`rt=0, rc=1`) khiến Client `aLogin.exe` không hiện thông báo nhận/mất vật phẩm
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L95): *"chỉ cho phép gửi `0x14 01..06` đối với `result_type ∈ 0..=6` mà client xử lý được"*
  - [EVE_BINARY_ANALYSIS_ALOGIN.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md#L62-L67): Trong `FUN_005eb530` của `aLogin.exe`, khi nhận gói `0x14 01` có `rt=0, rc=1`, client tra tên vật phẩm từ `Item.dat` theo `param` và hiển thị dòng thông báo nhận/mất vật phẩm lên màn hình.
- **Vị trí trong Codebase**:
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1103-L1120) (`execute_action_result_non_skill`, nhánh `rc == 1`).
- **Giải thích dễ hiểu**:
  - Trong khi `RewardPet` (`rt=0, rc=8`, [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L483)) và `NpcTeam` (`rt=0, rc=3`, [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L653)) đều gửi kèm gói `NpcTalkCodec::build_eve_result_hex(raw_result)` (`0x14 01`) để client hiển thị hiệu ứng/thông báo, thì nhánh `rc == 1` (`MutateItem`) chỉ âm thầm cập nhật `homdo` (`0x17 05`) mà không gửi `0x14 01`, khiến người chơi nhận hoặc mất vật phẩm nhiệm vụ mà không thấy dòng thông báo hệ thống từ `aLogin.exe`.

---

#### 10. [Spec-B06] Sự kiện vùng bước chân (`WalkStep`, `trigger_kind = 16`) bị bỏ sót trong `EveAutoChainEngine` và quét lại sai vùng khi chọn Menu `0x14 09`
- **Trích dẫn Spec**:
  - [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md#L35) (User Story 5): *"mọi tương tác chạm NPC (`ClickNpc`), nhấn vào cửa (`ClickDoor`), đi qua vùng cửa (`MeetDoor`), và sự kiện bước đi trên map (`WalkStep`) đều kích hoạt đúng kịch bản Eve tương ứng..."*
- **Vị trí trong Codebase**:
  - [auto_chain.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L287) (`const AUTO_CHAIN_TRIGGER_KINDS: [i32; 3] = [1, 4, 8];`) và [auto_chain.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L310-L327) (`event_list_for`).
  - [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1759-L1766) (`handle_talk_select_menu`).
- **Giải thích dễ hiểu**:
  - Tại [auto_chain.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L287), danh sách trigger được phép `AutoChain` chỉ có `[1, 4, 8]` (NPC và Door), thiếu `16` (`WalkStep`), và hàm `event_list_for` cũng chỉ tìm trong `scene.npcs` và `scene.doors` mà không tìm trong `scene.encounters`. Do đó một sự kiện `WalkStep` có nhiều bước nối tiếp (hoặc vừa đánh trận `WalkStep` xong cần tự nối sang bước nhận thưởng `OnWin`) sẽ **không thể AutoChain**!
  - Tại [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L1764), khi sự kiện `WalkStep` mở menu `0x14 09`, server gọi lại `NpcTrigger::WalkStep { x: mx, y: my }` (quét lại tất cả các vùng encounter chồng lấn tại tọa độ `(mx, my)`) thay vì tra đúng `encounter_id == ev.npc_click_id` đã mở menu đó.

---

**Tóm tắt (One-line summary)**: **Standards**: 10 phát hiện (5 vi phạm quy chuẩn cứng, 5 code smells; nghiêm trọng nhất là **[HV-03]** vi phạm Two-Phase State Lock khi không rollback RAM nếu lỗi DB, nhả khóa sớm trong `battle_ended` và tách rời transaction SQLite giữa các thành viên tổ đội); **Spec**: 10 phát hiện (3 thiếu/một phần, 1 scope creep, 6 lỗi triển khai/giao thức; nghiêm trọng nhất là **[Spec-B01]** `preflight_probe_batch` dừng ngay tại câu thoại `TalkStep` đầu tiên làm gãy cam kết `All-or-Nothing` và **[Spec-B02]** gửi nhầm gói `0x1A 03` khi cộng/trừ điểm thưởng làm bật thông báo lỗi *"Số tiền đạt giới hạn"* trên `aLogin.exe`).
