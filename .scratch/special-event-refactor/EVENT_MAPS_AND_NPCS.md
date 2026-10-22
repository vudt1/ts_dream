# BẢN ĐỒ SỰ KIỆN ĐẶC BIỆT — map & NPC (trích từ `Data/Scene.Dat` + `Data/Npc.dat`)

> **Nguồn:** 2 file dữ liệu mà **client đọc cục bộ** (đã xác minh qua loader
> `FUN_00535600` = Scene.Dat, `FUN_00623F28` = Npc.dat).
> **Mức bằng chứng:** **(A)** — đọc trực tiếp byte + giải mã VISCII 1.1.
> **Script sinh lại:** `tools/build_event_catalog.py`.

---

## 1. Layout `Data/Scene.Dat` (đã giải mã — xem [`SCENE_DAT_FORMAT.md`](./SCENE_DAT_FORMAT.md))

```
FileSize    = 1 002 454 byte
RecordSize  = 134 byte
Header      = KHÔNG có (134 byte đầu chính là bản ghi số 0 = sentinel)
So ban ghi  = 1 002 454 / 134 = 7 481   (bản ghi 0 = sentinel, 7 480 bản đồ thật)
```

Mỗi bản ghi (134 byte):

| Offset | Kích thước | Kiểu | Nội dung |
|---:|---:|---|---|
| `+0x00` | 2 | `u16` | **Map ID** — mã hóa: `giá trị = (w ^ 0xEA6C) - 9` |
| `+0x02` | 1 | `byte` | **Kind** — biến thể/bậc của cùng một map (1..44) |
| `+0x03` | 1 | `byte` | độ dài tên (ShortString) |
| `+0x04` | ≤ 20 | `char[]` VISCII | **Tên map** (rõ, không mã hóa) |
| `+0x18` … `+0x85` | 110 | 55 × `u16` | vùng dự trữ — mã hóa cùng phép `(w ^ 0xEA6C) - 9`, nhưng trong file **luôn = 0** |

> **Ý nghĩa:** `Scene.Dat` là **danh bạ map**: mỗi dòng là một *phòng* (kind) của một map
> (ví dụ `mapId=12003` có 15 kind: *Lôi đài thôn thực* (8), *Lôi đài 5 vs 5* (9),
> *LôiĐàiHỏiĐápNămMới* (10/11/12), *Tầng 1 Tâm Ma tháp* (13) …).
> Server gửi **MapID** + **kind** + toạ độ trong frame chuyển map; client tra tên để hiển thị.

---

## 2. Bản đồ sự kiện đặc biệt — `data_event_maps_by_name.tsv` / `data_mapid_to_name.tsv`

> ⚠ **ĐÍNH CHÍNH (Round-6):** các cột **X / Y** trong bản cũ **không phải toạ độ** —
> chúng là **2 byte của mã map đã mã hoá**. Bản đúng: **`mapId` + `kind` + tên map**.
> (File `Scene.Dat` **không** chứa toạ độ map; toạ độ do **server** gửi kèm frame chuyển map.)
>
> 🔑 **Khóa tra cứu của client là cặp `(mapId, kind)`** — cùng một `mapId` có thể có
> tới 44 `kind` (mỗi kind = một "phòng"/"lần" của cùng map, ví dụ 9 tầng Cửu Thiên).

### 2.1 LÔI ĐÀI (arena)

> Toàn bộ bản ghi có tên chứa "Lôi đài" / "LôiĐài" / "Tâm Ma tháp"

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Khu chuẩn bị lôi đài** | 54901, 54902, 54903, 54904 | 1 | 4 | 6736, 6737, 6738, 6739 |
| **Lôi đài 5 vs 5** | 12003 | 9 | 1 | 347 |
| **Lôi đài Kiến Nghiệp** | 18001, 18003 | 1 | 2 | 2479, 2495 |
| **Lôi đài Lạc Dương** | 13004, 13005 | 1, 2 | 2 | 924, 930 |
| **Lôi đài thôn thực** | 12003 | 8 | 1 | 346 |
| **Lôi đài tỉ võ** | 12011, 12061 | 8 | 2 | 369, 413 |
| **LôiĐàiHỏiĐápNămMới** | 12003 | 10, 11, 12 | 3 | 348, 349, 350 |
| **Tầng 1 Tâm Ma tháp** | 12003, 12994 | 1, 13 | 2 | 351, 861 |
| **Tầng 2 Tâm Ma tháp** | 12993, 12995 | 1, 2 | 2 | 860, 863 |
| **Tầng 3 Tâm Ma tháp** | 12994, 12996 | 1, 2 | 2 | 862, 865 |
| **Tầng 4 Tâm Ma tháp** | 12995, 12997 | 1, 2 | 2 | 864, 867 |
| **Tầng 5 Tâm Ma tháp** | 12996, 12998 | 1, 2 | 2 | 866, 869 |
| **Tầng 6 Tâm Ma tháp** | 12997, 12999 | 1, 2 | 2 | 868, 871 |
| **Tầng 7 Tâm Ma tháp** | 12998 | 2 | 1 | 870 |
| **Tầng 8 Tâm Ma tháp** | 12999 | 1 | 1 | 872 |

