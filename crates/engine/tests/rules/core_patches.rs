//! The engine half of patch v0.2.0's Core patches and cosmetics (docs/classic-sets.md B0, issue #40):
//!
//!   R429  §10.5 step 4 counts a card's plays on its instance when its script asks
//!         (`StaticFlags.countsPlays`, `timesPlayed`), which #31 KY's Math Equation reads; and, with
//!   R766  (issue #557), step 7 notes the price a Spell whose return keeps it was played at
//!         (`StaticFlags.returnKeepsPrice`), which #31's return gives back;
//!   R433  `createGame` learns the seats that were dealt a deck, whose library lists as unknown;
//!   R434  once the game is over, each view carries the other player's hand;
//!   R437  a delayed effect aimed at a card marks it in both views while it waits (`marks.ts`).
//!
//! R426 (#32 Prem Panther) rides `Script.afterAttack`, proved in after-attack.test.ts. Every behaviour
//! runs through fixture scripts (`fixtures/corePatches.ts`); the real cards prove it again in
//! packages/cards. Pauses are JSON round-tripped mid-way, and the games that can be folded
//! from `(seed, decks, log)` are folded and compared (§9.3).
//!
//! Port of `packages/engine/test/corePatches.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::damage;
use jackioh_engine::marks::marks_on;
use jackioh_engine::own_library::own_library_view;
use jackioh_engine::query::left_field_since_resolved;
use jackioh_engine::reduce::{begin_game, reduce};
use jackioh_engine::replay::{fold, hash_state};
use jackioh_engine::resolve::{CastOptions, HookOptions, cast_card, make_context};
use jackioh_engine::state_check::state_check;
use jackioh_engine::stays::exit_mark;
use jackioh_engine::testkit::*;
use jackioh_engine::times_played::times_played_of;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::zones::{MoveToZoneOptions, OffFieldZone, move_to_zone};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::combat::{big_body, plain};
use crate::rules::fixtures::core_patches::{
    CORE_PATCH_SCRIPTS, TEST_MARK, core_patch_catalog, counted, marker, plain_return, priced_return,
    quiet_trap, uncounted,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

/// TS's module-level `let nonce = 0`: every action this file sends gets a fresh nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
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

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. The events and
/// the rng are kept beside the state, so the test can read the state between calls as TS's shared
/// objects let it.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Sink {
    fn for_state(state: &GameState) -> Sink {
        Sink {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn register() {
    let mut catalog = registered_catalog().clone();
    catalog.extend(core_patch_catalog());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(CORE_PATCH_SCRIPTS.clone());
    register_scripts(scripts);
}

/// A main-phase board on p1's turn, both libraries the harness's vanilla decks.
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register();
    state.turn = 5;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// `reduce` with a fresh nonce; `body` is the TS `ActionInput` literal (its `playerId` included).
fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("core-patches-{nonce}"));
    let action: Action = json_as(action);
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn first_in_hand(state: &mut GameState, def_id: &str, player: PlayerId, what: &str) -> CardInstance {
    in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected {what}"))
}

fn instance_ids(events: &[GameEvent], ty: GameEventType) -> Vec<Value> {
    events_of_type(events, ty)
        .into_iter()
        .map(|event| json_of(event)["instanceId"].clone())
        .collect()
}

/// The events a view carries of one type, as JSON.
fn view_events_of(view: &Value, ty: &str) -> Vec<Value> {
    view["events"]
        .as_array()
        .map(|events| {
            events
                .iter()
                .filter(|event| event["type"] == json!(ty))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// The sum of a library list's entry counts (`list.cards.reduce((sum, entry) => sum + entry.count, 0)`).
fn listed(list: &Value) -> i64 {
    list["cards"]
        .as_array()
        .map(|cards| cards.iter().filter_map(|entry| entry["count"].as_i64()).sum())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// R429: the times a card has been played
// ---------------------------------------------------------------------------------------------

mod r429_10_5_step_4_counts_the_plays_of_a_card_that_asks {
    use super::*;

    #[test]
    fn r429_a_card_that_counts_its_plays_has_one_after_its_first_the_play_under_way_included_and_keeps_it_in_every_zone()
     {
        let mut state = board("r429-count");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let card = first_in_hand(&mut state, &counted.id, P1, "the counted Spell");
        state.players.p1.mana.current = 4;

        let played = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        )
        .state;
        let landed = played
            .players
            .p1
            .graveyard
            .iter()
            .find(|held| held.id == card.id)
            .expect("expected the Spell in the graveyard");
        assert_eq!(times_played_of(landed), 1);

        // R78's reset leaves it: moved back into the hand and played again, it counts 2.
        let mut again = clone_state(&played);
        let mut back = again.players.p1.graveyard.pop().expect("expected the Spell");
        back.zone = Zone::Hand { player: P1 };
        again.players.p1.hand.push(back.clone());
        let twice = act(
            &again,
            json!({ "type": "play", "instanceId": back.id, "playerId": "p1" }),
        )
        .state;
        assert_eq!(
            times_played_of(
                twice
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .find(|held| held.id == card.id)
                    .expect("expected the Spell")
            ),
            2
        );
    }

    #[test]
    fn r429_a_card_that_does_not_ask_carries_no_count_at_all_so_a_game_without_one_hashes_as_before() {
        let mut state = board("r429-none");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let card = first_in_hand(&mut state, &uncounted.id, P1, "the plain Spell");
        state.players.p1.mana.current = 4;

        let played = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        )
        .state;
        let landed = played
            .players
            .p1
            .graveyard
            .iter()
            .find(|held| held.id == card.id)
            .expect("expected the Spell");
        assert!(json_of(landed).get("timesPlayed").is_none());
    }

    #[test]
    fn r429_r70_a_cast_is_a_play_casting_the_card_counts_it() {
        let mut state = board("r429-cast");
        let card = new_instance(&mut state, &counted.id, P1, Zone::Resolving { player: P1 });
        state.players.p1.resolving.push(card.clone());
        let mut sink = Sink::for_state(&state);

        cast_card(&mut sink.on(&mut state), &card, CastOptions::default());

        // TS read the live instance the cast changed; Rust reads it back from the state by its id.
        assert_eq!(
            times_played_of(find_instance(&state, &card.id).expect("the cast card")),
            1
        );
        assert_eq!(
            instance_ids(&sink.events, GameEventType::CardPlayed),
            vec![json!(card.id)]
        );
    }

    #[test]
    fn r429_r57_a_new_instance_of_the_same_card_starts_its_own_count() {
        let mut state = board("r429-copy");
        let mut card = new_instance(&mut state, &counted.id, P1, Zone::Hand { player: P1 });
        card.times_played = Some(3);
        let copy = new_instance(&mut state, &counted.id, P1, Zone::Hand { player: P1 });
        assert_eq!(times_played_of(&card), 3);
        assert_eq!(times_played_of(&copy), 0);
    }
}

// ---------------------------------------------------------------------------------------------
// R429, R766 (issue #557): the price a return that keeps it was played at
// ---------------------------------------------------------------------------------------------

mod r429_r766_step_7_notes_the_price_a_return_that_keeps_it_was_played_at {
    use super::*;
    use jackioh_engine::mana::effective_cost;
    use jackioh_engine::query::return_price_of;
    use jackioh_engine::resolve::RETURN_PRICE_KEY;

    /// p1 plays its copy of `def` from hand at `cost_mod`: the state after the play, and the card's id.
    fn play_at(seed: &str, def: &CardDef, cost_mod: i32) -> (GameState, String) {
        let mut state = board(seed);
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let card = first_in_hand(&mut state, &def.id, P1, "the returning Spell");
        find_instance_mut(&mut state, &card.id)
            .expect("the Spell in hand")
            .cost_mod = cost_mod;
        state.players.p1.mana.current = 10;
        let played = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        )
        .state;
        (played, card.id)
    }

    /// The card under `id` in p1's graveyard.
    fn lying(state: &GameState, id: &str) -> CardInstance {
        state
            .players
            .p1
            .graveyard
            .iter()
            .find(|held| held.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("expected {id} in p1's graveyard"))
    }

    #[test]
    fn r429_r766_a_spell_whose_return_keeps_its_price_lies_in_its_graveyard_at_its_printed_cost_with_the_price_it_was_played_at_noted()
     {
        let (state, id) = play_at("r766-noted", &priced_return, 2);
        let card = lying(&state, &id);
        // R155: step 7 flagged it for its end-of-turn return.
        assert_eq!(card.return_to_hand_at_end_of_turn, Some(true));
        // R766: lying in the graveyard it is its printed card, price included.
        assert_eq!(card.cost_mod, 0);
        assert_eq!(effective_cost(&state, &card, Default::default()), 1);
        // R429: the price it was played at waits for its return, and survives JSON (§9.3).
        assert_eq!(return_price_of(&card), 2);
        assert_eq!(return_price_of(&lying(&round_trip(&state), &id)), 2);
    }

    #[test]
    fn r766_a_return_that_does_not_ask_and_a_price_of_0_note_nothing() {
        // #23 Reoccurring Dream's shape: flagged for its return, and R766's reset is all it gets.
        let (state, id) = play_at("r766-plain", &plain_return, 2);
        let card = lying(&state, &id);
        assert_eq!(card.return_to_hand_at_end_of_turn, Some(true));
        assert_eq!(card.cost_mod, 0);
        assert!(card.memory.get(RETURN_PRICE_KEY).is_none());
        assert_eq!(return_price_of(&card), 0);

        // A card that asks but was played at no price: nothing to note, so its state is the one it had
        // before the note existed.
        let (state, id) = play_at("r766-zero", &priced_return, 0);
        assert!(lying(&state, &id).memory.get(RETURN_PRICE_KEY).is_none());
    }

    #[test]
    fn r766_r155_the_note_goes_with_the_flag_as_the_card_leaves_the_graveyard_and_as_the_turn_ends() {
        // Taken out of the graveyard (a return, #72's Discover, a play from there): the landing it was
        // noted for is spent.
        let (mut state, id) = play_at("r766-leaves", &priced_return, 2);
        let mut card = lying(&state, &id);
        move_to_zone(
            &mut state,
            &mut card,
            OffFieldZone::Hand,
            MoveToZoneOptions::default(),
        );
        let held = find_instance(&state, &id).expect("the card in hand");
        assert!(matches!(held.zone, Zone::Hand { .. }));
        assert_ne!(held.return_to_hand_at_end_of_turn, Some(true));
        assert!(held.memory.get(RETURN_PRICE_KEY).is_none());
        assert_eq!(held.cost_mod, 0);

        // Left lying there to the end of the turn (this fixture's return does nothing): cleanup ends
        // the return, and the note with it.
        let (state, id) = play_at("r766-cleanup", &priced_return, 2);
        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        let card = lying(&ended, &id);
        assert_ne!(card.return_to_hand_at_end_of_turn, Some(true));
        assert!(card.memory.get(RETURN_PRICE_KEY).is_none());
    }

    #[test]
    fn r766_a_spell_whose_return_keeps_its_price_reaching_its_graveyard_any_other_way_has_nothing_noted() {
        // Discarded from a hand at a price: never played, never flagged, so R766's reset is all there is.
        let mut state = board("r766-discarded");
        let card = first_in_hand(&mut state, &priced_return.id, P1, "the returning Spell");
        find_instance_mut(&mut state, &card.id)
            .expect("the Spell in hand")
            .cost_mod = 2;
        let mut discarded = card.clone();
        move_to_zone(
            &mut state,
            &mut discarded,
            OffFieldZone::Graveyard,
            MoveToZoneOptions::default(),
        );
        let card = lying(&state, &card.id);
        assert_eq!(card.cost_mod, 0);
        assert_ne!(card.return_to_hand_at_end_of_turn, Some(true));
        assert!(card.memory.get(RETURN_PRICE_KEY).is_none());
    }
}

mod r427_r174_a_resolved_plays_card_that_something_answering_the_play_has_since_taken_off_the_field {
    use super::*;

    #[test]
    fn r427_left_field_since_resolved_is_false_while_the_card_stands_and_for_one_that_left_before_the_play_resolved_true_once_it_leaves_after()
     {
        let mut state = board("r427-left");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let resolved_now = |state: &GameState| -> GameEvent {
            json_as(json!({
                "type": "cardResolved",
                "player": "p1",
                "instanceId": unit.id,
                "defId": plain.id,
                "permanent": true,
                "costPaid": 1,
                "exitsFrom": exit_mark(state),
            }))
        };

        // Standing: nothing has taken it.
        let event = resolved_now(&state);
        assert!(!left_field_since_resolved(&state, &event));

        // Taken off after the play resolved — what a trap answering the play does (R174).
        let mut leaving = unit.clone();
        move_to_zone(
            &mut state,
            &mut leaving,
            OffFieldZone::Graveyard,
            MoveToZoneOptions::default(),
        );
        assert!(left_field_since_resolved(&state, &event));

        // A play whose card had already left as it resolved (its own resolution took it, R427): its
        // event's mark is after the departure, so nothing has taken it since.
        let mut later = json_of(resolved_now(&state));
        later["permanent"] = json!(false);
        assert!(!left_field_since_resolved(&state, &json_as(later)));
    }
}

// ---------------------------------------------------------------------------------------------
// R433: a dealt deck
// ---------------------------------------------------------------------------------------------

/// Both mulligans answered keeping everything, in seat order.
fn keep_all(state: GameState, log: &mut Vec<Action>) -> GameState {
    let mut next = state;
    for player in [P1, P2] {
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
        let action = Action::new(
            ActionBody::Mulligan {
                keep: ids(&next.players[player].hand),
            },
            player,
            format!("r433-{nonce}"),
        );
        let result = reduce(&next, &action);
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        log.push(action);
        next = result.state;
    }
    next
}

mod r433_a_dealt_deck_lists_only_the_cards_its_owner_has_been_shown {
    use super::*;

    fn decks() -> (Vec<String>, Vec<String>) {
        (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))
    }

    fn create(seed: &str, dealt: Option<Vec<PlayerId>>) -> GameState {
        create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks(),
            dealt,
            ..Default::default()
        })
    }

    #[test]
    fn r433_a_dealt_seats_starting_library_is_every_card_unknown_a_built_seats_is_listed_in_full_r310() {
        new_game("r433-setup", None);
        let state = create("r433", Some(vec![P1]));
        assert_eq!(
            json_of(own_library_view(&state, P1)),
            json!({ "cards": [], "unknown": DECK_SIZE })
        );
        assert_eq!(json_of(own_library_view(&state, P2))["unknown"], json!(0));
        assert_eq!(
            listed(&json_of(own_library_view(&state, P2))),
            i64::from(DECK_SIZE)
        );
        // The view carries it so: the list, and nothing else of the library (§9.1).
        assert_eq!(
            json_of(view_for(&state, P1))["you"]["ownLibrary"],
            json!({ "cards": [], "unknown": DECK_SIZE })
        );
        assert!(
            json_of(view_for(&state, P1))["opponent"]
                .get("ownLibrary")
                .is_none()
        );
    }

    #[test]
    fn r433_r311_what_goes_in_openly_afterwards_is_listed_a_mulligans_returns_are_known_the_rest_stays_unknown()
     {
        new_game("r433-setup", None);
        let mut state = begin_game(&create("r433-mulligan", Some(vec![P1]))).state;
        let hand = ids(&state.players.p1.hand);
        let returned = hand.first().cloned().expect("expected a card to return");
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": hand[1..], "playerId": "p1" }),
        )
        .state;
        let keep = ids(&state.players.p2.hand);
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
        )
        .state;

        // The one returned card went in openly (R311) and is listed while it is there; every other card
        // of the library, the dealt ones, stays unknown.
        let list = json_of(own_library_view(&state, P1));
        let known = listed(&list);
        let still_there = state.players.p1.library.iter().any(|card| card.id == returned);
        assert_eq!(known, if still_there { 1 } else { 0 });
        assert_eq!(
            list["unknown"].as_i64(),
            Some(state.players.p1.library.len() as i64 - known)
        );
        assert!(list["unknown"].as_i64().unwrap_or(0) > 0);
    }

    #[test]
    fn r433_no_seat_dealt_or_dealt_omitted_the_game_is_created_exactly_as_before() {
        new_game("r433-setup", None);
        let plain_game = create("r433-same", None);
        let none_dealt = create("r433-same", Some(vec![]));
        assert_eq!(hash_state(&none_dealt), hash_state(&plain_game));
        assert_ne!(
            hash_state(&create("r433-same", Some(vec![P2]))),
            hash_state(&plain_game)
        );
    }

    #[test]
    fn r433_9_3_a_fold_given_the_same_dealt_seats_reproduces_the_game_one_without_them_does_not() {
        new_game("r433-setup", None);
        let mut log: Vec<Action> = Vec::new();
        let live = keep_all(
            begin_game(&create("r433-fold", Some(vec![P1, P2]))).state,
            &mut log,
        );

        let replayed = fold(&json_as(json!({
            "seed": "r433-fold",
            "decks": decks(),
            "log": log,
            "dealt": ["p1", "p2"],
        })));
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&live));
        assert_eq!(
            json_of(view_for(&replayed.state, P1)),
            json_of(view_for(&live, P1))
        );

        let forgotten = fold(&json_as(
            json!({ "seed": "r433-fold", "decks": decks(), "log": log }),
        ));
        assert_ne!(hash_state(&forgotten.state), hash_state(&live));
    }
}

