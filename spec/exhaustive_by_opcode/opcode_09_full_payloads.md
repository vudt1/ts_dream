# Outer opcode 0x09 — exhaustive full payloads

> Số payload unique: **8**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 09 00` | `59 E9 AF AD A4 AD` | `branch-ladder-default` | `H[0]=0x00 0x00796347` | 0x0078D4CB..0x0078D4E4 | routing-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 09 01` | `59 E9 AF AD A4 AC` | `branch-ladder` | `H[0]=0x01 0x0078D4E9` | 0x0078D4E9..0x0078D4F7 | routing-and-state-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 3 | `0x04` | 2 | `04` | `F4 44 02 00 09 04` | `59 E9 AF AD A4 A9` | `branch-ladder` | `H[0]=0x04 0x0078D5A9` | 0x0078D5A9..0x0078D5B8 | routing-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 4 | `0x05` | 2 | `05` | `F4 44 02 00 09 05` | `59 E9 AF AD A4 A8` | `branch-ladder` | `H[0]=0x05 0x0078D5BD` | 0x0078D5BD..0x0078D5CC | routing-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 5 | `0x03` | 3 | `03 00` | `F4 44 03 00 09 03 00` | `59 E9 AE AD A4 AE AD` | `branch-ladder` | `H[0]=0x03,H[1]=0 0x0078D52A` | 0x0078D4FC..0x0078D5A4 | schema-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 6 | `0x03` | 3 | `03 01` | `F4 44 03 00 09 03 01` | `59 E9 AE AD A4 AE AC` | `branch-ladder` | `H[0]=0x03,H[1]=1 0x0078D549` | 0x0078D4FC..0x0078D5A4 | schema-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 7 | `0x03` | 3 | `03 02` | `F4 44 03 00 09 03 02` | `59 E9 AE AD A4 AE AF` | `branch-ladder` | `H[0]=0x03,H[1]=2 0x0078D579` | 0x0078D4FC..0x0078D5A4 | schema-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
| 8 | `0x03` | 3 | `03 03` | `F4 44 03 00 09 03 03` | `59 E9 AE AD A4 AE AE` | `branch-ladder-default` | `H[0]=0x03,H[1]=0x03 0x00796347` | 0x0078D518..0x0078D525 | routing-high | `fable_deep_vectors_08_09_0C_0D_0E.json` |
