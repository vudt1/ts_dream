# TÀI LIỆU BÀN GIAO CÔNG VIỆC (HANDOFF)
**Chuyên đề:** Audit & Khắc phục Hệ thống Sủng vật (Pet), Trạng thái Xuất chiến (`character_pets`), Gói tin Client và Cơ chế Rơi đồ (Battle Drop) trong TS Online Server Rust (`ts_dream`)

- **Ngày thực hiện:** 17/11/2026
- **Feature slug:** `pet-and-drop-audit`
- **Tài liệu nghiên cứu gốc:** [research-pet-and-drop-audit.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/research-pet-and-drop-audit.md)
- **Đặc tả kỹ thuật:** [spec-pet-and-drop-fixes.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/spec-pet-and-drop-fixes.md)
- **Thư mục Tickets:** [.scratch/pet-and-drop-audit/issues/](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/)
- **Thuật ngữ Domain:** [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)

---

## 1. TỔNG QUAN YÊU CẦU & KẾT QUẢ NGHIÊN CỨU (RESEARCH FINDINGS)

### A. Hệ thống Sủng vật & Gói tin Client (Câu hỏi 1)
1. **Kiến trúc dữ liệu:**
   - Database SQLite: Bảng `character_pets` ([migrations/0001_init.sql](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql)) lưu 4 vùng (`storagetype`: `1 = Carried` 1..4, `2 = Cart`, `3 = Hotel`, `4 = Warehouse`). Cột `isactive INTEGER DEFAULT 0` quản lý xuất chiến.
   - Runtime RAM: Quản lý tập trung qua biến `session.active_pet_stt: u8` (`0` = không xuất chiến, `1..=4` = slot sủng đang ra trận).
2. **Khảo sát 3 phương thức tiếp nhận:**
   - *Giao dịch (P2P Trade):* Opcode `0x19` Sub `0x0B` & `0x0C` (Bear-dialect). Chống trùng, chống đầy 8 sủng, lưu `persist_sessions_transaction`, gửi frame `0F 07`, `0F 01`, `0F 02`.
   - *Đổi từ vật phẩm (Item 31007 / Thẻ sủng):* Vé 31007 là kịch bản đối thoại EVE Quest (Scene 12003 NPC 8) qua Opcode `0x14 Sub 0x01`. Với thẻ sủng dùng trực tiếp (`add_pet > 10000`), đi qua Opcode `0x17 Sub 0x0F` (`use_item/mod.rs`).
   - *Bắt trong trận (Battle Capture):* Client gửi Opcode `0x32 Sub 0x01` với kỹ năng `15001`. Engine áp dụng 8 Validation Gates (`runner.rs:2360`), xóa quái `0B 01`, gán sủng qua `add_caught`.
3. **Đối chiếu Gói tin Client:**
   - **Hoàn toàn KHÔNG dùng chung gói tin**: Trade (`0x19`), Use Item (`0x17`), Capture (`0x32`).
4. **Trạng thái Xuất chiến (`isactive`):**
   - Sủng mới nhận luôn mặc định **`isactive = 0`** (nghỉ ngơi). Muốn xuất chiến bắt buộc phải gửi lệnh riêng **`Opcode 0x13 Sub 0x01`** (`active_pet_stt = pet.stt`, SQLite `isactive = 1`, frame `13 01`). Lệnh nghỉ ngơi là **`Opcode 0x13 Sub 0x02`**. (Lưu ý: `0x0F Sub 0x02` là lệnh thả/xóa vĩnh viễn sủng).

### B. Cơ chế Drop Item từ NPC trong Battle (Câu hỏi 2)
1. **Dữ liệu `Data/Npcs.txt` vs `Npc.dat`:**
   - Cả 2 file đều chỉ lưu danh sách 6 Item ID (offset 72..83 trong `Npc.dat`, giải mã XOR `0x5209`), **hoàn toàn không lưu tỷ lệ rơi (drop rate)**.
2. **Cách Server Rust tính tỷ lệ rơi đồ:**
   - Hardcode 6 băng xác suất tích lũy `[25, 23, 20, 4, 3, 1]` trên thang 1000 (tổng rate $\approx 7.608\%$, xịt $92.392\%$). Ngẫu nhiên thuần túy, độc lập với level.
