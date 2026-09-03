# Outer opcode 0x18 — exhaustive full payloads

> Số payload unique: **13**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 18 00` | `59 E9 AF AD B5 AD` | `exhaustive-selector-routing` | `H[0]=0x00 0x0079155D` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 18 01` | `59 E9 AF AD B5 AC` | `exhaustive-selector-routing` | `H[0]=0x01 0x00790F28` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 18 02` | `59 E9 AF AD B5 AF` | `exhaustive-selector-routing` | `H[0]=0x02 0x007910F5` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 18 03` | `59 E9 AF AD B5 AE` | `exhaustive-selector-routing` | `H[0]=0x03 0x007912C2` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 18 04` | `59 E9 AF AD B5 A9` | `exhaustive-selector-routing` | `H[0]=0x04 0x007912F1` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 18 05` | `59 E9 AF AD B5 A8` | `exhaustive-selector-routing` | `H[0]=0x05 0x00791441` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 18 06` | `59 E9 AF AD B5 AB` | `exhaustive-selector-routing` | `H[0]=0x06 0x0079152C` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 18 07` | `59 E9 AF AD B5 AA` | `exhaustive-selector-routing` | `H[0]=0x07 0x0079153D` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 18 08` | `59 E9 AF AD B5 A5` | `exhaustive-selector-routing` | `H[0]=0x08 0x0079154E` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 10 | `0x04` | 4 | `04 34 12` | `F4 44 04 00 18 04 34 12` | `59 E9 A9 AD B5 A9 99 BF` | `direct-schema` | `H[0]=0x04 0x007912F1` | 0x007912F1..0x0079143C | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 11 | `0x01` | 5 | `01 34 12 56` | `F4 44 05 00 18 01 34 12 56` | `59 E9 A8 AD B5 AC 99 BF FB` | `direct-schema` | `H[0]=0x01 0x00790F28` | route helper 0x00720CA8 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 12 | `0x02` | 5 | `02 34 12 56` | `F4 44 05 00 18 02 34 12 56` | `59 E9 A8 AD B5 AF 99 BF FB` | `direct-schema` | `H[0]=0x02 0x007910F5` | route helper 0x00720DF0 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 13 | `0x05` | 5 | `05 34 12 56` | `F4 44 05 00 18 05 34 12 56` | `59 E9 A8 AD B5 A8 99 BF FB` | `direct-schema` | `H[0]=0x05 0x00791441` | route helper 0x00721088 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
