# Outer opcode 0x47 — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 47 01` | `59 E9 AF AD EA AC` | `branch-routing` | `H[0]=0x01 0x007961EB` | 0x7961E1..0x796243 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 47 02` | `59 E9 AF AD EA AF` | `branch-routing` | `H[0]=0x02 0x00796226` | 0x7961E1..0x796243 | routing-high | `fable_deep_vectors_37_48_C7.json` |
