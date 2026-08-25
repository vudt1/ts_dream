# 06 — MySQL Schema Migration 3NF and Repository Layer

**What to build:** Xây dựng migration SQLx mới (`0002_modern_schema.sql`) chuẩn hóa quan hệ 3NF (Accounts 1:1 Characters — phiên bản PC chỉ hỗ trợ duy nhất 1 nhân vật mỗi tài khoản, ràng buộc UNIQUE trên `account_id`; hợp nhất 5 bảng túi đồ thành `inventories` 35B, 4 kho `character_pets`, Missions, BitFlags, Mails, Friends) với collation `latin1_bin` bảo toàn chuỗi byte VISCII, cùng hệ thống Async Repository Traits.

**Blocked by:** 02 — Standardized Domain Codecs (ThingData, Player, Battle)

**Status:** completed

- [x] Tạo file migration `migrations/0002_modern_schema.sql` định nghĩa đầy đủ các bảng: `accounts`, `characters`, `character_money`, `inventories` (composite PK: `char_id, storage_type, slot`), `character_pets` (storage_type 1..4), `character_skills`, `character_hotkeys`, `character_missions`, `character_mission_flags`, `character_bit_flags`, `character_completed_events`, `friends`, `mails`.
- [x] Bảo toàn cấu trúc tên nhân vật và text dưới dạng `VARCHAR(...) CHARACTER SET latin1 COLLATE latin1_bin`.
- [x] Xây dựng các Rust Repository Traits: `AccountRepository`, `CharacterRepository`, `InventoryRepository`, `PetRepository`, `QuestRepository`.
- [x] Cung cấp các thao tác transaction nguyên tử cho P2P Trade, Shop Buy, Bank Transfer.
- [x] Unit/Integration tests kiểm tra CRUD và transaction trên môi trường test database.
