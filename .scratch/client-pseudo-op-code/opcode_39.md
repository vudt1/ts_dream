# PHAN TICH — Main OP 0x39 (Case 50) — `FUN_00795579` @ `0x00795579` — **Dieu khien popup TSportManage: mo form theo id / dong form hien tai / hien banner (Server → Client)**

Ngay: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trang thai: **Da xac minh tu ma nguon so cap** (`ts_decompile/`). Anh xa jump table da kiem: `jumptable_byte200_0x78A8EE[0x39] = 50` → entry `0x0078AA7E` cua `jumptable_dword200_0x78A9B6` → target `0x00795579` = **Case 50** (tinh truc tiep tu 2 file `.hex` trong `redump/`). doi chep kep: inline trong dispatcher `0078a89c_FUN_0078a89c.c:6857–6919` (marker SEH `UNK_0079558c/007955d4/0079564b/00795663`) khop 100% `case_050`.

> Pham vi: **core logic opcode**; chi tiet render/animation cua form chi neu 1 dong de chung minh consumer du lieu.

---

## 1. Tom tat nghiep vu

OP 0x39 la **"remote control" cua `TSportManage`** — manager popup dang dong/cay (giao diem/su kien?) nam tren main form. `SubOp = RP[0]`, switch tren 1/2/3:

| SubOp | Hanh dong |
| :--- | :--- |
| `1` | **Mo form dang popup theo id**: `func_0x0055374c(mgr, RP[1], 0)` (ham nam trong HOLE — xem §4.1). Neu `RP[1] == 3` thi doc them `RP[2]`: `1` → ghi `form#3 + 0x38 = 100`; `2` → `= 1000` (value tuy chon, xem §5.3). |
| `2` | **Dong & giai phong form dang mo**: `FUN_00553410(mgr)` — free form theo `mgr+4` hien tai, set `mgr+4 = 0` (§4.2). Khong touch socket/queue. |
| `3` | **Banner/thong bao 1.2 giay**: `RP[1]==1` → hien chuoi `&UNK_00799200`, `RP[1]==2` → `&UNK_00799224`, qua slot `+0x90` cua `TSe_TalkMsgFormPlus` (`gvar_007DA084`), duration `0x4b0 = 1200ms` (§4.3). |

- **C→S: KHONG CO duong gui 0x39** — `case 0x39:` trong builder `FUN_0077F414` **ton tai nhung rong** (`break;` khong lam gi, `0077f414_FUN_0077F414.c:1020–1021`). Dim mau hiem: client **co** ham phat yeu cau gui 0x39 (`FUN_00553818`) tu callback nut cua form#1, nhung no chay vao case rong nen **khong frame nao ra loi** (§6).
- Chuoi banner `0x799200/0x799224` **chua dump** — mock server khong can biet noi dung (client tu in literal cuc bo), nhung tai lieu nay khong dich duoc (§7).

---

## 2. Entry & cach doc payload

Framing/dispatcher tay giao theo tien le da xac minh: `[F4 44][Len:Word LE][Payload]` XOR `0xAD`; `TForm1.CY_DelRevQueue` (`00516158_TForm1.CY_DelRevQueue.c:86–93`) cat `_LStrCopy(msg, 2, Len-1)` → **RP = payload bo MainOp**, chuoi Delphi 1-based: `RP[0]` = SubOp, `RP[1]` = byte dau tien cua tham so, `*(int*)(RP-4)` = do dai.

