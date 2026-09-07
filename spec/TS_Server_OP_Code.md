## Server (S) → Client (C) 完整 Opcode 表（72 handler）

| Opcode | Hex | 名稱 | 狀態 |
|--------|-----|------|------|
| 0 | 0x00 |  System Connect/Handshake | 共用 |
| 1 | 0x01 |  Auth | 共用 |
| 2 | 0x02 |   Chat | 共用 |
| 3 | 0x03 |   Look | 共用 |
| 4 | 0x04 |   PlayerAppear | 共用 |
| 5 | 0x05 |   PlayerUpdate | 共用 |
| 6 | 0x06 |   Move | 共用 |
| 7 | 0x07 | PlayerDetail | 共用 |
| 8 | 0x08 | StatUpdate | 共用 |
| 9 | 0x09 | CreateCharResult | 共用 |
| 11 | 0x0B | Battle | 共用 |
| 12 | 0x0C | Relocate | 共用 |
| 13 | 0x0D | Group | 共用 |
| 14 | 0x0E | Mail | 共用 |
| 15 | 0x0F | Pet | 共用 |
| 16 | 0x10 | NpcManage | 共用 |
| 19 | 0x13 | BattlePet | 共用 |
| 20 | 0x14 | Action | 共用 |
| 22 | 0x16 | Skill | 共用 |
| 23 | 0x17 | Item | 共用（112 sub） |
| 24 | 0x18 | ItemInfo | 共用 |
| 25 | 0x19 | SceneManage | 共用 |
| 26 | 0x1A | Talk | 共用 |
| 27 | 0x1B | Trade | 共用 |
| 29 | 0x1D | Bank | 共用 |
| 30 | 0x1E | Storage | 共用 |
| 31 | 0x1F | NpcShop | 共用 |
| 32 | 0x20 | Express | 共用 |
| 33 | 0x21 | Welcome | 共用 |
| 34 | 0x22 | GamePoints | 共用 |
| 35 | 0x23 | Guild | 共用 |
| 36 | 0x24 | GuildInfo | 共用 |
| 37 | 0x25 | GuildAction | 共用 |
| 38 | 0x26 | GuildBattle | 共用 |
| 39 | 0x27 | SystemMaster | 共用 |
| 40 | 0x28 | Hotkey | 共用 |
| 41 | 0x29 | Quest | 共用 |
| 42 | 0x2A | Friend | 共用 |
| 43 | 0x2B | Compound | 共用 |
| 44 | 0x2C | RebornPet | 共用 |
| 45 | 0x2D | Reborn | 共用 |
| 46 | 0x2E | WaterWar | 共用 |
| 50 | 0x32 | BattleCommand | 共用 |
| 51 | 0x33 | BattleView | 共用 |
| 52 | 0x34 | MountainThrow | 共用 |
| 53 | 0x35 | ShipSkill | 共用 |
| 54 | 0x36 | Keepalive | 共用 |
| 55 | 0x37 | Stall | 共用 |
| **56** | **0x38** | **Stub（空殼）** | **NTS 新增** |
| 57 | 0x39 | Gacha | 共用 |
| 58 | 0x3A | Wheel | 共用 |
| 59 | 0x3B | Festival | 共用 |
| 60 | 0x3C | Mount | 共用 |
| 61 | 0x3D | GuildWar | 共用 |
| 62 | 0x3E | BlissBag | 共用 |
| 63 | 0x3F | Outfit | 共用 |
| 64 | 0x40 | NavalCombat | 共用 |
| 65 | 0x41 | Rank | 共用 |
| 66 | 0x42 | GmTool | 共用 |
| 67 | 0x43 | HoleGame | 共用 |
| 68 | 0x44 | Connect | 共用 |
| 69 | 0x45 | BoatSkill | 共用 |
| 70 | 0x46 | Mark | 共用 |
| 71 | 0x47 | AntiAddiction | 共用 |
| 72 | 0x48 | CityEx | 共用 |
| **73** | **0x49** | **WorldBoss** | **NTS 新增** |
| **74** | **0x4A** | **NpcUpgrade** | **NTS 新增** |
| **75** | **0x4B** | **SaleRoom** | **NTS 新增** |
| **76** | **0x4C** | **ExpSlot** | **NTS 新增** |
| **77** | **0x4D** | **Activity** | **NTS 新增** |
| **78** | **0x4E** | **Astrolabe** | **NTS 新增** |
| 199 | 0xC7 | Reconnect | 共用 |

