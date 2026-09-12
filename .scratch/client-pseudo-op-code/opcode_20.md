# PHÂN TÍCH — Main OP 0x20 (32) / Case 28 / `FUN_0079285A` @ `0x0079285A`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Hai callee `func_0x006546E0` / `func_0x0072F534` chưa có body — ghi rõ giới hạn.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Handler nhỏ nhất trong cụm (chỉ 2 nhánh, 0 literal chuỗi, 0 decode ở tầng case). Tầng case chỉ đọc 1 byte SubOp rồi **passthrough nguyên RestPayload** cho hàm con.
- SubOp `0x01`: rẽ 3 đường theo cờ 1 byte `player+0x376` (`**gvar_007DA7BC`): `==0` → `FUN_0072C64C(mgr_007D9D34, RP)` (đã phục hồi: set 1 byte `v` cho slot tìm theo `id`); `!=0 && !=4` → `func_0x006546E0(mgr_007DA51C, RP)` (chưa phục hồi); `==4` → không làm gì.
- SubOp `0x02`: `func_0x0072F534(mgr_007D9D34, RP)` (chưa phục hồi).
- Không banner, không sound, không hiệu ứng ở bất kỳ tầng nào đã phục hồi. Thuần state.
- Chiều C→S `case 0x20: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x20 (32) → byte_table[0x78A8EE][0x20] = 0x1C (28)
                 → dword_table[0x78A9B6][28] @ 0x0078AA26 = 0x0079285A
                 → FUN_0079285a (Case 28)
```

- File chính: `ts_decompile/case_functions/functions/case_028_0079285A_FUN_0079285a.c` (69 dòng)
- Bản inline đối chiếu: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5179-5203` (`case 0x20:`) — khớp 1:1.
- Dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt:27-31` (bản `.asm.txt` chỉ còn prologue; logic đầy đủ ở bản `.c`).
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x20`), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`).
- Delphi `_LStrCopy(s, Index, Count)` 1-based: `_LStrCopy(RP,2,4)` → `RP[1..4]` = `P[2..5]`.

### 2.3. Đọc SubOp (dòng 20-37)

```c
if (*(RP-4) == 0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);   // RP[0] = P[1], 1 byte thường, không codec
if (SubOp == 1) {...} else if (SubOp == 2) {...}
// Không switch/default → SubOp 0x00 hoặc >=0x03 = no-op (chỉ epilogue)
```

