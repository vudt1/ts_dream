# 07 — Phase2: Battle channel 0x34 Reset + 0x35 EventEx (+ comment 0x32/0x33)

Status: ready-for-agent
Blocked by: 06

## Goal
Rename 2 opcode battle SAI rõ; 0x32/0x33 KHẢ_NGHI chỉ mở rộng comment.

## Evidence
- 0x34: handler S→C 1 nhánh SubOp 1 + guard `gvar_007DA51C`, callee `FUN_00650a24` = reset 21 slot + snapshot Now + chạy turn engine. Không byte ném/núi.
- 0x35: kênh S→C biến cố trận đánh 14 SubOp trên battle-record (`DAT_0098C63C`, lưới 4×5: move/commit/chat/item/FX). Không ship/skill thuyền. C→S không tồn tại → không trùng 0x45 (0x45 = child).
- 0x32: đúng kênh battle (`TFightManage`, C# `case 50 BattleCommandHandler`) nhưng body là battle-event record, không phải "command" client gửi → giữ tên, bổ sung comment.
- 0x33: S→C một chiều SubOp 1 → `FUN_0064cae0` áp flag/value lên unit; không bằng chứng spectate → giữ tên, bổ sung comment.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x34 | OP_MOUNTAIN_THROW | OP_BATTLE_RESET |
| 0x35 | OP_SHIP_SKILL | OP_BATTLE_EVENT_EX |
| 0x32 | OP_BATTLE_COMMAND | GIỮ + comment "event record, không phải client command" |
| 0x33 | OP_BATTLE_VIEW | GIỮ + comment "flag/value lên unit, chưa rõ spectate" |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`.

## Acceptance
- 2 tên mới đúng giá trị; test xanh.

## Comments
