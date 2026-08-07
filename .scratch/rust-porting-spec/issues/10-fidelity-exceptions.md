# Ngoại lệ fidelity (tên garble, MySQL branch)

Status: resolved
Type: grilling
Blocked by: 03

## Question

Nghiên cứu encoding phát hiện các ngoại lệ cần quyết định trước khi chốt hợp đồng: (1) 99 tên item + 23 tên NPC chứa ký tự CP1252 >0xFF bị server C# garble/abort khi gửi — spec yêu cầu tái hiện garble (byte-level fidelity) hay chuẩn hoá ngược về VISCII? (2) branch `item_code` MySQL (opcode 0x23 sub 3) degrade khi không có DB — giữ nguyên hành vi degrade hay bỏ? (3) `Title=` quest với 144 encoding 8-bit không xác định (server-GUI-only) — xử lý ra sao? Mỗi ngoại lệ: quyết định + lý do so với mục tiêu fidelity.

## Answer

(Quyết định từ grilling — HITL.)

1. **Garble tên → TÁI HIỆN byte-for-byte.** Port theo đúng `smethod_13` của C#: char <0x100 → 2 hex digit (VISCII); CP1252 punctuation ≥0x100 → `AscW.ToString("X2")` (4 hex digit → 2 byte rác trên wire); nhóm 3-digit (như `ă` item 48101) → abort packet. Lý do: mục tiêu fidelity là byte-exact với capture traffic C# thật, và acceptance harness diff Rust output vs capture — sửa "đúng VISCII" sẽ diverge và fail harness. Header thư mục spec dành riêng mục "Ngoại lệ garble" (bảng 122 tên con cụ thể + giá trị hex C# phát ra) để executor biết đây là bug-for-bug có chủ đích, không phải lỗi.
2. **Bỏ nhánh degrade `itemCode`.** MySQL bắt buộc, bootstrap fail-fast (quyết định từ ticket **Thiết kế schema MySQL 8**) ⇒ không tồn tại trạng thái "no DB" lúc chạy. Port đưa nhánh redeem functional đầy đủ (opcode 0x23 sub 3): bind parameter (chống SQL injection), transaction `SELECT … WHERE player_id=0` → UPDATE để chống redeem trùng. Không giữ code chết.
3. **`Title=` quest giữ opaque bytes.** Giữ raw bytes (0xA0–0xEF) trong bảng quest; KHÔNG transcode, KHÔNG gửi client — đã xác định chỉ dùng cho server-GUI strings (Data.cs:5770). Web dashboard hiển thị quest thì dùng id hoặc ghi chú tên raw; không cố decode. Không tác động wire fidelity.
