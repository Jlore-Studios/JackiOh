//! The v0.3.0 run's structural acceptance checks (docs/v0.3.0/README.md §6) that no crate's own
//! tests held, read off the repository itself:
//!
//!  - V2: the pure crates (engine, cards, ai, luau, run) link nothing but serde, serde_json, indexmap and
//!    each other (the AI not `jackioh-luau`), `jackioh-luau` alone `mlua` (#442 L3, L4), and the
//!    engine's, `jackioh-luau`'s and `jackioh-run`'s ts-rs only behind their optional `ts` features
//!    (CLAUDE.md rule 4; each crate's `clippy.toml` holds the rest of the rule, the five byte for byte
//!    the same and every entry shown to fire on `tests/fixtures/purity.rs`);
//!  - V23: no code under `apps/`, `crates/` or `e2e/` reads a Markdown file's prose, part 28's grep
//!    (`readFileSync(…md`, `include_str!(…md`, `read_to_string(…md`) as a test. The two structural
//!    readers it allows are named below;
//!  - V28: the TypeScript the Rust replaced is gone: no source file under `packages/`,
//!    `apps/server/` or `ladder/` (part 37; a checkout's leftover `node_modules` does not count).
//!
//! And one the CI scoping added (#751):
//!
//!  - CI scope: `scripts/ci-scope.sh`'s `rust_<group>` lines follow the crate graph `cargo metadata`
//!    reads, so a change in a crate runs every one of ci.yml's `rust-test` groups that tests it or a
//!    crate built on it (dev-dependencies included) and no other, and a crate or a path dependency
//!    the script's map lacks fails here; and a file a crate includes from outside its own directory
//!    runs a group that tests that crate.
//!
//! And one for the daily run:
//!
//!  - Daily: every `#[ignore]` under `crates/` reads `#[ignore = "daily: <why>"]`. super.yml's
//!    `daily tests` job runs each one (`cargo test -- --ignored`), and the reason names what holds
//!    the same rule per pull request, so an ignore moves a test to the daily run and never switches
//!    one off.
//!
//! And one for BUILD §2 (#1640):
//!
//!  - Config: every `pub const` of the engine's `config.rs` is named, outside comments, by a source
//!    file under `crates/*/src`, a crate's `build.rs`, the engine's `tests/export_config.rs` or
//!    another constant in `config.rs`, so flipping one changes the game, or stops the build where
//!    the code that plays it asserts the one value it plays.
//!
//! And one for this crate's own contract (#1720):
//!
//!  - README: `crates/tools/README.md`'s command table is the built `jackioh`'s clap surface, read
//!    off each command's `-h` page: every subcommand, flag and listed value a row names is the
//!    CLI's, and every command, flag and possible value of the CLI is in some row.
//!
//! And one for the tests' shared helpers (#1777, #1826):
//!
//!  - Helpers: each family of shared test helpers is defined once, in its home, and no other file
//!    under the trees it covers defines its own: `both` and `spread` under `crates/engine/tests/rules/`
//!    (`fixtures/harness.rs`); the testkit's `Scenario::unit_at` and one-instance pick builders
//!    (`crates/engine/src/testkit/pick.rs`) under `crates/engine/tests/`, `crates/cards/`,
//!    `crates/luau/tests/` and `crates/ai/tests/`; `register_fixture_script`, `fixture`, `in_hand`,
//!    `targets_of` and `tributes_of` under `crates/cards/tests/cross/` (its `mod.rs`); and `det`,
//!    `ai_options` and `js` under `crates/ai/tests/` (`ai/support.rs`). A test file copied from an
//!    old one brings no copy back (docs/ADDING_CARDS.md §2 asks the card tests the same).
//!
//! And one for R1420's preview (#2257):
//!
//!  - Preview: every `preview_sets(&[…])` under `crates/` names a set `SHIPPED_SETS` does not list.
//!    A preview opens a held set on one test's thread, and one of a set that ships opens nothing,
//!    so the patch that ships a set takes its previews out with it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use jackioh_engine::wire::SHIPPED_SETS;
use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every dependency a build of this manifest links, with its entry's text: the keys of
/// `[dependencies]`, `[build-dependencies]` and any `[target.….dependencies]` (and the name of a
/// `[dependencies.<name>]` table). `[dev-dependencies]` link into tests only, so they are left out.
fn linked_dependencies(manifest: &str) -> Vec<(String, String)> {
    let linked = |section: &str| section == "dependencies" || section == "build-dependencies";
    let mut out: Vec<(String, String)> = Vec::new();
    let mut in_linked = false;
    let mut depth = 0_i32;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if depth == 0 && line.starts_with('[') {
            let header = line.trim_matches(|c| c == '[' || c == ']');
            let parts: Vec<&str> = header.split('.').collect();
            in_linked = parts.last().is_some_and(|last| linked(last));
            if parts.len() >= 2 && linked(parts[parts.len() - 2]) {
                out.push((
                    parts[parts.len() - 1].trim_matches('"').to_string(),
                    String::new(),
                ));
            }
            continue;
        }
        if in_linked
            && depth == 0
            && let Some((key, value)) = line.split_once('=')
        {
            out.push((key.trim().trim_matches('"').to_string(), value.trim().to_string()));
        }
        depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
    }
    out
}

#[test]
fn v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other() {
    const ALLOWED: &[&str] = &[
        "serde",
        "serde_json",
        "indexmap",
        "jackioh-engine",
        "jackioh-cards",
        "jackioh-ai",
        "jackioh-luau",
        "jackioh-run",
    ];
    for crate_dir in ["engine", "cards", "ai", "luau", "run"] {
        let path = repo().join("crates").join(crate_dir).join("Cargo.toml");
        let manifest =
            fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let dependencies = linked_dependencies(&manifest);
        assert!(
            dependencies.iter().any(|(name, _)| name == "serde"),
            "{}: no [dependencies] read; the manifest's shape changed",
            path.display()
        );
        for (name, entry) in &dependencies {
            if matches!(crate_dir, "engine" | "luau" | "run") && name == "ts-rs" {
                assert!(
                    entry.contains("optional = true"),
                    "{}: ts-rs must stay optional, behind the `ts` feature",
                    path.display()
                );
                continue;
            }
            if crate_dir == "luau" && name == "mlua" {
                continue;
            }
            assert!(
                ALLOWED.contains(&name.as_str()) && !(crate_dir == "ai" && name == "jackioh-luau"),
                "{}: `{name}` is not one of the pure crates' dependencies (CLAUDE.md rule 4)",
                path.display()
            );
        }
    }
}

#[test]
fn v2_the_reader_sees_through_tables_and_inline_tables() {
    let manifest = "[package]\nname = \"x\"\n\n[dependencies]\nserde = { workspace = true }\nrand = {\n  version = \"1\",\n}\n\n[dev-dependencies]\ntokio = \"1\"\n\n[dependencies.regex]\nversion = \"1\"\n\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n";
    let names: Vec<String> = linked_dependencies(manifest)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["serde", "rand", "regex", "libc"]);
}

/// One use of every path the pure crates' `clippy.toml` bans (its header says why nothing compiles it).
const PURITY_FIXTURE: &str = include_str!("fixtures/purity.rs");

/// Every `path = "…"` entry of a `clippy.toml`, in its order.
fn banned_paths(toml: &str) -> Vec<String> {
    toml.lines()
        .filter_map(|line| line.split_once("path = \"")?.1.split_once('"'))
        .map(|(path, _)| path.to_string())
        .collect()
}

