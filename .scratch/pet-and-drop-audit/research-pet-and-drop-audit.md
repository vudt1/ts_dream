# BÁO CÁO NGHIÊN CỨU & AUDIT CHUYÊN SÂU: HỆ THỐNG SỦNG VẬT & CƠ CHẾ DROP ITEM BATTLE TRONG SERVER RUST VÀ CLIENT GAME TS ONLINE

- **Người thực hiện:** Antigravity Agent & Specialized Audit Subagents
- **Đối tượng rà soát:**
  - Hệ thống Sủng vật (Pet): Giao dịch (P2P Trade), Đổi từ vật phẩm (Item Use / EVE Quest), Bắt trong trận (Battle Capture), Quản lý `character_pets` và trạng thái Xuất chiến (`isactive`).
  - Cơ chế Rơi đồ trong Trận đấu (Battle Drop): Dữ liệu `Data/Npcs.txt` vs `Npc.dat`, Tỷ lệ Drop Rate, PK Battle, Cơ chế Hợp kích (Combo), và Đối chiếu Client Dump/Decompile.
- **Ngày hoàn thành:** 17/11/2026

---

## MỤC LỤC
1. [CÂU HỎI 1: HỆ THỐNG SỦNG VẬT, TIẾP NHẬN & XUẤT CHIẾN (`character_pets`)](#phần-1)
   - 1.1. Kiến trúc lưu trữ SQLite & RAM Session
   - 1.2. Phân tích 3 phương thức tiếp nhận Sủng vật
     - A. Giao dịch người chơi (P2P Trade)
     - B. Đổi từ vật phẩm (Vé đoái hoán tâm #31007 / Use Item / EVE Quest)
     - C. Thu phục từ trận chiến (Battle Capture)
   - 1.3. Đối chiếu Gói tin Client gửi lên: Có dùng chung gói tin không?
   - 1.4. Trạng thái Xuất chiến (`isactive`): Có tự động tham chiến không?
   - 1.5. Đánh giá tính đầy đủ, lỗi logic và thiếu sót (Gaps) của Server Rust
2. [CÂU HỎI 2: TỶ LỆ & CƠ CHẾ DROP ITEM TỪ NPC TRONG BATTLE](#phần-2)
   - 2.1. So sánh `Data/Npcs.txt` và `Npc.dat` về dữ liệu Drop1 -> Drop6
   - 2.2. Cách Server Rust tính toán Tỷ lệ Drop Item
   - 2.3. Quy tắc đối với PK Battle (PvP, Arena, Map-NPC PK)
   - 2.4. Cơ chế Hợp kích (Combo) và Drop Item: Có chia đều hay không?
   - 2.5. Đối chiếu Client TS Online gốc (Giao thức `35 04` vs `17 06`)
   - 2.6. Đánh giá tính đầy đủ, lỗi logic và thiếu sót (Gaps) của Server Rust
3. [TỔNG KẾT & DANH MỤC THIẾU SÓT CẦN KHẮC PHỤC](#phần-3)

---

<a name="phần-1"></a>
# PHẦN 1: HỆ THỐNG SỦNG VẬT, TIẾP NHẬN & XUẤT CHIẾN (`character_pets`)

### 1.1. Kiến trúc lưu trữ SQLite & RAM Session
- **Bảng dữ liệu SQLite:** [migrations/0001_init.sql#L134-L176](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql#L134-L176)
  - Khóa chính: `PRIMARY KEY (playerid, storagetype, slot)`.
  - Phân vùng kho (`storagetype`):
    - `1 = Carried` (Sủng mang theo / Roster 4 ô chiến đấu: slot `1..=4`).
    - `2 = Cart` (Sủng trên Xe ngựa / Rương xe).
    - `3 = Hotel` (Sủng gửi Nhà nghỉ / Chuồng trọ: slot `1..=n`, mapping RAM `5..=8`).
    - `4 = Warehouse` (Sủng gửi Kho quân doanh).
  - Cột trạng thái xuất chiến: `isactive INTEGER DEFAULT 0` (Dòng 171: `1 = Đang xuất chiến / theo sau / tham gia tổ đội; 0 = Nghỉ ngơi`).
- **Model thực thể trong Rust:**
  - DB Entity: [src/db/modern/model.rs#L185-L210](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/model.rs#L185-L210) định nghĩa `PetRecord` có trường `is_active: bool`.
  - Runtime RAM: [src/server/session.rs#L131-L163](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L131-L163) định nghĩa `PetState`.
  > [!IMPORTANT]
  > Trong RAM session, struct `PetState` **không lưu cờ `isactive` cục bộ trên từng con sủng**. Thay vào đó, toàn bộ trạng thái xuất chiến của nhân vật được quản lý tập trung qua trường:
  > `pub active_pet_stt: u8` tại [src/server/session.rs#L370](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L370) (giá trị `0` = không có sủng xuất chiến; `1..=4` = số thứ tự slot của sủng đang xuất chiến).
- **Cơ chế nạp / lưu giữa RAM và DB:**
  - Khi Login: [src/db/modern/sqlite/session.rs#L235-L238](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L235-L238): Nếu `storage == 1 && is_active == 1` thì nạp `active_pet_stt = slot`.
  - Khi Logout / Persist: [src/db/modern/sqlite/session.rs#L374-L378](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L374-L378): Nếu `storage == 1 && pet.stt == session.active_pet_stt` thì lưu `isactive = 1`, ngược lại `0`.
  - Cập nhật nhanh (Fast Update): [src/db/persist.rs#L60-L84](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/persist.rs#L60-L84) gọi `update_player(pool, player_id, "SttPetXuatchien", value)` thực thi 2 câu lệnh SQL nguyên tử:
    ```sql
    UPDATE character_pets SET isactive = 0 WHERE playerid = ?;
    UPDATE character_pets SET isactive = 1 WHERE playerid = ? AND storagetype = 1 AND slot = ?;
    ```

---

### 1.2. Phân tích 3 phương thức tiếp nhận Sủng vật

#### A. Giao dịch người chơi (P2P Trade)
- **Giao thức Client gửi lên:** Đi qua Opcode **`0x19` (`OP_TRADE`)**:
  1. *Yêu cầu giao dịch sủng vật:* Sub-opcode `0x0A` (`TRADE_SUB_PET_REQUEST`), mang `[partner_id: 4B LE]` ([src/server/handlers/trade_storage.rs#L318-L322](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L318-L322)).
  2. *Đưa sủng vào khay giao dịch:* Sub-opcode `0x0B` (`TRADE_SUB_PET_OFFER_CS`), mang `[gold: 4B LE] [stt: 1B]` ([src/server/handlers/trade_storage.rs#L544-L553](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L544-L553)).
  3. *Xác nhận giao dịch:* Sub-opcode `0x0C` (`TRADE_SUB_PET_CONFIRM_CS`), mang `[action: 1B]` (`1` = Đồng ý, `0`/`2` = Hủy) ([src/server/handlers/trade_storage.rs#L567-L570](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L567-L570)).
- **Xử lý phía Server Rust:**
  - Khi Offer: Server gửi thông tin pet sang cho bên kia xem qua frame `19 0C` (`pet_offer_frame`, [trade_storage.rs#L139-L172](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L139-L172)).
  - Khi Khóa & Chốt giao dịch: Hàm `pet_trade_settle(&mut a, &mut b)` ([src/server/trade_system.rs#L100-L144](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs#L100-L144)) kiểm tra:
    - Chống trùng sủng vật: Người nhận chưa có sủng mang cùng ID (`Err("duplicate")`).
    - Chống quá tải: Tổng số sủng vật không vượt quá 8 con (`Err("full")`).
    - Bốc sủng vật từ bên cho: `a.pets.retain(|x| x.stt != a_stt)`.
    - Tìm slot trống ở bên nhận: Ưu tiên `next_active_slot` (`1..=4`), nếu hết thì lấy `next_stable_slot` (`5..=8`), rồi `b.pets.push(p)`.
  - Lưu trữ DB nguyên tử: Gọi `persist_sessions_transaction(..., &[&a, &b], &["pet"])` ([trade_storage.rs#L624-L630](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L624-L630)).
  - Gói tin phản hồi:
    - Bên cho pet: Nhận frame xóa pet `0F 02 <owner_id: 4B LE><stt: 1B>` ([trade_storage.rs#L197-L199](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L197-L199)).
    - Bên nhận pet: Nhận frame dữ liệu pet `0F 07` ([trade_storage.rs#L182-L195](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L182-L195)) và frame hiện diện pet `0F 01` ([trade_storage.rs#L201-L209](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L201-L209)).
    - Cả hai nhận frame giao dịch thành công `F4440300190B04` ([trade_storage.rs#L646](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L646)).
- **Lưu ý đối chiếu Client Delphi gốc:**
  Theo [.scratch/client-pseudo-op-code/economy/03_player_trade.md#L458-L483](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/economy/03_player_trade.md#L458-L483), trong client `aLogin.exe` gốc không có luồng giao dịch sủng vật riêng trên Opcode `0x19`. Cụm `19 0A/0B/0C` cho Pet Trade là quy ước của Bear-dialect emulator mà server Rust đang tuân theo để tương thích.

---

#### B. Đổi từ vật phẩm (Item Use / Vé đoái hoán tâm #31007)
- **Bản chất của Item 31007 (*Vé đoái hoán tâm*):**
  - Trong TS Online, Item `31007` ("Vật để đoái hoán ba đậu yêu") **không phải là vật phẩm double-click để ăn trực tiếp**.
  - Người chơi phải mang vé đến gặp NPC sự kiện tại Scene `12003` (Trác Quận / Dinh Độc) để đối thoại đổi lấy Ba Đậu Yêu.
  - Căn cứ [.scratch/starter-item-and-mineral-battle/spec.md#L132-L133](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/starter-item-and-mineral-battle/spec.md#L132-L133) và [src/server/handlers/character.rs#L270-L274](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs#L270-L274): Server cấp sẵn `1x 31007` vào túi tân thủ, và quy định **không xử lý 31007 trong `use_item`**.
  - Việc đổi sủng từ vé 31007 đi qua kịch bản đối thoại EVE Quest: Client gửi Opcode `0x14 Sub 0x01` (`OP_NPC_EVENT`), server kiểm tra điều kiện nhiệm vụ và gọi hàm cấp pet qua `add_pet_to_quest` ([src/server/handlers/quest.rs#L509-L567](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L509-L567)).
- **Trường hợp Dùng vật phẩm thẻ triệu hồi trực tiếp (Use Item):**
  - Đối với các vật phẩm có cờ `add_pet > 10000` trong `Item.dat`:
    - Client gửi Opcode **`0x17` (`OP_ITEM`)** Sub-opcode **`0x0F`** (15 thập phân = Use Item), payload `[slot: 1B] [count: 1B] [use_type: 1B]` ([src/server/handlers/inventory.rs#L67](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs#L67)).
    - Xử lý tại [src/server/handlers/use_item/mod.rs#L298-L335](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs#L298-L335):
      Kiểm tra chống trùng pet, kiểm tra ô trống `1..=4`, sau đó thêm `PetState` vào `session.pets` và tiêu hao vật phẩm (gửi `17 06` / `17 07`).

---

#### C. Thu phục từ trận chiến (Battle Capture)
- **Giao thức Client gửi lên:**
  - Client gửi Opcode **`0x32`** (`handle_battle_command`, [src/server/handlers/battle.rs#L337-L380](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L337-L380)).
  - Sub-opcode: **`0x01`** (Lệnh hành động).
  - Payload binary (6 byte): `[srcGrp: 1B] [srcSlot: 1B] [dstGrp: 1B] [dstSlot: 1B] [skillId: 2B LE]`.
  - Trong đó `skillId` của hành động Bắt là **`15001`** (`0x3A99`). Server ánh xạ `15001 => BattleAction::Capture` ([battle.rs#L366](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L366)).
- **Xử lý phía Server Rust (Battle Runner):**
  - Hệ thống áp dụng nghiêm ngặt **8 Cổng kiểm soát (Validation Gates)** tại [src/battle/runner.rs#L2360-L2420](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L2360-L2420) (`apply_catch`):
    1. *Gate 1 (Chủ thể):* Chỉ người chơi (`caster.typ == 2`) mới được bắt, sủng vật không được thực hiện.
    2. *Gate 2 (Cấp độ):* `target.lv - attacker.lv <= 5` (quái không được cao hơn người quá 5 cấp).
    3. *Gate 3 (Loại đối tượng):* Không được bắt player, pet của người chơi khác, công thành xa, hoặc mục tiêu đang bị trạng thái đóng băng/hóa đá.
    4. *Gate 4 (Cờ Npc.dat):* `can_capture == true` (`not_pet == 0`), quái khoáng không thể bắt.
    5. *Gate 5 (Chống trùng):* Người chơi chưa có sủng trùng ID mẫu trong 4 ô sủng.
    6. *Gate 6 (Sức chứa):* Hành trang sủng phải còn ít nhất 1 slot trống trong dải `1..=4`.
    7. *Gate 7 (Kỹ năng đặc biệt):* NPC đặc biệt (`kind == 22 / 0x16`) yêu cầu người chơi đã học kỹ năng triệu gọi `14026` hoặc `14034`.
    8. *Gate 8 (Ngưỡng máu):* Máu (HP) hiện tại của mục tiêu phải dưới 50% MaxHP.
  - Công thức xác suất: Banker's Round: `percent = 50 + round((attacker.lv - target.lv) / 2)`.
  - Khi bắt thành công:
    - Xóa NPC khỏi bàn cờ: Server broadcast `F44404000B01<row><col>` (Opcode `0x0B Sub 0x01`, [src/battle/packets.rs#L155-L157](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/packets.rs#L155-L157)).
    - Bắn `Out::Catch { owner, npc_id }` về `BattleService` ([src/battle/service.rs#L244-L256](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L244-L256)).
    - Cấp pet qua `add_caught(&mut s.pets, npc_id, hp_max)` ([src/server/pet_box.rs#L36-L51](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs#L36-L51)).
    - Khi kết thúc trận, `BattleService::persist_sessions_transaction` ghi dữ liệu sủng mới xuống bảng `character_pets` SQLite.

---

### 1.3. Đối chiếu Gói tin Client gửi lên: Có dùng chung gói tin không?

> [!IMPORTANT]
> **KẾT LUẬN: KHÔNG! Client gửi 3 gói tin hoàn toàn khác nhau về Opcode, Sub-opcode và cấu trúc dữ liệu.**

| Tiêu chí | 1. Giao dịch (P2P Trade) | 2. Dùng thẻ đổi sủng (Use Item) | 3. Bắt trong trận (Capture) |
|---|---|---|---|
| **Opcode C→S** | **`0x19`** (`OP_TRADE`) | **`0x17`** (`OP_ITEM`) | **`0x32`** (`OP_BATTLE_COMMAND`) |
| **Sub-opcode C→S** | `0x0B` (Offer) & `0x0C` (Confirm) | `0x0F` (Use Item, byte 15) | `0x01` (Action) |
| **Payload C→S** | `[gold: 4B LE] [stt: 1B]` & `[action: 1B]` | `[slot: 1B] [count: 1B] [use_type: 1B]` | `[srcGrp, srcSlot, dstGrp, dstSlot, skillId=15001]` |
| **Bối cảnh** | Đứng cạnh người chơi ngoài map | Mở túi đồ ngoài map | Trong bàn cờ chiến đấu 20 ô |
| **Phản hồi từ Server** | `0F 07` (thông tin), `0F 01` (hiện diện) | `17 06` / `17 07` (trừ item) | `0B 01` (xóa quái trên bàn cờ) |

---

### 1.4. Trạng thái Xuất chiến (`isactive`): Có tự động tham chiến không?

> [!WARNING]
> **KẾT LUẬN: MẶC ĐỊNH LÀ `isactive = 0` (KHÔNG TỰ ĐỘNG XUẤT CHIẾN) QUA CẢ 3 CON ĐƯỜNG.**
> Sủng vật mới nhận luôn ở trạng thái nghỉ ngơi. Người chơi bắt buộc phải gửi lệnh Xuất chiến riêng.

1. **Bằng chứng mã nguồn:**
   - Khi nhận pet qua Trade ([trade_system.rs:120-135](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs#L120-L135)), qua Use Item ([use_item/mod.rs:316-334](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs#L316-L334)), hay qua Capture ([pet_box.rs:36-51](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs#L36-L51)), code chỉ tìm `stt` trống và `pets.push(...)`. **Hoàn toàn không gán `active_pet_stt`**.
   - Khi lưu xuống SQLite ([sqlite/session.rs#L374](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L374)): Cờ `isactive` được tính bằng `if storage == 1 && pet.stt == session.active_pet_stt { 1 } else { 0 }`. Vì `session.active_pet_stt` không trỏ vào pet mới, giá trị ghi vào DB luôn là `0`.
2. **Gói tin Bật / Tắt xuất chiến riêng:**
   - **Bật xuất chiến (Summon Pet):** Client gửi Opcode **`0x13 Sub 0x01`** (`OP_BATTLE_PET`, [src/server/handlers/pet_actions.rs#L416-L449](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L416-L449)), mang `[pet_id: 2B LE]`. Server cập nhật `active_pet_stt = pet.stt`, phản hồi frame `F44406001301<pet_id: 4B LE>`, và cập nhật SQLite `UPDATE character_pets SET isactive = 1 WHERE slot = ?`.
   - **Tắt xuất chiến / Nghỉ ngơi (Recall Pet):** Client gửi Opcode **`0x13 Sub 0x02`** ([pet_actions.rs#L450-L458](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L450-L458)). Server đặt `active_pet_stt = 0`, phản hồi `F44402001302`, và đặt tất cả `isactive = 0`.
   - *Lưu ý về Opcode `0x0F Sub 0x02`:* Đây là lệnh **Thả / Xóa sủng vật vĩnh viễn** ([pet_actions.rs#L185-208](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/pet_actions.rs#L185-L208)), **không phải** lệnh xuất chiến.

---

### 1.5. Đánh giá tính đầy đủ, lỗi logic và thiếu sót (Gaps) của Server Rust

1. 🔴 **Lỗi Desync trong P2P Trade Pet:**
   - Tại [trade_system.rs:120-127](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs#L120-L127): Khi người bán đem trade con sủng đang xuất chiến (`a.active_pet_stt == a_stt`), server xóa sủng khỏi `a.pets` nhưng **quên reset `a.active_pet_stt = 0`**. Biến `active_pet_stt` trỏ vào vị trí rác, gây lỗi khi vào trận đấu tiếp theo.
2. 🔴 **Thiếu gói tin đồng bộ trong Use Item triệu hồi Pet:**
   - Tại [use_item/mod.rs:315-334](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs#L315-L334): Server thêm pet vào RAM và trừ item, nhưng **không gửi bất kỳ gói tin đồng bộ sủng nào về Client** (như `0F 07` / `0F 01`). Người chơi dùng thẻ pet xong sẽ không thấy pet đâu cho đến khi relogin!
3. 🟡 **Chưa nạp chỉ số gốc từ `Npc.dat` khi Capture Pet:**
   - Tại [pet_box.rs:41-49](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs#L41-L49): Hàm `add_caught` khởi tạo sủng với `level: 1, hp: hp_max`, nhưng các chỉ số INT, ATK, DEF, AGI, Element và 4 Kỹ năng gốc của NPC đang để `Default::default()` (bằng 0), chưa hydrate từ dữ liệu `Npc.dat`.

---

<a name="phần-2"></a>
# PHẦN 2: TỶ LỆ & CƠ CHẾ DROP ITEM TỪ NPC TRONG BATTLE

### 2.1. So sánh `Data/Npcs.txt` và `Npc.dat` về dữ liệu Drop1 -> Drop6
- **Trong `Data/Npcs.txt`:**
  - Tiêu đề dòng 2:
    `//Id \t Name \t Level \t Element \t HpMax \t SpMax \t Hpx \t Spx \t Int \t Atk \t Def \t Agi \t Skill1 \t Skill2 \t Skill3 \t Skill4 \t Drop1 \t Drop2 \t Drop3 \t Drop4 \t Drop5 \t Drop6 \t NotPet \t Reborn \t`
  - Các cột `Drop1` đến `Drop6` nằm tại chỉ số cột **`16..=21`** (0-indexed).
  - Trích dẫn loader: [src/data/loader.rs#L582-L589](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L582-L589).
- **Trong file nhị phân `Data/Npc.dat`:**
  - Quy cách bản ghi: Cố định 92 bytes mỗi NPC ([src/data/loaders/npc.rs#L96](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/npc.rs#L96)).
  - Vị trí byte giải mã của 6 ô Drop:
    - Offset `72..73`: `drop1` = `(u16 ^ 0x5209).wrapping_sub(1)`
    - Offset `74..75`: `drop2` = `(u16 ^ 0x5209).wrapping_sub(1)`
    - Offset `76..77`: `drop3` = `(u16 ^ 0x5209).wrapping_sub(1)`
    - Offset `78..79`: `drop4` = `(u16 ^ 0x5209).wrapping_sub(1)`
    - Offset `80..81`: `drop5` = `(u16 ^ 0x5209).wrapping_sub(1)`
    - Offset `82..83`: `drop6` = `(u16 ^ 0x5209).wrapping_sub(1)`
  - Trích dẫn loader: [src/data/loaders/npc.rs#L194-L200](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/npc.rs#L194-L200).
- **Kết luận đối chiếu:**
  1. File `Npc.dat` **chứa thông tin hoàn toàn tương đương 100%** với các cột `Drop1` đến `Drop6` của file `Data/Npcs.txt`.
  2. Cả `Npcs.txt` và `Npc.dat` **CHỈ LƯU ID VẬT PHẨM (u16 Item ID)**, **hoàn toàn không lưu tỷ lệ rơi (Drop Rate), trọng số (Weight) hay xác suất nào**. Tỷ lệ này nằm trong logic hardcode của Game Server.

---

### 2.2. Cách Server Rust tính toán Tỷ lệ Drop Item
- **Hằng số băng thông xác suất:** [src/battle/runner.rs#L457](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L457):
  ```rust
  const DROP_PERCENTS: [i64; 6] = [25, 23, 20, 4, 3, 1];
  ```
- **Hàm rút thăm ngẫu nhiên:** [src/battle/damage.rs#L170-L182](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs#L170-L182) (`get_random_drop`):
  ```rust
  pub fn get_random_drop(rng: &mut DotNetRandom, npc: &Npc, percents: &[i64; 6]) -> i64 {
      let item: [i64; 6] = npc.item;
      let num = i64::from(rng.next_range(1, 1000));
      let mut lo = 0i64;
      for (i, &band) in percents.iter().enumerate() {
          let hi = lo + band;
          if num > lo && num <= hi {
              return item[i];
          }
          lo = hi;
      }
      0
  }
  ```
- **Bảng phân bổ tỷ lệ xác suất thực tế:**
  Hàm `rng.next_range(1, 1000)` sinh số nguyên ngẫu nhiên trong khoảng $[1..999]$ (999 giá trị):

| Slot Drop | Ngưỡng tích lũy | Độ rộng (Band) | Xác suất thực tế ($\frac{\text{Band}}{999}$) | Xác suất danh nghĩa (/1000) |
|:---:|:---:|:---:|:---:|:---:|
| **Drop 1** | $1 \le num \le 25$ | 25 | **$2.503\%$** | $2.5\%$ |
| **Drop 2** | $26 \le num \le 48$ | 23 | **$2.302\%$** | $2.3\%$ |
| **Drop 3** | $49 \le num \le 68$ | 20 | **$2.002\%$** | $2.0\%$ |
| **Drop 4** | $69 \le num \le 72$ | 4 | **$0.400\%$** | $0.4\%$ |
| **Drop 5** | $73 \le num \le 75$ | 3 | **$0.300\%$** | $0.3\%$ |
| **Drop 6** | $76 \le num \le 76$ | 1 | **$0.100\%$** | $0.1\%$ |
| **Không rơi đồ** | $77 \le num \le 999$ | 923 | **$92.392\%$** | $92.4\%$ |

> [!NOTE]
> - Tổng tỷ lệ rơi ra một món đồ bất kỳ là: $\frac{76}{999} \approx \mathbf{7.608\%}$.
> - Tỷ lệ này là **ngẫu nhiên thuần túy**, hoàn toàn không phụ thuộc vào cấp độ người chơi, cấp độ quái, chỉ số may mắn hay trang bị.

---

### 2.3. Quy tắc đối với PK Battle (PvP, Arena, Map-NPC PK)
- Quản lý qua enum `BattleKind` tại [src/battle/construction.rs#L24-L42](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs#L24-L42):
  `BattleKind::allows_rewards(self) -> bool { matches!(self, Self::Normal) }`
- **Khóa chặn phần thưởng:**
  - Tại `note_npc_hit` ([src/battle/runner.rs#L2994-L2997](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L2994-L2997)): Nếu `!self.kind.allows_rewards()` thì không ghi nhận NPC bị trúng đòn.
  - Tại `process_kills` ([src/battle/runner.rs#L3021-L3026](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L3021-L3026)): Nếu `!self.kind.allows_rewards()` thì xóa sạch danh sách `killed_npcs` và `combo_cells`, thoát ngay lập tức.
  - Tại `finish` ([src/battle/runner.rs#L3209, 3246, 3267](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L3209)): Khóa chặn không cộng EXP người chơi, không cộng EXP pet và không trừ độ bền Ngọc Phúc Thần.
- **Kết luận:** **Trong PK Battle, server Rust ngăn chặn tuyệt đối 100% việc rơi vật phẩm (`Out::Drop`), rơi vàng (`Out::Gold`) và nhận điểm kinh nghiệm**.

---

### 2.4. Cơ chế Hợp kích (Combo) và Drop Item: Cơ chế phân bổ phần thưởng và vai trò của Sủng vật

> [!IMPORTANT]
> **KẾT LUẬN VỀ CƠ CHẾ:**
> 1. **Sủng vật (Pet) không có túi đồ riêng (Homdo):** Trong thiết kế kiến trúc game TS Online, chỉ có nhân vật người chơi mới sở hữu hòm đồ cá nhân (`homdo`), pet chỉ có trang bị mang trên người. Do đó, khi một ô tham gia hợp kích là Sủng vật, việc Server Rust lấy ID chủ nhân của Sủng (`caster.id_char`) làm `owner` để nhận vật phẩm và vàng là **hoàn toàn chính xác theo đúng đặc tả nghiệp vụ**.
> 2. **Về việc chia đều vật phẩm:** Server **KHÔNG chia đều (split)** chia nhỏ 1 vật phẩm cho các thành viên, mà mỗi ô chiến đấu (player hoặc pet) tham gia hợp kích thành công đều tạo ra một lượt nhận vật phẩm/vàng về túi đồ của chủ nhân sở hữu ô đó.

Trích dẫn mã nguồn [src/battle/runner.rs#L3079-L3113](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L3079-L3113):
```rust
for entry in &ts.combo_cells {
    let (pr, pc, plv) = parse_cell_entry(entry);
    // ...
    if item_id > 0 {
        let caster = self.cell(pr, pc).cloned().unwrap_or_default();
        let owner = if caster.id_char != 0 {
            caster.id_char // Sủng vật: chuyển quyền nhận vật phẩm về túi đồ của chủ nhân
        } else {
            caster.id      // Người chơi: nhận trực tiếp vào túi đồ của mình
        };
        if owner > 0 {
            out.push(Out::Drop {
                item_id,
                npc_row,
                npc_col,
                row: pr,
                col: pc,
                owner,
            });
            out.push(Out::Broadcast(packets::drop_item(
                item_id as u16,
                npc_row,
                npc_col,
                pr,
                pc,
            )));
        }
    }
}
```

#### Phân tích chi tiết luồng xử lý phần thưởng Hợp kích
1. **Quy tắc định danh chủ sở hữu (`owner`):**
   - Trong dữ liệu ô đấu `WarInfo`:
     - Nếu ô là **Người chơi**: `id_char == 0` $\rightarrow$ `owner = caster.id` (chính người chơi).
     - Nếu ô là **Sủng vật**: `id_char` lưu Player ID của người chơi sở hữu $\rightarrow$ `owner = caster.id_char`.
   - Vì Sủng vật không có hòm đồ riêng, mọi chiến lợi phẩm (vật phẩm rơi, tiền vàng) do sủng vật đạt được trong trận đấu đều được chuyển giao trực tiếp vào hòm đồ (`homdo`) và túi tiền của chủ nhân. Đây là quy tắc xử lý đúng đắn và chặt chẽ.
2. **Cơ chế phân phối khi Người chơi cùng Sủng vật tham gia Hợp kích:**
   - Khi Người chơi A và Sủng vật của A cùng tham gia chuỗi hợp kích kết liễu quái rớt đồ:
     - Ô của Người chơi A kích hoạt: Server sinh 1 sự kiện `Out::Drop` gửi về tài khoản A và broadcast hoạt ảnh bay đồ về tọa độ của Người chơi A.
     - Ô của Sủng vật của A kích hoạt: Server sinh tiếp 1 sự kiện `Out::Drop` gửi về tài khoản A (`owner = caster.id_char`) và broadcast hoạt ảnh bay đồ về tọa độ của Sủng vật.
   - **Kết quả thực tế:** Mỗi ô tham gia thành công mang lại một phần chiến lợi phẩm độc lập. Người chơi có cả sủng vật cùng tham gia combo sẽ nhận được phần thưởng cho cả 2 ô chiến đấu vào hòm đồ cá nhân. Tương tự, vàng thưởng cũng được cộng dồn theo từng ô tham gia hợp kích.

---

### 2.5. Đối chiếu Client TS Online gốc (Giao thức `35 04` vs `17 06`)
- **Giao thức hiển thị hoạt ảnh rơi đồ (`Opcode 0x35 Sub 0x04`):**
  - Giải mã tại [.scratch/client-pseudo-op-code/battle/02_battle_positions.md#L602-L630](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/battle/02_battle_positions.md#L602-L630):
    Client handler `FUN_0064d128` nhận `35 04 <itemId: 2B LE> <npcRow> <npcCol> + K × [dstRow, dstCol]`.
    Gói tin này **thuần túy để client chạy animation vật phẩm bay từ quái đến các ô tham gia**, hoàn toàn không cộng vật phẩm vào túi đồ.
- **Giao thức nhận đồ thực tế vào túi (`Opcode 0x17 Sub 0x06`):**
  - Giải mã tại [.scratch/client-pseudo-op-code/battle/04_battle_end_rewards.md#L37-L105](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/battle/04_battle_end_rewards.md#L37-L105):
    Client handler `FUN_0076B458` nhận `F4 44 0E 00 17 06 [id: 2B LE] [count: 2B LE] [doben: 1B] [long: 1B] [giatri+100: 1B] [khang: 1B] [texp: 4B LE]`.
    Trong Rust, `BattleService::apply_drop` ([src/battle/service.rs#L217-L242](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L217-L242)) xử lý sự kiện `Out::Drop` bằng cách add item vào `homdo` của `owner` và gửi frame `17 06` này tới client của `owner`.

---

### 2.6. Đánh giá tính đầy đủ, lỗi logic và thiếu sót (Gaps) của Server Rust
1. ✅ **Ánh xạ túi đồ Sủng vật về Chủ nhân:**
   - Việc `runner.rs:3091` ánh xạ `caster.id_char` làm `owner` để nhận drop và vàng là hoàn toàn chính xác theo kiến trúc inventory của TS Online (Sủng vật không có homdo).
2. 🟡 **Chưa hỗ trợ Buff Tỷ lệ Drop khi Hợp kích:**
   - Server Rust hiện tính tỷ lệ rơi hoàn toàn độc lập ở dòng 3041 (`damage::get_random_drop`), không hề tăng tỷ lệ khi `ts.combo_cells.len() > 1`.
3. 🟡 **Chưa kiểm tra Sức chứa Túi đồ khi Drop đồ Battle:**
   - `BattleService::apply_drop` ([service.rs#L217](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L217)) gọi `s.add_homdo_item()` và luôn gửi `17 06` mà không kiểm tra hòm đồ đã đầy 25 ô hay chưa, có nguy cơ gây tràn hòm đồ hoặc desync giữa client và server.

---

<a name="phần-3"></a>
# PHẦN 3: TỔNG KẾT & DANH MỤC THIẾU SÓT CẦN KHẮC PHỤC

| Phân hệ | Tính năng | Hiện trạng Server Rust | Đánh giá / Khuyến nghị khắc phục |
|---|---|---|---|
| **Pet Storage** | Schema `character_pets` & `isactive` | Lưu đúng `storagetype` 1..4, `isactive` lưu đúng theo `active_pet_stt`. | ✅ **Hoàn chỉnh và đạt chuẩn.** |
| **Pet Acquisition** | P2P Trade Pet (0x19) | Hoán đổi thành công, chống trùng, chống đầy hòm. Gửi gói `0F 07`, `0F 01`, `0F 02`. | 🔴 **Bug:** Cần reset `active_pet_stt = 0` nếu người bán trade con pet đang xuất chiến. |
| **Pet Acquisition** | Đổi từ Vé 31007 (EVE Quest) | Kịch bản qua NPC sự kiện Scene 12003 NPC 8. Không xử lý trực tiếp qua `use_item`. | ✅ **Đúng thiết kế TS Online gốc.** |
| **Pet Acquisition** | Dùng Item triệu hồi Pet (0x17 Sub 15) | Thêm pet vào RAM `session.pets` khi `add_pet > 10000`. | 🔴 **Bug:** Thiếu frame gửi gói đồng bộ `0F 07` / `0F 01` về client khiến người chơi không thấy pet cho đến khi relogin. |
| **Pet Acquisition** | Bắt quái Battle (0x32 Sub 1, 15001) | Đã chuẩn hóa 8 Validation Gates, tính xác suất bắt, broadcast xóa quái `0B 01`. | 🟡 **Thiếu sót:** Chưa nạp chỉ số thực (INT, ATK, DEF, AGI, Skills) từ `Npc.dat` vào pet mới bắt (hiện đang = 0). |
| **Pet Summon** | Bật xuất chiến / Tham gia tổ đội | Mặc định `isactive = 0`. Dùng gói riêng `0x13 Sub 0x01` (bật) và `0x13 Sub 0x02` (nghỉ ngơi). | ✅ **Đạt chuẩn wire parity.** Client không gửi cùng gói tin với lúc nhận pet. |
| **Battle Drop** | Dữ liệu `Npcs.txt` vs `Npc.dat` | Đọc 6 cột `Drop1..Drop6` (offset 72..83 trong `Npc.dat`). Cả 2 đều chỉ chứa ID item, không có rate. | ✅ **Hoàn chỉnh và chính xác 100%.** |
| **Battle Drop** | Tỷ lệ Rơi đồ từ NPC | Hardcode 6 băng xác suất `[25, 23, 20, 4, 3, 1]` trên thang 1000 (tổng rate $\approx 7.6\%$). | 🟡 **Cải tiến:** Nên tách cấu hình drop rate theo cấp độ hoặc loại quái thay vì hardcode 7.6%. |
| **Battle Drop** | Khóa thưởng trong PK Battle | Chặn 100% Drop đồ, rơi vàng, nhận EXP người/pet trong `BattleKind::Pk`. | ✅ **Hoàn chỉnh, chặt chẽ và có unit test bảo vệ.** |
| **Battle Drop** | Chia đồ khi Hợp kích (Combo) | Duyệt từng ô combo và push item cho `owner` (`caster.id_char` nếu là Pet). | ✅ **Chính xác thiết kế:** Pet không có túi riêng nên ánh xạ về chủ nhân là đúng. Mỗi ô combo thành công cấp 1 phần thưởng về túi đồ chủ nhân. |
| **Battle Drop** | Hoạt ảnh Drop (`35 04`) & Nhận đồ (`17 06`) | Broadcast `35 04` diễn hoạt bay đồ và gửi `17 06` cộng đồ vào túi người nhận. | ✅ **Khớp hoàn toàn với luồng decompile client.** |
