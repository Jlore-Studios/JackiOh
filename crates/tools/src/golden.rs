//! `cargo jackioh golden check` and `cargo jackioh golden bless [--seeds N]` (SURFACE §12, §13;
//! part 23).
//!
//! `check` replays `crates/engine/tests/golden/games.jsonl` against the Rust engine with the same
//! logic as `crates/engine/tests/golden.rs` (§13.3): for each line `create_game(args)`, `begin_game`,
//! check `begin`; then for each step check `l` (the actor's legal actions, as a set) for the actor
//! `a.playerId`, apply `a` with `reduce`, check `s` (`hash_state`), `v` (both seats' `view_for`) and
//! `e` (the step's events); last, the ending and the length must be the line's `end`. A game's first
//! mismatch names the seed, the step, which hash and the action, writes the Rust side's canonical
//! text to `target/golden-diff/<seed>-<step>-<which>.json`, and prints the `record.ts --dump-step`
//! command that writes TS's side next to it, run in a checkout of 05f5cfd (the last commit that has
//! the TypeScript). The hotseat fixture must fold to "a798906b". Games run in
//! parallel (rayon); the report lists every divergent game, earliest seed first. Exit 1 on any
//! divergence.
//!
//! `bless` rewrites `games.jsonl` from the Rust engine, playing each seed exactly as
//! `scripts/golden/record.ts` does (and so as `packages/cards/test/fuzz.test.ts` and
//! `fuzz-handicap.test.ts` did): the same decks, game seeds, policy and mulligan-order streams,
//! nonces and step cap, and the same line format. It is for an intended rules change only — v0.3.0
//! has none (parts 32–35 never edit a trace) — because a blessed file is no longer TypeScript's
//! oracle but the Rust engine's own record. `--seeds N` (at most 200) records plain seeds 1..=N and
//! handicapped seeds 201..=200+N/5 with them, so the default 200 is §13.1's 240 games and 150 is the
//! brief's smaller fallback (1–150 and 201–230).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};

use jackioh_cards::register_all;
use jackioh_engine::replay::{canonical, fnv1a32_utf16};
use jackioh_engine::rng::Rng;
use jackioh_engine::subsystems::choose_action;
use jackioh_engine::{
    Action, CreateGameArgs, FoldArgs, GameEvent, GameOverReason, GameState, Handicap, PerPlayerOpt, PlayerId,
    Winner, begin_game, create_game, fold, hash_state, legal_actions, reduce, view_for,
};

use crate::fuzz::{
    SHIPPED_FUZZ_POOL, SHIPPED_HANDICAP_POOL, SeedHandicap, actor_of, decks_for_seed_from,
    handicap_decks_for_seed_from, handicap_for_seed,
};

// ---------------------------------------------------------------------------------------------
// The recording's shape (§13.1, §13.3), as scripts/golden/record.ts has it
// ---------------------------------------------------------------------------------------------

/// §13.3: the file `check` reads and `bless` writes, by default.
const GAMES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/golden/games.jsonl");

/// §13.3: the hotseat fixture, folded by `check` too.
const HOTSEAT: &str = include_str!("../../engine/tests/golden/01-hotseat-full-game.json");

/// The hotseat fixture's final state hash (`hotseat-replay.test.ts`'s `EXPECTED_HASH`, SURFACE §5.2).
const HOTSEAT_HASH: &str = "a798906b";

/// §13.3: where a divergence leaves the Rust side's canonical text (the workspace's `target/`).
const DIFF_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/golden-diff");

/// §13.3: the file format's version, each line's first key.
const FORMAT_VERSION: u32 = 1;

/// §13.1: seeds PLAIN_FIRST..=PLAIN_LAST are fuzz.test.ts's games.
const PLAIN_FIRST: u32 = 1;
const PLAIN_LAST: u32 = 200;

/// §13.1: seeds HANDICAP_FIRST..=HANDICAP_LAST are fuzz-handicap.test.ts's games.
const HANDICAP_FIRST: u32 = 201;
const HANDICAP_LAST: u32 = 240;

