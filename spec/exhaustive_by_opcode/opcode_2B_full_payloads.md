# Outer opcode 0x2B — exhaustive full payloads

> Số payload unique: **10**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 2B 00` | `59 E9 AF AD 86 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 2B 01` | `59 E9 AF AD 86 AC` | `selector-routing` | `H[0]=0x01 0x007947E4` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 2B 02` | `59 E9 AF AD 86 AF` | `selector-routing` | `H[0]=0x02 0x007947F8` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 2B 03` | `59 E9 AF AD 86 AE` | `selector-routing || direct-schema` | `H[0]=0x03 0x0079480C` | jump table 0x007947C8 // 0x0079480C..0x00794854 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 2B 04` | `59 E9 AF AD 86 A9` | `selector-routing` | `H[0]=0x04 0x00794859` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 2B 05` | `59 E9 AF AD 86 A8` | `selector-routing` | `H[0]=0x05 0x0079486D` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 2B 06` | `59 E9 AF AD 86 AB` | `selector-routing` | `H[0]=0x06 0x00794881` | jump table 0x007947C8 | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 8 | `0x06` | 3 | `06 01` | `F4 44 03 00 2B 06 01` | `59 E9 AE AD 86 AB AC` | `nested-selector-routing` | `H[0]=0x06,H[1]=0x01 0x007948AA` | 0x00794881..0x0079490B | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 9 | `0x06` | 3 | `06 02` | `F4 44 03 00 2B 06 02` | `59 E9 AE AD 86 AB AF` | `nested-selector-routing` | `H[0]=0x06,H[1]=0x02 0x007948CC` | 0x00794881..0x0079490B | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
| 10 | `0x06` | 3 | `06 03` | `F4 44 03 00 2B 06 03` | `59 E9 AE AD 86 AB AE` | `nested-selector-routing` | `H[0]=0x06,H[1]=0x03 0x007948EE` | 0x00794881..0x0079490B | routing-high | `fable_deep_vectors_28_29_2A_2B_2C.json` |
