# Spec: Tự động bỏ qua lượt & Chặn chọn hành động khi dính Trạng thái Khóa Lượt (Ticket 25)

**Status:** ready-for-agent  
**Ticket liên kết:** [25-debuff-turn-skip-and-input-blocking.md](issues/25-debuff-turn-skip-and-input-blocking.md)  
**Tham chiếu kiến trúc:** [ADR 0006](../../docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md) · [GLOSSARY.md](../../GLOSSARY.md)

---

## Problem Statement

Trong trận đấu theo lượt của `ts_dream`, khi **Nhân vật** hoặc **Sủng vật** đang chịu hiệu ứng **Trạng thái Khóa Lượt** (*Khống chế cứng* như Băng phong, Hôn mê, Choáng, Trói cây, Sa bộc, Thanh Long hoặc *Hỗn loạn / Cuồng nộ*), người chơi không được phép chọn bất kỳ hành động chiến đấu nào (Đánh thường, Phòng thủ, Dùng vật phẩm, Thi triển kỹ năng, Thu phục, Bỏ chạy) và lượt của ô bị khóa phải được tự động bỏ qua.

Hiện tại hệ thống mới chặn ở pha thực thi cuối lượt, dẫn đến 5 vấn đề nghiêm trọng với trải nghiệm người chơi và tính toàn vẹn dữ liệu:

1. **Treo trận 20 giây mỗi hiệp**: Vòng đời lượt đấu vẫn ép pha thu thập lệnh phải chờ đủ mọi ô còn sống phe người chơi. Khi client chuẩn tự động bỏ qua ô bị khống chế và không gửi lệnh lên, cả trận đấu bị đóng băng chờ hết 20 giây đếm ngược mỗi lượt.
2. **Mất oan vật phẩm trong túi đồ**: Nếu client (do lệch trạng thái hoặc gửi nhầm/bot) vẫn gửi lệnh dùng vật phẩm chiến đấu khi đang bị khóa lượt, tầng tiếp nhận lập tức trừ 1 vật phẩm khỏi túi đồ rồi tầng thực thi lại bỏ qua hiệu ứng hồi phục — người chơi mất vật phẩm mà không nhận được tác dụng.
3. **Hiển thị sai trạng thái đang hành động**: Với các lệnh tấn công/kỹ năng/phòng thủ/thu phục/bỏ chạy gửi lúc bị khóa lượt, server vẫn phát khung báo đang thực hiện hành động xuống cả trận rồi sau đó lại không làm gì.
4. **Đấu sĩ bị đóng băng/ngủ vẫn đánh thường được nếu kèm Hỗn loạn**: Khi một ô chiến trận đồng thời chịu *Khống chế cứng* và *Hỗn loạn / Cuồng nộ*, engine kiểm tra Hỗn loạn trước nên đấu sĩ đang bị đóng băng hoặc hôn mê vẫn thoát ra đánh thường.
5. **Thiếu đồng bộ giao thức khóa giao diện xuống client**: Đường gán trạng thái trực tiếp thiếu khung khởi tạo hiệu ứng lớp phủ, và hiệu ứng phụ từ đòn đánh vật lý/phép thuật không gửi khung cập nhật trạng thái HUD hay khung hiệu ứng lên client, khiến client không biết ô đó đang bị khóa để tự ẩn/bỏ qua menu lệnh; ngoài ra trạng thái có định danh bằng `1` không bao giờ tự giảm lượt.

---

## Solution

Hoàn thiện cơ chế **Tự động bỏ qua lượt & Chặn chọn hành động** xuyên suốt từ tầng điều phối vòng đời lượt, tầng tiếp nhận **Kênh Lệnh Chiến Đấu**, tầng thực thi lượt đến tầng đồng bộ gói tin trạng thái xuống client:

