# CODING_STANDARDS.md

Tài liệu quy chuẩn lập trình và tiêu chuẩn thiết kế kiến trúc dành cho các **Reviewer Agents** (`/code-review`) và **Implementation Agents** khi làm việc trên codebase `ts_dream`.

> [!IMPORTANT]
> **Định vị Phạm vi Tiêu chuẩn (Scope of Standards)**:
> - `CODING_STANDARDS.md` dành trọn vẹn cho các **Đánh giá Thiết kế & Kiến trúc (Judgement Calls)**: Two-Phase State Lock, tính nguyên tử DB, phân định ranh giới module (deep modules & clear seams), phản hồi lỗi wire parity và tính toàn vẹn trạng thái đa phiên.
> - Các quy tắc cơ học (như cấm `#[cfg(test)]` trong `src/`, cấm gọi trực tiếp `lock_online_sessions().insert(...)` trong handler, dọn dẹp legacy aliases, cấm chuỗi literal hex `F4440400...`) được tự động hóa và chốt chặn xác thực qua kiểm thử cơ học tại [`tests/architecture_guardrails_test.rs`](tests/architecture_guardrails_test.rs) hoặc checklist kiểm thử trong Spec template.

---

## 1. Concurrency & State Lock Standards (Quy chuẩn Đồng thời & Khóa phiên)

### 1.1 Thứ tự Khóa Hai Pha (Two-Phase State Lock Ordering)
Khi thực hiện bất kỳ giao dịch nào tương tác giữa 2 người chơi (P2P Trade `0x19`, Mua sạp cá nhân `17 21`, Lập nhóm `0x14`, Đấu PK `0x0B`):
- **Phase 1: Deterministic RAM Lock**
  - **Bắt buộc** gọi `lock_player_operations([min(id1, id2), max(id1, id2)]).await` **TRƯỚC KHI** truy vấn dữ liệu từ `registry_get(partner_id)`.
  - **Cấm** đọc session của đối tác từ registry trước khi có khóa, nhằm triệt tiêu hoàn toàn race condition và state drift khi nhiều người chơi cùng thao tác lên một mục tiêu.
- **Phase 2: SQLite Write Transaction**
  - Giao dịch cơ sở dữ liệu phải được thực thi độc quyền trên `write_pool` (`max_connections = 1`).
  - Nếu commit DB thành công: Phát các frame xác nhận wire cho cả hai bên.
  - Nếu commit DB thất bại: **Bắt buộc** rollback RAM state về snapshot trước giao dịch (`before_state`).

### 1.2 Đồng bộ Session Registry (`registry_update_if_online`)
- `ctx.conn.session` đại diện cho kết nối cục bộ của kết nối TCP hiện tại.
- Bất kỳ handler nào làm thay đổi trạng thái của `ctx.conn.session` trong luồng tương tác nhiều người chơi (mời trade, thêm/bớt offer, confirm, transfer vật phẩm/tiền) **bắt buộc** phải gọi:
  ```rust
  registry_update_if_online(&ctx.conn.session);
  ```
  để các phiên đối tác, background tasks (AutoSave), và Actor khác luôn đọc được trạng thái mới nhất từ `online_sessions`.

---

## 2. Wire Protocol Parity Standards (Quy chuẩn Định dạng Giao thức)

### 2.1 Kênh Thông báo Hệ thống (System Notice Channel)
- **Kênh Chat Hệ thống `020102`**: Sử dụng duy nhất hàm chuẩn hóa tại [`src/server/spawn.rs::sys_msg_frame(msg)`](src/server/spawn.rs).
- **Tuyệt đối cấm**:
  - Tự định nghĩa các hàm cục bộ trùng lặp (như `red_message`, `red`) tự sinh byte `020102`.
  - Sử dụng opcode `02 0B` làm thông báo chữ đỏ (trong client Delphi, `02 0B` là lệnh mở giao diện tạo Quân Đoàn `TFCreateOrganize`).

### 2.2 Xử lý Lỗi Giao thức (Error Response Parity)
- Khi một yêu cầu từ client bị từ chối hoặc thất bại, server **phải** trả về đúng mã phản hồi gốc (native error frame) của opcode đó:
  - Lỗi Shop NPC: S→C `1B 01 <code>` (`01` thiếu tiền, `02` đầy túi).
  - Lỗi Giao dịch P2P: S→C `19 02 01` (đối phương offline), `19 02 02` (đối phương đang bận).
  - Lỗi Ngân hàng: S→C `1D 03` (lỗi sức chứa) kèm frame mở khóa UI `1D 08` (rút) hoặc `1D 07` (nộp).
- **Cấm** dùng lệnh `return;` im lặng (silent drop) đối với các hành động người chơi chủ động kích hoạt (như mời trade, mua đồ, rút tiền).

### 2.3 Cấu trúc Payload Thao tác Kho (`0x1E`)
- Gói tin C→S thao tác kho/tiền trang luôn gửi theo cặp 2-byte:
  `[slot: u8, qty: u8]`
- **Cấm** phân tích `ctx.payload` như một mảng danh sách slot đơn lẻ (`for slot in payload`), vì byte `qty` sẽ bị hiểu nhầm là slot phụ cận tiếp theo gây lỗi ăn lan (collateral corruption bug).

