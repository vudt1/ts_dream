# HANDOFF — Tiến độ thực thi `battle-refactor` (01 → 18)

> Ngày cập nhật: 2026-10-17 · Repo: `/mnt/d/VUDT/GIT_PCC/ts_dream` (branch `ss2`) · Toàn bộ thay đổi **chưa commit** (người dùng tự commit).
> Đọc kèm: [`spec.md`](spec.md) (kế hoạch + thứ tự phụ thuộc), [`HANDOFF-client-reverse.md`](HANDOFF-client-reverse.md) (bằng chứng ASM byte-exact), [`RESEARCH-two-corrections.md`](RESEARCH-two-corrections.md) (nghiên cứu hai đính chính), [`issues/`](issues/) (18 ticket).

## 1. Bối cảnh phiên làm việc

- Mục tiêu: hoàn thiện giao tiếp frame trận chiến `ts_dream` (server Rust) ↔ `aLogin.exe` (client TS Online).
- Đợt 1 (01 → 15): đã hoàn thành 14/14 ticket lập trình. Ticket 14 (capture runtime thật) tạm để lại cho người dùng chạy capture sau (`ready-for-human`).
- Đợt 2 (Nghiên cứu & Đính chính):
  - Đã đối chiếu và làm rõ triệt để **Hai đính chính từ kết quả thực thi** (Ticket 11 & Ticket 12) bằng dữ liệu sơ cấp [`Data/Formula.Dat`](../../Data/Formula.Dat), PE [`client_delphi/aLogin.exe`](../../client_delphi/aLogin.exe), và mã xuất Ghidra [`client_pseudo_c/`](../../client_pseudo_c/).
  - Đã rà soát mã nguồn server C# Bear (`TS_Server_Bear/`) và tài liệu [`opcode_35.md`](../client-pseudo-op-code/opcode_35.md), làm rõ quy tắc timing 4 pha của Opcode `0x35` trên lưới trận $4 \times 5$.
  - Đã bổ sung bộ builder và test cho `35 0E` (`unit_action`), `35 0F` (`battle_unit_status`), `35 01` (`purge_status`).
  - Đã xuất bản 3 ticket mới (16, 17, 18) mở rộng cho thoại NPC, thông báo chat và đấu nối timing vào `runner.rs`.
- Ràng buộc bất biến (từ [`AGENTS.md`](../../AGENTS.md)): mọi test mới nằm trong `tests/`; **cấm** `#[cfg(test)]` trong `src/`; không đụng `DB/ts_dream.db`; **không** `git add`/`git commit`.

## 2. Trạng thái ticket

