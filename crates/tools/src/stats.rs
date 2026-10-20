//! `cargo jackioh stats [--games N] [--from N] [--patch v] [--series name] [--force id] [--out file]`
//! (SPEC §9.11, R378; SURFACE §12): an internal AI development run of the build in this checkout. Port of
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
//! cargo jackioh stats --force core-090 --out a.jsonl
//!                                                   core-090 in one seat's deck in every game, p1's in
//!                                                   odd games and p2's in even ones (R1284000)
//! cargo jackioh stats compare --before a.jsonl --after b.jsonl --card core-090
//!                                                   two such runs of the same seeds, on two builds,
//!                                                   compared on the seat that holds the card (R1284000)
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
//! `cargo jackioh stats report --games <file>…` prints the game figures over the same files instead,
//! per patch, mode and source: how often the first seat won (with its 95% Wilson interval), how the
//! games ended, how long they lasted, and what R1256000's counts say of fatigue, Traps, position
//! switches and Radiant upgrades. A record written before those counts is unknown for them, not
//! zero. The training lanes keep these figures
//! beside the card figures (`training/README.md`).
//!
//! A forced run files under a series of its own (`dev-force-<id>`) and is for `stats compare` and
//! `stats report`, never for `stats-import` (R1284000).
//!
//! `cargo jackioh stats fit-dealt <file>…` refits `DEALT_Q` and `DEALT_Q_PRIOR` over arena record
//! files and prints them to paste (`dealt_fit.rs`).
//!
//! Tooling, so it may read files, the clock and the console; `crates/ai` stays pure.

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::Value;

use jackioh_ai::{AI_DEV_RUN, DevRunOptions, SHADOW_BAN, SHADOW_WATCH, dev_game_record};
use jackioh_engine::{
    CardStatsFilter, CardStatsReport, DEFAULT_CARD_STATS_FILTER, DEV_RECORD_ID_PREFIX, FORCED_SERIES_PREFIX,
    GAME_MODES, GAME_OVER_REASONS, GameMode, GameOverReason, GameRecord, GameSource, GameSummary,
    PILOT_FILTERS, PLAYER_IDS, PilotFilter, PlayerId, SOURCE_FILTERS, SourceFilter, Winner, card_stats,
    describe_filter, format_card_stats, parse_game_record_lines, record_matches, seats_counted, sources_of,
};

use crate::arena::{self, paired_interval, wilson_interval};
use crate::{dealt_fit, golden};

/// Seconds print with this many decimals.
const SECONDS_DECIMALS: usize = 1;

/// Percentages print with this many decimals.
const PERCENT_DECIMALS: usize = 1;

/// A mean or a per-set figure prints with this many decimals.
const MEAN_DECIMALS: usize = 2;

/// A share as a percentage.
const PERCENT: f64 = 100.0;

/// The turns are cut into this many parts by their quartiles.
const QUARTERS: usize = 4;

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
    /// The seed series. Default AI_DEV_RUN.series ("dev"). Refused with --force (R1284000).
    #[arg(long, value_name = "NAME")]
    pub series: Option<String>,
    /// R1284000: deal this card to p1 in odd games and p2 in even ones; a token goes in through its card (R279).
    #[arg(long, value_name = "ID")]
    pub force: Option<String>,
    /// Write the records here, one JSON line per game (`stats-import` reads it).
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone, PartialEq)]
pub enum Command {
    /// R377's card win rates over record files, with no database, or with --games the game figures.
    Report(ReportArgs),
    /// DEALT_Q's refit over arena record files, printed to paste into crates/ai/src/shadow_ban.rs.
    FitDealt(dealt_fit::Args),
    /// R1284000: two forced runs compared: the holder's win rate before and after, and the paired difference.
    Compare(CompareArgs),
}

/// `cargo jackioh stats compare --before <file>… --after <file>… --card <id>`: two runs of
/// `stats --force <id>` over the same seeds on two builds (R1284000).
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct CompareArgs {
    /// The run on the build before the change: record files, read in the order named.
    #[arg(long, required = true, num_args = 1.., value_name = "FILE")]
    pub before: Vec<PathBuf>,
    /// The run on the build after it.
    #[arg(long, required = true, num_args = 1.., value_name = "FILE")]
    pub after: Vec<PathBuf>,
    /// The card both runs forced, read as it is written in the records' decks (a token is not resolved).
    #[arg(long, value_name = "ID")]
    pub card: String,
}

/// `cargo jackioh stats report <file>… [--source] [--mode] [--patch] [--pilot] [--card] [--games] [--json]`:
/// `jackioh-server stats-cards`' flags and defaults (R378: live games unless `--source` says
/// otherwise; R807000: every counted seat merged unless `--pilot` names a side), over record
/// files instead of the database.
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
    /// The seats counted: human, ai or unified (every seat). Default: unified, every counted
    /// seat; human reads the people's sides only, ai the bots' (R807000).
    #[arg(long, value_parser = pilot_filter)]
    pub pilot: Option<PilotFilter>,
    /// One card's row only.
    #[arg(long, value_name = "ID")]
    pub card: Option<String>,
    /// The game figures instead of the card win rates (per patch, mode and source). Not with --card.
    #[arg(long, conflicts_with = "card")]
    pub games: bool,
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

/// R807000: the seats a report that names no `--pilot` counts, as `stats-cards` counts them:
/// every counted seat merged, whatever the source reads.
pub fn default_pilot(_source: SourceFilter) -> PilotFilter {
    PilotFilter::Unified
}

/// The filter a report's flags name, with R378's and R807000's defaults.
fn filter_of(args: &ReportArgs) -> CardStatsFilter {
    CardStatsFilter {
        source: args.source,
        mode: args.mode,
        patch: args.patch.clone(),
        pilot: args.pilot.unwrap_or_else(|| default_pilot(args.source)),
    }
}

