# Outer opcode 0x1F — exhaustive full payloads

> Số payload unique: **23**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 1F 00` | `59 E9 AF AD B2 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 1F 01` | `59 E9 AF AD B2 AC` | `selector-routing` | `H[0]=0x01 0x00792351` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 1F 02` | `59 E9 AF AD B2 AF` | `selector-routing` | `H[0]=0x02 0x007923FA` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 1F 03` | `59 E9 AF AD B2 AE` | `selector-routing` | `H[0]=0x03 0x007925AB` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 1F 04` | `59 E9 AF AD B2 A9` | `selector-routing` | `H[0]=0x04 0x007925EC` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 1F 05` | `59 E9 AF AD B2 A8` | `selector-routing` | `H[0]=0x05 0x00792615` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 1F 06` | `59 E9 AF AD B2 AB` | `selector-routing` | `H[0]=0x06 0x00792656` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 1F 07` | `59 E9 AF AD B2 AA` | `selector-routing` | `H[0]=0x07 0x0079266A` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 1F 08` | `59 E9 AF AD B2 A5` | `selector-routing` | `H[0]=0x08 0x0079267B` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 1F 09` | `59 E9 AF AD B2 A4` | `selector-routing` | `H[0]=0x09 0x00792699` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 1F 0A` | `59 E9 AF AD B2 A7` | `selector-routing` | `H[0]=0x0A 0x007926AC` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 1F 0B` | `59 E9 AF AD B2 A6` | `selector-routing` | `H[0]=0x0B 0x007926D8` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 13 | `0x0C` | 2 | `0C` | `F4 44 02 00 1F 0C` | `59 E9 AF AD B2 A1` | `selector-routing` | `H[0]=0x0C 0x0079270B` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 14 | `0x0D` | 2 | `0D` | `F4 44 02 00 1F 0D` | `59 E9 AF AD B2 A0` | `selector-routing` | `H[0]=0x0D 0x00792824` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 15 | `0x0E` | 2 | `0E` | `F4 44 02 00 1F 0E` | `59 E9 AF AD B2 A3` | `selector-routing` | `H[0]=0x0E 0x00792846` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 16 | `0x01` | 3 | `01 00` | `F4 44 03 00 1F 01 00` | `59 E9 AE AD B2 AC AD` | `direct-schema` | `H[0]=0x01,H[1]=0x00 0x00792351` | 0x00792351..0x007923F5 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 17 | `0x01` | 3 | `01 01` | `F4 44 03 00 1F 01 01` | `59 E9 AE AD B2 AC AC` | `direct-schema` | `H[0]=0x01,H[1]=0x01 0x00792351` | 0x00792351..0x007923F5 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 18 | `0x01` | 3 | `01 02` | `F4 44 03 00 1F 01 02` | `59 E9 AE AD B2 AC AF` | `direct-schema` | `H[0]=0x01,H[1]=0x02 0x00792351` | 0x00792351..0x007923F5 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 19 | `0x04` | 3 | `04 7F` | `F4 44 03 00 1F 04 7F` | `59 E9 AE AD B2 A9 D2` | `direct-schema` | `H[0]=0x04,H[1]=byte 0x007925EC` | 0x007925EC..0x00792610 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 20 | `0x0B` | 3 | `0B 7F` | `F4 44 03 00 1F 0B 7F` | `59 E9 AE AD B2 A6 D2` | `direct-schema` | `H[0]=0x0B,H[1]=byte 0x007926D8` | 0x007926D8..0x00792706 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 21 | `0x03` | 4 | `03 7F 02` | `F4 44 04 00 1F 03 7F 02` | `59 E9 A9 AD B2 AE D2 AF` | `direct-schema` | `H[0]=0x03,H[1]=byte,H[2]=byte 0x007925AB` | 0x007925AB..0x007925E7 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 22 | `0x05` | 4 | `05 7F 02` | `F4 44 04 00 1F 05 7F 02` | `59 E9 A9 AD B2 A8 D2 AF` | `direct-schema` | `H[0]=0x05,H[1]=byte,H[2]=byte 0x00792615` | 0x00792615..0x00792651 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 23 | `0x02` | 6 | `02 44 33 22 11` | `F4 44 06 00 1F 02 44 33 22 11` | `59 E9 AB AD B2 AF E9 9E 8F BC` | `direct-schema` | `H[0]=0x02,H[1..4]=UInt32LE 0x007923FA` | 0x007923FA..0x0079253E | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
