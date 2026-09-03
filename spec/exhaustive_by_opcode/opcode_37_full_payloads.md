# Outer opcode 0x37 — exhaustive full payloads

> Số payload unique: **3**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 37 01` | `59 E9 AF AD 9A AC` | `branch-routing` | `H[0]=01 0x007954CF` | 0x7954C5..0x7954DB | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x02` | 3 | `02 01` | `F4 44 03 00 37 02 01` | `59 E9 AE AD 9A AF AC` | `nested-selector-routing` | `H[0]=02,H[1]=0x01 0x00795505` | 0x7954E0..0x795550 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 3 | `02 02` | `F4 44 03 00 37 02 02` | `59 E9 AE AD 9A AF AF` | `nested-selector-routing` | `H[0]=02,H[1]=0x02 0x00795527` | 0x7954E0..0x795550 | routing-high | `fable_deep_vectors_37_48_C7.json` |
