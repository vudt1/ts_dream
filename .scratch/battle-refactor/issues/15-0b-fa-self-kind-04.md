# 15: Marker self trong 0B FA phải là 04 (G1 — đã chốt bằng ASM)

**Vấn đề:** `0B FA` ghi byte `kind` của record self thẳng vào `player+0x376` của client. Cả hai đường "rời trận" của client đều đòi `0x376 == 4`, nhưng Rust lại gửi `03` (leader) và `05` (member) ⇒ client không bao giờ cho bấm rời trận, và `0B 00` không bao giờ reset được state.

**Chuỗi bằng chứng (mức A, xem `../HANDOFF-client-reverse.md` §2):**

| # | Bằng chứng | Vị trí |
|---|---|---|
| 1 | `*(char*)(player + 0x376) = kind` — record self | `006438ac:262` |
| 2 | nút "rời trận" chỉ chạy khi `player[0x376] == 4`, rồi đặt `byte = 3` và gửi `0B 01` | `00590ba8:159-163` |
| 3 | `0B 00` chỉ reset `player+0x376 = 0` khi `0x376 == 4`; nhánh khác chỉ dọn UI | `00641fa4:62,69-76` |
| 4 | kind `4` là trạng thái đặc biệt client tự trang bị: `player+0x1307 = 1`, `DAT_0098c640+0x25 = 1`, `FUN_0079b620(gvar_007DA0B4, 3)`, cuối handler `state+0x1C = 1` | `006438ac:263-270, 396-402` |
| 5 | kind `2`/`5` chỉ kích hoạt `FUN_0072b390(self, 8)` khi `player+0x384 == 0x3B` | `006438ac:253-256` |

**What to build:** đổi byte kind của **record self** trong mọi frame `0B FA` từ `03`/`05` sang `04`:

| File | Hiện tại | Sửa |
|---|---|---|
| `src/battle/packets.rs:9-15` `battle_open_leader` | `F4441C000BFA{diahinh}` + `03` + `leader_packet` (record của chính leader, frame gửi **cho** leader) | `03` → `04` |
| `src/battle/construction.rs:422-492` `member_battle_frame` | `text.push_str("05")` + self_packet | `"05"` → `"04"` |

**Không đụng** các marker của record không phải self (`03` cho leader + pet của leader, `64` cho member khác, và `0B 05 03`/`0B 05 05` cho entity NPC/player): client không dùng kind của record không-self vào gate nào đã biết (`FUN_006469b8`/`FUN_00659358` không đụng `+0x376`), và bộ giá trị này khớp loại entity (03 = NPC địch, 05 = player, 64 = đồng đội). Câu hỏi còn mở về giá trị `0B 04` (show-on-map) nằm ở ticket 14 mục 4.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] `battle_open_leader` và `member_battle_frame` dùng `04` cho record self; độ dài frame không đổi (`0x1C` / `4 + text`)
- [x] Test trong `tests/`: parse frame `0B FA` sinh ra, assert record đầu (self) có kind `0x04` và các record còn lại giữ `03`/`64`
- [x] Test: frame `0B FA` của leader **và** của member đều có kind self `04`
- [x] Comment tại call-site trích dẫn `006438ac:262` + `00590ba8:159` + `00641fa4:69` để không ai đổi lại