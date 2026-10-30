# HANDOFF: Hoàn Thành Phân Hệ Hợp Thành & Cường Hóa Khảm Lông (Compound & Feather Socketing System)

- **Ngày bàn giao:** 2026-10-30
- **Tác giả:** Lead Game Engine Protocol / Agentic AI Pair Programmer
- **Thư mục công việc:** `.scratch/compound-refactor/`
- **Trạng thái:** Hoàn tất 100% (5/5 tickets completed, review findings fully resolved, 21/21 unit/integration tests passed)

---

## 1. Tóm Tắt Công Việc Đã Thực Hiện (Executive Summary)

Quy trình kỹ thuật đã hoàn thành toàn diện từ nghiên cứu dịch ngược, thẩm định kiến trúc, lập đặc tả PRD/Spec, triển khai 5 tickets ban đầu, tiến hành rà soát 2 trục qua `/code-review` (Standards & Spec), và thực thi khắc phục triệt để toàn bộ các phát hiện theo kế hoạch [`plan-fix-review-findings.md`](plan-fix-review-findings.md):

1. **Nghiên cứu & Đối chiếu Mã máy Delphi (`aLogin.exe`)**:
   - Xác định thẩm quyền 100% thuộc về Game Server (Server Authoritative). Form `0x0079EE08` trong client chỉ là offline simulator (zero-network calls); gameplay trực tuyến sử dụng `form_compound` gửi Opcode `0x17 Sub 0x39`.
   - Mổ xẻ mã máy `TFT_SelectFeatherForm` (VMT `0x0058A3EC`, ctor `0x005B0BFC`), xác định gói tin chọn Lông vũ 4 hệ (Địa, Thủy, Hỏa, Phong) gửi qua Opcode `0x23 Sub 0x03`.
   - Phát hiện và sửa lỗi nghiêm trọng trong `Item.dat`: `chunk[46]` là `compound_material` (1..37), `chunk[47]` là `compound_rank` (1..10+), trước đây bị bỏ qua làm tê liệt thuật toán hợp thành.
   - Sửa lỗi nạp `Compound.Dat`: bỏ qua 8 byte header metadata của Slot 0, nạp đủ 1.393 bản ghi và lập chỉ mục tra cứu nhị phân $O(\log N)$ theo `u32 key`.
2. **Quy chuẩn Kiến trúc & Từ vựng Miền**:
   - Ban hành [ADR 0007](docs/adr/0007-compound-wire-parity-and-crafting-engine.md) ghi nhận quyết định kiến trúc: Wire parity, Two-Phase State Lock, Banker's Rounding, và cơ chế Fallback cấp 1 khi trượt công thức ($R_{\text{target}} = 1$, không bao giờ mất trắng đồ).
   - Cập nhật [GLOSSARY.md](GLOSSARY.md) bổ sung 5 thuật ngữ miền chuẩn hóa.
3. **Triển khai Toàn bộ 5 Tickets Ban đầu (100% Completed)**:
   - **Ticket 01**: Nạp & lập chỉ mục `CompoundCatalog`, sửa `ItemDef`.
   - **Ticket 02**: Động cơ `src/crafting/basic/` (`algorithm.rs`, `validator.rs`).
   - **Ticket 03**: Giao thức mạng `0x17 Sub 0x39`, Two-Phase State Lock & SQLite write transaction.
   - **Ticket 04**: Giao thức mạng `0x23 Sub 0x03` (`TFT_SelectFeatherForm`) cấp phát Lông vũ vào `homdo`.
   - **Ticket 05**: Động cơ cường hóa `src/crafting/socketing/feather.rs` khảm Lông vũ (+1..+10), tính toán tỷ lệ, tụt cấp khi thất bại, khảm đè đổi hệ nguyên tố.
