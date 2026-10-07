//! `cargo jackioh replay` (SPEC §9.2, §9.3; SURFACE §12): folds a recorded game from scratch and prints
//! its state hash. Replaces the TS fold that `e2e/support/tasks/replay-runner.ts` ran under tsx.
//!
//! Reads `{seed, decks, log, handicaps?, dealt?, lastBoards?, glitchBoards?}` as JSON on stdin and
//! prints one JSON line, `{"hash": hashState(folded state), "errors": [{nonce, error}, …]}`. The setup
//! fields are the fold's input exactly as the live `createGame` had them (R180 handicaps, R433 dealt
//! seats, R417 last boards, R678 Glitch boards); any other key (a browser's `state`, say) is ignored.
//! The card scripts are registered first: folding without them would diverge from the browser
//! instead of matching it, since every Cry in the log would fizzle differently (SPEC §10.9).
//!
//! A log may hold rejects, so a refused action is listed in `errors`, not fatal. The setup is not:
//! a deck its seat's handicap does not allow fails loudly instead of replaying a different game
//! (R180), and input that is not such an object exits 1.

use std::io::Read as _;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use jackioh_engine::replay::{FoldArgs, fold, hash_state};

/// `cargo jackioh replay`: no options; the game comes on stdin.
#[derive(clap::Args, Debug)]
pub struct Args {}

/// Folds one game's JSON (`FoldArgs`'s shape) and answers `{"hash", "errors"}`.
fn replay_json(input: Value) -> Result<Value> {
    let args: FoldArgs = serde_json::from_value(input).context("the input is not {seed, decks, log, …}")?;
    let folded = fold(&args);
    Ok(json!({
        "hash": hash_state(&folded.state),
        "errors": serde_json::to_value(&folded.errors)?,
    }))
}

/// `cargo jackioh replay`.
pub fn run(_args: Args) -> Result<()> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).context("reading stdin")?;
    let input: Value = serde_json::from_str(&text).context("stdin is not JSON")?;
    jackioh_cards::register_all();
    println!("{}", serde_json::to_string(&replay_json(input)?)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hotseat game e2e spec 01 recorded (SURFACE §13), committed beside the golden traces.
    const HOTSEAT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/golden/01-hotseat-full-game.json");
    /// Its pinned hash (`packages/cards/test/hotseat-replay.test.ts`).
    const HOTSEAT_HASH: &str = "a798906b";

    #[test]
    fn folds_the_hotseat_game_to_its_pinned_hash_with_no_errors() {
        jackioh_cards::register_all();
        let input: Value = serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap();
        let answer = replay_json(input).unwrap();
        assert_eq!(answer["hash"], HOTSEAT_HASH);
        assert_eq!(answer["errors"], json!([]));
    }

    #[test]
    fn a_refused_action_is_listed_with_its_nonce_and_changes_nothing() {
        jackioh_cards::register_all();
        let mut input: Value = serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap();
        // An attack by a card that does not exist, after the game's last action: refused, listed, and
        // the fold goes on to the same state.
        let refused = json!({
            "type": "attack", "attackerId": "no-such-card", "targetId": "hero-p2", "playerId": "p1", "nonce": "refused-1"
        });
        input["log"].as_array_mut().unwrap().push(refused);
        let answer = replay_json(input).unwrap();
        assert_eq!(answer["hash"], HOTSEAT_HASH);
        let errors = answer["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0]["nonce"], "refused-1");
        assert!(errors[0]["error"].as_str().is_some_and(|error| !error.is_empty()), "{errors:?}");
    }

    #[test]
    fn input_that_is_no_game_is_refused() {
        assert!(replay_json(json!({ "seed": "x" })).is_err());
        assert!(replay_json(json!([1, 2, 3])).is_err());
    }
}
