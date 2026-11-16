# Nghiên cứu: Xử lý Debuff tự động bỏ qua lượt & chặn chọn hành động (Auto-Skip / Input Blocking)

- **Ngày thực hiện**: 2026-11-16
- **Phạm vi kiểm tra**: `src/battle/`, `src/server/`, `src/data/loaders/`, `src/protocol/`, `tests/battle/`, `docs/adr/0006-*`, `.scratch/battle-refactor/`, `.scratch/client-pseudo-op-code/`, `.scratch/understand-skills/`
- **Câu hỏi nghiên cứu**: Kiểm tra tình trạng hiện tại của server Rust `ts_dream` đối với yêu cầu:
  > *"Đang dính debuff thì player/pet cần tự động bỏ qua lượt (không thể chọn atk/def/use item/skill/capture/flee)"*

---

## Tóm tắt Kết luận (Executive Summary)

**Tình trạng hiện tại: HỖ TRỢ MỘT PHẦN — Mới chặn ở tầng thực thi (`runner.rs`), CHƯA hỗ trợ ở tầng nhận lệnh, quản lý lượt (`manager.rs`, `handlers/battle.rs`) và chưa đồng bộ đầy đủ gói tin khóa lượt xuống client.**

1. **Ở tầng thực thi (`src/battle/runner.rs::turn_phase2`)**:
   - **Khống chế cứng (Type 3 — `type3_turn > 0`)**: Đã tự động bỏ qua cả 6 hành động (`NormalAttack`, `Defend`, `UseItem`, `CastSkill`, `Capture`, `Flee`) cho cả Player, Pet và NPC bằng cách đặt `c.attacked = true` trước khi đọc `commands`.
   - **Hỗn loạn / Cuồng nộ (Type 15 — `type15_id ∈ {14021, 20014}`)**: Ép chạy `auto_berserk` (đánh thường `10000` vào mục tiêu ngẫu nhiên) và bỏ qua lệnh người chơi. Tuy nhiên, nhánh `14021 | 20014` được kiểm tra **trước** `c.type3_turn`, nên nếu một đấu sĩ vừa bị Hỗn loạn vừa bị Đóng băng/Ngủ/Choáng thì vẫn đánh thường thay vì mất lượt.
2. **Ở tầng quản lý lượt & thu thập lệnh (`src/battle/manager.rs`) — LỖ HỔNG LỚN**:
   - `manager.rs` **không hề kiểm tra** `type3_turn` hay `type15_id` khi:
     - Gửi `34 01` (`your_turn()`) đầu hiệp (dòng 260-277).
     - Xây dựng `waiting_cells` trong `collect_commands` (dòng 327-330, dùng `is_living_player_controlled` tại dòng 388-390).
     - Xác thực `owns_commandable_cell` khi nhận `0x32` (dòng 392-397).
     - Điền lệnh mặc định trong `fill_timeout_commands` (dòng 357-386 — comment dòng 362 ghi *"non-stunned"* nhưng code không kiểm tra stun/type3).
   - **Hệ quả thực tế**: Khi Player hoặc Pet bị đóng băng/ngủ/trói/hỗn loạn và phía client bỏ qua không gửi lệnh `0x32` cho đơn vị đó, **`collect_commands` sẽ bị treo chờ hết timeout 20 giây ở mỗi hiệp** thay vì tự động bỏ qua lượt ngay lập tức.
3. **Ở tầng nhận gói tin (`src/server/handlers/battle.rs`) — LỖ HỔNG MẤT VẬT PHẨM & DESYNC**:
   - `handle_battle_command` (`0x32 sub 1` và `sub 2`, dòng 284-359) và `BattleService::submit_command` (`src/battle/service.rs:1661-1676`) luôn trả `true` miễn là đẩy được vào channel `mpsc`.
   - Khi client gửi `0x32 sub 1` lúc đang dính debuff, server vẫn broadcast `35 05` (`acting`) ở dòng 329.
   - Khi client gửi `0x32 sub 2` (`UseItem`) lúc đang dính debuff, handler lập tức gọi `conn.session.remove_homdo_item(...)` (dòng 347) để trừ vật phẩm trong túi, dù sau đó `runner.rs::turn_phase2` bỏ qua không hồi máu/SP -> **mất oan vật phẩm**.
