# HANDOFF: TS Online Dream Server — Pet Stat Recompute & Pet Equipment Integration

> **Ngày tạo**: 2026-10-14  
> **Ngữ cảnh**: Nghiên cứu source C# (Bear), hoàn thiện phần còn thiếu tại Step 8 (`Pet stat recompute (no packet)`) trong chuỗi login `build_logined_sequence_session_ext` (`src/server/spawn.rs`), bổ sung cơ chế tính toán chỉ số trang bị Pet, ngũ hành, tích hợp vào đấu trường Battle Grid và luồng live equip/unequip.

---

## 1. Tóm tắt công việc đã hoàn thành

### A. Nghiên cứu đối chiếu kiến trúc C# (Bear) vs `ts_dream` (Rust)
- **C# Bear (`TSCharacter.cs:568, 1086-1142` & `TSPet.cs:231-312, 429-477`)**:
  - Khi login (`loginChar()`), server gọi `loadCharDB()` -> `loadPet()`.
  - Mỗi pet được gọi `loadPetDB()`: nạp chỉ số gốc và đọc trang bị từ cột JSON `equip`.
  - Trong `loadEquipmentJson()`: tính ngay các bonus `addEquipBonus`, `addEquipSoul`, kiểm tra ngũ hành tương thích (`other_type == pet.element || other_type == 5`), set bonus 5 món cùng hệ (`evaluateElementSetBonus`) và tính lại `hp_max`, `sp_max`.
  - **Lý do Step 8 là `(no packet)`**: Gói `0F 08` (`sendPetInfo`) đã gửi danh sách 6 món trang bị. Client TS Online tự đọc thông tin item từ file data phía client để hiển thị chỉ số bonus màu xanh trên giao diện F2/F3. Server chỉ cần duy trì in-memory trên RAM session để phục vụ tính toán sát thương Battle. Vì vậy không phát sinh gói tin gửi ra socket ở Step 8.

### B. Bộ tính toán chỉ số trang bị Pet (`src/server/character_sheet.rs`)
- **Tập tin**: [`src/server/character_sheet.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/character_sheet.rs)
- Refactor `GearBonuses::from_gear` thành `from_gear_range` hỗ trợ lọc dải slot linh hoạt.
- Thêm `GearBonuses::from_pet_gear(trangbi, pet_stt, pet_element)` tính toán trang bị thuộc slot `(pet_stt * 10 + 1)..=(pet_stt * 10 + 6)`.
- Bổ sung struct `PetSheet` và method `PetSheet::recompute(...)`:
  - Tính toán dồn flat stat: `int1 + int2`, `atk1 + atk2`, `def1 + def2`, `hpx1 + hpx2`, `spx1 + spx2`, `agi1 + agi2`.
  - Áp dụng bonus tương thích hệ nguyên tố: nếu `thuoctinh == pet_element || thuoctinh == 5`, cộng thêm `giatri_thuoctinh` và `giatri_long`.
  - Tính toán `hp_max` và `sp_max` cho Pet theo công thức chuẩn `get_hp_max` / `get_sp_max` của TS Online.

### C. Recompute API trong Session & PetState (`src/server/session.rs`)
- **Tập tin**: [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs)
- Bổ sung `impl PetState` với method `recompute_stats(&mut self, trangbi: &[InventoryItem])`.
- Bổ sung `Session::recompute_pet_stats(&mut self)` để duyệt toàn bộ pet mang theo (`stt in 1..=4`).
- Tích hợp `self.recompute_pet_stats()` vào bên trong `Session::recompute_stats()`. Nhờ đó, khi session chạy `Session::hydrate_templates()` lúc đăng nhập, stats của cả nhân vật và toàn bộ Pet đều được recompute tự động và đồng bộ.

### D. Chuẩn hóa Wire Format và tài liệu Step 8 (`src/server/spawn.rs`)
- **Tập tin**: [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs)
- Trong `pet_stat_entry`: Thay thế chuỗi mock 4-byte ID thành cấu trúc 10-byte wire chuẩn hóa của từng món trang bị thông qua `encode_entry_10(item)` (`le16(id) + count + long_val + biased_giatri_long + khang + le32(texp)`), hoặc 10 byte `0x00` nếu ô trống.
- Bổ sung tài liệu giải thích rõ Step 8 tại `build_logined_sequence_session_ext`.

### E. Tích hợp Đấu trường Turn-based Battle Grid (`src/battle/construction.rs`)
- **Tập tin**: [`src/battle/construction.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs)
- Cập nhật hàm `Battle::add_pet`: cộng trực tiếp các chỉ số trang bị `pet.int2`, `pet.atk2`, `pet.def2`, `pet.agi2` vào ô chiến đấu `cell` (đồng bộ với cách tính của người chơi tại `add_player`).

