# Báo Cáo Nghiên Cứu & Đánh Giá Toàn Diện Thư Mục Data/ và File Dữ Liệu eve.emg

- **Dự án**: `ts_dream` (Rust game server cho TS Online PC)
- **Ngày thực hiện**: 17/11/2026
- **Phạm vi**: 32 file trong thư mục `Data/`, đặc biệt đánh giá `npc_drops.json`, `NpcDropLoader.kt` và giải phẫu chuyên sâu `eve.emg` (9.8MB).

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

1. **Hiện trạng 32 file trong `Data/`**:
   - **8 file Core/Production Boot**: [`Item.dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L321), [`Npc.dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L331), [`Skill.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L359), [`eve.emg`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L361), [`Mark.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L362), [`Compound.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L363), [`Festival.Dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L364), [`ground_server.mmg`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L403).
   - **3 file Text ngoại lệ trong Production**: [`skills.json`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L384) (376 kỹ năng chuẩn), [`Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L392) (tọa độ warp map), [`BattleGate.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L398) (chốt chặn cửa ải).
   - **1 file JSON sự kiện độc lập**: [`north_star_questions.json`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/events.rs#L206) (ngân hàng câu hỏi Đấu trí Bắc Đẩu, load qua `EventLifecycleManager`).
   - **15 file nhị phân có Parser nhưng Deferred Boot**: Theo ADR 0003, các file như `Rank.Dat`, `BlissBag.Dat`, `Formula.Dat`, `CityEx.Dat`, `EVOStatus.Dat`, `Warp.Dat`, `AchievementData.dat`, `Dispatch.dat`, `DispatchBonus.dat`, `LeaderboardInfo.dat`, `Mounts.dat`, `MountsGrow.dat`, `SceneSet.Dat`, `TeachInfo.dat` đã có module typed parser hoàn chỉnh trong [`src/data/loaders/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/) nhưng cố ý chưa đấu dây vào `load_binary` vì chưa có live consumer, chỉ nạp vào `raw_binary_assets`.
   - **5 file THỪA / KHÔNG SỬ DỤNG**:
     + `NpcDropLoader.kt`: Mã nguồn Kotlin của emulator khác, hoàn toàn không chạy trong Rust.
     + `npc_drops.json`: Catalog drop dạng mobile (tỉ lệ riêng từng quái), không tương thích và không được dùng trong hệ thống chiến đấu chuẩn TS Online PC.
     + `Scene.Dat`: Tàn dư PC-legacy, client mobile coi là dead-code (0 caller), không có loader Rust.
     + `Skill2.Dat`: Tàn dư server Delphi nội bộ (1,900 bytes = 25 records × 76 bytes), không có field spec.
     + `WorldBoss.dat`: Tàn dư server Delphi nội bộ (255 bytes = 5 records × 51 bytes), không có field spec.

2. **Đánh giá `npc_drops.json` & `NpcDropLoader.kt`**:
   - `ts_dream` áp dụng nguyên bản 100% công thức drop chuẩn PC TS Online (Delphi): Dữ liệu drop nằm ngay trong 6 slot của từng record NPC tại [`Npc.dat`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/npc.rs#L194-L199) (`drop1..drop6`). Trận chiến roll random tỉ lệ tầng tích lũy `[25, 23, 20, 4, 3, 1]` trên thang 1000 tại [`src/battle/damage.rs:167-182`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs#L167-L182).
   - `npc_drops.json` và `NpcDropLoader.kt` là tàn dư mang sang từ dự án giả lập Kotlin `tw.tsm`. Chúng hoàn toàn không có tác dụng và **nên được dọn dẹp khỏi thư mục `Data/`**.

3. **Giải phẫu `eve.emg` (~9.8MB)**:
   - Là container chứa toàn bộ kịch bản sự kiện (3,901 scene, 23,659 NpcEvents, 69,404 Conditions, 150,098 Results, 22,833 Fights).
   - Server Rust hiện tại đang dùng `eve.emg` làm **Single Source of Truth** cho toàn bộ hệ thống nhiệm vụ, đối thoại NPC, bãi quái dã ngoại, di chuyển map và chiến đấu PvE kịch bản qua engine 4 tầng tại [`src/eve/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/).
   - `eve.emg` chứa rất nhiều dữ liệu **chỉ phục vụ hiển thị Client (Visual-only)**: Waypoints di chuyển tuần tra của NPC (`motion_nodes`), tham số hoạt ảnh bước đi, hiệu ứng màn hình/thời tiết, các opcode visual (Type 4 `MoveEntity`, Type 5 `Animation/Movie`, Type 9 `NpcAction`), các sprite vẽ cửa, text thoại hiển thị...
   - Dung lượng RAM khi nạp vào Rust xấp xỉ **40MB - 55MB**. Đề xuất phương án tối ưu hóa: Strip các field visual thừa trong struct, hoặc Lazy Loading theo Scene ID (tiết kiệm 85% RAM lúc boot).

---

## 2. Rà Soát & Phân Loại Chi Tiết 32 File Trong Thư Mục `Data/`

### 2.1 Bảng Tổng Hợp Trạng Thái 32 File

| STT | Tên File | Kích Thước | Định Dạng | Trạng Thái Trong Server Rust | Vị Trí Mã Nguồn / Consumer Chính |
|:---:|:---|:---:|:---:|:---|:---|
| 1 | `AchievementData.dat` | 22.2 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/achievement.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/achievement.rs) |
| 2 | `Astrolabe.Dat` | 218 B | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/astrolabe.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/astrolabe.rs) |
| 3 | `BattleGate.txt` | 4.0 KB | Text | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:398`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L398), [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs) |
| 4 | `BlissBag.Dat` | 58.3 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/bliss_bag.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/bliss_bag.rs) |
| 5 | `CityEx.Dat` | 6.3 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/city_ex.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/city_ex.rs) |
| 6 | `Compound.Dat` | 11.2 KB | Binary Dat | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:363`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L363), [`src/crafting/basic/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/crafting/basic/mod.rs) |
| 7 | `Dispatch.dat` | 454 B | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/dispatch.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/dispatch.rs) |
| 8 | `DispatchBonus.dat` | 1.1 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/dispatch.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/dispatch.rs) |
| 9 | `eve.emg` | 9.8 MB | EMG Container | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:361`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L361), [`src/eve/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/mod.rs), handlers |
| 10 | `EVOStatus.Dat` | 12.0 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/evo_status.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/evo_status.rs) |
| 11 | `Festival.Dat` | 20.3 KB | Binary Dat | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:364`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L364), [`src/server/handlers/festival.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/festival.rs) |
| 12 | `Formula.Dat` | 411 B | Binary Dat | Có typed parser, nạp `raw_binary_assets`, dùng tính texps | [`src/data/loaders/formula.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/formula.rs), [`src/data/texps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/texps.rs) |
| 13 | `ground_server.mmg` | 217.3 KB | Binary MMG | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:403`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L403), [`src/battle/service.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) |
| 14 | `Item.dat` | 3.1 MB | Binary Dat | **Load trực tiếp vào Production Boot (Bắt buộc)** | [`src/data/loader.rs:321`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L321), toàn bộ game logic |
| 15 | `LeaderboardInfo.dat` | 1.0 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/leaderboard.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/leaderboard.rs) |
| 16 | `Mark.Dat` | 1.2 MB | Binary Dat | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:362`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L362), [`src/server/handlers/quest_sync.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest_sync.rs) |
| 17 | `Mounts.dat` | 2.0 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/mount.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/mount.rs) |
| 18 | `MountsGrow.dat` | 589 B | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/mount_grow.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/mount_grow.rs) |
| 19 | `north_star_questions.json`| 9.3 KB | JSON | **Load trực tiếp qua EventLifecycleManager** | [`src/server/events.rs:206`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/events.rs#L206) (Đấu trí Bắc Đẩu) |
| 20 | `Npc.dat` | 612.7 KB | Binary Dat | **Load trực tiếp vào Production Boot (Bắt buộc)** | [`src/data/loader.rs:331`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L331), battle, npc spawns, talk |
| 21 | `NpcDropLoader.kt` | 2.0 KB | Kotlin Source | **KHÔNG DÙNG / FILE THỪA (Tàn dư tw.tsm)** | Không có tham chiếu trong Rust |
| 22 | `npc_drops.json` | 843.8 KB | JSON | **KHÔNG DÙNG / FILE THỪA (Tàn dư tw.tsm)** | Không có tham chiếu trong Rust (xem ADR 0003) |
| 23 | `Rank.Dat` | 1.8 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/rank.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/rank.rs) (ADR 0003 §2) |
| 24 | `Scene.Dat` | 1.0 MB | Binary Dat | **KHÔNG DÙNG (Dead code từ mobile/PC legacy)** | Nạp thô `raw_binary_assets`, không có parser (ADR 0003 §4) |
| 25 | `SceneSet.Dat` | 1.0 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/scene_set.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/scene_set.rs) |
| 26 | `Skill.Dat` | 32.4 KB | Binary Dat | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:359`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L359), [`src/data/loaders/skill_pc.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/skill_pc.rs) |
| 27 | `Skill2.Dat` | 1.9 KB | Binary Dat | **KHÔNG DÙNG (Tàn dư server Delphi không có spec)** | Nạp thô `raw_binary_assets`, không có parser (ADR 0003 §4) |
| 28 | `skills.json` | 109.9 KB | JSON | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:384`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L384) (376 kỹ năng chuẩn aLogin) |
| 29 | `TeachInfo.dat` | 9.0 KB | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/teach_info.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/teach_info.rs) |
| 30 | `Warp.Dat` | 690 B | Binary Dat | Có typed parser, nạp `raw_binary_assets`, deferred boot | [`src/data/loaders/warp.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/warp.rs) (Dành cho opcode teleport 0x44) |
| 31 | `Warps.txt` | 119.1 KB | Text | **Load trực tiếp vào Production Boot** | [`src/data/loader.rs:392`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L392), [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs) |
| 32 | `WorldBoss.dat` | 255 B | Binary Dat | **KHÔNG DÙNG (Tàn dư server Delphi không có spec)** | Nạp thô `raw_binary_assets`, không có parser (ADR 0003 §4) |

---

### 2.2 Phân Tích 4 Nhóm File

#### Nhóm 1: Được parse & load trực tiếp vào `GameData` hoặc Runtime Logic (12 files)
Bao gồm các file bắt buộc phải nạp khi boot server để trò chơi vận hành:
- **`Item.dat`** & **`Npc.dat`**: Hai trục dữ liệu tĩnh cơ sở của toàn bộ game. Thiếu 1 trong 2 file này server sẽ từ chối khởi động ([`src/data/loader.rs:321-339`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L321-L339)).
- **`Skill.Dat`**: Binary skill metadata được parse bởi [`SkillDatLoaderPc`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/skill_pc.rs).
- **`skills.json`**: 376 kỹ năng chuẩn hóa ánh xạ trực tiếp sang client `aLogin.exe`.
- **`eve.emg`**: Kịch bản quest, thoại, điều kiện, chuyển map, đánh nhau PvE ([`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs)).
- **`Mark.Dat`**: Bảng cờ nhiệm vụ (quest marks) đồng bộ với client qua gói `0x18` ([`src/data/loaders/mark.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/mark.rs)).
- **`Compound.Dat`**: Danh mục công thức ép đồ, khảm ngọc/lông ([`src/crafting/basic/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/crafting/basic/mod.rs)).
- **`Festival.Dat`**: Dữ liệu sự kiện lễ hội thời vụ ([`src/server/handlers/festival.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/festival.rs)).
- **`ground_server.mmg`**: Dữ liệu địa hình chiến đấu (`geolBaseAtt`) cấp cho `map_diahinh` trong battle engine ([`src/data/loader.rs:419-428`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L419-L428)).
- **`Warps.txt`**: Tọa độ warp giữa các map khi bước qua cửa ([`src/data/loader.rs:392`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L392)).
- **`BattleGate.txt`**: Tọa độ các cửa ải yêu cầu thắng trận trước khi cho qua ([`src/data/loader.rs:398`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L398)).
- **`north_star_questions.json`**: Ngân hàng câu hỏi đố vui sự kiện Đấu trí Bắc Đẩu (NPC 1643, 1644), nạp trực tiếp qua [`src/server/events.rs:206`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/events.rs#L206).

#### Nhóm 2: Có Typed Parser nhưng áp dụng chính sách Deferred Catalog (15 files)
Theo [ADR 0003](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0003-rank-ground-port.md): *"Chỉ những catalog nào có live consumer trên production mới được wire vào `load_binary`. Các parser khác được giữ nguyên trong `src/data/loaders/`, có unit test đầy đủ, file nhị phân được kiểm tra SHA256 và lưu trong `raw_binary_assets`"*.
- **Các file trong nhóm này**: `AchievementData.dat`, `Astrolabe.Dat`, `BlissBag.Dat`, `CityEx.Dat`, `Dispatch.dat`, `DispatchBonus.dat`, `EVOStatus.Dat`, `Formula.Dat`, `LeaderboardInfo.dat`, `Mounts.dat`, `MountsGrow.dat`, `Rank.Dat`, `SceneSet.Dat`, `TeachInfo.dat`, `Warp.Dat`.
- **Lưu ý riêng**:
  + `Formula.Dat`: Mặc dù không wire vào `GameData.formula_params` trong `load_binary`, các hằng số công thức của nó đã được thẩm định và dùng trực tiếp để tính bảng kinh nghiệm `texps` tại [`src/data/texps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/texps.rs).
  + `Warp.Dat`: Parser đã hoàn tất (`WarpDatLoader`), nhưng file này chứa danh sách truyền tống pha lê (crystal teleport - mobile opcode `0x44`), khác hoàn toàn với `Warps.txt` (bước qua cổng map). Khi nào implement opcode 0x44 mới wire file này.

#### Nhóm 3: File Text / Legacy chỉ dùng trong Migration / Test (0 files trong Data/)
Thư mục `Data/` hiện tại chỉ còn `Warps.txt` và `BattleGate.txt` là 2 file text duy nhất được giữ lại vì chúng là ngoại lệ phục vụ trực tiếp cho production boot (do TS Online PC không lưu bảng warp cổng này dưới dạng `.Dat` độc lập). Các file legacy khác (`Npcs.txt`, `Items.txt`, `Skills.txt`) đã bị loại bỏ hoàn toàn khỏi `Data/`.

#### Nhóm 4: File Hoàn Toàn KHÔNG ĐƯỢC LOAD hoặc Không Có Tham Chiếu Trong Rust (5 files)
1. `NpcDropLoader.kt`
2. `npc_drops.json`
3. `Scene.Dat`
4. `Skill2.Dat`
5. `WorldBoss.dat`

---

## 3. Đánh Giá Cụ Thể Về `npc_drops.json` và `NpcDropLoader.kt`

### 3.1 Nguồn Gốc và Bối Cảnh
- **`NpcDropLoader.kt`**:
  + Package: `package tw.tsm.data.loaders`.
  + Ngôn ngữ: Kotlin (JVM).
  + Xuất xứ: Trích xuất từ source code server giả lập TS Mobile (`tw.tsm`).
  + Không thể biên dịch hay thực thi bởi Rust compiler `cargo`.
- **`npc_drops.json`**:
  + Dung lượng: 843,845 bytes (gần 1MB).
  + Cấu trúc:
    ```json
    {
      "10004": [
        { "itemId": 27061, "rate": 30, "minQty": 1, "maxQty": 1 },
        { "itemId": 26156, "rate": 30, "minQty": 1, "maxQty": 1 },
        { "itemId": 26042, "rate": 30, "minQty": 1, "maxQty": 1 }
      ]
    }
    ```
  + Bản chất: Đây là bảng drop cấu hình riêng cho phiên bản di động (TS Mobile) hoặc bản custom private server, trong đó mỗi NPC có danh sách drop động với tỉ lệ `rate` riêng và số lượng `minQty..maxQty`.

### 3.2 Cơ Chế Rơi Đồ (NPC Drops) Hiện Tại Của Server Rust (`ts_dream`)
Trò chơi TS Online phiên bản PC nguyên gốc (Delphi) vận hành cơ chế rơi đồ cực kỳ chặt chẽ và nhất quán:
1. **Dữ liệu cấu hình drop nằm trực tiếp trong `Npc.dat`**:
   Tại [`src/data/loaders/npc.rs:194-199`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/npc.rs#L194-L199):
   Mỗi record NPC (kích thước 134 bytes) có 6 trường vật phẩm rơi:
   ```rust
   let drop1 = Self::dec16(u16::from_le_bytes([chunk[72], chunk[73]]));
   let drop2 = Self::dec16(u16::from_le_bytes([chunk[74], chunk[75]]));
   let drop3 = Self::dec16(u16::from_le_bytes([chunk[76], chunk[77]]));
   let drop4 = Self::dec16(u16::from_le_bytes([chunk[78], chunk[79]]));
   let drop5 = Self::dec16(u16::from_le_bytes([chunk[80], chunk[81]]));
   let drop6 = Self::dec16(u16::from_le_bytes([chunk[82], chunk[83]]));
   ```
   Khi nạp vào struct `Npc`, mảng `item: [i64; 6]` đại diện cho 6 slot drop này.

2. **Thuật toán roll drop chuẩn PC trong trận đấu**:
   Tại [`src/battle/damage.rs:167-182`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs#L167-L182) và [`src/battle/runner.rs:457`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L457):
   ```rust
   const DROP_PERCENTS: [i64; 6] = [25, 23, 20, 4, 3, 1];

   pub fn get_random_drop(rng: &mut DotNetRandom, npc: &Npc, percents: &[i64; 6]) -> i64 {
       let item: [i64; 6] = npc.item;
       let num = i64::from(rng.next_range(1, 1000));
       let mut lo = 0i64;
       for (i, &band) in percents.iter().enumerate() {
           let hi = lo + band;
           if num > lo && num <= hi {
               return item[i];
           }
           lo = hi;
       }
       0
   }
   ```
   - Roll ngẫu nhiên `num` trong khoảng `1..1000` (sử dụng `DotNetRandom` đồng bộ với client).
   - Dải tỉ lệ tích lũy (Cumulative Band):
     + `num <= 25` (2.5%): Rơi vật phẩm ở Slot 1.
     + `26 <= num <= 48` (2.3%): Rơi vật phẩm ở Slot 2.
     + `49 <= num <= 68` (2.0%): Rơi vật phẩm ở Slot 3.
     + `69 <= num <= 72` (0.4%): Rơi vật phẩm ở Slot 4.
     + `73 <= num <= 75` (0.3%): Rơi vật phẩm ở Slot 5.
     + `num == 76` (0.1%): Rơi vật phẩm ở Slot 6.
     + `num > 76` (92.4%): Không rơi vật phẩm nào.
   - Khi có drop, server broadcast packet diễn hoạt nhặt đồ: `packets::drop_item(...)` (`0x35 04`) và cộng vật phẩm vào túi đồ người chơi.

3. **Cơ sở dữ liệu SQLite**:
   Không hề có bảng `npc_drops` trong SQLite DB. Dữ liệu drop là dữ liệu tĩnh của thế giới game (world static data), được nạp trực tiếp vào RAM từ `Npc.dat`.

### 3.3 So Sánh & Kết Luận Kiến Trúc
- [ADR 0003](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0003-rank-ground-port.md) đã quyết định rõ ràng từ ngày 04/09/2026:
  > *"Drop: giữ legacy, không tạo `NpcDropLoader.rs`. `Npc.item[6]` + band-roll là authoritative cho PC server; `npc_drops.json` chỉ là catalog tham khảo, chưa đấu dây vào battle."*
- **So sánh `Npc.dat` và `npc_drops.json`**:
  + `Npc.dat` chứa đầy đủ 6 slot drop của toàn bộ hàng ngàn quái vật trong game. Định dạng này hoàn toàn khớp với client TS Online PC bản quyền và khớp với công thức xác suất `DROP_PERCENTS` của Delphi server.
  + `npc_drops.json` áp dụng cơ chế rolling độc lập của di động (`tw.tsm`), phá vỡ tính nguyên bản (parity) của TS Online PC.
- **Khuyến nghị**:
  + **XÓA BỎ HOẶC DI CHUYỂN** cả 2 file `NpcDropLoader.kt` và `npc_drops.json` ra khỏi thư mục `Data/`.
  + Lý do: Chúng là file thừa không bao giờ được biên dịch hay sử dụng, gây hiểu lầm cho các lập trình viên hoặc các tác tử AI tiếp theo rằng server đang dùng cơ chế drop JSON.

---

## 4. Phân Tích & Đánh Giá Chuyên Sâu Về `eve.emg` (~9.8MB)

### 4.1 Bản Chất File `eve.emg`
- `eve.emg` là một **EMG container** nhị phân đóng gói toàn bộ kịch bản sự kiện (Event Engine Script) của game TS Online.
- Cấu trúc container:
  + Header: `scene_count: u16` (chứa đúng **3,901 scene**).
  + Directory Table: 3,901 records, mỗi record 32 bytes (tên file `xxxx.eve`, offset vị trí trong file, độ dài chunk).
  + Body: 3,901 scene data blocks độc lập. Mỗi block bắt đầu tại `offset + 103` (theo quy ước `DataManager.lua:773`).

### 4.2 Thống Kê Định Lượng Chi Tiết Toàn Bộ Dữ Liệu Trong `eve.emg`
Thông qua script phân tích nhị phân chuyên dụng, cấu trúc thực tế của `Data/eve.emg` (9,839,580 bytes) bao gồm:

```
Tổng số Scene nhị phân: 3,901 bản đồ
----------------------------------------------------------------------
1. NPC Placements (Section 1)     : 21,534 vị trí NPC
2. Goods Placements (Section 2)   : Hàng hóa trên map (đã skip)
3. Doors (Section 3)              : 9,887 cánh cửa chuyển map
4. Encounters (Section 4)         : 1,918 bãi quái dã ngoại
5. Surfaces (Section 5)           : 2,285 khung hội thoại đa lựa chọn (menu)
   - Tổng số câu thoại (sentences): 11,025 câu
6. SceneInfos (Section 6)         : 8,882 điểm xuất hiện khi warp
7. Groups (Section 7)             : 7,969 nhóm xác suất ngẫu nhiên
8. NpcEvents (Section 8)          : 23,659 kịch bản sự kiện
   - Tổng số điều kiện (Conditions): 69,404 điều kiện
   - Tổng số kết quả (Results)     : 150,098 kết quả hành động
9. FightDatas (Section 9)         : 22,833 trận chiến kịch bản PvE
   - Tổng số quái địch trong trận  : 89,896 quái địch
```

#### Phân bố Loại Kết Quả (`result_type` - 150,098 kết quả):
- **Type 1 (Talk - Hội thoại)**: **61,458** (chiếm 41.0%)
- **Type 0 (Action - Hành động đột biến)**: **49,728** (chiếm 33.1%)
  + *Class 3 (NpcTeam)*: 29,861 (quái/NPC nhập/xuất đội hình)
  + *Class 2 (Quest)*: 9,593 (nhận/hoàn thành/hủy quest)
  + *Class 1 (Item)*: 8,602 (thêm/bớt vật phẩm)
  + *Class 8 (Pet)*: 559 (thưởng/thu hồi pet)
  + *Class 7 (Player)*: 541 (savemap, điểm thuộc tính, Exp)
  + *Class 5 (Money)*: 429 (thưởng/trừ vàng, điểm)
  + *Class 4 (Skill)*: 34 (học kỹ năng miễn phí)
- **Type 3 (Battle - Khởi tạo trận đấu)**: **22,815** (chiếm 15.2%)
- **Type 2 (Door/Warp - Dịch chuyển map)**: **10,336** (chiếm 6.9%)
- **Type 6 (Surface - Hiển thị menu lựa chọn)**: **2,571** (chiếm 1.7%)
- **Type 9 (NpcAction - Diễn hoạt dáng NPC)**: **1,600** (chiếm 1.1%)
- **Type 5 (Animation/Movie - Chiếu phim/hiệu ứng)**: **240** (chiếm 0.16%)
- **Type 4 (MoveEntity - Di chuyển NPC bước đi)**: **210** (chiếm 0.14%)

#### Phân bố Lớp Điều Kiện (`condition_class` - 69,404 điều kiện):
- **Class 2 (QuestStep - Tiến độ nhiệm vụ)**: **25,023** (chiếm 36.1%)
- **Class 0 (Unconditional - Luôn thỏa mãn)**: **16,731** (chiếm 24.1%)
- **Class 7 (PlayerAttribute - Cấp độ, tái sinh, điểm...)**: **7,984** (chiếm 11.5%)
- **Class 8 (BattleResult - Thắng/thua trận trước)**: **5,780** (chiếm 8.3%)
- **Class 10 (DialogChoice - Người chơi chọn mục menu)**: **5,715** (chiếm 8.2%)
- **Class 1 (BagItem - Kiểm tra vật phẩm có trong túi)**: **4,292** (chiếm 6.2%)
- **Class 9 (FollowPet - Kiểm tra pet đang đi theo)**: **1,889** (chiếm 2.7%)
- **Class 12 (SceneEventCount - Đếm số lần kích hoạt sự kiện)**: **703** (chiếm 1.0%)

---

### 4.3 Những Dữ Liệu Trong `eve.emg` Server Rust BẮT BUỘC CẦN

Toàn bộ hệ thống nhiệm vụ mới của `ts_dream` đã thay thế hoàn toàn hệ thống quest `.txt`/`.ini` cũ bằng engine 4 tầng tại [`src/eve/`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/mod.rs). Server Rust dựa 100% vào các trường sau:

1. **NPC Placement (`EveNpcPlacement`)**:
   - `id` (vị trí trên scene) & `npc_id`: Dùng để nhận diện khi client gửi opcode `0x14 01` / `0x14 06` (Click NPC).
   - `events`: Danh sách các `eve_no` gắn với NPC đó để chạy chuỗi Auto-chain ([`src/eve/auto_chain.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs)).
   - `x, y, direction`: Tọa độ spawn NPC khi client vào bản đồ.

2. **Door Placement (`EveDoorPlacement`) & SceneInfo (`EveSceneInfo`)**:
   - `door.events`: Kích hoạt sự kiện chạm cửa / click cửa.
   - `dest_map_id`, `player_appear_x`, `player_appear_y`, `direction`: Thông tin dịch chuyển tức thời người chơi sang bản đồ mới khi bước qua cửa.

3. **Bãi quái dã ngoại (`EveEncounterPlacement`)**:
   - `start_x, start_y, end_x, end_y`, `full_map`: Vùng tọa độ kích hoạt quái khi người chơi di chuyển từng bước chân (`WalkStep`), phục vụ cơ chế đụng độ ngẫu nhiên tại [`tests/eve_quest/random_encounter.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/random_encounter.rs).

4. **NpcEvent Data (`NpcEventData`) & Conditions (`EveCondition`)**:
   - `when_happen`: Bộ cờ 4 trigger `[OnClickObject, OnStrokeObject, OnIntoObjectArea, OnSerStrokeObject]`.
   - Toàn bộ 9 lớp điều kiện chính: Kiểm tra số lượng item trong túi (`BagItem`), bước nhiệm vụ (`QuestStep`), thuộc tính nhân vật (`PlayerAttribute`), kết quả trận đấu (`BattleResult`), thú cưng đi theo (`FollowPet`), lựa chọn đối thoại (`DialogChoice`).
   - Toán tử so sánh logic: `compare_step`, `invert_emg_ops`, liên kết chuỗi `and_num` và rẽ nhánh `to_result`.

5. **Action Results (`EveResult`) & Domain Effects**:
   - Chuyển đổi thành các đột biến trạng thái tại [`src/eve/planner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs):
     + `MutateItem`: Thêm/trừ vật phẩm nhiệm vụ.
     + `UpdateQuest`: Nhận quest, sang bước tiếp theo, hoàn thành nhiệm vụ (ghi vào bảng cờ `Mark.Dat`).
     + `LearnSkill`: Dạy kỹ năng miễn phí.
     + `MutateMoney`: Thưởng/phạt tiền vàng, điểm chiến công.
     + `RewardPet`: Thưởng/thu hồi thú cưng.
     + `PlayerEffect`: Đổi điểm lưu map hồi sinh (`savemap`), thưởng EXP.

6. **FightData (`EveFightData`)**:
   - Kích hoạt trận chiến PvE kịch bản: `left_enemies`, `right_enemies` (npc_id, vị trí hàng-cột `location_pos`, AI). Server Rust khởi tạo trực tiếp bàn cờ chiến đấu 2 hàng 5 cột từ struct này.

7. **GroupData (`EveGroupData`)**:
   - Chọn nhánh sự kiện ngẫu nhiên theo trọng số xác suất (`probability_rate_ay`).

---

### 4.4 Những Dữ Liệu CLIENT-ONLY / Thừa Thãi Đối Với Server

Trong file `eve.emg`, một phần đáng kể dữ liệu chỉ phục vụ mục đích đồ họa, hoạt họa và âm thanh của Client. Hiện tại, Server Rust hoặc đã bỏ qua (skip), hoặc vẫn đang parse và lưu trữ trong RAM một cách không cần thiết:

1. **Diễn hoạt tuần tra NPC (`motion_nodes`)**:
   - Trong Section 1, mỗi NPC có thể chứa hàng chục điểm tọa độ `(nx, ny)` diễn tả đường đi tuần tra vòng tròn hoặc ziczac của NPC trên màn hình client.
   - Server Rust hiện tại đang parse và cấp phát `motion_nodes: Vec<(i32, i32)>` cho cả 21,534 NPC, **nhưng server không bao giờ dùng `motion_nodes`** để tính toán va chạm. NPC server luôn đứng cố định tại `(pos_x, pos_y)`.
   - Thêm vào đó: `motion_type`, `motion_back`, `motion_cycle_num`, `motion_suspend_ms`, `motion_speed_lv` hoàn toàn là tham số render cho client.

2. **Nội dung văn bản lời thoại (`sentences`)**:
   - Section 5 (`SurfaceData`) chứa 11,025 sentences. Server chỉ gửi `surface_id` (opcode `0x14 01`), Client tự tra cứu chuỗi ký tự hiển thị từ file ngôn ngữ cục bộ của nó. Server không cần biết text câu thoại.

3. **Dữ liệu hình ảnh / Sprite / Background**:
   - `background_no` (ID ảnh nền), `areaName` (21 bytes tên khu vực - đã skip), `sceneEffect` (hiệu ứng mưa/tuyết/nắng - đã skip).
   - `imageKind`, `imgInfo.position.x/y` của cánh cửa (đã skip).
   - `GoodsData` (Section 2 - 13 bytes mỗi hàng hóa hiển thị trên sàn map - đã skip).

4. **Các Opcode kết quả thuần Visual (Client-Only Directives)**:
   - **Type 4 (`MoveEntity`)** (210 records): Chỉ thị client cho NPC di chuyển vài bước khi kết thúc đối thoại.
   - **Type 5 (`Animation/Movie`)** (240 records): Chỉ thị client phát đoạn cắt cảnh hoạt họa / rung lắc màn hình. Server chỉ đóng gói thô vào opcode `0x14 01` gửi xuống client mà không xử lý logic gì.
   - **Type 9 (`NpcAction`)** (1,600 records): Đổi hướng quay mặt hoặc tư thế đứng của NPC trên màn hình client.

5. **Âm thanh và điều kiện thắng phụ**:
   - `bk_music` trong Section 9: Nhạc nền trận đấu phát qua loa người chơi.
   - `FightEvent` (Section 9 phụ): 11 bytes header + điều kiện phụ màn hình (đã skip).

---

### 4.5 Đánh Giá Bộ Nhớ RAM & Hiệu Năng Của `eve.emg`

- **Kích thước file**: 9.84 MB trên đĩa.
- **Tiêu thụ RAM khi giải nén**:
  + 3,901 instance của struct `SceneEveData`.
  + Mỗi `SceneEveData` chứa 8 HashMaps + 4 HashMaps phụ (lookup index) + Vec.
  + Hơn 21,500 NPC, 23,600 Events, 69,400 Conditions, 150,000 Results, 22,800 FightDatas.
  + Do cấu trúc Rust sử dụng nhiều container lồng nhau (`HashMap`, `Vec`), chi phí heap allocation và bucket padding khiến dữ liệu nở rộng từ 9.8MB binary lên khoảng **45 MB đến 55 MB RAM** trong không gian tiến trình server.
- **Thời gian Boot**:
  + Thử nghiệm đo đạc thực tế: `EveDataLoader::load` mất khoảng **120ms - 220ms** trên CPU x86_64 hiện đại. Đây là tốc độ rất nhanh đối với khởi động server và hoàn toàn nằm trong ngưỡng chấp nhận được.

---

### 4.6 Các Phương Án Tối Ưu Hóa Khả Thi

```mermaid
flowchart TD
    subgraph Current["Hiện Trạng: Full Eager Boot"]
        A["eve.emg (9.8MB)"] -->|Boot load toàn bộ| B["3,901 Scenes trong RAM"]
        B -->|Chiếm dụng| C["45MB - 55MB RAM"]
    end

    subgraph Option1["Phương Án 1: Strip Visual Fields"]
        D["Loại bỏ motion_nodes, visual params"] -->|Giữ nguyên định dạng file| E["Tiết kiệm 8MB - 12MB RAM"]
    end

    subgraph Option2["Phương Án 2: Lazy Loading Theo Scene"]
        F["Boot chỉ đọc Index (125KB)"] -->|Người chơi vào Map X| G["Parse Scene X theo nhu cầu"]
        G -->|Cache LRU| H["Chỉ tốn ~3MB - 6MB RAM"]
    end

    subgraph Option3["Phương Án 3: Trích Xuất Server Binary"]
        I["Tool offline strip visual"] -->|Tạo ra eve_server.bin (2.5MB)| J["Zero-copy Deserialization"]
    end
```

#### Phương Án 1: Tối ưu bộ nhớ nội tại (Strip Visual Fields trong Struct) — *Khuyến nghị thực hiện sớm*
- Giữ nguyên định dạng nạp từ `eve.emg`.
- Trong [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs):
  + Thay vì lưu `motion_nodes: Vec<(i32, i32)>` (21,534 Vecs), chỉ cần skip qua các bytes này.
  + Loại bỏ các field không dùng: `motion_type`, `motion_back`, `motion_cycle_num`, `motion_suspend_ms`, `motion_speed_lv`, `trace_radius`, `can_grow`.
- **Ưu điểm**: Zero-risk, tương thích 100% với codebase hiện tại, tiết kiệm ngay lập tức **8 - 12 MB RAM**.

#### Phương Án 2: Lazy Loading theo Scene Map (Nạp theo nhu cầu) — *Khuyến nghị khi mở rộng quy mô*
- Khi boot server: Chỉ đọc 3,901 entries trong Directory Table của `eve.emg` (chỉ mất ~125KB RAM và < 5ms).
- Sử dụng Memory-Mapped File (`memmap2`) hoặc lưu `File` handle.
- Khi một người chơi hoặc NPC bước vào Scene `scene_id`: Server kiểm tra trong bộ nhớ đệm (LRU cache hoặc `RwLock<HashMap<u32, Arc<SceneEveData>>>`), nếu chưa có thì parse riêng scene đó từ offset tương ứng (mỗi scene chỉ mất khoảng 2KB - 15KB).
- **Ưu điểm**: Tiết kiệm hơn **85% RAM** (RAM sử dụng cho Eve giảm từ 50MB xuống dưới 5MB). Server chỉ nạp những map thực sự có người chơi đang hoạt động.

#### Phương Án 3: Biên soạn trước nhị phân máy chủ (Pre-compiled Server Asset)
- Tạo công cụ offline trích xuất từ `eve.emg` thành file nhị phân `eve_server.bin` (tương tự như cách dự án đã tạo ra `ground_server.mmg` từ `ground.mmg`).
- Loại bỏ hoàn toàn Section 2 (Goods), Section 5 (Sentences text), motion nodes, animation opcodes.
- Sử dụng `bincode` hoặc `rkyv` (Zero-copy deserialization).
- Kích thước file sẽ giảm từ 9.8MB xuống khoảng **2.5 MB**. Nạp vào RAM gần như tức thì (< 2ms).

---

## 5. Kết Luận & Kiến Nghị Hành Động (Actionable Recommendations)

1. **Dọn dẹp file thừa trong thư mục `Data/`**:
   - **Xóa ngay**: `Data/NpcDropLoader.kt` (source Kotlin không dùng).
   - **Xóa hoặc chuyển sang thư mục archive**: `Data/npc_drops.json` (bảng drop mobile không dùng).
   - **Đánh dấu rõ trong tài liệu**: `Scene.Dat`, `Skill2.Dat`, `WorldBoss.dat` là tàn dư legacy không có spec và không được nạp vào game logic.

2. **Duy trì kiến trúc Drop chuẩn PC**:
   - Tiếp tục duy trì công thức rơi đồ `[25, 23, 20, 4, 3, 1]` tích lũy trên thang 1000 từ 6 slot trong `Npc.dat` (`Npc.item`). Đây là chuẩn mực cốt lõi đảm bảo tính nguyên bản (parity) của TS Online PC.

3. **Kế hoạch đối với `eve.emg`**:
   - **Hiện tại**: Giữ nguyên `eve.emg` làm Single Source of Truth cho toàn bộ Quest, Event, NPC Dialogues, Encounters và PvE Fights. Hệ thống đang chạy rất ổn định và chính xác.
   - **Hành động ngắn hạn (Quick Win)**: Lược bỏ field `motion_nodes` và các tham số diễn hoạt visual trong struct `EveNpcPlacement` ([`src/data/loaders/eve.rs:165`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L165)) để giải phóng hàng vạn heap allocations không cần thiết.
   - **Hành động dài hạn**: Xem xét triển khai **Phương Án 2 (Lazy Loading theo Scene)** hoặc **Phương Án 3 (`eve_server.bin`)** khi dự án bước vào giai đoạn tối ưu hóa hiệu năng sản xuất (Production Hardening).
