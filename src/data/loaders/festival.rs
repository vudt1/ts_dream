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
//! Decrypted record layout (`0x7EF` = 2,031 bytes):
//! - `+0x0000`: `f64` (client `FUN_0051e2dc` compares `(double)u32 == record.double_at_0x00`;
//!   `11.0` across all 7 records in unpatched `Data/Festival.Dat`)
//! - `+0x0008`: `u16` word flag (e.g. `0xB4C2` / 46274, unused by client)
//! - `+0x000A`: `u8` length of title
//! - `+0x000B`: `[u8]` VISCII title (ShortString up to 20 bytes)
//! - `+0x001F`: `[u8]` VISCII lore text 1 (null-terminated, up to 999 bytes)
//! - `+0x0407`: `[u8]` VISCII lore text 2 (null-terminated, up to 999 bytes)

use crate::encoding;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// A seasonal festival definition parsed and decrypted from `Festival.Dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FestivalDef {
    pub id: u32,
    pub date_time: f64,
    /// Integer lookup key derived directly from `date_time` when finite, integral, and in `1..=u32::MAX`.
    pub lookup_key: Option<u32>,
    /// Unique `S→C 0x24 0x19` key if this record is the lowest-ID owner of `lookup_key`;
    /// `None` when colliding with a lower-ID record (e.g., `#2..#7` on unpatched `Data/Festival.Dat`).
    pub unique_lore_key: Option<u32>,
    pub word_flag: u16,
    pub title: String,
    pub text1: String,
    pub text2: String,
    pub raw_data: Vec<u8>,
}

pub struct FestivalDatLoader;

impl FestivalDatLoader {
    pub const RECORD_SIZE: usize = 2031; // 0x7EF bytes

    /// Derive the client `u32` lookup key directly from `date_time` (`+0x00`).
    /// Client `FUN_0051d914` casts `u32` to `(double)u32` and compares `==` in `FUN_0051e2dc`.
    pub fn derive_lookup_key(dt: f64) -> Option<u32> {
        if dt.is_finite() && dt >= 1.0 && dt <= u32::MAX as f64 && dt.fract() == 0.0 {
            Some(dt as u32)
        } else {
            None
        }
    }

    /// Returns `true` if any loaded record shares its `lookup_key` with a lower-ID record
    /// (such as `11.0` across all 7 records in unpatched `Data/Festival.Dat`).
    pub fn has_lore_collisions(records: &HashMap<u32, FestivalDef>) -> bool {
        records
            .values()
            .any(|rec| rec.lookup_key.is_some() && rec.unique_lore_key.is_none())
    }

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
        let mut min_id_by_key: HashMap<u32, u32> = HashMap::new();

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
            let lookup_key = Self::derive_lookup_key(dt);

            let word_flag = if d.len() >= 10 {
                u16::from_le_bytes(d[8..10].try_into().unwrap_or([0; 2]))
            } else {
                0
            };

            let title_len = if d.len() > 0x0A { d[0x0A] as usize } else { 0 };
            let title_end = (0x0B + title_len).min(0x1F).min(d.len());
            let title_bytes = if d.len() >= 0x0B {
                &d[0x0B..title_end]
            } else {
                &[]
            };
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
            if let Some(k) = lookup_key {
                min_id_by_key
                    .entry(k)
                    .and_modify(|m| *m = (*m).min(id))
                    .or_insert(id);
            }

            map.insert(
                id,
                FestivalDef {
                    id,
                    date_time: dt,
                    lookup_key,
                    unique_lore_key: None,
                    word_flag,
                    title,
                    text1,
                    text2,
                    raw_data: d.clone(),
                },
            );
        }

        for def in map.values_mut() {
            if let Some(k) = def.lookup_key {
                if min_id_by_key.get(&k) == Some(&def.id) {
                    def.unique_lore_key = Some(k);
                }
            }
        }

        Ok(map)
    }
}

fn default_word_flag() -> u16 {
    0xB4C2
}

/// JSON/config input record for appending or building entries in `Festival.Dat`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FestivalRecordInput {
    pub key: u32,
    #[serde(default = "default_word_flag")]
    pub word_flag: u16,
    pub title: String,
    pub text1: String,
    pub text2: String,
}

/// Dual-XOR binary encoder for `Festival.Dat` (`0x7EF` byte records, max `0x16E = 366` records).
pub struct FestivalDatEncoder;

impl FestivalDatEncoder {
    pub const RECORD_SIZE: usize = FestivalDatLoader::RECORD_SIZE;
    pub const MAX_RECORDS: usize = 0x16E; // 366

