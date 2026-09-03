# Outer opcode 0x1B — exhaustive full payloads

> Số payload unique: **15**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x03` | 2 | `03` | `F4 44 02 00 1B 03` | `59 E9 AF AD B6 AE` | `direct-route` | `H[0]=0x03 0x00791F65` | 0x00791F65..0x00791F71 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 2 | `0x01` | 3 | `01 00` | `F4 44 03 00 1B 01 00` | `59 E9 AE AD B6 AC AD` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x00 0x00791ECE` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 3 | `0x01` | 3 | `01 01` | `F4 44 03 00 1B 01 01` | `59 E9 AE AD B6 AC AC` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x01 0x00791EDD` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 4 | `0x01` | 3 | `01 02` | `F4 44 03 00 1B 01 02` | `59 E9 AE AD B6 AC AF` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x02 0x00791EEC` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 5 | `0x01` | 3 | `01 03` | `F4 44 03 00 1B 01 03` | `59 E9 AE AD B6 AC AE` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x03 0x00791EFB` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 6 | `0x01` | 3 | `01 04` | `F4 44 03 00 1B 01 04` | `59 E9 AE AD B6 AC A9` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x04 0x00791F0A` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 7 | `0x01` | 3 | `01 05` | `F4 44 03 00 1B 01 05` | `59 E9 AE AD B6 AC A8` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x05 0x00791F19` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 8 | `0x01` | 3 | `01 06` | `F4 44 03 00 1B 01 06` | `59 E9 AE AD B6 AC AB` | `nested-selector-routing` | `H[0]=0x01,H[1]=0x06 0x00791F26` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 9 | `0x02` | 3 | `02 00` | `F4 44 03 00 1B 02 00` | `59 E9 AE AD B6 AF AD` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x00 0x00791ECE` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 10 | `0x02` | 3 | `02 01` | `F4 44 03 00 1B 02 01` | `59 E9 AE AD B6 AF AC` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x01 0x00791EDD` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 11 | `0x02` | 3 | `02 02` | `F4 44 03 00 1B 02 02` | `59 E9 AE AD B6 AF AF` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x02 0x00791EEC` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 12 | `0x02` | 3 | `02 03` | `F4 44 03 00 1B 02 03` | `59 E9 AE AD B6 AF AE` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x03 0x00791EFB` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 13 | `0x02` | 3 | `02 04` | `F4 44 03 00 1B 02 04` | `59 E9 AE AD B6 AF A9` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x04 0x00791F0A` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 14 | `0x02` | 3 | `02 05` | `F4 44 03 00 1B 02 05` | `59 E9 AE AD B6 AF A8` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x05 0x00791F19` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 15 | `0x02` | 3 | `02 06` | `F4 44 03 00 1B 02 06` | `59 E9 AE AD B6 AF AB` | `nested-selector-routing` | `H[0]=0x02,H[1]=0x06 0x00791F26` | 0x00791D9F..0x00791F60 | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
