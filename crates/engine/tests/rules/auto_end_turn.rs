//! R345: R82's automatic turn end is each player's to turn off (SPEC §2.5, §10.2, §10.8).
//!
//! Port of `packages/engine/test/auto-end-turn.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{spell_def, vanilla_deck};
use crate::rules::fixtures::harness::{events_of_type, new_game};

const SEED: &str = "r345";

fn decks() -> (Vec<String>, Vec<String>) {
    (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))
}

/// TS's module `let counter`: every action this file builds takes the next nonce.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn action(body: ActionInput) -> Action {
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    body.with_nonce(format!("r345-{counter}"))
}

fn act(state: &GameState, body: ActionInput) -> ReduceResult {
    let result = reduce(state, &action(body));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn mulligans(state: &GameState) -> Vec<Action> {
    [P1, P2]
        .into_iter()
        .map(|player| {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            action(input(json!({ "type": "mulligan", "keep": keep, "playerId": player })))
        })
        .collect()
}

/// Past both mulligans, in p1's first main phase.
fn started() -> GameState {
    let mut state = begin_game(&new_game(SEED, Some(decks()))).state;
    for answer in mulligans(&state) {
        state = reduce(&state, &answer).state;
    }
    assert_eq!(state.phase, Phase::Main);
    assert_eq!(state.active, P1);
    assert!(state.pending.is_none());
    state
}

/// p1's first turn with nothing left in it but ending it, conceding and offering a draw (R82).
fn idle_turn() -> GameState {
    let mut state = started();
    state.players.p1.hand = Vec::new();
    let left: Vec<ActionType> = legal_actions(&state, P1).iter().map(|body| body.action_type()).collect();
    assert!(
        left.iter()
            .all(|kind| [ActionType::EndTurn, ActionType::Concede, ActionType::OfferDraw].contains(kind))
    );
    state
}

mod r345_the_automatic_turn_end_is_each_players_to_turn_off {
    use super::*;

    #[test]
    fn r345_turned_off_a_turn_with_nothing_left_waits_for_end_turn_turned_on_again_it_ends_at_once() {
        let off = act(&idle_turn(), input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" })));
        assert!(off.events.is_empty());
        assert_eq!(off.state.active, P1);
        assert_eq!(off.state.players.p1.auto_end_turn, Some(false));

        let ended = act(&off.state, input(json!({ "type": "endTurn", "playerId": "p1" })));
        assert_eq!(events_of_type(&ended.events, GameEventType::TurnAutoEnded).len(), 0);
        assert_eq!(ended.state.active, P2);

        let on = act(&off.state, input(json!({ "type": "setAutoEndTurn", "enabled": true, "playerId": "p1" })));
        assert_eq!(
            json_of(events_of_type(&on.events, GameEventType::TurnAutoEnded)),
            json!([{ "type": "turnAutoEnded", "player": "p1", "turn": 1 }])
        );
        assert_eq!(on.state.active, P2);
        assert!(json_of(&on.state.players.p1).get("autoEndTurn").is_none());
    }

    #[test]
    fn r345_the_default_is_r82s_and_the_preference_is_the_senders_own() {
        // p2 turning it off for p2 leaves p1's idle turn to end by itself, as R82 says.
        let result = act(&idle_turn(), input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" })));
        assert_eq!(
            json_of(events_of_type(&result.events, GameEventType::TurnAutoEnded)),
            json!([{ "type": "turnAutoEnded", "player": "p1", "turn": 1 }])
        );
        assert_eq!(result.state.players.p2.auto_end_turn, Some(false));
        assert!(json_of(&result.state.players.p1).get("autoEndTurn").is_none());
    }

    #[test]
    fn r345_either_seat_may_send_it_while_the_mulligans_are_open_and_on_the_other_seats_turn() {
        let setup = begin_game(&new_game(SEED, Some(decks()))).state;
        assert!(setup.mulligan.is_some());
        for player in [P1, P2] {
            let result = act(&setup, input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": player })));
            assert!(result.events.is_empty());
            assert_eq!(result.state.players[player].auto_end_turn, Some(false));
        }
        let on_their_turn = act(&started(), input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" })));
        assert_eq!(on_their_turn.state.players.p2.auto_end_turn, Some(false));
    }

    #[test]
    fn r345_legal_actions_never_offers_it_so_no_policy_ever_sends_it() {
        for state in [begin_game(&new_game(SEED, Some(decks()))).state, started(), idle_turn()] {
            for player in [P1, P2] {
                assert!(
                    !legal_actions(&state, player)
                        .iter()
                        .any(|body| body.action_type() == ActionType::SetAutoEndTurn)
                );
            }
        }
    }

    #[test]
    fn r345_a_view_carries_the_viewers_own_preference_only_and_nothing_while_it_is_on() {
        let off = act(&idle_turn(), input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }))).state;
        assert_eq!(view_for(&off, P1).auto_end_turn, Some(false));
        assert!(json_of(view_for(&off, P2)).get("autoEndTurn").is_none());
        assert!(json_of(view_for(&started(), P1)).get("autoEndTurn").is_none());
    }

    #[test]
    fn r345_a_log_that_turns_it_off_and_on_again_folds_to_the_hash_of_one_that_never_touched_it() {
        let setup = begin_game(&new_game(SEED, Some(decks()))).state;
        let plain = mulligans(&setup);
        let mut toggled = vec![
            action(input(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }))),
            action(input(json!({ "type": "setAutoEndTurn", "enabled": true, "playerId": "p1" }))),
        ];
        toggled.extend(mulligans(&setup));
        let a = fold(&FoldArgs {
            seed: SEED.to_string(),
            decks: decks(),
            log: plain,
            ..Default::default()
        });
        let b = fold(&FoldArgs {
            seed: SEED.to_string(),
            decks: decks(),
            log: toggled.clone(),
            ..Default::default()
        });
        assert!(a.errors.is_empty());
        assert!(b.errors.is_empty());
        assert_eq!(hash_state(&b.state), hash_state(&a.state));

        // Left off, it is part of the state, so the fold says so.
        let mut left_off_log = vec![toggled[0].clone()];
        left_off_log.extend(mulligans(&setup));
        let left_off = fold(&FoldArgs {
            seed: SEED.to_string(),
            decks: decks(),
            log: left_off_log,
            ..Default::default()
        });
        assert_eq!(left_off.state.players.p1.auto_end_turn, Some(false));
        assert_ne!(hash_state(&left_off.state), hash_state(&a.state));
    }
}

