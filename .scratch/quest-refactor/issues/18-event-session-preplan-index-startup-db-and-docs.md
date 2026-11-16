# 18: [P2] Pre-plan `EventSession`, Thống nhất Chỉ mục `eve_no`, Startup DB Check & Cập nhật Docs

**What to build:** Tinh gọn chi phí thực thi `EventSession` bằng cách lập kế hoạch hiệu ứng một lần thay vì re-plan lặp lại qua từng câu thoại, thống nhất kiểu chỉ mục `eve_no` không cấp phát vector tạm, bổ sung kiểm tra schema SQLite lúc khởi động server (phát hiện sớm trường hợp `DB/ts_dream.db` rỗng `0` bảng trong khi vẫn giữ migrate thủ công theo ADR 0004), và cập nhật hoàn chỉnh tài liệu bàn giao `HANDOFF_01_10.md`.

**Blocked by:** `12: [P0] Giải mã EveSceneInfo.dest_map_id & Dịch chuyển Warp rt=2 Chuẩn PC`, `14: [P1] Sửa Ngữ nghĩa Engine: when_happen Cửa, GroupData.use_mode == 2 & EveResolver chain[1..]`, `15: [P1] Sửa mark_dont_on_step1 (quest_dont) & Regression Test cc=7 param=2`, `16: [P1] Tối ưu Hot-Path Di chuyển (WalkStep / quest_marks) & Cô lập run_sync_future`

**Status:** ready-for-agent

- [ ] Lưu sẵn danh sách `Vec<DomainEffect>` đã được `plan_results` biên dịch một lần khi mở hoặc cập nhật `EventSession` (kể cả khi nối chuỗi `AutoChain` hoặc resume hậu battle), để `execute_event_step_core` đọc tuần tự theo `current_index` mà không clone slice `EveResult` và gọi lại `plan_results` ở mỗi bước.
- [ ] Thống nhất kiểu chỉ mục `eve_no` (hoặc truyền slice trực tiếp) giữa `SceneEveData` và `NpcEventHandler` để loại bỏ cấp phát `Vec<u8>` tạm và ép kiểu qua lại `u8 <-> u16` ở mỗi lần tương tác NPC/Door/Encounter.
- [ ] Bổ sung kiểm tra schema sanity-check lúc khởi động server (xác nhận tồn tại các bảng/cột cốt lõi từ `0001_init.sql` và `0002_eve_quest_persistence.sql` như `character_completed_events` với PK `(playerid, scene_id, eventid)` và `savemap_id`); nếu thiếu (như `DB/ts_dream.db` rỗng `0` bảng hiện tại), báo lỗi rõ ràng kèm hướng dẫn áp dụng migration thủ công theo ADR 0004.
- [ ] Cập nhật `HANDOFF_01_10.md` đồng bộ với trạng thái codebase thực tế (tiêu đề `01..10`, `Warps.txt`, `DomainEffect::Unsupported`, `eve_runner.rs`, `EveCondition.to_result`, ghi nhận `rc=5` đã hỗ trợ, `0x46 BoatRace`/`0x41 Rank` không nằm trong `unimplemented`, cùng đúng số liệu `15` chuỗi AND / `223` scene event `cc=12 param != 0` / `rt=9 p_style ∈ {1,2}`).
- [ ] Chạy kiểm chứng toàn cục `cargo clippy --all-targets` và `cargo test --all-targets --no-fail-fast` xanh 100% ở trạng thái cuối cùng.
