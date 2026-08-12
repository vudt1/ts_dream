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

### WarInfo / Ô chiến trận (Battle Cell)
- **Định nghĩa**: Một ô trong lưới chiến trận 4 hàng × 5 cột chứa một thực thể tham chiến (nhân vật, sủng vật, quái vật, thủ thành). Mang trạng thái HP/SP hiện tại, chỉ số chiến đấu (đã cộng trang bị), đội, level, buff/debuff và snapshot gói tin 23 byte gửi cho client mỗi khi thay đổi.
- **Tránh dùng các từ mơ hồ**: *Cell*, *Unit*, *Grid Slot*.

### DiaHinh / Địa hình trận đấu
- **Định nghĩa**: Mã định danh "địa hình" của một trận đấu, chỉ được echo vào packet mở bàn cờ và các biến thể packet thành viên (PK/NPC thường dùng `112`, NPC chủ động `4712`, trận thủ thành dùng `TeamDef[0]`, bàn mở cho thành viên PK dùng `7000`).
- **Ràng buộc (Invariants)**: `DiaHinh` **không** ảnh hưởng targeting hay sát thương.

### TeamDef / Đội thủ thành (Defender Team)
- **Định nghĩa**: Danh sách tối đa 10 NPC thủ thành (2 hàng trên của lưới chiến trận) của trận đấu khởi tạo từ quest TeamDef/nhóm hoặc NPC chủ động. Trong trận NPC chủ động, danh sách được nhân bản từ cùng một npcId theo `SoLuong` (1..5) tại các slot cố định.

### ListQS / Người quan sát (Spectator)
- **Định nghĩa**: 50 chỗ dành cho người chơi đang xem một trận đấu đang diễn ra. Người quan sát nhận toàn bộ packet của trận và bị "đuổi" (clear trạng thái) khi trận kết thúc.

### Quan-su (Quân sư / QS)
- **Định nghĩa**: Thành viên nhóm được đội trưởng chỉ định để mỗi lượt hồi SP cho đội trưởng, sủng vật của đội trưởng, từng thành viên và sủng vật của họ theo công thức `Round((Int + Int2) / 15)`, giới hạn bởi SpMax. Chỉ đội trưởng được chỉ định hay hủy quan-su.

### Trận đấu NPC chủ động (Active-NPC Battle)
- **Định nghĩa**: Trận đấu tự khởi tạo bởi NPC tuần tra trên bản đồ khi một người chơi (lẻ hoặc đội trưởng, không trong trận) lọt vào phạm vi `Coord` của NPC. NPC di chuyển tới vị trí người chơi, khởi tạo trận với DiaHinh `4712` và đánh dấu NPC đang tham chiến (`_IdBattle`) để không kích hoạt lại; sau trận NPC respawn sau thời gian `Delay`.
- **Tránh dùng các từ mơ hồ**: *Walk Battle*, *Auto Fight*.

### NPC tuần tra (NpcOnMapWalk)
- **Định nghĩa**: Tiến trình nền điều khiển mọi NPC trên bản đồ: di chuyển ngẫu nhiên trong hộp tọa độ quanh vị trí gốc, đuổi theo người chơi trong tầm, phát broadcast thay đổi vị trí, giảm dần `Delay` và đánh dấu trạng thái tham chiến của NPC.

### Turn / Lượt chiến đấu
- **Định nghĩa**: Một chu kỳ của vòng chiến đấu gồm: reset trạng thái lệnh, tick buff (đốt/độc theo lượt), chờ lệnh người chơi tối đa ~21 giây, sắp xếp thứ tự hành động (`Attacked DESC, Agi DESC, Random DESC`) và thực thi hành động từng thực thể.

### RNG battle / Luồng ngẫu nhiên trận đấu
- **Định nghĩa**: Ba luồng ngẫu nhiên kiểu .NET (time-seeded, độc lập) của một trận: `random_0` (chọn drop/kỹ năng), `random_1` (tie-break thứ tự lượt + sai số sát thương), `random_2` (tọa độ respawn). Một luồng thứ tư `random_3` ở tầng thế giới lo việc di chuyển NPC tuần tra.
- **Ràng buộc (Invariants)**: Các luồng không được trộn lẫn; thứ tự sử dụng phải khớp tham chiếu C# để replay deterministic.

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

