# CONTEXT.md — TS Dream Domain Glossary

Tài liệu này lưu trữ **Từ vựng chung (Ubiquitous Language)** và các khái niệm nghiệp vụ cho miền ứng dụng **TS Dream**.

> [!NOTE]
> Theo nguyên tắc Domain-Driven Design (DDD) và quy chuẩn tại [`docs/agents/domain.md`](docs/agents/domain.md), `CONTEXT.md` tập trung thuần túy vào **Thuật ngữ Nghiệp vụ (Domain Glossary)**.


---

## 1. Thuật ngữ Bounded Context & Miền Nghiệp vụ

### Account (Tài khoản)
- **Định nghĩa**: Thực thể gốc định danh người dùng, PK là `player_id` (BIGINT AUTO_INCREMENT, `accounts.player_id`, wire dùng làm `player_id`). Chứa `pass1`/`pass2` (latin1_bin, so sánh byte-exact qua HEX), `gm_level`, trạng thái treo. Không có cột `account` riêng — identity duy nhất là `player_id`.
- **Ràng buộc (Invariants)**: PK duy nhất là `player_id`; mọi FK nhân vật là shared PK `characters.character_id = accounts.player_id` (1:1, PK đồng thời là FK). **Chính sách mật khẩu**: `pass1`/`pass2` dài **8–10 byte**, mỗi byte trong `0x21..=0x7E` (ASCII in được, không khoảng trắng/control), **không** UTF-8/VISCII để `HEX(pass)=HEX(?)` luôn round-trip; mặc định lúc tạo là `1111111111` (đặt qua web dashboard), người chơi đổi qua op `0x23` sub 1.
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
- **Định nghĩa**: Đơn vị thông điệp nghiệp vụ trao đổi giữa Client và Server. Mỗi Opcode đại diện cho một lệnh hoặc sự kiện nghiệp vụ (ví dụ: Chat `0x02`, Di chuyển `0x05`/`0x06`, Chiến đấu `0x32`).
- **Tránh dùng các từ mơ hồ**: *Data Buffer*, *Payload*, *Raw Bytes* (trừ khi xử lý ở tầng mạng hạ tầng).

### System Alert / OP_SYSTEM_ALERT (Cảnh báo Hệ thống)
- **Định nghĩa**: Gói tin thông báo lỗi hệ thống, ngắt kết nối hoặc cảnh báo người chơi từ Server gửi xuống Client (Opcode `0x00`). Đây là gói tin **đơn hướng 100% (S → C)**; Client không bao giờ gửi Opcode này lên Server. Cấu trúc gồm Header `F4 44`, độ dài 2 byte, Opcode `0x00` và Sub-opcode (status 0..57) mang thông điệp tương ứng.
- **Ràng buộc (Invariants)**: Chiều giao tiếp duy nhất là Server đến Client (S → C). Bất kỳ frame Opcode `0x00` nào từ Client gửi lên Server đều được coi là không hợp lệ và bị loại bỏ.


### Dispatcher (Bộ điều phối nghiệp vụ)
- **Định nghĩa**: Dịch vụ miền (Domain Service) có nhiệm vụ tiếp nhận Gói tin Opcode đã giải mã, phân tích mã lệnh (Opcode/Subcode) và điều phối tới các Handler xử lý logic tương ứng.

### Battle Session / Battle Engine (Hệ thống Trận đấu)
- **Định nghĩa**: Bounded Context độc lập quản lý trận đánh theo lượt (turn-based grid combat). Quy định thứ tự hành động dựa trên Agi, xử lý kỹ năng, tính toán sát thương, tiêu hao HP/SP và kết quả trận đấu (thắng, thua, nhận kinh nghiệm/vật phẩm).

### WarInfo / Ô chiến trận (Battle Cell)
- **Định nghĩa**: Một ô trong lưới chiến trận 4 hàng × 5 cột chứa một thực thể tham chiến (nhân vật, sủng vật, quái vật, thủ thành). Mang trạng thái HP/SP hiện tại, chỉ số chiến đấu (đã cộng trang bị), đội, level, buff/debuff và snapshot gói tin 23 byte gửi cho client mỗi khi thay đổi.
- **Tránh dùng các từ mơ hồ**: *Cell*, *Unit*, *Grid Slot*.

### Kênh Lệnh Chiến Đấu (Battle Command Channel) — opcode 0x32
- **Định nghĩa**: Kênh giao tiếp duy nhất tiếp nhận các hành động chiến thuật của Nhân vật hoặc Sủng vật được gửi từ client trong trận đấu. Khung nhị phân cố định 10 bytes:
  - Sub `0x01`: Đánh thường (`10000`), Thi triển kỹ năng, Phòng thủ (`17001`), Bỏ chạy (`18001`), Bắt võ tướng (`15001`).
  - Sub `0x02`: Sử dụng vật phẩm chiến đấu mục tiêu trên lưới ô cờ.
