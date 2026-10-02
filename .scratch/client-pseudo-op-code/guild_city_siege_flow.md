# Nghiên Cứu Luồng Client: Bang Hội (Guild) & Công/Thủ Thành Chiến

Ngày: 2026-10-02 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Tài liệu nền: [`post_login_stats_inventory_flow.md`](./post_login_stats_inventory_flow.md)
(`login_flow_research.md`, `opcode_*.md`), `client_pseudo_c/`, `TS_Server_Bear/TS_Server/`

> **Phương pháp.** Hai subagent nghiên cứu song song (guild / công-thủ thành) trên
> `client_pseudo_c/`. **Mọi kết luận dưới đây đã được tác giả đối chiếu lại độc lập**
> bằng cách decode trực tiếp `table_0x77F474_200B.hex` + `table_0x77F53C_dword200.hex`
> và đọc tay ASM/C. Phần bị bác bỏ được ghi rõ ở §6.

---

## 1. Kết Luận Quan Trọng Nhất

> **Không tồn tại main opcode riêng cho "công thành chiến" hay "thủ thành chiến".**
> Toàn bộ nghiệp vụ bang hội / công thành nằm trên **main opcode `0x27`** và
> **main opcode `0x2B`**.

| Main | Tên đúng | Nội dung |
|:--:|---|---|
| **`0x27`** | **Bang hội / Quân đoàn** (`OP_RANK_ANNOUNCE` hiện tại — **nhãn sai, phải đổi**) | Roster thành viên, 4 slot chức vị, mời/duyệt, kéo-và-xếp, thùng thư, lưu đồng, 26+9 banner, `27 10` mốc giải tán, `27 09` gán tên bang cho actor, **`27 1D` = 9 banner "đang công thành không thể…"** |
| **`0x2B`** | **Liên minh / Quan hệ quân đoàn** (`OP_COMPOUND` hiện tại — **nhãn sai**) | Ghép liên minh, hồi ứng, rời liên minh; **`2B 06` = 3 banner "đang công thành không thể liên minh/hồi ứng/rời khỏi"** |
| `0x3D` | Xổ số (`OP_LOTTO`; alias cũ `OP_GUILD_WAR` ở `protocol/mod.rs:226` **sai hoàn toàn**) | — |

**Client không bao giờ nhận frame "trận công thành bắt đầu".** Nó chỉ biết mình đang
trong chiến tranh công thành qua các frame **từ chối hành động**: server đẩy `27 1D 0K`
hoặc `2B 06 0b` → client hiện banner 2000 ms và **khoá chức năng quản lý bang hội**.
Đây là bằng chứng byte-level đã giải mã trực tiếp từ literal VISCII (§4.2).

### 1.1 Thủ thành chiến (thủ thành)

Thủ thành dùng **đúng cơ chế trận đánh TeamDef đã có sẵn**, không có frame riêng:
`0B FA` (`TFightField_Create` — frame duy nhất trong client tạo bàn cờ,
`.scratch/client-pseudo-op-code/battle/01_battle_start_scene.md:65-68`) với
`npc_type = 7` cho 10 NPC thủ thành ở 2 hàng trên (`src/battle/construction.rs:596-611`).
`DiaHinh` = `TeamDef[0]` (`CONTEXT.md:48`). Const `trigger_diahinh::TEAMDEF = 0` tại
`src/battle/construction.rs:554` hiện là **placeholder, chưa recover được giá trị thật**.

---

## 2. Bảng Opcode Đã Verify

### 2.1 C→S (Client → Server)

Bảng builder: `byte_table[0x77F474][main]` → index → `dword_table[0x77F53C][index]` → địa chỉ builder.

| Main | byte idx | Builder | Sub hỗ trợ | Bằng chứng |
|:--:|:--:|---|---|---|
| `0x27` | `0x24` (36) | **`0x0078734D`** | jump table **57 entry**, subsel `0x00..0x38` | ASM `0077f414_FUN_0077f414.asm.txt:2598-2602`; `0x00787362` |
| `0x2B` | `0x28` (40) | **`0x0078860B`** | **3 nhánh**: sub `1`, `2`, `3` | ASM `:2871-2879`, call `TForm1_CY_AddSedQueue` tại `:2896/2925/2955` |

Layout `0x2B` C→S (đã đọc tay ASM `0x00788628`):

