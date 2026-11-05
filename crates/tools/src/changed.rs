//! The cards a pull request changes, for `cargo jackioh fuzz`'s gate (#3113): the ids the gate deals
//! into both decks of a forced wave (#2387's `--cards`) and whose fused pairs it plays (#2381).
//!
//! `classify` names "the cards that play differently" (R1365000), but it needs the base's build and the
//! base's history, and the `rust (checks, fuzz 200)` job has a one-commit checkout and only the head's
//! binary. A card of a set that has not shipped is no change to it either (R1420), and a new card of such
//! a set is exactly what the gate should deal. So this reads the change itself, from two places:
//!
//!   1. The diff of the pull request, when `HEAD` is its merge commit (`rust (classify)` takes the base
//!      the same way, #3071): a card script whose code changed, as `catalog loc` counts it, and a
//!      catalog entry added or changed in a field that plays (`WORDING_FIELDS` is no change, a removed
//!      card has nothing to deal). Held sets count: no `ships` filter.
//!   2. The pending fragments' claims, `cards` and `plays` (R1366000): `classify --claims` (R1370000)
//!      holds those to what `classify` names, and `plays` carries the cards an engine change moves. With
//!      a base, a fragment it already held unchanged is an earlier pull request's, not this one's.
//!
//! Together that is a superset of what `classify` names. Nothing here fails the gate: a base the clone
//! lacks and cannot fetch is a warning, and the fragments' cards are played alone.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::claims::{PENDING_DIR, fragments_at};
use crate::classify::{CATALOG_PATH, WORDING_FIELDS, code_changed, script_id, show};
use crate::patches::git;
use crate::patches::patches_io::{Catalog, ChangeKind, NamedFragment, PendingFragment, diff_catalogs};

/// Where the card scripts live, whose diff names a card.
const SCRIPTS_DIR: &str = "crates/cards/src/scripts";

/// The characters of a commit's name a note shows.
const SHORT_SHA: usize = 7;

/// What a pull request changed, as the gate reads it.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Changed {
    /// The changed catalog ids, sorted, none twice. Some may be no card a deck can hold (a token, a
    /// card the change removed): `Forcing::from_changed` leaves those out.
    pub ids: Vec<String>,
    /// Where they came from, one line each.
    pub notes: Vec<String>,
    /// Set when the base could not be read, so the diff's cards are missing.
    pub warning: Option<String>,
}

/// The cards the change at `root`'s `HEAD` touches: the fragments' claims, and with a base to read the
/// diff against, the scripts and catalog entries that moved. Where the base is known, a fragment it
/// already held and the pull request left alone is an earlier pull request's claim, which stands until
/// `patches ship` (R1370000), and is not this change's.
pub(crate) fn changed_cards(root: &Path) -> Changed {
    let mut fragments = fragments_in(root);
    let mut changed = Changed::default();
    let mut ids = BTreeSet::new();
    if let Some(base) = merge_base(root) {
        let short = &base[..base.len().min(SHORT_SHA)];
        match ensure_commit(root, &base).and_then(|()| since_base(root, &base)) {
            Ok(diffed) => {
                changed
                    .notes
                    .push(format!("the base {short}: {} cards changed", diffed.len()));
                ids.extend(diffed);
                if let Ok(held) = fragments_at(root, &base) {
                    fragments.retain(|fragment| !held.contains(fragment));
                }
            }
            Err(error) => {
                changed.warning = Some(format!(
                    "could not read the base {base}: {error:#}; only the pending patches' cards are played"
                ));
            }
        }
    }
    let claimed: BTreeSet<String> = fragments
        .into_iter()
        .flat_map(|named| named.fragment.cards.into_iter().chain(named.fragment.plays))
        .collect();
    if !claimed.is_empty() {
        changed
            .notes
            .push(format!("the pending patches claim {}", claimed.len()));
    }
    ids.extend(claimed);
    changed.ids = ids.into_iter().collect();
    changed
}