### 2.2 THỦY CHIẾN — các chiến trường

> Căn cứ "Tiền tiêu" + bản đồ

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Cửa thành Cự Lộc** | 12000, 12012, 12014, 12133, 12173, 12174, 12202, 12222, 12242, 12262, 12282, 12302 | 1, 2, 4, 10 | 13 | 284, 375, 378, 382, 465, 474, 475, 482, 486, 489, 492, 495… |
| **Hổ Lao Quan** | 13423, 13432, 13433, 13438 | 1, 2 | 4 | 1125, 1140, 1141, 1153 |
| **Mỏ khoáng Cự Lộc** | 12013, 12521, 12522, 12523, 12524, 12525, 12526, 12527, 12528, 12529, 12530, 12531, 12532, 12533, 12534, 12535, 12536, 12537, 12538, 12539, 12540, 12541, 12542, 12543, 12544, 12545, 12546, 12547, 12548, 12549, 12550, 12551, 12552, 12553, 12554, 12555, 12556, 12557, 12558, 12559, 12560, 12561, 12562, 12563, 12564, 12565, 12566, 12567, 12568, 12569, 12570, 12571 | 1, 2, 3, 4, 5, 6 | 103 | 380, 585, 586, 587, 588, 590, 591, 592, 593, 594, 595, 596… |
| **Rừng Cự Lộc** | 12011 | 12 | 1 | 373 |
| **Rừng Hổ Lao** | 13433, 13571 | 3 | 2 | 1143, 1288 |
| **ThôngĐạo Hổ Lao Quan** | 13431, 13437 | 2 | 2 | 1138, 1152 |
| **Tiền tiêu Cự Lộc** | 12711 | 2 | 1 | 726 |
| **Tiền tiêu Hồ BàDương** | 12711 | 5, 6, 7, 8, 9 | 5 | 729, 730, 731, 732, 733 |
| **Tiền tiêu Hổ Lao** | 12711 | 4 | 1 | 728 |
| **Tiền tiêu ĐạiHưngSơn** | 12711 | 3 | 1 | 727 |
| **Tế đàn Cự Lộc** | 12011, 12012, 12013, 12014 | 1, 3, 13 | 4 | 374, 377, 379, 381 |
| **ĐườngLớn Hổ Lao Quan** | 13432, 13434, 13435, 13436, 13437 | 1, 2 | 5 | 1139, 1145, 1148, 1150, 1151 |
| **Động Hổ Lao Sơn** | 13436, 13572, 13573, 13574, 13575, 13576, 13577, 13578, 13579, 13580, 13581, 13582, 13583, 13584, 13585, 13586, 13587, 13588 | 1, 2, 3, 4, 5 | 34 | 1149, 1289, 1290, 1291, 1292, 1293, 1294, 1295, 1296, 1297, 1298, 1299… |
| **Động Đại Hưng Sơn** | 12000, 12501, 12502, 12503, 12504, 12505, 12506, 12507, 12508 | 1, 2, 3, 4, 5, 19 | 17 | 293, 552, 553, 554, 555, 556, 557, 558, 559, 560, 561, 562… |

### 2.3 CỬU THIÊN (9 tầng, 2 bản)