---

## Client (C) → Server (S) 完整 Opcode 表（69 handler）

| Opcode | Hex | Idx | 名稱 | 狀態 |
|--------|-----|-----|------|------|
| 0 | 0x00 | 1 | System Connect/Handshake | 共用 |
| 1 | 0x01 | 2 | Auth | 共用 |
| 2 | 0x02 | 3 | Chat | 共用 |
| **3** | **0x03** | **4** | **Look（請求）** | **NTS 新增** |
| 5 | 0x05 | 5 | MoveConfirm | 共用 |
| 6 | 0x06 | 6 | Move | 共用 |
| **7** | **0x07** | **7** | **PlayerDetail（請求）** | **NTS 新增** |
| 8 | 0x08 | 8 | StatPoint | 共用 |
| 9 | 0x09 | 9 | CreateChar | 共用 |
| 10 | 0x0A | 10 | （未命名） | 共用¹ |
| 11 | 0x0B | 11 | Battle | 共用 |
| 12 | 0x0C | 12 | Relocate | 共用 |
| 13 | 0x0D | 13 | Group | 共用 |
| 14 | 0x0E | 14 | Mail | 共用 |
| 15 | 0x0F | 15 | Pet | 共用 |
| 16 | 0x10 | 16 | NpcManage | 共用 |
| 18 | 0x12 | 17 | （未命名） | 共用¹ |
| 19 | 0x13 | 18 | BattlePet | 共用 |
| 20 | 0x14 | 19 | Action | 共用 |
| 22 | 0x16 | 20 | （未命名） | 共用¹ |
| 23 | 0x17 | 21 | Item | 共用 |
| 24 | 0x18 | 22 | ItemInfo | 共用 |
| 25 | 0x19 | 23 | SceneManage | 共用 |
| 26 | 0x1A | 24 | Talk | 共用 |
| 27 | 0x1B | 26 | Trade | 共用 |
| 28 | 0x1C | 25 | Skill | 共用 |
| 29 | 0x1D | 27 | Bank | 共用 |
| 30 | 0x1E | 28 | Storage | 共用 |
| 31 | 0x1F | 29 | NpcShop | 共用 |
| 32 | 0x20 | 30 | Express | 共用 |
| 33 | 0x21 | 31 | Welcome | 共用 |
| 34 | 0x22 | 32 | GamePoints | 共用 |
| 35 | 0x23 | 33 | Guild | 共用 |
| 36 | 0x24 | 34 | GuildInfo | 共用 |
| 37 | 0x25 | 35 | GuildAction | 共用 |
| 39 | 0x27 | 36 | SystemMaster | 共用 |
| 40 | 0x28 | 37 | Hotkey | 共用 |
| 41 | 0x29 | 38 | Quest | 共用 |
| 42 | 0x2A | 39 | Friend | 共用 |
| 43 | 0x2B | 40 | Compound | 共用 |
| 44 | 0x2C | 41 | RebornPet | 共用 |
| 45 | 0x2D | 42 | Reborn | 共用 |
| 46 | 0x2E | 43 | WaterWar | 共用 |
| 50 | 0x32 | 44 | BattleCommand | 共用 |
| 54 | 0x36 | 45 | Keepalive | 共用 |
| 55 | 0x37 | 46 | Stall | 共用 |
| 57 | 0x39 | 47 | Gacha | 共用 |
| 58 | 0x3A | 48 | Wheel | 共用 |
| 59 | 0x3B | 49 | Festival | 共用 |
| 60 | 0x3C | 50 | Mount | 共用 |
| 61 | 0x3D | 51 | GuildWar | 共用 |
| 63 | 0x3F | 52 | Outfit | 共用 |
| 64 | 0x40 | 53 | NavalCombat | 共用 |
| 65 | 0x41 | 54 | Rank | 共用 |
| 66 | 0x42 | 55 | GmTool | 共用 |
| 67 | 0x43 | 56 | HoleGame | 共用 |
| 68 | 0x44 | 57 | Connect | 共用 |
| 69 | 0x45 | 58 | BoatSkill | 共用 |
| 70 | 0x46 | 59 | Mark | 共用 |
| 71 | 0x47 | 60 | AntiAddiction | 共用 |
| 72 | 0x48 | 61 | CityEx | 共用 |
| **73** | **0x49** | **62** | **WorldBoss** | **NTS 新增** |
| **74** | **0x4A** | **63** | **NpcUpgrade** | **NTS 新增** |
| **75** | **0x4B** | **64** | **SaleRoom** | **NTS 新增** |
| **76** | **0x4C** | **65** | **ExpSlot** | **NTS 新增** |
| **77** | **0x4D** | **66** | **Activity** | **NTS 新增** |
| **78** | **0x4E** | **67** | **Astrolabe** | **NTS 新增** |
| **79** | **0x4F** | **68** | **（C→S 獨有）** | **NTS 新增** |
| 199 | 0xC7 | 69 | Reconnect | 共用 |

