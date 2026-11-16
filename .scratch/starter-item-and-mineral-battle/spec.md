# Spec: Đồ Mặc Định Tạo Nhân Vật (`31007`) & Cơ Chế Chiến Đấu Quái Khoáng Trang Bị Cuốc

Status: `ready-for-agent`

---

## Problem Statement

Server Rust `ts_dream` hiện tồn tại hai nhóm thiếu sót ảnh hưởng trực tiếp đến trải nghiệm khởi đầu của người chơi mới và tính đúng đắn của gameplay chiến đấu tại các khu vực hang động/mỏ khoáng:

1. **Thiếu vật phẩm mặc định `Vé đoái hoán tâm` (`31007`) khi tạo nhân vật mới**:
   - Khi người chơi khởi tạo nhân vật mới, server mới cấp sẵn `32012 × 4` trong `homdo` (ô `1`) và `19737 × 1` trong `trangbi` (ô `2`), chưa cấp `1x 31007` (*Vé đoái hoán tâm*) vào túi đồ mặc định.
   - Do thiếu vật phẩm khởi đầu này, người chơi mới vào thế giới không có sẵn đạo cụ `31007` trong hòm đồ như thiết kế mong muốn.

2. **Thiếu cơ chế Vũ khí Cuốc & Miễn nhiễm cho Quái Khoáng trong trận chiến, đồng thời người chơi và sủng vật hiện không thể mặc được Cuốc**:
   - Trong hệ thống chiến đấu hiện tại, mọi đòn đánh thường và kỹ năng gây hại đều có thể gây sát thương, hiệu ứng khống chế hoặc trừ SP lên các Quái Khoáng (`25001..=25252`, `22010` *Thủy Tinh Quái*, `22011` *KhoángChìQuái*) ngay cả khi nhân vật hoặc sủng vật không trang bị Cuốc.
   - Ngoài ra, dữ liệu mẫu vật phẩm đang ánh xạ trường phân loại chế tác (`kind = 39`) sang vị trí trang bị của vật phẩm thay vì dùng vị trí mặc thật (`equip_pos / fit_type = 3`), khiến cả Nhân vật lẫn Sủng vật đều bị từ chối khi cố trang bị bất kỳ cây Cuốc nào (`10001`, `10530..=10534`) vào ô vũ khí.
   - Hai NPC đặc biệt `22010` (*Thủy Tinh Quái*) và `22011` (*KhoángChìQuái*) mang cờ cho phép thu phục trong `Npc.dat`, dẫn tới nguy cơ người chơi bắt làm sủng vật thay vì bị chặn thu phục như các Quái Khoáng khác.

---

## Solution

Bổ sung đồng bộ hai cơ chế trên dựa trên các mô hình miền và bất biến đã chốt trong `GLOSSARY.md` và các ADR hiện hành (`ADR-0001`, `ADR-0002`, `ADR-0005`, `ADR-0006`):

1. **Bổ sung `1x 31007` (*Vé đoái hoán tâm*) vào Đồ mặc định tạo nhân vật**:
   - Cấp cố định `1x 31007` tại ô `2` của `homdo` ngay trong giao dịch nguyên tử khởi tạo nhân vật mới (`0x09 Sub 01`), song hành cùng `32012 × 4` (ô `1` `homdo`) và `19737 × 1` (ô `2` `trangbi`).
   - Giữ nguyên cột `characters.newbie = 0` khi tạo nhân vật và giữ toàn vẹn quyền sở hữu `characters.newbie` cho luồng gift code `TSVN123/TSVN456` và sách `46238` theo `ADR-0001`; không thực hiện kiểm tra hay cấp bù khi đăng nhập / vào game đối với nhân vật đã tồn tại.

