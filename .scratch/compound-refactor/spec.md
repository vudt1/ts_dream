# Spec: Hệ Thống Hợp Thành & Cường Hóa Khảm Lông (Compound & Feather Socketing System)

Status: `ready-for-agent`

---

## Problem Statement

Người chơi TS Online hiện không thể sử dụng tính năng Hợp thành (Mix đồ / Chế tạo) cũng như tính năng Cường hóa Khảm Lông (Feather Socketing) vào trang bị. 

Cụ thể:
1. **Hợp thành cơ bản qua `form_compound`**: Khi người chơi đặt các nguyên liệu (khoáng thạch, dược phẩm, trang bị phôi) vào 3 ô hợp thành và nhấn xác nhận, yêu cầu gửi lên qua Opcode `0x17 Subcode 0x39` bị server bỏ qua hoặc chưa được xử lý.
2. **Khảm Lông & Khảm Tâm (`Socketing`)**: Khi người chơi muốn ép Lông vũ (Đồng Thoại Vũ, Lông Vũ Địa/Thủy/Hỏa/Phong cấp 1..10) hoặc Tâm vào trang bị để tăng thuộc tính nguyên tố (`element`) và cấp cường hóa (`element_value` +1..+10), server chưa có cơ chế tiếp nhận, tính toán tỷ lệ thành công/thất bại và cập nhật chỉ số thuộc tính động của trang bị.
3. **Form Lựa chọn Lông vũ (`TFT_SelectFeatherForm`)**: Khi người chơi mở túi Lông tự chọn hoặc nhận thưởng sự kiện, client hiển thị giao diện chọn 1 trong 4 hệ (Địa, Thủy, Hỏa, Phong) và gửi gói tin Opcode `0x23 Subcode 0x03` lên server, nhưng server chưa có handler xử lý gói tin này để trao đúng viên lông tương ứng.
4. **Sai lệch dữ liệu nhị phân**:
   - Hai trường dữ liệu cốt tử trong `Data/Item.Dat` (`chunk[46]` là `compound_material` - mã chất liệu 1..37, và `chunk[47]` là `compound_rank` - bậc phẩm chất 1..10+) bị bỏ sót và gán nhãn nhầm thành các trường vô dụng, khiến server hoàn toàn không có dữ liệu để tính toán công thức hợp thành.
   - File công thức `Data/Compound.Dat` bị đọc nhầm cả phần header metadata 8 byte ở Slot 0, làm sai lệch cấu trúc dữ liệu và thiếu cơ chế lập chỉ mục tra cứu hiệu năng cao.
   - Opcode `0x2B` bị gán nhãn nhầm là `OP_COMPOUND` (thực chất `0x2B` là `OP_GUILD_ALLIANCE`), trong khi luồng hợp thành thực tế đi qua `OP_ITEM` (`0x17`).

## Solution

Xây dựng một Động cơ Chế tác & Cường hóa Toàn diện (**Crafting & Socketing Engine**) với thẩm quyền 100% thuộc Game Server (Server Authoritative) theo đúng đặc tả dịch ngược mã máy Delphi client (`aLogin.exe`) và các quyết định kiến trúc tại ADR 0007 & ADR 0005:

1. **Hiệu chỉnh Tầng Dữ liệu**:
   - Nạp chính xác `compound_material` (`ratA`) và `compound_rank` (`ratB`) vào danh mục vật phẩm `ItemDef`.
   - Bỏ qua header 8 byte và lập chỉ mục mảng công thức `Compound.Dat` theo khóa số nguyên `u32` để tìm kiếm nhị phân $O(\log N)$.
2. **Phân hệ Hợp thành Căn bản (Basic Compound Engine)**: Hiện thực hóa chính xác giải thuật Delphi `FUN_0079A890`:
   - Thẩm định đầu vào (tối thiểu 2 món, dải ID 10001..50000, số lượng $\le 50$, cấm loại 20..22).
   - Banker's Rounding (IEEE-754 Round to Even) tính bậc phẩm chất trung bình $R_{\text{avg}}$.
   - Quay số ngẫu nhiên cấp phẩm chất mục tiêu $R_{\text{target}} \in [1, R_{\text{avg}} + 1]$ (kẹp trần 10).
   - Ghép chuỗi thập phân tạo khóa `search_key` và kiểm tra điều kiện bậc tỷ lệ `input_deg`.
   - Cơ chế Fallback cấp 1 khi trượt công thức: ép $R_{\text{target}} = 1$ và trả về vật phẩm rác cấp 1 cùng loại của nguyên liệu đầu vào (không bao giờ mất trắng đồ).
   - Quy tắc số lượng trang bị: Ép `out_qty = 1` cho mọi trang bị vũ khí/giáp (`kind 1..14`), và cho phép hợp thành hàng loạt theo `min_qty` đối với vật phẩm xếp chồng (`kind >= 15`).
