# 12: Trả danh sách server khi client mở form chọn server (opcode 0x36)

**What to build:** Khi client yêu cầu danh sách server, server trả về danh sách hợp lệ với số lượng và trạng thái từng server, để form chọn server hiển thị đúng.

**Blocked by:**
- 11: Server chủ động chuyển người chơi sang cổng game khác

**Status:** needs-triage

> **Deferred theo Q5:** đợt sau. Xem ticket 11.

- [ ] Phản hồi có tổng độ dài payload chẵn (độ dài lẻ làm client báo lỗi phạm vi)
- [ ] Chỉ số server bắt đầu từ 1 và mức trạng thái nằm trong 0..3
- [ ] Client hiển thị đúng danh sách khi mở form chọn server (kiểm chứng thủ công)
