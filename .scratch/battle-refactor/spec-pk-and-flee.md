# Spec: Phân loại Trận chiến (`Normal` vs `PKBattle`), Hoàn thiện PvP & Tính năng Chạy trốn (`Flee`)

**Status:** ready-for-agent  
**Nghiên cứu nền tảng:** [research-pk-and-flee.md](research-pk-and-flee.md)  
**Tham chiếu kiến trúc:** [ADR 0006](../../docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md) · [GLOSSARY.md](../../GLOSSARY.md)

---

## Problem Statement

Trong `ts_dream`, hệ thống trận đấu hiện tại chưa phân biệt **Trận chiến Thường (`Normal`)** với **Trận chiến PK (`PKBattle`)**, luồng PvP chưa hoàn chỉnh, và cơ chế **Chạy trốn (Flee)** đang gặp nhiều sai lệch nghiêm trọng so với hành vi chuẩn của `aLogin.exe`:

1. **Chưa tách phân loại `PKBattle` (PvP & PK Võ tướng/NPC)**: Mọi trận chiến hiện dùng chung một nhánh thưởng và xử lý kết thúc. Khi người chơi bật chế độ con trỏ PK trên bản đồ và tấn công NPC (`0x0B 02 03`), server vẫn cho rơi vật phẩm (Drop), cộng Gold, cộng kinh nghiệm Nhân vật/Sủng vật và trừ lượt *Phúc Thần*. Ngoài ra, server còn đọc nhầm 2 byte cuối của gói `0x0B 02 03` (vốn là chỉ số vị trí NPC trên bản đồ `1..=100`) thành *DiaHinh*.
2. **Luồng PvP (`0x0B 02 02`) thiếu đối xứng & dễ lỗi kẹt trận**: Khởi tạo trận PvP mới nạp một mình đối thủ (bỏ quên *Sủng vật* và thành viên tổ đội của cả hai phe), không cấp mã trận và danh sách kỹ năng cho Phe 2 nên lệnh từ Phe 2 bị từ chối, gửi nhầm gói thực thể NPC cho đối thủ, và bước kết thúc trận bỏ qua toàn bộ Phe 2 khiến người chơi Phe 2 không nhận gói thoát trận hay hồi sinh 1 HP khi tử trận. Thiếu các kiểm tra an toàn trên người chơi khi thách đấu (tự PK chính mình, khác bản đồ, cùng tổ đội, mục tiêu là GM, đang quan chiến *ListQS* hoặc đang bày *Sạp hàng*).
3. **Nút Bỏ chạy trên giao diện client bỏ qua vòng đời lượt đấu**: Vì khung mở bàn cờ `0B FA` đang gán dấu `0x04` cho chiến đấu viên thay vì `0x03` (Đội trưởng) / `0x05` (Thành viên), khi người chơi bấm nút Bỏ chạy trên thanh công cụ, client gửi `0x0B 01 03` để thoát ngay lập tức miễn phí thay vì đi qua *Kênh Lệnh Chiến Đấu* (`0x32 01 18001`).
4. **Tính năng Chạy trốn (`Flee`) lỗi khi dùng `14002` và khi thành viên tổ đội thoát trận**:
   - Kỹ năng *Thuật đào tẩu* (`14002`) đang bỏ qua roll xác suất để thắng 100% tuyệt đối; khi trượt lại phát nhầm `skill + 1 = 14003` (*Thuật mua bán* bị động) khiến client không hiển thị đúng thông báo *Chạy trốn thất bại* (`18002`).
   - Khi thành viên tổ đội (hoặc *Sủng vật* của họ) chạy trốn thành công, engine thực thi sau đó ghi đè lại bản sao *Ô chiến trận* cũ lên lưới tạo ra "ô ma" (zombie cell), kích hoạt sai hồi sinh NPC giữa trận, phát trùng khung xóa ô, và không loại người vừa trốn khỏi danh sách người tham gia cuối trận. Ngược lại, một NPC có kỹ năng loại `12` cũng có thể bị nhận nhầm là Đội trưởng để ép kết thúc trận của người chơi.

---

## Solution