| Sub | Payload sau `[main][sub]` |
|:--:|---|
| `1` | `LE32(*(mgr+0xE0))` — echo đúng id mà S→C `2B 01` đã gửi |
| `2` | `LE32(*(mgr+0xE0))` + `u8(mgr+0x38)` |
| `3` | *(rỗng — chỉ 2 byte header)* |

> **⚠️ Cả hai claim cũ "client never sends 0x27 / 0x2B" đều SAI.**
> `.scratch/…/opcode_27.md:220-221` và `opcode_2b.md:117-118` dùng `case 0x27: break;`
> của Ghidra làm bằng chứng — đúng lỗi recovery đã được cảnh báo ở
> `post_login_stats_inventory_flow.md:499-514`.

> **🔴 `0x27` C→S: body 57 builder chưa có trong corpus.** Địa chỉ `0x0078734D` và
> bảng con `0x00787362` (57×4 = 228 byte) **đã xác minh**, nhưng `index.csv`/`manifest.csv`
> không có entry nào trong khoảng `0x007873xx..0x0078820E`, và không có
> `table_0x787362*.hex`. **Đây là lỗ hổng lớn nhất của luồng C→S.**

### 2.2 S→C (Server → Client)

**`0x27`** → `jumptable_byte200_0x78A8EE[0x27] = 35` → `case_035_007938C3_FUN_007938c3.c`.

| Sub | Layout | Ý nghĩa | Mức |
|:--:|---|---|:--:|
| **`0x09`** | `[27][09]{[id:4LE][D:4LE][n:1][name:n]}×N` | Gán **tên bang** vào `player+0x478`, `player+0x5F0 = D`. Lặp tới hết RP | **A** |
| **`0x02`** | `[27][02][hdrLen:1][hdr][page:1][count:1]{[id:4][n:1][name:n][A][B][C][D][D1:4][D2:4][E:1][D3:4][tail:22]}×count[listLen:1][list][8B double][8B double][flag:1]` | **Roster đầy đủ**. Row stride 78B. 4 record đầu = chức vị (`mgr+0x0C..0x18`), record 1 copy sang `player+0x474`. `double` cuối = **mốc giải tán**; `flag` = trạng thái ally-func | **A** |
| `0x06` | `[27][06][id:4LE]` | Xoá 1 thành viên khỏi roster | A |
| `0x08` | `[27][08][id:4LE]` | Thêm/refresh 1 dòng roster | A |
| `0x0C` | `[27][0C][id:4LE]` | Nâng lên slot chức vị 2..4 | A |
| `0x0E` | `[27][0E][mode:1][id:4LE]` | Xếp lại chức vị (`mode` 0/2/3) | A |
| `0x03` | `[27][03][id:4LE]` | Chọn mục tiêu hiện tại (`mgr+4`) | A |
| `0x07` | `[27][07][code:1]`, `code=0` → `[00][id:4LE]` | Duyệt/từ chối lời mời | A |
| `0x05` | `[27][05][id:4][8B][flag:1][text…]` | Bảng thông báo bang (1000 entry) | A |
| `0x0B` | `[27][0B][str…]` | Đẩy chuỗi vào list global | A |
| **`0x10`** | `[27][10][D:8B double]` | `D != 0` → chat-log `FormatDateTime` + chữ "Giải tán" | **A** |
| **`0x0F`** | `[27][0F][K:1]`, `K ∈ {1..0x1B,0x64,0x65,0xFF}` | **26 banner**, 1200 ms | **A** |
| **`0x1D`** | `[27][1D][K:1]`, `K ∈ 1..9` | **9 banner 2000 ms — 5 cái là "công thành"** | **A** |
| `0x14` | `[27][14]` | Banner "Tổ chức thùng thư đã đầy!" | A |
| `0x04` | `[27][04][id:4LE][sub:1]` | Banner hành động lên actor | A |
| `0x01` | `[27][01][code:1]` | 6 banner 6000 ms; `code=0` → mở flow mời | A |
| `0x32` | `[27][32][mode:1][dword:4]` | ⚠️ `mode=0` **crash client**; `mode≥6` ghi ngoài vùng (bug phía client) | A |
| `0x33` | `[27][33][mode:1][dword:4][word:2]` | Timer/mode | A |
| `0x39`/`0x3A` | `[27][39][dword:4]` / `[27][3A][dword:4][b:1][w:2]` | Timer | A |
| `0x35`/`0x38`/`0x3B` | 2 byte | Xoá state + exec(8) | A |
| `0x0A,0x0D,0x11-0x13,0x15-0x22,0x1E-0x22,0x36,0x37` | — | **body chưa có trong dump** | — |