### F. Handlers Mặc / Tháo đồ trực tiếp (`src/server/handlers/inventory.rs`)
- **Tập tin**: [`src/server/handlers/inventory.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs)
- Trong `handle_pet_equip` (Opcode `0x17 sub 17`) và `handle_pet_unequip` (Opcode `0x17 sub 18`):
  - Kích hoạt `conn.session.recompute_pet_stats()` ngay khi cập nhật danh sách `trangbi`.
  - Persist lại snapshot chỉ số mới của pet vào database qua `persist::upsert_pet`.

---

## 2. Trạng thái kiểm thử & Verification

- **Integration Tests chuyên sâu**: [`tests/pet_equip_unequip_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/pet_equip_unequip_test.rs)
  - `test_pet_stat_recompute_and_elemental_affinity`: **PASSED**.
  - `test_pet_battle_grid_includes_equipment_bonus`: **PASSED**.
  - `test_live_pet_equip_unequip_recomputes_session_stats`: **PASSED**.
  - `test_pet_summary_wire_format_includes_10_byte_equipment`: **PASSED**.
  - `test_pet_equip_bounds_and_player_equip_persist`: **PASSED**.
  - `test_pet_equip_and_unequip_flow`: **PASSED**.
- **Toàn bộ Test Suite**: `cargo test --all-targets --no-fail-fast` đạt **100% PASSED** (0 failures).
- **Trạng thái Git**: Mọi thay đổi tuân thủ quy định **không tự ý git commit**; mã nguồn sẵn sàng để người dùng commit thủ công.

---

## 3. Danh sách các tập tin đã chỉnh sửa

1. [`src/server/character_sheet.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/character_sheet.rs): Thêm `PetGearBonuses` và `PetSheet`.
2. [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs): Thêm `impl PetState`, `Session::recompute_pet_stats`, tích hợp vào `recompute_stats`.
3. [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs): Chuẩn hóa 10-byte equip wire encoding trong `pet_stat_entry` và tài liệu Step 8.
4. [`src/battle/construction.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs): Cộng `pet.int2..agi2` trong `Battle::add_pet`.
5. [`src/server/handlers/inventory.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs): Recompute và persist pet stats sau equip/unequip.
6. [`tests/pet_equip_unequip_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/pet_equip_unequip_test.rs): Thêm 4 test cases kiểm thử tự động.

---

## 4. Suggested Skills

Khi tiếp tục phiên làm việc tiếp theo, agent nên sử dụng các skills sau:
- **`reverse-engineer-anything`**: Sử dụng khi cần phân tích sâu thêm các cấu trúc dữ liệu nhị phân hoặc luồng xử lý từ client Delphi (`client_delphi/aLogin.exe`) hoặc các packet dump thực tế.
- **`agy-customizations`**: Tra cứu các quy chuẩn agent, system prompt, và kiến trúc hệ thống.

---

## 5. Hướng dẫn cho phiên làm việc tiếp theo

1. **Commit thủ công**: Người dùng review và thực hiện `git add` & `git commit` cho các tập tin đã chỉnh sửa.
2. **Kế hoạch tiếp theo**: Có thể tiếp tục triển khai các mục trong backlog hoặc các opcode capture live tại [`.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md).