3. **Quy tắc PK Battle:**
   - Được khóa chặt bởi `BattleKind::allows_rewards()`: Chặn 100% rơi đồ (`Out::Drop`), rơi vàng (`Out::Gold`), nhận EXP và hao mòn Ngọc Phúc Thần.
4. **Cơ chế Hợp kích (Combo):**
   - **Xác nhận chuẩn xác:** Sủng vật/pet không có hòm đồ riêng (`homdo`), nên việc lấy `owner = caster.id_char` chuyển quyền nhận vật phẩm/vàng về túi đồ của người chơi chủ nhân là hoàn toàn chính xác theo thiết kế game.
   - Khi hợp kích kết liễu quái, mỗi ô (player hoặc pet) tham gia combo thành công đều sinh một phần thưởng độc lập chuyển về túi đồ chủ nhân.

---

## 2. DANH SÁCH TICKETS ĐÃ TRIỂN KHAI & KẾT QUẢ

| Ticket ID | Tên công việc | Trạng thái | Tệp tin sửa đổi |
|:---:|---|:---:|---|
| **01** | [Trade Active Pet Auto-Recall](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/01-trade-active-pet-auto-recall.md) | `completed` | [src/server/trade_system.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs), [trade_storage.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs), [tests/economy_inventory/pet_trade_active_recall.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/pet_trade_active_recall.rs) |
| **02** | [Summon Item Roster Sync](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/02-summon-item-roster-sync.md) | `completed` | [src/server/handlers/trade_storage.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs), [use_item/mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs), [tests/economy_inventory/summon_item_sync.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/summon_item_sync.rs) |
| **03** | [Battle Capture Template Hydrate](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/03-battle-capture-template-hydrate.md) | `completed` | [src/server/pet_box.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs), [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs), [tests/battle/catch_verification.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/catch_verification.rs) |
| **04** | [Battle Drop Bag Capacity Guard](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/04-battle-drop-bag-capacity-guard.md) | `completed` | [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs), [tests/battle/item_frame.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/item_frame.rs) |
| **05** | [Fix Use Item Registry Sync & Slot Guard](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/05-fix-use-item-registry-sync-and-slot-guard.md) | `completed` | [src/server/handlers/use_item/mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs) |
| **06** | [Centralize Pet Recall Constant](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/06-centralize-pet-recall-constant.md) | `completed` | [src/server/handlers/pet_actions.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs), [src/server/handlers/trade_storage.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs) |
| **07** | [Fix Battle Drop Strict 25-Slot Guard](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/07-fix-battle-drop-strict-25-slot-guard.md) | `completed` | [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) |
| **08** | [Refactor Pet Box Clean Mapping](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/08-refactor-pet-box-clean-mapping.md) | `completed` | [src/server/pet_box.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs) |
| **09** | [Expand Regression Tests (Negative & Boundary)](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/issues/09-expand-regression-tests.md) | `completed` | [tests/economy_inventory/summon_item_sync.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/summon_item_sync.rs), [tests/battle/item_frame.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/item_frame.rs) |

---

## 3. CHI TIẾT CÁC THAY ĐỔI MÃ NGUỒN

1. **`src/server/trade_system.rs` & `src/server/handlers/trade_storage.rs`:**
   - Trong `pet_trade_settle`: Reset `active_pet_stt = 0` nếu con pet bị bán trùng với sủng vật đang xuất chiến của bên bán.
   - Trong `handle_pet_confirm`: Bắn frame recall `F44402001302` và cập nhật DB `persist::update_player(pool, player_id, "SttPetXuatchien", 0)`.
   - Xuất khẩu `pub fn received_pet_frame` và `pub fn pet_presence_frame` làm hàm tái sử dụng chung.
2. **`src/server/handlers/use_item/mod.rs`:**
   - Khi `add_pet > 10000`: Lưu ngay vào DB qua `persist::upsert_pet(ctx.pool, ...)` và gửi ngay gói tin `0F 07` + `0F 01` về client giúp pet hiển thị tức thì.
