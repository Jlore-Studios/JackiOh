//! Admin script: the ladder bots' seed ratings, measured, to paste into config (SPEC §9.12, R748000;
//! docs/bot-calibration.md).
//!
//! ```text
//! jackioh-server bot-calibrate [--games=<n>] [--seed=<text>] [--anchor=<rating>]
//!                              [--history=<dir>] [--threads=<n>]
//! jackioh-server bot-calibrate-live [--since=<epoch ms>]
//! ```
//!
//! `bot-calibrate` plays every pair of `BOT_STRENGTHS` against each other (`bots::calibrate`), fits
//! each strength's Elo offset from the strongest, and prints the `BOT_SEED_RATINGS` and
//! `BOT_CALIBRATION_GENERATION` rows to paste into `crates/server/src/config.rs` by hand, as
//! `timing-fit` prints its rows (CLAUDE.md rule 9): no number reaches a ladder bot without a person
//! reading it first. It needs no database. Its anchor, the rating the reference is given, is
//! `--anchor` or else the seed of record shifted by the Elo of each AI promotion since
//! `BOT_CALIBRATION_GENERATION` (`training/history/*.jsonl`, run from the repository root).
//!
//! `bot-calibrate-live` reads the rated games the reference bot played against people (DATABASE_URL)
//! and prints the rating they say it performs at, to give `bot-calibrate` as `--anchor`.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use serde_json::Value;

use crate::bots::calibrate::{
    PairTally, Promotion, REFERENCE_STRENGTH, StrengthOffset, calibrate, fit_offsets, parse_vs_parent,
    performance_rating, promotion_shift, reference_observations, seed_ratings,
};
use crate::cli::card_stats::flag;
use crate::cli::{connect_one, database_url};
use crate::config::{
    BOT_CALIBRATION_GAMES_PER_PAIR, BOT_CALIBRATION_GENERATION, BOT_CALIBRATION_MIN_GAMES, BOT_SEED_RATINGS,
    BOT_STRENGTHS,
};
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server bot-calibrate [--games=<n>] [--seed=<text>] [--anchor=<rating>]
                                    [--history=<dir>] [--threads=<n>]
       jackioh-server bot-calibrate-live [--since=<epoch ms>]

  bot-calibrate plays every pair of ladder bot strengths against each other and prints the
  BOT_SEED_RATINGS and BOT_CALIBRATION_GENERATION rows to paste into crates/server/src/config.rs
  (R748000). It reads no database and takes about an hour in release.

  --games=<n>        Games each pair plays, at least 30. Default: 100.
  --seed=<text>      The seed the games are drawn from. Default: bot-calibration:gen<generation>.
  --anchor=<rating>  The reference bot's rating. Default: the seed of record, shifted by the AI
                     promotions since it was measured (read from --history).
  --history=<dir>    The training lanes' history. Default: training/history.
  --threads=<n>      Games played at once. Default: the machine's cores.

  bot-calibrate-live reads the strongest bot's rated games against people (DATABASE_URL) and prints
  the rating they say it performs at, to give bot-calibrate as --anchor.

  --since=<ms>       Only games that ended at or after this epoch ms. Default: 0.";

/// `crates/ai/generation.json`, compiled in: the AI generation this build holds.
const GENERATION_JSON: &str = include_str!("../../../ai/generation.json");

/// The AI generation this build compiles in (`crates/ai/generation.json`).
pub fn compiled_generation() -> anyhow::Result<i64> {
    let value: Value = serde_json::from_str(GENERATION_JSON)?;
    value
        .get("generation")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("crates/ai/generation.json has no integer \"generation\""))
}

/// `bot-calibrate`'s options.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrateOptions {
    pub games: usize,
    pub seed: Option<String>,
    pub anchor: Option<f64>,
    pub history: PathBuf,
    pub threads: Option<usize>,
}

fn number<T: std::str::FromStr>(name: &str, raw: &str) -> anyhow::Result<T> {
    raw.parse()
        .map_err(|_| anyhow!("--{name} must be a number (got {}).\n\n{USAGE}", quoted(raw)))
}

