# BÁO CÁO NGHIÊN CỨU CHECKPOINT 4: TIẾP TỤC THOẠI & PHÂN NHÁNH MENU (OPCODE 0x14 SUB 0x06 & SUB 0x09)
## ĐẶC TẢ WIRE FORMAT C->S, MÁY TRẠNG THÁI DUYỆT BƯỚC EVE SCRIPT VÀ CƠ CHẾ PHÂN NHÁNH MENU LỰA CHỌN

- **Ngày thực hiện**: 2026-09-21
- **Phạm vi khảo sát**:
  - Mã nguồn Client C decompile & x86 ASM: `client_pseudo_c/0077f414_FUN_0077f414.asm.txt` (dòng 1289–1472), `case_018_0078EC3F_FUN_0078ec3f.c`, `005eb530_FUN_005eb530.c`.
  - Server Bear C#: `ActionHandler.cs`, `TSClient.cs` (`processStep`, `TalkQuestNpc`, `retrunTalk`), `QuestStepHelper/` (`StepMenuSelectionHandler.cs`, `StepConsumptionHandler.cs`, `PackageDispatchModeResolver.cs`, `PacketSendFinalizer.cs`).
  - Codebase Rust: `src/server/handlers/talk.rs`, `src/server/handlers/npc_event.rs`, `src/eve/auto_chain.rs` (`EventSession`, `EventPhase`, `EveAutoChainEngine`), `src/eve/evaluator.rs` (class 10), `src/eve/state.rs`.

---

## 1. Chi Tiết Gói Tin Client $\to$ Server: Tiếp Tục Thoại (`0x14 Sub 0x06`)

### 1.1. Bóc Tách Wire Format Nhị Phân & Mã Hóa XOR
Từ mã nguồn assembly client `0077f414_FUN_0077f414.asm.txt` (dòng 1411–1436):
- **Wire Payload**: **Đúng 2 bytes** `[0x14, 0x06]`. Không có byte tham số phụ.
- **Khung tin hoàn chỉnh qua TCP**:
  - Header: `0xF4, 0x44` (2 bytes)
  - Length (LE): `0x02, 0x00` (2 bytes)
  - Payload đã XOR `0xAD`: `0x14 ^ 0xAD = 0xB9`, `0x06 ^ 0xAD = 0xAB`
  - Chuỗi bytes socket: `F4 44 02 00 B9 AB` (Frame hex thô: `F4 44 02 00 14 06`).

### 1.2. Các Ngữ Cảnh Client Gửi `0x14 Sub 0x06`
1. **Người chơi bấm nút "Tiếp tục" / phím Enter / Space**: Khi hộp thoại hiển thị icon mũi tên nhấp nháy chờ sang câu thoại tiếp theo.
2. **Sau khi click lựa chọn menu**: Xác nhận chuyển sang bước kịch bản tiếp theo.
3. **Tick pump tự động (Auto-return)**: Tại hàm `FUN_005EE30C`, sau khi server gửi `0x14 Sub 0x07` cấp vé chuyển cảnh `0x44F`.

### 1.3. Quy Trình Xử Lý Phía Server Khi Nhận `0x14 Sub 0x06`
1. Kiểm tra `conn.session.current_event_session`. Nếu `None`, fallback về nhánh legacy hoặc gửi `EndTalk` (`F4 44 02 00 14 08`).
2. Tăng `ev.current_index += 1`.
3. Kiểm tra điều kiện vòng lặp `while ev.current_index < ev.results.len()`:
   - **`result_type == 1` (Talk)**:
     - Gửi packet thoại `build_talk_step_hex(&result)` (`14 01 00 [14B payload]`).
     - Tạm dừng thực thi, đợi client hiển thị và người chơi bấm "Tiếp tục".
   - **`result_type == 0` (Action)**:
     - Thực thi cập nhật dữ liệu: nếu `class == 1` cộng/trừ item; nếu `class == 7` cộng EXP/Gold; nếu `class == 2` cập nhật cờ quest.
     - **Không dừng lại**: tự động tăng `ev.current_index += 1` và xét ngay kết quả tiếp theo (non-blocking).
   - **`result_type == 6` (Surface / Menu)**:
     - Ghi nhận `ev.last_surface_id = result.result_mean_no as i32`.
     - Đặt `ev.phase = EventPhase::AwaitingChoice`.
     - Gửi packet Surface về client và dừng lại đợi người chơi chọn câu trả lời (`0x14 Sub 0x09`).
   - **`result_type == 2` (Door / Warp)**:
     - Thực thi chuyển map, mở khóa actor, gửi `14 08` EndTalk và dọn dẹp session.
   - **`result_type == 3` (Battle)**:
     - Đặt `ev.phase = EventPhase::AwaitingBattle`, kích hoạt trận đấu PvE.
