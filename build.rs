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
    if !src.is_dir() {
        // The bundle is committed at the repo root; if it is missing (e.g. a
        // source-only checkout) the build still succeeds and the runtime
        // reports a missing data dir at boot.
        println!("cargo:warning=Data/ not present at build time; binary ships without the static data bundle");
        return;
    }

    // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out  ->  +3 parents.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let Some(profile_dir) = out.parent().and_then(Path::parent).and_then(Path::parent) else {
        println!("cargo:warning=could not derive the profile output dir from OUT_DIR; skipping Data packaging");
        return;
    };
    copy_dir(&src, &profile_dir.join("Data"));
}

fn copy_dir(src: &Path, dst: &Path) {
    if dst.exists() {
        let _ = fs::remove_dir_all(dst);
    }
    fs::create_dir_all(dst).expect("create Data output dir");
    for entry in fs::read_dir(src).expect("read Data source dir") {
        let entry = entry.expect("read Data entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy Data file");
        }
    }
}
