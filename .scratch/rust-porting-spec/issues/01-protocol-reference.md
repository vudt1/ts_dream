# Trích xuất giao thức đầy đủ (Protocol Reference)

Status: resolved
Type: research

## Question

Trích xuất từ source C# (`Client.cs`, `Server.cs`, `Data.cs`, `FormServer.cs`, `FTalk.cs`, `FChat.cs`, `FTienTrang.cs`, `FWalk.cs`, `Class5.cs`) **toàn bộ** tài liệu tham chiếu giao thức cho spec: mọi opcode client→server có handler (29 case trong `Client.cs:863-957` + sub-dispatch 0x02/0x14/0x1D), mọi opcode server→client (toàn bộ chuỗi hex gửi đi trong `Logined1`, `SendToAllClient`, `SendStatus*`, `Server.cs`, `FTalk.cs`), bố cục byte từng packet, và các hàm tiện ích (smethod_4/5/10/11/12/13/14). Đầu ra là một tài liệu markdown chuẩn bị cho chapter Protocol của spec — đủ để executor viết Rust mà không cần đọc C#.

## Answer

Asset: [01-protocol-reference](../../research/01-protocol-reference.md) — extracted by research subagent.
