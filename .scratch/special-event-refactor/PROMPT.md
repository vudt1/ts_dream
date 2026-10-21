# PROMPT — Phân tích module "Special Events" (Sự kiện đặc biệt) của client TS Online (`aLogin.exe`)

> **File prompt này dành cho agent AI nghiên cứu.** Bỏ toàn bộ phần trong khung `>` vào đầu
> prompt khi giao cho agent. Phần còn lại là ngữ cảnh + checklist đầu vào cho agent.

---

## 0. Prompt (copy nguyên văn cho agent)

````text
Bạn là chuyên gia đảo ngược reverse-engineering game client. Nhiệm vụ của bạn là phân tích
TOÀN BỘ nghiệp vụ "SỰ KIỆN ĐẶC BIỆT" (special events) của client game TS Online — file
`client_delphi/aLogin.exe` (Delphi 7, PE x86, image base 0x00400000) — để từ đó viết được
server Rust tái tạo đúng nghiệp vụ.

────────────────────────────────────────────────────────────────────────
0. NGUỒN DỮ LIỆU (đọc file trong thư mục làm việc, KHÔNG cần mở Ghidra)
────────────────────────────────────────────────────────────────────────
Thư mục làm việc: `client_special_event_handler/`

  *.c                  — pseudo-C do Ghidra decompile (mỗi file = 1 hàm; header có
                          "Callers / Callees / References to entry")
  *.asm.txt            — assembly của cùng hàm đó (dùng để chốt byte-exact)
  *.raw.hex / *.hex    — dump byte thô của vùng dữ liệu / bảng tra / literal
  case_XXX_*.c         — 66 file, MỘT file cho MỘT Main OP (case_000 … case_065)
  strings_viscii_index.tsv  — toàn bộ chuỗi trong client, đã decode VISCII 1.1,
                          kèm Virtual Address (0x…); dùng để tra nhanh
  strings_event_catalog.tsv — tập chuỗi đã lọc theo nhóm sự kiện (đã có sẵn)
  FESTIVAL_DAT_FORMAT.md — đặc tả đã giải mã được cho `\Data\Festival.Dat`
  EVENT_MAPS_AND_NPCS.md — bản đồ sự kiện: map (Scene.Dat) + NPC (Npc.dat) đã giải mã
  DATA_FILES.md          — file .dat/.emg nào client đọc, loader nào, cái nào còn THIẾU
  data_map_catalog.tsv   — 7 480 map (index, tên, X, Y, kind)
  data_event_maps.tsv    — 187 map liên quan sự kiện
  data_npc_catalog.tsv   — 6 659 NPC (id, tên)
  data_event_npcs.tsv    — 737 NPC liên quan sự kiện (gồm nhóm "thủ binh" của từng sự kiện)
  data_event_items.tsv   — 179 vật phẩm sự kiện (id, tên, giá, mô tả) — nhiều mô tả tiếng Việt
  data_files/client_dat/ — bản gốc của 7 file .dat client đọc (Npc, Scene, Item, Warp, …)
  data_files/eve/         — ★ Data/eve.emg ĐÃ GIẢI MÃ (2 777 scene):
        eve_scene_index.tsv  chỉ mục scene (scene_id = map id thật) + số lượng từng section
        eve_npcs.tsv         NPC đặt trong map (npc_id, x, y, close, motion)
        eve_doors.tsv        cổng dịch chuyển trong map
        eve_logic.tsv        ★★ BẢNG ĐIỀU KIỆN → KẾT QUẢ (110 083 dòng, 60 430 dòng
                             có điều kiện QuestStep) — đây là logic quest/sự kiện thật sự
        eve_fights.tsv       trận PvE có kịch bản (2 248 dòng)
        eve_surface.tsv      hộp thoại / lựa chọn
        eve_groups.tsv       nhóm kết quả trọng số
        eve_encounters.tsv   vùng gặp quái ngẫu nhiên
        eve_full.jsonl       dump JSON đầy đủ, 1 scene/dòng
        (giải mã bằng `cargo run --example dump_eve` — script ở `examples/dump_eve.rs`)
  data_event_quests.tsv   ★ 250 dòng logic Eve đã lọc theo vật phẩm sự kiện
  data_event_items.tsv    ★ 179 vật phẩm sự kiện (id, tên, giá, MÔ TẢ tiếng Việt —
                           mô tả tiếng Việt là nguồn định danh sự kiện đáng tin cậy nhất)
  data_client_to_server.tsv ★ 115 dòng: Main OP → sub-op → địa chỉ builder, cho biết
                           sub-op nào client THẬT SỰ gửi (sub còn lại = epilogue, KHÔNG gửi)
  data_arena_modes.tsv    ★ bảng mode id lôi đài (0x60–0x63) → nhãn chat + hậu tố
  EVENT_ARENA_MODES.md    ★ diễn giải bảng trên (nguồn: FUN_0053BAB8)
  data_map_id_checks.tsv  ★★ 39 map id mà client so sánh (LocalPlayer+0x63A) → hành vi riêng + TÊN MAP
  EVENT_MAP_ID_CHECKS.md  ★★ map id ↔ hành vi client + thuật toán tra cứu tên map
                           (LocalPlayer+0x63A = map id hiện tại; class TKC_Scene quản lý
                            Scene.Dat; hàm tra tên map = FUN_0053558C)
  data_mapid_to_name.tsv  ★★★ 3 503 map id → tên map (giải mã Scene.Dat)
  SCENE_DAT_FORMAT.md     ★★★ đặc tả Scene.Dat sau giải mã: mapId+kind+tên map
  festival_dat_decrypted/  — 10 bản ghi đã giải mã (rec00.bin … rec09.bin)
  tools/festival_dat_decrypt.py — script giải mã (tham khảo thuật toán)
  MISSING_DUMPS.md      — danh sách source còn THIẾU, cần Ghidra dump bổ sung

