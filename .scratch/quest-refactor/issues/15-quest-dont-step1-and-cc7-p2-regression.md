# 15: [P1] Sửa `mark_dont_on_step1` (`quest_dont`) & Regression Test `cc=7 param=2`

**What to build:** Ngừng bật sớm cờ hoàn thành `quest_dont = 1` ở bước tăng `+1` (`p_style=1, step=1`) để các nhiệm vụ nhiều bước có `Mark.dat position > 0` (`10012, 11443, 11863, 12325, 12358, 12377, 12406`) chạy trọn vẹn tới bước cuối cùng, đồng thời bảo đảm `quest_dont` chỉ được đánh dấu khi có lệnh hoàn tất tường minh (`MarkDont` / `ExplicitMark`) và giữ nguyên hành vi `reborn_count` cho `cc=7, param=2, p_style=4`.

**Blocked by:** `11: [P0] Gỡ should_skip_event & Mở khóa Chuỗi Quest Đa giai đoạn / NPC Nhiều Quest`

**Status:** resolved

- [x] Xóa bỏ nhánh tự động đặt `quest_dont = 1` dựa trên `mark_dont_on_step1 && step == 1` trong `EventRunner` (và đơn giản hóa trường cờ thừa trong `Planner` nếu không còn người dùng nào khác); lệnh `rt=0, rc=2, p_style=1` chỉ cập nhật bước trong `Quest Ledger`.
- [x] Bảo đảm `set_quest_dont(..., 1)` (phát `S->C 0x18 05`) chỉ thực thi qua `QuestOp::MarkDont` (`p_style=5`) hoặc `QuestOp::ExplicitMark` (`p_style=3` với `result_value > 0`).
- [x] Có test E2E qua `dispatcher::dispatch` mô phỏng nhiệm vụ nhiều bước có `Mark.dat position > 0` (ví dụ chuỗi tăng bước `p_style=1, val=1` ở bước 1 rồi kiểm tra `cc=2` bước 2 và `cc=2 p_style=3` ở bước cuối): xác nhận bước 1 không phát `0x18 05`, bước 2 vẫn mở bình thường, và chỉ khi hoàn tất bằng `MarkDont`/`ExplicitMark` mới bật bit `quest_dont`.
- [x] Bổ sung regression test xác nhận `cc=7, param=2, p_style=4` tiếp tục đánh giá nhất quán theo `reborn_count` theo đúng quyết định chốt.
