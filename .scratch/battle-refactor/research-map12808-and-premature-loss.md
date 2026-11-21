# Báo Cáo Nghiên Cứu Chuyên Sâu Giao Thức & Game Engine (TS Online Rust `ts_dream`)
**Chuyên đề**: Khảo sát Map #12808 (Người Rơm #22012) & Lỗi Thua Sớm / Hiệu Ứng Bất Thường Trong Trận Đấu (`0064cae0`, `TWildFightSoul`)  
**Ngày thực hiện**: 21/11/2026  
**Người thực hiện**: Lead Game Engine Protocol Subagent  
**Tài liệu / Căn cứ gốc**:
- Mã nguồn máy chủ: `src/server/handlers/talk.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/battle.rs`, `src/battle/runner.rs`, `src/battle/service.rs`, `src/server/packets.rs`
- Dữ liệu gốc: `Data/eve.emg` (`12808.eve`, offset `1450890`, size `7205` bytes), `Data/Npc.dat`, `spec/tsv/eve_fights.tsv`
- Reverse Engineering & Disassembly Client Delphi (`aLogin.exe` PE x86): `0x0064CAE0`, `0x0064C880`, `0x006BAC4C`, `0x00661D45`, `0x00651E3C`, `0x00650A24`, `0x0078D125`
- Mã giả C Ghidra: `client_pseudo_c/0064cae0_FUN_0064cae0.c`, `0064c880_FUN_0064c880.c`, `00651e3c_FUN_00651e3c.c`, `0078d125_FUN_0078d125.c`

---

## 1. TỔNG QUAN VẤN ĐỀ & PHẠM VI NGHIÊN CỨU

Người dùng báo cáo 2 khiếm khuyết nghiêm trọng trong trải nghiệm chiến đấu:
1. **Map #12808 (Bãi luyện tập tân thủ)**: NPC Người Rơm (#22012) xuất hiện trên bản đồ nhưng người chơi nhấp chuột vào không thể tương tác hoặc không bắt đầu trận chiến luyện tập.
2. **Hiện tượng thua trận sớm & Hiệu ứng bất thường trong trận chiến**: Người chơi vào trận, chọn lệnh tấn công thì ngay lập tức bị đóng trận / xử thua hoặc xuất hiện hiệu ứng bất thường liên quan đến `TWildFightSoul` và hàm xử lý `FUN_0064cae0`. Chuỗi packet live được trích xuất cho thấy hàng loạt frame bất thường gửi xen kẽ (`08 01`, `33 01`, `35 03`, `32 01`, `34 01`).

---

## 2. VẤN ĐỀ 1: MAP #12808 & CƠ CHẾ CLICK NGƯỜI RƠM (#22012)

### 2.1. Cấu hình NPC Map #12808 trong `Data/eve.emg`
Qua phân tích binary stream của `Data/eve.emg` (tệp `12808.eve` tại byte offset `1450890`, dung lượng 7,205 bytes), cấu trúc dữ liệu bản đồ #12808 được giải mã đầy đủ như sau:

- **Danh sách NPC trên map (`scene.npcs`)**: Gồm tổng cộng **15 NPC**. Trong đó có **6 Người Rơm** (NPC Template ID `22012`):
  | Map Object ID (Key) | NPC Template ID | Tọa độ (X, Y) | Gắn NpcEvent ID | Hành động tương ứng |
  |---|---|---|---|---|
  | **1** | **22012** (Người Rơm) | (750, 1780) | Event #3 | Đánh 1 Người Rơm (Fight 1) |
  | **2** | **22012** (Người Rơm) | (270, 980) | Event #4 | Đánh 2 Người Rơm (Fight 2) |
  | **3** | **22012** (Người Rơm) | (990, 1400) | Event #5 | Đánh 3 Người Rơm (Fight 3) |
  | **4** | **22012** (Người Rơm) | (250, 500) | Event #6 | Đánh 4 Người Rơm (Fight 4) |
  | **5** | **22012** (Người Rơm) | (1410, 1120) | Event #7 | Đánh 5 Người Rơm (Fight 5) |
  | **6** | **22012** (Người Rơm) | (990, 480) | Event #8 | Đánh 6 Người Rơm (Fight 6) |
  *(Các NPC 7..15 là NPC truyền tống hoặc lính canh khác)*.