3. **Phân hệ Cường hóa Khảm Lông (Feather Socketing Engine)**:
   - Hỗ trợ thao tác khảm Lông vũ / Tâm vào trang bị thuộc Hòm đồ chính (`homdo`).
   - Kích hoạt hệ nguyên tố ban đầu (`element`: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong) và tăng dần cấp cường hóa (`element_value`: 1..10).
   - Bảng tỷ lệ thành công/thất bại chuẩn theo từng mốc cấp độ (+1..+3: 100%; +4: 80%; +5: 60%; +6: 40%; +7: 25%; +8: 15%; +9: 10%; +10: 5%).
   - Xử lý khi thất bại: Giữ nguyên cấp hoặc tụt 1 cấp nếu không có bùa hộ mệnh.
   - Cho phép khảm đè hệ nguyên tố khác (reset cấp cường hóa về +1).
   - Tiêu hao nguyên liệu Lông vũ, cập nhật đồng bộ các trường `element` và `elementvalue` trong SQLite `inventories` và RAM `Session`.
4. **Phân hệ Chọn Lông vũ (`TFT_SelectFeatherForm`)**:
   - Tiếp nhận gói tin Opcode `0x23 Subcode 0x03` khi người chơi chọn Lông/Bảo vật trên form giao diện.
   - Thẩm định tính hợp lệ của tham số hệ chọn lựa (`1=Địa, 2=Thủy, 3=Hỏa, 4=Phong`).
   - Cấp phát đúng Lông vũ tương ứng (`24021` Địa, `24022` Thủy, `24023` Hỏa, `24024` Phong) vào slot trống của túi đồ.
5. **Điều phối Giao thức Mạng & Concurrency**:
   - Áp dụng Two-Phase State Lock và SQLite write transaction nguyên tử cho cả 2 luồng Hợp thành (`0x17 Sub 0x39`) và Khảm Lông (`0x23 Sub 0x03` hoặc `0x17`).
   - Giới hạn phạm vi ô chứa duy nhất trong Hòm đồ chính (`homdo`, slot 1..25).
   - Kiểm tra ô trống trước khi trừ đồ (Probe-First All-or-Nothing, yêu cầu `free_slots >= 1`).
   - Gửi các gói tin wire chuẩn: Âm thanh rèn đúc `17 0F` (`WA0014.wav`), cập nhật slot tiêu hao `17 09`, cấp phát thành phẩm `17 06`, và thông báo hệ thống qua kênh `sys_msg_frame` (`020102`).

## User Stories

### Nhóm Hợp thành Căn bản (Compound Stories)
1. Là một người chơi, tôi muốn đặt 2 hoặc 3 loại khoáng thạch vào bảng hợp thành, để tôi có thể luyện ra khoáng thạch cấp cao hơn phục vụ cho việc chế tạo.
2. Là một người chơi, tôi muốn đặt 10 viên sắt và 10 viên đồng cùng lúc, để hệ thống tự động hợp thành hàng loạt ra 10 sản phẩm mà tôi không phải bấm từng lần một.
3. Là một người chơi, tôi muốn ghép khoáng thạch điểm cao cùng trang bị phôi cấp thấp, để nâng cấp ra trang bị có đẳng cấp cao hơn (ví dụ cấp 45, 55, 65, 80) theo đúng công thức kinh nghiệm trong game.
4. Là một người chơi, khi tôi lỡ tay đưa vào 2 nguyên liệu không có công thức trong sách, tôi muốn hệ thống vẫn trả về một món trang bị rác cấp 1 cùng loại nguyên liệu thay vì làm mất trắng đồ của tôi.
5. Là một người chơi, khi tôi đưa vào chỉ 1 vật phẩm và bấm hợp thành, tôi muốn hệ thống từ chối và báo thông báo yêu cầu ít nhất 2 vật phẩm để tôi không bị mất đồ oan.
6. Là một người chơi, khi tôi cố tình đưa vào các vật phẩm không được phép (như tiền tệ, vé đặc biệt thuộc loại 20, 21, 22), tôi muốn hệ thống từ chối hợp thành để bảo vệ các tài sản này.
7. Là một người chơi, khi tôi cố tình đặt một ô có số lượng vượt quá 50, tôi muốn hệ thống từ chối để tuân thủ giới hạn ngăn xếp của game.
8. Là một người chơi, khi túi đồ của tôi đã đầy 25/25 ô, tôi muốn hệ thống báo lỗi không đủ chỗ chứa ngay từ đầu để tôi dọn túi trước khi thực hiện.
9. Là một người chơi, khi hợp thành thành công ra một món vũ khí hoặc nón giáp, tôi muốn thành phẩm sinh ra luôn có số lượng là 1 bất kể nguyên liệu đưa vào có số lượng là bao nhiêu.
10. Là một người chơi, khi hợp thành hoàn tất, tôi muốn nghe thấy âm thanh rèn đúc đặc trưng (`WA0014.wav`) để có trải nghiệm chân thực như bản game gốc.

