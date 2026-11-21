# Nghiên Cứu Chuyên Sâu Live-Test Map `#12003`: Thương Nhân BDY (`#16024`) & Ngự Mã Giám (`#19035`)

> **Phạm vi**: Chỉ đọc/phân tích mã nguồn và dữ liệu nhị phân (`Data/eve.emg`, `Data/Npc.dat`, `Data/Item.dat`, `src/`). Không chỉnh sửa code sản phẩm.

---

## Tóm Tắt Nhanh

1. **Vấn đề 1 — Thương nhân BDY `#16024` (`place_id = 8`, click `14 01 08 00`)**:
   - **Nguyên nhân gốc**: Lỗi đảo ngược toán tử `condition_ops = 4` trong [`invert_emg_ops`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L46-L54) khi đánh giá `condition_class = 1` (`BagItem`).
   - Trong `Data/eve.emg` map `12003`, `eve_no = 8` (đổi **Vé đoái hoán tâm `#31007`** lấy **Ba Đậu Yêu `#22028`**) dùng `condition_ops = 4, condition_value = 1`.
   - `invert_emg_ops(4)` biến `4` thành `2` (`LessThan`), nên server hiểu là **`qty(31007) < 1`** (tức `qty == 0`!).
   - Vì người chơi **đang có** `#31007` (`qty >= 1`), `eve_no = 8` **không khớp** `Cond 2` (`Surface 8`), chỉ chạy `Cond 1` (`cls=0` chào hỏi `12438`, `15963`). Sau khi kết thúc `eve_no = 8`, [`auto_chain_after`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L931) → [`EveAutoChainEngine::try_auto_chain`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L171-L271) duyệt tiếp danh sách event của NPC `#8` và khớp nhầm **`eve_no = 19`** (đổi **Vé đoái hoán túi đeo `#31023`** lấy `#22029`) vì `eve_no = 19` cũng dùng `ops=4, val=1` cho `#31023`, mà người chơi lại đang có `0` vé `#31023` (`0 < 1` = `true`!).
   - `eve_no = 19` mở **Surface `9`** (`09 00` trong packet `s2c`, không phải Surface `8`). Khi người chơi bấm chọn nhánh 1 (`14 09 1E`), `eve_no = 19` chạy [`preflight_probe_batch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L109-L155) kiểm tra trừ `1x #31023` → thất bại `PreFlightError::InsufficientItem { item_id: 31023 }` → phát `"Vật phẩm không đủ, không thể thực hiện."` ([`eve_runner.rs:264-266`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L264-L266)).

2. **Vấn đề 2 — Ngự Mã giám `#19035` (`place_id = 41`, click `14 01 29 00`)**:
   - **Nguyên nhân gốc**: Trong `Data/eve.emg` map `12003`, NPC `place_id = 41` (`npc_id = 19035`, tọa độ `(820, 800)`) chỉ gắn 4 event `[89, 90, 91, 92]`. Cả 4 event đều yêu cầu có sẵn 1 trong 4 vé/thú cưỡi tương ứng (`#31208`, `#31209`, `#31210`, `#31211`, với `cls=1, ops=2, val=0` → `qty > 0`) và **hoàn toàn không có nhánh thoại chào hỏi mặc định (`cls=0`)**.
   - Khi người chơi không có vật phẩm nào trong 4 ID trên trong `homdo` (hoặc đứng ngoài bán kính `130 px` so với `(820, 800)`), [`npc_event::resolve_npc_event`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L250-L257) trả về `None`.
   - `19035` không thuộc các nhánh hardcoded Banker/Inn/`16012` trong [`handle_talk_start`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L285-L305), nên rơi xuống `end_talk(conn, out)` tại [`talk.rs:305`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L305) và trả ngay `s2c F44402001408`.

---

## Phần 1: Phân Tích Chi Tiết Vấn Đề 1 — Thương Nhân BDY `#16024` (Map `#12003`, `place_id = 8`)

### 1.1. Dữ liệu thực tế từ `Data/eve.emg`, `Data/Npc.dat`, `Data/Item.dat`

- **NPC Placement (`Data/eve.emg`, scene `12003`)**:
  - `place_id = 8`, `npc_id = 16024` (*Thương nhân BDY*, `kind = 7` trong `Data/Npc.dat`), `pos = (690, 860)`, `dir = 1`.
  - Danh sách `events` (30 event): `[8, 19, 20, 29, 45, 46, 47, 48, 49, 50, 51, 52, 57, 58, 59, 60, 61, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 78, 79, 80]`.

- **Cấu trúc `eve_no = 8` (Đoái hoán tâm `#31007` → Ba Đậu Yêu `#22028`)**:
  - `when_happen = [true, false, false, false]` (`OnClickObject`)
  - `Cond 1` (`and=1`): `cls=0` (Unconditional)
    - `Res 1`: `rt=1, rc=3, par=8, mean=12438` (`0x3096`)
    - `Res 2`: `rt=1, rc=3, par=8, mean=15963` (`0x3E5B`)
  - `Cond 2..3` (`and=2`):
    - `Cond 2`: `cls=1 (BagItem), par=31007, ps=1, ops=4, val=1`
    - `Cond 3`: `cls=7 (Attr), par=5, ps=2, ops=0, val=0` (túi võ tướng `< 4` slot)
    - `Res 1`: `rt=6 (Surface), rc=3, par=8, mean=8` (**Surface `8`**)
  - `Cond 4..5` (`and=2`): `cls=10 par=8 ps=30` + `cls=9 val=22028 ops=5` (đang mang pet `22028`) → `Talk(13163)`
  - `Cond 6..7` (`and=2`): `cls=10 par=8 ps=30` + `cls=7 par=6 ps=3 val=22028 ops=5` (đang gửi pet `22028` ở khách sạn) → `Talk(12549)`
  - `Cond 8..12` (`and=5`):
    - `Cond 8`: `cls=10 (DialogChoice), par=8, ps=30`
    - `Cond 9`: `cls=1 (BagItem), par=31007, ps=1, ops=4, val=1`
    - `Cond 10`: `cls=7, par=5, ps=2, ops=0, val=0` (`follow_pet_count < 4`)
    - `Cond 11`: `cls=9, ps=1, ops=6, val=22028` (chưa mang `22028`)
    - `Cond 12`: `cls=7, par=6, ps=3, ops=6, val=22028` (chưa gửi `22028` ở khách sạn)
    - `Res 1`: `rt=0, rc=1, par=31007, ps=1, val=-1` (trừ `1x #31007`)
    - `Res 2`: `rt=0, rc=8, par=22028, ps=1, val=1` (thưởng pet `#22028`)
    - `Res 3`: `rt=1, rc=3, par=8, mean=12437`

- **Cấu trúc `eve_no = 19` (Vé đoái hoán túi đeo `#31023` → Ba Đậu Yêu Túi Đeo `#22029`)**:
  - `when_happen = [true, false, false, false]`
  - `Cond 1..2` (`and=2`):
    - `Cond 1`: `cls=1 (BagItem), par=31023, ps=1, ops=4, val=1`
    - `Cond 2`: `cls=7, par=5, ps=2, ops=0, val=0`
    - `Res 1`: `rt=6 (Surface), rc=3, par=8, mean=9` (**Surface `9`**!)
  - `Cond 7..11` (`and=5`):
    - `Cond 7`: `cls=10 (DialogChoice), par=9, ps=30`
    - `Cond 8`: `cls=1 (BagItem), par=31023, ps=1, ops=4, val=1`
    - `Cond 9..11`: `< 4` pet & chưa sở hữu `22029`
    - `Res 1`: `rt=0, rc=1, par=31023, ps=1, val=-1` (trừ `1x #31023`)
    - `Res 2`: `rt=0, rc=8, par=22029, ps=1, val=1`
    - `Res 3`: `rt=1, rc=3, par=8, mean=15960`

### 1.2. Truy vết từng packet trong live capture

| Packet | Giải mã | Thực tế xảy ra trên server |
|---|---|---|
| `c2s F444040014010800` | Click NPC `place_id = 8` (`#16024`) | [`handle_talk_start`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L212-L283) gọi `resolve_npc_event(ClickNpc(8))`. Trong `eve_no = 8`, `Cond 2` (`qty(31007) < 1`) bị `false` vì người chơi đang có `1x #31007`. Chỉ `Cond 1` (`cls=0`) khớp → tạo `EventSession(eve_no=8, matched_condition_no=1)`. |
| `s2c F44402000602` + `142CE093040001` | Dừng đi bộ + khóa thoại | [`activate_event_session`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1247-L1254). |
| `s2c F44411001401...9630` | `rt=1, mean=12438` (`0x3096`) | Kết quả 1 của `eve_no = 8, Cond 1`. |
| `c2s F44402001406` → `s2c ...5B3E` | `rt=1, mean=15963` (`0x3E5B`) | Kết quả 2 của `eve_no = 8, Cond 1`. |
| `c2s F44402001406` | Tiếp tục sau `15963` | `eve_no = 8` hết kết quả (`idx >= len`) → [`finish_event_session_iter`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L914-L935) gọi `auto_chain_after`. |
| *(Auto-chain nội bộ)* | Duyệt `[8, 19, 20, ...]` | `eve_no = 8`: `Cond 1` bị chặn bởi Guard #1 (`detect_same_chain`, [`auto_chain.rs:226-233`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L226-L233)); `Cond 2` vẫn `false` (`1 < 1` = `false`). Chuyển sang **`eve_no = 19`**: `Cond 1` kiểm tra `qty(31023) < 1`. Do người chơi có `0` vé `#31023`, `0 < 1` = **`true`**! Auto-chain kích hoạt `eve_no = 19`. |
| `s2c F44411001401000000010603080000000000000900` | `rt=6, par=8, mean=0x0009` (**Surface `9`**) | Thuộc `eve_no = 19, Cond 1` (chứ không phải Surface `8` của `eve_no = 8`!). |
| `c2s F444030014091E` | Chọn nhánh `0x1E = 30` trên Surface `9` | [`handle_talk_select_menu`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L553-L639) lọc `chain_has_dialog_choice(chain, 9, 30)` → khớp `eve_no = 19, Cond 7..11` (vì `Cond 8` `qty(31023) < 1` cũng `true`). |
| `s2c F4442900020C56A774...` | `"Vật phẩm không đủ, không thể thực hiện."` | [`execute_event_step_core`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1040-L1048) chạy [`preflight_probe_batch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L134-L141) trên `MutateItem { item_id: 31023, delta: -1 }`. Trong `trial_homdo` không có `#31023` → `PreFlightError::InsufficientItem { item_id: 31023 }` → [`emit_preflight_rejection`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L262-L266) phát thông báo lỗi và `14 2C 02` + `14 08`. |

### 1.3. Bằng chứng toàn cục trên `Data/eve.emg` về `condition_ops = 4`

Quét toàn bộ `Data/eve.emg` cho `condition_class = 1` (`BagItem`) cho thấy:
- `ops = 2, val = 0`: **1.757 lần** (`0 < qty` → `qty >= 1`, hoạt động đúng qua `invert_emg_ops(2) = 4`).
- `ops = 5, val = 0`: **469 lần** (`0 >= qty` → `qty == 0`, hoạt động đúng qua `invert_emg_ops(5) = 3`).
- `ops = 4, val = 1`: **252 lần** (bao gồm 12 event BDY `eve_no = 8, 19, 20, 29, 45..52` tại map `12003`, và nhiều quest khác như `10992 eve_no=6`, `11424 eve_no=4`, `13013 eve_no=9`, `13131 eve_no=3`).
- Thống kê tương quan với `Res` của `cls=1, ops=4` trên **toàn bộ** `Data/eve.emg`:
  - **Không có bất kỳ trường hợp nào** `ops = 4` có `val = 0` (`0` lần).
  - **Không có bất kỳ trường hợp nào** `ops = 4` đi kèm hành động tặng vật phẩm đó (`gives_same = 0` lần).
  - Phần lớn đều đi kèm hành động **trừ đúng `val` vật phẩm đó**: `val=1, takes_same=True` (**167 lần**), `val=5, takes_same=True` (**20 lần**, ví dụ `11424 eve_no=4`, `13013 eve_no=9`, `13131 eve_no=3`), `val=3` (**9 lần**), `val=2` (**5 lần**), `val=10` (**3 lần**), `val=50` (**2 lần**).
- Tương tự ở `cls=7, par=0` (Level): ví dụ `13131 eve_no=3 Cond 5` dùng `ops=4, val=15` để kiểm tra `level >= 15` (cũng có 128 lần `(4, 15)` và 86 lần `(4, 10)`).
- Trong khi đó, [`invert_emg_ops`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L46-L54) hiện ánh xạ `4 => 2` (`LessThan`), khiến mọi điều kiện `ops = 4, val = N` bị đảo ngược hoàn toàn thành `actual < N` thay vì `actual >= N` (hoặc tùy ngữ nghĩa gốc của `eve.emg`).

---

## Phần 2: Phân Tích Chi Tiết Vấn Đề 2 — Ngự Mã Giám `#19035` (Map `#12003`, `place_id = 41`)

### 2.1. Dữ liệu thực tế từ `Data/eve.emg` & `Data/Npc.dat`

- **NPC Placement (`Data/eve.emg`, scene `12003`)**:
  - `place_id = 41`, `npc_id = 19035` (*Ngự Mã giám*, `kind = 10` trong `Data/Npc.dat`), `pos = (820, 800)`, `dir = 1`.
  - Danh sách `events` (4 event): `[89, 90, 91, 92]`.

- **Chi tiết 4 event của `#19035`**:
  - `eve_no = 89` (`when_happen = [true, false, false, false]`):
    - `Cond 1..2` (`and=2`): `cls=1, par=31208, ps=1, ops=2, val=0` (*Bách Thiết Kỵ* `#31208` > 0) + `< 4` pet → `Surface 55` (`par=41`).
    - `Cond 7..11` (`and=5`): `DialogChoice(55, 30)` + có `#31208` + `< 4` pet + chưa có pet `#18055` → trừ `1x #31208`, tặng pet `#18055` (*Bách Thiết Kỵ*).
  - `eve_no = 90`:
    - `Cond 1..2`: `cls=1, par=31209, ops=2, val=0` (*Báu Mã Thiết Kỵ* `#31209` > 0) → `Surface 56` (`par=41`) → đổi lấy pet `#18057` (*Báu Mã Thiết Kỵ*).
  - `eve_no = 91`:
    - `Cond 1..2`: `cls=1, par=31210, ops=2, val=0` (*Hắc Thiết Kỵ* `#31210` > 0) → `Surface 57` (`par=41`) → đổi lấy pet `#18058` (*Hắc Thiết Kỵ*).
  - `eve_no = 92`:
    - `Cond 1..2`: `cls=1, par=31211, ops=2, val=0` (*Tử Lan Thiết Kỵ* `#31211` > 0) → `Surface 58` (`par=8` — lưu ý lỗi dữ liệu nhỏ từ `eve.emg` ghi `par=8` thay vì `41`) → đổi lấy pet `#18059` (*Tử Lan Thiết Kỵ*).

### 2.2. Truy vết xử lý tại `talk.rs` và `npc_event.rs`

1. Client gửi `c2s F444040014012900` (`sub = 1`, `map_object_id = 0x0029 = 41`).
2. [`handle_talk_start`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L240-L249) gọi `resolve_npc(data, conn, 41)` → tìm thấy `place_id = 41, npc_id = 19035` tại `(820, 800)` (nếu người chơi đứng ngoài bán kính `130 px`, `in_range = false` cũng sẽ gọi `end_talk`).
3. [`event_npcs::try_handle_event_npc`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L263) trả về `false` vì `19035` không phải NPC event đặc biệt.
4. [`npc_event::resolve_npc_event`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L266-L283) duyệt `npc_events_by_npc[41] = [89, 90, 91, 92]`:
   - Vì cả 4 event đều yêu cầu điều kiện đầu tiên là có vật phẩm `#31208`, `#31209`, `#31210`, hoặc `#31211` trong `homdo` (`qty > 0`), và **không có bất kỳ condition nào `cls = 0` (Unconditional)** làm hội thoại chào hỏi/fallback, nên nếu người chơi không có 1 trong 4 vật phẩm này, cả 4 event đều fail → `resolve_npc_event` trả về `None`.
5. `19035` không nằm trong các nhánh hardcoded `16080 | 16004 | 16011 | 16015` (Banker), `15002 | 16001 | 16016` (Inn), hay `16012` ([`talk.rs:285-303`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L285-L303)).
6. Luồng rơi xuống [`end_talk(conn, out)`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L305) → phát `s2c F44402001408`.

> **Lưu ý thêm**: Ở `eve_no = 89..92` dùng `condition_ops = 2, condition_value = 0` (`qty > 0`), nên nếu người chơi **có** vật phẩm `#31208` / `#31209` / `#31210` / `#31211` trong `homdo` và đứng trong tầm `130 px` của `(820, 800)`, hội thoại đổi thú cưỡi sẽ mở bình thường. Việc không hiện thoại khi không có vé là do thiết kế dữ liệu gốc của `Data/eve.emg` thiếu nhánh `cls = 0` fallback.
