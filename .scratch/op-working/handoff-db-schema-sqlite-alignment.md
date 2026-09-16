# HANDOFF — Hoàn Tất Phân Tích & Căn Chỉnh Toàn Diện Tầng Repository SQLite Với Schema 0001_init.sql (Hướng 1)

**Thời gian lập**: 2026-09-16  
**Module**: `ts_dream` (`migrations/0001_init.sql`, `src/db/`, `tests/db_repository_init_test.rs`)  
**Tài liệu tiền nhiệm**: `.scratch/op-working/handoff-login-char-creation.md`, `.scratch/op-working/handoff-login-handshake.md`  
**Trạng thái**: Đã hoàn thành toàn diện Hướng 1, 33/33 unit & integration tests pass (toàn bộ 7 test suite). Chưa commit git (tuân thủ quy định tại `AGENTS.md`, người dùng tự commit thủ công).

---

## 1. Bối Cảnh & Các Vấn Đề Đã Giải Quyết

Phiên làm việc tập trung vào việc giải đáp thắc mắc kiến trúc của người dùng và khắc phục lỗi bất đồng bộ nghiêm trọng giữa cơ sở dữ liệu và mã nguồn Rust:

### 1.1. Phân tích gói tin chào mừng `LOGIN_SCENE_GREETING` (`src/web/server_control.rs`)
- **Vấn đề người dùng đặt ra**: Nghi ngờ dòng mã `tx.send(LOGIN_SCENE_GREETING)` gửi gói tin mở Login Scene thiếu điều kiện kiểm tra client kết nối, dẫn đến "lúc nào cũng gửi".
- **Kết luận kiến trúc**:
  - Dòng mã nằm ở đầu `handle_client_connection`, chỉ kích hoạt **duy nhất 1 lần** trên mỗi socket TCP vừa hoàn tất bắt tay `accept()`. Hoàn toàn không lặp lại trong `while !close`.
  - Do `aLogin.exe` hoàn toàn thụ động (passive, 0-byte connect) khi kết nối mạng, Server bắt buộc phải là bên phát tín hiệu đầu tiên (*Server speaks first*). Nếu Server đợi Client gửi trước sẽ dẫn đến **Deadlock** (treo kết nối).
  - Kết nối TCP này được duy trì liên tục trong suốt phiên chơi (chuyển map, chat, chiến đấu), do đó gói tin này không bao giờ bị gửi lại giữa chừng.

### 1.2. Phát hiện lỗi Mismatch Schema `migrations/0001_init.sql` và Tầng Rust DB
- **Vấn đề**: File `migrations/0001_init.sql` đã được tái cấu trúc trước đó theo quy chuẩn đặt tên truyền thống của TS Online / Bear C# (`playerid`, `gender`, `jobtype`, `rebornstage`, `curhp`, `maxhp`, `freepoints`, `baseint`, `baseatk`, `mapid`...), trong khi mã nguồn Rust trong `src/db/modern/sqlite/`, `src/db/accounts.rs`, `src/db/persist.rs` vẫn giữ các câu lệnh SQL viết theo chuẩn `snake_case` cũ (`character_id`, `stat_point`, `int_attr`, `hp_max`, `sex`, `job`...).
- **Hậu quả nếu không xử lý**: Mặc dù unit test mock `ServerEnv::none()` trước đó pass do không chạm DB, nhưng khi khởi chạy server thực tế với file `DB/ts_dream.db`, mọi thao tác đăng nhập, tạo nhân vật, nạp session và lưu dữ liệu đều sẽ bị crash ngay lập tức do lỗi `no such column`.
- **Quyết định**: Người dùng đã chốt lựa chọn **Hướng 1** (giữ nguyên chuẩn schema `migrations/0001_init.sql` và refactor toàn bộ tầng truy vấn Rust DB cho khớp chính xác).

---

## 2. Chi Tiết Các Thay Đổi Trong Mã Nguồn

### 2.1. Cập nhật Lược Đồ Cơ Sở Dữ Liệu (`migrations/0001_init.sql`)
- Thêm cột `newbie INTEGER DEFAULT 0` vào bảng `characters` để hỗ trợ cờ nhiệm vụ tân thủ.
- Thêm các cột `thd INTEGER DEFAULT 0`, `texp INTEGER DEFAULT 0`, `quest INTEGER DEFAULT 0` vào bảng `character_pets` để hỗ trợ đầy đủ các trường dữ liệu của `PetState`.