BỐI CẢNH KỸ THUẬT (đã xác minh, không cần tự chứng minh lại):
  • Frame: [F4 44][Length:Word LE][Payload]
  • Toàn bộ frame XOR với khóa tĩnh 0xAD.
  • Server LUÔN phát tín hiệu trước; client không tự kích hoạt sự kiện nào.
  • Dispatcher S→C: FUN_0078a89c (file `0078a89c_FUN_0078a89c.c`)
      – tra bảng byte `0x78A8EE` (index) → bảng dword `0x78A9B6` (target) → 65 case.
      – Payload = `RP` (Rest Payload, đã cắt byte Main OP đầu tiên).
      – `RP[0]` = SubOp.
  • Builder C→S: FUN_0077F414 (file `0077f414_FUN_0077F414.c`)
      – `switch(param_2 & 0xff)` — xem để biết client gửi frame nào, khung payload ra sao.
  • Codec: FUN_0077eb9c (đọc Word LE), FUN_0077ef7c (đọc DWORD LE),
           FUN_0077eb1c / FUN_0077ee84 (ghi Word/DWORD LE), FUN_0077f098,
           FUN_0077f298.
  • Chuỗi trong client là VISCII 1.1 (không phải cp1258). Bảng decode đã có trong
    `strings_viscii_index.tsv` — HÃY DÙNG BẢNG ĐÓ, đừng tự decode lại.
  • Ngữ cảnh giao thức chung đã có sẵn ở `.scratch/client-pseudo-op-code/opcode_*.md`
    và `guild_city_siege_flow.md` (đọc để thống nhất cách ghi báo cáo, KHÔNG cần
    phân tích lại các opcode không liên quan sự kiện).

