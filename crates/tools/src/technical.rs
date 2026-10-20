//! `cargo jackioh patches log <base>..<head> [--bodies <path>]` (#1369, R1369000): the technical log.
//! A change that ships no card data never bumps the catalog version (R105); it is logged instead on the
//! patch that was live when it reached players, the newest shipped X.
//!
//! Each first-parent commit of the range is classified by stage 1 of `classify` (R1365000) against its
//! first parent. A Cards commit adds nothing, its patch records it already. An App, AI or Internal commit
//! appends one entry to `crates/cards/patches/technical.json`, and a commit already there, under any
//! version, adds nothing, so running a range twice is running it once. The production promotion runs it
//! over the commits it promotes (#1370).
//!
//! `technical.json` is a list in the order entries were logged, each one object:
//!
//! ```text
//! { "version": "v0.3.15", "commit": "<sha>", "pr": 1394, "title": "<what it does>",
//!   "kind": "app" | "ai" | "internal", "player": "<one line>" }
//! ```
//!
//! - `version`: the newest shipped X when it was logged, a version of `patches.json` (opaque, R105).
//! - `commit`: the full sha, the entry's key.
//! - `pr`: the number its subject ends with (`… (#1394)`), absent for a commit with none.
//! - `title`: the subject without that number and without its convention prefix (`Architecture: `,
//!   `Patch v0.3.Y: `, …).
//! - `kind`: `classify`'s, lowercase.
//! - `player`: optional, the line under a "Player note" heading (or after `Player note:`) in the pull
//!   request's body, `--bodies`' `{ "<pr>": "<body>" }` (`gh` fills it in CI), else the commit message.
//!   A line that breaks the note rules (R724001) is left out and named on stderr. The Patch notes page
//!   shows the App and AI entries that have one, under their X; Internal entries are the record's only.
//!
//! The newest shipped X is the last entry of `patches.json` whose tier is a patch or a major. An entry
//! records its `tier` from #1366 on; one that does not is a major when it adds a card of a set no
//! earlier entry held, a patch when `PATCH_MIN_CARDS` or more families play differently by its
//! `changes` (added, removed, or a changed field outside `WORDING_FIELDS`), and a micro otherwise.
//! Nothing parses a version (R105).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::classify::{
    FileChange, Kind, PATCH_MIN_CARDS, Tier, WORDING_FIELDS, classify, family, read_diff, show, side_at,
};
use crate::patches::git;
use crate::patches::patches_io::{PlayerNote, player_note_problems};

const PATCHES_PATH: &str = "crates/cards/patches/patches.json";
const TECHNICAL_PATH: &str = "crates/cards/patches/technical.json";

/// The title prefixes of `docs/issues-and-patches.md`, Titles, that an entry's title drops. A title
/// starting `v` and a digit (`v0.6.0: …`) drops its prefix too.
const TITLE_PREFIXES: &[&str] = &["Patch", "Architecture", "CI", "Night bot"];

/// The heading or label a pull request's body names its player line with.
const PLAYER_NOTE: &str = "Player note";

/// One line of the technical log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub version: String,
    pub commit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr: Option<u64>,
    pub title: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<String>,
}

/// R1369000: a shipped entry's tier, recorded or, before #1366, read from its changes.
fn tiers(patches: &[Value]) -> Vec<Tier> {
    let mut sets_seen = BTreeSet::new();
    patches
        .iter()
        .map(|entry| {
            let changes = entry["changes"].as_array().map(Vec::as_slice).unwrap_or_default();
            let set_of = |id: &str| id.split('-').next().unwrap_or(id).to_string();
            let ids = changes.iter().filter_map(|change| change["id"].as_str());
            let new_set = changes
                .iter()
                .filter(|change| change["kind"] == "added")
                .filter_map(|change| change["id"].as_str())
                .any(|id| !sets_seen.contains(&set_of(id)));
            sets_seen.extend(ids.map(set_of));
            match entry["tier"].as_str() {
                Some("major") => return Tier::Major,
                Some("patch") => return Tier::Patch,
                Some(_) => return Tier::Micro,
                None => {}
            }
            let plays: BTreeSet<String> = changes
                .iter()
                .filter(|change| {
                    change["kind"] != "changed"
                        || change["fields"].as_array().is_none_or(|fields| {
                            fields
                                .iter()
                                .any(|field| !field.as_str().is_some_and(|f| WORDING_FIELDS.contains(&f)))
                        })
                })
                .filter_map(|change| change["id"].as_str().map(family))
                .collect();
            if new_set {
                Tier::Major
            } else if plays.len() >= PATCH_MIN_CARDS {
                Tier::Patch
            } else {
                Tier::Micro
            }
        })
        .collect()
}

