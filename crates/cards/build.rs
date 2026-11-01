//! The card registry, generated at compile time (SURFACE §7.4). Replaces `gen-registry.ts`,
//! `_generated.ts`, `missing-tests.ts` and `registry.test.ts`'s completeness checks.
//!
//! Walks `src/scripts/<set>/<file>.rs` (`<set>` one of `core`, `classic`, `classic_plus`,
//! `meditative`, `meditative_plus`, `stinkyscape`, `training_grounds`, the last a held set's, R2194000), reads each
//! file's `pub const ID: &str = "…";` line, and writes `$OUT_DIR/registry.rs`:
//!
//! ```text
//! pub mod scripts {
//!     pub mod core { #[path = "<abs>/src/scripts/core/c001_big_d_fender.rs"] pub mod c001_big_d_fender; … }
//!     pub mod classic { … }
//!     pub mod classic_plus { … }
//!     pub mod meditative { … }
//!     pub mod meditative_plus { … }
//!     pub mod stinkyscape { … }
//! }
//! pub static REGISTRY: &[(&str, fn() -> jackioh_engine::CardScripts)] = &[ (ID, script), … ];  // sorted by ID
//! ```
//!
//! It fails the build, naming the path, for a card file without `ID`, an `ID` the catalog lacks, a
//! duplicate `ID`, a file outside `<set>/`, a file name that is not a Rust identifier, a file outside
//! its `ID`'s set's folder or whose name starts with another card's number (`misfiled`), or a catalog
//! id with no file (all 318 are written; part 33 made the last one an error).
//!
//! It also compiles the catalog version in: the `version` of the last entry of
//! `patches/patches.json` (the newest shipped patch), as `JACKIOH_CATALOG_VERSION`.
//!
//! `$OUT_DIR/catalog.json` is `catalog.json` with the whitespace between its tokens taken out, which
//! `lib.rs`'s `catalog_json()` compiles in: the same JSON, keys in the same order, without the
//! indentation, which cost the browser's module about 2.7 KB gzipped against #442 L12's cap
//! (`apps/web/src/wasm/size.test.ts`).
//!
//! R767000, R388: and every shipped version, for the replays of games played on it and for `GET
//! /api/catalog/:version`. `$OUT_DIR/history.json` holds `{ order, patches, winRates, versions }`:
//! `order` is every shipped version, oldest first; each of `versions` is one shipped version, with
//! the catalog ids its snapshot (`patches/<version>.json`) lacks (`absent`), for each id whose entry
//! differs from `catalog.json`'s, an index into `patches` (`changed`), and, when it has a win-rate
//! table file (`patches/win_rates/<version>.json`, R1329000), an index into `winRates`, which holds
//! each table once, and, when it has a card-lists file (`patches/card_metrics/<version>.json`,
//! R2173000), an index into `cardLists`, which holds each version's BUSTED and complicated lists
//! once. Each patch is `{ set, unset }`: the snapshot's top-level keys whose value
//! differs, with that value, and the keys only `catalog.json`'s entry has. An id the catalog no longer
//! has is not carried. `src/history.rs` reads it.
//!
//! The browser's module (a `wasm32` target) carries only the versions from `BROWSER_CARRIED_FROM` on:
//! it folds only practice games, and a practice game folds on a carried version only with the setup
//! it was dealt, which the worker keeps from the builds that carry this. The server and the tools
//! carry every version, for the online games the retention purge has not taken yet.
//!
//! R1183004: `$OUT_DIR/code_blocks.rs` holds every card's code block but a token's (Meditative+ #25
//! KY's Intuition): its script's lines of code, cut to the engine's `CODE_BLOCK_MAX_LINES`, which
//! this script reads off `crates/engine/src/config.rs` so the number is stated once (CLAUDE.md
//! rule 9). `lib.rs` includes it and `register_all` hands it to the engine beside the catalog.
//!
//! R1846000: `$OUT_DIR/card_metrics.rs` holds the card metrics: every script's code lines (`CODE_LINES`,
//! the lines above its `#[cfg(test)]` that hold code once its comments are taken out, a token's
//! included), and the two lists they make, `BUSTED` (more than the engine's `BUSTED_CODE_LINES`) and
//! `COMPLICATED` (more than its `COMPLICATED_CODE_LINES`), both numbers read off
//! `crates/engine/src/config.rs` as `CODE_BLOCK_MAX_LINES` is. Each is sorted by id, and computed
//! anew on every build, so both lists grow with the catalog; `register_all` hands them to the engine.
//! Beside them, `LIST_READERS`: the ids whose script reads the two lists (`reads_card_lists`, R2173000),
//! which the engine's golden binary holds its committed logs free of (R1846003).
//!
//! The crate's `clippy.toml` bans file and environment reads (CLAUDE.md rule 4); a build script runs
//! at compile time, which is exactly where data may reach a pure crate (SURFACE §3), so it may read.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

