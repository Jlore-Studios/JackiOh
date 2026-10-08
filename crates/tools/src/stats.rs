//! `cargo jackioh stats [--games N] [--from N] [--patch v] [--series name] [--out file]` (SPEC §9.11,
//! R378; SURFACE §12): an internal AI development run of the build in this checkout. Port of
//! `packages/ai/scripts/stats.ts` (`pnpm ai:stats`).
//!
//! Plays AI-against-AI All Random games (`devGameRecord`, `crates/ai/src/dev_run.rs`), writes each
//! finished one as a game record, one JSON line per game, and prints the run's card win rates. The
//! records are tagged `source: "dev"` and filed under the patch the run tests, so once they are
//! loaded with `jackioh-server stats-import <file>`, `stats-cards --source=dev --patch=<version>`
//! reads them, to compare with what `stats-cards --patch=<version>` reads of that patch's live games,
//! and no live figure ever counts them unless asked.
//!
//! ```text
//! cargo jackioh stats                               AI_DEV_RUN.games games of the "dev" series,
//!                                                   tagged with the newest patch in patches.json
//! cargo jackioh stats --games 50 --from 51          games 51-100, so slices run in parallel
//! cargo jackioh stats --patch v0.2.5 --out v0.2.5.jsonl
//!                                                   a pre-release run of v0.2.5, kept in a file
//! cargo jackioh stats --series tuning-a             another seed series: other games
//! ```
//!
//! A game takes seconds at the browser's budget, so a full run takes a while; progress goes to
//! stderr. `--out` is resolved against the directory the command was started in, and the file is
//! written a line per game as the run goes, so a run that is stopped keeps the games it finished.
//!
//! `cargo jackioh stats report <file>…` reads record files instead (what `--out` writes, what
//! `jackioh-server stats-export` writes of the live games, what the training lanes keep) and prints
//! R377's card win rates over them with no database, with `stats-cards`' flags: the figures a
//! notebook computes over the same files (`analysis/`) are checked against these.
//!
//! Tooling, so it may read files, the clock and the console; `crates/ai` stays pure.

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

use jackioh_ai::{AI_DEV_RUN, DevRunOptions, dev_game_record};
use jackioh_engine::{
    CardStatsFilter, CardStatsReport, DEFAULT_CARD_STATS_FILTER, GAME_MODES, GameMode, GameRecord,
    PILOT_FILTERS, PilotFilter, SOURCE_FILTERS, SourceFilter, Winner, card_stats, format_card_stats,
    parse_game_record_lines,
};

/// Seconds print with this many decimals.
const SECONDS_DECIMALS: usize = 1;

/// `crates/cards/patches/patches.json`, the card patch history (R388), where this checkout keeps it.
const PATCHES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/patches/patches.json");

/// R388: the newest patch is the last one `patches.json` lists; the order of versions is the list's.
fn newest_patch(list: &Value) -> Result<String> {
    let newest = list.as_array().and_then(|entries| entries.last());
    match newest
        .and_then(|entry| entry.get("version"))
        .and_then(Value::as_str)
    {
        Some(version) if !version.is_empty() => Ok(version.to_string()),
        _ => Err(anyhow!("{PATCHES_PATH} names no newest patch (R388)")),
    }
}

/// A positive integer option, refused with TS's words otherwise (`--games` and `--from`).
fn count(name: &str, raw: &str) -> Result<i32, String> {
    match raw.parse::<i32>() {
        Ok(value) if value >= 1 => Ok(value),
        _ => Err(format!(
            "--{name} must be a positive integer (got {}).",
            serde_json::to_string(raw).unwrap_or_default()
        )),
    }
}

fn games_count(raw: &str) -> Result<i32, String> {
    count("games", raw)
}

fn from_count(raw: &str) -> Result<i32, String> {
    count("from", raw)
}

