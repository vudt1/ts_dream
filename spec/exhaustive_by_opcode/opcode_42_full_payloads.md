# Outer opcode 0x42 — exhaustive full payloads

> Số payload unique: **13**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 42 01` | `59 E9 AF AD EF AC` | `premap-routing` | `H[0]=0x01 -> premap index 0x01 0x00795E44` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 42 02` | `59 E9 AF AD EF AF` | `premap-routing` | `H[0]=0x02 -> premap index 0x02 0x00795E58` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 42 03` | `59 E9 AF AD EF AE` | `premap-routing` | `H[0]=0x03 -> premap index 0x03 0x00795E6C` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x04` | 2 | `04` | `F4 44 02 00 42 04` | `59 E9 AF AD EF A9` | `premap-routing` | `H[0]=0x04 -> premap index 0x04 0x00795E80` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x0B` | 2 | `0B` | `F4 44 02 00 42 0B` | `59 E9 AF AD EF A6` | `premap-routing` | `H[0]=0x0B -> premap index 0x05 0x00795E94` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 6 | `0x0C` | 2 | `0C` | `F4 44 02 00 42 0C` | `59 E9 AF AD EF A1` | `premap-routing` | `H[0]=0x0C -> premap index 0x06 0x00795EA8` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 7 | `0x0E` | 2 | `0E` | `F4 44 02 00 42 0E` | `59 E9 AF AD EF A3` | `premap-routing` | `H[0]=0x0E -> premap index 0x07 0x00795EBC` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 8 | `0x15` | 2 | `15` | `F4 44 02 00 42 15` | `59 E9 AF AD EF B8` | `premap-routing` | `H[0]=0x15 -> premap index 0x08 0x00795ED0` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 9 | `0x16` | 2 | `16` | `F4 44 02 00 42 16` | `59 E9 AF AD EF BB` | `premap-routing` | `H[0]=0x16 -> premap index 0x09 0x00795EE4` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 10 | `0x17` | 2 | `17` | `F4 44 02 00 42 17` | `59 E9 AF AD EF BA` | `premap-routing` | `H[0]=0x17 -> premap index 0x0A 0x00795EF8` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 11 | `0x18` | 2 | `18` | `F4 44 02 00 42 18` | `59 E9 AF AD EF B5` | `premap-routing` | `H[0]=0x18 -> premap index 0x0B 0x00795F0C` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 12 | `0x1F` | 2 | `1F` | `F4 44 02 00 42 1F` | `59 E9 AF AD EF B2` | `premap-routing` | `H[0]=0x1F -> premap index 0x0C 0x00795F20` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 13 | `0x20` | 2 | `20` | `F4 44 02 00 42 20` | `59 E9 AF AD EF 8D` | `premap-routing` | `H[0]=0x20 -> premap index 0x0D 0x00795F34` | premap 0x00795DEB + table 0x00795E0C | routing-high | `fable_deep_vectors_37_48_C7.json` |