/// R1369000: the newest shipped X's version.
pub(crate) fn newest_x(patches: &[Value]) -> Result<String> {
    let tiers = tiers(patches);
    let at = tiers
        .iter()
        .rposition(|tier| *tier != Tier::Micro)
        .context("patches.json holds no patch or major")?;
    patches[at]["version"]
        .as_str()
        .map(str::to_string)
        .context("a patches.json entry has no version")
}

/// A subject's pull request number and its title with neither that number nor its prefix.
fn split_subject(subject: &str) -> (Option<u64>, String) {
    let subject = subject.trim();
    let (rest, pr) = match subject.rsplit_once(" (#") {
        Some((rest, number)) => match number.strip_suffix(')').and_then(|n| n.parse().ok()) {
            Some(pr) => (rest, Some(pr)),
            None => (subject, None),
        },
        None => (subject, None),
    };
    let prefixed = TITLE_PREFIXES.iter().any(|prefix| {
        rest.strip_prefix(prefix)
            .is_some_and(|after| after.starts_with([':', ' ']))
    }) || (rest.starts_with('v') && rest[1..].starts_with(|c: char| c.is_ascii_digit()));
    let title = match rest.split_once(": ") {
        Some((_, title)) if prefixed => title,
        _ => rest,
    };
    (pr, title.trim().to_string())
}

/// The text with its HTML comments taken out.
fn without_comments(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some(open) = rest.find("<!--") {
        out.push_str(&rest[..open]);
        rest = rest[open..]
            .find("-->")
            .map_or("", |close| &rest[open + close + 3..]);
    }
    out.push_str(rest);
    out
}

/// R1369000: the player line a body gives, under a "Player note" heading (its first line that is not
/// blank, a list marker dropped) or after `Player note:` on one line.
pub(crate) fn player_line(body: &str) -> Option<String> {
    let body = without_comments(body);
    let mut lines = body.lines().map(str::trim);
    while let Some(line) = lines.next() {
        if let Some(rest) = line
            .strip_prefix(PLAYER_NOTE)
            .and_then(|rest| rest.strip_prefix(':'))
        {
            let rest = rest.trim();
            return (!rest.is_empty()).then(|| rest.to_string());
        }
        let heading = line.trim_start_matches('#');
        if heading.len() < line.len() && heading.trim() == PLAYER_NOTE {
            let next = lines.by_ref().find(|line| !line.is_empty())?;
            if next.starts_with('#') {
                return None;
            }
            let next = next
                .strip_prefix("- ")
                .or_else(|| next.strip_prefix("* "))
                .unwrap_or(next);
            return Some(next.trim().to_string()).filter(|line| !line.is_empty());
        }
    }
    None
}

/// Why a player line breaks the note rules (R724001), or [] when it keeps them.
fn line_problems(line: &str) -> Vec<String> {
    let note = PlayerNote {
        title: PLAYER_NOTE.to_string(),
        changes: vec![line.to_string()],
        ..PlayerNote::default()
    };
    player_note_problems(&note)
}

fn read_technical(path: &Path) -> Result<Vec<Entry>> {
    match fs::read_to_string(path) {
        Ok(text) => {
            serde_json::from_str(&text).with_context(|| format!("{} is not a technical log", path.display()))
        }
        Err(_) => Ok(Vec::new()),
    }
}

/// What one run logged.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Logged {
    pub added: Vec<Entry>,
    /// Card commits, which add nothing.
    pub cards: usize,
    /// Commits the log holds already.
    pub known: usize,
}

