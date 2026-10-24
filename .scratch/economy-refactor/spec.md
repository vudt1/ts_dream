# Specification: Hoàn thiện & Chuẩn hóa Giao thức Kinh tế (Economy Wire Parity)

Status: ready-for-agent

---

## Problem Statement

Người chơi TS Online khi sử dụng client nguyên bản (`aLogin.exe` Borland Delphi) kết nối tới server Rust `ts_dream` gặp phải các lỗi gián đoạn nghiêm trọng và trải nghiệm không mượt mà trong hệ thống kinh tế:
1. Khi mở quầy hàng cá nhân (Stall), client bị lỗi hoặc danh mục hiển thị sai lệch hoàn toàn do server giải mã sai thứ tự các trường dữ liệu; sau khi mở sạp, form thiết lập không thể đóng lại được.
2. Khi giao dịch trực tiếp giữa 2 người chơi (P2P Trade), số lượng và vị trí vật phẩm bị xáo trộn do server đọc nhầm byte số lượng thành slot tiếp theo; khi hủy giao dịch, client nhận nhầm frame "Cắt hàng" (`17 3B`) thay vì đóng form giao dịch an toàn (`19 04`).
3. Khi nhận thông báo lỗi từ server, client thường xuyên bị bật popup "Tạo Quân Đoàn" không mong muốn do server lạm dụng opcode `02 0B` làm thông báo chữ đỏ; đồng thời người chơi không nhận được Điểm tích lũy (Shop Point) vì frame tiền tệ bị ghép sai định dạng.
4. Sau khi gửi hoặc rút tiền trong Tiền Trang, các nút bấm trên giao diện client bị tê liệt, không thể thao tác tiếp do thiếu frame mở khóa UI.
5. Việc mua số lượng lớn tại Shop NPC bị giới hạn nhân tạo ở mức 255 món dù client cho phép mua tới 1.250 món.

---

## Solution

Cải tiến toàn diện tầng xử lý kinh tế phía server Rust để đạt chuẩn **100% Wire Parity** với Client Delphi:
1. Viết lại bộ giải mã và đóng gói frame Mở sạp cá nhân (`17 1E`), đọc đúng 100 byte danh mục trước rồi mới đến tên bảng hiệu; phản hồi frame 3 byte chuẩn xác để client đóng form thiết lập và hiển thị sạp trên bản đồ.
2. Tái cấu trúc parser Giao dịch P2P (`0x19`), giải mã đúng cấu trúc 5 cặp `(slot, qty)` và 4 byte kim; xử lý hủy giao dịch chuẩn xác bằng frame đóng form `19 04` và toast thông báo `19 02 03`. Áp dụng cơ chế Two-Phase State Lock để chống race condition và dupe đồ tuyệt đối.
3. Chấm dứt hoàn toàn việc lạm dụng opcode `02 0B`. Mọi thông báo lỗi được trả về đúng mã kết quả của từng opcode hoặc kênh Chat Hệ thống.
4. Tách biệt hoàn toàn frame Kim (`1A 04`, 6 byte) và Điểm (`1A 03`, 6 byte); bổ sung các frame mở khóa nút bấm UI Tiền Trang (`1D 07 / 08`).
5. Nâng giới hạn mua Shop NPC lên 1.250 món theo nguyên tắc All-or-Nothing, tự động phân tách thành các stack 50 món trong túi đồ.
6. Bảo toàn và chuẩn hóa kiến trúc 5 Container vật phẩm (`homdo` 25 ô, `tientrang` 50 ô, `tuideo` 25 ô, `luulang` 10 ô, `trangbi` 6 ô 10-byte).

---

## User Stories

