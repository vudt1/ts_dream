# BÁO CÁO AUDIT & NGHIÊN CỨU: SỰ KIỆN THEO MÙA / LỄ HỘI (CLIENT `aLogin.exe` ↔ RUST SERVER)

> **Ngày lập:** 2026-11-18  
> **Phạm vi:** Hạng mục *"Sự kiện theo mùa / lễ hội"* trong [`README.md`](README.md), đối chiếu trực tiếp mã dịch ngược `client_pseudo_c/` (`aLogin.exe`), dữ liệu đã giải mã trong `.scratch/special-event-refactor/`, và mã nguồn server Rust hiện tại (`src/`, `templates/dashboard.html`, `migrations/`, `tests/`).  
> **Nguyên tắc:** Mọi kết luận đều trích dẫn bằng chứng trực tiếp từ mã C/ASM dịch ngược, dữ liệu đã giải mã hoặc source Rust; ghi rõ những phần chưa có dump hoặc chưa cài đặt.

---

## 1. Tóm tắt kết luận (Executive Summary)

1. **Cách kích hoạt trong game (Client thụ động 100%):**
   - Client `aLogin.exe` **không** tự đọc đồng hồ máy người chơi (`GetLocalTime`/`GetSystemTime` không tồn tại trong `client_pseudo_c/`; `Now()` chỉ dùng làm delta timer cục bộ hoặc so sánh mốc server gửi).
   - Sự kiện theo mùa / lễ hội trong TS Online được Server điều phối thông qua **5 kênh phối hợp**:
     1. **Kênh `EVE` (`eve.emg` / `OP 0x14`, `0x18`) — Gameplay cốt lõi:** Spawn NPC lễ hội, mở cổng warp vào map sự kiện, xử lý hội thoại/đổi quà/đánh quái/boss theo `QuestStep`, và kích hoạt hiệu ứng thời tiết/tuyết rơi (`FUN_006221e8` đọc `SceneSet.Dat` theo `mapId` + `QuestStep`).
     2. **Kênh `S→C OP 0x17` (`FUN_007902fb`) — Thông báo mùa & Hiệu ứng pháo hoa:**
        - **`0x17 0x39 <sub2>`** (`FUN_00722cd4`) xử lý **66 mã thông báo** (`0x01..0x0D`, `0x15..0x4A`, không liên tục `1..74`) ra khung chat hệ thống / marquee / thoại NPC, kèm 3 nhánh có side-effect (`0x0A`, `0x1F`, `0x34`).
        - **`0x17 0x46`** (`FUN_0062024c`) điều khiển hạt pháo hoa `TFireLight` (`TFT_EffectManage`).
     3. **Kênh `S→C OP 0x24 0x19 <u32 LE>` (`FUN_0079346b` → `FUN_0051d914`) — Bảng lore `TFestival` (`Festival.Dat`):**
        - **Đính chính quan trọng:** Trước đây [`README.md`](README.md#L103) gắn `TFestival` vào `OP 0x17 → FUN_00722CD4`, nhưng mã ASM/C chứng minh `TFestival` và form `TMR_FestivalInfoForm` chỉ được mở bởi **`OP 0x24 sub 0x19`**, **không phải** `OP 0x17`.
     4. **Kênh `S→C/C→S OP 0x46` (`TLH_DragonBoat`) — Đua thuyền rồng Tết Đoan Ngọ:** Gắn trực tiếp với Tết Đoan Ngọ (check `mapId == 12000` *Giới Kiều / Ghi danh Đoan Ngọ*, chuỗi thông báo *"chúc mọi người Tết Đoan Ngọ vui vẻ"*).
     5. **Kênh phụ (`OP 0x14`, `OP 0x10`, `OP 0x48`, `OP 0xC7`):** Thông báo Nguyên Tiêu / Trung Thu / Thất Tịch / Đoan Ngọ (`OP 0x14`), đố đèn Nguyên Tiêu (`OP 0x10` `TWF_Day55QuestManage`), Vòng quay BĐY Tết Âm Lịch (`OP 0x48`), broadcast GM (`OP 0xC7`).

2. **Server Rust hiện tại đã hỗ trợ sự kiện theo mùa / lễ hội chưa?**
   - **Trả lời ngắn gọn: CHƯA — mới có khung dữ liệu tĩnh & stub rời rạc, chưa vận hành được trong game.**
   - Trên [`templates/dashboard.html`](../../templates/dashboard.html#L315-L351), tab **Special Events** lấy danh sách 9 sự kiện từ [`EventLifecycleManager::new`](../../src/server/events.rs#L162-L172), trong đó **đã có** mục `seasonal_festival` (`"Lễ Hội Theo Mùa"`).
   - Tuy nhiên, toàn bộ `src/` chỉ mới kiểm tra `is_event_active` cho duy nhất **`north_star`** ([`movement.rs:281`](../../src/server/handlers/movement.rs#L281)). Nút **Start / Stop** của `seasonal_festival` trên Dashboard hiện là **no-op với gameplay** (chỉ đổi trạng thái RAM và ghi bảng `admin_audit_log`).
   - [`FestivalDatLoader`](../../src/data/loaders/festival.rs#L45-L118) đã nạp được 7 bản ghi `Data/Festival.Dat` vào `GameData.festivals`, nhưng **không có bất kỳ caller nào trong `src/` đọc `data.festivals`**.
   - [`festival::build_seasonal_announcement_raw`](../../src/server/handlers/festival.rs#L32-L34) (`S→C 17 39`) cũng **không có caller nào trong `src/`** (chỉ được gọi trong unit test).
   - **Phát hiện bug Wire Parity nghiêm trọng ở `S→C 24 19`:** [`festival::handle_festival`](../../src/server/handlers/festival.rs#L42-L47) đang trả cứng `FESTIVAL_PARITY_DATE_DWORD = 0x0121EAC6` (`19000110`), do tài liệu trước đó hiểu nhầm `FUN_0077ed68` là hàm chuyển `YYYYMMDD` sang `TDateTime` (`11.0`). Thực tế ASM chứng minh `FUN_0077ed68` chỉ đọc `u32 LE` và `FUN_0051d914` ép thẳng sang `(double)u32` (`19000110.0`), nên so sánh với `11.0` trong `Data/Festival.Dat` **luôn thất bại** và client **không mở form**.

---

## 2. Cơ chế kích hoạt & xử lý phía Client (`client_pseudo_c/`)

### 2.1. Bằng chứng Client không tự kích hoạt theo đồng hồ máy
- Quét toàn bộ `client_pseudo_c/` không tồn tại bất kỳ lời gọi `GetLocalTime`, `GetSystemTime`, `DecodeDate`, `DecodeTime`.
- Hàm `Now()` chỉ được sử dụng cho bộ đếm delta cục bộ (animation/cooldown đua thuyền rồng tại `0051b174`, `0051b5b4`, `0051bda0`; pháo hoa tại `0062024c`) hoặc so sánh với mốc `TDateTime` do server gửi xuống (`0060e2a8_FUN_0060e2a8.c:43-49`).
- `FormatDateTime` (`007938c3:129,277`, `0060e2a8:46`) và `StrToDate` (`007a1218:98,135`) chỉ dùng để định dạng ngày công thành chiến / hạn tài khoản từ dữ liệu server.

### 2.2. Luồng Dispatcher tổng quát S→C trong `aLogin.exe`
1. Tại `TForm1.CY_DelRevQueue` ([`00516158_TForm1.CY_DelRevQueue.c:74-93`](../../client_pseudo_c/00516158_TForm1.CY_DelRevQueue.c#L74-L93), [`.asm.txt:42-73`](../../client_pseudo_c/00516158_TForm1.CY_DelRevQueue.asm.txt#L42-L73)): tách byte đầu `local_c[0]` làm `main_opcode` (`DL`) và phần còn lại `_LStrCopy(local_c, 2, len - 1, ...)` làm `payload` (`ECX`), sau đó gọi `FUN_0078a89c`.
2. Tại `FUN_0078a89c` ([`0078a89c_FUN_0078a89c.asm.txt:28-32`](../../client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt#L28-L32)): kiểm tra `main_opcode <= 0xC7`, tra bảng 1 byte tại `0x78A8EE` lấy index case rồi nhảy qua bảng `dword` tại `0x78A9B6`:
   - **`OP 0x17`** → index `0x14` (`20`) → `0x007902FB` (`FUN_007902fb`, [`case_020_007902FB_FUN_007902fb.c`](../../client_pseudo_c/case_020_007902FB_FUN_007902fb.c)).
   - **`OP 0x24`** → index `0x20` (`32`) → `0x0079346B` (`FUN_0079346b`, [`case_032_0079346B_FUN_0079346b.c`](../../client_pseudo_c/case_032_0079346B_FUN_0079346b.c)).

---

### 2.3. Kênh `S→C OP 0x17`: Thông báo mùa (`0x17 0x39`) & Pháo hoa (`0x17 0x46`)

`OP 0x17` (`FUN_007902fb`) là dispatcher tổng hợp (`case 0x02..0x75`) chủ yếu cho túi đồ/trang bị/kho/pet, đồng thời chứa 2 sub-opcode liên quan trực tiếp tới lễ hội:

#### a) `0x17 0x46` và `0x17 0x48` — Hiệu ứng pháo hoa (`TFT_EffectManage`)
- Trong [`007902fb_FUN_007902fb.c:463-474`](../../client_pseudo_c/007902fb_FUN_007902fb.c#L463-L474):
  - `case 0x46` gọi `FUN_0062024c(*(uint *)gvar_007DA34C, payload)` tạo hạt pháo hoa **`TFireLight`** (`VMT_61E28C_TFireLight`, [`0062024c_FUN_0062024c.c:109, 252`](../../client_pseudo_c/0062024c_FUN_0062024c.c#L109)) với `sub = 1/3/4`; xử lý `FUN_0062091c` với `sub = 2`; và ở nhánh `sub = 0x34 0x03` (`0062024c:327-339`) bật cờ `*(param_1 + 0x40) = 1`, lưu `Now()` vào `+0x48`, gán `*gvar_007DA2AC = gvar_006208AC`.
  - `case 0x48` gọi `FUN_00621938(*(uint *)gvar_007DA34C, payload)`.

#### b) `0x17 0x39 <sub2> [...]` — `FUN_00722cd4` (66 nhánh thông báo & điều khiển)
- Tại `0x00790AE4` ([`case_020_007902FB_FUN_007902fb.c:248-250`](../../client_pseudo_c/case_020_007902FB_FUN_007902fb.c#L248-L250)), `case 0x39` gọi `FUN_00722cd4(*(undefined4 *)gvar_007D9C48, payload)`.
- [`00722cd4_FUN_00722cd4.c:109-119`](../../client_pseudo_c/00722cd4_FUN_00722cd4.c#L109-L119) đọc **`payload[1]`** (`sub2` trong dải `0x01..0x4A`; **vắng mặt các mã `0x0E..0x14`**, nên tổng cộng có **66 nhánh**, không phải 74 mã liên tục). Một số nhánh đọc thêm `payload[2]` hoặc dữ liệu theo sau.
- Hàm này **không** gọi `TFestival` hay `FUN_0051d914`, mà xuất thông báo qua:
  - Khung chat hệ thống: `FUN_007ab870(*(int *)gvar_007DA1B0, ...)` (`TTalkMsgForm`).
  - Thông báo nổi / marquee: `vtable + 0x90` trên `TSe_TalkMsgFormPlus` (`gvar_007DA084`, timeout `2000` hoặc `0x4B0`).
  - Hộp thoại NPC: `FUN_0063c1d0` + `vtable + 0x90` trên `TSe_TalkMsgFormPlus` (`gvar_007D9D6C`).

| Nhóm trong `FUN_00722cd4` | Các giá trị `sub2` (`payload[1]`) | Nội dung & Hành vi thực tế |
|---|---|---|
| **Tết Nguyên Đán / Lì xì / Niên Thú** | `0x01, 0x02, 0x08..0x0D, 0x44..0x47` | Phát lì xì, đếm ngược, Niên thú Thôn Thực (`0x44`, chuỗi `0x0072690C`), thiệp năm mới, nấu ăn Tết Âm Lịch, Tà Lão Đầu đổi Huy Xuân. Riêng **`0x0A`** (`00722cd4:156-160`) gọi **`FUN_00620b20()`** đặt `DAT_0096bdd8 + 0x28 = 0`, `+ 0x30 = 1`. |
| **Tết Đoan Ngọ / Tháp Lôi Phong** | `0x03, 0x04, 0x33, 0x42` | Mừng Đoan Ngọ ăn bánh Chưng / tập hợp linh châu (`0x03`, chuỗi `0x00724424`), Tháp Lôi Phong (`0x33` sub 2, chuỗi `0x007254C0`), Hòa Sơn đạo nhân / Hùng Hoàng (`0x42`). |
| **Halloween & Quỷ thành Phong Đô** | `0x05, 0x06, 0x1B..0x1D` | Halloween 31/10 Vua Quỷ (`0x05`, `0x06` chuỗi `0x007245C0`), Quỷ thành Phong Đô (`0x1B..0x1D`). |
| **Giáng Sinh / Noel** | `0x15, 0x16, 0x29..0x2B, 0x3B, 0x43` | Noel Lư Long Khẩu, Christmas Poke, quà Giáng Sinh, cơn mưa lễ vật thôn Noel. |
| **Trung Thu / Nướng thịt** | `0x1E..0x20, 0x25, 0x39, 0x3A` | Hội nướng thịt Trung Thu, thưởng nguyệt / Hằng Nga / Ngọc Thố. Riêng **`0x1F`** (`00722cd4:213-217`) gọi **`FUN_0074927c(LocalPlayer)`** tắt cờ `LocalPlayer+0x1478` và gửi C→S `0x17/0x35`. |
| **Tết Nguyên Tiêu** | `0x2C, 0x3D, 0x3F, 0x49` | Nguyên Tiêu, Cự long nguyên thần, Lồng đèn tề tụ, gấp đôi EXP Nguyên Tiêu. |
| **Lễ Tình Nhân / Thất Tịch** | `0x34, 0x35` | Vườn Địa Đàng / Lễ Tình Nhân (`0x34`), Lễ Thất Tịch (`0x35`, chuỗi `0x007257AC`). Riêng **`0x34` sub 2** tắt cờ `*(gvar_007DA34C + 0x40) = 0`; **`0x34` sub 3** gọi `FUN_0062024c` bắn pháo hoa `TFireLight`. |
| **Phi lễ hội (sự kiện/gameplay khác)** | `0x07, 0x17..0x1A, 0x21..0x24, 0x26..0x28, 0x2D..0x32, 0x36..0x38, 0x3C, 0x3E, 0x40, 0x41, 0x48, 0x4A` | Kết hợp trân châu (`0x07`), đạt nhân Giới Cảo/Hương Cảm, lỗi điều kiện nướng thịt/dùng item, Thiên Châu, Thủy chiến (`0x2D..0x31`), Lôi đài Tiên đấu (`0x32`), Phúc thần (`0x36`), Công thành quân đoàn (`0x37`), thoại ngẫu nhiên (`0x38, 0x3C`), sưu tầm chữ năm Hợi (`0x3E`), máy chủ bận (`0x40`), nhận nhiệm vụ (`0x41`), Phong Ma Trận (`0x48`), Tiết Khánh BĐY (`0x4A`). |

---

### 2.4. Kênh `S→C OP 0x24 0x19`: Cơ chế thật sự của `TFestival` & `Festival.Dat`

#### a) Vòng đời khởi tạo và nạp `Festival.Dat`
- Trong `TForm1.FormCreate` ([`0050a4a0_TForm1.FormCreate.c:604-605`](../../client_pseudo_c/0050a4a0_TForm1.FormCreate.c#L604-L605)), client tạo đối tượng `TFestival` (`VMT 0x0051DE74`) lưu vào `DAT_009264e0` (`gvar_007DA5D4`) và gọi `FUN_0051e00c` (`TFestival.Load`) → `FUN_0051decc` (`TFestival.Decrypt`) giải mã `\Data\Festival.Dat` (10 bản ghi × `0x7EF` = 2031 byte; `count = nRec - 3 = 7`, bản ghi `1..7` XOR luân phiên với 2 bản ghi khóa cuối `0x00926FE0` và `0x009277CF`).
- Trong `FUN_0051189c` ([`0051189c_FUN_0051189c.c:1684-1686`](../../client_pseudo_c/0051189c_FUN_0051189c.c#L1684-L1686)), client tạo `TMR_FestivalInfoForm` (`VMT 0x0051D3B4`) lưu vào `gvar_007DA1EC`.

#### b) Giải phẫu chính xác luồng xử lý `S→C 24 19 <4B LE>` tại `FUN_0051d914`
1. `FUN_0079346b` (`OP 0x24`, [`case_032_0079346B_FUN_0079346b.c:154-155`](../../client_pseudo_c/case_032_0079346B_FUN_0079346b.c#L154-L155)) gặp `case 0x19` và gọi `FUN_0051d914(*(int **)gvar_007DA1EC, payload)`.
2. `FUN_0051d914` ([`0051d914_FUN_0051d914.c:66-71`](../../client_pseudo_c/0051d914_FUN_0051d914.c#L66-L71), [`.asm.txt:19-44`](../../client_pseudo_c/0051d914_FUN_0051d914.asm.txt#L19-L44)):
   - Cắt 4 byte ngay sau `0x19`: `_LStrCopy(param_2, 2, 4, local_1c)` (1-based index `2`, length `4`).
   - Gọi `FUN_0077ed68(*(gvar_007D9D30), local_1c[0])`. Kiểm chứng [`0077ed68_FUN_0077ed68.c:155-216`](../../client_pseudo_c/0077ed68_FUN_0077ed68.c#L155-L216): hàm này **hoàn toàn không tra bảng hay gọi `EncodeDate`**, mà chỉ ghép 4 byte thành số nguyên **`u32 LE`** (`b0 + b1*0x100 + b2*0x10000 + b3*0x1000000`).
   - Ngay sau đó, ASM dùng `FILD qword ptr [EBP - 0x20]` / `FSTP double ptr [EAX + 0x150]` ép thẳng giá trị `u32` đó thành số thực IEEE-754 **`(double)u32`**.
3. Gọi `FUN_0051e2dc(TFestival, ..., (double)u32)` ([`0051e2dc_FUN_0051e2dc.asm.txt:14-37`](../../client_pseudo_c/0051e2dc_FUN_0051e2dc.asm.txt#L14-L37)):
   - Duyệt `i = 1 .. *(word*)(self + 0xB5FA6)`, so sánh bằng tuyệt đối (`FLD`/`FCOMP`/`SAHF`/`JNZ`) trường `*(double*)(self + 4 + i*0x7EF)` (offset `+0x0000` của record) với `(double)u32`. Trả về index `i` đầu tiên khớp, hoặc `0` nếu không khớp.
   - Nếu trả về `0`, `FUN_0051d914` nhảy qua toàn bộ phần dựng UI (`JZ 0x0051daa0`) và **không hiện gì cả**.
4. Nếu tìm thấy `local_e = i != 0` (`0051d914:72-100`):
   - Ẩn 2 nút mũi tên `Arrow_R2` (`local_8[0x4F]`) và `Arrow_L2` (`local_8[0x4E]`) qua `FUN_007b0094(ctrl, 0)` ([`007b0094_FUN_007b0094.c:918-923`](../../client_pseudo_c/007b0094_FUN_007b0094.c#L918-L923)).
   - Xóa text khung nội dung `local_8[0x50]` (`FUN_007ba180`), đọc tiêu đề ShortString tại `TFestival + 0xE + i*0x7EF` (tức offset `+0x000A` trong record) vào `local_8 + 0x53` (`+0x14C`).
   - Gọi `FUN_0051e358` đọc **đoạn văn 1** (`+0x001F` trong record) đổ vào `local_8[0x50]` qua `FUN_007b8060`. Đặt `total_pages` (`+0x148`) = `1`, `current_page` (`+0x149`) = `1`.
   - Gọi `FUN_0051e3b8` đọc **đoạn văn 2** (`+0x0407` trong record). Nếu khác rỗng (`local_2c != 0`), đặt `total_pages = 2` và hiện lại cả 2 nút `Arrow_R2` (`0x4F`) và `Arrow_L2` (`0x4E`).
   - Gọi virtual `Show` (`(**(code **)(*local_8 + 0x20))()`). Khi người chơi bấm `Arrow_L2` (`FUN_0051d8e4`) hoặc `Arrow_R2` (`0051d8db` gọi `FUN_0051dd3c`), client chỉ chuyển trang `1 ↔ 2` cục bộ. **Không có nút nào trên `TMR_FestivalInfoForm` gửi packet C→S.**

#### c) Giải đáp dứt điểm các mục chưa xác minh trong `FESTIVAL_DAT_FORMAT.md` §6
- **Vì sao `double @+0x0000` trong `Data/Festival.Dat` hiện là `11.0` ở cả 7 bản ghi?** Không có bất kỳ hàm nào trong client ghi đè trường này lúc runtime hay truyền `TDateTime` vào `FUN_0051e2dc`. Do `Data/Festival.Dat` đang gán chung `11.0` cho cả 7 bản ghi, chỉ khi server gửi `u32 = 11` (`0B 00 00 00`) mới mở được `rec01` (*Tết Nguyên Đán*); các bản ghi `2..7` **không thể truy cập được** với file `Festival.Dat` hiện tại nếu không cấp lại `double @+0x0000` phân biệt cho từng bản ghi (hoặc build lại `Festival.Dat` với key `u32` riêng cho mỗi lễ hội).
- **Ý nghĩa `word 0xB4C2` tại offset `+0x0008`:** Không có bất kỳ hàm nào trong `client_pseudo_c/` đọc hoặc ghi offset `+0x0008` của bản ghi `TFestival` (`FUN_0051d914` nhảy từ `+0x0000` sang thẳng `+0x000A` cho tiêu đề). Đây là trường dư/unused ở phía client `aLogin.exe`.
- **Phần thiếu dump (Missing Dumps):** Thân hàm `LAB_0051d8ac` (`0x0051d8ac..0x0051d8e3`, handler `OnClick` của `Arrow_R2`) chưa có file `.c`/`.asm.txt` riêng trong `client_pseudo_c/`, nhưng header của [`0051dd3c_FUN_0051dd3c.c:18`](../../client_pseudo_c/0051dd3c_FUN_0051dd3c.c#L18) xác nhận `0051d8db -> 0051dd3c [UNCONDITIONAL_CALL]`.

#### d) Kiểm tra C→S cho `OP 0x17` và `OP 0x24`
- `TFestival` và `TMR_FestivalInfoForm` **không gửi bất kỳ gói tin C→S nào** (không gọi `FUN_0077f414` hay `0x0051633c`).
- C→S `OP 0x17` tồn tại ở `0x007833F0` ([`cs_op17_dispatch.asm.txt`](../../client_pseudo_c/cs_op17_dispatch.asm.txt)) với 44 sub-opcode (`0x02..0x04`, `0x0A..0x14`, `0x1E..0x3C`) phục vụ túi đồ, sạp hàng, kho, hợp thành, thành chiến — **không liên quan tới `TFestival`**.

---

### 2.5. Kênh `EVE`, Hiệu ứng môi trường (`SceneSet.Dat`) & các kênh lễ hội khác

Nội dung 7 bản ghi giải mã trong `Festival.Dat` (`festival_dat_decrypted/rec01.bin..rec07.bin`) **100% là văn bản giới thiệu văn hóa/lịch sử**, không chứa quy tắc gameplay. Gameplay thực tế của các lễ hội trong TS Online được vận hành qua:

1. **Kịch bản `EVE` (`data_files/eve/eve_logic.tsv`, `eve_npcs.tsv`, `eve_fights.tsv`) + Map/NPC/Item tĩnh:**
   - **Tết Nguyên Đán / Niên Thú:** NPC `1604` Tà Lão Đầu, `1659` Tà Lão Bác, `3998`/`4962` Thiên Niên Thú, `4945` Thủ binh N.T.; item `46335`, `46338` Bao lì xì năm mới (Scene `55002` `DialogChoice`).
   - **Giáng Sinh / Noel:** Map `12421` (*Đất tuyết Lô Long / Bộ lạc Giáng Sinh / Thôn Giáng Sinh*), `54827..54831`; NPC `2015`, `2016`, `3865..3873`, `3896..3900`, `3926..3928`, `3991..4003`; Scene `12421` (`QuestStep 10044, 11542`, `eve_fights.tsv:201` đấu `14120/14121/17001`), `54827..54831` (`QuestStep 12174, 12175, 12352, 12353`; item `46193..46296`, `29393..29395`).
   - **Thất Tịch / Trung Thu / Tháp Lôi Phong:** Map `12001` (*NguyênTiêu nhạc viên / Tháp Lôi Phong / Rừng đào Thất Tịch*), `10995`, `12941..12943`, `54811..54812`, `54821..54825`; NPC `1825`/`5778` Nguyệt Lão, `1879` Ngưu Lang, `3453`/`5653` Chức Nữ, `3468`/`5668` Hằng Nga, `3886` Ngọc Thố, `3962`/`5994` Lôi ph thủbinh; Scene `10995`, `10996`, `12001` (`QuestStep 10786`, item `30048..30053`, `23111`), Scene `12941..12943` (`QuestStep 11746, 11747`).
   - **Halloween / Quỷ Vương:** Map `57518`, `57531..57536`, `61551..61567`; NPC `1259`/`4668`/`5433`/`6554` Yêu Quỷ Vương, `1287`/`3875` Quỷ Bí Vàng; item `46073`, `46107`, `29117..29119`, `29305..29306`; Scene `57507..61441` (`QuestStep 10392, 11988, 11996`).
   - **Tết Đoan Ngọ / Linh Châu:** Item `19912`, `24050`, `26323`, `45020`, `46095..46097`, `46220`, `30044..30047` (Scene `12003`, `12711`, `31071`, `12305`...).

2. **Hiệu ứng thời tiết / tuyết rơi (`TFT_EffectManage` → `FUN_006221e8`):**
   - [`006221e8_FUN_006221e8.c`](../../client_pseudo_c/006221e8_FUN_006221e8.c) được gọi khi chuyển map (`0078bc95`, `00790ed5`, `007994a4`), đọc bảng `TFSceneSet` (`\Data\SceneSet.Dat` nạp tại `00525720`) để đặt `*gvar_007DA2AC`, và ghi đè theo Map ID cứng như vùng tuyết/Giáng Sinh (`12005`, `12421`, `10963..10964`, `10992`, `12815..12820`, `19001..19021`), Nguyên Tiêu (`54811..54812`, `49811`), hoặc map có `QuestStep` đặc biệt (`13017`, `21861..21864`, `25453`).

3. **Kênh `OP 0x46` (Đua thuyền rồng — gắn với Tết Đoan Ngọ):**
   - `FUN_007960bd` gọi `FUN_0051b600..0051cbf0` (`TLH_DragonBoat`). Các chuỗi `0x0051B9C0` & `0x0051BA64` ghi rõ *"Cuộc thi đua thuyền rồng... chúc mọi người Tết Đoan Ngọ vui vẻ"*, `0x0051BB2C` ghi lịch 3 ngày 7–9/6; `FUN_0051b2b8:33` & `FUN_0051b070:31` kiểm tra `LocalPlayer+0x63A == 12000` (*Giới Kiều / Ghi danh Đoan Ngọ*) để bật/tắt `SmallDoorLight`.

4. **Kênh `OP 0x14`, `OP 0x10`, `OP 0x48`:**
   - `OP 0x14` (`0078ec3f_FUN_0078ec3f.c`): sub `0x23`/`0x24` thông báo Tết Nguyên Tiêu, sub `0x2D`/`0x2E` thông báo Thất Tịch & Trung Thu bắt Ngọc Thố, và gọi `FUN_007485bc` (thông báo Đoan Ngọ & đố đèn Nguyên Tiêu).
   - `OP 0x10` (`FUN_007074cc`): có nhánh thưởng *"Bà Bà đoán câu đố Tết Nguyên Tiêu"* (`0x00707860`, `0x007079A0`).
   - `OP 0x48` (`00508204` sub `0x03`): thông báo *"Vòng quay BĐY Tết Âm lịch"*.

### 2.6. Đính chính sai lệch trong tài liệu bundle trước đây
| Tài liệu cũ | Nội dung cũ bị sai / chưa chuẩn | Thực tế xác minh từ `client_pseudo_c/` & dữ liệu giải mã |
|---|---|---|
| [`README.md:103`](README.md#L103) | Ghi `0x17 → FUN_00722CD4` quản lý `TFestival` | `0x17 0x39 → FUN_00722cd4` là kênh phát 66 thông báo sự kiện (+3 side-effect). `TFestival` chạy qua **`0x24 0x19` → `FUN_0051d914`**. |
| [`FESTIVAL_DAT_FORMAT.md:21,118-121`](FESTIVAL_DAT_FORMAT.md#L21) & [`SPECIAL_EVENTS_SPEC.md:16`](SPECIAL_EVENTS_SPEC.md#L16) | Ghi `FUN_0077ED68` tách `YYYYMMDD` và gọi `EncodeDate` trả về `TDateTime` | `0077ed68_FUN_0077ed68.c` chỉ đọc **`u32 LE`** 4 byte; `0051d914.asm.txt:29-34` dùng `FILD` ép thẳng thành `(double)u32`. |
| [`FESTIVAL_DAT_FORMAT.md:106`](FESTIVAL_DAT_FORMAT.md#L106) | Ghi `rec04` là *"Ngày Cà Thang Tử (Halloween)"* | Giải mã `festival_dat_decrypted/rec04.bin` là **"Ngày Cá Tháng Tư"** (1/4, vua Charles IX). |
| [`data_client_to_server.tsv:108-111`](data_client_to_server.tsv) | Ghi `OP 0x2D` là *"thông báo sự kiện theo mùa"* | `00794977_FUN_00794977.c` & chuỗi `0x00798FD8..0x00799200` xác nhận `OP 0x2D` là hệ thống **Kết hôn / Ly hôn / Lễ kim**. |

---

## 3. Kiểm toán Source Server Rust hiện tại (`src/`, `templates/`, `migrations/`)

### 3.1. Hiện trạng Tab `Special Events` trên `templates/dashboard.html`
- [`templates/dashboard.html:239, 315-351`](../../templates/dashboard.html#L315-L351) nhận dữ liệu `events` và `north_star_rate` từ `DashboardTemplate` ([`src/web/app.rs:86-102, 217-236`](../../src/web/app.rs#L217-L236)).
- Danh sách `events` được hardcode trong [`EventLifecycleManager::new`](../../src/server/events.rs#L160-L173) gồm **9 sự kiện**:

| ID (`src/server/events.rs`) | Tên hiển thị trên Dashboard | Có handler gameplay trong `src/`? | Có kiểm tra `is_event_active` trên Dashboard? |
|---|---|---|---|
| `north_star` | Vấn Đáp Bắc Tinh Quân | Có (`movement.rs:281`, `battle.rs:456-550`, `events.rs`, `server_control.rs:271-347`) | **Có** (`movement.rs:281`) |
| `continuous_40_npc` | Khiêu Chiến 40 NPC | Có (`event_npcs.rs`) | **Không** |
| `rescue_2k` | Giải Cứu Nhị Kiều 2K | Có (`event_npcs.rs`) | **Không** |
| `nine_heavens` | Chinh Phục Cửu Thiên | Có (`event_npcs.rs`) | **Không** |
| `arena` | Đấu Trường Lôi Đài | Một phần (`arena.rs`: `0x24 09/0A`, `0x25 03`; hàm `is_monday_reset_due` chưa có caller) | **Không** |
| `guild_siege` | Công Thành Chiến | Stub/Echo (`guild.rs:571-605`, trả 5 thành rỗng thay vì đọc `guild_cities`) | **Không** |
| `water_war` | Thủy Chiến Hải Tặc | Cục bộ (`guild.rs:501-517, 608-656`: `0x3F 01/02`, `0x40 01`) | **Không** |
| `dragon_boat` | Đua Thuyền Rồng | Cục bộ (`guild.rs:520-568`: `0x46 01` trên map `60510..=60521`) | **Không** |
| **`seasonal_festival`** | **Lễ Hội Theo Mùa** | **Chưa có gameplay** (chỉ có `FestivalDatLoader`, stub `0x24 0x19` và dead builder `S→C 17 39`) | **Không** (bấm Start/Stop chỉ đổi phase trong RAM + ghi `admin_audit_log`) |

- Tab **`World Bosses`** ([`templates/dashboard.html:244, 442-454`](../../templates/dashboard.html#L442-L454)) gọi [`db::domain::list_world_bosses`](../../src/db/domain.rs#L144-L152) query 2 bảng `world_bosses` và `world_boss_participants`, nhưng cả [`migrations/0001_init.sql`](../../migrations/0001_init.sql) lẫn [`migrations/0002_eve_quest_persistence.sql`](../../migrations/0002_eve_quest_persistence.sql) **đều chưa tạo 2 bảng này**, và `OP_WORLD_BOSS` (`0x49`) rơi vào `unimplemented::handle` ([`dispatcher.rs:432`](../../src/server/dispatcher.rs#L432)).

### 3.2. Hiện trạng các thành phần liên quan Lễ hội trong Rust
1. **Loader `Data/Festival.Dat` ([`src/data/loaders/festival.rs`](../../src/data/loaders/festival.rs)):**
   - Đã giải mã chính xác 7 bản ghi (`1..=7`) từ `Data/Festival.Dat` (20,310 bytes) vào `GameData.festivals` ([`src/data/loader.rs:72, 357, 371`](../../src/data/loader.rs#L72)).
   - Hạn chế: Gán cứng `date_dword: FESTIVAL_PARITY_DATE_DWORD` (`0x0121EAC6`) ở dòng 107; toàn bộ `src/` **không có bất kỳ consumer nào** đọc `data.festivals`.
   - `TFestival` không tồn tại trong server (chỉ là tên class phía client).
   - Tab `Data Integrity` trên Dashboard quét tự động file `.dat/.emg/.mng` trong `Data/` ([`loader.rs:475-523`](../../src/data/loader.rs#L475-L523)), nên chỉ hiển thị size & SHA-256 của `Festival.Dat` nếu có trong thư mục gốc `Data/`.
2. **Handler & Frame Builder ([`src/server/handlers/festival.rs`](../../src/server/handlers/festival.rs)):**
   - [`handle_festival`](../../src/server/handlers/festival.rs#L42-L47) được dispatch khi nhận C→S `0x24 sub 0x19` ([`dispatcher.rs:377-383`](../../src/server/dispatcher.rs#L377-L383)), nhưng **bỏ qua payload** và luôn gửi cứng `F44406002419C6EA2101` (`0x0121EAC6` = `19000110`).
   - [`build_seasonal_announcement_raw`](../../src/server/handlers/festival.rs#L32-L34) tạo frame `F4 44 03 00 17 39 <id>` (ghi chú hỗ trợ `1..=74`), nhưng **không có bất kỳ caller nào trong `src/`** — chỉ được test tại [`tests/events_arena/festival_quiz.rs:816-829`](../../tests/events_arena/festival_quiz.rs#L816-L829).
   - `OP_ITEM` (`0x17`) C→S tại [`dispatcher.rs:309`](../../src/server/dispatcher.rs#L309) → [`inventory.rs:34-88`](../../src/server/handlers/inventory.rs#L34-L88) chỉ xử lý túi đồ, hợp thành (`0x39`), khảm lông (`0x3A`), chuyển sinh (`46`), đổi điểm Lôi đài (`0x6E`) — không có nhánh lễ hội.
3. **Bộ lập lịch / Điều kiện thời gian (Scheduler / Calendar):**
   - **Chưa tồn tại** scheduler/cron theo ngày-tháng-mùa (dương lịch hoặc âm lịch) trong `src/`.
   - Vòng lặp nền duy nhất trong [`src/web/server_control.rs:259-281`](../../src/web/server_control.rs#L259-L281) chỉ chạy watchdog 120 phút (`events.check_watchdog()`) và timeout 45s cho Bắc Tinh Quân (`events.get_timed_out_quizzes()`).
   - [`src/eve/evaluator.rs:7-8, 62-359`](../../src/eve/evaluator.rs#L7-L8) chỉ kiểm tra `conditionClass` `{0, 1, 2, 7, 8, 9, 10, 12, 14}` (item, quest, stats, battle, dialog, count, role), **chưa có điều kiện theo ngày/mùa hay trạng thái sự kiện đang bật trên Dashboard**.

---

## 4. Tổng hợp các Gap & Bug cần xử lý để hỗ trợ Sự kiện theo mùa / lễ hội

1. **Bug Wire Parity `S→C 24 19` & `Data/Festival.Dat`:**
   - Server gửi `0x0121EAC6` (`19000110`) → client ép thành `(double)19000110.0` và tra `FUN_0051e2dc` trong `Festival.Dat` (đang chứa `11.0` ở cả 7 bản ghi) → **không khớp, form không bao giờ mở**.
   - Nếu muốn giữ `Festival.Dat` gốc không sửa file client, server chỉ có thể mở được `rec01` (*Tết Nguyên Đán*) bằng cách gửi `u32 = 11` (`0x0000000B`). Nếu muốn mở đủ cả 7 lễ hội trong `Festival.Dat`, phải cập nhật giá trị `double @+0x0000` phân biệt cho từng bản ghi trong `Festival.Dat` (ví dụ `1.0..7.0` hoặc `YYYYMMDD.0`) đồng bộ với `u32` server gửi qua `S→C 24 19`.
2. **`build_seasonal_announcement_raw` (`S→C 17 39`) chưa khớp cấu trúc đa dạng của `FUN_00722cd4`:**
   - Hàm hiện tại chỉ nhận một `announcement_id: u8` (`1..=74`) và không có caller.
   - Thực tế client (`00722cd4_FUN_00722cd4.c`) chỉ dùng `0x01..0x0D` và `0x15..0x4A` (66 nhánh, bỏ trống `0x0E..0x14`), trong đó nhiều nhánh (`0x27, 0x28, 0x2B, 0x2C, 0x31, 0x33..0x35, 0x37..0x39, 0x3B, 0x3D..0x4A`) **bắt buộc có byte `payload[2]` (sub3)** hoặc chuỗi/DWORD tham số kèm theo; một số mã như `0x07, 0x2D..0x32, 0x37` không phải lễ hội.
3. **Nút Start/Stop `seasonal_festival` trên Dashboard chưa kết nối gameplay:**
   - Chưa phân biệt từng lễ hội cụ thể (Tết / Nguyên Tiêu / Valentine / Đoan Ngọ / Thất Tịch / Trung Thu / Halloween / Giáng Sinh).
   - Chưa có cơ chế khi bật một lễ hội thì: (a) phát broadcast `S→C 17 39` mở màn cho toàn bộ player online, (b) kích hoạt/mở khóa các `EVE` scene/NPC/quest/warp hoặc drop item theo mùa tương ứng (ví dụ gắn cờ event active vào `evaluator.rs` / NPC spawner), (c) tùy chọn tự kích hoạt theo lịch (Calendar/Cron) hoặc chọn lễ hội cụ thể từ Dashboard.
4. **Cần đồng bộ thêm với sự kiện gắn mùa khác:**
   - **Đua thuyền rồng (`dragon_boat`, `OP 0x46`)** là hoạt động gắn với **Tết Đoan Ngọ** (map `12000` + đường đua `60510..60521`), hiện cũng chưa gate theo `is_event_active(EVENT_DRAGON_BOAT)` và chưa có pha đếm ngược / BXH đầy đủ (`0x46 02..08`, `0x24 18`).
   - **Pháo hoa (`S→C 17 46`)** và thông báo lễ hội qua `OP 0x14` (`0x23/0x24`, `0x2D/0x2E`) chưa có builder/trigger trong `src/server/handlers/festival.rs`.