2. **Mở khóa trang bị Cuốc (`equip_pos` vs `kind`) & Áp dụng Luật Quái Khoáng trong Battle Engine**:
   - **Tách vị trí trang bị (`equip_pos`) khỏi loại chế tác (`kind`)**: Tầng dữ liệu vật phẩm và túi đồ lưu riêng vị trí mặc (`equip_pos = fit_type`, hợp lệ `1..=6`) và giữ nguyên `kind` cho hệ thống Compound/Crafting, giúp cả Nhân vật (ô `3`) và Sủng vật (ô `pet.stt * 10 + 3`) mặc, tháo và hoán đổi được nhóm Vũ khí Cuốc (`kind = 39`, `equip_pos = 3`).
   - **Nhận diện Quái Khoáng & Cấm thu phục**: Định nghĩa Quái Khoáng là các NPC địch (`typ == 3` hoặc `7`) có `kind == 16` (`25001..=25252`) hoặc ID thuộc `{22010, 22011}`. Không áp dụng miễn nhiễm cho Sủng vật đồng hành của người chơi (`typ == 4`). Cấm thu phục (`15001`) mọi Quái Khoáng, bao gồm cả `22010` và `22011`.
   - **Kiểm tra Vũ khí Cuốc độc lập & Chặn đòn per-target/per-attacker**: Nạp vũ khí đang mặc của từng Nhân vật và từng Sủng vật vào `WarInfo` lúc dựng bàn cờ. Khi đấu sĩ không mặc một trong 6 Cuốc `[10001, 10530, 10531, 10532, 10533, 10534]` tấn công hoặc dùng kỹ năng gây hại vào ô Quái Khoáng, đòn đánh vào ô đó buộc trả về trạng thái `MISS` (`0` sát thương, không hiệu ứng/debuff/hút SP/phản đòn), trong khi vẫn tiêu tốn SP và lượt của người đánh.

---

## User Stories

### Phân hệ 1: Đồ mặc định tạo nhân vật (`31007`)
1. Là một Người chơi mới tạo nhân vật (`0x09 Sub 01`), tôi muốn nhận sẵn `1x` *Vé đoái hoán tâm* (`31007`) ở ô `2` của `homdo` bên cạnh `32012 × 4` ở ô `1` và `19737 × 1` ở `trangbi` ô `2`, để có đủ bộ vật phẩm khởi đầu ngay khi bước vào thế giới.
2. Là một Người chơi vừa tạo nhân vật và gửi lệnh vào game lần đầu, tôi muốn bản tin đồng bộ túi đồ (`17 05`) hiển thị ngay `1x 31007` ở ô `2` mà không bị mất hay trùng lặp qua các lần đăng nhập lại sau này.
3. Là một Hệ thống lưu trữ dữ liệu, tôi muốn việc chèn nhân vật mới, tiền tệ và các vật phẩm mặc định (bao gồm `31007`) diễn ra trong cùng một transaction SQLite nguyên tử, để nếu tạo nhân vật thất bại (như trùng tên) thì không có vật phẩm rác nào bị ghi xuống DB.
4. Là một Người chơi tạo nhân vật mới, tôi muốn giá trị `characters.newbie` vẫn được khởi tạo bằng `0` và không bị tự động đổi thành `1` khi tạo hay vào game, để tôi vẫn đủ điều kiện sử dụng tính năng gift code `TSVN123/TSVN456` theo `ADR-0001`.
5. Là một Người chơi sở hữu nhân vật đã tạo trước bản cập nhật này (với `characters.newbie = 0`), tôi muốn quá trình đăng nhập vào game giữ nguyên túi đồ hiện có của tôi và không tự động chèn thêm `31007` lúc Enter Game.

