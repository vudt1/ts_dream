# HANDOFF: Phân Hệ Sự Kiện Đặc Biệt (Special Events) & Quân Đoàn (Guild)

* **Dự án:** `ts_dream` (Rust Game Server parity với Client TS Online `aLogin.exe`)
* **Thời điểm bàn giao:** 2026-11-04
* **Đích đến:** `/tmp/handoff-special-events-implementation.md` (Hệ thống tập tin tạm của OS)
* **Trạng thái công việc:** Đã hoàn thành 100% rà soát, nghiên cứu dịch ngược, domain modeling, đặc tả PRD và tạo 8 tickets thực thi. **Sẵn sàng để agent tiếp theo bắt tay vào code Ticket 01 ngay lập tức.**

---

## 1. Suggested Skills Cho Agent Tiếp Theo

Agent tiếp nhận phiên làm việc nên kích hoạt và áp dụng các skills sau:

1. **`tdd`**: Áp dụng quy trình Test-Driven Development (Red-Green-Refactor) cho từng ticket. Mọi test phải đặt trong `tests/<feature>_test.rs` và sử dụng public API của `ts_dream`.
2. **`code-review`**: Rà soát diff sau mỗi ticket để đảm bảo không vi phạm các guardrails kiến trúc (không gọi `insert` vào `lock_online_sessions()`, không vi phạm Two-Phase State Lock, không đặt `#[cfg(test)]` trong `src/`).
3. **`diagnosing-bugs`**: Kích hoạt khi gặp lỗi parsing byte frame hoặc lệch wire opcode giữa client và server.

---

## 2. Tóm Tắt Ngữ Cảnh & Những Gì Đã Hoàn Thành

Trong phiên làm việc này, toàn bộ phân hệ **Special Events** đã được rà soát và chuẩn hóa:

1. **Phân tích đối chiếu & Nghiên cứu gốc (Research & Gap Analysis)**:
   - Đã đối chiếu chéo giữa thực thi `aLogin.exe` (Delphi 7, PE x86), mã dịch ngược Ghidra/C (`client_pseudo_c/`), dữ liệu tĩnh (`Data/`) và mã nguồn Rust (`src/`).
   - Đã phát hiện và sửa sai hiểu lầm cốt lõi: Vấn đáp Bắc Tinh Quân **không phải anti-bot thông thường**, mà là một **trận đấu dã ngoại đặc biệt (Encounter Battle)** kích hoạt ngẫu nhiên khi người chơi đi lại ngoài bản đồ (gặp NPC 1643/1644).
   - Đã giải mã toàn bộ cơ chế **Thần Tài (`FGod` x1.5 EXP)** và **Con Ma (`FGhost` phạt 0.5x EXP, HP=1)**, bao gồm gói đồng bộ thời gian thực `S→C 18 08` (phát âm thanh `WA0006.wav` & toast *"Thần xui đã rời xa bạn!"* khi giải ma qua NPC 1752 Bà Gọi Hồn 500 vàng hoặc Item 46180 "Cải vận phù").
   - Đã phát hiện lỗi `src/server/spawn.rs` hardcode `ghost=0, god=0` trong frame xuất hiện nhân vật `0x03` (offsets byte 5 & 6).
2. **Cập nhật Bounded Context & Từ vựng miền**:
   - Đã cập nhật đầy đủ các định nghĩa và ràng buộc bất biến (Invariants) vào [`GLOSSARY.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L449-L490).
   - Đã đồng bộ tài liệu kiến trúc kỹ thuật [`concepts_brief.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/concepts_brief.md).
   - Đã lưu trữ toàn văn kết quả nghiên cứu dịch ngược tại [`RESEARCH_AND_GAP_ANALYSIS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/RESEARCH_AND_GAP_ANALYSIS.md).
3. **Đặc tả yêu cầu (Specification - 47 User Stories)**:
   - Đã ban hành đặc tả đầy đủ tại [`.scratch/special-event-refactor/spec.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/spec.md) (Status: `ready-for-agent`).
4. **Phân rã Tickets thực thi (Tracer-Bullet Vertical Slices)**:
   - Đã được người dùng phê duyệt toàn bộ cấu trúc 8 tickets.
   - Toàn bộ 8 tệp ticket đã được xuất bản vào [`.scratch/special-event-refactor/issues/`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/) (từ `01-*.md` đến `08-*.md`).

---

## 3. Bản Đồ Frontier & Hướng Dẫn Thực Thi Từng Bước

Agent tiếp theo sẽ bắt đầu làm việc tại **Frontier** (Ticket không bị chặn bởi bất kỳ ticket nào):

