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

### System Alert / OP_SYSTEM_ALERT (Cảnh báo Hệ thống & Liveness ACK)
- **Định nghĩa**: Gói tin thông báo lỗi hệ thống, ngắt kết nối hoặc cảnh báo người chơi từ Server gửi xuống Client (Opcode `0x00`, cấu trúc `F4 44` + độ dài + `0x00` + status `0..57`). Riêng chiều Client → Server, khung payload rỗng `F4 44 01 00 00` được dùng làm **liveness ACK** (server chỉ làm mới `last_seen`, không phản hồi).
- **Ràng buộc (Invariants)**: Mọi cảnh báo/ngắt kết nối chỉ đi một chiều Server → Client (S → C); ở chiều Client → Server chỉ chấp nhận liveness ACK không mang thông điệp nghiệp vụ.

### Server Greeting & Reconnect / Relog Lifecycle (Chào mừng & Vòng đời Đăng nhập / Tái kết nối)
- **Định nghĩa**: Gói tin S → C do Server chủ động gửi ngay sau khi bắt tay TCP thành công (Opcode `0x01`, Sub `0x09`, cấu trúc `F4 44 03 00 01 09 <sceneMode>`). Phía Client (`aLogin.exe`) hoàn toàn thụ động không gửi tin nhắn trước mà chờ gói này để kích hoạt luồng giao diện đăng nhập (đồng thời client gửi phản hồi liveness ACK `F4 44 01 00 00`).
- **Quy cách Phân biệt Clean Login vs Reconnect / Relog**:
  - **Kết nối mới (Clean Login / First Connect)**: Bắt buộc gửi `sceneMode % 100 ∉ [90..99]` (chuẩn là `0x01`, frame `F4 44 03 00 01 09 01`). Khi đó điều kiện thẩm định `FUN_00504c9c` tại client trả về `FALSE`, kích hoạt `TFrmLogin.Show()` mở form đăng nhập trống với con trỏ nhấp nháy để người chơi tự nhập `player_id` và mật khẩu.
  - **Tái kết nối / Đăng nhập lại (Reconnect / Auto-Relog)**: Khi người chơi đã từng đăng nhập trước đó và thông tin tài khoản/mật khẩu đang được lưu trong cấu trúc `TPlayer` (`DAT_0092530c + 0x644`/`+ 0x648`), Server gửi `sceneMode % 100 ∈ [90..99]` (ví dụ `0x5A` = 90 decimal). Khi đó `FUN_00504c9c` trả về `TRUE`, client tự động lấy thông tin từ RAM điền vào form và gọi `FUN_007095c0` submit auth ngay lập tức mà không cần người dùng thao tác.
  - **Cảnh báo lỗi Invariant**: Tuyệt đối **không** gửi `sceneMode ∈ [90..99]` trong lần kết nối đầu tiên khi RAM client chưa có dữ liệu tài khoản (`_LStrLen < 3`), vì client sẽ fail validation trong `FUN_007095c0`, kích hoạt toast lỗi `DAT_007098b8` ("iD không chính xác") và đứng im tại màn hình chọn server.



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
- **Định nghĩa**: Mã định danh "địa hình" và nhạc nền của một trận đấu được truyền trong gói mở bàn cờ `0x0B 0xFA` (giá trị 16-bit LE): byte thấp chọn nền bàn cờ `"FField<N>"` (ví dụ `109`, `112`, `129` lấy từ `geolBaseAtt` của `ground_server.mmg` cho trận Eve/ngộ địch ngẫu nhiên, mặc định `112` khi thiếu dữ liệu hoặc PK/NPC thường; còn các trận đặc thù legacy dùng `4712`, `TeamDef[0]`, `7000`), và byte cao (khi `> 0`) chọn chỉ số `TFight` để phát `BKMusic`.
- **Ràng buộc (Invariants)**: `DiaHinh` **không** ảnh hưởng targeting hay sát thương. Không dùng `SceneInfo.background_no` (chứa Map ID `> 255`) làm `DiaHinh` vì sẽ làm sai `FField` và tràn sang byte cao `BKMusic`.

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

### Trạng thái Khóa Lượt (Turn-Locking Status) & Debuff mềm
- **Định nghĩa**: Phân loại các trạng thái bất lợi trên Ô chiến trận theo quyền điều khiển lượt trong chiến đấu:
  - **Khống chế cứng (Hard Crowd Control / Disable)**: Trạng thái làm đấu sĩ mất hoàn toàn khả năng hành động trong lượt (như Băng phong, Hôn mê, Choáng, Trói cây, Sa bộc, Thanh Long). Đấu sĩ bị bỏ qua lượt hoàn toàn và không thể chọn bất kỳ hành động chiến đấu nào (Đánh thường, Phòng thủ, Dùng vật phẩm, Thi triển kỹ năng, Thu phục, Bỏ chạy).
  - **Hỗn loạn / Cuồng nộ (Berserk / Confusion)**: Trạng thái làm Nhân vật hoặc Sủng vật mất quyền chọn lệnh của người chơi nhưng tự động thực hiện Đánh thường vào một mục tiêu ngẫu nhiên phe địch.
  - **Debuff mềm (Soft Debuff)**: Trạng thái gây sát thương theo lượt hoặc suy giảm chỉ số (như Bỏ độc, giảm Nhanh nhẹn, Thu nhỏ) nhưng **không** tước quyền chọn lệnh chiến đấu của người chơi.
- **Ràng buộc (Invariants)**:
  - Chỉ **Khống chế cứng** và **Hỗn loạn / Cuồng nộ** thuộc nhóm **Trạng thái Khóa Lượt**: người chơi không thể chọn lệnh cho ô đó và pha thu thập lệnh tự động bỏ qua ô bị khóa mà không chờ hết thời gian đếm ngược.
  - **Thứ tự ưu tiên**: Khi một đấu sĩ đồng thời chịu **Khống chế cứng** và **Hỗn loạn / Cuồng nộ**, **Khống chế cứng** luôn thắng — đấu sĩ mất lượt hoàn toàn và không tự động đánh thường.
  - Nếu hệ thống nhận được lệnh chiến đấu cho một ô đang mang **Trạng thái Khóa Lượt**, lệnh bị từ chối đồng bộ, không tiêu hao vật phẩm hay SP và trả về cảnh báo hệ thống cho người chơi.
- **Tránh dùng các từ mơ hồ**: *Debuff* (khi ngữ cảnh thực sự chỉ riêng *Trạng thái Khóa Lượt* hoặc *Khống chế cứng*).

