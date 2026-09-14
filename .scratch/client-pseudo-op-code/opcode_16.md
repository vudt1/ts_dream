# PHÂN TÍCH — Main OP 0x16 (Case 19) — `FUN_0078feaf` @ `0x0078FEAF` — **Đồng bộ World-Object & tác chiến (di chuyển / timer / trạng thái / lệnh hành động) (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x16] = 0x13 (=19)` → `jumptable_dword200_0x78A9B6[19]` (entry `0x0078AA02`, giá trị `AF FE 78 00`) → target `0x0078FEAF` = **Case 19**.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

> Phạm vi: **core logic opcode**. Bỏ qua chi tiết thuần **graphics/animation** (chỉ nhắc khi cần định vị). Một số field là *ô chiếm dụng lưới* / *hướng sprite* — mô tả đúng bản chất dữ liệu, không đi sâu vẽ.

---

## 1. Tóm tắt nghiệp vụ

**Main OP 0x16 là kênh S→C cập nhật/trạng thái các ĐỐI TƯỢNG THẾ GIỚI (world object / NPC chiến đấu) và registry tác chiến.** Đối tượng được đánh địa chỉ bằng **chỉ số slot 0..100** trong mảng `gvar_007DA6DC` (đã biết từ OP 0x14). 10 SubOp:

| Nhóm | SubOp | Bản chất |
| :--- | :---: | :--- |
| Di chuyển | `0x02` | **Ra lệnh đi TỚI** (X,Y) cho object `idx` (đặt *đích đến*, không đổi vị trí ngay). |
| Di chuyển | `0x05` | **Dặt (relocate) VỊ TRÍ THẬT** (X,Y) cho object `idx` + cập nhật ô lưới. |
| Di chuyển | `0x04` | **Batch**: 1 frame = N record 13 byte, mỗi record di chuyển + gắn trạng thái + timer (qua `TFightManage`). |
| Trạng thái | `0x03` | **Gắn mốc thời gian / trạng thái hẹn giờ** cho object `idx` (`v×1000`). |
| Trạng thái | `0x06` | **Bật/ghi byte trạng thái** `obj+0x34c` cho object `idx`. |
| Hành động | `0x09` | **Lệnh hành động/kỹ năng có kiểm tra + quay hướng**: A dùng action B nhắm/hướng tới C. |
| Registry/đội | `0x01, 0x07, 0x08, 0x10` | **ĐÃ BÓC TỪ BODY MỚI (2026-09-14)** — xem §4.7: `0x01` ghi byte trạng thái `obj+0xe1` (KHÔNG phải party roster như suy luận cũ), `0x07` đặt asset-index `obj+0x3d8 = payload[4]+100`, `0x08` chạy method VMT+0x18 + restore `+0xe3/+0x384`, `0x0A` dặt neo `+0x4c/+0x50` + ô lưới |

- `SubOp 0x00` và `≥0x0B`: **không có `default` → âm thầm bỏ qua** (`case_019.c:36–190` chỉ có `case 1..10`).
- Chuỗi trong các nhánh đã khôi phục: **chỉ có literal asset** — `sound\WA0014.wav` (SubOp 1, `007a2be8.c:68`) và tên sprite `"Boat06R…"` (đuôi SubOp 3, `0052b694.c:208`); **không có chuỗi tiếng Việt in-game** (xem §7).

**Sửa lại ghi chú cũ ở `opcode_14.md`:** tài liệu 0x14 tóm tắt `FUN_007127BC(obj,X,Y)` là "dặt tọa độ" — **chưa chính xác**. Đọc trực tiếp `007127bc.c` (mục 4.1) cho thấy nó **chỉ đặt ĐÍCH ĐẾN (walk-to)**, không đụng trường vị trí hiện hành. Trường vị trí thật do `FUN_0071BF18` (SubOp 5) đảm nhiệm.

---

## 2. Entry & cách đọc PayloadBuffer

Framing/dispatcher đã xác minh ở `handoff-opcode-exploration-guide.md` mục 1: `Frame = [F4 44][Len:Word LE][Payload]`, XOR `0xAD`; `Payload = [MainOp][SubOp][...]`.

