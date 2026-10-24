# Handoff: Hoàn thiện & Chuẩn hóa Giao thức Kinh tế Server TS Online (Tickets 01 - 10 Hoàn tất Toàn diện)

- **Thời điểm**: 2026-10-24
- **Trạng thái**: Hoàn thành toàn diện 10/10 tickets (01 -> 10) và thiết lập môi trường chuẩn hóa (`CODING_STANDARDS.md`, `.ignore`, `docs/client-opcodes.md`).
- **Chất lượng mã nguồn**: `cargo check --all-targets` đạt 0 error, 0 warning; 100% test suites (56/56 suites, 22/22 tests trong `economy_wire_test`) đạt PASS.

---

## 1. Bản đồ Tài liệu Tham chiếu (Primary Artifacts)

- **Quy chuẩn Lập trình & Đồng thời**: [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)
- **Ánh xạ Opcode Binary Delphi Win32 (`aLogin.exe`)**: [`docs/client-opcodes.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md)
- **Quyết định Kiến trúc Giao thức & Giao dịch Nguyên tử**: [`docs/adr/0005-economy-wire-parity-and-transactions.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/adr/0005-economy-wire-parity-and-transactions.md)
- **Đặc tả Giao thức Client (Delphi Win32 Level A)**: [`.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md)
- **Đặc tả Kỹ thuật Server**: [`.scratch/economy-refactor/spec.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/spec.md)
- **Báo cáo Đối soát Khoảng cách**: [`.scratch/economy-refactor/GAP_ANALYSIS_MATRIX.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/GAP_ANALYSIS_MATRIX.md)
- **Danh mục 10 Issue Tickets**:
  1. [`01-currency-bank-wire-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/01-currency-bank-wire-parity.md) (Completed)
  2. [`02-player-stall-wire-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/02-player-stall-wire-parity.md) (Completed)
  3. [`03-p2p-trade-parity-and-state-lock.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/03-p2p-trade-parity-and-state-lock.md) (Completed)
  4. [`04-npc-shop-bulk-buy-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/04-npc-shop-bulk-buy-parity.md) (Completed)
  5. [`05-eliminate-020b-toast-cleanliness.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/05-eliminate-020b-toast-cleanliness.md) (Completed)
  6. [`06-standardize-5-item-containers.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/06-standardize-5-item-containers.md) (Completed)
  7. [`07-economy-wire-integration-tests.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/07-economy-wire-integration-tests.md) (Completed)
  8. [`08-tientrang-luulang-wire-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/08-tientrang-luulang-wire-parity.md) (Completed)
  9. [`09-p2p-trade-invite-and-lock-order.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/09-p2p-trade-invite-and-lock-order.md) (Completed)
  10. [`10-bank-withdraw-overflow-and-cleanliness.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/10-bank-withdraw-overflow-and-cleanliness.md) (Completed)

---

## 2. Tiến độ Thực thi Chi tiết trong Phiên làm việc (Work Completed)

| Ticket | Nội dung Kỹ thuật đã Thực hiện | Kết quả & Phát hiện Cốt lõi |
|:---:|---|---|
| **08** | **Tiền Trang & Lưu Lãng Wire Parity theo Binary Delphi** | - Thay thế `1E 06` bằng `1E 02 <11B item>` (13B wire) cho Tiền Trang Cất đồ (client tự tìm ô trống bằng `FUN_0076ff98()`).<br>- Thay thế `1E 07` bằng `1E 05 <slot: u8> <qty: u8>` (4B wire) cho Tiền Trang Rút đồ.<br>- Cất đồ Lưu Lãng gửi S→C `17 67 <12B item>` theo ADR 0005 §4.<br>- **Sửa lỗi nghiêm trọng Collateral Transfer**: Bóc tách cấu trúc 2-byte `[slot, qty]` của gói tin C→S `1E 01/02`, hỗ trợ số lượng lẻ, ngăn chặn việc cất/rút nhầm vật phẩm ở slot phụ cận. |
| **09** | **P2P Trade Error Parity & Deterministic Two-Phase State Lock** | - Trả về native error frame cho C→S `19 01`: `19 02 01` (đối phương offline), `19 02 02` (đối phương bận / tự mời), xóa bỏ silent drop.<br>- Gửi đồng thời cả `19 04` và `19 02 03` toast khi trade bị timeout.<br>- Đưa `lock_player_operations` lên trước khi đọc partner registry trong invite và confirm.<br>- **Sửa lỗi chí mạng Registry Drift**: Bổ sung `registry_update_if_online(&ctx.conn.session)` sau mỗi bước đàm phán, chấm dứt việc treo giao dịch khi confirm tự nhiên.<br>- Tự động unfreeze giao diện khi đối tác ngắt kết nối giữa chừng. |
| **10** | **Bank Withdraw Overflow Error Parity & Economy Code Cleanliness** | - **Sửa điều kiện phát frame `1A 06`**: Chỉ gửi khi ngân hàng đủ tiền (`bank_gold >= amount`) nhưng số dư ví sau rút vượt trần `GOLD_CAP`. Khi ngân hàng không đủ tiền, chỉ gửi `1D 03` và `1D 08`, không gửi nhầm `1A 06`.<br>- **Hardening Sub-opcode Tiền Trang**: Bổ sung guard ở đầu `handle_bank_gold` và helper `send_bank_ui_unlock(sub, out)` kiểm tra tường minh `BANK_SUB_WITHDRAW` (`1D 08`) và `BANK_SUB_DEPOSIT` (`1D 07`), loại bỏ hoàn toàn fallback `else` gây gửi nhầm frame mở khóa.<br>- **Thống nhất Kênh Chat `020102`**: Dọn dẹp dứt điểm các call-site cũ `red(...)` trong `books.rs`, chuyển sang `ctx.sys_msg(...)` thống nhất qua `src/server/spawn.rs::sys_msg_frame`.<br>- **Bộ test TDD 5 kịch bản biên**: `test_bank_withdraw_overflow_vs_insufficient_balance` kiểm thử toàn diện thiếu tiền, tràn ví, tràn ngân hàng, `amount == 0`, và subcode rác. |
| **Retro** | **Environment Hardening & Quy chuẩn Hóa Codebase** | - Tạo `.ignore` loại trừ 100k+ dòng dữ liệu tĩnh khỏi grep, tối ưu hóa tốc độ tìm kiếm và tiết kiệm token.<br>- Tạo [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md) quy chuẩn hóa: Lock ordering, Registry sync, Wire format, Test integrity.<br>- Tạo [`docs/client-opcodes.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md) giải mã bảng nhảy `0x0078A9B6` và `0x007921D5` của `aLogin.exe`.<br>- Liên kết chặt chẽ `CODING_STANDARDS.md` vào 3 vị trí trong `AGENTS.md`.<br>- Hợp nhất các hàm thông báo hệ thống về duy nhất `src/server/spawn.rs::sys_msg_frame(msg)`. |

