//! The server test helpers that every test imports from one home (#2385).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// One family of test helpers defined once in its home.
struct SharedHelpers {
    home: &'static str,
    names: &'static [&'static str],
}

/// The test trees that the general server integration-test binary compiles with `tests/support/`.
const SERVER_SUPPORT_READERS: &[&str] = &[
    "tests/actor",
    "tests/api",
    "tests/bots",
    "tests/loops",
    "tests/support",
];

/// Fewer Rust files than this under `SERVER_SUPPORT_READERS` means the scan lost them.
const SERVER_SUPPORT_SOURCE_FLOOR: usize = 80;

const SHARED_TEST_HELPERS: &[SharedHelpers] = &[
    SharedHelpers {
        home: "tests/support/store.rs",
        names: &[
            "fake",
            "fake_ref",
            "fake_lock",
            "to_json",
            "from",
            "uuid",
            "advance",
            "advance_and_yield",
            "sleep_settled",
            "table",
            "fake_read",
            "fail_on",
            "stop_failing",
            "hmac_sha256_hex",
            "befriend",
            "named_cards",
        ],
    },
    SharedHelpers {
        home: "tests/support/socket.rs",
        names: &["read_frame_now"],
    },
    SharedHelpers {
        home: "tests/support/engine.rs",
        names: &["playable"],
    },
];

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() != "target" {
                rust_sources(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The 1-based lines of `text` that define one of `names` (`fn fake(` or `pub fn from<`, at
/// any indent), a `//` comment aside.
fn shared_helper_definitions(text: &str, names: &[&str]) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code: Vec<&str> = line.split("//").next().unwrap_or("").split_whitespace().collect();
            code.windows(2).any(|pair| {
                pair[0] == "fn"
                    && names.iter().any(|name| {
                        pair[1]
                            .strip_prefix(name)
                            .is_some_and(|rest| rest.starts_with(['(', '<']))
                    })
            })
        })
        .map(|(index, _)| index + 1)
        .collect()
}

/// Every copy of a shared server test helper under `root`, as `<path from root>:<line>`.
fn shared_helper_copies(root: &Path) -> Vec<String> {
    let at =
        |file: &Path, line: usize| format!("{}:{line}", file.strip_prefix(root).unwrap_or(file).display());
    let mut copies = Vec::new();
    for row in SHARED_TEST_HELPERS {
        let home = root.join(row.home);
        for dir in SERVER_SUPPORT_READERS {
            let mut files = Vec::new();
            rust_sources(&root.join(dir), &mut files);
            for file in files.iter().filter(|file| file.as_path() != home) {
                let text = fs::read_to_string(file).unwrap_or_default();
                copies.extend(
                    shared_helper_definitions(&text, row.names)
                        .into_iter()
                        .map(|line| at(file, line)),
                );
            }
        }
    }
    copies
}

#[test]
fn helpers_each_server_test_takes_its_shared_helpers_from_their_home() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in SERVER_SUPPORT_READERS {
        rust_sources(&root.join(dir), &mut files);
    }
    assert!(
        files.len() > SERVER_SUPPORT_SOURCE_FLOOR,
        "only {} files found under {SERVER_SUPPORT_READERS:?}",
        files.len()
    );
    for row in SHARED_TEST_HELPERS {
        let home = fs::read_to_string(root.join(row.home)).unwrap_or_default();
        for name in row.names {
            assert_eq!(
                shared_helper_definitions(&home, &[name]).len(),
                1,
                "{} defines `{name}` once",
                row.home
            );
        }
    }
    let copies = shared_helper_copies(root);
    let homes: Vec<&str> = SHARED_TEST_HELPERS.iter().map(|row| row.home).collect();
    assert!(
        copies.is_empty(),
        "a server test defines its own copy of a shared helper (#2385): import the one in {homes:?}, or give a helper whose body differs another name: {copies:?}"
    );
}

static SCRATCH: AtomicUsize = AtomicUsize::new(0);

/// A throwaway directory, removed when dropped, so a failed assertion leaves nothing behind.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn helpers_the_check_refuses_a_copy_in_the_server_tests() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "jackioh-server-helpers-{}-{}",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    )));
    let root = scratch.0.as_path();
    let _ = fs::remove_dir_all(root);
    let write = |path: &str, text: &str| {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("its directory");
        fs::write(path, text).expect("the file");
    };
    let from = "fn from<T: DeserializeOwned>(value: Value) -> T {\n    serde_json::from_value(value).expect(\"a value\")\n}\n";
    write("tests/api/planted.rs", from);
    // tests/store/ builds without tests/support/, so its own copy is not one.
    write("tests/store/planted.rs", from);
    write(
        "tests/bots/planted.rs",
        "use super::*;\nfn playable() -> Vec<String> {\n    Vec::new()\n}\n",
    );
    let mut copies = shared_helper_copies(root);
    drop(scratch);
    copies.sort();
    assert_eq!(copies, ["tests/api/planted.rs:1", "tests/bots/planted.rs:2"]);
}