`FUN_00795579` (`case_050_00795579_FUN_00795579.c:8–77`, DOC TOAN BO):
```pascal
// Tuong duong Delphi (bo guard BoundErr):
SubOp := Ord(RP[1]);                       // BoundErr neu RP rong
case SubOp of
  1: begin
       TSportManage_Open(RP[2], 0);        // func_0x0055374c — HOLE (§4.1)
       if RP[2] = 3 then
         case RP[3] of                     // BoundErr neu thieu byte
           1: TPortForm3.Field_38 := 100;
           2: TPortForm3.Field_38 := 1000;
         end;
     end;
  2: TSportManage_CloseCurrent;            // FUN_00553410 (§4.2)
  3: case RP[2] of
       1: TalkMsg.Banner(@UNK_00799200, 1200, 0, 0);   // VMT+0x90 (§4.3)
       2: TalkMsg.Banner(@UNK_00799224, 1200, 0, 0);
     end;
end;
// SubOp khac {1,2,3}: im lang, khong lam gi.
```
Toan bo `if len < k → _BoundErr` la guard index Delphi (`case_050.c:26,35,42,49,67`); vuot nguong = **ERangeError giua handler**, khong phai no-op. Duoi `case_050.c:78–105` la SEH cleanup frame dung cua dispatcher (`*in_FS_OFFSET`, `LAB_00796408`, `0x7963xx`) — khong phai logic.

---

## 3. Wire layout (offset tren Payload, `[0]` = MainOp)

| Offset | Noi dung | Rang buoc / xu ly client |
| :--- | :--- | :--- |
| `[0]` | `0x39` | — |
| `[1]` | SubOp | chi 1/2/3 co nghia; khac → bo qua im lang; **payload rong → BoundErr** |
| `[2]` (SubOp 1) | `id` form muon mo | ham mo nam trong HOLE; id hop le suy ra tu bang dong: `{1,2,3,4,6,0xFF}` (§5.2) |
| `[3]` (SubOp 1, chi khi `id==3`) | mode | `1` → form3`+0x38`=100; `2` → =1000; khac → khong ghi. **Thieu byte → BoundErr** |
| `[2]` (SubOp 3) | ma banner | `1` → literal `0x799200`; `2` → `0x799224`; khac → im lang. Thieu byte → BoundErr |

Khong co truong Word/DWORD chuoi nao — moi la byte don. SubOp 2 khong co tham so.

---

## 4. Phan tich tung SubOp + helper

### 4.1. SubOp 1 — `func_0x0055374c(mgr, id, 0)` : **HOLE**
- `index.csv` (kiem tra bang python): khong co ham nao phu `0x0055374C`; `FUN_00553410` ket thuc `0x005534E3`, tiep do la `TSportManage.Create` `0x005534E4–0x00553525`, `FUN_00553528` `0x00553528–0x0055355D`, roi `FUN_00553818` `0x00553818` → **hole `0x0055355D–0x00553818` chua `0x0055374C`**.
- Bang chung mo rong quanh hole (khong phai than ham):
  - cung hole co `0x00553574 → FUN_00553410 [UNCONDITIONAL_CALL]` (`00553410.c:14`) — code hole goi lai dong form;
  - `0x0055374C` duoc goi DUY NHAT tai day (`0078a89c.c:6876` + `case_050.c:39`), ky hieu `func_0x...` cua Ghidra (ham trong hole, khong export).
- **Suy luan co kiem chung (khong khanging dinh)**: day la `OpenForm(id, flag)` cua `TSportManage` — bang chung gian tiep: (a) `FUN_00553410` map `mgr+4 ∈ {1,2,3,4,6,0xFF}` → form tuong ung; (b) SubOp 1 xu ly rieng `id==3` vi et `gvar_007DA778` (dung la form map voi id 3 trong `00553410.c:39–41`); (c) nhieu guard input chi cho thao tac khi `*(mgr+4)==0` (§5.1) tuc "chua mo form nao". **Chi tiet branch cua id 1/2/4/6/0xFF va gia tri mode cua form 3 => UNKNOWN cho den khi redump `0x0055355D–0x00553818`.**