────────────────────────────────────────────────────────────────────────
1. RÀNH GIỚI — SỰ KIỆN NÀO CẦN MAPPER
────────────────────────────────────────────────────────────────────────
Đây là danh sách sự kiện đã xác nhận là tồn tại trong client (mỗi dòng kèm bằng chứng
chuỗi VISCII đã giải mã + Virtual Address). Bạn phải xác minh lại từ code và tìm ra
cơ chế Server→Client và Client→Server của từng sự kiện.

  A. LÔI ĐÀI / ĐẤU TRƯỜNG VÕ ĐẠI (arena, "lôi đài")
     - "đấu võ lôi đài", "lôi đài đoàn P" (= đấu nhóm), "lôi đài loạn đấu" (= solo PK),
       "lôi đài tiên đấu", "lôi đài thiêu chiến"  → 0x00729590…0x00729914,
       0x007106F0, 0x00710734
     - Prefix chat "[Lôi đài tranh đấu 1..4]" → 0x0053BD40, 0x0053BD60, 0x0053BD80, 0x0053BDA0
     - Bảng lịch thi đấu + giải thưởng: 0x00797210, 0x0079727C, 0x007972D8, 0x0079731C,
       0x00797368, 0x00797E94, 0x0072541C, 0x00796CAC, 0x007979E0, 0x00797A08
     - Manager: TMRBatterManage (VMT 0x0054C230) · gợi ý opcode 0x3F sub 0x01/0x02
     - Cần tìm: cấu trúc frame báo "lôi đài bắt đầu/kết thúc", cách client biết mình
       đã báo danh, tiền cược (5.000 / 10.000 lượng), hệ ngũ hành, bảng xếp hạng,
       cờ nhận thưởng (0x0075C438 "tạm thời không thể vào Lôi Đài Sinh Tồn").
     - ★ ĐÃ CÓ bảng 5 chế độ lôi đài (xem `EVENT_ARENA_MODES.md`): mode id 0x60/0x61/
       0x62/0x63 → nhãn chat "[Lôi đài tranh đấu 4/3/2/1]"; hậu tố theo flag:
       "Tỉ thí", "Đòan P" (≈Đoàn P), "Lọan đấu" (Solo PK), "Đấu trận cấp 100".
       Client lưu danh sách người chơi theo chế độ trong mảng record 7 byte tại
       self+0x164 (manager TMRBatterManage).

  B. THỦY CHIẾN / SHIP WAR (chiến thuyền)
     - "Hoạt động thủy chiến sắp bắt đầu / bắt đầu rồi / thủy chiến đã kết thúc"
       → 0x0054A910, 0x0054A96C, 0x0054A9CC
     - 3 kịch bản chiến trường (có luật chơi + điều kiện thắng):
       "Trận chiến Đại Hưng Sơn" (0x00557E64, 0x0055829C), "Trận chiến Cự Lộc"
       (0x00558558), "Trận chiến Đổng Trác thảo Phạt" (0x005587F8, 0x005590AC —
       nhiệm vụ "Trừ diệt Nghiêm Bạch Hổ", "Chiến thuyền Tôn Sách")
     - Manager: TMR_WaterBattleManage (0x0054A318), TMR_WBManageOrgManage, TWaterManage
       (0x00526448) · opcode 0x3F sub 0x0B/0x0C/0x0D/0x14/0x15/0x16 và opcode 0x40
     - Cần tìm: layout frame tàu thuyền (HP/SP/đạn/đội bè), cách ghép phe,
       bảng "binh đoàn", điều kiện thắng/không được phá hủy chiến thuyền chính.

  C. CÔNG THÀNH CHIẾN / CHIẾN CÔNG KÍCH THÀNH TRÌ (city siege / guild war)
     - "cuộc chiến công thành sẽ chính thức bắt đầu vào 8 giờ tối", "thời gian công
       thành đã bắt đầu", "thời gian công thành hôm nay đã kết thúc",
       "trận chiến công thành Đại Mạc đã bắt đầu / kết thúc" → 0x00636B30…0x00636DAC
     - "Rời khỏi nơi tham quan thành chiến" → 0x00590958
     - "Trong chiến tranh công kích thành trì không thể …" → 0x00798CB8, 0x00798D04,
       0x00798D50, 0x00798DB0, 0x00798E08 ; "Thành Môn Chiến, không cùng đoàn không thể
       tham chiến" → 0x00798530
     - Mô tả chiến trường có %s (tên quân đoàn) → 0x00636444, 0x0063653C, 0x00636604,
       0x006366CC, 0x00636C98 ("Quân đoàn thành công bả thủ"), 0x00636FD8
     - Manager/boss: TWarElephant (0x006D0C20), TElephantsCrush
     - Cần tìm: frame nào báo "công thành", điều kiện vào chiến trường, cách tính
       chiếm lĩnh, cờ trạng thái công thành trên LocalPlayer.

  D. ĐUA THUYỀN RỒNG (dragon boat race)
     - "Cuộc tranh tài đua thuyền …" 0x0051B370, "Cuộc thi đua thuyền rồng kỳ 2" 0x0051B890,
       "… đã kết thúc" 0x0051B8F8 / 0x0051B9C0, "Hoạt động tranh tài đua thuyền rồng
       Đoan Ngọ" 0x0051BA64, "Quyển tham gia cuộc thi" 0x0051C05C, "bảng xếp hạng người
       thắng giải" 0x0051C19C, "Bảng xếp hạng người thắng giải đua thuyền rồng" 0x0051D340
     - Manager: TLH_DragonBoatStanding (0x007DA1E8), Race Controller (0x007DA714)
     - Đã biết: opcode 0x46 có 8 sub-op (0x01 thông báo/toast, 0x02 mở cửa xuất phát,
       0x03 đồng bộ tọa độ thuyền, 0x04 phim .sty, 0x05 kết thúc, 0x06 bảng xếp hạng,
       0x07 đếm ngược 5 giây, 0x08 cờ tham gia của LocalPlayer+0x157A)
     - Cần tìm: chi tiết từng sub-op, layout record thuyền đua, cách server báo kết quả
       và trao thưởng, vì sao cần "Quyển tham gia cuộc thi".

  E. SỰ KIỆN THEO MÙA / LỄ HỘI (seasonal festival)
     - Tết Đoan Ngọ 0x00724424/0x00724494/0x00725470/0x007266DC/0x007486B0
     - Lễ Thất Tịch 0x007257AC/0x007258B0/0x00797BB8
     - Halloween + Quỷ Vương 0x007245C0/0x00724500
     - Niên thú Thôn Thực (đánh bom) 0x0072690C/0x007269E0
     - Tháp Lôi Phong 0x007254C0/0x0072555C
     - Tiêu diệt Tứ Hài 0x007489C4/0x00748AE4
     - Tiệc nguyên tiêu 0x007976C0 ; Hinet đại chiến 0x00797B64
     - Dữ liệu lore: `\Data\Festival.Dat` (xem FESTIVAL_DAT_FORMAT.md — đã giải mã xong)
     - Cần tìm: frame nào mở form "Lễ hội", cách ánh xạ ngày → bản ghi Festival.Dat,
       cơ chế đếm ngược giờ bắt đầu/kết thúc sự kiện.

  F. QUAN CHIẾN / PK trong sự kiện
     - "Quan chiến" 0x0058B4FC, "Rời khỏi quan chiến [Alt+R]" 0x00595CA8,
       "Đội viên không thể quan chiến" 0x0058B848/0x005FC9D4/0x005FEB3C,
       "Số người quan chiến đã đầy" 0x00644CF0, "Đối phương đang quan chiến, không thể PK"
       0x00643210/0x0064ECA0/0x0064F42C
     - Cần tìm: flag PK trong sự kiện (đặc biệt "loạn đấu" / "đoàn P"), giới hạn số người
       được PK, điều kiện được phép PK khi đang ở thành trì/công thành.

  G. CÂU HỎI / SỰ KIỆN GM (kênh 0x10) — "Bắc Tinh Quân" (quiz Day 55)
     - "Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến" 0x00798478
     - Manager: TWF_Day55QuestManage (0x00707488)

  H. BA TÊN SỰ KIỆN NGƯỜI DÙNG NÓI — ĐÃ TRA ĐƯỢC TÊN THẬT, GIỜ CẦN HOÀN THIỆN PHẦN WIRE
     • "2K"  → **"Giải cứu Nhị Kiều Giang Đông"** ✅
       Diễn ra tại **Đồng Tước Đài** trong **Diệp Thành** (map "Đồng Tước đài", Scene.Dat
       index 414/438/443/446/447/448/450/458/459/460/461, toạ độ (36–74, 197), kind 1/2/10).
       2 chuỗi thông báo: `0x00797A5C` (bắt đầu) và `0x00797B00` (kết thúc) —
       cả hai do `FUN_0078EC3F` phát ⇒ thuộc **Main OP `0x14`** (case_018).
       HÃY: tra frame nào mở/nộp quyền vào Đồng Tước Đài, điều kiện hoàn thành
       "giải cứu", cách trao phần thưởng.
     • "40 NPC" → **"Lôi Đài Khiêu Chiến"** ✅
       Bằng chứng trong `Item.dat` (đã decode, xem `data_event_items.tsv`):
         · item `31147` "Vé vip khiêu chiến" — *"Vé vip chuyên dùng trong **lôi đài khiêu
           chiến**"*  ⇒ chế độ này **cần vé VIP** để vào.
         · item `30014 … 30063` "Thắng lệnh 1 … 50" — *"Khiêu chiến N, dùng lệnh này lãnh
           thưởng"*  ⇒ sau mỗi vòng thắng người chơi nhận 1 "lệnh", tích lũy để đổi thưởng.
       HÃY: tra trong `data_files/eve/eve_logic.tsv` + `eve_npcs.tsv` các scene/map có
       `result_class = Item` với `result_value` = 30014..30063 và 31147, để dựng lại
       chuỗi hành động của chế độ này; tra tiếp frame C→S khi người chơi nộp vé / đổi lệnh.
     • "Hổ Lao Quan" → **CHIẾN TRƯỜNG THỦY CHIẾN** ✅ (không phải boss)
       Chi tiết ở `EVENT_MAPS_AND_NPCS.md` §2.2 & §3.1: map "Hổ Lao Quan" (Scene.Dat index
       1124/1139/1140/1152), cung "Tiền tiêu Hổ Lao" (index 727), NPC 4906
       "HổLao QThủBinh", nhiệm vụ "Trừ diệt Nghiêm Bạch Hổ" (`0x005590AC`).
       HÃY: điền nốt phần WIRE của chiến trường này (nó có vẻ chưa có chuỗi luật chơi
       riêng trong client — đối chiếu với "Đại Hưng Sơn" / "Cự Lộc" / "Đổng Trác").
     Ngoài ra 2 sự kiện liên quan đã xác định qua item:
       • "Loạn đấu" (Solo PK) — item `23160` *"Nhẫn bạo kích loạn đấu đài, 1 tuần"* và
         `46140` *"Hộp gỗ tặng thưởng loạn đấu"*.
       • "Công thành chiến" — item `30012` *"Công thành đồ sao phỏng"*, `23200`
         *"tháo dở khí giới công thành"*, `31121–31123` (xe công thành), `63001–63003`
         (tu bổ khí cụ công thành).

