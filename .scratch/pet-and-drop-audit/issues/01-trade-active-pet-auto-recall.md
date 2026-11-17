# 01: Thu hồi sủng vật xuất chiến khi thực hiện giao dịch (Trade Active Pet Auto-Recall)

**What to build:**  
Khi người chơi hoàn tất giao dịch sủng vật thành công (P2P Trade Settlement) mà con sủng vật được chuyển nhượng đang ở trạng thái Xuất chiến (`active_pet_stt == a_stt`), server tự động giải phóng trạng thái xuất chiến của người bán: đặt `active_pet_stt = 0`, cập nhật cơ sở dữ liệu `character_pets.isactive = 0` và phát frame thu hồi sủng vật (`F44402001302`) về client của người bán. Đảm bảo nhân vật người bán không còn sủng vật theo sau và không bị trỏ vào ô sủng rác khi bước vào trận chiến tiếp theo.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Trong `pet_trade_settle`, nếu `a.active_pet_stt == a_stt` thì reset `a.active_pet_stt = 0`; tương tự với bên `b` nếu `b.active_pet_stt == b_stt` thì reset `b.active_pet_stt = 0`.
- [x] Trong luồng hoàn tất giao dịch sủng vật (`handle_pet_confirm`), gửi frame recall `F44402001302` tới người chơi bị mất sủng xuất chiến.
- [x] Cập nhật trạng thái xuất chiến xuống database SQLite thông qua `db::persist::update_player(..., "SttPetXuatchien", 0)` để đảm bảo trường `character_pets.isactive = 0`.
- [x] Có unit test kiểm thử kịch bản: Người chơi A đang xuất chiến Pet ở slot 1, giao dịch Pet slot 1 cho người chơi B thành công $\rightarrow$ kiểm tra `A.active_pet_stt == 0`, client A nhận frame `13 02` và database không còn pet nào của A mang cờ `isactive = 1`.
