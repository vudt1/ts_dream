# Outer opcode 0x3F — exhaustive full payloads

> Số payload unique: **23**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 3F 00` | `59 E9 AF AD 92 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 3F 01` | `59 E9 AF AD 92 AC` | `selector-routing` | `H[0]=0x01 0x00795B4F` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 3F 02` | `59 E9 AF AD 92 AF` | `selector-routing` | `H[0]=0x02 0x00795B63` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 3F 03` | `59 E9 AF AD 92 AE` | `selector-routing` | `H[0]=0x03 0x00795B77` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 3F 04` | `59 E9 AF AD 92 A9` | `selector-routing` | `H[0]=0x04 0x00795B8B` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 3F 05` | `59 E9 AF AD 92 A8` | `selector-routing` | `H[0]=0x05 0x00795B9F` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 3F 06` | `59 E9 AF AD 92 AB` | `selector-routing` | `H[0]=0x06 0x00795BB3` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 3F 07` | `59 E9 AF AD 92 AA` | `selector-routing` | `H[0]=0x07 0x00795BC7` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 3F 08` | `59 E9 AF AD 92 A5` | `selector-routing` | `H[0]=0x08 0x00795BDB` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 3F 09` | `59 E9 AF AD 92 A4` | `selector-routing` | `H[0]=0x09 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 3F 0A` | `59 E9 AF AD 92 A7` | `selector-routing` | `H[0]=0x0A 0x00795BEF` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 3F 0B` | `59 E9 AF AD 92 A6` | `selector-routing` | `H[0]=0x0B 0x00795C03` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 13 | `0x0C` | 2 | `0C` | `F4 44 02 00 3F 0C` | `59 E9 AF AD 92 A1` | `selector-routing` | `H[0]=0x0C 0x00795C17` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 14 | `0x0D` | 2 | `0D` | `F4 44 02 00 3F 0D` | `59 E9 AF AD 92 A0` | `selector-routing` | `H[0]=0x0D 0x00795C2B` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 15 | `0x0E` | 2 | `0E` | `F4 44 02 00 3F 0E` | `59 E9 AF AD 92 A3` | `selector-routing` | `H[0]=0x0E 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 16 | `0x0F` | 2 | `0F` | `F4 44 02 00 3F 0F` | `59 E9 AF AD 92 A2` | `selector-routing` | `H[0]=0x0F 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 17 | `0x10` | 2 | `10` | `F4 44 02 00 3F 10` | `59 E9 AF AD 92 BD` | `selector-routing` | `H[0]=0x10 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 18 | `0x11` | 2 | `11` | `F4 44 02 00 3F 11` | `59 E9 AF AD 92 BC` | `selector-routing` | `H[0]=0x11 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 19 | `0x12` | 2 | `12` | `F4 44 02 00 3F 12` | `59 E9 AF AD 92 BF` | `selector-routing` | `H[0]=0x12 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 20 | `0x13` | 2 | `13` | `F4 44 02 00 3F 13` | `59 E9 AF AD 92 BE` | `selector-routing` | `H[0]=0x13 0x00796347` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 21 | `0x14` | 2 | `14` | `F4 44 02 00 3F 14` | `59 E9 AF AD 92 B9` | `selector-routing` | `H[0]=0x14 0x00795C3F` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 22 | `0x15` | 2 | `15` | `F4 44 02 00 3F 15` | `59 E9 AF AD 92 B8` | `selector-routing` | `H[0]=0x15 0x00795C53` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 23 | `0x16` | 2 | `16` | `F4 44 02 00 3F 16` | `59 E9 AF AD 92 BB` | `selector-routing` | `H[0]=0x16 0x00795C67` | L1 table for 3F | routing-high | `fable_deep_vectors_37_48_C7.json` |
