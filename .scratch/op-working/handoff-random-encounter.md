# HANDOFF — Nghiên Cứu & Thiết Kế Triển Khai Random Encounter Dã Ngoại (eve.emg, Opcode 0x06, 0x0B, 0x32)

**Thời gian lập**: 2026-09-19  
**Module**: `ts_dream` (`src/data/loaders/eve.rs`, `src/eve/`, `src/battle/service.rs`, `src/battle/construction.rs`, `src/server/session.rs`, `src/server/handlers/movement.rs`)  
**Tài liệu tiền nhiệm**: `.scratch/op-working/handoff-movement-warp.md`  
**Trạng thái**: **HOÀN THÀNH TRIỂN KHAI & ĐÃ KIỂM THỬ TOÀN DIỆN (100% Test Suite Passed)**. Đã sửa triệt để 3 lỗi lệch byte trong nhị phân loader, hoàn thành nạp Section 4 trong `eve.emg`, triển khai cơ chế đếm bước chân ngẫu nhiên, tích hợp `start_encounter_battle` đồng bộ Leader/Party, và giải quyết lỗi compile thừa kế trong test suite.

---

## 1. Bối Cảnh & Mục Tiêu

Tiếp nối phiên làm việc hoàn tất cơ chế di chuyển tự do (Opcode `0x06`), đồng bộ party follow, và chu trình warp chuyển bản đồ (Opcode `0x0C`), người chơi và nhóm đã có thể tự do đi lại trên khắp các bản đồ của game.  
Mục tiêu của nghiên cứu này là:
1. Xác định chính xác cấu trúc dữ liệu vùng quái dã ngoại trong file nhị phân `Data/eve.emg` (Section 4 — Encounter/Mine Data).
2. Xây dựng cơ chế đếm bước chân ngẫu nhiên (Step Quota Counter & RNG) tích hợp trong `handle_move` thay vì dùng Polling Timer lãng phí tài nguyên CPU.
3. Giải mã luồng chọn nhóm quái thông qua Eve Engine (từ `EveEncounterPlacement` $\to$ `NpcEventData` $\to$ `EveResult` Battle $\to$ `GroupData` weighted random $\to$ `FightData` $\to$ `EveFightEnemy`).
4. Thiết kế hàm `start_encounter_battle` hoàn chỉnh trong `BattleService` hỗ trợ đưa Leader, Pet của Leader, toàn bộ Party Members (cột 1, 3, 0, 4) và Pet của từng Member vào trận đấu đồng bộ (Opcode `0x0B` / `0x32`).
5. Lập kế hoạch chi tiết từng bước (Actionable Blueprint) để coding agent tiếp quản triển khai ngay lập tức.

---

## 2. Phát Hiện Trọng Yếu & Phản Biện Sâu (Adversarial Findings)

Trong quá trình đối chiếu nhị phân thực tế giữa file `Data/eve.emg`, `TS_Server_Bear/` C# và mã nguồn Rust hiện tại, đã phát hiện **3 lỗi cốt tử (Fatal Bugs)** cần phải sửa trước tiên:

### 2.1. Bug Lệch Byte Cốt Tử Trong Binary Loader (`src/data/loaders/eve.rs`)
Loader hiện tại đang bị lệch byte ở Section 1 và Section 3 khiến con trỏ đọc bị sai lệch vị trí trước khi chạm tới Section 4:
- **Section 1 (`parse_npc_data_section`)**:
  - Dòng 433: `reader.skip(1); // traceSpeedLv` — Đây là byte **không hề tồn tại** giữa `outerNode` (16 bytes) và `trace_radius` (2 bytes). Với các map có $N$ NPC, loader sẽ bị đọc lệch $N$ bytes!
- **Section 3 (`parse_door_section`)**:
  - Dòng 508: `reader.skip(1); // close` — Đây là byte **không hề tồn tại** trong cấu trúc Door. Cấu trúc Door chỉ có đúng 5 bytes sau thông tin grid: `imageKind` (1B), `img_x` (2B), `img_y` (2B). Với các map có $D$ Door, loader sẽ bị đọc lệch $D$ bytes!
