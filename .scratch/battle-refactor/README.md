# DANH MỤC MÃ NGUỒN CLIENT BATTLE HANDLER (aLogin.exe)

Thư mục này chứa toàn bộ mã nguồn decompile (`.c`), assembly (`.asm.txt`) và binary tables (`.hex`) được trích xuất từ client game `aLogin.exe` (Delphi), chuyên trách toàn bộ module **chiến đấu (Battle Engine & Combat Management)**.

---

## 1. Bản Đồ Phân Loại File Mã Nguồn Theo Giai Đoạn Trận Chiến

### A. Trước Trận Chiến (Battle Initialization & Scene Loading)
* **Khởi tạo chiến trường & Quản lý thực thể trận đấu**:
  - `0063fadc_TFightField.Create.c` & `.asm.txt`: Constructor của `TFightField` (đối tượng chiến trường toàn cục `DAT_0098C63C`), cấp phát ma trận ô lưới, khởi tạo danh sách unit, timer.
  - `0063cda0_TFightHum.Create.c` & `.asm.txt`: Constructor đơn vị chiến đấu (`TFightHum`) cho Player, Pet, NPC.
  - `0063ceb0_TFightHum.Destroy.c` & `.asm.txt`: Destructor giải phóng unit.
  - `00641f14_TFightManage.Create.c` & `.asm.txt`: Constructor của bộ điều phối trận đấu `TFightManage` (`gvar_007D9CE4`).
  - `0067e38c_TEnterFight.Create.c` & `.asm.txt`: Hiệu ứng đồ họa chuyển cảnh bắt đầu vào trận chiến.
* **Xử lý gói tin mở trận (Server → Client)**:
  - `006438ac_FUN_006438ac.c` & `.asm.txt` (**Opcode `0x0B` Sub `0xFA`**): Frame mở trận đấu (`0B FA <mode> <sub> <record 24B × n>`). Tạo `TFightField`, nạp danh sách 20 ô của trận đấu.
  - `00644294_FUN_00644294.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x05`**): Frame nạp danh sách đơn vị tham chiến/viện trợ (`0B 05 <mode> <record 23B × n>`).
  - `00642244_FUN_00642244.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x04`**): Hiển thị thực thể trên bản đồ / chiến trường (`0B 04 ...`).
  - `0064e978_FUN_0064e978.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x0A`**): Trailer kết thúc chuẩn bị, sẵn sàng bước vào vòng đấu.
* **Lưới trận & Tọa độ ô (Battle Grid 4×5 = 20 vị trí)**:
  - `0065178c_FUN_0065178c.c` & `.asm.txt`: Kiểm tra giới hạn biên ô lưới: nhóm hàng (`grp <= 3`) và cột (`slot <= 4`).
  - `00646634_FUN_00646634.c` & `.asm.txt`: Phân chia phe: `grp < 2` là Phe Địch (Phe 2), `grp >= 2` là Phe Ta (Phe 1).
  - `00651094`, `0065130c`, `0065185c`, `006519cc`, `00651e3c`: Tính toán pixel tọa độ Isometric hiển thị đơn vị trên lưới chiến đấu.
  - `asm_644294_rec23.asm.txt`: Layout chi tiết 23-byte record cho đơn vị chiến đấu.

---

### B. Trong Trận Chiến (Turn Engine, Countdown, Actions & Damage Math)
* **Đếm ngược thời gian (Time Countdown) & Vòng lặp Animation**:
  - `00654d90_FUN_00654d90.c` & `.asm.txt` (Kích thước lớn: ~13KB): Vòng lặp battle chính per-frame, xử lý bộ đếm lùi thời gian (countdown timer), kiểm tra timeout khi người chơi chưa kịp ra lệnh, cập nhật animation trạng thái.
  - `00650a24_FUN_00650a24.c` & `.asm.txt` (**Opcode `0x34` Sub `0x01`**): Reset trạng thái 21 slots cho lượt đấu mới, snapshot timestamp, gọi engine chuyển lượt `FUN_00658c30`.
  - `00658c30_FUN_00658c30.c` & `.asm.txt`: Động cơ lượt đấu (`Turn Engine`), xử lý thứ tự hành động và chuyển lượt kế tiếp.