### Nhóm Khảm Lông & Cường hóa Trang bị (Socketing Stories)
11. Là một người chơi, tôi muốn dùng Lông vũ nguyên tố (Địa, Thủy, Hỏa, Phong) ép vào trang bị trắng, để trang bị nhận được hệ nguyên tố tương ứng và tăng cấp cường hóa lên +1.
12. Là một người chơi, tôi muốn dùng Lông vũ cấp cao hơn (Đồng Thoại Vũ C2..C10) ép vào trang bị đã có hệ, để nâng cấp thuộc tính trang bị lên các mốc cao hơn từ +2 đến +10.
13. Là một người chơi, khi tôi mở form chọn lông `TFT_SelectFeatherForm`, tôi muốn có thể bấm chọn 1 trong 4 hệ (Địa/Thủy/Hỏa/Phong) và nhận đúng viên Lông vũ mong muốn sau khi bấm xác nhận.
14. Là một người chơi, khi tôi chưa chọn hệ nào trên form `TFT_SelectFeatherForm` mà đã bấm xác nhận, tôi muốn hệ thống hiện thông báo nhắc nhở "Hãy chọn lựa một trong các loại bảo vật" để tôi không bị mất lượt oan.
15. Là một người chơi, khi khảm Lông thất bại ở các cấp cao (+4 trở lên), tôi muốn hệ thống thông báo rõ ràng về việc giữ nguyên hay tụt cấp cường hóa của trang bị.
16. Là một người chơi, khi khảm Lông thành công, tôi muốn thấy thuộc tính nguyên tố và số cộng (+1..+10) hiển thị đầy đủ trên bảng thông tin trang bị trong túi đồ.
17. Là một người chơi, tôi muốn có thể khảm đè Lông hệ khác vào trang bị đã có hệ cũ để chuyển đổi hệ nguyên tố của trang bị, chấp nhận reset cấp cường hóa về +1.

### Nhóm Quản trị & Hệ thống (System & Integrity Stories)
18. Là một quản trị viên hệ thống, tôi muốn toàn bộ thao tác trừ nguyên liệu (Lông vũ/khoáng) và cập nhật thuộc tính trang bị diễn ra trong một giao dịch SQLite nguyên tử, để không bao giờ xảy ra tình trạng dupe đồ hoặc mất đồ khi rớt mạng.
19. Là một lập trình viên kiểm thử, tôi muốn có thể nạp một seed số ngẫu nhiên cố định vào động cơ hợp thành và khảm lông, để viết các bài test tự động deterministic đối chiếu chính xác kết quả.
20. Là một lập trình viên hệ thống, tôi muốn mã nguồn được tách biệt thành package `src/crafting/` chuyên biệt (`basic` và `socketing`), để kiểm thử và bảo trì độc lập mà không cần dựng cụm TCP server.

## Implementation Decisions

### 1. Kiến trúc Mô-đun Sâu (Deep Module) `src/crafting/`
- Tách biệt toàn bộ logic nghiệp vụ chế tác vào `src/crafting/`:
  - `src/crafting/basic/`: Chứa `algorithm.rs`, `catalog.rs`, `validator.rs` phục vụ hợp thành căn bản từ `Compound.Dat`.
  - `src/crafting/socketing/`: Chứa `feather.rs`, `rate_table.rs` phục vụ tính toán khảm Lông vũ, Tâm và nâng cấp thuộc tính động (`element`, `element_value`).
  - `src/crafting/scripted/`: Cầu nối với Eve Script Engine.
- Network Handlers:
  - `src/server/handlers/crafting.rs`: Tiếp nhận Opcode `0x17 Sub 0x39` (Hợp thành) và Opcode `0x23 Sub 0x03` (Chọn Lông `TFT_SelectFeatherForm`).
  - Sử dụng chung mô hình Two-Phase State Lock và SQLite write pool độc quyền.

### 2. Thuật toán Cường hóa Khảm Lông (Socketing Logic)
- **Kiểm tra hợp lệ**:
  - Trang bị thuộc `homdo` (slot 1..25), loại `kind ∈ 1..14`.
  - Lông vũ có ID hợp lệ (`24021..24024` Lông hệ cấp 1, hoặc `24031..24039` Đồng Thoại Vũ cấp 1..9, hoặc Lông cấp 10).
