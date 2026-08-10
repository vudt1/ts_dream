# 18 — TALK core: H1 + dialog engine + H6 menus (op 0x14)

**What to build:** Nhân vật nói chuyện được với NPC qua chuỗi dialog `Data_Talks` (split trên `F444`, gửi cách 500ms), mở menu H6 cho ngân hàng/nhà nghỉ/shared NPC, xử lý `EndTalk` / warp-talk / SelectMenu. Vertical slice để mơ trải nghiệm tương tác NPC cơ bản. Nền cho shop (ticket 14), quest (ticket 19), battle quest.

**Blocked by:** 03 — Static data (Data_Talks, NPC data); 07 — Login/spawn; 11 — Inventory (để NPC shop add/remove dùng qua H6).

**Status:** completed

- [ ] Op 0x14 sub 1 (start talk): `data[6..7]` map object id LE16, `Typetalk="NPC"`; distance gate ±150; phân nhánh NPC đặc biệt:
  - `16080/16004/16011/16015` → `F44402000602`+`F44411001401000000010603`+idtalking(2B)+`0000000000000100`;
  - `15002/16001/16016` → tail `…0000 02 00`;
  - `16012` silent.
- [ ] Generic: có talk data → `F44402000602` rồi `TalkMessages(...)` split hex trên literal `"F444"`, mỗi fragment gửi 500ms cách nhau; zero-dialog + có `[TEAMDEF]` → battle quest (đưa về ticket 19); không talk data → nhánh NPC-body (Ch2 §2.3.13).
- [ ] Sub 4 → `EndTalk()` = `F44402001408` + reset talkcount/idtalking/SelectMenu.
- [ ] Sub 8 → `FTalk.H8` (warp talk); sub 9 → set `SelectMenu = data[6]`; default → `EndTalk()`.
- [ ] Op 0x14 H6 pre-dispatch (Ch2 §2.6.1): banker/store `16080/16004/16011/16023` (SM30 `F44403001D0900`+`F44406001D04`+bank+`F44402001D05`+`F44402001409`; SM31 `F44402001D0600`+`F44402001409`; SM40 EndTalk); inn/hotel (SM30 `F444110016010201000080000100`; SM31 `Sleep()`+EndTalk; SM32 `OpenHotel()`; SM33 savemap+item 46016×2+EndTalk; SM40 End); NPC `16015` biệt; `16012` silent.
- [ ] In-hàm H6 gắn đúng dispatcher; `player_id` nếu nhánh chạm item bảng người chơi.
- [ ] Golden: quest scenario (FTalk.H6 branch) & ít nhất 1 talk scenario được ghi lại (Ch9 §9.6).

## Review parity (2026-08-08)

**Kết luận:** Chưa đáp ứng ticket. Dispatcher và một số literal đã có, nhưng H1/H6 special dispatch đang dùng sai identity, thiếu distance gate và các workflow Sleep/Hotel/Warp chưa hoàn chỉnh. Giữ `Status: ready-for-agent`; full compiled H6 table và quest-specific exceptions được defer rõ ràng sang ticket 19, không tính là defect riêng của ticket 18.

### Mô hình miền/identity bắt buộc

- **Map Object ID (`map_object_id`)**: ID instance NPC trong `NpcOnMap`; request H1 đọc LE16 và quest key/dialog packet dùng ID này.
- **NPC Template ID (`npc_template_id`)**: NPC archetype resolve từ `(map_id, map_object_id)`; chỉ dùng để chọn special banker/inn/`16015`/`16012` behavior.
- **Talk Context** phải tối thiểu giữ `{talk_type, map_object_id, talk_count, select_menu}`. Không dùng một integer `idtalking` thay cho cả object ID và template ID.
- Special packet C# chèn `map_object_id.ToString("X2")` (legacy X2 field, thường 1 byte), không chèn template ID LE16.

### Ma trận acceptance