1. Chuẩn hóa phân loại **Trạng thái Khóa Lượt** gồm *Khống chế cứng* và *Hỗn loạn / Cuồng nộ*; giữ nguyên quyền chọn đủ 6 hành động cho **Debuff mềm** (Bỏ độc, giảm Nhanh nhẹn, Thu nhỏ).
2. Tiếp tục phát **Khung Mở Hiệp Đấu** đầu hiệp cho mọi người chơi còn sống để client giữ đúng nhịp reset cờ lượt, đồng thời tự động loại mọi ô đang chịu **Trạng thái Khóa Lượt** khỏi danh sách chờ lệnh và danh sách tự động đánh thường khi hết giờ (thoát pha chờ ngay `0ms` nếu toàn bộ ô người chơi đều bị khóa).
3. Sử dụng hàm dự đoán nhất quán 1-1 với bước tick giảm lượt và giải hạn đầu hiệp để tránh lệch pha 1 lượt ở chính hiệp trạng thái vừa hết hạn.
4. Duy trì snapshot nhẹ tập ô bị khóa lượt trên handle trận đấu để tầng tiếp nhận từ chối đồng bộ mọi lệnh gửi cho ô bị khóa: không xếp hàng lệnh, không phát chỉ báo hành động giả, không trừ vật phẩm khỏi túi đồ, và trả về thông báo cảnh báo qua `sys_msg_frame`.
5. Ở pha thực thi, ưu tiên *Khống chế cứng* trước *Hỗn loạn / Cuồng nộ*, sửa điều kiện giảm lượt để không kẹt trạng thái có định danh `1`, và phát đầy đủ khung trạng thái xuống client cho cả đường cast trực tiếp lẫn hiệu ứng phụ trên đòn đánh.

---

## User Stories

1. As a player whose main character is under *Khống chế cứng* (e.g., Băng phong, Hôn mê, Choáng, Trói cây, Sa bộc, Thanh Long), I want my character's command turn to be skipped automatically without waiting for the 20-second countdown, so that the battle progresses smoothly.
2. As a player whose *Sủng vật* is under *Khống chế cứng* while my main character can still act, I want to be prompted only for my main character's command and see the turn resolve immediately after I submit it, so that the battle does not stall waiting on a disabled pet.
3. As a player whose main character is under *Khống chế cứng* while my *Sủng vật* is free to act, I want to still receive the turn prompt and submit commands for my *Sủng vật* without waiting 20 seconds for my disabled character, so that my *Sủng vật* can keep fighting or heal me.
4. As a player in a battle where every player-controlled *Ô chiến trận* (both *Nhân vật* and *Sủng vật* of all participants) is under a *Trạng thái Khóa Lượt*, I want the turn collection phase to complete immediately after the turn prompt without artificial delay, so that DOT ticks, AI actions, and animations proceed at full pace.
5. As a player whose *Nhân vật* or *Sủng vật* is under *Hỗn loạn / Cuồng nộ*, I want the command phase to skip waiting for manual input and let the unit perform its random basic attack automatically, so that I cannot override berserk with Defend, UseItem, CastSkill, Capture, or Flee.
6. As a player whose *Nhân vật* or *Sủng vật* is afflicted with both *Khống chế cứng* and *Hỗn loạn / Cuồng nộ* at the same time, I want *Khống chế cứng* to take priority and cancel the turn completely, so that a frozen or sleeping unit never breaks out to attack via berserk.
7. As a player whose *Nhân vật* or *Sủng vật* is only afflicted with a *Debuff mềm* (such as Bỏ độc, reduced Nhanh nhẹn, or Thu nhỏ), I want full access to all 6 battle actions (Đánh thường, Phòng thủ, Dùng vật phẩm, Thi triển kỹ năng, Thu phục, Bỏ chạy), so that soft debuffs do not falsely lock my controls.
8. As a player whose *Trạng thái Khóa Lượt* is about to expire at the start of the upcoming turn (remaining turns equals 1 before the turn-start tick), I want the server to recognize that my unit will be freed on that turn and wait for my command normally, so that I do not lose an extra turn due to off-by-one phase mismatch.
9. As a player whose *Nhân vật* or *Sủng vật* is under a *Trạng thái Khóa Lượt* whose client desyncs or forces a battle command packet (NormalAttack, Defend, CastSkill, Capture, or Flee), I want the server to reject the command cleanly without broadcasting a fake acting indicator, so that other players do not see misleading animations.
10. As a player whose *Nhân vật* or *Sủng vật* is under a *Trạng thái Khóa Lượt* whose client forces an in-battle item command, I want the server to reject the command before touching my inventory, so that my healing items are never consumed for nothing.
11. As a player whose battle command is rejected because the acting *Ô chiến trận* is locked, I want to receive a clear Vietnamese system notice via `sys_msg_frame` (`"Đang trong trạng thái khống chế, không thể hành động lượt này!"`), so that I immediately understand why my command did not go through.
12. As a player or party member in a battle where a unit is struck by a direct status skill or a physical/magical skill's secondary status effect, I want the server to emit the complete status frames so my client HUD updates and auto-skips locked units accurately, so that client UI and server turn state never drift out of sync.
13. As a battle participant or enemy NPC afflicted with a status whose identifier is `1`, I want its remaining turn counter to decrement each turn just like higher status IDs, so that units never get permanently soft-locked.
14. As a battle spectator or teammate, I want the server to keep sending the turn prompt frame at the start of every turn even when a unit is locked, so that client turn-flag resets and countdown visuals stay synchronized across all connected clients.

