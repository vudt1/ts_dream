# Outer opcode 0x46 — exhaustive full payloads

> Số payload unique: **10**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 46 00` | `59 E9 AF AD EB AD` | `selector-routing` | `H[0]=0x00 0x00796347` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 46 01` | `59 E9 AF AD EB AC` | `selector-routing` | `H[0]=0x01 0x00796110` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 46 02` | `59 E9 AF AD EB AF` | `selector-routing` | `H[0]=0x02 0x00796124` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 46 03` | `59 E9 AF AD EB AE` | `selector-routing` | `H[0]=0x03 0x00796135` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 46 04` | `59 E9 AF AD EB A9` | `selector-routing` | `H[0]=0x04 0x00796149` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 46 05` | `59 E9 AF AD EB A8` | `selector-routing` | `H[0]=0x05 0x0079615D` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 46 06` | `59 E9 AF AD EB AB` | `selector-routing` | `H[0]=0x06 0x00796171` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 46 07` | `59 E9 AF AD EB AA` | `selector-routing` | `H[0]=0x07 0x00796185` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 46 08` | `59 E9 AF AD EB A5` | `selector-routing` | `H[0]=0x08 0x00796196` | L1 table for 46 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 10 | `0x08` | 3 | `08 7F` | `F4 44 03 00 46 08 7F` | `59 E9 AE AD EB A5 D2` | `direct-schema` | `H[0]=08,H[1]=byte 0x00796196` | 0x796196..0x7961BC | schema-high | `fable_deep_vectors_37_48_C7.json` |
