# 03 — Phase1: Xoay vòng 0x19 Trade / 0x1B NpcShop / 0x1F PetHotel

Status: ready-for-agent
Blocked by: 02

## Goal
Sửa bộ ba bị xoay vòng tên giữa PC-table và mobile-table/C#.

## Evidence
- 0x19: pseudo bus pass-through 16 subop (toast/hoa/spawn `THuman`/hotbar), C# `case 25 TransferHandler` với `TRADE_CMD=25` = trade items/pets 2 chiều. Rust dispatcher đã tự ghi chú mâu thuẫn PC vs mobile table.
- 0x1B: pseudo toast văn bản tĩnh Outer/Inner + trigger form, không logic trade. C# `case 27 NpcShopsHandler` = mua/bán shop NPC theo `EveData.listNpcShopOnmap`. Trade P2P thật ở 0x19.
- 0x1F: pseudo push trạng thái (dialog 3 mode, lock-target, equip nhanh 7×57B, hotbar), không shop. C# `case 31 PetHotelHandler` = khách điếm pet. Rust dispatcher đã ghi "Pet stable in mobile-table; NPC shop in PC-table".

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x19 | OP_SCENE_MANAGE | OP_TRADE |
| 0x1B | OP_TRADE | OP_NPC_SHOP |
| 0x1F | OP_NPC_SHOP | OP_PET_HOTEL |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`, komentar `dispatcher.rs` (xóa ghi chú mâu thuẫn cũ, ghi rõ mapping mobile-table/C#).

## Steps
1. Rename cả 3 cùng lúc (tránh hoán đổi nhầm): const mới + alias deprecated từng tên cũ trỏ sang tên mới cùng giá trị.
2. Spec 3 dòng: `$19 TRADE — trade P2P items/pets (Shared)`; `$1B NPC_SHOP — mua/bán NPC (Shared)`; `$1F PET_HOTEL — khách điếm pet (Shared)`.
3. Grep `SCENE_MANAGE|OP_TRADE|OP_NPC_SHOP` — mọi reference nội bộ chuyển sang tên mới đúng giá trị (cẩn thận 0x1B cũ → OP_NPC_SHOP mới).

## Acceptance
- `OP_TRADE == 0x19`, `OP_NPC_SHOP == 0x1B`, `OP_PET_HOTEL == 0x1F`; không còn reference nội bộ dùng tên cũ. Test xanh.

## Comments
