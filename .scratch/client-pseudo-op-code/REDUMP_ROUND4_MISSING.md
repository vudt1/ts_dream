# Danh sách thiếu để redump — Battle / Economy / Guild / Skill / Marriage / Reconnect

Ngày: 2026-10-02 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Manifest: **`/mnt/d/VUDT/GIT_PCC/test/Round4Manifest.txt`** (154 directive, đã validate syntax)
Script: `Round3Redump.java` (đã có sẵn, không sửa)

> **Mọi địa chỉ dưới đây đã được đối chiếu với `test/ts_decompile/index.csv` (13 176 function)
> và với bảng instruction thực tế trong các file `.asm.txt` hiện có.**
> Những mục tài liệu nói là "thiếu" nhưng thực ra **đã có** đã bị loại khỏi danh sách
> (xem §5). Ngược lại, có **một phát hiện lớn** làm thay đổi bức tranh toàn bộ.

---

## 0. ⚠️ Phát hiện quan trọng nhất: `FUN_0077f414` chỉ được disassemble 43%

`FUN_0077f414` là **dispatcher C→S** (hàm `SendCommand`) — mega-function phủ toàn bộ dải
`0x0077F414 .. 0x0078A817`.

| Mục | Giá trị |
|---|---|
| `index.csv` ghi `size_bytes` | `19534` |
| Span thật (đo từ `.asm.txt`) | **`0x0077F414..0x0078A817` = 46 084 B** |
| Byte thật sự được disassemble | **19 534 B (42%)** |
| **Byte chưa bao giờ được disassemble** | **26 550 B (58%) — 15 khoang rời rạc** |

`index.csv` đang ghi sai `size_bytes`, nên khi tra cứu bằng script thì **mọi địa chỉ trong
khoảng `0x00784062 .. 0x0078A89C` đều báo "HOLE" giả** — 26 682 B. Nhưng 15 khoang thật
nhỏ hơn nhiều:

| # | Khoang chưa disassemble | Size | Nội dung là gì | Tài liệu yêu cầu |
|:-:|---|--:|---|---|
| 1 | `0x00783405..0x00785225` | **7 713 B** | C→S `0x17` túi đồ (61 sub) + `0x18`/`0x19` | `economy/README:243`, `04:442` |
| 2 | `0x00781A06..0x00782E8A` | 5 253 B | C→S `0x09`..`0x16` | `opcode_0d/0e/10/13/14/16` |
| 3 | **`0x00787362..0x0078820E`** | **3 757 B** | **C→S `0x27` bang hội (57 builder + bảng)** | `guild_city_siege §2.1` *"lỗ hổng lớn nhất"* |
| 4 | `0x0078125D..0x007819F0` | 1 940 B | C→S `0x13`/`0x14` | |
| 5 | `0x007853E5..0x00785B4B` | 1 895 B | C→S `0x1F`/`0x20` (stall, lotto) | |
| 6 | `0x00780B87..0x00780FA4` | 1 054 B | C→S `0x0C`/`0x0D` | |
| 7 | `0x00786598..0x00786986` | 1 007 B | C→S `0x1E`/`0x1F` | |
| 8 | **`0x00786221..0x00786582`** | **866 B** | **bảng 9 nhánh C→S `0x1E` (storage)** | `economy/05:493` (U9) |
| 9 | **`0x0078048D..0x007807D0`** | **836 B** | **C→S lệnh trận `0x0B` sub `00..04`** | `battle/README:141,329` P0 |
| 10 | `0x0077F883..0x0077FB47` | 709 B | C→S `0x01`/`0x02` | |
| 11 | `0x00785DBB..0x00785FFC` | 578 B | bảng 5 nhánh C→S `0x1C` (học kỹ năng) | `light_dark §2.1` |
| 12 | `0x0078A157..0x0078A317` | 449 B | C→S `0x48`/`0xC7` | |
| 13 | `0x0077F474..0x0077F633` | 448 B | byte-map `0x77F474` + case `0x00` | `reconnect §3.1` |
| 14 | `0x0077F771..0x0077F789` | 25 B | | |
| 15 | **`0x00780440..0x00780453`** | **20 B** | **bảng 5 entry C→S `0x0B`** | `battle/03:499` P0 |

**Hệ quả — 4 tuyên bố "chưa có trong corpus" của tài liệu là SAI:**

