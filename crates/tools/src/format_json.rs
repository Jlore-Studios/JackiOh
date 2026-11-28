//! `cargo jackioh format-json [--check] [--root <dir>]` (R5289000, #5289): the card data JSON laid out a
//! record per line by `json_layout::layout`, so a diff, a merge and a `grep` for a card id each meet one
//! card's whole entry on one line. `LAYOUTS` names each file, or folder of files, and the depth of its
//! records; every writer of those files lays them out at the same depth (`patches_io::write_records`,
//! `js::stringify_records`, `gen-voice.mjs` through `json-layout.mjs`). Without `--check` it rewrites
//! every file not yet laid out, which is how a hand edit of `catalog.json` is put in shape; with it, it
//! names each and fails, which `crates/tools/tests/acceptance.rs` runs on the tree.
//!
//! Only white space outside strings changes, so each file parses to the same value it did.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::json_layout::layout;
use crate::patches::patches_io::TABLE_VERSIONS_FILE;

/// `catalog.json`'s records: each card under its id.
pub const CATALOG_DEPTH: usize = 1;

/// `chinese.json`'s and `flavour.json`'s records: each card's entry under its id, and the `"//"` header.
pub const SIDECAR_DEPTH: usize = 1;

/// `patches.json`'s records: each shipped patch of its top-level array.
pub const PATCHES_DEPTH: usize = 1;

/// `technical.json`'s records: each logged commit of its top-level array.
pub const TECHNICAL_DEPTH: usize = 1;

/// `index.json`'s records: each card id's array of versions.
pub const INDEX_DEPTH: usize = 1;

/// A delta's records (R5268000): each card under `"set"`; `"unset"`, a list of ids, on one line.
pub const DELTA_DEPTH: usize = 2;

/// A table's records under `records/` and `win_rates/` (R3882000, R1329000): each row of `"cards"`.
pub const TABLE_DEPTH: usize = 2;

/// A card list file's records under `card_metrics/` (R2173000): each list, as one line.
pub const CARD_LISTS_DEPTH: usize = 1;

/// A table folder's `versions.json` (R5268001): each version's entry.
pub const TABLE_VERSIONS_DEPTH: usize = 1;

/// `stats report --json`'s records (`analysis/fixtures/dev-40.report.json`): each card's row, each
/// created card's and each choice's, and the filter on one line.
pub const REPORT_DEPTH: usize = 2;

/// The voice manifest's records: each line's entry under `"files"`. `apps/web/scripts/json-layout.mjs`
/// names the same depth for `gen-voice.mjs`, which writes it.
pub const VOICE_MANIFEST_DEPTH: usize = 2;

/// Each file, or folder of `*.json` files, relative to the repository root, and its records' depth.
pub const LAYOUTS: &[(&str, usize)] = &[
    ("crates/cards/catalog.json", CATALOG_DEPTH),
    ("crates/cards/chinese.json", SIDECAR_DEPTH),
    ("crates/cards/flavour.json", SIDECAR_DEPTH),
    ("crates/cards/patches/patches.json", PATCHES_DEPTH),
    ("crates/cards/patches/technical.json", TECHNICAL_DEPTH),
    ("crates/cards/patches/index.json", INDEX_DEPTH),
    ("crates/cards/patches/deltas", DELTA_DEPTH),
    ("crates/cards/patches/records", TABLE_DEPTH),
    ("crates/cards/patches/win_rates", TABLE_DEPTH),
    ("crates/cards/patches/card_metrics", CARD_LISTS_DEPTH),
    ("analysis/fixtures/dev-40.report.json", REPORT_DEPTH),
    ("apps/web/src/audio/voice-manifest.json", VOICE_MANIFEST_DEPTH),
];

/// Every file `LAYOUTS` names under `root`, with its depth: a file entry itself, a folder entry each
/// `*.json` directly inside it in name order (its `versions.json` at `TABLE_VERSIONS_DEPTH`). An entry
/// `root` lacks gives none.
pub fn files(root: &Path) -> Result<Vec<(PathBuf, usize)>> {
    let mut out = Vec::new();
    for &(rel, depth) in LAYOUTS {
        let path = root.join(rel);
        if path.is_file() {
            out.push((path, depth));
        } else if path.is_dir() {
            let mut inside: Vec<PathBuf> = fs::read_dir(&path)
                .with_context(|| format!("{} cannot be read", path.display()))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<std::io::Result<_>>()
                .with_context(|| format!("{} cannot be read", path.display()))?;
            inside.retain(|file| file.is_file() && file.extension().is_some_and(|ext| ext == "json"));
            inside.sort();
            for file in inside {
                let depth = if file.file_name().is_some_and(|name| name == TABLE_VERSIONS_FILE) {
                    TABLE_VERSIONS_DEPTH
                } else {
                    depth
                };
                out.push((file, depth));
            }
        }
    }
    Ok(out)
}

