//! R2173000, R2173001 (R1846003's obligation): the BUSTED and complicated lists (R1846000) read off a
//! tree's card scripts by `jackioh_cards::code_lines`, the counter `build.rs` compiles the build's lists
//! with. `patches ship` keeps each version's in `card_metrics/<version>.json`, and `classify` counts a
//! card moved across a threshold as a change to how those cards play.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, bail};
use jackioh_cards::code_lines::{code_lines, reads_card_lists};
use serde::{Deserialize, Serialize};

use crate::classify::script_id;
use crate::patches::patches_io::{Catalog, snapshot_path};

/// The folder beside the snapshots that holds each version's lists, as `win_rates/` holds its table.
pub(crate) const LISTS_DIR: &str = "card_metrics";

const SCRIPTS_REL: &str = "crates/cards/src/scripts";
const CONFIG_REL: &str = "crates/engine/src/config.rs";

/// A version's `card_metrics/<version>.json`: its BUSTED and complicated catalog ids, each sorted.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardLists {
    pub busted: Vec<String>,
    pub complicated: Vec<String>,
}

/// One tree's card scripts as the lists see them: each catalog entry's code lines, the cards whose
/// script reads the lists, and the engine's two thresholds as that tree states them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Scripts {
    pub counts: BTreeMap<String, usize>,
    pub readers: BTreeSet<String>,
    pub busted_over: usize,
    pub complicated_over: usize,
}

impl Scripts {
    /// The scripts among `sources` whose `ID` the catalog holds, with `config`'s thresholds.
    pub fn read(sources: &[String], catalog: &Catalog, config: &str) -> Result<Scripts> {
        let mut scripts = Scripts {
            busted_over: threshold(config, "BUSTED_CODE_LINES")?,
            complicated_over: threshold(config, "COMPLICATED_CODE_LINES")?,
            ..Scripts::default()
        };
        for source in sources {
            let Some(id) = script_id(source).filter(|id| catalog.contains_key(id)) else {
                continue;
            };
            scripts.counts.insert(id.clone(), code_lines(source));
            if reads_card_lists(source) {
                scripts.readers.insert(id);
            }
        }
        Ok(scripts)
    }

    /// The lists these counts make: more code lines than each threshold (R1846000), sorted by id.
    pub fn lists(&self) -> CardLists {
        let over = |threshold: usize| -> Vec<String> {
            self.counts
                .iter()
                .filter(|(_, count)| **count > threshold)
                .map(|(id, _)| id.clone())
                .collect()
        };
        CardLists {
            busted: over(self.busted_over),
            complicated: over(self.complicated_over),
        }
    }
}

/// One of the engine's `pub const NAME: usize = N;` lines in a `config.rs` (CLAUDE.md rule 9).
fn threshold(config: &str, name: &str) -> Result<usize> {
    let head = format!("pub const {name}: usize =");
    config
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix(head.as_str())?;
            rest.trim()
                .strip_suffix(';')?
                .trim()
                .replace('_', "")
                .parse()
                .ok()
        })
        .with_context(|| format!("{CONFIG_REL} has no `pub const {name}: usize = N;` line"))
}

/// `card_metrics/<version>.json` under `dir`, the lists `version` shipped with.
pub(crate) fn lists_path(version: &str, dir: &Path) -> Result<PathBuf> {
    snapshot_path(version, &dir.join(LISTS_DIR))
}

/// The lists a version's file holds, or `None` with no file.
pub(crate) fn read_lists(path: &Path) -> Result<Option<CardLists>> {
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .with_context(|| format!("{} is not {{ busted, complicated }}", path.display()))
}

/// The working tree's scripts under `repo_root`, against `catalog` (`catalog.json` as it stands). A
/// tree with no scripts folder (a fixture repository's) has none, and so no reader.
pub(crate) fn working(repo_root: &Path, catalog: &Catalog) -> Result<Scripts> {
    let dir = repo_root.join(SCRIPTS_REL);
    if !dir.is_dir() {
        return Ok(Scripts::default());
    }
    let mut sources = Vec::new();
    for set in fs::read_dir(&dir).with_context(|| format!("{} cannot be read", dir.display()))? {
        let set = set?.path();
        if !set.is_dir() {
            continue;
        }
        for file in fs::read_dir(&set).with_context(|| format!("{} cannot be read", set.display()))? {
            let file = file?.path();
            if file.extension().is_some_and(|extension| extension == "rs") {
                sources.push(
                    fs::read_to_string(&file)
                        .with_context(|| format!("{} cannot be read", file.display()))?,
                );
            }
        }
    }
    let config_path = repo_root.join(CONFIG_REL);
    let config = fs::read_to_string(&config_path)
        .with_context(|| format!("{} cannot be read", config_path.display()))?;
    Scripts::read(&sources, catalog, &config)
}