`FUN_0078feaf` (`case_019_0078FEAF_FUN_0078feaf.c:29–36`):
```c
iVar5 = *(int *)(unaff_EBP + -0xc);                     // ECX = RestPayload (bỏ MainOp)
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar5);  // SubOp = ECX[0] = payload[1]
switch (*(undefined4 *)(unaff_EBP + -0x14)) { case 1..10 }
```

### Quy đổi chỉ số (DÙNG CHO MỤC 3–4)

| Cách ghi trong mã | Ý nghĩa trên wire |
| :--- | :--- |
| `ECX[i]` (0-based) | `payload[1+i]` |
| `_LStrCopy(ECX, p, n, &out)` (Delphi 1-based) | `n` byte từ `payload[p]` |
| `FUN_0077EB9C(out)` | **Word LE** = `b0 + b1*0x100` (CHỈ đọc 2 byte đầu của `out`) |
| `FUN_0077ED68` / `FUN_0077EF7C(out)` | **DWORD LE** = `b0 + b1*0x100 + b2*0x10000 + b3*0x1000000` |

⚠ **Bẫy SubOp 3** (xác minh): `_LStrCopy(ECX,4,4)` cắt **4 byte** nhưng chỉ số dùng để decode là `FUN_0077EB9C` (**Word, 2 byte**) rồi `& 0xffff` (`case_019.c:67–69`) ⇒ giá trị thật = `payload[4..5]`; **`payload[6..7]` bị copy ra rồi VỨT** (4 byte cắt ra chỉ để… đọc 2). Codec 4-byte THẬT (`FUN_0077ED68`) chỉ xuất hiện ở SubOp 4 (`0064246c.c:142–143`).

---

## 3. Bảng tổng hợp SubOp & wire layout (S→C)

Quy ước object: `idx = payload[2..3]` (Word LE, **bound `≤100`**), con trỏ = `gvar_007DA6DC[idx]` (bỏ qua nếu `==0`).

| SubOp | Độ dài payload | Layout (`payload[...]`) | Hành vi | Handler |
| :---: | :---: | :--- | :--- | :--- |
| `0x01` | 5 | `[16][01][idx:W=payload[2..3]][state:1B=payload[4]]` | Ghi `obj+0xe1=state` (+WA0014.wav nếu `obj+0x2a==0x0B && state==1`) — **ĐÃ BÓC** `007a2be8.c:44-70`; **đính chính: không phải party roster** | `func_0x007a2be8(gvar_007D9D64, RP)` |
| `0x02` | 8 | `[16][02][idx:W][X:W][Y:W]` (`X=payload[4..5]`, `Y=[6..7]`) | **Walk-to** | `FUN_007127BC(obj,X,Y)` |
| `0x03` | ≥6 (8) | `[16][03][idx:W][v:W=payload[4..5]][payload[6..7]: bỏ]` | Gắn **timer/trạng thái** `v*1000` | `FUN_00712940` (+`0073c524` TLight-mark +`0052b694` TWreck — xem 4.3) |
| `0x04` | `2+13N` | `[16][04][ N× record 13B ]` — record = `[idx:W][r2:1B][pad:1B][X:W][Y:W][status:1B][timer:D]` (xem 4.4) | **Batch** move+state+timer | `FUN_0064246C(TFightManage=gvar_007D9CE4, RP)` |
| `0x05` | 8 | `[16][05][idx:W][X:W][Y:W]` | **Relocate** vị trí thật + grid | `FUN_0071BF18(obj,X,Y)` |
| `0x06` | ≥5 | `[16][06][idx:W][flag:1B = payload[4]]` | Ghi `obj+0x34c = flag` | inline (`case_019.c:156–177`) |
| `0x07` | 5 | `[16][07][idx:W][n:1B]` | `obj+0x3d8 = n+100` + resolve asset `"<prefix><n+100>"` (hiệu ứng/overlay) — **ĐÃ BÓC** `0073d29c.c:43-66`+`0072a7a8.c:60-68` | `func_0x0073d29c(gvar_007D9D34, RP)` |
| `0x08` | ≥5 | `[16][08][idx:W][b:1B]` | `VMT+0x18(obj,b)` đổi state/action, keep/restore `+0xE3/+0x384`, action-set validation `FUN_005f4194` rồi `FUN_0072b390(obj,b)` — **ĐÃ BÓC** `0073d374.c:50-124` | `func_0x0073d374(gvar_007D9D34, RP)` |
| `0x09` | 6 | `[16][09][A:W][B:1B=payload[4]][C:1B=payload[5]]` | **Lệnh action A→(nhắm)C, action-code B** | `FUN_00747E38(gvar_007D9D34, RP)` |
| `0x0A` | 8 | `[16][0A][idx:1B=payload[2]][pad][X:W=payload[4..5]][Y:W=payload[6..7]]` | Dặt **neo** `+0x4c/+0x50`+ô lưới, không đổi vị trí hiện hành — **ĐÃ BÓC** `007481d4.c:46-69`+`00711944.c` | `func_0x007481d4(gvar_007D9D34, RP)` |