────────────────────────────────────────────────────────────────────────
2. CÂU HỎI BẮT BUỘC PHẢI TRẢ LỜI (mỗi câu phải có bằng chứng)
────────────────────────────────────────────────────────────────────────
Q1. Bảng tra cứu đầy đủ 65 Main OP của client (Main OP → hàm xử lý → danh sách SubOp),
    kèm cột "có liên quan tới sự kiện đặc biệt không" và ghi rõ Main OP nào là kênh
    chính để Server kích hoạt sự kiện.
Q1b. **Bảng map id ↔ tên map** — **ĐÃ GIẢI MÃ XONG** ✅ (xem `SCENE_DAT_FORMAT.md`).
    `*(int16*)(LocalPlayer + 0x63A)` = map id hiện tại; `*gvar_007DA3F0` = class
    `TKC_Scene` (VMT `0x00535380`) giữ mảng bản ghi `Scene.Dat` tại `self+0x18`;
    `FUN_0053558C` tra tên map, `FUN_00535864` binary-search theo cặp `(mapId, kind)`,
    `FUN_0053542C` giải mã `w = (w ^ 0xEA6C) - 9`.
    Kết quả: `data_mapid_to_name.tsv` (3 503 map id → tên), `data_map_catalog.tsv`
    (7 481 bản ghi), `data_event_maps.tsv` (2 455/2 777 scene `eve.emg` khớp map id).
    VIỆC CẦN LÀM (còn lại): dùng `data_event_maps.tsv` + `data_files/eve/eve_logic.tsv`
    để lập **bảng "sự kiện nào nằm ở map nào"** — đây là phần agent phải tự suy luận.
