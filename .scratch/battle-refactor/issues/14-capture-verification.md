# 14: Capture aLogin.exe — chỉ còn các gap runtime

> **Cập nhật 2026-10-16:** 7/11 gap đã đóng tĩnh bằng ASM (xem `../HANDOFF-client-reverse.md` §10). **G1, G2, G3, G4, G6, G7, G8, G9 không cần capture nữa** — danh sách bên dưới là phần còn lại.

**Đã đóng tĩnh (không cần capture):**

| Gap | Kết luận |
|---|---|
| G1 | kind self `0B FA` phải = **`04`** (xem ticket 15) |
| G2 | bảng nhảy builder `0x780440` đã có trong export Round-4 |
| G3 | confirm `0B 01` = **3**; S→C `0B 01` cần ≥ 4 byte, pad `00` vô hại |
| G4 | `17 06` Rust **đã đúng** byte-exact, `LEN = 0x0E` |
| G5 | kho = **`1E 01`** (container `+0xA48`, 51 ô) — Rust đúng; `17 1E` là stall homdo |
| G6 | `k = {?, 3.0, 3.05, 3.0}` (reborn 0 đọc từ RAM `table+0x84`) |
| G7 | client chỉ có **một** cột dùng chung `table+0xAC`, **không** có cột reborn 4 |
| G8 | `05 03` Rust **đã đúng**: `hp_max` ở `content[0x20]`, `sp_max` ở `0x22`, header 111 byte, skills từ `0x71` |
| G9 | layout `0F 08` đầy đủ, stride `nameLen + 0x63` (xem ticket 07) |
| G10 | **không có frame kết quả trận**: `0x33` chỉ sub 1, `0x35` chỉ sub 1..4, `0x0B` không sub mang (thắng/thua/lượt/điểm) |
| G11 | `LAB_00644A18` = callback trễ: `battleState = 0` + gửi C→S **`13 02`** khi thú chết (Rust đã có handler) |

**What to build (phần còn lại — đều là *xác nhận*, không còn là *tìm ra*):** Chạy client thật, bắt traffic (proxy/hook console), ghi hex + mô tả vào `.scratch/battle-refactor/capture-notes.md`:

1. **Kind của record không phải self**: giá trị `03`/`05`/`64` server đặt cho người khác trong `0B FA`/`0B 05`, và value trong `0B 04` (show-on-map hiện gửi `03`/`05`/`02`) — để biết client có cần `04` (giá trị "đang trong trận", gate chặn tấn công tại `0064efa0:56`) cho người khác không.
2. **Chuỗi frame sau khi trận kết thúc**: bắt toàn bộ frame trong ~1 giây sau `0B 00`; xác nhận không có frame nào mang (thắng/thua, lượt, thời gian, điểm) và xác nhận client có gửi `13 02` khi thú chết.
3. **Số ô thực tế** dùng trong container `+0xA48` (client nhận `slot ≤ 50`; Rust dùng `1..=50`).
4. **`05 04` / `14 08`** (frame thoát trận): xác nhận thứ tự gửi và phản ứng client (UI có đóng đúng không).

**Blocked by:**
- 15 (đổi marker self sang `04`) — để log parse khớp trạng thái client
- 01: Kênh lệnh trận thật 0x0B (thay thế 0x32 chết) — cần log parse để đối chiếu

**Status:** ready-for-human

- [ ] `capture-notes.md` có hex thật cho từng mục 1..4, kèm frame server tương ứng và kết luận khớp/lệch
- [ ] Mỗi mục lệch ghi rõ frame/module cần sửa → mở ticket follow-up
- [ ] Phủ G10 ở mức A: danh sách frame nhận sau khi trận kết thúc + kết luận "không có frame kết quả"