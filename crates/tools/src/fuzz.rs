//! `cargo jackioh fuzz` (SURFACE §12): the M4 gate (BUILD "M4 gate", SPEC §10.9, REVIEW B8), and with
//! `--handicap` its handicapped wave (SPEC §9.9, R180–R184, R187). Port of
//! `packages/cards/test/fuzz.test.ts` and `packages/cards/test/fuzz-handicap.test.ts`, in that order.
//!
//! # The gate (fuzz.test.ts)
//!
//!   "cards/test/fuzz.test.ts: 1,000 games per wave (seeds 1–1000) with decks drawn randomly from
//!    all implemented cards, played by aiPolicy, never throw, always terminate (hero death or cap),
//!    and replay to the same hash. Any card that appears in a failing seed is listed in the failure
//!    message."
//!
//! Four assertions per seed, in this order:
//!   1. NO THROW: playing the game out raises nothing (a panic, here).
//!   2. TERMINATION: the game reaches a real `state.result` (hero death, both heroes dead, or the R2
//!      turn cap at TURN_CAP_PLAYER_TURNS player-turns), inside a bounded number of actions. The
//!      bound is not a pass condition: hitting it is a reported failure, so a non-terminating game
//!      fails loudly instead of hanging CI (the policy never concedes or offers a draw, R84, so those
//!      endings cannot occur here).
//!   3. REPLAY EQUALITY: folding `(seed, decks, log)` in a fresh `create_game` reproduces the same
//!      state hash, with no rejected actions. This is what makes SPEC §9.3's "(seed, log)
//!      reconstructs any match" true.
//!   4. FAILURE DETAIL: every card id in the failing seed's two decks is named in the report, because
//!      a bare seed number is useless against a 100-card pool.
//!
//! And at every step, INVARIANTS: `testkit::invariants`'s I1–I5 and I8 (summoning sickness and exertion,
//! R171, and nothing after game over, R216) hold on the state each action is chosen in and on the
//! state it produces (stage "invariant"), and I6 (no card a seat may not read in its view or legal
//! actions, §10.8) holds for both seats on every state, setup's included: every state under
//! `cargo test`, every I6_GATE_STRIDE-th under the gate.
//! I8 (no unlisted attack or switch `reduce` accepts, §4.1, §4.2) runs on every state under `cargo test`,
//! every I8_GATE_STRIDE-th under the gate and `--handicap`.
//!
//! And after every action, GROWTH (#1254): the gate weighs each action it applies by the events `reduce`
//! returned, the largest number on any card instance and the number of instances in play, prints each
//! measure's heaviest actions (one per game) in the summary and in `--json`, and fails a seed in the
//! "growth" stage when one action passes GROWTH_EVENTS_BOUND, GROWTH_NUMBER_BOUND or
//! GROWTH_INSTANCES_BOUND. That catches runaway growth inside one action (R874000's fusion, R884000's
//! University) at the step that grew, well before the engine's settle cap (§10.3) panics or a process
//! runs out of memory. The weighing reads the returned events and state only, never the clock, so it
//! is as deterministic as the game; `--handicap` is not weighed, and `--duos` is weighed by
//! `duos_fuzz.rs` through the shared helpers below.
//!
//! Determinism (SPEC §9.3, §10.7, CLAUDE.md rule 4): nothing here reads OS randomness or the clock
//! for a game (the clock only times the wave). The deck draw, the game and the policy each come from
//! `Rng::new` over a seed string derived from the seed number, so a reported seed reproduces its game
//! exactly, in this process or any other, on any thread.
//!
//! The policy rng is deliberately SEPARATE from the game rng: `reduce` takes no rng, so it builds its
//! own from `(state.seed, state.rng_cursor)` exactly as `fold` does. The policy's coin flips are
//! therefore outside the state, and the recorded action log is the only thing replay needs — which is
//! the property assertion 3 exists to prove.
//!
//! The seeds run in parallel (rayon): each game is its own state, its own rngs and its own monitor,
//! and the registries are the process's read-only `OnceLock`s (SURFACE §3), so a game on one thread
//! cannot see another's. The report reads the results back in seed order.
//!
//! HOW TO RUN
//!   cargo jackioh fuzz                              the gate: seeds 1–1000
//!   cargo jackioh fuzz --handicap                   the handicapped wave: seeds 1–1000
//!   cargo jackioh fuzz --duos                       Duos co-op games: seeds 1–200 (`duos_fuzz.rs`)
//!   cargo test -p jackioh-tools fuzz                seeds 1–20 of each (see SWEEP_SEEDS)
//!   cargo jackioh fuzz --seeds 200 --json           the wave as one JSON object
//!   cargo jackioh fuzz --from <seed> --seeds 1      one seed a report named
//!   cargo jackioh fuzz --regress                    the regression corpus
//!
//! # The regression corpus (#1197)
//!
//! A seed's deal moves whenever a card joins `FUZZ_POOL`, so a seed a past report named stops
//! reproducing the day the next card lands. Each kept failure therefore pins its own decks in
//! `crates/tools/fuzz-corpus/`; `--regress` replays them, each with its pinned decks and never a deal
//! from the live pool, through the gate's checks, and the tools' `cargo test` runs it.
//!
//! # The handicapped wave (fuzz-handicap.test.ts)
//!
//! The gate plays every seed with this spec's resources on both seats, which is what every online
//! match uses. Practice gives the AI's seat a handicap, and the handicap changes the rules a game runs
//! through: a 25- or 30-card deck (R184), one more crystal up to a higher cap (R181), an extra opening
//! card (R182), and at Hard a second start-of-turn draw with its own cast-on-draw chain, hand-cap
//! check and fatigue step (R183). So this wave plays the same random-policy games with one seat
//! handicapped, rotating by seed over Medium and Hard and over both seats, and holds them to the same
//! three assertions: no throw, a real ending inside the cap, and a log that folds back, handicaps
//! included, to the live hash. Each game also runs the gate's invariant monitor (I1–I8) the same way.

use std::cell::RefCell;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::{LazyLock, Once};
use std::time::Instant;

use anyhow::{Context, bail};
use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, register_all, shipped_versions};
use jackioh_engine::testkit::{
    I6_GATE_STRIDE, I8_GATE_STRIDE, create_invariant_monitor, preview_every_set, set_ships,
};
use jackioh_engine::{
    AI_DIFFICULTY, AI_PLAYOUT_STEP_CAP, Action, ActionBody, CardInstance, CreateGameOptions, DECK_SIZE,
    Difficulty, DuosActionBody, DuosEvent, DuosSeat, DuosState, FoldArgs, GameEvent, GameState, Handicap,
    PLAYER_IDS, PerPlayerOpt, Phase, PlayerId, Rng, TURN_CAP_PLAYER_TURNS, begin_game, create_game,
    find_instance, fold, hash_state, mulligan_owed, reduce, seat_to_act, subsystems, validate_deck,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::agent::panic_message;
use crate::gate::MS_PER_SECOND;

/// `cargo jackioh fuzz [--from N] [--seeds N] [--handicap | --duos | --regress]` (SURFACE §12).
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The first seed of the wave (TS `JACKIOH_FUZZ_FROM`): 1 for the gate. `--from 700 --seeds 1`
    /// replays exactly the seed a report named, without the 699 before it.
    #[arg(long)]
    pub from: Option<u32>,
    /// How many seeds this run plays (TS `JACKIOH_FUZZ_SEEDS`): narrows the wave by hand while chasing
    /// a failure, or widens it past WAVE_SEEDS (the daily run's quarters, `--seeds 2500`, #659).
    #[arg(long)]
    pub seeds: Option<u32>,
    /// Play the handicapped wave (fuzz-handicap.test.ts) instead of the gate: one seat on Medium or
    /// Hard, rotating by seed.
    #[arg(long)]
    pub handicap: bool,
    /// Play random-policy Duos co-op games instead of the gate (#936 B25, `duos_fuzz.rs`): 200 seeds
    /// unless `--seeds` says otherwise.
    #[arg(long)]
    pub duos: bool,
    /// Replay the regression corpus (`crates/tools/fuzz-corpus/`, #1197) instead of a wave: each kept
    /// failure's pinned decks, never a deal from the live pool.
    #[arg(long, conflicts_with_all = ["from", "seeds", "handicap", "duos"])]
    pub regress: bool,
    /// Print the gate's wave as one JSON object instead of text (#1254): its failures, each
    /// measure's heaviest actions and the growth bounds.
    #[arg(long, conflicts_with_all = ["handicap", "regress"])]
    pub json: bool,
}

/// Runs the gate, or with `--handicap` the handicapped wave, or with `--duos` the Duos wave, or with
/// `--regress` the regression corpus; prints its numbers and, when a seed failed, the report, and fails
/// (exit 1).
pub fn run(args: Args) -> anyhow::Result<()> {
    if args.regress {
        return run_regress();
    }
    let from = first_seed(args.from);
    if args.duos {
        return crate::duos_fuzz::run(from, args.seeds, args.json);
    }
    let ran = wave_size(args.seeds);
    if args.handicap {
        let wave = play_handicap_wave(from, ran);
        println!("{}", wave.summary());
        let failures = wave.failure_lines();
        if !failures.is_empty() {
            println!("{}", failures.join("\n"));
            bail!("fuzz-handicap: {} of {} seed(s) failed", failures.len(), ran);
        }
        return Ok(());
    }
    let wave = play_wave(from, ran);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&wave.json())?);
    } else {
        print!("{}", wave.summary());
    }
    if !wave.failures.is_empty() {
        if !args.json {
            println!("{}", report(&wave.failures, ran, wave.elapsed_ms));
        }
        bail!("M4 fuzz gate: {} of {} seeds failed", wave.failures.len(), ran);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Wave size
// ---------------------------------------------------------------------------------------------

/// BUILD M4 gate: seeds 1–1000, one game each. This is the gate and the default.
pub const WAVE_SEEDS: u32 = 1000;

/// The wave `cargo test` runs (TS: the first 100 under `pnpm test`; the Rust smoke wave is seeds 1–20,
/// part 22's brief).
///
/// The full wave is minutes against seconds for every other test of the tools crate, so the test
/// sweep runs the first seeds as a smoke wave and `cargo jackioh fuzz` runs all 1,000. The smoke wave
/// is the same games, same pool and same four assertions — the coverage of the CARD POOL is never
/// narrowed, only the number of seeds — and the summary line says which wave ran.
pub const SWEEP_SEEDS: u32 = 20;

/// Whether this process is the whole-suite sweep: the test build. Every other way in — `cargo
/// jackioh fuzz`, a CI step — gets the full 1,000, because the safe default when the caller is
/// unknown is the gate, not the smoke test.
fn is_whole_suite_sweep() -> bool {
    cfg!(test)
}

/// I6's stride: every state in the test sweep, every I6_GATE_STRIDE-th in the gate.
pub fn hidden_stride() -> usize {
    if is_whole_suite_sweep() { 1 } else { I6_GATE_STRIDE }
}

/// I8's stride: every state in the test sweep, every I8_GATE_STRIDE-th in the gate and the
/// handicapped wave.
pub fn unlisted_stride() -> usize {
    if is_whole_suite_sweep() { 1 } else { I8_GATE_STRIDE }
}

/// How many seeds this run plays (from `first_seed()`, which is 1 unless a developer moves it).
/// `--seeds 25` narrows it by hand while chasing a failure and `--seeds 2500` widens it (super.yml's
/// quarters); the summary line always reports which seeds actually ran.
pub fn wave_size(raw: Option<u32>) -> u32 {
    match raw {
        Some(parsed) if parsed >= 1 => parsed,
        _ => {
            if is_whole_suite_sweep() {
                SWEEP_SEEDS
            } else {
                WAVE_SEEDS
            }
        }
    }
}

/// The first seed of the wave. 1 for the gate; `--from 700 --seeds 1` replays exactly the seed a
/// report named, without the 699 before it. A seed is its own game whatever wave it sits in, so this
/// changes which games run and nothing about how they run.
pub fn first_seed(raw: Option<u32>) -> u32 {
    match raw {
        Some(parsed) if parsed >= 1 => parsed,
        _ => 1,
    }
}

/// Actions one game may take before it is declared non-terminating. A game is capped at
/// TURN_CAP_PLAYER_TURNS player-turns and a turn is capped at AI_PLAYOUT_STEP_CAP policy steps, so
/// their product is the engine's own worst case; this bound sits at that ceiling so it can only ever
/// fire on a genuine loop, never on a long-but-legal game.
fn max_actions_per_game() -> usize {
    usize::try_from(TURN_CAP_PLAYER_TURNS).unwrap_or(0) * AI_PLAYOUT_STEP_CAP
}

/// §2.5 plus R84: the policy never concedes or offers a draw, so only these can occur, a Glitch that
/// voids the match (R679) among them.
const TERMINAL_REASONS: &[&str] = &[
    "hero-death",
    "both-heroes-dead",
    "turn-cap",
    "alt-win",
    "won-by-effect",
    "voided",
];

// ---------------------------------------------------------------------------------------------
// The deck pool
// ---------------------------------------------------------------------------------------------

/// A card kept out of the pool, with the reason and the issue it waits on.
pub struct PoolExclusion {
    pub id: &'static str,
    /// For the reader of this list: no report prints it (TS's did not either).
    #[allow(dead_code)]
    pub why: &'static str,
}

/// Cards deliberately kept OUT of the fuzz deck pool.
///
/// This list is the only way coverage is ever narrowed, and it must stay empty in a green tree: the
/// BUILD definition of done is "1,000 seeds with the full card pool". Anything parked here is a
/// known-unimplemented or known-broken card that hides every other bug behind its own, and each
/// entry carries the reason and the issue it is waiting on. Emptying it is the fix; adding to it to
/// make the gate green is not.
pub const POOL_EXCLUSIONS: &[PoolExclusion] = &[];

static EXCLUDED_IDS: LazyLock<IndexSet<&'static str>> =
    LazyLock::new(|| POOL_EXCLUSIONS.iter().map(|entry| entry.id).collect());

/// Every deck-legal card: the whole catalog minus tokens (§2.6 L3, which `validate_deck` enforces)
/// minus `POOL_EXCLUSIONS`, of every set, shipped or not: each seed previews every set (R1420), so a
/// set's cards are fuzzed from the day they land. Sorted by catalog id so the pool a seed shuffles is identical on every
/// machine and in every process, whatever order the registry handed the defs over.
pub static FUZZ_POOL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.is_token())
        .map(|(id, _)| id.clone())
        .filter(|id| !EXCLUDED_IDS.contains(id.as_str()))
        .collect();
    pool.sort();
    pool
});