**`0x2B`** → `case_039_00794799_FUN_00794799.c`.

| Sub | Layout | Ý nghĩa | Mức |
|:--:|---|---|:--:|
| `0x01` | `[2B][01][id:4LE]` | `mgr+0xE0 = id` | A |
| `0x02` | `[2B][02][sel:1][id1:4LE]`(+`id2:4` nếu `sel==1`) | Mời liên minh / phản hồi | A |
| `0x03` | `[2B][03][flag:1]` | 4 banner 5000 ms | A |
| `0x04` | `[2B][04]{…}×N` | Batch cập nhật + sort | A |
| `0x05` | `[2B][05][blob…]` | Xoá 1 id (`00758270`) | A |
| **`0x06`** | **`[2B][06][b:1]`, `b ∈ {1,2,3}`** | **3 banner 2000 ms "công thành"** | **A** |

---

## 3. Đối Tượng Phía Client (cần để hiểu record)

`mgr = *gvar_007D9C20` — manager quân đoàn, dùng chung cho `0x27` và `0x2B`.

| Offset | Ý nghĩa |
|:--:|---|
| `+0x08` | Tên bang |
| `+0x0C..0x18` | 4 ID chức vị (commander, chief1/2/3) |
| `+0x20` | TStrings thông báo → form |
| `+0x28` (8B) | **Mốc giải tán** (`0.0` = không) |
| `+0x38` | Byte selector, echo ngược trong `27 02` C→S |
| `+0xB2` | Flag ally-func (chọn `Btn_EnableAllyFunc` vs `Btn_DisableAllyFunc`) |
| `+0xD4` | TList liên minh (`0x2B`) |
| `+0xE0` | ID thành viên đang chọn → gửi lên bằng `27 01/27 02/2B 01` |

`player = *gvar_007DA7BC`: `+0x474` = ID bang của chính mình · `+0x478` = tên bang ·
`+0x5F0` = field số của bang (level/exp/contribute — **chưa xác định**).

Roster table: `gvar_007DA6E8`, stride **78 byte**, 1-based, cap **103**.

> **Lead đã được loại trừ:** `0056b124_FUN_0056b124.c` (`Btn_ArmyList2`) và
> `00570ef4_FUN_00570ef4.c` (`Btn_EnableAllyFunc`) là **callback vẽ UI, đọc 0 byte wire**.
> Chúng chỉ chứng minh các form thuộc cụm quân đoàn — **không phải parser**.

---

## 4. Bằng Chứng Byte-Level Cho Công Thành Chiến

### 4.1 Byte thô đã giải mã

`client_pseudo_c/lit_798ea4_799208.hex` chứa **cả 9 chuỗi của `27 1D`** (mỗi chuỗi
Delphi AnsiString = `[FF FF FF FF][len:4LE][bytes][00]`), và
`lit_798eb4/798f14/798f7c.hex` chứa **3 chuỗi của `2B 06`**. Giải mã VISCII 1.1
(anchor: `src/encoding.rs:128-234`):

```
0x798eb4 (2B 06 b=1): "Trong chiến tranh công kích thành trì không thể chọn chức năng liên minh..."
0x798f14 (2B 06 b=2): "Trong chiến tranh công kích thành trì không thể chọn chức năng hồi ứng..."
0x798f7c (2B 06 b=3): "Trong chiến tranh công kích thành trì không thể chọn chức năng rời khỏi..."
```

Với `0x27 1D`, 5 chuỗi trong bộ 9 là: *rời khỏi* / *xóa bỏ* / *giải tán* /
*thiết lập quân đoàn* / *xóa nhân vật* (theo bảng hiệu đính `opcode_27.md:296-300`).

### 4.2 Đối chiếu tham chiếu C# (`TS_Server_Bear/TS_Server/`)

| File | Nội dung |
|---|---|
| `DataTools/GuildInfo.cs:3-30` | `id, name, rule, commander_id, chief1_id, chief2_id, chief3_id, commander_name, chief_count, icon[288], allie_id, resource[7], armory[22]` |
| `DataTools/GuildMemberInfo.cs` | `guildId, type, accId, contribute, packet[]` |
| `DataTools/GuildData.cs:85-102` | **Builder record thành viên 27 + nameLen byte** |
| `PacketHandlers/ActiveEventType.cs:10` | `CityWarGuild` — `/startevent 5` |
| `PacketHandlers/ActivityHandler.cs:196-197` | `case CityWarGuild: break;` — **rỗng hoàn toàn** |
| `QuestStepHelper/ArmyEffectHandler.cs:13-19` | `armypoint[type] += point`, `type` 1..5 = Kim/Thủy/Thổ/Hỏa/Mộc |

