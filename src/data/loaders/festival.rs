//! Festival loader for `Festival.Dat` (20,310 bytes binary, 10 records of 2,031 bytes).
//!
//! Matches client `aLogin.exe` `FUN_0051e00c` and `FUN_0051decc` structure:
//! - Record size: 2,031 bytes (`0x7EF`)
//! - Total records in file: `n = FileSize / RecordSize` (10 records for 20,310 bytes)
//! - Valid festival count: `count = n - 3` (7 active seasonal festivals, records 1..=7)
//! - Key A: record `[n - 2]` (record 8, decrypts even records 2, 4, 6)
//! - Key B: record `[n - 1]` (record 9, decrypts odd records 1, 3, 5, 7)
//! - Record 0: dummy / uninitialized
//!
//! Decrypted record layout (0x7EF = 2,031 bytes):
//! - `+0x0000`: `f64` (Delphi TDateTime timestamp, 11.0 for standard client records)
//! - `+0x0008`: `u16` word flag (e.g. 0xB4C2 / 46274)
//! - `+0x000A`: `u8` length of title
//! - `+0x000B`: `[u8]` VISCII title (ShortString up to 20 bytes)
//! - `+0x001F`: `[u8]` VISCII lore text 1 (null-terminated, up to 999 bytes)
//! - `+0x0407`: `[u8]` VISCII lore text 2 (null-terminated, up to 999 bytes)

use crate::encoding;
use crate::error::Result;
use std::collections::HashMap;

/// Canonical date DWORD timestamp for `Festival.Dat` client parity (11.0 timestamp: 19000110 / 0x0121EAC6).
pub const FESTIVAL_PARITY_DATE_DWORD: u32 = 0x0121EAC6;

/// A seasonal festival definition parsed and decrypted from `Festival.Dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FestivalDef {
    pub id: u32,
    pub date_time: f64,
    pub date_dword: u32,
    pub word_flag: u16,
    pub title: String,
    pub text1: String,
    pub text2: String,
    pub raw_data: Vec<u8>,
}

pub struct FestivalDatLoader;

impl FestivalDatLoader {
    pub const RECORD_SIZE: usize = 2031; // 0x7EF bytes

    /// Parse and decrypt all seasonal festival records from binary buffer.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u32, FestivalDef>> {
        let n = bytes.len() / Self::RECORD_SIZE;
        if n < 3 {
            return Ok(HashMap::new());
        }

        let count = n - 3;
        let mut recs: Vec<Vec<u8>> = (0..n)
            .map(|i| bytes[i * Self::RECORD_SIZE..(i + 1) * Self::RECORD_SIZE].to_vec())
            .collect();

        let key_a = recs[n - 2].clone();
        let key_b = recs[n - 1].clone();

        let mut map = HashMap::new();

        for i in 1..=count {
            let key = if (i & 1) != 0 { &key_b } else { &key_a };
            for j in 0..Self::RECORD_SIZE {
                recs[i][j] ^= key[j];
            }

            let d = &recs[i];
            let dt = if d.len() >= 8 {
                f64::from_le_bytes(d[0..8].try_into().unwrap_or([0; 8]))
            } else {
                0.0
            };

            let word_flag = if d.len() >= 10 {
                u16::from_le_bytes(d[8..10].try_into().unwrap_or([0; 2]))
            } else {
                0
            };

            let title_len = if d.len() > 0x0A { d[0x0A] as usize } else { 0 };
            let title_end = (0x0B + title_len).min(0x1F).min(d.len());
            let title_bytes = if d.len() >= 0x0B { &d[0x0B..title_end] } else { &[] };
            let title = encoding::viscii_decode(title_bytes);

            let text1 = if d.len() > 0x1F {
                let slice = &d[0x1F..d.len().min(0x407)];
                let null_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
                encoding::viscii_decode(&slice[..null_pos])
            } else {
                String::new()
            };

            let text2 = if d.len() > 0x407 {
                let slice = &d[0x407..];
                let null_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
                encoding::viscii_decode(&slice[..null_pos])
            } else {
                String::new()
            };

            let id = i as u32;
            map.insert(
                id,
                FestivalDef {
                    id,
                    date_time: dt,
                    date_dword: FESTIVAL_PARITY_DATE_DWORD,
                    word_flag,
                    title,
                    text1,
                    text2,
                    raw_data: d.clone(),
                },
            );
        }

        Ok(map)
    }
}
