# Outer opcode 0x3A — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 3A 01` | `59 E9 AF AD 97 AC` | `branch-routing` | `H[0]=0x01 0x007956DF` | 0x7956D8..0x7956EE | routing-high | `fable_deep_vectors_37_48_C7.json` |