Q1c. **Bảng điều kiện QuestStep trong `eve_logic.tsv`** → quest id nào mở/nộp quyền
    sự kiện nào, và chuỗi bước (step) của nó. Đây là phần lớn logic sự kiện.
    Đã dò sẵn một mẫu để agent làm theo (xem `DATA_FILES.md` §3b):
        scene 12003 · eve_no 24 · điều kiện "quest 10714 bước ≥ 2" → trao item 30014
        ("Thắng lệnh 1" — *"Khiêu chiến 1, dùng lệnh này lãnh thưởng"*)
    ⇒ **Lôi Đài Khiêu Chiến = map 12003, quest 10714/10715.**
    HÃY dựng nốt chuỗi đầy đủ cho từng quest trong `data_event_quests.tsv`
    (30 quest id → map id) và nối tiếp với tên map (Q1b).
Q2. Với MỖI sự kiện A–H: danh sách frame Server→Client (byte layout từng field, kiểu
    dữ liệu, thứ tự, byte-exact nếu có) và frame Client→Server (nếu có).
Q2b. **Đọc từng builder trong `data_client_to_server.tsv`** để chốt LAYOUT BYTE + ĐỘ DÀI
    payload của frame C→S (builder = địa chỉ `builderAddr`; đọc `.asm.txt` tương ứng,
    lấy `MOV CL,imm` trước `CALL _PStrNCat` = độ dài payload).
