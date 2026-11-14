# 02: Tách bạch `homdo (0x17)` vs Nhật ký Nhiệm vụ `0x18` (`Mark.dat` 200 ô & bitmask `1..=2400`) + Chặn `C→S 0x18`

**What to build:** Tách bạch hoàn toàn hai không gian theo đúng hành vi client `aLogin.exe`: toàn bộ vật phẩm `Item.dat` (bao gồm đạo cụ nhiệm vụ) đi qua túi chính `homdo` (`0x17`, 25 ô), còn kênh `0x18` chỉ quản lý Nhật ký Nhiệm vụ `Mark.dat` (`1..=200` slot) và mảng bit `quest_dont` (`bit_id 1..=2400` trên `0x18 05`, mảng 300 byte trên `0x18 07`); đồng thời từ chối hoàn toàn gói `C→S 0x18 01/02` cho mọi kết nối kể cả GM.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Gỡ bỏ hoàn toàn khái niệm `quest_items (0x18)` khỏi trạng thái phiên, các hàm đồng bộ `0x18`, và luồng gửi gói đăng nhập `Logined1` (mọi vật phẩm `Item.dat` chỉ thao tác qua `homdo 0x17`).
- [x] Cấp phát slot Nhật ký Nhiệm vụ (`quest_tasks`) chặt chẽ trong `1..=200`, xóa bỏ mọi vị trí gán `slot = 0` không hợp lệ (bao gồm các sự kiện map đặc biệt và đấu trường); khi đủ 200 ô mà nhận thêm nhiệm vụ mới thì từ chối All-or-Nothing và phát `S→C 0x18 03`.
- [x] Sửa `quest_dont` hỗ trợ đầy đủ `bit_id` trong phạm vi `1..=2400` (theo trường `position` của `Data/Mark.dat`); gói đơn `S→C 0x18 05` gửi `bit_id` (`1..=2400`), gói bulk `S→C 0x18 07` khi đăng nhập đóng gói bitmask `[byte_idx (1..=300): u16 LE][bitmask: u8]`, và đồng bộ `S→C 0x18 08` khi đăng nhập nếu nhân vật có trạng thái diễn viên.
- [x] Gói `C→S 0x18 01/02` (từ form debug `TMissionForm`) bị từ chối hoàn toàn đối với mọi kết nối bao gồm cả GM, chỉ ghi log cảnh báo và không biến đổi trạng thái phiên hay phát frame phản hồi thay đổi quest.
- [x] Cập nhật toàn bộ bộ kiểm thử `0x18` (loại bỏ assertion cũ về `quest_items 0x18` và giới hạn `mark <= 300`), bổ sung negative test tràn 200 ô (`0x18 03`), test `bit_id > 300` (như `1673`) ra đúng byte/bit trong `0x18 07`, test bảo toàn các slot phụ cận và từ chối `C→S 0x18 01/02` qua `dispatcher::dispatch`.

## Comments

- Hoàn thành toàn bộ thay đổi ở `src/server/session.rs`, `src/server/handlers/quest_sync.rs`, `src/server/dispatcher.rs`, `src/server/auto_save.rs`, `src/db/modern/sqlite/session.rs`, `src/battle/construction.rs`, `src/server/handlers/event_npcs.rs`, `tests/quest_sync_18_test.rs`, `tests/npc_eve_e2e_test.rs`, và `tests/db_repository_init_test.rs`.