/// Each file of `files(root)` that is not laid out a record per line, with the text it lays out to.
/// A file that is not JSON fails.
pub fn misformatted(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();
    for (path, depth) in files(root)? {
        let text = fs::read_to_string(&path).with_context(|| format!("{} cannot be read", path.display()))?;
        serde_json::from_str::<serde::de::IgnoredAny>(&text)
            .with_context(|| format!("{} is not JSON", path.display()))?;
        let laid = layout(&text, depth);
        if laid != text {
            out.push((path, laid));
        }
    }
    Ok(out)
}

#[derive(clap::Args)]
pub struct Args {
    /// Name each file not laid out a record per line and fail, rewriting none.
    #[arg(long)]
    pub check: bool,
    /// The repository to lay out (the one this tool was built in by default).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    let root = args.root.unwrap_or_else(crate::patches::repo_root);
    let changed = misformatted(&root)?;
    for (path, text) in &changed {
        let rel = path.strip_prefix(&root).unwrap_or(path).display();
        if args.check {
            println!("not a record per line: {rel}");
        } else {
            fs::write(path, text).with_context(|| format!("{} cannot be written", path.display()))?;
            println!("laid out {rel}");
        }
    }
    if args.check && !changed.is_empty() {
        bail!(
            "{} file(s) are not laid out a record per line: run `cargo jackioh format-json` (R5289000)",
            changed.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::patches::repo_root;

    #[test]
    fn r5289000_lays_wrappers_out_above_the_record_depth_and_each_record_on_one_line() {
        assert_eq!(
            layout(r#"{"a":{"x":[1,2],"y":{"z":null}},"b":[]}"#, 1),
            "{\n  \"a\": {\"x\":[1,2],\"y\":{\"z\":null}},\n  \"b\": []\n}\n"
        );
        // A delta: each card under "set" a line, "unset" (ids alone) on one.
        assert_eq!(
            layout(
                r#"{"set":{"c1":{"id":"c1","tags":["A"]},"c2":{}},"unset":["c3","c4"]}"#,
                2
            ),
            "{\n  \"set\": {\n    \"c1\": {\"id\":\"c1\",\"tags\":[\"A\"]},\n    \"c2\": {}\n  },\n  \"unset\": [\"c3\",\"c4\"]\n}\n"
        );
        // The root opens on lines of its own unless it is empty.
        assert_eq!(layout(r#"["a","b"]"#, 1), "[\n  \"a\",\n  \"b\"\n]\n");
        assert_eq!(layout("{}", 1), "{}\n");
        // A string is copied whole: its marks, spaces, escapes and quotes are no layout.
        let string = r#""a, [b]: {c} \"d\" \\ e""#;
        assert_eq!(layout(string, 2), format!("{string}\n"));
        assert_eq!(
            layout(&format!("{{\"k\": {string}, \"l\": [{string}]}}"), 1),
            format!("{{\n  \"k\": {string},\n  \"l\": [{string}]\n}}\n")
        );
        // Text past ASCII is copied as it is.
        let record = r#"{"name":"大D守卫","say":"🎺 \"嘟\""}"#;
        assert_eq!(
            layout(&format!("{{\"zh\":{record}}}"), 1),
            format!("{{\n  \"zh\": {record}\n}}\n")
        );
        // Depth 0 is compact: every space outside a string goes, and numbers keep their spelling.
        assert_eq!(layout("{\"a\": [ 1.0 ,\n 2e3 ] }", 0), "{\"a\":[1.0,2e3]}\n");
        // The two-space form of a value lays out the same as its compact form.
        let value = serde_json::json!({
            "set": { "c1": { "id": "c1", "tags": ["A", "B"], "base": { "text": "Hi, {x}: \"y\"" } } },
            "unset": ["c2"],
            "n": 1,
        });
        for depth in 0..4 {
            assert_eq!(
                layout(&serde_json::to_string_pretty(&value).unwrap(), depth),
                layout(&serde_json::to_string(&value).unwrap(), depth),
                "depth {depth}"
            );
        }
    }

    #[test]
    fn r5289000_every_file_is_laid_out_and_parses_to_the_same_value() {
        let root = repo_root();
        let files = files(&root).unwrap();
        for (rel, _) in LAYOUTS {
            let path = root.join(rel);
            assert!(
                files.iter().any(|(file, _)| file.starts_with(&path)),
                "{rel} gives no file"
            );
        }
        for (path, depth) in files {
            let text = fs::read_to_string(&path).unwrap();
            assert_eq!(
                layout(&text, depth),
                text,
                "{} is not laid out a record per line: run `cargo jackioh format-json`",
                path.display()
            );
            let value: Value = serde_json::from_str(&text).unwrap();
            let compact: Value = serde_json::from_str(&layout(&text, 0)).unwrap();
            assert_eq!(value, compact, "{}", path.display());
        }
    }
}
