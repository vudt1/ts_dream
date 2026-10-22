# ĐẶC TẢ KỸ THUẬT NGHIỆP VỤ SỰ KIỆN ĐẶC BIỆT TRÊN CLIENT TS ONLINE (`aLogin.exe`)

Tài liệu này tổng hợp kết quả dịch ngược toàn diện (byte-exact) cơ chế vận hành các **Sự kiện đặc biệt (Special Events)** từ thực thi `aLogin.exe` (Delphi 7, PE x86, Image Base `0x00400000`) cùng bộ dữ liệu tĩnh đi kèm (`Scene.Dat`, `Npc.dat`, `Item.dat`, `Festival.Dat`, `Warp.Dat`, `SceneSet.Dat`, `GS.Dat`, `BlissBag.Dat`, `Rank.Dat`, và `eve.emg`). Toàn bộ phân tích chỉ dựa trên bằng chứng từ phía client và được gán nhãn độ tin cậy:
- **(A) Đã xác minh trực tiếp**: Có địa chỉ hàm/lệnh ASM, chuỗi VISCII tại Virtual Address, cấu trúc byte frame hoặc bản ghi `.dat`/`eve.emg` khớp hoàn toàn.
- **(B) Suy luận có căn cứ**: Dựa trên ngữ cảnh gọi hàm, cấu trúc đối tượng VCL/Delphi, hoặc mối liên hệ giữa biến toàn cục và hàm xử lý.
- **(C) Giả thuyết**: Phần hành vi nằm hoàn toàn ở phía máy chủ mà client chỉ nhận kết quả cuối cùng hoặc các nhánh cụ thể chưa có hàm con decompile chi tiết.

## 1. Kiến trúc khung truyền thông và Bộ điều phối (Dispatcher & Codec)

### 1.1. Định dạng khung gói tin (Frame Format) và Giải mã tĩnh (A)
- **Cấu trúc vật lý trên TCP stream**: `[F4 44][Length: u16 LE][Payload]` — toàn bộ luồng byte được phép tính XOR với khóa tĩnh `0xAD` **(A)**.
- **Nguyên tắc điều khiển sự kiện**: Máy chủ luôn giữ vai trò phát tín hiệu kích hoạt, chuyển pha và kết thúc sự kiện; client không tự ý khởi tạo bất kỳ sự kiện nào theo đồng hồ cục bộ mà chỉ phản ứng khi nhận gói tin Server→Client (S→C) hoặc khi người chơi tương tác với NPC/cổng dịch chuyển trong `eve.emg` **(A)**.
- **Bộ đọc/ghi nguyên thủy (Primitive Codec)** **(A)**:
  - `FUN_0077EB9C`: Đọc `Word` (2 byte Little-Endian, `u16 LE`) từ bộ đệm nhận.
  - `FUN_0077EF7C`: Đọc `DWORD` (4 byte Little-Endian, `u32 LE`) từ bộ đệm nhận.
  - `FUN_0077ED68`: Đọc `DWORD` biểu diễn ngày tháng `YYYYMMDD`, tách `Year = val / 10000`, `Month = (val % 10000) / 100`, `Day = val % 100` rồi gọi `EncodeDate` trả về kiểu `TDateTime` (`double` 8 byte của Delphi) **(A)**.
  - `FUN_0077EB1C` / `FUN_0077EE84`: Ghi `Word LE` (`u16`) / `DWORD LE` (`u32`) vào bộ đệm gửi.
  - `FUN_0077F098` / `FUN_0077F298`: Đóng gói và gửi frame đã thêm header `[F4 44][Length]` qua socket **(A)**.

### 1.2. Bảng 65 Main OP của Dispatcher S→C (`FUN_0078A89C`) (A)
Hàm `FUN_0078A89C` (tại `0x0078A89C`) nhận mảng byte `Payload`, tách byte đầu tiên làm `MainOP`, phần còn lại là `RP` (`Rest Payload`, trong đó `RP[0]` đóng vai trò `SubOP`). Hàm tra bảng chỉ mục 1 byte tại `0x0078A8EE` (`mainOp <= 0xC7`) để nhảy vào bảng địa chỉ 65 nhánh (`0x0078A9B6`, ứng với `case_001` đến `case_065`, và `case_000` tại `0x00796347` là nhánh mặc định thoát hàm) **(A)**.

| Case | Main OP | Địa chỉ nhánh | Đối tượng / Biến quản lý chính | Phân loại & Vai trò trong Sự kiện đặc biệt (Level A) |
| :--- | :--- | :--- | :--- | :--- |
| `case_001` | `0x00` | `0x0078AABE` | `*gvar_007D9D74`, `*gvar_007DA5A0` | **Ngắt kết nối / Lý do kết thúc sự kiện** (`sub 0x01..0x38`): báo sai 3 lần câu hỏi Bắc Tinh Quân (`0x02`, `0x2E`), sự kiện phạm luật (`0x05`, `0x06`, `0x1A`), sai cảnh sự kiện (`0x2C`), sự kiện kết thúc trước khi trận đấu xong (`0x2F`), đăng nhập máy chủ thi đấu chuyên biệt (`0x32..0x34`), kết thúc lôi đài đấu trận (`0x36` → chuỗi `0x00796CAC`), sau đó gọi `FUN_0077F414(0)` gửi C→S `0x00` |
| `case_002` | `0x01` | `0x0078B149` | `*gvar_007DA7CC`, `LocalPlayer` | Khởi tạo nhân vật / danh sách nhân vật khi đăng nhập |
| `case_003` | `0x02` | `0x0078B6FC` | `*gvar_007DA5C8`, `LocalPlayer` | Thông tin kênh chat / tin nhắn hệ thống cơ bản |
| `case_004` | `0x03` | `0x0078BC95` | `LocalPlayer` (`*gvar_007DA7BC`) | Đồng bộ tọa độ, sự xuất hiện của người chơi trên bản đồ (`FUN_00733DC0`, kiểm tra cổng thành Công Thành Chiến) |
| `case_005` | `0x04` | `0x0078C7DB` | `LocalPlayer` | Cập nhật hướng nhìn / trạng thái di chuyển hoặc ẩn hiện thực thể |
| `case_006` | `0x05` | `0x0078CA56` | `*gvar_007DA7AC`, `LocalPlayer` | Cập nhật trạng thái cưỡi ngựa / phương tiện và ngoại hình đi kèm |
| `case_007` | `0x06` | `0x0078CB99` | `*gvar_007DA5A0`, `LocalPlayer` | Di chuyển bước chân / hiệu ứng dịch chuyển của người chơi và NPC |
| `case_008` | `0x07` | `0x0078CE37` | `LocalPlayer` | Biểu cảm nhân vật (emote), hành động ngồi/đứng |
| `case_009` | `0x08` | `0x0078D125` | `LocalPlayer`, Pet | Cập nhật chỉ số thuộc tính (HP, SP, EXP, Level, Ngũ hành) |
| `case_010` | `0x09` | `0x0078D4AF` | `LocalPlayer` | Cập nhật điểm kỹ năng và trạng thái học kỹ năng |
| `case_011` | `0x0B` | `0x0078D5D1` | `*gvar_007D9CE4` (`TFightManage`) | **Chiến đấu theo lượt (Turn-based Combat) & Quan chiến (Sự kiện F)**: điều phối trận đánh (`sub 0, 1, 3..7, 9..12, 0xFA`); riêng `sub 9` gán `LocalPlayer+0x1308 = RP[1]` (số lần dùng đạn tự động công kích) |
| `case_012` | `0x0C` | `0x0078D84D` | `FUN_007994A4`, `LocalPlayer+0x63A` | **Chuyển bản đồ (Map Warp)**: cập nhật `LocalPlayer+0x63A` (`mapId`), kích hoạt kiểm tra 39 bản đồ sự kiện và hiển thị/ẩn UI chuyên biệt |
| `case_013` | `0x0D` | `0x0078DBAE` | `LocalPlayer`, Party | Quản lý tổ đội (Party: mời, gia nhập, rời đội, giải tán, nhường đội trưởng) |
| `case_014` | `0x0E` | `0x0078E01C` | `LocalPlayer` | Giao dịch trực tiếp giữa hai người chơi (Trade) |
| `case_015` | `0x0F` | `0x0078E367` | `LocalPlayer`, Pet (`+0x294`) | Quản lý võ tướng / thú cưng (bắt, thả, đổi tên, xuất chiến) |
| `case_016` | `0x10` | `0x0078E593` | `*gvar_007D9D94`, `*gvar_007DA160`, `*gvar_007DA1D0` | **Sự kiện G (Vấn đáp Bắc Tinh Quân & GM) + Đố đèn Tết Nguyên Tiêu**: `sub 01` mở bảng câu hỏi 4 đáp án; `sub 02` báo kết quả đúng/sai; `sub 03..06, 09..0C, 0E..13` lệnh GM (`TGmManage`); `sub 0D` báo cấm chức năng quân đoàn trong Công Thành Chiến; `sub 18, 19` (`24, 25`) câu đố Miếu Ông / Miếu Bà (`TWF_Day55QuestManage`) |
| `case_017` | `0x12` | `0x0078EADC` | `LocalPlayer` | Quản lý trang bị trên người (mặc/tháo vũ khí, giáp) |
| `case_018` | `0x14` | `0x0078EC3F` | `*gvar_007DA5A0`, `LocalPlayer` | **Kịch bản Eve, Hộp thoại Sự kiện, Lịch Lôi Đài & Nhị Kiều (Sự kiện A, E, H1)**: `sub 01..11` đồng bộ bước kịch bản Eve; `sub 14, 15` hiển thị lịch Lôi Đài; `sub 1F` truy tìm Ác Ma Ba Đậu Yêu; `sub 24, 26, 2D..35` thông báo lễ hội (Nguyên Tiêu, Đoan Ngọ, Thất Tịch, Trung Thu, Tứ Hại); `sub 28` báo Lôi Đài bắt đầu/kết thúc; `sub 29` báo Giải cứu Nhị Kiều ("2K"); `sub 2A` báo đăng ký đội 5 người (`FUN_005F16F4`); `sub 37` báo lịch Lôi Đài Tiên Đấu; `sub 38` kiểm tra bản đồ Từ Vô Sơn Trắc Phong (`12406..12419`) |
| `case_019` | `0x16` | `0x0078FEAF` | `LocalPlayer` | Quản lý tiền vàng và ngân hàng (Bank) |
| `case_020` | `0x17` | `0x007902FB` | `LocalPlayer`, `*gvar_007DA5A0` | **Túi đồ (Inventory) & Thông báo Lễ hội / Thủy chiến (Sự kiện B, E)**: `sub 39` (`57`) gọi `FUN_00722CD4` xử lý 74 mã thông báo lễ hội theo mùa; `sub 64` (`100`) gọi `FUN_0076C850` trang bị vật phẩm chiến thuyền (`TWaterWarBankThing`) vào `LocalPlayer+0x9E0`; `sub 65` (`101`) dọn vật phẩm chiến thuyền; `sub 6E` (`110`) báo không đủ 50 điểm tích lũy; `sub 6F` (`111`) thu hồi Nhẫn loạn đấu / Nhẫn đoàn P / Văn chương vô giới sau 1 tuần |
| `case_021` | `0x18` | `0x0079186B` | `LocalPlayer` | Cửa hàng mua bán vật phẩm với NPC (Shop) |
| `case_022` | `0x19` | `0x00791A83` | `LocalPlayer` |Hiệu ứng kỹ năng trên bản đồ / sử dụng kỹ năng ngoài trận |
| `case_023` | `0x1B` | `0x00791C8D` | `LocalPlayer` | Bẫy / vật cản hoặc hiệu ứng môi trường trên bản đồ |
| `case_024` | `0x1C` | `0x00791EA1` | `LocalPlayer` | Thuật luyện đan / chế tạo vật phẩm (Compound/Alchemy) |
| `case_025` | `0x1D` | `0x007920C1` | `LocalPlayer` | Thư tín / bồ câu đưa thư |
| `case_026` | `0x1E` | `0x007921A6` | `*gvar_007D9D44` | Mở giao diện nhà trọ / nghỉ ngơi (phối hợp với `0x1F`) |
| `case_027` | `0x1F` | `0x007922E6` | `*gvar_007D9D44` | **Nhà trọ / Hồi phục (Inn)**: kiểm tra nếu đang ở thành Công Thành Chiến (`FUN_00634F3C(..., 6)`), bản đồ Sinh Tồn (`0x2AEF = 10991`), bản đồ `0xC2F5 = 49909` hoặc `LocalPlayer+0x1458 != 0` thì miễn phí tiền trọ (`cost = 0`) |
| `case_028` | `0x20` | `0x00792488` | `LocalPlayer` | Bày bán cá nhân (Vending / Stall) |
| `case_029` | `0x21` | `0x00792D1C` | `LocalPlayer` | Hệ thống nhiệm vụ nhật ký (Quest Log) |
| `case_030` | `0x22` | `0x007931E8` | `LocalPlayer` | Hiệu ứng thời tiết / bầu trời trên bản đồ |
| `case_031` | `0x23` | `0x0079325A` | `LocalPlayer` | Danh sách bạn bè / hảo hữu (Friend List) |
| `case_032` | `0x24` | `0x0079346B` | Nhiều Form UI sự kiện | **Điều khiển mở Bảng xếp hạng & Form Sự kiện (Sự kiện A, B, D, E)**: `sub 09` nạp dữ liệu bảng xếp hạng Lôi Đài (`FUN_0053C87C`); `sub 0A` mở form xếp hạng Lôi Đài (`*gvar_007DA098`); `sub 0C` cập nhật bảng điểm Thủy Chiến (`FUN_0052E950` trên `*gvar_007DA2B4`); `sub 0D` mở bảng thuyết minh chiến trường (`*gvar_007D9ED0`); `sub 18` mở bảng xếp hạng Đua Thuyền Rồng (`*gvar_007DA1E8`); `sub 19` đọc ngày 4 byte (`FUN_0077ED68`) và mở form Sự kiện Lễ Hội `Festival.Dat` (`FUN_0051D914` trên `*gvar_007DA1EC`) |
| `case_033` | `0x25` | `0x007937C6` | `LocalPlayer` | **Thông báo toàn cục Lôi Đài (Sự kiện A)**: `sub 03` gọi `FUN_00729430(LocalPlayer, RP)` phát 10 thông báo hệ thống của các chế độ Đấu Võ Lôi Đài, Đoàn P, Loạn Đấu, Tiên Đấu, Khiêu Chiến |
| `case_034` | `0x26` | `0x00793823` | `LocalPlayer` | Quản lý danh hiệu / chức quan |
| `case_035` | `0x27` | `0x007938C3` | `*gvar_007D9C20` (`TCY_OrganManage`) | **Quân đoàn & Công Thành Chiến (Sự kiện C)**: quản lý thành viên, chức vụ, quân huy 288 byte (`sub 12`), liên minh, tuyên chiến và trạng thái sở hữu thành trì (`sub 0x01..0x3B`) |
| `case_036` | `0x28` | `0x00794391` | `LocalPlayer` | Kho quân đoàn / đóng góp quân đoàn |
| `case_037` | `0x29` | `0x007943F9` | `*gvar_007D9C70` (`TCityManage`) | **Công Thành Chiến chuyên biệt (Sự kiện C)**: `sub 04` trạng thái thành (`FUN_00635BE4`); `sub 05` dữ liệu phòng thủ/tấn công (`FUN_00637070`); `sub 06` 16 mã thông báo chiến sự & lỗi khiêu chiến (`FUN_00635E20`); `sub 09` 13 thông báo toàn server về giờ công thành, Công Thành Đại Mạc & Mật đạo Vương Thành (`FUN_006367CC`); `sub 0B` cập nhật cổng/vật kiến trúc (`FUN_006355C8`); `sub 0C` gán `LocalPlayer+0x1435 = RP[1]`; `sub 0E` xóa `LocalPlayer+0x1436 = 0`; `sub 0F` bảng 5 thành trì có/không có quân đoàn chiếm đóng (`FUN_00637760`); `sub 32` (`50`) gán `LocalPlayer+0x142C = RP[1]` và `LocalPlayer+0x1430 = GetTickCount()`; `sub 33, 34` (`51, 52`) cập nhật khí giới công thành (`FUN_006378D0`, `FUN_006379F8`) |
| `case_038` | `0x2A` | `0x00794719` | `*gvar_007DA1D8`, `*gvar_007DA0F8` | **Mini-event / Tương tác đặc biệt**: `sub 01` (`FUN_005E5A38`), `sub 02` (`FUN_005E6AC4`), `sub 03` (`FUN_00748B94`), `sub 04` (`FUN_0074EC14`) |
| `case_039` | `0x2B` | `0x00794809` | `LocalPlayer` | Hệ thống sư đồ / kết nghĩa |
| `case_040` | `0x2C` | `0x007948E5` | `LocalPlayer` | Ghép ngọc / khảm nạm lông vũ, tâm |
| `case_041` | `0x2D` | `0x00794977` | `LocalPlayer` | Thẻ nạp giờ chơi, rút thăm may mắn & hôn nhân (`sub 01..10`, cập nhật `LocalPlayer+0x1438`, `+0x1440`, `+0x1448`) |
| `case_042` | `0x2E` | `0x00794EE3` | `LocalPlayer` | Đấu giá / ký gửi vật phẩm |
| `case_043` | `0x2F` | `0x0079504F` | `LocalPlayer` | Ủy thác rời mạng / treo máy |
| `case_044` | `0x31` | `0x007950AD` | `LocalPlayer` | Chuyển sinh / Tái sinh nhân vật và võ tướng |
| `case_045` | `0x32` | `0x00795186` | `LocalPlayer` | Trang trại / nhà riêng (Home/Farm) |
| `case_046` | `0x33` | `0x0079538D` | `LocalPlayer` | Xếp hạng danh vọng / điểm công trạng |
| `case_047` | `0x34` | `0x007954B9` | `LocalPlayer` | Hệ thống bảo mật / khóa rương |
| `case_048` | `0x35` | `0x0079557D` | `LocalPlayer` | Tách / phân giải trang bị |
| `case_049` | `0x36` | `0x007955C7` | `LocalPlayer` | Nhiệm vụ truy nã / săn tiền thưởng |
| `case_050` | `0x37` | `0x0079563B` | `LocalPlayer` | Nâng cấp / cường hóa trang bị |
| `case_051` | `0x39` | `0x0079569B` | `LocalPlayer` | Hệ thống quan chức triều đình |
| `case_052` | `0x3B` | `0x00795709` | `LocalPlayer` | Đổi quà tích lũy / mã quà tặng |
| `case_053` | `0x3C` | `0x0079575C` | `*gvar_007DA778` | **Trò chơi Cờ tự mò / Sòng bạc (Minigame)**: `sub 01..04` (`FUN_00541098`, `FUN_005408F4`, `FUN_0053FD78`, `FUN_0053F8FC`; yêu cầu tối thiểu 30 lượng vàng `0x0053FBAC`) |
| `case_054` | `0x3D` | `0x007957D8` | `LocalPlayer` | Hệ thống thú cưỡi đặc biệt / xe công thành |
| `case_055` | `0x3E` | `0x00795A72` | `*gvar_007DA250` (`TLifeManage`) | **Lôi Đài Sinh Tồn / Hoạt động Ác Đảo (Sự kiện A phụ)**: `sub 01` (`FUN_0075C22C`), `sub 02` (`FUN_0075C1C0`) hiển thị bảng xếp hạng Sinh Tồn (Hạng 1/2/3) và thông báo mở cửa Ác Đảo (`0x0075C338..0x0075C707`) |
| `case_056` | `0x3F` | `0x00795AC4` | `*gvar_007D9DFC` (`TMRBatterManage`), `*gvar_007DA27C` (`TMR_WaterBattleManage`) | **Chiến trường Lôi Đài Chiến Dịch & Thủy Chiến (Sự kiện A, B, H3)**: `sub 01..0A` điều khiển chiến dịch Đại Hưng Sơn / Cự Lộc / Đổng Trác Thảo Phạt (Hổ Lao Quan); `sub 0B..16` (`11..22`) điều khiển Thủy Chiến (ghép cặp quân đoàn, đếm ngược, trạng thái tàu, thông báo bắt đầu/kết thúc) |
| `case_057` | `0x40` | `0x00795C7B` | `*gvar_007DA2FC` (`TWaterManage`) | **Tác chiến Thủy Chiến trên mặt nước (Sự kiện B)**: `sub 01` (`FUN_00527674`) xử lý bắn đạn pháo/va chạm thuyền; `sub 02` (`FUN_00607D94`) cập nhật hiệu ứng chiến thuyền; `sub 03` (`FUN_0074CDD8`) đồng bộ trạng thái tàu của người chơi |
| `case_058` | `0x41` | `0x00795CFB` | `LocalPlayer` | Hệ thống Pháp bảo / Tiên giới |
| `case_059` | `0x42` | `0x00795DB6` | `*gvar_007D9C6C` | **Cửa hàng Điểm (Point Shop) & Túi Phúc (`BlissBag.Dat`)**: `sub 01..04, 0B..0E, 15..18, 1F..20` |
| `case_060` | `0x43` | `0x00795F6C` | `LocalPlayer` | Hệ thống chuyển server / liên thông máy chủ |
| `case_061` | `0x44` | `0x00795FD4` | `LocalPlayer` | Hệ thống thẻ biến thân / danh tướng |
| `case_062` | `0x46` | `0x007960BD` | `*gvar_007DA714` (`TLH_DragonBoat`), `*gvar_007DA1E8` | **Đua Thuyền Rồng (Sự kiện D)**: `sub 01..08` điều khiển toàn bộ vòng đời đường đua (chuẩn bị, đếm ngược 5 giây, đua phím Trái/Phải, bảng xếp hạng, về đích) |
| `case_063` | `0x47` | `0x00796199` | `LocalPlayer` | Hệ thống hộ tống / vận tiêu |
| `case_064` | `0x48` | `0x00796248` | `*gvar_007D9C68` | **Vòng quay may mắn (Lucky Wheel)**: `sub 01..06` (`FUN_00507004..FUN_00508714`, chuỗi `"Hoạt động thêm số tặng Vòng quay"`) |
| `case_065` | `0xC7` | `0x007962FC` | `*gvar_007DA160` (`TGmManage`) | **Kênh liên lạc GM trực tuyến (Sự kiện G)**: `sub 04` (`FUN_00613634`) tin nhắn riêng GM↔Người chơi; `sub 06` (`FUN_0061353C`) thông báo GM kèm âm thanh `sound\WA0033.wav` |

