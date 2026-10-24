# 21: Frame 32 02 kết thúc diễn hoạt hiệp đấu (turn_action_finish)

**What to build:** Xây dựng builder cho gói tin báo hoàn tất chuỗi hoạt ảnh chiêu thức `32 02` (`F4 44 02 00 32 02`) theo mã máy client `FUN_0064dd74` (`case_043`). Phát frame này ra `BattleSink` ở cuối mỗi hiệp đấu sau khi tất cả các gói tin diễn hoạt `32 01` đã phát xong, giúp client gán cờ `[TFightManage+0x1C] = 1`, giải phóng hàng đợi hoạt ảnh và chuẩn bị sẵn sàng cho hiệp tiếp theo.

**Blocked by:** 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**Status:** completed

- [x] Builder `packets::turn_action_finish() -> String` tạo frame chuẩn byte-exact: `F44402003202`.
- [x] Luồng điều phối trận đấu phát frame `32 02` ngay sau khi kết thúc chuỗi đòn đánh `32 01` của hiệp hiện tại.
- [x] Test unit trong `tests/battle_frames_test.rs` kiểm tra độ dài và giá trị hex chuẩn của frame `32 02`.
- [x] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
