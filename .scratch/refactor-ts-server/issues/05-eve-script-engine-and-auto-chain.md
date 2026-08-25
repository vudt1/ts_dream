# 05 — 4-Tier Eve Script Engine and Auto-Chain Resolver

**What to build:** Xây dựng động cơ kịch bản sự kiện nhiệm vụ và hội thoại `Eve Engine` hoàn chỉnh thay thế toàn bộ hệ thống quest cũ: `EveStateBuilder` (chụp snapshot trạng thái người chơi), `EveConditionEvaluator` (đánh giá 15 loại điều kiện), `EveChainResolver` (ghép chuỗi AND và sắp xếp 4 tầng ưu tiên), và `EveAutoChainEngine` (tự động nối tiếp sự kiện với 4 lớp bảo vệ chống lặp vô hạn).

**Blocked by:** 04 — Eve.emg Container Parser and Script Models

**Status:** completed

- [x] `EveConditionEvaluator` đánh giá chính xác 15 condition classes: Item túi đồ (đảo ngược), Bước nhiệm vụ, Thuộc tính nhân vật, Kết quả trận đấu, Sở hữu võ tướng, Lựa chọn dialog, Số lần hoàn thành event, Role counts. (9 class có ngữ nghĩa xác thực (0,1,2,7,8,9,10,12,14); class 3-6,11,13,15 fail-close như bản tham chiếu Kotlin)
- [x] `EveChainResolver` nhóm chuỗi AND (`andNum`) và thực hiện sắp xếp 4 tầng: `stepScore` cao nhất $\to$ chuỗi điều kiện dài nhất $\to$ số kết quả nhiều nhất $\to$ thứ tự xuất hiện.
- [x] `EveAutoChainEngine` tự động kích hoạt event tiếp theo và áp dụng 4 lớp bảo vệ: Same Condition Detection, Re-Question Prevention, Re-Battle Prevention, Duplicate Item Prevention.
- [x] Tích hợp `apply_group_data` (weighted random selection) cho các kết quả sự kiện có xác suất. (`DotNetRandom` parity với battle RNG)
- [x] Bộ test suites kiểm thử độc lập cho từng module của Eve Engine (Resolver, Evaluator, AutoChain). — `tests/eve_engine.rs`: 47 tests pass (kèm GroupData + StateBuilder suites)

Ghi chú triển khai: so sánh thuộc tính số class=7 (param=0/2) dùng chiều đảo ngược `conditionValue ops actual` đúng theo bản tham chiếu Kotlin + dữ liệu eve.emg thật (gate "level > T" viết `ops=2 val=T`); test file Kotlin tự mâu thuẫn với implementation nên đã bỏ qua 2 assertion đó.