4. **Khắc Phục Toàn Diện Các Vấn Đề Sau Code-Review (Review Fixes Completed)**:
   - **Standards (Hard Violations)**:
     - Chuyển `registry_update_if_online` thành hàm `pub` tại `src/server/session.rs`, thay thế toàn bộ các lệnh gọi trực tiếp `lock_online_sessions().insert(...)` trong `crafting.rs` (§1.2).
     - Khử triệt để silent return: phát thông báo toast lỗi qua `sys_msg_frame` khi payload rỗng, chọn trùng slot trang bị & lông, hoặc chọn sai mã hệ bảo vật (§2.2).
     - Bổ sung kiểm thử chống ăn lan ô phụ cận: pre-populate slot 3, 4, 5 với số lượng lẻ (7, 13, 21) và khẳng định các ô này không suy suyển cả trong RAM lẫn DB sau khi thao tác đồ (§3.2).
     - Xóa 7 trường thừa legacy aliases trong `CompoundDef` và alias `material` trong `ItemDef` (§4).
   - **Standards (Code Smells)**:
     - Trích xuất helper `upsert_inventory_slot` trong `src/server/inventory.rs` khử trùng lặp 23 bind parameters SQL.
     - Trích xuất helper `item_count_changed_frame` trong `src/server/inventory.rs` thay thế các chuỗi literal hex `F44404001709...`.
     - Tái sử dụng `CompoundDatLoader::compute_key_and_sort` trong `algorithm.rs`.
   - **Spec (Wire Parity & Sửa Lỗi Nghiệp Vụ)**:
     - Bóc tách wire payload `0x23 Sub 0x03` hỗ trợ cả 2 định dạng (chuỗi tiền tố Delphi và byte ngắn).
     - Hỗ trợ xếp nguyên liệu vào các Box tùy ý (1&2, 1&3, 2&3) và sửa lỗi che khuất US 5: trả về đúng toast *"Hợp thành cần ít nhất 2 vật phẩm!"* khi đưa 1 vật phẩm.
     - Ràng buộc khảm Lông trên đồ trắng và đổi hệ: chỉ chấp nhận Lông nguyên tố Cấp 1 (`feather.element in 1..=4`, `feather.level == 1`), chặn dùng Lông cấp cao C2..C10 và Lông vô hệ gây mất đồ oan (US 11).
   - **Kiểm thử tự động mở rộng**: Bổ sung 6 integration tests mới, nâng tổng số test lên **21/21 passed** tại `tests/compound_test.rs` và 100% test suites toàn dự án passed.

---

## 2. Danh Mục Tài Liệu & Artifacts Tham Chiếu

| Tài liệu / Cấu phần | Đường dẫn | Mô tả |
|---|---|---|
| **Kế hoạch & Báo cáo Nghiên cứu** | [`.scratch/compound-refactor/COMPOUND_RESEARCH_AND_ARCHITECTURE_PLAN.md`](COMPOUND_RESEARCH_AND_ARCHITECTURE_PLAN.md) | Phân tích chi tiết mã máy Delphi, đối chiếu wire protocol và thuật toán `FUN_0079A890`. |
| **Bản Đặc tả Kỹ thuật (Spec)** | [`.scratch/compound-refactor/spec.md`](spec.md) | Spec chuẩn PRD với 20 User Stories, Implementation Decisions, Testing Decisions. |
| **Kế hoạch Sửa Lỗi Review** | [`.scratch/compound-refactor/plan-fix-review-findings.md`](plan-fix-review-findings.md) | Kế hoạch chi tiết khắc phục toàn bộ vi phạm Standards & Spec sau phiên rà soát code. |
| **Quyết định Kiến trúc (ADR)** | [`docs/adr/0007-compound-wire-parity-and-crafting-engine.md`](../../docs/adr/0007-compound-wire-parity-and-crafting-engine.md) | ADR 0007 xác lập bất biến cho Compound & Socketing. |
| **Từ điển Miền (Ubiquitous Language)**| [`GLOSSARY.md`](../../GLOSSARY.md) | Thuật ngữ chuẩn hóa: `Compound`, `compound_material`, `compound_rank`, `Socketing`, `Scripted Crafting`. |
| **Danh sách 5 Tickets** | [`.scratch/compound-refactor/issues/`](issues/) | 5 tệp ticket từ `01` đến `05`, đều ở trạng thái `completed`. |
| **Mã nguồn Tầng Dữ liệu** | [`src/data/loaders/item.rs`](../../src/data/loaders/item.rs), [`src/data/loaders/compound.rs`](../../src/data/loaders/compound.rs) | Nạp `compound_material`, `compound_rank` và `CompoundCatalog`, sạch alias cũ. |
| **Mã nguồn Động cơ Miền** | [`src/crafting/`](../../src/crafting/) | `basic/` (thuật toán hợp thành), `socketing/` (khảm lông +1..+10), `error.rs`. |
| **Mã nguồn Trình xử lý Mạng** | [`src/server/handlers/crafting.rs`](../../src/server/handlers/crafting.rs) | Điều phối Opcode `0x17 Sub 0x39` và `0x23 Sub 0x03` với Two-Phase State Lock và Registry sync. |
| **Mã nguồn Quản lý Session & Túi đồ** | [`src/server/session.rs`](../../src/server/session.rs), [`src/server/inventory.rs`](../../src/server/inventory.rs) | Helper `registry_update_if_online`, `upsert_inventory_slot`, `item_count_changed_frame`. |
| **Bộ Kiểm thử Tập trung** | [`tests/compound_test.rs`](../../tests/compound_test.rs) | 21 integration tests kiểm tra pure logic, DB transaction, wire frames, collateral integrity và edge cases. |

