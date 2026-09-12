# PHÂN TÍCH — Main OP 0x14 (Case 18) — `FUN_0078ec3f` @ `0x0078EC3F` — Chuyển map / Dịch chuyển theo kịch bản (Map Teleport & Scene Script)

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Xác minh từ mã nguồn sơ cấp** (handler + helper + jump table + caller C→S). Đính chính: tài liệu cũ `.scratch/op-code/opcode_00_01.md` mục 6 ghi *"Case 21 ứng với OP 0x14"* là **SAI** — theo jump table đã kiểm chứng (`jumptable_byte200_0x78A8EE` + `jumptable_dword200_0x78A9B6`), **MainOp 0x14 → byte table index 18 → target 0x0078EC3F = Case 18**; còn Case 21 (`FUN_00790ed5`) thuộc MainOp **0x18**.

---

## 1. Tóm tắt nghiệp vụ

**Giả định ban đầu "Movement/Map" — KẾT LUẬN: ĐÚNG MỘT PHẦN, cần chính xác hoá.**

Theo evidence, Main OP 0x14 **KHÔNG phải đồng bộ chuyển động liên tục** (walk/movement per-tick). Nó là kênh **"Scene/Map Script & Teleport"** hai chiều:

- **S→C (chủ đạo)**: server đẩy 1 "lệnh kịch bản thế giới" 56 sub-op, trong đó:
  - **SubOp 0x01–0x06**: ghi khối tham số 15 byte vào `PlayerState + 0xA09B` rồi gọi `FUN_005EB530` — hàm *thi hành hiệu ứng*, có switch theo byte **loại hiệu ứng** (B2, `+0xA09F`): `1 = ĐỔI MAP/DI CHUYỂN LIÊN MAP` (set tọa độ vào tâm spawn map mới, cập nhật `a074 = slot map`, nạp tiêu đề bản đồ), `4 = TELEPORT TRONG MAP THEO TỌA ĐỘ Ô` (tile×20 px), `2 = SNAP/COPY CẶP TỌA ĐỘ`, `0 = hiệu ứng tài nguyên/số học/hộp thoại`, `5 = cắt cảnh FMV`, `6 = mở hộp thoại NPC/cửa hàng`. SubOp 1..6 chỉ là **6 biến thể kịch bản cùng layout** (byte đầu của payload lưu vào `+0xA09B`, không đọc lại).
  - **SubOp 0x07, 0x08, 0x09, 0x0A–0x11**: máy trạng thái **chuyển tiếp (transition)** — cấp "vé" trả về cổng (`+0x44F` của TFConnect), dựng/xóa cờ bận `a094`, cờ khóa nhập liệu `a096`, cờ dịch chuyển `a076`, và **hoàn tất/dọn dẹp sau load map** (case 0x08).
  - **SubOp 0x12–0x38**: chuỗi thông báo — **Toast** (2000 ms, `gvar_007DA084 + VMT 0x90`), **chat hệ thống** (`FUN_007AB870`), **âm thanh** (`sound\WA0014.wav` qua `FUN_007A7F20`), message box, cập nhật cờ/Word lên actor cục bộ (`gvar_007DA7BC + 0x628/0x62A/0x62C`), và 8 sub-op delegate sang helper chưa trích xuất decompile (`0x2A, 0x2C, 0x31, 0x32, 0x33, 0x35, 0x36, 0x38`).
- **C→S**: client **gửi yêu cầu** OP 0x14 (qua `TFConnect.SendCommand = FUN_0077F414`, chọn nhánh theo `CL` nội bộ, **không phải SubOp wire tự do**) với 4 tình huống: **click lối thoát (sub 1)**, **đọc vùng/tự nhảy theo region (sub 4)**, **tự động quay lại cổng khi server phát vé (sub 6)**, **hủy/đóng UI (sub 9)**.

**Đồng bộ tọa độ chạy realtime thuộc OP khác** — bằng chứng: trong `FUN_0077F414`, `case 6` gửi `[0x06][X:Word LE][Y:Word LE]...` (tọa độ actor `gvar_007DA7BC + 0x4C/0x50` khi thay đổi) và `case 7` gửi `[0x07][+0x138][+0x130][+0x134]` — tức movement báo cáo bằng OP 0x06/0x07, không phải 0x14.

---

## 2. Entry & cách đọc PacketBuffer