> Tầng 1 → Tầng 9 Cửu Thiên

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Cửu Thiên** | 33033 | 2 | 1 | 6276 |
| **Tầng 1 Cửu Thiên** | 33911, 33912, 33913, 49960, 49962 | 1, 2 | 5 | 6309, 6311, 6312, 6696, 6698 |
| **Tầng 2 Cửu Thiên** | 33912, 33914, 49961, 49963 | 1 | 4 | 6310, 6314, 6697, 6700 |
| **Tầng 3 Cửu Thiên** | 33913, 33915, 49962, 49964 | 1, 2 | 4 | 6313, 6316, 6699, 6702 |
| **Tầng 4 Cửu Thiên** | 33914, 33916, 49963, 49965 | 1, 2 | 4 | 6315, 6318, 6701, 6704 |
| **Tầng 5 Cửu Thiên** | 33915, 33917, 49964, 49966 | 1, 2 | 4 | 6317, 6320, 6703, 6706 |
| **Tầng 6 Cửu Thiên** | 33916, 33918, 49965, 49967 | 1, 2 | 4 | 6319, 6322, 6705, 6708 |
| **Tầng 7 Cửu Thiên** | 33917, 33919, 49966, 49968 | 1, 2 | 4 | 6321, 6324, 6707, 6710 |
| **Tầng 8 Cửu Thiên** | 33918, 33920, 49967, 49969 | 1, 2 | 4 | 6323, 6326, 6709, 6712 |
| **Tầng 9 Cửu Thiên** | 39919, 49968 | 2 | 2 | 6325, 6711 |

### 2.4 ĐUA THUYỀN RỒNG / NGƯU CHỮ

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Ao Ngưu Chữ** | 49051, 49054, 49055 | 1, 3 | 3 | 6585, 6592, 6595 |
| **Bến đò  Ngưu Chữ** | 18801 | 1 | 1 | 2763 |
| **Cửa vào Ngưu Chữ** | 15000, 49051, 49216, 49226 | 1, 2, 4, 35 | 4 | 1974, 6586, 6629, 6635 |
| **Rừng Ngưu Chữ** | 15000, 18000, 18801, 18802, 18803, 18804, 49054 | 1, 2, 3, 4, 15, 16, 31 | 9 | 1970, 2454, 2455, 2764, 2766, 2767, 2768, 2770, 6594 |
| **Thuyền rồng Tào Phi** | 15893, 18703 | 3 | 2 | 2336, 2757 |
| **Thuỷ động Ngưu Chữ** | 49054 | 2 | 1 | 6593 |
| **Trấn Ngưu Chữ** | 49051, 49052, 49053, 49059, 49136, 49185, 49186 | 1, 2 | 8 | 6583, 6584, 6590, 6591, 6597, 6607, 6622, 6623 |
| **U tàu Ngưu Chữ** | 49059 | 4 | 1 | 6599 |

### 2.5 SỰ KIỆN THEO MÙA

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Bác Vọng Ba** | 21000, 21801, 21802, 21803, 21804 | 1, 2, 4 | 7 | 3180, 3548, 3549, 3551, 3552, 3553, 3554 |
| **Ghi danh Đoan Ngọ** | 12000 | 43 | 1 | 311 |
| **Hội thi nướng thịt** | 12005 | 3, 4, 5 | 3 | 356, 357, 358 |
| **Mê cung Hứa Xương** | 13501, 13502, 13503, 13504, 13505, 13506, 13507 | 1, 2 | 12 | 1226, 1227, 1228, 1229, 1230, 1231, 1233, 1234, 1235, 1236, 1237, 1238 |
| **Mê cung Quỷ Vương** | 10988, 10989, 61178, 61551, 61552, 61553, 61554, 61555, 61556, 61557, 61558, 61559, 61560, 61561, 61562, 61563, 61564, 61565, 61566, 61567 | 1, 2, 3, 4, 5, 6 | 39 | 36, 37, 38, 7325, 7402, 7403, 7404, 7405, 7406, 7407, 7408, 7409… |
| **Mê cung Vạn Thắng** | 10986, 10987 | 1, 2 | 2 | 34, 35 |
| **Mê cung sinh tồn** | 10991 | 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17 | 16 | 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52… |
| **Nguyên Tiêu ký vực** | 12001 | 19 | 1 | 331 |
| **Nhà cỏ Quỷ Vương** | 61051, 61551 | 1 | 3 | 7303, 7400, 7401 |
| **Nhà gỗ Vọng Bình** | 19011 | 5 | 1 | 2883 |
| **Phật Giới** | 59000 | 16 | 1 | 6975 |
| **Tháp Lôi Phong** | 12001, 12941 | 1, 20 | 2 | 332, 839 |
| **Thôn Quỷ Vương** | 57536, 61178 | 2 | 2 | 6945, 7326 |
| **Thôn Tiên Giới** | 59000, 59101, 59131 | 1, 6 | 3 | 6965, 6981, 6983 |
| **Thôn Vọng Bình** | 19000, 19132, 19172, 19173, 19174 | 1, 4 | 5 | 2857, 2893, 2896, 2897, 2898 |
| **Tiên Giới** | 59001, 59011, 59401, 59411, 59421, 59431, 59441, 59451, 59461, 59471, 59801, 59811, 59821, 59831, 59842, 62000 | 1, 2, 3 | 16 | 6978, 6980, 6985, 6987, 6989, 6993, 6999, 7000, 7007, 7009, 7014, 7017… |
| **Tế đàn Tiên Giới** | 59000 | 2 | 1 | 6961 |
| **Vọng Hương Lộ** | 60000, 60452 | 1, 8 | 2 | 7035, 7089 |
| **Vọng Hương Đài** | 60451 | 2 | 1 | 7088 |
| **Động Quỷ Vương** | 57518, 57531, 57532, 57533, 57534, 57535, 57536, 61565 | 1, 2, 3 | 12 | 6918, 6935, 6936, 6937, 6938, 6939, 6940, 6941, 6942, 6943, 6944, 7435 |

