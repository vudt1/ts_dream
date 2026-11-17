# ĐẶC TẢ KỸ THUẬT (SPEC): KHẮC PHỤC CÁC VẤN ĐỀ PHÁT HIỆN QUA CODE REVIEW

- **Feature slug:** `pet-and-drop-audit`
- **Tài liệu tham chiếu:** [research-pet-and-drop-audit.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/research-pet-and-drop-audit.md), [spec-pet-and-drop-fixes.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/spec-pet-and-drop-fixes.md), [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
- **Tình trạng:** Tổng hợp và chuẩn hóa sau phiên Code Review độc lập.

---

## Problem Statement

Sau khi hoàn thành việc triển khai 4 tickets cho hệ thống sủng vật và battle drop, phiên rà soát mã nguồn độc lập (`code-review`) đã phát hiện các khiếm khuyết kỹ thuật sau:
1. Trong luồng dùng thẻ triệu hồi sủng vật (`use_item/mod.rs`), sau khi thêm pet và tiêu hao vật phẩm, hệ thống chưa gọi `registry_update_if_online`, dẫn đến snapshot session trong bộ nhớ dùng chung bị cũ (stale) so với các luồng giao dịch hoặc lưu tự động chạy ngầm.
2. Tại `use_item/mod.rs`, logic tìm ô sủng còn dùng fallback `.unwrap_or(1)` thay vì tái sử dụng triệt để hàm `pet_box::next_active_slot`.
3. Trong `trade_storage.rs`, chuỗi hex `"F44402001302"` bị hardcode lặp lại ở nhiều nơi thay vì dùng hằng số định danh domain.
4. Trong `src/battle/service.rs` (`apply_drop`), việc kiểm tra sức chứa hòm đồ dựa vào kết quả `affected.is_empty()` của `add_homdo_item`. Khi túi đồ đã đủ 25 ô nhưng nhặt vật phẩm trùng loại có thể cộng dồn (stack < 50), vật phẩm vẫn bị gộp vào túi thay vì kích hoạt quy tắc chặn rơi đồ khi túi đạt 25/25 ô như đặc tả yêu cầu.
5. Thiếu các kiểm thử biên và kiểm thử phủ định (Negative Tests): kịch bản dùng thẻ sủng khi túi đã đủ 4 sủng vật, kịch bản nhặt đồ thành công khi túi có 24/25 ô.
6. Code smell `Data Clumps` và `Duplicated Code` trong `src/server/pet_box.rs` khi giải nén tuple 11 phần tử và lặp lại logic ánh xạ kỹ năng.

*(Lưu ý: Các mục scope creep gồm gán `fai: 60`, tra cứu `pet_name` từ template NPC và `pub` visibility của `removed_pet_frame` đã được xác nhận là chuẩn xác và được giữ nguyên).*

---

## Solution

1. Bổ sung `registry_update_if_online(&ctx.conn.session)` và đồng bộ hóa `pet_box::next_active_slot` trong `use_item/mod.rs`, đảm bảo từ chối dùng thẻ khi danh sách sủng vật đã đủ 4 con (không trừ vật phẩm, không ghi đè slot).
2. Định nghĩa hằng số giao thức `PET_RECALL_FRAME` để loại bỏ chuỗi literal hardcode trong `trade_storage.rs`.
3. Tái cấu trúc hàm `apply_drop` trong `src/battle/service.rs`: Kiểm tra nghiêm ngặt số ô occupied (`1 <= slot <= 25 && id > 0`). Nếu số ô $\ge 25$, lập tức chặn nhận vật phẩm và phát thông báo chữ đỏ *"Hành trang đã đầy, không thể nhận vật phẩm."*.
4. Tái cấu trúc `add_caught` trong `pet_box.rs` để loại bỏ tuple 11 phần tử cồng kềnh và vòng lặp map kỹ năng gọn gàng.
5. Bổ sung các bài kiểm thử tự động cho ca túi sủng đầy 4 con và ca túi đồ 24/25 ô trong `economy_inventory_test` và `battle_test`.

---

## User Stories

1. As a player using a summon pet card, I want my session registry to be updated immediately, so that background auto-saves and trade sessions see my new pet without delay.
2. As a player with a full active pet roster (4/4 pets), I want using a summon pet card to be rejected with an alert and without consuming my card, so that I don't lose valuable summon items.
3. As a player whose inventory already occupies all 25 slots, I want dropped items in battle to be consistently rejected with a red system alert even if the item matches an existing item in my bag, so that the 25-slot capacity rule is strictly enforced.
4. As a player whose inventory occupies 24 slots, I want dropped items in battle to be received successfully into the 25th slot, so that I can use my full bag capacity.
5. As a developer maintaining the codebase, I want pet template hydration to use clean structured mapping rather than massive tuples, so that future NPC schema additions remain maintainable.
6. As a developer, I want wire protocol literals like pet recall frames to be centralized constants, so that protocol edits do not leave orphaned hex strings.
7. As a quality engineer, I want automated regression tests covering full-roster summon rejection and 24-to-25 bag drop transitions, so that future refactors cannot break capacity boundaries.

---

## Implementation Decisions

### 1. Đồng bộ Registry và Roster Slot Guard trong `use_item/mod.rs`
- Sử dụng trực tiếp `crate::server::pet_box::next_active_slot(&ctx.conn.session.pets)`.
- Nếu trả về `None`: Gửi thông báo chữ đỏ `ctx.red("Pet box full")` và thoát ngay, không tiêu hao vật phẩm.
- Khi thêm sủng vật thành công: Gọi `crate::server::session::registry_update_if_online(&ctx.conn.session)` ngay sau khi `upsert_pet`.

### 2. Định nghĩa Hằng số Giao thức Recall Frame
- Định nghĩa hằng số công khai `pub const PET_RECALL_FRAME: &str = "F44402001302";` tại `src/server/handlers/pet_actions.rs` (hoặc `src/protocol/mod.rs`).
- Tái sử dụng hằng số này trong `trade_storage.rs` và các test suites liên quan.

### 3. Kiểm soát Nghiêm ngặt 25 Ô Hòm đồ trong `BattleService::apply_drop`
- Trước khi thực hiện nạp đồ, kiểm tra:
  ```rust
  let is_bag_full = |bag: &[crate::server::session::InventoryItem]| {
      bag.iter().filter(|i| (1..=25).contains(&i.slot) && i.id > 0).count() >= 25
  };
  ```
- Nếu `is_bag_full == true`:
  - Phát thông báo chữ đỏ: `sys_msg_frame("Hành trang đã đầy, không thể nhận vật phẩm.")`.
  - Không gọi `add_homdo_item`, không gửi frame `17 06` và kết thúc hàm ngay lập tức.

### 4. Dọn dẹp Code Smell trong `src/server/pet_box.rs`
- Thay thế việc giải nén tuple 11 phần tử bằng việc gán trực tiếp hoặc dùng cấu trúc rõ ràng.
- Gọn hóa phần khởi tạo 4 kỹ năng bằng mảng hoặc vòng lặp `(0..4)`.

---

## Testing Decisions

1. **Kiểm thử biên và ca phủ định trong `tests/economy_inventory/summon_item_sync.rs`**:
   - Thêm test case: Người chơi đã có đủ 4 sủng vật trong túi mang theo (`stt 1..=4`).
   - Sử dụng thẻ sủng: Khẳng định số lượng thẻ trong `homdo` giữ nguyên, không có sủng mới được thêm vào, và nhận được thông báo lỗi hệ thống.
   - Kiểm tra ô kế cận trong `homdo` không bị ảnh hưởng khi trừ item.
2. **Kiểm thử chuyển tiếp sức chứa 24 lên 25 ô trong `tests/battle/item_frame.rs`**:
   - Thêm test case: Người chơi có 24 ô đồ, kết liễu NPC rớt đồ $\rightarrow$ nhận thành công frame `17 06`, hòm đồ tăng lên đúng 25 ô.

---

## Out of Scope

- Không hoàn tác hoặc thay đổi các bổ sung: `fai = 60`, tra cứu `name` từ template NPC trong `use_item`, hoặc tính công khai `pub` của `removed_pet_frame`.
- Không thay đổi công thức tính tỷ lệ rơi đồ cơ bản của trận đấu (vẫn giữ nguyên bảng xác suất ~7.6%).

---

## Further Notes

- Mọi thay đổi tiếp tục tuân thủ quy tắc giữ mã nguồn tại working tree và chạy toàn bộ kiểm thử `cargo test --all-targets --no-fail-fast` trước khi hoàn tất.