/// R1369000: logs `range`'s first-parent commits into `root`'s technical log. `bodies` maps a pull
/// request number to its body.
pub(crate) fn log(root: &Path, range: &str, bodies: &BTreeMap<String, String>) -> Result<Logged> {
    let Some((base, head)) = range.split_once("..") else {
        bail!("log takes a range, <base>..<head>");
    };
    if base.is_empty() || head.is_empty() || head.starts_with('.') {
        bail!("log takes a range, <base>..<head>");
    }
    let patches: Vec<Value> = serde_json::from_str(
        &fs::read_to_string(root.join(PATCHES_PATH)).with_context(|| format!("reading {PATCHES_PATH}"))?,
    )
    .with_context(|| format!("{PATCHES_PATH} is not a list"))?;
    let version = newest_x(&patches)?;
    let path = root.join(TECHNICAL_PATH);
    let mut entries = read_technical(&path)?;
    let mut known: HashSet<String> = entries.iter().map(|entry| entry.commit.clone()).collect();
    let commits = git(
        root,
        &[
            "rev-list",
            "--first-parent",
            "--reverse",
            &format!("{base}..{head}"),
        ],
        &[],
    )?;
    let mut logged = Logged::default();
    for commit in commits.lines().filter(|line| !line.is_empty()) {
        if known.contains(commit) {
            logged.known += 1;
            continue;
        }
        let parent = format!("{commit}^");
        let names = git(
            root,
            &["diff", "--no-renames", "--name-only", &parent, commit],
            &[],
        )?;
        let files: Vec<FileChange> = names
            .lines()
            .filter(|path| !path.is_empty())
            .map(|path| FileChange {
                path: path.to_string(),
                before: show(root, &parent, path),
                after: show(root, commit, path),
            })
            .collect();
        let diff = read_diff(&side_at(root, &parent)?, &side_at(root, commit)?, &files);
        let kind = classify(&diff, None).kind;
        if kind == Kind::Cards {
            logged.cards += 1;
            continue;
        }
        let message = git(root, &["log", "-1", "--format=%B", commit], &[])?;
        let subject = message.lines().next().unwrap_or_default();
        let (pr, title) = split_subject(subject);
        let body = pr
            .and_then(|pr| bodies.get(&pr.to_string()))
            .map_or(message.as_str(), String::as_str);
        let player = player_line(body).filter(|line| {
            let problems = line_problems(line);
            for problem in &problems {
                eprintln!("patches log: {commit}'s player line {problem}; left out");
            }
            problems.is_empty()
        });
        let entry = Entry {
            version: version.clone(),
            commit: commit.to_string(),
            pr,
            title,
            kind,
            player,
        };
        known.insert(entry.commit.clone());
        entries.push(entry.clone());
        logged.added.push(entry);
    }
    if !logged.added.is_empty() {
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&entries)?))
            .with_context(|| format!("writing {TECHNICAL_PATH}"))?;
    }
    Ok(logged)
}