/// R1420: `FUZZ_POOL` with the sets that have not shipped left out, in the same order. The golden
/// traces deal from it (`golden.rs`), so a set being built never changes a recorded game's decks:
/// while every set ships it is `FUZZ_POOL` itself, and the decks are §13.1's.
pub static SHIPPED_FUZZ_POOL: LazyLock<Vec<String>> = LazyLock::new(|| shipped_only(&FUZZ_POOL));

/// `pool` minus every card of a set that has not shipped (R1420).
fn shipped_only(pool: &[String]) -> Vec<String> {
    pool.iter()
        .filter(|id| CATALOG.get(id.as_str()).is_some_and(|def| set_ships(def.set)))
        .cloned()
        .collect()
}

static CARD_NAMES: LazyLock<IndexMap<String, String>> = LazyLock::new(|| {
    CATALOG
        .iter()
        .map(|(id, def)| (id.clone(), format!("{} (#{})", def.name, def.index)))
        .collect()
});

fn describe_card(id: &str) -> String {
    format!(
        "{id} — {}",
        CARD_NAMES.get(id).map_or("unknown card", String::as_str)
    )
}

/// The seed drives the deck draw (§9.3): one seeded shuffle of the whole pool, the first 20 cards to
/// p1 and the next 20 to p2. One shuffle rather than two draws means the 40 cards are distinct, so
/// `validate_deck`'s "no duplicate card ids" (§2.6 L3) holds by construction for both decks, and a
/// reported seed rebuilds its exact deck pair with no other input.
pub fn decks_for_seed(seed: u32) -> (Vec<String>, Vec<String>) {
    decks_for_seed_from(seed, &FUZZ_POOL)
}

/// `decks_for_seed` over another pool: the golden traces deal seed k from `SHIPPED_FUZZ_POOL`.
pub fn decks_for_seed_from(seed: u32, pool: &[String]) -> (Vec<String>, Vec<String>) {
    let mut rng = Rng::new(&format!("jackioh-fuzz-decks-{seed}"), 0);
    let shuffled = rng.shuffle(pool);
    let size = usize::try_from(DECK_SIZE).unwrap_or(0);
    (shuffled[..size].to_vec(), shuffled[size..size * 2].to_vec())
}

// ---------------------------------------------------------------------------------------------
// Playing one game with the real policy
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureStage {
    Play,
    Termination,
    Replay,
    Invariant,
    Growth,
}

impl FailureStage {
    fn as_str(self) -> &'static str {
        match self {
            FailureStage::Play => "play",
            FailureStage::Termination => "termination",
            FailureStage::Replay => "replay",
            FailureStage::Invariant => "invariant",
            FailureStage::Growth => "growth",
        }
    }
}

/// How far a game had got when it failed.
#[derive(Clone, Copy, Debug)]
struct At {
    turn: i32,
    actions: usize,
}

/// A failure carrying the stage it happened in, so the report can group by kind as well as by text,
/// plus how far the game had got — a raw panic out of `play_game` leaves no run for the report to
/// read that from.
#[derive(Debug)]
struct FuzzFailure {
    stage: FailureStage,
    message: String,
    at: At,
}

impl FuzzFailure {
    fn new(stage: FailureStage, message: String, at: At) -> FuzzFailure {
        FuzzFailure { stage, message, at }
    }
}

struct GameRun {
    state: GameState,
    log: Vec<Action>,
    decks: (Vec<String>, Vec<String>),
    /// This game's heaviest action by each measure (#1254).
    heaviest: Heaviest,
}

/// `pending ${state.pending === null ? "none" : `"${state.pending.kind}"`}`.
fn pending_label(state: &GameState) -> String {
    match &state.pending {
        None => "none".to_string(),
        Some(pending) => format!("\"{}\"", pending.kind),
    }
}

/// With a prompt open only its holder may act (§9.3); otherwise it is the active player's turn.
/// R265: while both mulligans are open either seat may answer first; `order` picks which.
pub(crate) fn actor_of(state: &GameState, order: &mut Rng) -> PlayerId {
    if mulligan_owed(state).len() == 2 && order.coin() {
        PlayerId::P2
    } else {
        seat_to_act(state).unwrap_or(state.active)
    }
}

