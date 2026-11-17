# 06: Tập trung hằng số giao thức Pet Recall Frame

**What to build:**  
Định nghĩa hằng số công khai `pub const PET_RECALL_FRAME: &str = "F44402001302";` tại `src/server/handlers/pet_actions.rs` (hoặc `src/protocol/mod.rs`). Thay thế tất cả chuỗi literal `"F44402001302"` trong `src/server/handlers/trade_storage.rs` bằng hằng số này.

**Blocked by:** None

**Status:** completed

- [x] Định nghĩa `pub const PET_RECALL_FRAME: &str = "F44402001302";` trong `src/server/handlers/pet_actions.rs`.
- [x] Import và sử dụng `PET_RECALL_FRAME` trong `src/server/handlers/trade_storage.rs`.