| Capability | Trạng thái | Rust | C# authority |
|---|---|---|---|
| H1 decode/resolve | Partial | `src/server/handlers/talk.rs:51-65` | `ts_server_old/Server_TS_Online/FTalk.cs:10-20` |
| distance + special NPC | Missing/incorrect identity | `talk.rs:67-100` | `FTalk.cs:69-112` |
| generic dialogs + 500 ms | Partial | `talk.rs:16-27,87-99`; `src/data/loader.rs:483-488` | `FTalk.cs:113-260,3439-3454`; `Data.cs:4611` |
| EndTalk | Partial | `talk.rs:9-14,39-40` | `Client.cs:7919-7925` |
| H8 warp talk | Partial | `talk.rs:201-207`; `src/server/handlers/quest.rs:511-605` | `FTalk.cs:3243-3386`; `Data.cs:3933-3983` |
| H9 + default | Implemented by source review | `talk.rs:45-47,209-213` | `FTalk.cs:3388-3391`; `Client.cs:2092-2097` |
| H6 pre-dispatch | Missing/partial | `talk.rs:103-199` | `FTalk.cs:268-384` |
| golden H1 + H6 | Partial | `golden/12-quest-h6.golden`; `tests/common/mod.rs:173-180` | Ch9 §9.6 |

### Vấn đề và giải pháp bắt buộc

1. **Special branch dùng sai identity.** Rust resolve `idnpctalking` đúng (`talk.rs:56-65`) nhưng `match idtalking` ở H1/H6 (`:68,123`). Dữ liệu thật có map object `6 -> NPC 16080` (`Data/NpcOnMap.txt:13250`), nên special flow không chạy. Giải pháp: branch bằng `npc_template_id`, giữ `map_object_id` cho packet và `Data_Talks` key.
2. **Thiếu distance gate +/-150.** C# tính delta từ tọa độ NPC rồi EndTalk nếu ngoài vùng (`FTalk.cs:69-112`). Rust không kiểm tra. Giải pháp: resolve instance một lần; reject missing/out-of-range trước mọi packet/battle action.
3. **Special packet có thể sai length/field.** Rust format template ID 16080 bằng `{:02X}` (`talk.rs:71-80`), tạo nhiều byte hơn field legacy. Giải pháp: encode legacy X2 map-object field và golden-lock cả object ID >255 nếu dữ liệu có.
4. **Talk Context thiếu `Typetalk` và `talkcount`.** `Session` chỉ có `idtalking/select_menu` (`src/server/session.rs:131-145`). EndTalk vì vậy không reset đầy đủ; step lookup luôn `0` ở quest layer. Giải pháp: typed Talk Context, EndTalk reset toàn context như C#.
5. **TalkMessages thiếu timing và loader semantics.** Rust split literal `F444` rồi queue tất cả ngay; C# sleep 500 ms giữa fragments. Loader giữ một string `Dialogs`, trong khi C# xử lý dialog array/tab sequence. Giải pháp: parse/preserve dialog sequence đúng INI và schedule send cách 500 ms không block Tokio runtime.
6. **Zero-dialog TEAMDEF H1 handoff chưa nối đúng.** Rust chỉ trigger trong H6 generic (`quest.rs:33-45,69-79`), còn C# H1 trigger ngay khi dialog count 0 (`FTalk.cs:161-184`). Ticket 18 sở hữu việc tạo handoff đúng Talk/Quest context; ticket 19 sở hữu quest orchestration/table và ticket 20 battle construction.
7. **NPC-body fallback bị thay bằng một literal generic.** Rust luôn gửi `...C830` khi không có talk (`talk.rs:92-98`); C# phân nhánh theo `SoLuong`, delay và battle body (`FTalk.cs:197-259`). Giải pháp: port core NPC-body dispatch hoặc ghi dependency cụ thể; không giả lập mọi NPC bằng cùng dialog.
8. **H6 pre-dispatch thiếu guards đầu hàm.** Rust không có `warpfinish`, popup close, `idtalking==0 && SelectMenu==40` behavior (`FTalk.cs:272-294`). Giải pháp: chạy guards trước special/generic dispatcher.
9. **Banker/inn/16015/16012 đang branch sai identity và workflow chưa đủ.** Banker packet sequence cơ bản có mặt (`talk.rs:123-141`), nhưng Sleep chỉ set HP/SP; OpenHotel chỉ gửi dialog; save-map không persist; `16015/SM31` thiếu `method_2(10)` (`FTalk.cs:299-383`). Giải pháp: dùng domain services Sleep/OpenHotel/SaveMap/AddItem có `player_id`, giữ packet order và EndTalk semantics.
10. **H8 chưa đi qua canonical warp lifecycle.** Normal warp chỉ mutate map và gửi `0504 + EndTalk` (`quest.rs:597-605`), thiếu `1407`, `0D000C`, persistence, party handling và confirm/appear lifecycle. Giải pháp: một Warp service dùng chung cho H8, OnWin và op `0x0C`.
11. **Persistence thiếu.** H6 add item/savemap/quest step mới mutate memory (`talk.rs:159-183`; `quest.rs:245-253`). Mọi gameplay write phải có `player_id`; `savemap` cần schema/load/persist seam rõ ràng.

