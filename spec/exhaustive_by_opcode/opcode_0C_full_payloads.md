# Outer opcode 0x0C — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x40` | 13 | `40 30 20 10 34 12 06 05 08 07 0A 09` | `F4 44 0D 00 0C 40 30 20 10 34 12 06 05 08 07 0A 09` | `59 E9 A0 AD A1 ED 9D 8D BD 99 BF AB A8 A5 AA A7 A4` | `direct-schema` | `linear parser; primary/non-primary runtime split 0x0078D84D` | 0x0078D84D..0x0078DBA9 | schema-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