- **Ràng buộc (Invariants)**: Phân biệt tuyệt đối với kênh `0x0B` (vòng đời trận đấu và thao tác ngoài bản đồ). Không nhận lệnh chiến đấu qua `0x0B`.

### Khung Mở Hiệp Đấu (Turn Prompt Frame) — opcode 0x34 sub 0x01
- **Định nghĩa**: Gói tin nhị phân 2 bytes (`F4 44 02 00 34 01`) được Server gửi xuống Client ở đầu mỗi hiệp đấu. Kích hoạt client reset trạng thái 21 ô cờ, mở thanh menu điều khiển và khởi chạy bộ đếm lùi thời gian (mặc định 20 giây).

### Khung Báo Dứt Hoạt Ảnh (Turn Action Finish Frame) — opcode 0x32 sub 0x02
- **Định nghĩa**: Gói tin nhị phân 2 bytes (`F4 44 02 00 32 02`) được Server gửi xuống Client ở cuối mỗi chuỗi đòn đánh của hiệp đấu để client giải phóng hàng đợi diễn hoạt và kết thúc chuỗi hoạt ảnh chiêu thức.

### Lưới Chiến Trận Isometric 4×5 (Isometric Battle Grid)
- **Định nghĩa**: Ma trận 20 ô (4 hàng `grp 0..3` × 5 cột `slot 0..4`) hiển thị theo hình chiếu trục đo Isometric. Cột trung tâm là `slot 2`, xếp sang hai cánh theo thứ tự `[2, 1, 3, 0, 4]`.
- **Ràng buộc (Invariants)**: `grp < 2` là Phe Địch (Phe 2); `grp >= 2` là Phe Ta (Phe 1). Các kỹ năng lan 3 mục tiêu (`sl_danh = 3`) khi nhắm ô tâm 2 phải lấy 2 ô kề Isometric là `{1, 2, 4}`.

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

### Thu phục Võ tướng / Bắt quái (Pet Capture) — skill 15001
- **Định nghĩa**: Hành động người chơi sử dụng lệnh kỹ năng `15001` (hoặc đạo cụ Lưới bắt) để thu phục một NPC quái vật trên bàn cờ làm Sủng vật tùy thân (chuyển vào 1 trong 4 ô trống của `character_pets`).
- **Ràng buộc (Invariants)**:
  - Chỉ nhân vật chính (`typ == 2`) mới được bắt (Sủng vật không thể bắt).
  - Quy tắc cấp độ: Độ chênh lệch cấp độ giữa nhân vật và võ tướng/NPC không được vượt quá 5 cấp (`npc.lv - player.lv <= 5`).
  - NPC không mang cờ cấm bắt (`bat == 0` trong `Npc.dat`), không phải người chơi, Sủng vật có chủ hoặc khí cụ.
  - Túi Sủng vật tùy thân phải còn ô trống (< 4 con) và chưa sở hữu võ tướng cùng `template_id`.
  - Máu NPC phải dưới 50% MaxHP.
  - Nếu NPC có thuộc tính đặc biệt (`FUN_00625128 == 0x16`), người chơi bắt buộc phải đã học kỹ năng Triệu gọi `14026`.

### Hợp kích / Combo (Battle Combo)
- **Định nghĩa**: Cơ chế hợp lực liên hoàn trong chiến đấu theo lượt khi 2 hoặc nhiều đấu sĩ cùng phe hành động liên tiếp nhắm vào cùng một mục tiêu, kích hoạt chuỗi diễn hoạt hội đồng (mã giả client `20006`/`20007`) và nhân dồn sát thương $\times 1.3$ cho mỗi mắt xích.
- **Ràng buộc (Invariants)**:
  - Combo được quyết định **100% phía Server**; client hoàn toàn thụ động hiển thị theo chuỗi record `32 01`.
  - Tiêu chí kỹ năng tham gia hợp kích: **Phụ thuộc 100% vào thuộc tính `skill_type == 1` (`Active`)** hoặc Đòn đánh thường (`10000`/`0`). Các kỹ năng Active nhiều mục tiêu (như Loạn kích `13013` 4 mục tiêu, Hỏa tiễn `12003` 3 mục tiêu, Ném đá `10005` 3 mục tiêu, Dương viêm `12031` 2 mục tiêu) đều được phép tham gia và kết nối combo bình thường.
  - Các kỹ năng có `skill_type != 1` (Phép thuật/Buff `skill_type == 2` như Phi sa, Hồng thủy, Thiên băng; Debuff `skill_type == 3` như Băng phong, Cây tinh; Hồi máu `skill_type == 4` như Thanh lưu) **bị vô hiệu hóa combo hoàn toàn**.
  - Hệ số dồn sát thương $\times 1.3$ áp dụng cho ô mục tiêu chính được khóa chung (`row_attack`, `col_attack`).

