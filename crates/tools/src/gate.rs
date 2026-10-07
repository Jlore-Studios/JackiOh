//! `cargo jackioh gate [--full] [--shard k/K] [--out dir]` and `cargo jackioh gate merge [dir]`: the
//! AI's quality gates (docs/polish/3-ai.md B28–B31, B42, SPEC §9.9; SURFACE §12). Port of
//! `packages/ai/test/gate-random.test.ts`, `gate-greedy.test.ts`, `gate-hard-easy.test.ts`,
//! `gate-perf.test.ts`, their shard helper `test/_shard.ts` and `packages/ai/scripts/gate-merge.ts`.
//! The gate modules' games are played and measured by `jackioh_ai::gate` (part 17); this file turns
//! a run into a pass or a fail, as the TS gate files did.
//!
//! Three matchups, each a run of seeded games in which the subject (the AI, or the Hard AI)
//! alternates seats, every game folded back from its log to prove it replays (B31):
//!
//! - `ai-vs-random` (B28): the AI on Easy at AI_GATE_BUDGET against §10.7's random policy.
//! - `ai-vs-greedy` (B29): the AI on Easy against the one-ply greedy baseline at equal (Easy)
//!   resources.
//! - `hard-vs-easy` (R180, B30): the same AI with Hard's handicap against itself on Easy. The tiers
//!   differ in resources alone, so the only thing separating the two seats is the handicap.
//!
//! The subject must win at least `gateNeeded(matchup, n)` of its n games (the rule SPEC §9.9 gives
//! every gate's count, held here by `check_gate_rule`). Only wins count, in every gate: a draw at the
//! turn cap is reported beside the wins (`turnCapDraws`) and counts for nothing. Without `--full` the
//! command plays the smoke size (AI_GATE.smokeSeeds per matchup, what `pnpm test` played); `--full`
//! (or `JACKIOH_AI_GATE=full`, as `pnpm ai:gate` set it) plays AI_GATE.fullSeeds[matchup]. A failure
//! names the seeds the subject did not win, which replay exactly through `gameConfig(matchup, n)`.
//! Every game also has to be clean (B31): nothing rejected, nothing thrown, no fallback, a real
//! result, and a log that folds back to the live hash.
//!
//! After the matchups the perf gate (B42) runs: a decision at the browser's budget stays under
//! AI_GATE.maxDecisionMs as the development machine would time it (`perf_gate`).
//!
//! CI plays the full gates in shards so no job runs for long (.github/workflows/ci.yml).
//! `--shard k/K` (or `JACKIOH_AI_GATE_SHARD=k/K`) makes each gate play only its share of the games
//! (`gateShardGames`): every game it plays must still be clean (B31) and the perf gate still times
//! every decision of its games, but a win-rate threshold is a property of the whole run, so a shard
//! writes its games to `--out` (or `JACKIOH_AI_GATE_OUT`, default `ai-gate-shards/`) and
//! `cargo jackioh gate merge` holds the wins of all shards together against `gateNeeded`.
//! Unsharded, every gate plays and judges its whole run as before.
//!
//! The games of one matchup are played in parallel (rayon), one game per task: a gate game is a pure
//! function of its matchup and number, so the run, its order and its verdict are exactly what one
//! sequential `runGateGames` gives. The perf gate times one decision at a time, never in parallel.
//!
//! Output goes to stdout; a failed gate returns `Err` naming every problem (exit 1).

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use indexmap::{IndexMap, IndexSet};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use jackioh_ai::{
    AI_BUDGET, AI_EVAL, AI_GATE, AI_GATE_BUDGET, AI_TUNING_SERIES, AiDeckOptions, AiOptions, GREEDY_EVAL,
    GateGame, GateReport, MatchConfig, MatchHooks, Matchup, SHADOW_BAN, SeatController, binomial_tail, build_ai_deck,
    candidate_actions, decide, game_config, gate_needed, gate_shard_games, greedy_action, play_match,
    run_gate_games,
};
use jackioh_engine::config::{AI_DIFFICULTY, HUMAN_HANDICAP};
use jackioh_engine::rng::Rng;
use jackioh_engine::{ActionBody, GameOverReason, GameState, Handicap, PerPlayer, PlayerId, Winner, opponent_of};

/// `cargo jackioh gate …` (SURFACE §12).
#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<GateCommand>,
    /// Play AI_GATE.fullSeeds per matchup and AI_GATE.perfFullGames perf games, as `pnpm ai:gate`
    /// did (also `JACKIOH_AI_GATE=full`). Without it, the smoke size.
    #[arg(long)]
    pub full: bool,
    /// Play only shard k of K ("k/K", 1 ≤ k ≤ K) of every gate's games, and write them to `--out`
    /// for `gate merge` (also `JACKIOH_AI_GATE_SHARD`).
    #[arg(long, value_name = "k/K")]
    pub shard: Option<String>,
    /// Where a shard writes its games (also `JACKIOH_AI_GATE_OUT`; default `ai-gate-shards`).
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

/// `gate`'s one subcommand.
#[derive(Subcommand, Debug)]
pub enum GateCommand {
    /// The full gates' verdict when CI has played them in shards (`gate-merge.ts`).
    Merge {
        /// The directory the shards wrote their files to.
        #[arg(default_value = DEFAULT_SHARD_DIR)]
        dir: PathBuf,
    },
}

/// Where a shard writes its games, and where `gate merge` reads them, unless told otherwise.
const DEFAULT_SHARD_DIR: &str = "ai-gate-shards";

/// The three matchups, in the order every gate file and `gate-merge.ts` listed them.
const MATCHUP_NAMES: [&str; 3] = ["ai-vs-random", "ai-vs-greedy", "hard-vs-easy"];

/// The games of n 1..=GAME_CONFIG_PROBES whose seating every gate's first check reads.
const GAME_CONFIG_PROBES: i32 = 3;

/// B29: the games whose decks the shadow-ban check deals.
const SHADOW_BAN_PROBES: i32 = 40;

/// B29: the greedy-against-greedy game the frozen-weights check plays, and its action limit.
const GREEDY_PROBE_GAME: i32 = 2;
const GREEDY_PROBE_ACTIONS: usize = 120;
/// B29: greedy has to have acted in its main phase more often than this for the check to mean anything.
const GREEDY_PROBE_MIN_STATES: usize = 3;

/// `toBeCloseTo(x, 12)`: equal within half of 10^-12.
const CLOSE_TO_12: f64 = 0.5e-12;
/// `toBeCloseTo(x, 10)`.
const CLOSE_TO_10: f64 = 0.5e-10;