### 4.2. SubOp 2 — `FUN_00553410(gvar_007D9D88)` : dong form hien tai (PHAN TICH KY)
`00553410_FUN_00553410.c:24–53` (211 bytes):
```pascal
id := TPortManage(mgr).ActiveId;            // byte @ mgr+4
if id <> 0 then begin
  case id of
    1:   Form_DA42C.Free;   Form_DA42C  := nil;   // :32-34
    2:   Form_DA0F4.Free;   Form_DA0F4  := nil;   // :36-38
    3:   Form_DA778.Free;   Form_DA778  := nil;   // :40-42  ← form cua SubOp 1
    4:   Form_D9F98.Free;   Form_D9F98  := nil;   // :28-30
    6:   Form_DA4EC.Free;   Form_DA4EC  := nil;   // :44-47
    0xFF:Form_DA0A0.Free;   Form_DA0A0  := nil;   // :48-51
  end;
  TPortManage(mgr).ActiveId := 0;                 // :52
end;
```
- Chi goi `TObject.Free @ 0x403074` (`00553410.c:11`) — **khong** socket, **khong** `CY_AddSedQueue`, **khong** guoc nguoc C→S. Day la lenh dong cua server, khong co ack.
- Cung khuon map id nay co 2 "anh em" la method khac cua TSportManage (cung address range): `FUN_00553840` (`00553840.c:28–50`, caller tai `0x00515d8e` — **HOLE** vung tick loop `~0x5157E6`, theo handoff) goi **ham tick/refresh tung form** (`0x548080/0x5524c4/0x541c6c/0x54e51c/0x54fbac/0x5531f4`), va `FUN_0055391c` (`0055391c.c:28–50`, caller `0x00515c15` — cung HOLE tick) goi nhip thu 2 (`0x548e98/0x552e58/0x541f8c/...`). Tuc `ActiveId` quyet dinh form nao duoc pump moi tick.
- `FUN_00603f20` (ham "rời world/reset ket noi", `00603f20.c:7,100–101`) cung goi `FUN_00553410` tai `0x006043e5` — khop voi tien le `opcode_36.md §5.2` (cung ham nay Hide form chon server).

### 4.3. SubOp 3 — goi `VMT+0x90` cua `gvar_007DA084`: **TU KIEM CHUNG (khong ke thua opcode_2b/33)**
**Danh tinh**: `gvar_007DA084` = instance **`TSe_TalkMsgFormPlus`** — classref `VMT_63B39C`, sinh trong FormCreate (`0051189c_FUN_0051189c.c:1265–1266`), dat panel `"panel10"` Y=250 ngay sau do (`:1275`), dang ky vao widget-manager `gvar_007DA234` (`:1277`, global nay sinh tai `0051189c.c:1156`).

**Xac dinh goc VMT**: nhan `VMT_63B39C` chinh la gia tri luong trong object (classref). Chung minh: ham `FUN_007AFA94` (virtual he thong, ~190 class co) nam tai `classref+0x7C` cho MOI class kiem tra duoc — `0x63b418-0x63b39c=0x7C` (TalkMsg), `0x5c709c-0x5c7020=0x7C` (TFightForm1), `0x707c64-0x707be8=0x7C` (form danh sach server), `0x5f3520-0x5f34a4=0x7C` (TAC_FaceSel)... ⇒ **slot +0x90 = `*[0x63B39C+0x90] = *[0x63B42C]`**, va header `007badb0_FUN_007badb0.c:26` xac nhan `0063b42c -> 007badb0 [DATA]`. ⇒ **VMT+0x90 = `FUN_007BADB0`**, khong phai override cuc bo cua TalkMsg.

**Than `FUN_007BADB0`** (asm `007badb0_FUN_007badb0.asm.txt:4–25`): luu `EAX=self, DL, ECX`; `PUSH [EBP+8]` (arg stack dau = con tro chuoi); goi `FUN_007AFBF8` roi `FUN_007AFEF8`; `FUN_007AFEF8` (`007afef8.c:66–76`) duyet chuoi node `node := node[+8]` den node cuoi (tail of list), kiem tra byte `node+0x14` — neu busy thi `RET 0x4` (pop 1 arg) **bo qua thong bao**; duoi cang `0x7AFC1F–0x7AFC24` cua `FUN_007AFBF8` **khong co trong asm dump** (cat ngat) → phan enqueue/format thuc te nam ngoai vung export.
Slot +0x90 nay xuat hien o nhieu widget `TSe_*` cung ham (`0x7af124/0x7af204/0x7af2c8/0x7af3a8/...` trong `007badb0.c:15–35`; `0x7af204 = VMT_7AF174 + 0x90` cua `TSe_FixedButton`) — **API banner chung cua bo widget**.

