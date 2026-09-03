# Outer opcode 0x20 — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 20 01` | `59 E9 AF AD 8D AC` | `branch-routing` | `H[0]=0x01 0x00792884` | 0x00792879..0x007928DF | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 20 02` | `59 E9 AF AD 8D AF` | `branch-routing` | `H[0]=0x02 0x007928D0` | 0x00792879..0x007928DF | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
