# 08: Tái cấu trúc ánh xạ sủng vật trong pet_box (loại bỏ tuple lớn & lặp code)

**What to build:**  
Trong `src/server/pet_box.rs` (`add_caught`), tái cấu trúc để loại bỏ việc giải nén tuple 11 phần tử `(npc.hp, npc.sp, ...)`. Khởi tạo cấu trúc `Pet` hoặc gán thuộc tính trực tiếp từ `NpcData`. Rút gọn phép gán kỹ năng bằng cách duyệt mảng hoặc ánh xạ thay vì gán thủ công từng chỉ mục `skills[0]..skills[3]`.

**Blocked by:** None

**Status:** completed

- [x] Loại bỏ tuple 11 phần tử trong `pet_box::add_caught`.
- [x] Gọn hóa việc sao chép kỹ năng `npc.skills` sang `pet.skills`.
