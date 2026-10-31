//! Mechanical Architecture Guardrail Tests.
//!
//! Enforces repository mechanical rules automatically:
//! 1. No inline `#[cfg(test)]` inside `src/` (all tests must live under `tests/`).
//! 2. Handlers must NOT mutate `online_sessions` directly via `lock_online_sessions().insert(...)`
//!    or bypass `registry_update_if_online`.
//! 3. No legacy aliases in newly refactored loader models (`CompoundDef`, `ItemDef`).
//! 4. No raw `F44404001709` literal hex inside new crafting handlers (must use `item_count_changed_frame`).

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(collect_rs_files(&path));
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
    files
}

#[test]
fn test_no_inline_cfg_test_in_src() {
    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "src/ directory must exist");

    let rs_files = collect_rs_files(src_dir);
    let mut violations = Vec::new();

    for file in rs_files {
        let content = fs::read_to_string(&file).unwrap_or_default();
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[cfg(test)]") || trimmed.contains("#[cfg(test)]") {
                violations.push(format!(
                    "{}:{} -> #[cfg(test)] inline is forbidden. Move test to tests/.",
                    file.display(),
                    line_idx + 1
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Detected inline #[cfg(test)] in src/:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_no_direct_lock_online_sessions_insert_in_handlers() {
    let handlers_dir = Path::new("src/server/handlers");
    assert!(handlers_dir.exists(), "src/server/handlers directory must exist");

    let rs_files = collect_rs_files(handlers_dir);
    let mut violations = Vec::new();

    for file in rs_files {
        let content = fs::read_to_string(&file).unwrap_or_default();
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }
            if trimmed.contains("lock_online_sessions().insert(")
                || trimmed.contains("lock_online_sessions().remove(")
            {
                violations.push(format!(
                    "{}:{} -> Direct mutation of lock_online_sessions() in handlers is forbidden. Call crate::server::session::registry_update_if_online(&conn.session).",
                    file.display(),
                    line_idx + 1
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Detected direct lock_online_sessions() mutation in handlers:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_no_legacy_aliases_in_compound_def() {
    let compound_loader = Path::new("src/data/loaders/compound.rs");
    assert!(compound_loader.exists());

    let content = fs::read_to_string(compound_loader).expect("read compound.rs");
    let forbidden_fields = [
        "compound_d",
        "material_a",
        "material_b",
        "material_c",
        "a_degree",
        "b_degree",
        "c_degree",
    ];

    for field in forbidden_fields {
        assert!(
            !content.contains(&format!("pub {}:", field)),
            "Legacy alias '{}' must not exist in CompoundDef! Use normalized fields.",
            field
        );
    }
}

#[test]
fn test_crafting_handler_uses_item_count_changed_frame() {
    let crafting_handler = Path::new("src/server/handlers/crafting.rs");
    if crafting_handler.exists() {
        let content = fs::read_to_string(crafting_handler).expect("read crafting.rs");
        assert!(
            !content.contains("F44404001709"),
            "crafting.rs must use crate::server::inventory::item_count_changed_frame instead of hardcoded 'F44404001709'"
        );
    }
}