/// The pending fragments in the work tree, by file name. A file that is no fragment is passed over:
/// `patches check` names it.
fn fragments_in(root: &Path) -> Vec<NamedFragment> {
    let Ok(listing) = fs::read_dir(root.join(PENDING_DIR)) else {
        return Vec::new();
    };
    let mut fragments: Vec<NamedFragment> = listing
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".json") || name.starts_with('.') {
                return None;
            }
            let text = fs::read_to_string(entry.path()).ok()?;
            let fragment = serde_json::from_str::<PendingFragment>(&text).ok()?;
            Some(NamedFragment { name, fragment })
        })
        .collect();
    fragments.sort_by(|a, b| a.name.cmp(&b.name));
    fragments
}

/// The base of a pull request's merge commit: `HEAD`'s first parent, when it has exactly two. A push to
/// `main`, a merge queue's commit or a branch of one's own has none, and so reads the fragments alone.
/// `cat-file -p` names the parents even where the clone is shallow and does not hold them.
fn merge_base(root: &Path) -> Option<String> {
    let commit = git(root, &["cat-file", "-p", "HEAD"], &[]).ok()?;
    let parents: Vec<&str> = commit
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix("parent "))
        .collect();
    match parents.as_slice() {
        [first, _second] => Some((*first).to_string()),
        _ => None,
    }
}

/// Makes `sha` readable: a shallow checkout holds one commit, so the base is fetched, itself alone.
fn ensure_commit(root: &Path, sha: &str) -> anyhow::Result<()> {
    if git(root, &["cat-file", "-e", &format!("{sha}^{{commit}}")], &[]).is_ok() {
        return Ok(());
    }
    git(
        root,
        &["fetch", "--no-tags", "--depth=1", "origin", sha],
        &[("GIT_TERMINAL_PROMPT", "0")],
    )
    .map(|_| ())
}

/// The card ids whose script's code or whose catalog entry changed between `base` and `HEAD`.
fn since_base(root: &Path, base: &str) -> anyhow::Result<BTreeSet<String>> {
    let listing = git(
        root,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            base,
            "HEAD",
            "--",
            SCRIPTS_DIR,
            CATALOG_PATH,
        ],
        &[],
    )?;
    let mut ids = BTreeSet::new();
    for path in listing.lines() {
        let (before, after) = (show(root, base, path), show(root, "HEAD", path));
        if path == CATALOG_PATH {
            ids.extend(catalog_changes(before.as_deref(), after.as_deref())?);
        } else if path.ends_with(".rs")
            && code_changed(path, before.as_deref(), after.as_deref())
            && let Some(id) = after.as_deref().and_then(script_id)
        {
            ids.insert(id);
        }
    }
    Ok(ids)
}

