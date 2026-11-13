# Phân tích Binary `aLogin.exe` & Làm rõ Raw Codes `Data/eve.emg`

> **Mục đích**: Đối chiếu trực tiếp client PC [`client_delphi/aLogin.exe`](file:///D:/VUDT/GIT_PCC/ts_dream/client_delphi/aLogin.exe) (từ kết quả disassemble x86, [`client_pseudo_c/`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/), [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs)) và quét nhị phân toàn bộ [`Data/eve.emg`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/eve.emg) để làm rõ các raw code chưa giải mã và các nhánh helper/callback client trong [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md).  
> **Phạm vi**: Nghiên cứu & tài liệu hóa thuần túy (chỉ đọc binary/data, không sửa code Rust/DB). Mọi ngữ nghĩa không có code xử lý trong `aLogin.exe` được gắn nhãn rõ ràng là **server-only**.

---

## 1. Kiến trúc Nạp & Sử dụng `Data/eve.emg` trong `aLogin.exe`

### 1.1. Tổng quan binary và dữ liệu

- **Binary client**: `client_delphi/aLogin.exe` — PE32 x86 (Delphi), kích thước `4,373,504` bytes, SHA-256 `1487059f0508d49e89946856eee4088b0cf53704efafc2d8d57988d148ff06f4`.
- **Container dữ liệu**: `Data/eve.emg` — `9,839,580` bytes, mục lục gồm `3,901` scene `.eve`.
- **Chuỗi hàm nạp khi đổi map**:
  1. `FUN_005e8534` đọc `scene_id` tại `[PlayerState + 0x63a]`, tạo đường dẫn scene và gọi `FUN_005efee8` ([`005efee8_FUN_005efee8.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c)) đọc lần lượt **9 section** tại `entry.position + 103` byte.
  2. Mọi chuỗi tên nhúng cố định 9 byte hoặc 21 byte (`SceneInfo.areaName`, `TEvent.Name`, `TFightEvent.Name`) đều được giải mã qua `FUN_007aa76c`: duyệt xuôi từng byte và trừ 2 (`(b - 2) & 0xff`). Ví dụ toàn bộ `45,383` tên `TFightEvent` trong `eve.emg` sau giải mã đều có chuỗi Big5 `"新事件"`.
  3. Khi thoát map/dọn bộ nhớ, `FUN_005e9960` giải phóng toàn bộ mảng con trỏ trên `PlayerState` (`gvar_007DA5A0`).

### 1.2. Phần nào `aLogin.exe` chỉ nạp (server-only) vs phần nào client đọc tại runtime?

Quét toàn bộ `CODE` section của `aLogin.exe` theo VMT của 12 lớp Eve (`0x7a9a58..0x7aa078`) và theo từng offset mảng trên `PlayerState` (`gvar_007DA5A0`):

| Section trong `.eve` | Offset trên `PlayerState` (`0x7DA5A0`) | Xref đọc/ghi trong `CODE` | Hành vi thực tế của `aLogin.exe` | Phân loại |
|---|---|---|---|---|
| **1. `TRole` (`NpcData`)** | — | Render, tương tác, di chuyển NPC | Khởi tạo & hiển thị NPC tĩnh/tuần tra trên map | Client + Server |
| **2. `TGoods`** | `+0x9ec8` | Chỉ `0x5e85f9`, `0x5e9b57`, `0x5e9b75` (`3` hit) | Chỉ nạp (13B/mục) và giải phóng; **0 reader runtime** | **Server-only** |
| **3. `TDoor`** | — | VMT `0x7aa044` | Quản lý vùng cổng dịch chuyển trên map | Client + Server |
| **4. `TMine`** | — | VMT `0x7a9fec` | Vùng chạm / ngộ địch trên map | Client + Server |
| **5. `TSurface`** | `+0x9d34` | `0x5e85f0`, `0x5e9c9a..d2`, **`0x5ec2f0`** (`5` hit) | Nạp + lọc placeholder `data_id=20393 (0x4fa9), kind=1, style=3`; render dialog ở `FUN_005eb530` khi server gửi **`S→C 0x14 rt=6`** | Server gửi ID, Client render |
| **6. `SceneInfo`** | `+0x4cc` | `0x5e85ba`, `0x5e9d02` (các hit `0x4cc` khác ở `0x74a3fe..beb1` thuộc class khác) | Chỉ nạp (39B/mục) và giải phóng; **0 reader `PlayerState+0x4cc` runtime** | **Server-only** (hoặc công cụ map) |
| **7. `GroupData`** | `+0x1365` | `0x5e85de`, `0x5e9d0f` (`0x420caf` là lệnh `call 0x422018`) | Chỉ nạp và giải phóng; **0 reader runtime** | **Server-only** |
| **8. `TEvent` / `TCondition` / `TResult`** | `+0x1a4` | VMT `0x7a9afc/0x7a9c00/0x7a9d0c` chỉ ở `0x5effb7..0x5f0f03`; cleanup ở `0x5e85e7`, `0x5e9c1e..56`, `0x7aa0c4..0x7aafed` | Nạp đủ từ `eve.emg` nhưng **0 hàm runtime đánh giá `TCondition` hay đọc `TResult` từ `eve.emg`**. Client chỉ chạy `FUN_005eb530` theo bản ghi `15B` do server gửi qua `S→C 0x14 (0x01..0x06)`! | **100% Server-only** (đối với `TCondition` & `eve.emg` `TResult`) |
| **9. `TFight`** | `+0x9a10`, active `+0xa0d0` | `0x5e85d5`, `0x5e9b9b..d7`, `0x5ebfe1` (`rt=3`), `0x5bfe40..0x5c0328` (`[alt+W]`), `0x64cf60` | Dùng đúng 3 việc: (a) giữ slot `TFight[W3]` vào `+0xa0d0` khi `0x14 rt=3`; (b) `FUN_0064cf60` lấy `+0x07 BKMusic` cho `TFightField.Create` (`0x63fceb`); (c) `FUN_005bfe40`/`0x5c01dc` đọc `+0xc0/+0xc4` (`TWinCondition`) cho UI `[alt+W]` | Client dùng 1 phần nhỏ, logic trận ở Server |

> **Kết luận cốt lõi**: Dù `aLogin.exe` chứa bộ parser `FUN_005efee8` đọc đầy đủ 9 section của `eve.emg` — giúp ta xác minh chính xác 100% layout nhị phân từng struct — nhưng **client không bao giờ chạy script `TEvent`/`TCondition`/`TResult` từ `eve.emg`**. Mọi `condition_class`, `condition_ops`, `p_style`, `result_type >= 7`, `fight_type`, `link_to_result`, và điều kiện kích hoạt `TFightEvent` đều được thực thi hoàn toàn phía server.

---

## 2. Xác minh Layout Nhị phân Section 8 & 9 (`FUN_005efee8` ↔ `eve.rs`)

Đối chiếu [`005efee8_FUN_005efee8.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c#L623-L866) với [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L758-L938) khẳng định thứ tự đọc byte khớp **1-1 tuyệt đối**:

### 2.1. `TCondition` (23 byte cố định trước `result_count` — L661–671)

| Thứ tự đĩa | Size | Field trong `eve.rs` | Offset trên object Delphi | Ghi chú |
|---:|---:|---|---|---|
| `0x00` | 1B | `condition_no` | `+0x04` | Thứ tự điều kiện (`1..30`) |
| `0x01` | 1B | `condition_class` | `+0x05` | Mã phân loại điều kiện (`0..17`) |
| `0x02` | 2B | `condition_parameter` | `+0x06` | `u16` tham số (quest ID, item ID, ...) |
| `0x04` | 1B | `condition_parameter_style` | `+0x08` | Kiểu diễn giải tham số |
| `0x05` | 1B | `condition_ops` | `+0x14` | Toán tử so sánh (`0..6`, cá biệt `142`) |
| `0x06` | 4B | `condition_value` | `+0x0c` | `i32` giá trị so sánh |
| `0x0A` | 8B | `CTimeValue` (skip trong `eve.rs`) | `+0x18` | `f64` double (8 byte) |
| `0x12` | 1B | `to_result` | `+0x20` | Nhóm kết quả nhánh đúng |
| `0x13` | 1B | `and_num` | `+0x10` | Liên kết AND với điều kiện kế |
| `0x14` | 2B | `condition_sub_item` | `+0xa2` | `u16` tham số phụ |
| `0x16` | 1B | `result_count` | `+0xa0` | Số lượng `TResult` theo sau |

### 2.2. `TResult` (14 byte cố định/mục — L689–696)

| Thứ tự đĩa | Size | Field trong `eve.rs` | Offset trên object Delphi | Ghi chú |
|---:|---:|---|---|---|
| `0x00` | 2B | `result_group_no` | `+0x04` | Nhóm kết quả |
| `0x02` | 1B | `result_no` | `+0x06` | Số thứ tự trong nhóm (`1..30`) |
| `0x03` | 1B | `result_type` | `+0x07` | Loại kết quả (`0..10`) |
| `0x04` | 1B | `result_class` | `+0x08` | Lớp đối tượng (`0..80`) |
| `0x05` | 2B | `parameter` | `+0x0a` | `u16` tham số |
| `0x07` | 1B | `parameter_style` | `+0x0c` | Kiểu tham số |
| `0x08` | 4B | `result_value` | `+0x10` | `i32` giá trị kết quả |
| `0x0C` | 2B | `result_mean_no` | `+0x14` | `u16` ID thoại/trận/door/surface |

### 2.3. `TFight`, `TFightEvent`, `TWinCondition` (L721–866) & Bất thường `62007.eve`

- `TFight`: `eve_no (u16, +4)`, `fight_type (u8, +6)`, `BKMusic (u8, +7)`, `link (u8, +8)`, `left_enemies_cnt (u16, +0x60)` × `6B TEnemy`, `right_enemies_cnt (u16, +0xb8)` × `6B TEnemy`, `fight_events_cnt (u16, +0x138)` × `TFightEvent`.
- `TFightEvent`: `id (u16, +4)`, `name (9B giải mã -2, +6)`, `win_cond_cnt (u8, +0x8c)` × `13B TWinCondition`.
- `TWinCondition` (13B): `+0x04 (1B no)`, `+0x05 (1B type)`, `+0x06 (2B param u16)`, `+0x08 (1B param_style)`, `+0x0c (4B value u32/i32)`, `+0x10 (1B and_num)`, `+0x14 (2B mean_no u16)`, `+0x16 (1B byte phụ)`.
- **Bất thường EOF ở `62007.eve`**: Trong `3,901` scene của `Data/eve.emg`, `3,900` scene kết thúc khớp chính xác tới byte cuối `pos + size`. Riêng `62007.eve` (`pos=9812735, sz=2993`) kết thúc parse hợp lệ tại offset `9815664` (`2,826` byte sau `pos+103`) và còn thừa **64 byte** rác/dư ở cuối (`00000100...06b975aac8a7f5...` — chứa mảnh event `"新事件"`). `FUN_005efee8` dừng đúng sau `TFight cnt=0` nên không bị ảnh hưởng.

---

## 3. Làm rõ Toàn bộ §2 Raw Codes trong `Data/eve.emg`

Kết quả parse kiểm chứng toàn bộ `Data/eve.emg` (`0` lỗi): `69,404` `TCondition`, `150,098` `TResult`, `22,833` `TFight`, `45,383` `TFightEvent`, `45,773` `TWinCondition`.

### 3.1. `EveCondition` (`condition_class`, `p_style`, `condition_ops=142`)

**Trạng thái phía client (`aLogin.exe`)**: **100% Server-only** (client không đọc `TCondition` sau khi nạp).

**Phân bố toàn bộ 17 mã `condition_class` thực đo:**
- Đã biết: `0` (`16,731`), `1` (`4,292`), `2` (`25,023`), `7` (`7,984`), `8` (`5,780`), `9` (`1,889`), `10` (`5,715`), `12` (`703`), `14` (`5`).
- Chưa map trong enum Rust (khớp đúng số lượng của `UNKNOWN_EVE_CODES_RESEARCH.md`), kèm mẫu thực tế trong `eve.emg`:

| `cc` | Số lần | Phân bố `p_style` / `param` tiêu biểu | Mẫu thực tế `(scene, eve, cond_no, param, p_style, ops, val, sub_item)` |
|---:|---:|---|---|
| `3` | `807` | `p_style`: `4` (`806`), `1` (`1`); `param != 0`: `807/807` | `(10951.eve, eve=33, cond=1, param=28, ps=4, ops=5, val=0, sub=0)` |
| `5` | `181` | Thường `param=0, ps=1` | `(10993.eve, eve=12, cond=2, param=0, ps=1, ops=1, val=100)`; `(11012.eve, eve=16, cond=1, param=0, ps=1, ops=1, val=5)` |
| `13` | `110` | `param=1, ps=1` | `(11031.eve, eve=17, cond=5/10/12, param=1, ps=1, ops=5/6, val=3)` |
| `11` | `77` | `param=0, ps=1, ops=0, val=0` | `(11001.eve, eve=12, cond=4)`, `(11012.eve, eve=13, cond=4)` |
| `4` | `45` | `param=14001/14005...` (giá trị dạng skill/thuộc tính 14xxx), `ps=1` | `(11424.eve, eve=2, cond=3, param=14005, ps=1, ops=5, val=10)` |
| `16` | `42` | `param` là 16-bit ID lớn (`65167`, `23174`), `ps=1, ops=2/5, val=0` | `(12721.eve, eve=15, cond=18, param=65167, ps=1, ops=2)` |
| `17` | `16` | `param=0, ps=1, ops=5, val=0, sub_item=1/2/3...` | `(12721.eve, eve=16, cond=5/8/11, param=0, ps=1, ops=5, val=0, sub=1/2/3)` |
| `15` | `4` | `param=0/1, ps=0, ops=5/6, val=0` | `(55002.eve, eve=4, cond=1/2, param=0, ps=0, ops=6/5)`, `(59102.eve, eve=2, cond=3, param=1, ps=0, ops=6)` |

**Các biến thể `p_style` thực đo (không tồn tại nhánh đọc nào trong `aLogin.exe`):**
- `cc = 2 (QuestStep)`: `p_style` có `9` giá trị: `1: 16315`, `2: 3075`, `3: 4685`, `4: 125`, `5: 109`, `6: 4`, `7: 2`, `9: 320`, `10: 388` (luôn có `param != 0`; ví dụ `ps=9/10` tại `11811.eve eve=23 param=2 ops=0 val=12152` đảo cặp quest-id sang `val`).
- `cc = 9 (FollowPet)`: `p_style ∈ {1: 1546, 2: 259, 3: 84}`, `param != 0` ở `434` bản ghi (ví dụ `ps=2` tại `11560.eve eve=3 cond=6 param=17198 ops=1 val=17198`).
- `cc = 12 (SceneEventCount)`: `p_style ∈ {0: 238, 1: 422, 2: 1, 3: 6, 4: 2, 5: 27, 6: 7}`, `param != 0` ở `250` bản ghi (ví dụ `ps=5` tại `10991.eve eve=4 cond=9 param=0 ops=2 val=14`).
- `condition_ops = 142`: Phân bố `condition_ops` là `(0: 35637, 1: 2929, 2: 6557, 3: 88, 4: 806, 5: 21844, 6: 1542, 142: 1)`. Bản ghi duy nhất `142` (`0x8e`) nằm tại **`55100.eve`, `eve=5, cond_no=14`**: raw 23B `0e 09 0000 01 8e 393c0000 ...` (`cc=9 (FollowPet), param=0, p_style=1, ops=142, val=15417, to_result=12`). Không có bất kỳ xử lý đặc biệt nào trong `aLogin.exe` (xác nhận khả năng cao là lỗi dữ liệu/bit dư trong file `.eve` gốc, tương đương fail-closed hoặc mask operator trên server).

### 3.2. `EveResult` (`result_type`, `result_class`) & Đính chính `rt = 4`

Phải tách bạch 2 lớp:
1. **`FUN_005eb530` (thực thi `S→C 0x14 SubOp 1..6` phía client)** ([`005eb530_FUN_005eb530.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c)): chỉ xử lý `result_type ∈ {0, 1, 2, 3, 4, 5, 6}`.
2. **`Data/eve.emg` (`TResult`)**: chứa `result_type ∈ 0..=10` (`(0: 49728), (1: 61458), (2: 10336), (3: 22815), (4: 210), (5: 240), (6: 2571), (7: 691), (8: 351), (9: 1600), (10: 98)`).

Kết quả làm rõ từng mục của §2.2:

- **`rt = 7` (`691`x), `rt = 8` (`351`x), `rt = 10` (`98`x)**:
  - `aLogin.exe` **không có case `7`, `8`, `10`** trong `FUN_005eb530` và không đọc `TResult` từ `eve.emg` → **100% Server-only**.
  - Tổ hợp `result_class` đi kèm thực đo: `rt=7` → `100% rc=7 (691)`; `rt=8` → `rc=7 (350)` + `rc=80 (1)` (`12901.eve, eve=13, p_style=9, val=0`); `rt=10` → `100% rc=7 (98)`.
- **ĐÍNH CHÍNH QUAN TRỌNG về `rt = 4` (`210` bản ghi)**:
  - Trong `UNKNOWN_EVE_CODES_RESEARCH.md` §2.2 ghi `210` là số bản ghi cần kiểm tra `rc != 3` hoặc ngoài 4 chữ số cuối. Kết quả quét toàn bộ `Data/eve.emg` cho thấy **`210` là TỔNG SỐ bản ghi `rt = 4` toàn game**, và **100% (`210/210`) đều có `rc == 3`** (`0` bản ghi `rc != 3`, `0` bản ghi `< 1000`).
  - Phân bố độ dài chữ số của `result_value` trong `210` bản ghi `rt = 4`: `5` chữ số (`2` bản ghi: `15431.eve eve=4 val=10034` và `19301.eve eve=2 val=70020`), `6` chữ số (`175` bản ghi), `7` chữ số (`33` bản ghi).
  - Trong `FUN_005eb530` (`case 4`, [`L683–L710`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c#L683-L710)), `aLogin.exe` set `PlayerState+0xa096 = 1`, chỉ kiểm tra `param_1[0xa0a0] == 3` (`rc == 3`), chuyển `result_value` sang chuỗi thập phân rồi cắt **4 ký tự cuối** làm tọa độ dịch chuyển, phần đầu làm ID bản đồ/điểm mốc (ngay cả giá trị 5 chữ số `10034` / `70020` vẫn tách hợp lệ thành `"1"+"0034"` và `"7"+"0020"`).
- **Thống kê `result_class` và biến thể `p_style` (toàn bộ `rc` không thuộc UI `0..6` hay không đọc từ `eve.emg` đều là Server-only)**:
  - Các `rc` hiếm thực đo:
    - `rc=0` (`23,118`x): ngoài trường hợp rỗng, còn xuất hiện ở `rt=5` animation (vd `10801.eve eve=7 rt=5 rc=0 p_style=1/2 val=17/18`).
    - `rc=5` (`430`x): gắn `rt=0` (vd `11012.eve eve=16 p_style=2 val=5`, `11031.eve eve=24 p_style=2 val=100`).
    - `rc=6` (`9`x): gắn `rt=1` (vd `12945.eve eve=2 mean=15291`).
    - `rc=10` (`9`x): gắn `rt=0` (`11171.eve eve=2 ps=1 val=50`).
    - `rc=11` (`35`x): gắn `rt=0` (`11572.eve eve=2 ps=1/3/4 val=50/100`).
    - `rc=12` (`21`x): gắn `rt=1` (`12721.eve eve=16 mean=42972/42950`).
    - `rc=9 (34001.eve eve=14)`, `rc=15..22 (12990.eve eve=14..21)` — toàn bộ đều có `rt=1, mean=51003`.
    - `rc=13 (12061.eve eve=29 rt=0 ps=1 val=44)`, `rc=80 (12901.eve eve=13 rt=8 ps=9 val=0)`.
  - `rc = 3`: `p_style ∈ {(0: 51460), (1: 672), (2: 1191), (3: 15714), (4: 13072), (5: 808), (6: 1)}` (`ps=6` tại `12827.eve eve=6 rt=0 param=5 val=0`).
  - `rc = 7`: `p_style` trải từ `0` (`23,073`x) đến `38` (`p_style=1..38`, ví dụ tại `10851.eve eve=1/2` và `10991.eve eve=13` dùng `rt=2` kèm `param >= 2`). Khi server gửi xuống `S→C 0x14`, `FUN_005eb530` không rẽ nhánh theo `parameter_style` của `rc=7`.

### 3.3. `TFight`, `TWinCondition`, `TFightEvent` & `[TEAMDEF] Diahinh`

Trong toàn bộ `aLogin.exe`, bảng `TFight` (`PlayerState+0x9a10`) **chỉ được đọc tại 3 điểm**:

1. **`FUN_005eb530` `result_type == 3`**: chép con trỏ `TFight[W3]` vào `PlayerState+0xa0d0`, set `+0xa094 = 1`, gửi `C→S 0x0B 0x01` (chi tiết §4.1).
2. **`FUN_0064cf60` (`0x64cf60..0x64cfe0`, gọi từ `TFightField.Create` tại `0x63fceb`)**: tra `TFight` theo `eve_no` để lấy **`TFight+0x07` (`BKMusic`)**:
   - `1, 2, 3` ánh xạ tương ứng `Sound\WC0007.wav`, `WC0008.wav`, `WC0010.wav` và ghi `LocalActor+0x1304 = 1/2/3`.
   - Census `eve.emg`: `BKMusic` có `(1: 22198), (2: 464), (3: 166), (4: 2), (34: 3)`.
3. **`FUN_005bfe40` & `FUN_005c01dc` (`0x5bfe40..0x5c0590`) — Giao diện "Điều kiện thắng thua [alt+W]"**:
   - `FUN_005bfe40` lấy `TFight` hiện tại qua chỉ số `[gvar_007DA51C + 0xeb6]`, chép các `TWinCondition` từ `TFight+0xc0` (`TFightEvent[1]`) vào buffer `+0x04..` và từ `TFight+0xc4` (`TFightEvent[2]`) vào `+0x9f..`.
   - `FUN_005c01dc` chỉ render text cho **đúng 3 giá trị `TWinCondition+0x05` (`wc_type`)**:
     - `slot 1` (`TFight+0xc0`) chỉ kiểm tra **`wc_type == 3`**: chép `TWinCondition+0x0c` (`val`, clamped `<= 0xff`) và ghép chuỗi VISCII (`0x5c04d4` / `0x5c050c`): `"Điều kiện mới gia tăng thắng thua"` + `<val>` + `" Điều kiện thắng thua [alt+W]"`.
     - `slot 2` (`TFight+0xc4`) kiểm tra **`wc_type == 4`** (ghép `"Tòan bộ địch bị tiêu diệt"` (`0x5c0534`) + `<TWinCondition+0x0c>` + `" Điều kiện thắng thua [alt+W]"`) và **`wc_type == 8`** (tra NPC ID 16-bit tại `TWinCondition+0x06` qua `FUN_00623f00(gvar_007D9DE4, id)` rồi ghép `"Điều kiện thắng lợi: <NpcName> Điều kiện chiến bại:"` từ `0x5c0558`/`0x5c0578`).
   - Census `TWinCondition+0x05` trong `Data/eve.emg` (`45,773` bản ghi): `(1: 45349), (3: 44), (4: 98), (8: 277), (9: 4), (20: 1)`. Các mã `1` (mặc định không hiển thị `[alt+W]`), `9` (`12722.eve`), `20` (`60301.eve`) không có nhánh xử lý trong `aLogin.exe`.

**Các trường hoàn toàn không được `aLogin.exe` sử dụng (100% Server-only):**
- `TFight+0x06` (`fight_type`, `16` giá trị: `1, 2, 4, 6, 7, 8, 16, 18, 32, 34, 36, 38, 48, 49, 50, 52`), `TFight+0x08` (`link_to_result`: `0` x29, `1` x22804), và logic turn/reinforcement của `TFightEvent`: **0 xref đọc trong `aLogin.exe`**.
- `[TEAMDEF] Diahinh` / `.ini` chiến đấu: tìm chuỗi `"Diahinh"`, `"FightData"`, `"eve.emg"` trong `aLogin.exe` cho `0` kết quả; tham chiếu `.ini` duy nhất trong binary là `\Setup.ini`. Bản đồ nền chiến đấu của client do gói tin server quyết định, không đọc từ file `.ini` của server cũ.

---

## 4. Giải mã Toàn bộ §3 Các nhánh Helper/Callback Client trong `aLogin.exe`

Tất cả 4 mục chưa phục hồi body trong §3 của [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md#L66-L74) đều đã được disassemble và xác minh trực tiếp từ `aLogin.exe`:

### 4.1. `0x14` `FUN_005eb530` tại `result_type == 3`: Gói `C→S 0x0B 0x01`

Disassemble tại `0x5ebfbf..0x5ebfff` và bảng nhảy `SendCommand` (`FUN_0077f414`) tại `0x780420..0x780590`:
1. Tại `0x5ebff6`: `mov cl, 0x1; mov dl, 0xb; call 0x77f414` → truyền **`CL = 0x01`** vào `SendCommand(0x0B, 0x01)`.
2. Bảng nhảy `[eax*4 + 0x780440]` tại `0x780439` ánh xạ `CL = 0x01` sang **`0x780511`** (không phải nhánh lỗi/nhánh khác trong Ghidra trước đó).
3. Đoạn `0x780511..0x780590` đóng gói đúng **3 byte C→S**:
   ```text
   [0x0B, 0x01, byte ptr [gvar_007DA5AC + 0x130]]
   ```
   (trong đó `gvar_007DA5AC` là `TFightForm1`; ví dụ tại `0x590f4d` client ghi `BYTE PTR [TFightForm1 + 0x130] = 0x03`).

### 4.2. `0x14` `result_type == 6` (`TSurface`): 4 Callback nút chọn → `PlayerState+0xa104`

Disassemble `0x5eecbc..0x5eee60` và [`005eb530_FUN_005eb530.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c#L2440-L2494) cho thấy cả 4 callback đều ghi byte kết quả vào `PlayerState+0xa104`, gọi `FUN_0077f414(dl=0x14, cl=0x09)` (`C→S 0x14 09`) rồi xóa cờ `PlayerState+0xa095 = 0`:

| Địa chỉ callback | Ngữ cảnh gắn trên `gvar_007D9D6C` (`L2460–2493`) | Giá trị ghi vào `PlayerState+0xa104` | Hành vi sau gửi `0x14 09` |
|---|---|---|---|
| `0x005eecbc` | Khi không có tùy chọn (`iVar23 == 0`), callback cuối | **`0x14` (`20`)** | Clear `a095 = 0`, `a104 = 0` |
| `0x005eecf4` | Khi không có tùy chọn (`iVar23 == 0`), nút xác nhận/tiếp | **`0x15` (`21`)** | Clear `a095 = 0`, `a104 = 0` |
| `0x005eee28` | Callback hủy/thoát (gắn ở cả nhánh có & không có tùy chọn khi `uType != 2`) | **`0x28` (`40`)** | Clear `a095 = 0`, `a104 = 0` |
| `0x005eed2c(self, edx=opt_idx)` | Khi có danh sách tùy chọn (`iVar23 != 0`), nhận `opt_idx` | Nếu `opt_idx < 10`: ghép chuỗi `'3' + IntToStr(opt_idx)` → **`30..39`**.<br>Nếu `opt_idx >= 10`: ghép `'6' + IntToStr(opt_idx - 9)` → **`61, 62, ...`** (clamped `<= 0xff`). | Clear `a095 = 0` |

### 4.3. Các helper `S→C 0x14` sự kiện phụ (`0x2A`, `0x2C`, `0x32`, `0x33`, `0x38 sel=2`)

1. **`0x14 0x2A` — `FUN_005f16f4`** ([`005f16f4_FUN_005f16f4.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005f16f4_FUN_005f16f4.c)): Đọc `sub = payload[1]`:
   - `sub == 1`: Đọc DWORD 4B `[payload+2..5]`, hiện toast 2000ms `"đểm kinh nghiệm tăng <N> Điểm"` (`0x5f1a34`/`0x5f1a54`) và phát âm thanh `"ExpUP"`.
   - `sub == 2`: Đọc `payload[2]` (`0..5`), hiện toast đăng ký đại tỷ thí / Hinet (`0`: `"đăng kýThành công"`, `1`: `"Trong đội ngũ có người đăng ký tên lại đại tì thí, đăng ký tên thất bại"`, `2`: `"Số người không đủ, sau khi 1 đội 5 người mới làm thủ tục"`, `5`: `"Máy chủ đang bận, xin vui lòng đợi rồi thử lại"`, `3/4/khác`: `"đăng kýThất bại"`).
   - `sub == 3`: Đọc `payload[2]` (`0..5`), hiện toast hủy đăng ký (`0`: `"thủ tiêuThành công"`, `1`: `"trong đội ngũ có người không đăng ký tỉ thí Hinet, thủ tiêu đăng ký thất bại"`, `2`: thông báo thiếu 5 người, `3`: `"Với những thành viên đăng ký tên trước đây không giống nhau, không thể thủ tiêu"`, `4`: máy chủ bận, `khác`: `"thủ tiêuThất bại"`).
2. **`0x14 0x32` — `FUN_007485bc`** ([`007485bc_FUN_007485bc.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/007485bc_FUN_007485bc.c)): `payload[1] ∈ 1..5` hiển thị toast (2000ms) hoặc chat hệ thống (`FUN_007ab870`) sự kiện Tết Đoan Ngọ / giải câu đố Nguyên Tiêu (`1`: kết thúc Đoan Ngọ; `2`: đối đáp bắt đầu; `3`: `"Khiêu gia quan đến quản trường Quận Trác chiệu mộ..."`; `4`: kết thúc Tết Nguyên Tiêu; `5`: hoạt động đã kết thúc).
3. **`0x14 0x33` — `FUN_00748860`** ([`00748860_FUN_00748860.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/00748860_FUN_00748860.c)): `payload[1] ∈ 1..6` hiển thị toast (`1,2`) hoặc chat hệ thống (`3..6`) cho sự kiện **Ba Đậu Yêu** (`1,5`: kết thúc trao đổi; `3`: bắt đầu tại quảng trường Trác Quận tìm `"Ba Đâu Yêu Ai Thần"`) và **Tứ Hải** cho người chơi mới (`2,6`: kết thúc; `4`: bắt đầu tại đường Trác Quận tìm `"Tiểu ca sai nha"`).
4. **`0x14 0x38 sel=2` — `FUN_0074e82c`** ([`0074e82c_FUN_0074e82c.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0074e82c_FUN_0074e82c.c)): Chỉ thực thi khi `LocalActor+0x63a ∈ {0x3077, 0x3083}` và `[+0x1511] == 0`: đặt timer `300000ms` (`+0x545/+0x546` DWORDs), bật cờ `[+0x1511]=1, [+0x1512]=2`, gọi `FUN_0072b390(self, 0x26)` rồi dịch chuyển nhân vật về tọa độ spawn slot `RP[2]` (`gvar_007DA6DC[RP[2]]+0x54/+0x58`) via `FUN_00731a9c`.
5. **`0x14 0x2C` hàm con `0x0071e288` & `0x0071f9ec`**:
   - `0x0071e288` (mode 1): set `actor+0x5d9 = 1`, `actor+0x5e0 = 1`, lưu timestamp `Now()` vào `QWORD [actor+0x5e8]`.
   - `0x0071f9ec` (mode 2): clear `actor+0x5d9 = 0`.
   - Khi `+0x5d9 != 0` và ngoài trận (`actor+0x376 == 0`), hàm render `0x71f888..0x71f9c5` cứ mỗi `500ms` (`0x1f4`) đổi frame `1..4` (`32px`) để vẽ sprite `"Waiteven"` (`0x71f9e0`) phía trên đầu nhân vật (`x+15, y-100`).

### 4.4. `0x18 0x08 action == 2` (`FUN_00729a88` L100–122)

- Khi nhận `S→C 0x18 0x08` với `action == 2`, [`00729a88_FUN_00729a88.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/00729a88_FUN_00729a88.c#L100-L122) ghi byte giá trị vào **`LocalActor+0x455`** (nếu là bản thân) hoặc **`gvar_007DA6BC[idx]+0x37`** và **`gvar_007DA300[idx]+0x455`** (nếu là player/NPC khác).
- Disassemble hàm render `0x7331b3..0x73364c`: kiểm tra `actor+0x455 > 0`, `actor+0x79 == 2` (do `0x7331a9` bỏ qua `1`, còn `0x7331c9` `dec al; sub al, 2; jae` chỉ đi tiếp khi `al == 2`), `[gvar_007D9D34]+0x53fc != 0`, sau đó vẽ khung ảnh `65x65` (`actor+0x458 * 0x41`) từ `gvar_007DA3E8` theo hướng `actor+0x460` (`0..3` tra mảng `gvar_007DA1C0[0..3]`) qua `0x7cb650`.

### 4.5. Ghi chú đối chiếu thêm cho §1 (`0x1A` trong `aLogin.exe`)

- Kiểm tra bảng nhảy `SendCommand` (`FUN_0077f414`) tại `0x785ffd` cho thấy `case 0x1A` trong `aLogin.exe` **không hoàn toàn rỗng**: có xử lý **`CL=1`** (`0x786011`: đọc `[gvar_007DA5A8+0x152]` / `+0x154`) và **`CL=2`** (`0x7860c8`: đọc `[gvar_007DA720+0x160]` / `+0x162`), và có 4 lệnh gọi ở `0x5dc42e (CL=1)`, `0x5dc476 (CL=2)`, `0x6324c7 (CL=3)`, `0x63284b (CL=4)` (riêng `CL=3,4` nhảy về nhánh thoát `0x78a4f2`). Điều này vẫn khớp với kết luận `0x1A` không phải kênh talk NPC.
