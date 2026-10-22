# `client_special_event_handler/` — Bundle mã nguồn client cho module "Sự kiện đặc biệt"

> **Mục đích:** gom toàn bộ mã nguồn C / ASM / HEX đã dump từ Ghidra project của
> `client_delphi/aLogin.exe` (TS Online client) liên quan tới **sự kiện đặc biệt**
> (special events), để đưa cho agent AI phân tích và tái tạo nghiệp vụ phía server.
>
> **Ngày lập:** 2026-10-21 · **Repo:** `/mnt/d/VUDT/GIT_PCC/ts_dream`

---

## 1. Bắt đầu từ đâu

| Muốn làm gì? | Mở file |
|---|---|
| **Giao cho agent AI nghiên cứu** | [`PROMPT.md`](./PROMPT.md) — copy nguyên khối trong ```` ```` ```` rồi dán cho agent |
| **Tự đọc hiểu module** | [`README.md` mục 3](#3-bản-đồ-sự-kiện--opcode--hàm--chuỗi) + `strings_event_catalog.tsv` |
| **Biết format file dữ liệu sự kiện** | [`FESTIVAL_DAT_FORMAT.md`](./FESTIVAL_DAT_FORMAT.md) (đã giải mã byte-exact) |
| **Biết map & NPC của từng sự kiện** | [`EVENT_MAPS_AND_NPCS.md`](./EVENT_MAPS_AND_NPCS.md) + `data_event_maps.tsv` + `data_event_npcs.tsv` |
| **Bổ sung source còn thiếu từ Ghidra** | [`MISSING_DUMPS.md`](./MISSING_DUMPS.md) |
| **Biết file `.dat` nào client đọc & cái nào còn thiếu** | [`DATA_FILES.md`](./DATA_FILES.md) |
| **Biết tên map + map id ↔ tên map (ĐÃ GIẢI MÃ `Scene.Dat`)** | [`SCENE_DAT_FORMAT.md`](./SCENE_DAT_FORMAT.md) + `data_mapid_to_name.tsv` (3 503 map id) |
| **Biết map id nào client có hành vi riêng** | [`EVENT_MAP_ID_CHECKS.md`](./EVENT_MAP_ID_CHECKS.md) + `data_map_id_checks.tsv` (39 map id) |
| **Biết 5 chế độ lôi đài & nhãn chat** | [`EVENT_ARENA_MODES.md`](./EVENT_ARENA_MODES.md) + `data_arena_modes.tsv` |
| **Biết client gửi frame C→S nào** | `data_client_to_server.tsv` (115 dòng: Main OP → sub-op → builder) |
| **Tra chuỗi VISCII trong client** | `strings_viscii_index.tsv` (206 713 chuỗi, đã decode) + `strings_event_catalog.tsv` (349 chuỗi lọc theo nhóm sự kiện) |
| **Đọc logic quest/sự kiện đã giải mã** | `data_files/eve/eve_logic.tsv` (110 083 dòng điều kiện→kết quả) |

---

## 2. Nội dung thư mục

```
client_special_event_handler/
├── PROMPT.md                     ★ PROMPT NGHIÊN CỨU (dán cho agent AI)
├── README.md                     ★ file này — chỉ mục bundle
├── FESTIVAL_DAT_FORMAT.md        ★ đặc tả \Data\Festival.Dat (đã giải mã byte-exact)
├── EVENT_ARENA_MODES.md          ★ bảng 5 chế độ lôi đài & nhãn chat (từ FUN_0053BAB8)
├── EVENT_MAP_ID_CHECKS.md        ★★ map id ↔ hành vi client + thuật toán tra cứu tên map
├── SCENE_DAT_FORMAT.md           ★★★ đặc tả Scene.Dat sau khi giải mã (map id + kind + tên map)
├── EVENT_MAPS_AND_NPCS.md        ★ bản đồ sự kiện: map (Scene.Dat) + NPC (Npc.dat)
├── MISSING_DUMPS.md              ★ danh sách source còn thiếu, cần Ghidra dump bổ sung
├── strings_event_catalog.tsv     ★ 349 chuỗi sự kiện (VA + text VISCII đã decode)
├── strings_viscii_index.tsv      206 713 chuỗi toàn bộ client (VA + text VISCII đã decode)
├── data_map_catalog.tsv          7 480 map: index, tên, X, Y, kind
├── data_event_maps.tsv           187 map liên quan sự kiện
├── data_npc_catalog.tsv          6 659 NPC: id, tên
├── data_event_npcs.tsv           737 NPC liên quan sự kiện (gồm nhóm "thủ binh")
├── data_event_items.tsv          179 vật phẩm sự kiện (id, tên, giá, mô tả tiếng Việt)
├── data_client_to_server.tsv     ★ 115 dòng: Main OP → sub-op → builder (sub-op client THẬT SỰ gửi)
├── data_arena_modes.tsv          ★ bảng mode id lôi đài (0x60–0x63) → nhãn chat + hậu tố
├── data_map_id_checks.tsv        ★★ 39 map id client so sánh (LocalPlayer+0x63A) → hành vi riêng + TÊN MAP
├── data_mapid_to_name.tsv        ★★★ 3 503 map id → tên map (giải mã Scene.Dat)
├── data_files/
│   ├── client_dat/               bản gốc 7 file .dat client đọc (Npc, Scene, Item, Warp…)
│   └── eve/                      ★ Data/eve.emg ĐÃ GIẢI MÃ (2 777 scene) — 9 file TSV + JSONL
├── case_000_*.c … case_065_*.c   66 file — MỘT file cho MỘT Main OP (bảng tra 65 opcode)
├── 00xxxxxx_FUN_*.c / *.asm.txt  pseudo-C + assembly của các hàm liên quan sự kiện
├── lit_*.hex / *.hex             dump byte thô: literal chuỗi, bảng tra, jump-table
├── cs_*.hex / cs_*.asm.txt       bảng dispatch / builder C→S cho từng opcode
├── jumptable_*.hex               bảng tra Main OP ↔ hàm xử lý (byte 0x78A8EE, dword 0x78A9B6)
├── table_0x77F474_200B.hex       byte-map Main OP → chỉ số bảng builder C→S
├── manifest.csv / index.csv      chỉ mục dump Ghidra gốc (giữ nguyên để đối chiếu)
├── tools/
│   ├── festival_dat_decrypt.py   script giải mã Festival.Dat (theo đúng thuật toán client)
│   └── build_event_catalog.py    script sinh catalog map/NPC từ Scene.Dat + Npc.dat
├── festival_dat_decrypted/       10 bản ghi đã giải mã: rec00.bin … rec09.bin
└── (khoảng 1 060 file .c/.asm.txt/.hex tổng cộng ~15 MB)
```

### Nguồn gốc (provenance)

| Thư mục gốc | Vai trò | Ưu tiên |
|---|---|---|
| `client_pseudo_c/` | dump Ghidra chính (header đầy đủ `Callers/Callees/References`) | ★ ưu tiên 1 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` | dump Ghidra bổ sung (6 658 hàm) | ưu tiên 2 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/` | 66 file `case_XXX` | ưu tiên 2 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump/` | 447 file hex/asm định hướng | ưu tiên 2 |
| `client_delphi/aLogin.exe` | binary gốc (PE x86, image base `0x00400000`) | để đối chiếu byte |
| `Data/Festival.Dat`, `Data/Scene.Dat`, `Data/Npc.dat` | file dữ liệu client đọc | ★ |

> Khi cùng tên file tồn tại ở cả `client_pseudo_c/` và `ts_decompile/functions/`,
> bản của `client_pseudo_c/` được ưu tiên (header đầy đủ hơn). 707 file trùng tên —
> đã đối chiếu byte-for-byte là giống nhau.

---

## 3. Bản đồ sự kiện ↔ opcode ↔ hàm ↔ chuỗi

> ⚠ **Đây là bản đồ tiền phân tích** (lập bằng xref thô trên binary), **không thay thế**
> cho kết luận cuối cùng của agent nghiên cứu. Mọi dòng đều kèm địa chỉ để tự kiểm chứng.

| Sự kiện | Kênh S→C | Hàm xử lý | Manager / class | Chuỗi bằng chứng (VA) |
|---|---|---|---|---|
| **Lôi Đài / Võ Đài** (tiên đấu, đoàn P, loạn đấu, thiêu chiến) | `0x3F` sub `01`–`08`, `0A` | `FUN_0054C348`, `FUN_0054CBC0` | `TMRBatterManage` (VMT `0x0054C230`) | `0x00729590`, `0x00729788`, `0x007297D8`, `0x00729914`, `0x0053BD40` |
| Lôi Đài (thông báo) | `0x27` → `sub_00793843` | `FUN_00729430` | — | `0x00729590`…`0x00729914` |
| Lôi Đài (lịch thi / help) | `0x14` → `FUN_0078EC3F` | `FUN_007485BC`, `FUN_00748860` | — | `0x00797210`, `0x00797368`, `0x00797E94` |
| **Thủy chiến / Ship War** (điều khiển) | `0x3F` sub `0B`–`16` | `FUN_0054A4F8`, `FUN_0054AAA0`, `FUN_0054B5E8`, `FUN_0054B6D0` | `TMR_WaterBattleManage` (`0x0054A318`), `TMR_WBManageOrgManage` | `0x0054A910`, `0x0054A96C`, `0x0054A9CC` |
| **Thủy chiến** (chiến đấu) | `0x40` | `FUN_0074CDD8`, `FUN_005265D4`… | `TWaterManage` (`0x00526448`) | `0x00557E64`, `0x00558558`, `0x005587F8`, `0x005590AC` |
| **Công thành chiến** (thông báo) | `0x29` → `sub_00794566` | `FUN_006367CC` | — | `0x00636B30`, `0x00636BB8`, `0x00636CBC`, `0x00636D28` |
| **Công thành / thành trì** (quân đoàn, boss) | `0x27` (43 sub-op) | `FUN_00752104`…`FUN_007564D0` | `TCY_OrganManage` (`0x007527E0`), `TWarElephant` (`0x006D0C20`) | `0x00590958`, `0x00798CB8`, `0x00798530` |
| **Đua thuyền rồng** | `0x46` (8 sub-op) | `FUN_0051B600`…`FUN_0051CBF0` | `TLH_DragonBoatStanding` (`0x007DA1E8`), Race Controller (`0x007DA714`) | `0x0051B370`, `0x0051B890`, `0x0051C05C`, `0x0051D340` |
| **"2K" = Giải cứu Nhị Kiều Giang Đông** | `0x14` → `FUN_0078EC3F` | `FUN_0078EC3F` | — | `0x00797A5C`, `0x00797B00` |
| **Sự kiện theo mùa / lễ hội** | `0x17` → `FUN_00722CD4` | `FUN_00722CD4` | `TFestival` (VMT `0x0051DE74`) | `0x00724424`, `0x007245C0`, `0x007254C0`, `0x007257AC`, `0x0072690C` |
| **Dữ liệu lễ hội (lore)** | file cục bộc | `FUN_0051E00C` (load), `FUN_0051DECC` (decrypt), `FUN_0051E2DC` (tra ngày) | `TFestival` | xem `FESTIVAL_DAT_FORMAT.md` |
| **Quan chiến (PK trong sự kiện)** | — | `FUN_0058A964` | — | `0x0058B4FC`, `0x00595CA8`, `0x00644CF0` |
| **Quiz Bắc Tinh Quân (Day-55)** | `0x10` | `FUN_00613230`… | `TWF_Day55QuestManage` (`0x00707488`) | `0x00798478`, `0x0074871C` |
| **Kênh GM** | `0xC7` | `FUN_006128BC`… | `TGmManage` (VMT `0x00613160`) | `0x00796CAC` ("Hoạt động lôi đài đấu trận kết thúc") |

### Ba tên sự kiện người dùng nêu — đã tra

| Tên | Kết luận | Chi tiết |
|---|---|---|
| **"2K"** | ✅ **Giải cứu Nhị Kiều Giang Đông** | map *"Đồng Tước đài"* (Scene.Dat 414/438/443/446–448/450/458–461), thông báo `0x00797A5C` + `0x00797B00` qua `FUN_0078EC3F` (OP `0x14`) |
| **"40 NPC"** | ✅ **Lôi Đài Khiêu Chiến** | item `31147` *"Vé vip chuyên dùng trong lôi đài khiêu chiến"*; item `30014…30063` *"Thắng lệnh N — Khiêu chiến N, dùng lệnh này lãnh thưởng"* (50 lệnh) |
| **"Hổ Lao Quan"** | ✅ **Chiến trường THỦY CHIẾN** | map *"Hổ Lao Quan"* (Scene.Dat 1124/1139/1140/1152), cung *"Tiền tiêu Hổ Lao"* (727), NPC 4906 *"HổLao QThủBinh"*, nhiệm vụ *"Trừ diệt Nghiêm Bạch Hổ"* |
| *(tự phát hiện)* | **Loạn đấu** (Solo PK) | item `23160` *"Nhẫn bạo kích loạn đấu đài, 1 tuần"*, `46140` *"Hộp gỗ tặng thưởng loạn đấu"* |

---

## 4. Những phát hiện quan trọng đã có sẵn trong bundle

| # | Phát hiện | Mức | Ảnh hưởng |
|:-:|---|:-:|---|
| 1 | **Client không tự kích hoạt sự kiện nào.** Mọi sự kiện đều do Server gửi frame xuống. Đã map được 9 kênh kích hoạt (0x10, 0x14, 0x17, 0x27, 0x29, 0x3F, 0x40, 0x46, 0xC7) | A | Xác định rõ ranh giới trách nhiệm client ↔ server |
| 2 | **`\Data\Festival.Dat`** là file dữ liệu duy nhất của client dành cho sự kiện. Layout: 10 bản ghi × 2031 byte, 2 bản ghi cuối là **khoá XOR 2031 byte** (bản ghi chẵn dùng keyA, bản ghi lẻ dùng keyB). Đã giải mã thành công → 7 lễ hội (Tết Nguyên Đán, Tết Nguyên Tiêu, Valentine, Cà Thang Tử, Giỗ Tổ, Phật Đản, Đoan Ngọ) | A | 🔴 Server **không** cần gửi lore, nhưng **phải** gửi mốc ngày để client mở đúng bản ghi |
| 3 | **`Data/Scene.Dat`** đã giải mã: header 134 byte, 7 480 bản ghi × 134 byte, tên map là ShortString tại `+0x03`/`+0x04`, `+0x00/+0x01` = toạ độ X/Y thế giới, `+0x02` = kind. Đã lập **catalog 7 480 map** + **187 map sự kiện** | A | Cho biết chính xác map nào thuộc sự kiện nào (5 chế độ lôi đài, 9 tầng Cửu Thiên, 8 cung thủy chiến, …) |
| 4 | **`Data/Npc.dat`** đã giải mã: record 92 byte, tên ở `+0x01..+0x0E` (đảo ngược + VISCII). Đã lập **catalog 6 659 NPC** + **737 NPC sự kiện**, trong đó có **nhóm "thủ binh"** xác định chính xác NPC của từng sự kiện (4906 Hổ Lao, 4896 thủy chiến, 4920 lôi đài, 4945 niên thú, 3962/5994 tháp lôi phong) | A | Server cần spawn đúng NPC thủ binh khi mở sự kiện |
| 4b | **`Data/eve.emg` đã giải mã** (bộ giải mã mới: `examples/dump_eve.rs`) — 2 777 scene, `scene_id` = **map id thật của server** (10851, 12000, 12851, 15000, 15510, 60011… trùng `Data/Warps.txt`). 110 083 dòng điều kiện→kết quả, **60 430 dòng có điều kiện `QuestStep`** | A | 🔴 **Mảnh ghép quan trọng nhất**: phần lớn sự kiện trong TS Online điều khiển bằng quest + Eve script, KHÔNG bằng opcode riêng. Không có file này thì agent sẽ hiểu sai bản chất nghiệp vụ. |
| 4c | **`Data/Item.dat`** đã giải mã (repo có sẵn `Data/items.json`, 8 675 item) → lọc ra **179 vật phẩm sự kiện**; mô tả tiếng Việt của chúng tự xác nhận tên sự kiện ("Vé vip … lôi đài khiêu chiến", "Công thành đồ sao phỏng", "Nhẫn bạo kích loạn đấu đài") | A | Xác định tên chính xác các sự kiện mà chỉ có tên tiếng Việt trong DB vật phẩm |
| 4d | **Bảng 5 chế độ lôi đài đã giải mã** (`FUN_0053BAB8`): mode id `0x60/0x61/0x62/0x63` → `[Lôi đài tranh đấu 4/3/2/1]`; hậu tố theo flag: *"Tỉ thí"*, *"Đòan P"* (≈Đoàn P), *"Lọan đấu"* (Solo PK), *"Đấu trận cấp 100"*. Client tự lập danh sách người chơi theo chế độ trong mảng record 7 byte tại `self+0x164` | A | Cho biết **5 chế độ lôi đài** và **mã mode** server phải gửi |
| 4e | **Bảng sub-op client THỰC SỰ gửi C→S** đã trích từ 9 bảng builder: `0x3F` chỉ gửi sub 1/2/3; `0x40` sub 1/2; `0x46` chỉ sub 1; `0x14` sub 4/6/9; `0x29` sub 1–4; `0x2A` sub 1/2; `0x2D` sub 1–4; `0xC7` sub 4/6; `0x27` **28/57**; `0x10` 36/37 (bảng nhảy). Tất cả sub còn lại → **epilogue = KHÔNG gửi** | A | 🔴 Loại bỏ được phần lớn handler server phải viết thừa |
| 4h | ⭐ **`Scene.Dat` đã giải mã xong.** Mỗi bản ghi 134 byte chứa **(mapId u16 mã hóa, kind u8, tên map VISCII)**; giải mã `w = (w ^ 0xEA6C) - 9`. **3 503 map id → tên map**; 2 455/2 777 scene `eve.emg` khớp map id | A | 🔴 Client tra tên map bằng cặp **`(mapId, kind)`** — `kind` do **server** gửi xuống. Sai `kind` ⇒ hiện sai tên bản đồ |
| 4f | **`*(int16*)(LocalPlayer + 0x63A)` = map id hiện tại.** Client so sánh trường này ở **86 hàm** với **39 map id** — phần lớn là map sự kiện (12000 đua thuyền, 12003 lôi đài, 10991, 10701, 59000…). Manager map = class `TKC_Scene` (VMT `0x00535380`); hàm tra tên map = `FUN_0053558C` | A | 🔴 **Server phải đặt đúng `mapid`** khi mở sự kiện, nếu không client chạy nhánh mặc định ⇒ sự kiện "không xảy ra" phía client |
| 4g | **Round-5 đã dump xong 85 builder C→S** (pseudo-C) ⇒ đọc được layout byte từng frame client gửi: vd OP `0x3F` sub 1 = ShortString cap 2 + `FUN_0077eb1c` (Word LE) | A | Chốt được wire format chiều C→S |
| 5 | **Toàn bộ 65 Main OP đã map** (bảng byte `0x78A8EE` + bảng dword `0x78A9B6`) | A | Nền tảng để agent tra cứu nhanh |
| 6 | **`FUN_00402864` là `System._CheckIntOverflow`**, không phải hàm đọc dữ liệu — Ghidra render sai, nhiều lời gọi liên tiếp trong `FUN_0051E00C` là dấu hiệu tách sai luồng | A | Tránh hiểu sai luồng loader |
| 7 | **Dispatcher `FUN_0078A89C` thực tế dài 46 084 byte**, `index.csv` ghi sai `size_bytes = 265` ⇒ 58% chưa từng được disassemble | A | Ẩn chứa ít nhất 1 nhánh sự kiện (`0x00796CAC` tại `0x0078AF6E`) |
| 8 | Chuỗi client là **VISCII 1.1**; đã trích & decode **206 713 chuỗi** kèm Virtual Address | A | Agent không cần tự giải mã |
| 9 | `Data/WorldBoss.dat` có trong repo nhưng **client này không đọc** | A | Đừng dùng làm nguồn |

---

## 5. Quy ước đọc file

* Mỗi file `.c` bắt đầu bằng header:
  ```
  // Function: FUN_007XXXXX
  // Entry:   007XXXXX
  // Size:    N bytes
  // Callers: ...
  // Callees: ...
  // References to entry: 007XXXXX -> 007XXXXX [UNCONDITIONAL_CALL]
  ```
  → dùng `Callers`/`Callees` để đi call-graph, **đừng** đọc tuần tự toàn bộ corpus.
* File `.asm.txt` trong `ts_decompile/functions/` **không** có tiền tố địa chỉ (chỉ có trong
  `client_pseudo_c/`); cả hai đều dùng hex không zero-pad (`0x51b890`).
* Chuỗi literal: `lit_<start>_<end>.hex` — dòng đầu `; 0xAAAAAAA..0xBBBBBBB`, sau đó mỗi
  dòng 16 byte hex. Dùng `tools_decode_literals.py` (`.scratch/client-pseudo-op-code/`) để decode.
* Bảng tra: `jumptable_byte200_0x78A8EE.hex` (Main OP → index case),
  `jumptable_dword200_0x78A9B6.hex` (index case → địa chỉ hàm),
  `table_0x77F474_200B.hex` (Main OP → index builder C→S),
  `table_0x77F53C_dword200.hex` (index builder → địa chỉ builder).
