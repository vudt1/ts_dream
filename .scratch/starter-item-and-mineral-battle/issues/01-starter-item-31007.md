# 01: Cấp `1x 31007` (Vé đoái hoán tâm) trong Đồ mặc định tạo nhân vật

**What to build:** Khi người chơi tạo nhân vật mới (`0x09 Sub 01`), nhân vật tự động có sẵn `1x` *Vé đoái hoán tâm* (`31007`) ở ô `2` của `homdo` bên cạnh `32012 × 4` ở ô `1` và trang bị `19737 × 1` ở ô `2` của `trangbi`, được lưu nguyên tử trong cùng transaction tạo nhân vật và đồng bộ đầy đủ qua khung `17 05` khi vào thế giới. Nghiệp vụ `characters.newbie` (`= 0`) vẫn được giữ nguyên hoàn toàn cho luồng gift code `TSVN123/TSVN456` và sách `46238` theo `ADR-0001`, đồng thời không cấp bù `31007` cho nhân vật cũ khi Enter Game.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Tạo nhân vật mới qua `0x09 Sub 01` ghi nhận `homdo` có `32012 × 4` ở ô `1` và `31007 × 1` ở ô `2`, cùng `trangbi` có `19737 × 1` ở ô `2` trên cả RAM `Session` lẫn bảng `inventories` trong cùng một SQLite write transaction.
- [x] Giao dịch tạo nhân vật (bao gồm đường dẫn in-memory fallback / golden replay) giữ `characters.newbie = 0` sau khi tạo và sau khi vào game.
- [x] Khi nhân vật mới tạo gửi lệnh vào game (`0x03 Sub 01`), khung `17 05` phát về client chứa đúng `31007 × 1` ở ô `2`, và lần đăng nhập lại (`0x01`) tiếp theo không nhân đôi vật phẩm này.
- [x] Nhân vật cũ đã tồn tại trong DB với `characters.newbie = 0` khi đăng nhập vào game không bị tự động chèn thêm `31007`.
- [x] Trường hợp tạo nhân vật thất bại (ví dụ trùng tên nhân vật) không để lại bản ghi `31007` rác trong `inventories`.
- [x] Có kiểm thử tự động trong domain suite `server_network` xác minh cả các ca tích cực, ca từ chối (tạo thất bại, nhân vật cũ không cấp bù) và toàn vẹn ô phụ cận (`32012 × 4` ở ô `1`).