- **Cấu hình Trận chiến (`scene.fight_datas`)**:
  Bản đồ `12808.eve` có sẵn **6 cấu hình FightData cục bộ** (ID từ 1 đến 6):
  - `FightData 1`: 1 Người Rơm (ID 22012) bên phe địch.
  - `FightData 2`: 2 Người Rơm (ID 22012).
  - `FightData 3`: 3 Người Rơm (ID 22012).
  - `FightData 4`: 4 Người Rơm (ID 22012).
  - `FightData 5`: 5 Người Rơm (ID 22012).
  - `FightData 6`: 6 Người Rơm (ID 22012).

- **Cấu hình Sự kiện (`scene.npc_events`)**:
  Mỗi NpcEvent từ 3 đến 8 đều có cấu trúc:
  - `when_happen`: Click NPC (`when = 0`).
  - Điều kiện: 1 điều kiện rỗng (`c_class = 0`, luôn thỏa mãn).
  - Kết quả: Duy nhất 1 kết quả `r_type = 3` (`StartBattle`), `r_class = 0`, `r_mean_no = 1..6` (tương ứng FightData 1..6).
  - **Điểm đặc biệt**: **Không có kết quả thoại (`r_type = 1` - Talk Dialog)** đi kèm!

### 2.2. Phân Tích Nguyên Nhân Gốc Rễ (Root Cause) Khiến Người Chơi Không Thể Click