/// The crates among `files` (each a crate and its `clippy.toml`) whose file is not byte for byte the
/// first one's.
fn diverging(files: &[(&str, String)]) -> Vec<String> {
    let Some((_, first)) = files.first() else {
        return Vec::new();
    };
    files
        .iter()
        .filter(|(_, text)| text != first)
        .map(|(name, _)| name.to_string())
        .collect()
}

#[test]
fn v2_the_pure_crates_share_one_clippy_toml() {
    let files: Vec<(&str, String)> = ["engine", "cards", "ai", "luau", "run"]
        .into_iter()
        .map(|crate_dir| {
            let path = repo().join("crates").join(crate_dir).join("clippy.toml");
            let text =
                fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            (crate_dir, text)
        })
        .collect();
    let diverging = diverging(&files);
    assert!(
        diverging.is_empty(),
        "these differ from crates/engine/clippy.toml; make one edit to all five pure crates' files (CLAUDE.md rule 4): {diverging:?}"
    );
}

#[test]
fn v2_the_parity_check_names_each_diverging_file() {
    let lint = "disallowed-types = []\n".to_string();
    let mut files = vec![
        ("engine", lint.clone()),
        ("cards", lint.clone()),
        ("ai", lint.clone()),
        ("luau", lint),
    ];
    assert!(diverging(&files).is_empty());
    files[2].1.push(' ');
    assert_eq!(diverging(&files), ["ai"]);
}

#[test]
fn v2_every_entry_of_the_pure_crates_clippy_toml_fires() {
    let conf = repo().join("crates/engine");
    let toml = fs::read_to_string(conf.join("clippy.toml")).expect("crates/engine/clippy.toml");
    let paths = banned_paths(&toml);
    assert!(
        paths.len() > 50,
        "only {} entries read; the file's shape changed: {paths:?}",
        paths.len()
    );
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "jackioh-purity-{}-{}",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    )));
    let dir = scratch.0.as_path();
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir.join("src")).expect("a scratch crate");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"purity-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
    )
    .expect("its manifest");
    fs::write(dir.join("src/lib.rs"), PURITY_FIXTURE).expect("its lib.rs");
    let out = Command::new(env!("CARGO"))
        .args(["clippy", "--offline", "--quiet", "--color", "never"])
        .env("CLIPPY_CONF_DIR", &conf)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .current_dir(dir)
        .output()
        .expect("cargo clippy runs");
    drop(scratch);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("does not refer to"),
        "an entry clippy cannot resolve bans nothing: {stderr}"
    );
    let silent: Vec<&String> = paths
        .iter()
        .filter(|path| {
            !stderr.contains(&format!("use of a disallowed method `{path}`"))
                && !stderr.contains(&format!("use of a disallowed type `{path}`"))
        })
        .collect();
    assert!(
        silent.is_empty(),
        "entries the fixture did not trip (give each a use in crates/tools/tests/fixtures/purity.rs): {silent:?}\n{stderr}"
    );
}

/// The readers part 28 left standing, and those since: each reads structure, never prose.
const STRUCTURAL_READERS: &[&str] = &[
    // The audit's table of Radiant faces, one row per card (R275).
    "crates/cards/tests/cross/radiant_standard.rs",
    // SPEC §7's table of tokens, its Token and Index cells, one row per token (R1177001).
    "crates/cards/tests/cross/catalog.rs",
    // This file names the patterns it looks for, and reads `crates/tools/README.md`'s command
    // table, the first cell of each row only (#1720).
    "crates/tools/tests/acceptance.rs",
];

const READERS: &[&str] = &["readFileSync(", "include_str!(", "read_to_string("];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(
                name.as_str(),
                "node_modules" | "target" | "dist" | "pkg" | ".vite"
            ) {
                sources(&path, out);
            }
        } else if [".rs", ".ts", ".tsx", ".mjs", ".js"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            out.push(path);
        }
    }
}

/// Where `text` reads a `.md` file the way part 28's grep finds it: a reader call whose argument,
/// up to the first `)`, names `.md`.
fn markdown_reads(text: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for reader in READERS {
        let mut from = 0;
        while let Some(found) = text[from..].find(reader) {
            let start = from + found + reader.len();
            let argument = text[start..].split(')').next().unwrap_or("");
            if argument.contains(".md") {
                lines.push(text[..start].matches('\n').count() + 1);
            }
            from = start;
        }
    }
    lines
}

#[test]
fn v23_no_code_reads_a_markdown_files_prose() {
    let root = repo();
    let mut files = Vec::new();
    for dir in ["apps", "crates", "e2e"] {
        sources(&root.join(dir), &mut files);
    }
    assert!(
        files.len() > 500,
        "only {} source files found under apps/, crates/ and e2e/",
        files.len()
    );
    let mut readers: Vec<String> = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        for line in markdown_reads(&text) {
            if !STRUCTURAL_READERS.contains(&relative.as_str()) {
                readers.push(format!("{relative}:{line}"));
            }
        }
    }
    assert!(
        readers.is_empty(),
        "code that reads a Markdown file (V23, #133): {readers:?}"
    );
}

#[test]
fn v23_the_scan_finds_each_reader() {
    let text = "a\nreadFileSync(join(HERE, \"SPEC.md\"), \"utf8\")\nconst X: &str = include_str!(\"../x.md\");\nfs::read_to_string(\"notes.json\")\n";
    assert_eq!(markdown_reads(text).len(), 2);
    assert!(markdown_reads(&text.replace(".md", ".json")).is_empty());
}

#[test]
fn v28_no_typescript_is_left_where_the_rust_replaced_it() {
    let root = repo();
    let mut left = Vec::new();
    for dir in ["packages", "apps/server", "ladder"] {
        sources(&root.join(dir), &mut left);
    }
    assert!(
        left.is_empty(),
        "TypeScript the Rust replaced is still here (V28): {left:?}"
    );
}

/// ci.yml's `rust-test` groups by their `scripts/ci-scope.sh` line, in its order, each with the
/// packages its `cargo test` names. The empty list is the last group's `--workspace --exclude …`:
/// every package no other group names.
const RUST_GROUPS: &[(&str, &[&str])] = &[
    ("rust_engine_rules", &["jackioh-engine"]),
    ("rust_engine", &["jackioh-engine"]),
    ("rust_cards_lib", &["jackioh-cards"]),
    ("rust_cards", &["jackioh-cards"]),
    ("rust_server", &["jackioh-server"]),
    ("rust_ai", &["jackioh-ai"]),
    ("rust_ai_slow", &["jackioh-ai"]),
    ("rust_luau", &["jackioh-luau"]),
    ("rust_tools_wasm", &[]),
];

static SCRATCH: AtomicUsize = AtomicUsize::new(0);

/// A throwaway directory, removed when dropped, so a failed assertion leaves nothing behind.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `git <args>` in `dir`, which must succeed.
fn git_in(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `scripts/ci-scope.sh`'s lines for a pull request that adds `files` (each a path and its
/// content), in a throwaway repository as `apps/web/src/net/ci-scope.test.ts` runs it.
fn scope_lines(files: &[(&str, &str)]) -> Vec<(String, bool)> {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "jackioh-ci-scope-{}-{}",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    )));
    let dir = scratch.0.as_path();
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).expect("a scratch directory");
    git_in(dir, &["init", "-q", "-b", "main"]);
    fs::write(dir.join("README.md"), "start\n").expect("README.md");
    git_in(dir, &["add", "-A"]);
    git_in(dir, &["commit", "-q", "-m", "start"]);
    git_in(dir, &["tag", "base"]);
    for (path, content) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("its directory");
        fs::write(&path, content).expect("the file");
    }
    git_in(dir, &["add", "-A"]);
    git_in(dir, &["commit", "-q", "-m", "change"]);
    let out = Command::new("sh")
        .arg(repo().join("scripts/ci-scope.sh"))
        .args(["base", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("sh runs");
    drop(scratch);
    assert!(
        out.status.success(),
        "scripts/ci-scope.sh: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| {
            let (name, value) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("not a `name=value` line: {line}"));
            (name.to_string(), value == "true")
        })
        .collect()
}

