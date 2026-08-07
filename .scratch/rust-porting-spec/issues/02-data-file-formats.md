# Định dạng dữ liệu tĩnh (Data File Formats)

Status: resolved
Type: research

## Question

Trích xuất từ `Data.cs` (các hàm `LoadData*`, `LoadDataTalks`), `DataTools/*.cs` (`ItemData`, `NpcData`, `SceneData`, `IniFile`, `TextEncoder`) và các file mẫu trong `ts_server_old/Data/` **định dạng chính xác của từng file dữ liệu**: `Items.txt`, `Npcs.txt` (UTF-16LE), `NpcOnMap.txt`, `ItemOnMap.txt`, `Warps.txt`, `Skills.txt`, `BattleGate.txt`, `Dolls.txt`, `EVe.txt`, `Member.ini`, và 813 quest `.ini`. Với mỗi file: cột nào, delimiter nào, kiểu dữ liệu, cách parse (theo cột hay theo tên), comment header `//`, và map vào struct nào (`DataStructure.Items`, `Npcs`, `Warps`, `_Talk`, ...). Đầu ra là tài liệu markdown cho chapter Data-loading của spec.

## Answer

Asset: [02-data-file-formats](../../research/02-data-file-formats.md) — extracted by research subagent.