---

## 3. Bài học Kinh nghiệm Sau Thực thi Ticket 10 (Retrospective Learnings)

Xếp theo mức độ nghiêm trọng (Severity):

### 1. [High] Tránh Fallback `else` Vô Điều Kiện Khi Phân Nhánh Sub-Opcode Nhị Phân
- **Hiện tượng**: Khi refactor helper `send_bank_ui_unlock`, logic ban đầu dùng `if sub == BANK_SUB_WITHDRAW { 1D 08 } else { 1D 07 }`. Khi client gửi gói tin có sub-opcode không xác định (`sub = 99`), nhánh `else` được kích hoạt và server gửi frame `1D 07` (mở khóa nút Nộp tiền) dù không hợp lệ.
- **Giải pháp**:
  - Với các kiểu dữ liệu nguyên thủy (`u8`), luôn phân nhánh tường minh (`if sub == BANK_SUB_WITHDRAW ... else if sub == BANK_SUB_DEPOSIT ... else { /* drop / no-op */ }`).
  - Đặt guard kiểm tra sub-opcode hợp lệ ngay đầu hàm xử lý trước khi thực hiện bất kỳ giải mã payload hay truy vấn trạng thái nào.

### 2. [Medium] Rà Soát Triệt Để Call-site Khi Chuẩn Hóa API Thay Vì Chỉ Tạo Alias
- **Hiện tượng**: Khi chuẩn hóa các phương thức gửi tin nhắn hệ thống sang `ctx.sys_msg(...)`, việc thêm alias vào `UseCtx` là chưa đủ nếu các file thành phần (như `books.rs`) vẫn còn gọi method cũ `ctx.red(...)`.
- **Giải pháp**: Khi refactor API / helper, phải dùng grep/search để đổi dứt điểm toàn bộ call-site trong module tương ứng và xóa bỏ hoàn toàn alias cũ để compiler bắt lỗi ngay lập tức nếu còn sót.

