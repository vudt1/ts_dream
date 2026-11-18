# Báo Cáo Nghiên Cứu Đối Chiếu: `Warps.txt`, `eve.emg` và `Warp.Dat`

- **Dự án**: `ts_dream` (Rust game server cho TS Online PC)
- **Ngày thực hiện**: 18/11/2026
- **Phạm vi**: Đối chiếu trực tiếp từ nguồn sự thật gốc (`Data/Warps.txt`, `Data/BattleGate.txt`, `Data/eve.emg`, `Data/Warp.Dat`, `Data/Mark.Dat`) và mã nguồn Rust (`src/data/`, `src/eve/`, `src/server/handlers/`, `tests/`, `docs/adr/`).
- **Tài liệu liên quan**: [`research-data-and-eve-audit.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/data-files-and-eve-audit/research-data-and-eve-audit.md)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

1. **Quan hệ giữa `Data/Warps.txt` và `Data/eve.emg`**:
   - Không có file `Wraps.txt` trong `Data/`; file thực tế là [`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Warps.txt) (`119,108` bytes, `5,043` dòng → **`4,978`** cặp khóa `(map1, warpid)` duy nhất).
   - `Warps.txt` là bảng ánh xạ phẳng thủ công tương đương với **chuỗi 3 tầng trong `Data/eve.emg`**: `Doors (Section 3)` → `NpcEvents (Section 8, result_type == 2)` → `SceneInfos (Section 6)`. Cột `warpid` trong `Warps.txt` và `BattleGate.txt` là **`Door.id`** (`EveDoorPlacement.id`), **không phải** trực tiếp là `SceneInfo.eve_no` (`1,729 / 8,113` cửa trong `eve.emg` có `rt=2 parameter != Door.id`).
   - **Phát hiện bug runtime quan trọng**: Hàm [`resolve_warp_destination`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L927-L957) hiện đang tra thẳng `scene.scene_infos.get(&warp_id)` trước khi fallback `data.warps`. Khi được gọi từ luồng `0x14 08` ([`handle_warp_confirm`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1214-L1233) truyền `Door.id`), cách tra thẳng này bị **lệch `dest_map` trên `677 / 4,978` (`13.6%`)** bản ghi `Warps.txt` (bao gồm **`22 / 68`** cổng của `BattleGate.txt`), và bỏ sót `54` bản ghi. Khi đối chiếu đúng qua chuỗi `Door → NpcEvent(rt=2) → SceneInfo`, độ trùng khớp với `Warps.txt` nhảy vọt từ `81.7%` lên **`94.9%` khớp tuyệt đối** (`4,722`) và **`99.0%` khớp `dest_map`** (`4,926 / 4,978`).

2. **Có nên thay thế `Warps.txt` bằng `eve.emg` để tiết kiệm RAM?**
   - **Về mặt tiết kiệm RAM: KHÔNG đáng kể**. `GameData.warps` chỉ chiếm khoảng **`456.0 KB` (`466,992` bytes ≈ `0.45 MB`)** RAM, trong khi `GameData.scene_eve_data` **đã được nạp sẵn** khi boot (`~18.89 MB+` cho các cấu trúc chính). Gỡ `Warps.txt` chỉ giảm `< 2%` RAM của `scene_eve_data`.
   - **Về mặt kiến trúc & tính đầy đủ**: `eve.emg` vượt trội (`8,113` cửa có `rt=2`, bổ sung **`3,162`** cửa mà `Warps.txt` thiếu, đồng thời hỗ trợ `282` cửa đa đích theo điều kiện quest/menu). Tuy nhiên, **không thể xóa thẳng `Warps.txt` ngay lập tức** vì:
     + Có **`27`** bản ghi trong `Warps.txt` không tồn tại chuỗi `Door → rt=2` trong `eve.emg` (`22` cửa không có `rt=2` như tháp `11901..11910`, `12994..12999`; `5` cửa không có trong `eve.doors` như `(11000, 21)`, `(11840, 1)`).
     + Có **`25`** bản ghi khác `dest_map` (bao gồm override custom/private server như `(59401, 2) -> 12003` và fix lỗi upstream trong `eve.emg` như `(25303, 2)`).
     + Có **`204`** bản ghi cùng `dest_map` nhưng khác tọa độ `(x, y)` (trong đó có `7` placeholder `(500, 500)`).
   - **Khuyến nghị**: Tách rõ lookup theo `SceneInfo.eve_no` (cho `eve_runner`) và lookup theo `Door.id` (cho `0x14 08`) để sửa bug sai map `677` cửa trước, sau đó giữ `Warps.txt` (hoặc chuyển thành patch-override `~52` dòng) tùy nhu cầu server.

