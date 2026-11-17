# 07: Kiểm tra nghiêm ngặt giới hạn 25 ô hành trang khi rớt đồ trận đấu

**What to build:**  
Trong `src/battle/service.rs` (`apply_drop`), trước khi thực hiện cộng đồ bằng `add_homdo_item`, đếm nghiêm ngặt số ô occupied (`(1..=25).contains(&slot) && it.id > 0`). Nếu số ô $\ge 25$, chặn ngay lập tức việc nhận đồ rơi, phát thông báo chữ đỏ: `sys_msg_frame("Hành trang đã đầy (25/25 ô), không thể nhận vật phẩm rơi từ trận đấu.")` và thoát hàm. Điều này ngăn chặn việc tự động cộng dồn vào slot đã có khi người chơi đã sử dụng hết 25 ô theo đúng quy định sức chứa hòm đồ.

**Blocked by:** None

**Status:** completed

- [x] Trong `apply_drop` của `src/battle/service.rs`, tính số ô đang sử dụng trong khoảng 1..=25.
- [x] Nếu `count >= 25`: phát `sys_msg_frame` màu đỏ cảnh báo túi đầy và return sớm không thêm item.
