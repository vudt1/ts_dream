# 15 — Trade (op 0x19) + storage transfer (op 0x1E) + bank gold (op 0x1D)

**What to build:** Trao đổi vật phẩm/vàng giữa 2 người chơi (gồm pet trade và transfer vật phẩm 1 chiều), chuyển đồ giữa các kho Homdo↔TienTrang↔LuuLang, và gửi/rút vàng vào ngân hàng. Nguồn tài nguyên và giao dịch.

**Blocked by:** 11 — Inventory base (cần item/slot path).

**Status:** ready-for-agent

- [ ] Op 0x19 sub 1 open: `data[6..9]` partner; cả 2 nhận `F44406001901`+le32(other); pet trade `F4440600190A` (pet names 28-char pad `6`) (Ch2 §2.3.15).
- [ ] Sub 2 — set gold + items: parner nhận `F444`+len+`1903`+gold+item entries.
- [ ] Sub 3 confirm/cancel: cả 2 accept → `GoldTransfer`, swap both directions; hết slot → `F4440300190207`; success → `F4440300190204`; cancel → `F4440300190203` partner + `F4440300190209` self, `TradeFinish`.
- [ ] Sub 10/11/12 — pet trade: open, offer pet, confirm/cancel (`F4440300190B03/04/07/0A/0F` family).
- [ ] Sub 20 — transfer item: recipient bytes 10..13, 9 slot/count; recipient `F4440E001706`+items; sender re-send `F444`+len+`1705`.
- [ ] Op 0x1E storage transfer (TienTrang): sub1 TienTrang→Homdo per-move detail + end `F44402001732`; sub2 Homdo→TienTrang per-move `F44404001709`+slot+`32` rồi `F444`+len+`1E04`; sub8 set `SelectMenu=40` (Ch2 §2.3.19).
- [ ] Op 0x1D bank gold: sub1 withdraw gate `bank ≥ amount && gold+amount ≤ 9999999` → `F44406001D02`+le16 + `F44406001A01`+le16; sub2 deposit `F44406001D01`+le16+`F44406001A02`+le16 (Ch2 §2.3.18).
- [ ] Mọi thao tác bảng cất trữ có `player_id`.
- [ ] Golden: một vài scenario trade/bank nếu capture đủ deterministic.