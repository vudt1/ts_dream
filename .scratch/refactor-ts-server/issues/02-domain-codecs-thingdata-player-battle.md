# 02 — Standardized Domain Codecs (ThingData, Player, Battle)

**What to build:** Xây dựng các Serializer và Codec chuyên biệt cho các thực thể miền chính: `ThingDataCodec` (chuẩn hóa 35-byte item struct chứa đủ 20 thuộc tính: linh khí, ngọc khảm, chuyên vũ, thời trang, 3 dòng tẩy luyện, hạn dùng OADate), `PlayerInfoCodec` (thẻ nhân vật và danh sách bạn bè) và `BattleRoleSerializer` (dữ liệu ngoại hình thực thể trong trận đấu).

**Blocked by:** 01 — Binary Packet Reader and Writer

**Status:** completed

- [x] `ThingDataCodec` serialize và deserialize chính xác cấu trúc 35 bytes của vật phẩm theo chuẩn Mobile/PC.
- [x] `PlayerInfoCodec` đóng gói thẻ bài người chơi (`write_player_card`) và thông tin bạn bè (`write_friend_extra`).
- [x] `BattleRoleSerializer` tuần tự hóa các thực thể tham chiến (Player, Pet, Summon, NPC) cho các opcode `0x0B` và `0x32`.
- [x] Unit tests kiểm tra roundtrip serialization/deserialization cho từng codec.
