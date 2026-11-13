# Research Backlog: Unverified Raw `Data/eve.emg` Codes & Client Branches

> **Type**: Pure Research Inventory (NOT an execution ticket; deferred outside the active `quest-refactor` implementation scope in [`spec.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md)).  
> **Runtime Policy**: While unverified, all items listed below MUST execute **fail-closed** (`false` in condition evaluation, safe skip + telemetry warning in result execution) per [`ADR 0009`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md).

---

## 1. Đính chính các mục KHÔNG còn là `UNKNOWN`

Qua đối chiếu bổ sung với tài liệu dịch ngược đã có trong repo:

1. **`0x14 0x0A` (`F4440200140A`) — ĐÃ ĐẶC TẢ**:
   Theo [`.scratch/client-pseudo-op-code/opcode_14.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L78), SubOp `0x0A` (và `0x0B`, `0x0D..0x11`) là lệnh chuyển tiếp trạng thái S→C set `PlayerState+0xA096 = 1` (transition lock/busy) và xóa `PlayerState+0xA094 = 0` (waiting server).
2. **`0x1A` (`handle_pc_talk` stub trong Rust) — ĐÃ ĐẶC TẢ**:
   Theo [`.scratch/client-pseudo-op-code/opcode_1a.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_1a.md#L10-L39), MainOp `0x1A` là kênh **1 chiều S→C đồng bộ tiền tệ & bộ đếm tài nguyên** (`0x01/0x02` Gold, `0x03` Gold full, `0x04` Gold + Shop Points, `0x05/0x06` Shop Points, `0x07` flag toast, `0x08/0x09/0x0A` Thủy binh / Tài bảo / Đạn dược). Trong `aLogin.exe`, `SendCommand` (`FUN_0077f414`) có `case 0x1a: break;` rỗng (`0/110` điểm gọi C→S). Do đó `npc_event::handle_pc_talk` trong Rust là giả định cũ sai hướng (nhầm schema payload S→C thành C→S talk selector).

---

## 2. Raw Codes Chưa Giải Mã trong `Data/eve.emg` (`3,901` scenes)

Số lần xuất hiện đo trực tiếp từ lần parse đầy đủ toàn bộ `Data/eve.emg` (`69,404` `EveCondition`, `150,098` non-empty `EveResult`, `22,833` `EveFightData`, `45,383` `fight_events`, `45,773` `win_conds`):

### 2.1 `EveCondition.condition_class` (`8 / 17` mã chưa ánh xạ)

| `cc` (u8) | Số lần xuất hiện | Mức ưu tiên nghiên cứu | Ví dụ / Ghi chú |
|---:|---:|---|---|
| `3` | `807` | Cao | Ví dụ `pstyle=4` tại scene `10951, eve=33, cond=1` |
| `5` | `181` | Trung bình | — |
| `13` | `110` | Trung bình | — |
| `11` | `77` | Trung bình | — |
| `4` | `45` | Thấp | — |
| `16` | `42` | Thấp | — |
| `17` | `16` | Thấp | — |
| `15` | `4` | Thấp | — |

**Biến thể `p_style` / `condition_ops` chưa xác minh trên các `condition_class` đã biết:**
- `condition_class = 2` (`QuestStep`): `p_style ∈ {4, 5, 6, 7, 9, 10}` (thực tế ngoài `1, 2, 3`).
- `condition_class = 9` (`FollowPet`): `p_style ∈ {2, 3}`.
- `condition_class = 12` (`SceneEventCount`): `p_style ∈ {2, 3, 4, 5, 6}` và `param != 0`.
- `condition_ops = 142` (`1` bản ghi duy nhất trong toàn bộ `Data/eve.emg`).

### 2.2 `EveResult.result_type` & `result_class` chưa giải mã

| Nhóm | Mã | Số lần xuất hiện | Ghi chú |
|---|---|---:|---|
| `result_type` | `7` | `691` | Chủ yếu đi cùng `rc = 7 (Player)` — không được xử lý trong `FUN_005EB530` |
| `result_type` | `8` | `351` | Chỉ server xử lý — chưa đối chiếu ngữ nghĩa với source/capture |
| `result_type` | `10` | `98` | Chỉ server xử lý |
| `result_type` | `4` (`rc != 3` hoặc `result_value` ngoài dạng XY 4 chữ số cuối) | `210` | `opcode_14.md` §4.1 đã xác minh `rt=4, rc=3` đọc 4 ký tự cuối làm tọa độ ô; cần kiểm tra tất cả `210` bản ghi xem có biến thể `rc != 3` |
| `result_class` | `5` | `430` | Xuất hiện tần suất trung bình, chưa map enum |
| `result_class` | `11` | `35` | — |
| `result_class` | `12` | `21` | — |
| `result_class` | `6`, `10` | `9` mỗi loại | — |
| `result_class` | `0`, `9`, `13`, `15..22`, `80` | `1` mỗi loại (`0` cần rà ngữ cảnh no-op vs special) | Tần suất cực hiếm |
| `result_class` | `3` (`NpcTeam`), `7` (`Player`) | Phổ biến | Chi tiết toàn bộ biến thể `p_style ∈ {1,3,4,5}` của `NpcTeam` và `param >= 2` của `Player` chưa map hết từng nhánh |

### 2.3 `EveFightData` & `.ini` Legacy Fields

- `fight_type`: `16` giá trị phân biệt trong `22,833` trận chiến (`1: 22285`, `2: 157`, `34: 140`, `7: 88`, `32: 52`, v.v.).
- `link_to_result`: Ý nghĩa rẽ nhánh kết quả sau trận của trường này.
- `fight_events` (`45,383` bản ghi) & `win_conds` (`45,773` bản ghi × `13B`): Đã xác định kích thước bản ghi nhị phân chuẩn, chưa giải mã cấu trúc các trường bên trong (`turn` trigger, spawn quân tiếp viện, điều kiện thắng đặc biệt ngoài tiêu diệt toàn bộ).
- `[TEAMDEF] Diahinh=628`: Ánh xạ chính xác ID địa hình `.ini` sang ID địa hình/bối cảnh chiến đấu trên server PC.

---

## 3. Các nhánh Helper/Callback Client Chưa Phục Hồi Body trong `aLogin.exe`

Ghi nhận từ `client_pseudo_c/` và [`.scratch/client-pseudo-op-code/opcode_14.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L102-L115):

1. `0x14` `FUN_005EB530` tại `result_type == 3`: Giá trị thanh ghi `CL` chính xác truyền vào `SendCommand(0x0B)` khi kích hoạt trận chiến.
2. `0x14` `result_type == 6` (Surface): Các callback nút chọn ngoài `TInterForm` (`0x005eecf4`, `0x005eecbc`, `0x005eee28`, `0x005eed2c`) ghi giá trị gì vào `0xa104` để gửi `C→S 0x14 09`.
3. `0x14` S→C 4 helper sự kiện phụ chưa phục hồi body: `0x005F16F4` (`0x2A`), `0x007485BC` (`0x32`), `0x00748860` (`0x33`), `0x0074E82C` (`0x38 sel=2`), và 2 hàm con `0x0071E288` / `0x0071F9EC` (`0x2C`).
4. `0x18` S→C `case 8 action == 2` (`FUN_00729a88` L100+).

---

## 4. Phương pháp Kiểm chứng đề xuất cho phiên nghiên cứu tiếp theo

1. **Ưu tiên theo tần suất thực tế**: Trích xuất các quest đại diện trong `Data/eve.emg` cho `cc = 3` (`807`x), `rt = 7` (`691`x), `rc = 5` (`430`x), `rt = 8` (`351`x), xem thoại NPC & điều kiện xung quanh để xác định bối cảnh (ví dụ: giới tính, nghề chuyển sinh, quân đoàn, thời gian thực, trạng thái map).
2. **Đối chiếu nguồn tham chiếu / capture**: So sánh trực tiếp với mã nguồn server tham chiếu (như `TS_Server_Bear` nếu được cung cấp) hoặc bắt gói tin thực tế từ client trên các nhiệm vụ mục tiêu.
3. **Giải mã Ghidra bổ sung**: Dump ASM/C tại các địa chỉ callback `0x005eecf4`..`0x005eed2c`, `FUN_005eb530` (`L681`), và các helper `0x14` chưa có body nếu phát sinh sự kiện map liên quan.