1. **Định danh rõ hai phân loại trận chiến (`Normal` vs `PKBattle`)**: Bổ sung phân loại trận chiến tách biệt hoàn toàn với `fight_type` của kịch bản Eve. Mọi trận khởi phát từ `0x0B 02 02` (PvP) và `0x0B 02 03` (PK Võ tướng/NPC — không phụ thuộc công tắc Option PK) đều mang phân loại `PKBattle`; toàn bộ các luồng PvE chuẩn (Eve, đụng độ ngẫu nhiên, *Trận đấu NPC chủ động*, *TeamDef*, 40 NPC, *Vấn Đáp Bắc Tinh Quân*) mang phân loại `Normal`. Sửa `0x0B 02 03` giải mã đúng chỉ số vị trí NPC trên bản đồ (`1..=100`) và dùng *DiaHinh* mặc định `112`.
2. **Áp dụng hạn chế phần thưởng chuẩn cho `PKBattle`**: Trong `PKBattle`, giữ nguyên 100% luật chiến đấu theo lượt, sát thương (gồm hệ số PvP 0.5x), trạng thái, hợp kích, **Thu phục Võ tướng (`15001`)** và hồi sinh NPC tuần tra sau trận; nhưng **cấm hoàn toàn** Drop vật phẩm, Gold, kinh nghiệm Nhân vật, kinh nghiệm *Sủng vật*, và không trừ lượt *Phúc Thần*.
3. **Hoàn thiện PvP Party + Sủng vật hai phe đối xứng**: Nạp đủ Đội trưởng, các Thành viên tổ đội đứng cùng bản đồ và *Sủng vật* của cả hai phe lên lưới (`Phe 1` hàng `3/2`, `Phe 2` đối xứng hàng `0/1`); cấp đủ mã trận, danh sách kỹ năng và khung thực thể người chơi (`0B 05 05`) cho cả hai phe; xử lý dọn dẹp kết thúc trận cho cả hai phe; và bổ sung đầy đủ các chốt chặn hợp lệ trên người chơi ở tầng tiếp nhận thách đấu PK.
4. **Chuẩn hóa `0B FA` (`0x03`/`0x05`/`0x04`) & Kênh Flee qua `0x32`**: Chuyển dấu bản thân trên `0B FA` của chiến đấu viên sang `0x03` (Đội trưởng) / `0x05` (Thành viên) và dành `0x04` riêng cho *ListQS / Người quan sát*. Khóa đường thoát trực tiếp `0x0B 01 03` chỉ dành cho Người quan sát.
5. **Chuẩn hóa công thức Flee & Sửa triệt để lỗi Tổ đội/Zombie Cell**: Giữ nguyên công thức xác suất theo chênh lệch cấp độ (`60 + (lv1 - lv2) + (lvtb1 - lvtb2)`), bỏ cơ chế thắng 100% của `14002` và thay bằng cộng thêm `+20%` với giới hạn `[0..=100]`; luôn phát mã `18002` khi Flee thất bại; khóa thực thể NPC khỏi nhánh Flee người chơi; thống nhất quy tắc thoát cả trận khi Đội trưởng (hoặc *Sủng vật* của Đội trưởng, ở bất kỳ phe nào trong PvP) trốn thành công, thoát cá nhân kèm xóa sạch mọi ô của chủ sở hữu khi Thành viên (hoặc *Sủng vật* của Thành viên) trốn thành công, và đồng bộ danh sách người tham gia trận.

---

## User Stories

