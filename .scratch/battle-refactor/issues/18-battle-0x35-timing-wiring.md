# 18: Đấu nối quy tắc timing các frame 0x35 & Dynamic Skill Delay Pacing vào Battle Runner

**What to build:** Đấu nối các gói tin thuộc Opcode `0x35` và điều phối nhịp độ nghỉ giữa hiệp vào bộ điều phối trận đấu theo đúng 4 pha của vòng đời hiệp đấu (Command -> Stance -> Action -> Cleanup):
1. **Pha 1 (Đầu hiệp)**: Bắn `35 01` (`purge_status`) và `clear_unit_status` (`35 0F` flag=0) để gỡ bỏ hiệu ứng/icon trạng thái đã hết thời hạn.
2. **Pha 2 (Khởi động đòn đánh)**: Trước khi unit thi triển đòn đánh: bắn `35 03` (`force_state7`) để xả FX cũ và đưa unit vào tư thế chiến đấu rút vũ khí (State 7).
3. **Pha 3 (Tác động chiêu thức)**: Khi kỹ năng buff/debuff tác động thành công mục tiêu: bắn `35 0F` (`battle_unit_status`) để hiển thị icon và số hiệp trên HUD client.
4. **Pha 4 (Dọn dẹp & Nghỉ diễn hoạt)**: Bắn `35 06` nếu có thoại NPC mở màn; phát `32 02` (`turn_action_finish`) và cho task battle nghỉ một khoảng thời gian động **Dynamic Skill Delay Pacing** (Quyết định Q3: A):
   $$\text{sleep\_ms} = \text{clamp}\left(\sum \text{delay}_{\text{action}} \times 100\,\text{ms},\, 800\,\text{ms},\, 3500\,\text{ms}\right)$$
   trước khi vòng lặp chuyển sang Bước 1 của hiệp tiếp theo.

**Blocked by:** 12: Builder 35 03 (ép state animation), 16: Builder 35 06 (Thoại NPC / banner diễn hoạt trong trận), 17: Builder 35 0B (Tin nhắn / bảng tin thông báo chiến đấu), 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01, 21: Frame 32 02 kết thúc diễn hoạt hiệp đấu (turn_action_finish)

**Status:** completed

- [x] Runner phát `force_state7` cho ô kẻ tấn công ngay trước khi xử lý gói diễn hoạt hành động `32 01`.
- [x] Runner phát `battle_unit_status` cho các mục tiêu bị ảnh hưởng bởi kỹ năng buff/debuff chỉ số và phong ấn.
- [x] Runner phát `clear_unit_status` và `purge_status` khi hiệu ứng hết hạn vào đầu hiệp hoặc khi bị chiêu giải trừ dập tắt.
- [x] Task trận đấu trong `manager.rs` áp dụng dynamic sleep sau `32 02` để client kịp chiếu xong animation trước khi nhận `34 01` hiệp mới.
- [x] Integration test mô phỏng trận đấu kiểm tra chuỗi frame phát ra tuân thủ đúng thứ tự timing mà không gây lỗi hoặc treo client.
- [x] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
