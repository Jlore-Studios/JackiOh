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
//!      endings come only from the endings channel, below).
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
//! I8 (no unlisted attack, switch or activation `reduce` accepts, §4.1, §4.2, §10.2) runs on every state under `cargo test`,
//! every I8_GATE_STRIDE-th under the gate and `--handicap`.
//!
//! And after every action, GROWTH (#1254): the gate weighs each action it applies by the events `reduce`
//! returned, the largest number on any card instance and the number of instances in play, prints each
//! measure's heaviest actions (one per game) in the summary and in `--json`, and fails a seed in the
//! "growth" stage when one action passes GROWTH_EVENTS_BOUND, GROWTH_NUMBER_BOUND or
//! GROWTH_INSTANCES_BOUND. That catches runaway growth inside one action (R874000's fusion, R884000's
//! University) at the step that grew, well before the engine's settle cap (§10.3) panics or a process
//! runs out of memory. The weighing reads the returned events and state only, never the clock, so it
//! is as deterministic as the game; the handicapped wave is weighed the same way through the same
//! loop, and `--duos` is weighed by `duos_fuzz.rs` through the shared helpers below.
//!
//! And on the states the gate reaches, PROBES (#1386): every `probe_stride()`-th state, forged
//! actions that are never logged. A wrong-seat probe sends the action the policy chose from the seat
//! that may not take it, and must be refused with no events (§9.3); a reused-nonce probe sends a
//! concede under the nonce of the last logged action, and must neither apply nor replace anything but
//! answer that action's own events (§10.1, §10.2); while a prompt is open, a malformed-answer probe
//! sends the policy's answer with a stale choice id, from the other seat, with a pick dropped below
//! `min`, with a pick doubled and with a pick no option offers, each owed a refusal with no events
//! (§10.6, #1574). Every way the state must be kept exactly, the nonce memory `hash_state` leaves out
//! included. A failure is the "probe" stage and names the step
//! and the probe. The handicapped wave probes the same way and `--duos` as `duos_fuzz.rs` says. The
//! probes draw from no random stream, so every game plays as it did before them.
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
//!   cargo jackioh fuzz --seeds 200 --handicap --json   the handicapped wave as one JSON object
//!   cargo jackioh fuzz --from <seed> --seeds 1      one seed a report named (`--handicap` too, for a handicapped one)
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
//! handicapped, rotating by seed over Medium and Hard and over both seats, through the gate's own
//! loop (`GameSetup` names the streams and the handicaps, `run_seed` plays and checks), and holds
//! them to the same checks: no throw, a real ending inside the cap in `Phase::Over`, a log that folds
//! back, handicaps included, to the live hash, the invariant monitor (I1–I8), growth weighing, the
//! same probes and the serialization checkpoint. A failure goes through `Failure` and `report` with
//! its tier and seat in the example line and both decks printed, and `--json` prints the wave as one
//! JSON object.
//!
//! # Starting states (#1503)
//!
//! A plain deal never sets the three setup inputs `create_game` and `fold` take beyond the decks, so
//! the paths that read them would have no random coverage. On `START_STATE_SHARE` of seeds, in both
//! waves, the game starts from a dealt deck (R433), last boards (R417, R564) or Glitch boards (R678).
//! The start is drawn from the seed's `start-` stream and its inputs from `start-inputs-`; the
//! boards are manufactured in the shape the server keeps, and each one holds one fused Unit (R179).
//! Create, fold and checkpoint all take the same inputs (`GameSetup::create_options` and
//! `fold_args`), so a game that starts from them still folds back to its live hash. The summary and
//! `--json` count seeds by start, and a failure names its start. `--from <seed>` plays a seed from
//! its start, since the seed alone draws it. The corpus replays from the plain start.
//!
//! # The endings channel (#1572)
//!
//! The policy never concedes, offers or answers a draw, or emotes (R84), and `timeout`,
//! `disconnectExpired`, `ceilingReached` and `setAutoEndTurn` are the server's, not its, so a plain
//! wave never plays those endings and cleanups across random boards. On `ENDINGS_SHARE` of seeds, in both
//! waves, the game also takes an endings channel: at each step, from a stream of its own, it may send
//! one of those actions in the policy's place (`endings_action`). A concede, an expired disconnect or
//! the ceiling comes at `ENDINGS_END_CHANCE`, an answer to a standing offer (R36) at
//! `ENDINGS_ANSWER_CHANCE`, and an offer, an emote (R1127), a timeout (R268) or the R345 setting at
//! `ENDINGS_ACT_CHANCE`, from either seat (R79). An offer, its answer and an emote go only while
//! `legal_actions` lists them. A channel action is logged like any other, so replay, the hash and
//! the checkpoint fold it, and the probes skip its step. The share, not every seed, keeps most games
//! at their depth; `GameSetup` draws it from the seed's `endings-` stream, so `--from <seed> --seeds
//! 1` replays it, and a game that took it may end by `ENDINGS_TERMINAL_REASONS`. The summary and
//! `--json` count the seeds that took it. The corpus replays without it, and the shipped §10.7
//! policy (`AI_SKIPPED_ACTIONS`) is unchanged.

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
    I6_GATE_STRIDE, I8_GATE_STRIDE, create_invariant_monitor, malformed_answers, preview_every_set, set_ships,
};
use jackioh_engine::{
    AI_DIFFICULTY, AI_PLAYOUT_STEP_CAP, Action, ActionBody, BACKROW_ZONES, CardInstance, CardType,
    CreateGameOptions, DECK_SIZE, Difficulty, DuosActionBody, DuosEvent, DuosSeat, DuosState, FoldArgs,
    GameEvent, GameState, Handicap, KeywordKind, LastBoardEntry, PLAYER_IDS, PerPlayerOpt, Phase, PlayerId,
    ReduceResult, Rng, TURN_CAP_PLAYER_TURNS, UNIT_ZONES, begin_game, create_game, find_instance, fold,
    has_standing_draw_offer, hash_state, legal_actions, mulligan_owed, opponent_of, reduce, seat_to_act,
    subsystems, validate_deck,
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
    /// Print the wave, the gate's or with `--handicap` the handicapped one, as one JSON object
    /// instead of text (#1254): its failures, each measure's heaviest actions and the growth bounds.
    #[arg(long, conflicts_with_all = ["regress"])]
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
    let wave = if args.handicap {
        play_handicap_wave(from, ran)
    } else {
        play_wave(from, ran)
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&wave.json())?);
    } else {
        print!("{}", wave.summary());
    }
    if !wave.failures.is_empty() {
        if !args.json {
            println!("{}", report(&wave.failures, ran, wave.elapsed_ms));
        }
        bail!(
            "{}: {} of {} seeds failed",
            wave_name(args.handicap),
            wave.failures.len(),
            ran
        );
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
/// voids the match (R679) among them, and, in a game that takes the endings channel,
/// `ENDINGS_TERMINAL_REASONS` (#1572).
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
// Starting states (#1503)
// ---------------------------------------------------------------------------------------------

/// What a game starts from beyond its decks and handicaps: the three setup inputs `create_game` and
/// `fold` take that a plain deal never sets (§9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartKind {
    /// Two built decks and no boards: every game before #1503, and every corpus record's.
    Plain,
    /// A deck dealt for one seat or both, its starting library unknown to its owner's view (R433).
    Dealt,
    /// Each seat's last board, which C+ #29 Portal to the Past can bring back (R417, R564).
    LastBoards,
    /// The two boards a Glitch may put on the field (R678).
    GlitchBoards,
}

impl StartKind {
    /// The name the summary, `--json` and the report print.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            StartKind::Plain => "plain",
            StartKind::Dealt => "dealt",
            StartKind::LastBoards => "last-boards",
            StartKind::GlitchBoards => "glitch-boards",
        }
    }
}

/// Every start, in the order the summary and `--json` list them.
pub(crate) const START_KINDS: [StartKind; 4] = [
    StartKind::Plain,
    StartKind::Dealt,
    StartKind::LastBoards,
    StartKind::GlitchBoards,
];

/// The starts a seed that does not play the plain one draws from.
const DRAWN_STARTS: [StartKind; 3] = [StartKind::Dealt, StartKind::LastBoards, StartKind::GlitchBoards];

/// The share of seeds, in both waves, that start from one of `DRAWN_STARTS`. Drawn from each seed's
/// own `start-` stream, so no other stream moves and a seed that draws `Plain` plays the game it
/// always played.
pub const START_STATE_SHARE: f64 = 0.5;

/// Which seats a `Dealt` start deals: p1's, p2's or both (R433).
const DEALT_SEATS: [&[PlayerId]; 3] = [&[PlayerId::P1], &[PlayerId::P2], &[PlayerId::P1, PlayerId::P2]];

/// The most cards a manufactured board draws from `BOARD_POOL`: every zone a field has, both sides'
/// (the board a game ends on, R417), before its one fused Unit.
const BOARD_MOST_CARDS: i32 = PLAYER_IDS.len() as i32 * (UNIT_ZONES + BACKROW_ZONES);

/// The head of a readable fused id (R179), which `manufactured_board` puts before its two ingredients.
const BOARD_FUSED_HEAD: &str = "t-1:";

/// Every catalog card that can stand on the field as a game ends: Units, the tokens among them (R11,
/// R417), Traps, Field Spells and Field Traps. It holds no Spell, and covers every set, as
/// `FUZZ_POOL` does (R1420). Sorted by catalog id, so a seeded shuffle of it is the same everywhere.
static BOARD_POOL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| def.type_ != CardType::Spell)
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    pool
});

/// The Units a manufactured board's fused card can be made of: no token, and no Immutable card, which
/// R23 means is never fused. Sorted by catalog id.
static FUSABLE_UNITS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| def.type_ == CardType::Unit && !def.is_token())
        .filter(|(_, def)| {
            ![&def.base, &def.radiant].iter().any(|face| {
                face.keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Immutable)
            })
        })
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    pool
});

/// One seat's board, in the shape the server keeps (R417, R564): up to `BOARD_MOST_CARDS` distinct
/// cards of `BOARD_POOL`, each on a face drawn by coin, and one fused Unit of two `FUSABLE_UNITS` so
/// the fused rebuild (R179) runs. Drawn from `rng` alone.
fn manufactured_board(rng: &mut Rng) -> Vec<LastBoardEntry> {
    let count = usize::try_from(1 + rng.int(BOARD_MOST_CARDS)).unwrap_or(1);
    let cards = rng.shuffle(BOARD_POOL.as_slice());
    let mut board: Vec<LastBoardEntry> = cards
        .into_iter()
        .take(count)
        .map(|def_id| LastBoardEntry {
            def_id,
            radiant: rng.coin(),
        })
        .collect();
    if let [first, second, ..] = rng.shuffle(FUSABLE_UNITS.as_slice()).as_slice() {
        board.push(LastBoardEntry {
            def_id: format!("{BOARD_FUSED_HEAD}{first}+{second}"),
            radiant: rng.coin(),
        });
    }
    board
}

// ---------------------------------------------------------------------------------------------
// The endings channel (#1572)
// ---------------------------------------------------------------------------------------------