Q3. Thứ tự timeline đầy đủ của một sự kiện (từ lúc Server báo "sẽ bắt đầu" → đến lúc
    trao thưởng): frame nào đến trước, cách hiển thị UI (toast/chat/banner/form),
    thời lượng, âm thanh, hiệu ứng.
Q4. Frame "kích hoạt" sự kiện: Server gửi gì để mở/đóng sự kiện, spawn NPC sự kiện
    (show/hide NPC), mở form UI, đặt cờ trên LocalPlayer.
Q5. Các field trạng thái sự kiện được ghi vào đâu trong cấu trúc LocalPlayer
    (player = **gvar_007DA7BC) — liệt kê offset + ý nghĩa + nơi đọc/ghi (hàm nào).
Q6. Các bảng dữ liệu client-side liên quan: Festival.Dat (đã giải mã xong, hãy đọc
    FESTIVAL_DAT_FORMAT.md và đối chiếu với code), Npc.dat, Item.dat, Scene.dat,
    SceneSet.dat, GS.dat, Warp.dat, RG.dat, TsoTw/TsoCn.dat, Rank.dat, Mark.dat.
    Với mỗi file: loader nào đọc, layout record, và mối liên quan tới sự kiện.
Q7. Phần client gửi lên (C→S) khi người chơi tương tác với sự kiện: báo danh lôi đài,
    vào thuyền, gia nhập phe, chọn mục tiêu, nhận thưởng, đăng ký tên đấu.
Q8. Điều kiện/ghi chú về giới hạn (đẳng cấp, tiền cược, số người, thời gian, map,
    trạng thái PK) mà client kiểm tra — client chặn ở tầng nào (UI hay logic).

────────────────────────────────────────────────────────────────────────
3. PHƯƠNG PHÁP (bắt buộc tuân thủ)
────────────────────────────────────────────────────────────────────────
• Mỗi file `.c` có header "Callers / Callees / References to entry" → dùng để đi theo
  call-graph, đừng đọc mù toàn bộ corpus.
• Khi cần biết một chuỗi nằm ở hàm nào: grep trong `strings_viscii_index.tsv` để lấy
  Virtual Address, rồi grep chuỗi địa chỉ đó (dạng `0x…` hoặc `DAT_00xxxxxx`) trong
  `*.asm.txt` / `*.c` để tìm hàm tham chiếu. (Đây là kỹ thuật đã dùng để lập bản đồ
  ban đầu cho prompt này.)
