# NGHIÊN CỨU CHUYÊN SÂU: CƠ CHẾ HỢP KÍCH (COMBO ENGINE) TRONG BATTLE TS ONLINE

- **Vai trò**: Lead Game Designer & Battle Engine Technical Architect
- **Dự án**: `ts_dream` (Rust game server cho TS Online PC `aLogin.exe`)
- **Tập tin tài liệu**: `.scratch/battle-refactor/research-combo-rules.md`
- **Mục tiêu**: Phân tích toàn diện hiện trạng mã nguồn, đối chiếu Clean-room Reverse Spec & Gameplay Nguyên bản TS Online, vạch trần 6 lỗ hổng chí mạng và đề xuất giải thuật chuẩn hóa Battle Combo Engine.

---

## 1. TỔNG QUAN KIẾN TRÚC & TẦM QUAN TRỌNG CỦA HỢP KÍCH (COMBO)

Trong hệ thống chiến đấu theo lượt (Turn-based Combat) của **TS Online**, cơ chế **Hợp kích (Combo)** là trụ cột thiết kế quan trọng nhất chi phối toàn bộ meta gameplay:
1. **Tối ưu hóa hỏa lực (Damage Multiplier)**: Cho phép 2 hoặc nhiều đấu sĩ (Người chơi + Võ tướng / Pet hoặc đồng đội) dồn sát thương vào cùng một mục tiêu trong cùng một nhịp xuất chiêu với hệ số nhân dồn $\times 1.3$ cho mỗi mắt xích.
2. **Quyết định hiệu suất Auto Train / Botting**: Là chìa khóa để dọn sạch bãi quái trong 1 hiệp (Turn 1 Clean-up), triệt tiêu hoàn toàn khả năng quái phản đòn, bảo toàn tối đa độ bền trang bị (Ngọc Phúc Thần, Bùa hộ mệnh).
3. **Khuếch đại Phần thưởng (EXP & Drop Rewards)**: Khi quái bị tiêu diệt bởi đòn hợp kích, toàn bộ thành viên tham gia chuỗi combo đều nhận được lượng kinh nghiệm cộng thưởng ($+8.6\%$) và tăng cơ hội nhận vật phẩm rơi (Drop).

### Kiến Trúc Client - Server Đối Với Cơ Chế Combo
Theo kết luận đảo ngược mã máy client `aLogin.exe` (tại `.scratch/client-pseudo-op-code/battle/03_skill_and_combo.md` §3):
> **COMBO LÀ QUYẾT ĐỊNH 100% PHÍA SERVER.**
> Client hoàn toàn thụ động, không có bảng điều kiện combo, không giữ state combo. Client chỉ nhận các mã diễn hoạt giả `20006` (Hộ chủ / Combo lead), `20007` (Chân combo / Combo footer) và chuỗi các nhịp đánh trong gói `0x33 Sub 01` (hoặc `0x32 Sub 01`) để vẽ hoạt ảnh bay vào đánh hội đồng.

Do đó, **toàn bộ tính toán xác suất, kiểm tra điều kiện tham gia, ghép chuỗi liên kích và nhân dồn sát thương đều nằm trọn vẹn tại Game Server (`src/battle/runner.rs`, `src/battle/damage.rs`)**.

---

## 2. ĐỐI CHIẾU 3 QUY TẮC VÀNG VÀ 6 SAI LỆCH TRỌNG YẾU HIỆN TẠI

Đối chiếu giữa **3 Quy tắc vàng của Hợp kích TS Online** (`.scratch/battle-refactor/combo_rule.md`) và mã nguồn hiện tại của `ts_dream` bộc lộ **6 sai lệch và lỗ hổng chí mạng**:

| # | Khía cạnh Gameplay | Đặc tả Chuẩn TS Online | Hiện trạng Code `ts_dream` | Mức độ Nghiêm trọng |
|---|---|---|---|:---:|
| **1** | **Ngưỡng Agi Gap** | Chênh lệch Agi giữa đấu sĩ cao nhất và thấp nhất trong toàn bộ chuỗi combo **$\le 11$ điểm** (`max_agi - min_agi <= 11`). | Kiểm tra `diff <= 800` (`runner.rs:1271 & 1290`), dùng công thức trung bình `mid` sai lệch toán học. | 🔴 **CRITICAL** (Phá vỡ meta cân chỉnh Agi bằng Tạ chì) |
| **2** | **Xác suất Kích hoạt** | Phụ thuộc vào $\text{LVTB Team} - \text{Level Quái}$. Đạt $100\%$ khi chênh lệch $\ge 25$. Giảm dần theo tỷ lệ phần trăm khi $< 25$. | `get_random_miss_combo` hardcoded `100%` vô điều kiện (`damage.rs:80`). Không nhận tham số level. | 🔴 **CRITICAL** (Bỏ qua cơ chế ép cấp train quái) |
| **3** | **LVTB Nhân vật Trùng Sinh** | Nhân vật đã Trùng Sinh (`reborn > 0`) **bắt buộc tính là Level 200** trong công thức LVTB, bất kể level hiển thị. | Lấy trực tiếp `c.lv` hiển thị (`runner.rs:431 & 434`), bỏ qua hoàn toàn trường `reborn`. | 🔴 **CRITICAL** (Tính sai toàn bộ LVTB của acc Trùng Sinh) |
| **4** | **Phân loại Kỹ năng Combo** | Chỉ đòn đánh thường (`10000`) và kỹ năng **Active** (`skill_type == 1`) mới được combo. Cấm phép/buff/debuff/hồi máu. | Đọc nhầm `s.kind` (`pc.type_` từ offset 21) thay vì `s.attribute` (`pc.skill_type` offset 33). Chỉ kỹ năng hệ Địa mới được combo! | 🔴 **CRITICAL** (Kỹ năng Active hệ Hỏa/Phong/Thủy bị từ chối combo) |
| **5** | **Đếm Mục tiêu AoE** | Chỉ kỹ năng $\le 3$ mục tiêu (`nbTarget <= 3`) mới được combo. Kỹ năng AoE $\ge 4$ mục tiêu bị cấm tuyệt đối. | `skill_target_count` đọc `def.fight_area` (`pc.unk4` offset 31, vốn luôn bằng 0). Kỹ năng AoE 4, 5, 7 mục tiêu lọt lưới toàn bộ. | 🔴 **CRITICAL** (Loạn kích, Phi sa, Hồng thủy vẫn kích hoạt combo vô tội vạ) |
| **6** | **Sát thương & EXP/Drop** | Mỗi mắt xích combo nhân dồn sát thương $\times 1.3$. Tiêu diệt quái bằng combo thưởng $+8.6\%$ EXP và chia drop. | Biến `num37` bị `let _ = (...)` discard (`runner.rs:1446`). `calc_combo_exp` bị bỏ xó thành dead-code. | 🟡 **HIGH** (Combo không tăng sát thương, không tăng EXP) |

---

## 3. PHÂN TÍCH KỸ THUẬT CHI TIẾT TỪNG LỖ HỔNG

### 3.1. Lỗ Hổng 1: Sai Ngưỡng Agi Gap & Giải Thuật Mở Rộng Chuỗi Sai Lệch Toán Học

#### Trích dẫn mã nguồn hiện tại (`src/battle/runner.rs:1269-1308`):
```rust
if ts.combo_hi == 0 {
    let diff = (next.agi - attacker.agi).abs();
    if diff <= 800 { // <-- LỖ HỔNG: Con số 800 tùy tiện
        if next.agi >= attacker.agi {
            ts.combo_hi = next.agi;
            ts.combo_lo = attacker.agi;
        } else {
            ts.combo_hi = attacker.agi;
            ts.combo_lo = next.agi;
        }
        ...
    }
} else {
    let mid = (ts.combo_lo + ts.combo_hi) / 2;
    if (next.agi - mid).abs() <= 800 { // <-- LỖ HỔNG: So sánh với mid sai toán học
        if next.agi < ts.combo_lo {
            ts.combo_lo = next.agi;
        } else if next.agi > ts.combo_hi {
            ts.combo_hi = next.agi;
        }
        ...
    }
}
```

#### Phân tích sai lệch:
1. **Ngưỡng 800 vô lý**: Trong TS Online, khoảng cách Agi giữa thành viên nhanh nhất và chậm nhất tham gia đòn đánh **không được vượt quá 11 điểm**. Ngưỡng `800` khiến cho đấu sĩ Agi 10 và đấu sĩ Agi 800 vẫn combo được với nhau, phá vỡ hoàn toàn cơ chế gameplay cân chỉnh tốc độ (dùng Tạ chì $-10, -20, -30, -40$ Agi cho pet hoặc tháo trang bị để set Agi).
2. **Sai lầm toán học khi dùng `mid`**:
   - Giả sử đã có 2 đấu sĩ mở chuỗi: Đấu sĩ A ($Agi = 100$) và Đấu sĩ B ($Agi = 111$). Khoảng cách hiện tại: $111 - 100 = 11$ (vừa chạm ngưỡng tối đa).
   - Giá trị `mid = (100 + 111) / 2 = 105`.
   - Đấu sĩ thứ 3 (C) có $Agi = 115$.
   - Nếu so với `mid`: $|115 - 105| = 10 \le 11$. Code hiện tại sẽ chấp nhận C vào chuỗi combo!
   - Sau khi chấp nhận C, `ts.combo_hi` được nâng lên $115$.
   - Khi đó, khoảng cách Agi thực tế giữa thành viên nhanh nhất ($C = 115$) và chậm nhất ($A = 100$) là $115 - 100 = 15 > 11$! Quy tắc vàng bị phá vỡ!

