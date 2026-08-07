# 14 — NPC shops (op 0x1B) + player shop (op 0x17 sub 30–33)

**What to build:** Người chơi mua/bán vật phẩm với NPC shop (bảng giá hardcoded `(map,menu)→(item,price)`) và mở quầy hàng cá nhân mua/bán được với người khác. Vertical slice kinh tế nguyên bản.

**Blocked by:** 11 — Inventory base (thao tác slot/gold); 12 — Use item (gold chênh / game economy). Chỉ kích hoạt được khi mở menu qua talk (ticket 18) — gắn dispatch vào chỗ đó.

**Status:** completed

- [x] Op 0x17 sub 0x1B (NPC shop) — buy: check gold ≥ price → `HomdoAddItem`, `PlayerUpdateDataId(Gold)` send `F4440A001A04`+gold+`00000000`, red message (Ch2 §2.3.16).
- [x] Sell: với `idnpctalking ∈ {16005, 99999}` scan inv `26001..26455` (hoặc `27001..27165` cho `16002/99999`); mỗi item bán cộng `data[7]` count vào gold; reply `F4440A001A04`+gold+`00000000`.
- [x] Bảng `(map, menu) → (itemId, price)` hardcoded transcribe verbatim từ C# (Ch2 §2.3.16, research §2.16).
- [x] Player shop (op 0x17 sub 30/31/32/33): open/close shop, open người khác, buy — reply + broadcast frames catalog (Ch2 §2.3.14 sub 30–33; 171E/1F/20/21 frames op 0x17).
- [x] Golden: buy-from-mall/mall-buy scenario được ghi lại (Ch9 §9.6).