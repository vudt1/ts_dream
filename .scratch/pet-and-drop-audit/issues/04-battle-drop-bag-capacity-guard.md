# 04: Kiểm soát sức chứa túi đồ khi rơi vật phẩm trong trận chiến (Battle Drop Bag Capacity Guard)

**What to build:**  
Khi một NPC bị tiêu diệt và rơi vật phẩm trong trận đấu (Battle Drop), server kiểm tra số lượng ô đồ đang sử dụng trong hòm đồ (`homdo`) của người nhận. Nếu túi đồ đã đầy (đủ 25 ô), server hủy việc trao vật phẩm, không gửi gói tin thêm item `17 06` và gửi thông báo chữ đỏ *"Hành trang đã đầy, không thể nhận vật phẩm"* về client. Đảm bảo dữ liệu túi đồ không bị tràn ô và không gây desync số lượng item giữa client và server.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Trong `BattleService::apply_drop` ([src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs)), bổ sung bước kiểm tra sức chứa hòm đồ của người nhận (`owner`):
  Đếm số lượng ô occupied trong `session.homdo` (ô có `1 <= slot <= 25` và `id > 0`).
- [ ] Nếu số ô occupied $\ge 25$:
  - Hủy xử lý `add_homdo_item`.
  - Không gửi frame nạp đồ `17 06`.
  - Gửi thông điệp cảnh báo chữ đỏ qua `sys_msg_frame("Hành trang đã đầy, không thể nhận vật phẩm.")` tới client của `owner`.
- [ ] Nếu số ô occupied $< 25$: Tiếp tục luồng cấp đồ và gửi frame `17 06` bình thường.
- [ ] Có unit test kiểm thử:
  1. Người chơi có 24/25 ô $\rightarrow$ nhặt đồ thành công, túi tăng lên 25 ô, nhận frame `17 06`.
  2. Người chơi có 25/25 ô $\rightarrow$ không được nạp thêm đồ, túi giữ nguyên 25 ô, nhận thông báo lỗi hệ thống chữ đỏ.