Chuỗi framing/dispatcher đã xác minh ở `handoff-opcode-exploration-guide.md` mục 1 (giữ nguyên, không lặp lại):
`Frame = [F4 44] [Len: Word LE] [Payload]`, XOR `0xAD`; Payload = `[MainOp][SubOp][...]`.

- `ClientSocket1Read` (`0050cd6c`) tách frame → `CY_AddRevQueue`; `CY_DelRevQueue` (`00516158`) mỗi tick pop: `MainOp = payload[0]`, `RestPayload = payload[1..]` (Delphi string, **0-based bên trong handler**, ghi là `ECX`/`iVar6`; độ dài thực ở `*(int*)(ECX - 4)`).
- Dispatcher `FUN_0078A89C`: `byte table 0x78A8EE[0x14] = 0x12` → `dword table 0x78A9B6[18]` (entry tại `0x0078A9FE`) → nhảy `0x0078EC3F`.
- **Pre-Handler preamble** (đọc `RestPayload[0]`, tức `payload[1]`):
  ```c
  iVar6 = *(int *)(unaff_EBP + -0xc);                       // ECX = RestPayload
  *(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6);    // SubOp = ECX[0] = payload[1]
  if ((*(char *)(*(int *)gvar_007DA664 + 0x44f) != 0) &&    // TFConnect: "vé quay về cổng"
      (*(char *)(*(int *)gvar_007DA37C + 0x1d) == 2))       // scene+0x1D == 2 (đang in-game)
      *(char *)(*(int *)gvar_007DA664 + 0x44f) = 0;         // BẤT KỲ message 0x14 nào cũng tiêu thụ vé
  switch (*(uint *)(unaff_EBP + -0x14)) { ... }
  ```

### Quy đổi chỉ số (DÙNG CHO TOÀN BỘ MỤC 4)

| Cách ghi trong mã | Ý nghĩa wire |
| :--- | :--- |
| `ECX[i]` (0-based, `iVar6 + iVar4`) | `payload[1+i]` |
| `_LStrCopy(ECX, p, n, &out)` (Delphi 1-based) | cắt `n` byte từ `ECX[p-1]` = `payload[p]` |
| `FUN_0077EB9C(out)` → Word LE | `b0 + b1*0x100` |
| `FUN_0077EF7C(out)` → DWORD LE | `b0 + b1*0x100 + b2*0x10000 + b3*0x1000000` |

### Đối tượng toàn cục tham chiếu (đã kiểm chứng)

| Symbol | Vai trò (evidence) |
| :--- | :--- |
| `gvar_007DA5A0` → `PlayerState` | "bộ điều khiển trạng thái người chơi/game" — chứa cụm cờ `+0xA074..0xA12A` và khối tham số effect `+0xA09B`; X/Y authoritative ở `+4`/`+8` |
| `gvar_007DA7BC` → `LocalActor` | actor cục bộ: `+4`=actorId (DWORD, so ở SubOp 0x1A), `+9`=tên, `+0x1C/+0x20`=X/Y, `+0x4C/+0x50`=vị trí hiện tại (movement C→S), `+0x63A`=map class id, `+0x656+W*3`=mảng 200 tài nguyên, cờ `+0x628/62A` (SubOp 0x22), `+0x62C` (SubOp 0x21), `+0x653` |
| `gvar_007DA6DC` | `array[≤100]` con trỏ **map-slot/world object**; mỗi đối tượng: `+4`=class id (0x947A=38010, 0x3B3A=15162, 0x3EA5=16037, 0x31A7=12711), `+0x1C/+0x20`=tọa độ tâm?, `+0x54/+0x58`=**spawn X/Y (DWORD)**, `+0xE3`, `+0x2A`, `+0x578` (timestamp double) |
| `gvar_007DA664` → `TFConnect`(form) | socket; `+0x44F` = cờ "vé trả về cổng" |
| `gvar_007DA37C` | game-state; `+0x1D` = sceneMode (0 thường / 1 movie / 2 in-game) |
| `gvar_007DA084` | Toast manager — VMT `+0x90`(text, 2000ms,…) |
| `gvar_007DA1B0` + `FUN_007AB870(win, actorId, text, kind)` | chat system: tự resolve tên (`FUN_0075DDB8` nếu không phải self), `kind='\0'` = prefix hệ thống |
| `gvar_007DA5A8` | hộp thoại NPC; `gvar_007D9C50` | form shop; `gvar_007DA434`/`TMovie_Create` | movie |
| `gvar_007D9D34` | "target/cursor registry": `+0x76` có object, `+0x7B` slot index, `+0x7F` kind (3=portal) |
| `gvar_007D9D74` → `TWF_BigNpcBmp` | overlay ảnh NPC lớn; `+4` = MainOp vừa nhận (chỉ ghi ở SubOp 1–6) |
| `gvar_007DA014 / 007DA540 / 007D9FD8 / 007D9DE4` | các bảng string/price/name (tra theo index) |