### Chuỗi Hợp kích & Ngưỡng Agi Chuỗi (Combo Chain & Agi Span)
- **Định nghĩa**: Một chuỗi liên kích gồm các đấu sĩ cùng phe xếp kề nhau trong thứ tự lượt đấu (`Agi DESC`) cùng tấn công một mục tiêu còn sống.
- **Ràng buộc (Invariants)**:
  - **Ngưỡng Agi Chuỗi (Agi Span)**: Chênh lệch Nhanh nhẹn giữa đấu sĩ có Agi cao nhất và thấp nhất trong toàn bộ chuỗi combo **không được vượt quá 11 điểm** ($\text{Max}(\text{Agi}_{\text{chuỗi}}) - \text{Min}(\text{Agi}_{\text{chuỗi}}) \le 11$).
  - Phạm vi áp dụng theo **tập hợp tham gia thực tế** của từng chuỗi combo; không khóa cứng toàn đội nếu các thành viên khác tấn công mục tiêu khác hoặc có Agi nằm ngoài chuỗi.

### Cấp Độ Trung Bình Đội Hình & Level Hiệu Dụng Trùng Sinh (LVTB & Reborn Effective Level)
- **Định nghĩa**: Chỉ số cấp độ trung bình của một phe trong trận đấu, được tính một lần ở đầu hiệp đấu: $\text{LVTB} = \frac{\sum \text{Level\_Hiệu\_Dụng}}{\text{Tổng số đấu sĩ trên sân}}$.
- **Ràng buộc (Invariants)**:
  - **Level Hiệu Dụng Trùng Sinh (Reborn Effective Level)**: Bất kỳ đấu sĩ nào đã Trùng Sinh (`reborn > 0`) **bắt buộc được tính là Level 200** khi tính toán LVTB phe (`avg1`/`avg2`), bất kể cấp độ hiển thị thực tế là bao nhiêu. Áp dụng cho toàn bộ các trận đấu (cả PvE và PvP). Đấu sĩ chưa Trùng Sinh tính theo `lv` thực tế.
  - **Ngưỡng Tỷ lệ Hợp kích 100%**: Khi $\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} \ge 25 \implies$ Tỷ lệ kích hoạt combo đạt $100\%$.
  - **Đường cong suy giảm khi thiếu cấp**: Khi $\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} < 25$, tỷ lệ combo tính theo:
    $$\text{Tỷ lệ (\%)} = \text{clamp}\!\left( \left\lfloor \frac{\text{LVTB}_{\text{team}} - 25}{\text{Level}_{\text{quái}}} \times 100\% \right\rfloor, 0, 100 \right)$$
    (Nếu $\text{LVTB}_{\text{team}} \le 25 \implies 0\%$).

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
  - Nhân vật mới: `Lv=1`, `Point/SkillPoint = base + (Lv-120)/5`, `Hp/Sp=181`, stats=0, `Texp` reset in-memory; `Reborn` tăng, `Job` đổi theo menu (reborn 2).
  - Chỉ giữ kỹ năng đặc biệt (10016-19, 11016-19, 12016-19, 13015-18); `DELETE FROM Skill` scope theo `player_id`.
  - Tail: replay `OnWin` quest hiện tại, cập nhật quest step NPC 59411, gửi `F444…F476`, rồi đóng socket.

### Pet Reborn (Hồi sinh Sủng vật) — opcode 0x2C
- **Định nghĩa**: Tiêu 1 đơn vị vật phẩm `RbPetFrom→RbPetTo` để biến đổi Sủng vật về NPC mới: level 1, skill từ NPC (skill 10016/11016/12016/13015 lv 10), bonus theo mốc 30/60.
- **Ràng buộc (Invariants)**:
  - Bonus point `(lv - threshold)/5` phân bổ theo **weighted random** `GetRandomPointPet` (theo 6 stat NPC, 7 lần `.NET` draw/điểm) — không deterministic.
  - `HpMax/Spmax` tính từ stat **gốc** (trước bonus) với mapping `getPetHpMax` (rb 0/1→`getHpMax(0)`, rb 2→`getHpMax(1)`).
  - Phát broadcast map `0F02`/`0F01` + `SendStatusPet` + `06001301` + `2C01`; guards fail → silent.