/// One full game played by SPEC §10.7's policy. `subsystems::choose_action` IS that policy — uniform
/// over `legal_actions` minus `AI_SKIPPED_ACTIONS` (R84), ending the turn when nothing else is on
/// offer or with `AI_END_TURN_PROBABILITY`, and answering an open prompt uniformly (R44) — so this
/// drives the shipped machinery rather than a second copy of the rule.
fn play_game(seed: u32, decks: (Vec<String>, Vec<String>)) -> Result<GameRun, FuzzFailure> {
    let game_seed = format!("jackioh-fuzz-{seed}");
    register_all();

    let begun = begin_game(&create_game(&CreateGameOptions {
        seed: game_seed,
        decks: decks.clone(),
        ..CreateGameOptions::default()
    }));
    let mut state = begun.state;
    let mut policy = Rng::new(&format!("jackioh-fuzz-policy-{seed}"), 0);
    // R265: while both mulligans are open either seat may answer first; a stream of its own picks
    // which, so the fuzz plays both orders (the game is the same either way, R265).
    let mut order = Rng::new(&format!("jackioh-fuzz-policy-order-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    let mut monitor = create_invariant_monitor(&state);
    let stride = hidden_stride();
    let unlisted_stride = unlisted_stride();
    let max_actions = max_actions_per_game();
    let mut heaviest = Heaviest::default();
    // Setup's own events: a Unit cast on draw in the opening deal (C+ #26 Tommy Tempo) enters there.
    let mut setup = monitor.after(&begun.events, &state);
    setup.extend(monitor.hidden(&state));
    if let Some(first) = setup.first() {
        return Err(FuzzFailure::new(
            FailureStage::Invariant,
            first.clone(),
            At {
                turn: state.turn,
                actions: 0,
            },
        ));
    }

    while state.result.is_none() {
        if log.len() >= max_actions {
            return Err(FuzzFailure::new(
                FailureStage::Termination,
                format!(
                    "game did not terminate within {max_actions} actions (turn {}/{TURN_CAP_PLAYER_TURNS}, phase \"{}\", pending {})",
                    state.turn,
                    state.phase,
                    pending_label(&state)
                ),
                At {
                    turn: state.turn,
                    actions: log.len(),
                },
            ));
        }

        // With a prompt open only its holder may act (§9.3); otherwise it is the active player's turn.
        let player = actor_of(&state, &mut order);
        let Some(chosen): Option<ActionBody> = subsystems::choose_action(&state, player, &mut policy) else {
            return Err(FuzzFailure::new(
                FailureStage::Termination,
                format!(
                    "no legal action for {player} while the game is live (turn {}, phase \"{}\", pending {}): R82 auto-ends a turn with nothing left to do, so this is a stall, not an ending",
                    state.turn,
                    state.phase,
                    pending_label(&state)
                ),
                At {
                    turn: state.turn,
                    actions: log.len(),
                },
            ));
        };

        let unsound = monitor.before(&state, player, &chosen);
        if let Some(first) = unsound.first() {
            return Err(FuzzFailure::new(
                FailureStage::Invariant,
                first.clone(),
                At {
                    turn: state.turn,
                    actions: log.len(),
                },
            ));
        }

        let action = Action::new(chosen, player, format!("fuzz-{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            return Err(FuzzFailure::new(
                FailureStage::Play,
                format!(
                    "legalActions offered \"{}\" but reduce refused it: {error}",
                    action.action_type().as_str()
                ),
                At {
                    turn: state.turn,
                    actions: log.len(),
                },
            ));
        }

        let action_type = action.action_type().as_str().to_string();
        let card = acting_card(&state, &action.body);
        log.push(action);
        state = result.state;

        let mut broken = monitor.after(&result.events, &state);
        if log.len().is_multiple_of(stride) || state.result.is_some() {
            broken.extend(monitor.hidden(&state));
        }
        if log.len().is_multiple_of(unlisted_stride) {
            broken.extend(monitor.unlisted(&state));
        }
        if let Some(first) = broken.first() {
            return Err(FuzzFailure::new(
                FailureStage::Invariant,
                first.clone(),
                At {
                    turn: state.turn,
                    actions: log.len(),
                },
            ));
        }

        // #1254: weigh the action; one past a growth bound fails the seed here.
        let step = log.len();
        let weights = weigh(&result.events, &state);
        for (measure, value) in MEASURES.into_iter().zip(weights) {
            if heaviest.list(measure).first().is_none_or(|top| value > top.value) {
                let heavy = Heavy {
                    seed,
                    step,
                    action: action_type.clone(),
                    card: card.clone(),
                    value,
                };
                heaviest.keep(measure, heavy, 1);
            }
        }
        if let Some(message) = growth_breach(&weights, &state, step, &action_type, card.as_deref()) {
            return Err(FuzzFailure::new(
                FailureStage::Growth,
                message,
                At {
                    turn: state.turn,
                    actions: step,
                },
            ));
        }
    }

    Ok(GameRun {
        state,
        log,
        decks,
        heaviest,
    })
}

/// Assertion 2: the game ended the way §2.5 says a policy game can end, inside the R2 cap.
fn check_termination(run: &GameRun) -> Result<(), FuzzFailure> {
    let at = At {
        turn: run.state.turn,
        actions: run.log.len(),
    };
    let Some(result) = &run.state.result else {
        return Err(FuzzFailure::new(
            FailureStage::Termination,
            "the game loop exited with no result".to_string(),
            at,
        ));
    };
    if !TERMINAL_REASONS.contains(&result.reason.as_str()) {
        return Err(FuzzFailure::new(
            FailureStage::Termination,
            format!(
                "ended with reason \"{}\"; the policy never concedes or offers a draw (R84), so §2.5 leaves only {}",
                result.reason,
                TERMINAL_REASONS.join(", ")
            ),
            at,
        ));
    }
    if run.state.turn > TURN_CAP_PLAYER_TURNS {
        return Err(FuzzFailure::new(
            FailureStage::Termination,
            format!(
                "reached turn {}, past the R2 cap of {TURN_CAP_PLAYER_TURNS} player-turns",
                run.state.turn
            ),
            at,
        ));
    }
    if run.state.phase != Phase::Over {
        return Err(FuzzFailure::new(
            FailureStage::Termination,
            format!("finished in phase \"{}\" rather than \"over\"", run.state.phase),
            at,
        ));
    }
    Ok(())
}

/// Assertion 3: `(seed, log)` is the truth (§9.3). Fold from scratch and compare the hash.
fn check_replay(seed: u32, run: &GameRun) -> Result<(), FuzzFailure> {
    let at = At {
        turn: run.state.turn,
        actions: run.log.len(),
    };
    register_all();
    let replayed = fold(&FoldArgs {
        seed: format!("jackioh-fuzz-{seed}"),
        decks: run.decks.clone(),
        log: run.log.clone(),
        catalog: None,
        handicaps: None,
        dealt: None,
        last_boards: None,
        glitch_boards: None,
        ..Default::default()
    });

    if let Some(first) = replayed.errors.first() {
        return Err(FuzzFailure::new(
            FailureStage::Replay,
            format!(
                "replaying the log rejected {} action(s); first at nonce \"{}\": {}",
                replayed.errors.len(),
                first.nonce,
                first.error
            ),
            at,
        ));
    }

    let live = hash_state(&run.state);
    let again = hash_state(&replayed.state);
    if live != again {
        return Err(FuzzFailure::new(
            FailureStage::Replay,
            format!(
                "replay hash {again} != live hash {live} after {} actions — folding (seed, log) did not reproduce the match (§9.3)",
                run.log.len()
            ),
            at,
        ));
    }

    let live_result = serde_json::to_string(&run.state.result).unwrap_or_default();
    let replayed_result = serde_json::to_string(&replayed.state.result).unwrap_or_default();
    if live_result != replayed_result {
        return Err(FuzzFailure::new(
            FailureStage::Replay,
            format!("replay ended {replayed_result}, live ended {live_result}"),
            at,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Growth inside one action (#1254)
// ---------------------------------------------------------------------------------------------

/// The most events one action may emit. Set from a 10,000-seed sweep of `main` at d981885 (seeds
/// 1–10,000, 2026-10-19, four quarters of 2,500): the most one passing game's action emitted was 843
/// (seed 7590, step 119), so this keeps a margin of about 12×, which clears R58's "draw your whole
/// library" chain of about 1,100 triggers. Runaway growth passes it far sooner than the settle cap
/// (§10.3) panics: seed 6540's resolution loop emitted 1,859 events at step 98, 7,628 at step 104 and
/// 30,361 at step 105, and panicked at the cap some steps later.
pub const GROWTH_EVENTS_BOUND: i64 = 10_000;

/// The largest number any card instance may carry after one action (an attack, a health, a cost, a
/// tuned number, a counter, a use count, a number in a card's memory). Set from the same sweep: the
/// largest was 132 (seed 5275, step 124, an attack by core-061), so this keeps a margin of about 15×.
/// Seed 6540's Joint Filing (meditative-009) `extra` reached 1,534 at step 98 and 6,142 at step 104,
/// and R884000's University an Activate N of 33,724.
pub const GROWTH_NUMBER_BOUND: i64 = 2_000;

/// The most card instances both sides may hold after one action, in every zone. Set from the same
/// sweep: the most was 334 (seed 3320, step 311), so this keeps a margin of about 6×. A fusion counts
/// as one instance whatever it was made from (R874000 bounds its size with `FUSED_CARDS_CAP`), so this
/// measure catches a chain that adds cards without end, not one card that grows.
pub const GROWTH_INSTANCES_BOUND: i64 = 2_000;

/// The most events one Duos action may emit. Measured 2026-10-20 over the release Duos wave
/// (seeds 1–200): the most one passing game's action emitted was 157 (seed 63, step 263), so this
/// keeps a margin of about 63×. A round-close legitimately settles two duels, so expect about twice
/// the solo traffic here; no bound/6 breach, so the solo-matching starting value stands.
pub const DUOS_GROWTH_EVENTS_BOUND: i64 = 10_000;

/// The largest number any card instance may carry after one Duos action, in either duel. Measured
/// 2026-10-20 over the same wave: the largest was 54 (seed 29, step 305), so this keeps a margin of
/// about 37×; no bound/6 breach, so the solo-matching starting value stands.
pub const DUOS_GROWTH_NUMBER_BOUND: i64 = 2_000;

/// The most card instances both duels may hold after one Duos action, in every zone. Measured
/// 2026-10-20 over the same wave: the most was 275 (seed 63, step 271), so this keeps a margin of
/// about 7×; no bound/6 breach, so the solo-matching starting value stands.
pub const DUOS_GROWTH_INSTANCES_BOUND: i64 = 2_000;

/// How many of a measure's heaviest actions the summary and `--json` list.
pub const HEAVIEST_SHOWN: usize = 5;

/// What one action is weighed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Measure {
    /// The events `reduce` returned.
    Events,
    /// The largest number on any card instance afterwards.
    Largest,
    /// The card instances afterwards, both sides, every zone.
    Instances,
}

/// Every measure, in the order `weigh` returns them.
pub(crate) const MEASURES: [Measure; 3] = [Measure::Events, Measure::Largest, Measure::Instances];

impl Measure {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Measure::Events => "events",
            Measure::Largest => "largest",
            Measure::Instances => "instances",
        }
    }

    fn bound(self) -> i64 {
        match self {
            Measure::Events => GROWTH_EVENTS_BOUND,
            Measure::Largest => GROWTH_NUMBER_BOUND,
            Measure::Instances => GROWTH_INSTANCES_BOUND,
        }
    }

    fn bound_name(self) -> &'static str {
        match self {
            Measure::Events => "GROWTH_EVENTS_BOUND",
            Measure::Largest => "GROWTH_NUMBER_BOUND",
            Measure::Instances => "GROWTH_INSTANCES_BOUND",
        }
    }

    /// The Duos wave's bound for this measure (`DUOS_GROWTH_*_BOUND`).
    fn duos_bound(self) -> i64 {
        match self {
            Measure::Events => DUOS_GROWTH_EVENTS_BOUND,
            Measure::Largest => DUOS_GROWTH_NUMBER_BOUND,
            Measure::Instances => DUOS_GROWTH_INSTANCES_BOUND,
        }
    }
}

/// One action's weight by one measure.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Heavy {
    pub(crate) seed: u32,
    /// The action's 1-based place in the game's log.
    pub(crate) step: usize,
    /// The action's wire name (`ActionType`).
    pub(crate) action: String,
    /// The def id of the card that acted, for an action with one.
    pub(crate) card: Option<String>,
    pub(crate) value: i64,
}

/// The heaviest actions by each measure, heaviest first, ties to the lower seed and then the lower step.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Heaviest {
    events: Vec<Heavy>,
    largest: Vec<Heavy>,
    instances: Vec<Heavy>,
}

impl Heaviest {
    pub(crate) fn list(&self, measure: Measure) -> &[Heavy] {
        match measure {
            Measure::Events => &self.events,
            Measure::Largest => &self.largest,
            Measure::Instances => &self.instances,
        }
    }

    /// Adds `heavy` and keeps the `limit` heaviest.
    pub(crate) fn keep(&mut self, measure: Measure, heavy: Heavy, limit: usize) {
        let list = match measure {
            Measure::Events => &mut self.events,
            Measure::Largest => &mut self.largest,
            Measure::Instances => &mut self.instances,
        };
        list.push(heavy);
        list.sort_by(|a, b| {
            b.value
                .cmp(&a.value)
                .then(a.seed.cmp(&b.seed))
                .then(a.step.cmp(&b.step))
        });
        list.truncate(limit);
    }

    /// Folds one game's heaviest actions in, keeping HEAVIEST_SHOWN of each measure.
    pub(crate) fn merge(&mut self, game: &Heaviest) {
        for measure in MEASURES {
            for heavy in game.list(measure) {
                self.keep(measure, heavy.clone(), HEAVIEST_SHOWN);
            }
        }
    }
}

/// Every card instance both sides hold, in every zone: a copy of `side_cards`'s list in
/// `crates/engine/src/state.rs` (private there), in its order.
pub(crate) fn instances_of(state: &GameState) -> impl Iterator<Item = &CardInstance> {
    PLAYER_IDS.into_iter().flat_map(move |player| {
        let side = &state.players[player];
        side.hand
            .iter()
            .chain(side.library.iter())
            .chain(side.graveyard.iter())
            .chain(side.exile.iter())
            .chain(side.units.iter().flatten().flatten())
            .chain(side.backrow.iter().flatten())
            .chain(side.backrow_piles.iter().flatten().flatten())
            .chain(side.carried.iter().flatten().flatten())
            .chain(side.resolving.iter())
    })
}

/// The largest absolute number a card instance carries: its damage, buffs, cost changes, X, exertion
/// count, overridden stats, counters, Brittle count, play count, tuned numbers and every number in its
/// memory. Turn stamps (`summoned_turn`, `frozen`, Brittle's `since`) are left out: they count turns,
/// which the game's own cap bounds.
pub(crate) fn largest_number(card: &CardInstance) -> i64 {
    let mut most = 0_i64;
    let mut see = |n: i32| most = most.max(i64::from(n).abs());
    see(card.damage);
    see(card.buffs.attack);
    see(card.buffs.health);
    see(card.cost_mod);
    card.cost_override.into_iter().for_each(&mut see);
    card.x.into_iter().for_each(&mut see);
    card.exertion.attacks.into_iter().for_each(&mut see);
    if let Some(stats) = &card.stats_override {
        see(stats.attack);
        see(stats.health);
    }
    card.armor_override.into_iter().for_each(&mut see);
    card.counters.plague.into_iter().for_each(&mut see);
    card.counters.grade.into_iter().for_each(&mut see);
    card.brittle.iter().for_each(|brittle| see(brittle.count));
    card.times_played.into_iter().for_each(&mut see);
    if let Some(tuning) = &card.tuning {
        tuning.attack.into_iter().for_each(&mut see);
        tuning.health.into_iter().for_each(&mut see);
        for numbers in [&tuning.x, &tuning.numbers, &tuning.set].into_iter().flatten() {
            numbers.values().for_each(|n| see(*n));
        }
    }
    card.memory.values().map(largest_in_json).fold(most, i64::max)
}

