# 10: Dọn dẹp các gap mức thấp: persist field phụ, cột không đọc, map không giải phóng, dashboard online

**What to build:** Các lỗi nhỏ không ảnh hưởng trực tiếp đến reconnect nhưng gây sai dữ liệu hoặc rò bộ nhớ: field phụ không được persist, cột `fai` ghi mà không đọc, map lưu lần lưu không bao giờ được xoá, và dashboard `/api/online` luôn trả rỗng.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Các field phụ của phiên được quyết định: persist hoặc ghi rõ là tạm thời (tham chiếu G12 trong plan)
- [x] Cột `fai` được đọc lại đúng, hoặc bỏ hẳn việc ghi nếu không dùng (G13)
- [x] Map lưu lần lưu được xoá khi người chơi thoát, không phình vô hạn (G14)
- [x] `/api/online` phản ánh danh sách người chơi online thật (G15)
- [x] Mỗi mục có test hoặc kiểm chứng thủ công được ghi trong ticket
