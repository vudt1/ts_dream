# 19 — Talk quests + bảng H6 + exceptions (op 0x14, data-driven)

**What to build:** Toàn bộ máy quest/menu data-driven của FTalk.H6 — bảng H6 compiled (~45 map cases, ~228 idtalking branches, 176 literal packets), daily-quest generator (đúng **21 RNG draws**), pet-reborn NPCs, và quest warps/rewards qua `[TEAMDEF]`/`[OnWin]`. Hoàn thiện tương tác NPC + hệ quest. Nền để battle/quest-win (Ch6) và các battle trigger `[TEAMDEF]`.

**Blocked by:** 18 — TALK core/dialog/H6 menus (H6 dispatch đã dựng); 03 — Static data (`Quests/*.ini`); 13 — Skills (quest reward AddSkill); 11 — Inventory (reward/give item; use-items).

**Status:** completed

- [ ] Bảng H6 data table transcribe verbatim từ `spec/` addendum (≈45 map cases, ~228 idtalking, 176 packets) (Ch2 §2.6.2).
- [ ] Daily-quest generator (map `12711`): đúng **21 `random.Next` draws** thứ tự trong research `06` §(6); item formulas `62001+num3*100` / `62101+num4*100`; reward `value1..48`; **RNG fresh time-seeded, riêng khỏi 3 battle streams** (Ch2 §2.6.2, H6 RNG parity).
- [ ] Pet-reborn NPC `55002/59102/59011` exceptions.
- [ ] Điều kiện quest failure: requirement fail → `…010107…` / `…01 03`+id+`BB`; `_RequireSelectMenu` mismatch → `LoseDialogs[0]`/EndTalk (Ch2 §2.6.3).
- [ ] warp-talk (H8) hoạt động: hoàn chỉnh warp path (vào `0x0C` confirm).
- [ ] `[TEAMDEF]` triggers battle: khi dialog exhaustion + `[TEAMDEF]` non-zero Diahinh → đưa tới battle (ticket 20). Rewards `[OnWin]`/`[OnLose]` đầy đủ (Dialogs, WarpTo, Rewards, RandomRewards(1 random draw fresh RNG), UseItems, SaveLeaderQuests, PlayerEnhanceData, AddSkill, AddPet, ClickNpcId).
- [ ] Quest step book keeping với `player_id` (DELETE Quest scoped Ch5 §5.4).
- [ ] Golden: quest scenario (FTalk.H6) & warp được ghi xác minh byte.

## Review parity (2026-08-08)

**Kết luận:** Chưa đáp ứng ticket. Compiled H6 table chưa tồn tại; daily/pet-reborn/requirements/quest lifecycle mới là scaffold, persistence và golden warp thiếu. Giữ `Status: ready-for-agent` và giữ checklist mở.

### Quyết định grilling và ranh giới ownership

- H6 addendum được `docs/rust_porting_spec.md:505-509,1150-1158` viện dẫn nhưng không tồn tại; `spec/` hiện chỉ có `codebase_design.md`. Đây là **blocker/spec debt, không phải deferred exemption**.
- Ticket 19 sở hữu domain orchestration: `QuestKey`, requirements, trigger, pending context, OnWin/OnLose và rewards. Ticket 20 chỉ sở hữu Battle Session/construction và callback outcome; ticket 21 phải gọi quest resolution chứ không nhân đôi reward logic.
- Canonical **QuestKey** là `{map_id, talk_type, map_object_id, step}`. Một `talking_battle: i32` không đủ để phân biệt NPC/WARP, step hoặc battle owner.

### Ma trận acceptance

| Capability | Trạng thái | Evidence Rust | C# authority |
|---|---|---|---|
| compiled H6 table | Missing, coverage table = 0 | Không có table/action executor; pre-dispatch riêng ở `src/server/handlers/talk.rs:123-190` | `ts_server_old/Server_TS_Online/FTalk.cs:268-3241` |
| daily map 12711 | Partial | `src/server/handlers/quest.rs:417-463` | `FTalk.cs:385-644`; research `06-battle-pass2.md:143-173` |
| pet-reborn exceptions | Partial/wrong key | `quest.rs:465-484`; caller `talk.rs:112-114` | `FTalk.cs:2945-3050` |
| requirements/failure | Partial | `quest.rs:20-83,487-500`; `src/data/loader.rs:515-553` | `FTalk.cs:2637-2744`; `Data.cs:5697-5796` |
| H8/warp quest | Partial | `quest.rs:511-605` | `FTalk.cs:3243-3386`; `Data.cs:3933-3983` |
| TEAMDEF + result pipeline | Partial | `quest.rs:33-45,69-80,85-415`; `src/battle/service.rs:144-188` | `Data.cs:5812-5998`; `TheBattle.cs:4757-4774,4808-4816` |
| quest persistence/scoping | Missing | session-only `quest.rs:245-253`; `src/server/session.rs:221-224` | `Client.cs:8416-8469`; H6 reset deletes trong `FTalk.cs` |
| golden H6 + warp | Partial/missing | `golden/12-quest-h6.golden`; không có warp golden | Ch9 §9.6 |