### ThingData (Dữ liệu Vật phẩm Chuẩn hóa 35 Bytes)
- **Định nghĩa**: Cấu trúc nhị phân 35-byte đại diện toàn hiện cho một vật phẩm (Item) trong game TS Online theo chuẩn Mobile & PC. Bao gồm 20 trường thuộc tính: mã vật phẩm (`id`), số lượng (`count`), chỉ số sát thương (`damage`), kháng cự (`defend`), hệ thuộc tính (`element`), ngọc khảm (`gem`), cấp cường hóa (`enhance`), cấp linh vũ khí (`grow`), thời gian hết hạn (`delete_time` theo chuẩn OADate), trạng thái khóa (`is_lock`), thuộc tính phụ (`attribute`), cấp tinh luyện (`refine`), độ bền (`durability`), cùng các đặc tính dòng ngọc và tẩy luyện.
- **Ràng buộc (Invariants)**: Kích thước mã hóa nhị phân luôn cố định đúng 35 bytes (`THING_DATA_SIZE = 35`).
- **Phạm vi persist (ADR 0001 defer)**: Chỉ 3/20 trường (`item_id`, `quantity`, `damage`) hiện được lưu xuống `inventories` (migration 0001_init.sql:119-145). 17 trường còn lại (`item_level, int1..fai2, item_hp, item_sp, item_type, item_element, item_element_value, grow_exp`) sống **in-memory only** trong `Session` và không qua restart. Khi cần persist trở lại → tạo migration mới.

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
- **Định nghĩa**: Tiến trình nền (Background Task) chạy định kỳ mỗi 3 phút trong Tokio runtime, sử dụng thuật toán băm FNV-1a để phát hiện những phiên người chơi có trạng thái biến động (dirty state: vàng, HP/SP, chỉ số, túi đồ, võ tướng, nhiệm vụ) và thực hiện ghi vào cơ sở dữ liệu SQLite theo từng Transaction nguyên tử.

### Map & Spatial Position (Bản đồ & Tọa độ)
- **Định nghĩa**: Không gian tọa độ thế giới game nơi các Nhân vật di chuyển, tương tác với NPC và kích hoạt các sự kiện/trận đấu.

### Tân thủ / Newbie (Trạng thái tân thủ)
- **Định nghĩa**: Cờ đếm `characters.newbie` (`BIGINT NOT NULL DEFAULT 0`, ánh xạ `Session.newbie: u32`) đánh dấu người chơi đã nhận gói quà tân thủ hay chưa. Giá trị `1` chặn nhận lại gói `TSVN123/TSVN456` (guard `AlreadyGifted` trong `src/db/item_code.rs:116`).
- **Ràng buộc (Invariants)**: `0` = chưa nhận, `1` = đã nhận quà tân thủ. Persist qua `src/db/persist.rs:49` (`"newbie" => "newbie"`) và `src/db/modern/sqlite/session.rs:79,261`.
- **Phạm vi hiện tại (sau ADR 0001 cleanup)**: Sách 46238 chỉ tăng `newbie` (đã xóa `Spx2+50` / `SpMax+50` và stat frame `0xD0`).
- **Tránh dùng các từ mơ hồ**: *tanthu* (tên legacy tiếng Việt không dấu, đã đổi thành `newbie` thống nhất).

### Web Admin Dashboard (Hệ thống Quản trị Web)
- **Định nghĩa**: Bounded Context vận hành & giám sát (Operations) cho phép Quản trị viên theo dõi số lượng người chơi online, xem log gói tin realtime (SSE) và điều khiển trạng thái server (Start/Stop).

### Deferred Drift (Đã Cleanup Sau ADR 0001)
- **Định nghĩa**: Tập hợp các trường/cột schema thuộc `characters`/`inventories`/`character_pets`/`character_missions` mà **migration 0001_init.sql chưa định nghĩa** nhưng source Rust vẫn tham chiếu. Sau cleanup, các tham chiếu này đã bị xóa khỏi Rust (giữ in-memory hoặc xóa hoàn toàn); migration **không** được sửa.
- **Ràng buộc (Invariants)**:
  - `Session` struct vẫn giữ các field deferred (`int2, atk2, def2, hpx2, spx2, agi2, texp, god, hp_store, sp_store, tiengtam, gocnhin, pk, tham_chien, savemap, color, fight_npc_id, title_id`) cho wire packet + in-memory state, **nhưng persist về DB không còn thực hiện** — restart = mất state.
  - `quest_steps`/`warp_steps` đã bị xóa khỏi `Session` — toàn bộ quest state mất khi restart. Persist lại cần `character_missions.npc_id/warp_id` (migration mới).
  - `PetState.int2/atk2/def2/hpx2/spx2/agi2` (in-memory, set = 0 tại load).
  - `Session.newbie` là **DUY NHẤT** field còn persist từ deferred list (vì cần cho TSVN gift guard).
- **Trường đã xóa hoàn toàn khỏi Rust**: `db::players` (file orphan), `db::quest` (file orphan), `src/server/handlers/quest::save_map` (chỉ còn in-memory, comment ghi rõ).

