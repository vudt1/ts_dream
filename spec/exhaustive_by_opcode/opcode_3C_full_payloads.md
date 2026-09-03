# Outer opcode 0x3C — exhaustive full payloads

> Số payload unique: **4**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 3C 01` | `59 E9 AF AD 91 AC` | `branch-routing` | `H[0]=0x01 0x0079578C` | 0x79577C..0x7957D7 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 3C 02` | `59 E9 AF AD 91 AF` | `branch-routing` | `H[0]=0x02 0x007957A0` | 0x79577C..0x7957D7 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 3C 03` | `59 E9 AF AD 91 AE` | `branch-routing` | `H[0]=0x03 0x007957B4` | 0x79577C..0x7957D7 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x04` | 2 | `04` | `F4 44 02 00 3C 04` | `59 E9 AF AD 91 A9` | `branch-routing` | `H[0]=0x04 0x007957C8` | 0x79577C..0x7957D7 | routing-high | `fable_deep_vectors_37_48_C7.json` |