### 🎯 Điểm Bắt Đầu: Ticket 01
* **Tệp ticket:** [`.scratch/special-event-refactor/issues/01-god-ghost-state-and-sync.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/01-god-ghost-state-and-sync.md)
* **Nhiệm vụ chính:**
  1. **Schema & Model**: Thêm cột `ghost INTEGER NOT NULL DEFAULT 0` và đảm bảo `god INTEGER NOT NULL DEFAULT 0` trong bảng `characters` (`migrations/0001_init.sql` và queries trong `src/db/`).
  2. **Session**: Thêm `pub ghost: u8` vào `src/server/session.rs`, hydrate khi nạp character vào session.
  3. **Map Appearance Codec**: Sửa `src/server/spawn.rs:567-568` để `player_appear` truyền đúng `s.ghost` (offset 5) và `(s.god > 0) as u8` (offset 6).
  4. **Real-time Synchronizer**: Xây dựng helper phát frame `S→C 18 08 [playerId: u32 LE][type: u16 LE][value: u8]` (type 1 = Ghost, type 2 = God).
  5. **Exorcism Handlers**:
     - Trong `src/server/handlers/use_item/books.rs`: thêm logic cho Item 46180 ("Cải vận phù") xóa Con Ma, phát gói `18 08` (type 1, value 0).
     - Trong kịch bản đối thoại NPC 1752 ("Bà Gọi Hồn"): kiểm tra `session.ghost == 1` và `session.gold >= 500`, trừ 500 vàng, gán `ghost = 0` và phát gói `18 08`.
  6. **Combat Formula**: Trong `src/battle/combat_formula.rs`:
     - Nếu `session.god > 0`: EXP = `(exp * 1.5)`, giảm `god` đi 1. Nếu `god == 0` phát `18 08` tắt Thần Tài.
     - Nếu `session.ghost == 1`: EXP = `(exp * 0.5)` (giảm 50% kinh nghiệm).
  7. **Integration Test**: Viết test trong `tests/festival_quiz_test.rs` kiểm tra toàn bộ luồng trên bằng lệnh:
     `cargo test --test festival_quiz_test`

### 🔄 Các Bước Tiếp Theo Sau Khi Xong Ticket 01:
* **Ticket 02**: [`.scratch/special-event-refactor/issues/02-north-star-quiz-battle-encounter.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/02-north-star-quiz-battle-encounter.md) (Tạo file JSON `Data/north_star_questions.json`, hook vào `try_trigger_random_encounter`, chặn quan chiến `21 01 03`, wire Q&A modal 45s `10 01/02`, cô lập Party Leader).
* **Ticket 03**: [`.scratch/special-event-refactor/issues/03-web-admin-hierarchical-event-control.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/03-web-admin-hierarchical-event-control.md) (Gắn `EventLifecycleManager` vào `ServerControl`, chặn 409 khi server stopped, cascading emergency stop, HTMX card điều khiển sự kiện trên port 8090).
* **Song song sau Ticket 01**:
  * **Ticket 04**: [`.scratch/special-event-refactor/issues/04-continuous-dungeon-40-npc-wave-chaining.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/04-continuous-dungeon-40-npc-wave-chaining.md) (In-Engine Wave Chaining 40 đợt liên tục trong `BattleRunner`).
  * **Ticket 05**: [`.scratch/special-event-refactor/issues/05-continuous-dungeon-2k-and-nine-heavens.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/05-continuous-dungeon-2k-and-nine-heavens.md) (Leo tháp 2K 18 tầng & Cửu Thiên 9 tầng).
* **Sau Ticket 03**:
  * **Ticket 06**: [`.scratch/special-event-refactor/issues/06-competitive-arena-and-timed-rewards.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/06-competitive-arena-and-timed-rewards.md) (Lôi Đài, bảng điểm 7-byte và thu hồi nhẫn 7 ngày `17 6F`).
  * **Ticket 07**: [`.scratch/special-event-refactor/issues/07-guild-warfare-siege-water-boat-race.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/07-guild-warfare-siege-water-boat-race.md) (Công Thành Chiến, Thủy Chiến túi thuyền 25 slot, Đua Thuyền Rồng).
  * **Ticket 08**: [`.scratch/special-event-refactor/issues/08-seasonal-festivals-and-static-dat-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/08-seasonal-festivals-and-static-dat-parity.md) (`Festival.Dat` date parity `19000110` và thông báo `17 39`).

---

## 4. Tài Liệu Tham Chiếu Trọng Tâm (Primary References)

1. **Đặc tả nghiệp vụ chuẩn (PRD)**: [`.scratch/special-event-refactor/spec.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/spec.md)
2. **Danh sách Issues**: [`.scratch/special-event-refactor/issues/`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/issues/)
3. **Báo cáo nghiên cứu chi tiết**: [`.scratch/special-event-refactor/RESEARCH_AND_GAP_ANALYSIS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/RESEARCH_AND_GAP_ANALYSIS.md)
4. **Từ điển nghiệp vụ dự án**: [`GLOSSARY.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
5. **Quy chuẩn lập trình & Guardrails**: [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)
6. **Mã nguồn giải mã Client Delphi quan trọng**:
   - `client_pseudo_c/case_016_0078E593_FUN_0078e593.c`: Dispatcher nhận câu hỏi modal `0x10 01/02`.
   - `client_pseudo_c/case_021_00790ED5_FUN_00790ed5.c`: Xử lý gói tin `0x18 08`.
   - `client_pseudo_c/00729a88_FUN_00729a88.c`: Logic gắn/gỡ Thần Tài/Con Ma, phát âm thanh `WA0006.wav` & toast *"Thần xui đã rời xa bạn!"*.
   - `client_pseudo_c/case_004_0078BC95_FUN_0078bc95.c`: Layout byte xuất hiện nhân vật `0x03` (byte 5 = ghost, byte 6 = god).

---
*Tệp bàn giao đã được lưu tại `/tmp/handoff-special-events-implementation.md`.*
