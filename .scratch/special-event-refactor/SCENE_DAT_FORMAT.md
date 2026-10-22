# ĐẶC TẢ `Data/Scene.Dat` — BẢN GIẢI MÃ ĐẦY ĐỦ

> **Nguồn:** giải mã ngược từ client TS Online (`aLogin.exe`) — Round-6 Ghidra.
> Toàn bộ kết luận dưới đây suy ra từ **pseudo-C + ASM của client**, không phải từ server.
>
> | Hàm client | Địa chỉ | Vai trò |
> |---|---|---|
> | `FUN_00535600` | `0x00535600` | Nạp `Data\Scene.Dat` vào object `TKC_Scene` |
> | `FUN_0053542C` | `0x0053542C` | **Giải mã** các bản ghi trong bộ nhớ |
> | `FUN_00535864` | `0x00535864` | Tìm chỉ số bản ghi theo cặp `(mapId, kind)` |
> | `FUN_0053558C` | `0x0053558C` | Trả về **tên map** (134 byte) |
> | `TKC_Scene` | VMT `0x00535380` | Lớp quản lý |

---

## 1. Cấu trúc file

| Mục | Giá trị |
|---|---|
| Kích thước file | `1 002 454` byte |
| Kích thước bản ghi | **134 byte** (`0x86`) |
| Số bản ghi | **7 481** (`1 002 454 / 134 = 7 481`) |
| **Header riêng** | **KHÔNG CÓ** — 134 byte đầu tiên *là* bản ghi số 0 |

> ⚠ **Đính chính quan trọng so với lần giải mã trước:** bản `data_map_catalog.tsv` cũ
> coi 134 byte đầu là "header" và bắt đầu bản ghi ở offset `0x86`. Điều đó **sai**:
> client `BlockRead` thẳng vào `self+0x18` ngay từ **byte 0** của file.

---

## 2. Layout bản ghi (134 byte = 67 word)

| Offset | Kích thước | Ý nghĩa | Mã hóa? |
|---|---|---|---|
| `+0x00` | `u16` | **Map ID** (mã bản đồ) | ✅ `(w ^ 0xEA6C) - 9` |
| `+0x02` | `u8` | **Kind** (biến thể / instance trong map) | ❌ rõ |
| `+0x03` | `u8` | Độ dài tên (ShortString) | ❌ rõ |
| `+0x04` | ≤ 20 byte | **Tên map** — VISCII 1.1 | ❌ rõ |
| `+0x18` … `+0x85` | 110 byte (55 word) | vùng dự trữ | ✅ `(w ^ 0xEA6C) - 9` *(thực tế luôn = 0)* |

* Bản ghi **0** là **sentinel**: client ép `*(u16*)(self+0x18) = 0` sau khi giải mã
  ⇒ map id = 0 = *"không tìm thấy"*.
* Bản ghi 1 … 7 480 là **bản đồ thật**.
* Tên map tối đa 20 byte; byte `+0x17` = `0x00` (kết thúc).

### 2.1. Thuật toán giải mã (`FUN_0053542C`)

```asm
; for c = 1 .. count:
;     AX = (u16)self[0x18 + c*134]        ; map id
;     AX = (AX ^ 0xEA6C) - 9
;     self[0x18 + c*134] = AX
;     for k = 1 .. 0x37:                  ; 55 word
;         AX = (u16)self[c*134 + 0x2e + k*2]   ; = ban ghi[c] + 0x18 .. +0x84
;         AX = (AX ^ 0xEA6C) - 9
;         self[c*134 + 0x2e + k*2] = AX
```

Nói gọn: **`giá trị = (giá trị_luu ^ 0xEA6C) - 9`** cho mọi word được mã hóa.

### 2.2. Ví dụ kiểm chứng

| Offset file | Byte thô | Giải mã | Map ID |
|---:|---|---|---:|
| `0x86` (bản ghi 1) | `00 C0` | `(0xC000 ^ 0xEA6C) - 9` = `0x2A63` | **10851** — *QuảngTrườngTrácQuận* |
| `0x10C` (bản ghi 2) | `01 C0` | `(0xC001 ^ 0xEA6C) - 9` | **10852** |

---

## 3. Tra cứu theo `(mapId, kind)` — `FUN_00535864`

```c
undefined2 FUN_00535864(int self, ushort mapId, char kind)
{
  lo = 0;  hi = *(u16*)(self + 4);        // count
  // binary search tren self+0x18 + idx*134
  //   so sanh (u16)ban_ghi[idx+0x00] == mapId
  //   va     (u8 )ban_ghi[idx+0x02] == kind
  // tra ve idx (0 = khong tim thay)
}
```

⇒ **Khóa chính của bản đồ là cặp `(mapId, kind)`**, không chỉ `mapId`.
Một `mapId` có thể có tới **44** `kind` khác nhau (mỗi kind = một "phòng"/"lần" của map).