---

## 4. Chi tiết từng SubOp + helper

### 4.1. `SubOp 0x02` — Ra lệnh đi tới (`FUN_007127BC`, `007127bc.c:22–79`)
```c
obj+0x100 = 2;            // state = "đang di chuyển theo lệnh"
obj+0x104 = 0;
obj+0x118 = X;            // ĐÍCH ĐẾN X
obj+0x11c = Y;            // ĐÍCH ĐẾN Y
// chỉ khi obj+0x550==1 && obj+0x34c ∉ {2,4}:  xóa bit 2 (=&0xFB) ở Ô LƯỚI CŨ (tính từ obj+0x1c/0x20)
```
⇒ **KHÔNG thay đổi vị trí hiện hành** (`+0x1c/+0x20` giữ nguyên). Bộ 4 field `+0x100/+0x104/+0x118/+0x11c` là "lệnh đi/follow" — trùng khuôn gán đích trong `FUN_0071D2CC` (follow). Renderer/tick sẽ tự tiến `obj` về đích.

### 4.2. `SubOp 0x05` — Dặt vị trí thật (`FUN_0071BF18`, `0071bf18.c:22–101`)
```c
// (gate + xóa ô cũ giống 0x127BC) rồi:
xóa bit 2 ở ô lưới (X,Y) CŨ;   // :50–71
set  bit 2 (|4) ở ô lưới (X,Y) MỚI;  // :72–93   <- occupancy grid cập nhật
obj+0x1c = X;  obj+0x20 = Y;   // vị trí hiện hành  (UNCONDITIONAL, :97–98)
obj+0x4c = X;  obj+0x50 = Y;   // "neo" / server-truth                 (:99–100)
```
**Phân biệt cốt lõi vs SubOp 2:**

| | `FUN_007127BC` (SubOp 2) | `FUN_0071BF18` (SubOp 5) |
| :--- | :--- | :--- |
| Field ghi | `+0x118/+0x11c` = **đích**, `+0x100=2` | `+0x1c/+0x20` = **vị trí thật**, `+0x4c/+0x50` = neo |
| Ô lưới | chỉ **xóa** ô cũ | **xóa ô cũ + set ô mới** (dịch occupancy) |
| Ngữ nghĩa | "hãy đi đến…" | "nhảy tức thì đến…" (teleport/dặt) |

`FUN_00424E00` dùng ở cả hai = hàm **Round** (`00424e00.c:109–121`).

