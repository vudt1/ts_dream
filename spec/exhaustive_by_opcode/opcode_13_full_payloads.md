# Outer opcode 0x13 — exhaustive full payloads

> Số payload unique: **4**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 13 01` | `59 E9 AF AD BE AC` | `outer-route` | `outer-route 0x0078EB1C -> 0x007A63DC(H), conditional reset 0x00650E40` | H[0] sparse selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 13 02` | `59 E9 AF AD BE AF` | `outer-route` | `outer-route 0x0078EB95 -> 0x0072B34C, conditional reset 0x00650E40` | H[0] sparse selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
| 3 | `0x04` | 2 | `04` | `F4 44 02 00 13 04` | `59 E9 AF AD BE A9` | `outer-route` | `outer-route 0x0078EC0B -> 0x007A2AE8(H)` | H[0] sparse selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
| 4 | `0x06` | 2 | `06` | `F4 44 02 00 13 06` | `59 E9 AF AD BE AB` | `outer-route` | `outer-route 0x0078EC2B -> 0x007A4D68(H)` | H[0] sparse selector; remaining callee payload omitted | high-routing | `gameplay_opcode_study_vectors.json` |