### RNG battle / Luồng ngẫu nhiên trận đấu
- **Định nghĩa**: Ba luồng ngẫu nhiên kiểu .NET (time-seeded, độc lập) của một trận: `random_0` (chọn drop/kỹ năng), `random_1` (tie-break thứ tự lượt + sai số sát thương), `random_2` (tọa độ respawn). Một luồng thứ tư `random_3` ở tầng thế giới lo việc di chuyển NPC tuần tra.
- **Ràng buộc (Invariants)**: Các luồng không được trộn lẫn; thứ tự sử dụng phải khớp thuật toán PRNG 56-phần-tử cố định để replay deterministic.

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
  - Không áp dụng cho **Quái Khoáng (Mineral NPC)**: mọi Quái Khoáng đều bị cấm thu phục (kể cả các NPC ngoại lệ như `22010`, `22011`).
  - **Khởi tạo Sủng vật mới bắt**: Giữ cấp độ `level = 1`, nạp đầy đủ thuộc tính Element, MaxHP, MaxSP, INT, ATK, DEF, AGI và 3-4 kỹ năng mặc định từ bản mẫu NPC (`Npc.dat`), người chơi tự luyện từ cấp 1.

### Quản lý Sủng vật & Giao dịch (Pet Storage & Trade Invariants)
- **Định nghĩa**: Các quy tắc toàn vẹn dữ liệu cho sủng vật trong kho đồ `character_pets` và giao dịch người chơi (P2P Trade).
- **Ràng buộc (Invariants)**:
  - **Thu hồi sủng vật xuất chiến khi giao dịch**: Khi giao dịch thành công một sủng vật đang ở trạng thái Xuất chiến (`active_pet_stt == stt`), server tự động thu hồi (`active_pet_stt = 0`, cập nhật SQLite `character_pets.isactive = 0` và phát frame `13 02` về client người bán).
  - **Đồng bộ khi dùng thẻ triệu hồi sủng vật (`add_pet > 10000`)**: Ngay khi tiêu hao thẻ, server lưu ngay vào SQLite (`upsert_pet`) và gửi ngay gói tin đồng bộ `0F 07` (thông tin sủng) + `0F 01` (hiện diện sủng) về client để sủng hiển thị tức thì trong danh sách.
  - **Kiểm soát sức chứa hòm đồ khi rơi vật phẩm Battle**: Kiểm tra số ô occupied trong `homdo` của người nhận. Nếu đã đạt 25/25 ô, server hủy lượt nạp vật phẩm và gửi thông báo chữ đỏ *"Hành trang đã đầy, không thể nhận vật phẩm"*.

### Quái Khoáng (Mineral NPC) & Vũ khí Cuốc (Pickaxe)
- **Định nghĩa**:
  - **Quái Khoáng (Mineral NPC)**: Thực thể NPC địch trong chiến đấu (`typ == 3` hoặc `7`) thuộc nhóm Khoáng/Thạch/Thủy Tinh/Quặng (`kind == 16`, dải `25001..=25252`) cùng hai NPC đặc biệt `22010` (*Thủy Tinh Quái*) và `22011` (*KhoángChìQuái*). Không áp dụng cho Sủng vật đồng hành (`typ == 4`) của người chơi.
  - **Vũ khí Cuốc (Pickaxe)**: Nhóm vũ khí khai khoáng chuyên dụng gồm đúng 6 vật phẩm (`10001` *Cuốc*, `10530` *Cuốc chân kim*, `10531` *Cuốc tân truyền*, `10532` *Cuốc hòa điền*, `10533` *Cuốc đá vũ hoa*, `10534` *Cuốc nham thạch đỏ*), trang bị tại ô Vũ khí (`equip_pos = 3`) của Nhân vật hoặc Sủng vật.
- **Ràng buộc (Invariants)**:
  - **Điều kiện gây sát thương & hiệu ứng**: Đấu sĩ đang ra đòn (Nhân vật hoặc Sủng vật kiểm tra độc lập trên ô vũ khí của chính mình) bắt buộc phải trang bị một trong 6 Vũ khí Cuốc mới có thể gây sát thương, hiệu ứng khống chế hoặc debuff lên ô Quái Khoáng. Ngược lại, mọi đòn tấn công/kỹ năng gây hại vào ô Quái Khoáng đều rơi vào trạng thái **Miss** (`0` sát thương, không áp dụng hiệu ứng) theo từng mục tiêu và từng người đánh (vẫn tiêu tốn SP và lượt).
  - **Cấm thu phục**: Toàn bộ Quái Khoáng (bao gồm cả `22010` và `22011`) bị cấm thu phục bằng kỹ năng `15001`.
- **Tránh dùng các từ mơ hồ**: *Quái đá*, *Cuốc đào 29335* (đạo cụ không mặc được), *Hoàng Cân Sừ Đầu 10497*.

### Hợp kích / Combo (Battle Combo)
- **Định nghĩa**: Cơ chế hợp lực liên hoàn trong chiến đấu theo lượt khi 2 hoặc nhiều đấu sĩ cùng phe hành động liên tiếp nhắm vào cùng một mục tiêu, kích hoạt chuỗi diễn hoạt hội đồng (mã giả client `20006`/`20007`) và nhân dồn sát thương $\times 1.3$ cho mỗi mắt xích.
- **Ràng buộc (Invariants)**:
  - Combo được quyết định **100% phía Server**; client hoàn toàn thụ động hiển thị theo chuỗi record `32 01`.
  - Tiêu chí kỹ năng tham gia hợp kích: **Phụ thuộc 100% vào thuộc tính `skill_type == 1` (`Active`)** hoặc Đòn đánh thường (`10000`/`0`). Các kỹ năng Active nhiều mục tiêu (như Loạn kích `13013` 4 mục tiêu, Hỏa tiễn `12003` 3 mục tiêu, Ném đá `10005` 3 mục tiêu, Dương viêm `12031` 2 mục tiêu) đều được phép tham gia và kết nối combo bình thường.
  - Các kỹ năng có `skill_type != 1` (Phép thuật/Buff `skill_type == 2` như Phi sa, Hồng thủy, Thiên băng; Debuff `skill_type == 3` như Băng phong, Cây tinh; Hồi máu `skill_type == 4` như Thanh lưu) **bị vô hiệu hóa combo hoàn toàn**.
  - Hệ số dồn sát thương $\times 1.3$ áp dụng cho ô mục tiêu chính được khóa chung (`row_attack`, `col_attack`).
  - **Phần thưởng Vật phẩm & Tiền vàng khi Hợp kích**: Tỷ lệ trúng thưởng rơi đồ giữ nguyên theo bảng xác suất tĩnh (~7.6%). Mỗi ô tham gia hợp kích kết liễu NPC thành công đều sinh một phần thưởng vật phẩm/vàng độc lập nạp vào hòm đồ (`homdo`) của người chơi sở hữu ô đó (sủng vật chuyển quyền nhận về cho chủ nhân do sủng không có homdo).

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
  - Max HP/SP (Hpmax/Spmax): khi cập nhật trong phân bổ Hpx/Spx chỉ cập nhật in-memory — **không phát packet nào** cho Max (`_Hpmax`/`_Spmax`).