// R1846000, R2173000: the code-line counter, shared with the crate (`jackioh_cards::code_lines`) and
// through it the tools, so the lists a build compiles in and the ones `patches ship` keeps per version
// are counted alike.
#[allow(dead_code)]
#[path = "src/code_lines.rs"]
mod code_lines;
use code_lines::{code_lines, reads_card_lists, without_comments};

/// The set directories under `src/scripts/`, in SPEC §5 order (SURFACE §4.1). A set that has not
/// shipped yet (R1420) has its folder too: its cards are built, tested and registered like any other.
/// `stinkyscape` is the Stinkyscape set's (issue #1836, R1857000), and `training_grounds` the held
/// Training Grounds set's (issue #2193, R2194000).
const SETS: [&str; 7] = [
    "core",
    "classic",
    "classic_plus",
    "meditative",
    "meditative_plus",
    "stinkyscape",
    "training_grounds",
];

/// One card file: its set, its module name, its absolute path and its `ID`.
struct CardFile {
    set: &'static str,
    module: String,
    path: PathBuf,
    id: String,
}

/// R767000: the oldest version the browser's module carries (the header). The worker kept no setup
/// before v0.3.7b's builds, so the browser needs no version before v0.3.7b; v0.3.7, the one before it,
/// is carried as well so that the browser carries a version from its first build and its tests fold
/// through one. Carrying every version would add about 30 KB to the gzipped module, against #442
/// L12's cap (`apps/web/src/wasm/size.test.ts`).
const BROWSER_CARRIED_FROM: &str = "v0.3.0c";

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let scripts_dir = manifest.join("src").join("scripts");
    let catalog_path = manifest.join("catalog.json");
    let patches_path = manifest.join("patches").join("patches.json");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", catalog_path.display());
    println!("cargo:rerun-if-changed={}", patches_path.display());
    println!("cargo:rerun-if-changed={}", scripts_dir.display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest.join("src").join("code_lines.rs").display()
    );
    let win_rates_path = manifest.join("data").join("win_rates.json");
    println!("cargo:rerun-if-changed={}", win_rates_path.display());
    let patches_dir = manifest.join("patches");
    println!("cargo:rerun-if-changed={}", patches_dir.display());

    let catalog_ids = read_catalog_ids(&catalog_path);
    let version = read_catalog_version(&patches_path);
    println!("cargo:rustc-env=JACKIOH_CATALOG_VERSION={version}");

    let mut problems: Vec<String> = Vec::new();
    check_win_rates(&win_rates_path, &mut problems);
    let files = find_card_files(&scripts_dir, &mut problems);

    // Every ID names a catalog card, its file sits in its set's folder under its number, and no two
    // files claim one.
    let mut by_id: BTreeMap<&str, &CardFile> = BTreeMap::new();
    for file in &files {
        if !catalog_ids.iter().any(|id| id == &file.id) {
            problems.push(format!(
                "{}: ID \"{}\" is not in crates/cards/catalog.json: a script names a catalog card",
                file.path.display(),
                file.id
            ));
        } else if let Some(problem) = misfiled(file, &catalog_ids) {
            problems.push(problem);
        }
        if let Some(first) = by_id.insert(&file.id, file) {
            problems.push(format!(
                "{} and {} both claim \"{}\": one card, one script file (CLAUDE.md rule 6)",
                first.path.display(),
                file.path.display(),
                file.id
            ));
        }
    }

    // Every catalog id has a file.
    let missing: Vec<&String> = catalog_ids
        .iter()
        .filter(|id| !by_id.contains_key(id.as_str()))
        .collect();
    if !missing.is_empty() {
        let shown: Vec<&str> = missing.iter().take(5).map(|id| id.as_str()).collect();
        let message = format!(
            "{} catalog id(s) have no script file under src/scripts/ ({}{})",
            missing.len(),
            shown.join(", "),
            if missing.len() > shown.len() { ", …" } else { "" }
        );
        problems.push(message);
    }

    if !problems.is_empty() {
        for problem in &problems {
            println!("cargo:warning={problem}");
        }
        panic!("the card registry is broken:\n  {}", problems.join("\n  "));
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out_dir.join("registry.rs"), render(&files, &by_id)).expect("write registry.rs");
    let catalog_text =
        std::fs::read_to_string(&catalog_path).unwrap_or_else(|e| panic!("{}: {e}", catalog_path.display()));
    std::fs::write(out_dir.join("catalog.json"), compact_json(&catalog_text)).expect("write catalog.json");
    let catalog = read_json(&catalog_path);
    let catalog = catalog
        .as_object()
        .unwrap_or_else(|| panic!("{}: not a JSON object", catalog_path.display()));
    let browser = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|arch| arch == "wasm32");
    let history = history_json(&patches_dir, catalog, browser.then_some(BROWSER_CARRIED_FROM));
    std::fs::write(out_dir.join("history.json"), history).expect("write history.json");

    // R1183004: every card's code block, cut to the engine's `CODE_BLOCK_MAX_LINES`.
    let config_path = manifest.join("..").join("engine").join("src").join("config.rs");
    println!("cargo:rerun-if-changed={}", config_path.display());
    let max_lines = read_config_usize(&config_path, "CODE_BLOCK_MAX_LINES");
    std::fs::write(
        out_dir.join("code_blocks.rs"),
        render_code_blocks(&by_id, catalog, max_lines),
    )
    .expect("write code_blocks.rs");

    // R1846000: every script's code lines, and the BUSTED and complicated cards they make.
    let busted_over = read_config_usize(&config_path, "BUSTED_CODE_LINES");
    let complicated_over = read_config_usize(&config_path, "COMPLICATED_CODE_LINES");
    std::fs::write(
        out_dir.join("card_metrics.rs"),
        render_card_metrics(&by_id, busted_over, complicated_over),
    )
    .expect("write card_metrics.rs");
}