/// Parses `--flag=value` arguments, refusing anything it does not know and any pair with fewer
/// than `BOT_CALIBRATION_MIN_GAMES` games.
pub fn parse_calibrate_args(argv: &[String]) -> anyhow::Result<CalibrateOptions> {
    let mut options = CalibrateOptions {
        games: BOT_CALIBRATION_GAMES_PER_PAIR,
        seed: None,
        anchor: None,
        history: PathBuf::from("training/history"),
        threads: None,
    };
    for arg in argv {
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "games" => {
                options.games = number(name, raw)?;
                if options.games < BOT_CALIBRATION_MIN_GAMES {
                    bail!("--games must be at least {BOT_CALIBRATION_MIN_GAMES} (got {raw}).\n\n{USAGE}");
                }
            }
            "seed" => options.seed = Some(raw.to_owned()),
            "anchor" => {
                let anchor: f64 = number(name, raw)?;
                if !anchor.is_finite() {
                    bail!(
                        "--anchor must be a finite number (got {}).\n\n{USAGE}",
                        quoted(raw)
                    );
                }
                options.anchor = Some(anchor);
            }
            "history" => options.history = PathBuf::from(raw),
            "threads" => {
                let threads: usize = number(name, raw)?;
                if threads == 0 {
                    bail!("--threads must be at least 1.\n\n{USAGE}");
                }
                options.threads = Some(threads);
            }
            _ => bail!("Unrecognised option --{name}.\n\n{USAGE}"),
        }
    }
    Ok(options)
}

/// `bot-calibrate-live`'s `--since` (epoch ms), 0 when it is not given.
pub fn parse_live_args(argv: &[String]) -> anyhow::Result<i64> {
    let mut since = 0;
    for arg in argv {
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "since" => since = number(name, raw)?,
            _ => bail!("Unrecognised option --{name}.\n\n{USAGE}"),
        }
    }
    Ok(since)
}

/// R732001: the promotions a lane's history file holds: one JSON object a line, blank lines left
/// out. A line that is not JSON is an error; a line whose `vsParent` does not read as `wins/games`
/// (a lane's first generation has none) is not a promotion with a measured gain and is skipped.
pub fn promotions_of(text: &str) -> anyhow::Result<Vec<Promotion>> {
    let mut promotions = Vec::new();
    for (number, line) in text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| anyhow!("history line {} is not JSON: {error}", number + 1))?;
        let generation = value.get("generation").and_then(Value::as_i64);
        let versus = value
            .get("vsParent")
            .and_then(Value::as_str)
            .and_then(parse_vs_parent);
        if let (Some(generation), Some((wins, games))) = (generation, versus) {
            promotions.push(Promotion {
                generation,
                wins,
                games,
            });
        }
    }
    Ok(promotions)
}

/// Every `*.jsonl` in `dir`, by file name, as promotions.
fn read_history(dir: &Path) -> anyhow::Result<Vec<Promotion>> {
    let entries = std::fs::read_dir(dir).map_err(|error| {
        anyhow!(
            "cannot read {}: {error} (run from the repo root or pass --history)",
            dir.display()
        )
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "jsonl"))
        .collect();
    files.sort();
    let mut promotions = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)?;
        promotions.extend(promotions_of(&text).map_err(|error| anyhow!("{}: {error}", file.display()))?);
    }
    Ok(promotions)
}

/// What a calibration found.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    pub generation: i64,
    pub seed_base: String,
    pub games_per_pair: usize,
    pub anchor: f64,
    /// Where the anchor came from, in words.
    pub anchor_source: String,
    pub tallies: Vec<PairTally>,
    pub offsets: Vec<StrengthOffset>,
    pub seeds: Vec<f64>,
}

/// The report as Markdown for `docs/bot-calibration.md`, ending in the two config rows to paste.
pub fn render_calibration(report: &CalibrationReport) -> String {
    let mut out = String::from("## Ladder bot calibration (R748000)\n\n");
    out.push_str(&format!(
        "Generation {}, seed `{}`, {} games per pair, anchor {:.1} ({}).\n\n",
        report.generation, report.seed_base, report.games_per_pair, report.anchor, report.anchor_source
    ));
    out.push_str("Wins, draws and losses are the lower strength's.\n\n");
    out.push_str("| Pair | Wins | Draws | Losses | Unfinished |\n| --- | ---: | ---: | ---: | ---: |\n");
    for tally in &report.tallies {
        out.push_str(&format!(
            "| {} v {} | {} | {} | {} | {} |\n",
            tally.a, tally.b, tally.a_wins, tally.draws, tally.b_wins, tally.unfinished
        ));
    }
    out.push_str("\n| Strength | Offset | ± | Seed |\n| --- | ---: | ---: | ---: |\n");
    for (fit, seed) in report.offsets.iter().zip(&report.seeds) {
        let mark = if fit.strength == REFERENCE_STRENGTH {
            " (reference)"
        } else {
            ""
        };
        out.push_str(&format!(
            "| {}{mark} | {:+.1} | {:.1} | {seed:.1} |\n",
            fit.strength, fit.offset, fit.error
        ));
    }
    for fit in report.offsets.iter().filter(|fit| fit.offset > 0.0) {
        out.push_str(&format!(
            "\nWarning: strength {} fits {:+.1} above the reference. The reference is not the strongest, \
             or the games are too few to tell.\n",
            fit.strength, fit.offset
        ));
    }
    out.push_str(
        "\n// jackioh-server bot-calibrate: paste over the rows of these names, crates/server/src/config.rs\n",
    );
    let seeds: Vec<String> = report.seeds.iter().map(|seed| format!("{seed:.1}")).collect();
    out.push_str(&format!(
        "pub const BOT_SEED_RATINGS: [f64; {}] = [{}];\n",
        report.seeds.len(),
        seeds.join(", ")
    ));
    out.push_str(&format!(
        "pub const BOT_CALIBRATION_GENERATION: i64 = {};\n",
        report.generation
    ));
    out
}

