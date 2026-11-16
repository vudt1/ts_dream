# 02: Tách `equip_pos` khỏi `kind` để Nhân vật & Sủng vật mặc được Vũ khí Cuốc

**What to build:** Người chơi và Sủng vật có thể mặc, tháo và hoán đổi toàn bộ 6 Vũ khí Cuốc (`10001` *Cuốc*, `10530` *Cuốc chân kim*, `10531` *Cuốc tân truyền*, `10532` *Cuốc hòa điền*, `10533` *Cuốc đá vũ hoa*, `10534` *Cuốc nham thạch đỏ*) từ `homdo` vào ô vũ khí của Nhân vật (ô `3` của `trangbi`) hoặc của Sủng vật (ô `pet.stt * 10 + 3` của `trangbi`) khi đủ cấp độ yêu cầu. Tầng dữ liệu tách bạch vị trí trang bị (`equip_pos`, lấy từ `fit_type`) khỏi loại vật phẩm (`kind`), giúp Cuốc (`kind = 39`, `equip_pos = 3`) và các trang bị có `kind > 6` hoạt động đúng khi trang bị trong khi vẫn giữ nguyên `kind` cho hệ thống Compound/Crafting, Shop và Trade.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Chuyển đổi mẫu vật phẩm (`ItemDef` sang `Item`) và hydrate `InventoryItem` từ `GameData` bảo toàn cả `equip_pos` (`fit_type`) lẫn `kind`/`loai`; các vật phẩm dựng thủ công trong bộ nhớ hoặc fixture cũ chỉ đặt `loai` trong `1..=6` vẫn tự suy luận fallback sang `equip_pos` tương ứng.
- [x] Người chơi mặc được các Vũ khí Cuốc (`10001`, `10530..=10534`) vào ô `3` của `trangbi` (đồng bộ `17 0B` và cộng đúng chỉ số trang bị), tháo hoặc hoán đổi với vũ khí khác bình thường khi ngoài trận đấu.
- [x] Sủng vật (`stt ∈ 1..=4`) mặc, tháo và hoán đổi được các Vũ khí Cuốc vào ô `pet.stt * 10 + 3` của `trangbi` khi đủ cấp độ yêu cầu và đang ngoài trận đấu (`battle_id == 0`); khi đang trong trận đấu (`battle_id > 0`), thao tác đổi đồ Sủng vật bị chặn giống như của Nhân vật.
- [x] Các thao tác trang bị từ chối an toàn khi vật phẩm không phải trang bị (như `29335` *Cuốc đào* có `equip_pos = 0`) hoặc chưa đủ cấp độ yêu cầu, không làm mất vật phẩm khỏi `homdo` và không làm sai lệch các ô phụ cận.
- [x] Giá trị `kind` (như `kind = 39` của Cuốc) và hành vi phân loại trang bị của hệ thống Compound, Shop, Trade được giữ nguyên không bị hồi quy.
- [x] Có kiểm thử tự động trong domain suite `economy_inventory` nạp dữ liệu từ `ItemDef` / `GameData` thật kiểm tra đầy đủ luồng mặc/tháo/hoán đổi cho cả Nhân vật lẫn Sủng vật, các ca từ chối (sai `equip_pos`, thiếu level, đang trong trận) và toàn vẹn ô phụ cận.
