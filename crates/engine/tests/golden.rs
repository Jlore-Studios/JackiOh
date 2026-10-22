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
//!
//! The replay is `jackioh_engine::testkit::golden::replay_game`, which `cargo jackioh golden check` runs too
//! (#1641); this file keeps the I/O, the sharding and the hotseat anchors.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::replay::fnv1a32_utf16;
use jackioh_engine::testkit::golden::{HOTSEAT, HOTSEAT_HASH, Mismatch, check_hotseat, replay_game};
use jackioh_engine::{
    Action, FoldArgs, FoldResult, GameOverReason, GameResult, Phase, Winner, fold, hash_state,
};

// The golden traces (§13)

/// §13.3: the recorded traces.
const GAMES: &str = include_str!("golden/games.jsonl");

/// §13.1: 200 games on this spec's resources and 40 with one seat handicapped.
const GOLDEN_GAMES: usize = 240;

/// §13.3: where a divergence leaves the Rust side's canonical text (the workspace's `target/`).
const DIFF_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/golden-diff");

/// §13.3: write the Rust side's canonical text next to the recorded side's, and say where it went.
#[allow(clippy::disallowed_methods)] // SURFACE §13.3: the diff file is this harness's output; clippy.toml's I/O ban guards the rules, which never reach this.
fn write_diff(mismatch: &mut Mismatch) {
    let Some(text) = &mismatch.text else {
        return;
    };
    let dir = PathBuf::from(DIFF_DIR);
    let path = dir.join(mismatch.diff_file());
    let written =
        match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, format!("{text}\n"))) {
            Ok(()) => path.display().to_string(),
            Err(error) => format!("(could not write {}: {error})", path.display()),
        };
    mismatch.diff = Some(written);
}

/// Replays one line of `games.jsonl` with `golden::replay_game` (§13.3) and writes the Rust side of its first
/// mismatch. `Ok` carries the number of steps replayed.
fn replay_line(index: usize, line: &str) -> Result<usize, Box<Mismatch>> {
    let game: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("games.jsonl line {}: not JSON: {error}", index + 1));
    let seed = game["seed"]
        .as_str()
        .unwrap_or_else(|| panic!("games.jsonl line {}: no `seed`", index + 1));
    replay_game(seed, &game).map_err(|mut mismatch| {
        write_diff(&mut mismatch);
        mismatch
    })
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

mod a_doctored_trace {
    use super::*;

    #[test]
    fn names_the_seed_the_step_the_hash_and_the_action_as_golden_check_does() {
        // crates/tools/tests/golden.rs doctors the same hash and holds `golden check` to the same words.
        register_all();
        let mut game: Value = serde_json::from_str(golden_lines()[0]).expect("line 1 is JSON");
        let recorded = game["steps"][0]["s"]
            .as_str()
            .expect("step 0 has an `s`")
            .to_string();
        let doctored = if recorded == "00000000" {
            "ffffffff"
        } else {
            "00000000"
        };
        game["steps"][0]["s"] = Value::from(doctored);
        let action = game["steps"][0]["a"].to_string();

        let mismatch = replay_game("jackioh-fuzz-1", &game).expect_err("a doctored hash diverges");

        assert_eq!(
            (mismatch.seed.as_str(), mismatch.step.as_str(), mismatch.which),
            ("jackioh-fuzz-1", "0", "s")
        );
        assert_eq!(
            (mismatch.expected.as_str(), mismatch.actual.as_str()),
            (doctored, recorded.as_str())
        );
        assert_eq!(mismatch.action.as_deref(), Some(action.as_str()));
        let text = mismatch.text.as_deref().expect("the Rust side's state text");
        assert_eq!(fnv1a32_utf16(text), recorded, "the text is the state `s` hashes");
        assert_eq!(mismatch.diff_file(), "jackioh-fuzz-1-0-s.json");
        let report = mismatch.report();
        assert!(
            report.starts_with(&format!(
                "seed jackioh-fuzz-1 step 0: `s` expected {doctored}, Rust has {recorded}\n    action: {action}\n"
            )),
            "{report}"
        );
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
// `HOTSEAT_HASH` is a literal: a fold compared with a fold would move together and prove nothing.

/// The recording, as `e2e/support/tasks/replay.ts` writes it: `{ seed, decks, log }`.
#[derive(Deserialize)]
struct Recording {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
}

/// The committed recording, compiled into the testkit (§3: data reaches a pure crate with `include_str!`).
const COMMITTED: &str = HOTSEAT;
const COMMITTED_PATH: &str = "crates/engine/tests/golden/01-hotseat-full-game.json";

/// Where Cypress leaves its copy. Gitignored, so it is present only after a local `pnpm test:e2e`.
const RECORDED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../e2e/artifacts/01-hotseat-full-game.json"
);

/// What the recorded game ends in — a second anchor, so the hash is not the only witness.
const EXPECTED_RESULT: GameResult = GameResult {
    winner: Winner::P1,
    reason: GameOverReason::HeroDeath,
};
const EXPECTED_ACTIONS: usize = 41;

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