---

## Implementation Decisions

- **Phân loại miền & Từ vựng thống nhất**:
  - Áp dụng đúng định nghĩa trong Domain Glossary: chỉ **Khống chế cứng** (kênh Type 3 còn hiệu lực) và **Hỗn loạn / Cuồng nộ** (kênh Type 15 mang mã Cuồng nộ/Hỗn loạn `14021` hoặc `20014` còn hiệu lực) mới thuộc **Trạng thái Khóa Lượt**. Các **Debuff mềm** thuộc kênh Type 15 khác không khóa lượt.
- **Giữ nguyên thứ tự vòng đời lượt & Dự đoán trạng thái sau tick đầu hiệp (Battle Engine + Battle Manager)**:
  - Theo ADR 0006, giữ nguyên thứ tự *Bước 1 (phát Khung Mở Hiệp Đấu)* → *Bước 2 (thu thập lệnh)* → *Bước 3 (thực thi lượt, gồm Pha 1 tick giảm lượt/DOT/giải hạn đầu hiệp)* → *Bước 4 (Khung Báo Dứt Hoạt Ảnh & Dynamic Skill Delay Pacing)*.
  - Đóng gói một hàm kiểm tra thuần trên thực thể trận đấu để dự đoán chính xác liệu một *Ô chiến trận* có bị **Trạng thái Khóa Lượt** trong hiệp sắp thực thi hay không (có tính đến bước trừ 1 lượt và tự giải hạn khi số lượt còn lại `<= 1` trong Pha 1).
  - Trong vòng lặp quản lý lượt:
    - Bước 1 luôn gửi Khung Mở Hiệp Đấu cho mọi người chơi có Nhân vật hoặc Sủng vật còn sống.
    - Trước khi mở pha chờ lệnh, tính tập tọa độ các ô đang bị khóa lượt trong hiệp này và cập nhật vào snapshot dùng chung trên handle trận đấu; xóa/làm mới snapshot mỗi đầu hiệp (và dọn sạch khi kết thúc pha thu thập lệnh hoặc kết thúc trận).
    - Danh sách ô phải chờ lệnh và danh sách dự phòng tự động đánh thường khi hết giờ chỉ giữ lại các ô phe người chơi còn sống **và không** nằm trong tập khóa lượt. Nếu danh sách chờ rỗng, pha thu thập lệnh kết thúc ngay (`0ms`) để chuyển thẳng sang thực thi lượt và Dynamic Skill Delay Pacing.
