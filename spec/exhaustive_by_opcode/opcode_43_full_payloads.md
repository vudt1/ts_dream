# Outer opcode 0x43 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 43 01` | `59 E9 AF AD EE AC` | `branch-routing` | `H[0]=0x01 0x00795F6E` | 0x795F68..0x795F7D | routing-high | `fable_deep_vectors_37_48_C7.json` |
