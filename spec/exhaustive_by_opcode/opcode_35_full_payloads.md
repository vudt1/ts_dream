# Outer opcode 0x35 — exhaustive full payloads

> Số payload unique: **17**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 35 00` | `59 E9 AF AD 98 AD` | `selector-routing` | `H[0]=0x00 0x00796347` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 35 01` | `59 E9 AF AD 98 AC` | `selector-routing` | `H[0]=0x01 0x0079531A` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 35 02` | `59 E9 AF AD 98 AF` | `selector-routing` | `H[0]=0x02 0x00796347` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 35 03` | `59 E9 AF AD 98 AE` | `selector-routing` | `H[0]=0x03 0x00795342` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 35 04` | `59 E9 AF AD 98 A9` | `selector-routing` | `H[0]=0x04 0x00795356` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 35 05` | `59 E9 AF AD 98 A8` | `selector-routing` | `H[0]=0x05 0x0079536A` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 35 06` | `59 E9 AF AD 98 AB` | `selector-routing` | `H[0]=0x06 0x0079537E` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 35 07` | `59 E9 AF AD 98 AA` | `selector-routing` | `H[0]=0x07 0x00795392` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 35 08` | `59 E9 AF AD 98 A5` | `selector-routing` | `H[0]=0x08 0x007953A6` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 35 09` | `59 E9 AF AD 98 A4` | `selector-routing` | `H[0]=0x09 0x007953BA` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 35 0A` | `59 E9 AF AD 98 A7` | `selector-routing` | `H[0]=0x0A 0x007953CE` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 35 0B` | `59 E9 AF AD 98 A6` | `selector-routing` | `H[0]=0x0B 0x007953E2` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 13 | `0x0C` | 2 | `0C` | `F4 44 02 00 35 0C` | `59 E9 AF AD 98 A1` | `selector-routing` | `H[0]=0x0C 0x007953F6` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 14 | `0x0D` | 2 | `0D` | `F4 44 02 00 35 0D` | `59 E9 AF AD 98 A0` | `selector-routing` | `H[0]=0x0D 0x0079540A` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 15 | `0x0E` | 2 | `0E` | `F4 44 02 00 35 0E` | `59 E9 AF AD 98 A3` | `selector-routing` | `H[0]=0x0E 0x0079541E` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 16 | `0x0F` | 2 | `0F` | `F4 44 02 00 35 0F` | `59 E9 AF AD 98 A2` | `selector-routing` | `H[0]=0x0F 0x00795432` | L1 table 0x007952DA | routing-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
| 17 | `0x0F` | 3 | `0F 02` | `F4 44 03 00 35 0F 02` | `59 E9 AE AD 98 A2 AF` | `shared-tail-schema` | `H[0]=0x0F,H[1]=byte 0x00795446` | 0x00795446..0x0079548F | schema-high | `fable_deep_vectors_2D_2E_34_35_36.json` |
