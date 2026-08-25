# 03 — Binary Dat Reader and Data Loaders

**What to build:** Xây dựng `DatReader` giải mã bảng khóa XOR (`xor1, xor2, xor4`) và offset (`number_offset`) cùng các loader nhị phân nạp trực tiếp toàn bộ 9 tệp `.Dat` (`Item.dat`, `Npc.dat`, `Warp.Dat`, `Formula.Dat`, `BlissBag.Dat`, `Compound.Dat`, `Astrolabe.Dat`, `CityEx.Dat`, `EVOStatus.Dat`) từ thư mục `Data/`. Khắc phục triệt để lỗi thiếu file `Npcs.txt` trong `tests/data.rs`.

**Blocked by:** None — can start immediately

**Status:** completed

- [x] `DatReader` hỗ trợ đọc Little-Endian có kèm giải mã XOR (`xor1, xor2, xor4`) và trừ `number_offset`.
- [x] `DatReader` hỗ trợ đọc chuỗi ký tự PC (1B length + reverse byte array + giải mã VISCII/Big5/UTF-8) và chuỗi Unicode (2B LE length + UTF-16LE), cùng giải mã khối `decode_all`.
- [x] `ItemDatLoader` nạp toàn bộ 3.09 MB `Item.dat` $\to$ `HashMap<u16, ItemDef>` với 54 trường dữ liệu (8,371 records).
- [x] `NpcDatLoader` nạp 612 KB `Npc.dat` $\to$ `HashMap<u16, NpcDef>` (6,659 records).
- [x] `WarpDatLoader`, `FormulaDatLoader`, `BlissBagDatLoader`, `CompoundDatLoader`, `AstrolabeDatLoader`, `CityExDatLoader`, `EVOStatusDatLoader` nạp thành công dữ liệu tương ứng.
- [x] Cập nhật `GameData` tích hợp toàn bộ các loader nhị phân mới và làm xanh toàn bộ 14 bài test trong `tests/data.rs`.
