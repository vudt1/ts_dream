# Outer opcode 0x1A — exhaustive full payloads

> Số payload unique: **18**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 1A 00` | `59 E9 AF AD B7 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 1A 01` | `59 E9 AF AD B7 AC` | `selector-routing` | `H[0]=0x01 0x007917BA` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 1A 02` | `59 E9 AF AD B7 AF` | `selector-routing` | `H[0]=0x02 0x007918E3` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 1A 03` | `59 E9 AF AD B7 AE` | `selector-routing` | `H[0]=0x03 0x007919AF` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 1A 04` | `59 E9 AF AD B7 A9` | `selector-routing` | `H[0]=0x04 0x007919DE` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 1A 05` | `59 E9 AF AD B7 A8` | `selector-routing` | `H[0]=0x05 0x007919F2` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 1A 06` | `59 E9 AF AD B7 AB` | `selector-routing` | `H[0]=0x06 0x00791A30` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 1A 07` | `59 E9 AF AD B7 AA` | `selector-routing` | `H[0]=0x07 0x00791A6E` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 1A 08` | `59 E9 AF AD B7 A5` | `selector-routing` | `H[0]=0x08 0x00791A82` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 1A 09` | `59 E9 AF AD B7 A4` | `selector-routing` | `H[0]=0x09 0x00791B93` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 1A 0A` | `59 E9 AF AD B7 A7` | `selector-routing` | `H[0]=0x0A 0x00791C93` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 12 | `0x09` | 3 | `09 7F` | `F4 44 03 00 1A 09 7F` | `59 E9 AE AD B7 A4 D2` | `direct-schema` | `H[0]=0x09,H[1]=byte 0x00791B93` | 0x00791B93..0x00791C8E | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 13 | `0x08` | 4 | `08 34 12` | `F4 44 04 00 1A 08 34 12` | `59 E9 A9 AD B7 A5 99 BF` | `direct-schema` | `H[0]=0x08,H[1..2]=UInt16LE 0x00791A82` | 0x00791A82..0x00791B8E | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 14 | `0x0A` | 4 | `0A 34 12` | `F4 44 04 00 1A 0A 34 12` | `59 E9 A9 AD B7 A7 99 BF` | `direct-schema` | `H[0]=0x0A,H[1..2]=UInt16LE 0x00791C93` | 0x00791C93..0x00791D7B | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 15 | `0x01` | 6 | `01 44 33 22 11` | `F4 44 06 00 1A 01 44 33 22 11` | `59 E9 AB AD B7 AC E9 9E 8F BC` | `direct-schema` | `H[0]=0x01,H[1..4]=UInt32LE 0x007917BA` | 0x007917BA..0x00791A69 | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 16 | `0x02` | 6 | `02 44 33 22 11` | `F4 44 06 00 1A 02 44 33 22 11` | `59 E9 AB AD B7 AF E9 9E 8F BC` | `direct-schema` | `H[0]=0x02,H[1..4]=UInt32LE 0x007918E3` | 0x007917BA..0x00791A69 | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 17 | `0x05` | 6 | `05 44 33 22 11` | `F4 44 06 00 1A 05 44 33 22 11` | `59 E9 AB AD B7 A8 E9 9E 8F BC` | `direct-schema` | `H[0]=0x05,H[1..4]=UInt32LE 0x007919F2` | 0x007917BA..0x00791A69 | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 18 | `0x06` | 6 | `06 44 33 22 11` | `F4 44 06 00 1A 06 44 33 22 11` | `59 E9 AB AD B7 AB E9 9E 8F BC` | `direct-schema` | `H[0]=0x06,H[1..4]=UInt32LE 0x00791A30` | 0x007917BA..0x00791A69 | schema-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