- **Quy tắc Thăng cấp & Đổi hệ**:
  - Nếu trang bị chưa có hệ (`item.element == 0`): Nhận hệ của Lông vũ, `element_value = 1` (Tỷ lệ thành công 100%).
  - Nếu trang bị cùng hệ (`item.element == feather.element`):
    - Yêu cầu cấp Lông $K == \text{item.element\_value} + 1$.
    - Quay số tỷ lệ thành công theo bảng:
      | Cấp đích | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 | +9 | +10 |
      |---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
      | **Tỷ lệ** | 100% | 100% | 100% | 80% | 60% | 40% | 25% | 15% | 10% | 5% |
    - Nếu thành công: `item.element_value += 1`.
    - Nếu thất bại: Nếu cấp $\le 3$, giữ nguyên; nếu cấp $\ge 4$, giảm 1 cấp (`item.element_value -= 1`).
  - Nếu trang bị khác hệ: Cho phép đổi hệ sang Lông mới, reset `element_value = 1` (Tỷ lệ 100%).
- Tiêu hao 1 viên Lông vũ trong mọi trường hợp.

### 3. Giao thức Mạng cho `TFT_SelectFeatherForm` (Opcode `0x23 Sub 0x03`)
- **Khung C→S**:
  - `Opcode 0x23`, `Subcode 0x03`.
  - Payload: Chuỗi tham số 1, Chuỗi tham số 2, Byte hệ chọn lựa (`elem_choice`: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong), Byte tùy chọn phụ (`sub_choice`).
- **Xử lý Server**:
  - Kiểm tra `elem_choice ∈ 1..=4`. Nếu bằng 0, từ chối giao dịch.
  - Kiểm tra túi đồ `homdo` còn ít nhất 1 slot trống.
  - Mở SQLite transaction nguyên tử, cấp phát Lông vũ tương ứng (`24021` Địa, `24022` Thủy, `24023` Hỏa, `24024` Phong) vào slot trống đầu tiên.
  - Gửi frame thêm đồ `17 06`, âm thanh `17 0F`, và thông báo chúc mừng `02 01 02`.

### 4. Tầng Dữ liệu & Giao dịch Cơ sở Dữ liệu
- Sửa `src/data/loaders/item.rs`: Nạp `chunk[46]` làm `compound_material`, `chunk[47]` làm `compound_rank`.
- Sửa `src/data/loaders/compound.rs`: Bỏ qua 8 byte Slot 0 header, nạp 1.393 bản ghi và lập chỉ mục khóa `u32`.
- Cập nhật trực tiếp các cột `element` và `elementvalue` của bảng `inventories` trong SQLite.

## Testing Decisions

### 1. Điểm Nối Kiểm Thử (Testing Seams)
- **Seam 1 (Primary - Network Wire & Database Integration)**:
  - Bơm gói tin C→S `0x17 Sub 0x39` (Hợp thành) và `0x23 Sub 0x03` (Chọn Lông) vào `PacketContext`.
  - Kiểm tra các frame S→C phát ra (`17 0F`, `17 09`, `17 06`, `02 01 02`).
  - Kiểm tra tính toàn vẹn của bảng `inventories` trong SQLite: số lượng trừ đúng, item mới xuất hiện đúng slot, trường `element` và `elementvalue` được cập nhật chính xác.
- **Seam 2 (Secondary - Pure Domain Logic)**:
  - Kiểm thử `BasicCompoundEngine` với seed cố định: kiểm tra Banker's Rounding, ghép khóa `search_key`, quy tắc ép số lượng trang bị `kind 1..14 = 1`, và luồng Fallback cấp 1.
  - Kiểm thử `SocketingEngine` với seed cố định: kiểm tra tỷ lệ thành công/thất bại, tăng cấp, tụt cấp khi xịt, và đổi hệ nguyên tố.

### 2. Tiền lệ Kiểm thử (Prior Art)
- Kế thừa mô hình kiểm thử từ `tests/economy_wire_test.rs` và `tests/trade_wire_test.rs`.
- Tất cả các bài test mới đặt tại `tests/compound_test.rs` và `tests/socketing_test.rs` ở repo root (không dùng inline `#[cfg(test)]` trong `src/`).

## Out of Scope

1. Chế tạo/Đổi đồ theo kịch bản hội thoại NPC thông qua Eve Script Engine (`eve.emg`).
2. Bảng ma trận khắc trang bị phụ trợ $15 \times 15$ trong `Compound2.Dat`.
3. Công thức tính sát thương chiến đấu trong `Formula.Dat`.
4. Hệ thống đục lỗ trang bị (Gem Sockets) cho ngọc khảm phụ (`stoneattr`, `stonelevel`).

## Further Notes

- Danh sách ID Lông vũ nguyên tố: `24021` (Địa), `24022` (Thủy), `24023` (Hỏa), `24024` (Phong).
- Danh sách ID Đồng Thoại Vũ cường hóa: `24031..24039` (Cấp 1 đến Cấp 9).
- Toast tiếng Việt khi chưa chọn hệ trên `TFT_SelectFeatherForm`: *"Hãy chọn lựa một trong các loại bảo vật"*.

