# 05: Kiến trúc `Pure Planner` + `EventRunner` (`Pre-Flight All-or-Nothing`, `AutoChain`, `Skill` & `FinishChain` Write-Through)

**What to build:** Chuyển hóa luồng xử lý nói chuyện/tiếp tục/chọn menu/nối chuỗi sang kiến trúc thuần `Pure Planner` (`DomainEffect`) + vòng lặp hợp nhất `EventRunner`: kiểm tra sức chứa trước (All-or-Nothing) cho `homdo` và `Quest Ledger`, giữ nguyên khóa hội thoại xuyên `AutoChain`, chỉ đánh dấu `quest_dont` khi hoàn thành nhiệm vụ thực sự, trao kỹ năng `Skill` miễn phí kèm đồng bộ `08 01 6E`, và ghi DB write-through ngay tại `FinishChain`.

**Blocked by:** `04: Thủy hóa PlayerEventSnapshot, Fail-Closed Evaluator & Lọc 0x14 Surface (MenuChoice)`

**Status:** resolved

- [x] Xây dựng tầng `Pure Eve Planner` (thuần, không I/O/lock) dịch các `EveResult` được chọn (bao gồm `GroupData` random/all) sang `Vec<DomainEffect>`, chỉ phát `0x14 01..06` cho `result_type ∈ 0..=6` mà client `FUN_005eb530` hiểu, tuyệt đối không gửi thô `result_type >= 7` (`7, 8, 10` đã xác nhận 100% không có trong `aLogin.exe`) qua `0x14`. *(Lưu ý: `rt == 3` khiến client gửi `C→S [0x0B, 0x01, [TFightForm1+0x130]]`; `rt == 6` `TSurface` gửi `C→S 0x14 09` với `20/21/40/30..39/61+`.)*
- [x] Thực hiện `Pre-Flight Probe All-or-Nothing` trước khi ghi nhận biến đổi tài nguyên: kiểm tra đủ ô trống `homdo (0x17)` cho mọi item `Item.dat` (`result_class = 1` gồm cộng và trừ số lượng lẻ) và đủ chỗ trong 200 ô `Quest Ledger (0x18)` — nếu bất kỳ phần nào thiếu chỗ thì từ chối toàn bộ bước thực thi (phát `S→C 0x18 03` khi tràn Quest Ledger), không làm thay đổi túi đồ, vàng hay bước quest.
- [x] Gán cờ `quest_dont` (`0x18 05` `bit_id 1..=2400`) **chỉ** khi nhiệm vụ thực sự hoàn tất hoặc qua lệnh đánh dấu tường minh (gỡ bỏ cơ chế Bear save-gate tự đánh dấu ở các bước trung gian), đồng thời hỗ trợ đầy đủ các `parameter_style` thực tế của `result_class = 2`.
- [x] Hỗ trợ nhánh trao kỹ năng `Skill (rc=4)` miễn `skill_point` kèm gói đồng bộ và ghi bền vững xuống DB.
- [x] Hợp nhất vòng lặp `EventRunner` theo thiết kế state machine đã chốt trong ADR 0009 (giữ `14 2C Lock` một lần xuyên `AutoChain`, sửa `detect_duplicate_items` hỗ trợ `p_style = 0` có `result_value` âm, không khóa khi bước 0 là `StartBattle`, chỉ phát `14 2C Unlock` + `14 08 EndTalk` một lần khi kết thúc thật, và gọi write-through DB nguyên tử tại `FinishChain`):

```text
Trigger -> Pre-Flight Probe -> Apply State Mutations -> Inspect Head:
  * AwaitMenuChoice -> Lock (if not locked) + Surface -> Wait choice
  * StartBattle     -> Cleanly Close/Unlock open Talk UI -> DB Checkpoint -> BattleService
  * Warp            -> Sync State + 0x0C -> DB Checkpoint
  * FinishChain     -> Increment (scene_id, eve_no) -> Try AutoChain ->
                         - Matched next chain -> Continue loop (Keep Lock!)
                         - None               -> DB Checkpoint -> Unlock + EndTalk (once)
```

- [x] Kiểm thử E2E qua `dispatcher::dispatch` + `setup_test_db()` cho chuỗi nhiệm vụ `12001` bắt sơn tặc Sĩ Triết (lặp lại + thưởng vàng, kiểm tra bảo toàn slot lân cận) và `12136` Giản Ung khảo nghiệm (trả lời chọn `0x14 09` -> nhận kỹ năng hệ Tâm `14001` + `FinishChain` write-through bền vững sau reload DB), cùng các negative test từ chối All-or-Nothing khi đầy `homdo` 25 ô hoặc đầy 200 ô `Quest Ledger`.
