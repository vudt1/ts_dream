# 02: Frame 33 01 — cập nhật HP/SP unit trong trận

**What to build:** Client chỉ cập nhật HP/SP hiển thị của unit trong trận qua frame `33 01` — hiện server chưa có builder này nên thanh máu/thể lực trên lưới không bao giờ đổi. Thêm builder `33 01` (record 7 byte: `[grp][slot][propId][value u32 LE]`, propId `0x19` = HP, `0x1A` = SP; với `0x19` + value>0 client chạy hiệu ứng hồi sinh) và phát frame cho mọi thành viên trận khi HP/SP của unit thay đổi: đòn đánh, hồi máu, dùng item, hồi sinh, thú.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Builder `33 01` byte-exact theo `02_battle_positions.md §2.5` (7 byte/record; nhiều record trong một frame được)
- [x] Đòn đánh/hồi máu/đổi SP trong trận phát `33 01` tới các client trong trận (không chỉ ghi DB)
- [x] Hồi sinh (HP 0 → >0) dùng propId `0x19` + value>0 để client chạy effect
- [x] Test trong `tests/` (ví dụ `tests/battle_frames_test.rs`): hex frame cho set HP, damage, revive; tổng độ dài frame đúng