/// `bless --seeds N` records one handicapped game per this many plain ones (200 → 40, 150 → 30).
const PLAIN_PER_HANDICAPPED: u32 = 5;

/// §13.1: a game is recorded to its end or to this many actions, whichever comes first.
const STEP_CAP: usize = 3000;

// ---------------------------------------------------------------------------------------------
// The command line (SURFACE §12)
// ---------------------------------------------------------------------------------------------

/// `golden check`, `golden bless [--seeds N]`.
#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Replay every golden game and the hotseat fixture against the Rust engine; exit 1 on a divergence.
    Check(CheckArgs),
    /// Rewrite games.jsonl from the Rust engine. Only after an intended rules change: v0.3.0 has none.
    Bless(BlessArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    /// The traces to replay.
    #[arg(long, value_name = "PATH", default_value = GAMES_PATH)]
    file: PathBuf,
    /// Replay only this seed (the number `record.ts --seed` takes); repeatable.
    #[arg(long = "seed", value_name = "K")]
    seeds: Vec<u32>,
}

#[derive(clap::Args)]
struct BlessArgs {
    /// Plain seeds 1..=N (at most 200), with handicapped seeds 201..=200+N/5.
    #[arg(long, value_name = "N", default_value_t = PLAIN_LAST)]
    seeds: u32,
    /// The file to write.
    #[arg(long, value_name = "PATH", default_value = GAMES_PATH)]
    file: PathBuf,
}

pub fn run(args: Args) -> Result<()> {
    register_all();
    match args.command {
        Command::Check(check) => run_check(&check),
        Command::Bless(bless) => run_bless(&bless),
    }
}

// ---------------------------------------------------------------------------------------------
// Hashing (§13.2, with §5.2's functions)
// ---------------------------------------------------------------------------------------------

/// §5.2 step 1: the state's canonical text as `hash_state` hashes it, without `applied` and `opening`.
fn hashed_state_text(state: &GameState) -> String {
    let mut value = serde_json::to_value(state).expect("a GameState serialises");
    if let Some(object) = value.as_object_mut() {
        object.remove("applied");
        object.remove("opening");
    }
    canonical(&value)
}

/// The canonical text of both seats' views, p1 first.
fn view_texts(state: &GameState) -> [String; 2] {
    [PlayerId::P1, PlayerId::P2].map(|player| {
        canonical(&serde_json::to_value(view_for(state, player)).expect("a PlayerView serialises"))
    })
}

fn events_text(events: &[GameEvent]) -> String {
    canonical(&serde_json::to_value(events).expect("GameEvents serialise"))
}

/// §13.2's `l`: the actor's legal actions as a set — each action's canonical text, distinct, sorted
/// by UTF-16 code unit (JS's default `sort`), joined by "\n".
fn legal_text(state: &GameState, actor: PlayerId) -> String {
    let mut lines: Vec<String> = legal_actions(state, actor)
        .iter()
        .map(|action| canonical(&serde_json::to_value(action).expect("an ActionBody serialises")))
        .collect();
    lines.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    lines.dedup();
    lines.join("\n")
}

// ---------------------------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------------------------

/// A game's first divergence from its trace.
struct Mismatch {
    seed: String,
    /// `begin`, the 0-based step index, or `end`.
    step: String,
    /// `s`, `v-p1`, `v-p2`, `e`, `l`, `refused` (reduce refused the recorded action), `end`, or
    /// `line` (the line itself is not a trace).
    which: &'static str,
    expected: String,
    actual: String,
    /// The recorded action, as JSON, for a step.
    action: Option<String>,
    /// Where the Rust side's canonical text was written, when there is one to write.
    diff: Option<String>,
}