// #188: `autoEndDue` (reduce.ts) needs to know only whether some action holds the turn open, so it
// walks `eachLegalAction` and stops at the first one. A card past that point is never priced: in a
// long game that walk ran after every simulated action and was nearly a third of the AI's time.
fn priced_def() -> CardDef {
    spell_def(0, json!({ "id": "fx-priced", "name": "Fixture Priced Spell" }))
}

/// TS's module `let priced`: how many times the fixture's cost hook has run.
static PRICED: AtomicU32 = AtomicU32::new(0);

fn priced_script() -> Script {
    Script {
        cost: Some(cost_hook(|_| {
            PRICED.fetch_add(1, Ordering::SeqCst);
            1
        })),
        ..Default::default()
    }
}

mod r82_the_automatic_turn_end_looks_no_further_than_the_first_action_that_holds_the_turn_open {
    use super::*;

    #[test]
    fn r82_a_hand_card_after_a_playable_one_is_never_priced_to_decide_that_the_turn_goes_on() {
        let mut state = started();
        let def = priced_def();
        let mut catalog = registered_catalog().clone();
        catalog.insert(def.id.clone(), def.clone());
        register_catalog(catalog);
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            def.id.clone(),
            CardScripts {
                base: priced_script(),
                radiant: priced_script(),
            },
        );
        register_scripts(scripts);
        let playable = state.players.p1.hand.first().cloned().expect("p1 kept no hand");
        let later = new_instance(&mut state, &def.id, P1, Zone::Hand { player: P1 });
        state.players.p1.hand = vec![playable, later.clone()];

        PRICED.store(0, Ordering::SeqCst);
        let result = act(&state, input(json!({ "type": "setAutoEndTurn", "enabled": true, "playerId": "p1" })));
        assert_eq!(events_of_type(&result.events, GameEventType::TurnAutoEnded).len(), 0);
        assert_eq!(result.state.active, P1);
        assert_eq!(PRICED.load(Ordering::SeqCst), 0);

        // The card is still on offer: the full list prices it and lists its play.
        let plays: Vec<ActionBody> = legal_actions(&result.state, P1)
            .into_iter()
            .filter(|body| matches!(body, ActionBody::Play { instance_id, .. } if *instance_id == later.id))
            .collect();
        assert!(!plays.is_empty());
        assert!(PRICED.load(Ordering::SeqCst) > 0);
    }
}
