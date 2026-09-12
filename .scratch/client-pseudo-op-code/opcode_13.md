# PHÂN TÍCH — Main OP 0x13 (Case 17) — `FUN_0078eadc` @ `0x0078EADC` — **Chọn / Quản lý ĐỘI NPC (Party & Squad selection) (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x13] = 0x11 (=17)` → `jumptable_dword200_0x78A9B6[17]` (entry `0x0078A9FA`) → target `0x0078EADC` = **Case 17**.

> Phạm vi tài liệu: chỉ **core logic xử lý từng opcode**. Các khối thuần **graphics/sound/animation** (đổi ảnh nút `.bmp`, tắt nhạc `FUN_0079b620`, repaint form) chỉ được nêu khái quát để định vị nhánh, không đi sâu vẽ vời.

---

## 1. Tóm tắt nghiệp vụ

**Main OP 0x13 là kênh S→C "chọn / bỏ chọn đơn vị trong ĐỘI HÌNH NPC (squad 4 ô) của người chơi"**, đồng thời cập nhật panel quản lý đội/NPC đi kèm.

| SubOp | Hành vi cốt lõi (đã xác minh) |
| :---: | :--- |
| `0x01` | **CHỌN một NPC trong đội hình 4 ô** theo `unitID` gửi từ server → ghi vào bộ ba field chọn của nhân vật local (`LocalActor+0x12E1` = ID, `+0x12E5` = slot 1..4), rồi đẩy con trỏ đơn vị đó vào thanh trạng thái chính (`TSe_MainStatus+0x1E4`) và làm mới menu NPC. |
| `0x02` | **BỎ CHỌN** — xóa đúng bộ ba field trên (`+0x12E1=0`, `+0x12E5=0`, `MainStatus+0x1E4=0`) và làm mới menu. Đối xứng SubOp 1. |
| `0x04` | Delegate toàn bộ `RestPayload` cho `func_0x007a2ae8(TCY_TeamManage)` — **hàm chưa được trích xuất decompile** (xem §9). Suy luận: cập nhật **danh sách/thành viên đội (party roster)** (theo họ hàm `0x7a26xx/0x7a27xx` cùng object). |
| `0x06` | Delegate toàn bộ `RestPayload` cho `func_0x007a4d68(TFNpcManage)` — **chưa trích xuất**. Suy luận: cập nhật **cửa sổ quản lý NPC đội hình**. |

- `SubOp 0x00 / 0x03 / 0x05 / ≥0x07`: **không có nhánh → âm thầm bỏ qua** (handler `case_017.c:29–63` chỉ so sánh `==1, ==2, ==4, ==6`; không có `default`).
- **Không có bất kỳ đường gửi C→S nào** cho OP 0x13 (bằng chứng §6): đây là kênh **server-push thuần túy**.
- Có **dọn dẹp trạng thái chọn mục tiêu** ở preamble (chỉ SubOp 1 & 2) và **refresh TeamForm** ở hậu đề (SubOp 1, 2, 4) — xem §4.5, §4.6.

**Lưu ý giả định trong handoff:** bảng "Lộ trình" không xếp 0x13 vào nhóm ưu tiên. Kết quả: đây **không phải** opcode dữ liệu gameplay nặng, mà là **opcode điều khiển UI chọn đơn vị/party**. Vẫn quan trọng khi mock server muốn dựng lại màn hình quản lý đội/NPC sau khi vào game.

---

## 2. Entry & cách đọc PayloadBuffer

Framing/dispatcher đã xác minh ở `handoff-opcode-exploration-guide.md` mục 1 (giữ nguyên): `Frame = [F4 44][Len:Word LE][Payload]`, XOR tĩnh `0xAD`; `Payload = [MainOp][SubOp][...]`.

Trong `FUN_0078eadc` (`case_017_0078EADC_FUN_0078eadc.c:21–27`):
```c
iVar3 = *(int *)(unaff_EBP + -0xc);                    // ECX = RestPayload (chuỗi Delphi, Bỏ MainOp)
// BoundErr nếu len(ECX)==0
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar3); // SubOp = ECX[0] = payload[1]
iVar3 = *(int *)(unaff_EBP + -0x14);
if (iVar3 == 1)      { ... }                           // SubOp 0x01
else if (iVar3 == 2) { ... }                           // SubOp 0x02
else if (iVar3 == 4) { ... }                           // SubOp 0x04
else if (iVar3 == 6) { func_0x007a4d68(gvar_007D9E00, ECX); }  // SubOp 0x06
```

### Quy đổi chỉ số (DÙNG CHO TOÀN BỘ MỤC 4)

