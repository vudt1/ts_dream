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
  1. **Bước 1 (Đầu hiệp — Turn Prompt)**: Server phát gói tin `34 01` (`packets::your_turn()`) cho mọi người chơi còn sống phe ta và khởi động bộ đếm lùi thời gian (mặc định 20 giây). Luôn phát `34 01` kể cả khi Nhân vật/Sủng vật đang bị khóa lượt để client đồng bộ reset cờ lượt.
  2. **Bước 2 (Thu thập lệnh — Command Collection & Ngoại lệ Trạng thái Khóa Lượt)**:
     - Lắng nghe và gom `PlayerInput` gửi lên qua kênh `0x32` từ các đấu sĩ hợp lệ (`owns_commandable_cell`).
     - **Auto-Skip ô bị Khóa Lượt**: Giữ nguyên thứ tự `collect_commands` chạy trước `run_turn`, đồng thời dùng hàm dự đoán trạng thái sau tick đầu hiệp (tính cả bước giảm 1 lượt và giải hạn khi `turn <= 1` ở Pha 1) để loại bỏ mọi ô đang chịu *Khống chế cứng* (Type 3) hoặc *Hỗn loạn/Cuồng nộ* (`14021 | 20014`) khỏi `waiting_cells` và `fill_timeout_commands`. Nếu toàn bộ ô điều khiển được của phe người chơi đều bị khóa lượt, `collect_commands` kết thúc ngay (`0ms`) để bước sang Bước 3.
     - **Từ chối đồng bộ gói `0x32` của ô bị Khóa Lượt**: `BattleHandle` duy trì snapshot nhẹ tập ô bị khóa lượt (cập nhật đầu mỗi hiệp trước `34 01`) để `submit_command` kiểm tra O(1) đồng bộ. Mọi lệnh `0x32 sub 1/2` gửi cho ô đang bị khóa bị từ chối: không đưa vào `commands`, không phát `35 05` giả ở `sub 1`, không trừ vật phẩm ở `sub 2`, và gửi cảnh báo `sys_msg_frame` (`"Đang trong trạng thái khống chế, không thể hành động lượt này!"`) xuống client.
     - Với các ô không bị khóa chưa kịp ra lệnh khi hết 20 giây timeout, tự động gán đòn đánh cơ bản.
  3. **Bước 3 (Thực thi & Diễn hoạt — Action Execution)**: Gọi `battle.run_turn(...)`, phát tuần tự các gói tin diễn hoạt hoạt ảnh `35 03` (thế võ), `32 01` (chiêu thức), `33 01` (HP/SP), `35 0F` (icon trạng thái) ra `BattleSink`. Khi một đấu sĩ đồng thời dính *Khống chế cứng* (Type 3) và *Hỗn loạn/Cuồng nộ* (`14021 | 20014`), **Type 3 được ưu tiên trước** để bỏ qua lượt hoàn toàn (không chạy `auto_berserk`).
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
3. **Pha 3 (Tác động chiêu thức)**: Sau khi phát `32 01` (bao gồm cả hiệu ứng phụ Type 3 trên đòn đánh vật lý/phép thuật), nếu kỹ năng gây buff/debuff thành công (cả cast trực tiếp lẫn phản đòn) thì bắn `35 01` (`troi_start` cho lớp FX trạng thái) và `35 0F` (`battle_unit_status`) để cập nhật 6 khe biểu tượng HUD ('B'..'G') và số hiệp.
4. **Pha 4 (Dọn dẹp)**: Bắn `35 04` nếu có vật phẩm rơi, `35 06` nếu có thoại NPC mở màn/kịch bản, `35 0B` cho bảng tin chiến sự.

### 2.5. Phân loại Trận chiến (`BattleKind`) & Giao thức Mở trận PK (`0x0B Sub 02`)
- **Tách biệt `BattleKind` (`Normal` vs `Pk`) khỏi `EveFightData::fight_type`**: `fight_type` phản ánh phân loại trận của kịch bản `Data/eve.emg` (`0` khi không phải Eve, `1..=52` trong Eve) nên không đủ ngữ nghĩa phân biệt PK. Mọi trận đấu mang thêm phân loại `BattleKind`:
  - `BattleKind::Pk` được gán cho **tất cả** trận PvP (`0x0B 02 kind=2`) và **tất cả** trận PK Võ tướng trên bản đồ (`0x0B 02 kind=3`). Theo dịch ngược `FUN_00642720` và `FUN_00642c2c`, phía client kích hoạt cả hai nhánh qua cùng chế độ con trỏ PK (`*gvar_007D9D5C == 1`), nhưng chỉ nhánh PvP (`kind=2`) kiểm tra công tắc Option PK (`TCY_OptionForm + 0x180`); do đó server không đòi hỏi `session.pk == 1` đối với `0x0B 02 03`.
  - Trường 16-bit cuối của `0x0B 02 03` (`TFightManage + 0x22`) là **index slot NPC trên bản đồ (`1..=100`)**, không phải `DiaHinh` (server mặc định `DiaHinh = 112` cho `0x0B 02 02/03`).
  - Mọi trận `BattleKind::Pk` chia sẻ toàn bộ cơ chế combat/hợp kích/buff-debuff/`apply_catch`/`npc_respawn`, nhưng **cấm tuyệt đối** Drop, Gold, Player EXP, Pet EXP và không trừ lượt `Stat::God`.
