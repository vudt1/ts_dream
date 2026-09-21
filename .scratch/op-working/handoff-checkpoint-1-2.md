# Handoff: NPC Talk & Eve Script System (Opcodes 0x14 & 0x18)

## 1. Summary of Work Completed

Tài liệu thiết kế chi tiết và lộ trình tổng thể:
- Kế hoạch: [`.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md)
- Research Checkpoint 1: [`.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md)
- Research Checkpoint 2: [`.scratch/op-working/research-checkpoint-2-click-npc-bridge.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-2-click-npc-bridge.md)
- Research Checkpoint 3: [`.scratch/op-working/research-checkpoint-3-talk-packets.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-3-talk-packets.md)

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

---

## 2. Current State & Next Steps

Hệ thống đã sẵn sàng cho **Checkpoint 3**:

- **Checkpoint 3: Triển khai Talk Packets Transmission & Actor Lock**:
  - Gửi gói tin khóa thao tác di chuyển (`0x14 Sub 0x2C`) khi bắt đầu vào thoại (`TalkLockMode::Lock`).
  - Gửi gói tin thoại `0x14 Sub 0x01` chứa đúng 14 bytes binary payload từ kết quả `EveResultWire`.
  - Nghiên cứu chi tiết nằm tại: [`.scratch/op-working/research-checkpoint-3-talk-packets.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-3-talk-packets.md).
- **Các checkpoint tiếp theo**:
  - Checkpoint 4: Talk Continue & Menu Selection (`0x14 Sub 0x01` step tiếp theo, `0x14 Sub 0x02` chọn menu).
  - Checkpoint 5: Quest Synchronization (`0x18 Sub 0x01..0x08`).
  - Checkpoint 6: Auto-chaining, Trigger Battle, và Persist.

---

## 3. Testing Rules Reminder
Tuân thủ tuyệt đối quy định trong [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md):
- **Cấm** viết `#[cfg(test)]` inline trong thư mục `src/`.
- Mọi unit / integration test mới bắt buộc phải nằm ở thư mục `tests/`.
- Chạy kiểm tra: `cargo test --test <tên_test>`.

---

## 4. Suggested Skills
Khi tiếp tục làm việc, các subagents hoặc agent kế tiếp nên sử dụng:
- `DeepCoder`: Cho các bước implement mã nguồn và chạy test suite.
- `DeepInvestigator`: Khi cần tra cứu decompile Ghidra C của `aLogin.exe` hoặc cấu trúc opcode trong client.