## 2. Bản đồ Sự kiện, Kịch bản `eve.emg` và Bảng tạo gói tin C→S (Q1b, Q1c, Q2b)

### 2.1. Bảng 39 Map ID mà Client kiểm tra trực tiếp (`LocalPlayer+0x63A`) (Q1b - Level A)
Trường `*(uint16_t*)(LocalPlayer + 0x63A)` lưu `mapId` hiện tại của người chơi. Quản lý bản đồ `TKC_Scene` nạp file `\Data\Scene.Dat` qua `FUN_00535600` và tra cứu tên bản đồ bằng hàm `FUN_0053558C` **(A)**. Dưới đây là toàn bộ 39 biểu thức kiểm tra `mapId` trong code client kèm `kind` giải mã từ `Scene.Dat`:

| Map ID (Hex) | Map ID (Dec) | Tên bản đồ (`Scene.Dat`) | Phép so sánh | SubOP / Ngữ cảnh | Có Eve | Địa chỉ gọi trong `aLogin.exe` (Level A) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `0x0000` | `0` | *(Sentinel / Đặc biệt)* | `!=,==` | `` | `` | `005e8534_FUN_005e8534;005ef398_FUN_005ef398;006355c8_FUN_006355c8;00643470_FUN_00643470;0064efa0_FUN_0064efa0;00716a94_FUN_00716a94;0073bed0_FUN_0073bed0;0078a89c_FUN_0078a89c;007a3b5c_FUN_007a3b5c` |
| `0x0003` | `3` | *(Sentinel / Đặc biệt)* | `==` | `` | `` | `00735ef8_FUN_00735ef8` |
| `0x0008` | `8` | *(Sentinel / Đặc biệt)* | `==` | `` | `` | `00642c2c_FUN_00642c2c;00735134_FUN_00735134` |
| `0x000A` | `10` | *(Sentinel / Đặc biệt)* | `==` | `` | `` | `00733b94_FUN_00733b94` |
| `0x003B` | `59` | *(Sentinel / Đặc biệt)* | `==` | `` | `` | `0076d578_FUN_0076d578` |
| `0x29CD` | `10701` | *(Sentinel / Đặc biệt)* | `==` | `` | `Y` | `00532904_FUN_00532904;0055ee9c_FUN_0055ee9c;0058b5a4_FUN_0058b5a4;005aa09c_FUN_005aa09c;005aa57c_FUN_005aa57c;005aabfc_FUN_005aabfc;005be9ec_FUN_005be9ec;005c182c_FUN_005c182c;007b3474_FUN_007b3474` |
| `0x2AEF` | `10991` | **QuảngTrườngTrácQuận | Mê cung sinh tồn** | `!=,==` | `` | `` | `0060ecdc_FUN_0060ecdc;0063cf58_FUN_0063cf58;0063fadc_TFightField.Create;00642720_FUN_00642720;00642c2c_FUN_00642c2c;00643470_FUN_00643470;0064f580_FUN_0064f580;00652454_FUN_00652454;0070d43c_FUN_0070d43c;0070f7b4_FUN_0070f7b4;0075ad70_FUN_0075ad70;0075b66c_FUN_0075b66c;0076d578_FUN_0076d578;0078a89c_FUN_0078a89c;007922e6_FUN_007922e6;case_027_007922E6_FUN_007922e6;jumptable_0x78A9B6_cases` |
| `0x2AF1` | `10993` | **U Châu** | `!=` | `` | `` | `0050f4ac_TForm1.Transition` |
| `0x2AF2` | `10994` | **QuảngTrườngTrácQuận** | `==` | `` | `Y` | `00642720_FUN_00642720;00642c2c_FUN_00642c2c` |
| `0x2EE0` | `12000` | **Thôn Lâu Tang | Cửa thành Trác Quận | CửaThành NghiệpThành | Trại Lê Dương | Giới Kiều | Bia Chi Lâm | Thanh Châu | Thôn Thường Sơn | Cửa thành Cự Lộc | Cửathành Bình Nguyên | Kiệt Thạch Sơn | Lô Long Khẩu | Hồ Quan | Rừng cây vô tận | Rừng Trung Sơn | Bờ biển Hoàng Hà | Động Đại Hưng Sơn | Đường Từ Vô Sơn | Nội Hoàng thụ lâm | Rừng Tây Sơn | Rừng Hà Nội | Rừng Triều Ca | Rừng Mục Dã | Rừng Dịch Huyện | Bờ biển Ký Châu | Duyện Châu | Ký Châu | Tàn tích Bột Hải | Quân trướng | Thôn Ẩn Thế | Ghi danh Đoan Ngọ | QuảngTrường ĐoànViên** | `!=,==` | `` | `` | `0051b070_FUN_0051b070;0051b2b8_FUN_0051b2b8;005e8534_FUN_005e8534` |
| `0x2EE1` | `12001` | **U Châu | Rừng ngoại ô | Nhà Trương Phi | Vườn Đào | Lò rèn | Bần hộ | Khách sạn Duyệt Lai | Phủ Quách Dịch | Cầu Thất Tịch | Nhà thuốc | Tiền trang | Đường lớn Trác Quận | Phú hộ | NguyênTiêu nhạc viên | Chiếntrường dũng giả | Nguyên Tiêu ký vực | Tháp Lôi Phong | Rừng đào Thất Tịch** | `!=` | `` | `Y` | `005d6f7c_FUN_005d6f7c` |
| `0x2EE2` | `12002` | **Tiệm trà | Cửa thành Trác Quận | Quan phủ | Bần hộ | QuảngTrườngTrácQuận** | `==` | `` | `` | `00743814_FUN_00743814` |
| `0x2EE3` | `12003` | **Đường lớn Trác Quận | Rừng Trác Quận | Bần hộ | Khách sạn Duyệt Lai | Sòng bạc | Lôi đài thôn thực | Lôi đài 5 vs 5 | LôiĐàiHỏiĐápNămMới | Tầng 1 Tâm Ma tháp | Nhà tranh** | `!=` | `` | `Y` | `005eee60_FUN_005eee60` |
| `0x2EE5` | `12005` | **Cửa thành Trác Quận | Hội thi nướng thịt | Thôn tuyết Bắc Quốc** | `!=` | `` | `Y` | `005e8534_FUN_005e8534` |
| `0x2F1D` | `12061` | **Ký Châu | ĐườngLớn NghiệpThành | Khách sạn Thiên Vạn | Bần hộ | Lò rèn | Lôi đài tỉ võ | Khách sạn | Đồng Tước đài | Tháp thí luyện** | `!=` | `` | `Y` | `005d6f7c_FUN_005d6f7c` |
| `0x2F94` | `12180` | **QuảngTrườngTrácQuận | Chợ P-TS Online** | `==` | `` | `Y` | `00642720_FUN_00642720;00642c2c_FUN_00642c2c` |
| `0x3073` | `12403` | **Đường mòn Từ Vô Sơn | Phòng nhỏ Từ Vô Sơn | Ký Châu** | `!=` | `` | `Y` | `005e8534_FUN_005e8534` |
| `0x3075` | `12405` | **Đỉnh Từ Vô Sơn | TừVôSơn TrắcPhong** | `!=` | `` | `Y` | `005e8534_FUN_005e8534` |
| `0x3076` | `12406` | **Đường Từ Vô Sơn | TừVôSơn TrắcPhong | Hương Dữu cự tháp** | `==` | `` | `Y` | `0074e638_FUN_0074e638` |
| `0x3077` | `12407` | **TừVôSơn TrắcPhong** | `==` | `` | `Y` | `0074e82c_FUN_0074e82c` |
| `0x3082` | `12418` | **Đường Từ Vô Sơn | TừVôSơn TrắcPhong | Hương Dữu cự tháp** | `==` | `` | `Y` | `0074e638_FUN_0074e638` |
| `0x3083` | `12419` | **TừVôSơn TrắcPhong** | `==` | `` | `Y` | `0074e82c_FUN_0074e82c` |
| `0x3085` | `12421` | **U Châu | Đất tuyết Lô Long | Bộ lạc Giáng Sinh | Thôn Giáng Sinh** | `!=` | `` | `Y` | `005e8534_FUN_005e8534` |
| `0x31A7` | `12711` | **Cửa thành Trác Quận | Tiền tiêu Cự Lộc | Tiền tiêu ĐạiHưngSơn | Tiền tiêu Hổ Lao | Tiền tiêu Hồ BàDương** | `==` | `` | `Y` | `005eb530_FUN_005eb530` |
| `0x3204` | `12804` | **Bia Chi Lâm** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3216` | `12822` | **Rừng Trung Sơn** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3219` | `12825` | **Thôn Lâu Tang | Rừng Lâu Tang** | `==` | `` | `Y` | `0073d558_FUN_0073d558` |
| `0x326F` | `12911` | **Q Trường NghiệpThành** | `==` | `` | `Y` | `00642720_FUN_00642720;00642c2c_FUN_00642c2c` |
| `0x348C` | `13452` | **Chân núi Ngoạ Ngưu | Sườn núi Ngoạ Ngưu** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3612` | `13842` | **Rừng Nhương Sơn | Động Nhương Sơn** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3613` | `13843` | **Động Nhương Sơn** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3DE1` | `15841` | **Rừng Lư Giang | Từ Châu** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0x3DE2` | `15842` | *(Sentinel / Đặc biệt)* | `==` | `` | `Y` | `0070d86c_FUN_0070d86c` |
| `0x5174` | `20852` | **Rừng Tấn Dương** | `==` | `` | `` | `0070d86c_FUN_0070d86c` |
| `0xBF7B` | `49019` | **Trấn Nhu Tu | Xưởng đóng tàu | U tàu Nhu Tu | QuảngTrườngTrácQuận** | `!=` | `` | `` | `005ef398_FUN_005ef398` |
| `0xBF9B` | `49051` | **Trấn Ngưu Chữ | Ao Ngưu Chữ | Cửa vào Ngưu Chữ | Quân trướng | Binh trướng** | `!=` | `` | `Y` | `007a6590_FUN_007a6590` |
| `0xE678` | `59000` | **Thông đạo Dao Trì | Tế đàn Tiên Giới | Khu vực Thuần Tinh | Thông đạo Thiên Đình | Khu vực Thái Bạch | Thôn Tiên Giới | Nam Thiên Môn | Khu vực Mê Hoặc | Thông đạo Bàn Đào | Khu vực Tuế Tinh | Núi Linh Hư | Đảo Thiên Ngoại | Địa Thiên Nghi | Chân núi Tiên Ẩn | Long Tuyền | Phật Giới** | `!=` | `` | `Y` | `007a6590_FUN_007a6590` |
| `0xEBFB` | `60411` | **Thôn U Linh | Bình Nguyên Tử Linh** | `!=,==` | `` | `Y` | `005ef1c0_FUN_005ef1c0;007a6590_FUN_007a6590` |
| `0xFFFF` | `65535` | *(Sentinel / Đặc biệt)* | `!=` | `` | `` | `0051c2ec_FUN_0051c2ec` |

Ngoài 39 điểm kiểm tra cứng trong mã thực thi trên, bảng dữ liệu `Scene.Dat` và `eve.emg` còn định nghĩa các cụm bản đồ sự kiện trọng tâm sau **(A)**:
- **Cụm Lôi Đài & 40 NPC (Sự kiện A & H2)**:
  - `12001` (`ĐấuTrườngVõĐại` — điểm báo danh gặp NPC `15162 LôiĐàiTruyềnSư` tại tọa độ `(522, 415)`), `10852` (`LôiĐàiDựBị` — phòng chờ có 4 NPC `12349..12352 PhápSưĐoànP1..4`), `10851` (`ĐấuTrườngPK` — sàn đấu chính, `kind=2`), `11010..11013` (`ĐấuTrườngĐoànP`, `kind=2`), `11014..11022` (`ĐấuTrường1..9`, `kind=2`), `11024..11033` (`LôiĐài1..9`, `LôiĐàiDựBị`, `kind=0`), `11034..11044` (`LôiĐàiĐoànP` / `KhánĐàiĐoànP`), `11045..11046` (`LôiĐàiBắcĐẩu` / `NamĐẩu`, `kind=2`), `10991..10997` (`Đ.TrườngSinhTử` & `KhánĐài1..6`, `kind=2/0`) **(A)**.