| # | Ticket | Status | Ghi chú |
|:-:|---|:-:|---|
| 01 | [`issues/01-0b-command-channel.md`](issues/01-0b-command-channel.md) | ✅ completed | Kênh lệnh trận `0x0B` thay `0x32` chết |
| 02 | [`issues/02-33-01-hp-sp-frame.md`](issues/02-33-01-hp-sp-frame.md) | ✅ completed | Builder `33 01` HP/SP unit trong trận |
| 03 | [`issues/03-0b-01-reset-and-35-07-move.md`](issues/03-0b-01-reset-and-35-07-move.md) | ✅ completed | `0B 01` chuẩn 4 byte + builder `35 07` |
| 04 | [`issues/04-17-06-item-frame.md`](issues/04-17-06-item-frame.md) | ✅ completed | Gộp builder `17 06` + `apply_drop` gửi frame |
| 05 | [`issues/05-1a-01-gold-frame.md`](issues/05-1a-01-gold-frame.md) | ✅ completed | `1A 01` cộng vàng + banner |
| 06 | [`issues/06-05-0a-levelup-and-maxhp-frames.md`](issues/06-05-0a-levelup-and-maxhp-frames.md) | ✅ completed | `05 0A` LevelUP + `08 01 CD/CE` |
| 07 | [`issues/07-0f-08-pet-exp-frame.md`](issues/07-0f-08-pet-exp-frame.md) | ✅ completed | Sửa `pet_stat_entry` (99+name.len) + `0F 08` |
| 08 | [`issues/08-team-row-rule.md`](issues/08-team-row-rule.md) | ✅ completed | `team_for_row`: `row < 2 → 2` |
| 09 | [`issues/09-finish-generalization.md`](issues/09-finish-generalization.md) | ✅ completed | `finish()` quét ô theo `team`/`typ` |
| 10 | [`issues/10-rng-determinism.md`](issues/10-rng-determinism.md) | ✅ completed | Bỏ `time_seeded()`, thêm stream `random_skill` |
| 11 | [`issues/11-exp-and-hp-formula-parity.md`](issues/11-exp-and-hp-formula-parity.md) | ✅ completed | `texps.rs` + MaxHP/SP khớp ASM & Formula.Dat |
| 12 | [`issues/12-35-03-builder.md`](issues/12-35-03-builder.md) | ✅ completed | Builder `35 03` (đính chính dispatcher Case 46) |
| 13 | [`issues/13-cleanup-dead-code.md`](issues/13-cleanup-dead-code.md) | ✅ completed | Xoá dead code + magic `F4441C` |
| 14 | [`issues/14-capture-verification.md`](issues/14-capture-verification.md) | ⏸ ready-for-human | **BỎ QUA** — người dùng tự capture runtime thật sau |
| 15 | [`issues/15-0b-fa-self-kind-04.md`](issues/15-0b-fa-self-kind-04.md) | ✅ completed | Marker self `0B FA` = `04` |
| 16 | [`issues/16-35-06-battle-npc-talk-builder.md`](issues/16-35-06-battle-npc-talk-builder.md) | ⏳ ready-for-agent | Builder `35 06` thoại NPC / banner trận |
| 17 | [`issues/17-35-0b-battle-chat-notice-builder.md`](issues/17-35-0b-battle-chat-notice-builder.md) | ⏳ ready-for-agent | Builder `35 0B` chat / thông báo trận đấu |
| 18 | [`issues/18-battle-0x35-timing-wiring.md`](issues/18-battle-0x35-timing-wiring.md) | ⏳ ready-for-agent | Đấu nối timing `0x35` (`35 03`, `35 0F`) vào `runner.rs` |

**Tiến độ hiện tại**:
- 14 ticket đợt đầu đã hoàn thành (01–13, 15).
- 3 ticket đợt mới (16–18) đã xuất bản, sẵn sàng thực thi (`ready-for-agent`).
- Ticket 14 giữ nguyên cho người dùng chạy capture runtime.

## 3. Thay đổi mã nguồn đã hoàn thành

### File source đã sửa
- `src/battle/packets.rs`:
  - `33 01` (`set_unit_prop`/`set_unit_prop_frame`/`set_hp`/`set_sp`).
  - `0B 01` chuẩn 4 byte (`reset_cell`, xoá `reposition`/`clear_pet_cell`).
  - `move_cell` (`35 07`).
  - `force_state7` (`35 03`, 4 byte payload `[35][03][grp][slot]`).
  - `unit_action` (`35 0E`, 6 byte payload `[35][0E][grp][slot][val_u16]`).
  - `battle_unit_status` & `clear_unit_status` (`35 0F`, 8 byte payload `[35][0F][grp][slot][mode 'B'..'G'][flag/turns][val_u16]`).
  - `purge_status`, `buff_end`, `buff_start_caster` (`35 01`).
  - `gold_gain` (`1A 01`), `level_up` (`05 0A`), `drop_reward` (`35 04`).
  - `battle_open_leader` marker self `03` → `04` (LEN vẫn `0x1C`).