3. **Hiện trạng `Data/Warp.Dat` (`690` bytes)**:
   - Theo [ADR 0003](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0003-rank-ground-port.md) và [`src/data/loader.rs:341-355`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L341-L355), `Warp.Dat` thuộc nhóm **Deferred Boot**: production `GameData::load()` **không** parse vào `GameData.warp_defs` (chỉ lưu raw bytes trong `raw_binary_assets`). `WarpDatLoader::load` chỉ được gọi trong [`load_legacy_text`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L248-L252) (không dùng cho production) và chưa có unit test hay live consumer (dành cho truyền tống thành thị / opcode `0x44`).
   - **Giải mã nhị phân thực tế `Data/Warp.Dat` (`690 = 30 × 23` bytes)**: Đây **không** phải bảng cổng bước map như `Warps.txt`, mà là **danh mục 28 thành thị lớn** (`12001` Trác Quận → `61011` Phù Nam) gắn trực tiếp với `Mark.Dat` (`11804..11831`, `bit_pos = 936..963`: *"Vào thành ..."*).
   - **Phát hiện 2 bug nghiêm trọng trong [`src/data/loaders/warp.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs#L46-L74)**:
     1. Gọi nhầm `reader.decode_all(Some(23), Some(count))` trong khi `Data/Warp.Dat` thực chất dùng mã hóa **PC fixed-record XOR/offset** (`scene`: `(u16 ^ 0x00C8) - 1`; `mark`: `(u16 ^ 0x5209) - 1`; `x, y`: `(u32 ^ 0x0BAEB716) - 1`; `name`: `1B len + 10B Big5 đảo ngược`).
     2. Vòng lặp đọc `4 + 2 + 2 + 4 + 4 + 3 = 19` bytes mỗi record trong khi kiểm tra `remaining() >= 23`, khiến dữ liệu bị lệch `4` bytes ở mọi bản ghi sau index `0`.

---

## 2. Phân Tích Chi Tiết Câu Hỏi 1: `eve.emg` vs `Data/Warps.txt`

### 2.1 Cấu trúc thực tế của `Data/Warps.txt` và `Data/BattleGate.txt`

1. **[`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Warps.txt)** (`119,108` bytes, `5,043` dòng ASCII):
   - Định dạng cột phân tách bằng `\t`: `map1 \t warpid \t map2 \t x \t y`.
   - Kiểm tra trực tiếp nội dung file: `49` dòng comment `//`, `4,994` dòng dữ liệu hợp lệ, `0` dòng có cột đích rỗng.
   - Có **`16` cặp khóa `(map1, warpid)` trùng lặp** trong file; [`GameData::load_warps`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L763-L791) chèn vào `HashMap` nên bản ghi sau ghi đè bản ghi trước, kết quả có đúng **`4,978`** bản ghi duy nhất trong `GameData.warps`:
     + `14` cặp trùng có giá trị `(map2, x, y)` giống hệt nhau (ví dụ `(11301, 1)`, `(12000, 14)`, `(12001, 8)`, `(13000, 29)`...).
     + `2` cặp trùng có giá trị **khác nhau**: dòng `1286` ghi đè `(13282, 1)` từ `(13002, 802, 1135)` sang `(13051, 1290, 870)`; dòng `4921` ghi đè `(27000, 2)` từ `(27021, 150, 830)` sang `(27851, 2310, 310)`.