---

## 3. Testing Guide & Test Integrity Standards (Quy chuẩn Kiểm thử)

### 3.1 Vị trí Kiểm thử Tập trung
- Mọi unit test / integration test mới **bắt buộc** phải đặt trong thư mục `tests/` ở repo root (ví dụ: `tests/economy_wire_test.rs`).
- **Tuyệt đối cấm** viết `#[cfg(test)]` inline trong `src/` (kể cả module test nhỏ).

### 3.2 Kiểm thử Chống Ăn Lan (Collateral Transfer Integrity)
- Mọi bài test kiểm tra thao tác vật phẩm (chuyển kho, rút/nộp tiền trang, mua sạp, trade đồ) **phải**:
  1. Truyền số lượng lẻ (`count < max_stack`).
  2. Tạo sẵn các vật phẩm phụ cận ở slot kế tiếp (ví dụ: thao tác slot 1 với qty 5 thì phải có item ở slot 5).
  3. Khẳng định (assert) slot phụ cận hoàn toàn không bị ảnh hưởng sau giao dịch.

### 3.3 Kiểm thử Luồng Tự nhiên Đa phiên (Natural Multi-Session Flow)
- Trong các bài test kiểm tra giao dịch đa bên (P2P Trade):
  - **Cấm** gọi `sessions.insert(conn.session.clone())` thủ công giữa các bước test để ép state.
  - Toàn bộ việc đồng bộ session phải diễn ra tự nhiên thông qua mã nguồn server (`registry_update_if_online`).

### 3.4 Tách biệt Cơ sở Dữ liệu
- DB test chính thức là file SQLite riêng `DB/ts_dream_test.db` (gitignored), chạy cùng chế độ WAL + dual-pool như production để test phản ánh đúng hành vi persist/checkpoint.
- Test nối DB qua helper `tests/common/test_db.rs` (`mod common;` + `common::test_db::setup_test_db().await`). Helper đọc `TS_TEST_DB_URL` (mặc định `sqlite://<repo>/DB/ts_dream_test.db`), khóa toàn cục để các test DB trong cùng binary chạy tuần tự, xóa sạch schema rồi chạy lại `migrations/0001_init.sql` → mỗi test bắt đầu từ DB trống, độc lập và lặp lại được.
- File production `DB/ts_dream.db` chỉ dành cho server; test luôn trỏ tới `DB/ts_dream_test.db`.

### 3.5 Nguyên Tắc "Every User Story must have a Negative Test"
- Khi viết test cho một phân hệ (`tests/<feature>_test.rs`), **bắt buộc** phải có nhóm test kiểm tra các trường hợp từ chối (Rejection/Validation Error) tương ứng với từng User Story trong `spec.md`.
- Không chỉ kiểm tra trạng thái trả về mà **bắt buộc** phải kiểm tra thông báo lỗi/Toast gửi về cho client qua wire (`sys_msg_frame` hoặc native error frame) không bị sai lệch hay che khuất (tránh bug masked error).

### 3.6 Chu Kỳ Chạy Test & Chốt Chặn Handoff
- Khi phát triển và sửa lỗi, chỉ chạy test tập trung cho tính năng đang làm (`cargo test --test <feature>_test`) để giữ chu kỳ phản hồi nhanh (tight loop).
- **Chỉ chạy lại toàn bộ test suite** (`cargo test --all-targets --no-fail-fast`) ở bước chốt chặn cuối cùng trước khi bàn giao (handoff).

---

## 4. Code Health & Smell Baseline (Tiêu chuẩn Sạch mã nguồn)

- **Tránh Magic Number / Primitive Obsession**:
  Sử dụng các hằng số định danh miền (domain constants) đã được khai báo trong `src/protocol/mod.rs` (ví dụ: dùng `BANK_SUB_WITHDRAW` thay vì literal số `1`). Các chuỗi literal hex (`F4440400...`) phải được thay thế bằng helper (như `item_count_changed_frame`), được chốt chặn bởi `tests/architecture_guardrails_test.rs`.
- **Phân nhánh Sub-Opcode Tường Minh (Explicit Sub-Opcode Branching)**:
  Khi xử lý dispatch hoặc sinh phản hồi theo sub-opcode nhị phân (`u8`), **cấm** sử dụng fallback `else` vô điều kiện. Luôn phân nhánh tường minh (`if sub == CONST_A ... else if sub == CONST_B ... else { /* drop / no-op */ }`) để các gói tin mang sub-opcode lạ hoặc rác không bao giờ kích hoạt nhầm trạng thái mở khóa UI hay đột biến dữ liệu.
- **Khử Duplicated Code & Legacy Aliases**:
  Các logic định dạng frame dùng chung (như format 11-byte item không slot `storage_entry_no_slot`, frame mở khóa nút bấm UI ngân hàng) phải được trích xuất thành helper dùng chung. Khi chuẩn hóa API mới, phải chuyển đổi dứt điểm toàn bộ call-site và loại bỏ alias cũ (được kiểm tra tự động bởi `tests/architecture_guardrails_test.rs`).
