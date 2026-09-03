# Outer opcode 0x3E — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 3E 01` | `59 E9 AF AD 93 AC` | `branch-routing` | `H[0]=0x01 0x00795A9C` | 0x795A92..0x795ABF | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 3E 02` | `59 E9 AF AD 93 AF` | `branch-routing` | `H[0]=0x02 0x00795AB0` | 0x795A92..0x795ABF | routing-high | `fable_deep_vectors_37_48_C7.json` |
