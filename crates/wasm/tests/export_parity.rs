//! #1505, #3093: the browser module (`crates/wasm`, wasm-bindgen) and the C module (`crates/wasm/c`, a
//! plain `wasm32-unknown-unknown` link, the module a Roblox port translates, #2898) wrap one set of
//! bodies in `jackioh-wasm-core`, so a binding cannot be added to one and forgotten in the other.
//!
//! The shape is one rule with no list of bindings: the C module exports every binding the browser
//! module binds, and nothing else but `C_ONLY`, its memory interface. A binding added for the web is
//! added in `crates/wasm/src/lib.rs` (`#[wasm_bindgen]`), in `crates/wasm/c/src/lib.rs`
//! (`#[unsafe(no_mangle)]`) and in `crates/wasm/c/src/tests.rs`, and this file needs no edit; a binding
//! the browser module alone binds fails it. It uses `std` only and reads files at run time.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What only the C module exports, and why: its memory interface, since a host that is not JavaScript
/// reads and writes the module's memory itself.
const C_ONLY: &[(&str, &str)] = &[
    ("jackioh_alloc", C_MEMORY),
    ("jackioh_free", C_MEMORY),
    ("jackioh_result_ptr", C_MEMORY),
    ("jackioh_result_len", C_MEMORY),
];

const C_MEMORY: &str = "the C module's memory interface (`crates/wasm/c/README.md`)";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo().join(path)).unwrap_or_else(|_| panic!("cannot read {path}"))
}

/// The functions a module binds: for each line that trims to `attribute`, the `fn` name on the
/// next line.
fn exported(source: &str, attribute: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut names = BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == attribute {
            let next = lines
                .get(index + 1)
                .unwrap_or_else(|| panic!("{attribute} at line {} has no next line", index + 1));
            let at = next
                .find("fn ")
                .unwrap_or_else(|| panic!("no `fn ` after {attribute} at line {}", index + 2));
            let rest = &next[at + "fn ".len()..];
            let end = rest
                .find('(')
                .unwrap_or_else(|| panic!("no `(` after {attribute} at line {}", index + 2));
            names.insert(rest[..end].trim().to_owned());
        }
    }
    names
}

fn normal_exports() -> BTreeSet<String> {
    exported(&read("crates/wasm/src/lib.rs"), "#[wasm_bindgen]")
}

fn c_exports() -> BTreeSet<String> {
    exported(&read("crates/wasm/c/src/lib.rs"), "#[unsafe(no_mangle)]")
}

/// One message for every difference between the two modules: a browser binding the C module lacks, an
/// export of the C module's own that `C_ONLY` does not name, and a `C_ONLY` name that no longer holds.
fn parity_problems(normal: &BTreeSet<String>, c: &BTreeSet<String>) -> Vec<String> {
    let c_only: BTreeSet<&str> = C_ONLY.iter().map(|(name, _)| *name).collect();
    let mut problems = Vec::new();
    for name in normal {
        if !c.contains(name) {
            problems.push(format!(
                "`{name}` is bound in crates/wasm/src/lib.rs but the C module does not export it: \
                 export it in crates/wasm/c/src/lib.rs (and crates/wasm/c/src/tests.rs's test of it), \
                 or drop the binding"
            ));
        }
    }
    for name in c {
        if !normal.contains(name) && !c_only.contains(name.as_str()) {
            problems.push(format!(
                "`{name}` is exported in crates/wasm/c/src/lib.rs only: bind it in \
                 crates/wasm/src/lib.rs too, or list it in C_ONLY with the reason"
            ));
        }
    }
    for (name, _) in C_ONLY {
        if !c.contains(*name) {
            problems.push(format!(
                "`{name}` is listed in C_ONLY but crates/wasm/c/src/lib.rs does not export it"
            ));
        }
    }
    problems
}

#[test]
fn the_c_module_exports_every_browser_binding_and_its_memory_interface_alone() {
    let problems = parity_problems(&normal_exports(), &c_exports());
    assert!(problems.is_empty(), "export parity:\n{}", problems.join("\n"));
}

/// The acceptance criterion as a test: a browser binding the C module lacks fails parity.
#[test]
fn a_browser_binding_the_c_module_lacks_fails_parity() {
    let text = read("crates/wasm/src/lib.rs") + "\n#[wasm_bindgen]\npub fn only_here() {}\n";
    let normal = exported(&text, "#[wasm_bindgen]");
    let problems = parity_problems(&normal, &c_exports());
    assert!(
        problems.iter().any(|problem| problem.contains("only_here")),
        "expected a parity problem for only_here, got: {problems:?}"
    );
}

/// The other way round: a C export the browser module lacks, and `C_ONLY` does not name, fails parity.
#[test]
fn a_c_export_the_browser_module_lacks_fails_parity() {
    let text =
        read("crates/wasm/c/src/lib.rs") + "\n#[unsafe(no_mangle)]\npub extern \"C\" fn only_here() {}\n";
    let c = exported(&text, "#[unsafe(no_mangle)]");
    let problems = parity_problems(&normal_exports(), &c);
    assert!(
        problems.iter().any(|problem| problem.contains("only_here")),
        "expected a parity problem for only_here, got: {problems:?}"
    );
}

/// The C module is one plain `wasm32-unknown-unknown` link with an empty import section (#3093):
/// nothing it depends on or calls comes from the host. `scripts/build-wasm.sh` checks the built
/// module's imports when it builds it (`JACKIOH_WASM_C=1`, which ci.yml's `web (wasm module)` job sets);
/// this holds the sources to the same, where no build runs.
#[test]
fn the_c_module_imports_nothing_from_the_host() {
    // Comments may name what the module avoids; code and manifest lines may not.
    let code = |path: &str, comment: &str| -> String {
        read(path)
            .lines()
            .filter(|line| !line.trim_start().starts_with(comment))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let manifest = code("crates/wasm/c/Cargo.toml", "#");
    for banned in ["wasm-bindgen", "js-sys", "console_error_panic_hook", "wasi"] {
        assert!(
            !manifest.to_lowercase().contains(banned),
            "crates/wasm/c/Cargo.toml names {banned}, which the C module may not take"
        );
    }
    let source = code("crates/wasm/c/src/lib.rs", "//");
    for banned in [
        "extern \"C\" {",
        "wasm_bindgen",
        "js_sys",
        "SystemTime",
        "Instant::now",
    ] {
        assert!(
            !source.contains(banned),
            "crates/wasm/c/src/lib.rs uses {banned}, which the C module may not"
        );
    }
}
