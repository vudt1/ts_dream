# 16: [P1] Tối ưu Hot-Path Di chuyển (`WalkStep` / `quest_marks`) & Cô lập `run_sync_future`

**What to build:** Loại bỏ chi phí CPU và cấp phát heap không cần thiết trên hot-path di chuyển (`0x06 01` / `0x05 01`) bằng cách chỉ dựng snapshot sự kiện khi tọa độ thực sự nằm trong vùng `WalkStep` encounter và chia sẻ bất biến bảng `2,394` `quest_marks` thay vì clone từng bước đi, đồng thời tránh dựng snapshot hai lần với `try_trigger_random_encounter` và cô lập `run_sync_future` khỏi đường chạy production.

**Blocked by:** `11: [P0] Gỡ should_skip_event & Mở khóa Chuỗi Quest Đa giai đoạn / NPC Nhiều Quest`, `13: [P1] Bóc tách ground_server.mmg, Sửa GroundMmgLoader & Chuẩn hóa DiaHinh Trận đấu`

**Status:** ready-for-agent

- [ ] Trong luồng xử lý `WalkStep` của `NpcEventHandler` / `MovementHandler`, kiểm tra nhanh `scene.encounters` tồn tại và có vùng chứa `(px, py)` **trước** khi gọi `snapshot_state_with_data`; khi ngoài mọi vùng encounter (chiếm `3,877 / 3,897` bản đồ), trả về ngay mà không dựng snapshot.
- [ ] Tái cấu trúc `PlayerEventState` / `EveStateBuilder` để dùng tham chiếu bất biến hoặc `Arc` tới `GameData.quest_marks`, loại bỏ hoàn toàn vòng lặp dựng `HashMap` `2,394` phần tử ở mỗi bước di chuyển hoặc mỗi lần click NPC/Door.
- [ ] Phối hợp gọn giữa `try_trigger_random_encounter` và `WalkStep` trong một gói tin di chuyển để không dựng lại snapshot thừa khi lượt roll ngộ địch không tạo ra trận chiến.
- [ ] Giữ toàn bộ luồng production đi qua các hàm async `*_with_pool(...).await`; thu hẹp phạm vi hiển thị hoặc cô lập `run_sync_future` cùng các sync wrapper chỉ phục vụ harness kiểm thử, không có caller production nào gọi hàm tạo OS thread + runtime lồng nhau.
- [ ] Có test kiểm chứng di chuyển `0x06 01` ngoài vùng `encounters` hoàn tất sạch không kích hoạt side-effect Eve, kiểm chứng các điều kiện `cc=2 p_style=3` (`quest_marks`) vẫn hoạt động chính xác với tham chiếu bất biến, và kiểm tra ranh giới không còn caller production của `run_sync_future`.
