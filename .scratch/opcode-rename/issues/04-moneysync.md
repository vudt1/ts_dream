# 04 — Phase1: 0x1A MoneySync (+ ghi chú 0x18 để Phase3)

Status: ready-for-agent
Blocked by: 03

## Goal
Rename 0x1A sai chắc; KHÔNG rename 0x18 (KHẢ_NGHI, để Phase 3).

## Evidence
- 0x1A: pseudo bác Talk hoàn toàn — kênh S→C "tiền + bộ đếm + banner" (sub 4 set `+0x12F8/+0x1300` + refresh `Form_Bank_Money`, không gọi `FUN_007AB870`, không form chat). C# S→C `(26,1)/(26,2)` = money-sync từ `GoldBankHandler`, khớp pseudo. Rust `npc_event::handle_pc_talk` chỉ đúng theo PC-table dialect.
- 0x18: pseudo "kho/vật phẩm + cờ trạng thái" (cộng/trừ/xóa/sync tồn kho, toast "Dung lượng nhiệm vụ đã đầy"), C# không có `case 24`. Có họ item nhưng chưa có bằng chứng "ItemInfo chi tiết" → giữ nguyên tên, chỉ mở rộng comment.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x1A | OP_TALK | OP_MONEY_SYNC |
| 0x18 | OP_ITEM_INFO | GIỮ NGUYÊN + bổ sung comment "stock/sync tồn kho, chưa rõ struct chi tiết — Phase3" |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`, `src/server/handlers/talk.rs` / `npc_event.rs` comment (ghi rõ 0x1A theo mobile-table = money, PC-table = talk).

## Steps
1. Rename 0x1A + alias deprecated. 0x18 chỉ sửa doc-comment.
2. Spec: `$1A MONEY_SYNC — đồng bộ tiền/bộ đếm/banner (S->C)`; `$18 ITEM_INFO — (tạm giữ) sync tồn kho, cần traffic Phase3`.

## Acceptance
- `OP_MONEY_SYNC == 0x1A`; `OP_ITEM_INFO` còn nguyên giá trị 0x18. Test xanh.

## Comments
