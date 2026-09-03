# Outer opcode 0x36 — exhaustive full payloads

> Số payload unique: **1**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x7F` | 3 | `7F 02` | `F4 44 03 00 36 7F 02` | `59 E9 AE AD 9B D2 AF` | `shared-tail-schema` | `len(H)>0; H[0]=byte,H[1]=byte 0x00795446` | 0x00795494..0x007954A0 + shared tail | schema-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
