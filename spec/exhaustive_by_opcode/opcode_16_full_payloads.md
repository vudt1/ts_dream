# Outer opcode 0x16 — exhaustive full payloads

> Số payload unique: **15**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 16 00` | `59 E9 AF AD BB AD` | `exhaustive-selector-routing` | `H[0]=0x00 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 16 01` | `59 E9 AF AD BB AC` | `exhaustive-selector-routing` | `H[0]=0x01 0x0078FF0A` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 16 02` | `59 E9 AF AD BB AF` | `exhaustive-selector-routing` | `H[0]=0x02 0x0078FF1E` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 16 03` | `59 E9 AF AD BB AE` | `exhaustive-selector-routing` | `H[0]=0x03 0x0078FFE8` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 16 04` | `59 E9 AF AD BB A9` | `exhaustive-selector-routing` | `H[0]=0x04 0x0079014A` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 16 05` | `59 E9 AF AD BB A8` | `exhaustive-selector-routing` | `H[0]=0x05 0x0079015E` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 16 06` | `59 E9 AF AD BB AB` | `exhaustive-selector-routing` | `H[0]=0x06 0x00790228` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 16 07` | `59 E9 AF AD BB AA` | `exhaustive-selector-routing` | `H[0]=0x07 0x007902AB` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 16 08` | `59 E9 AF AD BB A5` | `exhaustive-selector-routing` | `H[0]=0x08 0x007902BF` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 16 09` | `59 E9 AF AD BB A4` | `exhaustive-selector-routing` | `H[0]=0x09 0x007902D3` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 16 0A` | `59 E9 AF AD BB A7` | `exhaustive-selector-routing` | `H[0]=0x0A 0x007902E7` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 12 | `0x06` | 5 | `06 01 00 7F` | `F4 44 05 00 16 06 01 00 7F` | `59 E9 A8 AD BB AB AC AD D2` | `direct-schema` | `H[0]=0x06 0x00790228` | 0x00790228..0x007902A6 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 13 | `0x03` | 6 | `03 01 00 02 00` | `F4 44 06 00 16 03 01 00 02 00` | `59 E9 AB AD BB AE AC AD AF AD` | `direct-schema` | `H[0]=0x03 0x0078FFE8` | 0x0078FFE8..0x00790145 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 14 | `0x02` | 8 | `02 01 00 02 00 03 00` | `F4 44 08 00 16 02 01 00 02 00 03 00` | `59 E9 A5 AD BB AF AC AD AF AD AE AD` | `direct-schema` | `H[0]=0x02 0x0078FF1E` | 0x0078FF1E..0x0078FFE3 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
| 15 | `0x05` | 8 | `05 01 00 02 00 03 00` | `F4 44 08 00 16 05 01 00 02 00 03 00` | `59 E9 A5 AD BB A8 AC AD AF AD AE AD` | `direct-schema` | `H[0]=0x05 0x0079015E` | 0x0079015E..0x00790223 | schema-high | `fable_deep_vectors_0F_10_14_16_17_18.json` |