---

## 3. Bảng tổng hợp SubOp (S→C) — switch đầy đủ trong `FUN_0078EC3F`

| SubOp | Độ dài payload tối thiểu | Nhóm | Hành vi cốt lõi |
| :---: | :---: | :--- | :--- |
| `0x01–0x06` | 17 | **SCRIPT/TELEPORT** | ghi block `PlayerState+0xA09B` (15B) → `FUN_005EB530` theo B2 (`payload[6]`) |
| `0x07` | 2 | TRANSITION | set `a097=1, a094=0, a096=1`; cấp vé cổng `TFConnect+0x44F=1` (nếu scene==2) |
| `0x08` | 2 | TRANSITION-END | xóa hàng loạt cờ (`a095/076/097/096/094`, `LocalActor+0x653`), xử lý `a089==2` → Now()→slot, clear tên map `a0FC/a100`, clear `0x44F`, scene`+0x1D`=0, `a085=0`, nếu map slot class `0x947A`→`FUN_00711A90` (clear actor flag `+0x582`) |
| `0x09` | 2 | TRANSITION | `a094=0` (clear busy) |
| `0x0A, 0x0B` | 2 | TRANSITION | `a096=1, a094=0` |
| `0x0C` | 2 | TRANSITION-START | `a076=1, a094=1` (bắt đầu dịch chuyển, pending) |
| `0x0D–0x11` | 2 | TRANSITION | `a094=0, a096=1` (×5 biến thể giống hệt) |
| `0x12` | 2 | TOAST | text tĩnh `0x00797174` |
| `0x13` | 4 | SOUND+TOAST | play `sound\WA0014.wav`; toast nhúng `IntToStr(Word payload[2..3])` |
| `0x14` | 2 | TOAST (map-aware) | chọn toast theo `a085`/`a08A` & class map — **xem 4.3** |
| `0x15` | 2 | TOAST | text `0x0079727C` (đồng nhất branch `a085==4` của 0x14) |
| `0x16, 0x17` | 3 | SOUND+TOAST | play WAV; toast nhúng `IntToStr(byte payload[2])` |
| `0x18, 0x19` | 2 | TOAST | `0x00797414` / `0x00797438` |
| `0x1A` | 8+N | CHAT+TOAST | **chat hệ thống với id+name**; nếu actorID==self → toast; wire 4.4 |
| `0x1B` | 2 | TOAST | `0x007974F4` |
| `0x1C, 0x1D` | 2 | CHAT | `0x0079752C` / `0x00797570` qua `FUN_007AB870` |
| `0x1E` | 2 | TOAST | `0x007975B4` |
| `0x1F` | 3 | CHAT | sinh dòng chat "1,2..N" với `N=byte payload[2]` |
| `0x20` | 2 | TOAST | `0x00797654` |
| `0x21` | 3 | ACTOR-FLAG | `LocalActor+0x62C = (payload[2]==1)` |
| `0x22` | 6 | ACTOR-COORD | `LocalActor+0x628 = W(payload[2..3])`; `+0x62A = W(payload[4..5])` |
| `0x23` | 2 | TOAST | `0x0079768C` |
| `0x24` | 3 | CHAT chọn | 1→`0x007976C0`; 2→`0x00797794` |
| `0x25` | 2 | TOAST | `0x00797810` |
| `0x26` | 3 | CHAT/TOAST chọn | 1→`0x79783C`; 2→`0x7978F8`; 3→toast `0x7979A0` |
| `0x27` | 2 | TOAST | `0x007979BC` |
| `0x28` | 3 | CHAT chọn | 1→`0x7979E0`; 2→`0x797A08` |
| `0x29` | 3 | TOAST/CHAT chọn | 1→toast `0x797A30`; 2→`0x797A5C`; 3→`0x797B00` |
| `0x2A` | ? | HELPER | `func_0x005F16F4(PlayerState, ECX)` — **chưa có trong decompile** |
| `0x2B` | 2 | TOAST | `0x00797B64` |
| `0x2C` | ? | HELPER | `func_0x0074641C(gvar_007D9D34, ECX)` — chưa trích xuất |
| `0x2D` | 2 | TOAST | `0x00797BB8` |
| `0x2E` | 3 | CHAT/TOAST chọn | 1→`0x797BE4`; 2→`0x797C24`; 3→toast `0x797C5C` |
| `0x2F` | 3 | CHAT/TOAST chọn | 1,2→chat `0x797C98/0x797D28`; 3,4→toast `0x797D70/0x797DA0` |
| `0x30` | 3 | TOAST chọn | 1,2,3 → `0x797DD0/0x797E24/0x797E54` |
| `0x31,0x32,0x33` | ? | HELPER | `func_0x00748430/0x007485BC/0x00748860(gvar_007D9D34, ECX)` — chưa trích xuất |
| `0x34` | 3 | FLAG | `PlayerState+0xA12A = byte payload[2]` (cờ điều kiện cho `FUN_005EB530` case 6) |
| `0x35` | 3 | PANEL | 1→`func_0x00749198(LocalActor, ECX)`; 2→`FUN_0074927C` (đóng panel `+0x51E` + gửi **C→S OP 0x17**) |
| `0x36` | ? | HELPER | `func_0x007494D8(LocalActor, ECX)` — chưa trích xuất |
| `0x37` | 2 | TOAST | `0x00797E94` |
| `0x38` | 3 | PANEL | 1→`0x0074E638`; 2→`0x0074E82C`; 3→`FUN_0074E8FC` (đóng panel `+0x1511` + gửi C→S 0x17) |