### 4.3. `SubOp 0x03` — Timer / trạng thái hẹn giờ (`FUN_00712940`, `00712940.c:24–77`)
```c
obj+0x34c = 4;                 // byte trạng thái = 4
obj+0x350 = Now();             // double mốc "bắt đầu"
obj+0x358 = v*1000;            // giá trị ms (v = Word payload[4..5])
// + block xóa bit ô lưới như trên
```
**Bản chất `+0x358`:** các hàm đọc nó (`FUN_007C4D30`: `Now(); Round(); return (rounded < param_1)`) đối xử như **mốc thời gian tuyệt đối (ms)** ⇒ `v` trên wire nên hiểu là "**giây đồng hồ máy chủ ×1000**" hơn là "duration". Hết hạn → `FUN_0072073C.c:62–67` cắt liên kết `obj+0x4c7/+0x4c8`.
- Nhánh đặc biệt: nếu `obj+0x4 == 0x947A` (class id 38010 — cùng họ `THuman`) → gọi **`func_0x0073c524(gvar_007D9D34, obj+0x1c [,obj+0x20])`** — **đã bóc từ body mới** (`0073c524_FUN_0073c524.c:31-61`): ghi vào registry `gvar_007D9D34`: `+0x5508=1`, `+0x5510=GetTickCount()`, `+0x5514/+0x5518 = X/Y (param_2/param_3)`, tạo `TLight` (VMT_772E1C) tại `+0x550c` nếu chưa có, rồi `FUN_00774220(light, 10016, 30, …, tọa độ minus camera-origin `gvar_007D9C28+0xc/+0x10`)` — bản chất: **bật hiệu ứng ánh sáng đánh dấu vị trí (X,Y)** (graphics, tóm 1 dòng). *Lưu ý:* call-site (`case_019.c:103`) chỉ model 2 tham số; `param_3` (Y) nhiều khả năng là `obj+0x20` qua ECX — chưa kết luận được chắc chắn từ bản C.
- Cuối nhánh: nếu **`LocalActor+0x145c == 2`** (byte "chế độ map hiện hành" — tra từ map id `LocalActor+0x63a`) → gọi `func_0x0052b694(gvar_007DA2FC, idx)` — **đã bóc từ body mới** (`0052b694_FUN_0052b694.c:63-216`): `slot = FUN_0052b944(self)` (cấp slot hiệu ứng, bound `≤0x28`=40); gate `obj+0x54 ∈ [0,0x321)` && `obj+0x58 ∈ [0,0x259)`; đọc class `obj+4` → `FUN_00623f00(gvar_007D9DE4)` → byte kind `gvar_007D9DE4+0x42+idx*0x17*4` (1..6) + bit 2 của `obj+0xE4` (hướng); tạo **`TWreck`** (`VMT_525EDC_TWreck`) tại `self+0xf18+slot*4`, sprite tra bảng tên `PTR_s_Boat06R_007d7578[(kind-1)*2+hướng]`, vẽ qua `FUN_007742e4` tại `obj+0x1c/0x20` trừ origin camera. ⇒ **`gvar_007DA2FC` không còn UNKNOWN: là manager hiệu ứng xác/boat-wake (≤40 slot)** — hiệu ứng đồ họa trên map nước, không phải dữ liệu wire mới.

### 4.4. `SubOp 0x04` — Batch chiến đấu (`FUN_0064246C`, `0064246c.c:73–185`)
Method của **`TFightManage` (gvar_007D9CE4)**. Số record `N = (len(RestPayload)-1)/13`. Đầu record thứ `i` (0-based) tại `payload[base]` với `base = i*13 + 2`. **Mỗi record đúng 13 byte (r0..r12), trong đó `r3` là byte đệm KHÔNG đọc** (mã `_LStrCopy` nhảy thẳng từ `base+2` sang `base+4` — xem `0064246c.c:96–143`):

| record byte (`r_k` = `payload[base+k]`) | kiểu | tác dụng |
| :---: | :---: | :--- |
| `r0..r1` | Word LE (`FUN_0077EB9C`) | `idx` object (`gvar_007DA6DC[idx]`, bound `≤100`; **toàn bộ record bị bỏ nếu slot `==0`**) |
| `r2` | byte (`ECX[base+1]`) | **chỉ khi** `obj+0x2a == 0x0B` → `obj+0xe1 = r2` (`0064246c.c:108,153–158`) |
| `r3` | byte | **pad — không được đọc** |
| `r4..r5` | Word LE | X |
| `r6..r7` | Word LE | Y |
| `r8` | byte | nếu `∈{1,2}` → `obj+0x34c = r8` (trạng thái) |
| `r9..r12` | **DWORD LE** (`FUN_0077ED68`, `0064246c.c:142–143`) | timer; `≠0` → `FUN_00712940(obj, timer)` |

Mọi record hợp lệ đều **vô điều kiện** gọi `FUN_0071BF18(obj, X, Y)` (`:164`) — **record 0x04 dùng relocate** (nhảy vị trí tức thì), không phải walk-to. ⇒ 1 gói `0x16/0x04` = **snapshot hàng loạt vị trí + trạng thái + timer cho nhiều đơn vị đang chiến**.

### 4.5. `SubOp 0x06` — Ghi byte trạng thái (inline, `case_019.c:156–177`)
```c
idx = Word(payload[2..3]);
flag = payload[4];                 // đọc qua char ECX[3], bound-check len(RP) >= 4
if (gvar_007DA6DC[idx] != 0)  obj+0x34c = (char)flag;
```
`+0x34c` = byte **trạng thái đơn vị** (giá trị quan sát: `1,2` từ `0x04`/`005e8534.c`, `4` từ `0x03`; các vòng xử lý `005ea5d4/005ea804/0073ce00` **bỏ qua** đơn vị khi `+0x34c ∈ {2,4}`).

