# Outer opcode 0x1E — exhaustive full payloads

> Số payload unique: **14**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 1E 00` | `59 E9 AF AD B3 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 1E 01` | `59 E9 AF AD B3 AC` | `selector-routing` | `H[0]=0x01 0x0079220D` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 1E 02` | `59 E9 AF AD B3 AF` | `selector-routing` | `H[0]=0x02 0x00792221` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 1E 03` | `59 E9 AF AD B3 AE` | `selector-routing` | `H[0]=0x03 0x00792235` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 1E 04` | `59 E9 AF AD B3 A9` | `selector-routing` | `H[0]=0x04 0x00792249` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 1E 05` | `59 E9 AF AD B3 A8` | `selector-routing` | `H[0]=0x05 0x0079225D` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 1E 06` | `59 E9 AF AD B3 AB` | `selector-routing` | `H[0]=0x06 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 1E 07` | `59 E9 AF AD B3 AA` | `selector-routing` | `H[0]=0x07 0x00796347` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 1E 08` | `59 E9 AF AD B3 A5` | `selector-routing` | `H[0]=0x08 0x00792271` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 1E 09` | `59 E9 AF AD B3 A4` | `selector-routing` | `H[0]=0x09 0x00792282` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 1E 0A` | `59 E9 AF AD B3 A7` | `selector-routing` | `H[0]=0x0A 0x00792296` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 1E 0B` | `59 E9 AF AD B3 A6` | `selector-routing` | `H[0]=0x0B 0x007922AA` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 13 | `0x0C` | 2 | `0C` | `F4 44 02 00 1E 0C` | `59 E9 AF AD B3 A1` | `selector-routing` | `H[0]=0x0C 0x007922BE` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
| 14 | `0x0D` | 2 | `0D` | `F4 44 02 00 1E 0D` | `59 E9 AF AD B3 A0` | `selector-routing` | `H[0]=0x0D 0x007922D2` | PE-decoded local table plus r2 guard/jump confirmation | routing-high | `fable_deep_vectors_19_1A_1B_1D_1E.json` |
