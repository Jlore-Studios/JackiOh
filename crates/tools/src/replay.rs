//! `cargo jackioh replay` (SPEC §9.2, §9.3; SURFACE §12): folds a recorded game from scratch and prints
//! its state hash. Replaces the TS fold that `e2e/support/tasks/replay-runner.ts` ran under tsx.
//!
//! Reads `{seed, decks, log, handicaps?, dealt?, lastBoards?, glitchBoards?}` as JSON on stdin and
//! prints one JSON line, `{"hash": hashState(folded state), "errors": [{nonce, error}, …]}`. The setup
//! fields are the fold's input exactly as the live `createGame` had them (R180 handicaps, R433 dealt
//! seats, R417 last boards, R678 Glitch boards); any other key (a browser's `state`, say) is ignored.
//! The card scripts are registered first: folding without them would diverge from the browser
//! instead of matching it, since every Cry in the log would fizzle differently (SPEC §10.9). A game
//! with custom cards (`customCards`, R915006) folds with the card kit's host lent to the engine, as
//! the browser's Luau module folds it (R915009), so a practice game with a custom card replays here
//! to the browser's hash.
//!
//! A log may hold rejects, so a refused action is listed in `errors`, not fatal. The setup is not:
//! a deck its seat's handicap does not allow fails loudly instead of replaying a different game
//! (R180), and input that is not such an object exits 1.

use std::io::Read as _;
use std::rc::Rc;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use jackioh_engine::custom::with_custom_scripts;
use jackioh_engine::replay::{FoldArgs, fold, hash_state};
use jackioh_luau::custom::KitHost;

/// `cargo jackioh replay`: no options; the game comes on stdin.
#[derive(clap::Args, Debug)]
pub struct Args {}

/// Folds one game's JSON (`FoldArgs`'s shape) and answers `{"hash", "errors"}`.
fn replay_json(input: Value) -> Result<Value> {
    let args: FoldArgs = serde_json::from_value(input).context("the input is not {seed, decks, log, …}")?;
    let folded = with_custom_scripts(Rc::new(KitHost), || fold(&args));
    Ok(json!({
        "hash": hash_state(&folded.state),
        "errors": serde_json::to_value(&folded.errors)?,
    }))
}

