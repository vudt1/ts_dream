# 16 — Role & hệ-thống opcode (0x21 PK, 0x22 points, 0x41 rank, 0x42 GM, 0x0C teleport, 0x23 account mgmt)

**What to build:** Các handler "role/hệ thống" duy nhất hoạt động đúng packet: chế độ PK/war, game points, rank, GM/mall shop, teleport confirm, và quản lý account (đổi mật khẩu, xóa nhân vật, redeem `item_code`). Gồm phần đổi mật khẩu/xóa nhân vật cần thao tác DB sâu.

**Blocked by:** 12 — Use item (GM mall/gold points dựa điểm); móc nối thêm 04 (DB `item_code`/`accounts`), 07 (session) cho phần account mgmt.

**Status:** ready-for-agent

- [ ] Op 0x21 PK/war: sub1 `data[6]` 0→`Pk=0` rep `F4440400210200`+thamchien; 1→`Pk=1` `F4440400210201`; sub2 `data[6]` 0/1→`ThamChien` rep `F44404002102`+pk+`00/01` (Ch2 §2.3.22).
- [ ] Op 0x22 game points sub1 → `F44412002304`+le16(gold)+00×24 (Ch2 §2.3.23).
- [ ] Op 0x41 rank → `F44402004101` (sub1) / `F44402004102` (sub2) (Ch2 §2.3.28).
- [ ] Op 0x42 GM/mall: sub1 `data[9..10]` item, `[11..12]` price; gate `_Shop_Point ≥ price` + free slot → add/deduct; sub2 no-op; sub3 `F44406004202`+le16(points)+`0100` (Ch2 §2.3.29).
- [ ] Op 0x0C teleport confirm sub1: có leader khác → `F44402000504F44402001408`; ngược lại `warpFinish=false` + 2 packets đó, reset talk counters (Ch2 §2.3.9).
- [ ] Op 0x23 account mgmt (Ch2 §2.3.24):
  - sub1 change password: 4 len-prefixed strings; wrong oldPass1 → `F4440300230102`; wrong oldPass2 → `0103`; success write + `F4440300230101`.
  - sub2 delete char: verify pass; leave battle + map removal, `GiaiTanParty`, offline + map broadcast, **xóa `players` + toàn bộ 9 bảng gameplay theo player_id**, remove Client, close.
  - sub3 redeem `item_code`: len-prefixed code+password; **bind param** (không concat), transaction SELECT-gate-`UPDATE … WHERE player_id=0` rowcount==1 (chống double-redeem race); grant item; đã dùng → red msg; hardcode TSVN123/456 gift & tanthu flag (Ch5 §5.5).
- [ ] Golden: thao tác hệ thống deterministic (rank/points) được ghi lại.