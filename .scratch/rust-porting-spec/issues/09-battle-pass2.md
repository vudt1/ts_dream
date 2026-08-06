# Battle pass 2 — các gap còn lại

Status: resolved
Type: research
Blocked by: 04

## Answer

Đã resolve cả 6 gap + gap RNG-H6, ghi tại `research/06-battle-pass2.md`:

1. **Checksum**: KHÔNG có checksum — send path là `smethod_4`(hex→bytes) rồi `smethod_5` XOR từng byte 0xAD (`Class5.cs:132-166`, `Server.cs:524-554`). Research 04 §0 phán đoán "append checksum" là sai — chỉ là XOR 0xAD toàn frame.
2. **`GetTurn` case 14013**: fall-through vào ladder LvSKill giống GROUP_f — `1-3→2/4-6→3/7-9→4/10→5/else→3` (`TheBattle.cs:9260-9282`).
3. **Item hồi máu/mana**: `GetDataItem(id, "Hp"/"Sp")` trả `items._Hp/_Sp` (`Data.cs:4270-4275`); sau trận có restore HP/SP từ `_MY_HP_Store/_SP_Store` (`Client.cs:9646-9701`, include pet).
4. **`getHpMax/getSpMax`**: closed-form công thức luỹ thừa (pow 0.35/0.25) (`Data.cs:5537-5567`); exp curve là data-loop theo Level ups trong `Texps[]` (`TexpGetLvUp`, `Data.cs:4701-4747`).
5. **`BattleQuestWin`**: đầy đủ side-effect theo thứ tự (`Data.cs:5812-5998`) — consume item, red message, win/random reward, dùng item (player+pet packet), save quests, enhance, add skill, add pet, warp/end.
6. **H6 RNG parity**: block dùng `new Random()` riêng (time-seed, KHÔNG phải 3 stream battle) — đúng 21 lần `random.Next` theo thứ tự bảng trong `FTalk.cs:385-513`; formula item `62001/62002/... + num3*100` (+`62101.. num4*100`). Phải giữ nguyên 21 draw dù unused.

Không phát sinh fog mới graduate/ticket mới từ answer này. Research asset: [`06-battle-pass2.md`](../../research/06-battle-pass2.md).

## Question

Nghiên cứu pass 1 (Trích xuất Battle Engine) để lại 6 gap cần resolve để chapter Battle của spec đủ byte-faithful: (1) thuật toán checksum `smethod_5`, (2) fall-through của `GetTurn` case 14013, (3) giá trị item hồi máu/mana (`GetDataItem`), (4) bảng `getHpMax`/`getSpMax`/đường cong exp, (5) toàn bộ side-effects của `BattleQuestWin`, (6) parity số lần gọi RNG (repo không có test). Đầu ra: bổ sung vào research/04-battle-engine.md (hoặc asset riêng) với trả lời hoặc phương án xử lý cho từng gap.

**Bổ sung từ ticket 08 (FTalk.H6, resolved):** gap (6) RNG parity còn bao gồm **daily quest generator trong FTalk.H6** (dòng 385-508): thứ tự 7+ biến `random.Next`, công thức item id (62001+n*100), công thức reward (value1-48). Cần chốt thứ tự gọi RNG chính xác để H6 data table trong spec tái hiện byte-faithful — nhánh này được transcribe ở mục exceptions của ticket 08 nhưng phụ thuộc xác minh parity này.
