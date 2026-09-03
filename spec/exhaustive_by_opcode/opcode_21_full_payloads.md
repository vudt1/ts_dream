# Outer opcode 0x21 — exhaustive full payloads

> Số payload unique: **11**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x02` | 2 | `02` | `F4 44 02 00 21 02` | `59 E9 AF AD 8C AF` | `branch-routing` | `H[0]=0x02 0x00792A6F` | 0x00792A6F..0x00792AC7 | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 2 | `0x01` | 3 | `01 00` | `F4 44 03 00 21 01 00` | `59 E9 AE AD 8C AC AD` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x00 0x0079295F` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 3 | `0x01` | 3 | `01 01` | `F4 44 03 00 21 01 01` | `59 E9 AE AD 8C AC AC` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x01 0x00792981` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 4 | `0x01` | 3 | `01 02` | `F4 44 03 00 21 01 02` | `59 E9 AE AD 8C AC AF` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x02 0x007929A3` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 5 | `0x01` | 3 | `01 03` | `F4 44 03 00 21 01 03` | `59 E9 AE AD 8C AC AE` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x03 0x007929C5` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 6 | `0x01` | 3 | `01 04` | `F4 44 03 00 21 01 04` | `59 E9 AE AD 8C AC A9` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x04 0x007929E7` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 7 | `0x01` | 3 | `01 05` | `F4 44 03 00 21 01 05` | `59 E9 AE AD 8C AC A8` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x05 0x00792A09` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 8 | `0x01` | 3 | `01 06` | `F4 44 03 00 21 01 06` | `59 E9 AE AD 8C AC AB` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x06 0x00792A2B` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 9 | `0x01` | 3 | `01 07` | `F4 44 03 00 21 01 07` | `59 E9 AE AD 8C AC AA` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x07 0x00792A4D` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 10 | `0x01` | 3 | `01 08` | `F4 44 03 00 21 01 08` | `59 E9 AE AD 8C AC A5` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x08 0x00792A4D` | 0x00792912..0x00792A6A | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 11 | `0x02` | 4 | `02 7F 01` | `F4 44 04 00 21 02 7F 01` | `59 E9 A9 AD 8C AF D2 AC` | `direct-schema` | `H[0]=0x02,H[1]=byte,H[2]=byte 0x00792A6F` | 0x00792A6F..0x00792AC7 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