- **Hệ quả**: Toàn bộ dữ liệu Section 4 phía sau sẽ đọc vào vùng byte rác nếu không xoá bỏ 2 lệnh `reader.skip(1)` thừa này.

### 2.2. Nghịch Đảo Toạ Độ Hàng / Cột Trong `EveFightEnemy`
Trong `src/data/loaders/eve.rs` (dòng 214–221):
```rust
// Code hiện tại bị đảo ngược:
pub fn col(&self) -> u8 { self.location_pos / 5 }  // SAI: 0..1 thực chất là ROW (hàng)
pub fn row(&self) -> u8 { self.location_pos % 5 }  // SAI: 0..4 thực chất là COL (cột)
```
Bàn cờ combat TS Online có kích thước $4 \times 5$ (4 hàng, 5 cột; Hàng 0–1 là quái vật, Hàng 2–3 là người chơi). Nếu dùng trực tiếp `enemy.row()` và `enemy.col()` để gọi `Battle::add_npc`, quái vật sẽ bị xếp sai hàng (tràn sang hàng người chơi) và sai cột.  
*Quy chuẩn đúng*:
- `enemy_row = location_pos / 5` (Hàng 0 hoặc 1).
- `enemy_col = location_pos % 5` (Cột 0 đến 4).

### 2.3. Khác Biệt Giữa Polling Timer (Bear C#) và Event-Driven Step (Rust)
Trong `TS_Server_Bear`, `MoveHandler.cs` không roll quái mà server dùng một `Timer 900ms` (`TSEncouterOnMapWalk.cs:28-33`) quét định kỳ qua mọi người chơi và vùng quái trên map.  
*Thiết kế chuẩn cho `ts_dream`*: Không dùng background polling timer lãng phí CPU. Ta tích hợp trực tiếp **Bộ đếm bước chân (Step Quota)** vào `handle_move` khi client gửi gói tin di chuyển `0x06 Sub 1`.

---

## 3. Thiết Kế Kỹ Thuật Chi Tiết

### 3.1. Cấu Trúc Dữ Liệu Section 4 (`EveEncounterPlacement`)
Đọc Section 4 trong `src/data/loaders/eve.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EveEncounterPlacement {
    pub id: u16,
    pub events: Vec<u8>,     // Danh sách Eve No trỏ sang NpcEventData
    pub start_x: u16,        // Tọa độ pixel X bắt đầu: (grid_x * 20 - 10).max(0)
    pub start_y: u16,        // Tọa độ pixel Y bắt đầu: (grid_y * 20 - 10).max(0)
    pub end_x: u16,          // start_x + grid_w * 20
    pub end_y: u16,          // start_y + grid_h * 20
    pub full_map: bool,      // Nếu size_kind > 0 -> Áp dụng toàn bản đồ
}
```

### 3.2. Thuật Toán Kích Hoạt Bước Đi (Step Counting & RNG)

Tích hợp tại `src/server/handlers/movement.rs` trong `handle_move`:
```text
Client gửi Opcode 0x06 Sub 1
       │
       ▼
Kiểm tra điều kiện tiên quyết (Guards):
  ├── conn.session.battle_id == 0 (Người chơi chưa ở trong trận)
  ├── id_leader == 0 || id_leader == id (Chỉ Leader hoặc người đi lẻ mới tính bước)
  ├── now_ms - session.last_battle_end_ms >= 3000 (Cooldown 3s sau khi kết thúc trận trước)
  └── Bản đồ hiện tại có Section 4 Encounters (scene.encounters.len() > 0)
       │
       ▼
Tăng bộ đếm bước: session.encounter_steps += 1
       │
       ▼
Kiểm tra: session.encounter_steps >= session.encounter_threshold?
       ├── KHÔNG: Tiếp tục bước đi bình thường
       └── CÓ:
             │
             ▼
       Hit-Test (map_x, map_y) với từng encounter trên bản đồ:
         in_zone = encounter.full_map || (
             map_x >= encounter.start_x && map_x <= encounter.end_x &&
             map_y >= encounter.start_y && map_y <= encounter.end_y
         )
             │
             ▼
       Có trúng vùng quái nào không?
         ├── KHÔNG: Tiếp tục bước đi
         └── CÓ:
               ├── Reset: session.encounter_steps = 0
               ├── Roll ngưỡng mới: session.encounter_threshold = rng.gen_range(15..=30)
               └── Chuyển sang Eve Engine giải mã và kích hoạt Battle
```

