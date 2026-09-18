# 09 — Phase2: Minigame 0x39 Sport + 0x3A Dice + 0x3B Domino + 0x3C Chess

Status: ready-for-agent
Blocked by: 08

## Goal
Rename cụm minigame bị gán nhãn Gacha/Wheel/Festival/Mount.

## Evidence
- 0x39: remote `TSportManage` (mở form id 1/2/3/4/6/FF, đóng form, banner 1200ms "cấm công cụ"). C→S rỗng. Không gacha.
- 0x3A: kết quả xúc xắc Tài/Xỉu (`TRE_BiDaXiao`, 3 byte c1/c2/c3 + flag + token DWORD, nhãn "Tiểu/Đại"). C→S `[3A][f4][bet DWORD]`. Không vòng quay.
- 0x3B: bàn Domino (`TSBDManager`, SubOp 1 mode chờ, SubOp 2 nạp 3 xúc xắc 1..6, SubOp 3 set state; toast "Domino…"). C→S `[3B][01][DWORD]`. Không lễ hội.
- 0x3C: Cờ Tướng (`TRE_ZMChessMain`, 4 SubOp đồng bộ bàn/nước đi/trạng thái/đồng hồ, 13 mã banner). C→S `[3C][01][text]` / `[3C][02]`. Không thú cưỡi.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x39 | OP_GACHA | OP_SPORT_FORM |
| 0x3A | OP_WHEEL | OP_DICE_BIDAXIAO |
| 0x3B | OP_FESTIVAL | OP_DOMINO |
| 0x3C | OP_MOUNT | OP_ZM_CHESS |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`.

## Acceptance
- 4 tên mới đúng giá trị; test xanh.

## Comments