/// The scripts at commit `rev`, against that commit's `catalog`, read in one `git cat-file --batch`;
/// none for a commit with no scripts folder.
pub(crate) fn at_rev(root: &Path, rev: &str, catalog: &Catalog) -> Result<Scripts> {
    let names = crate::patches::git(
        root,
        &["ls-tree", "-r", "--name-only", rev, "--", SCRIPTS_REL],
        &[],
    )?;
    let mut wanted: Vec<String> = names
        .lines()
        .filter(|path| path.ends_with(".rs"))
        .map(|path| format!("{rev}:{path}"))
        .collect();
    // A tree with no card scripts (a fixture repository's) has none, and so no reader.
    if wanted.is_empty() {
        return Ok(Scripts::default());
    }
    wanted.push(format!("{rev}:{CONFIG_REL}"));
    let mut texts = batch(root, &wanted)?;
    let config = texts
        .pop()
        .with_context(|| format!("{rev} has no {CONFIG_REL}"))?;
    Scripts::read(&texts, catalog, &config)
}

/// Each object's text, in order, by `git cat-file --batch`; a missing one fails.
fn batch(root: &Path, objects: &[String]) -> Result<Vec<String>> {
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("git cat-file --batch did not start")?;
    let mut stdin = child.stdin.take().context("git cat-file's stdin")?;
    let request = objects
        .iter()
        .map(|object| format!("{object}\n"))
        .collect::<String>();
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));
    let mut out = Vec::new();
    child
        .stdout
        .take()
        .context("git cat-file's stdout")?
        .read_to_end(&mut out)?;
    writer
        .join()
        .map_err(|_| anyhow::anyhow!("git cat-file's writer panicked"))??;
    if !child.wait()?.success() {
        bail!("git cat-file --batch failed");
    }
    let mut texts = Vec::with_capacity(objects.len());
    let mut at = 0;
    for object in objects {
        let end = at
            + out[at..]
                .iter()
                .position(|b| *b == b'\n')
                .context("git cat-file's header")?;
        let header = String::from_utf8_lossy(&out[at..end]).to_string();
        let size: usize = match header.rsplit(' ').next().map(str::parse) {
            Some(Ok(size)) if !header.ends_with(" missing") => size,
            _ => bail!("{object}: {header}"),
        };
        let start = end + 1;
        texts.push(String::from_utf8_lossy(&out[start..start + size]).to_string());
        at = start + size + 1;
    }
    Ok(texts)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A card script of `id` with `lines` code lines, reading the lists when `reads`.
    pub(crate) fn script(id: &str, lines: usize, reads: bool) -> String {
        let mut text = format!("//! A fixture.\n\npub const ID: &str = \"{id}\";\n");
        let body = if reads {
            "let pool = json!({ \"busted\": true });"
        } else {
            "let x = 1;"
        };
        for _ in 1..lines {
            text.push_str(body);
            text.push('\n');
        }
        text.push_str("\n#[cfg(test)]\nmod tests {\n    fn long() {}\n}\n");
        text
    }

    const CONFIG: &str =
        "pub const BUSTED_CODE_LINES: usize = 100;\npub const COMPLICATED_CODE_LINES: usize = 60;\n";

    #[test]
    fn r2173000_counts_lists_and_readers_as_the_build_does() {
        let catalog: Catalog = ["core-001", "core-002", "core-003"]
            .into_iter()
            .map(|id| (id.to_string(), Default::default()))
            .collect();
        let sources = vec![
            script("core-001", 101, false),
            script("core-002", 61, true),
            script("core-003", 60, false),
            script("held-001", 500, false),
        ];
        let scripts = Scripts::read(&sources, &catalog, CONFIG).expect("read");
        assert_eq!(scripts.counts["core-001"], 101);
        assert!(!scripts.counts.contains_key("held-001"), "not in the catalog");
        assert_eq!(scripts.readers, BTreeSet::from(["core-002".to_string()]));
        assert_eq!(
            scripts.lists(),
            CardLists {
                busted: vec!["core-001".to_string()],
                complicated: vec!["core-001".to_string(), "core-002".to_string()],
            }
        );
    }

    #[test]
    fn r2173001_the_shipped_readers_are_xxfreshjack_and_shockwave447() {
        jackioh_cards::register_all();
        let root = crate::patches::repo_root();
        let catalog: Catalog = serde_json::from_str(jackioh_cards::catalog_json()).expect("the catalog");
        let scripts = working(&root, &catalog).expect("the working scripts");
        assert_eq!(
            scripts.readers,
            BTreeSet::from(["stinkyscape-109".to_string(), "stinkyscape-111".to_string()])
        );
        // The build finds the same readers (the golden replay's list test reads them, R1846003).
        let built: BTreeSet<String> = jackioh_cards::LIST_READERS
            .iter()
            .map(|id| id.to_string())
            .collect();
        assert_eq!(scripts.readers, built);
        // The tools count as the build does, and the lists they make are the build's.
        let counted: Vec<(&str, usize)> = scripts.counts.iter().map(|(id, n)| (id.as_str(), *n)).collect();
        assert_eq!(counted, jackioh_cards::CODE_LINES);
        assert_eq!(scripts.lists().busted, jackioh_cards::BUSTED);
        assert_eq!(scripts.lists().complicated, jackioh_cards::COMPLICATED);
    }
}
