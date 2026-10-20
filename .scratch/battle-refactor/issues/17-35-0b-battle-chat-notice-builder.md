# 17: Builder 35 0B (Tin nhắn / bảng tin thông báo chiến đấu)

**What to build:** Thêm builder cho gói tin thông báo chiến sự `35 0B` = `[35][0B]([type:1B][len:1B][text:len bytes])*n` (tối đa 21 record) theo đặc tả `opcode_35.md §4.3` (`FUN_0075c518`). Gói tin này đẩy các dòng thông báo chiến trận và sự kiện trực tiếp vào giao diện khung chat trận đấu `TTalkMsgForm` của client TS Online, sử dụng 3 tiền tố định dạng tích hợp sẵn (`type` 1, 2, 3).

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Builder `packets::battle_chat_notice(entries: &[(u8, &str)]) -> String` hỗ trợ gom từ 1 đến 21 bản ghi, mã hóa chuỗi theo VISCII 1.1 với độ dài động byte-exact.
- [ ] Helper `packets::battle_system_message(text: &str) -> String` gửi một dòng thông báo hệ thống đơn lẻ trong trận (`type = 1`).
- [ ] Bộ kiểm thử trong `tests/` kiểm tra round-trip chuỗi có dấu tiếng Việt (VISCII), giới hạn độ dài và định dạng byte header/payload.
