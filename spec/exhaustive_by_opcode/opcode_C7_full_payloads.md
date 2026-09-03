# Outer opcode 0xC7 — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x04` | 2 | `04` | `F4 44 02 00 C7 04` | `59 E9 AF AD 6A A9` | `branch-routing` | `H[0]=0x04 0x00796327` | 0x79631B..0x796345 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x06` | 2 | `06` | `F4 44 02 00 C7 06` | `59 E9 AF AD 6A AB` | `branch-routing` | `H[0]=0x06 0x00796338` | 0x79631B..0x796345 | routing-high | `fable_deep_vectors_37_48_C7.json` |
