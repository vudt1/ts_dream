# Research: Chat trên bản đồ — Opcode `0x02` (public / mật / đội / slash command)

Ngày: 2026-09-19 · Phạm vi: server Rust (`src/`), client aLogin decompile (`client_pseudo_c/` + `.scratch/client-pseudo-op-code/opcode_02.md`), server C# tham khảo (`TS_Server_Bear/`).
Trạng thái: đã đọc mã nguồn sơ cấp cả ba phía, chưa chạy packet-capture live (mục mở §8-P0.4).

---

## 1. Wire format (chuẩn chung, đã đối chiếu 3 nguồn)

Framing chung: `[F4 44][len:2B LE][payload]`, toàn khung XOR `0xAD` (`src/protocol/mod.rs:7-11`, `TS_Server_Bear/TS_Server/PacketProcessor.cs:37-48`, `opcode_02.md §2`).

### 1.1 S→C (server → client) — nguồn chân lý là `opcode_02.md §2-§4`

Payload: `[0x02][SubOp:1B][id:4B LE][msg:bytes tới hết gói]`, ngoại lệ `0x0C` không có `id` (`msg = RP[1..]`, `id` hằng `0`).

| SubOp S→C | Tag `FUN_007ab870` | Nhãn client | Gate phía client | Ghi chú |
|---|---|---|---|---|
| `0x00` | 0 | `(Công bố hệ thống)` | không gate | broadcast hệ thống |
| `0x01` | 1 | `(Thần)Thiên thần` / `(Gần)Thiên thần` | **gắt**: `class∈[5..8]` VÀ (`ChannelForm+0x16D` HOẶC `PlayerRec+0x48==0xB3B6`) | GM-broadcast có điều kiện; id 100..400 đổi nhánh NPC |
| `0x02` | 2 | `(Gần)` | `ChannelForm+0x168` | chat kênh gần / map chat |
| `0x03` | 3 | `(Thì Thầm)` / `(GM)Thiên thần` / `Hệ thống:` | `ChannelForm+0x169` | whisper; id 100..400 = nhánh NPC |
| `0x04` | 4 | `(GM)` | không gate | nhãn GM |
| `0x05` | 5 | `(Đài)` | `ChannelForm+0x16A` + **chỉ hiển thị khi resolve được tên** | loa/megaphone |
| `0x06` | 6 | `(Đoàn)` | `ChannelForm+0x16B` | đoàn/phái |
| `0x07` | 7 | `(Minh)` | `ChannelForm+0x16C` | tự nói/bản địa |
| `0x08` | — | (không phải tin nhắn) | `class∈[5..8]` | gắn cờ `InputBar+0x1A5/0x1A8` (cooldown nhập) |
| `0x0B` | 0x0B | `(Tổng Cũ)` + gom tới `#end` | không gate | memo dài chia nhiều gói |
| `0x0C` | 0 | `(Công bố hệ thống)` | không gate | dòng hệ thống thuần, `id=0` |

Chi tiết: `.scratch/client-pseudo-op-code/opcode_02.md:71-140` (bảng sub), `:144-166` (bảng nhãn VISCII đã decode), `007ab870` hội tụ + ignore-list `DAT_00949284` + ring-buffer 100 dòng (`:57-67`).

### 1.2 C→S (client → server) — điểm mâu thuẫn lớn nhất

