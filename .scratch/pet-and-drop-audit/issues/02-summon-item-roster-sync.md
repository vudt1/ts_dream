# 02: Đồng bộ hiển thị sủng vật tức thì khi dùng thẻ triệu hồi (Summon Item Roster Sync)

**What to build:**  
Khi người chơi sử dụng vật phẩm triệu hồi sủng vật (`add_pet > 10000`) thành công từ hòm đồ (`homdo`), server lưu ngay sủng vật mới vào cơ sở dữ liệu SQLite (`persist::upsert_pet`), đồng thời gửi tức thì các gói tin đồng bộ giao diện (`0F 07` nạp dữ liệu chi tiết sủng vật và `0F 01` nạp hiện diện sủng vật) về client của người chơi. Giúp sủng vật mới xuất hiện ngay lập tức trên bảng danh sách thú cưng của UI client mà không cần phải thoát game đăng nhập lại.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Trong `use_item/mod.rs` (nhánh `add_pet > 10000`), ngay sau khi push `PetState` vào `session.pets`, thực thi lưu bản ghi sủng vật xuống database thông qua `persist::upsert_pet`.
- [x] Xây dựng và gửi frame dữ liệu chi tiết sủng vật `0F 07` (`pet_info_frame` layout: `0F 07 <pet_id: 4B LE><stt: 1B><pet_id: 4B LE><quest: 1B><name_len: 1B><name>`) về client.
- [x] Gửi frame hiện diện sủng vật `0F 01` (`0F 01 <owner_id: 4B LE><stt: 1B><pet_id: 4B LE><quest: 1B>`) về client.
- [x] Có unit test kiểm thử: Sử dụng vật phẩm có `add_pet > 10000` $\rightarrow$ kiểm tra vật phẩm bị tiêu hao (`homdo` giảm 1), sủng vật được ghi nhận trong DB, và client nhận được cả 2 frame `0F 07` và `0F 01`.
