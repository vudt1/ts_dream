# Outer opcode 0x00 — exhaustive full payloads

> Số payload unique: **57**. Đây là các vector tổng hợp từ phân tích tĩnh, không phải traffic capture. `H[0]` ở offset frame `5`; full wire được XOR `0xAD`.

| # | H[0] | Payload length | H | Full decoded frame | Full wire XOR `0xAD` | Category | Route/target | Evidence | Confidence | Provenance |
|---:|---:|---:|---|---|---|---|---|---|---|---|
| 1 | `0x00` | 2 | `00` | `F4 44 02 00 00 00` | `59 E9 AF AD AD AD` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF97` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 2 | `0x01` | 2 | `01` | `F4 44 02 00 00 01` | `59 E9 AF AD AD AC` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ABD1` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 3 | `0x02` | 2 | `02` | `F4 44 02 00 00 02` | `59 E9 AF AD AD AF` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ABE3` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 4 | `0x03` | 2 | `03` | `F4 44 02 00 00 03` | `59 E9 AF AD AD AE` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ABF5` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 5 | `0x04` | 2 | `04` | `F4 44 02 00 00 04` | `59 E9 AF AD AD A9` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC07` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 6 | `0x05` | 2 | `05` | `F4 44 02 00 00 05` | `59 E9 AF AD AD A8` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC19` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 7 | `0x06` | 2 | `06` | `F4 44 02 00 00 06` | `59 E9 AF AD AD AB` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC2B` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 8 | `0x07` | 2 | `07` | `F4 44 02 00 00 07` | `59 E9 AF AD AD AA` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC3D` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 9 | `0x08` | 2 | `08` | `F4 44 02 00 00 08` | `59 E9 AF AD AD A5` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC4F` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 10 | `0x09` | 2 | `09` | `F4 44 02 00 00 09` | `59 E9 AF AD AD A4` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC61` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 11 | `0x0A` | 2 | `0A` | `F4 44 02 00 00 0A` | `59 E9 AF AD AD A7` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC73` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 12 | `0x0B` | 2 | `0B` | `F4 44 02 00 00 0B` | `59 E9 AF AD AD A6` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC85` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 13 | `0x0C` | 2 | `0C` | `F4 44 02 00 00 0C` | `59 E9 AF AD AD A1` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AC97` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 14 | `0x0D` | 2 | `0D` | `F4 44 02 00 00 0D` | `59 E9 AF AD AD A0` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ACA9` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 15 | `0x0E` | 2 | `0E` | `F4 44 02 00 00 0E` | `59 E9 AF AD AD A3` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ACBB` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 16 | `0x0F` | 2 | `0F` | `F4 44 02 00 00 0F` | `59 E9 AF AD AD A2` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ACCD` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 17 | `0x10` | 2 | `10` | `F4 44 02 00 00 10` | `59 E9 AF AD AD BD` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ACDF` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 18 | `0x11` | 2 | `11` | `F4 44 02 00 00 11` | `59 E9 AF AD AD BC` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ACF1` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 19 | `0x12` | 2 | `12` | `F4 44 02 00 00 12` | `59 E9 AF AD AD BF` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD03` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 20 | `0x13` | 2 | `13` | `F4 44 02 00 00 13` | `59 E9 AF AD AD BE` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD15` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 21 | `0x14` | 2 | `14` | `F4 44 02 00 00 14` | `59 E9 AF AD AD B9` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD27` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 22 | `0x15` | 2 | `15` | `F4 44 02 00 00 15` | `59 E9 AF AD AD B8` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD39` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 23 | `0x16` | 2 | `16` | `F4 44 02 00 00 16` | `59 E9 AF AD AD BB` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD4B` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 24 | `0x17` | 2 | `17` | `F4 44 02 00 00 17` | `59 E9 AF AD AD BA` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD5D` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 25 | `0x18` | 2 | `18` | `F4 44 02 00 00 18` | `59 E9 AF AD AD B5` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD6F` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 26 | `0x19` | 2 | `19` | `F4 44 02 00 00 19` | `59 E9 AF AD AD B4` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD81` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 27 | `0x1A` | 2 | `1A` | `F4 44 02 00 00 1A` | `59 E9 AF AD AD B7` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AD93` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 28 | `0x1B` | 2 | `1B` | `F4 44 02 00 00 1B` | `59 E9 AF AD AD B6` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADA5` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 29 | `0x1C` | 2 | `1C` | `F4 44 02 00 00 1C` | `59 E9 AF AD AD B1` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADB7` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 30 | `0x1D` | 2 | `1D` | `F4 44 02 00 00 1D` | `59 E9 AF AD AD B0` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADC9` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 31 | `0x1E` | 2 | `1E` | `F4 44 02 00 00 1E` | `59 E9 AF AD AD B3` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADDB` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 32 | `0x1F` | 2 | `1F` | `F4 44 02 00 00 1F` | `59 E9 AF AD AD B2` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADED` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 33 | `0x20` | 2 | `20` | `F4 44 02 00 00 20` | `59 E9 AF AD AD 8D` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078ADFF` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 34 | `0x21` | 2 | `21` | `F4 44 02 00 00 21` | `59 E9 AF AD AD 8C` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE11` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 35 | `0x22` | 2 | `22` | `F4 44 02 00 00 22` | `59 E9 AF AD AD 8F` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE23` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 36 | `0x23` | 2 | `23` | `F4 44 02 00 00 23` | `59 E9 AF AD AD 8E` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE35` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 37 | `0x24` | 2 | `24` | `F4 44 02 00 00 24` | `59 E9 AF AD AD 89` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE47` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 38 | `0x25` | 2 | `25` | `F4 44 02 00 00 25` | `59 E9 AF AD AD 88` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE59` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 39 | `0x26` | 2 | `26` | `F4 44 02 00 00 26` | `59 E9 AF AD AD 8B` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE6B` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 40 | `0x27` | 2 | `27` | `F4 44 02 00 00 27` | `59 E9 AF AD AD 8A` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF97` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 41 | `0x28` | 2 | `28` | `F4 44 02 00 00 28` | `59 E9 AF AD AD 85` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE7D` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 42 | `0x29` | 2 | `29` | `F4 44 02 00 00 29` | `59 E9 AF AD AD 84` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AE8F` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 43 | `0x2A` | 2 | `2A` | `F4 44 02 00 00 2A` | `59 E9 AF AD AD 87` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AEA1` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 44 | `0x2B` | 2 | `2B` | `F4 44 02 00 00 2B` | `59 E9 AF AD AD 86` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AEB3` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 45 | `0x2C` | 2 | `2C` | `F4 44 02 00 00 2C` | `59 E9 AF AD AD 81` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AEC5` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 46 | `0x2D` | 2 | `2D` | `F4 44 02 00 00 2D` | `59 E9 AF AD AD 80` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AED7` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 47 | `0x2E` | 2 | `2E` | `F4 44 02 00 00 2E` | `59 E9 AF AD AD 83` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AEE9` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 48 | `0x2F` | 2 | `2F` | `F4 44 02 00 00 2F` | `59 E9 AF AD AD 82` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AEFB` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 49 | `0x30` | 2 | `30` | `F4 44 02 00 00 30` | `59 E9 AF AD AD 9D` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF0D` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 50 | `0x31` | 2 | `31` | `F4 44 02 00 00 31` | `59 E9 AF AD AD 9C` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF1F` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 51 | `0x32` | 2 | `32` | `F4 44 02 00 00 32` | `59 E9 AF AD AD 9F` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF2E` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 52 | `0x33` | 2 | `33` | `F4 44 02 00 00 33` | `59 E9 AF AD AD 9E` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF3D` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 53 | `0x34` | 2 | `34` | `F4 44 02 00 00 34` | `59 E9 AF AD AD 99` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF4C` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 54 | `0x35` | 2 | `35` | `F4 44 02 00 00 35` | `59 E9 AF AD AD 98` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF5B` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 55 | `0x36` | 2 | `36` | `F4 44 02 00 00 36` | `59 E9 AF AD AD 9B` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF6A` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 56 | `0x37` | 2 | `37` | `F4 44 02 00 00 37` | `59 E9 AF AD AD 9A` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF79` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
| 57 | `0x38` | 2 | `38` | `F4 44 02 00 00 38` | `59 E9 AF AD AD 95` | `legacy-unknown` | `0x00: status/subopcode L1 0x0078AF88` | 0x78AADD cmp; table 0x78AAED | high-routing | `deep_selector_study_vectors.json` |