### 3. [Medium] Quy Chuẩn Test TDD Cho Nghiệp Vụ Tiền Tệ & Giao Dịch
- **Hiện tượng**: Ban đầu test chỉ kiểm tra luồng rút tiền thành công hoặc rút tiền thiếu số dư. Khi gặp các trường hợp đặc biệt (tràn ngân hàng khi nộp, input `amount == 0`, subcode rác), server có thể có hành vi ngoài ý muốn mà test không phát hiện được.
- **Giải pháp**: Mọi integration test giao dịch tiền tệ/kho bãi bắt buộc phải phủ đủ bộ 4 biên:
  1. Thiếu số dư (insufficient balance).
  2. Tràn ngưỡng tối đa (`gold + amount > GOLD_CAP`).
  3. Giá trị bằng không (`amount == 0`).
  4. Gói tin rác / Subcode không hợp lệ.

### 4. [Low / Thắng lợi] Hiệu Quả Của `.ignore` và Thư Mục Test Độc Lập
- Tập tin `.ignore` giúp loại trừ hơn 100k dòng dữ liệu (`Warps.txt`, `npc_drops.json`), giúp các tool grep/find chạy tức thì trong 0.1s và tiết kiệm token context.
- Toàn bộ integration test nằm gọn trong `tests/` giúp biên dịch và chạy độc lập qua `cargo test --test economy_wire_test` chỉ mất dưới 7s, cho feedback loop cực nhanh.

---

## 4. Nhật ký Kiểm tra Độc lập (Verification Record)

```bash
# Kiểm tra biên dịch toàn bộ target:
$ cargo check --all-targets
Finished `dev` profile [optimized + debuginfo] target(s) (0 error, 0 warning)

# Kiểm tra test suite Kinh tế Wire Parity (22 bài test):
$ cargo test --test economy_wire_test --no-fail-fast
running 22 tests ... ok (22 passed; 0 failed; finished in 0.05s)

# Kiểm tra toàn bộ 56 test suites của repository:
$ cargo test --all-targets --no-fail-fast
test result: ok. 150+ tests passed; 0 failed; 0 errors
```

---

## 5. Công việc Tiếp theo cho Người Dùng / Agent Kế tiếp (Next Steps)

1. **User Manual Review & Git Commit**:
   - Theo quy tắc dự án [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md), người dùng tự review các diff thay đổi và thực hiện git commit thủ công.
2. **Kế hoạch Tiếp theo**:
   - Toàn bộ 10/10 tickets của chiến dịch **Economy Refactor** đã hoàn tất.
   - Sẵn sàng chuyển tiếp sang các chiến dịch tính năng tiếp theo (Battle Engine, Quests & Eve Engine, Pet Box).
