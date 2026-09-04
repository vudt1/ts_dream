# ADR 0003 — Port Rank.Dat + ground.mmg tham khảo; defer Scene/Skill2/WorldBoss/NpcDrop JSON

- **Ngày**: 2026-09-04
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (grilling: drop-authority → legacy wins; rank key → index 1-based; Rank/Scene → reverse-engineer; Ground → full port tham khảo)

## Bối cảnh

`Data/` có 5 binary chưa có loader Rust: `Rank.Dat` (1849B), `Scene.Dat` (1MB),
`Skill2.Dat` (1900B), `WorldBoss.dat` (255B), `ground.mmg` (26.6MB — file lớn
nhất, đang bị bỏ qua hoàn toàn vì filter `loader.rs` chỉ nhận
`"dat"|"emg"|"mng"`). Thêm `Data/npc_drops.json` + `Data/NpcDropLoader.kt`
(mobile drop table) mới được bổ sung nhưng chưa có consumer Rust.

Battle Rust hiện roll drop từ `Npc.item[6]` + `DROP_PERCENTS`
(`src/battle/damage.rs::get_random_drop`, `runner.rs`), còn Kotlin mobile roll
từ `npc_drops.json` theo `rate/minQty/maxQty` (`BattleRewardCalc.kt:50-58`).

## Quyết định

1. **Drop: giữ legacy, không tạo `NpcDropLoader.rs`.** `Npc.item[6]` +
   band-roll là authoritative cho PC server; `npc_drops.json` chỉ là catalog
   tham khảo, chưa đấu dây vào battle.
2. **Port `Rank.Dat`** (`src/data/loaders/rank.rs::RankDatLoader` →>
   `GameData.rank_defs: HashMap<u16, RankDef>`). Spec chính xác từ
   `ts_mobile_client/Data/RankData.lua` + keys tại
   `DataManager.lua::OnLoadRankData`: `offset=9, xor1=0xFD, xor2=0xECEA,
   xor4=0x0B80F4B4` (cùng họ key `Item.dat`/`BlissBag.Dat`). Record 43B:
   string(20) + `honor: u16` + 4×(`kind: u8`, `value: i32`); rec0 dummy-zero
   skip qua empty-name guard. Key = **index 1-based** (parity với counter vòng
   lặp Lua `rankDatas[id]`). Decode verify: honor 15→2000, kinds
   207/208/211/214 (trùng attr kinds của item). Consumer tương lai:
   `RankData.GetAttribute(honor, kind)` cộng EquipMaxHp/Sp (`Calculator.lua`).
3. **Port `ground.mmg` ở mức tham khảo, KHÔNG đấu vào production**
   (`src/data/loaders/ground.rs` mirror `GroundMmgLoader.kt` =
   `DataManager.lua::OnLoadMapData`: tail index `count×29B`, walk body lấy
   `geolBaseAtt: u8`; body lỗi skip, chỉ giữ entries `> 0`). Theo quyết định
   owner (2026-09-04): chưa chắc server có cần dữ liệu này → parser đứng độc
   lập, **không** thêm field vào `GameData`, **không** thêm nhánh vào
   `load_binary`, **không** sửa filter `"mmg"` trong inventory (production
   boot giữ nguyên, không đọc thêm 26MB). Khi consumer landing, gọi
   `GroundMmgLoader::load` trực tiếp.
4. **Defer có lý do, giữ trong `raw_binary_assets`:**
   - `Scene.Dat`: `SceneData.ReadSceneData` là dead code (0 caller) trong mobile
     client; `DataManager.lua` không load file này; layout 134B không chia hết
     1002454B. File PC-legacy không consumer.
   - `Skill2.Dat` / `WorldBoss.dat`: server-side Delphi only
     (`spec/TS_Server_OP_Code.md:190`: WorldBoss đi với `WBPrize/WBScorePrize/
     WBSumScorePrize` — các file này không tồn tại). Dù phát hiện pattern
     fixed-record (`1900=76×25`, `255=51×5`, rec0 dummy) nhưng không có field
     spec → không bịa struct.

## Hậu quả

- Tích cực: 20/24 binary typed → 21/24 (+ Rank 42 records); Ground có parser
  tham khảo đã test mà production boot không nặng thêm byte nào;
  `ground.mmg` vẫn ngoài inventory (tàng hình như trước — cố ý, tránh đọc
  26MB khi boot khi chưa có consumer); mọi quyết định defer đều có evidence
  ghi lại, port sau không phải mò lại.
- Tiêu cực: `rank_defs` chưa có consumer (cố ý, tránh scope creep vào
  stat pipeline).
- Rollback: xóa `rank.rs` + 3 dòng wire trong `loader.rs` là xong, không ảnh
  hưởng catalog cũ.
