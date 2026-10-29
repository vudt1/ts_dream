# HANDOFF: Chuẩn hóa Giao thức & Vòng đời Trận đấu (Battle Refactor V2)

- **Ngày cập nhật**: 2026-10-26
- **Dự án**: `ts_dream` (Rust game server cho TS Online PC `aLogin.exe`)
- **Tập tin lưu**: `.scratch/battle-refactor/handoff-battle-refactor.md` (bản sao tại `/tmp/handoff-battle-refactor.md`)
- **Phiên bản đặc tả hiện tại**: `2.0` (tham chiếu [`.scratch/battle-refactor/spec_new.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec_new.md))
- **Theo dõi tiến độ chi tiết**: [`.scratch/battle-refactor/HANDOFF-implementation-progress.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-implementation-progress.md)

---

## 1. Tóm Tắt Tiến Độ: ĐÃ HOÀN THÀNH 100% ĐỢT 1, ĐỢT 2 & ĐỢT 3 (Tickets 16 → 24)

Toàn bộ 23/24 ticket của đợt tái cấu trúc đã hoàn tất với bộ test suite xanh 100%:
- **Đợt 1 (Protocol Parity & Turn Lifecycle P0)**: 
  - Ticket 19 (kênh lệnh `0x32` & domain enum `BattleAction`)
  - Ticket 20 (barrier handshake `0B FA` & timing mở hiệp `34 01`)
  - Ticket 21 (frame `32 02` `turn_action_finish`)
- **Đợt 2 (Event 0x35 & Animation Timing P1)**: 
  - Ticket 16 (builder `35 06` thoại NPC / banner trận)
  - Ticket 17 (builder `35 0B` battle chat & system notice)
  - Ticket 18 (đấu nối timing 4 pha `0x35` & dynamic skill delay pacing trong `runner.rs` và `manager.rs`)
- **Đợt 3 (Isometric Grid, Combo Rules & Capture Gates P2)**:
  - Ticket 22 (chuẩn hóa giải thuật quét ô AoE Isometric `[0, 1, 2, 4, 3]` theo client Delphi `00652454`)
  - Ticket 23 (quy tắc liên kích Combo: chặn tuyệt đối kỹ năng AoE $\ge 4$ mục tiêu)
  - Ticket 24 (hoàn thiện 8 rào chắn thẩm định thu phục NPC / bắt võ tướng `15001`)

---

### 1.1 Chi tiết các ticket Đợt 3 đã hoàn tất

#### 1. Ticket 22: Chuẩn hóa hình học Isometric ô lưới cho kỹ năng AoE (`targeting.rs`)
- **Mã máy client đối chiếu**: `FUN_00652454` (`00652454_FUN_00652454.c`).
- **Hình học đường chéo Isometric**: Thứ tự cột trên hàng ngang là `0 <-> 1 <-> 2 <-> 4 <-> 3` với `slot 2` là cột trung tâm.
- **Triển khai**:
  - Hằng số `ISOMETRIC_COL_SEQ = [0, 1, 2, 4, 3]` và hàm tra cứu kề cận `isometric_col_neighbors(col)`.
  - Nhánh `3 =>` (Row 3): tâm `2` lấy `{1, 2, 4}`, tâm `1` lấy `{0, 1, 2}`, tâm `4` lấy `{2, 3, 4}`, tâm `0` lấy `{0, 1}`, tâm `3` lấy `{3, 4}`.
  - Đồng bộ nhánh `2` (Trước-Sau `opp = r ^ 1`), nhánh `5` (Chữ thập / Cross), nhánh `6` (2 hàng $\times$ 3 cột), nhánh `7` (Full Row 5) và nhánh `8` (Full Team 10 ô) trong cả `expand_sl_danh` và `get_pos_attack_by_fight_area`.
- **Hồ sơ test**: [`tests/battle_isometric_targeting_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_isometric_targeting_test.rs) (7 tests).

#### 2. Ticket 23: Chuẩn hóa quy tắc hợp kích Combo theo giới hạn mục tiêu AoE (`runner.rs`)
- **Mã máy client đối chiếu**: `FUN_00647164` và `FUN_005ab678`.
- **Quy tắc**:
  - Chỉ đòn đánh thường (`10000`) và các kỹ năng có phạm vi tác dụng $\le 3$ mục tiêu (`nbTarget <= 3` hoặc `fight_area <= 3`) mới được phép tham gia chuỗi Combo.
  - Các kỹ năng AoE diện rộng từ 4 mục tiêu trở lên (`nbTarget >= 4`, ví dụ Loạn kích `13013`, Phi sa `10008`, Hồng thủy `11008`, Thiên băng `10012`) bị vô hiệu hóa Combo hoàn toàn.
- **Triển khai**:
  - Helper `skill_target_count(data, skill_id) -> u8` phân giải từ `BinarySkillDef.fight_area` hoặc `Skill.sl_danh`.
  - Cập nhật `detect_combo` kiểm tra cả đấu sĩ hiện tại và đồng đội kế tiếp; nếu một trong hai có `target_count > 3` thì hủy combo.
  - Kế thừa combo (`ts.combo_active == 1`): reset về 0 nếu đấu sĩ kế tiếp thi triển kỹ năng AoE $\ge 4$.
- **Hồ sơ test**: [`tests/battle_combo_rules_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_combo_rules_test.rs) (4 tests).

#### 3. Ticket 24: Hoàn thiện 8 bước kiểm định thu phục võ tướng (`apply_catch`)
- **Mã máy client đối chiếu**: `FUN_0063cf58` (dòng 362–486 & 631–638).
- **8 Rào chắn thẩm định**:
  1. Chủ tướng hành động: Chỉ Player (`typ == 2 && id_char == 0`) mới được bắt; Pet (`typ == 4`) bị từ chối.
  2. Chênh lệch cấp độ $\le 5$: `target.lv - attacker.lv <= 5`.
  3. Loại thực thể hợp lệ: Cấm bắt player (`typ == 2`), pet đã có chủ (`typ == 4`), khí cụ (`typ == 9`), hoặc `typ == 14`, và `type3_id == 0`.
  4. Cờ `can_capture == true` từ `Npc.dat` (`not_pet == 0`).
  5. Chống trùng lặp: Người chơi chưa sở hữu `template_id` trong 4 ô pet active.
  6. Sức chứa túi sủng vật: Còn ít nhất 1 ô trống (`contains(&0)`).
  7. Kỹ năng Triệu gọi `14026`: Nếu NPC có `kind == 0x16` (chiến mã/chiến tượng), người chơi bắt buộc phải đã học kỹ năng `14026`.
  8. Ngưỡng máu sinh lực: HP hiện tại của mục tiêu phải $< 50\%$ MaxHP.
- **Triển khai**:
  - Bổ sung `pub kind: u8` trên `Npc` và nạp từ `Npc.dat` chunk[15].
  - Bổ sung `pub player_skills: Option<&'a HashMap<i64, Vec<u16>>>` và builder `.with_player_skills(...)` trên `BattleData`.
  - Chuẩn hóa gói tin: khi bắt thành công, gửi `reset_cell` cho ô NPC và phát `Out::Catch`; nếu còn kẻ thù sống thì tiếp tục trận (`Outcome::Running`), nếu hết kẻ thù thì kết thúc thắng (`Outcome::PlayerWin`).
- **Hồ sơ test**: [`tests/battle_catch_verification_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_catch_verification_test.rs) (11 tests).

---

## 2. Trạng Thái Hiện Tại & Công Việc Tiếp Theo

```text
├── [ĐÃ XONG 100%] ĐỢT 1: SỬA LỖI GIAO THỨC & VÒNG ĐỜI LƯỢT ĐẤU (P0)
│   ├── ✅ Ticket 19: issues/19-restore-0x32-command-channel.md
│   ├── ✅ Ticket 20: issues/20-turn-lifecycle-timing-and-34-01.md
│   └── ✅ Ticket 21: issues/21-turn-action-finish-32-02-frame.md
│
├── [ĐÃ XONG 100%] ĐỢT 2: HOÀN THIỆN BIẾN CỐ 0x35 & TIMING DIỄN HOẠT (P1)
│   ├── ✅ Ticket 16: issues/16-35-06-battle-npc-talk-builder.md
│   ├── ✅ Ticket 17: issues/17-35-0b-battle-chat-notice-builder.md
│   └── ✅ Ticket 18: issues/18-battle-0x35-timing-wiring.md
│
├── [ĐÃ XONG 100%] ĐỢT 3: BÀN CỜ ISOMETRIC & GAMEPLAY PARITY (P2)
│   ├── ✅ Ticket 22: issues/22-isometric-grid-aoe-targeting.md
│   ├── ✅ Ticket 23: issues/23-combo-rules-aoe-threshold.md
│   └── ✅ Ticket 24: issues/24-capture-validation-gates.md
│
└── [DÀNH CHO NGƯỜI DÙNG]
    └── ⏸ Ticket 14: issues/14-capture-verification.md (Chạy capture runtime thực tế)
```

---

## 3. Các Ràng Buộc Kỹ Thuật Bất Biến (Non-Negotiable Invariants)

1. **Vị trí Test**: Mọi unit/integration test mới **bắt buộc đặt trong `tests/`**. Tuyệt đối **cấm** `#[cfg(test)]` trong `src/`.
2. **Cơ sở dữ liệu**: Không ghi đè hoặc can thiệp trực tiếp `DB/ts_dream.db`. Mọi test DB dùng in-memory SQLite (`sqlite::memory:?cache=shared`).
3. **Mã hóa chuỗi**: Chuỗi thoại và thông báo gửi client phải mã hóa **VISCII 1.1** (`crate::encoding`).
4. **Git Policy**: **Tuyệt đối không tự ý `git commit`**. Người dùng sẽ commit thủ công.