#### Giải pháp chuẩn mực:
Khi kiểm tra một ứng viên mới `next` gia nhập vào chuỗi combo đã có (`ts.combo_lo`, `ts.combo_hi`):
$$\text{new\_lo} = \min(\text{ts.combo\_lo}, \text{next.agi})$$
$$\text{new\_hi} = \max(\text{ts.combo\_hi}, \text{next.agi})$$
$$\text{Điều kiện gia nhập}: \text{new\_hi} - \text{new\_lo} \le 11$$

---

### 3.2. Lỗ Hổng 2: Tỷ Lệ Hợp Kích Hardcoded 100% Thay Vì Phụ Thuộc Vào Ngưỡng LVTB

#### Trích dẫn mã nguồn hiện tại (`src/battle/damage.rs:79-82`):
```rust
/// Combo roll — always hits (percent 100) => returns 1.
pub fn get_random_miss_combo(rng: &mut DotNetRandom) -> i64 {
    randomize_with_percent(rng, 1, 0, 100)
}
```

#### Phân tích sai lệch:
Code server hiện tại hardcode xác suất $100\%$ vô điều kiện. Bất kỳ ai đủ điều kiện Agi và mục tiêu đều combo thành công 100%.
Trong khi đó, nguyên lý cốt lõi của gameplay TS Online:
- **Ngưỡng an toàn tuyệt đối ($100\%$ Combo)**: Cấp độ trung bình của đội (LVTB Team) phải cao hơn cấp độ của Quái đang đánh tối thiểu 25 cấp:
  $$\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} \ge 25 \implies \text{Tỷ lệ Combo} = 100\%$$
- **Khi chưa đủ 25 cấp chênh lệch ($\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} < 25$)**: Tỷ lệ combo bị sụt giảm theo tỷ lệ thuận với khoảng cách cấp độ.
  Cộng đồng TS Online kinh điển và các bản dịch ngược tiếng Trung/Đài Loan xác định công thức xác suất combo:
  $$\text{Combo Rate (\%)} = \text{clamp}\!\left( \left\lfloor \frac{\text{LVTB}_{\text{team}} - 25}{\text{Level}_{\text{quái}}} \times 100\% \right\rfloor, 0, 100 \right)$$
  - *Ví dụ 1*: Quái level 25. Team có LVTB = 50. Hiệu số $50 - 25 = 25 \ge 25 \implies 100\%$ combo.
  - *Ví dụ 2*: Quái level 50. Team có LVTB = 60. Hiệu số $60 - 25 = 35 < 50$. Tỷ lệ combo $= \frac{60 - 25}{50} = \frac{35}{50} = 70\%$.
  - *Ví dụ 3*: Quái level 30. Team có LVTB = 25. Hiệu số $25 - 25 = 0 \implies 0\%$ combo (không thể hợp kích khi LVTB $\le 25$).

#### Giải pháp chuẩn mực:
Tái cấu trúc hàm `get_random_miss_combo`:
```rust
pub fn get_random_miss_combo(
    rng: &mut DotNetRandom,
    lvtb_team: f64,
    target_lv: i64,
) -> i64 {
    if target_lv <= 0 {
        return 1;
    }
    let diff = lvtb_team - 25.0;
    if diff <= 0.0 {
        return 0;
    }
    let percent = ((diff / target_lv as f64) * 100.0).floor() as i64;
    randomize_with_percent(rng, 1, 0, percent.clamp(0, 100))
}
```

---

### 3.3. Lỗ Hổng 3: Bỏ Sót Quy Tắc Level 200 Cho Nhân Vật Trùng Sinh (Reborn)

#### Trích dẫn mã nguồn hiện tại (`src/battle/runner.rs:428-437`):
```rust
// Average levels — only while the average is still 0.0 (first turn).
if ts.avg1.is_none() {
    if team == 1 {
        sum1 += c.lv; // <-- LỖ HỔNG: Lấy trực tiếp c.lv
        count1 += 1;
    } else if team == 2 {
        sum2 += c.lv;
        count2 += 1;
    }
}
```

#### Phân tích sai lệch:
- Khi nhân vật Trùng Sinh (Reborn lần 1 hoặc Tái sinh lần 2), level hiển thị sẽ quay về Level 1 (hoặc level thấp tương ứng).
- Nếu lấy trực tiếp `c.lv`, một đội hình gồm 5 nhân vật Trùng Sinh cấp 30 sẽ có LVTB là 30, dẫn tới việc đánh quái level 10 cũng không thể đạt 100% combo!
- **Quy tắc bất biến của TS Online (`combo_rule.md` §2)**:
  > *"Đối với các nhân vật đã Trùng Sinh, hệ thống game sẽ **mặc định tính là Level 200** khi đưa vào công thức tính LVTB, bất kể level hiển thị thực tế của nhân vật đó đang là bao nhiêu. Nhân vật chưa Trùng Sinh vẫn tính theo level hiện tại."*
