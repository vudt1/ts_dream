# BẢN ĐỒ MAP ID ↔ HÀNH VI CLIENT — `LocalPlayer + 0x63A`

> **Phát hiện từ Round-5:** `GVAR 0x007DA3F0` chỉ ra `FUN_007994A4` (đọc con trỏ `TKC_Scene`),
> và hàm đó gọi `FUN_0053558C` — **hàm tra cứu tên map theo (mapId, kind)**.
>
> **Phát hiện quan trọng nhất:** `*(int16*)(LocalPlayer + 0x63A)` = **map id hiện tại của người chơi**.
> Client so sánh trường này ở **86 hàm** với **39 map id khác nhau** — tức là **server phải
> đặt đúng map id** thì logic client-side của sự kiện mới chạy.

---

## 1. Xác định map id

| Thành phần | Địa chỉ | Ghi chú |
|---|---|---|
| LocalPlayer | `*gvar_007DA7BC` (offset `+0x63A`) | `int16` |
| Bản sao (màn hình khác) | `DAT_0092530C + 0x63A` | dùng ở `TForm1.Transition`, `KeyboardWalk`… |
| Manager map | `*gvar_007DA3F0` = `TKC_Scene` (VMT `0x00535380`) | nạp `Data\Scene.Dat` qua `FUN_00535600` |
| Object thông tin map hiện tại | `*gvar_007DA5A0` | `+0xA0AE` = mapId, `+0xA0B0` = X, `+0xA0B4` = Y, `+0xA0B8` = kind, `+0xA0B9` = tên map (20 byte) |

## 2. Thuật toán tra cứu tên map (`FUN_0053558C` — chưa có trong corpus)

Đọc trực tiếp bytecode trong `aLogin.exe`:

```
FUN_0053558C(self, mapId:i16, kind:u8, out:ptr)
  0053558C  PUSH EBP; MOV EBP,ESP; ADD ESP,-0x0C
  00535594  MOV  [EBP-7],CL          ; kind
  00535597  MOV  [EBP-6],DX          ; mapId
  0053559B  MOV  [EBP-4],EAX         ; out
  0053559E  MOV  EAX,[EBP+8]         ; self
  005355A1  XOR  ECX,ECX
  005355A3  MOV  EDX,0x86            ; 134 = STRIDE ban ghi Scene.Dat
  005355A8  CALL 0x00522CD0          ; index = f(mapId, kind) * 134
  005355AD  MOV  CL,[EBP-7]
  005355B0  MOV  DX,[EBP-6]
  005355B4  MOV  EAX,[EBP-4]
  005355B7  CALL 0x00535864          ; -> word index (tim theo mapId+kind)
  005355BC  MOV  [EBP-0xA],AX
  005355C0  CMP  word [EBP-0xA],0
  005355C6  JZ   end                ; index==0 -> khong doi ten
  005355C8  MOVZX EAX,word [EBP-0xA]
  005355CC  CMP  EAX,0x1F40          ; 8000 = gioi han
  005355D1  JBE  ok
  005355D3  CALL BoundErr
  005355D8  IMUL EAX,0x43            ; 67 word = 134 byte
  005355DB  JNO  ok2
  005355DD  CALL IntOver
  005355E2  MOV  EDX,[EBP-4]         ; out
  005355E5  MOV  ECX,[EBP+8]         ; self
  005355E8  LEA  ESI,[EDX+EAX*2+0x18]; dest = out + 0x18 + index*134
  005355EC  MOV  EDI,ECX             ; src  = self
  005355EE  MOV  ECX,0x21             ; 33 dword = 132 byte
  005355F3  REP  MOVSD
  005355F5  MOVSW                    ; +2 byte -> 134 byte
  005355F7  RET
```

⇒ **Tên map (134 byte) được copy từ bảng trong `TKC_Scene` sang `out + 0x18 + index*134`.**
⇒ `FUN_00535864` là hàm **tìm chỉ số bản ghi** theo cặp `(mapId, kind)` trong cấu trúc nội bộ
(đọc `u16` tại `[self+4]` làm số lượng phần tử).

> ⚠ **Cả 2 hàm (`0x0053558C`, `0x00535864`) + 4 method `TKC_Scene` khác đều KHÔNG có trong
> corpus Ghidra** ⇒ đã đưa vào manifest **Round-6**.

## 3. Nhánh đặc biệt map 12003 (Lôi Đài)

Trong `FUN_007994A4`:

```c
if (DAT_009f1650 == '\0') {
  if (local_8e == -0x17f7 && (*(short *)(gvar_007DA5A0 + 0xa0ae) == 0x2ee3)) {
      // copy 5 word tu DAT_00799930
```

* `0x2EE3` = **12003** — map của **Lôi Đài Khiêu Chiến** (đúng map mà `data_event_quests.tsv`
  chỉ ra: quest 10714/10715 → scene 12003).
* `DAT_00799930` = chuỗi **"Quảng trường Quận Hòa"** (20 byte) — tên thay thế khi ở trong
  đấu trường (đấu trường nằm *trong* quảng trường nên client hiện tên plaza).
* `local_8e == -0x17F7` → `u16` = **59393** — điều kiện phụ (giá trị lấy từ
  `FUN_0053558C`).

## 4. Bảng 39 map id client so sánh — `data_map_id_checks.tsv`

> ⭐ **Đã có tên map** (giải mã `Scene.Dat` — xem [`SCENE_DAT_FORMAT.md`](./SCENE_DAT_FORMAT.md)).