### Phân hệ 2: Trang bị Vũ khí Cuốc cho Nhân vật & Sủng vật
6. Là một Người chơi sở hữu bất kỳ cây Cuốc nào trong nhóm `[10001, 10530, 10531, 10532, 10533, 10534]` trong `homdo`, tôi muốn mặc nó vào ô vũ khí (ô `3` của `trangbi`) khi đủ cấp độ yêu cầu, để sẵn sàng chiến đấu với Quái Khoáng.
7. Là một Người chơi có Sủng vật đồng hành (`stt ∈ 1..=4`), tôi muốn trang bị bất kỳ cây Cuốc nào trong nhóm `[10001, 10530..=10534]` từ `homdo` vào ô vũ khí của Sủng vật (`pet.stt * 10 + 3` của `trangbi`) khi Sủng vật đủ cấp độ, để Sủng vật tự mình gây sát thương lên Quái Khoáng.
8. Là một Người chơi đang mặc hoặc cho Sủng vật mặc một cây Cuốc, tôi muốn tháo Cuốc hoặc hoán đổi giữa Cuốc và vũ khí thường, và khi đang ở trong trận đấu thì không thể thay đổi trang bị làm sai lệch trạng thái trận đang diễn ra.
9. Là một Người chơi sử dụng đạo cụ không thể mặc như *Cuốc đào* (`29335`, `equip_pos = 0`) hoặc vật phẩm chưa đủ cấp độ yêu cầu, tôi muốn yêu cầu trang bị bị từ chối an toàn mà không làm mất vật phẩm khỏi `homdo`.
10. Là một Người chơi tham gia chế tác hợp thành (Compound), tôi muốn việc tách `equip_pos` không làm thay đổi mã `kind` (`39`) của Cuốc hay các trang bị khác, để bảng công thức `Compound.Dat` và cơ chế fallback vẫn hoạt động đúng như trước.

### Phân hệ 3: Chiến đấu với Quái Khoáng & Miễn nhiễm khi thiếu Cuốc
11. Là một Đấu sĩ (Nhân vật hoặc Sủng vật) đang trang bị một trong 6 Cuốc `[10001, 10530, 10531, 10532, 10533, 10534]` khi đánh thường (`10000`) hoặc thi triển kỹ năng vật lý (`skill_type == 1`), phép thuật (`skill_type == 2`, `19`) hay debuff vào Quái Khoáng (`kind == 16` trong `25001..=25252`, `22010`, `22011`), tôi muốn đòn đánh và hiệu ứng của mình được tính toán sát thương và tỷ lệ trúng bình thường theo công thức chiến đấu chuẩn.
12. Là một Nhân vật **không** mặc Cuốc tấn công Quái Khoáng bằng đòn đánh thường hoặc kỹ năng gây sát thương/khống chế/debuff/hút SP, tôi muốn đòn đánh vào Quái Khoáng luôn rơi vào trạng thái `MISS` (`0` sát thương, không áp dụng trạng thái phụ hay trừ SP), ngay cả khi Sủng vật hoặc đồng đội đi cùng có mặc Cuốc.
13. Là một Sủng vật (`typ == 4`) **không** mặc Cuốc tại ô vũ khí của riêng mình (`pet.stt * 10 + 3`) tấn công Quái Khoáng, tôi muốn đòn đánh của Sủng vật đó luôn bị `MISS` (`0` sát thương, không hiệu ứng), ngay cả khi chủ tướng đang mặc Cuốc ở ô `3`.
14. Là một Đấu sĩ không mặc Cuốc nhưng mặc vật phẩm có `kind = 39` không thuộc danh sách 6 Cuốc chuẩn (như `10497` *Hoàng Cân Sừ Đầu*, `10499` *Đao hộ pháp*, `10500` *Đao Phục Thù*), khi đánh Quái Khoáng tôi vẫn bị coi là không có Cuốc và nhận `MISS` (`0` sát thương).
15. Là một Đấu sĩ không mặc Cuốc sử dụng kỹ năng AoE (như *Loạn kích*, *Hỏa tiễn*, *Ném đá*, *Hồng thủy*...) lên một đội hình địch hỗn hợp gồm cả quái thường và Quái Khoáng, tôi muốn quái thường vẫn nhận sát thương/hiệu ứng bình thường trong khi các ô Quái Khoáng trả về `MISS` (`0` sát thương) trong cùng khung tin `32 01`.
16. Là một Nhóm đấu sĩ (ví dụ Nhân vật có Cuốc và Sủng vật/đồng đội không có Cuốc) cùng kích hoạt **Hợp kích / Combo** vào một mục tiêu Quái Khoáng, tôi muốn lượt đánh của người có Cuốc vẫn gây sát thương (được nhân hệ số combo tại mục tiêu chính) còn lượt đánh của người không có Cuốc trả về `MISS` (`0` sát thương).
17. Là một Đấu sĩ không mặc Cuốc đánh vào Quái Khoáng, tôi muốn vẫn bị trừ SP kỹ năng và tính là đã hành động xong trong lượt đó (`attacked = true`), đồng thời không kích hoạt phản đòn khiên hay trạng thái phụ trên Quái Khoáng.
18. Là một Quái Khoáng ở hàng trước được đồng đội thường đứng hàng sau dùng kỹ năng *Hộ chủ* (`20006`) đỡ đòn, tôi muốn điều kiện miễn nhiễm được kiểm tra trên thực thể thực sự nhận sát thương (nếu đơn vị đỡ đòn không phải Quái Khoáng thì vẫn nhận sát thương bình thường).
19. Là một Người chơi gặp `22010` (*Thủy Tinh Quái*), `22011` (*KhoángChìQuái*) hoặc bất kỳ Quái Khoáng `25001..=25252` nào trong trận, tôi muốn thao tác *Thu phục Võ tướng* (`15001`) lên các Quái Khoáng này luôn bị từ chối thất bại, ngay cả khi máu của chúng đã xuống dưới 50% MaxHP và `Npc.dat` đánh dấu `not_pet == 0`.
20. Là một Người chơi mang Sủng vật đã sở hữu (ví dụ thông qua dữ liệu sẵn có hoặc GM) tham chiến ở phe mình (`typ == 4`), tôi muốn đối phương đánh vào Sủng vật của tôi theo quy tắc chiến đấu thông thường, vì miễn nhiễm khoáng chỉ áp dụng cho NPC địch (`typ == 3` hoặc `7`).

