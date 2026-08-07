# CONTEXT.md — TS Dream Domain Glossary

Tài liệu này lưu trữ **Từ vựng chung (Ubiquitous Language)** và các khái niệm nghiệp vụ cho miền ứng dụng **TS Dream**.

> [!NOTE]
> Theo nguyên tắc Domain-Driven Design (DDD) và quy chuẩn tại [`docs/agents/domain.md`](docs/agents/domain.md), `CONTEXT.md` tập trung thuần túy vào **Thuật ngữ Nghiệp vụ (Domain Glossary)**.


---

## 1. Thuật ngữ Bounded Context & Miền Nghiệp vụ

### Account (Tài khoản)
- **Định nghĩa**: Thực thể quản trị danh tính người dùng trên hệ thống, chứa thông tin đăng nhập (username, password), quyền hạn (User / Admin) và trạng thái tài khoản.
- **Tránh dùng các từ mơ hồ**: *User*, *Login Info*, *Client Account*.

### Character / Player (Nhân vật / Người chơi)
- **Định nghĩa**: Đại diện avatar của người chơi trong thế giới game TS Online. Một Account có thể sở hữu nhân vật với các chỉ số (Level, HP, SP, Atk, Def, Agi, Int), danh sách Kỹ năng, Túi đồ và Vị trí trên bản đồ.
- **Ràng buộc (Invariants)**:
  - Cấp độ tối đa của Nhân vật là 200.
  - Tên nhân vật và dữ liệu hội thoại trên giao thức truyền thông tuân thủ bảng mã VISCII 1.1.

### Session / Client Connection (Phiên kết nối)
- **Định nghĩa**: Trạng thái kết nối trực tuyến giữa ứng dụng Client của người chơi và Game Server qua mạng TCP. Mỗi Session tương ứng với một người chơi đang hoạt động thực tế.

### Opcode & Domain Packet (Gói tin Opcode)
- **Định nghĩa**: Đơn vị thông điệp nghiệp vụ trao đổi giữa Client và Server. Mỗi Opcode đại diện cho một lệnh hoặc sự kiện nghiệp vụ (ví dụ: Đăng nhập `0x00`/`0x01`, Chat `0x02`, Di chuyển `0x05`/`0x06`, Chiến đấu `0x32`).
- **Tránh dùng các từ mơ hồ**: *Data Buffer*, *Payload*, *Raw Bytes* (trừ khi xử lý ở tầng mạng hạ tầng).

### Dispatcher (Bộ điều phối nghiệp vụ)
- **Định nghĩa**: Dịch vụ miền (Domain Service) có nhiệm vụ tiếp nhận Gói tin Opcode đã giải mã, phân tích mã lệnh (Opcode/Subcode) và điều phối tới các Handler xử lý logic tương ứng.

### Battle Session / Battle Engine (Hệ thống Trận đấu)
- **Định nghĩa**: Bounded Context độc lập quản lý trận đánh theo lượt (turn-based grid combat). Quy định thứ tự hành động dựa trên Agi, xử lý kỹ năng, tính toán sát thương, tiêu hao HP/SP và kết quả trận đấu (thắng, thua, nhận kinh nghiệm/vật phẩm).

### Pet / Companion (Sủng vật / Đậu đậu)
- **Định nghĩa**: Nhân vật phi người chơi (NPC) có thể thu phục hoặc chiêu mộ đồng hành cùng Nhân vật người chơi trong các trận đấu và di chuyển.

### Stat Allocation (Phân bổ chỉ số) — opcode 0x08
- **Định nghĩa**: Hành động người chơi tiêu Point để tăng một chỉ số cơ bản (Int, Atk, Def, Agi, Hpx, Spx) hoặc tái tính Hpmax/Spmax. Mỗi thay đổi phát 1 packet stat `F4440C000801` (Type_Status + dấu + giá trị tuyệt đối).
- **Ràng buộc (Invariants)**:
  - Điều kiện gate: `Point >= points && points > 0`, ngoài chiến đấu.
  - Cap 400: chỉ áp dụng cho Int/Atk/Def/Agi/Hpx/Spx (id 27–32); Hpmax/Spmax (25/26) không tăng chỉ số và **không trừ Point**.
  - Max HP/SP (Hpmax/Spmax): khi cập nhật trong phân bổ Hpx/Spx chỉ cập nhật in-memory — **không phát packet nào** cho Max (C# `PlayerUpdateDataId` nhánh `_Hpmax`/`_Spmax`).

### Point / Skill Point (Điểm chỉ số)
- **Định nghĩa**: Điểm có thể phân bổ (allocatable) để tăng chỉ số; `Point` (query trong DB/game) phân biệt với `SkillPoint` (điểm học kỹ năng). Cả hai đều cập nhật dưới dạng opcode 0x08 packet type `0x26` (Point).

### Skill bar / Hotkey (Thanh kỹ năng) — opcode 0x28
- **Định nghĩa**: Bản đồ slot 1..10 gán một kỹ năng vào thanh phím tắt của nhân vật. Nhận dữ liệu client → lưu `SkillSave`; **không phản hồi** (C# chỉ `SkillSaveUpdateId`). Slot 0 = clear (no-op).

### Max HP (HpMax)
- **Định nghĩa**: Thuật ngữ chuẩn chỉ HP tối đa. Lưu ý: codebase dùng nhiều cách viết — C# in-memory `_My_HpMax`, hằng DB `_Hpmax`, Rust `hp_max` — tất cả đều là **Max HP (HpMax)**.

### Item / Inventory (Vật phẩm & Túi đồ)
- **Định nghĩa**: Trang bị, vật phẩm tiêu hao hoặc nguyên liệu do Nhân vật sở hữu trong túi đồ (Inventory) hoặc rương lưu trữ (Storage).

### Map & Spatial Position (Bản đồ & Tọa độ)
- **Định nghĩa**: Không gian tọa độ thế giới game nơi các Nhân vật di chuyển, tương tác với NPC và kích hoạt các sự kiện/trận đấu.

### Web Admin Dashboard (Hệ thống Quản trị Web)
- **Định nghĩa**: Bounded Context vận hành & giám sát (Operations) cho phép Quản trị viên theo dõi số lượng người chơi online, xem log gói tin realtime (SSE) và điều khiển trạng thái server (Start/Stop).


