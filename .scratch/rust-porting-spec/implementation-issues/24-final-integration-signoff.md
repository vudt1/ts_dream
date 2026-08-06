# 24 — Final integration + parity sign-off (Ch9 close)

**What to build:** Đưa toàn bộ hệ thống về một trạng thái đồng bộ: chạy end-to-end từ boot đến gameplay, chạy toàn bộ acceptance (golden byte-parity), gỡ các gap còn sót giữa các slice, và xác nhận dự án đạt parity so với server C#.

**Blocked by:** 23 — Golden scenario suite; và mọi feature ticket có golden (06–22) — đây là gate tổng cuối.

**Status:** ready-for-agent

- [ ] Toàn bộ `cargo test` (unit + golden integration) xanh.
- [ ] Start-up: MySQL up, migration, data load, TCP + web listener lên theo đúng Chuỗi.
- [ ] Chạy một phiên chơi thực (tạo account → login → spawn → move → talk → shop → battle) qua client thật, đúng byte (verify ngoài golden).
- [ ] Rà parity với các exclusion/wasgarble (Ch4), scoping `player_id` (Ch5 §5.4), banker's rounding (Ch6 §6.12).
- [ ] Bất kỳ opcode chưa xử lý trong danh sách 29 handler đều phải thỏa mãn: hoặc có golden, hoặc với ngã. Xác nhận "không phát minh packet" (Ch2 §2.4/§2.5).
- [ ] ADR/ghi chú: cập nhật `docs/adr/` + domain glossary nếu chưa có (môi trường single-context).