/// R377: the report the arguments ask for, over the records of the files they name.
pub fn report(args: &ReportArgs) -> Result<CardStatsReport> {
    let records = read_records(&args.files)?;
    let mut report = card_stats(&records, &filter_of(args));
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

/// `stats report --games`: one patch's figures in one mode from one source. A game is read whole,
/// whichever of its seats `--pilot` counts.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameGroup {
    /// The patch the games were played with.
    pub patch: String,
    /// The match type.
    pub mode: GameMode,
    /// Where the games were played: live matches or an AI development run's.
    pub source: GameSource,
    /// How many games.
    pub games: usize,
    /// How many of them nobody won.
    pub draws: usize,
    /// How many the seat that took the first turn won.
    pub first_wins: usize,
    /// `first_wins` over `games`, to three places.
    pub first_share: f64,
    /// The low end of the 95% Wilson interval of `first_share`.
    pub first_low: f64,
    /// The high end of the 95% Wilson interval of `first_share`.
    pub first_high: f64,
    /// Games per ending, every `GameOverReason` in its declared order, the ones no game had at 0.
    pub reasons: IndexMap<GameOverReason, usize>,
    /// The turn counter at the end: the first, second and third quartiles, nearest rank.
    pub turn_quartiles: [i32; QUARTERS - 1],
    /// How many of the games have R1256000's counts: the figures below are over these only.
    pub counted: usize,
    /// How many of the counted games had a draw from an empty library, or null when none is counted.
    pub fatigue_games: Option<usize>,
    /// `fatigue_games` over `counted`, to three places.
    pub fatigue_share: Option<f64>,
    /// The mean of the nominal fatigue damage a counted game dealt, both seats added, to three places.
    pub fatigue_damage_mean: Option<f64>,
    /// Traps set from a hand over the counted games, both seats.
    pub traps_set: Option<i32>,
    /// Traps that fired over the counted games, both seats.
    pub traps_fired: Option<i32>,
    /// `traps_fired` over `traps_set` to three places, or null when no Trap was set.
    pub traps_fired_per_set: Option<f64>,
    /// Position switches by units over the counted games, both seats.
    pub position_switches: Option<i32>,
    /// The mean of `position_switches` over the counted games, to three places.
    pub position_switches_mean: Option<f64>,
    /// Cards an effect made Radiant over the counted games, both seats.
    pub radiant_upgrades: Option<i32>,
    /// The mean of `radiant_upgrades` over the counted games, to three places.
    pub radiant_upgrades_mean: Option<f64>,
}

/// `stats report --games`: the groups a filter reads and the records it left to another source.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameReport {
    /// What the report read.
    pub filter: CardStatsFilter,
    /// How many records came from a source the filter does not read (R378).
    pub other_sources: usize,
    /// One group per patch, mode and source, in the order a record of it was first read.
    pub groups: Vec<GameGroup>,
}

/// R1256000's counts of one game, both seats added.
#[derive(Clone, Copy, Default)]
struct Counts {
    fatigue_draws: i32,
    fatigue_damage: i32,
    traps_set: i32,
    traps_fired: i32,
    position_switches: i32,
    radiant_upgrades: i32,
}

/// R1256000: a game's counts, or `None` when either seat's record lacks one (written before them).
fn counts(game: &GameSummary) -> Option<Counts> {
    let mut sum = Counts::default();
    for seat in PLAYER_IDS {
        let seat = &game.seats[seat];
        sum.fatigue_draws += seat.fatigue_draws?;
        sum.fatigue_damage += seat.fatigue_damage?;
        sum.traps_set += seat.traps_set?;
        sum.traps_fired += seat.traps_fired?;
        sum.position_switches += seat.position_switches?;
        sum.radiant_upgrades += seat.radiant_upgrades?;
    }
    Some(sum)
}

/// One group's figures over its records, which are never none.
fn game_group(key: (String, GameMode, GameSource), records: &[&GameRecord]) -> GameGroup {
    let (patch, mode, source) = key;
    let n = records.len();
    let draws = records
        .iter()
        .filter(|record| record.game.winner == Winner::Draw)
        .count();
    let first_wins = records
        .iter()
        .filter(|record| record.game.winner.player() == Some(record.game.first))
        .count();
    let (low, high) = wilson_interval(first_wins, n);

    let mut reasons: IndexMap<GameOverReason, usize> =
        GAME_OVER_REASONS.iter().map(|reason| (*reason, 0)).collect();
    for record in records {
        *reasons.entry(record.game.reason).or_default() += 1;
    }

    let mut turns: Vec<i32> = records.iter().map(|record| record.game.turns).collect();
    turns.sort_unstable();
    let turn_quartiles = std::array::from_fn(|cut| turns[((cut + 1) * n).div_ceil(QUARTERS) - 1]);

    let known: Vec<Counts> = records.iter().filter_map(|record| counts(&record.game)).collect();
    let counted = known.len();
    let any = counted > 0;
    let fatigue_games = known.iter().filter(|counts| counts.fatigue_draws > 0).count();
    let damage: i32 = known.iter().map(|counts| counts.fatigue_damage).sum();
    let traps_set: i32 = known.iter().map(|counts| counts.traps_set).sum();
    let traps_fired: i32 = known.iter().map(|counts| counts.traps_fired).sum();
    let switches: i32 = known.iter().map(|counts| counts.position_switches).sum();
    let upgrades: i32 = known.iter().map(|counts| counts.radiant_upgrades).sum();
    GameGroup {
        patch,
        mode,
        source,
        games: n,
        draws,
        first_wins,
        first_share: arena::round_share(first_wins as f64 / n as f64),
        first_low: arena::round_share(low),
        first_high: arena::round_share(high),
        reasons,
        turn_quartiles,
        counted,
        fatigue_games: any.then_some(fatigue_games),
        fatigue_share: any.then(|| arena::round_share(fatigue_games as f64 / counted as f64)),
        fatigue_damage_mean: any.then(|| arena::round_share(f64::from(damage) / counted as f64)),
        traps_set: any.then_some(traps_set),
        traps_fired: any.then_some(traps_fired),
        traps_fired_per_set: (any && traps_set > 0)
            .then(|| arena::round_share(f64::from(traps_fired) / f64::from(traps_set))),
        position_switches: any.then_some(switches),
        position_switches_mean: any.then(|| arena::round_share(f64::from(switches) / counted as f64)),
        radiant_upgrades: any.then_some(upgrades),
        radiant_upgrades_mean: any.then(|| arena::round_share(f64::from(upgrades) / counted as f64)),
    }
}

/// The game figures over the records the filter reads: a game counts as `CardStatsReport.games`
/// counts it (the filter's source, mode and patch, and a seat its pilot reads), and a record of a
/// source the filter does not read is only tallied (R378).
pub fn game_report(records: &[GameRecord], filter: &CardStatsFilter) -> GameReport {
    let sources = sources_of(filter.source);
    let mut other_sources = 0;
    let mut grouped: IndexMap<(String, GameMode, GameSource), Vec<&GameRecord>> = IndexMap::new();
    for record in records {
        if !sources.contains(&record.source) {
            other_sources += 1;
        } else if record_matches(record, filter) && !seats_counted(record, filter.pilot).is_empty() {
            grouped
                .entry((record.patch.clone(), record.mode, record.source))
                .or_default()
                .push(record);
        }
    }
    GameReport {
        filter: filter.clone(),
        other_sources,
        groups: grouped
            .into_iter()
            .map(|(key, records)| game_group(key, &records))
            .collect(),
    }
}

