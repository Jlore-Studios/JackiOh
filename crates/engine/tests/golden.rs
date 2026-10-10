//! Golden traces (§13) and the recorded hotseat game: the oracle the engine is held to (README V4).
//!
//! `golden/games.jsonl` holds 240 games, one per line: seeds 1–200 plain, 201–240 with one seat on
//! Medium or Hard (R180–R184). Each is replayed as §13.3 says: `create_game(args)`, `begin_game`, check
//! `begin`; then per step check `l` (the actor's legal actions, as a set), apply `a` with `reduce`, and
//! check `s` (`hash_state`, §5.2), `v` (both seats' `view_for`) and `e` (the events), each an FNV-1a 32
//! of the canonical JSON over its UTF-16 code units; last, the ending and length must be `end`.
//!
//! A game's first mismatch names the seed, step, hash and action, writes the Rust side's canonical text
//! to `target/golden-diff/<seed>-<step>-<which>.json` and prints the command that writes the recorded
//! side's. Every game is replayed, so fix the earliest divergent seed and step first. The games are
//! split over GOLDEN_SHARDS tests to run on many threads (the engine's `clippy.toml` bans spawning one).
//! Never edit a trace by hand; `cargo jackioh golden bless` rewrites the file from the Rust engine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::replay::{canonical, fnv1a32_utf16};
use jackioh_engine::{
    Action, CreateGameArgs, FoldArgs, FoldResult, GameEvent, GameOverReason, GameResult, GameState, Phase,
    PlayerId, Winner, begin_game, create_game, fold, hash_state, legal_actions, reduce, view_for,
};

// The golden traces (§13)

/// §13.3: the recorded traces.
const GAMES: &str = include_str!("golden/games.jsonl");

/// §13.1: 200 games on this spec's resources and 40 with one seat handicapped.
const GOLDEN_GAMES: usize = 240;

/// §13.3: the file format's version, each line's `v`.
const FORMAT_VERSION: u64 = 1;

/// §13.3: where a divergence leaves the Rust side's canonical text (the workspace's `target/`).
const DIFF_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/golden-diff");

/// A game's first divergence from its trace.
struct Mismatch {
    seed: String,
    /// `begin`, the 0-based step index, or `end`.
    step: String,
    /// `s`, `v-p1`, `v-p2`, `e`, `l`, `refused` (reduce refused the recorded action) or `end`.
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
            // A refusal or an ending is compared with the state the previous step left.
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

/// §13.3: write the Rust side's canonical text next to the recorded side's, and say where it went.
#[allow(clippy::disallowed_methods)] // SURFACE §13.3: the diff file is this harness's output; clippy.toml's I/O ban guards the rules, which never reach this.
fn write_diff(seed: &str, step: &str, which: &str, text: &str) -> String {
    let dir = PathBuf::from(DIFF_DIR);
    let path = dir.join(format!("{seed}-{step}-{which}.json"));
    match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, format!("{text}\n"))) {
        Ok(()) => path.display().to_string(),
        Err(error) => format!("(could not write {}: {error})", path.display()),
    }
}

/// §5.2 step 1: the state's canonical text as `hash_state` hashes it, without `applied` and `opening`.
fn hashed_state_text(state: &GameState) -> String {
    let mut value = serde_json::to_value(state).expect("a GameState serialises");
    if let Some(object) = value.as_object_mut() {
        object.remove("applied");
        object.remove("opening");
    }
    canonical(&value)
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

    for (index, (player, which)) in [(PlayerId::P1, "v-p1"), (PlayerId::P2, "v-p2")]
        .into_iter()
        .enumerate()
    {
        let view = serde_json::to_value(view_for(state, player)).expect("a PlayerView serialises");
        let text = canonical(&view);
        let hash = fnv1a32_utf16(&text);
        let expected = snapshot["v"][index]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("<no `v[{index}]` in the line>"));
        if hash != expected {
            return Err(mismatch(which, expected, hash, &text));
        }
    }

    let text = canonical(&serde_json::to_value(events).expect("GameEvents serialise"));
    let hash = fnv1a32_utf16(&text);
    let expected = recorded(snapshot, "e");
    if hash != expected {
        return Err(mismatch("e", expected, hash, &text));
    }
    Ok(())
}

