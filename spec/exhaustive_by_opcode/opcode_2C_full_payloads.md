# Outer opcode 0x2C — exhaustive full payloads

> Số payload unique: **3**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x01` | 2 | `01` | `F4 44 02 00 2C 01` | `59 E9 AF AD 81 AC` | `outer-route || branch-routing` | `outer-route || H[0]=0x01 0x0079493D -> 0x0055DA08(H)` | H[0] selector only // 0x0079492F..0x00794972 | high-routing | `fable_deep_vectors_28_29_2A_2B_2C.json, gameplay_opcode_study_vectors.json` |
| 2 | `0x02` | 2 | `02` | `F4 44 02 00 2C 02` | `59 E9 AF AD 81 AF` | `outer-route || branch-routing` | `outer-route || H[0]=0x02 0x00794951 -> 0x0055DFAC(DL=1)` | H[0] selector only // 0x0079492F..0x00794972 | high-routing | `fable_deep_vectors_28_29_2A_2B_2C.json, gameplay_opcode_study_vectors.json` |
| 3 | `0x03` | 2 | `03` | `F4 44 02 00 2C 03` | `59 E9 AF AD 81 AE` | `outer-route || branch-routing` | `outer-route || H[0]=0x03 0x00794964 -> 0x0055DFAC(DL=2)` | H[0] selector only // 0x0079492F..0x00794972 | high-routing | `fable_deep_vectors_28_29_2A_2B_2C.json, gameplay_opcode_study_vectors.json` |