- **Bear dialect** (server C#): C→S dùng chính opcode `0x02`, `data[0]=0x02`, `data[1]=sub (1..7)`, body `[msg]` (sub 1,2,4,5,6) hoặc `[targetID:4B][msg]` (sub 3). `PacketProcessor.cs:84-86` route `case 2 → new ChatHandler(client, data)`; `ChatHandler.cs:64-129` switch `data[1]`.
- **aLogin dialect** (client decompile): `FUN_0077f414` (`TFConnect.SendCommand`) có `case 2: break;` — **C→S opcode 0x02 rỗng**, client không gửi chat bằng 0x02 (`opcode_02.md:171-180`). **Đính chính (2026-09-19): C→S opcode 0x1A cũng rỗng** (`case 0x1a: break;`, 0/110 điểm gọi gửi 0x1A) — 0x1A là **MoneySync, kênh S→C đồng bộ tiền/bộ đếm số + banner** (10 sub-op, payload số fixed-width, không chuỗi; xem `opcode_1a.md §0-§5`). Các khuôn "opcode + text tự do" của chat input nằm ở **case 0x37** (text của input chat) và **0x1D/0x3A/0x3B** (`opcode_1a.md §5`), **không phải 0x1A**. Phía Rust, C→S 0x1A đã có handler riêng `handle_pc_talk` (`src/server/handlers/npc_event.rs:28-82`) chỉ parse selector số cho talk (không phải chat) — nhất quán với tài liệu.
- **Rust hiện tại**: chỉ nghe C→S `0x02` (`src/server/dispatcher.rs:248-249` → `chat::handle_chat`), body sau opcode/sub (`dispatcher.rs:216-218`: `opcode=decoded[4]`, `sub=decoded[5]`, `payload=decoded[6..]`).

→ Hệ quả ở §8-P0.4: nếu client vận hành là aLogin thật, toàn bộ chat gõ từ client đi đường 0x1A và **không bao giờ chạm** `handle_chat`; nếu client là Bear-dialect thì OK. Đây là unknowns lớn nhất, phải xác minh bằng capture trước mọi fix khác.

---

## 2. Bear C# xử lý opcode 0x02 thế nào (nguồn: `TS_Server_Bear/TS_Server/PacketHandlers/ChatHandler.cs`)

### 2.1 Thứ tự xử lý trong ctor (`ChatHandler.cs:47-130`)

1. `specialMsg(client, data)` trước mọi thứ (`:48-52`) — slash command được chặn **trên mọi sub**, dùng raw bytes `data[2..]` decode `Encoding.Default` (`:138`).
2. `gmchat` mode: ghi đè `data[1] = 4` (`:53-56`) — mọi tin thành kênh GM.
3. `syschat` mode: build `FillMessageData(client, 11, msg)` + `replyToAll(..., self:false)` (`:57-63`) — **không** `return`, rơi tiếp xuống switch (vừa gửi sub `0x0B` vừa gửi tiếp kênh gốc — double-send có chủ ý/không chủ ý).
4. Switch `data[1]`:
   - `1`: `FillMessageData(client,1,msg)` + `replyToAll(self:false)` — world chat, **sender không nhận echo**.
   - `2`: `FillMessageData(client,2,msg)` + `replyToMap(self:false)` — map chat, **sender không nhận echo** (client tự hiện text mình gõ tại chỗ).
   - `3` (whisper): `target = read32(data,2)`, đồng thời `client.targetIDBt = target`; chỉ gửi khi target online; `FillMessageData(client,3,msg)` — **id trong frame là `client.accID` (người gửi)**, gửi cho cả target (`reply`) lẫn sender (`reply`). Không báo lỗi khi target offline (rơi im).
   - `4`: `if (gm==1) { msg = new ... }` rồi `Array.Copy(data,2,msg,...)` **ngoài** if (`:99-108`) — khi `gm != 1`, `msg` null → `NullReferenceException` văng lên `processPacket` catch. Thực tế chỉ GM mới gửi được sub 4 mà không crash.
   - `5`: `replyToTeam` — gửi **tất cả members kể cả sender** (không loại trừ).
   - `6`: `replyToArmy(self:false)` + `reply` cho sender. Nhưng `replyToArmy` là **hàm rỗng** (`TSCharacter.cs:5917-5919`) → army chat thực tế chỉ echo cho sender, không tới ai khác.
   - `7`: `break` — no-op.
   - Không có case `0/8/0x0B/0x0C` ở chiều C→S (đúng, đó là S→C-only).

### 2.2 `FillMessageData` (`ChatHandler.cs:2112-2126`)

```
PacketCreator(2) + add8(type) + add32(type==11 ? 0 : client.accID) + addBytes(message)
```

- Mọi kênh (kể cả whisper sub 3) mang **id người gửi**. Sub `0x0B` mang id `0`.
- `announce(msg)` = `FillMessageData(client, 11, ...)` + `reply` cho chính mình (`TSCharacter.cs:1275-1281`).
- `sendGMMessage` = opcode 2 sub 4 id 0 (`TSCharacter.cs:1283-1293`).

### 2.3 Broadcast semantics (`TSCharacter.cs:4863-4920`, `TSMap.cs:326-346`)

- `reply(data)` → socket của chính mình (guard `client.online`).
- `replyToMap(data, self)` → `TSMap.BroadCast`: nếu `self==true` thì gửi cho mình trước, sau đó gửi mọi `listPlayers` cùng map trừ sender. Bear gọi với `self:false` cho chat → sender không echo.
- `replyToAll(data, self)` → lặp mọi map trong `TSWorld`, mỗi map `BroadCast(client, data, self)`.
- `replyToTeam(data)` → gửi mọi `party.member` (kể cả sender); `replyToTeamNotme` loại sender (chat không dùng hàm này).

### 2.4 Slash commands Bear (`ChatHandler.cs:132-850`)

- `specialMsg`: parse `text.split(' ', 2)`, `command` lowercase, `arguments` phần còn lại.
- Public (mọi người, kể cả `gm != 25`): `/bot /autoboom /nb /9pet /resetchar /resetpet /autosell /warp /warpguide /exchange /startevent /addq /sleep /removeq /removeqd /additem /addstat /enemypower /where /battle /questbot` (`:165-215`).
- `gm == 25` (GM99) mới được đi tiếp vào `HandleGm99Command` (~70 lệnh: `/help /bt /btg /addpet /addgold /level /tp... /broadcast? không — Bear không có /broadcast, có /allmsg /gm /goto /kick /save /spk /op /pks /list ...`). Người thường gõ `/lạ` → `return true` (nuốt, không broadcast) (`:152-155`). GM99 gõ lệnh lạ → `HandleGm99Command` trả `false` → **rơi xuống chat thường và broadcast luôn text `/lạ...`** (rò lệnh, hành vi gốc).
- `/where` (`:205-207`): `announce("MapID: " + mapID + " X: " + mapX + " Y: " + mapY)` — chú ý có space sau colon.
- `/sleep` (`:1353-1371`): heal cả party nếu có party (không phân biệt leader/member), `announce("Sleep command executed.")`, **không chặn khi đang battle**.
- `/warp` (`:890-933`): chỉ party-leader được dùng, cấm khi `battle != null`, validate map 5 chữ số + tồn tại trong `EveData.mapList`, default x=y=500.

---

## 3. Server Rust hiện tại làm gì (nguồn: `src/server/handlers/chat.rs`, `src/server/spawn.rs`, `src/server/gm.rs`)

### 3.1 Routing & frame builder

- `dispatcher.rs:248-249`: `OP_CHAT (0x02) → chat::handle_chat`.
- `spawn::chat_frame(sub, id, chat_raw)` (`spawn.rs:228-234`): body = `le32(id) + hex(chat_raw)`, code `02+sub` — echo **nguyên bytes client**, không re-encode (đúng, tránh mojibake).
- `sys_msg_frame` = `020B` id 0 + `viscii_encode(msg)` (`spawn.rs:247-250`); `announce_frame` = `020C` tương tự (`:252-255`).

### 3.2 Từng sub trong `handle_chat` (`chat.rs:44-152`)

| Sub C→S | Rust làm | Bear làm | Nhận xét |
|---|---|---|---|
| 1 (world) | decode VISCII, drop nếu >60 chars, `chat_frame(1, self_id, payload)` + `out.send` (echo sender) + `broadcast_except` | `replyToAll(self:false)`, không echo, không giới hạn độ dài | Rust thêm echo + giới hạn 60 — khác Bear |
| 2 (map/global+slash) | decode, drop >60 chars; nếu `/...` → `gm::handle` rồi `handle_slash`, `return` (không broadcast). Chat thường: nếu đeo item slot6 id 23100 → **đổi thành sub 1 broadcast toàn server**, else sub 2 `broadcast_map` (map sender) | slash bắt ở mọi sub; sub 2 luôn map-only, không có item promotion | promotion 2→1 là logic riêng của Rust (nguy cơ §8-P1.6) |
| 3 (whisper) | `payload<4` → drop; `target=le32(payload[0..4])`, `chat_raw=payload[4..]`, drop nếu `chat_raw.len()>60` (đếm **bytes**); `chat_frame(3, target_id, ...)` gửi cho sender (`out.send`) + `hub.send_to(target)` | frame mang **sender id**, gửi cả 2 đầu, set `targetIDBt`, check online | **Sai id** — Rust mang recipient id (§8-P0.1). Đếm bytes thay vì chars, không nhất quán với sub 1/2 |
| 4 (GM bcast) | echo + `broadcast_except` toàn server, **không check quyền** | chỉ `gm==1` mới build được (dù code crash khi khác) | **Lỗ hổng giả mạo GM** (§8-P0.2) |
| 5 (party) | echo + `send_to(leader)` + `send_to` từng `id_mem`, loại `self` | `replyToTeam` (live `party.member`, gồm cả sender) | Rust dùng snapshot `id_leader/id_mem` của sender, không verify sender thuộc party (§8-P1.10) |
| 6 (army/guild) | echo + `broadcast_except` **toàn server** | `replyToArmy` = no-op + echo sender (thực tế không gửi ai) | **Sai scope hoàn toàn** (§8-P0.3) |
| 7 | `_ => {}` (drop) | `break` (no-op) | khớp |
| 0/8/0x0B/0x0C C→S | `_ => {}` (drop) | không có case (drop) | khớp (S→C-only) |

### 3.3 Slash & GM (`chat.rs:154-333`, `gm.rs:1-471`)

- Player commands Rust: `/where /endtalk /sleep /openhotel /openbank /openstore`, còn lại drop im (`chat.rs:179-332`).
- GM commands (`gm.rs:48-69`): `/gm /gmhelp /additem /addgold /setgold /level /tp /goto /summon /broadcast /kick /setperm /setgm /battle /quest /setflag /test`; gate `authed && logined && gm_level > 0`, nếu không phải GM thì **nuốt im** (`gm.rs:169-173`) — tương đương Bear (không lộ sự tồn tại của lệnh).
- `/where` Rust: `"MapID:{} X:{} Y:{}"` qua `sys_msg_frame` (`chat.rs:183-187`); GM `/where` thêm `GM:{level}` (`gm.rs:179-185`). Khác Bear 1 space sau colon (cosmetic nhưng byte-khác).
- `/sleep` Rust (`chat.rs:194-284`): chặn khi `battle_id>0`; heal self + pets stt 1..4 + **chỉ lan sang members khi sender là leader** (`id_leader==player_id`); persist `Hp/Sp`/pets; gửi `F44402001F0A ... F44403001F0100`. Bear: heal cả party không phân biệt leader, không chặn battle, có `announce`, có `RestoreExtraBattlePets`. Hai implement khác nhau cả điều kiện lẫn phạm vi.
- Thiếu so với Bear public: `/warp /warpguide /exchange /bot /autoboom /nb /9pet /resetchar /resetpet /autosell /addq /removeq /removeqd /addstat /enemypower /battle /questbot` — tức toàn bộ nhóm bot/auto/battle-pet/quest/warp của Bear không tồn tại trên Rust (đúng nếu cố ý lược bot, nhưng `/warp /exchange /offq` là nhu cầu người chơi thật).

---

## 4. Server Rust đã đáp ứng được yêu cầu client chưa?

**S→C (hiển thị): có, đúng format.** `chat_frame` + `020B/020C` khớp layout `[02][sub][id][msg]` mà `opcode_02.md` đặc tả; echo raw bytes giữ VISCII nguyên vẹn; client sẽ render đúng nhãn `(Gần)/(Thì Thầm)/(GM)/(Đài)/(Đoàn)/(Minh)/(Công bố hệ thống)` nếu sub đi kèm đúng.

**C→S (gửi): chưa chắc — phụ thuộc dialect client.** Với Bear-dialect thì handler bao phủ đủ 6 kênh + slash; với aLogin (decompile hiện có) thì C→S 0x02 rỗng **và** C→S 0x1A cũng rỗng (0x1A là MoneySync S→C-only, `opcode_1a.md §0,§5`) — chat từ client aLogin đi qua các khuôn opcode + text ở **0x37/0x1D/0x3A/0x3B**, Rust hiện chưa nghe cửa nào trong số đó. Không thể kết luận "đáp ứng" trước khi xác minh §8-P0.4.

**Routing/phạm vi: chưa.** Whisper sai id, sub 4 thiếu gate, sub 6 sai scope toàn server, party dùng snapshot thay vì registry — cả 4 đều làm client hiển thị sai hoặc rò tin.

---

## 5. Vấn đề đang có (xếp hạng) và tiềm ẩn

### P0 — sai đúng / bảo mật

- **P0.1 Whisper mang sai id** (`chat.rs:106-115`). Rust đóng `target_id` vào frame gửi cho **cả hai đầu**; Bear (`FillMessageData(client,3,msg)` + `add32(client.accID)`) và client (`FUN_007ab870`: `myId==id → tên mình`, else tra cache) đều yêu cầu **sender id**. Hậu quả: bên nhận thấy tin từ chính mình (tên self), bên gửi thấy tin từ đối phương; kênh `(Thì Thầm)` mất tác dụng + `FUN_005961a0` log whisper ghi sai nguồn.
- **P0.2 Sub 4 không gate GM** (`chat.rs:118-124`). Bất kỳ client nào cũng broadcast nhãn `(GM)` toàn server. Bear (dù code lỗi NRE) có ý định chỉ `gm==1`. Cần gate `gm_level > 0` (hoặc tách: player gửi sub 4 → drop/audit).
- **P0.3 Sub 6 broadcast toàn server** (`chat.rs:143-149`). Chưa có khái niệm army/guild membership trên Rust; Bear `replyToArmy` là no-op nên thực tế Bear không phát tán. Rust hiện tại rò chat "đoàn" ra toàn cụm — vừa sai scope vừa là kênh spam toàn server không kiểm soát.
- **P0.4 Dialect C→S chưa xác minh** (aLogin decompile: C→S 0x02 rỗng theo `opcode_02.md §6`, C→S 0x1A cũng rỗng vì 0x1A là MoneySync S→C-only theo `opcode_1a.md §0,§5`; ứng viên gửi chat thật là các khuôn opcode + text ở **0x37/0x1D/0x3A/0x3B**). Nếu client thật gửi chat qua các opcode đó thì mọi fix 0x02 đều vô nghĩa với người chơi. Cần capture + đọc send-site trước khi chốt scope (kế hoạch §9-G0).

### P1 — lệch hành vi / thiếu tính năng

- **P1.5 Echo khác Bear.** Rust `out.send` echo cho sender ở sub 1/2/4/6 (`chat.rs:52,75,83,120,144`); Bear `self:false` không echo (client tự hiện text mình). Nếu client Bear-dialect tự hiện + nhận thêm echo → **hiển thị trùng 2 dòng**. Cần xác minh phía client rồi chọn 1 (khuyến nghị theo Bear: bỏ echo, hoặc giữ echo và xác nhận client không tự hiện).
- **P1.6 Promotion 2→1 bằng item 23100/slot 6** (`chat.rs:28-35,73-80`). Sub 1 phía client bị gate gắt (`class∈[5..8]` + cờ `+0x16D`/magic `0xB3B6` — `opcode_02.md:103-106`): đa số người chơi thường **không thấy** tin "global" này. Bear không có cơ chế tương đương. Nếu giữ, nên phát global bằng sub `0x00`/`0x0C`-style hoặc world channel riêng đã kiểm chứng gate, kèm test client thật.
- **P1.7 Giới hạn 60 ký tự không đồng nhất + drop im.** Sub 1/2 đếm `chars()` (`chat.rs:48,60`), sub 3 đếm bytes (`:108`), sub 5/6 không check. Bear không giới hạn. VISCII 1 byte/char nên chars≈bytes, nhưng drop im lặng gây mất tin khó debug; ít nhất phải log + (cân nhắc) trả `020B` báo "tin quá dài".
- **P1.8 Slash chỉ bắt ở sub 2** (`chat.rs:57-70`). Bear `specialMsg` chạy trước switch nên `/lệnh` ở mọi sub đều bị chặn. Rust: `/where` gõ ở kênh whisper/party/world sẽ **broadcast nguyên văn** ra kênh đó (rò lệnh + spam).
- **P1.9 Thiếu public commands Bear** (§3.3 cuối). Tối thiểu cần quyết định số phận từng lệnh: giữ (`/warp /exchange /sleep /where /offq≈/endtalk`), bỏ có chủ ý (nhóm bot/auto: `/bot /autoboom /autosell /ai...`), stub báo "chưa hỗ trợ" thay vì drop im.
- **P1.10 Party chat dùng snapshot sender** (`chat.rs:130-140`: `id_leader/id_mem` của người gửi). Không verify người gửi thuộc party, không dùng live registry như Bear `party.member`. Snapshot lệch (đổi leader, rời party, member offline) → tin lạc/mất/thiếu. Cũng không xử lý party rỗng (gửi sub 5 khi không có party → chỉ echo mình, Bear cũng vậy nhưng nên báo `020B`).
- **P1.11 Thiếu `gmchat/syschat` mode** (Bear `/gm /allmsg`, `ChatHandler.cs:53-63,506-531`). Rust không có tương đương; GM muốn phát ngôn kênh hệ thống phải dùng `/broadcast` (020C) — chấp nhận được nhưng là regression so với Bear nếu GM quen `/gm`.

### P2 — tiềm ẩn / hardening (Bear cũng thiếu, production nên có)

- Không rate-limit/flood-control, không mute/ban, không audit log chat (Bear cũng không; `gm.rs` có audit cho GM commands nhưng chat thường không).
- Whisper tới offline player mất im cả hai phía (Bear cũng vậy). Nên trả `020B` "người chơi không online".
- Không cập nhật `targetIDBt` như Bear (`ChatHandler.cs:84`) — field này còn dùng cho `/look`/battle observe; bỏ qua có thể làm lệch tính năng khác.
- `/sleep` Rust ≠ Bear (điều kiện battle, phạm vi leader-only, thiếu `announce` + `RestoreExtraBattlePets`). Cần chốt spec heal rồi test cả hai nhánh leader/member/solo.
- Thang GM khác Bear (`gm==1/25` vs `gm_level 0/1/10/50/99`): mọi gate sub 4/GM chat sau này phải ánh xạ rõ, tránh "GM Bear cũ" mất quyền hoặc "player mới" lọt quyền.
- Mâu thuẫn nhãn sub 5: Bear gọi sub 5 là PARTY nhưng client aLogin render sub 5 là `(Đài)` (megaphone). Một trong hai dialect đã dùng sai nhãn từ đầu — cần chốt bằng client thật trước khi gán nghĩa "party chat" cho sub 5.
- Không có test chat nào trong `tests/` (hiện có `encoding_test`, `movement_warp_test`... nhưng 0 file chat). Mọi fix trên đều phải có test theo `AGENTS.md` (test trong `tests/`, cấm `#[cfg(test)]` trong `src/`, DB test dùng memory/tempfile, không chạm `DB/ts_dream.db`).

---

## 6. Kế hoạch giải quyết tồn đọng

### G0 — Xác minh dialect trước khi sửa (điều kiện tiên quyết, ~0.5-1 ngày)

1. Capture live 1 phiên gõ chat từng kênh (gần/thì thầm/đội/đoàn) + whisper + `/where`: ghi raw frame C→S (op thực tế là 0x02 Bear-dialect hay 0x37/0x1D/0x3A/0x3B? sub nào? whisper có prefix id không?).
2. Đọc send-site phía client: xác minh khuôn "op + text" ở `FUN_0077f414` case 0x37 (dòng ~1012-1019) và 0x1D/0x3A/0x3B theo `opcode_1a.md §5`; xác nhận client vận hành là Bear-dialect hay aLogin. Lưu ý: **0x1A loại khỏi diện nghi vấn** — đã chứng minh là MoneySync S→C-only (C→S rỗng, payload số, không chuỗi); handler C→S 0x1A hiện có của Rust (`handle_pc_talk`, selector số cho talk) không liên quan chat và giữ nguyên.
3. Quyết định: (a) client = Bear-dialect → tiếp tục G1-G3 trên 0x02; (b) client = aLogin → bổ sung handler C→S cho opcode chat thực tế (0x37/0x1D/0x3A/0x3B, tái dùng cùng `chat_frame`/routing đã fix), giữ 0x02 cho Bear-dialect (dual-dialect, đã có tiền lệ ở `dispatcher.rs:289-301` cho trade/shop).
4. Output: bảng C→S thực tế (op/sub/body) + mẫu packet vàng cho mỗi kênh.

### G1 — Fix P0 (đúng/scope/bảo mật, mỗi mục 1 ticket)

1. **Whisper id**: `chat_frame(3, sender_id, chat_raw)` cho cả sender lẫn recipient; check recipient online qua `clients` map, offline → trả `020B`; ghi nhận có/không cập nhật `targetIDBt` (tạm không, chờ G0 nếu field đó thuộc Bear-struct).
2. **Gate sub 4**: yêu cầu `gm_level > 0`, player thường → drop + audit warn (không reveal). Test: player gửi sub 4 → không ai nhận; GM gửi → all nhận.
3. **Scope sub 6**: tạm thời no-op + echo sender (parity Bear `replyToArmy` rỗng) cho tới khi có guild/army membership; ghi log `unimplemented`. Tuyệt đối không `broadcast_except` toàn server nữa.
4. **(Sau G0)** nếu dual-dialect: thêm handler C→S cho opcode chat thực tế của aLogin (0x37/0x1D/0x3a/0x3b theo kết luận G0 — **không phải 0x1A**), tái dùng cùng `chat_frame`/routing đã fix.

### G2 — Parity hành vi + slash (mỗi mục 1 ticket)

5. **Echo semantics**: theo capture G0 — nếu client tự hiện, bỏ `out.send` ở sub 1/2 (giữ map/world fan-out), giữ echo ở whisper (Bear gửi cả 2 đầu) và party (Bear gửi cả team gồm sender).
6. **Bỏ/sửa promotion 2→1**: mặc định tắt (sub 2 luôn map-only như Bear); nếu giữ item global-chat thì phát bằng kênh đã verify gate (ưu tiên `020C`-style announce hoặc sub 0) + test trên client thật.
7. **Thống nhất giới hạn độ dài**: 1 hằng số (đề xuất 120 chars cho chat thường theo `/broadcast` hiện tại `gm.rs:379`, whisper giữ 60 theo text hiện tại hoặc nâng đồng bộ), đếm theo `chars()` sau `viscii_decode`, drop có log + (tùy chọn) `020B` báo lỗi cho sender.
8. **Slash mọi sub**: chuyển intercept `/` lên trước `match sub` (parity Bear `specialMsg`), giữ nguyên tắc Bear: unknown `/cmd` của player → nuốt im; unknown của GM → (chọn 1) nuốt im thay vì broadcast như Bear để tránh rò lệnh.
9. **Bổ sung slash thiếu**: `/warp` (validate + party-leader + anti-battle như Bear), `/exchange`, `/offq` (alias `/endtalk`), stub báo `020B` "chưa hỗ trợ" cho nhóm bot (`/bot /autoboom /autosell...`); bổ sung `/help` liệt kê lệnh player (Bear có, Rust chưa).
10. **Party chat qua registry**: resolve members từ party live thay vì snapshot sender; sender không trong party → `020B` báo lỗi thay vì echo câm.
11. **Whisper offline notice** + **sub 5 khi solo** → `020B` (gộp vào ticket 1 và 10).

### G3 — Hardening + test (1 ticket test + 1 ticket hardening)

12. `tests/chat_test.rs` (theo `AGENTS.md`): sub 1/2/4/5/6 routing + echo, whisper id đúng + offline notice, slash mọi sub, gate sub 4, drop im unknown cmd, `/where /sleep /endtalk` frames; DB dùng `sqlite::memory` hoặc tempfile, không chạm `DB/ts_dream.db`; chạy `cargo test --all-targets --no-fail-fast`.
13. Rate-limit (token-bucket/chat cooldown, tái dùng ý tưởng sub `0x08` InputBar flag của client), mute/ban table + audit log chat nhạy cảm (sub 4, `/broadcast`, `/kick`). Làm sau khi G1-G2 ổn định vì thay đổi hành vi người dùng.

### Thứ tự thực hiện đề xuất

`G0 (capture + send-site) → G1.1 whisper → G1.2 gate sub4 → G1.3 scope sub6 → G2.5 echo → G2.8 slash-mọi-sub → G2.6 promotion → G2.7 độ dài → G2.9/G2.10 slash+party → G3 test+hardening.`

---

## 7. Nguồn trích dẫn chính

- Rust: `src/server/dispatcher.rs:27,248-249` (route), `:216-218` (op/sub/payload), `src/server/handlers/chat.rs:1-152` (6 sub), `:154-333` (slash), `src/server/spawn.rs:228-255` (chat/sys/announce frames), `src/server/gm.rs:48-76,169-185,377-389` (GM cmds/broadcast), `src/web/server_control.rs:256-315` (broadcast_except/map/send_to), `src/protocol/mod.rs:33-34` (OP_CHAT), `src/encoding.rs:239-241,612-625` (VISCII decode/encode).
- Bear: `TS_Server_Bear/TS_Server/PacketProcessor.cs:84-86` (route), `PacketHandlers/ChatHandler.cs:47-130` (switch 1-7 + gmchat/syschat), `:132-215` (specialMsg + public cmds), `:217-245` (/help), `:519-531` (/gm//allmsg), `:1353-1371` (/sleep), `:2112-2126` (FillMessageData), `Client/TSCharacter.cs:1275-1293` (announce/sendGMMessage), `:4863-4920,5917-5919` (reply family, replyToArmy rỗng), `Server/TSMap.cs:326-346` (BroadCast self-flag).
- Client: `.scratch/client-pseudo-op-code/opcode_02.md:71-88` (bảng sub), `:99-140` (từng sub + gate), `:144-166` (nhãn VISCII), `:171-180` (C→S 0x02 rỗng), `:200-223` (source trail); `opcode_1a.md §0,§5-§6` (**đính chính 2026-09-19**: 0x1A = MoneySync S→C-only, C→S rỗng; khuôn gửi chat-text nằm ở 0x37/0x1D/0x3A/0x3B — báo cáo này trước đó gán nhầm 0x1A là kênh gửi chat).
