# PHÂN TÍCH — Main OP 0x36 (Case 47) — `FUN_00795494` @ `0x00795494` — **Batch trạng thái/tín hiệu SERVER trong form chọn server (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x36] = 0x2F (=47)` → `jumptable_dword200_0x78A9B6[47]` (entry `0x0078AA72`) → target `0x00795494` = **Case 47**. Đối chiếu kép: inline trong dispatcher tổng `0078a89c_FUN_0078a89c.c:6791–6815` (marker `UNK_0079549c`) khớp `case_047`.

> Phạm vi: **core logic opcode**; phần Paint render chỉ nêu để chứng minh consumer của dữ liệu.

---

## 1. Tóm tắt nghiệp vụ — CẤU TRÚC KHÁC BIỆT NHẤT TỪ TRƯỚC ĐẾN NAY

**OP 0x36 KHÔNG CÓ SubOp.** Đây là opcode duy nhất (trong các case đã khảo sát) mà payload sau MainOp được cắt thành **dãy CẶP 2 byte lặp lại**, mỗi cặp ghi một **mức trạng thái 0..3** vào **một dòng của danh sách server** trong form chọn server (`gvar_007D9CC4`):

```
Payload = [0x36][idx1][lv1][idx2][lv2][idx3][lv3]...   (tổng độ dài sau MainOp PHẢI chẵn)
```

- `idx`: **1-based** (chỉ số dòng server); `idx = 0` hoặc vượt số dòng → **cặp bị bỏ qua im lặng** (không crash).
- `lv`: 0..3 = mức tín hiệu; `lv > 3` → **clamp về 0**.
- Sender: form là **danh sách server để chọn/chuyển server trong game** — có `icon_ServerSignal` (sprite 4 frame), `btn_quit`, `btn_logIn` ở form tài khoản đi kèm. Form được **mở bởi OP 0x01 SubOp 0x09** (scene-switch — `case_002.c:176–179` → `FUN_0051aae0`, đã xác minh §5.2).
- **C→S CÓ THẬT** (hiếm): client gửi đúng **1 byte `[0x36]`** — body khôi phục được trong `0077f414.c` (§6). Nghi vấn: "xin server gửi lại bảng trạng thái" khi mở form.

**Edge case nguy hiểm cần biết khi mock:** độ dài lẻ ⇒ vòng lặp cuối đọc `RP[1]` fail bounds → **ERangeError (`_BoundErr`) giữa chừng** (`case_047.c:28–31`); các cặp trước đó đã áp dụng, phần còn lại mất. **Luôn gửi số cặp chẵn.**

---

## 2. Entry & cách đọc payload

Framing/dispatcher: `[F4 44][Len:Word LE][Payload]` XOR `0xAD` (handoff mục 1). Pipeline đã xác minh: `TForm1.CY_DelRevQueue` (`00516158_TForm1.CY_DelRevQueue.c:86–93`) cắt `_LStrCopy(msg, 2, Len-1)` → **RP = payload bỏ MainOp** rồi `FUN_0078a89c(RP, msg[0])`.

`FUN_00795494` (`case_047.c:23–42`, ĐÃ ĐỌC TOÀN BỘ):
```c
while (_LStrLen(RP) >= 1) {
    b = RP[1];                       // BoundErr nếu len(RP) < 2   (payload[2])
    a = RP[0];                       // BoundErr nếu len(RP) == 0  (payload[1])
    FUN_00708900(gvar_007D9CC4, a, b);
    _LStrDelete(RP, 1, 2);           // xóa 2 ký tự từ vị trí 1 (1-based) = bỏ cặp đã dùng
}
```
⇒ Không có khái niệm SubOp byte ở opcode này: **`payload[1]` đã là `idx` đầu tiên**.

---

## 3. Wire layout

