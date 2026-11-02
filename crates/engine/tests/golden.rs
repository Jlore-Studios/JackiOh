//! The two committed logs, folded with the real cards: the oracle the engine is held to (README V4).
//!
//! The recorded hotseat game must fold to `golden::HOTSEAT_HASH`, which `cargo jackioh golden check`
//! checks too (#1641); a log recorded before the Equip rename still folds (R781000); and neither game
//! moves whatever the card metrics' lists hold (R1846003).
//!
//! A change is held to its base build by `golden record` on the head and `golden check --file` on the
//! base build (`luau diff`, `classify --parent-bin`), with nothing committed (#2578).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::testkit::golden::{HOTSEAT, HOTSEAT_HASH, check_hotseat};
use jackioh_engine::{
    Action, EARLIER_DECK_SIZE, FoldArgs, FoldResult, GameOverReason, GameResult, Phase, Winner, fold,
    hash_state,
};

// The recorded hotseat game: spec 01 asserts the browser's own hash equals a fold of this log, and this
// file folds it with the real `reduce`, catalog and card scripts (`jackioh-cards` is a dev-dependency, §2:
// a fold without the scripts registered fizzles every Cry). The log is committed (`e2e/artifacts/` is
// gitignored), so the fold always runs. `HOTSEAT_HASH` is a literal: a fold compared with a fold would
// move together and prove nothing.

/// The recording, as `e2e/support/tasks/replay.ts` writes it: `{ seed, decks, log }`.
#[derive(Deserialize)]
struct Recording {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
}

/// The committed recording, compiled into the testkit (§3: data reaches a pure crate by `include_str!`).
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
const EXPECTED_ACTIONS: usize = 39;

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
        // The catalog and card scripts, as the web client registers them before its first `reduce`.
        register_all();
        let recording = recording();

        // Every recorded action is legal against a fold from scratch, and the fold hashes to
        // `HOTSEAT_HASH` (`golden check`'s own check); a rejection would make it a shorter game's hash.
        assert_eq!(check_hotseat(), Ok(HOTSEAT_HASH.to_string()));

        let replayed = fold_recording(&recording.seed, &recording.decks, &recording.log);

        // Anchors saying what that hash IS, so a future diff reads as a rules change, not a bare number.
        assert_eq!(replayed.state.result, Some(EXPECTED_RESULT));
        assert_eq!(replayed.state.phase, Phase::Over);
        assert!(replayed.state.players.p2.hero.health <= 0);
        assert!(replayed.state.players.p1.hero.health > 0);
    }

    #[test]
    fn is_a_fold_of_the_whole_log_one_action_fewer_is_a_different_hash() {
        // If `fold` ignored the log, or the hash ignored the state, these two would agree.
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
        // Recorded before #1705, so each deck held EARLIER_DECK_SIZE cards and folds as made (R1705003).
        let size = EARLIER_DECK_SIZE as usize;
        for deck in [&recording.decks.0, &recording.decks.1] {
            assert_eq!(deck.len(), size);
            assert_eq!(deck.iter().collect::<BTreeSet<_>>().len(), size);
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
            // `e2e/artifacts/` is gitignored: with no Cypress run there is nothing to compare, and the
            // fold above has already run against the committed copy.
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
// (golden seed 43's log, R1156000, its Sentient Cat Ears played onto a Unit at step 7), and it reads as
// `equip`: the old log and the same log renamed fold to one state. Since Player 1 opens on three cards
// (R2368000) the log holds its first nine actions, and under #2521's catalog p2 sends its whole hand back.
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
            .map(|action| serde_json::to_value(action).expect("an action serializes"));
        let played = played.expect("step 7 is in the log");
        assert_eq!(
            played["equip"],
            json!(true),
            "it is written back under the new name"
        );
        assert!(played.get("magnetic").is_none(), "and never under the old one");
    }
}

/// R1846003, R2173000: the card metrics stay the build's under a fold, and a game that plays neither reader
/// (Stinkyscape #109, #111, the only cards that ask for a BUSTED or complicated pool, R1846001) folds to one
/// hash whatever the lists hold: every card in both, or none, on this thread's override of the compiled
/// metrics. The two committed logs deal no reader and make none.
mod r1846003_the_card_metrics_reach_only_the_games_that_play_a_reader {
    use super::*;
    use jackioh_engine::testkit::golden::hashed_state_text;
    use jackioh_engine::testkit::{clear_overrides, register_code_lines};

    #[test]
    fn r1846003_the_readers_are_derived_from_the_scripts() {
        assert_eq!(
            jackioh_cards::LIST_READERS,
            ["stinkyscape-109", "stinkyscape-111"],
            "S #109 xxFreshJack and #111 Shockwave447 read the lists (R2173001)"
        );
    }

    #[test]
    fn r1846003_a_log_without_a_reader_folds_alike_with_every_card_or_none_busted_and_complicated() {
        register_all();
        let every: Vec<(&str, usize)> = CATALOG.keys().map(|id| (id.as_str(), usize::MAX)).collect();
        for (name, text) in [(COMMITTED_PATH, COMMITTED), ("pre-equip-log.json", PRE_EQUIP_LOG)] {
            let input: FoldArgs = serde_json::from_str(text).expect("the log is a fold's input");
            let folded = fold(&input).state;
            let end = hashed_state_text(&folded);
            for reader in jackioh_cards::LIST_READERS.iter() {
                let quoted = format!("{reader:?}");
                assert!(!text.contains(&quoted), "{name} deals {reader}");
                assert!(!end.contains(&quoted), "{name} makes {reader}");
            }
            let build = hash_state(&folded);
            for lines in [every.clone(), Vec::new()] {
                register_code_lines(lines);
                let hash = hash_state(&fold(&input).state);
                clear_overrides();
                assert_eq!(hash, build, "{name}: the lists moved a game that plays no reader");
            }
        }
    }
}
