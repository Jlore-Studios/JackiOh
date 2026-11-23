//! #4014 (#3897, part 14): the server never compares a secret a caller sent with `==` or `!=`, which stop
//! at the first wrong byte, so response times would leak the secret byte by byte. In code a secret is
//! compared with `api::crypto::constant_time_eq`; otherwise the store looks it up in SQL by its keyed
//! hash or its UUID. README.md's Conventions holds the rule and its "Comparing secrets" table lists each
//! comparison the audit checked. This test reads `src/` for an equality operator beside the name of a
//! hash, a token, a pepper or a password, so a new one fails here.

use std::path::{Path, PathBuf};

/// Names of values that are, or hold a keyed hash of, a secret.
const SECRET_NAMES: &[&str] = &[
    "body_hash",
    "code_hash",
    "ip_hash",
    "unsubscribe_token",
    "password",
    "pepper",
    "secret",
    "access_token",
    "refresh_token",
];

/// Files not read: the in-memory store, which only the tests run (the issue leaves it).
const SKIPPED: &[&str] = &["db/fake.rs"];

/// A file's suffix and a substring of a line it may hold: a comparison of a name that holds no secret.
const ALLOWED: &[(&str, &str)] = &[
    // An FNV-1a checksum of the server's own stored row (`actor/checkpoint.rs`), kept to see a torn write.
    (
        "actor/checkpoint.rs",
        "fnv1a32_utf16(&checkpoint.body) != checkpoint.body_hash",
    ),
];

/// Whether `line` holds `name` beside `==` or `!=`, on either side of it.
fn compares(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(at, _)| {
        let is_equality = |text: &str| text.starts_with("==") || text.starts_with("!=");
        let before = line[..at]
            .trim_end_matches(|c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '&' | '*'))
            .trim_end();
        is_equality(line[at + name.len()..].trim_start()) || before.ends_with("==") || before.ends_with("!=")
    })
}

/// Every `.rs` file under `dir`, sorted.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    out.sort();
}

#[test]
fn the_scan_finds_a_comparison_and_passes_a_constant_time_one() {
    assert!(compares(
        "if row.route != request.route || row.body_hash != request.body_hash {",
        "body_hash"
    ));
    assert!(compares("if x == &row.body_hash {", "body_hash"));
    assert!(compares("a.code_hash == b", "code_hash"));
    assert!(!compares(
        "if !constant_time_eq(&row.body_hash, &request.body_hash) {",
        "body_hash"
    ));
    assert!(!compares("row.body_hash.len() == 64", "body_hash"));
}

#[test]
fn no_secret_in_the_server_is_compared_with_an_equality_operator() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&source, &mut files);
    assert!(
        files.len() > 50,
        "read {} files under {}",
        files.len(),
        source.display()
    );
    let mut found = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&source)
            .expect("a file under src")
            .to_string_lossy()
            .replace('\\', "/");
        if SKIPPED.iter().any(|skipped| relative.ends_with(skipped)) {
            continue;
        }
        let text = std::fs::read_to_string(file).expect("a source file");
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            let allowed = ALLOWED
                .iter()
                .any(|(suffix, part)| relative.ends_with(suffix) && line.contains(part));
            if !allowed && SECRET_NAMES.iter().any(|name| compares(line, name)) {
                found.push(format!("{relative}:{}: {line}", index + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "compare a secret with `api::crypto::constant_time_eq`, or look it up by its keyed hash \
         (README.md, Conventions):\n{}",
        found.join("\n")
    );
}