Không có `default:` — SubOp ngoài danh sách bị bỏ qua silently.

---

## 4. Chi tiết từng SubOp trọng tâm

### 4.1. `0x01–0x06` — khối lệnh Teleport/Scene-Script (quan trọng nhất)

**Wire layout (17 byte payload, mọi SubOp 1–6 giống hệt nhau):**

| payload idx | kích cỡ | kiểu | lưu vào | tên tạm |
| :--- | :---: | :--- | :--- | :--- |
| `[0]` | 1 | `0x14` | — | MainOp |
| `[1]` | 1 | byte | — | **SubOp (1..6)** → biến thể kịch bản |
| `[2]` | 1 | byte | `PlayerState+0xA09B` (B0) | P0 — **không helper nào đọc lại** (trace `0xA09B..` chỉ xuất hiện trong handler/dispatcher/`FUN_005EB530`) |
| `[3..4]` | 2 | **Word LE** (`FUN_0077EB9C`) | `+0xA09C` (W1) | cũng không được `FUN_005EB530` đọc (reserved) |
| `[5]` | 1 | byte | `+0xA09E` (B1) | reserved |
| `[6]` | 1 | byte | `+0xA09F` (**B2 — KIND**) | **switch hiệu ứng** |
| `[7]` | 1 | byte | `+0xA0A0` (**B3 — SUBKIND**) | chọn nhánh con |
| `[8..9]` | 2 | **Word LE** | `+0xA0A1` (**W2**) | **map-slot index ≤100** → `gvar_007DA6DC[W2]` |
| `[10]` | 1 | byte | `+0xA0A3` (B4) | selector phụ |
| `[11..14]` | 4 | **DWORD LE** (`FUN_0077EF7C`) | `+0xA0A4` (D1) | số lượng/timer ms/số hiệu movie |
| `[15..16]` | 2 | **Word LE** | `+0xA0A8` (W3) | index bảng string/tiêu đề/event (≤100/200) |

Toàn bộ `_LStrCopy` tương ứng: `chars 3..4 / 8..9 / 11..14 / 15..16` (1-based). Kết thúc: `a094=0` (xong pending), `TWF_BigNpcBmp+4 = MainOp(0x14)`, rồi `FUN_005EB530(PlayerState)`; render chỉ ghi 1 dòng: **client cập nhật lại khung cảnh/overlay theo map mới — không cần mock.**

**Ngành switch `B2` trong `FUN_005EB530` (đồng bộ vị trí/map thật sự nằm ở đây):**

