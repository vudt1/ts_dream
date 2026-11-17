# 05: Sửa luồng dùng thẻ triệu hồi: Đồng bộ Registry và Chặn khi Roster đầy

**What to build:**  
1. Trong `src/server/handlers/use_item/mod.rs`, khi người chơi dùng vật phẩm thêm pet (`add_pet > 10000`), kiểm tra `crate::server::pet_box::next_active_slot(&ctx.conn.session.pets)`. Nếu trả về `None` (đã đủ 4 sủng vật), lập tức thông báo lỗi chữ đỏ (e.g. `ctx.red("Số lượng sủng vật mang theo đã đạt tối đa (4/4).")`) và return `Ok(())` mà KHÔNG trừ item hay thêm pet vào DB/bộ nhớ.
2. Khi thêm pet thành công, sau khi `upsert_pet` và cập nhật `ctx.conn.session`, gọi `crate::server::session::registry_update_if_online(&ctx.conn.session)` để đảm bảo snapshot session trong registry dùng chung được cập nhật ngay lập tức theo chuẩn Two-Phase State Lock.

**Blocked by:** None

**Status:** completed

- [x] Thay logic fallback `.unwrap_or(1)` trong `use_item/mod.rs` bằng `match pet_box::next_active_slot(&ctx.conn.session.pets)`; trả về lỗi khi `None` mà không tiêu hao vật phẩm.
- [x] Gọi `registry_update_if_online(&ctx.conn.session)` sau khi ghi nhận pet mới thành công.