/// A workspace crate: its package name, its directory relative to the workspace root, and the
/// workspace crates it depends on by path, of any kind.
#[derive(Clone)]
struct Package {
    name: String,
    dir: String,
    deps: Vec<String>,
}

fn workspace_packages(metadata: &Value) -> Vec<Package> {
    let root = Path::new(metadata["workspace_root"].as_str().expect("workspace_root"));
    metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .map(|package| {
            let manifest = Path::new(package["manifest_path"].as_str().expect("manifest_path"));
            let dir = manifest
                .parent()
                .and_then(|dir| dir.strip_prefix(root).ok())
                .expect("a crate inside the workspace")
                .components()
                .map(|part| part.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            let mut deps: Vec<String> = Vec::new();
            for dependency in package["dependencies"].as_array().expect("dependencies") {
                let name = dependency["name"].as_str().expect("a dependency's name");
                if dependency["path"].is_string() && !deps.iter().any(|known| known == name) {
                    deps.push(name.to_string());
                }
            }
            Package {
                name: package["name"].as_str().expect("name").to_string(),
                dir,
                deps,
            }
        })
        .collect()
}

fn metadata() -> Value {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1", "--offline"])
        .current_dir(repo())
        .output()
        .expect("cargo metadata runs");
    assert!(
        out.status.success(),
        "cargo metadata: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("cargo metadata's JSON")
}

/// The packages a group's tests name: its list, or for the empty list every package no other group
/// names.
fn tested_by(group: &[&str], packages: &[Package]) -> Vec<String> {
    if !group.is_empty() {
        return group.iter().map(|name| name.to_string()).collect();
    }
    packages
        .iter()
        .filter(|package| {
            !RUST_GROUPS
                .iter()
                .any(|(_, named)| named.contains(&package.name.as_str()))
        })
        .map(|package| package.name.clone())
        .collect()
}

/// The packages a group's tests compile: the ones it tests and every one they reach by path.
fn closure(group: &[&str], packages: &[Package]) -> Vec<String> {
    let mut reached = tested_by(group, packages);
    let mut next = 0;
    while next < reached.len() {
        let name = reached[next].clone();
        next += 1;
        for package in packages.iter().filter(|package| package.name == name) {
            for dep in &package.deps {
                if !reached.contains(dep) {
                    reached.push(dep.clone());
                }
            }
        }
    }
    reached
}

/// Where the script's Rust groups for a change in each crate differ from the groups whose tests
/// compile that crate.
fn rust_group_mismatches(packages: &[Package]) -> Vec<String> {
    let closures: Vec<Vec<String>> = RUST_GROUPS
        .iter()
        .map(|(_, group)| closure(group, packages))
        .collect();
    let mut mismatches = Vec::new();
    for package in packages {
        let expected: Vec<&str> = RUST_GROUPS
            .iter()
            .zip(&closures)
            .filter(|(_, reached)| reached.contains(&package.name))
            .map(|((line, _), _)| *line)
            .collect();
        let lines = scope_lines(&[(&format!("{}/src/lib.rs", package.dir), "x\n")]);
        let rust: Vec<&(String, bool)> = lines
            .iter()
            .filter(|(name, _)| name.starts_with("rust_"))
            .collect();
        assert_eq!(
            rust.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
            RUST_GROUPS.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
            "scripts/ci-scope.sh's Rust lines are not ci.yml's groups"
        );
        let actual: Vec<&str> = rust
            .iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| name.as_str())
            .collect();
        if actual != expected {
            mismatches.push(format!(
                "{}: scripts/ci-scope.sh turns on {actual:?} but the crate graph needs {expected:?}",
                package.dir
            ));
        }
    }
    mismatches
}

#[test]
fn ci_scope_runs_each_rust_group_for_every_crate_it_builds_on() {
    let packages = workspace_packages(&metadata());
    assert!(
        packages.iter().any(|package| package.dir == "crates/engine"),
        "cargo metadata named no crates/engine"
    );
    let mismatches = rust_group_mismatches(&packages);
    assert!(
        mismatches.is_empty(),
        "update the crate map in scripts/ci-scope.sh (and ci.yml's rust-test groups for a new crate): {mismatches:#?}"
    );
}

#[test]
fn ci_scope_check_finds_a_crate_or_a_path_dependency_the_map_lacks() {
    let packages = workspace_packages(&metadata());

    let mut with_crate = packages.clone();
    with_crate.push(Package {
        name: "jackioh-scratch".into(),
        dir: "crates/scratch".into(),
        deps: vec!["jackioh-engine".into()],
    });
    let mismatches = rust_group_mismatches(&with_crate);
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("crates/scratch:")),
        "{mismatches:#?}"
    );

    // The server building on the tools would make a change there run the server's group, which the
    // map's `crates/tools/*` line does not.
    let mut with_dependency = packages;
    for package in &mut with_dependency {
        if package.name == "jackioh-server" {
            package.deps.push("jackioh-tools".into());
        }
    }
    let mismatches = rust_group_mismatches(&with_dependency);
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("crates/tools:")),
        "{mismatches:#?}"
    );
}

/// The file paths `text` includes: the literal after `include_str!(` or `include_bytes!(`, which is
/// relative to the file (`false`), or after `env!("CARGO_MANIFEST_DIR"),` in a `concat!`, which is
/// relative to the crate (`true`), each with its line. A marker followed by anything but a string
/// literal (a `concat!`, an `env!("OUT_DIR")`) names no path of its own.
fn outside_includes(text: &str) -> Vec<(usize, String, bool)> {
    let markers = [
        ("include_str!(", false),
        ("include_bytes!(", false),
        ("env!(\"CARGO_MANIFEST_DIR\"),", true),
    ];
    let mut out = Vec::new();
    for (marker, from_crate) in markers {
        let mut from = 0;
        while let Some(found) = text[from..].find(marker) {
            let start = from + found + marker.len();
            from = start;
            let rest = text[start..].trim_start();
            let Some(literal) = rest.strip_prefix('"') else {
                continue;
            };
            let Some(end) = literal.find('"') else {
                continue;
            };
            let at = text.len() - rest.len();
            out.push((
                text[..at].matches('\n').count() + 1,
                literal[..end].to_string(),
                from_crate,
            ));
        }
    }
    out
}

