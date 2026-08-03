## Server (S) → Client (C) 完整 Opcode 表（72 handler）

### 分發器結構
- 入口：`0x006FD63C`
- Lookup table：`0x006FD68C`（200 bytes，opcode → index）
- Jump table：`0x006FD754`（73 × 4 bytes，index → handler VA）
- Default：`0x007079C8`
- 呼叫鏈：網路 → XOR 解密 → `0x0050BEF8` → `0x006FD63C` → handler

| Opcode | Hex | Handler | 名稱 | 狀態 |
|--------|-----|---------|------|------|
| 0 | 0x00 | 0x006FD878 | System/Login | 共用 |
| 1 | 0x01 | 0x006FDF3E | Auth | 共用 |
| 2 | 0x02 | 0x006FE49D | Chat | 共用 |
| 3 | 0x03 | 0x006FE559 | Look | 共用 |
| 4 | 0x04 | 0x006FEE09 | PlayerAppear | 共用 |
| 5 | 0x05 | 0x006FEFD0 | PlayerUpdate | 共用 |
| 6 | 0x06 | 0x006FF0FA | Move | 共用 |
| 7 | 0x07 | 0x006FF2D4 | PlayerDetail | 共用 |
| 8 | 0x08 | 0x006FF4E3 | StatUpdate | 共用 |
| 9 | 0x09 | 0x006FF8CC | CreateCharResult | 共用 |
| 11 | 0x0B | 0x006FF9BE | Battle | 共用 |
| 12 | 0x0C | 0x006FFBDA | Relocate | 共用 |
| 13 | 0x0D | 0x006FFE52 | Group | 共用 |
| 14 | 0x0E | 0x0070024B | Mail | 共用 |
| 15 | 0x0F | 0x00700553 | Pet | 共用 |
| 16 | 0x10 | 0x0070076C | NpcManage | 共用 |
| 19 | 0x13 | 0x00700C1C | BattlePet | 共用 |
| 20 | 0x14 | 0x00700D7B | Action | 共用 |
| 22 | 0x16 | 0x00701E36 | Skill | 共用 |
| 23 | 0x17 | 0x00702175 | Item | 共用（112 sub） |
| 24 | 0x18 | 0x00702EBF | ItemInfo | 共用 |
| 25 | 0x19 | 0x00703409 | SceneManage | 共用 |
| 26 | 0x1A | 0x007035ED | Talk | 共用 |
| 27 | 0x1B | 0x00703B99 | Trade | 共用 |
| 29 | 0x1D | 0x00703D38 | Bank | 共用 |
| 30 | 0x1E | 0x00703F45 | Storage | 共用 |
| 31 | 0x1F | 0x0070406C | NpcShop | 共用 |
| 32 | 0x20 | 0x007044F9 | Express | 共用 |
| 33 | 0x21 | 0x0070456A | Welcome | 共用 |
| 34 | 0x22 | 0x00704729 | GamePoints | 共用 |
| 35 | 0x23 | 0x007047AD | Guild | 共用 |
| 36 | 0x24 | 0x00704EE2 | GuildInfo | 共用 |
| 37 | 0x25 | 0x007051C0 | GuildAction | 共用 |
| 38 | 0x26 | 0x0070526A | GuildBattle | 共用 |
| 39 | 0x27 | 0x0070528B | SystemMaster | 共用 |
| 40 | 0x28 | 0x00705D45 | Hotkey | 共用 |
| 41 | 0x29 | 0x00705D66 | Quest | 共用 |
| 42 | 0x2A | 0x00706063 | Friend | 共用 |
| 43 | 0x2B | 0x007060CA | Compound | 共用 |
| 44 | 0x2C | 0x00706216 | RebornPet | 共用 |
| 45 | 0x2D | 0x00706264 | Reborn | 共用 |
| 46 | 0x2E | 0x00706966 | WaterWar | 共用 |
| 50 | 0x32 | 0x007069B6 | BattleCommand | 共用 |
| 51 | 0x33 | 0x00706A0A | BattleView | 共用 |
| 52 | 0x34 | 0x00706A2B | MountainThrow | 共用 |
| 53 | 0x35 | 0x00706A57 | ShipSkill | 共用 |
| 54 | 0x36 | 0x00706C1A | Keepalive | 共用 |
| 55 | 0x37 | 0x00706C2B | Stall | 共用 |
| 57 | 0x39 | 0x00706CBB | Gacha | 共用 |
| 58 | 0x3A | 0x00706D92 | Wheel | 共用 |
| 59 | 0x3B | 0x00706DB3 | Festival | 共用 |
| 60 | 0x3C | 0x00706E03 | Mount | 共用 |
| 61 | 0x3D | 0x00706E6A | GuildWar | 共用 |
| 62 | 0x3E | 0x007070D5 | BlissBag | 共用 |
| 63 | 0x3F | 0x0070710E | Outfit | 共用 |
| 64 | 0x40 | 0x007072AC | NavalCombat | 共用 |
| 65 | 0x41 | 0x007072FC | Rank | 共用 |
| 66 | 0x42 | 0x007073E2 | GmTool | 共用 |
| 67 | 0x43 | 0x00707575 | HoleGame | 共用 |
| 68 | 0x44 | 0x00707596 | Connect | 共用 |
| 69 | 0x45 | 0x007075D5 | BoatSkill | 共用 |
| 70 | 0x46 | 0x007076F7 | Mark | 共用 |
| 71 | 0x47 | 0x007077D0 | AntiAddiction | 共用 |
| 72 | 0x48 | 0x0070783E | CityEx | 共用 |
| 199 | 0xC7 | 0x00707996 | Reconnect | 共用 |

