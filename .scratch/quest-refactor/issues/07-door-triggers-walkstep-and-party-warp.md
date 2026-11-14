# 07: Trigger Cửa (`ClickDoor`/`MeetDoor`), `WalkStep` & Door Warp Tổ đội (`Two-Phase Lock` + Write-Through)

**What to build:** Kích hoạt các sự kiện Eve gắn trên cửa và bước di chuyển (`ClickDoor`, `MeetDoor`, `WalkStep`), đồng thời đảm bảo khi Đội trưởng kích hoạt dịch chuyển qua cửa (`Door Warp`), toàn bộ thành viên tổ đội cùng được khóa đồng bộ an toàn, chuyển bản đồ đồng loạt và ghi vị trí mới ngay xuống SQLite.

**Blocked by:** `05: Kiến trúc Pure Planner + EventRunner (Pre-Flight All-or-Nothing, AutoChain, Skill & FinishChain Write-Through)`

**Status:** resolved

- [x] Đấu nối `NpcTrigger::ClickDoor` qua `0x14 04`, `NpcTrigger::MeetDoor` qua trình xử lý di chuyển chạm cửa, và `NpcTrigger::WalkStep` vào `Pure Planner` -> `EventRunner`.
- [x] Xử lý `Warp` cho tổ đội tuân thủ tuyệt đối `Two-Phase State Lock`: gọi `lock_player_operations([min..max])` **trước khi** đọc `registry_get` của các thành viên, cập nhật vị trí mới, gọi `registry_update_if_online`, phát `0x0C` cùng đồng bộ bản đồ cho cả Đội trưởng và từng thành viên tham gia.
- [x] Kích hoạt write-through SQLite nguyên tử ngay tại checkpoint `Door Warp` cho cả Đội trưởng lẫn tất cả thành viên tổ đội được dịch chuyển.
- [x] Kiểm thử E2E qua `dispatcher::dispatch` + `setup_test_db()` xác nhận `ClickDoor`, `MeetDoor` có tổ đội chuyển sang map đích và lưu bền vững `(mapid, x, y)` trong SQLite cho cả Leader lẫn Member; kèm negative test điều kiện cửa chưa thỏa mãn hoặc đích warp không hợp lệ bị từ chối sạch sẽ không làm sai lệch vị trí hay DB của thành viên tổ đội.