4. **Ở tầng đồng bộ giao thức xuống client (`34 01`, `35 01`, `35 05`, `35 0F`)**:
   - `turn_phase1` (`runner.rs`) chỉ chạy **sau** `collect_commands`, nên `35 05` (`acting`, dòng 679-681) chỉ được phát lúc giải quyết lượt (và lại bị `34 01` của hiệp sau reset `unit+0x552 = 0` trên client).
   - `apply_status3` và `apply_buff15` phát `35 0F` (`battle_unit_status`) và `32 01` (`skilling_int` với `troi_byte::TYPE3/TYPE15`), nhưng `troi_start` (`35 01 ... 01 <skillId>`) hiện **chỉ** gọi trong nhánh phản debuff của `10026` (dòng 2044, 2625), không gọi khi cast debuff trực tiếp trúng mục tiêu.
   - Hiệu ứng phụ từ đòn đánh (`apply_physical` / `apply_magic` khi `attack.inflicted_status > 0` tại dòng 1654-1658, 1920-1924) gán `target.type3_id/lv/turn` nhưng **không** phát bất kỳ frame `35 0F` / `35 01` / `troi_byte::TYPE3` nào.
5. **Kiểm thử (`tests/`)**:
   - Hiện **chưa có test nào** trong `tests/` kiểm chứng việc chặn 6 hành động (`atk/def/use item/skill/capture/flee`) khi bị debuff ở `runner.rs` hay việc tự động skip trong `BattleManager::collect_commands` / `handlers/battle.rs`.

---

## 1. Hệ thống Debuff / Status Effect hiện có trong `src/`

### 1.1. Lưu trữ trạng thái trên đấu sĩ (`WarInfo`)

