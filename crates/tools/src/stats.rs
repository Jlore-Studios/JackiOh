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
//! Tooling, so it may read files, the clock and the console; `crates/ai` stays pure.

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

use jackioh_ai::{AI_DEV_RUN, DevRunOptions, dev_game_record};
use jackioh_engine::{CardStatsFilter, GameMode, GameRecord, PilotFilter, SourceFilter, Winner, card_stats, format_card_stats};

/// Seconds print with this many decimals.
const SECONDS_DECIMALS: usize = 1;

/// `crates/cards/patches/patches.json`, the card patch history (R388), where this checkout keeps it.
const PATCHES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/patches/patches.json");

/// R388: the newest patch is the last one `patches.json` lists; the order of versions is the list's.
fn newest_patch(list: &Value) -> Result<String> {
    let newest = list.as_array().and_then(|entries| entries.last());
    match newest.and_then(|entry| entry.get("version")).and_then(Value::as_str) {
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
}

/// `cargo jackioh stats`.
pub fn run(args: Args) -> Result<()> {
    let games = args.games.unwrap_or(AI_DEV_RUN.games as i32);
    let series = args.series.clone().unwrap_or_else(|| AI_DEV_RUN.series.to_string());
    jackioh_cards::register_all();

    let patch = match args.patch.clone() {
        Some(patch) => patch,
        None => {
            let text = std::fs::read_to_string(PATCHES_PATH).with_context(|| format!("reading {PATCHES_PATH}"))?;
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
        Some(path) => Some(std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?),
    };

    let mut records: Vec<GameRecord> = Vec::new();
    let started = Instant::now();
    for n in args.from..args.from + games {
        let game_started = Instant::now();
        let record = dev_game_record(n, &DevRunOptions { series: series.clone(), patch: patch.clone(), budget: None });
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
        eprintln!("[ai:stats] {series}:{n} {outcome} by {} in {} turns ({seconds}s)", record.game.reason, record.game.turns);
        records.push(record);
    }

    let elapsed = format!("{:.*}", SECONDS_DECIMALS, started.elapsed().as_secs_f64());
    let written = match &out {
        None => String::new(),
        Some(path) => format!(", written to {}", path.display()),
    };
    eprintln!("[ai:stats] {} of {games} games recorded in {elapsed}s{written}", records.len());
    let report = card_stats(
        &records,
        &CardStatsFilter {
            source: SourceFilter::Dev,
            mode: Some(GameMode::Random),
            patch: Some(patch),
            pilot: PilotFilter::Ai,
        },
    );
    println!("{}", format_card_stats(&report, |card| jackioh_cards::CATALOG.get(card).map(|def| def.name.clone())));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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
            Args { games: None, from: 1, patch: None, series: None, out: None }
        );
        assert_eq!(
            parse(&["--games=50", "--from=51", "--patch=v0.2.5", "--series=a", "--out=run.jsonl"]).unwrap(),
            Args {
                games: Some(50),
                from: 51,
                patch: Some("v0.2.5".to_string()),
                series: Some("a".to_string()),
                out: Some(PathBuf::from("run.jsonl")),
            }
        );
        assert_eq!(parse(&["--games", "50"]).unwrap().games, Some(50));
        assert!(parse(&["--games=0"]).unwrap_err().to_string().contains("--games must be a positive integer"));
        assert!(parse(&["--from=1.5"]).unwrap_err().to_string().contains("--from must be a positive integer"));
        assert!(parse(&["--budget=9"]).is_err());
        assert!(parse(&["50"]).is_err());
    }

    #[test]
    fn r388_the_newest_patch_is_the_last_one_patches_json_lists() {
        let list = serde_json::json!([{ "version": "v0.1.0" }, { "version": "v0.2.0" }]);
        assert_eq!(newest_patch(&list).unwrap(), "v0.2.0");
        assert!(newest_patch(&serde_json::json!([])).unwrap_err().to_string().contains("names no newest patch (R388)"));
        assert!(newest_patch(&serde_json::json!([{ "version": "" }])).is_err());
        assert!(newest_patch(&serde_json::json!({})).is_err());
        // The checkout's own history names one, the version the cards crate compiled in.
        let text = std::fs::read_to_string(PATCHES_PATH).unwrap();
        assert_eq!(newest_patch(&serde_json::from_str(&text).unwrap()).unwrap(), jackioh_cards::catalog_version());
    }
}