- `src/battle/construction.rs` — 3 call-site dùng `team_for_row`; `member_battle_frame` marker self `05` → `04`.
- `src/battle/engine.rs` — helper `team_for_row(row) -> i64` (`row < 2 => 2`, ngược lại `1`); `get_hp_max`/`get_sp_max` re-derive khớp `FUN_0065a554`/`FUN_0065ad10`.
- `src/battle/rng.rs` — stream tất định `BattleRng::random_skill` (seed `s0 + 12953`).
- `src/battle/runner.rs` — call-site `reset_cell`; `push_level_up_rewards(...)`; `finish()` quét ô theo `team`/`typ`; bỏ `time_seeded()` ở skill 10016..10019; `drop_reward` (`35 04`).
- `src/battle/service.rs` — `apply_gold` gửi `1A 01`; `apply_drop` gửi `17 06`; `apply_pet_exp` gửi `0F 08`; `Stat::Hpmax/Spmax → 0xCD/0xCE`.
- `src/battle/manager.rs` — doc `op 0x32` → `0x0B`.
- `src/server/dispatcher.rs` — bỏ nhánh `OP_BATTLE_COMMAND (0x32)`.
- `src/server/handlers/battle.rs` — `0x0B` là kênh lệnh duy nhất; parse sub 2/7/8/9/0A + log.
- `src/server/inventory.rs` — builder chung `item_added_frame` (`17 06`, 14 byte).
- `src/server/spawn.rs` — `pet_stat_entry` sửa layout (`le16(id)` + `le32(texp)`, tail 6 byte); thêm `pet_stat_frame`.
- `src/protocol/codecs/battle_role.rs` — **xoá hẳn**; `ehuman` chuyển sang `src/protocol/codecs/ehuman.rs`.
- `src/data/texps.rs` — viết lại: `TEXP_K = [2.9, 3.0, 3.05, 3.0]` (reborn 0 = 2.9 từ `Formula.Dat`), cột `base[lv]` tách riêng (`TEXP_BASE_DEFAULT = 5`), rounding half-away-from-zero.
- `src/data/tables.rs` — `TexpRow.reborn` `[i64; 3]` → `[i64; 4]`.

### Bộ test tập trung trong `tests/`
Toàn bộ test mới:
`battle_command_channel_test.rs`, `battle_frames_test.rs`, `battle_cell_frames_test.rs`, `battle_item_frame_test.rs`, `battle_gold_frame_test.rs`, `battle_levelup_frames_test.rs`, `pet_stat_frame_test.rs`, `pet_recall_13_02_test.rs`, `battle_team_row_test.rs`, `battle_finish_generalization_test.rs`, `battle_rng_determinism_test.rs`, `texps_formula_test.rs`, `max_hp_sp_formula_test.rs`, `battle_state7_frame_test.rs`, `battle_status_frames_test.rs` (6 test mới cho 35 0E/0F/01), `ehuman_public_test.rs`, `battle_open_leader_test.rs`, `battle_open_self_kind_test.rs`.

```bash
cargo test --all-targets --no-fail-fast    # 311 passed / 0 failed (55 test binaries)
```

## 4. Tổng hợp phân tích làm rõ (Deep Research Findings)

### 4.1 Hai đính chính từ kết quả thực thi (Đã đóng)
1. **Ticket 11 — EXP Formula & `+5`**:
   - `FUN_0065b0dc` nhánh Reborn 0 đọc `FLD double [EAX+0x80]` với `EAX = *(*gvar_007DA6B8)+4` — trỏ thẳng vào payload `Data/Formula.Dat` slot `w28` = **`2.9`**.
   - Cả 4 nhánh Reborn đều đọc `[EAX+0xA8]` — slot `extra_exp` = **`5`**.
   - `k = 2.9` và `+5` là hằng số gốc của game, không phải phỏng đoán. Tài liệu `spec.md` và `HANDOFF-client-reverse.md` đã được đính chính.