- **Cụm Giải cứu Nhị Kiều Giang Đông — "2K" (Sự kiện H1)**:
  - Khởi đầu tại `12003` (`ĐồngTướcĐài` trong Diệp Thành, gặp NPC `15312 TảTừ` tại `(610, 290)`), vượt qua chuỗi 18 tầng tháp/trận đồ `12911..12928` (`ĐồngTướcĐài`, `T.LangTinhTrận`, `ĐoảnBinhTrận`, `CựKiếmTrận`, `TrườngThương`, `TrườngKíchTrận`, `ĐoảnChùyTrận`, `TrườngChùyTrận`, `TrườngCungTrận`, `CườngĐầuTrận`, `H.ThươngBinh`, `TửuầuĐài`, `NhạcTấuĐài`, `VũCơĐài`, `LuyệnĐanĐài`, `QuanTinhĐài`, `KimPhụngĐỉnh`, `NgọcLongĐỉnh`) **(A)**.
- **Cụm Công Thành Chiến 5 thành trì (Sự kiện C)**:
  - Thành số 1: `25000..25004` (`ThànhThọXuân`, `CổngThànhTX`, `NghiễmTươngTX`, `TrườngLangTX`, `VươngĐiệnTX`, `kind=6`).
  - Thành số 2: `25005..25009` (`T.TrườngAn`, `CổngTrườngAn`, `NghiễmTươngTA`, `TrườngLangTA`, `VươngĐiệnTA`, `kind=6`).
  - Thành số 3: `25010..25014` (`T.GiangLăng`, `CổngGiangLăng`, `NghiễmTươngGL`, `TrườngLangGL`, `VươngĐiệnGL`, `kind=6`).
  - Thành số 4: `25015..25019` (`ThànhKhúcA`, `CổngKhúcA`, `NghiễmTươngKA`, `TrườngLangKA`, `VươngĐiệnKA`, `kind=6`).
  - Thành số 5: `25020..25024` (`ThànhLạcDương`, `CổngLạcDương`, `NghiễmTươngLD`, `TrườngLangLD`, `VươngĐiệnLD`, `kind=6`).
  - Khu vực Công Thành Đại Mạc / Cao Cư Lệ: `57536..57544` (`TrạiCaoCưLệ1..3`, `CổngCaoCưLệ1..3`, `ThànhCaoCưLệ1..3`, `kind=6/0`) **(A)**.
- **Cụm Thủy Chiến & Chiến dịch Hổ Lao Quan (Sự kiện B & H3)**:
  - Thủy chiến: `49401..49444` (`ThủyChiếnTào` `49401..49422`, `ThủyChiếnTôn` `49423..49444`, `kind=0`), `49900..49910` (sông/hồ chiến thuyền, `kind=8/0`).
  - Chiến dịch Đổng Trác Thảo Phạt (Hổ Lao Quan): `15001` (`HổLaoQuan`), `15501` (`TrạiLiênMinh`), `15502` (`RừngHuỳnhDương`) **(A)**.
- **Cụm Đua Thuyền Rồng (Sự kiện D)**:
  - `60509` (`KhuThuyềnRồng` — map đăng ký và chờ xuất phát), `60510..60521` (12 chặng đường đua `ĐườngĐua1..12`), `60522` (`ĐiểmCuốiĐ.Đua` — vạch đích) **(A)**.

### 2.2. Chuỗi điều kiện `QuestStep` trong `eve.emg` cho 34 Nhiệm vụ Sự kiện (Q1c - Level A)
File `\Data\eve.emg` chứa 2.777 bản đồ kịch bản, trong đó bảng `eve_logic` lưu các quy tắc chuyển trạng thái dạng `[Danh sách điều kiện (Conditions)] → [Danh sách hành động (Results)]`. Khi người chơi nhấp vào NPC hoặc dẫm lên cổng dịch chuyển, client gửi gói tin `C→S 0x14` (`sub 0x04 / 0x06 / 0x09`) kèm chỉ số bước kịch bản để máy chủ kiểm tra `QuestStep(quest_id == step)`, vật phẩm (`HasItem`), tiền vàng (`Gold>=`), quy mô tổ đội (`PartySize==`), rồi trả về gói `S→C 0x14` để mở hội thoại (`Dialog`), trao/thu vật phẩm (`GiveItem`/`TakeItem`), kích hoạt trận đánh (`Fight`), hoặc dịch chuyển bản đồ (`Warp`) **(A)**.

Bảng dưới đây liệt kê chi tiết chuỗi `QuestStep` và phần thưởng của các nhiệm vụ sự kiện tiêu biểu đã trích xuất từ `eve_logic.tsv` và `data_event_quests.tsv` **(A)**:

| Quest ID | Tên sự kiện / Nhiệm vụ | Bản đồ (`scene_id`) | Chuỗi bước `QuestStep` & Điều kiện kích hoạt | Hành động & Vật phẩm sự kiện (`Item ID`) (Level A) |
| :--- | :--- | :--- | :--- | :--- |
| `212` | **Giải cứu Nhị Kiều Giang Đông ("2K" — Tầng 1..18)** | `12003`, `12911..12928` | `12003` (Tả Từ): `QuestStep(212==0)` + `PartySize==1/5` → `SetQuestStep(212=1)`, `Warp(12911)`; chuỗi trận đánh vượt 18 tầng: `12911` (`Fight(0)`), `12912` (`Fight(3)`), `12913` (`Fight(3)`), `12914` (`Fight(6)`), `12915` (`Fight(7)`), `12916` (`Fight(3)`), `12917` (`Fight(5)`), `12918` (`Fight(6)`), `12919` (`Fight(9)`), `12920` (`Fight(11)`), `12921` (`Fight(16)`) → `12927` (`KimPhụngĐỉnh`, Đại Kiều) & `12928` (`NgọcLongĐỉnh`, Tiểu Kiều): kiểm tra `QuestStep(212==1..6)` | Trao ngẫu nhiên theo trọng số (`Group`): `23086` (`Lông của Nhị Kiều`, +3 Thuộc tính), `26051` (`Đậu thần Nhị Kiều`), `46049` (`Chứng nhận của Đại Kiều`), `46050` (`Chứng nhận của Tiểu Kiều`), `26036` (`Linh đơn thần bí`), `26046` (`Thiên tiên ngọc lộ`), sau đó nâng `QuestStep(212)` lên `2..7` |
| `218` | **Lôi Đài Khiêu Chiến ("40 NPC")** | `12001`, `10852`, `10851` | Tại `12001` (`ĐấuTrườngVõĐại`), nói chuyện với NPC `15162` (`LôiĐàiTruyềnSư`): kiểm tra `QuestStep(218==0)` và nộp phí `Gold>=1000` (`TakeGold(1000)`) → `Warp` vào `10852` (`LôiĐàiDựBị`) → tiến vào `10851` (`ĐấuTrườngPK`) đánh chuỗi 40 trận liên tiếp | Vật phẩm liên quan: `46072` (`Vé ưu đãi lôi đài`, giá 500, giảm một nửa phí báo danh khiêu chiến lôi đài) |
| `231` | **Đua Thuyền Rồng — Đổi thưởng Khuất Nguyên** | `60509`, `60522` | Tại vạch đích `60522` (`ĐiểmCuốiĐ.Đua`), gặp Khuất Nguyên / NPC trao thưởng: kiểm tra `QuestStep(231==1)` hoặc `QuestStep(231==2)` | Đặt `SetQuestStep(231=3)`, phát âm thanh `Effect(9170)`, trao vật phẩm lễ hội Đoan Ngọ: `46076` (`Túi thơm ngũ sắc`), `46077` (`Túi thơm Khuất Nguyên`), `50019..50022` (`Trứng vịt muối`, `Bánh ú tro`, `Bánh ú miền Bắc`, `Bánh ú miền Nam`) |
| `221` | **Lễ hội Búp Bê Nhật Bản (Hina Matsuri 3/3)** | `12001`, `12061` | `12001` (`logic 86..89`): `QuestStep(221==0..2)` → `12061` (`logic 3..9`): kiểm tra vật phẩm `HasItem(46066)` (`Búp bê天皇`), `HasItem(46067)` (`Búp bê Hoàng Hậu`) và `QuestStep(221==2)` | Thu hồi `TakeItem(46066x1)`, `TakeItem(46067x1)`, đặt `SetQuestStep(221=3)` và trao thưởng qua `Group(0)` |
| `229` | **Sự kiện Kỷ niệm / Đổi Rương Báu** | `12001` | Tại `12001` (`logic 96..102`): kiểm tra `QuestStep(229==0..2)` và thu thập 3 vật phẩm sự kiện `46073`, `46074`, `46075` | Thu hồi `TakeItem(46073x1, 46074x1, 46075x1)`, đặt `SetQuestStep(229=2)`, trao `GiveItem(46080x1)` (`Rương báu kỷ niệm`) hoặc `46081` (`Túi quà mừng công`) |
| `271` | **Tết Nguyên Đán — Nhiệm vụ Năm Mới** | `12001` | Tại `12001` (`logic 105..112`): kiểm tra `QuestStep(271==0..6)` | Tại bước `QuestStep(271==5)`: đặt `SetQuestStep(271=6)`, trao `GiveItem(46221x1)` |
| `297` | **Tết Đoan Ngọ — Treo Ngải Thảo & Xương Bồ** | `12001`, `12002` | `12001` (`logic 113..117`): `QuestStep(297==0..1)` → `12002` (`logic 35..38`): kiểm tra `QuestStep(297==1..2)` và `HasItem(50096)` (`Ngải thảo`), `HasItem(50097)` (`Xương bồ`) | Thu hồi `TakeItem(50096x1)`, `TakeItem(50097x1)`, đặt `SetQuestStep(297=3)`, trao thưởng từ `Group(2..4)` (gồm `46166..46168` `Túi hương Tôn Vũ / Ngô Khởi / Nhạc Nghị`, `50098` `Hùng hoàng tửu`) |
| `306` | **Thất Tịch (7/7 Âm lịch) — Ngưu Lang Chức Nữ** | `10000`, `10817` | `10000` (`logic 0..1`): `QuestStep(306==0)` → `10817` (`logic 0..5`): kiểm tra `QuestStep(306==1..3)` | Đặt `SetQuestStep(306=3)`, trao thưởng từ `Group(0)`: `26058` (`Thất tịch tình nhân thang`), `26059` (`Thất tịch sầu tâm thang`), `46188` (`Lông vũ chim khách`) |
| `309` | **Lễ hội Mùa Thu / Đổi Vật Phẩm Sự Kiện** | `12001` | `12001` (`logic 118..126`): kiểm tra `QuestStep(309==0..2)` và `HasItem(46195)` | Thu hồi `TakeItem(46195x1)`, đặt `SetQuestStep(309=2)`, mở `Group(5)` trao `46196` / `46197` |
| `310` | **Giáng Sinh (Noel) — Ông Già Noel & Cây Thông** | `12001`, `11000`, `11001`, `11802` | `12001` (`logic 127..136`): `QuestStep(310==0..6)`; `11000` (`logic 5..8`): nộp `46200` → nhận `46198`; `11001` (`logic 8..11`): nộp `46201` → nhận `46199`; `11802` (`logic 3..5`): nộp `46202` | Quay về `12001` nộp bộ vật phẩm `46198`, `46199`, `46202`, đặt `SetQuestStep(310=6)`, trao thưởng `Group(6)` (`46061` `Cây thông Noel`, `46062` `Tất Giáng Sinh`, `20092` `Mũ Giáng Sinh`) |
| `332` | **Mừng Xuân Ất Dậu / Tết Âm Lịch** | `12003`, `21000` | `12003` (`logic 62..64`): `QuestStep(332==0)` → `21000` (`logic 44..54`): `QuestStep(332==1..6)` | Kiểm tra `HasItem(46273)`, thu hồi `TakeItem(46273x1)`, đặt `SetQuestStep(332=6)`, trao thưởng `Group(0)` (`46274`, `46275`) |
| `333` | **Săn Gà Năm Dậu (Ất Dậu)** | `12003`, `12805` | `12003` (`logic 65..67`): `QuestStep(333==0..2)` → `12805` (`logic 5..14`): đánh trận `Fight(2..5)` bắt NPC / vật phẩm `46276` | Thu hồi `TakeItem(46276x1)`, đặt `SetQuestStep(333=2)`, trao thưởng `Group(1)` (`46277..46280`) |
| `334` | **Đố Đèn Tết Nguyên Tiêu (Rằm tháng Giêng)** | `12003` | `12003` (`logic 68..83`): kiểm tra `QuestStep(334==0..5)`, thu thập nguyên liệu làm lồng đèn `46281`, `46282`, `46283` | Đổi thành lồng đèn `46284` (`QuestStep 3`), sau đó đổi thưởng cuối `Group(2..4)` tại `QuestStep(334==5)` |
| `111..126` | **Chuỗi 16 nhiệm vụ phụ trợ Đổi Vật Phẩm Lễ Hội** | `12001`, `12003`, `15001` | Các bước đơn giản `QuestStep(id==0)` → `SetQuestStep(id=1)` khi người chơi mang vật phẩm sự kiện (`46049..46285`) đến gặp NPC lễ hội | Đổi trực tiếp vật phẩm tiêu hao sự kiện (bánh Trung Thu `50045..50049`, bánh trôi Nguyên Tiêu `50009..50012`, pháo hoa, bùa chúc phúc) |

### 2.3. Bảng tổng hợp 115 Sub-OP Client→Server (`FUN_0077F414`) & Cấu trúc Byte của các Gói Sự kiện (Q2b - Level A)
Hàm `FUN_0077F414` (tại `0x0077F414`) là cổng xây dựng gói tin C→S duy nhất của client. Tham số `param_2 & 0xFF` xác định `MainOP`, còn `*(uint8_t*)(*gvar_007DA5A0 + 0xA055)` giữ `SubOP` cần gửi **(A)**. Bảng dưới đây liệt kê đầy đủ cấu trúc byte của toàn bộ các gói C→S phục vụ sự kiện đặc biệt (đối chiếu trực tiếp từ 85 hàm con builder `00781a9a..0078a42e` và `data_client_to_server.tsv`):

