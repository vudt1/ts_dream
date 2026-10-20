# 18: Đấu nối quy tắc timing các frame 0x35 vào Battle Runner

**What to build:** Đấu nối các gói tin thuộc Opcode `0x35` vào bộ điều phối trận đấu `src/battle/runner.rs` theo đúng 4 pha của vòng đời hiệp đấu (Command -> Stance -> Action -> Cleanup):
1. Trước khi unit thi triển đòn đánh: bắn `35 03` (`force_state7`) để xả FX cũ và đưa unit vào tư thế chiến đấu rút vũ khí (State 7).
2. Khi kỹ năng buff/debuff tác động thành công mục tiêu: bắn `35 0F` (`battle_unit_status`) để hiển thị icon và số hiệp trên HUD client.
3. Khi buff/debuff hết hiệp hoặc bị giải trừ: bắn `clear_unit_status` (`35 0F` flag=0, val=0) và `purge_status` (`35 01`).
4. Nếu trận đánh có sự kiện thoại NPC mở màn: bắn `35 06`.

**Blocked by:** 12: Builder 35 03 (ép state animation), 16: Builder 35 06 (Thoại NPC / banner diễn hoạt trong trận), 17: Builder 35 0B (Tin nhắn / bảng tin thông báo chiến đấu)

**Status:** ready-for-agent

- [ ] Runner phát `force_state7` cho ô kẻ tấn công ngay trước khi xử lý gói diễn hoạt hành động `32 01`.
- [ ] Runner phát `battle_unit_status` cho các mục tiêu bị ảnh hưởng bởi kỹ năng buff/debuff chỉ số và phong ấn.
- [ ] Runner phát `clear_unit_status` và `purge_status` khi hiệu ứng hết hạn vào đầu hiệp hoặc khi bị chiêu giải trừ dập tắt.
- [ ] Integration test mô phỏng trận đấu kiểm tra chuỗi frame phát ra tuân thủ đúng thứ tự timing mà không gây lỗi hoặc treo client.
