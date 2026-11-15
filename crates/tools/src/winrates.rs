//! `cargo jackioh winrates --refresh <report.json>` (R3882000): the win rates of record, refreshed
//! from production's windowed card figures once per patch, so the tier pools (R3538001) and KPIs
//! (R3538007) read this patch's figures and not a placeholder.
//!
//! Its input is the report `jackioh-server stats-cards --window --json` prints off production's
//! totals (R1367000), the figures `/api/stats/cards` shows and each match's snapshot reads
//! (R3049000). A refresh is due when the report is at the newest shipped version, no pending fragment
//! carries win rates of record, no version of the report's patch window (the X a major or a patch
//! opens, R1365000) shipped any, and at least one card has `WIN_RATE_RECORD_MIN_GAMES` games. It then
//! writes a pending fragment carrying the table (`records`), each card's in-deck wins and games over
//! its own window, rows under that floor left out since they are never a record, and claiming
//! through `plays` every card whose script reads the record (`code_lines::reads_win_rate_record`).
//! It claims no catalog card, so `patches ship` names it a micro, which stays inside the X, and the
//! refresh is not due again until a major or a patch opens the next. When it is not due it says why
//! and writes nothing. `.github/workflows/winrates-refresh.yml` runs it daily and opens the pull
//! request. M #50 CN Tech and M+ #2 NA Tech still read each game's snapshot, never this table.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use jackioh_engine::config::WIN_RATE_RECORD_MIN_GAMES;
use jackioh_engine::win_rates::win_rates_from_report;
use jackioh_engine::wire::stats::WindowedReport;

use crate::card_lists;
use crate::patches::patches_io::{
    PendingFragment, PlayerNote, fragment_name, read_fragments, read_patches, records_path, write_json,
};
use crate::patches::{patch_paths, read_catalog, repo_root};

/// The refresh's title, the fragment's file name with it (one refresh is pending at a time).
const TITLE: &str = "Patch: the tier cards and KPIs read the newest win rates";

/// Refresh the win rates of record when a refresh is due (R3882000).
#[derive(clap::Args)]
pub struct Args {
    /// The windowed card report `jackioh-server stats-cards --window --json` printed.
    #[arg(long, value_name = "REPORT")]
    pub refresh: PathBuf,
}

/// What a refresh did: the fragment it wrote, or why none was due.
#[derive(Debug, PartialEq)]
pub enum Refresh {
    Written {
        fragment: String,
        rows: usize,
        plays: Vec<String>,
    },
    NotDue(String),
}

pub fn run(args: Args) -> Result<()> {
    let text = fs::read_to_string(&args.refresh)
        .with_context(|| format!("{} cannot be read", args.refresh.display()))?;
    let report: WindowedReport = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a windowed card report", args.refresh.display()))?;
    match refresh(&repo_root(), &report)? {
        Refresh::Written {
            fragment,
            rows,
            plays,
        } => println!(
            "Due: pending/{fragment} carries {rows} cards' win rates of record at {} and claims {}.",
            report.at,
            plays.join(", ")
        ),
        Refresh::NotDue(why) => println!("Not due: {why}"),
    }
    Ok(())
}