### Combat Damage Formula
- **Định nghĩa**: Công thức tính sát thương trong `src/battle/combat_formula.rs::calculate_attack` (đổi tên từ `mobile_damage.rs` theo ADR 0002). Áp dụng cho cả PC client lẫn mobile client: hit roll `(90 + (agi_atk - agi_def) / 2).clamp(5,99)`, base damage từ `int1` (skill thuộc tính `attribute=27`) hoặc `atk` (basic) với multiplier `0.5`/`0.6` × `lv`, sau đó nhân với element multiplier (1.5× khắc, 0.75× bị khắc, 1.0× đồng/hỗn), reborn multiplier `1+0.1×(reborn_atk-reborn_def)`, random factor `0.9..1.1`, thunder roll `5+agi/10` với multiplier 1.5×, PvP multiplier 0.5× nếu target là player, status multiplier 1.5× nếu bị đóng băng, defend multiplier 0.5× nếu có skill 17001.
- **Ràng buộc (Invariants)**:
  - Công thức này là **damage formula chính thức cho PC server** theo ADR 0002, bất kể client là PC hay mobile. Không dùng `damage::calc_physical_damage` / `damage::calc_magic_damage` (đã xóa sau cleanup).
  - Status infliction: nếu skill có `hit_status > 0 && round > 0`, land roll `(50 + (int1_atk - int1_def)/2).clamp(10,90)`, target sạch (`type3_id == 0`) mới nhận status.
- **Phạm vi áp dụng**: physical attack, magic attack, shield (20006) rear damage — gọi từ `runner.rs::apply_physical`/`apply_damage_to_rear`/`apply_magic`.
- **Tránh dùng các từ mơ hồ**: *mobile damage formula*, *PC damage formula* (đã gộp thành một), *Formula.Dat formula* (chỉ áp dụng cho PC classic, không dùng).