2. **Ticket 12 — Dispatcher Opcode 0x35 vs 0x3C**:
   - Dispatcher `FUN_0078a89c` dùng bảng tra 2 tầng: `byte[opcode + 0x78A8EE]` → Case Index → Jump Table `0x78A9B6`.
   - `byte[0x35] = 46` $\rightarrow$ `FUN_007952ab` (Dispatcher 0x35 thật, xử lý sub 1, 3, 4, 5, 6, 7, 8, 9, 0xA..0xF).
   - `byte[0x3C] = 53` $\rightarrow$ `FUN_0079575c` (Opcode 0x3C, xử lý sub 1..4).
   - Nhận định cũ cho rằng `35 0E`/`35 0F` là no-op là do nhầm Case 53 thành Opcode 0x35. Đã khôi phục đầy đủ handler cho `35 03` (`FUN_0064cfec`), `35 0E` (`FUN_0064fbd8`), `35 0F` (`FUN_0064fd34`).

### 4.2 Phân tích server C# Bear (`TS_Server_Bear/`)
- Bear server dùng các opcode theo số thập phân (`53` = `0x35`, `50` = `0x32`).
- Bear **chỉ sử dụng 4 sub** của `53`:
  - `53 05` (`acting` - `{53, 5, row, col}`).
  - `53 01` (`purge_status` - `{53, 1, 0, 0, type, 0, 0}`).
  - `53 04` (`giveDrop` - `{53, 4, item, init, dest}`).
  - `53 06` (`npcTalk` - `{53, 6, 1, 0, 0, 0, 0, dialog, 0, 0}`).
- Bear **hoàn toàn KHÔNG CÓ** `35 03` (`force_state7`), `35 07` (`move_cell`), `35 0E` (`unit_action`), `35 0F` (`battle_unit_status`).
- Toàn bộ diễn hoạt animation và hiệu ứng buff/debuff trong Bear được nhồi chung vào **`32 01` (`50 01`)** (`BuildExecutionPacket`), sau đó `Thread.Sleep()`.
- Hạn chế của Bear: client không có icon buff/debuff và không có tooltip số hiệp trên HUD. `ts_dream` bổ sung `35 0F` đem lại trải nghiệm hoàn chỉnh và vượt trội hơn Bear.

### 4.3 Quy tắc timing 4 pha của Opcode 0x35 (`opcode_35.md`)
1. **Pha 1 (Chọn lệnh / Đầu hiệp)**:
   - `35 05`: Báo ô đang thao tác / chọn lệnh.
   - `35 01` + `clear_unit_status` (`35 0F` flag=0): Xóa sprite và icon khi buff hết thời hạn.
2. **Pha 2 (Khởi động đòn đánh / Pre-action)**:
   - `35 03`: Bắn trước đòn đánh để gọi `FUN_00651824` xả sạch 21 khe FX cũ và ép unit sang State 7 (thủ thế/lao lên).
   - `35 07`: Dời ô lưới $4 \times 5$ nếu có kỹ năng dịch chuyển.
3. **Pha 3 (Thi triển chiêu thức / Action Impact)**:
   - `32 01`: Hoạt ảnh kỹ năng, sát thương, hiệu ứng sprite.
   - `35 0F`: Bắn ngay sau đòn đánh nếu dính buff/debuff để bật icon HUD ('B'..'G') và số hiệp.
   - `35 0E`: Kích hoạt action vtable của unit nếu có.
4. **Pha 4 (Dọn dẹp & Thu thưởng / Post-turn)**:
   - `35 04`: Diễn hoạt item rớt bay từ ô NPC sang ô người chơi.
   - `35 06`: Banner thoại NPC trong trận.
   - `35 0B`: Đẩy chuỗi chat chiến đấu vào `TTalkMsgForm`.

### 4.4 Rà soát 14 SubOp của Opcode 0x35 trong `ts_dream`
- **7 SubOp cốt lõi trên lưới trận**: `35 01`, `35 03`, `35 04`, `35 05`, `35 07`, `35 0E`, `35 0F` $\rightarrow$ **`ts_dream` đã có builder và test bao phủ 100%**.
- **7 SubOp phụ trợ (ngoại vi / chat / banner)**: `35 06` (thoại NPC), `35 08/09` (ghi player word), `35 0A` (registry), `35 0B` (chat trận), `35 0C` (cờ 5BD), `35 0D` (banner vật phẩm) $\rightarrow$ Đã lên ticket cho các sub hữu ích (`35 06` và `35 0B`).

