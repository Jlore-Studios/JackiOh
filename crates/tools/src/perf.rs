//! `cargo jackioh perf [--smoke] [--json]` (#3429): the engine's speed and the replay fold's, timed on a
//! frozen corpus of states and each median held to a budget.
//!
//! The corpus is the hotseat fixture's states (its opening and the state after each action of its log),
//! the AI gate's two wide boards (`gate::wide_boards`), the heaviest states by the fuzz's growth
//! measures (`fuzz::weigh`, #1254) of plain golden seeds PERF_SEED_FIRST..=PERF_SEED_LAST and handicapped
//! golden seeds PERF_HANDICAP_SEED_FIRST..=PERF_HANDICAP_SEED_LAST, one run-start game (§15), and one
//! seeded Duos co-op game (§13). Every state is dealt from seeds and fixed streams, so the corpus is the
//! same on every run of a build. On each 1v1 state a pass times `legal_actions`, `view_for` and
//! `combat_forecast` for each seat, `hash_state`, and `reduce` of PERF_ACTIONS_PER_SEAT evenly spread
//! listed actions of each seat; on each Duos state it times `duos_legal_actions` and `duos_view_for` for
//! all four seats, `combat_forecast` on each seat's underlying duel, and `duos_reduce` of
//! PERF_ACTIONS_PER_SEAT evenly spread listed actions; then it folds the longest 1v1 game and the Duos
//! game from their logs and hashes the results. After PERF_WARMUP_PASSES passes whose samples are
//! dropped, PERF_PASSES passes are timed (one under `--smoke`). One work measure needs no clock and
//! cannot flake: the bytes of each `view_for`. `--smoke` judges only its budget, so every pull request
//! holds it (#4196), and leaves the eleven timings to the daily run: one pass on a shared runner is too
//! noisy to judge a time.
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
use jackioh_engine::subsystems::{AI_SKIPPED_ACTIONS, choose_action};
use jackioh_engine::testkit::golden::{HOTSEAT, HOTSEAT_HASH};
use jackioh_engine::{
    Action, CreateGameArgs, CreateGameOptions, DUOS_SEATS, DeckMark, DuosAction, DuosActionBody,
    DuosCreateOptions, DuosReplayInput, DuosState, GameState, HUMAN_HANDICAP, PLAYER_IDS, PerPlayerOpt,
    begin_game, combat_forecast, create_game, create_recorded_game, duos_begin, duos_create, duos_fold,
    duos_hash, duos_legal_actions, duos_reduce, duos_seats_to_act, duos_view_for, hash_state, legal_actions,
    reduce, view_for,
};

use crate::duos_fuzz::duos_decks_for_seed_from;
use crate::fuzz::{SHIPPED_FUZZ_POOL, actor_of, decks_for_seed_from, weigh, weigh_duos};
use crate::gate;
use crate::golden::{STEP_CAP, spec_for_seed};

// The corpus and the passes

/// The first golden seed whose game the corpus plays (§13.1: 1–200 are the plain fuzz's games).
pub const PERF_SEED_FIRST: u32 = 1;
/// The last golden seed whose game the corpus plays. Forty games keep the smoke test to seconds.
pub const PERF_SEED_LAST: u32 = 40;
/// The first golden seed whose game the handicapped wave sample plays (§13.1: 201–240 are handicapped).
pub const PERF_HANDICAP_SEED_FIRST: u32 = 201;
/// Ten seeds keep the handicapped sample bounded.
pub const PERF_HANDICAP_SEED_LAST: u32 = 210;
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

// The time budgets, measured on 2026-11-22 on 4-vCPU GitHub Actions ubuntu runners by twelve runs of
// each profile: `cargo test -p jackioh-tools --test perf perf_holds_every_budget -- --ignored` for
// `cargo test`'s, `target/release/jackioh perf` for release's. Six ran on #4501's branch, over main's
// corpus at v0.6.9l (99a9c84d), and six on main at 98bb1823, after v0.6.9m, v0.6.9n and #4465: a card
// change deals the corpus's games again, and between those two corpora the 1v1 medians moved by up to
// 1.8x and the Duos ones, three states of one co-op game, by up to 5.6x. The runners' CPUs (Intel Xeon
// 6975P-C, 8370C and 8573C, AMD EPYC 7763 and 9V74) moved them by up to 1.3x more. Each time budget is
// 5x the largest of its twelve medians, rounded up to two significant figures, so a runner several
// times slower still passes and a slowdown of that size fails; each work budget is 2x its value,
// rounded up the same way. A budget crossed by a change that meant to cost more, or by a card change
// that made the corpus heavier, is measured again the same way.