- **Đối xứng hai phe trong PvP Party**: Hỗ trợ nạp đầy đủ Đội trưởng, Thành viên tổ đội cùng bản đồ và Sủng vật của cả hai phe (`Phe 1` tại hàng `3/2`, `Phe 2` đối xứng tại hàng `0/1`), phát `entity_player` (`0B 05 05`) cho cả hai phe, đồng bộ `battle_id` & kỹ năng của mọi thành viên tham chiến và dọn dẹp cả hai phe khi `Battle::finish`.

### 2.6. Chuẩn hóa Vai trò `0B FA` (`0x03`/`0x05` vs `0x04`) & Nghi thức Chạy trốn (`Flee`)
- **Khôi phục vai trò chuẩn trên `0B FA` (thay thế giải pháp tạm Ticket 15)**: Vì [`00590ba8`](../../client_pseudo_c/00590ba8_FUN_00590ba8.c) (`case 9`) kiểm tra `player+0x376 == 4` để gửi `0x0B 01 03` (thoát thẳng không theo lượt), mọi chiến đấu viên **phải** nhận dấu `0x03` (Đội trưởng) hoặc `0x05` (Thành viên) trên `0B FA`, chỉ giữ `0x04` cho *ListQS / Người quan sát*. Handler `0x0B 01 03` từ chối chiến đấu viên, chỉ cho phép Người quan sát rời trận trực tiếp; người chơi bấm nút Bỏ chạy trên UI thật sẽ gửi chuẩn `0x32 01 18001`.
- **Nghi thức Chạy trốn & Phạm vi tổ đội**:
  - Giữ công thức xác suất theo chênh lệch LV (`60 + (lv1 - lv2) + (lvtb1 - lvtb2)`), bỏ bypass thắng 100% của `14002` (*Thuật đào tẩu*) và thay bằng thưởng `+20%` với ngưỡng clamp `[0..=100]`.
  - Khi Flee thất bại (cho cả `18001` và `14002`), frame `32 01` luôn mang `skill_id = 18002` (`skill_type = 12`), **không** dùng `skill + 1` (vốn phát nhầm `14003` *Thuật mua bán*).
  - Khi Đội trưởng hoặc Sủng vật của Đội trưởng (kể cả một trong hai phe ở PvP) chạy trốn thành công, toàn bộ trận đấu kết thúc sạch cho cả hai phe; khi Thành viên hoặc Sủng vật của Thành viên chạy trốn thành công, chỉ rút sạch toàn bộ ô thuộc chủ sở hữu đó khỏi lưới (không ghi đè lại ô cũ, không kích hoạt `npc_respawn` giữa trận và gỡ đồng thời khỏi danh sách người tham gia trận).

---

## 3. Hệ quả & Cam kết Kỹ thuật (Consequences)

- **Tích cực**:
  - Client `aLogin.exe` thật kết nối vào sẽ hoạt động mượt mà, không còn hiện tượng treo đơ menu lệnh ở Hiệp 1, không bị drop lệnh đánh, và xem trọn vẹn hoạt ảnh giữa các hiệp; nút Bỏ chạy trên thanh công cụ đi đúng kênh `0x32 01 18001`.
  - Phân tách rạch ròi PvE chuẩn với PKBattle (cả PvP lẫn PK NPC) và hoàn thiện luồng PvP hai phe đối xứng.
- **Nhiệm vụ đi kèm**:
  - Cập nhật toàn bộ các test fixture trong `tests/` đang giả lập `0x0B Sub 3/4` chuyển sang `0x32 Sub 01/02`.
  - Cập nhật bộ test hợp đồng `0B FA` (`open_leader`, `open_self_kind`) và `0B 01 03` (`command_channel`) sang vai trò `0x03`/`0x05`/`0x04`.
  - Quản lý chặt chẽ nhịp bất đồng bộ trong Tokio task để không làm block thread khi dùng dynamic delay.
