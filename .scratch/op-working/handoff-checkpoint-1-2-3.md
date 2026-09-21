# Handoff: NPC Talk & Eve Script System (Opcodes 0x14 & 0x18)

## 1. Summary of Work Completed

Tài liệu thiết kế chi tiết và lộ trình tổng thể:
- Kế hoạch: [`.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md)
- Research Checkpoint 1: [`.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md)
- Research Checkpoint 2: [`.scratch/op-working/research-checkpoint-2-click-npc-bridge.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-2-click-npc-bridge.md)
- Research Checkpoint 3: [`.scratch/op-working/research-checkpoint-3-talk-packets.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-3-talk-packets.md)
- Research Checkpoint 4: [`.scratch/op-working/research-checkpoint-4-talk-continue-menu.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-4-talk-continue-menu.md)
- Research Checkpoint 5: [`.scratch/op-working/research-checkpoint-5-quest-sync.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-5-quest-sync.md)
- Research Checkpoint 6: [`.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md)

### Checkpoint 1 (Đã hoàn thành 100%)
- **Mục tiêu**: Xây dựng toàn bộ Wire Codec cho Opcode `0x14` (Hội thoại NPC / Lock di chuyển) và Opcode `0x18` (Đồng bộ Quest / Vật phẩm) chuẩn hóa theo decompile Ghidra C client `aLogin.exe` (`FUN_0078ec3f`, `FUN_00790ed5`).
- **Mã nguồn triển khai**:
  - [`src/protocol/codecs/npc_talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs): `EveResultWire` (chuẩn 14 bytes LE), `talk_step_packet` (`0x14 0x01`), `lock_unlock_actor_packet` (`0x14 0x2C`), `end_talk_packet` (`0x14 0x08`), enum `TalkLockMode`.
  - [`src/protocol/codecs/quest_sync.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/quest_sync.rs): 8 SubOpcodes tương ứng `0x18 Sub 0x01..0x08` (actor state, item add/remove, quest task/dont single & bulk, quest clear/full).
  - [`src/protocol/reader.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/reader.rs) & [`src/protocol/writer.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/writer.rs): Tích hợp đọc/ghi các struct nhiệm vụ và thoại.
  - [`src/protocol/codecs/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/mod.rs) & [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs): Export các module.
- **Kiểm thử**: [`tests/wire_codec_14_18_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/wire_codec_14_18_test.rs) đạt **11/11 PASS (100%)**.

### Checkpoint 2 (Đã hoàn thành 100%)
- **Mục tiêu**: Đấu nối sự kiện Click NPC (`Opcode 0x14`) từ Dispatcher vào Eve Engine Resolver.
- **Mã nguồn triển khai**:
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs):
    - Kiểm tra bán kính khoảng cách player - NPC ($\le 150$px). Nếu quá xa hoặc NPC không tồn tại, trả về ngay frame `0x14 Sub 0x08` (`F4 44 02 00 14 08`) để giải phóng client an toàn.
    - Xây dựng `EveStateBuilder::build_from_session()`, phân giải qua `eve_engine.resolve_npc_event(...)`.
    - Khi có kết quả: Khởi tạo và gán `current_event_session` vào `Session`, gửi packet kích hoạt mở thoại ban đầu (`0x06 Sub 0x02`).
    - Khi không khớp kịch bản: fallback sang talk thường an toàn.
  - [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs): Bổ sung `pub current_event_session: Option<ActiveEveSession>`.
- **Kiểm thử**: [`tests/npc_eve_resolve_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_eve_resolve_test.rs) đạt **3/3 PASS (100%)** với dữ liệu `eve.emg` thực tế của Trác Quận và các edge cases (NPC xa >150px, NPC không tồn tại, NPC có điều kiện thỏa mãn / không thỏa mãn).

