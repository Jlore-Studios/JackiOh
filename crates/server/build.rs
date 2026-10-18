//! #1169: the rules this build folds a match with, as `JACKIOH_RULES_FINGERPRINT`: FNV-1a 64 over the
//! path and the bytes of every file under `RULES`, walked in path order. `actor::checkpoint` stamps it
//! on every checkpoint it writes and refuses one stamped with another, so a checkpoint written under
//! different rules (an engine or card change, the catalog's data, the server's own fold and recorder)
//! is never resumed: the match folds its whole log instead. A root that is missing hashes as nothing.

use std::fs;
use std::path::{Path, PathBuf};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// What a match's fold and its telemetry replay read, relative to this crate.
const RULES: &[&str] = &[
    "../engine/src",
    "../cards/src",
    "../cards/build.rs",
    "../cards/catalog.json",
    "../cards/data",
    "../cards/patches",
    "src/actor/engine.rs",
    "src/actor/telemetry.rs",
];

fn files_under(path: &Path, found: &mut Vec<PathBuf>) {
    if path.is_file() {
        found.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();
    for child in children {
        files_under(&child, found);
    }
}

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut hash = FNV_OFFSET;
    for root in RULES {
        println!("cargo:rerun-if-changed={root}");
        let mut files = Vec::new();
        files_under(Path::new(root), &mut files);
        for file in files {
            let bytes = fs::read(&file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
            fnv(&mut hash, file.to_string_lossy().as_bytes());
            fnv(&mut hash, &bytes);
        }
    }
    println!("cargo:rustc-env=JACKIOH_RULES_FINGERPRINT={hash:016x}");
}