/// `reduce` of one listed action, in microseconds, at `cargo test`'s profile: twelve medians from 185.6 to
/// 298.3, 5x headroom.
pub const REDUCE_US_BUDGET_TEST: f64 = 1_500.0;
/// `view_for` of one seat, in microseconds, at `cargo test`'s profile: twelve medians from 268.7 to 438.5, 5x
/// headroom.
pub const VIEW_FOR_US_BUDGET_TEST: f64 = 2_200.0;
/// `combat_forecast` of one seat, in microseconds, at `cargo test`'s profile: twelve medians from 7.3 to
/// 11.7, 5x headroom.
pub const COMBAT_FORECAST_US_BUDGET_TEST: f64 = 59.0;
/// `legal_actions` of one seat, in microseconds, at `cargo test`'s profile: twelve medians from 9.3 to 13.5,
/// 5x headroom.
pub const LEGAL_ACTIONS_US_BUDGET_TEST: f64 = 68.0;
/// `hash_state`, in microseconds, at `cargo test`'s profile: twelve medians from 176.9 to 236.3, 5x headroom.
pub const HASH_STATE_US_BUDGET_TEST: f64 = 1_200.0;
/// The fold of the longest corpus game and its hash, per folded action, in microseconds per action, at
/// `cargo test`'s profile: twelve medians from 1002.0 to 1533.6, 5x headroom.
pub const FOLD_US_PER_ACTION_BUDGET_TEST: f64 = 7_700.0;
/// `reduce` of one listed action, in microseconds, at release: twelve medians from 233.7 to 252.3, 5x
/// headroom.
pub const REDUCE_US_BUDGET_RELEASE: f64 = 1_300.0;
/// `view_for` of one seat, in microseconds, at release: twelve medians from 311.3 to 349.1, 5x headroom.
pub const VIEW_FOR_US_BUDGET_RELEASE: f64 = 1_800.0;
/// `combat_forecast` of one seat, in microseconds, at release: twelve medians from 7.7 to 10.2, 5x headroom.
pub const COMBAT_FORECAST_US_BUDGET_RELEASE: f64 = 51.0;
/// `legal_actions` of one seat, in microseconds, at release: twelve medians from 10.0 to 12.4, 5x headroom.
pub const LEGAL_ACTIONS_US_BUDGET_RELEASE: f64 = 62.0;
/// `hash_state`, in microseconds, at release: twelve medians from 203.2 to 212.4, 5x headroom.
pub const HASH_STATE_US_BUDGET_RELEASE: f64 = 1_100.0;
/// The fold of the longest corpus game and its hash, per folded action, in microseconds per action, at
/// release: twelve medians from 916.5 to 1677.4, 5x headroom.
pub const FOLD_US_PER_ACTION_BUDGET_RELEASE: f64 = 8_400.0;
/// `duos_reduce` of one listed action, in microseconds, at `cargo test`'s profile: twelve medians from 289.1
/// to 1089.8, 5x headroom.
pub const DUOS_REDUCE_US_BUDGET_TEST: f64 = 5_500.0;
/// `duos_reduce` of one listed action, in microseconds, at release: twelve medians from 256.9 to 1209.7, 5x
/// headroom.
pub const DUOS_REDUCE_US_BUDGET_RELEASE: f64 = 6_100.0;
/// `duos_view_for` of one seat, in microseconds, at `cargo test`'s profile: twelve medians from 2037.9 to
/// 2729.7, 5x headroom.
pub const DUOS_VIEW_FOR_US_BUDGET_TEST: f64 = 14_000.0;
/// `combat_forecast` of one Duos seat's underlying duel, in microseconds, at `cargo test`'s profile: twelve
/// medians from 11.1 to 33.7, 5x headroom.
pub const DUOS_COMBAT_FORECAST_US_BUDGET_TEST: f64 = 170.0;
/// `duos_view_for` of one seat, in microseconds, at release: twelve medians from 2214.0 to 2568.7, 5x
/// headroom.
pub const DUOS_VIEW_FOR_US_BUDGET_RELEASE: f64 = 13_000.0;
/// `combat_forecast` of one Duos seat's underlying duel, in microseconds, at release: twelve medians from 9.0
/// to 25.9, 5x headroom.
pub const DUOS_COMBAT_FORECAST_US_BUDGET_RELEASE: f64 = 130.0;
/// `duos_legal_actions` of one seat, in microseconds, at `cargo test`'s profile: twelve medians from 6.8 to
/// 24.9, 5x headroom.
pub const DUOS_LEGAL_ACTIONS_US_BUDGET_TEST: f64 = 130.0;
/// `duos_legal_actions` of one seat, in microseconds, at release: twelve medians from 5.8 to 33.8, 5x
/// headroom.
pub const DUOS_LEGAL_ACTIONS_US_BUDGET_RELEASE: f64 = 170.0;
/// The fold of the Duos co-op game, per folded action, in microseconds per action, at `cargo test`'s profile:
/// twelve medians from 660.5 to 866.9, 5x headroom.
pub const DUOS_FOLD_US_PER_ACTION_BUDGET_TEST: f64 = 4_400.0;
/// The fold of the Duos co-op game, per folded action, in microseconds per action, at release: twelve medians
/// from 739.6 to 772.8, 5x headroom.
pub const DUOS_FOLD_US_PER_ACTION_BUDGET_RELEASE: f64 = 3_900.0;
/// The JSON size of one seat's `view_for`, in bytes, at either profile: a median of 7,381 over v0.6.9l's
/// corpus and 7,386 over 98bb1823's, in every run, 2x headroom.
pub const VIEW_FOR_BYTES_BUDGET: f64 = 15_000.0;