/// The largest absolute number anywhere in a memory value.
pub(crate) fn largest_in_json(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(i64::saturating_abs)
            .or_else(|| number.as_f64().map(|f| f.abs() as i64))
            .unwrap_or(0),
        Value::Array(items) => items.iter().map(largest_in_json).max().unwrap_or(0),
        Value::Object(fields) => fields.values().map(largest_in_json).max().unwrap_or(0),
        _ => 0,
    }
}

/// One action's weights, in `MEASURES`' order: the events `reduce` returned, the largest number on
/// any instance of `state` (the state it produced) and the instances in it.
fn weigh(events: &[GameEvent], state: &GameState) -> [i64; 3] {
    let (largest, instances) = instances_of(state).fold((0_i64, 0_i64), |(largest, count), card| {
        (largest.max(largest_number(card)), count + 1)
    });
    [
        i64::try_from(events.len()).unwrap_or(i64::MAX),
        largest,
        instances,
    ]
}

/// The def id of the card an action is by, read in the state it was chosen in: the instance a
/// play, switch or activation names, or the attacker. Any other action has none.
pub(crate) fn acting_card(state: &GameState, body: &ActionBody) -> Option<String> {
    let id = match body {
        ActionBody::Play { instance_id, .. }
        | ActionBody::SwitchPosition { instance_id }
        | ActionBody::Activate { instance_id, .. }
        | ActionBody::ActivatePower { instance_id, .. } => instance_id,
        ActionBody::Attack { attacker_id, .. } => attacker_id,
        _ => return None,
    };
    find_instance(state, id).map(|card| card.def_id.clone())
}

/// The def id of the first instance carrying `value` as its largest number.
fn holder_of_largest(state: &GameState, value: i64) -> String {
    instances_of(state)
        .find(|card| largest_number(card) == value)
        .map_or_else(|| "no card".to_string(), |card| card.def_id.clone())
}

/// The shared wording of a growth failure: the step, the action and the card that grew, what
/// one measure reached and whose bound it passed. Every arm names its measure (`events`,
/// `largest`, `instances`), so the report says what grew, not just how much.
fn growth_message(
    measure: Measure,
    value: i64,
    bound: (i64, &'static str),
    step: usize,
    action: &str,
    card: Option<&str>,
    holder: &str,
) -> String {
    let by = card.map_or_else(String::new, |id| format!(" by {id}"));
    let what = match measure {
        Measure::Events => format!("emitted {value} events"),
        Measure::Largest => format!("left a largest number of {value} on {holder}"),
        Measure::Instances => format!("left {value} card instances in play"),
    };
    format!(
        "step {step} (\"{action}\"{by}) {what}, past {} ({}): runaway growth inside one action",
        bound.1, bound.0,
    )
}

/// The failure message of the first measure past its bound, when one is: a value at the bound passes.
fn growth_breach(
    weights: &[i64; 3],
    state: &GameState,
    step: usize,
    action: &str,
    card: Option<&str>,
) -> Option<String> {
    let (measure, value) = MEASURES
        .into_iter()
        .zip(weights.iter().copied())
        .find(|(measure, value)| *value > measure.bound())?;
    let holder;
    let holder = match measure {
        Measure::Largest => {
            holder = holder_of_largest(state, value);
            holder.as_str()
        }
        _ => "no card",
    };
    Some(growth_message(
        measure,
        value,
        (measure.bound(), measure.bound_name()),
        step,
        action,
        card,
        holder,
    ))
}

/// One Duos action's weights, in `MEASURES`' order: the `duos_reduce` events, the largest number on
/// any instance of either duel afterwards and the instances in both.
pub(crate) fn weigh_duos(events: &[DuosEvent], state: &DuosState) -> [i64; 3] {
    let (largest, instances) = state
        .duels
        .iter()
        .flat_map(instances_of)
        .fold((0_i64, 0_i64), |(largest, count), card| {
            (largest.max(largest_number(card)), count + 1)
        });
    [
        i64::try_from(events.len()).unwrap_or(i64::MAX),
        largest,
        instances,
    ]
}

/// The def id of the card a Duos action is by, read in the state it was chosen in: a `Duel` action
/// delegates to `acting_card` in the seat's own duel; a pass or a concede names none.
pub(crate) fn duos_acting_card(state: &DuosState, seat: DuosSeat, body: &DuosActionBody) -> Option<String> {
    match body {
        DuosActionBody::Duel { body } => acting_card(&state.duels[seat.duel()], body),
        DuosActionBody::Pass { .. } | DuosActionBody::Concede => None,
    }
}

/// The def id of the first instance carrying `value` as its largest number, across both duels.
fn duos_holder_of_largest(state: &DuosState, value: i64) -> String {
    state
        .duels
        .iter()
        .flat_map(instances_of)
        .find(|card| largest_number(card) == value)
        .map_or_else(|| "no card".to_string(), |card| card.def_id.clone())
}

/// The Duos failure message of the first measure past its `DUOS_GROWTH_*_BOUND`, when one is: a
/// value at the bound passes. Same wording as the gate's, with the Duos bounds.
pub(crate) fn duos_growth_breach(
    weights: &[i64; 3],
    state: &DuosState,
    step: usize,
    action: &str,
    card: Option<&str>,
) -> Option<String> {
    let (measure, value) = MEASURES
        .into_iter()
        .zip(weights.iter().copied())
        .find(|(measure, value)| *value > measure.duos_bound())?;
    let holder;
    let holder = match measure {
        Measure::Largest => {
            holder = duos_holder_of_largest(state, value);
            holder.as_str()
        }
        _ => "no card",
    };
    Some(growth_message(
        measure,
        value,
        (
            measure.duos_bound(),
            match measure {
                Measure::Events => "DUOS_GROWTH_EVENTS_BOUND",
                Measure::Largest => "DUOS_GROWTH_NUMBER_BOUND",
                Measure::Instances => "DUOS_GROWTH_INSTANCES_BOUND",
            },
        ),
        step,
        action,
        card,
        holder,
    ))
}

// ---------------------------------------------------------------------------------------------
// Failure collection and the grouped report
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Failure {
    seed: u32,
    stage: FailureStage,
    signature: String,
    message: String,
    cards: Vec<String>,
    turn: i64,
    actions: i64,
}

/// `\w` for `\b`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `\b` at char `at`.
fn boundary_at(chars: &[char], at: usize) -> bool {
    let before = at > 0 && chars.get(at - 1).is_some_and(|c| is_word_char(*c));
    let after = chars.get(at).is_some_and(|c| is_word_char(*c));
    before != after
}

fn starts_with_at(chars: &[char], at: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, expected)| chars.get(at + offset) == Some(&expected))
}

/// `.replace(/"[^"]*"/g, '"…"')`.
fn blank_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('"') {
            Some(close) => {
                out.push_str("\"…\"");
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// `.replace(/\bcore-[0-9a-z.-]+\b/g, "<card>")`.
fn blank_card_ids(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let in_class = |c: char| c.is_ascii_digit() || c.is_ascii_lowercase() || c == '.' || c == '-';
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        if starts_with_at(&chars, at, "core-") && boundary_at(&chars, at) {
            let from = at + "core-".len();
            let mut end = from;
            while end < chars.len() && in_class(chars[end]) {
                end += 1;
            }
            // `+` is greedy and gives characters back until `\b` holds.
            if let Some(stop) = (from + 1..=end).rev().find(|stop| boundary_at(&chars, *stop)) {
                out.push_str("<card>");
                at = stop;
                continue;
            }
        }
        out.push(chars[at]);
        at += 1;
    }
    out
}

/// `.replace(/\bc\d+\b/g, "<inst>")`.
fn blank_instance_ids(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        if chars[at] == 'c' && boundary_at(&chars, at) {
            let mut end = at + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end > at + 1 && boundary_at(&chars, end) {
                out.push_str("<inst>");
                at = end;
                continue;
            }
        }
        out.push(chars[at]);
        at += 1;
    }
    out
}

/// `.replace(/\b[0-9a-f]{8}\b/g, "<hash>")`.
fn blank_hashes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let hex = |c: &char| c.is_ascii_digit() || ('a'..='f').contains(c);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        if at + 8 <= chars.len()
            && boundary_at(&chars, at)
            && chars[at..at + 8].iter().all(hex)
            && boundary_at(&chars, at + 8)
        {
            out.push_str("<hash>");
            at += 8;
            continue;
        }
        out.push(chars[at]);
        at += 1;
    }
    out
}

/// `.replace(/§\d+(?:\.\d+)*|\bR\d+\b|\d+/g, (match) => (/^\d+$/.test(match) ? "N" : match))`: one
/// pass, in which a §x.y or Rn reference is kept whole and any other run of digits becomes N.
fn blank_numbers(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let digits_from = |from: usize| {
        let mut end = from;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        end
    };
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        // §\d+(?:\.\d+)*
        if chars[at] == '§' && chars.get(at + 1).is_some_and(char::is_ascii_digit) {
            let mut end = digits_from(at + 1);
            while chars.get(end) == Some(&'.') && chars.get(end + 1).is_some_and(char::is_ascii_digit) {
                end = digits_from(end + 1);
            }
            out.extend(&chars[at..end]);
            at = end;
            continue;
        }
        // \bR\d+\b
        if chars[at] == 'R' && boundary_at(&chars, at) && chars.get(at + 1).is_some_and(char::is_ascii_digit)
        {
            let end = digits_from(at + 1);
            if boundary_at(&chars, end) {
                out.extend(&chars[at..end]);
                at = end;
                continue;
            }
        }
        // \d+
        if chars[at].is_ascii_digit() {
            out.push('N');
            at = digits_from(at);
            continue;
        }
        out.push(chars[at]);
        at += 1;
    }
    out
}

/// A stable shape for one bug: the message with the parts that vary between seeds blanked out
/// (instance ids, card ids, hashes, quoted text and bare numbers). 400 seeds tripping over one
/// missing effect collapse to a single group instead of 400 lines of noise. SPEC and ruling
/// references survive the blanking, because "§9.3" and "R81" are the most identifying part of an
/// engine error message.
fn signature_of(stage: FailureStage, message: &str) -> String {
    let shape = blank_numbers(&blank_hashes(&blank_instance_ids(&blank_card_ids(
        &blank_quoted(message),
    ))));
    format!("[{}] {}", stage.as_str(), shape.trim())
}

thread_local! {
    /// Where the last panic on this thread was raised (`file:line:column`), recorded by the hook
    /// `record_panic_locations` installs: Rust's stand-in for TS's error stack.
    static PANIC_AT: RefCell<Option<String>> = const { RefCell::new(None) };
}

static PANIC_HOOK: Once = Once::new();

/// Installs, once per process, a panic hook that records where each panic was raised for
/// `origin_of`, then hands the panic to the hook that was there before (which prints it).
fn record_panic_locations() {
    PANIC_HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let at = info
                .location()
                .map(|location| format!("{}:{}:{}", location.file(), location.line(), location.column()));
            PANIC_AT.with(|cell| *cell.borrow_mut() = at);
            previous(info);
        }));
    });
}

/// The first `crates/` location of a panic: where a raw engine or card-script panic came from (TS:
/// the first `packages/` frame of the stack that is not the fuzz test's own).
fn origin_of(location: Option<&str>) -> String {
    match location {
        Some(at) if at.contains("crates/") && !at.contains("crates/tools/src/fuzz.rs") => {
            format!(" [at {at}]")
        }
        _ => String::new(),
    }
}

/// Counts in first-seen order (a JS `Map`).
fn count_by<'a>(values: impl IntoIterator<Item = &'a str>) -> IndexMap<String, usize> {
    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0) += 1;
    }
    counts
}

