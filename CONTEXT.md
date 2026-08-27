# CONTEXT.md — TS Dream Domain Glossary

Tài liệu này lưu trữ **Từ vựng chung (Ubiquitous Language)** và các khái niệm nghiệp vụ cho miền ứng dụng **TS Dream**.

> [!NOTE]
> Theo nguyên tắc Domain-Driven Design (DDD) và quy chuẩn tại [`docs/agents/domain.md`](docs/agents/domain.md), `CONTEXT.md` tập trung thuần túy vào **Thuật ngữ Nghiệp vụ (Domain Glossary)**.


---

## 1. Thuật ngữ Bounded Context & Miền Nghiệp vụ

### Account (Tài khoản)
- **Định nghĩa**: Thực thể gốc định danh người dùng, PK là `player_id` (BIGINT AUTO_INCREMENT, `accounts.player_id`, wire dùng làm `player_id`). Chứa `pass1`/`pass2` (latin1_bin, so sánh byte-exact qua HEX), `gm_level`, trạng thái treo. Không có cột `account` riêng — identity duy nhất là `player_id`.
- **Ràng buộc (Invariants)**: PK duy nhất là `player_id`; mọi FK nhân vật là shared PK `characters.character_id = accounts.player_id` (1:1, PK đồng thời là FK).
- **Tránh dùng các từ mơ hồ**: *User*, *Login Info*, *Client Account*, *id* (thay bằng `player_id`), *account* (không tồn tại cột riêng).

### Character / Player (Nhân vật / Người chơi)
- **Định nghĩa**: Đại diện avatar duy nhất của một Account trong thế giới game TS Online (PC server **1 Account : 1 Character**, shared PK `character_id = player_id`). Mang chỉ số (Level, HP, SP, Atk, Def, Agi, Int), Kỹ năng, Túi đồ và Vị trí.
- **Ràng buộc (Invariants)**:
  - Quan hệ 1:1 Account–Character (`characters.character_id = accounts.player_id`, PK shared).
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

### ThingData (Dữ liệu Vật phẩm Chuẩn hóa 35 Bytes)
- **Định nghĩa**: Cấu trúc nhị phân 35-byte đại diện toàn diện cho một vật phẩm (Item) trong game TS Online theo chuẩn Mobile & PC. Bao gồm 20 trường thuộc tính: mã vật phẩm (`id`), số lượng (`count`), chỉ số sát thương (`damage`), kháng cự (`defend`), hệ thuộc tính (`element`), ngọc khảm (`gem`), cấp cường hóa (`enhance`), cấp linh vũ khí (`grow`), thời gian hết hạn (`delete_time` theo chuẩn OADate), trạng thái khóa (`is_lock`), thuộc tính phụ (`attribute`), cấp tinh luyện (`refine`), độ bền (`durability`), cùng các đặc tính dòng ngọc và tẩy luyện.
- **Ràng buộc (Invariants)**: Kích thước mã hóa nhị phân luôn cố định đúng 35 bytes (`THING_DATA_SIZE = 35`).

### 4 Kho Võ Tướng (Four Pet Storage Tiers)
- **Định nghĩa**: 4 phân vùng lưu trữ Sủng vật / Võ tướng của một Nhân vật trong hệ thống `character_pets` với `storage_type`:
  1. `Follow` (Tùy thân): Tối đa 4 võ tướng mang theo bên mình có thể xuất chiến (slot 1..4).
  2. `Cart` (Mã xa / Xe kéo): Tối đa 4 võ tướng đi kèm xe (slot 1..4).
  3. `Inn` (Khách sạn): Tối đa 30 võ tướng lưu giữ tại quán trọ (slot 1..30).
  4. `Warehouse` (Kho võ tướng): Tối đa 150 võ tướng lưu trữ trong kho mở rộng (slot 1..150).
- **Ràng buộc (Invariants)**: Khi chuyển đổi giữa các kho lưu trữ, toàn bộ chỉ số, kỹ năng, cấp độ, số lần ăn linh đơn và reborn của võ tướng được bảo toàn nguyên vẹn.

### Eve Script Engine & State Machine (Động cơ Kịch bản & Máy trạng thái Sự kiện)
- **Định nghĩa**: Bounded Context thực thi toàn bộ kịch bản tương tác thế giới (hội thoại NPC, câu hỏi trắc nghiệm, cổng dịch chuyển Door, trận đấu nhiệm vụ Fight, trao thưởng) dựa trên dữ liệu container nhị phân `eve.emg` (3,800+ scenes).
- **4 Tầng Ưu tiên (4-Tier Chain Resolution)**: Khi một NPC hoặc vị trí có nhiều nhánh kịch bản thỏa mãn điều kiện:
  1. `Highest Quest Step`: Nhánh có bước nhiệm vụ cao nhất được ưu tiên hàng đầu.
  2. `Longest Condition Chain`: Nhánh có nhiều điều kiện ràng buộc nhất (độ đặc thù cao hơn).
  3. `Result Count`: Nhánh có nhiều hành động kết quả hơn.
  4. `Declaration Order`: Thứ tự khai báo trong scene.
