# 01: Khôi phục nạp `Data/eve.emg` & bật mặc định động cơ Eve

**What to build:** Khi server khởi động ở chế độ mặc định, người chơi tương tác trên toàn bộ các bản đồ (bao gồm `1,120` scene trước đây bị rơi do lỗi đọc `FightData`) đều nhận được hội thoại, điều kiện và dữ liệu trận chiến chính xác từ `Data/eve.emg` làm nguồn sự thật duy nhất, thay vì rơi về phản hồi giả cứng hoặc bị file `.ini` legacy ghi đè sai bước.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Bộ nạp nhị phân `Data/eve.emg` đọc mỗi `win_cond` đúng `13` byte, loại bỏ đọc `fight_limit` (`0` byte), sửa bounds check bản ghi `EveResult` thành `14` byte, và lan truyền lỗi đọc scene thay vì bỏ qua âm thầm để nạp đủ toàn bộ `3,901` mục từ (`3,897` scene có nội dung).
- [ ] Xây dựng chỉ mục tra cứu nhanh chỉ-đọc theo khóa `(scene_id, eve_no)` cùng các chỉ mục theo NPC, Door và Encounter cho mỗi scene.
- [ ] Động cơ Eve bật mặc định trong môi trường production (`EVE_EVENTS_ENABLED` mặc định `true`) và gỡ bỏ hoàn toàn cơ chế ghi đè `.ini` legacy cũng như fallback phản hồi giả cũ.
- [ ] Kiểm thử tích hợp ở seam `GameData::load` xác nhận nạp thành công đủ `3,901` mục từ / `3,897` scene có dữ liệu và số lượng trận chiến của các scene tiêu biểu (như `12001` có 9 trận, `10951`, `10954`) khớp với nhị phân gốc.
- [ ] Kiểm thử tích hợp ở seam `dispatcher::dispatch` xác nhận tương tác NPC trên scene trước đây bị rơi (như `10951`) phản hồi hội thoại Eve thật, và trường hợp lỗi định dạng nhị phân giả lập bị từ chối rõ ràng thay vì nuốt lỗi.
