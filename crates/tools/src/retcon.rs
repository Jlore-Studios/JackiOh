//! `cargo jackioh patches retcon [--dry-run] [--records <dir>...]` (#1371, part 7 of #1360,
//! R1371000): every shipped version renamed to what the classifier would have called it, by a map
//! computed, never written by hand. In `patches.json`'s order each version's tier is stage 1 of
//! `classify` over its snapshot against the one before (as `patches ship` reads it, R1366000), plus
//! the scripts its shipping commits changed from the Rust scripts on (#306), which count only beside
//! a catalog change, so the mass rewrites, shipped in no patch, claim nothing. No stage 2 runs.
//!
//! The dry run prints the map. The write renames every stored version once: the files under
//! `crates/cards/patches/` (`card_metrics/` too, R2173000) with `aliases.json`, the map, every
//! `CATALOG_VERSION` site, other files' data and string literals (never comments or docs), and the
//! database's through a generated migration. A tree holding `aliases.json` was retconned already, so a
//! second run changes nothing. `--records <dir>` rewrites and marks record files outside the repository.
//!
//! R2532000: `--from <version> [--rule <version>]...` is the v0.5 retcon (#2532) over a tree whose
//! `aliases.json` names an older scheme than `VERSIONS_SCHEME`: versions from `--from` on take the tier
//! the classifier names now (R2455000, R2499000), `--rule` naming a rule every game meets, since no
//! record holds it; `aliases.json` keeps scheme 1's names, mapped to today's, and gains `marked`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use jackioh_cards::versions::rename_versions_with;

use crate::classify::{self, FileChange, Side, Tier};
use crate::patches::js::{self, Json};
use crate::patches::patch::bump_sites;
use crate::patches::patches_io::{
    Catalog, PatchEntry, git_blob_hash, read_patches, read_shipped, read_snapshot, rebuild_derived,
    snapshot_path, win_rates_path, write_json,
};
use crate::patches::versions::name_by_tier;
use crate::patches::{git, sets_of};

/// R1371000: the first shipped version's new name, the first major.
pub const FIRST_VERSION: &str = "v0.1.0";

/// R1371000: the scheme a marked file's versions are named by. A file without the marker names
/// them the old way and is read through `aliases.json`.
pub use jackioh_engine::config::VERSIONS_SCHEME;

const PATCHES_REL: &str = "crates/cards/patches";
const ALIASES_FILE: &str = "aliases.json";
const SCRIPTS_PREFIX: &str = "crates/cards/src/scripts/";

/// One shipped version's new name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    pub old: String,
    pub new: String,
    pub tier: Tier,
    /// The families that play differently in it, sorted.
    pub plays: Vec<String>,
}

/// The stage-1 diff of one patch's own commits' card scripts: each commit against its first
/// parent, from the Rust scripts on. Empty where the history lacks a commit or the scripts.
fn script_changes(repo_root: &Path, commits: &[String]) -> anyhow::Result<Vec<FileChange>> {
    let mut files = Vec::new();
    for commit in commits {
        let parent = format!("{commit}^1");
        let Ok(names) = git(
            repo_root,
            &[
                "diff",
                "--no-renames",
                "--name-only",
                &parent,
                commit,
                "--",
                SCRIPTS_PREFIX,
            ],
            &[],
        ) else {
            continue;
        };
        for path in names.lines().filter(|path| path.ends_with(".rs")) {
            let show = |rev: &str| git(repo_root, &["cat-file", "-p", &format!("{rev}:{path}")], &[]).ok();
            files.push(FileChange {
                path: path.to_string(),
                before: show(&parent),
                after: show(commit),
            });
        }
    }
    Ok(files)
}

/// R1371000: one patch's tier and the families that play differently, from the snapshot before it
/// (`None` for the first), its own, and its commits' script changes, which count only beside a
/// catalog change.
pub fn tier_of(
    previous: Option<&Catalog>,
    snapshot: &Catalog,
    scripts: &[FileChange],
) -> (Tier, Vec<String>) {
    let before = previous.map_or_else(
        || Side::of_catalog(Catalog::new(), Vec::new()),
        |catalog| Side::of_catalog(catalog.clone(), sets_of(catalog)),
    );
    let after = Side::of_catalog(snapshot.clone(), sets_of(snapshot));
    let mut diff = classify::read_diff(&before, &after, &[]);
    if !diff.play.is_empty() || !diff.wording.is_empty() {
        diff.scripts = classify::read_diff(&before, &after, scripts).scripts;
    }
    let classification = classify::classify(&diff, None);
    (classification.tier.unwrap_or(Tier::Micro), classification.cards)
}

/// R1371000: the map, in `patches.json`'s order, from the snapshots under `dir` and, where
/// `repo_root` holds the history, the patches' own commits.
pub fn compute_map(repo_root: &Path, dir: &Path) -> anyhow::Result<Vec<Renamed>> {
    let patches = read_patches(&dir.join("patches.json"))?;
    let mut previous: Option<Catalog> = None;
    let mut newest: Option<String> = None;
    let mut map = Vec::with_capacity(patches.len());
    for PatchEntry { version, commits, .. } in patches {
        let snapshot = read_snapshot(&version, dir)?;
        let scripts = script_changes(repo_root, commits.as_deref().unwrap_or_default())?;
        let (tier, plays) = tier_of(previous.as_ref(), &snapshot, &scripts);
        let new = match &newest {
            None => FIRST_VERSION.to_string(),
            Some(newest) => {
                name_by_tier(tier, newest).with_context(|| format!("{version} cannot be named"))?
            }
        };
        newest = Some(new.clone());
        previous = Some(snapshot);
        map.push(Renamed {
            old: version,
            new,
            tier,
            plays,
        });
    }
    let names: BTreeSet<&str> = map.iter().map(|renamed| renamed.new.as_str()).collect();
    if names.len() != map.len() {
        bail!("the map names two versions alike");
    }
    Ok(map)
}

/// The map as the dry run prints it: one row per version, old, new, tier and families.
pub fn map_table(map: &[Renamed]) -> String {
    let mut out = String::from("| Old | New | Tier | Families |\n|---|---|---|---:|\n");
    for renamed in map {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            renamed.old,
            renamed.new,
            format!("{:?}", renamed.tier).to_lowercase(),
            renamed.plays.len()
        ));
    }
    out
}

/// `aliases.json`: the map, old name to new, with the scheme the new names belong to.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Aliases {
    pub versions: u32,
    pub aliases: IndexMap<String, String>,
    /// R2532000: for each older marked scheme, its names a later run renamed, to their names now.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub marked: IndexMap<String, IndexMap<String, String>>,
}

impl Aliases {
    /// R2532000: what a file of `scheme` renames to reach today's names: scheme 1's map, an older
    /// marked scheme's renames, or nothing.
    pub fn renames_from(&self, scheme: u32) -> IndexMap<String, String> {
        if scheme <= 1 {
            return self.aliases.clone();
        }
        self.marked.get(&scheme.to_string()).cloned().unwrap_or_default()
    }
}

pub fn aliases_path(repo_root: &Path) -> PathBuf {
    repo_root.join(PATCHES_REL).join(ALIASES_FILE)
}

/// R1371000: the map as `aliases.json` holds it, old name to new.
pub fn aliases_of(map: &[Renamed]) -> Aliases {
    Aliases {
        versions: VERSIONS_SCHEME,
        aliases: map
            .iter()
            .map(|renamed| (renamed.old.clone(), renamed.new.clone()))
            .collect(),
        marked: IndexMap::new(),
    }
}

/// R2532000: the second run's map: every version before `from` as it is, and from it on each by the
/// tier the classifier names now (module header), `rules` naming those whose rule every game meets.
pub fn compute_scoped_map(dir: &Path, from: &str, rules: &[String]) -> anyhow::Result<Vec<Renamed>> {
    let patches = read_patches(&dir.join("patches.json"))?;
    let start = patches
        .iter()
        .position(|patch| patch.version == from)
        .with_context(|| format!("--from {from} is no shipped version"))?;
    for rule in rules {
        if !patches[start..].iter().any(|patch| &patch.version == rule) {
            bail!("--rule {rule} names no version from {from} on");
        }
    }
    let mut previous: Option<Catalog> = None;
    let mut newest: Option<String> = None;
    let mut map = Vec::with_capacity(patches.len());
    for (at, patch) in patches.into_iter().enumerate() {
        let snapshot = read_snapshot(&patch.version, dir)?;
        let renamed = if at < start {
            Renamed {
                tier: patch
                    .tier
                    .with_context(|| format!("{} records no tier", patch.version))?,
                plays: patch.plays.unwrap_or_default(),
                new: patch.version.clone(),
                old: patch.version,
            }
        } else {
            let plays = patch.plays.unwrap_or_default();
            let rule = rules.contains(&patch.version);
            let (tier, plays) = scoped_tier(previous.as_ref(), &snapshot, &plays, rule);
            let after = newest
                .as_deref()
                .with_context(|| format!("--from {from} has no version before it"))?;
            let new =
                name_by_tier(tier, after).with_context(|| format!("{} cannot be named", patch.version))?;
            Renamed {
                old: patch.version,
                new,
                tier,
                plays,
            }
        };
        newest = Some(renamed.new.clone());
        previous = Some(snapshot);
        map.push(renamed);
    }
    let names: BTreeSet<&str> = map.iter().map(|renamed| renamed.new.as_str()).collect();
    if names.len() != map.len() {
        bail!("the map names two versions alike");
    }
    Ok(map)
}

