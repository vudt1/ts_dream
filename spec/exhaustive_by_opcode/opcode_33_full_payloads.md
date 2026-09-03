# Outer opcode 0x33 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 33 01` | `59 E9 AF AD 9E AC` | `outer-route` | `outer-route 0x00795252 -> 0x0064CAE0(H)` | H[0]=1 is the only accepted selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