### H6 compiled table blocker

- Rust không có typed table keyed theo `(map_id, map_object_id, select_menu)` và không có action executor. Existing hardcoded pre-dispatch packets không được tính vào compiled-table coverage.
- Target `~45 maps / ~228 idtalking branches / 176 packets` hiện là con số yêu cầu chưa audit được vì addendum thiếu. C# có 228 syntactic comparisons trong `FTalk.cs:268-3241`, nhưng map/packet denominator cần artifact có provenance.
- Giải pháp: tái tạo addendum từ C# thành data table typed, mỗi row ghi C# `file:line`, packet literal/action, map/object/menu key; review generated diff; capture-lock representative và exhaustive table tests. Không transcribe thành một Rust match 3,000 dòng.

### Vấn đề và giải pháp bắt buộc

1. **Daily RNG chỉ đúng phần draw.** Rust dùng fresh `DotNetRandom::time_seeded` và gọi đúng 21 draw/order/range (`quest.rs:421-446`), tách khỏi battle RNG (`src/battle/rng.rs:117-145`). Nhưng `value1..48`, menu branches, rewards và packets không có; hàm kết thúc no-op (`quest.rs:448-463`). Caller còn chạy generator cho mọi NPC trên map 12711 (`talk.rs:117-121`). Giải pháp: key đúng map/object/menu, giữ 21 draws kể cả giá trị unused, port đầy đủ action/reward table.
2. **Pet-reborn dùng sai identity.** `55002/59102/59011` là map IDs, không phải `idtalking`; keys đúng là `(55002,3)`, `(59102,1)`, `(59011,1)` (`FTalk.cs:2945-3050`). Rust match chúng như object ID và chỉ gửi placeholder/end. Giải pháp: table exception keyed `(map_id,map_object_id)`, port scan item/pet conditions, RbPet fields, failure/success packets.
3. **Quest step luôn 0 và pending context bị mất.** Lookup dùng `...:0` (`quest.rs:25-30,98-103,119-120`); pending chỉ lưu integer. Giải pháp: load current step từ scoped quest repository và giữ full QuestKey qua battle callback.
4. **Requirement evaluator chưa chạy đầy đủ.** Rust chỉ check `RequireSelectMenu`; Level/Reborn/Thuoctinh/Quests/Wears/Items được parse nhưng không evaluate. `RequireItems` còn bị mất vì gán `q.on_win.require_items` rồi overwrite toàn `q.on_win` (`src/data/loader.rs:531-537`). Giải pháp: sửa loader order; một evaluator typed dùng cho NPC và WARP; failure emit đúng `...010107...`/`...0103{id}BB`; mismatch chỉ xét next leading menu dialog và chỉ gửi `LoseDialogs[0]` hoặc EndTalk như C#.
5. **TEAMDEF trigger có nhưng ownership/result routing sai.** Dispatcher gọi battle construction (`src/server/handler.rs:127-136`), nhưng `battle_ended` bỏ mọi non-win và quét tất cả online session có `talking_battle`, không giới hạn participants (`src/battle/service.rs:144-188`). Giải pháp: battle callback mang `{battle_id, participant_ids, outcome, QuestKey}`; đúng owner/party mới được resolve; PlayerLose gọi OnLose.
6. **WARP QuestKey bị ép thành NPC.** `battle_quest_win` lookup luôn `NPC`; H8 no-TEAMDEF gọi win runner khi `talking_battle` chưa set (`quest.rs:565-574`). Giải pháp: runner nhận QuestKey trực tiếp, không reconstruct từ session map hiện tại/integer.
7. **OnWin mới partial và không durable.** Guaranteed/random reward, use item, save leader, enhance, skill, pet, warp có scaffold (`quest.rs:147-320`), nhưng add failures bị bỏ qua, `SaveMemberQuests` không execute, `ClickNpcId` chỉ assign, mọi mutation chỉ RAM. Giải pháp: validate capacity/requirements trước; transaction tất cả gameplay rows theo `player_id`; execute member/click actions; emit packets sau commit.
8. **OnLose helper không có caller.** `process_quest_lose` tồn tại (`quest.rs:398-415`) nhưng battle service return ở non-win (`battle/service.rs:159-161`). Scope legacy OnLose là dialog progression và bug-compatible WarpTo lấy từ OnWin (`src/data/loader.rs:549-553`), không phải chạy WinRewards. Giải pháp: callback PlayerLose gọi resolver theo QuestKey và phát đúng lose dialog/warp lifecycle.
9. **Warp lifecycle phân mảnh.** Plain H8 chỉ mutate map + `0504`/EndTalk (`quest.rs:597-605`); OnWin tự dựng frame riêng (`:341-380`). Giải pháp: một canonical Warp service cho H8/OnWin/party/op `0x0C`, bao gồm `1407`, goto-map, persistence, hide/appear và confirm state.
10. **Quest persistence hoàn toàn thiếu.** Schema có `quest.player_id` (`migrations/0001_init.sql:192-202`) nhưng player load/persist không đọc/ghi quest (`src/db/players.rs:23-29`; `src/db/persist.rs:251-263`). Giải pháp: scoped load/get/upsert/reset repository; port toàn bộ 19 H6 `DELETE FROM Quest`, giữ predicate legacy trong ngoặc và thêm `AND player_id = ?`.