### Learn / Upgrade Skill (Học / nâng cấp kỹ năng) — opcode 0x1C
- **Định nghĩa**: Hành động chi `SkillPoint` để học mới hoặc nâng level một kỹ năng, áp dụng cho cả Nhân vật (sub 1) và Sủng vật (sub 2).
- **Ràng buộc (Invariants)**:
  - Kỹ năng mới: cần đủ element (không học kỹ năng khắc chế), đủ prereq `IdDK1..6` (tất cả 0 hoặc ≥1 đã học), và chi phí `GetPointSkillAdd(element, point) + (lv-1)`.
  - Nâng cấp slot đã tồn tại: chi phí chênh lệch level, chỉ nâng khi `lv_target > lv_hiện tại`.
  - Linh giới `lv <= LvMax` và `Reborn kỹ năng <= Reborn hiện tại`.
  - Mỗi success phát `F4440C0008016E01`+le32(lv)+le32(skill); kết thúc phát `SendSkillPointtoClient`.

### Reborn / Rebirth (Đổi nghề) — opcode 0x17 sub 46
- **Định nghĩa**: Nhân vật đạt ngưỡng (≥120) đổi nghề, đặt lại level/stats, giữ các kỹ năng đặc thù và pack nghề. Là quá trình "chết" (server đóng socket để ép đăng nhập lại).
- **Ràng buộc (Invariants)**:
  - Không được mặc trang bị ở slot ≤ 6.
  - Nhân vật mới: `Lv=1`, `Point/SkillPoint = base + (Lv-120)/5`, `Hp/Sp=181`, stats=0, `Texp=13`; `Reborn` tăng, `Job` đổi theo menu (reborn 2).
  - Chỉ giữ kỹ năng đặc biệt (10016-19, 11016-19, 12016-19, 13015-18); `DELETE FROM Skill` scope theo `player_id`.
  - Tail: replay `OnWin` quest hiện tại, cập nhật quest step NPC 59411, gửi `F444…F476`, rồi đóng socket.

### Pet Reborn (Hồi sinh Sủng vật) — opcode 0x2C
- **Định nghĩa**: Tiêu 1 đơn vị vật phẩm `RbPetFrom→RbPetTo` để biến đổi Sủng vật về NPC mới: level 1, skill từ NPC (skill 10016/11016/12016/13015 lv 10), bonus theo mốc 30/60.
- **Ràng buộc (Invariants)**:
  - Bonus point `(lv - threshold)/5` phân bổ theo **weighted random** `GetRandomPointPet` (theo 6 stat NPC, 7 lần `.NET` draw/điểm) — không deterministic.
  - `HpMax/Spmax` tính từ stat **gốc** (trước bonus) với mapping `getPetHpMax` (rb 0/1→`getHpMax(0)`, rb 2→`getHpMax(1)`).
  - Phát broadcast map `0F02`/`0F01` + `SendStatusPet` + `06001301` + `2C01`; guards fail → silent.

### Max HP (HpMax)
- **Định nghĩa**: Thuật ngữ chuẩn chỉ HP tối đa. Lưu ý: codebase dùng nhiều cách viết — C# in-memory `_My_HpMax`, hằng DB `_Hpmax`, Rust `hp_max` — tất cả đều là **Max HP (HpMax)**.

### Item / Inventory (Vật phẩm & Túi đồ)
- **Định nghĩa**: Trang bị, vật phẩm tiêu hao hoặc nguyên liệu do Nhân vật sở hữu trong túi đồ (Inventory) hoặc rương lưu trữ (Storage).

### Map & Spatial Position (Bản đồ & Tọa độ)
- **Định nghĩa**: Không gian tọa độ thế giới game nơi các Nhân vật di chuyển, tương tác với NPC và kích hoạt các sự kiện/trận đấu.

### Web Admin Dashboard (Hệ thống Quản trị Web)
- **Định nghĩa**: Bounded Context vận hành & giám sát (Operations) cho phép Quản trị viên theo dõi số lượng người chơi online, xem log gói tin realtime (SSE) và điều khiển trạng thái server (Start/Stop).


