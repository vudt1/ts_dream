# Xử lý hội thoại cứng FTalk.H6

Status: resolved
Type: grilling
Blocked by: 01

## Question

`FTalk.H6` trong `FTalk.cs` (~3000 dòng) chứa các nhánh hội thoại NPC cứng (hardcoded, không đến từ dữ liệu INI). Nghiên cứu giao thức tóm tắt theo frame template thay vì transcribe từng nhánh. Quyết định: spec sẽ yêu cầu (a) transcribe toàn bộ ~3000 dòng thành bảng dữ liệu hội thoại để executor port nguyên vẹn, hay (b) transcribe theo template + ghi chú các ngoại lệ, hay (c) một hình thức trung gian. Kèm đánh giá khối lượng và rủi ro sai lệch từng phương án so với mục tiêu byte-level fidelity.

## Answer

Chốt qua grilling (6 câu hỏi). Phát hiện nền: H6 (dòng 268-3241, ~2975 dòng) KHÔNG phải hội thoại text — là **logic nhánh menu** (đổi item, pet summon, shop, hotel, quest). Text nói thật nằm ở `Data_Talks` INI (generic path, dòng 3053+, đã có trong Protocol research).

1. **Hình thức transcribe**: **bảng data-driven** — mỗi nhánh 1 dòng. Không chép 3000 dòng code.
2. **Phân loại**: bảng pattern chỉ cho nhánh chuẩn; các nhánh đặc thù tách thành mục **H6 exceptions** (pseudocode + packet đầy đủ + tham chiếu dòng C#). Không ép mọi thứ vào bảng.
3. **Cột bảng pattern**: `map_id`, `idtalking` (step), `select_menu`, `action` (add_item / remove_item / add_pet / send_packet / hotel / sleep), `item_in_id` + count (điều kiện HomdoGetSlotExits), `item_out_id` + count (kết quả), `packet_literal`, ghi chú. Đủ để executor tái hiện không cần C#.
4. **Nhánh dùng chung + generic**: tách riêng thành **H6 pre-dispatch rules** (warpfinish / popup / SelectMenu 40 / nhóm banker 16080-16023 / hotel 15002-15118 / 16015) — 1 rule phục vụ nhiều NPC. Phần generic (WARP + Data_Talks + BattleQuestWin, 3053+) chỉ tham chiếu, không transcribe lại.
5. **Nhánh random quest NPC đầu (385-508)**: là **daily quest generator** — mục exceptions đầy đủ: thứ tự 7 biến RNG, công thức item id (62001+n*100), công thức reward (value1-48), nhánh SelectMenu. **Nối ticket 09** để xác minh RNG call-count parity.
6. **Vị trí trong spec**: bảng + exceptions đặt trong **chương Protocol** như phần 'FTalk.H6 data table' (phụ lục). Nếu bảng dài, tách file phụ lục trong spec/ và tham chiếu — không nhúng Data_Talks kiểu file runtime.

Đánh giá khối lượng: bảng pattern ~200-400 dòng (45 map case, 228 nhánh idtalking, 176 literal packet), exceptions ~150-200 dòng (daily quest + pet reborn 55002/59102/59011 + map đặc thù). Rủi ro sai lệch chính nằm ở daily quest (RNG) và nhánh map đặc thù — đã giao ticket 09.