- Trường `reborn: i64` đã có sẵn trên struct `WarInfo` (`src/battle/engine.rs:53`), nhưng hoàn toàn bị lãng quên khi tính `avg1`/`avg2`.

#### Công thức toán học chuẩn:
Với mỗi đấu sĩ $i$ trong đội hình ($N$ thành viên gồm cả nhân vật và pet trên sân):
$$\text{Level\_Hiệu\_Dụng}_i = \begin{cases} 200 & \text{nếu } \text{Reborn}_i > 0 \\ \text{Level}_i & \text{nếu } \text{Reborn}_i = 0 \end{cases}$$
$$\text{LVTB}_{\text{team}} = \frac{\sum_{i=1}^{N} \text{Level\_Hiệu\_Dụng}_i}{N}$$

---

### 3.4. Lỗ Hổng 4: Nhầm Lẫn Nghiêm Trọng Giữa Nguyên Tố (`pc.type_`) và Loại Kỹ Năng (`pc.skill_type`)

#### Trích dẫn mã nguồn hiện tại:
1. `src/data/loaders/skill_pc.rs:301-305`:
   ```rust
   pub fn pc_to_binary(pc: &PcSkillDef) -> BinarySkillDef {
       ...
       BinarySkillDef {
           id: pc.id,
           name: pc.name.clone(),
           kind: pc.type_,           // <-- Offset 21 (Loại nguyên tố/ngũ hành)
           ...
           attribute: pc.skill_type, // <-- Offset 33 (Loại chiêu thức: Active/Buff/Debuff)
           ...
       }
   }
   ```
2. `src/battle/runner.rs:886-895` & `1246-1254`:
   ```rust
   let skill_type = skill_def
       .map(|s| i64::from(s.kind)) // <-- LỖ HỔNG: Đọc s.kind (offset 21)
       .or_else(|| skill_row.map(|s| s.skill_type))
       ...
   
   let next_type = data
       .skills
       .get(&next.id_skill)
       .map(|s| s.skill_type)
       .unwrap_or_else(|| {
           binary_skill_def(data, next.id_skill)
               .map(|s| s.kind as i64) // <-- LỖ HỔNG: Lại đọc s.kind thay vì s.attribute!
               .unwrap_or(if next.id_skill == 10000 || next.id_skill == 0 { 1 } else { 0 })
       });
   
   if next_type == 1 ... // <-- Kiểm tra điều kiện "Active/Vật lý"
   ```

#### Phân tích dữ liệu thực tế từ `Data/Skill.Dat` (32,422 bytes, 377 records):
Khi giải mã trực tiếp từ file nhị phân `Data/Skill.Dat`:
- **Offset 21 (`type_`)**: Định danh Hệ/Thuộc tính nguyên tố:
  - `0`: Vô hệ / Vật lý thường (Đấu vật `10000`)
  - `1`: Hệ Địa (Thuật mưa đá `10001`, Ném đá `10005`, Phi sa `10008`)
  - `2`: Hệ Thủy (Hồng thủy `11008`)
  - `3`: Hệ Hỏa (Hỏa tiễn `12003`)
  - `4`: Hệ Phong (Loạn kích `13013`)
  - `5`: Hệ Tâm (Hộ chủ `20006`)
- **Offset 33 (`skill_type`)**: Định danh Loại kỹ năng (khớp 100% với `skillType` trong `Data/skills.json`):
  - `1`: **Active** (Kỹ năng tấn công chủ động có thể Combo: Đấu vật `10000`, Ném đá `10005`, Hỏa tiễn `12003`, Loạn kích `13013`)
  - `2`: **Buff / Phép** (Mưa đá `10001`, Phi sa `10008`, Hồng thủy `11008`, Thiên băng `10012`)
  - `3`: **Debuff** (Cây tinh `10004`, Băng phong `11014`, Hỗn loạn `14021`)
  - `4`: **Heal** (Kết giới `10010`, Thanh lưu, Toàn hồi ma pháp)
  - `17`: **Special / Guard** (Hộ chủ `20006`, Giải hộ chủ `20007`)

#### Hậu quả chí mạng:
Vì `runner.rs` đọc `s.kind` (chính là offset 21 `type_`) rồi so sánh `if next_type == 1`:
- Chỉ có kỹ năng thuộc **Hệ Địa** (`type_ == 1`) mới có `s.kind == 1`!
- Kỹ năng Active của **Hệ Hỏa** (ví dụ Hỏa tiễn `12003` có `type_ = 3`) có `s.kind = 3 != 1` $\implies$ **Bị từ chối Combo hoàn toàn**!
- Kỹ năng Active của **Hệ Phong** (ví dụ Loạn kích `13013` có `type_ = 4`) có `s.kind = 4 != 1` $\implies$ Bị từ chối Combo!
- Đòn đánh thường `10000` có `type_ = 0` $\implies$ Nếu có trong `BinarySkillDef` thì `s.kind = 0 != 1` $\implies$ Bị từ chối Combo!