| Cách ghi trong mã | Ý nghĩa trên wire |
| :--- | :--- |
| `ECX[i]` (0-based, `iVar3 + i`) | `payload[1+i]` |
| `_LStrCopy(ECX, p, n, &out)` (Delphi 1-based) | cắt `n` byte từ `ECX[p-1]` = **`payload[p]`** |
| `FUN_0077EF7C(&out)` → **DWORD LE** | `b0 + b1*0x100 + b2*0x10000 + b3*0x1000000` (`0077ef7c_FUN_0077ef7c.c`) |
| `FUN_0077EB9C(&out)` → Word LE | `b0 + b1*0x100` (không dùng trực tiếp ở 0x13, chỉ helper) |

---

## 3. Bảng tổng hợp SubOp & wire layout (S→C)

| SubOp | Độ dài payload tối thiểu | Layout payload | Hành vi | Handler / helper |
| :---: | :---: | :--- | :--- | :--- |
| `0x01` | **6** | `[13][01][unitID: DWORD LE]` → `unitID = payload[2..5]` | Chọn NPC đội hình | `FUN_007a63dc` (`007a63dc.c:45–49`) |
| `0x02` | 2 | `[13][02]` (không field) | Bỏ chọn | `FUN_0072b34c` (`0072b34c.c:18–25`) |
| `0x04` | ≥2 (opaque) | `[13][04][<RestPayload>]` | Roster đội (suy luận) | `func_0x007a2ae8` — **chưa có** |
| `0x06` | ≥2 (opaque) | `[13][06][<RestPayload>]` | Quản lý NPC (suy luận) | `func_0x007a4d68` — **chưa có** |

Ghi chú SubOp 1: `FUN_007a63dc` gọi `_LStrCopy(ECX,2,4)` = `payload[2..5]`; nếu `len(ECX) < 5` (payload < 6 byte) thì `FUN_0077EF7C` chạm BoundErr ⇒ **cần đúng 6 byte**.

---

## 4. Chi tiết từng nhánh + helper

### 4.1. `SubOp 0x01` — Chọn đơn vị trong đội hình NPC (`FUN_007a63dc`)

`FUN_007a63dc(param_1 = gvar_007D9E00, param_2 = RestPayload)` (`007a63dc.c:23–57`):
```c
_LStrCopy(RestPayload, 2, 4, &s);                       // s = payload[2..5]
local_10 = FUN_0077EF7C(codec, s);                      // unitID = DWORD LE
FUN_0072B2D8(LocalActor /*gvar_007DA7BC*/, unitID);     // <- thao tác chọn (bỏ param_1!)
*(gvar_007DA530 + 0x1E4) = FUN_0072C124(LocalActor);    // MainStatus trỏ tới đơn vị vừa chọn
```
**Điểm đáng chú ý:** `param_1` (object `gvar_007D9E00`) được lưu `local_8 = param_1` (`:39`) nhưng **không hề được đọc** trong toàn hàm → nhiều khả năng đây là self của Delphi method bị compiler loại (stub), không ảnh hưởng logic.

### 4.2. `FUN_0072B2D8` — ghi trạng thái chọn (`0072b2d8.c:21–39`)
```c
bVar1 = FUN_0071D1BC(LocalActor, unitID);   // tìm slot đội hình (1..4) chứa ID này
if (bVar1 != 0) {                           // nếu KHÔNG tìm thấy -> không làm gì
    LocalActor+0x12E1 = unitID;             // ID đơn vị đang chọn
    LocalActor+0x12E5 = bVar1;              // slot (byte, 1..4)
    FUN_0063AFA4(gvar_007DA0CC);            // refresh menu danh sách NPC (TCY_FNpcManageMenu)
    FUN_0058E0BC(gvar_007DA530);            // refresh thanh trạng thái chính (TSe_MainStatus)
}
```
`FUN_0071D1BC` (`0071d1bc.c:118–147`) duyệt **đúng 4 ô đội hình** `LocalActor+0x57C[slot*4]` (slot 1..4), chọn ô có `unit+0x78==1` (đang hoạt động) và `unit+4 == unitID`; không thấy thì trả 0 → **SubOp 1 chỉ có tác dụng nếu `unitID` thật sự thuộc đội hình 4 ô của player**. ⇒ Server không "thêm" NPC mà chỉ "chọn" một NPC đang có.

