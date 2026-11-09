//! `cargo jackioh perf [--smoke] [--json]` (#3429): the engine's speed and the replay fold's, timed on a
//! frozen corpus of states and each median held to a budget.
//!
//! The corpus is the hotseat fixture's states (its opening and the state after each action of its log),
//! the AI gate's two wide boards (`gate::wide_boards`) and the heaviest states, by the fuzz's growth
//! measures (`fuzz::weigh`, #1254), of the golden games of seeds PERF_SEED_FIRST..=PERF_SEED_LAST.
//! Every state is dealt from seeds and fixed streams, so the corpus is the same on every run of a build.
//! On each state a pass times `legal_actions` and `view_for` for each seat, `hash_state`, and `reduce`
//! of PERF_ACTIONS_PER_SEAT evenly spread listed actions of each seat; then it folds the longest of
//! those games from its log, as a reconnect and `jackioh replay` do, and hashes the result. After
//! PERF_WARMUP_PASSES passes whose samples are dropped, PERF_PASSES passes are timed (one under
//! `--smoke`, which judges no budget). Two work measures need no clock and cannot flake: the bytes of
//! each `view_for` and the `reduce` calls of the fold.
//!
//! Each budget bounds a measure's median (the p95 is shown, not judged), at the profile the binary
//! was built at: `cargo test`'s (`*_TEST`, debug assertions on) or release (`*_RELEASE`). No
//! yardstick divides the clock, as the AI's perf gate's does (`gate.rs`): that yardstick is engine
//! work, so an engine slowdown would slow it as much and hide itself.

use std::fmt::Write as _;
use std::time::Instant;

use anyhow::{Result, anyhow, bail};
use rayon::prelude::*;
use serde::Serialize;

use jackioh_cards::register_all;
use jackioh_engine::replay::{FoldArgs, fold};
use jackioh_engine::rng::Rng;
use jackioh_engine::subsystems::choose_action;
use jackioh_engine::testkit::golden::{HOTSEAT, HOTSEAT_HASH};
use jackioh_engine::{
    Action, CreateGameArgs, CreateGameOptions, GameState, PLAYER_IDS, begin_game, create_game,
    create_recorded_game, hash_state, legal_actions, reduce, view_for,
};

use crate::fuzz::{actor_of, weigh};
use crate::gate::{self, MS_PER_SECOND};
use crate::golden::{STEP_CAP, spec_for_seed};

// The corpus and the passes

/// The first golden seed whose game the corpus plays (§13.1: 1–200 are the plain fuzz's games).
pub const PERF_SEED_FIRST: u32 = 1;
/// The last golden seed whose game the corpus plays. Forty games keep the smoke test to seconds.
pub const PERF_SEED_LAST: u32 = 40;
/// How many of the games' heaviest states each growth measure adds to the corpus.
pub const PERF_HEAVIEST_PER_MEASURE: usize = 5;
/// How many of each seat's listed actions `reduce` is timed on in each state, evenly spread over the
/// list: the wide boards list hundreds, so all of them would make the pass the wide boards' alone.
pub const PERF_ACTIONS_PER_SEAT: usize = 4;
/// The timed passes over the corpus a judged run makes.
pub const PERF_PASSES: usize = 10;
/// The passes before them whose samples are dropped: the first touches cold caches and pages.
pub const PERF_WARMUP_PASSES: usize = 1;

const US_PER_SECOND: f64 = 1_000_000.0;
const MEDIAN: f64 = 0.5;
const P95: f64 = 0.95;

// The budgets, measured on 2026-11-09 on a 4-vCPU GitHub Actions ubuntu runner (AMD EPYC 7763) by
// three runs of each profile: `cargo test -p jackioh-tools --test perf perf_holds_every_budget --
// --ignored` for `cargo test`'s, `target/release/jackioh perf` for release's. Each time budget is
// 5x the largest of its three medians, rounded up to two significant figures, so a runner several
// times slower still passes and a slowdown of that size fails; each work budget is 2x its value,
// rounded up the same way. A budget crossed by a change that meant to cost more is measured again
// the same way.