---

## Implementation Decisions

1. **Đồ mặc định tạo nhân vật (`31007`)**:
   - Bổ sung mục khởi tạo `31007 × 1` vào ô `2` của container `homdo` trong bộ khởi tạo phiên nhân vật mới của handler tạo nhân vật, ngay sau mục `32012 × 4` ở ô `1`.
   - Giao dịch DB tạo nhân vật ghi toàn bộ các mục trong `homdo` và `trangbi` như hiện hành trong một transaction nguyên tử; giữ nguyên `newbie = 0` khi chèn bản ghi `characters` và không chèn logic cấp quà ở luồng đăng nhập / vào game.
   - Đảm bảo `Session` mẫu trong chế độ in-memory (golden replay) đồng bộ cùng danh sách vật phẩm khởi tạo với đường dẫn DB.

2. **Tách `equip_pos` (`fit_type`) khỏi `kind` ở tầng Dữ liệu & Túi đồ**:
   - Bổ sung trường vị trí trang bị `equip_pos` (lấy từ `fit_type` của `ItemDef` khi chuyển đổi sang `Item` và khi hydrate `InventoryItem` từ `GameData`), đồng thời giữ nguyên `loai` / `kind` cho các phân hệ phụ thuộc vào loại vật phẩm (như chế tác Compound và quy đổi dữ liệu legacy).
   - Khi khởi tạo vật phẩm từ mẫu dữ liệu (`ItemDef` -> `Item` -> `InventoryItem`), nạp đồng thời `equip_pos` và `kind`. Với các vật phẩm được dựng thủ công chỉ có `loai` trong `1..=6` (như đồ mặc định `19737` hoặc fixture test chưa đi qua hydrate), suy luận fallback an toàn về `equip_pos = loai` khi `equip_pos == 0` để không phá vỡ tương thích dữ liệu in-memory/test cũ.
   - Các hàm kiểm tra điều kiện mặc đồ cho Nhân vật và Sủng vật kiểm tra `equip_pos` nằm trong `1..=6` (thay vì `kind` nằm trong `1..=6`) và dùng `equip_pos` làm số thứ tự ô trang bị (`slot = equip_pos` cho Nhân vật, `slot = pet_stt * 10 + equip_pos` cho Sủng vật).
   - Bổ sung kiểm tra chặn thao tác trang bị/tháo đồ của Sủng vật khi đang trong trận đấu (`battle_id > 0`) nhất quán với thao tác trang bị của Nhân vật.