| Tuyên bố | Thực tế |
|---|---|
| `guild_city_siege §2.1`: *"body 57 builder `0x27` C→S chưa có trong corpus"* | Có — khoang #3. ASM chỉ ghi `0078734d:` rồi nhảy thẳng `0078820f:` |
| `economy/05:493` (U9): *"bảng `0x00786221` bị Ghidra coi là data, không có `.asm.txt` phía sau"* | Đúng về mặt kỹ thuật — nhưng là vì **chưa disassemble**, không phải vì Ghidra nhầm. Khoang #8 |
| `economy/README:243`: *"`0x17` C→S là lỗ hổng lớn nhất (~7 KB)"* | Đúng — khoang #1 (7 713 B). Nhưng con số "7 KB" ở `README:243` là do đo `0x784000..0x785000` (= 4 096 B); span đúng là `0x783403..0x785225` |
| `battle/README:141`: *"đây là thứ duy nhất cần redump"* | Đúng — khoang #9 + #15 |

> **Một lệnh `ASM 0x0078043C 0x0078A817` sẽ xoá sạch cả 15 khoang.**
> Nhưng **bảng 4-byte / len 2-byte vẫn phải dump `HEX` riêng** — `ASM` không đọc được
> data. Manifest đã tách sẵn `PTRTAB` cho từng bảng.

---

## 1. 🔴 P0 — Bảng dữ liệu bắt buộc (dump `HEX`, ASM không thay được)

| # | Địa chỉ | Kích thước | Vì sao chặn |
|:-:|---|--:|---|
| **1** | **`0x0094B440..0x0094B6A9`** | **765 B** | `TSkillTreeRow[5 tab][51 slot][u16 skillId LE + u8 requireByte]`. **Trả lời dứt điểm "id nào QUANG, id nào ÁM"** — `light_dark §6 #4` gọi đây là *"quan trọng nhất"*. Server **không bao giờ gửi** cây kỹ năng; client tự biết. |
| **2** | **`0x009AE128..0x009AE827`** | ~1 792 B | Bảng `skillId → index` của client. `FUN_00759B04` tra ngược qua đây rồi mới ghi vào `player+0xFD4+idx*3`. **Server gửi id không có trong bảng ⇒ client coi như chưa học** (gap G-06). |
| **3** | `0x00780440..0x0078048C` | 20 + 76 B | Bảng 5 entry C→S `0x0B` sub `00..04`. Xác nhận tập giá trị `TFightManage+0x24 ∈ {2,3,4,5,9}`. |
| **4** | `0x00783405..0x00783628` | 256 + 548 B | Bảng 61 entry C→S `0x17` (đã có `cs_op17_targets_0x783405.hex` nhưng thiếu phần rộng). |
| **5** | `0x00786221..0x00786540` | 24 + 800 B | Bảng 9 nhánh C→S `0x1E`. |
| **6** | `0x00787362..0x00787AB1` | 228 + 2 708 B | Bảng 57 entry C→S `0x27` (`0x00..0x38`). |
| **7** | `0x00785DBB..0x00785E32` | 20 + 88 B | Bảng 5 nhánh C→S `0x1C` (`0x00785DD7/0x00785E45/0x00785EB3/0x00785F21/0x00785F8F`). |
| 8 | `0x007DA6B8..0x007DA8D7` | 544 B | Bảng hệ số level (record 16 B/level) — battle/04:339, quyết định `texps.rs` dùng công thức nào. |
| 9 | `0x007DA6DC..0x007DA880` | 420 B | Bảng 101 entry (guard `≤100`) — `battle/01:710`. |

---

## 2. 🔴 P0 — Function body thật sự không có (không có trong `index.csv`)

