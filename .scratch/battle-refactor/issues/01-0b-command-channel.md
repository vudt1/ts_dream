# 01: Kênh lệnh trận thật 0x0B (thay thế 0x32 chết)

> **Cập nhật 2026-10-16 (bằng chứng ASM, xem `../HANDOFF-client-reverse.md`):** byte confirm `0B 01` = **3** (đã chốt, G3); ngữ nghĩa sub 8 / sub 0A = **thao tác map/PK ngoài trận** (không phải lệnh trận lưới).

**What to build:** Client `aLogin.exe` không bao giờ gửi opcode `0x32` — kênh lệnh trận thật là `0x0B` (tập sub `{0,1,2,3,4,6,7,8,9,0A,FA}`, chỉ sub 5 là no-op; **sub 2 không có trong switch S→C nhưng có trong builder C→S**). Server phải bỏ đường `0x32` và định tuyến đầy đủ trên `0x0B`: giữ sub 1 (rời trận, byte xác nhận = **3**), sub 2 (kind 2/3/4/5 như hiện tại, nhưng parse trọn 9 byte `[kind][target u32 LE][param u16 LE]` thay vì cắt ngắn), sub 6 (2 byte, không body); thêm parse + log có cấu trúc cho sub 7 (`[mode][counter]`), sub 8 (`[kind][u32 LE][u16 LE]` — 8 byte), sub 9 (rỗng) và sub 0A (`[kind][u32 LE][u16 LE]`). Hai hành vi sai phải bị xoá: sub 8 không còn bị ép thành PK trực tiếp, sub 5 không còn là nhánh flee của client. Item dùng trong trận chỉ được trừ khỏi túi SAU khi lệnh được engine chấp nhận; nếu bị từ chối, item còn nguyên.

**Bảng kind của sub 8 / sub 0A — ĐÃ CHỐT (HANDOFF §5):** đây là **thao tác click trên bản đồ/PK**, không phải lệnh trận. Guard đều nằm phía client (team `+0x5F0`, khoảng cách `< 0x97`, loại mục tiêu `FUN_004c9bf0 ∉ 5..8`, hộp thoại `player+0x380`):

| kind | hàm build | nguồn target | ngữ nghĩa |
|---:|---|---|---|
| `09` | `0064ea20`, `0064efa0:222` | `gvar_007D9D34+0x7B` (click) | tấn công người chơi (khác đội; nếu mục tiêu mang cờ `+0x35F==2` thì target = `+0x548`) |
| `0x0A` | `0064ed14` | `gvar_007D9FB4[idx].id` (≤200) | tấn công thú theo index (có check khoảng cách) |
| `0x0B` | `0064edf8` | `gvar_007D9FB4[idx].id` | thao tác thú theo index (không check khoảng cách) |
| `0x0C` | `0064eecc` | `gvar_007D9FB4[idx].id` | thao tác thú theo index |
| `0x0E` | `0064efa0:141` | `gvar_007DA300[idx].id` (≤800) | thách đấu / mời **cùng đội** (`FUN_00509cac`) |
| `12` | `0064f60c` | `gvar_007DA10C[idx].id` (≤10) | thao tác nhóm thứ 3 (cùng bảng mà `0B 04` type `0x10` ghi) |
| `13` | `0064f6e4` | `gvar_007DA10C[idx].id` | như trên |
| `15` | `0064f7bc` (**sub `0x0A`**) | `gvar_007DA300[idx].id` | cướp cờ / tương tác đặc biệt (`player+0x1458 == 2`) |

⇒ **Hành vi cần implement trong ticket này**: parse đúng + log có cấu trúc (sub, kind, target, index); **không** tự sinh trận từ kind nào; nếu muốn hỗ trợ PK/cướp cờ thì mở ticket riêng dựa trên kind `09` và `15`.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Dispatch không còn nhánh `0x32`; logic skill/item cũ được giữ lại phía sau kênh `0x0B` (không mất khả năng đang có)
- [x] Sub 2 parse trọn 9 byte `[kind][u32 LE][u16 LE]` và chuyển tiếp đúng các kind đã biết (2/3/4/5); kind lạ được log, không nuốt frame
- [x] Sub 7/8/9/0A parse đúng khung byte: `08` = `[kind][u32 LE][u16 LE]` (8 byte), `0A` = `[kind][u32 LE][u16 LE]`; log gồm sub, kind, target, index; frame ngắn không panic
- [x] Sub 1 nhận confirm `3` (contract đã xác nhận bằng call-site `00590ba8:160`) — log giá trị confirm khác 3, không xử lý như rời trận
- [x] Sub 8 không còn kích hoạt PK trực tiếp; sub 5 không còn nhánh flee; **không** mở trận tự động theo kind
- [x] Item dùng trong trận: bị từ chối ⇒ còn nguyên trong túi; được chấp nhận ⇒ trừ đúng 1
- [x] Test mới trong `tests/` (ví dụ `tests/battle_command_channel_test.rs`) cho từng sub + hai ca item

## Ghi chú triển khai (2026-10-16)

- **Nơi giữ logic skill/item cũ**: 0x32 sub 1 (skill) / sub 2 (item) được tái bố trí vào `0x0B`
  sub **3** (skill) và sub **4** (item) — hai sub "action" còn lại của client mà Rust chưa xử lý.
  Client native của sub 3/4 là `u32+u16` (ActionHolder), ngắn hơn layout 6 byte cũ ⇒ frame thật
  của client bị gate độ dài bỏ qua an toàn (không panic, không trừ item). Việc hợp nhất mô hình
  lệnh lưới (row/col) với ActionHolder (`actorId`/`param`) là refactor engine riêng, ngoài scope.
- **Đuôi sub 8 vs 0A**: HANDOFF §1.2 ghi cả hai là `kind+u32+u16`, nhưng `02_battle_positions.md
  §0.1.5` chứng minh sub 8 chỉ 1 byte đuôi (`@LStrFromChar`) và `03_skill_and_combo.md` ghi sub 0A
  chỉ có `kind+u32`. Bộ parse `parse_action_body` đọc đuôi theo độ dài thực (u16 nếu ≥7, 1 byte nếu
  =6, 0 nếu thiếu) nên log luôn đúng và không panic trên mọi biến thể.
- **"Không mở trận tự động theo kind"** được áp cho các kind của sub 8/0A (thao tác map/PK). Sub 2
  kinds 2/3/4/5 vẫn giữ hành vi cũ (mở PK / đánh NPC / join / jam) theo đúng "như hiện tại".
