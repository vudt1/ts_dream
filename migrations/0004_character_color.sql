-- Bổ sung cột color (16 ký tự hex = 8 byte màu tóc, da, áo, quần, giày từ packet 0x09 0x01 / reborn)
ALTER TABLE characters ADD COLUMN color TEXT NOT NULL DEFAULT '0000000000000000';