/// The share of seeds, in both 1v1 waves, that take the endings channel (#1572), drawn from the
/// seed's `endings-` stream. The rest play as the policy alone does, so they keep their depth.
pub const ENDINGS_SHARE: f64 = 0.25;
/// Per step, in a game that takes the channel: the chance it concedes, expires a disconnect or
/// reaches the match ceiling in the policy's place (§2.5, R79).
pub const ENDINGS_END_CHANCE: f64 = 0.004;
/// Per step while a draw offer stands, in a game that takes the channel: the chance the seat it was
/// made to answers it (R36); an offer left unanswered lapses with its turn (R269).
pub const ENDINGS_ANSWER_CHANCE: f64 = 0.2;
/// Per step, in a game that takes the channel: the chance it sends an offer, an emote (R1127), a
/// timeout (R268) or the R345 setting in the policy's place.
pub const ENDINGS_ACT_CHANCE: f64 = 0.03;

/// The endings only the endings channel reaches (#1572).
const ENDINGS_TERMINAL_REASONS: &[&str] = &["concede", "draw-accepted", "disconnect", "match-ceiling"];

/// The actions `legal_actions` offers `seat` that `keep` holds, in its order.
fn listed(state: &GameState, seat: PlayerId, keep: fn(&ActionBody) -> bool) -> Vec<ActionBody> {
    legal_actions(state, seat).into_iter().filter(keep).collect()
}

/// What the endings channel sends at `state` in the policy's place, and from which seat, or `None`
/// when the policy acts (#1572). `player` is the seat the game waits on, `rng` the seed's
/// `endings-steps-` stream. An offer, its answer and an emote go only while `legal_actions` lists
/// them; the rest are the server's, which `reduce` takes from either seat while the game is live.
fn endings_action(state: &GameState, player: PlayerId, rng: &mut Rng) -> Option<(PlayerId, ActionBody)> {
    let seats = [player, opponent_of(player)];
    if rng.chance(ENDINGS_END_CHANCE) {
        let seat = *rng.pick(&seats)?;
        let endings = [
            ActionBody::Concede,
            ActionBody::DisconnectExpired { player: seat },
            ActionBody::CeilingReached,
        ];
        return rng.pick(&endings).map(|body| (seat, body.clone()));
    }
    // R36, R269: the seat with an offer to answer is the one the active player made it to.
    if let Some(&seat) = seats.iter().find(|seat| has_standing_draw_offer(state, **seat))
        && rng.chance(ENDINGS_ANSWER_CHANCE)
    {
        let answers = listed(state, seat, |body| matches!(body, ActionBody::AnswerDraw { .. }));
        return rng.pick(&answers).map(|body| (seat, body.clone()));
    }
    if !rng.chance(ENDINGS_ACT_CHANCE) {
        return None;
    }
    let mut acts: Vec<(PlayerId, ActionBody)> = Vec::new();
    for seat in seats {
        let offered = listed(state, seat, |body| {
            matches!(body, ActionBody::OfferDraw | ActionBody::Emote { .. })
        });
        acts.extend(offered.into_iter().map(|body| (seat, body)));
        acts.push((seat, ActionBody::Timeout));
        let enabled = state.players[seat].auto_end_turn == Some(false);
        acts.push((seat, ActionBody::SetAutoEndTurn { enabled }));
    }
    rng.pick(&acts).cloned()
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
    Probe,
    Checkpoint,
}

