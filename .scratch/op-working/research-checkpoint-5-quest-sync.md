# BÁO CÁO NGHIÊN CỨU CHECKPOINT 5: ĐỒNG BỘ TIẾN TRÌNH NHIỆM VỤ & CỜ TRẠNG THÁI (OPCODE 0x18)
## ĐẶC TẢ WIRE FORMAT S->C, CƠ CHẾ QUẢN LÝ TÚI NHIỆM VỤ, NHẬT KÝ QUEST VÀ CỜ TRẠNG THÁI

- **Ngày thực hiện**: 2026-09-21
- **Phạm vi khảo sát**:
  - Client C decompiled: `client_pseudo_c/case_021_00790ED5_FUN_00790ed5.c`, `0072bb6c_FUN_0072bb6c.c`, `0072ba54_FUN_0072ba54.c`, `00729a88_FUN_00729a88.c`, `00721088_FUN_00721088.c`.
  - Server Bear C#: `TSClient.cs` (`refreshQuestTask`, `refreshDontTask`, `refreshOneQuestTask`, `refreshOneDontTask`), `QuestStepHelper/QuestSaveHandler.cs`.
  - Codebase Rust: `src/server/session.rs`, `src/server/dispatcher.rs`, `src/eve/state.rs`.

---

## 1. Khảo Sát Cấu Trúc Wire Nhị Phân Các Sub-Op Của Opcode 0x18 (S $\to$ C)

Toàn bộ gói tin tuân thủ quy tắc Framing:
```text
[Header: 2B (0xF4, 0x44)] [Length: 2B LE] [Payload: Length bytes] (mã hóa XOR 0xAD)
```

| SubOp | Tên gọi | Wire Payload | Độ dài | Hành vi / Xử lý phía Client |
| :---: | :--- | :--- | :---: | :--- |
| `0x01` | `QuestItemAdd` | `[0x18][0x01][ItemID: 2B LE][Count: 1B]` | 5B | `FUN_00720ca8`: Thêm/cộng dồn `Count` vật phẩm nhiệm vụ vào túi quest, toast tên vật phẩm. |
| `0x02` | `QuestItemRemove` | `[0x18][0x02][ItemID: 2B LE][Count: 1B]` | 5B | `FUN_00720df0`: Trừ/tiêu hao `Count` vật phẩm nhiệm vụ; nếu về 0 thì giải phóng slot. |
| `0x03` | `QuestFull` | `[0x18][0x03]` | 2B | VMT Toast 2000ms: *"Dung lượng nhiệm vụ đã đầy"*. |
| `0x04` | `QuestItemClear` | `[0x18][0x04][ItemID: 2B LE]` | 4B | `FUN_00720f00`: Xóa sạch vật phẩm có ID này khỏi túi quest. |
| `0x05` | `QuestDontSingle` | `[0x18][0x05][Mark: 2B LE][Flag: 1B]` | 5B | `FUN_00721088`: Đánh dấu cờ nhiệm vụ không thể nhận lại tại mảng 300 byte (`LocalActor + 0x8B2..0x9DE`), set dirty `+0x9DF = 1`. |
| `0x06` | `QuestTaskLog` | `[0x18][0x06] + N * [Slot: 1B][QuestID: 2B LE][MarkStep: 1B]` | $2 + 4N$ | `FUN_0072bb6c`: Cập nhật nhật ký nhiệm vụ. **Mỗi entry đúng 4 bytes**. Lưu vào `LocalActor + 0x654 + slot * 3 = QuestID`, `+ 0x656 = MarkStep`. |
| `0x07` | `QuestDontBulk` | `[0x18][0x07] + N * [Mark: 2B LE][Flag: 1B]` | $2 + 3N$ | `FUN_0072ba54`: Cập nhật hàng loạt cờ quest dont vào `LocalActor + 0x8B2 + mark_index = Flag`. |
| `0x08` | `ActorStateFlag` | `[0x18][0x08][CharID: 4B LE][Kind: 2B LE][Flag: 1B]` | 9B | `FUN_00729a88`: Cờ trạng thái đặc biệt. `Kind=1` là cờ "Thần Xui" (`LocalActor + 0x448`). Khi `Flag=0` (hết hạn), client phát âm thanh `sound\WA0006.wav` và toast *"Thần xui đã rời xa bạn!"*. `Kind=2` là cờ phụ (`LocalActor + 0x455`). |

---

## 2. Mô Hình Lưu Trữ Tiến Trình Nhiệm Vụ Trong `Session`