2. **[`Data/BattleGate.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/BattleGate.txt)** (`4,045` bytes):
   - Định dạng: `Mapid1 \t WarpId \t Diahinh \t Defender1..10`.
   - Có `2` dòng comment `//` (dòng 1 tiêu đề và dòng 69 `//12922 ...`) và **`68`** chốt chặn cửa ải đang hoạt động nạp vào `GameData.battle_gates`.

### 2.2 Cơ chế chuyển map trong `Data/eve.emg`

Qua phân tích nhị phân [`Data/eve.emg`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/eve.emg) (`9,839,580` bytes; `3,901` entries trong directory, **`3,897`** scene không rỗng sau [`EveDataLoader::load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L350-L416)), cơ chế warp trong `eve.emg` không nằm trong 1 bảng phẳng mà trải qua **3 section liên kết**:

1. **Section 3 — `Doors` ([`EveDoorPlacement`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L178-L183), [`parse_door_section`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L536-L582))**:
   - Tổng cộng **`9,887`** cửa trên toàn bộ `3,897` scene (`9,794` cửa có danh sách `events` không rỗng).
   - Mỗi cửa giữ `id: u16` (ID cổng trên map), tọa độ tâm pixel `(door_x, door_y)` tính từ `(grid_x + grid_w / 2) * 20`, và danh sách `events: Vec<u8>` trỏ sang `NpcEvents`.
2. **Section 8 — `NpcEvents` ([`NpcEventData`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L157-L162), [`parse_npc_event_section`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L797-L890))**:
   - Chứa `23,659` sự kiện, `69,404` điều kiện, `150,098` kết quả, trong đó có **`10,336` kết quả `result_type == 2` (`EveResultType::Door`)**.
   - **100% (`10,336 / 10,336`)** kết quả `rt = 2` đều có `parameter` trỏ tới một `EveSceneInfo` hợp lệ trong cùng scene (`dest_map_id > 0 && player_appear_x > 0 && player_appear_y > 0`).
   - Phân bố nguồn kích hoạt `rt = 2`: `9,630` gắn từ `Door`, `689` từ `NPC` (ví dụ NPC đưa đi map nhiệm vụ), `23` từ `Encounter`.
   - Có **`8,113` cửa** (`Door`) có ít nhất 1 đường dẫn `Door → NpcEvent → rt=2 → SceneInfo`:
     + **`6,384`** cửa có `rt=2 parameter == door.id`.
     + **`1,729`** cửa có **`rt=2 parameter != door.id`** (ID của `Door` và key `eve_no` của `SceneInfo` **không đồng nhất**).
     + **`282`** cửa có từ 2 đích `rt=2` khác nhau trở lên (ví dụ rẽ nhánh theo điều kiện nhiệm vụ `cc = 2` hoặc lựa chọn menu `cc = 10` như bến cảng Thanh Châu `(11000, 11)` đi `55000` hoặc `58000`).
   - Phân bố `when_happen` trên các event của `Door`: `[False, True, False, False]` (`on_stroke_obj`, tương ứng `MeetDoor` khi bước vào cổng) chiếm `9,832` trường hợp; `[True, False, False, False]` (`on_click_obj` / `ClickDoor`) chiếm `122` trường hợp.
3. **Section 6 — `SceneInfos` ([`EveSceneInfo`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L187-L194), [`parse_scene_info_section`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L704-L741))**:
   - Tổng cộng **`8,882`** bản ghi (`eve_no > 0`), 100% đều hợp lệ (`dest_map_id > 0 && player_appear_x > 0 && player_appear_y > 0`).
   - Tọa độ tile lưới `(player_appear_x, player_appear_y)` quy đổi sang tọa độ pixel trên server qua công thức `tile * 20 - 10` ([`src/server/handlers/quest.rs:935-936`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L935-L936)).

### 2.3 Kết quả đối chiếu định lượng giữa `Warps.txt` (`4,978`) và `eve.emg`

Bảng dưới đây đối chiếu toàn bộ **`4,978`** cặp `(map1, warpid)` của `Warps.txt` theo **2 cách tra cứu** trong `eve.emg`:

| Nhóm đối chiếu | Cách A: Tra thẳng `scene_infos[warpid]` (code hiện tại) | Cách B: Tra đúng chuỗi `Door[warpid] → NpcEvent(rt=2) → SceneInfo[param]` | Ghi chú kỹ thuật |
|:---|---:|---:|:---|
| **1. Khớp hoàn toàn (`dest_map`, `x`, `y`)** | **4,067** (81.7%) | **4,722** (94.9%) | Cách B sửa khớp thêm 655 cổng |
| **2. Cùng `dest_map`, khác `(x, y)`** | **180** (3.6%) | **204** (4.1%) | `7` dòng `(500, 500)`; `88` lệch `≤ ±40 px` |
| **3. Khác `dest_map`** | **677** (13.6%) | **25** (0.5%) | Cách A trỏ nhầm `SceneInfo` của cổng khác! |
| **4. Có `Door` nhưng không có `rt=2`** | — | **22** (0.4%) | Gồm `11901..11910`, `12994..12999`, v.v. |
| **5. Không tồn tại trong `eve`** | **54** (1.1%) | **5** (0.1%) | Chỉ 1 map (`11840`) vắng mặt hoàn toàn |
| **Tổng `Warps.txt`** | **4,978** | **4,978** | |
| **Có trong `eve.emg`, thiếu ở `Warps.txt`** | **3,958** (`SceneInfos`) | **3,162** (`Door → rt=2`) | Map mới (`30000+`), `10801..10820`, v.v. |

#### Giải mã nguyên nhân các sai lệch:
1. **Tại sao Cách A sai `677` `dest_map`?**
   - Trong thực tế dữ liệu PC, `Warps.txt` đánh khóa theo **`Door.id`**, còn `SceneInfos` đánh khóa theo **`EveResult.parameter`** của `rt = 2`. Trên các bản đồ lớn có nhiều cổng (như Thanh Châu `11000`, U Châu `12000`, Tư Lệ `13000`, Từ Châu `15000`), `Door.id` và `SceneInfo.eve_no` bị đánh số lệch pha nhau (ví dụ ở `12000`, `Door 14` kích hoạt `rt=2` đi `12441`, nhưng `scene_infos[14]` lại là điểm đi `12421`). Vì [`resolve_warp_destination`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L933-L945) hiện tra thẳng `scene.scene_infos.get(&warp_id)` trước khi xét `Warps.txt`, **tất cả `677` cổng này đang bị warp nhầm bản đồ nếu đi qua `0x14 08`**!
2. **Tại sao có `204` dòng cùng `dest_map` nhưng khác `(x, y)`?**
   - Tọa độ `eve.emg` luôn là `(tile * 20 - 10)` nên chữ số hàng đơn vị của `(x, y)` luôn lẻ dạng `*10, *30, *50, *70, *90`.
   - `Warps.txt` được thu thập thủ công (hoặc từ server private cũ) nên có `7` dòng điền tay `(500, 500)` (ví dụ `49902..49907`, `59401`) và nhiều dòng có đuôi `*2, *5` (ví dụ `522, 495`; `222, 295`; `3502, 1315`) là tọa độ đứng gần cổng khi sniff/ghi chép; `88 / 204` trường hợp lệch không quá `2` ô gạch (`≤ ±40 px`).
3. **Chi tiết `25` dòng khác `dest_map` ở Cách B**:
   - Một số là chỉnh sửa riêng của private server trong `Warps.txt`: ví dụ `(59401, 2)` trỏ về `(12003, 500, 500)` (quảng trường Trác Quận) thay vì `(23423, 290, 510)`; `(11000, 20)` trỏ `(58000, 490, 1790)` thay vì `(11047, 1110, 810)`; `(27000, 2)` dòng `4921` ghi đè thành `27851` trong khi dòng cũ trong chính `Warps.txt` khớp `eve.emg` là `27021`.
   - Một trường hợp lỗi dữ liệu gốc trong `eve.emg`: `(25303, 2)` trong `eve.emg` có `background_no = 25304` nhưng `dest_map_id = 35304` (nhầm đầu `3`), còn `Warps.txt` đúng là `25304`.
4. **Chi tiết `27` dòng trong `Warps.txt` thiếu chuỗi `Door → rt=2` trong `eve.emg`**:
   - **`22` dòng có `Door` trong `eve.emg` nhưng `NpcEvent` không có `rt=2`**:
     + `10` tầng map `11901..11910` (warp `2`) và `5` map `12994..12999` (warp `1`): `Door.events` chỉ chứa event hội thoại `[1]`/`[2]` không có `rt=2` (đáng chú ý `11901..11910` có `scene_infos[2]` đúng đích `11902..11911`, còn `12994..12999` có `scene_infos[1]` trỏ `12931`/`12938` khác với `Warps.txt`).
     + `2` cửa không có event (`door_evs=[]`): `(11000, 18)` → `15000` và `(12003, 9)` → `49917` (map event).
     + `5` cửa chỉ có hội thoại/battle không có `rt=2`: `(12922, 4)`, `(12923, 3)`, `(12934, 2)`, `(31416, 3)`, `(60511, 4)`.
   - **`5` dòng không có `Door` tương ứng trong `eve.doors`**: `(11000, 21) -> 55000`, `(11011, 17) -> 11821`, `(11583, 4) -> 11585`, `(12854, 2) -> 12853`, và `(11840, 1) -> 11000` (map `11840` không có trong `eve.emg`).

---

## 3. Phân Tích Câu Hỏi 2: Có Thể Thay Thế `Warps.txt` Bằng `eve.emg` Để Tiết Kiệm RAM?

### 3.1 Định lượng RAM thực tế trên Rust (64-bit `x86_64`)

Dựa trên định nghĩa struct tại [`src/data/tables.rs:243-258`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/tables.rs#L243-L258), [`src/data/loaders/eve.rs:128-299`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L128-L299) và kích thước cấp phát bảng băm `hashbrown::HashMap` (load factor `7/8`, công suất lũy thừa 2 kế tiếp + `1` byte control mỗi bucket):

| Thành phần trong `GameData` | Số lượng phần tử | Kích thước phần tử / Key-Value | Dung lượng RAM ước tính (gồm bucket capacity) | Trạng thái boot hiện tại |
|:---|---:|:---|---:|:---|
| **`data.warps`** (`HashMap<(i64, i64), Warp>`) | `4,978` warps | Key `16B` + `Warp` `40B` = `56B` (+`1B` ctrl), `cap = 8,192` | **`466,992` B (~`456.0 KB` / `0.45 MB`)** | Load từ `Warps.txt` |
| **`data.battle_gates`** (`HashMap<(i64, i64), BattleGate>`) | `68` gates | Key `16B` + `BattleGate` `104B` = `120B` (+`1B` ctrl), `cap = 128` | **`15,536` B (~`15.2 KB`)** | Load từ `BattleGate.txt` |
| **`data.scene_eve_data`** (outer `HashMap<u32, SceneEveData>`) | `3,897` scenes | Key `4B` + pad `4B` + `552B` = `560B` (+`1B` ctrl), `cap = 8,192` | **`4,595,760` B (~`4.38 MB`)** | **Đã luôn nạp sẵn** |
| └─ riêng `scene_infos` | `8,882` entries | Key `2B` + pad `2B` + `EveSceneInfo` `16B` = `20B` (+`1B` ctrl) | **`402,444` B (~`393.0 KB`)** | Thuộc `scene_eve_data` |
| └─ riêng `doors` + `door_events_by_door` | `9,887` doors | `48B/slot` + `32B/slot` + `Vec` buffers | **`1,676,759` B (~`1.60 MB`)** | Thuộc `scene_eve_data` |
| └─ `npcs`, `npc_events`, `fight_datas`, `encounters` | `21.5k` NPC, `23.6k` ev, `22.8k` fight | Chi tiết từng struct xem `eve.rs` | **`13,129,512` B (~`12.52 MB`)** | Thuộc `scene_eve_data` |
| **Tổng `scene_eve_data`** *(chưa tính `surfaces`, `groups` và allocator overhead)* | `3,897` scenes | | **`19,804,475` B (~`18.89 MB`)** *(thực tế RSS với `surfaces`/`groups`/allocator ~`22–26 MB`)* | **Bắt buộc cho Eve engine** |

> [!NOTE]
> `GameData.scene_eve_data` **đã luôn nằm trong RAM** ở production boot ([`src/data/loader.rs:375`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L375)) để phục vụ hệ thống quest/talk/encounter (`src/eve/`). Vì vậy, bỏ `Warps.txt` **chỉ giải phóng tối đa `~456 KB` (`0.45 MB`)** RAM (chiếm ~`1.8–2.4%` lượng RAM của riêng `scene_eve_data`).

### 3.2 Tác động tới `BattleGate.txt` và Runtime Handlers nếu xóa `Warps.txt`

Qua kiểm tra tất cả điểm tiêu thụ tại [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L911-L1249), [`src/server/handlers/eve_runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L915-L964), [`src/server/handlers/movement.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L67-L114), [`src/server/handlers/talk.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L328-L371), [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L624-L654), và [`tests/data_loaders/warps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders/warps.rs):

1. **Hai luồng warp đang dùng chung [`resolve_warp_destination`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L927-L957) với 2 không gian khóa khác nhau**:
   - **Luồng Eve (`MeetDoor` tại `movement.rs:90`, `ClickDoor` tại `talk.rs:348`, `eve_runner.rs:916`, `battle/service.rs:624`)**: Đi từ `Door.id` → `NpcEvent` → `EveResult(rt=2, parameter = P)` → `DomainEffect::Warp { warp_id: P }`. Ở đây tham số truyền vào `resolve_warp_destination` là **`SceneInfo.eve_no` (`P`)**, **không** cần `Warps.txt`.
   - **Luồng Talk-Warp `0x14 08` ([`handle_talk_warp`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L360-L371) → [`handle_warp_confirm`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1139-L1249))**: Client gửi `idtalking` là **`Door.id`**. Hàm `handle_warp_confirm` kiểm tra `data.battle_gates.get(&(map_id, idtalking))` rồi gọi `perform_party_warp_and_persist(..., warp_id_u16, out)` → `resolve_warp_destination(data, map_id, warp_id_u16)`.
2. **Hậu quả nếu xóa thẳng `Warps.txt` mà không sửa `resolve_warp_destination`**:
   - **677 cổng** (gồm `22 / 68` `BattleGate`) có `Door.id != SceneInfo.eve_no` sẽ tiếp tục warp **sai bản đồ** do `resolve_warp_destination` khớp nhầm vào `scene_infos[Door.id]`!
   - **`54` cổng** thiếu key trong `scene_infos[Door.id]` sẽ bị **từ chối warp hoàn toàn**, làm gãy **3 cổng `BattleGate.txt`** (`(12815, 5)`, `(13401, 2)`, `(15481, 5)`) — dù thực tế trong `eve.emg` cả 3 cổng này đều có `Door → NpcEvent → rt=2 → SceneInfo[4]` khớp 100% với `Warps.txt`!
   - Làm hỏng các test hiện có ([`tests/data_loaders/warps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders/warps.rs#L6-L17) và một số test trong `tests/server_network/code_review_remediation.rs` chèn trực tiếp vào `data.warps`).
3. **Quan hệ `BattleGate.txt` (`68` cổng) với `eve.emg`**:
   - **`68 / 68`** cổng của `BattleGate.txt` đều tồn tại trong `eve.doors`.
   - **`58 / 68`** cổng đã có trận đấu `rt=3` (Battle) ngay trong `Door → NpcEvent` của `eve.emg`.
   - **`10 / 68`** cổng **không** có `rt=3` trong `eve.emg` (gồm `8` cổng tháp `49901..49908` dùng kiểm tra quest/custom và `(15481, 4)`, `(21000, 40)`).
   - Khi đối chiếu đích đến qua `Door → rt=2 → SceneInfo`, `65 / 65` cổng có trong `Warps.txt` đều có đường `Door → rt=2` (`49` khớp tuyệt đối, `15` cùng map khác `(x, y)`, `1` khác map `(15402, 1)`), còn `3` cổng `BattleGate.txt` chưa có trong `Warps.txt` (`(12431, 3)`, `(21000, 40)`, `(15491, 2)`) đều có đích hợp lệ trong `eve.emg`.

### 3.3 Kết luận & Khuyến nghị cho Câu hỏi 2

- **Không nên xóa `Warps.txt` với lý do "tiết kiệm RAM"** vì chỉ tiết kiệm được `~456 KB` (`0.45 MB`), không đáng kể so với rủi ro làm gãy `27` warp đặc thù và `25` override custom.
- **Nên làm 2 bước**:
  1. **Sửa bug tra cứu `Door.id` vs `SceneInfo.eve_no` (ưu tiên cao)**: Tách `resolve_scene_info_warp(data, map_id, scene_info_id)` (cho `eve_runner` dùng `rt=2 param`) khỏi `resolve_door_warp(data, map_id, door_id)` (cho `0x14 08` dùng `Door.id` → `NpcEvent(rt=2)` → `SceneInfo`, hoặc `data.warps` nếu ưu tiên override). Việc này loại bỏ ngay lỗi warp nhầm map trên **`677` cổng** (bao gồm `22` `BattleGate`) và mở khóa thêm **`3,162` cổng** từ `eve.emg`.
  2. Nếu muốn đơn giản hóa dữ liệu sau đó: Có thể bỏ hầu hết `4,900+` dòng trùng lặp trong `Warps.txt` và chỉ giữ phần override/bổ sung (`~52` dòng gồm `27` thiếu `rt=2` + `25` khác `dest_map`), hoặc giữ nguyên `Warps.txt` như bảng fallback/override nhẹ (`456 KB`).

---

## 4. Phân Tích Câu Hỏi 3: `Data/Warp.Dat` và Hiện Trạng Trong Server Rust

### 4.1 Cách `Data/Warp.Dat` đang được nạp và sử dụng trong Rust

1. **Trong `GameData` & `loader.rs`**:
   - Trường lưu trữ: `pub warp_defs: HashMap<usize, WarpDef>` tại [`src/data/loader.rs:69`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L69).
   - **Production boot (`GameData::load` → `load_binary`)**: Theo chính sách *Deferred Catalog* ([ADR 0003](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0003-rank-ground-port.md), [`src/data/loader.rs:341-355`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L341-L355)), `load_binary` **không** gọi `WarpDatLoader::load` và để `data.warp_defs` rỗng mặc định. File `Data/Warp.Dat` chỉ được đọc thô vào `raw_binary_assets` tại [`load_binary_asset_inventory`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L499-L530).
   - **Legacy/Test path (`load_legacy_text`)**: [`src/data/loader.rs:248-252`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L248-L252) có gọi `WarpDatLoader::load(&bytes)` để điền `d.warp_defs`, nhưng hàm `load_legacy_text` chỉ dành cho migration fixtures cũ, không chạy ở production.
   - **Consumer & Tests**: Không có bất kỳ handler nào trong `src/server/` đọc `data.warp_defs`, và cũng **không có unit test hay integration test nào** kiểm tra `WarpDatLoader` trên `Data/Warp.Dat` trong `src/` hoặc `tests/` (`tests/data_loaders/` chỉ test `Warps.txt` và `BattleGate.txt`). Ghi chú ở `loader.rs:350-354` và [`src/protocol/mod.rs:307-308`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L307-L308) xác nhận opcode `0x44` (`OP_RESERVED_44`) hiện chưa có handler.

### 4.2 Nội dung nhị phân thực tế của `Data/Warp.Dat` (`690` bytes)

Kiểm tra trực tiếp `690` bytes của [`Data/Warp.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Warp.Dat) cho thấy:
- `690 = 30 × 23` bytes (**30 bản ghi cố định 23 byte**):
  + **Record `0`** (`bytes 0..23`): toàn byte `0x00` (header rỗng chuẩn file `.Dat` PC).
  + **Records `1..=28`**: **`28` điểm truyền tống thành thị lớn** (dịch chuyển bằng đạo cụ/kỹ năng Truyền Tống / Thần Hành / Đại Bạch Mã).
  + **Record `29`** (`bytes 667..690`): `00..00 c9 00 08 52 17 b7 ae 0b 17 b7 ae 0b` — chính là **giá trị `0` sau mã hóa** (`(0+1)^0xC8 = 0xC9`, `(0+1)^0x5209 = 0x5208`, `(0+1)^0x0BAEB716 = 0x0BAEB717`), đóng vai trò sentinel kết thúc bảng.

#### Cấu trúc từng bản ghi 23 byte của `Data/Warp.Dat`:

| Offset | Độ dài | Trường | Kiểu & Mã hóa thực tế trong `Data/Warp.Dat` | Giá trị giải mã mẫu (Record 1) |
|:---:|:---:|:---|:---|:---|
| `0..11` | `11` B | `name` | `len: u8` (`0x04`) + `10B` chuỗi **Big5 đảo ngược** (không XOR) | `b267b070` → `涿郡` (Trác Quận) |
| `11..13` | `2` B | `scene` | `u16 LE`: `(raw ^ 0x00C8).wrapping_sub(1)` | `0x2E2A ^ 0x00C8 - 1 = 12001` |
| `13..15` | `2` B | `mark` | `u16 LE`: `(raw ^ 0x5209).wrapping_sub(1)` *(giống `NpcDatLoader::dec16`)* | `0x7C14 ^ 0x5209 - 1 = 11804` |
| `15..19` | `4` B | `x` | `i32 LE`: `(raw ^ 0x0BAE_B716).wrapping_sub(1)` *(giống `NpcDatLoader::dec32`)* | `0x0BAEB7B7 ^ 0x0BAEB716 - 1 = 160` |
| `19..23` | `4` B | `y` | `i32 LE`: `(raw ^ 0x0BAE_B716).wrapping_sub(1)` *(giống `NpcDatLoader::dec32`)* | `0x0BAEB17F ^ 0x0BAEB716 - 1 = 1640` |

#### Danh sách đầy đủ 28 bản ghi giải mã từ `Data/Warp.Dat` (đối chiếu với `Mark.Dat`):

| # | Tên Big5 | Thành thị | `scene` (`map_id`) | `mark` (`Mark.Dat`) | `bit_pos` (`Mark.Dat`) | `(x, y)` | Khớp cổng vào trong `eve.emg` (`SI`) / `Warps.txt`? |
|---:|:---|:---|---:|---:|---:|:---:|:---|
| 1 | 涿郡 | Trác Quận | `12001` | `11804` | `936` | `(160, 1640)` | Khác tọa độ cổng (`Warps`/`SI` vào từ `12000` là `170, 1630`) |
| 2 | 北海 | Bắc Hải | `11011` | `11805` | `937` | `(360, 560)` | Tọa độ điểm truyền tống nội thành |
| 3 | 鄴城 | Nghiệp Thành (Diệp Thành) | `12061` | `11806` | `938` | `(160, 900)` | Tọa độ điểm truyền tống nội thành |
| 4 | 鉅鹿 | Cự Lộc | `12011` | `11807` | `939` | `(1200, 900)` | Tọa độ điểm truyền tống nội thành |
| 5 | 洛陽 | Lạc Dương | `13001` | `11808` | `940` | `(880, 860)` | Tọa độ điểm truyền tống nội thành |
| 6 | 許昌 | Hứa Xương | `13011` | `11809` | `941` | `(120, 660)` | Tọa độ điểm truyền tống nội thành |
| 7 | 長安 | Trường An | `14001` | `11810` | `942` | `(900, 660)` | Tọa độ điểm truyền tống nội thành |
| 8 | 徐州 | Từ Châu | `15001` | `11811` | `943` | `(860, 660)` | Tọa độ điểm truyền tống nội thành |
| 9 | 壽春 | Thọ Xuân | `15021` | `11812` | `944` | `(160, 640)` | Tọa độ điểm truyền tống nội thành |
| 10 | 建業 | Kiến Nghiệp | `18001` | `11813` | `945` | `(180, 1180)` | Tọa độ điểm truyền tống nội thành |
| 11 | 會稽 | Hội Kê | `18021` | `11814` | `946` | `(220, 620)` | Tọa độ điểm truyền tống nội thành |
| 12 | 襄平 | Tương Bình | `19001` | `11815` | `947` | `(1220, 1100)` | Tọa độ điểm truyền tống nội thành |
| 13 | 上黨 | Thượng Đảng | `20001` | `11816` | `948` | `(860, 660)` | Tọa độ điểm truyền tống nội thành |
| 14 | 襄陽 | Tương Dương | `21001` | `11817` | `949` | `(880, 660)` | Tọa độ điểm truyền tống nội thành |
| 15 | 江陵 | Giang Lăng | `21011` | `11818` | `950` | `(120, 660)` | Tọa độ điểm truyền tống nội thành |
| 16 | 長沙 | Trường Sa | `23001` | `11819` | `951` | `(710, 810)` | Trùng `(23000, 20)` `Warps` & `(23000, 22)` `SI` |
| 17 | 零陵 | Linh Lăng | `23011` | `11820` | `952` | `(150, 830)` | Trùng `(23000, 9)` `Warps` & `(23000, 10)` `SI` |
| 18 | 天水 | Thiên Thủy | `24001` | `11821` | `953` | `(290, 750)` | Trùng `(24000, 1)` `Warps` & `SI` |
| 19 | 敦煌 | Đôn Hoàng | `24011` | `11822` | `954` | `(630, 770)` | Trùng `(24000, 3)` `Warps` & `SI` |
| 20 | 成都 | Thành Đô | `25001` | `11823` | `955` | `(830, 630)` | Tọa độ điểm truyền tống nội thành |
| 21 | 南鄭 | Nam Trịnh | `25011` | `11824` | `956` | `(830, 630)` | Trùng `(25731, 1)` `SI` |
| 22 | 白帝 | Bạch Đế | `25021` | `11825` | `957` | `(770, 790)` | Tọa độ điểm truyền tống nội thành |
| 23 | 番禺 | Phiên Ngung | `26001` | `11826` | `958` | `(130, 790)` | Trùng `(26000, 27)` `Warps` & `SI` |
| 24 | 交趾 | Giao Chỉ | `26011` | `11827` | `959` | `(130, 790)` | Trùng `(26000, 10)` `Warps` & `SI` |
| 25 | 車師 | Xa Sư | `28011` | `11828` | `960` | `(130, 550)` | Trùng `(28000, 5)` `SI` |
| 26 | 疏勒 | Sơ Lặc | `29001` | `11829` | `961` | `(870, 550)` | Trùng `(29801, 2)` `SI` |
| 27 | 林邑 | Lâm Ấp | `61001` | `11830` | `962` | `(110, 850)` | Trùng `(61000, 14)` `Warps` & `(61000, 17)` `SI` |
| 28 | 扶南 | Phù Nam | `61011` | `11831` | `963` | `(1090, 850)` | Trùng `(61000, 9)` `Warps` & `(61000, 18)` `SI` |

> [!IMPORTANT]
> Cả `28` trường `mark` (`11804..11831`) trong `Warp.Dat` ánh xạ **1-1 hoàn hảo** tới `28` bản ghi liên tiếp trong [`Data/Mark.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Mark.Dat) (`bit_pos = 936..963`), với tên tiếng Việt VISCII trong `Mark.Dat` là *"Vào Trác Quận Thành"*, *"Vào Thành Bắc Hải"*, ..., *"Vào thành Phù Nam"*. Điều này khẳng định mục đích của `Warp.Dat`: kiểm tra cờ điều kiện xem người chơi đã từng đặt chân vào thành thị đó hay chưa (`quest_dont` bit `936..963`) trước khi cho phép truyền tống thẳng đến tọa độ `(scene, x, y)`.

### 4.3 Hai lỗi trong `src/data/loaders/warp.rs`

Khi chạy [`WarpDatLoader::load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs#L46-L74) lên `Data/Warp.Dat` hiện tại, parser cho ra toàn bộ giá trị rác (ví dụ `rec[0]: scene=0, mark=28672, x=-474847312`; `rec[1]: scene=0, mark=0, x=-1845493760`) vì 2 lỗi:
1. **Sai thuật toán giải mã ([`src/data/loaders/warp.rs:48-50`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs#L48-L50))**: Gọi `reader.decode_all(Some(23), Some(count))` (cơ chế XOR `key0/key1` 2 khối cuối file), khiến `key0` và `key1` thực chất là record `28` (Phù Nam) và record `29` (trailer `0`-encoded) bị XOR nhầm lên các bản ghi trước đó.
2. **Sai kích thước đọc bản ghi ([`src/data/loaders/warp.rs:53-60`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs#L53-L60))**: Đọc `4B` (`name: u32`) + `2B` (`scene`) + `2B` (`mark`) + `4B` (`x`) + `4B` (`y`) + `3B` padding = **`19` bytes**, trong khi guard vòng lặp là `remaining() >= 23`. Kết quả tạo ra `32` bản ghi lệch pha `4` bytes mỗi vòng (`pos = 0, 19, 38, 57...`) thay vì `28` bản ghi chuẩn `23` bytes.

---

## 5. Tổng Kết Hành Động Khuyến Nghị

1. **Sửa [`resolve_warp_destination`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L927-L957)** (tách `resolve_scene_info_warp` dùng `SceneInfo.eve_no` và `resolve_door_warp` dùng `Door.id → NpcEvent(rt=2) → SceneInfo`) để khắc phục triệt để `677` cổng bị warp nhầm bản đồ (trong đó có `22` `BattleGate`) ở luồng `0x14 08`, trước khi tính đến việc rút gọn `Warps.txt`.
2. **Không cần xóa `Warps.txt` vì mục tiêu RAM**: Chỉ tiết kiệm `~456 KB`, đổi lại mất `27` warp không có `rt=2` và `25` override custom; có thể giữ `Warps.txt` làm tầng override/fallback sau `Door → rt=2 → SceneInfo` hoặc thu gọn thành file override `~52` dòng.
3. **Nếu triển khai tính năng truyền tống thành thị (`Warp.Dat`) trong tương lai**: Cần sửa [`WarpDatLoader::load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs#L46-L74) sang đúng layout `23` bytes (`1B len + 10B Big5 đảo ngược + scene/mark/x/y` với các khóa XOR `0x00C8`, `0x5209`, `0x0BAEB716` và offset `-1`, bỏ qua record `0` và record `29`), đồng thời bổ sung test giải mã cho cả `28` thành thị kèm đối chiếu `Mark.Dat` `11804..11831`.
