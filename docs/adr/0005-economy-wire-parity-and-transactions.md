# ADR 0005 — Chuẩn hóa Giao thức Kinh tế & Cơ chế Giao dịch Nguyên tử (Wire Parity & Atomic Transactions)

- **Ngày**: 2026-10-23
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn grilling)

---

## Bối cảnh

1. Quá trình dịch ngược mã máy thân Client TS Online (`aLogin.exe` Borland Delphi Win32, độ tin cậy Mức A 98.5% tại `.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md` và `.scratch/client-pseudo-op-code/rust_server_plan/01-container-mapping.md`) đã bộc lộ nhiều sai lệch nghiêm trọng giữa Client và Server Rust `ts_dream`.
2. Các sai lệch này bao gồm:
   - Frame mở sạp C→S & S→C `17 1E` bị parse sai thứ tự byte (client gửi 100 byte items trước rồi mới đến tên bảng hiệu).
   - Frame giao dịch P2P C→S `19 02` bị parse sai cấu trúc cặp `(slot, qty)` và S→C gửi nhầm frame `17 3B` (Cắt hàng) khi hủy giao dịch thay vì `19 04`.
   - Tiền tệ `1A 04` bị gửi kèm 4 byte `shop_point` thừa làm client Delphi bỏ qua và không nhận được Điểm.
   - Lạm dụng frame `02 0B` làm thông báo lỗi đỏ (kích hoạt form tạo Quân Đoàn `TFCreateOrganize` trên client).
   - Ngân hàng thiếu frame `1D 07 / 08` mở khóa nút bấm UI sau khi nộp/rút.
3. Để đảm bảo giao tiếp giữa Client và Server diễn ra trơn tru nhất, triệt tiêu nguy cơ sập client hoặc hack/dupe đồ, các quyết định kiến trúc dưới đây được xác lập và cam kết thực thi.

---

## Quyết định Thiết kế

### 1. Cơ chế Chống Race Condition & Khóa nguyên tử (Two-Phase State Lock)
- Áp dụng mô hình **Two-Phase State Lock** cho mọi giao dịch tương tác giữa 2 bên (Giao dịch P2P `0x19`, Mua hàng từ quầy sạp người chơi `17 21`):
  - **Phase 1 (RAM Lock)**: Khóa đồng thời `RwLock` của cả 2 Session (hoặc Seller + Buyer) theo thứ tự `player_id` tăng dần (`min(p1, p2)` khóa trước, `max(p1, p2)` khóa sau) để triệt tiêu hoàn toàn nguy cơ deadlock RAM. Kiểm tra toàn bộ điều kiện tồn kho và tiền tệ trong RAM. Nếu hợp lệ, khấu trừ tạm thời trên RAM.
  - **Phase 2 (SQLite Write Transaction)**: Mở 1 transaction độc quyền trên `write_pool` của SQLite (`max_connections = 1`). Thực hiện cập nhật số dư tiền tệ và hoán chuyển vật phẩm nguyên tử. Nếu commit DB thành công, trả về các frame S→C xác nhận cho cả 2 bên. Nếu commit DB thất bại, rollback RAM state về trạng thái cũ.

### 2. Kênh Thông báo Lỗi & Triệt tiêu `02 0B`
- Tuyệt đối loại bỏ việc lạm dụng S→C `02 0B` làm kênh "Red Message Toast".
- Mọi lỗi nghiệp vụ phải được định tuyến về đúng mã phản hồi gốc của Opcode đó:
  - Lỗi Shop NPC: Trả về S→C `1B 01 <code>` hoặc `1B 02 <code>` (mã `0x01` thiếu tiền, `0x02` đầy túi, `0x03` số lượng đầy, `0x04` gián đoạn, `0x06` đầy kim; tránh mã `0x05`).
  - Lỗi Tiền trang: Trả về S→C `1D 03` (không đủ sức chứa) kèm `1D 07/08` mở khóa nút bấm.
  - Lỗi Giao dịch P2P: Trả về S→C `19 02 01` (đối phương offline) hoặc `19 02 02` (đối phương đang bận).
