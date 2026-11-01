//! `cargo jackioh golden check [--file <path> [--seed K]…]` and `cargo jackioh golden record --seeds A..B
//! [--force ids] --out <path>` (SURFACE §12, §13; part 23).
//!
//! `check` with no `--file` folds the hotseat fixture, which must fold to `golden::HOTSEAT_HASH`, and
//! exits 1 unless it does. `check --file <path>` replays a traces file against this build with
//! `jackioh_engine::testkit::golden::replay_game` (§13.3, #1641): for each line `create_game(args)`,
//! `begin_game`, check `begin`; then for each step check `l` (the actor's legal actions, as a set) for
//! the actor `a.playerId`, apply `a` with `reduce`, check `s` (`hash_state`), `v` (both seats'
//! `view_for`) and `e` (the step's events); last, the ending and the length must be the line's `end`. A
//! game's first mismatch names the seed, the step, which hash and the action, and writes this build's
//! canonical text to `target/golden-diff/<seed>-<step>-<which>.json`. The hotseat fixture is folded too
//! unless `--seed` picks seeds (which needs `--file`). Games run in parallel (rayon); the report lists
//! every divergent game, earliest seed first. Exit 1 on any divergence.
//!
//! `record --seeds A..B [--force ids] --out <path>` plays the golden seeds A..=B (each within 1–240,
//! so the decks, game seeds and policy streams are §13.1's) with the recorder into a file of its own,
//! playing each seed exactly as `scripts/golden/record.ts` did: the same decks, game seeds, policy and
//! mulligan-order streams, nonces and step cap, and the same line format. Each id of `--force` goes
//! into both decks, in the first slots, in place of the deck's own cards from the end (a deck keeps its
//! size and holds no card twice). A token cannot be in a deck (§2.6 L3), so one is forced through the
//! card whose `refs` name it (R279), the smallest such id of a card that can be in a deck; a token no
//! deck card names, a card of a set that has not shipped (R1420) and an id the catalog lacks are
//! refused. A forced card the deck already holds takes its own place (`fuzz::deal_deck`); a Core card
//! and its Legacy card are two cards, forced together (R2511001).
//! The file replays with `check --file` on this build or on any other: `luau diff` and `classify`
//! record with it and have a parent build replay it.
//!
//! The 240 golden games once committed as `crates/engine/tests/golden/`'s traces, and `bless`, which
//! rewrote them, were retired in #2578; `golden bless` now fails, pointing to `record`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::replay::fnv1a32_utf16;
use jackioh_engine::rng::Rng;
use jackioh_engine::subsystems::choose_action;
use jackioh_engine::testkit::golden::{
    FORMAT_VERSION, Mismatch, check_hotseat, events_text, hashed_state_text, legal_text, replay_game,
    seed_number, view_texts,
};
use jackioh_engine::{
    Action, CardDef, CreateGameArgs, GameEvent, GameOverReason, GameState, Handicap, PerPlayerOpt, Winner,
    begin_game, create_game, deckable, hash_state, reduce,
};

use crate::fuzz::{
    SHIPPED_FUZZ_POOL, SHIPPED_HANDICAP_POOL, SeedHandicap, actor_of, deal_deck, decks_for_seed_from,
    handicap_decks_for_seed_from, handicap_for_seed,
};

// ---------------------------------------------------------------------------------------------
// The recording's shape (§13.1, §13.3), as scripts/golden/record.ts has it
// ---------------------------------------------------------------------------------------------

/// §13.3: where a divergence leaves the Rust side's canonical text (the workspace's `target/`).
const DIFF_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/golden-diff");

/// §13.1: seeds PLAIN_FIRST..=PLAIN_LAST are fuzz.test.ts's games.
const PLAIN_FIRST: u32 = 1;
const PLAIN_LAST: u32 = 200;

/// §13.1: seeds HANDICAP_FIRST..=HANDICAP_LAST are fuzz-handicap.test.ts's games.
const HANDICAP_FIRST: u32 = 201;
const HANDICAP_LAST: u32 = 240;

/// §13.1: a game is recorded to its end or to this many actions, whichever comes first.
const STEP_CAP: usize = 3000;

// ---------------------------------------------------------------------------------------------
// The command line (SURFACE §12)
// ---------------------------------------------------------------------------------------------