### 4.6. `SubOp 0x09` — Lệnh hành động/kỹ năng (`FUN_00747E38`, `00747e38.c:68–195`)
```c
A = Word(payload[2..3]);   // object thi hành
B = payload[4];            // action-code (bound ≤0x4F)
C = payload[5];            // object thứ 2 (đích/hướng)
// 1) Validate B: FUN_005F4194(registry gvar_007D9ECC, obj[A]+0x7c + 1000, B) -> byte flag hành động hợp lệ?
// 2) Snap: nếu obj[A]+0x1c!=+0x4c || +0x20!=+0x50 -> +0x1c:=+0x4c; +0x20:=+0x50  (kéo về vị trí neo)
// 3) Thực thi: FUN_0072B390(obj[A], B) = vtable[+0x18](A,B) (chạy action/animation B)
//      nếu obj[A]+0x79==1 (local) -> GỬI C→S OP 0x20
// 4) Nếu obj[C].x!=0 && obj[C].y!=0: FUN_00731A9C(obj[A], obj[C].+0x1c - grid[0x0C], obj[C].+0x20 - grid[0x10])  // quay/đi hướng C
```
`FUN_00731A9C` **chính là hàm mà `TForm1.DXDraw1MouseDown` gọi khi người chơi click** ⇒ SubOp 9 = "server ra lệnh cho A dùng action B nhắm/hướng tới C", mô phỏng y hệt thao tác chuột.

### 4.7. SubOp 1, 7, 8, 10 — delegate (**ĐÃ BÓC TỪ BODY MỚI 2026-09-14 — đính chính lớn**)

Tất cả nhận `(self-global, RestPayload)`; wire thực sự được parse **bên trong callee**, không phải "opaque":

- **`0x01` → `func_0x007a2be8(gvar_007D9D64 = TCY_TeamManage, RP)`** (`007a2be8_FUN_007a2be8.c:44-70`):
  `idx = Word(Copy(RP,2,2))` (`:44-45`, guard `≤100`), `state = byte payload[4]` (`:46-52`); nếu `gvar_007DA6DC[idx]≠0` → **`obj+0xe1 = state`** (`:62`); đặc biệt: `obj+0x2a == 0x0B && state == 1` → phát `sound\WA0014.wav` (`:67-69`). **Đính chính:** suy luận cũ "cập nhật roster/thành viên party `[leaderID][N][members]`" là **SAI** — đây là **ghi byte trạng thái `+0xe1` theo slot + gắn âm thanh**, không đụng field party nào; `self` (`TCY_TeamManage`) không được đọc. Wire: `[16][01][idx:W][state:1B]` (≥5B).
- **`0x07` → `func_0x0073d29c(gvar_007D9D34, RP)`** (`0073d29c_FUN_0073d29c.c:43-66`): `idx = Word(payload[2..3])`; `v = byte payload[4] + 100`; `FUN_0072a7a8(obj, v)` → `obj+0x3d8 = v` (byte), `+0x3d9=0`, `+0x3da=1`, `+0x3df=Now()` (double), `+0x3db = FUN_007c9b38(gvar_007D9ED8, "<prefix>"+IntToStr(v))` (`0072a7a8_FUN_0072a7a8.c:60-68`) — **đặt chỉ số tài nguyên/overlay (hiệu ứng) cho object** (tra tên asset theo số). Wire `[16][07][idx:W][assetIdx:1B]` (≥5B).
- **`0x08` → `func_0x0073d374(gvar_007D9D34, RP)`** (`0073d374_FUN_0073d374.c:50-124`): `idx = Word(payload[2..3])`, `b = byte payload[4]`; lưu `obj+0xE3`, `obj+0x384` (900); gọi **method ảo `VMT+0x18(obj, b)`** (`:79-80`); khôi phục `+0xE3/+0x384` (`:96,107`); đọc `obj+0xE3` sau call, validate `FUN_005f4194(gvar_007D9ECC, obj+0x7c+1000, E3)` → nếu OK: `FUN_0072b390(obj, b)` (`:118-124`). ⇒ cùng khung "đổi state/action theo `b`" như SubOp 9 nhưng không có object thứ hai; **ý nghĩa `b` phụ thuộc action-set, chưa kết luận được enum**. Wire ≥5B.
- **`0x0A` → `func_0x007481d4(gvar_007D9D34, RP)`** (`007481d4_FUN_007481d4.c:46-69`): **`idx = byte payload[2]`** (1 byte — khác mọi SubOp khác dùng Word!), guard `≤100`; `X = Word(payload[4..5])` (`Copy(RP,4,2)`, byte `payload[3]` bị bỏ qua), `Y = Word(payload[6..7])`; `FUN_00711944(obj, X, Y)` → `obj+0x4c=X, +0x50=Y` (neo), `+0xe5=0`, `+0xf0=X/20, +0xf4=Y/20` (ô lưới), `FUN_00711aa4(obj)` (`00711944_FUN_00711944.c:20-26`). ⇒ **dặt neo server-truth + lưới cho object mà không đổi vị trí hiện hành** (bản "chỉ neo" của SubOp 5). Wire: `[16][0A][idx:1B][pad][X:W][Y:W]` (≥8B).

