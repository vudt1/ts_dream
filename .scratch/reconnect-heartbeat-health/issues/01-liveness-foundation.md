# 01: Nền tảng liveness: timeout cấu hình được và theo dõi thời điểm hoạt động cuối

**What to build:** Server nhận biết lần cuối mỗi kết nối có dữ liệu đến, và thời gian chờ không hoạt động được cấu hình qua biến môi trường (mặc định 300 giây). Frame ACK `00` từ client (`F444010000`) được nhận diện là tín hiệu còn sống và không gây phản hồi nào.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Idle timeout mặc định 300 giây, ghi đè được qua `TS_IDLE_TIMEOUT_SECS`
- [x] Mọi byte đọc được từ socket làm mới thời điểm hoạt động cuối của kết nối
- [x] Frame `F444010000` không sinh frame phản hồi nào; chỉ ghi log mức trace
- [x] Frame opcode `00` từ client không còn rơi vào nhánh không xử lý
- [x] Test trong `tests/` cho cấu hình mặc định, ghi đè bằng env, và ACK không phản hồi