### Nhóm Shop NPC (NpcShop - Opcode 0x14 & 0x1B)
1. As a player, I want to talk to an NPC merchant and select the Buy menu, so that the purchase interface (`TInterForm`) opens cleanly without popping up the Sell interface on top of it.
2. As a player, I want to buy up to 1.250 items in a single transaction from an NPC shop, so that I can quickly stock up on consumables without clicking repeatedly.
3. As a player, I want the server to check whether my inventory has enough empty slots for all purchased stacks (each stack max 50 items) before deducting my gold, so that I never lose gold without receiving items.
4. As a player, I want to receive an exact "Bag is Full" error response (`1B 01 02`) when I lack inventory slots, so that my client UI shows the correct native error banner.
5. As a player, I want to receive an exact "Insufficient Gold" error response (`1B 01 01`) when I lack gold, so that I understand why my purchase failed.
6. As a player, I want to sell items from my bag (slots 1 to 25) with up to 50 items per action via the Sell interface (`TThingSaleForm`), so that my inventory stays organized.
7. As a player, I want to receive the sold value added to my gold and my bag slot updated immediately, so that my balance is always accurate.

### Nhóm Tiền tệ & Ngân hàng (Currency & Bank - Opcode 0x1A & 0x1D)
8. As a player logging into the game, I want my client to receive separate, dedicated balance updates for Bag Gold (`1A 04`, 6 bytes), Bank Gold (`1D 04`, 6 bytes), and Shop Points (`1A 03`, 6 bytes), so that all HUD counters reflect my real balance without data corruption.
9. As a player, I want to deposit gold from my bag into my bank account (`1D 02`), so that my currency is stored securely.
10. As a player, I want to withdraw gold from my bank account into my bag (`1D 01`), so that I have currency available for trading.
11. As a player, I want the server to send an unlock frame (`1D 07` or `1D 08`) after every bank transaction, so that my Deposit and Withdraw buttons do not become unresponsive.
12. As a player, I want the server to reject any deposit or withdrawal that would exceed the gold cap of 9.999.999, so that my currency counters never overflow or roll over to negative values.

### Nhóm Sạp hàng Cá nhân (Player Stall - Opcode 0x17 Subs 1E..22)
13. As a player, I want to set up a vendor stall with up to 10 items, their individual prices, and a shop signboard name, so that I can sell items to other players while remaining in the world.
14. As a player, I want the server to parse my 10 stall items and signboard name accurately, so that my stall displays the exact items and prices I specified.
15. As a player, I want to receive an immediate 3-byte confirmation frame (`17 1E 01`) upon setting up my stall, so that the setup dialog closes cleanly and my stall becomes active on the map.
16. As a browsing player, I want to click on another player's stall to inspect their catalog (`17 21`), so that I see their full inventory with correct item IDs, stack counts, and prices.
17. As a browsing player, I want to purchase an item from another player's stall by specifying its 1-based slot index, so that I receive the exact item and stack count I selected.
18. As a vendor, I want the listing to be removed or stack count decremented atomically when an item is purchased, so that multiple buyers cannot purchase the same item concurrently.

### Nhóm Giao dịch Trực tiếp (P2P Trade - Opcode 0x19)
19. As a player, I want to invite another player to trade items and gold (`19 01`), so that the trade window opens simultaneously on both screens.
20. As a trading player, I want to offer up to 5 bag items and a gold amount (`19 02`), so that the server accurately conveys my offer to my trade partner without shifting item slots.
21. As a trading player, I want to lock and confirm the trade (`19 03`), so that the transaction completes atomically only when both parties have agreed.
22. As a trading player, I want the server to acquire a deterministic two-phase lock on both accounts before executing the trade, so that no race condition or duplication can occur even under high load.
23. As a trading player, I want the trade window to close immediately and safely with frame `19 04` if either party cancels (`19 03 00`) or if a timeout occurs, so that neither player's items are lost or locked.
24. As a trading player, I want to receive a polite toast notification (`19 02 03`) if my partner cancels the trade, so that I know what happened without unexpected popup menus appearing.