/// The game report as `stats report --games` prints it: lines of text, or JSON.
pub fn render_games(report: &GameReport, json: bool) -> String {
    if json {
        return serde_json::to_string_pretty(report).unwrap_or_default();
    }
    let mut lines = vec![format!("Games: {}.", describe_filter(&report.filter))];
    if report.groups.is_empty() {
        lines.push("No game the filter reads.".to_string());
    }
    for group in &report.groups {
        let GameGroup {
            patch,
            mode,
            source,
            games,
            ..
        } = group;
        lines.push(String::new());
        lines.push(format!(
            "{patch} {mode} {source}: {games} games, {} drawn",
            group.draws
        ));
        lines.push(format!(
            "  first seat won {} of {games}: {}, 95% interval {} to {}",
            group.first_wins,
            percent(group.first_share),
            percent(group.first_low),
            percent(group.first_high)
        ));
        let endings: Vec<String> = group
            .reasons
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(reason, count)| format!("{reason} {count}"))
            .collect();
        lines.push(format!("  endings: {}", endings.join(", ")));
        let [first, second, third] = group.turn_quartiles;
        lines.push(format!("  turns, quartiles: {first}, {second}, {third}"));
        if group.counted == 0 {
            lines.push(
                "  fatigue, traps, switches and upgrades: unknown, no record of these games carries R1256000's counts"
                    .to_string(),
            );
            continue;
        }
        lines.push(format!(
            "  fatigue: a draw from an empty library in {} of {} games ({}), {:.*} damage a game",
            group.fatigue_games.unwrap_or_default(),
            group.counted,
            percent(group.fatigue_share.unwrap_or_default()),
            MEAN_DECIMALS,
            group.fatigue_damage_mean.unwrap_or_default()
        ));
        lines.push(match group.traps_fired_per_set {
            Some(per_set) => format!(
                "  traps: {} fired of {} set, {:.*} a set",
                group.traps_fired.unwrap_or_default(),
                group.traps_set.unwrap_or_default(),
                MEAN_DECIMALS,
                per_set
            ),
            None => "  traps: none set".to_string(),
        });
        lines.push(format!(
            "  position switches: {}, {:.*} a game",
            group.position_switches.unwrap_or_default(),
            MEAN_DECIMALS,
            group.position_switches_mean.unwrap_or_default()
        ));
        lines.push(format!(
            "  Radiant upgrades: {}, {:.*} a game",
            group.radiant_upgrades.unwrap_or_default(),
            MEAN_DECIMALS,
            group.radiant_upgrades_mean.unwrap_or_default()
        ));
        if group.counted < *games {
            lines.push(format!(
                "  (fatigue, traps, switches and upgrades over the {} of {games} games whose records carry R1256000's counts)",
                group.counted
            ));
        }
    }
    if report.other_sources > 0 {
        lines.push(String::new());
        lines.push(format!(
            "Left out: {} records of a source --source={} does not read (R378).",
            report.other_sources, report.filter.source
        ));
    }
    lines.join("\n")
}

/// `cargo jackioh stats report`.
fn run_report(args: &ReportArgs) -> Result<()> {
    if args.games {
        let report = game_report(&read_records(&args.files)?, &filter_of(args));
        println!("{}", render_games(&report, args.json));
        return Ok(());
    }
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

/// R1284000: whether the seat whose deck alone holds `card` won; `None` when no seat or both seats hold it.
/// A draw is a game not won.
fn holder_won(record: &GameRecord, card: &str) -> Option<bool> {
    let holders: Vec<PlayerId> = record
        .game
        .seats
        .iter()
        .filter(|(_, seat)| seat.deck.iter().any(|id| id == card))
        .map(|(player, _)| player)
        .collect();
    match holders.as_slice() {
        [holder] => Some(record.game.winner == Winner::from(*holder)),
        _ => None,
    }
}

/// R1284000: the game a development record is, the seed its id ends with, so the same game on two builds
/// (two patches) has one key.
fn game_key(record: &GameRecord) -> &str {
    record
        .id
        .strip_prefix(&format!("{DEV_RECORD_ID_PREFIX}{}:", record.patch))
        .unwrap_or(&record.id)
}

/// R1284000: the win rate of the seat that holds the card over one side's records, with its 95% Wilson
/// interval (R850000's `wilson_interval`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HolderRate {
    /// Games in which exactly one seat holds the card.
    pub games: usize,
    pub wins: usize,
    pub share: f64,
    pub low: f64,
    pub high: f64,
    /// Games no seat or both seats hold the card in, which no rate counts.
    pub left_out: usize,
}

fn holder_rate(records: &[GameRecord], card: &str) -> HolderRate {
    let results: Vec<bool> = records
        .iter()
        .filter_map(|record| holder_won(record, card))
        .collect();
    let games = results.len();
    let wins = results.iter().filter(|won| **won).count();
    let (low, high) = wilson_interval(wins, games);
    HolderRate {
        games,
        wins,
        share: if games == 0 {
            0.0
        } else {
            wins as f64 / games as f64
        },
        low,
        high,
        left_out: records.len() - games,
    }
}

/// R1284000: the holder's result in each game both sides played, after less before (−1, 0 or 1), as a mean
/// with its 95% interval (R850000's `paired_interval`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairedDifference {
    pub games: usize,
    pub mean: f64,
    pub low: f64,
    pub high: f64,
}

/// The games on both sides, by `game_key`, each side's first record of a game; `None` when no game is on both.
fn paired_difference(before: &[GameRecord], after: &[GameRecord], card: &str) -> Option<PairedDifference> {
    let mut before_won: IndexMap<&str, bool> = IndexMap::new();
    for record in before {
        if let Some(won) = holder_won(record, card) {
            before_won.entry(game_key(record)).or_insert(won);
        }
    }
    let mut seen: IndexSet<&str> = IndexSet::new();
    let mut edges: Vec<f64> = Vec::new();
    for record in after {
        let key = game_key(record);
        let (Some(after_won), Some(before_won)) = (holder_won(record, card), before_won.get(key)) else {
            continue;
        };
        if seen.insert(key) {
            edges.push(f64::from(u8::from(after_won)) - f64::from(u8::from(*before_won)));
        }
    }
    if edges.is_empty() {
        return None;
    }
    let (mean, low, high) = paired_interval(&edges);
    Some(PairedDifference {
        games: edges.len(),
        mean,
        low,
        high,
    })
}