1. As a player in PK cursor mode who attacks a map NPC (`0x0B 02 03`), I want the server to treat the match as a `PKBattle` regardless of whether my Option PvP switch is on or off, so that map-NPC PK obeys PK rules without forcing me to toggle PvP vulnerability.
2. As a player in a `PKBattle` (either PvP or PK NPC), I want the battle to preserve full turn-based combat rules (AGI turn order, skills, buffs/debuffs, Berserk, combos, and PvP 0.5x damage scaling), so that fighting mechanics feel identical to normal combat.
3. As a player who defeats an NPC inside a `PKBattle`, I want zero item drops, zero Gold rewards, zero Character EXP, zero *Sủng vật* EXP, and zero *Phúc Thần* charge deduction, so that PK battles cannot be exploited for farming and do not waste my fortune charges.
4. As a player who initiates a `PKBattle` against a wild or map NPC, I want to still be allowed to use *Thu phục Võ tướng* (`15001`), and have roaming map NPCs respawn normally after the fight ends, so that map NPC ecosystems and capture gameplay remain intact.
5. As a player who sends `0x0B 02 03` against a map NPC, I want the server to read the packet's trailing 16-bit value as the map NPC index (`1..=100`) and render a clean default *DiaHinh* (`112`), so that terrain rendering and battle board background never get corrupted by NPC slot numbers.
6. As a player engaging in PvE content (random encounters, Eve quest battles, *Trận đấu NPC chủ động*, *TeamDef*, 40-NPC event, and *Vấn Đáp Bắc Tinh Quân*), I want those matches to always remain `Normal` battles, so that I always earn full Drops, Gold, and EXP even if my PK flag happens to be active.
7. As a player challenging another player to PvP (`0x0B 02 02`), I want the server to validate both parties and block invalid challenges (self-targeting, different map, same party, GM/Angel targets, *ListQS* spectators, or players running a *Sạp hàng người chơi*), so that illegal or griefing PK states cannot start.
8. As a party leader in a PvP challenge with teammates on the same map, I want both my party (with our *Sủng vật*) and the defending leader's party (with their *Sủng vật*) to spawn symmetrically on the battle grid (`Team 1` on rows `3/2`, `Team 2` on rows `0/1`), so that full-party PvP works as intended.
9. As a defending player or teammate on Team 2 in a PvP battle, I want to receive valid battle ID assignment, skill lookup, player entity spawn frames (`0B 05 05`), and full post-battle cleanup (exit frames and 1 HP recovery if defeated), so that I can submit commands normally and return cleanly to the map when the fight finishes.
10. As a combatant (leader or party member) entering any battle, I want the battle-open packet (`0B FA`) to mark me with `0x03` (leader) or `0x05` (member) while reserving `0x04` exclusively for *ListQS* spectators, so that clicking the Run button on `aLogin.exe` queues a real `0x32 01 18001` Flee command into the turn loop.
11. As a *ListQS* spectator watching an ongoing battle, I want to keep marker `0x04` and be allowed to leave immediately via `0x0B 01 03`, while active combatants sending `0x0B 01 03` are rejected, so that fighters cannot bypass turn-based Flee rolls.
12. As a character or *Sủng vật* executing Flee (`18001`), I want my escape probability to be rolled from `60 + (attacker.lv - target.lv) + (ally_lvtb - enemy_lvtb)` clamped to `[0..=100]`, so that level advantage meaningfully governs escape difficulty.
13. As a character casting *Thuật đào tẩu* (`14002`) for 10 SP, I want my escape probability to receive a `+20%` bonus clamped to `[0..=100]` instead of an unconditional 100% auto-win, so that the spell remains advantageous without guaranteeing instant escape against overwhelmingly higher-level foes.
14. As a player or *Sủng vật* whose Flee attempt (`18001` or `14002`) fails, I want the server to broadcast skill ID `18002` (*Chạy trốn thất bại*) and never emit `14003` (*Thuật mua bán*), so that the client displays the authentic escape-failure animation and Vietnamese notice.
15. As a party leader—or a player whose active *Sủng vật* belongs to a party leader—in either PvE or PvP, I want a successful Flee to end the battle immediately and return all battle participants cleanly to the map without rewards, so that the leader's escape extracts the whole side decisively.
16. As a non-leader party member—or a player whose active *Sủng vật* belongs to a non-leader party member—I want a successful Flee to extract only me and my *Sủng vật* from the grid and participant state while my teammates keep fighting, so that my departure neither cancels my leader's battle, leaves zombie cells on the grid, prematurely respawns the enemy NPC, nor lets end-of-battle cleanup overwrite my post-flee state.

---

## Implementation Decisions