3. **`src/server/pet_box.rs` & `src/battle/service.rs`:**
   - Hàm `add_caught(pets, npc: Option<&Npc>, npc_id)`: Trích xuất đầy đủ Hệ (Element), MaxHP, MaxSP, INT, ATK, DEF, AGI, HPX, SPX, độ trung thành 60 và 3-4 kỹ năng mặc định (cấp 1) từ `Npc.dat`, giữ `level = 1` để người chơi tự luyện.
   - `BattleService::apply_catch` truyền `self.data.npcs.get(&npc_id)` xuống session.
4. **`src/battle/service.rs` (`apply_drop`):**
   - Kiểm tra kết quả `s.add_homdo_item(item)`. Nếu `affected.is_empty()` (túi đồ đã đủ 25/25 ô): Hủy nạp item, không gửi frame `17 06`, gửi thông báo chữ đỏ `sys_msg_frame("Hành trang đã đầy, không thể nhận vật phẩm.")` về client.
5. **Cập nhật Bảng Thuật Ngữ Domain ([GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)):**
   - Đã bổ sung các ràng buộc bất biến (Invariants) cho Pet Capture Hydrate, Pet Trade Auto-Recall, Summon Item Sync, và Battle Drop Bag Guard.

---

## 4. BẰNG CHỨNG KIỂM THỬ (TEST VERIFICATION EVIDENCE)

- **Kiểm tra rào chắn kiến trúc:** `cargo test --test architecture_guardrails_test` $\rightarrow$ **8/8 passed** (đảm bảo không tạo file lẻ ngoài quy chuẩn `tests/<domain>/`, không chọc trực tiếp vào `ONLINE_SESSIONS`).
- **Kiểm tra nghiệp vụ kinh tế/kho:** `cargo test --test economy_inventory_test` $\rightarrow$ **59/59 passed** (bao gồm các test: `pet_trade_active_recall`, `summon_item_sync`, test từ chối khi đầy 4 pet `summon_item_when_roster_full_rejects_and_preserves_item`, test đồng bộ registry `summon_item_updates_session_registry`).
- **Kiểm tra cơ chế trận đấu:** `cargo test --test battle_test` $\rightarrow$ **186/186 passed** (bao gồm test nạp chỉ số gốc `test_add_caught_hydrates_npc_template_stats_at_level_1`, test chặn rơi đồ khi đầy túi `killed_npc_drop_blocked_when_bag_full_receives_sys_msg`, và test chuyển tiếp sức chứa biên 24 sang 25 ô `killed_npc_drop_at_boundary_24_to_25_accepted_then_subsequent_blocked`).
- **Kiểm tra toàn bộ hệ thống trước bàn giao:** `cargo test --all-targets --no-fail-fast` $\rightarrow$ **100% passed** trên mọi target mà không có cảnh báo hay lỗi biên dịch.

---

## 5. TÌNH TRẠNG WORKING TREE & HƯỚNG DẪN DÀNH CHO USER / NEXT AGENT

- **Working Tree:**
  Toàn bộ các thay đổi đang nằm sạch sẽ tại working tree trên branch `ss2` (chưa commit, tuân thủ tuyệt đối quy tắc user tự commit thủ công):
  - File sửa đổi: `src/server/trade_system.rs`, `src/server/handlers/trade_storage.rs`, `src/server/handlers/use_item/mod.rs`, `src/server/pet_box.rs`, `src/battle/service.rs`, `GLOSSARY.md`, `tests/economy_inventory_test.rs`, `tests/battle/catch_verification.rs`, `tests/battle/item_frame.rs`.
  - File test mới: `tests/economy_inventory/pet_trade_active_recall.rs`, `tests/economy_inventory/summon_item_sync.rs`.
  - Tài liệu kỹ thuật: Thư mục `.scratch/pet-and-drop-audit/` (chứa `research-*.md`, `spec-*.md`, `handoff.md`, và 4 files trong `issues/`).
- **Suggested Skills cho phiên làm việc tiếp theo:**
  - `code-review`: Nếu người dùng muốn rà soát diff của các file vừa sửa đổi trước khi tạo commit.
  - `pr`: Nếu người dùng muốn chuẩn bị nội dung PR tóm tắt các thay đổi của feature `pet-and-drop-audit`.