/// One of the engine's `usize` constants (CLAUDE.md rule 9: the number lives in
/// `crates/engine/src/config.rs` alone), read off its `pub const NAME: usize = N;` line:
/// `CODE_BLOCK_MAX_LINES` (R1183004), `BUSTED_CODE_LINES` and `COMPLICATED_CODE_LINES` (R1846000).
fn read_config_usize(path: &Path, name: &str) -> usize {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let head = format!("pub const {name}: usize =");
    text.lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix(head.as_str())?;
            rest.trim().strip_suffix(';')?.trim().parse::<usize>().ok()
        })
        .unwrap_or_else(|| panic!("{}: no `pub const {name}: usize = N;` line", path.display()))
}

/// R1846000: `$OUT_DIR/card_metrics.rs`: `CODE_LINES`, every catalog entry's script's code lines,
/// and `BUSTED` and `COMPLICATED`, the ids with more than `busted_over` and `complicated_over`,
/// each sorted by id (`by_id` is), so the engine finds an id by binary search.
fn render_card_metrics(
    by_id: &BTreeMap<&str, &CardFile>,
    busted_over: usize,
    complicated_over: usize,
) -> String {
    let mut readers: Vec<&str> = Vec::new();
    let counts: Vec<(&str, usize)> = by_id
        .iter()
        .map(|(id, file)| {
            let source = std::fs::read_to_string(&file.path)
                .unwrap_or_else(|e| panic!("{}: {e}", file.path.display()));
            if reads_card_lists(&source) {
                readers.push(id);
            }
            (*id, code_lines(&source))
        })
        .collect();
    let mut out = String::new();
    out.push_str("// Generated by crates/cards/build.rs (R1846000). Do not edit.\n\n");
    out.push_str("/// Every catalog entry's script's code lines, by catalog id, sorted by id (R1846000).\n");
    out.push_str("pub static CODE_LINES: &[(&str, usize)] = &[\n");
    for (id, count) in &counts {
        let _ = writeln!(out, "    ({id:?}, {count}),");
    }
    out.push_str("];\n");
    for (name, over, what) in [
        ("BUSTED", busted_over, "BUSTED_CODE_LINES"),
        ("COMPLICATED", complicated_over, "COMPLICATED_CODE_LINES"),
    ] {
        let _ = writeln!(
            out,
            "\n/// The ids whose script has more code lines than `{what}` ({over}), sorted by id (R1846000)."
        );
        let _ = writeln!(out, "pub static {name}: &[&str] = &[");
        for (id, count) in &counts {
            if *count > over {
                let _ = writeln!(out, "    {id:?},");
            }
        }
        out.push_str("];\n");
    }
    out.push_str(
        "\n/// The ids whose script reads the BUSTED or complicated lists, sorted by id (R2173000).\n",
    );
    out.push_str("pub static LIST_READERS: &[&str] = &[\n");
    for id in &readers {
        let _ = writeln!(out, "    {id:?},");
    }
    out.push_str("];\n");
    out
}