- **Kiểm tra đồng bộ khi tiếp nhận Kênh Lệnh Chiến Đấu (Battle Handle + Battle Service + Handler)**:
  - Handle trận đấu giữ một snapshot tập tọa độ ô khóa lượt được bảo vệ bởi khóa đọc-ghi nhẹ (đọc O(1) đồng bộ không cần `await`), giữ nguyên chữ ký đồng bộ của handler tiếp nhận lệnh chiến đấu.
  - Khi nhận lệnh từ client qua **Kênh Lệnh Chiến Đấu**:
    - Nếu ô nguồn nằm trong danh sách ô điều khiển hợp lệ của người chơi nhưng đang nằm trong snapshot khóa lượt, dịch vụ trận đấu từ chối lệnh đồng bộ (`false`).
    - Handler nhánh lệnh hành động không phát chỉ báo hành động giả xuống trận; handler nhánh dùng vật phẩm không trừ vật phẩm khỏi túi đồ của phiên; cả hai nhánh đều gửi thông báo hệ thống VISCII 1.1 `"Đang trong trạng thái khống chế, không thể hành động lượt này!"` qua `sys_msg_frame` về cho người chơi.
    - Xác thực sở hữu và trạng thái khóa lượt vẫn tiếp tục được kiểm tra lại bên trong tác vụ thu thập lệnh để đảm bảo an toàn trước mọi race condition giữa lúc giao chuyển hiệp.
- **Ưu tiên trạng thái & Sửa lỗi đếm lượt trong Pha Thực Thi (Battle Runner)**:
  - Đảo thứ tự kiểm tra trong pha phân giải hành động: kiểm tra *Khống chế cứng* (Type 3 còn lượt) **trước** *Hỗn loạn / Cuồng nộ*. Nếu đang dính Type 3 thì đánh dấu đã xong lượt (bỏ qua toàn bộ 6 hành động người chơi, bỏ qua AI NPC và bỏ qua `auto_berserk`).
  - Sửa điều kiện giảm lượt đầu hiệp cho các kênh trạng thái từ `id > 1` thành `id > 0` để tránh kẹt vĩnh viễn mọi trạng thái mang mã `1`.
- **Đồng bộ Wire Trạng thái xuống Client (Battle Runner)**:
  - Ở nhánh gán trực tiếp Type 3 và Type 15 lên mục tiêu, bổ sung phát khung khởi tạo lớp phủ hiệu ứng (`35 01` start) tương xứng với nhánh phản đòn.
  - Khi đòn đánh vật lý hoặc phép thuật kích hoạt thành công hiệu ứng phụ Type 3 lên mục tiêu sạch, bổ sung đầy đủ chuỗi khung đồng bộ gồm chỉ báo hiệu ứng Type 3 trên khung chiêu thức (`32 01`), khung khởi tạo lớp phủ (`35 01`), và khung cập nhật khe biểu tượng HUD (`35 0F`) với số lượt và giá trị đề kháng tương ứng.

---

## Testing Decisions

