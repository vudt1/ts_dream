# ADR 0006 — Chuẩn hóa Giao thức Chiến đấu & Vòng đời Lượt đấu (Battle Protocol Wire Parity & Turn Lifecycle)

- **Ngày**: 2026-10-24
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn grilling)

---

## 1. Bối cảnh (Context)

1. Quá trình dịch ngược mã máy thân Client TS Online (`aLogin.exe` Borland Delphi Win32 x86, calling convention `__register`) được tổng kết đầy đủ tại [`.scratch/battle-refactor/BATTLE_MODULE_REVERSE_SPEC_CLEAN.md`](../../.scratch/battle-refactor/BATTLE_MODULE_REVERSE_SPEC_CLEAN.md).
2. Khi đối chiếu với source code server Rust `ts_dream`, phát hiện 3 sai lệch kiến trúc nghiêm trọng:
   - **Kênh lệnh chiến đấu C→S bị triệt hạ sai**: Do bộ decompile C của Ghidra bỏ sót thanh ghi `ECX` và ngộ nhận nhánh dead-code tại `0x00788BCA`, Ticket 01 kết luận sai lầm rằng client không bao giờ gửi `0x32`. Server đã xóa `0x32` khỏi `dispatcher.rs` và chuyển logic skill/item sang `0x0B Sub 0x03/0x04`. Tuy nhiên, mã máy thực tế tại [`0063cf58_FUN_0063cf58.asm.txt`](../../client_pseudo_c/0063cf58_FUN_0063cf58.asm.txt) (dòng 382 và 1179) khẳng định client **luôn gửi `0x32 Sub 0x01` (10B)** cho mọi đòn đánh/kỹ năng/phòng thủ/chạy trốn/bắt tướng và **`0x32 Sub 0x02` (10B)** cho vật phẩm chiến đấu.
   - **Đảo lộn thời điểm phát lệnh mở lượt (`34 01` Timing Inversion)**: `manager.rs` chờ lệnh từ người chơi qua `collect_commands` (timeout 20s) **trước khi** gọi `run_turn`. Trong khi đó, gói tin mở lượt `34 01` (lệnh duy nhất giúp client mở menu điều khiển và khởi chạy đồng hồ đếm ngược) lại chỉ được sinh ra trong `turn_phase1` của `run_turn`. Hậu quả là ở Hiệp 1, server chờ 20 giây trong im lặng làm client bị đóng băng giao diện, không mở được menu lệnh.
   - **Thiếu gói tin báo kết thúc hoạt ảnh hiệp đấu (`32 02`)**: Client cần frame `32 02` (`FUN_0064dd74`) để giải phóng hàng đợi diễn hoạt và hoàn tất chuỗi hoạt ảnh trước khi bước vào hiệp kế tiếp.
3. Để đảm bảo giao tiếp giữa Client và Server diễn ra trơn tru nhất, triệt tiêu nguy cơ treo kẹt UI hoặc lệch nhịp trận đấu, các quyết định kiến trúc dưới đây được xác lập và cam kết thực thi.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Phục hồi Kênh Lệnh Chiến Đấu `0x32` Chuẩn Wire Parity
- **Đăng ký lại Opcode `0x32` (OP_BATTLE_COMMAND)** trong `src/server/dispatcher.rs` làm kênh tiếp nhận lệnh thao tác chiến đấu duy nhất từ client.
- **Cấu trúc khung tin nhị phân C→S (10 bytes)**:
  $$\text{[32]} \text{ [sub:1B]} \text{ [srcGrp:1B]} \text{ [srcSlot:1B]} \text{ [dstGrp:1B]} \text{ [dstSlot:1B]} \text{ [skillOrItemId:2B LE]} \text{ [rand1:1B]} \text{ [rand2:1B]}$$
  - `sub = 0x01`: Đòn đánh thường (`10000`), Thi triển Kỹ năng, Phòng thủ (`17001`), Bỏ chạy (`18001`), Bắt võ tướng (`15001`).
  - `sub = 0x02`: Sử dụng vật phẩm chiến đấu mục tiêu trên lưới ô cờ.
- **Loại bỏ hoàn toàn tính tương thích ngược với `0x0B Sub 0x03/0x04`**:
  - Toàn bộ test suite trong `tests/` phải được chuyển đổi sang gửi frame `0x32` chuẩn wire.
  - Phục hồi đúng ngữ nghĩa nguyên bản của `0x0B Sub 0x03` và `0x0B Sub 0x04` là các gói lệnh tương tác bản đồ ngoài trận (8 bytes: `[kind:1B][target:4B LE][param:2B LE]`), không còn gán ghép làm lệnh skill/item nữa.