/// Replays one line of `games.jsonl` (§13.3). `Ok` carries the number of steps replayed.
fn replay_line(index: usize, line: &str) -> Result<usize, Box<Mismatch>> {
    let game: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("games.jsonl line {}: not JSON: {error}", index + 1));
    let seed = game["seed"]
        .as_str()
        .unwrap_or_else(|| panic!("games.jsonl line {}: no `seed`", index + 1))
        .to_string();
    assert_eq!(
        game["v"].as_u64(),
        Some(FORMAT_VERSION),
        "games.jsonl line {} ({seed}): format version",
        index + 1
    );
    let args: CreateGameArgs = serde_json::from_value(game["args"].clone())
        .unwrap_or_else(|error| panic!("{seed}: `args` is not createGame's options: {error}"));

    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        return Err(Box::new(Mismatch {
            seed,
            step: "begin".to_string(),
            which: "refused",
            expected: "beginGame to set the game up".to_string(),
            actual: format!("an error: {error}"),
            action: None,
            diff: None,
        }));
    }
    let mut state = begun.state;
    check_snapshot(&seed, "begin", None, &game["begin"], &state, &begun.events)?;

    let steps = game["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("{seed}: no `steps`"));
    for (n, step) in steps.iter().enumerate() {
        let label = n.to_string();
        let action_json = step["a"].to_string();
        let action: Action = serde_json::from_value(step["a"].clone())
            .unwrap_or_else(|error| panic!("{seed} step {n}: `a` is not an Action ({error}): {action_json}"));

        // `l` for the actor, on the state the action is chosen in.
        let text = legal_text(&state, action.player_id);
        let hash = fnv1a32_utf16(&text);
        let expected = recorded(step, "l");
        if hash != expected {
            return Err(Box::new(Mismatch {
                seed: seed.clone(),
                step: label.clone(),
                which: "l",
                expected,
                actual: hash,
                action: Some(action_json),
                diff: Some(write_diff(&seed, &label, "l", &text)),
            }));
        }

        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            return Err(Box::new(Mismatch {
                seed: seed.clone(),
                step: label.clone(),
                which: "refused",
                expected: "reduce to apply the recorded action".to_string(),
                actual: format!("a refusal: {error}"),
                action: Some(action_json),
                diff: Some(write_diff(&seed, &label, "refused", &hashed_state_text(&state))),
            }));
        }
        state = result.state;
        check_snapshot(&seed, &label, Some(&action_json), step, &state, &result.events)?;
    }

    let ending = json!({
        "winner": state.result.as_ref().map(|result| result.winner),
        "reason": state.result.as_ref().map(|result| result.reason),
        "steps": steps.len(),
    });
    if ending != game["end"] {
        return Err(Box::new(Mismatch {
            seed,
            step: "end".to_string(),
            which: "end",
            expected: game["end"].to_string(),
            actual: ending.to_string(),
            action: None,
            diff: None,
        }));
    }
    Ok(steps.len())
}

/// The lines of `games.jsonl`, blank ones (a trailing newline) left out.
fn golden_lines() -> Vec<&'static str> {
    GAMES.lines().filter(|line| !line.trim().is_empty()).collect()
}

