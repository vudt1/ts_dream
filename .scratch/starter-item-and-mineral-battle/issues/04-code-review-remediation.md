# 04: Chuẩn hóa, khử trùng lặp và khắc phục các điểm review cho Cuốc & Quái Khoáng

**What to build:** Hoàn thiện toàn bộ các điểm phát hiện từ vòng code review cho phân hệ đồ mặc định, trang bị Cuốc và chiến đấu Quái Khoáng: bảo đảm khi thành viên tổ đội mặc/tháo trang bị (như Vũ khí Cuốc cho Nhân vật hoặc Sủng vật) trước khi vào trận thì trạng thái trang bị luôn đồng bộ qua `online_sessions` để nạp đúng `weapon_id` khi dựng bàn cờ; phản hồi thông báo hệ thống qua wire thay vì bỏ qua im lặng khi thao tác mặc/tháo đồ bị từ chối; chuẩn hóa hằng số miền và khử các đoạn mã trùng lặp ở tầng túi đồ, dựng bàn cờ và xử lý lượt chiến đấu mà không làm thay đổi hành vi chiến đấu hiện có; đồng thời gỡ bỏ mâu thuẫn trong tài liệu bàn giao `HANDOFF.md`.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Mọi thao tác mặc, tháo và hoán đổi trang bị của Nhân vật và Sủng vật đều đồng bộ phiên hiện tại lên `online_sessions` (`registry_update_if_online`), bảo đảm thành viên tổ đội vừa trang bị Vũ khí Cuốc trước trận luôn được nạp đúng `weapon_id` (và đánh trúng Quái Khoáng) khi đội trưởng khởi tạo trận đấu.
- [x] Các thao tác mặc/tháo trang bị của Nhân vật và Sủng vật khi bị từ chối (như đang trong trận đấu `battle_id > 0`, chưa đủ cấp độ, vật phẩm không thể trang bị, hoặc túi đồ đầy) đều gửi thông báo hệ thống chuẩn (`sys_msg_frame`) về client thay vì bỏ qua im lặng, không làm thay đổi túi đồ hoặc ô phụ cận.
- [x] Thay thế các magic number trong luồng dựng bàn cờ và kiểm tra Quái Khoáng (vị trí ô vũ khí mặc định, mã loại mục tiêu NPC địch trong trận) bằng các hằng số định danh miền tường minh.
- [x] Khử trùng lặp logic tại 3 điểm mà không làm sai lệch hành vi hiện hành: (1) fallback tính `equip_pos` khi dựng vật phẩm từ mẫu và khi đọc vị trí trang bị hiệu lực, (2) tra cứu ô vũ khí Sủng vật và gán `weapon_id` giữa nạp Sủng vật đội trưởng và Sủng vật thành viên, (3) guard miễn nhiễm Quái Khoáng và kiểm tra ghi nhận trúng đòn NPC giữa các nhánh kỹ năng gây hại và vòng lặp lượt đánh.
- [x] Tài liệu `HANDOFF.md` được cập nhật nhất quán: gỡ bỏ nội dung mâu thuẫn ở mục hướng dẫn cho rằng Ticket 03 vẫn đang mở, đồng thời ghi nhận trạng thái hoàn tất của Ticket 04 cùng kết quả kiểm thử mới nhất.
- [x] Có kiểm thử tự động bổ sung xác minh đồng bộ `online_sessions` cho thành viên tổ đội mặc Cuốc trước trận và xác minh khung thông báo từ chối qua wire khi mặc/tháo đồ không hợp lệ; toàn bộ `cargo test --all-targets --no-fail-fast` đều pass.