/// The catalog entries `after` adds or changes in a field that plays, as `classify`'s stage 1 reads
/// them (R1365000): a removed entry has nothing to deal, and a wording field plays no differently.
fn catalog_changes(before: Option<&str>, after: Option<&str>) -> anyhow::Result<BTreeSet<String>> {
    let Some(after) = after else {
        return Ok(BTreeSet::new());
    };
    let parse = |text: &str| serde_json::from_str::<Catalog>(text);
    let before = before.map(parse).transpose()?;
    let after = parse(after)?;
    Ok(diff_catalogs(before.as_ref(), &after)
        .into_iter()
        .filter(|change| change.kind != ChangeKind::Removed)
        .filter(|change| match &change.fields {
            Some(fields) => fields
                .iter()
                .any(|field| !WORDING_FIELDS.contains(&field.as_str())),
            None => true,
        })
        .map(|change| change.id)
        .collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    const CATALOG: &str = "crates/cards/catalog.json";
    const SCRIPT: &str = "crates/cards/src/scripts/core/c001.rs";
    const ZEROS: &str = "0000000000000000000000000000000000000000";

    /// A directory under the system temp dir, removed when dropped.
    struct TempRepo(PathBuf);

    impl TempRepo {
        fn new() -> TempRepo {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("jackioh-changed-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("a temp dir");
            let repo = TempRepo(path);
            repo.git(&["init", "-q", "-b", "main"]);
            repo
        }

        fn path(&self) -> &Path {
            &self.0
        }

        /// `git args` in the repo, as a test author with no config of their own.
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
                .expect("git runs");
            assert!(
                output.status.success(),
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).expect("git prints text")
        }

        fn write(&self, rel: &str, text: &str) {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
            fs::write(path, text).expect("a fixture file");
        }

        /// Commits everything as it stands and names the commit.
        fn commit(&self, message: &str) -> String {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "-m", message]);
            self.git(&["rev-parse", "HEAD"]).trim().to_string()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// One card's catalog entry, fields in the order a catalog writes them.
    fn entry(id: &str, cost: i32, flavor: &str) -> String {
        format!(r#""{id}": {{ "id": "{id}", "name": "Card {id}", "cost": {cost}, "flavor": "{flavor}" }}"#)
    }

    fn catalog(entries: &[String]) -> String {
        format!("{{\n{}\n}}\n", entries.join(",\n"))
    }

    fn script(id: &str, body: &str) -> String {
        format!(
            "use jackioh_engine::prelude::*;\n\npub const ID: &str = \"{id}\";\n\npub fn script() {{ {body} }}\n"
        )
    }

    /// A repo whose one commit holds Core #1's script and a catalog of it and one more card.
    fn base_repo() -> (TempRepo, String) {
        let repo = TempRepo::new();
        repo.write(
            CATALOG,
            &catalog(&[entry("core-001", 1, "a"), entry("core-002", 2, "b")]),
        );
        repo.write(SCRIPT, &script("core-001", "one()"));
        let base = repo.commit("base");
        (repo, base)
    }

    /// The ids `since_base` names for the work tree's next commit.
    fn named_after(repo: &TempRepo, base: &str) -> BTreeSet<String> {
        repo.commit("change");
        since_base(repo.path(), base).expect("the diff reads")
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    /// A pending fragment's text claiming `cards` and `plays`, each a comma-separated list of quoted
    /// ids, or "".
    fn fragment(cards: &str, plays: &str) -> String {
        format!(r#"{{ "title": "t", "sources": "s", "notes": "n", "cards": [{cards}], "plays": [{plays}] }}"#)
    }

    fn pending(name: &str) -> String {
        format!("{PENDING_DIR}/{name}")
    }

    #[test]
    fn the_first_parent_of_a_two_parent_checkout_is_its_base_and_any_other_commit_has_none() {
        let (repo, base) = base_repo();
        assert_eq!(merge_base(repo.path()), None, "a root commit has no base");
        repo.git(&["checkout", "-q", "-b", "topic"]);
        repo.write("topic.txt", "topic");
        repo.commit("topic");
        assert_eq!(
            merge_base(repo.path()),
            None,
            "a commit with one parent is no pull request's merge"
        );
        repo.git(&["checkout", "-q", "main"]);
        repo.write("main.txt", "main");
        let main = repo.commit("main");
        repo.git(&["merge", "-q", "--no-ff", "-m", "merge", "topic"]);
        assert_eq!(
            merge_base(repo.path()).as_deref(),
            Some(main.as_str()),
            "the first parent of the merge, not the root {base}"
        );
    }

    #[test]
    fn a_merge_commit_names_the_cards_its_second_parent_changed_against_its_first() {
        let (repo, _base) = base_repo();
        repo.write(&pending("earlier.json"), &fragment(r#""core-002""#, ""));
        repo.commit("an earlier pull request's claim, merged and not yet shipped");
        repo.git(&["checkout", "-q", "-b", "topic"]);
        repo.write(SCRIPT, &script("core-001", "two()"));
        repo.write(&pending("new.json"), &fragment("", r#""held-003""#));
        repo.commit("topic");
        repo.git(&["checkout", "-q", "main"]);
        repo.write(
            CATALOG,
            &catalog(&[entry("core-001", 1, "a"), entry("core-002", 3, "b")]),
        );
        let main = repo.commit("main moves on");
        repo.git(&["merge", "-q", "--no-ff", "-m", "merge", "topic"]);
        let changed = changed_cards(repo.path());
        assert_eq!(
            changed.ids,
            ["core-001", "held-003"],
            "the script and this pull request's own claim, not main's change or an earlier claim"
        );
        assert_eq!(
            changed.notes,
            [
                format!("the base {}: 1 cards changed", &main[..SHORT_SHA]),
                "the pending patches claim 1".to_string()
            ]
        );
        assert_eq!(changed.warning, None);
    }

    #[test]
    fn a_changed_script_or_catalog_entry_names_its_card_and_a_reflow_or_a_flavor_names_none() {
        let (repo, base) = base_repo();
        repo.write(SCRIPT, &script("core-001", "two()"));
        assert_eq!(named_after(&repo, &base), set(&["core-001"]), "a code edit");

        let (repo, base) = base_repo();
        repo.write(
            SCRIPT,
            "use jackioh_engine::prelude::*;\nuse std::fmt;\n\npub const ID: &str = \"core-001\";\n\npub fn script() {\n    one()\n}\n",
        );
        assert_eq!(
            named_after(&repo, &base),
            set(&[]),
            "a reflow and a `use` line are no code"
        );

        let (repo, base) = base_repo();
        repo.write(
            CATALOG,
            &catalog(&[entry("core-001", 1, "a"), entry("core-002", 3, "b")]),
        );
        assert_eq!(named_after(&repo, &base), set(&["core-002"]), "a cost change");

        let (repo, base) = base_repo();
        repo.write(
            CATALOG,
            &catalog(&[entry("core-001", 1, "new"), entry("core-002", 2, "b")]),
        );
        assert_eq!(named_after(&repo, &base), set(&[]), "a flavor-only edit");

        let (repo, base) = base_repo();
        repo.write(
            CATALOG,
            &catalog(&[
                entry("core-001", 1, "a"),
                entry("core-002", 2, "b"),
                entry("held-003", 3, "c"),
            ]),
        );
        assert_eq!(named_after(&repo, &base), set(&["held-003"]), "a new card");

        let (repo, base) = base_repo();
        repo.write(CATALOG, &catalog(&[entry("core-001", 1, "a")]));
        assert_eq!(named_after(&repo, &base), set(&[]), "a removed card");
    }

    #[test]
    fn the_pending_patches_claims_join_the_changed_cards_and_a_bad_fragment_is_passed_over() {
        let (repo, _base) = base_repo();
        repo.write(
            &pending("a.json"),
            &fragment(r#""core-002""#, r#""core-001", "core-002""#),
        );
        repo.write(&pending("b.json"), &fragment("", r#""held-003""#));
        repo.write(&pending("c.json"), "{ not json");
        repo.write(&pending("d.json"), r#"{ "title": "no cards" }"#);
        repo.write(&pending("notes.txt"), "ignored");
        let changed = changed_cards(repo.path());
        assert_eq!(changed.ids, ["core-001", "core-002", "held-003"]);
        assert_eq!(changed.notes, ["the pending patches claim 3"]);
        assert_eq!(changed.warning, None);

        let none = changed_cards(TempRepo::new().path());
        assert_eq!(none, Changed::default());
    }

    #[test]
    fn a_base_the_clone_lacks_and_cannot_fetch_is_reported_not_fatal() {
        let (repo, _base) = base_repo();
        assert!(ensure_commit(repo.path(), ZEROS).is_err());
        let held = repo.git(&["rev-parse", "HEAD"]);
        assert!(
            ensure_commit(repo.path(), held.trim()).is_ok(),
            "a commit held needs no fetch"
        );

        // A merge commit whose first parent is a commit the repo does not hold, planted by hand.
        let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
        let head = held.trim();
        let planted = Command::new("git")
            .args(["hash-object", "-t", "commit", "-w", "--stdin", "--literally"])
            .current_dir(repo.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                let text = format!(
                    "tree {}\nparent {ZEROS}\nparent {head}\nauthor t <t@t> 0 +0000\ncommitter t <t@t> 0 +0000\n\nplanted\n",
                    tree.trim()
                );
                child.stdin.take().expect("a stdin").write_all(text.as_bytes())?;
                child.wait_with_output()
            })
            .expect("git hash-object runs");
        let planted = String::from_utf8(planted.stdout).expect("a name");
        repo.git(&["checkout", "-q", "--detach", planted.trim()]);
        assert_eq!(merge_base(repo.path()).as_deref(), Some(ZEROS));
        let changed = changed_cards(repo.path());
        let warning = changed.warning.expect("a warning");
        assert!(warning.contains(ZEROS), "{warning}");
        assert!(warning.contains("only the pending patches' cards"), "{warning}");
        assert!(changed.ids.is_empty());
    }
}
