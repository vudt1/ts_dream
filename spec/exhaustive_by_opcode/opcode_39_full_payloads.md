# Outer opcode 0x39 — exhaustive full payloads

> Số payload unique: **5**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x02` | 2 | `02` | `F4 44 02 00 39 02` | `59 E9 AF AD 94 AF` | `branch-routing` | `H[0]=02 0x0079563F` | 0x79563F..0x79564B | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 2 | `0x03` | 3 | `03 01` | `F4 44 03 00 39 03 01` | `59 E9 AE AD 94 AE AC` | `nested-selector-routing` | `H[0]=03,H[1]=0x01 0x00795675` | 0x795650..0x7956B4 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 3 | `0x03` | 3 | `03 02` | `F4 44 03 00 39 03 02` | `59 E9 AE AD 94 AE AF` | `nested-selector-routing` | `H[0]=03,H[1]=0x02 0x00795697` | 0x795650..0x7956B4 | routing-high | `fable_deep_vectors_37_48_C7.json` |
| 4 | `0x01` | 4 | `01 03 01` | `F4 44 04 00 39 01 03 01` | `59 E9 A9 AD 94 AC AE AC` | `nested-selector-schema` | `H[0]=01,H[1]=03,H[2]=0x01 0x00795619` | 0x7955AE..0x79563A | schema-high | `fable_deep_vectors_37_48_C7.json` |
| 5 | `0x01` | 4 | `01 03 02` | `F4 44 04 00 39 01 03 02` | `59 E9 A9 AD 94 AC AE AF` | `nested-selector-schema` | `H[0]=01,H[1]=03,H[2]=0x02 0x0079562C` | 0x7955AE..0x79563A | schema-high | `fable_deep_vectors_37_48_C7.json` |