### Point / Skill Point (Điểm chỉ số)
- **Định nghĩa**: Điểm có thể phân bổ (allocatable) để tăng chỉ số; `Point` (query trong DB/game) phân biệt với `SkillPoint` (điểm học kỹ năng). Cả hai đều cập nhật dưới dạng opcode 0x08 packet type `0x26` (Point).

### Skill bar / Hotkey (Thanh kỹ năng) — opcode 0x28
- **Định nghĩa**: Bản đồ slot 1..10 gán một kỹ năng vào thanh phím tắt của nhân vật. Nhận dữ liệu client → lưu `SkillSave`; **không phản hồi**. Slot 0 = clear (no-op).

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

### Appearance Color (Màu ngoại hình)
- **Định nghĩa**: Chuỗi 8 byte (biểu diễn bằng 16 ký tự hex viết hoa `0-9A-F`) quy định màu tóc, da, áo, quần, giày của nhân vật. Nhận từ gói tạo nhân vật `0x09 0x01` (`payload[4..12]`) hoặc cập nhật khi `reborn`, tách biệt hoàn toàn với `hair` (1 byte kiểu tóc tại `payload[2]`).
- **Ràng buộc (Invariants)**: Lưu trữ chính thức tại cột `characters.color` (`TEXT NOT NULL DEFAULT '0000000000000000'`) qua migration `0004_character_color.sql`. Giá trị rỗng, sai độ dài hoặc chứa ký tự không phải hex được chuẩn hóa về `"0000000000000000"` (`normalize_color_hex`).

### Map & Spatial Position (Bản đồ & Tọa độ)
- **Định nghĩa**: Không gian tọa độ thế giới game nơi các Nhân vật di chuyển, tương tác với NPC và kích hoạt các sự kiện/trận đấu.

### Proximity Gate & Walk Interpolation (Kiểm tra khoảng cách & Nội suy bước đi)
- **Định nghĩa**: Cơ chế đồng bộ vị trí tức thời của nhân vật khi di chuyển (`0x06 0x01` gửi tọa độ đích ngay lúc click) bằng nội suy tuyến tính từ `(walk_origin, walk_dest, walk_start_ms)` theo tốc độ client: `160 px/s` (`0.16 px/ms`) ở bản đồ thường và `128 px/s` (`0.128 px/ms`) ở bản đồ thế giới (`map_id % 1000 == 0`).
- **Ràng buộc (Invariants)**: Kiểm tra khoảng cách tương tác (`14 01`, `14 04`, `14 08`) chuẩn hóa thống nhất theo bán kính **Euclidean `130 px`** ($\sqrt{\Delta x^2 + \Delta y^2} \le 130$) trên vị trí nội suy; kích hoạt cửa (`MeetDoor`) dùng timer hủy được thay vì kiểm tra tức thời tại thời điểm click.

### Tân thủ / Newbie (Trạng thái tân thủ)
- **Định nghĩa**: Cờ đếm `characters.newbie` (`BIGINT NOT NULL DEFAULT 0`, ánh xạ `Session.newbie: u32`) đánh dấu người chơi đã nhận gói quà gift code `TSVN123/TSVN456` hay chưa (đồng thời sách `46238` tăng giá trị đếm). Giá trị `1` chặn nhận lại gói `TSVN123/TSVN456` (guard `AlreadyGifted` trong `src/db/item_code.rs:116`).
- **Ràng buộc (Invariants)**: `0` = chưa nhận, `1` = đã nhận gift code `TSVN123/TSVN456`. Persist qua `src/db/persist.rs:49` (`"newbie" => "newbie"`) và `src/db/modern/sqlite/session.rs:79,261`.
- **Phạm vi hiện tại (sau ADR 0001 cleanup)**: Sách 46238 chỉ tăng `newbie` (đã xóa `Spx2+50` / `SpMax+50` và stat frame `0xD0`). Tách biệt hoàn toàn với **Đồ mặc định tạo nhân vật** (bao gồm `31007` *Vé đoái hoán tâm* cấp sẵn khi tạo nhân vật).
- **Tránh dùng các từ mơ hồ**: *tanthu* (tên legacy tiếng Việt không dấu, đã đổi thành `newbie` thống nhất).

### Đồ mặc định tạo nhân vật (Character Starter Items)
- **Định nghĩa**: Bộ vật phẩm và trang bị được cấp cố định một lần ngay tại thời điểm khởi tạo nhân vật mới (`0x09 Sub 01`), gồm `32012 × 4` (hòm đồ ô `1`), `31007 × 1` (*Vé đoái hoán tâm*, hòm đồ ô `2`) và trang bị `19737 × 1` (trang bị ô `2`).
- **Ràng buộc (Invariants)**: Chỉ cấp lúc tạo nhân vật mới (`characters.newbie` vẫn khởi tạo `0`); không kiểm tra hay cấp bù khi nhân vật cũ Enter Game.

### Web Admin Dashboard (Hệ thống Quản trị Web)
- **Định nghĩa**: Bounded Context vận hành & giám sát (Operations) cho phép Quản trị viên theo dõi số lượng người chơi online, xem log gói tin realtime (SSE) và điều khiển trạng thái server (Start/Stop).

