# BÁO CÁO NGHIÊN CỨU CHECKPOINT 1: ĐẶC TẢ WIRE FORMAT OPCODES 0x14 & 0x18
## HỆ THỐNG TƯƠNG TÁC NPC, SỰ KIỆN EVE VÀ ĐỒNG BỘ TIẾN TRÌNH NHIỆM VỤ

- **Ngày hoàn thành**: 2026-09-21
- **Đối tượng khảo sát**:
  - Mã nguồn decompiled client `aLogin.exe`: `client_pseudo_c/case_018_0078EC3F_FUN_0078ec3f.c` (0x14), `case_021_00790ED5_FUN_00790ed5.c` (0x18), `00729a88_FUN_00729a88.c`, `0072ba54_FUN_0072ba54.c`, `0072bb6c_FUN_0072bb6c.c`, `0074641c_FUN_0074641c.c`.
  - Server mẫu C# Bear: `TS_Server_Bear/TS_Server/Client/QuestStepHelper/PackageDispatchModeResolver.cs`, `ActionHandler.cs`, `TSClient.cs`, `EveData.cs`.
  - Rust loader & struct: `src/data/loaders/eve.rs` (`EveResult`).
- **Trạng thái**: Đã xác minh 100% khớp chéo giữa 3 nguồn (Client Decompile, C# Bear Server, Rust Loader).

---

## 1. Main Opcode `0x14` (`OP_NPC_EVENT`): Tương Tác Thế Giới, NPC & Kịch Bản Eve

Toàn bộ gói tin giao thức TS Online tuân theo framing:
```text
[Header: 2B (0xF4, 0x44)] [Length: 2B LE] [Payload: Length bytes] (mã hóa XOR 0xAD)
```

### 1.1. Chiều Client $\to$ Server ($C \to S$)
Client gửi Opcode 0x14 lên Server qua các hành vi sau:

| SubOp | Tên gọi | Wire Payload | Ý nghĩa / Hành vi |
| :---: | :--- | :--- | :--- |
| `0x01` | `ClickNpc` | `[0x14][0x01][MapObjectID: 2B LE]` | Người chơi click chuột vào NPC trên bản đồ. `MapObjectID` tương ứng `EveNpcPlacement.map_object_id`. |
| `0x04` | `EndTalk` / `StopTalk` | `[0x14][0x04]` | Người chơi đóng bảng thoại, hủy phiên thoại hoặc rời đi. |
| `0x06` | `TalkContinue` | `[0x14][0x06]` | Người chơi bấm nút "Tiếp tục" (Next step) hoặc phím Enter để sang bước thoại tiếp theo. |
| `0x08` | `ClickGate` / `ClickDoor` | `[0x14][0x08][DoorID: 2B LE]` | Người chơi click vào cửa/cổng dịch chuyển map. |
| `0x09` | `SelectMenu` | `[0x14][0x09][ChoiceIndex: 1B]` | Người chơi chọn câu trả lời / lựa chọn phân nhánh trong menu thoại (1-based index). |

---

### 1.2. Chiều Server $\to$ Client ($S \to C$)

#### A. Gói Kịch Bản Thoại / Thực Thi Hiệu Ứng (`SubOp 0x01`..`0x06`)
- **Wire Payload** (17 bytes):
  ```text
  [0x14] [SubOp: 0x01] [0x00] [14 bytes EveResult Binary]
  ```
  - `SubOp`: thường dùng `0x01` (các SubOp 2..6 dùng cùng cấu trúc, xử lý tại `case 1..6:` trong `case_018_0078EC3F_FUN_0078ec3f.c`).
  - Byte thứ 3: cố định `0x00`.
  - 14 bytes tiếp theo là cấu trúc **`EveResult`** nhị phân 14-byte tương thích Little-Endian:

| Byte Offset | Độ dài | Kiểu dữ liệu | Tên trường (`EveResult`) | Ý nghĩa nghiệp vụ |
| :---: | :---: | :---: | :--- | :--- |
| `0..1` | 2 | `u16 LE` | `result_group_no` | Số hiệu nhóm kết quả (Group No). |
| `2` | 1 | `u8` | `result_no` | Thứ tự kết quả trong nhóm. |
| `3` | 1 | `u8` | `result_type` | Loại kết quả: `0`=Action, `1`=Talk, `2`=Door, `3`=Battle, `5`=Animation, `6`=Surface (Menu/Shop), `9`=NpcAction. |
| `4` | 1 | `u8` | `result_class` | Phân lớp: `1`=Item, `2`=Quest, `3`=NpcTeam, `4`=Skill, `7`=Player, `8`=RewardPet. |
| `5..6` | 2 | `u16 LE` | `parameter` | Tham số đối tượng: Map ObjectID / Npc ID / Warp ID / Quest ID / Item ID. |
| `7` | 1 | `u8` | `parameter_style` | Kiểu tham số / cờ hiển thị phụ. |
| `8..11` | 4 | `i32 LE` | `result_value` | Giá trị định lượng: số lượng item, exp, gold, hoặc timer ms. |
| `12..13` | 2 | `u16 LE` | `result_mean_no` | **Dialog ID / Mean No**: mã chuỗi thoại để client tra cứu nội dung text hiển thị lên hộp thoại. |

#### B. Khóa / Mở Khóa Thao Tác Người Chơi (`SubOp 0x2C`)
- **Wire Payload** (7 bytes):
  ```text
  [0x14] [0x2C] [CharID: 4B LE] [Mode: 1B]
  ```
  - `Mode = 0x01`: **Lock** — khóa di chuyển và thao tác của người chơi khi mở hộp thoại (`FUN_0071e288`).
  - `Mode = 0x02`: **Unlock** — mở khóa thao tác khi kết thúc hội thoại (`FUN_0071f9ec`).
- **Frame hoàn chỉnh**:
  - Lock: `F4 44 07 00 14 2C [CharID: 4B LE] 01`
  - Unlock: `F4 44 07 00 14 2C [CharID: 4B LE] 02`

#### C. Đóng Bảng Thoại / Kết Thúc Hội Thoại (`SubOp 0x08`)
- **Wire Payload** (2 bytes):
  ```text
  [0x14] [0x08]
  ```
- **Frame hoàn chỉnh**: `F4 44 02 00 14 08`
- **Tác động tại client (`case 8:` trong `case_018`)**:
  - Gọi VMT `gvar_007DA5A8 + 0x24` đóng cửa sổ thoại.
  - Reset toàn bộ cờ trạng thái bận: `a076 = 0`, `a097 = 0`, `a096 = 0`, `a094 = 0`, `LocalActor + 0x653 = 0`.

---

## 2. Main Opcode `0x18` (`OP_ITEM_INFO` / `OP_QUEST_SYNC`): Đồng Bộ Nhiệm Vụ & Cờ Trạng Thái

Opcode 0x18 là kênh **1 chiều từ Server về Client ($S \to C$)** để quản lý túi nhiệm vụ, nhật ký quest và các cờ trạng thái. Client không bao giờ chủ động gửi Opcode 0x18 (`case 0x18: break;` trong `0077f414`).

### 2.1. Thêm / Cộng Dồn Vật Phẩm Nhiệm Vụ (`SubOp 0x01`)
- **Wire Payload** (5 bytes):
  ```text
  [0x18] [0x01] [ItemID: 2B LE] [Count: 1B]
  ```
- **Frame hoàn chỉnh**: `F4 44 05 00 18 01 [ItemID: 2B LE] [Count: 1B]`
- **Client core (`FUN_00720ca8`)**: Cộng dồn `Count` vào túi nhiệm vụ của người chơi và hiển thị toast tên vật phẩm.

### 2.2. Trừ / Tiêu Hao Vật Phẩm Nhiệm Vụ (`SubOp 0x02`)
- **Wire Payload** (5 bytes):
  ```text
  [0x18] [0x02] [ItemID: 2B LE] [Count: 1B]
  ```
- **Frame hoàn chỉnh**: `F4 44 05 00 18 02 [ItemID: 2B LE] [Count: 1B]`
- **Client core (`FUN_00720df0`)**: Trừ `Count` vật phẩm; nếu về 0 thì giải phóng slot và hiển thị toast.

### 2.3. Báo Đầy Túi Nhiệm Vụ (`SubOp 0x03`)
- **Wire Payload** (2 bytes):
  ```text
  [0x18] [0x03]
  ```
- **Frame hoàn chỉnh**: `F4 44 02 00 18 03`
- **Client core**: Hiển thị Toast thông báo 2000ms với nội dung: *"Dung lượng nhiệm vụ đã đầy"*.

### 2.4. Xóa Sạch Vật Phẩm Nhiệm Vụ Theo ID (`SubOp 0x04`)
- **Wire Payload** (4 bytes):
  ```text
  [0x18] [0x04] [ItemID: 2B LE]
  ```
- **Frame hoàn chỉnh**: `F4 44 04 00 18 04 [ItemID: 2B LE]`
- **Client core (`FUN_00720f00`)**: Xóa hoàn toàn vật phẩm có mã `ItemID` khỏi túi nhiệm vụ.

### 2.5. Cập Nhật Cờ Không Thể Nhận Lại Nhiệm Vụ Đơn Lẻ (`SubOp 0x05` - `QuestDont`)
- **Wire Payload** (5 bytes):
  ```text
  [0x18] [0x05] [Mark: 2B LE] [Flag: 1B]
  ```
- **Frame hoàn chỉnh**: `F4 44 05 00 18 05 [Mark: 2B LE] [Flag: 1B]`
- **Client core (`FUN_00721088`)**: Đánh dấu bit cờ tại mảng 300 bytes (`LocalActor + 0x8B2..0x9DE`), set dirty flag `LocalActor + 0x9DF = 1`.

### 2.6. Đồng Bộ Mục Nhật Ký Nhiệm Vụ Đang Làm (`SubOp 0x06` - `QuestTask` / `QuestLog`)
- **Cấu trúc mỗi entry** (4 bytes):
  ```text
  [SlotNumber: 1B] [QuestID: 2B LE] [MarkStep: 1B]
  ```
- **Hỗ trợ cả 2 chế độ gửi**:
  1. **Đơn lẻ (1 entry)**: Payload 5 bytes:
     `[0x18][0x06][SlotNumber: 1B][QuestID: 2B LE][MarkStep: 1B]`
     Frame: `F4 44 05 00 18 06 [Slot: 1B] [QuestID: 2B LE] [Mark: 1B]`
  2. **Hàng loạt (Bulk - N entries)**: Payload `1 + 1 + 4 * N` bytes:
     `[0x18][0x06] [Entry 1: 4B] [Entry 2: 4B] ... [Entry N: 4B]`
     Frame: `F4 44 [Len: 2B LE] 18 06 [Entries...]`
- **Client core (`FUN_0072bb6c`)**: Duyệt vòng lặp `Len / 4`, ghi vào `LocalActor + 0x654 + slot * 3 = QuestID` và `+ 0x656 + slot * 3 = MarkStep`.

### 2.7. Đồng Bộ Hàng Loạt Cờ Nhiệm Vụ Không Thể Nhận Lại (`SubOp 0x07` - `QuestDontBulk`)
- **Cấu trúc mỗi entry** (3 bytes):
  ```text
  [Mark: 2B LE] [Flag: 1B]
  ```
- **Wire Payload** (`1 + 1 + 3 * N` bytes):
  `[0x18][0x07] [Mark_1: 2B LE][Flag_1: 1B] ... [Mark_N: 2B LE][Flag_N: 1B]`
- **Client core (`FUN_0072ba54`)**: Đọc từng entry 3B, gán thẳng `LocalActor + 0x8B2 + mark_index = Flag`, set dirty `+0x9DF = 1`.

### 2.8. Đồng Bộ Cờ Trạng Thái Nhân Vật Đặc Biệt (`SubOp 0x08` - `ActorStateFlag`)
- **Wire Payload** (9 bytes):
  ```text
  [0x18] [0x08] [CharID: 4B LE] [Kind: 2B LE] [Flag: 1B]
  ```
- **Frame hoàn chỉnh**: `F4 44 09 00 18 08 [CharID: 4B LE] [Kind: 2B LE] [Flag: 1B]`
- **Client core (`FUN_00729a88`)**:
  - `Kind == 1`: Trạng thái "Thần xui" (`LocalActor + 0x448`).
    - `Flag == 1`: Bật trạng thái thần xui.
    - `Flag == 0`: Tắt trạng thái -> Phát âm thanh `"sound\WA0006.wav"` và Toast *"Thần xui đã rời xa bạn!"*.
  - `Kind == 2`: Cờ trạng thái phụ (`LocalActor + 0x455`).

---

## 3. Bản Đồ Quy Đổi & Sẵn Sàng Triển Khai Cho Checkpoint 2..6

| Checkpoint | Nghiệp vụ | Packet Wire liên quan | Codec / Builder tương ứng |
| :---: | :--- | :--- | :--- |
| **CP2** | Click NPC $\to$ Eve Engine | $C \to S$: `14 01 [NpcID: 2B]` | `PacketReader::read_u16_le` |
| **CP3** | Đóng gói Thoại & Lock/Unlock/EndTalk | $S \to C$: `14 01 00 [14B EveResult]`<br>$S \to C$: `14 2C [CharID: 4B] [Mode: 1B]`<br>$S \to C$: `14 08` | `build_talk_step_packet`<br>`build_talk_lock_packet`<br>`build_end_talk_packet` |
| **CP4** | Bấm tiếp tục & chọn menu | $C \to S$: `14 06` (Next step)<br>$C \to S$: `14 09 [Choice: 1B]` | `handle_talk_continue`<br>`handle_talk_select_menu` |
| **CP5** | Đồng bộ Quest & Cờ trạng thái | $S \to C$: `18 01 / 18 02` (Quest Items)<br>$S \to C$: `18 05 / 18 07` (Quest Dont)<br>$S \to C$: `18 06` (Quest Task Log)<br>$S \to C$: `18 08` (Status Flag) | `src/server/handlers/quest_sync.rs` |
| **CP6** | AutoChain & Persistence | Kích hoạt chuỗi sự kiện tự động & lưu DB | `character_quests` SQLite schema |