- **Kiến trúc Phân loại Trận đấu (`BattleKind`)**:
  - Bổ sung kiểu liệt kê hai trạng thái `BattleKind { Normal, Pk }` ngay trên thực thể cấu trúc trận chiến, tách biệt hoàn toàn với `fight_type` của dữ liệu Eve.
  - Hình thái quyết định:
    - `Normal`: cho phép ghi nhận NPC bị hạ để roll Drop/Gold/EXP, cấp EXP Nhân vật/*Sủng vật*, và trừ lượt *Phúc Thần* khi thắng.
    - `Pk`: chặn toàn bộ đường phát thưởng (Drop, Gold, Player EXP, Pet EXP) và bỏ qua bước trừ lượt *Phúc Thần*; giữ nguyên toàn bộ công thức chiến đấu, *Thu phục Võ tướng*, và hồi sinh NPC bản đồ sau trận kết thúc.
- **Định tuyến Khởi tạo & Guard Hợp lệ (`0x0B Sub 02`)**:
  - `0x0B 02 kind=3` (PK Võ tướng/NPC): giải mã 2 byte cuối thành chỉ số NPC trên bản đồ, đặt *DiaHinh* mặc định `112`, và luôn khởi tạo với `BattleKind::Pk` bất kể công tắc Option PK của người chơi đang bật hay tắt.
  - `0x0B 02 kind=2` (PK PvP): giữ nguyên kiểm tra công tắc Option PK của cả hai bên (phát cảnh báo `0x21 01 01` nếu đối thủ chưa bật PK), và bổ sung chốt chặn hợp lệ phía server trên người chơi: từ chối tự thách đấu chính mình, khác bản đồ, cùng tổ đội, mục tiêu thuộc nhóm GM/Thiên Thần, đang là *ListQS*, hoặc đang mở *Sạp hàng người chơi*. Không triển khai kiểm tra danh sách bản đồ Safe Zone phía server trong đợt này (dựa vào kiểm tra bản đồ phía client).
- **Đối xứng PvP Party & Vòng đời Kết thúc Hai phe**:
  - Hợp nhất việc khởi tạo PvP để nạp đầy đủ Đội trưởng, Thành viên tổ đội đứng cùng bản đồ và *Sủng vật* của cả Phe 1 (hàng `3` người, hàng `2` sủng vật) lẫn Phe 2 (hàng `0` người, hàng `1` sủng vật), sử dụng cùng quy tắc xếp cột `[2, 1, 3, 0, 4]`.
  - Đồng bộ gán mã trận cho mọi phiên kết nối trực tuyến của cả hai phe, nạp danh mục kỹ năng chiến đấu của cả hai phe vào handle trận, và phát khung khởi tạo thực thể người chơi (`0B 05 05`) thay vì khung NPC cho tất cả ô người chơi và *Sủng vật* ở cả hai phe.
  - Tổng quát hóa bước dọn dẹp kết thúc trận để duyệt qua toàn bộ người chơi còn lại ở **cả hai phe**, bảo đảm mọi người chơi cả Phe 1 lẫn Phe 2 đều được hồi tối thiểu 1 HP nếu tử trận và nhận đủ chuỗi gói rời trận (`0B 00`, `0B 01`, `05 04`, `14 08`).
- **Phân biệt Dấu Vai trò `0B FA` & Cổng Chặn `0x0B 01 03`**:
  - Thay đổi dấu bản thân trong khung `0B FA` sang chuẩn giao thức: `0x03` cho Đội trưởng, `0x05` cho Thành viên tổ đội, và `0x04` dành riêng cho *ListQS / Người quan sát*.
  - Cổng tiếp nhận `0x0B 01 03` chỉ chấp nhận thoát trực tiếp khi người gửi là *ListQS* (không nằm trong danh sách chiến đấu viên đang tham chiến); chiến đấu viên bắt buộc đi qua *Kênh Lệnh Chiến Đấu* `0x32 Sub 01`.
- **Công thức Flee, Luồng Thất bại & Dọn dẹp Thành viên Thoát trận**:
  - Giữ công thức xác suất theo cấp độ (`60 + (lv1 - lv2) + (lvtb1 - lvtb2)` với cấp độ mục tiêu đọc tại ô đích), cộng thêm `+20%` nếu sử dụng `14002` và kẹp kết quả trong `[0..=100]`, bỏ nhánh auto-thắng cho `14002`.
  - Cấm mọi thực thể không phải Nhân vật hoặc *Sủng vật* người chơi đi vào nhánh Flee của người chơi.
  - Khi roll thất bại (cho cả `18001` và `14002`), luôn nạp thông số kỹ năng từ mã `18002` (`skill_type = 12`) để dựng gói `32 01`.
  - Phân định hai phạm vi Flee thành công theo chủ sở hữu thực thể: nếu là Đội trưởng hoặc *Sủng vật* của Đội trưởng (bất kể Phe 1 hay Phe 2 trong PvP), trả kết quả kết thúc toàn trận ngay (`Outcome::PlayerFled`) và thoát sạch cả hai phe không kèm phần thưởng; nếu là Thành viên tổ đội hoặc *Sủng vật* của Thành viên, chỉ rút toàn bộ ô thuộc chủ sở hữu đó, bỏ gọi hồi sinh NPC giữa trận, bỏ phát trùng khung xóa ô, ngăn vòng thực thi ghi đè lại bản sao *Ô chiến trận* cũ lên lưới, và xóa chủ sở hữu khỏi cả danh sách thành viên lẫn tập người tham gia kết thúc trận.

---

## Testing Decisions

- **Chiến lược Kiểm thử theo Hành vi Ngoài (External Behavior Testing)**:
  - Chỉ kiểm chứng hành vi quan sát được qua gói tin nhị phân trả về (`Out`), thay đổi trạng thái phiên người chơi/bản đồ và kết quả lượt đấu từ public API; không kiểm tra các helper nội bộ riêng lẻ hay chi tiết cài đặt bên trong.
- **Đề xuất Test Seams (Cao nhất & Tối giản nhất)**:
  1. **Seam 1 — Tầng Dispatch / Handler (`handle_battle` / `BattleService`)**: Kiểm thử toàn luồng từ gói `C->S` nhị phân `0x0B 02 02`, `0x0B 02 03`, `0x0B 01 03`, `0x32 01` qua `BattleService` (nạp đủ 2 phe, `battle_id`, guard thách đấu PK, dấu `0B FA`, giải mã `npc_on_map_id`, và kết thúc không ghi đè thành viên đã chạy trốn).
  2. **Seam 2 — Tầng Engine Chiến trận (`Battle` / `run_turn` / `finish` với seeded RNG)**: Kiểm thử xác định các kịch bản phân loại `BattleKind::Pk` vs `Normal` (Drop/Gold/EXP/`Stat::God`/`apply_catch`/`npc_respawn` và cleanup Phe 2 PvP), công thức xác suất Flee (`18001` vs `14002`), phát đúng `18002` khi thất bại, ma trận Flee 4 vai trò tổ đội (kể cả PvP 2 phe) và hồi quy Zombie Cell xuyên nhiều hiệp liên tiếp.
- **Các module & test suite cần chạm/bổ sung**:
  - Toàn bộ kiểm thử mới đặt vào các submodule thích hợp dưới `tests/battle/` (trong suite `battle_test`), kế thừa mô hình test sẵn có tại `tests/battle/lifecycle.rs`, `tests/battle/command_channel.rs`, `tests/battle/debuff_turn_skip.rs`, `tests/battle/finish_generalization.rs` và `tests/eve_quest/random_encounter.rs`.
  - Cập nhật có chủ đích các test hợp đồng hiện hành bị thay đổi trực tiếp bởi quyết định kiến trúc mới: `open_leader.rs`, `open_self_kind.rs`, `command_channel.rs` (`0B 01 03`), `lifecycle.rs`, và `random_encounter.rs` (loại bỏ mock `14003` và kỳ vọng `14002` thắng 100%).

---

## Out of Scope

- Kiểm tra danh sách bản đồ cấm PK (Safe Zones) phía server theo bảng map tĩnh của client (tạm thời dựa vào chặn cục bộ phía `aLogin.exe`, bổ sung ở ticket sau nếu cần).
- Hỗ trợ cơ chế *Chen ngang / Tham chiến* (`0x0B 02 kind=5`) trong khi trận đang diễn ra.
- Đổi công thức Flee từ chênh lệch cấp độ (LV) sang chênh lệch Nhanh nhẹn (AGI).
- Thêm mới trường/bảng database ngoài cơ chế persistence sẵn có của `BattleService`.

---

## Further Notes

- Trong quá trình triển khai thực tế với `aLogin.exe`, cần chạy kiểm thử khói (manual smoke check) khi chuyển `0B FA` từ `0x04` về `0x03`/`0x05` để bảo đảm chuỗi gói thoát trận (`0B 00`, `05 04`, `14 08`, `0B 04` xóa trạng thái chiến đấu) đóng dứt điểm giao diện chiến đấu của client mà không kẹt cờ `player + 0x376`.
- Đã cập nhật đồng bộ thuật ngữ nghiệp vụ trong `GLOSSARY.md` và ghi nhận quyết định kiến trúc (mục `2.5` & `2.6`) vào `ADR 0006`.