/// `Math.round(x)`: .5 rounds up (SURFACE §4.4.4).
fn round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// The failure message BUILD asks for: every card id in a failing seed's decks, grouped so one bug
/// reads as one entry. Per group it prints the seeds, the cards present in EVERY failing seed of the
/// group (the narrowest suspects), the most frequent cards, and the full 40-card deck pair of the
/// first seed, which is the one a reader reproduces with.
fn report(failures: &[Failure], ran: u32, elapsed_ms: f64) -> String {
    let mut groups: IndexMap<String, Vec<&Failure>> = IndexMap::new();
    for failure in failures {
        groups.entry(failure.signature.clone()).or_default().push(failure);
    }
    let mut ordered: Vec<(String, Vec<&Failure>)> = groups.into_iter().collect();
    ordered.sort_by_key(|group| std::cmp::Reverse(group.1.len()));

    let deck_size = usize::try_from(DECK_SIZE).unwrap_or(0);
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "M4 fuzz gate: {} of {ran} seeds failed ({} distinct failure signature(s), {}s).",
        failures.len(),
        ordered.len(),
        round(elapsed_ms / MS_PER_SECOND)
    ));
    lines.push(format!(
        "Pool: {} deck-legal cards, {} excluded.",
        FUZZ_POOL.len(),
        POOL_EXCLUSIONS.len()
    ));
    lines.push(String::new());

    for (index, (signature, bucket)) in ordered.iter().enumerate() {
        let seeds: Vec<String> = bucket.iter().map(|failure| failure.seed.to_string()).collect();
        let first = bucket[0];
        let counts = count_by(
            bucket
                .iter()
                .flat_map(|failure| failure.cards.iter().map(String::as_str)),
        );
        let mut always: Vec<String> = counts
            .iter()
            .filter(|(_, n)| **n == bucket.len())
            .map(|(id, _)| id.clone())
            .collect();
        always.sort();
        let mut frequent: Vec<(&String, &usize)> = counts.iter().collect();
        frequent.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        frequent.truncate(10);

        lines.push(format!(
            "--- signature {}/{} — {} seed(s) ---",
            index + 1,
            ordered.len(),
            bucket.len()
        ));
        lines.push(format!("  {signature}"));
        lines.push(format!(
            "  example: seed {} (turn {}, {} actions)",
            first.seed, first.turn, first.actions
        ));
        lines.push(format!("  message: {}", first.message));
        let shown: Vec<String> = seeds.iter().take(40).cloned().collect();
        let more = if seeds.len() > 40 {
            format!(", … ({} total)", seeds.len())
        } else {
            String::new()
        };
        lines.push(format!("  seeds: {}{more}", shown.join(", ")));
        if bucket.len() > 1 {
            lines.push(if always.is_empty() {
                "  in EVERY failing seed of this group: none — the cause is shared, not a single card"
                    .to_string()
            } else {
                format!(
                    "  in EVERY failing seed of this group ({}): {}",
                    always.len(),
                    always
                        .iter()
                        .map(|id| describe_card(id))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            });
            lines.push(format!(
                "  most frequent: {}",
                frequent
                    .iter()
                    .map(|(id, n)| format!("{id}×{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let split = deck_size.min(first.cards.len());
        lines.push(format!(
            "  seed {} p1 deck: {}",
            first.seed,
            first.cards[..split].join(", ")
        ));
        lines.push(format!(
            "  seed {} p2 deck: {}",
            first.seed,
            first.cards[split..].join(", ")
        ));
        lines.push(String::new());
    }

    let mut all_cards: Vec<String> = failures
        .iter()
        .flat_map(|failure| failure.cards.iter().cloned())
        .collect::<IndexSet<String>>()
        .into_iter()
        .collect();
    all_cards.sort();
    lines.push(format!(
        "Cards appearing in at least one failing seed ({}):",
        all_cards.len()
    ));
    lines.push(format!("  {}", all_cards.join(", ")));
    lines.push(String::new());
    lines.push("Reproduce one seed with: cargo jackioh fuzz --from <seed> --seeds 1".to_string());
    lines.join("\n")
}

// ---------------------------------------------------------------------------------------------
// The gate
// ---------------------------------------------------------------------------------------------

/// One seed's passing game, for the wave's numbers.
struct Passed {
    /// The final state's hash (§9.3), for --regress's lines.
    hash: String,
    reason: String,
    actions: usize,
    turn: i32,
    /// The game's heaviest action by each measure (#1254).
    heaviest: Heaviest,
}

/// One seed through the four assertions: never panic, always terminate by hero death or the R2 cap,
/// and fold `(seed, log)` to the same hash; a failure names every card in this seed's decks.
fn fuzz_seed(seed: u32) -> Result<Passed, Failure> {
    fuzz_deal(seed, decks_for_seed(seed))
}

/// One game through the four assertions: `seed` names the game's and the policy's streams, `decks` the
/// deal.
fn fuzz_deal(seed: u32, decks: (Vec<String>, Vec<String>)) -> Result<Passed, Failure> {
    // R1420: the fuzz pool is the whole catalog, every set shipped or not, so this worker thread
    // previews every set for the game, its checks and its replay.
    let _preview = preview_every_set();
    let mut reached: Option<At> = None;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<Passed, FuzzFailure> {
        // 1. never panic
        let run = play_game(seed, decks.clone())?;
        reached = Some(At {
            turn: run.state.turn,
            actions: run.log.len(),
        });
        // 2. always terminate, by hero death or the R2 cap
        check_termination(&run)?;
        // 3. (seed, log) folds to the same hash
        check_replay(seed, &run)?;
        Ok(Passed {
            hash: hash_state(&run.state),
            reason: run
                .state
                .result
                .map_or_else(|| "none".to_string(), |result| result.reason.as_str().to_string()),
            actions: run.log.len(),
            turn: run.state.turn,
            heaviest: run.heaviest,
        })
    }));
    let (stage, message, turn, actions) = match caught {
        Ok(Ok(passed)) => return Ok(passed),
        Ok(Err(failure)) => (
            failure.stage,
            failure.message,
            i64::from(failure.at.turn),
            i64::try_from(failure.at.actions).unwrap_or(i64::MAX),
        ),
        Err(payload) => {
            // A raw panic out of the engine or a card script: the message alone rarely says which file
            // it came from, so carry where it was raised into the report.
            let location = PANIC_AT.with(|cell| cell.borrow_mut().take());
            (
                FailureStage::Play,
                format!(
                    "panic: {}{}",
                    panic_message(&*payload),
                    origin_of(location.as_deref())
                ),
                reached.map_or(-1, |at| i64::from(at.turn)),
                reached.map_or(-1, |at| i64::try_from(at.actions).unwrap_or(i64::MAX)),
            )
        }
    };
    // 4. name every card in this seed's decks
    let (mut cards, p2) = decks;
    cards.extend(p2);
    Err(Failure {
        seed,
        stage,
        signature: signature_of(stage, &message),
        message,
        cards,
        turn,
        actions,
    })
}

/// A whole wave's results, in seed order.
struct Wave {
    from: u32,
    ran: u32,
    failures: Vec<Failure>,
    reasons: IndexMap<String, usize>,
    actions_total: usize,
    actions_max: usize,
    turn_max: i32,
    /// The heaviest actions of the games that passed, one per game (#1254): a failed seed adds nothing.
    heaviest: Heaviest,
    elapsed_ms: f64,
}

/// Plays seeds `from..from + ran`, in parallel, and gathers them back in seed order.
fn play_wave(from: u32, ran: u32) -> Wave {
    record_panic_locations();
    register_all();
    let started_at = Instant::now();
    let results: Vec<Result<Passed, Failure>> = (from..from + ran).into_par_iter().map(fuzz_seed).collect();
    let elapsed_ms = started_at.elapsed().as_secs_f64() * MS_PER_SECOND;

    let mut wave = Wave {
        from,
        ran,
        failures: Vec::new(),
        reasons: IndexMap::new(),
        actions_total: 0,
        actions_max: 0,
        turn_max: 0,
        heaviest: Heaviest::default(),
        elapsed_ms,
    };
    for result in results {
        match result {
            Ok(passed) => {
                *wave.reasons.entry(passed.reason).or_insert(0) += 1;
                wave.actions_total += passed.actions;
                wave.actions_max = wave.actions_max.max(passed.actions);
                wave.turn_max = wave.turn_max.max(passed.turn);
                wave.heaviest.merge(&passed.heaviest);
            }
            Err(failure) => wave.failures.push(failure),
        }
    }
    wave
}

impl Wave {
    /// REVIEW B8 asks the fuzz run to REPORT its numbers — seeds, panics, hash mismatches, games
    /// over the cap — and not merely to pass: a green gate with no numbers is not evidence.
    fn summary(&self) -> String {
        let passed = self.ran as usize - self.failures.len();
        let by_stage = count_by(self.failures.iter().map(|failure| failure.stage.as_str()));
        let stage = |name: &str| by_stage.get(name).copied().unwrap_or(0);
        let endings = self
            .reasons
            .iter()
            .map(|(reason, n)| format!("{reason}={n}"))
            .collect::<Vec<_>>()
            .join(" ");
        let average = if passed > 0 {
            round(self.actions_total as f64 / passed as f64)
        } else {
            0
        };
        let mut text = format!(
            "[M4 fuzz] {}\n  seeds {}..{}: {passed} passed, {} failed\n  {} panic(s), {} over the {}-action bound or stalled, {} replay mismatch(es), {} invariant violation(s), {} past a growth bound\n  endings: {}\n  {average} actions/game on average, {} at most; longest game ended on turn {} of {TURN_CAP_PLAYER_TURNS}\n  pool: {} deck-legal cards, {} excluded; I6 every {} state(s), I8 every {} state(s); {}s\n",
            if self.ran >= WAVE_SEEDS {
                "FULL GATE".to_string()
            } else {
                format!("SHORT WAVE (the gate is {WAVE_SEEDS} seeds: cargo jackioh fuzz)")
            },
            self.from,
            self.from + self.ran - 1,
            self.failures.len(),
            stage("play"),
            stage("termination"),
            max_actions_per_game(),
            stage("replay"),
            stage("invariant"),
            stage("growth"),
            if endings.is_empty() {
                "none".to_string()
            } else {
                endings
            },
            self.actions_max,
            self.turn_max,
            FUZZ_POOL.len(),
            POOL_EXCLUSIONS.len(),
            hidden_stride(),
            unlisted_stride(),
            round(self.elapsed_ms / MS_PER_SECOND),
        );
        text.push_str(&heaviest_lines(&self.heaviest));
        text
    }

    /// `--json`: the wave's failures, each measure's heaviest actions and the growth bounds (#1254).
    fn json(&self) -> Value {
        json!({
            "from": self.from,
            "ran": self.ran,
            "passed": self.ran as usize - self.failures.len(),
            "failures": self.failures.iter().map(|failure| json!({
                "seed": failure.seed,
                "stage": failure.stage.as_str(),
                "turn": failure.turn,
                "step": failure.actions,
                "message": failure.message,
            })).collect::<Vec<_>>(),
            "heaviest": self.heaviest,
            "bounds": MEASURES
                .into_iter()
                .map(|measure| (measure.as_str(), measure.bound()))
                .collect::<IndexMap<_, _>>(),
        })
    }
}

/// The summary's lines for each measure's heaviest actions (#1254), with each bound read off
/// `bound_of`: the gate's below, the Duos wave's in `duos_heaviest_lines`.
fn heaviest_lines_with(heaviest: &Heaviest, bound_of: impl Fn(Measure) -> i64) -> String {
    let mut text = "  heaviest actions, one per game (seed/step action card: weight):\n".to_string();
    for measure in MEASURES {
        let entries = heaviest
            .list(measure)
            .iter()
            .map(|heavy| {
                format!(
                    "{}/{} {} {}: {}",
                    heavy.seed,
                    heavy.step,
                    heavy.action,
                    heavy.card.as_deref().unwrap_or("-"),
                    heavy.value
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        text.push_str(&format!(
            "    {} (bound {}): {}\n",
            measure.as_str(),
            bound_of(measure),
            if entries.is_empty() { "none" } else { &entries }
        ));
    }
    text
}

/// The summary's lines for each measure's heaviest actions (#1254).
fn heaviest_lines(heaviest: &Heaviest) -> String {
    heaviest_lines_with(heaviest, Measure::bound)
}

/// The Duos summary's lines for each measure's heaviest actions (#1364): the same shape as the
/// gate's, with the `DUOS_GROWTH_*_BOUND` values.
pub(crate) fn duos_heaviest_lines(heaviest: &Heaviest) -> String {
    heaviest_lines_with(heaviest, Measure::duos_bound)
}

// ---------------------------------------------------------------------------------------------
// The regression corpus (#1197)
// ---------------------------------------------------------------------------------------------

/// Where the kept failures live: one JSON file per record (`fuzz-corpus/README.md`).
pub const CORPUS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fuzz-corpus");

/// One kept fuzz failure: the seed and the exact decks it was dealt, so a later card in `FUZZ_POOL`
/// cannot redeal it.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CorpusRecord {
    /// Names the game's and the policy's streams (`jackioh-fuzz-<seed>`).
    pub seed: u32,
    /// What `cargo jackioh catalog-version` printed on the day it failed.
    pub catalog_version: String,
    /// p1's deck then p2's, as the report's `seed N p1 deck:` / `p2 deck:` lines print them.
    pub decks: (Vec<String>, Vec<String>),
    /// Which run failed, how, and the issue that fixed or filed it.
    pub found: String,
    /// Which ids left the catalog: a retired record is listed and not replayed, never edited.
    #[serde(default)]
    pub retired: Option<String>,
}

/// One line per problem with `record`: its shape, then its ids against the live catalog.
fn lint_record(record: &CorpusRecord) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    if record.seed == 0 {
        problems.push("seed must be 1 or more".to_string());
    }
    if !shipped_versions().contains(&record.catalog_version.as_str()) {
        problems.push(format!(
            "catalogVersion \"{}\" is no version in crates/cards/patches/patches.json",
            record.catalog_version
        ));
    }
    if record.found.trim().is_empty() {
        problems.push("found is empty: say which run failed, how, and the issue".to_string());
    }
    let decks = [("p1", &record.decks.0), ("p2", &record.decks.1)];
    let deck_size = usize::try_from(DECK_SIZE).unwrap_or(0);
    for (label, deck) in decks {
        if deck.len() != deck_size {
            problems.push(format!("{label} deck holds {} ids, not {DECK_SIZE}", deck.len()));
        }
    }
    let ids: Vec<&str> = decks
        .iter()
        .flat_map(|(_, deck)| deck.iter().map(String::as_str))
        .collect();
    if ids.iter().collect::<IndexSet<_>>().len() < ids.len() {
        problems.push("an id appears twice across the decks: one shuffle deals distinct cards".to_string());
    }
    match &record.retired {
        Some(why) if why.trim().is_empty() => {
            problems.push("retired is empty: say which ids left the catalog".to_string());
        }
        Some(_) => {}
        None => {
            for id in ids {
                if !CATALOG.contains_key(id) {
                    problems.push(format!(
                        "\"{id}\" names no catalog card: mark the record retired, never edit its decks"
                    ));
                }
            }
            if problems.is_empty() {
                // R1420: the live catalog with every set previewed, the way `fuzz_deal` deals.
                let _preview = preview_every_set();
                for (label, deck) in decks {
                    if let Err(error) = validate_deck(deck, &CATALOG, label, DECK_SIZE) {
                        problems.push(error.message);
                    }
                }
            }
        }
    }
    problems
}

/// A corpus file's name and the record in it.
type NamedRecord = (String, CorpusRecord);

/// Every `.json` file of `dir`, sorted by name, parsed; with every problem found, a file that is no
/// record included, one line each, led by the file's name.
fn read_corpus(dir: &Path) -> anyhow::Result<(Vec<NamedRecord>, Vec<String>)> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .with_context(|| format!("reading {}", dir.display()))?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "json"));
    paths.sort();
    let mut records = Vec::new();
    let mut problems = Vec::new();
    for path in paths {
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        match serde_json::from_str::<CorpusRecord>(&text) {
            Ok(record) => {
                problems.extend(
                    lint_record(&record)
                        .into_iter()
                        .map(|problem| format!("{name}: {problem}")),
                );
                records.push((name, record));
            }
            Err(error) => problems.push(format!("{name}: not a corpus record: {error}")),
        }
    }
    Ok((records, problems))
}

/// `record` through the gate's four assertions with its pinned decks, never a deal from the live pool.
fn replay_record(record: &CorpusRecord) -> Result<Passed, Failure> {
    fuzz_deal(record.seed, record.decks.clone())
}

/// What `--regress` found: a line per record that passed or is retired, and every problem.
struct Regress {
    lines: Vec<String>,
    failed: Vec<String>,
}

/// Lints every record of `dir`, then, when all are sound, replays each unretired one in parallel.
fn regress(dir: &Path) -> anyhow::Result<Regress> {
    record_panic_locations();
    register_all();
    let (records, problems) = read_corpus(dir)?;
    if !problems.is_empty() {
        return Ok(Regress {
            lines: Vec::new(),
            failed: problems,
        });
    }
    let live: Vec<&NamedRecord> = records.iter().filter(|entry| entry.1.retired.is_none()).collect();
    let mut results = live
        .par_iter()
        .map(|entry| replay_record(&entry.1))
        .collect::<Vec<_>>()
        .into_iter();
    let mut out = Regress {
        lines: Vec::new(),
        failed: Vec::new(),
    };
    for (name, record) in &records {
        let CorpusRecord {
            seed,
            catalog_version,
            ..
        } = record;
        if let Some(why) = &record.retired {
            out.lines
                .push(format!("{name}: seed {seed} ({catalog_version}): retired: {why}"));
            continue;
        }
        match results.next() {
            Some(Ok(passed)) => out.lines.push(format!(
                "{name}: seed {seed} ({catalog_version}): passed, {} on turn {}, {} actions, hash {}",
                passed.reason, passed.turn, passed.actions, passed.hash
            )),
            Some(Err(failure)) => out.failed.push(format!(
                "{name}: seed {seed} ({catalog_version}) failed [{}] at turn {}, {} actions: {}\n  found: {}",
                failure.stage.as_str(),
                failure.turn,
                failure.actions,
                failure.message,
                record.found
            )),
            None => {}
        }
    }
    Ok(out)
}

/// `cargo jackioh fuzz --regress`: prints each record's line and the totals, and fails on any problem.
fn run_regress() -> anyhow::Result<()> {
    let regress = regress(Path::new(CORPUS_DIR))?;
    for line in &regress.lines {
        println!("{line}");
    }
    println!(
        "[fuzz regress] {} record(s) passed or retired, {} problem(s)",
        regress.lines.len(),
        regress.failed.len()
    );
    if !regress.failed.is_empty() {
        println!("{}", regress.failed.join("\n"));
        bail!("fuzz regress: {} problem(s)", regress.failed.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The handicapped wave (fuzz-handicap.test.ts)
// ---------------------------------------------------------------------------------------------

/// Every deck-legal card, sorted, as the gate's FUZZ_POOL (its exclusion list is empty): every set,
/// shipped or not (R1420).
static HANDICAP_POOL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.is_token())
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    pool
});

/// R1420: `HANDICAP_POOL` with the sets that have not shipped left out, for the golden traces.
pub static SHIPPED_HANDICAP_POOL: LazyLock<Vec<String>> = LazyLock::new(|| shipped_only(&HANDICAP_POOL));

/// Which seat is handicapped, and how.
#[derive(Clone, Copy, Debug)]
pub struct SeedHandicap {
    pub seat: PlayerId,
    pub tier: Difficulty,
    pub handicap: Handicap,
}

/// Which seat is handicapped, and how: seeds rotate over Medium and Hard and over p1 and p2.
pub fn handicap_for_seed(seed: u32) -> SeedHandicap {
    let tier = if seed % 2 == 1 {
        Difficulty::Hard
    } else {
        Difficulty::Medium
    };
    let seat = if (seed / 2).is_multiple_of(2) {
        PlayerId::P1
    } else {
        PlayerId::P2
    };
    SeedHandicap {
        seat,
        tier,
        handicap: AI_DIFFICULTY[tier],
    }
}

/// One shuffle of the pool: the handicapped seat takes its deckSize cards, the other seat the next 20
/// (TS `decksForSeed` of fuzz-handicap.test.ts, renamed beside the gate's own).
pub(crate) fn handicap_decks_for_seed(
    seed: u32,
    seat: PlayerId,
    handicap: &Handicap,
) -> (Vec<String>, Vec<String>) {
    handicap_decks_for_seed_from(seed, seat, handicap, &HANDICAP_POOL)
}

/// `handicap_decks_for_seed` over another pool: the golden traces deal from `SHIPPED_HANDICAP_POOL`.
pub(crate) fn handicap_decks_for_seed_from(
    seed: u32,
    seat: PlayerId,
    handicap: &Handicap,
    pool: &[String],
) -> (Vec<String>, Vec<String>) {
    let shuffled = Rng::new(&format!("jackioh-fuzz-handicap-decks-{seed}"), 0).shuffle(pool);
    let big_size = usize::try_from(handicap.deck_size).unwrap_or(0);
    let small_size = usize::try_from(DECK_SIZE).unwrap_or(0);
    let big = shuffled[..big_size].to_vec();
    let small = shuffled[big_size..big_size + small_size].to_vec();
    match seat {
        PlayerId::P1 => (big, small),
        PlayerId::P2 => (small, big),
    }
}

#[derive(Clone, Debug)]
struct Outcome {
    seed: u32,
    error: Option<String>,
    tier: Difficulty,
    seat: PlayerId,
    reason: Option<String>,
}

fn play_seed(seed: u32) -> Outcome {
    // R1420: as `fuzz_seed`, every set is previewed for the game on this worker thread.
    let _preview = preview_every_set();
    let SeedHandicap { seat, tier, handicap } = handicap_for_seed(seed);
    let game_seed = format!("jackioh-fuzz-handicap-{seed}");
    let decks = handicap_decks_for_seed(seed, seat, &handicap);
    let mut handicaps: PerPlayerOpt<Handicap> = PerPlayerOpt::default();
    *handicaps.slot(seat) = Some(handicap);
    let fail = |error: String| Outcome {
        seed,
        error: Some(error),
        tier,
        seat,
        reason: None,
    };

    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<String, String> {
        register_all();
        let begun = begin_game(&create_game(&CreateGameOptions {
            seed: game_seed.clone(),
            decks: decks.clone(),
            handicaps: Some(handicaps.clone()),
            ..CreateGameOptions::default()
        }));
        let mut state = begun.state;
        let mut policy = Rng::new(&format!("jackioh-fuzz-handicap-policy-{seed}"), 0);
        // R265: while both mulligans are open either seat may answer first; a stream of its own picks
        // which, so the fuzz plays both orders (the game is the same either way, R265).
        let mut order = Rng::new(&format!("jackioh-fuzz-handicap-policy-order-{seed}"), 0);
        let mut log: Vec<Action> = Vec::new();
        let mut monitor = create_invariant_monitor(&state);
        let stride = if is_whole_suite_sweep() { 1 } else { I6_GATE_STRIDE };
        let unlisted_stride = unlisted_stride();
        let max_actions = max_actions_per_game();
        let mut setup = monitor.after(&begun.events, &state);
        setup.extend(monitor.hidden(&state));
        if let Some(first) = setup.first() {
            return Err(format!("invariant at setup: {first}"));
        }

        while state.result.is_none() {
            if log.len() >= max_actions {
                return Err(format!("no ending within {max_actions} actions"));
            }
            let player = actor_of(&state, &mut order);
            let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
                return Err(format!("no legal action for {player} on turn {}", state.turn));
            };
            let unsound = monitor.before(&state, player, &chosen);
            let action = Action::new(chosen, player, format!("fuzz-{}", log.len()));
            if let Some(first) = unsound.first() {
                return Err(format!("invariant on turn {}: {first}", state.turn));
            }
            let result = reduce(&state, &action);
            if let Some(error) = &result.error {
                return Err(format!(
                    "legalActions offered \"{}\" but reduce refused it: {error}",
                    action.action_type().as_str()
                ));
            }
            log.push(action);
            state = result.state;
            let mut broken = monitor.after(&result.events, &state);
            if log.len().is_multiple_of(stride) || state.result.is_some() {
                broken.extend(monitor.hidden(&state));
            }
            if log.len().is_multiple_of(unlisted_stride) {
                broken.extend(monitor.unlisted(&state));
            }
            if let Some(first) = broken.first() {
                return Err(format!("invariant after action {}: {first}", log.len()));
            }
        }

        let Some(ending) = state.result else {
            return Err("the game loop exited with no result".to_string());
        };
        if !TERMINAL_REASONS.contains(&ending.reason.as_str()) {
            return Err(format!("ended with reason \"{}\"", ending.reason));
        }
        if state.turn > TURN_CAP_PLAYER_TURNS {
            return Err(format!("reached turn {}, past the cap", state.turn));
        }

        // R180, R187: the handicaps are part of what a game replays from.
        let replayed = fold(&FoldArgs {
            seed: game_seed.clone(),
            decks: decks.clone(),
            log: log.clone(),
            catalog: None,
            handicaps: Some(handicaps.clone()),
            dealt: None,
            last_boards: None,
            glitch_boards: None,
            ..Default::default()
        });
        if !replayed.errors.is_empty() {
            return Err(format!(
                "replay rejected {} action(s): {}",
                replayed.errors.len(),
                replayed
                    .errors
                    .first()
                    .map_or(String::new(), |first| first.error.clone())
            ));
        }
        if hash_state(&replayed.state) != hash_state(&state) {
            return Err("replay hash differs from the live hash".to_string());
        }
        if serde_json::to_string(&replayed.state.result).unwrap_or_default()
            != serde_json::to_string(&Some(ending)).unwrap_or_default()
        {
            return Err("replay reached a different ending".to_string());
        }

        Ok(ending.reason.as_str().to_string())
    }));
    match caught {
        Ok(Ok(reason)) => Outcome {
            seed,
            error: None,
            tier,
            seat,
            reason: Some(reason),
        },
        Ok(Err(error)) => fail(error),
        Err(payload) => fail(format!("threw: {}", panic_message(&*payload))),
    }
}

