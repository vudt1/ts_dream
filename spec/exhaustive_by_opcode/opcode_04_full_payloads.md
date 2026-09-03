# Outer opcode 0x04 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x40` | 16 | `40 30 20 10 00 00 00 00 00 34 12 06 05 08 07` | `F4 44 10 00 04 40 30 20 10 00 00 00 00 00 34 12 06 05 08 07` | `59 E9 BD AD A9 ED 9D 8D BD AD AD AD AD AD 99 BF AB A8 A5 AA` | `direct-schema` | `linear parser; H[9..10] guard 0x0078C7DB` | 0x0078C7F1..0x0078C820 and 0x0078C826..0x0078C94B. | schema-high | `fable_deep_vectors_01_02_03_04_05_07.json` |
