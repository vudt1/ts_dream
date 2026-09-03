# Outer opcode 0x1D — exhaustive full payloads

> Số payload unique: **12**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 1D 00` | `59 E9 AF AD B0 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 1D 01` | `59 E9 AF AD B0 AC` | `selector-routing` | `H[0]=0x01 0x00791FCD` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 1D 02` | `59 E9 AF AD B0 AF` | `selector-routing` | `H[0]=0x02 0x0079206A` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 1D 03` | `59 E9 AF AD B0 AE` | `selector-routing || direct-state-route` | `H[0]=0x03 0x00792107` | PE-decoded local table plus r2 guard/jump confirmation // 0x00792107..0x0079218D (fixed message) | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 1D 04` | `59 E9 AF AD B0 A9` | `selector-routing` | `H[0]=0x04 0x00792136` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 1D 05` | `59 E9 AF AD B0 A8` | `selector-routing` | `H[0]=0x05 0x0079214A` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 1D 06` | `59 E9 AF AD B0 AB` | `selector-routing` | `H[0]=0x06 0x0079215B` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 1D 07` | `59 E9 AF AD B0 AA` | `selector-routing || direct-state-route` | `H[0]=0x07 0x0079216C` | PE-decoded local table plus r2 guard/jump confirmation // 0x00792107..0x0079218D (global +0x11C = 0) | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 1D 08` | `59 E9 AF AD B0 A5` | `selector-routing || direct-state-route` | `H[0]=0x08 0x0079217F` | PE-decoded local table plus r2 guard/jump confirmation // 0x00792107..0x0079218D (global +0x11D = 0) | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 1D 09` | `59 E9 AF AD B0 A4` | `selector-routing` | `H[0]=0x09 0x00792192` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 11 | `0x01` | 6 | `01 44 33 22 11` | `F4 44 06 00 1D 01 44 33 22 11` | `59 E9 AB AD B0 AC E9 9E 8F BC` | `direct-schema` | `H[0]=0x01,H[1..4]=UInt32LE 0x00791FCD` | route 0x01 scalar block | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 12 | `0x02` | 6 | `02 44 33 22 11` | `F4 44 06 00 1D 02 44 33 22 11` | `59 E9 AB AD B0 AF E9 9E 8F BC` | `direct-schema` | `H[0]=0x02,H[1..4]=UInt32LE 0x0079206A` | route 0x02 scalar block | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