/// R186, R390: the table that lists `card` and why, `SHADOW_BAN` before `SHADOW_WATCH`.
fn shadow_row(card: &str) -> Option<(&'static str, &'static str)> {
    let listed = |table: &'static [(&'static str, &'static str)]| {
        table
            .iter()
            .find(|(id, _)| *id == card)
            .map(|(_, reason)| *reason)
    };
    listed(SHADOW_BAN)
        .map(|reason| ("SHADOW_BAN", reason))
        .or_else(|| listed(SHADOW_WATCH).map(|reason| ("SHADOW_WATCH", reason)))
}

/// R1284000: what `stats compare` prints, as data.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub card: String,
    pub before: HolderRate,
    pub after: HolderRate,
    pub paired: Option<PairedDifference>,
    /// The table that lists the card and the reason it gives: the AI plays it badly or times out on it.
    pub shadow: Option<(&'static str, &'static str)>,
}

/// R1284000: the two sides' records compared on the seat that holds `card`.
pub fn compare(before: &[GameRecord], after: &[GameRecord], card: &str) -> Comparison {
    Comparison {
        card: card.to_string(),
        before: holder_rate(before, card),
        after: holder_rate(after, card),
        paired: paired_difference(before, after, card),
        shadow: shadow_row(card),
    }
}

/// R1284000: the comparison the arguments ask for, over the records of the files they name.
pub fn comparison(args: &CompareArgs) -> Result<Comparison> {
    let before = read_records(&args.before)?;
    let after = read_records(&args.after)?;
    Ok(compare(&before, &after, &args.card))
}

/// A share as a percentage, as `31.4%`.
fn percent(share: f64) -> String {
    format!("{:.*}%", PERCENT_DECIMALS, share * PERCENT)
}

/// A difference of shares in percentage points, signed, as `+3.0`.
fn points(share: f64) -> String {
    format!("{:+.*}", PERCENT_DECIMALS, share * PERCENT)
}

/// R1284000: the comparison as lines of text, the card named with the catalog's `name` when it has one.
pub fn render_compare(comparison: &Comparison, name: Option<&str>) -> String {
    let card = &comparison.card;
    let named = match name {
        Some(name) => format!("{card} {name}"),
        None => card.clone(),
    };
    let mut lines = vec![format!(
        "{named}: the win rate of the seat whose deck alone holds it (R1284000)"
    )];
    for (label, rate) in [("before", &comparison.before), ("after", &comparison.after)] {
        lines.push(format!(
            "  {label:<6}  {} of {} games, 95% interval {} to {}",
            percent(rate.share),
            rate.games,
            percent(rate.low),
            percent(rate.high)
        ));
    }
    lines.push(match &comparison.paired {
        Some(paired) => format!(
            "  paired  {} points over the {} games on both sides, 95% interval {} to {}",
            points(paired.mean),
            paired.games,
            points(paired.low),
            points(paired.high)
        ),
        None => "  paired  no game is on both sides".to_string(),
    });
    let (before_left, after_left) = (comparison.before.left_out, comparison.after.left_out);
    if before_left + after_left > 0 {
        lines.push(format!(
            "  left out: {before_left} game(s) before and {after_left} after, where no seat or both seats hold {card}"
        ));
    }
    if let Some((table, reason)) = comparison.shadow {
        lines.push(format!(
            "  {card} is in {table} ({reason}): its figures measure the AI as much as the card"
        ));
    }
    lines.join("\n")
}

/// `cargo jackioh stats compare`.
fn run_compare(args: &CompareArgs) -> Result<()> {
    jackioh_cards::register_all();
    let compared = comparison(args)?;
    let name = jackioh_cards::CATALOG.get(&args.card).map(|def| def.name.clone());
    println!("{}", render_compare(&compared, name.as_deref()));
    Ok(())
}

/// R1284000: the options of the run `args` ask for, `patch` being the version it tests. `--force`'s id is
/// resolved as `golden record --force` resolves it, so a token goes in through its card and an unknown id or
/// an unshipped set's card is refused. A forced run files under its default series, so its records'
/// ids never collide with a plain run's in `stats-import`; an explicit `--series` with `--force` is refused.
fn run_options(args: &Args, patch: String) -> Result<DevRunOptions> {
    let force = args.force.as_deref().map(golden::resolve_forced).transpose()?;
    if let (Some(series), Some(card)) = (&args.series, &force) {
        return Err(anyhow!(
            "--series {series} cannot go with --force: a forced run files under {FORCED_SERIES_PREFIX}{card}, for stats compare and stats report, never for stats-import (R1284000)"
        ));
    }
    let series = match &force {
        Some(card) => format!("{FORCED_SERIES_PREFIX}{card}"),
        None => args
            .series
            .clone()
            .unwrap_or_else(|| AI_DEV_RUN.series.to_string()),
    };
    Ok(DevRunOptions {
        series,
        patch,
        budget: None,
        force,
    })
}