- Khối `_LStrArrayClr/_LStrClr` cuối hàm là epilogue chung dispatcher, không phải nghiệp vụ.
- Tầng case **không gọi helper codec nào** trong 5 helper — callee tự decode.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer tầng case | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[20][01][data...]` (≥2B, callee parse) | `SubOp = RP[0]`, passthrough nguyên `RP` | Gate theo `player+0x376`: 0 → đường A đã biết; 4 → no-op; còn lại → đường B chưa biết |
| `0x02` | `[20][02][data...]` (≥2B) | như trên | `func_0x0072F534(mgr, RP)` — chưa phục hồi |
| `0x00`, `≥0x03` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation — handler này vốn không có)

### 4.1. SubOp `0x01` — Update slot có điều kiện theo `player+0x376`

- **Wire**: `P[0]=0x20, P[1]=0x01, P[2..]=data` (format do callee quyết).
- **Đọc**: 1 byte `RP[0]==1`, passthrough.
- **Xử lý**:
  ```c
  if (*(char*)(player + 0x376) == 0)
    FUN_0072C64C(*gvar_007D9D34, RP);        // đường A
  else if (*(char*)(player + 0x376) != 4)
    func_0x006546E0(*gvar_007DA51C, RP);     // đường B (chưa phục hồi)
  // ==4 → không làm gì
  ```

#### 4.1.1. Đường A — `FUN_0072C64C` (đã phục hồi, `0072c64c_FUN_0072c64c.c:50-66`)

Nhận `(mgr, RP)` với RP vẫn gồm cả byte SubOp:

1. `_LStrCopy(RP,2,4)` → `FUN_0077ef7c` decode **DWORD LE** `id = P[2..5]` (bỏ byte SubOp nhờ Index=2).
2. `idx = FUN_0070C20C(mgr_007D9D34, id)`: duyệt tuyến tính `gvar_007DA300[1..count]` (`count = *(mgr+0x60)`, trần 800), khớp khi `*(slot+0x4)==id`; không thấy → trả 0.
3. `if (idx==0) return` — id lạ bỏ qua, không crash.
4. `v = RP[5]` (Delphi index 6, guard `len(RP)>=6` nếu không `_BoundErr(5)`) — 1 byte tại `P[6]`.
5. `FUN_0072A7A8(*(gvar_007DA300+idx*4), v)` ghi slot:
   ```c
   *(slot+0x3d8) = v; *(slot+0x3d9) = 0; *(slot+0x3da) = 1;
   *(double*)(slot+0x3df) = Now();
   *(slot+0x3db) = FUN_007C9B38(..., PREFIX_0x72A870 + IntToStr(v));
   *(slot+999) = 0;
   ```
   Thuần state (byte + cờ + timestamp + handle). Không banner/sound/light.

- **Wire thực tế đường A**: tối thiểu 7 byte payload `[20][01][id:4B LE][v:1B]` (`len(RP)>=6`). Thừa byte bị bỏ qua.

#### 4.1.2. Đường B — `func_0x006546E0(*gvar_007DA51C, RP)` — chưa phục hồi

- Không entry trong `index.csv`, không file `006546e0*`; grep toàn cây chỉ 1 điểm gọi. Cùng khuôn passthrough như đường A. Nội dung parse nằm trong hàm chưa phục hồi.

### 4.2. SubOp `0x02` — `func_0x0072F534(*gvar_007D9D34, RP)` — chưa phục hồi

- **Wire**: `[20][02][data...]`, đọc 1 byte `RP[0]==2`, passthrough, không guard thêm ở tầng case.
- Không entry trong `index.csv`, không file decompile. Cùng manager `007D9D34` với đường A.

---

## 5. Chuỗi VISCII → UTF-8

- **Tầng Case 28: không có literal nào** — chỉ so sánh 2 byte, không có gì để decode.
- **Tầng `FUN_0072A7A8`**: có 1 literal prefix `@LAB_0072a870` (`prefix + IntToStr(v)`). `redump/` hiện không có dump cho `0x0072A870` → chưa decode được. Cần redump `.rodata` tại đó.
- Không có payload text trên dây ở nhánh đã phục hồi (mọi field là số: DWORD id + 1 byte v).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:960-961`: `case 0x20: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x20 S→C thuần. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
S→C [20][01][id u32LE][v u8]  ; đường A khi player+0x376==0: set slot(id).v
S→C [20][01][...]             ; player+0x376!=0,!=4 → manager khác (chưa đặc tả)
S→C [20][02][...]             ; chưa đặc tả — KHÔNG NÊN gửi
C→S [20]: KHÔNG TỒN TẠI
```

1. Chỉ mock được SubOp 01 đường A với format chắc chắn (7 byte). `id` phải khớp slot đang tồn tại nếu không bị bỏ qua im lặng.
2. `len(RP)>=6` bắt buộc cho đường A; ngắn hơn → RangeError trong `FUN_0072C64C`.
3. Khi test đường A đảm bảo `player+0x376==0` (`4` → no-op, còn lại → đường B chưa biết).
4. Quan sát: `slot+0x3D8` (byte v), `+0x3D9/0x3DA` (cờ 0/1), `+0x3DF` (timestamp).
5. Không gửi SubOp `0x00/≥0x03`, không gửi frame `L=1` (`_BoundErr(0)`).

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_028_0079285A_FUN_0079285a.c` | Handler chính, gate `+0x376`, 2 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5179-5203` | Bản inline đối chiếu 1:1 |
| 3 | `functions/0072c64c_FUN_0072c64c.c:50-66` | Decode đường A (`_LStrCopy(RP,2,4)` + `EF7C` + `RP[5]`) |
| 4 | `functions/0070c20c_FUN_0070c20c.c:121-150` | Tìm slot tuyến tính, trần 800 |
| 5 | `functions/0072a7a8_FUN_0072a7a8.c:52-66` | Ghi slot `+0x3D8/0x3D9/0x3DA/0x3DB/0x3DF/999` |
| 6 | `functions/0077f414_FUN_0077F414.c:960-961` | C→S rỗng |
| 7 | `index.csv` (grep `0072f534|006546e0` = 0 kết quả) | Giới hạn: 2 callee chưa phục hồi |