4. Khi đã duyệt hết kết quả (`current_index >= results.len()`):
   - Gửi gói mở khóa actor `14 2C [CharID: 4B LE] 02`.
   - Gửi gói kết thúc thoại `14 08` (`F4 44 02 00 14 08`).
   - Kích hoạt kiểm tra AutoChain qua `auto_chain_after`. Nếu có event tiếp theo khớp điều kiện, tự động khởi tạo và khóa actor bắt đầu phiên thoại mới; nếu không thì gán `current_event_session = None`.

---

## 2. Chi Tiết Gói Tin Client $\to$ Server: Chọn Menu / Câu Trả Lời (`0x14 Sub 0x09`)

### 2.1. Wire Format
Từ mã nguồn assembly client `0077f414_FUN_0077f414.asm.txt` (dòng 1437–1472):
- **Wire Payload**: **Đúng 3 bytes**:
  ```text
  [0x14] [0x09] [ChoiceCode: 1B]
  ```
- **Khung tin hoàn chỉnh qua TCP**:
  - Header: `0xF4, 0x44` (2 bytes)
  - Length (LE): `0x03, 0x00` (2 bytes)
  - Payload XOR `0xAD`: `0x14 ^ 0xAD = 0xB9`, `0x09 ^ 0xAD = 0xA4`, `ChoiceCode ^ 0xAD`
  - Chuỗi bytes ví dụ chọn câu 1: `F4 44 03 00 B9 A4 AC` (Thô: `F4 44 03 00 14 09 01`).

### 2.2. Quy Tắc Định Danh `ChoiceCode` — **CẬP NHẬT 2026-09-22 theo đối chiếu mã nguồn server C# tham chiếu**

> **Lịch sử**: Bản thảo ban đầu (2026-09-21) đề xuất `ChoiceCode` là 1-based index (`01/02/03`). Kết luận này **đã bị bác bỏ** sau khi đối chiếu với server C# tham chiếu và data `eve.emg` thật.

**Kết luận đã chốt**: `ChoiceCode` là **raw byte mã option lấy trực tiếp từ data**, server chỉ đọc và so bằng nhau — **không có bất kỳ quy đổi 1-based nào** ở cả 3 tầng (nhận packet, parse data, match điều kiện).

Bằng chứng từ mã nguồn server C# (3 mắt xích):
1. **Nhận packet** — `PacketHandlers/ActionHandler.cs`, `case 9` (Sub `0x09`):
   ```csharp
   client.selectMenu = data[2];   // đọc nguyên byte ChoiceCode, không normalize
   ```
2. **Parse data** — `DataTools/EveData.cs` (condition type 10): `optionId = read16(array4, num14 + 1)` và `DataTools/QuestLogics/ConditionTypeParserAdapter.cs` (`typeCondition == 10`): `tempStep.optionId = bit_4` — `optionId` lấy **trực tiếp** từ file data.
3. **Match điều kiện** — `Client/QuestStepHelper/StepMenuSelectionHandler.cs`:
   ```csharp
   x.idDialog == client.idDialog && x.optionId == client.selectMenu  // raw equality
   ```
4. **Cộng chứng** — `Client/TSClient.cs`: hardcode `selectMenu == 30` (lựa chọn đầu tiên của menu thử nhiệm) và `selectMenu == 40` (đóng thoại).

