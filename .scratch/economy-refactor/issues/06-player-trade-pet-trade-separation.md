# 06: Player-to-Player Trade & Pet Trade Separation

**What to build:** Hoàn thiện tính năng giao dịch trực tiếp 1-1 giữa hai người chơi qua Opcode `0x19`: Tách biệt hoàn toàn luồng giao dịch vật phẩm (`19 01 <partner_id: u32 LE>`) và luồng giao dịch thú nuôi (`19 0A <pet_id: u32 LE>`) vốn đang bị gộp chung `1 | 10` trong `trade_storage.rs`. Bổ sung gói tin hủy/timeout giao dịch `17 3B` để đóng form giao dịch an toàn trên client của cả hai bên khi một bên hủy hoặc mất kết nối, tránh tình trạng treo giao diện hoặc khóa phiên đồ.

**Blocked by:** 02 (Currency Model & Bank Gold Sync at Login), 04 (Storage & Warehouse Item Transfers).

**Status:** completed

- [x] Trong `src/server/handlers/trade_storage.rs`, tách nhánh xử lý `ctx.sub == 1` (`19 01` Item Trade) và `ctx.sub == 10` (`19 0A` Pet Trade), không xử lý chung một luồng.
- [x] Xử lý quy trình giao dịch vật phẩm 2 pha (2-phase commit): Mời giao dịch (`19 01`), Đặt đồ & tiền (`19 03`), Khóa giao dịch (`19 04`), Xác nhận đồng ý (`19 05`).
- [x] Thực hiện hoán đổi vật phẩm và tiền tệ nguyên tử giữa 2 người chơi qua transaction SQLite kép, đảm bảo an toàn dữ liệu và chống duplication bug.
- [x] Khi một trong hai bên bấm nút Hủy hoặc bị timeout, server phát gói tin `F4440200173B` tới cả hai client để giải phóng trạng thái giao dịch và đóng form giao diện sạch sẽ.
- [x] Trả về các mã toast giao dịch tương ứng theo bảng đặc tả `19 02 <code>` (ví dụ: đối phương đang bận, khoảng cách quá xa, túi đồ không đủ chỗ).