pub(crate) const MS_PER_SECOND: f64 = 1000.0;

// ---------------------------------------------------------------------------
// Matchups, seats and the gate's numbers
// ---------------------------------------------------------------------------

/// A matchup's literal as TS wrote it (`"ai-vs-random"` …).
pub(crate) fn matchup_name(matchup: Matchup) -> String {
    matchup.as_str().to_string()
}

/// The matchup a literal names (`"ai-vs-greedy"` …), or an error listing the three.
pub(crate) fn parse_matchup(text: &str) -> Result<Matchup> {
    serde_json::from_value(Value::String(text.to_string()))
        .map_err(|_| anyhow!("unknown matchup {text:?}: expected one of {}", MATCHUP_NAMES.join(", ")))
}

/// The three matchups, in TS's order.
pub(crate) fn matchups() -> Vec<Matchup> {
    Matchup::ALL.to_vec()
}

/// A serialised string union's literal (a matchup, a decision reason, a sweep flag); any other value
/// as its JSON text.
pub(crate) fn literal<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => text,
        Ok(other) => other.to_string(),
        Err(error) => format!("<{error}>"),
    }
}

/// Game n (1-based): the subject sits p1 when n is odd and p2 when n is even.
pub(crate) fn subject_seat_of(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// AI_GATE.fullSeeds[matchup].
fn full_seeds(matchup: Matchup) -> i32 {
    AI_GATE.full_seeds[matchup]
}

/// AI_GATE.smokeSeeds.
fn smoke_seeds() -> i32 {
    AI_GATE.smoke_seeds
}

/// `gateNeeded(matchup, games)`.
fn needed(matchup: Matchup, games: i32) -> i32 {
    gate_needed(matchup, games) as i32
}

/// `Math.ceil(AI_GATE.briefRate[matchup] * games)`: the most any count may ask for.
fn brief_count(matchup: Matchup, games: i32) -> i32 {
    (AI_GATE.brief_rate[matchup] * f64::from(games)).ceil() as i32
}

/// The game's seed, `${AI_GATE.seedSeries}:${matchup}:${n}`.
fn gate_seed(matchup: Matchup, n: i32) -> String {
    format!("{}:{}:{n}", AI_GATE.seed_series, matchup_name(matchup))
}

/// `config.decks[at(seat)]`.
fn deck_of(config: &MatchConfig, seat: PlayerId) -> &Vec<String> {
    match seat {
        PlayerId::P1 => &config.decks.0,
        PlayerId::P2 => &config.decks.1,
    }
}

/// `config.handicaps?.[seat] ?? HUMAN_HANDICAP`.
fn handicap_of(config: &MatchConfig, seat: PlayerId) -> Handicap {
    config.handicaps.as_ref().and_then(|handicaps| handicaps.get(seat)).copied().unwrap_or(HUMAN_HANDICAP)
}

/// Every seat's deck rule (R186 included): `buildAiDeck(createRng(`${seed}:deck:${seat}`),
/// handicap.deckSize, { manaCap: handicap.manaCap })`.
fn gate_deck(seed: &str, seat: PlayerId, handicap: &Handicap) -> Vec<String> {
    let mut rng = Rng::new(&format!("{seed}:deck:{seat}"), 0);
    build_ai_deck(&mut rng, handicap.deck_size, &AiDeckOptions { mana_cap: Some(handicap.mana_cap), ..AiDeckOptions::default() })
}

/// TS's `expect(…).toBe(…)` as a check: `Err` naming the label and both sides.
fn expect_eq<T: PartialEq + std::fmt::Debug>(actual: T, expected: T, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!("{label}: expected {expected:?}, got {actual:?}"))
    }
}

/// TS's `expect(cond, label).toBe(true)` as a check.
fn expect_that(holds: bool, label: impl FnOnce() -> String) -> Result<()> {
    if holds { Ok(()) } else { Err(anyhow!(label())) }
}

/// A game's result as `JSON.stringify(game.record.result)` wrote it.
fn result_json(result: &Option<jackioh_engine::GameResult>) -> String {
    serde_json::to_string(result).unwrap_or_else(|error| format!("<{error}>"))
}

/// A draw at the turn cap: reported beside the wins, and counts for nothing.
fn is_turn_cap_draw(result: &Option<jackioh_engine::GameResult>) -> bool {
    matches!(result, Some(r) if r.winner == Winner::Draw && r.reason == GameOverReason::TurnCap)
}

// ---------------------------------------------------------------------------
// Shards (test/_shard.ts)
// ---------------------------------------------------------------------------
//
// CI plays the full gates in shards so no job runs for long (.github/workflows/ci.yml). A shard plays
// only its share of the games (`gateShardGames`): every game it plays must still be clean (B31) and
// the perf gate still times every decision of its games, but a win-rate threshold is a property of
// the whole run, so a shard writes its games out and `gate merge` holds the wins of all shards
// together against `gateNeeded`. A shard keeps the whole run's allowance rather than a share of it:
// one game can take minutes on its own (a perf game of patch v0.2.0 outran 180 s), and the CI job's
// own timeout bounds a shard.

/// One shard of a gate run: shard `index` of `count`, both 1-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GateShard {
    index: i32,
    count: i32,
}

/// The shard this process plays, from `--shard` or `JACKIOH_AI_GATE_SHARD` ("k/K", 1 ≤ k ≤ K), or none.
fn gate_shard(raw: Option<&str>) -> Result<Option<GateShard>> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let parsed = raw.trim().split_once('/').and_then(|(index, count)| {
        if digits(index) && digits(count) {
            Some((index.parse::<i32>().ok()?, count.parse::<i32>().ok()?))
        } else {
            None
        }
    });
    match parsed {
        Some((index, count)) if index >= 1 && count >= 1 && index <= count => Ok(Some(GateShard { index, count })),
        _ => bail!(
            "JACKIOH_AI_GATE_SHARD (--shard) must be \"k/K\" with 1 ≤ k ≤ K, not {}",
            serde_json::to_string(raw).unwrap_or_default()
        ),
    }
}

/// The game numbers (1-based) this process plays out of a run of `total`.
fn games_to_play(total: i32, shard: Option<GateShard>) -> Vec<i32> {
    let numbers = match shard {
        None => gate_shard_games(total, 1, 1),
        Some(shard) => gate_shard_games(total, shard.index, shard.count),
    };
    numbers.into_iter().collect()
}