/// Replays every game of one shard and fails with every divergent game's first mismatch.
fn replay_shard(shard: usize) {
    register_all();
    let mut failures: Vec<Box<Mismatch>> = Vec::new();
    let mut games = 0usize;
    let mut steps = 0usize;
    for (index, line) in golden_lines().into_iter().enumerate() {
        if index % GOLDEN_SHARDS != shard {
            continue;
        }
        games += 1;
        match replay_line(index, line) {
            Ok(n) => steps += n,
            Err(mismatch) => failures.push(mismatch),
        }
    }
    assert!(
        failures.is_empty(),
        "golden shard {shard}: {} of {games} games diverged from their TypeScript trace \
         ({steps} steps of the others replayed identically):\n{}",
        failures.len(),
        failures
            .iter()
            .map(|mismatch| mismatch.report())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

mod golden_traces {
    use super::*;

    #[test]
    fn the_file_holds_every_recorded_game_once() {
        let lines = golden_lines();
        assert_eq!(
            lines.len(),
            GOLDEN_GAMES,
            "SURFACE §13.1: 200 games and 40 handicapped"
        );
        let mut seen = BTreeSet::new();
        for (index, line) in lines.iter().enumerate() {
            let game: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("games.jsonl line {}: not JSON: {error}", index + 1));
            let seed = game["seed"].as_str().unwrap_or_default().to_string();
            assert_eq!(
                game["args"]["seed"].as_str(),
                Some(seed.as_str()),
                "line {}: args.seed",
                index + 1
            );
            assert!(seen.insert(seed.clone()), "{seed} is recorded twice");
        }
    }
}

/// One test per shard of the games, numbered 0.. in order; `GOLDEN_SHARDS` is how many there are.
macro_rules! golden_shards {
    ($($name:ident = $shard:expr),* $(,)?) => {
        /// How many tests the games are split over: game `i` is replayed by shard `i % GOLDEN_SHARDS`,
        /// and the test harness runs the shards on as many threads.
        const GOLDEN_SHARDS: usize = [$($shard),*].len();

        mod replays_the_golden_traces {
            use super::*;
            $(
                #[test]
                fn $name() {
                    replay_shard($shard);
                }
            )*
        }
    };
}

golden_shards!(
    shard_00 = 0,
    shard_01 = 1,
    shard_02 = 2,
    shard_03 = 3,
    shard_04 = 4,
    shard_05 = 5,
    shard_06 = 6,
    shard_07 = 7,
    shard_08 = 8,
    shard_09 = 9,
    shard_10 = 10,
    shard_11 = 11,
);

// The recorded hotseat game: spec 01 asserts the browser's own hash equals a fold of this log, and
// this file folds it with the real `reduce`, catalog and card scripts. It lives in the engine's tests,
// which take `jackioh-cards` as a dev-dependency (§2), because a fold without the scripts registered
// fizzles every Cry. The log is committed (`e2e/artifacts/` is gitignored, CI never runs Cypress), so
// the fold always runs; the last test compares it with a local Cypress recording when there is one.
// `EXPECTED_HASH` is a literal: a fold compared with a fold would move together and prove nothing.

/// The recording, as `e2e/support/tasks/replay.ts` writes it: `{ seed, decks, log }`.
#[derive(Deserialize)]
struct Recording {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
}

/// The committed recording, compiled in (§3: data reaches a pure crate's tests with `include_str!`).
const COMMITTED: &str = include_str!("golden/01-hotseat-full-game.json");
const COMMITTED_PATH: &str = "crates/engine/tests/golden/01-hotseat-full-game.json";

/// Where Cypress leaves its copy. Gitignored, so it is present only after a local `pnpm test:e2e`.
const RECORDED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../e2e/artifacts/01-hotseat-full-game.json"
);

/// The final state hash of the recorded game.
///
/// Refresh it ONLY together with the recording, and only when the rules deliberately changed: run
/// `pnpm test:e2e` (spec 01 re-records `e2e/artifacts/01-hotseat-full-game.json` and checks the
/// browser against a fold of it), copy that file over the fixture here, and paste the hash this test
/// reports. Editing the number on its own turns the check into a rubber stamp.
///
/// The exception is a change to what the state RECORDS about the same game, which leaves the log as it
/// is and moves the hash alone: R42, R174, R177, R213, R223, R265, R311, R419, R426, R448, R451 and
/// R457 (§6.2; B5 E4). A change to the game itself re-records the log (R244, R275, R276, R360–R366).
/// Practice saves on players' devices and e2e specs 01, 13 and 22 rely on the hash holding (§5.2).
const EXPECTED_HASH: &str = "a798906b";

/// What the recorded game ends in — a second anchor, so the hash is not the only witness.
const EXPECTED_RESULT: GameResult = GameResult {
    winner: Winner::P1,
    reason: GameOverReason::HeroDeath,
};
const EXPECTED_ACTIONS: usize = 37;

fn read(path: &str, text: &str) -> Recording {
    let parsed: Value =
        serde_json::from_str(text).unwrap_or_else(|error| panic!("{path} is not JSON: {error}"));
    if !parsed["seed"].is_string() || !parsed["decks"].is_array() || !parsed["log"].is_array() {
        panic!("{path} is not a recorded hotseat game: expected {{ seed, decks, log }}");
    }
    serde_json::from_value(parsed)
        .unwrap_or_else(|error| panic!("{path} is not a recorded hotseat game: {error}"))
}

fn recording() -> Recording {
    read(COMMITTED_PATH, COMMITTED)
}

/// `fold({ seed, decks, log })`, the three fields the recording carries and nothing else.
fn fold_recording(seed: &str, decks: &(Vec<String>, Vec<String>), log: &[Action]) -> FoldResult {
    let input: FoldArgs = serde_json::from_value(json!({ "seed": seed, "decks": decks, "log": log }))
        .expect("{ seed, decks, log } is a fold's input");
    fold(&input)
}

mod the_recorded_hotseat_game_replays_in_vitest_build_m5_t3 {
    use super::*;

