# 02 — Phase1: 0x10 GmManage + 0x14 NpcEvent + 0x16 WorldObject

Status: ready-for-agent
Blocked by: 01

## Goal
Rename 3 bus bị đặt tên sai họ chức năng trong dải 0x10–0x16.

## Evidence
- 0x10: pseudo `opcode_10.md` — bus S→C "GM/Quiz-Day55 + chat hệ thống" (25 subop, `TGmManage`, `TAC_Question`), C→S rỗng. C# không có `case 16`. Không spawn quái/đối thoại NPC.
- 0x14: pseudo `opcode_14.md` — "Map Teleport & Scene Script" 2 chiều (sub 1–6 block 15B chạy `FUN_005EB530` đổi map + handshake). C# `case 20 ActionHandler` = ClickNpc/TalkQuestNpc/ClickGate/selectMenu (NPC talk/cổng). Emotion Ngồi/Đứng thật do 0x20 đảm nhiệm.
- 0x16: pseudo `opcode_16.md` — đồng bộ World-Object & tác chiến S→C (slot 0..100, sub 9 ra action A→C), không có skill nào. Cặp hậu tố SC/CS với 0x1C gây hiểu lầm.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x10 | OP_NPC_MANAGE | OP_GM_MANAGE |
| 0x14 | OP_ACTION | OP_NPC_EVENT (ghi chú alias ý nghĩa scene-script) |
| 0x16 | OP_SKILL_SC | OP_WORLD_OBJECT (đồng thời bỏ quy ước SC/CS với 0x1C) |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`, comment `dispatcher.rs`/handlers nhắc 0x10/0x14/0x16.

## Steps
1. Rename const + alias deprecated + `opcode_name()` + `SERVER_MAIN_OPCODES` như ticket 01.
2. Spec: `$10 GM_MANAGE — quiz/GM + syschat (S->C, C->S rỗng)`; `$14 NPC_EVENT — NPC talk/cổng/scene-script (Shared)`; `$16 WORLD_OBJECT — world-object sync slot 0..100 (S->C)`.
3. Sửa comment `dispatcher.rs` nếu ghi "Action (Ngồi/Đứng…)" cho 0x14.

## Acceptance
- 3 tên mới resolve, tên cũ chỉ còn deprecated. `cargo test --all-targets --no-fail-fast` xanh.

## Comments
