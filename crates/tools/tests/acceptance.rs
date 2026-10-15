//! The v0.3.0 run's structural acceptance checks (docs/v0.3.0/README.md §6) that no crate's own
//! tests held, read off the repository itself:
//!
//!  - V2: the pure crates (engine, cards, ai, luau) link nothing but serde, serde_json, indexmap and
//!    each other (the AI not `jackioh-luau`), `jackioh-luau` alone `mlua` (#442 L3, L4), and the
//!    engine's and `jackioh-luau`'s ts-rs only behind their optional `ts` features (CLAUDE.md rule 4;
//!    each crate's `clippy.toml` holds the rest of the rule);
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

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    ];
    for crate_dir in ["engine", "cards", "ai", "luau"] {
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
            if matches!(crate_dir, "engine" | "luau") && name == "ts-rs" {
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

/// The readers part 28 left standing: both read structure, never prose.
const STRUCTURAL_READERS: &[&str] = &[
    // The audit's table of Radiant faces, one row per card (R275).
    "crates/cards/tests/cross/radiant_standard.rs",
    // This file names the patterns it looks for.
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
    ("rust_cards_lib_core", &["jackioh-cards"]),
    ("rust_cards_lib_classic", &["jackioh-cards"]),
    ("rust_cards_lib_classic_plus", &["jackioh-cards"]),
    ("rust_cards_lib_meditative", &["jackioh-cards"]),
    ("rust_cards", &["jackioh-cards"]),
    ("rust_server", &["jackioh-server"]),
    ("rust_ai", &["jackioh-ai"]),
    ("rust_ai_slow_a", &["jackioh-ai"]),
    ("rust_ai_slow_b", &["jackioh-ai"]),
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