3. **Biểu diễn Vũ khí đang mặc trên `WarInfo` khi dựng bàn cờ**:
   - Bổ sung trường `weapon_id: u16` (mặc định `0`) vào cấu trúc ô chiến trận `WarInfo` để lưu trực tiếp mã ID vũ khí đang trang bị của thực thể đó; không làm thay đổi kích thước hay bố cục gói tin snapshot `23` byte (`0B 05` / `0B FA`).
   - Khi nạp Nhân vật vào bàn cờ, tra cứu container `trangbi` của `Session` tại ô `3` để gán `weapon_id`.
   - Khi nạp Sủng vật vào bàn cờ (cả đội trưởng và thành viên tổ đội), truyền thông tin `trangbi` của `Session` chủ nhân để tra cứu ô `pet.stt * 10 + 3` và gán `weapon_id` riêng cho từng Sủng vật.

4. **Định danh Quái Khoáng & Vũ khí Cuốc**:
   - Định nghĩa hằng số tập ID Vũ khí Cuốc hợp lệ gồm đúng 6 ID: `[10001, 10530, 10531, 10532, 10533, 10534]`, cùng hàm thuần kiểm tra `is_pickaxe(weapon_id)`.
   - Định nghĩa hàm thuần `is_mineral_npc(npc)` trả về `true` khi `npc.kind == 16 || npc.id == 22010 || npc.id == 22011`.
   - Định nghĩa hàm thuần `is_mineral_target(data, target)` trả về `true` khi `matches!(target.typ, 3 | 7)` và `data.npcs.get(&target.id)` thỏa `is_mineral_npc`.

5. **Áp đặt Luật Miễn nhiễm & Chặn Thu phục trong Battle Engine**:
   - Khối kiểm tra điều kiện chặn: một đòn đánh từ `attacker` vào `target` bị chặn miễn nhiễm khoáng khi `is_mineral_target(data, target) && !is_pickaxe(attacker.weapon_id)`.
   - Tích hợp kiểm tra ở **mức từng mục tiêu** (per-target) tại tất cả các nhánh gây sát thương và hiệu ứng bất lợi lên địch:
     - Đánh vật lý (`apply_physical`): bỏ qua tính toán sát thương/combo/trạng thái/phản đòn/chia EXP hit; ép `damage = 0`, `hit = miss_status::MISS` (`0`), giữ nguyên cờ `adl` theo trạng thái phòng thủ/né của mục tiêu và phát mảnh tin `32 01` chuẩn (`troi_byte::HP`).
     - Hộ chủ (`apply_damage_to_rear`): kiểm tra miễn nhiễm khoáng trên thực thể `rear` thực sự nhận sát thương (thay vì ô bị chọn ban đầu); nếu `rear` là Quái Khoáng và `attacker` không cầm Cuốc thì `rear` nhận `MISS` (`0` sát thương).
     - Đánh phép thuật (`apply_magic`) và nhánh tấn công phép thuộc nhóm buff/hỗn hợp (`apply_buff19` phần `Holy Spirit`): hành xử tương tự đánh vật lý khi bị chặn khoáng.
     - Kỹ năng khống chế & debuff/hút SP (`apply_status3`, `apply_buff15`): bỏ qua áp hiệu ứng, bỏ qua trừ SP/hút HP và bỏ qua phản bùa; phát mảnh tin `32 01` chuẩn với `miss_status::MISS` (`0`), `troi_byte::MISS` (`0x00`), `damage = 0`.
   - Không ngăn chặn các kỹ năng hỗ trợ/hồi máu đồng minh hoặc tự buff bản thân, và vẫn giữ việc trừ SP đầu hành động cùng đánh dấu `attacked = true` khi kết thúc hành động của đấu sĩ.
   - Tại Gate 4 của quy trình thu phục (`apply_catch` cho kỹ năng `15001`), từ chối thu phục nếu mục tiêu là Quái Khoáng (`is_mineral_target(data, target)`), đảm bảo `22010` và `22011` bị cấm bắt tương tự `25001..=25252`.