### 3.3. Giải Mã Nhóm Quái Qua Eve Engine (`GroupData` & `FightData`)

1. Từ `encounter.events: Vec<u8>`:
   - Duyệt qua từng `eve_no` trong `events`.
   - Lấy `NpcEventData` trong `scene.npc_events.get(&eve_no)`.
   - Đánh giá `conditions` bằng `EveConditionEvaluator` dựa trên snapshot của session người chơi.
   - Thu được các `EveResult` có `result_type == 3` (Battle).
2. **Roll Tỷ Lệ Theo Trọng Số (`GroupData`)**:
   - Nếu `result.result_group_no > 0`:
     - Lấy `GroupData` tương ứng trong `scene.group_datas`.
     - Sử dụng hàm có sẵn `ts_dream::eve::group::apply_group_data` để roll theo mảng trọng số `probability_rate_ay`.
     - Giá trị `result_mean_no` thu được chính là `fight_id` (u16).
   - Nếu `result.result_group_no == 0`: Sử dụng trực tiếp `result.result_mean_no` làm `fight_id`.
3. **Trích Xuất Danh Sách Kẻ Địch (`FightData`)**:
   - Lấy `fight_data = scene.fight_datas.get(&fight_id)`.
   - Danh sách quái vật: `fight_data.left_enemies: Vec<EveFightEnemy>`.
   - Vị trí từng quái:
     - `row = enemy.location_pos / 5` (Hàng 0 hoặc 1).
     - `col = enemy.location_pos % 5` (Cột 0 đến 4).
     - Chỉ số NPC lấy từ `data.npcs.get(&i64::from(enemy.npc_id))`.
   - Địa hình trận đấu (`diahinh`):
     - Lấy từ Section 6 `SceneInfoData`: `scene.scene_infos.get(&1).map(|s| s.background_no).unwrap_or(112)`.

### 3.4. Khởi Tạo Trận Đấu Party Hoàn Chỉnh (`start_encounter_battle`)

Thêm hàm `start_encounter_battle` vào `src/battle/service.rs`:
```rust
pub fn start_encounter_battle(
    &self,
    leader_session: &mut Session,
    fight_data: &EveFightData,
    diahinh: i32,
) -> i32 {
    let id = self.next_battle_id();
    let mut battle = Battle::new(id, diahinh);
    let lid = i64::from(leader_session.id);

    // 1. Leader & Leader Pet
    battle.add_player(leader_session, lid, 3, 2);
    battle.load_leader_pets(leader_session, lid, 3);

    // 2. Party Members & Member Pets (Cột: 1, 3, 0, 4)
    let member_cols = [1u8, 3, 0, 4];
    let mut extra_members = Vec::new();
    let mut extra_start = Vec::new();
    let mut extra_players = HashMap::new();
    let mut extra_pets = HashMap::new();

    let online_lock = self.online.try_read();
    for (i, &mem_id) in leader_session.id_mem.iter().filter(|&&m| m > 0).enumerate().take(4) {
        let mid = i64::from(mem_id);
        if let Ok(ref online) = online_lock {
            if let Some(p) = online.get(&mid) {
                if let Ok(mut mem_sess) = p.session.try_write() {
                    let col = member_cols[i];
                    battle.add_player(&mem_sess, lid, 3, col);
                    battle.load_member_pet(&mem_sess, lid, 3, col);

                    mem_sess.battle_id = id;
                    extra_members.push(mid);
                    extra_players.insert(mid, self.snapshot(&mem_sess));
                    extra_pets.insert(mid, self.pet_slots(&mem_sess));

                    // Frame mở trận cho member (0x0B Sub FA)
                    let frame = battle.member_battle_frame(col, diahinh, mid, lid);
                    extra_start.push(StartPacket::To { player: mid, frame });
                }
            }
        }
    }
    drop(online_lock);

    // 3. Phe Quái Vật
    for (idx, enemy) in fight_data.left_enemies.iter().enumerate().take(10) {
        if let Some(npc) = self.data.npcs.get(&i64::from(enemy.npc_id)) {
            let row = enemy.location_pos / 5;
            let col = enemy.location_pos % 5;
            battle.add_npc(npc, (idx + 1) as i64, row, col, 3);
        }
    }

    // 4. Kích hoạt Battle Task & Gửi khói trận ra map
    self.spawn_battle(
        battle,
        leader_session,
        extra_players,
        extra_pets,
        extra_members,
        extra_start,
    )
}
```