- Đối với các thông báo hệ thống tự do (không có opcode phản hồi riêng), sử dụng kênh Chat Hệ thống chuẩn `02 01 <text>` hoặc Server Announcement/Alert `0x00 <sub>`.

### 3. Quy tắc Kiểm tra Ô trống khi Mua Shop NPC Số lượng lớn (Tới 1.250 món)
- Client Delphi cho phép mua tới 1.250 món (`count: u16 LE`). Sức chứa tối đa của 1 ô túi đồ là 50 món (`MAX_STACK = 50`).
- Áp dụng nguyên tắc **All-or-Nothing**:
  - Server tính trước số ô trống cần thiết: `slots_needed = (count + 49) / 50`.
  - Nếu số ô trống trong túi đồ `< slots_needed` hoặc tổng kim `< total_cost`: Từ chối toàn bộ giao dịch, hoàn tiền nguyên tử và gửi mã lỗi `NPC_SHOP_BUY_BAG_FULL` (`1B 01 02`).
  - Tuyệt đối không áp dụng Partial Buy (mua một phần) để tránh lệch số dư hiển thị giữa client và server.

### 4. Chuẩn hóa Kiến trúc 5 Container Vật phẩm
Hệ thống vật phẩm được chuẩn hóa 1-1 theo 5 container đã được xác minh qua decompile binary `aLogin.exe`:

| Container | Tên DB | `storagetype` | Sức chứa | Opcode Wire Dump | Cơ chế Cập nhật & Xóa |
|---|---|:---:|:---:|:---:|---|
| **Hòm đồ** (Túi chính) | `homdo` | `1` | 25 ô (`1..=25`) | `17 05` (có clear pass) | Nhận `17 06` (14B không slot), Refresh `17 08`, Xóa `17 09` |
| **Tiền trang** (Rương kho lớn) | `tientrang` | `2` | 50 ô (`1..=50`) | `1E 01` (không clear pass) | Mass-clear `1E 08` trước khi dump; Thêm `1E 02` (11B data, auto tìm ô trống), Rút/xóa `1E 05 <slot> <qty>`, Bulk sync `1E 01` |
| **Túi đeo** (Ba Đậu Yêu Balo) | `tuideo` | `4` | 25 ô (`1..=25`) | `17 2F` (`player+0xF0C`) | Cập nhật `17 30`, Xóa `17 31 <slot>` |
| **Lưu Lãng** (Ba Đậu Yêu Lưu Lãng / Xe ngựa) | `luulang` | `16` | **10 ô (`1..=10`)** | `17 66` (`player+0xF74`) | Cất C→S `17 33`, Rút C→S `17 34`; S→C Cập nhật `17 67`, Xóa `17 68 <slot>`. Cấm vượt quá 10 ô |
| **Trang bị** (Đang mặc) | `trangbi` | `8` | 6 ô (`1..=6`) | `17 0B` | Stride **10 byte**, KHÔNG có byte slot, sắp xếp theo slot tăng dần |

### 5. Chuẩn hóa Hủy Giao dịch P2P
- Khi giao dịch P2P bị hủy (do người chơi bấm nút Hủy `19 03 00`, timeout hoặc ngắt kết nối):
  - Server gửi frame S→C `19 04` (`F4 44 02 00 19 04`) đóng form giao dịch an toàn.
  - Server gửi toast lỗi S→C `19 02 03` thông báo cho đối tác ("Đối phương đã hủy bỏ giao dịch").
  - Xóa bỏ việc gửi frame `17 3B` (vốn là chức năng Cắt hàng `TFCutItem` của client).

---

## Hệ quả & Kế hoạch Thực thi

- **Wire Parity**: Client Delphi giao tiếp chuẩn xác 100% không còn bị crash form sạp, đơ nút ngân hàng, giật popup tạo quân đoàn hoặc mất hiển thị Điểm.
- **Tính toàn vẹn Dữ liệu**: Không còn rủi ro race condition khi nhiều người cùng mua sạp hoặc đồng thời chốt trade P2P.
- **Suite Test**: Cập nhật lại toàn bộ các assertion trong [`tests/economy_wire_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs) theo các quy chuẩn wire mới ở Phase 3.