* **Thiết lập hành động của người chơi (Player Command & Target Selection)**:
  - `0050bff8_TForm1.DXDraw1MouseDown.c` & `.asm.txt`: Xử lý click chuột trên bàn cờ để chọn mục tiêu tấn công / kỹ năng.
  - `00652454_FUN_00652454.c` & `.asm.txt`: Giải thuật xác định mục tiêu hợp lệ (đơn mục tiêu, hàng ngang, cột dọc, toàn thể).
  - `006538bc_FUN_006538bc.c` & `.asm.txt`: Bộ xử lý lệnh người chơi:
    + Đánh thường (Normal Attack)
    + Sử dụng kỹ năng (Skill Cast)
    + Sử dụng vật phẩm trong trận (Use Battle Item)
    + Phòng thủ (Defense Mode)
    + Bắt võ tướng / NPC (Capture Mode)
    + Bỏ chạy khỏi trận đấu (Flee / Escape Mode)
    + Triệu hồi / Đổi Pet xuất chiến (Summon Pet)
  - `00733650_FUN_00733650.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x07`**): Trạng thái tự động chiến đấu (Auto-battle mode flag) và bộ đếm vòng tự động.
* **Công thức sát thương, Thuộc tính & Quy tắc Combo**:
  - `0065a300`, `0065a3d0`, `0065a554`, `0065aabc`, `0065ab8c`, `0065ad10`: Toàn bộ các hàm tính toán sát thương, ảnh hưởng của ngũ hành (Địa, Thủy, Hỏa, Phong, Tâm), phòng thủ, bạo kích (Critical), né tránh (Miss).
  - `005ab678_FUN_005ab678.c` & `.asm.txt`: Kiểm tra điều kiện thi triển kỹ năng (SP, loại vũ khí) và xử lý ngoại lệ đặc biệt cho skill ảo server combo (`0x4e26` = 20006).
  - `00759a8c_TFSkill.Create.c` & `00759b2c_FUN_00759b2c.c`: Nạp và đọc dữ liệu kỹ năng từ `Skill.dat`.
* **Hiệu ứng Kỹ năng (FX), Buff/Debuff & Biến cố trận đấu**:
  - `00648154_FUN_00648154.c` & `.asm.txt` (Kích thước: ~12KB, 241+ nhánh): Bảng tra cứu toàn bộ hiệu ứng chiêu thức, ánh xạ Skill ID sang FX Animation, căn chỉnh timing thi triển chiêu.
  - `case_046_007952AB_FUN_007952ab.c` & `.asm.txt` (**Opcode `0x35` BattleEventEx**):
    + `0064cd74_FUN_0064cd74.c` (Sub `0x01`)
    + `0064cfec_FUN_0064cfec.c` (Sub `0x03` - Biến cố sát thương / HP)
    + `0064d128_FUN_0064d128.c` (Sub `0x04` - Rơi vật phẩm khi hạ gục NPC)
    + `0064e3e0_FUN_0064e3e0.c` (Sub `0x07` - Di chuyển đơn vị giữa các ô trên lưới)
    + `0064faa8`, `0064fbd8`, `0064fd34`: Buff, debuff, trói, trạng thái khống chế.
* **Quy tắc chết & Xuất hồn (Death & Soul Return)**:
  - `0064cae0_FUN_0064cae0.c` & `.asm.txt`: Xử lý khi đơn vị bị cạn máu (HP = 0), kích hoạt trạng thái tử vong / xuất hồn.
  - `006bafdc_TWildFightSoul.Create.c` & `006bd124_TWildFightSoulME.Create.c`: Hiệu ứng xuất hồn bay về điểm lưu trữ/điểm hồi sinh.
  - `0064bb24_FUN_0064bb24.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x01`**): Xóa/dọn sạch ô của đơn vị đã chết hoặc thoát trận trên `TFightField`.

---