/// R1183004: `$OUT_DIR/code_blocks.rs`, `CODE_BLOCKS`: every card's `(id, code block)` but a
/// token's, sorted by id (`by_id` is), so the engine finds one by binary search.
fn render_code_blocks(
    by_id: &BTreeMap<&str, &CardFile>,
    catalog: &serde_json::Map<String, serde_json::Value>,
    max_lines: usize,
) -> String {
    let mut out = String::new();
    out.push_str("// Generated by crates/cards/build.rs (R1183004). Do not edit.\n\n");
    out.push_str("/// Every card's code block but a token's, by catalog id, sorted by id (R1183004).\n");
    out.push_str("pub static CODE_BLOCKS: &[(&str, &str)] = &[\n");
    for (id, file) in by_id {
        let Some(entry) = catalog.get(*id) else {
            continue;
        };
        if entry.get("token").and_then(|token| token.as_bool()) == Some(true) {
            continue;
        }
        let name = entry.get("name").and_then(|name| name.as_str()).unwrap_or("");
        let source =
            std::fs::read_to_string(&file.path).unwrap_or_else(|e| panic!("{}: {e}", file.path.display()));
        let block = code_block(&source, id, name, max_lines);
        let _ = writeln!(out, "    ({id:?}, {block:?}),");
    }
    out.push_str("];\n");
    out
}

/// R1183004: a card's code block — its script's lines of code as `cargo jackioh catalog loc` counts
/// them (above `#[cfg(test)]`, comments, blank lines and `use` declarations left out), less its `ID`
/// line, the first `max_lines` of them, each keeping its indentation; its own id is written `ID` and
/// its own name `???` wherever the code spells them, so the block names neither.
fn code_block(source: &str, id: &str, name: &str, max_lines: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_use = false;
    for code in without_comments(source) {
        let trimmed = code.trim();
        // A card's tests sit below its script and are not its code.
        if trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        if trimmed.is_empty() {
            continue;
        }
        if !in_use && starts_use(trimmed) {
            in_use = true;
        }
        if in_use {
            in_use = !trimmed.ends_with(';');
            continue;
        }
        if trimmed.starts_with("pub const ID") {
            continue;
        }
        let mut line = code.trim_end().replace(id, "ID");
        if !name.is_empty() {
            line = line.replace(name, "???");
        }
        lines.push(line);
        if lines.len() == max_lines {
            break;
        }
    }
    lines.join("\n")
}

/// A `use` declaration's first line, visibility included (`crates/tools`' `starts_use`).
fn starts_use(code: &str) -> bool {
    let mut rest = code;
    if let Some(after) = rest.strip_prefix("pub") {
        let after = after.trim_start();
        rest = match after.strip_prefix('(') {
            Some(inner) => match inner.find(')') {
                Some(close) => inner[close + 1..].trim_start(),
                None => return false,
            },
            None => after,
        };
    }
    rest.strip_prefix("use")
        .is_some_and(|tail| tail.is_empty() || tail.starts_with(|c: char| c.is_whitespace() || c == '{'))
}

/// `json` without the whitespace outside its strings (the header): every other character kept, in
/// order, so it parses to the same value with its keys in the same order.
fn compact_json(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let (mut in_string, mut escaped) = (false, false);
    for c in json.chars() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if !matches!(c, ' ' | '\n' | '\r' | '\t') {
            out.push(c);
        }
    }
    out
}