| Offset payload | Nội dung | Ràng buộc client |
| :--- | :--- | :--- |
| `[0]` | `0x36` | — |
| `[1], [2]` | cặp #1: `idx1` (1-based, byte), `lv1` ∈0..3 | idx ngoài `[1..Length(array)]` → bỏ qua; lv>3 → ghi 0 |
| `[3], [4]` | cặp #2 | như trên |
| … | lặp đến hết; **độ dài phần cặp phải CHẴN** | len lẻ → ERangeError ở cặp cuối |

Không có field chuỗi/ID lớn — mọi thứ là byte đơn.

---

## 4. Phân tích helper

### 4.1. `FUN_00708900(mgr, a, b)` — setter trạng thái (`00708900.c:21–61`, ĐÃ XÁC MINH TRỰC TIẾP)
```pascal
// Tương đương Delphi:
if (a >= 1) and (a-1 < High(ServerStates)) then        // ServerStates = dynamic array of Byte @ mgr+0x150
  if b > 3 then ServerStates[a-1] := 0
  else ServerStates[a-1] := b;
```
- `FUN_00405B24` (`00405b24.c:72–79` = `FUN_00405B1C(x)-1`; `00405b1c.c:125–135` = `if x<>0 then PDWORD(x-4)^ else 0`) chính là **`High()` của dynamic array Delphi** ⇒ **`+0x150` là con trỏ dữ liệu dynamic array of Byte** (độ dài tại `ptr-4` — khớp đúng phép thử bounds `*(uint*)(p-4) <= idx` trong `00708900.c:53`).
- Array chưa cấp phát (`nil`) ⇒ High=-1 ⇒ **mọi gói bị bỏ qua an toàn** (form chưa init không crash).

### 4.2. Nơi TIÊU THỤ mảng — `FUN_00708D10` (paint/hover của form, `00708d10.c:59–186`)
Bằng chứng chuỗi số học `* 0x12` (18):
```pascal
row_idx := list.TopLine + i;
state   := ServerStates[row_idx];                 // (+0x150)[k]
Rect    := Bounds(state*18, 0, 18, ...);          // frame thứ `state` của icon_ServerSignal
Draw(ImageManager, icon_ServerSignal, ..., Rect); // 00708d10.c:94-129
if PtInRect(row, Mouse) and (state <= 3) then     // hover:
   TextOut(red 0xff0000 / shadow 0xf98b3d, PTR_PTR_007d9cbc[state*4]);  // 4 label trạng thái
```
⇒ `icon_ServerSignal` là **dải 4 frame 18px** (mỗi mức 1 ảnh); hover hiện **nhãn chữ màu đỏ** tra từ bảng 4 chuỗi toàn cục `PTR_PTR_007D9CBC` (chưa dump — §7).

### 4.3. Các hàm cùng class (`VMT PTR_PTR_00707BE8`) đã export
| File | Vai trò |
| :--- | :--- |
| `007083b4.c` | **Create form**: `panel15` 300×421, `panel8` 216×376, **TStringList `+0x138`**, list control `+0x130` (200 dòng), scrollbar `+0x134`, `btn_quit +0x140` (`FUN_0044CCE0` = exit, owner TFConnect), `icon_ServerSignal +0x144`, `icon_SelectServer +0x148` (`007083b4.c:62–118`) |
| `00708738.c` | Load **TopLine** từ `user\save.dat` + đổ danh sách (Refresh/Show path — **một phần trong HOLE chưa export**) |
| `00708978.c` | Save **TopLine** vào `user\save.dat` khi đóng |
| `00708d10.c` | Paint/hover (§4.2) |

**3 call-site khác của `FUN_00708900`** (header `00708900.c:15–17`: `00519f4d`, `00708333`) và nơi **SetLength mảng `+0x150`** đều nằm trong **HOLE `0x00707BC1–0x007083B4`** (không hàm nào phủ theo `index.csv` — đã kiểm bằng python) ⇒ nhiều khả năng method Show/Refresh của form tự reset trạng thái + cấp phát mảng theo số dòng, nhưng **không khẳng định được từ single source**.