---

## 5. Bảng giải nghĩa global

| Symbol | Vai trò | Bằng chứng |
| :--- | :--- | :--- |
| `gvar_007DA6DC` | **Mảng con trỏ toàn cục chỉ số 0..100**; slot `1..100` = **world object / NPC**; element layout: `+4`=class id, `+0x1c/+0x20`=X/Y hiện hành, `+0x4c/+0x50`=X/Y neo, `+0x54/+0x58`=spawn, `+0x24`=zone, `+0x2a`=kind, `+0x100/+0x104/+0x118/+0x11c`=lệnh đi, `+0x34c`=state, `+0x350`=Now double, `+0x358`=deadline ms, `+0xe1/+0x582` | `005e8534.c:189–190` (`TMapNpc_Create`), bound `100` `case_019.c:51…`, field `007127bc/0071bf18/00712940/0064246c/00747e38` |
| `gvar_007D9D34` | **Registry entity/target**: `+0x60` số actor biết, `+0x6c/6d/71/75` trạng thái, `+0x76/7b/7f` target; method ID→index (`FUN_0070C20C`) | `0070c20c.c:1`, `0070c284.c:27–57`, `005e9d94.c:114–141`, callers `case_019.c:103,180,183,186,189` |
| `gvar_007D9CE4` | **`TFightManage`** (combat manager, VMT_63CCDC); method `FUN_0064246C` | `0050a4a0_TForm1.FormCreate.c:615–617` |
| `gvar_007D9D64` | **`TCY_TeamManage`** (party manager, VMT_7A1838); methods `007A2604/007A273C` | `0051189c.c:1387–1388`; caller `case_019.c:38`. **Lưu ý:** SubOp 1 (`func_0x007a2be8`) nhận self này nhưng **không đọc** — chỉ ghi `obj+0xe1` vào `gvar_007DA6DC[idx]` |
| `gvar_007DA7BC` | **LocalActor** | khắp nơi; gate SubOp 3 tại `case_019.c:125` |
| `LocalActor+0x145c` | byte "chế độ/bản đồ hiện hành" tra từ map id `+0x63a`; `==2` kích hoạt `func_0x0052b694` | `case_004_0078BC95.c:107–115`, `0077f414.asm.txt:281–283` |
| `gvar_007D9C28` | **Map grid**: origin cam `+0xc/+0x10`, origin lưới `+0x14/+0x18`, byte-grid `+400`, stride `0xFB` (251); **bit 2 (=4)=ô bị chiếm** | `007127bc.c:53–74`, `0071bf18.c:50–93` |
| `gvar_007D9ECC` | Registry asset/action-set (TList `+0x49c`, item byte-flag `+0x10..`), key=`obj+0x7c+1000` | `005f4194.c:55–62`, `00517054.c:159–167` |
| `gvar_007D9D30` | **Codec/self** cho `FUN_0077EB9C/ED68/EF7C` (không phải dữ liệu wire) | `0064246c.c:97,143`, `00747e38.c:69` |
| `gvar_007DA2FC` | **Manager hiệu ứng nước/xác wreck** — object có ≤40 slot `TWreck` tại `+0xf18`, helper cấp slot `FUN_0052b944` | **ĐÃ XÁC MINH (2026-09-14)** từ `0052b694_FUN_0052b694.c:63-216` (trước: UNKNOWN) |
| class id `0x947A` (38010) | loại object đặc thù (họ `THuman`) → bật `+0x582` khi chọn/enter | `case_019.c:79`, `005e9d94.c:140–145` |

