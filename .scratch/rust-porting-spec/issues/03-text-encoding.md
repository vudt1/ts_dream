# Encoding tiếng Việt (Text Encoding Contract)

Status: resolved
Type: research

## Question

Xác định chính xác cách text tiếng Việt được encode/decode trong toàn pipeline: `TextEncoder.cs` (DataTools) làm gì, mỗi file dữ liệu dùng encoding nào (`Npcs.txt` UTF-16LE, `Items.txt` hiện đang mojibake — cần xác định codepage gốc), cách `Class5.smethod_13` chuyển chuỗi→hex, và cách tên item/npc/player phải round-trip qua packet byte-exact. Đầu ra là **hợp đồng encoding** cho chapter Encoding của spec: decoding đầu vào + re-encoding đầu ra bắt buộc, đủ để executor port mà không đọc C#.

## Answer

Asset: [03-text-encoding](../../research/03-text-encoding.md) — extracted by research subagent.