/// `cargo jackioh replay`.
pub fn run(_args: Args) -> Result<()> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("reading stdin")?;
    let input: Value = serde_json::from_str(&text).context("stdin is not JSON")?;
    jackioh_cards::register_all();
    println!("{}", serde_json::to_string(&replay_json(input)?)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hotseat game e2e spec 01 recorded (SURFACE §13), committed beside the golden traces.
    const HOTSEAT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/golden/01-hotseat-full-game.json"
    );
    /// Its pinned hash (`packages/cards/test/hotseat-replay.test.ts`).
    const HOTSEAT_HASH: &str = "5c3d578c";

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
        assert!(
            errors[0]["error"].as_str().is_some_and(|error| !error.is_empty()),
            "{errors:?}"
        );
    }

    /// A 0-cost Spell that deals `{dmg}` (3) to the enemy hero, in the card kit.
    fn burner() -> Value {
        json!({
            "id": "custom-burner",
            "version": 1,
            "frame": {
                "name": "Burner", "type": "Spell", "cost": 0, "rarity": "Common",
                "text": "Deal {dmg} damage to the enemy hero.",
                "params": [{ "key": "dmg", "base": 3, "radiant": 6 }]
            },
            "source": "return Card {\n  cry = function(c, n)\n    c:damage(\"the enemy hero\", n.dmg)\n  end,\n}\n"
        })
    }

    /// practice/decks.ts' Human Vanguard: twenty distinct Core cards.
    const VANGUARD: [&str; 20] = [
        "core-004", "core-005", "core-008", "core-011", "core-015", "core-081", "core-001", "core-016",
        "core-020", "core-045", "core-061", "core-069", "core-077", "core-030", "core-013", "core-019",
        "core-053", "core-025", "core-054", "core-066",
    ];

    fn fold_of(input: &Value, host: bool) -> jackioh_engine::replay::ReplayResult {
        let args: FoldArgs = serde_json::from_value(input.clone()).unwrap();
        if host {
            with_custom_scripts(Rc::new(KitHost), || fold(&args))
        } else {
            fold(&args)
        }
    }

    fn view_of(state: &jackioh_engine::GameState, seat: jackioh_engine::PlayerId) -> Value {
        serde_json::to_value(jackioh_engine::view_for::view_for(state, seat)).unwrap()
    }

    /// R915006: a practice game whose deck holds a custom card replays here to the browser's state: the
    /// card's Cry runs through the card kit's host, and without the host it would do nothing.
    #[test]
    fn r915006_a_game_with_a_custom_card_folds_through_the_card_kit() {
        jackioh_cards::register_all();
        let mut human: Vec<String> = vec!["custom-burner".to_string()];
        human.extend(VANGUARD[..19].iter().map(|id| (*id).to_string()));
        let ai: Vec<String> = VANGUARD.iter().map(|id| (*id).to_string()).collect();
        // A seed whose opening hand holds the card: the deal is the seed's, so the search is fixed.
        let mut input = Value::Null;
        for n in 0..60 {
            let candidate = json!({
                "seed": format!("r915006-tools-{n}"), "decks": [human, ai], "log": [], "customCards": [burner()]
            });
            let state = fold_of(&candidate, true).state;
            let view = view_of(&state, jackioh_engine::PlayerId::P1);
            let holds = view["you"]["hand"]
                .as_array()
                .is_some_and(|hand| hand.iter().any(|card| card["defId"] == "custom-burner"));
            if holds {
                input = candidate;
                break;
            }
        }
        assert!(
            !input.is_null(),
            "no seed in 60 deals the custom card into the opening hand"
        );

        // Keep both hands, pass until the human's main phase, then play the card.
        let mut played_health = None;
        for step in 0..40 {
            let state = fold_of(&input, true).state;
            let Some(seat) = jackioh_engine::seat_to_act(&state) else {
                break;
            };
            let view = view_of(&state, seat);
            let pending = &view["pending"];
            let mut action = if pending["kind"] == "mulligan" {
                let keep: Vec<Value> = pending["options"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|option| option["key"].clone())
                    .collect();
                json!({ "type": "mulligan", "keep": keep })
            } else if seat == jackioh_engine::PlayerId::P1 && view["phase"] == "main" && pending.is_null() {
                let burner = view["you"]["hand"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|card| card["defId"] == "custom-burner")
                    .unwrap();
                played_health = Some(view["opponent"]["hero"]["health"].as_i64().unwrap());
                json!({ "type": "play", "instanceId": burner["instanceId"].clone() })
            } else {
                json!({ "type": "endTurn" })
            };
            action["playerId"] = serde_json::to_value(seat).unwrap();
            action["nonce"] = json!(format!("n{step}"));
            let legal = jackioh_engine::legal_actions(&state, seat);
            let body: jackioh_engine::ActionBody = serde_json::from_value(action.clone()).unwrap();
            assert!(
                action["type"] == "mulligan" || legal.contains(&body),
                "{action} is not legal"
            );
            input["log"].as_array_mut().unwrap().push(action);
            if played_health.is_some() {
                break;
            }
        }
        let before = played_health.expect("the human reaches a main phase with the card in hand");

        let answer = replay_json(input.clone()).unwrap();
        assert_eq!(answer["errors"], json!([]));
        let folded = fold_of(&input, true);
        assert_eq!(answer["hash"], hash_state(&folded.state));
        let after = view_of(&folded.state, jackioh_engine::PlayerId::P1)["opponent"]["hero"]["health"]
            .as_i64()
            .unwrap();
        assert_eq!(after, before - 3, "the Cry dealt its {{dmg}}");
        // Without the host the card has no script: the same log folds to another game.
        let bare = fold_of(&input, false);
        assert_ne!(hash_state(&bare.state), answer["hash"].as_str().unwrap());
    }

    #[test]
    fn input_that_is_no_game_is_refused() {
        assert!(replay_json(json!({ "seed": "x" })).is_err());
        assert!(replay_json(json!([1, 2, 3])).is_err());
    }
}