/// R2532000: one version's tier now and its families, as `patches ship` reads a patch (R1366000).
pub fn scoped_tier(
    previous: Option<&Catalog>,
    snapshot: &Catalog,
    plays: &[String],
    rule: bool,
) -> (Tier, Vec<String>) {
    let before = previous.map_or_else(
        || Side::of_catalog(Catalog::new(), Vec::new()),
        |catalog| Side::of_catalog(catalog.clone(), sets_of(catalog)),
    );
    let after = Side::of_catalog(snapshot.clone(), sets_of(snapshot));
    let diff = classify::read_diff(&before, &after, &[]);
    let checked = classify::Checked {
        scripts: plays.iter().map(|id| classify::family(id)).collect(),
        rule,
        ..classify::Checked::default()
    };
    let classification = classify::classify(&diff, Some(&checked));
    (classification.tier.unwrap_or(Tier::Micro), classification.cards)
}

/// R2532000: the map file after a second run: scheme 1's names mapped on to today's, and the
/// renames of the scheme `held` names, under its number. A third run would compose the marked
/// schemes too, so it is refused until it is needed.
pub fn scoped_aliases(held: &Aliases, map: &[Renamed]) -> anyhow::Result<Aliases> {
    if !held.marked.is_empty() {
        bail!("aliases.json already maps a marked scheme; a third retcon must compose them");
    }
    let renames: IndexMap<String, String> = map
        .iter()
        .filter(|renamed| renamed.old != renamed.new)
        .map(|renamed| (renamed.old.clone(), renamed.new.clone()))
        .collect();
    let now = |name: &String| renames.get(name).cloned().unwrap_or_else(|| name.clone());
    Ok(Aliases {
        versions: VERSIONS_SCHEME,
        aliases: held
            .aliases
            .iter()
            .map(|(old, new)| (old.clone(), now(new)))
            .collect(),
        marked: [(held.versions.to_string(), renames)].into_iter().collect(),
    })
}

/// What a write changed, for the summary it prints.
#[derive(Debug, Default)]
pub struct Written {
    pub snapshots: usize,
    pub files: usize,
    pub renamed: usize,
    pub migration: PathBuf,
}

/// R1371000: renames stored versions through map, computing all changes before writing.
pub fn write_retcon(repo_root: &Path, map: &[Renamed]) -> anyhow::Result<Written> {
    write_with(repo_root, map, &aliases_of(map), &FIRST_RETCON)
}

/// R1371000, R2532000: the write, with the map file it leaves (`aliases`) and the migration it adds
/// (`retcon`).
fn write_with(
    repo_root: &Path,
    map: &[Renamed],
    aliases: &Aliases,
    retcon: &RetconSql,
) -> anyhow::Result<Written> {
    let dir = repo_root.join(PATCHES_REL);
    let names: IndexMap<String, String> = map
        .iter()
        .map(|renamed| (renamed.old.clone(), renamed.new.clone()))
        .collect();
    let new_of = |old: &str| -> anyhow::Result<String> {
        names
            .get(old)
            .cloned()
            .with_context(|| format!("{old} is no shipped version"))
    };
    let mut patches = read_patches(&dir.join("patches.json"))?;
    if patches.len() != map.len() {
        bail!("patches.json changed since the map was made");
    }
    let mut snapshots = Vec::new();
    let mut tables = Vec::new();
    let mut lists = Vec::new();
    for (patch, renamed) in patches.iter_mut().zip(map) {
        if patch.version != renamed.old {
            bail!("patches.json changed since the map was made");
        }
        let snapshot = snapshot_path(&patch.version, &dir)?;
        let text = fs::read_to_string(&snapshot)
            .with_context(|| format!("{} cannot be read", snapshot.display()))?;
        snapshots.push((snapshot, snapshot_path(&renamed.new, &dir)?, text));
        let table = win_rates_path(&patch.version, &dir)?;
        if let Ok(text) = fs::read_to_string(&table) {
            tables.push((
                table,
                win_rates_path(&renamed.new, &dir)?,
                rename_versions(&text, &names),
            ));
        }
        // R2173000: and its BUSTED and complicated lists, which name cards, never versions.
        let kept = crate::card_lists::lists_path(&patch.version, &dir)?;
        if let Ok(text) = fs::read_to_string(&kept) {
            lists.push((kept, crate::card_lists::lists_path(&renamed.new, &dir)?, text));
        }
        patch.version = renamed.new.clone();
        patch.tier = Some(renamed.tier);
        patch.plays = Some(renamed.plays.clone());
    }
    let mut shipped = read_shipped(&dir.join("shipped.json"))?;
    for entry in &mut shipped {
        let new = new_of(&entry.version)?;
        if let Some((_, _, text)) = tables
            .iter()
            .find(|(_, to, _)| to.file_stem().and_then(|stem| stem.to_str()) == Some(new.as_str()))
        {
            entry.win_rates_blob = Some(git_blob_hash(text));
        }
        entry.version = new;
    }
    // Any other file the history keeps (a technical log, #1369) names versions as data.
    let mut others = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("{} cannot be read", dir.display()))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let handled = ["patches.json", "shipped.json", "index.json", ALIASES_FILE].contains(&name.as_str())
            || snapshots.iter().any(|(from, _, _)| *from == path);
        if handled || !path.is_file() || !name.ends_with(".json") {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| format!("{} cannot be read", path.display()))?;
        let next = rename_versions(&text, &names);
        if next != text {
            others.push((path, next));
        }
    }
    let tree = rewrite_tree(repo_root, &names)?;
    let (number, migration) = migration_path(repo_root, retcon.suffix)?;
    let sql = migration_sql_for(number, map, retcon);
    let migration_name = migration
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let registered = registered_migration(repo_root, &tree, &migration_name, &sql)?;
    let newest = map.last().context("patches.json holds no shipped patch")?;

    for (from, _, _) in &snapshots {
        fs::remove_file(from).with_context(|| format!("{} cannot be removed", from.display()))?;
    }
    for (from, _, _) in tables.iter().chain(&lists) {
        fs::remove_file(from).with_context(|| format!("{} cannot be removed", from.display()))?;
    }
    for (_, to, text) in snapshots.iter().chain(&tables).chain(&lists) {
        fs::write(to, text).with_context(|| format!("{} cannot be written", to.display()))?;
    }
    for (path, text) in &others {
        fs::write(path, text).with_context(|| format!("{} cannot be written", path.display()))?;
    }
    write_json(&dir.join("patches.json"), &patches)?;
    write_json(&dir.join("shipped.json"), &shipped)?;
    rebuild_derived(&dir)?;
    for file in &tree {
        let from = repo_root.join(&file.from);
        let to = repo_root.join(&file.to);
        let text = match &file.text {
            Some(text) => text.clone(),
            None => {
                fs::read_to_string(&from).with_context(|| format!("{} cannot be read", from.display()))?
            }
        };
        if from != to {
            fs::remove_file(&from).with_context(|| format!("{} cannot be removed", from.display()))?;
        }
        fs::write(&to, text).with_context(|| format!("{} cannot be written", to.display()))?;
    }
    bump_sites(&newest.new, repo_root)?;
    fs::write(&migration, sql).with_context(|| format!("{} cannot be written", migration.display()))?;
    for (path, text) in &registered {
        fs::write(path, text).with_context(|| format!("{} cannot be written", path.display()))?;
    }
    // A literal the rewrite lengthened can push a line past rustfmt's width.
    if tree
        .iter()
        .any(|file| file.text.is_some() && file.to.ends_with(".rs"))
        && repo_root.join("Cargo.toml").exists()
    {
        let status = std::process::Command::new("cargo")
            .args(["fmt", "--all"])
            .current_dir(repo_root)
            .status()
            .context("cargo fmt could not run")?;
        if !status.success() {
            bail!("cargo fmt failed after the rewrite ({status})");
        }
    }
    // Last, the marker: a run that failed before here runs again over the same tree.
    write_json(&aliases_path(repo_root), aliases)?;
    Ok(Written {
        snapshots: snapshots.len(),
        files: tree.iter().filter(|file| file.text.is_some()).count() + others.len(),
        renamed: tree.iter().filter(|file| file.from != file.to).count(),
        migration,
    })
}