/// `golden check [--file <path> [--seed K]…]`, `golden record --seeds A..B [--force ids] --out <path>`.
#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fold the hotseat fixture; with --file, replay a recorded traces file too; exit 1 on a divergence.
    Check(CheckArgs),
    /// Retired with the 240 committed golden games (#2578).
    #[command(hide = true)]
    Bless(RetiredArgs),
    /// Record golden seeds with cards forced into both decks, into another file.
    Record(RecordArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    /// The traces to replay, as `golden record` writes them; without it, the hotseat fixture alone.
    #[arg(long, value_name = "PATH")]
    file: Option<PathBuf>,
    /// Replay only this seed (the number `record.ts --seed` takes); repeatable; needs --file.
    #[arg(long = "seed", value_name = "K", requires = "file")]
    seeds: Vec<u32>,
}

/// Whatever `golden bless` is given, taken so every old invocation gets `BLESS_RETIRED`.
#[derive(clap::Args)]
struct RetiredArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    given: Vec<String>,
}

/// #2578: what `golden bless` answers since the 240 committed games it rewrote were retired.
const BLESS_RETIRED: &str = "golden bless is retired with the 240 committed golden games (#2578): \
     record games with `cargo jackioh golden record --seeds A..B [--force ids] --out <path>` and \
     replay them on any build with `cargo jackioh golden check --file <path>`";

#[derive(clap::Args)]
struct RecordArgs {
    /// The golden seeds to play, first..=last (each within 1–240).
    #[arg(long, value_name = "A..B")]
    seeds: String,
    /// Card ids to put into both decks; a token is forced through a card whose `refs` name it (R279).
    #[arg(long, value_name = "IDS", value_delimiter = ',')]
    force: Vec<String>,
    /// The file to write.
    #[arg(long, value_name = "PATH")]
    out: PathBuf,
}

pub fn run(args: Args) -> Result<()> {
    register_all();
    match args.command {
        Command::Check(check) => run_check(&check),
        Command::Bless(_) => bail!("{BLESS_RETIRED}"),
        Command::Record(record) => run_record(&record),
    }
}

// ---------------------------------------------------------------------------------------------
// The texts `check` compares (§13.2, `golden`'s)
// ---------------------------------------------------------------------------------------------

/// R1365000: this build's canonical text for one comparison `check` makes in a recorded game, with
/// the state the text was read from: `s`, `v-p1`, `v-p2` and `e` after `step` (a step number, or
/// `begin`), `l` before it. `None` for a comparison with no text (`refused`, `line`, `end`).
/// `classify` holds it against the parent's side of a divergence to name the cards it touches.
pub(crate) fn text_at(game: &Value, step: &str, which: &str) -> Result<Option<(String, GameState)>> {
    let args: CreateGameArgs =
        serde_json::from_value(game["args"].clone()).context("a recorded game's createGame options")?;
    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        bail!("the recorded game does not begin: {error}");
    }
    let (mut state, mut events) = (begun.state, begun.events);
    if step != "begin" {
        let n: usize = step
            .parse()
            .with_context(|| format!("step {step} is not a step number"))?;
        let steps = game["steps"]
            .as_array()
            .context("a recorded game with no steps")?;
        for (k, line) in steps.iter().enumerate().take(n + 1) {
            let action: Action = serde_json::from_value(line["a"].clone()).context("a recorded action")?;
            if k == n && which == "l" {
                return Ok(Some((legal_text(&state, action.player_id), state)));
            }
            let result = reduce(&state, &action);
            if let Some(error) = result.error {
                bail!("step {k} of the recorded game is refused: {error}");
            }
            (state, events) = (result.state, result.events);
        }
    }
    let text = match which {
        "s" => hashed_state_text(&state),
        "v-p1" | "v-p2" => {
            let [p1, p2] = view_texts(&state);
            if which == "v-p1" { p1 } else { p2 }
        }
        "e" => events_text(&events),
        _ => return Ok(None),
    };
    Ok(Some((text, state)))
}

// ---------------------------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------------------------

/// §13.3: write the Rust side's canonical text where `record.ts --dump-step` writes TS's, and say
/// where it went.
fn write_diff(mismatch: &mut Mismatch) {
    let Some(text) = &mismatch.text else {
        return;
    };
    let dir = Path::new(DIFF_DIR);
    let path = dir.join(mismatch.diff_file());
    let written = match fs::create_dir_all(dir).and_then(|()| fs::write(&path, format!("{text}\n"))) {
        Ok(()) => path.display().to_string(),
        Err(error) => format!("(could not write {}: {error})", path.display()),
    };
    mismatch.diff = Some(written);
}