/// `reduce` of one listed action, in microseconds, at `cargo test`'s profile: medians 180.7, 178.7
/// and 179.4, 5x headroom.
pub const REDUCE_US_BUDGET_TEST: f64 = 910.0;
/// `view_for` of one seat, in microseconds, at `cargo test`'s profile: medians 228.3, 228.9 and
/// 229.3, 5x headroom.
pub const VIEW_FOR_US_BUDGET_TEST: f64 = 1_200.0;
/// `legal_actions` of one seat, in microseconds, at `cargo test`'s profile: medians 12.1, 12.0 and
/// 11.9, 5x headroom.
pub const LEGAL_ACTIONS_US_BUDGET_TEST: f64 = 61.0;
/// `hash_state`, in microseconds, at `cargo test`'s profile: medians 214.2, 212.5 and 215.8, 5x
/// headroom.
pub const HASH_STATE_US_BUDGET_TEST: f64 = 1_100.0;
/// The fold of the longest corpus game and its hash, in milliseconds, at `cargo test`'s profile:
/// medians 242.0, 242.4 and 240.9 (fuzz seed 37, 256 actions), 5x headroom.
pub const FOLD_MS_BUDGET_TEST: f64 = 1_300.0;
/// `reduce` of one listed action, in microseconds, at release: medians 140.0, 137.9 and 136.4, 5x
/// headroom.
pub const REDUCE_US_BUDGET_RELEASE: f64 = 700.0;
/// `view_for` of one seat, in microseconds, at release: medians 177.9, 179.2 and 175.7, 5x headroom.
pub const VIEW_FOR_US_BUDGET_RELEASE: f64 = 900.0;
/// `legal_actions` of one seat, in microseconds, at release: medians 10.1, 10.0 and 10.0, 5x
/// headroom.
pub const LEGAL_ACTIONS_US_BUDGET_RELEASE: f64 = 51.0;
/// `hash_state`, in microseconds, at release: medians 193.2, 192.4 and 190.1, 5x headroom.
pub const HASH_STATE_US_BUDGET_RELEASE: f64 = 970.0;
/// The fold of the longest corpus game and its hash, in milliseconds, at release: medians 199.7,
/// 199.2 and 198.9 (fuzz seed 37, 256 actions), 5x headroom.
pub const FOLD_MS_BUDGET_RELEASE: f64 = 1_000.0;
/// The JSON size of one seat's `view_for`, in bytes, at either profile: a median of 6,976 in all six
/// runs, 2x headroom.
pub const VIEW_FOR_BYTES_BUDGET: f64 = 14_000.0;
/// The `reduce` calls the fold of the longest corpus game makes (its log's length), at either
/// profile: 256 in all six runs (fuzz seed 37), 2x headroom.
pub const FOLD_REDUCE_CALLS_BUDGET: f64 = 520.0;

/// The profile this binary was built at, which picks the time budgets.
const PROFILE: &str = if cfg!(debug_assertions) { "test" } else { "release" };

// The command line

/// `perf [--smoke] [--json]`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Time the corpus once and judge no budget (the per-pull-request smoke test).
    #[arg(long)]
    pub smoke: bool,
    /// Print the report as one JSON object instead of the table.
    #[arg(long)]
    pub json: bool,
}

/// `cargo jackioh perf`: the table (or `--json`'s object) on stdout, exit 1 when a median passes its
/// budget.
pub fn run(args: Args) -> Result<()> {
    register_all();
    let report = measure(args.smoke)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", perf_table(&report));
    }
    if !report.failures.is_empty() {
        bail!(
            "perf: {} budget(s) crossed:\n{}",
            report.failures.len(),
            report.failures.join("\n")
        );
    }
    Ok(())
}

// The measures

/// What a row of the report measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Measure {
    Reduce,
    ViewFor,
    LegalActions,
    HashState,
    Fold,
    ViewForBytes,
    FoldReduceCalls,
}

/// Every measure, in the report's order.
const MEASURES: [Measure; 7] = [
    Measure::Reduce,
    Measure::ViewFor,
    Measure::LegalActions,
    Measure::HashState,
    Measure::Fold,
    Measure::ViewForBytes,
    Measure::FoldReduceCalls,
];