impl Mismatch {
    fn report(&self) -> String {
        let mut text = format!(
            "seed {} step {}: `{}` expected {}, Rust has {}",
            self.seed, self.step, self.which, self.expected, self.actual
        );
        if let Some(action) = &self.action {
            text.push_str(&format!("\n    action: {action}"));
        }
        if let Some(diff) = &self.diff {
            text.push_str(&format!("\n    Rust side: {diff}"));
        }
        if let Some(k) = seed_number(&self.seed) {
            // A refusal is compared with the state the previous step left.
            let step = match (self.which, self.step.parse::<usize>()) {
                ("refused", Ok(0)) => "begin".to_string(),
                ("refused", Ok(n)) => (n - 1).to_string(),
                _ => self.step.clone(),
            };
            if step != "end" {
                text.push_str(&format!(
                    "\n    TS side:   in a checkout of 05f5cfd, the last commit with the TypeScript \
                     (git worktree add ../jackioh-ts 05f5cfd, then pnpm install there):\
                     \n               pnpm exec tsx scripts/golden/record.ts --seed {k} --dump-step {step}"
                ));
            }
        }
        text
    }
}

/// The seed number `record.ts --seed` takes: the trailing number of `jackioh-fuzz-<k>` or
/// `jackioh-fuzz-handicap-<k>`.
fn seed_number(seed: &str) -> Option<u32> {
    seed.rsplit('-').next().and_then(|tail| tail.parse::<u32>().ok())
}

/// §13.3: write the Rust side's canonical text where `record.ts --dump-step` writes TS's, and say
/// where it went.
fn write_diff(seed: &str, step: &str, which: &str, text: &str) -> String {
    let dir = Path::new(DIFF_DIR);
    let path = dir.join(format!("{seed}-{step}-{which}.json"));
    match fs::create_dir_all(dir).and_then(|()| fs::write(&path, format!("{text}\n"))) {
        Ok(()) => path.display().to_string(),
        Err(error) => format!("(could not write {}: {error})", path.display()),
    }
}

/// The recorded hash at `key` (a string), or a description of what is there instead.
fn recorded(snapshot: &Value, key: &str) -> String {
    snapshot[key]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("<no `{key}` in the line>"))
}

/// Checks `s`, `v` and `e` of one snapshot (`begin`, or the state a step left with its events).
fn check_snapshot(
    seed: &str,
    step: &str,
    action: Option<&str>,
    snapshot: &Value,
    state: &GameState,
    events: &[GameEvent],
) -> Result<(), Box<Mismatch>> {
    let mismatch = |which: &'static str, expected: String, actual: String, text: &str| {
        Box::new(Mismatch {
            seed: seed.to_string(),
            step: step.to_string(),
            which,
            expected,
            actual,
            action: action.map(str::to_string),
            diff: Some(write_diff(seed, step, which, text)),
        })
    };

    let s = hash_state(state);
    let expected = recorded(snapshot, "s");
    if s != expected {
        return Err(mismatch("s", expected, s, &hashed_state_text(state)));
    }

    for (index, (text, which)) in view_texts(state).into_iter().zip(["v-p1", "v-p2"]).enumerate() {
        let hash = fnv1a32_utf16(&text);
        let expected = snapshot["v"][index]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("<no `v[{index}]` in the line>"));
        if hash != expected {
            return Err(mismatch(which, expected, hash, &text));
        }
    }

    let text = events_text(events);
    let hash = fnv1a32_utf16(&text);
    let expected = recorded(snapshot, "e");
    if hash != expected {
        return Err(mismatch("e", expected, hash, &text));
    }
    Ok(())
}

/// One game's replay: the number of steps replayed, or where it diverged.
type Replayed = Result<usize, Box<Mismatch>>;