| # | Địa chỉ | HOLE chứa nó | Tài liệu yêu cầu |
|:-:|---|---|---|
| **1** | **`0x0076B458`** — handler `17 06` "thêm 1 item vào túi" | `0x0076AF6A..0x0076B84C` (2 274 B) | `battle/04:472` (G4), `economy/03:646` (G9), `economy/01:335`. **Nâng layout `17 06` từ mức B/C lên A** — đây là frame Rust dùng khắp nơi mà chưa từng được verify. |
| **2** | **`0x00644A18`** — `LAB_`, callback trễ sau khi thoát trận | `0x00644A16..0x00644B20` (266 B) | `battle/04:479` (G11) |
| **3** | **`0x0063FADC`** — `TFightField.Create` | `0x0063F9F3..0x00640984` (3 985 B) | `battle/02:737` (#3). Nơi client tự gán `cell.X/cell.Y`. Chặn việc dựng lại lưới bằng toạ độ pixel. |
| **4** | **`0x007311C8`** — handler `17 1F` (stall) | `0x007311AD..0x00731388` (475 B) | `economy/04:111,534` (U14) |
| **5** | **`0x00610290`** — handler `17 21` (mua sạp) | `0x0060FDC1..0x00611330` (5 487 B) | `economy/01:368`, `04:113` |
| **6** | **`0x0060E3CC`** — handler `17 22` | `0x0060E36B..0x0060E62C` (705 B) | `economy/04:115,534` (U14) |
| **7** | `0x00609218` — `LAB_` btn_Trade P2P | `0x006091D8..0x0060936C` (404 B) | `economy/03:684` (Undet.3b). Kỳ vọng `CL=0x03`. |
| **8** | `0x0040402C` — `@LStrFromString` | `0x00403FAF..0x004040F6` (327 B) | **Chốt framing `[main][sub]`** — 3 tài liệu (`marriage §0 #4`, `guild R5`, `economy/04`) đều dựa vào giả định này. |
| **9** | `0x00794977` — case 41, bus hôn nhân `0x2D` | `0x007922AE..0x007994A4` (29 174 B) | `marriage Q10` *"không có ASM"*. 17 sub-op đã đọc từ `.c` nhưng chưa có ASM để đối chiếu. |
| **10** | `0x00795F82` — case 61, hệ con cái `0x45` | (cùng hole) | `marriage Q10` |
| 11 | `0x0078D82F` | `0x0078D14D..0x007902FB` (12 718 B) | `economy/04:532` (U7) — frame hạn sạp 8 B |
| 12 | `0x00772788` / `0x007726F0` | `0x007726F0..0x00772924` (564 B) | `economy/05:485` (U1) — Ghidra **không tạo function** cho caller của `TFRollEgg.Create` |
| 13 | `0x0076AE24`, `0x0076AD78`, `0x0076974C` | 997 B / 5 733 B | `economy/03:646` |
| 14 | `0x0079C08C`, `0x0079C0EC` | `0x0079C04B..0x0079C150` (261 B) | `light_dark §2.1` — điểm vào UI mở cây kỹ năng NPC |
| 15 | `0x005157E6`, `0x005157EE` | `0x00514C62..0x00516108` (5 286 B) | `reconnect §5.2, §10.1` — **call-site của send/recv pump**. Xác nhận có timer heartbeat hay không. |
| 16 | `0x0046C6B0` — `TAbstractSocket.Open` | `0x0046C6A3..0x0046C7B0` (269 B) | `reconnect §5.4` — tìm mọi call-site mở lại socket |
| 17 | `0x0078C45F`, `0x0078CE37` | — | `reconnect §10.2`, `light_dark §2.3` |
| 18 | `0x00796347` — case 0 của `0x0A` | — | `reconnect §5.5` |

---

## 3. 🟠 P1 — ASM cần đọc lại (function đã có body, nhưng byte chưa được phân tích)

| Địa chỉ | Mục đích |
|---|---|
| `0x005D6A9B..0x005D6F7C` (1 249 B) | HOLE chứa `btn_Buy` — `economy/02:604`. Có thể tồn tại call-site MUA thứ 2. |
| `0x005DC98A..0x005DCCDF` (854 B) | HOLE chứa `btn_Sale` — `economy/02:605`. Xác nhận nó gán `+0x160`/`+0x162` rồi gọi `SendCommand(0x1B, 2)`. |
| `0x005D73C0`, `0x005DC500` | **Chốt index base 0-based hay 1-based** cho `1B 01`/`1B 02` (`economy/02:602-603`) |
| `0x0076D578..0x0076F034` | Gate element khi dùng skill — đã đọc 1 179-1 204, còn 6 841 B (`light_dark §6 #8`) |
| `0x0059C089..0x0059C0AB`, `0x0059C61F..0x0059CB2B`, `0x0059D750..0x0059D77F`, `0x0059EFE3..0x0059F08F`, `0x005A02DC..0x005A0387` | Method còn lại của form kỹ năng (`light_dark §6 #7, #10`) |
| `0x0075A948..0x0075AB54` | 8 accessor `TFSkill` — `03_skill_and_combo §2.1` ghi `skill_type` ở `+0x21`, ASM `FUN_0075A9F8` dùng hằng `0x25` |
| `0x00644294`, `0x0064DEF5`, `0x00651E3C..0x00652454`, `0x0065B0DC` | Byte lệch trong record 23B / block unreachable `32 02` / hit record 58 B / x87 `k[]` |
| `0x0066BF0C..0x0066C258` | FX id trong `FUN_0066bb34` — `battle/02:742` |
| `0x007A5010..0x007A51C0` | `0F 08` pet status — `battle/04:471` (G3) |
| `0x0060F89C`, `0x0060CD78` | 2 ctor vendor — `entry[10] stride 0x11` không ai khởi tạo (`economy/04:290-310`) |
| `0x007716DC..0x0077178F` | `TVenderItem.Create` — field `+0x20/24/4C/FC/FD/F0` |

---

## 4. 🟠 P1 — Literal VISCII còn thiếu hex (đọc được sau khi decode)

| Nhóm | Địa chỉ | Tài liệu |
|---|---|---|
| **Cây kỹ năng** | `0x5854E4`, `0x584384`, `0x72A870`, `0x72C665`, `0x5A02DC..0x5A039B` | `light_dark §6 #6, #10` — caption tab: tab 0 là "Quang" hay "Kim" (remap `1→2,2→3,3→0,4→1`) |
| **Con cái** | `0x7501CC`, `0x7501E4`, `0x7501F8`, `0x75020C`, `0x750228`, `0x74FC88`, `0x74F4D0`, `0x74FDE4`, `0x74FE10`, `0x74FE3C`, `0x74FDB8`(tail), `0x5E82F0`, `0x743AA4..0x743B04`, `0x74403C..0x744048`, `0x7988B8` | `marriage §6.2 (G24), §4.2/4.3`. **3 nhãn thuộc tính con chưa có** (`0x7501CC/E4/F8`) |
| **Trận** | `0x644CAC`, `0x644CFC`, `0x644CF0`, `0x644D14` | `battle/01:711` (Gap #9) — `0x644C7C` đã có, 4 cái còn lại thiếu |
| **Cửa hàng** | `0x5D75D0..0x5D769F`, `0x611460..0x6114C0` | `economy/02:608`, `04:534` (U9) |
| **Trao đổi** | `0x75B3A0..0x75B3D0`, `0x75BBEC..0x75BF14`, `0x75AF20` | `economy/03:309,691,692` — `lit_75b1e4_75b3a0.hex` **dừng đúng tại `0x75B3A0`**, 15 chuỗi phía sau chưa decode |
| **Casino** | `0x6091F8`, `0x60B710` | `economy/05:277-278` |
| **Reconnect** | `0x78A818..0x78A8ED`, `0x796D10..0x796D78` | `reconnect §3.2, §10.6-10.7` |

---

## 5. ✅ Đã loại khỏi danh sách — tài liệu nói thiếu nhưng **đã có trên đĩa**

Tôi đã kiểm tra từng địa chỉ. Những cái sau **không cần redump**:

| Tuyên bố | File | Thực tế |
|---|---|---|
| *"11 hàm `0x35` còn lại chưa có body"* | `battle/01:639-642` | Cả **14** đều có `.c` + `.asm.txt`. Tài liệu còn tự đếm sai (13 chứ không phải 11) |
| *"`FUN_00654590` vẫn không có body"* | `battle/03:512` | `00654590_FUN_00654590.c` + `.asm.txt` tồn tại. `battle/02:552` và `README:310` nói đúng — **`03` sai** |
| *"`func_0x0076b458` không có body"* | `battle/04:283` | **Đúng** — nằm trong hole 2 274 B. Giữ trong danh sách |
| *"`0x0060E2A8`, `0x0071E164`, `0x0075C004`, `0x007716DC`, `0x00771728`, `0x00772924` chưa decompile"* | `economy/04,05` | Đều có body. Chỉ cần ASM đọc chi tiết |
| *"13 file `functions/*.c` thiếu trong checkout"* | `economy/02:29,638` | `test/ts_decompile/functions/` có **13 176** file — đủ hết. Tác giả tài liệu đọc bên workspace khác |

**Sai lệch địa chỉ cần sửa trong tài liệu:**

| Tài liệu | Ghi | `index.csv` thật |
|---|---|---|
| `economy/02:281,604` | HOLE `0x5DC96A..0x5DCE0` | HOLE `0x5DC98A..0x5DCCDF` (`FUN_005DC500` kết thúc `0x5DC98A`, `FUN_005DCCE0` bắt đầu `0x5DCCE0`) |
| `economy/README:243` | `0x784000..0x785000` ≈ "7 KB" | span đó chỉ **4 096 B**; 7 713 B là `0x783405..0x785225` |
| `battle/03:127` | bảng dispatch `0x780440` "chưa có byte" | Đúng — 20 B data, đã thêm `HEX` + `PTRTAB` |

---

## 6. ⚪ Không giải được bằng redump (cần capture traffic thật)

| # | Câu hỏi | Nguồn |
|:-:|---|---|
| C1 | Byte `kind` đầu record `0B FA` cho self: `04` hay `05`/`03`/`64`? | `battle/README:320` |
| C2 | Hồi sinh trong trận: `33 01` hay `0B 08`? | `battle/01:707` |
| C3 | Layout `0B` sub 2 + nội dung AnsiString `1C` sub 1 | `battle/03:115` |
| C4 | **Thứ tự byte record `1C 01`: `[level][id]` hay `[id][level]`** — client ASM vs Bear C# **ngược chiều** | `light_dark §3 G-05` |
| C5 | `player+0x1450` là gì (index quà `46001..46005`?) | `marriage Q1` |
| C6 | `+0x151D == 2` là **sống** hay **chết**? (mâu thuẫn nội tại client) | `marriage Q6` |
| C7 | `17 20` có thật sự 1 byte pad? sub-op thật của stall? | `economy/04:484-486` |
| C8 | Thứ tự `1A 01` và `1A 04` khi bán | `economy/02:610` |
| C9 | `ClientSocket1Error` — không có handler trong dump | `reconnect §10.4` |
| C10 | Có NPC hôn nhân không; token `Đời 2 võ tướng` là gì | `marriage Q4, Q5` |

---

## 7. Cách chạy

```bash
# Script Manager (interactive): chọn Round3Redump.java, 2 lần Enter
# hoặc headless:
analyzeHeadless <projDir> TS_Ghidra -process aLogin.exe -noanalysis \
  -scriptPath /mnt/d/VUDT/GIT_PCC/test \
  -postScript Round3Redump.java \
  /mnt/d/VUDT/GIT_PCC/test/Round4Manifest.txt \
  /mnt/d/VUDT/GIT_PCC/test/ts_decompile

# Kết quả
cat /mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump/round3_report.txt | grep -E "ERROR|FAIL|NO_FUNCTION"
```

**Thứ tự nên làm:**

1. **Chạy T1 trước** (37 `ASM` trong `FUN_0077f414`) — xoá 15 khoang 26 550 B, mở khoá `0B`/`0x17`/`0x1E`/`0x27`/`0x1C` C→S cùng lúc.
2. **T2** (`PTRTAB` 7 bảng) — giải 57 entry `0x27`, 61 entry `0x17`, 9 nhánh `0x1E`, 5 nhánh `0x1C`, 5 entry `0x0B`.
3. **T3** (13 `FUNC` trong hole) — `0x7 6B458` và `0x63FADC` là 2 cái chặn nhiều nhất.
4. **T4/T5** — bổ sung.
5. Đọc `round3_report.txt`: các dòng `NO_FUNCTION` là address nằm trong vùng cần **Disassemble + Create Function thủ công** trong CodeBrowser rồi chạy lại `FUNC` cho riêng nó.

---

*Nguồn: 6 tài liệu `battle/*.md`, 9 tài liệu `economy/*.md`, và 4 tài liệu `.md` ở
`.scratch/client-pseudo-op-code/`. Mọi địa chỉ đã verify bằng `index.csv` (13 176 function)
và bằng bảng instruction thực tế trong `functions/*.asm.txt`. Ngày 2026-10-02.*