/// `cargo jackioh stats`.
pub fn run(args: Args) -> Result<()> {
    match &args.command {
        Some(Command::Report(report)) => return run_report(report),
        Some(Command::FitDealt(fit)) => return dealt_fit::run(fit),
        Some(Command::Compare(compared)) => return run_compare(compared),
        None => {}
    }
    let games = args.games.unwrap_or(AI_DEV_RUN.games);
    jackioh_cards::register_all();

    let patch = match args.patch.clone() {
        Some(patch) => patch,
        None => {
            let text =
                std::fs::read_to_string(PATCHES_PATH).with_context(|| format!("reading {PATCHES_PATH}"))?;
            newest_patch(&serde_json::from_str(&text).with_context(|| format!("parsing {PATCHES_PATH}"))?)?
        }
    };
    // Before `--out` is created, so a refused id writes no file.
    let options = run_options(&args, patch)?;
    let series = &options.series;
    if let Some(card) = &options.force {
        eprintln!("[ai:stats] {card} forced: p1's deck in odd games, p2's in even ({series})");
    }
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
        let record = dev_game_record(n, &options);
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
            patch: Some(options.patch.clone()),
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
                force: None,
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
                "--force=core-090",
                "--out=run.jsonl"
            ])
            .unwrap(),
            Args {
                games: Some(50),
                from: 51,
                patch: Some("v0.2.5".to_string()),
                series: Some("a".to_string()),
                force: Some("core-090".to_string()),
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
            _ => panic!("`stats report` parsed as something else"),
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
                pilot: None,
                card: None,
                games: false,
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
            (SourceFilter::All, Some(GameMode::Random), Some(PilotFilter::Ai))
        );
        assert_eq!(
            (all.patch.as_deref(), all.card.as_deref(), all.json),
            (Some("v0.2.5"), Some("core-001"), true)
        );

        // --games asks for the game figures and does not go with one card's row.
        assert!(report_args(&["a.jsonl", "--games"]).unwrap().games);
        assert!(report_args(&["a.jsonl", "--games", "--card=core-001"]).is_err());

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

    /// A development game of random mode played by two AIs, first seat p1, as a line of JSON.
    /// `counts` is each seat's `[fatigueDraws, fatigueDamage, trapsSet, trapsFired, positionSwitches, radiantUpgrades]`
    /// (R1256000), or none for a record written before them.
    fn game_line(
        id: &str,
        patch: &str,
        winner: &str,
        reason: &str,
        turns: i32,
        counts: Option<[[i32; 6]; 2]>,
    ) -> String {
        let seat = |n: usize| {
            let mut seat =
                serde_json::json!({ "deck": ["core-001"], "opening": [], "drawn": [], "played": [] });
            if let Some(counts) = counts {
                let [
                    fatigue_draws,
                    fatigue_damage,
                    traps_set,
                    traps_fired,
                    switches,
                    upgrades,
                ] = counts[n];
                seat["fatigueDraws"] = fatigue_draws.into();
                seat["fatigueDamage"] = fatigue_damage.into();
                seat["trapsSet"] = traps_set.into();
                seat["trapsFired"] = traps_fired.into();
                seat["positionSwitches"] = switches.into();
                seat["radiantUpgrades"] = upgrades.into();
            }
            seat
        };
        serde_json::json!({
            "id": id,
            "source": "dev",
            "mode": "random",
            "patch": patch,
            "pilots": { "p1": "ai", "p2": "ai" },
            "game": {
                "first": "p1",
                "winner": winner,
                "reason": reason,
                "turns": turns,
                "seats": { "p1": seat(0), "p2": seat(1) }
            }
        })
        .to_string()
    }

    /// Four development games of two patches, one drawn, one at the turn cap, two without
    /// R1256000's counts, and one live game.
    fn game_lines() -> Vec<String> {
        vec![
            game_line(
                "dev:v0.2.5:dev:1",
                "v0.2.5",
                "p1",
                "hero-death",
                9,
                Some([[0, 0, 2, 1, 3, 1], [2, 3, 1, 0, 1, 2]]),
            ),
            game_line(
                "dev:v0.2.5:dev:2",
                "v0.2.5",
                "draw",
                "turn-cap",
                40,
                Some([[0; 6]; 2]),
            ),
            game_line("dev:v0.2.5:dev:3", "v0.2.5", "p2", "hero-death", 12, None),
            game_line("dev:v0.2.4:dev:1", "v0.2.4", "p1", "alt-win", 15, None),
            record_line("m1", "live", "bo1", "p1", &["core-002"]),
        ]
    }

    /// The game report of `game_lines` under `stats report --games` and these flags.
    fn games_of(flags: &[&str]) -> GameReport {
        let args = report_args(&[&["a.jsonl", "--games"], flags].concat()).unwrap();
        game_report(
            &parse_game_record_lines(&game_lines().join("\n")).unwrap(),
            &filter_of(&args),
        )
    }

    #[test]
    fn games_report_json_counts_a_draw_a_turn_cap_and_a_record_written_before_the_counts() {
        let report = games_of(&["--source=dev"]);
        assert_eq!(
            render_games(&report, true),
            serde_json::to_string_pretty(&report).unwrap()
        );
        let reasons = |counts: &[(&str, i32)]| {
            let mut all = serde_json::json!({
                "hero-death": 0, "both-heroes-dead": 0, "concede": 0, "draw-accepted": 0, "turn-cap": 0,
                "disconnect": 0, "match-ceiling": 0, "voided": 0, "alt-win": 0, "won-by-effect": 0
            });
            for (reason, count) in counts {
                all[reason] = (*count).into();
            }
            all
        };
        // Compared as values: serde_json reads a float back to within an ulp of the one written.
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::json!({
                "filter": { "source": "dev", "mode": null, "patch": null, "pilot": "unified" },
                "otherSources": 1,
                "groups": [
                    {
                        "patch": "v0.2.5", "mode": "random", "source": "dev",
                        "games": 3, "draws": 1, "firstWins": 1,
                        "firstShare": 0.333, "firstLow": 0.061, "firstHigh": 0.792,
                        "reasons": reasons(&[("hero-death", 2), ("turn-cap", 1)]),
                        "turnQuartiles": [9, 12, 40],
                        "counted": 2, "fatigueGames": 1, "fatigueShare": 0.5, "fatigueDamageMean": 1.5,
                        "trapsSet": 3, "trapsFired": 1, "trapsFiredPerSet": 0.333,
                        "positionSwitches": 4, "positionSwitchesMean": 2.0, "radiantUpgrades": 3, "radiantUpgradesMean": 1.5
                    },
                    {
                        "patch": "v0.2.4", "mode": "random", "source": "dev",
                        "games": 1, "draws": 0, "firstWins": 1,
                        "firstShare": 1.0, "firstLow": 0.207, "firstHigh": 1.0,
                        "reasons": reasons(&[("alt-win", 1)]),
                        "turnQuartiles": [15, 15, 15],
                        "counted": 0, "fatigueGames": null, "fatigueShare": null, "fatigueDamageMean": null,
                        "trapsSet": null, "trapsFired": null, "trapsFiredPerSet": null,
                        "positionSwitches": null, "positionSwitchesMean": null, "radiantUpgrades": null, "radiantUpgradesMean": null
                    }
                ]
            })
        );
    }

    #[test]
    fn games_report_text_names_the_filter_each_group_and_what_it_left_out() {
        let text = render_games(&games_of(&["--source=dev"]), false);
        let expected = [
            "Games: AI development runs, every mode, every patch, human and AI pilots (unified).",
            "",
            "v0.2.5 random dev: 3 games, 1 drawn",
            "  first seat won 1 of 3: 33.3%, 95% interval 6.1% to 79.2%",
            "  endings: hero-death 2, turn-cap 1",
            "  turns, quartiles: 9, 12, 40",
            "  fatigue: a draw from an empty library in 1 of 2 games (50.0%), 1.50 damage a game",
            "  traps: 1 fired of 3 set, 0.33 a set",
            "  position switches: 4, 2.00 a game",
            "  Radiant upgrades: 3, 1.50 a game",
            "  (fatigue, traps, switches and upgrades over the 2 of 3 games whose records carry R1256000's counts)",
            "",
            "v0.2.4 random dev: 1 games, 0 drawn",
            "  first seat won 1 of 1: 100.0%, 95% interval 20.7% to 100.0%",
            "  endings: alt-win 1",
            "  turns, quartiles: 15, 15, 15",
            "  fatigue, traps, switches and upgrades: unknown, no record of these games carries R1256000's counts",
            "",
            "Left out: 1 records of a source --source=dev does not read (R378).",
        ];
        assert_eq!(text, expected.join("\n"));
    }

    #[test]
    fn games_report_adds_both_seats_switches_and_upgrades_whichever_seat_holds_them() {
        let totals = |counts: [[i32; 6]; 2]| {
            let line = game_line("dev:v0.2.5:dev:1", "v0.2.5", "p1", "hero-death", 9, Some(counts));
            let report = game_report(
                &parse_game_record_lines(&line).unwrap(),
                &filter_of(&report_args(&["a.jsonl", "--games", "--source=dev"]).unwrap()),
            );
            let group = &report.groups[0];
            (group.position_switches, group.radiant_upgrades)
        };
        let (p1, p2) = ([0, 0, 0, 0, 3, 1], [0, 0, 0, 0, 1, 2]);
        assert_eq!(totals([p1, p2]), (Some(4), Some(3)));
        assert_eq!(totals([p2, p1]), (Some(4), Some(3)));
    }

    #[test]
    fn games_report_counts_a_game_only_when_both_seats_carry_all_six_counts() {
        // A seat that carries the fatigue and trap counts but not the switches: unknown, not zero.
        let mut partial: Value = serde_json::from_str(&game_line(
            "dev:v0.2.5:dev:4",
            "v0.2.5",
            "p1",
            "hero-death",
            10,
            Some([[1; 6]; 2]),
        ))
        .unwrap();
        partial["game"]["seats"]["p2"]
            .as_object_mut()
            .unwrap()
            .remove("positionSwitches");
        let mut lines = game_lines();
        lines.push(partial.to_string());
        let args = report_args(&["a.jsonl", "--games", "--source=dev"]).unwrap();
        let report = game_report(
            &parse_game_record_lines(&lines.join("\n")).unwrap(),
            &filter_of(&args),
        );
        let group = &report.groups[0];
        assert_eq!((group.games, group.counted), (4, 2));
        assert_eq!(group.fatigue_games, Some(1));
        assert_eq!(
            (group.position_switches, group.radiant_upgrades),
            (Some(4), Some(3))
        );
        assert!(render_games(&report, false).contains(
            "  (fatigue, traps, switches and upgrades over the 2 of 4 games whose records carry R1256000's counts)"
        ));
    }

    #[test]
    fn games_report_reads_the_records_the_card_report_reads() {
        let groups = |flags: &[&str]| -> Vec<String> {
            games_of(flags)
                .groups
                .iter()
                .map(|group| format!("{} {} {}", group.patch, group.mode, group.source))
                .collect()
        };
        let both_dev = ["v0.2.5 random dev", "v0.2.4 random dev"];

        // R378: live games unless a development run is named.
        assert_eq!(groups(&[]), ["v0.2.5 bo1 live"]);
        assert_eq!(games_of(&[]).other_sources, 4);
        assert_eq!(
            groups(&["--source=all"]),
            ["v0.2.5 random dev", "v0.2.4 random dev", "v0.2.5 bo1 live"]
        );
        assert_eq!(groups(&["--source=all", "--patch=v0.2.4"]), ["v0.2.4 random dev"]);
        assert_eq!(games_of(&["--source=all", "--patch=v0.2.4"]).other_sources, 0);
        assert_eq!(groups(&["--source=all", "--mode=bo1"]), ["v0.2.5 bo1 live"]);
        // A game counts when a seat its pilot reads is in it.
        assert_eq!(groups(&["--source=all", "--pilot=human"]), ["v0.2.5 bo1 live"]);
        assert_eq!(groups(&["--source=all", "--pilot=ai"]), both_dev);

        let none = games_of(&["--patch=v0.0.1"]);
        assert!(none.groups.is_empty());
        assert!(render_games(&none, false).contains("No game the filter reads."));
    }

    #[test]
    fn fit_dealt_names_record_files_and_takes_no_run_option() {
        assert_eq!(
            parse(&["fit-dealt", "a.jsonl", "b.jsonl"]).unwrap().command,
            Some(Command::FitDealt(dealt_fit::Args {
                files: vec![PathBuf::from("a.jsonl"), PathBuf::from("b.jsonl")],
            }))
        );
        assert!(parse(&["fit-dealt"]).is_err());
        assert!(parse(&["fit-dealt", "a.jsonl", "--games=3"]).is_err());
    }

    /// A game record as a line of JSON: the shape `--out` writes. A live game is two people's, a
    /// development run's two AIs'.
    fn record_line(id: &str, source: &str, mode: &str, winner: &str, deck_p2: &[&str]) -> String {
        let pilot = if source == "live" { "human" } else { "ai" };
        record_line_piloted(id, source, mode, winner, deck_p2, [pilot, pilot])
    }

    fn record_line_piloted(
        id: &str,
        source: &str,
        mode: &str,
        winner: &str,
        deck_p2: &[&str],
        pilots: [&str; 2],
    ) -> String {
        serde_json::json!({
            "id": id,
            "source": source,
            "mode": mode,
            "patch": "v0.2.5",
            "pilots": { "p1": pilots[0], "p2": pilots[1] },
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

        // Live games unless asked otherwise (R378), every counted seat merged (R807000).
        let (_, by_default) = report_of(&paths, &[]);
        assert_eq!(by_default.games, 1);
        let live_merged = CardStatsFilter {
            pilot: PilotFilter::Unified,
            ..DEFAULT_CARD_STATS_FILTER.clone()
        };
        assert_eq!(by_default, card_stats(&records, &live_merged));

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
    fn r807000_report_splits_human_bot_and_merged_and_displays_merged() {
        // p1 a person, p2 a ladder bot holding core-003; and a development game, both seats AIs.
        // p1's deck holds core-001 and core-002, and p2 (the winner) holds core-003.
        let bot_game = record_line_piloted("m1", "live", "bo1", "p2", &["core-003"], ["human", "ai"]);
        let dev = record_line("dev:v0.2.5:dev:1", "dev", "random", "p1", &["core-003"]);
        let (dir, paths) = scratch(&[("a.jsonl", format!("{bot_game}\n{dev}\n"))]);
        let held = |report: &CardStatsReport, card: &str| {
            report
                .cards
                .iter()
                .find(|stats| stats.card == card)
                .map(|stats| stats.in_deck.games)
        };

        // With no --pilot: every counted seat merged, both sides of the bot game.
        let (_, by_default) = report_of(&paths, &[]);
        assert_eq!((by_default.games, by_default.decks), (1, 2));
        assert_eq!(by_default.filter.pilot, PilotFilter::Unified);
        assert_eq!(held(&by_default, "core-001"), Some(1));
        assert_eq!(held(&by_default, "core-003"), Some(1));
        // Split: each side reads alone.
        let (_, people) = report_of(&paths, &["--pilot=human"]);
        assert_eq!((people.games, people.decks), (1, 1));
        assert_eq!(held(&people, "core-001"), Some(1));
        assert_eq!(held(&people, "core-003"), None);
        let (_, bots) = report_of(&paths, &["--pilot=ai"]);
        assert_eq!((bots.games, bots.decks), (1, 1));
        assert_eq!(held(&bots, "core-001"), None);
        assert_eq!(held(&bots, "core-003"), Some(1));
        // Development data counts every seat unless told otherwise, as stats-cards does.
        let (_, dev_default) = report_of(&paths, &["--source=dev"]);
        assert_eq!(
            (dev_default.filter.pilot, dev_default.decks),
            (PilotFilter::Unified, 2)
        );
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

    /// The options a run of `argv` plays, as `run` reads them under the patch "v0.2.5".
    fn options_of(argv: &[&str]) -> Result<DevRunOptions> {
        jackioh_cards::register_all();
        run_options(&parse(argv).unwrap(), "v0.2.5".to_string())
    }

    #[test]
    fn r1284000_force_takes_a_tokens_card_and_refuses_an_unknown_id_or_an_unshipped_sets_card() {
        let plain = options_of(&[]).unwrap();
        assert_eq!((plain.series.as_str(), plain.force), ("dev", None));
        assert_eq!(plain.patch, "v0.2.5");

        // A forced run files under a series of its own; an explicit series with --force is refused.
        let forced = options_of(&["--force=core-090"]).unwrap();
        assert_eq!(
            (forced.series.as_str(), forced.force.as_deref()),
            ("dev-force-core-090", Some("core-090"))
        );
        // R279: a token goes in through the card whose refs name it, and the series names that card.
        let token = options_of(&["--force=core-t-felinor"]).unwrap();
        assert_eq!(
            (token.series.as_str(), token.force.as_deref()),
            ("dev-force-classic-032", Some("classic-032"))
        );
        let refused = options_of(&["--force=core-090", "--series=a"])
            .unwrap_err()
            .to_string();
        assert!(refused.contains("--series"), "{refused}");
        assert!(refused.contains("--force"), "{refused}");
        assert!(refused.contains("R1284000"), "{refused}");
        assert!(refused.contains("dev-force-core-090"), "{refused}");
        assert_eq!(options_of(&["--series=a"]).unwrap().series, "a");

        let unknown = options_of(&["--force=no-such-card"]).unwrap_err().to_string();
        assert!(unknown.contains("not in the catalog"), "{unknown}");
        let carried = options_of(&["--force=classic-t-glitch"]).unwrap_err().to_string();
        assert!(
            carried.contains("no deckable card's refs name this token"),
            "{carried}"
        );
        // R1420: no catalog set is unshipped today, so a card of the reserved Boss set is made from one.
        let mut unshipped = jackioh_cards::CATALOG["core-090"].clone();
        unshipped.set = jackioh_engine::SetName::Boss;
        let refused = golden::carrier_of("core-090", &unshipped)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("has not shipped (R1420)"), "{refused}");
    }

    /// `compare`'s options as `main.rs` mounts the subcommand.
    fn compare_args(argv: &[&str]) -> Result<CompareArgs, clap::Error> {
        let cli = Cli::try_parse_from(["stats", "compare"].into_iter().chain(argv.iter().copied()))?;
        match cli.args.command {
            Some(Command::Compare(compared)) => Ok(compared),
            _ => panic!("`stats compare` parsed as something else"),
        }
    }

    #[test]
    fn r1284000_compare_reads_its_options() {
        assert_eq!(
            compare_args(&[
                "--before", "a.jsonl", "b.jsonl", "--after", "c.jsonl", "--card", "core-090"
            ])
            .unwrap(),
            CompareArgs {
                before: vec![PathBuf::from("a.jsonl"), PathBuf::from("b.jsonl")],
                after: vec![PathBuf::from("c.jsonl")],
                card: "core-090".to_string(),
            }
        );
        assert!(compare_args(&["--before", "a.jsonl", "--card=core-090"]).is_err());
        assert!(compare_args(&["--before=a.jsonl", "--after=c.jsonl"]).is_err());
        assert!(compare_args(&["--after=c.jsonl", "--card=core-090"]).is_err());
        assert!(
            compare_args(&[
                "--before=a.jsonl",
                "--after=c.jsonl",
                "--card=core-090",
                "--games=3"
            ])
            .is_err()
        );
    }

    /// A forced run's record as a line of JSON, `record_line_piloted`'s shape: game `n` of `patch`'s run, the
    /// card "test-card" in the deck of `holder` ("p1", "p2", "none" or "both") and `winner` the winner.
    fn forced_line(patch: &str, n: i32, holder: &str, winner: &str) -> String {
        let deck = |seat: &str| {
            let mut deck = vec!["core-001", "core-002"];
            if holder == seat || holder == "both" {
                deck.push("test-card");
            }
            deck
        };
        serde_json::json!({
            "id": format!("dev:{patch}:t:{n}"),
            "source": "dev",
            "mode": "random",
            "patch": patch,
            "pilots": { "p1": "ai", "p2": "ai" },
            "game": {
                "first": "p1",
                "winner": winner,
                "reason": "hero-death",
                "turns": 9,
                "seats": {
                    "p1": { "deck": deck("p1"), "opening": [], "drawn": [], "played": [] },
                    "p2": { "deck": deck("p2"), "opening": [], "drawn": [], "played": [] }
                }
            }
        })
        .to_string()
    }

    /// Game `n` of a forced run, the holder (p1 when n is odd) winning or losing.
    fn forced_game(patch: &str, n: i32, holder_wins: bool) -> String {
        let (holder, other) = if n % 2 == 1 { ("p1", "p2") } else { ("p2", "p1") };
        forced_line(patch, n, holder, if holder_wins { holder } else { other })
    }

    /// Before (v0.2.5): the holder wins games 1 to 5 and loses 6 to 10; game 11 has no holder. After
    /// (v0.2.6): it wins 1 to 7, loses 8 and 9 and draws 10; wins 12, which only this side played; and
    /// game 13 has two holders.
    fn forced_runs() -> (Vec<GameRecord>, Vec<GameRecord>) {
        let mut before: Vec<String> = (1..=10).map(|n| forced_game("v0.2.5", n, n <= 5)).collect();
        before.push(forced_line("v0.2.5", 11, "none", "p1"));
        let mut after: Vec<String> = (1..=9).map(|n| forced_game("v0.2.6", n, n <= 7)).collect();
        after.push(forced_line("v0.2.6", 10, "p2", "draw"));
        after.push(forced_game("v0.2.6", 12, true));
        after.push(forced_line("v0.2.6", 13, "both", "p1"));
        let parse = |lines: Vec<String>| parse_game_record_lines(&lines.join("\n")).unwrap();
        (parse(before), parse(after))
    }

    #[test]
    fn r1284000_compare_prints_each_sides_rate_the_paired_difference_and_what_it_left_out() {
        let (before, after) = forced_runs();
        let compared = compare(&before, &after, "test-card");
        assert_eq!(
            (
                compared.before.games,
                compared.before.wins,
                compared.before.left_out
            ),
            (10, 5, 1)
        );
        assert_eq!(
            (compared.after.games, compared.after.wins, compared.after.left_out),
            (11, 8, 1)
        );
        assert_eq!(compared.paired.map(|paired| paired.games), Some(10));
        assert_eq!(compared.shadow, None);
        assert_eq!(
            render_compare(&compared, Some("Alpha")),
            "test-card Alpha: the win rate of the seat whose deck alone holds it (R1284000)\n\
             \x20 before  50.0% of 10 games, 95% interval 23.7% to 76.3%\n\
             \x20 after   72.7% of 11 games, 95% interval 43.4% to 90.3%\n\
             \x20 paired  +20.0 points over the 10 games on both sides, 95% interval -6.1 to +46.1\n\
             \x20 left out: 1 game(s) before and 1 after, where no seat or both seats hold test-card"
        );
        // A card the catalog does not name is printed by its id alone.
        assert!(render_compare(&compared, None).starts_with("test-card: the win rate"));
    }

    #[test]
    fn r1284000_compare_pairs_a_game_by_its_seed_once_and_only_where_both_sides_have_a_holder() {
        let line = |patch: &str, n: i32, holder: &str, winner: &str| forced_line(patch, n, holder, winner);
        // Game 1 is twice on the before side: its first record is the one paired. Game 2 has no holder
        // before, and game 3 none after.
        let before = parse_game_record_lines(
            &[
                line("v0.2.5", 1, "p1", "p1"),
                line("v0.2.5", 1, "p1", "p2"),
                line("v0.2.5", 2, "none", "p1"),
                line("v0.2.5", 3, "p1", "p1"),
            ]
            .join("\n"),
        )
        .unwrap();
        let after = parse_game_record_lines(
            &[
                line("v0.2.6", 1, "p1", "p2"),
                line("v0.2.6", 2, "p2", "p2"),
                line("v0.2.6", 3, "none", "p2"),
            ]
            .join("\n"),
        )
        .unwrap();
        let paired = compare(&before, &after, "test-card").paired.unwrap();
        // Game 1: won before, lost after, so −1; the one game has no spread, so the interval is all of it.
        assert_eq!(
            (paired.games, paired.mean, paired.low, paired.high),
            (1, -1.0, -1.0, 1.0)
        );
        // No game is on both sides at all.
        let apart = compare(&before[..1], &after[1..2], "test-card");
        assert_eq!(apart.paired, None);
        assert!(render_compare(&apart, None).contains("\n  paired  no game is on both sides"));
    }

    #[test]
    fn r1284000_compare_adds_the_shadow_ban_or_watch_line() {
        // The ids come from the tables, which every sweep rewrites.
        let (banned, why_banned) = SHADOW_BAN[0];
        let compared = compare(&[], &[], banned);
        assert_eq!(compared.shadow, Some(("SHADOW_BAN", why_banned)));
        let text = render_compare(&compared, None);
        assert!(text.contains(&format!("{banned} is in SHADOW_BAN (")), "{text}");
        assert!(
            text.ends_with("its figures measure the AI as much as the card"),
            "{text}"
        );

        let (watched, why_watched) = SHADOW_WATCH[0];
        let forced = |patch: &str, win: bool| {
            let lines: Vec<String> = (1..=2)
                .map(|n| {
                    forced_line(patch, n, "p1", if win { "p1" } else { "p2" }).replace("test-card", watched)
                })
                .collect();
            lines.join("\n") + "\n"
        };
        let (dir, paths) = scratch(&[
            ("before.jsonl", forced("v0.2.5", false)),
            ("after.jsonl", forced("v0.2.6", true)),
        ]);
        let args = compare_args(&[
            "--before",
            paths[0].to_str().unwrap(),
            "--after",
            paths[1].to_str().unwrap(),
            "--card",
            watched,
        ])
        .unwrap();
        let compared = comparison(&args).unwrap();
        assert_eq!(compared.shadow, Some(("SHADOW_WATCH", why_watched)));
        assert_eq!(
            compared.paired.map(|paired| (paired.games, paired.mean)),
            Some((2, 1.0))
        );
        let text = render_compare(&compared, None);
        assert!(
            text.contains(&format!("{watched} is in SHADOW_WATCH (")),
            "{text}"
        );
        assert!(
            text.ends_with("its figures measure the AI as much as the card"),
            "{text}"
        );
        assert!(!text.contains("left out"), "{text}");

        // A file that is not records is an error naming it, as `report`'s is.
        let missing = dir.join("missing.jsonl");
        let refused = comparison(&CompareArgs {
            before: vec![missing.clone()],
            ..args
        })
        .unwrap_err();
        assert!(format!("{refused:#}").contains(&format!("reading {}", missing.display())));
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