| B2 | Hành vi (evidence dòng) |
| :---: | :--- |
| `0` | "hiệu ứng số học/thông báo": theo B3: `1`→toast `IntToStr(D1)+<tên tra gvar_007DA540[W2]>` (1000ms); `2`→cộng/trừ tài nguyên `LocalActor+0x656+W2*3` theo B4 (1:+, 2:−) với lượng D1, clamp 255, toast "<tên gvar_007D9FD8[W2]> (.. ) = .."; `3`→theo B4: 2=`actor[W2].+0x100=2` & scale `+0x340*=` (tốc độ), 3/4=`FUN_00712940/712A88(actor[W2], D1*1000ms)` (buff đếm giờ), 5=toast; `8`→`MessageBoxA` với tên nhân vật tra `gvar_007D9DE4[W2]` (B4 chọn 2 template). Luôn set `a096=1`. |
| **`1`** | **ĐỔI MAP (Map Teleport):** `a096=0`; clear chuỗi `a0FC/a100`; **`PlayerState+4 = actor[W2]+0x54`, `PlayerState+8 = actor[W2]+0x58` (X/Y spawn DWORD)**; điều kiện phụ qua `FUN_0070E834(actor[W2])==8` & `+0x2A!=0xF` để set `a106/a107` (+restore `+0xE3`); **`a074 = W2` (slot map hiện tại)**; nếu `a106`→`FUN_005EEA58` (recenter camera actor, VMT+0x18 msg 8); theo **B3**: `3`→nạp tiêu đề `gvar_007DA014[W3]` vào `a0FC` qua `FUN_005F159C` + `gvar_007D9D6C` caption (`FUN_0063C1D0(actor[W2])`); `7`→`a100` + caption theo **self actor**; `0xC`→caption theo actor phương tiện `LocalActor+0x57C+[+0x151C]*4`. |
| `2` | SNAP vị trí: `a08A=a0DC; a08C=a070; a0AA=a0DC; a0AC=a070; a096=1` — copy cặp tọa độ/region hiện tại (`a0DC` = region/slot X-Word được client set khi vào vùng, `a070` = map id Word từ `FUN_005EFEE8`) vào ô lưu chờ. |
| `3` | Portal-event: `a0D0 = PlayerState[0x9A10 + W3*4]` (bảng portal ≤200); `a094=1`; gọi `SendCommand` **OP 0x0B** (xin dữ liệu map tiếp theo). |
| **`4`** | **TELEPORT TRONG MAP:** chỉ khi `B3==3`: cắt **4 ký tự cuối của chuỗi script `a0FC`** làm 2 tọa độ ô, `FUN_007127BC(actor[W2], (tileX*20−10), tileY*20)` — di chuyển actor tới **tọa độ pixel theo lưới 20px**. |
| `5` | Cắt cảnh: clear khóa `a096` theo B4, `gvar_007DA37C+0x1D=1`, `TMovie_Create` + `FUN_00618668(gvar_007DA434, <prefix+IntToStr(D1)>)` → phát FMV số D1. |
| `6` | Mở hội thoại/shop: cờ `a095/a096` theo B4 (0: khóa+đợi, 1: mở); **giống B2=1 phần set X/Y spawn + `a074=W2`**; `a0D4 = PlayerState[0x9D34 + W3*4]` (bảng hội thoại ≤100, record 5 byte/entry: `+4` itemId Word, `+6/7` type/kind, `+8` icon); nếu record `+0x74==1`→**hộp thoại NPC** `gvar_007DA5A8` (dòng tên từ `gvar_007DA014`, giá trị từ `gvar_007DA540` `+0x58` "      $"); `==2`→**shop** `gvar_007D9C50` (`FUN_0056E9E4/56E300` thêm item, callback bind PlayerState); nhánh đặc biệt: `LocalActor+0x63A==0x31A7` & class `actor[W2]==0x3EA5` & cờ `a12A` (từ SubOp 0x34!) & itemId `0x6EB0/0x6E6B/0x7426`. |

### 4.2. Máy trạng thái chuyển tiếp — `0x07, 0x08, 0x09, 0x0A..0x11` (không payload)

