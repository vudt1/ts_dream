# 05: Phần thưởng Battle Drop Theo mùa Chống Farm & Hệ số EXP Lễ hội

**What to build:** Từ góc nhìn người chơi chiến đấu ngoài dã ngoại trong mùa lễ hội, khi chiến thắng trận PvE `Normal` trên bản đồ hợp lệ, người chơi có cơ hội nhận vật phẩm mùa theo cấu hình chống farm (`drop_rate_pct`, `min_enemy_level`, `daily_drop_cap` theo ngày `UTC+7`) và hưởng hệ số kinh nghiệm lễ hội (`exp_multiplier_pct`) phối hợp chính xác với `perexp`, *Thần Tài (`FGod`)* và *Con Ma (`FGhost`)*.

**Blocked by:** 03: Đồng bộ Đăng nhập (`17 39` + Tối đa 1 Popup `24 19` theo `season_seq`) & Bộ `FestivalDatEncoder`

**Status:** ready-for-agent

- [ ] Mở rộng cấu hình `FestivalProfile` hỗ trợ danh sách `bonus_drops` (`item_id`, `drop_rate_pct`, `min_enemy_level`, `daily_drop_cap`) cho các vật phẩm mùa (như *Bao lì xì năm mới* `46335`/`46338`, *Túi Đoan Ngọ* `46220`, *Hộp kẹo phá đám* `46073`) và `exp_multiplier_pct: Option<u32>`.
- [ ] Trong luồng kết thúc thắng trận PvE `Normal`, kiểm tra các lễ hội đang `Active`: tính hệ số `exp_multiplier_pct` nhất quán cùng `perexp`, *Thần Tài (`FGod`)* và *Con Ma (`FGhost`)*; kiểm tra ngưỡng cấp quái `min_enemy_level`, roll tỷ lệ bằng `DotNetRandom`, đối chiếu và tăng bộ đếm `daily_drop_cap` theo ngày `UTC+7` (`YYYY-MM-DD`) trong `character_festival_claims`.
- [ ] Khi trúng vật phẩm mùa và túi đồ còn chỗ, cộng vật phẩm qua quy trình túi đồ chuẩn (All-or-Nothing + khung `17 06` + `registry_update_if_online` + ghi SQLite nguyên tử); nếu túi đầy thì bỏ qua an toàn không làm hỏng giao dịch trận chiến.
- [ ] Áp dụng đầy đủ Negative Tests theo đúng bất biến của hệ thống: trận `PKBattle` (PvP và PK NPC) tuyệt đối không rơi vật phẩm mùa và không áp dụng thưởng EXP lễ hội; trận đánh quái dưới ngưỡng `min_enemy_level` không rơi đồ; nhân vật đã đạt `daily_drop_cap` ngừng rơi trong ngày `UTC+7` hiện tại và được mở lại khi sang ngày `UTC+7` kế tiếp.
- [ ] Chạy kiểm tra chốt chặn toàn suite (`cargo test --test events_arena_test` và `cargo test --all-targets --no-fail-fast`) cùng `tests/architecture_guardrails_test.rs` xác nhận không vi phạm chuẩn kiến trúc.
