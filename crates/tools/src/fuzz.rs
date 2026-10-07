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
//!   1. NO THROW       — playing the game out raises nothing (a panic, here).
//!   2. TERMINATION    — the game reaches a real `state.result` (hero death, both heroes dead, or
//!                       the R2 turn cap at TURN_CAP_PLAYER_TURNS player-turns), inside a bounded
//!                       number of actions. The bound is not a pass condition: hitting it is a
//!                       reported failure, so a non-terminating game fails loudly instead of
//!                       hanging CI (the policy never concedes or offers a draw, R84, so those
//!                       endings cannot occur here).
//!   3. REPLAY EQUALITY— folding `(seed, decks, log)` in a fresh `create_game` reproduces the same
//!                       state hash, with no rejected actions. This is what makes SPEC §9.3's
//!                       "(seed, log) reconstructs any match" true.
//!   4. FAILURE DETAIL — every card id in the failing seed's two decks is named in the report,
//!                       because a bare seed number is useless against a 100-card pool.
//! And at every step, INVARIANTS: `testkit::invariants`'s I1–I5 (summoning sickness and exertion,
//! R171, and nothing after game over, R216) hold on the state each action is chosen in and on the
//! state it produces (stage "invariant"), and I6 (no card a seat may not read in its view or legal
//! actions, §10.8) holds for both seats on every state, setup's included: every state under
//! `cargo test`, every I6_GATE_STRIDE-th under the gate.
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
//!   cargo test -p jackioh-tools fuzz                seeds 1–20 of each (see SWEEP_SEEDS)
//!   cargo jackioh fuzz --from <seed> --seeds 1      one seed a report named
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
//! included, to the live hash. Each game also runs the gate's invariant monitor (I1–I6) the same way.

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{LazyLock, Once};
use std::time::Instant;

use anyhow::bail;
use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::testkit::{I6_GATE_STRIDE, create_invariant_monitor};
use jackioh_engine::{
    AI_DIFFICULTY, AI_PLAYOUT_STEP_CAP, Action, ActionBody, CreateGameOptions, DECK_SIZE, Difficulty, FoldArgs,
    GameState, Handicap, PerPlayerOpt, Phase, PlayerId, Rng, TURN_CAP_PLAYER_TURNS, Tag, begin_game, create_game,
    fold, hash_state, mulligan_owed, reduce, seat_to_act, subsystems,
};
use rayon::prelude::*;

/// `cargo jackioh fuzz [--from N] [--seeds N] [--handicap]` (SURFACE §12).
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The first seed of the wave (TS `JACKIOH_FUZZ_FROM`): 1 for the gate. `--from 700 --seeds 1`
    /// replays exactly the seed a report named, without the 699 before it.
    #[arg(long)]
    pub from: Option<u32>,
    /// How many seeds this run plays (TS `JACKIOH_FUZZ_SEEDS`): narrows the wave by hand while chasing
    /// a failure, and can never widen it past WAVE_SEEDS.
    #[arg(long)]
    pub seeds: Option<u32>,
    /// Play the handicapped wave (fuzz-handicap.test.ts) instead of the gate: one seat on Medium or
    /// Hard, rotating by seed.
    #[arg(long)]
    pub handicap: bool,
}