    #[test]
    fn folds_the_browsers_own_seed_decks_log_to_the_recorded_final_state_hash() {
        // The catalog and the card scripts, exactly as the web client registers them before the
        // browser's first `reduce`. Without them the Crys fizzle and the hash moves.
        register_all();
        let recording = recording();

        let replayed = fold_recording(&recording.seed, &recording.decks, &recording.log);

        // Every recorded action is legal against a fold from scratch. A rejection would mean the log
        // and the engine have parted company, and the hash below would be a hash of a shorter game.
        let refusals: Vec<String> = replayed
            .errors
            .iter()
            .map(|error| format!("{}: {}", error.nonce, error.error))
            .collect();
        assert_eq!(refusals, Vec::<String>::new());
        assert_eq!(hash_state(&replayed.state), EXPECTED_HASH);

        // Anchors that say what that hash IS, so a future diff reads as a rules change and not as an
        // unexplained number: the game is over, p1 won by hero death, and p2's hero is dead.
        assert_eq!(replayed.state.result, Some(EXPECTED_RESULT));
        assert_eq!(replayed.state.phase, Phase::Over);
        assert!(replayed.state.players.p2.hero.health <= 0);
        assert!(replayed.state.players.p1.hero.health > 0);
    }

    #[test]
    fn is_a_fold_of_the_whole_log_one_action_fewer_is_a_different_hash() {
        // The point of this one is that the assertion above cannot pass by accident. If `fold` ignored
        // the log, or the hash ignored the state, these two would agree.
        register_all();
        let recording = recording();
        let short = fold_recording(
            &recording.seed,
            &recording.decks,
            &recording.log[..recording.log.len() - 1],
        );

        assert_ne!(hash_state(&short.state), EXPECTED_HASH);
        assert_eq!(short.state.result, None);
    }

    #[test]
    fn is_a_fold_of_that_seed_another_seed_is_a_different_hash() {
        register_all();
        let recording = recording();
        let other = fold_recording(
            &format!("{}-not", recording.seed),
            &recording.decks,
            &recording.log,
        );
        assert_ne!(hash_state(&other.state), EXPECTED_HASH);
    }

    #[test]
    fn is_deterministic_folding_twice_in_a_row_gives_the_same_hash() {
        register_all();
        let recording = recording();
        let first = fold_recording(&recording.seed, &recording.decks, &recording.log);
        let second = fold_recording(&recording.seed, &recording.decks, &recording.log);
        assert_eq!(hash_state(&second.state), hash_state(&first.state));
    }

    #[test]
    fn carries_a_real_recording_51_stamped_actions_over_two_deck_legal_libraries() {
        let recording = recording();
        assert_eq!(recording.seed, "01-hotseat");
        assert_eq!(recording.log.len(), EXPECTED_ACTIONS);
        // The typed log cannot lack a nonce or a seat, so read the recording's own JSON for them.
        let raw: Value = serde_json::from_str(COMMITTED).expect("the recording is JSON");
        for action in raw["log"].as_array().expect("the recording has a log") {
            assert!(
                action["nonce"].is_string(),
                "every action carries a nonce: {action}"
            );
            assert!(
                ["p1", "p2"].contains(&action["playerId"].as_str().unwrap_or_default()),
                "every action names its seat: {action}"
            );
        }
        for deck in [&recording.decks.0, &recording.decks.1] {
            assert_eq!(deck.len(), 20);
            assert_eq!(deck.iter().collect::<BTreeSet<_>>().len(), 20);
            for id in deck {
                assert!(CATALOG.contains_key(id), "{id} is a catalog card");
            }
        }
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // reads Cypress's local recording when there is one, as TS's test did; never the rules.
    fn matches_the_last_browser_recording_when_a_local_cypress_run_has_left_one() {
        let recorded = match std::fs::read_to_string(RECORDED) {
            Ok(text) => text,
            // `e2e/artifacts/` is gitignored: on CI and on a checkout where Cypress has not run, there
            // is nothing to compare. The fold above has already run against the committed copy, so
            // nothing is being skipped here except a comparison with a file that does not exist.
            Err(_) => return,
        };

        let recorded: Value = serde_json::from_str(&recorded).expect("Cypress's recording is JSON");
        let committed: Value = serde_json::from_str(COMMITTED).expect("the committed recording is JSON");
        assert_eq!(
            recorded, committed,
            "e2e/artifacts/01-hotseat-full-game.json no longer matches \
             crates/engine/tests/golden/01-hotseat-full-game.json. Spec 01 recorded a different game, \
             so the fixture here is stale: copy the artifact over it and update EXPECTED_HASH with the \
             value this file reports."
        );
    }
}