> **Bear chỉ có model + loader, KHÔNG có frame guild nào.** `grep "GuildData\."` → 0 kết quả;
> `guildList`/`packet[]` là **dead code**. `TSCharacter.sendArmyname()`/`replyToArmy()`
> (`:5900-5919`) cũng rỗng. ⇒ Framing ở §2 hoàn toàn suy ra từ client.

### 4.3 Layout record `27 02` — đối chiếu với Bear

Client đọc: `[id:4][n:1][name:n][A][B][C][D][D1:4][D2:4][E:1][D3:4][tail:22]`
Bear ghi: `[id:4][n:1][name:n][b2][b3][b4][b5][D1:4][D2:4][0][contribute:4][5][0][0][0][0]`.

Prefix khớp chính xác. Bear ghi **5 byte + 4 DWORD + 1 + contribute + 5 + 4**.
**Suy luận mạnh:** record = **27 + nameLen** byte, tail 22 byte của client tương ứng
byte `5,0,0,0,0` + byte thứ 2 của contribute. **Cần capture `27 02` thật để chốt.**

---

## 5. GAP CỦA SERVER RUST

### 5.1 🔴 CHẶN — Schema DB không tồn tại

`migrations/0001_init.sql` khai báo **18 bảng** (`accounts`, `characters`, `character_money`,
`inventories`, `character_pets`, `character_skills`, `character_hotkeys`,
`character_missions`, `character_mission_flags`, `character_bit_flags`,
`character_completed_events`, `friends`, `mails`, `item_code`, `gm_audit_log`,
`character_quest_tasks`, `character_quest_dont`, `character_quest_items`).
**Không có** `guilds`, `guild_members`, `world_bosses`, `admin_audit_log`.

Nhưng:
- `src/db/domain.rs:28-36` `list_guilds()` **đang query `guilds` + `guild_members`**
- `src/web/app.rs:199-200` gọi nó và `unwrap_or_default()` → **dashboard luôn rỗng, im lặng**
- `list_world_bosses()` và `write_audit()` cùng lỗi

**→ Tạo `migrations/0002_guild.sql`:**

```sql
CREATE TABLE guilds (
  guild_id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  leader_id INTEGER NOT NULL,
  level INTEGER NOT NULL DEFAULT 1,
  experience INTEGER NOT NULL DEFAULT 0,
  treasury INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'normal',   -- normal | disband_pending
  allie_id INTEGER,                          -- 0x2B liên minh
  resource TEXT, armory TEXT,                -- JSON, Bear resource[7]/armory[22]
  disband_at REAL,                           -- -> mgr+0x28
  created_at INTEGER NOT NULL
);
CREATE TABLE guild_members (
  guild_id INTEGER NOT NULL REFERENCES guilds(guild_id),
  char_id  INTEGER NOT NULL,
  role     INTEGER NOT NULL,   -- 1 commander / 2 chief / 3 member
  contribute INTEGER NOT NULL DEFAULT 0,
  UNIQUE(char_id)
);
```

### 5.2 🔴 Nhãn opcode sai + không có route

| Hiện tại | File | Sửa thành |
|---|---|---|
| `OP_RANK_ANNOUNCE = 0x27` | `src/protocol/mod.rs:164` | `OP_GUILD_QUAN_DOAN` (alias deprecated cũ) |
| `OP_COMPOUND = 0x2B` | `src/protocol/mod.rs:176` | `OP_GUILD_ALLIANCE` |
| `OP_GUILD_WAR = OP_LOTTO` (0x3D) | `src/protocol/mod.rs:226` | xoá — hoàn toàn sai |
| `OP_GUILD = OP_ACCOUNT` (0x23) | `src/protocol/mod.rs:150` | rename đã đúng, giữ alias |

Cả hai `0x27` và `0x2B` đang rơi vào arm `unimplemented::handle`
(`src/server/dispatcher.rs:371,374`) → **chỉ log, không trả lời → UI treo**.

**→ Tạo `src/server/handlers/guild.rs`, route trước arm `unimplemented`.**

