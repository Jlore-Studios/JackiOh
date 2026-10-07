//! Small helpers shared by the engine tests: a registered fixture catalog, a sink and placement.
//!
//! Port of `packages/engine/test/fixtures/harness.ts`. The registries are the testkit's thread-local
//! override (SURFACE §8), so `setup_catalog` registers for the calling test's thread only. A TS
//! argument with a default is passed explicitly (`new_game("engine-test", None)`), an optional one is
//! an `Option`, and an options object literal is JSON (`put(state, id, at, json!({ "radiant": true }))`).
//! A helper TS wrote to hand back the live card it placed hands back the card as it stands in the
//! state afterwards (an owned copy: a test that changes it writes through `find_instance_mut`).

use jackioh_engine::testkit::*;

use super::catalog::{vanilla_catalog, vanilla_deck};
use super::combat::{COMBAT_SCRIPTS, combat_catalog};
use super::scripts::{FIXTURE_SCRIPTS, fixture_catalog};

/// The fixture catalog every engine test starts from: the vanilla units and the Rush token, the
/// scripted fixtures of `./scripts.ts` and the combat fixtures, with both files' scripts.
pub fn setup_catalog() {
    register_catalog(combat_catalog(fixture_catalog(vanilla_catalog(40, 1))));
    let mut merged: IndexMap<String, CardScripts> = FIXTURE_SCRIPTS.clone();
    merged.extend(COMBAT_SCRIPTS.clone());
    register_scripts(merged);
}

/// A game in setup over the fixture catalog. TS `newGame(seed = "engine-test", decks?)`; `None` deals
/// the two vanilla decks, `fx-1`…`fx-20` and `fx-21`…`fx-40`.
pub fn new_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    setup_catalog();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))),
        ..CreateGameOptions::default()
    })
}

/// A sink whose rng starts at the state's cursor, as reduce does. TS `sinkFor(state)`: the sink's
/// event list starts empty, and it and the rng live as long as the test (both are leaked, so the
/// sink borrows only the state).
pub fn sink_for(state: &mut GameState) -> EngineSink<'_> {
    let rng: &mut Rng = Box::leak(Box::new(Rng::new(&state.seed, state.rng_cursor)));
    let events: &mut Vec<GameEvent> = Box::leak(Box::default());
    EngineSink::new(state, events, rng)
}

/// TS `sinkFor(state, events)`: the same, writing to the test's own event list.
pub fn sink_for_events<'a>(state: &'a mut GameState, events: &'a mut Vec<GameEvent>) -> EngineSink<'a> {
    let rng: &mut Rng = Box::leak(Box::new(Rng::new(&state.seed, state.rng_cursor)));
    EngineSink::new(state, events, rng)
}

pub fn slot(player: PlayerId, row: Row, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row, lane }
}

/// A new instance of `def_id`, owned by the slot's player, placed straight into the slot. TS
/// `put(state, defId, ref, options = {})`: `options` is `json!({})` or `json!({ "radiant": true })`.
pub fn put(state: &mut GameState, def_id: &str, at: ZoneSlot, options: Value) -> CardInstance {
    let player = at.player;
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if options.get("radiant").and_then(Value::as_bool) == Some(true) {
        card.radiant = true;
    }
    if !place_on_field(state, &mut card, &at, Default::default()) {
        panic!("could not place {def_id} in {} {}", at.row, at.lane);
    }
    find_instance(state, &card.id).cloned().unwrap_or(card)
}

/// `count` new instances of `def_id` pushed onto the player's hand. TS `inHand(state, defId, player,
/// count = 1)`.
pub fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId, count: i32) -> Vec<CardInstance> {
    (0..count)
        .map(|_| {
            let card = new_instance(state, def_id, player, Zone::Hand { player });
            state.players[player].hand.push(card.clone());
            card
        })
        .collect()
}

/// The player's library, top first, as their own deck: each card known to its owner (R311).
pub fn set_library(state: &mut GameState, player: PlayerId, def_ids: &[impl AsRef<str>]) -> Vec<CardInstance> {
    let mut cards: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| new_instance(state, def_id.as_ref(), player, Zone::Library { player }))
        .collect();
    for card in cards.iter_mut() {
        show_to_owner(card);
    }
    state.players[player].library = cards.clone();
    cards
}

/// The events of one type, in order. TS `eventsOfType(events, type)`: `type_` is a `GameEventType`
/// or its string (`"damage"`).
pub fn events_of_type(events: &[GameEvent], type_: impl ToString) -> Vec<GameEvent> {
    let wanted = type_.to_string();
    events
        .iter()
        .filter(|event| event.event_type().as_str() == wanted)
        .cloned()
        .collect()
}

/// What `play_random_game` returns (TS's anonymous `{ state, log, decks }`).
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
        // TS's seatToAct always names a seat; its last fallback is the active player.
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
        let end_turn = actions.iter().find(|action| action.action_type() == ActionType::EndTurn);
        // `others.length === 0 || (endTurn !== undefined && policy.chance(…))`: the chance is drawn
        // only when there is something besides ending the turn and ending it is offered.
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

/// Part 24's brief, step 2: the catalog `setup_catalog` registers.
pub fn catalog() -> CardDefs {
    combat_catalog(fixture_catalog(vanilla_catalog(40, 1)))
}

/// Part 24's brief, step 2: the scripts `setup_catalog` registers.
pub fn scripts() -> IndexMap<String, CardScripts> {
    let mut merged: IndexMap<String, CardScripts> = FIXTURE_SCRIPTS.clone();
    merged.extend(COMBAT_SCRIPTS.clone());
    merged
}