/// Runs the gate, or with `--handicap` the handicapped wave; prints its numbers and, when a seed
/// failed, the report, and fails (exit 1).
pub fn run(args: Args) -> anyhow::Result<()> {
    let from = first_seed(args.from);
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
    print!("{}", wave.summary());
    if !wave.failures.is_empty() {
        println!("{}", report(&wave.failures, ran, wave.elapsed_ms));
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

/// How many seeds this run plays (from `first_seed()`, which is 1 unless a developer moves it).
/// `--seeds 25` narrows it by hand while chasing a failure; it can never widen the wave past
/// WAVE_SEEDS, and the summary line always reports which seeds actually ran.
pub fn wave_size(raw: Option<u32>) -> u32 {
    match raw {
        Some(parsed) if parsed >= 1 => parsed.min(WAVE_SEEDS),
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

/// §2.5 plus R84: the policy never concedes or offers a draw, so only these three can occur.
const TERMINAL_REASONS: &[&str] = &["hero-death", "both-heroes-dead", "turn-cap"];

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
/// minus `POOL_EXCLUSIONS`. Sorted by catalog id so the pool a seed shuffles is identical on every
/// machine and in every process, whatever order the registry handed the defs over.
pub static FUZZ_POOL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .filter(|id| !EXCLUDED_IDS.contains(id.as_str()))
        .collect();
    pool.sort();
    pool
});

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
    let mut rng = Rng::new(&format!("jackioh-fuzz-decks-{seed}"), 0);
    let shuffled = rng.shuffle(FUZZ_POOL.as_slice());
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
}

impl FailureStage {
    fn as_str(self) -> &'static str {
        match self {
            FailureStage::Play => "play",
            FailureStage::Termination => "termination",
            FailureStage::Replay => "replay",
            FailureStage::Invariant => "invariant",
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
fn actor_of(state: &GameState, order: &mut Rng) -> PlayerId {
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
fn play_game(seed: u32) -> Result<GameRun, FuzzFailure> {
    let game_seed = format!("jackioh-fuzz-{seed}");
    let decks = decks_for_seed(seed);
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
    let max_actions = max_actions_per_game();
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

        log.push(action);
        state = result.state;

        let mut broken = monitor.after(&result.events, &state);
        if log.len() % stride == 0 || state.result.is_some() {
            broken.extend(monitor.hidden(&state));
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
    }

    Ok(GameRun { state, log, decks })
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
    let mut index = at;
    for expected in needle.chars() {
        if chars.get(index) != Some(&expected) {
            return false;
        }
        index += 1;
    }
    true
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
        if chars[at] == 'R' && boundary_at(&chars, at) && chars.get(at + 1).is_some_and(char::is_ascii_digit) {
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
    let shape = blank_numbers(&blank_hashes(&blank_instance_ids(&blank_card_ids(&blank_quoted(
        message,
    )))));
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
        Some(at) if at.contains("crates/") && !at.contains("crates/tools/src/fuzz.rs") => format!(" [at {at}]"),
        _ => String::new(),
    }
}

/// A panic's message, as `${error.name}: ${error.message}` read it.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with no message".to_string()
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
    ordered.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let deck_size = usize::try_from(DECK_SIZE).unwrap_or(0);
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "M4 fuzz gate: {} of {ran} seeds failed ({} distinct failure signature(s), {}s).",
        failures.len(),
        ordered.len(),
        round(elapsed_ms / 1000.0)
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
                "  in EVERY failing seed of this group: none — the cause is shared, not a single card".to_string()
            } else {
                format!(
                    "  in EVERY failing seed of this group ({}): {}",
                    always.len(),
                    always.iter().map(|id| describe_card(id)).collect::<Vec<_>>().join("; ")
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
    reason: String,
    actions: usize,
    turn: i32,
}

/// One seed through the four assertions: never panic, always terminate by hero death or the R2 cap,
/// and fold `(seed, log)` to the same hash; a failure names every card in this seed's decks.
fn fuzz_seed(seed: u32) -> Result<Passed, Failure> {
    let mut reached: Option<At> = None;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<Passed, FuzzFailure> {
        // 1. never panic
        let run = play_game(seed)?;
        reached = Some(At {
            turn: run.state.turn,
            actions: run.log.len(),
        });
        // 2. always terminate, by hero death or the R2 cap
        check_termination(&run)?;
        // 3. (seed, log) folds to the same hash
        check_replay(seed, &run)?;
        Ok(Passed {
            reason: run
                .state
                .result
                .map_or_else(|| "none".to_string(), |result| result.reason.as_str().to_string()),
            actions: run.log.len(),
            turn: run.state.turn,
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
                format!("panic: {}{}", panic_message(&*payload), origin_of(location.as_deref())),
                reached.map_or(-1, |at| i64::from(at.turn)),
                reached.map_or(-1, |at| i64::try_from(at.actions).unwrap_or(i64::MAX)),
            )
        }
    };
    // 4. name every card in this seed's decks
    let decks = decks_for_seed(seed);
    let mut cards = decks.0;
    cards.extend(decks.1);
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
    elapsed_ms: f64,
}

/// Plays seeds `from..from + ran`, in parallel, and gathers them back in seed order.
fn play_wave(from: u32, ran: u32) -> Wave {
    record_panic_locations();
    register_all();
    let started_at = Instant::now();
    let results: Vec<Result<Passed, Failure>> = (from..from + ran).into_par_iter().map(fuzz_seed).collect();
    let elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0;

    let mut wave = Wave {
        from,
        ran,
        failures: Vec::new(),
        reasons: IndexMap::new(),
        actions_total: 0,
        actions_max: 0,
        turn_max: 0,
        elapsed_ms,
    };
    for result in results {
        match result {
            Ok(passed) => {
                *wave.reasons.entry(passed.reason).or_insert(0) += 1;
                wave.actions_total += passed.actions;
                wave.actions_max = wave.actions_max.max(passed.actions);
                wave.turn_max = wave.turn_max.max(passed.turn);
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
        format!(
            "[M4 fuzz] {}\n  seeds {}..{}: {passed} passed, {} failed\n  {} panic(s), {} over the {}-action bound or stalled, {} replay mismatch(es), {} invariant violation(s)\n  endings: {}\n  {average} actions/game on average, {} at most; longest game ended on turn {} of {TURN_CAP_PLAYER_TURNS}\n  pool: {} deck-legal cards, {} excluded; I6 every {} state(s); {}s\n",
            if self.ran == WAVE_SEEDS {
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
            if endings.is_empty() { "none".to_string() } else { endings },
            self.actions_max,
            self.turn_max,
            FUZZ_POOL.len(),
            POOL_EXCLUSIONS.len(),
            hidden_stride(),
            round(self.elapsed_ms / 1000.0),
        )
    }
}

// ---------------------------------------------------------------------------------------------
// The handicapped wave (fuzz-handicap.test.ts)
// ---------------------------------------------------------------------------------------------

/// Every deck-legal card, sorted, as the gate's FUZZ_POOL (its exclusion list is empty).
static HANDICAP_POOL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    pool
});

/// Which seat is handicapped, and how.
#[derive(Clone, Copy, Debug)]
pub struct SeedHandicap {
    pub seat: PlayerId,
    pub tier: Difficulty,
    pub handicap: Handicap,
}

/// Which seat is handicapped, and how: seeds rotate over Medium and Hard and over p1 and p2.
pub fn handicap_for_seed(seed: u32) -> SeedHandicap {
    let tier = if seed % 2 == 1 { Difficulty::Hard } else { Difficulty::Medium };
    let seat = if (seed / 2) % 2 == 0 { PlayerId::P1 } else { PlayerId::P2 };
    SeedHandicap {
        seat,
        tier,
        handicap: AI_DIFFICULTY[tier],
    }
}

/// One shuffle of the pool: the handicapped seat takes its deckSize cards, the other seat the next 20
/// (TS `decksForSeed` of fuzz-handicap.test.ts, renamed beside the gate's own).
fn handicap_decks_for_seed(seed: u32, seat: PlayerId, handicap: &Handicap) -> (Vec<String>, Vec<String>) {
    let shuffled = Rng::new(&format!("jackioh-fuzz-handicap-decks-{seed}"), 0).shuffle(HANDICAP_POOL.as_slice());
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
            if log.len() % stride == 0 || state.result.is_some() {
                broken.extend(monitor.hidden(&state));
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
        });
        if !replayed.errors.is_empty() {
            return Err(format!(
                "replay rejected {} action(s): {}",
                replayed.errors.len(),
                replayed.errors.first().map_or(String::new(), |first| first.error.clone())
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
                outcome
                    .error
                    .as_ref()
                    .map(|error| format!("seed {} ({} {}): {error}", outcome.seed, outcome.tier, outcome.seat))
            })
            .collect()
    }

    fn summary(&self) -> String {
        let failed = self.outcomes.iter().filter(|outcome| outcome.error.is_some()).count();
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

    mod fuzz_m4_gate {
        use super::*;

        #[test]
        fn draws_both_decks_from_the_seed_alone() {
            let deck_size = usize::try_from(DECK_SIZE).unwrap_or(0);
            assert!(FUZZ_POOL.len() >= deck_size * 2);
            for seed in [1, 500, 1000] {
                let (p1, p2) = decks_for_seed(seed);
                assert_eq!(decks_for_seed(seed), (p1.clone(), p2.clone()), "seed {seed} must redraw identically");
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
            assert_eq!(origin_of(Some("crates/engine/src/zones.rs:10:5")), " [at crates/engine/src/zones.rs:10:5]");
            assert_eq!(origin_of(Some("crates/tools/src/fuzz.rs:10:5")), "");
            assert_eq!(origin_of(None), "");
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
    }
}