### 2.2. Tái cấu trúc Toàn Bộ Tầng Repository SQLite (`src/db/`)
1. **`src/db/accounts.rs`**:
   - Chuyển toàn bộ query sang các cột chuẩn: `playerid`, `pass1`, `pass2`, `issuspended`, `gmlevel`, `createdat`, `updatedat`, `lastlogin_at`.
2. **`src/db/modern/sqlite/accounts.rs`**:
   - Đồng bộ `verify_pass1`, `verify_pass2`, `update_pass2`, `access`, `touch_login`, `set_gm_level`.
3. **`src/db/modern/sqlite/characters.rs`**:
   - Sử dụng `playerid` làm khóa chính/ngoại 1:1, khớp các cột `gender`, `jobtype`, `mapid`, `mapx`, `mapy`.
4. **`src/db/modern/sqlite/session.rs`**:
   - Viết lại câu lệnh `SELECT` nạp nhân vật: nạp chính xác `curhp`, `maxhp`, `cursp`, `maxsp`, `freepoints`, `skillpoint`, `baseint`, `baseatk`, `basedef`, `basehpx`, `basespx`, `baseagi`, `curexp AS texp`, `pk`, `newbie`.
   - Cập nhật `load_items` theo bảng `inventories`: `playerid`, `storagetype`, `slot`, `itemid`, `quantity`, `damage`.
   - Cập nhật `load_pets` theo bảng `character_pets`: khôi phục đúng chỉ số và cờ thú nuôi xuất chiến `isactive` vào `session.active_pet_stt`.
   - Viết lại hàm `save()` trong transaction atomic SQLite.
5. **`src/db/modern/sqlite/inventories.rs`, `pets.rs`, `quests.rs`**:
   - Đồng bộ hóa toàn diện tên cột và bảng.
6. **`src/db/persist.rs`**:
   - Cập nhật bảng tra cứu `character_column` (`"Hp" => "curhp"`, `"Int" => "baseint"`, `"Point" => "freepoints"`, `"SttPetXuatchien" => "isactive"`, `"Texp" => "curexp"`, `"Pk" => "pk"`...).
   - Bổ sung xử lý cập nhật `isactive` khi đổi tướng xuất chiến (`SttPetXuatchien`).
7. **Sửa 2 lỗi logic phát hiện trong quá trình rà soát**:
   - **Lỗi đảo dấu giao dịch rút tiền ngân hàng (`src/db/modern/transactions.rs`)**: Sửa hàm `bank_transfer` để tính toán đúng `gold_delta = -amount`, `bank_delta = amount`, `cost = amount.abs()`, đảm bảo rút tiền thì gold tăng, bank_gold giảm.
   - **Lỗi enum `StorageType` (`src/db/modern/model.rs`)**: Sửa giá trị `Bank = 2`, `Secondary = 4` để khớp chuẩn 1:1 với schema `inventories`.

### 2.3. Xây Dựng Bộ Integration Test Mới (`tests/db_repository_init_test.rs`)
Xây dựng 10 integration test case chạy trực tiếp trên SQLite in-memory (`sqlite::memory:?cache=shared`) thực thi trọn vẹn `0001_init.sql`:
- `test_account_creation_and_auth`: Tạo tài khoản (playerid >= 300000), xác thực pass1/pass2, cập nhật login, đổi pass.
- `test_character_creation_and_listing`: Tạo nhân vật, kiểm tra trùng tên bằng mã HEX VISCII, nạp tiền tệ ban đầu.
- `test_session_load_and_save`: Nạp và lưu đầy đủ session (5 loại kho đồ, danh sách võ tướng, kỹ năng, phím tắt hotkey, pk, texp, cờ xuất chiến).
- `test_inventory_repository_and_storage_types`: Kiểm thử lưu trữ độc lập giữa các kho đồ (Hành trang 1, Tiền trang 2, Túi đeo 4, Trang bị 8).
- `test_pet_repository_and_is_active`: Kiểm thử nạp/lưu trạng thái pet và cờ `isactive`.
- `test_atomic_transactions`: Kiểm thử giao dịch ngân hàng, mua hàng shop, và trao đổi P2P atomic giữa 2 người chơi.
- `test_persist_update_player_and_stt_pet`: Kiểm thử cập nhật nóng chỉ số PK, Texp, Pet xuất chiến qua tầng `persist`.
- `test_quest_repository_and_deletion`, `test_item_code_redeem_and_special_gift`, `test_nonexistent_and_edge_cases`.

