# HANDOFF: TS Online Dream Server — Tickets 01..08 & Login Refactor

> **Ngày tạo**: 2026-10-14  
> **Ngữ cảnh**: Xử lý toàn bộ tickets từ 01 đến 08 theo kế hoạch tại `.scratch/client-pseudo-op-code/rust_server_plan/README.md`, bổ sung cột `reputation`, chuẩn hóa hằng số wire protocol, refactor chuỗi `Logined1` và điều kiện hóa gửi kho lưu lãng (`17 66`) dựa trên database.

---

## 1. Tóm tắt công việc đã hoàn thành

### A. Database Migration & Reputation (`tiengtam`)
- **Tập tin**: [`migrations/0001_init.sql`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql)
- Đã thêm cột `reputation INTEGER DEFAULT 0` vào bảng `characters` kèm tài liệu chú thích rõ ràng.
- Đồng bộ hóa trường `reputation` trong [`src/db/modern/model.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/db/modern/model.rs) và [`src/db/modern/sqlite/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs) ánh xạ 1:1 với `Session.tiengtam`.

### B. Chuẩn hóa Hằng số Giao thức (Wire Opcodes & Magic Header)
- **Tập tin**: [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs)
- Định nghĩa tập trung các hằng số:
  - `TS_MAGIC_HEADER = "F444"`, `HEADER_TS_MAGIC = [0xF4, 0x44]`
  - `OPCODE_HOMDO = "1705"`, `OPCODE_TRANGBI = "170B"`, `OPCODE_TUIDEO = "172F"`, `OPCODE_LUULANG = "1766"`
  - `OPCODE_TIENTRANG = "1E01"`, `OPCODE_TIENTRANG_CLEAR = "1E08"`
  - `OPCODE_GOLD = "1A04"`, `OPCODE_HOTKEYS = "2801"`, `OPCODE_APPEAR = "03"`, `OPCODE_STATS = "0503"`, v.v.
- Loại bỏ toàn bộ magic strings phân tán trong [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs), [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs) và model container.

### C. Hàm dùng chung cho chuỗi đăng nhập (`Logined1`)
- **Tập tin**: [`src/server/handlers/login.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs)
- Tạo hàm dùng chung `pub fn send_logined_sequence(conn: &mut Conn, out: &mut HandleOutcome)` để đóng gói quy trình gửi sequence gói tin sau khi người chơi đăng nhập/tạo nhân vật thành công.
- Loại bỏ mã trùng lặp ở tất cả các nhánh login (`handle_login`, `handle_enter_game`, fallback golden).

### D. Refactor luồng gửi kho Lưu Lãng (`17 66`) từ Database
- **Vấn đề trước đây**: Dùng cờ tĩnh và biến môi trường `TS_SEND_LUULANG_FRAME` để bật/tắt gửi gói tin lưu lãng.
- **Giải pháp**:
  - Gỡ bỏ hoàn toàn `SEND_LUULANG_FRAME`, `should_send_luulang()`, `set_send_luulang_enabled()`, và `TS_SEND_LUULANG_FRAME`.
  - Trong [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs), frame `17 66` chỉ được phát khi nhân vật có item trong kho lưu lãng nạp từ DB (`s.luulang.iter().any(|i| i.id > 0)`).
  - Thêm test case `test_send_logined_sequence_luulang_behavior` trong [`tests/inventory_storage_type_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/inventory_storage_type_test.rs) để xác minh tự động chuyển đổi giữa 21 frame (không có item) và 22 frame (có item).