| map id | hex | cmp | số hàm | tên map (theo `Scene.Dat`) |
|---:|---:|---|---:|---|
| **0** | `0x0000` | !=,== | 9 | — |
| **3** | `0x0003` | == | 1 | — |
| **8** | `0x0008` | == | 2 | — |
| **10** | `0x000A` | == | 1 | — |
| **59** | `0x003B` | == | 1 | — |
| **10701** | `0x29CD` | == | 9 | *không có trong `Scene.Dat`* — map đầu của `eve.emg` |
| **10991** | `0x2AEF` | !=,== | 17 | QuảngTrườngTrácQuận / **Mê cung sinh tồn** |
| **10993** | `0x2AF1` | != | 1 | U Châu |
| **10994** | `0x2AF2` | == | 2 | QuảngTrườngTrácQuận |
| **12000** | `0x2EE0` | !=,== | 3 | **Ghi danh Đoan Ngọ / QuảngTrường ĐoànViên** / Giới Kiều / Lô Long Khẩu … |
| **12001** | `0x2EE1` | != | 1 | Chiếntrường dũng giả / Nguyên Tiêu ký vực / Tháp Lôi Phong |
| **12002** | `0x2EE2` | == | 1 | Tiệm trà  Cửa thành Trác Quận / Quan phủ / Bần hộ / QuảngTrườngTrácQuận |
| **12003** | `0x2EE3` | != | 1 | **Lôi đài thôn thực / Lôi đài 5 vs 5 / LôiĐàiHỏiĐápNămMới** |
| **12005** | `0x2EE5` | != | 1 | **Hội thi nướng thịt** |
| **12061** | `0x2F1D` | != | 1 | **Lôi đài tỉ võ / Đồng Tước đài** / Tháp thí luyện |
| **12180** | `0x2F94` | == | 2 | QuảngTrườngTrácQuận / **Chợ P-TS Online** |
| **12403** | `0x3073` | != | 1 | Đường mòn Từ Vô Sơn / Phòng nhỏ Từ Vô Sơn |
| **12405** | `0x3075` | != | 1 | Đỉnh Từ Vô Sơn / TừVôSơn TrắcPhong |
| **12406** | `0x3076` | == | 1 | **TừVôSơn TrắcPhong** |
| **12407** | `0x3077` | == | 1 | **TừVôSơn TrắcPhong** |
| **12418** | `0x3082` | == | 1 | **TừVôSơn TrắcPhong** |
| **12419** | `0x3083` | == | 1 | **TừVôSơn TrắcPhong** |
| **12421** | `0x3085` | != | 1 | U Châu / Đất tuyết Lô Long / Bộ lạc Giáng Sinh |
| **12711** | `0x31A7` | == | 1 | **Tiền tiêu Hổ Lao** (chiến trường thủy chiến) |
| **12804** | `0x3204` | == | 1 | Bia Chi Lâm |
| **12822** | `0x3216` | == | 1 | Rừng Trung Sơn |
| **12825** | `0x3219` | == | 1 | Thôn Lâu Tang / Rừng Lâu Tang |
| **12911** | `0x326F` | == | 2 | Q Trường NghiệpThành |
| **13452** | `0x348C` | == | 1 | Chân núi Ngoạ Ngưu  Sườn núi Ngoạ Ngưu |
| **13842** | `0x3612` | == | 1 | Rừng Nhương Sơn  Động Nhương Sơn |
| **13843** | `0x3613` | == | 1 | Động Nhương Sơn |
| **15841** | `0x3DE1` | == | 1 | Rừng Lư Giang  Từ Châu |
| **15842** | `0x3DE2` | == | 1 | — |
| **20852** | `0x5174` | == | 1 | Rừng Tấn Dương |
| **49019** | `0xBF7B` | != | 1 | Trấn Nhu Tu  Xưởng đóng tàu / U tàu Nhu Tu / QuảngTrườngTrácQuận |
| **49051** | `0xBF9B` | != | 1 | **Trấn Ngưu Chữ** / Ao Ngưu Chữ |
| **59000** | `0xE678` | != | 1 | **Phật Giới** / Tế đàn Tiên Giới |
| **60411** | `0xEBFB` | !=,== | 2 | Thôn U Linh / Bình Nguyên Tử Linh |
| **65535** | `0xFFFF` | != | 1 | — |

> **Vài map id không có trong `Scene.Dat`** (client so sánh nhưng bản đồ không tồn tại
> trong file, hoặc là giá trị đặc biệt): `0`, `3`, `8`, `10`, `59`, `10701`, `15842`,
> `65535`. Chúng nhiều khả năng là **map id động** (server tự sinh) hoặc ID đặc biệt.
>
> **Ý nghĩa với server:** bảng này là **danh sách map id mà client có hành vi riêng** —
> gần như tất cả đều là map sự kiện. Server phải đặt `mapid` đúng khi mở sự kiện, nếu không
> client sẽ chạy nhánh mặc định và sự kiện "không xảy ra" phía client.
>
> 🔴 **Thêm nữa:** client tra tên map bằng cặp **`(mapId, kind)`** — `kind` do server đưa
> xuống. Gửi đúng `mapId` nhưng sai `kind` ⇒ client hiện **sai tên bản đồ**
> (ví dụ `mapId=12003` có 15 `kind`: *Lôi đài thôn thực* (8), *Lôi đài 5 vs 5* (9),
> *LôiĐàiHỏiĐápNămMới* (10/11/12), *Tầng 1 Tâm Ma tháp* (13) …).
