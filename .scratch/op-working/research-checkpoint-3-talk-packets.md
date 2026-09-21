# BÁO CÁO NGHIÊN CỨU CHECKPOINT 3: ĐÓNG GÓI PACKET HỘI THOẠI & ĐIỀU KHIỂN THOẠI (OPCODE 0x14)
## WIRE SERIALIZER CHO EVERESULT, KHÓA THAO TÁC 0x14:0x2C VÀ KẾT THÚC THOẠI 0x14:0x08

- **Ngày thực hiện**: 2026-09-21
- **Phạm vi khảo sát**:
  - Mã nguồn Client C decompile: `client_pseudo_c/case_018_0078EC3F_FUN_0078ec3f.c`, `0074641c_FUN_0074641c.c`, `005eb530_FUN_005eb530.c`.
  - Server Bear C#: `PackageDispatchModeResolver.cs`, `PacketSendFinalizer.cs`, `TSClient.cs` (`processStep`, `stopAlltalking`, `ClickkNpc`).
  - Codebase Rust: `src/protocol/codecs/`, `src/protocol/writer.rs`, `src/data/loaders/eve.rs` (`EveResult`).

---

## 1. Quy Trình Chuyển Đổi `EveResult` Thành Wire Frame `0x14 0x01`

### 1.1. Cấu Trúc Khung Tin Nhị Phân (21 bytes tổng cộng, 17 bytes payload)
```text
[Header: 2B (0xF4, 0x44)] [Length: 2B LE (0x11, 0x00 = 17)] [Opcode: 0x14] [SubOp: 0x01] [Pad: 0x00] [14B EveResult Binary]
```

Tuần tự 14 bytes khớp 1:1 giữa struct `EveResult` (Rust) và Client C (`case_018_0078EC3F_FUN_0078ec3f.c`):
1. **Offset 0..1 (u16 LE)**: `result_group_no` $\to$ Ghidra đọc vào `TARGET + 1..2`
2. **Offset 2 (u8)**: `result_no` $\to$ Ghidra đọc vào `TARGET + 3`
3. **Offset 3 (u8)**: `result_type` $\to$ Ghidra đọc vào `TARGET + 4` (`0xa09f` trong `FUN_005eb530.c`: `1`=Talk, `2`=Door, `3`=Battle, `6`=Surface/Menu)
4. **Offset 4 (u8)**: `result_class` $\to$ Ghidra đọc vào `TARGET + 5` (`local_50[5]` trong `FUN_005eb530.c`: `3`=NpcTeam/NPC, `7`=Player)
5. **Offset 5..6 (u16 LE)**: `parameter` $\to$ Ghidra đọc vào `TARGET + 6..7` (`*(ushort *)(local_50 + 6)`: MapObjectID của NPC hoặc 0 của Player)
6. **Offset 7 (u8)**: `parameter_style` $\to$ Ghidra đọc vào `TARGET + 8`
7. **Offset 8..11 (i32 LE)**: `result_value` $\to$ Ghidra đọc vào `TARGET + 9..12` (Item qty, exp, gold)
8. **Offset 12..13 (u16 LE)**: `result_mean_no` $\to$ Ghidra đọc vào `TARGET + 13..14` (Dialog ID tra cứu chuỗi text trong bảng thoại client)

> **Lưu ý về byte đệm `0x00`**: Ở byte thứ 2 của Body (ngay sau SubOp `0x01`), byte `0x00` được client gán vào `TARGET[0]`. Server Bear C# (`PackageDispatchModeResolver.cs:64`) luôn gửi cố định byte `0x00` này (`p2.addByte(0)`).

### 1.2. Hiện Tượng Ghi Đè / Inject Trong Bear C#
- **Đối với kịch bản từ `eve.emg` (Talk, Menu, Battle)**: `packageToSend` được **giữ nguyên 100% binary gốc** từ `eve.emg`, **không hề ghi đè hay thay đổi bất kỳ byte nào**.
  * Khảo sát thực nghiệm Map 10817 Eve 1 (NPC tân thủ):
    - `Res #0`: `type=1, class=3, param=1, mean_no=10364` (NPC 1 nói, `param` đã là `1` - MapObjectID).
    - `Res #1`: `type=1, class=7, param=0, mean_no=10367` (Player nói, `param` đã là `0` - LocalPlayer).
  * $\implies$ Dữ liệu `eve.emg` đã chứa sẵn định danh actor chính xác. Server không được can thiệp sửa đổi `parameter`.
- **Trường hợp duy nhất Bear C# inject `idtalking`**:
  * Chỉ có ở nhánh **fallback dummy talk** (`TSClient.cs:1860-1868`): khi click NPC hoàn toàn không có sự kiện nào trong `eve.emg`, server tạo mảng dummy 14 bytes `{ 0, 0, 1, 1, 3, (byte)idNpcTalking, 0, 0, 0, 0, 0, 0, dialog_low, dialog_high }` để hiển thị bong bóng thoại `"..."` (Dialog ID 45703).

---

## 2. Phân Tích Gói Khóa / Mở Khóa Thao Tác `0x14 0x2C`