/// `path` with its `.` and `..` folded away, lexically; `None` when it climbs out of the root.
fn fold(path: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

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

#[test]
fn ci_scope_runs_a_crates_groups_for_every_file_it_includes_from_outside_it() {
    let root = repo();
    let packages = workspace_packages(&metadata());
    let mut found: Vec<String> = Vec::new();
    let mut missed: Vec<String> = Vec::new();
    for package in &packages {
        // The lines whose groups test this crate; one no group names is the last group's.
        let testing: Vec<&str> = RUST_GROUPS
            .iter()
            .filter(|(_, group)| tested_by(group, &packages).contains(&package.name))
            .map(|(line, _)| *line)
            .collect();
        let mut files = Vec::new();
        rust_sources(&root.join(&package.dir), &mut files);
        for file in files {
            let relative = fold(file.strip_prefix(&root).unwrap_or(&file)).unwrap_or_default();
            if relative == "crates/tools/tests/acceptance.rs" {
                continue;
            }
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for (line, literal, from_crate) in outside_includes(&text) {
                let base = if from_crate {
                    PathBuf::from(&package.dir)
                } else {
                    Path::new(&relative)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .to_path_buf()
                };
                let Some(path) = fold(&base.join(literal.trim_start_matches('/'))) else {
                    missed.push(format!("{relative}:{line}: {literal} leaves the repository"));
                    continue;
                };
                if path.starts_with(&format!("{}/", package.dir)) {
                    continue;
                }
                let ignored = Command::new("git")
                    .args(["check-ignore", "-q", &path])
                    .current_dir(&root)
                    .status()
                    .expect("git runs");
                if ignored.success() {
                    continue;
                }
                found.push(path.clone());
                let on = scope_lines(&[(&path, "x\n")]);
                if !on
                    .iter()
                    .any(|(name, on)| *on && testing.contains(&name.as_str()))
                {
                    missed.push(format!(
                        "{relative}:{line} includes {path}, but scripts/ci-scope.sh runs none of {testing:?} for it"
                    ));
                }
            }
        }
    }
    for expected in ["docs/radiant-audit.md", "e2e/support/config.ts"] {
        assert!(
            found.iter().any(|path| path == expected),
            "the scan found no include of {expected}: {found:?}"
        );
    }
    assert!(
        missed.is_empty(),
        "give each file its crate's groups in scripts/ci-scope.sh: {missed:#?}"
    );
}

#[test]
fn ci_scope_include_scan_finds_each_literal_path() {
    let text = "const A: &str = include_str!(\"../../docs/x.json\");\nconst B: &str = concat!(\n    env!(\"CARGO_MANIFEST_DIR\"),\n    \"/../../e2e/y.ts\"\n);\nconst C: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/z\"));\n";
    assert_eq!(
        outside_includes(text),
        [
            (1, "../../docs/x.json".to_string(), false),
            (4, "/../../e2e/y.ts".to_string(), true),
        ]
    );
}

/// The 1-based lines of `text` that ignore a test without saying it runs daily.
fn ignored_without_a_daily_reason(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let line = line.trim_start();
            line.starts_with("#[ignore") && !line.starts_with("#[ignore = \"daily: ")
        })
        .map(|(index, _)| index + 1)
        .collect()
}

#[test]
fn daily_every_ignored_test_says_it_runs_daily() {
    let root = repo();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    let mut bare = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for line in ignored_without_a_daily_reason(&text) {
            bare.push(format!(
                "{}:{line}",
                file.strip_prefix(&root).unwrap_or(&file).display()
            ));
        }
    }
    assert!(
        bare.is_empty(),
        "a test ignored without `#[ignore = \"daily: <why>\"]`, which super.yml's daily tests run and whose reason names what holds the rule per pull request: {bare:?}"
    );
}

#[test]
fn daily_the_scan_finds_each_bare_ignore() {
    let text = "#[test]\n#[ignore]\nfn a() {}\n    #[ignore = \"slow\"]\nfn b() {}\n#[ignore = \"daily: a sweep\"]\nfn c() {}\n";
    assert_eq!(ignored_without_a_daily_reason(text), [2, 4]);
}

/// `text` with each line's `//` comment cut off (`///` and `//!` among them), so a name only a
/// comment gives is no read. A `//` inside a string literal cuts its line too, which can only hide a
/// read and fail the check, never pass it.
fn without_line_comments(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// How many times `code` names `name` whole, not as part of a longer identifier.
fn token_count(code: &str, name: &str) -> usize {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    code.match_indices(name)
        .filter(|(at, _)| {
            !code[..*at].chars().next_back().is_some_and(is_ident)
                && !code[at + name.len()..].chars().next().is_some_and(is_ident)
        })
        .count()
}

/// The engine's `config.rs` above its `#[cfg(test)]` module: a test there plays no rule.
fn config_code(config: &str) -> &str {
    config.split("\n#[cfg(test)]").next().unwrap_or(config)
}

/// The names of the `pub const`s `config` defines at its top level, a `pub const fn` aside.
fn config_constants(config: &str) -> Vec<String> {
    config_code(config)
        .lines()
        .filter_map(|line| line.strip_prefix("pub const "))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty() && name.as_str() != "fn")
        .collect()
}

/// #1640: the constants of `config` that no code reads: no reader names one outside a comment, and
/// `config` names it nowhere but its own definition (a constant built from another reads that one).
fn unread_constants(config: &str, readers: &[String]) -> Vec<String> {
    let own = without_line_comments(config_code(config));
    let readers: Vec<String> = readers.iter().map(|text| without_line_comments(text)).collect();
    config_constants(config)
        .into_iter()
        .filter(|name| {
            token_count(&own, name) <= 1 && readers.iter().all(|code| token_count(code, name) == 0)
        })
        .collect()
}

#[test]
fn config_every_engine_constant_is_read_where_its_rule_is_played() {
    let root = repo();
    let config =
        fs::read_to_string(root.join("crates/engine/src/config.rs")).expect("the engine's config.rs");
    let mut files = vec![root.join("crates/engine/tests/export_config.rs")];
    for entry in fs::read_dir(root.join("crates"))
        .expect("crates/ lists")
        .flatten()
    {
        rust_sources(&entry.path().join("src"), &mut files);
        files.push(entry.path().join("build.rs"));
    }
    let readers: Vec<String> = files
        .iter()
        .filter(|file| !file.ends_with("crates/engine/src/config.rs"))
        .filter_map(|file| fs::read_to_string(file).ok())
        .collect();
    assert!(
        readers.len() > 500,
        "only {} files read under crates/",
        readers.len()
    );
    assert!(
        config_constants(&config).len() > 200,
        "only {} constants found in crates/engine/src/config.rs",
        config_constants(&config).len()
    );
    let unread = unread_constants(&config, &readers);
    assert!(
        unread.is_empty(),
        "constants in crates/engine/src/config.rs that no code reads (BUILD §2, #1640): read each where its rule is played, or assert there at compile time the one value that code plays: {unread:?}"
    );
}

#[test]
fn config_the_scan_finds_each_unread_constant() {
    let config = "\
/// READ_IN_CODE, and ONLY_IN_A_COMMENT, which nothing else names.
pub const READ_IN_CODE: i32 = 1;
pub const READ_BY_A_CONSTANT: i32 = 2;
pub const BUILT: i32 = READ_BY_A_CONSTANT * 2; // and READ_IN_A_STRING
pub const READ_IN_A_STRING: &str = \"x\";
pub const ONLY_IN_A_COMMENT: i32 = 3;
pub const SHORT: i32 = 4;
pub const ONLY_IN_TESTS: i32 = 5;
pub const fn helper() -> i32 {
    0
}

#[cfg(test)]
mod tests {
    const _: i32 = super::ONLY_IN_TESTS;
}
";
    let readers = [
        "let x = READ_IN_CODE + BUILT; // ONLY_IN_A_COMMENT\n/// SHORT\nlet y = SHORT_BUT_LONGER;\n"
            .to_string(),
        "line.strip_prefix(\"pub const READ_IN_A_STRING: &str =\")\n".to_string(),
    ];
    assert_eq!(
        config_constants(config),
        [
            "READ_IN_CODE",
            "READ_BY_A_CONSTANT",
            "BUILT",
            "READ_IN_A_STRING",
            "ONLY_IN_A_COMMENT",
            "SHORT",
            "ONLY_IN_TESTS"
        ]
    );
    assert_eq!(
        unread_constants(config, &readers),
        ["ONLY_IN_A_COMMENT", "SHORT", "ONLY_IN_TESTS"]
    );
}