/// One game's replay: the number of steps replayed, or where it diverged.
type Replayed = Result<usize, Box<Mismatch>>;

fn run_check(args: &CheckArgs) -> Result<()> {
    let Some(file) = &args.file else {
        return match check_hotseat() {
            Ok(hash) => {
                println!("hotseat fixture: folds to {hash}");
                Ok(())
            }
            Err(message) => {
                println!("hotseat fixture: {message}");
                bail!("{message}")
            }
        };
    };
    let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
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
                        text: None,
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
            let outcome = replay_game(&seed, &game).map_err(|mut mismatch| {
                write_diff(&mut mismatch);
                mismatch
            });
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
            file.display(),
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
            "{} of {games} golden games diverged from their recorded trace",
            failures.len()
        );
    }
    if let Some(Err(message)) = hotseat {
        bail!("{message}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The decks and policy streams, dealt by `fuzz.rs`, as scripts/golden/record.ts copied them from
// the fuzz files
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

/// Plays `spec`'s game as `record.ts` does (fuzz.test.ts's `playGame` loop, nonce `golden-<n>`, stopped
/// at the game's end or STEP_CAP) and records it: the one recorder `record` and the bench share.
fn record_game(spec: GameSpec) -> Result<GameLine> {
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

// ---------------------------------------------------------------------------------------------
// record: the same games with cards forced into both decks
// ---------------------------------------------------------------------------------------------

/// `--seeds A..B` as the seeds A through B.
fn parse_seed_range(text: &str) -> Result<Vec<u32>> {
    let bad = || anyhow!("--seeds takes first..last, two numbers with \"..\" between them, not {text:?}");
    let (first, last) = text.split_once("..").ok_or_else(bad)?;
    let first: u32 = first.trim().parse().map_err(|_| bad())?;
    let last: u32 = last.trim().parse().map_err(|_| bad())?;
    if first > last {
        bail!("--seeds {text}: {first} is after {last}");
    }
    Ok((first..=last).collect())
}

/// What forcing a card into a deck puts there for the catalog entry `def` of `id`: the card itself, or for
/// a token the card whose `refs` name it (R279); a card of a set that has not shipped is refused (R1420).
pub(crate) fn carrier_of(id: &str, def: &CardDef) -> Result<String> {
    if def.is_token() {
        CATALOG
            .iter()
            .filter(|(_, carrier)| {
                deckable(carrier)
                    && carrier
                        .refs
                        .as_ref()
                        .is_some_and(|refs| refs.iter().any(|named| named == id))
            })
            .map(|(carrier_id, _)| carrier_id)
            .min()
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "{id}: no deckable card's refs name this token (R279: it comes from a pool \
                     or a rule); force the card that makes it"
                )
            })
    } else if deckable(def) {
        Ok(id.to_string())
    } else {
        bail!("{id}: {:?} has not shipped (R1420)", def.set);
    }
}

/// What `--force id` puts in a deck: the id as it is, or for a token the card whose `refs` name it
/// (R279); an id the catalog lacks is refused.
pub(crate) fn resolve_forced(id: &str) -> Result<String> {
    let def = CATALOG
        .get(id)
        .ok_or_else(|| anyhow!("{id}: not in the catalog"))?;
    carrier_of(id, def)
}

/// What `--force` puts in a deck: each id as `resolve_forced` reads it; no id twice, in the order
/// given (a Core card and its Legacy card are two, R2511001).
pub(crate) fn resolve_force(ids: &[String]) -> Result<Vec<String>> {
    let mut forced: Vec<String> = Vec::new();
    for id in ids {
        let resolved = resolve_forced(id)?;
        if forced.contains(&resolved) {
            continue;
        }
        forced.push(resolved);
    }
    Ok(forced)
}

/// `deck` with `forced` in its first slots and as many of its own cards after them as fit: the same
/// size and no card twice (`fuzz::deal_deck`);
/// `fuzz --cards` forces with it too (#2387). No rng (`begin_game` shuffles the library).
pub(crate) fn force_into(deck: &[String], forced: &[String]) -> Vec<String> {
    deal_deck(&mut forced.iter().chain(deck).cloned(), deck.len())
}