| Cờ (`PlayerState`) | Ý nghĩa suy ra | SubOp set | SubOp clear |
| :--- | :--- | :--- | :--- |
| `+0xA094` "pending/waiting server" | client set =1 sau khi gửi 0x14; **server confirm bằng 0x14/xóa** | `0x0C` (=1) | `0x01–06, 07, 09, 0A, 0B, 0D–11` (=0) |
| `+0xA096` "transition lock/busy" | khóa xử lý timer `FUN_005EE30C` | `0x07, 0A–11` | `0x08`, (B2=1 của 5EB530) |
| `+0xA076` "đang dịch chuyển" | gate để client tự gửi trả `sub 6` | `0x0C` | `0x08`, `FUN_005EE30C` |
| `+0xA097` | "đích đến đã chốt (vé cổng)" | `0x07` | `0x08` |
| `TFConnect+0x44F` | "vé server cho phép tự-quay-về"; tiêu thụ bởi **bất kỳ** msg 0x14 (preamble) | `0x07` | preamble 1..6?, `0x08` |
| `gvar_007DA37C+0x1D` | sceneMode: `0x08`→0, movie→1, in-game→2 | — | `0x08` |

### 4.3. `0x14` — Toast phụ thuộc ngữ cảnh map

`a085` = **Word map-slot đang chờ/đích** (client set ở `FUN_005E9D94`; `0x08` reset về 0). Chuỗi if/else:
`actor[gvar_007DA6DC[a085]].class(+4)==0x3B3A` → toast `0x797210`; `a085==4` → `0x79727C`; `==3` → `0x7972D8`; `==5` → `0x79731C`; `==0xE` → `0x797368`; ngược lại `a08A==8` → `0x797210`. → toast "không thể đi tiếp/không tới được..." tuỳ map đích. **Mock server chỉ cần chọn SubOp; nội dung text do client.**

### 4.4. `0x1A` — Chat hệ thống có actorID + tên

**Wire:** `[14][1A][A:1B][actorID:4B LE][N:1B][name: N bytes]` (payload = 8+N bytes; `_LStrCopy(ECX,3,4)`=payload[3..6], `_LStrCopy(ECX,8,N)`=payload[7..]).
Logic: ghép `IntToStr(A) + const + name` → `FUN_007AB870(chatWin, selfActorId, text, 0)` (dòng chat system); **nếu `actorID == *(LocalActor+4)`** (gói tin nhắm vào chính mình) → thêm toast `0x7974C8 + IntToStr(A) + 0x7974D4`.

### 4.5. `0x22` — Cặp tọa độ Word cho actor

`LocalActor+0x628 = Word(payload[2..3])`; `LocalActor+0x62A = Word(payload[4..5])`. Hai field này được **zero ở `FUN_005EA3E0` ngay sau khi gửi sub-4** ⇒ là "vị trí/region chờ" do cả hai chiều cùng cập nhật. (Render: bỏ qua.)

---

## 5. Chiều C→S liên quan

### 5.1. Kiến trúc `SendCommand`

`FUN_0077F414(TFConnect-self@EAX, MainOp@DL, SubSel@CL)` — **`CL` không phải SubOp trên wire một cách máy móc**: nó chỉ chọn *template payload* nội bộ. Outer dispatch: byte table `0x77F474[MainOp≤0xC7]` → dword table `0x77F53C`. **Lưu ý decompile**: bản C chỉ khôi phục một số case (0,1,3,6,7,0xB,0x19,0x1D,0x32,0x36,0x37,0x3A,0x3B,0x3D,199) và để **`case 0x14: break;` (dòng 925) — đó là artifact jump-table bị gộp**; bằng chứng ngược: 5 call-site thực gửi `DL=0x14` (xem 5.2) và vùng ASM chứa sub-dispatch `[EBP-0x6]` khớp chính xác `{1..3, 4, 6, 9}` (chain `DEC EAX; SUB AL,3; JC→1..3 | JZ→4 | SUB AL,2 JZ→6 | SUB AL,3 JZ→9`).

### 5.2. Bốn request 0x14 của client (ASM + caller, đã xác minh lệnh `MOV CL, x; MOV DL, 0x14; CALL 0x77F414`)