### 2.6 CÔNG THÀNH / ĐIỂM BINH ĐÀI

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Vương thành bí đạo** | 11031, 11041, 12071, 12081, 19031 | 5 | 5 | 117, 118, 435, 436, 2890 |
| **Điểm binh đài** | 12022, 13012, 21012, 21706, 25013 | 1, 7, 8, 9 | 5 | 392, 941, 3250, 3441, 4297 |

### 2.7 GIỚI KIỀU / NHỊ KIỀU GIANG ĐÔNG

| tên map | mapId | kind | số bản ghi | idx |
|---|---:|---|---|---|
| **Chân núi Kiều Qua Lí** | 29402 | 1 | 1 | 5240 |
| **Giới Kiều** | 12000 | 5, 13 | 2 | 280, 287 |
| **Kiều Qua Lí sơn cước** | 29000 | 11 | 1 | 5180 |
| **Sườn núi Kiều Qua Lí** | 29401, 29403 | 1, 2 | 2 | 5239, 5242 |
| **Thất Tịch Kiều** | 54821 | 4 | 1 | 6720 |
| **Đỉnh núi Kiều Qua Lí** | 29402 | 2 | 1 | 5241 |

## 3. NPC liên quan sự kiện — `data_event_npcs.tsv` (737 NPC)

### 3.1 NPC "thủ binh" (người gác cổng sự kiện) — nhóm quan trọng nhất

| npcId | Tên | Sự kiện |
|---:|---|---|
| **4906** | **HổLao QThủBinh** | **Hổ Lao Quan** (thủy chiến) — xác nhận tên sự kiện của người dùng |
| 4896 | TỵThuỷQThủBinh | Thủy chiến (giám thị tàu) |
| 4920 | LDươngTThủBinh | Lôi Đài tỷ (giám thị đấu trường) |
| 4945 | Thủ binh N.T | Niên Thú |
| 3962 / 5994 | Lôi ph thủbinh | Tháp Lôi Phong |
| 4107 | ThủbinhP.H.Đài | Phi Hổ Đài |
| 2868 | ThủbinhV.Xương | Võ Xương |
| 1643 / 1644 | Bắc Tinh Quân / Nam Tinh Quân | Quiz (sự kiện GM) |

### 3.2 Boss / mục tiêu của sự kiện

| npcId | Tên | Sự kiện |
|---:|---|---|
| 1216, 4224, 4389, 5116, 6293 | **Nghiêm Bạch Hổ** | nhiệm vụ *"Trừ diệt Nghiêm Bạch Hổ"* (thủy chiến) |
| 4975 / 4978 | Bạch Hổ / Quỷ Bạch Hổ | |
| 3697 / 3706 / 3707 | Hổ / Dã phiêu hổ / Phiêu hổ vương | |
| 3718 | Mãnh hổ vương | |
| 3789 | Hổ Cự Hán | |
| 3891 | Quỷ Hổ Tướng | |
| 4983 | Dạ Hổ Vương | |
| 6078–6082 | Hổ Vương Tinh / Hổ Tinh | |
| 6589 | Hổ Vương Dị Giới | |
| 3883 | Thiên Quỷ Vương | Halloween |
| 4956–4959 | QủyVươngY.Kim | Halloween |
| 3998 / 4962 / 3874 | Thiên Niên Thú / Thú Thiên Niên | Niên Thú |
| 2192 / 2193 / 2194 / 2195 | Hổ Quân Vệ / Trưởng / Võ Sĩ / Dũng Sĩ | quân đoàn công thành |
| 2171 | Hổ Náo Quân | |
| 2512 | Hổ Tiêu Võ Sĩ | |
| 2961 / 2962 | Hổ / Mãnh hổ | |
| 1688 | Hổ Cang Đường | |
| 5501 | Hổ lâm quân | |
| 2229 | QuânLầuThuyền | thủy chiến (thuyền) |