/// `cargo jackioh stats …` (TS `DevRunArgs`, read by clap where TS's `parseDevRunArgs` read
/// `--flag=value` by hand; clap takes `--games 50` and `--games=50` alike, and refuses an option it
/// does not know with exit 2).
#[derive(clap::Args, Debug, Clone, PartialEq)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
    /// How many games. Default AI_DEV_RUN.games.
    #[arg(long, value_parser = games_count)]
    pub games: Option<i32>,
    /// The first game's number, so slices of a run can go in parallel. Default 1.
    #[arg(long, value_parser = from_count, default_value_t = 1)]
    pub from: i32,
    /// The patch the run tests. Default: the newest in patches.json.
    #[arg(long, value_name = "VERSION")]
    pub patch: Option<String>,
    /// The seed series. Default AI_DEV_RUN.series ("dev").
    #[arg(long, value_name = "NAME")]
    pub series: Option<String>,
    /// Write the records here, one JSON line per game (`stats-import` reads it).
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone, PartialEq)]
pub enum Command {
    /// R377's card win rates over record files, with no database.
    Report(ReportArgs),
}

/// `cargo jackioh stats report <file>… [--source] [--mode] [--patch] [--pilot] [--card] [--json]`:
/// `jackioh-server stats-cards`' flags and defaults (R378: live games unless `--source` says
/// otherwise), over record files instead of the database.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct ReportArgs {
    /// Record files, one GameRecord per line, read in the order named.
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,
    /// Live games (the default), an AI development run's, or both: live, dev or all.
    #[arg(long, value_parser = source_filter, default_value_t = DEFAULT_CARD_STATS_FILTER.source)]
    pub source: SourceFilter,
    /// One match type: bo1, bo3 or random. Default: every mode.
    #[arg(long, value_parser = game_mode)]
    pub mode: Option<GameMode>,
    /// One patch, as patches.json names it. Default: every patch.
    #[arg(long, value_name = "VERSION")]
    pub patch: Option<String>,
    /// The seats counted: human, ai or unified (every seat).
    #[arg(long, value_parser = pilot_filter, default_value_t = DEFAULT_CARD_STATS_FILTER.pilot)]
    pub pilot: PilotFilter,
    /// One card's row only.
    #[arg(long, value_name = "ID")]
    pub card: Option<String>,
    /// The report as JSON, as `stats-cards --json` prints it.
    #[arg(long)]
    pub json: bool,
}

/// One of `allowed`, by the literal it is written as.
fn one_of<T: Copy + std::fmt::Display>(raw: &str, allowed: &[T]) -> Result<T, String> {
    if let Some(found) = allowed.iter().find(|value| value.to_string() == raw) {
        return Ok(*found);
    }
    let names: Vec<String> = allowed.iter().map(T::to_string).collect();
    Err(format!("must be one of {}", names.join(", ")))
}

fn source_filter(raw: &str) -> Result<SourceFilter, String> {
    one_of(raw, SOURCE_FILTERS)
}

fn game_mode(raw: &str) -> Result<GameMode, String> {
    one_of(raw, GAME_MODES)
}

fn pilot_filter(raw: &str) -> Result<PilotFilter, String> {
    one_of(raw, PILOT_FILTERS)
}

/// Every record of every file, in the order the files are named and the lines written. A line that
/// is not a record is an error that names its file and line.
pub fn read_records(files: &[PathBuf]) -> Result<Vec<GameRecord>> {
    let mut records = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let read = parse_game_record_lines(&text)
            .map_err(|error| anyhow!("{}: {}", file.display(), error.message))?;
        records.extend(read);
    }
    Ok(records)
}

/// R377: the report the arguments ask for, over the records of the files they name.
pub fn report(args: &ReportArgs) -> Result<CardStatsReport> {
    let records = read_records(&args.files)?;
    let mut report = card_stats(
        &records,
        &CardStatsFilter {
            source: args.source,
            mode: args.mode,
            patch: args.patch.clone(),
            pilot: args.pilot,
        },
    );
    if let Some(only) = &args.card {
        report.cards.retain(|stats| &stats.card == only);
    }
    Ok(report)
}