---

## Testing Decisions

Mọi kiểm thử mới tuân thủ `CODING_STANDARDS.md` §3: đặt toàn bộ dưới thư mục `tests/` theo 3 domain suite hiện hữu (không đặt `#[cfg(test)]` trong `src/`), chỉ kiểm chứng hành vi bên ngoài (trạng thái DB/Session và khung tin wire trả về), không test chi tiết cài đặt nội bộ, kèm đầy đủ negative tests và kiểm tra chống ăn lan (collateral integrity).

### Các Seam Kiểm Thử Cao Nhất (Existing Seams)
1. **Seam Khởi tạo nhân vật & Túi mặc định (`server_network` suite — nối tiếp mẫu của `create_char_atomic`)**:
   - Đẩy packet tạo nhân vật `0x09 Sub 01` và gói vào game `0x03 Sub 01` qua `handle_character` / `handle_enter_game` với SQLite test DB (`setup_test_db`), kiểm tra trực tiếp:
     - Hàng `inventories` chứa cả `32012 × 4` (ô `1`) lẫn `31007 × 1` (ô `2`), và `trangbi` giữ nguyên `19737 × 1` (ô `2`).
     - Cột `characters.newbie` vẫn bằng `0`.
     - Khung `17 05` phát ra khi vào game chứa `31007 × 1` ở ô `2`.
     - Negative / Isolation: Nhân vật cũ có `newbie == 0` khi đăng nhập không bị tự động chèn thêm `31007`; tạo nhân vật thất bại do trùng tên không chèn bản ghi `31007`.

2. **Seam Trang bị Vật phẩm qua `GameData` thật (`economy_inventory` suite — nối tiếp mẫu của `pet_equip_unequip` và `inventory_template_hydrate`)**:
   - Nạp dữ liệu item (`10001`, `10530..=10534`, `29335`, `10497`) từ `ItemDef` / `GameData`, gửi các gói tin `0x17` mặc/tháo đồ cho cả Nhân vật và Sủng vật (`handle_inventory`), kiểm tra:
     - Player mặc thành công `10001` (cấp 0) và `10534` (khi đủ cấp 110) vào `trangbi` ô `3` và nhận đúng bonus chỉ số.
     - Pet mặc thành công `10001` / `10530..=10534` vào `trangbi` ô `pet.stt * 10 + 3`.
     - Negative & Collateral: Từ chối mặc `10530` khi chưa đủ cấp 90; từ chối mặc `29335` (*Cuốc đào*, `equip_pos = 0`); slot phụ cận trong `homdo` và `trangbi` không bị ảnh hưởng; `kind = 39` vẫn bảo toàn khi hydrate.