/// `patches log <base>..<head> [--bodies <path>]`.
pub(crate) fn main(root: &Path, argv: &[String]) -> Result<()> {
    let usage = "usage: patches log <base>..<head> [--bodies <path>]";
    let (mut range, mut bodies_path) = (None, None::<PathBuf>);
    let mut args = argv.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bodies" => bodies_path = Some(args.next().context(usage)?.into()),
            _ if range.is_none() && !arg.starts_with("--") => range = Some(arg.as_str()),
            _ => bail!("{usage}"),
        }
    }
    let range = range.context(usage)?;
    let bodies: BTreeMap<String, String> = match &bodies_path {
        Some(path) => serde_json::from_str(
            &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("{} is not {{ \"<pr>\": \"<body>\" }}", path.display()))?,
        None => BTreeMap::new(),
    };
    let logged = log(root, range, &bodies)?;
    for entry in &logged.added {
        let pr = entry.pr.map_or(String::new(), |pr| format!(" #{pr}"));
        println!(
            "patches log: {} {:?}{pr} {}",
            entry.version, entry.kind, entry.title
        );
    }
    println!(
        "patches log: {} entr{} added, {} card commit(s) skipped, {} already logged",
        logged.added.len(),
        if logged.added.len() == 1 { "y" } else { "ies" },
        logged.cards,
        logged.known
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use serde_json::json;

    use super::*;

    /// A git repository under the system temp dir, removed when dropped.
    struct Repo(PathBuf);

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl Repo {
        fn new() -> Repo {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "jackioh-technical-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let repo = Repo(dir);
            repo.git(&["init", "-q", "-b", "main"]);
            repo.write(
                "crates/engine/src/wire/catalog_types.rs",
                "pub const SHIPPED_SETS: [SetName; 1] = [SetName::Core];\n",
            );
            repo.write("crates/cards/catalog.json", &catalog(1));
            let patches = json!([
                { "version": "v1", "changes": [{ "id": "core-001", "name": "A", "kind": "added" }] },
                { "version": "v2", "changes": [{ "id": "core-001", "name": "A", "kind": "changed", "fields": ["cost"] }] },
            ]);
            repo.write(PATCHES_PATH, &patches.to_string());
            repo.commit("root", "");
            repo.write("README.md", "The start.\n");
            repo.commit("Architecture: the start (#1)", "");
            repo
        }

        fn write(&self, path: &str, text: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn git(&self, args: &[&str]) -> String {
            let output = Command::new("git")
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(&self.0)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        }

        fn commit(&self, subject: &str, body: &str) -> String {
            self.git(&["add", "-A"]);
            self.git(&[
                "commit",
                "-q",
                "-m",
                subject,
                "-m",
                if body.is_empty() { "." } else { body },
            ]);
            self.git(&["rev-parse", "HEAD"])
        }

        fn technical(&self) -> Vec<Entry> {
            read_technical(&self.0.join(TECHNICAL_PATH)).unwrap()
        }
    }

    fn catalog(cost: i64) -> String {
        json!({ "core-001": { "id": "core-001", "name": "A", "set": "Core", "cost": cost, "base": { "text": "" } } })
            .to_string()
    }

    /// A range of an App, an AI, an Internal and a Card commit, and the base it starts from.
    fn four_commits(repo: &Repo) -> (String, [String; 4]) {
        let base = repo.git(&["rev-parse", "HEAD"]);
        repo.write("apps/web/src/menu.ts", "export const A = 1;\n");
        let app = repo.commit(
            "Patch v0.3.Y: the menu remembers its tab (#11)",
            "## Player note\n\n<!-- one line -->\n- The main menu opens on the tab you left it on.\n",
        );
        repo.write("crates/ai/src/policy.rs", "pub fn f() -> u8 { 1 }\n");
        let ai = repo.commit("Architecture: the AI weighs Taunt (#12)", "");
        repo.write("docs/notes.md", "Notes.\n");
        let internal = repo.commit("CI: a faster cache (#13)", "Player note: Nothing players see.");
        repo.write("crates/cards/catalog.json", &catalog(2));
        let cards = repo.commit("Patch v0.3.Y: A costs (2) (#14)", "");
        (base, [app, ai, internal, cards])
    }

    #[test]
    fn r1369000_app_ai_and_internal_commits_each_append_one_entry_to_the_newest_x() {
        let repo = Repo::new();
        let (base, [app, ai, internal, _]) = four_commits(&repo);
        let bodies = BTreeMap::from([(
            "12".to_string(),
            "### Player note\nThe practice AI guards its Taunts.".to_string(),
        )]);
        let logged = log(&repo.0, &format!("{base}..HEAD"), &bodies).unwrap();
        assert_eq!((logged.added.len(), logged.cards, logged.known), (3, 1, 0));
        let expected = vec![
            Entry {
                version: "v1".to_string(),
                commit: app,
                pr: Some(11),
                title: "the menu remembers its tab".to_string(),
                kind: Kind::App,
                player: Some("The main menu opens on the tab you left it on.".to_string()),
            },
            Entry {
                version: "v1".to_string(),
                commit: ai,
                pr: Some(12),
                title: "the AI weighs Taunt".to_string(),
                kind: Kind::Ai,
                player: Some("The practice AI guards its Taunts.".to_string()),
            },
            Entry {
                version: "v1".to_string(),
                commit: internal,
                pr: Some(13),
                title: "a faster cache".to_string(),
                kind: Kind::Internal,
                player: Some("Nothing players see.".to_string()),
            },
        ];
        assert_eq!(repo.technical(), expected);
    }

    #[test]
    fn r1369000_a_card_commit_appends_nothing() {
        let repo = Repo::new();
        let (_, [.., cards]) = four_commits(&repo);
        let logged = log(&repo.0, &format!("{cards}^..{cards}"), &BTreeMap::new()).unwrap();
        assert_eq!((logged.added.len(), logged.cards), (0, 1));
        assert!(!repo.0.join(TECHNICAL_PATH).exists());
    }

    #[test]
    fn r1369000_running_the_same_range_twice_adds_nothing() {
        let repo = Repo::new();
        let (base, _) = four_commits(&repo);
        let range = format!("{base}..HEAD");
        log(&repo.0, &range, &BTreeMap::new()).unwrap();
        let once = fs::read_to_string(repo.0.join(TECHNICAL_PATH)).unwrap();
        let again = log(&repo.0, &range, &BTreeMap::new()).unwrap();
        assert_eq!((again.added.len(), again.known), (0, 3));
        assert_eq!(fs::read_to_string(repo.0.join(TECHNICAL_PATH)).unwrap(), once);
        // A wider range logs only what is new.
        let wider = log(&repo.0, &format!("{base}^..HEAD"), &BTreeMap::new()).unwrap();
        assert_eq!(wider.added.len(), 1);
        assert!(log(&repo.0, "HEAD", &BTreeMap::new()).is_err());
    }

    #[test]
    fn r1369000_the_newest_x_is_the_last_patch_or_major_recorded_or_read_from_changes() {
        let change =
            |id: &str, kind: &str, fields: &[&str]| json!({ "id": id, "kind": kind, "fields": fields });
        let wording: Vec<Value> = (1..=20)
            .map(|n| change(&format!("core-{n:03}"), "changed", &["base.text"]))
            .collect();
        let nine: Vec<Value> = (1..=9)
            .map(|n| change(&format!("core-{n:03}"), "changed", &["cost"]))
            .collect();
        let ten: Vec<Value> = (1..=10)
            .map(|n| change(&format!("core-{n:03}-1"), "changed", &["cost"]))
            .collect();
        let entry = |version: &str, changes: &[Value]| json!({ "version": version, "changes": changes });
        let mut patches = vec![
            entry("a", &[change("core-001", "added", &[])]),
            entry("b", &ten),
            entry("c", &nine),
            entry("d", &wording),
        ];
        assert_eq!(
            tiers(&patches),
            [Tier::Major, Tier::Patch, Tier::Micro, Tier::Micro]
        );
        assert_eq!(newest_x(&patches).unwrap(), "b");
        // A card of a set no earlier entry held makes a major; another of a set already held does not.
        patches.push(entry(
            "e",
            &[
                change("classic-001", "added", &[]),
                change("core-200", "added", &[]),
            ],
        ));
        patches.push(entry("f", &[change("classic-002", "added", &[])]));
        assert_eq!(tiers(&patches)[4..], [Tier::Major, Tier::Micro]);
        assert_eq!(newest_x(&patches).unwrap(), "e");
        // A recorded tier wins over the fallback.
        patches.push(json!({ "version": "g", "tier": "patch", "changes": [] }));
        patches.push(json!({ "version": "h", "tier": "micro", "changes": ten }));
        assert_eq!(newest_x(&patches).unwrap(), "g");
    }

    #[test]
    fn r1369000_the_real_history_logs_onto_meditative_plus() {
        let text = fs::read_to_string(crate::patches::repo_root().join(PATCHES_PATH)).unwrap();
        let patches: Vec<Value> = serde_json::from_str(&text).unwrap();
        let tiers = tiers(&patches);
        let majors: Vec<&str> = patches
            .iter()
            .zip(&tiers)
            .filter(|(_, tier)| **tier == Tier::Major)
            .filter_map(|(entry, _)| entry["version"].as_str())
            .collect();
        assert_eq!(majors[..4], ["v0.1.0", "v0.2.0", "v0.3.5", "v0.3.15"]);
        read_technical(&crate::patches::repo_root().join(TECHNICAL_PATH)).unwrap();
    }

    #[test]
    fn r1369000_a_title_drops_its_number_and_prefix_and_a_body_gives_its_player_line() {
        for (subject, pr, title) in [
            (
                "Architecture: patch numbers (part 5 of 8): the log (#1369)",
                Some(1369),
                "patch numbers (part 5 of 8): the log",
            ),
            (
                "Patch v0.3.Y (part 2 of 3): menus: tabs (#7)",
                Some(7),
                "menus: tabs",
            ),
            ("v0.6.0: the pack roll (#1374)", Some(1374), "the pack roll"),
            ("Night bot: a quieter queue", None, "a quieter queue"),
            ("Fix: keep this prefix (#x)", None, "Fix: keep this prefix (#x)"),
        ] {
            assert_eq!(split_subject(subject), (pr, title.to_string()), "{subject}");
        }
        assert_eq!(
            player_line("Intro\n\n## Player note\n\n* Duos games load faster.\n## Next"),
            Some("Duos games load faster.".to_string())
        );
        assert_eq!(player_line("## Player note\n<!-- none -->\n## Risks\nNone"), None);
        assert_eq!(player_line("Player note:   "), None);
        assert_eq!(player_line("No note here."), None);
        assert!(line_problems("Duos games load faster.").is_empty());
        assert!(!line_problems("Fixed #88 (R656).").is_empty());
    }
}
