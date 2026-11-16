# 17: [P1] Chuẩn hóa `Two-Phase State Lock` & Persist Tổ đội Ngoài `EventRunner`

**What to build:** Đảm bảo mọi thao tác đa người chơi nằm ngoài `EventRunner` — bao gồm NPC sự kiện đặc biệt (`15312`, `15313`, `61210`), trả lời đố Bắc Đẩu (`0x10`), đồng bộ vị trí thành viên khi đội trưởng di chuyển, và cập nhật bước nhiệm vụ `40 NPC` (`quest_tasks[218]`) — đều tuân thủ chặt chẽ `Two-Phase State Lock`, cập nhật `registry_update_if_online` và lưu bền vững trạng thái của toàn bộ thành viên tổ đội xuống SQLite.

**Blocked by:** `13: [P1] Bóc tách ground_server.mmg, Sửa GroundMmgLoader & Chuẩn hóa DiaHinh Trận đấu`

**Status:** ready-for-agent

- [ ] Chuẩn hóa các nhánh tổ đội của NPC sự kiện đặc biệt (`15312`, `15313`, `61210`) và xử lý kết thúc đố Bắc Đẩu (`0x10`): thu thập danh sách ID thành viên đã sắp xếp tăng dần, giữ `lock_player_operations` trước khi đọc/ghi `online_sessions`, cập nhật `registry_update_if_online`, và truyền toàn bộ session bị thay đổi của leader lẫn thành viên vào `persist_sessions_transaction`.
- [ ] Rà soát và áp dụng chuẩn lock ordering / đồng bộ registry nhất quán cho bước cập nhật tọa độ thành viên trong `MovementHandler` và bước cập nhật `quest_tasks[218]` khi tiến tầng `40 NPC` trong `BattleConstruction` / `BattleService`.
- [ ] Kiểm tra tuân thủ các quy chuẩn trong `architecture_guardrails_test` (không gọi `lock_online_sessions().insert(...)` trái quy chuẩn và không gây deadlock khi khóa hai chiều).
- [ ] Có multi-session party E2E test qua `dispatcher::dispatch` + `setup_test_db()` cho sự kiện `15312`/`15313`/`61210` và đố Bắc Đẩu `0x10`: xác nhận cả leader lẫn thành viên online được cập nhật đúng `map_id`, tọa độ, `battle_id`, `quest_tasks`, đồng bộ trong `online_sessions` và bảo toàn sau khi reload từ SQLite.
