# 03: Migration SQLite `0002`, Transaction nguyên tử duy nhất & Khóa `(playerid, scene_id, eventid)`

**What to build:** Hợp nhất toàn bộ việc lưu trạng thái nhiệm vụ/Eve vào một SQLite transaction duy nhất không nuốt lỗi, tách biệt bộ đếm hoàn thành sự kiện theo `(playerid, scene_id, eventid)`, bổ sung lưu bền vững `savemap`, cờ bit nhiệm vụ và loại kho sủng vật thực, đảm bảo khởi động lại server không làm va chạm hay thất thoát dữ liệu người chơi.

**Blocked by:** `02: Tách bạch homdo (0x17) vs Nhật ký Nhiệm vụ 0x18 (Mark.dat 200 ô & bitmask 1..=2400) + Chặn C→S 0x18`

**Status:** ready-for-agent

- [ ] Tạo SQLx migration mới (`0002`) đổi khóa chính bảng `character_completed_events` thành `(playerid, scene_id, eventid)`, thêm các cột lưu `savemap` (`savemap_id`, `savemap_x`, `savemap_y`) trên `characters`, và gỡ bỏ truy vấn/bảng `character_quest_items` lỗi thời hoặc dead code `SqliteQuestRepository`.
- [ ] Nối `character_bit_flags` và `savemap` vào vòng đời nạp/lưu phiên; giữ nguyên phạm vi `scene_id` trong `Session.completed_eve_counts` và xóa đúng các dòng đã bị loại khỏi tập bộ đếm khi lưu.
- [ ] Bảo toàn nguyên vẹn `PetStorageType` (`Cart = 2`, `Hotel = 3`, `Warehouse = 4`) khi đọc/ghi sủng vật; sửa thao tác xóa nhân vật xóa sạch `character_quest_tasks`, `character_quest_dont`, `character_bit_flags` và `character_completed_events`.
- [ ] Gộp việc lưu trạng thái Eve/quest vào cùng 1 transaction SQLite chính trong `SqliteSessionRepository::save`; không nuốt `Err` (lỗi lưu DB phải rollback sạch cả transaction và trả lỗi để `auto_save` không đánh dấu `SaveLedger` sai lệch).
- [ ] Cập nhật `auto_save::fingerprint()` bao phủ toàn bộ trường bền vững (`ghost`, `god`, `bit_flags`, `savemap`, `completed_eve_counts`, `pet.quest`, v.v.) và bỏ băm các mảng tạm không lưu.
- [ ] Kiểm thử trên seam `SqliteSessionRepository::load/save` & `characters::delete` (với `common::test_db::setup_test_db()`) xác nhận round-trip dữ liệu, hai bản đồ cùng `eve_no` không ghi đè nhau, negative test ép lỗi SQL giữa chừng làm rollback toàn bộ transaction và không làm lệch `SaveLedger`, và xóa nhân vật không để lại bản ghi mồ côi.