/// Replays one parsed line of `games.jsonl` (§13.3). `Ok` carries the number of steps replayed.
fn replay_game(seed: &str, game: &Value) -> Replayed {
    let fail = |step: &str, which: &'static str, expected: String, actual: String| {
        Box::new(Mismatch {
            seed: seed.to_string(),
            step: step.to_string(),
            which,
            expected,
            actual,
            action: None,
            diff: None,
        })
    };
    if game["v"].as_u64() != Some(u64::from(FORMAT_VERSION)) {
        return Err(fail(
            "begin",
            "line",
            format!("format version {FORMAT_VERSION}"),
            game["v"].to_string(),
        ));
    }
    let args: CreateGameArgs = match serde_json::from_value(game["args"].clone()) {
        Ok(args) => args,
        Err(error) => {
            return Err(fail(
                "begin",
                "line",
                "createGame's options".into(),
                error.to_string(),
            ));
        }
    };

    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        return Err(fail(
            "begin",
            "refused",
            "beginGame to set the game up".into(),
            format!("an error: {error}"),
        ));
    }
    let mut state = begun.state;
    check_snapshot(seed, "begin", None, &game["begin"], &state, &begun.events)?;

    let Some(steps) = game["steps"].as_array() else {
        return Err(fail("begin", "line", "a `steps` array".into(), "none".into()));
    };
    for (n, step) in steps.iter().enumerate() {
        let label = n.to_string();
        let action_json = step["a"].to_string();
        let action: Action = match serde_json::from_value(step["a"].clone()) {
            Ok(action) => action,
            Err(error) => {
                return Err(Box::new(Mismatch {
                    action: Some(action_json),
                    ..*fail(&label, "line", "an Action".into(), error.to_string())
                }));
            }
        };

        // `l` for the actor, on the state the action is chosen in.
        let text = legal_text(&state, action.player_id);
        let hash = fnv1a32_utf16(&text);
        let expected = recorded(step, "l");
        if hash != expected {
            return Err(Box::new(Mismatch {
                action: Some(action_json),
                diff: Some(write_diff(seed, &label, "l", &text)),
                ..*fail(&label, "l", expected, hash)
            }));
        }

        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            return Err(Box::new(Mismatch {
                action: Some(action_json),
                diff: Some(write_diff(seed, &label, "refused", &hashed_state_text(&state))),
                ..*fail(
                    &label,
                    "refused",
                    "reduce to apply the recorded action".into(),
                    format!("a refusal: {error}"),
                )
            }));
        }
        state = result.state;
        check_snapshot(seed, &label, Some(&action_json), step, &state, &result.events)?;
    }

    let ending = json!({
        "winner": state.result.as_ref().map(|result| result.winner),
        "reason": state.result.as_ref().map(|result| result.reason),
        "steps": steps.len(),
    });
    if ending != game["end"] {
        return Err(fail("end", "end", game["end"].to_string(), ending.to_string()));
    }
    Ok(steps.len())
}

/// `hotseat-replay.test.ts`'s first assertion: the fixture folds with no refusal to its hash.
fn check_hotseat() -> Result<String, String> {
    let input: FoldArgs = serde_json::from_str(HOTSEAT)
        .map_err(|error| format!("the hotseat fixture is not a fold's input: {error}"))?;
    let replayed = fold(&input);
    if let Some(first) = replayed.errors.first() {
        return Err(format!(
            "the hotseat fixture's fold refused {} action(s); first at nonce \"{}\": {}",
            replayed.errors.len(),
            first.nonce,
            first.error
        ));
    }
    let hash = hash_state(&replayed.state);
    if hash != HOTSEAT_HASH {
        return Err(format!(
            "the hotseat fixture folds to {hash}, not {HOTSEAT_HASH} (SURFACE §5.2)"
        ));
    }
    Ok(hash)
}