/// `patches retcon`'s own arguments, after `retcon`.
pub fn run(repo_root: &Path, argv: &[String]) -> anyhow::Result<()> {
    let mut dry_run = false;
    let mut records = Vec::new();
    let (mut from, mut rules) = (None, Vec::new());
    let mut args = argv.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--records" => records.push(PathBuf::from(args.next().context("--records needs a directory")?)),
            "--from" => from = Some(args.next().context("--from needs a version")?.clone()),
            "--rule" => rules.push(args.next().context("--rule needs a version")?.clone()),
            other => bail!("patches retcon: unknown argument {other}"),
        }
    }
    if !records.is_empty() {
        let aliases = read_aliases(repo_root)?;
        for dir in &records {
            let report = rewrite_records(dir, &aliases, dry_run)?;
            println!(
                "patches retcon: {}: {} record file(s) marked, {} renamed, {} already marked{}",
                dir.display(),
                report.marked,
                report.renamed,
                report.skipped,
                if dry_run {
                    " (dry run: nothing written)"
                } else {
                    ""
                }
            );
        }
        return Ok(());
    }
    if aliases_path(repo_root).exists() {
        let held = read_aliases(repo_root)?;
        if held.versions >= VERSIONS_SCHEME {
            println!(
                "patches retcon: {} names scheme {}, so this tree was retconned already; nothing changed",
                aliases_path(repo_root).display(),
                held.versions
            );
            return Ok(());
        }
        // R2532000: a tree of an older scheme takes the scoped run.
        let from = from.context(format!(
            "aliases.json names scheme {}, older than {VERSIONS_SCHEME}: name the first version to rename with --from",
            held.versions
        ))?;
        let map = compute_scoped_map(&repo_root.join(PATCHES_REL), &from, &rules)?;
        print!("{}", map_table(&map));
        let aliases = scoped_aliases(&held, &map)?;
        if dry_run {
            let renamed = map.iter().filter(|renamed| renamed.old != renamed.new).count();
            println!(
                "patches retcon --dry-run: {renamed} of {} versions renamed from {from}; nothing written",
                map.len()
            );
            return Ok(());
        }
        let written = write_with(repo_root, &map, &aliases, &V05_RETCON)?;
        println!(
            "patches retcon: renamed the versions from {from} ({} snapshots), rewrote {} file(s), renamed {}, wrote {} and {}",
            written.snapshots,
            written.files,
            written.renamed,
            written.migration.display(),
            aliases_path(repo_root).display()
        );
        return Ok(());
    }
    if from.is_some() || !rules.is_empty() {
        bail!("--from and --rule are for a tree retconned before (aliases.json)");
    }
    let map = compute_map(repo_root, &repo_root.join(PATCHES_REL))?;
    print!("{}", map_table(&map));
    if dry_run {
        let names = aliases_of(&map).aliases;
        let tree = rewrite_tree(repo_root, &names)?;
        println!(
            "patches retcon --dry-run: {} versions; the write would rewrite {} file(s) and rename {}; nothing written",
            map.len(),
            tree.iter().filter(|file| file.text.is_some()).count(),
            tree.iter().filter(|file| file.from != file.to).count()
        );
        return Ok(());
    }
    let written = write_retcon(repo_root, &map)?;
    println!(
        "patches retcon: renamed {} versions ({} snapshots), rewrote {} file(s), renamed {}, wrote {} and {}",
        map.len(),
        written.snapshots,
        written.files,
        written.renamed,
        written.migration.display(),
        aliases_path(repo_root).display()
    );
    Ok(())
}

/// `aliases.json`, which a retconned tree holds.
pub fn read_aliases(repo_root: &Path) -> anyhow::Result<Aliases> {
    let path = aliases_path(repo_root);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("{} cannot be read: run `patches retcon` first", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("{} is not a version map", path.display()))
}

/* the tree */

/// Paths the tree rewrite leaves alone: the patch history (rewritten field by field), the applied
/// migrations (history the database already ran), the docs and the spec (which keep their old
/// names), and what no agent may change.
const SKIP_PREFIXES: &[&str] = &[
    ".github/",
    ".harness/",
    ".squishy/",
    ".claude/",
    "bot/",
    "docs/",
    "spec/",
    "crates/cards/patches/",
    "crates/server/migrations/",
];

/// Files whose versions are not the shipped ones: fixtures that name made-up versions, and the old
/// retcon's test, which re-runs migration 0025 over its own names (R743).
const SKIP_FILES: &[&str] = &[
    "crates/cards/src/versions.rs",
    "apps/web/src/replay/retcon.test.ts",
    "apps/web/src/replay/retcon-v05.test.ts",
    "crates/tools/src/patches.rs",
    "crates/tools/src/classify.rs",
    "crates/tools/src/retcon.rs",
    "crates/server/tests/sql/14_patch_retcon.sql",
    "crates/server/tests/sql/28_version_retcon.sql",
    "crates/server/tests/sql/38_v0_5_retcon.sql",
];

/// How a file's versions are found: everywhere in a data file, in a code file's string literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    Data,
    Rust,
    Script,
    Python,
    Sql,
}

fn syntax_of(path: &str) -> Option<Syntax> {
    let extension = path.rsplit_once('.').map(|(_, extension)| extension)?;
    Some(match extension {
        "json" | "jsonl" | "yaml" | "yml" | "csv" => Syntax::Data,
        "rs" => Syntax::Rust,
        "ts" | "tsx" | "js" | "mjs" | "cjs" => Syntax::Script,
        "py" => Syntax::Python,
        "sql" => Syntax::Sql,
        _ => return None,
    })
}

/// `text` with every old version that stands alone renamed, each once (`jackioh_cards::versions`).
pub fn rename_versions(text: &str, map: &IndexMap<String, String>) -> String {
    rename_versions_with(text, |old| map.get(old).map(String::as_str))
}

/// The byte spans of a code file's string literals, its comments skipped.
fn string_spans(text: &str, syntax: Syntax) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut at = 0;
    let line_comment: &[u8] = match syntax {
        Syntax::Python => b"#",
        Syntax::Sql => b"--",
        _ => b"//",
    };
    let block_comments = matches!(syntax, Syntax::Rust | Syntax::Script | Syntax::Sql);
    while at < bytes.len() {
        let rest = &bytes[at..];
        if rest.starts_with(line_comment) {
            at += rest.iter().position(|b| *b == b'\n').unwrap_or(rest.len());
            continue;
        }
        if block_comments && rest.starts_with(b"/*") {
            let mut depth = 0usize;
            while at < bytes.len() {
                if bytes[at..].starts_with(b"/*") {
                    depth += 1;
                    at += 2;
                } else if bytes[at..].starts_with(b"*/") {
                    depth -= 1;
                    at += 2;
                    if depth == 0 || syntax != Syntax::Rust {
                        break;
                    }
                } else {
                    at += 1;
                }
            }
            continue;
        }
        // Rust's raw strings: r"…", r#"…"#, br"…".
        if syntax == Syntax::Rust
            && (rest.starts_with(b"r\"") || rest.starts_with(b"r#") || rest.starts_with(b"br"))
            && (at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_'))
        {
            let open = if rest[0] == b'b' { 2 } else { 1 };
            let hashes = rest[open..].iter().take_while(|b| **b == b'#').count();
            if rest.get(open + hashes) == Some(&b'"') {
                let start = at + open + hashes + 1;
                let mut close = b"\"".to_vec();
                close.extend(std::iter::repeat_n(b'#', hashes));
                let end = bytes[start..]
                    .windows(close.len())
                    .position(|window| window == close.as_slice())
                    .map_or(bytes.len(), |found| start + found);
                spans.push((start, end));
                at = (end + close.len()).min(bytes.len());
                continue;
            }
        }
        let quote = rest[0];
        let opens = match syntax {
            Syntax::Rust => quote == b'"' || (quote == b'\'' && rust_char_literal(rest)),
            Syntax::Script => matches!(quote, b'"' | b'\'' | b'`'),
            Syntax::Python | Syntax::Sql => matches!(quote, b'"' | b'\''),
            Syntax::Data => false,
        };
        if !opens {
            at += 1;
            continue;
        }
        let triple = syntax == Syntax::Python && rest.len() >= 3 && rest[1] == quote && rest[2] == quote;
        let width = if triple { 3 } else { 1 };
        let start = at + width;
        let mut end = start;
        while end < bytes.len() {
            if bytes[end] == b'\\' && syntax != Syntax::Sql {
                end += 2;
                continue;
            }
            if bytes[end] == quote && (!triple || bytes[end..].starts_with(&[quote, quote, quote])) {
                break;
            }
            end += 1;
        }
        let end = end.min(bytes.len());
        spans.push((start, end));
        at = (end + width).min(bytes.len());
    }
    spans
}

/// Whether a Rust `'` opens a char literal rather than a lifetime: `'x'` or `'\…'`.
fn rust_char_literal(rest: &[u8]) -> bool {
    let width = match rest.get(1) {
        Some(b'\\') => return true,
        Some(lead) if *lead < 0x80 => 1,
        Some(lead) if *lead < 0xE0 => 2,
        Some(lead) if *lead < 0xF0 => 3,
        Some(_) => 4,
        None => return false,
    };
    rest.get(1 + width) == Some(&b'\'')
}

/// A file's text with its old versions renamed, by its syntax.
pub fn rewrite_text(text: &str, syntax: Syntax, map: &IndexMap<String, String>) -> String {
    if syntax == Syntax::Data {
        return rename_versions(text, map);
    }
    let mut out = String::with_capacity(text.len());
    let mut kept = 0;
    for (start, end) in string_spans(text, syntax) {
        out.push_str(&text[kept..start]);
        out.push_str(&rename_versions(&text[start..end], map));
        kept = end;
    }
    out.push_str(&text[kept..]);
    out
}

/// One file the tree rewrite changes: its path before and after, and its new text.
pub struct Rewritten {
    pub from: String,
    pub to: String,
    pub text: Option<String>,
}

