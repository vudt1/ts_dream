# 18 — TALK core: H1 + dialog engine + H6 menus (op 0x14)

**What to build:** Nhân vật nói chuyện được với NPC qua chuỗi dialog `Data_Talks` (split trên `F444`, gửi cách 500ms), mở menu H6 cho ngân hàng/nhà nghỉ/shared NPC, xử lý `EndTalk` / warp-talk / SelectMenu. Vertical slice để mơ trải nghiệm tương tác NPC cơ bản. Nền cho shop (ticket 14), quest (ticket 19), battle quest.

**Blocked by:** 03 — Static data (Data_Talks, NPC data); 07 — Login/spawn; 11 — Inventory (để NPC shop add/remove dùng qua H6).

**Status:** ready-for-agent

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