---

## Client (C) → Server (S) 完整 Opcode 表（69 handler）

### 分發器結構
- 入口：`0x006EE9D0`
- Lookup table：`0x006EEA27`（200 bytes，opcode → index）
- Jump table：`0x006EEAEF`（70 × 4 bytes，index → handler VA）
- Default：`0x006FD1D2`
- 呼叫模式：`mov dl, opcode; mov cl, sub; call 0x006EE9D0`（395 個呼叫點）

| Opcode | Hex | Idx | Handler | 名稱 | 狀態 |
|--------|-----|-----|---------|------|------|
| 0 | 0x00 | 1 | 0x006EEC07 | Login | 共用 |
| 1 | 0x01 | 2 | 0x006EEC1D | Auth | 共用 |
| 2 | 0x02 | 3 | 0x006EEE99 | Chat | 共用 |
| 5 | 0x05 | 5 | 0x006EF298 | MoveConfirm | 共用 |
| 6 | 0x06 | 6 | 0x006EF3D1 | Move | 共用 |
| 8 | 0x08 | 8 | 0x006EF672 | StatPoint | 共用 |
| 9 | 0x09 | 9 | 0x006EF872 | CreateChar | 共用 |
| 10 | 0x0A | 10 | 0x006EFCA9 | （未命名） | 共用¹ |
| 11 | 0x0B | 11 | 0x006EFCA9 | Battle | 共用 |
| 12 | 0x0C | 12 | 0x006F04D5 | Relocate | 共用 |
| 13 | 0x0D | 13 | 0x006F0548 | Group | 共用 |
| 14 | 0x0E | 14 | 0x006F0A24 | Mail | 共用 |
| 15 | 0x0F | 15 | 0x006F0D16 | Pet | 共用 |
| 16 | 0x10 | 16 | 0x006F16EC | NpcManage | 共用 |
| 18 | 0x12 | 17 | 0x006F2FFC | （未命名） | 共用¹ |
| 19 | 0x13 | 18 | 0x006F30B7 | BattlePet | 共用 |
| 20 | 0x14 | 19 | 0x006F31CA | Action | 共用 |
| 22 | 0x16 | 20 | 0x006F34E0 | （未命名） | 共用¹ |
| 23 | 0x17 | 21 | 0x006F35E1 | Item | 共用 |
| 24 | 0x18 | 22 | 0x006F62EC | ItemInfo | 共用 |
| 25 | 0x19 | 23 | 0x006F64B7 | SceneManage | 共用 |
| 26 | 0x1A | 24 | 0x006F6DCD | Talk | 共用 |
| 27 | 0x1B | 26 | 0x006F7330 | Trade | 共用 |
| 28 | 0x1C | 25 | 0x006F7076 | Skill | 共用 |
| 29 | 0x1D | 27 | 0x006F758C | Bank | 共用 |
| 30 | 0x1E | 28 | 0x006F7627 | Storage | 共用 |
| 31 | 0x1F | 29 | 0x006F7A3D | NpcShop | 共用 |
| 32 | 0x20 | 30 | 0x006F7F31 | Express | 共用 |
| 33 | 0x21 | 31 | 0x006F8103 | Welcome | 共用 |
| 34 | 0x22 | 32 | 0x006F8264 | GamePoints | 共用 |
| 35 | 0x23 | 33 | 0x006F8345 | Guild | 共用 |
| 36 | 0x24 | 34 | 0x006F888A | GuildInfo | 共用 |
| 37 | 0x25 | 35 | 0x006F89C8 | GuildAction | 共用 |
| 39 | 0x27 | 36 | 0x006F8A51 | SystemMaster | 共用 |
| 40 | 0x28 | 37 | 0x006F9B8D | Hotkey | 共用 |
| 41 | 0x29 | 38 | 0x006F9C19 | Quest | 共用 |
| 42 | 0x2A | 39 | 0x006F9E97 | Friend | 共用 |
| 43 | 0x2B | 40 | 0x006FA053 | Compound | 共用 |
| 44 | 0x2C | 41 | 0x006FA233 | RebornPet | 共用 |
| 45 | 0x2D | 42 | 0x006FA443 | Reborn | 共用 |
| 46 | 0x2E | 43 | 0x006FA692 | WaterWar | 共用 |
| 50 | 0x32 | 44 | 0x006FA705 | BattleCommand | 共用 |
| 54 | 0x36 | 45 | 0x006FAB81 | Keepalive | 共用 |
| 55 | 0x37 | 46 | 0x006FABA6 | Stall | 共用 |
| 57 | 0x39 | 47 | 0x006FAC2E | Gacha | 共用 |
| 58 | 0x3A | 48 | 0x006FACA1 | Wheel | 共用 |
| 59 | 0x3B | 49 | 0x006FAD3E | Festival | 共用 |
| 60 | 0x3C | 50 | 0x006FADD2 | Mount | 共用 |
| 61 | 0x3D | 51 | 0x006FAF3E | GuildWar | 共用 |
| 63 | 0x3F | 52 | 0x006FB0CE | Outfit | 共用 |
| 64 | 0x40 | 53 | 0x006FB2FA | NavalCombat | 共用 |
| 65 | 0x41 | 54 | 0x006FB476 | Rank | 共用 |
| 66 | 0x42 | 55 | 0x006FB857 | GmTool | 共用 |
| 67 | 0x43 | 56 | 0x006FBE6D | HoleGame | 共用 |
| 68 | 0x44 | 57 | 0x006FBF84 | Connect | 共用 |
| 69 | 0x45 | 58 | 0x006FC09B | BoatSkill | 共用 |
| 70 | 0x46 | 59 | 0x006FC276 | Mark | 共用 |
| 71 | 0x47 | 60 | 0x006FC2E9 | AntiAddiction | 共用 |
| 72 | 0x48 | 61 | 0x006FC466 | CityEx | 共用 |
| 199 | 0xC7 | 69 | 0x006FCFD1 | Reconnect | 共用 |

---