### 5.3 🔴 BUG SỐNG — `27 09` ghi sai tên bang cho mọi người chơi

```rust
// src/server/spawn.rs:304-314
pub fn server_name_frame(id: u32, server_name: &str) -> String {
    body.push_str(&encoder::le32(id));
    body.push_str("C4000000");       // 0xC4 = 196 -> player+0x5F0
    body.push_str(&format!("{:02X}", name_len));
    body.push_str(&encoder::strhex(&visc));   // -> player+0x478
}
// src/server/spawn.rs:567 — gọi với "TSVN" cho MỌI player trong Logined1
```

Đọc tay `00754360_FUN_00754360.c:53-116` (xem §2.2 sub `0x09`):
`player+0x478 = <name>` và `player+0x5F0 = <D>`.

⇒ **Mỗi lần login, client hiển thị tên bang là `"TSVN"` và field số là 196 cho mọi nhân vật.**
Layout đúng vô tình, nhưng **nội dung sai hoàn toàn** — và đây chính là frame mà
client dùng để render tên bang trên đầu nhân vật.

**→ Thay bằng `guild_info_frame(char_id, guild_id, level_or_exp, guild_name)` lấy từ DB;
không có bang thì không gửi (hoặc `nameLen = 0`).**

### 5.4 🟠 Session không mang trạng thái bang

`grep -ri guild src/` chỉ ra `db/domain.rs`, `web/app.rs`, `protocol/mod.rs`, `chat.rs`.
**`src/server/session.rs` không có `guild_id`/`guild_name`.**
→ Thêm 2 field + nạp trong `src/db/modern/sqlite/session.rs:19-114`.

### 5.5 🟠 `armypoint` (điểm công thành theo hệ) chưa mô hình hoá

Bear: `TSCharacter.cs:222,374,665` (`Dictionary<string,ushort>` 5 key "1".."5"),
cập nhật ở `ArmyEffectHandler.cs:16-19`, đọc từ DB column JSON `armypoint`.
Rust `src/eve/` **không có** hiệu ứng `GetArmy` tương đương.

### 5.6 🟡 Các gap nhỏ

| # | Vấn đề | Vị trí |
|:-:|---|---|
| a | `trigger_diahinh::TEAMDEF = 0` là placeholder, chưa recover giá trị thật của `TeamDef[0]` | `src/battle/construction.rs:553-554` |
| b | `CityEx.Dat` đã parse (`Vec<[f64;7]>`) nhưng **không nơi nào đọc** | `src/data/loaders/city_ex.rs` |
| c | Chat sub `0x06` (kênh bang) chỉ echo về sender | `src/server/handlers/chat.rs:222-233` |
| d | Body 57 builder C→S của `0x27` không có trong corpus — **cần redump `aLogin.exe`** | `client_pseudo_c/` |
| e | Header C→S là `[main][sub]` hay `[sub][main]`? ASM dựng shortstring sub-trước-main-trước rồi `FUN_0040402C` (chưa dump) — **câu hỏi mở toàn repo, chặn mọi C→S parser** | xem §6 R5 |

---

## 6. Các Claim Cũ Bị Bác Bỏ

