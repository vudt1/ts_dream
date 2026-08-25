# 07 — Two-Tier Dispatcher and Modular Handlers

**What to build:** Tái cấu trúc bộ định tuyến 2 cấp (Two-tier Dispatcher), bóc tách tầng `ResponseSender`, xây dựng 36 handler modules chuyên trách theo Opcode chính/Subcode, tích hợp `PlayerStateManager`, `TradeSystem`, và `AutoSaveService` (Tokio interval 3 phút).

**Blocked by:** 01 — Binary Packet Reader and Writer, 02 — Standardized Domain Codecs (ThingData, Player, Battle), 05 — 4-Tier Eve Script Engine and Auto-Chain Resolver, 06 — MySQL Schema Migration 3NF and Repository Layer

**Status:** completed

- [x] Level 1 Dispatcher phân phối gói tin theo Main Opcode sạch sẽ và hiệu năng cao.
- [x] Xây dựng các Level 2 Handlers chuyên biệt: `login`, `chat`, `movement`, `character`, `stat`, `battle_manage`, `party`, `pet`, `npc_event` (tích hợp Eve Engine), `inventory`, `trade`, `shop`, `skill`, `bank`, `inn`, `battle_command`.
- [x] `ResponseSender` struct cung cấp các phương thức gửi gói tin chuẩn hóa (`send_player_appear`, `send_bag_items`, `send_dialog_talk`...) tách biệt hoàn toàn khỏi handler logic.
- [x] `PlayerStateManager` quản lý thuộc tính động, HP/SP max và equipment bonus in-memory an toàn tương tranh.
- [x] `TradeSystem` quản lý phiên giao dịch 2 người chơi nguyên tử chống duplicate vật phẩm.
- [x] `AutoSaveService` background task chạy mỗi 3 phút quét dirty session và lưu xuống DB an toàn.