| Main OP | Sub OP (Dec / Hex) | Địa chỉ Builder | Độ dài Payload | Cấu trúc Byte C→S (sau `[F4 44][Len]`) | Nguồn dữ liệu đọc trên Client & Ý nghĩa (Level A) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `0x00` | *(Không có SubOP)* | `0x0077F50E` | `1 B` | `[0x00]` | Xác nhận ngắt kết nối / rời phiên khi nhận S→C `0x00` |
| `0x0B` | `1` (`0x01`) | `0x00780744` | `2 B` | `[0x0B, 0x01]` | Báo kết thúc diễn hoạt hiệp đấu (Animation Ack) |
| `0x0B` | `2` (`0x02`) | `0x007807AA` | `12 B` | `[0x0B, 0x02, srcR:u8, srcC:u8, dstR:u8, dstC:u8, skillId:u16 LE, itemSlot:u16 LE, unk:u16 LE]` | Gửi lệnh chiến đấu của nhân vật/võ tướng trong mỗi hiệp (`TFightManage`) |
| `0x0B` | `4` (`0x04`) | `0x00780A44` | `3 B` | `[0x0B, 0x04, flag:u8]` | Xin rút lui / chạy trốn khỏi trận đấu hoặc rời chế độ Quan chiến (Sự kiện F) |
| `0x0B` | `5` (`0x05`) | `0x00780AC8` | `8 B` | `[0x0B, 0x05, targetId:u32 LE, mode:u8, flag:u8]` | **Khởi tạo PK / Quan chiến (Sự kiện F)**: gửi ID mục tiêu cần thách đấu hoặc vào xem trận đánh (`mode` lấy từ trạng thái con trỏ PK/Watch) |
| `0x10` | `1` (`0x01`) | `0x00781A9A` | `6 B` | `[0x10, 0x01, answerIdx:u32 LE]` | **Gửi đáp án Vấn đáp Bắc Tinh Quân (Sự kiện G)**: đọc `*(u32*)(*gvar_007D9D94 + 0x13C)` (chỉ số đáp án 1..4 do người chơi chọn) |
| `0x10` | `2..9` (`0x02..0x09`) | `0x00781B21`..`0x00781F41` | `6 B` | `[0x10, sub:u8, value:u32 LE]` | Nhóm lệnh quản trị/kiểm tra trạng thái đọc DWORD từ `*gvar_007DA5C8` |
| `0x10` | `10` (`0x0A`) | `0x00781FC5` | `2 + L B` | `[0x10, 0x0A, asciiText:bytes]` | Gửi chuỗi văn bản từ ô nhập `*gvar_007DA110 + 0x354` (công cụ GM) |
| `0x10` | `11..32` (`0x0B..0x20`) | `0x00782045`..`0x00782BD4` | `2..N B` | `[0x10, sub:u8, ...]` | Nhóm 22 lệnh bảng điều khiển GM (`*gvar_007DA110`: tra cứu, dịch chuyển, khóa tài khoản, phát thông báo) |
| `0x10` | `33` (`0x21`) | `0x00782C6F` | `6 B` | `[0x10, 0x21, choice:u32 LE]` | Trả lời hộp thoại lựa chọn `*gvar_007DA080 + 0x13C` |
| `0x10` | `34` (`0x22`) | `0x00782CF6` | `6 B` | `[0x10, 0x22, choice:u32 LE]` | Trả lời hộp thoại lựa chọn `*gvar_007DA678 + 0x13C` |
| `0x10` | `35` (`0x23`) | `0x00782D7D` | `6 B` | `[0x10, 0x23, choice:u32 LE]` | Trả lời hộp thoại lựa chọn `*gvar_007DA60C + 0x13C` |
| `0x10` | `36` (`0x24`) | `0x00782E04` | `6 B` | `[0x10, 0x24, choice:u32 LE]` | **Đố đèn Miếu Ông Tết Nguyên Tiêu (Sự kiện E)**: đọc `*(u32*)(*gvar_007D9F00 + 0x13C)` (`1` = Tiếp tục thách thức câu đố, `2` = Lãnh phần thưởng trực tiếp) |
| `0x14` | `4` (`0x04`) | `0x007830FC` | `4 B` | `[0x14, dynSub:u8, warpOrStepId:u16 LE]` | **Dẫm cổng dịch chuyển / Kích hoạt bước Eve**: nếu `mapId` thuộc `49401..49444` (Thủy chiến) thì gọi `FUN_0074B330(LocalPlayer, 3)` trước; byte thứ 2 bị ghi đè bởi `*(u8*)(*gvar_007DA5A0 + 0xA08E)` và 2 byte cuối là `*(u16*)(*gvar_007DA5A0 + 0xA08A)` |
| `0x14` | `6` (`0x06`) | `0x007831EE` | `2 B` | `[0x14, 0x06]` | **Đóng hộp thoại / Xác nhận bước kịch bản Eve**: báo cho server biết người chơi đã bấm qua câu thoại hiện tại |
| `0x14` | `9` (`0x09`) | `0x00783251` | `3 B` | `[0x14, 0x09, optionIdx:u8]` | **Chọn nhánh trả lời trong hộp thoại NPC (`eve_surface`)**: gửi `*(u8*)(*gvar_007DA5A0 + 0xA104)` (lựa chọn 1, 2, 3... khi báo danh Lôi Đài, Nhị Kiều, Đua Thuyền Rồng, Lễ hội) |
| `0x27` | `1..19, 31..32, 50..56` | `0x00787446`..`0x0078818B` | `2..290 B` | `[0x27, sub:u8, ...]` | **Hệ thống Quân đoàn & Công Thành (28 sub-op)**: gồm tạo quân đoàn (`sub 1`), mời/duyệt/trục xuất (`sub 2, 3, 7, 10..14`), tải lên quân huy `0x120 = 288 byte` (`sub 18` / `0x12`), hẹn giờ/tuyên chiến công thành (`sub 50..56` đọc `*gvar_007D9ED4` và `*gvar_007D9EFC`) |
| `0x29` | `1` (`0x01`) | `0x007882AC` | `3 B` | `[0x29, 0x01, cityAction:u8]` | **Công Thành Chiến (Sự kiện C)**: gửi `*(u8*)(*gvar_007D9D34 + 0x7B)` (thao tác quản lý thành trì / cổng thành) |
| `0x29` | `2` (`0x02`) | `0x0078832D` | `2 B` | `[0x29, 0x02]` | **Công Thành Chiến**: yêu cầu truy vấn trạng thái thành trì hiện tại |
| `0x29` | `3` (`0x03`) | `0x00788382` | `3 B` | `[0x29, 0x03, val:u8]` | **Công Thành Chiến**: gửi `*(u8*)(*gvar_007DA1C4 + 0x144)` (thiết lập thuế suất / điều phối phòng thủ thành) |
| `0x29` | `4` (`0x04`) | `0x00788406` | `3 B` | `[0x29, 0x04, val:u8]` | **Công Thành Chiến**: gửi `*(u8*)(*gvar_007DA188 + 0x154)` (chọn phương án nâng cấp / khí giới công thành) |
| `0x2A` | `1` (`0x01`) | `0x0078849E` | `5 B` | `[0x2A, 0x01, id:u16 LE, flag:u8]` | Gửi lựa chọn từ form sự kiện `*gvar_007DA1D8` (`+0x144` u16, `+0x146` u8) |
| `0x2A` | `2` (`0x02`) | `0x0078854E` | `4 B` | `[0x2A, 0x02, b1:u8, b2:u8]` | Gửi lựa chọn từ form sự kiện `*gvar_007DA0F8` (`+0x144` u8, `+0x146` u8) |
| `0x3C` | `1` (`0x01`) | `0x007891A6` | `2 B` | `[0x3C, 0x01]` | Gia nhập bàn chơi Cờ tự mò (`*gvar_007DA778`) |
| `0x3C` | `2` (`0x02`) | `0x00789205` | `3 B` | `[0x3C, 0x02, slot:u8]` | Chọn quân cờ / nước đi tại `*(u8*)(*gvar_007DA778 + 0x208)` |
| `0x3C` | `3` (`0x03`) | `0x00789282` | `2 B` | `[0x3C, 0x03]` | Rời bàn chơi Cờ tự mò |
| `0x3F` | `1` (`0x01`) | `0x00789557` | `4 B` | `[0x3F, 0x01, campaignId:u16 LE]` | **Báo danh Chiến trường Chiến dịch (Sự kiện B / H3)**: gửi `*(u16*)(*gvar_007D9ED0 + 0xDE76)` (mã kịch bản Đại Hưng Sơn / Cự Lộc / Đổng Trác Thảo Phạt từ bảng thuyết minh) |
| `0x3F` | `2` (`0x02`) | `0x007895DB` | `7 B` | `[0x3F, 0x02, mode:u8, playerId:u32 LE]` | **Báo danh / Thao tác Lôi Đài Chiến Dịch**: gửi `*(u8*)(*gvar_007DA7E0 + 0x150)` và `*(u32*)(LocalPlayer + 0x04)` (ID nhân vật) |
| `0x3F` | `3` (`0x03`) | `0x0078968A` | `3 B` | `[0x3F, 0x03, arenaIdx:u8]` | **Chọn khu vực Thủy Chiến ("Cuộc chiến Hồ Bà Dương" — Sự kiện B)**: gửi `*(u8*)(*gvar_007D9F10 + 0x15D)` |
| `0x40` | `1` (`0x01`) | `0x00789722` | `9 B` | `[0x40, 0x01, actionType:u8, targetId:u32 LE, param:u16 LE]` | **Khai hỏa / Tấn công trên mặt nước (Sự kiện B)**: đọc từ `LocalPlayer + 0x4D8` (1B), `LocalPlayer + 0x4D9` (4B `targetId`), `LocalPlayer + 0x4DD` (2B `skill/ammo`) |
| `0x40` | `2` (`0x02`) | `0x007897FF` | `2 B` | `[0x40, 0x02]` | **Thủy Chiến**: xác nhận đồng bộ trạng thái chiến thuyền / kết thúc hiệu ứng bắn |
| `0x46` | `1` (`0x01`) | `0x00789F85` | `2 B` | `[0x46, 0x01]` | **Đua Thuyền Rồng (Sự kiện D)**: phát tín hiệu tăng tốc mỗi khi người chơi hoàn thành 1 cặp phím mũi tên Trái → Phải luân phiên trong `FUN_0051BF04` |
| `0x48` | `1` (`0x01`) | `0x0078A225` | `2 B` | `[0x48, 0x01]` | Yêu cầu bắt đầu quay Vòng quay may mắn (`*gvar_007D9C68`) |
| `0x48` | `2` (`0x02`) | `0x0078A27A` | `3 B` | `[0x48, 0x02, betCount:u8]` | Đặt số lượt/mức cược Vòng quay may mắn tại `*(u8*)(*gvar_007D9C68 + 0x1BE)` |
| `0xC7` | `4` (`0x04`) | `0x0078A32C` | `10 + L B` | `[0xC7, 0x04, selfId:u32 LE, targetId:u32 LE, text:bytes]` | **Kênh GM (Sự kiện G)**: gửi tin nhắn hỗ trợ trực tiếp từ người chơi/GM kèm `LocalPlayer+0x04` và `targetId` |
| `0xC7` | `6` (`0x06`) | `0x0078A42E` | `6 + L B` | `[0xC7, 0x06, selfId:u32 LE, text:bytes]` | **Kênh GM (Sự kiện G)**: phát thông báo / phản hồi trên kênh GM |

*(Lưu ý: Trong hàm `FUN_0077F414`, các nhánh Main OP không nằm trong bảng 115 dòng của `data_client_to_server.tsv` chỉ nhảy về đoạn kết hàm epilogue mà không ghi byte nào vào buffer gửi)* **(A)**.

## 3. Phân tích Chuyên sâu Từng Sự kiện Đặc biệt (A → H) (Q2, Q3, Q4, Q7)

### 3.1. Sự kiện A: Lôi Đài / Đấu Trường Võ Đại (Arena & Survival Arena)

#### a) Các chế độ thi đấu và Bảng ánh xạ nhãn (`FUN_0053BAB8`) (Level A)
Quản lý lôi đài `TMRBatterManage` (VMT tại `0x0054C230`, biến toàn cục `*gvar_007D9DFC`) phối hợp cùng bảng xếp hạng lôi đài `*gvar_007DA098` (`FUN_0053BAB8`, `FUN_0053C87C`) chia hoạt động Lôi Đài thành các nhóm chế độ rõ rệt **(A)**:
1. **4 Chế độ Lôi Đài Tranh Đấu theo cấp độ (`mode_id = 0x60..0x63`)** + **4 hậu tố hình thức thi đấu (`suffix_flag = 1..4`)** trong hàm `FUN_0053BAB8` (tại `0x0053BAB8`) **(A)**:

| `mode_id` (Hex / Dec) | Tiền tố nhãn hiển thị (VA) | `suffix_flag = 1` (`0x0053BDB4`) | `suffix_flag = 2` (`0x0053BDC4`) | `suffix_flag = 3` (`0x0053BDD4`) | `suffix_flag = 4` (`0x0053BDE8`) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `0x60` (`96`) | `[Lôi đài tranh đấu 4]` (`0x0053BD40`) | `Tỉ thí` | `Đòan P` (Đoàn P) | `Lọan đấu` (Solo PK) | `Đấu trận cấp 100` |
| `0x61` (`97`) | `[Lôi đài tranh đấu 3]` (`0x0053BD60`) | `Tỉ thí` | `Đòan P` (Đoàn P) | `Lọan đấu` (Solo PK) | `Đấu trận cấp 100` |
| `0x62` (`98`) | `[Lôi đài tranh đấu 2]` (`0x0053BD80`) | `Tỉ thí` | `Đòan P` (Đoàn P) | `Lọan đấu` (Solo PK) | `Đấu trận cấp 100` |
| `0x63` (`99`) | `[Lôi đài tranh đấu 1]` (`0x0053BDA0`) | `Tỉ thí` | `Đòan P` (Đoàn P) | `Lọan đấu` (Solo PK) | `Đấu trận cấp 100` |

2. **5 Chế độ thông báo hệ thống toàn máy chủ (`S→C 25 03` → `FUN_00729430`)** **(A)**:
   - **Đấu võ lôi đài** (Tỉ thí chuẩn): bắt đầu / kết thúc (`0x00729590`, `0x00729628`).
   - **Lôi đài đoàn P** (Đấu nhóm tổ đội 5 người): bắt đầu / kết thúc (`0x00729674`, `0x00729710`).
   - **Lôi đài loạn đấu** (Solo PK hỗn chiến): bắt đầu / kết thúc (`0x0072975C`, `0x007297F0`).
   - **Lôi đài tiên đấu** (Đấu giới tiên / chuyển sinh): bắt đầu / kết thúc (`0x0072983C`, `0x007298CC`).
   - **Lôi đài thiêu chiến** (Khiêu chiến vượt ải 40 NPC — xem Sự kiện H2): bắt đầu / kết thúc (`0x00729914`, `0x00729998`).
3. **Lôi Đài Sinh Tồn / Ác Đảo (`TLifeManage`, `*gvar_007DA250`, `S→C 0x3E`)** **(A)**:
   - Diễn ra trên cụm bản đồ `10991` (`Đ.TrườngSinhTử`, `kind=2`) và `10992..10997` (`KhánĐài1..6`) **(A)**.

#### b) Lịch thi đấu, Phí báo danh, Ngũ hành và Phần thưởng (Q3 - Level A)
Từ các nhánh `S→C 14 14`, `14 15`, `14 37` (`case_018`) và chuỗi tại `0x00797210..0x00797E94`, `0x0072541C`, lịch biểu và quy tắc được ghi cứng trong client như sau **(A)**:
- **Lịch thi đấu cố định trong tuần**:
  - **Đấu võ tỉ thí (`0x00797210`)**: Chủ Nhật, Thứ Tư, Thứ Sáu từ `7:00` đến `9:00` tối, và Thứ Bảy từ `2:00` đến `4:00` chiều.
  - **Lôi đài Đoàn P (`0x0079727C`)**: Tối Chủ Nhật từ `7:00` đến `9:00` tối (tổ đội đủ 5 người mới làm thủ tục, kiểm tra tại `0x005F1A74`).
  - **Lôi đài Loạn đấu (`0x007972D8`)**: Tối Thứ Tư từ `7:00` đến `9:00` tối.
  - **Lôi đài Thứ Sáu (`0x0079731C`)**: Tối Thứ Sáu từ `7:00` đến `9:00` tối.
  - **Lôi đài Tiên Đấu (`0x00797E94`)**: Thứ Hai hằng tuần từ `7:00` đến `9:00` tối.
- **Lệ phí báo danh / Tiền cược & Ngũ hành** **(A)**:
  - Khi báo danh qua hội thoại NPC Lôi Đài Truyền Sư (`15162`), nếu không đúng khung giờ, server gửi mã thông báo lễ hội/sự kiện (`0x0072541C`): *"Bây giờ vẫn chưa đến giờ mở cửa lôi đài, bạn có đồng ý bỏ ra 5000 (10000) lượng vàng để mở lôi đài không?"* **(A)**.
  - Riêng Lôi Đài Khiêu Chiến (`Quest 218` trong `eve_logic.tsv` tại map `12001`), điều kiện kịch bản thu `TakeGold(1000)` (hoặc giảm nửa giá nếu có vật phẩm `46072 Vé ưu đãi lôi đài`) **(A)**.
  - Khi đạt chuỗi thắng hoặc chọn phần thưởng theo hệ Ngũ hành (Địa, Thủy, Hỏa, Phong, Tâm), server trả mã qua `FUN_00722CD4` (`S→C 17 39` các case `0x0B..0x0E` / `0x00723474..0x007236BC`) hiển thị thông báo nhận vật phẩm thuộc tính tương ứng **(A)**.
- **Phần thưởng xếp hạng & Thu hồi vật phẩm sau 1 tuần** **(A)**:
  - Phần thưởng danh hiệu/trang bị đặc biệt gồm: `Nhẫn loạn đấu` (`0x0076CCDC`), `Nhẫn đoàn P` (`0x0076CD28`), và `Văn chương vô giới` (`0x0076CD74`).
  - Khi hết hạn 1 tuần giữ giải hoặc không bảo vệ được thứ hạng, server gửi gói `S→C 17 6F` (`sub = 111`, hàm `FUN_0076CC38`):
    - `[17, 6F, 01]` → *"Vì thời gian bạn giữ nhẫn loạn đấu đã vượt quá 1 tuần, nên bị thu hồi."* (`0x0076CCDC`).
    - `[17, 6F, 02]` → *"Vì thời gian bạn giữ nhẫn đoàn P đã vượt quá 1 tuần, nên bị thu hồi."* (`0x0076CD28`).
    - `[17, 6F, 03]` → *"Vì bạn trong đại hội tỷ võ lần này chưa nhận được giải nhất, văn chương vô giới đã bị thu hồi."* (`0x0076CD74`).
  - Nếu người chơi đổi thưởng khi điểm tích lũy dưới 50, server gửi `S→C 17 6E` (`sub = 110`, hàm `FUN_0076C5CC`): *"Ít nhất phải đạt được 50 điểm trở lên mới có thể đổi giải thưởng."* (`0x0076C638`) **(A)**.

