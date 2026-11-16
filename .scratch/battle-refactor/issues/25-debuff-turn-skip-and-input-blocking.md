# 25: Tự động bỏ qua lượt & chặn chọn hành động khi dính Trạng thái Khóa Lượt (Debuff Turn-Skip & Input Blocking)

**What to build:** Hoàn thiện cơ chế tự động bỏ qua lượt và chặn chọn cả 6 hành động chiến đấu (`NormalAttack`, `Defend`, `UseItem`, `CastSkill`, `Capture`, `Flee`) cho Nhân vật và Sủng vật khi đang chịu **Trạng thái Khóa Lượt** (*Khống chế cứng* Type 3 hoặc *Hỗn loạn / Cuồng nộ* `14021 | 20014`), bao gồm từ tầng quản lý vòng đời lượt (`34 01` / thu thập lệnh), tầng tiếp nhận gói `0x32` (chống trừ oan vật phẩm & báo cảnh báo `sys_msg_frame`), tầng thực thi lượt (ưu tiên Type 3 trước `auto_berserk`, sửa điều kiện giảm lượt `id > 0`), đến đồng bộ wire trạng thái xuống client (`35 01` / `35 0F` / `32 01`).

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction, 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01, 21: Chuẩn hóa gói 32 02 turn_action_finish

**Spec:** [spec-debuff-turn-skip.md](../spec-debuff-turn-skip.md)

**Status:** ready-for-agent

## Acceptance Criteria

### 1. Tự động bỏ qua pha chờ lệnh (Turn Lifecycle Auto-Skip)
- [ ] Đầu mỗi hiệp, vẫn phát `34 01` cho mọi người chơi còn sống tham chiến để client giữ đúng nhịp reset cờ lượt.
- [ ] Loại bỏ mọi ô Nhân vật/Sủng vật đang chịu **Trạng thái Khóa Lượt** (*Khống chế cứng* Type 3 hoặc *Hỗn loạn/Cuồng nộ* `14021 | 20014`) khỏi danh sách ô chờ lệnh (`waiting_cells`) và danh sách tự động đánh thường khi hết giờ (`fill_timeout_commands`).
- [ ] Không phát sinh độ trễ nhân tạo: nếu một bên chỉ còn ô bị khóa lượt (ví dụ Player bị đóng băng còn Pet tự do thì chỉ chờ Pet; nếu cả Player lẫn Pet đều bị khóa thì thoát pha chờ lệnh sau `0ms` ngay để vào pha thực thi + `32 02` + Dynamic Skill Delay Pacing).
- [ ] Giữ nguyên thứ tự thu thập lệnh trước thực thi lượt; sử dụng hàm dự đoán nhất quán 1-1 với bước tick đầu hiệp (trừ `1` lượt và giải hạn khi `turn <= 1`) để không khóa nhầm đấu sĩ ở chính lượt trạng thái vừa hết hạn.

### 2. Chặn lệnh `0x32 sub 1/2` đồng bộ & Bảo vệ túi đồ
- [ ] Duy trì snapshot nhẹ tập ô đang bị khóa lượt trên handle trận đấu (cập nhật đầu mỗi hiệp trước khi phát `34 01`) để tầng tiếp nhận kiểm tra O(1) đồng bộ.
- [ ] Mọi gói `0x32 sub 1` (Đánh thường, Phòng thủ, Kỹ năng, Thu phục, Bỏ chạy) và `0x32 sub 2` (Dùng vật phẩm) gửi cho ô đang bị khóa lượt đều bị từ chối đồng bộ:
  - Không đưa lệnh vào hàng đợi thực thi.
  - Không phát frame `35 05` giả ở `sub 1`.
  - Không trừ vật phẩm khỏi túi đồ ở `sub 2`.
- [ ] Khi từ chối `0x32 sub 1/2` do ô đang bị khóa lượt, gửi thông báo hệ thống bằng VISCII 1.1 qua `sys_msg_frame`: `"Đang trong trạng thái khống chế, không thể hành động lượt này!"`.
- [ ] Các **Debuff mềm** (như Bỏ độc `14015`, giảm AGI/sát thương `10016..=10019`, `10025`, `20022`, Thu nhỏ `13011`) không thuộc Trạng thái Khóa Lượt và vẫn cho phép chọn đủ 6 hành động bình thường.

### 3. Thực thi lượt & Ưu tiên trạng thái
- [ ] Khi một đấu sĩ (Nhân vật, Sủng vật hoặc NPC) đồng thời dính **Khống chế cứng** (Type 3) và **Hỗn loạn/Cuồng nộ** (`14021 | 20014`), **Khống chế cứng** được ưu tiên trước để bỏ qua lượt hoàn toàn (không chạy `auto_berserk`).
- [ ] Sửa điều kiện giảm lượt đầu hiệp sang `id > 0` (thay vì `> 1`) để mọi trạng thái có `id == 1` (nếu có từ dữ liệu kỹ năng) không bị kẹt vĩnh viễn.

### 4. Đồng bộ Wire Trạng thái xuống Client
- [ ] Phát frame khởi tạo hiệu ứng trạng thái `35 01` (`troi_start`) khi gán Type 3 / Type 15 trực tiếp lên mục tiêu (tương xứng với nhánh phản đòn `10026`).
- [ ] Khi hiệu ứng phụ Type 3 từ đòn đánh vật lý hoặc phép thuật trúng đích (`inflicted_status > 0`), phát đồng bộ frame trạng thái (`35 0F`, `35 01` và chỉ báo `32 01` Type 3) để client cập nhật UI/khóa menu khớp với server.

### 5. Kiểm thử (`tests/battle/`)
- [ ] Bổ sung integration test trong `tests/battle/` (khai báo tại `tests/battle_test.rs`, tuân thủ cấm `#[cfg(test)]` trong `src/`):
  - Đấu sĩ (Player và Pet) dính Type 3 bị bỏ qua đủ cả 6 hành động (`NormalAttack`, `Defend`, `UseItem`, `CastSkill`, `Capture`, `Flee`); kiểm chứng tương tác ưu tiên Type 3 > `14021 | 20014`; kiểm chứng Debuff mềm vẫn hành động bình thường.
  - Vòng đời lượt `BattleManager`: tự động bỏ qua ô khóa lượt (thoát ngay không chờ 20s timeout), vẫn chờ Pet nếu chỉ Player bị khóa, và tự mở lại quyền nhập lệnh ở chính lượt hiệu ứng hết hạn (`turn == 1` trước tick).
  - Handler `0x32 sub 1/2`: từ chối lệnh ô khóa lượt, không phát `35 05`, không trừ item trong `homdo`, và phát đúng `sys_msg_frame` cảnh báo.
  - Wire parity cho `35 01` ở cast trực tiếp và `35 0F`/`35 01`/`32 01` ở hiệu ứng phụ đòn đánh, cùng case `type3_id == 1` tự hết hạn.
- [ ] `cargo test --test battle_test` và `cargo test --all-targets --no-fail-fast` vượt qua 100%.
