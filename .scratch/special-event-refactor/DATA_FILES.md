# FILE DỮ LIỆU (.dat / .emg) — client đọc ở đâu, dùng để làm gì

> **Đã kiểm chứng:** danh sách dưới đây lấy từ **tên file trong chuỗi hằng của chính
> `aLogin.exe`** (đã decode VISCII) — tức là chỉ những file **client thực sự mở ra đọc**.
> Mỗi dòng kèm hàm loader tương ứng trong bundle.
>
> **Nguyên tắc:** agent nên làm việc với bản **đã giải mã** (`.tsv`) chứ không phải binary thô.
> Bản thô chỉ để đối chiếu byte.

---

## 1. File đã có trong bundle

| File (bundle) | Nguồn gốc | Kích thước | Loader client | Nội dung / tại sao quan trọng |
|---|---|---:|---|---|
| `data_files/client_dat/Festival.Dat` | `Data/Festival.Dat` | 20 KB | `FUN_0051E00C` | ★★★ Dữ liệu 7 lễ hội (Tết Đoan Ngọ, Phật Đản…). Đã giải mã → `FESTIVAL_DAT_FORMAT.md` + `festival_dat_decrypted/` |
| `data_files/client_dat/Npc.dat` | `Data/Npc.dat` | 613 KB | `FUN_00623F28` | ★★★ Danh bạ NPC (id ↔ tên). Đã giải mã → `data_npc_catalog.tsv` (6 659) + `data_event_npcs.tsv` (737) |
| `data_files/client_dat/Scene.Dat` | `Data/Scene.Dat` | 1.0 MB | `FUN_00535600` | ★★★ **ĐÃ GIẢI MÃ** (bản ghi 134 B: `mapId` u16 mã hóa + `kind` u8 + tên VISCII; khoá `(w ^ 0xEA6C) - 9`). 7 481 bản ghi / 3 503 map id → `data_map_catalog.tsv`, `data_mapid_to_name.tsv`, `data_event_maps.tsv`. Xem [`SCENE_DAT_FORMAT.md`](./SCENE_DAT_FORMAT.md) |
| `data_files/client_dat/Item.Dat` | `Data/Item.dat` | 3.0 MB | `FUN_0077492C` | ★★ DB vật phẩm (8 675 item). Repo đã giải mã → `Data/items.json`; bản lọc sự kiện → `data_event_items.tsv` (179) |
| `data_files/client_dat/Warp.Dat` | `Data/Warp.Dat` | 690 B | `FUN_005BF788` | ★ Danh sách cổng dịch chuyển (23 byte/record, XOR 2 khoá ở 2 block cuối) |
| `data_files/client_dat/SceneSet.Dat` | `Data/SceneSet.Dat` | 1 KB | `FUN_00525720` | ★ Bảng thuộc tính map (max player, giới hạn level…) |
| `data_files/client_dat/Rank.Dat` | `Data/Rank.Dat` | 1.8 KB | `FUN_004DF424` | ★ Bảng xếp hạng (dùng cho bảng xếp hạng lôi đài / đua thuyền) |
| `data_files/eve/*.tsv` | `Data/eve.emg` | 21 MB | `FUN_005F2528` | ★★★ **Dữ liệu script sự kiện/quest của client** (đã giải mã — xem §3) |

> `Data/eve.emg` gốc (9.8 MB) **không** copy vào bundle — bản giải mã TSV ở trên hữu ích hơn
> nhiều với agent, và file gốc vẫn nằm ở `Data/eve.emg` trong repo.

---

## 2. File client đọc nhưng **THIẾU** trong repo — nên bổ sung nếu bạn còn client gốc

