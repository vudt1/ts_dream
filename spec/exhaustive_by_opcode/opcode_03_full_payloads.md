# Outer opcode 0x03 — exhaustive full payloads

> Số payload unique: **2**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x40` | 27 | `40 30 20 10 11 22 33 02 01 04 03 06 05 08 07 44 44 33 22 11 88 77 66 55 00 00` | `F4 44 1B 00 03 40 30 20 10 11 22 33 02 01 04 03 06 05 08 07 44 44 33 22 11 88 77 66 55 00 00` | `59 E9 B6 AD AE ED 9D 8D BD BC 8F 9E AF AC A9 AE AB A8 A5 AA E9 E9 9E 8F BC 25 DA CB F8 AD AD` | `direct-schema` | `linear parser 0x0078BC95` | 0x0078BCC5..0x0078C0D9: fixed scalar prefix; H[24] is count-like. | schema-high | `fable_deep_vectors_01_02_03_04_05_07.json` |
| 2 | `0x40` | 31 | `40 30 20 10 11 22 33 02 01 04 03 06 05 08 07 44 44 33 22 11 88 77 66 55 02 11 11 22 22 00` | `F4 44 1F 00 03 40 30 20 10 11 22 33 02 01 04 03 06 05 08 07 44 44 33 22 11 88 77 66 55 02 11 11 22 22 00` | `59 E9 B2 AD AE ED 9D 8D BD BC 8F 9E AF AC A9 AE AB A8 A5 AA E9 E9 9E 8F BC 25 DA CB F8 AF BC BC 8F 8F AD` | `direct-schema-repeated` | `linear parser; count-like H[24]=2 0x0078BC95` | 0x0078C101..0x0078C1D7: runtime loop reads two-byte entries at H[25+2*i]. | schema-high-for-loop-prefix | `fable_deep_vectors_01_02_03_04_05_07.json` |