---

## 3. Trạng Thái Mã Nguồn & Kiểm Thử

- **Kiểm thử tự động**: Chạy lệnh `cargo test --all-targets --no-fail-fast` $\to$ **33/33 tests PASSED** trên toàn bộ 7 test suite:
  - `tests/db_repository_init_test.rs`: 10 passed
  - `tests/encoding_test.rs`: 7 passed
  - `tests/login_char_flow_test.rs`: 5 passed
  - `tests/system_alert_test.rs`: 4 passed
  - `tests/rank_test.rs`: 3 passed
  - `tests/ground_test.rs`: 2 passed
  - `tests/warps_test.rs`: 2 passed
- **Linter**: `cargo clippy` đạt chuẩn không phát sinh lỗi hoặc cảnh báo mới trong toàn bộ các file sửa đổi.
- **Git Status**: Toàn bộ thay đổi nằm trong working tree, **chưa commit** (người dùng sẽ tự commit thủ công theo quy định tại `AGENTS.md`).

---

## 4. Danh Sách File Đã Thay Đổi & Tạo Mới

- `migrations/0001_init.sql`: Bổ sung cột `newbie` trong `characters`; `thd`, `texp`, `quest` trong `character_pets`.
- `src/db/accounts.rs`: Cập nhật các trường tài khoản.
- `src/db/item_code.rs`: Cập nhật trường `playerid`, `usedat`.
- `src/db/modern/model.rs`: Sửa enum `StorageType`, thêm `is_active` vào `PetRecord`.
- `src/db/modern/sqlite/accounts.rs`: Chuẩn hóa repository accounts theo `playerid`, `gmlevel`, `issuspended`...
- `src/db/modern/sqlite/characters.rs`: Chuẩn hóa repository characters theo `playerid`, `gender`, `jobtype`...
- `src/db/modern/sqlite/inventories.rs`: Chuẩn hóa repository inventories theo `playerid`, `storagetype`, `itemid`.
- `src/db/modern/sqlite/pets.rs`: Chuẩn hóa repository pets theo `character_pets` mới và cờ `isactive`.
- `src/db/modern/sqlite/quests.rs`: Chuẩn hóa repository quests theo `playerid`, `missionid`, `flagkey`...
- `src/db/modern/sqlite/session.rs`: Chuẩn hóa toàn diện nạp và lưu session nhân vật.
- `src/db/modern/transactions.rs`: Sửa lỗi tính toán đảo dấu rút tiền trong `bank_transfer`.
- `src/db/persist.rs`: Ánh xạ lại tên cột ghi-through.
- `tests/db_repository_init_test.rs`: File integration test mới (10 test cases).

---

## 5. Hướng Dẫn Tiếp Tục (Cho Session / Agent Kế Tiếp)

### Các Bước Ưu Tiên Tiếp Theo:
1. **Kiểm thử thực tế với Client `aLogin.exe` & SQLite DB**:
   - Chạy server: `cargo run`.
   - Kết nối `aLogin.exe`:
     - Kiểm tra màn hình đăng nhập hiển thị ngay lập tức.
     - Đăng nhập tài khoản mới $\to$ tạo nhân vật mới $\to$ kiểm tra dữ liệu được ghi đúng vào file `DB/ts_dream.db`.
     - Thoát client và đăng nhập lại $\to$ xác nhận nạp lại đúng chỉ số và vị trí nhân vật từ DB.
2. **Triển khai các tính năng in-game kế tiếp**:
   - Di chuyển và chuyển map (Opcode `0x05`, `0x06` - Movement & Warp map).
   - Hệ thống chat và lệnh slash (Opcode `0x02`).
   - Tương tác NPC và kịch bản nhiệm vụ Eve Engine (Opcode `0x18`).

---

## 6. Suggested Skills Cho Phiên Kế Tiếp

- **`research`**: Dùng để tra cứu mã decompile trong `client_pseudo_c/` hoặc các file spec trong `spec/exhaustive_by_opcode/` khi tiếp tục triển khai các opcode in-game (0x05 di chuyển, 0x06 warp map, 0x18 tương tác NPC).
- **`plan`**: Lập kế hoạch chi tiết cho việc đồng bộ hóa vị trí đa người chơi (multiplayer walk broadcasting) và chuyển map (warp gate handler).