/// One shard's games, as `gate merge` reads them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ShardFile {
    matchup: String,
    total: i32,
    shard: String,
    games: Vec<ShardGame>,
}

/// One game of a shard file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ShardGame {
    n: i32,
    seed: String,
    subject_seat: String,
    won: bool,
    turn_cap_draw: bool,
    result: Value,
}

/// Writes a shard's games to `dir` (`--out`, `JACKIOH_AI_GATE_OUT`, default `ai-gate-shards/` in the
/// cwd) and answers the file's path.
fn write_shard(run: &GateReport, total: i32, shard: GateShard, played: &[i32], dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let file = ShardFile {
        matchup: matchup_name(run.matchup),
        total,
        shard: format!("{}/{}", shard.index, shard.count),
        games: run
            .games
            .iter()
            .enumerate()
            .map(|(at, game)| ShardGame {
                n: played.get(at).copied().unwrap_or(0),
                seed: game.seed.clone(),
                subject_seat: game.subject_seat.to_string(),
                won: game.won,
                turn_cap_draw: is_turn_cap_draw(&game.record.result),
                result: serde_json::to_value(game.record.result).unwrap_or(Value::Null),
            })
            .collect(),
    };
    let path = dir.join(format!("{}-{}-of-{}.json", matchup_name(run.matchup), shard.index, shard.count));
    std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&file)?))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// gate merge (scripts/gate-merge.ts)
// ---------------------------------------------------------------------------
//
// The full quality gates' verdict when CI has played them in shards (.github/workflows/ci.yml). Each
// shard wrote the games it played, one file per matchup, to `dir` (default `ai-gate-shards/`); this
// reads them all and, per matchup, holds the run to exactly what one unsharded `gate --full` would:
// every game 1..AI_GATE.fullSeeds[matchup] played exactly once, and at least
// `gateNeeded(matchup, games)` won. The shards have already checked each game is clean (B31) and
// timed every perf decision (B42), which need no merging. A failure names the first problem: a
// missing or doubled game, or too few wins with the seeds lost.

/// Every `.json` file under `dir`, recursively, in a stable (sorted) order.
fn json_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            found.extend(json_files(&path)?);
        } else if path.to_string_lossy().ends_with(".json") {
            found.push(path);
        }
    }
    Ok(found)
}

