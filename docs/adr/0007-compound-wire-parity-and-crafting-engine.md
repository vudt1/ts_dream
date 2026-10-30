# ADR 0007 — Chuẩn hóa Giao thức Hợp thành & Kiến trúc Động cơ Chế tác (Compound Wire Parity & Crafting Engine)

- **Ngày**: 2026-10-30
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn grilling)

---

## 1. Bối cảnh (Context)

1. Quá trình dịch ngược mã máy thân Client TS Online (`aLogin.exe` Borland Delphi Win32 x86) tại `.scratch/compound-refactor/CLIENT_COMPOUND_SPEC.md` và kiểm tra đối chiếu dữ liệu `Data/Compound.Dat`, `Data/Item.Dat`, `Data/Formula.Dat` đã làm sáng tỏ cơ chế Hợp thành (Compound / Mix đồ):
   - **Form `0x0079EE08` là Developer Simulator Ngoại tuyến**: Chứa thuật toán đầy đủ nhưng nút `"合成"` (`0x0079F3C8`) chỉ tính toán nội bộ và in text ra màn hình, **không gửi bất kỳ gói tin mạng nào**.
   - **Gameplay trực tuyến dùng `form_compound`**: Khởi tạo tại `0x00532350`, nút `btn_OK` (`FUN_00532904`) kiểm tra trạng thái nhân vật (`player+0x63a == 0x29cd`), kiểm tra cờ cấm chế tạo `0x3c`, kiểm tra ô trống túi đồ (`FUN_00768D38`), rồi gửi gói tin yêu cầu hợp thành lên server qua **Main Opcode `0x17` Subcode `0x39`** (`0x00785045`).
2. Khi đối chiếu với source code server Rust `ts_dream`, phát hiện 4 sai lệch kiến trúc nghiêm trọng:
   - **Bỏ sót trường dữ liệu cốt tử trong `src/data/loaders/item.rs`**: Code hiện tại đặt tên `chunk[46]` là `_contribute` và `chunk[47]` là `_sell_price_rate` rồi bỏ qua không nạp vào struct `ItemDef`. Đồng thời đọc nhầm `chunk[82]` làm `material`. Thực tế mã máy xác nhận: `chunk[46]` là `compound_material` (`ratA`, 1..37) dùng tạo khóa `search_key` tra `Compound.Dat`, và `chunk[47]` là `compound_rank` (`ratB`, 1..10+) dùng tính bậc phẩm chất trung bình $R_{\text{avg}}$.
   - **Lỗi đọc Slot 0 trong `src/data/loaders/compound.rs`**: Slot 0 là header metadata 8 byte (`u16 = 2`). Code đọc cả header làm công thức khiến lệch dữ liệu (chỉ có 1.393 công thức hợp lệ). Đồng thời thiếu mảng sắp xếp khóa `u32` để tìm kiếm nhị phân $O(\log N)$.
   - **Nhầm lẫn Opcode trong `dispatcher.rs`**: Gán nhãn `OP_COMPOUND = 0x2B` là sai lệch (Opcode `0x2B` thực chất là `OP_GUILD_ALLIANCE`). Lệnh hợp thành vật phẩm nằm tại `0x17 Sub 0x39`.
   - **Hiểu nhầm về `Formula.Dat`**: `Formula.Dat` (235 bytes) chứa các tham số sát thương chiến đấu của `TCalculator`, hoàn toàn không phải công thức chế tạo đồ.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Thẩm quyền 100% thuộc Game Server (Authoritative Server Parity)
- Server là nơi duy nhất quyết định kết quả hợp thành, thẩm định sở hữu túi đồ, thực thi RNG và cập nhật cơ sở dữ liệu SQLite trong một giao dịch nguyên tử (Atomic Database Transaction). Client chỉ là giao diện thu thập thao tác và hiển thị kết quả.

### 2.2. Chuẩn hóa Wire Protocol cho Opcode `0x17 Sub 0x39`
- **Gói tin Yêu cầu C→S**:
  - Opcode `0x17`, Subcode `0x39`.
  - Payload: Danh sách 3 cặp `(bag_slot: u8, qty: u8)`.
  - **Phạm vi ô chứa (Strict Homdo Parity)**: Slot gửi lên **bắt buộc và duy nhất** thuộc `homdo` (Túi chính, slot `1..=25`). Từ chối mọi slot ngoài phạm vi hoặc tham chiếu sang các container khác (`tuideo`, `luulang`).
- **Gói tin Phản hồi S→C**:
  - Âm thanh hiệu ứng rèn đúc: `F4 44 02 00 17 0F` (kích hoạt phát file `sound\WA0014.wav`).
  - Gói trừ/xóa slot nguyên liệu tiêu hao: `17 09 <slot> <qty_left>` (hoặc xóa ô nếu hết).
  - Gói cấp phát thành phẩm mới: `17 06 <slot> <item_id> <qty> ...`.
  - Thông báo hệ thống qua kênh chuẩn: `02 01 02 <len> <nội dung VISCII>` (`sys_msg_frame`).