Sau khi tìm được chỉ số, `FUN_0053558C` copy **134 byte** của bản ghi đó ra buffer:

```c
_FillChar(out, 0x86, 0);
idx = FUN_00535864(self, mapId, kind);
if (idx != 0) {
    if (idx > 8000) BoundErr;
    src = self + 0x18 + idx * 134;
    memcpy(out, src, 134);          // 33 dword + 1 word
}
```

---

## 4. Bảng sinh ra từ bản giải mã

| File | Nội dung |
|---|---|
| `data_map_catalog.tsv` | 7 481 bản ghi: `idx, map_id, kind, name, w0..w2` |
| `data_mapid_to_name.tsv` | 3 503 `mapId` → danh sách tên (gom nhiều kind) |
| `data_event_maps.tsv` | `scene_id` của `eve.emg` → tên map (2 455 / 2 777 scene khớp) |

### 4.1. Thống kê

* 3 503 `mapId` khác nhau; 7 480 bản ghi thật.
* 3 977 bản ghi trùng `mapId` (cùng `mapId` nhưng **khác `kind`**).
* Chỉ **26** cặp `(mapId, kind)` trùng nhau.
* Mảng được **sắp xếp tăng dần theo `mapId`** ⇒ binary search hợp lệ (3 ngoại lệ nhỏ).
* Phân bố `kind`: `1`→3 467 bản ghi, `2`→1 898, `3`→735, `4`→360, … tối đa `44`.

---

## 5. Những map sự kiện đã nhận diện được

| `mapId` | `kind` | Tên map (theo `Scene.Dat`) |
|---:|---|---|
| **12000** | 43 / 44 | **Ghi danh Đoan Ngọ** / **QuảngTrường ĐoànViên** |
| 12000 | 5 / 13 | **Giới Kiều** (Nhị Kiều Giang Đông) |
| 12000 | 14 / 24 | Lô Long Khẩu / Rừng Triều Ca |
| **12003** | 8 | **Lôi đài thôn thực** |
| **12003** | 9 | **Lôi đài 5 vs 5** |
| **12003** | 10 / 11 / 12 | **LôiĐàiHỏiĐápNămMới** |
| 12003 | 13 | Tầng 1 Tâm Ma tháp |
| **12061** | 8 | **Lôi đài tỉ võ** |
| **12061** | 10 | **Đồng Tước đài** |
| 12061 | 11 | Tháp thí luyện |
| **12711** | 4 | **Tiền tiêu Hổ Lao** (chiến trường thủy chiến) |
| 12711 | 2 / 3 | Tiền tiêu Cự Lộc / Tiền tiêu ĐạiHưngSơn |
| 12001 | 16 / 17 | **Chiếntrường dũng giả** / **Nguyên Tiêu ký vực** |
| 12001 | 14 / 18 | NguyênTiêu nhạc viên / **Tháp Lôi Phong** |
| 12005 | 2 | **Hội thi nướng thịt** |
| 10991 | 1 / 2 | QuảngTrườngTrácQuận / **Mê cung sinh tồn** |
| 12406 / 12407 / 12418 / 12419 | 2 | **TừVôSơn TrắcPhong** (tháp lôi phong) |
| 49051 | 1 / 2 | **Trấn Ngưu Chữ** / Ao Ngưu Chữ |
| 59000 | 16 | **Phật Giới** (Phật Đản) |
| 60411 | 1 / 2 | Thôn U Linh / Bình Nguyên Tử Linh |

> 🔴 **Bài học cho server:** `mapId` **không đủ** — server phải gửi đúng **cả `kind`**
> (qua trường `*(u16*)(LocalPlayer + 0x63A)` cho map id và `kind` do server chỉ định),
> nếu không client sẽ hiện **sai tên bản đồ**.

---

## 6. `kind` đến từ đâu?

Trong `FUN_007994A4` (handler đổi map của client):

```c
kind = *(uint *)(param_1 + 0x14);          // tham so cua handler
if (kind > 0xFF) _BoundErr(kind);
FUN_0053558C(*(int *)gvar_007DA3F0,          // manager TKC_Scene
             *(ushort *)(*(int *)gvar_007DA7BC + 0x63A),  // map id hien tai
             (char)kind,
             out);
```

⇒ `kind` là **tham số do server đưa xuống** (qua handler đổi map), không phải client tự tính.

Trong dispatcher (`0x0078D980`):

```asm
0078d981: MOV byte ptr [0x009f1650],0x0
0078d996: CMP byte ptr [0x009f1650],0x0
0078d99d: JZ  0x0078d9e7                ; -> CALL 0x007994A4  (mac dinh)
0078d9d8: MOV dword ptr [EAX+0x458],0x7994a4   ; -> gan callback thay the
0078d9ea: CALL 0x007994a4
```

⇒ Có **hai đường** gọi handler đổi map: gọi trực tiếp, hoặc đăng ký làm callback
tại `[*gvar_007DA664 + 0x458]` (kèm tham số tại `+0x45C`).