#### c) Cấu trúc Gói tin S→C của Sự kiện A (Q2, Q4 - Level A)
1. **`S→C 25 03 [noticeCode:u8]` (`FUN_00729430` tại `0x00729430`) — 10 thông báo toàn cục Lôi Đài** **(A)**:
   - `RP[1]` (`noticeCode` từ `1..10`) quyết định chuỗi thông báo đỏ trên kênh hệ thống:
     - `1`: *"Hoạt động đấu võ lôi đài đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* (`0x00729590`)
     - `2`: *"Hoạt động đấu võ lôi đài đã kết thúc!"* (`0x00729628`)
     - `3`: *"Hoạt động lôi đài đoàn P đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* (`0x00729674`)
     - `4`: *"Hoạt động lôi đài đoàn P đã kết thúc!"* (`0x00729710`)
     - `5`: *"Hoạt động lôi đài loạn đấu đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* (`0x0072975C`)
     - `6`: *"Hoạt động lôi đài loạn đấu đã kết thúc!"* (`0x007297F0`)
     - `7`: *"Hoạt động lôi đài tiên đấu đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* (`0x0072983C`)
     - `8`: *"Hoạt động lôi đài tiên đấu đã kết thúc!"* (`0x007298CC`)
     - `9`: *"Hoạt động lôi đài thiêu chiến đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* (`0x00729914`)
     - `10`: *"Hoạt động lôi đài thiêu chiến đã kết thúc!"* (`0x00729998`)
2. **`S→C 14 28 [state:u8]` (`case_018` tại `0x0078F89B`)** **(A)**:
   - `RP[1] == 1`: Hiển thị `"Trận đấu lôi đài đã bắt đầu!"` (`0x007979E0`).
   - `RP[1] == 2`: Hiển thị `"Trận đấu lôi đài đã kết thúc!"` (`0x00797A08`).
3. **`S→C 24 09 [mode:u8][entries...]` (`FUN_0053C87C` tại `0x0053C87C`) & `S→C 24 0A` — Bảng xếp hạng Lôi Đài (`*gvar_007DA098`)** **(A)**:
   - `S→C 24 09` truyền danh sách người chơi đạt thứ hạng cao trong lôi đài. Hàm `FUN_0053C87C` đọc từng bản ghi trong `RP`, lưu vào mảng cấu trúc 7 byte tại `self + 0x164` (`[mode_id:u8, player_id:u32 LE, score_or_rank:u16 LE]`), sau đó `FUN_0053BAB8` đối chiếu `mode_id` (`0x60..0x63`) để kết xuất tên người chơi, điểm số và nhãn `[Lôi đài tranh đấu 1..4]` **(A)**.
   - `S→C 24 0A` bật hiển thị cửa sổ xếp hạng `*gvar_007DA098` (`+0x48 = 1`) **(A)**.
4. **`S→C 3E 01` (`FUN_0075C22C`) & `S→C 3E 02` (`FUN_0075C1C0`) — Lôi Đài Sinh Tồn / Ác Đảo (`TLifeManage` `*gvar_007DA250`)** **(A)**:
   - `3E 01`: Nhận bảng điểm Sinh Tồn (Hạng 1, Hạng 2, Hạng 3 kèm phần thưởng tại `0x0075C4A0..0x0075C6E8`) và cập nhật giao diện `TLifeManage`.
   - `3E 02 [code:u8]`: Hiển thị thông báo trạng thái Sinh Tồn / Ác Đảo:
     - *"Hoạt động ác đảo kịch liệt đã mở ra, hoan nghênh các vị anh hùng đến thử sức!"* (`0x0075C338`).
     - *"Tạm thời không thể vào Lôi Đài Sinh Tồn."* (`0x0075C438`).
5. **`S→C 3F 01..0A` (`TMRBatterManage` `*gvar_007D9DFC`) — Chiến dịch Lôi Đài Liên Minh** **(A)**:
   - Điều phối trạng thái đăng ký (`3F 01`, `3F 02` → `FUN_0054CC0C`, `FUN_0054CF38`), đếm ngược thời gian, bảng điểm chiến dịch (`3F 03..0A` → `FUN_0054CB58`, `FUN_0054CD24`, `FUN_0054C3AC`).

---

### 3.2. Sự kiện B & H3: Thủy Chiến (Ship War) & 3 Kịch bản Chiến Trường ("Hổ Lao Quan" / Đổng Trác Thảo Phạt, Đại Hưng Sơn, Cự Lộc)

#### a) Cơ chế hợp nhất giữa Chiến trường Lịch sử (H3) và Thủy Chiến (B) trên Client (Level A)
Trên client `aLogin.exe`, opcode `0x3F` (`case_056`) chia làm hai nửa quản lý liên kết chặt chẽ với nhau **(A)**:
- **Nửa đầu (`3F 01..0A`)**: Do `TMRBatterManage` (`*gvar_007D9DFC`, VMT `0x0054C230`) và bảng thuyết minh chiến dịch `*gvar_007D9ED0` (`FUN_005579F4`) đảm nhiệm, phục vụ **3 Kịch bản Chiến trường lớn** (mà cộng đồng gọi chung là sự kiện **"Hổ Lao Quan"** do kịch bản số 3 đánh Đổng Trác tại Hổ Lao Quan `map 15001` là nổi tiếng nhất) **(A)**:
  1. **Trận chiến Đại Hưng Sơn** (`0x00557E64`, `0x0055829C`): Phe Triều Đình đấu Phe Khăn Vàng (Trình Viễn Chí, Đặng Mậu).
  2. **Trận chiến Cự Lộc** (`0x00558558`): Chiến dịch công phá Trương Giác / Trương Bảo / Trương Lương.
  3. **Trận chiến Đổng Trác Thảo Phạt (Hổ Lao Quan)** (`0x005587F8`, `0x005590AC`): Gồm chuỗi nhiệm vụ chiến trường tại Hổ Lao Quan (`15001`), Trại Liên Minh (`15501`), Rừng Huỳnh Dương (`15502`) và nhánh đường thủy **"Trừ diệt Nghiêm Bạch Hổ" / "Chiến thuyền Tôn Sách"** (bảo vệ chiến thuyền chủ lực của Tôn Sách không bị đánh chìm) **(A)**.
- **Nửa sau (`3F 0B..16`) & Opcode `0x40`**: Do `TMR_WaterBattleManage` (`*gvar_007DA27C`, VMT `0x0054A318`), `TMR_WBManageOrgManage` (`*gvar_007D9F10`, form *"Cuộc chiến Hồ Bà Dương"*) và `TWaterManage` (`*gvar_007DA2FC`, VMT `0x00526448`) quản lý, phục vụ **Hoạt động Thủy Chiến (Ship War)** trên cụm bản đồ `49401..49444` (`ThủyChiếnTào`, `ThủyChiếnTôn`) và `49900..49910` (`kind=8`) **(A)**.

#### b) Cấu trúc Gói tin S→C & C→S của Thủy Chiến và Chiến dịch (Q2, Q4 - Level A)
1. **Nhóm gói tin `S→C 0x3F` (`case_056` tại `0x00795AC4`)** **(A)**:
   - `3F 01` (`FUN_0054CC0C`) & `3F 02` (`FUN_0054CF38`): Cập nhật danh sách phe phái, số lượng người tham gia và trạng thái báo danh chiến dịch (`TMRBatterManage`).
   - `3F 03` (`FUN_0054CB58`), `3F 04` (`FUN_0054CAEC`), `3F 05` (`FUN_0054CA58`), `3F 06` (`FUN_0054C998`), `3F 07` (`FUN_0054CD24`), `3F 08` (`FUN_0054CEC0`), `3F 09` (`FUN_0054CDF4`), `3F 0A` (`FUN_0054C3AC`): Đồng bộ tiến độ mục tiêu chiến trường (thời gian còn lại, điểm tích lũy hai phe, thông báo hoàn thành/thất bại mục tiêu như *"Chiến thuyền Tôn Sách bị phá hủy"*).
   - `3F 0B` (`11`, `FUN_0054AE20`): Cập nhật bảng ghép cặp thi đấu giữa các Quân đoàn trong Thủy Chiến (`TMR_WaterBattleManage`).
   - `3F 0C` (`12`, `FUN_0054A758`): Phát thông báo trạng thái hoạt động Thủy Chiến theo mã `RP[1]` **(A)**:
     - *"Hoạt động thủy chiến sắp bắt đầu, mời các quân đoàn chuẩn bị!"* (`0x0054A910`).
     - *"Hoạt động thủy chiến bắt đầu rồi!"* (`0x0054A96C`).
     - *"Hoạt động thủy chiến đã kết thúc!"* (`0x0054A9CC`).
   - `3F 0D` (`13`, `FUN_0054AD48`): Đồng bộ thời gian đếm ngược và điểm số hai bên (Ngụy / Ngô hoặc hai quân đoàn) trên HUD Thủy Chiến.
   - `3F 0E` (`14`, `FUN_0054A5FC`) & `3F 0F` (`15`, `FUN_0054A670`): Cập nhật độ bền (HP), năng lượng (SP), số lượng đạn pháo và trạng thái thuyền của đội.
   - `3F 10..13` (`16..19`): Các nhánh dự phòng trong `case_056` (nhảy thẳng tới epilogue `0x00795C6E`, client không xử lý) **(A)**.
   - `3F 14` (`20`, `FUN_0054B2E8`), `3F 15` (`21`, `FUN_0054B2A0`), `3F 16` (`22`, `FUN_0054ADAC`): Mở/cập nhật giao diện chọn khu vực *"Cuộc chiến Hồ Bà Dương"* (`*gvar_007D9F10`) và bảng tổng kết trận Thủy Chiến.
2. **Nhóm gói tin `S→C 0x40` (`case_057` tại `0x00795C7B`, `TWaterManage` `*gvar_007DA2FC`)** **(A)**:
   - `40 01` (`FUN_00527674`): Nhận sự kiện chiến đấu trên mặt nước (tàu bắn đạn pháo, trúng đạn, nổ thuyền hoặc hồi phục trên sông).
   - `40 02` (`FUN_00607D94`): Cập nhật hiệu ứng hình ảnh/trạng thái của các thực thể thuyền xung quanh.
   - `40 03` (`FUN_0074CDD8`): Đồng bộ thông số chiến thuyền của `LocalPlayer` (tốc độ chèo, giáp tàu, vũ khí tàu).
3. **Gói tin trang bị Chiến thuyền `S→C 17 64` (`sub = 100`, `FUN_0076C850`) & `17 65` (`sub = 101`, `FUN_0076CA80`)** **(A)**:
   - Khi vào khu vực Thủy Chiến, server gửi `17 64` chứa danh sách vật phẩm chuyên dụng của tàu (`TWaterWarBankThing`, kích thước mỗi phần tử `0x16 = 22 byte`) để nạp vào mảng kho tàu tại `LocalPlayer + 0x9E0` (tối đa 25 ô, đánh chỉ số `1..25`) **(A)**.
   - Khi rời Thủy Chiến, `17 65` (`FUN_0076CA80`) giải phóng và xóa sạch các ô vật phẩm chiến thuyền này **(A)**.
4. **Hành vi riêng trên Bản đồ Thủy Chiến (`mapId` `49401..49444` & `49900..49910`)** **(A)**:
   - Hàm `FUN_0073A3D8` kiểm tra nếu `mapId - 0xC2EC < 0xB` (`49900..49910`) thì tự động vẽ hình ảnh con thuyền dưới chân người chơi kèm thanh máu tàu và tên quân đoàn **(A)**.
   - Hàm `FUN_005F54E0` kiểm tra khi dẫm lên điểm dịch chuyển trong dải `49401..49444`: nếu là bản đồ chẵn (boong tàu) mà người chơi đang trong tổ đội (`PartySize > 1`) thì chặn dịch chuyển và hiện thông báo *"Trên chiến thuyền không thể tổ đội đi qua cửa trục!"* (`0x005F5644`) **(A)**.

---

### 3.3. Sự kiện C: Công Thành Chiến / Chiến Công Kích Thành Trì (City Siege / Guild War)

#### a) Danh sách 5 Thành trì & Khu vực Công Thành Đại Mạc (Level A)
Quản lý công thành `TCityManage` (`*gvar_007D9C70`, các hàm `00634f3c..006379f8`) và quản lý quân đoàn `TCY_OrganManage` (`*gvar_007D9C20`) điều khiển 5 thành trì chính (có `kind = 6` trong `Scene.Dat`, kiểm tra qua `FUN_00634F3C(mapId, 6)`) cùng chiến trường Công Thành Đại Mạc **(A)**:
1. **Thành Thọ Xuân** (`25000..25004`).
2. **Thành Trường An** (`25005..25009`).
3. **Thành Giang Lăng** (`25010..25014`).
4. **Thành Khúc A** (`25015..25019`).
5. **Thành Lạc Dương** (`25020..25024`).
6. **Chiến trường Công Thành Đại Mạc / Cao Cư Lệ** (`57536..57544`).

#### b) Lịch trình, Luật chơi và Chuỗi thông báo (`FUN_006367CC` & `FUN_00635E20`) (Q2, Q3 - Level A)
1. **`S→C 29 09 [code:u8, ...]` (`FUN_006367CC` tại `0x006367CC`) — 13 mã thông báo toàn cục Công Thành Chiến** **(A)**:
   - Báo trước giờ mở chiến dịch: *"Cuộc chiến công thành sẽ chính thức bắt đầu vào 8 giờ tối, xin các quân đoàn chuẩn bị sẵn sàng!"* (`0x00636B88`).
   - Khi bắt đầu (20:00): *"Thời gian công thành đã bắt đầu!"* (`0x00636BF4`).
   - Khi kết thúc: *"Thời gian công thành hôm nay đã kết thúc!"* (`0x00636C20`).
   - Công Thành Đại Mạc: *"Trận chiến công thành Đại Mạc đã bắt đầu!"* (`0x00636C54`) / *"Trận chiến công thành Đại Mạc đã kết thúc!"* (`0x00636C88`).
   - Khi cổng thành hoặc tượng chính bị công phá / đổi chủ: thông báo tên quân đoàn chiếm giữ thành và mở/đóng **Mật đạo Vương Thành** (`0x00636CC0..0x00636F90`).
2. **`S→C 29 06 [errCode:u8]` (`FUN_00635E20` tại `0x00635E20`) — 16 mã thông báo chiến thuật & điều kiện khiêu chiến** **(A)**:
   - Kiểm tra điều kiện tuyên chiến/công thành: yêu cầu quân đoàn cấp độ đủ, đủ ngân quỹ quân đoàn, không được tuyên chiến ngoài khung giờ quy định, hoặc thông báo kết quả sử dụng khí giới công thành (Xe bắn đá, Thang mây, Xe đục thành).
3. **`S→C 29 0F [5_city_records...]` (`FUN_00637760` tại `0x00637760`) — Bảng trạng thái chiếm đóng 5 thành trì** **(A)**:
   - Đọc trạng thái của 5 thành trì và hiển thị danh sách thành nào đang có quân đoàn chiếm giữ (`"Có quân đoàn chiếm đóng"`) hoặc bỏ trống (`"Chưa có quân đoàn chiếm đóng"`), kèm tên quân đoàn và mức thuế thành trì.
4. **Cơ chế Cổng thành vật lý, Khí giới và Cờ trạng thái trên `LocalPlayer`** **(A)**:
   - Trong `case_004` (`S→C 0x03`, hàm `FUN_00733DC0`), khi vẽ thực thể trên bản đồ Công Thành, client kiểm tra ID hình ảnh cổng thành và trạng thái `LocalPlayer+0x142C` / `LocalPlayer+0x1435` để chặn người chơi phe công đi xuyên qua cổng thành khi cổng chưa bị phá hủy **(A)**.
   - Gói `S→C 29 32` (`sub = 50`) gán `LocalPlayer + 0x142C = RP[1]` và đóng dấu thời gian `LocalPlayer + 0x1430 = GetTickCount()` (dùng để tính thời gian hồi chiêu hoặc trạng thái bảo vệ/khí giới trong Công Thành Chiến) **(A)**.
   - Gói `S→C 10 0D` (`case_016`) khóa thao tác quản lý thành viên quân đoàn khi đang diễn ra chiến sự: *"Trong chiến tranh công kích thành trì không thể chức năng quân đoàn"* (`0x0079714C`) **(A)**.
   - Tại nhà trọ (`S→C 0x1F`, `case_027`), nếu `FUN_00634F3C(mapId, 6)` trả về `true` (đang ở bản đồ Công Thành Chiến), chi phí nghỉ ngơi hồi HP/SP được miễn phí (`0 vàng`) **(A)**.

---

### 3.4. Sự kiện D: Đua Thuyền Rồng (Dragon Boat Race — Opcode `0x46`)

#### a) Quản lý đối tượng & Bản đồ đường đua (Level A)
- **Lớp quản lý**: `TLH_DragonBoat` (`*gvar_007DA714`, VMT tại `0x0051B408`, khởi tạo tại `FUN_0051BA74`) và bảng xếp hạng `TLH_DragonBoatStanding` (`*gvar_007DA1E8`, VMT tại `0x0051C820`) **(A)**.
- **Cụm bản đồ đường đua**:
  - `60509` (`KhuThuyềnRồng`): Khu vực tập kết và báo danh.
  - `60510..60521` (`0xEC5E..0xEC69`, 12 bản đồ `ĐườngĐua1..12`): Các chặng sông đua thuyền.
  - `60522` (`0xEC6A`, `ĐiểmCuốiĐ.Đua`): Khu vực về đích, nơi đặt NPC trao thưởng (`Quest 231`) **(A)**.

