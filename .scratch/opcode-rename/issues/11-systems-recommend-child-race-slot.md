# 11 — Phase2: Systems 0x43 Recommend + 0x45 Child + 0x46 BoatRace + 0x48 Slot + 0xC7 GmAnnounce

Status: ready-for-agent
Blocked by: 10

## Goal
Rename nhóm system cuối + chốt 0xC7 không phải reconnect.

## Evidence
- 0x43: form Giới thiệu (`TJC_Referee`, SubOp 1 duy nhất: Kind 0 đổi tên + chat + đóng form, Kind 1..4 toast lỗi, Kind 5 close socket + form chọn server). C→S rỗng (luồng gửi dùng 0x23). Không đào lỗ.
- 0x45: Hệ thống Con cái (block 93B `+0x151D`, 6 slot trang bị, điểm trưởng thành `+0x1565` ngưỡng 400→Adult, 9 skill). Không thuyền, không trùng 0x35.
- 0x46: Đua thuyền rồng (controller `gvar_007DA714`, sync `[CharID][X][Y][state]`, cutscene `.sty`, BXH 10 mục). `Mark.Dat` chỉ là loader.
- 0x48: Máy Slot (`TMR_Slotform`: SubOp 2 reel 1..16, SubOp 3 cờ/toast/chat, SubOp 5 trao thưởng `[UID][ItemID][Count]` + broadcast, SubOp 6 ghi Word). Không thành trì.
- 0xC7: GM chat announce (`TGmManage`, SubOp 4 tag 3 GM/thì thầm, SubOp 6 tag 1 Thiên thần + `WA0033.wav`, wire `[C7][04|06][UID][Msg]`). C→S không tồn tại.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x43 | OP_HOLE_GAME | OP_RECOMMEND |
| 0x45 | OP_BOAT_SKILL | OP_CHILD |
| 0x46 | OP_MARK | OP_BOAT_RACE |
| 0x48 | OP_CITY_EX | OP_SLOT_MACHINE |
| 0xC7 | OP_RECONNECT | OP_GM_ANNOUNCE |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`.

## Acceptance
- 5 tên mới đúng giá trị; test xanh. Kết thúc Phase 2.

## Comments