/// R3882000: the refresh over `report` in the tree at `repo_root`.
pub fn refresh(repo_root: &Path, report: &WindowedReport) -> Result<Refresh> {
    let paths = patch_paths(repo_root);
    let patches = read_patches(&paths.patches_json)?;
    let Some(newest) = patches.last() else {
        bail!("patches.json holds no shipped patch");
    };
    if report.at != newest.version {
        return Ok(Refresh::NotDue(format!(
            "the report is at {}, but the newest shipped version is {}",
            report.at, newest.version
        )));
    }
    if let Some(held) = read_fragments(&paths.pending_dir)?
        .into_iter()
        .find(|held| held.fragment.records.is_some())
    {
        return Ok(Refresh::NotDue(format!(
            "pending/{} already carries win rates of record",
            held.name
        )));
    }
    for version in &report.patch.versions {
        if records_path(version, &paths.dir)?.exists() {
            return Ok(Refresh::NotDue(format!(
                "{version} shipped win rates of record, and the patch window {} holds it",
                report.patch.versions.join(", ")
            )));
        }
    }
    let mut table = win_rates_from_report(report);
    table.cards.retain(|row| row.games >= WIN_RATE_RECORD_MIN_GAMES);
    if table.cards.is_empty() {
        return Ok(Refresh::NotDue(format!(
            "no card has {WIN_RATE_RECORD_MIN_GAMES} games in its window"
        )));
    }

    let catalog = read_catalog(&paths.catalog)?;
    let readers = card_lists::working(repo_root, &catalog)?.record_readers;
    let plays: Vec<String> = catalog
        .keys()
        .filter(|id| readers.contains(*id))
        .cloned()
        .collect();
    let names: Vec<String> = plays
        .iter()
        .map(|id| {
            catalog[id]
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or(id)
                .to_string()
        })
        .collect();
    let rows = table.cards.len();
    let fragment = PendingFragment {
        version: None,
        title: TITLE.to_string(),
        sources: format!(
            "winrates-refresh.yml: production's windowed card figures at {} (R3882000)",
            report.at
        ),
        notes: format!(
            "The win rates of record refresh from the card statistics at {}: {rows} cards with at least {WIN_RATE_RECORD_MIN_GAMES} games over their own windows ({} figures). The tier pools and KPIs read them from this version on (R3538000, R3538001); earlier versions keep theirs, and CN Tech and NA Tech still read each game's snapshot (R3049000).",
            report.at,
            table.source.as_str()
        ),
        player: PlayerNote {
            title: "Win rates refreshed".to_string(),
            changes: vec![format!(
                "{} now read the win rates of this patch's games.",
                listed(&names)
            )],
            ..PlayerNote::default()
        },
        plays: plays.clone(),
        records: Some(table),
        ..PendingFragment::default()
    };
    let name = fragment_name(TITLE);
    fs::create_dir_all(&paths.pending_dir)
        .with_context(|| format!("{} cannot be created", paths.pending_dir.display()))?;
    write_json(&paths.pending_dir.join(&name), &fragment)?;
    Ok(Refresh::Written {
        fragment: name,
        rows,
        plays,
    })
}

