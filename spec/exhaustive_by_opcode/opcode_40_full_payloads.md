# Outer opcode 0x40 — exhaustive full payloads

> Số payload unique: **3**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 40 01` | `59 E9 AF AD ED AC` | `branch-routing` | `H[0]=0x01 0x00795CA8` | 0x795C9B..0x795CDF | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 40 02` | `59 E9 AF AD ED AF` | `branch-routing` | `H[0]=0x02 0x00795CBC` | 0x795C9B..0x795CDF | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 40 03` | `59 E9 AF AD ED AE` | `branch-routing` | `H[0]=0x03 0x00795CD0` | 0x795C9B..0x795CDF | routing-high | `fable_deep_vectors_37_48_C7.json` |