---

## 5. Global & vòng đời form

### 5.1. Bảng field `gvar_007D9CC4` (form chọn server)

| Offset | Field | Bằng chứng |
| :--- | :--- | :--- |
| `+0x130` | List control (200 dòng; `+0xE4` TopLine, `+0x120` số dòng hiển thị; callback vẽ `DAT_00708B30`) | `007083b4.c:78–85`, `00708d10.c:61–62` |
| `+0x134` | Scrollbar `bar_H3/rail_H3` | `007083b4.c:86–106` |
| `+0x138` | **TStringList** — tên các server | `007083b4.c:76–77` (`param_1[0x4e]`) |
| `+0x13c` | `panel8` (nền danh sách, offset `+0x18`) | `007083b4.c:70–74` |
| `+0x140` | `btn_quit` | `007083b4.c:107–114` |
| `+0x144` | `icon_ServerSignal` (index ImageManager `gvar_007D9ED8`) | `007083b4.c:65–66` + paint `00708d10.c:128` |
| `+0x148` | `icon_SelectServer` | `007083b4.c:67–68` |
| `+0x14c` | TopLine persist (`user\save.dat`) | `00708738.c:62–93`, `00708978.c:59–85` |
| `+0x150` | **dynamic array of Byte — trạng thái 0..3 từng dòng server** | `00708900.c:37,52–57`; `00708d10.c:85–94` |

### 5.2. Vòng đời
1. **Create + Hide ngay**: `0051189c.c:1394–1396` tạo qua `FUN_007083B4(PTR_PTR_00707BE8)`; `:1744` gọi `vtable+0x20` (Hide).
2. **Show**: **OP 0x01 SubOp 0x09** (`case_002_0078B149.c:176–179` → `FUN_0051AAE0`, ĐÃ TỰ KIỂM CHỨNG): đọc `RP[1]` → `DAT_00926FC6` (id server đích), gọi **`vtable+0x24` (Show)** trên `gvar_007D9CC4`, đổ tài khoản/server-id vào form phụ `gvar_007D9E7C` (**`TSe_IDPassored`** — form nhập account server, `0051189c.c:1397–1399`) từ `DAT_0092530C+0x644/0x648` rồi bấm login hộ (`FUN_007095C0`) — tức luồng **chuyển server/autologin**.
3. **Hide khi out-world**: `00603f20.c:140` (`vtable+0x20`) trong hàm reset rời world (`:100–101` đóng socket).
4. **OP 0x36** (file này) chỉ có nghĩa khi form đang/ vừa mở: server đẩy định kỳ/sự kiện bảng trạng thái tín hiệu để **vẽ lại cột icon 4 mức**.

---

## 6. Chiều C→S — KHÔI PHỤC ĐƯỢC BODY THẬT (hiếm)

`0077f414_FUN_0077F414.c:1007–1011`:
```c
case 0x36:
  _LStrFromChar(s, param_2);                     // 1 ký tự = low byte param_2 = MainOp 0x36
  TForm1_CY_AddSedQueue(TFConnect /*gvar_007DA664*/, s);
  break;
```
⇒ **Client gửi đúng gói `[0x36]` (1 byte, không field)** — cùng khuôn "ký tự đầu = opcode" đã đối chiếu `case 7` (`0077f414.c:884–895`). Call-site `MOV DL,0x36` không có trong 6302 asm đã export (đã quét python) ⇒ caller nằm vùng chưa export; mục đích chính xác **không khẳng định được**, nhiều khả năng là *request bảng trạng thái khi mở form*.

---

## 7. Chuỗi literal & encoding (VISCII→UTF-8)

