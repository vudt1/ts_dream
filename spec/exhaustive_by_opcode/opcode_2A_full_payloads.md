# Outer opcode 0x2A — exhaustive full payloads

> Số payload unique: **4**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 2A 01` | `59 E9 AF AD 87 AC` | `branch-routing` | `H[0]=0x01 0x00794749` | 0x00794738..0x00794794 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 2A 02` | `59 E9 AF AD 87 AF` | `branch-routing` | `H[0]=0x02 0x0079475D` | 0x00794738..0x00794794 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 2A 03` | `59 E9 AF AD 87 AE` | `branch-routing` | `H[0]=0x03 0x00794771` | 0x00794738..0x00794794 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 4 | `0x04` | 2 | `04` | `F4 44 02 00 2A 04` | `59 E9 AF AD 87 A9` | `branch-routing` | `H[0]=0x04 0x00794785` | 0x00794738..0x00794794 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