/// One family of test helpers defined once, in its home, which every test under its trees takes
/// rather than copying (#1777, #1826, #1997, #2023, #2385).
struct SharedHelpers {
    /// The file that defines them, from the repository's root.
    home: &'static str,
    /// The helpers' names, each defined once in `home`.
    names: &'static [&'static str],
    /// The statics' names, each declared once in `home` (the rules tests' nonce counter, #1997).
    statics: &'static [&'static str],
    /// The field lists of the structs defined once in `home`: no file under `dirs` but `home` may
    /// define a struct whose named fields are exactly one of them, in any order (the rules tests' sink
    /// bench, #2024).
    structs: &'static [&'static [&'static str]],
    /// The trees, from the repository's root, where no file but `home` may define one.
    dirs: &'static [&'static str],
    /// Fewer Rust files than this under `dirs` means the scan lost them.
    more_than: usize,
}

/// The test trees that read the engine testkit, where none of `unit_at`, `must` and a one-instance
/// pick builder may be copied.
const KIT_READERS: &[&str] = &[
    "crates/engine/tests",
    "crates/cards",
    "crates/luau/tests",
    "crates/ai/tests",
];

/// The server test trees `tests/server.rs` compiles with `tests/support/`, where no file but a
/// helper's home may define one (#2385). `tests/store/` builds into `server_store` without them.
const SERVER_SUPPORT_READERS: &[&str] = &[
    "crates/server/tests/actor",
    "crates/server/tests/api",
    "crates/server/tests/bots",
    "crates/server/tests/loops",
    "crates/server/tests/support",
];

const SHARED_TEST_HELPERS: &[SharedHelpers] = &[
    SharedHelpers {
        home: "crates/engine/tests/rules/fixtures/harness.rs",
        names: &[
            "both",
            "spread",
            "next_nonce",
            "next_action",
            "act_result",
            "act_ok",
            "act",
            "act_logged",
            "act_with_nonce",
            "live",
            "live_mut",
            "run_effect",
            "sink_for",
            "ids_of",
            "types_of",
            "hand_ids",
        ],
        statics: &["NONCE"],
        structs: &[&["events", "rng"]],
        dirs: &["crates/engine/tests/rules"],
        more_than: 200,
    },
    SharedHelpers {
        home: "crates/engine/src/testkit/scenario.rs",
        names: &["unit_at", "must"],
        statics: &[],
        structs: &[],
        dirs: KIT_READERS,
        more_than: 800,
    },
    SharedHelpers {
        home: "crates/cards/tests/cross/mod.rs",
        names: &[
            "register_fixture_script",
            "fixture",
            "in_hand",
            "targets_of",
            "tributes_of",
        ],
        statics: &[],
        structs: &[],
        dirs: &["crates/cards/tests/cross"],
        more_than: 40,
    },
    SharedHelpers {
        home: "crates/ai/tests/ai/support.rs",
        names: &["det", "ai_options", "js"],
        statics: &[],
        structs: &[],
        dirs: &["crates/ai/tests"],
        more_than: 40,
    },
    SharedHelpers {
        home: "crates/server/tests/support/store.rs",
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
        ],
        statics: &[],
        structs: &[],
        dirs: SERVER_SUPPORT_READERS,
        more_than: 80,
    },
    SharedHelpers {
        home: "crates/server/tests/support/engine.rs",
        names: &["playable"],
        statics: &[],
        structs: &[],
        dirs: SERVER_SUPPORT_READERS,
        more_than: 80,
    },
];

/// The engine testkit's one-instance pick builders, the only home of one (#1826).
const PICK_HOME: &str = "crates/engine/src/testkit/pick.rs";

/// How a one-instance pick builder's body starts.
const PICK_LITERAL: &str = r#"json!([{ "pick": "instance", "instanceId":"#;

/// The 1-based lines of `text` that define one of `names` (`fn both(`, `pub fn spread<`, at any
/// indent), a `//` comment aside.
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

/// The 1-based lines of `text` that declare a static named one of `statics` (`static NONCE: AtomicU32 = …`,
/// `pub static NONCE`, one inside a `thread_local!`), a `//` comment aside.
fn shared_static_declarations(text: &str, statics: &[&str]) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code: Vec<&str> = line.split("//").next().unwrap_or("").split_whitespace().collect();
            code.windows(2).any(|pair| {
                pair[0] == "static"
                    && statics
                        .iter()
                        .any(|name| pair[1].strip_suffix(':') == Some(*name))
            })
        })
        .map(|(index, _)| index + 1)
        .collect()
}

/// The 1-based lines of `text` that open a struct whose named fields are exactly one of `field_sets`,
/// in any order (`struct Sink {` with `events` and `rng` on the lines below it, `pub struct Bench<'a>
/// { pub rng: Rng, pub events: Vec<GameEvent> }`), a `//` comment aside.
fn shared_struct_definitions(text: &str, field_sets: &[&[&str]]) -> Vec<usize> {
    let code: Vec<&str> = text
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect();
    (0..code.len())
        .filter(|&index| {
            if !code[index].split_whitespace().any(|word| word == "struct") {
                return false;
            }
            let rest = code[index..].join("\n");
            let Some((_, declaration)) = rest.split_once("struct") else {
                return false;
            };
            let Some(open) = declaration.find(['{', ';', '(']) else {
                return false;
            };
            if !declaration[open..].starts_with('{') {
                return false;
            }
            let body = declaration[open + 1..].split('}').next().unwrap_or("");
            let mut fields: Vec<&str> = body
                .split(',')
                .filter_map(|field| field.split_once(':'))
                .filter_map(|(name, _)| name.split_whitespace().last())
                .collect();
            fields.sort_unstable();
            field_sets.iter().any(|set| {
                let mut wanted = set.to_vec();
                wanted.sort_unstable();
                fields == wanted
            })
        })
        .map(|index| index + 1)
        .collect()
}

/// The 1-based header lines of `text`'s one-instance pick builders: a function whose signature
/// returns a `Value` and whose body, `//` comments aside, holds PICK_LITERAL closed by `}])` with no
/// second `"pick"`.
fn pick_builder_definitions(text: &str) -> Vec<usize> {
    let code: Vec<&str> = text
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect();
    let mut found = Vec::new();
    let mut open: Option<usize> = None;
    let mut index = 0;
    while index < code.len() {
        if code[index].split_whitespace().any(|word| word == "fn") {
            let header = index;
            let mut signature = String::new();
            while index < code.len() {
                signature.push_str(code[index]);
                if code[index].contains(['{', ';']) {
                    break;
                }
                index += 1;
            }
            open = signature
                .split_once("->")
                .is_some_and(|(_, returns)| returns.contains("Value"))
                .then_some(header);
        }
        let builds_one = code
            .get(index)
            .and_then(|line| line.split_once(PICK_LITERAL))
            .is_some_and(|(_, rest)| rest.contains("}])") && !rest.contains("\"pick\""));
        if let Some(header) = open.filter(|_| builds_one) {
            found.push(header + 1);
            open = None;
        }
        index += 1;
    }
    found
}

