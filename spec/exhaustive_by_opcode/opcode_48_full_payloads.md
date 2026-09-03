# Outer opcode 0x48 — exhaustive full payloads

> Số payload unique: **7**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 48 00` | `59 E9 AF AD E5 AD` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x00 0x00796347` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 48 01` | `59 E9 AF AD E5 AC` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x01 0x00796293` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 48 02` | `59 E9 AF AD E5 AF` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x02 0x007962A7` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 48 03` | `59 E9 AF AD E5 AE` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x03 0x007962BB` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 48 04` | `59 E9 AF AD E5 A9` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x04 0x007962CC` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 48 05` | `59 E9 AF AD E5 A8` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x05 0x007962DA` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 48 06` | `59 E9 AF AD E5 AB` | `legacy-unknown || selector-routing` | `0x48: outer selector L1 || H[0]=0x06 0x007962EB` | 0x796267 cmp; table 0x796277 // L1 table for 48 | high-routing | `deep_selector_study_vectors.json, fable_deep_vectors_37_48_C7.json` |
