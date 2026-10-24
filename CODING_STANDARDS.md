# CODING_STANDARDS.md

Tài liệu quy chuẩn lập trình và tiêu chuẩn thiết kế kiến trúc dành cho các **Reviewer Agents** (`/code-review`) và **Implementation Agents** khi làm việc trên codebase `ts_dream`.

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
- Mọi bài test tương tác cơ sở dữ liệu phải chạy trên In-memory SQLite:
  `sqlite::memory:?cache=shared`
- Tuyệt đối không chạy test ghi đè lên file production `DB/ts_dream.db`.

---

## 4. Code Health & Smell Baseline (Tiêu chuẩn Sạch mã nguồn)

- **Tránh Magic Number / Primitive Obsession**:
  Sử dụng các hằng số định danh miền (domain constants) đã được khai báo trong `src/protocol/mod.rs` (ví dụ: dùng `BANK_SUB_WITHDRAW` thay vì literal số `1`).
- **Khử Duplicated Code**:
  Các logic định dạng frame dùng chung (như format 11-byte item không slot `storage_entry_no_slot`, frame mở khóa nút bấm UI ngân hàng) phải được trích xuất thành helper dùng chung.