---

## 6. Chiều C→S

- Bản C `0077f414_FUN_0077F414.c:927–928`: `case 0x16: break;` → **artifact** (file C chỉ khôi phục 13 lệnh `AddSedQueue`, trong khi `.asm.txt` có ~103 `CALL 0x0051633c` ⇒ phần lớn block gửi bị gộp/mất).
- **Có tồn tại đường gửi OP 0x16**, nhưng thấy được ở **call-site**, không ở switch C: `005fc840_FUN_005fc840.asm.txt:15–19` → `MOV CL,0x1; MOV DL,0x16; CALL 0x0077f414`, bọc sau `MOV DL,0x69; CALL 0x004c9d3c` (gate `class(LocalActor+4)==0x69`). ⇒ **C→S `(0x16, SubSel=1)`, payload tối thiểu `16 01`, không kèm dữ liệu.**
- Khuôn dựng payload C→S xác minh ở asm (`0077f414.asm.txt:318–345`): `Chr([EBP-0x6])` = **SubSel chính là byte SubOp trên wire**; các "nặng" mới nối field.
- **Template đầy đủ cho 0x16 = chưa khôi phục được** vì `redump/` **không có** dump bảng gửi `0x77F474`/`0x77F53C` (chỉ có 2 bảng S→C + subtable op 0x19). ⇒ Cần redump để chốt.

---

## 7. Chuỗi literal & encoding

- Quét `UNK_/DAT_` trên `case_019.c` + các helper đã khôi phục: **không có chuỗi hiển thị tiếng Việt**; mọi field là Word/DWORD/byte nguyên → **không có gì để dịch sang UTF-8**.
- Literal ASCII duy nhất tìm thấy trong 6 callee (trước đây "chưa có body", **nay đã có**): `sound\WA0014.wav` trong `007a2be8.c:68`; tên sprite `"Boat06R"` (và chuỗi prefix asset trong `0072a7a8.c:63` + `007c9b38` tra `.bmp`) — **asset/game-internal name, không phải text in-game**, không cần dịch.
- 4 helper `0x0073d29c/0x0073d374/0x007481d4/0x007a2be8` và `0x0073c524/0x0052b694` **đã bóc xong**, không còn suy đoán chuỗi.
- Tham chiếu encoding (để redump đối chiếu sau này): dự án dùng bảng đơn-byte tiền tổ hợp; `windows-1258` đã chứng minh **sai** (xem `opcode_13.md` mục 7).

---

## 8. Ghi chú cho Mock Server

1. **Chọn object bằng slot, không bằng server-ID.** Mock phải biết bảng `gvar_007DA6DC[0..100]` phía client đã nạp (thường do OP nạp map/OP 0x14 dựng). Gửi sai `idx` → client bỏ qua (`obj==0`).
2. **Di chuyển**:
   - Muốn NPC **tự bước** tới (X,Y): `[16][02][idx:W][X:W][Y:W]` (walk-to, có tick).
   - Muốn **nhảy tức thì/teleport** + cập nhật chiếm dụng ô: `[16][05][idx:W][X:W][Y:W]`.
   - **Hàng loạt** (nhiều đơn vị cùng lúc, kèm state/timer): dùng `[16][04][N×13B]` — layout record xem §4.4.
3. **Trạng thái có thời hạn**: `[16][03][idx:W][v:W]` → client set `state=4`, `deadline=v*1000` (đọc là **mốc giờ tuyệt đối ms**, không phải duration). Chỉ gửi 2 byte `v` (payload[4..5]); payload[6..7] client vứt.
4. **Bật/tắt cờ**: `[16][06][idx:W][flag:1B]` → `obj+0x34c=flag` (`2`/`4` = trạng thái bị các vòng xử lý bỏ qua).
5. **Hành động/kỹ năng**: `[16][09][A:W][B:1B][C:1B]`. `B` **phải hợp lệ** với action-set của `A` (tra `gvar_007D9ECC`), ngược lại `FUN_005F4194` fail → không thi hành. `C` là object thứ hai (đích/hướng); nếu `obj[C]` ở (0,0) thì chỉ thi hành action không quay.
6. **SubOp 1/7/8/10 — ĐÃ MOCK ĐƯỢC (2026-09-14, body mới)**:
   - `0x01`: `[16][01][idx:W][state:1B]` → ghi `obj+0xe1`. **KHÔNG còn là party roster** (đính chính).
   - `0x07`: `[16][07][idx:W][n:1B]` → `obj+0x3d8 = n+100` + resolve asset hiệu ứng.
   - `0x08`: `[16][08][idx:W][b:1B]` → `VMT+0x18(obj,b)` (đổi state/action) + validate action-set.
   - `0x0A`: `[16][0A][idx:1B][pad][X:W][Y:W]` → ghi neo `+0x4c/+0x50` + ô lưới.
   Vẫn còn thiếu: không có gì của 4 SubOp này là "cần redump" nữa; enum `b`/`state` đầy đủ vẫn chưa kết luận được.