---

## 5. Danh sách các việc chuẩn bị làm (Upcoming Tasks)

### Frontier (Sẵn sàng thực hiện ngay):
1. **Ticket 16** ([`issues/16-35-06-battle-npc-talk-builder.md`](issues/16-35-06-battle-npc-talk-builder.md)):
   - Xây dựng builder `packets::battle_npc_talk` (payload 11 byte `[35][06][k][actor_id:4B LE][tpl_id:4B LE]`) và helper `battle_npc_dialog`.
   - Viết test trong `tests/battle_status_frames_test.rs`.
2. **Ticket 17** ([`issues/17-35-0b-battle-chat-notice-builder.md`](issues/17-35-0b-battle-chat-notice-builder.md)):
   - Xây dựng builder `packets::battle_chat_notice` (danh sách `[type:1B][len:1B][text:VISCII]`, tối đa 21 record).
   - Viết test round-trip tiếng Việt và kiểm tra prefix.

### Phụ thuộc (Sau khi xong Ticket 16 & 17):
3. **Ticket 18** ([`issues/18-battle-0x35-timing-wiring.md`](issues/18-battle-0x35-timing-wiring.md)):
   - Đấu nối `packets::force_state7` (`35 03`) vào `src/battle/runner.rs` trước mỗi lượt đánh của unit.
   - Đấu nối `packets::battle_unit_status` (`35 0F`) và `clear_unit_status` khi kích hoạt/hết hạn buff.
   - Đấu nối `35 06` cho thoại mở màn trận đánh nhiệm vụ / boss.
   - Viết integration test cho chuỗi frame theo đúng timing 4 pha.

### Người dùng thực hiện sau:
4. **Ticket 14** ([`issues/14-capture-verification.md`](issues/14-capture-verification.md)):
   - Chạy capture runtime thực tế để xác thực chính xác mili-giây độ trễ giữa `35 03` và `32 01` trên server chính thức.

---

## 6. Suggested skills cho phiên kế tiếp

- **`tdd`** — Thực hiện Ticket 16 và 17 theo chu trình test-driven: viết test assert byte-exact trước trong `tests/battle_status_frames_test.rs`, sau đó viết builder trong `src/battle/packets.rs`.
- **`reverse-engineer-anything`** — Tham chiếu thêm mã xuất Ghidra của `FUN_0064e064` (`35 06`) và `FUN_0075c518` (`35 0B`) nếu cần xác định thêm các template literal hay prefix đặc thù.
- **`handoff`** — Tiếp tục cập nhật file này sau khi hoàn thành từng ticket 16, 17, 18.

---

## 7. Nguồn tham chiếu

| Nội dung | Đường dẫn |
|---|---|
| Báo cáo nghiên cứu 2 đính chính | [`RESEARCH-two-corrections.md`](RESEARCH-two-corrections.md) |
| Kế hoạch tổng thể & phụ thuộc | [`spec.md`](spec.md) |
| Bằng chứng ASM byte-exact | [`HANDOFF-client-reverse.md`](HANDOFF-client-reverse.md) |
| Chi tiết phân tích Opcode 0x35 | [`.scratch/client-pseudo-op-code/opcode_35.md`](../client-pseudo-op-code/opcode_35.md) |
| Chi tiết phân tích Opcode 0x32 | [`.scratch/client-pseudo-op-code/opcode_32.md`](../client-pseudo-op-code/opcode_32.md) |
| Mã nguồn server C# Bear | [`TS_Server_Bear/TS_Server/Server/`](../../TS_Server_Bear/TS_Server/Server/) |
| Danh sách 18 ticket | [`issues/`](issues/) |
