//! The v0.3.0 run's structural acceptance checks (docs/v0.3.0/README.md §6) that no crate's own
//! tests held, read off the repository itself:
//!
//!  - V2: the pure crates (engine, cards, ai) link nothing but serde, serde_json, indexmap and each
//!    other, and the engine's ts-rs only behind its optional `ts` feature (CLAUDE.md rule 4; each
//!    crate's `clippy.toml` holds the rest of the rule);
//!  - V23: no code under `apps/`, `crates/` or `e2e/` reads a Markdown file's prose, part 28's grep
//!    (`readFileSync(…md`, `include_str!(…md`, `read_to_string(…md`) as a test. The two structural
//!    readers it allows are named below;
//!  - V28: the TypeScript the Rust replaced is gone: no source file under `packages/`,
//!    `apps/server/` or `ladder/` (part 37; a checkout's leftover `node_modules` does not count).

use std::fs;
use std::path::{Path, PathBuf};

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
    ];
    for crate_dir in ["engine", "cards", "ai"] {
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
            if crate_dir == "engine" && name == "ts-rs" {
                assert!(
                    entry.contains("optional = true"),
                    "{}: ts-rs must stay optional, behind the `ts` feature",
                    path.display()
                );
                continue;
            }
            assert!(
                ALLOWED.contains(&name.as_str()),
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
