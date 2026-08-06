# 09 — Chat (op 0x02) + slash commands

**What to build:** Hệ thống chat đầy đủ (toàn server/map/whisper/party) và slash-commands cho admin (`_My_Id < 300012`) lẫn thường dân. Người chơi nói chuyện và nhập lệnh thấy phản ứng.

**Blocked by:** 07 — Login + spawn.

**Status:** completed

- [x] Sub 2 — global/map chat: message = `data[6..]` ASCII; text > 60 bị drop. Thứ tự chọn kênh: nếu `Trangbi` slot6 id == 23100 → broadcast toàn client (op 0x02 sub 0x01); ngược lại chỉ map (sub 0x02). Slash-command dispatch theo server/admin và thường (Ch2 §2.3.3). **Routing/global qua `ServerControl.broadcast_except`; self echo giữ nguyên (golden).**
- [x] Slash admin (`_My_Id < 300012` gate): `/additem ID[,count]`, `/addpet ID`, `/addskpoint N`, `/where`, `/warp mapid`, `/test N`, `/reloadtalks`, `/battle N`, `/packet …`, `/sendpacket HEX`, `/endtalk`, `/loadnpcs`, `/loaditems`, `/loadscenes`; tất cả player: `/sleep`, `/openhotel`, `/openstore`, `/openbank`.
- [x] Sub 3 — whisper: target bytes 6–9 LE u32, msg bytes 10..; **cả 2 (sender + recipient)** nhận op 0x02 sub 0x03 mang id **recipient** (`ServerControl.send_to`).
- [x] Sub 4 — no-op; Sub 5 — party chat: gửi leader + thành viên (op 0x02 sub 0x05, sender id embedded) qua `send_to`.
- [x] Chat wire forms đúng: `F444`+len(`6+chat`)+`0201/0202/0203/0205`+sender/recipient(`le32`)+chatraw; không re-encode (Ch2 §2.3.3).
- [x] Golden: chat scenario ghi lại.

**Notes / deferred:** fan-out map/global hiện dùng `broadcast_except` (mọi client trừ sender) vì `ServerControl.clients` chưa lưu map_id từng client — chưa giới hạn theo bản đồ như C# `SendToAllClientMapid`. `/battle N` trigger TEAMDEF qua `out.battle_trigger` với defender ids deterministic. `/sleep` chưa hồi máu thú nuôi (cần pet update).