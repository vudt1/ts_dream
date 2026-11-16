# 14: [P1] Sửa Ngữ nghĩa Engine: `when_happen` Cửa, `GroupData.use_mode == 2` & `EveResolver` `chain[1..]`

**What to build:** Khắc phục 3 sai lệch ngữ nghĩa cốt lõi trong bộ chọn và lập kế hoạch sự kiện Eve để người chơi không bị kích hoạt nhầm sự kiện click cửa khi đi ngang (và ngược lại) mà vẫn giữ nguyên tương thích với `3,122` NPC `on_ser_stroke` và `1,835` Encounter `on_click_obj`, nhận đủ toàn bộ phần tử nhóm có `use_mode == 2`, và nhận đủ kết quả gắn ở các điều kiện sau đầu chuỗi AND (`chain[1..].results`).

**Blocked by:** `11: [P0] Gỡ should_skip_event & Mở khóa Chuỗi Quest Đa giai đoạn / NPC Nhiều Quest`

**Status:** ready-for-agent

- [ ] Áp dụng lọc `when_happen` riêng cho nhánh cửa (`Door`): `ClickDoor` chỉ khớp sự kiện có `on_click_obj` (`when_happen[0]`), `MeetDoor` chỉ khớp sự kiện có `on_stroke_obj || on_into_area` (`when_happen[1] || when_happen[2]`).
- [ ] Bảo đảm `ClickNpc` chấp nhận `on_click_obj || on_stroke_obj || on_ser_stroke` (kèm fallback khi cả 4 cờ đều `false`) và `WalkStep` Encounter không bị lọc loại bởi `when_happen`, giữ hoạt động nguyên vẹn cho `3,122` NPC `on_ser_stroke` và `1,835` Encounter `on_click_obj`.
- [ ] Trong `GroupData`, kiểm tra `use_mode == 2` trước `pick_member` và trọng số để trả về `KEEP_ALL` (giữ toàn bộ phần tử trong nhóm), trong khi giữ `use_mode == 1` và `use_mode == 3` đi theo đường chọn ngẫu nhiên / `pick_member` hiện tại.
- [ ] Trong `EveResolver::resolve`, thu thập `results` từ tất cả các mắt xích `chain[0..]` theo đúng thứ tự chuỗi thay vì chỉ `chain[0].results` (khôi phục `29` node thuộc `15` chuỗi AND), bảo đảm tổng kết quả chuỗi khác rỗng và cập nhật doc comment khớp 3 tiêu chí + tie-break thứ tự khai báo.
- [ ] Có unit test và E2E test kiểm chứng phân tách `ClickDoor` vs `MeetDoor`, giữ nguyên `on_ser_stroke` NPC & `on_click_obj` Encounter, `use_mode == 2` phát đủ mọi phần tử kèm bảo toàn slot `homdo` phụ cận, và chuỗi AND nhiều mắt xích có `results` ở `chain[1..]` thực thi đầy đủ.