/// Every copy of a shared test helper under `root`, as `<path from root>:<line>`: a definition of
/// one of a SHARED_TEST_HELPERS row's names, a declaration of one of its statics or a struct of one
/// of its field lists under its trees outside its home, and a one-instance pick builder under
/// KIT_READERS.
fn shared_helper_copies(root: &Path) -> Vec<String> {
    let at =
        |file: &Path, line: usize| format!("{}:{line}", file.strip_prefix(root).unwrap_or(file).display());
    let mut copies = Vec::new();
    for row in SHARED_TEST_HELPERS {
        for dir in row.dirs {
            let mut files = Vec::new();
            rust_sources(&root.join(dir), &mut files);
            for file in files.iter().filter(|file| !file.ends_with(row.home)) {
                let text = fs::read_to_string(file).unwrap_or_default();
                let mut lines = shared_helper_definitions(&text, row.names);
                lines.extend(shared_static_declarations(&text, row.statics));
                lines.extend(shared_struct_definitions(&text, row.structs));
                copies.extend(lines.into_iter().map(|line| at(file, line)));
            }
        }
    }
    for dir in KIT_READERS {
        let mut files = Vec::new();
        rust_sources(&root.join(dir), &mut files);
        for file in &files {
            let text = fs::read_to_string(file).unwrap_or_default();
            copies.extend(
                pick_builder_definitions(&text)
                    .into_iter()
                    .map(|line| at(file, line)),
            );
        }
    }
    copies
}

#[test]
fn helpers_each_test_takes_its_shared_helpers_from_their_home() {
    let root = repo();
    for row in SHARED_TEST_HELPERS {
        let mut files = Vec::new();
        for dir in row.dirs {
            rust_sources(&root.join(dir), &mut files);
        }
        assert!(
            files.len() > row.more_than,
            "only {} files found under {:?}",
            files.len(),
            row.dirs
        );
        let home = fs::read_to_string(root.join(row.home)).unwrap_or_default();
        for name in row.names {
            assert_eq!(
                shared_helper_definitions(&home, &[name]).len(),
                1,
                "{} defines `{name}` once",
                row.home
            );
        }
        for name in row.statics {
            assert_eq!(
                shared_static_declarations(&home, &[name]).len(),
                1,
                "{} declares `static {name}` once",
                row.home
            );
        }
        for fields in row.structs {
            assert_eq!(
                shared_struct_definitions(&home, &[fields]).len(),
                1,
                "{} defines a struct of exactly {fields:?} once",
                row.home
            );
        }
    }
    let picks = fs::read_to_string(root.join(PICK_HOME)).unwrap_or_default();
    assert_eq!(
        pick_builder_definitions(&picks).len(),
        1,
        "{PICK_HOME} builds the one-instance pick once"
    );
    let copies = shared_helper_copies(&root);
    let homes: Vec<&str> = SHARED_TEST_HELPERS.iter().map(|row| row.home).collect();
    assert!(
        copies.is_empty(),
        "a test defines its own copy of a shared helper, nonce counter or sink bench (#1777, #1826, #1997, #2023, #2024, #2385): import the one in {homes:?} or {PICK_HOME}, or give a helper whose body differs another name: {copies:?}"
    );
}

#[test]
fn helpers_the_scan_finds_each_local_copy() {
    let text = "fn both(script: Script) -> CardScripts {\n}\n    pub fn spread(mut base: Value, extra: Value) -> Value {\n}\npub(crate) fn both<T>(script: T) {}\nfn both_faces() {}\nfn spreads(x: i32) {}\n// fn both(script)\nlet scripts = both(script);\n";
    assert_eq!(shared_helper_definitions(text, &["both", "spread"]), [1, 3, 5]);
}

#[test]
fn helpers_the_scan_finds_each_nonce_static() {
    let text = "static NONCE: AtomicU32 = AtomicU32::new(0);\nthread_local! {\n    static NONCE: Cell<u32> = const { Cell::new(0) };\n}\npub static NONCE: u32 = 0;\nstatic NONCES: u32 = 0;\nstatic SEQ: AtomicU32 = AtomicU32::new(0);\n// static NONCE: u32 = 0;\nlet nonce = NONCE.with(|n| n.get());\n";
    assert_eq!(shared_static_declarations(text, &["NONCE"]), [1, 3, 5]);
}

#[test]
fn helpers_the_scan_finds_each_sink_bench() {
    let text = "struct Bench {\n    events: Vec<GameEvent>,\n    rng: Rng,\n}\npub struct Sink<'a> { pub rng: Rng, pub events: Vec<GameEvent> }\nstruct Held<'a> {\n    state: &'a mut GameState,\n    events: Vec<GameEvent>,\n    rng: Rng,\n}\nstruct Log {\n    events: Vec<GameEvent>,\n}\nstruct Pair(Vec<GameEvent>, Rng);\n// struct Bench { events: Vec<GameEvent>, rng: Rng }\nlet bench = Bench { events: Vec::new(), rng };\n";
    assert_eq!(shared_struct_definitions(text, &[&["events", "rng"]]), [1, 5]);
}

#[test]
fn helpers_the_pick_scan_finds_each_one_instance_builder() {
    let text = [
        r#"fn at(card: &CardInstance) -> Value {"#,
        r#"    json!([{ "pick": "instance", "instanceId": card.id }])"#,
        r#"}"#,
        r#"fn unit_pick(s: &Scenario, lane: i32) -> Value {"#,
        r#"    let Some(unit) = s.unit("p2", lane) else {"#,
        r#"        panic!("p2 should hold a unit in lane {lane}");"#,
        r#"    };"#,
        r#"    json!([{ "pick": "instance", "instanceId": unit.id }])"#,
        r#"}"#,
        r#"#[test]"#,
        r#"fn answers_with_one_pick() {"#,
        r#"    s.answer(json!([{ "pick": "instance", "instanceId": id }]));"#,
        r#"}"#,
        r#"fn both_picks(a: &str, b: &str) -> Value {"#,
        r#"    json!([{ "pick": "instance", "instanceId": a }, { "pick": "instance", "instanceId": b }])"#,
        r#"}"#,
        r#"fn targets(id: &str) -> Value {"#,
        r#"    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })"#,
        r#"}"#,
        r#"// fn at(id: &str) -> Value {"#,
        r#"//     json!([{ "pick": "instance", "instanceId": id }])"#,
        r#"// }"#,
    ]
    .join("\n");
    assert_eq!(pick_builder_definitions(&text), [1, 4]);
}

#[test]
fn helpers_the_check_refuses_a_copy_in_each_covered_directory() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "jackioh-helpers-{}-{}",
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
    let mut expected = Vec::new();
    for (i, row) in SHARED_TEST_HELPERS.iter().enumerate() {
        let defs: String = row
            .names
            .iter()
            .map(|name| format!("pub fn {name}() {{}}\n"))
            .chain(
                row.statics
                    .iter()
                    .map(|name| format!("static {name}: u32 = 0;\n")),
            )
            .chain(row.structs.iter().enumerate().map(|(j, fields)| {
                let fields: Vec<String> = fields.iter().map(|field| format!("{field}: u32")).collect();
                format!("struct Planted{j} {{ {} }}\n", fields.join(", "))
            }))
            .collect();
        write(row.home, &defs);
        for dir in row.dirs {
            write(&format!("{dir}/copy_{i}.rs"), &defs);
            expected.extend(
                (1..=row.names.len() + row.statics.len() + row.structs.len())
                    .map(|line| format!("{dir}/copy_{i}.rs:{line}")),
            );
        }
    }
    let builder = r#"fn at(id: &str) -> Value { json!([{ "pick": "instance", "instanceId": id }]) }"#;
    write(PICK_HOME, builder);
    for dir in KIT_READERS {
        write(&format!("{dir}/pick_copy.rs"), builder);
        expected.push(format!("{dir}/pick_copy.rs:1"));
    }
    let mut copies = shared_helper_copies(root);
    drop(scratch);
    copies.sort();
    expected.sort();
    assert_eq!(copies, expected);
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
    write("crates/server/tests/api/planted.rs", from);
    // tests/store/ builds without tests/support/, so its own copy is not one.
    write("crates/server/tests/store/planted.rs", from);
    write(
        "crates/server/tests/bots/planted.rs",
        "use super::*;\nfn playable() -> Vec<String> {\n    Vec::new()\n}\n",
    );
    let mut copies = shared_helper_copies(root);
    drop(scratch);
    copies.sort();
    assert_eq!(
        copies,
        [
            "crates/server/tests/api/planted.rs:1",
            "crates/server/tests/bots/planted.rs:2"
        ]
    );
}

