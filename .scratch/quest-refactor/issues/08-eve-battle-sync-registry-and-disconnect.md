# 08: Chuẩn hóa `EveFightData`, Đồng bộ `BattleService` (`Lock` + `Registry`) & Hậu chiến khi Disconnect

**What to build:** Đưa toàn bộ trận chiến nhiệm vụ Eve hoạt động trọn vẹn theo `EveFightData` (`TeamDef`/`right_enemies`, `DiaHinh`), đảm bảo kết thúc trận đấu luôn cập nhật registry an toàn dưới `Two-Phase State Lock` (không bị gói tin kế tiếp ghi đè mất thưởng), truyền đủ broadcast/tin nhắn trực tiếp, tách biệt phạm vi tổ đội (chỉ Leader tiến quest Eve chuẩn), và giải quyết đầy đủ hậu sự kiện + lưu DB ngay cả khi người chơi mất kết nối giữa trận.

**Blocked by:** `06: Bổ sung Action Executors (NpcTeam, RewardPet, EXP Level-Up, Gold/Point Cap, MoveEntity, Movie, NpcAction, savemap)`

**Status:** ready-for-agent

- [ ] Khởi tạo trận chiến Eve nạp đầy đủ `EveFightData` (`left` + `right_enemies`, `fight_type`, terrain từ `SceneInfo.background_no` hoặc fallback `112` — đã xác nhận `aLogin.exe` không đọc `.ini` hay `[TEAMDEF] Diahinh`, chỉ tra `TFight.BKMusic` và đọc `TWinCondition.wc_type ∈ {3,4,8}` cho UI `[alt+W]`, còn logic trận đấu là 100% server-only); đảm bảo giao diện hội thoại đang mở được đóng/mở khóa sạch trước khi chuyển vào trận.
- [ ] Trong `BattleService::battle_ended`: thu thập ID người tham gia đã sắp xếp và giữ `lock_player_operations` trước khi đụng session; thực thi nhánh thắng/thua/chạy (`resume_eve_after_battle` / `battle_quest_win` / `quest_lose_frames`), gọi `registry_update_if_online` cho mọi người chơi còn online, và làm mới session handle trong `register_sender`.
- [ ] Bảo toàn và phát đầy đủ `direct_messages` lẫn `map_broadcast` từ `resume_eve_after_battle`, đồng thời kích hoạt write-through SQLite nguyên tử ngay tại checkpoint `Battle End` (cả thắng, thua và chạy).
- [ ] Xử lý ngắt kết nối giữa trận Eve: khi trận đấu kết thúc mà người chơi đã offline, `battle_ended` vẫn chạy đầy đủ `resume_eve_after_battle` (theo kết quả thắng/thua/chạy) và ghi kết quả trực tiếp xuống SQLite cho tài khoản offline đó.
- [ ] Đảm bảo nhiệm vụ Eve chuẩn chỉ Đội trưởng thăng tiến trạng thái và lưu DB (các sự kiện map đặc biệt có tổ đội chỉ đồng bộ `quest_tasks` với slot hợp lệ `1..=200`).
- [ ] Kiểm thử E2E qua `BattleService::battle_ended` + `dispatcher::dispatch` + `setup_test_db()` cho chuỗi `12001` Tống Dũng (vào trận -> thắng -> nhận võ tướng `14106`, `NpcTeam`, `Quest 10028` & viết DB), test gửi gói tin di chuyển/nói chuyện ngay sau trận không ghi đè mất thưởng, negative test thua/chạy trận không kích hoạt `[OnWin]`, test giữ nguyên tiến độ `Member` ở quest Eve chuẩn, test disconnect giữa trận vẫn cập nhật hậu chiến xuống DB cho lần đăng nhập sau, và chạy xác nhận `architecture_guardrails_test`.