### 3.3 Đấu trường lôi đài

| npcId | Tên |
|---:|---|
| 1706 | Chủ LôiĐàiNgầm |
| 1756 | LôiĐàiTruyềnSư |
| 2149 | **Quản Lý Lôi Đài** |
| 3961 / 5993 | Lôi phthủtướng |
| 5995 | Lôi Phủ Quan |

---

## 4. Ba tên sự kiện người dùng nêu — kết luận (đã đối chiếu `Scene.Dat` đã giải mã)

| Tên người dùng | Kết luận sau khi tra client | Bằng chứng trong client |
|---|---|---|
| **"2K"** | ✅ **XÁC NHẬN** — sự kiện **"Giải cứu Nhị Kiều Giang Đông"**, diễn ra tại **Đồng Tước Đài** (Diệp Thành). Client có 2 chuỗi thông báo (bắt đầu / kết thúc), phát bởi **`FUN_0078EC3F` (case_018 = Main OP `0x14`)**. | `0x00797A5C` *"Hành động giải cứu Nhị Kiều Giang Đông đã được bắt đầu, bạn hãy nhanh chống khởi hành đến Đồng Tước Đài trong Diệp Thành để triển khai hành động giải cứu."* · `0x00797B00` *"...đã kết thúc…"* · **`Scene.Dat`: "Đồng Tước đài" → `mapId=12061`, `kind=10`** (cùng map còn có *"Lôi đài tỉ võ"* kind 8, *"Tháp thí luyện"* kind 11) |
| **"40 NPC"** | ⚠ **NGHI NGỜ** — một chế độ của **"Lôi Đài Khiêu Chiến"**. Client **không** có chuỗi "Khiêu Chiến" (quét toàn binary, `Item.dat`, `eve.emg`, `AchievementData.dat`, `Dispatch.dat`, `LeaderboardInfo.dat` — 0 hit). Bằng chứng gián tiếp: `0x004D8A04` = `"Top 40"` (bảng xếp hạng 40 người) và `0x0074871C` *"Khiêu gia quan đến quản trường Quận Trác chiệu mộ những người chơi thông minh giải câu đố, người nào có hứng thú mau đến ghi danh tham gia nào."* (quiz Bắc Tinh Quân, `FUN_007485BC`, cũng gọi từ `FUN_0078EC3F`). | **`Scene.Dat`: `mapId=12003` có 15 kind** → kind 8 *"Lôi đài thôn thực"*, kind 9 *"Lôi đài 5 vs 5"*, kind 10/11/12 *"LôiĐàiHỏiĐápNămMới"*, kind 13 *"Tầng 1 Tâm Ma tháp"* · thêm *"Lôi đài tỉ võ"* (`12011`/`12061` kind 8), *"Lôi đài Lạc Dương"* (`13004`/`13005`), *"Lôi đài Kiến Nghiệp"* (`18001`/`18003`), *"Khu chuẩn bị lôi đài"* (`54901`–`54904`) |
| **"Hổ Lao Quan"** | ✅ **XÁC NHẬN** — CHIẾN TRƯỜNG **THỦY CHIẾN** (không phải boss). Có bản đồ riêng, đường lớn, thông đạo, rừng và **Động Hổ Lao Sơn** (34 bản ghi). Nhiệm vụ liên quan: *"Trừ diệt Nghiêm Bạch Hổ"*. | **`Scene.Dat`**: *"Hổ Lao Quan"* → `mapId=13423/13432/13433/13438` (kind 1,2) · *"ThôngĐạo Hổ Lao Quan"* → `13431/13437` · *"ĐườngLớn Hổ Lao Quan"* → `13432`–`13437` · *"Rừng Hổ Lao"* → `13433/13571` · *"Động Hổ Lao Sơn"* → `13436`, `13572`–`13588` · *"Tiền tiêu Hổ Lao"* → `mapId=12711` kind 4 · `Npc.dat` 4906 (*"HổLao QThủBinh"*) · client string `0x005590AC` (*"Trừ diệt Nghiêm Bạch Hổ"*) |