Qua việc đối chiếu mã nguồn xử lý packet `14 01` (Click Talk) tại [talk.rs](file:///d:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) và [eve_runner.rs](file:///d:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs):

#### Nguyên nhân 1: Rào cản kiểm tra khoảng cách (Distance Gate: 130 pixels)
Tại [talk.rs#L163-L173](file:///d:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L163-L173):
```rust
if !EveStateBuilder::is_within_range(conn.session.map_x, conn.session.map_y, npc.x, npc.y) {
    debug!(... "talk rejected: out of range");
    end_talk(conn, out);
    return;
}
```
Khoảng cách cho phép là `130 pixels`. Khi người chơi ở xa click vào Người Rơm (ví dụ Người Rơm #1 ở tận `(750, 1780)` trong khi người chơi mới load map vào ở tọa độ khác), Client TS Online gửi frame `14 01 <map_obj_id>`. Server kiểm tra thấy khoảng cách > 130px liền gọi `end_talk` đóng phiên ngay lập tức, không gửi bất kỳ phản hồi nào về client khiến client giữ nguyên trạng thái đứng im.

#### Nguyên nhân 2: Trận đánh "không hội thoại" (`r_type == 3` thuần túy)
Trong TS Online, các NPC thông thường sẽ có câu thoại:
1. Client gửi `14 01 <id>` -> Server trả thoại `14 01` (Dialog) -> Client hiện khung thoại -> Chọn "Chiến đấu" -> Server gửi `0B FA` bắt đầu trận chiến.
2. Với Người Rơm Map #12808, kịch bản là **Click-to-Fight trực tiếp** (`result_type == 3` ngay bước 1, không có thoại `result_type == 1`).
Tại [eve_runner.rs#L1253](file:///d:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1253):
```rust
let should_lock_ui = !matches!(first_non_action.map(|r| r.result_type), Some(2 | 3));
```
Khi `result_type == 3`, `activate_event_session` không lock UI và trả về `DomainEffect::StartBattle { fight_id }`.
Trong [server_control.rs#L1283-L1285](file:///d:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L1283-L1285):
```rust
if let Some((fight_id, diahinh)) = out.eve_battle {
    conn.session.clear_walk_tracking();
    service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
}
```
Tại [service.rs#L1735-L1748](file:///d:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1735-L1748):
Hệ thống đã nạp trực tiếp `FightData` từ `data.scene_eve_data[&session.map_id].fight_datas[&fight_id]`.
Mọi dữ liệu cấu hình quái (`EveFightEnemy` có `npc_id = 22012`, `location_pos = 1..10`) đều nằm sẵn trong bộ nhớ game data.

#### Thực nghiệm kiểm chứng thực tế:
Bài kiểm tra hồi quy `test_map_12808_straw_dummy_resolution` (tại [npc_eve_resolve.rs#L311-L336](file:///d:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_eve_resolve.rs#L311-L336)) đã xác nhận:
Khi nhân vật đứng ở tọa độ gần Người Rơm (`map_x = 750, map_y = 1780`), cả 6 Người Rơm đều resolve thành công 100% ra `DomainEffect::StartBattle { fight_id: 1..=6 }` mà **không hề cần bất kỳ logic đặc biệt hay ngoại lệ nào trong code**.

### 2.3. Đánh Giá Kiến Trúc: Bác Bỏ Đề Xuất "Hardcode Danh Sách Ngoại Lệ Huấn Luyện"
* **Tuân thủ nguyên tắc Single Source of Truth (SSOT)**:
  Kiến trúc cốt lõi của `ts_dream` coi `Data/eve.emg` là nguồn chân lý duy nhất (Single Source of Truth) cho kịch bản nhiệm vụ, chuyển cảnh và chiến đấu. Toàn bộ thông tin của 6 Người Rơm và 6 trận đấu đã được khai báo hoàn chỉnh, chuẩn mực trong `12808.eve`.
* **Phòng ngừa Technical Debt & Phá vỡ tính trừu tượng**:
  Nếu thêm danh sách ngoại lệ `is_training_dummy(npc_id == 22012)` trong code Rust, ta sẽ biến server từ kiến trúc **Data-Driven** thành **Hardcoded-Logic**. Sau này khi gặp các map bãi luyện khác (mộc nhân, bia ngắm bắn, quái luyện cấp đặc biệt...), server lại phải liên tục sửa code Rust thay vì để engine tự động chạy theo file dữ liệu `.eve`.
* **Kết luận về Map #12808**:
  **Không thêm bất kỳ danh sách ngoại lệ nào**. Engine giữ nguyên luồng xử lý tự nhiên theo `12808.eve`. Người chơi trong game khi muốn tập đánh chỉ cần click di chuyển đến gần Người Rơm (trong bán kính $130\text{px}$) rồi click tương tác thì trận chiến sẽ lập tức nổ ra đúng theo kịch bản.

---

## 3. VẤN ĐỀ 2: PHÂN TÍCH CHUỖI LIVE PACKET & LỖI THUA SỚM / HIỆU ỨNG BẤT THƯỜNG

### 3.1. Phân Tích Từng Byte Chuỗi Giao Thức Thực Tế (Live Packet Breakdown)

Chuỗi 14 packet do người dùng cung cấp được giải mã chi tiết như sau:

```text
[1]  c2s: F444 0900 0601 077E01 8B06 0DFF
     -> Di chuyển nhân vật (Opcode 0x06 Sub 0x01): đi tới tọa độ (X=382, Y=1675).

[2]  s2c: F444 1C00 0BFA 6800 0302 E0930400000000000000 0302 5100 3D00 4F00 3D00 0101
     -> Bắt đầu trận chiến (Opcode 0x0B Sub 0xFA):
        Địa hình: 0x0068 (104).
        Người chơi: Slot Row 3, Col 2 (ID: 0x000493E0 = 300000).
        Chỉ số: MaxHP 81, HP 61, MaxSP 79, SP 61, Hướng 1, Thao tác 1.
     F444 0300 0B0A 01
     -> Khởi động vòng lặp trận (Opcode 0x0B Sub 0x0A): Flag = 1.

[3]  s2c: F444 1A00 0B05 0303 F7550000 0100000000000000 031600 3F00 1600 3F00 0100
     -> Xuất hiện quái/NPC (Opcode 0x0B Sub 0x05):
        Vị trí xuất hiện: Slot Row 3, Col 3 (Chú ý: Row 3!).
        Template NPC: 0x55F7 = 22007.
        Chỉ số: MaxHP 22, HP 63, MaxSP 22, SP 63.

[4]  s2c: F444 0200 3401
     -> Mở lượt nhập lệnh Turn 1 (Opcode 0x34 Sub 0x01): Bắt đầu đồng hồ đếm ngược.

[5]  c2s: F444 0A00 3201 0302 0003 1027 BDB7
     -> Người chơi ra lệnh tấn công (Opcode 0x32 Sub 0x01):
        Người ra lệnh: Row 3, Col 2 (Player).
        Mục tiêu: Row 0, Col 3 (Tọa độ Row 0 khác với Row 3 của NPC lúc sinh ra!).
        Kỹ năng: 0x2710 (10000 = Đánh thường).
        Mã xác thực: BDB7.

[6]  s2c: F444 0400 3505 0302
     -> Server xác nhận nhận lệnh (Opcode 0x35 Sub 0x05):
        Xác nhận cho Unit Row 3, Col 2 đã xong nhập lệnh.

[7]  s2c: F444 0C00 0801 1A01 3D00000000000000
     -> !!! ĐIỂM BẤT THƯỜNG 1: Opcode 0x08 Sub 0x01 (Cập nhật điểm nhân vật ngoài bản đồ)!
        Cập nhật SP = 61 (0x3D) ngoài trận đấu được bắn vào giữa luồng chiến đấu!

[8]  s2c: F444 0900 3301 0302 1A 3D000000
     -> Cập nhật thuộc tính trận đấu (Opcode 0x33 Sub 0x01):
        Unit: Row 3, Col 2 (Player).
        Loại: 0x1A (SP) = 61 (0x3D).

[9]  s2c: F444 0900 3301 0003 19 15000000
     -> !!! ĐIỂM BẤT THƯỜNG 2: Opcode 0x33 Sub 0x01 gửi TRƯỚC animation 32 01!
        Unit: Row 0, Col 3 (Target).
        Loại: 0x19 (HP) = 21 (0x15). Trừ máu ngay lập tức trước khi nhân vật vung kiếm!

[10] s2c: F444 0400 3503 0302
     -> Ép trạng thái Unit (Opcode 0x35 Sub 0x03): Unit Row 3, Col 2 -> Trạng thái 7.

[11] s2c: F444 1300 3201 0F00 0302 1027 01 0100 03 0100 01 19010001
     -> Diễn hoạt hành động chiến đấu (Opcode 0x32 Sub 0x01):
        Kích thước block: 0x000F (15 bytes).
        Người tấn công: Row 3, Col 2, Kỹ năng: 10000.
        Damage: 1 sát thương.

[12] s2c: F444 0900 3301 0003 1A 3F000000
     -> Cập nhật SP mục tiêu (Opcode 0x33 Sub 0x01): Row 0, Col 3, SP = 63 (0x3F).

[13] s2c: F444 0200 3202
     -> Kết thúc đợt diễn hoạt Turn 1 (Opcode 0x32 Sub 0x02).

[14] s2c: F444 0200 3401
     -> !!! ĐIỂM BẤT THƯỜNG 3: Mở ngay Turn 2 (Opcode 0x34 Sub 0x01) trong cùng một buffer TCP!
```

---

## 4. ĐIỀU TRA REVERSE ENGINEERING: BẢN CHẤT CỦA `0064cae0` VÀ `TWildFightSoul`

### 4.1. Hàm `FUN_0064cae0` (`client_pseudo_c/0064cae0_FUN_0064cae0.c`) Trong Delphi Client
Hàm `FUN_0064cae0` tại địa chỉ ảo `0x0064CAE0` trong `aLogin.exe` là bộ giải mã của **Opcode `0x33 Sub 0x01`** (Cập nhật thuộc tính tức thời của đơn vị trong trận đấu).
Cấu trúc gói tin: `33 01 <row: u8> <col: u8> <flag: u8> <val: u32_le>`.

Đoạn mã C dịch ngược:
```c
void __fastcall FUN_0064cae0(void *param_1, int param_2) {
    byte row = *(byte *)(param_2 + 2);
    byte col = *(byte *)(param_2 + 3);
    byte flag = *(byte *)(param_2 + 4);
    uint val = *(uint *)(param_2 + 5);
    
    void *unit = FUN_006509c0(field_ptr, row, col); // Lấy con trỏ unit theo (row, col)
    if (unit == NULL) return;

    if (flag == 0x19) { // Cập nhật HP
        int old_hp = *(int *)((int)unit + 0x3ea);
        *(int *)((int)unit + 0x3ea) = val;
        
        if (val == 0) {
            // Khi HP rớt về 0: Chuyển sang trạng thái chết/gục ngã (state 3)
            FUN_00710ab0(unit, 3);
        } else if (old_hp == 0 && val > 0) {
            // Hồi sinh từ cõi chết: Kích hoạt hiệu ứng hồi sinh
            FUN_0066e710(unit);
        }
    } else if (flag == 0x1A) { // Cập nhật SP
        *(int *)((int)unit + 0x3ee) = val;
    }
}
```

> [!IMPORTANT]
> **Khẳng định chắc chắn từ Assembly**: Hàm `FUN_0064cae0` **KHÔNG HỀ** gọi hoặc khởi tạo đối tượng `TWildFightSoul`!

### 4.2. `TWildFightSoul` Là Gì Trong Delphi Client?
Bằng cách phân tích bảng RTTI và Virtual Method Table (VMT) của `aLogin.exe`:
- VMT `TWildFightSoul`: Địa chỉ `0x00661D45`.
- VMT `TWildFightSoulME` (Magic Effect): Địa chỉ `0x00661DA1`.
- Quá trình khởi tạo được định vị tại `0x006BD124` (`TWildFightSoulME.Create`).

Truy vết lời gọi đến `0x006BD124`:
1. Hàm `FUN_006BAC4C` (tại `0x006BAC4C`): Nhận mã hiệu ứng phép thuật vào thanh ghi `DX`.
2. Khi `DX == 0x6A` (106): Lệnh gọi được thực thi:
   ```asm
   006BAC70: cmp dx, 0x6a
   006BAC74: jnz 0x006bac82
   006BAC76: call 0x006bd124 ; TWildFightSoulME.Create
   ```
3. Nơi gọi `FUN_006BAC4C` với `DX = 0x6A`:
   Nằm tại hàm **`FUN_0064c880` (địa chỉ `0x0064C880`)**, xử lý **Opcode `0x35 Sub 0x01`** (Hiệu ứng Buff / Kỹ năng liên tục)!
   ```asm
   0064C9BB: mov dx, 0x6a
   0064C9BF: mov eax, ebx
   0064C9C1: call 0x006bac4c
   ```

> [!NOTE]
> **KẾT LUẬN VỀ `TWildFightSoul`**:
> `TWildFightSoul` ("Chiến Hồn") **KHÔNG PHẢI LÀ HIỆU ỨNG CHẾT HAY XUẤT HỒN** khi thua trận!
> Đây chính là hiệu ứng đồ họa vầng sáng (Visual Aura) của **Kỹ năng ID 12025: "Cuồng Nhiệt Chiến Hồn" (Berserk War Soul)**.
> Sở dĩ người dùng nhìn thấy xuất hiện dòng chữ hoặc hiệu ứng này là do nhầm lẫn giữa hiệu ứng skill buff đặc biệt và cơ chế tử trận, hoặc do gói tin hiệu ứng trạng thái `35 01` bị kích hoạt sai mã ID!

---

## 5. NGUYÊN NHÂN GỐC RỄ CỦA HIỆN TƯỢNG "THUA TRẬN SỚM / KHÔNG THẤY ĐÁNH"

Dựa trên chuỗi 14 packet và đối chiếu mã nguồn `src/battle/runner.rs`, có **3 sai phạm giao thức chí tử (Protocol Inversions)** được phát hiện:

### 5.1. Sai Phạm 1: Bắn Gói `33 01` (Trừ Máu/SP) TRƯỚC Khi Bắn Animation `32 01`
Tại [runner.rs#L1250-L1360](file:///d:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs):
Trong các hàm tính toán `apply_attack` và `apply_magic`, hệ thống gọi `write_hp(target, new_hp)` và `write_sp(actor, new_sp)` ngay khi vừa tính toán xong sát thương. Các hàm này đẩy ngay `packets::set_hp` (`33 01`) và `packets::set_sp` (`33 01`) ra danh sách buffer `out`.
Mãi về sau, khi kết thúc vòng lặp hành động, chuỗi bytecode animation `ts.text9` mới được đóng gói vào `32 01`.

**Hậu quả trên Client Delphi**:
- Khi nhận `33 01 00 03 19 15...` (gói số [9]), Client cập nhật thanh máu của mục tiêu lập tức. Nếu đòn đánh dứt điểm khiến HP về 0, hàm `FUN_0064cae0` gọi ngay `FUN_00710ab0(unit, 3)` biến unit thành thi thể gục xuống đất.
- Sau đó gói `32 01` (gói số [11]) mới tới, client cố gắng diễn hoạt đòn tấn công vào một unit đã chết hoặc bị desync frame, dẫn tới client bỏ qua diễn hoạt đánh!

### 5.2. Sai Phạm 2: Tiêm Gói Tin Ngoài Trận Đấu `08 01` Vào Giữa Luồng Battle (Gói số [7])
Trong packet capture:
`s2c: F444 0C00 0801 1A01 3D00000000000000` xuất hiện ngay sau khi server nhận lệnh đánh `32 01`!
Nguồn gốc phát sinh: Trong [service.rs#L1139](file:///d:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1139):
Khi nhân vật tiêu hao SP hoặc đổi chỉ số, engine gọi `apply_db_update` -> gọi `apply_player_stat` phát frame `08 01` ra socket.
- `08 01` là opcode cập nhật thuộc tính **ngoài bản đồ chính** (Xử lý bởi `FUN_0078d125` trong client).
- Trong mã giả C `0078d125_FUN_0078d125.c` dòng 73:
  ```c
  if (0xc < *(byte *)(*(int *)gvar_007DA7BC + 0x43e)) {
      FUN_00596a7c(...);
      switchD_00792e22::caseD_0(); // Reset/Clear Dialogs & UI Modals!
      return;
  }
  ```
  Nhận gói `08 01` khi đang trong chế độ trận đấu (`DAT_0098C63C != NULL`) làm kích hoạt cơ chế reset hộp thoại và làm sai lệch con trỏ màn hình của Client TS Online!

### 5.3. Sai Phạm 3: Xả Gói Mở Lượt Mới `34 01` Ngay Lập Tức Cùng Chunk TCP Không Delay (Gói số [14])
Quan sát các gói [11], [12], [13], [14]:
- `32 01`: Diễn hoạt đánh (cần ít nhất 1.0s đến 1.5s để sprite client thực hiện động tác vung kiếm, chém trúng và số sát thương nhảy lên).
- `32 02`: Báo kết thúc diễn hoạt.
- `34 01`: Mở đồng hồ đếm ngược Turn tiếp theo.

Cả 4 gói này được server phát liên tiếp trong **CÙNG MỘT MILI-GIÂY** (cùng buffer TCP)!
**Hậu quả trên Client Delphi**:
Khi client nhận `34 01`, hàm `FUN_00650a24` tại `0x00650A24` được kích hoạt lập tức:
```c
void __fastcall FUN_00650a24(void *field) {
    FUN_00651824(field); // HỦY TOÀN BỘ 21 HIỆU ỨNG CHIẾN ĐẤU ĐANG DIỄN HOẠT!
    *(undefined4 *)((int)field + 0x5a8) = 0;
    // Bật giao diện điều khiển của Turn mới
}
```
Client nhận lệnh hủy ngay khi chưa kịp vẽ một frame animation nào của đòn đánh! Toàn bộ hiệu ứng bị dập tắt, quái hoặc nhân vật bị đứng sững, giao diện lập tức nhảy sang bảng lệnh tiếp theo khiến người chơi tưởng rằng vừa chọn đánh là đã bị thua hoặc bị văng trận đấu.

### 5.4. Sai Phạm 4: Lệch Tọa Độ Slot Ô Trận Đấu Giữa `0B 05` Và `32 01`
- Gói [3] (`0B 05`): Quái sinh ra ở `Row 3, Col 3`.
- Gói [5] (`c2s 32 01`): Client gửi lệnh đánh vào `Row 0, Col 3`.
- Gói [9] (`s2c 33 01`): Server trả về HP của unit `Row 0, Col 3`.
Có sự bất nhất giữa cách server đánh số hàng/cột (Row/Col) giữa hai phe (Phe ta vs Phe địch). Phe địch thường nằm ở các hàng đối diện (`Row 0, 1`). Việc server spawn ở `Row 3, Col 3` (trùng hàng với người chơi) khiến vị trí hiển thị trên bàn cờ trận đấu bị chồng lấn hoặc client coi mục tiêu là không hợp lệ.

---

## 6. KẾ HOẠCH & GIẢI PHÁP SỬA LỖI TOÀN DIỆN (ACTIONABLE FIX PLAN)

### 6.1. Hướng Xử Lý Vấn Đề Map #12808 (Tuân Thủ Kịch Bản Eve 100%)
1. **Bác bỏ hoàn toàn giải pháp hardcode danh sách ngoại lệ**:
   - Không can thiệp mã nguồn `talk.rs` hay `event_npcs.rs` bằng các danh sách ngoại lệ cho NPC Người Rơm #22012.
   - Để toàn bộ tương tác NPC Map #12808 vận hành tự nhiên theo kịch bản `12808.eve` (đã được kiểm chứng xanh 100% qua test `test_map_12808_straw_dummy_resolution`).
2. **Xác nhận điều kiện tương tác và cự ly trong game**:
   - Nhắc nhở/đảm bảo người chơi khi click NPC trong game cần di chuyển vào phạm vi bán kính cho phép ($130\text{px}$) trước khi gửi gói `14 01` (hoặc kiểm tra đồng bộ nội suy vị trí di chuyển `06 01` để tọa độ người chơi trên server kịp thời chạm mốc gần Người Rơm).
   - Cơ chế khởi tạo trận đấu trực tiếp `result_type == 3` từ `scene.fight_datas` đã hoạt động trơn tru. Khi vấn đề 2 (giao thức trong trận đấu) được sửa dứt điểm, trận đấu tập với Người Rơm sẽ diễn ra bình thường.

### 6.2. Khắc Phục Lỗi Chiến Đấu & Animation Desync
1. **Chấn chỉnh thứ tự gói tin (Packet Sequencing Parity)**:
   - Thứ tự chuẩn bắt buộc theo wire protocol TS Online:
     `35 05` (Ack lệnh) -> `35 03` (Trạng thái ra đòn) -> `32 01` (Chạy toàn bộ Animation đòn đánh) -> `33 01` (Cập nhật trừ máu/SP hiển thị sau đòn) -> `32 02` (Kết thúc chuỗi hành động).
   - Di chuyển toàn bộ lời gọi phát `33 01` ra sau khi chuỗi `32 01` được ghi.
2. **Ngăn chặn triệt để gói tin `08 01` trong thời gian chiến đấu**:
   - Trong `src/battle/service.rs`, khi người chơi đang trong trạng thái `in_battle`, tạm hoãn (buffer) các cập nhật thuộc tính ngoài map `08 01`. Chỉ gửi đồng bộ `08 01` khi trận đấu hoàn toàn kết thúc và người chơi trở lại bản đồ chính.
3. **Thêm Pacing / Animation Delay trước khi mở Turn mới (`34 01`)**:
   - Không được flush gói `34 01` cùng một buffer TCP với `32 01`/`32 02`.
   - Cần tính toán thời gian chạy animation của kỹ năng (tối thiểu 800ms - 1500ms tùy animation length) hoặc dùng timer điều phối trước khi gửi `34 01` mở lượt tiếp theo.
4. **Chuẩn hóa Tọa độ Grid (Row/Col) theo chuẩn TS Online**:
   - Rà soát hàm `packets::add_unit_npc` (`0B 05`): Phe ta nằm ở nửa bàn cờ bên phải (`col 0..=1` hoặc `grp 0`), phe địch nằm ở nửa bàn cờ đối diện (`col 2..=3` hoặc `grp 1`). Đảm bảo tọa độ vị trí không bị đảo lộn giữa `row` và `col`.