### 4.3. `FUN_0072C124` — lấy con trỏ đơn vị đang chọn (`0072c124.c:24–38`)
```c
if (LocalActor+0x12E5 != 0)                  // có slot đang chọn (1..4)
    return *(LocalActor + 0x57C + slot*4);   // -> con trỏ đơn vị; ngược lại 0
```
Kết quả được `FUN_007a63dc` ghi vào **`TSe_MainStatus+0x1E4`** — field "đơn vị đang được hiển thị trên thanh trạng thái". Cùng field này được `FUN_0058BC38` (TSe_MainStatus) gán khi self-click ⇒ **SubOp 1 chuyển thanh trạng thái chính sang NPC vừa được server chọn.**

### 4.4. `SubOp 0x02` — Bỏ chọn (`FUN_0072B34C`, `0072b34c.c:18–25`)
```c
LocalActor+0x12E5 = 0;  LocalActor+0x12E1 = 0;
TSe_MainStatus+0x1E4 = 0;
FUN_0063AFA4(gvar_007DA0CC);   // refresh menu NPC
```
Hoàn toàn đối xứng SubOp 1 (không đọc thêm payload).

### 4.5. Preamble dọn mục tiêu (chỉ chạy ở SubOp 1 & 2) — `case_017.c:30–38, 44–52`
```c
if (LocalActor+0x376 != 0 && gvar_007DA51C+0xE78 != -1) {   // có cờ chọn & đang có mục tiêu
    slot = gvar_007DA51C+0xE78;                              // chỉ số mục tiêu 0..20
    if (gvar_007DA51C+0x158[slot*4])->+0x79 == 4)            // đơn vị đó đang ở state 4
        FUN_00650E40(gvar_007DA51C);                         // bỏ chọn mục tiêu + refresh HUD
}
```
- `gvar_007DA51C` = **bộ quản lý danh sách đơn vị + "target đang chọn bằng chuột"**: mảng `[0..20]` con trỏ tại `+0x158`, slot mục tiêu tại `+0xE78` (`-1` = không chọn). Được click chuột đổ vào (`0050bff8_TForm1.DXDraw1MouseDown.c:99`).
- `FUN_00650E40` (`00650e40.c:24–33`): đặt `mgr+0xE5B=0xFF`, `mgr+0xE78=0xFF` (**hủy chọn mục tiêu**), rồi gọi **Refresh** (vtable+0x24) của 5 form HUD (`gvar_007DA32C` EquipForm2, `gvar_007D9D7C` TeamForm, `gvar_007D9E5C` StatusInfoForm, `gvar_007D9EB8` HUD, `gvar_007DA1F0` FightForm). (Có kèm tắt âm `FUN_0079b620` — **bỏ qua, thuộc sound**.)
- **Ý nghĩa:** khi server đổi chọn đội hình (SubOp 1/2), nếu mục-tiêu-chuột đang trỏ vào một đơn vị vừa chuyển sang `state 4`, thì phải **giải phóng selection cũ** để HUD không treo trên đơn vị hết hiệu lực.

### 4.6. Hậu đề — `SubOp 0x04`
`func_0x007a2ae8(gvar_007D9D64 /*TCY_TeamManage*/, RestPayload)` rồi `FUN_005A3018(gvar_007D9D7C /*TSe_TeamForm*/)`.

### 4.7. `FUN_005A3018` — Refresh TeamForm (gọi sau SubOp 1, 2, 4) — `005a3018.c`
Thuần **UI của `TSe_TeamForm` (gvar_007D9D7C)**, **không gửi mạng, không toast**:
- Ẩn/hiện + đặt caption hàng loạt control (`form+0x100[0..12]`, `+0xF0[0..3]`, `+0x2B4[0..3]`) qua `FUN_007B0094` (Visible) / `FUN_007B0628` (Caption).
- `form+0x140 = LocalActor+0x12E1` (`:119`) — lấy **ID NPC đang chọn** (chính SubOp 1 ghi) để tô sáng dòng tương ứng.
- Vẽ **đội hình NPC**: `LocalActor+0x35E` = số NPC trong đội (≤4), dịch "vị trí đội hình" qua `FUN_0071D234` (`unit+0x55C` → slot 1..4), chọn ảnh nút theo `unit+0x55F` (1=còn sống→`btn_AssignRest`, 2→`btn_Dead`), so `unit+4 == form+0x140` → bật nút chiến đấu.
- Vẽ **party**: `LocalActor+0x548` (leader/team ID, ≠0 = đang có đội), `+0x578` = số thành viên (≤4), `+0x550 + s*8` = cặp `(memberID, memberIndex)`; chọn `btn_dismiss` (mình là trưởng) vs `btn_LeaveTeam`.
- Các `btn_*` là **tên file ảnh** tra qua `FUN_007C9B38` (`<name>.bmp`) → **graphics, không hiển thị text cho người chơi.**

