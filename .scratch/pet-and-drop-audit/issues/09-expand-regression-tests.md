# 09: Bổ sung kiểm thử phủ định cho túi sủng 4 con và biên 24 sang 25 ô hành trang

**What to build:**  
1. Trong `tests/economy_inventory/summon_item_sync.rs`: Thêm test case người chơi đã có 4 sủng vật trong túi mang theo (`stt` 1..4). Dùng thẻ triệu hồi: xác nhận thẻ không bị trừ trong hành trang, không có sủng thứ 5 nào được tạo và nhận thông báo lỗi.
2. Trong `tests/battle/item_frame.rs`: Thêm test case người chơi có 24 ô hành trang đã sử dụng, khi kết liễu NPC rớt đồ $\rightarrow$ vật phẩm rơi lấp đầy ô thứ 25 thành công; khi đánh tiếp con thứ 2 rơi đồ thì bị chặn và nhận thông báo đỏ.

**Blocked by:** Tickets 05, 07

**Status:** completed

- [x] Viết test `test_summon_pet_roster_full_rejected` trong `tests/economy_inventory/summon_item_sync.rs`.
- [x] Viết test `test_battle_drop_at_capacity_boundary_24_to_25` trong `tests/battle/item_frame.rs`.