| # | Claim cũ | Nguồn | Phán quyết |
|:-:|---|---|---|
| **R1** | "`0x27` = `OP_RANK_ANNOUNCE`, không phải guild" | `opcode-rename/issues/05:14`, `protocol/mod.rs:164` | ❌ **Sai một phần.** Handler `0x27` chứa literal *Quân đoàn* (tạo/rời/giải tán), luật **10 đoàn viên cấp 15**, và gán tên bang vào `player+0x478`. Là kênh quản lý bang hội. Nhãn "RankAnnounce" thiếu ngữ nghĩa. |
| **R2** | "`0x23` 'Guild' là nhãn sai → `OP_ACCOUNT`" | `issues/05:10`, `CONTEXT.md:213` | ✅ **Đúng** — `opcode_23.md:194-215` là đổi pass/xoá nhân vật/thẻ quà. |
| **R3** | "`0x24` 'GuildInfo' là nhãn sai → `OP_JOB_CHANGE`" | `issues/05:11` | ✅ **Đúng** — 25 nhánh fan-out UI, không literal guild. |
| **R4** | "`0x2B` = `OP_COMPOUND` (Luyện đồ)" | `protocol/mod.rs:176`, `server_main_opcode.md:41` | ❌ **Sai.** Handler `FUN_00794799` là manager liên minh; C→S builder `0x0078860B` không liên gì đến luyện đồ. |
| **R5** | "C→S `case 0x27: break;` → client không bao giờ gửi" | `opcode_27.md:220-221` | ❌ **Sai.** Builder `0x0078734D` + bảng 57 subsel tại `0x00787362`. |
| **R6** | "C→S `case 0x2b: break;` → client không bao giờ gửi" | `opcode_2b.md:117-118` | ❌ **Sai.** Builder `0x0078860B`, 3 nhánh sub 1/2/3, đều `CALL 0x0051633C`. |
| **R7** | "`27 09` = Server name + id LE32" | `post_login_stats_inventory_flow.md:113`, `spawn.rs:304` | ❌ **Sai.** Là frame gán **tên bang**. `spawn.rs:567` đang ghi "TSVN" vào ô tên bang của mọi player. |
| **R8** | "`Btn_ArmyList2` / `Btn_EnableAllyFunc` là candidate cho guild protocol" | lead trong brief | ❌ **Sai với vai trò parser.** Là callback vẽ UI, đọc **0 byte wire**. Chỉ dùng để định danh form của cụm quân đoàn. |
| **R9** | "`OP_GUILD_WAR = 0x3D`" | `protocol/mod.rs:226` | ❌ **Sai hoàn toàn** — 0x3D là Xổ số (Lotto). |
| **R10** | "Bear có guild protocol tham chiếu" | giả định | ⚠️ **Một nửa.** Bear có model + DB loader + member-record builder, nhưng **0 frame**, `loadGuilds()` không bao giờ được gọi. `CityWarGuild` là `case: break;` rỗng. |

> **⚠️ Sai số đã phát hiện ở subagent:** bảng tra cứu thứ tự `0x27 ↔ 0x2B` bị đảo trong
> một báo cáo. Decode trực tiếp 2 bảng hex cho kết quả chuẩn:
> `byte[0x27]=0x24 → builder 0x0078734D`; `byte[0x2B]=0x28 → builder 0x0078860B`.
> Bảng ở §2 là bản đã verify.

---

## 7. Thứ Tự Triển Khai Đề Xuất

1. **`migrations/0002_guild.sql`** — mở khóa `src/db/domain.rs:28` + dashboard.
2. **Đổi nhãn opcode** trước khi ai đó build trên nhãn sai:
   `0x27 → OP_GUILD_QUAN_DOAN`, `0x2B → OP_GUILD_ALLIANCE`, xoá alias `OP_GUILD_WAR`.
   Cập nhật `spec/server_main_opcode.md:37,41` + `CONTEXT.md:213`.
3. **Sửa `spawn.rs:304-314` + `:567`** — `guild_info_frame()` thật, không gửi khi
   không có bang. *Gỡ bug sống ngay, không cần chờ phần còn lại.*
4. **Thêm `guild_id`/`guild_name` vào `Session`** + load từ `sqlite/session.rs`.
5. **`src/server/handlers/guild.rs`** — xử lý `27 01`/`27 02` (payload **ASCII thập phân**,
   xem `FUN_0077EC70`), trả `27 02` roster, `27 09` self-info, `27 10` mốc giải tán.
6. **Subset công thành 100% chắc chắn & 3 byte, làm trước:**
   `27 1D 0K` (`K=1..9`) + `2B 06 0b` (`b=1..3`). Không parse payload, mock được ngay,
   và là **các frame duy nhất client hiểu về công thành**.
7. **`army_war_points`** (char_id, element 1..5, points) + hiệu ứng Eve `GetArmy`,
   port đúng `ArmyEffectHandler.cs:13-19` (cộng dồn, `u16` no-tràn).
8. **Đóng chat sub `0x06`** khi đã có membership.
9. **Redump `0x0078734D..0x0078820E`** (57 builder + bảng `0x00787362`) — lỗ hổng
   lớn nhất của luồng C→S.
10. **Giải quyết R5** (`[main][sub]` hay `[sub][main]`) — dump `0x0040402C` hoặc
    capture `08 01`. **Chặn mọi parser C→S, không riêng guild.**

---

*Tài liệu tổng hợp từ 2 lượt subagent + đối chiếu độc lập của tác giả ngày 2026-10-02.
Mọi claim đã được phân loại **A (đã verify byte-level)** / suy luận mạnh / **mở**.
Nguồn chính: `client_pseudo_c/`; `TS_Server_Bear/` chỉ dùng đối chiếu model.*