# 04 — Eve.emg Container Parser and Script Models

**What to build:** Xây dựng bộ nạp `EveDataLoader` đọc tệp container nhị phân `Data/eve.emg` (~9.8 MB) nạp toàn bộ 3,800+ scene script và giải mã đầy đủ 9 section nhị phân (NpcData, DoorData, SurfaceData, GroupData, NpcEventData, FightData, GoodsData, MineData, SceneInfoData).

**Blocked by:** 03 — Binary Dat Reader and Data Loaders

**Status:** completed

- [x] `EveDataLoader` đọc chính xác Scene Directory (32 bytes/entry) để xác định danh sách ~3,823 scene ID và offset tương ứng.
- [x] Parser đọc khối dữ liệu scene (+103 bytes header offset) và trích xuất đầy đủ `NpcPlacement`, `DoorPlacement`, `SceneInfo`, `GroupData`, `NpcEventData`, và `FightData`.
- [x] Định nghĩa các cấu trúc dữ liệu miền: `EveCondition` (với 15 condition classes), `EveResult` (với các result types), `EveFightData`.
- [x] Tích hợp `Arc<HashMap<u32, SceneEveData>>` vào `GameDataManager` nạp một lần khi boot server trong thời gian < 100ms.
- [x] Unit tests kiểm tra parser trên các scene bản đồ lớn (ví dụ: Tân thủ thôn 10801, Trác Quận 12001).