// ---------------------------------------------------------------------------------------------
// R434: the game's end reveals both hands
// ---------------------------------------------------------------------------------------------

mod r434_once_the_game_is_over_each_view_carries_the_other_players_hand {
    use super::*;

    #[test]
    fn r434_before_the_end_the_opponents_hand_is_a_count_after_it_the_cards_as_they_stand_and_nothing_else_is_revealed()
     {
        let mut state = board("r434");
        let theirs = in_hand(&mut state, &plain.id, P2, 2);
        in_hand(&mut state, &uncounted.id, P1, 1);
        put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));

        assert_eq!(
            json_of(view_for(&state, P1))["opponent"]["hand"],
            json!({ "count": 2 })
        );

        // p2 concedes: the game is decided (§2.5).
        let over = act(&state, json!({ "type": "concede", "playerId": "p2" })).state;
        assert!(over.result.is_some());

        let p1_view = json_of(view_for(&over, P1));
        let hand = &p1_view["opponent"]["hand"];
        assert!(hand.is_array());
        let hand_ids: Vec<Value> = hand
            .as_array()
            .map(|cards| cards.iter().map(|card| card["instanceId"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(
            hand_ids,
            theirs.iter().map(|card| json!(card.id)).collect::<Vec<_>>()
        );
        // As the owner sees them: a Unit's stats ride with it (R243).
        assert!(matches_object(
            &hand[0],
            &json!({ "defId": plain.id, "attack": 3, "health": 3 })
        ));
        // And for the other seat, symmetrically.
        let other: Vec<Value> = json_of(view_for(&over, P2))["opponent"]["hand"]
            .as_array()
            .map(|cards| cards.iter().map(|card| card["defId"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(other, vec![json!(uncounted.id.clone())]);
        // The libraries stay a count, and the opponent's list is never sent (§9.1).
        assert_eq!(
            p1_view["opponent"]["libraryCount"],
            json!(over.players.p2.library.len())
        );
        assert!(p1_view["opponent"].get("ownLibrary").is_none());
    }

    #[test]
    fn r434_r97_an_event_that_named_a_hand_card_while_it_was_hidden_stays_redacted_after_the_end() {
        let mut state = board("r434-events");
        put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        in_hand(&mut state, &uncounted.id, P1, 1);
        // p1 ends the turn; p2's draw names a card p1 may not read.
        let turned = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        let draw_event = view_events_of(&json_of(view_for(&turned, P1)), "drawn")
            .into_iter()
            .find(|event| event["player"] == json!("p2"))
            .expect("expected p2's draw");
        assert!(matches_object(&draw_event, &json!({ "instanceId": HIDDEN_ID })));

        let over = act(&turned, json!({ "type": "concede", "playerId": "p1" })).state;
        let after = json_of(view_for(&over, P1));
        assert!(after["opponent"]["hand"].is_array());
        let still_hidden: Vec<Value> = view_events_of(&after, "drawn")
            .into_iter()
            .filter(|event| event["player"] == json!("p2"))
            .collect();
        assert!(!still_hidden.is_empty());
        for event in &still_hidden {
            assert!(matches_object(event, &json!({ "instanceId": HIDDEN_ID })));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// R437: a marked card shows its mark
// ---------------------------------------------------------------------------------------------

fn unit_view_of(state: &GameState, viewer: PlayerId, side: &str, lane: usize) -> Value {
    let unit = json_of(view_for(state, viewer))[side]["units"][lane - 1].clone();
    assert!(!unit.is_null(), "expected {viewer}'s {side} lane {lane}");
    unit
}

mod r437_a_delayed_effect_aimed_at_a_card_marks_it_in_both_views_while_it_waits {
    use super::*;

    struct Marked {
        state: GameState,
        target: CardInstance,
        events: Vec<GameEvent>,
    }

    fn marked(seed: &str) -> Marked {
        let mut state = board(seed);
        put(&mut state, &big_body.id, slot(P1, Row::Units, 5), json!({}));
        let target = put(&mut state, &big_body.id, slot(P2, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        in_hand(&mut state, &plain.id, P2, 1);
        let card = first_in_hand(&mut state, &marker.id, P1, "the marker");
        in_hand(&mut state, &uncounted.id, P1, 1);
        state.players.p1.mana.current = 4;
        let played = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "targets": [{ "pick": "instance", "instanceId": target.id }],
                "playerId": "p1",
            }),
        );
        Marked {
            state: played.state,
            target,
            events: played.events,
        }
    }

    /// `TEST_MARK` as the views and events carry it.
    fn mark() -> Value {
        json_of(TEST_MARK)
    }

    fn marked_event(instance_id: &str, added: bool) -> Value {
        json!({
            "type": "marked",
            "instanceId": instance_id,
            "mark": mark()["mark"],
            "color": mark()["color"],
            "added": added,
        })
    }

    #[test]
    fn r437_the_play_marks_its_target_with_a_marked_event_and_both_seats_views_carry_the_mark() {
        let Marked {
            state,
            target,
            events,
        } = marked("r437-mark");
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Marked)),
            json!([marked_event(&target.id, true)])
        );
        assert_eq!(json_of(marks_on(&state, &target.id)), json!([mark()]));
        assert_eq!(unit_view_of(&state, P1, "opponent", 1)["marks"], json!([mark()]));
        assert_eq!(unit_view_of(&state, P2, "you", 1)["marks"], json!([mark()]));
        // No other card carries one, and an unmarked card carries no key at all.
        assert!(unit_view_of(&state, P2, "you", 2).get("marks").is_none());
        // Both seats read the event (the target is public).
        for viewer in [P1, P2] {
            assert_eq!(
                json!(view_events_of(&json_of(view_for(&state, viewer)), "marked")),
                json_of(events_of_type(&events, GameEventType::Marked))
            );
        }
    }

    #[test]
    fn r437_the_mark_survives_a_round_trip_and_goes_when_the_effect_resolves_with_a_marked_removal() {
        let first = marked("r437-resolve");
        let thawed = round_trip(&first.state);
        assert_eq!(thawed, first.state);

        let mut state = act(&thawed, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert_eq!(unit_view_of(&state, P1, "opponent", 1)["marks"], json!([mark()]));
        let ReduceResult {
            state: back, events, ..
        } = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        state = back;

        // The hit landed at p1's start of turn, and the mark is gone from both views.
        assert_eq!(state.active, P1);
        assert!(
            events_of_type(&events, GameEventType::Damage)
                .into_iter()
                .any(|event| json_of(event)["targetId"] == json!(first.target.id))
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Marked)),
            json!([marked_event(&first.target.id, false)])
        );
        assert!(unit_view_of(&state, P1, "opponent", 1).get("marks").is_none());
        assert!(state.marks.is_none());
    }

    #[test]
    fn r437_r174_the_mark_goes_the_moment_its_card_leaves_the_field_and_the_effect_with_it() {
        let Marked {
            mut state, target, ..
        } = marked("r437-leave");
        let mut sink = Sink::for_state(&state);
        {
            let mut engine = sink.on(&mut state);
            let mut ctx = make_context(
                &mut engine,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (damage(json_as(
                json!({ "to": { "of": "instance", "instanceId": target.id }, "amount": 20 }),
            ))
            .apply)(&mut ctx);
        }
        state_check(&mut sink.on(&mut state));
        // The next time the loop collects events, the removal is said.
        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(
            json_of(events_of_type(&after.events, GameEventType::Marked)),
            json!([marked_event(&target.id, false)])
        );
        assert!(after.state.delayed.is_empty());
        assert!(after.state.marks.is_none());
        // The view drops it at once, before any sweep: the effect it belonged to is gone.
        assert_eq!(json_of(marks_on(&state, &target.id)), json!([]));
    }

    #[test]
    fn r437_r33_a_face_down_target_its_controller_reads_the_mark_on_the_card_the_other_player_on_its_back_and_the_event_names_the_sentinel()
     {
        let mut state = board("r437-face-down");
        put(&mut state, &big_body.id, slot(P1, Row::Units, 5), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        in_hand(&mut state, &plain.id, P2, 1);
        let trap = put(&mut state, &quiet_trap.id, slot(P2, Row::Backrow, 1), json!({}));
        let card = first_in_hand(&mut state, &marker.id, P1, "the marker");
        in_hand(&mut state, &uncounted.id, P1, 1);
        state.players.p1.mana.current = 4;

        let played = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "targets": [{ "pick": "instance", "instanceId": trap.id }],
                "playerId": "p1",
            }),
        )
        .state;

        // p1 may not read the trap (R33): its back carries the mark, and nothing names it.
        let p1_view = json_of(view_for(&played, P1));
        assert_eq!(
            p1_view["opponent"]["backrow"][0],
            json!({ "faceDown": true, "cost": 1, "marks": [mark()] })
        );
        let p1_event = view_events_of(&p1_view, "marked")
            .into_iter()
            .next()
            .expect("expected p1's marked event");
        assert!(matches_object(
            &p1_event,
            &json!({ "instanceId": HIDDEN_ID, "mark": mark()["mark"], "color": mark()["color"], "added": true })
        ));
        // p2 reads its own trap, mark and all, and the event names it.
        let p2_view = json_of(view_for(&played, P2));
        assert!(matches_object(
            &p2_view["you"]["backrow"][0],
            &json!({ "instanceId": trap.id, "marks": [mark()] })
        ));
        assert!(matches_object(
            &view_events_of(&p2_view, "marked")
                .into_iter()
                .next()
                .unwrap_or(Value::Null),
            &json!({ "instanceId": trap.id })
        ));
    }
}