/// The handicapped wave's outcomes, in seed order.
struct HandicapWave {
    from: u32,
    size: u32,
    outcomes: Vec<Outcome>,
}

fn play_handicap_wave(from: u32, size: u32) -> HandicapWave {
    record_panic_locations();
    register_all();
    let outcomes: Vec<Outcome> = (from..from + size).into_par_iter().map(play_seed).collect();
    HandicapWave { from, size, outcomes }
}

impl HandicapWave {
    fn failure_lines(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter_map(|outcome| {
                outcome.error.as_ref().map(|error| {
                    format!(
                        "seed {} ({} {}): {error}",
                        outcome.seed, outcome.tier, outcome.seat
                    )
                })
            })
            .collect()
    }

    fn summary(&self) -> String {
        let failed = self
            .outcomes
            .iter()
            .filter(|outcome| outcome.error.is_some())
            .count();
        let mut endings: IndexMap<String, usize> = IndexMap::new();
        for outcome in &self.outcomes {
            if let Some(reason) = &outcome.reason {
                *endings.entry(reason.clone()).or_insert(0) += 1;
            }
        }
        format!(
            "[fuzz-handicap] {} seed(s) from {}: {failed} failed; endings {}",
            self.size,
            self.from,
            serde_json::to_string(&endings).unwrap_or_default()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::{GameOverReason, GameResult, Winner};

    mod fuzz_m4_gate {
        use super::*;

        #[test]
        fn draws_both_decks_from_the_seed_alone() {
            let deck_size = usize::try_from(DECK_SIZE).unwrap_or(0);
            assert!(FUZZ_POOL.len() >= deck_size * 2);
            for seed in [1, 500, 1000] {
                let (p1, p2) = decks_for_seed(seed);
                assert_eq!(
                    decks_for_seed(seed),
                    (p1.clone(), p2.clone()),
                    "seed {seed} must redraw identically"
                );
                assert_eq!(p1.len(), deck_size);
                assert_eq!(p2.len(), deck_size);
                // §2.6 L3: no duplicate ids inside a deck, and one shuffle keeps the pair disjoint too.
                let distinct: IndexSet<&String> = p1.iter().chain(p2.iter()).collect();
                assert_eq!(distinct.len(), deck_size * 2);
                for id in p1.iter().chain(p2.iter()) {
                    assert!(FUZZ_POOL.contains(id), "{id}");
                }
            }
            // Different seeds must give different decks, or the "random decks" of the gate are a fiction.
            assert_ne!(decks_for_seed(1), decks_for_seed(2));
        }

        #[test]
        fn seeds_widens_the_wave_past_the_gate_as_the_daily_run_asks() {
            // super.yml plays each quarter of its 10,000 seeds as `--from <q> --seeds 2500` (#659).
            assert_eq!(wave_size(Some(2500)), 2500);
            assert_eq!(wave_size(Some(WAVE_SEEDS)), WAVE_SEEDS);
            assert_eq!(wave_size(Some(1)), 1);
            assert_eq!(wave_size(Some(0)), SWEEP_SEEDS);
            assert_eq!(wave_size(None), SWEEP_SEEDS);
        }

        #[test]
        fn a_wave_of_at_least_the_gates_seeds_reports_as_the_full_gate() {
            let wave = |ran| Wave {
                from: 5001,
                ran,
                failures: Vec::new(),
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest: Heaviest::default(),
                elapsed_ms: 0.0,
            };
            assert!(wave(2500).summary().starts_with("[M4 fuzz] FULL GATE\n"));
            assert!(wave(WAVE_SEEDS).summary().starts_with("[M4 fuzz] FULL GATE\n"));
            assert!(wave(200).summary().starts_with("[M4 fuzz] SHORT WAVE"));
        }

        #[test]
        fn plays_the_seeded_games_to_a_result_and_replays_each_to_the_same_hash_short_wave() {
            let ran = wave_size(None);
            let wave = play_wave(first_seed(None), ran);
            print!("{}", wave.summary());
            let detail = if wave.failures.is_empty() {
                String::new()
            } else {
                report(&wave.failures, ran, wave.elapsed_ms)
            };
            assert_eq!(wave.failures.len(), 0, "{detail}");
            // #1254: each measure lists the wave's heaviest actions, one per game.
            for measure in MEASURES {
                let list = wave.heaviest.list(measure);
                assert_eq!(
                    list.len(),
                    HEAVIEST_SHOWN.min(ran as usize),
                    "{}",
                    measure.as_str()
                );
                let seeds: IndexSet<u32> = list.iter().map(|heavy| heavy.seed).collect();
                assert_eq!(
                    seeds.len(),
                    list.len(),
                    "{}: a game listed twice",
                    measure.as_str()
                );
            }
        }

        #[test]
        fn a_failure_signature_blanks_what_varies_between_seeds_and_keeps_spec_and_ruling_references() {
            assert_eq!(
                signature_of(
                    FailureStage::Play,
                    "legalActions offered \"play\" but reduce refused it: c12 core-043 a798906b R81 §9.3 needs 3"
                ),
                "[play] legalActions offered \"…\" but reduce refused it: <inst> <card> <hash> R81 §9.3 needs N"
            );
            assert_eq!(
                signature_of(FailureStage::Replay, "  turn 12/60, R2x and c3a  "),
                "[replay] turn N/N, RNx and cNa"
            );
            assert_eq!(
                origin_of(Some("crates/engine/src/zones.rs:10:5")),
                " [at crates/engine/src/zones.rs:10:5]"
            );
            assert_eq!(origin_of(Some("crates/tools/src/fuzz.rs:10:5")), "");
            assert_eq!(origin_of(None), "");
        }

        #[test]
        fn r679_a_glitch_that_voids_the_match_is_a_real_ending() {
            register_all();
            // R1420: the fuzz pool holds the sets that have not shipped, so the decks it deals are
            // legal only while every set is previewed, as `fuzz_seed` previews them.
            let _preview = preview_every_set();
            let decks = decks_for_seed(1);
            let mut state = create_game(&CreateGameOptions {
                seed: "jackioh-fuzz-1".to_string(),
                decks: decks.clone(),
                ..CreateGameOptions::default()
            });
            state.result = Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::Voided,
            });
            state.phase = Phase::Over;
            let run = GameRun {
                state,
                log: Vec::new(),
                decks,
                heaviest: Heaviest::default(),
            };
            assert_eq!(check_termination(&run).err().map(|failure| failure.message), None);
        }
    }

