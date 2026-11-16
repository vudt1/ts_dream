# 12: [P0] Giải mã `EveSceneInfo.dest_map_id` & Dịch chuyển Warp `rt=2` Chuẩn PC

**What to build:** Khôi phục `4,857` điểm dịch chuyển sự kiện (`47.0%`) đang bị bỏ qua và loại bỏ sai lệch bản đồ/tọa độ do phụ thuộc vào `Warps.txt`, bằng cách giải mã `dest_map_id` từ `EveSceneInfo` trong `Data/eve.emg` và ưu tiên `EveSceneInfo` (kèm quy đổi tọa độ lưới sang pixel) cho toàn bộ lệnh warp `rt=2` trực tiếp cũng như hoãn warp hậu chiến đấu.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Mở rộng `EveSceneInfo` trong bộ nạp `EveDataLoader` để đọc và lưu `dest_map_id` (`u16`) thay cho thao tác bỏ qua 2 byte trước `player_appear_x`.
- [x] Tại mọi điểm thực thi `DomainEffect::Warp` và hoãn warp hậu chiến đấu (bao gồm `EventRunner`, `QuestHandler` và `BattleService`), ưu tiên tra cứu `scene_infos` của bản đồ hiện tại theo `warp_id == EveSceneInfo.eve_no`: khi `dest_map_id > 0` và `player_appear_x > 0, player_appear_y > 0`, dịch chuyển người chơi (và tổ đội theo `Two-Phase State Lock`) tới `dest_map_id` với tọa độ pixel `(player_appear_x * 20 - 10, player_appear_y * 20 - 10)`; chỉ fallback sang `Warps.txt` khi `scene_infos` không có khóa tương ứng.
- [x] Thống nhất toàn bộ thông điệp log và chú thích gọi nhầm `"Warps.dat"` sang đúng tên `"Warps.txt"`.
- [x] Có test kiểm chứng bộ nạp trên `Data/eve.emg` xác nhận `10,336 / 10,336` lệnh `rt=2` phân giải được `EveSceneInfo` có `dest_map_id > 0` và tọa độ dương.
- [x] Có test E2E qua `dispatcher::dispatch` (và test hậu chiến đấu) xác nhận lệnh `rt=2` chỉ tồn tại trong `EveSceneInfo` dịch chuyển thành công + lưu DB `savemap`/vị trí tổ đội; khi trùng khóa với `Warps.txt` ưu tiên đúng `EveSceneInfo`; và negative test khi thiếu cả hai nguồn hoặc `dest_map_id == 0` thì bỏ qua an toàn không đổi `map_id`.