/// `gate merge <dir>`.
fn merge(dir: &Path) -> Result<()> {
    let files = json_files(dir)?
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str::<ShardFile>(&text).with_context(|| format!("parsing {}", path.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut problems: Vec<String> = Vec::new();
    for matchup in matchups() {
        let name = matchup_name(matchup);
        let total = full_seeds(matchup);
        let needed = needed(matchup, total);
        let mut games: Vec<&ShardGame> = Vec::new();
        for file in files.iter().filter(|file| file.matchup == name) {
            if file.total != total {
                bail!("{name}: shard {} played a run of {}, not {total}", file.shard, file.total);
            }
            games.extend(file.games.iter());
        }

        let mut seen: IndexMap<i32, i32> = IndexMap::new();
        for game in &games {
            *seen.entry(game.n).or_insert(0) += 1;
        }
        let mut missing: Vec<i32> = Vec::new();
        let mut doubled: Vec<i32> = Vec::new();
        for n in 1..=total {
            let count = seen.get(&n).copied().unwrap_or(0);
            if count == 0 {
                missing.push(n);
            }
            if count > 1 {
                doubled.push(n);
            }
        }
        let strays: Vec<i32> = seen.keys().copied().filter(|n| !(*n >= 1 && *n <= total)).collect();

        let wins = games.iter().filter(|game| game.won).count();
        let draws = games.iter().filter(|game| game.turn_cap_draw).count();
        println!("[gate {name}] {wins} wins and {draws} turn-cap draws of {total}; {needed} wins needed");

        let list = |numbers: &[i32]| numbers.iter().map(i32::to_string).collect::<Vec<_>>().join(", ");
        if !missing.is_empty() || !doubled.is_empty() || !strays.is_empty() {
            problems.push(format!(
                "{name}: games missing [{}], played twice [{}], out of range [{}]; every shard must upload its file",
                list(&missing),
                list(&doubled),
                list(&strays)
            ));
        } else if (wins as i32) < needed {
            let mut lost: Vec<&&ShardGame> = games.iter().filter(|game| !game.won).collect();
            lost.sort_by_key(|game| game.n);
            let lost = lost
                .iter()
                .map(|game| format!("{} ({}, {})", game.seed, game.subject_seat, game.result))
                .collect::<Vec<_>>()
                .join(", ");
            problems.push(format!("{name}: {wins} wins, {needed} needed; not won: {lost}"));
        }
    }
    if problems.is_empty() { Ok(()) } else { Err(anyhow!(problems.join("\n"))) }
}

// ---------------------------------------------------------------------------
// The matchups (gate-random.test.ts, gate-greedy.test.ts, gate-hard-easy.test.ts)
// ---------------------------------------------------------------------------

/// Plays the listed games of a matchup (1-based, as `gameConfig` numbers them), each folded with its
/// handicaps by `runGateGames` to fill replayHash/replayErrors. One game per rayon task, collected in
/// the listed order.
fn play_gate(matchup: Matchup, played: &[i32]) -> GateReport {
    let per_game: Vec<Vec<GateGame>> =
        played.par_iter().map(|&n| run_gate_games(matchup, &[n], AI_GATE_BUDGET).games).collect();
    let games: Vec<GateGame> = per_game.into_iter().flatten().collect();
    let wins = games.iter().filter(|game| game.won).count() as i32;
    let turn_cap_draws = games.iter().filter(|game| is_turn_cap_draw(&game.record.result)).count() as i32;
    let rate = if games.is_empty() { 0.0 } else { f64::from(wins) / games.len() as f64 };
    GateReport { matchup, games, wins, turn_cap_draws, rate }
}

/// The seeds the subject did not win, each with its seat and result.
fn losing_seeds(run: &GateReport) -> String {
    run.games
        .iter()
        .filter(|game| !game.won)
        .map(|game| format!("{} ({}, {})", game.seed, game.subject_seat, result_json(&game.record.result)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// B28/B29/R180 B30: `gameConfig` seats the subject and its opponent as the matchup says, alternating
/// seats, both on the specified handicaps and decks. ai-vs-random: the Easy AI against the random
/// policy; ai-vs-greedy: the Easy AI against the greedy baseline at equal resources; hard-vs-easy:
/// Hard's handicap against Easy's, the same AI and budget on both.
fn check_game_config(matchup: Matchup) -> Result<()> {
    let name = matchup_name(matchup);
    let hard_vs_easy = name == "hard-vs-easy";
    for n in 1..=GAME_CONFIG_PROBES {
        let config = game_config(matchup, n, AI_GATE_BUDGET, AI_GATE.seed_series);
        let seed = gate_seed(matchup, n);
        let subject = subject_seat_of(n);
        let other = opponent_of(subject);
        let subject_handicap = if hard_vs_easy { AI_DIFFICULTY.hard } else { AI_DIFFICULTY.easy };
        let easy = AI_DIFFICULTY.easy;
        let label = format!("{name} game {n}");

        expect_eq(config.seed.as_str(), seed.as_str(), &format!("{label}: seed"))?;
        expect_eq(
            &config.controllers[subject],
            &SeatController::Ai { budget: Some(AI_GATE_BUDGET) },
            &format!("{label}: the subject's controller"),
        )?;
        let opponent = match name.as_str() {
            "ai-vs-random" => SeatController::Random,
            "ai-vs-greedy" => SeatController::Greedy,
            _ => SeatController::Ai { budget: Some(AI_GATE_BUDGET) },
        };
        expect_eq(&config.controllers[other], &opponent, &format!("{label}: the opponent's controller"))?;
        expect_eq(handicap_of(&config, subject), subject_handicap, &format!("{label}: the subject's handicap"))?;
        expect_eq(handicap_of(&config, other), easy, &format!("{label}: the opponent's handicap"))?;

        if hard_vs_easy {
            expect_eq(
                deck_of(&config, subject).len(),
                subject_handicap.deck_size as usize,
                &format!("{label}: the Hard deck's size"),
            )?;
            expect_eq(deck_of(&config, other).len(), easy.deck_size as usize, &format!("{label}: the Easy deck's size"))?;
        }
        expect_eq(
            deck_of(&config, subject),
            &gate_deck(&seed, subject, &subject_handicap),
            &format!("{label}: the subject's deck"),
        )?;
        expect_eq(deck_of(&config, other), &gate_deck(&seed, other, &easy), &format!("{label}: the opponent's deck"))?;
    }
    Ok(())
}

/// `binomialTail` is P(X >= k) for X ~ Binomial(n, p).
fn check_binomial_tail() -> Result<()> {
    let close = |actual: f64, expected: f64, label: &str| {
        expect_that((actual - expected).abs() < CLOSE_TO_12, || {
            format!("binomialTail{label}: expected {expected} within 1e-12, got {actual}")
        })
    };
    close(binomial_tail(2, 0.5, 0), 1.0, "(2, 0.5, 0)")?;
    close(binomial_tail(2, 0.5, 1), 0.75, "(2, 0.5, 1)")?;
    close(binomial_tail(2, 0.5, 2), 0.25, "(2, 0.5, 2)")?;
    close(binomial_tail(3, 0.2, 3), 0.008, "(3, 0.2, 3)")?;
    expect_eq(binomial_tail(20, 0.68, 21), 0.0, "binomialTail(20, 0.68, 21)")?;
    close(binomial_tail(20, 1.0, 20), 1.0, "(20, 1, 20)")?;
    expect_eq(binomial_tail(20, 0.0, 1), 0.0, "binomialTail(20, 0, 1)")?;
    Ok(())
}

/// B28-B30: each gate needs the brief's share of its games, or the count an AI as strong as measured
/// reaches in 1 - falseAlarm of runs of that size, whichever is lower (SPEC §9.9).
fn check_gate_rule() -> Result<()> {
    for matchup in matchups() {
        let measured = AI_GATE.measured_rate[matchup];
        let false_alarm = AI_GATE.false_alarm;
        for games in [smoke_seeds(), full_seeds(matchup)] {
            let needed = needed(matchup, games);
            let label = format!("{}, {games} games: {needed}", matchup_name(matchup));
            // Never more than the brief asks for.
            expect_that(needed <= brief_count(matchup, games), || format!("{label}: more than the brief asks for"))?;
            // An AI exactly as strong as the one measured passes at least 1 - falseAlarm of the time...
            expect_that(binomial_tail(games, measured, needed) >= 1.0 - false_alarm, || {
                format!("{label}: an AI as strong as measured fails more often than falseAlarm")
            })?;
            // ...and one more win would either break that or ask for more than the brief.
            let tighter = binomial_tail(games, measured, needed + 1) < 1.0 - false_alarm;
            expect_that(tighter || needed == brief_count(matchup, games), || {
                format!("{label}: one more win would still pass and stay within the brief")
            })?;
        }
    }
    Ok(())
}

/// B29: the gate plays its own frozen seed series, never the one tuning plays, and deals both seats
/// by one deck rule.
fn check_series(matchup: Matchup) -> Result<()> {
    expect_eq(AI_GATE.seed_series, "gate:v3", "AI_GATE.seedSeries")?;
    expect_that(AI_TUNING_SERIES != AI_GATE.seed_series, || "AI_TUNING_SERIES is the gate's own series".to_string())?;
    expect_eq(
        game_config(matchup, 1, AI_GATE_BUDGET, AI_GATE.seed_series).seed,
        gate_seed(matchup, 1),
        "game 1's seed",
    )?;
    expect_eq(
        game_config(matchup, 1, AI_GATE_BUDGET, AI_TUNING_SERIES).seed,
        format!("{AI_TUNING_SERIES}:{}:1", matchup_name(matchup)),
        "game 1's seed in the tuning series",
    )?;
    // Neither side is dealt a card the AI's shadow ban keeps out of the other (R186).
    let banned: IndexSet<&str> = SHADOW_BAN.iter().map(|(id, _)| *id).collect();
    for n in 1..=SHADOW_BAN_PROBES {
        let config = game_config(matchup, n, AI_GATE_BUDGET, AI_GATE.seed_series);
        for deck in [&config.decks.0, &config.decks.1] {
            for id in deck {
                expect_that(!banned.contains(id.as_str()), || format!("game {n}: {id}"))?;
            }
        }
    }
    Ok(())
}

/// B29: the greedy baseline keeps its own frozen weights, so tuning the AI's never moves it.
///
/// TS proved it by turning AI_EVAL upside down at run time and asking greedy for every decision of a
/// game again. Rust's AI_EVAL and GREEDY_EVAL are constants, which nothing can change while the
/// program runs: that greedy reads GREEDY_EVAL and nothing else is a fact of `baselines.rs`'s source,
/// which no run can contradict. What a run can still prove is kept: the two tables differ, greedy acts
/// in its main phase, and every greedy decision is a function of its state and its rng alone (asked
/// twice, it answers the same).
fn check_greedy_frozen(matchup: Matchup) -> Result<()> {
    expect_that(GREEDY_EVAL != AI_EVAL, || "GREEDY_EVAL is AI_EVAL".to_string())?;
    // Greedy plays both seats here, so the states come quickly.
    let config = game_config(matchup, GREEDY_PROBE_GAME, AI_GATE_BUDGET, AI_GATE.seed_series);
    let greedy_seat = PlayerId::P1;
    expect_eq(&config.controllers[greedy_seat], &SeatController::Greedy, "game 2's p1 controller")?;
    let mut states: Vec<GameState> = Vec::new();
    {
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(|before: &GameState, _after: &GameState, seat: PlayerId, _action: &ActionBody| {
                if seat == greedy_seat && before.pending.is_none() {
                    states.push(before.clone());
                }
            })),
            ..MatchHooks::default()
        };
        let both_greedy = MatchConfig {
            controllers: PerPlayer { p1: SeatController::Greedy, p2: SeatController::Greedy },
            max_actions: Some(GREEDY_PROBE_ACTIONS),
            ..config
        };
        play_match(&both_greedy, &mut hooks);
    }
    expect_that(states.len() > GREEDY_PROBE_MIN_STATES, || {
        format!("greedy acted in its main phase only {} time(s)", states.len())
    })?;
    let decisions = || -> Vec<Option<ActionBody>> {
        states
            .iter()
            .enumerate()
            .map(|(i, state)| greedy_action(state, greedy_seat, &mut Rng::new(&format!("greedy-probe:{i}"), 0)))
            .collect()
    };
    let before = decisions();
    expect_eq(decisions(), before, "greedy's decisions asked twice")?;
    Ok(())
}

/// The win-count check's first half: the run holds one game per number played, each with its seed and
/// subject seat, and `won` is exactly "the result names the subject".
fn check_report(run: &GateReport, played: &[i32]) -> Result<()> {
    let name = matchup_name(run.matchup);
    expect_eq(run.games.len(), played.len(), &format!("{name}: games played"))?;
    for (at, game) in run.games.iter().enumerate() {
        let n = played.get(at).copied().unwrap_or(0);
        expect_eq(game.seed.as_str(), gate_seed(run.matchup, n).as_str(), &format!("{name} game {n}: seed"))?;
        expect_eq(game.subject_seat, subject_seat_of(n), &format!("{name} game {n}: subject seat"))?;
        let subject_won = game.record.result.as_ref().is_some_and(|result| result.winner == Winner::from(game.subject_seat));
        expect_eq(game.won, subject_won, &format!("{name} game {n}: won"))?;
    }
    // Only wins count: a draw at the turn cap is reported beside them and is a game the AI did not close.
    let wins = run.games.iter().filter(|game| game.won).count() as i32;
    expect_eq(run.wins, wins, &format!("{name}: wins"))?;
    let capped = run.games.iter().filter(|game| is_turn_cap_draw(&game.record.result)).count() as i32;
    expect_eq(run.turn_cap_draws, capped, &format!("{name}: turn-cap draws"))?;
    if !played.is_empty() {
        let rate = f64::from(run.wins) / played.len() as f64;
        expect_that((run.rate - rate).abs() < CLOSE_TO_10, || format!("{name}: rate {} is not {rate}", run.rate))?;
    }
    Ok(())
}

/// B31: every game is clean: nothing rejected or thrown, no fallback, a result, and a replay that
/// matches.
fn check_clean(run: &GateReport) -> Result<()> {
    for game in &run.games {
        let label = format!("{} ({})", game.seed, game.subject_seat);
        expect_that(game.record.rejected.is_empty(), || format!("{label}: rejected {:?}", game.record.rejected))?;
        expect_that(game.record.thrown.is_empty(), || format!("{label}: thrown {:?}", game.record.thrown))?;
        expect_eq(game.record.fallbacks as i64, 0, &format!("{label}: fallbacks"))?;
        expect_that(game.record.result.is_some(), || format!("{label}: no result"))?;
        expect_eq(game.replay_errors as i64, 0, &format!("{label}: replay errors"))?;
        expect_eq(game.replay_hash.as_str(), game.record.hash.as_str(), &format!("{label}: replay hash"))?;
    }
    Ok(())
}

/// The checks that read no game, per matchup, in the order its TS gate file ran them.
fn config_checks(matchup: Matchup) -> Vec<Result<()>> {
    match matchup_name(matchup).as_str() {
        "ai-vs-random" => vec![check_game_config(matchup), check_binomial_tail(), check_gate_rule()],
        "ai-vs-greedy" => vec![check_game_config(matchup), check_series(matchup), check_greedy_frozen(matchup)],
        _ => vec![check_game_config(matchup)],
    }
}

/// One matchup's gate file: its checks, then its run of `games` games (or its shard's share), judged
/// against `gateNeeded` unless sharded, and B31 over every game played. Answers the problems found.
fn gate_matchup(matchup: Matchup, full: bool, shard: Option<GateShard>, out: &Path) -> Vec<String> {
    let name = matchup_name(matchup);
    let games = if full { full_seeds(matchup) } else { smoke_seeds() };
    let needed = needed(matchup, games);
    // The games this process plays: all of them, or its shard's when CI splits the run.
    let played = games_to_play(games, shard);
    let shard_label = match shard {
        None => String::new(),
        Some(shard) => format!(", shard {}/{}: {} played", shard.index, shard.count, played.len()),
    };
    println!("gate {name} ({}: {games} games{shard_label})", if full { "full" } else { "smoke" });

    let mut problems: Vec<String> = config_checks(matchup)
        .into_iter()
        .filter_map(|check| check.err().map(|error| format!("{name}: {error:#}")))
        .collect();

    let started = Instant::now();
    let run = play_gate(matchup, &played);
    let seconds = started.elapsed().as_secs_f64();
    if let Err(error) = check_report(&run, &played) {
        problems.push(format!("{name}: {error:#}"));
    }
    if let Err(error) = check_clean(&run) {
        problems.push(format!("{name}: {error:#}"));
    }

    // A shard's wins are a share of the run's, so `gate merge` holds them against NEEDED together with
    // the other shards'. The games' cleanliness is still checked here (B31).
    if let Some(shard) = shard {
        match write_shard(&run, games, shard, &played, out) {
            Ok(path) => println!(
                "[gate {name}{shard_label}] {} wins and {} turn-cap draws, written to {} ({seconds:.1}s)",
                run.wins,
                run.turn_cap_draws,
                path.display()
            ),
            Err(error) => problems.push(format!("{name}: {error:#}")),
        }
        return problems;
    }
    // Written to stdout, so a green gate still shows how it passed.
    println!(
        "[gate {name}] {} wins and {} turn-cap draws of {games}; {needed} wins needed ({seconds:.1}s)",
        run.wins, run.turn_cap_draws
    );
    if run.wins < needed {
        problems.push(format!(
            "{name}: {} wins, {} turn-cap draws, {needed} needed; not won: {}",
            run.wins,
            run.turn_cap_draws,
            losing_seeds(&run)
        ));
    }
    problems
}

// ---------------------------------------------------------------------------
// The perf gate (gate-perf.test.ts)
// ---------------------------------------------------------------------------
//
// Quality gate: a decision at the browser's budget stays under AI_GATE.maxDecisionMs as the
// development machine would time it (docs/polish/3-ai.md "Budgets", SPEC §9.9). Budgets count nodes,
// so what a decision does is fixed and the node budget is asserted exactly; only its speed depends on
// the machine. The states are every decision the Easy AI faced in real gate games against the greedy
// baseline, the opponent's reply included.
//
// The clock is read against a yardstick, not on its own: a wall-clock limit failed whenever the
// machine was busy (1,520 ms in a full `pnpm test` at load 26, 5,523 ms beside an e2e run) and passed
// alone. So each run of a decision is timed right after a fixed piece of engine work
// (`AI_GATE.calibrationGames` random-policy games), and the decision's cost is its time over the
// yardstick's, times the yardstick's time on the development machine (`calibrationRefMs`). Load, or a
// slower CI runner, slows both, and the ratio stands. Each state is decided up to AI_GATE.perfRepeats
// times and its smallest ratio counts, so a burst of load during one run fails nothing, while a
// decision that is slow on its own still does. The runs stop at the first one under
// AI_GATE.maxDecisionMs: a decision fails only when every run is over, so the runs after a passing one
// cannot change the verdict, and they were two thirds of a shard's perf time (#188).
//
// `gate` times the decisions of AI_GATE.perfSmokeGames games; `gate --full` times
// AI_GATE.perfFullGames.
//
// Ordinary games seldom reach the worst case, so two hand-built wide boards are timed as well: five
// units a side and a hand of X-cost and targeted spells beside a Lava Golem, at Hard's seven crystals
// and at Easy's four. There the lethal solver's best-first walk runs to its allowance and every
// candidate list runs to hundreds of entries.

/// One decision's timing.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Timing {
    /// The decision's cost on the development machine: its smallest ratio to the yardstick, in ms.
    ms: f64,
    /// The fastest run as this machine timed it, for the report.
    raw_ms: f64,
    nodes: i64,
    reason: String,
}

/// A decision of a gate game, timed.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Timed {
    game: i32,
    turn: i32,
    #[serde(flatten)]
    timing: Timing,
}

/// Every state the AI seat decided in the ai-vs-greedy gate games `played`, played at AI_BUDGET.
fn decision_states(played: &[i32]) -> Result<Vec<(i32, PlayerId, GameState)>> {
    let greedy = parse_matchup("ai-vs-greedy")?;
    let mut states: Vec<(i32, PlayerId, GameState)> = Vec::new();
    for &n in played {
        let seat = subject_seat_of(n);
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(|before: &GameState, _after: &GameState, actor: PlayerId, _action: &ActionBody| {
                if actor == seat {
                    states.push((n, seat, before.clone()));
                }
            })),
            ..MatchHooks::default()
        };
        play_match(&game_config(greedy, n, AI_BUDGET, AI_GATE.seed_series), &mut hooks);
    }
    Ok(states)
}

