# Outer opcode 0x07 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x40` | 11 | `40 30 20 10 34 12 06 05 08 07` | `F4 44 0B 00 07 40 30 20 10 34 12 06 05 08 07` | `59 E9 A6 AD AA ED 9D 8D BD 99 BF AB A8 A5 AA` | `direct-schema` | `linear parser; primary-ID or other-entity branch 0x0078CE37` | 0x0078CE37..0x0078D11A. | schema-high | `fable_deep_vectors_01_02_03_04_05_07.json` |