| File | Loader client | Vai trò với sự kiện | Ưu tiên |
|---|---|---|:-:|
| **`Talk.Dat`** | `FUN_0061D778` | **Hội thoại NPC** — lời thoại mở đầu/nhắc nhiệm vụ của NPC thủ binh sự kiện (4906 Hổ Lao, 2149 Quản lý Lôi Đài, 1643 Bắc Tinh Quân…) | ★★★ |
| **`RG.Dat`** | `FUN_0077299C` | **Thang cược** (wager) — liên quan trực tiếp tiền cược lôi đài 5.000 / 10.000 lượng | ★★★ |
| **`GS.Dat`** | `FUN_00524DC0` | Bảng guild / arena (cần xem loader để xác nhận) | ★★ |
| `TsoTw.Dat`, `TsoCn.Dat` | `FUN_00535D0C` | Bảng chuỗi song ngữ (Trung/Việt) | ★ |
| `Suit.Dat`, `WeapBody.Dat` | `FUN_004DBE44` / `FUN_004CA580` | Trang bị (không trực tiếp sự kiện) | ○ |
| `dat\cuts.dat` | `FUN_00485508` | Dữ liệu cắt hình | ○ |
| `AllOrgEnsign.dat` | — | Quân hiệu bang hội (liên quan công thành) | ★ |
| `Data\Wem.MMG` | — | Ground tile (bổ sung cho `ground.mmg`) | ○ |

> ⚠ **KHÔNG cần** các file sau: chúng **không có** trong client này (đã kiểm tra bằng chuỗi
> hằng) — chúng là dữ liệu phía **server** hoặc của client thế hệ khác:
> `Dispatch.dat`, `DispatchBonus.dat`, `AchievementData.dat`, `LeaderboardInfo.dat`,
> `Mounts.dat`, `MountsGrow.dat`, `TeachInfo.dat`, `WorldBoss.dat`, `CityEx.Dat`,
> `EVOStatus.Dat`, `Astrolabe.Dat`, `items.json`, `npc_drops.json`, `skills.json`.

---

## 3. `Data/eve.emg` — dữ liệu script sự kiện/quest (đã giải mã)

Đây là **mảnh ghép quan trọng nhất** còn thiếu: trong TS Online, phần lớn "sự kiện đặc biệt"
được điều khiển bởi **quest + điều kiện Eve script**, không phải bằng opcode riêng.

* Bộ giải mã: `examples/dump_eve.rs` (thêm vào repo) — chạy `cargo run --example dump_eve`.
* Kết quả: **2 777 scene** (scene_id = **map id** thật của server: 10701, 10851, 12000, 12851,
  15000, 15510, 60011 … — trùng với `Data/Warps.txt` và `Data/BattleGate.txt`).
* Khối lượng: 110 083 dòng logic, trong đó **60 430 dòng có điều kiện `QuestStep`**.

| File | Nội dung | Cột chính |
|---|---|---|
| `eve_scene_index.tsv` | chỉ mục scene | `scene_id, npc, door, event, fight, surface, group, encounter, npc_ids` |
| `eve_npcs.tsv` | NPC đặt trong map | `scene_id, place_id, npc_id, x, y, close, direction, motion_*` |
| `eve_doors.tsv` | cổng dịch chuyển trong map | `scene_id, door_id, x, y` |
| **`eve_logic.tsv`** | ★ **bảng điều kiện → kết quả** (logic sự kiện/quest) | `scene_id, eve_no, when_happen, cond_no, cond_class, cond_param, param_style, ops, cond_value, sub_item, to_result, and_num, result_*` |
| `eve_fights.tsv` | trận PvE có kịch bản | `scene_id, eve_no, fight_type, link_to_result, limit, left/right (no:npc_id:pos)` |
| `eve_surface.tsv` | hộp thoại / lựa chọn | `scene_id, surface_id, sentence_count, option_*` |
| `eve_groups.tsv` | nhóm kết quả trọng số (random) | `scene_id, eve_no, event_no, condition_no, use_mode, members, rates` |
| `eve_encounters.tsv` | vùng gặp quái ngẫu nhiên | `scene_id, id, start_x, start_y, end_x, end_y, full_map` |
| `eve_full.jsonl` | dump JSON đầy đủ 1 scene/dòng (13 MB) | — |

### Ngữ nghĩa các giá trị `cond_class` (điều kiện)