#### Giải pháp chuẩn mực:
Loại kỹ năng Active thực sự được lưu tại trường `attribute` của `BinarySkillDef` (từ `pc.skill_type` offset 33).
Hàm trích xuất `skill_type` trong `runner.rs` phải đọc:
```rust
let skill_type = skill_def
    .map(|s| i64::from(s.attribute)) // <-- ĐỌC s.attribute (offset 33 pc.skill_type)
    .or_else(|| skill_row.map(|s| s.skill_type))
    .unwrap_or(if skill == 10000 || skill == 0 { 1 } else { 0 });
```

---

### 3.5. Lỗ Hổng 5: Đếm Mục Tiêu AoE Đọc Nhầm Trường Dữ Liệu Luôn Bằng 0

#### Trích dẫn mã nguồn hiện tại:
1. `src/data/loaders/skill_pc.rs:307-308`:
   ```rust
   fight_way: pc.nb_target, // <-- Offset 34 (Số mục tiêu thực tế)
   fight_area: pc.unk4,     // <-- Offset 31 (Trường chưa rõ, luôn bằng 0)
   ```
2. `src/battle/runner.rs:137-148`:
   ```rust
   pub fn skill_target_count(data: &BattleData<'_>, skill_id: i64) -> u8 {
       if skill_id == 10000 || skill_id == 0 {
           return 1;
       }
       if let Some(def) = binary_skill_def(data, skill_id) {
           return def.fight_area; // <-- LỖ HỔNG: Đọc fight_area (vốn mang pc.unk4 = 0)
       }
       if let Some(s) = data.skills.get(&skill_id) {
           return s.sl_danh.max(1) as u8;
       }
       1
   }
   ```

#### Bằng chứng kiểm tra thực tế trên `Data/Skill.Dat`:
| Skill ID | Tên Kỹ năng | `nb_target` (Offset 34) | `unk4` (Offset 31) | `fight_area` trong Rust | `skill_target_count` thực tế trả về | Kết luận |
|:---:|:---|:---:|:---:|:---:|:---:|:---:|
| `10000` | Đấu vật | **1** | 0 | 0 | 1 (do nhánh riêng) | Khớp |
| `10005` | Ném đá | **3** | 0 | 0 | **0** | ❌ Sai (thực tế 3) |
| `13013` | Loạn kích | **4** | 0 | 0 | **0** | ❌ **LỌT LƯỚI** (thực tế 4) |
| `10008` | Phi sa tẩu thạch | **5** | 0 | 0 | **0** | ❌ **LỌT LƯỚI** (thực tế 5) |
| `11008` | Hồng thủy | **7** | 0 | 0 | **0** | ❌ **LỌT LƯỚI** (thực tế 7) |
| `10012` | Thiên Băng Địa Liệt | **8** | 0 | 0 | **0** | ❌ **LỌT LƯỚI** (thực tế 8) |

#### Phân tích sai lệch:
- Toàn bộ 376 kỹ năng trong `Data/Skill.Dat` đều có byte offset 31 (`pc.unk4`) mang giá trị `0x00`.
- Do `skill_pc.rs` gán `fight_area = pc.unk4`, trường `fight_area` của mọi kỹ năng nhị phân **luôn luôn bằng 0**.
- Khi `skill_target_count` lấy `def.fight_area`, nó trả về `0`. Trong `detect_combo`, điều kiện `targets <= 3` trở thành `0 <= 3` (luôn đúng!).
- Do đó, trong gameplay thực tế, **kỹ năng Loạn kích (4 mục tiêu), Phi sa (5 mục tiêu), Hồng thủy (7 mục tiêu) đều không bị chặn mà vẫn kích hoạt combo bình thường**!
- Test `tests/battle_combo_rules_test.rs` vượt qua 100% chỉ vì test đã tự tạo mock `BinarySkillDef { fight_area: 4, ... }`, che giấu hoàn toàn bug ở tầng nạp dữ liệu thật!

#### Giải pháp chuẩn mực:
- Số mục tiêu chuẩn xác nằm ở `pc.nb_target` (offset 34).
- Trong `skill_pc.rs`, cần chuẩn hóa ánh xạ:
  `fight_area: pc.nb_target` (hoặc `skill_target_count` phải đọc `def.fight_way` vốn đang nhận `pc.nb_target`).