**Quy ước thực tế** (khớp 100% data `eve.emg`: toàn bộ 1127 điều kiện `class == 10` dùng mã bắt đầu từ 30 — trùng quy ước legacy H6 `select_menu`):
- Lựa chọn thứ nhất: `30` (`0x1E`)
- Lựa chọn thứ hai: `31` (`0x1F`)
- Các lựa chọn kế tiếp: `32`, `33`, ...
- `40` (`0x28`): đóng menu / kết thúc thoại
- `0x00`: trạng thái mặc định (chưa chọn) — server bỏ qua, không xử lý

**Áp dụng trong Rust** (`src/server/handlers/talk.rs` `handle_talk_select_menu`): lưu nguyên byte `payload[0]` vào `ev.last_choice_code` và `session.select_menu` (không normalize) — tương đương `selectMenu` phía server C#; `ev.last_surface_id` (từ `result_mean_no` của Surface) tương đương `idDialog` phía server C#.

### 2.3. Nguồn Gốc Menu & Phân Nhánh Kịch Bản (`condition_class == 10`)
1. **Nguồn gốc UI Menu**:
   - Khi server gửi `EveResultType::Surface` (`result_type == 6`), trường `result_mean_no` chính là **`Surface ID`**.
   - Client tra cứu `Surface ID` trong Section 5 của `eve.emg` (`EveSurfaceData`), lấy số lượng tùy chọn `option_count` và các câu văn `sentences` để vẽ bảng menu lựa chọn.
2. **Đánh giá điều kiện phân nhánh (`src/eve/evaluator.rs`)**:
   ```rust
   // Dialogue choice: param=surfaceId, pStyle=choiceCode.
   10 => {
       state.last_surface_id == param && state.last_choice_code == i32::from(p_style)
   }
   ```
3. **Quy trình kích hoạt nhánh kịch bản mới khi nhận `Sub 0x09`**:
   - Ghi nhận `ev.last_choice_code = choice as i32;` và `session.select_menu = choice as i32;`.
   - Cập nhật ảnh chụp `PlayerEventState` với `last_surface_id` và `last_choice_code`.
   - Gọi `resolve_npc_event` tra cứu lại danh sách event của NPC hiện tại.
   - Nhánh kịch bản có điều kiện `class == 10` khớp với `(last_surface_id, choice_code)` sẽ trả về kết quả `ChainResolveResult`.
   - Gán `session.current_event_session = Some(branch_ev)` và gửi ngay bước 1 của nhánh kịch bản mới về client.

---

## 3. Bản Thiết Kế API & Tích Hợp Vào Codebase

### 3.1. `src/server/session.rs`
Thêm trường vào `Session`:
```rust
pub current_event_session: Option<crate::eve::auto_chain::EventSession>,
```

### 3.2. `src/server/handlers/talk.rs`
Xây dựng các hàm:
- `execute_event_step(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome)`
- `handle_talk_continue(conn: &mut Conn, payload: &[u8], data: &GameData, pool: Option<&DbPool>, out: &mut HandleOutcome)`
- `handle_talk_select_menu(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome)`

### 3.3. Test Kịch Bản Nghiệm Thu (`tests/npc_talk_multistep_test.rs`)
1. **Multi-step dialog**: Click NPC 1 Trác Quận $\to$ Bước 1 $\to$ Bấm `Sub 6` liên tiếp 6 lần qua các bước 2..7 $\to$ Bấm `Sub 6` lần thứ 7 $\to$ Nhận Unlock Actor và EndTalk.
2. **Action execution**: Click NPC 3 Trác Quận với item `32012` $\to$ Trừ item `32012` và cộng item `26012` $\to$ Đóng thoại an toàn.
3. **Menu branching** (đã hiệu chỉnh theo quy ước `ChoiceCode` §2.2): Gặp Surface $\to$ Chọn mã 30 (`Sub 9 1E`) $\to$ Sang nhánh 1; Chọn mã 31 (`Sub 9 1F`) $\to$ Sang nhánh 2. Test thật hiện dùng map 10851, NPC click id 2 (Surface `mean_no=3`, nhánh `conditionNo=3` / `conditionNo=4`).