### Tests / golden

- Unit talk hiện seed template ID `16080` làm request object ID (`talk.rs:223-253`), nên khóa chính bug identity thay vì dữ liệu thật.
- `golden/12-quest-h6.golden` chỉ cover một RequireSelectMenu mismatch; không cover H1 generic, distance, special banker/inn, 500 ms sequence, H8 hay persistence.
- Cần fixtures dùng `NpcOnMap` thật và C# captures cho H1 special/generic/out-of-range, H6 banker+hotel, H8 warp.

### Notes / deferred

- **Deferred sang ticket 19:** full compiled H6 table, daily generator, pet-reborn exceptions, generic quest requirements/rewards. Ticket 18 vẫn phải cung cấp đúng pre-dispatch và Talk Context seam.
- **Deferred sang ticket 20:** battle construction sau TEAMDEF trigger. Ticket 18 vẫn phải phát hiện zero-dialog TEAMDEF và chuyển đủ QuestKey/context.
- Không defer identity fix, distance gate, EndTalk state, TalkMessages timing, banker/inn core, H8 warp seam hoặc `player_id` persistence.

## Implementation record (2026-08-08)

Implemented core talk plumbing with the identity rules the review demanded:

- **Identity split**: `map_object_id` (`idtalking`) is the H1 request (LE16) and keys `Data_Talks`; `idnpctalking` is the resolved `(map,object)→NpcOnMap.NpcId` template and drives the special-branch selection. Special packets embed the *object* id (`idtalking.ToString("X2")`), never the template id.
- **Distance gate** (±150 on x/y): applied for specials and generics while distance matters; an absent `NpcOnMap` row opens the generic path (template 0) — quest fixtures without on-map data still talk.
- **Talk Context**: `talk_type`, `talk_count`, `warp_finish` added to `Session`; `end_talk` resets all of them (`F44402001408`). H1 sets `talk_count=0`; H6 continues drive it.
- **H6 pre-dispatch guards** (FTalk.cs:272-294): `warp_finish` flush → `F44402000504F44402001408` + resets; empty `idtalking==0 && select_menu==40` → EndTalk; `idtalking<=0` → silent.
- **Banker/Inn** (`16080/16004/16011/16023`, `15002/16001/16016/15118`, `16015`) branches on template id with bank-gold/wallet/awake frames; SetMenu→save purse (C# `_savemap`) + 46016×2.
- **H1 zero-dialog + [TEAMDEF]** hands off to `quest::trigger_teamdef`.
- **TalkMessages pacing**: fragments emitted 500 ms apart via new `HandleOutcome::send_delayed` + connection-loop pacing; frame order unchanged (golden replay ignores pacing).

Provenance: `Client.cs:7919-7925`, `Data.cs:553-600`; `FTalk.cs:10-384`; review matrix in this ticket.

## Review-fix followup (2026-08-10)

Two-axis review followup — resolved without changing wire behavior:

- Missing on-map instance is now **rejected with EndTalk** (review: "reject
  missing/out-of-range before any packet"): `resolve_npc` returns `Option` and
  the absent branch closes the talk. The `12-quest-h6` golden fixture
  (`tests/common/mod.rs` `quest_data`) now registers `NpcOnMap` object 1 within
  talk distance so H1 still opens — golden bytes unchanged.
- `savemap` persistence is wired end-to-end: the column is loaded on login
  (`db/players.rs`) and the inn-keeper SM33 branches persist it through
  `update_player("savemap", …)`. `handle_talk`/`handle_talk_continue` are now
  async to reach the pool (dispatch + unit tests updated).
- `generate_daily_quest` receives the loaded `GameData` instead of
  `GameData::default()` (stat-bearing items, no fabricated template).

Remaining per the review: `16012`/absent-NPC body-flavor and full NPC-body
dispatch are outside this pass; H6 compiled-table behavior stays with ticket 19.