#### b) Vòng đời gói tin `S→C 0x46` (`case_062` tại `0x007960BD`) & Cơ chế bấm phím `C→S 46 01` (Q2, Q4, Q7 - Level A)
Khi nhận gói `S→C 0x46`, `case_062` phân nhánh theo `RP[0]` (`sub 01..08`) như sau **(A)**:
1. **`46 01 [state:u8]` (`FUN_0051BE8C`)**:
   - Thiết lập trạng thái cuộc đua tại `*gvar_007DA714 + 0x49 = RP[1]`.
   - Nếu `RP[1] == 2` (Bắt đầu đếm ngược xuất phát): gán `+0x48 = 1` (hiện giao diện Đua Thuyền), `+0x54 = 5` (đếm ngược 5 giây), `+0x50 = GetTickCount()`, và nạp hình ảnh hiệu ứng đếm ngược từ `\data\number.xp` (`0x0051BEEC`) **(A)**.
2. **`46 02 [speedOrBoost:u8]` (`FUN_0051BEF0`)**:
   - Cập nhật chỉ số tốc độ / thanh năng lượng chèo thuyền tại `*gvar_007DA714 + 0x4A = RP[1]` **(A)**.
3. **`46 03 [standingData...]` (`FUN_0051CEB0` trên `*gvar_007DA1E8`)**:
   - Nạp danh sách thứ hạng các đội thuyền rồng vừa về đích vào bảng `TLH_DragonBoatStanding` và bật hiển thị bảng xếp hạng **(A)**.
4. **`46 04 [msgCode:u8]` (`FUN_0051BF80`)**:
   - Phát thông báo hệ thống về tiến trình cuộc đua Đua Thuyền Rồng (`RP[1] = 1..3`) **(A)**:
     - `1`: *"Hoạt động đua thuyền rồng sắp bắt đầu, mời các đội chuẩn bị!"* (`0x0051C004`).
     - `2`: *"Hoạt động đua thuyền rồng đã chính thức bắt đầu!"* (`0x0051C050`).
     - `3`: *"Hoạt động đua thuyền rồng đã kết thúc!"* (`0x0051C090`).
5. **`46 05` (`FUN_0051C0AC`)**:
   - Xử lý hiệu ứng cản trở / vật phẩm trên đường đua (như trúng xoáy nước hoặc tăng tốc đột xuất) **(A)**.
6. **`46 06` (`FUN_0051C168`) & `46 07` (`FUN_0051C24C`)**:
   - Đồng bộ vị trí tương đối giữa các thuyền trên bản đồ đường đua và cập nhật hiệu ứng sóng nước **(A)**.
7. **`46 08` (`FUN_0051C254`)**:
   - Kết thúc chặng đua cho người chơi hiện tại, ẩn HUD chèo thuyền và chuyển trạng thái sang nhận thưởng tại map `60522` **(A)**.
8. **Cơ chế điều khiển chèo thuyền từ bàn phím (`FUN_0051BF04` → `C→S 46 01`)** **(A)**:
   - Khi đang ở trên một trong 12 bản đồ đường đua (`mapId - 0xEC5E < 0x0C`, tức `60510..60521`) và trạng thái đua `+0x49 == 3` (đang đua), hàm xử lý phím `FUN_0051BF04` bắt phím mũi tên **Trái (`VK_LEFT = 0x25`)** và **Phải (`VK_RIGHT = 0x27`)** **(A)**:
     - Người chơi phải bấm **luân phiên** Trái rồi Phải (kiểm tra cờ phím trước đó tại `+0x58` và `+0x59`).
     - Cứ mỗi lần hoàn tất 1 nhịp chèo hợp lệ (Trái → Phải), client gọi `FUN_0077F414(..., 0x46)` với `SubOP = 1` để gửi gói 2 byte `[0x46, 0x01]` lên server (`FUN_00789F85`) **(A)**.

---

### 3.5. Sự kiện E: Lễ Hội / Sự Kiện Theo Mùa (`Festival.Dat`, `S→C 24 19`, `S→C 17 39`, `S→C 10 18/19`, `S→C 14 24..35`)

#### a) Giao diện Thuyết minh Lễ hội `\Data\Festival.Dat` (`TLH_FesInfoForm` `*gvar_007DA1EC`) (Q4, Q6 - Level A)
- Khi server gửi gói **`S→C 24 19 [dateYYYYMMDD:u32 LE]`** (`case_032` tại `0x00793782`), client gọi `FUN_0077ED68` chuyển số nguyên 4 byte `YYYYMMDD` thành giá trị `TDateTime` (`double`), sau đó gọi `FUN_0051D914(*gvar_007DA1EC, dateDouble)` **(A)**.
- Hàm `FUN_0051D914` gọi `FUN_0051E2DC` quét qua các bản ghi đã giải mã của `\Data\Festival.Dat` (mỗi bản ghi dài `0x7EF = 2031 byte`) để tìm bản ghi có trường `double` tại `+0x00` khớp với `dateDouble`. Nếu tìm thấy, form `TLH_FesInfoForm` hiển thị 5 trường thông tin lễ hội: **Tên lễ hội**, **Thời gian**, **Nguồn gốc**, **Hoạt động trong game**, và **Vật phẩm liên quan** **(A)**.
- **Phát hiện quan trọng từ dữ liệu thực tế `festival_dat_decrypted/rec01..rec07.bin`** **(A)**:
  - Trong file `Festival.Dat` hiện tại của bản client Việt hóa (10 bản ghi × 2031B = 20.310 byte, trong đó `rec00` là header, `rec08` và `rec09` là 2 khóa XOR chẵn/lẻ), cả **7 bản ghi nội dung (`rec01..rec07`) đều có giá trị `double` tại `+0x00` bằng `11.0` (`00 00 00 00 00 00 26 40`, ứng với ngày `1900-01-10`)** và `u16` tại `+0x08` bằng `0xB4C2` **(A)**:

| Bản ghi | Giá trị `double` (`+0x00`) | Tên Lễ hội (`+0x0A`) | Khung giờ ghi trong bản ghi (`+0x1F`) | Tóm tắt nội dung & Vật phẩm trong `Festival.Dat` (Level A) |
| :--- | :--- | :--- | :--- | :--- |
| `rec01.bin` | `11.0` (`19000110`) | **Lễ Tình Nhân (Valentine 14/2)** | `18:00 - 24:00` | Thu thập Sô-cô-la tình nhân (`50013..50018`), tặng hoa hồng và nhận quà chúc phúc |
| `rec02.bin` | `11.0` (`19000110`) | **Đêm Giao Thừa (Trừ Tịch)** | `19:00 - 24:00` | Đánh đuổi Niên Thú, nhận lì xì năm mới và thức đêm đón giao thừa |
| `rec03.bin` | `11.0` (`19000110`) | **Tết Nguyên Đán (Mùng 1 Tết)** | `00:00 - 24:00` | Chúc Tết đầu xuân, nhận bao lì xì may mắn và vật phẩm mừng năm mới |
| `rec04.bin` | `11.0` (`19000110`) | **Tết Nguyên Tiêu (Rằm tháng Giêng)** | `19:00 - 23:00` | Thưởng thức Bánh trôi tàu (`50009..50012`), treo lồng đèn và giải câu đố đèn cùng Miếu Ông / Miếu Bà |
| `rec05.bin` | `11.0` (`19000110`) | **Lễ hội Búp Bê (Hina Matsuri 3/3)** | `19:00 - 22:00` | Tìm lại bộ Búp bê Thiên Hoàng (`46066`) và Hoàng Hậu (`46067`) (`Quest 221`) |
| `rec06.bin` | `11.0` (`19000110`) | **Tết Đoan Ngọ (5/5 Âm lịch)** | `19:00 - 22:00` | Đua Thuyền Rồng tưởng niệm Khuất Nguyên, treo Ngải thảo (`50096`) & Xương bồ (`50097`), ăn Bánh ú (`50019..50022`) |
| `rec07.bin` | `11.0` (`19000110`) | **Lễ Thất Tịch (7/7 Âm lịch)** | `19:00 - 23:00` | Cầu Ô Thước giúp Ngưu Lang - Chức Nữ đoàn tụ (`Quest 306`), nhận `Thất tịch tình nhân thang` (`26058`) |

  - *Hệ quả kỹ thuật cho Server Rust*: Nếu giữ nguyên file `Festival.Dat` gốc trên client, để mở được form giới thiệu lễ hội qua `S→C 24 19`, máy chủ phải gửi giá trị ngày `19000110` (`0x0121EAC6` LE → `C6 EA 21 01`, khi đó `rec01` sẽ luôn khớp đầu tiên), hoặc cập nhật lại trường `double` tại `+0x00` của `rec01..rec07` trong `Festival.Dat` trước khi mã hóa lại **(A)**.

#### b) Bộ điều phối 74 Mã thông báo Lễ hội `S→C 17 39 [code:u8, ...]` (`FUN_00722CD4`) (Q2 - Level A)
Trong `case_020` (`Main OP 0x17`, `sub 0x39 = 57`), hàm `FUN_00722CD4` (tại `0x00722CD4`, dài hơn 600 dòng pseudo-C) xử lý `RP[1]` từ `1` đến `74` (`0x01..0x4A`), bao quát toàn bộ các thông báo sự kiện theo mùa trong năm **(A)**:
- **`0x01..0x0A`**: Các thông báo đổi thưởng sự kiện, kiểm tra túi đồ đầy, điểm tích lũy lễ hội và thông báo mở lôi đài ngoài giờ bằng `5.000 / 10.000` lượng vàng (`0x0072541C`).
- **`0x0B..0x0E`**: Thông báo chọn và nhận phần thưởng theo hệ Ngũ hành (Địa, Thủy, Hỏa, Phong, Tâm) tại `0x00723474..0x007236BC`.
- **`0x0F..0x22`**: Chuỗi sự kiện **Tết Nguyên Đán, Giao Thừa, Mừng Xuân Ất Dậu & Đập Niên Thú / Bắt Gà** (nhận bao lì xì, pháo tết, chúc phúc đầu năm).
- **`0x23..0x32`**: Chuỗi sự kiện **Lễ Tình Nhân (Valentine), Lễ hội Búp Bê (3/3), Tết Hàn Thực, Tết Đoan Ngọ** (nhận bánh ú, túi thơm Khuất Nguyên, rượu hùng hoàng).
- **`0x33..0x4A`**: Chuỗi sự kiện **Thất Tịch (7/7), Tết Trung Thu (Nướng thịt Trung Thu, Ngọc Thố, Hằng Nga), Giáng Sinh (Ông già Noel phát quà, Cây thông Noel)** **(A)**.

#### c) Các gói tin Lễ hội chuyên biệt trong `0x10` và `0x14` (Q2 - Level A)
1. **Đố đèn Tết Nguyên Tiêu cùng Miếu Ông / Miếu Bà (`TWF_Day55QuestManage` `*gvar_007DA1D0`)** **(A)**:
   - `S→C 10 18` (`sub = 24`, `FUN_0070736C`): Mở hộp thoại `*gvar_007D9F00` hỏi người chơi: *"Bạn muốn tiếp tục thách thức các câu đố của Miếu Ông, hay là trực tiếp lãnh giải thưởng?"* với 2 nút *"Tiếp tục"* và *"Lãnh phần thưởng"*. Khi người chơi chọn, client gửi **`C→S 10 24`** (`sub = 36`, 6 byte `[0x10, 0x24, choice:u32 LE]`) **(A)**.
   - `S→C 10 19 [riddleCode:u8]` (`sub = 25`, `FUN_007074CC`): Hiển thị phản hồi của Miếu Ông / Miếu Bà tương ứng với `RP[1] = 1..10` (chúc mừng giải đố đúng, báo trả lời sai, hoặc trao thưởng đèn lồng Nguyên Tiêu tại `0x007076B4..0x00707AF0`) **(A)**.
2. **Các nhánh Lễ hội trong `S→C 0x14` (`case_018`)** **(A)**:
   - `14 1F [count:u8, ...]`: Thông báo hoạt động *"Truy tìm hoạt động của ÁcMaBaDoYau"* (`0x007975F8`).
   - `14 24 [sub:u8 (1..2)]`: Thông báo bắt đầu / kết thúc lễ hội Tết Nguyên Tiêu (`0x007976C0`, `0x00797794`).
   - `14 26 [sub:u8 (1..3)]`: Thông báo bắt đầu / diễn biến / kết thúc lễ hội Tết Đoan Ngọ (`0x0079783C`, `0x007978F8`, `0x007979A0`).
   - `14 2D`: Hiển thị *"Hôm nay không phải là thất tịch đâu"* (`0x00797BB8`).
   - `14 2E`, `14 2F`, `14 30`: Chuỗi thông báo **Tết Trung Thu** (Thỏ Ngọc / Hằng Nga giáng trần, thu thập bánh Trung Thu tại `0x00797BE4..0x00797E54`).
   - `14 32 [code:u8 (1..5)]` (`FUN_007485BC`): Thông báo giải câu đố Khiêu Gia Quan / Đối đáp lễ hội (`0x007486B0..0x007487FC`).
   - `14 33 [code:u8 (1..6)]` (`FUN_00748860`): Hoạt động bình chọn *"Ba Đậu Yêu yêu thích nhất"* và sự kiện *"Tiêu diệt Tứ Hại"* (`0x00748964..0x00748B04`).
   - `14 35 [sub:u8 (1..2)]` (`FUN_00749198` / `FUN_0074927C`): Hiệu ứng biến thân và thông báo hoạt động **Nướng thịt Trung Thu** **(A)**.

---

### 3.6. Sự kiện F: Quan Chiến / Event PK (Spectator & Tournament PK — Opcode `0x0B`)

#### a) Cơ chế Quan chiến và PK Sự kiện trên Client (Level A)
- **Quản lý trận đấu**: `TFightManage` (`*gvar_007D9CE4`, điều phối qua `case_011` — `Main OP 0x0B`) kết hợp với các bản đồ có thuộc tính PK/Đấu trường (`Scene.Dat kind = 2`, kiểm tra qua `FUN_00634F3C(mapId, 2)`) **(A)**.
- **Luồng kích hoạt từ phía người chơi (`C→S 0B 05`)** **(A)**:
  - Khi người chơi bật biểu tượng Quan chiến (Watch) hoặc PK trên thanh công cụ rồi nhấp vào một người chơi đang có biểu tượng giao chiến trên bản đồ, client gọi `FUN_0077F414(0x0B)` với `SubOP = 5` (`FUN_00780AC8`).
  - Gói tin gửi đi dài `8 byte`: `[0x0B, 0x05, targetPlayerId:u32 LE, mode:u8, flag:u8]` **(A)**.
- **Luồng phản hồi từ Server (`S→C 0x0B`)** **(A)**:
  - Server gửi chuỗi gói `S→C 0B 0B` (`sub = 11`, khởi tạo bối cảnh sân đấu và danh sách các đơn vị ở hai phe trong `*gvar_007D9CE4`), tiếp theo là `0B 00`, `0B 01`, `0B 03`, `0B 04`, `0B 05` để phát diễn hoạt từng hiệp đấu cho người quan chiến y hệt người đang đánh, nhưng khóa bảng chọn lệnh chiến đấu (chỉ cho phép bấm nút Rút lui / Thoát quan chiến gửi **`C→S 0B 04 [flag:u8]`** dài 3 byte) **(A)**.
- **Kiểm soát đặc biệt tại Máy chủ Thi đấu & Bản đồ PK (`kind = 2`)** **(A)**:
  - Tại các bản đồ đấu trường (`10851`, `10991`, `11010..11022`, `11045..11046`), client tự động cho phép thách đấu/quan chiến trực tiếp và miễn phí tiền nghỉ ngơi nhà trọ (`case_027`, `mapId == 0x2AEF`) **(A)**.
  - Nếu trận đấu kéo dài quá thời lượng sự kiện, server gửi `S→C 00 2F`: *"Sự kiện kết thúc trước khi chiến đấu kết thúc"* (`0x00796BB4`) hoặc `S→C 00 36`: *"Hoạt động lôi đài đấu trận kết thúc"* (`0x00796CAC`) để đưa người chơi ra khỏi trận **(A)**.

---

### 3.7. Sự kiện G: Vấn Đáp Bắc Tinh Quân (`0x10`) & Kênh GM (`0xC7`)

#### a) Vấn Đáp Bắc Tinh Quân (Anti-Bot / In-game Quiz — `S→C 10 01 / 10 02` & `C→S 10 01`) (Q2, Q4 - Level A)
Sự kiện Vấn đáp Bắc Tinh Quân (Bắc Đẩu Tinh Quân) vừa là sự kiện đố vui có thưởng vừa là cơ chế kiểm tra chống treo máy (anti-bot) của TS Online **(A)**:
1. **Gói mở bảng câu hỏi `S→C 10 01` (`case_016` tại `0x0078E5D7`)** **(A)**:
   - **Cấu trúc byte**:
     ```
     [0x10][0x01][qLen:u8][question: qLen bytes]
     [opt1Len:u8][opt1: opt1Len bytes]
     [opt2Len:u8][opt2: opt2Len bytes]
     [opt3Len:u8][opt3: opt3Len bytes]
     [opt4: toàn bộ phần byte còn lại của gói tin (Length - offset)]
     ```
   - **Hành vi trên Client**:
     - Lưu chuỗi câu hỏi (VISCII) vào `*gvar_007D9D94 + 0x134` và `+0x14C`.
     - Xóa danh sách đáp án cũ tại `*(TList**)(*gvar_007D9D94 + 0x138)` rồi thêm lần lượt 4 chuỗi đáp án `opt1..opt4`.
     - Gán cờ hiển thị `*gvar_007D9D94 + 0x148 = 1` và gọi `FUN_0060FF34` để vẽ hộp thoại Bắc Tinh Quân lên màn hình **(A)**.