        // Every recorded action is legal against a fold from scratch, and the fold hashes to
        // `golden::HOTSEAT_HASH`: `golden check`'s own check. A rejection would mean the log and the
        // engine have parted company, and the hash would be a hash of a shorter game.
        assert_eq!(check_hotseat(), Ok(HOTSEAT_HASH.to_string()));

        let replayed = fold_recording(&recording.seed, &recording.decks, &recording.log);

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

        assert_ne!(hash_state(&short.state), HOTSEAT_HASH);
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
        assert_ne!(hash_state(&other.state), HOTSEAT_HASH);
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
             so the fixture here is stale: copy the artifact over it and update HOTSEAT_HASH \
             (crates/engine/src/testkit/golden.rs) with the value this file reports."
        );
    }
}

// R781000: Magnetic is now Equip. A log recorded before the rename spells the play's flag `magnetic`
// (golden seed 43's log as re-blessed for R1156000, its Equip flag spelled `magnetic` as logs before the
// rename spell it, its Sentient Cat Ears played onto a Unit at step 7), and it reads as `equip`: the old
// log and the same log renamed fold to one state, the old-name play accepted.
const PRE_EQUIP_LOG: &str = include_str!("golden/pre-equip-log.json");

mod r781000_a_log_recorded_before_the_equip_rename_still_folds {
    use super::*;

    fn fold_text(text: &str) -> FoldResult {
        let input: FoldArgs = serde_json::from_str(text).expect("the log is a fold's input");
        fold(&input)
    }

    #[test]
    fn r781000_the_old_flag_reads_as_equip_and_folds_to_the_state_the_renamed_log_does() {
        register_all();
        assert!(
            PRE_EQUIP_LOG.contains("\"magnetic\":true"),
            "the fixture is the log as recorded"
        );
        let old = fold_text(PRE_EQUIP_LOG);
        let renamed = fold_text(&PRE_EQUIP_LOG.replace("\"magnetic\":true", "\"equip\":true"));

        assert!(
            old.errors.iter().all(|error| error.nonce != "golden-7"),
            "the Equip play recorded as `magnetic` is accepted: {:?}",
            old.errors
        );
        assert_eq!(hash_state(&old.state), hash_state(&renamed.state));
        assert_eq!(old.errors.len(), renamed.errors.len());
        let input: FoldArgs = serde_json::from_str(PRE_EQUIP_LOG).expect("the log is a fold's input");
        let played = input
            .log
            .iter()
            .find(|action| action.nonce == "golden-7")
            .map(|action| serde_json::to_value(action).expect("an action serialises"));
        let played = played.expect("step 7 is in the log");
        assert_eq!(
            played["equip"],
            json!(true),
            "it is written back under the new name"
        );
        assert!(played.get("magnetic").is_none(), "and never under the old one");
    }
}