    mod fuzz_growth_stage {
        use super::*;
        use jackioh_engine::testkit::{
            CardScripts, GameEventType, Script, TriggerDef, clear_overrides, effects, json_as,
            register_catalog_as, register_scripts,
        };
        use jackioh_engine::{CardDef, Tuning};

        /// A fixture Unit that Buffs itself at the start of every turn, University's mistake before
        /// R884000.
        const GROWER: &str = "fuzz-grower";
        /// Each trigger adds this many times the Buffs it already holds, so they run 9, 90, 900, 9,000.
        const GROWER_FACTOR: i32 = 9;
        /// Seeds the planted test tries; the grower is dealt into both decks of each.
        const PLANTED_SEEDS: u32 = 20;

        /// Puts the grower in this thread's catalog and scripts; `clear_overrides` takes it out.
        fn install_grower() {
            register_all();
            let face = json!({ "attack": 1, "health": 1, "keywords": [{ "kind": "Can't attack" }],
                "text": "Can't attack\nAt the start of each turn: this gains nine times its Buffs (at least +9/+9)." });
            let def: CardDef = json_as(
                json!({ "id": GROWER, "index": GROWER, "name": "Grower (fuzz fixture)",
                "set": "Core", "type": "Unit", "tags": [], "rarity": "Common", "token": false, "cost": 1,
                "base": face, "radiant": face }),
            );
            let mut catalog = (*CATALOG).clone();
            catalog.insert(GROWER.to_string(), def);
            register_catalog_as(catalog, jackioh_cards::catalog_version());
            let grow = TriggerDef::new(
                "fuzz-grower-grow",
                &[GameEventType::TurnStarted],
                |ctx, _event| {
                    let Some(gain) = ctx.live_self().map(|me| GROWER_FACTOR * me.buffs.attack.max(1)) else {
                        return vec![];
                    };
                    vec![effects::buff(json_as(
                        json!({ "target": { "of": "self" }, "attack": gain, "health": gain }),
                    ))]
                },
            );
            let script = Script {
                triggers: vec![grow],
                ..Script::default()
            };
            let mut scripts = jackioh_cards::scripts_of();
            scripts.insert(
                GROWER.to_string(),
                CardScripts {
                    base: script.clone(),
                    radiant: script,
                },
            );
            register_scripts(scripts);
        }

