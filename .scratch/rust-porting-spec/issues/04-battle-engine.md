# Trích xuất Battle Engine (TheBattle.cs)

Status: resolved
Type: research

## Question

Trích xuất cạn kiệt `TheBattle.cs` (9.6K LOC) thành tài liệu tham chiếu cho chapter Battle của spec: vòng đời battle (tạo battle NPC/PK, thêm member, `Battling()`), cấu trúc `WarInfo`, targeting theo địa hình (`GetPosRandom*`/`GetPosAttack*` cho normal/combo/TG/3_15/honLoan), công thức damage (`GetDamageThuoctinh`, `GetDamageSkillInt`), luật team, và **toàn bộ chuỗi packet** battle gửi đi (`SendBattleMem*`, `SendEnemyEntities`, `SendBattleLeader*`, `SendSKillingToParty`...). Đầu ra đủ để executor port battle byte-faithful mà không cần đọc `TheBattle.cs`.

## Answer

Asset: [04-battle-engine](../../research/04-battle-engine.md) — extracted by research subagent.