**Doc nghia theo gia dinh goi (cross-check 20+ call-site)**: luon la `(self, <ptr chuoi literal>, ms, 0, <0|1>)` voi `ms ∈ {1000, 0x4b0=1200, 2000, 3000, 5000, 6000, 10000}` trong dong doi loc/loi/validate UI (`00506bf4.c:28-47`, `00508204.c:36`, `0051f5f4.c:110-125`, `0054790c.c:28-42`, `00552008.c:49-52`...). Mot thuat toan banner day du THUC SU duoc export o **override slot +0xDC** cung class: `*[0x63B478] = FUN_0063C84C` (`0063c84c.c:20`) → wrapper `FUN_007BC1C8(self, msg:string, ms, styleByte, altFlag)` (`007bc1c8.c:44–77`: gan timestamp tu tick counter `**(gvar_007D9D20)`, luu chuoi qua `FUN_007BC998`, `ms<1` → tat auto-hide (+0x5e), nguoc lai luu duration; `altFlag==0` → goi virtual +0x20 de hien/refresh). **Ket luan**: doc "VMT+0x90 = hien banner/thong bao tam thoi" (nhu `opcode_2b.md §1` goi "banner") **PHU HOP toan bo bang chung**; `0x4b0 = 1200ms` cung don vi/cung thang do voi cac call-site anh em — **khong** con la gia thiet thuan tuy. (Do chanh xac cua `+0x90` vs `+0xDC` la 2 entry khac nhau cung ho widget; gi nguyen goi `+0x90` nhu dispatcher ghi.)

**Literal** `&UNK_00799200` / `&UNK_00799224` (cach nhau 0x24): xem §7.

**Doi chieu inline `0078a89c.c:6912,6916`**: cung 1 goi, inline chi hien 2 arg (`..., &UNK_00799200, 0x4b0)`) vs standalone 4 arg (`..., 0x4b0, 0, 0`) — nhieu suy luan tham so cua Ghidra (asm dung `RET 0x4`), ban standalone giu day du hon; thuat toan giong het.

---

## 5. Global & ngu canh song

### 5.1. `gvar_007D9D88` = **TSportManage** (VERIFIED)
- Sinh trong `TForm1.FormCreate`: `piVar6 = TSportManage_Create((int *)VMT_5532F4_TSportManage,1,...); *(int**)gvar_007D9D88 = piVar6;` (`0050a4a0_TForm1.FormCreate.c:629–630`); constructor thuans TObject, khong init field (`005534e4_TSportManage.Create.c:20–42`) → byte `+4` = 0 luc khoi dong (zero-fill).
- **`mgr+4` = ActiveId** — popup dang mo (bang id→form §4.2). Writer cua `+4`: KHONG co trong SSOT export (only-reader) → gan `func_0x0055374c` trong HOLE. Nguoi doc `+4` (ich cho mock — chung "1 form/lan"): guard chuot/phim `005186e4.c:68`, `0050bff8.c:135`, `00566884.c:27`, `005eee60.c:60`, `00611330.c:30`, `00717e78.c:165`, `0073ce00.c:35`, `007c511c.c:29`, va `00777448.c:34`.
- Click/nut cua form#1 duoc router toi TSportManage: `FUN_00547110` (`00547110.c:24`, gan trong bang callback tai `0x547081` ∈ `FUN_00546F8C`) → `FUN_005467D0` (`005467d0.c:24`) → §6.

### 5.2. Bang popup id→form (nguoi dung)

| id | Global | Tick/refresh (id-branch cua `FUN_00553840`/`0055391c`) | Ghi chu khac |
| :--- | :--- | :--- | :--- |
| 1 | `gvar_007DA42C` | `FUN_00548080` (`00548080.c`, may trang thai animation `+0x61`) | Case `0x3A` dispatcher `:6932` bom payload vao `FUN_00547C84`; **goi xin gui 0x39** §6 |
| 2 | `0x5524c4` / `0x552e58` | `00553528.c:21–23`: double-click → `Form2.Close(vmt+4)` |
| 3 | `gvar_007DA778` | `0x541c6c` / `0x541f8c` | **form muc tieu cua SubOp 1 mode==3**; duoi day |
| 4 | `gvar_007D9F98` | `0x54e51c` / `0x54f224` | |
| 6 | `gvar_007DA4EC` | `0x54fbac` / `0x550700` | |
| 0xFF | `gvar_007DA0A0` | `0x5531f4` / `0x5532a4` | `00553528.c:25–27`: double-click → `FormFF.Close(vmt+4)` |