    /// Extract `(key_a, key_b)` from the trailing 2 records of an existing `Festival.Dat`.
    pub fn extract_keys(
        bytes: &[u8],
    ) -> Result<([u8; Self::RECORD_SIZE], [u8; Self::RECORD_SIZE])> {
        let n = bytes.len() / Self::RECORD_SIZE;
        if n < 3 {
            return Err(TsError::Data(
                "Festival.Dat has fewer than 3 records; cannot extract XOR keys".to_string(),
            ));
        }
        let a_off = (n - 2) * Self::RECORD_SIZE;
        let b_off = (n - 1) * Self::RECORD_SIZE;
        let key_a: [u8; Self::RECORD_SIZE] = bytes[a_off..a_off + Self::RECORD_SIZE]
            .try_into()
            .map_err(|_| TsError::Data("Invalid keyA slice".to_string()))?;
        let key_b: [u8; Self::RECORD_SIZE] = bytes[b_off..b_off + Self::RECORD_SIZE]
            .try_into()
            .map_err(|_| TsError::Data("Invalid keyB slice".to_string()))?;
        Ok((key_a, key_b))
    }

    /// Encode a single decrypted `0x7EF`-byte `FestivalDef` record in VISCII 1.1.
    pub fn encode_record(def: &FestivalDef) -> [u8; Self::RECORD_SIZE] {
        let mut buf = [0u8; Self::RECORD_SIZE];
        if def.raw_data.len() == Self::RECORD_SIZE {
            buf.copy_from_slice(&def.raw_data);
            buf[0..8].copy_from_slice(&def.date_time.to_le_bytes());
            buf[8..10].copy_from_slice(&def.word_flag.to_le_bytes());
            return buf;
        }

        buf[0..8].copy_from_slice(&def.date_time.to_le_bytes());
        buf[8..10].copy_from_slice(&def.word_flag.to_le_bytes());

        let title_bytes = encoding::viscii_encode(&def.title);
        let title_len = title_bytes.len().min(0x1F - 0x0B); // <= 20 bytes
        buf[0x0A] = title_len as u8;
        buf[0x0B..0x0B + title_len].copy_from_slice(&title_bytes[..title_len]);

        let text1_bytes = encoding::viscii_encode(&def.text1);
        let t1_len = text1_bytes.len().min(0x407 - 0x1F - 1); // <= 999 bytes + NUL
        buf[0x1F..0x1F + t1_len].copy_from_slice(&text1_bytes[..t1_len]);

        let text2_bytes = encoding::viscii_encode(&def.text2);
        let t2_len = text2_bytes.len().min(Self::RECORD_SIZE - 0x407 - 1); // <= 999 bytes + NUL
        buf[0x407..0x407 + t2_len].copy_from_slice(&text2_bytes[..t2_len]);

        buf
    }

    /// Encode records `1..=count` with dual-XOR `key_a` (even) and `key_b` (odd).
    pub fn encode(
        records: &[FestivalDef],
        key_a: &[u8; Self::RECORD_SIZE],
        key_b: &[u8; Self::RECORD_SIZE],
    ) -> Result<Vec<u8>> {
        if records.len() > Self::MAX_RECORDS {
            return Err(TsError::Data(format!(
                "Festival.Dat record count {} exceeds MAX_RECORDS {}",
                records.len(),
                Self::MAX_RECORDS
            )));
        }

        let count = records.len();
        let total_records = count + 3;
        let mut out = vec![0u8; total_records * Self::RECORD_SIZE];

        for (idx, def) in records.iter().enumerate() {
            let rec_idx = idx + 1;
            let mut plain = Self::encode_record(def);
            let key = if (rec_idx & 1) != 0 { key_b } else { key_a };
            for (b, k) in plain.iter_mut().zip(key.iter()) {
                *b ^= *k;
            }
            let off = rec_idx * Self::RECORD_SIZE;
            out[off..off + Self::RECORD_SIZE].copy_from_slice(&plain);
        }

        let key_a_off = (count + 1) * Self::RECORD_SIZE;
        let key_b_off = (count + 2) * Self::RECORD_SIZE;
        out[key_a_off..key_a_off + Self::RECORD_SIZE].copy_from_slice(key_a);
        out[key_b_off..key_b_off + Self::RECORD_SIZE].copy_from_slice(key_b);

        Ok(out)
    }

    /// Patch `date_time` keys `1..=count` onto records loaded from an existing `Festival.Dat`
    /// and optionally append extra records (e.g. `8..=11`).
    pub fn patch_festival_dat(
        original_bytes: &[u8],
        extra: &[FestivalRecordInput],
    ) -> Result<Vec<u8>> {
        let loaded = FestivalDatLoader::load(original_bytes)?;
        let (key_a, key_b) = Self::extract_keys(original_bytes)?;

        let mut ids: Vec<u32> = loaded.keys().copied().collect();
        ids.sort_unstable();

        let mut defs: Vec<FestivalDef> = Vec::with_capacity(ids.len() + extra.len());
        for id in ids {
            if let Some(mut def) = loaded.get(&id).cloned() {
                def.date_time = id as f64;
                def.lookup_key = Some(id);
                def.unique_lore_key = Some(id);
                defs.push(def);
            }
        }

        for item in extra {
            defs.push(FestivalDef {
                id: item.key,
                date_time: item.key as f64,
                lookup_key: Some(item.key),
                unique_lore_key: Some(item.key),
                word_flag: item.word_flag,
                title: item.title.clone(),
                text1: item.text1.clone(),
                text2: item.text2.clone(),
                raw_data: Vec::new(),
            });
        }

        Self::encode(&defs, &key_a, &key_b)
    }
}