- Handler + setter + paint: **0 chuỗi hiển thị** (chỉ hằng tọa độ/màu; `LAB_/UNK_` là SEH marker).
- Hằng ASCII thuần (không cần dịch): `"user\\save.dat"`, `"TopLine"`, `"TopLine="` (`00708738/00708978`), `"icon_ServerSignal"`, `"icon_SelectServer"`, `"panel8"`, `"btn_quit"`, `"bar_H3"`, `"rail_H3"` (`007083b4`).
- **4 nhãn trạng thái** vẽ khi hover: bảng con trỏ `PTR_PTR_007D9CBC[state*4]` (`00708d10.c:180–181`) — vùng data **không có trong `redump/` → chưa dịch được** (không bịa). Kỳ vọng nội dung: kiểu "Đông/Trung bình/Vắng/Bảo trì" hoặc 4 mức ping — **UNKNOWN**.
- Tên debug opcode 0x36 tại `0x00796CAC` (`0078a89c.c:590,751–752`): không có dump → không dịch được.

---

## 8. Ghi chú cho Mock Server

1. **Wire**: `[0x36] + k×[idx:1B][lv:1B]`, `lv∈{0,1,2,3}`, `idx` **1-based**, **tổng byte sau MainOp chẵn**. Ví dụ cập nhật server #1 tốt, #3 đông: `36 01 00 03 03`.
2. **Gửi khi client đã/đang mở form chọn server** (sau `0x01/0x09`). Trước đó mảng `+0x150` có thể nil ⇒ vô hại nhưng vô nghĩa.
3. **Không gửi `idx` vượt số dòng** thực tế của danh sách server (không crash, chỉ bị ignore — nhưng đừng trông chờ hiển thị).
4. Có thể gộp **nhiều cặp trong 1 frame** — handler loop đến hết payload.
5. Nhận phía server **`[0x36]` 1 byte** từ client (request làm mới) — nên respond bằng đúng gói batch trên.
6. Giá trị `lv` mapping sang frame thứ `lv` của `icon_ServerSignal` — chọn mức theo thông số server muốn thể hiện; **mock không cần biết nội dung nhãn** (client tự vẽ bằng bảng chuỗi cục bộ).

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng trực tiếp:**
1. `case_functions/functions/case_047_00795494_FUN_00795494.c:23–42` — handler loop.
2. `functions/0078a89c_FUN_0078a89c.c:6791–6815` — inline dispatcher (xác nhận kép).
3. `functions/00708900_FUN_00708900.c:21–61` — setter; `00405b24.c:72–79` + `00405b1c.c:125–135` — High()/Length() Delphi dyn-array.
4. `functions/007083b4_FUN_007083b4.c:32–120` — Create form (toàn bộ field + ảnh `00708d10.c:85–135`).
5. `case_functions/functions/case_002_0078B149_FUN_0078b149.c:176–179` + `functions/0051aae0_FUN_0051aae0.c:30–70` — Show path qua OP 0x01 sub 9 (**đã tự đọc**).
6. `functions/0051189c_FUN_0051189c.c:1394–1399,1744` — create/hide + form `TSe_IDPassored` kề sau; `00603f20.c:100–101,140` — hide out-world.
7. `functions/00516158_TForm1.CY_DelRevQueue.c:86–93` — RP cắt bỏ MainOp trước dispatcher.
8. `functions/0077f414_FUN_0077F414.c:1007–1011` — body C→S; quét 6302 asm không có call-site `DL,0x36`.
9. `redump/jumptable_*.hex` — dispatch `0x36→47→0x00795494`; kiểm HOLE `0x707BC1–0x7083B4` qua `index.csv` bằng python.

**UNKNOWN (cần redump):** điểm SetLength mảng `+0x150` + nơi nạp tên server vào `+0x138` (HOLE `0x707BC1–0x7083B4`); 2 method ở HOLE `0x70868E–0x708738` & `0x708AC4–0x708D10`; nội dung 4 label `PTR_PTR_007D9CBC` và tên op tại `0x796CAC` (không dump); call-site C→S `[0x36]`; ý nghĩa `DAT_00926FC6` (id server hay channel — thuộc luồng OP 0x01/0x09, khảo sát riêng).
