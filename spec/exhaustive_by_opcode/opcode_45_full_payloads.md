# Outer opcode 0x45 — exhaustive full payloads

> Số payload unique: **12**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 45 00` | `59 E9 AF AD E8 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 45 01` | `59 E9 AF AD E8 AC` | `selector-routing` | `H[0]=0x01 0x00795FE1` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 45 02` | `59 E9 AF AD E8 AF` | `selector-routing` | `H[0]=0x02 0x00795FF5` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 45 03` | `59 E9 AF AD E8 AE` | `selector-routing` | `H[0]=0x03 0x00796009` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 45 04` | `59 E9 AF AD E8 A9` | `selector-routing` | `H[0]=0x04 0x0079601D` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 45 05` | `59 E9 AF AD E8 A8` | `selector-routing` | `H[0]=0x05 0x00796031` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 45 06` | `59 E9 AF AD E8 AB` | `selector-routing` | `H[0]=0x06 0x00796045` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 45 07` | `59 E9 AF AD E8 AA` | `selector-routing` | `H[0]=0x07 0x00796059` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 45 08` | `59 E9 AF AD E8 A5` | `selector-routing` | `H[0]=0x08 0x0079606D` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 45 09` | `59 E9 AF AD E8 A4` | `selector-routing` | `H[0]=0x09 0x00796081` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 45 0A` | `59 E9 AF AD E8 A7` | `selector-routing` | `H[0]=0x0A 0x00796095` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 45 0B` | `59 E9 AF AD E8 A6` | `selector-routing` | `H[0]=0x0B 0x007960A9` | L1 table for 45 | routing-high | `fable_deep_vectors_37_48_C7.json` |
