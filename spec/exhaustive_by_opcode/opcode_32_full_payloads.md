# Outer opcode 0x32 — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 32 01` | `59 E9 AF AD 9F AC` | `outer-route` | `outer-route 0x00795204 -> 0x00647164(H)` | H[0] selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 32 02` | `59 E9 AF AD 9F AF` | `outer-route` | `outer-route 0x00795218 -> 0x0064DD74(H)` | H[0] selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
