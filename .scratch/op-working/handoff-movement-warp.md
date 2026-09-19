# HANDOFF — Nghiên Cứu, Thiết Kế & Triển Khai Di Chuyển (Opcode 0x06) & Warp Map (Opcode 0x0C, 0x14, 0x05)

**Thời gian cập nhật**: 2026-09-19  
**Module**: `ts_dream` (`src/server/handlers/movement.rs`, `src/server/handlers/quest.rs`, `src/server/handlers/system.rs`, `src/server/dispatcher.rs`, `src/server/auto_save.rs`, `src/web/server_control.rs`, `src/server/spawn.rs`, `src/server/map_drops.rs`)  
**Tài liệu tiền nhiệm**: `.scratch/op-working/handoff-login-char-creation.md`  
**Trạng thái**: **ĐÃ HOÀN TẤT TRIỂN KHAI & VERIFY (100% PASS)**. Toàn bộ 5 bước cốt lõi về Opcode 0x05, 0x06, 0x0C warp chu trình đầy đủ, đồng bộ thực thể map mới, party follow/warp, autosave dirty fingerprint và SQLite persistence khi ngắt kết nối đã được đưa vào codebase và vượt qua toàn bộ 14 test suites.

---

## 1. Bối Cảnh & Mục Tiêu

Tiếp nối phiên làm việc hoàn tất màn hình Login và Tạo nhân vật, người chơi đã được đưa vào map tân thủ Trác Quận (`10817`).  
Mục tiêu đã hoàn thành trong phiên làm việc này:
1. Nghiên cứu sâu và phân định rõ ràng giữa Opcode `0x05` (Non-movement / Actor state sync) và Opcode `0x06` (Movement & Remote walk).
2. Đối chiếu toàn diện giữa 4 nguồn (Rust server, TS_Server_Bear C#, Client decompile C/x86 ASM, Đặc tả `.scratch/client-pseudo-op-code/`).
3. Chuẩn hóa tài liệu đặc tả: `.scratch/client-pseudo-op-code/opcode_05.md` và `opcode_06.md`.
4. Triển khai hoàn chỉnh cơ chế di chuyển đơn vị, party follow, chu trình Warp Map 0x0C chuẩn, đồng bộ thực thể sau khi nạp map mới.
5. Vá triệt để lỗi mất tọa độ trong database (bổ sung fingerprint dirty và persist ngay khi disconnect).
6. Viết bộ integration test chuyên sâu `tests/movement_warp_test.rs` gồm 8 test cases và đảm bảo không có warning / compile error.

---

## 2. Kết Quả Đối Chiếu & Bản Chất Nghiệp Vụ

### 2.1. Phân Biệt Tuyệt Đối Giữa Opcode 0x05 Và Opcode 0x06
Trong client `aLogin.exe` (`0077f414_FUN_0077F414.c`, `case_006_*.c`, `case_007_*.c`), hai opcode này hoàn toàn độc lập:

- **Opcode 0x05 (S$\to$C: World & Actor State Sync)**:
  - Chiều C$\to$S: Client **không dùng để di chuyển**. Chỉ gửi khi chọn mục trên form (`05, sub=6`) hoặc đổi tab panel (`05, sub=7`).
  - Chiều S$\to$C: Kênh server push cập nhật thực thể:
    - `[05 04]`: **World-Ready** (`scene+0x53fc = 1`, reset mode chuột `TMouseInfo`). Bắt buộc gửi sau spawn hoặc sau khi warp để mở khóa điều khiển chuột & phím.
    - `[05 0A [charID: 4B LE]]`: Hiệu ứng thăng cấp `"LevelUP"` + âm thanh `sound\WA0013.wav`.
    - `[05 01/02]`: Tháo/mặc 1 món trang bị của actor từ xa.
    - `[05 00]`: Đặt lại toàn bộ danh sách trang bị của actor từ xa.
    - `[05 03]`: Snapshot toàn bộ state của self (Class, HP, stats, job-state).
    - `[05 05]`: Thay đổi/phục hồi diện mạo.
    - `[05 08]`: Patch byte cờ cache `rec+0x8c`.
    - `[05 09]`: Đổi tên/refresh đệ tử/follower.
- **Opcode 0x06 (C$\leftrightarrow$S: Movement & Remote Walk)**:
  - **Chiều C$\to$S (Client báo cáo bước đi — đúng 9 bytes payload)**:
    ```text
    [0x06] [sub: 1B] [orient: 1B] [X: Word LE (2B)] [Y: Word LE (2B)] [sigA: 1B] [sigB: 1B]
    ```
    - `sub`: `0x01` (bước đi tự nguyện: click chuột, bàn phím, cưỡi ngựa); `0x02` (echo vị trí do server ép).
    - `orient`: Hướng nhìn của nhân vật (`player + 0xE4`).
    - `X`, `Y`: Tọa độ đích đến (pixel, little-endian).
    - `sigA`: Chữ ký xác thực: `(charID % 13 + job_state_byte[0x3FA]) & 0xFF`.
    - `sigB`: Nonce ngẫu nhiên PRNG LCG (`0x8088405`).
  - **Chiều S$\to$C (Server điều khiển di chuyển)**:
    - `SubOp 0x01` (11 bytes): `[06][01][actorID: 4B LE][dir: 1B][destX: 2B LE][destY: 2B LE]` — Điều khiển actor từ xa đi tới đích. Client Leader tự filter gói này nếu `actorID` là thành viên trong nhóm của mình.
    - `SubOp 0x02` (2 bytes): `[06][02]` — Khóa di chuyển (`player+0x653 = 1`), ép client dừng tại chỗ và trả ngay gói C$\to$S `[06][02]...` để reconcile vị trí. Khóa chỉ được mở bằng `[14 08]`.

### 2.2. Chu Trình Warp Map Chuẩn (Chuyển Bản Đồ)
Theo Bear C# (`TSMap.cs`, `TSWorld.cs`, `RelocateHandler.cs`) và Client x86:
1. Client chạm cổng / click warp $\to$ gửi `[14 08 idGate]`.
2. Server gửi `[14 07]` (bắt đầu chuyển cảnh/fade màn hình).
3. Server gửi **Opcode 0x0C (Relocate Map — 13 bytes)**:
   ```text
   F4 44 0D 00 0C [accID: 4B LE] [mapID: 2B LE] [destX: 2B LE] [destY: 2B LE] [warpID: 1B] [00]
   ```
4. Server broadcast xóa/ẩn player khỏi bản đồ cũ (`packets::hide_from_map`).
5. Client nạp map mới xong $\to$ gửi C$\to$S `[0C 01]` (Relocate Loaded / Confirm).
6. Server gửi `[05 04]` (World-Ready) và `[14 08]` (Clear walk lock).
7. Server gửi danh sách player đang online trên map mới cho người mới đến, và broadcast người mới đến cho các player cũ trên map.
8. Server gửi danh sách item rơi trên map (`ItemOnMap`) của map mới.

---

## 3. Các Điểm Đã Xử Lý & Khắc Phục

1. **Dispatcher & Handler Opcode 0x05 & 0x06**:
   - Tách `OP_PLAYER_UPDATE (0x05)` gọi `movement::handle_player_update`.
   - Định tuyến `OP_MOVE (0x06)` độc quyền vào `movement::handle_move`.
   - Bóc tách đầy đủ cấu trúc 9 bytes của `0x06`, hỗ trợ cả `sub == 1` (walk) và `sub == 2` (reconcile echo trả `1408`).
2. **Party Movement & Follow Constraint**:
   - Đặt guard clause chặn thành viên tự ý di chuyển độc lập khi đang trong nhóm (`id_leader > 0 && id_leader != id`).
   - Khi Leader di chuyển, tự động cập nhật tọa độ đồng thời cho các thành viên trong nhóm và phát broadcast trong map.
3. **Chu Trình Warp Map (Opcode 0x0C)**:
   - Viết `spawn::build_relocate_packet` (Opcode 0x0C, 13 bytes).
   - Trong `handle_warp_confirm`: gửi fade `1407`, gửi `0x0C`, broadcast `hide_from_map` tới các người chơi ở map cũ.
   - Cập nhật ngay `online_sessions()` của người kích hoạt warp và toàn bộ thành viên trong nhóm.
   - Hỗ trợ Party Warp: Leader kích hoạt warp sẽ tự động kéo toàn bộ thành viên đang online đi theo.
4. **Đồng Bộ Thực Thể Map Mới Sau Khi Nạp Xong (`handle_teleport_confirm`)**:
   - Khi nhận `0x0C 0x01`: Đặt `in_world = true`, gửi `[14 08]` mở khóa bước đi.
   - Broadcast sự xuất hiện (`player_appear`) tới những người chơi đang có mặt ở map mới.
   - Gửi danh sách các người chơi khác cùng map về cho client vừa vào map.
   - Đồng bộ danh sách NPC và vật phẩm rơi trên mặt đất qua `map_drops::drops_on_map`.
5. **Persistence Tọa Độ Vào SQLite**:
   - Bổ sung `map_id`, `map_x`, `map_y` vào hàm `fingerprint` của `auto_save.rs` để phát hiện trạng thái dirty khi di chuyển.
   - Trong `server_control.rs::disconnect_player`: Lưu ngay lập tức session vào SQLite qua `persist_sessions_transaction` trước khi xóa khỏi bộ nhớ RAM.
6. **Thread-Safety trong `broadcast_map`**:
   - Giải phóng `MutexGuard` của `online_sessions` trong scope block đóng trước khi gọi async `.await` trên `self.clients`, đảm bảo an toàn luồng và trait `Send`.

---

## 4. Danh Sách Tập Tin Đã Thay Đổi & Tạo Mới

### Mã nguồn & Kiểm thử:
- [`src/server/dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs): Bổ sung trường `map_id: Option<u16>` vào `MapBroadcast`, tách định tuyến `OP_PLAYER_UPDATE` và `OP_MOVE`.
- [`src/server/handlers/movement.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs): Thêm `handle_player_update`, chuẩn hóa `handle_move` với guard clause party follow và `sub == 2` echo reconciliation.
- [`src/server/handlers/quest.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs): Tái cấu trúc `handle_warp_confirm` thành async, phát fade `1407`, `0x0C`, `hide_from_map`, cập nhật `online_sessions` và hỗ trợ party warp follow.
- [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs): Truyền `env` vào `handle_talk_warp` để hỗ trợ async warp dispatch.
- [`src/server/handlers/system.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs): Triển khai đầy đủ `handle_teleport_confirm` (xuất hiện người chơi, gửi danh sách người chơi cùng map, NPC, map drops).
- [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs): Thêm hàm `build_relocate_packet` (Opcode 0x0C).
- [`src/server/map_drops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/map_drops.rs): Thêm hàm `drops_on_map(map_id)`.
- [`src/server/auto_save.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/auto_save.rs): Thêm `map_id`, `map_x`, `map_y` vào `fingerprint`.
- [`src/web/server_control.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs): Scope guard cho `broadcast_map` và persist dữ liệu khi `disconnect_player`.
- [`src/server/handlers/chat.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/chat.rs): Cập nhật `MapBroadcast` initializer.
- [`tests/movement_warp_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/movement_warp_test.rs): Bộ kiểm thử tích hợp 8 test cases hoàn chỉnh.

### Tài liệu đặc tả:
- [`.scratch/client-pseudo-op-code/opcode_05.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_05.md): Cảnh báo non-movement, làm rõ sub 6/7, world-ready gate `05 04`.
- [`.scratch/client-pseudo-op-code/opcode_06.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_06.md): Chuẩn hóa 9 bytes payload, giải thích `orient`, leader client-side filtering, sub 2 echo closed-loop.

---

## 5. Kết Quả Kiểm Thử (Verification Record)

Chạy lệnh kiểm thử toàn diện toàn bộ dự án:
```bash
cargo test --all-targets --no-fail-fast
```
Kết quả: **14/14 test suites passed (100%)**:
1. `tests/create_char_atomic_test.rs` (5 tests passed)
2. `tests/db_repository_init_test.rs` (11 tests passed)
3. `tests/encoding_test.rs` (7 tests passed)
4. `tests/ground_test.rs` (2 tests passed)
5. `tests/login_char_flow_test.rs` (5 tests passed)
6. `tests/movement_warp_test.rs` (8 tests passed: movement, party follow, sub 2 echo, warp confirm, teleport confirm cycle, party warp, dirty fingerprint, disconnect persistence)
7. `tests/p0_p1_p2_roadmap_test.rs` (10 tests passed)
8. `tests/rank_test.rs` (3 tests passed)
9. `tests/system_alert_test.rs` (4 tests passed)
10. `tests/warps_test.rs` (2 tests passed)
11. `tests/wiring_hotkey_notice_test.rs` (6 tests passed)
12. `examples/verify_skill_pc.rs` (passed)
13. `src/lib.rs` & `src/main.rs` (passed)

Kiểm tra `cargo clippy`: Không còn cảnh báo nào phát sinh từ các đoạn mã mới triển khai.

---

## 6. Hướng Dẫn & Khuyến Nghị Cho Phiên Làm Việc Kế Tiếp

1. **Random Encounter khi di chuyển**:
   - Hiện tại người chơi đã có thể di chuyển và đổi bản đồ tự do. Bước tiếp theo là liên kết số bước đi trong `handle_move` với bảng Encounter (trong `eve.emg` hoặc cấu hình map) để kích hoạt trận chiến ngẫu nhiên dã ngoại (Opcode `0x0B` / `0x32`).
2. **Kiểm thử End-to-End với Client Thật**:
   - Chạy server và đăng nhập bằng client `aLogin.exe`, thử nghiệm:
     - Dùng chuột/bàn phím click di chuyển trong Trác Quận.
     - Tạo nhóm 2 người chơi, di chuyển kiểm tra tính năng leader kéo thành viên đi theo.
     - Đi qua cổng dịch chuyển ra ngoại thành và kiểm tra hiệu ứng fade màn hình, chuyển cảnh mượt mà, đồng bộ NPC và người chơi khác.
     - Thoát game và đăng nhập lại để xác nhận nhân vật vẫn ở đúng tọa độ map mới trong database.

---

## 7. Suggested Skills

- **`cuder`**: Tiếp tục triển khai logic Rust hiệu năng cao cho hệ thống combat ngẫu nhiên dã ngoại (Random Encounter).
- **`backend_architect`**: Tối ưu hóa việc phân chia spatial partitioning (grid / chunk) nếu số lượng người chơi trên một map tăng cao để giảm tải fanning out trong `broadcast_map`.
- **`research`**: Phân tích cấu trúc dữ liệu Encounter section trong file nhị phân `Data/eve.emg`.