/// The built `jackioh`, whose clap surface the README's command table is held to (#1720).
const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// Commands that parse their own arguments: clap sees one trailing list (`patches`' `ARGS`), so the
/// words and flags a row gives after the command's name are its parser's to check, not clap's
/// (`patches.rs`'s `main` reads `check`, `ship`, `log`, `retcon` and the fragment's flags).
const HAND_PARSED: &[&str] = &["patches"];

/// A long flag of one command, as its `-h` page prints it.
struct CliFlag {
    name: String,
    /// Whether it takes a value (`--out <DIR>`, `--write [<REPORT>...]`).
    takes_value: bool,
    /// Its `[possible values: …]`, empty when clap lists none.
    values: Vec<String>,
}

/// One command as its `-h` page prints it: its subcommands (clap's own `help` aside) and its long
/// flags (`--help` and `--version` aside).
#[derive(Default)]
struct CliCommand {
    subcommands: Vec<String>,
    flags: Vec<CliFlag>,
}

/// One `jackioh … -h` page read: the entries of its `Commands:` and `Options:` sections, each one
/// line (clap wraps nothing without its `wrap_help` feature, which the workspace leaves off).
fn cli_command(help: &str) -> CliCommand {
    let mut command = CliCommand::default();
    let mut section = "";
    for line in help.lines() {
        if !line.starts_with(' ') {
            section = line;
            continue;
        }
        let entry = line.trim_start();
        match section {
            "Commands:" if line.len() - entry.len() == 2 => {
                let name = entry.split_whitespace().next().unwrap_or_default();
                if name != "help" {
                    command.subcommands.push(name.to_string());
                }
            }
            "Options:" if entry.starts_with('-') => {
                let spec = entry.split("  ").next().unwrap_or_default();
                let values: Vec<String> = line
                    .split_once("[possible values: ")
                    .and_then(|(_, rest)| rest.split_once(']'))
                    .map(|(list, _)| list.split(", ").map(str::to_string).collect())
                    .unwrap_or_default();
                for word in spec.split([' ', ',']) {
                    if let Some(name) = word.strip_prefix("--")
                        && !matches!(name, "help" | "version")
                    {
                        command.flags.push(CliFlag {
                            name: name.to_string(),
                            takes_value: spec.contains('<'),
                            values: values.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    command
}

/// Every command of the built `jackioh` by its path of words (the binary itself is the empty
/// path), read off `jackioh <path> -h` and each subcommand's page in turn.
fn cli_tree() -> BTreeMap<Vec<String>, CliCommand> {
    let mut tree = BTreeMap::new();
    let mut next: Vec<Vec<String>> = vec![Vec::new()];
    while let Some(path) = next.pop() {
        let out = Command::new(BIN)
            .args(&path)
            .arg("-h")
            .output()
            .expect("jackioh runs");
        assert!(
            out.status.success(),
            "jackioh {} -h: {}",
            path.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        let command = cli_command(&String::from_utf8_lossy(&out.stdout));
        for name in &command.subcommands {
            next.push([path.clone(), vec![name.clone()]].concat());
        }
        tree.insert(path, command);
    }
    tree
}

/// The usage forms of `readme`'s command table: each code span of a row's first cell, in order,
/// the table's escaped `\|` read as `|`.
fn table_usages(readme: &str) -> Vec<String> {
    readme
        .lines()
        .skip_while(|line| !line.starts_with("| Command |"))
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .flat_map(|row| {
            let cells = &row[1..];
            let end = cells
                .match_indices('|')
                .map(|(at, _)| at)
                .find(|&at| !cells[..at].ends_with('\\'))
                .unwrap_or(cells.len());
            cells[..end]
                .split('`')
                .skip(1)
                .step_by(2)
                .map(|usage| usage.replace("\\|", "|"))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A command's name as the table would spell it: lowercase letters, digits and `-`.
fn is_command_word(token: &str) -> bool {
    token.starts_with(|c: char| c.is_ascii_lowercase())
        && token
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Where the table and the CLI disagree (#1720). Each usage is read as clap reads a command line: a
/// word naming a subcommand of the command reached so far descends into it, a `--flag` must be a
/// flag of that command, and the word after a flag that takes a value is its value, which must be
/// one of the flag's possible values (each of `a|b`) when clap lists them; any other word is a
/// placeholder. Then every command, flag and possible value of the CLI must be named by some usage.
fn table_drift(usages: &[String], tree: &BTreeMap<Vec<String>, CliCommand>) -> Vec<String> {
    let shown = |path: &[String]| {
        if path.is_empty() {
            "jackioh".to_string()
        } else {
            format!("jackioh {}", path.join(" "))
        }
    };
    let hand_parsed = |path: &[String]| {
        path.first()
            .is_some_and(|name| HAND_PARSED.contains(&name.as_str()))
    };
    let mut problems = Vec::new();
    let mut reached: BTreeSet<Vec<String>> = BTreeSet::from([Vec::new()]);
    let mut named: BTreeSet<(Vec<String>, String)> = BTreeSet::new();
    for usage in usages {
        let mut path: Vec<String> = Vec::new();
        let mut value_of: Option<&CliFlag> = None;
        for word in usage.split_whitespace() {
            if hand_parsed(&path) {
                break;
            }
            let command = &tree[&path];
            let token = word.trim_matches(|c| matches!(c, '[' | ']' | '(' | ')' | '…'));
            if let Some(flag) = token.strip_prefix("--") {
                let name: String = flag
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                    .collect();
                value_of = command.flags.iter().find(|known| known.name == name);
                match value_of {
                    Some(_) => {
                        named.insert((path.clone(), name));
                    }
                    None => problems.push(format!("`{usage}`: `{}` has no --{name}", shown(&path))),
                }
                value_of = value_of.filter(|known| known.takes_value);
            } else if let Some(flag) = value_of.take() {
                for value in token.split('|').filter(|_| !flag.values.is_empty()) {
                    if flag.values.iter().any(|known| known == value) {
                        named.insert((path.clone(), format!("{}={value}", flag.name)));
                    } else {
                        problems.push(format!(
                            "`{usage}`: --{} takes no `{value}` (its values: {})",
                            flag.name,
                            flag.values.join(", ")
                        ));
                    }
                }
            } else if !command.subcommands.is_empty() && is_command_word(token) {
                if !command.subcommands.iter().any(|known| known == token) {
                    problems.push(format!(
                        "`{usage}`: `{}` has no subcommand `{token}`",
                        shown(&path)
                    ));
                    break;
                }
                path.push(token.to_string());
                reached.insert(path.clone());
            }
        }
    }
    for (path, command) in tree {
        if !reached.contains(path) {
            problems.push(format!("`{}` is in no row of the table", shown(path)));
        }
        if hand_parsed(path) {
            continue;
        }
        for flag in &command.flags {
            if !named.contains(&(path.clone(), flag.name.clone())) {
                problems.push(format!(
                    "`{} --{}` is in no row of the table",
                    shown(path),
                    flag.name
                ));
            }
            for value in &flag.values {
                if !named.contains(&(path.clone(), format!("{}={value}", flag.name))) {
                    problems.push(format!(
                        "`{} --{} {value}` is in no row of the table",
                        shown(path),
                        flag.name
                    ));
                }
            }
        }
    }
    problems
}

#[test]
fn readme_the_command_table_is_the_clis() {
    let tree = cli_tree();
    let commands = tree[&Vec::new()].subcommands.len();
    assert!(
        commands > 15,
        "only {commands} commands read off `jackioh -h`; clap's help format changed"
    );
    assert!(
        tree[&vec!["promote".to_string()]]
            .flags
            .iter()
            .any(|flag| flag.name == "lane" && !flag.values.is_empty()),
        "no possible values read off `jackioh promote -h`; clap's help format changed"
    );
    for name in HAND_PARSED {
        let command = &tree[&vec![name.to_string()]];
        assert!(
            command.subcommands.is_empty() && command.flags.is_empty(),
            "`jackioh {name}` now has clap subcommands or flags: take it off HAND_PARSED so its rows are checked"
        );
    }
    let readme = fs::read_to_string(repo().join("crates/tools/README.md")).expect("the tools README");
    let usages = table_usages(&readme);
    assert!(
        usages.len() > 30,
        "only {} usages read off crates/tools/README.md's command table; its shape changed",
        usages.len()
    );
    let problems = table_drift(&usages, &tree);
    assert!(
        problems.is_empty(),
        "crates/tools/README.md's command table and `jackioh` disagree (#1720): fix the row, or the CLI: {problems:#?}"
    );
}

#[test]
fn readme_the_drift_check_finds_each_disagreement() {
    let root = cli_command(
        "Tools\n\nUsage: jackioh <COMMAND>\n\nCommands:\n  run      Run it\n  patches  Its own parser\n  help     Print this message or the help of the given subcommand(s)\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n",
    );
    assert_eq!(root.subcommands, ["run", "patches"]);
    assert!(root.flags.is_empty());
    let run = cli_command(
        "Run it\n\nUsage: jackioh run [OPTIONS] --lane <LANE> [FILES]...\n\nArguments:\n  [FILES]...  The files\n\nOptions:\n      --lane <LANE>  The lane [default: a] [possible values: a, b, c]\n      --dry-run      Write nothing\n  -h, --help         Print help\n",
    );
    let flags: Vec<_> = run
        .flags
        .iter()
        .map(|f| (f.name.as_str(), f.takes_value, f.values.clone()))
        .collect();
    assert_eq!(
        flags,
        [
            (
                "lane",
                true,
                vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            ("dry-run", false, Vec::new())
        ]
    );
    let tree = BTreeMap::from([
        (Vec::new(), root),
        (vec!["run".to_string()], run),
        (vec!["patches".to_string()], CliCommand::default()),
    ]);
    let drift = |rows: &str| {
        let readme = format!("# x\n\n| Command | Module | Does |\n| --- | --- | --- |\n{rows}\n\nAfter.\n");
        table_drift(&table_usages(&readme), &tree)
    };
    let row = "| `run --lane a\\|b\\|c [--dry-run] <files>…`, `patches retcon [--any]` | `run.rs` | `run --gone`, in prose, is not read |";
    let readme = format!("| Command | Module | Does |\n| --- | --- | --- |\n{row}\n");
    assert_eq!(
        table_usages(&readme),
        ["run --lane a|b|c [--dry-run] <files>…", "patches retcon [--any]"]
    );
    assert_eq!(drift(row), Vec::<String>::new());
    assert_eq!(
        drift(&row.replace("a\\|b\\|c", "a\\|b")),
        ["`jackioh run --lane c` is in no row of the table"]
    );
    assert_eq!(
        drift(&row.replace("a\\|b\\|c", "a\\|b\\|c\\|d")),
        ["`run --lane a|b|c|d [--dry-run] <files>…`: --lane takes no `d` (its values: a, b, c)"]
    );
    assert_eq!(
        drift(&row.replace(" [--dry-run]", "")),
        ["`jackioh run --dry-run` is in no row of the table"]
    );
    assert_eq!(
        drift(&row.replace("[--dry-run]", "[--dry-run] [--gone]")),
        ["`run --lane a|b|c [--dry-run] [--gone] <files>…`: `jackioh run` has no --gone"]
    );
    assert_eq!(
        drift(&row.replace("`patches retcon [--any]`", "`patches`, `walk`")),
        ["`walk`: `jackioh` has no subcommand `walk`"]
    );
    assert_eq!(
        drift(&row.replace("`patches retcon [--any]`", "`run`")),
        ["`jackioh patches` is in no row of the table"]
    );
}

/// The 1-based lines of `text` where a `preview_sets(&[…])` names only sets `SHIPPED_SETS` lists,
/// read by their `SetName` variants: such a preview opens nothing (R1420).
fn previews_of_shipped_sets_only(text: &str) -> Vec<usize> {
    const CALL: &str = "preview_sets(&[";
    let shipped: Vec<String> = SHIPPED_SETS.iter().map(|set| format!("{set:?}")).collect();
    let mut lines = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find(CALL) {
        let start = from + found + CALL.len();
        from = start;
        let list = text[start..].split(']').next().unwrap_or("");
        let shipped_only = list
            .split(',')
            .map(|set| set.trim().rsplit("::").next().unwrap_or(""))
            .filter(|name| !name.is_empty())
            .all(|name| shipped.iter().any(|set| set == name));
        if shipped_only {
            lines.push(text[..start].matches('\n').count() + 1);
        }
    }
    lines
}

#[test]
fn preview_no_test_previews_only_sets_that_ship() {
    let root = repo();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    let mut calls = 0;
    let mut stale = Vec::new();
    for file in files {
        let relative = fold(file.strip_prefix(&root).unwrap_or(&file)).unwrap_or_default();
        // This file plants previews of shipped sets, to show the scan finds them.
        if relative == "crates/tools/tests/acceptance.rs" {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap_or_default();
        calls += text.matches("preview_sets(&[").count();
        for line in previews_of_shipped_sets_only(&text) {
            stale.push(format!("{relative}:{line}"));
        }
    }
    assert!(calls > 0, "the scan found no `preview_sets(&[…])` under crates/");
    assert!(
        stale.is_empty(),
        "a preview of sets that all ship opens nothing (R1420): delete it, or keep only the sets `SHIPPED_SETS` does not list: {stale:?}"
    );
}

#[test]
fn preview_the_scan_finds_each_preview_of_shipped_sets_only() {
    let text = "let _a = preview_sets(&[SetName::Core]);\nlet _b = preview_sets(&[SetName::Boss]);\nlet _c = preview_sets(&[\n    SetName::Meditative,\n    jackioh_engine::SetName::Stinkyscape,\n]);\nlet _d = preview_sets(&[SetName::MeditativePlus, SetName::Custom]);\nlet _e = preview_every_set();\n";
    assert_eq!(previews_of_shipped_sets_only(text), [1, 3]);
}
