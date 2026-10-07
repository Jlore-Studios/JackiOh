//! Port of `packages/engine/test/draw-limit.test.ts`.
//!
//! Draws counted per turn and limited (docs/classic-sets.md B5 E3, E4, R457), and the cast-on-draw
//! enchantment and a Unit cast on draw with nowhere to stand (B5 E39, Classic+ #26, R459).
//!
//! What is pinned: every draw that happens is counted for its player on the turn it happens, whoever's
//! turn that is — the start-of-turn draw and a fatigue draw included — and the `drawn` event carries the
//! draw's number, which a trap reads however much later the loop hands it the event (Classic #9); a draw
//! past a limit does not happen at all (no card moves, no fatigue, nothing is cast) and says so with a
//! public `drawLimited`; the lowest limit holds and "both" binds its own controller; a named draw is a
//! draw; and both survive a JSON round trip and a replay.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::draw::{draw_blocked, draw_limit_of, draw_one, draws_this_turn, DrawOutcome};
use jackioh_engine::effects::draw_from_library;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, setup_catalog, slot};
use crate::rules::fixtures::turn::{
    anti_greed, cast_spell, cast_unit, draw_two, log_card, notes, palantir, plain, taxman, turn_catalog,
    TURN_SCRIPTS, LOG_LANE,
};

fn register() {
    register_catalog(turn_catalog(registered_catalog().clone()));
    let mut all = registered_scripts().clone();
    all.extend(TURN_SCRIPTS.clone());
    register_scripts(all);
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// TS `act`: `{ state, events }`, and a refusal throws.
fn act(state: &GameState, body: Value) -> (GameState, Vec<GameEvent>) {
    let n = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("dl{n}"));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

/// Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("draw-limit-{seed}"), None)).state;
    register();
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
        state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player })).0;
    }
    put(&mut state, &log_card().id, slot(PlayerId::P2, Row::Backrow, LOG_LANE), Default::default());
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// TS `sinkFor(state)`: a sink's events and rng (from the state's cursor), lent with the state to one
/// engine call at a time, so the test reads and writes the state between calls as TS did.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// `eventsOfType(events, type)`, each event as its JSON, so a test reads its fields by TS's names and
/// TS's `toEqual` literals compare key for key.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// A view's events as JSON.
fn view_events(state: &GameState, viewer: PlayerId) -> Vec<Value> {
    view_for(state, viewer)
        .events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// Jest's `toMatchObject`: every key of `expected` is in `actual` with a matching value (objects by
/// subset, arrays by length and element).
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

fn first_card(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    in_hand(state, def_id, player, 1).into_iter().next().expect("a card in hand")
}

mod r457_b5_e4_draws_counted_per_player_per_turn {
    use super::*;

    #[test]
    fn r457_every_draw_that_happens_is_counted_for_its_player_this_turn_on_either_players_turn_fatigue_included() {
        let mut state = playing("count");
        let mut bench = Bench::new(&state);
        // Whatever p1 has drawn this turn so far, the next draw is one more.
        let already = draws_this_turn(&state, PlayerId::P1);
        set_library(&mut state, PlayerId::P1, &strings(&["fx-5", "fx-6"]));
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25"]));
        draw_one(&mut bench.sink(&mut state), PlayerId::P1, None);
        draw_one(&mut bench.sink(&mut state), PlayerId::P2, None);
        draw_one(&mut bench.sink(&mut state), PlayerId::P2, None);
        assert_eq!(draws_this_turn(&state, PlayerId::P1), already + 1);
        // p2's second draw found an empty library: a fatigue draw, and a draw that happened.
        assert_eq!(state.players.p2.fatigue_count, 1);
        assert_eq!(draws_this_turn(&state, PlayerId::P2), 2);
        let numbered: Vec<Value> = of_type(&bench.events, GameEventType::Drawn)
            .iter()
            .map(|event| json!([event["player"], event["turnDraw"]]))
            .collect();
        assert_eq!(numbered, vec![json!(["p1", already + 1]), json!(["p2", 1])]);
    }

    #[test]
    fn r457_r225_setup_is_no_players_turn_the_opening_deal_and_the_mulligan_count_nothing_and_number_no_draw() {
        let opened = begin_game(&new_game("draw-limit-setup", None));
        assert!(!of_type(&opened.events, GameEventType::Drawn).is_empty());
        assert!(of_type(&opened.events, GameEventType::Drawn).iter().all(|event| event.get("turnDraw").is_none()));
        assert_eq!(opened.state.players.p1.draws, None);
        assert_eq!(opened.state.players.p2.draws, None);
    }

    #[test]
    fn r457_r97_a_drawn_events_number_is_public_though_the_card_it_names_is_not() {
        let mut state = playing("public-count");
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26"]));
        let (after, _) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let their_draw = view_events(&after, PlayerId::P1)
            .into_iter()
            .find(|event| event["type"] == "drawn" && event["player"] == "p2")
            .expect("p1 sees p2's draw");
        assert!(matches_object(&their_draw, &json!({ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "turnDraw": 1 })));
        let own_draw = view_events(&after, PlayerId::P2)
            .into_iter()
            .find(|event| event["type"] == "drawn" && event["player"] == "p2")
            .expect("p2 sees its own draw");
        assert!(matches_object(&own_draw, &json!({ "defId": "fx-25", "turnDraw": 1 })));
    }

    #[test]
    fn r457_the_count_resets_with_the_turn_as_the_turn_log_does_and_the_turns_own_draw_is_the_first_of_the_new_one() {
        let mut state = playing("reset");
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26", "fx-27"]));
        draw_one(&mut Bench::new(&state).sink(&mut state), PlayerId::P2, None);
        assert_eq!(draws_this_turn(&state, PlayerId::P2), 1);
        let (after, events) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(draws_this_turn(&after, PlayerId::P2), 1);
        let theirs: Vec<Value> = of_type(&events, GameEventType::Drawn)
            .iter()
            .filter(|event| event["player"] == "p2")
            .map(|event| event["turnDraw"].clone())
            .collect();
        assert_eq!(theirs, vec![json!(1)]);
        assert_eq!(draws_this_turn(&after, PlayerId::P1), 0);
    }

    #[test]
    fn r457_a_trap_on_the_opponents_2nd_draw_reads_the_draws_own_number_however_late_the_loop_hands_it_the_event() {
        let mut state = playing("taxman");
        put(&mut state, &taxman().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26", "fx-27", "fx-28"]));
        // p2's turn: the start-of-turn draw is the 1st, and sets nothing off.
        let (mut started, _) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&started), Vec::<String>::new());
        // One effect draws two more: both `drawn` events reach the trigger after the whole effect, when
        // the count already reads 3, and only the one that was the 2nd sets it off.
        let two = first_card(&mut started, PlayerId::P2, &draw_two().id);
        let (after, events) = act(&started, json!({ "type": "play", "instanceId": two.id, "playerId": "p2" }));
        let numbers: Vec<Value> =
            of_type(&events, GameEventType::Drawn).iter().map(|event| event["turnDraw"].clone()).collect();
        assert_eq!(numbers, vec![json!(2), json!(3)]);
        assert_eq!(notes(&after), strings(&["taxed"]));
        assert_eq!(draws_this_turn(&after, PlayerId::P2), 3);
    }
}

