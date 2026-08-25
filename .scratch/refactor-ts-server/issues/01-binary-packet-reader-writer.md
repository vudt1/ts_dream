# 01 — Binary Packet Reader and Writer

**What to build:** Xây dựng bộ công cụ đọc và ghi gói tin nhị phân chuẩn hóa (`PacketReader` và `PacketWriter`) hỗ trợ Little-Endian cho tất cả các kiểu dữ liệu số nguyên (u8..u64, i8..i32), số thực f64 (OADate), boolean, và chuỗi ký tự theo chuẩn VISCII 1.1 / Big5 / UTF-8. Thay thế toàn bộ cơ chế ghép chuỗi Hex string trung gian bằng bộ đệm nhị phân Zero-Copy.

**Blocked by:** None — can start immediately

**Status:** completed

- [x] `PacketReader<'a>` hỗ trợ đọc các kiểu dữ liệu `read_u8`, `read_u16_le`, `read_u32_le`, `read_u64_le`, `read_i8`, `read_i16_le`, `read_i32_le`, `read_f64_le`, `read_bool`, `read_bytes` với kiểm tra biên an toàn (bounds checking).
- [x] `PacketReader<'a>` hỗ trợ giải mã chuỗi `read_viscii_pascal` (1-byte length prefix) và `read_viscii_fixed`.
- [x] `PacketWriter` hỗ trợ fluent buffer building (`write_u8`, `write_u16_le`, `write_u32_le`, `write_bytes`, `write_viscii_pascal`, `write_viscii_fixed`).
- [x] `PacketWriter` cung cấp hàm `build_frame()` (Header `F4 44` + LE Length + Body) và `build_wire()` (mã hóa XOR `0xAD`).
- [x] 100% unit tests kiểm tra tính chính xác của reader/writer với mảng byte nhị phân thực tế.
