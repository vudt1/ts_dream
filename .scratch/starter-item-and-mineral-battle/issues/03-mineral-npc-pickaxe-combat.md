# 03: Luật Vũ khí Cuốc & Miễn nhiễm cho Quái Khoáng trong Trận đấu

**What to build:** Khi dựng bàn cờ chiến đấu, mỗi ô Nhân vật (ô `3` của `trangbi`) và mỗi ô Sủng vật (ô `pet.stt * 10 + 3` của `trangbi`) nạp độc lập mã vũ khí đang mặc vào `WarInfo` (không thay đổi định dạng gói tin `23` byte của ô cờ). Trong trận đấu, mọi Quái Khoáng — định nghĩa là NPC địch (`typ == 3` hoặc `7`) có `kind == 16` (`25001..=25252`) hoặc ID thuộc `{22010, 22011}` — đều bị cấm thu phục bằng kỹ năng `15001`, và chỉ nhận sát thương hoặc hiệu ứng bất lợi từ đấu sĩ đang tự trang bị một trong 6 Vũ khí Cuốc (`[10001, 10530, 10531, 10532, 10533, 10534]`); mọi đòn đánh/kỹ năng gây hại từ đấu sĩ không cầm Cuốc vào ô Quái Khoáng đều rơi về `MISS` (`0` sát thương, không hiệu ứng).

**Blocked by:** 02: Tách `equip_pos` khỏi `kind` để Nhân vật & Sủng vật mặc được Vũ khí Cuốc

**Status:** ready-for-agent

- [ ] Khi dựng trận đấu từ `Session`, `WarInfo.weapon_id` được nạp độc lập cho Nhân vật (từ `trangbi` ô `3`) và cho từng Sủng vật (cả đội trưởng lẫn thành viên tổ đội, từ `trangbi` ô `pet.stt * 10 + 3`), giữ nguyên cấu trúc snapshot `23` byte `0B 05` / `0B FA`.
- [ ] Đấu sĩ (Nhân vật hoặc Sủng vật) đang trang bị bất kỳ cây Cuốc nào trong `[10001, 10530, 10531, 10532, 10533, 10534]` khi đánh thường (`10000`), dùng kỹ năng vật lý (`skill_type == 1`), phép thuật (`skill_type == 2`, `19`) hoặc debuff/khống chế (`type 3`, `15`) lên Quái Khoáng (`kind == 16`, `22010`, `22011`) được tính toán sát thương và hiệu ứng bình thường.
- [ ] Đấu sĩ **không** trang bị 1 trong 6 Cuốc chuẩn (bao gồm không cầm vũ khí, cầm vũ khí thường, hoặc cầm `10497` *Hoàng Cân Sừ Đầu*, `10499`, `10500`) khi tấn công hoặc dùng bất kỳ kỹ năng gây hại nào (đánh thường, vật lý, phép, `Holy Spirit` `19`, khống chế/debuff/hút SP `type 3`/`15`) vào ô Quái Khoáng đều trả về khung `MISS` (`miss_status::MISS = 0`, `damage = 0`), không áp dụng hiệu ứng phụ, không hút SP/hồi HP, không kích hoạt phản đòn khiên, trong khi vẫn trừ SP và đánh dấu kết thúc lượt của người đánh.
- [ ] Kiểm tra trang bị Cuốc độc lập tuyệt đối giữa Nhân vật và Sủng vật: Nhân vật cầm Cuốc không giúp Sủng vật đánh trúng Quái Khoáng (nếu Sủng vật không cầm Cuốc), và ngược lại Sủng vật cầm Cuốc không giúp Nhân vật hay đồng đội đánh trúng Quái Khoáng.
- [ ] Với kỹ năng AoE trên đội hình địch hỗn hợp (quái thường + Quái Khoáng) và với **Hợp kích / Combo** (thành viên có Cuốc + thành viên không có Cuốc cùng nhắm Quái Khoáng), việc kiểm tra miễn nhiễm thực hiện đúng theo từng mục tiêu và từng người đánh trong khung `32 01`.
- [ ] Khi Quái Khoáng ở hàng trước được đơn vị ở hàng sau *Hộ chủ* (`20006`) đỡ đòn (hoặc ngược lại), điều kiện miễn nhiễm khoáng được đánh giá trên đúng thực thể thực sự nhận sát thương (`rear`).
- [ ] Kỹ năng *Thu phục Võ tướng* (`15001`) luôn bị từ chối khi nhắm vào bất kỳ Quái Khoáng nào thuộc `25001..=25252`, `22010` (*Thủy Tinh Quái*) hoặc `22011` (*KhoángChìQuái*), kể cả khi HP mục tiêu dưới 50% MaxHP.
- [ ] Sủng vật phe người chơi (`typ == 4`) mang ID `22010` hoặc `22011` khi tham gia PvP/trận đấu **không** được hưởng miễn nhiễm khoáng.
- [ ] Có kiểm thử tự động toàn diện trong domain suite `battle` bao phủ đầy đủ các kịch bản dương tính, âm tính (bao gồm AoE hỗn hợp, Combo hỗn hợp, *Hộ chủ*, cấm thu phục `22010`/`22011`, PvP pet `typ == 4`).