### 2.2. Tái cấu trúc Vòng Đời Lượt Đấu (Turn Lifecycle) & Handshake Mở Màn
- **Handshake Khởi đầu Trận chiến**:
  - Sau khi phát `0B FA` (nạp scene) và `0B 0A` (trailer thời gian lượt), task trận đấu thiết lập rào chắn (Barrier) chờ gói ack `0B FA` (2 bytes) từ client chủ phòng.
  - Áp dụng cơ chế **Fallback Timeout 1.5s**: Nếu sau 1.5s chưa nhận được ack (do packet loss hoặc client cũ), server tự động giải phóng rào chắn và tiến hành mở hiệp đầu tiên.
- **Trật tự 4 Bước của Mỗi Hiệp Đấu trong `manager.rs`**:
  1. **Bước 1 (Đầu hiệp — Turn Prompt)**: Server phát gói tin `34 01` (`packets::your_turn()`) cho mọi người chơi còn sống phe ta và khởi động bộ đếm lùi thời gian (mặc định 20 giây).
  2. **Bước 2 (Thu thập lệnh — Command Collection)**: Lắng nghe và gom `PlayerInput` gửi lên qua kênh `0x32` từ các đấu sĩ hợp lệ (`owns_commandable_cell`). Khi tất cả đã sẵn sàng hoặc hết 20 giây timeout, tự động gán đòn đánh cơ bản cho đơn vị chưa kịp ra lệnh.
  3. **Bước 3 (Thực thi & Diễn hoạt — Action Execution)**: Gọi `battle.run_turn(...)`, phát tuần tự các gói tin diễn hoạt hoạt ảnh `35 03` (thế võ), `32 01` (chiêu thức), `33 01` (HP/SP), `35 0F` (icon trạng thái) ra `BattleSink`.
  4. **Bước 4 (Kết thúc diễn hoạt & Độ trễ hiệp — Action Finish & Pacing)**:
     - Phát gói tin `32 02` (`packets::turn_action_finish()`).
     - Tạm dừng luồng trận đấu bằng **Dynamic Skill Delay Pacing**:
       $$\text{sleep\_ms} = \text{clamp}\left(\sum \text{delay}_{\text{action}} \times 100\,\text{ms},\, 800\,\text{ms},\, 3500\,\text{ms}\right)$$
     - Sau khi hết thời gian diễn hoạt, quay trở lại Bước 1 của hiệp tiếp theo.

### 2.3. Bổ sung Frame Báo Dứt Diễn Hoạt `32 02` (`turn_action_finish`)
- Xây dựng builder `packets::turn_action_finish() -> String` tạo frame chuẩn wire: `F4 44 02 00 32 02`.
- Gọi hàm này ngay sau khi luồng diễn hoạt đòn đánh của hiệp kết thúc để client gọi `FUN_0064dd74`, giải phóng hàng đợi hoạt ảnh `TFightField`.

### 2.4. Chuẩn hóa 4 Pha Timing của Biến Cố Trận Đấu `0x35`
Tuân thủ nghiêm ngặt 4 pha timing trên lưới cờ $4 \times 5$:
1. **Pha 1 (Đầu hiệp)**: Bắn `35 01` (`purge_status`) và `clear_unit_status` (`35 0F` flag=0) để gỡ bỏ hiệu ứng/icon trạng thái đã hết thời hạn.
2. **Pha 2 (Khởi động đòn đánh)**: Bắn `35 03` (`force_state7`) trước mỗi đòn đánh để xả sạch FX cũ và ép đơn vị vào tư thế rút vũ khí. Nếu có dịch chuyển ô cờ, bắn `35 07`.
3. **Pha 3 (Tác động chiêu thức)**: Sau khi phát `32 01`, nếu kỹ năng gây buff/debuff thành công thì bắn `35 0F` (`battle_unit_status`) để cập nhật 6 khe biểu tượng HUD ('B'..'G') và số hiệp.
4. **Pha 4 (Dọn dẹp)**: Bắn `35 04` nếu có vật phẩm rơi, `35 06` nếu có thoại NPC mở màn/kịch bản, `35 0B` cho bảng tin chiến sự.

---

## 3. Hệ quả & Cam kết Kỹ thuật (Consequences)

- **Tích cực**:
  - Client `aLogin.exe` thật kết nối vào sẽ hoạt động mượt mà, không còn hiện tượng treo đơ menu lệnh ở Hiệp 1, không bị drop lệnh đánh, và xem trọn vẹn hoạt ảnh giữa các hiệp.
  - Wire Parity đạt mức A chuẩn hóa 100% với giao thức gốc Delphi.
- **Nhiệm vụ đi kèm**:
  - Cập nhật toàn bộ các test fixture trong `tests/` đang giả lập `0x0B Sub 3/4` chuyển sang `0x32 Sub 01/02`.
  - Quản lý chặt chẽ nhịp bất đồng bộ trong Tokio task để không làm block thread khi dùng dynamic delay.
