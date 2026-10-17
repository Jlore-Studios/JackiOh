//! Small helpers shared by the engine tests: a registered fixture catalog, a sink and placement.
//!
//! The registries are the testkit's thread-local override, so `setup_catalog` registers for the
//! calling test's thread only. Surface contract: docs/v0.3.0/SURFACE.md §8.
//! `put` hands back an owned copy of the placed card; a test that changes it writes through
//! `find_instance_mut`.

use jackioh_engine::testkit::*;

use super::catalog::{vanilla_catalog, vanilla_deck};
use super::combat::{COMBAT_SCRIPTS, combat_catalog};
use super::scripts::{FIXTURE_SCRIPTS, fixture_catalog};

/// The fixture catalog every engine test starts from: the vanilla units, the Rush token and the
/// scripted and combat fixtures, with their scripts.
pub fn setup_catalog() {
    register_catalog(combat_catalog(fixture_catalog(vanilla_catalog(40, 1))));
    let mut merged: IndexMap<String, CardScripts> = FIXTURE_SCRIPTS.clone();
    merged.extend(COMBAT_SCRIPTS.clone());
    register_scripts(merged);
}

/// A game in setup; `None` deals the vanilla decks `fx-1`…`fx-20` and `fx-21`…`fx-40`.
pub fn new_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    setup_catalog();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))),
        ..CreateGameOptions::default()
    })
}

/// A sink whose rng starts at the state's cursor, as reduce does. Its event list starts empty, and
/// it and the rng are leaked, so the sink borrows only the state.
pub fn sink_for(state: &mut GameState) -> EngineSink<'_> {
    let rng: &mut Rng = Box::leak(Box::new(Rng::new(&state.seed, state.rng_cursor)));
    let events: &mut Vec<GameEvent> = Box::leak(Box::default());
    EngineSink::new(state, events, rng)
}

pub fn slot(player: PlayerId, row: Row, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row, lane }
}

/// A new instance of `def_id`, owned by the slot's player, placed straight into the slot.
/// `options` is `json!({})` or `json!({ "radiant": true })`.
pub fn put(state: &mut GameState, def_id: &str, at: ZoneSlot, options: Value) -> CardInstance {
    let player = at.player;
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if options.get("radiant").and_then(Value::as_bool) == Some(true) {
        card.radiant = true;
    }
    // R1081000: a Flying Unit put in a backrow zone stands there, as one played into it does.
    let stands = at.row == Row::Backrow && unit_has(state, &card, KeywordKind::Flying);
    let options = PlaceOnFieldOptions::default();
    let placed = if stands {
        place_standing_on_field(state, &mut card, at, options)
    } else {
        place_on_field(state, &mut card, at, options)
    };
    if !placed {
        panic!("could not place {def_id} in {} {}", at.row, at.lane);
    }
    find_instance(state, &card.id).cloned().unwrap_or(card)
}

pub fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId, count: i32) -> Vec<CardInstance> {
    (0..count)
        .map(|_| {
            let card = new_instance(state, def_id, player, Zone::Hand { player });
            state.players[player].hand.push(card.clone());
            card
        })
        .collect()
}

/// The player's library, top first, as their own deck: each card known to its owner (R311) and
/// dealt, so not Created (R943), as the testkit scenario's setup deals its piles.
pub fn set_library(
    state: &mut GameState,
    player: PlayerId,
    def_ids: &[impl AsRef<str>],
) -> Vec<CardInstance> {
    let mut cards: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| new_dealt_instance(state, def_id.as_ref(), player, Zone::Library { player }))
        .collect();
    for card in cards.iter_mut() {
        show_to_owner(card);
    }
    state.players[player].library = cards.clone();
    cards
}

/// The events of one type, in order; `type_` is a `GameEventType` or its string (`"damage"`).
pub fn events_of_type(events: &[GameEvent], type_: impl ToString) -> Vec<GameEvent> {
    let wanted = type_.to_string();
    events
        .iter()
        .filter(|event| event.event_type().as_str() == wanted)
        .cloned()
        .collect()
}

#[derive(Clone, Debug)]
pub struct RandomGame {
    pub state: GameState,
    pub log: Vec<Action>,
    pub decks: (Vec<String>, Vec<String>),
}

/// A full game played by the random policy of §10.7: uniform over the legal actions, ending the
/// turn when it is the only option or with AI_END_TURN_PROBABILITY. Concedes and draw offers are
/// left out, so a game ends by hero death or the turn cap (M1 gate).
pub fn play_random_game(seed: &str, deck_pair: Option<(Vec<String>, Vec<String>)>) -> RandomGame {
    let decks = deck_pair.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)));
    let mut state = begin_game(&new_game(seed, Some(decks.clone()))).state;
    let mut policy = Rng::new(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();

    let mut step = 0;
    while state.result.is_none() {
        if step > 4000 {
            panic!("game {seed} did not finish");
        }
        // The last fallback for the seat to act is the active player.
        let player = seat_to_act(&state).unwrap_or(state.active);
        let actions: Vec<ActionBody> = legal_actions(&state, player)
            .into_iter()
            .filter(|action| {
                !matches!(
                    action.action_type(),
                    ActionType::Concede | ActionType::OfferDraw | ActionType::AnswerDraw
                )
            })
            .collect();
        if actions.is_empty() {
            panic!("no legal action for {player} in game {seed}");
        }

        let others: Vec<&ActionBody> = actions
            .iter()
            .filter(|action| action.action_type() != ActionType::EndTurn)
            .collect();
        let end_turn = actions
            .iter()
            .find(|action| action.action_type() == ActionType::EndTurn);
        // The chance is drawn only when something besides ending the turn exists and ending it is offered.
        let take_end = others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY));
        let chosen: ActionBody = if take_end {
            match end_turn {
                Some(end) => end.clone(),
                None => others[policy.int(others.len() as i32) as usize].clone(),
            }
        } else {
            others[policy.int(others.len() as i32) as usize].clone()
        };

        let action = Action::new(chosen, player, format!("a{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        log.push(action);
        state = result.state;
        step += 1;
    }

    RandomGame { state, log, decks }
}

/// A value as JSON, the shape the tests compare (the TS tests' `JSON.parse(JSON.stringify(…))`).
pub fn json_of(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("serialises")
}

/// A value through JSON and back, as a paused state crosses a socket or a store (§10.1).
pub fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(json_of(value)).expect("parses back")
}

/// TS `toMatchObject`: every key the expected object names matches, recursively; an array matches
/// element for element and in length; anything else is equal.
pub fn matches_object(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}
