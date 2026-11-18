//! CLI helper to patch `Festival.Dat` with unique lookup keys `1..=7` and optional
//! extra records `8..=11` without ever overwriting `Data/Festival.Dat`.
//!
//! Usage:
//!   patch_festival_dat --output /tmp/Festival_patched.Dat [--input Data/Festival.Dat] [--extra extra.json]

use std::path::{Path, PathBuf};
use std::process;
use ts_dream::data::loaders::festival::{
    FestivalDatEncoder, FestivalDatLoader, FestivalRecordInput,
};

fn is_repo_original_festival_dat(output: &Path) -> bool {
    let repo_orig = Path::new("Data").join("Festival.Dat");
    if let (Ok(canon_out), Ok(canon_orig)) = (output.canonicalize(), repo_orig.canonicalize()) {
        return canon_out == canon_orig;
    }
    let norm = output
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase();
    norm == "data/festival.dat"
}

fn parse_args() -> Result<(PathBuf, PathBuf, Option<PathBuf>), String> {
    let mut input = PathBuf::from("Data/Festival.Dat");
    let mut output: Option<PathBuf> = None;
    let mut extra: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                let val = args.next().ok_or("--input requires a path")?;
                input = PathBuf::from(val);
            }
            "--output" => {
                let val = args.next().ok_or("--output requires a path")?;
                output = Some(PathBuf::from(val));
            }
            "--extra" => {
                let val = args.next().ok_or("--extra requires a path")?;
                extra = Some(PathBuf::from(val));
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: patch_festival_dat --output <path> [--input Data/Festival.Dat] [--extra extra.json]"
                        .to_string(),
                );
            }
            other => return Err(format!("Unknown argument: {other}")),
        }
    }

    let output = output.ok_or("--output <path> is required")?;
    Ok((input, output, extra))
}

fn main() {
    let (input, output, extra_path) = match parse_args() {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            process::exit(2);
        }
    };

    if is_repo_original_festival_dat(&output) {
        eprintln!(
            "Refusing to overwrite original repository file '{}'",
            output.display()
        );
        process::exit(1);
    }

    let orig_bytes = match std::fs::read(&input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read input '{}': {e}", input.display());
            process::exit(1);
        }
    };

    let extra_records: Vec<FestivalRecordInput> = match extra_path {
        Some(path) => {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read extra JSON '{}': {e}", path.display());
                    process::exit(1);
                }
            };
            match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to parse extra JSON '{}': {e}", path.display());
                    process::exit(1);
                }
            }
        }
        None => Vec::new(),
    };

    let patched = match FestivalDatEncoder::patch_festival_dat(&orig_bytes, &extra_records) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to encode patched Festival.Dat: {e}");
            process::exit(1);
        }
    };

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    if let Err(e) = std::fs::write(&output, &patched) {
        eprintln!("Failed to write output '{}': {e}", output.display());
        process::exit(1);
    }

    let count = FestivalDatLoader::load(&patched)
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "Wrote {} bytes ({} records) to {}",
        patched.len(),
        count,
        output.display()
    );
}