/// Seed k's game with `forced` in both decks.
fn record_forced(k: u32, forced: &[String]) -> Result<GameLine> {
    let mut spec = spec_for_seed(k)?;
    for deck in [&mut spec.decks.0, &mut spec.decks.1] {
        if forced.len() > deck.len() {
            bail!(
                "{}: {} forced cards do not fit a deck of {}",
                spec.seed,
                forced.len(),
                deck.len()
            );
        }
        *deck = force_into(deck, forced);
    }
    record_game(spec)
}

/// What `record_seeds` wrote.
pub(crate) struct Recorded {
    pub games: usize,
    pub steps: usize,
    /// The cards that went into both decks, tokens resolved to their carriers.
    pub forced: Vec<String>,
}

/// Plays each of `seeds` with `force` in both decks and writes the lines, in seed order, to `out`.
pub(crate) fn record_seeds(seeds: &[u32], force: &[String], out: &Path) -> Result<Recorded> {
    let forced = resolve_force(force)?;
    let games: Vec<GameLine> = seeds
        .par_iter()
        .map(|&k| record_forced(k, &forced))
        .collect::<Result<Vec<_>>>()?;
    let steps: usize = games.iter().map(|game| game.steps.len()).sum();

    let mut text = String::new();
    for game in &games {
        text.push_str(&serde_json::to_string(game)?);
        text.push('\n');
    }
    fs::write(out, text).with_context(|| format!("writing {}", out.display()))?;
    Ok(Recorded {
        games: games.len(),
        steps,
        forced,
    })
}

fn run_record(args: &RecordArgs) -> Result<()> {
    let seeds = parse_seed_range(&args.seeds)?;
    let recorded = record_seeds(&seeds, &args.force, &args.out)?;
    let forced = if recorded.forced.is_empty() {
        "none".to_string()
    } else {
        recorded.forced.join(", ")
    };
    println!(
        "golden record: wrote {} games ({} steps) to {}, forced: {forced}",
        recorded.games,
        recorded.steps,
        args.out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn a_seed_range_is_inclusive_and_runs_forward() {
        assert_eq!(parse_seed_range("1..3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_seed_range("5..5").unwrap(), vec![5]);
        for bad in ["3..1", "1-3", "x..3", "1.."] {
            assert!(parse_seed_range(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_forced_card_takes_the_first_slots_and_the_deck_keeps_its_size() {
        assert_eq!(
            force_into(&ids(&["a", "b", "c", "d", "e"]), &ids(&["c", "x"])),
            ids(&["c", "x", "a", "b", "d"])
        );
    }

    #[test]
    fn r2511001_a_forced_card_the_deck_holds_takes_its_own_place_and_a_legacy_card_goes_beside_its_core_card()
    {
        assert_eq!(
            force_into(&ids(&["core-002", "core-001", "core-003"]), &ids(&["core-002"])),
            ids(&["core-002", "core-001", "core-003"])
        );
        assert_eq!(
            force_into(
                &ids(&["traininggrounds-041", "core-001", "core-003"]),
                &ids(&["core-002"])
            ),
            ids(&["core-002", "traininggrounds-041", "core-001"])
        );
        assert_eq!(
            resolve_force(&ids(&["core-002", "traininggrounds-041"])).unwrap(),
            ids(&["core-002", "traininggrounds-041"])
        );
    }

    #[test]
    fn a_token_is_forced_through_the_first_card_by_id_whose_refs_name_it() {
        // classic-032, core-062 and core-098 all name the Felinor token; the smallest id carries it,
        // and a card named twice is forced once.
        assert_eq!(
            resolve_force(&ids(&["core-t-felinor", "core-090"])).unwrap(),
            ids(&["classic-032", "core-090"])
        );
        assert_eq!(
            resolve_force(&ids(&["core-t-sheep", "core-041"])).unwrap(),
            ids(&["core-041"])
        );
    }

    #[test]
    fn a_card_nothing_can_carry_into_a_deck_is_refused() {
        // No deckable card's refs name the Glitch token.
        for id in ["classic-t-glitch", "no-such-card"] {
            assert!(resolve_force(&ids(&[id])).is_err(), "{id}");
        }
        // The Coin was such a token until the Meditative set shipped in v0.3.5: Meditative #27
        // Clip-Farming Lawyer names it, so it carries it now.
        assert_eq!(
            resolve_force(&ids(&["core-t-coin"])).unwrap(),
            ids(&["meditative-027"])
        );
    }
}
