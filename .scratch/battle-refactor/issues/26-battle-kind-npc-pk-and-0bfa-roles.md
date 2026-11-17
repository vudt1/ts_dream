# 26: Phân loại `BattleKind` (`Normal` vs `PKBattle`), Sửa `0x0B 02 03` PK Võ tướng & Chuẩn hóa `0B FA` (`0x03`/`0x05`/`0x04`)

**What to build:** Thiết lập phân loại vòng đời trận đấu `Normal` và `PKBattle` (tách biệt khỏi `fight_type` của Eve), sửa tiếp nhận `0x0B 02 03` (PK Võ tướng trên bản đồ) để giải mã đúng chỉ số slot NPC bản đồ (`1..=100`) cùng *DiaHinh* mặc định `112` và luôn mở `PKBattle` bất kể công tắc Option PK; áp dụng hạn chế phần thưởng chuẩn cho `PKBattle` (cấm Drop, Gold, Player EXP, Pet EXP và không trừ *Phúc Thần*, nhưng vẫn giữ *Thu phục Võ tướng* `15001` và hồi sinh NPC bản đồ `npc_respawn`); đồng thời khôi phục dấu bản thân `0B FA` về `0x03` (Đội trưởng) / `0x05` (Thành viên), dành `0x04` riêng cho *ListQS / Người quan sát* và khóa `0x0B 01 03` chỉ cho *ListQS* thoát thẳng.

**Blocked by:** None (can start immediately)

**Spec:** [spec-pk-and-flee.md](../spec-pk-and-flee.md)

**Status:** completed

## Acceptance Criteria

### 1. Phân loại `BattleKind` (`Normal` vs `PKBattle`) & Sửa `0x0B 02 03`
- [x] Thực thể trận đấu mang phân loại `BattleKind` (`Normal` mặc định và `Pk`), độc lập hoàn toàn với `fight_type` của dữ liệu Eve.
- [x] Mọi gói `0x0B 02 03` đều khởi tạo `PKBattle` (kể cả khi công tắc Option PK của người chơi đang tắt), đọc 2 byte cuối gói làm chỉ số slot NPC trên bản đồ (`npc_on_map_id`) và sử dụng *DiaHinh* mặc định `112`.
- [x] Toàn bộ các trận PvE chuẩn (đụng độ ngẫu nhiên, kịch bản Eve, *Trận đấu NPC chủ động*, *TeamDef*, sự kiện 40 NPC, *Vấn Đáp Bắc Tinh Quân*) giữ nguyên phân loại `Normal` và phần thưởng như cũ.
- [x] Trong `PKBattle` đánh NPC, mọi cơ chế sát thương, trạng thái, hợp kích, *Thu phục Võ tướng* (`15001`) và hồi sinh NPC tuần tra sau trận (`npc_respawn` khi `talking_battle > 0`) hoạt động bình thường.
- [x] Trong `PKBattle`, không ghi nhận kill thưởng, không rơi vật phẩm (`35 04` / `Out::Drop`), không cộng Gold (`1A 01` / `Out::Gold`), không cộng EXP Nhân vật (`Stat::Texp`) hoặc kích hoạt thăng cấp, không cộng EXP *Sủng vật* (`0F 08` / `Out::PetExp`), và không trừ lượt *Phúc Thần* (`Stat::God`).

### 2. Chuẩn hóa dấu vai trò `0B FA` & Cổng `0x0B 01 03` cho ListQS
- [x] Khung `0B FA` phát cho Đội trưởng mang dấu bản thân `0x03`, khung phát cho Thành viên tổ đội mang dấu bản thân `0x05`, và chỉ khung phát cho *ListQS* (khi tham gia quan chiến) mang dấu `0x04`.
- [x] Handler `0x0B 01 03` chỉ cho phép *ListQS* rời trận trực tiếp kèm dọn trạng thái quan chiến, còn chiến đấu viên đang tham chiến gửi `0x0B 01 03` sẽ bị từ chối (giữ nguyên `battle_id`, không phát khung thoát trận).

### 3. Kiểm thử (`battle_test`)
- [x] Cập nhật có chủ đích các test hợp đồng `0B FA` (`open_leader`, `open_self_kind`) và `0x0B 01 03` (`command_channel`) khớp chuẩn `0x03`/`0x05`/`0x04` và quy tắc *ListQS*-only.
- [x] Bổ sung kiểm thử xác định ở tầng handler/service và engine chứng minh `0x0B 02 03` giải mã đúng `npc_on_map_id` với `DiaHinh = 112`, `PKBattle` trả `0` thưởng / giữ nguyên *Phúc Thần* / vẫn bắt được NPC, còn `Normal` giữ nguyên phần thưởng.
- [x] `cargo test --test battle_test` vượt qua 100%.
