# Outer opcode 0x22 — exhaustive full payloads

> Số payload unique: **4**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 22 01` | `59 E9 AF AD 8F AC` | `branch-routing` | `H[0]=0x01 0x00792AF6` | 0x00792AEC..0x00792BA8 | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 22 02` | `59 E9 AF AD 8F AF` | `branch-routing` | `H[0]=0x02 0x00792B6E` | 0x00792AEC..0x00792BA8 | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 3 | `0x01` | 4 | `01 34 12` | `F4 44 04 00 22 01 34 12` | `59 E9 A9 AD 8F AC 99 BF` | `direct-schema` | `H[0]=0x01,H[1..2]=UInt16LE 0x00792AF6` | 0x00792AF6..0x00792B69 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 4 | `0x02` | 5 | `02 41 42 43` | `F4 44 05 00 22 02 41 42 43` | `59 E9 A8 AD 8F AF EC EF EE` | `runtime-schema` | `H[0]=0x02,H[1..]=raw ABC 0x00792B6E` | 0x00792B6E..0x00792BA8 | schema-medium | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