fn run_check(args: &CheckArgs) -> Result<()> {
    let text = fs::read_to_string(&args.file).with_context(|| format!("reading {}", args.file.display()))?;
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .collect();

    // `None` for a game the --seed filter leaves out.
    let outcomes: Vec<Option<(String, Replayed)>> = lines
        .par_iter()
        .map(|&(index, line)| {
            let game: Value = match serde_json::from_str(line) {
                Ok(game) => game,
                Err(error) => {
                    let seed = format!("line-{}", index + 1);
                    let outcome = Err(Box::new(Mismatch {
                        seed: seed.clone(),
                        step: "begin".into(),
                        which: "line",
                        expected: "a JSON line".into(),
                        actual: error.to_string(),
                        action: None,
                        diff: None,
                    }));
                    return Some((seed, outcome));
                }
            };
            let seed = game["seed"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("line-{}", index + 1));
            if !args.seeds.is_empty() && !seed_number(&seed).is_some_and(|k| args.seeds.contains(&k)) {
                return None;
            }
            let outcome = replay_game(&seed, &game);
            Some((seed, outcome))
        })
        .collect();

    let mut games = 0usize;
    let mut steps = 0usize;
    let mut failures: Vec<Box<Mismatch>> = Vec::new();
    for (_, outcome) in outcomes.into_iter().flatten() {
        games += 1;
        match outcome {
            Ok(n) => steps += n,
            Err(mismatch) => failures.push(mismatch),
        }
    }
    if games == 0 {
        bail!(
            "no golden game in {} matches --seed {:?}",
            args.file.display(),
            args.seeds
        );
    }

    for failure in &failures {
        println!("{}", failure.report());
    }
    let hotseat = if args.seeds.is_empty() {
        Some(check_hotseat())
    } else {
        None
    };
    match &hotseat {
        Some(Ok(hash)) => println!("hotseat fixture: folds to {hash}"),
        Some(Err(message)) => println!("hotseat fixture: {message}"),
        None => {}
    }
    println!(
        "golden check: {} of {games} games replayed identically ({steps} steps), {} diverged",
        games - failures.len(),
        failures.len()
    );

    if !failures.is_empty() {
        bail!(
            "{} of {games} golden games diverged from their TypeScript trace",
            failures.len()
        );
    }
    if let Some(Err(message)) = hotseat {
        bail!("{message}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// bless: the decks and policy streams, dealt by `fuzz.rs`, as scripts/golden/record.ts copied them
// from the fuzz files
// ---------------------------------------------------------------------------------------------

/// One seed's game: its setup and its two policy streams.
struct GameSpec {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    /// R180, R187: the handicapped seat's handicap, or `None` for a game on this spec's resources.
    handicaps: Option<PerPlayerOpt<Handicap>>,
    /// The policy's stream (§10.7), separate from the game rng, as the fuzz has it.
    policy: String,
    /// R265: the stream that picks which seat answers first while both mulligans are open.
    order: String,
}

/// §13.1: seed k's game, dealt and seeded as the fuzz file that owns k deals it, from the shipped
/// sets alone (R1420), so a set still being built never moves a recorded game.
fn spec_for_seed(k: u32) -> Result<GameSpec> {
    if (HANDICAP_FIRST..=HANDICAP_LAST).contains(&k) {
        let SeedHandicap { seat, handicap, .. } = handicap_for_seed(k);
        let mut handicaps = PerPlayerOpt::default();
        *handicaps.slot(seat) = Some(handicap);
        return Ok(GameSpec {
            seed: format!("jackioh-fuzz-handicap-{k}"),
            decks: handicap_decks_for_seed_from(k, seat, &handicap, &SHIPPED_HANDICAP_POOL),
            handicaps: Some(handicaps),
            policy: format!("jackioh-fuzz-handicap-policy-{k}"),
            order: format!("jackioh-fuzz-handicap-policy-order-{k}"),
        });
    }
    if (PLAIN_FIRST..=PLAIN_LAST).contains(&k) {
        return Ok(GameSpec {
            seed: format!("jackioh-fuzz-{k}"),
            decks: decks_for_seed_from(k, &SHIPPED_FUZZ_POOL),
            handicaps: None,
            policy: format!("jackioh-fuzz-policy-{k}"),
            order: format!("jackioh-fuzz-policy-order-{k}"),
        });
    }
    bail!(
        "seed {k} is not a golden seed: {PLAIN_FIRST}–{PLAIN_LAST} are fuzz.test.ts's games and \
         {HANDICAP_FIRST}–{HANDICAP_LAST} fuzz-handicap.test.ts's (SURFACE §13.1)"
    )
}

/// §13.3's line, fields in the file's key order.
#[derive(Serialize)]
struct GameLine {
    v: u32,
    seed: String,
    args: LineArgs,
    begin: Hashes,
    steps: Vec<StepLine>,
    end: Ending,
}

#[derive(Serialize)]
struct LineArgs {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    /// `null` for a game without one, as the TS recorder writes it.
    handicaps: Option<PerPlayerOpt<Handicap>>,
}

#[derive(Serialize)]
struct Hashes {
    s: String,
    v: (String, String),
    e: String,
}

#[derive(Serialize)]
struct StepLine {
    a: Action,
    l: String,
    s: String,
    v: (String, String),
    e: String,
}

#[derive(Serialize)]
struct Ending {
    winner: Option<Winner>,
    reason: Option<GameOverReason>,
    steps: usize,
}

fn hashes_of(state: &GameState, events: &[GameEvent]) -> Hashes {
    let [p1, p2] = view_texts(state);
    Hashes {
        s: hash_state(state),
        v: (fnv1a32_utf16(&p1), fnv1a32_utf16(&p2)),
        e: fnv1a32_utf16(&events_text(events)),
    }
}

/// Plays seed k's game as `record.ts` does (fuzz.test.ts's `playGame` loop, nonce `golden-<n>`,
/// stopped at the game's end or STEP_CAP) and records it.
fn record_seed(k: u32) -> Result<GameLine> {
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
    let begin = hashes_of(&state, &begun.events);

    let mut policy = Rng::new(&spec.policy, 0);
    // R265: while both mulligans are open either seat may answer first; a stream of its own picks
    // which, so the fuzz plays both orders (the game is the same either way, R265).
    let mut order = Rng::new(&spec.order, 0);
    let mut steps: Vec<StepLine> = Vec::new();

    while state.result.is_none() && steps.len() < STEP_CAP {
        let n = steps.len();
        let player = actor_of(&state, &mut order);
        let l = fnv1a32_utf16(&legal_text(&state, player));
        let chosen = choose_action(&state, player, &mut policy).ok_or_else(|| {
            anyhow!(
                "{} step {n}: no legal action for {player} while the game is live (turn {}): R82 auto-ends \
                 a turn with nothing left to do, so this is a stall",
                spec.seed,
                state.turn
            )
        })?;
        let action = Action::new(chosen, player, format!("golden-{n}"));
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            bail!(
                "{} step {n}: legalActions offered \"{}\" but reduce refused it: {error}",
                spec.seed,
                action.action_type()
            );
        }
        state = result.state;
        let Hashes { s, v, e } = hashes_of(&state, &result.events);
        steps.push(StepLine {
            a: action,
            l,
            s,
            v,
            e,
        });
    }

    Ok(GameLine {
        v: FORMAT_VERSION,
        seed: spec.seed.clone(),
        args: LineArgs {
            seed: spec.seed,
            decks: spec.decks,
            handicaps: spec.handicaps,
        },
        begin,
        end: Ending {
            winner: state.result.as_ref().map(|result| result.winner),
            reason: state.result.as_ref().map(|result| result.reason),
            steps: steps.len(),
        },
        steps,
    })
}

fn run_bless(args: &BlessArgs) -> Result<()> {
    if args.seeds < PLAIN_FIRST || args.seeds > PLAIN_LAST {
        bail!("--seeds takes {PLAIN_FIRST}–{PLAIN_LAST}, not {}", args.seeds);
    }
    let handicapped = args.seeds / PLAIN_PER_HANDICAPPED;
    let seeds: Vec<u32> = (PLAIN_FIRST..=args.seeds)
        .chain(HANDICAP_FIRST..HANDICAP_FIRST + handicapped)
        .collect();

    let games: Vec<GameLine> = seeds
        .par_iter()
        .map(|&k| record_seed(k))
        .collect::<Result<Vec<_>>>()?;
    let steps: usize = games.iter().map(|game| game.steps.len()).sum();

    let mut text = String::new();
    for game in &games {
        text.push_str(&serde_json::to_string(game)?);
        text.push('\n');
    }
    fs::write(&args.file, text).with_context(|| format!("writing {}", args.file.display()))?;
    println!(
        "golden bless: wrote {} games ({} plain, {handicapped} handicapped, {steps} steps) to {}",
        games.len(),
        args.seeds,
        args.file.display()
    );
    Ok(())
}