| SubSel (CL) | Caller → ngữ cảnh | Template wire (reconstructed từ ASM body, tự tin khá cao) |
| :---: | :--- | :--- |
| `1` (nhóm 1..3) | `FUN_005E9D94` ← **`TForm1.DXDraw1MouseDown`**: click **lối thoát** khi `gvar_007D9D34.kind==3`, khoảng cách ≤ `0x95`px (đọc `gvar_007DA7BC+0x1C/0x20` vs slot); trước gửi set `a085=slotWord`, `a089=1`, `a0D8=a085`, `a078=destPtr`, `a076=1` | `[0x14][a089:1B][a085: Word LE]` → sau gửi `a094=1` |
| `4` | `FUN_005EA3E0` & `FUN_005EE590` (đi bộ tự động chạm **vùng dịch chuyển**, table `PlayerState+0x338+idx*4`, lưới `tile*0x14=20px`): set `a0DC=regionIdx`, `a08A=a0DC`, `a08C=a070`, **`a08E=8`** (ea3e0) / **`=4`** (ee590), `a078`, `a076=1` | `[0x14][a08E:1B][a08A: Word LE]` → `a094=1`; (đúng cặp giá trị mà S→C `0x14`/`0x22`/`B2=2` đọc: `a08E==8`, `a08A`) |
| `6` | `FUN_005EE30C` (game-tick pump @caller `0x00515949`, **chỉ chạy khi S→C case 0x07 cấp vé `0x44F`** và `a076≠0, a094==0, a095==0`): clear `a096` | `[0x14][06]` (2 byte) → `a094=1`; kèm hiệu ứng `FUN_005752A8(gvar_007DA688, actor[W].class)` |
| `9` | `FUN_005D7000` (**đóng UI chọn lối thoát**, ref data-table form): `a104=0x28`, `a095=0` | `[0x14][09][a104=0x28]` → `a094=1` |

> Ghi chú body CL=4: khi `LocalActor+0x63A` ∈ các map-class đặc thù (0xC0F9..0xC103…) client còn gọi `FUN_0074B330(actor,3)` trước khi gửi.

### 5.3. Vòng request→response đầy đủ (đã đóng)

```
Client click exit      →  C→S [14][01][slot]        (a094=1 pending)
Server accept          →  S→C [14][0C]              (a076=1, a094=1)
Server effect          →  S→C [14][sub 1..6] W2=slot mới (B2=1) → PlayerState+4/8=spawn, a074=W2, a094=0
Server transition end  →  S→C [14][08]              (dọn cờ, a085=0, scene=0)
Server "return ticket" →  S→C [14][07]  (0x44F=1)
Client auto-return     →  C→S [14][06]              (tick pump FUN_005EE30C)
Server failure/map-lock→  S→C [14][14] / [15] toast theo a085
```

---

## 6. Ghi chú cho Mock Server

