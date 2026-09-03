# Outer opcode 0x41 — exhaustive full payloads

> Số payload unique: **8**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 41 00` | `59 E9 AF AD EC AD` | `selector-routing` | `H[0]=0x00 0x00796347` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 41 01` | `59 E9 AF AD EC AC` | `selector-routing` | `H[0]=0x01 0x00795D33` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 41 02` | `59 E9 AF AD EC AF` | `selector-routing` | `H[0]=0x02 0x00795D44` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 41 03` | `59 E9 AF AD EC AE` | `selector-routing` | `H[0]=0x03 0x00795D55` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 41 04` | `59 E9 AF AD EC A9` | `selector-routing` | `H[0]=0x04 0x00795D69` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 41 05` | `59 E9 AF AD EC A8` | `selector-routing` | `H[0]=0x05 0x00795D7D` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 41 06` | `59 E9 AF AD EC AB` | `selector-routing` | `H[0]=0x06 0x00795D8E` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 41 07` | `59 E9 AF AD EC AA` | `selector-routing` | `H[0]=0x07 0x00795DA2` | L1 table for 41 | routing-high | `fable_deep_vectors_37_48_C7.json` |