/// `a`, `a and b`, `a, b and c`.
fn listed(names: &[String]) -> String {
    match names {
        [] => "No card".to_string(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use serde_json::{Value, json};

    use super::*;
    use crate::patches::patches_io::{CheckFragmentsArgs, check_fragments, player_note_problems};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("jackioh-winrates-{}-{n}", std::process::id()));
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

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the folder");
        fs::write(path, text).expect("written");
    }

    /// A script whose code reads the record through a query's `tier`, and one that only names `tier`
    /// in its tests.
    const READER: &str = "pub const ID: &str = \"core-001\";\n\nfn pool() -> Value {\n    json!({ \"tier\": \"S\" })\n}\n\n#[cfg(test)]\nmod tests {}\n";
    const OTHER: &str = "pub const ID: &str = \"core-002\";\n\nfn pool() -> Value {\n    json!({ \"cost\": 1 })\n}\n\n#[cfg(test)]\nmod tests {\n    const POOL: &str = \"\\\"tier\\\"\";\n}\n";

    /// A tree that shipped v0.1.1, with one card reading the record.
    fn tree() -> TempDir {
        let temp = TempDir::new();
        let root = &temp.0;
        let catalog = json!({
            "core-001": { "id": "core-001", "name": "Reader" },
            "core-002": { "id": "core-002", "name": "Other" },
        });
        let patches = json!([{
            "version": "v0.1.1", "date": "2026-11-01", "title": "t", "source": "s", "notes": "n",
            "player": { "title": "t", "changes": ["A card changed."] },
        }]);
        write(root, "crates/cards/patches/patches.json", &patches.to_string());
        write(root, "crates/cards/patches/v0.1.1.json", &catalog.to_string());
        write(root, "crates/cards/catalog.json", &catalog.to_string());
        write(root, "crates/cards/src/scripts/core/c001_reader.rs", READER);
        write(root, "crates/cards/src/scripts/core/c002_other.rs", OTHER);
        write(
            root,
            "crates/engine/src/config.rs",
            "pub const BUSTED_CODE_LINES: usize = 100;\npub const COMPLICATED_CODE_LINES: usize = 60;\n",
        );
        temp
    }

    fn window(versions: &[&str]) -> Value {
        json!({
            "versions": versions, "liveGames": 0, "source": "provisional",
            "counted": versions, "countedLiveGames": 0,
        })
    }

    /// A windowed report at `at` over `versions`, each row `(id, wins, games)` in deck.
    fn report(at: &str, versions: &[&str], rows: &[(&str, i32, i32)]) -> WindowedReport {
        let tally = |games: i32, wins: i32| json!({ "games": games, "wins": wins, "draws": 0 });
        let cards: Vec<Value> = rows
            .iter()
            .map(|&(id, wins, games)| {
                json!({
                    "window": window(versions),
                    "stats": {
                        "card": id, "inDeck": tally(games, wins), "openingHand": tally(0, 0),
                        "goingFirst": tally(0, 0), "goingSecond": tally(0, 0), "played": tally(0, 0),
                        "drawnNotPlayed": tally(0, 0),
                    },
                    "games": games,
                    "decks": games,
                })
            })
            .collect();
        serde_json::from_value(json!({
            "at": at, "patch": window(versions), "games": 100, "decks": 200, "cards": cards,
        }))
        .expect("a windowed report")
    }

    #[test]
    fn r3882000_a_due_refresh_writes_the_record_and_claims_the_cards_that_read_it() {
        let temp = tree();
        let root = &temp.0;
        let thin = WIN_RATE_RECORD_MIN_GAMES - 1;
        let at = report(
            "v0.1.1",
            &["v0.1.1"],
            &[
                ("core-001", 30, 40),
                ("core-002", 1, thin),
                ("stinkyscapeplus-001", 12, 20),
            ],
        );
        let fragment = "the-tier-cards-and-kpis-read-the-newest-win-rates.json".to_string();
        assert_eq!(
            refresh(root, &at).unwrap(),
            Refresh::Written {
                fragment: fragment.clone(),
                rows: 2,
                plays: vec!["core-001".to_string()],
            }
        );
        let held = read_fragments(&root.join("crates/cards/patches/pending")).unwrap();
        assert_eq!(held.len(), 1);
        let written = &held[0].fragment;
        let table = written.records.as_ref().expect("the record");
        let rows: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        // A thin row is never a record (R3538000), so the refresh leaves it out.
        assert_eq!(rows, [("core-001", 30, 40), ("stinkyscapeplus-001", 12, 20)]);
        assert_eq!(table.patch, "v0.1.1");
        assert!(written.cards.is_empty(), "no catalog card: a micro");
        assert_eq!(
            written.player.changes,
            ["Reader now read the win rates of this patch's games."]
        );
        assert!(player_note_problems(&written.player).is_empty());
        let catalog = read_catalog(&root.join("crates/cards/catalog.json")).unwrap();
        assert_eq!(
            check_fragments(&CheckFragmentsArgs {
                files: &held,
                catalog: &catalog,
                newest: &catalog,
            }),
            Vec::<String>::new()
        );
    }

    #[test]
    fn r3882000_a_refresh_is_due_once_per_patch_window_at_the_newest_version() {
        let temp = tree();
        let root = &temp.0;
        let rows = [("core-001", 30, 40)];
        let not_due = |report: &WindowedReport| matches!(refresh(root, report).unwrap(), Refresh::NotDue(_));
        // A report at an older version, or with no card at the floor, writes nothing.
        assert!(not_due(&report("v0.1.0", &["v0.1.0"], &rows)));
        assert!(not_due(&report("v0.1.1", &["v0.1.1"], &[("core-001", 1, 2)])));
        assert!(!root.join("crates/cards/patches/pending").exists());
        // A refresh pending is the patch's refresh.
        let due = report("v0.1.1", &["v0.1.1"], &rows);
        assert!(!not_due(&due));
        assert!(not_due(&due));
        // Once it ships, every version of the window holds it, so the X is refreshed.
        fs::remove_dir_all(root.join("crates/cards/patches/pending")).unwrap();
        write(root, "crates/cards/patches/records/v0.1.1.json", "{}");
        assert!(not_due(&due));
        // A version past the window (a patch opened the next X) is due again.
        let patches = json!([
            { "version": "v0.1.1", "date": "2026-11-01", "title": "t", "source": "s", "notes": "n" },
            { "version": "v0.1.2", "date": "2026-11-02", "title": "t", "source": "s", "notes": "n" },
        ]);
        write(root, "crates/cards/patches/patches.json", &patches.to_string());
        assert!(!not_due(&report("v0.1.2", &["v0.1.2"], &rows)));
    }
}