| Giá trị | Tên | Ý nghĩa |
|---:|---|---|
| 0 | `Unconditional` | luôn đúng |
| 1 | `BagItem` | điều kiện theo vật phẩm trong túi (`cond_param` = item id) |
| 2 | `QuestStep` | **điều kiện theo bước nhiệm vụ** (`cond_param` = quest id, `cond_value` = bước) — cơ chế chính điều khiển sự kiện |
| 7 | `PlayerAttribute` | điều kiện theo thuộc tính người chơi |
| 8 | `BattleResult` | điều kiện theo kết quả trận đánh |
| 9 | `FollowPet` | điều kiện theo pet đang theo |
| 10 | `DialogChoice` | điều kiện theo lựa chọn trong hộp thoại |
| 12 | `SceneEventCount` | điều kiện theo số sự kiện đã xảy ra trong scene |
| 14 | `RoleCount` | điều kiện theo số người chơi trong nhóm |

### Ngữ nghĩa các giá trị `result_type` / `result_class`

| `result_type` | | `result_class` | |
|---|---|---|---|
| 0 `Action` | hành động | 1 `Item` | trao vật phẩm (`result_value` = item id) |
| 1 `Talk` | hộp thoại | 2 `Quest` | **đổi bước nhiệm vụ** (`result_value` = bước mới) |
| 2 `Door` | qua cổng | 3 `NpcTeam` | NPC/đội ngũ |
| 3 `Battle` | mở trận | 4 `Skill` | kỹ năng |
| 5 `Animation` | hiệu ứng | 7 `Player` | người chơi |
| 6 `Surface` | bảng lựa chọn | 8 `RewardPet` | thưởng pet |
| 9 `NpcAction` | NPC hành động | | |

---

## 3b. Ví dụ đã dò được: truy vết một sự kiện trong `eve_logic.tsv`

Bảng `eve_logic.tsv` cho phép **dò ngược** từ vật phẩm sự kiện → quest → map.
Kết quả đã trích sẵn ở `data_event_quests.tsv` (250 dòng) và bảng tổng hợp:

| Quest id | Scene (map id) | Vật phẩm liên quan | Sự kiện |
|---:|---|---|---|
| **10714**, **10715** | **12003** | 30014 "Thắng lệnh 1" (*"Khiêu chiến 1, dùng lệnh này lãnh thưởng"*) | ★ **Lôi Đài Khiêu Chiến** (map 12003) |
| **10250** | 13105, 13513 | 30012 "Công thành đồ S.Ph" (*"Công thành đồ sao phỏng"*) | Công thành chiến |
| 10072, 10074, 10078 | 11012, 11172 | — | sự kiện map 11012/11172 |
| 10092 | 15001 | — | sự kiện map 15001 |
| 10110 | 11131 | — | sự kiện map 11131 |
| 10166 | 12137 | — | sự kiện map 12137 |
| 10218 | 12854 | — | sự kiện map 12854 |
| **10392** | 57507, 57515, 57528, 57535 | — | sự kiện 4 map cùng họ 575xx |
| 10463 | 12022 | — | sự kiện map 12022 |
| 10652 | 18990 | — | sự kiện map 18990 |
| 10772 | 13042 | — | sự kiện map 13042 |
| 10784 | 21043 | — | sự kiện map 21043 |
| 10786 | 10995, 10996, 12001 | — | sự kiện 3 map |
| 10824, 10874, 10904, 11464, 11614, 11618, 11696, 11706, 11758, 11759, 11780, 11852, 11882, 11912, 11988, 11996, 12072, 12280 | (xem `data_event_quests.tsv`) | — | sự kiện còn lại |

**Mẫu dòng để đọc** (cột theo thứ tự trong header):

```
12003  24  0  1  QuestStep  10714  1  5  2  0  1  2  18  2  Action  Item  30014  1  -1  0
 └scene └eve  └when └cond │      └quest└ps└ops└val └sub └to └grp └no │      │      │       │  │  └─ result_value = item id
                          └ điều kiện: quest 10714, ops=5 (>=), value=2 (bước ≥ 2) ─┘        └ parameter_style
```

→ Nghĩa là: **khi quest 10714 đạt bước ≥ 2 thì client trao item 30014 ("Thắng lệnh 1")**.

**Hãy làm tiếp:** với mỗi quest trong bảng trên, dựng chuỗi đầy đủ
`quest → bước → điều kiện → kết quả` (tra cả `eve_logic.tsv` **không** lọc theo item),
rồi nối với tên map (xem §4) để biết sự kiện diễn ra ở đâu và theo trình tự nào.

