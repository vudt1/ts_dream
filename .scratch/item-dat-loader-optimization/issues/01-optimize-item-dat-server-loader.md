# 01: Tối ưu nạp Item.dat cho server và hợp nhất bảng vật phẩm trên GameData

**What to build:** Khi khởi động từ dữ liệu nhị phân `Item.dat`, Game Server chỉ giải mã các cột phục vụ nghiệp vụ server và lưu trữ duy nhất một bảng tra cứu vật phẩm (`items`) trên `GameData`, thay vì giải mã cả các cột hiển thị phía client (đặc biệt là mô tả 254 bytes) và giữ song song hai bảng `item_defs` / `items` trong RAM. Toàn bộ luồng NPC Shop (mua/bán theo `price`/`sell_price`), Crafting (kiểm tra `kind`, `compound_material`, `compound_rank` và thông báo `sys_msg_frame` UTF-8), Inventory Hydration, Pet Reborn, Use Item, Quest, và Battle Session tiếp tục hoạt động chính xác đầu cuối; đồng thời `BattleService` chia sẻ bảng vật phẩm bất biến từ `GameData` qua tham chiếu đếm `Arc` thay vì nhân bản toàn bộ bảng khi mở trận đấu.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Bộ nạp `Item.dat` cho server bỏ qua hoàn toàn việc giải mã, chuyển đổi bảng mã và cấp phát bộ nhớ cho `description`, `description_viscii`, các mã icon/hình ảnh, bảng màu tint, `display_material`, `sort`, `btn_state` và các trường mobile chưa dùng; vẫn giữ nguyên quy tắc bỏ qua record `0`, bỏ qua `id == 0`, từ chối file sai bội số độ dài record 370 bytes, và cộng `+100` cho `attr*_value` khi `attr*_kind` nằm trong `65..=67`.
- [x] Bản ghi vật phẩm chuẩn của server (`Item`) có đầy đủ `price` và `sell_price` được nạp từ `Item.dat` (và mặc định `0` khi fallback từ `Items.txt` legacy); tên UTF-8 phục vụ thông báo hệ thống được giải mã theo nhu cầu từ byte VISCII 1.1 thay vì cấp phát sẵn `String` cho toàn bộ vật phẩm lúc khởi động.
- [x] `GameData` không còn lưu trường `item_defs` song song với `items`; các phân hệ Crafting và NPC Shop đọc trực tiếp từ `items`, không thêm alias tương thích ngược vi phạm architecture guardrails.
- [x] `BattleService` chia sẻ `Arc` trỏ tới `GameData` (hoặc bảng `items` dùng chung) thay vì clone toàn bộ `HashMap` vật phẩm mỗi lần khởi tạo trận đấu.
- [x] Có test tự động mới cho bộ nạp nhị phân `Item.dat` (cả dữ liệu tổng hợp 370-byte và dữ liệu thật) cùng các test Crafting, Economy/NPC Shop, Inventory Hydration, Battle và Architecture Guardrails đều vượt qua khi chạy toàn bộ test suite (`cargo test --all-targets --no-fail-fast`).

## Comments

- **2026-11-18**: Đã triển khai qua subagent và rà soát 2 trục (`/code-review`: Standards + Spec). Sau review đã chỉnh `Item::utf8_name()` sang `encoding::viscii_decode` (thay cho `cp1252_to_string` để đảm bảo round-trip VISCII 1.1 qua `sys_msg_frame`), gộp `match attr1_kind`/`attr2_kind` vào helper `apply_attr`, và thay literal `0x1B`/`1` trong test NPC Shop bằng `OP_NPC_SHOP`/`NPC_SHOP_SUB_BUY`. Toàn bộ 11 test suites (`cargo test --all-targets --no-fail-fast`) đều pass (0 failed, 0 ignored). Thay đổi đang giữ ở working tree để user tự commit theo `AGENTS.md`.