> ¹ 0x0A, 0x12, 0x16 在 TW 中也存在（標記為「僅 C→S」），但 CLAUDE.md §5 未列出名稱。
> TW 有 0x11（僅 C→S），NTS 已移除。

---

## NTS 新增 Opcode 詳細分析

### S→C 新增（7 個）

#### 0x38 — Stub（空殼）｜信心 95%
- **S→C handler**：`0x00706CB0`（11 bytes）
- 只有 3 條指令：讀 sub → 直接跳到 epilogue，不做任何事
- TW 原本此位置預留給某功能，NTS 保留空殼

#### 0x49 — WorldBoss（世界 BOSS 挑戰）｜信心 99%
- **S→C handler**：`0x007078E5`（20 bytes），轉呼叫 `[0x738A50]→0x5369C4`
- **C→S handler**：`0x006FC69F`（119 bytes）
- **Delphi 類別**：`TFWorldBoss`（VMT=`0x4D0AC4`）、`TFWorldBossPrize`、`TFWorldBossBuff`
- **字串證據**：
  - 「世界BOSS挑戰已經開始，請至涿郡城門找「世界BOSS挑戰員」進行討戰！」
  - 「本次挑戰世界Boss獲得績分為: %d」
  - 「恭喜您得到 世界BOSS挑戰賽全伺服單次傷害排名第 %d 名」
- **關聯 .dat**：`WorldBoss.dat`、`WBPrize.dat`、`WBScorePrize.dat`、`WBSumScorePrize.dat`
- **子指令碼**：4+ sub（活動開始/結束/獎勵/排名/績分）

#### 0x4A — NpcUpgrade（武將強化/進化）｜信心 95%
- **S→C handler**：`0x007078F9`（20 bytes），轉呼叫 `[0x738FA4]→0x5E4538`
- **C→S handler**：`0x006FC716`（333 bytes）
- **Delphi 類別**：`TFNpcUpgradeManage`、`TFNpcUpgradeForm`、`TFNpcUpgradeMsgMenu`
- **字串證據**：「武將強化成功」
- **關聯 .dat**：`EVOStatus.Dat`
- **子指令碼**：2 sub（sub=2: 強化結果、sub=3: UI 重置）

#### 0x4B — SaleRoom（拍賣場/交易所）｜信心 99%
- **S→C handler**：`0x0070790D`（20 bytes），轉呼叫 `[0x739450]→0x5136CC`
- **C→S handler**：`0x006FC863`（743 bytes — C→S 最大）
- **Delphi 類別**：`TFSaleRoomForm`
- **字串證據**：
  - 「上架成功」「上架物品資料異常」「拍賣場已無空間」
  - 「購買成功」「購買失敗」「下架成功」「下架失敗」
  - 「拍賣場忙碌中請稍後。」「拍賣場關閉中。」
  - 「點數不足」「請先下架到期商品」
- **子指令碼**：7 sub（列表/上架/購買/下架/狀態/關閉等）