### Deferred Drift (Đã Cleanup Sau ADR 0001)
- **Định nghĩa**: Tập hợp các trường/cột schema thuộc `characters`/`inventories`/`character_pets`/`character_missions` mà **migration 0001_init.sql chưa định nghĩa** nhưng source Rust vẫn tham chiếu. Sau cleanup, các tham chiếu này đã bị xóa khỏi Rust (giữ in-memory hoặc xóa hoàn toàn); migration **không** được sửa.
- **Ràng buộc (Invariants)**:
  - `Session` struct vẫn giữ các field deferred (`int2, atk2, def2, hpx2, spx2, agi2, texp, hp_store, sp_store, tiengtam, gocnhin, pk, tham_chien, savemap, fight_npc_id, title_id`) cho wire packet + in-memory state, **nhưng persist về DB không còn thực hiện** — restart = mất state.
  - `quest_steps`/`warp_steps` đã bị xóa khỏi `Session` — toàn bộ quest state mất khi restart. Persist lại cần `character_missions.npc_id/warp_id` (migration mới).
  - `PetState.int2/atk2/def2/hpx2/spx2/agi2` (in-memory, set = 0 tại load).
  - `Session.newbie`, `Session.god`, `Session.ghost`, và `Session.color` đã được định nghĩa cột chính thức trong `characters` và **persist đầy đủ xuống SQLite**.
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
    - **PC `Skill.Dat`** (Pack=1, 86 bytes/record, 86-byte zero header): reverse-decoded name/des, `DecodeItem8/16/32` XOR obfuscation. Parser ở `src/data/loaders/skill_pc.rs::SkillDatLoaderPc`.
    - **Mobile `Skill_C.dat`** (count-prefix, variable-length UTF-16LE names): parser ở `src/data/loaders/skill.rs::SkillDatLoader`.
  - Field `binary_skills: Option<&HashMap<u16, BinarySkillDef>>` trong `BattleData` được fill bởi `BattleService` từ `GameData::binary_skill_defs` (cùng một HashMap, không phân biệt PC/mobile sau load qua bridge `pc_to_binary`).
  - PC bridge (`skill_pc::pc_to_binary`): map PC fields sang `BinarySkillDef` theo best-effort (vd: `sp_cost` → `require_sp`, `elem` → `element`, `state` → `round`, `delay` → `spend_second`, `require_sk` → pre_skill_id1, `unk21` → pre_skill_id2). Unknown fields zero-fill.
  - VISCII decode: `decode_viscii` route qua `encoding::viscii_to_unicode` (bảng 102-entry VISCII 1.1 với extension `Đ` ở 0xD0/0xDD) — render đầy đủ ký tự tiếng Việt có dấu (ế, ộ, ặ, ẫ, ẩ, ằ, ắ, ọ, ợ, ử, ữ, ị, ể, ễ, ẽ...).
  - **PC `Skill.Dat` đã parse được** (từ ADR 0002 follow-up #1): 352 records, id range 9984..=23019, tất cả đều có name (VISCII) + description (VISCII) + require_sk. Skill 10000 (basic attack) tồn tại với name "Nham quái".
- **Tránh dùng các từ mơ hồ**: *mobile skills* (gây hiểu nhầm là chỉ dùng cho mobile), *Skill.dat* (chưa rõ PC hay mobile — phải ghi rõ).

### Opcode Dialect (Đã đơn giản hóa sau ADR 0002)
- **Định nghĩa**: PC server chỉ phục vụ một dialect duy nhất — PC `aLogin.exe` port 6414. Không còn enum `ProtocolProfile` (`PcALogin` / `KotlinMobile`) — đã xóa theo ADR 0002. Mọi opcode đều dùng PC table semantic; opcode nào chưa implement thì rơi vào `unimplemented::handle` (chỉ log, không phản hồi).
- **Ràng buộc (Invariants)**:
  - `FROZEN_PC_COLLISIONS` const đã xóa — khái niệm "collision" chỉ có nghĩa khi có 2 dialect, không áp dụng cho PC-only.
  - Các opcode `0x19 OP_TRADE`, `0x1B OP_NPC_SHOP`, `0x1F OP_PET_HOTEL` đã có handler đầy đủ trên đường dẫn PC; `0x23 OP_ACCOUNT` xử lý account-management (đổi mật khẩu / xóa nhân vật / gift code / `SelectFeatherForm`), `0x1A OP_MONEY_SYNC` là kênh S→C thuần. Các hằng số đã chuẩn hóa theo handler thực tế gồm: `0x03 OP_ENTER_GAME`, `0x10 OP_QUIZ`, `0x18 OP_QUEST_SYNC`, `0x1C OP_SKILL_LEARN`, `0x20 OP_EXPRESSION`, `0x24 OP_ARENA_FESTIVAL`, `0x27 OP_GUILD`, `0x29 OP_CITY_SIEGE`, `0x2B OP_GUILD_ALLIANCE`, `0x41 OP_RANK`.
- **Tránh dùng các từ mơ hồ**: *Protocol Profile*, *Kotlin dialect*, *PC dialect* (sau refactor không còn khái niệm này).
- **Phạm vi hiện tại**: Server chỉ phát và tiếp nhận wire protocol chuẩn PC `aLogin.exe`.

### Quân hàm (Rank)
- **Định nghĩa**: Cấp bậc quân sự của Nhân vật (Giáp binh → … → Đại tướng quân, 42 bậc), xác định bởi điểm chiến công (honor) tích lũy. Mỗi bậc cho bộ 4 bonus thuộc tính `(kind, value)` tra cứu theo kiểu `RankData.GetAttribute(honor, kind)` (ví dụ cộng EquipMaxHp/EquipMaxSp).
- **Ràng buộc (Invariants)**:
  - Nguồn catalog: PC `Rank.Dat` (43 records cố định 43B, record 0 dummy), key là index 1-based.
  - Ngưỡng honor tăng dần từ 15 (bậc 1) đến 2000 (bậc cao nhất).
- **Tránh dùng các từ mơ hồ**: *Level*, *Reborn* (cấp độ/đổi nghề khác với quân hàm).

### Địa chất bản đồ (geolBaseAtt)
- **Định nghĩa**: Mã địa chất 1 byte (`u8`) của một bản đồ trích từ `ground_server.mmg` (tệp rút gọn cho server từ `ground.mmg` của client, giữ `width`, `height` và `geolBaseAtt` theo cấu trúc `MapData.New`), biểu thị trực tiếp mã nền chiến đấu `"FField<N>"` (như `109`, `112`, `129`).
- **Ràng buộc (Invariants)**: Được nạp vào `GameData` qua `GroundMmgLoader` làm nguồn xác định byte thấp của `DiaHinh` cho các trận chiến Eve và ngộ địch ngẫu nhiên (fallback `112` khi bản đồ không có bản ghi).
- **Tránh dùng các từ mơ hồ**: *Terrain*, *SceneInfo.background_no* (khi nói gói tin chiến đấu phải dùng `DiaHinh`).

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

### Thần Tài / Phúc Thần (God of Fortune — `FGod`)
- **Định nghĩa**: Trạng thái chúc phúc của Nhân vật lưu số trận chiến dã ngoại còn hiệu lực (`god`, tối đa 240 trận), hiển thị biểu tượng Thần Tài bay lơ lửng trên đầu avatar.
- **Ràng buộc (Invariants)**:
  - Mỗi trận thắng PvE dã ngoại nhân kinh nghiệm nhận được với hệ số $1.5\times$ và trừ `god` đi 1 trận.
  - Khi chuyển trạng thái giữa `0` và `> 0`, đồng bộ thời gian thực qua gói `S→C 18 08` (`kind = 2`, `val = 1` hoặc `0`) và truyền `(god > 0) as u8` tại byte 6 của gói `player_appear` (`0x03`).
  - Lưu bền vững qua các lần đăng nhập và chuyển bản đồ.

### Con Ma / Thần Xui (Ghost Debuff — `FGhost`)
- **Định nghĩa**: Trạng thái trừng phạt đeo bám Nhân vật (`ghost == 1`), hiển thị biểu tượng Con Ma xanh lam trên đầu avatar và giảm $50\%$ kinh nghiệm ($0.5\times$ EXP) trong mọi trận chiến PvE cho đến khi được giải trừ.
- **Ràng buộc (Invariants)**:
  - Không tự giảm theo số trận chiến; chỉ được hóa giải khi tương tác với NPC "Bà Gọi Hồn" (`1752`, phí 500 Kim) hoặc sử dụng đạo cụ "Cải vận phù" (`46180`). Sủng vật "Bà Gọi Hồn" không có khả năng giải trừ.
  - Đồng bộ thời gian thực qua gói `S→C 18 08` (`kind = 1`, `val = 1` hoặc `0`; khi về `0` client phát âm thanh `WA0006.wav` và thông báo *"Thần xui đã rời xa bạn!"*) và truyền tại byte 5 của gói `player_appear` (`0x03`).
  - Lưu bền vững qua các lần đăng nhập và chuyển bản đồ.

### Vấn Đáp Bắc Tinh Quân (North Star Quiz Encounter) — `OP_QUIZ` (`0x10`)
- **Định nghĩa**: Trận chiến PvE đặc biệt kích hoạt ngẫu nhiên khi di chuyển trên bản đồ dã ngoại trong thời gian sự kiện mở, đối diện Bắc Tinh Quân (`1643`) hoặc Nam Tinh Quân (`1644`) với hộp thoại trắc nghiệm 4 đáp án (`OP_QUIZ` `0x10`) trong 45 giây.
- **Ràng buộc (Invariants)**:
  - Cấm tuyệt đối người chơi ngoài trận vào quan chiến (từ chối bằng `S→C 21 01 03`).
  - Khóa toàn bộ thao tác chiến đấu thường; chỉ Đội trưởng nhận câu hỏi, trả lời và chịu phần thưởng (`+10` trận Thần Tài) hoặc hình phạt (rút máu hiện tại về `1` qua thuộc tính `0x19` và gán Con Ma). Các thành viên tổ đội đi cùng giữ nguyên trạng thái độc lập.

### Quân Đoàn (`OP_GUILD` `0x27`) & Công Thành Chiến (`OP_CITY_SIEGE` `0x29`)
- **Định nghĩa**: Tổ chức liên minh người chơi có Quân Chủ đứng đầu, danh sách thành viên, phân cấp chức vụ, huy hiệu nhị phân 288-byte và quyền tham gia Công Thành Chiến (`OP_CITY_SIEGE` `0x29`) / Thủy Chiến (`0x3F`, `0x40`).
- **Ràng buộc (Invariants)**:
  - Gia nhập Quân Đoàn bắt buộc qua quy trình 2 bước có sự đồng thuận: người mời gửi lời mời (`0x27 0x02`), người được mời phải phản hồi chấp thuận (`0x27 0x03`) mới trở thành thành viên chính thức.
  - Chỉ Quân Chủ mới có quyền trục xuất thành viên hoặc chuyển nhượng chức vị Quân Chủ. Mọi tương tác giữa 2 người chơi đều phải khóa đồng bộ hai pha.

### Túi Chiến Thuyền (Boat Inventory)
- **Định nghĩa**: Không gian chứa đạo cụ thủy chiến tạm thời gồm 25 ô (`1..=25`), chỉ tồn tại trong phiên tham gia Thủy Chiến (`0x3F`).
- **Ràng buộc (Invariants)**:
  - Khởi tạo qua `S→C 17 64` khi vào bản đồ Thủy Chiến (cấp đạn dược thủy chiến mặc định để khai hỏa Hải Pháo `0x40 0x01`) và xóa sạch hoàn toàn qua `S→C 17 65` khi rời Thủy Chiến hoặc khi sự kiện dừng khẩn cấp.
  - Tách biệt hoàn toàn với 5 Container Vật phẩm vĩnh viễn của Nhân vật; không bao giờ lưu trữ lâu dài sang túi đồ thường.

### Kịch bản Eve (Eve Script Engine — `Data/eve.emg`)
- **Định nghĩa**: Hệ thống sự kiện và nhiệm vụ chính thống duy nhất của game (`Data/eve.emg`, `3,901` bản đồ/scene), điều khiển hội thoại NPC (`ClickNpc`), cửa chuyển cảnh (`ClickDoor` / `MeetDoor`), menu lựa chọn (`Surface`), chuỗi tự nối (`AutoChain`) và trận chiến nhiệm vụ qua `0x14` / `0x18` / `0x0B` / `0x0C`.
- **Ràng buộc (Invariants)**:
  - **Single Source of Truth**: `Data/eve.emg` là nguồn sự thật duy nhất (bật mặc định ở production); mọi file `.ini` quest legacy không được ghi đè lên kịch bản Eve.
  - **Định danh theo Scene (`scene_id`, `eve_no`)**: Chỉ số `eve_no` và bộ đếm hoàn thành (`character_completed_events`) mang phạm vi cục bộ trong từng `scene_id`, tuyệt đối không đánh khóa toàn cục theo `eventid`.
  - **Định dạng nhị phân `eve.emg`**: Phần `FightData` dùng `win_cond = 13` byte và `fight_limit = 0` byte (không có trường `u32`); mỗi bản ghi `EveResult` trong `eve.emg` dài `14` byte (wire `0x14 01` chèn thêm byte padding khi gửi).
  - **Fail-closed với mã chưa giải mã**: Mọi `condition_class`, `result_type`, `result_class` chưa hỗ trợ bắt buộc đánh giá `false` / bỏ qua có ghi log telemetry; cấm rơi về `0 == 0 -> true`.
- **Tránh dùng các từ mơ hồ**: *Data_Talks*, *Quest INI override* (không còn là nguồn chuẩn).

### Nhật ký Nhiệm vụ (`OP_QUEST_SYNC` — Opcode `0x18`) & Cờ Bit `Mark.dat`
- **Định nghĩa**: Hệ thống đồng bộ trạng thái nhiệm vụ hai lớp của Client (`LocalActor`), hoàn toàn tách biệt với 5 Container Vật phẩm:
  1. **Danh sách Nhiệm vụ Đang làm (Active Quest Log — `0x18 Sub 01/02/04/06`)**: Mảng 200 ô (`slot 1..=200`) tại `LocalActor + 0x654`, mỗi ô lưu `{quest_id: u16, mark_step: u8}` với `quest_id` tra cứu từ **`Data/Mark.dat`** (`10001..12461`), **không** tra cứu `Item.dat`.
  2. **Mảng Bit Cờ Hoàn thành (`quest_dont` — `0x18 Sub 05/07`)**: Mảng 300 byte bitmask (`2,400` bit) tại `LocalActor + 0x8b2`, ánh xạ theo trường `position` (`bit_id 1..=2400`) trong `Mark.dat`.
- **Ràng buộc (Invariants)**:
  - **Không tồn tại túi vật phẩm `0x18` (`quest_items`)**: 100% vật phẩm `Item.dat` (bao gồm đạo cụ nhiệm vụ từ `result_class = 1`) đều đi qua `homdo` (`0x17`, 25 ô) với kiểm tra All-or-Nothing trước khi thực thi. Trên `0x14 01`, `result_type=0, result_class=1` chỉ hiển thị toast từ `Item.dat`, không thêm đồ vào túi.
  - **Bit-packing `0x18 05` vs `0x18 07`**: `S→C 0x18 05` mang `bit_id` (`1..=2400`) để client bật/tắt bit `(bit_id - 1) % 8` tại byte `(bit_id - 1) / 8 + 1`; `S→C 0x18 07` gửi hàng loạt `[byte_idx (1..=300): u16][bitmask: u8]`. Cờ `quest_dont` chỉ được đánh dấu khi nhiệm vụ hoàn tất thực sự (hoặc lệnh đánh dấu tường minh), không đánh dấu ở bước trung gian.
  - **Toàn vẹn ô 1..200**: Cấp phát slot từ `quest_sync` (`1..=200`), tuyệt đối cấm dùng `slot = 0`. Khi đủ 200 ô thì từ chối All-or-Nothing và phát `S→C 0x18 03`.
  - **Chặn tuyệt đối `C→S 0x18 01/02`**: Gói này sinh từ form debug/dev `TMissionForm`; server từ chối hoàn toàn đối với mọi tài khoản kể cả GM.
- **Tránh dùng các từ mơ hồ**: *Kho vật phẩm nhiệm vụ (`quest_items`)*, *Túi quest 200 ô* (dễ gây nhầm với `homdo 0x17`).

### Giao diện Lựa chọn Eve (`0x14 Surface`) vs Vấn đáp Tinh quân (`0x10 Quiz`)
- **Định nghĩa**: Hai cơ chế hiển thị lựa chọn độc lập trên client:
  - **`0x14 Surface` (`result_type = 6`, chọn qua `C→S 0x14 09`)**: Bảng nút chọn hoặc lưới `TInterForm` nằm trong chuỗi hội thoại Eve (bao gồm cả các nhiệm vụ vấn đáp như Giản Ung / Tống Nhân). Handler `0x14 09` bắt buộc lọc chain bằng `ConditionClass::DialogChoice(surface_id, choice_code)`.
  - **`0x10 Quiz` (`TAC_Question` / `TKC_Question`)**: Khung câu hỏi trắc nghiệm đếm ngược 45 giây của sự kiện dã ngoại (như Vấn Đáp Bắc Tinh Quân).
- **Tránh dùng các từ mơ hồ**: Gọi chung là *Quiz* hoặc *Menu* mà không phân biệt kênh `0x14` hay `0x10`.

### Trận chiến Thường (Normal Battle) vs Trận chiến PK (`PKBattle`)
- **Định nghĩa**: Hai phân loại vòng đời của một trận chiến theo nguồn khởi tạo và chính sách phần thưởng:
  - **Trận chiến Thường (`Normal`)**: Các trận PvE từ kịch bản Eve, đụng độ ngẫu nhiên trên bản đồ, *Trận đấu NPC chủ động*, *TeamDef*, sự kiện 40 NPC, và *Vấn Đáp Bắc Tinh Quân*. Trả đầy đủ vật phẩm rơi (Drop), tiền tệ (Gold), điểm kinh nghiệm Nhân vật và Sủng vật, đồng thời tiêu hao lượt *Thần Tài / Phúc Thần* khi thắng.
  - **Trận chiến PK (`PKBattle`)**: Các trận được người chơi chủ động phát động qua chế độ con trỏ PK trên bản đồ, bao gồm cả **PK Người chơi (PvP)** (`0x0B 02 02`, đòi hỏi công tắc PK trong Option của cả hai bên đều bật) và **PK Võ tướng / NPC** (`0x0B 02 03`, chỉ cần dùng con trỏ PK bản đồ click NPC, không phụ thuộc công tắc Option PK).
- **Ràng buộc (Invariants)**:
  - `PKBattle` giữ nguyên toàn bộ luật chiến đấu theo lượt, sát thương (kể cả hệ số PvP 0.5x lên người chơi/sủng vật), buff/debuff, hợp kích, **Thu phục Võ tướng** (`15001`) và cơ chế hồi sinh NPC tuần tra sau trận.
  - `PKBattle` **cấm hoàn toàn** rơi vật phẩm, cộng Gold, cộng kinh nghiệm Nhân vật/Sủng vật, và **không** trừ lượt *Phúc Thần*.
  - Trong khung `0x0B 02 03`, trường 16-bit cuối gói là **chỉ số vị trí NPC trên bản đồ (`1..=100`)**, tuyệt đối không hiểu nhầm thành *DiaHinh*.
  - Giá trị `fight_type` từ kịch bản Eve (`eve.emg`) dùng để phân loại trận sự kiện/hệ thống Eve (bằng `0` với mọi trận ngoài Eve), **không** dùng thay thế cho phân loại `Normal` vs `PKBattle`.
- **Tránh dùng các từ mơ hồ**: *PK Mode* (dễ lẫn giữa công tắc Option PvP và con trỏ PK bản đồ), *fight_type = 0* (không đồng nghĩa với PKBattle).

### Chạy trốn (Flee / Escape) & Vai trò Tham chiến (`0B FA` Marker)
- **Định nghĩa**: Hành động chiến thuật trên *Kênh Lệnh Chiến Đấu* (`0x32 sub 01`) cho phép *Nhân vật* hoặc *Sủng vật* rời khỏi trận đấu trong lượt của mình thông qua lệnh nút thanh công cụ (`18001` — *Chạy trốn*) hoặc kỹ năng chủ động tốn 10 SP (`14002` — *Thuật đào tẩu*). Khi thất bại, hệ thống phát hiệu ứng và thông báo chuẩn bằng mã **`18002`** (*Chạy trốn thất bại*, `skillType = 12`).
- **Ràng buộc (Invariants)**:
  - **Vai trò `0B FA`**: Khung mở bàn cờ gán dấu bản thân `0x03` cho Đội trưởng, `0x05` cho Thành viên tổ đội, và chỉ dành `0x04` cho *ListQS / Người quan sát*. Chỉ *ListQS* mang dấu `0x04` mới được rời trận tức thì qua `0x0B 01 03`; chiến đấu viên bắt buộc dùng *Kênh Lệnh Chiến Đấu* `0x32 01 18001` hoặc `14002`.
  - **Quy tắc tỷ lệ & Phạm vi thoát trận**: Tỷ lệ thành công tính theo chênh lệch cấp độ giữa người thực hiện với mục tiêu đối phương và LVTB hai đội, cộng thêm `+20%` khi dùng *Thuật đào tẩu* (`14002`, không bypass 100%). Nếu Đội trưởng (hoặc *Sủng vật* của Đội trưởng, kể cả ở một trong hai phe của trận PvP) chạy trốn thành công thì toàn bộ trận/đội thoát ngay; nếu Thành viên (hoặc *Sủng vật* của Thành viên) chạy trốn thành công thì chỉ riêng Thành viên đó cùng các *Sủng vật* của họ rời lưới chiến trận, các đồng đội còn lại tiếp tục đánh.
- **Tránh dùng các từ mơ hồ**: *Leave Battle* (chỉ dành cho *ListQS* thoát quan chiến qua `0x0B 01 03`), `14003` (*Thuật mua bán* bị động, không liên quan đến Flee).

### Cửa Bản Đồ (Door) vs Điểm Xuất Hiện (`SceneInfo`) & Dịch Chuyển Cổng (`Talk-Warp 0x14 08`)
- **Định nghĩa**: Hai thực thể định danh tách biệt trong chuỗi chuyển cảnh của kịch bản Eve (`Data/eve.emg`), kết nối qua sự kiện `NpcEvent` có kết quả chuyển cảnh (`result_type == 2`):
  - **`Door ID` (`Door.id`)**: Định danh cổng đặt trên bản đồ hiện tại (`Section 3`). Khi người chơi chạm (`MeetDoor`), nhấp (`ClickDoor`), hoặc gửi xác nhận dịch chuyển `C→S 0x14 08` (`idtalking`), mã mà Client và `Data/Warps.txt` / `Data/BattleGate.txt` dùng là **`Door ID`**.
  - **`SceneInfo ID` (`eve_no`)**: Định danh bản ghi điểm đến (`Section 6`: bản đồ đích, tọa độ tile xuất hiện `(x, y)` quy đổi pixel `tile * 20 - 10`, hướng mặt). Giá trị `parameter` của kết quả `result_type == 2` trỏ tới **`SceneInfo ID`**, không phải `Door ID`.
- **Ràng buộc (Invariants)**:
  - **Không đồng nhất khóa**: `Door ID` và `SceneInfo ID` là hai không gian khóa độc lập cục bộ trong mỗi `scene_id` (nhiều cổng có `Door ID != SceneInfo ID`). Tuyệt đối không dùng `Door ID` để tra trực tiếp vào bảng `SceneInfo`.
  - **Thứ tự phân giải cho `Talk-Warp 0x14 08`**: Bắt buộc đi theo chuỗi `Door ID → NpcEvent (đánh giá đầy đủ điều kiện trên trạng thái người chơi) → result_type == 2 → SceneInfo ID`. Không fallback sang `Warps.txt` nếu `Door` có nhánh `result_type == 2` nhưng người chơi chưa thỏa điều kiện nhiệm vụ/hội thoại; chỉ dùng `Warps.txt` cho các cổng thiếu `result_type == 2` hoặc ngoại lệ override đã phê duyệt.
- **Tránh dùng các từ mơ hồ**: *warp_id* dùng chung không phân biệt `Door ID` hay `SceneInfo ID`; *Warp.Dat* (danh mục truyền tống 28 thành thị lớn gắn `Mark.dat` `11804..11831`, không liên quan tới chuyển cổng bản đồ).

### Cửa Ải Chiến Đấu (BattleGate)
- **Định nghĩa**: Cổng chuyển cảnh có đội quân thủ ải chặn đường (`Data/BattleGate.txt`), yêu cầu người chơi/đội trưởng chiến thắng trận chiến trước khi đi qua.
- **Ràng buộc (Invariants)**:
  - **Nguồn chân lý**: Các cổng đã có trận chiến kịch bản (`result_type == 3`) trong `Data/eve.emg` do kịch bản Eve toàn quyền điều phối; `BattleGate.txt` chỉ giữ lại phần cổng ngoại lệ chưa có `result_type == 3` trong `eve.emg` (như chuỗi tháp `49901..49908` và các cửa ải đặc thù).
  - **Nhất quán giữa `MeetDoor` và `0x14 08`**: Với các cổng còn lại trong `BattleGate.txt`, chốt chặn phải được kiểm tra đồng nhất ở cả luồng va chạm `MeetDoor` và luồng `Talk-Warp 0x14 08`.
  - **Hậu chiến cửa ải**: Khi chiến thắng trận chiến từ `BattleGate.txt`, hệ thống tự động dịch chuyển cả đội qua đích của cổng ngay sau trận và ghi nhận trạng thái đã vượt ải (hoặc chuyển sang bản đồ tầng kế tiếp) để tránh lặp trận vô hạn.
- **Tránh dùng các từ mơ hồ**: *Boss Gate* chung chung hoặc nhầm lẫn với cổng chỉ có điều kiện hội thoại không chiến đấu.

### Sự kiện Đặc biệt (`SpecialEvent`) vs Lễ hội Theo mùa (`SeasonalFestival`)
- **Định nghĩa**: Hai phân hệ sự kiện tách biệt về vòng đời và cơ chế vận hành:
  - **Sự kiện Đặc biệt (`SpecialEvent`)**: Các hoạt động thi đấu, phụ bản hoặc đụng độ theo phiên ngắn hạn (`north_star`, `continuous_40_npc`, `rescue_2k`, `nine_heavens`, `arena`, `guild_siege`, `water_war`, `dragon_boat`), chịu giới hạn watchdog 120 phút và luật riêng của từng đấu trường.
  - **Lễ hội Theo mùa (`SeasonalFestival`)**: Các đợt lễ hội vận hành dài ngày (như *Tết Nguyên Đán*, *Tết Nguyên Tiêu*, *Valentine*, *Cá Tháng Tư*, *Giỗ Tổ Hùng Vương*, *Lễ Phật Đản*, *Tết Đoan Ngọ*, *Lễ Thất Tịch*, *Tết Trung Thu*, *Halloween*, *Lễ Giáng Sinh*), do Admin bật/tắt thủ công độc lập từng lễ hội qua Bảng điều khiển Lễ hội, điều phối cổng/NPC/kịch bản `eve.emg`, vật phẩm rơi dã ngoại, pháo hoa và bảng lore.
- **Ràng buộc (Invariants)**:
  - **Không áp dụng watchdog 120 phút cho Lễ hội**: Trạng thái bật/tắt và bộ đếm đợt lễ hội (`season_seq`) của `SeasonalFestival` được lưu bền vững qua khởi động lại server; không dùng chung `EventPhase` hay `EventLifecycleManager` của đấu trường.
  - **Độc lập với Đua Thuyền Rồng (`dragon_boat` / `OP 0x46`)**: Dù Đua Thuyền Rồng có nội dung văn bản gắn với Tết Đoan Ngọ trên client, `dragon_boat` vận hành độc lập thuộc nhóm `SpecialEvent`; bật/tắt lễ hội `doan_ngo` không khóa hay ép trạng thái `dragon_boat`.
  - **Gating theo ranh giới bản đồ dùng chung**: Các bản đồ phụ bản lễ hội thuần (`54811..54831`, `12941..12943`, `57535..57536`) bị khóa toàn bộ và di tản người chơi về điểm lưu (`savemap`) khi lễ hội tắt; ngược lại, các bản đồ dùng chung với tuyến chính (`12000`, `12001`, `12005`, `12421`) chỉ gate chi tiết theo từng cặp `(scene_id, door_id / place_id / quest_id)` để không chặn người chơi thường.
- **Tránh dùng các từ mơ hồ**: *Special Event* cho lễ hội mùa; gộp chung *seasonal_festival* vào bảng phụ bản/đấu trường.

### Bảng Lore Lễ hội (`Festival.Dat` & `S→C 0x24 0x19`)
- **Định nghĩa**: Giao diện hiển thị bài viết văn hóa/lịch sử của lễ hội (`TMR_FestivalInfoForm`) được client mở ra khi nhận gói `S→C 0x24 0x19 [key: u32 LE]`, đối chiếu bằng phép so sánh bằng trực tiếp `(double)key == record.double_at_0x00` trên file cục bộ `Data/Festival.Dat` (mã hóa XOR luân phiên hai khóa `keyA`/`keyB`, tối đa `0x16E = 366` bản ghi).
- **Ràng buộc (Invariants)**:
  - **Khóa tra cứu là số nguyên ép sang `double`, không phải `TDateTime`**: Client đọc `4` byte `u32 LE` và ép trực tiếp thành số thực IEEE-754 `(double)u32` (không gọi `EncodeDate`); giá trị `19000110` (`0x0121EAC6`) không khớp với `11.0` trong `Data/Festival.Dat` gốc.
  - **An toàn với `Festival.Dat` gốc**: Khi toàn bộ `7` bản ghi trong `Festival.Dat` mang cùng khóa `11.0`, chỉ bản ghi `1` (*Tết Nguyên Đán*) là hợp lệ để mở bằng `u32 = 11`; cấm gửi `24 19` với `u32 = 11` cho các lễ hội `2..7`.
  - **Tối đa 1 hộp thoại mỗi lần đăng nhập**: Client chỉ có một instance `TMR_FestivalInfoForm` duy nhất; mỗi lần đăng nhập chỉ đẩy tối đa `1` khung `24 19` chưa nhận trong đợt (`season-<seq>`).
- **Tránh dùng các từ mơ hồ**: *TDateTime 19000110*, gọi nhầm `OP 0x17` là kênh mở `Festival.Dat`, hoặc gọi bản ghi `rec04` (*Ngày Cá Tháng Tư*) là *Halloween*.

### Thông báo & Hiệu ứng Mùa (`S→C 0x17 0x39` & `0x17 0x46`)
- **Định nghĩa**: Hai kênh phản hồi thời gian thực phía `OP 0x17` do server đẩy xuống:
  - **`S→C 0x17 0x39 <sub2> [payload...]`**: Kênh phát `66` nhánh thông báo (`sub2` thuộc `0x01..0x0D` và `0x15..0x4A`, khuyết `0x0E..0x14`) ra khung chat hệ thống, bảng nổi hoặc thoại NPC, kèm `3` nhánh có hiệu ứng phụ (`0x0A`, `0x1F`, `0x34`).
  - **`S→C 0x17 0x46 <sub3> [x: u16 LE][y: u16 LE][count: u8]`**: Kênh tạo hiệu ứng hạt pháo hoa `TFireLight` trên bản đồ.
- **Ràng buộc (Invariants)**:
  - Không giả định `0x17 0x39` nhận duy nhất `1` byte `1..=74`: nhiều nhánh (như `0x2B`, `0x33`, `0x34`, `0x35`, `0x39`, `0x43`, `0x44`, `0x46`) bắt buộc có `payload[2]` (`sub3`) và các nhánh phi-lễ-hội (`0x07`, `0x2D..0x32`, `0x36..0x38`, `0x40`, `0x41`) không thuộc phân hệ lễ hội.
- **Tránh dùng các từ mơ hồ**: *74 mã thông báo lễ hội* (đánh đồng toàn bộ `0x17 0x39` là lễ hội), *kênh thời tiết tuyết rơi* (hiệu ứng tuyết do `SceneSet.Dat` / `mapId` / `QuestStep` quyết định cục bộ, không qua `0x17 0x39`).