> **Hành động đề xuất cho agent:** quét `Data/Item.dat` (item loại "đạo cụ sự kiện"),
> `Data/AchievementData.dat`, `Data/Dispatch.dat` và `Data/eve.emg`
> (mục `Eve_Quest`, quest id 10xxx & 12xxx) để tìm 2 sự kiện còn lại.

---

## 5. Bảng tổng hợp: sự kiện → map (đã giải mã) → NPC thủ binh

| Sự kiện | `mapId` (+ `kind`) — theo `Scene.Dat` | NPC thủ binh | Chuỗi client chính |
|---|---|---|---|
| **Lôi Đài (5 chế độ)** | `12003` kind 8/9/10/11/12 (*thôn thực* / *5 vs 5* / *HỏiĐápNămMới* ×3) · `12011`+`12061` kind 8 (*tỉ võ*) · `12993`–`12999` kind 1–3 (*Tâm Ma tháp*) · `13004`/`13005` (*Lạc Dương*) · `18001`/`18003` (*Kiến Nghiệp*) · `54901`–`54904` (*khu chuẩn bị*) | 2149, 1756, 1706, 3961/5993/5995, 4920 | `0x00729590`…`0x00729914`, `0x0053BD40` |
| **Thủy chiến** | `12711` kind 2/3/4/5–9 (*Tiền tiêu* Cự Lộc / ĐạiHưngSơn / **Hổ Lao** / Hồ BàDương ×5) · `13423`–`13438` + `13571`–`13588` (*Hổ Lao Quan* / *Động Hổ Lao Sơn*) | 4896, **4906**, 2229, 1216 (Nghiêm Bạch Hổ) | `0x00557E64`, `0x00558558`, `0x005587F8`, `0x005590AC`, `0x0054A910` |
| **Cửu Thiên (9 tầng ×2)** | `33911`–`33920` (tầng 1→9) + `49960`–`49969` (bản 2) + `33033` kind 2 | — | *(không có chuỗi riêng trong client)* |
| **Đua thuyền rồng** | `15893`/`18703` kind 3 (*Thuyền rồng Tào Phi*) | — | `0x0051B370`…`0x0051D340` |
| **Công thành / điểm binh đài** | `12022`/`13012`/`21012`/`21706`/`25013` (*Điểm binh đài*) · `11031`/`11041`/`12071`/`12081`/`19031` (*Vương thành bí đạo*) | 2192–2195 (Hổ Quân Vệ/Trưởng/Võ Sĩ/Dũng Sĩ), 2512 | `0x00636B30`…`0x00636DAC`, `0x00798CB8` |
| **Lễ Thất Tịch / Đoan Ngọ** | `12000` kind 43 (*Ghi danh Đoan Ngọ*), kind 44 (*QuảngTrường ĐoànViên*) · `12001` kind 14 (*NguyênTiêu nhạc viên*), 17 (*Nguyên Tiêu ký vực*), 16 (*Chiếntrường dũng giả*) | 321, 323 | `0x007257AC`, `0x00724424` |
| **Tháp Lôi Phong** | `12001` kind 1, `12941` kind 20 · `12406`/`12407`/`12418`/`12419` kind 2 (*TừVôSơn TrắcPhong*) | 3962, 5994 | `0x007254C0`, `0x0072555C` |
| **Halloween / Quỷ Vương** | `61551`–`61567` kind 1–6 (*Mê cung Quỷ Vương*, 39 bản ghi) · `61051` (*Nhà cỏ*) · `61178`/*57536* (*Thôn*) · `57518`–`57536` (*Động*) | 3883, 4956–4959 | `0x007245C0`, `0x00724500` |
| **Niên Thú** | `12941` kind 20 (*Tháp Lôi Phong*) · `12406`/`12407`/`12418`/`12419` (*TừVôSơn TrắcPhong*) | 3874, 3998, 4962, 4945 | `0x0072690C`, `0x007269E0` |
| **Phật Đản** | `59000` kind 1–16 (*Phật Giới*, *Tế đàn Tiên Giới*) | — | — |
| **Đồng Tước Đài (2K)** | `12061` kind 10 | — | `0x00797A5C`, `0x00797B00` |

> ⚠ Cột `mapId` + `kind` ở trên là **cặp khóa client dùng để tra tên bản đồ**
> (`FUN_00535864`); server phải gửi **đúng cả hai** khi chuyển map.