impl FailureStage {
    fn as_str(self) -> &'static str {
        match self {
            FailureStage::Play => "play",
            FailureStage::Termination => "termination",
            FailureStage::Replay => "replay",
            FailureStage::Invariant => "invariant",
            FailureStage::Growth => "growth",
            FailureStage::Probe => "probe",
            FailureStage::Checkpoint => "checkpoint",
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
    /// The prefix lengths whose end state held paused work (#1387): step 0 is the setup state,
    /// otherwise the state after that many actions. The checkpoint is drawn from these.
    paused: Vec<usize>,
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

/// What one game is played as: the seed that names its streams and, for the handicapped wave, which
/// seat is handicapped and how (`handicap_for_seed`), and what it starts from (`StartKind`, #1503).
/// The gate's and the handicapped wave's games go through the same loop, told apart by this alone.
#[derive(Clone, Copy, Debug)]
struct GameSetup {
    seed: u32,
    /// The handicapped wave's rotation for `seed`; `None` for the gate's two plain seats.
    rotation: Option<SeedHandicap>,
    /// What the game starts from beyond its decks (#1503), drawn from `seed` alone.
    start: StartKind,
    /// Whether the game also takes the endings channel's actions (#1572), drawn from seed alone.
    endings: bool,
}

impl GameSetup {
    /// The gate's game for `seed`, from the start the seed draws.
    fn gate(seed: u32) -> GameSetup {
        GameSetup::plain(seed).with_drawn_start().with_drawn_endings()
    }

    /// The gate's game for `seed` from the plain start, drawing none: the game a corpus record
    /// replays (#1503).
    fn plain(seed: u32) -> GameSetup {
        GameSetup {
            seed,
            rotation: None,
            start: StartKind::Plain,
            endings: false,
        }
    }

    /// The handicapped wave's game for `seed`, with its rotation and the start the seed draws.
    fn handicapped(seed: u32) -> GameSetup {
        GameSetup {
            seed,
            rotation: Some(handicap_for_seed(seed)),
            start: StartKind::Plain,
            endings: false,
        }
        .with_drawn_start()
        .with_drawn_endings()
    }

    /// `self` with the start its seed draws (#1503): on `START_STATE_SHARE` of seeds one of
    /// `DRAWN_STARTS`, else `Plain`. The draw reads the `start-` stream and nothing else.
    fn with_drawn_start(self) -> GameSetup {
        let mut rng = Rng::new(&self.stream("start-"), 0);
        let start = if rng.chance(START_STATE_SHARE) {
            rng.pick(&DRAWN_STARTS).copied().unwrap_or(StartKind::Plain)
        } else {
            StartKind::Plain
        };
        GameSetup { start, ..self }
    }

    /// `self` taking the endings channel on `ENDINGS_SHARE` of seeds (#1572). The draw reads the
    /// `endings-` stream and nothing else.
    fn with_drawn_endings(self) -> GameSetup {
        let endings = Rng::new(&self.stream("endings-"), 0).chance(ENDINGS_SHARE);
        GameSetup { endings, ..self }
    }

    /// The stream named `kind` (`""` for the game's own, `policy-`, `policy-order-`, `checkpoint-`,
    /// `endings-` and `endings-steps-`):
    /// `jackioh-fuzz-<kind><seed>` for the gate and `jackioh-fuzz-handicap-<kind><seed>` for the
    /// handicapped wave, the names every seed was played under before the two shared a loop.
    fn stream(&self, kind: &str) -> String {
        let wave = if self.rotation.is_some() { "handicap-" } else { "" };
        format!("jackioh-fuzz-{wave}{kind}{}", self.seed)
    }

    /// The handicap the game is created, replayed and checkpointed with (R180, R187): one seat's.
    fn handicaps(&self) -> Option<PerPlayerOpt<Handicap>> {
        self.rotation.map(|SeedHandicap { seat, handicap, .. }| {
            let mut handicaps = PerPlayerOpt::default();
            *handicaps.slot(seat) = Some(handicap);
            handicaps
        })
    }

    /// What `create_game` takes for this game (#1503): the game's stream, `decks`, the handicaps and
    /// the start's one input, drawn from the `start-inputs-` stream. Equal on every call, which is
    /// what lets the fold and the checkpoint rebuild the game from `fold_args`.
    fn create_options(&self, decks: (Vec<String>, Vec<String>)) -> CreateGameOptions {
        let mut options = CreateGameOptions {
            seed: self.stream(""),
            decks,
            handicaps: self.handicaps(),
            ..CreateGameOptions::default()
        };
        let mut rng = Rng::new(&self.stream("start-inputs-"), 0);
        match self.start {
            StartKind::Plain => {}
            StartKind::Dealt => options.dealt = rng.pick(&DEALT_SEATS).map(|seats| seats.to_vec()),
            StartKind::LastBoards => {
                options.last_boards = Some((manufactured_board(&mut rng), manufactured_board(&mut rng)));
            }
            StartKind::GlitchBoards => {
                options.glitch_boards = Some((manufactured_board(&mut rng), manufactured_board(&mut rng)));
            }
        }
        options
    }

    /// What `fold` takes to replay `log` over `decks`: the same setup `create_options` gives
    /// `create_game` (§9.3, R180, R187, R433, R417, R678).
    fn fold_args(&self, decks: (Vec<String>, Vec<String>), log: Vec<Action>) -> FoldArgs {
        let CreateGameOptions {
            seed,
            decks,
            handicaps,
            dealt,
            last_boards,
            glitch_boards,
            ..
        } = self.create_options(decks);
        FoldArgs {
            seed,
            decks,
            log,
            handicaps,
            dealt,
            last_boards,
            glitch_boards,
            ..FoldArgs::default()
        }
    }
}

/// One full game played by SPEC §10.7's policy. `subsystems::choose_action` IS that policy — uniform
/// over `legal_actions` minus `AI_SKIPPED_ACTIONS` (R84), ending the turn when nothing else is on
/// offer or with `AI_END_TURN_PROBABILITY`, and answering an open prompt uniformly (R44) — so this
/// drives the shipped machinery rather than a second copy of the rule. In a game that takes the
/// endings channel, `endings_action` may send a step's action in the policy's place (#1572). The
/// probes' forged actions go through `probe` (#1386).
fn play_game(
    setup: &GameSetup,
    decks: (Vec<String>, Vec<String>),
    probe: Reducer,
) -> Result<GameRun, FuzzFailure> {
    register_all();

    let begun = begin_game(&create_game(&setup.create_options(decks.clone())));
    let mut state = begun.state;
    let mut policy = Rng::new(&setup.stream("policy-"), 0);
    // R265: while both mulligans are open either seat may answer first; a stream of its own picks
    // which, so the fuzz plays both orders (the game is the same either way, R265).
    let mut order = Rng::new(&setup.stream("policy-order-"), 0);
    // #1572: the endings channel's own stream, in a game that takes it.
    let mut endings = setup
        .endings
        .then(|| Rng::new(&setup.stream("endings-steps-"), 0));
    let mut log: Vec<Action> = Vec::new();
    let mut monitor = create_invariant_monitor(&state);
    let stride = hidden_stride();
    let unlisted_stride = unlisted_stride();
    let probe_stride = probe_stride();
    let max_actions = max_actions_per_game();
    let mut heaviest = Heaviest::default();
    // The events the last logged action returned, which a probe of its nonce is owed (#1386).
    let mut last_events: Vec<GameEvent> = Vec::new();
    // Setup's own events: a Unit cast on draw in the opening deal (C+ #26 Tommy Tempo) enters there.
    let mut opening = monitor.after(&begun.events, &state);
    opening.extend(monitor.hidden(&state));
    if let Some(first) = opening.first() {
        return Err(FuzzFailure::new(
            FailureStage::Invariant,
            first.clone(),
            At {
                turn: state.turn,
                actions: 0,
            },
        ));
    }
    // #1387: the checkpoint prefers a state with paused work; step 0 is the setup state.
    let mut paused_steps: Vec<usize> = Vec::new();
    if state_paused(&state) {
        paused_steps.push(0);
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
        // #1572: in a game that takes the endings channel, the channel may send this step's action.
        let sent = endings
            .as_mut()
            .and_then(|rng| endings_action(&state, player, rng));
        let from_channel = sent.is_some();
        let (player, chosen) = if let Some(sent) = sent {
            sent
        } else {
            let Some(chosen): Option<ActionBody> = subsystems::choose_action(&state, player, &mut policy)
            else {
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
            (player, chosen)
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

        // #1386: forged actions against this state, never logged. A channel action is skipped: the
        // other seat may send a concede or a server action too (§9.3).
        if !from_channel
            && log.len().is_multiple_of(probe_stride)
            && let Some(message) = probe_state(
                &state,
                player,
                &chosen,
                log.last().map(|logged| (logged, last_events.as_slice())),
                log.len(),
                probe,
            )
        {
            return Err(FuzzFailure::new(
                FailureStage::Probe,
                message,
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
                    "{} \"{}\" but reduce refused it: {error}",
                    if from_channel {
                        "the endings channel sent"
                    } else {
                        "legalActions offered"
                    },
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
        if state_paused(&state) {
            paused_steps.push(log.len());
        }

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
                    seed: setup.seed,
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
        last_events = result.events;
    }

    Ok(GameRun {
        state,
        log,
        decks,
        heaviest,
        paused: paused_steps,
    })
}

/// Assertion 2: the game ended the way §2.5 says a policy game can end, inside the R2 cap.
fn check_termination(setup: &GameSetup, run: &GameRun) -> Result<(), FuzzFailure> {
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
    // #1572: a game that takes the endings channel may also end by one of its endings.
    let channel: &[&str] = if setup.endings {
        ENDINGS_TERMINAL_REASONS
    } else {
        &[]
    };
    let allowed: Vec<&str> = TERMINAL_REASONS.iter().chain(channel).copied().collect();
    if !allowed.contains(&result.reason.as_str()) {
        return Err(FuzzFailure::new(
            FailureStage::Termination,
            format!(
                "ended with reason \"{}\"; the policy never concedes or offers a draw (R84), so §2.5 leaves only {}",
                result.reason,
                allowed.join(", ")
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
fn check_replay(setup: &GameSetup, run: &GameRun) -> Result<(), FuzzFailure> {
    let at = At {
        turn: run.state.turn,
        actions: run.log.len(),
    };
    register_all();
    // R180, R187: the handicaps are part of what a game replays from, and so is its start (#1503).
    let replayed = fold(&setup.fold_args(run.decks.clone(), run.log.clone()));

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
// The serialization checkpoint (#1387)
// ---------------------------------------------------------------------------------------------

/// Whether the state holds paused work (SPEC §10.1): an open prompt, a parked `work` remainder,
/// or an owed mulligan. This is the single definition of the states the checkpoint prefers.
pub(crate) fn state_paused(state: &GameState) -> bool {
    state.pending.is_some() || !state.work.is_empty() || !mulligan_owed(state).is_empty()
}

/// Which prefix length the checkpoint restarts from, and whether it holds paused work: a
/// seed-drawn paused step when one occurred, else the mid-game fallback. `actions` is the live
/// log's length, so step 0 is the setup state and the fallback halves the game.
pub(crate) fn checkpoint_step(stream: &str, paused: &[usize], actions: usize) -> (usize, bool) {
    if paused.is_empty() {
        return (actions / 2, false);
    }
    let mut rng = Rng::new(stream, 0);
    (rng.pick(paused).copied().unwrap_or(actions / 2), true)
}

/// The state through JSON text and back: what SPEC §9.3's checkpoint stores after the log's
/// newest row and §10.1 requires paused work to survive. A 1v1 game's or a Duos game's.
pub(crate) fn round_trip<S: Serialize + for<'de> Deserialize<'de>>(state: &S) -> Result<S, String> {
    let text = serde_json::to_string(state)
        .map_err(|error| format!("the round trip could not serialise the state: {error}"))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("the round trip could not parse the state back: {error}"))
}

/// `state` restored by `restore`, required to hash as `state` does under `hash`: the restored
/// state, or the mismatch. Every wave restores with `round_trip`; a test plants a lossy one.
pub(crate) fn verify_round_trip<S>(
    state: &S,
    hash: impl Fn(&S) -> String,
    restore: impl FnOnce(&S) -> Result<S, String>,
) -> Result<S, String> {
    let restored = restore(state)?;
    let (live, again) = (hash(state), hash(&restored));
    if again != live {
        return Err(format!(
            "the round trip changed the state: hash {again} != {live}"
        ));
    }
    Ok(restored)
}

/// What one game's checkpoint crossed and what it cost: whether it restarted from paused work
/// and the cost in milliseconds. (The step itself stays inside `check_serial_restart`: it picks
/// the prefix, so carrying it out would be a field nobody reads.)
struct CheckpointReport {
    paused: bool,
    ms: u128,
}

/// Assertion 4 (#1387): the game survives a serialized restart. Folds the log's prefix to the
/// checkpoint step, round-trips that state through JSON requiring identical hashes, then applies
/// the rest of the log from the restored state and requires the live hash and ending — so the
/// existing invariant, termination, growth and replay checks cover the restored game. Fails in
/// the "checkpoint" stage, and past `CHECKPOINT_TIME_BOUND_MS`.
fn check_serial_restart(
    setup: &GameSetup,
    decks: &(Vec<String>, Vec<String>),
    log: &[Action],
    live: &GameState,
    paused: &[usize],
) -> Result<CheckpointReport, FuzzFailure> {
    let at = At {
        turn: live.turn,
        actions: log.len(),
    };
    let started_at = Instant::now();
    let (step, was_paused) = checkpoint_step(&setup.stream("checkpoint-"), paused, log.len());
    register_all();
    let prefix = fold(&setup.fold_args(decks.clone(), log[..step].to_vec()));
    if let Some(first) = prefix.errors.first() {
        return Err(FuzzFailure::new(
            FailureStage::Checkpoint,
            format!(
                "checkpoint prefix fold rejected {} action(s); first at nonce \"{}\": {}",
                prefix.errors.len(),
                first.nonce,
                first.error
            ),
            at,
        ));
    }
    let mut state = verify_round_trip(&prefix.state, hash_state, round_trip)
        .map_err(|message| FuzzFailure::new(FailureStage::Checkpoint, message, at))?;
    for action in &log[step..] {
        let result = reduce(&state, action);
        if let Some(error) = &result.error {
            return Err(FuzzFailure::new(
                FailureStage::Checkpoint,
                format!(
                    "checkpoint continuation refused \"{}\" at nonce \"{}\": {error}",
                    action.action_type().as_str(),
                    action.nonce
                ),
                at,
            ));
        }
        state = result.state;
    }
    let live_hash = hash_state(live);
    let again = hash_state(&state);
    if live_hash != again {
        return Err(FuzzFailure::new(
            FailureStage::Checkpoint,
            format!(
                "checkpoint hash {again} != live hash {live_hash} after {} actions — the restored game diverged (§9.3)",
                log.len()
            ),
            at,
        ));
    }
    let live_result = serde_json::to_string(&live.result).unwrap_or_default();
    let continued_result = serde_json::to_string(&state.result).unwrap_or_default();
    if live_result != continued_result {
        return Err(FuzzFailure::new(
            FailureStage::Checkpoint,
            format!("checkpoint ended {continued_result}, live ended {live_result}"),
            at,
        ));
    }
    let ms = started_at.elapsed().as_millis();
    if ms > CHECKPOINT_TIME_BOUND_MS {
        return Err(FuzzFailure::new(
            FailureStage::Checkpoint,
            format!("checkpoint took {ms}ms, past CHECKPOINT_TIME_BOUND_MS ({CHECKPOINT_TIME_BOUND_MS}ms)"),
            at,
        ));
    }
    Ok(CheckpointReport {
        paused: was_paused,
        ms,
    })
}

// ---------------------------------------------------------------------------------------------
// Growth inside one action (#1254)
// ---------------------------------------------------------------------------------------------

/// The most wall-clock time one game's serialization checkpoint may take (#1387). A round
/// trip is normally milliseconds — the state is already in memory and the fold replays its log
/// with no I/O — so this bound only fires on pathological serialization blowup.
pub const CHECKPOINT_TIME_BOUND_MS: u128 = 5_000;

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
// Forged-action probes (#1386)
// ---------------------------------------------------------------------------------------------

/// The probes under the gate, the handicapped wave and `--duos`: every this-many-th state (#1386);
/// the test sweep probes every state. Measured 2026-10-20 (4 cores, release) over seeds 1–200, in wall
/// seconds before the probes and with this stride: `cargo jackioh fuzz` 14.8 and 14.4–14.9, `--handicap`
/// 15.2 and 15.4, `--duos` 43.6 and 44.5, so the gate's time is kept.
pub const PROBE_GATE_STRIDE: usize = 10;

/// A probe failure's name: the policy's action sent by a seat that may not take it.
pub(crate) const WRONG_SEAT_PROBE: &str = "wrong-seat";

/// A probe failure's name: another action under a nonce already applied.
pub(crate) const REUSED_NONCE_PROBE: &str = "reused-nonce";

/// A probe failure's name: the policy's answer to the open prompt, made malformed (#1574).
pub(crate) const MALFORMED_ANSWER_PROBE: &str = "malformed-answer";

/// The probes' stride: every state in the test sweep, every PROBE_GATE_STRIDE-th otherwise.
pub fn probe_stride() -> usize {
    if is_whole_suite_sweep() {
        1
    } else {
        PROBE_GATE_STRIDE
    }
}

/// What a probe's forged action goes through: `reduce`, or in a test a broken one it must catch.
type Reducer = fn(&GameState, &Action) -> ReduceResult;

/// What a probe's `result` did wrong (§9.3, §10.2): with `owed` `None` it owed a refusal with no
/// events, with `Some` its nonce's first answer; either way `state` exactly, the nonce memory that
/// `hash_state` leaves out included. `None` when it held.
fn judge_probe(state: &GameState, result: &ReduceResult, owed: Option<&[GameEvent]>) -> Option<String> {
    let mut wrong: Vec<String> = Vec::new();
    match (owed, &result.error) {
        (None, None) => wrong.push("was accepted".to_string()),
        (Some(_), Some(error)) => wrong.push(format!("was refused ({error})")),
        _ => {}
    }
    if result.events.as_slice() != owed.unwrap_or_default() {
        wrong.push(match owed {
            None => format!("emitted {} event(s)", result.events.len()),
            Some(first) => format!(
                "answered {} event(s), not the {} the nonce first returned",
                result.events.len(),
                first.len()
            ),
        });
    }
    if result.state != *state {
        wrong.push(format!(
            "moved the state (hash {} became {})",
            hash_state(state),
            hash_state(&result.state)
        ));
    }
    (!wrong.is_empty()).then(|| wrong.join(" and "))
}

/// The probes on one state (#1386), never logged. First a concede by `player` under the last logged
/// action's nonce, owed that action's own events (`last`); then `chosen` sent by the other seat, owed
/// a refusal, unless that seat still owes its own mulligan, which it may answer (R265); then, while a
/// prompt is open, `chosen` made malformed (`malformed_answers`), each owed a refusal (#1574).
fn probe_state(
    state: &GameState,
    player: PlayerId,
    chosen: &ActionBody,
    last: Option<(&Action, &[GameEvent])>,
    step: usize,
    probe: Reducer,
) -> Option<String> {
    if let Some((logged, first)) = last {
        let forged = Action::new(ActionBody::Concede, player, logged.nonce.clone());
        if let Some(wrong) = judge_probe(state, &probe(state, &forged), Some(first)) {
            return Some(format!(
                "{REUSED_NONCE_PROBE} probe after step {step}: {player} sent \"concede\" under step {step}'s nonce \"{}\", and it {wrong}",
                logged.nonce
            ));
        }
    }
    let other = opponent_of(player);
    if !mulligan_owed(state).contains(&other) {
        let forged = Action::new(chosen.clone(), other, format!("fuzz-probe-{step}"));
        if let Some(wrong) = judge_probe(state, &probe(state, &forged), None) {
            return Some(format!(
                "{WRONG_SEAT_PROBE} probe after step {step}: {other} sent \"{}\" while {player} was to act, and it {wrong}",
                chosen.action_type().as_str()
            ));
        }
    }
    for (variant, seat, body) in malformed_answers(state, chosen) {
        let forged = Action::new(body, seat, format!("fuzz-probe-{step}"));
        if let Some(wrong) = judge_probe(state, &probe(state, &forged), None) {
            return Some(format!(
                "{MALFORMED_ANSWER_PROBE} probe after step {step}: {seat} sent the {variant} answer to the open prompt, and it {wrong}"
            ));
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Failure collection and the grouped report
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Failure {
    seed: u32,
    /// The handicapped wave's tier and seat for `seed`; `None` in the gate's. Kept out of
    /// `signature`, so one bug stays one group across the rotation's four slots.
    rotation: Option<(Difficulty, PlayerId)>,
    /// What the seed's game started from (#1503). Kept out of `signature` as the rotation is.
    start: StartKind,
    stage: FailureStage,
    signature: String,
    message: String,
    /// The two decks the seed was dealt, p1's then p2's: a handicapped seat's is longer than
    /// `DECK_SIZE`, so the report cannot split one flat list.
    decks: (Vec<String>, Vec<String>),
    turn: i64,
    actions: i64,
}

impl Failure {
    /// Every card id in the two decks.
    fn cards(&self) -> impl Iterator<Item = &String> {
        self.decks.0.iter().chain(&self.decks.1)
    }
}

/// The name of the gate's wave or of the handicapped one, in the report's heading and the exit message.
fn wave_name(handicapped: bool) -> &'static str {
    if handicapped {
        "fuzz-handicap"
    } else {
        "M4 fuzz gate"
    }
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
/// group (the narrowest suspects), the most frequent cards, and the full deck pair of the first seed,
/// which is the one a reader reproduces with. A handicapped wave's example names the seed's tier and
/// seat, and its reproduce line says `--handicap`.
fn report(failures: &[Failure], ran: u32, elapsed_ms: f64) -> String {
    let mut groups: IndexMap<String, Vec<&Failure>> = IndexMap::new();
    for failure in failures {
        groups.entry(failure.signature.clone()).or_default().push(failure);
    }
    let mut ordered: Vec<(String, Vec<&Failure>)> = groups.into_iter().collect();
    ordered.sort_by_key(|group| std::cmp::Reverse(group.1.len()));

    let handicapped = failures.first().is_some_and(|failure| failure.rotation.is_some());
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{}: {} of {ran} seeds failed ({} distinct failure signature(s), {}s).",
        wave_name(handicapped),
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
                .flat_map(|failure| failure.cards().map(String::as_str)),
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
        let who = first
            .rotation
            .map_or_else(String::new, |(tier, seat)| format!("{tier} {seat}, "));
        lines.push(format!(
            "  example: seed {} ({who}turn {}, {} actions)",
            first.seed, first.turn, first.actions
        ));
        lines.push(format!("  start: {}", first.start.as_str()));
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
        lines.push(format!(
            "  seed {} p1 deck: {}",
            first.seed,
            first.decks.0.join(", ")
        ));
        lines.push(format!(
            "  seed {} p2 deck: {}",
            first.seed,
            first.decks.1.join(", ")
        ));
        lines.push(String::new());
    }

    let mut all_cards: Vec<String> = failures
        .iter()
        .flat_map(|failure| failure.cards().cloned())
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
    lines.push(format!(
        "Reproduce one seed with: cargo jackioh fuzz --from <seed> --seeds 1{}",
        if handicapped { " --handicap" } else { "" }
    ));
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
    /// Whether the game's checkpoint restarted from paused work (#1387).
    checkpoint_paused: bool,
    /// What the game's checkpoint cost, in milliseconds (#1387).
    checkpoint_ms: u128,
    /// What the game started from (#1503).
    start: StartKind,
}

/// One seed through the four assertions: never panic, always terminate by hero death or the R2 cap,
/// and fold `(seed, log)` to the same hash; a failure names every card in this seed's decks.
fn fuzz_seed(seed: u32) -> Result<Passed, Failure> {
    fuzz_deal(seed, decks_for_seed(seed))
}

/// One game through the four assertions: `seed` names the game's and the policy's streams, `decks` the
/// deal.
fn fuzz_deal(seed: u32, decks: (Vec<String>, Vec<String>)) -> Result<Passed, Failure> {
    fuzz_deal_probed(seed, decks, reduce)
}

/// `fuzz_deal` with the probes' forged actions going through `probe` (#1386).
fn fuzz_deal_probed(seed: u32, decks: (Vec<String>, Vec<String>), probe: Reducer) -> Result<Passed, Failure> {
    run_seed(&GameSetup::gate(seed), decks, probe)
}

/// One seed of the handicapped wave through the same checks as the gate's: its rotation's tier and
/// seat (`handicap_for_seed`), its decks (`handicap_decks_for_seed`), its own streams.
fn fuzz_handicap_seed(seed: u32) -> Result<Passed, Failure> {
    let SeedHandicap { seat, handicap, .. } = handicap_for_seed(seed);
    fuzz_handicap_deal(seed, handicap_decks_for_seed(seed, seat, &handicap), reduce)
}

/// `decks` played as `seed`'s handicapped game, the probes' forged actions going through `probe`.
fn fuzz_handicap_deal(
    seed: u32,
    decks: (Vec<String>, Vec<String>),
    probe: Reducer,
) -> Result<Passed, Failure> {
    run_seed(&GameSetup::handicapped(seed), decks, probe)
}

/// One game through the four assertions, whichever wave `setup` is of.
fn run_seed(setup: &GameSetup, decks: (Vec<String>, Vec<String>), probe: Reducer) -> Result<Passed, Failure> {
    // R1420: the fuzz pool is the whole catalog, every set shipped or not, so this worker thread
    // previews every set for the game, its checks and its replay.
    let _preview = preview_every_set();
    let mut reached: Option<At> = None;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<Passed, FuzzFailure> {
        // 1. never panic
        let run = play_game(setup, decks.clone(), probe)?;
        reached = Some(At {
            turn: run.state.turn,
            actions: run.log.len(),
        });
        // 2. always terminate, by hero death or the R2 cap
        check_termination(setup, &run)?;
        // 3. (seed, log) folds to the same hash
        check_replay(setup, &run)?;
        // 4. the game survives a serialized restart (#1387)
        let checkpoint = check_serial_restart(setup, &run.decks, &run.log, &run.state, &run.paused)?;
        Ok(Passed {
            hash: hash_state(&run.state),
            reason: run
                .state
                .result
                .map_or_else(|| "none".to_string(), |result| result.reason.as_str().to_string()),
            actions: run.log.len(),
            turn: run.state.turn,
            heaviest: run.heaviest,
            checkpoint_paused: checkpoint.paused,
            checkpoint_ms: checkpoint.ms,
            start: setup.start,
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
    Err(Failure {
        seed: setup.seed,
        rotation: setup.rotation.map(SeedHandicap::who),
        start: setup.start,
        stage,
        signature: signature_of(stage, &message),
        message,
        decks,
        turn,
        actions,
    })
}

/// A whole wave's results, in seed order.
struct Wave {
    /// Whether this is the handicapped wave (`--handicap`) and not the gate's.
    handicapped: bool,
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
    /// How many passing games checkpointed from paused work (#1387).
    checkpoint_paused: usize,
    /// How many passing games took the mid-game fallback (#1387).
    checkpoint_mid: usize,
    /// The slowest passing game's checkpoint, in milliseconds (#1387).
    checkpoint_max_ms: u128,
    /// How many seeds, passed or failed, started from each kind of start (#1503).
    starts: IndexMap<&'static str, usize>,
    /// How many seeds, passed or failed, took the endings channel (#1572).
    endings: usize,
}

/// Plays the gate's seeds `from..from + ran`, in parallel, and gathers them back in seed order.
fn play_wave(from: u32, ran: u32) -> Wave {
    play_seeds(false, from, ran, fuzz_seed)
}

/// Plays the handicapped wave's seeds `from..from + ran` the same way.
fn play_handicap_wave(from: u32, ran: u32) -> Wave {
    play_seeds(true, from, ran, fuzz_handicap_seed)
}

/// `seed` over `from..from + ran`, in parallel, gathered back in seed order into one wave's numbers.
fn play_seeds(handicapped: bool, from: u32, ran: u32, seed: fn(u32) -> Result<Passed, Failure>) -> Wave {
    record_panic_locations();
    register_all();
    let started_at = Instant::now();
    let results: Vec<Result<Passed, Failure>> = (from..from + ran).into_par_iter().map(seed).collect();
    let elapsed_ms = started_at.elapsed().as_secs_f64() * MS_PER_SECOND;
    let setup_of = if handicapped {
        GameSetup::handicapped
    } else {
        GameSetup::gate
    };

    let mut wave = Wave {
        handicapped,
        from,
        ran,
        failures: Vec::new(),
        reasons: IndexMap::new(),
        actions_total: 0,
        actions_max: 0,
        turn_max: 0,
        heaviest: Heaviest::default(),
        elapsed_ms,
        checkpoint_paused: 0,
        checkpoint_mid: 0,
        checkpoint_max_ms: 0,
        starts: IndexMap::new(),
        endings: (from..from + ran).filter(|seed| setup_of(*seed).endings).count(),
    };
    for result in results {
        let start = match &result {
            Ok(passed) => passed.start,
            Err(failure) => failure.start,
        };
        *wave.starts.entry(start.as_str()).or_insert(0) += 1;
        match result {
            Ok(passed) => {
                *wave.reasons.entry(passed.reason).or_insert(0) += 1;
                wave.actions_total += passed.actions;
                wave.actions_max = wave.actions_max.max(passed.actions);
                wave.turn_max = wave.turn_max.max(passed.turn);
                wave.heaviest.merge(&passed.heaviest);
                if passed.checkpoint_paused {
                    wave.checkpoint_paused += 1;
                } else {
                    wave.checkpoint_mid += 1;
                }
                wave.checkpoint_max_ms = wave.checkpoint_max_ms.max(passed.checkpoint_ms);
            }
            Err(failure) => wave.failures.push(failure),
        }
    }
    wave
}

impl Wave {
    /// How many seeds started from each of `START_KINDS`, in that order, none counted as 0 (#1503).
    fn start_counts(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
        START_KINDS.into_iter().map(|start| {
            let name = start.as_str();
            (name, self.starts.get(name).copied().unwrap_or(0))
        })
    }

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
        let (label, full, wave, command) = if self.handicapped {
            (
                "fuzz-handicap",
                "FULL HANDICAP WAVE",
                "the handicapped wave",
                "cargo jackioh fuzz --handicap",
            )
        } else {
            ("M4 fuzz", "FULL GATE", "the gate", "cargo jackioh fuzz")
        };
        let mut text = format!(
            "[{label}] {}\n  seeds {}..{}: {passed} passed, {} failed\n  {} panic(s), {} over the {}-action bound or stalled, {} replay mismatch(es), {} invariant violation(s), {} past a growth bound, {} failed a probe, {} checkpoint mismatch(es)\n  endings: {}\n  {average} actions/game on average, {} at most; longest game ended on turn {} of {TURN_CAP_PLAYER_TURNS}\n  checkpoints: {} paused, {} mid-game fallback; slowest {}ms of {CHECKPOINT_TIME_BOUND_MS}ms\n  pool: {} deck-legal cards, {} excluded; I6 every {} state(s), I8 every {} state(s), probes every {} state(s); {}s\n",
            if self.ran >= WAVE_SEEDS {
                full.to_string()
            } else {
                format!("SHORT WAVE ({wave} is {WAVE_SEEDS} seeds: {command})")
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
            stage("probe"),
            stage("checkpoint"),
            if endings.is_empty() {
                "none".to_string()
            } else {
                endings
            },
            self.actions_max,
            self.turn_max,
            self.checkpoint_paused,
            self.checkpoint_mid,
            self.checkpoint_max_ms,
            FUZZ_POOL.len(),
            POOL_EXCLUSIONS.len(),
            hidden_stride(),
            unlisted_stride(),
            probe_stride(),
            round(self.elapsed_ms / MS_PER_SECOND),
        );
        let starts = self
            .start_counts()
            .map(|(start, n)| format!("{start}={n}"))
            .collect::<Vec<_>>()
            .join(" ");
        text.push_str(&format!("  starts: {starts}\n"));
        text.push_str(&format!("  endings channel: {} seed(s)\n", self.endings));
        text.push_str(&heaviest_lines(&self.heaviest));
        text
    }

    /// `--json`: the wave's failures (the handicapped wave's with their tier and seat, `null` in the
    /// gate's), each seed's start counted (#1503), each measure's heaviest actions and the growth
    /// bounds (#1254).
    fn json(&self) -> Value {
        json!({
            "from": self.from,
            "ran": self.ran,
            "passed": self.ran as usize - self.failures.len(),
            "failures": self.failures.iter().map(|failure| json!({
                "seed": failure.seed,
                "tier": failure.rotation.map(|(tier, _)| tier.to_string()),
        "seat": failure.rotation.map(|(_, seat)| seat.to_string()),
                "start": failure.start.as_str(),
                "stage": failure.stage.as_str(),
                "turn": failure.turn,
                "step": failure.actions,
                "message": failure.message,
            })).collect::<Vec<_>>(),
            "heaviest": self.heaviest,
            "starts": self.start_counts().collect::<IndexMap<_, _>>(),
            "endingsChannel": self.endings,
            "bounds": MEASURES
                .into_iter()
                .map(|measure| (measure.as_str(), measure.bound()))
                .collect::<IndexMap<_, _>>(),
            "checkpoint": {
                "paused": self.checkpoint_paused,
                "midGame": self.checkpoint_mid,
                "slowestMs": u64::try_from(self.checkpoint_max_ms).unwrap_or(u64::MAX),
                "boundMs": u64::try_from(CHECKPOINT_TIME_BOUND_MS).unwrap_or(u64::MAX),
            },
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
    // #1503: from the plain start, the one every kept failure was found from.
    run_seed(&GameSetup::plain(record.seed), record.decks.clone(), reduce)
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

impl SeedHandicap {
    /// The tier and seat, which a failure names.
    fn who(self) -> (Difficulty, PlayerId) {
        (self.tier, self.seat)
    }
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
                handicapped: false,
                from: 5001,
                ran,
                failures: Vec::new(),
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest: Heaviest::default(),
                elapsed_ms: 0.0,
                checkpoint_paused: 0,
                checkpoint_mid: 0,
                checkpoint_max_ms: 0,
                starts: IndexMap::new(),
                endings: 0,
            };
            assert!(wave(2500).summary().starts_with("[M4 fuzz] FULL GATE\n"));
            assert!(wave(WAVE_SEEDS).summary().starts_with("[M4 fuzz] FULL GATE\n"));
            assert!(wave(200).summary().starts_with("[M4 fuzz] SHORT WAVE"));
        }

        #[test]
        fn a_handicapped_wave_reports_under_its_own_name_and_command() {
            let wave = |ran| Wave {
                handicapped: true,
                from: 1,
                ran,
                failures: Vec::new(),
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest: Heaviest::default(),
                elapsed_ms: 0.0,
                checkpoint_paused: 0,
                checkpoint_mid: 0,
                checkpoint_max_ms: 0,
                starts: IndexMap::new(),
                endings: 0,
            };
            assert!(
                wave(WAVE_SEEDS)
                    .summary()
                    .starts_with("[fuzz-handicap] FULL HANDICAP WAVE\n")
            );
            let short = wave(200).summary();
            assert!(
                short.starts_with("[fuzz-handicap] SHORT WAVE (the handicapped wave is"),
                "{short}"
            );
            assert!(short.contains("cargo jackioh fuzz --handicap)"), "{short}");
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
                paused: Vec::new(),
            };
            assert_eq!(
                check_termination(&GameSetup::plain(1), &run)
                    .err()
                    .map(|failure| failure.message),
                None
            );
        }
    }

    mod fuzz_serial_restart {
        use super::*;

        #[test]
        fn checkpoint_step_prefers_a_paused_step_and_is_deterministic() {
            let stream = "jackioh-fuzz-checkpoint-1";
            let paused = vec![3, 7, 11];
            // The same seed draws the same step twice, and a non-empty paused list always wins
            // over the mid-game fallback.
            let first = checkpoint_step(stream, &paused, 20);
            assert_eq!(first, checkpoint_step(stream, &paused, 20));
            assert!(first.1, "{first:?}");
            assert!(paused.contains(&first.0), "{first:?}");
            // No paused state: the fallback halves the game, and the empty game checkpoints setup.
            assert_eq!(checkpoint_step(stream, &[], 20), (10, false));
            assert_eq!(checkpoint_step(stream, &[], 21), (10, false));
            assert_eq!(checkpoint_step(stream, &[], 0), (0, false));
        }

        /// A planted lossy restore: the JSON round trip, then one hero health point added.
        fn lossy(state: &GameState) -> Result<GameState, String> {
            let mut restored = round_trip(state)?;
            restored.players[PlayerId::P1].hero.health += 1;
            Ok(restored)
        }

        /// Seed 1's game, played and checked as the gate plays it (under the caller's preview).
        fn seed_one() -> GameRun {
            play_game(&GameSetup::gate(1), decks_for_seed(1), reduce)
                .unwrap_or_else(|failure| panic!("{}", failure.message))
        }

        /// Seed 1's checkpoint over `log` against `live`, as `fuzz_deal` checks it.
        fn seed_one_checkpoint(
            run: &GameRun,
            log: &[Action],
            live: &GameState,
        ) -> Result<CheckpointReport, FuzzFailure> {
            check_serial_restart(&GameSetup::gate(1), &run.decks, log, live, &run.paused)
        }

        #[test]
        fn a_planted_lossy_round_trip_fails_verification() {
            register_all();
            let _preview = preview_every_set();
            let begun = begin_game(&create_game(&CreateGameOptions {
                seed: "jackioh-fuzz-1".to_string(),
                decks: decks_for_seed(1),
                ..CreateGameOptions::default()
            }));
            let restored = verify_round_trip(&begun.state, hash_state, round_trip)
                .expect("the honest state round-trips");
            assert_eq!(hash_state(&restored), hash_state(&begun.state));
            let error = verify_round_trip(&begun.state, hash_state, lossy)
                .expect_err("one health point lost in the round trip must fail it");
            assert!(error.contains("the round trip changed the state"), "{error}");
        }

        #[test]
        fn a_restored_game_that_misses_the_live_hash_fails_in_the_checkpoint_stage() {
            let _preview = preview_every_set();
            let run = seed_one();
            let mut live = run.state.clone();
            live.players[PlayerId::P1].hero.health += 1;
            let Err(failure) = seed_one_checkpoint(&run, &run.log, &live) else {
                panic!("a live state the restored game never reaches must fail the checkpoint");
            };
            assert_eq!(failure.stage, FailureStage::Checkpoint, "{}", failure.message);
            assert!(
                failure.message.contains("the restored game diverged"),
                "{}",
                failure.message
            );
        }

        #[test]
        fn a_continuation_the_restored_game_refuses_fails_in_the_checkpoint_stage() {
            let _preview = preview_every_set();
            let run = seed_one();
            let mut log = run.log.clone();
            // A fresh nonce: a repeated one is answered as before, never refused (§9.3).
            let mut after = log.last().cloned().expect("seed 1 played an action");
            after.nonce = "planted-after-the-ending".to_string();
            log.push(after);
            let Err(failure) = seed_one_checkpoint(&run, &log, &run.state) else {
                panic!("an action after the ending must fail the checkpoint");
            };
            assert_eq!(failure.stage, FailureStage::Checkpoint, "{}", failure.message);
            assert!(
                failure.message.contains("continuation refused"),
                "{}",
                failure.message
            );
            assert!(
                failure.message.contains("the game is over"),
                "{}",
                failure.message
            );
        }

        #[test]
        fn one_seed_crosses_its_checkpoint_and_reports_cost() {
            let passed =
                fuzz_deal(1, decks_for_seed(1)).unwrap_or_else(|failure| panic!("{}", failure.message));
            assert!(passed.checkpoint_ms <= CHECKPOINT_TIME_BOUND_MS);
            // The direct check crosses the same checkpoint deterministically.
            let _preview = preview_every_set();
            let run = seed_one();
            let report = seed_one_checkpoint(&run, &run.log, &run.state)
                .unwrap_or_else(|failure| panic!("{}", failure.message));
            assert_eq!(report.paused, !run.paused.is_empty());
            assert!(report.ms <= CHECKPOINT_TIME_BOUND_MS);
            // The same seed crosses the same checkpoint again.
            let again = seed_one_checkpoint(&run, &run.log, &run.state)
                .unwrap_or_else(|failure| panic!("{}", failure.message));
            assert_eq!(again.paused, report.paused);
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

        /// `seed`'s handicapped deal with the grower first in both decks.
        fn planted_handicap_decks(seed: u32) -> (Vec<String>, Vec<String>) {
            let SeedHandicap { seat, handicap, .. } = handicap_for_seed(seed);
            let (mut p1, mut p2) = handicap_decks_for_seed(seed, seat, &handicap);
            p1[0] = GROWER.to_string();
            p2[0] = GROWER.to_string();
            (p1, p2)
        }

        #[test]
        fn a_planted_grower_in_handicapped_decks_fails_in_the_growth_stage() {
            install_grower();
            // The overrides are this thread's, so the seeds run here and not under rayon.
            let failure = (1..=PLANTED_SEEDS)
                .find_map(|seed| fuzz_handicap_deal(seed, planted_handicap_decks(seed), reduce).err())
                .expect("a card that grows without end must fail one of the planted handicapped seeds");
            clear_overrides();
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
            let decks = planted_handicap_decks(failure.seed);
            assert_eq!(failure.decks, decks);
            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(text.contains("[growth]"), "{text}");
            assert!(
                text.contains(&format!("seed {} p1 deck: {}", failure.seed, decks.0.join(", "))),
                "{text}"
            );
            assert!(
                text.contains(&format!("seed {} p2 deck: {}", failure.seed, decks.1.join(", "))),
                "{text}"
            );
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
                handicapped: false,
                from: 1,
                ran: 7,
                failures: Vec::new(),
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest,
                elapsed_ms: 0.0,
                checkpoint_paused: 0,
                checkpoint_mid: 0,
                checkpoint_max_ms: 0,
                starts: IndexMap::new(),
                endings: 0,
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
            assert!(summary.contains("0 paused, 0 mid-game fallback"), "{summary}");
            assert_eq!(json["checkpoint"]["paused"], json!(0));
            assert_eq!(json["checkpoint"]["midGame"], json!(0));
            assert_eq!(
                json["checkpoint"]["boundMs"],
                json!(u64::try_from(CHECKPOINT_TIME_BOUND_MS).unwrap_or(u64::MAX))
            );
        }
    }

    mod fuzz_probe_stage {
        use super::*;

        /// A reducer that ignores who sent an action and answers for the seat the game waits on.
        pub(super) fn seatless(state: &GameState, action: &Action) -> ReduceResult {
            let seat = seat_to_act(state).unwrap_or(state.active);
            reduce(
                state,
                &Action::new(action.body.clone(), seat, action.nonce.clone()),
            )
        }

        /// A reducer that forgets every nonce it has applied.
        fn forgetful(state: &GameState, action: &Action) -> ReduceResult {
            let mut fresh = state.clone();
            fresh.applied.clear();
            reduce(&fresh, action)
        }

        /// `probe` fails seed 1 in the probe stage, naming the probe called `name` and its step.
        fn assert_caught(probe: Reducer, name: &str) {
            register_all();
            let failure = fuzz_deal_probed(1, decks_for_seed(1), probe)
                .err()
                .expect("a reducer that breaks the probe must fail the seed");
            assert_eq!(failure.stage, FailureStage::Probe, "{}", failure.message);
            assert!(
                failure
                    .message
                    .starts_with(&format!("{name} probe after step {}:", failure.actions)),
                "{}",
                failure.message
            );
            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(text.contains("example: seed 1 ("), "{text}");
            assert!(text.contains("[probe]"), "{text}");
        }

        #[test]
        fn a_reducer_that_takes_the_wrong_seat_s_action_fails_the_wrong_seat_probe() {
            assert_caught(seatless, WRONG_SEAT_PROBE);
        }

        #[test]
        fn a_reducer_that_forgets_its_nonces_fails_the_reused_nonce_probe() {
            assert_caught(forgetful, REUSED_NONCE_PROBE);
        }

        /// A reducer that takes any answer from the prompt's holder as the prompt's first listed answer.
        fn lenient(state: &GameState, action: &Action) -> ReduceResult {
            if let (ActionBody::Answer { .. }, Some(pending)) = (&action.body, &state.pending)
                && pending.player_id == action.player_id
                && let Some(first) = jackioh_engine::prompt_answers(pending).into_iter().next()
            {
                return reduce(state, &Action::new(first, action.player_id, action.nonce.clone()));
            }
            reduce(state, action)
        }

        #[test]
        fn a_reducer_that_takes_a_malformed_answer_fails_the_malformed_answer_probe() {
            assert_caught(lenient, MALFORMED_ANSWER_PROBE);
        }

        #[test]
        fn a_handicapped_probe_failure_reports_tier_seat_stage_signature_and_both_decks() {
            register_all();
            // Seed 1 is Hard on p1, whose deck is longer than the other seat's.
            let SeedHandicap { seat, tier, handicap } = handicap_for_seed(1);
            let decks = handicap_decks_for_seed(1, seat, &handicap);
            let failure = fuzz_handicap_deal(1, decks.clone(), seatless)
                .err()
                .expect("a reducer that breaks the probe must fail the handicapped seed");
            assert_eq!(failure.stage, FailureStage::Probe, "{}", failure.message);
            assert_eq!((tier, seat), (Difficulty::Hard, PlayerId::P1));
            assert_eq!(failure.rotation, Some((tier, seat)));
            assert_eq!(failure.decks, decks);
            assert!(failure.decks.0.len() > failure.decks.1.len());
            assert_eq!(
                failure.signature,
                signature_of(FailureStage::Probe, &failure.message)
            );

            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(text.starts_with("fuzz-handicap: 1 of 1 seeds failed"), "{text}");
            assert!(text.contains("[probe]"), "{text}");
            assert!(
                text.contains(&format!(
                    "example: seed 1 (hard p1, turn {}, {} actions)",
                    failure.turn, failure.actions
                )),
                "{text}"
            );
            assert!(
                text.contains(&format!("seed 1 p1 deck: {}", decks.0.join(", "))),
                "{text}"
            );
            assert!(
                text.contains(&format!("seed 1 p2 deck: {}", decks.1.join(", "))),
                "{text}"
            );
            assert!(
                text.contains("cargo jackioh fuzz --from <seed> --seeds 1 --handicap"),
                "{text}"
            );

            // The rotation is not in the signature: the same bug on another tier and seat is one group.
            let other = Failure {
                seed: 2,
                rotation: Some(handicap_for_seed(2).who()),
                ..failure.clone()
            };
            let both = report(&[failure, other], 2, 0.0);
            assert!(both.contains("(1 distinct failure signature(s)"), "{both}");
            assert!(both.contains("seeds: 1, 2"), "{both}");

            // The gate's failure has no tier or seat to name.
            let gate = fuzz_deal_probed(1, decks_for_seed(1), seatless)
                .err()
                .expect("the gate's seed 1 fails under the same reducer");
            assert!(gate.rotation.is_none());
            let text = report(std::slice::from_ref(&gate), 1, 0.0);
            assert!(text.starts_with("M4 fuzz gate: 1 of 1 seeds failed"), "{text}");
            assert!(text.contains("example: seed 1 (turn "), "{text}");
            assert!(!text.contains("--handicap"), "{text}");
        }

        #[test]
        fn the_judge_passes_a_refusal_and_a_nonce_s_first_answer_and_names_a_moved_nonce_memory() {
            register_all();
            let _preview = preview_every_set();
            let state = begin_game(&create_game(&CreateGameOptions {
                seed: "jackioh-fuzz-1".to_string(),
                decks: decks_for_seed(1),
                ..CreateGameOptions::default()
            }))
            .state;
            let act = |state: &GameState, body: ActionBody, player: PlayerId, nonce: &str| {
                reduce(state, &Action::new(body, player, nonce.to_string()))
            };

            // 1. The mulligans are open, so End turn is refused with the state kept.
            let refused = act(&state, ActionBody::EndTurn, PlayerId::P1, "probe-test-1");
            assert!(refused.error.is_some());
            assert_eq!(judge_probe(&state, &refused, None), None);

            // 2. A nonce answers with its first action's events whatever came under it the second time.
            let kept = act(
                &state,
                ActionBody::Mulligan { keep: Vec::new() },
                PlayerId::P1,
                "probe-test-2",
            );
            assert_eq!(kept.error, None);
            let again = act(&kept.state, ActionBody::Concede, PlayerId::P2, "probe-test-2");
            assert_eq!(
                judge_probe(&kept.state, &again, Some(kept.events.as_slice())),
                None
            );

            // 3. An action that is accepted where a refusal was owed moves the state.
            let accepted = act(&state, ActionBody::Concede, PlayerId::P1, "probe-test-3");
            let wrong = judge_probe(&state, &accepted, None).expect("an accepted concede is no refusal");
            assert!(wrong.contains("was accepted"), "{wrong}");
            assert!(wrong.contains("moved the state"), "{wrong}");

            // 4. A refusal that remembered its nonce hashes as the state it refused, yet is not it.
            let mut remembered = refused.clone();
            remembered.state.applied = kept.state.applied.clone();
            assert_eq!(hash_state(&remembered.state), hash_state(&state));
            let wrong = judge_probe(&state, &remembered, None).expect("a moved nonce memory is no refusal");
            assert!(wrong.contains("moved the state"), "{wrong}");
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
            print!("{}", wave.summary());
            let detail = if wave.failures.is_empty() {
                String::new()
            } else {
                report(&wave.failures, size, wave.elapsed_ms)
            };
            assert_eq!(wave.failures.len(), 0, "{detail}");
        }

        #[test]
        fn a_game_setup_names_the_streams_and_handicaps_each_wave_has_always_played_under() {
            let gate = GameSetup::gate(7);
            assert_eq!(gate.stream(""), "jackioh-fuzz-7");
            assert_eq!(gate.stream("policy-"), "jackioh-fuzz-policy-7");
            assert_eq!(gate.stream("policy-order-"), "jackioh-fuzz-policy-order-7");
            assert_eq!(gate.stream("checkpoint-"), "jackioh-fuzz-checkpoint-7");
            assert_eq!(gate.handicaps(), None);

            // Seed 7 is Hard (odd), on p2 (7 / 2 = 3 is odd).
            let handicapped = GameSetup::handicapped(7);
            assert_eq!(handicapped.stream(""), "jackioh-fuzz-handicap-7");
            assert_eq!(handicapped.stream("policy-"), "jackioh-fuzz-handicap-policy-7");
            assert_eq!(
                handicapped.stream("policy-order-"),
                "jackioh-fuzz-handicap-policy-order-7"
            );
            assert_eq!(
                handicapped.stream("checkpoint-"),
                "jackioh-fuzz-handicap-checkpoint-7"
            );
            let handicaps = handicapped.handicaps().expect("a handicapped game has handicaps");
            assert_eq!(handicaps.p1, None);
            assert_eq!(handicaps.p2, Some(AI_DIFFICULTY[Difficulty::Hard]));
        }

        #[test]
        fn a_handicapped_game_that_ends_outside_over_fails_in_the_termination_stage() {
            register_all();
            let _preview = preview_every_set();
            let setup = GameSetup::handicapped(2);
            let SeedHandicap { seat, handicap, .. } = handicap_for_seed(2);
            let decks = handicap_decks_for_seed(2, seat, &handicap);
            let mut run = play_game(&setup, decks, reduce)
                .unwrap_or_else(|failure| panic!("[{}] {}", failure.stage.as_str(), failure.message));
            assert_eq!(run.state.phase, Phase::Over);
            assert_eq!(
                check_termination(&setup, &run)
                    .err()
                    .map(|failure| failure.message),
                None
            );
            run.state.phase = Phase::Main;
            let failure = check_termination(&setup, &run)
                .expect_err("a finished game still in the main phase must fail");
            assert_eq!(failure.stage, FailureStage::Termination, "{}", failure.message);
            assert!(
                failure
                    .message
                    .contains("finished in phase \"main\" rather than \"over\""),
                "{}",
                failure.message
            );
        }

        #[test]
        fn a_handicapped_wave_prints_as_one_json_object_with_each_failure_s_tier_and_seat() {
            let wave = play_handicap_wave(1, 2);
            let json = wave.json();
            assert_eq!(json["from"], json!(1));
            assert_eq!(json["ran"], json!(2));
            assert_eq!(json["bounds"]["events"], json!(GROWTH_EVENTS_BOUND));
            assert!(json["heaviest"]["events"].is_array(), "{json}");
            assert_eq!(json["failures"], json!([]), "{json}");

            let failure = Failure {
                seed: 1,
                rotation: Some(handicap_for_seed(1).who()),
                start: StartKind::Plain,
                stage: FailureStage::Growth,
                signature: String::new(),
                message: "step 3 grew".to_string(),
                decks: (Vec::new(), Vec::new()),
                turn: 4,
                actions: 3,
            };
            let failed = Wave {
                failures: vec![
                    failure.clone(),
                    Failure {
                        rotation: None,
                        ..failure
                    },
                ],
                ..wave
            };
            let json = failed.json();
            assert_eq!(json["failures"][0]["tier"], json!("hard"));
            assert_eq!(json["failures"][0]["seat"], json!("p1"));
            assert_eq!(json["failures"][0]["stage"], json!("growth"));
            assert!(json["failures"][1]["tier"].is_null(), "{json}");
            assert!(json["failures"][1]["seat"].is_null(), "{json}");
        }

        #[test]
        fn r679_seed_4679_whose_glitch_voids_the_match_passes() {
            // The daily run's handicapped seed that a Glitch voided at 96b68c9 (#1055).
            if let Err(failure) = fuzz_handicap_seed(4679) {
                panic!("[{}] {}", failure.stage.as_str(), failure.message);
            }
        }
    }

    mod fuzz_starting_states {
        use super::*;
        use jackioh_cards::CATALOG_VERSION;

        /// The seeds CI's `cargo jackioh fuzz --seeds 200` plays.
        const CI_SEEDS: u32 = 200;

        /// How many seeds the tests that plant a card try.
        const PLANTED_SEEDS: u32 = 20;

        /// C+ #29 Portal to the Past, which reads the last board (R417, R564).
        const PORTAL: &str = "classicplus-029";

        /// The first seed of `1..=CI_SEEDS` whose gate game draws `start`.
        fn first_seed_drawing(start: StartKind) -> u32 {
            (1..=CI_SEEDS)
                .find(|seed| GameSetup::gate(*seed).start == start)
                .unwrap_or_else(|| panic!("no seed of 1..={CI_SEEDS} draws {}", start.as_str()))
        }

        /// The three setup inputs `options` sets, in the order dealt, last boards, Glitch boards.
        fn inputs_set(options: &CreateGameOptions) -> (bool, bool, bool) {
            (
                options.dealt.is_some(),
                options.last_boards.is_some(),
                options.glitch_boards.is_some(),
            )
        }

        #[test]
        fn ci_s_seeds_draw_every_start_in_both_waves_and_a_corpus_record_draws_none() {
            let waves: [fn(u32) -> GameSetup; 2] = [GameSetup::gate, GameSetup::handicapped];
            for wave in waves {
                let drawn: IndexSet<&str> = (1..=CI_SEEDS).map(|seed| wave(seed).start.as_str()).collect();
                for start in START_KINDS {
                    assert!(drawn.contains(start.as_str()), "no seed draws {}", start.as_str());
                }
            }
            assert_eq!(GameSetup::gate(7).stream("start-"), "jackioh-fuzz-start-7");
            assert_eq!(
                GameSetup::handicapped(7).stream("start-inputs-"),
                "jackioh-fuzz-handicap-start-inputs-7"
            );
            for seed in 1..=CI_SEEDS {
                assert_eq!(GameSetup::plain(seed).start, StartKind::Plain);
            }
        }

        #[test]
        fn each_start_sets_its_own_create_game_input_and_the_fold_takes_the_same() {
            let decks = decks_for_seed(1);
            let plain = GameSetup::plain(1).create_options(decks.clone());
            assert_eq!(inputs_set(&plain), (false, false, false));
            for start in DRAWN_STARTS {
                let setup = GameSetup {
                    start,
                    ..GameSetup::plain(1)
                };
                let options = setup.create_options(decks.clone());
                assert_eq!(options, setup.create_options(decks.clone()), "{}", start.as_str());
                let only = match start {
                    StartKind::Plain => (false, false, false),
                    StartKind::Dealt => (true, false, false),
                    StartKind::LastBoards => (false, true, false),
                    StartKind::GlitchBoards => (false, false, true),
                };
                assert_eq!(inputs_set(&options), only, "{}", start.as_str());
                let without = CreateGameOptions {
                    dealt: None,
                    last_boards: None,
                    glitch_boards: None,
                    ..options.clone()
                };
                assert_eq!(without, plain, "{}", start.as_str());
                let fold = setup.fold_args(decks.clone(), Vec::new());
                assert_eq!(fold.seed, options.seed);
                assert_eq!(fold.decks, options.decks);
                assert_eq!(fold.dealt, options.dealt, "{}", start.as_str());
                assert_eq!(fold.last_boards, options.last_boards, "{}", start.as_str());
                assert_eq!(fold.glitch_boards, options.glitch_boards, "{}", start.as_str());
            }
        }

        #[test]
        fn a_manufactured_board_holds_field_cards_and_one_fusion_and_create_game_keeps_it_whole() {
            register_all();
            let _preview = preview_every_set();
            let most = usize::try_from(BOARD_MOST_CARDS).unwrap_or(0) + 1;
            for seed in 1..=PLANTED_SEEDS {
                let setup = GameSetup {
                    start: StartKind::LastBoards,
                    ..GameSetup::plain(seed)
                };
                let options = setup.create_options(decks_for_seed(seed));
                let (p1, p2) = options
                    .last_boards
                    .clone()
                    .expect("a last-boards start sets them");
                for board in [&p1, &p2] {
                    assert!(
                        (2..=most).contains(&board.len()),
                        "seed {seed}: {} cards",
                        board.len()
                    );
                    let (fused, cards) = board.split_last().expect("a board holds a fusion");
                    assert!(
                        fused.def_id.starts_with(BOARD_FUSED_HEAD),
                        "seed {seed}: {}",
                        fused.def_id
                    );
                    let ids: IndexSet<&str> = cards.iter().map(|entry| entry.def_id.as_str()).collect();
                    assert_eq!(ids.len(), cards.len(), "seed {seed}: a card twice");
                    for id in ids {
                        let def = CATALOG
                            .get(id)
                            .unwrap_or_else(|| panic!("seed {seed}: {id} is no card"));
                        assert_ne!(def.type_, CardType::Spell, "seed {seed}: {id}");
                    }
                }
                let kept = create_game(&options)
                    .last_boards
                    .expect("create_game freezes the boards");
                assert_eq!(kept.p1, Some(p1), "seed {seed}: R564 drops nothing");
                assert_eq!(kept.p2, Some(p2), "seed {seed}: R564 drops nothing");
            }
        }

        #[test]
        fn a_game_from_each_start_plays_monitor_clean_and_folds_back_to_its_live_hash() {
            register_all();
            let plain = {
                let _preview = preview_every_set();
                hash_state(&create_game(
                    &GameSetup::plain(1).create_options(decks_for_seed(1)),
                ))
            };
            for start in DRAWN_STARTS {
                let setup = GameSetup {
                    start,
                    ..GameSetup::plain(1)
                };
                let passed = run_seed(&setup, decks_for_seed(1), reduce).unwrap_or_else(|failure| {
                    panic!(
                        "{}: [{}] {}",
                        start.as_str(),
                        failure.stage.as_str(),
                        failure.message
                    )
                });
                assert_eq!(passed.start, start);
                let _preview = preview_every_set();
                let created = create_game(&setup.create_options(decks_for_seed(1)));
                assert_ne!(hash_state(&created), plain, "{} changes nothing", start.as_str());
            }
        }

        #[test]
        fn a_planted_portal_to_the_past_is_cast_from_a_manufactured_last_board_and_the_game_folds_back() {
            register_all();
            let _preview = preview_every_set();
            let setup = |seed| GameSetup {
                start: StartKind::LastBoards,
                ..GameSetup::plain(seed)
            };
            // A deck takes no duplicate card, so p1's first card gives way only when it is not Portal.
            let planted = |seed: u32| {
                let (mut p1, p2) = decks_for_seed(seed);
                if !p1.iter().any(|id| id == PORTAL) {
                    p1[0] = PORTAL.to_string();
                }
                (p1, p2)
            };
            let seed = (1..=PLANTED_SEEDS)
                .find(|seed| {
                    let decks = planted(*seed);
                    let created = create_game(&setup(*seed).create_options(decks.clone()));
                    let portal = created.players[PlayerId::P1]
                        .library
                        .iter()
                        .find(|card| card.def_id == PORTAL)
                        .map(|card| card.id.clone())
                        .expect("p1's library holds Portal at the setup");
                    let run = play_game(&setup(*seed), decks, reduce)
                        .unwrap_or_else(|failure| panic!("seed {seed}: {}", failure.message));
                    run.log.iter().any(|action| {
                        matches!(&action.body, ActionBody::Play { instance_id, .. } if *instance_id == portal)
                    })
                })
                .expect("one of the planted seeds casts Portal to the Past");
            if let Err(failure) = run_seed(&setup(seed), planted(seed), reduce) {
                panic!("seed {seed}: [{}] {}", failure.stage.as_str(), failure.message);
            }
        }

        #[test]
        fn a_seed_played_alone_starts_from_the_start_it_draws_in_its_wave() {
            for start in DRAWN_STARTS {
                let seed = first_seed_drawing(start);
                let wave = play_wave(seed, 1);
                assert!(wave.failures.is_empty(), "{}", report(&wave.failures, 1, 0.0));
                let counts: Vec<(&str, usize)> = wave.start_counts().collect();
                let expected: Vec<(&str, usize)> = START_KINDS
                    .into_iter()
                    .map(|kind| (kind.as_str(), usize::from(kind == start)))
                    .collect();
                assert_eq!(counts, expected);
                let name = start.as_str();
                assert!(
                    wave.summary().contains(&format!("{name}=1")),
                    "{}",
                    wave.summary()
                );
                assert_eq!(wave.json()["starts"][name], json!(1));
            }
        }

        #[test]
        fn a_failure_names_its_start_in_the_report_and_the_json_and_the_summary_counts_every_start() {
            let failure = Failure {
                seed: 3,
                rotation: None,
                start: StartKind::GlitchBoards,
                stage: FailureStage::Replay,
                signature: String::new(),
                message: "it broke".to_string(),
                decks: (Vec::new(), Vec::new()),
                turn: 2,
                actions: 1,
            };
            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(text.contains("\n  start: glitch-boards\n"), "{text}");
            let wave = Wave {
                handicapped: false,
                from: 3,
                ran: 1,
                failures: vec![failure],
                reasons: IndexMap::new(),
                actions_total: 0,
                actions_max: 0,
                turn_max: 0,
                heaviest: Heaviest::default(),
                elapsed_ms: 0.0,
                checkpoint_paused: 0,
                checkpoint_mid: 0,
                checkpoint_max_ms: 0,
                starts: IndexMap::from([("glitch-boards", 1)]),
                endings: 0,
            };
            let summary = wave.summary();
            assert!(
                summary.contains("\n  starts: plain=0 dealt=0 last-boards=0 glitch-boards=1\n"),
                "{summary}"
            );
            let json = wave.json();
            assert_eq!(json["failures"][0]["start"], json!("glitch-boards"));
            assert_eq!(
                json["starts"],
                json!({"plain": 0, "dealt": 0, "last-boards": 0, "glitch-boards": 1})
            );
        }

        #[test]
        fn a_corpus_record_replays_from_the_plain_start_whatever_start_its_seed_draws() {
            register_all();
            let seed = first_seed_drawing(StartKind::LastBoards);
            let record = CorpusRecord {
                seed,
                catalog_version: CATALOG_VERSION.to_string(),
                decks: decks_for_seed(seed),
                found: "a test's record".to_string(),
                retired: None,
            };
            let replayed = replay_record(&record).unwrap_or_else(|failure| panic!("{}", failure.message));
            assert_eq!(replayed.start, StartKind::Plain);
            let drawn = fuzz_seed(seed).unwrap_or_else(|failure| panic!("{}", failure.message));
            assert_eq!(drawn.start, StartKind::LastBoards);
            assert_ne!(replayed.hash, drawn.hash);
        }
    }

    mod fuzz_endings_channel {
        use super::*;
        use jackioh_engine::testkit::{
            CardScripts, Script, clear_overrides, json_as, register_catalog_as, register_scripts,
        };
        use jackioh_engine::{ActionType, CardDef, StaticFlags};

        /// The seeds the channel tests search: wide enough that a card joining the pool cannot leave
        /// a kind of action out of the games they play.
        const CHANNEL_SEEDS: u32 = 400;

        /// What the endings channel sends, by wire name: each must reach a log in the gate's waves.
        const CHANNEL_KINDS: [&str; 7] = [
            "concede",
            "offerDraw",
            "answerDraw",
            "timeout",
            "setAutoEndTurn",
            "disconnectExpired",
            "ceilingReached",
        ];

        /// A fixture Unit whose owner's opponent's emotes reach the board (Radiant M#71's flag), the
        /// only card that makes an emote legal (R1127).
        const LISTENER: &str = "fuzz-listener";

        /// `failure` as a panic that names its seed and stage, the report's own order.
        fn fail(seed: u32, failure: FuzzFailure) -> ! {
            panic!("seed {seed}: [{}] {}", failure.stage.as_str(), failure.message)
        }

        /// `setup`'s game played and held to termination, replay and the serialized restart, a
        /// failure of any panicking with its seed and stage.
        fn checked_run(setup: &GameSetup, decks: (Vec<String>, Vec<String>)) -> GameRun {
            let seed = setup.seed;
            let run = play_game(setup, decks, reduce).unwrap_or_else(|failure| fail(seed, failure));
            check_termination(setup, &run).unwrap_or_else(|failure| fail(seed, failure));
            check_replay(setup, &run).unwrap_or_else(|failure| fail(seed, failure));
            check_serial_restart(setup, &run.decks, &run.log, &run.state, &run.paused)
                .unwrap_or_else(|failure| fail(seed, failure));
            run
        }

        /// Puts the listener in this thread's catalog and scripts; `clear_overrides` takes it out.
        fn install_listener() {
            register_all();
            let face = json!({ "attack": 1, "health": 1, "keywords": [],
                "text": "Your opponent's emotes reach the board (fuzz fixture)." });
            let def: CardDef = json_as(
                json!({ "id": LISTENER, "index": LISTENER, "name": "Listener (fuzz fixture)",
                "set": "Core", "type": "Unit", "tags": [], "rarity": "Common", "token": false, "cost": 1,
                "base": face, "radiant": face }),
            );
            let mut catalog = (*CATALOG).clone();
            catalog.insert(LISTENER.to_string(), def);
            register_catalog_as(catalog, jackioh_cards::catalog_version());
            let script = Script {
                static_flags: Some(StaticFlags {
                    hears_emotes: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            };
            let mut scripts = jackioh_cards::scripts_of();
            scripts.insert(
                LISTENER.to_string(),
                CardScripts {
                    base: script.clone(),
                    radiant: script,
                },
            );
            register_scripts(scripts);
        }

        #[test]
        fn a_share_of_ci_s_seeds_takes_the_channel_in_both_waves_and_a_corpus_record_takes_none() {
            for setup_of in [GameSetup::gate, GameSetup::handicapped] {
                let taking = (1..=200).filter(|seed| setup_of(*seed).endings).count();
                assert!(
                    0 < taking && taking < 200,
                    "{taking} of 200 seeds take the channel"
                );
            }
            assert_eq!(GameSetup::gate(7).stream("endings-"), "jackioh-fuzz-endings-7");
            assert_eq!(
                GameSetup::handicapped(7).stream("endings-steps-"),
                "jackioh-fuzz-handicap-endings-steps-7"
            );
            assert!((1..=200).all(|seed| !GameSetup::plain(seed).endings));
        }

        #[test]
        fn the_gate_s_channel_games_log_its_actions_and_fold_back_from_a_concede_and_an_accepted_draw() {
            register_all();
            let _preview = preview_every_set();
            let mut kinds: IndexSet<String> = IndexSet::new();
            let mut declined = false;
            let mut reasons: IndexSet<String> = IndexSet::new();
            for seed in (1..=CHANNEL_SEEDS).filter(|seed| GameSetup::gate(*seed).endings) {
                let run = checked_run(&GameSetup::gate(seed), decks_for_seed(seed));
                kinds.extend(
                    run.log
                        .iter()
                        .map(|action| action.action_type().as_str().to_string()),
                );
                declined |= run
                    .log
                    .iter()
                    .any(|action| action.body == ActionBody::AnswerDraw { accept: false });
                reasons.extend(run.state.result.map(|result| result.reason.as_str().to_string()));
                let all_kinds = CHANNEL_KINDS.iter().all(|kind| kinds.contains(*kind));
                if all_kinds && declined && reasons.contains("concede") && reasons.contains("draw-accepted") {
                    break;
                }
            }
            for kind in CHANNEL_KINDS {
                assert!(
                    kinds.contains(kind),
                    "no channel game of seeds 1..={CHANNEL_SEEDS} logged \"{kind}\""
                );
            }
            assert!(declined, "no channel game logged a declined draw");
            for reason in ["concede", "draw-accepted"] {
                assert!(
                    reasons.contains(reason),
                    "no channel game ended by \"{reason}\": {reasons:?}"
                );
            }
        }

        #[test]
        fn a_planted_listener_s_opponent_emotes_through_the_channel_and_the_game_folds_back() {
            install_listener();
            let _preview = preview_every_set();
            // The overrides are this thread's, so the seeds run here and not under rayon.
            let emoted = (1..=CHANNEL_SEEDS)
                .filter(|seed| GameSetup::gate(*seed).endings)
                .find(|seed| {
                    let (mut p1, mut p2) = decks_for_seed(*seed);
                    p1[0] = LISTENER.to_string();
                    p2[0] = LISTENER.to_string();
                    let run = checked_run(&GameSetup::gate(*seed), (p1, p2));
                    run.log
                        .iter()
                        .any(|action| action.action_type() == ActionType::Emote)
                });
            clear_overrides();
            assert!(
                emoted.is_some(),
                "no planted channel game of seeds 1..={CHANNEL_SEEDS} logged an emote"
            );
        }

        #[test]
        fn a_game_without_the_channel_still_fails_on_each_channel_ending() {
            register_all();
            let _preview = preview_every_set();
            let decks = decks_for_seed(1);
            for reason in [
                GameOverReason::Concede,
                GameOverReason::DrawAccepted,
                GameOverReason::Disconnect,
                GameOverReason::MatchCeiling,
            ] {
                let mut state = create_game(&CreateGameOptions {
                    seed: "jackioh-fuzz-1".to_string(),
                    decks: decks.clone(),
                    ..CreateGameOptions::default()
                });
                state.result = Some(GameResult {
                    winner: Winner::Draw,
                    reason,
                });
                state.phase = Phase::Over;
                let run = GameRun {
                    state,
                    log: Vec::new(),
                    decks: decks.clone(),
                    heaviest: Heaviest::default(),
                    paused: Vec::new(),
                };
                let failure = check_termination(&GameSetup::plain(1), &run)
                    .expect_err("a game without the channel cannot end by one of its endings");
                assert_eq!(failure.stage, FailureStage::Termination, "{}", failure.message);
                assert!(
                    failure
                        .message
                        .contains(&format!("ended with reason \"{}\"", reason.as_str())),
                    "{}",
                    failure.message
                );
                let channel = GameSetup {
                    endings: true,
                    ..GameSetup::plain(1)
                };
                assert_eq!(
                    check_termination(&channel, &run)
                        .err()
                        .map(|failure| failure.message),
                    None
                );
            }
        }

        #[test]
        fn a_channel_seed_s_failure_names_its_seed_and_both_decks_and_the_summary_counts_the_channel() {
            register_all();
            let seed = (1..=200)
                .find(|seed| GameSetup::gate(*seed).endings)
                .expect("some seed of 1..=200 takes the channel");
            let decks = decks_for_seed(seed);
            let failure = fuzz_deal_probed(seed, decks.clone(), fuzz_probe_stage::seatless)
                .err()
                .expect("a reducer that breaks the wrong-seat probe must fail the seed");
            assert_eq!((failure.seed, &failure.decks), (seed, &decks));
            let text = report(std::slice::from_ref(&failure), 1, 0.0);
            assert!(text.contains(&format!("example: seed {seed} (")), "{text}");
            assert!(
                text.contains(&format!("seed {seed} p1 deck: {}", decks.0.join(", "))),
                "{text}"
            );
            assert!(
                text.contains(&format!("seed {seed} p2 deck: {}", decks.1.join(", "))),
                "{text}"
            );

            let wave = play_wave(seed, 1);
            assert_eq!(wave.endings, 1);
            let summary = wave.summary();
            assert!(summary.contains("\n  endings channel: 1 seed(s)\n"), "{summary}");
            assert_eq!(wave.json()["endingsChannel"], json!(1));
        }
    }
}