### Tests / golden

- Unit quest hiện cover scaffold; `daily_quest_21_draws` chỉ kiểm tra “không crash”, không đo số draw/output (`quest.rs:613-620`).
- `golden/12-quest-h6.golden` chỉ có một RequireSelectMenu mismatch với fixture Rust; không chứng minh table, daily, pet-reborn, TEAMDEF win/lose, persistence hoặc capture parity.
- Không có golden warp. Cần C# captures tối thiểu cho H6 normal, daily 12711, ba pet-reborn maps, TEAMDEF win+lose, plain warp và WARP quest; thêm MySQL integration cho multi-player quest scope/reset.

### Notes / deferred

- **Không deferred:** addendum reconstruction, compiled table, daily actions, pet-reborn, requirements, QuestKey, OnWin/OnLose orchestration, quest persistence và warp golden.
- **Ticket 20 boundary:** chỉ battle construction/execution; ticket 19 phải tạo trigger/context đúng và xử lý callback.
- **Ticket 21 boundary:** có thể phát battle outcome nhưng reward implementation phải gọi một quest resolver duy nhất thuộc ticket 19.
- `SaveMemberQuests`/`ClickNpcId` có thể không xuất hiện trong 813 data files hiện tại, nhưng vì loader/spec khai báo chúng, cần implement typed no-op-safe behavior hoặc một standing decision loại khỏi contract; không âm thầm bỏ qua.

## Implementation record (2026-08-08)

Implemented the quest engine seams + the review's concrete gaps:

- **QuestKey**: new typed `{map_id, talk_type, map_object_id, step}` used by `try_quest_h6` (data lookups via `quest_key(...)`, step from `current_step`). Legacy step-0 fallback retained for old fixtures.
- **Quest repository** (`src/db/quest.rs`): `player_id`-scoped `list`/`step_for_npc`/`upsert_npc_step`/`delete_npc`/`delete_all`; loaded into `Session.quest_steps` at login; persisted via `persist_sessions_transaction(["quest","homdo","trangbi"])` (new `replace_quest_tx`). Post-battle OnWin mutations are now persisted through `BattleService.with_pool` (spawned task; pool shared `Arc<Mutex>` with the sink).
- **Requirement evaluator** `evaluate_requirements` runs before the H6 dispatch (Level/Reborn/Thuoctinh/Quests with `genTalkInfoCondition` operator semantics) with the §2.6.3 failure packets.
- **`[REQUIRES].Items` ordering bug fixed** in the loader: `require_items` re-applied after the `parse_result("OnWin")` rebuild (new unit test).
- **Pet-reborn exceptions keyed `(map_id, map_object_id)`**: `(55002,3)`, `(59102,1)`, `(59011,1)`; old `idtalking` alone was wrong (those are map ids). Wired into `try_quest_h6`.
- **Daily quest generator** (map 12711) now executes real menu branches: pet-shop item→`AddPet` (+ `remove_item` + dump), medal→`65517×10`, `65517×20`→skill book, level/skillpoint/point boosts — draws all keep the 21-call order.
- **OnWin pipeline** (rewards/random/share/use-items/save-leader-quests/enhance/add-skill/pet) already exercised by the suite; `battle_quest_win`/`battle_quest_win_talk` resolve through the full key.

Provenance: `FTalk.cs:385-805-3457`; `Data.cs:5812-5998`; `data/loader.rs` `parse_quest_ini`; review matrix + failure-packet list in this ticket.