/// The wide boards: p1 (the AI) to act on turn 9, no lethal on the board, hundreds of candidates.
const WIDE_HAND: [&str; 6] = ["core-055", "core-024", "core-074", "core-035", "core-031", "core-044"];
const WIDE_FIELD: [&str; 5] = ["core-020", "core-008", "core-011", "core-045", "core-030"];
const WIDE_ENEMY: [&str; 5] = ["core-019", "core-025", "core-037", "core-032", "core-068"];
/// The wide boards' turn, and their crystals at Hard and at Easy.
const WIDE_TURN: i32 = 9;
const WIDE_HARD_MANA: i32 = 7;
const WIDE_EASY_MANA: i32 = 4;
/// The enemy hero's health on the wide boards.
const WIDE_ENEMY_HEALTH: i32 = 30;
/// Wide enough to be the worst case the header describes: 195 candidates at Easy's four crystals,
/// more at Hard's seven. Easy's count was above 250 until task 4's `TargetDecl.forModes` (R90) stopped
/// listing Efficiency Dividend's mana mode once per target it never reads, and 232 until R348 (patch
/// v0.1.1) dropped Efficiency Dividend's and Adaptive UI's X = 0 plays.
const WIDE_MIN_CANDIDATES: usize = 190;

/// The two wide boards' scenario options, by name.
fn wide_boards() -> Vec<(&'static str, Value)> {
    let board = |mana: i32| {
        json!({
            "p1": { "hand": WIDE_HAND, "mana": mana, "field": WIDE_FIELD },
            "p2": { "field": WIDE_ENEMY, "health": WIDE_ENEMY_HEALTH },
        })
    };
    vec![("wide-hard", board(WIDE_HARD_MANA)), ("wide-easy", board(WIDE_EASY_MANA))]
}