#### 0x4C — ExpSlot（武將拉霸/經驗老虎機）｜信心 90%
- **S→C handler**：`0x00707921`（20 bytes），轉呼叫 `[0x738F08]→0x4F7A80`
- **C→S handler**：`0x006FCB4A`（622 bytes）
- **Delphi 類別**：`TFExpSlotForm`、`TFExpSlotPrzForm`
- **字串證據**：
  - 「「%s」在武將拉霸中拉到了頭獎，獲得了「500萬」經驗值和額外獎勵「%d」經驗值。」
  - 「功能尚未開放。」
- **子指令碼**：6 sub（開啟/轉動/結果/全服廣播/功能關閉等）

#### 0x4D — Activity（複合活動系統）｜信心 95%
- **S→C handler**：`0x00707935`（80 bytes — S→C 最大，handler 層級做子分發）
- **C→S handler**：`0x006FCDB8`（273 bytes）
- **Handler 層級分發**：
  ```
  sub=1 → GovRequire（官府徵召）→ [0x738818]→0x4FBF6C
  sub=2 → SwapItem（兌換活動）→ [0x739498]→0x511928
  sub=3 → EpicBattle（史詩戰役）→ [0x739408]→0x4FE218
  ```
- **Delphi 類別**：`TFGovRequire`、`TFSwapItem`、`EpicBattleManage`
- **字串證據**：
  - 「官府徵召活動已經開始囉！」「上繳成功」
  - 「兌換活動已經開始囉！」「兌換成功」
  - 「史詩戰役關閉中。」
- **關聯 .dat**：`GovRequire.Dat`、`SwapItem.Dat`
- **關聯 UI**：`form_GovReward.bmp`、`form_GSN.bmp`

#### 0x4E — Astrolabe（星盤系統）｜信心 99%
- **S→C handler**：`0x00707985`（17 bytes），轉呼叫 `[0x738880]→0x4FE980`
- **C→S handler**：`0x006FCEC9`（132 bytes）
- **Delphi 類別**：`TLH_AstrolabeLvUpImage`、`TLH_AstrolabeForm`
- **字串證據**：「星盤升級」
- **關聯 .dat**：`Astrolabe.Dat`
- **關聯 UI**：`from_astrolabe.bmp`
- **資料結構**：8 格子，每格 1 byte（sub=1 時讀取 count + 8 bytes）

### C→S 獨有新增（2 個）

#### 0x03 C→S — Look Request（請求外觀）
- **C→S handler**：`0x006EF223`（117 bytes）
- TW 中 0x03 僅 S→C（伺服器發送外觀）
- NTS 新增 C→S 方向 = 客戶端可主動請求查看其他玩家外觀
- 功能：構建封包（opcode + sub + 目標 ID），呼叫 `0x0050C080`（封包發送）

#### 0x07 C→S — PlayerDetail Request（請求玩家詳細資訊）
- **C→S handler**：`0x006EF5C5`（173 bytes）
- TW 中 0x07 僅 S→C（伺服器發送玩家詳細資訊）
- NTS 新增 C→S 方向 = 客戶端可主動請求查看其他玩家詳細屬性
- 功能：從全域物件讀取目標資訊，多次呼叫 `0x4B97D4`（WriteShort/WriteInt）

#### 0x4F C→S — （未命名，C→S 獨有）
- **C→S handler**：`0x006FCF4D`（132 bytes）
- 無對應 S→C handler
- 與 0x0A 結構相似（DEC AL + JNE，簡單封包構建器）
- 可能是某個新功能的請求封包，回應透過其他 opcode 的 sub 返回

### 未對應到新 opcode 的 NTS 獨有功能
- **FashionStress（時裝造型）**：`TFFashionStress`、`TFStyleAttrManage`、`FashionStress.Dat` — 可能透過現有 opcode 的子指令碼實現
- **NewPack（新背包）**：`form_NewPack.bmp` — 可能擴展 Item(0x17) 子指令碼
- **Style（造型）**：`from_Style.bmp`、`btn_Style`、`btn_StyleEQ` — 可能擴展 Outfit(0x3F)