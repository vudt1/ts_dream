# Guardrail ObjectID `100..400` — Dải ID Dành Riêng Cho GM/Admin "Thiên Thần" (aLogin.exe)

Ngày nghiên cứu: 2026-10-03
Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn chính: `client_pseudo_c/` (C + ASM + HEX) · Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`
Bộ giải mã: [`.scratch/client-pseudo-op-code/tools_decode_literals.py`](./tools_decode_literals.py) + bảng VISCII 1.1 trong `src/encoding.rs`
Tài liệu liên quan: [`opcode_02.md`](./opcode_02.md) (chat OP `0x02`), [`opcode_03.md`](./opcode_03.md) (spawn OP `0x03`),
[`opcode_04.md`](./opcode_04.md), [`opcode_10.md`](./opcode_10.md) (kênh GM), [`opcode_c7.md`](./opcode_c7.md),
[`opcode-exploration-guide.md`](./opcode-exploration-guide.md)

> **Phương pháp:** 5 subagent độc lặp lại trên 2 corpus, kèm 2 vòng đối chứng byte-level do tác giả tự đọc file
> (`004c9bf0`, `007ab870`, `0073bd24`, `005964c8`, `005967b0`, `0058b5a4`, `00590ba8`, `0077f414.asm.txt`,
> `cs_dispatch_byte200_0x77f474.hex`, `cs_builder_dword62_0x77f53c.hex`, `lit_75af20.hex`, `lit_7ABDAC.hex`).
> Có **3 mâu thuẫn giữa các subagent** về ý nghĩa nghiệp vụ; tất cả đã được giải quyết bằng byte/ASM verification
> và ghi lại ở §9.

> **Mức độ bằng chứng:** `✅ VERIFIED` = đã đối chiếu ASM hoặc byte thô. `⚠️ INFERRED` = suy luận từ 2 nguồn
> độc lập. `❓` = chưa đóng.

---

## 0. TÓM TẮT ĐIỀU HÀNH

> **Giả thuyết của người viết ("100–400 là dải ID dành cho tài khoản GM/Mod/Admin") là ĐÚNG.**
>
> Không chỉ "hợp lý" — **client tự đặt tên cho dải này là GM**, và tự gọi chúng là **"Thiên thần"**.

Bằng chứng quyết định, tác giả tự giải mã từ `lit_*.hex` + `src/encoding.rs`:

| Hằng số | VA | Byte thô | VISCII → NFC | Bind vào |
|---|---|---|---|---|
| `DAT_0075af20` | `0x75AF20` | `4B 68 F4 6E 67 20 74 68 AC 20 …` | **"Không thể giao dịch với Thiên Thần"** | toast chặn click lên actor dải này |
| `DAT_007abe14` | `0x7ABE14` | `28 47 4D 29 54 68 69 EA 6E 20 74 68 A5 6E` | **`(GM)Thiên thần`** | tag 3 **khi `100 ≤ id ≤ 400`** |
| `DAT_007abe54` | `0x7ABE54` | `28 47 4D 29` | **`(GM)`** | tag 4 (màu chat slot 4) |
| `DAT_007abe00` | `0x7ABE00` | `48 AE 20 74 68 AF 6E 67 3A` | **`Hệ thống:`** | tag 3 **khi `id == 100`** (sender hệ thống) |

Lệnh verify:

```bash
cd .scratch/client-pseudo-op-code
python3 tools_decode_literals.py 75af20 75af5f
python3 tools_decode_literals.py 7abd50 7abec3
```

Kết luận cấu trúc:

1. **Không phải NPC/quái.** Actor id `100..400` được instantiate bằng `VMT_70B5D0_TPlayers` — **cùng lớp** với
   mọi người chơi khác, nằm cùng mảng 800-slot `gvar_007DA300`. Quái map là `TMapNpc` với id **800–899** do
   **client tự gán**. ⇒ `✅ VERIFIED` (§3)
2. **Client ép kênh `(GM)`** cho mọi tin nhắn có sender id trong dải, và **thay tên người gửi bằng số ID**. (§4.2)
3. **Không tương tác được**: không tấn công / giao dịch / kết bạn / nhắn tin / chọn dạng. (§4.3)
4. **Avatar bị làm phẳng** về một model duy nhất, và `[100,150] ∪ [301,400]` bị **ẩn hoàn toàn**. (§4.1)
5. **Profile phía local**: bypass collision nhưng mất click-to-walk, mất gần hết UI shop, nhưng có **một lệnh
   C→S độc quyền** (`cmd 0x14` sub 1) mà không account nào khác phát được. (§4.4 – §4.6)

---

## 1. HẠ TẦNG: CHỈ ĐÚNG 2 HÀM CHỨA GUARDRAIL

Guardrail `(id < 100) || (400 < id)` **chỉ xuất hiện ở 2 file** trong toàn bộ corpus client (grep toàn bộ `*.c`):

| File | Dòng | Vai trò |
|---|---|---|
| [`client_pseudo_c/004c9bf0_FUN_004c9bf0.c`](../client_pseudo_c/004c9bf0_FUN_004c9bf0.c) | `86-87`, `120-127` | Bộ phân loại ObjectID → "class" byte |
| [`client_pseudo_c/007ab870_FUN_007ab870.c`](../client_pseudo_c/007ab870_FUN_007ab870.c) | `258`, `269`, `289`, `333` | Bộ render dòng chat / thông báo |

Không có file nào khác chứa ngưỡng 100/400 ⇒ **toàn bộ hành vi GM-band đều đi qua 2 hàm này + các call site
của chúng**.

### 1.1 Bảng phân loại ID — `FUN_004c9bf0` @ `0x004C9BF0`

Signature: `undefined4 __register FUN_004c9bf0(int param_1)` — nhận 1 ObjectID, trả về **byte thấp** là "class".

ASM authoritative (`test/ts_decompile/functions/004c9bf0_FUN_004c9bf0.asm.txt`), range đã tự re-derive:

| class | Dải ID (inclusive) | Line C | Ghi chú |
|---:|---|---|---|
| `2` | `[5,10] ∪ {23} ∪ {81}` | `:73-75` | |
| `3` | `[11,16] ∪ [91,93]` | `:78-80` | |
| `4` | `[17,22]` | `:83-85` | |
| **`5`** | **`[100,150]`** | `:86`, `:125-127` | ★ **GM tier 1** |
| **`7`** | **`[151,400]`** | `:87`, `:120-123` | ★ **GM tier 2** |
| `9` | `[1001,1057]` | `:88`, `:117-119` | namespace dự trữ kế tiếp |
| `10` (0xA) | `[26,32] ∪ [34,38] ∪ [74,80] ∪ [1058,1119] ∪ {64006,64007}` | `:89-95` | server-object, không phải actor |
| `11` (0xB) | `[24,25] ∪ {39,40}` | `:98-101` | như trên (chỉ 4 id — xem §1.3) |
| `12` (0xC) | `{33} ∪ {63} ∪ [72,73]` | `:104-107` | |
| `13` (0xD) | `[41,62] ∪ [64,65]` | `:110` | |
| `0` | **mọi thứ khác** | `:71` (default) | ⇒ **người chơi thường** |

**Idiom guardrail phổ biến nhất**: `(byte)(cls - 5) < 4` ⟺ `cls ∈ {5,6,7,8}` ⟺ (vì 6/8 không thể xảy ra)
`cls ∈ {5,7}` ⟺ **`100 ≤ id ≤ 400`**. Idiom này xuất hiện ở **18 call site**.

### 1.2 Hai call-site dạng khác

| Idiom | Ý nghĩa | Call site |
|---|---|---|
| `(byte)(cls - 5) < 2` | `cls ∈ {5,6}` ⇒ **`{5}` = [100,150]** | `FUN_0071e340:62`, `FUN_0071f5f4:43` |
| `cls == 8` | **dead code** (8 không bao giờ được trả về) | `FUN_0071e340:65`, `FUN_0071f5f4:46`, `004c9d3c:155` |

### 1.3 Bảng class là **legacy hand-edited** — 3 bằng chứng

1. **Code test `6` và `8`** ở nhiều nơi, nhưng hàm **không bao giờ trả về 6 hay 8** (`004c9d3c.c:88,135,155,225`;
   `0071e340:65`; `0071f5f4:46`).
2. `FUN_004c9d3c` có `case 1:` body, nhưng class `1` cũng không bao giờ xuất hiện.
3. Nhánh `0xB` bị **dead do thứ tự**: khối `0xA` test trước rồi `JMP` thẳng epilogue, nên `[27,32]` đã bị
   `0xA` claim trước, `[34,38]` cũng vậy ⇒ `0xB` chỉ còn `{24,25,39,40}`. Trông như **copy-paste bug** ở mã
   Pascal gốc (`[27,32]`/`[34,40]` có lẽ đáng ra `[33]`/`[39,40]`).

⇒ Bảng class được viết từ lâu rồi vá dần; **client không đồng bộ hoàn toàn** với bản gốc.

### 1.4 Cấu trúc namespace ObjectID

```text
  5 –  99 : các dải nhỏ, lấn át nhau        → class 2,3,4,10,11,12,13
