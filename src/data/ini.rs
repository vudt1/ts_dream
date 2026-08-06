//! Win32 INI semantics for `Quests/*.ini` (Chapter 3 §3.4).
//!
//! Mandatory behaviours replicated from `GetPrivateProfileString`:
//! 1. Absent key → literal `"nothing"` sentinel.
//! 2. Section & key matching is case-insensitive (`[OnWin]` vs query "ONWIN").
//! 3. Value buffer capped at 1024 chars.
//! 4. `Dialogs=` hex forwarded verbatim.
//! 5. `[OnLose]` WarpTo read from ONWIN (C# copy-paste bug).

/// Maximum value buffer length (GetPrivateProfileString cap).
pub const VALUE_CAP: usize = 1024;

/// Sentinel returned for absent keys.
pub const NOTHING: &str = "nothing";

/// A parsed INI file: sections -> keys -> values. Preserves raw byte values
/// for `Dialogs=`/`Title=` (opaque).
#[derive(Debug, Clone, Default)]
pub struct Ini {
    sections: Vec<String>,
    data: std::collections::HashMap<String, Vec<(String, String)>>,
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        let mut ini = Ini::default();
        // Scan lines preserving order; store per-(lower-section, lower-key).
        let mut cur_section = String::new();
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                cur_section = line[1..line.len() - 1].to_string();
                if !ini.sections.iter().any(|s| s == &cur_section) {
                    ini.sections.push(cur_section.clone());
                }
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_string();
                let mut value = line[eq + 1..].trim().to_string();
                if value.len() > VALUE_CAP {
                    value.truncate(VALUE_CAP);
                }
                let entry = ini.data.entry(cur_section.to_lowercase()).or_default();
                entry.push((key, value));
            }
        }
        ini
    }

    /// Case-insensitive section/key lookup. Returns the literal `"nothing"`
    /// sentinel for an absent key. The LAST occurrence wins for a repeated key.
    pub fn get(&self, section: &str, key: &str) -> String {
        let sl = section.to_lowercase();
        if let Some(map) = self.data.get(&sl) {
            for (k, v) in map.iter().rev() {
                if k.eq_ignore_ascii_case(key) {
                    return v.clone();
                }
            }
        }
        NOTHING.to_string()
    }

    /// Raw value, without the sentinel substitution (None if absent).
    pub fn get_raw(&self, section: &str, key: &str) -> Option<&str> {
        let sl = section.to_lowercase();
        let map = self.data.get(&sl)?;
        map.iter().rev().find_map(|(k, v)| {
            if k.eq_ignore_ascii_case(key) {
                Some(v.as_str())
            } else {
                None
            }
        })
    }

    /// True if a section exists (case-insensitive).
    pub fn has_section(&self, section: &str) -> bool {
        let sl = section.to_lowercase();
        self.data.contains_key(&sl)
    }

    pub fn sections(&self) -> &[String] {
        &self.sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_key_is_nothing() {
        let ini = Ini::parse("[BASE]\nMapId=1\n");
        assert_eq!(ini.get("BASE", "Missing"), NOTHING);
        assert_eq!(ini.get_raw("BASE", "Missing"), None);
    }

    #[test]
    fn case_insensitive_section_key() {
        let ini = Ini::parse("[OnWin]\nWarpTo=5\n");
        assert_eq!(ini.get("onwin", "warpto"), "5");
        assert_eq!(ini.get("ONWIN", "WARPTO"), "5");
    }

    #[test]
    fn on_lose_warpto_reads_onwin() {
        // [OnLose] WarpTo must read from ONWIN — a C# bug the spec keeps.
        let ini = Ini::parse("[OnWin]\nWarpTo=99");
        assert_eq!(ini.get("ONLOSE", "WarpTo"), NOTHING); // absent -> sentinel
                                                          // Executor must replicate the bug by reading ONWIN for OnLose.WarpTo.
        assert_eq!(ini.get("ONWIN", "WarpTo"), "99");
    }

    #[test]
    fn value_capped() {
        let long = "x".repeat(5000);
        let ini = Ini::parse(&format!("[S]\nK={}", long));
        assert_eq!(ini.get_raw("S", "K").unwrap().len(), VALUE_CAP);
    }
}