`00553840/0055391c` duoc pump tu vung tick `0x5157E6` (HOLE — handoff). Noi dung 2 ham tick id3: `FUN_00541c6c.c:45–205` — vong quay 8 o × 7 phan tu (`+0x158` dem, `+0x159+i*7` record), dem nguoc `+0x18`, am thanh `"Sound\WA0045.wav"` (`:187`), ve so tai toa do `(0x186,0x122)` (`FUN_0079c150 :194`) — 1 dong chung minh consumer: giong **form quay so/so gao nuoc**. `FUN_00541f8c.c:64–189` — nhan `+(0x84)` tu `gvar_007DA7BC+0x12f8` (player/avatar), giat chuoi `+0x1c`... (chi tiet UI, bo qua).

### 5.3. `gvar_007DA778` (form id3) va field `+0x38`
- 16 reference duy nhat trong SSOT: free/reset (`00553410.c:40–41`), 2 tick (§5.2), **2 write cua chinh OP 0x39** (`case_050.c:54,57`), va 4 method `func_0x00541098/0x005408F4/0x0053FD78/0x0053F8FC` duoc **OP 0x3C** bom payload (dispatcher `:6972–6984`, `case_053_0079575C.c:29–38`) — **ca 4 deu trong HOLE** (kiem tra `index.csv`).
- `+0x38` (int): **chỉ được ghi 100/1000 boi OP 0x39 SubOp 1; KHÔNG có bất kỳ read/export nào** (quét `+ 0x38)` trong cưa số 0x53E000–0x542800 = 0 ket qua). Nghi van "gia tri cua/dat cuoc" cho 2 che do cua form quay so — **UNKNOWN, không khẳng định**.
- Khai sinh object `gvar_007DA778`: không có `*(int **)gvar_007DA778 = ...` trong toan bo export ⇒ sinh lazy trong HOLE (nhiều khả năng chính `func_0x0055374c`).

### 5.4. Global ph
- `gvar_007DA084` = `TSe_TalkMsgFormPlus` (§4.3).
- `gvar_007D9D30` = **TFConnect** — kênh game-server thứ hai theo handoff (`handoff-2026-09-10-protocol-analysis.md`, mục Wire format); duoc truyen lam `param_1` cua `FUN_0077F414` o §6 va duoc Form id1 "tu-inject" message `DL=0x1A` vao dispatcher (`00548080.c:154–160`).
- `gvar_007DA5A0` = object du lieu lon (mang `+0x9a10+i*4`, flag `+0xa096`) — `FUN_00553818` set flag `=1` (§6); sinh trong HOLE.
- `gvar_007D9D20` = con tro tick-counter dung làm timestamp banner (§4.3).

---

## 6. Chieu C→S — case TON TAI nhung RONG (feature bi tat/unfinished)

**a) Builder**: `0077f414_FUN_0077F414.c:768` `switch (param_2 & 0xff)` (gate `*gvar_007DA3A0 != 0` = con ket noi, `:767`; `AddSedQueue` xuat hien 13 nôi dung khác):
```c
case 0x39:
  break;                       // :1020–1021 — KHÔNG build gì, KHÔNG gửi
```
⇒ **Client hien tai khong he gui frame 0x39.** Mock server khong mong đợi packet 0x39.

