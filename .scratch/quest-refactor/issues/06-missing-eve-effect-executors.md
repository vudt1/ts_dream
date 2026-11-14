# 06: Bổ sung Action Executors (`NpcTeam`, `RewardPet`, `EXP Level-Up`, `Gold/Point Cap`, `MoveEntity`, `Movie`, `NpcAction`, `savemap`)

**What to build:** Hoàn thiện toàn bộ các loại `DomainEffect` phổ biến còn thiếu trong kịch bản Eve để các nhiệm vụ trao sủng vật (như Tống Nhân), thay đổi nhóm NPC, nhận EXP thăng cấp thật, cộng/trừ Kim hoặc Điểm giới hạn trần, lưu điểm hồi sinh `savemap`, di chuyển thực thể và phát phim/hoạt ảnh đều thực thi chính xác và đồng bộ đầy đủ gói tin cho client.

**Blocked by:** `05: Kiến trúc Pure Planner + EventRunner (Pre-Flight All-or-Nothing, AutoChain, Skill & FinishChain Write-Through)`

**Status:** resolved

- [x] Hiện thực `NpcTeam (rc=3)` (nhóm action phổ biến nhất trong `Data/eve.emg`), `RewardPet (rc=8)` (kiểm tra slot túi sủng vật + đồng bộ khung `0x14`/`0x19` tương ứng), `MoveEntity (rt=4)` (đã xác minh `210/210` bản ghi trong `eve.emg` đều có `rc == 3` và `>= 5` chữ số thập phân tách 4 chữ số cuối), `Movie (rt=5)`, và `NpcAction (rt=9)` theo đúng thông số nhị phân đã xác minh.
- [x] Hoàn thiện `Player (rc=7)` (loại bỏ lỗi fallthrough `param == 3`): khi cộng `texp` (`p_style = 4`) phải kiểm tra thăng cấp, tăng chỉ số, hồi phục HP/SP và phát đủ gói cập nhật chỉ số `0x08` + `0x14 0x2A 0x01 <exp_delta: u32_LE>`; hỗ trợ thiết lập và đồng bộ `savemap`.
- [x] Hiện thực `Gold/Points (rc=5)` áp dụng kiểm tra biên `GOLD_CAP` / `POINT_CAP` trong `Pre-Flight Probe` (từ chối All-or-Nothing nếu vượt trần hoặc trừ âm) và phát chuẩn gói `0x1A 01/02/04/05`.
- [x] Các `result_type` (`7, 8, 10`) hoặc `result_class` (`0` ngoài `rt=5` animation, `5, 6, 9..22, 80`) thuộc nhóm `UNKNOWN` (đã xác nhận 100% server-only, không có nhánh đọc trong `aLogin.exe`) bị bỏ qua an toàn có ghi log cảnh báo telemetry, không gây panic hoặc gửi frame `0x14` lỗi cho client.
- [x] Kiểm thử E2E qua `dispatcher::dispatch` cho chuỗi `12001` Tống Nhân (chọn menu `0x14 09` -> nhận võ tướng `14105`, cập nhật `NpcTeam`, `Player` + `Quest 10027`, write-through DB) và bài kiểm tra tăng `texp` gây lên cấp + lưu `savemap`, cùng negative test từ chối khi vượt `GOLD_CAP` hoặc đầy túi sủng vật (4 ô) mà không làm mất item/bước quest trước đó.