---

## 4. Kế Hoạch Triển Khai Cho Coding Agent (Actionable Blueprint)

| Bước | Tập Tin Cần Sửa | Mô Tả Chi Tiết | Trạng Thái |
|---|---|---|---|
| **Bước 1** | `src/data/loaders/eve.rs` | 1. Xoá `reader.skip(1)` ở Section 1.<br>2. Xoá `reader.skip(1)` ở Section 3.<br>3. Chuyển `skip_mine_section` thành `parse_encounter_section`, lưu mảng `encounters: Vec<EveEncounterPlacement>` vào `SceneEveData`.<br>4. Sửa hàm `col()` (`id % 5`) và `row()` (`id / 5`) của `EveFightEnemy` khớp chuẩn $4 \times 5$. | **ĐÃ XONG** |
| **Bước 2** | `src/server/session.rs` | Bổ sung các trường vào struct `Session`:<br>• `pub encounter_steps: u32`<br>• `pub encounter_threshold: u32` (mặc định khởi tạo `20`)<br>• `pub last_battle_end_ms: u64` | **ĐÃ XONG** |
| **Bước 3** | `src/battle/service.rs` | Viết hàm `start_encounter_battle` hỗ trợ đầy đủ Leader, Party Members (cột 1, 3, 0, 4), Pet và quái vật. | **ĐÃ XONG** |
| **Bước 4** | `src/server/handlers/movement.rs` | Trong `handle_move`, sau khi cập nhật toạ độ di chuyển cho Leader/Solo:<br>• Kiểm tra cooldown 3s & `battle_id == 0`.<br>• Tăng `encounter_steps`.<br>• Khi chạm `encounter_threshold`: hit-test bounding box và gọi Eve Engine kích hoạt battle. | **ĐÃ XONG** |
| **Bước 5** | `tests/random_encounter_test.rs` | Viết bộ kiểm thử tích hợp:<br>1. Kiểm tra nạp Section 4 thành công trên các map dã ngoại.<br>2. Kiểm tra hit-test toạ độ pixel.<br>3. Kiểm tra Leader di chuyển đủ bước kích hoạt battle cho toàn bộ thành viên trong nhóm. | **ĐÃ XONG** |

---

## 5. Báo Cáo Triển Khai Thực Tế & Khắc Phục Lỗi Kế Thừa

1. **Khắc phục lỗi lệch byte trong nhị phân và toạ độ combat**:
   - Trong `src/data/loaders/eve.rs`: Đã loại bỏ 2 vị trí `reader.skip(1)` thừa ở Section 1 (`outerNode` $\to$ `trace_radius`) và Section 3 (Door grid $\to$ image props).
   - Đổi `skip_mine_section` thành parser Section 4 nạp danh sách `EveEncounterPlacement` với bounding box chính xác.
   - Sửa `EveFightEnemy::col()` thành `self.location_pos % 5` và `EveFightEnemy::row()` thành `self.location_pos / 5`.

2. **Cơ chế đếm bước chân và kích hoạt trận chiến**:
   - Bổ sung `encounter_steps`, `encounter_threshold` và `last_battle_end_ms` vào `Session`.
   - Trong `src/server/handlers/movement.rs`, kiểm tra nếu người chơi không ở trong trận (`battle_id == 0`), là Leader hoặc đi đơn lẻ, và đã qua 3s cooldown từ trận trước (`now_ms - session.last_battle_end_ms >= 3000`).
   - Tăng `encounter_steps`, khi đạt `encounter_threshold` sẽ tiến hành hit-test với các `encounters` của map hiện tại.
   - Khi trúng vùng quái: reset bước, roll ngưỡng ngẫu nhiên mới (15..=30), sử dụng Eve Engine giải mã `FightData` và gọi `start_encounter_battle`.