3. **Seam Bàn cờ & Lượt chiến đấu (`battle` suite — nối tiếp mẫu của `catch_verification` và `combo_rules`)**:
   - Dựng trận đấu bằng `Battle::add_player`, `Battle::load_leader_pets`, `Battle::load_member_pet`, `Battle::add_npc` (nạp `Session.trangbi` thật + bảng `Npc`) và thực thi `Battle::run_turn`, kiểm tra chuỗi `Out::Broadcast` (`32 01`, `33 01`, `Out::Catch`) và HP/SP/Status của từng ô:
     - *Positive*: Player mặc từng Cuốc `10001`/`10534` hoặc Pet mặc Cuốc đánh thường, dùng kỹ năng vật lý (`skill_type == 1`), phép thuật (`skill_type == 2`), debuff (`type 3`/`15`) lên Quái Khoáng (`kind == 16`, `22010`, `22011`) đều gây sát thương / hiệu ứng đúng công thức.
     - *Negative*: Player không mặc Cuốc (hoặc mặc vũ khí thường / `10497`, kể cả khi Pet đi cùng có mặc Cuốc), và Pet không mặc Cuốc (kể cả khi chủ tướng mặc Cuốc) đánh thường, phép, `Holy Spirit` (`19`), hoặc debuff lên `25002`, `22010`, `22011` đều trả `MISS` (`miss = 0`, `dmg = 0`), HP/SP/Status mục tiêu không đổi, người đánh vẫn mất SP và kết thúc lượt.
     - *Mixed AoE & Combo*: AoE trúng đồng thời quái thường + Quái Khoáng khi không có Cuốc → quái thường mất HP, Quái Khoáng nhận `MISS`; Combo giữa Player có Cuốc và Pet không có Cuốc vào Quái Khoáng → lượt Player gây sát thương (nhân `×1.3`), lượt Pet trả `MISS`.
     - *Rear Guard (`20006`)*: Quái Khoáng được đồng đội thường ở hàng sau đỡ đòn → đồng đội thường vẫn nhận sát thương; ngược lại nếu Quái Khoáng đứng hàng sau đỡ cho quái thường thì vẫn miễn nhiễm khi người đánh thiếu Cuốc.
     - *Capture & PvP Pet isolation*: Thu phục (`15001`) lên `22010`, `22011` và `25002` (khi `< 50%` HP) luôn thất bại (không phát `Out::Catch`); Sủng vật `typ == 4` mang ID `22010`/`22011` ở phe người chơi trong PvP vẫn nhận sát thương bình thường khi không có Cuốc.

---

## Out of Scope

- **Không** triển khai cơ chế tặng quà qua điều kiện `characters.newbie == 0 -> 1` lúc Enter Game (đã chốt cấp `31007` như đồ mặc định tạo nhân vật và giữ `newbie` cho `TSVN123/TSVN456` theo `ADR-0001`).
- **Không** xử lý chức năng sử dụng item `31007` (*Vé đoái hoán tâm*) trong `use_item` nếu chưa có đặc tả riêng cho nghiệp vụ hoán đổi Ba Đậu Yêu.
- **Không** bổ sung loại Cuốc mới hoặc gộp `10497` (*Hoàng Cân Sừ Đầu*), `29335` (*Cuốc đào*), `35013`, `35025` vào cơ chế Quái Khoáng/Cuốc.
- **Không** thay đổi wire format hiển thị thông báo chữ "imutable" riêng biệt trên `aLogin.exe` ngoài frame `MISS` chuẩn (`miss_status::MISS = 0`, `damage = 0`).

---

## Further Notes

- Trong quá trình kiểm thử thực tế với client `aLogin.exe`, nếu phát hiện client kỳ vọng một mã byte trạng thái khác với `miss_status::MISS` (`0`) để hiển thị hiệu ứng miễn nhiễm đặc thù cho quái khoáng, có thể mở rộng nhánh trả về trong `skilling_int` dựa trên kết quả capture thực tế mà không ảnh hưởng kiến trúc kiểm tra per-target.
- Tài liệu khảo sát nguyên bản và đối chiếu dữ liệu `Data/items.json`, `Data/Item.dat`, `Data/Npc.dat`: xem `.scratch/starter-item-and-mineral-battle/research-newbie-gift-and-mineral-battle.md`.