### E. Xử lý các Ticket 01 -> 08 (Inventories, Pet Equip, AutoSave, Starter Items)
- **Ticket 01 (Multi-container)**: Nạp và lưu đồng nhất 5 container (`homdo`, `tientrang`, `tuideo`, `trangbi`, `luulang`) qua `StorageType`.
- **Ticket 02 (Item hydration)**: Hàm `Session::hydrate_templates` nạp toàn bộ thuộc tính tĩnh từ `Item.dat`.
- **Ticket 03 (Equip/Unequip flow)**:
  - Sửa lỗi không persist ô rỗng khi trang bị vào slot trống.
  - Sửa lỗi ghi nhầm slot túi đồ vào bảng `trangbi` khi tháo đồ.
  - Thêm guard chỉ cho phép pet xuất chiến (`pet_stt` 1..=4) và slot hòm đồ hợp lệ (1..=25).
- **Ticket 04 (Stats Calculation)**: Tính toán lại HP/SP/ATK/DEF/INT/AGI sau khi thay đổi trang bị (`recompute_stats`).
- **Ticket 05 (Pet & Hotkeys)**: Đồng bộ pet xuất chiến (`13 01`), thống kê pet (`0F 08`), và hotkey bar (`28 01`).
- **Ticket 06 (Notice Toast)**: Gửi thông báo hệ thống qua Opcode `0x00`.
- **Ticket 07 (Starter items)**: Khởi tạo mặc định 4 item túi + 6 trang bị khi tạo nhân vật mới.
- **Ticket 08 (Deferred Capture)**: 9 mục cần capture thêm khi kết nối client live đã được tổng hợp tại [`.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md).

---

## 2. Trạng thái kiểm thử & Kiểm chứng (Verification)

- **Biên dịch**: `cargo check --all-targets` đạt 100%, **0 warning, 0 error**.
- **Test Suite**: `cargo test --all-targets --no-fail-fast` đạt **100% passed trên toàn bộ 28 integration test suites** (tất cả các test đều đặt trong `tests/`, không vi phạm quy tắc `src/` của `AGENTS.md`).
- **Git Commit**: Tuân thủ quy định **không tự ý commit git**. Mọi thay đổi vẫn đang ở dạng uncommitted working directory cho người dùng review.

---

## 3. Các tập tin trọng tâm đã chỉnh sửa / tạo mới

```text
migrations/
└── 0001_init.sql                                   # Thêm cột reputation vào bảng characters
src/
├── protocol/
│   └── mod.rs                                      # Khai báo tập trung hằng số Opcode & TS_MAGIC_HEADER
├── db/
│   ├── modern/model.rs                             # Mapping reputation và StorageType dump_code
│   └── modern/sqlite/session.rs                    # Load reputation & items từ DB
├── server/
│   ├── spawn.rs                                    # Dynamic luulang frame (any > 0), dùng hằng số opcode
│   ├── session.rs                                  # Dump container logic & hotkeys frame
│   └── handlers/
│       ├── login.rs                                # send_logined_sequence chung, bỏ env flag luulang
│       └── inventory.rs                            # Fix persist ô rỗng & bounds check equip
tests/
├── inventory_storage_type_test.rs                  # Test 21 vs 22 frame Logined1 luulang
└── pet_equip_unequip_test.rs                       # Test equip/unequip bounds & persist
```

---

## 4. Công việc tiếp theo (Next Steps)

1. **Kiểm thử với Client game thật (aLogin.exe / Delphi)**:
   - Chạy server Rust và kết nối bằng client TS Online.
   - Xác nhận chuyển map, nhận diện vật phẩm khởi tạo, và giao diện kho đồ hiển thị chính xác.
2. **Thực hiện checklist Issue 08**:
   - Khi có môi trường packet capture live, kiểm tra 9 mục trong [`.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/rust_server_plan/issues/08-deferred-needs-capture.md) (đặc biệt là độ bền trang bị `doben` và giao diện kho xe/ngựa `form_Horse`).

---

## 5. Suggested Skills cho Agent kế tiếp

- **`reverse-engineer-anything`**: Khi cần đối chiếu decompile các hàm xử lý gói tin của `aLogin.exe` hoặc kiểm tra cấu trúc bộ nhớ game client.
- **`antigravity-guide`**: Hướng dẫn điều phối công cụ và các quy chuẩn môi trường làm việc.
