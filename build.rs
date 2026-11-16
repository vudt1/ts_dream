//! Package the bundled static `Data/` next to the produced binary.
//!
//! A `cargo build` must ship the game data with the executable (Chapter 8
//! Config): the runtime resolves the directory via `Config::resolve_data_dir`,
//! preferring the CWD `./Data`, then the bundle beside the current executable.
//!
//! Cargo places the final binary in `<target>/<profile>/`, while `OUT_DIR` is
//! `<target>/<profile>/build/<pkg>-<hash>/out` — so walking three parents from
//! `OUT_DIR` yields the profile directory where the binary is written. We copy
//! `Data/` there so the packaged binary and data ship together.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("Data");
    println!("cargo:rerun-if-changed=Data");
    // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out  ->  +3 parents.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let Some(profile_dir) = out.parent().and_then(Path::parent).and_then(Path::parent) else {
        println!("cargo:warning=could not derive the profile output dir from OUT_DIR; skipping Data/DB packaging");
        return;
    };

    if src.is_dir() {
        sync_dir(&src, &profile_dir.join("Data"), &[]);
    } else {
        println!("cargo:warning=Data/ not present at build time; binary ships without the static data bundle");
    }

    // Watch only the production DB seed (never the `DB/` dir itself, because
    // tests write `DB/ts_dream_test.db*` and would otherwise force a full
    // recompile/relink of every target on every `cargo test` invocation).
    let prod_db = manifest.join("DB").join("ts_dream.db");
    if prod_db.exists() {
        println!("cargo:rerun-if-changed=DB/ts_dream.db");
    }
    let db_src = manifest.join("DB");
    if db_src.is_dir() {
        sync_dir(
            &db_src,
            &profile_dir.join("DB"),
            &["ts_dream_test.db", "ts_dream_test.db-wal", "ts_dream_test.db-shm"],
        );
    } else {
        let _ = fs::create_dir_all(profile_dir.join("DB"));
    }
}

fn sync_dir(src: &Path, dst: &Path, skip_names: &[&str]) {
    fs::create_dir_all(dst).expect("create output dir");
    let mut expected = std::collections::HashSet::new();
    for entry in fs::read_dir(src).expect("read source dir") {
        let entry = entry.expect("read entry");
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy();
        if skip_names.contains(&name.as_ref()) {
            continue;
        }
        expected.insert(name_os.clone());
        let path = entry.path();
        let target = dst.join(&name_os);
        if path.is_dir() {
            sync_dir(&path, &target, skip_names);
        } else if needs_copy(&path, &target) {
            fs::copy(&path, &target).expect("copy file");
        }
    }
    if let Ok(entries) = fs::read_dir(dst) {
        for entry in entries.flatten() {
            if !expected.contains(&entry.file_name()) {
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(path);
                } else {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}

fn needs_copy(src: &Path, dst: &Path) -> bool {
    let Ok(src_meta) = fs::metadata(src) else {
        return true;
    };
    let Ok(dst_meta) = fs::metadata(dst) else {
        return true;
    };
    if src_meta.len() != dst_meta.len() {
        return true;
    }
    match (src_meta.modified(), dst_meta.modified()) {
        (Ok(sm), Ok(dm)) => sm > dm,
        _ => true,
    }
}
