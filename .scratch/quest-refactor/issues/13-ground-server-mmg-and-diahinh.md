# 13: [P1] Bóc tách `ground_server.mmg`, Sửa `GroundMmgLoader` & Chuẩn hóa `DiaHinh` Trận đấu

**What to build:** Bóc tách dữ liệu bản đồ gọn nhẹ dành riêng cho server (`ground_server.mmg`, ~`217 KB`, giữ `width`, `height` và `geolBaseAtt`), loại bỏ tệp `ground.mmg` `26.7 MB` của client khỏi repo, sửa lỗi clone buffer lặp `4,528` lần khi nạp, và sử dụng `geolBaseAtt` từ `GameData` làm nguồn `DiaHinh` chuẩn cho các trận chiến Eve và ngộ địch ngẫu nhiên thay cho `SceneInfo[1].background_no` (vốn làm sai nền `"FField<N>"` và tràn byte cao sang `BKMusic` trên `aLogin.exe`).

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Xây dựng công cụ trích xuất (CLI/example) đọc `ground.mmg` gốc của client và xuất `Data/ground_server.mmg` giữ nguyên chuẩn `MapData.New` (index đuôi `29B/mục + count(u16)`), rút gọn `mapPicCount = 0`, `blockWidth = 0`, `blockHeight = 0`, `waveCount = 0`, `elementCount = 0`, `npcCount = 0` nhưng giữ đầy đủ `width` (`i32`), `height` (`i32`) và `geolBaseAtt` (`u8`) cho toàn bộ `4,528` bản đồ.
- [x] Sửa `GroundMmgLoader` đọc trực tiếp trên slice `&[u8]` (xóa bỏ hoàn toàn clone buffer theo từng map trong vòng lặp), mở rộng API trả metadata `(width, height, geolBaseAtt)` bên cạnh `map_id -> geolBaseAtt`, và xóa `Data/ground.mmg` khỏi repo sau khi kiểm chứng khớp 100% `4,528` map với dữ liệu gốc.
- [x] Đấu nối `GroundMmgLoader` vào `GameData` (ưu tiên nạp `ground_server.mmg`, fallback `ground.mmg` nếu tồn tại) và cập nhật `ground_test` đọc `ground_server.mmg` kèm kiểm chứng `width`/`height` hợp lệ.
- [x] Tại các luồng mở trận Eve (`EventRunner`) và ngộ địch ngẫu nhiên / Bắc Đẩu (`MovementHandler`), bỏ đọc `scene_infos.get(&1).background_no`; tra `geolBaseAtt` của bản đồ hiện tại (fallback `112` khi thiếu bản ghi) để bảo đảm gói `0x0B 0xFA` truyền đúng mã `"FField<N>"` ở byte thấp và byte cao bằng `0`.
- [x] Có test E2E / unit test xác nhận khung `0x0B 0xFA` phát ra cho trận Eve và Random Encounter mang đúng `geolBaseAtt` của bản đồ (fallback `112`), byte cao bằng `0`, và không bị Map ID `> 255` làm tràn sang `BKMusic`.