impl Measure {
    fn name(self) -> &'static str {
        match self {
            Measure::Reduce => "reduce",
            Measure::ViewFor => "view_for",
            Measure::LegalActions => "legal_actions",
            Measure::HashState => "hash_state",
            Measure::Fold => "fold",
            Measure::ViewForBytes => "view_for bytes",
            Measure::FoldReduceCalls => "fold reduce calls",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Measure::Reduce | Measure::ViewFor | Measure::LegalActions | Measure::HashState => "us",
            Measure::Fold => "ms",
            Measure::ViewForBytes => "bytes",
            Measure::FoldReduceCalls => "calls",
        }
    }

    /// The measure's budget at this binary's profile, and the constant's name.
    fn budget(self) -> (f64, &'static str) {
        let test = cfg!(debug_assertions);
        match self {
            Measure::Reduce if test => (REDUCE_US_BUDGET_TEST, "REDUCE_US_BUDGET_TEST"),
            Measure::Reduce => (REDUCE_US_BUDGET_RELEASE, "REDUCE_US_BUDGET_RELEASE"),
            Measure::ViewFor if test => (VIEW_FOR_US_BUDGET_TEST, "VIEW_FOR_US_BUDGET_TEST"),
            Measure::ViewFor => (VIEW_FOR_US_BUDGET_RELEASE, "VIEW_FOR_US_BUDGET_RELEASE"),
            Measure::LegalActions if test => (LEGAL_ACTIONS_US_BUDGET_TEST, "LEGAL_ACTIONS_US_BUDGET_TEST"),
            Measure::LegalActions => (LEGAL_ACTIONS_US_BUDGET_RELEASE, "LEGAL_ACTIONS_US_BUDGET_RELEASE"),
            Measure::HashState if test => (HASH_STATE_US_BUDGET_TEST, "HASH_STATE_US_BUDGET_TEST"),
            Measure::HashState => (HASH_STATE_US_BUDGET_RELEASE, "HASH_STATE_US_BUDGET_RELEASE"),
            Measure::Fold if test => (FOLD_MS_BUDGET_TEST, "FOLD_MS_BUDGET_TEST"),
            Measure::Fold => (FOLD_MS_BUDGET_RELEASE, "FOLD_MS_BUDGET_RELEASE"),
            Measure::ViewForBytes => (VIEW_FOR_BYTES_BUDGET, "VIEW_FOR_BYTES_BUDGET"),
            Measure::FoldReduceCalls => (FOLD_REDUCE_CALLS_BUDGET, "FOLD_REDUCE_CALLS_BUDGET"),
        }
    }
}

/// The value at nearest rank `p` (0 < p <= 1) of a sorted, non-empty list.
fn nearest_rank(sorted: &[f64], p: f64) -> f64 {
    let len = sorted.len();
    sorted[((p * len as f64).ceil() as usize).clamp(1, len) - 1]
}

/// `take` indices spread evenly over a list of `len`, the first among them; all of them when the list
/// is no longer.
fn spread(len: usize, take: usize) -> Vec<usize> {
    let take = take.min(len);
    (0..take).map(|i| i * len / take).collect()
}

/// How long `work` takes, in seconds; its result is kept from the optimizer and dropped untimed.
fn seconds<T>(work: impl FnOnce() -> T) -> f64 {
    let started = Instant::now();
    let done = std::hint::black_box(work());
    let took = started.elapsed().as_secs_f64();
    drop(done);
    took
}

// The corpus

/// The hotseat fixture's states: the opening, then the state after each action of its log.
fn hotseat_states() -> Result<Vec<GameState>> {
    let input: FoldArgs = serde_json::from_str(HOTSEAT)?;
    let begun = begin_game(&create_recorded_game(&CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        ..CreateGameOptions::default()
    }));
    if let Some(error) = begun.error {
        bail!("the hotseat fixture does not begin: {error}");
    }
    let mut states = vec![begun.state];
    for (n, action) in input.log.iter().enumerate() {
        let result = reduce(states.last().expect("the opening is in"), action);
        if let Some(error) = result.error {
            bail!("the hotseat fixture's action {n} is refused: {error}");
        }
        states.push(result.state);
    }
    let hash = states.last().map(hash_state).unwrap_or_default();
    if hash != HOTSEAT_HASH {
        bail!("the hotseat fixture folds to {hash}, not {HOTSEAT_HASH} (SURFACE §5.2)");
    }
    Ok(states)
}