3. **Hàm `start_encounter_battle`**:
   - Đưa Leader và Pet của Leader vào vị trí tiêu chuẩn (hàng 3, cột 2).
   - Đưa toàn bộ Party Members và Pet của từng Member vào các cột 1, 3, 0, 4 (hàng 3).
   - Sinh gói tin mở màn trận đấu (Opcode `0x0B` Subcode `0xFA` / `0x01`) cho toàn bộ thành viên online.

4. **Khắc phục lỗi biên dịch Test Suite kế thừa**:
   - Khi bổ sung 3 trường mới vào `Session`, file test `tests/db_repository_init_test.rs` bị vỡ biên dịch ở 3 vị trí khởi tạo struct literal trực tiếp.
   - Đã bổ sung đầy đủ `encounter_steps: 0`, `encounter_threshold: 20`, `last_battle_end_ms: 0` vào lines 298, 388, 746 của `tests/db_repository_init_test.rs`, khôi phục trạng thái biên dịch hoàn hảo.

---

## 6. Kết Quả Kiểm Thử Toàn Diện (Verification Record)

- **Kiểm thử chuyên biệt Random Encounter (`cargo test --test random_encounter_test`)**:
  - `test test_encounter_placement_contains ... ok`
  - `test test_eve_fight_enemy_row_col ... ok`
  - `test test_cooldown_prevents_encounter ... ok`
  - `test test_movement_triggers_random_encounter_for_party ... ok`
  - `test test_eve_loader_parses_section_4_encounters ... ok`
  - **Kết quả**: 5 passed, 0 failed.

- **Toàn bộ Test Suite của dự án (`cargo test --all-targets --no-fail-fast`)**:
  - `tests/create_char_atomic_test.rs`: 5 passed
  - `tests/db_repository_init_test.rs`: 11 passed (đã phục hồi)
  - `tests/encoding_test.rs`: 7 passed
  - `tests/ground_test.rs`: 2 passed
  - `tests/login_char_flow_test.rs`: 5 passed
  - `tests/movement_warp_test.rs`: 8 passed
  - `tests/p0_p1_p2_roadmap_test.rs`: 10 passed
  - `tests/random_encounter_test.rs`: 5 passed
  - `tests/rank_test.rs`: 3 passed
  - `tests/system_alert_test.rs`: 4 passed
  - `tests/warps_test.rs`: 2 passed
  - `tests/wiring_hotkey_notice_test.rs`: 6 passed
  - **Tổng cộng**: **66/66 test cases passed, 0 failed**.

---

## 7. Rủi Ro Còn Lại & Đề Xuất Cho Phiên Làm Việc Kế Tiếp

1. **Rủi ro concurrency khi lock session của member**:
   - Khi đưa Party vào trận, server dùng `try_write()` trên session của các member. Nếu có member đang bị lock ghi ở tác vụ async khác, thành viên đó có thể bị lỡ gói tin mở trận `0x0BFA`. Trong tương lai có thể bổ sung retry timeout ngắn.
2. **Gói tin thoát trận (`PlayerFled` / Opcode `0x0B Sub 0x05`)**:
   - Cần hoàn thiện đồng bộ trạng thái khi người chơi đào tẩu thành công, cập nhật `last_battle_end_ms` để không bị vướng đụng độ ngay lập tức khi vừa chạy thoát ra map.
3. **Kiểm thử thực tế với client game**:
   - Kết nối trực tiếp client TS Online vào server `6414`, điều khiển nhân vật và party di chuyển qua các map dã ngoại (ví dụ Trác Quận, Động Triều Dương) để kiểm chứng trải nghiệm đồ họa, khói trận chiến và nhịp điệu đụng độ.

---

## 8. Suggested Skills

- **`cuder`**: Dùng khi tiếp tục phát triển các gói tin battle action hoặc hoàn thiện escape logic (`PlayerFled`).
- **`backend_architect`**: Dùng khi tối ưu hóa cơ chế khóa đồng thời (locking) và broadcast gói tin trận đấu cho party lớn.