### 2.3. Quy tắc Kiểm tra Ô trống (Probe-First All-or-Nothing)
- Đồng bộ tuyệt đối với logic Client UI (`FUN_00768D38`): Server kiểm tra `free_slots >= 1` ngay đầu handler trước khi tiêu hao bất kỳ nguyên liệu nào.
- Nếu túi đồ đang đầy `25/25` ô trống, Server lập tức từ chối giao dịch, gửi toast lỗi hệ thống, ngay cả khi các nguyên liệu đưa vào sẽ bị tiêu hao hết 100%.

### 2.4. Thuật toán Delphi Parity 1:1 (`FUN_0079A890`) & Cơ chế Fallback Cấp 1
- **Tính bậc phẩm chất mục tiêu**: Sử dụng Banker's Rounding (IEEE-754 Round to Even) tính $R_{\text{avg}} = \max(1, \text{ROUND}(\sum \text{ratB} / N))$. Quay số ngẫu nhiên $R_{\text{target}} = \text{Random}(R_{\text{avg}} + 1) + 1$, kẹp trần tại $10$.
- **Tạo khóa tra cứu `search_key`**: Sắp xếp mảng `ratA` tăng dần, ghép chuỗi thập phân (không đệm số 0) và đổi thành `u32`.
- **Cơ chế Fallback khi trượt công thức**: Nếu không tìm thấy công thức nào trong `Compound.Dat`:
  - Chọn ngẫu nhiên loại trang bị (`kind`) của 1 trong các nguyên liệu hợp lệ đầu vào.
  - **Ép bậc phẩm chất mục tiêu về 1**: $R_{\text{target}} = 1$.
  - Quét `Item.dat` trả về trang bị rác cấp 1 cùng loại. Tuyệt đối không bao giờ báo lỗi thất bại hay làm mất trắng đồ của người chơi.
- **Ràng buộc số lượng trang bị**: Nếu thành phẩm thuộc loại trang bị không xếp chồng (`kind ∈ 1..14`), ép `out_qty = 1`. Nếu thuộc loại xếp chồng (`kind >= 15`), cho phép hợp thành hàng loạt theo `min_qty` (tối đa 50 món).

### 2.5. Tách biệt Mô-đun Sâu (Deep Module) `src/crafting/` & Mở rộng Khảm Lông (Socketing)
Tách biệt toàn bộ logic nghiệp vụ chế tác vào `src/crafting/` bao gồm cả 2 phân hệ:
- `src/crafting/basic/`: Chứa `algorithm.rs` (toán Banker's rounding, RNG, fallback), `catalog.rs` (bảng `CompoundCatalog` sắp xếp khóa `u32` tìm kiếm nhị phân $O(\log N)$), `validator.rs` (thẩm định dải ID, loại cấm `20..22`).
- `src/crafting/socketing/`: 
  - Phân hệ Cường hóa Khảm Lông / Khảm Tâm: Quản lý việc ép Lông vũ (`24021..24024`, `24031..24039`) vào trang bị `homdo` (`kind 1..14`), kích hoạt hệ nguyên tố (`element`: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong) và thăng cấp thuộc tính động (`element_value`: +1..+10).
  - Tỷ lệ thăng cấp chuẩn: +1..+3 đạt 100%, +4..+10 giảm dần ($80\%, 60\%, 40\%, 25\%, 15\%, 10\%, 5\%$). Thất bại tụt 1 cấp nếu không có bùa hộ mệnh.
  - Phân hệ Chọn Lông vũ `TFT_SelectFeatherForm`: Tiếp nhận Opcode `0x23 Subcode 0x03` khi người chơi chọn 1 trong 4 hệ Lông/Bảo vật trên giao diện, cấp phát Lông vũ tương ứng vào túi đồ và phát gói tin đồng bộ `17 06` kèm SFX `17 0F`.
- `src/crafting/scripted/`: Cầu nối kịch bản chế tạo NPC qua Eve Script Engine (`eve.emg`).
- `src/server/handlers/crafting.rs`: Network coordinator chịu trách nhiệm áp dụng Two-Phase State Lock, quản lý transaction SQLite và đồng bộ wire packets cho cả `0x17 Sub 0x39` và `0x23 Sub 0x03`.

---

## 3. Hệ quả (Consequences)

- **Sửa triệt để rủi ro dữ liệu**: Nạp đúng `compound_material` (`chunk[46]`) và `compound_rank` (`chunk[47]`) vào `ItemDef`, cho phép thuật toán hợp thành hoạt động chính xác 100% như server gốc.
- **Tính toàn vẹn kinh tế**: Không xảy ra race condition hay dupe đồ nhờ Two-Phase State Lock và SQLite write transaction.
- **Kiểm thử độc lập (Testability)**: Toàn bộ thuật toán hợp thành có thể kiểm thử đơn vị độc lập tại `tests/compound_test.rs` mà không phụ thuộc vào hạ tầng mạng TCP.