### Checkpoint 3 (Đã hoàn thành 100%)
- **Mục tiêu**: Truyền phát packet thoại 14-byte payload (`0x14 Sub 0x01`) và khóa/mở khóa thao tác nhân vật (`0x14 Sub 0x2C`) khi bắt đầu/kết thúc thoại — đúng thứ tự wire của Bear `ClickkNpc`/`processStep`.
- **Mã nguồn triển khai**:
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs):
    - Import `NpcTalkCodec`, `TalkLockMode` từ `crate::protocol::codecs::npc_talk`.
    - `handle_talk_start` (nhánh Eve match): sau khi resolve được `event_session`, build hex talk step của `results[0]` (chỉ khi `result_type == 1`) **trước khi move** vào `conn.session.current_event_session`. Thứ tự frame phát đi:
      1. `F44402000602` — mở bảng thoại
      2. `F4440700142C<char_id LE>01` — Lock actor (`NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock)`, `char_id = conn.session.id as u32`)
      3. `F4441100140100<14B EveResult>` — talk step đầu tiên (byte-perfect từ `build_talk_step_hex`)
      - Non-talk results (`result_type` 0 Action / 2 Door / 6 Surface) đánh dấu `// TODO(CP4)` — chưa dispatch ở checkpoint này.
    - `end_talk`: khi `conn.session.current_event_session.is_some()` → gửi Unlock (`...14 2C ... 02`) **trước** `F44402001408`. Legacy path (không có event session) giữ nguyên frame đơn `F44402001408` để bảo toàn golden parity.
  - [`tests/db_repository_init_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/db_repository_init_test.rs): Fix lỗi pre-existing — 3 struct literal `Session` thiếu field `current_event_session` (kế thừa từ Checkpoint 2) khiến `cargo test --all-targets` không compile → thêm `current_event_session: None`.
- **Kiểm thử**: [`tests/npc_talk_transmit_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_talk_transmit_test.rs) đạt **3/3 PASS (100%)**:
  - `test_click_npc_emits_lock_then_first_talk_step`: assert thứ tự index `0602` < lock `F4440700142CE903000001` < talk step `F4441100140100...7C28` (mean_no 10364 LE).
  - `test_end_talk_emits_unlock_before_close`: unlock `F4440700142CE903000002` xuất hiện trước `F44402001408`.
  - `test_talk_step_payload_matches_eve_result`: frame emit khớp byte-perfect với `build_talk_step_hex(&ev.results[0])`, decode đúng 21 bytes.
- **Kết quả toàn suite**: `cargo test --all-targets --no-fail-fast` → **112 tests, 0 failed** (19 targets).

---

## 2. Current State & Next Steps

Hệ thống đã sẵn sàng cho **Checkpoint 4**:

- **Checkpoint 4: Talk Continue & Menu Selection (`0x14 Sub 0x06` & `0x14 Sub 0x09`)**:
  - `0x14 Sub 0x06` (Next, payload đúng 2 bytes `[0x14, 0x06]`): tăng `current_index` trong `current_event_session`; nếu result kế là Talk (`type 1`) → gửi talk step kế; nếu Action (`type 0`) → thi hành cập nhật túi đồ/chỉ số rồi duyệt tiếp không dừng; nếu Surface (`type 6`) → dựng UI menu và chờ chọn; nếu hết results → Unlock actor (`14 2C 02`), gửi `14 08` EndTalk, kiểm tra `auto_chain_after`.
  - `0x14 Sub 0x09` (chọn menu, payload 3 bytes `[0x14, 0x09, ChoiceCode: 1B]`): `ChoiceCode` 1-based; đánh giá `condition_class == 10` với `(last_surface_id, choice_code)` để kích hoạt nhánh kịch bản mới.
  - Thiết kế sẵn: hàm điều khiển trung tâm `execute_event_step` trong `src/server/handlers/talk.rs`; test plan `tests/npc_talk_multistep_test.rs`.
  - Nghiên cứu chi tiết: [`.scratch/op-working/research-checkpoint-4-talk-continue-menu.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-4-talk-continue-menu.md).
- **Các checkpoint tiếp theo**:
  - Checkpoint 5: Quest Synchronization (`0x18 Sub 0x01..0x08`) — codec đã sẵn sàng từ Checkpoint 1, cần handler + Session fields (`quest_tasks`, `quest_dont`, `quest_items`).
  - Checkpoint 6: Auto-chaining, Trigger Battle (`result_type == 3` → `BattleTrigger::Eve`), Door/Warp (`result_type == 2`), và SQLite Persist.

---

## 3. Testing Rules Reminder
Tuân thủ tuyệt đối quy định trong [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md):
- **Cấm** viết `#[cfg(test)]` inline trong thư mục `src/`.
- Mọi unit / integration test mới bắt buộc phải nằm ở thư mục `tests/`.
- Chạy kiểm tra: `cargo test --test <tên_test>`.
- Khi chạy lại toàn suite: `cargo test --all-targets --no-fail-fast`.

---

## 4. Suggested Skills
Khi tiếp tục làm việc, các subagents hoặc agent kế tiếp nên sử dụng:
- `DeepCoder`: Cho các bước implement mã nguồn và chạy test suite.
- `DeepInvestigator`: Khi cần tra cứu decompile Ghidra C của `aLogin.exe` hoặc cấu trúc opcode trong client.