• Với mỗi kết luận phải gắn nhãn mức bằng chứng:
    (A) đọc trực tiếp byte/ASM/Dump — chắc chắn
    (B) suy luận có cơ sở từ code
    (C) giả định — phải nêu rõ và không được dùng làm nền cho kết luận khác
• TUYỆT ĐỐI KHÔNG kết luận "client không làm gì" chỉ vì thấy `case N: break;` rỗng.
  Phải tra bảng builder (`.hex` trong thư mục) xem target có trỏ tới epilogue `RET`
  hay không. Đây là sai lầm đã bị phát hiện 3 lần trong các tài liệu trước.
• Khi gặp chỗ chưa có body trong `.c`, hãy ghi rõ "cần Ghidra dump" và đưa vào danh sách
  thiếu (xem MISSING_DUMPS.md) thay vì đoán.
• Ghi chú chính xác mọi giả định về byte order, độ dài record, điều kiện biên (BoundErr).

────────────────────────────────────────────────────────────────────────
4. ĐỊNH DẠNG OUTPUT
────────────────────────────────────────────────────────────────────────
• Ngôn ngữ: TIẾNG VIỆT (thuật ngữ kỹ thuật giữ nguyên tiếng Anh).
• Định dạng: 1 file Markdown (khuyến nghị `SPECIAL_EVENTS_CLIENT.md`) — có thể kèm thêm
  1 file HTML nếu bạn muốn, nhưng Markdown là bắt buộc.
• Vị trí lưu: `client_special_event_handler/SPECIAL_EVENTS_CLIENT.md`
  (đặt cạnh PROMPT.md này để tôi tải về trực tiếp).
• Cấu trúc tài liệu đề xuất:
   0. Tóm tắt điều hành (10 phát hiện quan trọng nhất, dạng bảng)
   1. Bảng tra 65 Main OP + cột "sự kiện"
   2. Mỗi sự kiện A–H là 1 chương: điều kiện kích hoạt, frame S→C (bảng field),
      frame C→S, timeline, UI/âm thanh, điều kiện giới hạn, dữ liệu liên quan,
      trích dẫn file/địa chỉ cho từng kết luận
   3. Bảng đặc tả \Data\Festival.Dat (đối chiếu FESTIVAL_DAT_FORMAT.md)
   4. Bảng mapping tên sự kiện ↔ opcode ↔ hàm ↔ chuỗi VISCII (để tôi tra nhanh)
   5. Danh sách câu hỏi chưa giải được + danh sách nguồn còn thiếu cần Ghidra dump
   6. Phụ lục: bảng offset LocalPlayer liên quan sự kiện
• Bảng phải có cột "bằng chứng" (file + dòng/địa chỉ) và "mức A/B/C".
• TUYỆT ĐỐI không đoán bừa. Mỗi lần chưa chắc chắn phải ghi rõ mức (C) và câu hỏi
  cần xác minh thêm.