Trong [`src/battle/engine.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/engine.rs#L49-L99), mỗi ô trên lưới chiến đấu $4 \times 5$ (dùng chung cho Player `typ == 2`, Pet `typ == 4`, Hostile NPC `typ == 3`, TeamDef NPC `typ == 7`) lưu 4 kênh trạng thái độc lập dạng triplet `(id, lv, turn)`:

| Kênh trên `WarInfo` | Vị trí khai báo | Bản chất trong Engine | Các ID tiêu biểu đã đối chiếu trong `src/` |
|---|---|---|---|
| **Type 3** (`type3_id`, `type3_lv`, `type3_turn`) | `engine.rs:82-84` | **Khống chế cứng / Disable / Crowd Control** (một số kèm DOT) | • `10004` (Cây tinh), `10033` (Sa Bộc Cầm): khóa lượt + DOT (`runner.rs:540-569`)<br>• `11014` (Băng phong), `11008`, `11027`, `11028`, `20026`, `6` (`ICE_BOUND_STATUS_ID` tại `combat_formula.rs:17,82-86` nhân `1.5x` sát thương)<br>• `14008` (Hôn mê), `11003` (`dispel.rs:172,180,187,194`)<br>• `13007` (Huyền kích), `13029` (Sơn lam): Type 1 kèm roll gán Type 3 3 lượt (`runner.rs:1661-1690`)<br>• `13015..=13018` (Thanh Long / hút SP + hồi HP caster, `runner.rs:2013-2036`)<br>• Mọi skill có `skill_type == 3` trong `apply_status3` hoặc `hit_status > 0` từ đòn đánh |
| **Type 4** (`type4_id`, `type4_lv`, `type4_turn`) | `engine.rs:85-87` | **Buff phòng vệ / Khiên / Phản** | `10010` (Kết giới), `10015` (Kính), `10026` (Linh Kính phản debuff), `10031`, `13021`, `13003` (Lẩn tránh), `13005` (Ẩn mình, tự xóa sau khi hành động tại `runner.rs:1230-1244`) |
| **Type 15** (`type15_id`, `type15_lv`, `type15_turn`) | `engine.rs:88-90` | **Debuff mềm / Curse / DOT / Hỗn loạn** | • `14015` (Bỏ độc): Poison DOT `30 + lv * 15` (`runner.rs:572-597`); cùng `20023`, `21008` trong `dispel.rs:202`<br>• `14021` (Hỗn loạn), `20014` (Cuồng nộ): ép `auto_berserk` (`runner.rs:718-719, 845-876, 1501-1504`)<br>• `10016..=10019`, `10025`, `20022`: trừ `200` AGI khi trúng (`runner.rs:2619-2621, 2636-2638`) và cộng lại `+200` AGI khi hết hạn/giải (`runner.rs:636-638`, `dispel.rs:117-122`); riêng `10016..=10019` giảm dmg `apply_physical`/`apply_magic` (`1/3` miss, `2/3` chia `10` tại `runner.rs:1628-1638, 1867-1874`) |
| **Type 19** (`type19_id`, `type19_lv`, `type19_turn`) | `engine.rs:91-93` | **Aura nghề nghiệp / Buff đặc biệt** | `14040` (Bá ý), `14044` (Thiên thần thuẫn), `14046` (Ngưng khí), `14053` (Thánh linh thuật), `13020` (Phong Chi Linh, cộng/trừ `ts.agi_13020`) |

### 1.2. Vòng đời Apply → Tick → Expire / Cure

1. **Apply (Gắn hiệu ứng)**:
   - **Kênh Type 3 (`apply_status3`, `runner.rs:1932-2079`) & Type 15 (`apply_buff15`, `runner.rs:2537-2657`)**:
     - Chống cộng dồn trong cùng kênh: nếu `target.type3_id > 0` (hoặc `target.type15_id > 0`) thì trả `miss_status::MISS`, không ghi đè và không làm mới lượt.
     - Roll trúng đích bằng `damage::get_random_miss_troi(...)` + `self.apply_sach_dong_reroll(attacker, &mut hit)`.
     - Tính số lượt qua `skill_round` (`runner.rs:182-187`), ưu tiên `BinarySkillDef.round > 0` và fallback sang `damage::get_turn(skill_id, skill_lv)` (`damage.rs:227-317`). Trong [`src/data/loaders/skill_pc.rs:312`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/skill_pc.rs#L312), `pc_to_binary` luôn gán `round: 0` để tránh nhầm `pc.state` (visual sprite ID) thành số lượt.
     - Nếu mục tiêu đang có khiên phản `target.type4_id == 10026`, debuff dội ngược về `attacker` nếu `attacker.typeX_id == 0` (`runner.rs:2037-2052, 2615-2634`).
   - **Hiệu ứng phụ từ đòn đánh Type 1 / Type 2 (`combat_formula.rs:107-116`, `runner.rs:1654-1658, 1920-1924`)**:
     - Khi `skill.hit_status > 0 && skill.round > 0` (trong đó `binary_skill_input` tại `runner.rs:236` lấy `damage::get_turn(skill_id, 1)` khi `def.round == 0`) và roll `status_chance` thành công, nếu `target.type3_id == 0` thì gán `target.type3_id = attack.inflicted_status`, `type3_lv`, `type3_turn`.
   - **Kỹ năng vật lý cũ `13007 | 13029` (`runner.rs:1661-1690`)**: Roll `get_random_miss_troi` và gán `type3_id = skill`, `type3_turn = 3` nếu `target.type3_id == 0`.
2. **Tick & Expire (`turn_phase1`, `runner.rs:494-684`)**:
   - Đầu `run_turn`, cho mỗi ô có `cell.id > 0`:
     - Reset `c.id_skill = 0`, `c.row_attack = 0`, `c.col_attack = 0`, `c.attacked = false` (dòng 508-511).
     - Giảm lượt: `if c.type3_id > 1 { c.type3_turn -= 1; }` (tương tự cho `type4`, `type15`, `type19` tại dòng 526-537). *(Ghi chú: điều kiện `> 1` khiến trạng thái có `id == 1` sẽ không bao giờ giảm lượt).*
     - Gây DOT: Burn (`10004 | 10033`, dòng 540-569) và Poison (`14015`, dòng 572-597) gọi `apply_hp_loss` và phát `32 01` (`20001` hoặc `20003`) + `33 01` (`set_hp`).
     - Giải hạn (Auto-dispel): nếu `c.typeX_id > 0 && c.typeX_turn <= 1` (dòng 600-676), reset `id/lv/turn = 0`, hoàn trả `+200` AGI (với `10016..=10019, 10025, 20022`) hoặc trừ `ts.agi_13020` (với `13020`), rồi broadcast `35 01` (`purge_status`) và `35 0F` (`clear_unit_status`).
     - Nếu `c.type3_turn > 0` sau khi kiểm tra hết hạn: broadcast `packets::acting(row, col)` (`35 05`, dòng 679-681).
3. **Cure / Dispel (`src/battle/dispel.rs`, gọi qua `runner.rs:2126-2156`)**:
   - Gồm 3 nhóm: Archetype A (`apply_specific_dispel`, dòng 146-220), Archetype B (`apply_mass_cleanse`, dòng 224-243), Archetype C (`apply_cleanse_and_heal`, dòng 247-306). Dùng `purge_type3` (dòng 105-113) và `purge_type15` (dòng 116-131) để xóa debuff và phát cặp `35 01` + `35 0F`.

---

## 2. Phân tích 3 Tầng xử lý: Chặn chọn hành động & Tự động bỏ qua lượt

### 2.1. Tầng 1: Tiếp nhận gói tin & Quản lý lượt (`handlers/battle.rs`, `service.rs`, `manager.rs`)

Hiện tại, **Tầng 1 hoàn toàn KHÔNG biết và KHÔNG kiểm tra debuff của đấu sĩ**:

1. **Handler nhận lệnh `0x32` ([`src/server/handlers/battle.rs:284-359`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L284-L359))**:
   - `handle_battle_command` chỉ kiểm tra `payload.len() >= 6`, `conn.session.battle_id != 0`, biên lưới `src_grp/dst_grp <= 3`, `src_slot/dst_slot <= 4`.
   - `sub == 1` (dòng 306-331): Ánh xạ `10000` (`NormalAttack`), `17001` (`Defend`), `18001` (`Flee`), `15001` (`Capture`), và `skill_id` (`CastSkill`). Nếu `service.submit_command(&conn.session, cmd)` trả `true`, handler lập tức `service.broadcast(format!("F44404003505{:02X}{:02X}", src_grp, src_slot))` (dòng 327-330).
   - `sub == 2` (dòng 333-354): Tạo `BattleAction::UseItem { item_id }`. Nếu `service.submit_command(&conn.session, cmd)` trả `true`, handler **lập tức trừ 1 vật phẩm khỏi túi đồ** (`conn.session.remove_homdo_item(item_id as u16, 1)` tại dòng 345-347).
   - Trong [`src/battle/service.rs:1661-1676`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1661-L1676), `submit_command` chỉ tra `BattleHandle` và gọi `h.command(...)` (`self.tx.send(input).is_ok()` tại `manager.rs:54-56`). Do là async channel, `submit_command` **luôn trả `true`** miễn là battle task còn sống — **không có bất kỳ xác thực debuff nào**.
   - **Hệ quả**: Nếu client gửi lệnh dùng item `0x32 sub 2` khi nhân vật/pet đang bị đóng băng/ngủ/hỗn loạn, server vẫn trừ vật phẩm khỏi `session.homdo`, sau đó `runner.rs` lại bỏ qua lệnh đó -> **mất item oan**.

2. **Vòng lặp lượt đấu trong [`src/battle/manager.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/manager.rs#L257-L407)**:
   - **Bước 1 — Phát `34 01` (`manager.rs:260-277`)**:
     ```rust
     if cell.hp > 0 {
         if cell.typ == 2 && cell.id > 0 { return Some(cell.id); }
         else if cell.typ == 4 && cell.id_char > 0 { return Some(cell.id_char); }
     }
     ```
     Server gửi `packets::your_turn()` (`34 01`) cho **mọi** người chơi còn sống (hoặc có pet còn sống), **bất kể** player/pet đang bị đóng băng, hóa đá, ngủ hay hỗn loạn.
   - **Bước 2 — Chờ lệnh trong `collect_commands` (`manager.rs:317-355`)**:
     - Danh sách ô phải chờ (`waiting_cells`, dòng 327-330) dùng predicate `is_living_player_controlled` (dòng 388-390):
       ```rust
       fn is_living_player_controlled(cell: &crate::battle::engine::WarInfo) -> bool {
           cell.id > 0 && cell.hp > 0 && (cell.typ == 2 || (cell.typ == 4 && cell.id_char > 0))
       }
       ```
       **Không có kiểm tra `type3_turn` hay `type15_id` (`14021 | 20014`).**
     - Khi nhận `PlayerInput` từ `rx.recv()` (dòng 340-348), `owns_commandable_cell` (dòng 392-397) cũng chỉ gọi `is_living_player_controlled` + kiểm tra chủ sở hữu (`cell.id == player` hoặc `cell.id_char == player`). Lệnh gửi cho ô đang dính debuff **vẫn được chấp nhận** vào `commands` và đánh dấu sẵn sàng cho ô đó.
     - **Hệ quả (Treo 20s timeout)**: Nếu client không gửi `0x32` cho Player hoặc Pet đang bị khống chế (đúng với hành vi bỏ qua slot trên client khi bị debuff), `all_ready` (`manager.rs:331`) luôn `false`, buộc trận đấu phải chờ hết **20 giây timeout** (`default_timeout` tại dòng 95) mới chạy tiếp hiệp đó!
   - **Fallback khi hết giờ (`fill_timeout_commands`, `manager.rs:357-386`)**:
     - Comment tại dòng 362 ghi:
       `// Auto-confirm a basic attack for every living, non-stunned player-controlled fighter that timed out.`
     - Tuy nhiên code thực tế (dòng 365-385) **không hề kiểm tra stunned/debuff**, vẫn điền `BattleAction::NormalAttack` cho tất cả `waiting_cells` chưa có lệnh.

3. **Bẫy kỹ thuật về thứ tự tick giữa `manager.rs` và `runner.rs`**:
   - Hiện tại `manager.rs:280` gọi `collect_commands` **trước** `battle.run_turn` (`manager.rs:281`), trong khi việc giảm `typeX_turn -= 1` và auto-dispel (`typeX_turn <= 1`) lại nằm bên trong `turn_phase1` của `run_turn` (`runner.rs:526-676`).
   - Do đó, tại thời điểm `collect_commands` đang chạy ở đầu Hiệp N, một hiệu ứng sắp hết hạn ngay trong `turn_phase1` của Hiệp N vẫn đang mang giá trị `type3_turn == 1` (với `type3_id > 1`). Nếu kiểm tra ngây thơ `cell.type3_turn > 0` ngay trong `collect_commands` mà không tính tới việc `turn_phase1` sắp giảm 1 lượt và giải ở ngưỡng `<= 1`, server sẽ khóa nhầm đấu sĩ ở chính lượt mà họ được giải khống chế!

### 2.2. Tầng 2: Thực thi hành động (`src/battle/runner.rs`)

Kiểm tra trực tiếp `turn_phase2` (`runner.rs:710-772`) và `turn_phase4` / `execute_entity` (`runner.rs:880-968`):

```rust
// src/battle/runner.rs:717-769
if c.hp > 0 {
    if matches!(c.type15_id, 14021 | 20014) {
        self.auto_berserk(data, &mut c);
    } else if c.type3_turn == 0 && c.id_skill == 0 && matches!(c.typ, 3 | 7) {
        self.auto_npc(data, &mut c);
    } else if c.id_skill > 0 || c.type3_turn > 0 {
        c.attacked = true;
    }

    // Apply a submitted player command (op 0x32 sub 1 / sub 2).
    if let Some(cmd) = commands.get(&(c.row, c.col)) {
        if !c.attacked && c.row == cmd.row && c.col == cmd.col {
            match cmd.action { ... }
            c.attacked = true;
        }
    }
}
```

Đánh giá chi tiết cho từng đối tượng và từng hành động:

- **Đối với Player (`typ == 2`) và Pet (`typ == 4`) bị Type 3 (`c.type3_turn > 0` sau `turn_phase1`)**:
  - `c.type3_turn > 0` làm nhánh `else if c.id_skill > 0 || c.type3_turn > 0` đặt `c.attacked = true` (dòng 722-724).
  - Do `!c.attacked` ở dòng 728 là `false`, **cả 6 nhánh `BattleAction` (`UseItem`, `NormalAttack`, `Defend`, `Flee`, `Capture`, `CastSkill`) đều bị bỏ qua** (không gán `c.id_skill`, không gọi `apply_use_item`).
  - Trong `turn_phase4::execute_entity` (dòng 940-950), `skill_ok` đòi hỏi `skill > 0` (`attacker.id_skill > 0`). Vì `c.id_skill` đã bị `turn_phase1` reset về `0` (dòng 508), `execute_entity` thoát sớm -> **không thực hiện hành động nào trong pha resolve**.
  - `runner.rs` **gom chung** toàn bộ debuff kênh Type 3 (`c.type3_turn > 0`), không phân biệt từng ID (choáng, đóng băng, ngủ, trói cây, phong ấn...).
- **Đối với NPC (`typ == 3 | 7`) bị Type 3 (`c.type3_turn > 0`)**:
  - Nhánh `auto_npc` (dòng 720) yêu cầu `c.type3_turn == 0`, nên NPC bị khóa Type 3 không được chọn skill và rơi xuống dòng 722 đặt `c.attacked = true`, bị bỏ qua lượt giống Player/Pet.
- **Đối với Hỗn loạn / Cuồng nộ (`type15_id ∈ {14021, 20014}`)**:
  - Áp dụng đồng nhất cho cả Player, Pet và NPC: gọi `self.auto_berserk` (dòng 718-719, 845-876), ép `c.id_skill = 10000`, `c.lv_skill = 1`, random mục tiêu phe đối địch và đặt `c.attacked = true` -> mọi lệnh `0x32` từ người chơi (kể cả `Defend`, `UseItem`, `Flee`, `Capture`, `CastSkill`) đều bị bỏ qua.
  - **Bug ưu tiên trạng thái**: Vì `if matches!(c.type15_id, 14021 | 20014)` đứng trước `c.type3_turn > 0`, nếu một đơn vị đồng thời dính Type 3 (VD: Đóng băng `11014` / Hôn mê `14008`) và Type 15 Hỗn loạn (`14021`), đơn vị đó **vẫn đánh thường qua `auto_berserk`** thay vì mất lượt!
- **Các Type 15 khác** (`14015` Bỏ độc, `10016..=10019` giảm AGI/dmg, `13011` Thu nhỏ...): **không** khóa hành động và không tự đánh.

### 2.3. Tầng 3: Giao thức báo về Client (`34 01`, `32 01`, `32 02`, `35 01`, `35 05`, `35 0F`)

- **Khi dính debuff từ `apply_status3` / `apply_buff15`**:
  - Server gửi `32 01` (`skilling_int` với `troi_byte::TYPE3 = 0xDD` hoặc `troi_byte::TYPE15 = 0xDF`) và `35 0F` (`battle_unit_status` với `STATUS_SEAL_RESIST_PCT = 'F'` hoặc `STATUS_DEF_MODIFIER = 'B'`, kèm số lượt và giá trị).
  - **Thiếu `35 01` (`troi_start`) ở đường thường**: `packets::troi_start(row, col, skill)` (`F44407003501{r}{c}01{le16(skill)}`) chỉ được phát khi bị `10026` phản debuff về `attacker` (`runner.rs:2044, 2625`), **không** phát khi cast debuff trực tiếp lên `target`. Trong khi đó theo `BATTLE_MODULE_REVERSE_SPEC_CLEAN.md` (§2.4.1), `35 01` điều khiển 5 lớp FX trạng thái trên `TFightField` (Layer 1 Khống chế cứng, Layer 3 Debuff/Độc/Hỗn loạn).
  - **Thiếu đồng bộ wire cho secondary status trên đòn đánh (`apply_physical` / `apply_magic`)**: Tại `runner.rs:1654-1658` và `1920-1924`, khi `attack.inflicted_status > 0` (ví dụ `ICE_BOUND_STATUS_ID = 6`), server chỉ gán `target.type3_id/lv/turn` trong bộ nhớ, **không** phát `35 0F`, `35 01` hay `troi_byte::TYPE3` trong `32 01`.
- **Khi đang dính debuff ở các lượt tiếp theo**:
  - `turn_phase1` phát `packets::acting(row, col)` (`F44404003505{r}{c}`, dòng 679-681) khi `c.type3_turn > 0`.
  - Tuy nhiên `turn_phase1` chỉ chạy sau khi `manager.rs` đã gửi `34 01` và kết thúc `collect_commands`; hơn nữa khi `34 01` (`FUN_00650a24`) đến ở hiệp kế tiếp, client tự xóa cờ `*(slot + 0x552) = 0`.
  - Server không phát gói riêng nào báo "đã auto-skip trước khi mở menu lệnh" ngoài các frame trạng thái `35 01` / `35 0F` / `32 01` từ lượt trước đó.

---

## 3. Đối chiếu Spec & Tài liệu (`docs/adr/0006-*`, `spec/`, `.scratch/`)

| Tài liệu đối chiếu | Mô tả chuẩn của TS Online PC (`aLogin.exe`) / Thiết kế | Đối chiếu với code Rust hiện tại | Đánh giá |
|---|---|---|---|
| **`.scratch/battle-refactor/BATTLE_MODULE_REVERSE_SPEC_CLEAN.md`** (§4.1.1, dòng 414-427)<br>& **`.scratch/client-pseudo-op-code/opcode_34.md`** (§4.2, dòng 107-135) | Khi nhận `34 01` (`FUN_00650a24`):<br>- Reset `+0x569 = 0`, `+0x56a = 0`, `+0x552 = 0`, chọn `slot 0` (`+0xe78 = 0`).<br>- Kiểm tra `slot + 0x3ea == 0` (chết) hoặc `(byte)(*(char*)(slot + 0xdb) - 1) < 100` (`slot + 0xdb ∈ 1..=100`, tức đang bị trạng thái khóa hành động).<br>- Nếu rơi vào điều kiện trên, client tự đặt `*(slot + 0x56a) = 1` (đã xong lệnh) và gọi `FUN_00651094` / `FUN_00650f40` để bỏ qua slot bị khóa, chuyển sang Pet (hoặc đặt `+0xe78 = -1` nếu cả hai đều không thể hành động). | - `manager.rs:260-277` vẫn gửi `34 01` cho tất cả đơn vị sống.<br>- `manager.rs:327-334` (`collect_commands`) **chỉ** lọc `hp > 0` (`is_living_player_controlled`), không bỏ qua đơn vị bị debuff.<br>- Vì client tự skip slot bị khóa và không gửi `0x32` cho slot đó, `collect_commands` bị treo chờ timeout 20s mỗi lượt. | ⚠️ **CHƯA KHỚP**<br>(Lệch nghiêm trọng ở `manager.rs`) |
| **`.scratch/battle-refactor/BATTLE_MODULE_REVERSE_SPEC_CLEAN.md`** (§2.4.1, dòng 201-220)<br>& **`.scratch/client-pseudo-op-code/opcode_35.md`** (§4.3) | - `35 01` cập nhật 5 layer FX trên `TFightField` (Layer 1: Hard CC như trói/băng/choáng/ngủ; Layer 3: Độc/Hỗn loạn).<br>- `35 0F` cập nhật 6 khe HUD `'B'..'G'` (`turns`, `skillId`). | - `apply_status3` / `apply_buff15` có gửi `35 0F` (`'F'` / `'B'`), và `turn_phase1` / `dispel` có gửi `35 01` purge + `35 0F` clear khi hết hạn.<br>- `troi_start` (`35 01`) thiếu ở đường cast trực tiếp (chỉ có ở `10026` reflect).<br>- Secondary status từ đòn đánh (`combat_formula`) không phát `35 0F` hay `35 01`. | ⚠️ **HỖ TRỢ MỘT PHẦN** |
| **`.scratch/understand-skills/battle-buff-debuff-lifecycle.md`** (dòng 159-164) | Trong `Action / Command Phase`: nếu `type3_turn > 0` -> đơn vị bị khóa, phát `packets::acting`, bỏ qua đòn đánh (`Skip attack`). | `runner.rs::turn_phase2` (dòng 722-724) đặt `c.attacked = true`, bỏ qua toàn bộ `BattleAction` và `auto_npc`; `turn_phase1` (dòng 679-681) phát `packets::acting`. | ✅ **ĐÃ ĐẠT** ở `runner.rs` *(trừ case chồng Hỗn loạn `14021 | 20014` + Type 3)* |
| **`docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md`** (§2.2, dòng 37-39) | Bước 1 phát `34 01` cho người chơi còn sống; Bước 2 gom lệnh từ `owns_commandable_cell` và auto-attack nếu timeout 20s. | `manager.rs` triển khai đúng nguyên văn ADR 0006, nhưng chính ADR 0006 **chưa đặc tả ngoại lệ** bỏ qua lượt cho đơn vị bị debuff khóa hành động (`type3` / `14021 | 20014`). | ⚠️ **THIẾU ĐẶC TẢ TRONG ADR** |

---

## 4. Kiểm tra Test hiện có (`tests/`)

Qua rà soát toàn bộ test suite trong `tests/battle/` (liệt kê tại [`tests/battle_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle_test.rs#L1-L48)) và [`tests/skill_job_system_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/skill_job_system_test.rs):

| Tệp test đã kiểm tra | Nội dung đang phủ | Khoảng trống đối với yêu cầu Debuff Auto-Skip / Input Blocking |
|---|---|---|
| [`tests/battle/buff_debuff_lifecycle.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/buff_debuff_lifecycle.rs) | 5 tests: bảng `get_turn` + loader `round: 0`; chống dồn `MISS` trên Type 3/4/15; tồn tại song song 4 kênh; secondary status từ đòn đánh; poison DOT + auto-dispel `35 01`/`35 0F`. | **Không** gửi lệnh cho nạn nhân đang dính Type 3; các turn sau truyền `BattleCommands::new()` rỗng. Không test `auto_berserk` (`14021 | 20014`). |
| [`tests/battle/dispel_cleanse.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/dispel_cleanse.rs) | Giải/thanh tẩy Type 3, Type 4, Type 15, Type 18 (Archetype A/B/C) và thứ tự frame `35 01 -> 35 0F -> 32 01`. | Không test nạn nhân bị chặn 6 hành động trước khi được giải. |
| [`tests/battle/timing_wiring.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/timing_wiring.rs) | `turn_phase1` phát `35 01` + `35 0F` khi status hết hạn (`type3/4/15/19`); `35 03 -> 32 01 -> 35 0F`; `35 06 -> 32 02`; dynamic delay. | Không test chặn hành động hay bỏ qua lượt trong `BattleManager` hoặc `runner.rs`. |
| [`tests/battle/lifecycle.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/lifecycle.rs) | `0B FA` handshake barrier; `34 01` trước `32 02`; timeout fallback đánh thường; player chết nhưng pet sống vẫn nhận `34 01`; `Flee`. | Không test trường hợp player/pet đang dính Type 3 hoặc `14021 | 20014` (xem có tự skip không hay treo chờ timeout). |
| [`tests/battle/command_channel.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/command_channel.rs) | Parse & dispatch `0x32 sub 1/2`, `0x0B` subs, trừ 1 item khi `sub 2` được chấp nhận. | Không test từ chối `0x32 sub 1/2` (và giữ nguyên `homdo` ở `sub 2`) khi unit đang dính debuff. |
| [`tests/battle/status_frames.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/battle/status_frames.rs) | Byte-exact builder `35 0E`, `35 0F`, `35 01` (`purge_status`). | Chỉ test format gói tin. |

**Kết luận về Test**: **Chưa có bất kỳ test nào** kiểm chứng:
1. Đấu sĩ (Player hoặc Pet) bị Type 3 (`type3_turn > 0`) gửi đủ 6 loại `BattleAction` (`NormalAttack`, `Defend`, `UseItem`, `CastSkill`, `Capture`, `Flee`) đều không được thực thi.
2. `BattleManager::collect_commands` tự động bỏ qua lượt (kết thúc thu thập lệnh ngay, không chờ timeout 20s) khi Player/Pet đang dính debuff khóa lượt hoặc Hỗn loạn.
3. Gói `0x32 sub 2` (`UseItem`) không làm mất vật phẩm trong `session.homdo` nếu đấu sĩ đang bị debuff không thể hành động.
4. Thứ tự ưu tiên khi cùng lúc dính Type 3 (khóa lượt) và Type 15 Hỗn loạn (`14021 | 20014`).

---

## 5. Tổng hợp các lỗ hổng cần xử lý để đáp ứng hoàn chỉnh yêu cầu

1. **Lỗ hổng #1 (`manager.rs` — Critical)**: `collect_commands` (`waiting_cells`, `owns_commandable_cell`, `fill_timeout_commands`) phải nhận biết đơn vị đang bị khóa hành động (cả Type 3 còn hiệu lực sau tick đầu hiệp lẫn Type 15 `14021 | 20014` Hỗn loạn/Cuồng nộ) để **không chờ lệnh** (auto-skip ngay lập tức, không treo 20s timeout) và **từ chối nhận lệnh `0x32`** cho ô đó.
2. **Lỗ hổng #2 (`manager.rs` + `runner.rs` — Timing Tick)**: Cần thống nhất thời điểm tick/expire `turn_phase1` so với lúc phát `34 01` và `collect_commands` (hiện `run_turn` chạy *sau* `collect_commands`), nếu không sẽ sai lệch 1 lượt ở lượt mà debuff vừa hết hạn (`type3_turn == 1` trước tick).
3. **Lỗ hổng #3 (`handlers/battle.rs` + `service.rs` — Trừ oan item & phát `35 05` giả)**: Khi `0x32 sub 2` bị từ chối vì đang dính debuff (hoặc xác thực qua battle state trước khi xóa item), không được gọi `remove_homdo_item`, và `sub 1` không được broadcast `35 05` cho lệnh bị từ chối.
4. **Lỗ hổng #4 (`runner.rs:718-724` — Ưu tiên Type 3 vs Hỗn loạn)**: Đảo hoặc bổ sung điều kiện để Type 3 (Đóng băng/Ngủ/Choáng/Trói...) ngăn `auto_berserk` (`14021 | 20014`) thay vì bị Hỗn loạn ghi đè.
5. **Lỗ hổng #5 (`runner.rs` — Wire parity cho client biết unit đang bị debuff)**: Bổ sung phát `35 01` (hoặc đồng bộ đúng cơ chế client cần để set trạng thái khóa trên unit) khi gán debuff trong `apply_status3`/`apply_buff15` và cho secondary status từ `apply_physical`/`apply_magic`.
6. **Lỗ hổng #6 (`tests/battle/`)**: Bổ sung integration test cho `manager.rs` (auto-skip không timeout), `handlers/battle.rs` (chặn `0x32 sub 1/2` và không mất item), và `runner.rs` (chặn đủ 6 `BattleAction` khi dính Type 3 + tương tác Type 3 vs `14021 | 20014`).