### 4.8. `SubOp 0x06`
`func_0x007a4d68(gvar_007D9E00 /*TFNpcManage*/, RestPayload)` — **chưa trích xuất decompile** ⇒ không dựng được wire.

---

## 5. Bảng giải nghĩa global (suy từ cách dùng trong `ts_decompile/`)

| Symbol | Vai trò | Bằng chứng / nơi tạo |
| :--- | :--- | :--- |
| `gvar_007DA7BC` → **LocalActor / TPlayer** | nhân vật local; fields chọn đội hình: `+0x12E1` (ID NPC đang chọn), `+0x12E5` (slot 1..4), `+0x35E` (số NPC đội ≤4), `+0x57C[1..4]` (ptr NPC đội), `+0x55C` (vị trí đội hình), `+0x55F` (sống/chết), `+0x548` (team/leader ID), `+0x578` (số thành viên party), `+0x550+s*8` (cặp memberID/index), `+0x376` (cờ state đơn vị) | Tạo `00603f20.c:189–190` (`TPlayer_Create`); field `+0x376` ghi từ byte wire tại `007450f4.c:248`; còn lại `005a3018/0072b2d8/0071d1bc/0071d234` |
| `gvar_007DA530` → **TSe_MainStatus** | thanh trạng thái chính; `+0x1E4` = con trỏ đơn vị đang hiển thị | `0051189c.c:1576–1577`; dùng `007a63dc.c:49`, `0072b34c.c:23` |
| `gvar_007D9D7C` → **TSe_TeamForm** | form đội/NPC, đích refresh của `FUN_005A3018` | `0051189c.c:1570–1571` |
| `gvar_007DA0CC` → **TCY_FNpcManageMenu** | menu danh sách NPC đội (refresh qua `FUN_0063AFA4`) | `0051189c.c:1304–1306` |
| `gvar_007D9E00` → **TFNpcManage** | cửa sổ quản lý NPC (self SubOp 1 & 6) | `0051189c.c:1389–1390`; SubOp 6 → `func_0x007a4d68` |
| `gvar_007D9D64` → **TCY_TeamManage** | panel quản lý đội (self SubOp 4) | `0051189c.c:1387–1388`; họ hàm `007a2604/007a273c` |
| `gvar_007DA51C` → **UnitList/Target manager** | mảng mục tiêu `[0..20]@+0x158`, slot chọn `+0xE78`, `+0xE5B` | click `0050bff8.c:99`; hủy chọn `00650e40.c:24–25`; **nơi tạo: UNKNOWN** |
| `gvar_007D9D30` | codec/self khi gọi `FUN_0077EF7C` | `007a63dc.c:46`; **giá trị thật: UNKNOWN** |

---

## 6. Chiều C→S

- `0077f414_FUN_0077F414.c:923–924`: `case 0x13: break;` → **artifact jump-table** (bản C không khôi phục được body).
- Bảng gửi C→S (`0x77F474` byte → `0x77F53C` dword): chỉ có mặt trong `.asm.txt` (`:33–34`: `MOV AL,[EAX+0x77f474]` / `JMP [EAX*4+0x77F53C]`), **vùng data của 2 bảng này KHÔNG có trong `redump/`** ⇒ không định vị được template payload gửi của 0x13 từ single source.
- Kiểm chứng chéo: quét các caller của `FUN_0077f414` — **không có lệnh `MOV DL, 0x13`** nào. Thao tác đội phía client được gửi bằng **OP 0x0F** (`005a197c.c:165,168`, `005a217c.c:22`), không phải 0x13.
- **Kết luận:** độ tin cậy cao rằng **client không gửi OP 0x13**; đây là kênh S→C một chiều.

---

## 7. Chuỗi literal & encoding (dịch sang UTF-8)

**Trong 4 nhánh của handler 0x13 + các helper đã khôi phục: KHÔNG có chuỗi in-game nào** — chỉ có:
- Tên file ảnh `btn_AssignRest/btn_Dead/btn_AssignFight/btn_dismiss/btn_LeaveTeam/btn_AssignConsole/btn_AssignTeamMate` (ASCII thuần, nạp `.bmp`) — **không cần dịch, thuộc graphics**.
- Caption `DAT_005A402C`, `DAT_005A4044` (2 nhãn trạng thái dòng NPC trong TeamForm) và `LAB_0063B0DC` (suffix nối sau tên NPC đang chọn) → **chưa có dump `lit_*` cho các địa chỉ này ⇒ không dịch được từ single source**.

**Bối cảnh lân cận (không thuộc handler 0x13 nhưng cùng cụm TeamManage để tham chiếu):** `redump/lit_7A2094.hex`, `redump/lit_7A20A8.hex` chứa 2 nhãn toast của `FUN_007A1964` (hàm này được gọi từ **MainOp 0x01 Case 4**, không phải 0x13):

