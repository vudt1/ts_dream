# 16: Builder 35 06 (Thoại NPC / banner diễn hoạt trong trận)

**What to build:** Thêm builder cho gói tin biến cố trận đấu `35 06` = `[35][06][k:1B][actor_id:4B LE][tpl_id:4B LE]` (payload 11 byte, frame 15 byte) theo đặc tả `opcode_35.md §4.3` (`FUN_0064e064`) và đối chiếu Bear C# (`PacketCreator(53, 6)`). Gói tin này kích hoạt tạo thực thể thoại `THuman` và hiển thị banner toast thoại 2000ms cho NPC/tướng trong các trận đánh kịch bản hoặc sự kiện trận đấu.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Builder `packets::battle_npc_talk(k: u8, actor_id: u32, tpl_id: u32) -> String` byte-exact với payload 11 byte (`F4440B003506...`).
- [x] Hỗ trợ hàm bọc tiện ích `packets::battle_npc_dialog(dialog_id: u16) -> String` tương thích với quy ước `PacketCreator(53, 6)` của Bear C# (`k=1`, `actor_id=0`, `tpl_id=dialog_id`).
- [x] Bộ kiểm thử trong `tests/` (`tests/battle_talk_notice_test.rs`) xác nhận độ dài, cấu trúc byte Little-Endian và cờ hiển thị.