### C. Sau Trận Chiến (Battle Rewards & Post-Battle Settlement)
* **Thoát trận & Dọn dẹp**:
  - `00641fa4_FUN_00641fa4.c` & `.asm.txt` (**Opcode `0x0B` Sub `0x00`**): Xóa trạng thái chiến đấu của nhân vật (`player+0x376 = 0`), đóng UI trận chiến.
  - `0067e6a0_TExitFight.Create.c` & `.asm.txt`: Hiệu ứng kết thúc trận chiến, hoàn trả camera về thế giới bản đồ.
* **Điểm Kinh nghiệm, Cấp độ, Vàng & Vật phẩm thưởng**:
  - `0065b0dc_FUN_0065b0dc.c` & `.asm.txt`: Công thức tính toán điểm kinh nghiệm cần thiết theo cấp độ của client (`base + ROUND(lv^k)`).
  - `case_009_0078D125_FUN_0078d125.c` (**Opcode `0x08` Sub `0x01`**): Cập nhật EXP tuyệt đối (`attr = 0x24`), thăng cấp Level (`attr = 0x23`), điểm thuộc tính (`0x26`), điểm kỹ năng (`0x25`).
  - `0072f970_FUN_0072f970.c` & `.asm.txt` (**Opcode `0x05` Sub `0x0A`**): Hiệu ứng ánh sáng và âm thanh chúc mừng thăng cấp (`sound\WA0013.wav`).
  - `0072b084_FUN_0072b084.c` & `.asm.txt` (**Opcode `0x1A` Sub `0x01`**): Cộng tiền vàng nhận được sau trận chiến kèm toast banner.
  - `case_020_007902FB_FUN_007902fb.c` (**Opcode `0x17` Sub `0x06`**): Thêm vật phẩm thưởng/drop vào túi đồ của người chơi.
  - `0071d1bc_FUN_0071d1bc.c` & `.asm.txt`: Tìm kiếm đối tượng Pet đồng hành để trao kinh nghiệm và cập nhật chỉ số (`0x0F` Sub `0x08`).

---

### D. Bộ Đóng Gói / Gửi Tin (Client → Server) & Dispatcher Tổng
* **Dispatcher tổng điều phối gói tin (Server → Client)**:
  - `0078a89c_FUN_0078a89c.c` & `.asm.txt`: Hàm điều phối mạng chính phân nhánh toàn bộ Opcode qua jump table `0x78A8EE` và `0x78A9B6`.
  - `jumptable_byte200_0x78A8EE.hex` & `jumptable_dword200_0x78A9B6.hex`: Bảng nhảy nhị phân của switch-case S→C.
* **Bộ đóng gói và gửi tin (Client → Server)**:
  - `0077f414_FUN_0077f414.c` & `.asm.txt`: Bộ xây dựng khung tin `F4 44`, mã hóa tĩnh XOR `0xAD` và gửi qua socket.
  - `asm_cs0b_78048d_builders.asm.txt`: Toàn bộ mã máy hợp ngữ chi tiết của các builder C→S cho Opcode `0x0B` (Sub 01, 02, 03, 07, 08, 0A).
  - `cs0b_780440_jmptbl_5xDWORD.hex`: Jump table phân nhánh các builder của Opcode `0x0B`.
  - `table_0x77F474_200B.hex` & `table_0x77F53C_dword200.hex`: Bảng dispatch C→S toàn cục.
* **Khởi xướng trận đấu từ Client**:
  - `00642720_FUN_00642720.c` & `.asm.txt`: Người chơi click tấn công NPC quái (`0x0B` Sub `0x02` kind 3).
  - `00642c2c_FUN_00642c2c.c` & `.asm.txt`: Người chơi thách đấu PK người khác (`0x0B` Sub `0x02` kind 2).
  - `00643310_FUN_00643310.c` & `.asm.txt`: Xem trận đấu / Spectate (`0x0B` Sub `0x02` kind 4).
  - `00643470_FUN_00643470.c` & `.asm.txt`: Chen ngang / Jam vào trận chiến (`0x0B` Sub `0x02` kind 5).