/// Every tracked file outside the skipped paths that spells an old version, in its text or its name.
pub fn rewrite_tree(repo_root: &Path, map: &IndexMap<String, String>) -> anyhow::Result<Vec<Rewritten>> {
    let files = git(repo_root, &["ls-files", "-z"], &[])?;
    let mut out = Vec::new();
    for path in files.split('\0').filter(|path| !path.is_empty()) {
        if path.ends_with(".md")
            || SKIP_FILES.contains(&path)
            || SKIP_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
        {
            continue;
        }
        let renamed_path = match path.rsplit_once('/') {
            Some((dir, name)) => format!("{dir}/{}", rename_versions(name, map)),
            None => rename_versions(path, map),
        };
        let text = match syntax_of(path) {
            Some(syntax) => {
                let Ok(text) = fs::read_to_string(repo_root.join(path)) else {
                    continue;
                };
                let next = rewrite_text(&text, syntax, map);
                (next != text).then_some(next)
            }
            None => None,
        };
        if text.is_some() || renamed_path != path {
            out.push(Rewritten {
                from: path.to_string(),
                to: renamed_path,
                text,
            });
        }
    }
    Ok(out)
}

/* the migration */

const MIGRATIONS_REL: &str = "crates/server/migrations";

/// Which retcon a migration is: the issue its temporary table and function are named after, its
/// heading, the scheme marker a game record takes, and its file's suffix.
pub struct RetconSql {
    pub issue: u32,
    pub title: &'static str,
    pub marker: u32,
    pub suffix: &'static str,
}

/// R1371000: the version retcon's, migration 0049.
pub const FIRST_RETCON: RetconSql = RetconSql {
    issue: 1371,
    title: "the version retcon (issue #1371, part 7 of #1360, R1371000)",
    marker: 2,
    suffix: "_version_retcon.sql",
};

/// R2532000: the v0.5 retcon's.
pub const V05_RETCON: RetconSql = RetconSql {
    issue: 2532,
    title: "the v0.5 retcon (issue #2532, R2532000)",
    marker: VERSIONS_SCHEME,
    suffix: "_v0_5_retcon.sql",
};

/// R1371000: the season a version plays in (R609's `v<major>.<minor>`), `None` for a name it cannot read.
fn season_of(version: &str) -> Option<String> {
    let (a, b, _) = crate::patches::versions::split_version(version)?;
    let number = |digits: &str| digits.trim_start_matches('0').to_string();
    let (a, b) = (number(a), number(b));
    Some(format!(
        "v{}.{}",
        if a.is_empty() { "0" } else { &a },
        if b.is_empty() { "0" } else { &b }
    ))
}

