# Delphi Win32 Client Opcode Reference (`aLogin.exe`)

Tài liệu tra cứu nhanh ánh xạ các Opcode giữa Client Delphi và Server Rust `ts_dream`, được giải mã và kiểm chứng trực tiếp từ binary [`client_delphi/aLogin.exe`](client_delphi/aLogin.exe).

---

## 1. Cơ Chế Phân Luồng Gói Tin Đến (Main Packet Dispatcher)

- **Entry Point**: `0x0078A8EA`
- **Bảng Tra Cứu Opcode sang Case Index (LUT)**: `0x0078A8EE` (Kích thước `0xC8` bytes = 200 opcodes).
- **Bảng Nhảy Hàm Xử Lý (Main Jump Table)**: `0x0078A9B6`

| Opcode | Case Index | Target VA | Callee / Function | Ý nghĩa & Container đích |
|:---:|:---:|:---:|:---|:---|
| `0x02` | `3` | `0x0078B6FC` | `FUN_0078b6fc` | Chat, Banner, Tạo Quân Đoàn (`02 0B` là Quân Đoàn, cấm dùng làm toast) |
| `0x08` | `9` | `0x0078D125` | `FUN_0078d125` | Điểm thuộc tính, Thủy binh, Tài bảo, Đạn dược |
| `0x14` | `18` | `0x0078EC3F` | `FUN_0078ec3f` | Party, Hội thoại NPC (`14 01` mở mua, `14 09` mở kho) |
| `0x17` | `20` | `0x007902FB` | `FUN_007902fb` | Túi đồ chính (`homdo`), Ba Đậu Yêu Balo (`tuideo`), Ba Đậu Yêu Lưu Lãng (`luulang`), Sạp hàng cá nhân |
| `0x18` | `21` | `0x00790ED5` | `FUN_00790ed5` | Eve Script Action, Quest Sync, Quân Lương Thủy Binh |
| `0x19` | `22` | `0x0079157C` | `FUN_0079157c` | Giao dịch trực tiếp P2P & Thú nuôi |
| `0x1A` | `23` | `0x0079175F` | `FUN_0079175f` | Đồng bộ Vàng (`1A 04`, 6B), Điểm tích lũy (`1A 03`, 6B), Lỗi trần vàng (`1A 06`) |
| `0x1B` | `24` | `0x00791D80` | `FUN_00791d80` | Mua/Bán Shop NPC (`1B 01` kết quả mua, `1B 02` kết quả bán) |
| `0x1D` | `25` | `0x00791F76` | `FUN_00791f76` | Ngân hàng Vàng (`1D 04` sync, `1D 07` mở khóa Nộp, `1D 08` mở khóa Rút) |
| `0x1E` | `26` | `0x007921A6` | `FUN_007921a6` | Các container Kho: Tiền Trang (`tientrang`), Kho Nhà Trọ, Kho Phụ |
| `0x23` | `31` | `0x00792BAD` | `FUN_00792bad` | Hệ số phòng trọ (`23 04 <A> <B>`), banner chào hỏi |

---

## 2. Bảng Phân Luồng Chi Tiết Opcode `0x1E` (Container Kho)

Hàm xử lý `FUN_007921a6` tại VA `0x007921A6` đọc `sub = P[0]` và nhảy qua bảng `0x007921D5`:

| Subcode | Target VA | Handler | Payload Data | Hành vi & Container thực tế trên Client |
|:---:|:---:|:---|:---:|:---|
| `1E 01` | `0x0079220D` | `FUN_007707a0` | `N × 12B` | **Dump Tiền Trang 50 ô** (`player + 0x0A48`). Mỗi record gồm: `[slot:u8][id:u16][qty:u8][attrs:4B][exp:u32]`. |
| `1E 02` | `0x00792221` | `FUN_00770454` | **11B** | **Thêm 1 vật phẩm vào Tiền Trang 50 ô** (`id:u16, qty:u8, attrs:4B, exp:u32`). KHÔNG có byte slot, client tự tìm ô trống bằng `FUN_0076ff98()`. Frame dài 13 byte: `F4440D001E02...`. |
| `1E 03` | `0x00792235` | `FUN_00770170` | 3B | **Trừ vật phẩm trong Tiền Trang theo ID** (`id:u16, qty:u8`), client tự tìm slot qua `FUN_0076fff4()`. |
| `1E 04` | `0x00792249` | `FUN_007705d8` | 12B | **Thêm vật phẩm vào Kho Nhà Trọ/Thương Nhân** tại slot chỉ định (`slot:u8, id:u16, qty:u8, attrs:4B, exp:u32`). |
| `1E 05` | `0x0079225D` | `FUN_0077021c` | 2B | **Trừ vật phẩm trong Tiền Trang theo Slot** (`slot:u8, qty:u8`), gọi `FUN_0077250c`. Frame dài 4 byte: `F44404001E05<slot><qty>`. |
| `1E 06` | `0x00796347` | `caseD_0` | — | **No-op / Dead branch**: Client Delphi nhảy thẳng về Exit, không xử lý gì. CẤM dùng cho Tiền Trang. |
| `1E 07` | `0x00796347` | `caseD_0` | — | **No-op / Dead branch**: Client Delphi nhảy thẳng về Exit. CẤM dùng cho Tiền Trang. |
| `1E 08` | `0x00792271` | `FUN_0076fdb8` | 0B | **Mass-clear toàn bộ 50 ô Tiền Trang**. Bắt buộc gửi trước khi phát `1E 01`. |
| `1E 09` | `0x00792282` | `FUN_007714a0` | `N × 12B` | **Dump Kho Phụ 5 ô** (`player + 0x0B14`). |
| `1E 0A` | `0x00792296` | `FUN_007702d0` | 11B | **Thêm vật phẩm vào Kho Phụ 5 ô**. |
| `1E 0B` | `0x007922AA` | `FUN_007700bc` | 2B | **Trừ vật phẩm trong Kho Phụ 5 ô theo Slot**. |

---

## 3. Bảng Mã Phản Hồi Giao Dịch P2P (`0x19`)

| Frame S→C | Ý nghĩa Wire | Kích hoạt khi nào |
|:---|:---|:---|
| `F4 44 03 00 19 02 01` | Đối phương offline | Mời người chơi không có trong session registry. |
| `F4 44 03 00 19 02 02` | Đối phương đang bận | Mời người chơi đang `trade.active == true`, tự mời mình, hoặc người mời đang bận. |
| `F4 44 03 00 19 02 03` | Toast đối tác đã hủy | Người chơi bên kia bấm Hủy hoặc phiên giao dịch bị timeout. |
| `F4 44 03 00 19 02 04` | Xác nhận chốt giao dịch | Khi cả hai bên đã đồng ý và SQLite transaction commit thành công. |
| `F4 44 02 00 19 04` | Đóng form giao dịch | Bắt buộc gửi kèm khi hủy giao dịch, timeout hoặc partner disconnect. |

---

## 4. Lưu ý khi Dịch Ngược & Viết Handler

1. **Luôn kiểm tra bảng nhảy nhị phân**: Không suy đoán tính đối xứng giữa C→S và S→C. Các subcode `1E 06` và `1E 07` từng bị gán nhầm do suy đoán lý thuyết.
2. **Cấu trúc 2-byte C→S**: Giao thức C→S gửi thao tác kho luôn đi theo cặp `[slot: u8, qty: u8]`. Cấm phân tích payload như một danh sách mảng slot đơn lẻ.