### Nhóm Quản lý 5 Container Vật phẩm
25. As a player, I want to move items between my main bag (`homdo`, 25 slots) and my Ba Đậu Yêu Balo (`tuideo`, 25 slots), so that I can manage my extended inventory.
26. As a player, I want to store items in my Ba Đậu Yêu Lưu Lãng (`luulang`, 10 slots) via `17 33` and retrieve them via `17 34`, so that my mobile storage functions correctly within its strict 10-slot capacity.
27. As a player, I want to store and retrieve items from my 50-slot Tiền Trang container (`1E 01..07`), with obsolete items cleared via `1E 08` upon login, so that my bank storage never shows ghost items.
28. As a player, I want my equipped gear (`trangbi`, 6 slots) to be sent as 10-byte records without slot bytes sorted by slot ascending, so that my character wears all equipment in the correct body positions.

---

## Implementation Decisions

### Module Boundaries & Interfaces
- **Shop Handler**: Sẽ tinh chỉnh lại logic giải mã và phản hồi của các handler trong module Shop. Nâng mức giới hạn mua tối đa lên 1.250 món; triển khai thuật toán tính toán ô trống All-or-Nothing trước khi trừ tiền; loại bỏ lệnh gửi nhầm `1B 03` khi chọn mở mua hàng từ hội thoại NPC.
- **Trade & Storage Handler**: Sửa lại toàn diện bộ giải mã packet Giao dịch P2P. Đọc 16 byte payload của gói tin offer gồm 4 byte vàng và 5 cặp `(slot, qty)`. Bắt sự kiện xác nhận và hủy theo đúng mã quy ước (`action = 1` đồng ý, `action = 0` hủy). Thay thế hoàn toàn mã phản hồi hủy giao dịch bằng gói tin đóng form giao dịch chuẩn (`19 04`) và mã toast hủy (`19 02 03`).
- **Vendor Stall Handler**: Viết lại bộ giải mã gói tin mở sạp. Đọc tuần tự mảng 10 ô vật phẩm cố định (mỗi ô 10 byte) trước, sau đó mới giải mã độ dài chuỗi tên bảng hiệu và các trường phụ trợ. Chuẩn hóa phản hồi mở sạp thành gói tin 3 byte.
- **Currency & Bank Service**: Chuẩn hóa gói tin đồng bộ tiền tệ thành các frame đơn lẻ 6 byte riêng biệt cho Vàng mang theo (`1A 04`) và Điểm tích lũy (`1A 03`). Bổ sung gói tin mở khóa nút bấm giao diện sau mỗi lần thực hiện gửi hoặc rút tiền trang (`1D 07` và `1D 08`).
- **Container Registry & Emitters**: Củng cố cơ chế xuất danh mục của 5 container:
  - Hòm đồ: 25 ô, chuỗi 12 byte có chứa byte slot.
  - Tiền trang: 50 ô, chuỗi 12 byte, luôn phát lệnh dọn sạch 50 ô (`1E 08`) trước khi phát dữ liệu.
  - Ba Đậu Yêu Balo: 25 ô, chuỗi 12 byte (`17 2F`).
  - Ba Đậu Yêu Lưu Lãng: 10 ô, chuỗi 12 byte (`17 66`), áp đặt giới hạn cứng tối đa 10 ô.
  - Trang bị: 6 ô, chuỗi 10 byte không chứa byte slot, sắp xếp tuần tự theo vị trí trang bị tăng dần.