/// The wide board `name`'s state: `scenario({ seed: `perf-${name}`, active: "p1", turn: 9, ...setup })`.
fn wide_state(name: &str, setup: &Value) -> GameState {
    let mut options = json!({ "seed": format!("perf-{name}"), "active": "p1", "turn": WIDE_TURN });
    if let (Some(into), Some(from)) = (options.as_object_mut(), setup.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    jackioh_engine::testkit::scenario(options).state().clone()
}

/// The yardstick: a fixed piece of engine work, random-policy games played through `reduce`.
fn yardstick_ms() -> Result<f64> {
    let random = parse_matchup("ai-vs-random")?;
    let started = Instant::now();
    for n in 1..=AI_GATE.calibration_games {
        let config = game_config(random, n, AI_BUDGET, AI_GATE.seed_series);
        let both_random =
            MatchConfig { controllers: PerPlayer { p1: SeatController::Random, p2: SeatController::Random }, ..config };
        play_match(&both_random, &mut MatchHooks::default());
    }
    Ok(started.elapsed().as_secs_f64() * MS_PER_SECOND)
}

/// Up to AI_GATE.perfRepeats runs of one decision, each timed right after the yardstick, stopping at
/// the first under AI_GATE.maxDecisionMs: the smallest ratio of the two, in the development machine's
/// milliseconds, with the node count (the same on every run: budgets count nodes).
fn time_decision(state: &GameState, seat: PlayerId, rng_seed: &str) -> Result<Timing> {
    let reference = AI_GATE.calibration_ref_ms;
    let limit = AI_GATE.max_decision_ms;
    let mut ratio = f64::INFINITY;
    let mut raw_ms = f64::INFINITY;
    let mut nodes: i64 = 0;
    let mut reason = String::new();
    for _ in 0..AI_GATE.perf_repeats {
        let unit = yardstick_ms()?;
        let started = Instant::now();
        let mut options = AiOptions { rng: Rng::new(rng_seed, 0), budget: AI_BUDGET, should_stop: None };
        let decision = decide(state, seat, &mut options);
        let ms = started.elapsed().as_secs_f64() * MS_PER_SECOND;
        ratio = ratio.min(ms / unit);
        raw_ms = raw_ms.min(ms);
        nodes = decision.as_ref().map_or(0, |decision| decision.stats.nodes as i64);
        reason = decision.as_ref().map_or_else(|| "none".to_string(), |decision| literal(&decision.reason));
        if ratio * reference < limit {
            break;
        }
    }
    Ok(Timing { ms: ratio * reference, raw_ms, nodes, reason })
}

/// B42: every decision of the perf games this process times (all of them, or its shard's: every
/// decision is judged on its own, so the shards together time exactly what one run would) stays within
/// the node budget and under AI_GATE.maxDecisionMs, and so does a decision on each wide board (timed in
/// every shard). `judge_time` false checks the node budget and the boards' width but not the clock
/// (the unit tests' unoptimised build). Answers the problems found.
fn perf_gate(full: bool, shard: Option<GateShard>, judge_time: bool) -> Vec<String> {
    let games = if full { AI_GATE.perf_full_games } else { AI_GATE.perf_smoke_games };
    let played = games_to_play(games, shard);
    let limit = AI_GATE.max_decision_ms;
    let budget_nodes = AI_BUDGET.nodes as i64;
    let shard_label = match shard {
        None => String::new(),
        Some(shard) => format!(", shard {}/{}: {} timed", shard.index, shard.count, played.len()),
    };
    println!("gate perf: one decision at AI_BUDGET ({}: {games} game(s){shard_label})", if full { "full" } else { "smoke" });
    let mut problems: Vec<String> = Vec::new();

    // The yardstick's first run warms the caches; it is no measurement.
    if let Err(error) = yardstick_ms() {
        problems.push(format!("perf: {error:#}"));
        return problems;
    }

    // A shard with more shards than games has none of its own; the wide boards below still run.
    if !played.is_empty() {
        let mut timed: Vec<Timed> = Vec::new();
        match decision_states(&played) {
            Ok(states) => {
                for (game, seat, state) in states {
                    match time_decision(&state, seat, &format!("perf:{game}:{}", state.turn)) {
                        Ok(timing) => timed.push(Timed { game, turn: state.turn, timing }),
                        Err(error) => problems.push(format!("perf: {error:#}")),
                    }
                }
            }
            Err(error) => problems.push(format!("perf: {error:#}")),
        }
        if timed.is_empty() {
            problems.push("perf: no decision was timed".to_string());
        }
        for entry in &timed {
            if entry.timing.nodes > budget_nodes {
                problems.push(format!("perf: over the node budget: {}", serde_json::to_string(entry).unwrap_or_default()));
            }
        }
        if let Some(slowest) = timed.iter().reduce(|worst, entry| if entry.timing.ms > worst.timing.ms { entry } else { worst }) {
            let line = serde_json::to_string(slowest).unwrap_or_default();
            println!("[gate perf] {} decision(s); slowest: {line}", timed.len());
            if judge_time && slowest.timing.ms >= limit {
                problems.push(format!("perf: slowest decision: {line}"));
            }
        }
    }

    for (name, setup) in wide_boards() {
        let state = wide_state(name, &setup);
        let candidates = candidate_actions(&state, PlayerId::P1).len();
        if candidates <= WIDE_MIN_CANDIDATES {
            problems.push(format!("perf: {name}: {candidates} candidates, not more than {WIDE_MIN_CANDIDATES}"));
        }
        match time_decision(&state, PlayerId::P1, &format!("perf:{name}")) {
            Ok(timing) => {
                let line = serde_json::to_string(&timing).unwrap_or_default();
                println!("[gate perf] {name}: {line}");
                if timing.nodes > budget_nodes {
                    problems.push(format!("perf: {name}: over the node budget: {line}"));
                }
                if judge_time && timing.ms >= limit {
                    problems.push(format!("perf: {name}: {line}"));
                }
            }
            Err(error) => problems.push(format!("perf: {name}: {error:#}")),
        }
    }
    problems
}

// ---------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------

/// A switch's value from its flag, else its environment variable (the TS gates' only way in).
fn flag_or_env(flag: Option<String>, variable: &str) -> Option<String> {
    flag.or_else(|| std::env::var(variable).ok())
}

/// `cargo jackioh gate …`: the three matchups and the perf gate, or `merge`.
pub fn run(args: Args) -> Result<()> {
    if let Some(GateCommand::Merge { dir }) = args.command {
        return merge(&dir);
    }
    jackioh_cards::register_all();

    let full = args.full || std::env::var("JACKIOH_AI_GATE").is_ok_and(|value| value == "full");
    let shard = gate_shard(flag_or_env(args.shard, "JACKIOH_AI_GATE_SHARD").as_deref())?;
    let out = args
        .out
        .or_else(|| std::env::var("JACKIOH_AI_GATE_OUT").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SHARD_DIR));

    let mut problems: Vec<String> = Vec::new();
    for matchup in matchups() {
        problems.extend(gate_matchup(matchup, full, shard, &out));
    }
    problems.extend(perf_gate(full, shard, true));

    if problems.is_empty() { Ok(()) } else { Err(anyhow!("the gate failed:\n{}", problems.join("\n"))) }
}