        /// `seed`'s deal with the grower first in both decks.
        fn planted_decks(seed: u32) -> (Vec<String>, Vec<String>) {
            let (mut p1, mut p2) = decks_for_seed(seed);
            p1[0] = GROWER.to_string();
            p2[0] = GROWER.to_string();
            (p1, p2)
        }

        #[test]
        fn a_planted_card_that_buffs_itself_each_turn_fails_in_the_growth_stage_naming_seed_and_step() {
            install_grower();
            // The overrides are this thread's, so the seeds run here and not under rayon.
            let failure = (1..=PLANTED_SEEDS)
                .find_map(|seed| fuzz_deal(seed, planted_decks(seed)).err())
                .expect("a card that grows without end must fail one of the planted seeds");
            clear_overrides();
            // The growth stage, so no settle cap or arithmetic overflow panicked first.
            assert_eq!(failure.stage, FailureStage::Growth, "{}", failure.message);
            assert!(
                failure.message.contains(&format!("step {}", failure.actions)),
                "{}",
                failure.message
            );
            assert!(failure.message.contains(GROWER), "{}", failure.message);
            assert!(
                failure.message.contains("GROWTH_NUMBER_BOUND"),
                "{}",
                failure.message
            );
            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(
                text.contains(&format!(
                    "example: seed {} (turn {}, {} actions)",
                    failure.seed, failure.turn, failure.actions
                )),
                "{text}"
            );
            assert!(text.contains("[growth]"), "{text}");
        }

        #[test]
        fn weighs_an_action_by_its_events_the_largest_number_on_a_card_and_the_cards_in_play() {
            register_all();
            let _preview = preview_every_set();
            let begun = begin_game(&create_game(&CreateGameOptions {
                seed: "jackioh-fuzz-1".to_string(),
                decks: decks_for_seed(1),
                ..CreateGameOptions::default()
            }));
            let weights = weigh(&begun.events, &begun.state);
            assert_eq!(weights[0], i64::try_from(begun.events.len()).unwrap());
            assert!(weights[2] >= 2 * i64::from(DECK_SIZE), "{weights:?}");

            let mut state = begun.state.clone();
            let card = &mut state.players[PlayerId::P1].library[0];
            card.buffs.health = -700;
            assert_eq!(largest_number(card), 700);
            card.buffs.health = 0;
            card.memory
                .insert("activations".to_string(), json!({ "turn": 3, "count": 4100 }));
            assert_eq!(largest_number(card), 4100);
            card.memory.clear();
            card.tuning = Some(Tuning {
                numbers: Some(IndexMap::from([("times".to_string(), 33_724)])),
                ..Tuning::default()
            });
            assert_eq!(largest_number(card), 33_724);
            assert_eq!(weigh(&[], &state)[1], 33_724);
        }

        #[test]
        fn the_growth_stage_names_the_first_measure_past_its_bound_and_passes_one_at_it() {
            register_all();
            let _preview = preview_every_set();
            let state = create_game(&CreateGameOptions {
                seed: "jackioh-fuzz-1".to_string(),
                decks: decks_for_seed(1),
                ..CreateGameOptions::default()
            });
            assert_eq!(
                growth_breach(&MEASURES.map(Measure::bound), &state, 5, "play", Some("core-001")),
                None
            );
            let events = growth_breach(
                &[GROWTH_EVENTS_BOUND + 1, GROWTH_NUMBER_BOUND + 1, 0],
                &state,
                7,
                "play",
                Some("core-001"),
            )
            .expect("events past their bound");
            assert!(
                events.starts_with("step 7 (\"play\" by core-001) emitted"),
                "{events}"
            );
            assert!(events.contains("GROWTH_EVENTS_BOUND"), "{events}");
            let instances = growth_breach(&[0, 0, GROWTH_INSTANCES_BOUND + 1], &state, 9, "endTurn", None)
                .expect("instances past their bound");
            assert!(instances.starts_with("step 9 (\"endTurn\") left"), "{instances}");
            assert!(instances.contains("GROWTH_INSTANCES_BOUND"), "{instances}");
        }

        #[test]
        fn the_summary_and_json_list_each_measure_s_heaviest_actions_heaviest_first() {
            let heavy = |seed: u32, value: i64| Heavy {
                seed,
                step: 3,
                action: "play".to_string(),
                card: Some("core-001".to_string()),
                value,
            };
            let mut heaviest = Heaviest::default();
            for (seed, value) in [(1, 10), (2, 50), (3, 30), (4, 50), (5, 20), (6, 40), (7, 5)] {
                heaviest.keep(Measure::Events, heavy(seed, value), HEAVIEST_SHOWN);
            }
            let seeds: Vec<u32> = heaviest.list(Measure::Events).iter().map(|h| h.seed).collect();
            assert_eq!(seeds, vec![2, 4, 6, 3, 5]);

            let wave = Wave {
                from: 1,
                ran: 7,
                failures: Vec::new(),
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest,
                elapsed_ms: 0.0,
            };
            let summary = wave.summary();
            assert!(
                summary.contains(&format!(
                    "    events (bound {GROWTH_EVENTS_BOUND}): 2/3 play core-001: 50; 4/3 play core-001: 50; 6/3"
                )),
                "{summary}"
            );
            assert!(
                summary.contains(&format!("    largest (bound {GROWTH_NUMBER_BOUND}): none")),
                "{summary}"
            );
            let json = wave.json();
            assert_eq!(
                json["heaviest"]["events"][0],
                json!({ "seed": 2, "step": 3, "action": "play", "card": "core-001", "value": 50 })
            );
            assert_eq!(json["bounds"]["largest"], json!(GROWTH_NUMBER_BOUND));
            assert_eq!(json["passed"], json!(7));
        }
    }

    mod fuzz_regression_corpus {
        use super::*;
        use jackioh_cards::CATALOG_VERSION;

        /// A record that lints clean: seed `seed`'s deal from the live pool.
        fn sound(seed: u32) -> CorpusRecord {
            CorpusRecord {
                seed,
                catalog_version: CATALOG_VERSION.to_string(),
                decks: decks_for_seed(seed),
                found: "a test's record".to_string(),
                retired: None,
            }
        }

        #[test]
        fn a_record_replays_its_pinned_decks_whatever_cards_join_the_live_pool() {
            register_all();
            // The pool before its last cards landed: the grown pool redeals seed 1.
            const EXTRA_CARDS: usize = 3;
            let before = FUZZ_POOL[..FUZZ_POOL.len() - EXTRA_CARDS].to_vec();
            let record = CorpusRecord {
                decks: decks_for_seed_from(1, &before),
                ..sound(1)
            };
            assert_eq!(lint_record(&record), Vec::<String>::new());
            assert_ne!(decks_for_seed(1), record.decks);
            let replay = || replay_record(&record).unwrap_or_else(|failure| panic!("{}", failure.message));
            let first = replay();
            assert_eq!(first.hash, replay().hash);
            let live = fuzz_seed(1).unwrap_or_else(|failure| panic!("{}", failure.message));
            assert_ne!(first.hash, live.hash);
        }

        #[test]
        fn the_lint_passes_a_sound_record_and_names_each_malformed_one() {
            register_all();
            assert_eq!(lint_record(&sound(1)), Vec::<String>::new());
            let token = CATALOG
                .iter()
                .find(|(_, def)| def.is_token())
                .map(|(id, _)| id.clone())
                .unwrap_or_default();
            let cases: Vec<(CorpusRecord, &str)> = vec![
                (CorpusRecord { seed: 0, ..sound(1) }, "seed must be"),
                (
                    CorpusRecord {
                        catalog_version: "v9.9.9".to_string(),
                        ..sound(1)
                    },
                    "catalogVersion",
                ),
                (
                    CorpusRecord {
                        found: " ".to_string(),
                        ..sound(1)
                    },
                    "found is empty",
                ),
                (
                    {
                        let mut record = sound(1);
                        record.decks.0.pop();
                        record
                    },
                    "p1 deck holds 19",
                ),
                (
                    {
                        let mut record = sound(1);
                        record.decks.1[0] = record.decks.0[0].clone();
                        record
                    },
                    "twice",
                ),
                (
                    {
                        let mut record = sound(1);
                        record.decks.0[0] = "core-gone".to_string();
                        record
                    },
                    "mark the record retired",
                ),
                (
                    {
                        let mut record = sound(1);
                        record.decks.0[0] = token;
                        record
                    },
                    "Token",
                ),
                (
                    CorpusRecord {
                        retired: Some(String::new()),
                        ..sound(1)
                    },
                    "retired is empty",
                ),
            ];
            for (record, phrase) in cases {
                let problems = lint_record(&record).join("\n");
                assert!(
                    problems.contains(phrase),
                    "expected \"{phrase}\" in: {problems:?}"
                );
            }
            // A retired record keeps its unknown ids: it is listed, not linted for them.
            let mut retired = sound(1);
            retired.decks.0[0] = "core-gone".to_string();
            retired.retired = Some("core-gone left the catalog".to_string());
            assert_eq!(lint_record(&retired), Vec::<String>::new());
        }

        #[test]
        fn a_corpus_file_of_the_wrong_shape_does_not_parse() {
            let record = sound(1);
            let fields = serde_json::json!({
                "seed": record.seed,
                "catalogVersion": record.catalog_version,
                "decks": record.decks,
                "found": record.found,
            });
            let parses = |value: &serde_json::Value| serde_json::from_str::<CorpusRecord>(&value.to_string());
            assert_eq!(parses(&fields).ok(), Some(record.clone()));

            let mut missing = fields.clone();
            missing
                .as_object_mut()
                .map(|object| object.remove("catalogVersion"));
            assert!(parses(&missing).is_err());

            let mut three = fields.clone();
            three["decks"] = serde_json::json!([record.decks.0, record.decks.1, record.decks.0]);
            assert!(parses(&three).is_err());

            let mut unknown = fields;
            unknown["log"] = serde_json::json!([]);
            assert!(parses(&unknown).is_err());
        }

        #[test]
        fn the_checked_in_corpus_lints_clean_and_every_record_replays() {
            let regress = regress(Path::new(CORPUS_DIR)).unwrap_or_else(|error| panic!("{error:#}"));
            assert_eq!(
                regress.failed,
                Vec::<String>::new(),
                "{}",
                regress.failed.join("\n")
            );
            assert!(!regress.lines.is_empty());
        }
    }

    mod fuzz_handicapped_games_r180_r184_r187 {
        use super::*;

        #[test]
        fn r183_the_rotation_covers_medium_and_hard_on_both_seats() {
            let seen: IndexSet<String> = [1, 2, 3, 4]
                .into_iter()
                .map(|seed| {
                    let rotation = handicap_for_seed(seed);
                    format!("{}:{}", rotation.tier, rotation.seat)
                })
                .collect();
            let mut seen: Vec<String> = seen.into_iter().collect();
            seen.sort();
            assert_eq!(seen, vec!["hard:p1", "hard:p2", "medium:p1", "medium:p2"]);
        }

        #[test]
        fn r180_seeds_with_one_seat_on_medium_or_hard_no_throw_a_real_ending_and_a_replay_that_matches() {
            let from = first_seed(None);
            let size = wave_size(None);
            let wave = play_handicap_wave(from, size);
            println!("{}", wave.summary());
            assert_eq!(wave.failure_lines(), Vec::<String>::new());
        }

        #[test]
        fn r679_seed_4679_whose_glitch_voids_the_match_passes() {
            // The daily run's handicapped seed that a Glitch voided at 96b68c9 (#1055).
            let outcome = play_seed(4679);
            assert_eq!(outcome.error, None);
        }
    }
}