/// The profile this binary was built at, which picks the time budgets.
const PROFILE: &str = if cfg!(debug_assertions) { "test" } else { "release" };

// The command line

/// `perf [--smoke] [--json]`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Time the corpus once and judge only the work budget (the per-pull-request smoke test).
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
    CombatForecast,
    LegalActions,
    HashState,
    Fold,
    DuosReduce,
    DuosViewFor,
    DuosCombatForecast,
    DuosLegalActions,
    DuosFold,
    ViewForBytes,
}

/// Every measure, in the report's order.
const MEASURES: [Measure; 12] = [
    Measure::Reduce,
    Measure::ViewFor,
    Measure::CombatForecast,
    Measure::LegalActions,
    Measure::HashState,
    Measure::Fold,
    Measure::DuosReduce,
    Measure::DuosViewFor,
    Measure::DuosCombatForecast,
    Measure::DuosLegalActions,
    Measure::DuosFold,
    Measure::ViewForBytes,
];

impl Measure {
    fn name(self) -> &'static str {
        match self {
            Measure::Reduce => "reduce",
            Measure::ViewFor => "view_for",
            Measure::CombatForecast => "combat_forecast",
            Measure::LegalActions => "legal_actions",
            Measure::HashState => "hash_state",
            Measure::Fold => "fold",
            Measure::DuosReduce => "duos_reduce",
            Measure::DuosViewFor => "duos_view_for",
            Measure::DuosCombatForecast => "duos_combat_forecast",
            Measure::DuosLegalActions => "duos_legal_actions",
            Measure::DuosFold => "duos_fold",
            Measure::ViewForBytes => "view_for bytes",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Measure::Reduce
            | Measure::ViewFor
            | Measure::CombatForecast
            | Measure::LegalActions
            | Measure::HashState
            | Measure::DuosReduce
            | Measure::DuosViewFor
            | Measure::DuosCombatForecast
            | Measure::DuosLegalActions => "us",
            Measure::Fold | Measure::DuosFold => "us/action",
            Measure::ViewForBytes => "bytes",
        }
    }

    /// Whether the measure is a time, which only a judged run's passes are enough to judge; the work
    /// measure comes out the same on every run, so `--smoke` judges it too.
    fn needs_clock(self) -> bool {
        match self {
            Measure::Reduce
            | Measure::ViewFor
            | Measure::CombatForecast
            | Measure::LegalActions
            | Measure::HashState
            | Measure::Fold
            | Measure::DuosReduce
            | Measure::DuosViewFor
            | Measure::DuosCombatForecast
            | Measure::DuosLegalActions
            | Measure::DuosFold => true,
            Measure::ViewForBytes => false,
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
            Measure::CombatForecast if test => {
                (COMBAT_FORECAST_US_BUDGET_TEST, "COMBAT_FORECAST_US_BUDGET_TEST")
            }
            Measure::CombatForecast => (
                COMBAT_FORECAST_US_BUDGET_RELEASE,
                "COMBAT_FORECAST_US_BUDGET_RELEASE",
            ),
            Measure::LegalActions if test => (LEGAL_ACTIONS_US_BUDGET_TEST, "LEGAL_ACTIONS_US_BUDGET_TEST"),
            Measure::LegalActions => (LEGAL_ACTIONS_US_BUDGET_RELEASE, "LEGAL_ACTIONS_US_BUDGET_RELEASE"),
            Measure::HashState if test => (HASH_STATE_US_BUDGET_TEST, "HASH_STATE_US_BUDGET_TEST"),
            Measure::HashState => (HASH_STATE_US_BUDGET_RELEASE, "HASH_STATE_US_BUDGET_RELEASE"),
            Measure::Fold if test => (FOLD_US_PER_ACTION_BUDGET_TEST, "FOLD_US_PER_ACTION_BUDGET_TEST"),
            Measure::Fold => (
                FOLD_US_PER_ACTION_BUDGET_RELEASE,
                "FOLD_US_PER_ACTION_BUDGET_RELEASE",
            ),
            Measure::DuosReduce if test => (DUOS_REDUCE_US_BUDGET_TEST, "DUOS_REDUCE_US_BUDGET_TEST"),
            Measure::DuosReduce => (DUOS_REDUCE_US_BUDGET_RELEASE, "DUOS_REDUCE_US_BUDGET_RELEASE"),
            Measure::DuosViewFor if test => (DUOS_VIEW_FOR_US_BUDGET_TEST, "DUOS_VIEW_FOR_US_BUDGET_TEST"),
            Measure::DuosViewFor => (DUOS_VIEW_FOR_US_BUDGET_RELEASE, "DUOS_VIEW_FOR_US_BUDGET_RELEASE"),
            Measure::DuosCombatForecast if test => (
                DUOS_COMBAT_FORECAST_US_BUDGET_TEST,
                "DUOS_COMBAT_FORECAST_US_BUDGET_TEST",
            ),
            Measure::DuosCombatForecast => (
                DUOS_COMBAT_FORECAST_US_BUDGET_RELEASE,
                "DUOS_COMBAT_FORECAST_US_BUDGET_RELEASE",
            ),
            Measure::DuosLegalActions if test => (
                DUOS_LEGAL_ACTIONS_US_BUDGET_TEST,
                "DUOS_LEGAL_ACTIONS_US_BUDGET_TEST",
            ),
            Measure::DuosLegalActions => (
                DUOS_LEGAL_ACTIONS_US_BUDGET_RELEASE,
                "DUOS_LEGAL_ACTIONS_US_BUDGET_RELEASE",
            ),
            Measure::DuosFold if test => (
                DUOS_FOLD_US_PER_ACTION_BUDGET_TEST,
                "DUOS_FOLD_US_PER_ACTION_BUDGET_TEST",
            ),
            Measure::DuosFold => (
                DUOS_FOLD_US_PER_ACTION_BUDGET_RELEASE,
                "DUOS_FOLD_US_PER_ACTION_BUDGET_RELEASE",
            ),
            Measure::ViewForBytes => (VIEW_FOR_BYTES_BUDGET, "VIEW_FOR_BYTES_BUDGET"),
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

/// How long `work` takes, in seconds, and its result, kept from the optimizer.
fn seconds_with_result<T>(work: impl FnOnce() -> T) -> (f64, T) {
    let started = Instant::now();
    let done = std::hint::black_box(work());
    let took = started.elapsed().as_secs_f64();
    (took, done)
}

/// How long `work` takes, in seconds; its result is dropped untimed.
fn seconds<T>(work: impl FnOnce() -> T) -> f64 {
    let (took, done) = seconds_with_result(work);
    drop(done);
    took
}

/// The fold's time per folded action, in microseconds per action.
fn fold_us_per_action(elapsed_secs: f64, actions: usize) -> f64 {
    elapsed_secs * US_PER_SECOND / actions.max(1) as f64
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

/// A run-start game (§15): decks from fuzz pool, hero health, armor, extra opening cards, mana bonus,
/// and deck marks deal Radiant cards with upgrades.
fn play_run_start() -> Result<Played> {
    let decks = decks_for_seed_from(1, &SHIPPED_FUZZ_POOL);
    let mut handicaps = PerPlayerOpt::default();
    let mut marks = PerPlayerOpt::default();
    for player in PLAYER_IDS {
        let mut handicap = HUMAN_HANDICAP;
        handicap.hero_health = Some(35);
        handicap.hero_armor = Some(5);
        handicap.extra_opening_cards = 1;
        handicap.mana_bonus = 1;
        *handicaps.slot(player) = Some(handicap);
        *marks.slot(player) = Some(vec![DeckMark {
            index: 0,
            radiant: true,
            upgrades: 1,
        }]);
    }
    let options = CreateGameOptions {
        seed: "perf-run-start".to_string(),
        decks: decks.clone(),
        handicaps: Some(handicaps.clone()),
        deck_marks: Some(marks.clone()),
        ..CreateGameOptions::default()
    };
    let begun = begin_game(&create_recorded_game(&options));
    if let Some(error) = begun.error {
        bail!("perf-run-start: beginGame refused: {error}");
    }
    let mut state = begun.state;
    let mut policy = Rng::new("perf-run-start-policy", 0);
    let mut order = Rng::new("perf-run-start-order", 0);
    let mut input = FoldArgs {
        seed: "perf-run-start".to_string(),
        decks,
        handicaps: Some(handicaps),
        deck_marks: Some(marks),
        ..FoldArgs::default()
    };
    let mut heaviest: [Option<(i64, usize, GameState)>; 3] = [None, None, None];

    while state.result.is_none() && input.log.len() < STEP_CAP {
        let n = input.log.len();
        let player = actor_of(&state, &mut order);
        let chosen = choose_action(&state, player, &mut policy).ok_or_else(|| {
            anyhow!(
                "perf-run-start step {n}: no legal action for {player} while the game is live (turn {}): R82 auto-ends \
                 a turn with nothing left to do, so this is a stall",
                state.turn
            )
        })?;
        let action = Action::new(chosen, player, format!("perf-run-start-{n}"));
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            bail!(
                "perf-run-start step {n}: legalActions offered \"{}\" but reduce refused it: {error}",
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
        seed: 0,
        input,
        hash: hash_state(&state),
        heaviest,
    })
}

struct PlayedDuos {
    input: DuosReplayInput,
    hash: String,
    heaviest: [Option<(i64, usize, DuosState)>; 3],
}

fn play_coop_game(seed: u32) -> Result<PlayedDuos> {
    let options = DuosCreateOptions {
        seed: format!("perf-duos-{seed}"),
        decks: duos_decks_for_seed_from(seed, &SHIPPED_FUZZ_POOL),
        ..DuosCreateOptions::default()
    };
    let begun = duos_begin(&duos_create(&options));
    let mut state = begun.state;
    let mut rng = Rng::new(&format!("perf-duos-policy-{seed}"), 0);
    let mut log: Vec<DuosAction> = Vec::new();
    let mut heaviest: [Option<(i64, usize, DuosState)>; 3] = [None, None, None];

    while log.len() < 200 && state.result.is_none() {
        let seats = duos_seats_to_act(&state);
        let Some(&seat) = rng.pick(&seats) else { break };
        let actions: Vec<DuosActionBody> = duos_legal_actions(&state, seat)
            .into_iter()
            .filter(|action| match action {
                DuosActionBody::Concede => false,
                DuosActionBody::Duel { body } => !AI_SKIPPED_ACTIONS.contains(&body.action_type()),
                DuosActionBody::Pass { .. } => true,
            })
            .collect();
        let Some(body) = rng.pick(&actions).cloned() else {
            break;
        };
        let action = DuosAction {
            seat,
            nonce: format!("perf-duos-{seed}-{}", log.len()),
            body,
        };
        let result = duos_reduce(&state, &action);
        if let Some(error) = result.error {
            bail!("perf duos step {}: duos_reduce refused: {error}", log.len());
        }
        log.push(action);
        for (kept, value) in heaviest.iter_mut().zip(weigh_duos(&result.events, &result.state)) {
            if kept.as_ref().is_none_or(|(most, _, _)| value > *most) {
                *kept = Some((value, log.len(), result.state.clone()));
            }
        }
        state = result.state;
    }

    Ok(PlayedDuos {
        input: DuosReplayInput { options, log },
        hash: duos_hash(&state),
        heaviest,
    })
}

fn take_heaviest(games: &[Played]) -> Vec<GameState> {
    let mut states = Vec::new();
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
    states
}

fn take_heaviest_duos(games: &[PlayedDuos]) -> Vec<DuosState> {
    let mut states = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    for measure in 0..3 {
        let mut candidates: Vec<(i64, usize, &DuosState)> = games
            .iter()
            .filter_map(|game| {
                game.heaviest[measure]
                    .as_ref()
                    .map(|(value, step, state)| (*value, *step, state))
            })
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut added = 0;
        for (_, step, state) in candidates {
            if added == PERF_HEAVIEST_PER_MEASURE {
                break;
            }
            if taken.contains(&step) {
                continue;
            }
            taken.push(step);
            states.push(state.clone());
            added += 1;
        }
    }
    states
}

/// The states every pass walks, and the game the fold replays.
struct Corpus {
    states: Vec<GameState>,
    coop_states: Vec<DuosState>,
    hotseat: usize,
    wide: usize,
    plain: usize,
    handicapped: usize,
    run_start: usize,
    coop: usize,
    fold: FoldArgs,
    fold_seed: u32,
    fold_hash: String,
    duos_fold: DuosReplayInput,
    duos_fold_hash: String,
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

    let plain_games: Vec<Played> = (PERF_SEED_FIRST..=PERF_SEED_LAST)
        .into_par_iter()
        .map(play_seed)
        .collect::<Result<Vec<_>>>()?;

    let handicapped_games: Vec<Played> = (PERF_HANDICAP_SEED_FIRST..=PERF_HANDICAP_SEED_LAST)
        .into_par_iter()
        .map(play_seed)
        .collect::<Result<Vec<_>>>()?;

    let run_start_game = play_run_start()?;

    let plain_states = take_heaviest(&plain_games);
    let plain = plain_states.len();
    states.extend(plain_states);

    let handicapped_states = take_heaviest(&handicapped_games);
    let handicapped = handicapped_states.len();
    states.extend(handicapped_states);

    let run_start_states = take_heaviest(std::slice::from_ref(&run_start_game));
    let run_start = run_start_states.len();
    states.extend(run_start_states);

    let coop_game = play_coop_game(1)?;
    let coop_states = take_heaviest_duos(std::slice::from_ref(&coop_game));
    let coop = coop_states.len();

    // The longest 1v1 game, the lower seed on a tie.
    let longest = plain_games
        .iter()
        .chain(&handicapped_games)
        .chain(std::slice::from_ref(&run_start_game))
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
        coop_states,
        hotseat,
        wide,
        plain,
        handicapped,
        run_start,
        coop,
        fold: longest.input.clone(),
        fold_seed: longest.seed,
        fold_hash: longest.hash.clone(),
        duos_fold: coop_game.input,
        duos_fold_hash: coop_game.hash,
    })
}

// The passes

/// One pass's samples of the timed measures.
#[derive(Default)]
struct Samples {
    reduce: Vec<f64>,
    view_for: Vec<f64>,
    combat_forecast: Vec<f64>,
    legal_actions: Vec<f64>,
    hash_state: Vec<f64>,
    fold: Vec<f64>,
    duos_reduce: Vec<f64>,
    duos_view_for: Vec<f64>,
    duos_combat_forecast: Vec<f64>,
    duos_legal_actions: Vec<f64>,
    duos_fold: Vec<f64>,
    forecast_attacks: usize,
    max_forecast_attacks: usize,
    duos_forecast_attacks: usize,
}

impl Samples {
    fn extend(&mut self, pass: Samples) {
        self.reduce.extend(pass.reduce);
        self.view_for.extend(pass.view_for);
        self.combat_forecast.extend(pass.combat_forecast);
        self.legal_actions.extend(pass.legal_actions);
        self.hash_state.extend(pass.hash_state);
        self.fold.extend(pass.fold);
        self.duos_reduce.extend(pass.duos_reduce);
        self.duos_view_for.extend(pass.duos_view_for);
        self.duos_combat_forecast.extend(pass.duos_combat_forecast);
        self.duos_legal_actions.extend(pass.duos_legal_actions);
        self.duos_fold.extend(pass.duos_fold);
        self.forecast_attacks += pass.forecast_attacks;
        self.max_forecast_attacks = self.max_forecast_attacks.max(pass.max_forecast_attacks);
        self.duos_forecast_attacks += pass.duos_forecast_attacks;
    }
}

/// One pass: every state in order (each seat's `legal_actions`, `view_for`, then `combat_forecast`,
/// then `hash_state`, then `reduce` of each of `kept`'s actions for that state), then the fold and its
/// hash.
fn timed_pass(corpus: &Corpus, kept: &[Vec<Action>], duos_kept: &[Vec<DuosAction>]) -> Result<Samples> {
    let mut samples = Samples::default();
    for (state, actions) in corpus.states.iter().zip(kept) {
        for seat in PLAYER_IDS {
            samples
                .legal_actions
                .push(seconds(|| legal_actions(state, seat)) * US_PER_SECOND);
            samples
                .view_for
                .push(seconds(|| view_for(state, seat)) * US_PER_SECOND);
            let (took, forecast) = seconds_with_result(|| combat_forecast(state, seat));
            samples.combat_forecast.push(took * US_PER_SECOND);
            let attacks = forecast.attacks.len();
            samples.forecast_attacks += attacks;
            samples.max_forecast_attacks = samples.max_forecast_attacks.max(attacks);
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
    samples.fold.push(fold_us_per_action(
        started.elapsed().as_secs_f64(),
        corpus.fold.log.len(),
    ));
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

    for (state, actions) in corpus.coop_states.iter().zip(duos_kept) {
        for seat in DUOS_SEATS {
            samples
                .duos_legal_actions
                .push(seconds(|| duos_legal_actions(state, seat)) * US_PER_SECOND);
            samples
                .duos_view_for
                .push(seconds(|| duos_view_for(state, seat)) * US_PER_SECOND);
            let (took, forecast) =
                seconds_with_result(|| combat_forecast(&state.duels[seat.duel()], seat.player()));
            samples.duos_combat_forecast.push(took * US_PER_SECOND);
            samples.duos_forecast_attacks += forecast.attacks.len();
        }
        for action in actions {
            samples
                .duos_reduce
                .push(seconds(|| duos_reduce(state, action)) * US_PER_SECOND);
        }
    }

    let started = Instant::now();
    let duos_folded = std::hint::black_box(duos_fold(&corpus.duos_fold));
    samples.duos_fold.push(fold_us_per_action(
        started.elapsed().as_secs_f64(),
        corpus.duos_fold.log.len(),
    ));
    if let Some(at) = duos_folded.refused_at {
        bail!("the fold of duos co-op game refused action at index {at}");
    }
    if duos_folded.hash != corpus.duos_fold_hash {
        bail!(
            "the fold of duos co-op game reaches {}, not the live game's {}",
            duos_folded.hash,
            corpus.duos_fold_hash
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
    /// Whether the measure is a time (`Measure::needs_clock`), which `--smoke` does not judge.
    #[serde(skip)]
    needs_clock: bool,
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
        needs_clock: measure.needs_clock(),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CorpusCounts {
    states: usize,
    hotseat: usize,
    wide: usize,
    plain: usize,
    handicapped: usize,
    run_start: usize,
    coop: usize,
    forecast_attacks: usize,
    max_forecast_attacks: usize,
    duos_forecast_attacks: usize,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DuosFoldInfo {
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
    duos_fold: DuosFoldInfo,
    measures: Vec<Row>,
    /// One message per median past its budget; under `--smoke`, only the work measure is judged.
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

    let duos_kept: Vec<Vec<DuosAction>> = corpus
        .coop_states
        .iter()
        .map(|state| {
            DUOS_SEATS
                .into_iter()
                .flat_map(|seat| {
                    let listed = duos_legal_actions(state, seat);
                    spread(listed.len(), PERF_ACTIONS_PER_SEAT)
                        .into_iter()
                        .map(|i| DuosAction {
                            seat,
                            nonce: format!("perf-duos-timed-{seat}-{i}"),
                            body: listed[i].clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .collect();

    for _ in 0..warmups {
        timed_pass(&corpus, &kept, &duos_kept)?;
    }
    let mut samples = Samples::default();
    for _ in 0..passes {
        samples.extend(timed_pass(&corpus, &kept, &duos_kept)?);
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
                Measure::CombatForecast => std::mem::take(&mut samples.combat_forecast),
                Measure::LegalActions => std::mem::take(&mut samples.legal_actions),
                Measure::HashState => std::mem::take(&mut samples.hash_state),
                Measure::Fold => std::mem::take(&mut samples.fold),
                Measure::DuosReduce => std::mem::take(&mut samples.duos_reduce),
                Measure::DuosViewFor => std::mem::take(&mut samples.duos_view_for),
                Measure::DuosCombatForecast => std::mem::take(&mut samples.duos_combat_forecast),
                Measure::DuosLegalActions => std::mem::take(&mut samples.duos_legal_actions),
                Measure::DuosFold => std::mem::take(&mut samples.duos_fold),
                Measure::ViewForBytes => std::mem::take(&mut bytes),
            };
            row(measure, taken)
        })
        .collect::<Result<Vec<_>>>()?;
    let failures = if smoke {
        judge_smoke(&measures)
    } else {
        judge(&measures)
    };
    Ok(Report {
        profile: PROFILE,
        smoke,
        passes,
        corpus: CorpusCounts {
            states: corpus.states.len() + corpus.coop_states.len(),
            hotseat: corpus.hotseat,
            wide: corpus.wide,
            plain: corpus.plain,
            handicapped: corpus.handicapped,
            run_start: corpus.run_start,
            coop: corpus.coop,
            forecast_attacks: samples.forecast_attacks / passes,
            max_forecast_attacks: samples.max_forecast_attacks,
            duos_forecast_attacks: samples.duos_forecast_attacks / passes,
            seeds: [PERF_SEED_FIRST, PERF_SEED_LAST],
        },
        fold: FoldInfo {
            seed: corpus.fold_seed,
            actions: corpus.fold.log.len(),
            passes,
        },
        duos_fold: DuosFoldInfo {
            actions: corpus.duos_fold.log.len(),
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

/// `--smoke`'s verdict: `judge` over the rows that need no clock, the one work measure.
fn judge_smoke(rows: &[Row]) -> Vec<String> {
    let unclocked: Vec<Row> = rows.iter().filter(|row| !row.needs_clock).cloned().collect();
    judge(&unclocked)
}

/// The report as one table: a line saying what was measured, then measure, unit, median, p95 and budget.
fn perf_table(report: &Report) -> String {
    let corpus = &report.corpus;
    let mut out = format!(
        "perf: {} profile, {} timed pass(es) over {} states ({} hotseat, {} wide, {} plain, {} handicapped, {} run-start, {} co-op); forecast: {} attacks (max {}); duos forecast: {} attacks; fold: fuzz seed {}, {} actions; duos fold: {} actions",
        report.profile,
        report.passes,
        corpus.states,
        corpus.hotseat,
        corpus.wide,
        corpus.plain,
        corpus.handicapped,
        corpus.run_start,
        corpus.coop,
        corpus.forecast_attacks,
        corpus.max_forecast_attacks,
        corpus.duos_forecast_attacks,
        report.fold.seed,
        report.fold.actions,
        report.duos_fold.actions,
    );
    if report.smoke {
        out.push_str("; smoke: work budget judged, clock budgets not");
    }
    out.push('\n');
    let _ = writeln!(
        out,
        "{:<21} {:<5} {:>12} {:>12} {:>12}",
        "measure", "unit", "median", "p95", "budget"
    );
    for row in &report.measures {
        let _ = writeln!(
            out,
            "{:<21} {:<5} {:>12.1} {:>12.1} {:>12.1}",
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
            needs_clock: measure.needs_clock(),
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
    fn under_smoke_only_the_work_budget_fails_even_at_twice_every_budget() {
        let rows: Vec<Row> = MEASURES
            .iter()
            .map(|&measure| row_at(measure, 2.0 * measure.budget().0))
            .collect();
        let failures = judge_smoke(&rows);
        assert_eq!(failures.len(), 1, "{failures:#?}");
        assert!(
            failures[0].starts_with("view_for bytes:") && failures[0].ends_with("(VIEW_FOR_BYTES_BUDGET)"),
            "{failures:#?}"
        );
    }

    #[test]
    fn fold_divides_the_fold_time_by_its_actions() {
        assert_eq!(fold_us_per_action(0.2424, 256), 946.875);
        assert_eq!(Measure::Fold.unit(), "us/action");
        let expected = if cfg!(debug_assertions) {
            "FOLD_US_PER_ACTION_BUDGET_TEST"
        } else {
            "FOLD_US_PER_ACTION_BUDGET_RELEASE"
        };
        assert_eq!(Measure::Fold.budget().1, expected);
        assert_eq!(Measure::DuosFold.unit(), "us/action");
        let expected_duos = if cfg!(debug_assertions) {
            "DUOS_FOLD_US_PER_ACTION_BUDGET_TEST"
        } else {
            "DUOS_FOLD_US_PER_ACTION_BUDGET_RELEASE"
        };
        assert_eq!(Measure::DuosFold.budget().1, expected_duos);
    }

    #[test]
    fn under_smoke_every_measure_at_its_budget_passes() {
        let rows: Vec<Row> = MEASURES
            .iter()
            .map(|&measure| row_at(measure, measure.budget().0))
            .collect();
        assert_eq!(judge_smoke(&rows), Vec::<String>::new());
    }

    #[test]
    fn the_table_has_a_line_per_measure_under_its_header() {
        let report = Report {
            profile: PROFILE,
            smoke: true,
            passes: 1,
            corpus: CorpusCounts {
                states: 6,
                hotseat: 1,
                wide: 1,
                plain: 1,
                handicapped: 1,
                run_start: 1,
                coop: 1,
                forecast_attacks: 13,
                max_forecast_attacks: 14,
                duos_forecast_attacks: 15,
                seeds: [PERF_SEED_FIRST, PERF_SEED_LAST],
            },
            fold: FoldInfo {
                seed: 7,
                actions: 12,
                passes: 1,
            },
            duos_fold: DuosFoldInfo {
                actions: 18,
                passes: 1,
            },
            measures: MEASURES.iter().map(|&measure| row_at(measure, 1.5)).collect(),
            failures: Vec::new(),
        };
        let table = perf_table(&report);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 2 + MEASURES.len(), "{table}");
        assert!(lines[0].starts_with("perf: "), "{table}");
        assert!(
            lines[0].ends_with("; smoke: work budget judged, clock budgets not"),
            "{table}"
        );
        assert!(lines[0].contains("fuzz seed 7, 12 actions"), "{table}");
        assert!(lines[0].contains("duos fold: 18 actions"), "{table}");
        assert!(lines[0].contains("forecast: 13 attacks (max 14)"), "{table}");
        assert!(lines[0].contains("duos forecast: 15 attacks"), "{table}");
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
