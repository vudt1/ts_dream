# Outer opcode 0x2E — exhaustive full payloads

> Số payload unique: **3**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 2E 01` | `59 E9 AF AD 83 AC` | `branch-routing` | `H[0]=0x01 0x0079519E` | 0x00795190..0x007951D5 | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 2E 02` | `59 E9 AF AD 83 AF` | `branch-routing` | `H[0]=0x02 0x007951B2` | 0x00795190..0x007951D5 | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 2E 03` | `59 E9 AF AD 83 AE` | `branch-routing` | `H[0]=0x03 0x007951C6` | 0x00795190..0x007951D5 | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
