# 10 — Phase2: Economy/Arena 0x3D Lotto + 0x3E Life + 0x3F Arena + 0x41 Apparatus + 0x42 Mall

Status: ready-for-agent
Blocked by: 09

## Goal
Rename cụm kinh tế/arena dải 0x3D–0x42 (0x40 NavalCombat đã đúng, không đụng).

## Evidence
- 0x3D: máy Xổ số Lotto (`TLottoManager` + `TMachineManager`: 5 số 1..42, 6 ball, record 5B, 9 banner VISCII) + kênh message. C→S `[3D][01][5 số]`. Khác 0x26 GuildBattle thật.
- 0x3E: `TLifeManage` 2 SubOp (SubOp 1: 6 notify cố định theo `RP[1]`; SubOp 2 ghi Word `self+4`). C→S không tồn tại. `BlissBag.dat` chỉ là loader.
- 0x3F: Lôi đài + Thủy chiến (`TMRBatterManage`: ngũ hành/cược 5k–10k; `TMR_WaterBattleManage/Org/Help`; ghi `LocalPlayer+0x145A…`). `showOutfit()` trong C# nằm ở Action/Chat handler.
- 0x41: Khí quan/Apparatus (`TLH_ApparatusMenu` on/off/menu, 3 slot `[Slot][ItemID][Dura]`; `TPetNpc` spawn/despawn). Không rank.
- 0x42: pseudo multi-module UI (bảng skill 18×10, log quest, toast "Mua hư bảo…"); C# `case 66 ItemMallHandler` + dispatcher dùng 0x42 làm GM/Mall shop → tên GM tool chỉ đúng một nửa mall. 0x40 giữ nguyên (TWaterManage, khác 0x2E).

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x3D | OP_GUILD_WAR | OP_LOTTO |
| 0x3E | OP_BLISS_BAG | OP_LIFE_NOTIFY |
| 0x3F | OP_OUTFIT | OP_ARENA_WATERWAR |
| 0x41 | OP_RANK | OP_APPARATUS |
| 0x42 | OP_GM_TOOL | OP_ITEM_MALL (giữ gate GM ở handler) |
| 0x40 | OP_NAVAL_COMBAT | GIỮ NGUYÊN |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`.

## Acceptance
- 5 tên mới đúng giá trị; test xanh.

## Comments