Cần bổ sung các trường sau vào `src/server/session.rs`:
```rust
/// Danh sách nhiệm vụ đang thực hiện: QuestID -> (Slot, MarkStep)
pub quest_tasks: std::collections::HashMap<u16, (u8, u8)>,

/// Tập hợp các mã cờ nhiệm vụ không thể làm lại (Quest Dont / Mark)
pub quest_dont: std::collections::HashSet<u16>,

/// Túi vật phẩm nhiệm vụ (tách biệt với túi đồ thông thường homdo)
pub quest_items: Vec<InventoryItem>,
```

### Luồng đồng bộ hóa dữ liệu:
1. **Khi người chơi vào game (`OP_LOGIN_COMPLETE` / `handle_enter_game`)**:
   - Gửi bulk quest log (`0x18 Sub 0x06`) đồng bộ toàn bộ nhiệm vụ đang làm.
   - Gửi bulk quest dont (`0x18 Sub 0x07`) đồng bộ các cờ nhiệm vụ đã hoàn thành.
   - Gửi danh sách vật phẩm nhiệm vụ trong `quest_items` qua `0x18 Sub 0x01`.
2. **Khi hoàn thành / thay đổi bước nhiệm vụ trong kịch bản Eve**:
   - Gửi cập nhật đơn lẻ `0x18 Sub 0x06` để cập nhật bước `mark_step`.
   - Nếu nhiệm vụ kết thúc và không lặp lại: gửi `0x18 Sub 0x05` đánh dấu cờ `mark`.

---

## 3. Bản Thiết Kế API & Module Rust (`src/server/handlers/quest_sync.rs`)

### 3.1. Các Hàm Builder Đóng Gói
```rust
use crate::protocol::encoder;

pub const OP_QUEST_SYNC: u8 = 0x18;
pub const SUB_ITEM_ADD: u8 = 0x01;
pub const SUB_ITEM_REMOVE: u8 = 0x02;
pub const SUB_QUEST_FULL: u8 = 0x03;
pub const SUB_ITEM_CLEAR: u8 = 0x04;
pub const SUB_DONT_SINGLE: u8 = 0x05;
pub const SUB_TASK_LOG: u8 = 0x06;
pub const SUB_DONT_BULK: u8 = 0x07;
pub const SUB_STATUS_FLAG: u8 = 0x08;

pub fn build_quest_item_add_frame(item_id: u16, count: u8) -> Vec<u8>;
pub fn build_quest_item_add_hex(item_id: u16, count: u8) -> String;

pub fn build_quest_item_remove_frame(item_id: u16, count: u8) -> Vec<u8>;
pub fn build_quest_item_remove_hex(item_id: u16, count: u8) -> String;

pub fn build_quest_full_frame() -> Vec<u8>;
pub fn build_quest_full_hex() -> &'static str;

pub fn build_quest_item_clear_frame(item_id: u16) -> Vec<u8>;
pub fn build_quest_item_clear_hex(item_id: u16) -> String;

pub fn build_quest_dont_single_frame(mark: u16, flag: u8) -> Vec<u8>;
pub fn build_quest_dont_single_hex(mark: u16, flag: u8) -> String;

pub fn build_quest_task_frame(slot: u8, quest_id: u16, mark_step: u8) -> Vec<u8>;
pub fn build_quest_task_hex(slot: u8, quest_id: u16, mark_step: u8) -> String;

pub fn build_quest_task_bulk_frame(entries: &[(u8, u16, u8)]) -> Vec<u8>;
pub fn build_quest_task_bulk_hex(entries: &[(u8, u16, u8)]) -> String;

pub fn build_quest_dont_bulk_frame(entries: &[(u16, u8)]) -> Vec<u8>;
pub fn build_quest_dont_bulk_hex(entries: &[(u16, u8)]) -> String;

pub fn build_quest_status_flag_frame(char_id: u32, kind: u16, flag: u8) -> Vec<u8>;
pub fn build_quest_status_flag_hex(char_id: u32, kind: u16, flag: u8) -> String;
```

---

## 4. Kế Hoạch Kiểm Thử Nghiệm Thu (`tests/quest_sync_18_test.rs`)

1. **`test_build_quest_item_add_remove`**:
   - `build_quest_item_add_hex(10001, 2)` $\to$ `F44405001801112702`
   - `build_quest_item_remove_hex(10001, 1)` $\to$ `F44405001802112701`
2. **`test_build_quest_full_and_clear`**:
   - `build_quest_full_hex()` $\to$ `F44402001803`
   - `build_quest_item_clear_hex(10001)` $\to$ `F444040018041127`
3. **`test_build_quest_dont_single_and_bulk`**:
   - `build_quest_dont_single_hex(101, 1)` $\to$ `F44405001805650001`
   - `build_quest_dont_bulk_hex(&[(101, 1), (102, 1)])` $\to$ `F44408001807650001660001`