/// R1371000: the migration that renames every catalog version the database holds, once: one statement
/// per column joined to the map, so a name the map reuses is renamed from its old value only, and a key
/// column goes through a `~` name first, so no row meets a name another still holds. R2532000:
/// `retcon` names the run, the first (`FIRST_RETCON`) or the v0.5 retcon (`V05_RETCON`).
pub fn migration_sql_for(number: u32, map: &[Renamed], retcon: &RetconSql) -> String {
    let RetconSql {
        issue, title, marker, ..
    } = retcon;
    let renamed: Vec<&Renamed> = map.iter().filter(|renamed| renamed.old != renamed.new).collect();
    let pairs: Vec<String> = renamed
        .iter()
        .map(|renamed| format!("  ('{}', '{}')", renamed.old, renamed.new))
        .collect();
    // R2532000: the renaming function holds the map as two constant arrays, so a call never queries,
    // the longest names first, so a name another holds (v0.5.1 in v0.5.10) is looked for after it.
    let mut renamed = renamed;
    renamed.sort_by(|a, b| b.old.len().cmp(&a.old.len()).then_with(|| a.old.cmp(&b.old)));
    let quoted = |names: Vec<&str>| {
        names
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let olds = quoted(renamed.iter().map(|renamed| renamed.old.as_str()).collect());
    let news = quoted(renamed.iter().map(|renamed| renamed.new.as_str()).collect());
    let mut table = String::new();
    for renamed in map {
        table.push_str(&format!("--   {:<9} {}\n", renamed.old, renamed.new));
    }
    let season = map
        .last()
        .and_then(|last| Some((season_of(&last.old)?, season_of(&last.new)?)));
    let season_sql = match season {
        Some((old, new)) if old != new => format!(
            r#"
-- The ladder's season is named by the live version's minor (R609), which the
-- retcon moves from {old} to {new} with no new expansion: the season in play
-- takes the new name, so no build opens a new season, nor soft-resets every
-- rated player, for a rename (R1371001). Its ranks and rated games follow it.
-- An older season keeps its name, a label of its time.
do $$
begin
  if exists (select 1 from public.seasons where id = '{old}')
     and not exists (select 1 from public.seasons where id = '{new}') then
    insert into public.seasons (id, patch_version, started_at)
      select '{new}', patch_version, started_at from public.seasons where id = '{old}';
    update public.season_ranks set season_id = '{new}' where season_id = '{old}';
    update public.rated_games set season_id = '{new}' where season_id = '{old}';
    delete from public.seasons where id = '{old}';
  end if;
end $$;
"#
        ),
        _ => String::new(),
    };
    format!(
        r#"-- ============================================================================
-- Migration {number:04}: {title}
-- ============================================================================
-- GENERATED by `cargo jackioh patches retcon` from its map; never edit it.
--
-- Every shipped catalog version is renamed to what the classifier calls it
-- (R1365000, R1366000), each exactly once:
--
--   old       new
{table}--
-- Old and new names overlap (a new name can be another version's old one), so
-- no name is renamed twice: each column is
-- renamed in one statement joined to the map on its old value, and a column a
-- key holds (a record id, a totals table's patch) goes through a `~` name
-- first. Not safe to re-apply (the migration runner applies each file once).
--
-- What moves: the catalog version of `cards`, `tickets`, `matches`, `decks`,
-- `series` (whose compare-and-set version steps, R263), `match_checkpoints`,
-- a bot's decks and trio, a Duos match's options and a practice seed's setup
-- (0047); the patch and catalog version of `rated_games` and the patch
-- version of `seasons`; a game record's
-- patch and id (`dev:<patch>:<series>:<n>`, its series' own versions too),
-- with their copies in its record, which takes the scheme marker
-- (`versions: {marker}`) so an export of it is never read through the map again,
-- each of a renamed version alone; the totals' patches (`game_totals`' among
-- them, 0063) and the record ids they keep; and `app.settings`' catalog
-- version, last, after the `cards` it names, so its launch grant (0016) finds
-- every card granted. What stays, as in 0025: `collection_grants` is
-- append-only and keeps its `launch:<version>` references, which nothing
-- reads; the loadout tables are never written again (R254).
-- ============================================================================

create temporary table retcon_{issue} (
  old text primary key,
  new text not null unique
) on commit drop;

insert into retcon_{issue} (old, new) values
{pairs};

-- Every version in a text, each renamed once (`rename_versions` in
-- crates/tools/src/retcon.rs): a `v<n>.<n>.<n>` and at most one letter, with
-- no letter, digit, `_` or `.` before it and no letter or digit, nor a dot
-- and a digit, after it. Each old name becomes a placeholder first and each
-- placeholder its new name after, so a name another one is renamed to is
-- never renamed again; every pass is one `regexp_replace` or `replace` over
-- the whole text, taken only for a name the text holds, so a long text (a
-- card's co-played map) costs a pass per name it holds, never a call per
-- version (R2532000: 0049's call per version ran for minutes over a season of
-- totals and timed the deploy out).
create function pg_temp.retcon_{issue}(s text) returns text
language plpgsql immutable as $fn$
declare
  olds constant text[] := array[{olds}]::text[];
  news constant text[] := array[{news}]::text[];
begin
  if s is null or s !~ 'v[0-9]' then
    return s;
  end if;
  for k in 1 .. coalesce(array_length(olds, 1), 0) loop
    if strpos(s, olds[k]) > 0 then
      -- A record id's version stands between `:`s (its patch) or `-`s (a
      -- series'); those are renamed by `replace`, anything else by the regex.
      s := replace(replace(s, ':' || olds[k] || ':', ':' || chr(1) || k || chr(2) || ':'),
                   '-' || olds[k] || '-', '-' || chr(1) || k || chr(2) || '-');
      if strpos(s, olds[k]) > 0 then
        s := regexp_replace(s, '(?<![0-9A-Za-z_.])' || replace(olds[k], '.', '\.') || '(?![0-9A-Za-z]|\.[0-9])',
                            chr(1) || k || chr(2), 'g');
      end if;
    end if;
  end loop;
  if strpos(s, chr(1)) = 0 then
    return s;
  end if;
  for k in 1 .. coalesce(array_length(news, 1), 0) loop
    s := replace(s, chr(1) || k || chr(2), news[k]);
  end loop;
  return s;
end $fn$;

update public.cards t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.tickets t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.matches t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.decks t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.series t set catalog_version = m.new, version = t.version + 1
  from retcon_{issue} m where t.catalog_version = m.old;
update public.match_checkpoints t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.bots t set decks_catalog_version = m.new
  from retcon_{issue} m where t.decks_catalog_version = m.old;
update public.bots t set trio_catalog_version = m.new
  from retcon_{issue} m where t.trio_catalog_version = m.old;
update public.duos_matches t
   set options = jsonb_set(t.options, '{{catalogVersion}}', to_jsonb(m.new))
  from retcon_{issue} m where t.options ->> 'catalogVersion' = m.old;
update public.practice_seeds t
   set setup = jsonb_set(t.setup, '{{catalogVersion}}', to_jsonb(m.new))
  from retcon_{issue} m where t.setup ->> 'catalogVersion' = m.old;
update public.rated_games t set patch_version = m.new
  from retcon_{issue} m where t.patch_version = m.old;
update public.rated_games t set catalog_version = m.new
  from retcon_{issue} m where t.catalog_version = m.old;
update public.seasons t set patch_version = m.new
  from retcon_{issue} m where t.patch_version = m.old;

-- A game record of a renamed version: its patch, its id and their copies in
-- one statement (`game_records_copies_check`), a development id through `dev:~`
-- first (`game_records_dev_id_check` holds throughout), then back. A record
-- names the version it was played on, and its id a series of that version or
-- an earlier one (`learned-<version>-…`), so no other record holds a renamed
-- name, and the rest are left as they are.
update public.game_records g
   set patch = r.patch,
       id = case when r.renamed = g.id then g.id else 'dev:~' || substr(r.renamed, 5) end,
       record = case when g.record is null then null
                     else g.record || jsonb_build_object(
                       'id', case when r.renamed = g.id then g.id else 'dev:~' || substr(r.renamed, 5) end,
                       'patch', r.patch, 'versions', {marker}) end
  from (select id, pg_temp.retcon_{issue}(id) as renamed,
               coalesce((select m.new from retcon_{issue} m where m.old = patch), patch) as patch
          from public.game_records
         where patch in (select old from retcon_{issue})) r
 where g.id = r.id;
update public.game_records
   set id = 'dev:' || substr(id, 6),
       record = case when record is null then null
                     else record || jsonb_build_object('id', 'dev:' || substr(id, 6)) end
 where starts_with(id, 'dev:~');

-- The totals (0041): their patch is in their key, so through `~` first, and a
-- renamed version's rows take the record ids they keep (a card's co-played
-- map, a decklist's first record) renamed in the same statement. As with the
-- records, only a renamed version's rows can hold a renamed name.
update public.record_totals t set patch = '~' || m.new
  from retcon_{issue} m where t.patch = m.old;
update public.record_totals set patch = substr(patch, 2) where starts_with(patch, '~');
update public.card_totals t
   set patch = '~' || m.new,
       co = pg_temp.retcon_{issue}(t.co::text)::jsonb
  from retcon_{issue} m where t.patch = m.old;
update public.card_totals set patch = substr(patch, 2) where starts_with(patch, '~');
update public.decklist_totals t
   set patch = '~' || m.new,
       first_record = pg_temp.retcon_{issue}(t.first_record)
  from retcon_{issue} m where t.patch = m.old;
update public.decklist_totals set patch = substr(patch, 2) where starts_with(patch, '~');
-- The whole games' totals (0063), keyed by patch as the others are.
update public.game_totals t set patch = '~' || m.new
  from retcon_{issue} m where t.patch = m.old;
update public.game_totals set patch = substr(patch, 2) where starts_with(patch, '~');
{season_sql}
-- Last: the live version, after the cards it names.
update app.settings s set value = to_jsonb(m.new)
  from retcon_{issue} m
 where s.key = 'catalog_version' and s.value = to_jsonb(m.old);
"#,
        pairs = pairs.join(",\n"),
    )
}

const MIGRATE_REL: &str = "crates/server/src/db/migrate.rs";
const PINNED_REL: &str = "crates/server/tests/store/migrations_pinned.rs";

/// `text` with `entry` put before the `\n];` that closes the list opening at `list`.
fn insert_in_list(text: &str, list: &str, entry: &str) -> anyhow::Result<String> {
    let start = text.find(list).with_context(|| format!("no `{list}`"))?;
    let close = start
        + text[start..]
            .find("\n];")
            .with_context(|| format!("`{list}` does not close"))?;
    Ok(format!("{}\n{entry}{}", &text[..close], &text[close..]))
}

/// The server's two lists of migrations with the retcon's added: the runner's embedded files and the
/// pinned checksums (`tests/store/migrations_pinned.rs`), each `(path, text)`.
fn registered_migration(
    repo_root: &Path,
    tree: &[Rewritten],
    name: &str,
    sql: &str,
) -> anyhow::Result<Vec<(PathBuf, String)>> {
    // A file the tree rewrite changes is read as it will be written.
    let read = |rel: &str| {
        let path = repo_root.join(rel);
        match tree
            .iter()
            .find(|file| file.from == rel)
            .and_then(|file| file.text.clone())
        {
            Some(text) => Ok((path, text)),
            None => fs::read_to_string(&path).map(|text| (path, text)),
        }
    };
    let mut out = Vec::new();
    if let Ok((path, text)) = read(MIGRATE_REL)
        && !text.contains(name)
    {
        let entry =
            format!("    (\n        \"{name}\",\n        include_str!(\"../../migrations/{name}\"),\n    ),");
        out.push((path, insert_in_list(&text, "pub const MIGRATIONS", &entry)?));
    }
    if let Ok((path, text)) = read(PINNED_REL)
        && !text.contains(name)
    {
        let sum = jackioh_engine::replay::fnv1a32_utf16(sql);
        out.push((
            path,
            insert_in_list(&text, "const PINNED", &format!("    (\"{name}\", \"{sum}\"),"))?,
        ));
    }
    Ok(out)
}

/// The migration's path: the one this retcon wrote (`suffix`), else the next number after the newest.
fn migration_path(repo_root: &Path, suffix: &str) -> anyhow::Result<(u32, PathBuf)> {
    let dir = repo_root.join(MIGRATIONS_REL);
    let mut newest = 0;
    for entry in fs::read_dir(&dir).with_context(|| format!("{} cannot be read", dir.display()))? {
        let name = entry?.file_name().to_string_lossy().to_string();
        let Some((digits, _)) = name.split_once('_') else {
            continue;
        };
        let Ok(number) = digits.parse::<u32>() else {
            continue;
        };
        if name.ends_with(suffix) {
            return Ok((number, dir.join(name)));
        }
        newest = newest.max(number);
    }
    let number = newest + 1;
    Ok((number, dir.join(format!("{number:04}{suffix}"))))
}

/* records outside the repository */

/// What `--records` did under one directory.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RecordsReport {
    /// Files whose objects took the marker, their versions renamed.
    pub marked: usize,
    /// Files whose name named an old version.
    pub renamed: usize,
    /// Files already marked, or holding no object a marker fits.
    pub skipped: usize,
}

/// A value with every version in its strings and keys renamed.
fn rename_json(value: &Json, map: &IndexMap<String, String>) -> Json {
    match value {
        Json::String(text) => Json::String(rename_versions(text, map)),
        Json::Array(items) => Json::Array(items.iter().map(|item| rename_json(item, map)).collect()),
        Json::Object(object) => Json::Object(
            object
                .iter()
                .map(|(key, value)| (rename_versions(key, map), rename_json(value, map)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// R1371000: one record object marked: `None` when it carries today's marker already (or is no
/// object), else its versions renamed from the scheme it names (R2532000: 1 unmarked, else its
/// `versions`) and `versions: VERSIONS_SCHEME` set.
pub fn mark_record(value: &Json, aliases: &Aliases) -> Option<Json> {
    let object = value.as_object()?;
    let scheme = match object.get("versions") {
        None => 1,
        Some(Json::Number(scheme)) if *scheme >= f64::from(VERSIONS_SCHEME) => return None,
        Some(Json::Number(scheme)) => *scheme as u32,
        Some(_) => return None,
    };
    let Json::Object(mut renamed) = rename_json(value, &aliases.renames_from(scheme)) else {
        return None;
    };
    renamed.insert("versions".to_string(), Json::Number(f64::from(VERSIONS_SCHEME)));
    Some(Json::Object(renamed))
}

/// Whether a record object carries today's marker.
fn marked_now(value: &Json) -> bool {
    matches!(
        value.as_object().and_then(|object| object.get("versions")),
        Some(Json::Number(scheme)) if *scheme >= f64::from(VERSIONS_SCHEME)
    )
}

/// A record file's new text: a `.json` file's one object, or a `.jsonl` file's every line, marked;
/// `None` when the file is marked already or holds no object to mark.
fn marked_text(text: &str, lines: bool, aliases: &Aliases) -> anyhow::Result<Option<String>> {
    if !lines {
        let value = js::parse(text).context("not JSON")?;
        return Ok(mark_record(&value, aliases).map(|marked| format!("{}\n", js::stringify(&marked))));
    }
    let mut out = String::with_capacity(text.len());
    let mut any = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let value = js::parse(line).context("a line is not JSON")?;
        match mark_record(&value, aliases) {
            Some(marked) => {
                any = true;
                out.push_str(&js::stringify(&marked));
            }
            None if marked_now(&value) => return Ok(None),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    Ok(any.then_some(out))
}

/// R1371000: the file `--records` leaves in a directory it rewrote, `.jackioh-versions-<scheme>`. A
/// name carries no marker of its own, so a directory holding today's is never renamed again, and one
/// holding an older one (R2532000) has its names read as that scheme's.
pub const RECORDS_MARKER_PREFIX: &str = ".jackioh-versions-";

/// The scheme a directory's file names are in: the newest `RECORDS_MARKER_PREFIX` file it holds, else 1.
fn records_scheme(dir: &Path) -> u32 {
    (2..=VERSIONS_SCHEME)
        .rev()
        .find(|scheme| dir.join(format!("{RECORDS_MARKER_PREFIX}{scheme}")).exists())
        .unwrap_or(1)
}

/// R1371000: `--records <dir>`: every `.json` and `.jsonl` file under `dir` (record files, self-play
/// and positions files, weights) marked and its versions renamed, and every file whose name names an
/// old version renamed. A marked file's text is left as it is, and a directory holding
/// today's `RECORDS_MARKER_PREFIX` file is left whole, so a second run changes nothing.
pub fn rewrite_records(dir: &Path, aliases: &Aliases, dry_run: bool) -> anyhow::Result<RecordsReport> {
    let mut report = RecordsReport::default();
    let scheme = records_scheme(dir);
    if scheme >= VERSIONS_SCHEME {
        return Ok(report);
    }
    let map = aliases.renames_from(scheme);
    let mut pending = vec![dir.to_path_buf()];
    let mut files = Vec::new();
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).with_context(|| format!("{} cannot be read", next.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    for path in files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let lines = name.ends_with(".jsonl");
        let text = if lines || name.ends_with(".json") {
            let text =
                fs::read_to_string(&path).with_context(|| format!("{} cannot be read", path.display()))?;
            let marked = marked_text(&text, lines, aliases).with_context(|| format!("{}", path.display()))?;
            if marked.is_none() {
                report.skipped += 1;
            }
            marked
        } else {
            None
        };
        let renamed = rename_versions(&name, &map);
        let to = path.with_file_name(&renamed);
        if text.is_some() {
            report.marked += 1;
        }
        if renamed != name {
            report.renamed += 1;
            if to.exists() {
                bail!(
                    "{} would be renamed onto {}, which exists",
                    path.display(),
                    to.display()
                );
            }
        }
        if dry_run {
            continue;
        }
        if let Some(text) = text {
            fs::write(&path, text).with_context(|| format!("{} cannot be written", path.display()))?;
        }
        if renamed != name {
            fs::rename(&path, &to).with_context(|| format!("{} cannot be renamed", path.display()))?;
        }
    }
    if !dry_run {
        fs::write(
            dir.join(format!("{RECORDS_MARKER_PREFIX}{VERSIONS_SCHEME}")),
            format!("{VERSIONS_SCHEME}\n"),
        )
        .with_context(|| format!("{} cannot be marked", dir.display()))?;
    }
    Ok(report)
}

/* tests */

// This file spells old names on purpose (`SKIP_FILES`): its tests read the retconned repository
// through them, as an unmarked file would.
#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use serde_json::json;

    use super::*;
    use crate::patches::repo_root;

    /// A directory under the system temp dir, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("jackioh-retcon-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("a temp dir");
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn card(id: &str, set: &str, name: &str, cost: u32) -> serde_json::Value {
        json!({ "id": id, "set": set, "name": name, "cost": cost })
    }

    fn catalog(cards: &[serde_json::Value]) -> String {
        let object: serde_json::Map<String, serde_json::Value> = cards
            .iter()
            .map(|card| (card["id"].as_str().expect("an id").to_string(), card.clone()))
            .collect();
        format!("{}\n", serde_json::to_string_pretty(&object).expect("JSON"))
    }

    fn patch(version: &str) -> serde_json::Value {
        json!({ "version": version, "date": "2026-10-01", "title": version, "source": "#1", "notes": "n", "changes": [] })
    }

    /// A repository of four shipped versions: Core, a wording change, a Classic release, a cost.
    fn fixture() -> TempDir {
        let dir = TempDir::new();
        let root = &dir.0;
        let patches = root.join(PATCHES_REL);
        fs::create_dir_all(&patches).expect("a folder");
        fs::create_dir_all(root.join(MIGRATIONS_REL)).expect("a folder");
        fs::create_dir_all(root.join("crates/server")).expect("a folder");
        fs::create_dir_all(root.join("src")).expect("a folder");
        let one = card("core-001", "Core", "One", 1);
        let snapshots = [
            ("v0.1.0", vec![one.clone()]),
            ("v0.1.1", vec![card("core-001", "Core", "Uno", 1)]),
            (
                "v0.1.2",
                vec![
                    card("core-001", "Core", "Uno", 1),
                    card("classic-001", "Classic", "Two", 2),
                ],
            ),
            (
                "v0.1.3",
                vec![
                    card("core-001", "Core", "Uno", 3),
                    card("classic-001", "Classic", "Two", 2),
                ],
            ),
        ];
        for (version, cards) in &snapshots {
            fs::write(patches.join(format!("{version}.json")), catalog(cards)).expect("a snapshot");
        }
        let list: Vec<_> = snapshots.iter().map(|(version, _)| patch(version)).collect();
        fs::write(
            patches.join("patches.json"),
            serde_json::to_string_pretty(&list).expect("JSON"),
        )
        .expect("a list");
        let shipped: Vec<_> = snapshots
            .iter()
            .map(|(version, _)| json!({ "version": version, "commit": "c", "blob": "b" }))
            .collect();
        fs::write(
            patches.join("shipped.json"),
            serde_json::to_string_pretty(&shipped).expect("JSON"),
        )
        .expect("a list");
        fs::write(root.join(MIGRATIONS_REL).join("0001_start.sql"), "-- v0.1.1\n").expect("a migration");
        fs::write(
            root.join("crates/server/.env.example"),
            "CATALOG_VERSION=v0.1.3\n",
        )
        .expect("a site");
        fs::write(
            root.join("render.yaml"),
            "services:\n  - envVars:\n      - key: CATALOG_VERSION\n        value: v0.1.3\n",
        )
        .expect("a site");
        fs::write(
            root.join("src/lib.rs"),
            "// Since v0.1.1.\nconst A: &str = \"v0.1.1\";\nconst B: &str = \"dev:v0.1.2:learned-v0.1.3-1:4\";\n",
        )
        .expect("a source");
        fs::write(
            root.join("src/seed-1-v0.1.1.json"),
            "{\"catalogVersion\":\"v0.1.1\"}\n",
        )
        .expect("a corpus file");
        for args in [&["init", "-q"][..], &["add", "-A"][..]] {
            let status = Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .expect("git runs");
            assert!(status.success());
        }
        dir
    }

    #[test]
    fn r1371000_names_follow_the_tiers_from_v0_1_0() {
        let dir = fixture();
        let map = compute_map(&dir.0, &dir.0.join(PATCHES_REL)).expect("a map");
        let names: Vec<(&str, &str, Tier)> = map
            .iter()
            .map(|renamed| (renamed.old.as_str(), renamed.new.as_str(), renamed.tier))
            .collect();
        assert_eq!(
            names,
            [
                ("v0.1.0", "v0.1.0", Tier::Major),
                ("v0.1.1", "v0.1.0b", Tier::Micro),
                ("v0.1.2", "v0.2.0", Tier::Major),
                ("v0.1.3", "v0.2.0b", Tier::Micro),
            ]
        );
        assert_eq!(map[3].plays, ["core-001"]);
        assert!(map[1].plays.is_empty(), "a wording change plays no differently");
    }

    #[test]
    fn r1371000_a_script_change_counts_only_beside_a_catalog_change() {
        let core = |name: &str| catalog(&[card("core-001", "Core", name, 1)]);
        let (before, same, renamed): (Catalog, Catalog, Catalog) = (
            serde_json::from_str(&core("One")).expect("a catalog"),
            serde_json::from_str(&core("One")).expect("a catalog"),
            serde_json::from_str(&core("Uno")).expect("a catalog"),
        );
        let script = |body: &str| {
            Some(format!(
                "pub const ID: &str = \"core-001\";\npub fn script() {{ {body} }}\n"
            ))
        };
        let scripts = [FileChange {
            path: "crates/cards/src/scripts/core/c001_one.rs".to_string(),
            before: script("a()"),
            after: script("b()"),
        }];
        assert_eq!(tier_of(Some(&before), &same, &scripts), (Tier::Micro, Vec::new()));
        assert_eq!(
            tier_of(Some(&before), &renamed, &scripts),
            (Tier::Micro, vec!["core-001".to_string()])
        );
    }

    #[test]
    fn r1371000_a_code_file_has_its_strings_renamed_never_its_comments() {
        let map: IndexMap<String, String> = [("v0.3.4", "v0.2.7b"), ("v0.3.12", "v0.3.4")]
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .into();
        let rust = "// v0.3.4 shipped first\nlet a = \"v0.3.4\"; /* v0.3.12 */ let b = r#\"[\"v0.3.12\"]\"#;\nlet c = 'v'; fn f<'a>() {}\nlet d = \"v0.3.4b v0.3.40 v0.3.4.1 xv0.3.4 dev:v0.3.12:x\";\n";
        assert_eq!(
            rewrite_text(rust, Syntax::Rust, &map),
            "// v0.3.4 shipped first\nlet a = \"v0.2.7b\"; /* v0.3.12 */ let b = r#\"[\"v0.3.4\"]\"#;\nlet c = 'v'; fn f<'a>() {}\nlet d = \"v0.3.4b v0.3.40 v0.3.4.1 xv0.3.4 dev:v0.3.4:x\";\n"
        );
        let script = "// v0.3.4\nexpect(`at ${\"v0.3.4\"}`).toBe('v0.3.12');\n";
        assert_eq!(
            rewrite_text(script, Syntax::Script, &map),
            "// v0.3.4\nexpect(`at ${\"v0.2.7b\"}`).toBe('v0.3.4');\n"
        );
        assert_eq!(
            rewrite_text(
                "{\"patch\":\"v0.3.12\",\"id\":\"seed-1-v0.3.4.json\"}",
                Syntax::Data,
                &map
            ),
            "{\"patch\":\"v0.3.4\",\"id\":\"seed-1-v0.2.7b.json\"}"
        );
        assert_eq!(
            rewrite_text("# v0.3.4\nx = 'v0.3.4'\n", Syntax::Python, &map),
            "# v0.3.4\nx = 'v0.2.7b'\n"
        );
    }

    #[test]
    fn r1371000_the_write_renames_everything_once_and_a_second_run_changes_nothing() {
        let dir = fixture();
        let root = &dir.0;
        let map = compute_map(root, &root.join(PATCHES_REL)).expect("a map");
        write_retcon(root, &map).expect("the write");
        let patches = root.join(PATCHES_REL);
        for new in ["v0.1.0", "v0.1.0b", "v0.2.0", "v0.2.0b"] {
            assert!(patches.join(format!("{new}.json")).exists(), "{new}'s snapshot");
        }
        assert!(!patches.join("v0.1.3.json").exists());
        let list = read_patches(&patches.join("patches.json")).expect("the list");
        assert_eq!(list[2].version, "v0.2.0");
        assert_eq!(list[2].tier, Some(Tier::Major));
        assert_eq!(list[3].plays.as_deref(), Some(&["core-001".to_string()][..]));
        let shipped = read_shipped(&patches.join("shipped.json")).expect("shipped");
        assert_eq!(shipped[1].version, "v0.1.0b");
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).expect("the source"),
            "// Since v0.1.1.\nconst A: &str = \"v0.1.0b\";\nconst B: &str = \"dev:v0.2.0:learned-v0.2.0b-1:4\";\n"
        );
        assert!(
            root.join("src/seed-1-v0.1.0b.json").exists(),
            "a file named after a version is renamed"
        );
        assert_eq!(
            fs::read_to_string(root.join("crates/server/.env.example")).expect("the site"),
            "CATALOG_VERSION=v0.2.0b\n"
        );
        assert_eq!(
            fs::read_to_string(root.join(MIGRATIONS_REL).join("0001_start.sql")).expect("the migration"),
            "-- v0.1.1\n",
            "an applied migration is history"
        );
        let sql = fs::read_to_string(root.join(MIGRATIONS_REL).join("0002_version_retcon.sql"))
            .expect("the migration");
        assert!(sql.contains("('v0.1.2', 'v0.2.0')"));
        assert!(!sql.contains("('v0.1.0', 'v0.1.0')"), "a name kept is no rename");
        let aliases = read_aliases(root).expect("the map");
        assert_eq!(aliases.versions, VERSIONS_SCHEME);
        assert_eq!(aliases.aliases.get("v0.1.3").map(String::as_str), Some("v0.2.0b"));

        let snapshot = |root: &Path| {
            let mut files: Vec<(PathBuf, String)> = Vec::new();
            let mut pending = vec![root.to_path_buf()];
            while let Some(next) = pending.pop() {
                for entry in fs::read_dir(&next).expect("a folder") {
                    let path = entry.expect("an entry").path();
                    if path.ends_with(".git") {
                        continue;
                    }
                    if path.is_dir() {
                        pending.push(path);
                    } else {
                        files.push((path.clone(), fs::read_to_string(&path).unwrap_or_default()));
                    }
                }
            }
            files.sort();
            files
        };
        let before = snapshot(root);
        run(root, &[]).expect("a second run");
        assert_eq!(snapshot(root), before, "a second run changes nothing");
    }

    #[test]
    fn r1371000_a_record_is_marked_once() {
        let map: IndexMap<String, String> = [("v0.3.4", "v0.2.7b"), ("v0.3.12", "v0.3.4")]
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .into();
        let dir = TempDir::new();
        let out = dir.0.join("training-out");
        fs::create_dir_all(&out).expect("a folder");
        fs::write(
            out.join("games-v0.3.12.jsonl"),
            "{\"id\":\"dev:v0.3.12:gather:1\",\"patch\":\"v0.3.12\"}\n{\"id\":\"dev:v0.3.4:gather:2\",\"patch\":\"v0.3.4\"}\n",
        )
        .expect("a record file");
        fs::write(out.join("cursor-v0.3.4"), "7\n").expect("a cursor");
        let aliases = Aliases {
            versions: VERSIONS_SCHEME,
            aliases: map,
            marked: IndexMap::new(),
        };
        let report = rewrite_records(&out, &aliases, false).expect("the records");
        assert_eq!(
            report,
            RecordsReport {
                marked: 1,
                renamed: 2,
                skipped: 0
            }
        );
        let text = fs::read_to_string(out.join("games-v0.3.4.jsonl")).expect("renamed");
        assert_eq!(
            text,
            format!(
                "{{\"id\":\"dev:v0.3.4:gather:1\",\"patch\":\"v0.3.4\",\"versions\":{VERSIONS_SCHEME}}}\n{{\"id\":\"dev:v0.2.7b:gather:2\",\"patch\":\"v0.2.7b\",\"versions\":{VERSIONS_SCHEME}}}\n"
            )
        );
        assert!(out.join("cursor-v0.2.7b").exists());
        let again = rewrite_records(&out, &aliases, false).expect("the records again");
        assert_eq!(again, RecordsReport::default());
        assert!(
            out.join("cursor-v0.2.7b").exists(),
            "a name is never renamed twice"
        );
        assert_eq!(
            fs::read_to_string(out.join("games-v0.3.4.jsonl")).expect("kept"),
            text,
            "a marked file is never mapped twice"
        );
    }

    /// A tree the first retcon left (scheme 2): v0.5.0 a major, v0.5.1 a patch by a rule alone (one
    /// family), and v0.5.1b a micro, each with its tier and plays recorded.
    fn scheme_two() -> TempDir {
        let dir = TempDir::new();
        let root = &dir.0;
        let patches = root.join(PATCHES_REL);
        fs::create_dir_all(&patches).expect("a folder");
        fs::create_dir_all(root.join(MIGRATIONS_REL)).expect("a folder");
        let one = card("core-001", "Core", "One", 1);
        let snapshots = [
            ("v0.4.0", vec![one.clone()], "major", vec!["core-001"]),
            (
                "v0.5.0",
                vec![one.clone(), card("stinkyscape-001", "Stinkyscape", "Two", 2)],
                "major",
                vec!["stinkyscape-001"],
            ),
            (
                "v0.5.1",
                vec![one.clone(), card("stinkyscape-001", "Stinkyscape", "Two", 2)],
                "patch",
                vec!["core-001"],
            ),
            (
                "v0.5.1b",
                vec![
                    card("core-001", "Core", "One", 2),
                    card("stinkyscape-001", "Stinkyscape", "Two", 2),
                ],
                "micro",
                vec!["core-001"],
            ),
        ];
        for (version, cards, _, _) in &snapshots {
            fs::write(patches.join(format!("{version}.json")), catalog(cards)).expect("a snapshot");
        }
        let list: Vec<_> = snapshots
            .iter()
            .map(|(version, _, tier, plays)| {
                let mut entry = patch(version);
                entry["tier"] = json!(tier);
                entry["plays"] = json!(plays);
                entry
            })
            .collect();
        fs::write(
            patches.join("patches.json"),
            serde_json::to_string_pretty(&list).expect("JSON"),
        )
        .expect("a list");
        dir
    }

    // R2532000: the scoped run keeps every version before `--from` and names the rest by today's
    // tiers: a patch by a rule alone is a micro unless `--rule` names it.
    #[test]
    fn r2532000_the_scoped_map_keeps_what_comes_before_and_retiers_the_rest() {
        let dir = scheme_two();
        let patches = dir.0.join(PATCHES_REL);
        let names = |rules: &[String]| -> Vec<(String, String, Tier)> {
            compute_scoped_map(&patches, "v0.5.0", rules)
                .expect("a map")
                .into_iter()
                .map(|renamed| (renamed.old, renamed.new, renamed.tier))
                .collect()
        };
        let row = |old: &str, new: &str, tier: Tier| (old.to_string(), new.to_string(), tier);
        assert_eq!(
            names(&[]),
            vec![
                row("v0.4.0", "v0.4.0", Tier::Major),
                row("v0.5.0", "v0.5.0", Tier::Major),
                row("v0.5.1", "v0.5.0b", Tier::Micro),
                row("v0.5.1b", "v0.5.0c", Tier::Micro),
            ]
        );
        assert_eq!(
            names(&["v0.5.1".to_string()]),
            vec![
                row("v0.4.0", "v0.4.0", Tier::Major),
                row("v0.5.0", "v0.5.0", Tier::Major),
                row("v0.5.1", "v0.5.1", Tier::Patch),
                row("v0.5.1b", "v0.5.1b", Tier::Micro),
            ]
        );
        assert!(compute_scoped_map(&patches, "v0.5.0", &["v0.4.0".to_string()]).is_err());
        assert!(compute_scoped_map(&patches, "v9.9.9", &[]).is_err());
    }

    // R2532000: scheme 1's names are mapped on to today's, and the older marked scheme's renames
    // are kept under its number; the records of either are read through them, once.
    #[test]
    fn r2532000_the_map_file_chains_its_schemes_and_records_are_read_once() {
        let held = Aliases {
            versions: 2,
            aliases: [("v0.3.15", "v0.4.0"), ("v0.3.99", "v0.5.1")]
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .into(),
            marked: IndexMap::new(),
        };
        let dir = scheme_two();
        let map = compute_scoped_map(&dir.0.join(PATCHES_REL), "v0.5.0", &[]).expect("a map");
        let aliases = scoped_aliases(&held, &map).expect("the map file");
        assert_eq!(aliases.versions, VERSIONS_SCHEME);
        assert_eq!(aliases.aliases.get("v0.3.15").map(String::as_str), Some("v0.4.0"));
        assert_eq!(
            aliases.aliases.get("v0.3.99").map(String::as_str),
            Some("v0.5.0b")
        );
        let two = aliases.renames_from(2);
        assert_eq!(two.get("v0.5.1").map(String::as_str), Some("v0.5.0b"));
        assert_eq!(two.get("v0.5.1b").map(String::as_str), Some("v0.5.0c"));
        assert!(!two.contains_key("v0.5.0"), "a name kept is no rename");
        assert!(
            scoped_aliases(&aliases, &map).is_err(),
            "a third run must compose"
        );

        let out = dir.0.join("training-out");
        fs::create_dir_all(&out).expect("a folder");
        fs::write(out.join(format!("{RECORDS_MARKER_PREFIX}2")), "2\n").expect("the old marker");
        fs::write(
            out.join("games-v0.5.1.jsonl"),
            "{\"id\":\"dev:v0.5.1:gather:1\",\"patch\":\"v0.5.1\",\"versions\":2}\n",
        )
        .expect("a record file");
        let report = rewrite_records(&out, &aliases, false).expect("the records");
        assert_eq!((report.marked, report.renamed), (1, 1));
        assert_eq!(
            fs::read_to_string(out.join("games-v0.5.0b.jsonl")).expect("renamed"),
            format!(
                "{{\"id\":\"dev:v0.5.0b:gather:1\",\"patch\":\"v0.5.0b\",\"versions\":{VERSIONS_SCHEME}}}\n"
            )
        );
        assert_eq!(
            rewrite_records(&out, &aliases, false).expect("again"),
            RecordsReport::default()
        );
        let sql = migration_sql_for(70, &map, &V05_RETCON);
        assert!(sql.contains("create temporary table retcon_2532"));
        assert!(sql.contains("('v0.5.1', 'v0.5.0b')"));
        assert!(sql.contains(&format!("'versions', {VERSIONS_SCHEME}")));
        assert!(sql.contains("update public.game_totals t set patch = '~' || m.new"));
        assert!(!sql.contains("public.seasons (id"), "v0.5 stays v0.5");
    }

    // R2532000: migration 0068 is what the generator writes from the map its own header lists, so a
    // fix to the generator reaches the file (this run rewrites it when they differ: commit it). It
    // renames a text in one pass per name, and only a renamed version's rows: a call per version ran
    // for minutes over a season of totals and timed Render's release out.
    #[test]
    fn r2532000_migration_0068_is_what_the_generator_writes() {
        let path = repo_root().join(MIGRATIONS_REL).join("0068_v0_5_retcon.sql");
        let held = fs::read_to_string(&path).expect("migration 0068");
        let map: Vec<Renamed> = held
            .lines()
            .skip_while(|line| !line.starts_with("--   old"))
            .skip(1)
            .map_while(|line| line.strip_prefix("--   "))
            .filter_map(|row| {
                let mut names = row.split_whitespace();
                Some(Renamed {
                    old: names.next()?.to_string(),
                    new: names.next()?.to_string(),
                    tier: Tier::Micro,
                    plays: Vec::new(),
                })
            })
            .collect();
        assert!(
            map.iter()
                .any(|renamed| renamed.old == "v0.5.11" && renamed.new == "v0.5.7")
        );
        let written = migration_sql_for(68, &map, &V05_RETCON);
        if written != held {
            fs::write(&path, &written).expect("rewrite migration 0068");
            panic!(
                "{} differed from what the generator writes; this run rewrote it: commit the result",
                path.display()
            );
        }
        assert!(
            !written.contains("jsonb_each(t.co)"),
            "a co-played map is renamed as one text"
        );
    }

    // The repository itself, retconned.

    #[test]
    fn r1371000_a_marked_replay_under_v0_4_0_and_an_unmarked_one_under_v0_3_15_fold_under_meditative_plus() {
        jackioh_cards::register_all();
        let marked = crate::replay::pinned_catalog(&json!({ "catalogVersion": "v0.4.0", "versions": 2 }), 2)
            .expect("carried")
            .expect("an older catalog");
        assert!(
            marked.defs().contains_key("meditativeplus-001"),
            "v0.4.0 is Meditative+'s release"
        );
        assert_eq!(marked.version(), "v0.4.0");
        let unmarked = crate::replay::pinned_catalog(&json!({ "catalogVersion": "v0.3.15" }), 1)
            .expect("carried")
            .expect("an older catalog");
        assert!(
            std::ptr::eq(marked, unmarked),
            "the old v0.3.15 is the new v0.4.0"
        );
    }

    #[test]
    fn r1371000_an_unmarked_v0_3_4_folds_under_the_old_v0_3_4_not_the_new() {
        jackioh_cards::register_all();
        let unmarked = crate::replay::pinned_catalog(&json!({ "catalogVersion": "v0.3.4" }), 1)
            .expect("carried")
            .expect("an older catalog");
        assert_eq!(unmarked.version(), "v0.2.7b");
        assert!(
            !unmarked.defs().contains_key("meditative-001"),
            "the old v0.3.4 shipped before Meditative"
        );
        let marked = crate::replay::pinned_catalog(&json!({ "catalogVersion": "v0.3.4", "versions": 2 }), 2)
            .expect("carried")
            .expect("an older catalog");
        assert!(
            marked.defs().contains_key("meditative-001"),
            "the new v0.3.4 is the old v0.3.12"
        );
    }

    #[test]
    fn r1371000_an_old_bundle_misses_since_no_new_name_reuses_its_version() {
        let shipped = jackioh_cards::shipped_versions();
        for old in ["v0.3.15", "v0.3.16", "v0.3.17", "v0.3.17b"] {
            assert!(!shipped.contains(&old), "{old} names no new version");
            let now = jackioh_cards::versions::current_version_name(old);
            assert!(shipped.contains(&now), "{old} is {now} now");
            assert_ne!(jackioh_cards::catalog_version(), old, "a bundle at {old} misses");
        }
        assert_eq!(
            jackioh_cards::versions::current_version_name("v0.3.17b"),
            "v0.4.0d"
        );
    }

    // R2532000: the old v0.5.7 is Training Grounds' release and the new v0.5.7 is the old v0.5.11,
    // so a replay marked 2 at v0.5.7 folds under the carried new v0.5.5.
    #[test]
    fn r2532000_a_replay_marked_2_folds_under_its_v0_5_name_now() {
        jackioh_cards::register_all();
        let old = crate::replay::pinned_catalog(&json!({ "catalogVersion": "v0.5.7", "versions": 2 }), 2)
            .expect("carried")
            .expect("an older catalog");
        assert_eq!(old.version(), "v0.5.5");
        assert!(old.defs().contains_key("traininggrounds-001"));
        let aliases = read_aliases(&repo_root()).expect("the map");
        assert_eq!(aliases.versions, VERSIONS_SCHEME);
        assert_eq!(
            aliases.renames_from(2).get("v0.5.11").map(String::as_str),
            Some("v0.5.7")
        );
        assert_eq!(
            aliases.renames_from(2).len(),
            17,
            "the v0.5 line's renames, and nothing before it"
        );
    }

    #[test]
    fn r1371000_the_repository_is_retconned_and_a_second_run_changes_nothing() {
        let root = repo_root();
        let aliases = read_aliases(&root).expect("the tree holds the map");
        assert_eq!(aliases.versions, VERSIONS_SCHEME);
        let list = root.join(PATCHES_REL).join("patches.json");
        let before = fs::read_to_string(&list).expect("the list");
        run(&root, &[]).expect("the run");
        assert_eq!(fs::read_to_string(&list).expect("the list"), before);
        let patches = read_patches(&list).expect("the list");
        assert!(
            patches
                .iter()
                .all(|patch| patch.tier.is_some() && patch.plays.is_some())
        );
        let news: Vec<&str> = patches.iter().map(|patch| patch.version.as_str()).collect();
        let mapped: Vec<&str> = aliases.aliases.values().map(String::as_str).collect();
        // Promotions only append (R646): the map's new names open the list, and any version past
        // them shipped after the retcon.
        assert_eq!(
            news.get(..mapped.len()),
            Some(&mapped[..]),
            "patches.json opens with the map's new names in order"
        );
    }

    #[test]
    fn r1371001_the_migration_renames_the_season_in_play() {
        let map = [
            Renamed {
                old: "v0.3.16".to_string(),
                new: "v0.4.0b".to_string(),
                tier: Tier::Micro,
                plays: Vec::new(),
            },
            Renamed {
                old: "v0.3.17".to_string(),
                new: "v0.4.0c".to_string(),
                tier: Tier::Micro,
                plays: Vec::new(),
            },
        ];
        let sql = migration_sql_for(47, &map, &FIRST_RETCON);
        assert!(
            sql.contains("insert into public.seasons (id, patch_version, started_at)\n      select 'v0.4'")
        );
        assert!(sql.contains("where id = 'v0.3'"));
        assert!(
            !migration_sql_for(
                47,
                &map[..1]
                    .iter()
                    .map(|renamed| Renamed {
                        new: "v0.3.16".to_string(),
                        ..renamed.clone()
                    })
                    .collect::<Vec<_>>(),
                &FIRST_RETCON
            )
            .contains("public.seasons (id"),
            "a season that keeps its name is left alone"
        );
    }
}