/// One golden seed's game, played as the fuzz plays it.
struct Played {
    seed: u32,
    /// What its fold reads: the setup and every action taken.
    input: FoldArgs,
    /// The live game's final hash.
    hash: String,
    /// For each growth measure (`fuzz::weigh`'s order), the first state that weighed the most by it:
    /// the weight, the step it followed (1-based) and the state.
    heaviest: [Option<(i64, usize, GameState)>; 3],
}

/// Golden seed k's game: `golden::record_game`'s loop (nonce `perf-<n>`, stopped at the game's end or
/// STEP_CAP), with no hashing on the way.
fn play_seed(k: u32) -> Result<Played> {
    let spec = spec_for_seed(k)?;
    let args = CreateGameArgs {
        seed: spec.seed.clone(),
        decks: spec.decks.clone(),
        handicaps: spec.handicaps.clone(),
        ..CreateGameArgs::default()
    };
    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        bail!("{}: beginGame refused: {error}", spec.seed);
    }
    let mut state = begun.state;
    let mut policy = Rng::new(&spec.policy, 0);
    // R265: the stream that picks which seat answers first while both mulligans are open.
    let mut order = Rng::new(&spec.order, 0);
    let mut input = FoldArgs {
        seed: spec.seed.clone(),
        decks: spec.decks,
        handicaps: spec.handicaps,
        ..FoldArgs::default()
    };
    let mut heaviest: [Option<(i64, usize, GameState)>; 3] = [None, None, None];

    while state.result.is_none() && input.log.len() < STEP_CAP {
        let n = input.log.len();
        let player = actor_of(&state, &mut order);
        let chosen = choose_action(&state, player, &mut policy).ok_or_else(|| {
            anyhow!(
                "{} step {n}: no legal action for {player} while the game is live (turn {}): R82 auto-ends \
                 a turn with nothing left to do, so this is a stall",
                spec.seed,
                state.turn
            )
        })?;
        let action = Action::new(chosen, player, format!("perf-{n}"));
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            bail!(
                "{} step {n}: legalActions offered \"{}\" but reduce refused it: {error}",
                spec.seed,
                action.action_type()
            );
        }
        input.log.push(action);
        for (kept, value) in heaviest.iter_mut().zip(weigh(&result.events, &result.state)) {
            if kept.as_ref().is_none_or(|(most, _, _)| value > *most) {
                *kept = Some((value, n + 1, result.state.clone()));
            }
        }
        state = result.state;
    }

    Ok(Played {
        seed: k,
        input,
        hash: hash_state(&state),
        heaviest,
    })
}

/// The states every pass walks, and the game the fold replays.
struct Corpus {
    states: Vec<GameState>,
    hotseat: usize,
    wide: usize,
    heaviest: usize,
    fold: FoldArgs,
    fold_seed: u32,
    fold_hash: String,
}