/// The report as `stats-cards` prints it: the table, with the catalog's names, or JSON.
pub fn render(
    report: &CardStatsReport,
    args: &ReportArgs,
    name_of: impl Fn(&str) -> Option<String>,
) -> String {
    if args.json {
        return serde_json::to_string_pretty(report).unwrap_or_default();
    }
    let table = format_card_stats(report, name_of);
    match &args.card {
        Some(card) if report.cards.is_empty() => format!("{table}\nNo deck the filter counts held {card}."),
        _ => table,
    }
}

/// `cargo jackioh stats report`.
fn run_report(args: &ReportArgs) -> Result<()> {
    jackioh_cards::register_all();
    let report = report(args)?;
    println!(
        "{}",
        render(&report, args, |card| jackioh_cards::CATALOG
            .get(card)
            .map(|def| def.name.clone()))
    );
    Ok(())
}

/// `cargo jackioh stats`.
pub fn run(args: Args) -> Result<()> {
    if let Some(Command::Report(report)) = &args.command {
        return run_report(report);
    }
    let games = args.games.unwrap_or(AI_DEV_RUN.games);
    let series = args
        .series
        .clone()
        .unwrap_or_else(|| AI_DEV_RUN.series.to_string());
    jackioh_cards::register_all();

    let patch = match args.patch.clone() {
        Some(patch) => patch,
        None => {
            let text =
                std::fs::read_to_string(PATCHES_PATH).with_context(|| format!("reading {PATCHES_PATH}"))?;
            newest_patch(&serde_json::from_str(&text).with_context(|| format!("parsing {PATCHES_PATH}"))?)?
        }
    };
    // A path the user typed means the directory they typed it in.
    let out = match &args.out {
        None => None,
        Some(path) => Some(std::env::current_dir()?.join(path)),
    };
    let mut file = match &out {
        None => None,
        Some(path) => {
            Some(std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?)
        }
    };

    let mut records: Vec<GameRecord> = Vec::new();
    let started = Instant::now();
    for n in args.from..args.from + games {
        let game_started = Instant::now();
        let record = dev_game_record(
            n,
            &DevRunOptions {
                series: series.clone(),
                patch: patch.clone(),
                budget: None,
            },
        );
        let seconds = format!("{:.*}", SECONDS_DECIMALS, game_started.elapsed().as_secs_f64());
        let Some(record) = record else {
            eprintln!("[ai:stats] {series}:{n} did not finish; no record ({seconds}s)");
            continue;
        };
        if let Some(file) = file.as_mut() {
            writeln!(file, "{}", serde_json::to_string(&record)?)?;
            file.flush()?;
        }
        let outcome = match record.game.winner {
            Winner::Draw => "draw".to_string(),
            winner => format!("{winner} won"),
        };
        eprintln!(
            "[ai:stats] {series}:{n} {outcome} by {} in {} turns ({seconds}s)",
            record.game.reason, record.game.turns
        );
        records.push(record);
    }

    let elapsed = format!("{:.*}", SECONDS_DECIMALS, started.elapsed().as_secs_f64());
    let written = match &out {
        None => String::new(),
        Some(path) => format!(", written to {}", path.display()),
    };
    eprintln!(
        "[ai:stats] {} of {games} games recorded in {elapsed}s{written}",
        records.len()
    );
    let report = card_stats(
        &records,
        &CardStatsFilter {
            source: SourceFilter::Dev,
            mode: Some(GameMode::Random),
            patch: Some(patch),
            pilot: PilotFilter::Ai,
        },
    );
    println!(
        "{}",
        format_card_stats(&report, |card| jackioh_cards::CATALOG
            .get(card)
            .map(|def| def.name.clone()))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serde_json::Value;

    /// `Args` under a parser of its own, as `main.rs` mounts it.
    #[derive(Parser, Debug)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn parse(argv: &[&str]) -> Result<Args, clap::Error> {
        Cli::try_parse_from(std::iter::once("stats").chain(argv.iter().copied())).map(|cli| cli.args)
    }

    #[test]
    fn reads_the_runs_options_and_refuses_one_it_does_not_know() {
        assert_eq!(
            parse(&[]).unwrap(),
            Args {
                games: None,
                from: 1,
                patch: None,
                series: None,
                out: None,
                command: None
            }
        );
        assert_eq!(
            parse(&[
                "--games=50",
                "--from=51",
                "--patch=v0.2.5",
                "--series=a",
                "--out=run.jsonl"
            ])
            .unwrap(),
            Args {
                games: Some(50),
                from: 51,
                patch: Some("v0.2.5".to_string()),
                series: Some("a".to_string()),
                out: Some(PathBuf::from("run.jsonl")),
                command: None,
            }
        );
        assert_eq!(parse(&["--games", "50"]).unwrap().games, Some(50));
        assert!(
            parse(&["--games=0"])
                .unwrap_err()
                .to_string()
                .contains("--games must be a positive integer")
        );
        assert!(
            parse(&["--from=1.5"])
                .unwrap_err()
                .to_string()
                .contains("--from must be a positive integer")
        );
        assert!(parse(&["--budget=9"]).is_err());
        assert!(parse(&["50"]).is_err());
    }

    fn report_args(argv: &[&str]) -> Result<ReportArgs, clap::Error> {
        let cli = Cli::try_parse_from(["stats", "report"].into_iter().chain(argv.iter().copied()))?;
        match cli.args.command {
            Some(Command::Report(report)) => Ok(report),
            None => panic!("`stats report` parsed as a run"),
        }
    }

    #[test]
    fn report_reads_stats_cards_flags_with_its_defaults_and_refuses_what_it_does_not_know() {
        let none = report_args(&["a.jsonl"]).unwrap();
        assert_eq!(
            none,
            ReportArgs {
                files: vec![PathBuf::from("a.jsonl")],
                source: SourceFilter::Live,
                mode: None,
                patch: None,
                pilot: PilotFilter::Unified,
                card: None,
                json: false,
            }
        );
        let all = report_args(&[
            "a.jsonl",
            "b.jsonl",
            "--source=all",
            "--mode=random",
            "--patch=v0.2.5",
            "--pilot",
            "ai",
            "--card=core-001",
            "--json",
        ])
        .unwrap();
        assert_eq!(all.files, [PathBuf::from("a.jsonl"), PathBuf::from("b.jsonl")]);
        assert_eq!(
            (all.source, all.mode, all.pilot),
            (SourceFilter::All, Some(GameMode::Random), PilotFilter::Ai)
        );
        assert_eq!(
            (all.patch.as_deref(), all.card.as_deref(), all.json),
            (Some("v0.2.5"), Some("core-001"), true)
        );

        // A report names files, and a run's options are not a report's.
        assert!(report_args(&[]).is_err());
        assert!(report_args(&["a.jsonl", "--games=3"]).is_err());
        let refused = report_args(&["a.jsonl", "--source=practice"])
            .unwrap_err()
            .to_string();
        assert!(refused.contains("must be one of live, dev, all"), "{refused}");
        assert!(report_args(&["a.jsonl", "--mode=bo5"]).is_err());
        assert!(report_args(&["a.jsonl", "--pilot=robot"]).is_err());
        assert!(parse(&["--games=3", "report", "a.jsonl"]).is_err());
        assert_eq!(parse(&["--games=3"]).unwrap().command, None);
    }

    /// A game record as a line of JSON: the shape `--out` writes.
    fn record_line(id: &str, source: &str, mode: &str, winner: &str, deck_p2: &[&str]) -> String {
        serde_json::json!({
            "id": id,
            "source": source,
            "mode": mode,
            "patch": "v0.2.5",
            "pilots": { "p1": "ai", "p2": "ai" },
            "game": {
                "first": "p1",
                "winner": winner,
                "reason": "hero-death",
                "turns": 9,
                "seats": {
                    "p1": { "deck": ["core-001", "core-002"], "opening": ["core-001"], "drawn": ["core-002"],
                            "played": ["core-001"], "playedTurns": [2] },
                    "p2": { "deck": deck_p2, "opening": [], "drawn": ["core-002"], "played": [] }
                }
            }
        })
        .to_string()
    }

    /// A scratch directory of its own, holding the files given as (name, text).
    fn scratch(files: &[(&str, String)]) -> (PathBuf, Vec<PathBuf>) {
        static SCRATCH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = SCRATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("jackioh-stats-report-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = files
            .iter()
            .map(|(name, text)| {
                let path = dir.join(name);
                std::fs::write(&path, text).unwrap();
                path
            })
            .collect();
        (dir, paths)
    }

    fn report_of(files: &[PathBuf], flags: &[&str]) -> (ReportArgs, CardStatsReport) {
        let named: Vec<&str> = files.iter().map(|path| path.to_str().unwrap()).collect();
        let args = report_args(&[named, flags.to_vec()].concat()).unwrap();
        let report = report(&args).unwrap();
        (args, report)
    }

    #[test]
    fn report_over_files_is_card_stats_over_their_records_in_the_order_named() {
        let live = record_line("m1", "live", "bo1", "p1", &["core-002", "core-003"]);
        let dev_a = record_line("dev:v0.2.5:dev:1", "dev", "random", "p2", &["core-002"]);
        let dev_b = record_line("dev:v0.2.5:dev:2", "dev", "random", "p1", &["core-003"]);
        let (dir, paths) = scratch(&[
            ("live.jsonl", format!("{live}\n")),
            // A blank line is skipped, as `stats-import` skips it.
            ("dev.jsonl", format!("{dev_a}\n\n{dev_b}\n")),
        ]);
        let records = parse_game_record_lines(&format!("{live}\n{dev_a}\n{dev_b}\n")).unwrap();

        // Live games unless asked otherwise (R378).
        let (_, by_default) = report_of(&paths, &[]);
        assert_eq!(by_default.games, 1);
        assert_eq!(by_default, card_stats(&records, &DEFAULT_CARD_STATS_FILTER));

        let (_, everything) = report_of(&paths, &["--source=all"]);
        assert_eq!(everything.games, 3);
        let all = CardStatsFilter {
            source: SourceFilter::All,
            ..DEFAULT_CARD_STATS_FILTER.clone()
        };
        assert_eq!(everything, card_stats(&records, &all));
        // The order the files are named in only reorders the records, never the figures.
        let reversed: Vec<PathBuf> = paths.iter().rev().cloned().collect();
        assert_eq!(report_of(&reversed, &["--source=all"]).1, everything);

        let (_, narrowed) = report_of(
            &paths,
            &["--source=dev", "--mode=random", "--patch=v0.2.5", "--pilot=ai"],
        );
        let filter = CardStatsFilter {
            source: SourceFilter::Dev,
            mode: Some(GameMode::Random),
            patch: Some("v0.2.5".to_string()),
            pilot: PilotFilter::Ai,
        };
        assert_eq!(narrowed, card_stats(&records, &filter));
        assert_eq!((narrowed.games, narrowed.decks), (2, 4));

        // --card keeps one row; a card no counted deck held leaves the table and says so.
        let (one_args, one) = report_of(&paths, &["--source=all", "--card=core-003"]);
        assert_eq!(
            one.cards
                .iter()
                .map(|stats| stats.card.as_str())
                .collect::<Vec<_>>(),
            ["core-003"]
        );
        let (none_args, none) = report_of(&paths, &["--card=core-099"]);
        assert!(render(&none, &none_args, |_| None).ends_with("No deck the filter counts held core-099."));
        assert!(!render(&one, &one_args, |_| None).contains("No deck"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn report_json_is_the_report_as_stats_cards_prints_it() {
        let (dir, paths) = scratch(&[(
            "a.jsonl",
            record_line("m1", "live", "bo1", "p1", &["core-002"]) + "\n",
        )]);
        let (args, report) = report_of(&paths, &["--json"]);
        let printed = render(&report, &args, |_| Some("never shown".to_string()));
        let parsed: Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(parsed, serde_json::to_value(&report).unwrap());
        assert_eq!(
            parsed["filter"],
            serde_json::json!({ "source": "live", "mode": null, "patch": null, "pilot": "unified" })
        );
        assert_eq!((&parsed["games"], &parsed["decks"]), (&1.into(), &2.into()));
        // core-001 is in p1's deck only: it is in the opening hand and played, a win going first.
        let first = &parsed["cards"][0];
        assert_eq!(first["card"], "core-001");
        assert_eq!(
            first["inDeck"],
            serde_json::json!({ "games": 1, "wins": 1, "draws": 0 })
        );
        assert_eq!(first["goingFirst"], first["played"]);
        assert_eq!(
            first["drawnNotPlayed"],
            serde_json::json!({ "games": 0, "wins": 0, "draws": 0 })
        );
        // The table is the report's, with the names the caller gives.
        let table = render(&report, &ReportArgs { json: false, ..args }, |card| {
            (card == "core-001").then(|| "Alpha".to_string())
        });
        assert!(table.starts_with(
            "Card win rates: live games, every mode, every patch, human and AI pilots (unified)."
        ));
        assert!(table.contains("core-001 Alpha"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn report_names_the_file_and_line_it_cannot_read() {
        let good = record_line("m1", "live", "bo1", "p1", &["core-002"]);
        let (dir, paths) = scratch(&[
            ("good.jsonl", format!("{good}\n")),
            ("bad.jsonl", format!("{good}\n{{\"id\":\"x\"}}\n")),
            ("garbled.jsonl", "not json\n".to_string()),
        ]);
        let named = |path: &PathBuf| path.display().to_string();
        let args = |files: &[PathBuf]| ReportArgs {
            files: files.to_vec(),
            ..report_args(&["x"]).unwrap()
        };

        let bad = report(&args(&[paths[0].clone(), paths[1].clone()]))
            .unwrap_err()
            .to_string();
        assert!(
            bad.starts_with(&format!("{}: line 2: ", named(&paths[1]))),
            "{bad}"
        );
        let garbled = report(&args(&paths[2..])).unwrap_err().to_string();
        assert!(
            garbled.starts_with(&format!("{}: line 1: ", named(&paths[2]))),
            "{garbled}"
        );
        let missing = dir.join("missing.jsonl");
        let absent = format!("{:#}", report(&args(std::slice::from_ref(&missing))).unwrap_err());
        assert!(
            absent.contains(&format!("reading {}", named(&missing))),
            "{absent}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn r388_the_newest_patch_is_the_last_one_patches_json_lists() {
        let list = serde_json::json!([{ "version": "v0.1.0" }, { "version": "v0.2.0" }]);
        assert_eq!(newest_patch(&list).unwrap(), "v0.2.0");
        assert!(
            newest_patch(&serde_json::json!([]))
                .unwrap_err()
                .to_string()
                .contains("names no newest patch (R388)")
        );
        assert!(newest_patch(&serde_json::json!([{ "version": "" }])).is_err());
        assert!(newest_patch(&serde_json::json!({})).is_err());
        // The checkout's own history names one, the version the cards crate compiled in.
        let text = std::fs::read_to_string(PATCHES_PATH).unwrap();
        assert_eq!(
            newest_patch(&serde_json::from_str(&text).unwrap()).unwrap(),
            jackioh_cards::catalog_version()
        );
    }
}