---

## 3. Trạng Thái Hiện Tại & Lưu Ý Cho Agent Kế Tiếp

1. **Trạng thái mã nguồn**:
   - `cargo check --tests`: Sạch sẽ 100%, 0 warnings, 0 errors.
   - `cargo test --test compound_test`: **21/21 passed** (thời gian chạy ~6.96s).
   - `cargo test --all-targets --no-fail-fast`: **100% tất cả test suites toàn codebase đều passed**.
2. **Tuân thủ quy chuẩn dự án (`AGENTS.md`)**:
   - **Chưa commit Git**: Agent tuyệt đối không tự ý chạy `git commit`. Người dùng sẽ tự review diff và commit thủ công.
   - **Vị trí test**: Không có bất kỳ thẻ `#[cfg(test)]` inline nào trong `src/`. Tất cả test tập trung tại `tests/compound_test.rs`.
   - **Database**: Sử dụng in-memory SQLite (`sqlite::memory:?cache=shared`) trong test, không chạm vào DB sản xuất.
3. **Các tính năng mở rộng tiềm năng trong tương lai (Next Steps nếu có yêu cầu)**:
   - Bùa hộ mệnh chống tụt cấp khi khảm Lông thất bại (`proofkind` trong `inventories`).
   - Đục lỗ trang bị và khảm ngọc phụ (`stoneattr`, `stonelevel` trong `inventories`).
   - Ma trận tương khắc trang bị phụ trợ $15 \times 15$ trong `Data/Compound2.Dat`.

---

## 4. Suggested Skills (Kỹ Năng Đề Xuất Cho Agent Kế Tiếp)

Nếu agent kế tiếp tiếp tục làm việc trên repo này, hãy gọi công cụ `Skill` cho các kỹ năng sau tùy theo nhiệm vụ:
- **`pr`**: Khi người dùng đã sẵn sàng commit và yêu cầu tạo pull request body tóm tắt toàn bộ tính năng Hợp thành & Khảm Lông.
- **`test-audit`**: Nếu người dùng muốn audit chuyên sâu độ bao phủ và chất lượng của 21 test cases trong `tests/compound_test.rs`.
- **`domain-modeling`**: Nếu có thêm các khái niệm nghiệp vụ mới cần cập nhật vào `GLOSSARY.md` hoặc cần ban hành thêm ADR mới cho các phân hệ tiếp theo.
- **`diagnosing-bugs`**: Nếu người dùng chạy thử nghiệm thực tế với client game Delphi và báo cáo có packet không mong muốn.