4. **`test_build_quest_task_single_and_bulk`**:
   - `build_quest_task_hex(1, 10801, 3)` $\to$ `F4440600180601312A03` — ⚠️ bản nháp §4 ghi `0500` là typo: entry 4 byte sau `[18][06]` ⇒ payload 6 byte ⇒ `0600` (đối chiếu `FUN_0072bb6c` stride 4 + `tests/wire_codec_14_18_test.rs`, sửa 2026-09-22)
   - `build_quest_task_bulk_hex(&[(1, 10801, 3), (2, 10802, 1)])` $\to$ `F4440A00180601312A0302322A01`
5. **`test_build_quest_status_flag`**:
   - `build_quest_status_flag_hex(1001, 1, 0)` $\to$ `F44409001808E9030000010000` (Thần xui rời xa)

---

## 5. CẬP NHẬT SAU IMPLEMENT (2026-09-22)

Bằng chứng lấy thêm từ decompile trực tiếp + đối chiếu server Bear trong lúc code Checkpoint 5:

1. **Túi quest vật phẩm và nhật ký quest dùng CHUNG một mảng row của client.**
   - `FUN_00720ca8` (Sub `01`, add quest item) ghi `*(short*)(LocalActor + 0x654 + slot*3) = itemId` và `*(byte*)(LocalActor + 0x656 + slot*3) += count`.
   - `FUN_0072bb6c` (Sub `06`, quest log) ghi đúng 2 offset đó với `(questId, markStep)`.
   - Cả hai đều nhận `LocalActor = *(int*)gvar_007DA7BC` ⇒ **một pool 200 row dùng chung**, không phải 2 cấu trúc rời.
   - $\implies$ Server phải cấp slot từ **một allocator dùng chung** cho `quest_items` + `quest_tasks`. Server Bear cấp task slot theo `TaskQuest.Count + 1` (`ActivityHandler.cs:517`, `ChatHandler.cs:1617`) **không** nhìn các row item ⇒ có thể ghi đè row client. Implementation Rust tránh được lỗi này (test `test_quest_task_rows_share_pool_with_items`: item chiếm row 1 → task nhận row 2).
2. **Biên cứng phía client (đều `_BoundErr` — crash, không phải ignore):**
   - Slot: `if (200 < slot)` trong `FUN_0072bb6c`, `FUN_00720ca8`, `FUN_00720df0` ⇒ **1..=200**.
   - Quest-dont mark: `mark - 1` rồi bound `299` trong `FUN_0072ba54`/`FUN_00721088` ⇒ **1..=300** (mark `0` underflow).
   - Stack quest item: `iVar9 = 0xff - count` trong `FUN_00720ca8` ⇒ add chỉ thành công khi **toàn bộ** `count` lọt dưới 255 (không partial-merge).
   - Remove: `FUN_00720df0` chỉ trừ khi `count <= owned`, ngược lại trả 0 và **không ghi gì**.
   $\implies$ Mutator server từ chối các trường hợp này **trước** wire (không gửi frame client sẽ bỏ), tránh desync hai túi.
3. **Sub `0x03` là tín hiệu "quest capacity full" duy nhất** của client (case 3 → VMT toast 2000 ms) ⇒ dùng làm phản hồi cho **mọi** lần add bị từ chối (tràn stack / hết row). Lưu ý: **Bear không gửi Sub 1/2/3/7/8** (chỉ thấy `PacketCreator(24, 4|5|6)`), nên các nhánh này suy từ decompile, chưa có capture thật.
4. **Bear `refreshQuestTask` gửi `add16(mark)`** (payload 7 byte/entry) trong khi client chỉ đọc `MarkStep` 1 byte tại vị trí 4 rồi bỏ byte thừa nhờ `Len >> 2`. Codec Rust giữ entry **đúng 4 byte** theo decompile.
5. **Thứ tự login**: Bear `TSCharacter.loginChar()` gọi `loadQuestDB → refreshQuestTask → refreshDontTask` ở **cuối chuỗi login** (sau `sendItems`/`sendGold`/`sendHotkey`). Rust map vào **Step 22** của `build_logined_sequence_session` (task bulk → dont bulk → item adds); trạng thái rỗng ⇒ không frame `0x18` nào ⇒ golden parity giữ nguyên.
6. **Không có sub-opcode "xóa quest log"** (khác Sub `0x04` Clear item) ⇒ `Session.quest_tasks` không có hàm remove trong CP5; việc kết thúc quest thuộc mission store của CP6.
7. **C→S `0x18` không tồn tại** trong corpus client (kênh S→C thuần) ⇒ dispatcher giữ nguyên `OP_ITEM_INFO` ở unimplemented boundary.
