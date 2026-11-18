# 03: Đồng bộ Đăng nhập (`17 39` + Tối đa 1 Popup `24 19` theo `season_seq`) & Bộ `FestivalDatEncoder`

**What to build:** Từ góc nhìn người chơi đăng nhập vào game khi có lễ hội đang mở và người vận hành muốn mở rộng đủ bảng lore trên client, server tự động gửi thông báo mùa (`17 39`) khi đăng nhập và đẩy tối đa `1` hộp thoại *Bảng Lore Lễ hội* (`S→C 0x24 0x19`) chưa từng nhận trong đợt `season-<seq>` hiện tại; đồng thời cung cấp `FestivalDatEncoder` và công cụ CLI xuất file `Festival.Dat` đã cấp khóa phân biệt `1..=7` (và tùy chọn `8..=11`) để đồng bộ sang client mà giữ nguyên file gốc trong repo.

**Blocked by:** 02: Wire Parity `S→C 0x24 0x19` (`Festival.Dat` Gốc), `SeasonalNotice` (`0x17 0x39`), Pháo hoa (`0x17 0x46`) & Broadcast Trực tiếp

**Status:** ready-for-agent

- [ ] Thêm migration SQLite cho bảng `character_festival_claims` (`PRIMARY KEY (playerid, festival_id, period_key, claim_type)`) lưu vết nhận thưởng/hiển thị theo đợt (`season-<seq>`) và theo ngày (`YYYY-MM-DD`).
- [ ] Trong luồng hoàn tất đăng nhập, phát `S→C 0x17 0x39` cho tất cả *Lễ hội Theo mùa* đang `Active` và chọn tối đa `1` lễ hội đủ điều kiện (có khóa `Festival.Dat` duy nhất hợp lệ và chưa có claim `lore_shown` cho `season-<seq>`) để gửi `S→C 0x24 0x19` rồi lưu vết vào `character_festival_claims`.
- [ ] Xác minh hành vi đa lần đăng nhập & chuyển đợt (Positive + Negative Tests): relog nhiều lần trong cùng `season-<seq>` không bật lại popup `24 19`; khi nhiều lễ hội hợp lệ cùng `Active`, mỗi lần login chỉ gửi tối đa `1` khung `24 19`; khi Admin `Stop` rồi `Start` lại (`season_seq` tăng), người chơi đăng nhập sẽ nhận lại popup `24 19` một lần cho đợt mới.
- [ ] Xây dựng `FestivalDatEncoder` (chuẩn `0x7EF` byte/record, mã hóa XOR hai khóa đối xứng `keyA`/`keyB`, tối đa `0x16E = 366` bản ghi) cùng CLI helper xuất file `Festival.Dat` patch khóa `1..=7` từ `Data/Festival.Dat` gốc và hỗ trợ cấu hình thêm bản ghi `8..=11` (*Thất Tịch*, *Trung Thu*, *Halloween*, *Giáng Sinh*), giữ nguyên file `Data/Festival.Dat` gốc trên đĩa.
- [ ] Kiểm chứng round-trip `FestivalDatEncoder::encode` → `FestivalDatLoader::load` bảo toàn toàn bộ tiêu đề và hai đoạn văn bản VISCII 1.1 của `7` (và `11`) bản ghi với khóa duy nhất `1..=7` (và `1..=11`), không còn báo va chạm khóa.