fn corpus() -> Result<Corpus> {
    let mut states = hotseat_states()?;
    let hotseat = states.len();
    let wide_boards = gate::wide_boards();
    let wide = wide_boards.len();
    states.extend(
        wide_boards
            .iter()
            .map(|(name, setup)| gate::wide_state(name, setup)),
    );

    let games: Vec<Played> = (PERF_SEED_FIRST..=PERF_SEED_LAST)
        .into_par_iter()
        .map(play_seed)
        .collect::<Result<Vec<_>>>()?;

    let mut taken: Vec<(u32, usize)> = Vec::new();
    for measure in 0..3 {
        let mut candidates: Vec<(i64, u32, usize, &GameState)> = games
            .iter()
            .filter_map(|game| {
                game.heaviest[measure]
                    .as_ref()
                    .map(|(value, step, state)| (*value, game.seed, *step, state))
            })
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut added = 0;
        for (_, seed, step, state) in candidates {
            if added == PERF_HEAVIEST_PER_MEASURE {
                break;
            }
            if taken.contains(&(seed, step)) {
                continue;
            }
            taken.push((seed, step));
            states.push(state.clone());
            added += 1;
        }
    }

    // The longest game, the lower seed on a tie.
    let longest = games
        .iter()
        .reduce(|best, game| {
            if game.input.log.len() > best.input.log.len() {
                game
            } else {
                best
            }
        })
        .ok_or_else(|| anyhow!("no corpus game was played"))?;

    Ok(Corpus {
        states,
        hotseat,
        wide,
        heaviest: taken.len(),
        fold: longest.input.clone(),
        fold_seed: longest.seed,
        fold_hash: longest.hash.clone(),
    })
}

// The passes

/// One pass's samples of the timed measures.
#[derive(Default)]
struct Samples {
    reduce: Vec<f64>,
    view_for: Vec<f64>,
    legal_actions: Vec<f64>,
    hash_state: Vec<f64>,
    fold: Vec<f64>,
}

impl Samples {
    fn extend(&mut self, pass: Samples) {
        self.reduce.extend(pass.reduce);
        self.view_for.extend(pass.view_for);
        self.legal_actions.extend(pass.legal_actions);
        self.hash_state.extend(pass.hash_state);
        self.fold.extend(pass.fold);
    }
}

/// One pass: every state in order (each seat's `legal_actions` then `view_for`, then `hash_state`, then
/// `reduce` of each of `kept`'s actions for that state), then the fold and its hash.
fn timed_pass(corpus: &Corpus, kept: &[Vec<Action>]) -> Result<Samples> {
    let mut samples = Samples::default();
    for (state, actions) in corpus.states.iter().zip(kept) {
        for seat in PLAYER_IDS {
            samples
                .legal_actions
                .push(seconds(|| legal_actions(state, seat)) * US_PER_SECOND);
            samples
                .view_for
                .push(seconds(|| view_for(state, seat)) * US_PER_SECOND);
        }
        samples
            .hash_state
            .push(seconds(|| hash_state(state)) * US_PER_SECOND);
        for action in actions {
            samples
                .reduce
                .push(seconds(|| reduce(state, action)) * US_PER_SECOND);
        }
    }

    let started = Instant::now();
    let folded = std::hint::black_box(fold(&corpus.fold));
    let hash = std::hint::black_box(hash_state(&folded.state));
    samples.fold.push(started.elapsed().as_secs_f64() * MS_PER_SECOND);
    if let Some(first) = folded.errors.first() {
        bail!(
            "the fold of fuzz seed {} refused {} action(s); first at nonce \"{}\": {}",
            corpus.fold_seed,
            folded.errors.len(),
            first.nonce,
            first.error
        );
    }
    if hash != corpus.fold_hash {
        bail!(
            "the fold of fuzz seed {} reaches {hash}, not the live game's {}",
            corpus.fold_seed,
            corpus.fold_hash
        );
    }
    Ok(samples)
}

/// One measure's row of the report.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    measure: &'static str,
    unit: &'static str,
    samples: usize,
    median: f64,
    p95: f64,
    budget: f64,
    #[serde(skip)]
    budget_name: &'static str,
}

