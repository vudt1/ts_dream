# 09: Tổng quát hoá finish() theo ô người chơi thực tế

**What to build:** `finish()` hardcode hàng 3 cho người chơi và hàng 2 cho pet của leader (giả định người chơi luôn ở row 3). Sau khi có quy tắc team `row < 2` và người chơi có thể nằm ở hàng 2, phải quét theo `team`/`typ` thực tế của từng ô: cộng exp/level/stat cho mọi ô player của phe ta, pet-exp đi theo chủ, và gửi cụm dọn ô đúng cặp ô. Đây là latent bug hôm nay nhưng phải sửa trước khi đấu nhiều hàng người chơi.

**Blocked by:**
- 06: Frame 05 0A LevelUP + 08 01 CD/CE khoá MaxHP/MaxSP

**Status:** completed

- [x] `finish()` không còn số hàng 3 / hàng 2 hardcode; duyệt theo `team`/`typ` của ô
- [x] Trận có người chơi ở hàng 2 vẫn nhận đủ exp/level/stat; pet của họ nhận pet-exp
- [x] Không cộng trùng phần thưởng; frame dọn ô gửi cho đúng cặp ô
- [x] Test trong `tests/` cho cấu hình 2 hàng người chơi
