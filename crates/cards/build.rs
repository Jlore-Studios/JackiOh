//! The card registry, generated at compile time (SURFACE §7.4). Replaces `gen-registry.ts`,
//! `_generated.ts`, `missing-tests.ts` and `registry.test.ts`'s completeness checks.
//!
//! Walks `src/scripts/<set>/<file>.rs` (`<set>` one of `core`, `classic`, `classic_plus`,
//! `meditative`, `meditative_plus`), reads each
//! file's `pub const ID: &str = "…";` line, and writes `$OUT_DIR/registry.rs`:
//!
//! ```text
//! pub mod scripts {
//!     pub mod core { #[path = "<abs>/src/scripts/core/c001_big_d_fender.rs"] pub mod c001_big_d_fender; … }
//!     pub mod classic { … }
//!     pub mod classic_plus { … }
//!     pub mod meditative { … }
//!     pub mod meditative_plus { … }
//! }
//! pub static REGISTRY: &[(&str, fn() -> jackioh_engine::CardScripts)] = &[ (ID, script), … ];  // sorted by ID
//! ```
//!
//! It fails the build, naming the path, for a card file without `ID`, an `ID` the catalog lacks, a
//! duplicate `ID`, a file outside `<set>/`, a file name that is not a Rust identifier, or a catalog id
//! with no file (all 318 are written; part 33 made the last one an error).
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
//! each table once. Each patch is `{ set, unset }`: the snapshot's top-level keys whose value
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
//! The crate's `clippy.toml` bans file and environment reads (CLAUDE.md rule 4); a build script runs
//! at compile time, which is exactly where data may reach a pure crate (SURFACE §3), so it may read.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The set directories under `src/scripts/`, in SPEC §5 order (SURFACE §4.1). A set that has not
/// shipped yet (R1420) has its folder too: its cards are built, tested and registered like any other.
const SETS: [&str; 5] = ["core", "classic", "classic_plus", "meditative", "meditative_plus"];

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
const BROWSER_CARRIED_FROM: &str = "v0.3.7";

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let scripts_dir = manifest.join("src").join("scripts");
    let catalog_path = manifest.join("catalog.json");
    let patches_path = manifest.join("patches").join("patches.json");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", catalog_path.display());
    println!("cargo:rerun-if-changed={}", patches_path.display());
    println!("cargo:rerun-if-changed={}", scripts_dir.display());
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

    // Every ID names a catalog card, and no two files claim one.
    let mut by_id: BTreeMap<&str, &CardFile> = BTreeMap::new();
    for file in &files {
        if !catalog_ids.iter().any(|id| id == &file.id) {
            problems.push(format!(
                "{}: ID \"{}\" is not in crates/cards/catalog.json: a script names a catalog card",
                file.path.display(),
                file.id
            ));
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
    let max_lines = read_code_block_max_lines(&config_path);
    std::fs::write(
        out_dir.join("code_blocks.rs"),
        render_code_blocks(&by_id, catalog, max_lines),
    )
    .expect("write code_blocks.rs");
}

/// R1183004: the engine's `CODE_BLOCK_MAX_LINES` (CLAUDE.md rule 9: the number lives in
/// `crates/engine/src/config.rs` alone), read off its `pub const` line.
fn read_code_block_max_lines(path: &Path) -> usize {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .find_map(|line| {
            let rest = line
                .trim()
                .strip_prefix("pub const CODE_BLOCK_MAX_LINES: usize =")?;
            rest.trim().strip_suffix(';')?.trim().parse::<usize>().ok()
        })
        .unwrap_or_else(|| {
            panic!(
                "{}: no `pub const CODE_BLOCK_MAX_LINES: usize = N;` line",
                path.display()
            )
        })
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

/// Each line of `source` with its comments taken out (`crates/tools`' `count_loc` scanner): `//` to
/// the line's end and nested `/* */` blocks, while a string (plain or raw) or a character literal is
/// read as such, so a `//` inside one is code.
fn without_comments(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut block_depth = 0usize;
    // `Some(0)` inside a plain string, `Some(n + 1)` inside a raw string of `n` hashes.
    let mut quote: Option<usize> = None;
    for line in source.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        let mut code = String::new();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            let next = chars.get(i + 1).copied();
            if block_depth > 0 {
                if ch == '*' && next == Some('/') {
                    block_depth -= 1;
                    i += 2;
                } else if ch == '/' && next == Some('*') {
                    block_depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            match quote {
                Some(0) => {
                    code.push(ch);
                    if ch == '\\' {
                        if let Some(next) = next {
                            code.push(next);
                        }
                        i += 2;
                        continue;
                    }
                    if ch == '"' {
                        quote = None;
                    }
                    i += 1;
                    continue;
                }
                Some(raw) => {
                    let hashes = raw - 1;
                    code.push(ch);
                    let closes = ch == '"'
                        && chars.len() > i + hashes
                        && chars[i + 1..=i + hashes].iter().all(|c| *c == '#');
                    if closes {
                        code.extend(std::iter::repeat_n('#', hashes));
                        quote = None;
                        i += 1 + hashes;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                None => {}
            }
            if ch == '/' && next == Some('/') {
                break;
            }
            if ch == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
                continue;
            }
            if ch == '"' {
                quote = Some(0);
                code.push(ch);
                i += 1;
                continue;
            }
            let raw_start = (ch == 'r' || (ch == 'b' && next == Some('r')))
                && !(i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'));
            if raw_start {
                let after_r = if ch == 'b' { i + 2 } else { i + 1 };
                let hashes = chars[after_r.min(chars.len())..]
                    .iter()
                    .take_while(|c| **c == '#')
                    .count();
                if chars.get(after_r + hashes) == Some(&'"') {
                    code.extend(&chars[i..=after_r + hashes]);
                    quote = Some(hashes + 1);
                    i = after_r + hashes + 1;
                    continue;
                }
            }
            if ch == '\'' {
                // A character literal (`'x'`, `'\n'`) is one token; a lifetime is a quote and a name.
                let length = if next == Some('\\') {
                    chars[i + 2..]
                        .iter()
                        .position(|c| *c == '\'')
                        .map_or(1, |at| at + 3)
                } else if chars.get(i + 2) == Some(&'\'') {
                    3
                } else {
                    1
                };
                code.extend(&chars[i..(i + length).min(chars.len())]);
                i += length;
                continue;
            }
            code.push(ch);
            i += 1;
        }
        out.push(code);
    }
    out
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
        carried.push(entry);
    }
    serde_json::json!({ "order": versions, "patches": patches, "winRates": tables, "versions": carried })
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