- **Eve Auto-Chain & Loop Protection**: Động cơ tự động nối tiếp chuỗi sự kiện với 4 lớp bảo vệ chống treo/lặp vô hạn:
  1. `Same Condition Guard`: Ngắt nếu vòng lặp lặp lại cùng một điều kiện kiểm tra.
  2. `Re-Question Guard`: Ngắt nếu câu hỏi trắc nghiệm hiển thị lặp lại mà không có tương tác mới.
  3. `Re-Battle Guard`: Ngắt nếu cùng một trận đấu bị kích hoạt lặp liên tục.
  4. `Duplicate Item Guard`: Ngắt nếu trao trùng vật phẩm đặc biệt trong cùng một phiên chuỗi.
- **Event Phase (Các giai đoạn của Máy trạng thái Sự kiện)**:
  - `Idle`: Không có kịch bản nào đang diễn ra.
  - `Dialogue`: Đang hiển thị thoại NPC hoặc chuỗi câu thoại.
  - `Question`: Đang chờ người chơi lựa chọn đáp án câu hỏi / menu.
  - `Battle`: Đang diễn ra trận đấu nhiệm vụ do Eve Script kích hoạt.
  - `Completed`: Hoàn thành kịch bản, trao phần thưởng và cập nhật trạng thái cờ.

### Mission & MissionFlags / BitFlags (Hệ thống Nhiệm vụ & Cờ Trạng thái)
- **Định nghĩa**: Mô hình dữ liệu quản lý tiến trình nhiệm vụ và thế giới của người chơi:
  - `Mission`: Cặp `(mission_id, step)` lưu bước tiến độ hiện tại của từng tuyến nhiệm vụ.
  - `MissionFlags`: Các cờ đánh dấu tạm thời phục vụ rẽ nhánh trong nhiệm vụ.
  - `BitFlags (Forever Flags)`: Mảng bit đánh dấu vĩnh viễn các sự kiện lịch sử thế giới mà người chơi đã từng hoàn thành một lần trong đời (chống nhận lại phần thưởng hoặc lặp lại cốt truyện một lần).
  - `RoleCounts`: Bộ đếm số lần tương tác / đánh bại các vai trò hoặc NPC cụ thể.
  - `CompletedEvents`: Danh sách mã sự kiện đã hoàn tất.

### ResponseSender (Bộ Đóng Gói Phản Hồi Nghiệp Vụ)
- **Định nghĩa**: Lớp bao bọc abstraction ở tầng Server Handlers đóng gói các thông điệp phản hồi gửi về Client (`send_bag_items`, `send_equipment_items`, `send_storage_items`, `send_dialog_talk`, `end_talk`, `send_stat_update`, `send_hp_sp_updates`, `broadcast`). Tách biệt hoàn toàn tầng nghiệp vụ khỏi việc định dạng và nối chuỗi byte thô.

### PlayerStateManager (Bộ Quản Lý Trạng Thái Người Chơi)
- **Định nghĩa**: Dịch vụ miền (Domain Service) thread-safe quản lý toàn bộ trạng thái sống in-memory của người chơi đang online: HP, SP, chỉ số cơ bản, chỉ số trang bị cộng thêm, trạng thái biến thân và định vị. Đảm bảo tính toán chỉ số chiến đấu và biến động máu/mana diễn ra tức thì không bị nghẽn I/O.

### TradeSystem (Hệ thống Giao Dịch Hai Pha)
- **Định nghĩa**: Domain Engine thực thi giao dịch an toàn 2-phase atomic giữa hai người chơi:
  - Khóa đồng thời phiên giao dịch của 2 người chơi theo thứ tự ID ổn định (chống deadlock).
  - Kiểm tra trước chỗ trống (probe-first) đối với túi đồ và võ tướng của cả hai bên.
  - Hoán đổi tài sản (vật phẩm, võ tướng, tiền vàng) nguyên tử; tự động hoàn trả (rollback) trọn vẹn nếu có sự cố ngắt kết nối hoặc hủy giao dịch.

### AutoSave (Dịch vụ Tự Động Lưu Trữ Nền)
- **Định nghĩa**: Tiến trình nền (Background Task) chạy định kỳ mỗi 3 phút trong Tokio runtime, sử dụng thuật toán băm FNV-1a để phát hiện những phiên người chơi có trạng thái biến động (dirty state: vàng, HP/SP, chỉ số, túi đồ, võ tướng, nhiệm vụ) và thực hiện ghi vào MySQL theo từng Transaction nguyên tử.

### Map & Spatial Position (Bản đồ & Tọa độ)
- **Định nghĩa**: Không gian tọa độ thế giới game nơi các Nhân vật di chuyển, tương tác với NPC và kích hoạt các sự kiện/trận đấu.

### Web Admin Dashboard (Hệ thống Quản trị Web)
- **Định nghĩa**: Bounded Context vận hành & giám sát (Operations) cho phép Quản trị viên theo dõi số lượng người chơi online, xem log gói tin realtime (SSE) và điều khiển trạng thái server (Start/Stop).