fn row(measure: Measure, mut samples: Vec<f64>) -> Result<Row> {
    if samples.is_empty() {
        bail!("perf: no sample of {}", measure.name());
    }
    samples.sort_by(f64::total_cmp);
    let (budget, budget_name) = measure.budget();
    Ok(Row {
        measure: measure.name(),
        unit: measure.unit(),
        samples: samples.len(),
        median: nearest_rank(&samples, MEDIAN),
        p95: nearest_rank(&samples, P95),
        budget,
        budget_name,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CorpusCounts {
    states: usize,
    hotseat: usize,
    wide: usize,
    heaviest: usize,
    /// The golden seeds whose games were played, first and last.
    seeds: [u32; 2],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FoldInfo {
    /// The golden seed of the game folded.
    seed: u32,
    actions: usize,
    passes: usize,
}

/// What `perf` measured, as the table prints it and `--json` emits it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    profile: &'static str,
    smoke: bool,
    passes: usize,
    corpus: CorpusCounts,
    fold: FoldInfo,
    measures: Vec<Row>,
    /// One message per median past its budget; none under `--smoke`.
    failures: Vec<String>,
}

fn measure(smoke: bool) -> Result<Report> {
    let corpus = corpus()?;
    let passes = if smoke { 1 } else { PERF_PASSES };
    let warmups = if smoke { 0 } else { PERF_WARMUP_PASSES };

    let kept: Vec<Vec<Action>> = corpus
        .states
        .iter()
        .map(|state| {
            PLAYER_IDS
                .into_iter()
                .flat_map(|seat| {
                    let listed = legal_actions(state, seat);
                    spread(listed.len(), PERF_ACTIONS_PER_SEAT)
                        .into_iter()
                        .map(|i| Action::new(listed[i].clone(), seat, format!("perf-timed-{i}")))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .collect();

    for _ in 0..warmups {
        timed_pass(&corpus, &kept)?;
    }
    let mut samples = Samples::default();
    for _ in 0..passes {
        samples.extend(timed_pass(&corpus, &kept)?);
    }

    let mut bytes = Vec::new();
    for state in &corpus.states {
        for seat in PLAYER_IDS {
            bytes.push(serde_json::to_string(&view_for(state, seat))?.len() as f64);
        }
    }

    let measures = MEASURES
        .into_iter()
        .map(|measure| {
            let taken = match measure {
                Measure::Reduce => std::mem::take(&mut samples.reduce),
                Measure::ViewFor => std::mem::take(&mut samples.view_for),
                Measure::LegalActions => std::mem::take(&mut samples.legal_actions),
                Measure::HashState => std::mem::take(&mut samples.hash_state),
                Measure::Fold => std::mem::take(&mut samples.fold),
                Measure::ViewForBytes => std::mem::take(&mut bytes),
                Measure::FoldReduceCalls => vec![corpus.fold.log.len() as f64],
            };
            row(measure, taken)
        })
        .collect::<Result<Vec<_>>>()?;
    let failures = if smoke { Vec::new() } else { judge(&measures) };
    Ok(Report {
        profile: PROFILE,
        smoke,
        passes,
        corpus: CorpusCounts {
            states: corpus.states.len(),
            hotseat: corpus.hotseat,
            wide: corpus.wide,
            heaviest: corpus.heaviest,
            seeds: [PERF_SEED_FIRST, PERF_SEED_LAST],
        },
        fold: FoldInfo {
            seed: corpus.fold_seed,
            actions: corpus.fold.log.len(),
            passes,
        },
        measures,
        failures,
    })
}

// The verdict and the table

/// One message per row whose median is past its budget, naming the measure, the median, the p95 and
/// the budget's value and constant.
fn judge(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.median > row.budget)
        .map(|row| {
            let Row {
                measure,
                unit,
                median,
                p95,
                budget,
                budget_name,
                ..
            } = row;
            format!(
                "{measure}: median {median:.1} {unit}, p95 {p95:.1} {unit}, over its budget of {budget:.1} {unit} ({budget_name})"
            )
        })
        .collect()
}

/// The report as one table: a line saying what was measured, then measure, unit, median, p95 and budget.
fn perf_table(report: &Report) -> String {
    let corpus = &report.corpus;
    let mut out = format!(
        "perf: {} profile, {} timed pass(es) over {} states ({} hotseat, {} wide, {} heaviest of fuzz seeds {}-{}); fold: fuzz seed {}, {} actions",
        report.profile,
        report.passes,
        corpus.states,
        corpus.hotseat,
        corpus.wide,
        corpus.heaviest,
        corpus.seeds[0],
        corpus.seeds[1],
        report.fold.seed,
        report.fold.actions
    );
    if report.smoke {
        out.push_str("; smoke: no budget judged");
    }
    out.push('\n');
    let _ = writeln!(
        out,
        "{:<18} {:<5} {:>12} {:>12} {:>12}",
        "measure", "unit", "median", "p95", "budget"
    );
    for row in &report.measures {
        let _ = writeln!(
            out,
            "{:<18} {:<5} {:>12.1} {:>12.1} {:>12.1}",
            row.measure, row.unit, row.median, row.p95, row.budget
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_at(measure: Measure, value: f64) -> Row {
        let (budget, budget_name) = measure.budget();
        Row {
            measure: measure.name(),
            unit: measure.unit(),
            samples: 1,
            median: value,
            p95: value,
            budget,
            budget_name,
        }
    }

    #[test]
    fn nearest_rank_reads_the_median_and_the_p95() {
        let twenty: Vec<f64> = (1..=20).map(f64::from).collect();
        assert_eq!(nearest_rank(&twenty, MEDIAN), 10.0);
        assert_eq!(nearest_rank(&twenty, P95), 19.0);
        assert_eq!(nearest_rank(&[5.0], MEDIAN), 5.0);
        assert_eq!(nearest_rank(&[5.0], P95), 5.0);
    }

    #[test]
    fn spread_picks_evenly_spaced_actions() {
        assert_eq!(spread(10, 4), [0, 2, 5, 7]);
        assert_eq!(spread(3, 4), [0, 1, 2]);
        assert!(spread(0, 4).is_empty());
    }

    #[test]
    fn every_measure_over_its_budget_fails_naming_its_median_p95_and_budget() {
        let rows: Vec<Row> = MEASURES
            .iter()
            .map(|&measure| row_at(measure, 2.0 * measure.budget().0))
            .collect();
        let failures = judge(&rows);
        assert_eq!(failures.len(), MEASURES.len(), "{failures:#?}");
        for (message, measure) in failures.iter().zip(MEASURES) {
            let (budget, name) = measure.budget();
            for needed in [
                format!("{}:", measure.name()),
                format!("median {:.1} {}", 2.0 * budget, measure.unit()),
                format!("p95 {:.1} {}", 2.0 * budget, measure.unit()),
                format!("budget of {budget:.1} {}", measure.unit()),
                name.to_string(),
            ] {
                assert!(message.contains(&needed), "{message:?} lacks {needed:?}");
            }
        }
    }

    #[test]
    fn a_measure_at_its_budget_passes() {
        let rows: Vec<Row> = MEASURES
            .iter()
            .map(|&measure| row_at(measure, measure.budget().0))
            .collect();
        assert_eq!(judge(&rows), Vec::<String>::new());
    }

    #[test]
    fn the_table_has_a_line_per_measure_under_its_header() {
        let report = Report {
            profile: PROFILE,
            smoke: true,
            passes: 1,
            corpus: CorpusCounts {
                states: 3,
                hotseat: 1,
                wide: 1,
                heaviest: 1,
                seeds: [PERF_SEED_FIRST, PERF_SEED_LAST],
            },
            fold: FoldInfo {
                seed: 7,
                actions: 12,
                passes: 1,
            },
            measures: MEASURES.iter().map(|&measure| row_at(measure, 1.5)).collect(),
            failures: Vec::new(),
        };
        let table = perf_table(&report);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 2 + MEASURES.len(), "{table}");
        assert!(lines[0].starts_with("perf: "), "{table}");
        assert!(lines[0].ends_with("; smoke: no budget judged"), "{table}");
        assert!(lines[0].contains("fuzz seed 7, 12 actions"), "{table}");
        let header: Vec<&str> = lines[1].split_whitespace().collect();
        assert_eq!(header, ["measure", "unit", "median", "p95", "budget"]);
        for (line, measure) in lines[2..].iter().zip(MEASURES) {
            assert!(line.starts_with(measure.name()), "{line}");
            let numbers: Vec<&str> = line.split_whitespace().rev().take(3).collect();
            assert_eq!(numbers[1..], ["1.5", "1.5"], "{line}");
            assert!(line.contains(&format!(" {} ", measure.unit())), "{line}");
        }
    }
}