2. **Gói gửi đáp án `C→S 10 01` (`FUN_00781A9A` tại `0x00781A9A`)** **(A)**:
   - Khi người chơi nhấp chọn 1 trong 4 đáp án, chỉ số đáp án (`1..4`) được ghi vào `*(uint32_t*)(*gvar_007D9D94 + 0x13C)`.
   - Client gửi gói 6 byte: `[0x10, 0x01, answerIndex:u32 LE]` **(A)**.
3. **Gói thông báo kết quả `S→C 10 02 [resultCode:u8]` (`case_016` tại `0x0078E82C`)** **(A)**:
   - `RP[1] == 1`: Hiển thị hộp thoại `*gvar_007DA084`: *"Thật thông minh! Chúc mừng bạn đã trả lời đúng!!"* (`0x0079706C`).
   - `RP[1] == 2`: Đóng bảng câu hỏi (`*gvar_007D9D94 + 0x148 = 0`, `*gvar_007D9D6C + 0x48 = 0`), hiển thị: *"Ồ! bạn trả lời sai rồi!"* (`0x007970A8`).
   - `RP[1] == 3`: Đóng bảng câu hỏi, hiển thị: *"Ồ! Bạn đã sai đến lần thứ 3 rồi!"* (`0x007970CC`). Ngay sau đó server có thể gửi kèm `S→C 00 02` (*"Trả lời sai 3 lần sẽ bị mất kết nối"*) hoặc `S→C 00 2E` (*"Gian xảo trong vấn đáp của Bắc Đẩu Quân"*) để ngắt kết nối **(A)**.
   - `RP[1] == 9`: Đóng bảng câu hỏi, hiển thị: *"Ồ, Trả lời sai sẽ bị bắt đi đó!"* (`0x007970F8`) (dịch chuyển nhân vật vào ngục tối / bản đồ hình phạt) **(A)**.

#### b) Công cụ Quản trị GM (`0x10 sub 03..20`) & Kênh Liên lạc GM (`0xC7 sub 04, 06`) (Level A)
- **Trong `Main OP 0x10`**: Các nhánh `sub 03, 04, 05, 06, 09, 0A, 0B, 0C, 0E, 0F, 10, 11, 12, 13` gọi các phương thức của `TGmManage` (`*gvar_007DA160`, `FUN_00613230`..`FUN_00614BB0`) để đồng bộ danh sách người chơi bị giam, danh sách khiếu nại và bảng điều khiển GM (`*gvar_007DA110`) **(A)**.
- **Trong `Main OP 0xC7` (`case_065` tại `0x007962FC`)**:
  - `S→C C7 04` (`FUN_00613634`): Nhận tin nhắn hội thoại trực tiếp từ GM.
  - `S→C C7 06` (`FUN_0061353C`): Nhận thông báo khẩn từ GM và phát tệp âm thanh cảnh báo `sound\WA0033.wav` **(A)**.
  - Phía gửi C→S có 2 hàm tương ứng: `C→S C7 04` (`FUN_0078A32C`, 10 byte header + chuỗi) và `C→S C7 06` (`FUN_0078A42E`, 6 byte header + chuỗi) **(A)**.

---

### 3.8. Sự kiện H: Giải mã 3 Sự kiện Tên lóng Cộng đồng ("2K", "40 NPC", "Hổ Lao Quan") (Level A)

| Tên lóng cộng đồng | Tên chính thức trong Client (`aLogin.exe`) | Bản đồ & NPC trọng tâm | Cơ chế kích hoạt S→C & Kịch bản `eve.emg` (Level A) |
| :--- | :--- | :--- | :--- |
| **1. "2K"** | **Hành động giải cứu Nhị Kiều Giang Đông** | Khởi hành tại `12003` (`ĐồngTướcĐài` - Diệp Thành, gặp NPC `15312 TảTừ` ở `610, 290`); leo 18 tầng tháp `12911..12928` (`ĐồngTướcĐài` → `KimPhụngĐỉnh` `12927` & `NgọcLongĐỉnh` `12928`) | • **Thông báo S→C**: `14 29 01` (*"Xin lỗi, trước mắt tạm thời không nhận đăng ký"* `0x00797A30`), `14 29 02` (*"Hành động giải cứu Nhị Kiều Giang Đông đã được bắt đầu, bạn hãy nhanh chống khởi hành đến Đồng Tước Đài trong Diệp Thành để triển khai hành động giải cứu."* `0x00797A5C`), `14 29 03` (*"...đã kết thúc..."* `0x00797B00`).<br>• **Logic Eve (`Quest 212`)**: Yêu cầu tổ đội (`PartySize==5` hoặc `1`), kiểm tra `QuestStep(212)` từ `0` đến `7` qua 18 trận đánh trấn ải (`Fight`), cuối cùng giải cứu Đại Kiều (`12927`) và Tiểu Kiều (`12928`) để nhận `23086 Lông của Nhị Kiều`, `26051 Đậu thần Nhị Kiều`, `46049/46050 Chứng nhận Đại/Tiểu Kiều`. |
| **2. "40 NPC"** | **Hoạt động Lôi Đài Khiêu Chiến (Lôi Đài Thiêu Chiến)** | Báo danh tại `12001` (`ĐấuTrườngVõĐại`, gặp NPC `15162 LôiĐàiTruyềnSư` ở `522, 415`) → phòng chờ `10852` (`LôiĐàiDựBị`) → sàn đấu `10851` (`ĐấuTrườngPK`) | • **Thông báo S→C**: `25 03 09` (*"Hoạt động lôi đài thiêu chiến đã bắt đầu, người tham gia đến chỗ nhân viên lôi đài ghi danh!"* `0x00729914`) và `25 03 0A` (*"Hoạt động lôi đài thiêu chiến đã kết thúc!"* `0x00729998`).<br>• **Logic Eve (`Quest 218`)**: Tại `12001`, nộp 1.000 vàng (`Gold>=1000` / `TakeGold(1000)`, hoặc dùng `46072 Vé ưu đãi lôi đài`) để dịch chuyển vào đấu trường và đánh vượt 40 đợt NPC liên tiếp. |
| **3. "Hổ Lao Quan"** | **Trận chiến Đổng Trác Thảo Phạt** (thuộc bộ 3 Chiến trường `TMRBatterManage` cùng Đại Hưng Sơn & Cự Lộc) | `15001` (`HổLaoQuan`), `15501` (`TrạiLiênMinh`), `15502` (`RừngHuỳnhDương`) | • **Điều khiển S→C**: Mở bảng thuyết minh chiến dịch qua `S→C 24 0D` (`*gvar_007D9ED0`, chuỗi thuyết minh tại `0x005587F8` & `0x005590AC`). Người chơi bấm nút tham gia gửi **`C→S 3F 01 [campaignId:u16 LE]`** (`FUN_00789557`).<br>• **Tiến trình chiến dịch**: Điều phối qua `S→C 3F 01..0A` (`TMRBatterManage` `*gvar_007D9DFC`), bao gồm cả nhiệm vụ phụ *"Trừ diệt Nghiêm Bạch Hổ"* và bảo vệ *"Chiến thuyền Tôn Sách"*. |


---

### 3.9. Sự kiện I: Chinh Phục Cửu Thiên ("9 Tầng Cửu Thiên") — Điều khiển 100% bằng Kịch bản `eve.emg` (Level A)

#### a) Vì sao `aLogin.exe` không có Opcode hay chuỗi thông báo cứng cho 9 Tầng Cửu Thiên? (Level A)
- Trong thực thi `aLogin.exe`, **không có Main OP/SubOP riêng** và **không có chuỗi hằng VISCII** nào chứa cụm từ `"Cửu Thiên"` **(A)**.
- Tuy nhiên, toàn bộ sự kiện **Chinh Phục Cửu Thiên (9 tầng Cửu Thiên)** lại **tồn tại đầy đủ 100% trong bộ dữ liệu tĩnh của Client** (`\Data\Scene.Dat`, `\Data\Npc.dat`, `\Data\Item.Dat`, `\Data\Talk.dat`, và `\Data\eve.emg`) và vận hành hoàn toàn thông qua máy trạng thái kịch bản **`S→C / C→S 0x14`** (`QuestStep`, `DialogChoice`, `BattleResult`, `FollowPet`, `RewardPet`) cùng hệ thống chiến đấu **`0x0B`** **(A)**.

#### b) Hệ thống Bản đồ 9 Tầng Cửu Thiên trong `\Data\Scene.Dat` (2 phiên bản) (Level A)
Trong `Scene.Dat` (giải mã bằng `(w ^ 0xEA6C) - 9`), Cửu Thiên có mặt ở cả 2 dải bản đồ **(A)**:
1. **Cổng vào sự kiện**:
   - `mapId = 33033` (`kind = 1`: `Eo núi Kim Long`, `kind = 2`: `Cửu Thiên`) — nơi đặt NPC khởi phát sự kiện `61209` (**Hầu Tôn**) tại tọa độ `(500, 510)` **(A)**.
2. **Dải bản đồ số 1 (`33911..33920` & `39919`)**:
   - `33911` (`Tầng 1`), `33912` (`Tầng 1/2`), `33913` (`Tầng 1/3`), `33914` (`Tầng 2/4`), `33915` (`Tầng 3/5`), `33916` (`Tầng 4/6`), `33917` (`Tầng 5/7`), `33918` (`Tầng 6/8`), `33919` (`Tầng 7`), `33920` (`Tầng 8`), `39919` (`Tầng 9 Cửu Thiên`) **(A)**.
3. **Dải bản đồ số 2 (`49960..49969` — Dải bản đồ gắn kịch bản `eve.emg` thực tế)**:
   - `49960` (`Tầng 1`), `49961` (`Tầng 2`), `49962` (`Tầng 1/3`), `49963` (`Tầng 2/4`), `49964` (`Tầng 3/5`), `49965` (`Tầng 4/6`), `49966` (`Tầng 5/7`), `49967` (`Tầng 6/8`), `49968` (`Tầng 7/9`), `49969` (`Tầng 8 Cửu Thiên | Tân Kinh Nam` — Đỉnh Cửu Thiên có 83 NPC Thiên Binh Thiên Tướng dàn trận) **(A)**.

#### c) Lời thoại sự kiện giải mã từ `\Data\Talk.dat` (`talkId = ((w ^ 0xECEA) - 5) & 0xFFFF`) (Level A)
Toàn bộ cốt truyện *"Hầu Tôn muốn nối gót Tề Thiên Đại Thánh công phá Cửu Thiên, đại náo Thiên Cung"* nằm ở các bản ghi `53726..53740` của `Talk.dat` **(A)**:
- `53726`: *"Đại Vương của chúng ta từng đến đây và công phá Cửu Thiên, đại náo Thiên Cung khiến Tiên Giới phải một phen khiếp sợ. Ta rất muốn làm được như Người. Ngươi có thể giúp ta không?"*
- `53727`: *"Đại náo Thiên Cung? Không lẽ ngươi đang nói đến Tề Thiên Đại Thánh sao?"*
- `53728`: *"Ngươi cũng biết Đại Vương của ta sao? Không biết đến khi nào ta mới có thể làm được như Đại Vương? Ngươi có thể giúp ta không?"*
- `53730`: *"Ta muốn được như Đại Vương: Trước là Công phá Cửu Thiên, sau là đại náo Thiên Cung!"*
- `53731`: *"Hãy cùng ta Chinh phục Cửu Thiên. Mỗi lần chỉ có thể đi một người thôi nhé!"*
- `53733`: *"Đây là Cửu Thiên. Không được lệnh của Ngọc Đế, bất cứ ai cũng không thể vượt qua cánh cổng này!"*
- `53734`: *"Mau cút về dưới cho ta!"* (Khi đánh thua hoặc chưa đủ `QuestStep`).
- `53735`: *"Loài người to gan dám xông vào đài Cửu Thiên. Hãy xem thiên la địa võng của Thiên Đình."*
- `53736`: *"Cảm ơn thiếu hiệp đã giúp ta thực hiện được ước mơ này! Ta muốn ở lại đây tham quan một chút. Thiếu hiệp có muốn quay về Núi Kim Long không?"*
- `53737`: *"Xem ra ngươi cũng có chút bản lĩnh! Bây giờ, ngươi đã có thể vượt qua trận tiếp theo! Nhớ đừng quay lại tìm chúng ta nhé!"*
- `53738`: *"Bạn phải giải tán đội ngũ trước. Sau đó cho ta xuất chiến và nhận thưởng từng người một thôi nhé!"*
- `53739`: *"Đủ bản lĩnh đến được nơi này. Ngươi khiến ta nhớ lại Tôn Ngộ Không năm xưa?"*
- `53740`: *"Trên người bạn đang có Hầu Tôn nên không thể tham gia sự kiện này."*

#### d) Cơ chế Chuỗi `QuestStep` (`12309..12319`, `12338..12345`), Đội hình Boss và Phần thưởng (Level A)
Từ `data_files/eve/eve_logic.tsv`, `eve_npcs.tsv`, `eve_fights.tsv` và `\Data\Item.Dat` (`itemId = ((w ^ 0xEFC3) - 9) & 0xFFFF`), quy trình nghiệp vụ của **Chinh Phục Cửu Thiên** diễn ra như sau **(A)**:
1. **Điều kiện khởi hành tại `map 33033` (`eve 2` & `eve 3`)**:
   - Khi vào `33033`, `eve 2` tự động xóa sạch các biến nhiệm vụ `12309..12319` về `0` nếu bắt đầu lượt mới (`Action Quest 12309..12319 = 0`).
   - Khi nói chuyện với NPC `61209` (**Hầu Tôn**):
     - Kiểm tra người chơi **chưa sở hữu võ tướng `14735` (Hầu Tôn)** (nếu đang có thì báo `Talk 53740` và từ chối).
     - Kiểm tra `QuestStep(12310 == 0)` (còn lượt đi trong tuần; nếu `12310 == 1` thì báo `Talk 37722`: *"Hiện tại vẫn chưa đến lúc..."*. Để đi thêm lượt trong tuần, người chơi sử dụng vật phẩm **`46644 Cửu Thiên Chiến Kỳ`** — *"Chiến Kỳ dùng để tăng thêm lượt tham gia sự kiện Chinh Phục Cửu Thiên trong tuần"*).
     - Kiểm tra nhân vật đã Chuyển sinh (`PlayerAttribute(5, 2, 0)`), `Level >= 120` (`PlayerAttribute(0, 1, 2, 119)`), hoàn thành `QuestStep(11449 == 1)`.
     - Khi chọn *"Đồng ý"* (`DialogChoice 1`), server cho NPC **Hầu Tôn (`14735`)** đi theo hỗ trợ (`Action NpcTeam 1`), đặt **`QuestStep(12309 = 1)`** và dịch chuyển (`Door 2`) vào tầng đầu tiên **(A)**.
2. **Cấu trúc 18 Ải Chiến đấu trên 9 Tầng (`Quest 12309` từ `Step 1` → `Step 19`)**:
   - Mỗi bản đồ Cửu Thiên gồm **2 cụm Boss gác cổng** (ứng với 2 bậc `QuestStep(12309)` liên tiếp) và **1 Thùng Báu Vật (`NPC 20001`)**:
     - Bắt buộc phải có `FollowPet(14735)` (**Hầu Tôn** đi theo) mới kích hoạt được hội thoại thách đấu.
     - Thắng ải lẻ (`Battle 1/3`): `QuestStep(12309)` tăng `+1`.
     - Thắng ải chẵn (`Battle 2/4`): `QuestStep(12309)` tăng `+1` (nếu thua bị trừ `-1` lùi lại ải trước và bị đẩy về cổng!). Nếu là lần đầu vượt ải trong tuần (`Quest 12338..12345 == 0`), người chơi còn nhận thêm túi quà rơi trực tiếp sau trận đánh và mở được **Thùng Báu Vật (`NPC 20001`)** trên tầng đó (đánh dấu vào `Quest 12311..12318`) **(A)**:

