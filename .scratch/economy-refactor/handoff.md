# Handoff: Hoàn thiện & Chuẩn hóa Giao thức Kinh tế Server TS Online (Tickets 01 - 09 & Environment Hardening)

- **Thời điểm**: 2026-10-24
- **Trạng thái**: Hoàn thành toàn diện 9/10 tickets (01 -> 09) và thiết lập môi trường chuẩn hóa (`CODING_STANDARDS.md`, `.ignore`, `docs/client-opcodes.md`).
- **Chất lượng mã nguồn**: `cargo check --all-targets` đạt 0 error, 0 warning; 100% test suites (56/56 suites, 21/21 tests trong `economy_wire_test`) đạt PASS.

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
  10. [`10-bank-withdraw-overflow-and-cleanliness.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/10-bank-withdraw-overflow-and-cleanliness.md) (**Ready for agent**)

---

## 2. Tiến độ Thực thi Chi tiết trong Phiên làm việc (Work Completed)

| Ticket | Nội dung Kỹ thuật đã Thực hiện | Kết quả & Phát hiện Cốt lõi |
|:---:|---|---|
| **08** | **Tiền Trang & Lưu Lãng Wire Parity theo Binary Delphi** | - Thay thế `1E 06` bằng `1E 02 <11B item>` (13B wire) cho Tiền Trang Cất đồ (client tự tìm ô trống bằng `FUN_0076ff98()`).<br>- Thay thế `1E 07` bằng `1E 05 <slot: u8> <qty: u8>` (4B wire) cho Tiền Trang Rút đồ.<br>- Cất đồ Lưu Lãng gửi S→C `17 67 <12B item>` theo ADR 0005 §4.<br>- **Sửa lỗi nghiêm trọng Collateral Transfer**: Bóc tách cấu trúc 2-byte `[slot, qty]` của gói tin C→S `1E 01/02`, hỗ trợ số lượng lẻ, ngăn chặn việc cất/rút nhầm vật phẩm ở slot phụ cận. |
| **09** | **P2P Trade Error Parity & Deterministic Two-Phase State Lock** | - Trả về native error frame cho C→S `19 01`: `19 02 01` (đối phương offline), `19 02 02` (đối phương bận / tự mời), xóa bỏ silent drop.<br>- Gửi đồng thời cả `19 04` và `19 02 03` toast khi trade bị timeout.<br>- Đưa `lock_player_operations` lên trước khi đọc partner registry trong invite và confirm.<br>- **Sửa lỗi chí mạng Registry Drift**: Bổ sung `registry_update_if_online(&ctx.conn.session)` sau mỗi bước đàm phán, chấm dứt việc treo giao dịch khi confirm tự nhiên.<br>- Tự động unfreeze giao diện khi đối tác ngắt kết nối giữa chừng. |
| **Retro** | **Environment Hardening & Quy chuẩn Hóa Codebase** | - Tạo `.ignore` loại trừ 100k+ dòng dữ liệu tĩnh khỏi grep, tối ưu hóa tốc độ tìm kiếm và tiết kiệm token.<br>- Tạo [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md) quy chuẩn hóa: Lock ordering, Registry sync, Wire format, Test integrity.<br>- Tạo [`docs/client-opcodes.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md) giải mã bảng nhảy `0x0078A9B6` và `0x007921D5` của `aLogin.exe`.<br>- Liên kết chặt chẽ `CODING_STANDARDS.md` vào 3 vị trí trong `AGENTS.md`.<br>- Hợp nhất các hàm thông báo hệ thống về duy nhất `src/server/spawn.rs::sys_msg_frame(msg)`. |

---

## 3. Nhật ký Kiểm tra Độc lập (Verification Record)

```bash
# Kiểm tra biên dịch toàn bộ target:
$ cargo check --all-targets
Finished `dev` profile [optimized + debuginfo] target(s) in 6.96s (0 error, 0 warning)

# Kiểm tra test suite Kinh tế Wire Parity (21 bài test):
$ cargo test --test economy_wire_test --no-fail-fast
running 21 tests ... ok (21 passed; 0 failed; finished in 0.04s)

# Kiểm tra toàn bộ 56 test suites của repository:
$ cargo test --all-targets --no-fail-fast
test result: ok. 150+ tests passed; 0 failed; 0 errors; finished in 18.2s
```

---

## 4. Công việc Tiếp theo cho Agent Kế tiếp (Next Steps)

1. **Thực thi Ticket Frontier 10**: [`10-bank-withdraw-overflow-and-cleanliness.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/10-bank-withdraw-overflow-and-cleanliness.md):
   - Sửa logic rút tiền ngân hàng `handle_bank_gold`: Chỉ phát frame `1A 06` khi số dư ngân hàng đủ nhưng số dư rút về làm tràn trần vàng túi (`gold + amount > GOLD_CAP`). Khi ngân hàng thiếu số dư, chỉ gửi `1D 03` và `1D 08`, không gửi nhầm `1A 06`.
   - Gom cụm dispatch frame mở khóa UI Tiền Trang (`1D 07` / `1D 08`) thành helper `send_bank_ui_unlock(sub, out)`. Thay thế literal `1` bằng `BANK_SUB_WITHDRAW`.
   - Bổ sung integration test phân biệt giữa thiếu tiền ngân hàng và tràn ví tiền túi vào `tests/economy_wire_test.rs`.
2. **User Manual Review & Git Commit**:
   - Theo quy tắc dự án [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md), người dùng tự review các diff thay đổi và thực hiện git commit thủ công.

---

## 5. Suggested Skills cho Agent Kế tiếp

- **`/implement`** hoặc **`boost` (DeepCoder)**: Thực thi hoàn tất Ticket 10 theo đúng quy chuẩn TDD.
- **`code-review`**: Rà soát lại toàn diện các diff sau khi hoàn thành Ticket 10 đối soát theo `CODING_STANDARDS.md` và `spec.md`.
