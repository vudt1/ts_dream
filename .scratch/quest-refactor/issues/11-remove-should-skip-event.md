# 11: [P0] Gỡ `should_skip_event` & Mở khóa Chuỗi Quest Đa giai đoạn / NPC Nhiều Quest

**What to build:** Mở khóa toàn bộ chuỗi nhiệm vụ nhiều bước cùng một `eve_no` không gắn điều kiện `condition_class == 12` (hơn `1,500` quest) và các NPC sở hữu nhiều nhiệm vụ độc lập (`169` NPC), để người chơi quay lại nói chuyện, nộp vật phẩm, nhận bước kế tiếp và kích hoạt `AutoChain` bình thường (kể cả khi `battle_result == 0` hoặc chọn menu `0x14 09`) mà vẫn được bảo vệ bởi 4 lớp chống lặp vô hạn.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Gỡ bỏ hoàn toàn Quy tắc 1 (chặn khi `completed_count > 0` mà thiếu `cc=12`) và Quy tắc 2 (chặn khi tồn tại sự kiện đứng sau trong mảng đã hoàn thành) khỏi `should_skip_event` tại cả đường kích hoạt trực tiếp (`WalkStep`, `ClickNpc`, `ClickDoor`, `MeetDoor`, kể cả khi có `chain_filter`) lẫn đường tự nối chuỗi `try_auto_chain` (khi `battle_result == 0`).
- [ ] Điều hướng nhánh kịch bản hoàn toàn dựa trên `EveCondition` (`cc=2`, `cc=12`, ...) và thứ tự ưu tiên của `EveResolver`, giữ nguyên 4 lớp chống lặp (`SameCondition`, `ReQuestion`, `ReBattle`, `DuplicateItem`) của `EveAutoChainEngine`.
- [ ] Có test E2E qua `dispatcher::dispatch` cho chuỗi quest nhiều bước cùng `eve_no` thiếu `cc=12` (nhận bước 1 -> quay lại trả bước 2 -> hoàn thành) và cho NPC có ít nhất 2 quest độc lập (hoàn thành quest đứng sau vẫn nhận/trả được quest đứng trước).
- [ ] Có negative test xác nhận quest một lần thực sự có điều kiện chặn (`cc=12` hoặc `cc=2`) vẫn không cho nhận lại, và các guard chống lặp của `AutoChain` vẫn ngắt đúng khi điều kiện không đổi.
- [ ] Toàn bộ test suite liên quan và clippy xanh 100%, không gây hồi quy cho các quest `01..10` hiện hữu.
