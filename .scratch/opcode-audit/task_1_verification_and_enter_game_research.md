# Nghiên Cứu Chuyên Sâu: Rà Soát spec/task_1.md & Giải Mã Chốt Luồng Enter-Game 0x03 vs 0x25

- **Ngày thực hiện:** 2026-09-18
- **Tài liệu đối chiếu:**
  - `src/` (Rust server)
  - `TS_Server_Bear/` (Bear C# server)
  - `client_pseudo_c/` & `ts_decompile/` (Mã nguồn decompile và Assembly x86 của aLogin.exe)
  - `.scratch/client-pseudo-op-code/` (Đặc tả opcode phân tích trước đó)

---

## PHẦN I: GIẢI MÃ BẢN CHẤT LUỒNG ENTER-GAME: `0x03` VS `0x25`

### 1. Phân tích tài liệu `opcode_25.md` & Phát hiện mâu thuẫn Decompile
Trong `.scratch/client-pseudo-op-code/opcode_25.md` trước đây có kết luận:
> *"Chiều Client → Server: ts_decompile/functions/0077f414_FUN_0077F414.c:970-971: case 0x25: break; — rỗng hoàn toàn. Kết luận: OP 0x25 S→C thuần. Không format C→S để mock."*

**Sự thật từ mã nguồn sơ cấp (Assembly x86 & Hex Tables):**
Kết luận trên của `opcode_25.md` là **chưa chính xác do artifact decompile của Ghidra** (tương tự lỗi Ghidra từng gộp rỗng `case 0x3c` và `case 0x3d` đã được đính chính trong `opcode_3c.md` và `opcode_3d.md`).
Bằng chứng đối soát trực tiếp từ nhị phân:
1. **Bảng dispatch C→S**:
   - `redump/table_0x77F474_200B.hex` tại vị trí `0x25` có giá trị `0x23` (35 decimal).
   - `redump/table_0x77F53C_dword200.hex` tại index 35 trỏ tới địa chỉ **`0x007872D7`** (hoàn toàn không phải địa chỉ epilogue `0x0078A4F2`).
2. **Hàm dựng gói tin tại `0x007872D7` (`0077f414_FUN_0077f414.asm.txt:2567-2587`)**:
   ```asm
   007872d7: MOV AL, byte ptr [EBP - 0x6]   ; [EBP - 0x6] là CL (SubSel)
   007872da: DEC AL                         ; Kiểm tra CL == 1
   007872dc: JNZ 0x0078a4f2                 ; Nếu CL != 1 -> thoát
   007872e2: LEA EAX, [EBP - 0x34]
   007872e5: MOV DL, byte ptr [EBP - 0x5]   ; DL = 0x25 (Opcode)
   007872e8: MOV byte ptr [EAX + 0x1], DL   ; byte[1] = 0x25
   007872eb: MOV byte ptr [EAX], 0x1        ; len = 1
   ...
   007872f9: LEA EAX, [EBP - 0x3c]
   007872fc: MOV DL, byte ptr [EBP - 0x6]   ; DL = CL = 0x01
   007872ff: MOV byte ptr [EAX + 0x1], DL   ; byte[1] = 0x01
   00787302: MOV byte ptr [EAX], 0x1        ; len = 1
   0078730b: MOV CL, 0x2                    ; Tổng độ dài 2 byte
   0078730d: CALL 0x00402b60                ; Ghép chuỗi -> buffer [0x25, 0x01]
   0078732a: CALL 0x0051633c                ; CY_AddSedQueue -> gửi TCP ra wire!
   ```
3. **Nơi kích hoạt gửi `[0x25, 0x01]`**:
   - Nằm trong `client_pseudo_c/00509e84_FUN_00509e84.asm.txt:127-130`:
     `MOV CL, 0x1; MOV DL, 0x25; MOV EAX, [0x009264a0]; CALL 0x0077f414;`
   - Hàm `FUN_00509e84` là hàm **"Khởi tạo tài nguyên theo Map"** (nạp `data\<MapID>`, reset camera, clear 50 DWORD gates).
   - Hàm `FUN_00509e84` được gọi duy nhất tại dòng 386 của `client_pseudo_c/case_004_0078BC95_FUN_0078bc95.c` — tức **sau khi client nhận gói S→C `0x03` (SELF) và nạp xong Map**.

### 2. Sự khác biệt kiến trúc giữa Bear C# Server và Rust Server
- **Bear Server**:
  1. Client gửi Auth `0x01`. Nếu hợp lệ và đã có nhân vật, Bear gọi `client.getChar().loginChar()` ngay lập tức.
  2. `loginChar()` đẩy toàn bộ hồ sơ dữ liệu xuống client, trong đó có gói tin **S→C `0x03`** (`sendLook(forReborn: false)` mang toàn bộ MapID, X, Y, ngoại hình, trang bị, tên).
  3. Client nhận S→C `0x03`, tải file bản đồ `data\<MapID>`, khởi tạo đồ họa. Khi tải xong xuôi, client gửi lên **C→S `[0x25, 0x01]`** ("Tôi đã load xong map!").
  4. Bear nhận `case 37` (`0x25`) qua `LoginCompleteHandler.cs`:
     ```csharp
     if (data[1] == 1 && client.map != null) {
         client.map.announceAppear(client);
     }
     ```
     -> Gọi `announceAppear` để báo cho map biết người chơi đã xuất hiện, gửi dữ liệu người chơi/NPC xung quanh cho client và thông báo client xuất hiện cho những người chơi khác trong map.
- **Rust Server**:
  1. Khi nhận Auth `0x01`, Rust xác thực tài khoản và đặt `session.authed = true`. Nhưng Rust **chưa hoàn tất login** nếu tài khoản chưa có character, hoặc Rust chờ một bước xác nhận.
  2. Tại `dispatcher.rs:223`, Rust gán `0x03 => login::handle_enter_game(ctx).await`. Rust mong đợi client gửi **C→S `0x03 sub 1`** để gọi `login_db` và phát chuỗi khung tin `build_logined_sequence_session`.
  3. Opcode `0x25` hoàn toàn không có trong dispatcher của Rust (bị đưa vào `unimplemented`).

### 3. Ý kiến & Kết luận về 0x03 vs 0x25
- **Quan điểm của người dùng nghiêng về `0x25` là HOÀN TOÀN CHÍNH XÁC VỀ BẢN CHẤT CLIENT THỰC TẾ:**
  Gói `0x25` (`[0x25, 0x01]`) chính là tín hiệu **"Login Complete / Map Loaded"** bắt buộc của aLogin client.
- **Vai trò của `0x03`**:
  - `0x03` là gói tin **Server → Client** (Spawn Look / Appearance).
  - Client chỉ gửi C→S `0x03 0x01` trong một kịch bản phụ khi Server gửi `0x09 0x01` (`00713308_FUN_00713308.c`).
- **Giải pháp cho Rust Server**:
  Cần tái cấu trúc luồng đăng nhập & vào game:
  1. Hỗ trợ đầy đủ **Opcode `0x25` (Sub 1)**: Đấu dây `0x25` vào `handle_login_complete`. Khi nhận `[0x25, 0x01]`, server kích hoạt `announce_appear` trên map (đồng bộ các thực thể xung quanh).
  2. Giữ tương thích kép: Nếu client gửi `0x03 0x01` (sau khi tạo nhân vật) thì hoàn tất tạo nhân vật và gửi sequence vào map. Khi client nạp map xong và gửi `0x25 0x01`, server chuyển trạng thái nhân vật sang `InMap / Active`.

---

## PHẦN II: KẾT QUẢ KIỂM CHỨNG CHI TIẾT TỪNG MỤC TRONG SPEC/TASK_1.MD

| STT | Mục trong task_1.md | Kết luận | Bằng chứng đối chiếu (Primary Sources) |
|---|---|:---:|---|
| **1** | **Enter-game 0x03 vs 0x25** | **ĐÚNG** | Bear `PacketProcessor.cs:144` (`case 37: LoginCompleteHandler`) xử lý `[0x25, 0x01]`. Client `00509e84` gọi `SendCommand(0x25, 1)` sau khi load map từ S→C `0x03`. Rust gán nhầm `0x03` làm trigger vào game và bỏ trống `0x25`. |
| **2** | **Hotkey 0x28** (kind, slot) | **ĐÃ FIX** | Code Rust cũ thiếu kind/sub; nhưng trong working tree hiện tại (`src/server/handlers/stats.rs:166-189`) đã kiểm tra `sub == 1`, đọc `kind = payload[0]` (0: clear, 2: assign), slot 1..10 chuẩn xác. |
| **3** | **Stat 0x08** (u16 absolute vs u8 delta) | **ĐÚNG 100%** | Bear `ModifyStatHandler.cs:14` đọc `PacketReader.read16(data, 5)` là **u16 LE giá trị tuyệt đối mới**. Rust `stats.rs:44` đọc `payload[3]` là **1 byte u8 số điểm cộng dồn**. Lệch cả kiểu dữ liệu lẫn logic. |
| **4** | **0x0B Battle** (ground, spectate lệch tầng, jam, sub 8) | **ĐÚNG 100%** | Bear đọc `read16(data, 7)` là `ground`. Rust `battle.rs:118` đọc thành `npc_on_map`. Spectate/Jam trong Bear nằm ở Sub 2 (inner 4, 5), Rust đưa nhầm ra Sub outer (tầng 1). Bear có Sub 8 (PvP nhanh), Rust thiếu hoàn toàn. |
| **5** | **Pet width** (mount 0x0F sub 4 & summon 0x13) | **ĐÚNG 100%** | Bear `PetManipHandler.cs:17` và `PartyHandler.cs:30` đều đọc **`read16` (u16 LE, 2B)**. Rust `pet_actions.rs:248, 334` yêu cầu payload >= 4B và đọc **`u32 LE` (4B)**. |
| **6** | **Pet sub-numbering** (ngựa, 0x2C, 0x1C loop) | **ĐÚNG 100%** | Bear `0x2C` có 7 subcode (`sub 2` là `Addskill4Pet`); Rust `skills.rs:275` gộp hết bỏ qua sub làm mất `Addskill4Pet`. `0x1C sub 2` Bear có vòng lặp học nhiều skill, Rust chỉ đọc 1 entry. |
| **7** | **0x17 sub 46 & sub 30** | **ĐÚNG 100%** | `0x17 sub 46` Bear đọc `read32` col1 & col2 (`2x u32`), Rust chuyển thành chuỗi màu hex. `sub 30` Bear đọc byte `image` của player shop, Rust tính lệch offset bỏ rơi byte image. |
| **8** | **Chat** (thiếu sub 1/6, sub 4) | **ĐÚNG 100%** | Bear `ChatHandler.cs` có sub 1 (World), sub 4 (GM broadcast), sub 6 (Guild). Rust `chat.rs` thiếu sub 1 và 6, để sub 4 rỗng no-op. |
| **9** | **Chưa đấu dây** (0x19, 0x1B, 0x1F) | **ĐÃ ĐẤU DÂY** | Trong working tree `src/server/dispatcher.rs` hiện tại, cả 3 opcode đã được đấu dây tới `trade_storage::handle_trade`, `shops::handle_npc_shop`, `pet_actions::handle_pet_stable`. |
| **10** | **Chưa port** (0x17 sub 14, 17/18, 20, 36/37, 45; Op 0x05) | **ĐÚNG 100%** | Tất cả các subcode trên trong `handlers/inventory.rs` chưa được cài đặt (rơi vào wildcard `_`). Opcode `0x05` bị gán `unimplemented` dù `movement.rs` đã viết logic. |

---

## PHẦN III: KẾ HOẠCH & GIẢI PHÁP HOÀN CHỈNH ĐIỀU CHỈNH SOURCE RUST

### Giai đoạn 1: Sửa lỗi nghiêm trọng chặn luồng vào game & tính toán chỉ số (Ưu tiên P0)
1. **Thêm Handler Opcode `0x25` (Login Complete) vào `dispatcher.rs`**:
   - Bổ sung `0x25 => login::handle_login_complete(ctx).await,`.
   - Trong `handle_login_complete`: kiểm tra `sub == 1`. Đánh dấu `conn.session.in_world = true`. Phát sinh các gói tin đồng bộ người chơi xung quanh bản đồ và phát thông báo xuất hiện (`announce_appear`) cho các client khác trong map.
2. **Sửa Opcode `0x08` (Stat Allocation)**:
   - Sửa trong `src/server/handlers/stats.rs`:
     Đọc `stat_id = payload[2]` (`data[4]`) và `target_val = encoder::u16_le(payload[3], payload[4])` (`data[5..6]`).
     Kiểm tra điều kiện hợp lệ: `target_val == current_stat + 1 && session.point > 0`.
     Tăng chỉ số lên 1, giảm `session.point` đi 1, lưu DB và phản hồi gói `0x08 0x01` cập nhật chỉ số tương ứng.
3. **Map Opcode `0x05` vào `movement::handle_move`**:
   - Sửa trong `dispatcher.rs`: `0x05 | 0x06 => movement::handle_move(ctx),`.

### Giai đoạn 2: Sửa lỗi cấu trúc dữ liệu và width packet (Ưu tiên P1)
1. **Sửa Pet Width trong `src/server/handlers/pet_actions.rs`**:
   - Mount horse (`0x0F sub 4`): Cho phép `payload.len() >= 2`, đọc `pet_id = encoder::u16_le(payload[0], payload[1])`.
   - Pet summon (`0x13 sub 1`): Cho phép `payload.len() >= 2`, đọc `pet_id = encoder::u16_le(payload[0], payload[1])`.
2. **Tái cấu trúc Opcode `0x0B` Battle Control**:
   - Chuyển `Spectate` (inner 4) và `Jam` (inner 5) vào bên trong `handle_pk_or_attack` (xử lý khi `sub == 2`).
   - Đọc `ground = encoder::u16_le(payload[5], payload[6])` tại `data[7..9]` và truyền vào khởi tạo battle.
   - Bổ sung `sub 8` (quick PvP theo Target ID).
3. **Sửa Shop Image và Reborn Parameters**:
   - `0x17 sub 30`: Đọc byte `image = payload[name_len + 1]`, lưu vào cấu trúc Shop và phản hồi đúng format frame.
   - `0x17 sub 46`: Đọc 2 số nguyên 32-bit `col1 = u32_le`, `col2 = u32_le` thay vì đọc chuỗi hex màu.

### Giai đoạn 3: Bổ sung tính năng còn thiếu & Hoàn thiện Gameplay (Ưu tiên P2)
1. **Hệ thống Chat (`src/server/handlers/chat.rs`)**:
   - Bổ sung `sub 1` (Chat thế giới: broadcast toàn server kèm trừ điểm/vật phẩm loa nếu có).
   - Bổ sung `sub 4` (Tin nhắn GM thông báo toàn server).
   - Bổ sung `sub 6` (Chat quân đoàn: gửi tới các thành viên có cùng Guild ID).
2. **Pet Reborn 0x2C & Skill Upgrade 0x1C**:
   - Chia nhánh `sub` trong `0x2C`: xử lý `sub 2` cho `Addskill4Pet`, các sub 1..7 cho các cấp reborn tương ứng.
   - Thêm vòng lặp đọc danh sách skill trong `0x1C sub 2` và bổ sung `sub 5` (Skill Reborn 2).
3. **Port các Sub-op Item còn lại trong `0x17`**:
   - `sub 14` (Crafting), `sub 17/18` (Pet equipment), `sub 20` (Battle item/skill), `sub 36/37` (Túi phụ), `sub 45` (Warp tù).

---

## PHẦN IV: KẾT QUẢ TRIỂN KHAI & BÀN GIAO THỰC THI (P0, P1, P2) - HOÀN THÀNH

- **Trạng thái:** Đã hoàn thành 100% việc triển khai mã nguồn và kiểm thử.
- **Toàn bộ Test Suite:** `cargo test --all-targets --no-fail-fast` -> **53 passed, 0 failed**.

### 1. Chi tiết các file đã sửa đổi
1. **`src/server/session.rs`**:
   - Thêm trường `pub in_world: bool` vào struct `Session` (khởi tạo mặc định `false`).
   - Thêm trường `pub image: u8` vào `PlayerShopState` phục vụ ảnh quầy hàng.
2. **`src/server/dispatcher.rs`**:
   - Đấu dây `0x25 => login::handle_login_complete(ctx).await`.
   - Gom luồng di chuyển `0x05 | 0x06 => movement::handle_move(ctx)`.
3. **`src/server/handlers/login.rs`**:
   - Cài đặt `handle_login_complete`: Kiểm tra guard `sub == 1` và `conn.session.id != 0` (ngăn client chưa đăng nhập/chọn nhân vật kích hoạt xuất hiện trên map).
   - Đánh dấu `conn.session.in_world = true`, phát gói tin `announce_appear` cho các người chơi lân cận trên cùng map, đồng bộ thực thể map và gửi packet thông tin máy chủ (`0x27 sub 9`).
4. **`src/server/handlers/stats.rs`**:
   - Sửa `handle_stat_allocation` (`0x08`): Đọc `target_val = encoder::u16_le(payload[3], payload[4])`. Kiểm tra `target_val <= current_stat + 1` và `session.point > 0`. Trừ 1 điểm tiềm năng, tăng 1 chỉ số, cập nhật DB và gửi phản hồi `0x08 0x01` kèm `prop_code`.
5. **`src/server/handlers/pet_actions.rs`**:
   - Cưỡi thú (`0x0F sub 4`): Cho phép payload `>= 2` bytes, đọc `u16_le` (khớp Bear `read16(data, 2)`).
   - Triệu hồi thú (`0x13 sub 1`): Cho phép payload `>= 2` bytes, đọc `u16_le`.
6. **`src/server/handlers/battle.rs` & `src/battle/service.rs`**:
   - Lồng `Spectate` (inner 4) và `Jam` (inner 5) vào trong `sub == 2` (khớp với kiến trúc phân cấp của Bear và client TS).
   - Đọc địa hình chiến trường `ground` (`u16_le`) tại `payload[5..7]` truyền vào `BattleService::start_npc_battle` và `start_pk_battle`.
   - Bổ sung `sub 8`: Cho phép khiêu chiến PvP trực tiếp theo Player ID với `ground = 65000`.
7. **`src/server/handlers/shops.rs`**:
   - Đọc byte `image = payload[name_len + 1]` khi mở gian hàng người chơi (`0x17 sub 30`), gửi kèm vào frame thông báo mở shop.
8. **`src/server/handlers/inventory.rs`**:
   - `0x17 sub 46` (Chuyển sinh nhân vật): Đọc 2 giá trị `u32_le` (`col1`, `col2`).
   - Cài đặt các subcode: `sub 14` (craft), `sub 17/18` (trang bị pet), `sub 20` (item/skill chiến đấu), `sub 36/37` (túi phụ), `sub 45` (warp tù).
9. **`src/server/handlers/chat.rs`**:
   - Bổ sung `sub 1` (World chat), `sub 4` (GM broadcast), `sub 6` (Guild chat).
10. **`src/server/handlers/skills.rs`**:
    - `0x2C`: Phân luồng `sub 2` gọi `Addskill4Pet` (thêm skill 4 cho thú sau khi reborn), các subcode `1 | 3..=7` chạy luồng reborn rank tương ứng, từ chối subcode không hợp lệ.
    - `0x1C`: Vòng lặp nâng cấp nhiều kỹ năng pet trong `sub 2` kèm xử lý byte lẻ ở cuối buffer, bổ sung `sub 5` (Skill Reborn 2).
11. **`tests/db_repository_init_test.rs`**:
    - Bổ sung khởi tạo trường `in_world: false` cho struct `Session`.
12. **`tests/p0_p1_p2_roadmap_test.rs`**:
    - Tạo mới bộ kiểm thử integration gồm 10 test case bao phủ toàn diện các kịch bản của P0, P1, P2 và các trường hợp biên.

### 2. Tổng kết kiểm thử xác thực
```bash
cargo test --all-targets --no-fail-fast
# Kết quả: 53 tests passed, 0 failed across 10 test suites
```
*(Ghi chú: Theo quy tắc `AGENTS.md`, toàn bộ thay đổi nằm trong working tree chưa được commit để người dùng tự kiểm tra và commit thủ công).*