- **Nguyên tắc viết test**: Kiểm thử hành vi quan sát được từ bên ngoài (kết quả lượt, thay đổi HP/SP/túi đồ, thời gian hoàn tất pha chờ lệnh, chuỗi gói tin phát ra), không ghép chặt vào biến nội bộ; tuân thủ quy chuẩn không đặt `#[cfg(test)]` trong mã nguồn chính và kiểm tra cả phản hồi lỗi `sys_msg_frame` cho mọi ca từ chối.
- **3 Seam kiểm thử hiện có được sử dụng (không tạo seam test-only mới)**:
  1. **Seam Dispatcher & Service (cao nhất cho luồng gói tin C→S)**: Gọi trực tiếp bộ điều phối / handler lệnh chiến đấu kèm ngữ cảnh kết nối, dữ liệu game và dịch vụ trận đấu thật (tương tự kiểm thử kênh lệnh chiến đấu hiện có). Dùng seam này để xác nhận gói dùng vật phẩm và các lệnh hành động gửi khi ô bị khóa lượt bị từ chối đồng bộ, túi đồ giữ nguyên số lượng (bao gồm cả vật phẩm ở slot lân cận), không phát frame hành động giả, và nhận đúng `sys_msg_frame` cảnh báo.
  2. **Seam Vòng đời Bất đồng bộ (`BattleManager` + `BattleSink`)**: Khởi chạy trận qua bộ quản lý với timeout đủ lớn (ví dụ vài giây) và sink thu thập frame (tương tự kiểm thử vòng đời lượt hiện có). Dùng seam này để xác nhận: vẫn phát Khung Mở Hiệp Đấu, tự động bỏ qua ô bị khóa lượt và kết thúc pha chờ ngay lập tức (nhanh hơn nhiều lần ngưỡng timeout), vẫn chờ Sủng vật khi chỉ Nhân vật bị khóa, và tự mở lại quyền chờ lệnh ở chính hiệp hiệu ứng hết hạn.
  3. **Seam Thực thi Động cơ Trận đấu (`Battle::run_turn`)**: Gọi trực tiếp chu kỳ chạy lượt với tập lệnh đầu vào (tương tự kiểm thử vòng đời buff/debuff hiện có). Dùng seam này để xác nhận: Nhân vật và Sủng vật dính Type 3 bị chặn đủ 6 loại hành động; *Khống chế cứng* thắng *Hỗn loạn / Cuồng nộ* khi dính đồng thời; *Debuff mềm* vẫn thực thi đủ hành động; trạng thái có mã `1` vẫn giảm lượt và tự giải hạn; đồng bộ đầy đủ wire `35 01`, `35 0F`, `32 01` cho cast trực tiếp và hiệu ứng phụ đòn đánh.
- **Vị trí lưu kiểm thử**: Thêm submodule kiểm thử mới dưới thư mục kiểm thử trận đấu chuẩn và khai báo trong suite trận đấu tổng hợp (không tạo file test lẻ ở gốc thư mục test).

---

## Out of Scope

- Không tách pha tick đầu hiệp (`turn_phase1`) ra chạy trước khi phát Khung Mở Hiệp Đấu (giữ nguyên hợp đồng của ADR 0006 và bộ test timing hiện hành).
- Không thay đổi giao thức hoặc mã nhị phân phía client Delphi (`aLogin.exe`).
- Không thay đổi logic chế độ Tự động đánh (`0B 07`) phía client ngoài việc đồng bộ đúng các khung trạng thái tiêu chuẩn mà client vốn đọc.
- Không bổ sung thêm phân loại debuff mới ngoài các kỹ năng/trạng thái đã tồn tại trong bảng dữ liệu hiện tại của server.

---

## Further Notes

- Trong thực tế, nếu cả Nhân vật chính và Sủng vật của người chơi cùng đang bị khóa lượt khi mở hiệp mới, client nhận `34 01` sẽ tự đặt cờ hoàn tất cho mọi slot; phía server thoát `collect_commands` sau `0ms` và dựa vào Dynamic Skill Delay Pacing (`clamp(800ms..3500ms)`) sau `32 02` để giữ nhịp hiển thị mượt giữa các hiệp.
- Khi triển khai hàm dự đoán khóa lượt cho hiệp kế tiếp, cần lưu ý trường hợp đặc biệt của hiệu ứng có `id > 0` vừa được sửa điều kiện giảm lượt (từ `> 1` sang `> 0`) để hàm dự đoán và pha tick đầu hiệp dùng chung một ngưỡng giải hạn duy nhất (`turn <= 1` sau khi đã kiểm tra `id > 0`).