**b) Duong "dinh gui" bi cut**: `FUN_00553818` (`00553818.c:20–26`; asm `00553818.asm.txt`: `MOV CL,1; MOV DL,0x39; CALL 0x77f414`):
```c
FUN_0077f414(*gvar_007D9D30, 0x39 /*EDX*/);      // → case 0x39: break → rơi vào rỗng
*(byte *)(**gvar_007DA5A0 + 0xa096) = 1;         // dirty/pending flag
```
Hai call-site: (1) chuoi click nut form#1: `FUN_00547110` (`00547110.c:24`, moc bang callback `0x547081`) → `FUN_005467D0` (`005467d0.c:24` `FUN_00553818(mgr, p2)`) → refresh `FUN_00545D0C` + `vmt+0x24`; (2) trong may trang thai form#1 `FUN_00548080` (ref `0x5485cc`, header `00553818.c:9–10`). ⇒ **Đúng ra client GUI tung "gui 0x39 sub 1" (CL=1) khi nguoi dung tác động form id1, nhưng builder rong khiên no thành dead-request + flag.** Không loai tru kha nang ban build khac/server game co doc 0x39 — riêng **binary nay: khong gui**.

---

## 7. Chuoi literal & encoding

- **`0x00799200` va `0x00799224`: CHUA DUMP — khong dich duoc.** `redump/` chi co `lit_5957D8/595800/595810/595828/595844/596078`, `lit_77F771`, `lit_78A854`, `lit_7A2094/7A20A8`, `lit_7ABD54/BDAC/BDF0/BE40/BE64/BE74` — khong vu khi nao phu 0x799xxx.
- Khu vuc nay la **block chuoi cua ho banner**: OP 0x37 cung goi `+0x90` voi `&UNK_007991d8`, `&UNK_007991ec` (2000ms) (`0078a89c.c:6840,6844`); khoang cach 0x20–0x24/entry. **De nghi redump**: tu `0x007991C0` den het `0x00799248` (cat toi null-terminator tung entry) — roI giai theo tien le **cp1258 → NFC** (`opcode_02.md §5`); neu ra van mo co, thu **VISCII** (ngoai le `opcode_13.md`) va ghi bang chung. KHONG bia noi dung.
- Chuoi ASCII thuan da doc: `"Sound\\WA0045.wav"` (`00541c6c.c:187`), `"panel10"/"panel13"` (`0051189c.c:1275`).
- Bang ten debug opcode trong dispatcher (`:583–759`) **khong co case 0x39** (dừng ở `0x38 → 0x796d10`, default `0x796d38` `:592`) → log trace 0x39 in ten mac dinh/default; cac dia chi ten `0x796xxx` cung chua dump.

---

## 8. Ghi chu cho Mock Server

Payload duoi day la **truoc XOR** (frame thuc te: XOR 0xAD toan bo theo handoff §Wire format).

| Muc dich | Bytes payload | Ghi chu |
| :--- | :--- | :--- |
| Mo form id1 | `39 01 01` | id hop le theo bang §5.2: 01/02/03/04/06/FF |
| Mo form id3, mode 1 | `39 01 03 01` | form3`+0x38` = 100 |
| Mo form id3, mode 2 | `39 01 03 02` | form3`+0x38` = 1000; **thieu byte cuoi → BoundErr** |
| Dong form dang mo | `39 02` | neu `ActiveId==0`: no-op an toan |
| Banner 1 (1.2s) | `39 03 01` | chuoi cuc bo `0x799200` — client tu hien thi |
| Banner 2 (1.2s) | `39 03 02` | `0x799224` |

Vi du full frame `39 01 03 02`: `F4 44 | A9 AD | 94 AC AE AF` (Len=04→`04 00`^ADAD=`A9 AD`; payload `39 01 03 02`^AD=`94 AC AE AF`).

1. **Sequencing**: day SubOp 1 truoc de form mo, roi `0x3C...` (case_053) bom noi dung vao form id3; SubOp 2 dong khi xong. Neu mo form id3 ma chua gui `0x39 0x3C` du lieu, client van chay tick voi trang thai `+0x38` gan nhat.
2. Khong gui SubOp khac 1/2/3 (an toan nhung vo nghia); payload rong (`39` don loi) → **BoundErr** — han che.
3. Client **khong bao gio gui 0x39** (§6a) — khong can xu ly o server; neu mo phong server game "that", hay ghi nho co yeu cau `0x39 sub 1` bi missing do case rong.
4. Banner SubOp 3 chi hieu luc khi widget TalkMsg con song (FormCreate den `00603F20`); `node+0x14 busy` → banner bi drop im lang (`007afbf8.asm.txt`).