1. **Tối thiểu hoá**: mọi SubOp 0x14 trừ {0x01–06, 0x13, 0x16, 0x17, 0x1A, 0x1F, 0x21–0x38} **chỉ cần payload đúng 2 byte `[0x14][sub]`**.
2. **Teleport map (nghiệp vụ chính của game chuyển map)** — gửi `[14][01][P0][W1][B1][B2=0x01][B3][W2][B4][D1][W3]` với `W2 = slot map trong bảng client (≤100)` và `B3∈{3,7,0xC}`, `W3 = index tên map trong string-table `gvar_007DA014` (≤200 nếu dùng nhánh B3=3... kiểm chứng thêm khi dựng spec). Client **tự** đặt X/Y từ `actor[W2]+0x54/0x58`; mock **không gửi tọa độ** ở sub-op này.
3. **Teleport tọa độ trong map**: dùng `B2=4, B3=3`; tọa độ không lấy từ field số mà server phải **nhét vào cuối chuỗi script `a0FC` dạng "…%xx%yy" (4 ký tự số cuối, lưới 20px)** — chỉ khả dụng khi map đã nạp chuỗi này (xem 4.1.B2=4); nếu mock cần di chuyển tọa độ tổng quát, cân nhắc qua kênh movement OP 0x06/0x07 phía client và OP broadcast S→C khác.
4. **Cờ máy trạng thái phải tôn trọng handshake**: client set `a094=1` sau mọi request 0x14; gate của client từ chối request mới khi `a094!=0` hoặc `a096!=0` ⇒ **mock luôn phải phản hồi** (bằng `0x01..0x06` hoặc `0x09/0x0A/0x0D` để clear, hoặc `0x14` toast khi từ chối), nếu không người chơi kẹt "bận" vĩnh viễn.
5. **Auto-return chỉ hoạt động khi server cấp vé**: gửi `[14][07]` ⇒ tick pump sẽ tự động gửi `[14][06]`.
6. Gửi **0x08** để đóng màn hình transition (xóa `0x44F`, scene `+0x1D=0`, `a085=0`).
7. Các helper **chưa trích xuất** (`0x005F16F4, 0x0074641C, 0x00748430, 0x007485BC, 0x00748860, 0x00749198, 0x007494D8, 0x0074E638, 0x0074E82C`) — khi cần đặc tả sub `0x2A, 0x2C, 0x31–0x33, 0x35, 0x36, 0x38` phải dump thêm hàm từ binary (Ghidra `Function` chưa export).
8. Nội dung toast/chat (UNK_0x00797xxx) là chuỗi tĩnh phía client — mock chỉ chọn SubOp/sub-selector, không nhét text.

---

## 7. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Dùng cho |
| :--- | :--- | :--- |
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` (mục 1, 2-Nguồn 1, bảng ánh xạ dòng 56, Nguồn 3+4) | framing, dispatcher, mapping `0x14→Case18`, codec |
| 2 | `.scratch/op-code/opcode_00_01.md` | mẫu định dạng; phát hiện sai "Case 21↔0x14" ở mục 6 |
| 3 | `ts_decompile/case_functions/functions/case_018_0078EC3F_FUN_0078ec3f.c` (`@0x0078EC3F`, jump entry `0x0078A9FE`) | **handler chính** — toàn bộ SubOp, wire 1..6, cờ, toast/chat |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` (dòng 3019–3109 — bản inline case 18, `local_9=param_2`=MainOp dòng 572) | cross-verify wire 1..6, `TWF_BigNpcBmp+4 = MainOp` |
| 5 | `ts_decompile/functions/005eb530_FUN_005eb530.c` (`@0x005EB530`, switch `param_1[0xA09F]`, block base `param_1+0xA09B`) | ngữ nghĩa B2=0..6: **đổi map +4/+8=+0x54/58, a074, tile×20, FMV, shop/NPC** |
| 6 | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c`, `0077ef7c_FUN_0077ef7c.c` | Word/DWORD LE |
| 7 | `ts_decompile/functions/0077f414_FUN_0077F414.c` (dòng 757–1180: `case 0x14: break;`, `case 6/7` movement) + `0077f414_FUN_0077f414.asm.txt` (outer `0x77F474/0x77F53C`; chain sub-dispatch `[EBP-0x6]` dòng 1288–1297; templates dòng 1306–1470) | chiều C→S, giới hạn decompile |
| 8 | `005e9d94_FUN_005e9d94.c` (+asm `MOV CL,1; DL,0x14`), `005ea3e0_FUN_005ea3e0.c` (CL=4, a08E=8), `005ee590_FUN_005ee590.c` (CL=4, a08E=4), `005ee30c_FUN_005ee30c.c` (+asm CL=6), `005d7000_FUN_005d7000.c` (+asm CL=9) | 4 request 0x14 & cờ trước/sau gửi |
| 9 | `007ab870_FUN_007ab870.c` (chat system), `007a7f20_FUN_007a7f20.c` (play WAV), `00711a90_FUN_00711a90.c` (`actor+0x582=0`), `0074927c_FUN_0074927c.c` & `0074e8fc_FUN_0074e8fc.c` (đóng panel + gửi OP 0x17) | hành vi từng SubOp |
| 10 | `005eec44/005eea58/005752a8/0070e834/0075ddb8` (.c) | chi tiết map-change (restore `+0xE3`, recenter, actor-kind==8, tên actor) |
| 11 | `005efee8_FUN_005efee8.c` (dòng 200–215: `a070 = StrToInt(mapId)`, `a0DC` region), `007aaf68`, `00603f20`, `0051189c` (dòng 1419 `TWF_BigNpcBmp`) | định nghĩa global/offsets |
| 12 | `ts_decompile/redump/` (`subtable_0x7853E5/0x78540F.hex`, `jumptable_*.hex`) | loại trừ: sub-table 0x7853E5 thuộc case 0x19, không phải 0x14 |

**Kết luận giả định:** "Movement/Map" → **ĐÚNG ở vế Map/Teleport** (chuyển map, teleport kịch bản, máy trạng thái transition, vị trí spawn từ map slot +0x54/0x58); **SAI ở vế Movement liên tục** (movement realtime nằm ở C→S OP 0x06/0x07). Tên chính xác nhất cho đặc tả: **"Map Transition & World Scene Script (S→C 0x14 ↔ C→S 0x14 request/handshake)"**.