### 2.1. Wire Format
```text
[Header: 2B (0xF4, 0x44)] [Length: 2B LE (0x07, 0x00)] [Opcode: 0x14] [SubOp: 0x2C] [CharID: 4B LE] [Mode: 1B]
```
- Độ dài Payload: 7 bytes. Tổng frame: 11 bytes.
- **Mode = `0x01` (Lock)**: Khóa actor (gọi `FUN_0071e288`). Gửi khi bắt đầu click NPC thoại hoặc click cửa chuyển map.
- **Mode = `0x02` (Unlock)**: Mở khóa actor (gọi `FUN_0071f9ec`). Gửi khi kết thúc thoại (`end_talk`, `stopAlltalking`, đóng bảng thoại).

### 2.2. Hành Vi Party / Team (`replyToTeam` trong Bear)
- Khi người chơi là Đội trưởng (`getChar().isTeamLeader()`):
  * Server Bear gửi gói `14 2C [LeaderCharID] [Mode]` broadcast tới toàn bộ thành viên trong đội (`replyToTeam`).
  * Phía client thành viên (`0074641c_FUN_0074641c.c`): Khi nhận gói tin mang `CharID` của Đội trưởng, client thành viên tra cứu actor đội trưởng trên map và gọi `FUN_0071e288` khóa actor đội trưởng lại. Nhờ đó, cả đội ngũ đứng yên đồng bộ, không bị desync di chuyển khi đội trưởng đang bận nói chuyện với NPC.
- Với người chơi Solo: Gửi trực tiếp về client người chơi.

---

## 3. Phân Tích Gói Kết Thúc Thoại `0x14 0x08` (EndTalk)

### 3.1. Wire Format
- Cố định 6 bytes: `F4 44 02 00 14 08` (Chuỗi hex: `"F44402001408"`).

### 3.2. Client Reset Những Cờ Gì? (`case 8:` trong `case_018_0078EC3F_FUN_0078ec3f.c`)
Khi client nhận `14 08`:
1. Gọi `(**(code **)(**(int **)gvar_007DA5A8 + 0x24))()`: **Đóng ngay lập tức form cửa sổ hội thoại (Dialog UI)**.
2. Gán `gvar_007DA5A0 + 0xa095 = 0`: Reset cờ chờ phản hồi thoại.
3. Gán `gvar_007DA5A0 + 0xa076 = 0`: Reset chỉ số bước thoại.
4. Gán `gvar_007DA5A0 + 0xa097 = 0` và `+ 0xa096 = 0`: Reset cờ busy thoại.
5. Gán `gvar_007DA5A0 + 0xa094 = 0`: Reset cờ thực thi script.
6. Gán `LocalActor + 0x653 = 0`: Đánh dấu nhân vật chính thoát khỏi trạng thái kịch bản. Client mở lại tương tác chuột với bản đồ.

---

## 4. Thiết Kế Kiến Trúc & API Rust

### 4.1. Module Codec Mới: `src/protocol/codecs/npc_talk.rs`
```rust
use crate::data::loaders::EveResult;

pub const OP_NPC_EVENT: u8 = 0x14;
pub const SUB_TALK_STEP: u8 = 0x01;
pub const SUB_END_TALK: u8 = 0x08;
pub const SUB_LOCK_ACTOR: u8 = 0x2C;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TalkLockMode {
    Lock = 0x01,
    Unlock = 0x02,
}

/// Đóng gói payload 17 bytes: [0x14][0x01][0x00][14 bytes EveResult]
pub fn build_talk_step_payload(result: &EveResult) -> [u8; 17] {
    let mut payload = [0u8; 17];
    payload[0] = OP_NPC_EVENT;
    payload[1] = SUB_TALK_STEP;
    payload[2] = 0x00;
    payload[3..5].copy_from_slice(&result.result_group_no.to_le_bytes());
    payload[5] = result.result_no;
    payload[6] = result.result_type;
    payload[7] = result.result_class;
    payload[8..10].copy_from_slice(&result.parameter.to_le_bytes());
    payload[10] = result.parameter_style;
    payload[11..15].copy_from_slice(&result.result_value.to_le_bytes());
    payload[15..17].copy_from_slice(&result.result_mean_no.to_le_bytes());
    payload
}

/// Đóng gói frame hoàn chỉnh F4 44 11 00 [17B payload]
pub fn build_talk_step_frame(result: &EveResult) -> Vec<u8>;
pub fn build_talk_step_hex(result: &EveResult) -> String;

/// Đóng gói frame khóa/mở khóa thao tác F4 44 07 00 14 2C [CharID: 4B LE] [Mode: 1B]
pub fn build_talk_lock_frame(char_id: u32, mode: TalkLockMode) -> Vec<u8>;
pub fn build_talk_lock_hex(char_id: u32, mode: TalkLockMode) -> String;

/// Đóng gói frame kết thúc thoại F4 44 02 00 14 08
pub fn build_end_talk_frame() -> [u8; 6];
pub fn build_end_talk_hex() -> &'static str;
```

### 4.2. Bộ Test Nghiệm Thu (`tests/npc_talk_packet_test.rs`)
1. **`test_build_talk_step_trac_quan_npc1`**: Bước thoại đầu tiên của NPC 1 Trác Quận (`mean_no = 10364`):
   `F44411001401000000010103010000000000007C28`
2. **`test_build_talk_step_player_reply`**: Bước thoại của nhân vật chính (`class = 7, param = 0, mean_no = 10367`):
   `F44411001401000000020107000000000000007F28`
3. **`test_build_talk_lock_unlock`**: Lock/Unlock Mode 1 & 2 với CharID `1001`.
4. **`test_build_end_talk`**: Khớp 100% `"F44402001408"`.