7. **Cảm ơn client tự gửi C→S `(16 01)`** khi người chơi bấm nút được gate class `0x69` — mock chỉ cần sẵn sàng nhận, không bắt buộc xử lý.

---

## 9. Source trail (file / địa chỉ đã xác minh)

| # | Nguồn | Dùng cho |
| :--- | :--- | :--- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x16]=0x13`) + `jumptable_dword200_0x78A9B6.hex` (entry19→`0x0078FEAF`) | xác minh dispatch |
| 2 | `case_functions/functions/case_019_0078FEAF_FUN_0078feaf.c:29–190` | **handler + switch SubOp 1..10** |
| 3 | `functions/007127bc_FUN_007127bc.c`, `0071bf18_FUN_0071bf18.c` | walk-to vs relocate (§4.1–4.2) |
| 4 | `functions/00712940_FUN_00712940.c` + `007c4d30_FUN_007c4d30.c`, `0072073c.c:62–67`, `005ea5d4.c`, `005e8534.c` | timer `+0x358`, state `+0x34c` (§4.3,4.5) |
| 5 | `functions/0064246c_FUN_0064246c.c:73–185` + `0077ed68_FUN_0077ed68.c` | batch record 13B (§4.4) |
| 6 | `functions/00747e38_FUN_00747e38.c:68–195` + `005f4194.c:55–62`, `0072b390.c`, `00731a9c.c` | lệnh action (§4.6) |
| 7 | `functions/0070c20c.c`, `0070c284.c`, `005e9d94.c:114–165`, `007a2604.c`, `007a273c.c`, `007119ac_TMapNpc.Create.c` | gvar `gvar_007DA6DC/9D34/9D64` + format roster |
| 8 | `functions/0050a4a0_TForm1.FormCreate.c:615–617`, `0051189c.c:1387–1388` | tên VMT `TFightManage`/`TCY_TeamManage` |
| 9 | `functions/0077f414_FUN_0077F414.c:927` + `.asm.txt:246–254,281–283,318–345`; `005fc840_FUN_005fc840.asm.txt:15–19` | chiều C→S (mục 6) |
| 10 | `functions/0077eb9c_FUN_0077eb9c.c` (Word LE) | bẫy SubOp 3 |
| 11 | **Bodies mới (2026-09-14)**: `functions/007a2be8_FUN_007a2be8.c` (SubOp 1), `0073d29c` (7), `0073d374` (8), `007481d4` (0A), `0073c524`+`0052b694` (đuôi SubOp 3), `0072a7a8` (asset `+0x3d8`), `00711944` (neo `+0x4c/50`) | §4.7 + §4.3 — đặc tả 4 delegate + 2 helper đuôi SubOp 3; `gvar_007DA2FC` = manager TWreck |

**Còn là UNKNOWN / cần redump (cập nhật 2026-09-14):** bảng gửi C→S `0x77F474/0x77F53C` (vẫn chưa dump); enum đầy đủ của `obj+0x34c`, `obj+0x2a`, `obj+0xe1`, class `0x947A`, tham số `b` SubOp 8; param thứ 3 của `0x0073c524` (Y?) không rõ từ call-site C. **Đã gỡ khỏi UNKNOWN:** `func_0x007a2be8/0073d29c/0073d374/007481d4` (SubOp 1/7/8/10 — đã bóc §4.7), `func_0x0073c524`/`func_0x0052b694` (đuôi SubOp 3 — §4.3), `gvar_007DA2FC` (manager TWreck).
