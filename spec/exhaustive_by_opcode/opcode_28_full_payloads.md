# Outer opcode 0x28 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 28 01` | `59 E9 AF AD 85 AC` | `branch-routing` | `H[0]=0x01 0x007943E5` | 0x007943DE..0x007943F4 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