- Đảm bảo `skill_target_count` trả về đúng số lượng mục tiêu thực tế (1, 2, 3, 4, 5, 6, 7, 8).

---

### 3.6. Lỗ Hổng 6: Hệ Số Dồn Sát Thương Bị Bỏ Rơi & Thưởng EXP/Drop Chưa Tích Hợp

#### 1. Hệ số sát thương `num37` bị Discard:
- Trong `runner.rs:903`, server khởi tạo `let mut num37 = 2.0f64;`.
- Khi kích hoạt combo (`detect_combo`), code gọi `*num37 *= 1.3;`. Khi nối tiếp combo, `num37 *= 1.3;`.
- Tuy nhiên, tại `runner.rs:1446`, biến `num37` bị gán vào tuple discard:
  `let _ = (combo_field, num37, num34, do_manh, skill_tt, atk, int_stat);`
- Sát thương thực tế được tính bằng `combat_formula::calculate_attack(...)` mà hàm này không nhận bất kỳ tham số combo nào!
- **Hậu quả**: Hợp kích thành công nhưng sát thương gây ra hoàn toàn không tăng thêm một điểm nào so với đòn đánh đơn lẻ!
- **Giải pháp**: Nếu `ts.combo_active == 1`, áp dụng hệ số nhân dồn sát thương vào kết quả tính toán:
  $$\text{Damage}_{\text{combo}} = \left\lfloor \text{Damage}_{\text{base}} \times 1.3 \right\rfloor$$

#### 2. Thưởng Kinh Nghiệm khi Tiêu Diệt Quái bằng Combo:
- Trong `src/battle/damage.rs:208`, hàm `calc_combo_exp` được định nghĩa:
  ```rust
  pub fn calc_combo_exp(base_exp: i64) -> i64 {
      banker_round(base_exp as f64 * 1.086) as i64
  }
  ```
  Hệ số `1.086` phản ánh phần thưởng tăng thêm $+8.6\%$ EXP kinh điển khi hợp kích diệt quái của TS Online.
- Tuy nhiên, trong `runner.rs:3073-3108`, khi cấp phát EXP cho các đấu sĩ trong `ts.combo_cells`, server lại gọi hàm `reward_exp(plv, npc_lv)` tự chế mà không áp dụng `calc_combo_exp`!
- **Giải pháp**: Tích hợp `damage::calc_combo_exp` khi tính toán exp cho các thành viên trong chuỗi combo.

---

## 4. BẢNG TỔNG HỢP SO SÁNH HIỆN TRẠNG VS THIẾT KẾ MỚI

| Tiêu chí | Hiện trạng (Trạng thái lỗi) | Đề xuất Chuẩn hóa (Refactored Spec) | File Cần Sửa |
|---|---|---|---|
| **Agi Gap Ngưỡng Mở** | `|next - attacker| <= 800` | `(next.agi - attacker.agi).abs() <= 11` | `src/battle/runner.rs` |
| **Agi Gap Chuỗi Nối Dài** | `|next - (lo+hi)/2| <= 800` | `max(combo_hi, next.agi) - min(combo_lo, next.agi) <= 11` | `src/battle/runner.rs` |
| **Xác suất Kích hoạt** | Luôn luôn $100\%$ (`get_random_miss_combo()`) | Tuyến tính theo $(\text{LVTB}_{\text{team}} - 25) / \text{Level}_{\text{quái}}$ | `src/battle/damage.rs` & `runner.rs` |
| **Level Nhân vật Trùng Sinh** | Lấy level hiển thị `c.lv` | Bắt buộc gán $200$ khi `c.reborn > 0` | `src/battle/runner.rs` |
| **Nguồn Đọc Loại Kỹ năng** | `s.kind` (`pc.type_` offset 21) | `s.attribute` (`pc.skill_type` offset 33) | `src/battle/runner.rs` |
| **Tiêu chí Kỹ năng Hợp lệ** | `next_type == 1` (chỉ hệ Địa) | `id == 10000 || skill_type == 1` (Active mọi hệ) | `src/battle/runner.rs` |
| **Nguồn Đọc Số Mục tiêu AoE** | `def.fight_area` (`pc.unk4` = 0) | `pc.nb_target` (offset 34) $\rightarrow$ `def.fight_area` | `src/data/loaders/skill_pc.rs` & `runner.rs` |
| **Hệ số Sát thương Combo** | `num37` bị discard, damage gốc | Nhân $\times 1.3$ khi `ts.combo_active == 1` | `src/battle/runner.rs` |
| **Cộng thưởng EXP Combo** | Bị bỏ quên, dùng heuristic | Nhân hệ số `1.086` (`calc_combo_exp`) | `src/battle/runner.rs` |

---

## 5. THIẾT KẾ GIẢI THUẬT & MÃ NGUỒN ĐỀ XUẤT (TECHNICAL DESIGN)