mod r457_b5_e3_draw_limits {
    use super::*;

    #[test]
    fn r457_a_draw_from_the_other_players_deck_is_the_drawers_draw_so_the_drawers_limit_stops_it_before_any_card_moves_e16() {
        let mut state = playing("limit-opponent-deck");
        // Palantir in p2's backrow limits p2's opponent, p1, to one draw a turn.
        put(&mut state, &palantir().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        assert_eq!(draw_limit_of(&state, PlayerId::P1), Some(1));
        // p1's turn's own draw has been made: the one draw the limit allows.
        assert_eq!(draws_this_turn(&state, PlayerId::P1), 1);
        let theirs = set_library(&mut state, PlayerId::P2, &strings(&["fx-26", "fx-27"]));
        let mut bench = Bench::new(&state);
        assert_eq!(
            draw_from_library_of(&mut bench.sink(&mut state), PlayerId::P1, PlayerId::P2, LibraryEnd::Bottom),
            Some(DrawOutcome::Limited)
        );
        assert_eq!(ids(&state.players.p2.library), ids(&theirs));
        assert_eq!(of_type(&bench.events, GameEventType::Stolen), Vec::<Value>::new());
        assert_eq!(
            of_type(&bench.events, GameEventType::DrawLimited),
            vec![json!({ "type": "drawLimited", "player": "p1" })]
        );
    }

    #[test]
    fn r457_a_draw_past_the_limit_does_not_happen_no_card_moves_nothing_is_cast_no_fatigue_and_draw_limited_says_so() {
        let mut state = playing("limit");
        put(&mut state, &palantir().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        assert_eq!(draw_limit_of(&state, PlayerId::P2), Some(1));
        assert_eq!(draw_limit_of(&state, PlayerId::P1), None);

        let second = set_library(&mut state, PlayerId::P2, &["fx-25".to_string(), cast_spell().id])[1].clone();
        let mut bench = Bench::new(&state);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P2, None), DrawOutcome::Drawn);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P2, None), DrawOutcome::Limited);
        // The cast-on-draw card stays where it was, uncast.
        assert_eq!(ids(&state.players.p2.library), vec![second.id.clone()]);
        assert_eq!(notes(&state), Vec::<String>::new());
        assert_eq!(draws_this_turn(&state, PlayerId::P2), 1);
        assert_eq!(
            of_type(&bench.events, GameEventType::DrawLimited),
            vec![json!({ "type": "drawLimited", "player": "p2" })]
        );

        // An empty library behind a limit fatigues no one.
        state.players.p2.library = vec![];
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P2, None), DrawOutcome::Limited);
        assert_eq!(state.players.p2.fatigue_count, 0);
        assert_eq!(of_type(&bench.events, GameEventType::Fatigue), Vec::<Value>::new());
    }

    #[test]
    fn r457_the_start_of_turn_draw_counts_toward_the_limit_so_a_draw_2_on_that_turn_draws_nothing() {
        let mut state = playing("start-draw");
        put(&mut state, &palantir().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26", "fx-27"]));
        let (mut started, _) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(draws_this_turn(&started, PlayerId::P2), 1);
        let card = started.players.p2.hand.iter().find(|held| held.def_id == "fx-25");
        assert!(card.is_some());

        let two = first_card(&mut started, PlayerId::P2, &draw_two().id);
        let (after, events) = act(&started, json!({ "type": "play", "instanceId": two.id, "playerId": "p2" }));
        assert_eq!(
            of_type(&events, GameEventType::DrawLimited),
            vec![
                json!({ "type": "drawLimited", "player": "p2" }),
                json!({ "type": "drawLimited", "player": "p2" }),
            ]
        );
        assert_eq!(def_ids(&after.players.p2.library), strings(&["fx-26", "fx-27"]));
    }

    #[test]
    fn r457_with_several_limits_the_lowest_holds_and_a_limit_on_both_players_binds_its_own_controller_too() {
        let mut state = playing("lowest");
        put(&mut state, &anti_greed().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        assert_eq!(draw_limit_of(&state, PlayerId::P1), Some(2));
        assert_eq!(draw_limit_of(&state, PlayerId::P2), Some(2));
        put(&mut state, &palantir().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        assert_eq!(draw_limit_of(&state, PlayerId::P2), Some(1));
        assert_eq!(draw_limit_of(&state, PlayerId::P1), Some(2));
        // §6.3 Vanilla: a card with no text sets no limit.
        let greed = &mut state.players.p2.units[0].as_mut().expect("p2's unit zone 1 holds Anti-Greed")[0];
        greed.vanilla = true;
        assert_eq!(draw_limit_of(&state, PlayerId::P1), None);
    }

    #[test]
    fn r457_a_named_draw_is_a_draw_the_limit_stops_it_and_the_card_stays_in_the_library() {
        let mut state = playing("named");
        put(&mut state, &palantir().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        let library = ids(&set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26"])));
        let (first, second) = (library[0].clone(), library[1].clone());
        let mut bench = Bench::new(&state);
        let mut sink = bench.sink(&mut state);
        let mut ctx =
            make_context(&mut sink, None, HookOptions { controller: Some(PlayerId::P2), ..Default::default() });
        (draw_from_library(json_as(json!({ "instanceId": second }))).apply)(&mut ctx);
        (draw_from_library(json_as(json!({ "instanceId": first }))).apply)(&mut ctx);
        assert_eq!(ids(&ctx.state.players.p2.library), vec![first.clone()]);
        assert_eq!(of_type(ctx.events.as_slice(), GameEventType::DrawLimited).len(), 1);
        drop(ctx);
        drop(sink);
        assert!(draw_blocked(&mut Bench::new(&state).sink(&mut state), PlayerId::P2));
    }

    #[test]
    fn r457_a_cast_on_draw_chain_stops_at_the_limit_the_next_draw_of_the_chain_does_not_happen() {
        let mut state = playing("chain");
        put(&mut state, &palantir().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        set_library(&mut state, PlayerId::P1, &[cast_spell().id, plain().id]);
        // No draw of p1's yet this turn, so the chain's first draw is the one the limit allows.
        state.players.p1.draws = Some(DrawCount { turn: state.turn, count: 0 });
        let mut bench = Bench::new(&state);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Cast);
        assert_eq!(notes(&state), strings(&["cast-spell"]));
        assert_eq!(def_ids(&state.players.p1.library), vec![plain().id]);
        assert_eq!(
            of_type(&bench.events, GameEventType::DrawLimited),
            vec![json!({ "type": "drawLimited", "player": "p1" })]
        );
    }

    #[test]
    fn r457_draw_limited_is_public_in_both_views_it_names_a_player_and_no_card() {
        let mut state = playing("public");
        put(&mut state, &palantir().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        set_library(&mut state, PlayerId::P2, &strings(&["fx-25", "fx-26"]));
        let (mut started, _) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let two = first_card(&mut started, PlayerId::P2, &draw_two().id);
        let (after, _) = act(&started, json!({ "type": "play", "instanceId": two.id, "playerId": "p2" }));
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let limited: Vec<Value> =
                view_events(&after, viewer).into_iter().filter(|event| event["type"] == "drawLimited").collect();
            assert_eq!(
                limited,
                vec![
                    json!({ "type": "drawLimited", "player": "p2" }),
                    json!({ "type": "drawLimited", "player": "p2" }),
                ]
            );
        }
    }

    #[test]
    fn r457_a_limited_game_survives_json_and_replays_from_its_log() {
        let seed = "draw-limit-replay";
        let deck_one: Vec<String> =
            std::iter::once(palantir().id).chain(vanilla_deck(DECK_SIZE - 1, 1)).collect();
        let deck_two: Vec<String> =
            std::iter::once(draw_two().id).chain(vanilla_deck(DECK_SIZE - 1, 21)).collect();
        setup_catalog();
        register();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: (deck_one.clone(), deck_two.clone()),
            ..Default::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        let step = |state: &mut GameState, log: &mut Vec<Action>, body: Value| {
            let mut action = body;
            action["nonce"] = json!(format!("rp{}", log.len()));
            let action: Action = json_as(action);
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            *state = result.state;
        };
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            step(&mut state, &mut log, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
        }
        let limit_card = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == palantir().id)
            .expect("p1 holds the Palantir")
            .clone();
        step(&mut state, &mut log, json!({ "type": "play", "instanceId": limit_card.id, "playerId": "p1" }));
        step(&mut state, &mut log, json!({ "type": "endTurn", "playerId": "p1" }));
        let library = state.players.p2.library.len();
        let two = state
            .players
            .p2
            .hand
            .iter()
            .find(|card| card.def_id == draw_two().id)
            .expect("p2 holds the Draw 2")
            .clone();
        step(&mut state, &mut log, json!({ "type": "play", "instanceId": two.id, "playerId": "p2" }));
        assert_eq!(state.players.p2.library.len(), library);
        let round: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("a state serialises")).expect("and parses");
        assert_eq!(round, state);

        let replayed = fold(&json_as(json!({ "seed": seed, "decks": [deck_one, deck_two], "log": log })));
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
    }
}

mod r459_b5_e39_and_classic_plus_26_cast_on_draw {
    use super::*;

    #[test]
    fn r459_the_cast_on_draw_enchantment_makes_a_drawn_card_cast_on_draw() {
        let mut state = playing("enchanted");
        let library = set_library(&mut state, PlayerId::P1, &[plain().id, "fx-6".to_string()]);
        assert!(library.first().is_some(), "no card");
        let mut bench = Bench::new(&state);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Drawn);
        assert_eq!(notes(&state), Vec::<String>::new());

        let enchanted = set_library(&mut state, PlayerId::P1, &[plain().id, "fx-6".to_string()])
            .first()
            .cloned()
            .expect("no card");
        find_instance_mut(&mut state, &enchanted.id).expect("the top card").enchantments =
            Some(vec![Enchantment::CastOnDraw]);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Cast);
        assert_eq!(notes(&state), strings(&["plain"]));
        assert!(ids(&state.players.p1.graveyard).contains(&enchanted.id));
        // The chain drew on: the card beneath came to hand.
        assert!(def_ids(&state.players.p1.hand).contains(&"fx-6".to_string()));
    }

    #[test]
    fn r459_a_unit_cast_on_draw_with_no_open_unit_zone_goes_to_the_hand_uncast_and_with_one_it_is_cast_into_it() {
        let mut state = playing("unit-room");
        for lane in 1..=UNIT_ZONES {
            put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, lane), Default::default());
        }
        let unit = set_library(&mut state, PlayerId::P1, &[cast_unit().id]).first().cloned().expect("no card");
        let mut bench = Bench::new(&state);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Drawn);
        assert!(ids(&state.players.p1.hand).contains(&unit.id));
        assert_eq!(notes(&state), Vec::<String>::new());
        assert_eq!(of_type(&bench.events, GameEventType::CardPlayed), Vec::<Value>::new());

        let mut open = playing("unit-open");
        put(&mut open, "fx-2", slot(PlayerId::P1, Row::Units, 1), Default::default());
        let second = set_library(&mut open, PlayerId::P1, &[cast_unit().id]).first().cloned();
        let mut open_bench = Bench::new(&open);
        assert_eq!(draw_one(&mut open_bench.sink(&mut open), PlayerId::P1, None), DrawOutcome::Cast);
        assert_eq!(
            open.players.p1.units[1].as_ref().and_then(|pile| pile.first()).map(|card| card.id.clone()),
            second.map(|card| card.id)
        );
        assert_eq!(notes(&open), strings(&["cast-unit"]));
    }
}
