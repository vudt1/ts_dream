# Implementation Plan: Battle Protocol Parity, Map #12808 Interact Range & SSE Live Export

## Goal Description
Khắc phục triệt để các lỗi giao thức gây hiện tượng "thua trận sớm / không thấy đánh / mất animation" trong trận đấu, mở rộng bán kính tương tác NPC Map #12808 lên `150px` (vẫn giữ `12808.eve` làm Single Source of Truth), và bổ sung nút **Export Packets** tại tab `Live SSE Terminal` trên [`dashboard.html`](file:///D:/VUDT/GIT_PCC/ts_dream/templates/dashboard.html).

> [!NOTE]
> **Kết quả kiểm chứng từ mã nguồn & byte gốc**:
> - **Tọa độ `0B 05` vs `32 01`**: Trong gói `[3]` (`0B05 03 03 F7550000 010000000000 0003 ...`), byte `03` thứ hai là `typ = 3` (NPC), theo sau bởi `id` (4B) + 6 bytes `010000000000` + `row=00, col=03`. Vị trí quái đã đúng là `Row 0, Col 3` khớp với `c2s 32 01`, không cần sửa tọa độ lưới.
> - **Pacing `34 01`**: [`manager.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/manager.rs#L437-L443) đã có sẵn `tokio::time::sleep` với `clamp(sum(delay) * 100ms, 800ms, 3500ms)` sau khi gửi `32 02` và trước khi gửi `34 01`. Giữ nguyên sàn `800ms`.

---

## Proposed Changes

### 1. Map #12808 & NPC Interact Radius (`150px`)
Giữ nguyên cơ chế Data-Driven từ `Data/eve.emg` (`12808.eve`), không hardcode danh sách ngoại lệ cho Người Rơm (`#22012`). Nâng bán kính kiểm tra tương tác từ `130px` lên `150px`.

#### [MODIFY] [state.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/state.rs)
- Cập nhật `INTERACT_RADIUS_PX` từ `130` lên `150` (và cập nhật docstring tương ứng).

```rust
pub const INTERACT_RADIUS_PX: i64 = 150;
const MAX_INTERACT_DISTANCE_SQ: i64 = INTERACT_RADIUS_PX * INTERACT_RADIUS_PX;
```

---

### 2. Battle Protocol Sequencing (`33 01` sau `32 01`)
Đảm bảo trong mọi hành động chiến đấu (kể cả chuỗi hợp kích/combo), các frame `33 01` (cập nhật HP/SP tức thời) chỉ được phát **ngay sau** khi frame diễn hoạt `32 01` được ghi ra buffer, tránh việc client `aLogin.exe` (`FUN_0064cae0`) gục nhân vật hoặc trừ máu trước khi animation đánh diễn ra. Các `33 01` ở Pha 1 (độc/đốt đầu lượt) và Pha 4 (hồi SP quân sư/nội tại cuối lượt, không có `32 01`) giữ nguyên phát trực tiếp.

#### [MODIFY] [runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs)
1. Thêm `stat_frames: Vec<String>` vào `TurnState`.
2. Tách/mở rộng helper `write_hp_with_buf` / `write_sp_with_buf` nhận `stat_buf: Option<&mut Vec<String>>`:
   - Vẫn đẩy `Out::Db(...)` vào `out` ngay lập tức như cũ.
   - Nếu `Some(buf)` -> đẩy `packets::set_hp` / `packets::set_sp` vào `buf` (`ts.stat_frames`); nếu `None` (Pha 1 / Pha 4) -> đẩy trực tiếp `Out::Broadcast(...)` vào `out`.
3. Chuyển các lời gọi `write_hp` / `write_sp` trong phạm vi hành động (`execute_action`, `apply_attack`, `apply_magic`, `apply_target_hp`, v.v.) sang dùng `&mut ts.stat_frames`.
4. Tại điểm flush `32 01` (khi `!ts.text9.is_empty() && ts.combo_active == 0`):
   - Phát `35 03` (`force_state7`), phát `32 01`, sau đó `std::mem::take(&mut ts.stat_frames)` đẩy toàn bộ `33 01` ra `out`, rồi mới tới `status_frames` và `text11`.
5. Trong `emit_phase4_cleanup` (trước `turn_action_finish` `32 02`), drain nốt `ts.stat_frames` nếu còn sót (safety net cho các nhánh không sinh `text9`).

---

### 3. Buffer Frame `08 01` Trong Thời Gian Chiến Đấu
Ngăn chặn hoàn toàn việc tiêm frame ngoài bản đồ `F4440C000801` (`08 01`) vào giữa luồng battle (gây reset UI/hộp thoại trong `FUN_0078d125` của client).

#### [MODIFY] [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs)
- Thêm trường `pub pending_stat_frames: Vec<String>` vào struct `Session` (khởi tạo `Vec::new()` trong `Session::new` / `Default`).

#### [MODIFY] [service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs)
1. Trong `apply_db_update`:
   - Khi `s.battle_id > 0`: cập nhật `Session` trong RAM như cũ nhưng đẩy `f` vào `s.pending_stat_frames.push(f)` thay vì `p.frames.try_send(f)`.
   - Khi `s.battle_id <= 0`: giữ nguyên `p.frames.try_send(f)`.
2. Trong `battle_ended`:
   - Ngay sau khi hạ cờ `s.battle_id = 0;`, drain toàn bộ `s.pending_stat_frames.drain(..)` gửi ra `tx.try_send(f)` trước khi xử lý bước tiếp theo (kể cả khi `resume_eve_for_participant` nối chuỗi sang trận mới).

---

### 4. Nút Export Packets Tại Tab SSE Live
Thêm nút **Export Packets** cạnh `Clear Console` trên [`dashboard.html`](file:///D:/VUDT/GIT_PCC/ts_dream/templates/dashboard.html) để xuất toàn bộ gói tin `c2s` và `s2c` đang hiển thị trên màn hình.

#### [MODIFY] [dashboard.html](file:///D:/VUDT/GIT_PCC/ts_dream/templates/dashboard.html)
- Thêm nút `<button class="btn btn-blue" style="padding: 6px 12px; font-size: 12px;" onclick="exportPackets()">Export Packets</button>` cạnh nút `Clear Console`.
- Thêm hàm JS `exportPackets()`:
  - Duyệt qua các `.log-line` trong `#terminal-console`.
  - Chỉ lấy các dòng có `.log-level` là `c2s` hoặc `s2c` (case-insensitive, trim).
  - Bỏ qua `.log-ts` (timestamp), không tạo dòng header.
  - Format mỗi dòng thành `<level> <msg>` (ví dụ: `c2s [127.0.0.1:62866] F4440A003201...` hoặc `s2c [127.0.0.1:62866] F44404003505...`).
  - Nếu không có gói tin nào, không tải file rỗng (có thể `alert` nhẹ hoặc bỏ qua).
  - Tạo `Blob` `text/plain;charset=utf-8` và trigger tải xuống với tên `sse-packets-<YYYYMMDD-HHMMSS>.txt`.

---

## Verification Plan

### Automated Tests
1. **Unit & Domain Integration Tests**:
   - Chạy `cargo test --test eve_quest_test` (bao gồm `test_map_12808_straw_dummy_resolution` và kiểm tra ngưỡng `150px` của `is_within_range`).
   - Chạy `cargo test --test battle_test` để xác minh toàn bộ chuỗi packet `32 01` -> `33 01` -> `32 02`, combo, và không phát `08 01` khi `battle_id > 0` mà xả đầy đủ sau `battle_ended`.
   - Bổ sung unit/integration test mới cho:
     - `is_within_range` tại ranh giới `150px` (trong `150px` pass, `151px` fail).
     - Thứ tự frame `32 01` xuất hiện trước `33 01` trong `TurnState` khi tấn công/thi triển kỹ năng.
     - `08 01` được buffer vào `pending_stat_frames` khi `battle_id > 0` và được xả ra khi `battle_ended`.
2. **Full Regression Before Handoff**:
   - Chạy `cargo test --all-targets --no-fail-fast`.

### Manual Verification
1. Mở Web Admin (`http://127.0.0.1:8090`), vào tab **Live SSE Terminal**, kết nối client thật đánh Người Rơm tại Map #12808:
   - Kiểm tra nhân vật đứng trong bán kính `150px` click Người Rơm vào trận bình thường.
   - Quan sát luồng packet trên SSE Live: `35 05` -> `35 03` -> `32 01` -> `33 01` -> `32 02`, không có `08 01` xen giữa, `08 01` được xả sau khi kết thúc trận.
   - Bấm **Export Packets** và kiểm tra file `.txt` tải về chỉ chứa các dòng `c2s [ip:port] <hex>` / `s2c [ip:port] <hex>`, không có header và không có cột timestamp.
