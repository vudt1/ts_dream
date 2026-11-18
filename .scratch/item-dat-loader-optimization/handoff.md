# TÀI LIỆU BÀN GIAO CÔNG VIỆC (HANDOFF)

**Chuyên đề:** Tối ưu hóa nạp nhị phân `Data/Item.dat` cho Server và Hợp nhất bảng `items` trên `GameData`

- **Ngày thực hiện:** 2026-11-18
- **Feature slug:** `item-dat-loader-optimization`
- **Tài liệu liên quan (không lặp lại nội dung chi tiết):**
  - Nghiên cứu & phân loại 54 trường: [research-item-dat-loader.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/item-dat-loader-optimization/research-item-dat-loader.md)
  - Đặc tả kỹ thuật: [spec.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/item-dat-loader-optimization/spec.md)
  - Ticket thực thi (`Status: completed`): [01-optimize-item-dat-server-loader.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/item-dat-loader-optimization/issues/01-optimize-item-dat-server-loader.md)
  - Quy chuẩn & ADR liên quan: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md), [ADR 0005](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0005-economy-wire-parity-and-transactions.md), [ADR 0006](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md), [ADR 0007](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0007-compound-wire-parity-and-crafting-engine.md)

---

## 1. Trạng thái hiện tại

- **Ticket `#01` đã hoàn thành (`Status: completed`)**, đã qua `/code-review` (trục **Standards** và **Spec**) và đã sửa xong toàn bộ điểm cần khắc phục sau review.
- **Trạng thái Git**: Base commit `710565efad8719b6b8d28975966a1389f3a506ed` (`HEAD`). Theo quy tắc trong `AGENTS.md` (*User tự commit thủ công; giữ thay đổi ở working tree*), toàn bộ thay đổi đang nằm ở **working tree** chưa commit (`git diff HEAD` để xem chi tiết).

---

## 2. Tóm tắt công việc đã hoàn thành

### 2.1 Mã nguồn `src/`
- [src/data/loaders/item.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/item.rs) & [src/data/loaders/mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/mod.rs): Loại bỏ hoàn toàn struct `ItemDef` (54 trường); `ItemDatLoader::load` giải mã thẳng sang `HashMap<i64, Item>`, bỏ qua vùng `description`/`description_viscii` (`115..370`), icon/pic, color tints, UI flags và trường mobile chưa dùng; gộp nhánh `attr1_kind`/`attr2_kind` qua helper `apply_attr`.
- [src/data/tables.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/tables.rs#L70-L87): Bổ sung `price: u32`, `sell_price: u32` và `Item::utf8_name(&self)` giải mã on-demand từ byte VISCII 1.1 bằng `encoding::viscii_decode`.
- [src/data/loader.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs): Xóa `GameData.item_defs`, nạp trực tiếp vào `GameData.items` (giữ nguyên fallback `Items.txt` với `price = 0, sell_price = 0`).
- [src/crafting/basic/validator.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/crafting/basic/validator.rs), [src/crafting/basic/algorithm.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/crafting/basic/algorithm.rs), [src/server/handlers/crafting.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/crafting.rs), [src/server/handlers/shops.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs): Chuyển toàn bộ sang đọc trực tiếp từ `data.items`.
- [src/battle/manager.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/manager.rs) & [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs): Bổ sung `BattleTables::Shared(Arc<GameData>)` và `BattleManager::spawn_with_game_data` để `BattleService` chia sẻ `Arc::clone(&self.data)` thay vì clone `HashMap`.

### 2.2 Kiểm thử `tests/`
- Mới: [tests/data_loaders/item.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders/item.rs) (đăng ký trong [tests/data_loaders_test.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders_test.rs)).
- Đã cập nhật từ `item_defs`/`ItemDef` sang `items`/`Item`: [tests/compound_test.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/compound_test.rs), [tests/economy_inventory/economy_wire.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/economy_wire.rs), [tests/economy_inventory/inventory_template_hydrate.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/inventory_template_hydrate.rs), [tests/economy_inventory/pet_equip_unequip.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/pet_equip_unequip.rs).

---

## 3. Lưu ý kỹ thuật & Quyết định thiết kế quan trọng

1. **`Item::utf8_name()` bắt buộc dùng `encoding::viscii_decode`**: Ban đầu triển khai nháp dùng `cp1252_to_string`, nhưng khi truyền qua `sys_msg_frame` (`viscii_encode`), các byte VISCII trong dải `0x80..=0x9F` bị biến thành ký tự Unicode `> 0xFF` (như `Š`) và rơi về `'?'`. Đã chuyển sang `viscii_decode(&self.name)` để round-trip chuẩn VISCII 1.1.
2. **Giữ `GameData.items` là `HashMap<i64, Item>`**: Thay vì đổi kiểu của `GameData.items` sang `Arc<...>` (ảnh hưởng hàng loạt setup test), `BattleService` dùng `BattleManager::spawn_with_game_data` để chia sẻ `Arc<GameData>` sẵn có.
3. **`attr*_kind` `65..=67` (`+100`)**: Logic `decode_attr_value` giữ nguyên quy tắc cộng `+100`, tuy nhiên `tables::Item` hiện không ánh xạ `65..=67` sang trường nào trên `Item` (giống `ItemDef::to_item()` trước đây), nên tác động của `+100` chưa quan sát được qua `Item` cho tới khi có trường gameplay tương ứng.

---

## 4. Kết quả kiểm chứng sau chỉnh sửa cuối cùng

Lệnh chạy: `cargo test --all-targets --no-fail-fast` — **thoát mã 0 (0 failed, 0 ignored)**:

- `architecture_guardrails_test`: 8 passed
- `battle_test`: 186 passed
- `compound_test`: 21 passed
- `data_loaders_test`: 40 passed
- `db_test`: 21 passed
- `economy_inventory_test`: 59 passed
- `eve_quest_test`: 137 passed
- `events_arena_test`: 62 passed
- `server_network_test`: 103 passed
- `server_stop_test`: 1 passed
- `skill_job_system_test`: 19 passed

---

## 5. Việc tiếp theo / Ngoài phạm vi (nếu làm tiếp ở phiên sau)

1. **Commit thay đổi**: User tự kiểm tra `git diff HEAD` và commit thủ công (theo `AGENTS.md`).
2. **Dọn dẹp / tối ưu kế tiếp (Out of Scope đợt này)**:
   - Thu hẹp kiểu các trường số trên `tables::Item` từ `i64` xuống `u16`/`u8`/`i32` để giảm thêm footprint mỗi record.
   - Đóng gói nhóm tham số `spawn_inner` trong `src/battle/manager.rs` thành struct cấu hình nếu mở rộng thêm `BattleManager`.
   - Kiểm tra xem còn trường nào trong 9 trường chưa dùng của `Item.dat` (`special_ability`, `open_used`, `threshold`, `restrict2`, `role_count_value`...) cần đưa vào gameplay hay không.

---

## 6. Suggested Skills

Nếu một agent mới tiếp tục từ điểm bàn giao này:
- **`code-review`**: Nếu cần rà soát lại toàn bộ `git diff HEAD` trước khi user commit hoặc sau khi chỉnh sửa thêm.
- **`tdd`**: Nếu mở rộng thêm trường gameplay mới trên `Item` hoặc thu hẹp kiểu dữ liệu của `tables::Item` (làm test-first tại `Binary Loader Seam` và `GameData & Domain Handler Seam`).
- **`research`**: Nếu muốn khảo sát tương tự để tối ưu các loader `.dat` khác (`Npc.dat`, `Skill.Dat`, `Talk.dat`).
