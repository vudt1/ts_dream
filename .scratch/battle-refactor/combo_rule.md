# Quy tắc Combo (Hợp kích) trong TS Online

Trong tựa game chiến thuật **TS Online **, **quy tắc combo (hợp kích)** là yếu tố cốt lõi giúp đội hình tối ưu hóa sát thương, nhận nhiều kinh nghiệm (EXP) và tăng tỷ lệ rơi vật phẩm khi treo máy (auto train).

Để cả đội hình (gồm các nhân vật và pet) có thể kích hoạt đòn đánh hợp kích 100%, bạn cần tuân thủ nghiêm ngặt **3 quy tắc vàng** sau đây:

### 1. Khoảng cách Agi (Tốc độ) không quá 11 điểm
Đây là điều kiện quan trọng nhất để các nhân vật có thể xuất chiêu cùng một lúc.
*   **Quy tắc:** Chênh lệch Agi giữa thành viên có Agi **cao nhất** và thành viên có Agi **thấp nhất** trong toàn bộ đội hình (bao gồm cả chủ acc và pet của tất cả các slot tham gia trận đấu) **không được vượt quá 11 điểm**. 
*   *Ví dụ:* Nếu nhân vật chậm nhất trong team có Agi là 330, thì nhân vật nhanh nhất tối đa chỉ được đạt Agi là 341. Chỉ cần có một pet hoặc nhân vật đạt Agi 342, tỷ lệ combo của team sẽ lập tức bị hụt.
*   **Mẹo chỉnh Agi:** Bạn có thể tăng/giảm Agi bằng cách cộng điểm thuộc tính khi lên cấp, thay đổi trang bị tăng/giảm Agi, hoặc cho pet đeo các loại **Tạ nặng** mua trong shop để chủ động giảm Agi xuống cho bằng với team.

### 2. Cấp độ trung bình (LVTB) của Team phải cao hơn Quái ≥ 25 cấp
Để đạt tỷ lệ combo **ổn định 100%**, cấp độ trung bình của cả đội phải áp đảo cấp độ của bãi quái đang train.
*   **Công thức tính LVTB Team:** 
    $$\text{LVTB Team} = \frac{\text{Tổng Level của tất cả Nhân vật + Pet trên sân}}{\text{Tổng số lượng Nhân vật + Pet trên sân}}$$
*   **Quy tắc:** LVTB Team - Level NPC Quái $\ge$ 25.
*   **Lưu ý đặc biệt cho nhân vật Trùng Sinh (TS):** Đối với các nhân vật đã Trùng Sinh, hệ thống game sẽ **mặc định tính là Level 200** khi đưa vào công thức tính LVTB, bất kể level hiển thị thực tế của nhân vật đó đang là bao nhiêu. Nhân vật chưa Trùng Sinh vẫn tính theo level hiện tại.
*   *Mẹo nhỏ:* Nếu tính toán LVTB chưa đủ gánh bãi quái lớn, bạn có thể sử dụng thêm **Đá Hợp Kích** (tìm thấy khi mở Túi Tinh Thạch) để bù đắp và kích tỷ lệ combo lên.

### 3. Bắt buộc sử dụng kỹ năng có khả năng Hợp Kích
Không phải kỹ năng nào cũng có thể kích hoạt combo. 
*   **Quy tắc:** Tất cả các nhân vật và pet tham gia chuỗi combo đều phải cùng dùng các chiêu thức có hỗ trợ "Hợp kích", ghi chú trong skills.json thì là thuộc tính  skillTypeName là Active, cần kiểm tra field tương ứng trong skill_pc.rs.