### 5.1. Cập nhật `src/data/loaders/skill_pc.rs`
Chuẩn hóa ánh xạ từ `PcSkillDef` sang `BinarySkillDef`:
```rust
pub fn pc_to_binary(pc: &PcSkillDef) -> BinarySkillDef {
    let mut pre_skills = Vec::with_capacity(6);
    if pc.require_sk != 0 {
        pre_skills.push(pc.require_sk);
    }
    if pc.unk21 != 0 {
        pre_skills.push(pc.unk21);
    }

    BinarySkillDef {
        id: pc.id,
        name: pc.name.clone(),
        kind: pc.type_,               // Offset 21: Element/Type (0=None, 1=Earth, 2=Water, 3=Fire, 4=Wind, 5=Mind)
        require_sp: pc.sp_cost,
        element: pc.elem,
        numerical: pc.unk3,
        attribute: pc.skill_type,     // Offset 33: SkillType (1=Active, 2=Buff, 3=Debuff, 4=Heal, 17=Special)
        level: 0,
        fight_way: pc.nb_target,      // Offset 34: Target count
        fight_area: pc.nb_target,     // <-- FIX: Gán số lượng mục tiêu vào fight_area thay vì pc.unk4 (luôn = 0)
        round: pc.state,
        spend_second: pc.delay,
        hit_status: pc.unk11,
        how_much_times: pc.grade,
        limit_lv: pc.unk12,
        learn_point: pc.sk_point,
        level_up_point: pc.unk14,
        max_lv: pc.max_lvl,
        pre_skills,
        atk_kind: pc.unk17,
        turn_kind: (pc.unk19 & 0xFF) as u8,
        learn_limit: pc.unk18,
        use_limit: pc.unk20 as u16,
        fight_way_grow_type: pc.unk22,
        description: String::new(),
    }
}
```

### 5.2. Cập nhật `src/battle/damage.rs`
Xây dựng lại hàm tính toán tỷ lệ hợp kích theo chuẩn LVTB:
```rust
/// Roll xác suất Hợp kích (Combo) theo quy tắc TS Online:
/// - Khi LVTB Team - Level Quái >= 25 => Tỷ lệ = 100%.
/// - Khi LVTB Team - Level Quái < 25 => Tỷ lệ = clamp(floor((LVTB - 25) / Level_Quái * 100), 0, 100).
pub fn get_random_miss_combo(
    rng: &mut DotNetRandom,
    lvtb_team: f64,
    target_lv: i64,
) -> i64 {
    if target_lv <= 0 {
        return 1;
    }
    let diff = lvtb_team - 25.0;
    if diff <= 0.0 {
        return 0;
    }
    let percent = ((diff / target_lv as f64) * 100.0).floor() as i64;
    randomize_with_percent(rng, 1, 0, percent.clamp(0, 100))
}
```

### 5.3. Cập nhật `src/battle/runner.rs`

#### 1. Tính toán LVTB với Level 200 cho Nhân vật Trùng Sinh:
```rust
// Average levels (tính ở lượt đầu tiên)
if ts.avg1.is_none() {
    let effective_lv = if c.reborn > 0 { 200 } else { c.lv };
    if team == 1 {
        sum1 += effective_lv;
        count1 += 1;
    } else if team == 2 {
        sum2 += effective_lv;
        count2 += 1;
    }
}
```

#### 2. Chuẩn hóa trích xuất `skill_target_count` & `is_combo_eligible_skill`:
```rust
/// Trích xuất số lượng mục tiêu tác dụng của kỹ năng.
pub fn skill_target_count(data: &BattleData<'_>, skill_id: i64) -> u8 {
    if skill_id == 10000 || skill_id == 0 {
        return 1;
    }
    if let Some(def) = binary_skill_def(data, skill_id) {
        return def.fight_way.max(def.fight_area); // Đọc an toàn cả hai trường
    }
    if let Some(s) = data.skills.get(&skill_id) {
        return s.sl_danh.max(1) as u8;
    }
    1
}

/// Kiểm tra kỹ năng có đủ tư cách tham gia chuỗi Combo hay không:
/// 1. Đòn đánh thường (10000 hoặc 0).
/// 2. HOẶC Kỹ năng Active (skill_type == 1) VÀ phạm vi tác dụng <= 3 mục tiêu.
pub fn is_combo_eligible_skill(data: &BattleData<'_>, skill_id: i64) -> bool {
    if skill_id == 10000 || skill_id == 0 {
        return true;
    }
    let target_count = skill_target_count(data, skill_id);
    if target_count > 3 {
        return false;
    }
    let skill_type = binary_skill_def(data, skill_id)
        .map(|s| s.attribute as i64) // <-- Attribute chứa pc.skill_type
        .or_else(|| data.skills.get(&skill_id).map(|s| s.skill_type))
        .unwrap_or(0);

    skill_type == 1 // Bắt buộc là Active
}
```