────────────────────────────────────────────────────────────────────────
5. RÀNG BUỘC
────────────────────────────────────────────────────────────────────────
• CHỈ dùng client (aLogin.exe + dump trong thư mục này + Data/*.dat) làm nguồn.
  KHÔNG dùng, KHÔNG nhắc tới, KHÔNG đối chiếu với bất kỳ mã nguồn server nào
  (Rust hay C# hay bất kỳ ngôn ngữ nào khác). Mọi kết luận phải tự đứng được trên
  bằng chứng từ client.
• Không sửa file trong `client_special_event_handler/` ngoài file output của bạn.
• Nếu cần thêm dữ liệu: đọc `Data/*.dat` trong repo (đã có sẵn Festival.Dat, Npc.dat,
  Item.dat, Scene.dat, ...) và giải mã theo layout bạn tự suy ra từ loader.
````

---

## Phụ lục cho người đọc prompt (không bắt agent dùng, nhưng để bạn đối chiếu)

### P.1 Nguồn dữ liệu đã dùng để lập bản đồ ban đầu

| Nguồn | Vai trò |
|---|---|
| `client_delphi/aLogin.exe` | binary gốc (PE x86, image base `0x00400000`, 8 section: CODE `0x401000–0x7D4400`, DATA `0x7D5000–0x7DA800`, BSS `0x7DB000–0xA118CD`) |
| `client_pseudo_c/` | dump Ghidra chính (pseudo-C + ASM + HEX), 787 file `.c` |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` | dump Ghidra bổ sung, 6 658 file `.c` |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/` | 66 file `case_XXX` (1 file / 1 Main OP) |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump/` | 447 file hex/asm định hướng (dispatch table, literal) |
| `Data/Festival.Dat` | file dữ liệu sự kiện — **đã giải mã xong**, xem `FESTIVAL_DAT_FORMAT.md` |

### P.2 Bản đồ sự kiện → opcode (kết quả tiền phân tích, agent phải kiểm chứng lại)

| Sự kiện | Kênh S→C nghi ngờ | Hàm xử lý chính | Manager |
|---|---|---|---|
| Lôi Đài | `0x3F` sub `01/02/03/04/05/06/07/08/0A` | `FUN_0054C348`, `FUN_0054CBC0` | `TMRBatterManage` (VMT `0x0054C230`) |
| Lôi Đài (thông báo) | `0x27` → `sub_00793843` | `FUN_00729430` | — |
| Thủy chiến (điều khiển) | `0x3F` sub `0B/0C/0D/14/15/16` | `FUN_0054A4F8` … | `TMR_WaterBattleManage` (`0x0054A318`) |
| Thủy chiến (chiến đấu) | `0x40` | `FUN_0074CDD8`, `FUN_005265D4`… | `TWaterManage` (`0x00526448`) |
| Công thành (thông báo) | `0x29` → `sub_00794566` | `FUN_006367CC` | — |
| Công thành (chiến trường) | `0x27` (43 sub-op) | `FUN_00752104`…`FUN_007564D0` | `TCY_OrganManage` (`0x007527E0`) |
| Đua thuyền rồng | `0x46` (8 sub-op) | `FUN_0051B600`…`FUN_0051CBF0` | `TLH_DragonBoatStanding` (`0x007DA1E8`) |
| Sự kiện theo mùa | `0x17` → `FUN_00722CD4` | `FUN_00722CD4` | `TFestival` (VMT `0x0051DE74`) |
| Lịch lôi đài / help | `0x14` → `FUN_0078EC3F` | `FUN_007485BC`, `FUN_00748860` | — |
| Quan chiến (PK) | — | `FUN_0058A964` | — |
| Quiz Bắc Tinh Quân | `0x10` | `FUN_00613230`… | `TWF_Day55QuestManage` (`0x00707488`) |
| GM channel | `0xC7` | `FUN_006128BC`… | `TGmManage` (VMT `0x00613160`) |

### P.3 Lưu ý kỹ thuật cho bạn (người đọc prompt)

* `FUN_00402864` là `System._CheckIntOverflow` — **không phải** hàm đọc dữ liệu.
  Ghidra render nó thành lời gọi hàm trong pseudo-C nhưng thực tế chỉ là kiểm tra tràn số
  sau phép tính. Nhiều lời gọi `CALL 0x00402864` liên tiếp trong cùng một hàm là dấu hiệu
  Ghidra đã tách sai luồng điều khiển — hãy đọc `.asm.txt` khi có nghi vấn.
* Bảng `jumptable_byte200_0x78A8EE.hex` (200 byte) và `jumptable_dword200_0x78A9B6.hex`
  là bảng tra duy nhất giữa Main OP và hàm xử lý; 65/65 Main OP đã được map.
* Chuỗi trong client đều là **VISCII 1.1**. Bảng decode chuẩn nằm ở `src/encoding.rs`
  (`viscii_to_unicode`).
* `Data/WorldBoss.dat` có trong repo nhưng **client này không đọc file đó** (không có
  chuỗi "WorldBoss.dat" trong binary) — đừng dùng làm nguồn.