/// A JSON file, parsed; one that cannot be read or parsed fails the build, naming it.
fn read_json(path: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// R767000, R388: `$OUT_DIR/history.json` (the header): every shipped version, or with
/// `from` those from `from` on, as its snapshot differs from `catalog`.
fn history_json(
    patches_dir: &Path,
    catalog: &serde_json::Map<String, serde_json::Value>,
    from: Option<&str>,
) -> String {
    let list = patches_dir.join("patches.json");
    let versions: Vec<String> = read_json(&list)
        .as_array()
        .unwrap_or_else(|| panic!("{}: not a JSON array", list.display()))
        .iter()
        .map(|patch| {
            patch
                .get("version")
                .and_then(|version| version.as_str())
                .unwrap_or_else(|| panic!("{}: an entry with no string \"version\"", list.display()))
                .to_string()
        })
        .collect();
    let mut patches: Vec<serde_json::Value> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut tables: Vec<serde_json::Value> = Vec::new();
    let mut seen_tables: BTreeMap<String, usize> = BTreeMap::new();
    let mut card_lists: Vec<serde_json::Value> = Vec::new();
    let mut seen_lists: BTreeMap<String, usize> = BTreeMap::new();
    let mut carried: Vec<serde_json::Value> = Vec::new();
    let first = from.map_or(0, |from| {
        versions
            .iter()
            .position(|version| version == from)
            .unwrap_or_else(|| panic!("BROWSER_CARRIED_FROM {from} is not in {}", list.display()))
    });
    for version in versions.iter().skip(first) {
        let path = patches_dir.join(format!("{version}.json"));
        let snapshot = read_json(&path);
        let snapshot = snapshot
            .as_object()
            .unwrap_or_else(|| panic!("{}: not a JSON object", path.display()));
        let absent: Vec<&String> = catalog.keys().filter(|id| !snapshot.contains_key(*id)).collect();
        let mut changed = serde_json::Map::new();
        for (id, old) in snapshot {
            let Some(now) = catalog.get(id) else {
                continue;
            };
            if old == now {
                continue;
            }
            let patch = def_patch(old, now)
                .unwrap_or_else(|| panic!("{}: \"{id}\" is not a JSON object", path.display()));
            let at = *seen.entry(patch.to_string()).or_insert(patches.len());
            if at == patches.len() {
                patches.push(patch);
            }
            changed.insert(id.clone(), serde_json::Value::from(at));
        }
        let mut entry = serde_json::json!({ "version": version, "absent": absent, "changed": changed });
        let table_path = patches_dir.join("win_rates").join(format!("{version}.json"));
        if table_path.exists() {
            let table = read_json(&table_path);
            let at = *seen_tables.entry(table.to_string()).or_insert(tables.len());
            if at == tables.len() {
                tables.push(table);
            }
            entry["winRates"] = serde_json::Value::from(at);
        }
        let lists_path = patches_dir.join("card_metrics").join(format!("{version}.json"));
        if lists_path.exists() {
            let lists = read_json(&lists_path);
            let at = *seen_lists.entry(lists.to_string()).or_insert(card_lists.len());
            if at == card_lists.len() {
                card_lists.push(lists);
            }
            entry["cardLists"] = serde_json::Value::from(at);
        }
        carried.push(entry);
    }
    // R1371000: the old names the version retcon renamed, which a reader of an unmarked file maps;
    // R2532000: and, by the scheme an older marked file names, the names a later retcon renamed.
    let aliases_path = patches_dir.join("aliases.json");
    let (aliases, marked) = if aliases_path.exists() {
        let file = read_json(&aliases_path);
        (
            file["aliases"].clone(),
            file.get("marked").cloned().unwrap_or(serde_json::json!({})),
        )
    } else {
        (serde_json::json!({}), serde_json::json!({}))
    };
    serde_json::json!({
        "order": versions,
        "patches": patches,
        "winRates": tables,
        "cardLists": card_lists,
        "versions": carried,
        "aliases": aliases,
        "marked": marked,
    })
    .to_string()
}

/// R767000: what turns `now`'s entry back into `old`'s: `old`'s top-level keys whose value differs
/// or is missing, with `old`'s value (`set`), and the keys only `now` has (`unset`). `None` when
/// either is not an object.
fn def_patch(old: &serde_json::Value, now: &serde_json::Value) -> Option<serde_json::Value> {
    let (old, now) = (old.as_object()?, now.as_object()?);
    let set: serde_json::Map<String, serde_json::Value> = old
        .iter()
        .filter(|(key, value)| now.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let unset: Vec<&String> = now.keys().filter(|key| !old.contains_key(*key)).collect();
    Some(serde_json::json!({ "set": set, "unset": unset }))
}

/// The catalog's ids, in `catalog.json`'s order.
fn read_catalog_ids(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{}: not a JSON object", path.display()));
    // serde_json without `preserve_order` sorts keys; the order is not used, only membership.
    object.keys().cloned().collect()
}

/// ME-STATS (Meditative #50 CN Tech): `data/win_rates.json` holds `{ patch, source, cards }`,
/// and the build fails, naming the path, unless `patch` is a string, `source` is `live` or
/// `provisional`, every row id is unique, and every row reads `0 <= wins <= games`.
fn check_win_rates(path: &Path, problems: &mut Vec<String>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let table = value
        .as_object()
        .unwrap_or_else(|| panic!("{}: not a JSON object", path.display()));
    if table.get("patch").and_then(|patch| patch.as_str()).is_none() {
        problems.push(format!("{}: \"patch\" must be a string", path.display()));
    }
    if !matches!(
        table.get("source").and_then(|source| source.as_str()),
        Some("live") | Some("provisional")
    ) {
        problems.push(format!(
            "{}: \"source\" must be \"live\" or \"provisional\"",
            path.display()
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for row in table
        .get("cards")
        .and_then(|cards| cards.as_array())
        .into_iter()
        .flatten()
    {
        let id = row.get("id").and_then(|id| id.as_str()).unwrap_or("");
        let wins = row.get("wins").and_then(|wins| wins.as_i64());
        let games = row.get("games").and_then(|games| games.as_i64());
        if id.is_empty() {
            problems.push(format!("{}: a win-rate row has no string \"id\"", path.display()));
            continue;
        }
        if seen.contains(&id) {
            problems.push(format!(
                "{0}: win-rate row \"{id}\" is listed twice",
                path.display()
            ));
        }
        seen.push(id);
        match (wins, games) {
            (Some(wins), Some(games)) if 0 <= wins && wins <= games => {}
            _ => problems.push(format!(
                "{0}: win-rate row \"{id}\" must read 0 <= wins <= games",
                path.display()
            )),
        }
    }
    if table.get("cards").and_then(|cards| cards.as_array()).is_none() {
        problems.push(format!("{}: \"cards\" must be an array", path.display()));
    }
}

/// The `version` of `patches.json`'s last entry: the newest shipped patch (SURFACE §7.4, §11.3).
fn read_catalog_version(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    value
        .as_array()
        .and_then(|patches| patches.last())
        .and_then(|patch| patch.get("version"))
        .and_then(|version| version.as_str())
        .unwrap_or_else(|| panic!("{}: no last entry with a string \"version\"", path.display()))
        .to_string()
}

/// Every `src/scripts/<set>/<file>.rs`, sorted by path; anything else under `src/scripts/` that is
/// a `.rs` file is a problem.
fn find_card_files(scripts_dir: &Path, problems: &mut Vec<String>) -> Vec<CardFile> {
    let mut files = Vec::new();
    if !scripts_dir.exists() {
        return files;
    }
    let mut stray = Vec::new();
    walk(scripts_dir, &mut stray);
    stray.sort();
    for path in stray {
        let relative = path.strip_prefix(scripts_dir).expect("under src/scripts");
        let parts: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let set = match parts.as_slice() {
            [set, _file] => SETS.iter().find(|s| **s == set.as_str()).copied(),
            _ => None,
        };
        let Some(set) = set else {
            problems.push(format!(
                "{}: a card file lives at src/scripts/<set>/<file>.rs, <set> one of {}",
                path.display(),
                SETS.join(", ")
            ));
            continue;
        };
        let module = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !is_identifier(&module) {
            problems.push(format!(
                "{}: the file name must be a Rust identifier (SURFACE §4.1: `012-1-devour.ts` → `c012_1_devour.rs`)",
                path.display()
            ));
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        match read_id(&text) {
            Some(id) => files.push(CardFile {
                set,
                module,
                path,
                id,
            }),
            None => problems.push(format!(
                "{}: no `pub const ID: &str = \"…\";` line (SURFACE §7.1)",
                path.display()
            )),
        }
    }
    files
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display())).path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The string of the file's `pub const ID: &str = "…";` line.
fn read_id(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("pub const ID")?.trim_start();
        let rest = rest
            .strip_prefix(':')?
            .trim_start()
            .strip_prefix("&str")?
            .trim_start();
        let rest = rest.strip_prefix('=')?.trim_start().strip_prefix('"')?;
        let (id, tail) = rest.split_once('"')?;
        (tail.trim_start().starts_with(';') && !id.is_empty()).then(|| id.to_string())
    })
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// CLAUDE.md rule 6: where a card's file is filed, its `ID`'s set's folder and the prefix its number
/// spells: `core-051-1` → `core`, `c051_1`; `classicplus-t-ai-01` → `classic_plus`, `t_ai_01`. An id
/// spells its set's folder without the underscore (`classicplus-…`). `None` for an id whose set has no
/// folder in `SETS`.
fn home_of(id: &str) -> Option<(&'static str, String)> {
    let (segment, number) = id.split_once('-')?;
    let set = SETS.iter().copied().find(|set| set.replace('_', "") == segment)?;
    let stem = number.replace('-', "_");
    let prefix = if number.starts_with("t-") {
        stem
    } else {
        format!("c{stem}")
    };
    Some((set, prefix))
}

/// CLAUDE.md rule 6: a card file sits in its `ID`'s set's folder, and of that set's catalog ids the
/// one with the longest prefix its name starts with is its `ID` (`c051_1_…` is #51.1's, not #51's;
/// `c025_4_mana_7_7` is #25's, as no `core-025-4` exists). The slug after the prefix is free: some
/// differ from the card's name on purpose (`core/c097_zephyrs.rs` for Zephrys). The problem, naming
/// where the file belongs, or `None` when it is filed right.
fn misfiled(file: &CardFile, catalog_ids: &[String]) -> Option<String> {
    let Some((set, prefix)) = home_of(&file.id) else {
        return Some(format!(
            "{}: ID \"{}\" names no set with a folder under src/scripts/ ({})",
            file.path.display(),
            file.id,
            SETS.join(", ")
        ));
    };
    let carries = |stem: &str| file.module == stem || file.module.starts_with(&format!("{stem}_"));
    let owner = catalog_ids
        .iter()
        .filter_map(|id| home_of(id).map(|home| (id, home)))
        .filter(|(_, (folder, stem))| *folder == file.set && carries(stem))
        .max_by_key(|(_, (_, stem))| stem.len())
        .map(|(id, _)| id.as_str());
    (set != file.set || owner != Some(file.id.as_str())).then(|| {
        format!(
            "{}: ID \"{}\" belongs in src/scripts/{set}/, in a file named `{prefix}.rs` or `{prefix}_<slug>.rs` (CLAUDE.md rule 6)",
            file.path.display(),
            file.id
        )
    })
}

fn render(files: &[CardFile], by_id: &BTreeMap<&str, &CardFile>) -> String {
    let mut out = String::new();
    out.push_str("// Generated by crates/cards/build.rs (SURFACE §7.4). Do not edit.\n\n");
    out.push_str("pub mod scripts {\n");
    for set in SETS {
        let _ = writeln!(out, "    pub mod {set} {{");
        for file in files.iter().filter(|file| file.set == set) {
            let _ = writeln!(
                out,
                "        #[path = {:?}]\n        pub mod {};",
                file.path.to_string_lossy(),
                file.module
            );
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n\n");
    out.push_str("/// A card file's `script` (SURFACE §7.1).\n");
    out.push_str("pub type ScriptFn = fn() -> jackioh_engine::CardScripts;\n\n");
    out.push_str("/// Every card file's `(ID, script)`, sorted by `ID` (SURFACE §7.4).\n");
    out.push_str("pub static REGISTRY: &[(&str, ScriptFn)] = &[\n");
    for (id, file) in by_id {
        let _ = writeln!(
            out,
            "    ({id:?}, scripts::{}::{}::script),",
            file.set, file.module
        );
    }
    out.push_str("];\n");
    out
}