#[cfg(test)]
mod tests {
    //! The gates' smoke run: every check that reads no game, and 4 games of each matchup held to the
    //! same rule as the full run (`gateNeeded(matchup, 4)`) and to B31. `cargo jackioh gate` plays the
    //! smoke size (AI_GATE.smokeSeeds) and `--full` the full one.

    use super::*;

    /// The games each matchup's smoke test plays.
    const SMOKE_GAMES: i32 = 4;

    fn smoke(name: &str) {
        jackioh_cards::register_all();
        let matchup = parse_matchup(name).unwrap();
        let played: Vec<i32> = (1..=SMOKE_GAMES).collect();
        let run = play_gate(matchup, &played);
        check_report(&run, &played).unwrap();
        check_clean(&run).unwrap();
        let needed = needed(matchup, SMOKE_GAMES);
        assert!(
            run.wins >= needed,
            "{} wins, {} turn-cap draws, {needed} needed; not won: {}",
            run.wins,
            run.turn_cap_draws,
            losing_seeds(&run)
        );
    }

    #[test]
    fn b28_game_config_seats_the_easy_ai_against_the_random_policy_alternating_seats_with_the_specified_decks() {
        jackioh_cards::register_all();
        check_game_config(parse_matchup("ai-vs-random").unwrap()).unwrap();
    }