### Skill Catalog (Danh mục kỹ năng)
- **Định nghĩa**: Bảng tra cứu metadata kỹ năng (`BinarySkillDef` trong `data/tables.rs:206`) load từ `Data/Skill.Dat` (PC binary, ưu tiên theo ADR 0002) hoặc `Data/Skill_C.dat` (mobile binary, fallback). Dùng cho: damage calculation (element, numerical, attribute, round, hit_status), targeting (fight_area), status infliction.
- **Ràng buộc (Invariants)**:
  - Hai file **khác format hoàn toàn**:
    - **PC `Skill.Dat`** (Pack=1, 86 bytes/record, 86-byte zero header): reverse-decoded name/des, `DecodeItem8/16/32` XOR obfuscation. Spec từ `SkillData.cs`/`SkillInfo.cs`. Parser ở `src/data/loaders/skill_pc.rs::SkillDatLoaderPc`.
    - **Mobile `Skill_C.dat`** (count-prefix, variable-length UTF-16LE names): parser ở `src/data/loaders/skill.rs::SkillDatLoader` (tham khảo từ Kotlin `ts_mobile_server`).
  - Field `binary_skills: Option<&HashMap<u16, BinarySkillDef>>` trong `BattleData` được fill bởi `BattleService` từ `GameData::binary_skill_defs` (cùng một HashMap, không phân biệt PC/mobile sau load qua bridge `pc_to_binary`).
  - PC bridge (`skill_pc::pc_to_binary`): map PC fields sang `BinarySkillDef` theo best-effort (vd: `sp_cost` → `require_sp`, `elem` → `element`, `state` → `round`, `delay` → `spend_second`, `require_sk` → pre_skill_id1, `unk21` → pre_skill_id2). Unknown fields zero-fill.
  - VISCII decode: `decode_viscii` route qua `encoding::viscii_to_unicode` (bảng 102-entry VISCII 1.1 với extension `Đ` ở 0xD0/0xDD) — render đầy đủ ký tự tiếng Việt có dấu (ế, ộ, ặ, ẫ, ẩ, ằ, ắ, ọ, ợ, ử, ữ, ị, ể, ễ, ẽ...).
  - **PC `Skill.Dat` đã parse được** (từ ADR 0002 follow-up #1): 352 records, id range 9984..=23019, tất cả đều có name (VISCII) + description (VISCII) + require_sk. Skill 10000 (basic attack) tồn tại với name "Nham quái".
- **Tránh dùng các từ mơ hồ**: *mobile skills* (gây hiểu nhầm là chỉ dùng cho mobile), *Skill.dat* (chưa rõ PC hay mobile — phải ghi rõ).

### Opcode Dialect (Đã đơn giản hóa sau ADR 0002)
- **Định nghĩa**: PC server chỉ phục vụ một dialect duy nhất — PC `aLogin.exe` port 6414. Không còn enum `ProtocolProfile` (`PcALogin` / `KotlinMobile`) — đã xóa theo ADR 0002. Mọi opcode đều dùng PC table semantic; opcode nào chưa implement thì rơi vào `unimplemented::handle` (chỉ log, không phản hồi).
- **Ràng buộc (Invariants)**:
  - `FROZEN_PC_COLLISIONS` const đã xóa — khái niệm "collision" chỉ có nghĩa khi có 2 dialect, không áp dụng cho PC-only.
  - 3 opcode (`0x19/0x1B/0x1F`) hiện rơi vào `unimplemented::handle` vì chưa port PC handler — không phải "compat", mà là "chưa viết". `0x23` **không** còn ở đây: nó là account-management (`OP_ACCOUNT`: đổi mật khẩu / xóa nhân vật / gift code), "Guild" trong bảng opcode PC chỉ là nhãn đặt sai. Cùng họ nhãn sai đã chốt rename: `0x24 OP_JOB_CHANGE`, `0x25 OP_LOGIN_COMPLETE`, `0x26 OP_EXP_LEVEL`, `0x27 OP_RANK_ANNOUNCE`, `0x2A OP_RESET` (xem `.scratch/opcode-rename/spec.md`).
- **Tránh dùng các từ mơ hồ**: *Protocol Profile*, *Kotlin dialect*, *PC dialect* (sau refactor không còn khái niệm này).
- **Phạm vi hiện tại**: Kotlin `ts_mobile_server/` chỉ là source tham chiếu để port logic, không bao giờ được dịch ra wire từ Rust server.

### Quân hàm (Rank)
- **Định nghĩa**: Cấp bậc quân sự của Nhân vật (Giáp binh → … → Đại tướng quân, 42 bậc), xác định bởi điểm chiến công (honor) tích lũy. Mỗi bậc cho bộ 4 bonus thuộc tính `(kind, value)` tra cứu theo kiểu `RankData.GetAttribute(honor, kind)` (ví dụ cộng EquipMaxHp/EquipMaxSp).
- **Ràng buộc (Invariants)**:
  - Nguồn catalog: PC `Rank.Dat` (43 records cố định 43B, record 0 dummy), key là index 1-based.
  - Ngưỡng honor tăng dần từ 15 (bậc 1) đến 2000 (bậc cao nhất).
- **Tránh dùng các từ mơ hồ**: *Level*, *Reborn* (cấp độ/đổi nghề khác với quân hàm).

### Địa chất bản đồ (geolBaseAtt)
- **Định nghĩa**: Mã địa chất của một bản đồ, trích từ file `ground.mmg` (mapId → 1 byte), dùng cho chất liệu nền chiến đấu (battle background).
- **Ràng buộc (Invariants)**: Parser tham khảo độc lập (`GroundMmgLoader`), **chưa** đấu vào `GameData::load` vì chưa chắc server cần — **không** nhầm với `DiaHinh` (mã địa hình trận đấu echo trên wire).
- **Tránh dùng các từ mơ hồ**: *DiaHinh*, *Terrain* (khi nói wire packet phải dùng `DiaHinh`).

### Tiền tệ (Currencies: Kim, Tiền trang, Điểm)
- **Định nghĩa**: Hệ thống 3 loại tài nguyên tiền tệ của nhân vật:
  - **Kim mang theo (Gold)**: Lưu tại `player+0x12F8`, trần `GOLD_CAP = 9_999_999`. Sử dụng trong mua bán NPC, mở sạp, giao dịch P2P, tạo quân đoàn. Frame S→C: `1A 01` (+Δ), `1A 02` (-Δ), `1A 04` (=Gold, 6 byte LE).
  - **Tiền trang ngân lượng (Bank Gold)**: Lưu tại `player+0x12FC`, trần `9_999_999`. Rút nộp qua Form `TMoneyAccountForm`. Frame S→C: `1D 01` (+Δ), `1D 02` (-Δ), `1D 04` (=Bank, 6 byte LE), `1D 07/08` (mở khóa nút).
  - **Điểm tích lũy / Tiền mềm (Shop Point)**: Lưu tại `player+0x1300`, trần `2_000_000_000`. Frame S→C: `1A 05` (+Δ), `1A 03` (=Point, 6 byte LE). Không có frame trừ delta S→C.
- **Ràng buộc (Invariants)**:
  - Các phép cộng/trừ delta (`1A 01/02`, `1D 01/02`) từ chối nguyên tử (atomic reject) nếu vượt trần hoặc làm số dư âm.
  - Các phép set tuyệt đối (`1A 04`, `1D 04`, `1A 03`) ghi đè trực tiếp đúng 6 byte payload (`F4440600...`). Tuyệt đối không ghép chung `shop_point` vào frame `1A 04`.

### 5 Container Vật phẩm (Item Containers)
- **Định nghĩa**: Năm không gian chứa vật phẩm chuẩn hóa của nhân vật theo kiến trúc Client Delphi:
  1. **Hòm đồ (`homdo`, storagetype = 1)**: Túi chính 25 ô (`1..=25`). Dump S→C `17 05` (12B/record, có clear loop). Thêm đồ S→C `17 06` (14B, không slot).
  2. **Tiền trang (`tientrang`, storagetype = 2)**: Kho rương lớn 50 ô (`1..=50`). Mảng `player+0xA48`. Dump S→C `1E 01` (12B/record, không có clear pass). Bắt buộc gửi mass-clear `1E 08` trước khi dump. Thêm ô `1E 06`, rút/xóa ô `1E 07 <slot> <qty>`.
  3. **Túi đeo (`tuideo`, storagetype = 4)**: Ba Đậu Yêu Balo 25 ô (`1..=25`). Mảng `player+0xF0C`. Dump S→C `17 2F` (12B/record). Cập nhật `17 30`, xóa ô `17 31 <slot>`.
  4. **Lưu Lãng (`luulang`, storagetype = 16)**: Ba Đậu Yêu Lưu Lãng / Kho xe ngựa `form_Horse` đúng 10 ô (`1..=10`). Mảng `player+0xF74`. Dump S→C `17 66` (12B/record). Cập nhật `17 67`, xóa ô `17 68 <slot>`. Cất C→S `17 33`, Rút C→S `17 34`.
  5. **Trang bị (`trangbi`, storagetype = 8)**: Đồ đang mặc 6 ô (`1..=6`). Dump S→C `17 0B` đúng **10 byte/record, không có byte slot**. Sắp xếp theo thứ tự slot tăng dần.
- **Tránh dùng các từ mơ hồ**: *Túi phụ* (phải phân biệt `tuideo` 25 ô vs `luulang` 10 ô), *Kho đồ* (phải phân biệt `tientrang` 50 ô vs `luulang` 10 ô).

### Sạp hàng người chơi (Player Stall / Vendor)
- **Định nghĩa**: Tính năng người chơi mở quầy bày bán vật phẩm trên bản đồ (`TCY_VenderMenu` / `TSallForm`).
- **Giao thức Wire**:
  - Mở sạp C→S `17 1E`: Dài 100 byte (10 ô × 10B `[slot:u8][qty:u8][price:u32 LE][vitality:u32 LE]`) + chuỗi `boardName` + `cardIdx:u8` + 4B giá thẻ.
  - Phản hồi mở sạp S→C `17 1E`: 3 byte `F4440300171E01` (thành công) hoặc `02/03` (thất bại).
  - Duyệt sạp người khác S→C `17 21`: Header 18 byte (`17 21 <vendorUid:u32> 12_zeros`) + `N` records 16 byte (`[slot:u8][itemId:u16][qty:u8][b4:u8][price:u32][b9:u8][b10:u8][b11:u8][exp:u32]`).
  - Mua sạp C→S `17 21 <vendorUid:u32 LE> <slot:u8> <qty:u8>`: `slot` là 1-based index trên sạp.

### Giao dịch P2P (Player-to-Player Trade)
- **Định nghĩa**: Giao dịch 2 pha trực tiếp giữa hai người chơi (`TTradeForm`, opcode `0x19`).
- **Giao thức Wire**:
  - Tách biệt hoàn toàn kênh Vật phẩm/Kim (`19 01..04`) và kênh Võ tướng (`19 0A..0D`, `19 14..15`).
  - Offer C→S `19 02`: 16 byte (`gold:u32 LE` + 5 cặp `(slot:u8, qty:u8)`).
  - Confirm C→S `19 03`: `action = 1` (Xác nhận), `action = 0` (Hủy bỏ).
  - Hủy giao dịch S→C: Gửi `19 04` (`F44402001904`) đóng form + `19 02 03` (đối tác hủy). Tuyệt đối không dùng `17 3B` (Cắt hàng).
  - Khóa đồng bộ: Áp dụng Two-Phase State Lock (RAM lock theo thứ tự `player_id` tăng dần -> SQLite Write Transaction).

### Shop NPC (NpcShop)
- **Định nghĩa**: Giao dịch mua bán giữa người chơi và NPC thương nhân (`TInterForm` mua, `TThingSaleForm` bán).
- **Giao thức Wire**:
  - Danh mục hàng và giá không đi qua mạng; client tự đọc `Data/Eve.emg` và `Data/Item.Dat`.
  - Mở form mua: S→C `14 01` (`B2 = 6`, `W3 = surfaceIdx`). Không gửi kèm `1B 03`.
  - Mở form bán: S→C `1B 03` (`F44402001B03`).
  - Mua đồ C→S `1B 01`: `index: u8` (0-based) + `count: u16 LE` (1..1250). Áp dụng quy tắc All-or-Nothing; nếu không đủ chỗ trả lỗi `1B 01 02`.
  - Bán đồ C→S `1B 02`: `slot: u8` (1-based `1..25`) + `qty: u8` (1..50).

### Compound / Hợp thành căn bản
- **Định nghĩa**: Tính năng người chơi kết hợp 2 đến 3 nguyên liệu (khoáng thạch, dược phẩm, trang bị phôi) để chế tạo ra vật phẩm mới thông qua bảng công thức `Data/Compound.Dat` (1.393 bản ghi).
- **Ràng buộc (Invariants)**:
  - **Quyền quyết định 100% thuộc Server**: Form `0x0079EE08` trong client chỉ là Developer Simulator offline (zero network calls). Gameplay trực tuyến dùng `form_compound`, gửi yêu cầu lên Server qua Opcode `0x17 Sub 0x39`.
  - **Phạm vi ô chứa (Container Scope)**: Chỉ tiếp nhận nguyên liệu từ `homdo` (Túi chính, slot `1..=25`). Tuyệt đối từ chối các slot ngoài phạm vi hoặc tham chiếu sang `tuideo`/`luulang`.
  - **Kiểm tra ô trống (Probe-First All-or-Nothing)**: Server thẩm định `free_slots >= 1` trước khi trừ nguyên liệu. Nếu `free_slots == 0`, từ chối ngay cả khi nguyên liệu sẽ bị xóa hết (đồng bộ 1-1 với client UI check `FUN_00768D38`).
  - **Cơ chế Fallback khi trượt công thức**: Nếu không khớp bất kỳ công thức nào trong `Compound.Dat`, hệ thống chọn ngẫu nhiên loại (`kind`) của 1 nguyên liệu đầu vào, ép cấp mục tiêu $R_{\text{target}} = 1$ và trả về vật phẩm rác cấp 1 cùng loại trong `Item.dat` (chuẩn Delphi `FUN_0079A890`, không bao giờ mất trắng đồ).
- **Tránh dùng các từ mơ hồ**: *Crafting* (quá chung chung), *Mix đồ* (từ lóng), *Formula.Dat* (bảng sát thương chiến đấu `TCalculator`, không phải công thức hợp thành).

### compound_material / ratA (Chất liệu hợp thành)
- **Định nghĩa**: Mã định danh chất liệu của vật phẩm (giá trị nguyên `1..37`, lưu tại `chunk[46]` của `Data/Item.Dat`). Dùng để sắp xếp tăng dần và ghép chuỗi thập phân tạo khóa `u32` tra cứu trong `Compound.Dat`.
- **Ràng buộc (Invariants)**:
  - Phân biệt tuyệt đối với `chunk[82]` (`raw_material_legacy` / hiển thị phụ trợ). Không được dùng `chunk[82]` thay cho `ratA`.
- **Tránh dùng các từ mơ hồ**: *material* (gây xung đột với `chunk[82]`), *item_type*.

### compound_rank / ratB (Cấp phẩm chất hợp thành)
- **Định nghĩa**: Bậc phẩm chất hợp thành của vật phẩm (giá trị nguyên `1..10+`, lưu tại `chunk[47]` của `Data/Item.Dat`). Dùng để tính bậc phẩm chất trung bình $R_{\text{avg}}$ theo Banker's Rounding và quay số ngẫu nhiên cấp phẩm chất đầu ra $R_{\text{target}} \in [1, R_{\text{avg}} + 1]$.
- **Ràng buộc (Invariants)**:
  - Đại diện cho cấp độ phôi/khoáng thạch (khoáng 11..17 điểm hoặc trang bị lv 25..75).
- **Tránh dùng các từ mơ hồ**: *rank* (tránh nhầm với Quân hàm `Rank.Dat`), *level* (tránh nhầm với `item.level` tại `chunk[83]`).

### Socketing / Cường hóa Khảm (Khảm Lông & Khảm Tâm)
- **Định nghĩa**: Tính năng nâng cấp trang bị bằng cách khảm Lông vũ (+1..+10) hoặc Tâm tăng chỉ số đặc biệt vào trang bị Hòm đồ chính, kết hợp giao diện chọn bảo vật/lông `TFT_SelectFeatherForm` (VMT `0x0058A3EC`, ctor `0x005B0BFC`).
- **Ràng buộc (Invariants)**:
  - Hoàn toàn độc lập với bảng tỷ lệ ngẫu nhiên của `Compound.Dat`.
  - Không sinh ra item mới; tác động trực tiếp lên 2 thuộc tính động `element` (hệ nguyên tố: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong) và `elementvalue` (cấp cường hóa +1..+10) của vật phẩm trong túi đồ (`ThingData` và CSDL SQLite `inventories`).
  - Giao thức chọn lông `TFT_SelectFeatherForm`: Client gửi Opcode `0x23 Subcode 0x03` với `elem_choice` (1..4) khi bấm xác nhận; nếu chưa chọn hệ (`elem_choice == 0`), hiển thị toast *"Hãy chọn lựa một trong các loại bảo vật"*.
- **Tránh dùng các từ mơ hồ**: *Hợp thành lông*, *Compound feather*.

### Scripted Crafting / Chế tạo Kịch bản NPC
- **Định nghĩa**: Luồng chế tạo hoặc đổi vật phẩm thông qua hội thoại NPC (Thợ rèn, Dược sư, Nấu rượu, Event đổi đồ) được điều khiển bởi Động cơ Kịch bản `eve.emg` (Eve Script Engine).
- **Ràng buộc (Invariants)**:
  - Giao tiếp qua Opcode `0x14` (Dialog) $\rightarrow$ `0x01 Sub 0x09` / `0x14 Sub 0x01` $\rightarrow$ `0x18` (Eve Action).
  - Sử dụng công thức cố định theo kịch bản (`CheckItem` $\rightarrow$ `DelItem` $\rightarrow$ `AddItem`), không sử dụng bảng tỷ lệ ngẫu nhiên của `Compound.Dat`.
- **Tránh dùng các từ mơ hồ**: *Compound NPC*, *Shop Crafting*.
