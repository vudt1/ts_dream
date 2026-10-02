# 05 — Rust Battle Engine Audit (hiện trạng server `src/`)

Ngày audit: 2026-10-01 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream` · Crate: `ts_dream`
Phạm vi: `src/battle/*`, `src/server/handlers/battle.rs`, `src/server/dispatcher.rs`, `src/protocol/codecs/battle_role.rs`
Bằng chứng client: `.scratch/client-pseudo-op-code/opcode_0b.md`, `opcode_32.md`, `opcode_33.md`, `opcode_34.md`, `opcode_35.md`, `opcode_13.md`

> Không sửa code. Báo cáo chỉ đọc.

---

## §0 Kiến trúc hiện tại

```text
                      ┌────────────────────────────────────────────────┐
  TCP 6414             │ dispatcher::dispatch(opcode)                    │
  ────────────────►   │  0x0B → handlers::battle::handle_battle        │
                      │  0x32 → handlers::battle::handle_battle_command│
                      │  0x13 → pet_actions::handle_pet_summon          │
                      │  0x33/0x34/0x35 C→S → _ => unimplemented        │
                      └───────────────┬────────────────────────────────┘
                                      │ (sync, validate + gate)
                                      ▼
                      ┌────────────────────────────────────────────────┐
                      │ BattleService  (battle/service.rs)              │
                      │  online: HashMap<i64, OnlinePlayer>            │
                      │      OnlinePlayer{ Arc<RwLock<Session>>,         │
                      │                    mpsc::UnboundedSender<Str> }│
                      │  sink: BattleSinkImpl (impl BattleSink)        │
                      │  handles / join_cells / per_exp / next_battle  │
                      └───────────────┬────────────────────────────────┘
                                      │ spawn_battle → manager.spawn_timeout
                                      ▼
                      ┌────────────────────────────────────────────────┐
                      │ BattleManager (battle/manager.rs)               │
                      │  battles: RwLock<HashMap<i32, BattleHandle>>   │
                      │  tokio::spawn per battle:                       │
                      │    collect_commands(rx, 20s)                   │
                      │    Battle::run_turn(&data, &cmds, &mut out)     │
                      │    dispatch(&out, sink)                         │
                      │    Battle::finish(...)  → sink.battle_ended    │
                      └───────────────┬────────────────────────────────┘
                                      │ (sync, in-task; grid + RNG task-local)
                                      ▼
   ┌──────────────────────────────────────────────────────────────────────┐
   │ battle/runner.rs   Battle::run_turn                                  │
   │   turn_phase1 (buff tick / DOT / your_turn 0x34)                     │
   │   turn_phase2 (input + NPC auto + berserk auto)                      │
   │   turn_phase4 → execute_entity (SP gate, skill_type dispatch 1..19)  │
   │   leader_sp_regen, process_kills, npc_respawn, finish                │
   │   → Vec<Out> (Broadcast/ToPlayer/MapBroadcast/Db/Drop/Catch/         │
   │                Fled/Respawn/PetExp/Gold)                             │
   └───────┬───────────────┬──────────────┬──────────────┬────────────────┘
           │               │              │              │
    packets.rs       damage.rs      combat_formula.rs  targeting.rs
  (frame builders)  (rolls, exp,    (calculate_attack,  (fight_area 1..8,
                   get_turn)       element_multiplier)   combo/type4/…)
           │
    engine.rs (WarInfo 23B _Packet, get_hp_max/get_sp_max)
    construction.rs (Battle grid 4×5, npc_battle/teamdef_battle, start packets)
    rng.rs (DotNetRandom + BattleRng: random_0/1/2 + random_damage)
    npc_world.rs (NpcWorld walk 900ms, respawn bounds, 1602 frame)
```

Luồng dữ liệu: handler đồng bộ → `BattleService::submit_command` → `mpsc` channel → task battle gom lệnh (timeout 20 s, `fill_timeout_commands` auto-attack skill 10000) → `run_turn` sinh `Vec<Out>` → `dispatch` gọi `BattleSink` → `BattleSinkImpl` đẩy hex string vào `mpsc` channel của từng player → socket writer của `Conn` rút ra ghi wire. Mọi session-mutation (`apply_db_update`, `apply_drop`, `apply_pet_exp`, `apply_gold`) chạy **hai lần**: một lần qua `self.online`, một lần qua global `session::lock_online_sessions()` (fallback/đồng bộ 2 nguồn state).

Module nào gọi ai:

| Module | Vai trò | Gọi ra |
|---|---|---|
| `engine.rs` | `WarInfo` + `packet_hex()` (23B) + `get_hp_max/get_sp_max` + `war_key` | `data::texps` |
| `construction.rs` | `Battle` (grid 20 ô, `turn_order`, `npc_battle_start_packets`, `member_battle_frame`, `teamdef_battle`) | `packets`, `rng`, `server::session` |
| `packets.rs` | 26 builder hex thuần + 5 module hằng số | `protocol::encoder` |
| `runner.rs` | turn engine, 20 hàm `apply_*` theo skill_type, `Out` | `damage`, `combat_formula`, `targeting`, `packets`, `npc_world`, `rng` |
| `manager.rs` | registry + task vòng lặp + `collect_commands` + `dispatch` | `construction`, `runner` |
| `service.rs` | online registry, sink, spawn các loại battle, join/leave, submit, battle_ended (quest win/lose, persist) | `manager`, `construction`, `handlers::quest/talk` |
| `npc_world.rs` | NPC trên map, walk 900 ms, chase, SoLuong → TeamDef 4712 | `WorldSink` (impl bởi `BattleService`) |
| `handlers/battle.rs` | C→S 0x0B sub 1/2/5/6/8, 0x32 sub 1/2 | `service` |
| `protocol/codecs/battle_role.rs` | `BattleRoleData` encode/decode (38 B base + appearance) | **KHÔNG được battle engine dùng** — chỉ có test/golden harness |

---

## §1 BẢNG ĐẦY ĐỦ frame battle server ĐANG GỬI

### 1.1 Frame mở trận / hiện thực (opcode 0x0B)

| # | Hàm builder | File:dòng | Hex frame (mẫu) | Sub | Mục đích | Nơi gọi | Claim nguồn |
|---|---|---|---|---|---|---|---|
| 1 | `battle_open_leader` | `packets.rs:9` | `F4441C000BFA{le16(diahinh)}03{_Packet 23B}F44403000B0A01` | `0B FA` | Mở bàn cờ cho leader: diahinh + marker `03` + 1 record 23B của chính leader | `construction.rs:342` (`npc_battle_start_packets`) | opcode_0b.md §"SubOp 0xFA": `[0B][FA][mode=RP1][sub=RP2][n record 24B]`. **LỆCH**: server đặt `le16(diahinh)` ở vị trí `mode`/`sub` (2 byte) và chỉ gửi **1 record 23B** có marker 1 byte, trong khi client tính `n = (Len-3)/0x18` (24 B/record) → server gửi nhiều record nối tiếp (mỗi record = marker + 23B = 24 B). Marker `03` = leader, `05` = self, `64` = member khác. `F44403000B0A01` trailer không có trong doc |
| 2 | `battle_open_text` | `packets.rs:18` | `F444{le16(len)}0BFA{le16(diahinh)}{text}F44403000B0A01` | `0B FA` | Bản tổng quát của (1) | **KHÔNG có nơi gọi — code chết** | — |
| 3 | `battle_open_pk_member` | `packets.rs:29` | `F444{le16(len)}0BFA7000{text}F44403000B0A01` | `0B FA` | PK member frame, diahinh cố định 7000 | **KHÔNG có nơi gọi — code chết** | opcode_0b.md không mô tả 7000; PK hiện dùng `member_battle_frame` |
| 4 | `member_battle_frame` (không có builder riêng, format! inline) | `construction.rs:482` | `F444{le16(4+len/2)}0BFA{le16(diahinh)}05{self}03{leader}[03{leaderPet}](64{member}[64{memberPet}])*F44403000B0A01` | `0B FA` | Frame mở trận riêng cho từng member: `05`+self, `03`+leader(+pet), `64`+member khác(+pet) | `construction.rs:386`, `service.rs:797,1013,1046` | record 24B = marker+23B — khớp `n=(L-3)/24` |
| 5 | `build_join_frame` (inline, không dùng `packets.rs`) | `service.rs:1100` | `F444{le16(len)}0BFA{le16(diahinh)}0402{le32(id)}000000000000FFFF{le16 hpmax}{le16 spmax}{le16 hp}{le16 sp}{lv}{tt}(+20 cell record 24B)` | `0B FA` + `04 02` | Join trận giữa chừng (op 0x0B sub 2 inner 4): record của chính mình + 20 ô lưới (rỗng = 24 byte 0) | `service.rs:1085` (`join_battle`) | opcode_0b.md: SubOp 4 đọc record **8B** (`[kind][id 4B][w 2B][val 1B]`), SubOp FA đọc record 24B. Server dùng `0402` marker + record 24B → là biến thể FA, không phải SubOp 4 chuẩn. **Nghi vấn layout** |
| 6 | `record_join_cells` | `service.rs:1253` | (không gửi trực tiếp — chỉ cache string cho (5)) | — | Render 20 cell record 24B lúc spawn, marker `03` nếu leader/owner, `100` (0x64) nếu member | `service.rs:1358` | marker `100` = 0x64 khớp marker member trong (4) |
| 7 | `entity_npc` | `packets.rs:40` | `F4441A000B0503{_Packet 23B}` | `0B 05 03` | Nạp entity NPC vào lưới | `construction.rs:406,516` | opcode_0b.md SubOp 5 = `FUN_00644294` nạp đơn vị; `03` phân biệt loại |
| 8 | `entity_player` | `packets.rs:45` | `F4441A000B0505{_Packet 23B}` | `0B 05 05` | Nạp entity player/pet (typ 2 hoặc 4) | `construction.rs:356,368,381,392,504` | idem |
| 9 | `show_on_map` / `show_player_on_map` | `packets.rs:50,55` | `F4440A000B0402{le32(id)}000003` | `0B 04 02` | Show player lên map khi vào trận | `construction.rs:348,496` | opcode_0b.md SubOp 4 đọc record 8B — server dùng sub `04` với tail 3 byte; doc chỉ mô tả record stream |
| 10 | `show_member_on_map` | `packets.rs:60` | `F4440A000B0402{le32(id)}000005` | `0B 04 02` | Show member (tail `05`) | `construction.rs:372,438,468` | — |
| 11 | `show_pk_leader_on_map` | `packets.rs:65` | `F4440A000B0402{le32(id)}000002` | `0B 04 02` | Show PK leader (tail `02`) | **KHÔNG có nơi gọi — code chết** | — |
| 12 | `hide_from_map` | `packets.rs:70` | `F44408000B00{le32(id)}0000` | `0B 00` | Ẩn entity khỏi map (kết thúc trận, rời trận, flee) | `runner.rs:2142,2890,2895`; `handlers/battle.rs:60,78`; `handlers/quest.rs:880,904`; `handlers/system.rs:519` | opcode_0b.md SubOp 0 = `FUN_00641fa4` |
| 13 | `reposition` | `packets.rs:75` | `F44405000B01{row}{col}00` | `0B 01` | Reposition entity trên lưới | `runner.rs:2143,2891` | opcode_0b.md SubOp 1 đọc **2 byte** `[i1=RP1 0..3][i2=RP2 0..4]` → server gửi 3 byte (`row,col,00`) → **LỆCH 1 byte thừa** |
| 14 | `clear_pet_cell` | `packets.rs:80` | `F44404000B01{(row^1)}{col}` | `0B 01` | Xoá ô pet khi player flee / kết thúc | `runner.rs:2140,2141,2892` | doc: callee đòi tối thiểu 4 byte payload → server gửi 3 byte (`0B 01 row col`) → **LỆCH, thiếu 1 byte** |
| 15 | `battle_trailer` | `packets.rs:151` | `F44403000B0A01` | `0B 0A 01` | Trailer sau mọi frame mở trận | `service.rs:1087` (join), và nhúng trong (1)(4) | opcode_0b.md SubOp `0x0A` = `FUN_0064e978` với `mode: 1B RP[1]` → payload tối thiểu 3, server gửi đúng 3 |
| 16 | `F44406000B06{le32(id)}` (inline) | `handlers/battle.rs:28` | `F44406000B06{le32(id)}` | `0B 06` | Broadcast show-on-map khi op 0x0B sub 6 | `handlers/battle.rs:26` | opcode_0b.md SubOp 6 = `FUN_0072c9e8` đọc `id: DWORD LE RP[1..4]` → **khớp byte-exact** |

### 1.2 Frame lượt / kỹ năng (opcode 0x32, 0x34, 0x35)

| # | Hàm builder | File:dòng | Hex frame | Sub | Mục đích | Nơi gọi |
|---|---|---|---|---|---|---|
| 17 | `your_turn` | `packets.rs:85` | `F44402003401` | `34 01` | Prompt "đến lượt bạn" cho từng ô typ 2 | `runner.rs:483` |
| 18 | `acting` | `packets.rs:90` | `F44404003505{row}{col}` | `35 05` | Đánh dấu ô đang "acting" (silent / flee) | `runner.rs:487,2138,2139`; `handlers/battle.rs:213` (inline copy) |
| 19 | `turn_action` | `packets.rs:95` | `F444{le16(len)}3201{text9}` | `32 01` | Wrapper chung của frame hành động | **KHÔNG có nơi gọi** — `runner.rs:1059` tự dựng `format!("3201{}", text9)`; **code chết** |
| 20 | `combo` | `packets.rs:102` | `F444130032010F00{row}{col}264E0101{row}{col}010301E0000000` | `32 01` (code `0x264E`=9806? thực tế là mã combo) | Prefix combo (shield 20006 rear-hit path) | `runner.rs:1533` (`apply_damage_to_rear`) |
| 21 | `combo_footer_20007` | `packets.rs:243` | `F444130032010F00{row}{col}{le16(20007)}0101{row}{col}01030119000000` | `32 01` | Footer sau khi flush 1 turn có combo | `runner.rs:1082` |
| 22 | `skilling_full_frame` + `skilling_full` | `packets.rs:255,198` | `F44413003201` + `0F00{row}{col}{le16(skillId)}{sl_danh}{skillType}{rowAtk}{colAtk}{miss}{adl}{count}{troi}{le16(dmg)}{buff}` | `32 01` | Frame **17 byte** "Skilling" đầy đủ — hiện chỉ dùng cho **flee thất bại** | `runner.rs:2163` (`apply_flee`) |
| 23 | `troi_start` | `packets.rs:118` | `F44407003501{row}{col}01{le16(skillId)}` | `35 01` | Buff/debuff bắt đầu trên caster | `runner.rs:1745,2330` |
| 24 | `troi_end` | `packets.rs:110` | `F44407003501{row}{col}{troiEndByte}0000` | `35 01` | Buff/debuff hết lượt | `runner.rs:437,447,460,474,997` |
| 25 | `drop_item` | `packets.rs:128` | `F44408003504{le16(itemId)}{npcRow}{npcCol}{row}{col}` | `35 04` | Rơi vật phẩm từ ô NPC sang ô người chơi | `runner.rs:2681,2719` |
| 26 | frame DOT (inline, runner) | `runner.rs:380,396` | `F444{le16(len)}3201{row}{col}{le16(20001)}0101{skilling_int(...)}` | `32 01` | Thiêu đốt (Type3 10004/10033) mỗi lượt | `runner.rs:397` |
| 27 | frame poison (inline) | `runner.rs:408,424` | `F444{le16(len)}3201{row}{col}{le16(20003)}0101{skilling_int(...)}` | `32 01` | Độc (Type15 14015) mỗi lượt | `runner.rs:425` |
| 28 | frame reflect (inline, qua `text11`) | `runner.rs:1426,1442` | `F444{le16(len)}3201{le16(20003-body)}…` | `32 01` | Phản đòn (10015/10031/13021) — gửi **sau** frame hành động chính | `runner.rs:1070` |
| 29 | frame turn chính (inline) | `runner.rs:1059,1060` | `{text_combo}` + `F444{le16(len)}3201{text9}` | `32 01` | Flush toàn bộ hành động của 1 lượt; `text9` là chuỗi record `le16(W){row}{col}{le16(skill)}{num34}{count}{skilling_int…}` | `runner.rs:1066` |

Helper payload (không tự frame, ghép vào (26)-(29)):

| Hàm | File:dòng | Hex | Ghi chú |
|---|---|---|---|
| `skilling_int` | `packets.rs:168` | `{row}{col}{miss}{adl}{count}{troi}{le16(dmg)}{buff}` (10 B) | dùng 27 chỗ |
| `skilling_short` | `packets.rs:232` | `{row}{col}{miss}{adl}` (4 B) | `runner.rs:1385` |
| `skilling_effect` | `packets.rs:237` | `{troi}{le16(dmg)}{buff}` (4 B) | `runner.rs:1393,1395` |

### 1.3 Frame kết thúc trận / UI

| # | Hàm | File:dòng | Hex | Sub | Mục đích | Nơi gọi |
|---|---|---|---|---|---|---|
| 30 | `battle_exit_move` | `packets.rs:156` | `F44402000504` | `05 04` | Bật lại quyền điều khiển / thoát UI trận (opcode 0x05 sub 4 = "world ready / clear mouse mode") | `runner.rs:2149,2907`; `handlers/battle.rs:63,81` |
| 31 | `battle_exit_talk` | `packets.rs:161` | `F44402001408` | `14 08` | Đóng hội thoại trận (opcode 0x14 NpcEvent sub 8) | `runner.rs:2150,2908`; `handlers/battle.rs:64,82` |

### 1.4 Frame trạng thái / chỉ số (0x08), NPC map (0x16)

| # | Hàm | File:dòng | Hex | Mục đích | Nơi gọi |
|---|---|---|---|---|---|
| 32 | `status_update` | `packets.rs:140` | `F4440C000801{statusByte}{sign}{le32(abs)}{00000000}` | Push HP/SP/Texp/Lv/Point/SkillPoint/Fai sau mỗi Db write của battle (opcode 0x08 sub 1) | `service.rs:557` (`apply_player_stat`) |
| 33 | `apply_respawn` (inline) | `service.rs:256` | `F44406001603{le16(id)}0A00F44408001605{le16(id)}{le16(x)}{le16(y)}` | Respawn NPC trên map sau khi thắng/trốn | `runner.rs:2766` (`npc_respawn`) |
| 34 | `npc_walk_frame` (inline) | `npc_world.rs:209` | `F44408001602{le16(id)}{le16(x)}{le16(y)}` | NPC tuần/chase trên map (900 ms tick) | `npc_world.rs:154,175` |

Byte hằng số dùng trong frame: `packets::troi_byte` (`MISS 00, TYPE3 DD, TYPE4 DE, TYPE15 DF, TYPE19 E1, HP 19, SP 1A, HOCHU 0E`), `troi_end_byte` (`TYPE3 1, TYPE4 2, TYPE15 3, TYPE19 5`), `attack_status` (`ATTACK 0, DEF 1, LANTRANH 2`), `miss_status` (`ATTACK 1, MISS 0`), `stat_byte` (`HP 19 … POINT 26`). Lưu ý `stat_byte` và `troi_byte::HOCHU` **không được tham chiếu** (`stat_byte::` grep = 0 kết quả ngoài `packets.rs`); `troi_byte::HOCHU` cũng vậy.

### 1.5 Đối chiếu với tài liệu client

| Frame server | Doc client | Kết luận |
|---|---|---|
| `0B FA …` | opcode_0b.md SubOp FA: `[0B][FA][mode][sub][n×24B]`, `n=(Len-3)/24`, `TFightField_Create(...,mode,sub)` | **Cấu trúc khớp** nếu coi `le16(diahinh)` = mode+sub và mỗi cặp `marker+23B` = 1 record 24 B. Marker dùng: `03` (leader), `05` (self), `64` (member). `64` = 100 khớp `record_join_cells`. Cần xác nhận mapping `mode`/`sub` ↔ `diahinh` |
| `0B 06` | SubOp 6: `id: DWORD LE RP[1..4]` | Khớp byte-exact |
| `0B 05 03/05` | SubOp 5: `FUN_00644294` nạp đơn vị | Khớp |
| `0B 04 02` + tail | SubOp 4: `n=(Len-1)/8` record 8B `[kind][id 4B][w 2B][val]` | **LỆCH**: server gửi `0B 04 02` + le32(id) + 3 byte tail = 5 byte, không phải stream record 8B |
| `0B 01` (reposition/clear_pet) | SubOp 1: callee đòi ≥4 byte, đọc `[RP1 0..3][RP2 0..4]` | **LỆCH**: server gửi 3 byte (`01 row col`) |
| `0B 0A 01` | SubOp 0x0A: `mode: 1B RP[1]` | Khớp |
| `32 01` records | opcode_32.md SubOp 1: record `W(2) srcGrp srcSlot code(2) b2 b3 dstGrp dstSlot text(W-6)`, stride `W+2` | **Khớp cấu trúc**: `runner.rs:1041` dựng `{row}{col}{le16(skill)}{num34}{count}` + `skilling_int` — tức `srcGrp,srcSlot,code,num34(b2),count(b3)` rồi text. `combo()`/`combo_footer_20007` dùng prefix `0F00` (W=15) — giống `skilling_full` 17 byte |
| `34 01` | opcode_34.md: SubOp 1 + `gvar_007DA51C != 0` → `FUN_00650a24` reset + **chạy turn engine `FUN_00658c30`** | Khớp: `your_turn` chính là lệnh "tới lượt" |
| `35 01` | opcode_35.md SubOp 1: `2+5n`, mỗi record `[grp≤3][slot≤4][type∈1..5][w Word LE]` | **Khớp tuyệt đối**: `troi_end` = `35 01 row col type 0000` = 1 record 5 B; `troi_start` = `35 01 row col 01 le16(skill)` = 1 record 5 B |
| `35 04` | SubOp 4: `6+2m`, `[id Word≠0][grp≤3][slot≤4](b1 b2)×m`, `m=(L-5)/2` | **Khớp**: `35 04 id(2) npcRow npcCol row col` = 6+2 = 8 B, `m=1` |
| `35 05` | SubOp 5: `[grp≤3][slot≤4]`, set `unit+0x552=1` | Khớp byte-exact |
| `33` | opcode_33.md: SubOp 1 → `FUN_0064cae0` parse record 7 byte trên lưới | **SERVER KHÔNG GỬI 0x33** (xem §6) |
| `35 03, 06, 07, 08, 09, 0A, 0B, 0C, 0D, 0E, 0F` | opcode_35.md | **Server không gửi** (xem §6) |

---

## §2 BẢNG ĐẦY ĐỦ frame battle server ĐANG NHẬN (C→S)

| Opcode | Sub | Inner sub | Parse offset | Handler | Mục đích / gate |
|---|---|---|---|---|---|
| `0x0B` | `1` | — | `payload[0]` phải `== 3` | `handlers/battle.rs:49` `handle_leave_battle` | Rời trật. Gate: `confirm==3 && battle_id!=0`. Gửi về: `hide_from_map`, `battle_exit_move`, `battle_exit_talk` |
| `0x0B` | `2` | `2` | `payload[1..5]` = target id LE32; `payload[5..7]` = ground LE16 | `handlers/battle.rs:115` `handle_pk_challenge` | PK challenge. Gate: `pk==1`, len≥5, target online, target không trong trận. Target `pk==0` → trả `F4440300210101`; ngược lại `start_pk_battle` |
| `0x0B` | `2` | `3` | `payload[1..5]` = npc id LE32; `payload[5..7]` = ground | `handlers/battle.rs:146` `handle_attack_npc` | Đánh NPC. Gate: `battle_id==0`, len≥5, **chặn npc id 20000..22000 / 23000..25000 / 26000..27000** |
| `0x0B` | `2` | `4` | `payload[1..5]` = leader battle id LE32 | `handlers/battle.rs:96` | Spectate/join → `service.join_battle` |
| `0x0B` | `2` | `5` | `payload[1..5]` | `handlers/battle.rs:102` | `JamPlayerToBattle` — **đọc rồi bỏ (no-op)** |
| `0x0B` | `5` | — | — | `handlers/battle.rs:68` `handle_flee_battle` | Trốn khỏi trận (client-side, không qua roll server) |
| `0x0B` | `6` | — | — | `handlers/battle.rs:24` | Broadcast `F44406000B06` + le32(id) ra map. **Không validate `battle_id`** |
| `0x0B` | `8` | — | `payload[1..5]` (nếu len≥5) hoặc `payload[0..4]` = target id | `handlers/battle.rs:31` | PvP trực tiếp theo id, ground = **65000** |
| `0x0B` | khác | — | — | `_ => {}` | bỏ qua |
| `0x32` | `1` | — | `payload[0]`=row, `[1]`=col, `[2]`=rowAttack, `[3]`=colAttack, `[4..6]`=skill LE16 | `handlers/battle.rs:182` `handle_skill_command` | Lệnh skill. Gate: len≥6, `battle_id!=0`, row≤3, col≤4, rowAtk≤3, colAtk≤4, skill≠0. **Không kiểm tra skill có thuộc người chơi** — `skill_level_for` fallback `1`. Broadcast `F44404003505 row col` |
| `0x32` | `2` | — | cùng offset, `payload[4..6]` = item LE16 | `handlers/battle.rs:220` `handle_use_item` | Dùng item trong trận. Gate: len≥6, `battle_id!=0`, item ∈ `26001..=27165`. Trừ 1 item inventory **ngay** (`remove_homdo_item`) trước khi submit — không rollback nếu command bị từ chối |
| `0x32` | khác | — | — | `_ => {}` | bỏ qua |
| `0x13` | `1` | — | `payload[0..2]` = pet id LE16 | `pet_actions.rs:327` | Đổi pet chiến đấu. **Chỉ set `active_pet_stt` khi `battle_id==0`**; trong trận thì bỏ qua im lặng (comment: "battle task owns the grid") |
| `0x13` | `2` | — | — | `pet_actions.rs:358` | Thu pet (`active_pet_stt=0`, `F44402001302`) — **không gate `battle_id`** |
| `0x33` | — | — | — | `dispatcher.rs:394` `_ => unimplemented::handle(ctx)` | **Chưa nhận** |
| `0x34` | — | — | — | như trên | **Chưa nhận** |
| `0x35` | — | — | — | như trên | **Chưa nhận** |

Ghi chú ownership: `manager.rs:287 owns_commandable_cell` là gate thứ 2 ở tầng task — lệnh chỉ được chấp nhận nếu `player` sở hữu ô nguồn (`typ==2 && id==player` hoặc `typ==4 && id_char==player`) và `hp>0`.

---

## §3 Battle engine nội bộ

### 3.1 Lưới trận và turn order

- Lưới **4 row × 5 col = 20 ô**, key `format!("{row:x}{col:x}")` (`engine.rs:14`). Rows 0-1 = team 2 (địch), rows 2-3 = team 1 (người chơi). `team = (row==0) ? 2 : 1` cho player (`construction.rs:94`); pet đặt ở `(row^1, col)` (`add_pet`, `construction.rs:159`).
- `typ`: `2` = player, `4` = pet/follow-NPC, `3` = NPC địch, `7` = TeamDef. `ehuman` module (`battle_role.rs:14`) định nghĩa bảng client nhưng **không dùng** trong engine.
- Turn order (`construction.rs:310` `turn_order`): sort `attacked DESC` → `agi DESC` → `random DESC`. `random` = `rng.random_1.next_range(0,100)` gán mỗi lượt ở phase1 (`runner.rs:346`).
- Mỗi lượt (`runner.rs:288 run_turn`): check outcome ở đầu → `turn_phase1` → `turn_phase2` → `turn_phase4` → `leader_sp_regen` → check outcome.

### 3.2 Phase 1 (`turn_phase1`, `runner.rs:328`)

- Reset `id_skill/row_attack/col_attack/attacked`, gán `random`.
- Tính `avg1`/`avg2` (lv trung bình mỗi team) **chỉ lượt đầu**.
- Tick buff: `type3/4/15/19_turn -= 1`; khi về 1 → clear + gửi `troi_end`.
- DOT: burn (`type3_id ∈ {10004,10033}`, dmg `10+lv*2` / `30+lv*10`), poison (`type15_id == 14015`, dmg `30+lv*15`) → `apply_hp_loss` + frame `32 01` với mã `20001`/`20003`.
- Type15 `id ∈ {10016..10019, 10025, 20022}` khi hết → `agi += 200`; type19 `13020` → trừ `agi_13020`.
- Gửi `your_turn` (`34 01`) cho mỗi ô `typ==2`; `acting` (`35 05`) cho ô có `type3_turn > 0`.

### 3.3 Phase 2 (`turn_phase2`, `runner.rs:530`)

- Berserk (`type15_id ∈ {14021, 20014}`) → `auto_berserk`: random row đối phương (50/50) + random cột sống theo thứ tự `[2,1,3,0,4]` bằng `randomize_array` (gấp 50% từng cặp), skill `10000`.
- NPC (`typ ∈ {3,7}` và không bị silence) → `auto_npc`: random row 2/3 (50/50) + random cột sống; skill qua `damage::get_random_skill_npc` (thang 5/15/30 theo `reborn`), lv = `max_lv`/`lv_max`.
- Lệnh người chơi: nếu `!attacked` → set `id_skill/lv_skill/row_attack/col_attack`, hoặc `use_item > 0` → `apply_use_item`.

### 3.4 Phase 4 + `execute_entity` (`runner.rs:673, 700`)

1. Force `attacked = true` cho toàn lưới, duyệt theo `turn_order`.
2. **SP gate**: `cost = skill_sp` (binary `require_sp` ưu tiên, fallback `skills.sp`); nếu `sp < cost` → fallback `skill = 10000`.
3. **Skill validity**: `skill_ok = hp>0 && skill>0 && row<4 && col<5 && (skill tồn tại || typ∈{3,7})`; `typ==4` (pet) phải có skill trong `npc.skill[]`; whitelist `ALWAYS_ALLOWED = [10000,15001,15002,15003,17001,18001,18002,19001]`.
4. Đọc metadata: `skill_type` (binary `kind` ưu tiên), `num34 = fight_area`, `combo_field`, `skill_tt = element`, `do_manh`. Remap `13008 → 13012 lv3`.
5. `pick_targets` (`runner.rs:1172`) — xem §3.5.
6. **Combo detect** (`runner.rs:1098`) nếu `skill_type==1` và `idx+1 < len`.
7. Lặp target; lặp `hit_times` (`binary how_much_times`, mặc định 1).
8. Shield 20006: nếu target ở row 0/3 và ô sau (`row^1, col`) có `id_skill == 20006` → `apply_damage_to_rear` (ghi `text10` + `text_combo = combo()`).
9. Dispatch theo `skill_type`:

| skill_type | Hàm | File:dòng | Ghi chú |
|---|---|---|---|
| 1 | `apply_physical` | `runner.rs:1276` | `combat_formula::calculate_attack`; miss → `adl = DEF` nếu `id_skill==17001`, `LANTRANH` nếu `type4_id==13003`; reflect (`type4 ∈ {10010,10015,10031,13021}`) → `apply_reflect`; type15 `10016..10019` → 1/3 chặn hoặc `/10`; skill `13007/13029` → roll `get_random_miss_troi` + 2 skilling block |
| 2 | `apply_magic` | `runner.rs:1539` | Cùng roll, không reflect |
| 3 | `apply_status3` | `runner.rs:1632` | Debuff type3 |
| 4 | `apply_buff4` | `runner.rs:1770` | Buff type4 |
| 5 | `apply_dispel5` | `runner.rs:1800` | Gỡ type4 / cure |
| 6 | `apply_sp_restore` | `runner.rs:1652` | Hồi SP (dùng `troi_byte::SP`) |
| 7 | `apply_hp_restore` | `runner.rs:1700` | Hồi HP |
| 8 | `apply_revive` | `runner.rs:1740` | Hồi sinh; target phải cùng team |
| 11 | `apply_catch` | `runner.rs:2045` | Bắt pet: điều kiện `!has_npc && typ∉{2,4} && npc.bat==0 && type3_id==0 && lv diff ≥5 && hp%<50`; roll `50+round(dlv/2)`; pet (`caster==id_char`) không bắt được; cần slot trống; thành công → `Outcome::PlayerWin` |
| 12 | `apply_flee` | `runner.rs:2107` | Roll `60+dlv+dlvtb`; `skill==14002` luôn thành công. Leader → `PlayerFled` kết thúc trận. Member → gửi `acting`×2 + `clear_pet_cell`×2 + `hide_from_map` + `reposition`, xoá 2 ô, `npc_respawn`, `Out::Fled` |
| 14 | `apply_heal14` | `runner.rs:2184` | Hồi HP+SP |
| 15 | `apply_buff15` | `runner.rs:1985` | Buff/debuff type15 |
| 16 | `apply_dispel16` | `runner.rs:2080` | Gỡ cặp type4 |
| 18 | `apply_cleanse18` | `runner.rs:2118` | Tẩy + hồi |
| 19 | `apply_buff19` | `runner.rs:2218` | Buff/debuff type19 |

10. Sau mỗi hành động: `type4_id==13005` → gửi `troi_end`; attacker chết với skill 20006 → `id_skill=0`.
11. **Frame assembly** (`runner.rs:1015-1084`): nếu `ts.heal > 0` thêm 1 skilling_int HP cuối; `text9 += le16(blocklen/2) + block`; khi `text9` khác rỗng **và** `combo_active == 0` → flush `text_combo + F444 le16 3201 text9` → `text11` (reflect) → `process_kills` → `text_troi` → nếu `text_combo` đã dùng → `combo_footer_20007`.
12. `leader_sp_regen` (`runner.rs:2535`): nếu `leader_id_qs > 0`, mọi ô player trên row của leader + pet tương ứng hồi `round((QS.Int+Int2)/15)` SP.

### 3.5 Targeting (`targeting.rs`)

- `get_pos_attack_by_fight_area` (`:33`) — 8 vùng: 1 đơn, 2 cột (2 ô, ưu tiên `row-1`), 3 hàng ngang ≤3 ô, 4 nhảy (`row±2`), 5 chữ thập, 6 cột 4 ô, 7 hàng 5 ô, 8 toàn bộ phe đối (`is_heal` → phe mình).
- `pick_targets` (`runner.rs:1172`) chọn nhánh: binary skill → `get_pos_attack_by_fight_area`; combo active → `get_pos_attack_combo`; `type 8` → ô đồng đội; `type 17` → chính mình; `type 3/15` → `get_pos_attack_3_15`; `type 4/6/7/14/19` → `get_pos_attack_type4` (với bảng area riêng cho skill `11009/11010/11026/11030`); hôn loan (`14021/20014`) → `get_pos_attack_hon_loan`; multi (`10016/11016/12016/13042/13015` hoặc `type 18`) → `get_pos_attack_tg`; `type 5/16/17/18` → `get_pos_attack_giai_tru` (ghi đè).
- Helpers khác: `excluded_type4`, `pick_anchor`, `expand_sl_danh`, `is_valid_target` — `pick_anchor`/`expand_sl_danh`/`is_valid_target`/`get_pos_attack`/`excluded_type4` **không được `runner.rs` gọi** (dead hoặc chỉ test).

### 3.6 Damage formula (`combat_formula.rs:42`)

```text
hit_rate = clamp(90 + (attacker.agi - target.agi)/2, 5, 99)
roll >= hit_rate → miss
base:
  attribute == 27 (INT) → ((int1*0.5 + lv) as i64) * numerical/100 + lv*2
  numerical==100 && attribute==0 → ((atk*0.6 + lv) as i64) + lv*2          // basic attack
  else → ((atk*0.6 + int1*0.1 + lv) as i64) * numerical/100 + lv*2
base clamp MAX_BASE_DAMAGE = 50_000
reduced = base - target.lv - target.def
damage = round(reduced
             * element_multiplier(skill.element, target.thuoctinh)   // 1.5 / 0.75 / 1.0
             * (1 + 0.1*max(attacker.reborn - target.reborn, 0))
             * (0.9 + rand(0,10000)/10000 * 0.2)
             * (thunder ? 1.5 : 1)      // thunder = rand(0,100) < clamp(5+agi/10, 1, 50)
             * (target.typ ∈ {2,4} ? 0.5 : 1.0)   // PvP
             * (target.type3_id == 6 ? 1.5 : 1.0)  // ice bound
             * (target.id_skill == 17001 ? 0.5 : 1.0))  // defend
             .max(1.0)
status (nếu skill.hit_status>0 && round>0): chance = clamp(50 + (atk.int1 - tgt.int1)/2, 10, 90)
```

Roll trong `damage.rs`: `randomize_with_percent` clamp `[0,100]`, so sánh `next_range(1,1000) <= p*10`; `get_random_miss_attack` (`100 + round(dlv/10) + round(dlvtb/10)`), `get_random_miss_troi` (`30 + max(int1,atk1)/30 - spx2/30 + round(dlv/20) + round(dlvtb/20) + 5*(rb1-rb2)`), `get_random_miss_flee` (`60 + dlv + dlvtb`), `get_random_miss_combo` (luôn 1), `get_random_drop` (band `[25,23,20,4,3,1]`), `get_random_skill_npc`, `get_turn` (bảng 6 nhóm + inner 14021/14013). `calc_kill_exp`/`calc_combo_exp`/`get_random_drop_slot` **không được gọi** (`reward_exp` riêng ở `runner.rs:2947` dùng bảng level_modifier 1.2/1.0/0.7/0.3).

### 3.7 RNG (`rng.rs`)

- `DotNetRandom` — bản sao thuật toán `System.Random` .NET (mảng 56 phần tử, Knuth subtractive), `next_range(min,max)`, `next_max`, `time_seeded`.
- `BattleRng` có **4** stream (doc §6.0 nói 3): `random_0` (drop/skill pick/randomize_array/combo roll/troi roll), `random_1` (tie-break `_Random`), `random_2` (respawn coords), `random_damage` (**thêm**, dùng cho `calculate_attack` + gold roll — tách riêng để giữ thứ tự draw của drop/NPC-AI). `NpcWorld.random_3` là stream thứ 5 cho wander.
- **Rủi ro parity**: `runner.rs:1341` và `:1567` tạo `DotNetRandom::time_seeded()` **mới** cho type15 `10016..10019` → phá vỡ tính tái lập của battle seed (golden replay không tái tạo được). Cũng `DropNetRandom` non-deterministic.

---

## §4 Skill / combo / chain skill

**CÓ cơ chế combo**, là cơ chế *frame-prefix + damage multiplier*, không phải chain-skill đệ quy.

| Thành phần | File:dòng | Chi tiết |
|---|---|---|
| Combo state | `runner.rs:245 TurnState` | `combo_active: i64`, `combo_hi`, `combo_lo`, `delay`, `combo_cells: Vec<String>`, `text_combo`, `last_target` |
| Điều kiện detect | `runner.rs:1098 detect_combo` | Ô kế tiếp trong `turn_order` phải: `id>0`, **cùng team**, `skill_type == 1`, `row_attack/col_attack` **giống hệt** attacker, target còn sống. Chỉ chạy khi `idx+1 < sorted.len()`. Lần đầu: `|agi_khác| ≤ 800` → set `combo_hi/lo`. Lần sau (3+ đơn vị): `|agi - mid| ≤ 800`. Mỗi lần roll `get_random_miss_combo` (percent 100 → luôn 1), `delay = max(delay, next_delay)`. Nếu `combo_active==1 && text10.is_empty()` → `num37 *= 1.3` + `add_combo_cell` cho cả hai ô. Cuối cùng (không match) → `combo_active = 0` |
| Kế thừa | `runner.rs:801` | `if ts.combo_active == 1 { num37 *= 1.3 }` — entity sau cũng nhân 1.3 |
| Target theo combo | `runner.rs:1197` | `combo_active==1` → `targeting::get_pos_attack_combo` thay vì `get_pos_attack_default` |
| Frame prefix | `packets.rs:102 combo` (dùng ở `runner.rs:1533`) | `F444130032010F00{row}{col}264E0101{row}{col}010301E0000000` — **chỉ set trong `apply_damage_to_rear`** (shield 20006), không set trong `detect_combo` |
| Frame footer | `packets.rs:243 combo_footer_20007` (dùng ở `runner.rs:1082`) | Gửi sau `process_kills` khi `text_combo` đã được dùng; mã skill `le16(20007)` |
| Combo drop/exp | `runner.rs:2694-2728` | Mỗi combo cell (`row.col/lv`) nhận `reward_exp` và **bản sao drop item** (mỗi người 1 item) |
| Chain skill theo diễn biến | **KHÔNG có** | Không có cơ chế `skill → kích hoạt skill khác` / `ComboField` chuỗi. `skill.combo` (`combo_field` từ `skills.rs:787`) được đọc vào biến nhưng **không dùng** (`apply_physical:1301` `let _ = (combo_field, ...)`) |

Kết luận: server có *combo (same-target multi-actor)* với hệt số `×1.3` và frame prefix/footer riêng, nhưng **không có chain/combo skill theo danh sách** (`Skill.combo` bị bỏ qua).

---

## §5 Phần thưởng sau trận

### 5.1 EXP

- Tích lũy trong trận: `WarInfo.exp` — `note_npc_hit` (`runner.rs:2616`) cộng `hit_exp(attLv, npcLv)` khi đánh trúng type 3/7 (nếu chết cộng full, sống cộng `round(exp/10)`), chỉ khi `attLv - npcLv ≤ 20`. `process_kills` (`runner.rs:2689`) cộng thêm `reward_exp` cho attacker, và cho từng combo cell.
- `reward_exp` (`runner.rs:2947`): `floor(npcLv * 10 * level_modifier)` min 1; modifier 1.2 (npc ≥ lv+5), 1.0 (±5), 0.7 (≥ -10), 0.3 (nhỏ hơn).
- Lúc kết thúc (`runner.rs:2780 finish`): chỉ xét ô `typ==2` ở **row 3** (`for col in 0..5 { cell(3, col) }`) → **chỉ cộng exp cho người chơi ở row 3; row 2 bị bỏ qua** (may mà `add_player` luôn dùng row 3, nhưng `add_player` có tham số row nên đây là giả định ngầm). Điều kiện: `!fled && cell.hp > 0 && cell.lv < 200 && exp_raw > 0`. `reborn == 2` → exp `banker_round(exp/2)`.
- Level-up: `texp_get_lv_up(texps, lv, reborn, new_texp)` → nếu `lv_ups > 0` phát `Out::Db` cho `Lv`, `Hpmax`, `Hp(=hpmax)`, `Spmax`, `Sp(=spmax)`, `Point(+2/lv)`, `SkillPoint(+1/lv)`. Hpmax/Spmax dùng `get_hp_max/get_sp_max(reborn, job, new_lv, hpx/spx) + hpx2/spx2` (snapshot lúc spawn).
- `per_exp` mặc định **1** (`service.rs:626`) — chưa đọc từ config.

### 5.2 Item / drop

- `get_random_drop` (band 25/23/20/4/3/1 trên `next_range(1,1000)`, `runner.rs:2655`) → `Out::Drop{item_id, npc_row/col, row/col, owner}` + frame `35 04`.
- `BattleSinkImpl::apply_drop` (`service.rs:157`) — tạo `InventoryItem{ id, count:1, loai:1, doben:100 }`, `s.add_homdo_item(...)`. **Không persist DB ngay**; persist xảy ra trong `battle_ended` qua `persist_sessions_transaction(pool, &refs, &["stats","pet"])` — mà `persist.rs:264` **bỏ qua tham số `_tables`** và gọi `repo.save(session)` (session đầy đủ → bao gồm homdo). Vậy drop có persist, nhưng **không theo scope `tables`** như tên hàm gợi ý.
- Combo: mỗi combo cell nhận 1 bản sao drop riêng.

### 5.3 Pet exp

- `finish` (`runner.rs:2864`): với mỗi player, lặp cols `[0,1,3,4]` của row 2 → `Out::PetExp{ owner, stt: pet.id_npc_on_map, exp: per_exp * pet.exp }` nếu `exp > 0` và `!fled`.
- `apply_pet_exp` (`service.rs:266`) cộng vào `pet.texp` (session + global). **Không có `DbUpdate` và không tính pet level-up** — pet không lên cấp trong battle.
- Lưu ý: cột pet 2 (pet của leader) **bị bỏ qua** vì danh sách `[0,1,3,4]` — nhất quán với `load_leader_pets` đặt pet ở cols `[1,3,0,4]` (không có col 2), nhưng doc comment nói "cols 0,1,3,4 → stt = active..+3" khớp.

### 5.4 Gold

- `process_kills:2656` — `gold_reward = npcLv * random_damage.next_range(2,6)`, chỉ khi không combo và `owner > 0` → `Out::Gold`. `apply_gold` (`service.rs:284`) cộng `session.gold`. **Không có frame nào báo client** về gold trong battle (gold push qua `F444…1A` money sync là việc của handler khác — không có ở đây).

### 5.5 Pet catch

- `apply_catch` thành công → `Out::Catch{owner, npc_id}` → `apply_catch` (`service.rs:180`) `pet_box::add_caught(&mut s.pets, npc_id, hp_max)` với `hp_max = get_hp_max(0,0,1,0)` (hằng số, không theo level thật). Battle kết thúc `PlayerWin`.

### 5.6 Persist sau trận

| Thời điểm | Nơi | Scope |
|---|---|---|
| Fled (1 người) | `service.rs:234` | `persist_sessions_transaction(pool, &[session], &["stats","pet"])` |
| `battle_ended` — mọi member | `service.rs:385` | `&["stats","pet"]` |
| `battle_ended` — quest win (per player) | `service.rs:456` | `&["quest","homdo","trangbi"]` |

`persist_sessions_transaction` bỏ qua `_tables` → thực tế **luôn full-session save**. Không có transaction rollback nếu một player fail (`return false` sau `warn!`).

### 5.7 Quest progression

- `battle_ended` (`service.rs:301`): nếu `Outcome::PlayerLose` → `quest_lose_frames` cho mọi session có `talking_battle > 0`, rồi `return` (bỏ qua nhánh win). Nếu win → `battle_quest_win` cho session có `talking_battle > 0` (Eve-scripted session đã resume ở block trước nên không có `talking_battle` → gate bỏ qua).
- Eve chain: `resume_eve_after_battle` (`handlers/talk.rs:326`) đặt `battle_result` 1/2/3, chuyển phase → `Executing`, gọi `execute_event_step`; nếu trả `next_battle` thì `service.start_eve_encounter` (qua `eve_service` Weak backref, `service.rs:911`).

---

## §6 Frame battle client CẦN nhưng server CHƯA gửi

| Nhóm | Frame | Doc | Vì sao cần |
|---|---|---|---|
| **0x33** | `33 01` + record 7 byte `[grp][slot][word 2B][n][b6][b7]` (`FUN_0064cae0`) | opcode_33.md §3 | Client có callee parse lưới trận 4×5; server **không gửi opcode 0x33 nào cả** (grep `0033` trong `src/` chỉ trúng comment damage.rs). Nhiều khả năng là kênh "unit state/damage-lite" mà engine hiện dùng `32 01` để thay |
| **0x35 SubOp 0x03** | `[35][03][grp≤3][slot≤4]` | opcode_35.md SubOp 3 (đã phân tích đầy đủ) | Ép unit ở ô chuyển **state 7 (hoạt ảnh di chuyển/tấn công)** + xả 21 khe FX. Server **không gửi** → client có thể không render animation tấn công đúng |
| **0x35 SubOp 0x06** | `[35][06][k][id LE32][tpl LE32]` (11 B) | SubOp 6 | Tạo `THuman` + toast tên NPC. Server chỉ gửi `0B 05 03` khi mở trận, không tạo human khi NPC chết/xuất hiện giữa trận |
| **0x35 SubOp 0x07** | `[35][07][srcGrp][srcSlot][dstGrp][dstSlot]` (6 B) | SubOp 7 | **Chuyển ô trên lưới**. Server chỉ có `0B 01` reposition 3 byte (và thiếu byte) — không có frame di chuyển đơn vị đúng layout 6 byte |
| **0x35 SubOp 0x0C** | `[35][0C][grp][slot]` | SubOp 0x0C | Set `unit+0x5BD = 1` (cờ trạng thái sau hành động) |
| **0x35 SubOp 0x0E** | `[35][0E][grp][slot][w LE16]` | SubOp 0x0E | `unit.VMT+0x1C(unit, w, 1)` — gọi virtual method trên unit; có thể là set HP/exp hiển thị |
| **0x35 SubOp 0x0F** | `[35][0F][grp][slot][mode 'B'..'G'][byte][w LE16]` (8 B) | SubOp 0x0F | Ghi 6 cặp `(byte, Word)` vào `unit+0x5BE…0x5CE` + `FUN_0058f5e0` — frame **hiển thị trạng thái chi tiết** (nhiều khả năng là thanh HP/buff icon). Server **không gửi** → thanh máu đơn vị có thể không cập nhật |
| **0x35 SubOp 0x08 / 0x09** | `[35][08][w1]`, `[35][09][w1][w2]` | SubOp 8/9 | Ghi Word vào `gvar_007DA7BC + 0x62E / 0x630 / 0x632` — nhiều khả năng là **tiến trình/nhiệm vụ trong trận** |
| **0x35 SubOp 0x0A** | `[35][0A]([len][text][w])×3` | SubOp 0x0A | Ghi 3 chuỗi vào `gvar_007D9D34 + 0x53DB` — có thể là nhãn tên hiển thị trên lưới |
| **0x35 SubOp 0x0B** | `[35][0B]([type 1..3][len][text])×≤21` | SubOp 0x0B | Đẩy chat/thông báo trong trận (prefix `0x75C6A8/B8/C8`) |
| **0x35 SubOp 0x0D** | `[35][0D][id LE16][count][flag]` | SubOp 0x0D | Ghi `+0x1505 = id`, `+0x1507 = count` và cập nhật index item/bag + banner. Server gửi drop bằng `35 04` (itemId + 4 byte toạ độ) — **không có frame cập nhật số lượng item trong túi** |
| **0x32 SubOp 0x02** | `32 02` + record 7 byte (`FUN_0064dd74`) | opcode_32.md §4.3 | Client có callee riêng cho sub 2; server **chỉ gửi `32 01`**. (Ghi chú: client thân này không dùng field nào — có thể bỏ qua an toàn) |
| **0x0B SubOp 0x01 đúng layout** | `[0B][01][b1 0..3][b2 0..4]` — callee đòi ≥4 byte | opcode_0b.md §3 SubOp 1 | Server gửi `F44405000B01{row}{col}00` (reposition) và `F44404000B01{r}{c}` (clear pet) = 3 byte. Cần thêm byte để khớp guard |
| **0x0B SubOp 0x03** | `[0B][03]` toast 4 loại | SubOp 3 | Server không dùng; có thể dùng cho thông báo "rớt item", "hết lượt" |
| **0x0B SubOp 0x07 / 0x09** | `[0B][07][b1][b2]`, `[0B][09][a][b]` (2 byte cờ + toast) | SubOp 7/9 | Cờ UI; `09` ghi `PlayerRec+0x1308` và toast số khi `b==1` |
| **0x0B SubOp 0x0B / 0x0C** | `[0B][0B][blob8→double]`, `[0B][0C][blob8→double]` | SubOp 0x0B/0x0C | **Deadline** cho menu (`obj+0x230`). Server **không gửi deadline lượt** → thanh đếm ngược phía client không có nguồn |
| **0x0B SubOp 0x04 (8B record)** | `n=(Len-1)/8`, record `[kind][id 4B][w 2B][val 1B]` | SubOp 4 | Server dùng `0B 04 02` + le32 + 3 tail = không phải stream record 8B |
| **0x0B SubOp 0x00 (record `(id,kind)`)** | `[0B][00][id LE32][kind LE16]` | SubOp 0 | Xoá/reset hiệu ứng theo actor — server dùng `0B 00` + le32 + `0000` (kind=0) chỉ để hide |
| **BattleRole 42B** | `BattleRoleSerializer::serialize` | `protocol/codecs/battle_role.rs:277` | Codec 38B base + appearance đầy đủ (name VISCII, sex/face/hair, color1/2, turn, career, **equip + outfit item id list**) — đây là thứ client cần để render **avatar có trang bị** trong lưới trận. Server dùng `_Packet` 23B (`engine.rs:71`) không có name/appearance/equip → **chưa có frame nào trong battle engine dùng codec này** (grep: chỉ `protocol/reader.rs:164`, `protocol/writer.rs:151` và test). Đây là thiếu sót lớn nhất về mặt hiển thị |

---

## §7 Rủi ro / TODO / code chết

### 7.1 Code chết trong `packets.rs` (không có nơi gọi)

| Hàm | Dòng | Ghi chú |
|---|---|---|
| `battle_open_text` | 18 | Bị `battle_open_leader` thay thế |
| `battle_open_pk_member` | 29 | PK dùng `member_battle_frame` |
| `show_pk_leader_on_map` | 65 | tail `000002` không dùng |
| `turn_action` | 95 | `runner.rs:1059` tự dựng `format!("3201{}")` |
| `stat_byte` module | 293 | Không nơi gọi (`service.rs:543` hardcode `0x19/0x1A/0x24/0x23/0x26/0x25/0x40`) |
| `troi_byte::HOCHU` | 268 | Không dùng |

### 7.2 Hằng số / module không dùng

- `damage.rs`: `calc_kill_exp` (`:187`), `calc_combo_exp` (`:208`), `get_random_drop_slot` (`:160`) — không được `runner.rs` gọi (dead hoặc chỉ test).
- `targeting.rs`: `pick_anchor` (`:160`), `expand_sl_danh` (`:175`), `is_valid_target` (`:277`), `get_pos_attack` (`:283`), `excluded_type4` (`:137`) — không gọi từ `runner.rs`.
- `engine.rs`: `exp_lv_up` (`:9`) — `runner.rs:2812` gọi thẳng `crate::data::texps::texp_get_lv_up`.
- `engine.rs`: `WarInfo.fly_out` set ở `runner.rs:2611` nhưng **không ai đọc** — dead field.
- `construction.rs`: `clear_cell` (`apply_use_item`/`apply_catch`/`apply_flee` dùng), `BattleCounter` (`:529`) — `BattleService` dùng `AtomicI32 next_battle` riêng nên `BattleCounter` **chết**.
- `construction.rs`: `StartPacket::Map` chỉ dùng cho show-on-map; `Battle::with_seeds` chỉ dùng trong `*_seeded` variant (test/golden).

### 7.3 Rủi ro hành vi

1. **Non-deterministic RNG trong turn loop**: `runner.rs:1341` và `:1567` tạo `DotNetRandom::time_seeded()` mới cho type15 `10016..10019` → `with_seeds`/`*_seeded` không còn tái lập được, golden replay vỡ.
2. **`persist_sessions_transaction` bỏ qua `_tables`** (`db/persist.rs:267`) → mọi lời gọi với scope `"stats"/"pet"/"quest"/"homdo"/"trangbi"` đều là full-session save. Tốn I/O, và scope không còn là rào chắn.
3. **`finish` chỉ cộng exp/pet-exp cho row 3** (`runner.rs:2790, 2866`) — hardcode giả định leader row. `add_player(…, row, col)` nhận row bất kỳ → nếu sau này đặt người chơi vào row 2 thì mất exp.
4. **`handle_use_item` trừ item inventory trước khi lệnh được chấp nhận** (`handlers/battle.rs:233`) — nếu `submit_command` trả false (sai ownership theo `owns_commandable_cell`), item đã mất.
5. **Double session-mutation**: mọi `apply_*` ghi cả vào `self.online` lẫn global `lock_online_sessions()` (`service.rs:130-299`) — 2 nguồn state có thể lệch nhau; đặc biệt `apply_drop`/`apply_pet_exp`/`apply_gold`/`apply_catch` cộng 2 lần nếu cùng object (`apply_drop` gọi `add_homdo_item` trên `p.session` rồi lại lần nữa trên global map — nếu global map chứa **cùng clone** thì OK, nhưng nếu là cùng object thì cộng đôi). **Cần verify**.
6. **Retry lock helper dùng `std::thread::sleep` trong async context** (`service.rs:66, 85`) — block tokio worker thread trong battle task (async).
7. **`BattleManager::spawn_timeout` đăng ký handle trong một task riêng** (`manager.rs:203`) → có race window: `join_battle`/`submit_command` ngay sau `spawn` có thể thấy `handles` rỗng (service giữ `handles` riêng nên `submit_command` dùng `self.handles`, còn `battle_exists` cũng dùng `self.handles` — nhưng `manager.battles` thì không dùng ở đâu → **`BattleManager::get/len/is_empty` và `next_id` có thể là dead API**).
8. **`0x0B` sub 6 không gate `battle_id`** (`handlers/battle.rs:24`) → spam show-on-map cho bất kỳ ai.
9. **`0x0B` sub 2 inner 5 (JamPlayerToBattle) đọc rồi bỏ** (`handlers/battle.rs:102-107`) — tính năng chưa làm.
10. **`0x13` sub 2 (thu pet) không gate `battle_id`** (`pet_actions.rs:358`) → có thể set `active_pet_stt = 0` giữa trận, làm hỏng `apply_use_item` (chọn pet theo `active_pet_stt`).
11. **`skill_level_for` fallback `1` và không verify ownership skill** (`handlers/battle.rs:248`) → người chơi có thể gửi skill ID bất kỳ; `runner.rs:766` cố tình tin player (`valid = true`).
12. **`record_join_cells` marker `100` (0x64)** vs `member_battle_frame` dùng chuỗi `"64"` — nhất quán về byte nhưng hai chỗ khác format (`{:02X}` của 100 = `64`, `"64"` literal) → dễ vỡ nếu đổi.
13. **`0B 01` frame thiếu byte** so với doc client (xem §1.1 #13/#14, §6).
14. **`battle_open_leader` hardcode `F4441C`** (`packets.rs:11`) — chỉ đúng khi payload có đúng 1 record 24B; nếu sau này thêm record phải sửa magic.
15. **BattleRole codec 42B không dùng** → client không nhận name/equip/outfit trong trận.
16. **Không có `0x33`, không có `0x35 03/06/07/0C/0E/0F`** → nhiều khả năng thiếu frame hiển thị/anim bắt buộc.
17. **`per_exp` hardcode 1** (`service.rs:626`) — không đọc `Server.PerEXP` từ config.
18. **`Battle::list_qs` (50 slot khán giả) khởi tạo nhưng không ai đọc/ghi** (`construction.rs:36,74`) — dead state (spectate chưa implement).
19. **`attacker.attacked = 2` comment sai** (`runner.rs:1086` ghi "Attacked = 2" nhưng gán `true`).
20. **`_Delay = 10` respawn không lưu toạ độ** (`runner.rs:2758` comment nói rõ) — NPC respawn sẽ random lại mỗi tick sau khi hết cooldown.

### 7.4 Test coverage

- `tests/` không có file battle-specific nào. `random_encounter_test.rs` (1152 dòng) là test gần nhất: assert frame chứa `"0BFA"` cho member, `"0504"/"1408"/"0B00"` cho flee op 0x0B sub 5, và sync drop/exp vào online session. `checkpoint6_parity_test.rs` (512 dòng) **không** test battle (chỉ gold type1/2, skill-point toast, Mark.dat, quest, party warp, TRAC NPC flow).
- Theo `AGENTS.md`, mọi test mới phải ở `tests/`. Đề xuất: `tests/battle_frames_test.rs` (byte-exact từng frame), `tests/battle_combo_test.rs`, `tests/battle_turn_order_test.rs`.

---

## Phụ lục — Bảng tra nhanh file:dòng

| Symbol | File:dòng |
|---|---|
| `Battle::run_turn` | `runner.rs:288` |
| `turn_phase1` / `2` / `4` | `runner.rs:328` / `530` / `673` |
| `execute_entity` | `runner.rs:700` |
| `detect_combo` | `runner.rs:1098` |
| `pick_targets` | `runner.rs:1172` |
| `apply_physical` | `runner.rs:1276` |
| `apply_magic` | `runner.rs:1539` |
| `apply_catch` | `runner.rs:2045` |
| `apply_flee` | `runner.rs:2107` |
| `leader_sp_regen` | `runner.rs:2535` |
| `note_npc_hit` | `runner.rs:2616` |
| `process_kills` | `runner.rs:2634` |
| `npc_respawn` | `runner.rs:2734` |
| `finish` | `runner.rs:2780` |
| `reward_exp` | `runner.rs:2947` |
| `BattleSink` trait | `manager.rs:29` |
| `collect_commands` | `manager.rs:210` |
| `fill_timeout_commands` | `manager.rs:250` |
| `owns_commandable_cell` | `manager.rs:287` |
| `dispatch` | `manager.rs:304` |
| `BattleService::spawn_battle` | `service.rs:1321` |
| `BattleService::start_*` | `service.rs:727,747,776,814,847,923,940` |
| `join_battle` / `build_join_frame` / `leave_battle` | `service.rs:1076` / `1100` / `1129` |
| `submit_command` | `service.rs:1158` |
| `record_join_cells` | `service.rs:1253` |
| `battle_ended` | `service.rs:301` |
| `WarInfo::packet_hex` | `engine.rs:71` |
| `get_hp_max` / `get_sp_max` | `engine.rs:100` / `117` |
| `turn_order` | `construction.rs:310` |
| `npc_battle_start_packets` | `construction.rs:327` |
| `member_battle_frame` | `construction.rs:422` |
| `npc_battle` / `teamdef_battle` | `construction.rs:559` / `576` |
| `handle_battle` (0x0B) | `handlers/battle.rs:15` |
| `handle_battle_command` (0x32) | `handlers/battle.rs:166` |
| `BattleRoleSerializer` | `protocol/codecs/battle_role.rs:273` |