| Địa chỉ | Byte (ShortString `[len:1][chars]`) | UTF-8 |
| :--- | :--- | :--- |
| `0x007A2094` | `10 47 69 E4 69 20 74 E1 6E 20 F0 B5 69 20 6E 67 FB` | **Giải tán đội ngũ** |
| `0x007A20A8` | `0E 52 B6 69 20 62 F6 20 F0 B5 69 20 6E 67 FB` | **Rời bỏ đội ngũ** |

> **Hiệu chỉnh encoding:** kiểm chứng bằng Python cho thấy `windows-1258/cp1258` cho ra `Giäi tán đµi ngû` (**SAI**). Bảng đúng là **đơn-byte tiền tổ hợp VISCII/TCVN-5712** với các neo: `E4=ả, E1=á, F0=đ, B5=ộ, FB=ũ, B6=ờ, F6=ỏ`. Kết quả đọc được xác nhận chéo bởi ngữ nghĩa party (giải tán/rời bỏ đội). ⇒ Nhận định của `opcode_09.md` ("cp1258") **không đứng vững** với các byte này; cần rà lại toàn dự án (nằm ngoài phạm vi tài liệu này).

---

## 8. Ghi chú cho Mock Server

1. **Chọn NPC đội hình**: gửi `[13][01][unitID:4B LE]`. **Bắt buộc** `unitID` đã tồn tại trong đội hình 4 ô (`LocalActor+0x57C[1..4]`, `+0x78==1`) — nếu không client **im lặng bỏ qua** (`FUN_0071D1BC` trả 0). Đây là kênh *chọn* chứ không phải *thêm* thành viên.
2. **Bỏ chọn**: gửi `[13][02]` (2 byte). An toàn, không điều kiện.
3. **Không phải để điều khiển party roster**: mọi request party của client đi **OP 0x0F**; mock chỉ cần đẩy 0x13 khi muốn **tự chuyển selection/HUD** phía client.
4. **SubOp 0x04 / 0x06**: chưa đặc tả được vì `func_0x007a2ae8` / `func_0x007a4d68` vắng mặt trong decompile → **cần dump 2 hàm này** trước khi mock dựng panel đội/NPC đầy đủ.
5. SubOp ngoài {01,02,04,06} vô hại nhưng vô nghĩa — client không xử lý.

---

## 9. Source trail (file / địa chỉ đã xác minh)

| # | Nguồn | Dùng cho |
| :--- | :--- | :--- |
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` (mục 1, 2) | framing, dispatcher, mapping |
| 2 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` | xác minh `0x13 → idx 0x11 → 0x0078EADC` |
| 3 | `ts_decompile/case_functions/functions/case_017_0078EADC_FUN_0078eadc.c:21–93` | **handler chính** + preamble + các switch |
| 4 | `007a63dc_FUN_007a63dc.c`, `0072b2d8_FUN_0072b2d8.c`, `0072c124_FUN_0072c124.c`, `0071d1bc_FUN_0071d1bc.c:118–147`, `0072b34c_FUN_0072b34c.c` | logic chọn/bỏ chọn NPC (SubOp 1,2) |
| 5 | `00650e40_FUN_00650e40.c:24–33`, `005a3018_FUN_005a3018.c` | preamble hủy target + hậu đề refresh TeamForm |
| 6 | `0051189c_FUN_0051189c.c:1304–1585`, `00603f20_FUN_00603f20.c:189–190`, `007450f4_FUN_007450f4.c:248` | gán tên VMT cho gvar + field `+0x376` |
| 7 | `0077f414_FUN_0077F414.c:923–924` + `.asm.txt:33–34`; `005a197c`, `005a217c` | chiều C→S (không có 0x13; party dùng 0x0F) |
| 8 | `redump/lit_7A2094.hex`, `lit_7A20A8.hex`; kiểm bằng `python3` | bảng chuỗi VISCII (mục 7) |
| 9 | `0077ef7c_FUN_0077ef7c.c`, `0077eb9c_FUN_0077eb9c.c` | codec DWORD/Word LE |

**UNKNOWN / cần redump tiếp:** wire của SubOp 4 & 6 (`func_0x007a2ae8`, `func_0x007a4d68` chưa có body); nội dung `DAT_005A402C/4044`, `LAB_0063B0DC` (chưa dump); nghĩa chính xác `LocalActor+0x376`, `unit+0x79==4`, `unit+0x55F`; nơi tạo `gvar_007DA51C` và `gvar_007D9D30`.