#### 3. Chuẩn hóa giải thuật `detect_combo`:
```rust
fn detect_combo(
    &mut self,
    data: &BattleData,
    ts: &mut TurnState,
    sorted: &[(u8, u8)],
    idx: usize,
    attacker: &WarInfo,
    num37: &mut f64,
) {
    if !is_combo_eligible_skill(data, attacker.id_skill) {
        ts.combo_active = 0;
        return;
    }

    let (nr, nc) = sorted[idx + 1];
    let next = self.list_war[&crate::battle::engine::war_key(nr, nc)].clone();
    
    // Cùng phe và mục tiêu hợp lệ
    if next.id > 0 && next.team == attacker.team {
        if !is_combo_eligible_skill(data, next.id_skill) {
            ts.combo_active = 0;
            return;
        }

        if next.row_attack == attacker.row_attack && next.col_attack == attacker.col_attack {
            let target_cell = self.cell(attacker.row_attack, attacker.col_attack);
            let target_alive = target_cell.map(|c| c.hp > 0).unwrap_or(false);
            let target_lv = target_cell.map(|c| c.lv).unwrap_or(1);

            if target_alive {
                let lvtb_team = if attacker.team == 1 {
                    ts.avg1.unwrap_or(0.0)
                } else {
                    ts.avg2.unwrap_or(0.0)
                };

                // Kiểm tra Agi Gap <= 11
                if ts.combo_hi == 0 {
                    // Mắt xích đầu tiên giữa attacker và next
                    let agi_diff = (next.agi - attacker.agi).abs();
                    if agi_diff <= 11 {
                        ts.combo_hi = next.agi.max(attacker.agi);
                        ts.combo_lo = next.agi.min(attacker.agi);
                        ts.combo_active = damage::get_random_miss_combo(
                            &mut self.rng.random_0,
                            lvtb_team,
                            target_lv,
                        );
                        ts.delay = next_delay(data, &next);
                        if ts.combo_active == 1 && ts.text10.is_empty() {
                            *num37 *= 1.3;
                            add_combo_cell(ts, attacker.row, attacker.col, attacker.lv);
                            add_combo_cell(ts, next.row, next.col, next.lv);
                        }
                        return;
                    }
                } else {
                    // Mắt xích nối tiếp: kiểm tra span cực đại không vượt quá 11
                    let new_hi = ts.combo_hi.max(next.agi);
                    let new_lo = ts.combo_lo.min(next.agi);
                    if new_hi - new_lo <= 11 {
                        ts.combo_hi = new_hi;
                        ts.combo_lo = new_lo;
                        ts.combo_active = damage::get_random_miss_combo(
                            &mut self.rng.random_0,
                            lvtb_team,
                            target_lv,
                        );
                        let nd = next_delay(data, &next);
                        if ts.delay <= nd {
                            ts.delay = nd;
                        }
                        if ts.combo_active == 1 && ts.text10.is_empty() {
                            *num37 *= 1.3;
                            add_combo_cell(ts, attacker.row, attacker.col, attacker.lv);
                            add_combo_cell(ts, next.row, next.col, next.lv);
                        }
                        return;
                    }
                }
            }
        }
    }

    ts.combo_active = 0;
}
```

---

## 6. KẾT LUẬN & ĐỀ XUẤT LỘ TRÌNH THỰC HIỆN

Nghiên cứu trên đã chứng minh rằng ticket `.scratch/battle-refactor/issues/23-combo-rules-aoe-threshold.md` trước đây chỉ giải quyết được một góc nhỏ (điều kiện AoE $\le 3$) nhưng lại bị vô hiệu hóa bởi lỗi nạp dữ liệu `unk4 = 0`, đồng thời bỏ sót hoàn toàn 5 quy tắc sống còn khác của hệ thống Hợp kích TS Online.

Cần tiến hành tái cấu trúc toàn diện ticket 23 thành **"Chuẩn hóa toàn diện cơ chế Hợp kích (Battle Combo Engine V2)"** bao gồm 6 nhiệm vụ đồng bộ:
1. Chuẩn hóa ánh xạ dữ liệu `Skill.Dat` (`nb_target`, `skill_type`).
2. Sửa công thức LVTB cho nhân vật Trùng Sinh (`reborn > 0 \implies 200`).
3. Sửa hàm tính tỷ lệ combo theo hiệu số $\text{LVTB} - \text{Level Quái}$.
4. Siết chặt Agi Gap $\le 11$ cho cả mắt xích mở đầu và toàn bộ chuỗi nối dài.
5. Lọc kỹ năng Active chuẩn xác cho mọi hệ nguyên tố.
6. Kích hoạt nhân dồn sát thương $\times 1.3$ và thưởng EXP $\times 1.086$ khi combo thành công.