100 – 400  : ★★ GM / Admin / System announcer ★★ → class 5, 7
401 – 1000 : (class 0) — cửa sổ cấp phát người chơi
1001– 1119 : namespace dự trữ tiếp             → class 9, 10
1120+      : (class 0) — không gian người chơi chính
─────────────────────────────────────────────
800 – 899  : quái map (TMapNpc)  ← CLIENT TỰ GÁN, không do server
904+       : rác / hộp (TGarNpc)
1104+      : boss nước ngoài (TForeignNpc)
+0x13      : pet (TPetNpc)
```

Dải GM **nằm ở đáy** không gian ID, ngay dưới vùng cấp phát người chơi — đúng layout reserved-band điển hình
của MMO Đông Á thập niên 2000.

---

## 2. HAI GLOBAL LÀM MỐC

| Global | Ý nghĩa | Layout đã xác định |
|---|---|---|
| `gvar_007DA7BC` | **con trỏ→con trỏ tới record local player** | `+4` = ObjectID của mình · `+9` = tên (PChar[14]) · `+0x08` = byte costume · `+0x7C` = word sprite-id · `+0x79` = class-tag (xem §3.3) · `+0x63A` = MapID · `+0x1437` = map-permission code · `+0x376` = state |
| `gvar_007DA300` | **mảng 800 pointer** mọi actor remote nhìn thấy (index 0 = "không tìm thấy") | `+4` = ObjectID, `+0x79` = class-tag = **2** vīa được gán bởi `TPlayers.Create` |
| `gvar_007DA6BC` | **mảng cache 2100 slot** hồ sơ ngoại hình, key = ObjectID | `+4` = id · `+8` = tên (PChar[14]) · `+0x1C/+0x3C/+0x3D/+0x3E` = mirror 3 byte flag + word `0x7C` |
| `gvar_007DA6DC` | **mảng 100 slot quái map** — mảng RIÊNG, không trộn | root id 800, quái 801–900 |

Reverse lookup ObjectID → index: `FUN_0070c20c(gvar_007D9D34, id)` / `func_0x0070c158(...)`.

---

## 3. BẰNG CHỨNG LOẠI TRỪ: ĐÂY KHÔNG PHẢI NPC / QUÁI

Đây là điểm 2 subagent bất đồng. Giải quyết bằng cách trace đường spawn từ packet.

### 3.1 Bảng VMT trong world-actor family

| VMT | Lớp | `+0x79` | Instantiated ở đâu | ID |
|---|---|---:|---|---|
| `VMT_70B1C4_THuman` | `THuman` | 0 | `00530290`, `00716eec`, `007119ac`, `007447f8`, `0074d4f4` | base |
| `VMT_70B2DC_TMapNpc` | **quái map** | 3 | `FUN_005e8534` (NpcOnMapWalk loader) | **800–900** |
| `VMT_70B354_TFollowNpc` | follower sprite | 4 | children của mọi `TPlayer`/`TPlayers` | `-1` / 900–904 |
| `VMT_70B3D0_TGarNpc` | rác / hộp | — | `FUN_007369e8` | 904+mapbase |
| `VMT_70B45C_TForeignNpc` | boss NN | `0x10` | `FUN_007450f4` | 1104+mapbase |
| `VMT_70B4D8_TPetNpc` | pet | `0x13` | `FUN_0074dbc4` | |
| **`VMT_70B5D0_TPlayers`** | **remote player** | **`2`** | **`case_004:423`, `case_005:43`, `case_008`, `case_012`, `0078a89c`×4, `FUN_00722578:43`** | **do server cấp** |
| `VMT_70B550_TBasePlayer` | parent chung | — | `FUN_0070e878:463` | |
| `VMT_70B650_TCartNpc` | cưỡi (cart) | — | `FUN_0071e080` | |
| `VMT_70B6CC_TWorldPlayer` | sprite cache | — | `FUN_0072174c` | |
| **`VMT_70B740_TPlayer`** | **LOCAL player (self)** | **`1`** | **`FUN_00603f20:189`**, `TForm1.FormCreate:529` | |
| `VMT_70B7E0_THorse` | ngựa | — | `FUN_0071dae0` | |

Chú thí: **không tồn tại** VMT nào tên `TMonster` / `TNpc` — quái là `TMapNpc`.

### 3.2 Đường spawn — VMT là hằng số, KHÔNG phụ thuộc ID ✅ VERIFIED

`case_004_0078BC95_FUN_0078bc95.c` (Main OP `0x03` — actor xuất hiện / nạp hồ sơ), nhánh "OTHER actor"
(khoảng dòng 412-443, đã lược bỏ `_BoundErr`):

```c
sVar6 = FUN_0077eb9c(stream, buf);                       // 2 byte -> ushort
if (sVar6 == *(short *)(*(int *)gvar_007DA7BC + 0x63a)) { // == MapID của tôi
  uVar7 = func_0x0070c158(*(undefined4 *)gvar_007D9D34, charID_from_payload);   // cấp slot
  if (uVar7 != 0) {
    if (*(int *)(gvar_007DA300 + uVar9 * 4) == 0) {
      piVar11 = TPlayers_Create((int *)VMT_70B5D0_TPlayers,'\x01', uVar9);   // <== HẰNG SỐ
      *(int **)(gvar_007DA300 + uVar9 * 4) = piVar11;
    }
    ... FUN_00722950(cache, charID, actor);              // nạp hồ sơ
```

**Không có** `if (class) vmt = TGarNpc else vmt = TPlayers` ở bất kỳ đâu. Xác nhận ở mức máy — cả 10 site đều
**đúng 8 lệnh**, không lệnh `ClassCreate`, không nạp VMT:

```asm
0078bf9f: CALL 0x004c9bf0
0078bfa4: ADD  AL,0xfb
0078bfa6: SUB  AL,0x4
0078bfa8: JNC  0x0078bfc2        ; không thuộc dải -> bỏ qua
0078bfb1: MOV  byte ptr [EAX+0x8],0x0
0078bfbc: MOV  word ptr [EAX+0x7c],0x78
```

Danh sách 10 site ép ngoại hình:

| Handler | Self | Remote |
|---|---|---|
| `case_004` (OP `0x03`) | `:149-153` | `:490-502` |
| `case_005` (OP `0x05`) | — | `:105-117` |
| `case_008` (OP `0x08`) | — | `:108-120` |
| `case_012` (OP `0x0C`) | — | `:129-141` |
| `0078a89c` (bản gộp) | `:1287-1291` | `:1627-1638`, `:1725-1730`, `:1971-1982`, `:2390-2395` |
| `jumptable_0x78A9B6_cases.c` | — | `:967-968`, `:1312-1317`, `:1465-1470`, `:1872-1877`, `:2516-2521` |

### 3.3 `actor+0x79` = **class-tag do constructor ghi** ✅ VERIFIED (sửa claim "không tìm thấy writer")

Các subagent đầu báo "`+0x79` không có writer nào trong corpus". **Sai** — writer chính là constructor:

| Writer | Line | Ghi | Lớp |
|---|---|---:|---|
| `THuman.Create` | `0070d064:127` | 0 | `THuman` |
| **`TPlayer.Create`** | `00716370:66` | **1** | **self** |
| **`TPlayers.Create`** | `007169b4:43` (asm `MOV byte ptr [EAX+0x79],0x2`) | **2** | **remote actor** |
| **`TMapNpc.Create`** | `007119ac:38` | **3** | **quái map** |
| `TFollowNpc.Create` | `00716eec:48` | 4 | follower |
| `FUN_00659358` | `:52` | 2 | tạo actor tag-2 |
| `FUN_00646a34` (factory chung) | `:86-107` | `param_9` | id `0x908E..0x9097` |
| `TForeignNpc.Create` | `007447f8:38` | `0x10` | |
| `TPetNpc.Create` | `0074d4d4:37` | `0x13` | |

**Nó không bao giờ được gán từ packet.** Hai vùng "undumped" từng bị nghi ngờ (`0x0062d690`, `0x00767e6c`) thực ra
chỉ là call site của `FUN_004c9bf0`, không liên quan.

Ngữ nghĩa từ read site rõ nhất — `FUN_0070f7b4:478-545` (chọn màu RGB555 tên/HP):

```c
if      (+0x79 == 1)                    local_20 = 0xffe0;              // TPlayer  -> vàng
else if (+0x79 == 6)                    local_20 = party ? 0x55bf : 0xfb2b; // xanh/cyan
else if (+0x79 == 2) {                                            // TPlayers
    local_20 = 0x7ff;                                             //   trắng
    if ((byte)(actor[0x4B0] - 1) < 2) local_20 = 0xf81f;          //   magenta (PK/party)
}
else if (+0x79 == 0x10)                 local_20 = self.0x376 ? 0xfb2b : 0xffe0;
```

Ngoài ra `FUN_0072fa98:142,231,338`, `FUN_0073f2f0:119`, `FUN_0063ea8c:73,218`, `FUN_00646028:67,141,174` đều test
`(byte)(+0x79 - 1) < 2` = "actor giống người chơi (self hoặc remote)" → vẽ thanh HP + load 4 overlay trang bị.

`FUN_00710ab0` (updater thuộc tính mỗi actor) chỉ biết phục vụ tag **1** (`TPlayer`, nhận skill) và tag **4**
(`TFollowNpc`, nhận effect). Tag **2** (`TPlayers`) **không nhận gói skill/buff** — chỉ có hồ sơ ngoại hình.

⇒ **Band 100–400 không tương quan với `TFollowNpc`**; band là tag **2**.

### 3.4 Xác nhận thêm: quái không thể cưỡi, `TPlayers` thì có

`FUN_0071e080:36-80` (gọi trên **mọi** actor mọi lúc spawn):

```c
*(undefined1 *)(param_1 + 0x4b0) = param_2;      // mã cưỡi từ server
if ((byte)((*(undefined1*)(param_1+0x4b0) - 1) < 2) ) {   // mã cưỡi 1 hoặc 2
  if (*(char *)(param_1 + 0x79) == '\x01')       // TPlayer (self)
      *(int **)(param_1 + 0x61c) = TCartNpc_Create(VMT_70B650_TCartNpc,1,mapNpcId,param_1);
  else { cVar2 = *(char*)(param_1+0x79) - 2;
 if (cVar2 == 0)                                // TPlayers (remote)
           *(int **)(param_1 + 0x61c) = TCartNpc_Create(VMT_70B650_TCartNpc,1,-1,param_1); }
}
```

⇒ `TPlayers` (band 100–400) **có thể cưỡi**; `TMapNpc` (quái) thì không.

### 3.5 Kết luận §3

> `✅ VERIFIED` — Actor id `100..400` là **player-class account**, cùng lớp `TPlayers`, cùng mảng
> `gvar_007DA300[1..799]`, đi qua **cùng đường packet** với mọi người chơi khác.
> Quái thì **không bao giờ** dùng mảng này và **luôn có id ≥ 800 do client tự gán**.
> Giả thuyết "100–400 = NPC/monster band" bị loại trừ.

---

## 4. SÁU LUỒNG XỬ LÝ PHÍA CLIENT

### 4.1 🔵 Luồng A — Login / Spawn: **ép ngoại hình về một avatar duy nhất**

Áp dụng cho **cả self lẫn mọi remote actor** trong dải (10 site ở §3.2).

```c
uVar7 = FUN_004c9bf0(<actor>.ObjectID);
if ((byte)((char)uVar7 - 5U) < 4) {     // class ∈ {5,7}
  actor+0x08 = 0;
  actor+0x7C = 0x78;                   // 120
}
```

**Ngữ nghĩa 2 field** (`✅ VERIFIED` từ mọi read site):

| Field | Kiểu | Nguồn | Ngữ nghĩa |
|---|---|---|---|
| `+0x08` | byte | byte thứ 4 packet OP `0x03` | byte enum **costume/body-set + style name-tag**: `0` = phẳng · `1`/`2` = 2 bảng sprite song song `gvar_007DA540[i*0xB9*2] + 0x32/0x3A/0x42/0x4A` + chọn prefix tên. Ép về `0` ⇒ **mất bộ trang bị, mất prefix tên trong chat** |
| `+0x7C` | word | byte `0x0F` packet, zero-extend | **sprite/graphic-set id**, dùng như `(id + 1000)` để index resource list `gvar_007D9ECC+0x49C` qua `FUN_005f4194` (`00747e38:98-104`, `0073d374:112-118`). Giá trị ma thuật: `0x7A`(122), `[0x1D8,0x1DD]`(472–477 → `FUN_005e8534:439` ép `= 0.015`), `9`. Ép về `120` ⇒ **pin resource 1120**, đồng thời loại khỏi dải 472–477 |

Hệ quả: **mọi avatar dải 100–400 bị làm phẳng về cùng một model, bất kể server gửi gì.**

**Ẩn hoàn toàn** — `FUN_0071e340` (render đầy đủ) và `FUN_0071f5f4` (render rút gọn):

```c
if (*(char *)((int)param_1 + 0x79) == '\x02') {              // chỉ TPlayers
  if ((300 < actor.id) && (actor.id < 0x191)) return;         // [301,400] -> ẨN
  uVar2 = FUN_004c9bf0(actor.id);
  if ((byte)((char)uVar2 - 5U) < 2) return;                  // [100,150] -> ẨN
  if ((char)uVar2 == '\b') return;                           // class 8: dead
}
LAB_0071e483:
FUN_0070e878(param_1,1,1,0,0xff);      // <== đường bình thường: vẽ thân + mọi layer
```

`return` xảy ra **trước `FUN_0070e878`** ⇒ ẩn **toàn bộ**: sprite thân, overlay trang bị, bóng, tên, thanh HP.
ASM xác nhận đúng biên (`0071e340.asm.txt:21-37`):

```asm
CMP byte ptr [EAX+0x79],0x2   ; JNZ -> normal
CMP dword ptr [EAX+0x4],0x12d ; JL  -> normal      ; id >= 301
CMP dword ptr [EAX+0x4],0x190 ; JLE -> HIDE        ; id <= 400
CALL 0x004c9bf0 ; ADD AL,0xfb ; SUB AL,0x2 ; JC HIDE
               ; SUB AL,0x1 ; JZ HIDE              ; class == 8 (dead)
```

⇒ Ba ngưỡng phụ bên trong dải: `[100,150]` ẩn · `[151,300]` vẽ với model 1120 · `[301,400]` ẩn.

### 4.2 🔵 Luồng B — Chat: **ép kênh `(GM)`, thay tên bằng số ID**

`FUN_007ab870(form, id, msg, tag)` — `tag` = kênh do server gửi. Đây là hàm hội tụ cho **mọi** dòng tin
nhắn của client.

**Bước 1 — resolve tên (trước mọi thứ):**

```c
if (*(int *)(*(int *)gvar_007DA7BC + 4) == id)  _LStrFromString(&name, *(int *)gvar_007DA7BC + 9);
else                                             FUN_0075ddb8(id, &name);
if (FUN_005631c4(id) != 0) return;    // id ∈ DAT_00949284 → BỎ TIN (ignore list)
```

**Bước 2 — ghép nhãn (có guardrail ở dòng 258/269/289):**

| id | tag 1 | tag 2 | tag 3 |
|---|---|---|---|
| `<100` / `>400` | `(Tất Cả)` + Tên + `:` + msg | `(Gần)` + Tên + `:` + msg | `(Thì Thầm)` + Tên + `:` + msg |
| **`== 100`** | như trên | như trên | **`Hệ thống:` + msg** (tên bị bỏ) |
| **`101–400`** | `(Thần)Thiên thần` + `IntToStr(id)` + `Tiếng cọ…` + `:` + msg | `(Gần)Thiên thần` + id + `Hạo nhỏ tiếng` + `:` + msg | **`(GM)Thiên thần` + `IntToStr(id)` + `Nói …` + `:` + msg** |

Các tag còn lại (0, 4, 5, 6, 7, 10, 11) **không nhánh theo dải** ⇒ hiển thị như người chơi thường.

**Bước 3 — ép màu (dòng 333, 362-365):**

```c
if ((id < 100) || (400 < id)) {
   ... xử lý #end (tag 0x0B) / tag 0 ...
   FUN_007ad614(*(int **)(form+0x130), line, param_4, id, '\x01');   // màu = tag từ server
}
else {
   FUN_007ad614(*(int **)(form+0x130), line, 4,        id, '\x01');   // màu ÉP = 4 = slot (GM)
}
```

**Ba hệ quả quan trọng:**

1. **Ép màu 4 (= slot `(GM)`)** cho mọi tin có id trong dải — kể cả khi nhãn là `(Đội)`/`(Đoàn)`/`(Minh)`.
   ⇒ **nhãn và màu mâu thuẫn nhau**: dòng `(Đội)…` vẫn vẽ bằng màu GM.
2. **Tên bị thay bằng số ID** — `_LStrCatN` nhận `IntToStr(id)`, **không nhận name**. Name đã resolve bị ghi đè.
   ⇒ dải này được client coi là **nguồn phát thanh**, không phải nhân vật có tên hiển thị.
3. **`#end` tích lũy bị chết** (dòng 333 gate): tag `0x0B` với id trong dải ⇒
   - không gom vào `form+0x16C`
   - **không cắt sentinel `#end`** ⇒ `#end` hiện nguyên trong dòng render
   - **không gọi `VMT+0x34`** ⇒ cửa sổ `form+0x174` không nhận memo
   - cờ `form+0x162` ("đang tích lũy") **có thể kẹt vĩnh viễn ở 1**

**Chiều đối xứng** — `FUN_005964c8` (`005964c8:25-31`, tác giả tự verify):

```c
uVar1 = FUN_004c9bf0(*(int *)(*(int *)gvar_007DA7BC + 4));
if (3 < (byte)((char)uVar1 - 5U)) {        // class của MÌNH ∉ {5,6,7,8}
  *(undefined1 *)(param_1 + 0x19c) = 1;
  *(uint       *)(param_1 + 0x1a0) = kernel32_GetTickCount();
}
```

Hàm này **chỉ được gọi từ nhánh `[100,400]` của `FUN_007ab870`** (dòng 265) ⇒ cờ `InputBar+0x19C` là
**cờ riêng cho người nhận GM**. Đây là **kênh GM↔GM**: người chơi thường không bao giờ thấy nó được set.
`FUN_005965e8` là bản anh em trên `+0x1A5/+0x1A8`, cùng logic.

**Bảng mapping MainOP → tag → render** (từ 20+ call site `FUN_007ab870`):

| MainOP | SubOp | tag | id | Render với id ∈ [100,400] |
|---|---|---:|---|---|
| `0x01` | `0x01` | `0xA8`(168) | từ packet | NAME trần (rơi vào jump-table default) |
| `0x02` | `0x00` | 0 | từ packet | `(Công bố hệ thống)` + msg |
| `0x02` | `0x01` | 1 | từ packet | `(Thần)Thiên thần` + id + `Tiếng cọ…` + msg |
| `0x02` | `0x02` | 2 | từ packet | `(Gần)Thiên thần` + id + `Hạo nhỏ tiếng` + msg |
| `0x02` | `0x03` | 3 | từ packet | `id==100`→`Hệ thống:` · `101..400`→**`(GM)Thiên thần`** |
| `0x02` | `0x04` | 4 | từ packet | `(GM)` + NAME + msg (tên thường rỗng) |
| `0x02` | `0x05` | 5 | từ packet | `(Đội)` + NAME + msg — chỉ hiện nếu resolve được tên |
| `0x02` | `0x06` | 6 | từ packet | `(Đoàn)` + NAME + msg |
| `0x02` | `0x07` | 7 | từ packet | `(Minh)` + NAME + msg |
| `0x02` | `0x0B` | 11 | từ packet | **mảnh thô giữ luôn `#end`** |
| `0x02` | `0x0C` | 0 | hằng `0` | (không áp dụng — id=0 thuộc bucket thường) |
| `0x10`, `0x14`, `0x17`, `0x23`, `0x27` | nhiều | 0..10 | hằng `0` hoặc own-id | không áp dụng |

### 4.3 🔵 Luồng C — Tương tác: **bị từ chối toàn bộ**

12 call site chặn mọi hành động lên actor dải `100..400`: `toast(...)` + `return`, **trước** mọi logic khác.

| Hàm | Điều kiện chọn | Lệnh bị chặn | Toast |
|---|---|---|---|
| `FUN_00642c2c:94` | click chính | `cmd 0x0B` tấn công/move | `DAT_00643244` (1000ms) |
| `FUN_007a2d20:55` | chuột phải | `cmd 0x0D` giao dịch | `DAT_007a31fc` (1500ms) |
| `FUN_007a3260:55` | special click | `cmd 0x0D` | `DAT_007a35fc` (1500ms) |
| `FUN_0064ea20:49` | `*gvar_007D9D5C == 0` | `cmd 0x09` | `DAT_0064ecd4` (1000ms) |
| `FUN_0064efa0:65` | `*gvar_007D9D5C == 1` | `cmd 0x0E` / `0x09` | `DAT_0064f460` (1000ms) |
| `FUN_0064f7bc:40` | `*gvar_007D9D5C < 2` | `cmd 0x15` | `DAT_0064f9b0` (1000ms) |
| `FUN_0075ad70:44` | chuột trái | `cmd 0x19` | `DAT_0075af20` ← **"Không thể giao dịch với Thiên Thần"** |
| `FUN_0075b66c:44` | chuột phải | `cmd 0x19` | `DAT_0075b81c` (1200ms) |
| `FUN_0075ebc0:42` | btn 5 | **`cmd 0x0E` kết bạn** | `DAT_0075ed10` (1500ms) |
| `FUN_00752728:33` | btn `0x12` | `cmd 0x27` | `DAT_007527b8` (1500ms) |
| `FUN_00562a34:62` | chọn từ list | — (xóa selection) | `DAT_00562c90` (2000ms) |
| `FUN_007967b0:80` | whisper target | — (xem dưới) | chuyển input mode 4 |

**Whisper** — `FUN_005967b0:78-96` (tác giả tự verify):

```c
uVar1 = FUN_004c9bf0(*(int *)(param_1 + 0x1ac));   // target UID
if ((byte)((char)uVar1 - 5U) < 4) {
  FUN_005952f4(local_8,4);                          // canned input mode, +0x1ac = 0
}
```

⇒ **không cho nhắn tin cho dải này**, đích bị xóa.
Thêm: nếu **chính mình** thuộc dải, dù target hợp lệ thì echo vẫn bị ép về dạng **số ID** thay vì `>>Tên`
(`005967b0:87` → `">>" + "(" + IntToStr(id) + ")"`).

⇒ **Không thể tấn công / giao dịch / kết bạn / nhắn tin / chọn dạng GM.** Đúng luật "GM là bất khả xâm phạm".

### 4.4 🔵 Luồng D — Di chuyển: **bypass collision, nhưng mất click-to-walk**

`FUN_0073bd24(actor, x, y)` = cổng "có được bước tới ô này không". Tác giả đã **verify ngược cực** qua cả 2 caller:

```c
// 00712d7c:178-236 (click-to-move)
uVar8 = FUN_0073bd24(local_8, local_8[...], iVar4);
if ((char)uVar8 == '\0') {          // FALSE = ĐƯỢC TỪ CHỐI
  toast(&DAT_007132f0, 0x5dc);      // "không đi được tới đó"
  local_8[0x40] = 0; local_8[0x41] = 0;   // xóa hàng đợi đường
  local_8[0x13] = local_8[7];  local_8[0x14] = local_8[8];  // reset đích về ô hiện tại
} else {                            // TRUE = đi
  FUN_0070dc78(...); FUN_0077f414(*(gvar_007D9D30), 6);   // phát move packet
}
```

`FUN_00731ffc:42-141` (bàn phím) **giống hệt**. ⇒ `TRUE` = đi.

Với class `{5,7}`, hàm `return TRUE` ngay ở đầu (asm `:10-19`), bỏ qua **toàn bộ**:

- map-permission `*(char*)(*(int*)gvar_007DA7BC + 0x1437)` (`FUN_00634f3c(mapList, MapID, 3)`)
- map-category `FUN_00635258(gvar_007D9C70, self+0x5F0)`
- **tới 5 lần** `FUN_00509cac(gvar_007DA37C, x, y, mode, code)` — phép so khoảng cách
- escape hatch `if (*(short*)(*(int*)(param_1+0x44)+4) == 0x5A9F) local_11 = true;`

**Nhưng** `FUN_0070d43c` (dispatcher click) — kiểm tra class nằm **BÊN TRONG `case 3`**, sau `CALL 0x004c9bf0`,
và `JC` tới epilogue dùng chung (asm `:275-278`):

```asm
0070d43c.asm.txt:103-109
103: MOV EAX,[0x007da7bc]
104: MOV EAX,dword ptr [EAX]
105: MOV EAX,dword ptr [EAX + 0x4]
106: CALL 0x004c9bf0
107: ADD AL,0xfb
108: SUB AL,0x4
109: JC 0x0070d84e        ; class ∈ {5,6,7,8} -> return
```

⇒ với dải này **click-to-walk/attack bị cắt**, nhưng các case khác (1, 2, 4, 6, `0x10`, `0x11`, và nhánh
`self+0x145c == 2`) **vẫn hoạt động bình thường**.

> **⚠️ ĐÍNH CHÍNH:** 2 subagent đầu viết "bỏ qua *toàn bộ* hàm". **Sai** — chỉ một arm của switch.
> Đây là dispatch theo **armed-selection mode byte** `[*gvar_007D9D34 + 0x7F]` (giá trị `0..0x11`, set bởi
> `FUN_0070c5a0`; mode `3` = "đang cầm 1 ô túi/kho"), **không** phải theo nút chuột.

⇒ **Đi lại không kiểm tra va chạm, nhưng phải do server điều khiển** — đúng mô hình avatar do server đặt vị trí
(GM teleport / observer mode), không phải client tự do.

### 4.5 🔵 Luồng E — UI / Shop: **bị chặn gần hết**

Cả hai dispatcher đều gate theo MapID `49911–49915` (`MapID - 0xC2F7 ≤ 4`).

**`FUN_0058b5a4`** (chọn chế độ chuột + mở form) — `:41-47`, `:100-106`:

```c
if ((param_2 - 7 < 2) && (*(short *)(*(int *)gvar_007DA7BC + 0x63a) == 0x29cd)) {
  toast(&DAT_0058b78c, 0x4b0, 0, 0);           // 10701: page 7/8 bị chặn riêng
}
else if ((param_2 == 6) ||
         (uVar4 = FUN_004c9bf0(self.id), 3 < (byte)((char)uVar4 - 5U))) {
  /* xử lý bình thường */
}
else {
  toast(&DAT_0058b7cc, 0x4b0, 0, 0);           // ← TỪ CHỐI, 1200ms
}
```

⇒ class `{5,7}` chỉ được vào **page 6** (`FUN_0058b7e4` = `SetupMouse(...,3)`).

**`FUN_00590ba8`** (**dispatcher shop/vendor**) — `:51-58`:

```c
if ((param_2 != 0x4) &&
    (((param_2 != 0x9 && (param_2 != 0x3)) &&
     (uVar4 = FUN_004c9bf0(self.id), (byte)((char)uVar4 - 5U) < 4)))
{
  piVar5 = toast(gvar_007DA084, &DAT_00591034, 0x5dc, 0, 0);   // ← TỪ CHỐI, 1500ms
  return piVar5;
}
```

⇒ class `{5,7}` chỉ được vào **page 3, 4, 9**.

Bảng phím nóng của `FUN_00596378:27-48` (gọi `FUN_00590ba8`):

| Phím | `param_2` | Cho phép `{5,7}`? |
|---|---|---|
| `A` | 5 | ✗ |
| `C` | 8 | ✗ |
| `D` | 7 | ✗ |
| `E` | 10 (`0xA`) | ✗ |
| **`R`** | **9** | **✓** |
| `S` | 6 | ✗ |
| `W` | 11 (`0xB`) | ✗ |

Page 3/4 không có phím nóng tương ứng. Nội dung page: `0..6` = mua/bán/xác nhận/số lượng · `7` = đóng shop ·
`8` = sửa chữa/nâng cấp · `9` = shop action hoặc `SendCommand(0x0B)` · `0xA` = `FUN_0072c988` · `0xB` =
`FUN_005c0590`.

### 4.6 🔵 Luồng F — Inventory C→S: **một lệnh độc quyền** ⭐

Đây là phát hiện quan trọng nhất, và **2 subagent đầu đọc sai** (tưởng nằm trước switch / chạy với mọi lệnh).

**Tác giả tự verify chuỗi dispatch 2 tầng:**

```bash
# redump/cs_dispatch_byte200_0x77f474.hex — hàng cmdId 0x10..0x1F:
# 10 00 11 12 13 00 14 15 16 17 18 1A 19 1B 1C 1D
#        ^^ cmdId 0x14 -> index 0x13 = 19

# redump/cs_builder_dword62_0x77f53c.hex — index 0..19:
#  [17] 8B 2E 78 00 = 0x00782E8B
#  [18] 33 2F 78 00 = 0x00782F33
#  [19] 1F 30 78 00 = 0x0078301F   <== index 0x13
```

⇒ `cmdId == 0x14` → arm `0x0078301F`. Trong arm đó có **switch con thứ hai theo sub-command** (`ECX`):

```asm
0078301f: MOV AL,byte ptr [EBP + -0x6]
00783022: DEC AL
00783023: SUB AL,0x3
00783025: JC  0x00783042     ; sub ∈ {0,1,2,3}  -> ★ NHÁNH CLASS 5/7
00783027: JZ  0x007830fc     ; sub == 4         -> nhánh MapID
0078302d: SUB AL,0x2
0078302f: JZ  0x007831ee     ; sub == 6
00783035: SUB AL,0x3
00783037: JZ  0x00783251     ; sub == 9
0078303d: JMP 0x0078a4f2     ; sub khác -> phát ra KHÔNG BYTE nào
```

| sub | Arm | Gate | Caller |
|---:|---|---|---|
| **0,1,2,3** | `0x00783042` | **chỉ class 5/7** | `FUN_005e9d94` (`CL=1`) |
| 4 | `0x007830FC` | MapID ∈ `[49401,49414] ∪ [49421,49444]` | `FUN_005ea3e0`, `FUN_005ee590` |
| 6 | `0x007831EE` | — | `FUN_005ee30c` |
| 9 | `0x00783251` | — | `FUN_005d7000` |

Block class 5/7 (`asm.txt:1299-1345`):

```asm
0078304c: CALL 0x004c9bf0
00783051: ADD AL,0xfb ; 00783053: SUB AL,0x4
00783055: JC 0x0078a4f2                      ; KHÔNG thuộc dải -> emit NOTHING
0078305b: LEA EAX,[EBP-0x34]
0078305e: MOV DL,byte ptr [EBP + -0x5]       ; cmdId
00783061: MOV byte ptr [EAX + 0x1],DL
00783064: MOV byte ptr [EAX],0x1             ; ShortString len = 1
...
0078307d: MOV DL,byte ptr [EDX + 0xa089]      ; self+0xA089  (request kind)
00783091: CALL 0x00402b60                    ; @PStrNCat -> 2 ký tự
...
007830b8: MOV DX,word ptr [EAX + 0xa085]     ; self+0xA085 (armed slot / zone index)
007830c6: CALL 0x0077eb1c                    ; word -> 2 byte LE
007830e4: CALL 0x0051633c                    ; TForm1_CY_AddSedQueue(S)  -> GỬI
007830f0: MOV byte ptr [EAX + 0xa094],0x1     ; set busy latch
```

⇒ **Client chỉ gửi `cmd 0x14` sub-1 (đưa Thing cho ô đang "armed") khi class ∈ {5,7}**.
Mọi class khác gọi cùng call site đó ⇒ **phát ra 0 byte trên wire**.

**Đây là đặc quyền GM duy nhất client có thể phát** — và nó là đặc quyền *inspector/ops*, không phải quyền ghi.

**Ngữ nghĩa các field trên `gvar_007DA5A0`** (mà 2 subagent gọi nhầm là "scene/config object"): thực ra là
**container Thing + state pending-operation của nhân vật local**, record ~`0xA140` byte:

| Offset | Nội dung | Writer |
|---|---|---|
| `+0x10 + i*4` (i≤100) | mảng pointer Thing | `005e9d94:81,86,159` |
| `+0x338 + i*4` | mảng **rectangle** (left/top/w/h + flag) | `005ea3e0:41-64` |
| `+0xA085` (word) | **ô/zone đang armed** | `005e9d94:152`, `005ea5d4:99`, `005ea804:72` |
| `+0xA089` (byte) | **request kind** (1 = actor-target, 2 = zone/rect, 3 = khác) | `005e9d94:153`(1), `005ea5d4:100`(2), `005ea804:73`(3) |
| `+0xA094` (byte) | **cờ busy / request outstanding** | writer: 12 site trong `case_018` (reply) + `005eb530:678` |

Nhánh MapID (`asm.txt:1346-1370`) cũng set `+0xA094 = 1` sau khi gửi, nhưng **không có test class**; nó gọi
`FUN_0074b330(localPlayer, 3)` → `dest := currentPos` + `SendCommand(6)` = **force stop**.

### 4.7 🔵 Luồng G — Ma trận capability `FUN_004c9d3c`

Signature: `FUN_004c9d3c(int id, int featureId)` — boolean "class này có được dùng feature F không".
Dispatch: `cmp featureId, 0x6a` → byte-map `0x4c9d72` → dword jump-table `0x4c9ddd`.

**42 case.** Ma trận rút gọn (đã prune class 1/4/6/8 không thể xảy ra):

| Nhóm feature | Class được cấp |
|---|---|
| `0x14` (20) | **`{5,7}` — EXCLUSIVE** ⭐ |
| `0x16` (22) | **`{5}` — EXCLUSIVE** ⭐ |
| `0x0F,0x10,0x11,0x12,0x13,0x18,0x1B` | `{2,5,7}` |
| `0x19,0x1A,0x1C` | `{2,5}` |
| `0x02,0x03,0x05,0x06,0x08,0x0A,0x0C,0x0D,0x0E,0x17,0x65,0x67` | `{2,3,5,7}` |
| `0x01,0x66` | `{2,3,5,7,11}` |
| `0x04,0x07,0x09,0x0B` | `{2,3,5}` |
| `0x15,0x32,0x33,0x46,0x47,0x48,0x49,0x64` | `{2,3,11}` |
| `0x68,0x69,0x6A` | **`{2,3}`** |
| `0x00`, default | không ai |

**18 call site — tất cả đều truyền ID của CHÍNH MÌNH** (`MOV EAX,[0x007da7bc]; MOV EAX,[EAX]; MOV EAX,[EAX+4]`)
và chỉ dùng **4 feature**:

| feature | Caller | UI bị gate |
|---|---|---|
| `0x68` | `FUN_005d3908:54` | nút tuỳ chọn thêm ở màn tạo nhân vật (image `form_NewRole_1`) |
| `0x69` | **15 handler** `005fc840/870/91c/95c/cbf4/cc2c/cc70/ca8/f4c/f84/d014/d548/d580/d5c0/d5f8` | **thanh module `btn_module`** của `TBK_GamePlatform` + `TBK_GamePopupMenu` |
| `0x46` | `FUN_005fcba4:23` | `Show()` form vendor |
| `0x48` | `FUN_005fd72c:25` | refresh 2 list server (`SendCommand(1,8)` ×2) |

15 handler `0x69` tạo thành **một thanh 6 nút**: 12 handler chỉ là `Visible := not Visible` (mở/đóng panel),
3 handler phát `SendCommand(0x16)`, `SendCommand(1,0x0D)`, `SetupMouse(2/3)` + `PlaySound("sound\WA0018.wav")`.

> **⭐ Bằng chứng còn sót của bản GM:** feature `0x14` (exclusive `{5,7}`) và `0x16` (exclusive `{5}`)
> **không có call site nào** trong toàn bộ corpus truyền. Đây là enum còn sót từ bản GM cũ — **bảng feature
> được viết cho cả 2 thế giới (player tier + GM tier) nhưng code GM đã bị gỡ khỏi bản release**.
>
> Nói cách khác: `FUN_004c9d3c` là **bảng quyền GM còn sót**, và mọi hành vi GM còn hoạt động (§4.1–§4.6) đều
> **nằm ngoài** bảng này — chúng dùng trực tiếp `FUN_004c9bf0`.

---

## 5. TỔNG HỢP HÀNH VI THEO VAI TRÒ

| Hành vi | Local id ∈ dải | Remote id ∈ dải |
|---|---|---|
| Ép `+0x08 = 0`, `+0x7C = 120` | ✅ `:149-153` | ✅ `:490-502` + 7 site khác |
| Ẩn hoàn toàn khi `[100,150]` hoặc `[301,400]` | n/a (self không tự ẩn) | ✅ `0071e340`/`0071f5f4` |
| Chat ép màu slot `(GM)` | ✅ (`005964c8` set cờ riêng) | ✅ |
| Chat thay tên bằng số ID (tag 1/2/3) | — | ✅ |
| Chat tag 3 = `(GM)Thiên thần` | — | ✅ |
| Bị chặn tấn công/giao dịch/kết bạn/nhắn tin | ✅ (mục tiêu là self) | ✅ |
| Bypass collision & reach | ✅ `0073bd24` | — |
| Mất click-to-walk/attack | ✅ `0070d43c` case 3 | — |
| Mất gần hết UI shop/vendor | ✅ `0058b5a4` + `00590ba8` | — |
| **Được phát `cmd 0x14` sub 1** | ✅ **độc quyền** | — |
| Không thuộc bất kỳ feature nào của `004c9d3c` | ✅ | — |

**Profile tổng thể của tài khoản GM-band:**

1. **Bất khả xâm phạm** — không ai tấn công / giao dịch / kết bạn / nhắn tin được.
2. **Điều khiển vị trí do server đặt** — không kiểm tra va chạm, nhưng không tự điều khiển.
3. **UI bị thu hẹp mạnh** — gần như mọi cửa sổ shop/vendor đều bị từ chối.
4. **Danh tính hình ảnh bị đồng nhất hoá** — model cố định 1120, không trang bị, không prefix tên; phần lớn
   bị ẩn hoàn toàn.
5. **Một kênh giao tiếp riêng** — chat tag riêng + cờ `InputBar+0x19C` chỉ người GM mới thấy.
6. **Một đặc quyền gửi lệnh** — `cmd 0x14` sub 1.
7. **Avatar bị ẩn khỏi client khác** — nếu client khác cùng dùng guardrail này thì cũng ẩn.

⇒ Đây là **profile "GM / Admin / operator ở chế độ quan sát"**, KHÔNG phải GM console đầy đủ.

---

## 6. ĐỐI CHIẾU VỚI SERVER RUST (`src/`)

> Người viết cho biết chưa thấy khái niệm này trong source server. **Xác nhận: đúng, `src/` chưa có.**

| Vị trí | Nội dung hiện tại | Cần sửa |
|---|---|---|
| `src/protocol/mod.rs:53` | `/// Frame id must be the *sender* id; ids 100..400 take NPC branches.` | → dải GM/Thiên thần, không phải NPC |
| `src/server/handlers/system.rs` | `gm_level > PLAYER_GM_LEVEL` gate cho GM shop | tách khỏi dải ID — đây là **2 hệ thống quyền khác nhau** |
| `DB/` schema | `gmlevel` (`0001_init.sql:18`) | chưa map sang dải `100..400` |
| `src/protocol/codecs/` | không có codec nào phân nhánh theo dải id | cần thêm `fn is_gm_band(id: u32) -> bool` |

**Đề xuất đặt tên cho Rust port:**

```rust
/// Dải ObjectID dành riêng cho tài khoản GM/Admin và sender hệ thống ("Thiên thần").
///
/// Bằng chứng từ client (aLogin.exe):
///   * `FUN_004c9bf0` @ 0x004C9BF0: id 100..150 -> class 5, id 151..400 -> class 7.
///     Guard idiom `(byte)(class - 5) < 4` == `100 <= id <= 400`, dùng ở 18 call site.
///   * `FUN_007ab870` @ 0x007AB870 ép chat sang kênh 4 = slot nhãn `(GM)` (VISCII @0x7ABE54),
///     thay tên người gửi bằng `IntToStr(id)`, và dùng nhãn `(GM)Thiên thần` (@0x7ABE14)
///     cho tag 3 khi 100 <= id <= 400.
///   * id == 100 -> nhãn `Hệ thống:` (@0x7ABE00)  => sender hệ thống.
///   * Toast @0x75AF20: "Không thể giao dịch với Thiên Thần".
///   * Actor trong dải là TPlayers (VMT_70B5D0), cùng lớp với người chơi; quái là TMapNpc id 800..899.
pub const GM_BAND: std::ops::RangeInclusive<u32> = 100..=400;
pub const GM_BAND_TIER_PRIMARY:   RangeInclusive<u32> = 100..=150; // class 5, superset của tier 2
pub const GM_BAND_TIER_SECONDARY: RangeInclusive<u32> = 151..=400; // class 7
pub const SYSTEM_SENDER_ID: u32 = 100;                            // nhãn "Hệ thống:"

#[inline]
pub fn is_gm_band(id: u32) -> bool { GM_BAND.contains(&id) }
```

---

## 7. BẢNG GIẢI MÃ CHUỖI ĐẦY ĐỦ

Nguồn: `test/ts_decompile/redump/lit_7ABD54.hex`, `lit_7ABDAC.hex`, `lit_7ABDF0.hex`, `lit_7ABE40.hex`,
`lit_7ABE64.hex`, `client_pseudo_c/lit_75af20.hex`. Bảng VISCII: `src/encoding.rs:128-236`.

| Hằng số | VA | len | Byte | Giải mã | Dùng ở |
|---|---|---:|---|---|---|
| `DAT_007abd54` | `0x7ABD54` | 18 | `28 43 F4 6E 67 20 62 AF 20 68 AE 20 74 68 AF 6E 67 29` | **(Công bố hệ thống)** | tag 0, chunk đầu của tag `0x0B` |
| `DAT_007abd70` | `0x7ABD70` | 16 | `28 54 68 A5 6E 29 54 68 69 EA 6E 20 74 68 A5 6E` | **(Thần)Thiên thần** | tag 1, nhánh ngoài dải |
| `DAT_007abd8c` | `0x7ABD8C` | 9 | `54 69 AA 6E 67 20 67 F7 ??` | **"Tiếng cọ…"** ⚠️ byte `0x7ABD94` chưa dump | tag 1, **trong dải** |
| `DAT_007abdac` | `0x7ABDAC` | 8 | `28 54 A4 74 20 43 E4 29` | **(Tất Cả)** | tag 1 ngoài dải; tag 2 self-map-`0x5A3C` |
| `DAT_007abdc0` | `0x7ABDC0` | 15 | `28 47 A5 6E 29 54 68 69 EA 6E 20 74 68 A5 6E` | **(Gần)Thiên thần** | tag 2, **trong dải** |
| `DAT_007abdd8` | `0x7ABDD8` | 13 | `48 D5 6F 20 6E 68 F6 20 74 69 AA 6E 67` | **Hạo nhỏ tiếng** ⚠️ lỗi chính tả trong bản game | tag 2, **trong dải** |
| `DAT_007abdf0` | `0x7ABDF0` | 5 | `28 47 A5 6E 29` | **(Gần)** | tag 2, ngoài dải |
| `DAT_007abe00` | `0x7ABE00` | 9 | `48 AE 20 74 68 AF 6E 67 3A` | **Hệ thống:** ⭐ | tag 3, **`id == 100`** |
| `DAT_007abe14` | `0x7ABE14` | 14 | `28 47 4D 29 54 68 69 EA 6E 20 74 68 A5 6E` | **(GM)Thiên thần** ⭐ | tag 3, **`100 ≤ id ≤ 400`** |
| `DAT_007abe2c` | `0x7ABE2C` | 8 | `4E F3 69 20 ?? ?? ?? ??` | **"Nói …"** ⚠️ 4 byte chưa dump | tag 3, **trong dải** |
| `DAT_007abe40` | `0x7ABE40` | 10 | `28 54 68 EC 20 54 68 A5 6D 29` | **(Thì Thầm)** | tag 3, người chơi |
| `DAT_007abe54` | `0x7ABE54` | 4 | `28 47 4D 29` | **(GM)** ⭐ | tag 4 = **màu ép cho dải** |
| `DAT_007abe64` | `0x7ABE64` | 5 | `28 D0 B5 69 29` | **(Đội)** — Team | tag 5 |
| `DAT_007abe74` | `0x7ABE74` | 6 | `28 D0 6F E0 6E 29` | **(Đoàn)** | tag 6 |
| *(inline)* | `0x7ABE84` | 6 | `28 4D 69 6E 68 29` | **(Minh)** | tag 7 |
| *(inline)* | `0x7ABE94` | 4 | `23 65 6E 64` | `#end` | sentinel tag `0x0B` |
| `DAT_007abd…` | `0x75AF20` | 34 | `4B 68 F4 6E 67 20 74 68 AC …` | **Không thể giao dịch với Thiên Thần** ⭐ | `FUN_0075ad70` |
| `LAB_007abd3d` | — | — | — | **KHÔNG phải chuỗi** — nhãn SEH handler | `007ab870.asm.txt:19-22` |

### 7.1 ⚠️ Bug công cụ giải mã: offset `lit_*.hex` lệch 4 byte

`lit_<ADDR>.hex` chứa byte **bắt đầu tại `ADDR-4`**, vì Delphi `AnsiString` trỏ tới byte ký tự đầu tiên
còn **len dword nằm ở `ptr-4`** (đó là thứ `_LStrCatN` đọc: `ADD EAX,dword ptr [ECX + -0x4]`,
`00404148.asm.txt:14`).

⇒ **Text in ra đúng, nhưng ĐỊA CHỈ in ra cao hơn 4.** Ba bằng chứng:

1. File `lit_7ABE40.hex` (start `0x7ABE3C`) và `lit_7ABE64.hex` (start `0x7ABE60`) **chồng lấn** trên
   `0x7ABE60..0x7ABE7F` và **khớp byte-for-byte**.
2. Với `start = ADDR-4`, mọi refcount `FF FF FF FF` rơi đúng boundary 4 byte.
3. Mọi hằng push đều resolve thành `<len>` + NUL hợp lệ.

⇒ `tools_decode_literals.py:40` (`start=int(hex-from-filename)`) **lệch +4**.

**Ngoài ra:** `redump/lit_7ABE74.hex` **KHÔNG** bị mislabel (nó là `0x7ABE70..`, đúng quy tắc) — trùng nội dung
byte-for-byte với `lit_7ABE64.hex[16..67]`. Claim "mislabel" của subagent là **sai**.

---

## 8. DANH SÁCH CHUỖI CÒN THIẾU (hành động tiếp theo)

13 toast dưới đây **không có file `.hex` nào phủ** (đã liệt kê toàn bộ ~250 file `lit_*.hex` ở cả 2 corpus).
Chúng nằm trong **section code**, thường ngay sau thân hàm chủ, và Ghidra chỉ render thành immediate
`&DAT_…` chứ không render thành chuỗi.

| Hằng số | VA | Hàm chặn | Nội dung lẽ ra |
|---|---|---|---|
| `DAT_00562c90` | `0x562C90` | `FUN_00562a34` | "không thể chọn …" |
| `DAT_0058b7cc` | `0x58B7CC` | `FUN_0058b5a4` | lý do chặn UI |
| `DAT_00591034` | `0x591034` | `FUN_00590ba8` | lý do chặn shop |
| `DAT_00643244` | `0x643244` | `FUN_00642c2c` | "không thể tấn công …" |
| `DAT_0064ecd4` | `0x64ECD4` | `FUN_0064ea20` | "không thể …" |
| `DAT_0064f460` | `0x64F460` | `FUN_0064efa0` | "không thể …" |
| `DAT_0064f9b0` | `0x64F9B0` | `FUN_0064f7bc` | "không thể …" |
| `DAT_0075b81c` | `0x75B81C` | `FUN_0075b66c` | "không thể …" |
| `DAT_0075ed10` | `0x75ED10` | `FUN_0075ebc0` | "không thể kết bạn …" |
| `DAT_007527b8` | `0x7527B8` | `FUN_00752728` | "không thể …" |
| `DAT_007a31fc` | `0x7A31FC` | `FUN_007a2d20` | "không thể giao dịch …" |
| `DAT_007a35fc` | `0x7A35FC` | `FUN_007a3260` | "không thể giao dịch …" |

Dump đề xuất:

```bash
# dump ~32 byte cho mỗi địa chỉ (đủ chứa [FF FF FF FF][len][chars][00])
objdump -s -j .text --start-address=0x562c90 --stop-address=0x562cb0 aLogin.exe
```

Nội dung 12 chuỗi này sẽ nói thẳng dải 100–400 bị gọi là gì **khi người chơi cố tương tác** — nhiều khả năng
xác nhận lần nữa chữ "Thiên thần" / "GM", hoặc mở ra tên rank nội bộ (Mod/Admin).

---

## 9. NHẬT KÝ MÂU THUẪN & ĐÍNH CHÍNH

Có **3 mâu thuẫn** giữa các subagent. Tất cả đã giải quyết bằng byte/ASM verification.

### 9.1 "NPC/monster band" vs "GM band"

| Nguồn | Kết luận | Bằng chứng mà họ đưa ra |
|---|---|---|
| Subagent #1 (chat) | *server-reserved pseudo-entity* | tên bị thay bằng số ID; `id==100` gọi là "Hệ thống" |
| Subagent #2 (toast) | *NPC/monster* | 1 toast giải mã được không chứa từ quyền |
| Subagent #3 (capability) | *restricted/denied tier, KHÔNG đặc quyền* | mọi feature của `004c9d3c` đều chặn `{5,7}` |
| Subagent #4 (VMT) | **player-class, ~90%** | `TPlayers` vs `TMapNpc` id 800–899 |
| **Tác giả (verify)** | **GM band** | `TPlayers` (10/10 site spawn, asm) + 4 chuỗi literal + collision bypass + `cmd 0x14` độc quyền |

**Phán quyết:** subagent #3 **đọc sai cực**. Nó kết luận "bị từ chối ⇒ không đặc quyền", bỏ qua rằng:
`FUN_0073bd24` **bypass toàn bộ collision/reach**, `cmd 0x14` sub 1 **độc quyền**, và 2 feature
`0x14`/`0x16` trong `004c9d3c` là **exclusive `{5,7}`**. Đó là hồ sơ đặc quyền, không phải hạn chế.
Subagent #4 là bằng chứng cấu trúc mạnh nhất và đã được xác nhận.

Subagent #2 dựa trên **1 toast duy nhất** (`0x75AF20` = "Không thể giao dịch với Thiên Thần") và **đọc sai nó** —
chuỗi đó chứa chữ **"Thiên Thần"**, tức chính là tên client đặt cho dải, chứ không phải bằng chứng chống lại.

### 9.2 Vị trí block `cmd 0x14`

Subagent #1 và #3 viết "block nằm **trước switch**, chạy với **mọi** lệnh". **Sai.**
Đã tự verify 2 tầng dispatch (§4.6): `cmdId 0x14 → index 0x13 → 0x0078301F`, trong đó switch con theo
`ECX`; block class-5/7 nằm ở arm `sub ∈ {0,1,2,3}`.

### 9.3 Cực của `FUN_0073bd24`

Subagent #1 và #2 cùng nói "`TRUE` = bypass" nhưng **không ai kiểm chứng cực**.
Đã tự đọc `00712d7c:178-236` và `00731ffc:42-141`: `FALSE` ⇒ toast + xóa hàng đợi + reset đích;
`TRUE` ⇒ phát `SendCommand(6)`. ⇒ **`TRUE` = đi**, bypass là đúng.

Ngoài ra `FUN_0070d43c` **không** "bỏ qua toàn bộ hàm" mà chỉ bỏ arm `case 3`.

### 9.4 Đính chính khác

| Claim sai | Sự thật |
|---|---|
| "`+0x79` không tìm thấy writer" | writer là **constructor**: `TPlayer.Create`=1, `TPlayers.Create`=2, `TMapNpc.Create`=3 … |
| "class 6/8 được trả về" | không bao giờ; nhưng code vẫn test ⇒ bảng legacy |
| "`redump/lit_7ABE74.hex` bị mislabel" | không; đúng quy tắc `ADDR-4`, trùng nội dung `lit_7ABE64.hex` |
| "`0x7ABDD8` = *Hơi nhỏ tiếng*" | **"Hạo nhỏ tiếng"** (`0xD5`=ạ, `0x6F`=o) |
| "`0x7ABDAC` = *(Tổng Cũ)*" | **"(Tất Cả)"** (`0xA4`=ấ, `0xE4`=ả) |
| "`0x7ABE64` = *(Đài)*" | **"(Đội)"** = Team (`0xB5`=ộ) |
| "class 10 = `[1057,1120]`" | `[1058,1119]`; 1057 là class 9, 1120+ là class 0 |
| "class 11 = `[33,38] ∪ [41,47]`" | chỉ `{24,25,39,40}` — nhánh bị khối `0xA` claim trước |
| "SubOp `0x01` gate = `class ∈ [5..8]`" | **đảo ngược** — hiển thị khi `class ∉ [5..8]` **hoặc** UI check khác 0 |

---

## 10. DANH SÁCH SỬA TÀI LIỆU

| File:line | Nội dung cũ | Sửa thành |
|---|---|---|
| `opcode_02.md:116` | "`id∈[100,400]` (NPC/kênh hệ thống)" | **dải GM/Thiên thần** |
| `opcode_02.md:63-64` | "NPC → color=4" | color 4 = slot `(GM)`, **ép** cho dải |
| `opcode_02.md:105` | gate SubOp `0x01` là `class ∈[5..8]` | **đảo ngược** (xem §4.2) |
| `opcode_02.md:154` | `0x7ABDAC = "(Tổng Cũ)"` | **`"(Tất Cả)"`** |
| `opcode_02.md:156` | `0x7ABDD8 = "Hơi nhỏ tiếng"` | **`"Hạo nhỏ tiếng"`** |
| `opcode_02.md:162` | `0x7ABE64 = "(Đài)"` | **`"(Đội)"`** |
| `opcode_02.md:192` | "`100≤id≤400` (NPC/hệ thống)" | dải GM |
| `opcode_04.md:152` | "nhóm `[5..8]` ≈ NPC/monster theo dải ID" | **dải ID GM/Admin** |
| `opcode_03.md:112` | "ép trạng thái/anim cho một số loại map" | ép **ngoại hình**, không liên quan map |
| `src/protocol/mod.rs:53` | "ids 100..400 take NPC branches" | dải sender GM |
| `tools_decode_literals.py:40` | `start = hex(filename)` | **`start = hex(filename) - 4`** |

---

## 11. PHẦN CÒN MỞ

| # | Việc | Vì sao quan trọng |
|---|---|---|
| 1 | Dump 13 chuỗi toast ở §8 | sẽ cho biết client gọi dải này bằng từ gì khi người chơi cố tương tác |
| 2 | Xác định sprite resource **1120** (`+0x7C = 120`) là avatar nào | nếu là model admin/statue thì củng cố kết luận |
| 3 | Ý nghĩa `+0x44` → `*(short *)(rec+0x44 + 4)`, so với `0x5A3C`(23068) / `0xB3B6`(45974) | là **MapID**, không phải guild id. Không tìm thấy writer trong `.c` — nằm ở vùng asm-only |
| 4 | Ý nghĩa `DAT_007abda0` (dấu `:` phân tách name:msg) | dump thiếu, record nằm trong hole `0x7ABD94..0x7ABDA7` |
| 5 | Feature `0x14` / `0x16` của `FUN_004c9d3c` | là gì? Call site nào đã bị gỡ khỏi bản release |
| 6 | Byte cuối của `DAT_007abd8c` (`0x7ABD94`) và `DAT_007abe2c` (`0x7ABE30..0x7ABE33`) | hoàn thiện 2 chuỗi |
| 7 | Bảng màu của chat-list control (tag → RGB) | control VMT quanh `0x7AB5xx`, không decompile, không có `vmt_*.hex` |
| 8 | `case 035` (`FUN_007938c3`) tag = `(char)in_stack_00000000` | artifact decompiler — cần asm `0x007938C3` |
| 9 | Vì sao dải `[151,300]` vẫn render còn `[301,400]` thì ẩn | quy ước sub-range hay legacy |

---

## 12. PHỤ LỤC — NHẬT KÝ BYTE-VERIFY CỦA TÁC GIẢ

Các lệnh đã chạy trực tiếp (không qua subagent):

```bash
$ python3 tools_decode_literals.py 75af20 75af5f
### range 0x0075AF20..0x0075AF5F
0x0075AF20 len= 34  Không thể giao dịch với Thiên Thần

$ python3 tools_decode_literals.py 7abd50 7abec3
0x007ABD74 len= 16  (Thần)Thiên thần
0x007ABDC4 len= 15  (Gần)Thiên thần
0x007ABDDC len= 13  Hạo nhỏ tiếng
0x007ABDF4 len=  5  (Gần)
0x007ABE04 len=  9  Hệ thống:
0x007ABE18 len= 14  (GM)Thiên thần
0x007ABE58 len=  4  (GM)
0x007ABE68 len=  5  (Đội)
0x007ABE78 len=  6  (Đoàn)
0x007ABE88 len=  6  (Minh)
0x007ABE98 len=  4  #end
```

`DAT_007abdac` phải giải mã tay vì tool không thấy (thiếu refcount ở `0x7ABDA8` trong index):

```bash
$ cat redump/lit_7ABDAC.hex
08 00 00 00 28 54 A4 74 20 43 E4 29 00 00 00 00 ...
# ^len=8, chars = "(Tất Cả)"  (0xA4=ấ, 0xE4=ả theo src/encoding.rs:172,218)
```

Tra cứu bảng VISCII:

```bash
$ grep -nE "0x(A4|E4|D5|F6|AC|B8|BE|BD)\b" src/encoding.rs
172: 0xA4 => '\u{1EA5}', // ấ
178: 0xAA => '\u{1EBF}', // ế
180: 0xAC => '\u{1EC3}', // ể
192: 0xB8 => '\u{1ECB}', // ị
197: 0xBD => '\u{01A1}', // ơ
198: 0xBE => '\u{1EDB}', // ớ
209: 0xD5 => '\u{1EA1}', // ạ
218: 0xE4 => '\u{1EA3}', // ả
227: 0xF6 => '\u{1ECF}', // ỏ
```

Verify chuỗi dispatch 2 tầng của `FUN_0077f414`:

```bash
$ sed -n '3,4p' redump/cs_dispatch_byte200_0x77f474.hex
10 00 11 12 13 00 14 15 16 17 18 1A 19 1B 1C 1D     # cmdId 0x14 -> index 0x13

$ sed -n '5,6p' redump/cs_builder_dword62_0x77f53c.hex
8B 2E 78 00 | 1F 30 78 00                              # index 19 (0x13) = 0x0078301F
```