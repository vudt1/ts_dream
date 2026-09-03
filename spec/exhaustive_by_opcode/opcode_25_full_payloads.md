# Outer opcode 0x25 — exhaustive full payloads

> Số payload unique: **12**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 25 00` | `59 E9 AF AD 88 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 25 01` | `59 E9 AF AD 88 AC` | `selector-routing` | `H[0]=0x01 0x00793811` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 25 02` | `59 E9 AF AD 88 AF` | `selector-routing` | `H[0]=0x02 0x00793825` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 25 03` | `59 E9 AF AD 88 AE` | `selector-routing` | `H[0]=0x03 0x00793839` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 25 04` | `59 E9 AF AD 88 A9` | `selector-routing || callee-schema` | `H[0]=0x04 0x0079384D` | PE decoded jump table plus r2 guard/jump // callee 0x7281A8 | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 25 05` | `59 E9 AF AD 88 A8` | `selector-routing` | `H[0]=0x05 0x00793861` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 25 06` | `59 E9 AF AD 88 AB` | `selector-routing` | `H[0]=0x06 0x00793875` | PE decoded jump table plus r2 guard/jump | routing-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 8 | `0x03` | 3 | `03 0A` | `F4 44 03 00 25 03 0A` | `59 E9 AE AD 88 AE A7` | `callee-schema` | `H[0]=0x03,H[1]=10 0x00793839` | callee 0x729430 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 9 | `0x02` | 7 | `02 44 33 22 11 7F` | `F4 44 07 00 25 02 44 33 22 11 7F` | `59 E9 AA AD 88 AF E9 9E 8F BC D2` | `callee-schema` | `H[0]=0x02,UInt32LE,byte 0x00793825` | callee 0x729D24 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 10 | `0x06` | 7 | `06 44 33 22 11 7F` | `F4 44 07 00 25 06 44 33 22 11 7F` | `59 E9 AA AD 88 AB E9 9E 8F BC D2` | `callee-schema` | `H[0]=0x06,UInt32LE,byte 0x00793875` | callee 0x729964 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 11 | `0x01` | 8 | `01 44 33 22 11 66 55` | `F4 44 08 00 25 01 44 33 22 11 66 55` | `59 E9 A5 AD 88 AC E9 9E 8F BC CB F8` | `callee-schema` | `H[0]=0x01,UInt32LE,UInt16LE 0x00793811` | callee 0x729E48 | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
| 12 | `0x05` | 30 | `05 01 00 00 00 02 00 00 00 01 00 00 00 11 11 11 11 22 22 22 22 33 33 33 33 44 44 44 44` | `F4 44 1E 00 25 05 01 00 00 00 02 00 00 00 01 00 00 00 11 11 11 11 22 22 22 22 33 33 33 33 44 44 44 44` | `59 E9 B3 AD 88 A8 AC AD AD AD AF AD AD AD AC AD AD AD BC BC BC BC 8F 8F 8F 8F 9E 9E 9E 9E E9 E9 E9 E9` | `dynamic-group-schema` | `H[0]=0x05; counts [1,2,1]; 4 grouped UInt32LE 0x00793861` | callee 0x727F4C | schema-high | `fable_deep_vectors_1F_20_21_22_23_24_25.json` |