---

## 9. Source trail + UNKNOWN

**Da doc/kiểm chứng trực tiếp:**
1. `case_functions/functions/case_050_00795579_FUN_00795579.c:8–105` — toan bo handler (+ SEH tail `:78–105` = cleanup chung).
2. `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` — tinh python: `byte[0x39]=50`, entry `0x0078AA7E`, target `0x00795579`; sanity `0x36→47→0x00795494` khop `opcode_36.md`.
3. `functions/0078a89c_FUN_0078a89c.c:6857–6919` (inline case, doi chieu kep; `:6912,6916` vs standalone `:72,75`), `:583–759` (bang ten), `:6840–6844` (banner OP 0x37 cung block literal), `:6920–6984` (case 0x3A/0x3C lien quan form id1/id3).
4. `functions/00553410_FUN_00553410.c:19–55` — dong form; `functions/00553840_FUN_00553840.c:23–52`, `0055391c_FUN_0055391c.c:23–52` — pump tick; `00603f20_FUN_00603f20.c:7,100–150` — out-world free.
5. `functions/0050a4a0_TForm1.FormCreate.c:629–630` + `005534e4_TSportManage.Create.c` — danh tinh `TSportManage` (VMT_5532F4); `00553528.c:18–29` — double-click close.
6. `functions/0051189c_FUN_0051189c.c:1156,1263–1277` — `TSe_TalkMsgFormPlus`/`panel10`/widget-manager; scan DATA-ref toan `functions/` xác lập slot VMT: `+0x7C`=0x7AFA94 (~190 class), `[0x63b42c]=FUN_007BADB0` (`007badb0.c:26`), `[0x63b478]=FUN_0063C84C` (`0063c84c.c:20`); `007badb0.asm.txt:4–25`, `007afbf8.asm.txt` (cat ngat), `007afef8.c:66–76`, `007bc1c8.c:24–77`.
7. `functions/00541c6c_FUN_00541c6c.c:45–205`, `00541f8c.c:64–189` — tick form id3; `case_053_0079575C.c:24–38` — OP 0x3C bom payload vao form id3.
8. `functions/0077f414_FUN_0077F414.c:767–768,1020–1021` — case 0x39 rong; `functions/00553818.c` + `.asm.txt` — dead-send + flag `0xa096`; `005467d0.c`, `00547110.c` — chuoi click; `00548080.c:80–215` — may state form#1.
9. `index.csv` (python): HOLE `0x0055355D–0x00553818` (chua `func_0x0055374c`), `0x00540/0x00541/0x0053F` method form id3, caller tick `0x515C15/0x515D8E`; `redump/` khong ph `0x799xxx`; `00516158.c:86–93` — RP pipeline (tien le).

**UNKNOWN (con lại):**
- Than `func_0x0055374c` = `OpenForm`: branch id 1/2/4/6/0xFF, behavior vs id trung/lap, nghi thức dat `mgr+4` — **redump `0x0055355D–0x00553818`**.
- Noi dung 2 banner `0x799200/0x799224` (va 0x7991d8/0x7991ec cua OP 0x37) — **redump `0x7991C0–0x799248`**, thu cp1258→NFC truoc, VISCII làm phương án hai.
- Consumer cua form3`+0x38` (100/1000): khong co trong export (nghi van nam trong 4 method id3 thuoc OP 0x3C dang HOLE).
- Duoi cang `0x7AFC1F–0x7AFC24` cua `FUN_007AFBF8` (phan enqueue chuoi/thoi gian that cua slot +0x90) — asm dump cat ngat.
- Khai sinh `gvar_007DA778/D9D30/DA5A0` (lazy trong HOLE); y nghia flag `**(gvar_007DA5A0)+0xa096`; gia tri `CL=1` trong dead-send (sub-op ?) — khong ket luan.