## 4. Bản đồ map id ↔ tên map — **CẦN AGENT TRA TIẾP**

* `eve.emg` dùng **map id thật** (10851, 12000, 12851…) — trùng `Data/Warps.txt`.
* ~~`Scene.Dat` có **7 480 map** đánh số theo **record index 1..7480** (tên ShortString tại `+0x03`).~~
  ⇒ **ĐÃ GIẢI MÃ XONG** (Round-6): map id **không** nằm ở record index mà nằm ngay trong bản ghi
  (u16 tại `+0x00`, mã hóa `(w ^ 0xEA6C) - 9`); khóa tra cứu của client là cặp **`(mapId, kind)`**.
  Bảng: `data_mapid_to_name.tsv` (3 503 map id → tên) — chi tiết ở [`SCENE_DAT_FORMAT.md`](./SCENE_DAT_FORMAT.md).
* Ứng viên bổ sung trong client:
  `SceneSet.Dat` (`FUN_00525720`, có field `name_id` u32), `GS.Dat` (`FUN_00524DC0`),
  `Wem.MMG`, hoặc chính hàm tra cứu trong `TMap` (VMT `0x0077711C`).
* **Việc cần làm:** dựng bảng này để agent đối chiếu *tên map* ↔ *script sự kiện trong map đó*
  (ví dụ: map nào có `Thắng lệnh`/`Khiêu chiến`, map nào spawn NPC 4906 "HổLao QThủBinh").

---

## 5. Vật phẩm sự kiện đã xác định (`data_event_items.tsv`, 179 item)

| Item id | Tên | Mô tả (từ `Item.dat`) | Sự kiện |
|---:|---|---|---|
| 30012 | Công thành đồ S.Ph | "Công thành đồ sao phỏng" | **Công thành chiến** |
| 23200 | Công cụ tháo hủy | "Công cụ dùng để tháo dở khí giới công thành" | Công thành chiến |
| 31121–31123 | Chủy thiết / Bánh xe lớn / Dây xích thô | "…để tấn công thành" | Công thành chiến (xe công thành) |
| 63001–63003 | Tu bổ mộc đầu / sam mộc / cối mộc | "…sửa khí cụ công thành" | Công thành chiến |
| **30014+** | **Thắng lệnh 1 … 50** | **"Khiêu chiến N, dùng lệnh này lãnh thưởng"** | ★ **Lôi Đài Khiêu Chiến** (50 lệnh) |
| **31147** | **Vé vip khiêu chiến** | "Vé vip chuyên dùng trong **lôi đài khiêu chiến**" | ★ **Lôi Đài Khiêu Chiến** (cần vé VIP) |
| 23160 | Lđấu QuánQuânGiới | "Nhẫn bạo kích **loạn đấu đài**, 1 tuần" | ★ **Loạn đấu / Solo PK** |
| 46140 | L.Đấu vũ khí hộp | "Hộp gỗ tặng thưởng **loạn đấu**" | Loạn đấu |
| 18508 | Thủy Chiến Thư | "Binh Pháp của Danh tướng Ngũ Tử Định" | Thủy chiến |
| 18975 | Cửu thiên pháp | "Tác phẩm của Cửu Thiên Huyền Nữ" | Cửu Thiên |
| 29117–29119 | Bản Đồ Quỷ Vương 1–3 | "1 trong các bản đồ Đi Đến Quỷ Vương Động" | Halloween / Quỷ Vương |
| 23111 | Chúc Thất Tịch | "Hoa văn chúc tình nhân được lấy nhau" | Lễ Thất Tịch |
| 29305 / 29306 | ThiệnTâm của YQVương / Tà tâm của YQuỷvương | — | Quỷ Vương |

> 🔎 **Kết luận quan trọng:** "40 NPC" (tên người dùng nói) = **Lôi Đài Khiêu Chiến** —
> xác nhận qua item `31147` ("Vé vip chuyên dùng trong lôi đài khiêu chiến") và chuỗi
> "Thắng lệnh N — Khiêu chiến N, dùng lệnh này lãnh thưởng".
> Còn "2K" = **Giải cứu Nhị Kiều Giang Đông** (đã xác nhận qua chuỗi client).