### Architectural & Concurrency Decisions
- **Two-Phase State Lock**: Mọi giao dịch hoán đổi tài sản giữa 2 đối tượng phải tuân thủ nghiêm ngặt quy trình khóa 2 pha. Khóa bộ nhớ RAM theo thứ tự định danh người chơi tăng dần để triệt tiêu nguy cơ bế tắc (deadlock), sau đó thực thi giao dịch nguyên tử duy nhất trên luồng ghi cơ sở dữ liệu SQLite.
- **Loại bỏ Hoàn toàn Opcode `02 0B`**: Không sử dụng opcode `02 0B` cho bất kỳ mục đích hiển thị thông báo lỗi nào. Toàn bộ thông báo nghiệp vụ quy về mã lỗi chuẩn của giao thức, thông báo tự do chuyển sang kênh Chat Hệ thống.
- **Bảo toàn Giới hạn Tiền tệ**: Toàn bộ thao tác cộng/trừ tiền tệ phải được chặn trần ở mức 9.999.999 cho Vàng và 2.000.000.000 cho Điểm, từ chối nguyên tử mọi thao tác làm số dư âm hoặc vượt trần.

---

## Testing Decisions

### What Makes a Good Test
- **Kiểm thử hành vi bên ngoài qua Wire Packet (Wire Parity Testing)**: Không kiểm thử các hàm nội bộ; mọi bài test phải giả lập đúng chuỗi byte wire mà client gửi lên (Little-Endian, opcode, subcode) và kiểm tra chính xác các frame byte wire server phát ra cùng với sự thay đổi trạng thái trong cơ sở dữ liệu.
- **Tách biệt Môi trường Cơ sở dữ liệu**: Toàn bộ bài test phải chạy trên cơ sở dữ liệu SQLite In-Memory (`sqlite::memory:?cache=shared`), không chạm vào tệp cơ sở dữ liệu sản xuất.
- **Không viết test inline trong mã nguồn**: Toàn bộ test phải đặt tập trung tại thư mục kiểm thử ngoài repo root theo quy chuẩn kiến trúc.

### Modules to Test
- `tests/economy_wire_test.rs`: Bộ kiểm thử tích hợp chính, bao phủ:
  1. Đồng bộ 3 loại tiền tệ độc lập khi đăng nhập (`1A 04`, `1D 04`, `1A 03`).
  2. Mua hàng NPC số lượng lớn (tới 1.250 món) và các kịch bản từ chối All-or-Nothing.
  3. Bán hàng NPC từ túi đồ.
  4. Mở sạp cá nhân với 10 ô đồ và tên bảng hiệu, xác nhận phản hồi 3 byte.
  5. Mua hàng từ sạp cá nhân của người khác, trừ đồ và tiền nguyên tử.
  6. Giao dịch P2P 2 người chơi với đầy đủ vàng và vật phẩm, xác nhận chốt deal và hủy deal an toàn (`19 04`).
  7. Nộp và rút tiền trang kèm kiểm tra frame mở khóa nút bấm UI (`1D 07 / 08`).
  8. Kiểm tra tính toàn vẹn của 5 container vật phẩm (đặc biệt là Lưu Lãng 10 ô và Trang bị 10-byte).

### Prior Art
- Tham chiếu cấu trúc test runner và mô phỏng kết nối phiên từ `tests/economy_wire_test.rs`, `tests/inventory_storage_type_test.rs`, và `tests/login_char_flow_test.rs`.

---

## Out of Scope
1. Hệ thống cờ bạc, sòng bạc, máy slot và xổ số Lotto (`0x36` đến `0x3E`).
2. Quản lý quân lương và quỹ chiến dịch của Ngân hàng Thủy binh (`0x18`).
3. Đóng góp quỹ Quân Đoàn qua `0x1A 01..04`.
4. Cơ chế chuyển phát nhanh bưu kiện (`0x20 Express`).

---

## Further Notes
- Đặc tả này tuân thủ đầy đủ tài liệu phân tích mã máy client [`.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md), các phát hiện kiểm chứng tại [`.scratch/economy-refactor/GAP_ANALYSIS_MATRIX.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/GAP_ANALYSIS_MATRIX.md), Quyết định Kiến trúc [`docs/adr/0005-economy-wire-parity-and-transactions.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/adr/0005-economy-wire-parity-and-transactions.md), và Từ vựng chung tại [`CONTEXT.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CONTEXT.md).