| Bản đồ (`scene_id`) | Bước `Quest 12309` | Đội hình Boss Trấn Ải (Giải mã từ `Npc.dat`) | Quà rơi sau trận & Quà mở `Thùng Báu Vật (20001)` (Giải mã từ `Item.Dat`) |
| :--- | :--- | :--- | :--- |
| `49961` (Cụm Tầng 1–2) | `Step 1 → 3` | • **Ải 1**: `61230` (Thiên Môn Thần), `47067` (Hữu Môn Thần), `47068` (Tả Môn Thần), `61231` (Cự Linh Thần)<br>• **Ải 2**: `47049` (Xích Cước Đại Tiên), `47104` (Diêm Nam Quân), `47105` (Diêm Bắc Quân) | • **Thắng Ải 2 (`Quest 12338`)**: ngẫu nhiên `46293` (*Túi bảo cự nham*), `46294` (*Túi bảo thiên thủy*), `46295` (*Túi bảo hỏa phụng*), `46296` (*Túi bảo phi long*)<br>• **Thùng Báu Vật (`Quest 12311`)**: `46931` (*Túi item 132*) hoặc `46625` (*Túi ngọc Phúc Lộc*) |
| `49963` (Cụm Tầng 3–4) | `Step 5 → 7` | • **Ải 1**: `61181` (Bách Niên Đậu Yêu), `61177` & `61180` (BDY Hoàng Kim)<br>• **Ải 2**: `47081` (Vạn Niên Quy Tiên), `47080` (T.RằmT7thầnbí), `47078` (Du Dịch Linh Quan) | • **Thắng Ải 2 (`Quest 12340`)**: `46921` (*Túi tinh tượng*)<br>• **Thùng Báu Vật (`Quest 12313`)**: `46947` (*Túi item 138*) hoặc `46625` (*Túi ngọc Phúc Lộc*) |
| `49967` (Cụm Tầng 6–7) | `Step 13 → 15` | • **Ải 1**: `47122` (Trung Đẩu Quân), `47121` (Thủy Đức Quân), `47120` (Hỏa Đức Quân)<br>• **Ải 2**: `47128` (Bảo Sanh Đại Đế), `47126` (Hoa Đà), `47132` (Từ Phúc) | • **Thắng Ải 2 (`Quest 12344`)**: ngẫu nhiên `24067` (*Di Giai Vũ Mao*), `24068` (*Thư Di Vũ Mao*), `45024` (*Vĩ Kỳ Băng Tinh*)<br>• **Thùng Báu Vật (`Quest 12317`)**: `31139` (*Mộc Ngưu Vé*) hoặc `31140` (*Phiếu Lưu Mã*) |
| `49968` (Cụm Tầng 7–8) | `Step 15 → 17` | • **Ải 1**: `47178` (Thiên Quan Đại Đế), `47179` (Thủy Quan Đại Đế), `47177` (Địa Quan Đại Đế)<br>• **Ải 2**: `47108` (Chân Vũ), `47137` (Thần Tài), `47136` (Nguyệt Lão) | • **Thắng Ải 2 (`Quest 12345`)**: ngẫu nhiên `57939` (*Đông tử tiếu*), `57940` (*Tây tử tiếu*), `57941` (*Lưu tinh.Bất ý đả*), `57942` (*Lưu tinh.Thần ý đả*)<br>• **Thùng Báu Vật (`Quest 12318`)**: `23360` (*Tân Tần Vô Nhị*) hoặc `23361` (*Tinh Húc Vô Nhị*) |
| `49969` (Đỉnh Tầng 9) | `Step 17 → 19` | • **Đại trận 83 NPC Thiên Đình**: `47109` (Kim Long), `47049` (Xích Cước Đại Tiên), `47077` (Gia Lực Kim Cang), `47099` (Đế Nghiêu), `61214` (Dương Nhị Lang), `47164` (Xi Vưu), `27002` (Thái Thượng Lão Quân) | • **Thùng Báu Vật Đỉnh Cửu Thiên (`Step 19`, `Quest 12319`)**: yêu cầu giải tán đội ngũ nhận từng người (`Talk 53738`), trao ngẫu nhiên 1 món trong bộ trang bị **Cứu Kim / Cứu Pháp** (`62705..62712`: *Cứu Kim Khôi/Giáp/Oản/Lý*, *Cứu Pháp Quan/Bào/Thủ/Ủng*) hoặc `63105` (*Thiên Nguyên Thạch* x1/x2/x3), đồng thời **thu phục võ tướng `14735` (Hầu Tôn)** (`RewardPet 14735`) và đặt `Quest 12310 = 1`! |

## 4. Bản đồ Bộ nhớ `LocalPlayer` (`**gvar_007DA7BC`) Liên quan đến Sự kiện (Q5 - Level A)

Con trỏ toàn cục `gvar_007DA7BC` trỏ tới con trỏ đối tượng `LocalPlayer` (`TPlayer` / nhân vật chính). Dưới đây là toàn bộ các offset trên `LocalPlayer` được đọc hoặc ghi bởi các hàm sự kiện đặc biệt **(A)**:

| Offset | Kiểu dữ liệu | Hàm / Case truy cập | Ý nghĩa & Vai trò trong Sự kiện đặc biệt (Level A) |
| :--- | :--- | :--- | :--- |
| `+0x04` | `u32` | `FUN_007895DB` (`3F 02`), `FUN_0078A32C` (`C7 04`) | **Player ID (UID)** của người chơi hiện tại, được đính kèm trực tiếp vào gói báo danh chiến dịch `3F 02` và gói chat GM `C7 04 / C7 06` |
| `+0x294` | `Pointer` | `case_009`, `case_015`, `case_020` | Con trỏ tới mảng/đối tượng Võ tướng (Pet) đang xuất chiến hoặc trong danh sách mang theo |
| `+0x4D8` | `u8` | `FUN_00789722` (`C→S 40 01`) | Mã hành động chiến thuyền (loại đạn pháo / kỹ năng tàu) trong Thủy Chiến |
| `+0x4D9` | `u32` | `FUN_00789722` (`C→S 40 01`) | ID mục tiêu (`targetId`) mà chiến thuyền của người chơi nhắm bắn trong Thủy Chiến |
| `+0x4DD` | `u16` | `FUN_00789722` (`C→S 40 01`) | Tham số phụ / vị trí ô đạn chiến thuyền khi gửi gói `C→S 40 01` |
| `+0x628` | `u16` | `case_018` (`S→C 14 22`) | Tham số trạng thái sự kiện 1 (gán từ `Word LE` đầu tiên của `S→C 14 22`) |
| `+0x62A` | `u16` | `case_018` (`S→C 14 22`) | Tham số trạng thái sự kiện 2 (gán từ `Word LE` thứ hai của `S→C 14 22`) |
| `+0x62C` | `u8` | `case_018` (`S→C 14 21`) | Cờ kích hoạt sự kiện (`1` nếu `RP[1] == 1`, ngược lại `0`) |
| `+0x63A` | `u16` | Toàn bộ 39 hàm kiểm tra map | **Map ID hiện tại (`currentMapId`)** — trung tâm điều phối hành vi bản đồ sự kiện, hiển thị tàu Thủy Chiến, chặn cửa trục, miễn phí nhà trọ và bật phím Đua Thuyền Rồng |
| `+0x653` | `u8` | `case_018` (`S→C 14 08..0D`) | Cờ khóa điều khiển nhân vật khi đang chạy cắt cảnh / hội thoại kịch bản `eve.emg` |
| `+0x9E0` | `Struct[25]` | `FUN_0076C850` (`17 64`), `FUN_0076CA80` (`17 65`) | **Kho trang bị Chiến thuyền (`TWaterWarBankThing`)**: mảng 25 phần tử × 22 byte (`0x16`), lưu đạn pháo và đạo cụ tàu khi tham gia Thủy Chiến |
| `+0x1308` | `u8` | `case_011` (`S→C 0B 09`) | Số lượt sử dụng đạn tự động công kích còn lại (`"Sử dụng đạn tự động công kích X lần"`) |
| `+0x142C` | `u8` | `case_037` (`S→C 29 32`) | Trạng thái tham chiến / phe phái hoặc khí giới trong Công Thành Chiến |
| `+0x1430` | `u32` | `case_037` (`S→C 29 32`) | Dấu thời gian `GetTickCount()` ghi nhận lúc nhận trạng thái Công Thành Chiến `29 32` |
| `+0x1435` | `u8` | `case_037` (`S→C 29 0C`) | Cờ trạng thái đặc biệt trong Công Thành Chiến (nhận trực tiếp từ `RP[1]` của `29 0C`) |
| `+0x1436` | `u8` | `case_037` (`S→C 29 0E`) | Cờ trạng thái phụ Công Thành Chiến (bị xóa về `0` khi nhận `29 0E`) |
| `+0x1438` | `u64 / Double` | `case_041` (`S→C 0x2D`) | Thông tin thời gian / điểm tích lũy thẻ nạp hoặc sự kiện bốc thăm |
| `+0x144C` | `u32` | `FUN_00788A6A` (`C→S 2D 03`) | Mã số lựa chọn gửi đi trong gói sự kiện `2D 03` (kèm byte tại `+0x1450`) |
| `+0x1458` | `u8` | `case_027` (`S→C 0x1F`) | Cờ miễn phí nghỉ ngơi tại nhà trọ (nếu `!= 0` thì chi phí ngủ trọ bằng `0`) |

## 5. Đặc tả Định dạng File `.dat` và Bộ nạp Dữ liệu trên Client (Q6 - Level A)

Toàn bộ các file `.dat` trong thư mục `\Data\` của client đều sử dụng chung cơ chế **khóa XOR 2 bản ghi cuối file** (bản ghi chẵn XOR với `keyA = record[nRec - 2]`, bản ghi lẻ XOR với `keyB = record[nRec - 1]`), nhưng khác nhau về kích thước bản ghi (`record_size`), số bản ghi đệm ở đầu/cuối và phép biến đổi số học sau khi XOR **(A)**:

| Tên File | Hàm Loader / Giải mã | Kích thước 1 bản ghi | Số lượng bản ghi | Thuật toán giải mã & Cấu trúc trường dữ liệu (Level A) |
| :--- | :--- | :--- | :--- | :--- |
| `\Data\Festival.Dat` | `FUN_0051E00C` / `FUN_0051DECC` | `2031 B` (`0x7EF`) | `nRec - 3` (7 bản ghi trong file 10 rec) | Bỏ `rec[0]`, dùng `rec[nRec-2]` & `rec[nRec-1]` làm khóa XOR chẵn/lẻ. Cấu trúc: `+0x00` `double` (`TDateTime`), `+0x08` `u16` mã hiệu (`0xB4C2`), `+0x0A` `ShortString[20]` Tên lễ hội, `+0x1F` `ShortString[40]` Thời gian, tiếp theo là 3 chuỗi dài (Nguồn gốc, Hoạt động, Vật phẩm). |
| `\Data\Scene.Dat` | `FUN_00535600` / `FUN_0053542C` | `134 B` (`0x86`) | `7480` bản ghi hợp lệ (trên tổng 7481) | Sau khi XOR khóa chẵn/lẻ: `+0x00` là `u16` `mapId = ((raw_u16 ^ 0xEA6C) - 9) & 0xFFFF`; `+0x02` là `u8` `kind` (`0`=thường, `2`=Đấu trường PK, `6`=Thành Công Thành Chiến, `8`=Thủy Chiến); `+0x03` là `ShortString[20]` Tên bản đồ (VISCII). |
| `\Data\Npc.dat` | `FUN_00623F28` | `92 B` (`0x5C`) | `6659` NPC | `+0x00` là `ShortString[14]` Tên NPC bị đảo ngược thứ tự ký tự (reversed VISCII); `+0x10` là `u16` `npcId = ((raw_u16 ^ 0x5209) - 1) & 0xFFFF`; kèm các trường Hệ ngũ hành, Level, HP, SP, AGI, ATK, DEF. |
| `\Data\Warp.Dat` | `FUN_005BF788` / `FUN_005BF98C` | `23 B` (`0x17`) | `nRec - 1` (lưu tại `self+0x1704`) | Giải mã bằng `FUN_005BF98C`, lưu bảng ánh xạ cổng dịch chuyển liên bản đồ (`srcMapId`, `warpId`, `dstMapId`, `dstX`, `dstY`). |
| `\Data\SceneSet.Dat` | `FUN_00525720` | `5 B` (`0x05`) | `nRec - 1` (`205` bản ghi trong file 1030B) | Lưu tại mảng `self+0x04` (tối đa 251 bản ghi = `0x4E7` byte, số lượng đặt tại `self+0x4EC`). Cấu trúc 5 byte thiết lập hiệu ứng/thuộc tính cụm cảnh. |
| `\Data\GS.Dat` | `FUN_00524DC0` | `11 B` (`0x0B`) | `nRec - 1` (lưu tại `self+0x2B828`) | Nạp vào bộ đệm `self+0x04` (tối đa `16364` bản ghi × 11 byte = `0x2B824` byte). |
| `\Data\BlissBag.Dat` | `FUN_004DF424` / `FUN_004DEF9C` | `190 B` (`0xBE`) | `nRec - 3` (lưu tại `self+0x08`) | *(Lưu ý đính chính `DATA_FILES.md`: Hàm `FUN_004DF424` mở `\Data\BlissBag.Dat`, không phải `Rank.Dat`)* **(A)**. Mỗi bản ghi 190 byte định nghĩa danh sách vật phẩm và tỷ lệ rơi của Túi Phúc / Vòng quay. |
| `\Data\Rank.Dat` | Dữ liệu tĩnh 43B/rec | `43 B` (`0x2B`) | `42` cấp bậc (file `1849 B` = `43 × 43`) | Chứa 42 danh hiệu quân hàm triều đình (`Giáp binh`, `Hổ bôn binh`, `Trường Hổ bôn binh`, `Huyện úy`, `Kỳ Đô úy`, `Võ vệ đô úy`... khớp với bảng danh hiệu chiến trường tại `lit_004cb698_tiger_npcs.hex`). |

## 6. Giới hạn Kiểm tra phía Client & Các Quy tắc Bắt buộc trên Server Rust (Q8 - Level A/B)

Qua việc rà soát toàn bộ 66 nhánh nhận S→C và 85 hàm tạo gói C→S, có thể khẳng định **client thực hiện rất ít kiểm tra bảo mật mà ủy thác gần như toàn bộ quyền phán quyết cho máy chủ** **(A)**:

1. **Những gì Client CÓ tự kiểm tra trước khi gửi gói tin (Client-side Checks) (Level A)**:
   - **Chặn tổ đội qua cửa trục trên Chiến thuyền (`FUN_005F54E0`)**: Nếu `mapId` thuộc `49401..49444` và là map chẵn, người chơi đang có tổ đội sẽ bị chặn ngay tại client với thông báo `0x005F5644` mà không gửi gói dịch chuyển.
   - **Kiểm tra nhịp phím Trái/Phải trong Đua Thuyền Rồng (`FUN_0051BF04`)**: Chỉ khi `mapId` thuộc `60510..60521`, trạng thái đua `+0x49 == 3`, và người chơi nhấn luân phiên đúng cặp phím Trái → Phải thì client mới gọi `FUN_00789F85` gửi `C→S 46 01`. Tuy nhiên, client **không** giới hạn tần số gửi gói `46 01` nếu có công cụ giả lập phím hoặc bơm gói tin.
   - **Chặn đi xuyên cổng thành trong Công Thành Chiến (`FUN_00733DC0`)**: Kiểm tra va chạm tọa độ với thực thể cổng thành dựa trên trạng thái cổng mà server đã báo về.
   - **Kiểm tra số tiền tối thiểu trong Cờ tự mò (`0x0053FBAC`)**: Yêu cầu tối thiểu 30 lần tiền cược trước khi cho bấm nút gia nhập trên UI.

2. **Những gì Server Rust BẮT BUỘC phải tự xác thực (Server-side Enforcement) (Level A/B)**:
   - **Xác thực `QuestStep`, `HasItem`, `Gold>=` và `PartySize` của kịch bản `eve.emg`**: Khi client gửi `C→S 14 04`, `14 06`, hoặc `14 09`, gói tin chỉ chứa mã lựa chọn 1 byte hoặc ID cổng 2 byte **(A)**. Server phải tự đối chiếu bảng `eve_logic` của `mapId` hiện tại để kiểm tra nhân vật có thực sự đang ở bước `QuestStep` đó không, có đủ vàng/vật phẩm không và có đủ 5 thành viên tổ đội (như trong Giải cứu Nhị Kiều "2K" hoặc Lôi Đài Đoàn P) hay không **(A)**.
   - **Chống gian lận tốc độ Đua Thuyền Rồng (`C→S 46 01`)**: Vì gói `46 01` chỉ gồm đúng 2 byte `[0x46, 0x01]` không kèm timestamp hay chữ ký **(A)**, server phải kiểm tra người chơi có đang ở map `60510..60521` trong trạng thái đua hay không, đồng thời áp đặt ngưỡng tốc độ tối đa (rate-limit số gói `46 01` mỗi giây) **(B)**.
   - **Xác thực thời gian và tư cách tham gia Sự kiện (`0x29`, `0x3F`, `0x40`)**: Trong gói `C→S 3F 02` (`[3F, 02, mode, playerId]`) và `C→S C7 04 / 06`, client tự điền `LocalPlayer+0x04` (`playerId`) vào payload **(A)**. Server Rust tuyệt đối không được tin trường `playerId` do client gửi lên mà phải lấy `playerId` từ phiên kết nối (session) đã xác thực, đồng thời kiểm tra khung giờ sự kiện (ví dụ: 19:00–21:00 cho Lôi Đài, 20:00 cho Công Thành Chiến) và tư cách Quân đoàn trước khi chấp nhận lệnh **(A/B)**.
   - **Kiểm soát bộ đếm trả lời sai và thời gian trả lời Bắc Tinh Quân (`10 01`)**: Client chỉ hiển thị câu hỏi và gửi chỉ số đáp án `1..4` qua `[10, 01, ansIdx:u32 LE]` **(A)**. Server phải tự đếm số lần sai (đến lần thứ 3 thì gửi `10 02 03` kèm `00 02` ngắt kết nối hoặc `10 02 09` bắt giam) và kiểm tra timeout nếu người chơi không trả lời **(A)**.