    #[test]
    fn binomial_tail_is_p_x_at_least_k_for_x_binomial_n_p() {
        check_binomial_tail().unwrap();
    }

    #[test]
    fn b28_b30_each_gate_needs_the_briefs_share_or_the_count_an_ai_as_strong_as_measured_reaches() {
        check_gate_rule().unwrap();
    }

    #[test]
    fn b28_b31_the_easy_ai_wins_against_the_random_policy_and_every_game_is_clean() {
        smoke("ai-vs-random");
    }

    #[test]
    fn b29_game_config_seats_the_easy_ai_against_the_greedy_baseline_at_equal_resources() {
        jackioh_cards::register_all();
        check_game_config(parse_matchup("ai-vs-greedy").unwrap()).unwrap();
    }

    #[test]
    fn b29_the_gate_plays_its_own_frozen_seed_series_and_deals_both_seats_by_one_deck_rule() {
        jackioh_cards::register_all();
        check_series(parse_matchup("ai-vs-greedy").unwrap()).unwrap();
    }

    #[test]
    fn b29_the_greedy_baseline_keeps_its_own_frozen_weights() {
        jackioh_cards::register_all();
        check_greedy_frozen(parse_matchup("ai-vs-greedy").unwrap()).unwrap();
    }

    #[test]
    fn b29_b31_the_easy_ai_wins_against_the_greedy_baseline_and_every_game_is_clean() {
        smoke("ai-vs-greedy");
    }

    #[test]
    fn r180_b30_game_config_seats_hards_handicap_against_easys_the_same_ai_and_budget_on_both() {
        jackioh_cards::register_all();
        check_game_config(parse_matchup("hard-vs-easy").unwrap()).unwrap();
    }

    #[test]
    fn b30_b31_the_hard_ai_wins_against_itself_on_easy_and_every_game_is_clean() {
        smoke("hard-vs-easy");
    }

    /// The clock is judged only in an optimised build: an unoptimised test binary slows the search and
    /// the yardstick by different factors, so the ratio would not stand. The node budget and the wide
    /// boards' width are judged in every build.
    #[test]
    fn b42_every_decision_stays_within_the_node_budget_and_the_wide_boards_are_wide() {
        jackioh_cards::register_all();
        let problems = perf_gate(false, None, !cfg!(debug_assertions));
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn the_shard_switch_reads_k_of_k_and_refuses_anything_else() {
        assert_eq!(gate_shard(None).unwrap(), None);
        assert_eq!(gate_shard(Some("  ")).unwrap(), None);
        assert_eq!(gate_shard(Some("2/3")).unwrap(), Some(GateShard { index: 2, count: 3 }));
        assert_eq!(gate_shard(Some(" 1/1 ")).unwrap(), Some(GateShard { index: 1, count: 1 }));
        for bad in ["0/3", "4/3", "1/0", "a/b", "1/", "/2", "1-2", "1/2/3", "-1/2"] {
            let error = gate_shard(Some(bad)).unwrap_err().to_string();
            assert!(error.contains("must be \"k/K\""), "{bad}: {error}");
        }
    }

    #[test]
    fn every_shard_plays_every_kth_game_and_the_shards_together_play_each_once() {
        assert_eq!(games_to_play(5, None), vec![1, 2, 3, 4, 5]);
        assert_eq!(games_to_play(10, Some(GateShard { index: 2, count: 3 })), vec![2, 5, 8]);
        let mut all: Vec<i32> = (1..=3).flat_map(|index| games_to_play(10, Some(GateShard { index, count: 3 }))).collect();
        all.sort_unstable();
        assert_eq!(all, (1..=10).collect::<Vec<_>>());
        assert!(games_to_play(1, Some(GateShard { index: 2, count: 2 })).is_empty());
    }

    #[test]
    fn merge_holds_the_shards_together_and_names_a_missing_or_doubled_game() {
        let dir = std::env::temp_dir().join(format!("jackioh-gate-merge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        let shard_of = |name: &str, total: i32, numbers: &[i32], won: bool| ShardFile {
            matchup: name.to_string(),
            total,
            shard: "1/1".to_string(),
            games: numbers
                .iter()
                .map(|&n| ShardGame {
                    n,
                    seed: format!("gate:v3:{name}:{n}"),
                    subject_seat: subject_seat_of(n).to_string(),
                    won,
                    turn_cap_draw: false,
                    result: json!({ "winner": subject_seat_of(n).to_string(), "reason": "hero-death" }),
                })
                .collect(),
        };
        let write = |path: PathBuf, file: &ShardFile| std::fs::write(path, serde_json::to_string_pretty(file).unwrap()).unwrap();
        for matchup in matchups() {
            let name = matchup_name(matchup);
            let total = full_seeds(matchup);
            let all: Vec<i32> = (1..=total).collect();
            let (first, second) = all.split_at(all.len() / 2);
            write(dir.join(format!("{name}-1-of-2.json")), &shard_of(&name, total, first, true));
            write(dir.join("nested").join(format!("{name}-2-of-2.json")), &shard_of(&name, total, second, true));
        }
        merge(&dir).unwrap();

        // A game played twice fails the merge and is named.
        let name = MATCHUP_NAMES[0];
        let total = full_seeds(parse_matchup(name).unwrap());
        write(dir.join(format!("{name}-extra.json")), &shard_of(name, total, &[1], true));
        let error = merge(&dir).unwrap_err().to_string();
        assert!(error.contains("played twice [1]"), "{error}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