/// `bot-calibrate`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_calibrate_args(&args)?;
    let generation = compiled_generation()?;
    let seed_base = options
        .seed
        .clone()
        .unwrap_or_else(|| format!("bot-calibration:gen{generation}"));
    let (anchor, anchor_source) = match options.anchor {
        Some(anchor) => (anchor, "from --anchor".to_owned()),
        None => {
            let promotions = read_history(&options.history)?;
            let since = promotions
                .iter()
                .filter(|promotion| {
                    (BOT_CALIBRATION_GENERATION + 1..=generation).contains(&promotion.generation)
                })
                .count();
            let shift = promotion_shift(&promotions, BOT_CALIBRATION_GENERATION, generation);
            let record = BOT_SEED_RATINGS[REFERENCE_STRENGTH];
            let source = if since == 0 {
                format!(
                    "the seed of record {record:.1}, no promotion since generation {BOT_CALIBRATION_GENERATION}"
                )
            } else {
                format!(
                    "the seed of record {record:.1} {shift:+.1} for the {since} promotion(s) of generations {} to {generation}",
                    BOT_CALIBRATION_GENERATION + 1
                )
            };
            (record + shift, source)
        }
    };
    let threads = options
        .threads
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(1);
    let games_per_pair = options.games;
    eprintln!(
        "bot-calibrate: {} games per pair on {threads} threads; this takes a while",
        games_per_pair
    );
    let base = seed_base.clone();
    let tallies = tokio::task::spawn_blocking(move || calibrate(&base, games_per_pair, threads)).await?;
    let offsets = fit_offsets(&tallies, BOT_STRENGTHS.len(), REFERENCE_STRENGTH);
    let seeds = seed_ratings(anchor, &offsets);
    let report = CalibrationReport {
        generation,
        seed_base,
        games_per_pair,
        anchor,
        anchor_source,
        tallies,
        offsets,
        seeds,
    };
    print!("{}", render_calibration(&report));
    Ok(())
}

/// `bot-calibrate-live`'s arguments after the subcommand (`main.rs`).
pub async fn run_live(args: Vec<String>) -> anyhow::Result<()> {
    let since = parse_live_args(&args)?;
    let connection_string = database_url()?;
    // One connection: this process runs one read and exits.
    let db = connect_one(&connection_string).await?;
    let read = async {
        let mut tx = db.begin(None).await?;
        let games = tx.ranked_bot_games_since(since).await?;
        let bots = tx.bots_list().await?;
        tx.commit().await?;
        Ok::<_, crate::db::store::StoreError>((games, bots))
    }
    .await;
    db.close().await;
    let (games, bots) = read?;
    let observations = reference_observations(&games, &bots, REFERENCE_STRENGTH);
    let performed =
        performance_rating(&observations).filter(|_| observations.len() >= BOT_CALIBRATION_MIN_GAMES);
    let Some((rating, error)) = performed else {
        eprintln!(
            "bot-calibrate-live: the reference holds {} rated games against people since {since}; \
             {BOT_CALIBRATION_MIN_GAMES} are needed",
            observations.len()
        );
        return Ok(());
    };
    println!(
        "The reference (strength {REFERENCE_STRENGTH}) performed at {rating:.1} ± {error:.1} over {} rated \
         games against people since {since}.",
        observations.len()
    );
    println!("Next: jackioh-server bot-calibrate --anchor={}", rating.round());
    Ok(())
}
