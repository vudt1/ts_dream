# 04: Thủy hóa `PlayerEventSnapshot`, Fail-Closed Evaluator & Lọc `0x14 Surface` (`MenuChoice`)

**What to build:** Khi người chơi nói chuyện hoặc chọn một phương án trên hộp thoại `0x14 Surface`, hệ thống đánh giá đúng toàn bộ trạng thái thật của nhân vật (túi đồ `homdo`, võ tướng đi theo/trong kho, bộ đếm theo `scene_id`, cờ `Mark.dat` và `bit_flags`), fail-closed an toàn đối với điều kiện chưa giải mã, và đảm bảo lựa chọn menu `0x14 09` chỉ khớp chuỗi có điều kiện `DialogChoice` tương ứng.

**Blocked by:** `01: Khôi phục nạp Data/eve.emg & bật mặc định động cơ Eve`, `03: Migration SQLite 0002, Transaction nguyên tử duy nhất & Khóa (playerid, scene_id, eventid)`

**Status:** ready-for-agent

- [ ] Thủy hóa đầy đủ `PlayerEventSnapshot`: `homdo` (`0x17`), `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `role_count_values`, `completed_eve_counts` theo `(scene_id, eve_no)`, `quest_tasks` (`1..=200`), ánh xạ `Mark.dat` (`quest_id <-> bit_id`) cho kiểm tra `quest_dont`, và `character_bit_flags`.
- [ ] Loại bỏ lỗi fallthrough `actual = 0 -> 0 == 0 -> true`: mọi `condition_class` hoặc tham số chưa hỗ trợ (`UNKNOWN`) bắt buộc trả về `false` kèm log cảnh báo telemetry; đồng nhất thứ tự đối số của `compare_step` trên mọi lớp điều kiện.
- [ ] Xử lý `C→S 0x14 09` (`MenuChoice { surface_id, choice_code }`) bắt buộc truyền bộ lọc `ChainFilter::RequireDialogChoice(surface_id, choice_code)` vào bộ giải quyết chuỗi, ngăn các chuỗi thường không có điều kiện lựa chọn đè lên kết quả chọn của người chơi.
- [ ] Kiểm thử ở seam `dispatcher::dispatch` trên dữ liệu thật `Data/eve.emg` + `Data/Mark.dat` xác nhận phân nhánh menu đúng ở chuỗi Giản Ung (`12136`) và Tống Nhân (`12001`), kiểm tra điều kiện dắt tướng `FollowPet`, kiểm tra `ConditionClass=2 pStyle=3` đọc đúng cờ `quest_dont`, và negative tests chứng minh mã điều kiện `UNKNOWN` hoặc `choice_code` không hợp lệ/bị khóa điều kiện luôn bị từ chối.
