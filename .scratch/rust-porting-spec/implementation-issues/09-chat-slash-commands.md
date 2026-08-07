# 09 — Chat (op 0x02) + slash commands

**What to build:** Hệ thống chat đầy đủ (toàn server/map/whisper/party) và slash-commands cho người chơi. **Không có vai trò admin** — mọi tài khoản đều là player như nhau; các slash-command quản trị của C# (`_My_Id < 300012`) được **tắt** và giữ code comment-out trong `src/server/handlers/chat.rs` để tham khảo sau này. Người chơi nói chuyện và nhập lệnh thấy phản ứng.

**Blocked by:** 07 — Login + spawn.

**Status:** completed

- [x] Sub 2 — global/map chat: message = `data[6..]` ASCII; text > 60 bị drop. Thứ tự chọn kênh: nếu `Trangbi` slot6 id == 23100 → broadcast toàn client (op 0x02 sub 0x01); ngược lại chỉ map (sub 0x02). **Map chat fan-out giới hạn theo map qua `ServerControl.broadcast_map` (C# `SendToAllClientMapid`, Server.cs:596); global vẫn `broadcast_except`. Self echo giữ nguyên (golden).**
- [x] Slash-command player — **mọi người chơi như nhau, không còn gate admin**: `/where`, `/endtalk`, `/sleep`, `/openhotel`, `/openstore`, `/openbank`; `/cmd` không nhận biết → bỏ qua im lặng (C# sẽ broadcast text như chat — Rust cố ý không tái hiện). Slash-command admin cũ (`/additem ID[,count]`, `/addpet ID`, `/addskpoint N`, `/warp mapid`, `/test N`, `/reloadtalks`, `/battle N`, `/packet …`, `/sendpacket HEX`, `/loadnpcs`, `/loaditems`, `/loadscenes`) **bị tắt** — code gốc giữ nguyên dưới dạng comment trong `handle_slash` (kèm `is_admin()` + `ADMIN_ID_THRESHOLD` + 3 unit test admin).
- [x] `/sleep` full parity (C# `Client.Sleep`, Client.cs:646-846): hồi HP/SP người chơi (frame stat + persist), hồi thú nuôi stt 1..4 (frame `F4440F00080204` + persist `upsert_pet`), gửi `F44403001F0100`; leader (`id_leader == id`) còn hồi cho từng thành viên online qua `send_to` + cập nhật snapshot `online_sessions()`.
- [x] `/openhotel` full parity (C# `OpenHotel`, Client.cs:10002): frame `1F06` cho từng slot chuồng stt 5..10 (slot rỗng id=0), kèm tên thú (VISCII), gộp một lần gửi + `F44402001F07`; `/openbank` dùng `bank_gold` thật (không hardcode 0); `/where` đúng format `MapID:… X:… Y:…` + VISCII.
- [x] Sub 3 — whisper: target bytes 6–9 LE u32, msg bytes 10..; **cả 2 (sender + recipient)** nhận op 0x02 sub 0x03 mang id **recipient** (`ServerControl.send_to`).
- [x] Sub 4 — no-op; Sub 5 — party chat: gửi leader + thành viên (op 0x02 sub 0x05, sender id embedded) qua `send_to`.
- [x] Chat wire forms đúng: `F444`+len(`6+chat`)+`0201/0202/0203/0205`+sender/recipient(`le32`)+chatraw; không re-encode (Ch2 §2.3.3).
- [x] Golden: chat scenario ghi lại (`golden/08-chat.golden`); `golden/11-warp.golden` (admin `/warp` cũ) đã gỡ khỏi suite khi tắt admin.

**Notes / deferred:**
- Fan-out map/global: **đã resolve** — map chat qua `ServerControl.broadcast_map` (scope từ `online_sessions()` map_id), global giữ `broadcast_except`; không còn broadcast nhầm sang map khác như C# `SendToAllClientMapid` trước đây.
- `/battle N`: không còn hoạt động (admin command bị tắt) — code deterministic RNG chỉ còn trong comment reference.
- `/sleep`: **đã resolve** — hồi thú nuôi + party propagation + persist (`update_player`/`upsert_pet`). Lưu ý race nhỏ khi leader hồi member đang giữa vòng dispatch (snapshot ghi lại có thể bị member ghi đè) — chấp nhận, tương đương mức race của C#.
